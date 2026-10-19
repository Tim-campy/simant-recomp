//! Paleta indexada, carregada de data/paleta.json: 16 cores do Windows e, depois, cores do sistema
//! (ver docs/spec/formatos/bitmap.md, seção Paleta).

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Paleta {
    /// Pelo menos 16 cores em "#rrggbb".
    pub cores: Vec<String>,
}

impl Paleta {
    pub fn rgb(&self) -> Result<Vec<[u8; 3]>, String> {
        if self.cores.len() < 16 || self.cores.len() > 256 {
            return Err(format!("paleta deve ter de 16 a 256 cores, tem {}", self.cores.len()));
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

/// A paleta de `data/paleta.json` com as cores do sistema do perfil de driver ativo
/// (`data/resolucao.json`, `perfil_driver`; docs/spec/telas/resolucao.md §7).
pub fn carregar_rgb(recursos: &crate::recursos::Recursos) -> Result<Vec<[u8; 3]>, String> {
    carregar_rgb_do_perfil(recursos, None)
}

/// Como [`carregar_rgb`], com o perfil `perfil` no lugar do ativo.
pub fn carregar_rgb_do_perfil(recursos: &crate::recursos::Recursos, perfil: Option<&str>) -> Result<Vec<[u8; 3]>, String> {
    use crate::core::resolucao::{ConfigResolucao, ARQUIVO};
    let mut rgb = recursos.dados::<Paleta>("paleta.json")?.rgb()?;
    if recursos.existe_dado(ARQUIVO) {
        recursos.dados::<ConfigResolucao>(ARQUIVO)?.perfil(perfil)?.aplicar_cores(&mut rgb)?;
    }
    Ok(rgb)
}
