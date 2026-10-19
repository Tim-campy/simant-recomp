//! Layouts de janela (tipo 0), nomes (tipo 1), tabela de cores (0/129) e posições (tipo 9).
//! Especificação: docs/spec/formatos/janela.md.

pub const TIPO_LAYOUT: u8 = 0;
pub const TIPO_NOMES: u8 = 1;
pub const TIPO_POSICOES: u8 = 9;
pub const ID_CABECALHO_CORES: u16 = 0x80;
pub const ID_TABELA_CORES: u16 = 0x81;
/// Tipo de objeto "contorno"; como objeto 0, a janela encolhe pela espessura (`_win_Recalc`).
pub const TIPO_CONTORNO: u8 = 15;

const CABECALHO: usize = 0x2C;
const CAMPO_TAMANHO: usize = 0x22;
const CAMPO_TIPO: usize = 0x21;
const TAMANHO_NOME: usize = 32;
const ENTRADAS_POSICAO: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Retangulo {
    pub esq: i16,
    pub topo: i16,
    pub dir: i16,
    pub base: i16,
}

impl Retangulo {
    /// O retângulo de um objeto como o original o guarda depois de `_win_Recalc` (`1030:e6e2`): a
    /// direita e a base 1 pixel para dentro (as coordenadas viram inclusivas). É o que bate com todas
    /// as medições que usam o retângulo de um objeto (texto da faixa e do fim de jogo, triângulo e
    /// níveis das janelas de controle, barras da faixa, Status, as 25 colunas da edição): o +1 que
    /// `_win_GetObjRect`, `_win_PrintfAtObj` e `_win_DrawHBar` somam com a faixa não aparece em
    /// nenhuma delas [causa do +1 ausente em aberto]. docs/spec/formatos/janela.md, "Cálculo do layout".
    pub fn do_recalc(self) -> Retangulo {
        Retangulo { dir: self.dir - 1, base: self.base - 1, ..self }
    }
}

/// Tipo de objeto, conforme o switch de `_win_DrawObjectI`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conteudo {
    Janela,
    Area,
    Preenchimento,
    Bitmap { id: u16 },
    BotaoBitmap { selecionado: u16, normal: u16 },
    Contorno,
    Relevo { espessura: u8, invertido: bool, preencher: bool },
    Texto { fonte: u8, texto: String, preencher: bool },
    /// Tipos sem desenho próprio (4, 7, 8, 12, 18) ou desconhecidos.
    Outro(u8),
}

#[derive(Clone, Debug)]
pub struct Objeto {
    pub nome: String,
    /// Tipo bruto (offset 0x21).
    pub tipo: u8,
    /// Byte em 0x28: espessura no contorno/relevo, fonte no texto.
    pub byte_28: u8,
    pub retangulo: Retangulo,
    pub flags: u16,
    pub cor: u8,
    pub cor_selecionado: u8,
    pub conteudo: Conteudo,
}

impl Objeto {
    pub fn visivel(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn selecionado(&self) -> bool {
        self.flags & 4 != 0
    }
}

#[derive(Clone, Debug)]
pub struct Janela {
    /// Objeto 0 é a própria janela.
    pub objetos: Vec<Objeto>,
    /// Bytes 0x1C e 0x1D do cabeçalho: estilo da janela do Windows criada por `_win_Open`
    /// (ver `core::gerenciador::Estilo::do_layout`).
    pub flags: [u8; 2],
}

/// Entrada da tabela de cores: frente, fundo e segunda cor de fundo (padrão).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cor {
    pub frente: u8,
    pub fundo: u8,
    pub fundo2: u8,
}

fn i16_le(d: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([d[o], d[o + 1]])
}

fn u16_le(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}

fn asciiz(d: &[u8]) -> String {
    let fim = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    String::from_utf8_lossy(&d[..fim]).into_owned()
}

fn retangulo(d: &[u8], o: usize) -> Retangulo {
    Retangulo { esq: i16_le(d, o), topo: i16_le(d, o + 2), dir: i16_le(d, o + 4), base: i16_le(d, o + 6) }
}

fn conteudo(tipo: u8, ob: &[u8]) -> Conteudo {
    let byte = |o: usize| ob.get(o).copied().unwrap_or(0);
    let palavra = |o: usize| if o + 1 < ob.len() { u16_le(ob, o) } else { 0 };
    let texto = |o: usize| ob.get(o..).map(asciiz).unwrap_or_default();
    match tipo {
        0 => Conteudo::Janela,
        1 => Conteudo::Area,
        2 => Conteudo::Preenchimento,
        5 | 9 => Conteudo::Texto { fonte: byte(0x28), texto: texto(0x2A), preencher: tipo == 5 },
        6 => Conteudo::Bitmap { id: palavra(0x28) },
        13 => Conteudo::BotaoBitmap { selecionado: palavra(0x28), normal: palavra(0x2A) },
        15 => Conteudo::Contorno,
        16 | 17 => Conteudo::Texto { fonte: byte(0x28), texto: texto(0x2E), preencher: tipo == 17 },
        19..=22 => Conteudo::Relevo {
            espessura: byte(0x28),
            invertido: tipo == 20 || tipo == 22,
            preencher: tipo >= 21,
        },
        t => Conteudo::Outro(t),
    }
}

impl Janela {
    /// `layout`: objeto tipo 0; `nomes`: objeto tipo 1 correspondente (pode faltar).
    pub fn decodificar(layout: &[u8], nomes: Option<&[u8]>) -> Result<Janela, String> {
        if layout.len() < CABECALHO {
            return Err("layout de janela menor que o cabeçalho".into());
        }
        let n = i16_le(layout, 0x0C);
        if n < 1 {
            return Err(format!("janela com {n} objetos"));
        }
        let n = n as usize;
        let nome = |i: usize| {
            nomes
                .and_then(|d| d.get(i * TAMANHO_NOME..(i + 1) * TAMANHO_NOME))
                .map(asciiz)
                .unwrap_or_default()
        };
        let mut o = CABECALHO + n * 4;
        let mut objetos = Vec::with_capacity(n);
        for i in 0..n {
            if o + 0x28 > layout.len() {
                return Err(format!("objeto {i} passa do fim do layout"));
            }
            let tamanho = u16_le(layout, o + CAMPO_TAMANHO) as usize;
            if tamanho < 0x28 || o + tamanho > layout.len() {
                return Err(format!("objeto {i} com tamanho inválido ({tamanho})"));
            }
            let ob = &layout[o..o + tamanho];
            objetos.push(Objeto {
                nome: nome(i),
                tipo: ob[CAMPO_TIPO],
                byte_28: ob.get(0x28).copied().unwrap_or(0),
                retangulo: retangulo(ob, 0),
                flags: u16_le(ob, 0x24),
                cor: ob[0x26],
                cor_selecionado: ob[0x27],
                conteudo: conteudo(ob[CAMPO_TIPO], ob),
            });
            o += tamanho;
        }
        if o != layout.len() {
            return Err(format!("sobraram {} bytes depois do último objeto", layout.len() - o));
        }
        Ok(Janela { objetos, flags: [layout[0x1C], layout[0x1D]] })
    }

    pub fn retangulo(&self) -> Retangulo {
        self.objetos[0].retangulo
    }
}

/// Tabela de cores: `cabecalho` = objeto 0/0x80, `tabela` = objeto 0/0x81.
pub fn decodificar_cores(cabecalho: &[u8], tabela: &[u8]) -> Result<Vec<Cor>, String> {
    if cabecalho.len() < 4 {
        return Err("cabeçalho de cores curto".into());
    }
    let n = u16_le(cabecalho, 2) as usize;
    if tabela.len() < n * 6 {
        return Err(format!("tabela de cores com {} bytes, esperava {}", tabela.len(), n * 6));
    }
    Ok((0..n)
        .map(|i| Cor { frente: tabela[i * 6], fundo: tabela[i * 6 + 2], fundo2: tabela[i * 6 + 3] })
        .collect())
}

/// Posição (x, y, largura, altura) de cada janela, a partir de um objeto tipo 9 (ids 0/5/7/8).
pub fn decodificar_posicoes(d: &[u8]) -> Result<Vec<[i16; 4]>, String> {
    if d.len() < ENTRADAS_POSICAO * 8 {
        return Err("tabela de posições curta".into());
    }
    Ok((0..ENTRADAS_POSICAO)
        .map(|i| [i16_le(d, i * 8), i16_le(d, i * 8 + 2), i16_le(d, i * 8 + 4), i16_le(d, i * 8 + 6)])
        .collect())
}
