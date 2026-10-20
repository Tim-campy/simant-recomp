//! Banco de recursos `.NDX`/`.DAT`. Especificação: docs/spec/formatos/banco-dat-ndx.md.

use std::collections::HashMap;
use std::path::Path;

use super::lzss;

const ASSINATURA: u32 = 0x1234_5678;
const CABECALHO_DAT: usize = 14;
const CABECALHO_REGISTRO: usize = 10;
const CABECALHO_NDX: usize = 0x14;
const FLAG_COMPRIMIDO: u8 = 0x01;

/// Identifica um objeto do banco: (tipo, id).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Chave {
    pub tipo: u8,
    pub id: u16,
}

#[derive(Clone, Debug)]
pub struct Entrada {
    pub chave: Chave,
    pub flags: u8,
    /// Posição do conteúdo (já depois do cabeçalho do registro) no `.DAT`.
    inicio: usize,
    tamanho: usize,
}

/// Um par NDX/DAT carregado em memória.
pub struct Banco {
    pub nome: String,
    dat: Vec<u8>,
    entradas: Vec<Entrada>,
    por_chave: HashMap<Chave, usize>,
}

fn u16_le(b: &[u8], o: usize) -> Result<u16, String> {
    b.get(o..o + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| format!("read outside the file at 0x{o:x}"))
}

fn u32_le(b: &[u8], o: usize) -> Result<u32, String> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| format!("read outside the file at 0x{o:x}"))
}

impl Banco {
    /// Abre `base.NDX` e `base.DAT` (base sem extensão). Aceita extensão em maiúsculas ou minúsculas.
    pub fn abrir(base: &Path) -> Result<Banco, String> {
        let ler = |ext: &str| -> Result<Vec<u8>, String> {
            for e in [ext.to_uppercase(), ext.to_lowercase()] {
                let caminho = base.with_extension(&e);
                if let Ok(d) = std::fs::read(&caminho) {
                    return Ok(d);
                }
            }
            Err(format!("could not find {}.{}", base.display(), ext))
        };
        let nome = base.file_name().map(|n| n.to_string_lossy().to_uppercase()).unwrap_or_default();
        Banco::de_bytes(nome, &ler("ndx")?, ler("dat")?)
    }

    pub fn de_bytes(nome: String, ndx: &[u8], dat: Vec<u8>) -> Result<Banco, String> {
        if u32_le(&dat, 0)? != ASSINATURA {
            return Err(format!("{nome}.DAT: invalid signature"));
        }
        let n = u32_le(ndx, 0)? as usize;
        let mut entradas = Vec::with_capacity(n);
        let mut por_chave = HashMap::with_capacity(n);
        for i in 0..n {
            let o = CABECALHO_NDX + i * 8;
            let pos = CABECALHO_DAT + u32_le(ndx, o)? as usize;
            let id = u16_le(ndx, o + 4)?;
            let tipo = *ndx.get(o + 6).ok_or("NDX truncated")?;
            let flags = *ndx.get(o + 7).ok_or("NDX truncated")?;
            if u32_le(&dat, pos)? != ASSINATURA {
                return Err(format!("{nome}: record without signature at 0x{pos:x}"));
            }
            let tamanho = u32_le(&dat, pos + 6)? as usize;
            let inicio = pos + CABECALHO_REGISTRO;
            if inicio + tamanho > dat.len() {
                return Err(format!("{nome}: record at 0x{pos:x} runs past the end of the file"));
            }
            let chave = Chave { tipo, id };
            por_chave.insert(chave, entradas.len());
            entradas.push(Entrada { chave, flags, inicio, tamanho });
        }
        Ok(Banco { nome, dat, entradas, por_chave })
    }

    pub fn entradas(&self) -> &[Entrada] {
        &self.entradas
    }

    pub fn contem(&self, chave: Chave) -> bool {
        self.por_chave.contains_key(&chave)
    }

    /// Conteúdo do objeto, já descomprimido.
    pub fn objeto(&self, chave: Chave) -> Option<Result<Vec<u8>, String>> {
        let e = &self.entradas[*self.por_chave.get(&chave)?];
        Some(self.ler(e))
    }

    fn ler(&self, e: &Entrada) -> Result<Vec<u8>, String> {
        let bruto = &self.dat[e.inicio..e.inicio + e.tamanho];
        if e.flags & FLAG_COMPRIMIDO == 0 {
            return Ok(bruto.to_vec());
        }
        let tamanho = u16_le(bruto, 0)? as usize;
        lzss::descomprimir(&bruto[2..], tamanho)
            .map_err(|err| format!("{} {:?}: {err}", self.nome, e.chave))
    }
}
