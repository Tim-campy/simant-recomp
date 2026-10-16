//! Bitmaps do banco (objeto tipo 2). Especificação: docs/spec/formatos/bitmap.md.

pub const TIPO_BITMAP: u8 = 2;
const CABECALHO: usize = 12;
const FORMATO_MONO: u8 = 0x01;

/// Imagem indexada: um índice de paleta por pixel, linhas de cima para baixo.
#[derive(Clone, Debug)]
pub struct Bitmap {
    pub largura: usize,
    pub altura: usize,
    pub pixels: Vec<u8>,
}

fn bytes_por_linha(largura: usize, bpp: usize) -> usize {
    (largura * bpp).div_ceil(32) * 4
}

impl Bitmap {
    pub fn decodificar(d: &[u8]) -> Result<Bitmap, String> {
        if d.len() < CABECALHO {
            return Err("bitmap menor que o cabeçalho".into());
        }
        let formato = d[2];
        let largura = u16::from_le_bytes([d[8], d[9]]) as usize;
        let altura = u16::from_le_bytes([d[10], d[11]]) as usize;
        let mono = formato == FORMATO_MONO;
        let bpp = if mono { 1 } else { 4 };
        let stride = bytes_por_linha(largura, bpp);
        let dados = &d[CABECALHO..];
        if dados.len() < stride * altura {
            return Err(format!(
                "bitmap {largura}x{altura}: esperava {} bytes de pixels, há {}",
                stride * altura,
                dados.len()
            ));
        }
        let mut pixels = vec![0u8; largura * altura];
        for y in 0..altura {
            // 4 bpp está de baixo para cima; 1 bpp, de cima para baixo.
            let linha_origem = if mono { y } else { altura - 1 - y };
            let linha = &dados[linha_origem * stride..(linha_origem + 1) * stride];
            for x in 0..largura {
                pixels[y * largura + x] = if mono {
                    // 1 = branco (índice 15), 0 = preto (índice 0) [hipótese de cores do modo mono]
                    if linha[x / 8] & (0x80 >> (x % 8)) != 0 { 15 } else { 0 }
                } else {
                    let b = linha[x / 2];
                    if x % 2 == 0 { b >> 4 } else { b & 0x0F }
                };
            }
        }
        Ok(Bitmap { largura, altura, pixels })
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cabecalho(formato: u8, w: u16, h: u16) -> Vec<u8> {
        let mut v = vec![0, 0, formato, 0, 0, 0, 0, 0];
        v.extend_from_slice(&w.to_le_bytes());
        v.extend_from_slice(&h.to_le_bytes());
        v
    }

    #[test]
    fn quatro_bpp_de_baixo_para_cima_com_alinhamento() {
        // 3x2: linha de 2 bytes úteis, alinhada a 4.
        let mut d = cabecalho(0x84, 3, 2);
        d.extend_from_slice(&[0x12, 0x30, 0, 0]); // linha de baixo: 1 2 3
        d.extend_from_slice(&[0x45, 0x60, 0, 0]); // linha de cima: 4 5 6
        let b = Bitmap::decodificar(&d).unwrap();
        assert_eq!(b.pixels, vec![4, 5, 6, 1, 2, 3]);
    }

    #[test]
    fn mono_de_cima_para_baixo() {
        let mut d = cabecalho(0x01, 9, 1);
        d.extend_from_slice(&[0b1000_0001, 0b1000_0000, 0, 0]);
        let b = Bitmap::decodificar(&d).unwrap();
        assert_eq!(b.pixels, vec![15, 0, 0, 0, 0, 0, 0, 15, 15]);
    }

    #[test]
    fn dados_curtos_sao_erro() {
        let d = cabecalho(0x84, 8, 8);
        assert!(Bitmap::decodificar(&d).is_err());
    }
}
