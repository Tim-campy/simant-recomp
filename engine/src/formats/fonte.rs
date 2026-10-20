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

    /// Linha de base, contada do topo da célula (`dfAscent` do `.FNT`; no FONT do Mac, a altura
    /// menos um quarto [hipótese: só usada para sublinhar mnemônicos nos diálogos]).
    pub fn ascendente(&self) -> usize {
        match self {
            Fonte::Mac(f) => (f.altura_retangulo - 1) * 3 / 4,
            Fonte::Fnt(f) => f.ascendente,
        }
    }

    /// Negrito simulado (só `.FNT`).
    pub fn negrito(&self) -> bool {
        matches!(self, Fonte::Fnt(f) if f.negrito)
    }

    pub fn largura_texto(&self, texto: &[u8]) -> usize {
        match self {
            Fonte::Mac(f) => texto.iter().map(|&c| f.avanco(c)).sum(),
            Fonte::Fnt(f) => texto.iter().map(|&c| f.avanco(c)).sum(),
        }
    }

    /// Bytes de um texto do port (UTF-8) para esta fonte: as fontes do Windows (FNT) usam o
    /// Latin-1; as do Macintosh (FONT), o Mac Roman. Letra que a fonte não tem vira a letra sem
    /// acento (ou '?').
    pub fn codificar(&self, texto: &str) -> Vec<u8> {
        texto
            .chars()
            .map(|c| {
                let (latin1, mac) = codigos_do_caractere(c);
                let preferido = match self {
                    Fonte::Mac(_) => mac,
                    Fonte::Fnt(_) => latin1,
                };
                match preferido {
                    Some(b) if self.tem_glifo(b) => b,
                    _ => sem_acento(c),
                }
            })
            .collect()
    }

    fn tem_glifo(&self, c: u8) -> bool {
        match self {
            Fonte::Mac(f) => c < 0x80 || f.indice(c) != f.ultimo + 1,
            Fonte::Fnt(f) => (f.primeiro..=f.ultimo).contains(&c),
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
    d.get(o..o + 2).map(|s| i16::from_be_bytes([s[0], s[1]])).ok_or_else(|| "truncated font".into())
}

fn le16(d: &[u8], o: usize) -> Result<u16, String> {
    d.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or_else(|| "truncated font".into())
}

fn le32(d: &[u8], o: usize) -> Result<u32, String> {
    d.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or_else(|| "truncated font".into())
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
            return Err(format!("FONT with characters {primeiro}..{ultimo} not supported"));
        }
        let kern_max = be16(d, 8)?;
        let altura_retangulo = be16(d, 0x0E)? as usize;
        let bytes_por_linha = be16(d, 0x18)? as usize * 2;
        let n = ultimo - primeiro + 3;
        let ini_faixa = 0x1A;
        let ini_loc = ini_faixa + bytes_por_linha * altura_retangulo;
        let ini_ow = ini_loc + 2 * n;
        if d.len() < ini_ow + 2 * n {
            return Err("FONT truncated".into());
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

    /// Avanço contado na largura da string (`__font_StringWidth`, `__font_CharWidth`): o ausente
    /// mede como o glifo `lastChar − firstChar + 1`.
    fn avanco(&self, c: u8) -> usize {
        let i = self.indice(c);
        (self.ow[i] & 0xFF) as usize + usize::from(self.kern_max < 0)
    }

    /// Glifo desenhado por `_font_MakeImage`: para o ausente, o jogo guarda o índice
    /// `lastChar − firstChar + 1` num **byte** (`1030:ade0`), então com `lastChar` = 255 desenha o
    /// glifo 0 (vazio nas quatro fontes) e anda o avanço dele, não o medido. Conferido no balão do
    /// menino nº 4 ("I hope I don't" + 0x7F: nada depois do "t" na bancada; telas/quintal.md §2.3).
    fn indice_desenhado(&self, c: u8) -> usize {
        let i = self.indice(c);
        if i == self.ultimo + 1 {
            i & 0xFF
        } else {
            i
        }
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
            let i = self.indice_desenhado(c);
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
            pena += (self.ow[i] & 0xFF) as usize + usize::from(self.kern_max < 0);
        }
        ImagemTexto { largura, altura, bits }
    }
}

/// Fonte `.FNT` (versão 2.0) do Windows.
pub struct Fnt {
    pub nome: String,
    pub altura: usize,
    pub ascendente: usize,
    primeiro: u8,
    ultimo: u8,
    padrao: u8,
    tabela: Vec<(usize, usize)>,
    dados: Vec<u8>,
    /// Negrito simulado do GDI (fonte de peso 400 pedida em negrito): cada glifo desenhado de novo
    /// um pixel à direita, avanço + 1 [medido nos diálogos comuns do Windows 3.1, telas/arquivos.md].
    pub negrito: bool,
}

impl Fnt {
    pub fn decodificar(d: &[u8]) -> Result<Fnt, String> {
        let versao = le16(d, 0)?;
        if versao != 0x200 {
            return Err(format!(".FNT version {versao:#x} not supported"));
        }
        let altura = le16(d, 88)? as usize;
        let ascendente = le16(d, 74)? as usize;
        let (primeiro, ultimo) = (*d.get(95).ok_or("FNT")?, *d.get(96).ok_or("FNT")?);
        let padrao = *d.get(97).ok_or("FNT")?;
        let face = le32(d, 105)? as usize;
        let nome = d.get(face..).map(|s| s.split(|&b| b == 0).next().unwrap_or(&[]).to_vec()).unwrap_or_default();
        let n = (ultimo - primeiro) as usize + 2;
        let tabela = (0..n)
            .map(|i| Ok((le16(d, 118 + i * 4)? as usize, le16(d, 120 + i * 4)? as usize)))
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Fnt {
            nome: String::from_utf8_lossy(&nome).into_owned(),
            altura,
            ascendente,
            primeiro,
            ultimo,
            padrao,
            tabela,
            dados: d.to_vec(),
            negrito: false,
        })
    }

    /// (largura, offset) do glifo; fora da faixa usa o caractere padrão.
    fn glifo(&self, c: u8) -> (usize, usize) {
        let c = if (self.primeiro..=self.ultimo).contains(&c) { c } else { self.padrao.clamp(self.primeiro, self.ultimo) };
        self.tabela[(c - self.primeiro) as usize]
    }

    fn avanco(&self, c: u8) -> usize {
        self.glifo(c).0 + usize::from(self.negrito)
    }

    fn imagem(&self, texto: &[u8]) -> ImagemTexto {
        let largura: usize = texto.iter().map(|&c| self.avanco(c)).sum();
        let mut bits = vec![false; largura * self.altura];
        let mut pena = 0;
        for &c in texto {
            let (w, off) = self.glifo(c);
            for y in 0..self.altura {
                for x in 0..w {
                    let b = self.dados.get(off + (x / 8) * self.altura + y).copied().unwrap_or(0);
                    if b & (0x80 >> (x % 8)) != 0 {
                        bits[y * largura + pena + x] = true;
                        if self.negrito {
                            bits[y * largura + pena + x + 1] = true;
                        }
                    }
                }
            }
            pena += self.avanco(c);
        }
        ImagemTexto { largura, altura: self.altura, bits }
    }
}

/// Extrai os recursos FONT (tipo 8) de um executável NE como `FONTRES.FON`, na ordem da tabela.
pub fn fontes_de_fon(d: &[u8]) -> Result<Vec<Fnt>, String> {
    let ne = le16(d, 0x3C)? as usize;
    if d.get(ne..ne + 2) != Some(b"NE") {
        return Err(".FON file is not NE".into());
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
                let bloco = d.get(inicio..inicio + tamanho).ok_or("FONT resource outside the file")?;
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

    /// FONT1 (lastChar 255): o 0x7F ausente mede como o glifo 256 (a caixa, avanço 5 + 1), mas
    /// `_font_MakeImage` guarda o índice num byte e desenha o glifo 0, vazio (balão do menino nº 4).
    #[test]
    fn ausente_com_last_char_255_desenha_o_glifo_zero() {
        let dir = std::env::var_os("SIMANT_DIR").map(std::path::PathBuf::from);
        let Some(d) = dir.and_then(|d| std::fs::read(d.join("FONT1")).ok()) else {
            eprintln!("AVISO: FONT1 não encontrada; teste ignorado");
            return;
        };
        let f = Fonte::Mac(FonteMac::decodificar(&d).unwrap());
        let (t, ausente) = (f.largura_texto(b"t"), f.largura_texto(&[0x7F]));
        assert_eq!(ausente, 6, "mede a caixa");
        let img = f.imagem(b"t");
        assert_eq!(img.largura, t + ausente);
        for y in 0..img.altura {
            assert!((t..img.largura).all(|x| !img.bits[y * img.largura + x]), "nada desenhado depois do t");
        }
    }
}

/// Código Latin-1 e Mac Roman de um caractere (só os de 0x00–0xFF do Unicode).
fn codigos_do_caractere(c: char) -> (Option<u8>, Option<u8>) {
    let u = c as u32;
    if u < 0x80 {
        return (Some(u as u8), Some(u as u8));
    }
    if !(0xA0..=0xFF).contains(&u) {
        return (None, None);
    }
    // Latin-1 0xA0–0xFF → Mac Roman (0 = sem equivalente).
    const MAC: [u8; 96] = [
        0xCA, 0xC1, 0xA2, 0xA3, 0xDB, 0xB4, 0, 0xA4, 0xAC, 0xA9, 0xBB, 0xC7, 0xC2, 0, 0xA8, 0xF8, //
        0xA1, 0xB1, 0, 0, 0xAB, 0xB5, 0xA6, 0xE1, 0xFC, 0, 0xBC, 0xC8, 0, 0, 0, 0xC0, //
        0xCB, 0xE7, 0xE5, 0xCC, 0x80, 0x81, 0xAE, 0x82, 0xE9, 0x83, 0xE6, 0xE8, 0xED, 0xEA, 0xEB, 0xEC, //
        0, 0x84, 0xF1, 0xEE, 0xEF, 0xCD, 0x85, 0, 0xAF, 0xF4, 0xF2, 0xF3, 0x86, 0, 0, 0xA7, //
        0x88, 0x87, 0x89, 0x8B, 0x8A, 0x8C, 0xBE, 0x8D, 0x8F, 0x8E, 0x90, 0x91, 0x93, 0x92, 0x94, 0x95, //
        0, 0x96, 0x98, 0x97, 0x99, 0x9B, 0x9A, 0xD6, 0xBF, 0x9D, 0x9C, 0x9E, 0x9F, 0, 0, 0xD8,
    ];
    let mac = MAC[(u - 0xA0) as usize];
    (Some(u as u8), (mac != 0).then_some(mac))
}

/// A letra sem acento (para fontes sem o glifo acentuado).
fn sem_acento(c: char) -> u8 {
    const BASE: [(&str, u8); 12] = [
        ("ÀÁÂÃÄÅ", b'A'),
        ("àáâãäå", b'a'),
        ("ÈÉÊË", b'E'),
        ("èéêë", b'e'),
        ("ÌÍÎÏ", b'I'),
        ("ìíîï", b'i'),
        ("ÒÓÔÕÖ", b'O'),
        ("òóôõö", b'o'),
        ("ÙÚÛÜ", b'U'),
        ("ùúûü", b'u'),
        ("Ç", b'C'),
        ("ç", b'c'),
    ];
    if (c as u32) < 0x80 {
        return c as u8;
    }
    BASE.iter().find(|(l, _)| l.contains(c)).map_or(b'?', |&(_, b)| b)
}

#[cfg(test)]
mod testes_codificar {
    use super::*;

    #[test]
    fn latin1_e_mac_roman() {
        assert_eq!(codigos_do_caractere('ç'), (Some(0xE7), Some(0x8D)));
        assert_eq!(codigos_do_caractere('ã'), (Some(0xE3), Some(0x8B)));
        assert_eq!(codigos_do_caractere('A'), (Some(b'A'), Some(b'A')));
        assert_eq!(codigos_do_caractere('€'), (None, None));
        assert_eq!(sem_acento('ç'), b'c');
        assert_eq!(sem_acento('Ú'), b'U');
        assert_eq!(sem_acento('€'), b'?');
    }
}
