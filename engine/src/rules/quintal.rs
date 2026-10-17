//! Quintal (`_DoSimYard`, `1030:14a8`, com menino, cortador, gato, cachorro, pássaro, chuva e
//! `_SimColonies`) e água (`_DoWater`, `1020:07c6`).
//! Especificação: docs/spec/regras/quintal.md (e ninho.md §8 para a água).
//!
//! - As tabelas do executável e as constantes das regras vêm de `data/quintal.json`
//!   ([`TabelasQuintal`]), gerado por `tools/extrair_quintal.py`.
//! - Os sorteios seguem a ordem do original, inclusive os que dependem da vista aberta
//!   (`_MapMode`, `_YardMode`) e dos relógios (ver [`Relogios`]).
//! - Mensagens e músicas saem como [`Evento`]; sons e o balão do menino não geram evento.

use std::sync::OnceLock;

use serde::Deserialize;

use super::amarela::{self, CausaMorte, TabelasAmarela};
use super::comum::{self, achar_na_lista_a, Direcoes};
use super::estado::{Estado, Evento};
use super::geracao::Cor;
use super::inimigos::TabelasInimigos;
use super::tick::{RegrasOriginais, RegrasQuintal};
use crate::formats::save::Lista;
use crate::recursos::Recursos;

/// Nome do arquivo de dados (em `data/` ou num mod).
pub const ARQUIVO_TABELAS: &str = "quintal.json";

/// Patches do quintal: `px` 0..11, `py` 0..15; índice `px * 16 + py` (quintal.md, "Coordenadas").
pub const LARGURA: i32 = 12;
pub const ALTURA: i32 = 16;

// ---------------------------------------------------------------------------------------------
// Dados
// ---------------------------------------------------------------------------------------------

/// Tabelas do menino lá fora (quintal.md §12).
#[derive(Clone, Debug, Deserialize)]
pub struct TabMenino {
    /// `_BxTab`/`_ByTab`: passo por direção 0–3.
    pub passo_x: [i8; 4],
    pub passo_y: [i8; 4],
    /// Quadro somado pela fase `_YardCycle & 3`.
    pub quadro_por_passo: [i8; 4],
    /// Quadro base por direção.
    pub quadro_por_dir: [i8; 4],
    /// Cortador relativo ao pé.
    pub corte_dx: [i16; 4],
    pub corte_dy: [i16; 4],
    /// Patch vizinho por direção (escolha do próximo patch a cortar).
    pub vizinho_dx: [i8; 4],
    pub vizinho_dy: [i8; 4],
    /// Direção do pé pelo quadro (menino dentro de casa, quadros 0–11).
    pub dir_do_quadro: [u8; 12],
}

/// Tabelas do cachorro (direções 0–3 andando, 4–5 sentado).
#[derive(Clone, Debug, Deserialize)]
pub struct TabCachorro {
    pub passo_x: [i8; 6],
    pub passo_y: [i8; 6],
    pub quadro_por_passo: [i8; 4],
    pub quadro_por_dir: [i8; 6],
}

/// Um trecho de caminhada do roteiro da casa: quadro, x, y por passo (`_NodeCnt`).
#[derive(Clone, Debug, Deserialize)]
pub struct Roteiro {
    pub no: u8,
    pub quadro: Vec<i16>,
    pub x: Vec<u8>,
    pub y: Vec<u8>,
}

/// Um nó de parada (30–38) do roteiro da casa.
#[derive(Clone, Debug, Deserialize)]
pub struct NoFixo {
    pub no: u8,
    pub quadro: i16,
    pub x: u8,
    pub y: u8,
    /// Sai do nó com `S(chance) == 0`.
    pub chance: u8,
    pub proximo: u8,
}

/// Roteiro do menino dentro de casa (quintal.md §4).
#[derive(Clone, Debug, Deserialize)]
pub struct TabCasa {
    /// `_NodeCnt` máximo de cada nó 0–21.
    pub duracao: Vec<u8>,
    /// Três tabelas de próximo nó, escolhidas por `S(3)`.
    pub proximo: Vec<Vec<u8>>,
    pub roteiros: Vec<Roteiro>,
    pub nos_fixos: Vec<NoFixo>,
}

/// Vizinhos do placar das colônias, lidos **sem sinal** (quintal.md §10.1).
#[derive(Clone, Debug, Deserialize)]
pub struct TabColonias {
    pub vizinho_dx: [u8; 6],
    pub vizinho_dy: [u8; 6],
}

/// Constantes das regras (bloco `regras`; origem de cada uma em `_origem` no JSON).
#[derive(Clone, Debug, Deserialize)]
pub struct RegrasDados {
    pub mac_por_tick: i64,
    pub chuva_tecla: i16,
    pub chuva_chance_1: i16,
    pub chuva_chance_2: i16,
    pub chuva_duracao_min: i16,
    pub chuva_duracao_sorteio: u16,
    pub msg_menino_duracao: i32,
    pub msg_menino_limite: i16,
    pub gato_atraso: i32,
    pub gato_atraso_fugiu: i32,
    pub gato_chance: u16,
    pub gato_tecla: i16,
    pub passaro_atraso: i32,
    pub passaro_atraso_saiu: i32,
    pub latido_atraso: i32,
    pub latido_sorteio: u16,
    pub colonia_teto: i32,
    pub colonia_chance_reproduzir: u16,
    pub colonia_espalhar: u16,
    pub casa_limite: i16,
    pub nova_vermelha: u8,
    pub enxame_atraso_min: i16,
    pub enxame_teto: i16,
    pub inseticida_por_formiga: i32,
}

/// Valores de `_InitSimYard` (quintal.md §8).
#[derive(Clone, Debug, Deserialize)]
pub struct ValoresIniciais {
    pub menino_x: i16,
    pub menino_y: i16,
    pub menino_dir: i16,
    pub menino_giro: i16,
    pub menino_espera: i16,
    pub cachorro_x: i16,
    pub cachorro_y: i16,
    pub cachorro_giro: i16,
    pub passaro_quadro: i16,
    pub menino_msg: i16,
    pub ciclo_quintal: i16,
}

/// Mensagens (SHARED 4) e músicas de `_SimColonies` no Jogo Completo.
#[derive(Clone, Debug, Deserialize)]
pub struct Mensagens {
    pub vermelhas_eliminadas: u16,
    pub falta_vermelhas: u16,
    pub casa_tomada: u16,
    pub tome_a_casa: u16,
    pub derrota: u16,
    pub vitoria: u16,
    pub musica_vermelhas_eliminadas: u16,
    pub musica_casa_tomada: u16,
    pub musica_vitoria: u16,
}

/// Ids dos efeitos sonoros (objetos tipo 5 do banco SOUND, `_myBeginSound`) pedidos por esta parte
/// (docs/spec/regras/som.md). Seção opcional no JSON: um id ausente não toca (mods anteriores à API 0.8).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SonsQuintal {
    /// Latidos do `_MakeBark` pelo sorteio `S(4)` do cachorro (0x17, 0x15, 0x17, 0x16).
    pub latidos: Vec<u16>,
    /// Latido quando o cachorro acompanha o menino cortando a grama (0x15).
    pub latido_cortador: Option<u16>,
    pub gato: Option<u16>,
    pub passaro: Option<u16>,
    /// Pisada do menino (`_FootFall`, 0x23) e inseticida (9).
    pub pisada: Option<u16>,
    pub inseticida: Option<u16>,
    /// Menino dentro de casa (0x18) nos pares (contagem do nó, nó, quadro) do original.
    pub menino_casa: Option<u16>,
    pub menino_casa_quando: Vec<[i16; 3]>,
    /// Chuva (`_DoWater`, `S(50)` = 0; 0x29).
    pub chuva: Option<u16>,
}

/// Conteúdo de `data/quintal.json`.
#[derive(Clone, Debug, Deserialize)]
pub struct TabelasQuintal {
    /// `_Dx8`/`_Dy8` (passo do gato × 4).
    pub dx8: [i8; 8],
    pub dy8: [i8; 8],
    pub menino: TabMenino,
    pub cachorro: TabCachorro,
    pub casa: TabCasa,
    pub colonias: TabColonias,
    pub regras: RegrasDados,
    pub valores_iniciais: ValoresIniciais,
    pub mensagens: Mensagens,
    #[serde(default)]
    pub sons: SonsQuintal,
    /// Tabelas do tutorial e dos conselhos (docs/spec/regras/conselhos.md).
    pub conselhos: super::conselhos::TabelasConselhos,
}

impl TabelasQuintal {
    /// Lê `quintal.json` (com mods).
    pub fn carregar(r: &Recursos) -> Result<TabelasQuintal, String> {
        let t: TabelasQuintal = r.dados(ARQUIVO_TABELAS)?;
        t.validar()?;
        Ok(t)
    }

    pub fn de_json(texto: &str) -> Result<TabelasQuintal, String> {
        let t: TabelasQuintal = serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_TABELAS}: {e}"))?;
        t.validar()?;
        Ok(t)
    }

    /// Cópia embutida de `data/quintal.json` (sem mods), para `RegrasOriginais::default` e testes.
    pub fn padrao() -> &'static TabelasQuintal {
        static T: OnceLock<TabelasQuintal> = OnceLock::new();
        T.get_or_init(|| {
            TabelasQuintal::de_json(include_str!("../../../data/quintal.json")).expect("data/quintal.json embutido")
        })
    }

    fn validar(&self) -> Result<(), String> {
        let c = &self.casa;
        if c.proximo.len() != 3 || c.proximo.iter().any(|p| p.len() < c.duracao.len()) {
            return Err(format!("{ARQUIVO_TABELAS}: casa.proximo deve ter 3 tabelas do tamanho de casa.duracao"));
        }
        for r in &c.roteiros {
            let n = c.duracao.get(r.no as usize).map(|&d| d as usize + 1).unwrap_or(0);
            if n == 0 || r.quadro.len() < n || r.x.len() < n || r.y.len() < n {
                return Err(format!("{ARQUIVO_TABELAS}: roteiro do nó {} menor que a duração", r.no));
            }
        }
        // `gato_chance` é o K de um `Sk(K)` (`SRand16`): potência de 2 entre 2 e 256.
        let k = self.regras.gato_chance;
        if !(k.is_power_of_two() && (2..=256).contains(&k)) {
            return Err(format!("{ARQUIVO_TABELAS}: regras.gato_chance deve ser potência de 2 entre 2 e 256"));
        }
        if c.nos_fixos.iter().any(|f| f.no < 30) {
            return Err(format!("{ARQUIVO_TABELAS}: nós fixos começam em 30"));
        }
        Ok(())
    }

    fn roteiro(&self, no: i16) -> Option<&Roteiro> {
        self.casa.roteiros.iter().find(|r| r.no as i16 == no)
    }

    fn no_fixo(&self, no: i16) -> Option<&NoFixo> {
        self.casa.nos_fixos.iter().find(|f| f.no as i16 == no)
    }
}

impl Direcoes for TabelasQuintal {
    fn deslocamentos(&self) -> (&[i8; 8], &[i8; 8]) {
        (&self.dx8, &self.dy8)
    }
}

/// O que o quintal usa das tabelas de outras partes quando o menino mata a aranha ou a amarela.
#[derive(Clone, Copy)]
pub struct Vitimas<'a> {
    /// Tabelas da amarela, para `_YellowDeath`.
    pub amarela: &'a TabelasAmarela,
    /// Duração da agonia de `_KillSpider` (`inimigos.json`, `regras.aranha_agonia`).
    pub agonia_aranha: i16,
}

impl<'a> Vitimas<'a> {
    pub fn de(amarela: &'a TabelasAmarela, inimigos: &TabelasInimigos) -> Vitimas<'a> {
        Vitimas { amarela, agonia_aranha: inimigos.regras.aranha_agonia }
    }
}

// ---------------------------------------------------------------------------------------------
// Relógios
// ---------------------------------------------------------------------------------------------

/// Os relógios de parede que o quintal lê (quintal.md §1, "Relógios").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Relogios {
    /// `_gameCycles`: voltas do laço do temporizador (pássaro).
    pub ciclos: i64,
    /// `_MacTickCount`: 1/60 s (gato, latido, balão do menino).
    pub mac: i64,
}

impl Relogios {
    /// Relógios do port. O relógio do Mac do original é `GetTickCount() / 55 × 3` (tempo real,
    /// quintal.md "Relógios"): quando `Estado::relogio_mac` traz esse valor (vetor de tick), ele é usado.
    /// Senão, regra do port: com `Estado::ciclos_jogo` preenchido, ele é o `_gameCycles` e também o
    /// relógio do Mac; sem ele, `_gameCycles` = `_GameTime` e relógio do Mac = `_GameTime × mac_por_tick`.
    pub fn do_port(e: &Estado, t: &TabelasQuintal) -> Relogios {
        let (ciclos, mac) = if e.ciclos_jogo != 0 {
            let c = e.ciclos_jogo as i64;
            (c, c)
        } else {
            let tempo = e.partida.tempo as i64;
            (tempo, tempo * t.regras.mac_por_tick)
        };
        Relogios { ciclos, mac: e.relogio_mac.map_or(mac, i64::from) }
    }
}

/// Atrasos vindos de um save do original estão no relógio de outra sessão: um atraso maior que
/// "agora + o maior atraso que a regra grava" é tratado como vencido (decisão do port, quintal.md).
pub fn normalizar_atrasos(e: &mut Estado, t: &TabelasQuintal, rel: Relogios) {
    let r = &t.regras;
    let max_mac = r.gato_atraso.max(r.gato_atraso_fugiu).max(r.latido_atraso + r.latido_sorteio as i32) as i64;
    let max_ciclos = r.passaro_atraso.max(r.passaro_atraso_saiu) as i64;
    // Com um relógio de verdade (o do Mac no jogo ao vivo, ou o `_gameCycles`/relógio de um vetor do
    // original), o original compara os atrasos como estão, mesmo vindos de outra sessão (vetores
    // `tutorial`: `_CatDelay` 4712 do arquivo do tutorial com o relógio em ~3300 fica intacto). A
    // normalização fica só para o relógio sintético (sem nenhum dos dois).
    let mac_real = e.relogio_mac.is_some() || e.ciclos_jogo != 0;
    let b = &mut e.bichos;
    if !mac_real && b.gato_atraso as i64 > rel.mac + max_mac {
        b.gato_atraso = rel.mac as i32;
    }
    if !mac_real && b.cachorro_atraso_latido as i64 > rel.mac + max_mac {
        b.cachorro_atraso_latido = rel.mac as i32;
    }
    if b.passaro_atraso as i64 > rel.ciclos + max_ciclos {
        b.passaro_atraso = rel.ciclos as i32;
    }
}

// ---------------------------------------------------------------------------------------------
// Trait do tick
// ---------------------------------------------------------------------------------------------

impl RegrasQuintal for RegrasOriginais {
    fn simular_quintal(&mut self, e: &mut Estado) {
        let t = &self.quintal;
        let rel = Relogios::do_port(e, t);
        normalizar_atrasos(e, t, rel);
        simular_quintal_com(e, t, Vitimas::de(&self.amarela, &self.inimigos), rel);
    }

    fn simular_agua(&mut self, e: &mut Estado) {
        simular_agua(e, &self.quintal);
    }
}

// ---------------------------------------------------------------------------------------------
// `_InitSimYard` e `_DoSimYard`
// ---------------------------------------------------------------------------------------------

/// `_InitSimYard` (`1030:1378`, quintal.md §8).
pub fn iniciar_quintal(e: &mut Estado, t: &TabelasQuintal) {
    let v = &t.valores_iniciais;
    let m = &mut e.menino;
    m.boy_msg_cnt = 0;
    m.x = v.menino_x;
    m.y = v.menino_y;
    m.giro_contagem = v.menino_giro;
    m.espera = v.menino_espera;
    m.dir = v.menino_dir;
    m.mensagem_ligada = 0;
    m.px = 0;
    m.py = 0;
    m.quadro = 0;
    m.presente = 0;
    m.parado_contagem = 0;
    m.no_contagem = 0;
    m.no_numero = 0;
    m.pe_presente = 0;
    m.pe_alterna = 0;
    m.pe_x = 0;
    m.pe_y = 0;
    m.cortador_x = 0;
    m.cortador_y = 0;
    m.cortando_grama = 0;
    m.mensagem_offset = v.menino_msg;
    let b = &mut e.bichos;
    b.passaro_atraso = 0;
    b.gato_atraso = 0;
    b.cachorro_x = v.cachorro_x;
    b.cachorro_y = v.cachorro_y;
    b.cachorro_giro_contagem = v.cachorro_giro;
    b.passaro_ligado = 0;
    b.gato_ciclo = 0;
    b.gato_quadro = 0;
    b.gato_ligado = 0;
    b.cachorro_quadro = 0;
    b.cachorro_dir = 0;
    b.passaro_quadro = v.passaro_quadro;
    e.ambiente.chovendo = 0;
    e.preta.enxame_atraso = 0;
    e.vermelha.enxame_atraso = 0;
    e.ambiente.ciclo_quintal = v.ciclo_quintal;
}

/// `_DoSimYard` (`1030:14a8`, quintal.md §2), com relógios explícitos.
pub fn simular_quintal_com(e: &mut Estado, t: &TabelasQuintal, v: Vitimas, rel: Relogios) {
    let a = &mut e.ambiente;
    a.ciclo_quintal += 1;
    if a.ciclo_quintal > 0x3FF {
        a.ciclo_quintal = 0;
    }
    if e.menino.no_numero < 0x26 {
        menino_dentro(e, t, v, rel);
    } else {
        menino_fora(e, t, v, rel);
    }
    passaro(e, t, rel);
    gato(e, t, rel);
    if e.ambiente.casa_a_venda == 0 {
        cachorro(e, t, rel);
    }
    if e.ambiente.terreno != 1 {
        chuva(e, t, rel);
    }
    simular_colonias(e, t);
}

fn chuva(e: &mut Estado, t: &TabelasQuintal, rel: Relogios) {
    let r = &t.regras;
    if e.ambiente.chovendo == 0 {
        let comeca = e.tecla_global == r.chuva_tecla
            || (e.r.rrand(r.chuva_chance_1) == 0 && e.r.rrand(r.chuva_chance_2) == 0);
        if comeca {
            e.ambiente.chovendo = 1;
            e.ambiente.chuva_contagem = e.s.rand1(r.chuva_duracao_sorteio) as i16 + r.chuva_duracao_min;
            iniciar_agua(e);
            balao_menino(e, t, rel, 0);
        }
    } else {
        let a = &mut e.ambiente;
        if a.chuva_contagem > 0 {
            a.chuva_contagem -= 1;
        }
        if a.chuva_contagem < 1 {
            a.chovendo = 0;
        }
    }
}

/// Balão de fala nº `k` do menino (corpo de `_SendBoyMsg`).
fn balao_menino(e: &mut Estado, t: &TabelasQuintal, rel: Relogios, k: i16) {
    if k < t.regras.msg_menino_limite {
        e.menino.boy_msg_cnt = (rel.mac + t.regras.msg_menino_duracao as i64) as i32;
        e.menino.mensagem_ligada = 1;
        e.menino.mensagem_offset = k;
    }
}

fn no_quintal(x: i32, y: i32) -> bool {
    (0..LARGURA).contains(&x) && (0..ALTURA).contains(&y)
}

fn indice_patch(x: i32, y: i32) -> usize {
    (x * ALTURA + y) as usize
}

fn vista_do_quintal(e: &Estado) -> bool {
    e.vistas.modo_mapa == 0 && e.vistas.modo_quintal < 2
}

// ---------------------------------------------------------------------------------------------
// Menino lá fora (quintal.md §3)
// ---------------------------------------------------------------------------------------------

fn grama_por_cortar(e: &Estado, x: i32, y: i32) -> bool {
    (e.ambiente.grama[x as usize] as u16) & (1u16 << y) != 0
}

fn cortar_grama(e: &mut Estado, x: i32, y: i32) {
    let g = &mut e.ambiente.grama[x as usize];
    *g = ((*g as u16) & !(1u16 << y)) as i16;
}

fn menino_fora(e: &mut Estado, t: &TabelasQuintal, vt: Vitimas, rel: Relogios) {
    let ciclo = e.ambiente.ciclo_quintal;
    if e.menino.pe_presente != 0 && ciclo & 3 != 0 {
        return;
    }
    let tm = &t.menino;

    // Escolha da direção.
    if e.menino.cortando_grama == 0 {
        e.menino.giro_contagem -= 1;
        if e.menino.giro_contagem < 0 {
            e.menino.dir = (e.s.rand1(3) as i16 + e.menino.dir - 1) & 3;
            e.menino.giro_contagem = 4;
        }
    } else if (e.menino.ultimo_corte_x, e.menino.ultimo_corte_y) != (e.menino.px, e.menino.py) {
        let (px, py) = (e.menino.px as i32, e.menino.py as i32);
        let mut escolhida = None;
        for k in (0..4).rev() {
            let (vx, vy) = (px + tm.vizinho_dx[k] as i32, py + tm.vizinho_dy[k] as i32);
            if no_quintal(vx, vy) && grama_por_cortar(e, vx, vy) {
                cortar_grama(e, vx, vy);
                escolhida = Some(k as i16);
                break;
            }
        }
        e.menino.dir = match escolhida {
            Some(k) => k,
            None if px >= 11 => 0,
            None if py >= 15 => 1,
            None if py < 1 => {
                balao_menino(e, t, rel, 6);
                e.menino.presente = 4;
                2
            }
            None => ciclo & 3,
        };
    }

    // Estado.
    let (px, py) = (e.menino.px, e.menino.py);
    match e.menino.presente {
        1 => {
            if e.s.rand1(900) == 0 {
                balao_menino(e, t, rel, 1);
                e.menino.presente = 2;
            } else if e.s.rand1(600) == 0 || e.partida.tipo_jogo == 3 || e.partida.tipo_jogo == 0 {
                balao_menino(e, t, rel, 2);
                e.menino.presente = 5;
            } else {
                if e.s.rand1(200) == 0 {
                    let k = e.s.rand1(8) as i16;
                    balao_menino(e, t, rel, 7 + k);
                }
                if e.ambiente.chovendo != 0 {
                    e.menino.presente = 5;
                }
            }
        }
        2 => {
            if py < 12 {
                e.menino.dir = 2;
            } else if py == 12 {
                if px < 4 {
                    let m = &mut e.menino;
                    m.presente = 3;
                    m.ultimo_corte_x = 3;
                    m.ultimo_corte_y = 13;
                    m.cortando_grama = 1;
                    for (i, g) in e.ambiente.grama.iter_mut().enumerate() {
                        *g = if i < 3 { 0 } else { -1 };
                    }
                    m.dir = 2;
                    m.x = 0x7D;
                    m.y = 0xAA;
                } else {
                    e.menino.dir = 3;
                }
            } else {
                e.menino.dir = 0;
            }
        }
        3 => {
            if e.s.rand1(100) == 0 {
                let k = e.s.rand1(3) as i16;
                balao_menino(e, t, rel, 3 + k);
            }
            if e.s.rand1(200) == 0 {
                balao_menino(e, t, rel, 6);
                e.menino.presente = 4;
            }
            let m = &mut e.menino;
            m.ultimo_corte_x = m.px;
            m.ultimo_corte_y = m.py;
            m.cortando_grama = 1;
        }
        4 => {
            let m = &mut e.menino;
            if py < 12 {
                m.dir = 2;
            } else if py == 12 {
                if px < 4 {
                    m.presente = 5;
                    m.cortando_grama = 0;
                } else {
                    m.dir = 3;
                }
            } else {
                m.dir = 0;
            }
            m.ultimo_corte_x = m.px;
            m.ultimo_corte_y = m.py;
        }
        5 => {
            let m = &mut e.menino;
            if py < 5 {
                m.dir = 2;
            } else if py > 5 {
                m.dir = 0;
            } else if px > 3 {
                m.dir = 3;
            } else {
                m.presente = 1;
            }
        }
        _ => {}
    }

    // Quadro e passo.
    let dir = (e.menino.dir & 3) as usize;
    e.menino.quadro = tm.quadro_por_dir[dir] as i16 + tm.quadro_por_passo[(ciclo & 3) as usize] as i16;
    let anda = if e.menino.cortando_grama == 0 {
        if e.menino.parado_contagem == 0 {
            if e.s.mascara(32) == 0 {
                e.menino.parado_contagem = 20;
            }
            true
        } else {
            e.menino.parado_contagem -= 1;
            e.menino.quadro = e.menino.dir + 20;
            false
        }
    } else {
        e.menino.quadro += 100;
        true
    };
    if anda {
        e.menino.x += tm.passo_x[dir] as i16;
        e.menino.y += tm.passo_y[dir] as i16;
    }
    let a = e.menino.y as i32 + e.menino.x as i32 - 200;
    let b = e.menino.y as i32 - 38;
    e.menino.px = (a / 28).clamp(0, 11) as i16;
    e.menino.py = (b / 10).clamp(0, 15) as i16;

    // Bordas (só sem cortador).
    if e.menino.cortando_grama == 0 {
        let m = &mut e.menino;
        if m.py < 1 {
            m.dir = 2;
        } else if m.py > 14 {
            m.dir = 0;
        }
        if m.px >= 11 {
            m.dir = 3;
        } else if m.py < 5 {
            if m.px <= 3 {
                m.dir = 1;
            }
        } else if m.px <= 2 {
            m.dir = 0;
            if m.py == 5 {
                if m.presente != 3 {
                    m.no_numero = 0x15;
                } else {
                    m.dir = 1;
                }
            }
        }
    }

    // Pé e cortador.
    e.menino.pe_presente = 0;
    e.menino.pe_alterna = (e.menino.pe_alterna == 0) as i16;
    if (e.menino.px, e.menino.py) == e.ambiente.patch {
        e.menino.pe_presente = 1;
        posicionar_pe(e, a, b);
        let d = (e.menino.dir & 3) as usize;
        e.menino.cortador_x = e.menino.pe_x + tm.corte_dx[d];
        e.menino.cortador_y = e.menino.pe_y + tm.corte_dy[d];
        alternar_pe(e);
        e.pedir_som(t.sons.pisada);
        pisada(e, vt, e.menino.pe_x as i32, e.menino.pe_y as i32);
        if e.menino.cortando_grama != 0 && e.partida.tipo_jogo != 0 {
            cortador_no_patch(e, vt);
        }
    } else if e.menino.cortando_grama != 0 {
        let i = indice_patch(e.menino.px as i32, e.menino.py as i32);
        for cor in [Cor::Preta, Cor::Vermelha] {
            let c = e.colonia_mut(cor);
            let v = c.populacao_quintal[i] as i32;
            if v != 0 {
                let n = v - (v >> 2);
                if n == 0 {
                    c.colonias_extintas += 1;
                }
                c.populacao_quintal[i] = n as u8;
            }
        }
    }
}

/// `_FootX/_FootY` a partir de `a = X + Y − 200` e `b = Y − 38` (resto com o sinal do dividendo).
fn posicionar_pe(e: &mut Estado, a: i32, b: i32) {
    e.menino.pe_x = (((a % 28) * 4 + 6) & 0x7F) as i16;
    e.menino.pe_y = (((b % 10 + 1) * 6) & 0x3F) as i16;
}

fn alternar_pe(e: &mut Estado) {
    let d = if e.menino.pe_alterna != 0 { 6 } else { -6 };
    if e.menino.dir & 1 == 0 {
        e.menino.pe_x += d;
    } else {
        e.menino.pe_y += d;
    }
}

/// `_FootFall(x, y)` (`1030:3386`, quintal.md §3.1).
fn pisada(e: &mut Estado, vt: Vitimas, x: i32, y: i32) {
    let (x1, y1) = if e.menino.dir & 1 == 0 { (x + 6, y + 16) } else { (x + 16, y + 6) };
    for cx in x..x1 {
        for cy in y..y1 {
            // Índice cx·64 + cy com cy até 127: lê a coluna seguinte, como o original.
            if !(0..=127).contains(&cx) || !(0..128).contains(&cy) {
                continue;
            }
            let k = (cx * 64 + cy) as usize;
            let Some(&v) = e.superficie.vida.dados.get(k) else { continue };
            if v == 0 {
                continue;
            }
            if comum::e_amarela(v) {
                amarela::matar_amarela(e, vt.amarela, CausaMorte::Pisada);
            } else if let Some(j) = achar_na_lista_a(e, cx, cy) {
                let vermelha = comum::e_vermelha(e.superficie.lista.vagas[j].tipo);
                comum::cadaver_aqui(e, cx, cy, vermelha);
                e.superficie.lista.vagas[j].tipo = 0;
            }
        }
    }
    let (sx, sy) = (e.aranha.destino_x as i32, e.aranha.destino_y as i32);
    if x <= sx && sx < x1 && y <= sy && sy < y1 {
        comum::matar_aranha(e, vt.agonia_aranha);
        if e.amarela.modo == 1 {
            amarela::matar_amarela(e, vt.amarela, CausaMorte::Pisada);
        }
    }
}

/// Corpo de `_MowerFall` (`1030:34da`, quintal.md §3.2).
fn cortador_no_patch(e: &mut Estado, vt: Vitimas) {
    for j in (0..e.superficie.lista.usadas()).rev() {
        let f = e.superficie.lista.vagas[j];
        if f.tipo != 0 && e.s.mascara(4) != 0 {
            e.superficie.vida.definir(f.x as i32, f.y as i32, 0);
            e.superficie.lista.vagas[j].tipo = 0;
        }
    }
    if e.aranha.ligada != 0 && e.s.mascara(4) != 0 {
        comum::matar_aranha(e, vt.agonia_aranha);
    }
    if e.amarela.modo < 2 && e.amarela.plano == 1 && e.s.mascara(4) != 0 {
        amarela::matar_amarela(e, vt.amarela, CausaMorte::Cortador);
    }
}

// ---------------------------------------------------------------------------------------------
// Menino dentro de casa (quintal.md §4)
// ---------------------------------------------------------------------------------------------

fn menino_dentro(e: &mut Estado, t: &TabelasQuintal, vt: Vitimas, rel: Relogios) {
    if e.ambiente.casa_a_venda == 1 {
        return;
    }
    let casa = &t.casa;
    if e.menino.no_numero < 30 {
        e.menino.no_contagem += 1;
        let dur = casa.duracao.get(e.menino.no_numero.max(0) as usize).copied().unwrap_or(0) as i16;
        if e.menino.no_contagem > dur {
            e.menino.no_contagem = 0;
            let k = e.s.rand1(3) as usize;
            let proximo = casa.proximo[k].get(e.menino.no_numero.max(0) as usize).copied().unwrap_or(0) as i16;
            e.menino.no_numero = proximo;
            match proximo {
                0x21 => balao_menino(e, t, rel, 20),
                6 => balao_menino(e, t, rel, 21),
                7 => balao_menino(e, t, rel, 22),
                14 => {
                    let k = e.s.rand1(3) as i16;
                    balao_menino(e, t, rel, 15 + k);
                }
                16 => {
                    let k = e.s.rand1(2) as i16;
                    balao_menino(e, t, rel, 18 + k);
                }
                _ => {}
            }
        }
    }
    let no = e.menino.no_numero;
    let passo = if (30..=38).contains(&no) {
        t.no_fixo(no).map(|f| {
            let p = (f.quadro, f.x as i16, f.y as i16);
            if f.chance > 0 && e.s.rand1(f.chance as u16) == 0 {
                e.menino.no_numero = f.proximo as i16;
                e.menino.no_contagem = 0;
            }
            p
        })
    } else {
        t.roteiro(no).and_then(|r| {
            let i = e.menino.no_contagem.max(0) as usize;
            Some((*r.quadro.get(i)?, *r.x.get(i)? as i16, *r.y.get(i)? as i16))
        })
    };
    let no = e.menino.no_numero;
    if no == 0x26 {
        e.menino.dir = 1;
    }
    e.menino.presente = (no == 0x26) as i16;
    if no == 20 && e.menino.no_contagem > 2 {
        e.menino.presente = 1;
    }
    if no == 21 && e.menino.no_contagem < 3 {
        e.menino.presente = 1;
    }
    // Nós sem tabela (16–19, 22–29) não são alcançáveis; o original usaria variáveis sem valor.
    if let Some((quadro, x, y)) = passo {
        e.menino.x = x;
        e.menino.y = y;
        e.menino.quadro = quadro;
    }
    let chave = [e.menino.no_contagem, e.menino.no_numero, e.menino.quadro];
    if vista_do_quintal(e) && t.sons.menino_casa_quando.contains(&chave) {
        e.pedir_som(t.sons.menino_casa);
    }
    let a = e.menino.y as i32 + e.menino.x as i32 - 200;
    let b = e.menino.y as i32 - 38;
    e.menino.px = (a / 28) as i16;
    e.menino.pe_presente = 0;
    e.menino.pe_alterna = (e.menino.pe_alterna == 0) as i16;
    e.menino.py = (b / 10) as i16;
    let (cx, cy) = e.ambiente.patch;
    if (e.menino.px, e.menino.py) != (cx, cy) {
        return;
    }
    if matches!(cx as i32 * 16 + cy as i32, 0 | 1 | 16 | 32) {
        return;
    }
    let q = e.menino.quadro;
    e.menino.dir = if (0..=11).contains(&q) { t.menino.dir_do_quadro[q as usize] as i16 } else { 2 };
    e.menino.pe_presente = 1;
    posicionar_pe(e, a, b);
    alternar_pe(e);
    e.pedir_som(t.sons.pisada);
    pisada(e, vt, e.menino.pe_x as i32, e.menino.pe_y as i32);
    // Inseticida (`_SimKidInside` toca o som 9 depois do `_FootFall`).
    e.pedir_som(t.sons.inseticida);
    let n = (e.vermelha.populacao as i32 + e.preta.populacao as i32) * t.regras.inseticida_por_formiga;
    for _ in 0..n.max(0) {
        let x = e.s.mascara(128);
        let y = e.s.mascara(64);
        if e.superficie.vida.em(x, y).unwrap_or(0) != 0 {
            let j = achar_na_lista_a(e, x, y);
            e.tindex = j.map(|j| j as i32).unwrap_or(-1);
            if let Some(j) = j {
                let vermelha = comum::e_vermelha(e.superficie.lista.vagas[j].tipo);
                comum::cadaver_aqui(e, x, y, vermelha);
                e.superficie.lista.vagas[j].tipo = 0;
                e.superficie.vida.definir(x, y, 0);
            }
        }
        if (e.aranha.x as i32 >> 4, e.aranha.y as i32 >> 4) == (x, y) {
            comum::matar_aranha(e, vt.agonia_aranha);
        }
    }
    if e.partida.tipo_jogo == 2 && e.amarela.plano == 1 && e.s.mascara(4) == 0 {
        amarela::matar_amarela(e, vt.amarela, CausaMorte::Inseticida);
    }
}

// ---------------------------------------------------------------------------------------------
// Cachorro, gato e pássaro (quintal.md §5–7)
// ---------------------------------------------------------------------------------------------

fn cachorro(e: &mut Estado, t: &TabelasQuintal, rel: Relogios) {
    let ciclo = e.ambiente.ciclo_quintal;
    e.bichos.cachorro_giro_contagem -= 1;
    if e.bichos.cachorro_giro_contagem < 0 {
        let (cpx, cpy) = (e.bichos.cachorro_px, e.bichos.cachorro_py);
        let d = if e.bichos.gato_ligado != 0 {
            if cpx < 5 {
                1
            } else if cpx > 8 {
                3
            } else if cpy > 0 {
                0
            } else {
                ciclo & 3
            }
        } else if e.menino.no_numero < 0x26 {
            (e.s.rand1(3) as i16 + e.bichos.cachorro_dir - 1) & 3
        } else {
            let dy = cpy - e.menino.py;
            let dx = cpx - e.menino.px;
            if dx.abs() < 1
                && dy.abs() < 1
                && e.menino.cortando_grama != 0
                && vista_do_quintal(e)
                && rel.mac > e.bichos.cachorro_atraso_latido as i64
            {
                e.pedir_som(t.sons.latido_cortador);
                latir(e, t, rel);
            }
            if dx.abs() > 1 || dy.abs() > 1 {
                if dy < 0 {
                    2
                } else if dy > 1 {
                    0
                } else if dx < 0 {
                    1
                } else if dx > 1 {
                    3
                } else {
                    ciclo & 3
                }
            } else {
                ciclo & 3
            }
        };
        e.bichos.cachorro_dir = d;
        e.bichos.cachorro_giro_contagem = 4;
    }
    if e.bichos.cachorro_dir == 1 && e.s.rand1(10) == 0 {
        e.bichos.cachorro_dir = e.s.mascara(2) as i16 + 4;
        e.bichos.cachorro_giro_contagem = 6;
    }
    let tc = &t.cachorro;
    let b = &mut e.bichos;
    let d = b.cachorro_dir.clamp(0, 5) as usize;
    b.cachorro_quadro = if d < 4 {
        tc.quadro_por_passo[(ciclo & 3) as usize] as i16 + tc.quadro_por_dir[d] as i16
    } else {
        tc.quadro_por_dir[d] as i16 + (ciclo & 1)
    };
    b.cachorro_y += tc.passo_y[d] as i16;
    b.cachorro_py = ((b.cachorro_y as i32 - 38) / 10) as i16;
    b.cachorro_x += tc.passo_x[d] as i16;
    b.cachorro_px = ((b.cachorro_y as i32 + b.cachorro_x as i32 - 200) / 28) as i16;
    b.cachorro_py = b.cachorro_py.clamp(0, 15);
    b.cachorro_px = b.cachorro_px.clamp(0, 11);
    if b.cachorro_py < 1 {
        b.cachorro_dir = 2;
    } else if b.cachorro_py > 14 {
        b.cachorro_dir = 0;
    }
    if b.cachorro_px >= 11 {
        b.cachorro_dir = 3;
    } else if b.cachorro_px < 4 {
        b.cachorro_dir = 1;
    }
    if (100..=103).contains(&b.cachorro_quadro) && e.s.rand1(6) == 0 {
        // `_MakeBark(k)`: o sorteio escolhe o latido (0x17, 0x15, 0x17, 0x16).
        let k = e.s.mascara(4) as usize;
        if vista_do_quintal(e) && rel.mac > e.bichos.cachorro_atraso_latido as i64 {
            e.pedir_som(t.sons.latidos.get(k).copied());
            latir(e, t, rel);
        }
    }
}

/// Som de latido e novo atraso (`_MakeBark` embutido): `relógio + 60 + S(30)`.
fn latir(e: &mut Estado, t: &TabelasQuintal, rel: Relogios) {
    let r = &t.regras;
    let s = e.s.rand1(r.latido_sorteio) as i64;
    e.bichos.cachorro_atraso_latido = (rel.mac + s + r.latido_atraso as i64) as i32;
}

fn gato(e: &mut Estado, t: &TabelasQuintal, rel: Relogios) {
    let r = &t.regras;
    if e.bichos.gato_ligado == 0 {
        if rel.mac <= e.bichos.gato_atraso as i64 {
            return;
        }
        e.bichos.gato_atraso = (rel.mac + r.gato_atraso as i64) as i32;
        if e.s.mascara(r.gato_chance) != 0 && e.tecla_global != r.gato_tecla {
            return;
        }
        let b = &mut e.bichos;
        b.gato_ligado = 1;
        b.gato_x = 0xFC;
        b.gato_y = 0x19;
        b.gato_dir = 2;
        b.gato_quadro = 0;
        b.gato_ciclo = 0;
        return;
    }
    e.bichos.gato_ciclo = (e.bichos.gato_ciclo + 1) & 0xFFF;
    match e.bichos.gato_ligado {
        1 => {}
        2 => {
            let b = &mut e.bichos;
            b.gato_quadro += 1;
            if b.gato_quadro > 12 {
                b.gato_quadro = 11;
            }
            b.gato_ciclo += 1;
            if b.gato_ciclo >= 31 {
                b.gato_ciclo = 0;
                b.gato_ligado = 1;
                b.gato_quadro = if b.gato_dir == 2 { 0 } else { 3 };
            }
            return;
        }
        _ => {
            let b = &mut e.bichos;
            if b.gato_quadro + 1 < 30 {
                b.gato_quadro += 1;
            } else {
                b.gato_quadro = 0;
                b.gato_ligado = 0;
                b.gato_atraso = (rel.mac + r.gato_atraso_fugiu as i64) as i32;
            }
            return;
        }
    }
    let d = (e.bichos.gato_dir & 7) as usize;
    let nx = e.bichos.gato_x as i32 + 4 * t.dx8[d] as i32;
    let ny = e.bichos.gato_y as i32 + 4 * t.dy8[d] as i32;
    if e.s.mascara(16) == 0 || !(0xFC..=0x1EF).contains(&nx) {
        let b = &mut e.bichos;
        if b.gato_dir == 2 {
            b.gato_dir = 6;
            b.gato_quadro = 4;
        } else {
            b.gato_dir = 2;
            b.gato_quadro = 1;
        }
    } else {
        let b = &mut e.bichos;
        b.gato_x = nx as i16;
        b.gato_y = ny as i16;
        b.gato_quadro += 1;
        if b.gato_dir == 2 {
            if b.gato_quadro > 2 {
                b.gato_quadro = 1;
            }
        } else if b.gato_quadro > 5 {
            b.gato_quadro = 4;
        }
    }
    if vista_do_quintal(e) && e.s.mascara(64) == 0 {
        e.pedir_som(t.sons.gato);
        return;
    }
    if e.s.mascara(16) == 0 && e.bichos.gato_ciclo > 100 && e.bichos.cachorro_dir == 2 && e.menino.presente == 0 {
        let b = &mut e.bichos;
        b.gato_ligado = 2;
        b.gato_ciclo = 0;
        b.gato_quadro = 10;
        return;
    }
    let b = &e.bichos;
    if b.gato_ciclo > 50 {
        let dx = b.gato_x as i64 - b.cachorro_x as i64;
        let dy = b.gato_y as i64 - b.cachorro_y as i64;
        if dx * dx + dy * dy < 2401 {
            let b = &mut e.bichos;
            b.gato_ligado = 3;
            b.gato_quadro = 20;
        }
    }
}

fn passaro(e: &mut Estado, t: &TabelasQuintal, rel: Relogios) {
    let r = &t.regras;
    if e.bichos.passaro_ligado == 0 {
        if rel.ciclos > e.bichos.passaro_atraso as i64 {
            e.bichos.passaro_atraso = (rel.ciclos + r.passaro_atraso as i64) as i32;
            let (x, y) = e.ambiente.patch;
            if x > 4 && e.s.mascara(2) == 0 && (e.preta.enxame_contagem > 0 || e.vermelha.enxame_contagem > 0) {
                let b = &mut e.bichos;
                b.passaro_alvo_x = x * 28 - y * 10 + 178;
                b.passaro_alvo_y = y * 10 + 46;
                b.passaro_ligado = 1;
                b.passaro_x = 0;
                b.passaro_y = e.s.mascara(64) as i16 + 4;
                b.passaro_quadro = 0;
            }
        }
        return;
    }
    let b = &mut e.bichos;
    b.passaro_x += if b.passaro_ligado == 1 { 16 } else { 8 };
    if b.passaro_ligado == 1 {
        let d = b.passaro_alvo_y - b.passaro_y;
        if d != 0 {
            let passo = if d.abs() < 4 { 1 } else { 4 };
            b.passaro_y += d.signum() * passo;
        }
    } else {
        b.passaro_y -= 8;
    }
    b.passaro_quadro = (b.passaro_quadro + 1) & 1;
    if (0..512).contains(&b.passaro_x) && (0..256).contains(&b.passaro_y) {
        if b.passaro_ligado == 1 && b.passaro_x >= b.passaro_alvo_x && (b.passaro_y - b.passaro_alvo_y).abs() <= 4 {
            for c in [&mut e.preta, &mut e.vermelha] {
                for v in [&mut c.enxame_contagem, &mut c.rainhas_guardadas] {
                    if *v > 0 {
                        *v -= (*v + 7) >> 3;
                    }
                }
            }
            if vista_do_quintal(e) && e.s.mascara(2) == 0 {
                e.pedir_som(t.sons.passaro);
            }
            e.bichos.passaro_ligado = 2;
        }
    } else {
        b.passaro_ligado = 0;
        b.passaro_atraso = (rel.ciclos + r.passaro_atraso_saiu as i64) as i32;
    }
}

// ---------------------------------------------------------------------------------------------
// Colônias do quintal (quintal.md §10)
// ---------------------------------------------------------------------------------------------

/// `_MaintainSwarm` (corpo embutido em `_SimColonies`): decaimento da revoada de uma cor.
pub fn manter_enxame(e: &mut Estado, cor: Cor, teto: i16) {
    let c = e.colonia_mut(cor);
    if c.enxame_contagem > 0 {
        c.enxame_contagem -= if c.enxame_contagem < 4 { 1 } else { c.enxame_contagem >> 2 };
    }
    c.enxame_contagem = c.enxame_contagem.max(c.rainhas_guardadas).min(teto);
}

/// `_GetNearbyPatches(px, py)` (`1030:3ce4`): placar com os deslocamentos lidos sem sinal.
pub fn placar_vizinhos(e: &Estado, t: &TabelasQuintal, px: i32, py: i32) -> i32 {
    let mut p = 0;
    for k in 0..6 {
        let vx = px + t.colonias.vizinho_dx[k] as i32;
        let vy = py + t.colonias.vizinho_dy[k] as i32;
        if no_quintal(vx, vy) {
            let i = indice_patch(vx, vy);
            if e.preta.populacao_quintal[i] != 0 {
                p += 3;
            }
            if e.vermelha.populacao_quintal[i] != 0 {
                p -= 3;
            }
        }
    }
    p
}

/// `_Reproduce(px, py, cor)` (`1030:3d4c`): uma colônia nova (ou +1) a até ±3 patches.
pub fn reproduzir(e: &mut Estado, t: &TabelasQuintal, px: i32, py: i32, cor: Cor) {
    let n = t.regras.colonia_espalhar;
    let x = (e.s.sgs_rand(n) + px).clamp(0, LARGURA - 1);
    let y = (e.s.sgs_rand(n) + py).clamp(0, ALTURA - 1);
    if (x, y) != (px, py) {
        let c = e.colonia_mut(cor);
        let i = indice_patch(x, y);
        if c.populacao_quintal[i] == 0 {
            c.colonias_fundadas += 1;
        }
        c.populacao_quintal[i] = c.populacao_quintal[i].wrapping_add(1);
    }
}

/// `_SimColonies` (`1030:35f8`): só age com `_YardCycle & 0x1F == 0`.
pub fn simular_colonias(e: &mut Estado, t: &TabelasQuintal) {
    if e.ambiente.ciclo_quintal & 0x1F != 0 {
        return;
    }
    let r = &t.regras;
    // `_LastColonyPopR` não está no save: é o `_ColonyTotalRed` da passada anterior.
    let ultima_vermelha = e.vermelha.total_colonias;
    e.ambiente.atualizar_colonias = 1;
    manter_enxame(e, Cor::Preta, r.enxame_teto);
    manter_enxame(e, Cor::Vermelha, r.enxame_teto);
    e.preta.total_colonias = (e.preta.populacao > 0) as i16;
    e.vermelha.total_colonias = (e.vermelha.populacao > 0) as i16;
    e.ambiente.colonias_na_casa = 0;
    let (ax, ay) = (e.ambiente.patch.0 as i32, e.ambiente.patch.1 as i32);
    let atual = indice_patch(ax, ay);
    let teto = r.colonia_teto;
    for cor in [Cor::Preta, Cor::Vermelha] {
        let c = e.colonia_mut(cor);
        c.populacao_quintal[atual] = ((c.populacao & 0x3FF) as i32).min(teto) as u8;
    }
    for px in 0..LARGURA {
        let mut py = 0;
        while py < ALTURA {
            if (px, py) == (ax, ay) {
                py += 1;
                continue;
            }
            let i = indice_patch(px, py);
            let b = e.preta.populacao_quintal[i] as i32;
            let v = e.vermelha.populacao_quintal[i] as i32;
            if b == 0 && v == 0 {
                py += 1;
                continue;
            }
            if e.partida.tipo_jogo == 2 && (px < 2 || (px == 3 && py < 5)) {
                e.ambiente.colonias_na_casa += 1;
            }
            let p = placar_vizinhos(e, t, px, py);
            if b != 0 {
                e.preta.total_colonias += 1;
                let n = if p == 0 { b + 1 } else { b + p };
                if n < 1 {
                    e.preta.colonias_extintas += 1;
                    e.preta.populacao_quintal[i] = 0;
                } else if n < teto {
                    e.preta.populacao_quintal[i] = n as u8;
                } else {
                    e.preta.populacao_quintal[i] = teto as u8;
                    if e.s.rand1(r.colonia_chance_reproduzir) == 0 {
                        reproduzir(e, t, px, py, Cor::Preta);
                    }
                }
            }
            if v != 0 {
                e.vermelha.total_colonias += 1;
                let n = if p == 0 { v + 1 } else { v - p };
                if n < 1 {
                    e.vermelha.colonias_extintas += 1;
                    e.vermelha.populacao_quintal[i] = 0;
                    if px == 11 {
                        // Desmontagem `1030:392d`: `py` do laço passa a 8 (continua em 9).
                        py = 8;
                        let k = e.s.rand1(6);
                        e.vermelha.populacao_quintal[indice_patch(2 + k, 0)] = r.nova_vermelha;
                    }
                } else if n < teto {
                    e.vermelha.populacao_quintal[i] = n as u8;
                } else {
                    e.vermelha.populacao_quintal[i] = teto as u8;
                    if e.s.rand1(r.colonia_chance_reproduzir) == 0 {
                        reproduzir(e, t, px, py, Cor::Vermelha);
                    }
                }
            }
            py += 1;
        }
    }
    // Revoada guardada: preta só com `_LayDownQueenMode == 1`.
    for cor in [Cor::Preta, Cor::Vermelha] {
        if cor == Cor::Preta && e.controles.modo_por_rainha != 1 {
            continue;
        }
        e.colonia_mut(cor).enxame_atraso += 1;
        let s = e.s.mascara(8) as i16;
        if s + r.enxame_atraso_min < e.colonia(cor).enxame_atraso {
            e.colonia_mut(cor).enxame_atraso = 0;
            while e.colonia(cor).rainhas_guardadas > 0 {
                reproduzir(e, t, ax, ay, cor);
                e.colonia_mut(cor).rainhas_guardadas -= 1;
            }
        }
    }
    if e.partida.tipo_jogo == 2 {
        jogo_completo(e, t, ultima_vermelha);
    }
}

/// Mensagens, casa tomada, vitória e derrota do Jogo Completo (fim de `_SimColonies`).
fn jogo_completo(e: &mut Estado, t: &TabelasQuintal, ultima_vermelha: i16) {
    let m = &t.mensagens;
    if e.vermelha.total_colonias == 0 && ultima_vermelha != 0 {
        e.eventos.push(Evento::Musica(m.musica_vermelhas_eliminadas));
        e.eventos.push(Evento::Mensagem(m.vermelhas_eliminadas));
        if e.ambiente.casa_a_venda == 0 {
            e.eventos.push(Evento::Mensagem(m.tome_a_casa));
        }
    }
    if e.ambiente.casa_a_venda == 0 && e.ambiente.colonias_na_casa > t.regras.casa_limite {
        e.ambiente.casa_a_venda = 1;
        e.menino.no_numero = 0;
        e.menino.presente = 0;
        e.menino.quadro = -1;
        e.eventos.push(Evento::Musica(m.musica_casa_tomada));
        e.eventos.push(Evento::Mensagem(m.casa_tomada));
        if e.vermelha.total_colonias != 0 {
            e.eventos.push(Evento::Mensagem(m.falta_vermelhas));
        }
    }
    if e.ambiente.casa_a_venda != 0 && e.vermelha.total_colonias == 0 {
        e.eventos.push(Evento::Musica(m.musica_vitoria));
        e.eventos.push(Evento::Mensagem(m.vitoria));
        e.partida.fim_de_jogo = 1;
        e.partida.pretas_venceram = 1;
        return;
    }
    let c = &e.preta.castas;
    if e.preta.total_colonias < 2 && c[5] == 0 && c[0] == 0 && c[3] == 0 && c[4] == 0 {
        e.partida.fim_de_jogo = 1;
        e.partida.pretas_venceram = 0;
        e.eventos.push(Evento::Mensagem(m.derrota));
    }
}

/// `_StartMigrate(X, Y)` (`1030:3df2`): origem de uma migração, ou `None` (`_MigrateX = −1`).
pub fn iniciar_migracao(e: &Estado, x: i32, y: i32) -> Option<(i32, i32)> {
    let (mx, my) = ((y + x - 0xEE) / 28, (y - 0x42) / 10);
    (no_quintal(mx, my) && e.preta.populacao_quintal[indice_patch(mx, my)] != 0).then_some((mx, my))
}

/// `_EndMigrate(X, Y)` (`1030:3e6c`): leva metade da colônia preta da origem ao destino.
pub fn terminar_migracao(e: &mut Estado, origem: Option<(i32, i32)>, x: i32, y: i32, teto: i32) {
    let Some((ox, oy)) = origem else { return };
    let (dx, dy) = ((y + x - 0xEE) / 28, (y - 0x42) / 10);
    if !no_quintal(dx, dy) {
        return;
    }
    let p = &mut e.preta.populacao_quintal;
    let o = indice_patch(ox, oy);
    let h = p[o] >> 1;
    p[o] -= h;
    let d = indice_patch(dx, dy);
    p[d] = (p[d] as i32 + h as i32).min(teto) as u8;
}

// ---------------------------------------------------------------------------------------------
// Água (`_DoWater`, ninho.md §8; quintal.md §9)
// ---------------------------------------------------------------------------------------------

/// `_DoWater` (`1020:07c6`), todo tick.
pub fn simular_agua(e: &mut Estado, t: &TabelasQuintal) {
    if e.ambiente.terreno != 0 {
        return;
    }
    if e.ambiente.chovendo != 0 {
        if e.s.rand1(50) == 0 {
            e.pedir_som(t.sons.chuva);
        }
        e.ambiente.limpar_chuva = 1;
        if e.s.rand1(10) == 0 && e.ambiente.lencol > 4 {
            e.ambiente.lencol -= 1;
            let w = e.ambiente.lencol as i32;
            afogar(e, Lista::B, w);
            afogar(e, Lista::R, w);
            for x in 0..64 {
                for mapa in [&mut e.ninho_b.mapa, &mut e.ninho_r.mapa] {
                    let v = mapa.em(x, w).unwrap_or(0);
                    mapa.definir(x, w, if v < 0x20 { 0x4E } else { v.wrapping_add(0x2F) });
                }
            }
        }
        for i in 0..100 {
            let (x, y) = (e.ambiente.gotas_x[i] as i32, e.ambiente.gotas_y[i] as i32);
            let v = e.superficie.mapa.em(x, y).unwrap_or(0);
            if (0x74..=0x76).contains(&v) {
                e.superficie.mapa.definir(x, y, v + 1);
            } else {
                if v == 0x77 {
                    let s = e.s.rand1(14) as u8;
                    e.superficie.mapa.definir(x, y, s);
                }
                por_gota(e, i);
            }
        }
        return;
    }
    if e.ambiente.limpar_chuva == 1 {
        e.ambiente.limpar_chuva = 0;
        for i in 0..100 {
            let (x, y) = (e.ambiente.gotas_x[i] as i32, e.ambiente.gotas_y[i] as i32);
            let v = e.superficie.mapa.em(x, y).unwrap_or(0);
            if (0x74..0x78).contains(&v) {
                let s = e.s.rand1(14) as u8;
                e.superficie.mapa.definir(x, y, s);
            }
        }
    }
    if e.ambiente.lencol < 64 && e.s.rand1(10) == 0 {
        let w = e.ambiente.lencol as i32;
        for x in 0..64 {
            for cor in [Cor::Preta, Cor::Vermelha] {
                let v = e.ninho(cor).mapa.em(x, w).unwrap_or(0);
                let n = if v == 0x4E { e.s.rand1(8) as u8 } else { v.wrapping_sub(0x2F) };
                e.ninho_mut(cor).mapa.definir(x, w, n);
            }
        }
        e.ambiente.lencol += 1;
    }
}

/// `_DrownBList/_DrownRList(w)` (ninho.md §8.2): M = 17 nas formigas da linha w com grupo 1..11.
fn afogar(e: &mut Estado, area: Lista, w: i32) {
    let l = e.lista_mut(area);
    for j in (0..l.usadas()).rev() {
        let f = &mut l.vagas[j];
        let grupo = (f.tipo & 0x78) >> 3;
        if f.y as i32 == w && f.tipo != 0 && (1..12).contains(&grupo) {
            f.modo = 0x11;
        }
    }
}

/// `_PlaceDrop(i)` (`1020:0acc`): nova posição da gota i (R(128), R(64)) e poça nova no chão.
pub fn por_gota(e: &mut Estado, i: usize) {
    let x = e.r.rrand(128);
    let y = e.r.rrand(64);
    e.ambiente.gotas_x[i] = x as u8;
    e.ambiente.gotas_y[i] = y as u8;
    if e.superficie.mapa.em(x, y).unwrap_or(0xFF) < 0x0E {
        e.superficie.mapa.definir(x, y, 0x74);
        let (cx, cy) = (x >> 1, y >> 1);
        let f = &mut e.feromonios;
        f.alarme.definir(cx, cy, 0);
        f.f.definir(cx, cy, 0);
        f.trilha_b.definir(cx, cy, 0);
        f.trilha_r.definir(cx, cy, 0);
        for g in [&mut f.ninho_b, &mut f.ninho_r] {
            let v = g.em(cx, cy).unwrap_or(0);
            g.definir(cx, cy, v.saturating_sub(20));
        }
    }
}

/// `_InitWater` (`1020:0b76`): sorteia as 100 gotas.
pub fn iniciar_agua(e: &mut Estado) {
    for i in 0..100 {
        por_gota(e, i);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn t() -> &'static TabelasQuintal {
        TabelasQuintal::padrao()
    }

    #[test]
    fn tabelas_embutidas_carregam() {
        let t = t();
        assert_eq!(t.menino.passo_x, [4, 6, -4, -6]);
        assert_eq!(t.colonias.vizinho_dx, [0, 1, 0, 255, 0, 0]);
        assert_eq!(t.casa.roteiros.len(), 18);
        assert_eq!(t.no_fixo(38).unwrap().proximo, 38);
    }

    /// quintal.md §3.1: o pé do menino sobre a amarela chama `_YellowDeath(5)` (formiga-amarela.md §9):
    /// música de morte, mensagem de pisada, `_BAntsKilled` + 1 e, sem corpo para reencarnar no jogo
    /// rápido, fim de jogo.
    #[test]
    fn pisada_mata_a_amarela() {
        let ta = TabelasAmarela::padrao();
        let mut e = Estado::vazio();
        e.partida.tipo_jogo = 1;
        e.partida.opcoes[3] = 1;
        e.amarela.plano = 1;
        e.amarela.x = 20;
        e.amarela.y = 20;
        e.amarela.tipo = 0x10;
        e.superficie.vida.definir(20, 20, 0xFF);
        let v = Vitimas::de(ta, TabelasInimigos::padrao());
        pisada(&mut e, v, 18, 15);
        let ev = e.retirar_eventos();
        assert!(ev.contains(&Evento::Mensagem(ta.mensagens.pisada)), "{ev:?}");
        assert!(ev.iter().any(|x| matches!(x, Evento::Musica(m) if (ta.musicas.morte_base..ta.musicas.morte_base + ta.musicas.morte_variacoes).contains(m))));
        assert_eq!(e.preta.mortas_em_luta, 1);
        assert_eq!(e.superficie.vida.em(20, 20), Some(0));
        assert_eq!((e.partida.fim_de_jogo, e.partida.pretas_venceram), (1, 0));
    }

    #[test]
    fn grama_e_bits() {
        let mut e = Estado::vazio();
        e.ambiente.grama[4] = -1;
        assert!(grama_por_cortar(&e, 4, 15));
        cortar_grama(&mut e, 4, 15);
        assert!(!grama_por_cortar(&e, 4, 15));
        assert_eq!(e.ambiente.grama[4] as u16, 0x7FFF);
    }

    #[test]
    fn migracao() {
        let mut e = Estado::vazio();
        // Patch (2, 3): mx = (X + Y − 238)/28 = 2 → X + Y = 294; my = (Y − 66)/10 = 3 → Y = 96.
        e.preta.populacao_quintal[indice_patch(2, 3)] = 41;
        let o = iniciar_migracao(&e, 198, 96);
        assert_eq!(o, Some((2, 3)));
        assert_eq!(iniciar_migracao(&e, 198 + 28, 96), None, "sem colônia preta");
        terminar_migracao(&mut e, o, 198 + 28, 96, 250);
        assert_eq!(e.preta.populacao_quintal[indice_patch(2, 3)], 21);
        assert_eq!(e.preta.populacao_quintal[indice_patch(3, 3)], 20);
    }

    #[test]
    fn enxame_decai() {
        let mut e = Estado::vazio();
        e.preta.enxame_contagem = 40;
        manter_enxame(&mut e, Cor::Preta, 50);
        assert_eq!(e.preta.enxame_contagem, 30);
        e.preta.enxame_contagem = 3;
        manter_enxame(&mut e, Cor::Preta, 50);
        assert_eq!(e.preta.enxame_contagem, 2);
        e.preta.rainhas_guardadas = 70;
        manter_enxame(&mut e, Cor::Preta, 50);
        assert_eq!(e.preta.enxame_contagem, 50);
    }
}
