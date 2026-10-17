//! Janela do mapa (janela 1, `mapWin`): a visão geral do plano em 4 × 4 pixels por célula, com as
//! formigas, os mapas de feromônio, as trilhas (`_showTrails`), a aranha, a pegada e o retângulo da
//! vista de edição. Especificação: docs/spec/telas/mapa.md; dados: data/mapa.json.
//!
//! Nada aqui sorteia (as rotinas de desenho do mapa do original não chamam `SRand`/`RRand`).

use std::collections::HashMap;

use serde::Deserialize;

use super::navegacao::Navegacao;
use super::quadro::Quadro;
use crate::formats::banco::Chave;
use crate::formats::bitmap::{Bitmap, TIPO_BITMAP};
use crate::formats::janela::Retangulo;
use crate::recursos::Recursos;
use crate::rules::estado::{Estado, Grade};

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_CONFIG: &str = "mapa.json";

/// Largura e altura do mapa em células (a superfície; os ninhos ocupam as 64 do meio).
pub const COLUNAS: usize = 128;
pub const LINHAS: usize = 64;
/// Pixels por célula (`_mapXsize`/`_mapYsize`).
pub const LADO: i32 = 4;

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigMapa {
    pub janela: u16,
    pub objeto_area: String,
    pub lista_titulos: u16,
    pub ctab: Vec<u8>,
    pub htab: Vec<u8>,
    pub ltab: Vec<u8>,
    pub ctab_r: Vec<u8>,
    /// Cores 16..31: blocos 4 × 4 (linhas de cima para baixo).
    pub blocos: Vec<[[u8; 4]; 4]>,
    pub conv: [u8; 16],
    pub bitmap_aranha: u16,
    pub bitmap_pe: u16,
    pub bitmap_cortador: u16,
    pub cor_transparente: u8,
    pub espessura_cursor: i16,
    /// Objeto da faixa -> `_MapMode` (4 a 8).
    pub botoes_cheiro: HashMap<String, i16>,
}

impl ConfigMapa {
    /// `_ConvColor(c)`.
    fn conv(&self, c: usize) -> u8 {
        self.conv[c & 15]
    }
}

/// Estado da janela do mapa (fora do save, como no original): o buffer de células da última
/// geração (`_lastMapMapBuf`), base das trilhas, e o modo com que foi gerado.
#[derive(Clone, Debug)]
pub struct JanelaMapa {
    pub cfg: ConfigMapa,
    pub celulas: Vec<u8>,
    ultimo_modo: Option<i16>,
    /// `_GameTime` da última geração (o original gera em `_UpdateWindows`, depois do tick).
    ultimo_tempo: Option<u32>,
    bitmaps: HashMap<u16, Bitmap>,
    titulos: Vec<String>,
}

fn indice(x: usize, y: usize) -> usize {
    y * COLUNAS + x
}

/// Cor de uma célula de ninho (`_GenNestMap`): sem vida, `_CTabR[terreno >> 2]`; a amarela,
/// `_ConvColor(1)`; pretas (vida < 0x80), `_ConvColor(0xF)`; vermelhas, `_ConvColor(3)`.
fn cor_ninho(cfg: &ConfigMapa, terreno: u8, vida: u8) -> Option<u8> {
    match vida {
        0 => cfg.ctab_r.get(usize::from(terreno >> 2)).copied(),
        0xFE | 0xFF => Some(cfg.conv(1)),
        v if v < 0x80 => Some(cfg.conv(0xF)),
        _ => Some(cfg.conv(3)),
    }
}

impl JanelaMapa {
    pub fn carregar(recursos: &Recursos) -> Result<JanelaMapa, String> {
        let cfg: ConfigMapa = recursos.dados(ARQUIVO_CONFIG)?;
        let mut bitmaps = HashMap::new();
        let ids = (0..8).map(|d| cfg.bitmap_aranha + d).chain((0..4).map(|d| cfg.bitmap_pe + d)).chain([cfg.bitmap_cortador, cfg.bitmap_cortador + 1]);
        for id in ids {
            if let Ok(b) = recursos.objeto(Chave { tipo: TIPO_BITMAP, id }).and_then(|d| Bitmap::decodificar(&d)) {
                bitmaps.insert(id, b);
            }
        }
        let titulos = super::avisos::linhas_do_texto(recursos, cfg.lista_titulos)
            .iter()
            .map(|l| l.iter().map(|&b| b as char).collect())
            .collect();
        Ok(JanelaMapa { cfg, celulas: vec![0; COLUNAS * LINHAS], ultimo_modo: None, ultimo_tempo: None, bitmaps, titulos })
    }

    /// Título (`_SetMapTitle`): a entrada `_MapMode` da lista 1000.
    pub fn titulo(&self, e: &Estado) -> String {
        usize::try_from(e.vistas.modo_mapa).ok().and_then(|m| self.titulos.get(m).cloned()).unwrap_or_default()
    }

    /// Gera as células (o `FUN_1010_d7a0` do original) se o tick avançou ou o modo mudou; com
    /// trilhas (`_showTrails`) e o mesmo modo, parte das células da última geração e só escreve
    /// as que têm vida, acumulando os rastros.
    pub fn atualizar(&mut self, e: &Estado, trilhas: bool) {
        let modo = e.vistas.modo_mapa;
        let tempo = e.partida.tempo;
        if self.ultimo_modo == Some(modo) && self.ultimo_tempo == Some(tempo) {
            return;
        }
        let acumula = trilhas && self.ultimo_modo == Some(modo);
        if !acumula {
            self.celulas.iter_mut().for_each(|c| *c = 0);
        }
        let cfg = &self.cfg;
        match modo {
            2 | 3 => {
                let n = if modo == 2 { &e.ninho_b } else { &e.ninho_r };
                let faixa = cfg.conv(0xF);
                for y in 0..LINHAS {
                    for x in 0..COLUNAS {
                        let i = indice(x, y);
                        if !(32..96).contains(&x) {
                            self.celulas[i] = faixa;
                            continue;
                        }
                        let (nx, ny) = ((x - 32) as i32, y as i32);
                        let vida = n.vida.em(nx, ny).unwrap_or(0);
                        if trilhas && vida == 0 && acumula {
                            continue;
                        }
                        if let Some(c) = cor_ninho(cfg, n.mapa.em(nx, ny).unwrap_or(0), vida) {
                            self.celulas[i] = c;
                        }
                    }
                }
            }
            4..=8 => {
                let f = &e.feromonios;
                let g: &Grade = match modo {
                    4 => &f.ninho_b,
                    5 => &f.trilha_b,
                    6 => &f.ninho_r,
                    7 => &f.trilha_r,
                    _ => &f.alarme,
                };
                for y in 0..LINHAS {
                    for x in 0..COLUNAS {
                        let v = g.em((x / 2) as i32, (y / 2) as i32).unwrap_or(0);
                        self.celulas[indice(x, y)] = if v == 0 { cfg.conv(0xB) } else { 0x10 + (v >> 4) };
                    }
                }
            }
            _ => {
                let casa = crate::rules::comum::em_casa(e);
                let tab = if casa { &cfg.htab } else { &cfg.ctab };
                for y in 0..LINHAS {
                    for x in 0..COLUNAS {
                        let (cx, cy) = (x as i32, y as i32);
                        let vida = e.superficie.vida.em(cx, cy).unwrap_or(0);
                        let c = if vida != 0 {
                            cfg.ltab.get(usize::from(vida >> 3)).copied()
                        } else if acumula {
                            None
                        } else {
                            tab.get(usize::from(e.superficie.mapa.em(cx, cy).unwrap_or(0))).copied()
                        };
                        if let Some(c) = c {
                            self.celulas[indice(x, y)] = c;
                        }
                    }
                }
            }
        }
        self.ultimo_modo = Some(modo);
        self.ultimo_tempo = Some(tempo);
    }

    /// Desenha o conteúdo em `area` (canto da área do mapa): as células, a aranha e a pegada (no
    /// plano 1, sem trilhas) e o retângulo XOR da vista de edição.
    pub fn desenhar(&self, q: &mut Quadro, area: Retangulo, e: &Estado, nav: &Navegacao, trilhas: bool) {
        let (ox, oy) = (i32::from(area.esq), i32::from(area.topo));
        let antes = q.recorte.replace(Retangulo { dir: area.esq + (COLUNAS as i16) * 4, base: area.topo + (LINHAS as i16) * 4, ..area });
        for y in 0..LINHAS {
            for x in 0..COLUNAS {
                let c = self.celulas[indice(x, y)];
                let (px, py) = (ox + x as i32 * LADO, oy + y as i32 * LADO);
                if c < 16 {
                    q.preencher(Retangulo { esq: px as i16, topo: py as i16, dir: (px + LADO) as i16, base: (py + LADO) as i16 }, c, c);
                } else if let Some(b) = self.cfg.blocos.get(usize::from(c - 16)) {
                    for (j, linha) in b.iter().enumerate() {
                        for (i, &cor) in linha.iter().enumerate() {
                            q.ponto(px + i as i32, py + j as i32, cor);
                        }
                    }
                }
            }
        }
        if nav.plano_mapa == 1 && !trilhas {
            let t = self.cfg.cor_transparente;
            if e.aranha.ligada != 0 {
                let id = self.cfg.bitmap_aranha.wrapping_add(e.aranha.dir as u16);
                if let Some(b) = self.bitmaps.get(&id) {
                    q.bitmap_transparente(b, ox + 4 * (i32::from(e.aranha.x) >> 4), oy + 4 * (i32::from(e.aranha.y) >> 4), t);
                }
            }
            let m = &e.menino;
            if m.pe_presente != 0 {
                if let Some(b) = self.bitmaps.get(&(self.cfg.bitmap_pe + (m.dir & 3) as u16)) {
                    q.bitmap_transparente(b, ox + 4 * i32::from(m.pe_x), oy + 4 * i32::from(m.pe_y), t);
                }
                if m.presente == 3 || m.presente == 4 {
                    if let Some(b) = self.bitmaps.get(&(self.cfg.bitmap_cortador + (!m.dir & 1) as u16)) {
                        q.bitmap_transparente(b, ox + 4 * i32::from(m.cortador_x), oy + 4 * i32::from(m.cortador_y), t);
                    }
                }
            }
        }
        // Cursor: a vista de edição, contorno XOR de 2 pixels (`_DrawMapCursor`).
        let just = if matches!(e.vistas.modo_mapa, 2 | 3) { 32 * LADO } else { 0 };
        let v = &nav.vista;
        let (cx, cy) = (ox + LADO * v.x + just, oy + LADO * v.y);
        let (w, h) = (LADO * v.largura, LADO * v.altura);
        let k = i32::from(self.cfg.espessura_cursor);
        let r = |x0: i32, y0: i32, x1: i32, y1: i32| Retangulo { esq: x0 as i16, topo: y0 as i16, dir: x1 as i16, base: y1 as i16 };
        q.inverter(r(cx, cy, cx + w, cy + k));
        q.inverter(r(cx, cy + h - k, cx + w, cy + h));
        q.inverter(r(cx, cy + k, cx + k, cy + h - k));
        q.inverter(r(cx + w - k, cy + k, cx + w, cy + h - k));
        q.recorte = antes;
    }

    /// Célula do plano sob (x, y) da área do mapa (`_MapAreaEvent`): x/4, y/4, menos 32 nos ninhos.
    pub fn celula_em(&self, area: Retangulo, e: &Estado, x: i32, y: i32) -> Option<(i32, i32)> {
        let (dx, dy) = (x - i32::from(area.esq), y - i32::from(area.topo));
        if dx < 0 || dy < 0 || dy >= LINHAS as i32 * LADO {
            return None;
        }
        let ninho = matches!(e.vistas.modo_mapa, 2 | 3);
        let (cx, cy) = (dx / LADO - if ninho { 32 } else { 0 }, dy / LADO);
        let limite = if ninho { 64 } else { COLUNAS as i32 };
        (0..limite).contains(&cx).then_some((cx, cy))
    }
}

/// `_SetMapModeAnt(n)` (`1000:9e82`): n de 4 a 8 alterna com o mapa simples (1); os feromônios só
/// existem na superfície (devolve se precisa trocar para o plano 1). Grava `_MapMode`.
pub fn modo_do_botao_de_cheiro(atual: i16, n: i16) -> i16 {
    if (4..=8).contains(&n) && atual == n {
        1
    } else {
        n
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cfg() -> ConfigMapa {
        serde_json::from_str(include_str!("../../../data/mapa.json")).unwrap()
    }

    fn janela() -> JanelaMapa {
        JanelaMapa { cfg: cfg(), celulas: vec![0; COLUNAS * LINHAS], ultimo_modo: None, ultimo_tempo: None, bitmaps: HashMap::new(), titulos: Vec::new() }
    }

    #[test]
    fn tabelas_convertidas() {
        let c = cfg();
        assert_eq!(c.ctab.len(), 0x90);
        assert_eq!(c.ctab[0], 3, "terra do quintal");
        assert_eq!(c.ctab[0x48], 10, "comida");
        assert_eq!(c.ltab[31], 11, "vida 0xF8+ (amarela) amarela");
        assert_eq!(c.blocos.len(), 16);
        assert_eq!(c.blocos[15], [[13; 4]; 4], "a cor 31 é magenta lisa");
    }

    #[test]
    fn superficie_formiga_por_cima_e_trilhas_acumulam() {
        let mut j = janela();
        let mut e = Estado::vazio();
        e.vistas.modo_mapa = 1;
        e.superficie.vida.definir(10, 5, 0x20);
        j.atualizar(&e, true);
        assert_eq!(j.celulas[indice(10, 5)], j.cfg.ltab[4]);
        assert_eq!(j.celulas[indice(11, 5)], j.cfg.ctab[0]);
        // A formiga anda: com trilhas, a célula antiga continua pintada.
        e.superficie.vida.definir(10, 5, 0);
        e.superficie.vida.definir(11, 5, 0x20);
        e.partida.tempo += 1;
        j.atualizar(&e, true);
        assert_eq!(j.celulas[indice(10, 5)], j.cfg.ltab[4]);
        assert_eq!(j.celulas[indice(11, 5)], j.cfg.ltab[4]);
        // Sem trilhas, o chão volta.
        e.partida.tempo += 1;
        j.atualizar(&e, false);
        assert_eq!(j.celulas[indice(10, 5)], j.cfg.ctab[0]);
    }

    #[test]
    fn feromonio_em_blocos_de_2_por_2_e_ninho_centrado() {
        let mut j = janela();
        let mut e = Estado::vazio();
        e.vistas.modo_mapa = 8;
        e.feromonios.alarme.definir(3, 4, 0x55);
        j.atualizar(&e, false);
        for (x, y) in [(6, 8), (7, 8), (6, 9), (7, 9)] {
            assert_eq!(j.celulas[indice(x, y)], 0x15);
        }
        assert_eq!(j.celulas[indice(8, 8)], j.cfg.conv(0xB));
        e.vistas.modo_mapa = 2;
        e.ninho_b.vida.definir(0, 0, 0x30);
        j.atualizar(&e, false);
        assert_eq!(j.celulas[indice(0, 0)], j.cfg.conv(0xF), "faixa lateral");
        assert_eq!(j.celulas[indice(32, 0)], j.cfg.conv(0xF), "preta");
        let area = Retangulo { esq: 100, topo: 50, dir: 612, base: 306 };
        assert_eq!(j.celula_em(area, &e, 100 + 4 * 33, 50 + 4), Some((1, 1)));
        assert_eq!(j.celula_em(area, &e, 100 + 4, 50), None);
        assert_eq!(modo_do_botao_de_cheiro(5, 5), 1);
        assert_eq!(modo_do_botao_de_cheiro(1, 5), 5);
    }
}
