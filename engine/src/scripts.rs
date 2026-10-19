//! Scripts Lua dos mods: carregamento, sandbox, limites e a API pública.
//! Contrato (o que um script pode fazer): docs/api-mods.md, seção "Scripts Lua" (API 0.12; menu,
//! painel, ações e avanço de ticks desde a 0.17).
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
//!
//! Ações (API 0.17): os métodos de ação do `estado` e `avancar_ticks` só validam os argumentos e
//! guardam um [`PedidoMod`]; quem os executa, logo depois que o gancho termina e na ordem em que
//! foram feitos, é `core::acoes_mods` (com as tabelas das regras). Cada ação chama a rotina do
//! original correspondente; nenhuma sorteia além do que a rotina sorteia.

use std::cell::Cell;
use std::path::{Component, Path};
use std::rc::Rc;
use std::time::{Duration, Instant};

use mlua::{ChunkMode, Function, HookTriggers, Lua, LuaOptions, StdLib, Table, UserDataMethods, Value, Variadic, VmState};

use crate::formats::save::{Formiga, Lista};
use crate::recursos::{Mod, Recursos};
use crate::rules::amarela::TabelasAmarela;
use crate::rules::comum;
use crate::core::navegacao::Evento as EventoTecla;
use crate::rules::estado::{cor_do_tipo, Estado, Evento, Grade};
use crate::rules::experimental::{self, TabelasExperimental};
use crate::rules::geracao::Cor;
use crate::rules::inimigos::TabelasInimigos;

/// Versão da API de mods que os scripts veem em `API_VERSAO`.
pub const VERSAO_API: &str = "0.25";

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
    /// `ao_menu(indice, texto, estado)` (API 0.17): o jogador escolheu um item do menu do mod.
    AoMenu,
}

const N_GANCHOS: usize = 5;

impl Gancho {
    pub const TODOS: [Gancho; N_GANCHOS] =
        [Gancho::AoIniciar, Gancho::AntesDoTick, Gancho::DepoisDoTick, Gancho::AoEvento, Gancho::AoMenu];

    pub fn nome(self) -> &'static str {
        match self {
            Gancho::AoIniciar => "ao_iniciar",
            Gancho::AntesDoTick => "antes_do_tick",
            Gancho::DepoisDoTick => "depois_do_tick",
            Gancho::AoEvento => "ao_evento",
            Gancho::AoMenu => "ao_menu",
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

/// Itens de um menu de mod (API 0.17).
pub const MAX_ITENS_MENU: usize = 40;
/// Caracteres do título e de cada item do menu.
pub const MAX_TEXTO_ITEM: usize = 40;
/// Linhas e caracteres por linha do painel de um mod.
pub const MAX_LINHAS_PAINEL: usize = 16;
pub const MAX_TEXTO_PAINEL: usize = 60;
/// Ticks pedidos de uma vez por `avancar_ticks`.
pub const MAX_TICKS_PEDIDOS: i64 = 10_000;
/// Pedidos de ação por chamada de gancho.
pub const MAX_PEDIDOS: usize = 200;

/// Tecla que abre o menu de um mod: nome da tecla no SDL e modificadores.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeclaMod {
    pub nome: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl TeclaMod {
    /// "F12", "Ctrl+Shift+T", "Alt+F11"... Exige uma tecla de função (F1–F24) ou Ctrl/Alt, para não
    /// tirar do jogo as letras e números (comandos da amarela e códigos de trapaça).
    pub fn de_texto(texto: &str) -> Result<TeclaMod, String> {
        let partes: Vec<&str> = texto.split('+').map(str::trim).collect();
        let Some((nome, mods)) = partes.split_last() else { return Err("tecla vazia".into()) };
        let mut t = TeclaMod { nome: nome.to_string(), ctrl: false, shift: false, alt: false };
        for m in mods {
            match m.to_ascii_lowercase().as_str() {
                "ctrl" => t.ctrl = true,
                "shift" => t.shift = true,
                "alt" => t.alt = true,
                outro => return Err(format!("modificador desconhecido: {outro} (use Ctrl, Shift ou Alt)")),
            }
        }
        let funcao = nome.len() >= 2
            && nome.as_bytes()[0].eq_ignore_ascii_case(&b'F')
            && nome[1..].parse::<u8>().is_ok_and(|n| (1..=24).contains(&n));
        if nome.is_empty() || !(funcao || t.ctrl || t.alt) {
            return Err(format!("tecla \"{texto}\": use uma tecla de função (F1–F24) ou uma combinação com Ctrl ou Alt"));
        }
        Ok(t)
    }

    /// A tecla apertada é esta?
    pub fn casa(&self, ev: &EventoTecla) -> bool {
        let EventoTecla::Tecla { nome, ctrl, shift, alt } = ev;
        self.nome.eq_ignore_ascii_case(nome) && self.ctrl == *ctrl && self.shift == *shift && self.alt == *alt
    }

    /// Como o jogador lê a tecla ("Ctrl+Shift+T").
    pub fn texto(&self) -> String {
        let mut s = String::new();
        for (ligado, nome) in [(self.ctrl, "Ctrl+"), (self.shift, "Shift+"), (self.alt, "Alt+")] {
            if ligado {
                s.push_str(nome);
            }
        }
        s + &self.nome
    }
}

/// Menu de um mod (`definir_menu`): título, tecla e itens ("-" = separador).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuMod {
    pub titulo: String,
    pub tecla: TeclaMod,
    pub itens: Vec<String>,
}

/// Casta de uma formiga posta por `colocar_formiga`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastaMod {
    Operaria,
    Soldado,
}

/// Ação pedida por um script, executada por `core::acoes_mods` logo depois do gancho (API 0.17).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PedidoMod {
    ComecarChuva,
    PararChuva,
    MeninoCortarGrama,
    MeninoInseticida,
    CriarReprodutores { cor: Cor, n: i32 },
    ArmarRevoada(Cor),
    GuardarRainhas { cor: Cor, n: i16 },
    PovoarPatch { cor: Cor, x: i16, y: i16, n: u8 },
    TrocarPatch { x: i16, y: i16 },
    TrazerGato,
    Latir,
    TrazerAranha,
    PorFormigaLeao { x: i32, y: i32 },
    SaudeAmarela(i32),
    MatarAmarela,
    SaudeColonia { cor: Cor, v: i16 },
    AvancarLicao,
    TerminarJogo { vitoria: bool },
    Misterio,
    AvancarTicks(u32),
    PararAvanco,
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
    pedidos: Vec<PedidoMod>,
    menu: Option<MenuMod>,
    painel: Option<Vec<String>>,
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
    ganchos: [bool; N_GANCHOS],
    orcamento: Rc<Orcamento>,
}

/// Os scripts de todos os mods ativos, na ordem dos mods.
pub struct Scripts {
    mods: Vec<ScriptMod>,
    registro: Vec<String>,
    mensagens: Vec<MensagemScript>,
    pedidos: Vec<(String, PedidoMod)>,
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
        let mut s = Scripts { mods: Vec::new(), registro: Vec::new(), mensagens: Vec::new(), pedidos: Vec::new() };
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
                    recolher(&mut s.mods[i], &mut s.registro, &mut s.mensagens, &mut s.pedidos);
                }
                Err(erro) => {
                    let (m, e) = *erro;
                    if let Some(mut m) = m {
                        recolher(&mut m, &mut s.registro, &mut s.mensagens, &mut s.pedidos);
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

    /// Pedidos de ação dos scripts (API 0.17), com o nome do mod, na ordem em que foram feitos.
    pub fn retirar_pedidos(&mut self) -> Vec<(String, PedidoMod)> {
        std::mem::take(&mut self.pedidos)
    }

    /// Menus definidos pelos mods ativos (`definir_menu`), com o nome do mod.
    pub fn menus(&self) -> Vec<(String, MenuMod)> {
        self.mods
            .iter()
            .filter(|m| m.ativo)
            .filter_map(|m| Some((m.nome.clone(), m.lua.app_data_ref::<Contexto>()?.menu.clone()?)))
            .collect()
    }

    /// Painéis de texto dos mods ativos (`definir_painel`), com o nome do mod.
    pub fn paineis(&self) -> Vec<(String, Vec<String>)> {
        self.mods
            .iter()
            .filter(|m| m.ativo)
            .filter_map(|m| Some((m.nome.clone(), m.lua.app_data_ref::<Contexto>()?.painel.clone()?)))
            .collect()
    }

    /// `ao_menu(indice, texto, estado)` do mod `nome` (índice a partir de 1, contando os
    /// separadores). Nada se o mod não define o gancho ou não está ativo.
    pub fn ao_menu(&mut self, nome: &str, indice: usize, e: &mut Estado) {
        let Some(i) = self.mods.iter().position(|m| m.nome == nome && m.ativo && m.ganchos[Gancho::AoMenu.indice()]) else {
            return;
        };
        let texto = self.mods[i]
            .lua
            .app_data_ref::<Contexto>()
            .and_then(|c| c.menu.as_ref()?.itens.get(indice.wrapping_sub(1)).cloned())
            .unwrap_or_default();
        let Scripts { mods, registro, mensagens, pedidos } = self;
        chamar_mod(&mut mods[i], Gancho::AoMenu, e, (registro, mensagens, pedidos), |lua, ud| {
            let f: Function = lua.globals().get(Gancho::AoMenu.nome())?;
            f.call::<()>((indice, texto.as_str(), ud))
        });
    }

    fn chamar(&mut self, g: Gancho, e: &mut Estado, ev: Option<&Evento>) {
        let Scripts { mods, registro, mensagens, pedidos } = self;
        for m in mods.iter_mut().filter(|m| m.ativo && m.ganchos[g.indice()]) {
            chamar_mod(m, g, e, (&mut *registro, &mut *mensagens, &mut *pedidos), |lua, ud| {
                let f: Function = lua.globals().get(g.nome())?;
                match ev {
                    Some(ev) => f.call::<()>((tabela_evento(lua, ev)?, ud)),
                    None => f.call::<()>(ud),
                }
            });
        }
    }
}

type Saidas<'s> = (&'s mut Vec<String>, &'s mut Vec<MensagemScript>, &'s mut Vec<(String, PedidoMod)>);

/// Uma chamada de gancho num mod, com o `estado` válido só durante ela; um erro desliga o mod e
/// descarta os pedidos de ação daquela chamada.
fn chamar_mod(m: &mut ScriptMod, g: Gancho, e: &mut Estado, saidas: Saidas, chamada: impl FnOnce(&Lua, mlua::AnyUserData) -> mlua::Result<()>) {
    let (registro, mensagens, pedidos) = saidas;
    m.orcamento.reiniciar();
    let lua = m.lua.clone();
    let r = lua.scope(|s| {
        let ud = s.create_any_userdata_ref_mut(&mut *e)?;
        chamada(&lua, ud)
    });
    if r.is_err() {
        if let Some(mut c) = m.lua.app_data_mut::<Contexto>() {
            c.pedidos.clear();
        }
    }
    recolher(m, registro, mensagens, pedidos);
    if let Err(err) = r {
        m.ativo = false;
        m.ganchos = [false; N_GANCHOS];
        registro.push(format!("[{}] script desligado em {}: {err}", m.nome, g.nome()));
        mensagens.push(MensagemScript { texto: format!("Mod {}: script desligado por erro", m.nome), duracao: None });
    }
}

/// Passa as saídas pendentes da máquina do mod para o hospedeiro.
fn recolher(m: &mut ScriptMod, registro: &mut Vec<String>, mensagens: &mut Vec<MensagemScript>, pedidos: &mut Vec<(String, PedidoMod)>) {
    if let Some(mut c) = m.lua.app_data_mut::<Contexto>() {
        registro.extend(c.registro.drain(..).map(|l| format!("[{}] {l}", m.nome)));
        mensagens.append(&mut c.mensagens);
        pedidos.extend(c.pedidos.drain(..).map(|p| (m.nome.clone(), p)));
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
    let mut m = ScriptMod { nome: nome.to_string(), lua, ativo: true, ganchos: [false; N_GANCHOS], orcamento };
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

fn procurar_ganchos(lua: &Lua) -> [bool; N_GANCHOS] {
    let g = lua.globals();
    Gancho::TODOS.map(|k| matches!(g.raw_get::<Value>(k.nome()), Ok(Value::Function(_))))
}

/// Máquina Lua com a sandbox, os limites e a API.
fn nova_maquina(nome: &str, tabelas: Rc<TabelasExperimental>, limites: Limites) -> mlua::Result<(Lua, Rc<Orcamento>)> {
    let lua = Lua::new_with(StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8, LuaOptions::default())?;
    lua.set_app_data(Contexto {
        sorteio: semente_do_nome(nome),
        registro: Vec::new(),
        mensagens: Vec::new(),
        experimental: tabelas,
        pedidos: Vec::new(),
        menu: None,
        painel: None,
    });
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
    registrar_globais_017(&lua, &g)?;
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
        t.set("rainhas_guardadas", c.rainhas_guardadas)?;
        t.set("rainhas_vivas", c.rainhas)?;
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
    reg.add_method_mut("colocar_formiga", |lua, e, (cor, x, y, casta): (String, i64, i64, Option<String>)| {
        let cor = cor_de_nome(&cor)?;
        let casta = casta_de_nome(casta.as_deref())?;
        let (x, y) = (coord(x), coord(y));
        let tabelas = lua.app_data_ref::<Contexto>().map(|c| c.experimental.clone()).ok_or_else(|| mlua::Error::runtime("sem contexto"))?;
        let r = &tabelas.regras;
        if e.superficie.vida.em(x, y) != Some(0) || !comum::celula_livre(e, 1, x, y) {
            return Ok(false);
        }
        let base = if casta == CastaMod::Soldado { TIPO_SOLDADO } else { r.formiga_tipo };
        let tipo = if cor == Cor::Vermelha { base.wrapping_add(r.formiga_vermelha) } else { base };
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
    registrar_api_017(reg);
}

// ---------------------------------------------------------------------------------------------
// API 0.17: menu, painel, ações (pedidos) e leituras para depuração
// ---------------------------------------------------------------------------------------------

/// Tipo T de um soldado (grupo 6, formigas.md; o mesmo de `_AddBlackAnts`).
const TIPO_SOLDADO: u8 = 0x30;

fn erro(texto: String) -> mlua::Error {
    mlua::Error::runtime(texto)
}

/// Guarda um pedido de ação (devolve `true`: aceito para execução).
fn pedir(lua: &Lua, p: PedidoMod) -> mlua::Result<bool> {
    let mut c = lua.app_data_mut::<Contexto>().ok_or_else(|| erro("sem contexto".into()))?;
    if c.pedidos.len() >= MAX_PEDIDOS {
        return Err(erro(format!("mais de {MAX_PEDIDOS} ações numa só chamada")));
    }
    c.pedidos.push(p);
    Ok(true)
}

fn texto_curto(t: &str, max: usize) -> String {
    t.chars().take(max).collect()
}

/// Patch do quintal (x 0..11, y 0..15).
fn patch(funcao: &str, x: i64, y: i64) -> mlua::Result<(i16, i16)> {
    if !(0..12).contains(&x) || !(0..16).contains(&y) {
        return Err(erro(format!("{funcao}: patch fora do quintal ({x}, {y}); use x 0..11 e y 0..15")));
    }
    Ok((x as i16, y as i16))
}

fn faixa(funcao: &str, nome: &str, v: i64, min: i64, max: i64) -> mlua::Result<i64> {
    if !(min..=max).contains(&v) {
        return Err(erro(format!("{funcao}: {nome} fora de {min}..{max}: {v}")));
    }
    Ok(v)
}

/// Funções globais da 0.17: `definir_menu`, `definir_painel`, `avancar_ticks`, `parar_avanco`.
fn registrar_globais_017(lua: &Lua, g: &Table) -> mlua::Result<()> {
    g.set(
        "definir_menu",
        lua.create_function(|lua, t: Option<Table>| {
            let menu = match t {
                None => None,
                Some(t) => {
                    let titulo: String = t.get::<Option<String>>("titulo")?.unwrap_or_default();
                    let tecla: String = t.get::<Option<String>>("tecla")?.ok_or_else(|| erro("definir_menu: falta a tecla".into()))?;
                    let tecla = TeclaMod::de_texto(&tecla).map_err(|e| erro(format!("definir_menu: {e}")))?;
                    let itens: Vec<String> = t.get::<Option<Vec<String>>>("itens")?.unwrap_or_default();
                    if itens.is_empty() || itens.len() > MAX_ITENS_MENU {
                        return Err(erro(format!("definir_menu: de 1 a {MAX_ITENS_MENU} itens, não {}", itens.len())));
                    }
                    let itens = itens.iter().map(|i| texto_curto(i, MAX_TEXTO_ITEM)).collect();
                    Some(MenuMod { titulo: texto_curto(&titulo, MAX_TEXTO_ITEM), tecla, itens })
                }
            };
            if let Some(mut c) = lua.app_data_mut::<Contexto>() {
                c.menu = menu;
            }
            Ok(())
        })?,
    )?;
    g.set(
        "definir_painel",
        lua.create_function(|lua, texto: Option<String>| {
            let painel = texto.map(|t| {
                t.lines().take(MAX_LINHAS_PAINEL).map(|l| texto_curto(l, MAX_TEXTO_PAINEL)).collect::<Vec<_>>()
            });
            if let Some(mut c) = lua.app_data_mut::<Contexto>() {
                c.painel = painel;
            }
            Ok(())
        })?,
    )?;
    g.set(
        "avancar_ticks",
        lua.create_function(|lua, n: i64| {
            let n = faixa("avancar_ticks", "n", n, 1, MAX_TICKS_PEDIDOS)?;
            pedir(lua, PedidoMod::AvancarTicks(n as u32))
        })?,
    )?;
    g.set("parar_avanco", lua.create_function(|lua, ()| pedir(lua, PedidoMod::PararAvanco))?)?;
    Ok(())
}

/// Métodos do `estado` da 0.17.
fn registrar_api_017(reg: &mut mlua::UserDataRegistry<Estado>) {
    // --- leitura ---
    reg.add_method("sementes", |_, e, ()| Ok((e.s.semente(), e.r.estado())));
    reg.add_method("chuva", |lua, e, ()| {
        let t = lua.create_table()?;
        let a = &e.ambiente;
        t.set("chovendo", a.chovendo != 0)?;
        t.set("contagem", a.chuva_contagem)?;
        t.set("lencol", a.lencol)?;
        Ok(t)
    });
    reg.add_method("revoada", |_, e, cor: String| Ok(e.colonia(cor_de_nome(&cor)?).revoada));
    reg.add_method("menino", |lua, e, ()| {
        let m = &e.menino;
        let t = lua.create_table()?;
        t.set("dentro_de_casa", m.no_numero < 0x26)?;
        t.set("no", m.no_numero)?;
        t.set("presente", m.presente)?;
        t.set("cortando_grama", m.cortando_grama != 0)?;
        t.set("px", m.px)?;
        t.set("py", m.py)?;
        Ok(t)
    });
    reg.add_method("patch", |_, e, ()| Ok(e.ambiente.patch));
    reg.add_method("quintal", |_, e, (cor, x, y): (String, i64, i64)| {
        let (x, y) = patch("quintal", x, y)?;
        Ok(e.colonia(cor_de_nome(&cor)?).populacao_quintal.0[(x * 16 + y) as usize])
    });
    reg.add_method("licao", |_, e, ()| Ok((e.tutorial.licao, e.tutorial.dada != 0)));

    // --- ações (pedidos executados logo depois do gancho) ---
    reg.add_method("comecar_chuva", |lua, _, ()| pedir(lua, PedidoMod::ComecarChuva));
    reg.add_method("parar_chuva", |lua, _, ()| pedir(lua, PedidoMod::PararChuva));
    reg.add_method("menino_cortar_grama", |lua, _, ()| pedir(lua, PedidoMod::MeninoCortarGrama));
    reg.add_method("menino_inseticida", |lua, _, ()| pedir(lua, PedidoMod::MeninoInseticida));
    reg.add_method("criar_reprodutores", |lua, _, (cor, n): (String, i64)| {
        let n = faixa("criar_reprodutores", "n", n, 1, 200)? as i32;
        pedir(lua, PedidoMod::CriarReprodutores { cor: cor_de_nome(&cor)?, n })
    });
    reg.add_method("armar_revoada", |lua, _, cor: String| pedir(lua, PedidoMod::ArmarRevoada(cor_de_nome(&cor)?)));
    reg.add_method("guardar_rainhas", |lua, _, (cor, n): (String, i64)| {
        let n = faixa("guardar_rainhas", "n", n, 1, 100)? as i16;
        pedir(lua, PedidoMod::GuardarRainhas { cor: cor_de_nome(&cor)?, n })
    });
    reg.add_method("povoar_patch", |lua, _, (cor, x, y, n): (String, i64, i64, i64)| {
        let (x, y) = patch("povoar_patch", x, y)?;
        let n = faixa("povoar_patch", "n", n, 0, 255)? as u8;
        pedir(lua, PedidoMod::PovoarPatch { cor: cor_de_nome(&cor)?, x, y, n })
    });
    reg.add_method("trocar_patch", |lua, _, (x, y): (i64, i64)| {
        let (x, y) = patch("trocar_patch", x, y)?;
        pedir(lua, PedidoMod::TrocarPatch { x, y })
    });
    reg.add_method("trazer_gato", |lua, _, ()| pedir(lua, PedidoMod::TrazerGato));
    reg.add_method("latir", |lua, _, ()| pedir(lua, PedidoMod::Latir));
    reg.add_method("trazer_aranha", |lua, _, ()| pedir(lua, PedidoMod::TrazerAranha));
    reg.add_method("por_formiga_leao", |lua, _, (x, y): (i64, i64)| {
        let x = faixa("por_formiga_leao", "x", x, 1, 126)? as i32;
        let y = faixa("por_formiga_leao", "y", y, 1, 62)? as i32;
        pedir(lua, PedidoMod::PorFormigaLeao { x, y })
    });
    reg.add_method("saude_amarela", |lua, _, v: i64| {
        let v = faixa("saude_amarela", "valor", v, 0, 100)? as i32;
        pedir(lua, PedidoMod::SaudeAmarela(v))
    });
    reg.add_method("matar_amarela", |lua, _, ()| pedir(lua, PedidoMod::MatarAmarela));
    reg.add_method("saude_colonia", |lua, _, (cor, v): (String, i64)| {
        let v = faixa("saude_colonia", "valor", v, 0, 100)? as i16;
        pedir(lua, PedidoMod::SaudeColonia { cor: cor_de_nome(&cor)?, v })
    });
    reg.add_method("avancar_licao", |lua, _, ()| pedir(lua, PedidoMod::AvancarLicao));
    reg.add_method("terminar_jogo", |lua, _, resultado: String| {
        let vitoria = match resultado.as_str() {
            "vitoria" => true,
            "derrota" => false,
            _ => return Err(erro(format!("terminar_jogo: use \"vitoria\" ou \"derrota\", não {resultado}"))),
        };
        pedir(lua, PedidoMod::TerminarJogo { vitoria })
    });
    reg.add_method("misterio", |lua, _, ()| pedir(lua, PedidoMod::Misterio));
}

fn casta_de_nome(nome: Option<&str>) -> mlua::Result<CastaMod> {
    match nome {
        None | Some("operaria") => Ok(CastaMod::Operaria),
        Some("soldado") => Ok(CastaMod::Soldado),
        Some(outro) => Err(erro(format!("casta desconhecida: {outro} (use \"operaria\" ou \"soldado\")"))),
    }
}

/// O evento como tabela Lua (`tipo` e os campos do pedido).
fn tabela_evento(lua: &Lua, ev: &Evento) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    match ev {
        Evento::Mensagem(id) => {
            t.set("tipo", "mensagem")?;
            t.set("id", *id)?;
        }
        Evento::EscolherColonia => t.set("tipo", "escolher_colonia")?,
        Evento::DialogoAranha { texto } => {
            t.set("tipo", "dialogo_aranha")?;
            t.set("texto", *texto)?;
        }
        Evento::QuadroAmarela { imagem, indice } => {
            t.set("tipo", "quadro_amarela")?;
            t.set("imagem", *imagem)?;
            t.set("indice", *indice)?;
        }
        Evento::QuadroFormigaLeao => t.set("tipo", "quadro_formiga_leao")?,
        Evento::AnimacaoVitoria => t.set("tipo", "animacao_vitoria")?,
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
        Evento::CentralizarNaAmarela { plano, x, y } => {
            t.set("tipo", "centralizar_na_amarela")?;
            t.set("plano", *plano)?;
            t.set("x", *x)?;
            t.set("y", *y)?;
        }
        Evento::MensagemFaixa(m) => {
            t.set("tipo", "mensagem_faixa")?;
            t.set("lista", m.lista)?;
            t.set("indice", m.indice)?;
        }
        Evento::Espera(ms) => {
            t.set("tipo", "espera")?;
            t.set("ms", *ms)?;
        }
        Evento::Animacao(a) => {
            t.set("tipo", "animacao")?;
            t.set("codigo", a.codigo)?;
            t.set("plano", a.plano)?;
            t.set("quadros", a.quadros.len())?;
            t.set("espera_ms", a.espera_ms)?;
        }
    }
    Ok(t)
}
