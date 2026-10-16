//! Paleta de 16 cores, carregada de data/paleta.json (ver docs/spec/formatos/bitmap.md, seção Paleta).

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Paleta {
    /// 16 cores em "#rrggbb".
    pub cores: Vec<String>,
}

impl Paleta {
    pub fn rgb(&self) -> Result<Vec<[u8; 3]>, String> {
        if self.cores.len() != 16 {
            return Err(format!("paleta deve ter 16 cores, tem {}", self.cores.len()));
        }
        self.cores
            .iter()
            .map(|c| {
                let h = c.strip_prefix('#').filter(|h| h.len() == 6).ok_or(format!("cor inválida: {c}"))?;
                let v = u32::from_str_radix(h, 16).map_err(|_| format!("cor inválida: {c}"))?;
                Ok([(v >> 16) as u8, (v >> 8) as u8, v as u8])
            })
            .collect()
    }
}
