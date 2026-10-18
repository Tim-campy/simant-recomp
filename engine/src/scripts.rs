//! Scripts Lua dos mods: carregamento, sandbox, limites e a API pública.
//! Contrato (o que um script pode fazer): docs/api-mods.md, seção "Scripts Lua" (API 0.12).
//!
//! Cada mod com `"scripts"` no `mod.json` ganha uma máquina Lua só sua (estado isolado dos outros
//! mods), com as bibliotecas `table`, `string`, `math` e `utf8` e sem `io`, `os`, `package`,
//! `require`, `debug`, `load`/`loadfile`/`dofile` nem `string.dump`. Os arquivos rodam em ordem e
//! definem funções globais com os nomes dos ganchos ([`Gancho`]).
//!
//! O script nunca vê o [`Estado`] cru: recebe um objeto `estado` válido só durante a chamada, com
//! métodos de leitura (cópias) e poucos métodos de escrita validados. A aleatoriedade do script
//! (`aleatorio`, `math.random`) é um gerador próprio por mod: os geradores S e R do jogo não são
//! tocados. Cada chamada tem limite de instruções e de tempo, e a máquina tem limite de memória; um
//! erro (de carga, de execução ou de limite) desliga os scripts daquele mod, registra o erro e pede
//! um aviso na faixa. O jogo continua.
//!
//! Fidelidade: os ganchos rodam fora de `rules::tick::tick` (antes e depois dele, em
//! `core::partida`); sem scripts, nada aqui é chamado.

use std::cell::Cell;
use std::path::{Component, Path};
use std::rc::Rc;
use std::time::{Duration, Instant};

use mlua::{ChunkMode, Function, HookTriggers, Lua, LuaOptions, StdLib, Table, UserDataMethods, Value, Variadic, VmState};

use crate::formats::save::{Formiga, Lista};
use crate::recursos::{Mod, Recursos};
use crate::rules::amarela::TabelasAmarela;
use crate::rules::comum;
use crate::rules::estado::{cor_do_tipo, Estado, Evento, Grade};
use crate::rules::experimental::{self, TabelasExperimental};
use crate::rules::geracao::Cor;
use crate::rules::inimigos::TabelasInimigos;

/// Versão da API de mods que os scripts veem em `API_VERSAO`.
pub const VERSAO_API: &str = "0.12";

/// Tamanho máximo do texto de `mostrar_mensagem` (caracteres).
pub const MAX_TEXTO_MENSAGEM: usize = 120;

/// Linhas de registro guardadas por mod entre duas retiradas (o excesso é descartado).
const MAX_REGISTRO: usize = 1000;

/// A cada quantas instruções da VM o limite é conferido.
const INTERVALO_CONFERENCIA: u32 = 1000;

/// Funções globais que o script pode definir.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gancho {
    /// `ao_iniciar(estado)`: uma vez, quando a partida começa (jogo novo ou Load).
    AoIniciar,
    /// `antes_do_tick(estado)`: antes de cada tick da simulação.
    AntesDoTick,
    /// `depois_do_tick(estado)`: depois de cada tick.
    DepoisDoTick,
    /// `ao_evento(evento, estado)`: a cada pedido das regras à interface (som, música, mensagem...).
    AoEvento,
}

impl Gancho {
    pub const TODOS: [Gancho; 4] = [Gancho::AoIniciar, Gancho::AntesDoTick, Gancho::DepoisDoTick, Gancho::AoEvento];

    pub fn nome(self) -> &'static str {
        match self {
            Gancho::AoIniciar => "ao_iniciar",
            Gancho::AntesDoTick => "antes_do_tick",
            Gancho::DepoisDoTick => "depois_do_tick",
            Gancho::AoEvento => "ao_evento",
        }
    }

    fn indice(self) -> usize {
        self as usize
    }
}

/// Limites de cada mod.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limites {
    /// Instruções da VM por chamada (carga de um arquivo ou um gancho).
    pub instrucoes: u64,
    /// Tempo real por chamada.
    pub tempo: Duration,
    /// Memória total da máquina Lua do mod (bytes).
    pub memoria: usize,
}

impl Default for Limites {
    fn default() -> Limites {
        Limites { instrucoes: 1_000_000, tempo: Duration::from_millis(250), memoria: 16 * 1024 * 1024 }
    }
}

/// Pedido de mensagem na faixa feito por um script (`mostrar_mensagem`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MensagemScript {
    pub texto: String,
    /// Duração em unidades de `_TickCount` (como `_EditMessage`); `None` = a duração dos conselhos.
    pub duracao: Option<u32>,
}

/// Contagem de instruções e tempo da chamada em andamento.
struct Orcamento {
    instrucoes: Cell<u64>,
    inicio: Cell<Instant>,
    estourou: Cell<bool>,
}

impl Orcamento {
    fn reiniciar(&self) {
        self.instrucoes.set(0);
        self.inicio.set(Instant::now());
        self.estourou.set(false);
    }
}

/// O que fica dentro de cada máquina Lua (app data): nome, gerador próprio, saídas pendentes e as
/// tabelas usadas pelas escritas.
struct Contexto {
    sorteio: u32,
    registro: Vec<String>,
    mensagens: Vec<MensagemScript>,
    experimental: Rc<TabelasExperimental>,
}

impl Contexto {
    /// xorshift32: o gerador do script, separado de S e R.
    fn proximo(&mut self) -> u32 {
        let mut x = self.sorteio;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.sorteio = x;
        x
    }
}

/// Semente inicial do gerador de um mod: FNV-1a do nome (nunca 0).
fn semente_do_nome(nome: &str) -> u32 {
    let mut h: u32 = 0x811C_9DC5;
    for b in nome.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    if h == 0 {
        1
    } else {
        h
    }
}

struct ScriptMod {
    nome: String,
    lua: Lua,
    ativo: bool,
    ganchos: [bool; 4],
    orcamento: Rc<Orcamento>,
}

/// Os scripts de todos os mods ativos, na ordem dos mods.
pub struct Scripts {
    mods: Vec<ScriptMod>,
    registro: Vec<String>,
    mensagens: Vec<MensagemScript>,
}

/// Os arquivos de script de um mod já lidos: (nome do arquivo, código).
pub type ArquivosScript = Vec<(String, String)>;

impl Scripts {
    /// Scripts dos mods dos recursos (na ordem de aplicação dos mods). `None` se nenhum mod tem
    /// `"scripts"`. Erros de um mod (arquivo ausente, caminho inválido, erro de sintaxe) desligam só
    /// aquele mod e ficam no registro; o `Err` é só para `data/experimental.json` inválido.
    pub fn carregar(r: &Recursos, limites: Limites) -> Result<Option<Scripts>, String> {
        let com_scripts: Vec<&Mod> = r.mods().iter().filter(|m| !m.manifesto.scripts.is_empty()).collect();
        if com_scripts.is_empty() {
            return Ok(None);
        }
        let tabelas = TabelasExperimental::carregar(r)?;
        let fontes = com_scripts.into_iter().map(|m| (m.manifesto.nome.clone(), ler_scripts_do_mod(m))).collect();
        Ok(Some(Scripts::de_fontes(fontes, tabelas, limites)))
    }

    /// Scripts a partir do código já lido (testes e ferramentas). Cada item: nome do mod e os
    /// arquivos, ou o erro de leitura (o mod entra desligado).
    pub fn de_fontes(
        fontes: Vec<(String, Result<ArquivosScript, String>)>,
        tabelas: TabelasExperimental,
        limites: Limites,
    ) -> Scripts {
        let tabelas = Rc::new(tabelas);
        let mut s = Scripts { mods: Vec::new(), registro: Vec::new(), mensagens: Vec::new() };
        for (nome, arquivos) in fontes {
            let arquivos = match arquivos {
                Ok(a) => a,
                Err(e) => {
                    s.avisar_desligado(&nome, &e);
                    continue;
                }
            };
            match carregar_mod(&nome, &arquivos, tabelas.clone(), limites) {
                Ok(m) => {
                    s.mods.push(m);
                    let i = s.mods.len() - 1;
                    recolher(&mut s.mods[i], &mut s.registro, &mut s.mensagens);
                }
                Err(erro) => {
                    let (m, e) = *erro;
                    if let Some(mut m) = m {
                        recolher(&mut m, &mut s.registro, &mut s.mensagens);
                    }
                    s.avisar_desligado(&nome, &e.to_string());
                }
            }
        }
        s
    }

    fn avisar_desligado(&mut self, nome: &str, erro: &str) {
        self.registro.push(format!("[{nome}] script desligado: {erro}"));
        self.mensagens.push(MensagemScript { texto: format!("Mod {nome}: script desligado por erro"), duracao: None });
    }

    /// Algum mod ativo define o gancho?
    pub fn tem_gancho(&self, g: Gancho) -> bool {
        self.mods.iter().any(|m| m.ativo && m.ganchos[g.indice()])
    }

    /// Nomes dos mods com scripts carregados e se ainda estão ativos (desligados por erro = `false`).
    pub fn situacao(&self) -> Vec<(String, bool)> {
        self.mods.iter().map(|m| (m.nome.clone(), m.ativo)).collect()
    }

    /// Mod ativo? (`None`: não há scripts carregados com esse nome.)
    pub fn ativo(&self, nome: &str) -> Option<bool> {
        self.mods.iter().find(|m| m.nome == nome).map(|m| m.ativo)
    }

    /// `ao_iniciar(estado)`. Depois dele os ganchos são procurados de novo (um script pode
    /// defini-los dentro de `ao_iniciar`).
    pub fn ao_iniciar(&mut self, e: &mut Estado) {
        self.chamar(Gancho::AoIniciar, e, None);
        for m in self.mods.iter_mut().filter(|m| m.ativo) {
            m.ganchos = procurar_ganchos(&m.lua);
        }
    }

    pub fn antes_do_tick(&mut self, e: &mut Estado) {
        self.chamar(Gancho::AntesDoTick, e, None);
    }

    pub fn depois_do_tick(&mut self, e: &mut Estado) {
        self.chamar(Gancho::DepoisDoTick, e, None);
    }

    pub fn ao_evento(&mut self, ev: &Evento, e: &mut Estado) {
        self.chamar(Gancho::AoEvento, e, Some(ev));
    }

    /// Linhas de `registrar`/`print` e avisos de desligamento, prefixadas com o nome do mod.
    pub fn retirar_registro(&mut self) -> Vec<String> {
        std::mem::take(&mut self.registro)
    }

    /// Pedidos de mensagem na faixa (`mostrar_mensagem` e avisos de desligamento).
    pub fn retirar_mensagens(&mut self) -> Vec<MensagemScript> {
        std::mem::take(&mut self.mensagens)
    }

    fn chamar(&mut self, g: Gancho, e: &mut Estado, ev: Option<&Evento>) {
        let Scripts { mods, registro, mensagens } = self;
        for m in mods.iter_mut().filter(|m| m.ativo && m.ganchos[g.indice()]) {
            m.orcamento.reiniciar();
            let lua = m.lua.clone();
            let r = lua.scope(|s| {
                let f: Function = lua.globals().get(g.nome())?;
                let ud = s.create_any_userdata_ref_mut(&mut *e)?;
                match ev {
                    Some(ev) => f.call::<()>((tabela_evento(&lua, ev)?, ud)),
                    None => f.call::<()>(ud),
                }
            });
            recolher(m, registro, mensagens);
            if let Err(err) = r {
                m.ativo = false;
                m.ganchos = [false; 4];
                registro.push(format!("[{}] script desligado em {}: {err}", m.nome, g.nome()));
                mensagens.push(MensagemScript { texto: format!("Mod {}: script desligado por erro", m.nome), duracao: None });
            }
        }
    }
}

/// Passa as saídas pendentes da máquina do mod para o hospedeiro.
fn recolher(m: &mut ScriptMod, registro: &mut Vec<String>, mensagens: &mut Vec<MensagemScript>) {
    if let Some(mut c) = m.lua.app_data_mut::<Contexto>() {
        registro.extend(c.registro.drain(..).map(|l| format!("[{}] {l}", m.nome)));
        mensagens.append(&mut c.mensagens);
    }
}

/// Lê os arquivos listados em `"scripts"`: caminhos relativos dentro do mod, sem `..`, terminando em
/// `.lua`.
pub fn ler_scripts_do_mod(m: &Mod) -> Result<ArquivosScript, String> {
    m.manifesto
        .scripts
        .iter()
        .map(|nome| {
            let p = Path::new(nome);
            let valido = nome.ends_with(".lua") && p.components().all(|c| matches!(c, Component::Normal(_)));
            if !valido {
                return Err(format!("caminho de script inválido: {nome} (relativo ao mod, sem '..', terminando em .lua)"));
            }
            let caminho = m.dir.join(p);
            let codigo = std::fs::read_to_string(&caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
            Ok((nome.clone(), codigo))
        })
        .collect()
}

type ErroCarga = Box<(Option<ScriptMod>, mlua::Error)>;

fn carregar_mod(nome: &str, arquivos: &ArquivosScript, tabelas: Rc<TabelasExperimental>, limites: Limites) -> Result<ScriptMod, ErroCarga> {
    let (lua, orcamento) = nova_maquina(nome, tabelas, limites).map_err(|e| Box::new((None, e)))?;
    let mut m = ScriptMod { nome: nome.to_string(), lua, ativo: true, ganchos: [false; 4], orcamento };
    for (arquivo, codigo) in arquivos {
        m.orcamento.reiniciar();
        let r = m.lua.load(codigo.as_str()).set_name(format!("@{nome}/{arquivo}")).set_mode(ChunkMode::Text).exec();
        if let Err(e) = r {
            m.ativo = false;
            return Err(Box::new((Some(m), e)));
        }
    }
    m.ganchos = procurar_ganchos(&m.lua);
    Ok(m)
}

fn procurar_ganchos(lua: &Lua) -> [bool; 4] {
    let g = lua.globals();
    Gancho::TODOS.map(|k| matches!(g.raw_get::<Value>(k.nome()), Ok(Value::Function(_))))
}

/// Máquina Lua com a sandbox, os limites e a API.
fn nova_maquina(nome: &str, tabelas: Rc<TabelasExperimental>, limites: Limites) -> mlua::Result<(Lua, Rc<Orcamento>)> {
    let lua = Lua::new_with(StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8, LuaOptions::default())?;
    lua.set_app_data(Contexto { sorteio: semente_do_nome(nome), registro: Vec::new(), mensagens: Vec::new(), experimental: tabelas });
    let g = lua.globals();

    // Sandbox: nada que leia arquivos, carregue código binário ou mexa no coletor/console.
    for n in ["dofile", "loadfile", "load", "require", "collectgarbage", "print", "warn"] {
        g.raw_set(n, Value::Nil)?;
    }
    let string: Table = g.get("string")?;
    string.raw_set("dump", Value::Nil)?;

    // Funções globais da API.
    let registrar = lua.create_function(|lua, args: Variadic<Value>| {
        let tostring: Function = lua.globals().get("tostring")?;
        let partes = args.into_iter().map(|v| tostring.call::<String>(v)).collect::<mlua::Result<Vec<_>>>()?;
        if let Some(mut c) = lua.app_data_mut::<Contexto>() {
            if c.registro.len() < MAX_REGISTRO {
                c.registro.push(partes.join("\t"));
            }
        }
        Ok(())
    })?;
    g.set("registrar", registrar.clone())?;
    g.set("print", registrar)?;
    g.set(
        "mostrar_mensagem",
        lua.create_function(|lua, (texto, duracao): (String, Option<i64>)| {
            let texto: String = texto.chars().take(MAX_TEXTO_MENSAGEM).collect();
            if texto.trim().is_empty() {
                return Err(mlua::Error::runtime("mostrar_mensagem: texto vazio"));
            }
            let duracao = match duracao {
                None => None,
                Some(d @ 1..=10_000) => Some(d as u32),
                Some(d) => return Err(mlua::Error::runtime(format!("mostrar_mensagem: duração fora de 1..10000: {d}"))),
            };
            if let Some(mut c) = lua.app_data_mut::<Contexto>() {
                c.mensagens.push(MensagemScript { texto, duracao });
            }
            Ok(())
        })?,
    )?;
    let aleatorio = lua.create_function(|lua, (a, b): (Option<i64>, Option<i64>)| {
        let mut c = lua.app_data_mut::<Contexto>().ok_or_else(|| mlua::Error::runtime("sem contexto"))?;
        let x = c.proximo();
        let (lo, hi) = match (a, b) {
            (None, _) => return Ok(Value::Number(f64::from(x) / (f64::from(u32::MAX) + 1.0))),
            (Some(n), None) => (1, n),
            (Some(a), Some(b)) => (a, b),
        };
        if lo > hi {
            return Err(mlua::Error::runtime(format!("aleatorio: intervalo vazio ({lo}, {hi})")));
        }
        let largura = (hi - lo) as u64 + 1;
        Ok(Value::Integer(lo + (u64::from(x) % largura) as i64))
    })?;
    let semear = lua.create_function(|lua, n: i64| {
        if let Some(mut c) = lua.app_data_mut::<Contexto>() {
            c.sorteio = (n as u32).max(1);
        }
        Ok(())
    })?;
    g.set("aleatorio", aleatorio.clone())?;
    g.set("semear_aleatorio", semear.clone())?;
    let math: Table = g.get("math")?;
    math.raw_set("random", aleatorio)?;
    math.raw_set("randomseed", semear)?;
    g.set("API_VERSAO", VERSAO_API)?;
    g.set("MOD_NOME", nome)?;

    lua.register_userdata_type::<Estado>(registrar_api)?;

    // Limites: memória total; instruções e tempo por chamada.
    lua.set_memory_limit(limites.memoria)?;
    let orcamento =
        Rc::new(Orcamento { instrucoes: Cell::new(0), inicio: Cell::new(Instant::now()), estourou: Cell::new(false) });
    let o = orcamento.clone();
    lua.set_hook(HookTriggers::new().every_nth_instruction(INTERVALO_CONFERENCIA), move |_, _| {
        let n = o.instrucoes.get() + u64::from(INTERVALO_CONFERENCIA);
        o.instrucoes.set(n);
        if n > limites.instrucoes || o.inicio.get().elapsed() > limites.tempo {
            o.estourou.set(true);
        }
        if o.estourou.get() {
            return Err(mlua::Error::runtime(format!(
                "limite excedido ({} instruções ou {} ms por chamada)",
                limites.instrucoes,
                limites.tempo.as_millis()
            )));
        }
        Ok(VmState::Continue)
    })?;
    // `pcall`/`xpcall` não podem engolir o erro de limite: depois de estourar, o erro sobe.
    let o = orcamento.clone();
    let estourou = lua.create_function(move |_, ()| Ok(o.estourou.get()))?;
    lua.load(
        r#"
        local _pcall, _xpcall, estourou, pack, unpack, error = ...
        local function repassar(r)
          if not r[1] and estourou() then error(r[2], 0) end
          return unpack(r, 1, r.n)
        end
        pcall = function(f, ...) return repassar(pack(_pcall(f, ...))) end
        xpcall = function(f, h, ...) return repassar(pack(_xpcall(f, h, ...))) end
        "#,
    )
    .set_name("=sandbox")
    .call::<()>((
        g.get::<Function>("pcall")?,
        g.get::<Function>("xpcall")?,
        estourou,
        lua.globals().get::<Table>("table")?.get::<Function>("pack")?,
        lua.globals().get::<Table>("table")?.get::<Function>("unpack")?,
        g.get::<Function>("error")?,
    ))?;
    Ok((lua, orcamento))
}

// ---------------------------------------------------------------------------------------------
// API do objeto `estado`
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Plano {
    Superficie,
    NinhoPreto,
    NinhoVermelho,
}

impl Plano {
    fn de_nome(nome: &str) -> mlua::Result<Plano> {
        match nome {
            "superficie" => Ok(Plano::Superficie),
            "ninho_preto" => Ok(Plano::NinhoPreto),
            "ninho_vermelho" => Ok(Plano::NinhoVermelho),
            _ => Err(mlua::Error::runtime(format!(
                "plano desconhecido: {nome} (use \"superficie\", \"ninho_preto\" ou \"ninho_vermelho\")"
            ))),
        }
    }

    /// Plano no sentido do original (`_MePlane`, `_SetMap`): 1 superfície, 2 ninho preto, 3 vermelho.
    fn numero(self) -> i32 {
        match self {
            Plano::Superficie => 1,
            Plano::NinhoPreto => 2,
            Plano::NinhoVermelho => 3,
        }
    }

    fn do_numero(n: i16) -> Option<&'static str> {
        match n {
            0 | 1 => Some("superficie"),
            2 => Some("ninho_preto"),
            3 => Some("ninho_vermelho"),
            _ => None,
        }
    }

    fn mapa(self, e: &Estado) -> &Grade {
        match self {
            Plano::Superficie => &e.superficie.mapa,
            Plano::NinhoPreto => &e.ninho_b.mapa,
            Plano::NinhoVermelho => &e.ninho_r.mapa,
        }
    }

    fn vida(self, e: &Estado) -> &Grade {
        match self {
            Plano::Superficie => &e.superficie.vida,
            Plano::NinhoPreto => &e.ninho_b.vida,
            Plano::NinhoVermelho => &e.ninho_r.vida,
        }
    }

    fn lista(self) -> Lista {
        match self {
            Plano::Superficie => Lista::A,
            Plano::NinhoPreto => Lista::B,
            Plano::NinhoVermelho => Lista::R,
        }
    }
}

fn cor_de_nome(nome: &str) -> mlua::Result<Cor> {
    match nome {
        "preta" => Ok(Cor::Preta),
        "vermelha" => Ok(Cor::Vermelha),
        _ => Err(mlua::Error::runtime(format!("cor desconhecida: {nome} (use \"preta\" ou \"vermelha\")"))),
    }
}

fn nome_da_cor(c: Cor) -> &'static str {
    match c {
        Cor::Preta => "preta",
        Cor::Vermelha => "vermelha",
    }
}

/// Coordenada vinda do Lua para i32 (fora de i32 vira um valor que nenhum mapa contém).
fn coord(v: i64) -> i32 {
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn registrar_api(reg: &mut mlua::UserDataRegistry<Estado>) {
    // --- leitura ---
    reg.add_method("tempo", |_, e, ()| Ok(e.partida.tempo));
    reg.add_method("ciclo", |_, e, ()| Ok(e.partida.ciclo));
    reg.add_method("tipo_jogo", |_, e, ()| {
        Ok(match e.partida.tipo_jogo {
            0 => "tutorial",
            1 => "rapido",
            2 => "completo",
            3 => "experimental",
            _ => "desconhecido",
        })
    });
    reg.add_method("velocidade", |_, e, ()| Ok(e.partida.velocidade));
    reg.add_method("pausado", |_, e, ()| Ok(e.partida.pausado != 0));
    reg.add_method("fim_de_jogo", |_, e, ()| Ok(e.partida.fim_de_jogo != 0));
    reg.add_method("comida_superficie", |_, e, ()| Ok(e.ambiente.comida_superficie));
    reg.add_method("colonia", |lua, e, cor: String| {
        let c = e.colonia(cor_de_nome(&cor)?);
        let t = lua.create_table()?;
        t.set("saude", c.saude)?;
        t.set("comida", c.comida)?;
        t.set("adultos", c.populacao)?;
        for (i, nome) in ["ovos", "operarias", "soldados", "machos", "rainhas_novas", "rainha"].into_iter().enumerate() {
            t.set(nome, c.castas[i])?;
        }
        t.set("modo_estrategico", c.modo_estrategico)?;
        Ok(t)
    });
    reg.add_method("amarela", |lua, e, ()| {
        let a = &e.amarela;
        let t = lua.create_table()?;
        t.set("plano", Plano::do_numero(a.plano))?;
        t.set("x", a.x)?;
        t.set("y", a.y)?;
        t.set("saude", a.saude)?;
        t.set("cor", if a.cor != 0 { "vermelha" } else { "preta" })?;
        t.set("morreu", a.morreu != 0)?;
        Ok(t)
    });
    reg.add_method("dimensoes", |_, e, plano: String| {
        let g = Plano::de_nome(&plano)?.mapa(e);
        Ok((g.largura, g.altura))
    });
    reg.add_method("celula", |_, e, (plano, x, y): (String, i64, i64)| {
        Ok(Plano::de_nome(&plano)?.mapa(e).em(coord(x), coord(y)))
    });
    reg.add_method("ocupante", |_, e, (plano, x, y): (String, i64, i64)| {
        Ok(Plano::de_nome(&plano)?.vida(e).em(coord(x), coord(y)))
    });
    reg.add_method("formigas", |lua, e, plano: String| {
        let lista = e.lista(Plano::de_nome(&plano)?.lista());
        let t = lua.create_table()?;
        for (_, f) in lista.ativas() {
            let item = lua.create_table()?;
            item.set("x", f.x)?;
            item.set("y", f.y)?;
            item.set("tipo", f.tipo)?;
            item.set("modo", f.modo)?;
            item.set("cor", nome_da_cor(cor_do_tipo(f.tipo)))?;
            item.set("grupo", (f.tipo & 0x78) >> 3)?;
            item.set("direcao", f.tipo & 7)?;
            t.raw_push(item)?;
        }
        Ok(t)
    });

    // --- escrita (validada) ---
    reg.add_method_mut("colocar_comida", |lua, e, (plano, x, y): (String, i64, i64)| {
        let p = Plano::de_nome(&plano)?;
        let (x, y) = (coord(x), coord(y));
        let tabelas = lua.app_data_ref::<Contexto>().map(|c| c.experimental.clone()).ok_or_else(|| mlua::Error::runtime("sem contexto"))?;
        let (Some(v), Some(0)) = (p.mapa(e).em(x, y), p.vida(e).em(x, y)) else {
            return Ok(false);
        };
        let r = &tabelas.regras;
        let limiar = if p == Plano::Superficie { r.comida_superficie } else { r.comida_ninho };
        let v = i32::from(v);
        // Só chão (vira comida) ou comida que ainda cabe uma unidade; outros tiles recusam.
        if v > limiar + r.comida_somar_ate || (v < limiar && v >= r.comida_chao_abaixo_de) {
            return Ok(false);
        }
        let ctx = experimental::Contexto { tabelas: &tabelas, amarela: TabelasAmarela::padrao(), inimigos: TabelasInimigos::padrao() };
        Ok(experimental::por_comida(e, ctx, p.numero(), x, y, false))
    });
    reg.add_method_mut("colocar_pedra", |lua, e, (x, y): (i64, i64)| {
        let (x, y) = (coord(x), coord(y));
        let tabelas = lua.app_data_ref::<Contexto>().map(|c| c.experimental.clone()).ok_or_else(|| mlua::Error::runtime("sem contexto"))?;
        let r = &tabelas.regras;
        match (e.superficie.mapa.em(x, y), e.superficie.vida.em(x, y)) {
            (Some(v), Some(0)) if v < r.barreira_abaixo_de => Ok(e.superficie.mapa.definir(x, y, r.pedrinha)),
            _ => Ok(false),
        }
    });
    reg.add_method_mut("colocar_formiga", |lua, e, (cor, x, y): (String, i64, i64)| {
        let cor = cor_de_nome(&cor)?;
        let (x, y) = (coord(x), coord(y));
        let tabelas = lua.app_data_ref::<Contexto>().map(|c| c.experimental.clone()).ok_or_else(|| mlua::Error::runtime("sem contexto"))?;
        let r = &tabelas.regras;
        if e.superficie.vida.em(x, y) != Some(0) || !comum::celula_livre(e, 1, x, y) {
            return Ok(false);
        }
        let tipo = if cor == Cor::Vermelha { r.formiga_tipo.wrapping_add(r.formiga_vermelha) } else { r.formiga_tipo };
        let f = Formiga { x: x as u8, y: y as u8, modo: r.formiga_modo_superficie, tipo, s: 0 };
        Ok(e.adicionar_formiga(Lista::A, f))
    });
    reg.add_method_mut("definir_velocidade", |_, e, v: i64| {
        if !(0..=3).contains(&v) {
            return Err(mlua::Error::runtime(format!("definir_velocidade: use 0..3, não {v}")));
        }
        e.partida.velocidade = v as i16;
        Ok(())
    });
}

/// O evento como tabela Lua (`tipo` e os campos do pedido).
fn tabela_evento(lua: &Lua, ev: &Evento) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    match ev {
        Evento::Mensagem(id) => {
            t.set("tipo", "mensagem")?;
            t.set("id", *id)?;
        }
        Evento::Musica(id) => {
            t.set("tipo", "musica")?;
            t.set("id", *id)?;
        }
        Evento::Som(id) => {
            t.set("tipo", "som")?;
            t.set("id", *id)?;
        }
        Evento::TemaDoNinho => t.set("tipo", "tema_do_ninho")?,
        Evento::DialogoFimDeJogo => t.set("tipo", "fim_de_jogo")?,
        Evento::CentralizarNaAmarela => t.set("tipo", "centralizar_na_amarela")?,
        Evento::MensagemFaixa(m) => {
            t.set("tipo", "mensagem_faixa")?;
            t.set("lista", m.lista)?;
            t.set("indice", m.indice)?;
        }
        Evento::Espera(ms) => {
            t.set("tipo", "espera")?;
            t.set("ms", *ms)?;
        }
    }
    Ok(t)
}
