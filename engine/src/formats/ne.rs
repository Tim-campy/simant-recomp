//! Recursos de um executável NE (Windows 3.x): por enquanto só os cursores (`GROUP_CURSOR` +
//! `CURSOR`), lidos do `SIMANTW.EXE` do usuário. Especificação: docs/spec/formatos/ne-recursos.md.

use std::collections::HashMap;

const RT_CURSOR: u16 = 1;
const RT_GROUP_CURSOR: u16 = 12;

/// Cursor monocromático: `e` = máscara AND (1 = transparente), `x` = máscara XOR (1 = branco ou
/// inverte), linha a linha de cima para baixo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub largura: usize,
    pub altura: usize,
    pub quente: (u16, u16),
    pub e: Vec<bool>,
    pub x: Vec<bool>,
}

fn u16_le(d: &[u8], o: usize) -> Result<u16, String> {
    d.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or_else(|| "NE truncado".into())
}

fn u32_le(d: &[u8], o: usize) -> Result<u32, String> {
    d.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or_else(|| "NE truncado".into())
}

/// Um recurso: tipo, id (número ou nome) e os bytes.
struct Recurso<'a> {
    tipo: u16,
    numero: Option<u16>,
    nome: Option<String>,
    dados: &'a [u8],
}

fn recursos(exe: &[u8]) -> Result<Vec<Recurso<'_>>, String> {
    if exe.get(0..2) != Some(b"MZ") {
        return Err("não é um executável MZ".into());
    }
    let ne = u32_le(exe, 0x3C)? as usize;
    if exe.get(ne..ne + 2) != Some(b"NE") {
        return Err("não é um executável NE".into());
    }
    let tabela = ne + u16_le(exe, ne + 0x24)? as usize;
    let deslocamento = u16_le(exe, tabela)?;
    let mut o = tabela + 2;
    let mut saida = Vec::new();
    loop {
        let tipo = u16_le(exe, o)?;
        if tipo == 0 {
            break;
        }
        let n = u16_le(exe, o + 2)? as usize;
        o += 8;
        for _ in 0..n {
            let off = (u16_le(exe, o)? as usize) << deslocamento;
            let tam = (u16_le(exe, o + 2)? as usize) << deslocamento;
            let id = u16_le(exe, o + 6)?;
            let (numero, nome) = if id & 0x8000 != 0 {
                (Some(id & 0x7FFF), None)
            } else {
                let p = tabela + id as usize;
                let len = *exe.get(p).ok_or("NE: nome fora")? as usize;
                (None, Some(String::from_utf8_lossy(exe.get(p + 1..p + 1 + len).ok_or("NE: nome fora")?).into_owned()))
            };
            let dados = exe.get(off..off + tam).ok_or("NE: recurso fora do arquivo")?;
            saida.push(Recurso { tipo: tipo & 0x7FFF, numero, nome, dados });
            o += 12;
        }
    }
    Ok(saida)
}

/// Bits de 1 bpp de uma DIB de baixo para cima, em linhas de 4 bytes, de cima para baixo.
fn mascara(d: &[u8], largura: usize, altura: usize) -> Vec<bool> {
    let linha = largura.div_ceil(32) * 4;
    let mut v = vec![false; largura * altura];
    for y in 0..altura {
        let base = (altura - 1 - y) * linha;
        for x in 0..largura {
            v[y * largura + x] = d.get(base + x / 8).is_some_and(|b| b & (0x80 >> (x % 8)) != 0);
        }
    }
    v
}

fn decodificar_cursor(d: &[u8]) -> Result<Cursor, String> {
    let quente = (u16_le(d, 0)?, u16_le(d, 2)?);
    let bmp = d.get(4..).ok_or("cursor truncado")?;
    let tam_cab = u32_le(bmp, 0)? as usize;
    let largura = u32_le(bmp, 4)? as usize;
    let altura = u32_le(bmp, 8)? as usize / 2;
    let bits = u16_le(bmp, 14)?;
    if bits != 1 {
        return Err(format!("cursor com {bits} bits por pixel"));
    }
    let cores = match u32_le(bmp, 32)? {
        0 => 2,
        n => n as usize,
    };
    let inicio = tam_cab + cores * 4;
    let tam_mascara = largura.div_ceil(32) * 4 * altura;
    let xor = bmp.get(inicio..inicio + tam_mascara).ok_or("cursor truncado")?;
    let and = bmp.get(inicio + tam_mascara..inicio + 2 * tam_mascara).ok_or("cursor truncado")?;
    Ok(Cursor { largura, altura, quente, e: mascara(and, largura, altura), x: mascara(xor, largura, altura) })
}

/// Cursores do executável pelo nome do grupo (`HELPCURSOR`, `MAGCURSOR`…): a primeira imagem do grupo.
pub fn cursores(exe: &[u8]) -> Result<HashMap<String, Cursor>, String> {
    let rs = recursos(exe)?;
    let mut saida = HashMap::new();
    for g in rs.iter().filter(|r| r.tipo == RT_GROUP_CURSOR) {
        let Some(nome) = g.nome.clone().or_else(|| g.numero.map(|n| format!("#{n}"))) else { continue };
        // CURSORDIR: reservado, tipo (2), número; entradas de 14 bytes com o id do CURSOR no fim.
        if u16_le(g.dados, 4)? == 0 {
            continue;
        }
        let id = u16_le(g.dados, 6 + 12)?;
        if let Some(c) = rs.iter().find(|r| r.tipo == RT_CURSOR && r.numero == Some(id)) {
            saida.insert(nome, decodificar_cursor(c.dados)?);
        }
    }
    Ok(saida)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cursores_do_simantw() {
        let dir = std::env::var_os("SIMANT_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../SimAnt-T/MAXIS/ANTWIN"));
        let Ok(exe) = std::fs::read(dir.join("SIMANTW.EXE")) else {
            eprintln!("AVISO: SIMANTW.EXE não encontrado; teste ignorado");
            return;
        };
        let c = cursores(&exe).unwrap();
        for n in ["HELPCURSOR", "MAGCURSOR", "ROCKCURSOR", "DIGCURSOR", "ANTCURSOR", "FOODCURSOR", "DROPCURSOR", "SPRAYCURSOR"] {
            let k = c.get(n).unwrap_or_else(|| panic!("{n}"));
            assert_eq!((k.largura, k.altura), (32, 32), "{n}");
            assert!(k.e.iter().any(|&b| !b), "{n} tem pixels visíveis");
        }
        // A ponta da seta do HELPCURSOR é o ponto quente (bancada: a1/r01 mostram a seta com o "?").
        assert_eq!(c["HELPCURSOR"].quente, (0, 0));
    }
}
