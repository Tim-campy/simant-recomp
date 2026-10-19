//! Formigas na superfície (lista A), comida, estratégia e líder vermelha.
//! Especificação: docs/spec/regras/superficie.md (com feromonios.md, formigas.md, rng.md).
//!
//! - [`simular_superficie`] é o `_DoAntSimA`: a lista A de trás para frente, com o `S256()` de cada
//!   vaga e o despacho por modo (seção 3 da spec). Os sorteios acontecem exatamente na ordem e nas
//!   condições da spec; cada função abaixo diz a sua ordem.
//! - [`adicionar_comida`] (`_AddFood`), [`escolher_estrategia`] (`_GetStrategy` + `_GstrR` +
//!   `_SetCasteProd` + `_SetModeProd`), [`criar_lider_vermelha`] (`_MakeRedInitiator`).
//! - Recrutamento (`_Recruit`, `_UnRecruit`, `_RecruitRed`, `_UnRecruitRed`) é público para a
//!   formiga amarela (comandos 1–5).
//!
//! Tabelas e constantes: `data/superficie.json` ([`TabelasSuperficie`], extraído por
//! `tools/extrair_superficie.py`) e `data/feromonios.json` (direções e `_TurnTab`), guardadas em
//! `RegrasOriginais` (com mods); as funções públicas recebem as tabelas explicitamente. As rotinas que
//! o original compartilha com outras partes (`_GetWinner`, `_GetNewMode`, `_DeadAntHere`,
//! `_DigTileB/R`...) estão em `rules/comum.rs`; as da amarela (`_YellowFight`, `_DoTroph`), em
//! `rules/amarela.rs`.
//!
//! Os mapas de cheiro são lidos e gravados pelas rotinas de `rules/feromonios.rs` sobre uma cópia
//! (`feromonios::Feromonios`) feita no começo do laço e devolvida ao `Estado` no fim; nada dentro do
//! laço lê os mapas do `Estado` diretamente.

use super::baloes::{self, TipoBalao};
use std::sync::OnceLock;

use serde::Deserialize;

use super::amarela::{self, TabelasAmarela};
use super::comum::{
    self, cor_do_tipo as cor, e_amarela, grupo, operaria_ou_soldado, TabelaLuta, TabelaModos,
};
use super::estado::{indice_cor, Estado, Feromonios as MapasDoEstado, Grade};
use super::feromonios::{self as fer, MapaCheiro, RegrasFeromonio, TabelasFeromonio};
use super::geracao::{Cor, TabelasGeracao};
use super::tick::{RegrasOriginais, RegrasSuperficie};
use crate::formats::save::{Formiga, Lista};

pub use super::comum::{cadaver_aqui, desrecrutar, recrutar};

/// Arquivo de dados com as tabelas desta parte.
pub const ARQUIVO_TABELAS: &str = "superficie.json";

// Valores de tile e de mapa de vida (docs/spec/regras/tiles-valores.md, superficie.md).
const COMIDA_ULTIMA_QUINTAL: u8 = 0x48;
const NUVEM_DE_LUTA: u8 = 0x70;
/// Modos (formigas.md §5).
const M_PASSEIO: u8 = 0;
const M_FORRAGEAR: u8 = 2;
const M_COM_COMIDA: u8 = 3;
const M_DEFESA: u8 = 6;
const M_LUTA: u8 = 10;
const M_ALARME: u8 = 11;
const M_DESCANSO: u8 = 13;
const M_DECOLAR: u8 = 15;
const M_VOO: u8 = 16;
const M_LIDER: u8 = 19;
/// Capacidade dos ninhos (`_GoInNest`: compacta e desiste com ≥ 500).
const CAPACIDADE_NINHO: usize = 500;

// ---------------------------------------------------------------------------------------------
// Tabelas
// ---------------------------------------------------------------------------------------------

/// Constantes (bloco `regras` de `superficie.json`; origem de cada uma em `_origem`).
#[derive(Clone, Debug, Deserialize)]
pub struct RegrasSuperficieDados {
    pub s_ao_pegar_comida: u8,
    pub alarme_de_luta: u8,
    pub cheiro_defender_ninho: u8,
    pub cheiro_decolar: u8,
    pub revoada_pre_voo: i16,
    pub limite_enxame: i16,
    pub populacao_lider: i16,
    pub tipo_lider: u8,
    pub raio_aranha_perseguida: i32,
    pub saude_desespero: i16,
    pub saude_fome: i16,
    pub saude_pouca: i16,
    pub populacao_grande: i16,
    pub revoada_vermelha_reprodutores: i16,
    pub revoada_vermelha_contagem: i16,
    pub malvadeza_minima_populacao: i16,
    pub malvadeza_base: i16,
    pub malvadeza_sorteio: u16,
    pub musica_ataque_vermelho: u16,
    /// `_EditMessage(_AdviceStrs[5], 0x78, 0)`: "The red ants are attacking!".
    pub mensagem_ataque_vermelho: super::estado::MensagemFaixa,
}

/// Tabelas de `data/comum.json` que a parte da superfície usa (chave aqui, chave em `comum.json`).
pub const USOS_COMUNS: &[comum::UsoComum] = &[
    ("grupo_sem_carga", "grupo_sem_carga"),
    ("modos_operaria", "modos_operaria"),
    ("modos_soldado", "modos_soldado"),
    ("modo_por_grupo", "modo_por_grupo"),
    ("classe_luta", "classe_luta"),
    ("matriz_luta", "matriz_luta"),
];

/// Conteúdo de `data/superficie.json`, com as tabelas comuns de `data/comum.json` ([`USOS_COMUNS`]).
#[derive(Clone, Debug, Deserialize)]
pub struct TabelasSuperficie {
    /// `_IsCarryCaste` (`8:0074`): 1 se o grupo carrega algo.
    pub carrega_carga: [i16; 16],
    /// `_UnCarryCaste` (`8:0094`): grupo depois de largar a carga.
    pub grupo_sem_carga: [i8; 16],
    /// `_CasteTabB` (`8:89ae`): grupo do próximo ovo por escolha de `_SetCasteProd`.
    pub casta_por_escolha: [i16; 4],
    /// `_ModeTabB` (`8:89b6`): modo das próximas operárias por escolha de `_SetModeProd`.
    pub modo_por_tarefa: [i16; 3],
    /// `_ModeTabWB` (`8:89e6`): [modo estratégico][SRand8] → modo da operária.
    pub modos_operaria: [[u8; 8]; 6],
    /// `_ModeTabSB` (`8:8a16`): idem, soldado.
    pub modos_soldado: [[u8; 8]; 6],
    /// `_CasteModeTabB` (`8:8a46`): modo dos grupos que não sorteiam.
    pub modo_por_grupo: [u8; 16],
    /// Classe de luta por grupo (`10:22c6`).
    pub classe_luta: [u8; 16],
    /// Chance em 10 de a formiga da célula vencer (`10:22d6`), [classe a][classe b].
    pub matriz_luta: [[u8; 4]; 4],
    pub regras: RegrasSuperficieDados,
    #[serde(default)]
    pub sons: SonsSuperficie,
}

/// Ids dos efeitos sonoros (objetos tipo 5 do banco SOUND, `_myBeginSound`) pedidos por esta parte
/// (docs/spec/regras/som.md). Seção opcional no JSON: um id ausente não toca (mods anteriores à API 0.8).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SonsSuperficie {
    /// `_AddFood(n, 1)`: comida nova no quintal (0x20).
    pub comida_nova: Option<u16>,
}

impl TabelasSuperficie {
    pub fn de_json(texto: &str) -> Result<TabelasSuperficie, String> {
        comum::parte_do_texto::<TabelasSuperficie>(ARQUIVO_TABELAS, texto, USOS_COMUNS)?.validar()
    }

    fn validar(self) -> Result<TabelasSuperficie, String> {
        let t = self;
        // As tabelas comuns já vêm de comum.json; uma cópia antiga no superficie.json também passa aqui.
        if t.classe_luta.iter().any(|&c| c > 3) {
            return Err(format!("{ARQUIVO_TABELAS}: classe_luta fora de 0–3"));
        }
        if t.grupo_sem_carga.iter().any(|&g| !(0..16).contains(&g)) {
            return Err(format!("{ARQUIVO_TABELAS}: grupo_sem_carga fora de 0–15"));
        }
        Ok(t)
    }

    /// Lê `superficie.json` e `comum.json` de `data/` (com mods).
    ///
    /// Antes da unificação lia com `Recursos::arquivo`, que procura na pasta do jogo e em
    /// `arquivos/` dos mods, e nunca achava o arquivo de `data/` [corrigido].
    pub fn carregar(r: &crate::recursos::Recursos) -> Result<TabelasSuperficie, String> {
        comum::parte_dos_recursos::<TabelasSuperficie>(r, ARQUIVO_TABELAS, USOS_COMUNS)?.validar()
    }

    /// Cópia embutida de `data/superficie.json` (sem mods), para `RegrasOriginais::default` e testes.
    pub fn padrao() -> &'static TabelasSuperficie {
        static T: OnceLock<TabelasSuperficie> = OnceLock::new();
        T.get_or_init(|| TabelasSuperficie::de_json(include_str!("../../../data/superficie.json")).expect("superficie.json"))
    }

    /// Tabelas de `_GetNewMode`.
    pub fn modos(&self) -> TabelaModos<'_> {
        TabelaModos { operaria: &self.modos_operaria, soldado: &self.modos_soldado, por_grupo: &self.modo_por_grupo }
    }

    /// Tabelas de `_GetWinner`.
    pub fn luta(&self) -> TabelaLuta<'_> {
        TabelaLuta { classe: &self.classe_luta, chance: &self.matriz_luta }
    }
}

/// Cópias embutidas de `superficie.json` e `feromonios.json` (sem mods).
pub fn tabelas_padrao() -> (&'static TabelasSuperficie, &'static TabelasFeromonio) {
    (TabelasSuperficie::padrao(), TabelasFeromonio::padrao())
}

// ---------------------------------------------------------------------------------------------
// A trait do tick
// ---------------------------------------------------------------------------------------------

impl RegrasSuperficie for RegrasOriginais {
    fn simular_superficie(&mut self, e: &mut Estado) {
        simular_superficie(e, &self.superficie, &self.feromonios, &self.amarela);
    }

    fn adicionar_comida(&mut self, e: &mut Estado, quantidade: i32, som: bool) {
        // Sem as tabelas do banco (testes), o seno é o calculado, que coincide com o objeto 9/1000
        // (`inimigos::seno_calculado`). O som (0x20) vem antes, como no original.
        if som {
            e.pedir_som(self.superficie.sons.comida_nova);
        }
        adicionar_comida(e, self.geracao.as_ref(), quantidade, som);
    }

    fn escolher_estrategia(&mut self, e: &mut Estado) {
        escolher_estrategia(e, &self.superficie);
    }

    fn criar_lider_vermelha(&mut self, e: &mut Estado) {
        criar_lider_vermelha(e, &self.superficie);
    }

    fn fase_de_cheiro(&mut self, e: &mut Estado, fase: fer::FaseCheiro) {
        rodar_fase_de_cheiro(e, fase, &self.feromonios.regras);
    }
}

// ---------------------------------------------------------------------------------------------
// Peças simples: tipo, cor, mapas
// ---------------------------------------------------------------------------------------------

fn base(t: u8) -> u8 {
    t & 0xF8
}

fn dir(t: u8) -> u8 {
    t & 7
}

/// Byte de cor da amarela (`_MeColor`, 0 ou 0x80).
fn cor_amarela(e: &Estado) -> u8 {
    e.amarela.cor as u8
}

fn tile(e: &Estado, x: i32, y: i32) -> u8 {
    // Fora do mapa o original leria memória vizinha; o port trata como obstáculo. Os destinos de
    // passo nunca saem do mapa (as bordas são tratadas antes), então isso não acontece em jogo.
    e.superficie.mapa.em(x, y).unwrap_or(0xFF)
}

fn vida(e: &Estado, x: i32, y: i32) -> u8 {
    e.superficie.vida.em(x, y).unwrap_or(0)
}

fn definir_vida(e: &mut Estado, x: i32, y: i32, v: u8) {
    e.superficie.vida.definir(x, y, v);
}

/// Teste de buraco (seção 2.1): `_IsValidA` e o tile de buraco do conjunto atual
/// ([`comum::e_buraco_superficie`]).
fn e_buraco(e: &Estado, x: i32, y: i32) -> bool {
    comum::e_buraco_superficie(e, x, y)
}

/// `_IsItFood` (`1028:2d1a`).
fn e_comida(e: &Estado, v: u8) -> bool {
    comum::e_comida_superficie(comum::em_casa(e), v)
}

/// Destino acima de `_Barrier` (comparação com sinal, tile sem sinal).
fn e_barreira(e: &Estado, v: u8) -> bool {
    v as i32 > e.ambiente.barreira as i32
}

/// `_GetDis` (`1020:1122`, [`comum::get_dis`]), truncado a 16 bits com sinal como nos chamadores
/// desta parte, que guardam o resultado num `int`.
fn distancia(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    comum::get_dis(x1, y1, x2, y2) as i16 as i32
}

/// `_GetDir` (`1020:10cc`, [`comum::get_dir`]): 1 + direção de (x1, y1) para (x2, y2); 0 se coincidem.
fn direcao_para(x1: i32, y1: i32, x2: i32, y2: i32) -> u8 {
    comum::get_dir(x1, y1, x2, y2) as u8
}

/// `_Bounce` ([`comum::quicar`]).
fn quicar(e: &mut Estado, x: i32, y: i32) -> Option<u8> {
    comum::quicar(&mut e.s, x, y)
}

use comum::achar_na_lista_a;

// ---------------------------------------------------------------------------------------------
// Modo novo, luta, cadáver, comida
// ---------------------------------------------------------------------------------------------

/// `_GetNewMode(g, T)` (`1030:0910`, superficie.md 9.3; [`comum::novo_modo`]). Sorteia `SRand8` só
/// para grupo 2 ou 6 e (vermelha, ou preta com `_ModeAuto` = 1).
pub fn modo_novo(e: &mut Estado, t: &TabelasSuperficie, g: u8, tipo: u8) -> u8 {
    comum::novo_modo(e, t.modos(), g, tipo)
}

/// `_GetWinner(Ta, Tb)` (`1028:26f4`, 6.2; [`comum::vencedor_da_luta`]): `a` é a formiga da célula,
/// `b` a que chega.
pub fn vencedor_da_luta(e: &mut Estado, t: &TabelasSuperficie, ta: u8, tb: u8) -> u8 {
    comum::vencedor_da_luta(e, t.luta(), ta, tb)
}

/// `_PickupFoodA(x, y)` (`1020:0d18`, 8.1).
fn pegar_comida(e: &mut Estado, x: i32, y: i32) {
    comum::pegar_comida_superficie(e, x, y);
}

/// `_AddFood(n, som)` (`1030:6a58`, 8.3 e geracao-mundo.md 3.E). `n < 0` é o caso do novo jogo
/// (200 unidades em torno de (64, 8…55)). Ordem: centro (`S128`, `S64`, ou `S1(48)`), raio
/// `S8()`, e por unidade `S256()` e `S1(raio)`.
///
/// `som = true` toca o efeito 0x20: o pedido (`Evento::Som`) é feito por `RegrasSuperficie::adicionar_comida`,
/// que tem as tabelas da superfície.
pub fn adicionar_comida(e: &mut Estado, g: Option<&TabelasGeracao>, quantidade: i32, _som: bool) {
    let seno = super::inimigos::Seno(g);
    let (cx, cy, n) = if quantidade < 0 {
        (0x40, e.s.rand1(0x30) + 8, 200)
    } else {
        let cx = e.s.mascara(128);
        let cy = e.s.mascara(64);
        (cx, cy, quantidade)
    };
    e.ambiente.ultima_comida = (cx as i16, cy as i16);
    let raio = e.s.mascara(8) as u16 + 5;
    for _ in 0..n {
        let a = e.s.mascara(256);
        let r = e.s.rand1(raio);
        let x = cx + seno.cos(a) * r / 0x7FFF;
        let y = cy + seno.sen(a) * r / 0x7FFF;
        if !(0..128).contains(&x) || !(0..64).contains(&y) || vida(e, x, y) != 0 {
            continue;
        }
        let v = tile(e, x, y);
        let novo = if e.ambiente.terreno == 0 {
            match v {
                0..=0x17 => Some(COMIDA_ULTIMA_QUINTAL),
                0x48..=0x4A => Some(v + 1),
                _ => None,
            }
        } else {
            match v {
                0..=3 => Some((v + 6) * 4),
                4..=0x17 => Some((v.wrapping_sub(8) & 0xFC).wrapping_add(0x18)),
                0x18..=0x27 if v & 3 < 3 => Some(v + 1),
                _ => None,
            }
        };
        if let Some(nv) = novo {
            e.superficie.mapa.definir(x, y, nv);
            e.ambiente.comida_superficie += 1;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Ninhos: entrada (`_GoInNest`; a escavação `_DigTileB/R` é [`comum::cavar`])
// ---------------------------------------------------------------------------------------------

/// `_GoInNest(x, y, i)` (`1028:257a`, seção 7): o ninho é escolhido pela coluna do buraco.
fn entrar_no_ninho(e: &mut Estado, f: &TabelasFeromonio, x: i32, y: i32, i: usize) {
    let c = if x < 64 { Cor::Preta } else { Cor::Vermelha };
    let area = if c == Cor::Preta { Lista::B } else { Lista::R };
    if e.lista(area).usadas() >= CAPACIDADE_NINHO {
        e.lista_mut(area).compactar();
    }
    if e.lista(area).usadas() >= CAPACIDADE_NINHO {
        return;
    }
    let a = e.superficie.lista.vagas[i];
    e.adicionar_formiga(area, Formiga { x: y as u8, y: 1, modo: a.modo, tipo: base(a.tipo) + 4, s: a.s });
    if e.colonia(c).buracos.get(y as usize).is_some_and(|&b| b != 0) {
        comum::cavar(e, f, c, y, 1);
    }
    e.superficie.lista.vagas[i].tipo = 0;
    definir_vida(e, x, y, 0);
}

// ---------------------------------------------------------------------------------------------
// O laço da superfície
// ---------------------------------------------------------------------------------------------

/// Quando a formiga encontra a amarela da mesma cor, dá comida a ela?
#[derive(Clone, Copy, PartialEq, Eq)]
enum Trofalaxia {
    Nunca,
    /// `_MeWantFood == 1`.
    SeIgualAUm,
    /// `_MeWantFood ≠ 0` (modo 6).
    SeDiferenteDeZero,
}

/// Variações do passo padrão por modo (tabela 2.4).
#[derive(Clone, Copy)]
struct Passo {
    pega_comida: bool,
    cheiro_de_ninho: bool,
    gasta_trilha: bool,
    vira_forrageira: bool,
    trofalaxia: Trofalaxia,
    modo_novo_na_mesma_cor: bool,
    /// Modo 2: depois do giro na barreira, `S16() == 0` → modo novo.
    modo_novo_na_barreira: bool,
    /// Modo 6: na barreira, `T = base | S8()` sem `_TurnTab`.
    barreira_sem_tabela: bool,
}

const PASSO_MUDO: Passo = Passo {
    pega_comida: false,
    cheiro_de_ninho: false,
    gasta_trilha: false,
    vira_forrageira: false,
    trofalaxia: Trofalaxia::Nunca,
    modo_novo_na_mesma_cor: false,
    modo_novo_na_barreira: false,
    barreira_sem_tabela: false,
};

struct Sim<'a> {
    e: &'a mut Estado,
    t: &'a TabelasSuperficie,
    f: &'a TabelasFeromonio,
    /// Tabelas da amarela, para `_YellowFight` e `_DoTroph`.
    ta: &'a TabelasAmarela,
    /// Cópia dos mapas de cheiro, lida e gravada pelas rotinas de `feromonios.rs`.
    cheiro: fer::Feromonios,
}

/// Cópia dos mapas do `Estado` para o tipo de `feromonios.rs`.
pub(crate) fn copiar_cheiros(m: &MapasDoEstado) -> fer::Feromonios {
    let c = |g: &Grade| MapaCheiro::de_bytes(&g.dados).expect("mapa de cheiro 64 × 32");
    fer::Feromonios {
        alarme: c(&m.alarme),
        ninho_preto: c(&m.ninho_b),
        trilha_preta: c(&m.trilha_b),
        ninho_vermelho: c(&m.ninho_r),
        trilha_vermelha: c(&m.trilha_r),
    }
}

/// Devolve a cópia ao `Estado`.
pub(crate) fn devolver_cheiros(f: &fer::Feromonios, m: &mut MapasDoEstado) {
    m.alarme.dados.copy_from_slice(&f.alarme.celulas);
    m.ninho_b.dados.copy_from_slice(&f.ninho_preto.celulas);
    m.trilha_b.dados.copy_from_slice(&f.trilha_preta.celulas);
    m.ninho_r.dados.copy_from_slice(&f.ninho_vermelho.celulas);
    m.trilha_r.dados.copy_from_slice(&f.trilha_vermelha.celulas);
}

/// Parte de cheiro de `_DoSmells` para o tick (`rules/tick.rs`): roda `fase` de
/// `feromonios.rs` sobre os mapas do `Estado` (feromonios.md §2–3).
pub(crate) fn rodar_fase_de_cheiro(e: &mut Estado, fase: fer::FaseCheiro, regras: &RegrasFeromonio) {
    let mut cheiro = copiar_cheiros(&e.feromonios);
    let mapa_a = crate::formats::save::Mapa {
        largura: e.superficie.mapa.largura,
        altura: e.superficie.mapa.altura,
        dados: &e.superficie.mapa.dados,
    };
    cheiro.rodar_fase(fase, &e.preta.buracos[..], &e.vermelha.buracos[..], &mapa_a, regras);
    devolver_cheiros(&cheiro, &mut e.feromonios);
}

/// `_DoAntSimA` (`1028:04d8`, seção 1) com tabelas explícitas.
pub fn simular_superficie(e: &mut Estado, t: &TabelasSuperficie, f: &TabelasFeromonio, ta: &TabelasAmarela) {
    let cheiro = copiar_cheiros(&e.feromonios);
    let mut sim = Sim { e, t, f, ta, cheiro };
    sim.laco();
    let Sim { e, cheiro, .. } = sim;
    devolver_cheiros(&cheiro, &mut e.feromonios);
}

impl Sim<'_> {
    fn formiga(&self, i: usize) -> Formiga {
        self.e.superficie.lista.vagas[i]
    }

    fn vaga(&mut self, i: usize) -> &mut Formiga {
        &mut self.e.superficie.lista.vagas[i]
    }

    fn laco(&mut self) {
        let mut tindex = self.e.superficie.lista.usadas();
        while tindex > 0 {
            tindex -= 1;
            self.e.tindex = tindex as i32;
            let i = tindex;
            let r = self.e.s.mascara(256);
            let a = self.formiga(i);
            if r == 0 && a.tipo != 0 {
                let saude = self.e.colonia(cor(a.tipo)).saude as i32;
                if saude < self.e.s.mascara(32) {
                    comum::cadaver_aqui(self.e, a.x as i32, a.y as i32, a.tipo & 0x80 != 0);
                    self.vaga(i).tipo = 0;
                    // Também para vermelhas (peculiaridade P6).
                    self.e.preta.mortes_naturais = self.e.preta.mortes_naturais.wrapping_add(1);
                }
            }
            let a = self.formiga(i);
            if a.tipo == 0 {
                continue;
            }
            let m = a.modo as usize;
            if let Some(c) = self.e.modos_do_tick[indice_cor(cor(a.tipo))].get_mut(m) {
                *c += 1;
            }
            self.despachar(i, a);
        }
    }

    fn despachar(&mut self, i: usize, a: Formiga) {
        let (x, y) = (a.x as i32, a.y as i32);
        match a.modo {
            0 => self.passear(i),
            1 | 4 => self.ir_ao_ninho(i),
            2 => self.forragear(i),
            3 => self.voltar_com_comida(i),
            5 => self.levar_terra(i),
            6 => self.defender(i),
            7 => self.atacar(i),
            8 => {
                // Ovo na superfície (_SimEggA).
                definir_vida(self.e, x, y, a.tipo);
                if self.e.s.rand1(200) == 0 {
                    self.vaga(i).tipo = 0;
                    definir_vida(self.e, x, y, 0);
                }
            }
            9 => self.rainha(i, a),
            10 => self.nuvem_de_luta(i),
            11 => self.ir_ao_alarme(i),
            12 => {
                // Defender o ninho (_DoDefendNest).
                if self.cheiro_de_ninho(a.tipo, x, y) < self.t.regras.cheiro_defender_ninho {
                    self.ir_ao_ninho(i);
                } else {
                    self.passear_mudo(i);
                }
            }
            13 => self.descansar(i),
            14 => {
                // Pré-voo (_DoRepoLoit).
                if self.e.s.mascara(2) == 0 {
                    self.ir_ao_ninho(i);
                } else {
                    self.passear_mudo(i);
                }
                if self.revoada(self.formiga(i).tipo) > self.t.regras.revoada_pre_voo {
                    self.vaga(self.e.tindex as usize).modo = M_DECOLAR;
                }
            }
            15 => self.decolar(i),
            16 => self.voar(i, a),
            19 => self.lider(i),
            // 17, 18 e os demais: nada (peculiaridade P7).
            _ => {}
        }
    }

    fn cheiro_de_ninho(&self, tipo: u8, x: i32, y: i32) -> u8 {
        self.cheiro.camada(fer::Camada::ninho(cor(tipo))).no_tile(x, y)
    }

    fn revoada(&self, tipo: u8) -> i16 {
        self.e.colonia(cor(tipo)).revoada
    }

    // --- peças do passo ---

    /// Giro sem movimento (2.2): `T = base | Giro[dir][S8()]`, `Vida(x, y) = T`.
    fn girar(&mut self, i: usize, x: i32, y: i32, tipo: u8) {
        let k = self.e.s.mascara(8) as u8;
        let novo = base(tipo) | self.f.virar(dir(tipo), k);
        self.vaga(i).tipo = novo;
        definir_vida(self.e, x, y, novo);
    }

    /// Anda de (x, y) para (nx, ny) com direção `d` (`Vida` e lista).
    fn andar(&mut self, i: usize, x: i32, y: i32, nx: i32, ny: i32, novo: u8) {
        definir_vida(self.e, nx, ny, novo);
        let v = self.vaga(i);
        v.tipo = novo;
        v.x = nx as u8;
        v.y = ny as u8;
        definir_vida(self.e, x, y, 0);
    }

    /// Se `S > 0`: `S −= 1` e deposita `S` na camada dada (`_JamScent*`).
    fn deixar_rastro(&mut self, i: usize, camada: fer::Camada, x: i32, y: i32) {
        let s = self.formiga(i).s;
        if s != 0 {
            self.vaga(i).s = s - 1;
            self.cheiro.camada_mut(camada).depositar(x, y, s - 1);
        }
    }

    /// Início de luta (6.1, `_StartFightA` embutido).
    fn iniciar_luta(&mut self, i: usize, x: i32, y: i32, nx: i32, ny: i32) {
        let tb = self.formiga(i).tipo;
        self.vaga(i).tipo = 0;
        definir_vida(self.e, x, y, 0);
        let Some(j) = achar_na_lista_a(self.e, nx, ny) else {
            return; // o atacante some sem luta (P5)
        };
        let ta = self.formiga(j).tipo;
        let v = vencedor_da_luta(self.e, self.t, ta, tb);
        let nuvem = NUVEM_DE_LUTA + (v & 0x80);
        let vj = self.vaga(j);
        vj.tipo = nuvem;
        vj.modo = M_LUTA;
        vj.s = v;
        definir_vida(self.e, nx, ny, nuvem);
        self.cheiro.alarme.alarme_de_luta(nx, ny, self.t.regras.alarme_de_luta);
    }

    /// O passo padrão (2.3) com as variações de `p`. `tipo` dá a base e a direção original (no
    /// modo 7, já sem a carga); a luta usa o T gravado na vaga.
    fn passo(&mut self, i: usize, x: i32, y: i32, tipo: u8, d: u8, p: Passo) {
        let (nx, ny) = (x + self.f.dx8[d as usize] as i32, y + self.f.dy8[d as usize] as i32);
        let g = grupo(tipo);
        let v = tile(self.e, nx, ny);
        if p.pega_comida && e_comida(self.e, v) {
            if operaria_ou_soldado(g) {
                let novo = d | base(tipo) | 8;
                self.vaga(i).tipo = novo;
                definir_vida(self.e, x, y, novo);
                self.vaga(i).modo = M_COM_COMIDA;
                pegar_comida(self.e, nx, ny);
                self.vaga(i).s = self.t.regras.s_ao_pegar_comida;
                return;
            }
            // Comida para quem não pega: o teste de barreira é pulado.
        } else if e_barreira(self.e, v) {
            if p.barreira_sem_tabela {
                let novo = base(tipo) | self.e.s.mascara(8) as u8;
                self.vaga(i).tipo = novo;
                definir_vida(self.e, x, y, novo);
            } else {
                self.girar(i, x, y, tipo);
            }
            if p.modo_novo_na_barreira && self.e.s.mascara(16) == 0 {
                let m = modo_novo(self.e, self.t, g, tipo);
                let a = self.vaga(i);
                a.modo = m;
                a.s = 0;
            }
            return;
        }
        let ocupante = vida(self.e, nx, ny);
        let c = cor(tipo);
        if ocupante == 0 {
            self.andar(i, x, y, nx, ny, d | base(tipo));
            if p.cheiro_de_ninho {
                self.deixar_rastro(i, fer::Camada::ninho(c), nx, ny);
            }
            if p.gasta_trilha {
                self.cheiro.camada_mut(fer::Camada::trilha(c)).gastar(nx, ny);
            }
            if p.vira_forrageira && self.e.s.mascara(8) == 0 && operaria_ou_soldado(g) {
                self.vaga(i).modo = M_FORRAGEAR;
            }
            return;
        }
        if e_amarela(ocupante) {
            if (tipo ^ cor_amarela(self.e)) & 0x80 != 0 {
                amarela::lutar_com_amarela(self.e, self.ta, 1, i as i32);
                return;
            }
            let quer = self.e.amarela.quer_comida;
            let alimenta = match p.trofalaxia {
                Trofalaxia::Nunca => false,
                Trofalaxia::SeIgualAUm => quer == 1,
                Trofalaxia::SeDiferenteDeZero => quer != 0,
            };
            if alimenta {
                let novo = d | base(tipo);
                self.vaga(i).tipo = novo;
                definir_vida(self.e, x, y, novo);
                amarela::trofalaxia(self.e, self.ta, x, y, d as i32);
            }
            self.girar(i, x, y, tipo);
            return;
        }
        if (ocupante ^ tipo) & 0x80 == 0 {
            self.girar(i, x, y, tipo);
            if p.modo_novo_na_mesma_cor {
                self.vaga(i).modo = modo_novo(self.e, self.t, g, tipo);
            }
            return;
        }
        self.iniciar_luta(i, x, y, nx, ny);
    }

    // --- funções de direção (2.5) ---

    /// `_GetRandDir` (`1030:0f72`).
    fn direcao_aleatoria(&mut self, x: i32, y: i32, d: u8) -> u8 {
        if let Some(b) = quicar(self.e, x, y) {
            return b;
        }
        let k = self.e.s.mascara(8) as u8;
        self.f.virar(d, k)
    }

    /// `_GetNestDir` (`1030:0c30`) para o cheiro de ninho da cor de `tipo`.
    fn direcao_ninho(&mut self, x: i32, y: i32, d: u8, tipo: u8) -> u8 {
        let c = cor(tipo);
        let (bx, by) = self.e.colonia(c).ultimo_buraco;
        fer::direcao_ninho(&self.cheiro, self.f, x, y, d, c, (bx as i32, by as i32), &mut self.e.s)
    }

    /// `_GetDefendDir` (`1030:1026`): segue a amarela.
    fn direcao_defesa(&mut self, x: i32, y: i32, d: u8) -> u8 {
        if let Some(b) = quicar(self.e, x, y) {
            return b;
        }
        let k = match self.e.amarela.plano {
            2 => return self.direcao_ninho(x, y, d, 0),
            3 => return self.direcao_ninho(x, y, d, 0x80),
            1 => {
                if self.e.aranha.perseguir == 1 {
                    direcao_para(x, y, (self.e.aranha.x >> 4) as i32, (self.e.aranha.y >> 4) as i32)
                } else {
                    let (fx, fy) = (self.e.amarela.difusa_x as i32, self.e.amarela.difusa_y as i32);
                    if distancia(x, y, fx, fy) <= (self.e.preta.modos[5] >> 1) as i32 {
                        self.e.s.rand1(8) as u8 + 1
                    } else {
                        direcao_para(x, y, fx, fy)
                    }
                }
            }
            _ => 0,
        };
        if k == 0 {
            d
        } else {
            self.f.virar(d, k - 1)
        }
    }

    /// `_GetRedDefendDir` (`1030:1194`): segue a líder vermelha.
    fn direcao_defesa_vermelha(&mut self, x: i32, y: i32, d: u8) -> u8 {
        if let Some(b) = quicar(self.e, x, y) {
            return b;
        }
        let k = match self.e.lider_vermelha.plano {
            2 => return self.direcao_ninho(x, y, d, 0),
            3 => return self.direcao_ninho(x, y, d, 0x80),
            1 => {
                let (lx, ly) = (self.e.lider_vermelha.x as i32, self.e.lider_vermelha.y as i32);
                if ((self.e.vermelha.modos[5] >> 1) as i32) < distancia(x, y, lx, ly) {
                    direcao_para(x, y, lx, ly)
                } else {
                    self.e.s.rand1(8) as u8 + 1
                }
            }
            _ => 0,
        };
        if k == 0 {
            d
        } else {
            self.f.virar(d, k - 1)
        }
    }

    // --- os modos (seção 3) ---

    /// Teste de buraco; devolve `true` se a formiga entrou no ninho.
    fn buraco(&mut self, i: usize, x: i32, y: i32) -> bool {
        if e_buraco(self.e, x, y) {
            entrar_no_ninho(self.e, self.f, x, y, i);
            true
        } else {
            false
        }
    }

    /// Modo 0: `_DoRandAntA` (3.1).
    fn passear(&mut self, i: usize) {
        let a = self.formiga(i);
        let (x, y) = (a.x as i32, a.y as i32);
        if self.buraco(i, x, y) {
            return;
        }
        let d = self.direcao_aleatoria(x, y, dir(a.tipo));
        let p = Passo {
            pega_comida: true,
            cheiro_de_ninho: true,
            gasta_trilha: true,
            vira_forrageira: true,
            trofalaxia: Trofalaxia::SeIgualAUm,
            modo_novo_na_mesma_cor: true,
            ..PASSO_MUDO
        };
        self.passo(i, x, y, a.tipo, d, p);
    }

    /// Modo 0': `_DoRandAntAA` (3.2).
    fn passear_mudo(&mut self, i: usize) {
        let a = self.formiga(i);
        let (x, y) = (a.x as i32, a.y as i32);
        if self.buraco(i, x, y) {
            return;
        }
        let d = self.direcao_aleatoria(x, y, dir(a.tipo));
        self.passo(i, x, y, a.tipo, d, PASSO_MUDO);
    }

    /// Modos 1 e 4: `_DoToNestAnt` (3.3).
    fn ir_ao_ninho(&mut self, i: usize) {
        let a = self.formiga(i);
        let (x, y) = (a.x as i32, a.y as i32);
        if self.buraco(i, x, y) {
            return;
        }
        let d = self.direcao_ninho(x, y, dir(a.tipo), a.tipo);
        let p = Passo { pega_comida: true, cheiro_de_ninho: true, trofalaxia: Trofalaxia::SeIgualAUm, ..PASSO_MUDO };
        self.passo(i, x, y, a.tipo, d, p);
    }

    /// Modo 2: `_DoForageAnt` (3.4).
    fn forragear(&mut self, i: usize) {
        let a = self.formiga(i);
        let (x, y) = (a.x as i32, a.y as i32);
        if self.buraco(i, x, y) {
            return;
        }
        if self.e.s.mascara(32) == 0 {
            self.vaga(i).modo = M_DESCANSO;
            return;
        }
        if self.cheiro.alarme.no_tile(x, y) != 0 {
            self.vaga(i).modo = M_ALARME;
            return;
        }
        let g = grupo(a.tipo);
        if !operaria_ou_soldado(g) {
            let m = modo_novo(self.e, self.t, g, a.tipo);
            let v = self.vaga(i);
            v.modo = m;
            v.s = 0;
            return;
        }
        let c = cor(a.tipo);
        let Some(d) = fer::direcao_forrageio(&self.cheiro, self.f, x, y, dir(a.tipo), c, &mut self.e.s) else {
            // Fim da trilha.
            let m = if self.e.s.mascara(8) == 0 { modo_novo(self.e, self.t, g, a.tipo) } else { M_PASSEIO };
            let v = self.vaga(i);
            v.modo = m;
            v.s = 0;
            // Peculiaridade P1: o original passa (x >> 1, y >> 1) a `_DecTSmell`, que divide de novo.
            self.cheiro.camada_mut(fer::Camada::trilha(c)).gastar(x >> 1, y >> 1);
            return;
        };
        let p = Passo {
            pega_comida: true,
            cheiro_de_ninho: true,
            gasta_trilha: true,
            trofalaxia: Trofalaxia::SeIgualAUm,
            modo_novo_na_barreira: true,
            ..PASSO_MUDO
        };
        self.passo(i, x, y, a.tipo, d, p);
    }

    /// Modo 3: `_DoReturnFoodAnt` (3.5). Anda por cima de qualquer formiga (P2).
    fn voltar_com_comida(&mut self, i: usize) {
        let a = self.formiga(i);
        let (x, y) = (a.x as i32, a.y as i32);
        if self.buraco(i, x, y) {
            return;
        }
        let d = self.direcao_ninho(x, y, dir(a.tipo), a.tipo);
        let (nx, ny) = (x + self.f.dx8[d as usize] as i32, y + self.f.dy8[d as usize] as i32);
        if e_barreira(self.e, tile(self.e, nx, ny)) {
            self.girar(i, x, y, a.tipo);
            return;
        }
        self.andar(i, x, y, nx, ny, d | base(a.tipo));
        self.deixar_rastro(i, fer::Camada::trilha(cor(a.tipo)), nx, ny);
    }

    /// Modo 5: `_DoDigOutAntA` (3.6). Sem teste de buraco.
    fn levar_terra(&mut self, i: usize) {
        let a = self.formiga(i);
        let (x, y) = (a.x as i32, a.y as i32);
        let k = self.e.s.mascara(8) as u8;
        let mut d = self.f.virar(dir(a.tipo), k);
        if let Some(b) = quicar(self.e, x, y) {
            d = b;
        }
        let g = grupo(a.tipo);
        if g != 5 && g != 9 {
            let m = modo_novo(self.e, self.t, g, a.tipo);
            let v = self.vaga(i);
            v.modo = m;
            v.s = 0;
            return;
        }
        if self.e.s.mascara(8) == 0 {
            // Larga a terra (some): grupo 5 → 2, 9 → 6.
            let novo = a.tipo.wrapping_sub(0x18);
            self.vaga(i).tipo = novo;
            let m = modo_novo(self.e, self.t, g, a.tipo);
            let v = self.vaga(i);
            v.modo = m;
            v.s = 0;
            definir_vida(self.e, x, y, novo);
            return;
        }
        let (nx, ny) = (x + self.f.dx8[d as usize] as i32, y + self.f.dy8[d as usize] as i32);
        if e_barreira(self.e, tile(self.e, nx, ny)) || vida(self.e, nx, ny) != 0 {
            self.girar(i, x, y, a.tipo);
            return;
        }
        self.andar(i, x, y, nx, ny, d | base(a.tipo));
        self.deixar_rastro(i, fer::Camada::ninho(cor(a.tipo)), nx, ny);
    }

    /// Modo 6: `_DoRecruitAnt` (3.7).
    fn defender(&mut self, i: usize) {
        let a = self.formiga(i);
        let (x, y) = (a.x as i32, a.y as i32);
        if self.buraco(i, x, y) {
            return;
        }
        let d = if self.cheiro.alarme.no_tile(x, y) != 0 {
            fer::direcao_alarme(&self.cheiro, self.f, x, y, dir(a.tipo), &mut self.e.s)
        } else if a.tipo < 0x80 {
            self.direcao_defesa(x, y, dir(a.tipo))
        } else {
            self.direcao_defesa_vermelha(x, y, dir(a.tipo))
        };
        let p = Passo { trofalaxia: Trofalaxia::SeDiferenteDeZero, barreira_sem_tabela: true, ..PASSO_MUDO };
        self.passo(i, x, y, a.tipo, d, p);
    }

    /// Modo 7: `_DoAttackAnt` (3.8).
    fn atacar(&mut self, i: usize) {
        let a = self.formiga(i);
        let (x, y) = (a.x as i32, a.y as i32);
        if self.buraco(i, x, y) {
            return;
        }
        let g = grupo(a.tipo) as usize;
        let mut tipo = a.tipo;
        if self.t.carrega_carga[g] == 1 {
            tipo = ((self.t.grupo_sem_carga[g] as u8) << 3) | (a.tipo & 0x87);
        }
        let d = self.direcao_ninho(x, y, dir(tipo), tipo ^ 0x80);
        self.passo(i, x, y, tipo, d, PASSO_MUDO);
    }

    /// Modo 9: rainha na superfície (3.10). Só a cauda é verificada.
    fn rainha(&mut self, i: usize, a: Formiga) {
        let (x, y) = (a.x as i32, a.y as i32);
        definir_vida(self.e, x, y, a.tipo);
        if a.tipo & 0x7F <= 0x67 {
            return;
        }
        let d = dir(a.tipo) as usize;
        let (hx, hy) = (x + self.f.dx8[d] as i32, y + self.f.dy8[d] as i32);
        let cabeca = vida(self.e, hx, hy) == a.tipo.wrapping_sub(8) || achar_na_lista_a(self.e, hx, hy).is_some();
        if !cabeca {
            self.vaga(i).tipo = 0;
            definir_vida(self.e, x, y, 0);
        }
    }

    /// Modo 10: `_DoFightA` (6.3).
    fn nuvem_de_luta(&mut self, i: usize) {
        let a = self.formiga(i);
        let (x, y) = (a.x as i32, a.y as i32);
        let q = self.e.s.rand1(7) as u8;
        let novo = base(a.tipo).wrapping_add(q);
        self.vaga(i).tipo = novo;
        definir_vida(self.e, x, y, novo);
        if self.e.s.mascara(16) == 0 {
            let v = a.s;
            definir_vida(self.e, x, y, v);
            self.vaga(i).tipo = v;
            let m = modo_novo(self.e, self.t, grupo(v), v);
            self.vaga(self.e.tindex as usize).modo = m;
            self.vaga(i).s = 0;
            // Cadáver com a cor do vencedor (P3).
            comum::cadaver_aqui(self.e, x, y, v & 0x80 != 0);
        } else if self.e.partida.opcoes[5] == 1 {
            // `_FightBalloons(x, y, 1)`: só anota o balão (sem sorteio).
            baloes::anotar(self.e, TipoBalao::Luta, x, y, 1);
        }
    }

    /// Modo 11: `_DoToAlarm` (3.11).
    fn ir_ao_alarme(&mut self, i: usize) {
        let a = self.formiga(i);
        let (x, y) = (a.x as i32, a.y as i32);
        if self.buraco(i, x, y) {
            return;
        }
        if self.cheiro.alarme.no_tile(x, y) == 0 && self.e.s.mascara(4) == 0 {
            definir_vida(self.e, x, y, a.tipo);
            self.vaga(i).modo = modo_novo(self.e, self.t, grupo(a.tipo), a.tipo);
            return;
        }
        let d = fer::direcao_alarme(&self.cheiro, self.f, x, y, dir(a.tipo), &mut self.e.s);
        self.passo(i, x, y, a.tipo, d, PASSO_MUDO);
    }

    /// Modo 13: descanso (3.13).
    fn descansar(&mut self, i: usize) {
        let a = self.formiga(i);
        let (x, y) = (a.x as i32, a.y as i32);
        if self.buraco(i, x, y) {
            return;
        }
        if self.e.s.mascara(4) == 0 {
            self.vaga(i).modo = M_FORRAGEAR;
        } else if self.e.partida.opcoes[5] == 1 {
            // `_RestBalloons(x, y, 1)`: só anota o balão (sem sorteio).
            baloes::anotar(self.e, TipoBalao::Descanso, x, y, 1);
        }
    }

    /// Modo 15: `_DoRepoExit` (3.15).
    fn decolar(&mut self, i: usize) {
        let a = self.formiga(i);
        if self.cheiro_de_ninho(a.tipo, a.x as i32, a.y as i32) < self.t.regras.cheiro_decolar {
            self.ir_ao_ninho(i);
        } else {
            self.passear_mudo(i);
        }
        let f = self.revoada(self.formiga(i).tipo);
        let voa = match f {
            0 => false,
            1 => true,
            _ => self.e.s.rand1(f as u16) == 0,
        };
        if voa {
            self.vaga(self.e.tindex as usize).modo = M_VOO;
        }
    }

    /// Modo 16: voo (3.16).
    fn voar(&mut self, i: usize, a: Formiga) {
        let c = cor(a.tipo);
        if self.e.s.mascara(32) != 0 || self.e.colonia(c).enxame_contagem >= self.t.regras.limite_enxame {
            return;
        }
        self.vaga(i).tipo = 0;
        definir_vida(self.e, a.x as i32, a.y as i32, 0);
        if self.e.partida.tipo_jogo == 2 {
            let col = self.e.colonia_mut(c);
            col.enxame_contagem = col.enxame_contagem.wrapping_add(1);
            if self.e.s.mascara(16) == 0 {
                let col = self.e.colonia_mut(c);
                col.rainhas_guardadas = col.rainhas_guardadas.wrapping_add(1);
                // [lacuna] Na preta, o original também redesenha o mostrador (_InvalQueenStorageDisp).
            }
        }
    }

    /// Modo 19: `_DoRedInitiator` (9.2).
    fn lider(&mut self, i: usize) {
        let a = self.formiga(i);
        let (x, y) = (a.x as i32, a.y as i32);
        let l = &mut self.e.lider_vermelha;
        l.x = a.x as i16;
        l.y = a.y as i16;
        l.plano = 1;
        if self.e.lider_vermelha.tarefa == 0 {
            desrecrutar_vermelhas(self.e);
            let caca = self.e.amarela.plano == 1
                && self.e.s.rand1(32) + 64 < self.e.amarela.x as i32
                && self.e.s.rand1(10) < self.e.preta.modos[5] as i32;
            if caca {
                self.e.lider_vermelha.tarefa = 2;
                recrutar_vermelhas(self.e, self.e.preta.modos[5] as i32);
            } else {
                let (mut dx, mut dy) = self.e.ambiente.ultima_comida;
                if dx < 31 {
                    if dy < 20 {
                        dy += 5;
                    } else if dy > 40 {
                        dy -= 5;
                    }
                } else {
                    dx -= 5;
                }
                self.e.lider_vermelha.destino_x = dx;
                self.e.lider_vermelha.destino_y = dy;
                let n = self.e.vermelha.castas[1] as i32 + self.e.vermelha.castas[2] as i32;
                let n = if n < 20 { n >> 2 } else { n >> 3 };
                recrutar_vermelhas(self.e, n);
                self.e.lider_vermelha.tarefa = 1;
            }
        } else if self.e.lider_vermelha.tarefa == 2 && self.e.amarela.plano == 1 {
            self.e.lider_vermelha.destino_x = self.e.amarela.difusa_x;
            self.e.lider_vermelha.destino_y = self.e.amarela.difusa_y;
        }
        let (tx, ty) = (self.e.lider_vermelha.destino_x as i32, self.e.lider_vermelha.destino_y as i32);
        match melhor_direcao_vermelha(self.e, self.f, x, y, tx, ty) {
            None => {
                if self.e.s.rand1(10) == 0 {
                    self.e.lider_vermelha.tarefa = 0;
                }
            }
            Some(d) => {
                let (nx, ny) = (x + self.f.dx8[d as usize] as i32, y + self.f.dy8[d as usize] as i32);
                let ocupante = vida(self.e, nx, ny);
                if ocupante == 0 {
                    self.andar(i, x, y, nx, ny, base(a.tipo) | d);
                    return;
                }
                if e_amarela(ocupante) {
                    amarela::lutar_com_amarela(self.e, self.ta, 1, i as i32);
                    return;
                }
                if ocupante < 0x80 {
                    return; // formiga preta: nada, nem regrava a vida
                }
                let k = self.e.s.mascara(8) as u8;
                self.vaga(i).tipo = base(a.tipo) | self.f.virar(dir(a.tipo), k);
            }
        }
        let t = self.formiga(i).tipo;
        definir_vida(self.e, x, y, t);
    }
}

/// `_GetRedBestDirs(1, x, y, tx, ty)` (`1028:9a18`, 9.2). `None` = −1/−2 do original.
fn melhor_direcao_vermelha(e: &Estado, f: &TabelasFeromonio, x: i32, y: i32, tx: i32, ty: i32) -> Option<u8> {
    let mut dist = distancia(x, y, tx, ty);
    if dist <= 0 {
        return None;
    }
    let mut livre = None;
    let mut bloqueada = None;
    for k in 0..8u8 {
        let (vx, vy) = (x + f.dx8[k as usize] as i32, y + f.dy8[k as usize] as i32);
        if !pode_pisar(e, vx, vy) {
            continue;
        }
        let dv = distancia(vx, vy, tx, ty);
        if dv >= dist {
            continue;
        }
        dist = dv;
        let vazia = e.superficie.vida.em(vx, vy).is_none_or(|v| v == 0);
        if vazia && celula_livre(e, vx, vy) {
            livre = Some(k);
        } else {
            bloqueada = Some(k);
        }
    }
    livre.or(bloqueada)
}

/// `_TileCanBeMovedOn` no plano da superfície ([`comum::pode_pisar_superficie`]).
fn pode_pisar(e: &Estado, x: i32, y: i32) -> bool {
    comum::pode_pisar_superficie(e, x, y)
}

/// `_IsClearTile` na superfície (`1020:5b2c`, [`comum::celula_livre`]).
fn celula_livre(e: &Estado, x: i32, y: i32) -> bool {
    comum::celula_livre(e, 1, x, y)
}

// ---------------------------------------------------------------------------------------------
// Recrutamento (9.4)
// ---------------------------------------------------------------------------------------------

// `_Recruit` e `_UnRecruit` estão em `rules/comum.rs` (a amarela também os usa) e são reexportados
// aqui como [`recrutar`] e [`desrecrutar`].

/// `_RecruitRed(n)` (`1030:0866`): vermelhas de grupo 2/6 da lista A com M ≠ 19 e ≠ 6 → M = 6,
/// S = 0, até `n`.
pub fn recrutar_vermelhas(e: &mut Estado, mut n: i32) {
    let l = &mut e.superficie.lista;
    for j in (0..l.usadas()).rev() {
        if n < 1 {
            return;
        }
        let f = &mut l.vagas[j];
        if f.tipo >= 0x80 && operaria_ou_soldado(grupo(f.tipo)) && f.modo != M_LIDER && f.modo != M_DEFESA {
            f.modo = M_DEFESA;
            f.s = 0;
            n -= 1;
        }
    }
}

/// `_UnRecruitRed()` (`1030:08da`): toda vermelha da lista A com M = 6 → M = 0.
pub fn desrecrutar_vermelhas(e: &mut Estado) {
    let l = &mut e.superficie.lista;
    let n = l.usadas();
    for f in l.vagas[..n].iter_mut().rev() {
        if f.tipo >= 0x80 && f.modo == M_DEFESA {
            f.modo = M_PASSEIO;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Estratégia (9.1) e líder vermelha (9.2)
// ---------------------------------------------------------------------------------------------

/// `_GetStrategy` (`1030:0000`), com `_GstrR`, `_SetCasteProd` e `_SetModeProd`.
pub fn escolher_estrategia(e: &mut Estado, t: &TabelasSuperficie) {
    let r = &t.regras;
    e.aranha.perseguir = 0;
    if e.amarela.plano == 1 {
        let fx = (e.amarela.x as i32 + e.s.rand1(5) - 2).clamp(0, 127);
        let fy = (e.amarela.y as i32 + e.s.rand1(5) - 2).clamp(0, 63);
        e.amarela.difusa_x = fx as i16;
        e.amarela.difusa_y = fy as i16;
        if e.aranha.ligada != 0
            && distancia(
                (e.aranha.x >> 4) as i32,
                (e.aranha.y >> 4) as i32,
                e.amarela.x as i32,
                e.amarela.y as i32,
            ) < r.raio_aranha_perseguida
        {
            e.aranha.perseguir = 1;
        }
    }
    // Sobrescreve sempre: o sorteio de X é descartado.
    e.amarela.difusa_x = e.amarela.x;
    e.preta.modo_estrategico = estrategia_preta(e, t);
    e.vermelha.modo_estrategico = estrategia_vermelha(e, t);
    escolher_casta(e, t);
    escolher_tarefa(e, t);
}

/// A escada preta embutida em `_GetStrategy` (igual a `_GstrB`). Sem sorteio.
fn estrategia_preta(e: &Estado, t: &TabelasSuperficie) -> i16 {
    let r = &t.regras;
    let (s, p, d) = (e.preta.saude, e.preta.populacao, e.preta.tiles_escavados);
    let (pr, rainhas) = (e.vermelha.populacao, e.vermelha.rainhas);
    if s < r.saude_desespero && p >> 1 > pr && pr >= 1 && rainhas >= 1 {
        return 0;
    }
    if s < r.saude_fome {
        5
    } else if s < r.saude_pouca {
        4
    } else if d < p {
        3
    } else if (d as i32) < p as i32 * 2 {
        2
    } else if p < r.populacao_grande || pr < 1 || rainhas < 1 || p / 3 <= pr {
        1
    } else {
        0
    }
}

/// `_GstrR` (`1030:03c2`).
fn estrategia_vermelha(e: &mut Estado, t: &TabelasSuperficie) -> i16 {
    let r = &t.regras;
    if e.vermelha.revoada == 0 && e.vermelha.castas[3] + e.vermelha.castas[4] > r.revoada_vermelha_reprodutores {
        e.vermelha.revoada = r.revoada_vermelha_contagem;
    }
    if e.lider_vermelha.malvadeza != 0 {
        e.lider_vermelha.malvadeza -= 1;
        return 0;
    }
    let (s, p, d) = (e.vermelha.saude, e.vermelha.populacao, e.vermelha.tiles_escavados);
    let (pb, rainhas_b) = (e.preta.populacao, e.preta.rainhas);
    let desespero = s < r.saude_desespero && pb < p >> 1 && pb > 0 && rainhas_b > 0;
    if !desespero {
        if s < r.saude_fome {
            return 5;
        }
        if s < r.saude_pouca {
            return 4;
        }
        if d < p {
            return 3;
        }
        if (d as i32) < p as i32 * 2 {
            return 2;
        }
        let normal = p < r.populacao_grande || pb < 1 || rainhas_b < 1 || p / 3 <= pb;
        // Curto-circuito: S32 só com `normal`; S128 só se os testes anteriores falharem.
        if normal
            && (e.s.mascara(32) != 0 || p < r.malvadeza_minima_populacao || p <= pb || e.s.mascara(128) != 0)
        {
            return 1;
        }
    }
    // MALVADO.
    e.lider_vermelha.malvadeza = e.s.rand1(r.malvadeza_sorteio) as i16 + r.malvadeza_base;
    e.eventos.push(super::estado::Evento::Musica(r.musica_ataque_vermelho));
    e.eventos.push(super::estado::Evento::MensagemFaixa(r.mensagem_ataque_vermelho));
    0
}

/// `_SetCasteProd` (`1030:026e`, ciclo-de-vida.md 6.1). Sem sorteio.
fn escolher_casta(e: &mut Estado, t: &TabelasSuperficie) {
    let castas = e.preta.castas;
    let ideais = e.controles.castas_ideais;
    let p: i32 = castas[1..5].iter().map(|&c| c as i32).sum();
    let q: i32 = ideais.iter().map(|&c| c as i32).sum();
    let mut menor = 0i32;
    let mut escolhido = 0usize;
    for k in 0..4 {
        let real = if p < 1 { 0 } else { (castas[k + 1] as i32 * 100 / p) as i16 as i32 };
        let ideal = if q < 1 { 0 } else { (ideais[k] as i32 * 100 / q) as i16 as i32 };
        let dif = (real - ideal) as i16 as i32;
        if dif < menor {
            menor = dif;
            escolhido = k;
        }
    }
    e.controles.proxima_casta = t.casta_por_escolha[escolhido];
}

/// `_SetModeProd` (`1030:0326`, formigas.md §5). Sem sorteio.
fn escolher_tarefa(e: &mut Estado, t: &TabelasSuperficie) {
    let modos = e.preta.modos;
    let total = modos[0] as i32 + modos[1] as i32 + modos[2] as i32;
    let mut maior = 0i32;
    let mut escolhido = 0usize;
    for (k, (&nivel, &atual)) in e.controles.tarefas.iter().zip(&modos).enumerate() {
        let produto = (nivel as i64 * total as i64) as u32;
        let ideal = (produto / 0xFFFF) as u16 as i16 as i32;
        let falta = (ideal - atual as i32) as i16 as i32;
        if falta > maior {
            maior = falta;
            escolhido = k;
        }
    }
    e.controles.proximo_modo = t.modo_por_tarefa[escolhido];
}

/// `_MakeRedInitiator` (`1028:967c`, 9.2). Não atualiza o `_LifeA` (P4).
pub fn criar_lider_vermelha(e: &mut Estado, t: &TabelasSuperficie) {
    e.lider_vermelha.ligada = 0;
    if e.preta.populacao < t.regras.populacao_lider {
        return;
    }
    let l = &mut e.superficie.lista;
    let Some(j) = (0..l.usadas()).rev().find(|&j| l.vagas[j].tipo >= 0x80) else {
        return;
    };
    let f = &mut l.vagas[j];
    f.tipo = t.regras.tipo_lider;
    f.modo = M_LIDER;
    f.s = 0;
    e.lider_vermelha.tarefa = 0;
    e.lider_vermelha.ligada = 1;
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn tabelas_embutidas_conferem_com_a_spec() {
        let (t, f) = tabelas_padrao();
        assert_eq!(t.modo_por_grupo, [8, 1, 0, 3, 14, 5, 0, 3, 14, 5, 0, 0, 9, 9, 10, 0]);
        assert_eq!(t.matriz_luta[1], [8, 5, 9, 4]);
        assert_eq!(t.regras.tipo_lider, 0xB0);
        assert_eq!(f.dx8, [0, 1, 1, 1, 0, -1, -1, -1]);
    }

    #[test]
    fn get_dir_pelos_sinais() {
        assert_eq!(direcao_para(5, 5, 5, 5), 0);
        assert_eq!(direcao_para(5, 5, 5, 0), 1);
        assert_eq!(direcao_para(5, 5, 9, 0), 2);
        assert_eq!(direcao_para(5, 5, 9, 5), 3);
        assert_eq!(direcao_para(5, 5, 9, 9), 4);
        assert_eq!(direcao_para(5, 5, 5, 9), 5);
        assert_eq!(direcao_para(5, 5, 0, 9), 6);
        assert_eq!(direcao_para(5, 5, 0, 5), 7);
        assert_eq!(direcao_para(5, 5, 0, 0), 8);
    }

    #[test]
    fn bounce_aponta_para_dentro() {
        let mut e = Estado::vazio();
        e.s.definir_semente(0x1234);
        for _ in 0..200 {
            for (x, y) in [(0, 0), (0, 30), (0, 63), (127, 0), (127, 30), (127, 63), (60, 0), (60, 63)] {
                let d = quicar(&mut e, x, y).unwrap() as usize;
                let (dx, dy) = (tabelas_padrao().1.dx8[d] as i32, tabelas_padrao().1.dy8[d] as i32);
                assert!((0..128).contains(&(x + dx)) && (0..64).contains(&(y + dy)), "({x},{y}) dir {d}");
            }
        }
        assert_eq!(quicar(&mut e, 5, 5), None);
    }
}
