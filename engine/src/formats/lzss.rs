//! Descompressor LZSS dos registros do banco. Especificação: docs/spec/formatos/banco-dat-ndx.md.

const JANELA: usize = 4096;
/// Posição inicial de escrita na janela (4096 - 18).
const INICIO: usize = 0xFEE;
const COMPRIMENTO_MINIMO: usize = 3;

/// Descomprime `entrada` até produzir `tamanho` bytes.
/// Devolve erro se a entrada acabar antes disso.
pub fn descomprimir(entrada: &[u8], tamanho: usize) -> Result<Vec<u8>, String> {
    // O original preenche as posições 0..0xFED com espaço (_UnpackInit).
    let mut janela = [b' '; JANELA];
    let mut pos_janela = INICIO;
    let mut saida = Vec::with_capacity(tamanho);
    let mut i = 0;

    while saida.len() < tamanho {
        let controle = *entrada.get(i).ok_or("LZSS: entrada terminou antes do tamanho declarado")?;
        i += 1;
        for bit in 0..8 {
            if saida.len() >= tamanho {
                break;
            }
            if controle & (1 << bit) != 0 {
                let byte = *entrada.get(i).ok_or("LZSS: literal truncado")?;
                i += 1;
                saida.push(byte);
                janela[pos_janela] = byte;
                pos_janela = (pos_janela + 1) % JANELA;
            } else {
                let (a, b) = match (entrada.get(i), entrada.get(i + 1)) {
                    (Some(&a), Some(&b)) => (a as usize, b as usize),
                    _ => return Err("LZSS: referência truncada".into()),
                };
                i += 2;
                let origem = a | ((b & 0xF0) << 4);
                let comprimento = (b & 0x0F) + COMPRIMENTO_MINIMO;
                for k in 0..comprimento {
                    let byte = janela[(origem + k) % JANELA];
                    saida.push(byte);
                    janela[pos_janela] = byte;
                    pos_janela = (pos_janela + 1) % JANELA;
                }
            }
        }
    }
    saida.truncate(tamanho);
    Ok(saida)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn so_literais() {
        // controle 0xFF = 8 literais
        let entrada = [0xFF, b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h'];
        assert_eq!(descomprimir(&entrada, 8).unwrap(), b"abcdefgh");
    }

    #[test]
    fn referencia_repete_o_que_acabou_de_ser_escrito() {
        // 1 literal 'x' em 0xFEE, depois referência a 0xFEE com comprimento 3 (sobreposta).
        let entrada = [0b0000_0001, b'x', 0xEE, 0xF0];
        assert_eq!(descomprimir(&entrada, 4).unwrap(), b"xxxx");
    }

    #[test]
    fn janela_comeca_com_espacos() {
        // referência à posição 0 sem nada escrito antes: devolve espaços.
        let entrada = [0b0000_0000, 0x00, 0x00];
        assert_eq!(descomprimir(&entrada, 3).unwrap(), b"   ");
    }

    #[test]
    fn entrada_truncada_e_erro() {
        assert!(descomprimir(&[0xFF, b'a'], 4).is_err());
    }
}
