//! Comparador de tick: roda **um** tick do port a partir de um vetor gravado no original e compara o
//! resultado com o estado que o original deixou, bloco a bloco do save.
//!
//! Especificação e protocolo: docs/spec/regras/testes-tick.md ("Protocolo de teste" e "Placar do
//! port"). Um vetor é um diretório `tick_NNNNNN/` com:
//!
//! - `estado_antes.bin` / `estado_depois.bin`: os 307 blocos do save (formato `.ANT`) na entrada e no
//!   retorno de `_DoAntSim`;
//! - `vida_antes.bin` / `vida_depois.bin`: `_LifeA` (8.192) + `_LifeB` (4.096) + `_LifeR` (4.096);
//! - `extras.json`: `{"antes": {...}, "depois": {...}}` com sementes S e R, `_gameCycles`, `_Tindex`,
//!   `_GlobalKey`, `GameTime` e `Cycle`.
//!
//! Uso: [`VetorTick::ler`] → [`comparar_tick`] → [`RelatorioTick`] (implementa `Display`).
//! Ferramenta de linha de comando: `cargo run -p engine --example comparar_tick -- <dir>`.

use std::fmt;
use std::path::Path;

use serde::Deserialize;

use super::estado::Estado;
use super::rng::GeradorR;
use super::tick::{tick, Regras};
use crate::core::vida::{self, CamadasVida};
use crate::formats::save::{JogoSalvo, LayoutSave};

/// Quantas diferenças detalhadas cada bloco guarda (a contagem de bytes é sempre completa).
pub const MAX_DETALHES: usize = 12;

/// Tamanhos dos três mapas de vida no arquivo `vida_*.bin`, nessa ordem.
const VIDA_A: usize = 128 * 64;
const VIDA_NINHO: usize = 64 * 64;

// ---------------------------------------------------------------------------------------------
// Leitura do vetor
// ---------------------------------------------------------------------------------------------

/// Variáveis fora do save capturadas junto com o estado (testes-tick.md, "Lista de captura adotada").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extras {
    /// Estado do gerador S (`10:cbf2`).
    pub semente_s: u16,
    /// Estado do gerador R (`10:ae34`, 32 bits).
    pub semente_r: u32,
    /// `_gameCycles` (`9:9b2c`).
    pub game_cycles: u32,
    /// `_Tindex` (`9:9b6a`).
    pub tindex: i16,
    /// `_GlobalKey` (`9:7866`).
    pub global_key: i16,
    /// `_GameTime` (redundante com o save, para conferência).
    pub game_time: u32,
    /// `_Cycle` (idem).
    pub cycle: i16,
    /// Relógio do Mac (`_MacTickCount`) devolvido na primeira chamada dentro do tick; `None` se o tick
    /// não o leu ou se o vetor é anterior à captura (só existe em `antes`).
    pub relogio_mac: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ExtrasJson {
    semente_s: i64,
    semente_r: i64,
    #[serde(rename = "gameCycles")]
    game_cycles: i64,
    tindex: i64,
    global_key: i64,
    game_time: i64,
    cycle: i64,
    #[serde(rename = "MacTickCount", default)]
    mac_tick_count: Option<i64>,
}

impl From<ExtrasJson> for Extras {
    fn from(j: ExtrasJson) -> Extras {
        // Os valores vêm da memória do original com o tamanho de cada variável; o corte é intencional.
        Extras {
            semente_s: j.semente_s as u16,
            semente_r: j.semente_r as u32,
            game_cycles: j.game_cycles as u32,
            tindex: j.tindex as i16,
            global_key: j.global_key as i16,
            game_time: j.game_time as u32,
            cycle: j.cycle as i16,
            relogio_mac: j.mac_tick_count.map(|v| v as u32),
        }
    }
}

#[derive(Deserialize)]
struct ExtrasArquivo {
    antes: ExtrasJson,
    depois: ExtrasJson,
}

/// Um vetor de tick do original, já lido.
#[derive(Clone, Debug)]
pub struct VetorTick {
    /// Nome do diretório (ex.: `tick_000079`).
    pub nome: String,
    pub antes: JogoSalvo,
    pub depois: JogoSalvo,
    pub vida_antes: CamadasVida,
    pub vida_depois: CamadasVida,
    pub extras_antes: Extras,
    pub extras_depois: Extras,
}

fn ler_arquivo(dir: &Path, nome: &str) -> Result<Vec<u8>, String> {
    let p = dir.join(nome);
    std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))
}

/// Separa `_LifeA` + `_LifeB` + `_LifeR` (16.384 bytes, todos em colunas, índice x × 64 + y).
pub fn vida_de_bytes(d: &[u8]) -> Result<CamadasVida, String> {
    if d.len() != VIDA_A + 2 * VIDA_NINHO {
        return Err(format!("vida com {} bytes; esperado {}", d.len(), VIDA_A + 2 * VIDA_NINHO));
    }
    Ok(CamadasVida {
        a: d[..VIDA_A].to_vec(),
        b: d[VIDA_A..VIDA_A + VIDA_NINHO].to_vec(),
        r: d[VIDA_A + VIDA_NINHO..].to_vec(),
    })
}

impl VetorTick {
    pub fn ler(dir: &Path, layout: &LayoutSave) -> Result<VetorTick, String> {
        let nome = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let antes = JogoSalvo::ler(layout, ler_arquivo(dir, "estado_antes.bin")?)?;
        let depois = JogoSalvo::ler(layout, ler_arquivo(dir, "estado_depois.bin")?)?;
        let vida_antes = vida_de_bytes(&ler_arquivo(dir, "vida_antes.bin")?)?;
        let vida_depois = vida_de_bytes(&ler_arquivo(dir, "vida_depois.bin")?)?;
        let texto = String::from_utf8(ler_arquivo(dir, "extras.json")?).map_err(|e| e.to_string())?;
        let ex: ExtrasArquivo = serde_json::from_str(&texto).map_err(|e| format!("{nome}/extras.json: {e}"))?;
        Ok(VetorTick {
            nome,
            antes,
            depois,
            vida_antes,
            vida_depois,
            extras_antes: ex.antes.into(),
            extras_depois: ex.depois.into(),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Montagem do estado de entrada
// ---------------------------------------------------------------------------------------------

/// Uma célula diferente num mapa (de vida ou do save).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiferencaCelula {
    /// `'A'`, `'B'` ou `'R'` para os mapas de vida.
    pub plano: char,
    pub x: usize,
    pub y: usize,
    pub original: u8,
    pub port: u8,
}

/// Diferenças entre dois conjuntos de mapas de vida (todas as células, plano A, depois B, depois R).
pub fn comparar_vida(original: &CamadasVida, port: &CamadasVida) -> Vec<DiferencaCelula> {
    let mut v = Vec::new();
    for (plano, o, p) in [('A', &original.a, &port.a), ('B', &original.b, &port.b), ('R', &original.r, &port.r)] {
        for (i, (&a, &b)) in o.iter().zip(p.iter()).enumerate() {
            if a != b {
                v.push(DiferencaCelula { plano, x: i / 64, y: i % 64, original: a, port: b });
            }
        }
    }
    v
}

/// Os mapas de vida do estado, no formato do arquivo.
pub fn vida_do_estado(e: &Estado) -> CamadasVida {
    CamadasVida { a: e.superficie.vida.dados.clone(), b: e.ninho_b.vida.dados.clone(), r: e.ninho_r.vida.dados.clone() }
}

/// O estado de entrada do port para um vetor, e a conferência da reconstrução da vida.
#[derive(Clone, Debug)]
pub struct Entrada {
    /// Save + extras de `antes` + vida gravada (`vida_antes.bin`).
    pub estado: Estado,
    /// Células em que a reconstrução das listas (`core::vida::reconstruir`, como
    /// `_DoLoadInitializations`) difere de `vida_antes.bin`.
    pub vida_reconstruida: Vec<DiferencaCelula>,
    /// O que a reconstrução avisou que não reproduz.
    pub pendencias_vida: Vec<String>,
    /// Blocos em que `para_save(de_save(antes))` não devolve os mesmos bytes (deve ser vazio).
    pub ida_e_volta: Vec<String>,
}

/// Monta o estado de entrada: `estado_antes` sem as inicializações de carga que mexem nos geradores,
/// S, R, `_gameCycles`, `_Tindex` e `_GlobalKey` de `extras["antes"]`, e a vida gravada.
pub fn montar_entrada(v: &VetorTick, layout: &LayoutSave) -> Result<Entrada, String> {
    let (mut e, pendencias_vida) = Estado::de_save_com_pendencias(&v.antes)?;
    let vida_reconstruida = comparar_vida(&v.vida_antes, &vida_do_estado(&e));
    let regravado = e.para_save(layout)?;
    let ida_e_volta =
        v.antes.simbolos().filter(|s| v.antes.bloco(s) != regravado.bloco(s)).map(str::to_string).collect();

    let x = &v.extras_antes;
    e.s.definir_semente(x.semente_s);
    e.r = GeradorR::de_estado(x.semente_r);
    e.ciclos_jogo = x.game_cycles;
    e.relogio_mac = x.relogio_mac;
    e.tindex = x.tindex as i32;
    e.tecla_global = x.global_key;
    e.superficie.vida.dados.clone_from(&v.vida_antes.a);
    e.ninho_b.vida.dados.clone_from(&v.vida_antes.b);
    e.ninho_r.vida.dados.clone_from(&v.vida_antes.r);
    Ok(Entrada { estado: e, vida_reconstruida, pendencias_vida, ida_e_volta })
}

// ---------------------------------------------------------------------------------------------
// Comparação
// ---------------------------------------------------------------------------------------------

/// Resultado de um bloco do save.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparacaoBloco {
    pub simbolo: String,
    /// Bytes diferentes entre o `estado_depois` do original e o do port.
    pub bytes_diferentes: usize,
    /// O original mudou o bloco neste tick (antes ≠ depois).
    pub original_mudou: bool,
    /// O port mudou o bloco neste tick.
    pub port_mudou: bool,
    /// As primeiras diferenças, legíveis (até [`MAX_DETALHES`]).
    pub detalhes: Vec<String>,
    /// Quantas diferenças de elemento/célula/formiga existem no total.
    pub total_detalhes: usize,
}

impl ComparacaoBloco {
    pub fn igual(&self) -> bool {
        self.bytes_diferentes == 0
    }
}

/// Uma variável fora do save (sementes, `_gameCycles`...): valor do original e do port depois do tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparacaoExtra {
    pub nome: &'static str,
    pub original: u32,
    pub port: u32,
}

/// Relatório de um tick.
#[derive(Clone, Debug)]
pub struct RelatorioTick {
    pub nome: String,
    pub game_time: u32,
    pub cycle: i16,
    pub blocos: Vec<ComparacaoBloco>,
    pub extras: Vec<ComparacaoExtra>,
    /// Vida depois do tick: port × `vida_depois.bin`.
    pub vida: Vec<DiferencaCelula>,
    /// Vida reconstruída das listas × `vida_antes.bin` (entrada).
    pub vida_reconstruida: Vec<DiferencaCelula>,
    /// Vida reconstruída das listas do `estado_depois` do original × `vida_depois.bin` (confere a
    /// reconstrução também na saída, sem depender do port).
    pub vida_reconstruida_depois: Vec<DiferencaCelula>,
    pub pendencias_vida: Vec<String>,
    pub ida_e_volta: Vec<String>,
}

impl RelatorioTick {
    pub fn bloco(&self, simbolo: &str) -> Option<&ComparacaoBloco> {
        self.blocos.iter().find(|b| b.simbolo == simbolo)
    }

    pub fn iguais(&self) -> usize {
        self.blocos.iter().filter(|b| b.igual()).count()
    }

    pub fn diferentes(&self) -> impl Iterator<Item = &ComparacaoBloco> {
        self.blocos.iter().filter(|b| !b.igual())
    }

    /// Blocos que o original mudou neste tick.
    pub fn mudados_pelo_original(&self) -> usize {
        self.blocos.iter().filter(|b| b.original_mudou).count()
    }

    /// Blocos que o original mudou e o port deixou iguais ao original.
    pub fn mudados_e_acertados(&self) -> usize {
        self.blocos.iter().filter(|b| b.original_mudou && b.igual()).count()
    }

    pub fn extras_iguais(&self) -> usize {
        self.extras.iter().filter(|x| x.original == x.port).count()
    }

    /// Uma linha de placar: `tick_000079 (GameTime 79): 290/307 blocos iguais ...`.
    pub fn placar(&self) -> String {
        format!(
            "{} (GameTime {} → {}): {}/{} blocos iguais; dos {} que o original mudou, {} iguais; \
             extras {}/{}; vida {} células diferentes",
            self.nome,
            self.game_time,
            self.game_time.wrapping_add(1),
            self.iguais(),
            self.blocos.len(),
            self.mudados_pelo_original(),
            self.mudados_e_acertados(),
            self.extras_iguais(),
            self.extras.len(),
            self.vida.len()
        )
    }
}

fn eh_lista(simbolo: &str) -> Option<char> {
    let b = simbolo.as_bytes();
    (b.len() == 6 && matches!(b[0], b'A' | b'B' | b'R') && &simbolo[1..5] == "list").then(|| b[5] as char)
}

/// Altura (em células) de um bloco de mapa guardado em colunas, se for um mapa.
fn altura_mapa(simbolo: &str) -> Option<usize> {
    if simbolo.starts_with("PherMap") {
        Some(32)
    } else if matches!(simbolo, "MapA" | "MapB" | "MapR" | "ExitMapB" | "ExitMapR") {
        Some(64)
    } else {
        None
    }
}

fn valor(d: &[u8], tamanho: usize) -> String {
    match tamanho {
        1 => d[0].to_string(),
        2 => i16::from_le_bytes([d[0], d[1]]).to_string(),
        4 => {
            let v = i32::from_le_bytes([d[0], d[1], d[2], d[3]]);
            let (lo, hi) = (i16::from_le_bytes([d[0], d[1]]), i16::from_le_bytes([d[2], d[3]]));
            format!("{v} ({lo}, {hi})")
        }
        _ => d.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "),
    }
}

/// Compara um bloco. `tamanho_elemento` vem do layout (o campo `count`: 1, 2 ou 4 bytes).
fn comparar_bloco(
    simbolo: &str,
    tamanho_elemento: usize,
    antes: &[u8],
    original: &[u8],
    port: &[u8],
) -> ComparacaoBloco {
    let bytes_diferentes = original.iter().zip(port).filter(|(a, b)| a != b).count();
    let mut detalhes = Vec::new();
    let mut total = 0;
    let mut anotar = |s: String| {
        total += 1;
        if detalhes.len() < MAX_DETALHES {
            detalhes.push(s);
        }
    };
    if bytes_diferentes > 0 {
        if let Some(campo) = eh_lista(simbolo) {
            for (i, (a, b)) in original.iter().zip(port).enumerate() {
                if a != b {
                    anotar(format!("formiga {i} campo {campo}: original {a}, port {b} (antes {})", antes[i]));
                }
            }
        } else if let Some(h) = altura_mapa(simbolo) {
            for (i, (a, b)) in original.iter().zip(port).enumerate() {
                if a != b {
                    anotar(format!("célula ({}, {}): original {a}, port {b} (antes {})", i / h, i % h, antes[i]));
                }
            }
        } else {
            let t = if matches!(tamanho_elemento, 1 | 2 | 4) && original.len().is_multiple_of(tamanho_elemento) {
                tamanho_elemento
            } else {
                1
            };
            let n = original.len() / t;
            for i in 0..n {
                let r = i * t..(i + 1) * t;
                if original[r.clone()] != port[r.clone()] {
                    let onde = if n == 1 { String::new() } else { format!("[{i}] ") };
                    anotar(format!(
                        "{onde}original {}, port {} (antes {})",
                        valor(&original[r.clone()], t),
                        valor(&port[r.clone()], t),
                        valor(&antes[r], t)
                    ));
                }
            }
        }
    }
    ComparacaoBloco {
        simbolo: simbolo.to_string(),
        bytes_diferentes,
        original_mudou: antes != original,
        port_mudou: antes != port,
        detalhes,
        total_detalhes: total,
    }
}

/// Roda um tick do port sobre o vetor e compara com o original.
/// Largura e altura da janela de edição em tiles na bancada QEMU (640×480; telas/jogo.md).
const VISTA_BANCADA: (i32, i32) = (25, 19);

/// O original move a vista dentro do tick (`_GotoMyAnt` → `_CenterEdit` / `_SetMapPlaneLocation`,
/// `1000:a764`), e `_MapPnt`, `_MapPlane` e `_MapMode` vão no save. No port isso é da interface
/// (`Evento::CentralizarNaAmarela`, tratado por `core::partida`); o comparador reproduz só o efeito no
/// save, com a geometria da bancada: centraliza `_MapPnt` na amarela e, se ela está noutro plano, troca
/// `_MapPlane` e `_MapMode` (`_SetMapModeAnt`). Fora do Experimental, como `core::partida`.
fn aplicar_vista_da_bancada(e: &mut Estado) {
    let pedidos = e.eventos.iter().filter(|ev| matches!(ev, crate::rules::estado::Evento::CentralizarNaAmarela)).count();
    if pedidos == 0 || e.partida.tipo_jogo == 3 {
        return;
    }
    let (plano, x, y) = (e.amarela.plano, e.amarela.x as i32, e.amarela.y as i32);
    if plano <= 0 {
        return;
    }
    if e.vistas.plano != plano {
        e.vistas.plano = plano;
        e.vistas.modo_mapa = plano;
    }
    let (px, py) = centralizar_edicao(e.vistas.ponto, x, y, plano);
    e.vistas.ponto = (px as i16, py as i16);
}

/// `_CenterEdit` (`1010:1620`, telas/jogo.md "Centralizar"): anda `_MapPnt` até (x − w/2, y − h/2)
/// como uma reta de Bresenham em ponto fixo (o eixo menor pode ficar um passo aquém) e aplica os limites.
fn centralizar_edicao(ponto: (i16, i16), x: i32, y: i32, plano: i16) -> (i32, i32) {
    let (w, h) = VISTA_BANCADA;
    let (mut px, mut py) = (ponto.0 as i32, ponto.1 as i32);
    let (dx, dy) = (x - px - w / 2, y - py - h / 2);
    let (ax, ay) = (dx.unsigned_abs(), dy.unsigned_abs());
    let (sx, sy) = (dx.signum(), dy.signum());
    let (maior, menor) = (ax.max(ay), ax.min(ay));
    if let Some(fracao) = (menor << 16).checked_div(maior) {
        let mut acum: u32 = 0;
        for _ in 0..maior {
            acum += fracao;
            let anda_menor = acum & 0x1_0000 != 0;
            acum &= !0x1_0000;
            if ax >= ay {
                px += sx;
                py += if anda_menor { sy } else { 0 };
            } else {
                py += sy;
                px += if anda_menor { sx } else { 0 };
            }
        }
    }
    let largura = if plano == 1 { 128 } else { 64 };
    (px.clamp(0, largura - w), py.clamp(0, 64 - h))
}

pub fn comparar_tick(v: &VetorTick, layout: &LayoutSave, regras: &mut dyn Regras) -> Result<RelatorioTick, String> {
    comparar_com(v, layout, |e| {
        tick(e, regras);
        aplicar_vista_da_bancada(e);
        Ok(())
    })
}

/// Como [`comparar_tick`], mas o port aplica `acao` (uma ação da interface gravada no original com o
/// jogo pausado, ex.: `_XferPatch`) em vez de um tick. O vetor tem o mesmo formato.
pub fn comparar_com(
    v: &VetorTick,
    layout: &LayoutSave,
    acao: impl FnOnce(&mut Estado) -> Result<(), String>,
) -> Result<RelatorioTick, String> {
    let entrada = montar_entrada(v, layout)?;
    let mut e = entrada.estado;
    acao(&mut e)?;
    let port = e.para_save(layout)?;

    let mut blocos = Vec::with_capacity(layout.blocos.len());
    for b in &layout.blocos {
        let s = b.simbolo.as_str();
        fn pega<'a>(j: &'a JogoSalvo, s: &str) -> Result<&'a [u8], String> {
            j.bloco(s).ok_or_else(|| format!("bloco {s} ausente"))
        }
        blocos.push(comparar_bloco(s, b.count, pega(&v.antes, s)?, pega(&v.depois, s)?, pega(&port, s)?));
    }

    let x = &v.extras_depois;
    let extras = vec![
        ComparacaoExtra { nome: "SementeS", original: x.semente_s as u32, port: e.s.semente() as u32 },
        ComparacaoExtra { nome: "SementeR", original: x.semente_r, port: e.r.estado() },
        ComparacaoExtra { nome: "gameCycles", original: x.game_cycles, port: e.ciclos_jogo },
        ComparacaoExtra { nome: "Tindex", original: x.tindex as u16 as u32, port: e.tindex as u16 as u32 },
        ComparacaoExtra { nome: "GlobalKey", original: x.global_key as u16 as u32, port: e.tecla_global as u16 as u32 },
    ];

    Ok(RelatorioTick {
        nome: v.nome.clone(),
        game_time: v.extras_antes.game_time,
        cycle: v.extras_antes.cycle,
        blocos,
        extras,
        vida: comparar_vida(&v.vida_depois, &vida_do_estado(&e)),
        vida_reconstruida: entrada.vida_reconstruida,
        vida_reconstruida_depois: comparar_vida(&v.vida_depois, &vida::reconstruir(&v.depois)?.camadas),
        pendencias_vida: entrada.pendencias_vida,
        ida_e_volta: entrada.ida_e_volta,
    })
}

// ---------------------------------------------------------------------------------------------
// Texto
// ---------------------------------------------------------------------------------------------

fn escrever_celulas(f: &mut fmt::Formatter<'_>, d: &[DiferencaCelula]) -> fmt::Result {
    for c in d.iter().take(MAX_DETALHES) {
        writeln!(f, "      Life{} ({}, {}): original {}, port {}", c.plano, c.x, c.y, c.original, c.port)?;
    }
    if d.len() > MAX_DETALHES {
        writeln!(f, "      ... mais {}", d.len() - MAX_DETALHES)?;
    }
    Ok(())
}

impl fmt::Display for RelatorioTick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "== {} (GameTime {}, Cycle {}) ==", self.nome, self.game_time, self.cycle)?;
        writeln!(f, "{}", self.placar())?;
        if !self.ida_e_volta.is_empty() {
            writeln!(f, "ATENÇÃO: ida e volta do save diferente em {:?}", self.ida_e_volta)?;
        }

        for (titulo, d) in [
            ("Entrada: vida reconstruída das listas × vida_antes.bin", &self.vida_reconstruida),
            ("Saída do original: vida reconstruída das listas × vida_depois.bin", &self.vida_reconstruida_depois),
        ] {
            writeln!(f, "\n{titulo}")?;
            if d.is_empty() {
                writeln!(f, "    igual")?;
            } else {
                writeln!(f, "    {} células diferentes", d.len())?;
                escrever_celulas(f, d)?;
            }
        }
        for p in &self.pendencias_vida {
            writeln!(f, "    pendência: {p}")?;
        }

        writeln!(f, "\nExtras depois do tick:")?;
        for x in &self.extras {
            let s = if x.original == x.port { "igual" } else { "DIFERENTE" };
            writeln!(f, "    {:<11} {s}: original {}, port {}", x.nome, x.original, x.port)?;
        }

        writeln!(f, "\nVida depois do tick:")?;
        if self.vida.is_empty() {
            writeln!(f, "    igual")?;
        } else {
            writeln!(f, "    {} células diferentes", self.vida.len())?;
            escrever_celulas(f, &self.vida)?;
        }

        let acertados: Vec<&str> =
            self.blocos.iter().filter(|b| b.original_mudou && b.igual()).map(|b| b.simbolo.as_str()).collect();
        writeln!(f, "\nBlocos que o original mudou e o port acertou ({}): {}", acertados.len(), acertados.join(", "))?;

        let difs: Vec<&ComparacaoBloco> = self.diferentes().collect();
        writeln!(f, "\nBlocos diferentes ({}):", difs.len())?;
        for b in difs {
            let quem = match (b.original_mudou, b.port_mudou) {
                (true, true) => "os dois mudaram",
                (true, false) => "só o original mudou",
                (false, true) => "só o port mudou",
                (false, false) => "nenhum mudou",
            };
            writeln!(f, "  {} — {} bytes diferentes ({quem})", b.simbolo, b.bytes_diferentes)?;
            for d in &b.detalhes {
                writeln!(f, "      {d}")?;
            }
            if b.total_detalhes > b.detalhes.len() {
                writeln!(f, "      ... mais {}", b.total_detalhes - b.detalhes.len())?;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------
// Ações da interface gravadas no original (tests/referencias/acoes/)
// ---------------------------------------------------------------------------------------------

/// Aplica ao estado a ação da interface descrita em `extras.json["acao"]` de um vetor de ação
/// (`tools/cenarios_tick.py`): `{"xfer": [px, py]}` = duplo clique no patch da Yard View
/// (`_XferPatch`); `{"place": [px, py]}` = PLACE e clique no patch (`_PlaceQueenInYard`).
/// Devolve uma descrição do resultado.
pub fn aplicar_acao(
    e: &mut Estado,
    acao: &serde_json::Value,
    tab: &super::geracao::TabelasGeracao,
    cfg: &super::estado::ConfigSimulacao,
) -> Result<String, String> {
    let par = |k: &str| acao[k].as_array().map(|p| (p[0].as_i64().unwrap_or(-1) as i16, p[1].as_i64().unwrap_or(-1) as i16));
    if let Some((x, y)) = par("xfer") {
        e.vistas.ponto_quintal = (x, y);
        return Ok(format!("{:?}", super::troca_patch::trocar_patch(e, tab, cfg)?));
    }
    if let Some((x, y)) = par("place") {
        // No original o clique que encerra o PLACE também chega a `_YardArea`: `_YMapPnt` = patch
        // clicado, dentro ou fora do alcance (bancada, acoes/place e place_fora).
        e.vistas.ponto_quintal = (x, y);
        return Ok(format!("{:?}", super::troca_patch::colocar_rainha(e, x, y)));
    }
    if let Some((x, y)) = par("special") {
        // Um tick em que a amarela morre sem corpo no Jogo Completo com mais de uma colônia preta: o
        // original chama `_SpecialXfer` (`1000:7eb6`) DENTRO do tick e o jogador escolhe a colônia.
        // Aqui: o tick do port (que só pede a escolha) e depois o efeito de `_SpecialXfer` com o patch
        // escolhido na bancada. Ver testes-tick.md (o que difere é a ordem: no original o resto do tick
        // já roda no patch novo).
        let mut regras = super::tick::RegrasOriginais::default();
        tick(e, &mut regras);
        transferencia_especial(e, x, y);
        return Ok(format!("{:?}", super::troca_patch::trocar_patch(e, tab, cfg)?));
    }
    use super::experimental::{self as exp, Contexto, ControleExperimental, Ferramenta, TabelasExperimental};
    let c = Contexto::padrao();
    if acao.get("quick_exp").is_some() {
        return Ok(format!("{:?}", exp::quick_para_experimental(e, c)));
    }
    let mut ctl = ControleExperimental::novo(TabelasExperimental::padrao());
    if acao.get("start").is_some() {
        let plano = e.vistas.plano as i32;
        return Ok(format!("{:?}", exp::escolher_ferramenta(e, c, &mut ctl, Ferramenta::ComecarJogo, None, plano)));
    }
    if let Some(passos) = acao["exp"].as_array() {
        // Cada passo: ferramenta e subopção (escritas na memória na bancada), Ctrl, e as células por
        // onde o cursor passou com o botão apertado (a primeira = onde apertou).
        let mut efeitos = Vec::new();
        for p in passos {
            let f = p["f"].as_i64().ok_or("passo sem f")? as i16;
            let fer = Ferramenta::de_i16(f).ok_or(format!("ferramenta {f}"))?;
            e.partida.ferramenta_experimental = f;
            ctl.subopcoes[fer.indice()] = p["sub"].as_u64().unwrap_or(0) as u8;
            let ctrl = p["ctrl"].as_bool().unwrap_or(false);
            let plano = e.vistas.plano as i32;
            let pontos: Vec<(i32, i32)> = p["pontos"]
                .as_array()
                .ok_or("passo sem pontos")?
                .iter()
                .map(|q| (q[0].as_i64().unwrap_or(-1) as i32, q[1].as_i64().unwrap_or(-1) as i32))
                .collect();
            let ((x0, y0), resto) = pontos.split_first().ok_or("passo sem pontos")?;
            efeitos.extend(exp::pressionar(e, c, &mut ctl, plano, *x0, *y0, ctrl));
            for &(x, y) in resto {
                efeitos.extend(exp::arrastar(e, c, &mut ctl, plano, x, y, ctrl));
            }
            exp::soltar(&mut ctl);
        }
        return Ok(format!("{efeitos:?}"));
    }
    Err(format!("ação desconhecida: {acao}"))
}

/// O que `_SpecialXfer` (`1000:7eb6`) faz no estado antes do `_XferPatch` do patch escolhido:
/// zera a colônia preta do patch atual no mapa do quintal, abre a Strategic View (`_YardMode = 2`,
/// `_ColonyUpdateFlag = 1`), `_SetMapPlane(0)` (`_OldMapPlane`, `_MapPlane = 0`, `_YMapPnt` = patch
/// atual) e grava `_YMapPnt` = escolhido.
pub fn transferencia_especial(e: &mut Estado, x: i16, y: i16) {
    let (cx, cy) = e.ambiente.patch;
    e.preta.populacao_quintal.0[(cx * 16 + cy) as usize] = 0;
    e.vistas.modo_quintal = 2;
    e.ambiente.atualizar_colonias = 1;
    // `_SetMapPlane(0)`: plano e modo do mapa 0. Na bancada `_OldMapPlane` terminou como estava (0),
    // e não com o plano anterior [em aberto: quem o repõe].
    e.vistas.plano = 0;
    e.vistas.modo_mapa = 0;
    e.vistas.ponto_quintal = (x, y);
}

/// `extras.json["acao"]` de um vetor de ação.
pub fn ler_acao(dir: &Path) -> Result<serde_json::Value, String> {
    let texto = String::from_utf8(ler_arquivo(dir, "extras.json")?).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&texto).map_err(|e| e.to_string())?;
    Ok(v["acao"].clone())
}

// ---------------------------------------------------------------------------------------------
// Sequência longa (tests/referencias/ticks_longos/)
// ---------------------------------------------------------------------------------------------

/// Um ponto conferido de uma sequência longa: o estado depois de `ticks` ticks encadeados.
#[derive(Clone, Debug)]
pub struct PontoSequencia {
    pub ticks: usize,
    /// Blocos do save diferentes do original (com os primeiros detalhes).
    pub blocos: Vec<ComparacaoBloco>,
    pub vida: Vec<DiferencaCelula>,
}

/// Resultado de rodar uma sequência longa no port.
#[derive(Clone, Debug)]
pub struct RelatorioSequencia {
    pub ticks: usize,
    /// Primeiro tick (1 = o primeiro) em que S ou R do port diferem do original depois do tick.
    pub primeiro_sorteio_diferente: Option<usize>,
    pub pontos: Vec<PontoSequencia>,
}

/// Roda no port, encadeados, os ticks de uma sequência longa gravada por `tools/cenarios_tick.py longo`:
/// `inicio/estado.bin` + `inicio/vida.bin` (entrada do 1º tick), `extras.json` (lista com os extras
/// antes/depois de cada tick, inclusive o relógio do Mac) e `depois_NNNN/` (estado e vida depois de
/// NNNN ticks). Antes de cada tick põe `_gameCycles`, o relógio do Mac e `_GlobalKey` do original (o
/// que muda fora do tick); S e R seguem do próprio port.
pub fn rodar_sequencia(dir: &Path, layout: &LayoutSave, regras: &mut dyn Regras) -> Result<RelatorioSequencia, String> {
    let inicio = JogoSalvo::ler(layout, ler_arquivo(dir, "inicio/estado.bin")?)?;
    let vida_inicio = vida_de_bytes(&ler_arquivo(dir, "inicio/vida.bin")?)?;
    let texto = String::from_utf8(ler_arquivo(dir, "extras.json")?).map_err(|e| e.to_string())?;
    let lista: Vec<ExtrasArquivo> = serde_json::from_str(&texto).map_err(|e| format!("extras.json: {e}"))?;
    let extras: Vec<(Extras, Extras)> = lista.into_iter().map(|x| (x.antes.into(), x.depois.into())).collect();
    let (x0, d0) = *extras.first().ok_or("extras.json vazio")?;
    let v = VetorTick {
        nome: "inicio".into(),
        antes: inicio.clone(),
        depois: inicio.clone(),
        vida_antes: vida_inicio.clone(),
        vida_depois: vida_inicio,
        extras_antes: x0,
        extras_depois: d0,
    };
    let mut e = montar_entrada(&v, layout)?.estado;
    let mut primeiro_sorteio_diferente = None;
    let mut pontos = Vec::new();
    for (k, (antes, depois)) in extras.iter().enumerate() {
        e.ciclos_jogo = antes.game_cycles;
        e.relogio_mac = antes.relogio_mac;
        e.tecla_global = antes.global_key;
        tick(&mut e, regras);
        aplicar_vista_da_bancada(&mut e);
        e.retirar_eventos();
        if primeiro_sorteio_diferente.is_none()
            && (e.s.semente() != depois.semente_s || e.r.estado() != depois.semente_r)
        {
            primeiro_sorteio_diferente = Some(k + 1);
        }
        let ponto = dir.join(format!("depois_{:04}", k + 1));
        if ponto.is_dir() {
            let original = JogoSalvo::ler(layout, ler_arquivo(&ponto, "estado.bin")?)?;
            let vida_original = vida_de_bytes(&ler_arquivo(&ponto, "vida.bin")?)?;
            let port = e.para_save(layout)?;
            let mut blocos = Vec::new();
            for b in &layout.blocos {
                let s = b.simbolo.as_str();
                let pega = |j: &JogoSalvo| j.bloco(s).map(<[u8]>::to_vec).ok_or_else(|| format!("bloco {s} ausente"));
                let c = comparar_bloco(s, b.count, &pega(&inicio)?, &pega(&original)?, &pega(&port)?);
                if !c.igual() {
                    blocos.push(c);
                }
            }
            pontos.push(PontoSequencia { ticks: k + 1, blocos, vida: comparar_vida(&vida_original, &vida_do_estado(&e)) });
        }
    }
    Ok(RelatorioSequencia { ticks: extras.len(), primeiro_sorteio_diferente, pontos })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn detalhes_de_lista_mapa_e_escalar() {
        let b = comparar_bloco("BlistT", 1, &[0, 0, 0], &[0, 5, 0], &[0, 6, 0]);
        assert_eq!((b.bytes_diferentes, b.original_mudou, b.port_mudou), (1, true, true));
        assert_eq!(b.detalhes, vec!["formiga 1 campo T: original 5, port 6 (antes 0)"]);

        let mut m = vec![0u8; 4096];
        m[64 * 3 + 7] = 9;
        let b = comparar_bloco("MapB", 1, &vec![0; 4096], &vec![0; 4096], &m);
        assert_eq!(b.detalhes, vec!["célula (3, 7): original 0, port 9 (antes 0)"]);
        assert!(!b.original_mudou && b.port_mudou);

        let b = comparar_bloco("CastePopB", 2, &[1, 0, 2, 0], &[1, 0, 3, 0], &[1, 0, 2, 0]);
        assert_eq!(b.detalhes, vec!["[1] original 3, port 2 (antes 2)"]);
        let b = comparar_bloco("GameTime", 4, &[79, 0, 0, 0], &[80, 0, 0, 0], &[80, 0, 0, 0]);
        assert!(b.igual() && b.original_mudou);
    }

    #[test]
    fn vida_em_tres_planos() {
        let mut d = vec![0u8; 16384];
        d[VIDA_A + 65] = 3;
        let v = vida_de_bytes(&d).unwrap();
        let dif = comparar_vida(&CamadasVida::vazia(), &v);
        assert_eq!(dif, vec![DiferencaCelula { plano: 'B', x: 1, y: 1, original: 0, port: 3 }]);
        assert!(vida_de_bytes(&d[1..]).is_err());
    }
}
