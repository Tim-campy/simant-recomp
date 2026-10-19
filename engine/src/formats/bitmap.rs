//! Bitmaps do banco (objeto tipo 2). Especificação: docs/spec/formatos/bitmap.md.

pub const TIPO_BITMAP: u8 = 2;
const CABECALHO: usize = 12;
const FORMATO_MONO: u8 = 0x01;
/// 4 planos de 1 bit por linha (sem o bit 0x80): o menu de ícones (2/7200) [medido].
const FORMATO_PLANAR: u8 = 0x04;
/// `kind` 0xFFFF: imagem empacotada (LZSS dentro do objeto), aberta por `_GUnpackPic` e
/// `_DrawBitMapToBuffer` (bitmap.md, "Campo kind").
const KIND_EMPACOTADO: u16 = 0xFFFF;

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
        if u16::from_le_bytes([d[0], d[1]]) == KIND_EMPACOTADO {
            // u16 em +2 = tamanho aberto (cabeçalho de 12 bytes + pixels); LZSS a partir de +4. O
            // original abre o cabeçalho e só `tamanho − 16` bytes de pixels (`_GUnpackPic`,
            // `1008:40d4`): os 4 últimos ficam com o que havia na memória; o port os deixa 0.
            let tamanho = u16::from_le_bytes([d[2], d[3]]) as usize;
            let mut aberto = crate::formats::lzss::descomprimir(&d[4..], tamanho.saturating_sub(4))?;
            aberto.resize(tamanho.max(CABECALHO), 0);
            if aberto.len() >= 2 && u16::from_le_bytes([aberto[0], aberto[1]]) == KIND_EMPACOTADO {
                return Err("bitmap empacotado dentro de outro".into());
            }
            return Bitmap::decodificar(&aberto);
        }
        let formato = d[2];
        let largura = u16::from_le_bytes([d[8], d[9]]) as usize;
        let altura = u16::from_le_bytes([d[10], d[11]]) as usize;
        // Sem o bit 0x80 e fora do planar, 1 bpp: 0x01 nos bancos e 0x00 nos empacotados do MWINNT
        // (2/2000, 2/2500), que o modo monocromático desenha com `_CopyMonoBitmap`.
        let mono = formato == FORMATO_MONO || formato == 0;
        let bpp = if mono { 1 } else { 4 };
        let planar = formato == FORMATO_PLANAR;
        // Planar: cada plano tem ⌈w/8⌉ bytes por linha, sem alinhar (`_ConvertBitmap`, `1008:246e`).
        let mut stride = if planar { 4 * largura.div_ceil(8) } else { bytes_por_linha(largura, bpp) };
        let dados = &d[CABECALHO..];
        // Algumas figuras mono dos cartões do Info (ex.: SHARED 2/150, 102 × 116 com 1508 bytes) têm
        // linhas alinhadas a byte, não a 32 bits [medido nos dados].
        if mono && dados.len() < stride * altura && dados.len() >= largura.div_ceil(8) * altura {
            stride = largura.div_ceil(8);
        }
        if dados.len() < stride * altura {
            return Err(format!(
                "bitmap {largura}x{altura}: esperava {} bytes de pixels, há {}",
                stride * altura,
                dados.len()
            ));
        }
        let mut pixels = vec![0u8; largura * altura];
        if planar {
            // Cada linha (de cima para baixo) tem os 4 planos, um após o outro, cada um com a largura
            // de uma linha de 1 bpp; o plano k é o bit k do índice [medido contra a captura j07].
            let plano = largura.div_ceil(8);
            for y in 0..altura {
                let linha = &dados[y * stride..(y + 1) * stride];
                for x in 0..largura {
                    let mut v = 0u8;
                    for k in 0..4 {
                        if linha[k * plano + x / 8] & (0x80 >> (x % 8)) != 0 {
                            v |= 1 << k;
                        }
                    }
                    pixels[y * largura + x] = v;
                }
            }
            return Ok(Bitmap { largura, altura, pixels });
        }
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
    fn planar_quatro_planos_por_linha() {
        // 2x1, formato 0x04: planos de ⌈2/8⌉ = 1 byte, sem alinhar; pixel 0 = 0b0101, 1 = 0b1010.
        let mut d = cabecalho(0x04, 2, 1);
        d.extend_from_slice(&[0x80, 0x40, 0x80, 0x40]);
        let b = Bitmap::decodificar(&d).unwrap();
        assert_eq!(b.pixels, vec![5, 10]);
        // 19x1 (como o 2/8905 do MWINNT): 3 bytes por plano, 12 por linha.
        let mut d = cabecalho(0x04, 19, 1);
        d.extend_from_slice(&[0, 0, 0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0x20]);
        let b = Bitmap::decodificar(&d).unwrap();
        assert_eq!((b.pixels[18], b.pixels[0]), (1 | 8, 0));
    }

    #[test]
    fn empacotado_abre_o_lzss_de_dentro() {
        // Cabeçalho 4 bpp de 2x1 + uma linha de 4 bytes, tudo literal no LZSS (controle 0xFF).
        let mut aberto = cabecalho(0x84, 2, 1);
        aberto.extend_from_slice(&[0x12, 0, 0, 0]);
        let mut d = vec![0xFF, 0xFF];
        d.extend_from_slice(&((aberto.len() + 4) as u16).to_le_bytes());
        for bloco in aberto.chunks(8) {
            d.push(0xFF);
            d.extend_from_slice(bloco);
        }
        let b = Bitmap::decodificar(&d).unwrap();
        assert_eq!((b.largura, b.altura, b.pixels.clone()), (2, 1, vec![1, 2]));
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
