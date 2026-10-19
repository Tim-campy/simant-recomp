//! Menu da janela principal (objeto tipo 6 do banco), lido por `_InitMenu` (`1008:4e36`) para montar a
//! barra de menus com `CreateMenu`/`AppendMenu`. O `SIMANTW.EXE` não tem recurso MENU.
//! Especificação: docs/spec/formatos/menu.md.

/// Tipo do objeto no banco.
pub const TIPO_MENU: u8 = 6;

/// Texto de um item que vira separador (`MF_SEPARATOR`).
pub const SEPARADOR: &str = "---";

/// Barra de menus: título de cada menu (com `&`) e os itens dele (com `&` e `\t` antes do atalho).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuBanco {
    pub titulos: Vec<String>,
    pub itens: Vec<Vec<String>>,
}

/// Comando (`WM_COMMAND`) do item `item` do menu `menu`, como `_InitMenu` numera:
/// `0xFD01 + 16 × menu + item` (os separadores também ocupam um número).
pub fn comando(menu: usize, item: usize) -> u16 {
    0xFD01u16.wrapping_add((menu * 16 + item) as u16)
}

fn u32_le(d: &[u8], o: usize) -> Result<usize, String> {
    d.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize).ok_or_else(|| format!("menu: ponteiro fora do objeto em {o}"))
}

/// Lista de ponteiros (deslocamentos de 32 bits a partir do início do objeto) terminada em 0.
fn ponteiros(d: &[u8], mut o: usize) -> Result<Vec<usize>, String> {
    let mut v = Vec::new();
    loop {
        let p = u32_le(d, o)?;
        if p == 0 {
            return Ok(v);
        }
        v.push(p);
        o += 4;
    }
}

fn texto(d: &[u8], o: usize) -> Result<String, String> {
    let resto = d.get(o..).ok_or("menu: texto fora do objeto")?;
    let fim = resto.iter().position(|&b| b == 0).ok_or("menu: texto sem fim")?;
    Ok(resto[..fim].iter().map(|&b| b as char).collect())
}

impl MenuBanco {
    /// Objeto: no início, a lista de ponteiros das listas (a primeira com os títulos, as seguintes com
    /// os itens de cada menu), cada lista de ponteiros para textos ASCIIZ.
    pub fn decodificar(d: &[u8]) -> Result<MenuBanco, String> {
        let listas = ponteiros(d, 0)?;
        let (&primeira, resto) = listas.split_first().ok_or("menu vazio")?;
        let titulos = ponteiros(d, primeira)?.into_iter().map(|p| texto(d, p)).collect::<Result<Vec<_>, _>>()?;
        let mut itens = Vec::new();
        for &l in resto.iter().take(titulos.len()) {
            itens.push(ponteiros(d, l)?.into_iter().map(|p| texto(d, p)).collect::<Result<Vec<_>, _>>()?);
        }
        if itens.len() != titulos.len() {
            return Err(format!("menu: {} títulos e {} listas de itens", titulos.len(), itens.len()));
        }
        Ok(MenuBanco { titulos, itens })
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn objeto_sintetico() {
        // Listas: [títulos], [itens do menu 0]; títulos "&A"; itens "&X\tCtrl+X", "---".
        let mut d = vec![0u8; 12];
        d[0..4].copy_from_slice(&12u32.to_le_bytes());
        d[4..8].copy_from_slice(&20u32.to_le_bytes());
        // 12: lista de títulos (1 ponteiro + 0)
        d.extend_from_slice(&32u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        // 20: lista de itens (2 ponteiros + 0)
        d.extend_from_slice(&35u32.to_le_bytes());
        d.extend_from_slice(&45u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(b"&A\0&X\tCtrl+X\0---\0");
        let m = MenuBanco::decodificar(&d).unwrap();
        assert_eq!(m.titulos, vec!["&A"]);
        assert_eq!(m.itens, vec![vec!["&X\tCtrl+X".to_string(), SEPARADOR.to_string()]]);
    }

    #[test]
    fn numeracao_dos_comandos() {
        assert_eq!(comando(0, 0), 0xFD01);
        assert_eq!(comando(0, 7), 0xFD08);
        assert_eq!(comando(1, 6), 0xFD17);
        assert_eq!(comando(4, 5), 0xFD46);
    }
}
