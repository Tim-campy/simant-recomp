//! Listas de strings (objeto tipo 4 do banco). Especificação: docs/spec/formatos/banco-dat-ndx.md
//! ("Tipos de objeto") e docs/spec/telas/mensagens.md (`_LoadStringAnt`).
//!
//! Conteúdo: u16 big-endian com a contagem, seguido de strings Pascal (1 byte de tamanho + bytes).
//! `_LoadStringAnt` (`1020:03f4`) lê só o byte baixo da contagem (o segundo byte), então listas com
//! mais de 255 strings não existem no original.

/// Tipo de objeto das listas de strings.
pub const TIPO_STRINGS: u8 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListaStrings {
    /// Bytes de cada string (Latin-1 / Windows-1252, como no original), sem o byte de tamanho.
    pub strings: Vec<Vec<u8>>,
}

impl ListaStrings {
    pub fn decodificar(d: &[u8]) -> Result<ListaStrings, String> {
        if d.len() < 2 {
            return Err("string list without a count".into());
        }
        let n = d[1] as usize;
        let mut strings = Vec::with_capacity(n);
        let mut o = 2;
        for i in 0..n {
            let tam = *d.get(o).ok_or(format!("string {i} runs past the end of the list"))? as usize;
            let s = d.get(o + 1..o + 1 + tam).ok_or(format!("string {i} runs past the end of the list"))?;
            strings.push(s.to_vec());
            o += 1 + tam;
        }
        Ok(ListaStrings { strings })
    }

    /// String `i` como texto (Latin-1 → Unicode), para registro e testes.
    pub fn texto(&self, i: usize) -> Option<String> {
        self.strings.get(i).map(|s| s.iter().map(|&b| b as char).collect())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn decodifica_contagem_e_strings_pascal() {
        let d = [0, 2, 3, b'a', b'b', b'c', 0];
        let l = ListaStrings::decodificar(&d).unwrap();
        assert_eq!(l.strings, vec![b"abc".to_vec(), vec![]]);
        assert_eq!(l.texto(0).as_deref(), Some("abc"));
    }

    #[test]
    fn lista_truncada_e_erro() {
        assert!(ListaStrings::decodificar(&[0, 1, 5, b'a']).is_err());
        assert!(ListaStrings::decodificar(&[0]).is_err());
    }
}
