//! Fontes do jogo: FONT do Macintosh (`font1`–`font4`) e `.FNT` do Windows dentro de `FONTRES.FON`.
//! Especificação: docs/spec/formatos/fonte.md.

/// Imagem de texto de 1 bit: `true` = tinta (cor de frente), `false` = fundo.
pub struct ImagemTexto {
    pub largura: usize,
    pub altura: usize,
    pub bits: Vec<bool>,
}

pub enum Fonte {
    Mac(FonteMac),
    Fnt(Fnt),
}

impl Fonte {
    pub fn altura(&self) -> usize {
        match self {
            Fonte::Mac(f) => f.altura_retangulo - 1,
            Fonte::Fnt(f) => f.altura,
        }
    }

    pub fn largura_texto(&self, texto: &[u8]) -> usize {
        match self {
            Fonte::Mac(f) => texto.iter().map(|&c| f.avanco(c)).sum(),
            Fonte::Fnt(f) => texto.iter().map(|&c| f.glifo(c).0).sum(),
        }
    }

    /// Imagem da string como o jogo a monta antes de desenhar (opaca, sem recorte).
    pub fn imagem(&self, texto: &[u8]) -> ImagemTexto {
        match self {
            Fonte::Mac(f) => f.imagem(texto),
            Fonte::Fnt(f) => f.imagem(texto),
        }
    }
}

fn be16(d: &[u8], o: usize) -> Result<i16, String> {
    d.get(o..o + 2).map(|s| i16::from_be_bytes([s[0], s[1]])).ok_or_else(|| "fonte truncada".into())
}

fn le16(d: &[u8], o: usize) -> Result<u16, String> {
    d.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or_else(|| "fonte truncada".into())
}

fn le32(d: &[u8], o: usize) -> Result<u32, String> {
    d.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or_else(|| "fonte truncada".into())
}

/// FONT do Macintosh, como lido por `_font_ReadFont`.
pub struct FonteMac {
    ultimo: usize,
    kern_max: i16,
    altura_retangulo: usize,
    bytes_por_linha: usize,
    faixa: Vec<u8>,
    loc: Vec<u16>,
    ow: Vec<u16>,
}

impl FonteMac {
    pub fn decodificar(d: &[u8]) -> Result<FonteMac, String> {
        let primeiro = be16(d, 2)? as usize;
        let ultimo = be16(d, 4)? as usize;
        if primeiro != 0 || ultimo < primeiro {
            return Err(format!("FONT com caracteres {primeiro}..{ultimo} não suportada"));
        }
        let kern_max = be16(d, 8)?;
        let altura_retangulo = be16(d, 0x0E)? as usize;
        let bytes_por_linha = be16(d, 0x18)? as usize * 2;
        let n = ultimo - primeiro + 3;
        let ini_faixa = 0x1A;
        let ini_loc = ini_faixa + bytes_por_linha * altura_retangulo;
        let ini_ow = ini_loc + 2 * n;
        if d.len() < ini_ow + 2 * n {
            return Err("FONT truncada".into());
        }
        let tabela = |o: usize| (0..n).map(|i| u16::from_be_bytes([d[o + 2 * i], d[o + 2 * i + 1]])).collect();
        Ok(FonteMac {
            ultimo,
            kern_max,
            altura_retangulo,
            bytes_por_linha,
            faixa: d[ini_faixa..ini_loc].to_vec(),
            loc: tabela(ini_loc),
            ow: tabela(ini_ow),
        })
    }

    /// Índice efetivo do caractere: ausentes (0xFFFF ou fora da tabela) viram o glifo "ausente".
    fn indice(&self, c: u8) -> usize {
        let ausente = self.ultimo + 1;
        match self.ow.get(c as usize) {
            Some(&v) if v != 0xFFFF && (c as usize) <= self.ultimo => c as usize,
            _ => ausente,
        }
    }

    fn avanco(&self, c: u8) -> usize {
        let i = self.indice(c);
        (self.ow[i] & 0xFF) as usize + usize::from(self.kern_max < 0)
    }

    fn pixel_faixa(&self, x: usize, y: usize) -> bool {
        self.faixa.get(y * self.bytes_por_linha + x / 8).is_some_and(|b| b & (0x80 >> (x % 8)) != 0)
    }

    fn imagem(&self, texto: &[u8]) -> ImagemTexto {
        let largura: usize = texto.iter().map(|&c| self.avanco(c)).sum();
        let altura = self.altura_retangulo - 1;
        let mut bits = vec![false; largura * altura];
        let mut pena = 0usize;
        for &c in texto {
            let i = self.indice(c);
            let desloc = if self.kern_max < 0 { 0 } else { ((self.ow[i] >> 8) as i32 + self.kern_max as i32).max(0) as usize };
            let (x0, x1) = (self.loc[i] as usize, self.loc[i + 1] as usize);
            for y in 0..altura {
                for (k, sx) in (x0..x1).enumerate() {
                    let dx = pena + desloc + k;
                    // A linha 0 da faixa é ignorada pelo jogo (cópia começa em source + rowBytes).
                    if dx < largura && self.pixel_faixa(sx, y + 1) {
                        bits[y * largura + dx] = true;
                    }
                }
            }
            pena += self.avanco(c);
        }
        ImagemTexto { largura, altura, bits }
    }
}

/// Fonte `.FNT` (versão 2.0) do Windows.
pub struct Fnt {
    pub nome: String,
    pub altura: usize,
    primeiro: u8,
    ultimo: u8,
    padrao: u8,
    tabela: Vec<(usize, usize)>,
    dados: Vec<u8>,
}

impl Fnt {
    pub fn decodificar(d: &[u8]) -> Result<Fnt, String> {
        let versao = le16(d, 0)?;
        if versao != 0x200 {
            return Err(format!(".FNT versão {versao:#x} não suportada"));
        }
        let altura = le16(d, 88)? as usize;
        let (primeiro, ultimo) = (*d.get(95).ok_or("FNT")?, *d.get(96).ok_or("FNT")?);
        let padrao = *d.get(97).ok_or("FNT")?;
        let face = le32(d, 105)? as usize;
        let nome = d.get(face..).map(|s| s.split(|&b| b == 0).next().unwrap_or(&[]).to_vec()).unwrap_or_default();
        let n = (ultimo - primeiro) as usize + 2;
        let tabela = (0..n)
            .map(|i| Ok((le16(d, 118 + i * 4)? as usize, le16(d, 120 + i * 4)? as usize)))
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Fnt { nome: String::from_utf8_lossy(&nome).into_owned(), altura, primeiro, ultimo, padrao, tabela, dados: d.to_vec() })
    }

    /// (largura, offset) do glifo; fora da faixa usa o caractere padrão.
    fn glifo(&self, c: u8) -> (usize, usize) {
        let c = if (self.primeiro..=self.ultimo).contains(&c) { c } else { self.padrao.clamp(self.primeiro, self.ultimo) };
        self.tabela[(c - self.primeiro) as usize]
    }

    fn imagem(&self, texto: &[u8]) -> ImagemTexto {
        let largura: usize = texto.iter().map(|&c| self.glifo(c).0).sum();
        let mut bits = vec![false; largura * self.altura];
        let mut pena = 0;
        for &c in texto {
            let (w, off) = self.glifo(c);
            for y in 0..self.altura {
                for x in 0..w {
                    let b = self.dados.get(off + (x / 8) * self.altura + y).copied().unwrap_or(0);
                    if b & (0x80 >> (x % 8)) != 0 {
                        bits[y * largura + pena + x] = true;
                    }
                }
            }
            pena += w;
        }
        ImagemTexto { largura, altura: self.altura, bits }
    }
}

/// Extrai os recursos FONT (tipo 8) de um executável NE como `FONTRES.FON`, na ordem da tabela.
pub fn fontes_de_fon(d: &[u8]) -> Result<Vec<Fnt>, String> {
    let ne = le16(d, 0x3C)? as usize;
    if d.get(ne..ne + 2) != Some(b"NE") {
        return Err("arquivo .FON não é NE".into());
    }
    let tab = ne + le16(d, ne + 0x24)? as usize;
    let deslocamento = le16(d, tab)?;
    let mut o = tab + 2;
    let mut fontes = Vec::new();
    loop {
        let tipo = le16(d, o)?;
        if tipo == 0 {
            break;
        }
        let n = le16(d, o + 2)? as usize;
        o += 8;
        for _ in 0..n {
            let inicio = (le16(d, o)? as usize) << deslocamento;
            let tamanho = (le16(d, o + 2)? as usize) << deslocamento;
            if tipo == 0x8008 {
                let bloco = d.get(inicio..inicio + tamanho).ok_or("recurso FONT fora do arquivo")?;
                fontes.push(Fnt::decodificar(bloco)?);
            }
            o += 12;
        }
    }
    Ok(fontes)
}

#[cfg(test)]
mod testes {
    use super::*;

    /// FONT mínima: 2 caracteres (0 e 1), altura 3, glifo 0 = coluna cheia nas linhas 1-2.
    fn fonte_mac(kern: i16) -> Vec<u8> {
        let mut d = Vec::new();
        for v in [0x9000u16 as i16, 0, 1, 2, kern, 0, 2, 3, 0, 2, 0, 0, 1] {
            d.extend_from_slice(&v.to_be_bytes());
        }
        d.extend_from_slice(&[0x00, 0x00, 0x80, 0x00, 0x80, 0x00]); // faixa: 3 linhas de 2 bytes
        for v in [0u16, 1, 1, 1] {
            d.extend_from_slice(&v.to_be_bytes()); // loc
        }
        for v in [0x0002u16, 0xFFFF, 0x0001, 0] {
            d.extend_from_slice(&v.to_be_bytes()); // ow: avanço 2; char 1 ausente; glifo ausente avanço 1
        }
        d
    }

    #[test]
    fn mac_ignora_linha_zero_e_usa_avanco() {
        let f = Fonte::Mac(FonteMac::decodificar(&fonte_mac(0)).unwrap());
        let img = f.imagem(&[0]);
        assert_eq!((img.largura, img.altura), (2, 2));
        assert_eq!(img.bits, vec![true, false, true, false]);
    }

    #[test]
    fn mac_kern_negativo_soma_um_ao_avanco() {
        let f = Fonte::Mac(FonteMac::decodificar(&fonte_mac(-1)).unwrap());
        assert_eq!(f.largura_texto(&[0]), 3);
        // caractere ausente usa o glifo "ausente" (avanço 1 + 1)
        assert_eq!(f.largura_texto(&[1]), 2);
    }
}
