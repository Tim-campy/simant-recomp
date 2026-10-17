//! Efeitos sonoros (tipo 5, "instrumento DAC"), cabeçalhos de música (tipo 18) e gravação de WAV.
//! Especificação: docs/spec/formatos/som.md.

/// Tipo do banco com os efeitos sonoros do Windows.
pub const TIPO_EFEITO: u8 = 5;
/// Tipo do banco com os cabeçalhos de música.
pub const TIPO_CABECALHO_MUSICA: u8 = 18;
/// Tipo do banco com as músicas MIDI do modo Sound Blaster.
pub const TIPO_MIDI_SOUNDBLASTER: u8 = 20;

/// Tamanho da tabela de deltas no começo de um efeito.
const TAM_TABELA: usize = 16;

/// Decodifica um efeito tipo 5 em PCM de 8 bits sem sinal (0x80 = silêncio), mono.
/// Cada byte depois da tabela gera duas amostras: nibble alto, depois baixo; a soma estoura em 8 bits.
pub fn decodificar_efeito(dados: &[u8]) -> Result<Vec<u8>, String> {
    if dados.len() < TAM_TABELA {
        return Err(format!("efeito com {} bytes, menor que a tabela de deltas", dados.len()));
    }
    let (tabela, corpo) = dados.split_at(TAM_TABELA);
    let mut amostra: u8 = 0x80;
    let mut saida = Vec::with_capacity(corpo.len() * 2);
    for &b in corpo {
        for nibble in [b >> 4, b & 0x0F] {
            amostra = amostra.wrapping_add(tabela[nibble as usize]);
            saida.push(amostra);
        }
    }
    Ok(saida)
}

/// Cabeçalho de música (tipo 18): 9 palavras big-endian.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CabecalhoMusica {
    /// Id do objeto MIDI (tipos 20 e 50).
    pub midi: u16,
    /// Transposição em semitons (só o modo Sound Blaster aplica).
    pub transposicao: i16,
    /// As 9 palavras, para os campos ainda sem significado conhecido.
    pub palavras: [i16; 9],
}

impl CabecalhoMusica {
    pub fn de_bytes(b: &[u8]) -> Result<CabecalhoMusica, String> {
        if b.len() < 18 {
            return Err(format!("cabeçalho de música com {} bytes (esperado 18)", b.len()));
        }
        let mut palavras = [0i16; 9];
        for (i, p) in palavras.iter_mut().enumerate() {
            *p = i16::from_be_bytes([b[2 * i], b[2 * i + 1]]);
        }
        Ok(CabecalhoMusica { midi: palavras[0] as u16, transposicao: palavras[3], palavras })
    }
}

/// Arquivo WAV (RIFF) com PCM inteiro: 8 bits sem sinal ou 16 bits com sinal, intercalado.
pub fn wav(amostras: &[u8], taxa: u32, canais: u16, bits: u16) -> Vec<u8> {
    let bloco = canais as u32 * bits as u32 / 8;
    let mut w = Vec::with_capacity(44 + amostras.len() + 1);
    w.extend_from_slice(b"RIFF");
    let tam_dados = amostras.len() as u32;
    let preenchimento = tam_dados & 1;
    w.extend_from_slice(&(36 + tam_dados + preenchimento).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&canais.to_le_bytes());
    w.extend_from_slice(&taxa.to_le_bytes());
    w.extend_from_slice(&(taxa * bloco).to_le_bytes());
    w.extend_from_slice(&(bloco as u16).to_le_bytes());
    w.extend_from_slice(&bits.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&tam_dados.to_le_bytes());
    w.extend_from_slice(amostras);
    if preenchimento == 1 {
        w.push(0);
    }
    w
}

/// WAV de 16 bits estéreo a partir de amostras em ponto flutuante (−1..1), intercaladas E, D.
pub fn wav_estereo_f32(amostras: &[f32], taxa: u32) -> Vec<u8> {
    let bytes: Vec<u8> = amostras
        .iter()
        .flat_map(|&x| ((x.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes())
        .collect();
    wav(&bytes, taxa, 2, 16)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn decodifica_nibble_alto_primeiro_com_estouro_em_8_bits() {
        let mut d = vec![0u8; 16];
        d[1] = 1;
        d[2] = 0x7F;
        d[15] = 0xFF; // −1
        d.extend_from_slice(&[0x12, 0x2F, 0x22]);
        let pcm = decodificar_efeito(&d).unwrap();
        // 0x80 +1 → 0x81, +0x7F → 0x00 (estoura), +0x7F → 0x7F, −1 → 0x7E, +0x7F +0x7F → 0xFD, 0x7C.
        assert_eq!(pcm, vec![0x81, 0x00, 0x7F, 0x7E, 0xFD, 0x7C]);
    }

    #[test]
    fn efeito_menor_que_a_tabela_e_erro() {
        assert!(decodificar_efeito(&[0; 15]).is_err());
        assert_eq!(decodificar_efeito(&[0; 16]).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn cabecalho_le_midi_e_transposicao_em_big_endian() {
        let b = [0x2B, 0x05, 0, 1, 0, 0, 0, 12, 0, 6, 0, 4, 0x40, 0x10, 0, 0, 0, 0];
        let c = CabecalhoMusica::de_bytes(&b).unwrap();
        assert_eq!(c.midi, 11013);
        assert_eq!(c.transposicao, 12);
        assert_eq!(c.palavras[6], 0x4010);
    }

    #[test]
    fn wav_tem_cabecalho_de_44_bytes_e_preenche_tamanho_impar() {
        let w = wav(&[1, 2, 3], 4096, 1, 8);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(w[4..8].try_into().unwrap()), 36 + 4);
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), 4096);
        assert_eq!(u32::from_le_bytes(w[40..44].try_into().unwrap()), 3);
        assert_eq!(w.len(), 48);
    }
}
