//! Formiga amarela (a do jogador): `_DoAntSimY`, `_DoAntMoveY`, morte, reencarnação, troca de corpo e
//! a API de comandos do jogador (cliques e teclas) que a interface chama.
//!
//! Especificação: docs/spec/regras/formiga-amarela.md (números de seção citados como §n).
//!
//! Organização:
//! - [`TabelasAmarela`]: tabelas do executável e constantes (`data/amarela.json`, gerado por
//!   `tools/extrair_amarela.py`), guardadas em `RegrasOriginais::amarela` (com mods).
//! - Peças de mapa de vida e terreno próprias da amarela (`_SetMyLife`, `_SetLife`, `_DigMyTile`…):
//!   privadas. As rotinas que o original compartilha com as outras formigas (`_GetMap`, `_GetLife`,
//!   `_IsClearTile`, `_DigTileB/R`, `_MakeNewHole*`, `_PlaceEgg*`, `_DeadAntHere`, `_GetWinner`,
//!   `_PickupFoodA`, `_DropFoodA`, `_Recruit`, `_UnRecruit`, `_GetBestDir`...) estão em `rules/comum.rs`.
//! - Regras do tick: `impl RegrasAmarela for RegrasOriginais`.
//! - API pública usada por outras regras: [`lutar_com_amarela`] (`_YellowFight`; superfície e ninhos),
//!   [`trofalaxia`] (`_DoTroph`; superfície e ninho preto), [`matar_amarela`] (`_YellowDeath`; quintal,
//!   formiga-leão e aranha) e [`mover_marca`] (`_MoveMyLife`; formiga-leão e aranha).
//! - API de comandos do jogador: [`clicar_no_mapa`], [`ordem_ir`], [`ordem_agir`], [`comando`],
//!   [`tecla_de_comando`] e [`trocar_de_corpo`]. Estado da interface que não vai para o save fica em
//!   [`ControleJogador`].
//!
//! Redesenho e centralização da vista são da interface; músicas, efeitos sonoros (ids em `sons` de
//! `amarela.json`, docs/spec/regras/som.md), o tema do ninho e os quadros de mensagem viram [`Evento`]s. Sorteios (geradores S e R) acontecem na
//! mesma ordem do original, inclusive os das animações (§10.2, peculiaridade A5).

use std::sync::OnceLock;

use serde::Deserialize;

use super::comum::{
    self, celula_livre, dentro, e_pedrinha, em_casa as casa, get_dir, get_dis, lista_do_plano as area,
    nao_e_obstaculo, tile, vida, vida_bruta, Direcoes, TabelaLuta,
};
use super::estado::{AnimacaoDaAmarela, Estado, Evento, Feromonios, Grade, MensagemFaixa, QuadroAnimacao};
use super::geracao::Cor;
use super::tick::{RegrasAmarela, RegrasOriginais};
use crate::formats::save::{Formiga, Lista};
use crate::recursos::Recursos;

/// Nome do arquivo de dados (em `data/` ou num mod).
pub const ARQUIVO_TABELAS: &str = "amarela.json";

// ---------------------------------------------------------------------------------------------
// Dados
// ---------------------------------------------------------------------------------------------

/// Constantes das regras (bloco `regras` de `amarela.json`).
#[derive(Clone, Debug, Deserialize)]
pub struct ConstantesAmarela {
    /// `_MeScent` ao pegar comida.
    pub rastro_inicial: i16,
    /// `_MeScent` só diminui enquanto for maior que isto.
    pub rastro_minimo: i16,
    /// Alarme somado a cada passo com `_MeDropAlarm`.
    pub alarme_por_passo: u8,
    /// Teto de `_AlarmHere`.
    pub teto_alarme: u8,
    /// Vida gasta por ovo posto pela amarela rainha.
    pub custo_ovo: i16,
    pub vida_maxima: i16,
    /// Com vida ≤ isto a amarela sempre quer comida.
    pub fome_minima: i16,
    /// Com vida < isto a amarela pega (e come) ovos.
    pub vida_canibal: i16,
    /// `_MeHealthCntDwn` acima disto mata de fome/afogada.
    pub tempo_sem_vida: i16,
    /// Distância² (em 1/16 de célula) para virar aranha.
    pub raio_aranha: i64,
    /// Vida doada por `_EatMyFood(2)`.
    pub custo_doar: i16,
}

/// Quadros das animações que consomem sorteios (§10.2) e o estado de interface assumido.
#[derive(Clone, Debug, Deserialize)]
pub struct AnimacaoAmarela {
    /// [hipótese] `_win_IsWinInFront(0)`: a janela de edição está à frente. O port não tem a janela; o
    /// valor decide quantos quadros (e sorteios) as animações de morte e de loucura gastam.
    pub janela_edicao_na_frente: bool,
    pub quadros_luta: i32,
    pub quadros_choque: i32,
    pub quadros_morte_atras: i32,
    pub quadros_morte_frente: i32,
    pub quadros_morte_frente_musica: i32,
    pub quadros_loucura_frente: i32,
    pub quadros_loucura_atras: i32,
    /// Tempo real (ms) entre dois quadros de `_AnimYellowFight`: espera o relógio do Mac passar de
    /// "início + 6"; como ele anda de 3 em 3 a cada 55 ms, são 3 degraus = 165 ms. Vira
    /// `Evento::Espera` entre os quadros da luta e do choque, os que pedem som.
    #[serde(default = "espera_quadro_padrao")]
    pub espera_quadro_ms: u16,
    /// `_AnimYellowInsane` espera "início + 3" no relógio do Mac: 1 degrau = 55 ms por quadro.
    #[serde(default = "espera_loucura_padrao")]
    pub espera_loucura_ms: u16,
    /// Com música ligada, as mortes 1–9 saem no primeiro quadro (antes do sorteio), porque no
    /// caminho MCI do Windows `_mySongIsDone` devolve sempre verdadeiro (regras/som.md). Falso: faz
    /// todos os quadros, como o caminho Sound Blaster.
    #[serde(default = "verdadeiro")]
    pub morte_para_com_musica: bool,
}

fn espera_quadro_padrao() -> u16 {
    165
}

fn espera_loucura_padrao() -> u16 {
    55
}

fn verdadeiro() -> bool {
    true
}

/// Ids de música (`_myBeginSong`).
#[derive(Clone, Debug, Deserialize)]
pub struct MusicasAmarela {
    pub recrutar_5: u16,
    pub recrutar_10: u16,
    pub recrutar_todas: u16,
    pub liberar_metade: u16,
    pub liberar_todas: u16,
    pub morte_base: u16,
    pub morte_variacoes: u16,
    pub renascer: u16,
    pub fundar_inicio: u16,
    pub fundar_fim: u16,
    pub virou_rainha: u16,
}

/// Ids dos quadros de mensagem (SHARED tipo 4, decimal).
#[derive(Clone, Debug, Deserialize)]
pub struct MensagensAmarela {
    pub luta_atacou: u16,
    pub luta_atacada: u16,
    pub cercada: u16,
    pub fugiu: u16,
    pub pisada: u16,
    pub cortador: u16,
    pub afogada: u16,
    pub fome: u16,
    pub inseticida: u16,
    pub choque: u16,
    pub sem_corpo: u16,
    pub outra_colonia: u16,
    pub transferida: u16,
    /// Mensagens da faixa (`_EditMessage`): `_EatMyFood` 1 (trofalaxia), 2 (doação), 3 (ovo comido)
    /// e a passagem para o ninho vermelho (`_DoAntMoveY`).
    pub faixa_comida_recebida: MensagemFaixa,
    pub faixa_doacao: MensagemFaixa,
    pub faixa_canibalismo: MensagemFaixa,
    pub faixa_ninho_vermelho: MensagemFaixa,
}

/// Conteúdo de `data/amarela.json`.
#[derive(Clone, Debug, Deserialize)]
pub struct TabelasAmarela {
    /// `_Dx8`/`_Dy8`.
    pub dx8: [i8; 8],
    pub dy8: [i8; 8],
    /// `_Dx9`/`_Dy9`: índice = valor de `_GetDir` (0 = parado).
    pub dx9: [i8; 9],
    pub dy9: [i8; 9],
    /// `_absSearchDirs`.
    pub desvios_busca: [i8; 8],
    /// `_UnCarryCaste`: grupo sem a carga.
    pub descarregar_grupo: [u8; 16],
    /// Classe de luta por grupo (`10:22c6`).
    pub classe_luta: [u8; 16],
    /// Chance em 10 de "a" vencer, `[classe(a)][classe(b)]` (`10:22d6`).
    pub matriz_luta: [[u8; 4]; 4],
    /// Borda do buraco por direção (`10:230c`).
    pub borda_buraco: [u8; 8],
    pub regras: ConstantesAmarela,
    pub animacao: AnimacaoAmarela,
    pub musicas: MusicasAmarela,
    pub mensagens: MensagensAmarela,
    #[serde(default)]
    pub sons: SonsAmarela,
}

/// Ids dos efeitos sonoros (objetos tipo 5 do banco SOUND, `_myBeginSound`) pedidos por esta parte
/// (docs/spec/regras/som.md). Seção opcional no JSON: um id ausente não toca (mods anteriores à API 0.8).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SonsAmarela {
    /// Erro (1): ordem que fracassa, comando impossível, morte cercada ou em fuga.
    pub erro: Option<u16>,
    /// Ok/centralizar (15): Home, alarme, troca de corpo, alvo da aranha.
    pub ok: Option<u16>,
    /// Passagem entre os ninhos (1, em `_DoAntMoveY`).
    pub passagem: Option<u16>,
    /// Cavar ao entrar na terra (`_SetLife`, 19).
    pub cavar: Option<u16>,
    /// Pegar/largar ovo (28).
    pub ovo: Option<u16>,
    /// Pegar/largar comida (29).
    pub comida: Option<u16>,
    /// Pegar/largar pedrinha (30).
    pub pedrinha: Option<u16>,
    /// Comer (`_EatMyFood` 0 e 3, 44) e o arroto do canibalismo (10).
    pub comer: Option<u16>,
    pub arroto: Option<u16>,
    /// Luta (`_AnimYellowFight` código 0, com `S2` ≠ 0; 37) e choque (código 10; 49).
    pub luta: Option<u16>,
    pub choque: Option<u16>,
    /// `_YellowBirth`: ovo posto (28) e nascimento (49).
    pub nascimento_ovo: Option<u16>,
    pub nascimento: Option<u16>,
    /// Aranha: caça por faro e desligar (2), laser (41).
    pub aranha_faro: Option<u16>,
    pub aranha_laser: Option<u16>,
}

impl TabelasAmarela {
    /// Lê `amarela.json` (com mods).
    pub fn carregar(r: &Recursos) -> Result<TabelasAmarela, String> {
        let t: TabelasAmarela = r.dados(ARQUIVO_TABELAS)?;
        t.validar()?;
        Ok(t)
    }

    pub fn de_json(texto: &str) -> Result<TabelasAmarela, String> {
        let t: TabelasAmarela = serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_TABELAS}: {e}"))?;
        t.validar()?;
        Ok(t)
    }

    /// Cópia embutida de `data/amarela.json`, usada pelas regras do tick.
    pub fn padrao() -> &'static TabelasAmarela {
        static TABELAS: OnceLock<TabelasAmarela> = OnceLock::new();
        TABELAS.get_or_init(|| {
            TabelasAmarela::de_json(include_str!("../../../data/amarela.json")).expect("data/amarela.json inválido")
        })
    }

    fn validar(&self) -> Result<(), String> {
        comum::validar_direcoes(ARQUIVO_TABELAS, &self.dx8, &self.dy8)?;
        comum::validar_direcoes(ARQUIVO_TABELAS, &self.dx9, &self.dy9)?;
        if self.classe_luta.iter().any(|&c| c > 3) {
            return Err(format!("{ARQUIVO_TABELAS}: classe_luta fora de 0–3"));
        }
        if self.descarregar_grupo.iter().any(|&g| g > 15) {
            return Err(format!("{ARQUIVO_TABELAS}: descarregar_grupo fora de 0–15"));
        }
        if self.musicas.morte_variacoes == 0 {
            return Err(format!("{ARQUIVO_TABELAS}: morte_variacoes deve ser > 0"));
        }
        Ok(())
    }

    /// Tabelas de `_GetWinner`.
    fn luta(&self) -> TabelaLuta<'_> {
        TabelaLuta { classe: &self.classe_luta, chance: &self.matriz_luta }
    }

    /// `_Dx9/_Dy9[k]`.
    fn d9(&self, k: i32) -> (i32, i32) {
        let k = k.clamp(0, 8) as usize;
        (self.dx9[k] as i32, self.dy9[k] as i32)
    }
}

impl Direcoes for TabelasAmarela {
    fn deslocamentos(&self) -> (&[i8; 8], &[i8; 8]) {
        (&self.dx8, &self.dy8)
    }
}

// ---------------------------------------------------------------------------------------------
// Constantes de valores de tile (tiles-valores.md)
// ---------------------------------------------------------------------------------------------

const MARCA_AMARELA: u8 = 0xFF;
const MARCA_CAUDA: u8 = 0xFE;
const TIPO_RAINHA: i32 = 0x60;
const TIPO_RAINHA_ALADA: i32 = 0x40;
const TIPO_OPERARIA: i32 = 0x10;
const TIPO_SOLDADO: i32 = 0x30;
const ABERTURA_NINHO: u8 = 0x18;
const PASSAGEM_ENTRE_NINHOS: i32 = 0x14;
/// `_ELayerMode` do feromônio de alarme (`Estado::camada_edicao`).
pub const CAMADA_ALARME: i16 = 0;
const BURACO_QUINTAL: u8 = 0x50;
const CENTRO_FORMIGA_LEAO: i32 = 0x38;

// ---------------------------------------------------------------------------------------------
// Acesso a mapas e listas
// ---------------------------------------------------------------------------------------------

fn mapa_mut(e: &mut Estado, plano: i32) -> Option<&mut Grade> {
    match plano {
        0 | 1 => Some(&mut e.superficie.mapa),
        2 => Some(&mut e.ninho_b.mapa),
        3 => Some(&mut e.ninho_r.mapa),
        _ => None,
    }
}

fn grade_vida_mut(e: &mut Estado, plano: i32) -> Option<&mut Grade> {
    match plano {
        0 | 1 => Some(&mut e.superficie.vida),
        2 => Some(&mut e.ninho_b.vida),
        3 => Some(&mut e.ninho_r.vida),
        _ => None,
    }
}

fn por_tile(e: &mut Estado, plano: i32, x: i32, y: i32, v: u8) {
    if dentro(plano, x, y) {
        if let Some(g) = mapa_mut(e, plano) {
            g.definir(x, y, v);
        }
    }
}

/// `_GetAntIndex`: a vaga `i` se `0 ≤ i < _ListIndex`.
fn formiga_em(e: &Estado, plano: i32, i: i32) -> Option<Formiga> {
    let l = e.lista(area(plano));
    if i < 0 || i >= l.indice as i32 || i as usize >= l.capacidade() {
        return None;
    }
    Some(l.vagas[i as usize])
}

/// `_SetAntIndex`.
fn gravar_formiga(e: &mut Estado, plano: i32, i: i32, f: Formiga) {
    let l = e.lista_mut(area(plano));
    if i >= 0 && i < l.indice as i32 && (i as usize) < l.capacidade() {
        l.vagas[i as usize] = f;
    }
}

const VAGA_VAZIA: Formiga = Formiga { x: 0, y: 0, modo: 0, tipo: 0, s: 0 };

/// `_FindAntIndex` ([`comum::achar_formiga`]): última vaga com X, Y e T exatos, ou −1 (T fora de
/// 0..255, como o −1 de `_GetLife`, nunca casa).
fn achar_formiga(e: &Estado, plano: i32, x: i32, y: i32, t: i32) -> i32 {
    let Ok(t) = u8::try_from(t) else { return -1 };
    comum::achar_formiga(e, area(plano), x, y, t).map_or(-1, |i| i as i32)
}

/// `_AddAntTo?List` sem o mapa de vida (quem chama grava a vida). Devolve se coube.
fn acrescentar(e: &mut Estado, plano: i32, f: Formiga) -> bool {
    let limite = if plano < 2 { 1000 } else { 500 };
    let l = e.lista_mut(area(plano));
    if l.indice as i32 >= limite {
        return false;
    }
    l.adicionar(f)
}

/// [`comum::e_terra`] para um valor de [`tile`] (−1 fora do mapa).
fn e_terra(v: i32) -> bool {
    u8::try_from(v).is_ok_and(comum::e_terra)
}

/// [`comum::e_capim`] para um valor de [`tile`].
fn e_capim(v: i32) -> bool {
    u8::try_from(v).is_ok_and(comum::e_capim)
}

/// `_IsItFood` (superfície).
fn e_comida_superficie(e: &Estado, v: i32) -> bool {
    u8::try_from(v).is_ok_and(|v| comum::e_comida_superficie(casa(e), v))
}

/// `_IsThisFood`.
fn e_comida(e: &Estado, plano: i32, v: i32) -> bool {
    if v < 0 {
        return false;
    }
    if plano < 2 {
        e_comida_superficie(e, v)
    } else {
        (0x10..=0x13).contains(&v)
    }
}


/// `_IsItHole` (superfície, [`comum::e_buraco_superficie`]).
fn e_buraco(e: &Estado, x: i32, y: i32) -> bool {
    comum::e_buraco_superficie(e, x, y)
}

/// `_IsItAHole`: superfície → `_IsItHole`; ninho → só a abertura 0x18 na linha 0.
fn e_abertura(e: &Estado, plano: i32, x: i32, y: i32) -> bool {
    if plano < 2 {
        e_buraco(e, x, y)
    } else {
        y < 1 && tile(e, plano, x, y) == ABERTURA_NINHO as i32
    }
}

/// `_IsItDigable`.
pub(crate) fn e_escavavel(e: &Estado, plano: i32, x: i32, y: i32) -> bool {
    if plano < 2 || !(0..64).contains(&x) || !(0..64).contains(&y) {
        return false;
    }
    let v = tile(e, plano, x, y);
    e_terra(v) || e_capim(v)
}

/// `_TileCanBeMovedOn(plano, x, y, plano_alvo, ax, ay, cavar)` (§5.4).
#[allow(clippy::too_many_arguments)]
fn pode_pisar(e: &Estado, plano: i32, x: i32, y: i32, plano_alvo: i32, ax: i32, ay: i32, cavar: bool) -> bool {
    if plano < 2 {
        return dentro(plano, x, y) && comum::pode_pisar_superficie(e, x, y);
    }
    if !dentro(plano, x, y) {
        return false;
    }
    let v = tile(e, plano, x, y);
    let (passa, a_cavar) = if v < 0x19 || (0x30..=0x31).contains(&v) {
        (true, false)
    } else if cavar && (e_terra(v) || e_capim(v)) {
        (true, true)
    } else {
        (false, false)
    };
    if !passa {
        return false;
    }
    if y > 1 || plano_alvo != plano {
        return true;
    }
    if !cavar {
        if y != 0 {
            return x == ax || ay != 0;
        }
    } else if a_cavar {
        if ax != x {
            return false;
        }
        if y == 0 {
            let abaixo = tile(e, plano, x, 1);
            return !(0x20..0x2F).contains(&abaixo);
        }
        return true;
    } else if y != 0 {
        return true;
    }
    x == ax && y == ay
}

// ---------------------------------------------------------------------------------------------
// Mapa de vida da amarela (§2)
// ---------------------------------------------------------------------------------------------

/// `_SetLife`: grava `v`; num ninho, um valor ≥ 1 numa célula de terra ou de superfície fechada escava
/// a célula (`_DigMyTile`).
fn por_vida(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, v: u8) {
    if !dentro(plano, x, y) {
        return;
    }
    let Some(g) = grade_vida_mut(e, plano) else { return };
    g.definir(x, y, v);
    if (2..=3).contains(&plano) && v >= 1 {
        let m = tile(e, plano, x, y);
        if e_terra(m) || e_capim(m) {
            cavar_minha_celula(e, t, plano, x, y);
            e.pedir_som(t.sons.cavar);
        }
    }
}

/// `_ClearLife`: apaga a célula se ela contém exatamente `tipo`.
fn limpar_vida(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, tipo: u8) {
    if vida(e, plano, x, y) == tipo as i32 {
        por_vida(e, t, plano, x, y, 0);
    }
}

fn cauda(t: &TabelasAmarela, x: i32, y: i32, dir: i32) -> (i32, i32) {
    let (dx, dy) = t.d8(dir ^ 4);
    (x + dx, y + dy)
}

/// `_ClearMyLife`: apaga a célula só se contém 0xFF (e a cauda só se contém 0xFE).
pub(crate) fn limpar_minha_vida(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, tipo: i32, dir: i32) {
    if vida_bruta(e, plano, x, y) == Some(MARCA_AMARELA) {
        por_vida(e, t, plano, x, y, 0);
    }
    if tipo == TIPO_RAINHA {
        let (cx, cy) = cauda(t, x, y, dir);
        if vida_bruta(e, plano, cx, cy) == Some(MARCA_CAUDA) {
            por_vida(e, t, plano, cx, cy, 0);
        }
    }
}

/// `_SetMyLife`: grava `v` (e a cauda da rainha) e, se `v ≠ 0`, copia posição, tipo e direção para as `_Me*`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn marcar_amarela(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, tipo: i32, dir: i32, v: u8) {
    if !dentro(plano, x, y) {
        return;
    }
    por_vida(e, t, plano, x, y, v);
    if tipo == TIPO_RAINHA {
        let (cx, cy) = cauda(t, x, y, dir);
        por_vida(e, t, plano, cx, cy, if v == MARCA_AMARELA { MARCA_CAUDA } else { v });
    }
    if v != 0 {
        let a = &mut e.amarela;
        a.x = x as i16;
        a.y = y as i16;
        a.dir = dir as i16;
        a.tipo = tipo as i16;
        a.plano = plano as i16;
    }
}

/// `_MoveMyLife(plano, x, y, tipo, dir)` (§2): apaga a marca atual e marca a nova célula (plano 0 vira
/// 1). Também usada pela formiga-leão e pela aranha antes de `_YellowDeath`.
pub(crate) fn mover_marca(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, tipo: i32, dir: i32) {
    let a = e.amarela.clone();
    limpar_minha_vida(e, t, a.plano as i32, a.x as i32, a.y as i32, a.tipo as i32, a.dir as i32);
    let p = if plano == 0 { 1 } else { plano };
    if !dentro(p, x, y) {
        return;
    }
    por_vida(e, t, p, x, y, MARCA_AMARELA);
    if tipo == TIPO_RAINHA {
        let (cx, cy) = cauda(t, x, y, dir);
        por_vida(e, t, p, cx, cy, MARCA_CAUDA);
    }
    let a = &mut e.amarela;
    a.x = x as i16;
    a.y = y as i16;
    a.dir = dir as i16;
    a.tipo = tipo as i16;
    a.plano = p as i16;
}

/// Regrava a marca da amarela na posição atual (`_SetMyLife(_MePlane, _MeLoc, _MeType, _MeDir, v)`).
fn remarcar(e: &mut Estado, t: &TabelasAmarela, v: u8) {
    let a = e.amarela.clone();
    marcar_amarela(e, t, a.plano as i32, a.x as i32, a.y as i32, a.tipo as i32, a.dir as i32, v);
}

/// `_ClearMyLife` na posição atual.
fn desmarcar(e: &mut Estado, t: &TabelasAmarela) {
    let a = e.amarela.clone();
    limpar_minha_vida(e, t, a.plano as i32, a.x as i32, a.y as i32, a.tipo as i32, a.dir as i32);
}

/// `_SetMyHealth(v)` (`1020:8c70`, §3), para outras partes (lições do tutorial).
pub(crate) fn definir_saude(e: &mut Estado, t: &TabelasAmarela, v: i32) {
    definir_vida(e, t, v);
}

/// `_SetMyHealth` (§3).
fn definir_vida(e: &mut Estado, t: &TabelasAmarela, v: i32) {
    let maxima = t.regras.vida_maxima as i32;
    let mut v = v;
    if e.partida.nunca_com_fome != 0 {
        v = maxima;
    }
    if v > 0 {
        e.amarela.saude_contagem = 0;
    }
    v = v.clamp(0, maxima);
    e.amarela.saude = v as i16;
    recalcular_fome(e, t);
}

/// `_MeWantFood` = 0 se `vida > _MeWarnHealth` e `vida > 9`, senão 1.
fn recalcular_fome(e: &mut Estado, t: &TabelasAmarela) {
    let a = &mut e.amarela;
    a.quer_comida = if a.aviso_saude < a.saude && a.saude > t.regras.fome_minima { 0 } else { 1 };
}

/// `_ResetYellowVars(plano, x, y)`: encerra a ordem.
pub(crate) fn encerrar_ordem(e: &mut Estado, plano: i32, x: i32, y: i32) {
    let a = &mut e.amarela;
    a.plano = plano as i16;
    a.alvo_plano = plano as i16;
    a.x = x as i16;
    a.ultimo_x = x as i16;
    a.alvo_x = x as i16;
    a.y = y as i16;
    a.ultimo_y = y as i16;
    a.alvo_y = y as i16;
    a.comando = 0;
    a.mover = 0;
    a.dis = 0;
    a.dis_anterior = 0;
    a.passos = 0;
    a.contorno_contagem = -2;
}

/// `_GotoMyAnt`/`_CenterAnt`: a parte que não é desenho (`_LessonTemp = 1`). Mover a vista é da interface.
/// `_GotoMyAnt` (`1000:a764`): grava `_LessonTemp = 1` e pede à interface que leve a vista até a
/// amarela (o movimento da vista é da interface: `core::partida`).
fn centralizar(e: &mut Estado) {
    e.tutorial.temporario = 1;
    e.pedir_centralizar_na_amarela();
}

// ---------------------------------------------------------------------------------------------
// Escavação, buracos e mapa de saída (ninho.md §7)
// ---------------------------------------------------------------------------------------------

use comum::cor_do_plano as cor_do_ninho;

/// `_MakeNewHoleB/R(c)` ([`comum::novo_buraco`]).
fn novo_buraco(e: &mut Estado, t: &TabelasAmarela, cor: Cor, c: i32) {
    comum::novo_buraco(e, t, &t.borda_buraco, cor, c);
}

/// `_CreateNewHole(x, y)` (ninho.md 7.5): o buraco que a rainha alada cava na própria célula.
fn criar_buraco(e: &mut Estado, t: &TabelasAmarela, x: i32, y: i32) {
    if !(1..127).contains(&x) || !(1..63).contains(&y) {
        return;
    }
    if !casa(e) {
        por_tile(e, 1, x, y, BURACO_QUINTAL);
        comum::por_borda_do_buraco(e, t, &t.borda_buraco, x, y);
    } else {
        por_tile(e, 1, x, y, 0x59);
    }
    // A coluna do ninho é o y do buraco (`_HoleMap?[y] = x`); o vermelho tem `_DigTileR` embutido.
    let cor = if x < 64 { Cor::Preta } else { Cor::Vermelha };
    let col = e.colonia_mut(cor);
    col.buracos[y as usize] = x as u8;
    comum::cavar(e, t, cor, y, 1);
    let col = e.colonia_mut(cor);
    col.ultimo_buraco = (x as i16, y as i16);
    col.ultima_saida = (y as i16, 0);
}

/// `_DigMyNewHole(x, y)`: exige 3 × 3 livre (quintal) ou tile < 200 (casa).
pub(crate) fn cavar_novo_buraco(e: &mut Estado, t: &TabelasAmarela, x: i32, y: i32) -> bool {
    if !(1..128).contains(&x) || !(1..64).contains(&y) {
        return false;
    }
    let ok = if casa(e) { tile(e, 1, x, y) < 200 } else { comum::livre_3x3(e, t, x, y) };
    if ok {
        criar_buraco(e, t, x, y);
    }
    ok
}

/// `_DigMyTile(plano, x, y)`: a escavação que a amarela faz ao entrar numa célula de terra.
fn cavar_minha_celula(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32) {
    if !e_escavavel(e, plano, x, y) {
        return;
    }
    let cor = cor_do_ninho(plano);
    if y < 2 {
        e.ninho_mut(cor).mapa.definir(x, 0, ABERTURA_NINHO);
        novo_buraco(e, t, cor, x);
        if y != 1 {
            return;
        }
    }
    // `_DigTileB`; no vermelho, a mesma conta embutida (sem a passagem, que é só do preto).
    comum::cavar(e, t, cor, x, y);
}

/// `_PlaceEggB/R(x, y, estágio)` ([`comum::por_ovo`]).
fn por_ovo(e: &mut Estado, t: &TabelasAmarela, cor: Cor, x: i32, y: i32, estagio: u8) {
    comum::por_ovo(e, t, cor, x, y, estagio);
}

use comum::gastar_reserva;

// ---------------------------------------------------------------------------------------------
// Superfície: comida, cheiro, cadáver, luta (superficie.md)
// ---------------------------------------------------------------------------------------------

use comum::largar_comida_superficie as por_comida_superficie;
use comum::pegar_comida_superficie as tirar_comida_superficie;

/// `_JamScentBT/RT`: a célula de cheiro do tile fica com o maior valor.
fn depositar_trilha(e: &mut Estado, cor: Cor, x: i32, y: i32, v: u8) {
    let (cx, cy) = Feromonios::celula(x, y);
    let g = e.feromonios.trilha_mut(cor);
    if let Some(c) = g.em(cx, cy) {
        g.definir(cx, cy, c.max(v));
    }
}

/// `_AlarmHere`: soma e limita ao teto.
fn somar_alarme(e: &mut Estado, t: &TabelasAmarela, x: i32, y: i32) {
    let (cx, cy) = Feromonios::celula(x, y);
    let g = &mut e.feromonios.alarme;
    if let Some(c) = g.em(cx, cy) {
        let v = (c as u16 + t.regras.alarme_por_passo as u16).min(t.regras.teto_alarme as u16);
        g.definir(cx, cy, v as u8);
    }
}

/// `_DeadAntHere(x, y, cor)` ([`comum::cadaver_aqui`]); `cor` ≠ 0 = vermelha.
fn cadaver(e: &mut Estado, x: i32, y: i32, cor: i32) {
    comum::cadaver_aqui(e, x, y, cor != 0);
}

/// `_GetWinner(Ta, Tb)` ([`comum::vencedor_da_luta`]): "a" é a da célula.
fn vencedor(e: &mut Estado, t: &TabelasAmarela, ta: i32, tb: i32) -> i32 {
    comum::vencedor_da_luta(e, t.luta(), ta as u8, tb as u8) as i32
}

use comum::recrutar;

/// `_UnRecruit(p)` ([`comum::desrecrutar`]).
fn liberar(e: &mut Estado, todas: bool) {
    comum::desrecrutar(e, todas as i32);
}

// ---------------------------------------------------------------------------------------------
// Caminho (§5)
// ---------------------------------------------------------------------------------------------

fn cavando(e: &Estado) -> bool {
    e.amarela.comando == 2
}

/// `_GetMyBestDirs(plano, x, y, ax, ay)` (§5.2).
fn melhor_direcao(e: &Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, ax: i32, ay: i32) -> i32 {
    let mut dist = get_dis(x, y, ax, ay);
    if dist <= 0 {
        return -1;
    }
    let a = &e.amarela;
    let (gp, gx, gy) = (a.alvo_plano as i32, a.alvo_x as i32, a.alvo_y as i32);
    let cavar = cavando(e);
    let mut livre = -1;
    let mut bloqueada = -2;
    for k in 0..8 {
        let (dx, dy) = t.d8(k);
        let (nx, ny) = (x + dx, y + dy);
        if !pode_pisar(e, plano, nx, ny, gp, gx, gy, cavar) {
            continue;
        }
        let d2 = get_dis(nx, ny, ax, ay);
        if d2 < dist {
            dist = d2;
            if vida(e, plano, nx, ny) < 1 && celula_livre(e, plano, nx, ny) {
                livre = k;
            } else {
                bloqueada = k;
            }
        }
    }
    if livre >= 0 {
        livre
    } else {
        bloqueada
    }
}

/// `_CheckMyBestDirs`: simula o guloso até 64 passos; −2 se empacou, −1 se chegou ou completou.
fn checar_caminho(e: &Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, ax: i32, ay: i32) -> i32 {
    let mut d = melhor_direcao(e, t, plano, x, y, ax, ay);
    if d >= 0 {
        let (dx, dy) = t.d8(d);
        let (mut cx, mut cy) = (x + dx, y + dy);
        for _ in 0..64 {
            d = melhor_direcao(e, t, plano, cx, cy, ax, ay);
            if d < 0 {
                break;
            }
            let (dx, dy) = t.d8(d);
            cx += dx;
            cy += dy;
        }
    }
    if d >= 0 {
        -1
    } else {
        d
    }
}

/// `_GetMyRandDirs(&rot, &dirc, plano, x, y, ax, ay)` (§5.3): contorno, sem sorteio.
fn contornar(e: &mut Estado, plano: i32, x: i32, y: i32, ax: i32, ay: i32, t: &TabelasAmarela) -> i32 {
    let dist = get_dis(x, y, ax, ay);
    if dist <= 0 {
        return -1;
    }
    let a = &e.amarela;
    let (gp, gx, gy) = (a.alvo_plano as i32, a.alvo_x as i32, a.alvo_y as i32);
    let (lx, ly) = (a.ultimo_x as i32, a.ultimo_y as i32);
    let cavar = cavando(e);
    let mut ok = [false; 8];
    for (k, ok_k) in ok.iter_mut().enumerate() {
        let (dx, dy) = t.d8(k as i32);
        let (nx, ny) = (x + dx, y + dy);
        *ok_k = !(nx == lx && ny == ly) && pode_pisar(e, plano, nx, ny, gp, gx, gy, cavar);
    }
    if !ok.iter().any(|&v| v) {
        return -2;
    }
    let rot = e.amarela.contorno_rot;
    let mut a_ = (e.amarela.contorno_dir as i32) & 7;
    let mut b_ = a_;
    let rumo = get_dir(x, y, ax, ay) - 1;
    if rot == 0 {
        for _ in 0..8 {
            if ok[a_ as usize] {
                e.amarela.contorno_dir = rumo as i16;
                e.amarela.contorno_rot = 1;
                return a_;
            }
            if ok[b_ as usize] {
                e.amarela.contorno_dir = rumo as i16;
                e.amarela.contorno_rot = -1;
                return b_;
            }
            a_ = (a_ + 1) & 7;
            b_ = (b_ - 1) & 7;
        }
    } else {
        for _ in 0..8 {
            let c = if rot >= 1 { a_ } else { b_ };
            if ok[c as usize] {
                let (dx, dy) = t.d8(c);
                if get_dis(x + dx, y + dy, ax, ay) > dist {
                    return c;
                }
                e.amarela.contorno_dir = rumo as i16;
                e.amarela.contorno_rot = 0;
                return c;
            }
            a_ = (a_ + 1) & 7;
            b_ = (b_ - 1) & 7;
        }
    }
    -1
}

/// Máquina de contorno de `_GetMyDir` (§5.3) rumo ao alvo imediato (ax, ay).
fn direcao_com_contorno(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, ax: i32, ay: i32) -> i32 {
    if e.amarela.contorno_contagem < 0 {
        let mut d = melhor_direcao(e, t, plano, x, y, ax, ay);
        if d == -2 && e.amarela.contorno_contagem == -2 {
            e.amarela.contorno_dir = (get_dir(x, y, ax, ay) - 1) as i16;
            e.amarela.contorno_contagem = 16;
            e.amarela.contorno_rot = 0;
            d = contornar(e, plano, x, y, ax, ay, t);
        }
        return d;
    }
    let d = if checar_caminho(e, t, plano, x, y, ax, ay) == -2 {
        contornar(e, plano, x, y, ax, ay, t)
    } else {
        e.amarela.contorno_contagem = -1;
        melhor_direcao(e, t, plano, x, y, ax, ay)
    };
    e.amarela.contorno_contagem -= 1;
    d
}

/// `_GetMyDir(plano, x, y, plano_destino, dx, dy)` (§5.1).
#[allow(clippy::too_many_arguments)]
fn direcao_da_ordem(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, pd: i32, dx: i32, dy: i32) -> i32 {
    let (ax, ay) = if plano < 2 {
        if pd < 2 {
            (dx, dy)
        } else {
            let b = if pd == 2 { e.preta.ultimo_buraco } else { e.vermelha.ultimo_buraco };
            (b.0 as i32, b.1 as i32)
        }
    } else if pd == plano {
        (dx, dy)
    } else {
        let s = if plano == 2 { e.preta.ultima_saida } else { e.vermelha.ultima_saida };
        (s.0 as i32, s.1 as i32)
    };
    direcao_com_contorno(e, t, plano, x, y, ax, ay)
}

// ---------------------------------------------------------------------------------------------
// Pegar, largar e comer (§7)
// ---------------------------------------------------------------------------------------------

/// `_FindEggAt(plano, x, y)`: (vaga, T) do ovo na célula. A vaga pode faltar (o original devolve o T
/// do mapa de vida mesmo sem achar a vaga).
fn procurar_ovo(e: &Estado, plano: i32, x: i32, y: i32) -> Option<(i32, u8)> {
    let v = vida(e, plano, x, y);
    let l = e.lista(area(plano));
    if v >= 0 && (1..=7).contains(&(v & 0x7F)) && v != 0xFF && v != 0xFE {
        let i = achar_formiga(e, plano, x, y, v);
        return Some((i, v as u8));
    }
    (0..l.usadas())
        .rev()
        .find(|&i| {
            let f = &l.vagas[i];
            f.x as i32 == x && f.y as i32 == y && (1..=7).contains(&(f.tipo & 0x7F))
        })
        .map(|i| (i as i32, l.vagas[i].tipo))
}

/// `_IsLiftable`: comida, pedrinha ou ovo na célula.
fn levantavel(e: &Estado, plano: i32, x: i32, y: i32) -> bool {
    let ovo = procurar_ovo(e, plano, x, y).is_some();
    let v = tile(e, plano, x, y);
    e_comida(e, plano, v) || e_pedrinha(plano, v) || ovo
}

/// `_PickupMyRock(plano, x, y)` (§7.1).
fn pegar_pedrinha(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32) -> bool {
    let tipo = e.amarela.tipo as i32;
    if tipo != TIPO_OPERARIA && tipo != TIPO_SOLDADO {
        return false;
    }
    let v = tile(e, plano, x, y);
    if plano < 2 {
        if !e_pedrinha(plano, v) {
            return false;
        }
        let (cor, plano_ninho) = if x < 64 { (Cor::Preta, 2) } else { (Cor::Vermelha, 3) };
        let tampava = (0..64).contains(&y) && e.colonia(cor).buracos[y as usize] as i32 == x;
        if tampava {
            por_tile(e, plano_ninho, y, 0, ABERTURA_NINHO);
            let novo = if casa(e) { e.s.rand1(7) as u8 + 0x59 } else { BURACO_QUINTAL };
            por_tile(e, 1, x, y, novo);
        } else {
            let novo = if casa(e) { 0 } else { e.s.mascara(16) as u8 };
            por_tile(e, 1, x, y, novo);
        }
    } else if v == 0x30 {
        let novo = e.s.mascara(8) as u8;
        por_tile(e, plano, x, y, novo);
    } else if v == 0x31 {
        por_tile(e, plano, x, y, ABERTURA_NINHO);
        let cor = cor_do_ninho(plano);
        let bx = if (0..64).contains(&x) { e.colonia(cor).buracos[x as usize] as i32 } else { 0 };
        por_tile(e, 1, bx, x, BURACO_QUINTAL);
    } else {
        return false;
    }
    e.amarela.tipo = if tipo == TIPO_OPERARIA { 0x28 } else { 0x48 };
    e.pedir_som(t.sons.pedrinha);
    true
}

/// `_PickupMyFood(plano, x, y)` (§7.1).
fn pegar_comida(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32) -> bool {
    let v = tile(e, plano, x, y);
    if !e_comida(e, plano, v) {
        return false;
    }
    let com_fome = e.amarela.quer_comida != 0;
    let tipo = e.amarela.tipo as i32;
    if !com_fome && tipo != TIPO_OPERARIA && tipo != TIPO_SOLDADO {
        return false;
    }
    if plano < 2 {
        tirar_comida_superficie(e, x, y);
        if !com_fome {
            let a = &mut e.amarela;
            a.rastro = t.regras.rastro_inicial;
            let b = e.preta.ultimo_buraco;
            let d = get_dis(x, y, b.0 as i32, b.1 as i32);
            e.amarela.dis = d as i16;
            e.amarela.dis_anterior = (d + 1) as i16;
            // Peculiaridade A6: sempre a trilha preta.
            depositar_trilha(e, Cor::Preta, x, y, t.regras.rastro_inicial as u8);
        }
    } else {
        let novo = if v == 0x10 { e.s.mascara(8) as u8 } else { (v - 1) as u8 };
        por_tile(e, plano, x, y, novo);
        let c = e.colonia_mut(cor_do_ninho(plano));
        if c.comida > 0 {
            c.comida -= 1;
        }
    }
    if com_fome {
        e.amarela.comeu_tipo = 0;
    } else {
        e.pedir_som(t.sons.comida);
        e.amarela.tipo += 8;
    }
    true
}

/// Busca de onde largar (comum a `_DropMyFood`, `_DropMyRock`, `_DropMyEgg`): começa na direção do
/// destino (ou `_MeDir`) e tenta os desvios de `_absSearchDirs`; sem vizinha, tenta a própria célula.
/// `serve(e, x, y, propria)` decide se a célula serve. Devolve (x, y, direção).
#[allow(clippy::too_many_arguments)]
fn onde_largar(
    e: &Estado,
    t: &TabelasAmarela,
    plano: i32,
    x: i32,
    y: i32,
    ax: i32,
    ay: i32,
    serve: &dyn Fn(&Estado, i32, i32, bool) -> bool,
) -> Option<(i32, i32, i32)> {
    let k = get_dir(x, y, ax, ay);
    let base = if k < 1 { e.amarela.dir as i32 } else { k - 1 };
    for &desvio in &t.desvios_busca {
        let d = (base + desvio as i32) & 7;
        let (dx, dy) = t.d8(d);
        let (nx, ny) = (x + dx, y + dy);
        if dentro(plano, nx, ny) && vida(e, plano, nx, ny) < 0 && serve(e, nx, ny, false) {
            return Some((nx, ny, d));
        }
    }
    if dentro(plano, x, y) && serve(e, x, y, true) {
        return Some((x, y, base));
    }
    None
}

/// `_DropMyFood` (§7.1).
#[allow(clippy::too_many_arguments)]
fn largar_comida(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, ax: i32, ay: i32) -> bool {
    let tipo = e.amarela.tipo as i32;
    if tipo != 0x18 && tipo != 0x38 {
        return false;
    }
    let serve = |e: &Estado, cx: i32, cy: i32, _: bool| {
        let v = tile(e, plano, cx, cy);
        (e_comida(e, plano, v) && v & 3 <= 2) || celula_livre(e, plano, cx, cy) || v == CENTRO_FORMIGA_LEAO
    };
    let Some((cx, cy, d)) = onde_largar(e, t, plano, x, y, ax, ay, &serve) else { return false };
    e.amarela.dir = d as i16;
    if plano < 2 {
        por_comida_superficie(e, cx, cy);
    } else {
        let v = tile(e, plano, cx, cy);
        let novo = if e_comida(e, plano, v) && v & 3 <= 2 { v + 1 } else { 0x10 };
        por_tile(e, plano, cx, cy, novo as u8);
        if plano == 2 {
            e.preta.comida += 1;
        } else {
            e.vermelha.comida += 1;
        }
    }
    e.amarela.tipo &= !8;
    e.pedir_som(t.sons.comida);
    true
}

/// `_DropPebble(plano, x, y)`.
fn soltar_pedrinha(e: &mut Estado, plano: i32, x: i32, y: i32) {
    let novo = if plano < 2 {
        if e_buraco(e, x, y) && (0..64).contains(&y) {
            if x < 64 && e.preta.buracos[y as usize] as i32 == x {
                por_tile(e, 2, y, 0, 0x31);
            } else if e.vermelha.buracos[y as usize] as i32 == x {
                por_tile(e, 3, y, 0, 0x31);
            }
        }
        0x51
    } else if e_abertura(e, plano, x, y) {
        let bx = if (0..64).contains(&x) { e.colonia(cor_do_ninho(plano)).buracos[x as usize] as i32 } else { 0 };
        por_tile(e, 1, bx, x, 0x51);
        0x31
    } else {
        0x30
    };
    por_tile(e, plano, x, y, novo);
}

/// `_DropMyRock` (§7.1).
#[allow(clippy::too_many_arguments)]
fn largar_pedrinha(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, ax: i32, ay: i32) -> bool {
    let tipo = e.amarela.tipo as i32;
    if tipo != 0x28 && tipo != 0x48 {
        return false;
    }
    let serve = |e: &Estado, cx: i32, cy: i32, _: bool| {
        if celula_livre(e, plano, cx, cy) {
            return true;
        }
        let v = tile(e, plano, cx, cy);
        !e_comida(e, plano, v) && !e_pedrinha(plano, v) && (e_abertura(e, plano, cx, cy) || v == CENTRO_FORMIGA_LEAO)
    };
    let Some((cx, cy, d)) = onde_largar(e, t, plano, x, y, ax, ay, &serve) else { return false };
    e.amarela.dir = d as i16;
    soltar_pedrinha(e, plano, cx, cy);
    e.amarela.tipo -= 0x18;
    e.pedir_som(t.sons.pedrinha);
    true
}

/// `_DropMyEgg` (§7.1).
#[allow(clippy::too_many_arguments)]
fn largar_ovo(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, ax: i32, ay: i32) -> bool {
    if e.amarela.tipo != 8 {
        return false;
    }
    let serve =
        |e: &Estado, cx: i32, cy: i32, _: bool| celula_livre(e, plano, cx, cy) || tile(e, plano, cx, cy) == CENTRO_FORMIGA_LEAO;
    let Some((cx, cy, d)) = onde_largar(e, t, plano, x, y, ax, ay, &serve) else { return false };
    let ovo = e.amarela.ovo_carregado as u8;
    if !acrescentar(e, plano, Formiga { x: cx as u8, y: cy as u8, modo: 8, tipo: ovo, s: 0 }) {
        return false;
    }
    por_vida(e, t, plano, cx, cy, ovo);
    e.amarela.dir = d as i16;
    e.amarela.tipo = TIPO_OPERARIA as i16;
    e.amarela.ovo_carregado = 0xFD;
    let a = e.amarela.clone();
    if dentro(a.plano as i32, a.x as i32, a.y as i32) {
        por_vida(e, t, a.plano as i32, a.x as i32, a.y as i32, MARCA_AMARELA);
    }
    e.pedir_som(t.sons.ovo);
    true
}

/// `_DropMyObject` (usado por `_YellowDeath`).
fn largar_objeto(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32) -> bool {
    match e.amarela.tipo {
        0x28 | 0x48 => largar_pedrinha(e, t, plano, x, y, x, y),
        8 => largar_ovo(e, t, plano, x, y, x, y),
        0x18 | 0x38 => largar_comida(e, t, plano, x, y, x, y),
        _ => false,
    }
}

/// `_TryMyDropOrLift(plano, x, y)` (§7.1): 1 = feito, −1 = falhou, 0 = nada a fazer.
fn tentar_largar_ou_pegar(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32) -> i32 {
    let (mx, my) = (e.amarela.x as i32, e.amarela.y as i32);
    if e.amarela.tipo & 8 != 0 {
        let ok = match e.amarela.tipo {
            0x28 | 0x48 => largar_pedrinha(e, t, plano, mx, my, x, y),
            8 => largar_ovo(e, t, plano, mx, my, x, y),
            0x18 | 0x38 => largar_comida(e, t, plano, mx, my, x, y),
            _ => false,
        };
        return if ok { 1 } else { -1 };
    }
    let mut ok = false;
    if e.amarela.tipo as i32 == TIPO_OPERARIA || e.amarela.saude < t.regras.vida_canibal {
        e.amarela.ovo_carregado = 0xFD;
        if let Some((i, ovo)) = procurar_ovo(e, plano, x, y) {
            gravar_formiga(e, plano, i, VAGA_VAZIA);
            if x != mx || y != my {
                por_vida(e, t, plano, x, y, 0);
            }
            if e.amarela.saude < t.regras.vida_canibal {
                e.amarela.comeu_tipo = 3;
            } else {
                e.pedir_som(t.sons.ovo);
                e.amarela.ovo_carregado = ovo as i16;
                e.amarela.tipo = 8;
            }
            ok = true;
        }
    }
    if !ok {
        ok = pegar_pedrinha(e, t, plano, x, y) || pegar_comida(e, t, plano, x, y);
    }
    if ok {
        let k = get_dir(mx, my, x, y);
        if k > 0 {
            let a = e.amarela.clone();
            mover_marca(e, t, a.plano as i32, mx, my, a.tipo as i32, k - 1);
        }
        if e.amarela.comeu_tipo >= 0 {
            let tipo = e.amarela.comeu_tipo;
            comer(e, t, tipo as i32);
            e.amarela.comeu_tipo = -1;
        }
        return 1;
    }
    // `_CommandKey` (8:85e2) não está no estado do port: tratado como 0 [lacuna].
    let rainha_sem_colonia = e.amarela.tipo as i32 == TIPO_RAINHA_ALADA && e.amarela.ninho_iniciado == 0;
    if !rainha_sem_colonia || e.amarela.plano != 1 || x != mx || y != my {
        return 0;
    }
    if !cavar_novo_buraco(e, t, x, y) {
        return -1;
    }
    entrar_no_ninho(e, t);
    centralizar(e);
    1
}

/// `_EatMyFood(tipo)` (§7.2).
fn comer(e: &mut Estado, t: &TabelasAmarela, tipo: i32) {
    let maxima = t.regras.vida_maxima;
    // _EatMyFood: os tipos 1, 2 e 3 põem a mensagem na faixa antes do resto (sem sorteio).
    let m = &t.mensagens;
    let faixa = match tipo {
        1 => Some(m.faixa_comida_recebida),
        2 => Some(m.faixa_doacao),
        3 => Some(m.faixa_canibalismo),
        _ => None,
    };
    if let Some(f) = faixa {
        e.eventos.push(Evento::MensagemFaixa(f));
    }
    // Tipo 0 (comida): 0x2C; tipo 3 (ovo, canibalismo): 0x2C e o arroto 10.
    if tipo == 0 || tipo == 3 {
        e.pedir_som(t.sons.comer);
    }
    if tipo == 3 {
        e.pedir_som(t.sons.arroto);
    }
    if tipo == 2 {
        let a = &mut e.amarela;
        if a.saude < t.regras.custo_doar + 1 {
            return;
        }
        a.saude -= t.regras.custo_doar;
        if e.partida.nunca_com_fome != 0 {
            e.amarela.saude = maxima;
        }
        if e.amarela.saude > 0 {
            e.amarela.saude_contagem = 0;
        }
        e.amarela.saude = e.amarela.saude.clamp(0, maxima);
        recalcular_fome(e, t);
        return;
    }
    if tipo == 1 {
        // `_YellowDialog` da trofalaxia (quadro com imagem): ainda não no port [lacuna].
    }
    let vida = e.amarela.saude as i32;
    let pop = e.preta.populacao as i32;
    if vida > 0 && tipo != 1 && pop - 1 > 0 {
        e.preta.saude = (e.preta.saude as i32 + vida / (pop - 1)).min(100) as i16;
    }
    e.amarela.saude = maxima;
    e.amarela.saude_contagem = 0;
    recalcular_fome(e, t);
}

// ---------------------------------------------------------------------------------------------
// Ninho: entrar e sair (§6)
// ---------------------------------------------------------------------------------------------

/// `_EnterNest` (§6.1), também embutido em `_DoAntMoveY`.
fn entrar_no_ninho(e: &mut Estado, t: &TabelasAmarela) {
    e.eventos.push(Evento::TemaDoNinho);
    if e.amarela.soltando_alarme != 0 {
        // `_SetAlarmDropState(0, 1)`: desliga o alarme e a camada de cheiro da janela de edição.
        e.amarela.soltando_alarme = 0;
        e.camada_edicao = -1;
    }
    desmarcar(e, t);
    let a = &mut e.amarela;
    let linha = a.y;
    // Peculiaridade A1: x < 65 (as outras formigas usam x < 64).
    a.plano = if a.x < 0x41 { 2 } else { 3 };
    a.x = linha;
    a.y = if a.tipo as i32 == TIPO_RAINHA { 2 } else { 1 };
    a.dir = 4;
    let (p, y) = (a.plano as i32, a.y as i32);
    cavar_minha_celula(e, t, p, linha as i32, y);
    remarcar(e, t, MARCA_AMARELA);
}

/// `_ExitNest` (§6.2).
fn sair_do_ninho(e: &mut Estado, t: &TabelasAmarela) {
    e.eventos.push(Evento::TemaDoNinho);
    remarcar(e, t, 0);
    let plano = e.amarela.plano as i32;
    let coluna = e.amarela.x as i32;
    let linha = coluna & 0x3F;
    let cor = cor_do_ninho(plano);
    if e.colonia(cor).buracos[linha as usize] == 0 {
        novo_buraco(e, t, cor, coluna);
    }
    let x0 = e.colonia(cor).buracos[linha as usize] as i32;
    e.amarela.x = x0 as i16;
    e.amarela.y = linha as i16;
    let passo = if e.amarela.tipo as i32 == TIPO_RAINHA { 2 } else { 1 };
    let a = e.amarela.clone();
    let base = if a.alvo_plano == 1 {
        let k = get_dir(x0, linha, a.alvo_x as i32, a.alvo_y as i32);
        if k > 0 {
            k - 1
        } else {
            k
        }
    } else if a.alvo_plano as i32 == plano {
        if x0 < 64 {
            2
        } else {
            6
        }
    } else {
        let d = direcao_da_ordem(e, t, 1, x0, linha, a.alvo_plano as i32, a.alvo_x as i32, a.alvo_y as i32);
        if d < 0 {
            e.s.mascara(8)
        } else {
            d
        }
    };
    let mut saida = None;
    for &desvio in &t.desvios_busca {
        let d = (base + desvio as i32) & 7;
        let (dx, dy) = t.d8(d);
        let (nx, ny) = (x0 + passo * dx, linha + passo * dy);
        if nao_e_obstaculo(e, 1, nx, ny) {
            saida = Some((nx, ny, d));
            break;
        }
    }
    let (nx, ny, d) = saida.unwrap_or(((x0 + passo) & 0x7F, linha, base));
    let a = &mut e.amarela;
    a.x = nx as i16;
    a.y = ny as i16;
    a.dir = d as i16;
    a.plano = 1;
    remarcar(e, t, MARCA_AMARELA);
}

// ---------------------------------------------------------------------------------------------
// Tick: `_DoAntSimY` e `_DoAntMoveY`
// ---------------------------------------------------------------------------------------------

/// `_DoAntSimY` (§3).
pub fn simular_amarela(e: &mut Estado, t: &TabelasAmarela) {
    if e.amarela.modo != 0 {
        return;
    }
    remarcar(e, t, MARCA_AMARELA);
    let ciclo = e.partida.ciclo as i32;
    if ciclo & 63 == 0 {
        let v = e.amarela.saude as i32 - 1;
        definir_vida(e, t, v);
    }
    let a = e.amarela.clone();
    let (plano, x, y) = (a.plano as i32, a.x as i32, a.y as i32);
    let mut chao = 0;
    if plano >= 2 {
        chao = tile(e, plano, x, y);
        if chao > 0x4D {
            let v = e.amarela.saude as i32 - 1;
            definir_vida(e, t, v);
        }
    }
    if a.tipo as i32 == TIPO_RAINHA && plano >= 2 && ciclo & 15 == 0 {
        let (dx, dy) = t.d8(a.dir as i32 ^ 4);
        let (px, py) = (x + 2 * dx, y + 2 * dy);
        if (0..64).contains(&px) && (1..64).contains(&py) && e.s.mascara(128) <= e.amarela.saude as i32 {
            let cor = if a.cor == 0 { Cor::Preta } else { Cor::Vermelha };
            por_ovo(e, t, cor, px, py, 1);
            gastar_reserva(e, cor);
            let v = e.amarela.saude as i32 - t.regras.custo_ovo as i32;
            definir_vida(e, t, v);
        }
    }
    if e.amarela.saude < 1 {
        e.amarela.saude_contagem += 1;
        if e.amarela.saude_contagem > t.regras.tempo_sem_vida {
            let causa = if plano < 2 || chao < 0x4E { CausaMorte::Fome } else { CausaMorte::Afogada };
            matar_amarela(e, t, causa);
        }
    }
}

/// Dados da formiga-alvo de uma ordem 3 ou 4.
#[derive(Clone, Copy)]
struct Alvo {
    x: i32,
    y: i32,
    f: Formiga,
}

/// `_DoAntMoveY` (§4).
pub fn mover_amarela(e: &mut Estado, t: &TabelasAmarela) {
    if e.amarela.mover == 0 {
        return;
    }
    let (r, alvo) = passo_da_ordem(e, t);
    let a = e.amarela.clone();
    if e.partida.opcoes[0] != 0 {
        centralizar(e);
    }
    let mut r = r;
    if r == 0 && a.alvo_plano == a.plano && a.alvo_x == a.x && a.alvo_y == a.y && a.comando == 0 {
        r = -1;
    }
    if r == 0 {
        return;
    }
    if r == -2 {
        e.pedir_som(t.sons.erro);
        centralizar(e);
    } else if a.comando == 3 {
        e.amarela.comecou_luta = 1;
        lutar_com_amarela(e, t, a.alvo_vida_plano as i32, a.alvo_indice as i32);
        e.amarela.comecou_luta = 0;
    } else if a.comando == 4 {
        if let Some(al) = alvo {
            let k = get_dir(al.x, al.y, a.x as i32, a.y as i32);
            if k > 0 && al.f.tipo & 0x70 != 0x60 {
                let tipo = (al.f.tipo & 0xF8) | (k - 1) as u8;
                let plano = e.amarela.plano as i32;
                gravar_formiga(e, plano, a.alvo_indice as i32, Formiga { tipo, ..al.f });
                por_vida(e, t, plano, al.x, al.y, tipo);
            }
        }
        comer(e, t, 1);
    }
    let a = e.amarela.clone();
    encerrar_ordem(e, a.plano as i32, a.x as i32, a.y as i32);
    if casa(e) && a.plano == 1 && matches!(tile(e, 1, a.x as i32, a.y as i32), 0x76 | 0x78) {
        // Peculiaridade A8: o choque só é testado quando uma ordem termina.
        matar_amarela(e, t, CausaMorte::Choque);
    }
}

/// O corpo de `_DoAntMoveY` até o rótulo FIM: devolve r (0 continua, −1 concluída, −2 fracassou) e a
/// formiga-alvo lida (ordens 3 e 4).
fn passo_da_ordem(e: &mut Estado, t: &TabelasAmarela) -> (i32, Option<Alvo>) {
    let mut alvo = None;
    if e.amarela.comando >= 3 {
        let a = e.amarela.clone();
        let Some(f) = formiga_em(e, a.alvo_vida_plano as i32, a.alvo_indice as i32) else { return (-2, None) };
        if (f.tipo ^ a.alvo_vida as u8) & 0xF0 != 0 {
            return (-2, None);
        }
        let (tx, ty) = (f.x as i32, f.y as i32);
        alvo = Some(Alvo { x: tx, y: ty, f });
        let vizinho = (a.x as i32 - tx).abs() <= 1 && (a.y as i32 - ty).abs() <= 1;
        if a.alvo_vida_plano != a.plano || !vizinho {
            let m = &mut e.amarela;
            m.alvo_vida_x = tx as i16;
            m.alvo_x = tx as i16;
            m.alvo_vida_y = ty as i16;
            m.alvo_y = ty as i16;
        } else {
            let k = get_dir(a.x as i32, a.y as i32, tx, ty);
            if k > 0 {
                e.amarela.dir = (k - 1) as i16;
            }
            return (-1, alvo);
        }
    }

    let a = e.amarela.clone();
    let (plano, mx, my) = (a.plano as i32, a.x as i32, a.y as i32);
    let (gp, gx, gy) = (a.alvo_plano as i32, a.alvo_x as i32, a.alvo_y as i32);
    if a.comando == 1 && gp == plano {
        let (dx, dy) = t.d9(get_dir(mx, my, gx, gy));
        let mut px = mx + dx;
        let mut py = my + dy;
        if !(0..128).contains(&px) {
            px = mx;
        }
        if !(0..64).contains(&py) {
            py = my;
        }
        if px == gx && py == gy {
            let q = tentar_largar_ou_pegar(e, t, plano, px, py);
            if q != 0 {
                return (if q == 1 { -1 } else { -2 }, alvo);
            }
        }
    }

    let d = direcao_da_ordem(e, t, plano, mx, my, gp, gx, gy);
    if d < 0 {
        return (d, alvo);
    }
    e.amarela.ultimo_x = mx as i16;
    e.amarela.ultimo_y = my as i16;
    let (dx, dy) = t.d8(d);
    let (nx, ny) = (mx + dx, my + dy);
    mover_marca(e, t, plano, nx, ny, a.tipo as i32, d);
    e.amarela.passos = e.amarela.passos.wrapping_add(1);
    let a = e.amarela.clone();
    let (plano, mx, my) = (a.plano as i32, a.x as i32, a.y as i32);
    if plano == 1 {
        if a.rastro > 0 && (a.tipo == 0x18 || a.tipo == 0x38) {
            let cor = if a.cor == 0 { Cor::Preta } else { Cor::Vermelha };
            depositar_trilha(e, cor, mx, my, a.rastro as u8);
            if a.rastro > t.regras.rastro_minimo {
                e.amarela.rastro -= 1;
            }
        }
        if a.soltando_alarme != 0 {
            somar_alarme(e, t, mx, my);
        }
    } else {
        comum::corrigir_saida(e, t, cor_do_ninho(plano), mx, my);
    }
    if e.amarela.passos & 1 != 0 {
        if a.tipo as i32 == TIPO_RAINHA_ALADA
            && a.ninho_iniciado == 0
            && e.preta.saude > 0
            && e.partida.sempre_saudavel == 0
        {
            e.preta.saude -= 1;
        }
        let v = e.amarela.saude as i32 - 1;
        definir_vida(e, t, v);
    }
    if plano == 1 {
        if !e_buraco(e, mx, my) || (gp < 2 && (gp != plano || gx != mx || gy != my)) {
            return (0, alvo);
        }
        entrar_no_ninho(e, t);
    } else {
        if my != 0 {
            let passagem = tile(e, plano, gx, gy) == PASSAGEM_ENTRE_NINHOS;
            if !passagem || a.tipo as i32 == TIPO_RAINHA || gx != mx || gy != my {
                return (0, alvo);
            }
            desmarcar(e, t);
            e.amarela.plano = if plano == 2 { 3 } else { 2 };
            remarcar(e, t, MARCA_AMARELA);
            e.pedir_som(t.sons.passagem);
            centralizar(e);
            // Entrando no vermelho: "You have entered the red nest" na faixa.
            if e.amarela.plano == 3 {
                e.eventos.push(Evento::MensagemFaixa(t.mensagens.faixa_ninho_vermelho));
            }
            return (-1, alvo);
        }
        sair_do_ninho(e, t);
    }
    let novo_plano = e.amarela.plano as i32;
    if gp == plano && gx == mx && gy == my && novo_plano != e.vistas.plano as i32 {
        // O original cai no rótulo de "ordem concluída" (r = −1) depois do `_GotoMyAnt`: a ordem termina
        // e `_ResetYellowVars` põe o destino na posição nova (vetor amarela_superficie, tick 1852).
        if e.partida.opcoes[0] == 0 {
            centralizar(e);
        }
        return (-1, alvo);
    }
    (0, alvo)
}

// ---------------------------------------------------------------------------------------------
// Animações que consomem sorteios (§10.2, §9.1)
// ---------------------------------------------------------------------------------------------

fn quadros_morte(e: &Estado, t: &TabelasAmarela) -> i32 {
    let an = &t.animacao;
    if !an.janela_edicao_na_frente {
        an.quadros_morte_atras
    } else if e.partida.opcoes[1] != 0 {
        an.quadros_morte_frente_musica
    } else {
        an.quadros_morte_frente
    }
}

/// Fotografa a janela de edição do plano `plano` como `_DoEditUpdateDraw` a desenharia agora (mapa,
/// vida e os campos `_Me*` da marca 0xFF). Só lê o estado.
fn quadro_da_tela(e: &Estado, plano: i32) -> Option<QuadroAnimacao> {
    let (mapa, vida) = match plano {
        0 | 1 => (&e.superficie.mapa, &e.superficie.vida),
        2 => (&e.ninho_b.mapa, &e.ninho_b.vida),
        3 => (&e.ninho_r.mapa, &e.ninho_r.vida),
        _ => return None,
    };
    let a = &e.amarela;
    Some(QuadroAnimacao {
        mapa: mapa.dados.clone(),
        vida: vida.dados.clone(),
        x: a.x,
        y: a.y,
        tipo: a.tipo,
        dir: a.dir,
        ovo: a.ovo,
        base: a.base,
    })
}

/// Junta os quadros desenhados de uma animação num `Evento::Animacao` (nada se não houve quadro).
fn pedir_animacao(e: &mut Estado, codigo: i32, plano: i32, espera_ms: u16, quadros: Vec<QuadroAnimacao>) {
    if quadros.is_empty() {
        return;
    }
    let plano = if plano == 0 { 1 } else { plano } as i16;
    e.eventos.push(Evento::Animacao(Box::new(AnimacaoDaAmarela { codigo: codigo as i16, plano, espera_ms, quadros })));
}

/// `_AnimYellowFight(plano, x, y, dir, tipo, código)`. Cada quadro espera o relógio
/// (`Evento::Espera`, menos no primeiro), toca o som do quadro, sorteia, move a marca e é desenhado
/// (`Evento::Animacao` com os quadros, pedido antes das esperas e dos sons para a interface parar a
/// simulação a partir do primeiro quadro). Com música ligada e código 1–9, sai no primeiro quadro,
/// antes do sorteio: no caminho MCI `_mySongIsDone` é sempre verdadeiro (regras/som.md); com
/// `animacao.morte_para_com_musica` = falso, o port faz todos os quadros.
#[allow(clippy::too_many_arguments)]
fn animar_luta(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, dir: i32, tipo: i32, codigo: i32) {
    remarcar(e, t, MARCA_AMARELA);
    let tipo_salvo = e.amarela.tipo as i32;
    let mut quadros = if codigo == 0 { t.animacao.quadros_luta } else { quadros_morte(e, t) };
    let mut tile_original = 0u8;
    if codigo == 10 {
        tile_original = tile(e, 1, x, y).max(0) as u8;
        quadros = t.animacao.quadros_choque;
    }
    let para_com_musica = (1..10).contains(&codigo) && e.partida.opcoes[1] != 0 && t.animacao.morte_para_com_musica;
    // Os pedidos de som e espera ficam numa fila à parte e vão depois do pedido da animação.
    let fila_antes = std::mem::take(&mut e.eventos);
    let mut fotos = Vec::new();
    let mut leao = 3;
    for q in 0..quadros {
        if q > 0 {
            // Cada quadro começa esperando o relógio (o som do quadro sai depois da espera); o
            // primeiro espera só o próximo degrau (até 55 ms), que o port não conta.
            e.eventos.push(Evento::Espera(t.animacao.espera_quadro_ms));
        }
        if codigo == 0 && e.s.mascara(2) != 0 {
            e.pedir_som(t.sons.luta);
        }
        if codigo == 10 {
            e.pedir_som(t.sons.choque);
            let pisca = e.s.mascara(2) != 0;
            por_tile(e, 1, x, y, if pisca { tile_original.wrapping_sub(3) } else { tile_original });
        }
        if para_com_musica {
            break;
        }
        if codigo == 2 {
            por_tile(e, plano, x, y, (leao + 0x38) as u8);
            if leao < 5 {
                leao += 1;
            } else if leao > 3 {
                leao -= 1;
            }
        }
        let d = e.s.mascara(8);
        mover_marca(e, t, plano, x, y, tipo, d);
        fotos.extend(quadro_da_tela(e, plano));
    }
    let pedidos = std::mem::replace(&mut e.eventos, fila_antes);
    pedir_animacao(e, codigo, plano, t.animacao.espera_quadro_ms, fotos);
    e.eventos.extend(pedidos);
    if codigo == 10 {
        por_tile(e, 1, x, y, tile_original);
    }
    let d = e.amarela.dir as i32;
    limpar_minha_vida(e, t, plano, x, y, tipo, d);
    let a = &mut e.amarela;
    a.plano = plano as i16;
    a.x = x as i16;
    a.y = y as i16;
    a.tipo = tipo_salvo as i16;
    a.dir = dir as i16;
}

/// `_GetBestDir` ([`comum::melhor_direcao`]; usada só pela animação de loucura aqui).
fn melhor_direcao_simples(e: &Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, ax: i32, ay: i32) -> i32 {
    comum::melhor_direcao(e, t, plano, x, y, ax, ay)
}

/// `_AnimYellowInsane(plano, x, y, dir, tipo)`: a amarela corre em volta (gerador R para o alvo, S8 quando
/// trava) e volta à posição.
fn animar_loucura(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, dir: i32, tipo: i32) {
    remarcar(e, t, MARCA_AMARELA);
    let tipo_salvo = e.amarela.tipo as i32;
    let quadros = if t.animacao.janela_edicao_na_frente {
        t.animacao.quadros_loucura_frente
    } else {
        t.animacao.quadros_loucura_atras
    };
    let (mut cx, mut cy) = (x, y);
    let fila_antes = std::mem::take(&mut e.eventos);
    let mut fotos = Vec::new();
    for k in 0..quadros {
        if k > 0 {
            e.eventos.push(Evento::Espera(t.animacao.espera_loucura_ms));
        }
        let (mut ax, mut ay) = (x, y);
        if get_dis(cx, cy, x, y) == 0 && quadros - k != 1 {
            ay = e.r.rrand(7) + cy - 3;
            ax = e.r.rrand(7) + cx - 3;
        }
        let mut d = melhor_direcao_simples(e, t, plano, cx, cy, ax, ay);
        if d < 0 {
            d = e.s.mascara(8);
        } else {
            let (dx, dy) = t.d8(d);
            cx += dx;
            cy += dy;
        }
        mover_marca(e, t, plano, cx, cy, tipo, d);
        fotos.extend(quadro_da_tela(e, plano));
    }
    let pedidos = std::mem::replace(&mut e.eventos, fila_antes);
    pedir_animacao(e, -1, plano, t.animacao.espera_loucura_ms, fotos);
    e.eventos.extend(pedidos);
    let d = e.amarela.dir as i32;
    limpar_minha_vida(e, t, plano, cx, cy, tipo, d);
    let a = &mut e.amarela;
    a.plano = plano as i16;
    a.x = x as i16;
    a.y = y as i16;
    a.dir = dir as i16;
    a.tipo = tipo_salvo as i16;
}

// ---------------------------------------------------------------------------------------------
// Luta, trofalaxia, morte e nascimento (§7.2, §9, §10)
// ---------------------------------------------------------------------------------------------

/// `_YellowFight(plano, i)` (§10.1): a formiga `i` da lista do plano (< 2 = A, 2 = B, 3 = R) luta com a
/// amarela. Chamada pelas outras regras quando uma inimiga tenta entrar na célula da amarela.
pub fn lutar_com_amarela(e: &mut Estado, t: &TabelasAmarela, plano: i32, i: i32) {
    let Some(f) = formiga_em(e, plano, i).or_else(|| {
        // O original lê a vaga mesmo além de `_ListIndex`.
        let l = e.lista(area(plano));
        (i >= 0 && (i as usize) < l.capacidade()).then(|| l.vagas[i as usize])
    }) else {
        return;
    };
    let (fx, fy) = (f.x as i32, f.y as i32);
    limpar_vida(e, t, plano, fx, fy, f.tipo);
    let me_tipo = e.amarela.tipo as i32;
    let v = vencedor(e, t, me_tipo, f.tipo as i32);
    if v != me_tipo {
        centralizar(e);
    }
    let a = e.amarela.clone();
    animar_luta(e, t, a.plano as i32, a.x as i32, a.y as i32, a.dir as i32, 0x70, 0);
    if v == me_tipo {
        remarcar(e, t, MARCA_AMARELA);
        if plano < 2 {
            cadaver(e, fx, fy, (f.tipo & 0x80) as i32);
        }
        e.lista_mut(area(plano)).vagas[i as usize].tipo = 0;
        let a = e.amarela.clone();
        if a.comando == 3 && a.alvo_vida_plano == a.plano && a.alvo_indice as i32 == i {
            encerrar_ordem(e, a.plano as i32, a.x as i32, a.y as i32);
        }
    } else {
        por_vida(e, t, plano, fx, fy, f.tipo);
        if plano < 2 {
            let a = e.amarela.clone();
            cadaver(e, a.x as i32, a.y as i32, a.cor as i32);
        }
        matar_amarela(e, t, CausaMorte::Luta);
    }
}

/// `_DoTroph(x, y, d)` (§7.2): a formiga em (x, y), andando na direção `d`, alimenta a amarela.
pub fn trofalaxia(e: &mut Estado, t: &TabelasAmarela, x: i32, y: i32, d: i32) {
    let (dx, dy) = t.d8(d);
    let (ax, ay) = (x + dx, y + dy);
    let k = get_dir(ax, ay, x, y);
    let a = e.amarela.clone();
    mover_marca(e, t, a.plano as i32, ax, ay, a.tipo as i32, k - 1);
    comer(e, t, 1);
}

/// Causa de morte de `_YellowDeath` (§9.1); o número é o código do original.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CausaMorte {
    Luta = 0,
    Aranha = 1,
    FormigaLeao = 2,
    Cercada = 3,
    Fugiu = 4,
    Pisada = 5,
    Cortador = 6,
    Afogada = 7,
    Fome = 8,
    Inseticida = 9,
    Choque = 10,
}

/// `_YellowDeath(causa)` (§9): morte, contadores, mensagens e reencarnação (§9.2).
pub fn matar_amarela(e: &mut Estado, t: &TabelasAmarela, causa: CausaMorte) {
    let codigo = causa as i32;
    // Ferramentas 10/11: encerradas. A pausa salva (`_SaveGamePaused`) é da interface [ver ControleJogador].
    if e.partida.ferramenta == 10 || e.partida.ferramenta == 11 {
        e.partida.ferramenta = -1;
    }
    if e.amarela.modo == 0 && e.amarela.tipo & 8 != 0 {
        let a = e.amarela.clone();
        if !largar_objeto(e, t, a.plano as i32, a.x as i32, a.y as i32) {
            let g = ((a.tipo & 0x78) >> 3) as usize;
            e.amarela.tipo = ((t.descarregar_grupo[g] as i16) << 3) | (a.tipo & 0x80);
        }
    }
    centralizar(e);
    let a = e.amarela.clone();
    if (7..10).contains(&codigo) {
        animar_loucura(e, t, a.plano as i32, a.x as i32, a.y as i32, a.dir as i32, a.tipo as i32);
    }
    let eventos = e.partida.opcoes[3] != 0;
    let m = &t.mensagens;
    let musica = |e: &mut Estado| {
        let k = e.s.rand1(t.musicas.morte_variacoes) as u16;
        e.eventos.push(Evento::Musica(t.musicas.morte_base + k));
    };
    let mensagem = |e: &mut Estado, id: u16| {
        if eventos {
            e.eventos.push(Evento::Mensagem(id));
        }
    };
    let mut morta_em_luta = true;
    match causa {
        CausaMorte::Luta => {
            musica(e);
            let id = if e.amarela.comecou_luta != 0 { m.luta_atacou } else { m.luta_atacada };
            mensagem(e, id);
            morta_em_luta = false;
        }
        CausaMorte::Aranha | CausaMorte::FormigaLeao => {
            // Quadro da aranha/formiga-leão: interface.
            e.preta.comidas_por_predadores = e.preta.comidas_por_predadores.wrapping_add(1);
            musica(e);
            let a = e.amarela.clone();
            animar_luta(e, t, a.plano as i32, a.x as i32, a.y as i32, a.dir as i32, a.tipo as i32, codigo);
            morta_em_luta = false;
        }
        CausaMorte::Cercada => {
            e.pedir_som(t.sons.erro);
            mensagem(e, m.cercada)
        }
        CausaMorte::Fugiu => {
            e.pedir_som(t.sons.erro);
            mensagem(e, m.fugiu)
        }
        CausaMorte::Pisada | CausaMorte::Cortador | CausaMorte::Afogada | CausaMorte::Inseticida => {
            musica(e);
            let id = match causa {
                CausaMorte::Pisada => m.pisada,
                CausaMorte::Cortador => m.cortador,
                CausaMorte::Afogada => m.afogada,
                _ => m.inseticida,
            };
            mensagem(e, id);
        }
        CausaMorte::Fome => {
            musica(e);
            mensagem(e, m.fome);
            e.preta.mortes_naturais = e.preta.mortes_naturais.wrapping_add(1);
            morta_em_luta = false;
        }
        CausaMorte::Choque => {
            e.preta.mortas_em_luta = e.preta.mortas_em_luta.wrapping_add(1);
            let a = e.amarela.clone();
            animar_luta(e, t, a.plano as i32, a.x as i32, a.y as i32, a.dir as i32, a.tipo as i32, 10);
            mensagem(e, m.choque);
            morta_em_luta = false;
        }
    }
    if morta_em_luta {
        e.preta.mortas_em_luta = e.preta.mortas_em_luta.wrapping_add(1);
    }
    if e.amarela.modo == 0 {
        desmarcar(e, t);
    }
    liberar(e, true);
    if e.amarela.soltando_alarme != 0 {
        e.amarela.soltando_alarme = 0;
        e.camada_edicao = -1;
    }
    reencarnar(e, t);
}

/// Vaga achada na reencarnação: (plano, vaga, T).
fn procurar_corpo(e: &Estado, lista: Lista, ovo: bool) -> Option<(usize, Formiga)> {
    let l = e.lista(lista);
    // Peculiaridade A3: começa em `_ListIndex` (uma vaga além do fim), inclusive.
    let inicio = (l.indice.max(0) as usize).min(l.capacidade().saturating_sub(1));
    (0..=inicio).rev().map(|i| (i, l.vagas[i])).find(|(_, f)| {
        if ovo {
            (1..8).contains(&f.tipo)
        } else {
            (8..0x68).contains(&f.tipo)
        }
    })
}

/// A busca de um novo corpo (§9.2).
fn reencarnar(e: &mut Estado, t: &TabelasAmarela) {
    let mut achado = None;
    for (lista, plano, ovo) in [(Lista::B, 2, true), (Lista::B, 2, false), (Lista::A, 1, false), (Lista::R, 3, false)] {
        if let Some((i, f)) = procurar_corpo(e, lista, ovo) {
            achado = Some((lista, plano, ovo, i, f));
            break;
        }
    }
    let Some((lista, plano, ovo, i, f)) = achado else {
        let tipo = e.partida.tipo_jogo;
        if tipo < 2 {
            e.partida.fim_de_jogo = 1;
            e.partida.pretas_venceram = 0;
        } else if tipo == 2 {
            if e.preta.total_colonias > 1 {
                // `_SpecialXfer`: o jogador escolhe outra colônia no mapa do quintal [lacuna: interface].
                e.eventos.push(Evento::Mensagem(t.mensagens.outra_colonia));
            } else {
                e.partida.fim_de_jogo = 1;
                e.partida.pretas_venceram = 0;
                e.eventos.push(Evento::Mensagem(t.mensagens.sem_corpo));
            }
        }
        return;
    };
    let (rx, ry) = (f.x as i32, f.y as i32);
    e.amarela.renascer_x = rx as i16;
    e.amarela.renascer_y = ry as i16;
    e.lista_mut(lista).vagas[i].tipo = 0;
    if let Some(g) = grade_vida_mut(e, plano) {
        g.definir(rx, ry, 0);
    }
    e.amarela.plano = plano as i16;
    if ovo {
        if e.amarela.tipo as i32 == TIPO_RAINHA {
            e.amarela.tipo = TIPO_OPERARIA as i16;
        }
        let tipo = e.amarela.tipo as i32;
        nascer_de_ovo(e, t, plano, rx, ry, tipo);
    } else {
        e.amarela.modo = 0;
        let tipo = (f.tipo & 0xF8) as i32;
        let dir = e.amarela.dir as i32;
        // Peculiaridade A4: a marca no mapa de vida é o T do adulto, não 0xFF.
        marcar_amarela(e, t, plano, rx, ry, tipo, dir, (tipo + dir) as u8);
        encerrar_ordem(e, plano, rx, ry);
        definir_vida(e, t, 100);
        centralizar(e);
        e.eventos.push(Evento::Mensagem(t.mensagens.transferida));
    }
}

/// Partes de `_YellowBirth` comuns aos dois casos, depois do ovo aparecer.
fn depois_do_nascimento(e: &mut Estado, t: &TabelasAmarela) {
    let a = e.amarela.clone();
    encerrar_ordem(e, a.plano as i32, a.x as i32, a.y as i32);
    e.amarela.ninho_iniciado = 1;
    definir_vida(e, t, 100);
}

/// `_YellowBirth(plano, x, y, tipo, 0)` (§9.3): renascer como ovo. A animação é em tempo real (não
/// consome ticks nem sorteios); o port aplica só o estado final.
fn nascer_de_ovo(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32, tipo: i32) {
    let tipo_inicial = e.amarela.tipo as i32;
    e.amarela.modo = 0;
    e.amarela.ovo = 1;
    let dir = e.amarela.dir as i32;
    sem_efeitos(e, |e| marcar_amarela(e, t, plano, x, y, 0, dir, MARCA_AMARELA));
    e.pedir_som(t.sons.nascimento_ovo);
    centralizar(e);
    depois_do_nascimento(e, t);
    // `_YellowDeath` já trocou 0x60 por 0x10, então na prática a direção é sempre 6.
    e.amarela.dir = if tipo_inicial == TIPO_RAINHA { 2 } else { 6 };
    for estagio in 2..=7 {
        e.amarela.ovo = estagio;
        let dir = e.amarela.dir as i32;
        marcar_amarela(e, t, plano, x, y, 0, dir, MARCA_AMARELA);
    }
    let dir = e.amarela.dir as i32;
    marcar_amarela(e, t, plano, x, y, tipo, dir, MARCA_AMARELA);
    e.pedir_som(t.sons.nascimento);
    e.eventos.push(Evento::Musica(t.musicas.renascer));
}

/// `_YellowBirth` desliga os efeitos sonoros (`_OptionStates[2]` = 0) até pôr o ovo: os `Evento::Som`
/// pedidos dentro de `f` são descartados; os demais eventos ficam.
fn sem_efeitos(e: &mut Estado, f: impl FnOnce(&mut Estado)) {
    let n = e.eventos.len();
    f(e);
    let novos = e.eventos.split_off(n);
    e.eventos.extend(novos.into_iter().filter(|ev| !matches!(ev, Evento::Som(_))));
}

/// `_MakeBlkQueen/_MakeRedQueen(x, y, d)` (ciclo-de-vida.md).
pub(crate) fn fazer_rainha(e: &mut Estado, t: &TabelasAmarela, cor: Cor, x: i32, y: i32, d: i32) {
    let (dx, dy) = t.d8(d ^ 4);
    comum::cavar(e, t, cor, x, y);
    comum::cavar(e, t, cor, x + dx, y + dy);
    comum::cavar(e, t, cor, x + 2 * dx, y + 2 * dy);
    let n = e.ninho_mut(cor);
    n.lista.adicionar(Formiga { x: x as u8, y: y as u8, modo: 9, tipo: (d + 0x60) as u8, s: 0 });
    n.lista.adicionar(Formiga { x: (x + dx) as u8, y: (y + dy) as u8, modo: 9, tipo: (d + 0x68) as u8, s: 0 });
    e.colonia_mut(cor).rainhas += 1;
}

/// `_YellowBirth(plano, x, y, 0x10, 1)` (§9.3, comando 9): fundar a colônia.
fn fundar_colonia(e: &mut Estado, t: &TabelasAmarela, ox: i32, oy: i32) {
    e.amarela.modo = 0;
    let plano_ovo = e.amarela.plano as i32;
    sem_efeitos(e, |e| {
        desmarcar(e, t);
        remarcar_como(e, t, TIPO_RAINHA);
        e.eventos.push(Evento::Musica(t.musicas.fundar_inicio));
        por_vida(e, t, plano_ovo, ox, oy, 1);
    });
    e.pedir_som(t.sons.nascimento_ovo);
    depois_do_nascimento(e, t);
    for estagio in 2..=7u8 {
        let p = e.amarela.plano as i32;
        por_vida(e, t, p, ox, oy, estagio);
    }
    desmarcar(e, t);
    let a = e.amarela.clone();
    let cor = if a.plano == 2 { Cor::Preta } else { Cor::Vermelha };
    fazer_rainha(e, t, cor, a.x as i32, a.y as i32, a.dir as i32);
    e.colonia_mut(cor).castas[5] += 1;
    marcar_amarela(e, t, a.plano as i32, ox, oy, TIPO_OPERARIA, a.dir as i32 ^ 4, MARCA_AMARELA);
    let c = if a.cor == 0 { &mut e.preta } else { &mut e.vermelha };
    c.castas[1] += 1;
    c.castas[4] -= 1;
    c.populacao += 1;
    e.pedir_som(t.sons.nascimento);
    e.eventos.push(Evento::Musica(t.musicas.fundar_fim));
}

/// `_SetMyLife(_MePlane, _MeLoc, tipo, _MeDir, 0xFF)`.
fn remarcar_como(e: &mut Estado, t: &TabelasAmarela, tipo: i32) {
    let a = e.amarela.clone();
    marcar_amarela(e, t, a.plano as i32, a.x as i32, a.y as i32, tipo, a.dir as i32, MARCA_AMARELA);
}

// ---------------------------------------------------------------------------------------------
// Troca de corpo (§8)
// ---------------------------------------------------------------------------------------------

/// T de um corpo devolvido à lista: grupo sem terra; carregando ovo, S = ovo carregado.
fn corpo_para_lista(e: &Estado, t: &TabelasAmarela, cor: u8) -> (u8, u8, u8) {
    let a = &e.amarela;
    let mut g = ((a.tipo & 0x78) >> 3) as u8;
    let mut s = 0u8;
    if a.tipo & 8 != 0 {
        if g == 5 || g == 9 {
            g = t.descarregar_grupo[g as usize];
        } else if g == 1 {
            s = a.ovo_carregado as u8;
        }
    }
    let tipo = (g << 3) | cor | (a.dir as u8 & 7);
    let modo = if a.tipo as i32 == TIPO_RAINHA { 9 } else { 0 };
    (tipo, modo, s)
}

/// Devolve o corpo atual à lista do plano como formiga comum (e a cauda, se rainha).
fn devolver_corpo(e: &mut Estado, t: &TabelasAmarela, tipo: u8, modo: u8, s: u8) -> bool {
    let a = e.amarela.clone();
    let (plano, x, y) = (a.plano as i32, a.x as i32, a.y as i32);
    if !acrescentar(e, plano, Formiga { x: x as u8, y: y as u8, modo, tipo, s }) {
        return false;
    }
    por_vida(e, t, plano, x, y, tipo);
    if a.tipo as i32 == TIPO_RAINHA {
        let (cx, cy) = cauda(t, x, y, a.dir as i32);
        if !acrescentar(e, plano, Formiga { x: cx as u8, y: cy as u8, modo, tipo: tipo + 8, s: 0 }) {
            return false;
        }
        por_vida(e, t, plano, cx, cy, tipo + 8);
    }
    true
}

/// `_DoLifeExchange(plano, x, y)` (§8.3).
fn trocar_corpo(e: &mut Estado, t: &TabelasAmarela, plano: i32, x: i32, y: i32) -> bool {
    let v = vida(e, plano, x, y);
    let dist_aranha = |e: &Estado| {
        let dx = (x * 16 + 8) as i64 - e.aranha.x as i64;
        let dy = (y * 16 + 8) as i64 - e.aranha.y as i64;
        dx * dx + dy * dy
    };
    if v < 1 {
        // 1. Célula vazia na superfície, perto da aranha: vira aranha.
        if plano != 1 || dist_aranha(e) >= t.regras.raio_aranha {
            return false;
        }
        if e.amarela.modo == 1 {
            return true;
        }
        let cor = (e.amarela.cor & 0x80) as u8;
        let (tipo, modo, s) = corpo_para_lista(e, t, cor);
        if !devolver_corpo(e, t, tipo, modo, s) {
            return false;
        }
        let a = e.amarela.clone();
        if dentro(a.plano as i32, a.x as i32, a.y as i32) {
            // O corpo devolvido fica na lista sem marca no mapa de vida.
            por_vida(e, t, a.plano as i32, a.x as i32, a.y as i32, 0);
            if a.tipo as i32 == TIPO_RAINHA {
                let (cx, cy) = cauda(t, a.x as i32, a.y as i32, a.dir as i32);
                por_vida(e, t, a.plano as i32, cx, cy, 0);
            }
        }
        let (sx, sy) = ((e.aranha.x >> 4) as i32, (e.aranha.y >> 4) as i32);
        e.aranha.alvo = -2;
        e.aranha.destino_x = sx as i16;
        e.aranha.destino_y = sy as i16;
        e.amarela.modo = 1;
        e.aranha.estado = 0;
        e.amarela.modo_aranha = 0;
        e.amarela.cor = 0;
        encerrar_ordem(e, 1, sx, sy);
        e.amarela.saude = t.regras.vida_maxima;
        e.amarela.saude_contagem = 0;
        e.amarela.quer_comida = if e.amarela.aviso_saude < 100 { 0 } else { 1 };
        return true;
    }
    // 2. Clicou em si mesma.
    let p = if plano == 0 { 1 } else { plano };
    if e.amarela.plano as i32 == p {
        if e.amarela.modo == 1 {
            if plano < 2 && dist_aranha(e) < t.regras.raio_aranha {
                return true;
            }
        } else if v == 0xFF || v == 0xFE {
            return true;
        }
    }
    if v == 0xFF || v == 0xFE {
        return true;
    }
    // 3. Rainha: cabeça e cauda.
    let (mut hx, mut hy, mut cabeca) = (x, y, v);
    let mut cauda_alvo: Option<(i32, i32, i32)> = None;
    let grupo = (v & 0x78) >> 3;
    if grupo == 12 {
        let (dx, dy) = t.d8((v ^ 0xFC) & 7);
        let (cx, cy) = (x + dx, y + dy);
        cauda_alvo = Some((cx, cy, vida(e, plano, cx, cy)));
    } else if grupo == 13 {
        let (dx, dy) = t.d8(v & 7);
        cauda_alvo = Some((x, y, v));
        hx = x + dx;
        hy = y + dy;
        cabeca = vida(e, plano, hx, hy);
    }
    // 4. As vagas.
    let i = achar_formiga(e, plano, hx, hy, cabeca);
    let j = cauda_alvo.map_or(-1, |(cx, cy, cv)| achar_formiga(e, plano, cx, cy, cv));
    if i < 0 {
        return false;
    }
    // 5. Recusas.
    let rejeitar = |c: i32| {
        let g = (c & 0x78) >> 3;
        c & 0x80 != 0 || g == 0 || g > 13 || g == 10 || g == 11
    };
    if rejeitar(cabeca) {
        return false;
    }
    let g = (cabeca & 0x78) >> 3;
    let mut novo_t = cabeca;
    if g == 5 || g == 9 {
        novo_t = ((t.descarregar_grupo[g as usize] as i32) << 3) | (cabeca & 7);
    }
    let ovo_do_alvo = if g == 1 { formiga_em(e, plano, i).map(|f| f.s as i16) } else { None };
    if e.amarela.modo == 0 {
        let (tipo, modo, s) = corpo_para_lista(e, t, (cabeca & 0x80) as u8);
        if let Some(o) = ovo_do_alvo {
            e.amarela.ovo_carregado = o;
        }
        gravar_formiga(e, plano, i, VAGA_VAZIA);
        if cauda_alvo.is_some() {
            gravar_formiga(e, plano, j, VAGA_VAZIA);
        }
        // Peculiaridade A2: com a lista cheia, o alvo já foi removido.
        if !devolver_corpo(e, t, tipo, modo, s) {
            return false;
        }
    } else {
        if let Some(o) = ovo_do_alvo {
            e.amarela.ovo_carregado = o;
        }
        gravar_formiga(e, plano, i, VAGA_VAZIA);
        if cauda_alvo.is_some() {
            gravar_formiga(e, plano, j, VAGA_VAZIA);
        }
    }
    // 9. O novo corpo.
    let a = &mut e.amarela;
    a.cor = (novo_t & 0x80) as i16;
    a.base = 0;
    a.modo = 0;
    a.modo_aranha = 0;
    a.saude = t.regras.vida_maxima;
    a.saude_contagem = 0;
    recalcular_fome(e, t);
    if dentro(plano, hx, hy) {
        por_vida(e, t, plano, hx, hy, MARCA_AMARELA);
        if novo_t & 0x78 == TIPO_RAINHA {
            let (dx, dy) = t.d8((novo_t ^ 0xFC) & 7);
            por_vida(e, t, plano, hx + dx, hy + dy, MARCA_CAUDA);
        }
        let a = &mut e.amarela;
        a.x = hx as i16;
        a.y = hy as i16;
        a.dir = (novo_t & 7) as i16;
        a.tipo = (novo_t & 0x78) as i16;
        a.plano = plano as i16;
    }
    if e.amarela.tipo < 8 {
        e.amarela.ovo = e.amarela.dir;
    }
    true
}

// ---------------------------------------------------------------------------------------------
// Comandos do jogador (§10.3, §11)
// ---------------------------------------------------------------------------------------------

/// Estado da interface usado pelos comandos que não vai para o save.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControleJogador {
    /// `_SaveGamePaused` (`9:9ea2`): pausa anterior à ferramenta de troca de corpo ou de alvo.
    pub pausa_salva: i16,
}

/// O que a interface deve fazer depois de um clique no mapa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RespostaClique {
    /// Nada (ou só som).
    Nada,
    /// Uma ordem (ou alvo) foi dada à amarela ou à aranha.
    Ordem,
    /// Centralizar a vista na amarela.
    Centralizar,
    /// Abrir o menu da amarela (`_AntMenu`, §11.5).
    MenuAmarela,
    /// Abrir a janela Examine (`_MagnifyMenu`) da formiga na célula.
    Examinar,
    /// Modo Experimental: a ferramenta `_processExp` é da interface (§11.6).
    Experimental,
    /// Ferramenta de troca de corpo: `true` se trocou.
    TrocaDeCorpo(bool),
}

/// O que a interface deve fazer depois de uma tecla.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RespostaTecla {
    /// A tecla não é de comando: segue para os outros tratadores.
    Ignorada,
    /// Tratada.
    Tratada,
    /// Tratada; centralizar a vista na amarela.
    Centralizar,
}

/// Teclas modificadoras seguras no momento da tecla.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modificadores {
    pub shift: bool,
    pub ctrl: bool,
    pub tab: bool,
}

/// Códigos de tecla virtual do Windows usados por `_YellowCommandKey`.
pub mod vk {
    pub const BACKSPACE: u16 = 0x08;
    pub const HOME: u16 = 0x24;
    pub const DIGITO_0: u16 = 0x30;
    pub const DIGITO_1: u16 = 0x31;
    pub const DIGITO_2: u16 = 0x32;
    pub const DIGITO_3: u16 = 0x33;
    pub const DIGITO_4: u16 = 0x34;
    pub const J: u16 = 0x4A;
    pub const X: u16 = 0x58;
    pub const NUMPAD_0: u16 = 0x60;
    pub const NUMPAD_1: u16 = 0x61;
    pub const NUMPAD_2: u16 = 0x62;
    pub const NUMPAD_3: u16 = 0x63;
    pub const NUMPAD_4: u16 = 0x64;
    pub const NUMPAD_MULTIPLICAR: u16 = 0x6A;
    pub const F9: u16 = 0x78;
}

/// "ORDEM(cmd)" de `_processEdit` (§11.3) para a amarela formiga ou aranha.
fn dar_ordem(e: &mut Estado, plano_vista: i32, x: i32, y: i32, cmd: i32) -> RespostaClique {
    if plano_vista < 2 {
        if e.amarela.modo == 1 {
            ordem_aranha(e, x, y, cmd);
            return RespostaClique::Ordem;
        }
        let a = &mut e.amarela;
        a.alvo_plano = if plano_vista == 0 { 1 } else { plano_vista as i16 };
        a.alvo_x = x as i16;
        a.alvo_y = y as i16;
        a.comando = cmd as i16;
        a.mover = 1;
        a.contorno_contagem = -2;
        return RespostaClique::Ordem;
    }
    if e.amarela.modo != 0 {
        return RespostaClique::Nada;
    }
    let c = if cmd == 1 {
        let fechada = e.amarela.plano > 1 && y < 1 && e_capim(tile(e, plano_vista, x, y));
        if e_escavavel(e, plano_vista, x, y) || fechada {
            2
        } else if e.amarela.tipo & 8 != 0 || levantavel(e, plano_vista, x, y) {
            1
        } else {
            2
        }
    } else {
        0
    };
    e.amarela.comando = c;
    definir_destino(e, plano_vista, x, y);
    RespostaClique::Ordem
}

/// Destino de uma ordem (a parte comum de `_processEdit` e `_SetGoalsY`): x limitado a 1..62 nas linhas
/// 0–1 de um ninho.
fn definir_destino(e: &mut Estado, plano_vista: i32, x: i32, y: i32) {
    let a = &mut e.amarela;
    a.alvo_plano = if plano_vista == 0 { 1 } else { plano_vista as i16 };
    a.alvo_x = x as i16;
    a.alvo_y = y as i16;
    a.contorno_contagem = -2;
    if plano_vista >= 2 && y < 2 {
        a.alvo_x = x.clamp(1, 62) as i16;
    }
    a.mover = 1;
}

/// `_processSpider(x, y, dupla)`.
/// Devolve `true` se escolheu uma formiga como alvo.
fn ordem_aranha(e: &mut Estado, x: i32, y: i32, dupla: i32) -> bool {
    let modo_alvo = e.partida.ferramenta == 11;
    if dupla >= 1 || modo_alvo {
        let v = vida_bruta(e, 1, x, y).unwrap_or(0) as i32;
        let i = if v != 0 { achar_formiga(e, 1, x, y, v) } else { -1 };
        if i >= 0 {
            e.aranha.alvo_tipo = v as i16;
            e.aranha.alvo = i as i16;
            e.aranha.estado = 2;
            if modo_alvo {
                e.amarela.modo_aranha = 6;
                e.partida.ferramenta = -1;
            }
            return true;
        }
        if modo_alvo {
            return false;
        }
    }
    e.aranha.alvo = -2;
    e.aranha.alvo_tipo = -1;
    e.aranha.estado = 0;
    e.aranha.destino_x = x as i16;
    e.aranha.destino_y = y as i16;
    e.amarela.comando = dupla as i16;
    false
}

/// "Ir para (x, y)": clique simples numa célula do mapa exibido (`plano_vista` = `_MapPlane`), sem os
/// casos de menu (ver [`clicar_no_mapa`]).
pub fn ordem_ir(e: &mut Estado, plano_vista: i32, x: i32, y: i32) -> RespostaClique {
    dar_ordem(e, plano_vista, x, y, 0)
}

/// "Agir em (x, y)": duplo clique (§11.3). Numa formiga que não é ovo nem a amarela, ataca (outra cor)
/// ou pede comida (mesma cor); senão vai até lá e pega/larga (ou cava, num ninho).
pub fn ordem_agir(e: &mut Estado, plano_vista: i32, x: i32, y: i32) -> RespostaClique {
    let v = vida(e, plano_vista, x, y);
    if e.amarela.modo != 0 || v < 0 || (v & 0x7F) < 8 || v == 0xFF || v == 0xFE {
        return dar_ordem(e, plano_vista, x, y, 1);
    }
    let i = achar_formiga(e, plano_vista, x, y, v);
    e.amarela.alvo_indice = i as i16;
    if i < 0 {
        return RespostaClique::Nada;
    }
    let p = if plano_vista == 0 { 1 } else { plano_vista };
    let a = &mut e.amarela;
    a.comando = if (a.cor as i32 ^ v) & 0x80 == 0 { 4 } else { 3 };
    a.alvo_vida = v as i16;
    a.alvo_vida_plano = p as i16;
    a.alvo_vida_x = x as i16;
    a.alvo_vida_y = y as i16;
    definir_destino(e, plano_vista, x, y);
    RespostaClique::Ordem
}

/// `_processEdit` (§11.3) completo: `duplo` = duplo clique (esquerdo ou direito); `segurando` = o botão
/// continua apertado depois da espera e o Shift não está apertado (abre menus).
#[allow(clippy::too_many_arguments)]
pub fn clicar_no_mapa(
    e: &mut Estado,
    t: &TabelasAmarela,
    ctl: &mut ControleJogador,
    plano_vista: i32,
    x: i32,
    y: i32,
    duplo: bool,
    segurando: bool,
) -> RespostaClique {
    if e.partida.tipo_jogo == 3 {
        return RespostaClique::Experimental;
    }
    if e.amarela.morreu != 0 {
        e.amarela.morreu = 0;
    }
    match e.partida.ferramenta {
        -1 => {}
        10 => {
            let p = if plano_vista == 0 { 1 } else { plano_vista };
            return RespostaClique::TrocaDeCorpo(trocar_de_corpo(e, t, ctl, p, x, y));
        }
        11 => {
            if plano_vista > 1 || e.amarela.modo != 1 {
                return RespostaClique::Nada;
            }
            let achou = ordem_aranha(e, x, y, duplo as i32);
            e.pedir_som(if achou { t.sons.ok } else { t.sons.erro });
            return RespostaClique::Ordem;
        }
        _ => return RespostaClique::Nada,
    }
    if duplo {
        return ordem_agir(e, plano_vista, x, y);
    }
    // `_IsItYellow` olha o plano da amarela, não o exibido (como no original).
    let me_plano = e.amarela.plano as i32;
    let marca = vida(e, me_plano, x, y);
    if marca == 0xFF || marca == 0xFE {
        if segurando {
            return RespostaClique::MenuAmarela;
        }
        if marca == 0xFE {
            return RespostaClique::Nada;
        }
        centralizar(e);
        return RespostaClique::Centralizar;
    }
    if segurando && vida(e, plano_vista, x, y) > 0 {
        return RespostaClique::Examinar;
    }
    ordem_ir(e, plano_vista, x, y)
}

fn alternar_ferramenta(e: &mut Estado, ctl: &mut ControleJogador, f: i16) {
    if e.partida.ferramenta == f {
        e.partida.ferramenta = -1;
        e.partida.pausado = ctl.pausa_salva;
    } else {
        ctl.pausa_salva = e.partida.pausado;
        e.partida.ferramenta = f;
        e.partida.pausado = 1;
    }
}

/// `_YellowCommand(c)` (§10.3). Devolve `false` quando o original toca o som de erro.
pub fn comando(e: &mut Estado, t: &TabelasAmarela, ctl: &mut ControleJogador, c: u8) -> bool {
    if e.amarela.modo > 1 || e.partida.fim_de_jogo != 0 {
        return true;
    }
    let mus = &t.musicas;
    match c {
        0 => {} // ajuda (WinHelp): interface
        1..=3 => {
            let (id, n) = match c {
                1 => (mus.recrutar_5, 5),
                2 => (mus.recrutar_10, 10),
                _ => (mus.recrutar_todas, 1000),
            };
            e.eventos.push(Evento::Musica(id));
            recrutar(e, n);
        }
        4 | 5 => {
            e.eventos.push(Evento::Musica(if c == 4 { mus.liberar_metade } else { mus.liberar_todas }));
            liberar(e, c == 5);
        }
        6 => alternar_ferramenta(e, ctl, 10),
        7 => {
            if e.amarela.soltando_alarme == 0 {
                if e.amarela.plano != 1 || e.amarela.modo != 0 {
                    e.pedir_som(t.sons.erro);
                    return false;
                }
                e.amarela.soltando_alarme = 1;
                // A janela de edição passa a mostrar o feromônio de alarme (`_ELayerMode` = 0).
                e.camada_edicao = CAMADA_ALARME;
            } else {
                e.amarela.soltando_alarme = 0;
                e.camada_edicao = -1;
            }
            e.pedir_som(t.sons.ok);
        }
        8 => {
            let a = e.amarela.clone();
            let ok = a.plano == 1 && tentar_largar_ou_pegar(e, t, 1, a.x as i32, a.y as i32) != -1;
            if !ok {
                e.pedir_som(t.sons.erro);
            }
            return ok;
        }
        9 => {
            let a = e.amarela.clone();
            if a.plano != 2 || a.y <= 2 {
                e.pedir_som(t.sons.erro);
                return false;
            }
            let (dx, dy) = t.d8(a.dir as i32 ^ 4);
            fundar_colonia(e, t, a.x as i32 + 2 * dx, a.y as i32 + 2 * dy);
        }
        10 => alternar_ferramenta(e, ctl, 11),
        11 => {
            e.amarela.modo_aranha = if e.amarela.modo_aranha == 7 { 0 } else { 7 };
            e.pedir_som(t.sons.aranha_faro);
        }
        12 => {
            let liga = e.amarela.modo_aranha != 8;
            e.amarela.modo_aranha = if liga { 8 } else { 0 };
            e.pedir_som(if liga { t.sons.aranha_laser } else { t.sons.aranha_faro });
        }
        _ => {}
    }
    true
}

/// `_YellowCommandKey(vk)` (§11.4): `vk` é o código de tecla virtual do Windows (ver [`vk`]).
pub fn tecla_de_comando(
    e: &mut Estado,
    t: &TabelasAmarela,
    ctl: &mut ControleJogador,
    tecla: u16,
    m: Modificadores,
) -> RespostaTecla {
    let formiga = e.amarela.modo == 0;
    let rainha_sem_colonia = e.amarela.tipo as i32 == TIPO_RAINHA_ALADA && e.amarela.ninho_iniciado == 0;
    let c = match tecla {
        vk::X | vk::F9 => 6,
        vk::DIGITO_1 | vk::NUMPAD_1 => {
            if !formiga {
                10
            } else if rainha_sem_colonia {
                8
            } else {
                1
            }
        }
        vk::DIGITO_2 | vk::NUMPAD_2 => {
            if !formiga {
                11
            } else if rainha_sem_colonia {
                9
            } else if m.ctrl {
                3
            } else {
                2
            }
        }
        vk::DIGITO_3 | vk::NUMPAD_3 => 4,
        vk::DIGITO_4 | vk::NUMPAD_4 => 5,
        vk::DIGITO_0 | vk::NUMPAD_0 => 7,
        vk::J | vk::NUMPAD_MULTIPLICAR => {
            if !m.tab {
                return RespostaTecla::Ignorada;
            }
            if e.amarela.modo != 1 {
                return RespostaTecla::Tratada;
            }
            12
        }
        vk::BACKSPACE => return parar_ou_centralizar(e, m.shift),
        vk::HOME => {
            e.pedir_som(t.sons.ok);
            centralizar(e);
            return RespostaTecla::Centralizar;
        }
        _ => return RespostaTecla::Ignorada,
    };
    comando(e, t, ctl, c);
    RespostaTecla::Tratada
}

/// Backspace (§11.4): para a ordem em andamento, ou centraliza.
fn parar_ou_centralizar(e: &mut Estado, shift: bool) -> RespostaTecla {
    if !shift {
        match e.amarela.modo {
            0 => {
                if e.amarela.mover != 0 {
                    let a = &mut e.amarela;
                    a.contorno_contagem = -2;
                    a.mover = 0;
                    a.dis_anterior = 0;
                    a.alvo_x = a.x;
                    a.alvo_y = a.y;
                    a.comando = 0;
                    a.alvo_plano = a.plano;
                    a.passos = 0;
                    a.dis = 0;
                    a.ultimo_x = a.x;
                    a.ultimo_y = a.y;
                    return RespostaTecla::Tratada;
                }
            }
            1 => {
                let a = &e.amarela;
                if e.aranha.destino_x != a.x || e.aranha.destino_y != a.y {
                    e.aranha.destino_x = a.x;
                    e.aranha.destino_y = a.y;
                    e.aranha.estado = 0;
                    return RespostaTecla::Tratada;
                }
            }
            _ => return RespostaTecla::Tratada,
        }
    }
    centralizar(e);
    RespostaTecla::Centralizar
}

/// `_ExchangeLives(plano, x, y)` (§8.2): troca de corpo pela ferramenta 10. Sucesso desliga a ferramenta
/// e restaura a pausa anterior.
pub fn trocar_de_corpo(
    e: &mut Estado,
    t: &TabelasAmarela,
    ctl: &mut ControleJogador,
    plano: i32,
    x: i32,
    y: i32,
) -> bool {
    e.tutorial.temporario = 1;
    if !trocar_corpo(e, t, plano, x, y) {
        e.pedir_som(t.sons.erro);
        return false;
    }
    e.partida.ferramenta = -1;
    e.partida.pausado = ctl.pausa_salva;
    if e.amarela.tipo as i32 == TIPO_RAINHA {
        e.eventos.push(Evento::Musica(t.musicas.virou_rainha));
    } else {
        e.pedir_som(t.sons.ok);
    }
    true
}

impl RegrasAmarela for RegrasOriginais {
    fn simular_amarela(&mut self, e: &mut Estado) {
        simular_amarela(e, &self.amarela);
    }

    fn mover_amarela(&mut self, e: &mut Estado) {
        mover_amarela(e, &self.amarela);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn t() -> &'static TabelasAmarela {
        TabelasAmarela::padrao()
    }

    #[test]
    fn tabelas_embutidas() {
        let t = t();
        assert_eq!(t.d9(0), (0, 0));
        for d in 0..8 {
            assert_eq!(t.d9(d + 1), t.d8(d));
        }
        assert_eq!(t.desvios_busca, [0, 1, -1, 2, -2, 3, -3, 4]);
        assert_eq!(t.descarregar_grupo[5], 2);
        assert_eq!(t.descarregar_grupo[9], 6);
    }

    #[test]
    fn get_dir_e_dis() {
        assert_eq!(get_dir(5, 5, 5, 5), 0);
        assert_eq!(get_dir(5, 5, 5, 0), 1);
        assert_eq!(get_dir(5, 5, 9, 0), 2);
        assert_eq!(get_dir(5, 5, 9, 5), 3);
        assert_eq!(get_dir(5, 5, 9, 9), 4);
        assert_eq!(get_dir(5, 5, 5, 9), 5);
        assert_eq!(get_dir(5, 5, 0, 9), 6);
        assert_eq!(get_dir(5, 5, 0, 5), 7);
        assert_eq!(get_dir(5, 5, 0, 0), 8);
        assert_eq!(get_dis(0, 0, 3, 4), 25);
    }

    #[test]
    fn pode_pisar_linhas_do_ninho() {
        let mut e = Estado::vazio();
        // (10, 1) túnel; (10, 0) abertura; (11, 1) terra; (11, 0) capim.
        e.ninho_b.mapa.definir(10, 1, 3);
        e.ninho_b.mapa.definir(10, 0, 0x18);
        e.ninho_b.mapa.definir(11, 1, 0x2E);
        e.ninho_b.mapa.definir(11, 0, 0x1D);
        // Destino no mesmo ninho, linha 5, coluna 10: linha 1 só na coluna do destino... ou destino fora da linha 0.
        assert!(pode_pisar(&e, 2, 10, 1, 2, 10, 5, false));
        assert!(pode_pisar(&e, 2, 10, 1, 2, 3, 5, false));
        assert!(!pode_pisar(&e, 2, 10, 1, 2, 3, 0, false));
        // Linha 0 sem cavar: só o próprio destino.
        assert!(!pode_pisar(&e, 2, 10, 0, 2, 10, 5, false));
        assert!(pode_pisar(&e, 2, 10, 0, 2, 10, 0, false));
        // Destino em outro plano: basta ser passável.
        assert!(pode_pisar(&e, 2, 10, 0, 1, 50, 50, false));
        // Cavando: terra só na coluna do destino; capim na linha 0 só se a de baixo não for terra.
        assert!(pode_pisar(&e, 2, 11, 1, 2, 11, 9, true));
        assert!(!pode_pisar(&e, 2, 11, 1, 2, 12, 9, true));
        assert!(!pode_pisar(&e, 2, 11, 0, 2, 11, 0, true));
        e.ninho_b.mapa.definir(11, 1, 2);
        assert!(pode_pisar(&e, 2, 11, 0, 2, 11, 0, true));
        assert!(!pode_pisar(&e, 2, 11, 0, 2, 11, 0, false));
    }
}
