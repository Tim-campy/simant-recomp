//! Composição da camada de chão dos mapas (superfície e ninhos) em tiles de 16×16.
//! Especificação: docs/spec/formatos/tiles.md, seções "Indexação dos mapas" e "Montagem da imagem".

use super::quadro::Quadro;
use crate::formats::tiles::{Tile, Tileset, LADO, TRANSPARENTE};

/// Qual mapa está sendo desenhado (`_MapPlane`: 0/1 = superfície, 2 = ninho preto, 3 = ninho vermelho).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plano {
    Superficie,
    NinhoPreto,
    NinhoVermelho,
}

/// Alturas dos mapas em tiles; os bytes estão em colunas (índice = x × 64 + y).
pub const ALTURA: usize = 64;
/// Deslocamento subtraído (com estouro de byte) dos valores dos ninhos em `_DoCalcTile`.
const DESLOCAMENTO_NINHO: u8 = 0x70;
/// Primeiro tile das camadas de feromônio no tileset de chão (`(v >> 4) - 0x10` em byte).
const BASE_FEROMONIO: u8 = 0xF0;

impl Plano {
    /// Largura em tiles: 128 na superfície (`x & 0x7f`), 64 nos ninhos (`x & 0x3f`).
    pub fn largura(self) -> usize {
        match self {
            Plano::Superficie => 128,
            Plano::NinhoPreto | Plano::NinhoVermelho => 64,
        }
    }

    pub fn bytes(self) -> usize {
        self.largura() * ALTURA
    }

    /// Índice no tileset de chão para um byte do mapa.
    pub fn tile_de_chao(self, valor: u8) -> u8 {
        match self {
            Plano::Superficie => valor,
            Plano::NinhoPreto | Plano::NinhoVermelho => valor.wrapping_sub(DESLOCAMENTO_NINHO),
        }
    }
}

/// Tile de feromônio para um byte de uma camada `_PherMap*` (64×32, uma célula por 2×2 tiles),
/// ou None se o valor não passa de 0x10 e o chão normal aparece.
pub fn tile_de_feromonio(valor: u8) -> Option<u8> {
    (valor > 0x10).then(|| BASE_FEROMONIO.wrapping_add(valor >> 4))
}

/// Mapa de chão já validado.
pub struct MapaChao<'a> {
    pub plano: Plano,
    bytes: &'a [u8],
    /// Camada de feromônio sobreposta (só na superfície; 2048 bytes), como com `_ELayerMode` ≠ −1.
    feromonio: Option<&'a [u8]>,
}

impl<'a> MapaChao<'a> {
    pub fn new(plano: Plano, bytes: &'a [u8]) -> Result<MapaChao<'a>, String> {
        if bytes.len() != plano.bytes() {
            return Err(format!("mapa {plano:?} com {} bytes, esperava {}", bytes.len(), plano.bytes()));
        }
        Ok(MapaChao { plano, bytes, feromonio: None })
    }

    pub fn com_feromonio(mut self, camada: &'a [u8]) -> Result<MapaChao<'a>, String> {
        if self.plano != Plano::Superficie || camada.len() != (self.plano.largura() / 2) * (ALTURA / 2) {
            return Err("camada de feromônio só existe na superfície, com 2048 bytes".into());
        }
        self.feromonio = Some(camada);
        Ok(self)
    }

    /// Byte do mapa na coluna `x`, linha `y` (sem envolver; o chamador garante os limites).
    pub fn valor(&self, x: usize, y: usize) -> u8 {
        self.bytes[x * ALTURA + y]
    }

    /// Índice no tileset de chão para a célula (x, y).
    pub fn tile(&self, x: usize, y: usize) -> u8 {
        if let Some(f) = self.feromonio {
            if let Some(t) = tile_de_feromonio(f[(x / 2) * (ALTURA / 2) + y / 2]) {
                return t;
            }
        }
        self.plano.tile_de_chao(self.valor(x, y))
    }

    /// Desenha as células [x0, x0+largura) × [y0, y0+altura) com o canto superior esquerdo em
    /// `destino` (pixels). Células fora do mapa são ignoradas. Tiles ausentes no tileset viram
    /// erro, para não esconder um índice errado.
    pub fn desenhar(
        &self,
        q: &mut Quadro,
        tileset: &Tileset,
        celulas: (usize, usize, usize, usize),
        destino: (i32, i32),
    ) -> Result<(), String> {
        let (x0, y0, largura, altura) = celulas;
        let x1 = (x0 + largura).min(self.plano.largura());
        let y1 = (y0 + altura).min(ALTURA);
        for x in x0..x1 {
            for y in y0..y1 {
                let i = self.tile(x, y);
                let t = tileset.tile(i as usize).ok_or_else(|| format!("tile {i} ausente em ({x}, {y})"))?;
                let px = destino.0 + ((x - x0) * LADO) as i32;
                let py = destino.1 + ((y - y0) * LADO) as i32;
                desenhar_tile(q, t, px, py, None);
            }
        }
        Ok(())
    }
}

/// Copia um tile para o quadro. Com `transparente`, pixels com esse índice não são desenhados
/// (tiles de vida usam `TRANSPARENTE`).
pub fn desenhar_tile(q: &mut Quadro, t: &Tile, x: i32, y: i32, transparente: Option<u8>) {
    for j in 0..LADO {
        for i in 0..LADO {
            let c = t[j * LADO + i];
            if Some(c) != transparente {
                q.ponto(x + i as i32, y + j as i32, c);
            }
        }
    }
}

/// Atalho para tiles de vida (formigas, ovos etc.) sobre o chão já desenhado.
pub fn desenhar_tile_de_vida(q: &mut Quadro, t: &Tile, x: i32, y: i32) {
    desenhar_tile(q, t, x, y, Some(TRANSPARENTE));
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ninhos_subtraem_0x70_com_estouro() {
        assert_eq!(Plano::NinhoPreto.tile_de_chao(0x2e), 0xbe);
        assert_eq!(Plano::NinhoVermelho.tile_de_chao(0x75), 0x05);
        assert_eq!(Plano::Superficie.tile_de_chao(0x2e), 0x2e);
    }

    #[test]
    fn feromonio_so_acima_de_0x10() {
        assert_eq!(tile_de_feromonio(0x10), None);
        assert_eq!(tile_de_feromonio(0x11), Some(0xf1));
        assert_eq!(tile_de_feromonio(0xff), Some(0xff));
    }

    #[test]
    fn mapa_em_colunas() {
        let mut b = vec![0u8; 64 * 64];
        b[3 * 64 + 5] = 0x71;
        let m = MapaChao::new(Plano::NinhoPreto, &b).unwrap();
        assert_eq!(m.tile(3, 5), 1);
        assert_eq!(m.tile(5, 3), 0x90);
        assert!(MapaChao::new(Plano::Superficie, &b).is_err());
    }

    #[test]
    fn feromonio_cobre_2x2_celulas() {
        let a = vec![7u8; 128 * 64];
        let mut f = vec![0u8; 2048];
        f[32 + 1] = 0x35; // célula (1, 1) da camada = tiles x 2..3, y 2..3
        let m = MapaChao::new(Plano::Superficie, &a).unwrap().com_feromonio(&f).unwrap();
        assert_eq!(m.tile(2, 2), 0xf3);
        assert_eq!(m.tile(3, 3), 0xf3);
        assert_eq!(m.tile(1, 2), 7);
    }

    #[test]
    fn transparencia_preserva_o_fundo() {
        let mut q = Quadro::new(16, 16, 4);
        let mut t = [TRANSPARENTE; LADO * LADO];
        t[0] = 9;
        desenhar_tile_de_vida(&mut q, &t, 0, 0);
        assert_eq!(q.pixels[0], 9);
        assert_eq!(q.pixels[1], 4);
    }

    #[test]
    fn desenhar_posiciona_por_celula() {
        let mut ts = Tileset { tiles: vec![[0; LADO * LADO]; 256] };
        ts.tiles[0x91] = [5; LADO * LADO];
        let mut b = vec![0u8; 64 * 64]; // 0x00 -> tile 0x90
        b[64 + 1] = 0x01; // célula (1, 1) -> tile 0x91
        let m = MapaChao::new(Plano::NinhoPreto, &b).unwrap();
        let mut q = Quadro::new(32, 32, 0);
        m.desenhar(&mut q, &ts, (0, 0, 2, 2), (0, 0)).unwrap();
        assert_eq!(q.pixels[16 * 32 + 16], 5);
        assert_eq!(q.pixels[0], 0);
    }
}
