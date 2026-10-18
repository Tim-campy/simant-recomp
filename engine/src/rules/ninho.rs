//! Formigas dentro dos ninhos (`_DoAntSimB` / `_DoAntSimR`), com ovos, rainha, escavação, comida
//! guardada, lutas no ninho, invasoras e a passagem ninho ↔ superfície.
//!
//! Especificação: docs/spec/regras/ninho.md e ciclo-de-vida.md (pseudocódigo próprio, sorteios na
//! ordem do original). Este arquivo segue as seções da spec; cada função cita a rotina do original.
//!
//! - As tabelas (direções, giro, modos, castas vermelhas, luta, borda do buraco) vêm de
//!   `data/ninho.json` ([`TabelasNinho`]), gerado por `tools/extrair_ninho.py`, e ficam em
//!   `RegrasOriginais::ninho` (com mods).
//! - A ordem e o tipo de cada sorteio (`S(n)` = [`GeradorS::rand1`], `Sk(K)` = [`GeradorS::mascara`],
//!   `SG(n)` = [`GeradorS::sg_rand`], `RR(n)` = [`GeradorR::rrand`]) é parte da regra: um sorteio só
//!   acontece se o ramo em que está for executado.
//! - As rotinas que o original compartilha com outras partes (escavação `_DigTile*`, `_SmoothEdges*`,
//!   `_FixExitMap*`, `_MakeNewHole*`, `_PlaceEgg*`, `_GetNewMode*`, `_GetWinner`, comida, `_ExitHole`)
//!   estão em `rules/comum.rs`; as da formiga amarela (`_YellowFight`, `_DoTroph`), em
//!   `rules/amarela.rs`.
//!
//! [`GeradorS::rand1`]: super::rng::GeradorS::rand1
//! [`GeradorS::mascara`]: super::rng::GeradorS::mascara
//! [`GeradorS::sg_rand`]: super::rng::GeradorS::sg_rand
//! [`GeradorR::rrand`]: super::rng::GeradorR::rrand

use std::sync::OnceLock;

use serde::Deserialize;

use super::amarela::{self, TabelasAmarela};
use super::comum::{
    self, e_amarela, e_comida_ninho as e_comida, e_terra, e_vermelha as vermelha, grupo, lista_do_ninho,
    plano_do_ninho, Direcoes, TabelaDeGiro, TabelaLuta, TabelaModos, ABERTURA_NINHO as ABERTURA, COMIDA_NINHO_CHEIA as COMIDA_CHEIA,
    COMIDA_NINHO_UMA as COMIDA_UMA, LADO_NINHO as LADO, PASSAGEM_ENTRE_NINHOS as PASSAGEM, VIDA_AMARELA,
};
use super::estado::{indice_cor, Colonia, Estado, Evento};
use super::geracao::Cor;
use super::tick::{RegrasNinho, RegrasOriginais};
use crate::formats::save::{Formiga, Lista};
use crate::recursos::Recursos;

/// Nome do arquivo de dados (em `data/` ou num mod).
pub const ARQUIVO_TABELAS: &str = "ninho.json";

// Valores de célula dos ninhos (tiles-valores.md, "Ninhos"); os comuns estão em `comum.rs`.
/// `MOVER` só entra em células abaixo disto (túnel, comida, passagem, abertura).
const LIMITE_PASSAVEL: u8 = 0x1C;
/// `PASSO` e os modos de escavação param diante de células a partir disto (terra inundada, pedrinha…).
const LIMITE_BLOQUEIO: u8 = 0x30;

// ---------------------------------------------------------------------------------------------
// Tabelas (dados)
// ---------------------------------------------------------------------------------------------

/// Tabelas de `data/comum.json` que a parte dos ninhos usa (chave aqui, chave em `comum.json`).
pub const USOS_COMUNS: &[comum::UsoComum] = &[
    ("dx8", "dx8"),
    ("dy8", "dy8"),
    ("turn_tab", "turn_tab"),
    ("modos_operaria", "modos_operaria"),
    ("modos_soldado", "modos_soldado"),
    ("modo_por_grupo", "modo_por_grupo"),
    ("classe_luta", "classe_luta"),
    ("chance_vitoria", "matriz_luta"),
    ("borda_buraco", "borda_buraco"),
];

/// Conteúdo de `data/ninho.json` (endereços de origem em `tools/extrair_ninho.py`), com as tabelas
/// comuns de `data/comum.json` ([`USOS_COMUNS`]; endereços em `tools/extrair_comum.py`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct TabelasNinho {
    /// `_Dx8`/`_Dy8`: deslocamento por direção (0 = cima, 2 = direita, 4 = baixo, 6 = esquerda).
    pub dx8: [i8; 8],
    pub dy8: [i8; 8],
    /// `_TurnTab[atual][alvo]` (`_RandTurn` usa `alvo = Sk(8)`).
    pub turn_tab: [[u8; 8]; 8],
    /// `_ModeTabWB[modo estratégico][Sk(8)]`: novo modo da operária.
    pub modos_operaria: Vec<[u8; 8]>,
    /// `_ModeTabSB[modo estratégico][Sk(8)]`: novo modo do soldado.
    pub modos_soldado: Vec<[u8; 8]>,
    /// `_CasteModeTabB[grupo]`: novo modo dos demais grupos.
    pub modo_por_grupo: [u8; 16],
    /// `_CasteTabC[_StrategicModeR mod 7][Sk(8)]`: grupo do adulto que nasce no ninho vermelho.
    pub castas_vermelhas: Vec<[u8; 8]>,
    /// Modo da vencedora vermelha no fim de uma luta no ninho vermelho (DGROUP `0x22E6`).
    pub vitoria_vermelha: [u8; 16],
    /// Classe de luta por grupo (DGROUP `0x22C6`): 0 operária/ovo, 1 soldado, 2 reprodutor, 3 rainha.
    pub classe_luta: [u8; 16],
    /// `chance_vitoria[ocupante][atacante]`: chance em 10 de a ocupante vencer (DGROUP `0x22D6`).
    pub chance_vitoria: [[u8; 4]; 4],
    /// Borda do buraco na superfície por direção (DGROUP `0x230C`).
    pub borda_buraco: [u8; 8],
    #[serde(default)]
    pub sons: SonsNinho,
}

/// Ids dos efeitos sonoros (objetos tipo 5 do banco SOUND, `_myBeginSound`) pedidos por esta parte
/// (docs/spec/regras/som.md). Seção opcional no JSON: um id ausente não toca (mods anteriores à API 0.8).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct SonsNinho {
    /// `_DoDigInB` / `_DoDigInR`: a formiga pega a terra que cavou (0x11 / 0x12).
    pub cavar_preta: Option<u16>,
    pub cavar_vermelha: Option<u16>,
}

impl TabelasNinho {
    /// Lê `ninho.json` (com mods).
    pub fn carregar(r: &Recursos) -> Result<TabelasNinho, String> {
        let t: TabelasNinho = comum::parte_dos_recursos(r, ARQUIVO_TABELAS, USOS_COMUNS)?;
        t.validar()?;
        Ok(t)
    }

    pub fn de_json(texto: &str) -> Result<TabelasNinho, String> {
        let t: TabelasNinho = comum::parte_do_texto(ARQUIVO_TABELAS, texto, USOS_COMUNS)?;
        t.validar()?;
        Ok(t)
    }

    /// Cópia embutida de `data/ninho.json` (sem mods), para `RegrasOriginais::default` e testes.
    pub fn padrao() -> &'static TabelasNinho {
        static PADRAO: OnceLock<TabelasNinho> = OnceLock::new();
        PADRAO.get_or_init(|| {
            TabelasNinho::de_json(include_str!("../../../data/ninho.json")).expect("data/ninho.json inválido")
        })
    }

    fn validar(&self) -> Result<(), String> {
        let erro = |m: &str| Err(format!("{ARQUIVO_TABELAS}: {m}"));
        comum::validar_direcoes(ARQUIVO_TABELAS, &self.dx8, &self.dy8)?;
        if self.turn_tab.iter().flatten().any(|&d| d > 7) {
            return erro("turn_tab com direção fora de 0..7");
        }
        if self.modos_operaria.is_empty() || self.modos_soldado.is_empty() || self.castas_vermelhas.is_empty() {
            return erro("modos_operaria, modos_soldado e castas_vermelhas precisam de ao menos uma linha");
        }
        if self.classe_luta.iter().any(|&c| c > 3) {
            return erro("classe_luta fora de 0..3");
        }
        Ok(())
    }

    /// Tabelas de `_GetNewModeB/R`.
    pub fn modos(&self) -> TabelaModos<'_> {
        TabelaModos { operaria: &self.modos_operaria, soldado: &self.modos_soldado, por_grupo: &self.modo_por_grupo }
    }

    /// Tabelas de `_GetWinner`.
    pub fn luta(&self) -> TabelaLuta<'_> {
        TabelaLuta { classe: &self.classe_luta, chance: &self.chance_vitoria }
    }
}

impl Direcoes for TabelasNinho {
    fn deslocamentos(&self) -> (&[i8; 8], &[i8; 8]) {
        (&self.dx8, &self.dy8)
    }
}

impl TabelaDeGiro for TabelasNinho {
    fn turn_tab(&self) -> &[[u8; 8]; 8] {
        &self.turn_tab
    }
}

// ---------------------------------------------------------------------------------------------
// Ligação com o tick
// ---------------------------------------------------------------------------------------------

impl RegrasNinho for RegrasOriginais {
    fn simular_ninho(&mut self, e: &mut Estado, cor: Cor) {
        simular_ninho(e, cor, &self.ninho, &self.amarela);
    }
}

/// `_DoAntSimB` (`1028:2d4e`) para `Cor::Preta` e `_DoAntSimR` (`1028:5344`) para `Cor::Vermelha`
/// (ninho.md §2): percorre a lista do ninho de `_ListIndex − 1` até 0, lido uma vez no começo, e
/// processa as vagas com T ≠ 0. Vagas vazias não sorteiam nada. Entradas acrescentadas durante o
/// laço só são processadas no tick seguinte. `ta` são as tabelas da amarela (luta e trofalaxia).
pub fn simular_ninho(e: &mut Estado, cor: Cor, tab: &TabelasNinho, ta: &TabelasAmarela) {
    let n = e.ninho(cor).lista.usadas();
    for i in (0..n).rev() {
        e.tindex = i as i32;
        let f = e.ninho(cor).lista.vagas[i];
        if f.tipo != 0 {
            let mut fm = Formigueiro { e, tab, ta, cor, i, t0: f.tipo, x0: f.x as i32, y0: f.y as i32, m0: f.modo };
            fm.processar();
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Rotinas comuns, com as tabelas do ninho (implementação em `comum.rs`)
// ---------------------------------------------------------------------------------------------

fn no_ninho(x: i32, y: i32) -> bool {
    (0..LADO).contains(&x) && (0..LADO).contains(&y)
}

/// `_FindInBList/RList(x, y, t)` (`1020:2c86`/`2cce`).
fn procurar(e: &Estado, cor: Cor, x: i32, y: i32, t: u8) -> Option<usize> {
    comum::achar_formiga(e, lista_do_ninho(cor), x, y, t)
}

/// `_DigTileThemB/R(x, y)` ([`comum::cavar_tunel`]). Devolve se cavou.
fn cavar_tunel(e: &mut Estado, tab: &TabelasNinho, cor: Cor, x: i32, y: i32) -> bool {
    comum::cavar_tunel(e, tab, &tab.borda_buraco, cor, x, y)
}

/// `_MakeNewHoleB/R(c)` ([`comum::novo_buraco`]).
fn novo_buraco(e: &mut Estado, tab: &TabelasNinho, cor: Cor, c: i32) {
    comum::novo_buraco(e, tab, &tab.borda_buraco, cor, c);
}

/// `_GetNewModeB(grupo)` (`1030:09d0`).
fn novo_modo_preto(e: &mut Estado, tab: &TabelasNinho, g: u8) -> u8 {
    comum::novo_modo_preto(e, tab.modos(), g)
}

/// `_GetNewModeR(grupo)` (`1030:0a50`).
fn novo_modo_vermelho(e: &mut Estado, tab: &TabelasNinho, g: u8) -> u8 {
    comum::novo_modo_vermelho(e, tab.modos(), g)
}

/// `_GetNewMode(grupo, T)` (`1030:0910`).
fn novo_modo_pela_cor(e: &mut Estado, tab: &TabelasNinho, g: u8, t: u8) -> u8 {
    comum::novo_modo(e, tab.modos(), g, t)
}

/// `_GetWinner(ocupante, atacante)` (`1028:26f4`, formigas.md §6; [`comum::vencedor_da_luta`]) com as
/// tabelas do ninho.
pub fn vencedor_da_luta(e: &mut Estado, tab: &TabelasNinho, ocupante: u8, atacante: u8) -> u8 {
    comum::vencedor_da_luta(e, tab.luta(), ocupante, atacante)
}

/// `_TryEatFoodB/R`: [`comum::comer_no_ninho`] só se a célula tiver comida (0x10–0x13).
fn comer_se_houver(e: &mut Estado, cor: Cor, x: i32, y: i32) {
    if e.ninho(cor).mapa.em(x, y).is_some_and(e_comida) {
        comum::comer_no_ninho(e, cor, x, y);
    }
}

// ---------------------------------------------------------------------------------------------
// A formiga em processamento
// ---------------------------------------------------------------------------------------------

/// Uma formiga do ninho `cor` em processamento (índice `i` = `_Tindex`), com os valores que o
/// original recebe por parâmetro (T, X, Y e M lidos no começo).
struct Formigueiro<'a> {
    e: &'a mut Estado,
    tab: &'a TabelasNinho,
    ta: &'a TabelasAmarela,
    cor: Cor,
    i: usize,
    t0: u8,
    x0: i32,
    y0: i32,
    m0: u8,
}

impl Formigueiro<'_> {
    // --- acesso ---

    fn eu(&self) -> Formiga {
        self.e.ninho(self.cor).lista.vagas[self.i]
    }

    fn eu_mut(&mut self) -> &mut Formiga {
        &mut self.e.ninho_mut(self.cor).lista.vagas[self.i]
    }

    fn t(&self) -> u8 {
        self.eu().tipo
    }

    fn pos(&self) -> (i32, i32) {
        let f = self.eu();
        (f.x as i32, f.y as i32)
    }

    fn formiga_mut(&mut self, j: usize) -> &mut Formiga {
        &mut self.e.ninho_mut(self.cor).lista.vagas[j]
    }

    fn mapa(&self, x: i32, y: i32) -> u8 {
        self.e.ninho(self.cor).mapa.em(x, y).unwrap_or(0xFF)
    }

    fn vida(&self, x: i32, y: i32) -> u8 {
        self.e.ninho(self.cor).vida.em(x, y).unwrap_or(0)
    }

    fn saida(&self, x: i32, y: i32) -> u8 {
        self.e.ninho(self.cor).saida.em(x, y).unwrap_or(0)
    }

    fn definir_vida(&mut self, x: i32, y: i32, v: u8) {
        self.e.ninho_mut(self.cor).vida.definir(x, y, v);
    }

    fn colonia(&self) -> &Colonia {
        self.e.colonia(self.cor)
    }

    fn colonia_mut(&mut self) -> &mut Colonia {
        self.e.colonia_mut(self.cor)
    }

    fn procurar(&self, x: i32, y: i32, t: u8) -> Option<usize> {
        procurar(self.e, self.cor, x, y, t)
    }

    /// Novo modo do grupo pela regra do ninho (`_GetNewModeB` ou `_GetNewModeR`).
    fn novo_modo(&mut self, g: u8) -> u8 {
        match self.cor {
            Cor::Preta => novo_modo_preto(self.e, self.tab, g),
            Cor::Vermelha => novo_modo_vermelho(self.e, self.tab, g),
        }
    }

    fn sk(&mut self, k: u16) -> i32 {
        self.e.s.mascara(k)
    }

    fn s(&mut self, n: u16) -> i32 {
        self.e.s.rand1(n)
    }

    // --- despacho (ninho.md §3) ---

    /// `_DoNestAntB` (`1028:2dae`) / `_DoNestAntR` (`1028:53a4`).
    fn processar(&mut self) {
        let dona = vermelha(self.t0) == (self.cor == Cor::Vermelha);
        if dona {
            self.processar_dona();
        } else {
            match self.cor {
                Cor::Preta => self.invasora_no_preto(),
                Cor::Vermelha => self.invasora_no_vermelho(),
            }
        }
    }

    fn contar_modo(&mut self, cor_contada: Cor) {
        if let Some(c) = self.e.modos_do_tick[indice_cor(cor_contada)].get_mut(self.m0 as usize) {
            *c += 1;
        }
    }

    /// §3.1: formiga da cor do ninho.
    fn processar_dona(&mut self) {
        self.contar_modo(self.cor);
        let m = self.m0;
        if self.sk(256) == 0 && m != 9 {
            let b = self.sk(32);
            if (self.colonia().saude as i32) < b {
                self.eu_mut().tipo = 0;
                self.definir_vida(self.x0, self.y0, 0);
                let c = self.colonia_mut();
                c.mortes_naturais = c.mortes_naturais.wrapping_add(1);
                return;
            }
        }
        let g = grupo(self.t0);
        match m {
            1 => self.cuidar_das_crias(g),
            2 | 5 | 7 | 11 | 12 | 15 | 16 => self.sair_cavando(),
            3 => self.levar_comida(),
            4 => self.cavar_para_dentro(g),
            6 => {
                if self.cor == Cor::Vermelha || self.e.amarela.plano != 2 {
                    self.sair_cavando();
                } else {
                    self.passeio(false);
                }
            }
            8 => self.simular_ovo(),
            9 => self.simular_rainha(g),
            10 => self.nuvem_de_luta(),
            13 => self.descansar(),
            14 => self.pre_voo(g),
            17 => self.afogando(),
            _ => self.passeio(true),
        }
    }

    // --- primitivas (ninho.md §4) ---

    /// `_TryMoveDirB/R(x, y, d)` (`1028:439e`/`6850`): MOVER. Sem teste de ocupação: grava o T na
    /// célula nova e zera a antiga.
    fn mover_de(&mut self, x: i32, y: i32, d: i32) -> bool {
        if !(0..8).contains(&d) {
            // d < 0 falha; d > 7 leria fora de `_Dx8` no original (não acontece nas regras).
            return false;
        }
        let (dx, dy) = self.tab.d8(d);
        let (nx, ny) = (x + dx, y + dy);
        if !(0..LADO).contains(&nx) || ny >= LADO {
            return false;
        }
        if ny < 1 {
            return self.sair_do_ninho(x);
        }
        if self.mapa(nx, ny) >= LIMITE_PASSAVEL {
            return false;
        }
        let novo = (self.t() & 0xF8) | d as u8;
        // Só no preto: alimenta a amarela. O teste `X < 0x80` do original é sempre verdadeiro.
        if self.cor == Cor::Preta
            && self.vida(nx, ny) == VIDA_AMARELA
            && self.e.amarela.quer_comida != 0
            && self.eu().x < 0x80
        {
            self.definir_vida(x, y, novo);
            amarela::trofalaxia(self.e, self.ta, x, y, d);
        }
        self.definir_vida(nx, ny, novo);
        self.definir_vida(x, y, 0);
        let f = self.eu_mut();
        f.x = nx as u8;
        f.y = ny as u8;
        f.tipo = novo;
        true
    }

    fn mover(&mut self, d: i32) -> bool {
        let (x, y) = self.pos();
        self.mover_de(x, y, d)
    }

    /// TENTA(d): `MOVER(d)` e, se falhar, uma tentativa com `Sk(8)`.
    fn tentar(&mut self, d: i32) {
        if !self.mover(d) {
            let r = self.sk(8);
            self.mover(r);
        }
    }

    /// `_GetExitDirB/R(x, y, d)` (`1020:119c`/`1240`): direção + 1 rumo ao maior valor do mapa de
    /// saída (excluindo a direção oposta a `d`), ou 0.
    fn direcao_de_saida(&mut self, x: i32, y: i32, d: i32) -> i32 {
        if y == 1 {
            if self.mapa(x, 0) == ABERTURA {
                return 1;
            }
            return self.sk(2) * 4 + 3;
        }
        let mut melhor = 0u8;
        let mut res = 0;
        for k in 0..8 {
            if k == d ^ 4 {
                continue;
            }
            let (dx, dy) = self.tab.d8(k);
            if !no_ninho(x + dx, y + dy) {
                continue;
            }
            let v = self.saida(x + dx, y + dy);
            if v > melhor {
                melhor = v;
                res = k + 1;
            }
        }
        res
    }

    /// `_GetEnterDirB/R(x, y, d)` (`1020:12e4`/`137c`): direção rumo ao menor valor não nulo do mapa
    /// de saída, ou −1. `Sk(2)` só em empate.
    fn direcao_de_entrada(&mut self, x: i32, y: i32, d: i32) -> i32 {
        let mut atual = self.saida(x, y);
        let mut res = -1;
        for k in 0..8 {
            if k == d ^ 4 {
                continue;
            }
            let (dx, dy) = self.tab.d8(k);
            if !no_ninho(x + dx, y + dy) {
                continue;
            }
            let v = self.saida(x + dx, y + dy);
            if v != 0 && v <= atual {
                if v >= atual && self.sk(2) == 0 {
                    continue;
                }
                res = k;
                atual = v;
            }
        }
        res
    }

    /// `_RandTurn(d)` (`1028:2a22`).
    fn giro_aleatorio(&mut self, d: i32) -> i32 {
        let k = self.sk(8);
        self.tab.virar(d, k) as i32
    }

    /// NUVEM: a inimiga `j`, na célula `c` com tipo `v`, vira nuvem de luta.
    fn virar_nuvem(&mut self, j: usize, c: (i32, i32), v: u8, t_ref: u8) {
        let w = vencedor_da_luta(self.e, self.tab, v, t_ref);
        let nuvem = 0x70u8.wrapping_add(w & 0x80);
        let f = self.formiga_mut(j);
        f.s = w;
        f.tipo = nuvem;
        f.modo = 10;
        self.definir_vida(c.0, c.1, nuvem);
    }

    /// LUTA(c, Tref) (`_CheckNestFightB/R` embutido, ninho.md §4.3): devolve se houve luta. A
    /// formiga que checa não some; quem vira nuvem é a inimiga encontrada.
    fn luta(&mut self, c: (i32, i32), t_ref: u8) -> bool {
        let v = self.vida(c.0, c.1);
        match self.cor {
            Cor::Preta => {
                if e_amarela(v) && self.e.amarela.cor != 0 {
                    amarela::lutar_com_amarela(self.e, self.ta, plano_do_ninho(self.cor), self.i as i32);
                    return true;
                }
                if (0x88..=0xE7).contains(&v) {
                    if let Some(j) = self.procurar(c.0, c.1, v) {
                        self.virar_nuvem(j, c, v, t_ref);
                        return true;
                    }
                }
                false
            }
            Cor::Vermelha => {
                if (0x08..=0x67).contains(&v) {
                    if let Some(j) = self.procurar(c.0, c.1, v) {
                        self.virar_nuvem(j, c, v, t_ref);
                        return true;
                    }
                    return false;
                }
                if e_amarela(v) && self.e.amarela.cor == 0 {
                    amarela::lutar_com_amarela(self.e, self.ta, plano_do_ninho(self.cor), self.i as i32);
                    return true;
                }
                false
            }
        }
    }

    /// PASSO(n, d) (ninho.md §6.4): sai da célula atual e, se não houver luta em `n`, entra nela.
    /// Devolve se andou.
    fn passo(&mut self, n: (i32, i32), d: i32, t_ref: u8) -> bool {
        let (x, y) = self.pos();
        self.definir_vida(x, y, 0);
        if self.luta(n, t_ref) {
            return false;
        }
        let novo = (self.t() & 0xF8) | d as u8;
        self.definir_vida(n.0, n.1, novo);
        let f = self.eu_mut();
        f.tipo = novo;
        f.x = n.0 as u8;
        f.y = n.1 as u8;
        true
    }

    /// Vira a formiga para `d` (T e `Vida`), como fazem Got Food, Dig In e Dig Out antes de andar.
    /// Devolve o T resultante (calculado a partir do T de entrada).
    fn virar_para(&mut self, d: i32) -> u8 {
        let t = (self.t0 & 0xF8) | d as u8;
        self.definir_vida(self.x0, self.y0, t);
        self.eu_mut().tipo = t;
        t
    }

    /// Célula vizinha na direção `d` para Got Food, Dig In e Dig Out: `None` se sai do mapa pela
    /// esquerda, direita ou fundo; `Some(None)` se a formiga tentou subir da linha 1 (já tratado por
    /// `_GetOut`); `Some(Some(n))` caso contrário.
    fn vizinha_ou_saida(&mut self, d: i32) -> Option<Option<(i32, i32)>> {
        let (dx, dy) = self.tab.d8(d);
        let (nx, ny) = (self.x0 + dx, self.y0 + dy);
        if !(0..LADO).contains(&nx) || ny >= LADO {
            return None;
        }
        if ny < 1 {
            self.sair_do_ninho(self.x0);
            return Some(None);
        }
        Some(Some((nx, ny)))
    }

    /// PASSEIO (M = 0, 18, 19 e outros; `_DoRandB` embutido) e, sem a troca de modo, o "passeio sem
    /// troca" dos soldados de defesa com a amarela no ninho preto (`_DoRecruitN`).
    fn passeio(&mut self, troca: bool) {
        if troca && self.sk(32) == 0 {
            let m = self.novo_modo(grupo(self.t0));
            self.eu_mut().modo = m;
        }
        if self.luta((self.x0, self.y0), self.t0) {
            return;
        }
        self.tentar((self.t0 & 7) as i32);
    }

    // --- sair e entrar (ninho.md §4.6) ---

    /// `_GetOutB/R(x)` (`1028:520a`/`74ba`): a formiga em (x, 1) tentou subir.
    fn sair_do_ninho(&mut self, x: i32) -> bool {
        if self.mapa(x, 0) == ABERTURA {
            let t = self.t();
            self.eu_mut().tipo = 0;
            if self.colonia().buracos.get(x as usize).copied().unwrap_or(0) == 0 {
                novo_buraco(self.e, self.tab, self.cor, x);
            }
            let r = self.sk(8) as u8;
            let buraco = self.colonia().buracos.get(x as usize).copied().unwrap_or(0);
            let f = self.eu();
            if comum::sair_pelo_buraco(self.e, self.tab, buraco as i32, x, (t & 0xF8).wrapping_add(r), f.modo, f.s) {
                self.definir_vida(x, 1, 0);
                return true;
            }
            let f = self.eu_mut();
            f.tipo = t;
            f.modo = 0;
            return false;
        }
        let s0 = self.saida(x, 0);
        if s0 != 0 {
            self.e.ninho_mut(self.cor).saida.definir(x, 0, s0 - 1);
        }
        if self.sk(2) == 0 {
            if x <= 62 && e_terra(self.mapa(x + 1, 1)) {
                cavar_tunel(self.e, self.tab, self.cor, x + 1, 1);
            }
        } else if x >= 1 && e_terra(self.mapa(x - 1, 1)) {
            cavar_tunel(self.e, self.tab, self.cor, x - 1, 1);
        }
        let r = self.sk(8);
        self.mover_de(x, 1, r);
        false
    }

    // --- cuidar das crias (ninho.md §5) ---

    /// `_DoNestingB` (`1028:44a8`) / `_DoNestingR` (`1028:690a`).
    fn cuidar_das_crias(&mut self, g: u8) {
        match self.cor {
            Cor::Preta => self.cuidar_preta(g),
            Cor::Vermelha => self.cuidar_vermelha(g),
        }
    }

    fn cuidar_preta(&mut self, g: u8) {
        let (x, y) = (self.x0, self.y0);
        let dir = (self.t0 & 7) as i32;
        let s = self.eu().s;
        let estagio = s & 7;
        let v = self.vida(x, y);
        let alto = s >> 3;
        if g == 1 {
            if alto == 0 {
                let d = self.direcao_de_entrada(x, y, dir);
                if d < 0 {
                    self.eu_mut().s = estagio | 8;
                }
                self.tentar(d);
                return;
            }
            if v == 0 || v > 8 {
                let t = self.t().wrapping_add(8);
                self.eu_mut().tipo = t;
                comum::por_ovo(self.e, self.tab, Cor::Preta, x, y, estagio);
                let t = self.t();
                self.definir_vida(x, y, t);
                self.eu_mut().s = 8;
                let m = self.novo_modo(1);
                self.eu_mut().modo = m;
            }
            let e = self.direcao_de_saida(x, y, dir);
            let d = if e == 0 { self.sk(8) } else { e - 1 };
            self.tentar(d);
        } else if g != 2 {
            let r = self.s(100);
            if (self.colonia().saude as i32) < r {
                comer_se_houver(self.e, Cor::Preta, x, y);
            }
            if self.sk(8) == 0 {
                let m = self.novo_modo(g);
                self.eu_mut().modo = m;
            }
            self.tentar(dir);
        } else if alto == 0 {
            if v == 0 || v > 7 {
                let r = self.s(100);
                if (self.colonia().saude as i32) < r {
                    comer_se_houver(self.e, Cor::Preta, x, y);
                } else if self.sk(16) == 0 {
                    let m = self.novo_modo(2);
                    self.eu_mut().modo = m;
                }
            } else if let Some(j) = self.procurar(x, y, v) {
                // Pega o ovo: ele sai da lista e o estágio vai para S. Não anda neste tick.
                self.formiga_mut(j).tipo = 0;
                let f = self.eu_mut();
                f.tipo = f.tipo.wrapping_sub(8);
                f.s = v;
                return;
            }
            self.tentar(dir);
        } else {
            if self.sk(8) == 0 {
                self.eu_mut().s = 0;
            }
            let e = self.direcao_de_saida(x, y, dir);
            let d = if e == 0 { self.sk(8) } else { e - 1 };
            self.tentar(d);
        }
    }

    fn cuidar_vermelha(&mut self, g: u8) {
        let (x, y) = (self.x0, self.y0);
        let dir = (self.t0 & 7) as i32;
        if g == 1 {
            if self.sk(4) == 0 && self.mapa(x, y) < COMIDA_UMA {
                let t = self.t().wrapping_add(8);
                self.eu_mut().tipo = t;
                self.definir_vida(x, y, t);
                comum::por_ovo(self.e, self.tab, Cor::Vermelha, x, y, 0x82);
                self.eu_mut().s = 0;
                let m = self.novo_modo(1);
                self.eu_mut().modo = m;
                return;
            }
            if self.sk(4) != 0 {
                let d = self.direcao_de_entrada(x, y, dir);
                if d >= 0 {
                    self.tentar(d);
                    return;
                }
            }
            let d = self.sk(8);
            self.tentar(d);
        } else if g != 2 {
            let m = self.novo_modo(g);
            self.eu_mut().modo = m;
            self.tentar(dir);
        } else {
            if self.sk(4) == 0 {
                let v = self.vida(x, y);
                if v == 0 || (v & 0x7F) > 7 {
                    let r = self.s(100);
                    if (self.colonia().saude as i32) < r {
                        comer_se_houver(self.e, Cor::Vermelha, x, y);
                    } else {
                        let m = self.novo_modo(2);
                        self.eu_mut().modo = m;
                    }
                } else if let Some(j) = self.procurar(x, y, v) {
                    // O ovo sai da lista; S não guarda o estágio (ao largar, volta a 0x82).
                    self.formiga_mut(j).tipo = 0;
                    let f = self.eu_mut();
                    f.tipo = f.tipo.wrapping_sub(8);
                    return;
                }
            }
            if self.sk(4) != 0 {
                self.tentar(dir);
            } else {
                let d = self.sk(8);
                self.tentar(d);
            }
        }
    }

    // --- comida (ninho.md §6) ---

    /// `_DoFoodInB/R` (`1028:492a`/`6c8c`) (M = 3).
    fn levar_comida(&mut self) {
        let (x, y) = (self.x0, self.y0);
        let d = self.direcao_de_entrada(x, y, (self.t0 & 7) as i32);
        if d < 0 || self.sk(16) == 0 {
            // LARGAR_COMIDA: 0x13 cheia e 0x14 não mudam, mas a comida guardada sobe sempre.
            let v = self.mapa(x, y);
            if v < COMIDA_UMA {
                self.e.ninho_mut(self.cor).mapa.definir(x, y, COMIDA_UMA);
            } else if v < COMIDA_CHEIA {
                self.e.ninho_mut(self.cor).mapa.definir(x, y, v + 1);
            }
            let c = self.colonia_mut();
            c.comida = c.comida.wrapping_add(1);
            if self.t() & 8 != 0 {
                let f = self.eu_mut();
                f.tipo -= 8;
            }
            let r = self.s(100);
            if (self.colonia().saude as i32) < r {
                // Sem o teste de faixa (`_EatFoodB` embutido): numa célula 0x14 isso a transforma em 0x13.
                comum::comer_no_ninho(self.e, self.cor, x, y);
            }
            // Com o grupo do T de ENTRADA (3 ou 7): a "comida dobrada" (ninho.md §6.2).
            let m = self.novo_modo(grupo(self.t0));
            self.eu_mut().modo = m;
            return;
        }
        let t_ref = self.virar_para(d);
        let Some(Some(n)) = self.vizinha_ou_saida(d) else { return };
        if self.mapa(n.0, n.1) >= LIMITE_BLOQUEIO {
            return;
        }
        self.passo(n, d, t_ref);
    }

    // --- escavação (ninho.md §7) ---

    /// `_DoDigInB/R` (`1028:4bd0`/`6f0c`) (M = 4).
    fn cavar_para_dentro(&mut self, g: u8) {
        if g != 2 && g != 6 {
            let m = self.novo_modo(g);
            self.eu_mut().modo = m;
            return;
        }
        let (x, y) = (self.x0, self.y0);
        let mut d = self.direcao_de_entrada(x, y, (self.t0 & 7) as i32);
        if d < 0 {
            d = self.sk(8);
        }
        let t_ref = self.virar_para(d);
        if y == LADO - 1 {
            let m = self.novo_modo(g);
            self.eu_mut().modo = m;
            return;
        }
        let Some(Some(n)) = self.vizinha_ou_saida(d) else { return };
        let v = self.mapa(n.0, n.1);
        if v >= LIMITE_BLOQUEIO {
            return;
        }
        if e_terra(v) {
            if !cavar_tunel(self.e, self.tab, self.cor, n.0, n.1) {
                let m = match self.cor {
                    Cor::Preta => self.novo_modo(g),
                    Cor::Vermelha => 0,
                };
                self.eu_mut().modo = m;
                return;
            }
            // Carrega a terra (grupo 2 → 5, 6 → 9) e passa a "sair", com o som 0x11 (R: 0x12).
            let f = self.eu_mut();
            f.tipo = f.tipo.wrapping_add(0x18);
            f.modo = 5;
            let som = match self.cor {
                Cor::Preta => self.tab.sons.cavar_preta,
                Cor::Vermelha => self.tab.sons.cavar_vermelha,
            };
            self.e.pedir_som(som);
        }
        // O T de referência da luta é o de antes de pegar a terra (desmontagem: `uVar8`).
        if !self.passo(n, d, t_ref) {
            return;
        }
        let r = self.sk(64);
        if (self.colonia().saude as i32) < r {
            // No vermelho o original testa a célula ANTIGA (ninho.md §11).
            let c = if self.cor == Cor::Preta { n } else { (x, y) };
            comer_se_houver(self.e, self.cor, c.0, c.1);
        }
        if self.sk(4) == 0 {
            comum::corrigir_saida(self.e, self.tab, self.cor, n.0, n.1);
        }
        if self.cor == Cor::Vermelha && self.mapa(n.0, n.1) == PASSAGEM {
            self.atravessar_passagem(n);
        }
    }

    /// Passagem 0x14 (ninho.md §7.7): a vermelha some do ninho vermelho e aparece no preto com
    /// T = 0x90 + `Sk(8)` (operária) ou 0xB0 + `Sk(8)`, M = 3, S = 0.
    fn atravessar_passagem(&mut self, n: (i32, i32)) {
        let t = self.t();
        self.eu_mut().tipo = 0;
        self.definir_vida(n.0, n.1, 0);
        let base = if (t & 0x7F) < 0x30 { 0x90 } else { 0xB0 };
        let novo = base + self.sk(8) as u8;
        let f = Formiga { x: n.0 as u8, y: n.1 as u8, modo: 3, tipo: novo, s: 0 };
        self.e.adicionar_formiga(Lista::B, f);
        // O original grava `_LifeB` mesmo com a lista B cheia.
        self.e.ninho_b.vida.definir(n.0, n.1, novo);
    }

    /// `_DoDigOutB/R` (`1028:4eb0`/`722a`) (M = 2, 5, 7, 11, 12, 15, 16; e 6 no vermelho).
    fn sair_cavando(&mut self) {
        let (x, y) = (self.x0, self.y0);
        let dir = (self.t0 & 7) as i32;
        let e = self.direcao_de_saida(x, y, dir);
        let d = if e >= 1 { e - 1 } else { self.giro_aleatorio(dir) };
        let t_ref = self.virar_para(d);
        let Some(Some(n)) = self.vizinha_ou_saida(d) else { return };
        let v = self.mapa(n.0, n.1);
        if v >= LIMITE_BLOQUEIO {
            let s = self.saida(x, y);
            let novo = match self.cor {
                // Sem teste: 0 vira 0xFF (ninho.md §7.2). [hipótese: defeito do original]
                Cor::Preta => Some(s.wrapping_sub(1)),
                Cor::Vermelha => (s != 0).then(|| s - 1),
            };
            if let Some(novo) = novo {
                self.e.ninho_mut(self.cor).saida.definir(x, y, novo);
            }
            let g = grupo(self.t0);
            if g == 5 || g == 9 {
                let f = self.eu_mut();
                f.tipo = f.tipo.wrapping_sub(0x18);
                f.modo = 4;
            }
            if g == 2 || g == 6 {
                self.eu_mut().modo = 4;
            }
            return;
        }
        if e_terra(v) {
            return;
        }
        if !self.passo(n, d, t_ref) {
            return;
        }
        let r = self.sk(64);
        if (self.colonia().saude as i32) < r {
            let c = if self.cor == Cor::Preta { n } else { (x, y) };
            comer_se_houver(self.e, self.cor, c.0, c.1);
        }
    }

    // --- luta (ninho.md §4.4) ---

    /// `_DoNestFightB/R` (`1028:3a54`/`6072`): animação e, com 1/16 de chance, o fim da luta.
    fn nuvem_de_luta(&mut self) {
        let (x, y) = (self.x0, self.y0);
        let r = self.s(7) as u8;
        let t = (self.t() & 0xF8).wrapping_add(r);
        self.eu_mut().tipo = t;
        self.definir_vida(x, y, t);
        if self.sk(16) != 0 {
            // Balão de luta: só desenho, sem sorteio (lacuna no port).
            return;
        }
        let w = self.eu().s;
        self.definir_vida(x, y, w);
        self.eu_mut().tipo = w;
        if (w & 0x78) == 0x60 {
            // Cabeça de rainha: recria a cauda atrás dela, sem mexer em `_BlkQueens`/`_RedQueens`.
            let (dx, dy) = self.tab.d8(((w & 7) ^ 4) as i32);
            let f = Formiga { x: (x + dx) as u8, y: (y + dy) as u8, modo: 9, tipo: w.wrapping_add(8), s: 0 };
            self.e.adicionar_formiga(lista_do_ninho(self.cor), f);
        }
        let m = match self.cor {
            Cor::Preta if vermelha(w) => 7,
            Cor::Preta => novo_modo_pela_cor(self.e, self.tab, grupo(w), w),
            Cor::Vermelha if !vermelha(w) => 7,
            Cor::Vermelha => self.tab.vitoria_vermelha[grupo(w) as usize],
        };
        self.eu_mut().modo = m;
    }

    // --- descanso, pré-voo, afogamento (ninho.md §8.3, §10) ---

    /// DESCANSO (M = 13; `_DoRestB` embutido).
    fn descansar(&mut self) {
        let (x, y) = (self.x0, self.y0);
        if self.luta((x, y), self.t0) {
            return;
        }
        let t = self.t();
        self.definir_vida(x, y, t);
        if self.s(20) != 0 {
            // Balão de descanso: só desenho.
            return;
        }
        let m = novo_modo_pela_cor(self.e, self.tab, grupo(t), t);
        self.eu_mut().modo = m;
    }

    /// PRÉ-VOO (M = 14).
    fn pre_voo(&mut self, g: u8) {
        if self.sk(32) == 0 {
            let m = self.novo_modo(g);
            self.eu_mut().modo = m;
        }
        if !self.luta((self.x0, self.y0), self.t0) {
            self.tentar((self.t0 & 7) as i32);
        }
        if self.colonia().revoada > 100 {
            self.eu_mut().modo = 15;
        }
    }

    /// AFOGANDO (M = 17; `_DoDrownB` embutido).
    fn afogando(&mut self) {
        let (x, y) = (self.x0, self.y0);
        if self.mapa(x, y) <= COMIDA_CHEIA {
            let m = self.novo_modo(grupo(self.t0));
            self.eu_mut().modo = m;
            return;
        }
        let r = self.s(3) as u8;
        let t = (self.t0.wrapping_add(r).wrapping_sub(1) & 7) | (self.t0 & 0xF8);
        self.eu_mut().tipo = t;
        self.definir_vida(x, y, t);
        if self.s(100) != 0 {
            return;
        }
        self.definir_vida(x, y, 0);
        self.eu_mut().tipo = 0;
        let morta = if vermelha(self.t0) { &mut self.e.vermelha } else { &mut self.e.preta };
        morta.mortes_naturais = morta.mortes_naturais.wrapping_add(1);
    }

    // --- ovos e rainha (ciclo-de-vida.md §2, §3, §5) ---

    /// `_SimEggB` (`1028:3ca0`) / `_SimEggR` (`1028:62a6`).
    fn simular_ovo(&mut self) {
        let mut t = self.t();
        let ciclo = self.e.partida.ciclo as u16;
        let mascara = match self.cor {
            Cor::Preta if self.e.preta.populacao < 3 => 0x1F,
            Cor::Vermelha if self.e.vermelha.populacao == 1 => 0x1F,
            _ => 0x7F,
        };
        if ciclo & mascara == 0 {
            t = t.wrapping_add(1);
            if t & 0xF == 8 {
                match self.cor {
                    Cor::Preta => {
                        let morre = self.e.controles.modo_automatico == 0 && {
                            let sorteio = self.e.s.sg_rand(255);
                            ((self.e.controles.tarefas[2] >> 7) as i32) < sorteio
                        };
                        if morre {
                            t = 0;
                            self.e.partida.ovos_perdidos_b = self.e.partida.ovos_perdidos_b.wrapping_add(1);
                        } else {
                            let c = self.e.controles.proxima_casta;
                            t = (c as u8).wrapping_mul(8).wrapping_add(2);
                            let m = if c == 2 { 1 } else { novo_modo_preto(self.e, self.tab, c as u8) };
                            self.eu_mut().modo = m;
                        }
                    }
                    Cor::Vermelha => {
                        let k = self.sk(8) as usize;
                        let linha_c = (self.e.vermelha.modo_estrategico as i32).rem_euclid(7) as i16;
                        let c = comum::linha_por_modo(&self.tab.castas_vermelhas, linha_c)[k];
                        t = c.wrapping_mul(8).wrapping_add(0x82);
                        let m = novo_modo_vermelho(self.e, self.tab, c);
                        self.eu_mut().modo = m;
                    }
                }
            }
        }
        // Balão de ovo: só desenho, sem sorteio.
        let (x, y) = (self.x0, self.y0);
        self.definir_vida(x, y, t);
        let f = self.eu_mut();
        f.tipo = t;
        f.s = 0;
    }

    /// `_SimQueenB` (`1028:3dc2`) / `_SimQueenR` (`1028:6386`): cabeça (grupo 12) e cauda (13).
    fn simular_rainha(&mut self, g: u8) {
        let (x, y) = (self.x0, self.y0);
        if g == 12 {
            if self.sk(64) == 0 {
                if self.colonia().saude == 0 {
                    self.eu_mut().tipo = 0;
                    self.definir_vida(x, y, 0);
                    let msg = if self.cor == Cor::Preta { 10015 } else { 10016 };
                    self.e.eventos.push(Evento::Mensagem(msg));
                    return;
                }
                if self.andar_rainha() {
                    return;
                }
            }
            let mut t = self.t();
            let (dx, dy) = self.tab.d8(((t & 7) ^ 4) as i32);
            if !self.metade_existe(x + dx, y + dy, t as u16 + 8) {
                t = 0;
                self.eu_mut().tipo = 0;
                let c = self.colonia_mut();
                c.rainhas = c.rainhas.wrapping_sub(1);
            }
            self.definir_vida(x, y, t);
            // Balão de rainha: só desenho.
        } else if g == 13 {
            let t = self.t();
            self.definir_vida(x, y, t);
            if self.colonia().rainhas > 0 {
                let (dx, dy) = self.tab.d8((t & 7) as i32);
                if !self.metade_existe(x + dx, y + dy, (t as u16).wrapping_sub(8)) {
                    let c = self.colonia_mut();
                    c.rainhas = c.rainhas.wrapping_sub(1);
                    self.eu_mut().tipo = 0;
                    self.definir_vida(x, y, 0);
                    return;
                }
            }
            let (dx, dy) = self.tab.d8(((self.t0 & 7) ^ 4) as i32);
            let (ex, ey) = (x + dx, y + dy);
            if !((0..LADO).contains(&ex) && (1..LADO).contains(&ey)) {
                return;
            }
            self.colonia_mut().ultimo_ovo = (ex as i16, ey as i16);
            if self.e.partida.ciclo & 0xF != 0 {
                return;
            }
            let r = self.sk(128);
            if r > self.colonia().saude as i32 {
                return;
            }
            let estagio = if self.cor == Cor::Preta { 1 } else { 0x81 };
            comum::por_ovo(self.e, self.tab, self.cor, ex, ey, estagio);
            // `_DecEatB/R`.
            comum::gastar_reserva(self.e, self.cor);
            if self.cor == Cor::Preta {
                self.e.partida.ovos_postos_b = self.e.partida.ovos_postos_b.wrapping_add(1);
            }
        }
    }

    /// A outra metade da rainha existe em (x, y): `Vida` mostra o tipo esperado ou a lista tem uma
    /// entrada com ele ali.
    fn metade_existe(&self, x: i32, y: i32, t: u16) -> bool {
        let vida = self.e.ninho(self.cor).vida.em(x, y);
        if vida.map(u16::from) == Some(t) {
            return true;
        }
        t <= 0xFF && self.procurar(x, y, t as u8).is_some()
    }

    /// `_QueenMoveB/R` (`1028:4154`/`6606`) com `_GetBestDir` (`1028:405e`): a cabeça anda rumo ao
    /// centro de massa da escavação e puxa a cauda para a posição antiga.
    fn andar_rainha(&mut self) -> bool {
        let (x, y) = (self.x0, self.y0);
        let alvo = (self.colonia().centro_x as i32, self.colonia().centro_y as i32);
        let mut d = self.melhor_direcao(x, y, alvo);
        if d < 0 {
            if d == -1 {
                return false;
            }
            d = self.sk(8);
        }
        if !(y > 2 || (3..=5).contains(&d)) {
            return false;
        }
        if !self.mover(d) {
            return false;
        }
        let (dx, dy) = self.tab.d8(((self.t0 & 7) ^ 4) as i32);
        let cauda = (x + dx, y + dy);
        self.definir_vida(cauda.0, cauda.1, 0);
        // Cauda = cabeça + 8 com a direção antiga (0x68 + d no preto, 0xE8 + d no vermelho).
        let base = if self.cor == Cor::Preta { 0x68 } else { 0xE8 };
        if let Some(j) = self.procurar(cauda.0, cauda.1, base + (self.t0 & 7)) {
            let nova = base + d as u8;
            let f = self.formiga_mut(j);
            if f.tipo != 0 {
                f.x = x as u8;
                f.y = y as u8;
                f.tipo = nova;
                self.definir_vida(x, y, nova);
            }
        }
        true
    }

    /// `_GetBestDir(plano, x, y, alvo)` no ninho ([`comum::melhor_direcao`]): direção que aproxima do
    /// alvo, preferindo célula livre; −1 se já está no alvo; −2 se nenhuma vizinha aproxima.
    fn melhor_direcao(&self, x: i32, y: i32, alvo: (i32, i32)) -> i32 {
        comum::melhor_direcao(self.e, self.tab, plano_do_ninho(self.cor), x, y, alvo.0, alvo.1)
    }

    // --- invasoras (ninho.md §3.2, §9) ---

    /// Ninho preto, formiga vermelha.
    fn invasora_no_preto(&mut self) {
        self.contar_modo(Cor::Vermelha);
        if self.t0 >= 0xF0 {
            self.nuvem_de_luta();
            return;
        }
        let (x, y) = (self.x0, self.y0);
        let v = self.vida(x, y);
        if e_amarela(v) && self.e.amarela.cor == 0 {
            amarela::lutar_com_amarela(self.e, self.ta, plano_do_ninho(self.cor), self.i as i32);
            return;
        }
        if v != 0 && v < 0x68 && self.encontro_na_celula(v, 0x60, 7) {
            return;
        }
        if self.m0 == 7 {
            self.atacar_o_ninho();
            return;
        }
        self.sair_como_invasora();
    }

    /// Ninho vermelho, formiga preta.
    fn invasora_no_vermelho(&mut self) {
        self.contar_modo(Cor::Preta);
        let (x, y) = (self.x0, self.y0);
        if self.t0 < 8 {
            // Ovo preto no ninho vermelho: some, sem contagem.
            self.eu_mut().tipo = 0;
            self.definir_vida(x, y, 0);
            return;
        }
        if self.t0 >= 0x70 {
            self.nuvem_de_luta();
            return;
        }
        let v = self.vida(x, y);
        if v > 0x80 && v < 0xE8 && self.encontro_na_celula(v, 0xE0, 0x87) {
            return;
        }
        match self.m0 {
            6 if self.e.amarela.plano == 3 => self.ficar_no_vermelho(),
            7 => self.atacar_o_ninho(),
            _ => self.sair_como_invasora(),
        }
    }

    /// Invasora na mesma célula de uma dona (ou ovo) cujo T está em `Vida`: luta (a invasora some)
    /// ou roubo do ovo. `rainha` = início da faixa da cabeça da rainha; `ovo_max` = maior T de ovo.
    /// Devolve se tratou (a dona foi achada na lista).
    fn encontro_na_celula(&mut self, v: u8, rainha: u8, ovo_max: u8) -> bool {
        let (x, y) = (self.x0, self.y0);
        let Some(j) = self.procurar(x, y, v) else { return false };
        if v >= rainha {
            // Cabeça da rainha: apagada antes da luta, que recebe 0 como ocupante. [hipótese: defeito]
            let f = self.formiga_mut(j);
            f.tipo = 0;
            let (jx, jy) = (f.x as i32, f.y as i32);
            self.definir_vida(jx, jy, 0);
        }
        if v > ovo_max {
            let ocupante = self.e.ninho(self.cor).lista.vagas[j].tipo;
            let w = vencedor_da_luta(self.e, self.tab, ocupante, self.t0);
            self.eu_mut().tipo = 0;
            let nuvem = 0x70u8.wrapping_add(w & 0x80);
            let f = self.formiga_mut(j);
            f.s = w;
            f.tipo = nuvem;
            f.modo = 10;
            self.definir_vida(x, y, nuvem);
            return true;
        }
        // Roubo do ovo: a invasora passa a "com comida" e vai sair.
        let f = self.eu_mut();
        f.modo = 3;
        f.tipo |= 8;
        let t = f.tipo;
        self.definir_vida(x, y, t);
        self.formiga_mut(j).tipo = 0;
        if self.cor == Cor::Preta {
            self.e.partida.ovos_perdidos_b = self.e.partida.ovos_perdidos_b.wrapping_add(1);
        }
        true
    }

    /// `_RaidInB/R` (`1028:3524`/`5b2a`): invasora com M = 7.
    fn atacar_o_ninho(&mut self) {
        let (x, y) = (self.x0, self.y0);
        if e_comida(self.mapa(x, y)) {
            comum::tirar_comida_do_ninho(self.e, self.cor, x, y);
            let f = self.eu_mut();
            f.modo = 3;
            f.tipo |= 8;
            let t = f.tipo;
            self.definir_vida(x, y, t);
            return;
        }
        let r = self.s(3) as u8;
        if self.mover((self.t0.wrapping_add(r).wrapping_sub(2) & 7) as i32) {
            return;
        }
        let mut d = self.direcao_de_entrada(x, y, (self.t0 & 7) as i32);
        if d < 0 {
            d = self.s(8);
        }
        if !self.mover(d) {
            self.eu_mut().modo = 1;
            let t = self.t();
            self.definir_vida(x, y, t);
        }
    }

    /// SAIR_DO_NINHO (`_RaidOutB/R` embutido).
    fn sair_como_invasora(&mut self) {
        let (x, y) = (self.x0, self.y0);
        let e = self.direcao_de_saida(x, y, 8);
        let d = if e != 0 { e - 1 } else { self.sk(8) };
        if !self.mover(d) {
            let r = self.sk(8);
            if !self.mover(r) {
                let t = self.t();
                self.definir_vida(x, y, t);
            }
        }
    }

    /// `_StayInR` (`1028:5c16`): preta em Defense no ninho vermelho, com a amarela lá.
    fn ficar_no_vermelho(&mut self) {
        let (x, y) = (self.x0, self.y0);
        if e_comida(self.mapa(x, y)) {
            comum::tirar_comida_do_ninho(self.e, Cor::Vermelha, x, y);
            let f = self.eu_mut();
            f.modo = 3;
            f.tipo |= 8;
        } else {
            let r = self.s(3) as u8;
            let d = self.t0.wrapping_add(r).wrapping_sub(2) & 7;
            let f = self.eu_mut();
            f.tipo = (f.tipo & 0xF8) | d;
            if self.mover(d as i32) {
                return;
            }
            let mut d2 = self.direcao_de_entrada(x, y, (self.t0 & 7) as i32);
            if d2 < 0 {
                d2 = self.s(8);
            }
            if self.mover(d2) {
                return;
            }
        }
        let t = self.t();
        self.definir_vida(x, y, t);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn tabelas_embutidas_batem_com_a_spec() {
        let t = TabelasNinho::padrao();
        assert_eq!(t.modo_por_grupo, [8, 1, 0, 3, 14, 5, 0, 3, 14, 5, 0, 0, 9, 9, 10, 0]);
        assert_eq!(t.vitoria_vermelha, [1, 1, 1, 3, 0, 5, 2, 3, 0, 5, 0, 0, 9, 9, 10, 0]);
        assert_eq!(t.chance_vitoria[1], [8, 5, 9, 4]);
        assert_eq!(t.castas_vermelhas[1], [4, 4, 8, 8, 2, 2, 2, 6]);
        assert_eq!(t.modos_operaria[1], [4, 1, 1, 1, 4, 2, 2, 2]);
    }
}
