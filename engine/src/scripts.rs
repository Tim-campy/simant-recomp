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
//! Fidelidade: `antes_do_tick`/`depois_do_tick` rodam fora de `rules::tick::tick` (em
//! `core::partida`). Os ganchos de parte (API 0.26, `antes_<parte>`/`depois_<parte>`) rodam dentro
//! do tick pelas regras embrulhadas em [`RegrasComScripts`], entre as sub-rotinas e na ordem do
//! original; o embrulho só é usado quando algum mod define um gancho de parte. Sem scripts, nada
//! aqui é chamado.
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
use crate::rules::tick::{Regras, RegrasAmarela, RegrasConselhos, RegrasInimigos, RegrasNinho, RegrasQuintal, RegrasSuperficie};

/// Versão da API de mods que os scripts veem em `API_VERSAO`.
pub const VERSAO_API: &str = "0.36";

/// Tamanho máximo do texto de `mostrar_mensagem` (caracteres).
pub const MAX_TEXTO_MENSAGEM: usize = 120;

/// Linhas de registro guardadas por mod entre duas retiradas (o excesso é descartado).
const MAX_REGISTRO: usize = 1000;

/// A cada quantas instruções da VM o limite é conferido.
const INTERVALO_CONFERENCIA: u32 = 1000;

/// Partes do tick em que um script pode ter ganchos `antes_<parte>`/`depois_<parte>` (API 0.26).
/// Cada parte é uma sub-rotina chamada por `rules::tick::tick` (um método das traits de
/// `rules::tick`), na ordem do original (`_DoAntSim`, simulacao-visao-geral.md §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParteTick {
    /// `_DoSimYard` (menino, bichos, chuva, colônias do quintal).
    Quintal,
    /// `_DoWater`.
    Agua,
    /// `_DoAntLions`.
    FormigasLeao,
    /// `_MoveSpider`.
    Aranha,
    /// `_DoPillar` (só nos ticks com `_Cycle` ímpar).
    Lagarta,
    /// `_GetStrategy` (com `_SetCasteProd`, `_SetModeProd`).
    Estrategia,
    /// `_DoAntSimA` (lista A).
    Superficie,
    /// `_DoAntSimB`.
    NinhoPreto,
    /// `_DoAntSimR`.
    NinhoVermelho,
    /// `_DoAntSimY`.
    Amarela,
    /// `_MakeRedInitiator` (só quando nenhuma vermelha está no modo 19).
    LiderVermelha,
    /// `_DoAntMoveY`.
    MovimentoAmarela,
    /// `_Feedback` (a do tick e a do `_FullCount`, a cada 64 ticks).
    Conselhos,
}

const N_PARTES: usize = 13;

impl ParteTick {
    pub const TODAS: [ParteTick; N_PARTES] = [
        ParteTick::Quintal,
        ParteTick::Agua,
        ParteTick::FormigasLeao,
        ParteTick::Aranha,
        ParteTick::Lagarta,
        ParteTick::Estrategia,
        ParteTick::Superficie,
        ParteTick::NinhoPreto,
        ParteTick::NinhoVermelho,
        ParteTick::Amarela,
        ParteTick::LiderVermelha,
        ParteTick::MovimentoAmarela,
        ParteTick::Conselhos,
    ];

    /// Nomes dos dois ganchos da parte: (`antes_<parte>`, `depois_<parte>`).
    pub fn nomes(self) -> (&'static str, &'static str) {
        match self {
            ParteTick::Quintal => ("antes_quintal", "depois_quintal"),
            ParteTick::Agua => ("antes_agua", "depois_agua"),
            ParteTick::FormigasLeao => ("antes_formigas_leao", "depois_formigas_leao"),
            ParteTick::Aranha => ("antes_aranha", "depois_aranha"),
            ParteTick::Lagarta => ("antes_lagarta", "depois_lagarta"),
            ParteTick::Estrategia => ("antes_estrategia", "depois_estrategia"),
            ParteTick::Superficie => ("antes_superficie", "depois_superficie"),
            ParteTick::NinhoPreto => ("antes_ninho_preto", "depois_ninho_preto"),
            ParteTick::NinhoVermelho => ("antes_ninho_vermelho", "depois_ninho_vermelho"),
            ParteTick::Amarela => ("antes_amarela", "depois_amarela"),
            ParteTick::LiderVermelha => ("antes_lider_vermelha", "depois_lider_vermelha"),
            ParteTick::MovimentoAmarela => ("antes_movimento_amarela", "depois_movimento_amarela"),
            ParteTick::Conselhos => ("antes_conselhos", "depois_conselhos"),
        }
    }
}

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
    /// `ao_salvar(estado)` (API 0.26): devolve a tabela do mod que vai para o arquivo lateral do save.
    AoSalvar,
    /// `ao_carregar(dados, estado)` (API 0.26): no Load, com a tabela gravada por `ao_salvar`.
    AoCarregar,
    /// `antes_<parte>(estado)` (API 0.26): dentro do tick, logo antes da parte.
    Antes(ParteTick),
    /// `depois_<parte>(estado)` (API 0.26): dentro do tick, logo depois da parte.
    Depois(ParteTick),
}

const N_BASICOS: usize = 7;
const N_GANCHOS: usize = N_BASICOS + 2 * N_PARTES;

impl Gancho {
    /// Os ganchos que não são de parte do tick.
    pub const BASICOS: [Gancho; N_BASICOS] = [
        Gancho::AoIniciar,
        Gancho::AntesDoTick,
        Gancho::DepoisDoTick,
        Gancho::AoEvento,
        Gancho::AoMenu,
        Gancho::AoSalvar,
        Gancho::AoCarregar,
    ];

    /// Todos os ganchos, na ordem dos índices.
    pub fn todos() -> impl Iterator<Item = Gancho> {
        Gancho::BASICOS
            .into_iter()
            .chain(ParteTick::TODAS.into_iter().flat_map(|p| [Gancho::Antes(p), Gancho::Depois(p)]))
    }

    pub fn nome(self) -> &'static str {
        match self {
            Gancho::AoIniciar => "ao_iniciar",
            Gancho::AntesDoTick => "antes_do_tick",
            Gancho::DepoisDoTick => "depois_do_tick",
            Gancho::AoEvento => "ao_evento",
            Gancho::AoMenu => "ao_menu",
            Gancho::AoSalvar => "ao_salvar",
            Gancho::AoCarregar => "ao_carregar",
            Gancho::Antes(p) => p.nomes().0,
            Gancho::Depois(p) => p.nomes().1,
        }
    }

    fn indice(self) -> usize {
        match self {
            Gancho::Antes(p) => N_BASICOS + 2 * p as usize,
            Gancho::Depois(p) => N_BASICOS + 2 * p as usize + 1,
            basico => Gancho::BASICOS.iter().position(|&g| g == basico).unwrap_or(0),
        }
    }

    /// Gancho de uma parte do tick?
    fn de_parte(self) -> bool {
        matches!(self, Gancho::Antes(_) | Gancho::Depois(_))
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

/// Tabelas das regras usadas pelas escritas dos scripts (com mods): as do Experimental (comida,
/// pedra, formiga, cheiro), as da amarela (direções de `_DigTile*`) e as dos inimigos.
#[derive(Clone, Debug)]
pub struct TabelasScripts {
    pub experimental: TabelasExperimental,
    pub amarela: TabelasAmarela,
    pub inimigos: TabelasInimigos,
}

impl TabelasScripts {
    /// As tabelas pelos recursos (`data/` com mods).
    pub fn carregar(r: &Recursos) -> Result<TabelasScripts, String> {
        Ok(TabelasScripts {
            experimental: TabelasExperimental::carregar(r)?,
            amarela: TabelasAmarela::carregar(r)?,
            inimigos: TabelasInimigos::carregar(r)?,
        })
    }

    /// As do Experimental dadas e as outras embutidas (testes).
    pub fn com_experimental(experimental: TabelasExperimental) -> TabelasScripts {
        TabelasScripts { experimental, amarela: TabelasAmarela::padrao().clone(), inimigos: TabelasInimigos::padrao().clone() }
    }

    fn contexto(&self) -> experimental::Contexto<'_> {
        experimental::Contexto { tabelas: &self.experimental, amarela: &self.amarela, inimigos: &self.inimigos }
    }
}

/// O que fica dentro de cada máquina Lua (app data): nome, gerador próprio, saídas pendentes e as
/// tabelas usadas pelas escritas.
struct Contexto {
    sorteio: u32,
    registro: Vec<String>,
    mensagens: Vec<MensagemScript>,
    tabelas: Rc<TabelasScripts>,
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
        let tabelas = TabelasScripts::carregar(r)?;
        let fontes = com_scripts.into_iter().map(|m| (m.manifesto.nome.clone(), ler_scripts_do_mod(m))).collect();
        Ok(Some(Scripts::de_fontes_com(fontes, tabelas, limites)))
    }

    /// Scripts a partir do código já lido (testes e ferramentas). Cada item: nome do mod e os
    /// arquivos, ou o erro de leitura (o mod entra desligado). As tabelas da amarela e dos inimigos
    /// são as embutidas.
    pub fn de_fontes(
        fontes: Vec<(String, Result<ArquivosScript, String>)>,
        tabelas: TabelasExperimental,
        limites: Limites,
    ) -> Scripts {
        Scripts::de_fontes_com(fontes, TabelasScripts::com_experimental(tabelas), limites)
    }

    /// Como [`Scripts::de_fontes`], com todas as tabelas dadas.
    pub fn de_fontes_com(
        fontes: Vec<(String, Result<ArquivosScript, String>)>,
        tabelas: TabelasScripts,
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

    /// Algum mod ativo define um gancho de parte do tick (`antes_<parte>`/`depois_<parte>`, API 0.26)?
    pub fn tem_ganchos_do_tick(&self) -> bool {
        self.mods.iter().any(|m| m.ativo && Gancho::todos().any(|g| g.de_parte() && m.ganchos[g.indice()]))
    }

    /// `antes_<parte>(estado)` (`depois` = falso) ou `depois_<parte>(estado)`, nos mods que o definem.
    pub fn parte(&mut self, p: ParteTick, depois: bool, e: &mut Estado) {
        let g = if depois { Gancho::Depois(p) } else { Gancho::Antes(p) };
        if self.tem_gancho(g) {
            self.chamar(g, e, None);
        }
    }

    /// `ao_salvar(estado)` de cada mod ativo que o define (API 0.26): a tabela devolvida, já em JSON,
    /// com o nome do mod. `nil` = nada a guardar. Um valor que não vai para JSON é erro (desliga o
    /// mod). As ações pedidas dentro de `ao_salvar` são descartadas (salvar não muda o jogo).
    pub fn ao_salvar(&mut self, e: &mut Estado) -> Vec<(String, serde_json::Value)> {
        let mut saida = Vec::new();
        let Scripts { mods, registro, mensagens, pedidos } = self;
        for m in mods.iter_mut().filter(|m| m.ativo && m.ganchos[Gancho::AoSalvar.indice()]) {
            let antes = pedidos.len();
            let mut dados = None;
            chamar_mod(m, Gancho::AoSalvar, e, (&mut *registro, &mut *mensagens, &mut *pedidos), |lua, ud| {
                let f: Function = lua.globals().get(Gancho::AoSalvar.nome())?;
                let v: Value = f.call(ud)?;
                if v.is_nil() {
                    return Ok(());
                }
                let j = lua_para_json(&v, 0)?;
                let tamanho = serde_json::to_string(&j).map_or(usize::MAX, |t| t.len());
                if tamanho > MAX_ESTADO_SALVO {
                    return Err(erro(format!("ao_salvar: a tabela tem {tamanho} bytes em JSON (máximo {MAX_ESTADO_SALVO})")));
                }
                dados = Some(j);
                Ok(())
            });
            pedidos.truncate(antes);
            if let (true, Some(d)) = (m.ativo, dados) {
                saida.push((m.nome.clone(), d));
            }
        }
        saida
    }

    /// `ao_carregar(dados, estado)` de cada mod ativo que o define e tem dados em `por_mod` (o
    /// objeto `mods` do arquivo lateral, API 0.26).
    pub fn ao_carregar(&mut self, por_mod: &serde_json::Map<String, serde_json::Value>, e: &mut Estado) {
        let Scripts { mods, registro, mensagens, pedidos } = self;
        for m in mods.iter_mut().filter(|m| m.ativo && m.ganchos[Gancho::AoCarregar.indice()]) {
            let Some(dados) = por_mod.get(&m.nome) else { continue };
            chamar_mod(m, Gancho::AoCarregar, e, (&mut *registro, &mut *mensagens, &mut *pedidos), |lua, ud| {
                let f: Function = lua.globals().get(Gancho::AoCarregar.nome())?;
                f.call::<()>((json_para_lua(lua, dados)?, ud))
            });
        }
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

fn carregar_mod(nome: &str, arquivos: &ArquivosScript, tabelas: Rc<TabelasScripts>, limites: Limites) -> Result<ScriptMod, ErroCarga> {
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
    let mut achados = [false; N_GANCHOS];
    for k in Gancho::todos() {
        achados[k.indice()] = matches!(g.raw_get::<Value>(k.nome()), Ok(Value::Function(_)));
    }
    achados
}

/// Máquina Lua com a sandbox, os limites e a API.
fn nova_maquina(nome: &str, tabelas: Rc<TabelasScripts>, limites: Limites) -> mlua::Result<(Lua, Rc<Orcamento>)> {
    let lua = Lua::new_with(StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8, LuaOptions::default())?;
    lua.set_app_data(Contexto {
        sorteio: semente_do_nome(nome),
        registro: Vec::new(),
        mensagens: Vec::new(),
        tabelas,
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
        let todas = tabelas_do_mod(lua)?;
        let tabelas = &todas.experimental;
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
        let ctx = todas.contexto();
        Ok(experimental::por_comida(e, ctx, p.numero(), x, y, false))
    });
    reg.add_method_mut("colocar_pedra", |lua, e, (x, y): (i64, i64)| {
        let (x, y) = (coord(x), coord(y));
        let todas = tabelas_do_mod(lua)?;
        let tabelas = &todas.experimental;
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
        let todas = tabelas_do_mod(lua)?;
        let tabelas = &todas.experimental;
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
    registrar_api_026(reg);
}

/// As tabelas das regras guardadas na máquina do mod.
fn tabelas_do_mod(lua: &Lua) -> mlua::Result<Rc<TabelasScripts>> {
    lua.app_data_ref::<Contexto>().map(|c| c.tabelas.clone()).ok_or_else(|| mlua::Error::runtime("sem contexto"))
}

// ---------------------------------------------------------------------------------------------
// API 0.26: cheiros, castas e comportamento, formigas em qualquer plano, comida dos ninhos
// ---------------------------------------------------------------------------------------------

/// Tipo de cheiro → subopção da ferramenta Cheiro do Experimental (`experimental::subopcao`).
fn cheiro_de_nome(nome: &str) -> mlua::Result<u8> {
    use experimental::subopcao as s;
    match nome {
        "ninho_preto" => Ok(s::NINHO_PRETO),
        "trilha_preta" => Ok(s::TRILHA_PRETA),
        "ninho_vermelho" => Ok(s::NINHO_VERMELHO),
        "trilha_vermelha" => Ok(s::TRILHA_VERMELHA),
        "alarme" => Ok(s::ALARME),
        _ => Err(erro(format!(
            "cheiro desconhecido: {nome} (use \"ninho_preto\", \"trilha_preta\", \"ninho_vermelho\", \"trilha_vermelha\" ou \"alarme\")"
        ))),
    }
}

fn grade_de_cheiro(e: &mut Estado, sub: u8) -> &mut Grade {
    use experimental::subopcao as s;
    let f = &mut e.feromonios;
    match sub {
        s::NINHO_PRETO => &mut f.ninho_b,
        s::TRILHA_PRETA => &mut f.trilha_b,
        s::NINHO_VERMELHO => &mut f.ninho_r,
        s::TRILHA_VERMELHA => &mut f.trilha_r,
        _ => &mut f.alarme,
    }
}

/// Tile (x, y) da superfície (os mapas de cheiro cobrem só a superfície, uma célula a cada 2 × 2).
fn na_superficie(x: i32, y: i32) -> bool {
    (0..128).contains(&x) && (0..64).contains(&y)
}

/// Grupo (`(T & 0x78) >> 3`) de uma casta posta por `por_formiga` (formigas.md §3).
fn grupo_da_casta(nome: Option<&str>) -> mlua::Result<u8> {
    match nome {
        None | Some("operaria") => Ok(2),
        Some("soldado") => Ok(6),
        Some("macho") => Ok(4),
        Some("rainha_nova") => Ok(8),
        Some(outro) => Err(erro(format!(
            "casta desconhecida: {outro} (use \"operaria\", \"soldado\", \"macho\" ou \"rainha_nova\")"
        ))),
    }
}

/// Três níveis de um triângulo (Caste ou Behavior) a partir de pesos ≥ 0 com soma > 0: cada um
/// proporcional ao peso, somando 65535; o arredondamento fica no do meio, como `_GetTriLatDist`
/// (o nível 1 é o resto).
fn niveis_do_triangulo(funcao: &str, pesos: [f64; 3]) -> mlua::Result<[u16; 3]> {
    if pesos.iter().any(|p| !p.is_finite() || *p < 0.0) {
        return Err(erro(format!("{funcao}: os três valores precisam ser números ≥ 0")));
    }
    let soma: f64 = pesos.iter().sum();
    if soma <= 0.0 {
        return Err(erro(format!("{funcao}: a soma dos três valores precisa ser > 0")));
    }
    let n = |p: f64| ((p * 65535.0 / soma).floor() as u32).min(0xFFFF) as u16;
    let (n0, n2) = (n(pesos[0]), n(pesos[2]));
    Ok([n0, 0xFFFFu16.saturating_sub(n0).saturating_sub(n2), n2])
}

/// Métodos do `estado` da 0.26 (escritas imediatas e validadas, como as da 0.12).
fn registrar_api_026(reg: &mut mlua::UserDataRegistry<Estado>) {
    // --- cheiros (só na superfície) ---
    reg.add_method("cheiro", |_, e, (tipo, x, y): (String, i64, i64)| {
        use experimental::subopcao as s;
        let sub = cheiro_de_nome(&tipo)?;
        let (x, y) = (coord(x), coord(y));
        if !na_superficie(x, y) {
            return Ok(None);
        }
        let f = &e.feromonios;
        let g = match sub {
            s::NINHO_PRETO => &f.ninho_b,
            s::TRILHA_PRETA => &f.trilha_b,
            s::NINHO_VERMELHO => &f.ninho_r,
            s::TRILHA_VERMELHA => &f.trilha_r,
            _ => &f.alarme,
        };
        Ok(g.em(x >> 1, y >> 1))
    });
    for (nome, tirar) in [("depositar_cheiro", false), ("tirar_cheiro", true)] {
        reg.add_method_mut(nome, move |lua, e, (tipo, x, y): (String, i64, i64)| {
            let sub = cheiro_de_nome(&tipo)?;
            let (x, y) = (coord(x), coord(y));
            if !na_superficie(x, y) {
                return Ok(false);
            }
            let t = tabelas_do_mod(lua)?;
            Ok(experimental::mudar_cheiro(e, &t.experimental.regras, sub, x, y, tirar))
        });
    }
    reg.add_method_mut("limpar_cheiro", |_, e, (tipo, x, y): (String, i64, i64)| {
        let sub = cheiro_de_nome(&tipo)?;
        let (x, y) = (coord(x), coord(y));
        if !na_superficie(x, y) {
            return Ok(false);
        }
        Ok(grade_de_cheiro(e, sub).definir(x >> 1, y >> 1, 0))
    });

    // --- castas e comportamento (janelas Caste e Behavior Control) ---
    reg.add_method("castas", |lua, e, ()| {
        let k = &e.controles;
        let t = lua.create_table()?;
        t.set("reprodutores", k.castas[0])?;
        t.set("operarias", k.castas[1])?;
        t.set("soldados", k.castas[2])?;
        t.set("automatico", k.casta_automatica != 0)?;
        let ideais = lua.create_table()?;
        for (i, nome) in ["operaria", "soldado", "macho", "rainha_nova"].into_iter().enumerate() {
            ideais.set(nome, k.castas_ideais[i])?;
        }
        t.set("ideais", ideais)?;
        Ok(t)
    });
    reg.add_method("comportamento", |lua, e, ()| {
        let k = &e.controles;
        let t = lua.create_table()?;
        t.set("forragear", k.tarefas[0])?;
        t.set("cavar", k.tarefas[1])?;
        t.set("cuidar", k.tarefas[2])?;
        t.set("automatico", k.modo_automatico != 0)?;
        Ok(t)
    });
    reg.add_method_mut("definir_castas", |_, e, (r, o, s): (f64, f64, f64)| {
        // Como arrastar o botão do triângulo (`_ProcCasteEvent`): níveis, `_IdealCaste` e Manual.
        let n = niveis_do_triangulo("definir_castas", [r, o, s])?;
        e.controles.castas = n;
        e.controles.castas_ideais = crate::core::triangulo::castas_ideais(n);
        e.controles.casta_automatica = 0;
        Ok(())
    });
    reg.add_method_mut("definir_comportamento", |_, e, (f, c, n): (f64, f64, f64)| {
        // `_ProcModeEvent`: níveis e Manual.
        e.controles.tarefas = niveis_do_triangulo("definir_comportamento", [f, c, n])?;
        e.controles.modo_automatico = 0;
        Ok(())
    });
    reg.add_method_mut("castas_automaticas", |_, e, ligar: bool| {
        e.controles.casta_automatica = i16::from(ligar);
        Ok(())
    });
    reg.add_method_mut("comportamento_automatico", |_, e, ligar: bool| {
        e.controles.modo_automatico = i16::from(ligar);
        Ok(())
    });

    // --- formigas em qualquer plano ---
    reg.add_method_mut(
        "por_formiga",
        |lua, e, (plano, cor, x, y, casta, modo): (String, String, i64, i64, Option<String>, Option<i64>)| {
            let p = Plano::de_nome(&plano)?;
            let cor = cor_de_nome(&cor)?;
            let grupo = grupo_da_casta(casta.as_deref())?;
            let modo = modo.map(|m| faixa("por_formiga", "modo", m, 0, 19).map(|m| m as u8)).transpose()?;
            let (x, y) = (coord(x), coord(y));
            let todas = tabelas_do_mod(lua)?;
            let r = &todas.experimental.regras;
            let base = grupo << 3;
            let tipo = if cor == Cor::Vermelha { base.wrapping_add(r.formiga_vermelha) } else { base };
            let f = |modo| Formiga { x: x as u8, y: y as u8, modo, tipo, s: 0 };
            match p {
                Plano::Superficie => {
                    if e.superficie.vida.em(x, y) != Some(0) || !comum::celula_livre(e, 1, x, y) {
                        return Ok(false);
                    }
                    Ok(e.adicionar_formiga(Lista::A, f(modo.unwrap_or(r.formiga_modo_superficie))))
                }
                Plano::NinhoPreto | Plano::NinhoVermelho => {
                    // `_ExpAddAnt` no ninho, sem o sorteio do T: nunca na linha 0; cava a célula se
                    // o tile for > `cavar_formiga_acima_de` (`_DigTile*`, que sorteia S).
                    if y == 0 || p.vida(e).em(x, y) != Some(0) {
                        return Ok(false);
                    }
                    let ninho = if p == Plano::NinhoPreto { Cor::Preta } else { Cor::Vermelha };
                    if e.ninho(ninho).mapa.em(x, y).is_some_and(|v| v > r.cavar_formiga_acima_de) {
                        comum::cavar(e, &todas.amarela, ninho, x, y);
                    }
                    let padrao = if ninho == cor { r.formiga_modo_proprio_ninho } else { r.formiga_modo_ninho_alheio };
                    Ok(e.adicionar_formiga(comum::lista_do_ninho(ninho), f(modo.unwrap_or(padrao))))
                }
            }
        },
    );
    reg.add_method_mut("remover_formiga", |_, e, (plano, x, y): (String, i64, i64)| {
        let p = Plano::de_nome(&plano)?;
        let (x, y) = (coord(x), coord(y));
        if !experimental::posicao_valida(p.numero(), x, y) {
            return Ok(false);
        }
        Ok(experimental::tirar_formiga(e, p.numero(), x, y))
    });

    // --- comida (superfície e ninhos) ---
    reg.add_method_mut("tirar_comida", |lua, e, (plano, x, y): (String, i64, i64)| {
        let p = Plano::de_nome(&plano)?;
        let (x, y) = (coord(x), coord(y));
        if !experimental::posicao_valida(p.numero(), x, y) {
            return Ok(false);
        }
        let todas = tabelas_do_mod(lua)?;
        Ok(experimental::por_comida(e, todas.contexto(), p.numero(), x, y, true))
    });
}

// ---------------------------------------------------------------------------------------------
// API 0.26: ganchos dentro do tick
// ---------------------------------------------------------------------------------------------

/// As regras do tick com os ganchos `antes_<parte>`/`depois_<parte>` dos scripts em volta de cada
/// parte. Cada método chama o gancho de antes, a parte das regras de dentro (sem mudança) e o de
/// depois; a ordem das partes é a de `rules::tick::tick`, a do original. Só é usada quando algum
/// script define um gancho de parte ([`Scripts::tem_ganchos_do_tick`]); sem isso a `Partida` chama
/// o tick com as regras diretamente.
pub struct RegrasComScripts<'a, R: Regras + ?Sized> {
    pub regras: &'a mut R,
    pub scripts: &'a mut Scripts,
}

impl<R: Regras + ?Sized> RegrasComScripts<'_, R> {
    fn em_volta(&mut self, p: ParteTick, e: &mut Estado, parte: impl FnOnce(&mut R, &mut Estado)) {
        self.scripts.parte(p, false, e);
        parte(self.regras, e);
        self.scripts.parte(p, true, e);
    }
}

impl<R: Regras + ?Sized> RegrasQuintal for RegrasComScripts<'_, R> {
    fn simular_quintal(&mut self, e: &mut Estado) {
        self.em_volta(ParteTick::Quintal, e, |r, e| r.simular_quintal(e));
    }
    fn simular_agua(&mut self, e: &mut Estado) {
        self.em_volta(ParteTick::Agua, e, |r, e| r.simular_agua(e));
    }
}

impl<R: Regras + ?Sized> RegrasInimigos for RegrasComScripts<'_, R> {
    fn simular_formigas_leao(&mut self, e: &mut Estado) {
        self.em_volta(ParteTick::FormigasLeao, e, |r, e| r.simular_formigas_leao(e));
    }
    fn mover_aranha(&mut self, e: &mut Estado) {
        self.em_volta(ParteTick::Aranha, e, |r, e| r.mover_aranha(e));
    }
    fn simular_lagarta(&mut self, e: &mut Estado) {
        self.em_volta(ParteTick::Lagarta, e, |r, e| r.simular_lagarta(e));
    }
}

impl<R: Regras + ?Sized> RegrasSuperficie for RegrasComScripts<'_, R> {
    fn simular_superficie(&mut self, e: &mut Estado) {
        self.em_volta(ParteTick::Superficie, e, |r, e| r.simular_superficie(e));
    }
    fn adicionar_comida(&mut self, e: &mut Estado, quantidade: i32, som: bool) {
        self.regras.adicionar_comida(e, quantidade, som);
    }
    fn escolher_estrategia(&mut self, e: &mut Estado) {
        self.em_volta(ParteTick::Estrategia, e, |r, e| r.escolher_estrategia(e));
    }
    fn criar_lider_vermelha(&mut self, e: &mut Estado) {
        self.em_volta(ParteTick::LiderVermelha, e, |r, e| r.criar_lider_vermelha(e));
    }
    fn fase_de_cheiro(&mut self, e: &mut Estado, fase: crate::rules::feromonios::FaseCheiro) {
        self.regras.fase_de_cheiro(e, fase);
    }
}

impl<R: Regras + ?Sized> RegrasNinho for RegrasComScripts<'_, R> {
    fn simular_ninho(&mut self, e: &mut Estado, cor: Cor) {
        let p = if cor == Cor::Preta { ParteTick::NinhoPreto } else { ParteTick::NinhoVermelho };
        self.em_volta(p, e, |r, e| r.simular_ninho(e, cor));
    }
}

impl<R: Regras + ?Sized> RegrasAmarela for RegrasComScripts<'_, R> {
    fn simular_amarela(&mut self, e: &mut Estado) {
        self.em_volta(ParteTick::Amarela, e, |r, e| r.simular_amarela(e));
    }
    fn mover_amarela(&mut self, e: &mut Estado) {
        self.em_volta(ParteTick::MovimentoAmarela, e, |r, e| r.mover_amarela(e));
    }
}

impl<R: Regras + ?Sized> RegrasConselhos for RegrasComScripts<'_, R> {
    fn dar_conselhos(&mut self, e: &mut Estado) {
        self.em_volta(ParteTick::Conselhos, e, |r, e| r.dar_conselhos(e));
    }
}

// ---------------------------------------------------------------------------------------------
// API 0.26: estado dos scripts no arquivo lateral do save
// ---------------------------------------------------------------------------------------------

/// Sufixo do arquivo lateral: `NOME.ANT` → `NOME.ANT.mods.json`.
pub const SUFIXO_ARQUIVO_LATERAL: &str = ".mods.json";
/// Valor de `formato` no arquivo lateral.
pub const FORMATO_ARQUIVO_LATERAL: &str = "simant-recomp/estado-dos-mods";
/// Tamanho máximo do JSON da tabela de um mod (bytes).
pub const MAX_ESTADO_SALVO: usize = 1024 * 1024;
/// Profundidade máxima de tabelas aninhadas no estado salvo.
const MAX_PROFUNDIDADE: usize = 32;

/// O arquivo lateral de um save (`NOME.ANT` → `NOME.ANT.mods.json`, na mesma pasta).
pub fn arquivo_lateral(save: &Path) -> std::path::PathBuf {
    let mut s = save.as_os_str().to_owned();
    s.push(SUFIXO_ARQUIVO_LATERAL);
    std::path::PathBuf::from(s)
}

/// Valor Lua → JSON: `nil`, booleanos, números (finitos), textos UTF-8 e tabelas (sequência 1..n
/// vira lista; só chaves de texto vira objeto; tabela vazia vira objeto). Funções, userdata, chaves
/// misturadas e listas com buracos são erro.
pub fn lua_para_json(v: &Value, profundidade: usize) -> mlua::Result<serde_json::Value> {
    use serde_json::Value as J;
    if profundidade > MAX_PROFUNDIDADE {
        return Err(erro(format!("ao_salvar: tabelas aninhadas demais (mais de {MAX_PROFUNDIDADE})")));
    }
    Ok(match v {
        Value::Nil => J::Null,
        Value::Boolean(b) => J::Bool(*b),
        Value::Integer(i) => J::from(*i),
        Value::Number(f) => serde_json::Number::from_f64(*f)
            .map(J::Number)
            .ok_or_else(|| erro(format!("ao_salvar: número não finito ({f})")))?,
        Value::String(s) => J::String(s.to_str().map_err(|_| erro("ao_salvar: texto que não é UTF-8".into()))?.to_string()),
        Value::Table(t) => {
            let n = t.raw_len();
            let mut pares = Vec::new();
            for par in t.clone().pairs::<Value, Value>() {
                pares.push(par?);
            }
            if n > 0 && pares.len() == n {
                let mut lista = Vec::with_capacity(n);
                for i in 1..=n {
                    lista.push(lua_para_json(&t.raw_get::<Value>(i)?, profundidade + 1)?);
                }
                J::Array(lista)
            } else {
                let mut obj = serde_json::Map::new();
                for (k, v) in pares {
                    let Value::String(k) = k else {
                        return Err(erro(
                            "ao_salvar: uma tabela precisa ser uma lista 1..n sem buracos ou ter só chaves de texto".into(),
                        ));
                    };
                    let k = k.to_str().map_err(|_| erro("ao_salvar: chave que não é UTF-8".into()))?.to_string();
                    obj.insert(k, lua_para_json(&v, profundidade + 1)?);
                }
                J::Object(obj)
            }
        }
        outro => return Err(erro(format!("ao_salvar: valor que não vai para o save ({})", outro.type_name()))),
    })
}

/// JSON → valor Lua (o inverso de [`lua_para_json`]; números inteiros voltam inteiros).
pub fn json_para_lua(lua: &Lua, v: &serde_json::Value) -> mlua::Result<Value> {
    use serde_json::Value as J;
    Ok(match v {
        J::Null => Value::Nil,
        J::Bool(b) => Value::Boolean(*b),
        J::Number(n) => match n.as_i64() {
            Some(i) => Value::Integer(i),
            None => Value::Number(n.as_f64().unwrap_or(0.0)),
        },
        J::String(s) => Value::String(lua.create_string(s)?),
        J::Array(l) => {
            let t = lua.create_table()?;
            for (i, x) in l.iter().enumerate() {
                t.raw_set(i + 1, json_para_lua(lua, x)?)?;
            }
            Value::Table(t)
        }
        J::Object(o) => {
            let t = lua.create_table()?;
            for (k, x) in o {
                t.raw_set(k.as_str(), json_para_lua(lua, x)?)?;
            }
            Value::Table(t)
        }
    })
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
