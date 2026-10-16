//! Tilesets do banco (objetos tipo 9). Especificação: docs/spec/formatos/tiles.md.
//!
//! Cada tile tem 16×16 pixels a 4 bpp (128 bytes), com as linhas guardadas de baixo para cima.
//! Os tiles de vida usam o índice 13 como transparente.

use super::banco::{Banco, Chave};

pub const TIPO_TILES: u8 = 9;
/// Lado do tile em pixels (`_tileWidth` = `_tileHeight` no modo `W`).
pub const LADO: usize = 16;
pub const BYTES_POR_TILE: usize = LADO * LADO / 2;
/// Índice de cor que os tiles de vida não desenham (`_XferLifeTileColor`).
pub const TRANSPARENTE: u8 = 13;
/// Quantos tiles cada objeto de vida ocupa no buffer concatenado (0x4000 bytes).
pub const TILES_POR_PARTE_DE_VIDA: usize = 128;

/// Ids (tipo 9) dos tilesets no `WINGANT`.
pub const ID_CHAO_QUINTAL: u16 = 10;
pub const ID_CHAO_CASA: u16 = 9;
pub const IDS_VIDA_SUPERFICIE: [u16; 3] = [15, 16, 17];
pub const IDS_VIDA_NINHO: [u16; 3] = [18, 19, 20];

/// Um tile decodificado: 256 índices de paleta, linhas de cima para baixo.
pub type Tile = [u8; LADO * LADO];

#[derive(Clone, Debug)]
pub struct Tileset {
    pub tiles: Vec<Tile>,
}

fn decodificar_tile(b: &[u8]) -> Tile {
    let mut t = [0u8; LADO * LADO];
    for linha in 0..LADO {
        // A primeira linha guardada é a de baixo.
        let origem = &b[linha * LADO / 2..(linha + 1) * LADO / 2];
        let y = LADO - 1 - linha;
        for (i, byte) in origem.iter().enumerate() {
            t[y * LADO + 2 * i] = byte >> 4;
            t[y * LADO + 2 * i + 1] = byte & 0x0F;
        }
    }
    t
}

impl Tileset {
    /// Decodifica um objeto de tiles: o tamanho deve ser múltiplo de 128 bytes.
    pub fn decodificar(d: &[u8]) -> Result<Tileset, String> {
        if d.is_empty() || !d.len().is_multiple_of(BYTES_POR_TILE) {
            return Err(format!("tileset com {} bytes, que não é múltiplo de {BYTES_POR_TILE}", d.len()));
        }
        Ok(Tileset { tiles: d.as_chunks::<BYTES_POR_TILE>().0.iter().map(|b| decodificar_tile(b)).collect() })
    }

    /// Monta um buffer de vida: cada parte começa num múltiplo de 128 tiles, como em `_LoadTiles`.
    /// Uma parte menor deixa o resto transparente; uma maior é erro.
    pub fn concatenar_vida(partes: &[Vec<u8>]) -> Result<Tileset, String> {
        let vazio = [TRANSPARENTE; LADO * LADO];
        let mut tiles = Vec::with_capacity(partes.len() * TILES_POR_PARTE_DE_VIDA);
        for (i, p) in partes.iter().enumerate() {
            let mut t = Tileset::decodificar(p).map_err(|e| format!("parte {i}: {e}"))?.tiles;
            if t.len() > TILES_POR_PARTE_DE_VIDA {
                return Err(format!("parte {i}: {} tiles, máximo {TILES_POR_PARTE_DE_VIDA}", t.len()));
            }
            t.resize(TILES_POR_PARTE_DE_VIDA, vazio);
            tiles.extend(t);
        }
        Ok(Tileset { tiles })
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    pub fn tile(&self, i: usize) -> Option<&Tile> {
        self.tiles.get(i)
    }
}

fn objeto(banco: &Banco, id: u16) -> Result<Vec<u8>, String> {
    banco
        .objeto(Chave { tipo: TIPO_TILES, id })
        .ok_or_else(|| format!("{}: falta o objeto {TIPO_TILES}/{id}", banco.nome))?
}

/// Tileset de chão: `ID_CHAO_QUINTAL` ou `ID_CHAO_CASA`.
pub fn carregar_chao(banco: &Banco, id: u16) -> Result<Tileset, String> {
    Tileset::decodificar(&objeto(banco, id)?)
}

/// Tileset de vida (`IDS_VIDA_SUPERFICIE` ou `IDS_VIDA_NINHO`), já concatenado.
pub fn carregar_vida(banco: &Banco, ids: &[u16]) -> Result<Tileset, String> {
    let partes = ids.iter().map(|&id| objeto(banco, id)).collect::<Result<Vec<_>, _>>()?;
    Tileset::concatenar_vida(&partes)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn linhas_de_baixo_para_cima_e_nibble_alto_primeiro() {
        let mut d = vec![0u8; BYTES_POR_TILE];
        d[0] = 0x12; // primeira linha guardada = linha de baixo
        d[BYTES_POR_TILE - 1] = 0x34; // última linha guardada = linha de cima, últimos 2 pixels
        let t = Tileset::decodificar(&d).unwrap();
        let tile = t.tile(0).unwrap();
        assert_eq!(&tile[15 * LADO..15 * LADO + 2], &[1, 2]);
        assert_eq!(&tile[14..16], &[3, 4]);
    }

    #[test]
    fn tamanho_invalido_e_erro() {
        assert!(Tileset::decodificar(&[0; 100]).is_err());
        assert!(Tileset::decodificar(&[]).is_err());
    }

    #[test]
    fn partes_de_vida_comecam_em_multiplos_de_128() {
        let t = Tileset::concatenar_vida(&[vec![0; BYTES_POR_TILE], vec![0x11; BYTES_POR_TILE]]).unwrap();
        assert_eq!(t.len(), 256);
        assert_eq!(t.tile(1).unwrap()[0], TRANSPARENTE);
        assert_eq!(t.tile(128).unwrap()[0], 1);
    }
}
