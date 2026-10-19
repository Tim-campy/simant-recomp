//! Ferramentas do modo Experimental (`_DoTool` `1010:727e`, `_processExp` `1010:7022`) e as trocas de
//! modo do menu de ferramentas (Quick Game → Experimental, "Start Game" do Experimental).
//! Especificação: docs/spec/telas/experimental.md. Dados: data/experimental.json (com mods).
//!
//! Tudo aqui é regra sobre o [`Estado`], sem janela: a interface traduz cliques e o menu em
//! chamadas a [`pressionar`], [`arrastar`] e [`escolher_ferramenta`], e recebe de volta os
//! [`Efeito`]s (sons, música, aviso no rodapé) que o original produz junto.
//!
//! Estado de interface que o original guarda fora do save e que fica em [`ControleExperimental`]:
//! `_ExpSubStates` (`8:8600`, a subopção de cada ferramenta) e `_ExpLastPnt` (`9:7d46`, o ponto
//! anterior do arrasto). A ferramenta atual (`_CurExpTool`, `9:79e2`) está no save:
//! `Estado::partida.ferramenta_experimental`.

use serde::Deserialize;

use super::amarela::{self, TabelasAmarela};
use super::comum::{self, achar_formiga, achar_na_lista_a, cadaver_aqui, cor_do_plano, get_dir, tile, vida_bruta};
use super::estado::{Estado, Grade};
use super::geracao::Cor;
use super::inimigos::{self, TabelasInimigos};
use crate::formats::save::{Formiga, Lista};
use crate::recursos::Recursos;

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_TABELAS: &str = "experimental.json";

const JSON_PADRAO: &str = include_str!("../../../data/experimental.json");

/// Tipo de jogo Quick Game e Experimental (`_CurGameType`).
const TIPO_RAPIDO: i16 = 1;
const TIPO_COMPLETO: i16 = 2;
const TIPO_EXPERIMENTAL: i16 = 3;

/// Ferramentas do menu do Experimental (janela 9 `editToolsMenu`, objetos `0x902 + n`).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Ferramenta {
    /// `editExpInfo`: examinar a formiga da célula (`_MagnifyMenu`).
    Info = 0,
    /// `editExpBarrier`: muros ou pedrinhas (`_DropWall`).
    Barreira = 1,
    /// `editExpDig`: cavar ou encher (`_ExpDig`).
    Cavar = 2,
    /// `editExpAddAnt`: pôr formiga preta ou vermelha (`_ExpAddAnt`).
    Formiga = 3,
    /// `editExpAddFood`: comida, um pedaço ou vinte espalhados (`_IncFoodHere`).
    Comida = 4,
    /// `editExpTrails`: cheiro (`_ExpIncSmell`).
    Cheiro = 5,
    /// `editExpInsecticide`: inseticida em spray ou jato (`_ExpKillAnts`).
    Inseticida = 6,
    /// `editExpStartGame`: sai do Experimental e começa um Quick Game no mesmo mundo.
    ComecarJogo = 7,
}

impl Ferramenta {
    pub const TODAS: [Ferramenta; 8] = [
        Ferramenta::Info,
        Ferramenta::Barreira,
        Ferramenta::Cavar,
        Ferramenta::Formiga,
        Ferramenta::Comida,
        Ferramenta::Cheiro,
        Ferramenta::Inseticida,
        Ferramenta::ComecarJogo,
    ];

    /// Ferramenta pelo valor de `_CurExpTool`.
    pub fn de_i16(v: i16) -> Option<Ferramenta> {
        Ferramenta::TODAS.get(usize::try_from(v).ok()?).copied()
    }

    pub fn indice(self) -> usize {
        self as usize
    }
}

/// Subopções (`_ExpSubStates[ferramenta]`): o índice do item escolhido no submenu da ferramenta.
pub mod subopcao {
    /// Barreira (janela 12 `wallRockMenu`): 0 `toolWalls`, 1 `toolRocks`.
    pub const MUROS: u8 = 0;
    pub const PEDRAS: u8 = 1;
    /// Cavar (janela 15 `digFillMenu`): 0 `toolDig`, 1 `toolFill`.
    pub const CAVAR: u8 = 0;
    pub const ENCHER: u8 = 1;
    /// Formiga (janela 16 `blackRedMenu`): 0 `toolBlack`, 1 `toolRed`.
    pub const PRETA: u8 = 0;
    pub const VERMELHA: u8 = 1;
    /// Comida (janela 17 `foodMenu`): 0 `onePiece`, 1 `twentyPieces`.
    pub const UM_PEDACO: u8 = 0;
    pub const VINTE_PEDACOS: u8 = 1;
    /// Cheiro (janela 13 `placeScentMenu`): ninho preto, trilha preta, ninho vermelho, trilha
    /// vermelha, alarme.
    pub const NINHO_PRETO: u8 = 0;
    pub const TRILHA_PRETA: u8 = 1;
    pub const NINHO_VERMELHO: u8 = 2;
    pub const TRILHA_VERMELHA: u8 = 3;
    pub const ALARME: u8 = 4;
    /// Inseticida (janela 14 `cideMenu`): 0 `toolSpray` (8 jatos espalhados), 1 `toolStream`
    /// (8 jatos na célula).
    pub const SPRAY: u8 = 0;
    pub const JATO: u8 = 1;
}

/// Constantes das ferramentas (data/experimental.json, "regras").
#[derive(Clone, Debug, Deserialize)]
pub struct RegrasExperimental {
    pub muro: u8,
    pub muro_ultimo: u8,
    pub barreira_abaixo_de: u8,
    pub pedrinha: u8,
    pub pedrinha_sorteio: u16,
    pub apagar_acima_de: u8,
    pub apagar_abaixo_de: u8,
    pub apagar_chao_sorteio: u16,
    pub som_pedra_sorteio: u16,
    pub som_pedra_base: i32,
    pub terra_macica: u8,
    pub cavar_formiga_acima_de: u8,
    pub comida_superficie: i32,
    pub comida_ninho: i32,
    pub comida_somar_ate: i32,
    pub comida_tirar_ate: i32,
    pub comida_chao_abaixo_de: i32,
    pub comida_tirar_sorteio: u16,
    pub comida_espalhar_vezes: u32,
    pub espalhar_lado: u16,
    pub cheiro_somar: i32,
    pub cheiro_tirar: i32,
    pub inseticida_vezes: u32,
    pub inseticida_som_sorteio: u16,
    pub inseticida_som_base: i32,
    pub leao_de: u8,
    pub leao_ate: u8,
    pub formiga_tipo: u8,
    pub formiga_tipo_sorteio: u16,
    pub formiga_vermelha: u8,
    pub formiga_modo_superficie: u8,
    pub formiga_modo_proprio_ninho: u8,
    pub formiga_modo_ninho_alheio: u8,
    pub amarela_x: i16,
    pub amarela_y: i16,
    pub amarela_tipo: i16,
    pub amarela_tipo_alada: i16,
    pub amarela_modo_experimental: i16,
    pub amarela_ovo_carregado: i16,
    pub amarela_saude: i32,
    pub aranha_x: i16,
    pub aranha_y: i16,
    pub aranha_arrotos: i16,
    pub aranha_destino: i16,
}

/// Ids dos sons (`_myBeginSound`) e da música (`_myBeginSong`).
#[derive(Clone, Debug, Deserialize)]
pub struct SonsExperimental {
    pub erro: u16,
    pub para_experimental: u16,
    pub inseticida: u16,
    pub pedra: u16,
    pub encher: u16,
    pub cavar: u16,
    pub cheiro: u16,
    pub formiga: u16,
    pub comida: u16,
    pub comida_espalhada: u16,
    pub muro: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MusicasExperimental {
    pub comecar_jogo: u16,
}

/// Avisos do rodapé (índices na lista de textos `lista`).
#[derive(Clone, Debug, Deserialize)]
pub struct MensagensExperimental {
    pub lista: u16,
    pub encher_na_superficie: u16,
    pub sem_lugar_para_buraco: u16,
}

/// Listas de textos do quadro de exame.
#[derive(Clone, Debug, Deserialize)]
pub struct ListasExame {
    pub estagios: u16,
    pub grupos: u16,
    pub modos: u16,
    pub cargas: u16,
}

/// Conteúdo de data/experimental.json.
#[derive(Clone, Debug, Deserialize)]
pub struct TabelasExperimental {
    /// Tile do muro pela máscara das vizinhas (`DGROUP:0x1bce`): esquerda × 8 + baixo × 4 +
    /// direita × 2 + cima.
    pub conexao_muro: [u8; 16],
    /// Valores iniciais de `_ExpSubStates[0..8]`.
    pub subopcoes_iniciais: [u8; 8],
    pub regras: RegrasExperimental,
    pub sons: SonsExperimental,
    pub musicas: MusicasExperimental,
    pub mensagens: MensagensExperimental,
    pub exame: ListasExame,
}

impl TabelasExperimental {
    /// Lê `experimental.json` (com mods).
    pub fn carregar(r: &Recursos) -> Result<TabelasExperimental, String> {
        let t: TabelasExperimental = r.dados(ARQUIVO_TABELAS)?;
        t.validar()?;
        Ok(t)
    }

    pub fn de_json(texto: &str) -> Result<TabelasExperimental, String> {
        let t: TabelasExperimental =
            serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_TABELAS}: {e}"))?;
        t.validar()?;
        Ok(t)
    }

    /// A cópia de data/experimental.json embutida na compilação.
    pub fn padrao() -> &'static TabelasExperimental {
        static PADRAO: std::sync::OnceLock<TabelasExperimental> = std::sync::OnceLock::new();
        PADRAO.get_or_init(|| TabelasExperimental::de_json(JSON_PADRAO).expect("data/experimental.json embutido"))
    }

    fn validar(&self) -> Result<(), String> {
        let r = &self.regras;
        let sorteios = [
            r.pedrinha_sorteio,
            r.apagar_chao_sorteio,
            r.som_pedra_sorteio,
            r.comida_tirar_sorteio,
            r.espalhar_lado,
            r.inseticida_som_sorteio,
            r.formiga_tipo_sorteio,
        ];
        if sorteios.contains(&0) {
            return Err(format!("{ARQUIVO_TABELAS}: sorteio com n = 0 (SRand1(0) divide por zero)"));
        }
        if r.muro > r.muro_ultimo {
            return Err(format!("{ARQUIVO_TABELAS}: muro > muro_ultimo"));
        }
        Ok(())
    }

    /// `_IsWall`: tile de muro (0x60..0x67).
    fn e_muro(&self, v: i32) -> bool {
        (self.regras.muro as i32..=self.regras.muro_ultimo as i32).contains(&v)
    }
}

/// Tabelas de que as ferramentas precisam: as próprias e as das rotinas de outras partes que elas
/// chamam (amarela: `_SetMyLife`, `_DigMyNewHole`, `_Dx9`; inimigos: `_KillSpider`, `_KillAntLion`).
#[derive(Clone, Copy)]
pub struct Contexto<'a> {
    pub tabelas: &'a TabelasExperimental,
    pub amarela: &'a TabelasAmarela,
    pub inimigos: &'a TabelasInimigos,
}

impl Contexto<'static> {
    /// Com as cópias embutidas de data/*.json.
    pub fn padrao() -> Contexto<'static> {
        Contexto {
            tabelas: TabelasExperimental::padrao(),
            amarela: TabelasAmarela::padrao(),
            inimigos: TabelasInimigos::padrao(),
        }
    }
}

/// Estado de interface das ferramentas que não está no save.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControleExperimental {
    /// `_ExpSubStates[ferramenta]`.
    pub subopcoes: [u8; 8],
    /// `_ExpLastPnt`: célula anterior do arrasto.
    pub ultimo_ponto: (i32, i32),
    /// Célula atual do botão apertado em `_processExp` (`None` = botão solto).
    pub atual: Option<(i32, i32)>,
}

impl ControleExperimental {
    pub fn novo(t: &TabelasExperimental) -> ControleExperimental {
        ControleExperimental { subopcoes: t.subopcoes_iniciais, ultimo_ponto: (0, 0), atual: None }
    }

    /// Subopção da ferramenta.
    pub fn subopcao(&self, f: Ferramenta) -> u8 {
        self.subopcoes[f.indice()]
    }
}

/// O que o original faz fora do estado junto com a ferramenta: a interface toca ou mostra.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Efeito {
    /// `_myBeginSound(id, parâmetro)` (o parâmetro é 0 ou um valor sorteado: o sorteio já conta para
    /// o gerador S).
    Som { id: u16, parametro: i32 },
    /// `_myBeginSong(id)`.
    Musica(u16),
    /// `_EditMessage` com o texto `indice` da lista `lista` (SHARED tipo 4).
    Aviso { lista: u16, indice: u16 },
    /// A ferramenta Info achou uma formiga: abrir o quadro de exame (`_MagnifyMenu`).
    Examinar(Exame),
}

/// O que o quadro de exame mostra (`_win_DrawExamineWindow` `1010:69c0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exame {
    pub plano: i32,
    pub x: i32,
    pub y: i32,
    /// Byte da vida (T, ou 0xFF/0xFE da amarela).
    pub vida: u8,
    /// `(vida & 0x78) >> 3`: grupo (lista `grupos`); 0 = ovo, e o estágio é `vida & 7` (lista
    /// `estagios`). A terceira linha do quadro usa a lista `cargas` com o mesmo grupo.
    pub grupo: u8,
    pub estagio: u8,
    /// M da formiga na lista (lista `modos`); 0 quando não foi achada.
    pub modo: u8,
}

// ---------------------------------------------------------------------------------------------
// Validação e acesso
// ---------------------------------------------------------------------------------------------

/// `_IsValidLocation(plano, x, y)` (`1020:56da`): x em 0..127 nos planos 0 e 1, 0..63 nos ninhos; y em
/// 0..63.
pub fn posicao_valida(plano: i32, x: i32, y: i32) -> bool {
    let largura = if plano < 2 { 128 } else { 64 };
    (0..largura).contains(&x) && (0..64).contains(&y)
}

/// `_IsValidA`.
fn valida_a(x: i32, y: i32) -> bool {
    (0..128).contains(&x) && (0..64).contains(&y)
}

fn mapa_do_plano(e: &mut Estado, plano: i32) -> Option<&mut Grade> {
    match plano {
        0 | 1 => Some(&mut e.superficie.mapa),
        2 => Some(&mut e.ninho_b.mapa),
        3 => Some(&mut e.ninho_r.mapa),
        _ => None,
    }
}

/// `_SetMap(plano, x, y, v)`.
fn gravar_mapa(e: &mut Estado, plano: i32, x: i32, y: i32, v: u8) {
    if posicao_valida(plano, x, y) {
        if let Some(g) = mapa_do_plano(e, plano) {
            g.definir(x, y, v);
        }
    }
}

/// `_Dx9/_Dy9[k]`.
fn d9(ta: &TabelasAmarela, k: i32) -> (i32, i32) {
    let k = k.clamp(0, 8) as usize;
    (ta.dx9[k] as i32, ta.dy9[k] as i32)
}

fn som(id: u16) -> Efeito {
    Efeito::Som { id, parametro: 0 }
}

// ---------------------------------------------------------------------------------------------
// Clique e arrasto (`_processExp`)
// ---------------------------------------------------------------------------------------------

/// Botão apertado na célula (x, y) do plano mostrado (`_processExp`, início): `_ExpLastPnt` = (x, y) e
/// a ferramenta atual em (x, y), se a célula é válida.
pub fn pressionar(e: &mut Estado, c: Contexto, ctl: &mut ControleExperimental, plano: i32, x: i32, y: i32, ctrl: bool) -> Vec<Efeito> {
    ctl.ultimo_ponto = (x, y);
    ctl.atual = Some((x, y));
    if posicao_valida(plano, x, y) {
        aplicar_ferramenta(e, c, ctl, plano, x, y, ctrl)
    } else {
        Vec::new()
    }
}

/// Uma volta do laço de `_processExp` com o botão ainda apertado e o cursor na célula (x, y): se a
/// célula mudou, `_ExpLastPnt` = a anterior e a ferramenta age na nova (muro e cavar percorrem o
/// caminho entre as duas); com as ferramentas 5 e 6 (cheiro e inseticida), age de novo em (x, y) em
/// toda volta, mudando ou não de célula. O número de voltas por segundo depende do redesenho
/// (tempo real), não do tick: quem chama decide o ritmo.
pub fn arrastar(e: &mut Estado, c: Contexto, ctl: &mut ControleExperimental, plano: i32, x: i32, y: i32, ctrl: bool) -> Vec<Efeito> {
    let mut efeitos = Vec::new();
    let atual = ctl.atual.unwrap_or((x, y));
    if atual != (x, y) {
        ctl.ultimo_ponto = atual;
        ctl.atual = Some((x, y));
        if posicao_valida(plano, x, y) {
            efeitos.extend(aplicar_ferramenta(e, c, ctl, plano, x, y, ctrl));
        }
    }
    if e.partida.ferramenta_experimental > 4 && posicao_valida(plano, x, y) {
        efeitos.extend(aplicar_ferramenta(e, c, ctl, plano, x, y, ctrl));
    }
    efeitos
}

/// Botão solto: fim do laço de `_processExp`.
pub fn soltar(ctl: &mut ControleExperimental) {
    ctl.atual = None;
}

/// `_DoTool(x, y)` (`1010:727e`) com a ferramenta `_CurExpTool` e as subopções de `ctl`. `ctrl` = tecla
/// Ctrl apertada (`GetAsyncKeyState(VK_CONTROL)`), que inverte muro, cavar, comida e cheiro.
pub fn aplicar_ferramenta(e: &mut Estado, c: Contexto, ctl: &ControleExperimental, plano: i32, x: i32, y: i32, ctrl: bool) -> Vec<Efeito> {
    let Some(f) = Ferramenta::de_i16(e.partida.ferramenta_experimental) else { return Vec::new() };
    let sub = ctl.subopcao(f);
    let (ux, uy) = ctl.ultimo_ponto;
    match f {
        Ferramenta::Info => examinar(e, plano, x, y).map(Efeito::Examinar).into_iter().collect(),
        Ferramenta::Barreira if plano < 2 => por_barreira(e, c, sub, (ux, uy), (x, y), ctrl),
        Ferramenta::Barreira => Vec::new(),
        Ferramenta::Cavar => cavar_ou_encher(e, c, plano, sub, (ux, uy), (x, y), ctrl),
        Ferramenta::Formiga => por_formiga(e, c, plano, sub, x, y),
        Ferramenta::Comida => {
            let r = &c.tabelas.regras;
            if sub != subopcao::UM_PEDACO {
                let meio = (r.espalhar_lado / 2) as i32;
                for _ in 0..r.comida_espalhar_vezes {
                    let px = e.s.rand1(r.espalhar_lado) + x - meio;
                    let py = e.s.rand1(r.espalhar_lado) + y - meio;
                    if posicao_valida(plano, px, py) {
                        por_comida(e, c, plano, px, py, ctrl);
                    }
                }
                vec![som(c.tabelas.sons.comida_espalhada)]
            } else if por_comida(e, c, plano, x, y, ctrl) {
                vec![som(c.tabelas.sons.comida)]
            } else {
                Vec::new()
            }
        }
        Ferramenta::Cheiro => por_cheiro(e, c, plano, sub, x, y, ctrl),
        Ferramenta::Inseticida => inseticida(e, c, plano, sub, x, y),
        Ferramenta::ComecarJogo => Vec::new(),
    }
}

// ---------------------------------------------------------------------------------------------
// Ferramenta 0: Info
// ---------------------------------------------------------------------------------------------

/// `_MagnifyMenu` (`1010:6b4c`) e o conteúdo de `_win_DrawExamineWindow`: `None` se a célula não tem
/// formiga.
pub fn examinar(e: &Estado, plano: i32, x: i32, y: i32) -> Option<Exame> {
    let vida = vida_bruta(e, plano, x, y).filter(|&v| v != 0)?;
    let grupo = (vida & 0x78) >> 3;
    let estagio = vida & 7;
    let indice = match plano {
        0 | 1 => achar_na_lista_a(e, x, y).map(|i| (Lista::A, i)),
        2 => achar_formiga(e, Lista::B, x, y, vida).map(|i| (Lista::B, i)),
        3 => achar_formiga(e, Lista::R, x, y, vida).map(|i| (Lista::R, i)),
        _ => None,
    };
    let modo = indice.map_or(0, |(l, i)| e.lista(l).vagas[i].modo);
    Some(Exame { plano, x, y, vida, grupo, estagio, modo })
}

// ---------------------------------------------------------------------------------------------
// Ferramenta 1: Barreira (`_DropWall`, `_ConnectAll`)
// ---------------------------------------------------------------------------------------------

/// `_DropWall(de, para)` (`1010:7450`): percorre de `de` até `para` (primeiro em x, depois em y) e,
/// em cada célula: com Ctrl apaga pedra/muro; senão põe muro (subopção 0) ou pedrinha (1).
fn por_barreira(e: &mut Estado, c: Contexto, sub: u8, de: (i32, i32), para: (i32, i32), ctrl: bool) -> Vec<Efeito> {
    let t = c.tabelas;
    let r = &t.regras;
    let mut efeitos = Vec::new();
    let (mut x, mut y) = de;
    loop {
        if !valida_a(x, y) {
            return efeitos;
        }
        let v = e.superficie.mapa.em(x, y).unwrap_or(0);
        if ctrl {
            if v > r.apagar_acima_de && v < r.apagar_abaixo_de {
                let novo = e.s.rand1(r.apagar_chao_sorteio) as u8;
                e.superficie.mapa.definir(x, y, novo);
            }
            ligar_muros(e, t, x, y);
        } else {
            let livre = v < r.barreira_abaixo_de && e.superficie.vida.em(x, y) == Some(0);
            if sub == subopcao::MUROS {
                if livre {
                    e.superficie.mapa.definir(x, y, r.muro);
                }
                ligar_muros(e, t, x, y);
                efeitos.push(som(t.sons.muro));
            } else {
                if livre {
                    let novo = r.pedrinha + e.s.rand1(r.pedrinha_sorteio) as u8;
                    e.superficie.mapa.definir(x, y, novo);
                }
                let p = e.s.rand1(r.som_pedra_sorteio) + r.som_pedra_base;
                efeitos.push(Efeito::Som { id: t.sons.pedra, parametro: p });
            }
        }
        let k = get_dir(x, y, para.0, para.1);
        let (dx, dy) = d9(c.amarela, k);
        if dx != 0 {
            x += dx;
        } else {
            y += dy;
        }
        if k == 0 {
            return efeitos;
        }
    }
}

/// Máscara de muros em volta de (x, y), lida com `_GetMap(1, …)` (fora do mapa não é muro).
fn mascara_muro(e: &Estado, t: &TabelasExperimental, x: i32, y: i32) -> usize {
    let muro = |x: i32, y: i32| t.e_muro(tile(e, 1, x, y)) as usize;
    muro(x - 1, y) * 8 + muro(x, y + 1) * 4 + muro(x + 1, y) * 2 + muro(x, y - 1)
}

/// `_ConnectAll(x, y)` (`1010:805a`): a célula e as quatro vizinhas (cima, direita, baixo, esquerda),
/// se forem muro, recebem o tile pela máscara das próprias vizinhas.
fn ligar_muros(e: &mut Estado, t: &TabelasExperimental, x: i32, y: i32) {
    for (cx, cy, checar) in [(x, y, false), (x, y - 1, true), (x + 1, y, true), (x, y + 1, true), (x - 1, y, true)] {
        if checar && !valida_a(cx, cy) {
            continue;
        }
        if t.e_muro(tile(e, 1, cx, cy)) {
            let novo = t.conexao_muro[mascara_muro(e, t, cx, cy)];
            e.superficie.mapa.definir(cx, cy, novo);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Ferramenta 2: Cavar/encher (`_ExpDig`)
// ---------------------------------------------------------------------------------------------

/// `_ExpDig(de, para)` (`1010:756e`): percorre de `de` até `para` (em diagonal). Cavar (subopção 0, ou
/// 1 com Ctrl): buraco novo na superfície (`_DigMyNewHole`), ou célula de terra no ninho
/// (`_DigTileB/R`; na linha 1 também `_MakeNewHoleB/R`). Encher: só nos ninhos.
fn cavar_ou_encher(e: &mut Estado, c: Contexto, plano: i32, sub: u8, de: (i32, i32), para: (i32, i32), ctrl: bool) -> Vec<Efeito> {
    let t = c.tabelas;
    let ta = c.amarela;
    let encher = (sub != subopcao::CAVAR) ^ ctrl;
    let mut efeitos = Vec::new();
    let aviso = |indice| Efeito::Aviso { lista: t.mensagens.lista, indice };
    let (mut x, mut y) = de;
    loop {
        if (0..2).contains(&plano) {
            if encher {
                efeitos.push(som(t.sons.erro));
                efeitos.push(aviso(t.mensagens.encher_na_superficie));
            } else if amarela::cavar_novo_buraco(e, ta, x, y) {
                efeitos.push(som(t.sons.cavar));
            } else {
                efeitos.push(som(t.sons.erro));
                efeitos.push(aviso(t.mensagens.sem_lugar_para_buraco));
            }
        } else if plano == 2 || plano == 3 {
            let cor = cor_do_plano(plano);
            let escavavel = amarela::e_escavavel(e, plano, x, y);
            if !encher && escavavel && y != 0 {
                comum::cavar(e, ta, cor, x, y);
                if y == 1 {
                    comum::novo_buraco(e, ta, &ta.borda_buraco, cor, x);
                }
                efeitos.push(som(t.sons.cavar));
            } else if encher && !escavavel && y != 0 {
                encher_celula(e, t, cor, x, y);
                efeitos.push(som(t.sons.encher));
            }
        }
        let k = get_dir(x, y, para.0, para.1);
        let (dx, dy) = d9(ta, k);
        x += dx;
        y += dy;
        if k == 0 {
            return efeitos;
        }
    }
}

/// Encher uma célula do ninho (parte de `_ExpDig`): apaga as formigas da lista do ninho naquela
/// célula (T = 0, sem cadáver), grava terra maciça, desconta a célula dos totais de escavação (se
/// `_TilesDug?` > 1; o centro de massa não é recalculado), alisa as quatro vizinhas e zera a saída.
fn encher_celula(e: &mut Estado, t: &TabelasExperimental, cor: Cor, x: i32, y: i32) {
    let lista = comum::lista_do_ninho(cor);
    let n = e.ninho(cor);
    if n.vida.em(x, y).is_some_and(|v| v != 0) {
        let l = e.lista_mut(lista);
        let usadas = l.usadas();
        for f in l.vagas[..usadas].iter_mut().rev() {
            if f.tipo != 0 && f.x as i32 == x && f.y as i32 == y {
                f.tipo = 0;
            }
        }
    }
    let n = e.ninho_mut(cor);
    n.mapa.definir(x, y, t.regras.terra_macica);
    n.vida.definir(x, y, 0);
    let col = e.colonia_mut(cor);
    if col.tiles_escavados > 1 {
        col.soma_x_escavada = (col.soma_x_escavada - x).max(0);
        col.soma_y_escavada = (col.soma_y_escavada - y).max(0);
        col.tiles_escavados -= 1;
    }
    comum::alisar(e, cor, x, y - 1);
    comum::alisar(e, cor, x + 1, y);
    comum::alisar(e, cor, x, y + 1);
    comum::alisar(e, cor, x - 1, y);
    e.ninho_mut(cor).saida.definir(x, y, 0);
}

// ---------------------------------------------------------------------------------------------
// Ferramenta 3: pôr formiga (`_ExpAddAnt`)
// ---------------------------------------------------------------------------------------------

/// `_ExpAddAnt(x, y)` (`1010:79fc`): T = `SRand1(8)` + 0x10 (+0x80 vermelha), sempre sorteado. Na
/// superfície só numa célula livre (`_IsClearTile(1, …)`), M = 2; no ninho, nunca na linha 0, cava a
/// célula se o tile for > 0x1B e põe com M = 2 no ninho da própria cor ou 7 no da outra. A formiga
/// entra no fim da lista (cheia: descartada).
fn por_formiga(e: &mut Estado, c: Contexto, plano: i32, sub: u8, x: i32, y: i32) -> Vec<Efeito> {
    let r = &c.tabelas.regras;
    let mut tipo = r.formiga_tipo + e.s.rand1(r.formiga_tipo_sorteio) as u8;
    if sub == subopcao::VERMELHA {
        tipo = tipo.wrapping_add(r.formiga_vermelha);
    }
    let vermelha = tipo > 0x80;
    let f = |modo| Formiga { x: x as u8, y: y as u8, modo, tipo, s: 0 };
    match plano {
        0 | 1 => {
            if !comum::celula_livre(e, 1, x, y) {
                return Vec::new();
            }
            e.adicionar_formiga(Lista::A, f(r.formiga_modo_superficie));
        }
        2 | 3 => {
            if y == 0 {
                return Vec::new();
            }
            let cor = cor_do_plano(plano);
            if e.ninho(cor).mapa.em(x, y).is_some_and(|v| v > r.cavar_formiga_acima_de) {
                comum::cavar(e, c.amarela, cor, x, y);
            }
            let da_casa = (cor == Cor::Vermelha) == vermelha;
            let modo = if da_casa { r.formiga_modo_proprio_ninho } else { r.formiga_modo_ninho_alheio };
            e.adicionar_formiga(comum::lista_do_ninho(cor), f(modo));
        }
        _ => {}
    }
    vec![som(c.tabelas.sons.formiga)]
}

// ---------------------------------------------------------------------------------------------
// Ferramenta 4: comida (`_IncFoodHere`)
// ---------------------------------------------------------------------------------------------

/// `_IncFoodHere(x, y)` (`1010:7f3a`) no plano mostrado. Limiar L = 0x48 na superfície, 0x10 no ninho.
/// Sem Ctrl: se o tile ≤ L + 2, soma uma unidade (abaixo de L: chão < 0x18 vira L, outro tile fica
/// como está) e conta mais um em `_FoodA/B/R`. Com Ctrl: um tile L..L + 3 vira chão `SRand1(8)`; só
/// na superfície desconta `_FoodA` (ou, com `_FoodA` < 1, `_FoodR`). Devolve se gravou.
pub fn por_comida(e: &mut Estado, c: Contexto, plano: i32, x: i32, y: i32, ctrl: bool) -> bool {
    let r = &c.tabelas.regras;
    let superficie = plano == 0 || plano == 1;
    let limiar = if superficie { r.comida_superficie } else { r.comida_ninho };
    let mut v = tile(e, plano, x, y);
    if !ctrl {
        if v > limiar + r.comida_somar_ate {
            return false;
        }
        if v >= limiar {
            v += 1;
        } else if v < r.comida_chao_abaixo_de {
            v = limiar;
        }
        match plano {
            0 | 1 => e.ambiente.comida_superficie += 1,
            2 => e.preta.comida += 1,
            _ => e.vermelha.comida += 1,
        }
    } else {
        if !(limiar..=limiar + r.comida_tirar_ate).contains(&v) {
            return false;
        }
        v = e.s.rand1(r.comida_tirar_sorteio);
        if superficie {
            if e.ambiente.comida_superficie >= 1 {
                e.ambiente.comida_superficie -= 1;
            } else if e.vermelha.comida > 0 {
                // O original testa `_MapPlane == 2` para escolher `_FoodB`, o que aqui nunca vale.
                e.vermelha.comida -= 1;
            }
        }
    }
    gravar_mapa(e, plano, x, y, v as u8);
    true
}

// ---------------------------------------------------------------------------------------------
// Ferramenta 5: cheiro (`_ExpIncSmell`, `_SmoothMany`)
// ---------------------------------------------------------------------------------------------

fn mapa_de_cheiro(e: &mut Estado, sub: u8) -> Option<&mut Grade> {
    let f = &mut e.feromonios;
    Some(match sub {
        subopcao::NINHO_PRETO => &mut f.ninho_b,
        subopcao::TRILHA_PRETA => &mut f.trilha_b,
        subopcao::NINHO_VERMELHO => &mut f.ninho_r,
        subopcao::TRILHA_VERMELHA => &mut f.trilha_r,
        subopcao::ALARME => &mut f.alarme,
        _ => return None,
    })
}

/// `_ExpIncSmell(x, y)` (`1010:7b9e`): só na superfície. Na célula (x ≫ 1, y ≫ 1) do mapa de cheiro da
/// subopção soma 70 (Ctrl: tira 40), limitado a 0..255; depois `_SmoothMany`.
fn por_cheiro(e: &mut Estado, c: Contexto, plano: i32, sub: u8, x: i32, y: i32, ctrl: bool) -> Vec<Efeito> {
    if plano > 1 {
        return Vec::new();
    }
    mudar_cheiro(e, &c.tabelas.regras, sub, x, y, ctrl);
    vec![som(c.tabelas.sons.cheiro)]
}

/// O miolo de `_ExpIncSmell` no tile (x, y) da superfície, no mapa de cheiro da subopção `sub`
/// (`subopcao::NINHO_PRETO`..`ALARME`): soma `cheiro_somar` (Ctrl: tira `cheiro_tirar`), limitado a
/// 0..255, e `_SmoothMany`. Não sorteia. Também é a escrita `depositar_cheiro`/`tirar_cheiro` dos
/// scripts (API de mods 0.26). Devolve se a célula existe.
pub fn mudar_cheiro(e: &mut Estado, r: &RegrasExperimental, sub: u8, x: i32, y: i32, ctrl: bool) -> bool {
    let (cx, cy) = (x >> 1, y >> 1);
    let Some(g) = mapa_de_cheiro(e, sub) else { return false };
    let Some(v) = g.em(cx, cy) else {
        suavizar_varias(g, cx, cy);
        return false;
    };
    let novo = if ctrl { v as i32 - r.cheiro_tirar } else { v as i32 + r.cheiro_somar };
    g.definir(cx, cy, novo.clamp(0, 255) as u8);
    suavizar_varias(g, cx, cy);
    true
}

/// `_SmoothMany(x, y)` (`1010:889c`): `_SmoothACell` na célula, em cima, à direita, embaixo, à
/// esquerda e de novo na célula, nessa ordem, no mesmo mapa.
fn suavizar_varias(g: &mut Grade, x: i32, y: i32) {
    for (cx, cy) in [(x, y), (x, y - 1), (x + 1, y), (x, y + 1), (x - 1, y), (x, y)] {
        suavizar_celula(g, cx, cy);
    }
}

/// `_SmoothACell(x, y)` (`1010:88f4`): média das quatro vizinhas (fora do mapa vale −1; soma de 16
/// bits dividida por 4 com truncamento para zero), somada ao valor da célula e dividida por 2
/// (truncamento para zero); limitada a 255 e gravada se a célula existe.
fn suavizar_celula(g: &mut Grade, x: i32, y: i32) {
    let v = |g: &Grade, x: i32, y: i32| g.em(x, y).map_or(-1, i32::from);
    let soma = v(g, x, y - 1) + v(g, x + 1, y) + v(g, x, y + 1) + v(g, x - 1, y);
    let novo = (soma / 4 + v(g, x, y)) / 2;
    if g.contem(x, y) {
        g.definir(x, y, novo.clamp(0, 255) as u8);
    }
}

// ---------------------------------------------------------------------------------------------
// Ferramenta 6: inseticida (`_ExpKillAnts`)
// ---------------------------------------------------------------------------------------------

/// `_ExpKillAnts(x, y)` (`1010:7d5c`): 8 jatos. Jato (subopção 1): todos em (x, y), sem sorteio. Spray
/// (0): cada um em (x + `SRand1(9)` − 4, y + `SRand1(9)` − 4), com um `SRand1(1000)` para o som. Em cada
/// célula válida: superfície = a formiga da lista A vira cadáver (`_DeadAntHere`), a aranha cuja célula
/// (`_SpidX ≫ 4`, `_SpidY ≫ 4`) é essa morre (`_KillSpider`), e a formiga-leão com centro ali morre
/// (`_KillAntLion`); ninho = a formiga da lista do ninho some, sem cadáver. A amarela não é atingida.
fn inseticida(e: &mut Estado, c: Contexto, plano: i32, sub: u8, x: i32, y: i32) -> Vec<Efeito> {
    let t = c.tabelas;
    let r = &t.regras;
    let mut efeitos = Vec::new();
    let jato = sub != subopcao::SPRAY;
    let meio = (r.espalhar_lado / 2) as i32;
    for _ in 0..r.inseticida_vezes {
        let (px, py, parametro) = if jato {
            (x, y, 0)
        } else {
            let px = e.s.rand1(r.espalhar_lado) + x - meio;
            let py = e.s.rand1(r.espalhar_lado) + y - meio;
            (px, py, e.s.rand1(r.inseticida_som_sorteio) + r.inseticida_som_base)
        };
        efeitos.push(Efeito::Som { id: t.sons.inseticida, parametro });
        if !posicao_valida(plano, px, py) {
            continue;
        }
        tirar_formiga(e, plano, px, py);
        match plano {
            0 | 1 => {
                if (e.aranha.x >> 4) as i32 == px && (e.aranha.y >> 4) as i32 == py {
                    inimigos::matar_aranha(e, c.inimigos);
                }
                let v = e.superficie.mapa.em(px, py).unwrap_or(0);
                if (r.leao_de..=r.leao_ate).contains(&v) {
                    // Sem formiga-leão na lista com esse centro, o original passa −1 a
                    // `_KillAntLion` (inimigos.md §1.4): ver `matar_formiga_leao_sem_entrada`.
                    match inimigos::achar_formiga_leao(e, px, py) {
                        Some(k) => inimigos::matar_formiga_leao(e, c.inimigos, k),
                        None => inimigos::matar_formiga_leao_sem_entrada(e),
                    }
                }
            }
            _ => {}
        }
    }
    efeitos
}

/// A parte de `_ExpKillAnts` que atinge a formiga da célula (x, y) do plano (já validada): na
/// superfície, a formiga da lista A vira cadáver (`_DeadAntHere`, que sorteia S); no ninho, a formiga
/// da lista do ninho some, sem cadáver. A amarela não é atingida. Também é a escrita
/// `remover_formiga` dos scripts (API de mods 0.26). Devolve se havia formiga.
pub fn tirar_formiga(e: &mut Estado, plano: i32, x: i32, y: i32) -> bool {
    match plano {
        0 | 1 => {
            if !e.superficie.vida.em(x, y).is_some_and(|v| v != 0) {
                return false;
            }
            let achada = achar_na_lista_a(e, x, y);
            e.tindex = achada.map_or(-1, |i| i as i32);
            let Some(i) = achada else { return false };
            let vermelha = e.superficie.lista.vagas[i].tipo & 0x80 != 0;
            cadaver_aqui(e, x, y, vermelha);
            e.superficie.lista.vagas[i].tipo = 0;
            e.superficie.vida.definir(x, y, 0);
            true
        }
        2 | 3 => {
            let cor = cor_do_plano(plano);
            let lista = comum::lista_do_ninho(cor);
            let v = e.ninho(cor).vida.em(x, y).unwrap_or(0);
            if v == 0 {
                return false;
            }
            let achada = achar_formiga(e, lista, x, y, v);
            e.tindex = achada.map_or(-1, |i| i as i32);
            let Some(i) = achada else { return false };
            e.lista_mut(lista).vagas[i].tipo = 0;
            e.ninho_mut(cor).vida.definir(x, y, 0);
            true
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------------------------
// Menu de ferramentas e trocas de modo
// ---------------------------------------------------------------------------------------------

/// Resultado de `_DoExpMenu` aplicado como em `_EditToolsMenu`/`_MapToolsMenu`/`_RibbonToolsMenu` no
/// Experimental: guarda a subopção (se o item tinha submenu e algo foi escolhido nele), e a ferramenta
/// 7 começa o jogo ([`comecar_jogo`]); depois `_CurExpTool` = a ferramenta (inclusive 7). Fora do
/// Experimental não faz nada.
pub fn escolher_ferramenta(
    e: &mut Estado,
    c: Contexto,
    ctl: &mut ControleExperimental,
    f: Ferramenta,
    subopcao: Option<u8>,
    plano_mapa: i32,
) -> Vec<Efeito> {
    if e.partida.tipo_jogo != TIPO_EXPERIMENTAL {
        return Vec::new();
    }
    if let Some(s) = subopcao {
        ctl.subopcoes[f.indice()] = s;
    }
    let efeitos = if f == Ferramenta::ComecarJogo { comecar_jogo(e, c, plano_mapa) } else { Vec::new() };
    e.partida.ferramenta_experimental = f as i16;
    efeitos
}

/// "Start Game" do menu do Experimental (item 7): `_CurGameType` = 1 (Quick Game no mesmo mundo),
/// `_InitYelloAnt` (a amarela renasce como operária no ninho preto, em `_RebornX/Y`) e, se o mapa mostra
/// a superfície (`_MapPlane` = 1), a marca da amarela é gravada de novo. `_CurExpTool` fica 7.
pub fn comecar_jogo(e: &mut Estado, c: Contexto, plano_mapa: i32) -> Vec<Efeito> {
    e.partida.tipo_jogo = TIPO_RAPIDO;
    iniciar_amarela(e, c);
    if plano_mapa == 1 {
        let a = e.amarela.clone();
        amarela::marcar_amarela(
            e,
            c.amarela,
            a.plano as i32,
            a.x as i32,
            a.y as i32,
            a.tipo as i32,
            a.dir as i32,
            0xFF,
        );
    }
    e.partida.ferramenta_experimental = Ferramenta::ComecarJogo as i16;
    vec![Efeito::Musica(c.tabelas.musicas.comecar_jogo)]
}

/// O botão de ferramentas num Quick Game (janela 10 `editToExp`, item `editToExpMode`): passa a
/// partida para o Experimental. Apaga a marca da amarela, `_CurGameType` = 3, `_InitYelloAnt` (a
/// amarela some do mapa: `_MeMode` = 2), `_InitSpider`. No Full Game o botão só mostra a janela 11
/// (`editNoExpMenu`, uma figura) e não muda nada; no Tutorial e no Experimental, nada.
pub fn quick_para_experimental(e: &mut Estado, c: Contexto) -> Vec<Efeito> {
    if e.partida.tipo_jogo != TIPO_RAPIDO {
        return Vec::new();
    }
    let a = e.amarela.clone();
    amarela::limpar_minha_vida(e, c.amarela, a.plano as i32, a.x as i32, a.y as i32, a.tipo as i32, a.dir as i32);
    e.partida.tipo_jogo = TIPO_EXPERIMENTAL;
    iniciar_amarela(e, c);
    e.amarela.modo = c.tabelas.regras.amarela_modo_experimental;
    iniciar_aranha(e, c);
    vec![som(c.tabelas.sons.para_experimental)]
}

/// `_InitYelloAnt` (`1030:6492`) sobre o estado do jogo: `_MeBase` = 0, `_MeEggCarried` = 0xFD,
/// `_MeDropAlarm` = 0, `_SetMyHealth(100)`; no Experimental a amarela fica fora do mapa (plano 1, (0x40,
/// 0x20), operária, `_MeMode` = 2, sem marca); no Full Game sem ninho, rainha alada numa célula livre
/// da superfície perto de (0x20, 0x20) (até 100 sorteios `Sk(16)`, `Sk(16)`, `Sk(8)`, `Sk(8)`; sem
/// lugar, (0x40, 0x20)); nos demais, operária no ninho preto em `_RebornX/Y`. Depois `_ResetYellowVars`.
pub fn iniciar_amarela(e: &mut Estado, c: Contexto) {
    let r = &c.tabelas.regras;
    let ta = c.amarela;
    e.amarela.base = 0;
    e.amarela.ovo_carregado = r.amarela_ovo_carregado;
    if e.amarela.soltando_alarme != 0 {
        e.amarela.soltando_alarme = 0;
        e.camada_edicao = -1;
    }
    amarela::definir_saude(e, ta, r.amarela_saude);
    if e.partida.tipo_jogo == TIPO_EXPERIMENTAL {
        let a = &mut e.amarela;
        a.plano = 1;
        a.modo = r.amarela_modo_experimental;
        a.x = r.amarela_x;
        a.y = r.amarela_y;
        a.tipo = r.amarela_tipo;
    } else {
        e.amarela.modo = 0;
        let (plano, x, y, tipo) = if e.partida.tipo_jogo == TIPO_COMPLETO && e.amarela.ninho_iniciado == 0 {
            let mut pos = (r.amarela_x as i32, r.amarela_y as i32);
            for _ in 0..100 {
                let x = e.s.mascara(16) - e.s.mascara(16) + 0x20;
                let y = e.s.mascara(8) - e.s.mascara(8) + 0x20;
                if e.superficie.mapa.em(x, y).is_some_and(|v| v < 0x10) {
                    pos = (x, y);
                    break;
                }
            }
            (1, pos.0, pos.1, r.amarela_tipo_alada as i32)
        } else {
            (2, e.amarela.renascer_x as i32, e.amarela.renascer_y as i32, r.amarela_tipo as i32)
        };
        amarela::marcar_amarela(e, ta, plano, x, y, tipo, 2, 0xFF);
    }
    let a = e.amarela.clone();
    amarela::encerrar_ordem(e, a.plano as i32, a.x as i32, a.y as i32);
}

/// `_InitSpider` (`1020:4668`): aranha parada em (0x400, 0x200) (×16), direção 0, 10 arrotos, sem alvo,
/// ligada fora do Tutorial; zera ciclos, mastigação, cadáver, vingança, ré e `_MeSMode`; destino do
/// jogador (0x40, 0x40). Não mexe em `_DeathCnt` nem em `_ChaseSpid`.
pub fn iniciar_aranha(e: &mut Estado, c: Contexto) {
    let r = &c.tabelas.regras;
    let ligada = (e.partida.tipo_jogo != 0) as i16;
    let a = &mut e.aranha;
    a.arrotos = r.aranha_arrotos;
    a.mastigando = 0;
    a.base_cadaver = 0;
    a.ciclo_pernas = 0;
    a.ciclo = 0;
    a.vinganca = 0;
    a.dir = 0;
    a.de_re = 0;
    a.ligada = ligada;
    a.x = r.aranha_x;
    a.y = r.aranha_y;
    a.estado = 0;
    a.alvo = -2;
    a.alvo_tipo = -1;
    a.destino_x = r.aranha_destino;
    a.destino_y = r.aranha_destino;
    e.amarela.modo_aranha = 0;
}

#[cfg(test)]
mod testes {
    use super::*;

    fn quintal() -> Estado {
        let mut e = Estado::vazio();
        e.partida.tipo_jogo = TIPO_EXPERIMENTAL;
        e.partida.ferramenta_experimental = 0;
        e
    }

    fn ctl() -> ControleExperimental {
        ControleExperimental::novo(TabelasExperimental::padrao())
    }

    #[test]
    fn dados_carregam_e_subopcoes_iniciais_sao_zero() {
        let t = TabelasExperimental::padrao();
        assert_eq!(t.conexao_muro[0], 0x60);
        assert_eq!(t.conexao_muro[15], 0x67);
        assert_eq!(&t.subopcoes_iniciais[1..7], &[0; 6]);
        assert_eq!(Ferramenta::de_i16(6), Some(Ferramenta::Inseticida));
        assert_eq!(Ferramenta::de_i16(8), None);
        assert_eq!(Ferramenta::de_i16(-1), None);
    }

    #[test]
    fn muro_liga_com_vizinhos() {
        let mut e = quintal();
        let mut k = ctl();
        e.partida.ferramenta_experimental = Ferramenta::Barreira as i16;
        let c = Contexto::padrao();
        pressionar(&mut e, c, &mut k, 1, 10, 10, false);
        // Sozinho: máscara 0 → 0x60.
        assert_eq!(e.superficie.mapa.em(10, 10), Some(0x60));
        // Arrasto para (12, 10): percorre (11, 10) e (12, 10).
        arrastar(&mut e, c, &mut k, 1, 12, 10, false);
        // (10,10): direita = muro → máscara 2 → 0x65; (11,10): esq + dir → 8 + 2 = 10 → 0x60;
        // (12,10): esquerda → 8 → 0x63.
        assert_eq!(e.superficie.mapa.em(10, 10), Some(0x65));
        assert_eq!(e.superficie.mapa.em(11, 10), Some(0x60));
        assert_eq!(e.superficie.mapa.em(12, 10), Some(0x63));
        assert_eq!(k.ultimo_ponto, (10, 10));
    }

    #[test]
    fn suavizacao_usa_menos_um_fora_do_mapa() {
        let mut g = Grade::nova((64, 32));
        g.definir(0, 0, 100);
        suavizar_celula(&mut g, 0, 0);
        // vizinhas: cima −1, direita 0, baixo 0, esquerda −1 → soma −2 → −2/4 = 0; (0 + 100)/2 = 50.
        assert_eq!(g.em(0, 0), Some(50));
    }
}
