//! Formiga-leão, aranha, lagarta e bicho-de-conta.
//! Especificação: docs/spec/regras/inimigos.md (com as correções registradas lá em "Implementação no port").
//!
//! - [`formigas_leao`]: `_DoAntLions` (`1030:43fa`), com `_AddRandAntLion` embutido.
//! - [`aranha`]: `_MoveSpider` (`1020:4704`), com `_SFoundAnt`, `_SpiderScan` (laser), o cerco e a
//!   aranha controlada pelo jogador (`_MeMode = 1`).
//! - [`lagarta`]: `_DoPillar` (`1030:4cdc`), que começa por `_DoSow` (bichos-de-conta, inclusive o
//!   "fantasma" da vaga 0).
//! - [`matar_aranha`] (`_KillSpider`) e [`matar_formiga_leao`] (`_KillAntLion`): para o quintal e o
//!   modo Experimental.
//!
//! Tabelas e constantes: `data/inimigos.json` ([`TabelasInimigos`]), gerado por
//! `tools/extrair_inimigos.py`. O seno do laser é o mesmo de `_AddFood` (objeto 9/1000 do banco), lido
//! de `RegrasOriginais::geracao`.
//!
//! As regras não tocam som nem janela: as mensagens vão para `Estado::eventos`; sons e o desenho do
//! laser são omitidos (não consomem sorteio).

use std::sync::OnceLock;

use serde::Deserialize;

use super::estado::{Estado, Evento};
use super::geracao::TabelasGeracao;
use super::tick::{RegrasInimigos, RegrasOriginais};
use crate::recursos::Recursos;

/// Arquivo de dados com as tabelas e constantes.
pub const ARQUIVO_TABELAS: &str = "inimigos.json";

/// Cópia de `data/inimigos.json` embutida no executável, usada quando ninguém carregou as tabelas
/// pelos [`Recursos`] (testes e `RegrasOriginais::default()`).
const JSON_PADRAO: &str = include_str!("../../../data/inimigos.json");

const LARGURA: i32 = 128;
const ALTURA: i32 = 64;

/// Marca da formiga amarela no `_LifeA` (cabeça e cauda da rainha).
const VIDA_AMARELA: u8 = 0xFF;
const VIDA_AMARELA_2: u8 = 0xFE;

// ---------------------------------------------------------------------------------------------
// Tabelas
// ---------------------------------------------------------------------------------------------

/// Constantes das regras (bloco `regras` de `inimigos.json`; origem de cada uma em `_origem`).
#[derive(Clone, Debug, Deserialize)]
pub struct RegrasInimigosDados {
    pub leao_chance_nova: u16,
    pub leao_chance_cuspir: u16,
    pub leao_tempo_cuspir: u8,
    pub leao_tempo_comer: u8,
    pub leao_cerco: i32,
    pub leao_maximo: i16,
    pub leao_tentativas: i32,
    pub leao_tentativas_3x3: i32,
    pub tile_leao_centro: u8,
    pub tile_leao_borda: u8,
    pub tile_leao_morta: u8,
    pub mensagem_leao_morta: u16,
    pub aranha_chance_aparecer: u16,
    pub aranha_cerco_morte: i32,
    pub aranha_cerco_fuga: i32,
    pub aranha_agonia: i16,
    pub aranha_vinganca_maxima: i16,
    pub aranha_vinganca_faro: i16,
    pub aranha_vinganca_laser: i16,
    pub mensagem_aranha_morta: u16,
    pub aranha_chance_andar: u16,
    pub aranha_chance_girar_parada: u16,
    pub aranha_chance_girar_andando: u16,
    pub aranha_chance_parar: u16,
    pub aranha_corrida: i16,
    pub aranha_desiste: i32,
    pub aranha_mastigar: i16,
    pub aranha_mastigar_rapido: i16,
    pub aranha_arroto: i16,
    pub aranha_faro: i32,
    pub aranha_visao: i32,
    pub aranha_visao_celulas: i32,
    pub aranha_laser_raio: u16,
    pub aranha_agonia_lenta: i16,
    pub aranha_teto_casa: u8,
    pub lagarta_cerco: i32,
    pub lagarta_fases: i16,
    pub lagarta_fase_inicial: i16,
    pub lagarta_comida: u8,
    pub lagarta_limite_comida: u8,
    /// Cabeça por direção (0 sobe, 1 direita, 2 desce, 3 esquerda).
    pub tiles_lagarta_cabeca: [u8; 4],
    /// Corpo `atrás(s + 1)` e `atrás(1)`: [vertical, horizontal].
    pub tiles_lagarta_corpo_1: [u8; 2],
    /// Corpo `atrás(s)`: [vertical, horizontal].
    pub tiles_lagarta_corpo_2: [u8; 2],
    /// [xmin, xmax, ymin, ymax]: fora disto a lagarta some.
    pub lagarta_limites: [i32; 4],
    pub bicho_chao: u8,
}

/// Conteúdo de `data/inimigos.json`.
#[derive(Clone, Debug, Deserialize)]
pub struct TabelasInimigos {
    /// `_Dx8`/`_Dy8`.
    pub dx8: [i8; 8],
    pub dy8: [i8; 8],
    /// `_Dx9`/`_Dy9`: 0 = parado, 1–8 = direções 0–7.
    pub dx9: [i8; 9],
    pub dy9: [i8; 9],
    /// `_TurnTab[atual][alvo]`.
    pub turn_tab: [[u8; 8]; 8],
    /// `_SpX`/`_SpY`: passo da aranha em dezesseis avos de célula.
    pub passo_aranha_x: [i8; 8],
    pub passo_aranha_y: [i8; 8],
    /// `_SeatX`/`_SeatY`: célula da boca.
    pub boca_aranha_x: [i8; 8],
    pub boca_aranha_y: [i8; 8],
    /// `_SowTab`.
    pub sow_tab: [u8; 8],
    /// Borda da armadilha por direção (somada a `tile_leao_borda`).
    pub borda_formiga_leao: [u8; 8],
    pub regras: RegrasInimigosDados,
}

impl TabelasInimigos {
    /// Lê `inimigos.json` (com mods).
    pub fn carregar(r: &Recursos) -> Result<TabelasInimigos, String> {
        r.dados(ARQUIVO_TABELAS)
    }

    pub fn de_json(texto: &str) -> Result<TabelasInimigos, String> {
        serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_TABELAS}: {e}"))
    }

    /// As tabelas de `data/inimigos.json` como estavam na compilação.
    pub fn padrao() -> &'static TabelasInimigos {
        static PADRAO: OnceLock<TabelasInimigos> = OnceLock::new();
        PADRAO.get_or_init(|| {
            TabelasInimigos::de_json(JSON_PADRAO).expect("data/inimigos.json embutido")
        })
    }

    fn d8(&self, d: usize) -> (i32, i32) {
        (self.dx8[d & 7] as i32, self.dy8[d & 7] as i32)
    }

    fn giro(&self, atual: i16, alvo: i32) -> i16 {
        self.turn_tab[(atual & 7) as usize][(alvo & 7) as usize] as i16
    }
}

/// Seno de `_fracSIN` / `_fracCOS` (ângulo 0–255 por volta, em 1/32767). Usa a tabela do banco quando
/// as tabelas da geração foram carregadas; senão, `trunc(sin(i·π/128)·32767)`, que coincide com os 64
/// valores do objeto 9/1000 (verificado pelo teste `seno_calculado_igual_ao_do_banco`, com `SIMANT_DIR`).
struct Seno<'a>(Option<&'a TabelasGeracao>);

impl Seno<'_> {
    fn sen(&self, angulo: i32) -> i32 {
        if let Some(g) = self.0 {
            return g.seno(angulo);
        }
        let mut u = angulo & 0x7F;
        if u > 0x3F {
            u = 0x80 - u;
        }
        let v = if u == 0x40 {
            0x7FFF
        } else {
            seno_calculado(u as usize) as i32
        };
        if angulo & 0xFF > 0x7F {
            -v
        } else {
            v
        }
    }

    fn cos(&self, angulo: i32) -> i32 {
        self.sen((angulo + 0x40) & 0xFF)
    }
}

/// `trunc(sin(i·π/128)·32767)`, i = 0..63.
pub fn seno_calculado(i: usize) -> i16 {
    ((i as f64 * std::f64::consts::PI / 128.0).sin() * 32767.0) as i16
}

// ---------------------------------------------------------------------------------------------
// Ligação com o tick
// ---------------------------------------------------------------------------------------------

impl RegrasInimigos for RegrasOriginais {
    fn simular_formigas_leao(&mut self, e: &mut Estado) {
        formigas_leao(e, TabelasInimigos::padrao());
    }

    fn mover_aranha(&mut self, e: &mut Estado) {
        aranha(e, TabelasInimigos::padrao(), self.geracao.as_ref());
    }

    fn simular_lagarta(&mut self, e: &mut Estado) {
        lagarta(e, TabelasInimigos::padrao());
    }
}

// ---------------------------------------------------------------------------------------------
// Auxiliares do mapa, da vida e da lista A (privadas; candidatas a módulo comum)
// ---------------------------------------------------------------------------------------------

/// `_IsValidA`.
fn no_mapa(x: i32, y: i32) -> bool {
    (0..LARGURA).contains(&x) && (0..ALTURA).contains(&y)
}

fn mapa(e: &Estado, x: i32, y: i32) -> Option<u8> {
    e.superficie.mapa.em(x, y)
}

fn gravar_mapa(e: &mut Estado, x: i32, y: i32, v: u8) {
    e.superficie.mapa.definir(x, y, v);
}

/// `_LifeA` com teste de validade (0 fora do mapa).
fn vida(e: &Estado, x: i32, y: i32) -> u8 {
    e.superficie.vida.em(x, y).unwrap_or(0)
}

/// `_LifeA[x·64 + y]` **sem** teste de validade, como `_DoAntLions` lê as vizinhas: fora da coluna,
/// o índice cai na coluna vizinha. Fora do bloco inteiro (memória de outra variável no original)
/// devolve 0 [hipótese].
fn vida_linear(e: &Estado, x: i32, y: i32) -> u8 {
    let i = x * ALTURA + y;
    if (0..LARGURA * ALTURA).contains(&i) {
        e.superficie.vida.dados[i as usize]
    } else {
        0
    }
}

fn e_amarela(v: u8) -> bool {
    v == VIDA_AMARELA || v == VIDA_AMARELA_2
}

/// `_IsClearTile(1, x, y)`: no mapa, sem formiga (a amarela conta como livre) e chão (< 0x10).
fn livre(e: &Estado, x: i32, y: i32) -> bool {
    if !no_mapa(x, y) {
        return false;
    }
    let v = vida(e, x, y);
    (v == 0 || e_amarela(v)) && mapa(e, x, y).is_some_and(|m| m < 0x10)
}

/// `_IsClear3x3(1, x, y)`.
fn livre_3x3(e: &Estado, t: &TabelasInimigos, x: i32, y: i32) -> bool {
    livre(e, x, y)
        && (0..8).all(|d| {
            let (dx, dy) = t.d8(d);
            livre(e, x + dx, y + dy)
        })
}

/// `_IsThisPebble(1, v)`.
fn e_pedrinha(v: u8) -> bool {
    (0x51..=0x53).contains(&v)
}

/// `_IsItFood(v)` (quintal 0x48–0x4B, casa 0x18–0x27).
fn e_comida(e: &Estado, v: u8) -> bool {
    if e.ambiente.terreno == 0 {
        (0x48..=0x4B).contains(&v)
    } else {
        (0x18..=0x27).contains(&v)
    }
}

/// `_IsNotObstacle(1, x, y)`: no mapa e v ≤ 0x53 (quintal) ou ≤ 0x5F (casa).
fn nao_e_obstaculo(e: &Estado, x: i32, y: i32) -> bool {
    let limite = if e.ambiente.terreno == 0 { 0x53 } else { 0x5F };
    mapa(e, x, y).is_some_and(|v| v <= limite)
}

/// `_GetDis`: distância ao quadrado.
fn distancia2(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    (x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1)
}

/// `_GetDir`: 1 + direção pelo sinal de dx e dy, ou 0 se coincidem.
fn direcao_para(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    let (dx, dy) = ((x2 - x1).signum(), (y2 - y1).signum());
    match (dx, dy) {
        (0, -1) => 1,
        (0, 1) => 5,
        (0, _) => 0,
        (1, -1) => 2,
        (1, 0) => 3,
        (1, _) => 4,
        (_, -1) => 8,
        (_, 0) => 7,
        _ => 6,
    }
}

/// `_GetBestDir(1, x, y, tx, ty)` (inimigos.md 2.9): melhor vizinha livre, senão a melhor bloqueada,
/// senão −2; −1 se já está no alvo.
fn melhor_direcao(e: &Estado, t: &TabelasInimigos, x: i32, y: i32, tx: i32, ty: i32) -> i32 {
    let mut dist = distancia2(x, y, tx, ty);
    if dist <= 0 {
        return -1;
    }
    let (mut livre_k, mut bloqueada_k) = (-1, -2);
    for k in 0..8 {
        let (dx, dy) = t.d8(k);
        let (vx, vy) = (x + dx, y + dy);
        if !nao_e_obstaculo(e, vx, vy) || mapa(e, vx, vy).is_some_and(e_pedrinha) {
            continue;
        }
        let d = distancia2(vx, vy, tx, ty);
        if d < dist {
            dist = d;
            if vida(e, vx, vy) == 0 && livre(e, vx, vy) {
                livre_k = k as i32;
            } else {
                bloqueada_k = k as i32;
            }
        }
    }
    if livre_k >= 0 {
        livre_k
    } else {
        bloqueada_k
    }
}

/// `_FindInAList(x, y)`: maior índice < `_ListIndexA` com essa posição e T ≠ 0.
fn achar_na_lista_a(e: &Estado, x: i32, y: i32) -> Option<usize> {
    let l = &e.superficie.lista;
    (0..l.usadas()).rev().find(|&i| {
        let f = &l.vagas[i];
        f.x as i32 == x && f.y as i32 == y && f.tipo != 0
    })
}

/// `_FindAntIndex(1, x, y, T)`: maior índice < `_ListIndexA` com essa posição e esse T.
fn achar_formiga(e: &Estado, x: i32, y: i32, t: u8) -> Option<usize> {
    let l = &e.superficie.lista;
    (0..l.usadas()).rev().find(|&i| {
        let f = &l.vagas[i];
        f.x as i32 == x && f.y as i32 == y && f.tipo == t
    })
}

/// `_RemoveFromAList(i)`: zera a vida da formiga e desloca as vagas seguintes uma posição para trás
/// (a última vaga usada fica repetida, como no original).
fn remover_da_lista_a(e: &mut Estado, i: usize) {
    let f = e.superficie.lista.vagas[i];
    let indice_vida = f.x as usize * ALTURA as usize + f.y as usize;
    if let Some(c) = e.superficie.vida.dados.get_mut(indice_vida) {
        *c = 0;
    }
    let l = &mut e.superficie.lista;
    if l.indice > 0 {
        l.indice -= 1;
    }
    let n = l.indice.max(0) as usize;
    if n > i {
        l.vagas.copy_within(i + 1..n + 1, i);
    }
}

/// `_DeadAntHere(x, y, cor)` (superficie.md 6.4): cadáver no anel de 100 posições. `cor` ≠ 0 =
/// vermelha.
fn cadaver(e: &mut Estado, x: i32, y: i32, cor: i32) {
    let p = &mut e.partida;
    p.cadaver_ponteiro += 1;
    if p.cadaver_ponteiro > 99 {
        p.cadaver_ponteiro = 0;
    }
    let k = p.cadaver_ponteiro as usize;
    let (ox, oy) = (p.cadaveres_x[k] as i32, p.cadaveres_y[k] as i32);
    let velho = mapa(e, ox, oy).unwrap_or(0);
    let casa = e.ambiente.terreno != 0;
    if !casa {
        if (0x10..0x18).contains(&velho) {
            let v = e.s.mascara(16) as u8;
            gravar_mapa(e, ox, oy, v);
        }
    } else if (8..0x18).contains(&velho) {
        gravar_mapa(e, ox, oy, (velho - 8) >> 2);
    }
    e.partida.cadaveres_x[k] = x as u8;
    e.partida.cadaveres_y[k] = y as u8;
    let v = mapa(e, x, y).unwrap_or(0xFF);
    if !casa {
        if v <= 0x17 {
            let base = if cor == 0 { 0x10 } else { 0x14 };
            let novo = base + e.s.mascara(4) as u8;
            gravar_mapa(e, x, y, novo);
        }
    } else if v <= 3 {
        let novo = if cor == 0 {
            (v + 2) * 4 + e.s.rand1(2) as u8
        } else {
            v * 4 + 10 + e.s.rand1(2) as u8
        };
        gravar_mapa(e, x, y, novo);
    }
    e.superficie.vida.definir(x, y, 0);
}

/// `_DropFoodA(x, y)` (superficie.md 8.2): uma unidade de comida na célula; `false` se não coube.
/// Na casa, o caso "a comida cai" (tabela `10:22be/22c2` indexada por `9:9c66`) não está no estado
/// do port [em aberto]: devolve `false`.
fn largar_comida(e: &mut Estado, x: i32, y: i32) -> bool {
    let Some(v) = mapa(e, x, y) else { return false };
    let novo = if e.ambiente.terreno == 0 {
        if v > 0x4A {
            return false;
        }
        if v < 0x48 {
            0x48
        } else {
            v + 1
        }
    } else if v <= 3 {
        (v + 6) * 4
    } else if (8..=0x17).contains(&v) {
        (((v - 8) >> 2) + 6) * 4
    } else if (0x18..=0x26).contains(&v) {
        v + 1
    } else {
        return false;
    };
    gravar_mapa(e, x, y, novo);
    e.ambiente.comida_superficie += 1;
    true
}

// ---------------------------------------------------------------------------------------------
// Formiga amarela (provisório: pertence a rules/amarela.rs)
// ---------------------------------------------------------------------------------------------

/// Mapa de vida do plano da amarela (0 e 1 = superfície, 2 = ninho preto, 3 = vermelho).
fn vida_do_plano(e: &mut Estado, plano: i16) -> &mut super::estado::Grade {
    match plano {
        2 => &mut e.ninho_b.vida,
        3 => &mut e.ninho_r.vida,
        _ => &mut e.superficie.vida,
    }
}

/// `_MoveMyLife(plano, x, y, tipo, dir)` (formiga-amarela.md §2): apaga a marca atual (só onde há
/// 0xFF / 0xFE) e grava 0xFF (e 0xFE na cauda da rainha) na nova posição. Simplificação: nos ninhos não
/// reproduz a escavação de `_SetLife` (os inimigos só chamam com plano 1).
fn mover_vida_amarela(
    e: &mut Estado,
    t: &TabelasInimigos,
    plano: i16,
    x: i32,
    y: i32,
    tipo: i16,
    dir: i16,
) {
    apagar_vida_amarela(e, t);
    let plano = if plano == 0 { 1 } else { plano };
    let largura = if plano < 2 { LARGURA } else { ALTURA };
    if !((0..largura).contains(&x) && (0..ALTURA).contains(&y)) {
        return;
    }
    let g = vida_do_plano(e, plano);
    g.definir(x, y, VIDA_AMARELA);
    if tipo == 0x60 {
        let (dx, dy) = t.d8((dir ^ 4) as usize);
        g.definir(x + dx, y + dy, VIDA_AMARELA_2);
    }
    let a = &mut e.amarela;
    a.x = x as i16;
    a.y = y as i16;
    a.dir = dir;
    a.tipo = tipo;
    a.plano = plano;
}

/// `_ClearMyLife` na posição atual da amarela.
fn apagar_vida_amarela(e: &mut Estado, t: &TabelasInimigos) {
    let a = e.amarela.clone();
    let (x, y) = (a.x as i32, a.y as i32);
    let g = vida_do_plano(e, a.plano);
    if g.em(x, y) == Some(VIDA_AMARELA) {
        g.definir(x, y, 0);
    }
    if a.tipo == 0x60 {
        let (dx, dy) = t.d8((a.dir ^ 4) as usize);
        if g.em(x + dx, y + dy) == Some(VIDA_AMARELA_2) {
            g.definir(x + dx, y + dy, 0);
        }
    }
}

/// `_YellowDeath(causa)` **provisório** (formiga-amarela.md §9; a implementação completa, com
/// reencarnação e animações, é de `rules/amarela.rs`). Aqui só: o sorteio da música (causas 1 e 2),
/// o contador da causa e apagar a marca da amarela do mapa de vida.
fn morte_amarela(e: &mut Estado, t: &TabelasInimigos, causa: i32) {
    match causa {
        1 | 2 => {
            let cor = super::geracao::Cor::Preta;
            e.colonia_mut(cor).comidas_por_predadores += 1;
            let n = e.s.rand1(5) as u16;
            e.eventos.push(Evento::Musica(0x4E27 + n));
        }
        _ => e.preta.mortas_em_luta += 1,
    }
    if e.amarela.modo == 0 {
        apagar_vida_amarela(e, t);
    }
}

// ---------------------------------------------------------------------------------------------
// Formiga-leão (inimigos.md §1)
// ---------------------------------------------------------------------------------------------

/// Sorteio de posição de `_AddRandAntLion` (1.2): até 200 tentativas.
fn sortear_formiga_leao(e: &mut Estado, t: &TabelasInimigos) -> Option<(i32, i32)> {
    let r = &t.regras;
    for tentativa in 0..r.leao_tentativas {
        let x = e.s.rand1(64) + e.s.rand1(65);
        let y = e.s.rand1(32) + e.s.rand1(33);
        if livre_3x3(e, t, x, y) || (livre(e, x, y) && tentativa >= r.leao_tentativas_3x3) {
            return Some((x, y));
        }
    }
    None
}

/// `_AddAntLion(x, y)`: centro, borda nas vizinhas livres e a vaga `_LionIndex` (incrementado até 9).
fn criar_formiga_leao(e: &mut Estado, t: &TabelasInimigos, x: i32, y: i32) {
    let r = &t.regras;
    gravar_mapa(e, x, y, r.tile_leao_centro);
    for d in 0..8 {
        let (dx, dy) = t.d8(d);
        if livre(e, x + dx, y + dy) {
            gravar_mapa(
                e,
                x + dx,
                y + dy,
                r.tile_leao_borda + t.borda_formiga_leao[d],
            );
        }
    }
    let fl = &mut e.formigas_leao;
    let k = fl.quantidade.clamp(0, 9) as usize;
    fl.x[k] = x as u8;
    fl.y[k] = y as u8;
    fl.m[k] = 0;
    fl.s[k] = 0;
    fl.t[k] = 0;
    if fl.quantidade < r.leao_maximo {
        fl.quantidade += 1;
    }
}

/// Retira a formiga-leão `k` (centro vira "morta", a borda fica) e desloca as seguintes.
fn remover_formiga_leao(e: &mut Estado, t: &TabelasInimigos, k: usize) {
    let (x, y) = (e.formigas_leao.x[k] as i32, e.formigas_leao.y[k] as i32);
    gravar_mapa(e, x, y, t.regras.tile_leao_morta);
    let fl = &mut e.formigas_leao;
    if fl.quantidade > 0 {
        fl.quantidade -= 1;
        let n = (fl.quantidade.max(0) as usize).min(9);
        if k < n {
            fl.x.copy_within(k + 1..n + 1, k);
            fl.y.copy_within(k + 1..n + 1, k);
            fl.t.copy_within(k + 1..n + 1, k);
            fl.m.copy_within(k + 1..n + 1, k);
            fl.s.copy_within(k + 1..n + 1, k);
        }
    }
}

/// `_KillAntLion(k)` (1.4): como a morte por cerco, sem mensagem. Para o Inseticida do Experimental.
pub fn matar_formiga_leao(e: &mut Estado, t: &TabelasInimigos, k: usize) {
    if k < 10 {
        remover_formiga_leao(e, t, k);
    }
}

/// `_FindInLionList(x, y)`: maior índice < `_LionIndex` com esse centro.
pub fn achar_formiga_leao(e: &Estado, x: i32, y: i32) -> Option<usize> {
    let fl = &e.formigas_leao;
    (0..(fl.quantidade.clamp(0, 10) as usize))
        .rev()
        .find(|&k| fl.x[k] as i32 == x && fl.y[k] as i32 == y)
}

/// `_DoAntLions` (1.3), todo tick.
pub fn formigas_leao(e: &mut Estado, t: &TabelasInimigos) {
    let r = &t.regras;
    let fl = &e.formigas_leao;
    // Curto-circuito: S1(1024) só é sorteado com nenhuma formiga-leão e um patch que tinha alguma.
    if fl.quantidade == 0 && fl.iniciais > 0 && e.s.rand1(r.leao_chance_nova) == 0 {
        if let Some((x, y)) = sortear_formiga_leao(e, t) {
            criar_formiga_leao(e, t, x, y);
        }
        return;
    }
    let mut k = 0usize;
    // A quantidade é relida a cada volta: uma formiga-leão criada no estado 2 roda no mesmo tick.
    while k < 10 && (k as i16) < e.formigas_leao.quantidade {
        let (x, y) = (e.formigas_leao.x[k] as i32, e.formigas_leao.y[k] as i32);
        match e.formigas_leao.m[k] {
            0 => formiga_leao_esperando(e, t, k, x, y),
            1 => {
                let n = (0..8)
                    .filter(|&d| {
                        let (dx, dy) = t.d8(d);
                        vida_linear(e, x + dx, y + dy) != 0
                    })
                    .count() as i32;
                if n >= r.leao_cerco {
                    e.eventos.push(Evento::Mensagem(r.mensagem_leao_morta));
                    remover_formiga_leao(e, t, k);
                    // O original segue para k + 1: a formiga-leão deslocada para a vaga k não roda neste
                    // tick (fato visto na desmontagem, 1030:48ce → 4ac8).
                } else {
                    let fl = &mut e.formigas_leao;
                    if fl.t[k] < 4 {
                        fl.t[k] += 1;
                    } else if fl.s[k] == 0 {
                        fl.m[k] = 2;
                        fl.t[k] = 4;
                    } else {
                        fl.s[k] -= 1;
                        if fl.s[k] & 1 != 0 {
                            fl.t[k] = e.s.rand1(4) as u8 + 3;
                        }
                    }
                    let tile = r.tile_leao_centro + e.formigas_leao.t[k];
                    gravar_mapa(e, x, y, tile);
                }
            }
            2 => {
                if e.formigas_leao.t[k] == 0 {
                    e.formigas_leao.m[k] = 0;
                    e.partida.comidas_por_formiga_leao =
                        e.partida.comidas_por_formiga_leao.wrapping_add(1);
                    if e.formigas_leao.quantidade < r.leao_maximo
                        && e.partida.comidas_por_formiga_leao & 15 == 15
                    {
                        if let Some((nx, ny)) = sortear_formiga_leao(e, t) {
                            criar_formiga_leao(e, t, nx, ny);
                        }
                    }
                } else {
                    e.formigas_leao.t[k] -= 1;
                }
                let tile = r.tile_leao_centro + e.formigas_leao.t[k];
                gravar_mapa(e, x, y, tile);
            }
            _ => {}
        }
        k += 1;
    }
}

/// Estado 0 da formiga-leão `k`: cospe o que caiu na armadilha ou olha o centro e uma vizinha.
fn formiga_leao_esperando(e: &mut Estado, t: &TabelasInimigos, k: usize, x: i32, y: i32) {
    let r = &t.regras;
    let v = mapa(e, x, y).unwrap_or(0);
    if e_pedrinha(v) || e_comida(e, v) {
        if e.s.rand1(r.leao_chance_cuspir) == 0 {
            let mut d = e.s.mascara(8) as usize;
            for _ in 0..8 {
                let (dx, dy) = t.d8(d);
                let (ax, ay) = (x + 2 * dx, y + 2 * dy);
                if livre(e, ax, ay) {
                    gravar_mapa(e, ax, ay, v);
                    break;
                }
                d = (d + 1) & 7;
            }
            e.formigas_leao.s[k] = r.leao_tempo_cuspir;
            formiga_leao_comecar_a_comer(e, t, k, x, y);
        }
        return;
    }
    let mut c = 0usize; // índice em _Dx9/_Dy9: o primeiro é o próprio centro
    for _ in 0..2 {
        let (cx, cy) = (x + t.dx9[c] as i32, y + t.dy9[c] as i32);
        let presa = vida_linear(e, cx, cy);
        if presa != 0 {
            if e_amarela(presa) {
                let (tipo, dir) = (e.amarela.tipo, e.amarela.dir);
                mover_vida_amarela(e, t, 1, x, y, tipo, dir);
                morte_amarela(e, t, 2);
            } else {
                let _som = e.s.mascara(8) == 0;
                if let Some(j) = achar_na_lista_a(e, cx, cy) {
                    remover_da_lista_a(e, j);
                }
                let c = if presa & 0x80 == 0 {
                    &mut e.preta
                } else {
                    &mut e.vermelha
                };
                c.comidas_por_predadores = c.comidas_por_predadores.wrapping_add(1);
            }
            e.formigas_leao.s[k] = r.leao_tempo_comer;
            formiga_leao_comecar_a_comer(e, t, k, x, y);
            return;
        }
        // Sorteado também depois da segunda célula vazia.
        c = e.s.rand1(8) as usize + 1;
    }
}

fn formiga_leao_comecar_a_comer(e: &mut Estado, t: &TabelasInimigos, k: usize, x: i32, y: i32) {
    e.formigas_leao.m[k] = 1;
    e.formigas_leao.t[k] = 1;
    gravar_mapa(e, x, y, t.regras.tile_leao_centro + 1);
}

// ---------------------------------------------------------------------------------------------
// Aranha (inimigos.md §2)
// ---------------------------------------------------------------------------------------------

/// `_KillSpider` (2.8): agonia, sem mensagem. Chamada pelo menino, pé, cortador e Inseticida.
pub fn matar_aranha(e: &mut Estado, t: &TabelasInimigos) {
    let a = &mut e.aranha;
    a.estado = 5;
    a.agonia = t.regras.aranha_agonia;
    a.ciclo_pernas = 0;
}

/// `_ScanForAnts` / cerco: células ocupadas no 3 × 3 em volta da célula (cx, cy).
fn ocupadas_3x3(e: &Estado, cx: i32, cy: i32) -> i32 {
    let mut n = 0;
    for dx in -1..=1 {
        for dy in -1..=1 {
            if vida(e, cx + dx, cy + dy) != 0 {
                n += 1;
            }
        }
    }
    n
}

/// Um passo da aranha na direção atual: `vezes` = 1 (andar) ou 5 (correr); `_RevSpider` inverte.
fn passo_aranha(e: &mut Estado, t: &TabelasInimigos, vezes: i16, de_re_vale: bool) {
    let a = &mut e.aranha;
    let d = (a.dir & 7) as usize;
    let sinal = if de_re_vale && a.de_re != 0 { -1 } else { 1 };
    a.x = a.x.wrapping_add(t.passo_aranha_x[d] as i16 * vezes * sinal);
    a.y = a.y.wrapping_add(t.passo_aranha_y[d] as i16 * vezes * sinal);
    let quadros = if vezes == 1 { 1 } else { 2 };
    a.ciclo_pernas = (a.ciclo_pernas + quadros * sinal) & 1023;
}

/// Achou presa (`_Starg` ≠ −2): passa a perseguir e guarda o T da presa (0xFF para a amarela).
fn aranha_achou(e: &mut Estado) {
    let a = &mut e.aranha;
    a.estado = 2;
    a.alvo_tipo = if a.alvo >= 0 {
        e.superficie.lista.vagas[a.alvo as usize].tipo as i16
    } else {
        0xFF
    };
}

/// `_MoveSpider` (2.2), todo tick.
pub fn aranha(e: &mut Estado, t: &TabelasInimigos, geracao: Option<&TabelasGeracao>) {
    let r = &t.regras;
    let seno = Seno(geracao);
    e.aranha.ciclo = (e.aranha.ciclo + 1) & 1023;
    let (cx, cy) = ((e.aranha.x >> 4) as i32, (e.aranha.y >> 4) as i32);

    if e.amarela.modo == 1 && e.aranha.estado != 2 && e.aranha.estado != 3 {
        aranha_do_jogador(e, t, &seno);
        return;
    }
    if e.aranha.ligada == 0 {
        aranha_aparecer(e, t);
        return;
    }

    // Cerco, a cada 4 ticks e fora da agonia.
    if e.aranha.ciclo & 3 == 0 && e.aranha.estado < 5 {
        let n = ocupadas_3x3(e, cx, cy);
        if n >= r.aranha_cerco_morte {
            matar_aranha(e, t);
            if e.amarela.modo == 1 {
                e.amarela.modo = 0;
                morte_amarela(e, t, 3);
                return;
            }
            e.eventos.push(Evento::Mensagem(r.mensagem_aranha_morta));
            let a = &mut e.aranha;
            if a.vinganca < r.aranha_vinganca_maxima {
                a.vinganca += 1;
            }
            if a.vinganca >= r.aranha_vinganca_faro {
                e.amarela.modo_aranha = if a.vinganca < r.aranha_vinganca_laser {
                    7
                } else {
                    8
                };
            }
            // (O sorteio S2() que zeraria a vingança com _SpidRevenge ≥ 6 é inalcançável.)
            return;
        }
        if n >= r.aranha_cerco_fuga {
            e.aranha.estado = 4;
        }
    }
    if e.aranha.estado < 4 && e.amarela.modo_aranha == 8 {
        laser(e, t, &seno);
    }

    match e.aranha.estado {
        0 => {
            e.aranha.ciclo_pernas = 2;
            if e.s.rand1(r.aranha_chance_andar) == 0 {
                e.aranha.estado = 1;
            }
            e.aranha.alvo = procurar_presa(e, t);
            if e.aranha.alvo != -2 {
                aranha_achou(e);
                return;
            }
            if e.s.rand1(r.aranha_chance_girar_parada) == 0 {
                let k = e.s.rand1(8);
                e.aranha.dir = t.giro(e.aranha.dir, k);
            }
            if e.amarela.modo_aranha == 8 {
                laser(e, t, &seno);
            }
        }
        1 => {
            passo_aranha(e, t, 1, true);
            if e.s.rand1(r.aranha_chance_girar_andando) == 0 {
                let k = e.s.rand1(8);
                e.aranha.dir = t.giro(e.aranha.dir, k);
            }
            e.aranha.alvo = procurar_presa(e, t);
            if e.aranha.alvo != -2 {
                aranha_achou(e);
                return;
            }
            if e.s.rand1(r.aranha_chance_parar) == 0 {
                e.aranha.estado = 0;
                e.aranha.ciclo_pernas = 2;
            }
        }
        2 => {
            if !aranha_perseguir(e, t, cx, cy) {
                return;
            }
        }
        3 => aranha_comer(e, t, cx, cy),
        4 => {
            let k = e.s.rand1(8);
            e.aranha.dir = t.giro(e.aranha.dir, k);
            passo_aranha(e, t, r.aranha_corrida, true);
            if e.s.rand1(r.aranha_chance_parar) == 0 {
                e.aranha.estado = 0;
                e.aranha.ciclo_pernas = 2;
            }
        }
        5 => {
            e.aranha.agonia -= 1;
            if e.aranha.agonia == 0 {
                e.aranha.ligada = 0;
                e.aranha.estado = 0;
                for _ in 0..3 {
                    largar_comida(e, cx, cy);
                }
                return;
            }
            if (e.s.rand1(1000) as i16) < e.aranha.agonia {
                e.aranha.ciclo_pernas = if e.aranha.agonia <= r.aranha_agonia_lenta {
                    e.s.rand1(2) as i16 + 2
                } else {
                    e.s.rand1(3) as i16 + 1
                };
            }
        }
        _ => {}
    }

    // Saiu do mapa; na casa, sobre tile alto.
    let (nx, ny) = ((e.aranha.x >> 4) as i32, (e.aranha.y >> 4) as i32);
    if !no_mapa(nx, ny) {
        e.aranha.ligada = 0;
        if e.amarela.modo == 1 {
            e.amarela.modo = 0;
            morte_amarela(e, t, 4);
        }
    }
    if e.ambiente.terreno != 0 && mapa(e, nx, ny).is_some_and(|v| v > r.aranha_teto_casa) {
        e.aranha.ligada = 0;
    }
}

/// 2.3: a aranha volta, por cima ou por baixo, fora do Tutorial.
fn aranha_aparecer(e: &mut Estado, t: &TabelasInimigos) {
    if e.partida.tipo_jogo == 0 || e.s.rand1(t.regras.aranha_chance_aparecer) != 0 {
        return;
    }
    e.aranha.x = e.s.rand1(1024) as i16 + 512;
    e.aranha.ligada = 1;
    e.aranha.estado = 1;
    e.aranha.de_re = 0;
    if e.s.rand1(2) != 0 {
        e.aranha.y = 1;
        e.aranha.dir = 4;
    } else {
        e.aranha.y = 0x3FF;
        e.aranha.dir = 0;
    }
}

/// Estado 2. Devolve `true` se o tick continua (para os testes finais de posição); `false` = fim.
/// Cai direto no estado 3 quando chega na presa.
fn aranha_perseguir(e: &mut Estado, t: &TabelasInimigos, cx: i32, cy: i32) -> bool {
    let r = &t.regras;
    let alvo = e.aranha.alvo;
    if alvo < 0 {
        if e.amarela.plano > 1 {
            e.aranha.estado = 0;
            e.aranha.alvo = -2;
            return false;
        }
    } else {
        let tipo = e.superficie.lista.vagas[alvo as usize].tipo as i16;
        if (tipo ^ e.aranha.alvo_tipo) & 0xF0 != 0 {
            // A presa mudou (morreu, pegou carga...).
            e.aranha.estado = 0;
            e.aranha.alvo = -2;
            if e.amarela.modo == 1 {
                e.aranha.destino_x = cx as i16;
                e.aranha.destino_y = cy as i16;
                e.amarela.x = cx as i16;
                e.amarela.y = cy as i16;
                if e.amarela.modo_aranha == 6 {
                    e.amarela.modo_aranha = 0;
                }
            }
            return false;
        }
    }
    let (px, py) = if alvo < 0 {
        (e.amarela.x as i32, e.amarela.y as i32)
    } else {
        let f = &e.superficie.lista.vagas[alvo as usize];
        (f.x as i32, f.y as i32)
    };
    let dist = (px - cx).abs() + (py - cy).abs();
    if dist > r.aranha_desiste && e.amarela.modo != 1 {
        e.aranha.estado = 0;
        e.aranha.alvo = -2;
        return false;
    }
    if dist > 1 {
        let d = direcao_para(cx, cy, px, py) - 1;
        e.aranha.dir = t.giro(e.aranha.dir, d);
        passo_aranha(e, t, r.aranha_corrida, true);
        if e.amarela.modo == 1 {
            e.amarela.x = e.aranha.x >> 4;
            e.amarela.y = e.aranha.y >> 4;
        }
        return true;
    }
    e.aranha.estado = 3;
    e.aranha.x = (e.aranha.x & !0xF) + 8;
    e.aranha.y = (e.aranha.y & !0xF) + 8;
    aranha_comer(e, t, cx, cy);
    true
}

/// Estado 3 (comer e mastigar). `(cx, cy)` é a célula do começo do tick.
fn aranha_comer(e: &mut Estado, t: &TabelasInimigos, cx: i32, cy: i32) {
    let r = &t.regras;
    if e.amarela.modo == 1 {
        e.amarela.x = e.aranha.x >> 4;
        e.amarela.y = e.aranha.y >> 4;
        if e.amarela.modo_aranha == 6 {
            e.amarela.modo_aranha = 0;
        }
    }
    let d = (e.aranha.dir & 7) as usize;
    let bx = (cx + t.boca_aranha_x[d] as i32) & 0x7F;
    let by = (cy + t.boca_aranha_y[d] as i32) & 0x3F;
    let alvo = e.aranha.alvo;
    if alvo != -2 {
        if alvo < 0 {
            let (plano, tipo, dir) = (e.amarela.plano, e.amarela.tipo, e.amarela.dir);
            mover_vida_amarela(e, t, plano, bx, by, tipo, dir);
            morte_amarela(e, t, 1);
            e.aranha.base_cadaver = 0;
        } else {
            let f = e.superficie.lista.vagas[alvo as usize];
            let tipo_guardado = e.aranha.alvo_tipo;
            if (tipo_guardado ^ f.tipo as i16) & 0xF0 != 0 {
                e.aranha.alvo = -2;
                aranha_terminar(e, cx, cy);
                return;
            }
            if tipo_guardado & 0x80 == 0 {
                e.preta.comidas_por_predadores = e.preta.comidas_por_predadores.wrapping_add(1);
                e.aranha.base_cadaver = 0;
            } else {
                e.vermelha.comidas_por_predadores =
                    e.vermelha.comidas_por_predadores.wrapping_add(1);
                e.aranha.base_cadaver = 4;
            }
            let i = f.x as usize * ALTURA as usize + f.y as usize;
            if let Some(c) = e.superficie.vida.dados.get_mut(i) {
                *c = 0;
            }
            e.superficie.lista.vagas[alvo as usize].tipo = 0;
        }
        e.aranha.alvo = -2;
        e.aranha.mastigando = if e.amarela.modo_aranha < 7 {
            r.aranha_mastigar
        } else {
            r.aranha_mastigar_rapido
        };
    }
    // Mastiga: restos na boca.
    if e.ambiente.terreno == 0 && mapa(e, bx, by).is_some_and(|v| v < 0x18) {
        let v = e.s.mascara(4) as u8 + e.aranha.base_cadaver as u8 + 0x10;
        gravar_mapa(e, bx, by, v);
    }
    if e.aranha.mastigando > 0 {
        e.aranha.mastigando -= 1;
        if e.aranha.mastigando % 10 == 0 && e.s.mascara(2) != 0 {
            let _som = e.s.mascara(256);
        }
        return;
    }
    e.aranha.arrotos -= 1;
    if e.aranha.arrotos == 0 {
        e.aranha.arrotos = r.aranha_arroto;
        if e.partida.opcoes[5] != 0 {
            let _arroto = e.s.mascara(2) == 0;
        }
    }
    let base = e.aranha.base_cadaver as i32;
    cadaver(e, bx, by, base);
    aranha_terminar(e, cx, cy);
}

fn aranha_terminar(e: &mut Estado, cx: i32, cy: i32) {
    e.aranha.estado = 0;
    if e.amarela.modo == 1 {
        if e.amarela.modo_aranha == 6 {
            e.amarela.modo_aranha = 0;
        }
        e.aranha.destino_x = cx as i16;
        e.aranha.destino_y = cy as i16;
    }
}

/// 2.7: o jogador é a aranha (fora dos estados 2 e 3).
fn aranha_do_jogador(e: &mut Estado, t: &TabelasInimigos, seno: &Seno) {
    match e.amarela.modo_aranha {
        7 => {
            e.aranha.alvo = procurar_presa(e, t);
            if e.aranha.alvo != -2 {
                aranha_achou(e);
                return;
            }
        }
        8 => {
            laser(e, t, seno);
        }
        _ => {}
    }
    let (mx, my) = (e.amarela.x as i32, e.amarela.y as i32);
    let (ux, uy) = (e.aranha.destino_x as i32, e.aranha.destino_y as i32);
    let dist = distancia2(mx, my, ux, uy);
    if dist < 1 {
        e.aranha.ciclo_pernas = 2;
        return;
    }
    let mut d = melhor_direcao(e, t, mx, my, ux, uy);
    if d == -1 {
        return;
    }
    if d == -2 {
        d = direcao_para(mx, my, ux, uy) - 1;
        if d < 0 {
            return;
        }
    }
    e.aranha.dir = t.giro(e.aranha.dir, d);
    // O jogador nunca anda de ré.
    if e.amarela.comando == 0 || dist < 3 {
        passo_aranha(e, t, 1, false);
    } else {
        passo_aranha(e, t, t.regras.aranha_corrida, false);
    }
    e.amarela.x = e.aranha.x >> 4;
    e.amarela.y = e.aranha.y >> 4;
}

/// `_SFoundAnt` (2.6): índice da presa na lista A, −1 = a amarela, −2 = nada. Sem sorteio.
fn procurar_presa(e: &Estado, t: &TabelasInimigos) -> i16 {
    let r = &t.regras;
    let (cx, cy) = ((e.aranha.x >> 4) as i32, (e.aranha.y >> 4) as i32);
    if e.amarela.modo_aranha == 7 {
        let l = &e.superficie.lista;
        for i in (0..l.usadas()).rev() {
            let f = &l.vagas[i];
            if f.tipo != 0 && distancia2(cx, cy, f.x as i32, f.y as i32) < r.aranha_faro {
                return i as i16;
            }
        }
        let a = &e.amarela;
        if a.modo == 0 && a.plano == 1 && distancia2(cx, cy, a.x as i32, a.y as i32) < r.aranha_faro
        {
            return -1;
        }
        return -2;
    }
    let (dx, dy) = t.d8((e.aranha.dir & 7) as usize);
    let (mut x, mut y) = (cx, cy);
    for _ in 0..r.aranha_visao_celulas {
        x += dx;
        y += dy;
        if !no_mapa(x, y) || distancia2(cx, cy, x, y) > r.aranha_visao {
            return -2;
        }
        let v = vida(e, x, y);
        if v != 0 {
            if e_amarela(v) {
                return -1;
            }
            if let Some(j) = achar_formiga(e, x, y, v) {
                return j as i16;
            }
        }
    }
    -2
}

/// `_SpiderScan` (2.6, laser; só com `_MeSMode = 8`). Até 128 sorteios `S1(12)` sem alvo.
fn laser(e: &mut Estado, t: &TabelasInimigos, seno: &Seno) -> i32 {
    if e.aranha.estado == 5 {
        return -1;
    }
    let base = (((e.aranha.dir - 2) & 7) * 32) as i32;
    let (cx, cy) = ((e.aranha.x >> 4) as i32, (e.aranha.y >> 4) as i32);
    for _ in 0..2 {
        for a in base - 32..base + 32 {
            let raio = e.s.rand1(t.regras.aranha_laser_raio) + 1;
            let px = cx + seno.cos(a) * raio / 0x7FFF;
            let py = cy + seno.sen(a) * raio / 0x7FFF;
            if !no_mapa(px, py) {
                continue;
            }
            let v = vida(e, px, py);
            if v == 0 {
                continue;
            }
            if let Some(j) = achar_formiga(e, px, py, v) {
                // (_DoLaserFire: só desenho.)
                if e.s.mascara(4) != 0 {
                    e.superficie.vida.definir(px, py, 0);
                    e.superficie.lista.vagas[j].tipo = 0;
                    cadaver(e, px, py, (v & 0x80) as i32);
                }
                return j as i32;
            }
        }
    }
    -1
}

// ---------------------------------------------------------------------------------------------
// Lagarta e bicho-de-conta (inimigos.md §3 e §4)
// ---------------------------------------------------------------------------------------------

/// Passo da lagarta por direção (0 sobe, 1 direita, 2 desce, 3 esquerda).
fn passo_lagarta(dir: i16) -> Option<(i32, i32)> {
    match dir {
        0 => Some((0, -1)),
        1 => Some((1, 0)),
        2 => Some((0, 1)),
        3 => Some((-1, 0)),
        _ => None,
    }
}

/// Índice em `_PillarMap` da célula (x, y): `y % 6` na vertical, `x % 6` na horizontal. Só chamado
/// com a célula no mapa (coordenadas não negativas).
fn indice_chao(dir: i16, x: i32, y: i32) -> usize {
    (if dir & 1 == 0 { y } else { x } % 6) as usize
}

/// Guarda o chão sob (x, y) em `_PillarMap`, se a célula existe.
fn guardar_chao_lagarta(e: &mut Estado, x: i32, y: i32) {
    if let Some(v) = mapa(e, x, y) {
        let lb = &mut e.lagarta_e_bichos;
        lb.lagarta_chao[indice_chao(lb.lagarta_dir, x, y)] = v as i16;
    }
}

/// Repõe o chão guardado em (x, y), se a célula existe.
fn repor_chao_lagarta(e: &mut Estado, x: i32, y: i32) {
    if no_mapa(x, y) {
        let lb = &e.lagarta_e_bichos;
        let v = lb.lagarta_chao[indice_chao(lb.lagarta_dir, x, y)] as u8;
        gravar_mapa(e, x, y, v);
    }
}

/// `_DoPillar` (3.2), em ticks ímpares; só no quintal. Começa por `_DoSow`.
pub fn lagarta(e: &mut Estado, t: &TabelasInimigos) {
    if e.ambiente.terreno == 1 {
        return;
    }
    bichos_de_conta(e, t);
    let r = &t.regras;
    if e.lagarta_e_bichos.lagarta_estado == 0 {
        criar_lagarta(e, t);
        e.lagarta_e_bichos.lagarta_estado = 1;
        e.lagarta_e_bichos.lagarta_fase = r.lagarta_fase_inicial;
        return;
    }
    let lb = &e.lagarta_e_bichos;
    let dir = lb.lagarta_dir;
    // [hipótese] Direção fora de 0–3 (save corrompido): o original leria lixo; aqui nada acontece.
    let Some((px, py)) = passo_lagarta(dir) else {
        return;
    };
    let (hx, hy) = (lb.lagarta_x as i32, lb.lagarta_y as i32);
    if vida(e, hx + px, hy + py) != 0 {
        // Uma formiga bloqueia: a lagarta não anda; cercada, morre e vira comida.
        if ocupadas_3x3(e, hx, hy) >= r.lagarta_cerco {
            e.lagarta_e_bichos.lagarta_estado = 0;
            comida_de_lagarta(e, t);
        }
        return;
    }
    let horizontal = (dir & 1) as usize;
    let atras = |k: i32| (hx - k * px, hy - k * py);
    let s = e.lagarta_e_bichos.lagarta_fase - 1;
    e.lagarta_e_bichos.lagarta_fase = s;
    let s = s as i32;
    if s == 4 {
        let (x, y) = atras(5);
        repor_chao_lagarta(e, x, y);
    } else {
        let (x, y) = atras(s + 1);
        gravar_mapa(e, x, y, r.tiles_lagarta_corpo_1[horizontal]);
    }
    let (x, y) = atras(s);
    gravar_mapa(e, x, y, r.tiles_lagarta_corpo_2[horizontal]);
    if s != 0 {
        return;
    }
    let (nx, ny) = (hx + px, hy + py);
    e.lagarta_e_bichos.lagarta_x = nx as i16;
    e.lagarta_e_bichos.lagarta_y = ny as i16;
    let [xmin, xmax, ymin, ymax] = r.lagarta_limites;
    if nx < xmin || nx > xmax || ny < ymin || ny > ymax {
        e.lagarta_e_bichos.lagarta_estado = 0;
        return;
    }
    guardar_chao_lagarta(e, nx, ny);
    gravar_mapa(e, nx, ny, r.tiles_lagarta_cabeca[dir as usize]);
    gravar_mapa(e, nx - px, ny - py, r.tiles_lagarta_corpo_1[horizontal]);
    e.lagarta_e_bichos.lagarta_fase = r.lagarta_fases;
}

/// `_MakeAPill` (3.2): direção e a cabeça na borda oposta.
fn criar_lagarta(e: &mut Estado, t: &TabelasInimigos) {
    let dir = e.s.rand1(4) as i16;
    let (x, y) = match dir {
        0 => (e.s.rand1(128), 63),
        1 => (0, e.s.rand1(64)),
        2 => (e.s.rand1(128), 0),
        _ => (127, e.s.rand1(64)),
    };
    let lb = &mut e.lagarta_e_bichos;
    lb.lagarta_dir = dir;
    lb.lagarta_x = x as i16;
    lb.lagarta_y = y as i16;
    guardar_chao_lagarta(e, x, y);
    gravar_mapa(e, x, y, t.regras.tiles_lagarta_cabeca[dir as usize]);
}

/// `_MakePillFood` (3.2): ao longo do corpo (cabeça e 5 atrás), repõe o chão e, se for < 0x18,
/// grava comida.
fn comida_de_lagarta(e: &mut Estado, t: &TabelasInimigos) {
    let lb = &e.lagarta_e_bichos;
    let Some((px, py)) = passo_lagarta(lb.lagarta_dir) else {
        return;
    };
    let (hx, hy) = (lb.lagarta_x as i32, lb.lagarta_y as i32);
    for k in 0..6 {
        let (x, y) = (hx - k * px, hy - k * py);
        if !no_mapa(x, y) {
            continue;
        }
        repor_chao_lagarta(e, x, y);
        if mapa(e, x, y).is_some_and(|v| v < t.regras.lagarta_limite_comida) {
            gravar_mapa(e, x, y, t.regras.lagarta_comida);
        }
    }
}

/// `_DoSow` (4.3): as três vagas, inclusive a 0, que nunca é inicializada (bicho "fantasma" em (0, 0)
/// num jogo novo; peculiaridade P-Sow).
pub fn bichos_de_conta(e: &mut Estado, t: &TabelasInimigos) {
    for v in 0..3 {
        if e.s.mascara(4) == 0 {
            continue;
        }
        if e.s.mascara(4) == 0 {
            let giro = e.s.rand1(3) as i16 - 1;
            let lb = &mut e.lagarta_e_bichos;
            lb.bicho_dir[v] = ((lb.bicho_dir[v] & 0xFF) + giro) & 7;
            let (x, y) = (lb.bicho_x[v] as i32, lb.bicho_y[v] as i32);
            let tile = t.sow_tab[lb.bicho_dir[v] as usize];
            // Redesenha no lugar, sem teste de validade (índice linear x·64 + y).
            let i = x * ALTURA + y;
            if (0..LARGURA * ALTURA).contains(&i) {
                e.superficie.mapa.dados[i as usize] = tile;
            }
        }
        let lb = &e.lagarta_e_bichos;
        let d = (lb.bicho_dir[v] & 7) as usize;
        let (x, y) = (lb.bicho_x[v] as i32, lb.bicho_y[v] as i32);
        let (dx, dy) = t.d8(d);
        let (ax, ay) = (x + dx, y + dy);
        let Some(destino) = mapa(e, ax, ay) else {
            continue;
        };
        if vida(e, ax, ay) != 0 || destino >= t.regras.bicho_chao {
            continue;
        }
        let chao = e.lagarta_e_bichos.bicho_chao[v] as u8;
        let i = x * ALTURA + y;
        if (0..LARGURA * ALTURA).contains(&i) {
            e.superficie.mapa.dados[i as usize] = chao;
        }
        let lb = &mut e.lagarta_e_bichos;
        lb.bicho_chao[v] = destino as i16;
        lb.bicho_x[v] = ax as i16;
        lb.bicho_y[v] = ay as i16;
        gravar_mapa(e, ax, ay, t.sow_tab[d]);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn tabelas_embutidas_carregam() {
        let t = TabelasInimigos::padrao();
        assert_eq!(t.dx9[0], 0);
        assert_eq!(&t.dx9[1..], &t.dx8[..]);
        assert_eq!(t.sow_tab, [0x72, 0x73, 0x71, 0x70, 0x72, 0x73, 0x71, 0x70]);
        assert_eq!(t.passo_aranha_y, [-4, -3, 0, 3, 4, 3, 0, -3]);
        assert_eq!(t.regras.tile_leao_centro, 0x38);
    }

    #[test]
    fn seno_calculado_da_volta_certa() {
        let s = Seno(None);
        assert_eq!(s.sen(0), 0);
        assert_eq!(s.sen(64), 0x7FFF);
        assert_eq!(s.sen(192), -0x7FFF);
        assert_eq!(s.cos(0), 0x7FFF);
        assert_eq!(s.cos(-32), s.cos(32));
    }

    #[test]
    fn direcao_e_distancia() {
        assert_eq!(direcao_para(5, 5, 5, 5), 0);
        assert_eq!(direcao_para(5, 5, 5, 1), 1);
        assert_eq!(direcao_para(5, 5, 9, 1), 2);
        assert_eq!(direcao_para(5, 5, 1, 9), 6);
        assert_eq!(distancia2(0, 0, 3, 4), 25);
    }
}
