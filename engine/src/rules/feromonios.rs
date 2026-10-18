//! Os cinco mapas de feromônio da superfície e as regras que os alteram e leem.
//! Especificação: docs/spec/regras/feromonios.md (seção "Implementação no port" lista a
//! correspondência com as funções do original).
//!
//! - Cada mapa tem 64 × 32 células de um byte, guardadas coluna a coluna (índice `cx * 32 + cy`);
//!   a célula `(cx, cy)` cobre os tiles `(2cx..2cx+1, 2cy..2cy+1)` da superfície (128 × 64).
//! - As tabelas de direção (`_Dx8`, `_Dy8`, `_TurnTab`) e as constantes das regras vêm de
//!   `data/feromonios.json` ([`TabelasFeromonio`]), gerado por `tools/extrair_feromonios.py`.
//! - As rotinas de decaimento, suavização e depósito não usam números aleatórios. As de escolha de
//!   direção usam o [`GeradorS`] na mesma ordem do original (ver cada função).

use serde::Deserialize;

use std::sync::OnceLock;

use crate::formats::save::{JogoSalvo, Mapa};
use crate::recursos::Recursos;
use crate::rules::comum::{self, Direcoes, TabelaDeGiro};
use crate::rules::rng::GeradorS;

/// Cor de uma colônia (bit 7 do tipo da formiga: 0 = preta, 1 = vermelha). É a mesma de todas as
/// regras ([`crate::rules::geracao::Cor`]).
pub use crate::rules::geracao::Cor;

/// Nome do arquivo de dados (em `data/` ou num mod).
pub const ARQUIVO_TABELAS: &str = "feromonios.json";

/// Colunas do mapa de cheiro (metade das 128 colunas da superfície).
pub const LARGURA: usize = 64;
/// Linhas do mapa de cheiro (metade das 64 linhas da superfície).
pub const ALTURA: usize = 32;
/// Bytes de cada mapa.
pub const CELULAS: usize = LARGURA * ALTURA;

/// Última coluna e última linha de tiles da superfície (borda do mapa nas funções de direção).
const ULTIMO_X: i32 = 127;
const ULTIMO_Y: i32 = 63;

/// Constantes das regras (bloco `regras` de `feromonios.json`).
#[derive(Clone, Debug, Deserialize)]
pub struct RegrasFeromonio {
    /// Trilha abaixo deste valor zera no decaimento; acima perde a metade (`_ColonySmellBT/RT`).
    pub limiar_trilha: u8,
    /// Resultado da suavização do alarme abaixo deste valor vira 0 (`_SmoothAlarm`).
    pub limiar_alarme: u8,
    /// Teto do alarme somado pela formiga amarela (`_AlarmHere`).
    pub teto_alarme_amarela: u8,
    /// Valor posto na célula de cada buraco do ninho (`_FillHolesBN/RN`).
    pub valor_buraco: u8,
    /// Tile do `_MapA` que faz a célula do buraco receber 0 em vez de `valor_buraco`.
    pub tile_buraco_fechado: u8,
}

/// Tabelas de `data/comum.json` que a parte dos feromônios usa (chave aqui, chave em `comum.json`).
pub const USOS_COMUNS: &[comum::UsoComum] = &[
    ("dx8", "dx8"),
    ("dy8", "dy8"),
    ("turn_tab", "turn_tab"),
];

/// Conteúdo de `data/feromonios.json`, com as tabelas comuns de `data/comum.json` ([`USOS_COMUNS`]).
#[derive(Clone, Debug, Deserialize)]
pub struct TabelasFeromonio {
    /// `_Dx8`/`_Dy8`: deslocamento por direção 0–7 (0 = cima, 2 = direita, 4 = baixo, 6 = esquerda).
    pub dx8: [i8; 8],
    pub dy8: [i8; 8],
    /// `_TurnTab[atual][alvo]`: nova direção ao virar de `atual` rumo a `alvo` (no máximo 45°).
    pub turn_tab: [[u8; 8]; 8],
    pub regras: RegrasFeromonio,
}

impl TabelasFeromonio {
    /// Lê `feromonios.json` (com mods).
    pub fn carregar(r: &Recursos) -> Result<TabelasFeromonio, String> {
        let t: TabelasFeromonio = comum::parte_dos_recursos(r, ARQUIVO_TABELAS, USOS_COMUNS)?;
        t.validar()?;
        Ok(t)
    }

    pub fn de_json(texto: &str) -> Result<TabelasFeromonio, String> {
        let t: TabelasFeromonio = comum::parte_do_texto(ARQUIVO_TABELAS, texto, USOS_COMUNS)?;
        t.validar()?;
        Ok(t)
    }

    /// Cópia embutida de `data/feromonios.json` (sem mods), para `RegrasOriginais::default` e testes.
    pub fn padrao() -> &'static TabelasFeromonio {
        static T: OnceLock<TabelasFeromonio> = OnceLock::new();
        T.get_or_init(|| {
            TabelasFeromonio::de_json(include_str!("../../../data/feromonios.json")).expect("data/feromonios.json")
        })
    }

    fn validar(&self) -> Result<(), String> {
        if self.turn_tab.iter().flatten().any(|&d| d > 7) {
            return Err(format!(
                "{ARQUIVO_TABELAS}: turn_tab com direção fora de 0–7"
            ));
        }
        comum::validar_direcoes(ARQUIVO_TABELAS, &self.dx8, &self.dy8)
    }

    /// `_TurnTab[atual][alvo]` ([`TabelaDeGiro::virar`]).
    pub fn virar(&self, atual: u8, alvo: u8) -> u8 {
        TabelaDeGiro::virar(self, atual as i32, alvo as i32)
    }
}

impl Direcoes for TabelasFeromonio {
    fn deslocamentos(&self) -> (&[i8; 8], &[i8; 8]) {
        (&self.dx8, &self.dy8)
    }
}

impl TabelaDeGiro for TabelasFeromonio {
    fn turn_tab(&self) -> &[[u8; 8]; 8] {
        &self.turn_tab
    }
}

/// As cinco camadas, na ordem em que aparecem no `.ANT`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Camada {
    /// `_PherMapA`: alarme, comum às duas cores.
    Alarme,
    /// `_PherMapBN`: cheiro de ninho preto.
    NinhoPreto,
    /// `_PherMapBT`: trilha preta.
    TrilhaPreta,
    /// `_PherMapRN`: cheiro de ninho vermelho.
    NinhoVermelho,
    /// `_PherMapRT`: trilha vermelha.
    TrilhaVermelha,
}

impl Camada {
    pub const TODAS: [Camada; 5] = [
        Camada::Alarme,
        Camada::NinhoPreto,
        Camada::TrilhaPreta,
        Camada::NinhoVermelho,
        Camada::TrilhaVermelha,
    ];

    /// Símbolo do bloco no `.ANT` (sem o `_`).
    pub fn simbolo(self) -> &'static str {
        match self {
            Camada::Alarme => "PherMapA",
            Camada::NinhoPreto => "PherMapBN",
            Camada::TrilhaPreta => "PherMapBT",
            Camada::NinhoVermelho => "PherMapRN",
            Camada::TrilhaVermelha => "PherMapRT",
        }
    }

    pub fn ninho(cor: Cor) -> Camada {
        match cor {
            Cor::Preta => Camada::NinhoPreto,
            Cor::Vermelha => Camada::NinhoVermelho,
        }
    }

    pub fn trilha(cor: Cor) -> Camada {
        match cor {
            Cor::Preta => Camada::TrilhaPreta,
            Cor::Vermelha => Camada::TrilhaVermelha,
        }
    }
}

/// Índice da célula que contém o tile `(x, y)` da superfície, ou `None` fora do mapa.
///
/// O original calcula `(x >> 1) * 32 + (y >> 1)` sem conferir limites; com coordenadas fora da
/// superfície ele leria ou escreveria fora do mapa. O port recusa (ver a spec).
pub fn celula_do_tile(x: i32, y: i32) -> Option<usize> {
    if (0..=ULTIMO_X).contains(&x) && (0..=ULTIMO_Y).contains(&y) {
        Some((x as usize >> 1) * ALTURA + (y as usize >> 1))
    } else {
        None
    }
}

/// Um mapa de cheiro 64 × 32, coluna a coluna.
#[derive(Clone, PartialEq, Eq)]
pub struct MapaCheiro {
    pub celulas: [u8; CELULAS],
}

impl Default for MapaCheiro {
    fn default() -> MapaCheiro {
        MapaCheiro {
            celulas: [0; CELULAS],
        }
    }
}

impl std::fmt::Debug for MapaCheiro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let nao_nulas: Vec<_> = self
            .celulas
            .iter()
            .enumerate()
            .filter(|(_, v)| **v != 0)
            .map(|(i, v)| (i / ALTURA, i % ALTURA, *v))
            .collect();
        write!(
            f,
            "MapaCheiro {{ não nulas (cx, cy, valor): {nao_nulas:?} }}"
        )
    }
}

impl MapaCheiro {
    pub fn de_bytes(b: &[u8]) -> Result<MapaCheiro, String> {
        let celulas: [u8; CELULAS] = b
            .try_into()
            .map_err(|_| format!("mapa de cheiro com {} bytes; esperado {CELULAS}", b.len()))?;
        Ok(MapaCheiro { celulas })
    }

    /// Valor da célula `(cx, cy)` (coordenadas do mapa de cheiro, não de tile).
    pub fn em(&self, cx: usize, cy: usize) -> u8 {
        self.celulas[cx * ALTURA + cy]
    }

    pub fn definir(&mut self, cx: usize, cy: usize, v: u8) {
        self.celulas[cx * ALTURA + cy] = v;
    }

    /// Valor na célula que contém o tile `(x, y)`; 0 fora da superfície.
    pub fn no_tile(&self, x: i32, y: i32) -> u8 {
        celula_do_tile(x, y).map_or(0, |i| self.celulas[i])
    }

    /// Valor do vizinho na direção `d` da célula `(cx, cy)`, dando a volta nas bordas
    /// (`& 0x3F` na coluna, `& 0x1F` na linha), como as funções de direção do original.
    fn vizinho_circular(&self, tab: &TabelasFeromonio, cx: i32, cy: i32, d: usize) -> u8 {
        let nx = (cx + tab.dx8[d] as i32) & 0x3F;
        let ny = (cy + tab.dy8[d] as i32) & 0x1F;
        self.em(nx as usize, ny as usize)
    }

    /// Direção 0–7 do maior vizinho (o primeiro, em caso de empate) e o seu valor. Se todos são 0,
    /// devolve (0, 0).
    fn maior_vizinho(&self, tab: &TabelasFeromonio, cx: i32, cy: i32) -> (u8, u8) {
        let mut melhor = (0u8, 0u8);
        for d in 0..8 {
            let v = self.vizinho_circular(tab, cx, cy, d);
            if v > melhor.1 {
                melhor = (d as u8, v);
            }
        }
        melhor
    }

    // --- depósito (seção 2 da spec) ---

    /// `_JamScentBN/RN/BT/RT`: a célula do tile fica com o maior entre o valor atual e `v`.
    pub fn depositar(&mut self, x: i32, y: i32, v: u8) {
        if let Some(i) = celula_do_tile(x, y) {
            self.celulas[i] = self.celulas[i].max(v);
        }
    }

    /// `_AlarmHere`: soma `v` e limita ao `teto`. Se a célula já passava do teto (ferramenta
    /// Experimental), ela **cai** para o teto: o original grava `min(c + v, teto)` sem olhar `c`.
    pub fn somar_alarme(&mut self, x: i32, y: i32, v: u8, teto: u8) {
        if let Some(i) = celula_do_tile(x, y) {
            self.celulas[i] = (self.celulas[i] as u16 + v as u16).min(teto as u16) as u8;
        }
    }

    /// `_AlarmHere2`: alarme de uma luta; a célula fica com o maior entre o valor atual e `v`.
    pub fn alarme_de_luta(&mut self, x: i32, y: i32, v: u8) {
        self.depositar(x, y, v);
    }

    /// `_DecTSmell` (só para as trilhas): a passagem de uma formiga tira 1 da célula, se não é 0.
    pub fn gastar(&mut self, x: i32, y: i32) {
        if let Some(i) = celula_do_tile(x, y) {
            self.celulas[i] = self.celulas[i].saturating_sub(1);
        }
    }

    // --- decaimento (seção 3) ---

    /// `_ColonySmellBN/RN`: toda célula não nula perde 1.
    pub fn decair_ninho(&mut self) {
        for c in &mut self.celulas {
            *c = c.saturating_sub(1);
        }
    }

    /// `_ColonySmellBT/RT`: abaixo do `limiar` a célula zera; senão perde a metade (arredondada
    /// para baixo), isto é, fica com `c − ⌊c/2⌋`.
    pub fn decair_trilha(&mut self, limiar: u8) {
        for c in &mut self.celulas {
            *c = if *c < limiar { 0 } else { *c - (*c >> 1) };
        }
    }

    /// `_SmoothAlarm`: cada célula vira `(c + soma_dos_4_vizinhos / 4) / 2` (divisões inteiras),
    /// calculada sobre uma cópia do mapa (`_TemDensity`); vizinho fora do mapa conta 0, sem dar a
    /// volta. Resultado abaixo do `limiar` vira 0.
    pub fn suavizar_alarme(&mut self, limiar: u8) {
        let copia = self.celulas;
        let em = |cx: usize, cy: usize| copia[cx * ALTURA + cy] as u32;
        for cx in 0..LARGURA {
            for cy in 0..ALTURA {
                let mut soma = 0;
                if cx > 0 {
                    soma += em(cx - 1, cy);
                }
                if cy > 0 {
                    soma += em(cx, cy - 1);
                }
                if cx + 1 < LARGURA {
                    soma += em(cx + 1, cy);
                }
                if cy + 1 < ALTURA {
                    soma += em(cx, cy + 1);
                }
                let novo = (em(cx, cy) + soma / 4) / 2;
                self.celulas[cx * ALTURA + cy] = if novo < limiar as u32 { 0 } else { novo as u8 };
            }
        }
    }

    /// `_FillHolesBN/RN`: para cada linha `y` da superfície com buraco (`buracos[y] = x`, 0 = sem
    /// buraco; é o `_HoleMapB/R`), a célula do tile `(x, y)` recebe `valor_buraco`, ou 0 se o tile
    /// do `_MapA` ali for `tile_buraco_fechado`. Varre `y` de 0 a 63.
    pub fn preencher_buracos(
        &mut self,
        buracos: &[u8],
        mapa_a: &Mapa<'_>,
        regras: &RegrasFeromonio,
    ) {
        for (y, &x) in buracos.iter().enumerate() {
            if x == 0 {
                continue;
            }
            // Buraco fora da superfície: o original leria além do _MapA; o port ignora.
            let (Some(tile), Some(i)) =
                (mapa_a.em(x as usize, y), celula_do_tile(x as i32, y as i32))
            else {
                continue;
            };
            self.celulas[i] = if tile == regras.tile_buraco_fechado {
                0
            } else {
                regras.valor_buraco
            };
        }
    }
}

/// Fase da manutenção periódica (`_DoSmells`), escolhida por `_Cycle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaseCheiro {
    /// Fases 0 e 2 (`_Cycle mod 128` = 0 ou 64): suavização do alarme.
    Alarme,
    /// Fase 1 (`mod 128 = 32`): buracos pretos, decaimento do ninho e da trilha pretos.
    Pretos,
    /// Fase 3 (`mod 128 = 96`): idem, vermelhos.
    Vermelhos,
}

/// Fase de cheiro que roda no tick em que `_Cycle` (já incrementado no começo do tick) vale
/// `ciclo`, ou `None` se `ciclo & 0x1F ≠ 0`.
///
/// `_Cycle` vai de 0 a 0x1000 e volta a 0, então 0x1000 e o 0 seguinte rodam a fase 0 duas vezes
/// seguidas (suavização em dois ticks consecutivos), e daí em diante as fases continuam alinhadas
/// a `_Cycle`, não a `_GameTime`.
pub fn fase_do_ciclo(ciclo: u16) -> Option<FaseCheiro> {
    if ciclo & 0x1F != 0 {
        return None;
    }
    Some(match (ciclo & 0x60) >> 5 {
        0 | 2 => FaseCheiro::Alarme,
        1 => FaseCheiro::Pretos,
        _ => FaseCheiro::Vermelhos,
    })
}

/// Os cinco mapas.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Feromonios {
    pub alarme: MapaCheiro,
    pub ninho_preto: MapaCheiro,
    pub trilha_preta: MapaCheiro,
    pub ninho_vermelho: MapaCheiro,
    pub trilha_vermelha: MapaCheiro,
}

impl Feromonios {
    pub fn camada(&self, c: Camada) -> &MapaCheiro {
        match c {
            Camada::Alarme => &self.alarme,
            Camada::NinhoPreto => &self.ninho_preto,
            Camada::TrilhaPreta => &self.trilha_preta,
            Camada::NinhoVermelho => &self.ninho_vermelho,
            Camada::TrilhaVermelha => &self.trilha_vermelha,
        }
    }

    pub fn camada_mut(&mut self, c: Camada) -> &mut MapaCheiro {
        match c {
            Camada::Alarme => &mut self.alarme,
            Camada::NinhoPreto => &mut self.ninho_preto,
            Camada::TrilhaPreta => &mut self.trilha_preta,
            Camada::NinhoVermelho => &mut self.ninho_vermelho,
            Camada::TrilhaVermelha => &mut self.trilha_vermelha,
        }
    }

    /// Lê os blocos `PherMap*` de um `.ANT`.
    pub fn do_save(save: &JogoSalvo) -> Result<Feromonios, String> {
        let mut f = Feromonios::default();
        for c in Camada::TODAS {
            let b = save
                .bloco(c.simbolo())
                .ok_or_else(|| format!("bloco {} não existe no layout", c.simbolo()))?;
            *f.camada_mut(c) =
                MapaCheiro::de_bytes(b).map_err(|e| format!("{}: {e}", c.simbolo()))?;
        }
        Ok(f)
    }

    /// Grava os cinco mapas de volta nos blocos `PherMap*` do `.ANT`.
    pub fn gravar_no_save(&self, save: &mut JogoSalvo) -> Result<(), String> {
        for c in Camada::TODAS {
            let b = save
                .bloco_mut(c.simbolo())
                .ok_or_else(|| format!("bloco {} não existe no layout", c.simbolo()))?;
            if b.len() != CELULAS {
                return Err(format!(
                    "bloco {} tem {} bytes; esperado {CELULAS}",
                    c.simbolo(),
                    b.len()
                ));
            }
            b.copy_from_slice(&self.camada(c).celulas);
        }
        Ok(())
    }

    /// Roda a fase dada de `_DoSmells`, só na parte de cheiro (as compactações de lista,
    /// `_FullCount` e `_HistUpdate` das mesmas fases ficam fora). Ordem do original:
    /// buracos → decaimento do ninho → decaimento da trilha.
    pub fn rodar_fase(
        &mut self,
        fase: FaseCheiro,
        buracos_b: &[u8],
        buracos_r: &[u8],
        mapa_a: &Mapa<'_>,
        regras: &RegrasFeromonio,
    ) {
        match fase {
            FaseCheiro::Alarme => self.alarme.suavizar_alarme(regras.limiar_alarme),
            FaseCheiro::Pretos => {
                self.ninho_preto
                    .preencher_buracos(buracos_b, mapa_a, regras);
                self.ninho_preto.decair_ninho();
                self.trilha_preta.decair_trilha(regras.limiar_trilha);
            }
            FaseCheiro::Vermelhos => {
                self.ninho_vermelho
                    .preencher_buracos(buracos_r, mapa_a, regras);
                self.ninho_vermelho.decair_ninho();
                self.trilha_vermelha.decair_trilha(regras.limiar_trilha);
            }
        }
    }

    /// Etapa de cheiro de um tick (item 5 de `_DoAntSim`): roda a fase de `ciclo`, se houver.
    /// Devolve a fase rodada.
    pub fn etapa_do_tick(
        &mut self,
        ciclo: u16,
        buracos_b: &[u8],
        buracos_r: &[u8],
        mapa_a: &Mapa<'_>,
        regras: &RegrasFeromonio,
    ) -> Option<FaseCheiro> {
        let fase = fase_do_ciclo(ciclo)?;
        self.rodar_fase(fase, buracos_b, buracos_r, mapa_a, regras);
        Some(fase)
    }

    /// `_GetSmellT`: trilha da cor dada na célula vizinha `(cx + dx8[d], cy + dy8[d])`, em
    /// coordenadas de **célula**; 0 fora do mapa (sem dar a volta). O original não tem chamadas a
    /// esta função; fica aqui pela completude.
    pub fn trilha_na_direcao(
        &self,
        tab: &TabelasFeromonio,
        cx: i32,
        cy: i32,
        d: u8,
        cor: Cor,
    ) -> u8 {
        let nx = cx + tab.dx8[d as usize] as i32;
        let ny = cy + tab.dy8[d as usize] as i32;
        if (0..LARGURA as i32).contains(&nx) && (0..ALTURA as i32).contains(&ny) {
            self.camada(Camada::trilha(cor))
                .em(nx as usize, ny as usize)
        } else {
            0
        }
    }
}

// --- escolha de direção (seção 4 da spec) ---

/// Borda usada por `_GetNestDir` e `_GetAlarmDir`: [`comum::quicar`] (`_Bounce`).
fn borda_larga(x: i32, y: i32, rng: &mut GeradorS) -> Option<u8> {
    comum::quicar(rng, x, y)
}

/// `_GetDir` ([`comum::get_dir`]) como direção 0–7, ou `None` se os pontos coincidem.
fn direcao_para(x1: i32, y1: i32, x2: i32, y2: i32) -> Option<u8> {
    let d = comum::get_dir(x1, y1, x2, y2);
    (d > 0).then(|| (d - 1) as u8)
}

/// `_GetForageDir` (`1030:0ab0`): direção de uma formiga em Foraging no tile `(x, y)`, apontando
/// para `dir`, subindo a trilha da cor `cor`. `None` significa "a trilha acabou" (−1 no original:
/// o maior vizinho é menor que a célula atual).
///
/// Ordem do gerador: na borda, `SRand1(3)` uma vez (nos cantos, nada); no interior, **sempre** um
/// `SRand8` antes de examinar os vizinhos (o valor é descartado) e um segundo `SRand8` se todos os
/// vizinhos forem 0.
pub fn direcao_forrageio(
    mapas: &Feromonios,
    tab: &TabelasFeromonio,
    x: i32,
    y: i32,
    dir: u8,
    cor: Cor,
    rng: &mut GeradorS,
) -> Option<u8> {
    // Borda estreita (3 opções, cantos fixos), diferente da de _GetNestDir.
    if x == 0 {
        return Some(if y == 0 {
            3
        } else if y == ULTIMO_Y {
            1
        } else {
            rng.rand1(3) as u8 + 1
        });
    }
    if y == 0 {
        return Some(if x == ULTIMO_X {
            5
        } else {
            rng.rand1(3) as u8 + 3
        });
    }
    if x == ULTIMO_X {
        return Some(if y == ULTIMO_Y {
            7
        } else {
            rng.rand1(3) as u8 + 5
        });
    }
    if y == ULTIMO_Y {
        return Some(((rng.rand1(3) - 1) & 7) as u8);
    }
    let mapa = mapas.camada(Camada::trilha(cor));
    let (cx, cy) = (x >> 1, y >> 1);
    let atual = mapa.em(cx as usize, cy as usize);
    let _descartado = rng.mascara(8);
    let (melhor, valor) = mapa.maior_vizinho(tab, cx, cy);
    if valor == 0 {
        let alvo = rng.mascara(8) as u8;
        return Some(tab.virar(dir, alvo));
    }
    if valor < atual {
        return None;
    }
    Some(tab.virar(dir, melhor))
}

/// `_GetNestDir` (`1030:0c30`): direção para o ninho da cor `cor` (a formiga em ataque pede a cor
/// inimiga). `buraco` é o `_LastNewHoleB/R` da cor (x, y em tiles).
///
/// - Borda: [`borda_larga`].
/// - Célula sem cheiro: se o buraco não está na mesma posição, `SRand4`; se ≠ 0, vira rumo ao
///   buraco. Senão (ou se o buraco coincide), passeio: `SRand8` como alvo do giro.
/// - Com cheiro: maior vizinho (se todos forem 0, o alvo é a direção 0); depois `SRand2`: se ≠ 0,
///   o giro é aplicado **duas vezes** (até 90°), senão uma vez (até 45°).
#[allow(clippy::too_many_arguments)]
pub fn direcao_ninho(
    mapas: &Feromonios,
    tab: &TabelasFeromonio,
    x: i32,
    y: i32,
    dir: u8,
    cor: Cor,
    buraco: (i32, i32),
    rng: &mut GeradorS,
) -> u8 {
    if let Some(d) = borda_larga(x, y, rng) {
        return d;
    }
    let mapa = mapas.camada(Camada::ninho(cor));
    let (cx, cy) = (x >> 1, y >> 1);
    if mapa.em(cx as usize, cy as usize) == 0 {
        if let Some(rumo) = direcao_para(x, y, buraco.0, buraco.1) {
            if rng.mascara(4) != 0 {
                return tab.virar(dir, rumo);
            }
        }
        let alvo = rng.mascara(8) as u8;
        return tab.virar(dir, alvo);
    }
    let (melhor, _) = mapa.maior_vizinho(tab, cx, cy);
    let mut d = dir;
    if rng.mascara(2) != 0 {
        d = tab.virar(d, melhor);
    }
    tab.virar(d, melhor)
}

/// `_GetAlarmDir` (`1030:0e54`): sobe o gradiente do alarme. Borda: [`borda_larga`]. Interior:
/// maior vizinho com giro de até 45°; sem alarme em volta, `SRand8` como alvo do giro.
pub fn direcao_alarme(
    mapas: &Feromonios,
    tab: &TabelasFeromonio,
    x: i32,
    y: i32,
    dir: u8,
    rng: &mut GeradorS,
) -> u8 {
    if let Some(d) = borda_larga(x, y, rng) {
        return d;
    }
    let (melhor, valor) = mapas.alarme.maior_vizinho(tab, x >> 1, y >> 1);
    let alvo = if valor == 0 {
        rng.mascara(8) as u8
    } else {
        melhor
    };
    tab.virar(dir, alvo)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn tabelas() -> TabelasFeromonio {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../data")
            .join(ARQUIVO_TABELAS);
        TabelasFeromonio::de_json(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    fn mapa_a_vazio(dados: &[u8]) -> Mapa<'_> {
        Mapa {
            largura: 128,
            altura: 64,
            dados,
        }
    }

    #[test]
    fn indice_da_celula() {
        assert_eq!(celula_do_tile(0, 0), Some(0));
        assert_eq!(celula_do_tile(1, 1), Some(0));
        assert_eq!(celula_do_tile(3, 32), Some(32 + 16));
        assert_eq!(celula_do_tile(127, 63), Some(63 * 32 + 31));
        assert_eq!(celula_do_tile(128, 0), None);
        assert_eq!(celula_do_tile(0, 64), None);
        assert_eq!(celula_do_tile(-1, 0), None);
    }

    #[test]
    fn decaimento_do_ninho_tira_1_e_para_em_0() {
        let mut m = MapaCheiro::default();
        m.definir(5, 5, 255);
        m.definir(6, 6, 1);
        m.decair_ninho();
        assert_eq!(m.em(5, 5), 254);
        assert_eq!(m.em(6, 6), 0);
        assert_eq!(m.em(0, 0), 0);
    }

    #[test]
    fn decaimento_da_trilha_pela_metade_arredondando_para_cima() {
        let r = tabelas().regras;
        let mut m = MapaCheiro::default();
        for (i, v) in [210u8, 105, 53, 27, 8, 7, 1, 255, 9]
            .into_iter()
            .enumerate()
        {
            m.celulas[i] = v;
        }
        m.decair_trilha(r.limiar_trilha);
        assert_eq!(&m.celulas[..9], &[105, 53, 27, 14, 4, 0, 0, 128, 5]);
        // Sequência do roteiro de teste da spec (seção 6): 210 → 105 → 53 → 27.
    }

    #[test]
    fn suavizacao_de_pico_isolado() {
        let r = tabelas().regras;
        let mut m = MapaCheiro::default();
        m.definir(10, 10, 200);
        m.suavizar_alarme(r.limiar_alarme);
        assert_eq!(m.em(10, 10), 100);
        for (x, y) in [(9, 10), (11, 10), (10, 9), (10, 11)] {
            assert_eq!(m.em(x, y), 25, "vizinho ({x}, {y})");
        }
        assert_eq!(m.em(9, 9), 0, "diagonal não recebe");
        assert_eq!(m.celulas.iter().filter(|v| **v != 0).count(), 5);
        // Segunda passada: centro (100 + 100/4)/2 = 62; vizinho (25 + 100/4)/2 = 25;
        // diagonal (0 + 50/4)/2 = 6 < 9 → 0; a dois passos (0 + 25/4)/2 = 3 → 0.
        m.suavizar_alarme(r.limiar_alarme);
        assert_eq!(m.em(10, 10), 62);
        assert_eq!(m.em(9, 10), 25);
        assert_eq!(m.em(9, 9), 0);
        assert_eq!(m.em(8, 10), 0);
    }

    #[test]
    fn suavizacao_usa_copia_e_nao_da_volta_na_borda() {
        let r = tabelas().regras;
        let mut m = MapaCheiro::default();
        m.definir(0, 0, 200);
        m.definir(1, 0, 200);
        m.suavizar_alarme(r.limiar_alarme);
        // (200 + 200/4)/2 = 125 nos dois: cada um vê o outro com o valor antigo.
        assert_eq!(m.em(0, 0), 125);
        assert_eq!(m.em(1, 0), 125);
        // Sem volta: a coluna 63 e a linha 31 não recebem nada.
        assert_eq!(m.em(63, 0), 0);
        assert_eq!(m.em(0, 31), 0);
        // Limiar: (17 + 0)/2 = 8 < 9 vira 0; (18 + 0)/2 = 9 fica.
        let mut m = MapaCheiro::default();
        m.definir(20, 20, 17);
        m.definir(40, 20, 18);
        m.suavizar_alarme(r.limiar_alarme);
        assert_eq!(m.em(20, 20), 0);
        assert_eq!(m.em(40, 20), 9);
    }

    #[test]
    fn deposito_e_maximo() {
        let mut m = MapaCheiro::default();
        m.depositar(10, 20, 119);
        assert_eq!(m.no_tile(11, 21), 119, "mesma célula 2 × 2");
        m.depositar(11, 21, 100);
        assert_eq!(m.no_tile(10, 20), 119);
        m.depositar(10, 20, 200);
        assert_eq!(m.no_tile(10, 20), 200);
        m.depositar(500, 20, 50); // fora do mapa: ignorado
        m.alarme_de_luta(4, 4, 40);
        assert_eq!(m.no_tile(4, 4), 40);
        m.alarme_de_luta(4, 4, 30);
        assert_eq!(m.no_tile(4, 4), 40);
    }

    #[test]
    fn alarme_da_amarela_soma_com_teto() {
        let teto = tabelas().regras.teto_alarme_amarela;
        let mut m = MapaCheiro::default();
        for esperado in [50, 100, 150, 200, 200] {
            m.somar_alarme(7, 7, 50, teto);
            assert_eq!(m.no_tile(7, 7), esperado);
        }
        // Acima do teto (ferramenta Experimental): cai para o teto.
        m.celulas[celula_do_tile(9, 9).unwrap()] = 250;
        m.somar_alarme(9, 9, 50, teto);
        assert_eq!(m.no_tile(9, 9), 200);
    }

    #[test]
    fn gasto_da_trilha() {
        let mut m = MapaCheiro::default();
        m.depositar(2, 2, 2);
        m.gastar(2, 2);
        m.gastar(2, 2);
        m.gastar(2, 2);
        assert_eq!(m.no_tile(2, 2), 0);
    }

    #[test]
    fn buracos() {
        let r = tabelas().regras;
        let mut tiles = vec![0u8; 128 * 64];
        let mut buracos = [0u8; 64];
        buracos[32] = 3; // buraco em (3, 32)
        buracos[10] = 40; // buraco "fechado" em (40, 10)
        tiles[40 * 64 + 10] = r.tile_buraco_fechado;
        buracos[11] = 200; // fora da superfície: ignorado
        let mut m = MapaCheiro::default();
        m.definir(20, 5, 77);
        m.preencher_buracos(&buracos, &mapa_a_vazio(&tiles), &r);
        assert_eq!(m.em(1, 16), 255);
        assert_eq!(m.em(20, 5), 0);
        assert_eq!(m.celulas.iter().filter(|v| **v != 0).count(), 1);
    }

    #[test]
    fn fases_do_ciclo() {
        assert_eq!(fase_do_ciclo(0), Some(FaseCheiro::Alarme));
        assert_eq!(fase_do_ciclo(32), Some(FaseCheiro::Pretos));
        assert_eq!(fase_do_ciclo(64), Some(FaseCheiro::Alarme));
        assert_eq!(fase_do_ciclo(96), Some(FaseCheiro::Vermelhos));
        assert_eq!(fase_do_ciclo(128 + 32), Some(FaseCheiro::Pretos));
        assert_eq!(fase_do_ciclo(0x1000), Some(FaseCheiro::Alarme));
        for c in [1, 31, 33, 63, 78, 0xFFF] {
            assert_eq!(fase_do_ciclo(c), None);
        }
    }

    #[test]
    fn tabela_de_giro() {
        let t = tabelas();
        for a in 0..8u8 {
            assert_eq!(t.virar(a, a), a);
            for b in 0..8u8 {
                let n = t.virar(a, b);
                let passo = (n + 8 - a) % 8;
                assert!(passo <= 1 || passo == 7, "giro de {a} para {b} deu {n}");
            }
        }
        // Direção oposta: 0 → 4 gira para a esquerda (7); 1 → 5 para a direita (2).
        assert_eq!(t.virar(0, 4), 7);
        assert_eq!(t.virar(1, 5), 2);
    }

    #[test]
    fn direcao_para_segue_os_sinais() {
        assert_eq!(direcao_para(5, 5, 5, 5), None);
        assert_eq!(direcao_para(5, 5, 5, 0), Some(0));
        assert_eq!(direcao_para(5, 5, 9, 1), Some(1));
        assert_eq!(direcao_para(5, 5, 9, 5), Some(2));
        assert_eq!(direcao_para(5, 5, 9, 9), Some(3));
        assert_eq!(direcao_para(5, 5, 5, 9), Some(4));
        assert_eq!(direcao_para(5, 5, 1, 9), Some(5));
        assert_eq!(direcao_para(5, 5, 1, 5), Some(6));
        assert_eq!(direcao_para(5, 5, 1, 1), Some(7));
    }

    #[test]
    fn forrageio_sobe_a_trilha_e_detecta_o_fim() {
        let t = tabelas();
        let mut f = Feromonios::default();
        // Formiga no tile (20, 20) → célula (10, 10), apontando para cima (0).
        f.trilha_preta.definir(10, 10, 50);
        f.trilha_preta.definir(11, 10, 60); // leste
        let mut rng = GeradorS::new(0x1234);
        let mut esperado = GeradorS::new(0x1234);
        esperado.mascara(8);
        assert_eq!(
            direcao_forrageio(&f, &t, 20, 20, 0, Cor::Preta, &mut rng),
            Some(1),
            "gira 45° rumo ao leste"
        );
        assert_eq!(rng, esperado, "um SRand8 descartado");
        // Vizinho menor que a célula atual: fim da trilha.
        f.trilha_preta.definir(11, 10, 40);
        assert_eq!(
            direcao_forrageio(&f, &t, 20, 20, 2, Cor::Preta, &mut rng),
            None
        );
        // A trilha vermelha está vazia: passeio, dois SRand8.
        let mut r1 = GeradorS::new(0x1234);
        let mut r2 = GeradorS::new(0x1234);
        r2.mascara(8);
        let alvo = r2.mascara(8) as u8;
        assert_eq!(
            direcao_forrageio(&f, &t, 20, 20, 2, Cor::Vermelha, &mut r1),
            Some(t.virar(2, alvo))
        );
        assert_eq!(r1, r2);
    }

    #[test]
    fn forrageio_na_borda() {
        let t = tabelas();
        let f = Feromonios::default();
        let mut rng = GeradorS::new(0x4321);
        let copia = rng;
        assert_eq!(
            direcao_forrageio(&f, &t, 0, 0, 0, Cor::Preta, &mut rng),
            Some(3)
        );
        assert_eq!(
            direcao_forrageio(&f, &t, 0, 63, 0, Cor::Preta, &mut rng),
            Some(1)
        );
        assert_eq!(
            direcao_forrageio(&f, &t, 127, 0, 0, Cor::Preta, &mut rng),
            Some(5)
        );
        assert_eq!(
            direcao_forrageio(&f, &t, 127, 63, 0, Cor::Preta, &mut rng),
            Some(7)
        );
        assert_eq!(rng, copia, "cantos não sorteiam");
        for _ in 0..50 {
            assert!((1..=3)
                .contains(&direcao_forrageio(&f, &t, 0, 30, 0, Cor::Preta, &mut rng).unwrap()));
            assert!((3..=5)
                .contains(&direcao_forrageio(&f, &t, 60, 0, 0, Cor::Preta, &mut rng).unwrap()));
            assert!((5..=7)
                .contains(&direcao_forrageio(&f, &t, 127, 30, 0, Cor::Preta, &mut rng).unwrap()));
            assert!([7, 0, 1]
                .contains(&direcao_forrageio(&f, &t, 60, 63, 0, Cor::Preta, &mut rng).unwrap()));
        }
    }

    #[test]
    fn borda_larga_aponta_para_dentro() {
        let mut rng = GeradorS::new(0x0BAD);
        let casos: [((i32, i32), &[u8]); 8] = [
            ((0, 0), &[2, 3, 4]),
            ((0, 63), &[0, 1, 2]),
            ((0, 30), &[0, 1, 2, 3, 4]),
            ((60, 0), &[2, 3, 4, 5, 6]),
            ((127, 0), &[4, 5, 6]),
            ((127, 30), &[4, 5, 6, 7, 0]),
            ((127, 63), &[6, 7, 0]),
            ((60, 63), &[6, 7, 0, 1, 2]),
        ];
        for ((x, y), opcoes) in casos {
            for _ in 0..60 {
                let d = borda_larga(x, y, &mut rng).unwrap();
                assert!(opcoes.contains(&d), "({x}, {y}) deu {d}");
            }
        }
        let antes = rng;
        assert_eq!(borda_larga(60, 30, &mut rng), None);
        assert_eq!(rng, antes);
    }

    #[test]
    fn ninho_com_cheiro_gira_uma_ou_duas_vezes() {
        let t = tabelas();
        let mut f = Feromonios::default();
        f.ninho_preto.definir(10, 10, 100);
        f.ninho_preto.definir(10, 11, 110); // sul (4)
                                            // Apontando para cima (0), alvo 4: um giro → 7, dois giros → 6.
        for semente in 1..200u16 {
            let mut rng = GeradorS::new(semente);
            let mut prev = GeradorS::new(semente);
            let dois = prev.mascara(2) != 0;
            let d = direcao_ninho(&f, &t, 20, 20, 0, Cor::Preta, (3, 32), &mut rng);
            assert_eq!(d, if dois { 6 } else { 7 });
            assert_eq!(rng, prev);
        }
    }

    #[test]
    fn ninho_sem_cheiro_aponta_para_o_buraco() {
        let t = tabelas();
        let f = Feromonios::default();
        for semente in 1..200u16 {
            let mut rng = GeradorS::new(semente);
            let mut prev = GeradorS::new(semente);
            // Buraco a oeste (6), formiga apontando para oeste: fica em 6 se SRand4 ≠ 0.
            let d = direcao_ninho(&f, &t, 40, 20, 6, Cor::Preta, (3, 20), &mut rng);
            let esperado = if prev.mascara(4) != 0 {
                6
            } else {
                t.virar(6, prev.mascara(8) as u8)
            };
            assert_eq!(d, esperado);
            assert_eq!(rng, prev);
        }
        // Formiga em cima do buraco: não sorteia SRand4, só SRand8.
        let mut rng = GeradorS::new(77);
        let mut prev = GeradorS::new(77);
        let d = direcao_ninho(&f, &t, 3, 20, 2, Cor::Preta, (3, 20), &mut rng);
        assert_eq!(d, t.virar(2, prev.mascara(8) as u8));
        assert_eq!(rng, prev);
    }

    #[test]
    fn alarme_vizinhanca_da_volta() {
        let t = tabelas();
        let mut f = Feromonios::default();
        // Na coluna 63 (tile 126), o vizinho a leste é a coluna 0 (dá a volta).
        f.alarme.definir(0, 10, 90);
        let mut rng = GeradorS::new(5);
        let antes = rng;
        assert_eq!(direcao_alarme(&f, &t, 126, 20, 2, &mut rng), 2);
        assert_eq!(rng, antes, "com alarme em volta não sorteia");
        // Sem alarme: SRand8.
        let f = Feromonios::default();
        let mut prev = rng;
        assert_eq!(
            direcao_alarme(&f, &t, 60, 20, 0, &mut rng),
            t.virar(0, prev.mascara(8) as u8)
        );
    }

    #[test]
    fn trilha_na_direcao_nao_da_volta() {
        let t = tabelas();
        let mut f = Feromonios::default();
        f.trilha_vermelha.definir(1, 0, 33);
        assert_eq!(f.trilha_na_direcao(&t, 0, 0, 2, Cor::Vermelha), 33);
        assert_eq!(f.trilha_na_direcao(&t, 0, 0, 2, Cor::Preta), 0);
        assert_eq!(f.trilha_na_direcao(&t, 0, 0, 6, Cor::Vermelha), 0);
    }
}
