//! Arquivo de ajuda do WinHelp 3.0 (`SIMANT.HLP`, compilado pelo HC30): sistema de arquivos interno,
//! tópicos com parágrafos, fontes, contextos e figuras. Só leitura.
//!
//! Especificação: docs/spec/formatos/hlp.md (o que foi conferido no `SIMANT.HLP` do usuário).
//! Arquivos do WinHelp 3.1 (HC31, |TOPIC comprimido) não são lidos: o SimAnt só traz o HC30.

use std::collections::HashMap;

fn u16_em(d: &[u8], o: usize) -> Result<u16, String> {
    d.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or_else(|| "HLP truncated".into())
}

fn i16_em(d: &[u8], o: usize) -> Result<i16, String> {
    u16_em(d, o).map(|v| v as i16)
}

fn u32_em(d: &[u8], o: usize) -> Result<u32, String> {
    d.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or_else(|| "HLP truncated".into())
}

fn i32_em(d: &[u8], o: usize) -> Result<i32, String> {
    u32_em(d, o).map(|v| v as i32)
}

fn byte_em(d: &[u8], o: usize) -> Result<u8, String> {
    d.get(o).copied().ok_or_else(|| "HLP truncated".into())
}

/// Valores comprimidos do WinHelp (o bit 0 do primeiro byte diz se o valor ocupa o dobro).
/// "compressed unsigned short": 1 byte (par) ou 2 bytes (ímpar), metade do valor lido.
fn cus(d: &[u8], o: &mut usize) -> Result<u16, String> {
    let b = byte_em(d, *o)?;
    if b & 1 != 0 {
        let v = u16_em(d, *o)? >> 1;
        *o += 2;
        Ok(v)
    } else {
        *o += 1;
        Ok(u16::from(b >> 1))
    }
}

/// "compressed signed short": como o sem sinal, menos 64 (1 byte) ou 16384 (2 bytes).
fn css(d: &[u8], o: &mut usize) -> Result<i16, String> {
    let b = byte_em(d, *o)?;
    if b & 1 != 0 {
        let v = (u16_em(d, *o)? >> 1) as i32 - 16384;
        *o += 2;
        Ok(v as i16)
    } else {
        *o += 1;
        Ok(i16::from(b >> 1) - 64)
    }
}

/// "compressed signed long": 2 bytes (par) ou 4 (ímpar), metade, menos 16384 ou 0x4000000.
fn csl(d: &[u8], o: &mut usize) -> Result<i32, String> {
    let b = byte_em(d, *o)?;
    if b & 1 != 0 {
        let v = (u32_em(d, *o)? >> 1) as i32 - 0x400_0000;
        *o += 4;
        Ok(v)
    } else {
        let v = i32::from(u16_em(d, *o)? >> 1) - 16384;
        *o += 2;
        Ok(v)
    }
}

/// "compressed unsigned long": 2 bytes (par) ou 4 (ímpar), metade.
fn cul(d: &[u8], o: &mut usize) -> Result<u32, String> {
    let b = byte_em(d, *o)?;
    if b & 1 != 0 {
        let v = u32_em(d, *o)? >> 1;
        *o += 4;
        Ok(v)
    } else {
        let v = u32::from(u16_em(d, *o)? >> 1);
        *o += 2;
        Ok(v)
    }
}

/// Descompressão LZ77 do WinHelp: byte de flags (bit 0 primeiro); bit 1 = par (u16): comprimento
/// `(v >> 12) + 3`, distância `(v & 0xFFF) + 1`; bit 0 = byte literal.
pub fn lz77(dados: &[u8], limite: usize) -> Vec<u8> {
    let mut saida = Vec::new();
    let mut i = 0;
    while i < dados.len() && saida.len() < limite {
        let flags = dados[i];
        i += 1;
        for bit in 0..8 {
            if i >= dados.len() || saida.len() >= limite {
                break;
            }
            if flags & (1 << bit) != 0 {
                if i + 1 >= dados.len() {
                    return saida;
                }
                let v = u16::from_le_bytes([dados[i], dados[i + 1]]) as usize;
                i += 2;
                let n = (v >> 12) + 3;
                let dist = (v & 0xFFF) + 1;
                for _ in 0..n {
                    if dist > saida.len() {
                        return saida;
                    }
                    let c = saida[saida.len() - dist];
                    saida.push(c);
                }
            } else {
                saida.push(dados[i]);
                i += 1;
            }
        }
    }
    saida.truncate(limite);
    saida
}

/// RunLen das figuras: byte n; com o bit 7, copia os `n & 0x7F` bytes seguintes; sem ele, repete o
/// byte seguinte `n` vezes.
fn runlen(dados: &[u8]) -> Vec<u8> {
    let mut saida = Vec::new();
    let mut i = 0;
    while i < dados.len() {
        let n = dados[i] as usize;
        i += 1;
        if n & 0x80 != 0 {
            let k = (n & 0x7F).min(dados.len() - i);
            saida.extend_from_slice(&dados[i..i + k]);
            i += k;
        } else if i < dados.len() {
            saida.extend(std::iter::repeat_n(dados[i], n));
            i += 1;
        }
    }
    saida
}

/// Fonte do |FONT (descritor do HC30, 11 bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FonteAjuda {
    pub negrito: bool,
    pub italico: bool,
    pub sublinhado: bool,
    /// Tamanho em meios pontos (24 = 12 pt).
    pub meios_pontos: u8,
    pub familia: u8,
    pub face: String,
    pub cor: [u8; 3],
}

/// Destino de um salto (hotspot).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Salto {
    /// 0xE0 (popup) ou 0xE1 (salto) do HC30: número do tópico (índice do |TOMAP).
    pub numero_topico: u32,
    /// Popup: o WinHelp mostra o tópico numa janela pequena, sem trocar o tópico atual.
    pub popup: bool,
}

/// Um pedaço de um parágrafo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    /// Texto numa fonte (índice do |FONT), talvez dentro de um hotspot.
    Texto { fonte: u16, texto: Vec<u8>, salto: Option<Salto> },
    /// 0x81: quebra de linha dentro do parágrafo.
    Quebra,
    /// 0x83: tabulação.
    Tab,
    /// 0x86 (no texto), 0x87 (à esquerda), 0x88 (à direita): figura `|bmN`.
    Figura { numero: u16, posicao: u8, salto: Option<Salto> },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alinhamento {
    #[default]
    Esquerda,
    Direita,
    Centro,
}

/// Parágrafo (termina num 0x82). As medidas vêm como estão no arquivo [hipótese quanto à unidade;
/// no SIMANT.HLP nenhum parágrafo as usa].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Paragrafo {
    pub alinhamento: Alinhamento,
    pub espaco_acima: i16,
    pub espaco_abaixo: i16,
    pub espaco_linhas: i16,
    pub recuo_esquerdo: i16,
    pub recuo_direito: i16,
    pub recuo_primeira: i16,
    pub tabs: Vec<i16>,
    pub itens: Vec<Item>,
    /// Fonte em vigor no fim do parágrafo (dá a altura de um parágrafo vazio).
    pub fonte_final: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Topico {
    /// Posição do TOPICLINK do cabeçalho no |TOPIC (é o que o |CTXOMAP e o |TOMAP guardam).
    pub posicao: u32,
    /// Título como está no arquivo (no SIMANT.HLP, com "(id=0x…)" no fim).
    pub titulo_bruto: String,
    /// Título sem o "(id=…)".
    pub titulo: String,
    /// Sequência de navegação (<< e >>): números de tópico, ou None.
    pub anterior: Option<u32>,
    pub seguinte: Option<u32>,
    pub paragrafos: Vec<Paragrafo>,
}

/// Uma palavra-chave da busca (Search do WinHelp) e as posições dos tópicos que a têm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PalavraChave {
    /// Como está no arquivo (Latin-1).
    pub palavra: String,
    /// Posições no |TOPIC (como as do |CTXOMAP: `ArquivoAjuda::topico_da_posicao`).
    pub topicos: Vec<u32>,
}

/// Figura decodificada (RGB, de cima para baixo).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Figura {
    pub largura: usize,
    pub altura: usize,
    pub rgb: Vec<[u8; 3]>,
}

pub struct ArquivoAjuda {
    pub titulo: String,
    pub topicos: Vec<Topico>,
    pub fontes: Vec<FonteAjuda>,
    /// |CTXOMAP: número de contexto (HELP_CONTEXT) → posição do tópico.
    contextos: HashMap<u32, u32>,
    /// |TOMAP: número do tópico → posição.
    mapa_topicos: Vec<u32>,
    /// Posição → índice em `topicos`.
    por_posicao: HashMap<u32, usize>,
    dados: Vec<u8>,
    arquivos: HashMap<String, usize>,
}

/// Tamanho dos blocos do |TOPIC no HC30 e do cabeçalho de cada um.
const BLOCO: usize = 2048;
const CABECALHO_BLOCO: usize = 12;

impl ArquivoAjuda {
    pub fn ler(dados: Vec<u8>) -> Result<ArquivoAjuda, String> {
        if u32_em(&dados, 0)? != 0x0003_5F3F {
            return Err("not a WinHelp help file".into());
        }
        let dir = i32_em(&dados, 4)? as usize;
        let arquivos = diretorio(&dados, dir)?;
        let mut a = ArquivoAjuda {
            titulo: String::new(),
            topicos: Vec::new(),
            fontes: Vec::new(),
            contextos: HashMap::new(),
            mapa_topicos: Vec::new(),
            por_posicao: HashMap::new(),
            dados,
            arquivos,
        };
        a.titulo = titulo_do_sistema(a.interno("|SYSTEM")?)?;
        let frases = a.frases()?;
        a.fontes = a.ler_fontes()?;
        a.topicos = a.ler_topicos(&frases)?;
        a.por_posicao = a.topicos.iter().enumerate().map(|(i, t)| (t.posicao, i)).collect();
        if let Ok(c) = a.interno("|CTXOMAP") {
            let c = c.to_vec();
            let n = u16_em(&c, 0)? as usize;
            for k in 0..n {
                a.contextos.insert(u32_em(&c, 2 + 8 * k)?, u32_em(&c, 6 + 8 * k)?);
            }
        }
        if let Ok(t) = a.interno("|TOMAP") {
            a.mapa_topicos = t.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect();
        }
        Ok(a)
    }

    /// Conteúdo de um arquivo interno (sem o cabeçalho de 9 bytes).
    pub fn interno(&self, nome: &str) -> Result<&[u8], String> {
        interno(&self.dados, &self.arquivos, nome)
    }

    /// Só o título do |SYSTEM (`[OPTIONS] TITLE` do projeto), sem ler os tópicos. Usado pelas
    /// referências de texto de `data/ajuda.json` (recursos.rs).
    pub fn titulo_de(dados: &[u8]) -> Result<String, String> {
        if u32_em(dados, 0)? != 0x0003_5F3F {
            return Err("not a WinHelp help file".into());
        }
        let arquivos = diretorio(dados, i32_em(dados, 4)? as usize)?;
        titulo_do_sistema(interno(dados, &arquivos, "|SYSTEM")?)
    }

    /// Palavras-chave da busca (`|KWBTREE` + `|KWDATA`), na ordem da árvore (a alfabética do
    /// arquivo): cada uma com as posições dos tópicos (`|KWDATA`, deslocamentos de 4 bytes).
    /// Sem os arquivos internos, a lista vem vazia.
    pub fn palavras_chave(&self) -> Result<Vec<PalavraChave>, String> {
        let (Ok(k), Ok(dados)) = (self.interno("|KWBTREE"), self.interno("|KWDATA")) else { return Ok(Vec::new()) };
        if u16_em(k, 0)? != 0x293B {
            return Err("invalid |KWBTREE".into());
        }
        let pagina = u16_em(k, 4)? as usize;
        let raiz = i16_em(k, 26)?;
        let niveis = i16_em(k, 32)?;
        let mut p = raiz;
        for _ in 1..niveis {
            p = i16_em(k, 38 + p as usize * pagina + 4)?;
        }
        let mut saida = Vec::new();
        let mut voltas = 0;
        // Folhas: u16 livre, i16 entradas, i16 anterior, i16 seguinte; entradas = palavra (ASCIIZ),
        // u16 quantidade, i32 deslocamento no |KWDATA.
        while p >= 0 && voltas < 10_000 {
            voltas += 1;
            let b = 38 + p as usize * pagina;
            let n = i16_em(k, b + 2)?;
            let prox = i16_em(k, b + 6)?;
            let mut e = b + 8;
            for _ in 0..n {
                let fim = k.get(e..).and_then(|s| s.iter().position(|&c| c == 0)).ok_or("unterminated word in |KWBTREE")? + e;
                let palavra = k[e..fim].iter().map(|&c| c as char).collect();
                let quantos = u16_em(k, fim + 1)? as usize;
                let desl = i32_em(k, fim + 3)? as usize;
                let topicos = (0..quantos).map(|i| u32_em(dados, desl + 4 * i)).collect::<Result<Vec<_>, _>>()?;
                saida.push(PalavraChave { palavra, topicos });
                e = fim + 7;
            }
            p = prox;
        }
        Ok(saida)
    }

    /// Nomes dos arquivos internos.
    pub fn nomes_internos(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.arquivos.keys().map(String::as_str).collect();
        v.sort();
        v
    }

    /// Tópico do número de contexto (`WinHelp(…, HELP_CONTEXT, n)`).
    pub fn topico_do_contexto(&self, n: u32) -> Option<usize> {
        self.contextos.get(&n).and_then(|p| self.topico_da_posicao(*p))
    }

    /// Tópico do número usado pelos saltos e pela sequência de navegação.
    pub fn topico_do_numero(&self, n: u32) -> Option<usize> {
        self.mapa_topicos.get(n as usize).and_then(|p| self.topico_da_posicao(*p))
    }

    /// Tópico que começa na posição (ou o último que começa antes dela).
    pub fn topico_da_posicao(&self, p: u32) -> Option<usize> {
        if let Some(&i) = self.por_posicao.get(&p) {
            return Some(i);
        }
        self.topicos.iter().rposition(|t| t.posicao <= p)
    }

    fn frases(&self) -> Result<Vec<Vec<u8>>, String> {
        let Ok(p) = self.interno("|Phrases") else { return Ok(Vec::new()) };
        let n = u16_em(p, 0)? as usize;
        // HC30: tabela de n + 1 deslocamentos a partir do byte 4, relativos ao início da tabela.
        let offs = (0..=n).map(|k| u16_em(p, 4 + 2 * k).map(|v| v as usize + 4)).collect::<Result<Vec<_>, _>>()?;
        (0..n).map(|k| p.get(offs[k]..offs[k + 1]).map(<[u8]>::to_vec).ok_or_else(|| "|Phrases truncated".to_string())).collect()
    }

    fn ler_fontes(&self) -> Result<Vec<FonteAjuda>, String> {
        let Ok(f) = self.interno("|FONT") else { return Ok(Vec::new()) };
        let (nfaces, ndesc, off_face, off_desc) = (u16_em(f, 0)? as usize, u16_em(f, 2)? as usize, u16_em(f, 4)? as usize, u16_em(f, 6)? as usize);
        let tam_face = off_desc.saturating_sub(off_face).checked_div(nfaces).unwrap_or(0);
        let faces: Vec<String> = (0..nfaces)
            .map(|k| {
                let s = f.get(off_face + k * tam_face..off_face + (k + 1) * tam_face).unwrap_or(&[]);
                let fim = s.iter().position(|&b| b == 0).unwrap_or(s.len());
                String::from_utf8_lossy(&s[..fim]).into_owned()
            })
            .collect();
        (0..ndesc)
            .map(|k| {
                let o = off_desc + 11 * k;
                let attr = byte_em(f, o)?;
                let face = u16_em(f, o + 3)? as usize;
                Ok(FonteAjuda {
                    negrito: attr & 1 != 0,
                    italico: attr & 2 != 0,
                    sublinhado: attr & 4 != 0,
                    meios_pontos: byte_em(f, o + 1)?,
                    familia: byte_em(f, o + 2)?,
                    face: faces.get(face).cloned().unwrap_or_default(),
                    cor: [byte_em(f, o + 5)?, byte_em(f, o + 6)?, byte_em(f, o + 7)?],
                })
            })
            .collect()
    }

    /// `n` bytes do |TOPIC a partir da posição bruta, pulando os cabeçalhos dos blocos.
    fn ler_topic(t: &[u8], mut pos: usize, n: usize) -> Vec<u8> {
        let mut saida = Vec::with_capacity(n);
        while saida.len() < n && pos < t.len() {
            let off = pos % BLOCO;
            if off < CABECALHO_BLOCO {
                pos += CABECALHO_BLOCO - off;
                continue;
            }
            let k = (n - saida.len()).min(BLOCO - off).min(t.len() - pos);
            saida.extend_from_slice(&t[pos..pos + k]);
            pos += k;
        }
        saida
    }

    fn ler_topicos(&self, frases: &[Vec<u8>]) -> Result<Vec<Topico>, String> {
        let t = self.interno("|TOPIC")?;
        let mut topicos: Vec<Topico> = Vec::new();
        let mut pos = CABECALHO_BLOCO;
        let mut fonte = 0u16;
        while pos < t.len() {
            let cab = Self::ler_topic(t, pos, 21);
            if cab.len() < 21 {
                break;
            }
            let tam = i32_em(&cab, 0)? as usize;
            let d2len = i32_em(&cab, 4)? as usize;
            let prox = i32_em(&cab, 12)?;
            let d1len = i32_em(&cab, 16)? as usize;
            let tipo = cab[20];
            if tam < 21 || d1len < 21 || d1len > tam {
                return Err(format!("invalid TOPICLINK at {pos}"));
            }
            let dados = Self::ler_topic(t, pos, tam);
            let d1 = &dados[21..d1len.min(dados.len())];
            let bruto = &dados[d1len.min(dados.len())..];
            let d2 = if d2len > bruto.len() { expandir(bruto, frases) } else { bruto.to_vec() };
            match tipo {
                2 => {
                    let fim = d2.iter().position(|&b| b == 0).unwrap_or(d2.len());
                    let titulo_bruto = String::from_utf8_lossy(&d2[..fim]).into_owned();
                    let titulo = match titulo_bruto.find("(id=") {
                        Some(i) => titulo_bruto[..i].trim_end().to_string(),
                        None => titulo_bruto.clone(),
                    };
                    let num = |o: usize| i32_em(d1, o).ok().filter(|&v| v >= 0).map(|v| v as u32);
                    topicos.push(Topico { posicao: pos as u32, titulo_bruto, titulo, anterior: num(4), seguinte: num(8), paragrafos: Vec::new() });
                    fonte = 0;
                }
                1 => {
                    if let Some(top) = topicos.last_mut() {
                        let ps = paragrafos(d1, &d2, &mut fonte).map_err(|e| format!("topic {}: {e}", top.titulo))?;
                        top.paragrafos.extend(ps);
                    }
                }
                _ => {}
            }
            if prox <= 0 {
                break;
            }
            pos += prox as usize;
        }
        Ok(topicos)
    }

    /// Figura `|bmN` (formato MRB/SHG "lp", primeira imagem; só DIB/DDB).
    pub fn figura(&self, n: u16) -> Result<Figura, String> {
        let d = self.interno(&format!("bm{n}"))?;
        if u16_em(d, 0)? != 0x706C {
            return Err(format!("bm{n}: not MRB/SHG"));
        }
        let base = u32_em(d, 4)? as usize;
        let tipo = byte_em(d, base)?;
        let empacotamento = byte_em(d, base + 1)?;
        if tipo != 5 && tipo != 6 {
            return Err(format!("bm{n}: picture type {tipo} not supported"));
        }
        let mut o = base + 2;
        let _xdpi = cul(d, &mut o)?;
        let _ydpi = cul(d, &mut o)?;
        let _planos = cus(d, &mut o)?;
        let bits = cus(d, &mut o)? as usize;
        let largura = cul(d, &mut o)? as usize;
        let altura = cul(d, &mut o)? as usize;
        let cores_usadas = cul(d, &mut o)? as usize;
        let _importantes = cul(d, &mut o)?;
        let tam = cul(d, &mut o)? as usize;
        let _hotspots = cul(d, &mut o)?;
        let off_dados = u32_em(d, o)? as usize;
        o += 8;
        let ncores = if cores_usadas == 0 && bits <= 8 { 1 << bits } else { cores_usadas };
        let paleta: Vec<[u8; 3]> = if tipo == 6 {
            (0..ncores).map(|k| Ok([byte_em(d, o + 4 * k + 2)?, byte_em(d, o + 4 * k + 1)?, byte_em(d, o + 4 * k)?])).collect::<Result<_, String>>()?
        } else {
            Vec::new()
        };
        let comp = d.get(base + off_dados..base + off_dados + tam).ok_or(format!("bm{n}: data outside the file"))?;
        let linha = (largura * bits).div_ceil(32) * 4;
        let total = linha * altura;
        let crus = match empacotamento {
            0 => comp.to_vec(),
            1 => runlen(comp),
            2 => lz77(comp, total),
            3 => runlen(&lz77(comp, usize::MAX)),
            e => return Err(format!("bm{n}: packing {e}")),
        };
        if tipo == 5 || !matches!(bits, 1 | 4 | 8) {
            return Err(format!("bm{n}: {bits} bits per pixel (type {tipo}) not supported"));
        }
        let mut rgb = vec![[0u8; 3]; largura * altura];
        for y in 0..altura {
            // DIB: linhas de baixo para cima.
            let l = &crus.get((altura - 1 - y) * linha..(altura - y) * linha).unwrap_or(&[]);
            for x in 0..largura {
                let i = match bits {
                    1 => l.get(x / 8).map_or(0, |b| (b >> (7 - x % 8)) & 1),
                    4 => l.get(x / 2).map_or(0, |b| if x % 2 == 0 { b >> 4 } else { b & 0xF }),
                    _ => l.get(x).copied().unwrap_or(0),
                } as usize;
                rgb[y * largura + x] = paleta.get(i).copied().unwrap_or([0, 0, 0]);
            }
        }
        Ok(Figura { largura, altura, rgb })
    }
}

/// Diretório (árvore B+ de nomes → posição do arquivo interno).
fn interno<'a>(dados: &'a [u8], arquivos: &HashMap<String, usize>, nome: &str) -> Result<&'a [u8], String> {
    let &o = arquivos.get(nome).ok_or_else(|| format!("internal file {nome} does not exist"))?;
    let usado = i32_em(dados, o + 4)? as usize;
    dados.get(o + 9..o + 9 + usado).ok_or_else(|| format!("{nome} truncated"))
}

/// Título do |SYSTEM do HC30: cabeçalho de 12 bytes (0x036C, versão menor até 16, ...) e o texto
/// ASCIIZ. Os bytes viram caracteres Latin-1, como nas listas de strings do banco.
fn titulo_do_sistema(sistema: &[u8]) -> Result<String, String> {
    if u16_em(sistema, 0)? != 0x036C {
        return Err("invalid |SYSTEM".into());
    }
    let menor = u16_em(sistema, 2)?;
    if menor > 16 {
        return Err(format!("WinHelp 3.1 help (|SYSTEM minor {menor}) not supported; only HC30"));
    }
    Ok(sistema.get(12..).unwrap_or(&[]).iter().take_while(|&&b| b != 0).map(|&b| b as char).collect())
}

fn diretorio(d: &[u8], inicio: usize) -> Result<HashMap<String, usize>, String> {
    let o = inicio + 9;
    if u16_em(d, o)? != 0x293B {
        return Err("invalid HLP directory".into());
    }
    let pagina = u16_em(d, o + 4)? as usize;
    let raiz = i16_em(d, o + 26)?;
    let niveis = i16_em(d, o + 32)?;
    let paginas = o + 38;
    let mut p = raiz;
    for _ in 1..niveis {
        p = i16_em(d, paginas + p as usize * pagina + 4)?;
    }
    let mut saida = HashMap::new();
    let mut voltas = 0;
    while p >= 0 && voltas < 10_000 {
        voltas += 1;
        let b = paginas + p as usize * pagina;
        let n = i16_em(d, b + 2)?;
        let prox = i16_em(d, b + 6)?;
        let mut e = b + 8;
        for _ in 0..n {
            let fim = d.get(e..).and_then(|s| s.iter().position(|&c| c == 0)).ok_or("unterminated name in the directory")? + e;
            let nome = String::from_utf8_lossy(&d[e..fim]).into_owned();
            saida.insert(nome, i32_em(d, fim + 1)? as usize);
            e = fim + 5;
        }
        p = prox;
    }
    Ok(saida)
}

/// Texto com frases: bytes 1..9 seguidos de um byte formam o índice `256 × (c − 1) + d`; a frase é
/// `índice / 2`, seguida de espaço se o índice é ímpar.
fn expandir(d: &[u8], frases: &[Vec<u8>]) -> Vec<u8> {
    let mut s = Vec::with_capacity(d.len() * 2);
    let mut i = 0;
    while i < d.len() {
        let c = d[i];
        if (1..10).contains(&c) && i + 1 < d.len() && !frases.is_empty() {
            let idx = 256 * (c as usize - 1) + d[i + 1] as usize;
            if let Some(f) = frases.get(idx / 2) {
                s.extend_from_slice(f);
            }
            if idx & 1 != 0 {
                s.push(b' ');
            }
            i += 2;
        } else {
            s.push(c);
            i += 1;
        }
    }
    s
}

/// Parágrafos de um registro de texto (tipo 1 do HC30): `d1` = LinkData1, `d2` = texto expandido.
/// `fonte` é a fonte corrente, que passa de um registro para o seguinte.
fn paragrafos(d1: &[u8], d2: &[u8], fonte: &mut u16) -> Result<Vec<Paragrafo>, String> {
    let mut o = 0;
    let _tamanho = csl(d1, &mut o)?;
    o += 4; // dois bytes sem uso conhecido e o id
    let bits = u16_em(d1, o)?;
    o += 2;
    let mut modelo = Paragrafo::default();
    if bits & 0x0001 != 0 {
        csl(d1, &mut o)?;
    }
    if bits & 0x0002 != 0 {
        modelo.espaco_acima = css(d1, &mut o)?;
    }
    if bits & 0x0004 != 0 {
        modelo.espaco_abaixo = css(d1, &mut o)?;
    }
    if bits & 0x0008 != 0 {
        modelo.espaco_linhas = css(d1, &mut o)?;
    }
    if bits & 0x0010 != 0 {
        modelo.recuo_esquerdo = css(d1, &mut o)?;
    }
    if bits & 0x0020 != 0 {
        modelo.recuo_direito = css(d1, &mut o)?;
    }
    if bits & 0x0040 != 0 {
        modelo.recuo_primeira = css(d1, &mut o)?;
    }
    if bits & 0x0100 != 0 {
        o += 3;
    }
    if bits & 0x0200 != 0 {
        let n = css(d1, &mut o)?;
        for _ in 0..n.max(0) {
            let t = cus(d1, &mut o)?;
            if t & 0x4000 != 0 {
                cus(d1, &mut o)?;
            }
            modelo.tabs.push((t & 0x3FFF) as i16);
        }
    }
    modelo.alinhamento = if bits & 0x0800 != 0 {
        Alinhamento::Centro
    } else if bits & 0x0400 != 0 {
        Alinhamento::Direita
    } else {
        Alinhamento::Esquerda
    };
    let textos: Vec<&[u8]> = d2.split(|&b| b == 0).collect();
    let mut k = 0;
    let mut saida = Vec::new();
    let mut atual = Paragrafo { itens: Vec::new(), ..modelo.clone() };
    let mut salto: Option<Salto> = None;
    let por_texto = |p: &mut Paragrafo, k: &mut usize, fonte: u16, salto: Option<Salto>| {
        if let Some(t) = textos.get(*k) {
            if !t.is_empty() {
                p.itens.push(Item::Texto { fonte, texto: t.to_vec(), salto });
            }
        }
        *k += 1;
    };
    loop {
        por_texto(&mut atual, &mut k, *fonte, salto);
        let Some(&c) = d1.get(o) else { break };
        o += 1;
        match c {
            0xFF => break,
            0x80 => {
                *fonte = u16_em(d1, o)?;
                o += 2;
            }
            0x81 => atual.itens.push(Item::Quebra),
            0x82 => {
                atual.fonte_final = *fonte;
                saida.push(std::mem::replace(&mut atual, Paragrafo { itens: Vec::new(), ..modelo.clone() }));
            }
            0x83 => atual.itens.push(Item::Tab),
            0x86..=0x88 => {
                let tipo = byte_em(d1, o)?;
                o += 1;
                let tam = csl(d1, &mut o)?.max(0) as usize;
                if tipo == 0x22 {
                    cus(d1, &mut o)?;
                }
                let embutida = i16_em(d1, o)?;
                if embutida == 0 && (tipo == 3 || tipo == 0x22) {
                    let numero = u16_em(d1, o + 2)?;
                    atual.itens.push(Item::Figura { numero, posicao: c, salto });
                }
                o += tam;
            }
            0x89 => salto = None,
            0x8B | 0x8C => {}
            0xC8 | 0xCC => {
                let n = u16_em(d1, o)? as usize;
                o += 2 + n;
            }
            0xE0..=0xE7 => {
                let n = u32_em(d1, o)?;
                o += 4;
                salto = Some(Salto { numero_topico: n, popup: c & 1 == 0 });
            }
            0xEA..=0xEF => {
                let n = u16_em(d1, o)? as usize;
                o += 2 + n;
            }
            outro => return Err(format!("unknown formatting command 0x{outro:02X}")),
        }
    }
    if !atual.itens.is_empty() {
        atual.fonte_final = *fonte;
        saida.push(atual);
    }
    Ok(saida)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn valores_comprimidos() {
        let mut o = 0;
        assert_eq!(csl(&[0x08, 0x80], &mut o).unwrap(), 4);
        assert_eq!(o, 2);
        let mut o = 0;
        assert_eq!(cus(&[0x08], &mut o).unwrap(), 4);
        let mut o = 0;
        assert_eq!(cus(&[0x03, 0x01], &mut o).unwrap(), 0x81);
        let mut o = 0;
        assert_eq!(css(&[0x80], &mut o).unwrap(), 0);
    }

    #[test]
    fn lz77_e_runlen() {
        // literais "ab" e um par (distância 2, comprimento 4) → "ababab"
        assert_eq!(lz77(&[0b100, b'a', b'b', 0x01, 0x10], 100), b"ababab");
        assert_eq!(runlen(&[3, 7, 0x82, 1, 2]), vec![7, 7, 7, 1, 2]);
    }
}
