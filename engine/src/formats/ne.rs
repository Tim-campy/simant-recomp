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
    d.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or_else(|| "NE truncated".into())
}

fn u32_le(d: &[u8], o: usize) -> Result<u32, String> {
    d.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or_else(|| "NE truncated".into())
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
        return Err("not an MZ executable".into());
    }
    let ne = u32_le(exe, 0x3C)? as usize;
    if exe.get(ne..ne + 2) != Some(b"NE") {
        return Err("not an NE executable".into());
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
                let len = *exe.get(p).ok_or("NE: name outside the file")? as usize;
                (None, Some(String::from_utf8_lossy(exe.get(p + 1..p + 1 + len).ok_or("NE: name outside the file")?).into_owned()))
            };
            let dados = exe.get(off..off + tam).ok_or("NE: resource outside the file")?;
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
    let bmp = d.get(4..).ok_or("cursor truncated")?;
    let tam_cab = u32_le(bmp, 0)? as usize;
    let largura = u32_le(bmp, 4)? as usize;
    let altura = u32_le(bmp, 8)? as usize / 2;
    let bits = u16_le(bmp, 14)?;
    if bits != 1 {
        return Err(format!("cursor with {bits} bits per pixel"));
    }
    let cores = match u32_le(bmp, 32)? {
        0 => 2,
        n => n as usize,
    };
    let inicio = tam_cab + cores * 4;
    let tam_mascara = largura.div_ceil(32) * 4 * altura;
    let xor = bmp.get(inicio..inicio + tam_mascara).ok_or("cursor truncated")?;
    let and = bmp.get(inicio + tam_mascara..inicio + 2 * tam_mascara).ok_or("cursor truncated")?;
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

const RT_ACCELERATOR: u16 = 9;

/// Entrada de um recurso ACCELERATORS (5 bytes no NE): `flags` (1 = tecla virtual, 4 = Shift, 8 = Ctrl,
/// 0x10 = Alt), a tecla (código virtual ou caractere) e o comando.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Acelerador {
    pub flags: u8,
    pub tecla: u16,
    pub comando: u16,
}

/// Tabelas ACCELERATORS do executável pelo nome (ou `#número`).
pub fn aceleradores(exe: &[u8]) -> Result<HashMap<String, Vec<Acelerador>>, String> {
    let mut saida = HashMap::new();
    for r in recursos(exe)?.iter().filter(|r| r.tipo == RT_ACCELERATOR) {
        let Some(nome) = r.nome.clone().or_else(|| r.numero.map(|n| format!("#{n}"))) else { continue };
        let mut v = Vec::new();
        for c in r.dados.as_chunks::<5>().0 {
            v.push(Acelerador { flags: c[0] & 0x7F, tecla: u16::from_le_bytes([c[1], c[2]]), comando: u16::from_le_bytes([c[3], c[4]]) });
            if c[0] & 0x80 != 0 {
                break;
            }
        }
        saida.insert(nome, v);
    }
    Ok(saida)
}

/// Bytes de um segmento do executável como estão no arquivo (`n` a partir de 1, a numeração lógica
/// do `SIMANTW.SYM` e dos endereços `n:offset` da documentação: 10 = DGROUP, seletor 1048). Só a
/// parte inicializada; a tabela de relocação que vem depois não entra. Formato: tabela de segmentos
/// do cabeçalho NE (`ne + 0x22`, 8 bytes por segmento: setor, tamanho no arquivo (0 = 64 KB),
/// flags, alocação mínima; posição = setor << `ne + 0x32`). docs/spec/formatos/ne-recursos.md.
pub fn segmento(exe: &[u8], n: u16) -> Result<&[u8], String> {
    if exe.get(0..2) != Some(b"MZ") {
        return Err("not an MZ executable".into());
    }
    let ne = u32_le(exe, 0x3C)? as usize;
    if exe.get(ne..ne + 2) != Some(b"NE") {
        return Err("not an NE executable".into());
    }
    let quantos = u16_le(exe, ne + 0x1C)?;
    if n == 0 || n > quantos {
        return Err(format!("segment {n} does not exist (the executable has {quantos})"));
    }
    let tabela = ne + u16_le(exe, ne + 0x22)? as usize;
    let deslocamento = u16_le(exe, ne + 0x32)?;
    let e = tabela + (n as usize - 1) * 8;
    let setor = u16_le(exe, e)? as usize;
    if setor == 0 {
        return Err(format!("segment {n} has no data in the file"));
    }
    let tamanho = match u16_le(exe, e + 2)? {
        0 => 0x10000,
        t => t as usize,
    };
    let inicio = setor << deslocamento;
    exe.get(inicio..inicio + tamanho).ok_or_else(|| format!("segment {n} runs past the end of the file"))
}

#[cfg(test)]
/// Executável NE mínimo com um segmento de dados (n = 1) de `dados`, alinhamento 16.
pub(crate) fn exe_de_teste(dados: &[u8]) -> Vec<u8> {
    let mut v = vec![0u8; 0x90];
    v[0..2].copy_from_slice(b"MZ");
    v[0x3C] = 0x40;
    v[0x40..0x42].copy_from_slice(b"NE");
    v[0x40 + 0x1C] = 1; // um segmento
    v[0x40 + 0x22] = 0x40; // tabela de segmentos em ne + 0x40 = 0x80
    v[0x40 + 0x32] = 4; // setores de 16 bytes
    v[0x80..0x82].copy_from_slice(&(0x90u16 >> 4).to_le_bytes());
    v[0x82..0x84].copy_from_slice(&(dados.len() as u16).to_le_bytes());
    v[0x84] = 1; // DATA
    v.extend_from_slice(dados);
    v
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn segmento_pela_tabela() {
        let exe = exe_de_teste(b"\0abc\0de");
        assert_eq!(segmento(&exe, 1).unwrap(), b"\0abc\0de");
        assert!(segmento(&exe, 2).is_err());
        assert!(segmento(&exe, 0).is_err());
        assert!(segmento(b"MZ", 1).is_err());
    }

    /// O executável não tem recurso MENU (a barra vem do objeto 6/0 do banco, formats/menu.rs); a tabela
    /// ACCELERATORTABLE tem os atalhos que os itens mostram (docs/spec/telas/menus.md).
    #[test]
    fn aceleradores_do_simantw() {
        let dir = std::env::var_os("SIMANT_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../SimAnt-T/MAXIS/ANTWIN"));
        let Ok(exe) = std::fs::read(dir.join("SIMANTW.EXE")) else {
            eprintln!("AVISO: SIMANTW.EXE não encontrado; teste ignorado");
            return;
        };
        assert!(recursos(&exe).unwrap().iter().all(|r| r.tipo != 4), "sem recurso MENU");
        let t = &aceleradores(&exe).unwrap()["ACCELERATORTABLE"];
        assert_eq!(t.len(), 47);
        // Ctrl+N → 0xFD03 (File → New), Ctrl+Q → 0xFD08 (Quit), Ctrl+T → 0xFD17 (Window → Status),
        // Shift+0 → 0xFD41 (Pause), Ctrl+F4 → 0xFDA0 (fecha a janela de cima).
        for (flags, tecla, comando) in [(9, 0x4E, 0xFD03), (9, 0x51, 0xFD08), (9, 0x54, 0xFD17), (5, 0x30, 0xFD41), (9, 0x73, 0xFDA0)] {
            assert!(t.contains(&Acelerador { flags, tecla, comando }), "{comando:#x}");
        }
    }

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
