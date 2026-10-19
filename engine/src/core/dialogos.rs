//! Diálogos do Windows que o SimAnt usa durante a partida: a caixa de mensagem (`MessageBox`, como
//! "SimAnt Load: Do you want to save the current ant hill?" e o "%s Saved correctly" de `_PopMsg`)
//! e o diálogo comum de arquivo (`_FileSelect`: "Load Ant Hill" / "Save Ant Hill As").
//!
//! Especificação: docs/spec/telas/arquivos.md. Geometria e textos: data/dialogos.json (medidos na
//! bancada QEMU). O desenho imita o Windows 3.1 com as fontes do Windows do usuário; os ícones são
//! desenhos próprios simplificados.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::carregar::{arquivos_ant, caminho_do_nome};
use super::controles::CampoTexto;
use super::quadro::Quadro;
use super::telas::Interface;
use crate::formats::fonte::Fonte;
use crate::formats::janela::Retangulo;

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_CONFIG: &str = "dialogos.json";

/// Retângulo (x, y, largura, altura) na área cliente.
pub type Caixa = [i16; 4];

fn ret(c: Caixa) -> Retangulo {
    Retangulo { esq: c[0], topo: c[1], dir: c[0] + c[2], base: c[1] + c[3] }
}

fn contem(r: Retangulo, x: i32, y: i32) -> bool {
    x >= i32::from(r.esq) && x < i32::from(r.dir) && y >= i32::from(r.topo) && y < i32::from(r.base)
}

#[derive(Clone, Debug, Deserialize)]
pub struct Cores {
    pub linha: u8,
    pub fundo: u8,
    pub titulo: u8,
    pub texto_titulo: u8,
    pub caixa_sistema: u8,
    pub texto: u8,
    pub claro: u8,
    pub escuro: u8,
    pub botao: u8,
    pub campo: u8,
    pub selecao: u8,
    pub texto_selecao: u8,
    pub desabilitado: u8,
    pub moldura: u8,
    pub icone: u8,
    /// Ícone de informação ("i" de `_PopMsg`): azul-escuro no original (bancada, "Saved correctly").
    pub icone_informacao: u8,
    pub pasta: u8,
    /// Cor dos pontos do retângulo de foco sobre o realce (amarelo; opcional, API 0.40).
    #[serde(default)]
    pub foco_selecao: Option<u8>,
}


/// Estilo da caixa: espessura da moldura (contorno incluído: esquerda, cima, direita, baixo) e a
/// margem branca em volta do título [medido: a pergunta do Load tem só o contorno; a "Message" do
/// `_PopMsg`, a moldura de diálogo azul]. A posição é a regra do Windows
/// ([`Interface::centrar_dialogo`]: centrada na tela, x no byte).
#[derive(Clone, Debug, Deserialize)]
pub struct EstiloCaixa {
    pub borda: [i16; 4],
    pub margem_titulo: i16,
    /// Obsoleto desde a API 0.29 (o centro medido em 640×480); lido e ignorado.
    #[serde(default)]
    pub centro: Option<[i16; 2]>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigCaixaMensagem {
    pub fontes: Vec<usize>,
    pub estilos: Vec<EstiloCaixa>,
    pub altura_titulo: i16,
    /// x do ícone e do texto em relação ao interior; o ícone fica `icone_abaixo_titulo` linhas abaixo
    /// da linha do título; o texto, centrado na altura do ícone quando é mais baixo que ele.
    pub icone_x: i16,
    pub icone_abaixo_titulo: i16,
    pub lado_icone: i16,
    pub texto_x: i16,
    pub margem_direita: i16,
    pub altura_linha: i16,
    /// Botões: `folga_botoes` abaixo do ícone ou do texto; o interior acaba `margem_baixo` abaixo deles.
    pub folga_botoes: i16,
    pub margem_baixo: i16,
    /// Largura e altura de cada botão.
    pub botao: [i16; 2],
    /// (número de botões, largura do grupo, x de cada botão no grupo).
    pub grupos_botoes: Vec<(usize, i16, Vec<i16>)>,
    pub titulo_load: String,
    pub pergunta_load: String,
    /// Título da pergunta de `_MenuQuit` (SIMANTW.EXE 10:1650; o port traz a referência); o texto e os
    /// botões são os do Load. Sem ele (um mod que troca `caixa_mensagem` inteira sem a chave), vale
    /// o título do Load.
    #[serde(default)]
    pub titulo_quit: Option<String>,
    pub titulo_mensagem: String,
    pub salvo: String,
    pub sim: String,
    pub nao: String,
    pub cancelar: String,
    pub ok: String,
    pub padrao_load: usize,
}

fn topo_na_tela_padrao() -> i16 {
    38
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigDialogoArquivo {
    pub fontes: Vec<usize>,
    pub externo: Caixa,
    /// Espessura da moldura (contorno incluído): esquerda, cima, direita, baixo.
    pub moldura: [i16; 4],
    pub altura_titulo: i16,
    pub titulo_abrir: String,
    pub titulo_salvar: String,
    pub campo: Caixa,
    pub lista: Caixa,
    pub pastas: Caixa,
    pub tipo: Caixa,
    pub unidades: Caixa,
    pub ok: Caixa,
    pub cancelar: Caixa,
    pub largura_rolagem: i16,
    pub altura_item: i16,
    pub altura_item_pasta: i16,
    pub rotulos_abrir: Vec<(String, [i16; 2])>,
    pub rotulos_salvar: Vec<(String, [i16; 2])>,
    pub texto_tipo: String,
    pub texto_unidade: String,
    pub texto_ok: String,
    pub texto_cancelar: String,
    pub somente_leitura: (String, [i16; 2]),
    pub campo_inicial: String,
    pub caminho: [i16; 2],
    pub recuo_pasta: i16,
    pub texto_pasta: i16,
    /// Listas suspensas, ícones das unidades, limite do nome, repetição das setas e a roda do
    /// mouse (telas/arquivos.md §3.1). Opcional (API 0.38): sem ela, os valores medidos.
    #[serde(default)]
    pub controles: ControlesArquivo,
}

/// Medidas dos controles do diálogo de arquivo [bancada 2026-10-19, `qemu_dialogo_controles.txt`,
/// capturas `re_trabalho/ag_dlgaj/dlg/g00`–`g21`].
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ControlesArquivo {
    /// Lista aberta de "List Files of Type": itens de 13 e 7 linhas visíveis (g01: a lista vai da
    /// linha de baixo da caixa fechada até 93 linhas abaixo).
    pub tipo_altura_item: i16,
    pub tipo_visiveis: usize,
    /// Lista aberta de "Drives:": itens de 16 e 5 visíveis (g03).
    pub unidades_altura_item: i16,
    pub unidades_visiveis: usize,
    /// Ícone e texto de cada unidade na lista aberta (x, y do ícone e x do texto, a partir do
    /// canto do item) e na caixa fechada (a partir do canto da caixa).
    pub unidade_icone: [i16; 2],
    pub unidade_texto: i16,
    pub unidade_icone_fechada: [i16; 2],
    pub unidade_texto_fechada: i16,
    /// Ícones 16 × 9 das unidades por tipo ("disquete", "disco"), uma linha de texto por linha de
    /// pixels: '#' preto, 'w' branco, '.' cinza-claro, ':' cinza-escuro, outro = transparente.
    pub icones_unidade: std::collections::HashMap<String, Vec<String>>,
    /// Caracteres do nome de arquivo (`nMaxFile` do diálogo comum: 256) [o port: 127].
    pub limite_nome: usize,
    /// Repetição das setas e do trilho com o botão apertado: espera antes da primeira e intervalo
    /// (ms) [hipótese: o ritmo do Windows 3.1 não foi medido].
    pub repeticao_ms: [u32; 2],
    /// Linhas por passo da roda do mouse (extensão do port: o Windows 3.1 não tinha roda).
    pub linhas_roda: usize,
}

impl Default for ControlesArquivo {
    fn default() -> Self {
        ControlesArquivo {
            tipo_altura_item: 13,
            tipo_visiveis: 7,
            unidades_altura_item: 16,
            unidades_visiveis: 5,
            unidade_icone: [5, 3],
            unidade_texto: 27,
            unidade_icone_fechada: [7, 6],
            unidade_texto_fechada: 28,
            icones_unidade: std::collections::HashMap::new(),
            limite_nome: 127,
            repeticao_ms: [400, 50],
            linhas_roda: 3,
        }
    }
}


impl ConfigDialogos {
    /// `data/dialogos.json` com o diálogo de arquivo no byte do perfil de driver ativo
    /// (`data/resolucao.json`).
    pub fn carregar(recursos: &crate::recursos::Recursos) -> Result<ConfigDialogos, String> {
        use super::resolucao::{ConfigResolucao, ARQUIVO};
        let mut c: ConfigDialogos = recursos.dados(ARQUIVO_CONFIG)?;
        if recursos.existe_dado(ARQUIVO) {
            c.arquivo.aplicar_driver(&recursos.dados::<ConfigResolucao>(ARQUIVO)?.perfil(None)?);
        }
        Ok(c)
    }
}

impl ConfigDialogoArquivo {
    /// O diálogo comum fica num lugar fixo em relação à janela principal (o mesmo em 640×480,
    /// 800×600 e 1024×768); os x de `data/dialogos.json` são os sem alinhamento (256 cores) e o
    /// Windows põe o canto no byte mais próximo (`CS_BYTEALIGNWINDOW`; telas/resolucao.md §3).
    pub fn aplicar_driver(&mut self, driver: &super::resolucao::PerfilDriver) {
        let dx = driver.alinhar(self.externo[0]) - self.externo[0];
        for c in [&mut self.externo, &mut self.campo, &mut self.lista, &mut self.pastas, &mut self.tipo, &mut self.unidades, &mut self.ok, &mut self.cancelar] {
            c[0] += dx;
        }
        for (_, p) in self.rotulos_abrir.iter_mut().chain(self.rotulos_salvar.iter_mut()) {
            p[0] += dx;
        }
        self.somente_leitura.1[0] += dx;
        self.caminho[0] += dx;
    }
}

/// Conteúdo de data/dialogos.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigDialogos {
    /// Linha da tela onde começa a área cliente (título e barra de menus em cima: 38 na bancada); usada
    /// para pôr os diálogos sobre as telas da abertura, que são da tela inteira.
    #[serde(default = "topo_na_tela_padrao")]
    pub topo_na_tela: i16,
    pub fontes_titulo: Vec<usize>,
    pub cores: Cores,
    pub caixa_mensagem: ConfigCaixaMensagem,
    pub arquivo: ConfigDialogoArquivo,
}

fn fonte<'i>(lista: &[usize], interface: &'i Interface) -> Option<&'i Fonte> {
    lista.iter().find_map(|&i| interface.fontes.get(i).and_then(|f| f.as_ref()))
}

fn bytes(texto: &str) -> Vec<u8> {
    texto.chars().filter(|&c| c != '&').map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect()
}

fn largura(f: &Fonte, texto: &str) -> i32 {
    f.largura_texto(&bytes(texto)) as i32
}

/// Texto com a célula em (x, y), transparente; `&` sublinha a letra seguinte (na linha de base + 2).
fn escrever(q: &mut Quadro, f: &Fonte, x: i32, y: i32, texto: &str, cor: u8) {
    let img = f.imagem(&bytes(texto));
    for j in 0..img.altura {
        for i in 0..img.largura {
            if img.bits[j * img.largura + i] {
                q.ponto(x + i as i32, y + j as i32, cor);
            }
        }
    }
    if let Some(p) = texto.find('&') {
        let antes = largura(f, &texto[..p]);
        let letra: String = texto[p + 1..].chars().take(1).collect();
        // Largura do glifo, mais o pixel do negrito simulado.
        let w = largura(f, &letra) + i32::from(f.negrito());
        let yb = y + f.ascendente() as i32 + 1;
        q.linha(x + antes, yb, x + antes + w - 1, yb, cor);
    }
}

/// Relevo de um botão no retângulo interno `i` (dentro do contorno preto): `claro` linhas de luz em
/// cima e à esquerda e duas de sombra embaixo e à direita, em escada nos cantos.
fn relevo(q: &mut Quadro, c: &Cores, i: Retangulo, claro: i16) {
    q.preencher(i, c.botao, c.botao);
    q.preencher(Retangulo { base: i.topo + claro, ..i }, c.claro, c.claro);
    q.preencher(Retangulo { dir: i.esq + claro, ..i }, c.claro, c.claro);
    let e = c.escuro;
    q.preencher(Retangulo { topo: i.base - 1, ..i }, e, e);
    q.preencher(Retangulo { esq: i.esq + 1, topo: i.base - 2, base: i.base - 1, ..i }, e, e);
    q.preencher(Retangulo { esq: i.dir - 1, ..i }, e, e);
    q.preencher(Retangulo { esq: i.dir - 2, dir: i.dir - 1, topo: i.topo + 1, ..i }, e, e);
}

/// Título do Windows: caixa do menu de sistema com o traço, faixa azul com o texto centrado e a
/// linha de baixo (na linha `r.base`).
fn titulo(q: &mut Quadro, f: Option<&Fonte>, c: &Cores, r: Retangulo, texto: &str) {
    let h = r.base - r.topo;
    q.preencher(r, c.titulo, c.titulo);
    let caixa = Retangulo { dir: r.esq + h, ..r };
    q.preencher(caixa, c.caixa_sistema, c.caixa_sistema);
    let (cx, cy) = (caixa.esq + 2, caixa.topo + 7);
    q.contorno(Retangulo { esq: cx, topo: cy, dir: cx + 12, base: cy + 2 }, c.linha);
    q.preencher(Retangulo { esq: cx + 1, topo: cy + 1, dir: cx + 12, base: cy + 2 }, c.claro, c.claro);
    q.preencher(Retangulo { esq: cx + 1, topo: cy + 3, dir: cx + 14, base: cy + 4 }, c.escuro, c.escuro);
    q.preencher(Retangulo { esq: cx + 13, topo: cy + 1, dir: cx + 14, base: cy + 4 }, c.escuro, c.escuro);
    q.preencher(Retangulo { esq: caixa.dir, dir: caixa.dir + 1, ..r }, c.linha, c.linha);
    q.preencher(Retangulo { topo: r.base, base: r.base + 1, ..r }, c.linha, c.linha);
    if let Some(f) = f {
        let w = largura(f, texto);
        let x = (i32::from(caixa.dir + 1) + i32::from(r.dir) - w) / 2;
        let y = (i32::from(r.topo) + i32::from(r.base) - f.altura() as i32) / 2;
        escrever(q, f, x, y, texto, c.texto_titulo);
    }
}

/// Botão de comando do Windows 3.1: contorno preto de cantos cortados (dois no botão padrão),
/// relevo de 2 pixels e o texto centrado.
fn botao(q: &mut Quadro, f: Option<&Fonte>, c: &Cores, r: Retangulo, texto: &str, padrao: bool) {
    let (l, t, d, b) = (r.esq, r.topo, r.dir - 1, r.base - 1);
    let k = c.linha;
    q.preencher(Retangulo { esq: l + 1, topo: t, dir: d, base: t + 1 }, k, k);
    q.preencher(Retangulo { esq: l + 1, topo: b, dir: d, base: b + 1 }, k, k);
    q.preencher(Retangulo { esq: l, topo: t + 1, dir: l + 1, base: b }, k, k);
    q.preencher(Retangulo { esq: d, topo: t + 1, dir: d + 1, base: b }, k, k);
    let e = if padrao { 2 } else { 1 };
    if padrao {
        q.contorno(Retangulo { esq: l + 1, topo: t + 1, dir: d - 1, base: b - 1 }, k);
    }
    relevo(q, c, Retangulo { esq: l + e, topo: t + e, dir: d + 1 - e, base: b + 1 - e }, 2);
    if let Some(f) = f {
        let w = largura(f, texto);
        let x = (i32::from(r.esq) + i32::from(r.dir) - w - 1) / 2;
        let y = (i32::from(r.topo) + i32::from(r.base) - f.altura() as i32) / 2;
        escrever(q, f, x, y, texto, c.texto);
    }
}

/// Caixa branca com contorno preto (campo, lista, combinação).
fn caixa_branca(q: &mut Quadro, c: &Cores, r: Retangulo) {
    q.preencher(r, c.campo, c.campo);
    q.contorno(Retangulo { dir: r.dir - 1, base: r.base - 1, ..r }, c.linha);
}

/// Seta (triângulo de 7 colunas e haste de 3, nas linhas 4 a 10 do botão `r`), para cima ou para
/// baixo, com a barra embaixo (caixa de combinação). Desabilitada: cinza escuro com o relevo branco
/// deslocado (+1, +1), como as setas da barra de rolagem de uma lista que cabe inteira.
fn seta(q: &mut Quadro, c: &Cores, r: Retangulo, para_baixo: bool, barra: bool, desabilitada: bool) {
    let cx = (i32::from(r.esq) + i32::from(r.dir)) / 2;
    let topo = i32::from(r.topo) + 4;
    let forma = [(0, 0), (1, 1), (2, 2), (3, 3), (4, 1), (5, 1), (6, 1)];
    let mut desenhar = |d: i32, cor: u8| {
        for (dy, meia) in forma {
            let y = if para_baixo { topo + 6 - dy } else { topo + dy };
            q.linha(cx - meia + d, y + d, cx + meia + d, y + d, cor);
        }
        if barra {
            q.linha(cx - 3 + d, topo + 8 + d, cx + 3 + d, topo + 8 + d, cor);
        }
    };
    if desabilitada {
        desenhar(1, c.claro);
        desenhar(0, c.escuro);
    } else {
        desenhar(0, c.linha);
    }
}

/// Partes da barra de rolagem vertical à direita de `r` (contorno incluído): os botões das setas e
/// a linha de cima do polegar, que anda de `cima.base` (posição 0) a `baixo.topo − 1 − (lado + 1)`.
struct PartesRolagem {
    x: i16,
    cima: Retangulo,
    baixo: Retangulo,
}

impl PartesRolagem {
    fn de(r: Retangulo, largura: i16) -> PartesRolagem {
        let x = r.dir - largura;
        let lado = largura - 2;
        let cima = Retangulo { esq: x + 1, topo: r.topo + 1, dir: r.dir - 1, base: r.topo + 1 + lado };
        let baixo = Retangulo { topo: r.base - 1 - lado, base: r.base - 1, ..cima };
        PartesRolagem { x, cima, baixo }
    }

    fn lado(&self) -> i16 {
        self.cima.base - self.cima.topo
    }

    /// Linha de cima do polegar na posição `pos` de `0..=max`: o curso inteiro vezes `pos / max`,
    /// arredondado (bancada, 18 arquivos: posições 2, 8 e 9 de 10 em 11, 46 e 51 linhas).
    fn linha_polegar(&self, pos: usize, max: usize) -> i16 {
        let curso = i64::from(self.baixo.topo - 1 - self.cima.base - (self.lado() + 1));
        let (p, m) = (pos.min(max) as i64, max.max(1) as i64);
        self.cima.base + ((2 * curso * p + m) / (2 * m)) as i16
    }

    /// Clique na barra na linha `y`: setas, um item; trilho acima ou abaixo do polegar, uma página
    /// (os visíveis − 1; bancada: de 2 para 9 com 8 visíveis). Devolve o primeiro item novo.
    fn rolar(&self, pos: usize, max: usize, visiveis: usize, y: i32) -> usize {
        let linha = i32::from(self.linha_polegar(pos, max));
        let pagina = visiveis.saturating_sub(1).max(1);
        let novo = if y < i32::from(self.cima.base) {
            pos.saturating_sub(1)
        } else if y >= i32::from(self.baixo.topo) {
            pos + 1
        } else if y < linha {
            pos.saturating_sub(pagina)
        } else if y > linha + i32::from(self.lado()) + 1 {
            pos + pagina
        } else {
            pos
        };
        novo.min(max)
    }
}

/// Barra de rolagem vertical à direita de `r` (contorno incluído). `barra` = (posição, máximo) com
/// máximo > 0 quando a lista não cabe: trilho cinza, setas pretas e o polegar (um botão quadrado
/// entre duas linhas pretas); sem ela, trilho branco, sem polegar e setas desabilitadas [bancada,
/// diálogo de arquivo com 18 arquivos, 2026-10-19].
fn rolagem(q: &mut Quadro, c: &Cores, r: Retangulo, largura: i16, barra: Option<(usize, usize)>) {
    let p = PartesRolagem::de(r, largura);
    let (x, cima, baixo) = (p.x, p.cima, p.baixo);
    let barra = barra.filter(|&(_, max)| max > 0);
    q.preencher(Retangulo { esq: x, dir: x + 1, ..r }, c.linha, c.linha);
    relevo(q, c, cima, 1);
    seta(q, c, cima, false, false, barra.is_none());
    q.linha(i32::from(x), i32::from(cima.base), i32::from(r.dir) - 1, i32::from(cima.base), c.linha);
    relevo(q, c, baixo, 1);
    seta(q, c, baixo, true, false, barra.is_none());
    q.linha(i32::from(x), i32::from(baixo.topo) - 1, i32::from(r.dir) - 1, i32::from(baixo.topo) - 1, c.linha);
    if let Some((pos, max)) = barra {
        let trilho = Retangulo { topo: cima.base + 1, base: baixo.topo - 1, ..cima };
        q.preencher(trilho, c.botao, c.botao);
        let y = p.linha_polegar(pos, max);
        let lado = p.lado();
        for l in [y, y + lado + 1] {
            q.linha(i32::from(x), i32::from(l), i32::from(r.dir) - 1, i32::from(l), c.linha);
        }
        relevo(q, c, Retangulo { topo: y + 1, base: y + 1 + lado, ..cima }, 1);
    }
}

/// Ícone próprio da caixa de mensagem (círculo com "?" ou "i"; o do Windows não é reproduzido).
fn icone(q: &mut Quadro, f: Option<&Fonte>, c: &Cores, x: i32, y: i32, letra: &str) {
    // Círculo de 32 pixels: dentro se a distância (em meios pixels) ao centro for até 31,8; a borda
    // é o pixel de dentro com um vizinho de fora.
    let dentro = |i: i32, j: i32| {
        let (dx, dy) = (2 * i - 31, 2 * j - 31);
        dx * dx + dy * dy <= 1010
    };
    for j in 0..32 {
        for i in 0..32 {
            if dentro(i, j) {
                let borda = !(dentro(i - 1, j) && dentro(i + 1, j) && dentro(i, j - 1) && dentro(i, j + 1));
                let cor = if letra == "i" { c.icone_informacao } else { c.icone };
                q.ponto(x + i, y + j, if borda { c.linha } else { cor });
            }
        }
    }
    if let Some(f) = f {
        let img = f.imagem(&bytes(letra));
        let (ox, oy) = (x + 16 - img.largura as i32, y + 16 - f.altura() as i32);
        for j in 0..img.altura {
            for i in 0..img.largura {
                if img.bits[j * img.largura + i] {
                    for (a, b) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        q.ponto(ox + 2 * i as i32 + a, oy + 2 * j as i32 + b, c.claro);
                    }
                }
            }
        }
    }
}

// --- caixa de mensagem ---------------------------------------------------------------------------

/// Ícone da caixa de mensagem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconeMensagem {
    Pergunta,
    Informacao,
}

struct GeometriaCaixa {
    externo: Retangulo,
    interior: Retangulo,
    titulo: Retangulo,
    icone: (i16, i16),
    texto: (i16, i16),
    botoes: Vec<Retangulo>,
    moldura: bool,
}

/// `MessageBox`: título, linhas de texto, ícone e botões (o padrão realçado).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaixaMensagem {
    pub titulo: String,
    pub linhas: Vec<String>,
    pub icone: IconeMensagem,
    pub botoes: Vec<String>,
    pub padrao: usize,
    /// Índice em `estilos`.
    pub estilo: usize,
}

impl CaixaMensagem {
    /// "SimAnt Load / Do you want to save the current ant hill?" com Yes, No e Cancel.
    pub fn pergunta_load(cfg: &ConfigDialogos) -> CaixaMensagem {
        let m = &cfg.caixa_mensagem;
        CaixaMensagem {
            titulo: m.titulo_load.clone(),
            linhas: vec![m.pergunta_load.clone()],
            icone: IconeMensagem::Pergunta,
            botoes: vec![m.sim.clone(), m.nao.clone(), m.cancelar.clone()],
            padrao: m.padrao_load,
            estilo: 0,
        }
    }

    /// `_MenuQuit` (`1000:d628`): "SimAnt Quit / Do you want to save the current ant hill?" com Yes, No
    /// e Cancel, No como padrão (`MB_DEFBUTTON2`), como a do Load [bancada: m19].
    pub fn pergunta_sair(cfg: &ConfigDialogos) -> CaixaMensagem {
        let m = &cfg.caixa_mensagem;
        CaixaMensagem { titulo: m.titulo_quit.clone().unwrap_or_else(|| m.titulo_load.clone()), ..CaixaMensagem::pergunta_load(cfg) }
    }

    /// `_PopMsg("%s\nSaved correctly")`: título "Message", ícone de informação e OK; `arquivo` é o
    /// caminho completo, e o `\n` do formato quebra a linha.
    pub fn salvo(cfg: &ConfigDialogos, arquivo: &str) -> CaixaMensagem {
        let m = &cfg.caixa_mensagem;
        CaixaMensagem {
            titulo: m.titulo_mensagem.clone(),
            linhas: m.salvo.replace("%s", arquivo).split('\n').map(str::to_owned).collect(),
            icone: IconeMensagem::Informacao,
            botoes: vec![m.ok.clone()],
            padrao: 0,
            estilo: 1,
        }
    }

/// Geometria calculada: externo, interior, título, canto do ícone, y do texto e botões.
    fn geometria(&self, cfg: &ConfigCaixaMensagem, f: Option<&Fonte>, interface: &Interface) -> GeometriaCaixa {
        self.geometria_com(cfg, f, |w, h| interface.centrar_dialogo(w, h))
    }

    /// Como `geometria`, com o canto da caixa (largura, altura → x, y) dado por `canto`.
    fn geometria_com(&self, cfg: &ConfigCaixaMensagem, f: Option<&Fonte>, canto: impl Fn(i16, i16) -> (i16, i16)) -> GeometriaCaixa {
        let est = cfg.estilos.get(self.estilo).or(cfg.estilos.first()).cloned().unwrap_or(EstiloCaixa { borda: [1; 4], margem_titulo: 0, centro: None });
        let [be, bt, bd, bb] = est.borda;
        let texto = self.linhas.iter().map(|l| f.map_or(8 * l.len() as i32, |f| largura(f, l))).max().unwrap_or(0) as i16;
        let (_, grupo, xs) =
            cfg.grupos_botoes.iter().find(|g| g.0 == self.botoes.len()).cloned().unwrap_or((1, cfg.botao[0], vec![0]));
        let n = self.linhas.len().max(1) as i16;
        // Alturas relativas ao topo do interior.
        let linha_titulo = est.margem_titulo + cfg.altura_titulo;
        let icone_y = linha_titulo + cfg.icone_abaixo_titulo;
        let alto_texto = n * cfg.altura_linha;
        let texto_y = icone_y + ((cfg.lado_icone - alto_texto) / 2).max(0);
        let botoes_y = (icone_y + cfg.lado_icone).max(texto_y + alto_texto) + cfg.folga_botoes;
        let h_interior = botoes_y + cfg.botao[1] + cfg.margem_baixo;
        let w_interior = (cfg.texto_x + texto + cfg.margem_direita).max(grupo + 2 * cfg.texto_x);
        let (w, h) = (be + w_interior + bd, bt + h_interior + bb);
        let (x, y) = canto(w, h);
        let externo = Retangulo { esq: x, topo: y, dir: x + w, base: y + h };
        let interior = Retangulo { esq: x + be, topo: y + bt, dir: x + w - bd, base: y + h - bb };
        let m = est.margem_titulo;
        let titulo = Retangulo { esq: interior.esq + m, topo: interior.topo + m, dir: interior.dir - m, base: interior.topo + linha_titulo };
        let x0 = interior.esq + (w_interior - grupo + 1) / 2;
        let yb = interior.topo + botoes_y;
        let botoes = xs.iter().map(|&dx| Retangulo { esq: x0 + dx, topo: yb, dir: x0 + dx + cfg.botao[0], base: yb + cfg.botao[1] }).collect();
        GeometriaCaixa {
            externo,
            interior,
            titulo,
            icone: (interior.esq + cfg.icone_x, interior.topo + icone_y),
            texto: (interior.esq + cfg.texto_x, interior.topo + texto_y),
            botoes,
            moldura: be > 1,
        }
    }

    pub fn desenhar(&self, q: &mut Quadro, interface: &Interface, cfg: &ConfigDialogos) {
        let m = &cfg.caixa_mensagem;
        let f = fonte(&m.fontes, interface);
        let c = &cfg.cores;
        let g = self.geometria(m, f, interface);
        let e = g.externo;
        q.preencher(e, if g.moldura { c.moldura } else { c.fundo }, if g.moldura { c.moldura } else { c.fundo });
        q.contorno(Retangulo { dir: e.dir - 1, base: e.base - 1, ..e }, c.linha);
        q.preencher(g.interior, c.fundo, c.fundo);
        titulo(q, fonte(&cfg.fontes_titulo, interface), c, g.titulo, &self.titulo);
        let letra = if self.icone == IconeMensagem::Pergunta { "?" } else { "i" };
        icone(q, f, c, i32::from(g.icone.0), i32::from(g.icone.1), letra);
        if let Some(f) = f {
            for (i, l) in self.linhas.iter().enumerate() {
                let y = i32::from(g.texto.1) + i as i32 * i32::from(m.altura_linha);
                escrever(q, f, i32::from(g.texto.0), y, l, c.texto);
            }
        }
        let botoes = g.botoes;
        for (i, (r, b)) in botoes.iter().zip(&self.botoes).enumerate() {
            botao(q, f, c, *r, b, i == self.padrao);
        }
    }

    /// Contorno externo da caixa na área cliente.
    pub fn externo(&self, interface: &Interface, cfg: &ConfigDialogos) -> Retangulo {
        self.geometria(&cfg.caixa_mensagem, fonte(&cfg.caixa_mensagem.fontes, interface), interface).externo
    }

    /// Botão sob (x, y).
    pub fn botao_em(&self, interface: &Interface, cfg: &ConfigDialogos, x: i32, y: i32) -> Option<usize> {
        let g = self.geometria(&cfg.caixa_mensagem, fonte(&cfg.caixa_mensagem.fontes, interface), interface);
        g.botoes.iter().position(|r| contem(*r, x, y))
    }

    /// Tecla: Enter = botão padrão, Esc = o último (Cancel), letra sublinhada = o botão dela.
    pub fn tecla(&self, nome: &str) -> Option<usize> {
        match nome {
            "Return" | "Keypad Enter" => Some(self.padrao),
            "Escape" => self.botoes.len().checked_sub(1),
            _ => self.botoes.iter().position(|b| {
                b.find('&').and_then(|p| b[p + 1..].chars().next()).is_some_and(|l| l.to_string().eq_ignore_ascii_case(nome))
            }),
        }
    }
}

// --- diálogo de arquivo --------------------------------------------------------------------------

/// O que o diálogo de arquivo decidiu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RespostaArquivo {
    Nada,
    /// OK com um nome: o arquivo escolhido.
    Escolher(PathBuf),
    Cancelar,
}

/// Um item da árvore "Directories:": os ancestrais da pasta (abertos) e as subpastas dela.
#[derive(Clone, Debug)]
struct ItemPasta {
    nivel: usize,
    nome: String,
    aberta: bool,
    caminho: PathBuf,
}

/// Controle com o foco do teclado, na ordem do Tab do diálogo comum do Windows 3.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Foco {
    Campo,
    Lista,
    Pastas,
    Tipo,
    Unidades,
    Ok,
    Cancelar,
    SomenteLeitura,
}

const ORDEM_FOCO: [Foco; 8] = [Foco::Campo, Foco::Lista, Foco::Pastas, Foco::Tipo, Foco::Unidades, Foco::Ok, Foco::Cancelar, Foco::SomenteLeitura];

/// Lista suspensa (combo box `CBS_DROPDOWNLIST`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combo {
    /// "List Files of Type:" / "Save File as Type:".
    Tipo,
    /// "Drives:".
    Unidades,
}

/// Lista suspensa aberta: o item realçado e o primeiro visível.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComboAberto {
    pub qual: Combo,
    pub realce: usize,
    pub primeiro: usize,
}

/// Tipo de unidade (`GetDriveType`): decide o ícone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoUnidade {
    Disquete,
    Disco,
}

/// Uma unidade da lista "Drives:".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unidade {
    pub letra: char,
    pub tipo: TipoUnidade,
}

/// As unidades do sistema, na ordem das letras (Windows: `GetLogicalDrives` e `GetDriveTypeW`).
/// Fora do Windows, nenhuma (a caixa mostra só a unidade do caminho, ou o texto padrão).
pub fn unidades_do_sistema() -> Vec<Unidade> {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetLogicalDrives() -> u32;
            fn GetDriveTypeW(raiz: *const u16) -> u32;
        }
        // SAFETY: funções do kernel32 sem estado; a raiz é uma cadeia UTF-16 terminada em zero.
        let mascara = unsafe { GetLogicalDrives() };
        (0..26u32)
            .filter(|i| mascara & (1 << i) != 0)
            .map(|i| {
                let letra = char::from(b'a' + i as u8);
                let raiz: Vec<u16> = format!("{letra}:\\").encode_utf16().chain([0]).collect();
                // SAFETY: `raiz` vive até o fim da chamada.
                let tipo = unsafe { GetDriveTypeW(raiz.as_ptr()) };
                // 2 = DRIVE_REMOVABLE; as outras (fixa, rede, CD, RAM) com o ícone de disco
                // [hipótese: só o disquete e o disco rígido foram vistos na bancada].
                Unidade { letra, tipo: if tipo == 2 { TipoUnidade::Disquete } else { TipoUnidade::Disco } }
            })
            .collect()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Qual lista com barra de rolagem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QualLista {
    Arquivos,
    Pastas,
    Combo,
}

/// Arrasto (ou botão segurado) em andamento no diálogo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArrastoDialogo {
    /// Título: o contorno pontilhado segue o cursor; o diálogo vai ao soltar (bancada g19, g20).
    Titulo { inicio: (i32, i32), atual: (i32, i32) },
    /// Polegar de uma barra: `pega` = distância do cursor à linha de cima do polegar.
    Polegar { lista: QualLista, pega: i32 },
    /// Seta ou trilho segurados: repete (`y` na barra, em coordenadas do diálogo).
    Repetir { lista: QualLista, y: i32, espera: u32 },
    /// Seleção de texto com o mouse no campo do nome.
    Selecao,
}

/// "Load Ant Hill" / "Save Ant Hill As" (`_FileSelect`, diálogo comum do Windows).
#[derive(Clone, Debug)]
pub struct DialogoArquivo {
    pub salvar: bool,
    pub pasta: PathBuf,
    /// Campo "File Name:" (com cursor e seleção).
    pub campo: CampoTexto,
    pub arquivos: Vec<String>,
    pub selecionado: Option<usize>,
    pub primeiro: usize,
    /// Primeiro item visível da árvore de pastas.
    pub primeira_pasta: usize,
    /// Item realçado na árvore de pastas (o clique; depois do clique duplo, a pasta nova).
    pub pasta_realcada: Option<usize>,
    /// Controle com o foco.
    pub foco: Foco,
    /// "Read Only" marcado. O SimAnt não lê a marca (`OFN_READONLY`) depois do diálogo: o Load
    /// carrega e o Save grava como sem ela (bancada g21; `_FileSelect` só usa o nome).
    pub somente_leitura: bool,
    /// Deslocamento do diálogo arrastado pelo título (o lugar medido é o do `externo`).
    pub deslocamento: (i16, i16),
    /// Lista suspensa aberta.
    pub combo: Option<ComboAberto>,
    /// Unidades do sistema (a da pasta entra se faltar) e a escolhida.
    pub unidades: Vec<Unidade>,
    pub unidade: usize,
    arrasto: Option<ArrastoDialogo>,
}

/// Linha de cima do polegar → posição (o inverso de `PartesRolagem::linha_polegar`).
fn posicao_do_polegar(p: &PartesRolagem, linha: i32, max: usize) -> usize {
    let curso = i32::from(p.baixo.topo - 1 - p.cima.base - (p.lado() + 1));
    if curso <= 0 || max == 0 {
        return 0;
    }
    let d = (linha - i32::from(p.cima.base)).clamp(0, curso) as i64;
    (((2 * d * max as i64) + i64::from(curso)) / (2 * i64::from(curso))).clamp(0, max as i64) as usize
}

impl DialogoArquivo {
    pub fn new(salvar: bool, pasta: &Path, cfg: &ConfigDialogos) -> DialogoArquivo {
        DialogoArquivo::com_unidades(salvar, pasta, cfg, unidades_do_sistema())
    }

    /// Como [`DialogoArquivo::new`], com a lista de unidades dada (testes).
    pub fn com_unidades(salvar: bool, pasta: &Path, cfg: &ConfigDialogos, unidades: Vec<Unidade>) -> DialogoArquivo {
        let mut d = DialogoArquivo {
            salvar,
            pasta: pasta.to_path_buf(),
            campo: CampoTexto::new("", cfg.arquivo.controles.limite_nome),
            arquivos: Vec::new(),
            selecionado: None,
            primeiro: 0,
            primeira_pasta: 0,
            pasta_realcada: None,
            foco: Foco::Campo,
            somente_leitura: false,
            deslocamento: (0, 0),
            combo: None,
            unidades,
            unidade: 0,
            arrasto: None,
        };
        d.trocar_pasta(pasta, cfg);
        d
    }

    /// O foco está no campo (o texto inicial aparece selecionado e o cursor pisca nele).
    pub fn foco_no_campo(&self) -> bool {
        self.foco == Foco::Campo
    }

    /// Lê os `.ANT` da pasta e volta a lista ao começo, com o campo no texto inicial (`*.ant`)
    /// selecionado.
    fn trocar_pasta(&mut self, pasta: &Path, cfg: &ConfigDialogos) {
        self.pasta = pasta.to_path_buf();
        self.arquivos = arquivos_ant(pasta)
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
            .collect();
        self.campo.definir(&cfg.arquivo.campo_inicial, true);
        self.selecionado = None;
        self.primeiro = 0;
        self.primeira_pasta = 0;
        // A unidade da pasta na lista "Drives:".
        if let Some(l) = self.letra_da_pasta() {
            match self.unidades.iter().position(|u| u.letra == l) {
                Some(i) => self.unidade = i,
                None => {
                    self.unidades.push(Unidade { letra: l, tipo: TipoUnidade::Disco });
                    self.unidades.sort_by_key(|u| u.letra);
                    self.unidade = self.unidades.iter().position(|u| u.letra == l).unwrap_or(0);
                }
            }
        }
    }

    fn letra_da_pasta(&self) -> Option<char> {
        match self.caminho_absoluto().components().next() {
            Some(std::path::Component::Prefix(p)) => p.as_os_str().to_string_lossy().chars().next().map(|c| c.to_ascii_lowercase()),
            _ => None,
        }
    }

    fn linhas_visiveis(&self, cfg: &ConfigDialogoArquivo) -> usize {
        ((cfg.lista[3] - 2) / cfg.altura_item).max(1) as usize
    }

    fn pastas_visiveis(&self, cfg: &ConfigDialogoArquivo) -> usize {
        ((cfg.pastas[3] - 2) / cfg.altura_item_pasta).max(1) as usize
    }

    /// Maior primeiro item visível (0 quando a lista cabe).
    fn primeiro_maximo(&self, cfg: &ConfigDialogoArquivo) -> usize {
        self.arquivos.len().saturating_sub(self.linhas_visiveis(cfg))
    }

    /// Pasta que o texto do campo nomeia ("a:", "c:\", "..", um caminho), se existir.
    fn pasta_do_campo(&self) -> Option<PathBuf> {
        let t = self.campo.texto.trim();
        if t.is_empty() || t.contains('*') || t.contains('?') {
            return None;
        }
        // "a:" sozinho é a pasta corrente da unidade (no Windows, a do processo para ela).
        let p = if t.len() == 2 && t.ends_with(':') {
            std::path::absolute(t).unwrap_or_else(|_| PathBuf::from(format!("{t}\\")))
        } else {
            self.pasta.join(t)
        };
        p.is_dir().then(|| std::path::absolute(&p).unwrap_or(p))
    }

    /// OK: uma pasta no campo troca de pasta; senão o nome vira caminho (com curinga, nada).
    fn ok(&mut self, cfg: &ConfigDialogos) -> RespostaArquivo {
        if let Some(p) = self.pasta_do_campo() {
            self.trocar_pasta(&p, cfg);
            self.pasta_realcada = None;
            self.foco = Foco::Campo;
            return RespostaArquivo::Nada;
        }
        if self.campo.texto.contains('*') || self.campo.texto.contains('?') {
            return RespostaArquivo::Nada;
        }
        match caminho_do_nome(&self.pasta, &self.campo.texto) {
            Some(p) => RespostaArquivo::Escolher(p),
            None => RespostaArquivo::Nada,
        }
    }

    /// Abre a pasta do item `k` da árvore (duplo clique, Enter com o foco na árvore).
    fn abrir_pasta(&mut self, k: usize, cfg: &ConfigDialogos) {
        let Some(item) = self.pastas().get(k).cloned() else { return };
        self.trocar_pasta(&item.caminho, cfg);
        self.campo.definir(&cfg.arquivo.campo_inicial, false);
        // A pasta aberta fica realçada: é o último ancestral da lista nova.
        self.pasta_realcada = self.pastas().iter().rposition(|p| p.aberta);
    }

    /// Escolhe a unidade `i` da lista "Drives:": vai à pasta corrente dela. Uma unidade sem disco
    /// (ou sem acesso) não troca [no original, uma caixa de erro do Windows].
    fn escolher_unidade(&mut self, i: usize, cfg: &ConfigDialogos) {
        let Some(u) = self.unidades.get(i).cloned() else { return };
        if i == self.unidade {
            return;
        }
        let destino = std::path::absolute(format!("{}:", u.letra)).unwrap_or_else(|_| PathBuf::from(format!("{}:\\", u.letra)));
        if destino.is_dir() {
            self.trocar_pasta(&destino, cfg);
            self.pasta_realcada = None;
        }
    }

    fn itens_do_combo(&self, qual: Combo, cfg: &ConfigDialogoArquivo) -> usize {
        let _ = cfg;
        match qual {
            Combo::Tipo => 1,
            Combo::Unidades => self.unidades.len().max(1),
        }
    }

    fn visiveis_do_combo(qual: Combo, cfg: &ConfigDialogoArquivo) -> (usize, i16) {
        let c = &cfg.controles;
        match qual {
            Combo::Tipo => (c.tipo_visiveis.max(1), c.tipo_altura_item),
            Combo::Unidades => (c.unidades_visiveis.max(1), c.unidades_altura_item),
        }
    }

    fn caixa_do_combo(qual: Combo, cfg: &ConfigDialogoArquivo) -> Retangulo {
        ret(match qual {
            Combo::Tipo => cfg.tipo,
            Combo::Unidades => cfg.unidades,
        })
    }

    /// A lista aberta: da linha de baixo da caixa até os itens visíveis e o contorno (g01, g03).
    fn lista_do_combo(qual: Combo, cfg: &ConfigDialogoArquivo) -> Retangulo {
        let r = DialogoArquivo::caixa_do_combo(qual, cfg);
        let (n, h) = DialogoArquivo::visiveis_do_combo(qual, cfg);
        Retangulo { topo: r.base - 1, base: r.base - 1 + n as i16 * h + 2, ..r }
    }

    fn selecao_do_combo(&self, qual: Combo) -> usize {
        match qual {
            Combo::Tipo => 0,
            Combo::Unidades => self.unidade,
        }
    }

    fn abrir_combo(&mut self, qual: Combo, cfg: &ConfigDialogoArquivo) {
        let realce = self.selecao_do_combo(qual);
        let (n, _) = DialogoArquivo::visiveis_do_combo(qual, cfg);
        let total = self.itens_do_combo(qual, cfg);
        let primeiro = realce.saturating_sub(n.saturating_sub(1)).min(total.saturating_sub(n));
        self.combo = Some(ComboAberto { qual, realce, primeiro });
        self.foco = match qual {
            Combo::Tipo => Foco::Tipo,
            Combo::Unidades => Foco::Unidades,
        };
    }

    fn escolher_no_combo(&mut self, qual: Combo, i: usize, cfg: &ConfigDialogos) {
        self.combo = None;
        if qual == Combo::Unidades {
            self.escolher_unidade(i, cfg);
        }
    }

    /// Ponto da tela → coordenadas do diálogo no lugar medido.
    fn local(&self, x: i32, y: i32) -> (i32, i32) {
        (x - i32::from(self.deslocamento.0), y - i32::from(self.deslocamento.1))
    }

    fn titulo(cfg: &ConfigDialogoArquivo) -> Retangulo {
        let e = ret(cfg.externo);
        let [me, mt, md, _] = cfg.moldura;
        Retangulo { esq: e.esq + me + 1, topo: e.topo + mt + 1, dir: e.dir - md - 1, base: e.topo + mt + 1 + cfg.altura_titulo }
    }

    /// Barra de rolagem da lista e (posição, máximo, visíveis).
    fn barra_da_lista(&self, lista: QualLista, cfg: &ConfigDialogoArquivo) -> Option<(Retangulo, PartesRolagem, usize, usize, usize)> {
        let (r, pos, max, vis) = match lista {
            QualLista::Arquivos => (ret(cfg.lista), self.primeiro, self.primeiro_maximo(cfg), self.linhas_visiveis(cfg)),
            QualLista::Pastas => {
                let vis = self.pastas_visiveis(cfg);
                (ret(cfg.pastas), self.primeira_pasta, self.pastas().len().saturating_sub(vis), vis)
            }
            QualLista::Combo => {
                let c = self.combo?;
                let (vis, _) = DialogoArquivo::visiveis_do_combo(c.qual, cfg);
                let r = DialogoArquivo::lista_do_combo(c.qual, cfg);
                (r, c.primeiro, self.itens_do_combo(c.qual, cfg).saturating_sub(vis), vis)
            }
        };
        let barra = Retangulo { esq: r.dir - cfg.largura_rolagem, ..r };
        Some((barra, PartesRolagem::de(r, cfg.largura_rolagem), pos, max, vis))
    }

    fn definir_primeiro(&mut self, lista: QualLista, v: usize) {
        match lista {
            QualLista::Arquivos => self.primeiro = v,
            QualLista::Pastas => self.primeira_pasta = v,
            QualLista::Combo => {
                if let Some(c) = self.combo.as_mut() {
                    c.primeiro = v;
                }
            }
        }
    }

    /// Rola a lista `n` linhas (negativo = para cima), dentro dos limites.
    fn rolar_lista(&mut self, lista: QualLista, n: i32, cfg: &ConfigDialogoArquivo) {
        let Some((_, _, pos, max, _)) = self.barra_da_lista(lista, cfg) else { return };
        let novo = (pos as i32 + n).clamp(0, max as i32) as usize;
        self.definir_primeiro(lista, novo);
    }

    /// Clique na barra da lista em `y` (coordenadas do diálogo): seta, trilho ou polegar.
    fn clicar_barra(&mut self, lista: QualLista, y: i32, cfg: &ConfigDialogoArquivo) {
        let Some((_, p, pos, max, vis)) = self.barra_da_lista(lista, cfg) else { return };
        if max == 0 {
            return;
        }
        let linha = i32::from(p.linha_polegar(pos, max));
        let no_polegar = y >= linha && y <= linha + i32::from(p.lado()) + 1 && y >= i32::from(p.cima.base) && y < i32::from(p.baixo.topo);
        if no_polegar {
            self.arrasto = Some(ArrastoDialogo::Polegar { lista, pega: y - linha });
            return;
        }
        self.definir_primeiro(lista, p.rolar(pos, max, vis, y));
        self.arrasto = Some(ArrastoDialogo::Repetir { lista, y, espera: cfg.controles.repeticao_ms[0] });
    }

    /// Lista com barra sob o ponto (coordenadas do diálogo), se o ponto está na barra dela.
    fn barra_em(&self, x: i32, y: i32, cfg: &ConfigDialogoArquivo) -> Option<QualLista> {
        [QualLista::Combo, QualLista::Arquivos, QualLista::Pastas]
            .into_iter()
            .find(|&l| self.barra_da_lista(l, cfg).is_some_and(|(b, ..)| contem(b, x, y)))
    }

    /// Lista sob o ponto (para a roda do mouse).
    fn lista_em(&self, x: i32, y: i32, cfg: &ConfigDialogoArquivo) -> Option<QualLista> {
        if let Some(c) = self.combo {
            if contem(DialogoArquivo::lista_do_combo(c.qual, cfg), x, y) {
                return Some(QualLista::Combo);
            }
        }
        if contem(ret(cfg.lista), x, y) {
            return Some(QualLista::Arquivos);
        }
        contem(ret(cfg.pastas), x, y).then_some(QualLista::Pastas)
    }

    /// Campo: a posição de texto sob x (coordenadas do diálogo).
    fn posicao_no_campo(&self, x: i32, cfg: &ConfigDialogos, interface: Option<&Interface>) -> usize {
        let a = &cfg.arquivo;
        let Some(f) = interface.and_then(|i| fonte(&a.fontes, i)) else { return self.campo.texto.len() };
        let r = ret(a.campo);
        let larg = |s: &str| largura(f, s);
        let ini = self.campo.primeiro_visivel(i32::from(r.dir - r.esq) - 6, larg);
        let dx = x - (i32::from(r.esq) + 3) + larg(&self.campo.texto[..ini]);
        self.campo.posicao_em(dx, larg)
    }

    /// Clique em (x, y) da área cliente; `duplo` = segundo clique de um duplo; `shift`: Shift
    /// apertado (estende a seleção do campo).
    pub fn clicar(&mut self, cfg: &ConfigDialogos, interface: Option<&Interface>, x: i32, y: i32, duplo: bool, shift: bool) -> RespostaArquivo {
        let a = &cfg.arquivo;
        let (x, y) = self.local(x, y);
        // Lista suspensa aberta: o clique é dela (fora, ela fecha e o clique não segue).
        if let Some(c) = self.combo {
            let lista = DialogoArquivo::lista_do_combo(c.qual, a);
            if contem(lista, x, y) {
                if let Some(QualLista::Combo) = self.barra_em(x, y, a) {
                    self.clicar_barra(QualLista::Combo, y, a);
                    return RespostaArquivo::Nada;
                }
                let (_, h) = DialogoArquivo::visiveis_do_combo(c.qual, a);
                let k = ((y - i32::from(lista.topo) - 1) / i32::from(h)).max(0) as usize + c.primeiro;
                if k < self.itens_do_combo(c.qual, a) {
                    self.escolher_no_combo(c.qual, k, cfg);
                }
                return RespostaArquivo::Nada;
            }
            self.combo = None;
            if contem(DialogoArquivo::caixa_do_combo(c.qual, a), x, y) {
                return RespostaArquivo::Nada;
            }
        }
        let t = DialogoArquivo::titulo(a);
        if contem(t, x, y) {
            let caixa = Retangulo { dir: t.esq + (t.base - t.topo), ..t };
            if contem(caixa, x, y) {
                // Duplo clique na caixa do menu de sistema: Close (= Cancel). O menu não abre.
                return if duplo { RespostaArquivo::Cancelar } else { RespostaArquivo::Nada };
            }
            self.arrasto = Some(ArrastoDialogo::Titulo { inicio: (x, y), atual: (x, y) });
            return RespostaArquivo::Nada;
        }
        if contem(ret(a.ok), x, y) {
            self.foco = Foco::Ok;
            return self.ok(cfg);
        }
        if contem(ret(a.cancelar), x, y) {
            return RespostaArquivo::Cancelar;
        }
        let (texto_sl, [sx, sy]) = &a.somente_leitura;
        let largura_sl = interface.and_then(|i| fonte(&a.fontes, i)).map_or(80, |f| largura(f, texto_sl) as i16 + 20);
        if contem(Retangulo { esq: *sx, topo: *sy - 2, dir: sx + 18 + largura_sl, base: sy + 15 }, x, y) {
            self.somente_leitura = !self.somente_leitura;
            self.foco = Foco::SomenteLeitura;
            return RespostaArquivo::Nada;
        }
        for qual in [Combo::Tipo, Combo::Unidades] {
            if contem(DialogoArquivo::caixa_do_combo(qual, a), x, y) {
                self.abrir_combo(qual, a);
                return RespostaArquivo::Nada;
            }
        }
        if let Some(l) = self.barra_em(x, y, a) {
            self.clicar_barra(l, y, a);
            return RespostaArquivo::Nada;
        }
        if contem(ret(a.campo), x, y) {
            let i = self.posicao_no_campo(x, cfg, interface);
            if duplo {
                self.campo.selecionar_palavra(i);
            } else {
                self.campo.mover(i, shift);
                self.arrasto = Some(ArrastoDialogo::Selecao);
            }
            self.foco = Foco::Campo;
            return RespostaArquivo::Nada;
        }
        let lista = ret(a.lista);
        if contem(lista, x, y) {
            self.foco = Foco::Lista;
            let k = ((y - i32::from(lista.topo) - 1) / i32::from(a.altura_item)).max(0) as usize;
            let i = k + self.primeiro;
            if i < self.arquivos.len() && k < self.linhas_visiveis(a) {
                self.selecionar_arquivo(i, a);
                if duplo {
                    return self.ok(cfg);
                }
            }
            return RespostaArquivo::Nada;
        }
        let pastas = ret(a.pastas);
        if contem(pastas, x, y) {
            self.foco = Foco::Pastas;
            let v = ((y - i32::from(pastas.topo) - 1) / i32::from(a.altura_item_pasta)).max(0) as usize;
            let k = v + self.primeira_pasta;
            if k < self.pastas().len() && v < self.pastas_visiveis(a) {
                self.pasta_realcada = Some(k);
                if duplo {
                    self.abrir_pasta(k, cfg);
                }
            }
        }
        RespostaArquivo::Nada
    }

    /// Escolhe o arquivo `i` da lista: o nome vai para o campo (sem seleção) e o item fica à vista.
    fn selecionar_arquivo(&mut self, i: usize, a: &ConfigDialogoArquivo) {
        let Some(nome) = self.arquivos.get(i).cloned() else { return };
        self.selecionado = Some(i);
        self.campo.definir(&nome, false);
        let vis = self.linhas_visiveis(a);
        if i < self.primeiro {
            self.primeiro = i;
        } else if i >= self.primeiro + vis {
            self.primeiro = i + 1 - vis;
        }
    }

    fn realcar_pasta(&mut self, k: usize, a: &ConfigDialogoArquivo) {
        self.pasta_realcada = Some(k);
        let vis = self.pastas_visiveis(a);
        if k < self.primeira_pasta {
            self.primeira_pasta = k;
        } else if k >= self.primeira_pasta + vis {
            self.primeira_pasta = k + 1 - vis;
        }
    }

    /// Mouse andando com o botão apertado. Devolve se mudou algo (redesenhar).
    pub fn mover_mouse(&mut self, cfg: &ConfigDialogos, interface: Option<&Interface>, x: i32, y: i32) -> bool {
        let Some(arr) = self.arrasto else { return false };
        let a = &cfg.arquivo;
        let (lx, ly) = self.local(x, y);
        match arr {
            ArrastoDialogo::Titulo { inicio, .. } => {
                self.arrasto = Some(ArrastoDialogo::Titulo { inicio, atual: (lx, ly) });
            }
            ArrastoDialogo::Polegar { lista, pega } => {
                let Some((_, p, _, max, _)) = self.barra_da_lista(lista, a) else { return false };
                let v = posicao_do_polegar(&p, ly - pega, max);
                self.definir_primeiro(lista, v);
            }
            ArrastoDialogo::Selecao => {
                let i = self.posicao_no_campo(lx, cfg, interface);
                self.campo.mover(i, true);
            }
            ArrastoDialogo::Repetir { .. } => return false,
        }
        true
    }

    /// Botão solto: termina o arrasto (o diálogo vai para o lugar do contorno, com o x no byte
    /// como o Windows põe as janelas de diálogo: arrastado 60 pixels, o canto foi de 56 a 120, não
    /// 116; bancada g19, g20).
    pub fn soltar(&mut self, cfg: &ConfigDialogos, interface: Option<&Interface>) -> bool {
        match self.arrasto.take() {
            Some(ArrastoDialogo::Titulo { inicio, atual }) => {
                let (dx, dy) = (atual.0 - inicio.0, atual.1 - inicio.1);
                let base = ret(cfg.arquivo.externo).esq;
                let x = base + self.deslocamento.0 + dx as i16;
                let x = interface.map_or(x, |i| i.driver.alinhar(x));
                self.deslocamento.0 = x - base;
                self.deslocamento.1 += dy as i16;
                true
            }
            Some(_) => true,
            None => false,
        }
    }

    /// Tempo passando: a seta ou o trilho segurados repetem (como a barra do Windows).
    pub fn avancar(&mut self, cfg: &ConfigDialogos, ms: u32) -> bool {
        let Some(ArrastoDialogo::Repetir { lista, y, espera }) = self.arrasto else { return false };
        let a = &cfg.arquivo;
        let [_, intervalo] = a.controles.repeticao_ms;
        let mut resta = ms;
        let mut esp = espera;
        let mut mudou = false;
        while resta >= esp {
            resta -= esp;
            esp = intervalo.max(1);
            if let Some((_, p, pos, max, vis)) = self.barra_da_lista(lista, a) {
                let novo = p.rolar(pos, max, vis, y);
                mudou |= novo != pos;
                self.definir_primeiro(lista, novo);
            }
        }
        self.arrasto = Some(ArrastoDialogo::Repetir { lista, y, espera: esp - resta });
        mudou
    }

    /// Roda do mouse sobre uma lista (extensão do port): `linhas` > 0 = para cima.
    pub fn rolar(&mut self, cfg: &ConfigDialogos, x: i32, y: i32, linhas: i32) -> bool {
        let a = &cfg.arquivo;
        let (lx, ly) = self.local(x, y);
        let Some(l) = self.lista_em(lx, ly, a) else { return false };
        self.rolar_lista(l, -linhas * a.controles.linhas_roda as i32, a);
        true
    }

    /// Mnemônico de um rótulo (a letra depois do `&`).
    fn mnemonico_de(texto: &str) -> Option<char> {
        texto.find('&').and_then(|p| texto[p + 1..].chars().next()).map(|c| c.to_ascii_lowercase())
    }

    /// Tecla (nome do SDL) com Shift, Ctrl e Alt.
    pub fn tecla_com(&mut self, nome: &str, shift: bool, ctrl: bool, alt: bool, cfg: &ConfigDialogos) -> RespostaArquivo {
        let a = &cfg.arquivo;
        // Lista suspensa aberta.
        if let Some(c) = self.combo {
            let total = self.itens_do_combo(c.qual, a);
            let (vis, _) = DialogoArquivo::visiveis_do_combo(c.qual, a);
            let realce = match nome {
                "Up" if alt => {
                    self.escolher_no_combo(c.qual, c.realce, cfg);
                    return RespostaArquivo::Nada;
                }
                "Up" => c.realce.saturating_sub(1),
                "Down" => (c.realce + 1).min(total.saturating_sub(1)),
                "Home" => 0,
                "End" => total.saturating_sub(1),
                "Return" | "Keypad Enter" | "F4" => {
                    self.escolher_no_combo(c.qual, c.realce, cfg);
                    return RespostaArquivo::Nada;
                }
                "Escape" => {
                    self.combo = None;
                    return RespostaArquivo::Nada;
                }
                _ => return RespostaArquivo::Nada,
            };
            let primeiro = if realce < c.primeiro { realce } else if realce >= c.primeiro + vis { realce + 1 - vis } else { c.primeiro };
            self.combo = Some(ComboAberto { realce, primeiro, ..c });
            return RespostaArquivo::Nada;
        }
        if alt {
            let letra = nome.chars().next().filter(|_| nome.chars().count() == 1).map(|c| c.to_ascii_lowercase());
            let rotulos = if self.salvar { &a.rotulos_salvar } else { &a.rotulos_abrir };
            let alvos = [Foco::Campo, Foco::Pastas, Foco::Tipo, Foco::Unidades];
            if let Some(l) = letra {
                if let Some(k) = rotulos.iter().position(|(t, _)| DialogoArquivo::mnemonico_de(t) == Some(l)) {
                    self.foco = alvos.get(k).copied().unwrap_or(Foco::Campo);
                    if self.foco == Foco::Campo {
                        self.campo.selecionar_tudo();
                    }
                } else if DialogoArquivo::mnemonico_de(&a.somente_leitura.0) == Some(l) {
                    self.somente_leitura = !self.somente_leitura;
                    self.foco = Foco::SomenteLeitura;
                }
            }
            if nome == "Down" {
                match self.foco {
                    Foco::Tipo => self.abrir_combo(Combo::Tipo, a),
                    Foco::Unidades => self.abrir_combo(Combo::Unidades, a),
                    _ => {}
                }
            }
            return RespostaArquivo::Nada;
        }
        match nome {
            "Tab" => {
                let i = ORDEM_FOCO.iter().position(|&f| f == self.foco).unwrap_or(0);
                let n = ORDEM_FOCO.len();
                self.foco = ORDEM_FOCO[if shift { (i + n - 1) % n } else { (i + 1) % n }];
                if self.foco == Foco::Campo {
                    self.campo.selecionar_tudo();
                }
                return RespostaArquivo::Nada;
            }
            "Escape" => return RespostaArquivo::Cancelar,
            "Return" | "Keypad Enter" => {
                return match self.foco {
                    Foco::Cancelar => RespostaArquivo::Cancelar,
                    Foco::Pastas => {
                        if let Some(k) = self.pasta_realcada {
                            self.abrir_pasta(k, cfg);
                        }
                        RespostaArquivo::Nada
                    }
                    _ => self.ok(cfg),
                };
            }
            _ => {}
        }
        match self.foco {
            Foco::Campo => {
                self.campo.tecla(nome, shift, ctrl);
            }
            Foco::Lista => {
                let n = self.arquivos.len();
                if n > 0 {
                    let vis = self.linhas_visiveis(a);
                    let atual = self.selecionado;
                    let novo = match nome {
                        "Up" => atual.map_or(0, |i| i.saturating_sub(1)),
                        "Down" => atual.map_or(0, |i| (i + 1).min(n - 1)),
                        "PageUp" => atual.map_or(0, |i| i.saturating_sub(vis - 1)),
                        "PageDown" => atual.map_or(0, |i| (i + vis - 1).min(n - 1)),
                        "Home" => 0,
                        "End" => n - 1,
                        _ => return RespostaArquivo::Nada,
                    };
                    self.selecionar_arquivo(novo, a);
                }
            }
            Foco::Pastas => {
                let n = self.pastas().len();
                if n > 0 {
                    let vis = self.pastas_visiveis(a);
                    let atual = self.pasta_realcada.unwrap_or(0);
                    let novo = match nome {
                        "Up" => atual.saturating_sub(1),
                        "Down" => (atual + 1).min(n - 1),
                        "PageUp" => atual.saturating_sub(vis - 1),
                        "PageDown" => (atual + vis - 1).min(n - 1),
                        "Home" => 0,
                        "End" => n - 1,
                        _ => return RespostaArquivo::Nada,
                    };
                    self.realcar_pasta(novo, a);
                }
            }
            Foco::Tipo | Foco::Unidades => {
                let qual = if self.foco == Foco::Tipo { Combo::Tipo } else { Combo::Unidades };
                match nome {
                    "F4" => self.abrir_combo(qual, a),
                    // Fechada, as setas trocam a escolha [hipótese: a troca de unidade na hora].
                    "Up" | "Down" if qual == Combo::Unidades => {
                        let n = self.unidades.len();
                        if n > 0 {
                            let i = if nome == "Up" { self.unidade.saturating_sub(1) } else { (self.unidade + 1).min(n - 1) };
                            self.escolher_unidade(i, cfg);
                        }
                    }
                    _ => {}
                }
            }
            Foco::SomenteLeitura => {
                if nome == "Space" {
                    self.somente_leitura = !self.somente_leitura;
                }
            }
            Foco::Ok => {
                if nome == "Space" {
                    return self.ok(cfg);
                }
            }
            Foco::Cancelar => {
                if nome == "Space" {
                    return RespostaArquivo::Cancelar;
                }
            }
        }
        RespostaArquivo::Nada
    }

    /// Tecla sem modificadores: Enter = OK, Esc = Cancelar, as de edição no campo.
    pub fn tecla(&mut self, nome: &str, cfg: &ConfigDialogos) -> RespostaArquivo {
        self.tecla_com(nome, false, false, false, cfg)
    }

    /// Texto digitado (eventos de texto do SDL): no campo, troca a seleção; na lista de arquivos,
    /// vai ao próximo nome que começa com a letra (a `LISTBOX` do Windows).
    pub fn digitar(&mut self, texto: &str, cfg: &ConfigDialogos) {
        match self.foco {
            Foco::Lista => {
                let Some(l) = texto.chars().next().map(|c| c.to_ascii_lowercase()) else { return };
                let n = self.arquivos.len();
                let ini = self.selecionado.map_or(0, |i| i + 1);
                if let Some(i) = (0..n).map(|k| (ini + k) % n.max(1)).find(|&i| self.arquivos[i].starts_with(l)) {
                    self.selecionar_arquivo(i, &cfg.arquivo);
                }
            }
            Foco::Campo => self.campo.inserir(texto),
            _ => {
                // Digitar com o foco noutro controle não escreve [como no Windows: o texto vai ao
                // controle com o foco].
            }
        }
    }

    fn caminho_absoluto(&self) -> PathBuf {
        std::path::absolute(&self.pasta).unwrap_or_else(|_| self.pasta.clone())
    }

    /// Pastas mostradas: os ancestrais da pasta (da raiz até ela, abertos) e as subpastas dela.
    fn pastas(&self) -> Vec<ItemPasta> {
        let caminho = self.caminho_absoluto();
        let mut v: Vec<ItemPasta> = Vec::new();
        let mut ate_aqui = PathBuf::new();
        for comp in caminho.components() {
            ate_aqui.push(comp.as_os_str());
            let nome = match comp {
                std::path::Component::Prefix(p) => format!("{}\\", p.as_os_str().to_string_lossy()),
                std::path::Component::RootDir if !v.is_empty() => {
                    // "c:" + "\": o item da unidade é a raiz dela.
                    if let Some(u) = v.last_mut() {
                        u.caminho = ate_aqui.clone();
                    }
                    continue;
                }
                std::path::Component::RootDir => "\\".into(),
                c => c.as_os_str().to_string_lossy().into_owned(),
            };
            v.push(ItemPasta { nivel: v.len(), nome: nome.to_lowercase(), aberta: true, caminho: ate_aqui.clone() });
        }
        let nivel = v.len();
        let mut sub: Vec<(String, PathBuf)> = std::fs::read_dir(&caminho)
            .map(|it| {
                it.flatten()
                    .filter(|e| e.path().is_dir())
                    .map(|e| (e.file_name().to_string_lossy().to_lowercase(), e.path()))
                    .collect()
            })
            .unwrap_or_default();
        sub.sort();
        v.extend(sub.into_iter().map(|(nome, caminho)| ItemPasta { nivel, nome, aberta: false, caminho }));
        v
    }

    /// Contorno do diálogo na área cliente (no lugar para onde foi arrastado).
    pub fn externo(&self, cfg: &ConfigDialogos) -> Retangulo {
        let e = ret(cfg.arquivo.externo);
        let (dx, dy) = self.deslocamento;
        Retangulo { esq: e.esq + dx, topo: e.topo + dy, dir: e.dir + dx, base: e.base + dy }
    }

    /// Desenha o diálogo no lugar dele (com o deslocamento do arrasto), a lista suspensa aberta e o
    /// contorno pontilhado de um arrasto do título.
    pub fn desenhar(&self, q: &mut Quadro, interface: &Interface, cfg: &ConfigDialogos) {
        let a = &cfg.arquivo;
        // Desenha no lugar medido num quadro à parte e copia para o lugar atual.
        let mut t = Quadro::new(q.largura, q.altura, 0);
        let (dx, dy) = (i32::from(self.deslocamento.0), i32::from(self.deslocamento.1));
        for y in 0..q.altura as i32 {
            for x in 0..q.largura as i32 {
                let (sx, sy) = (x + dx, y + dy);
                if sx >= 0 && sy >= 0 && (sx as usize) < q.largura && (sy as usize) < q.altura {
                    t.pixels[y as usize * q.largura + x as usize] = q.pixels[sy as usize * q.largura + sx as usize];
                }
            }
        }
        self.desenhar_no_lugar(&mut t, interface, cfg);
        let mut regioes = vec![ret(a.externo)];
        if let Some(c) = self.combo {
            regioes.push(DialogoArquivo::lista_do_combo(c.qual, a));
        }
        for r in regioes {
            for y in i32::from(r.topo)..i32::from(r.base) {
                for x in i32::from(r.esq)..i32::from(r.dir) {
                    let (tx, ty) = (x + dx, y + dy);
                    if x < 0 || y < 0 || x as usize >= q.largura || y as usize >= q.altura {
                        continue;
                    }
                    if tx >= 0 && ty >= 0 && (tx as usize) < q.largura && (ty as usize) < q.altura {
                        q.pixels[ty as usize * q.largura + tx as usize] = t.pixels[y as usize * q.largura + x as usize];
                    }
                }
            }
        }
        if let Some(ArrastoDialogo::Titulo { inicio, atual }) = self.arrasto {
            let e = self.externo(cfg);
            let (mx, my) = ((atual.0 - inicio.0) as i16, (atual.1 - inicio.1) as i16);
            let r = Retangulo { esq: e.esq + mx, topo: e.topo + my, dir: e.dir - 1 + mx, base: e.base - 1 + my };
            // Pontos invertidos onde x + y da tela é par (g19; a tela é a área cliente + 38).
            super::gerenciador::contorno_pontilhado_fase(q, r, 15, 1);
        }
    }


    fn desenhar_no_lugar(&self, q: &mut Quadro, interface: &Interface, cfg: &ConfigDialogos) {
        let c = &cfg.cores;
        let a = &cfg.arquivo;
        let e = ret(a.externo);
        // Moldura de diálogo: contorno preto, borda azul e o interior branco.
        q.preencher(e, c.moldura, c.moldura);
        q.contorno(Retangulo { dir: e.dir - 1, base: e.base - 1, ..e }, c.linha);
        let [me, mt, md, mb] = a.moldura;
        let i = Retangulo { esq: e.esq + me, topo: e.topo + mt, dir: e.dir - md, base: e.base - mb };
        q.preencher(i, c.fundo, c.fundo);
        titulo(q, fonte(&cfg.fontes_titulo, interface), c, DialogoArquivo::titulo(a), if self.salvar { &a.titulo_salvar } else { &a.titulo_abrir });
        let Some(f) = fonte(&a.fontes, interface) else { return };
        let rotulos = if self.salvar { &a.rotulos_salvar } else { &a.rotulos_abrir };
        for (texto, [x, y]) in rotulos {
            escrever(q, f, i32::from(*x), i32::from(*y), texto, c.texto);
        }
        self.desenhar_campo(q, f, cfg);
        // Lista de arquivos (cinza no Save As, como no original).
        let lista = ret(a.lista);
        caixa_branca(q, c, lista);
        rolagem(q, c, lista, a.largura_rolagem, Some((self.primeiro, self.primeiro_maximo(a))));
        let cor = if self.salvar { c.desabilitado } else { c.texto };
        let dentro_lista = Retangulo { esq: lista.esq + 1, topo: lista.topo + 1, dir: lista.dir - a.largura_rolagem, base: lista.base - 1 };
        for (k, nome) in self.arquivos.iter().skip(self.primeiro).take(self.linhas_visiveis(a)).enumerate() {
            let y = i32::from(lista.topo) + 1 + k as i32 * i32::from(a.altura_item);
            let x = i32::from(lista.esq) + 5;
            let r = Retangulo { topo: y as i16, base: y as i16 + a.altura_item, ..dentro_lista };
            if Some(k + self.primeiro) == self.selecionado {
                q.preencher(r, c.selecao, c.selecao);
                escrever(q, f, x, y, nome, c.texto_selecao);
            } else {
                escrever(q, f, x, y, nome, cor);
            }
        }
        if self.foco == Foco::Lista {
            // Retângulo de foco no item selecionado (ou no primeiro visível).
            let k = self.selecionado.filter(|&s| s >= self.primeiro && s < self.primeiro + self.linhas_visiveis(a)).unwrap_or(self.primeiro) - self.primeiro;
            let y = lista.topo + 1 + k as i16 * a.altura_item;
            retangulo_de_foco_sobre(q, c, Retangulo { topo: y, base: y + a.altura_item, ..dentro_lista });
        }
        // Pastas: o caminho e a árvore (ícones de pasta próprios, simplificados).
        let [cx, cy] = a.caminho;
        let caminho = self.caminho_absoluto().display().to_string().to_lowercase();
        let pastas = ret(a.pastas);
        let antes = q.recorte.replace(Retangulo { esq: cx, topo: cy, dir: pastas.dir, base: pastas.topo });
        escrever(q, f, i32::from(cx), i32::from(cy), &caminho, c.texto);
        q.recorte = antes;
        caixa_branca(q, c, pastas);
        let itens = self.pastas();
        let max_pastas = itens.len().saturating_sub(self.pastas_visiveis(a));
        rolagem(q, c, pastas, a.largura_rolagem, Some((self.primeira_pasta, max_pastas)));
        let dentro = Retangulo { esq: pastas.esq + 1, topo: pastas.topo + 1, dir: pastas.dir - a.largura_rolagem, base: pastas.base - 1 };
        let antes = q.recorte.replace(dentro);
        for (k, p) in itens.iter().enumerate().skip(self.primeira_pasta) {
            let y = pastas.topo + 1 + (k - self.primeira_pasta) as i16 * a.altura_item_pasta;
            let x = pastas.esq + 5 + p.nivel as i16 * a.recuo_pasta;
            let realcada = self.pasta_realcada == Some(k);
            if realcada {
                q.preencher(Retangulo { topo: y, base: y + a.altura_item_pasta, ..dentro }, c.selecao, c.selecao);
            }
            let r = Retangulo { esq: x, topo: y + 3, dir: x + 16, base: y + 13 };
            q.preencher(r, c.pasta, c.pasta);
            q.contorno(Retangulo { dir: r.dir - 1, base: r.base - 1, ..r }, c.linha);
            if p.aberta {
                q.linha(i32::from(x) + 2, i32::from(y) + 7, i32::from(x) + 15, i32::from(y) + 7, c.linha);
            }
            let cor = if realcada { c.texto_selecao } else { c.texto };
            escrever(q, f, i32::from(x + a.texto_pasta), i32::from(y), &p.nome, cor);
            if realcada && self.foco == Foco::Pastas {
                retangulo_de_foco_sobre(q, c, Retangulo { topo: y, base: y + a.altura_item_pasta, ..dentro });
            }
        }
        if self.foco == Foco::Pastas && self.pasta_realcada.is_none() {
            retangulo_de_foco(q, Retangulo { base: dentro.topo + a.altura_item_pasta, ..dentro });
        }
        q.recorte = antes;
        // Listas suspensas fechadas: tipo e unidade.
        for qual in [Combo::Tipo, Combo::Unidades] {
            let r = DialogoArquivo::caixa_do_combo(qual, a);
            caixa_branca(q, c, r);
            let x = r.dir - a.largura_rolagem;
            q.preencher(Retangulo { esq: x, dir: x + 1, ..r }, c.linha, c.linha);
            let b = Retangulo { esq: x + 1, topo: r.topo + 1, dir: r.dir - 1, base: r.base - 1 };
            relevo(q, c, b, 1);
            seta(q, c, b, true, true, false);
            // Com a lista aberta, a caixa não fica realçada (g01, g03).
            let com_foco = self.combo.is_none()
                && ((qual == Combo::Tipo && self.foco == Foco::Tipo) || (qual == Combo::Unidades && self.foco == Foco::Unidades));
            if com_foco {
                // Com o foco, o texto da caixa fica realçado (g05).
                let campo = Retangulo { esq: r.esq + 2, topo: r.topo + 2, dir: x - 1, base: r.base - 2 };
                q.preencher(campo, c.selecao, c.selecao);
            }
            let cor_texto = if com_foco { c.texto_selecao } else { c.texto };
            let ty = (i32::from(r.topo) + i32::from(r.base) - f.altura() as i32) / 2;
            match qual {
                Combo::Tipo => escrever(q, f, i32::from(r.esq) + 3, ty, &a.texto_tipo, cor_texto),
                Combo::Unidades => {
                    let k = &a.controles;
                    let (texto, tipo) = self.texto_da_unidade(self.unidade, a);
                    self.icone_unidade(q, c, a, tipo, i32::from(r.esq + k.unidade_icone_fechada[0]), i32::from(r.topo + k.unidade_icone_fechada[1]));
                    escrever(q, f, i32::from(r.esq + k.unidade_texto_fechada), ty, &texto, cor_texto);
                }
            }
        }
        let padrao_ok = self.foco != Foco::Cancelar;
        botao(q, Some(f), c, ret(a.ok), &a.texto_ok, padrao_ok);
        botao(q, Some(f), c, ret(a.cancelar), &a.texto_cancelar, !padrao_ok);
        for (foco, caixa) in [(Foco::Ok, a.ok), (Foco::Cancelar, a.cancelar)] {
            if self.foco == foco {
                let r = ret(caixa);
                retangulo_de_foco(q, Retangulo { esq: r.esq + 4, topo: r.topo + 4, dir: r.dir - 4, base: r.base - 4 });
            }
        }
        let (texto, [x, y]) = &a.somente_leitura;
        q.preencher(Retangulo { esq: *x, topo: *y, dir: x + 13, base: y + 13 }, c.campo, c.campo);
        q.contorno(Retangulo { esq: *x, topo: *y, dir: x + 12, base: y + 12 }, c.linha);
        if self.somente_leitura {
            // O "X" da caixa marcada do Windows 3.1 (g12): as duas diagonais do interior.
            let (x0, y0) = (i32::from(*x) + 1, i32::from(*y) + 1);
            for k in 0..11 {
                q.ponto(x0 + k, y0 + k, c.linha);
                q.ponto(x0 + 10 - k, y0 + k, c.linha);
            }
        }
        escrever(q, f, i32::from(*x) + 18, i32::from(*y) + 1, texto, c.texto);
        if self.foco == Foco::SomenteLeitura {
            let w = largura(f, texto) as i16 + i16::from(f.negrito());
            retangulo_de_foco(q, Retangulo { esq: x + 18, topo: *y, dir: x + 18 + w + 2, base: y + f.altura() as i16 + 3 });
        }
        // Lista suspensa aberta, por cima de tudo (pode passar da borda do diálogo).
        if let Some(cb) = self.combo {
            self.desenhar_lista_aberta(q, f, cfg, cb);
        }
    }

    /// Texto e tipo da unidade `i` (só a letra e os dois-pontos, como na bancada: "a:", "c:").
    fn texto_da_unidade(&self, i: usize, a: &ConfigDialogoArquivo) -> (String, TipoUnidade) {
        match self.unidades.get(i) {
            Some(u) => (format!("{}:", u.letra), u.tipo),
            None => (a.texto_unidade.clone(), TipoUnidade::Disco),
        }
    }

    fn icone_unidade(&self, q: &mut Quadro, c: &Cores, a: &ConfigDialogoArquivo, tipo: TipoUnidade, x: i32, y: i32) {
        let nome = match tipo {
            TipoUnidade::Disquete => "disquete",
            TipoUnidade::Disco => "disco",
        };
        let Some(linhas) = a.controles.icones_unidade.get(nome) else { return };
        for (j, l) in linhas.iter().enumerate() {
            for (i, ch) in l.chars().enumerate() {
                let cor = match ch {
                    '#' => c.linha,
                    'w' => c.claro,
                    '.' => c.botao,
                    ':' => c.escuro,
                    _ => continue,
                };
                q.ponto(x + i as i32, y + j as i32, cor);
            }
        }
    }

    fn desenhar_lista_aberta(&self, q: &mut Quadro, f: &Fonte, cfg: &ConfigDialogos, cb: ComboAberto) {
        let (c, a) = (&cfg.cores, &cfg.arquivo);
        let r = DialogoArquivo::lista_do_combo(cb.qual, a);
        let (vis, h) = DialogoArquivo::visiveis_do_combo(cb.qual, a);
        let total = self.itens_do_combo(cb.qual, a);
        caixa_branca(q, c, r);
        let com_barra = total > vis;
        let dir = if com_barra { r.dir - a.largura_rolagem } else { r.dir - 1 };
        if com_barra {
            rolagem(q, c, r, a.largura_rolagem, Some((cb.primeiro, total - vis)));
        }
        for k in 0..vis.min(total.saturating_sub(cb.primeiro)) {
            let i = cb.primeiro + k;
            let y = r.topo + 1 + k as i16 * h;
            let item = Retangulo { esq: r.esq + 1, topo: y, dir, base: y + h };
            let realce = i == cb.realce;
            if realce {
                q.preencher(item, c.selecao, c.selecao);
            }
            let cor = if realce { c.texto_selecao } else { c.texto };
            let ty = i32::from(y) + (i32::from(h) - f.altura() as i32) / 2;
            match cb.qual {
                Combo::Tipo => escrever(q, f, i32::from(r.esq) + 3, ty, &a.texto_tipo, cor),
                Combo::Unidades => {
                    let k = &a.controles;
                    let (texto, tipo) = self.texto_da_unidade(i, a);
                    self.icone_unidade(q, c, a, tipo, i32::from(r.esq + k.unidade_icone[0]), i32::from(y + k.unidade_icone[1]));
                    escrever(q, f, i32::from(r.esq + k.unidade_texto), ty, &texto, cor);
                }
            }
        }
    }

    /// O campo "File Name:" com a seleção (só com o foco, como o `EDIT` do Windows) e o cursor.
    fn desenhar_campo(&self, q: &mut Quadro, f: &Fonte, cfg: &ConfigDialogos) {
        let (c, a) = (&cfg.cores, &cfg.arquivo);
        let campo = ret(a.campo);
        caixa_branca(q, c, campo);
        let larg = |s: &str| largura(f, s);
        let (tx, ty) = (i32::from(campo.esq) + 3, (i32::from(campo.topo) + i32::from(campo.base) - f.altura() as i32) / 2);
        let visivel = i32::from(campo.dir - campo.esq) - 6;
        let ini = self.campo.primeiro_visivel(visivel, larg);
        let t = &self.campo.texto;
        let x_de = |i: usize| tx + larg(&t[ini..i.max(ini)]);
        let antes = q.recorte.replace(Retangulo { esq: campo.esq + 1, topo: campo.topo + 1, dir: campo.dir - 1, base: campo.base - 1 });
        escrever(q, f, tx, ty, &t[ini..], c.texto);
        if self.foco_no_campo() {
            let (s0, s1) = self.campo.selecao();
            if s1 > s0 {
                let (x0, x1) = (x_de(s0), x_de(s1));
                let r = Retangulo { esq: x0 as i16, topo: ty as i16, dir: x1 as i16, base: (ty + f.altura() as i32) as i16 + 1 };
                q.preencher(r, c.selecao, c.selecao);
                escrever(q, f, x0, ty, &t[s0.max(ini)..s1], c.texto_selecao);
            } else {
                // Cursor: uma coluna da altura da fonte mais 1 (g13).
                let xc = x_de(self.campo.cursor);
                q.linha(xc, ty, xc, ty + f.altura() as i32, c.linha);
            }
        }
        q.recorte = antes;
    }
}

/// Retângulo de foco do Windows (`DrawFocusRect`): pontilhado de 1 pixel em XOR. Sobre o realce
/// (cor do sistema, fora das 16), o ponto fica na complementar do azul-escuro do VGA: amarelo
/// (bancada g09: os pontos do item selecionado são amarelos).
fn retangulo_de_foco_sobre(q: &mut Quadro, c: &Cores, r: Retangulo) {
    let (e, t, d, b) = (i32::from(r.esq), i32::from(r.topo), i32::from(r.dir) - 1, i32::from(r.base) - 1);
    let mut ponto = |x: i32, y: i32| {
        if (x + y + 1).rem_euclid(2) == 1 && x >= 0 && y >= 0 && (x as usize) < q.largura && (y as usize) < q.altura {
            let i = y as usize * q.largura + x as usize;
            let v = q.pixels[i];
            if v == c.selecao {
                q.pixels[i] = c.foco_selecao.unwrap_or(11);
            } else if v < 16 {
                q.pixels[i] = v ^ 15;
            }
        }
    };
    for x in e..=d {
        ponto(x, t);
        ponto(x, b);
    }
    for y in t + 1..b {
        ponto(e, y);
        ponto(d, y);
    }
}

/// Retângulo de foco do Windows (`DrawFocusRect`): pontilhado de 1 pixel em XOR.
fn retangulo_de_foco(q: &mut Quadro, r: Retangulo) {
    // Pontilhado alinhado à tela (a área cliente + 38; g12).
    super::gerenciador::contorno_pontilhado_fase(q, Retangulo { dir: r.dir - 1, base: r.base - 1, ..r }, 15, 1);
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cfg() -> ConfigDialogos {
        serde_json::from_str(&crate::recursos::marcar_referencias(include_str!("../../../data/dialogos.json"))).unwrap()
    }


    #[test]
    fn ok_com_curinga_nao_escolhe_e_nome_vira_caminho() {
        // O campo inicial vem do SIMANTW.EXE (referência); aqui, um curinga qualquer.
        let mut c = cfg();
        c.arquivo.campo_inicial = "*.ant".into();
        let i: Option<&Interface> = None;
        let mut d = DialogoArquivo::com_unidades(false, Path::new("nao_existe"), &c, vec![]);
        assert_eq!(d.campo.texto, c.arquivo.campo_inicial, "o campo volta ao texto inicial");
        assert_eq!(d.campo.selecao(), (0, 5), "selecionado");
        assert_eq!(d.tecla("Return", &c), RespostaArquivo::Nada);
        d.digitar("TESTE", &c);
        assert_eq!(d.campo.texto, "TESTE");
        let esperado = RespostaArquivo::Escolher(Path::new("nao_existe").join("TESTE.ANT"));
        assert_eq!(d.tecla("Return", &c), esperado);
        assert_eq!(d.tecla("Escape", &c), RespostaArquivo::Cancelar);
        let ok = ret(c.arquivo.ok);
        assert_eq!(d.clicar(&c, i, i32::from(ok.esq) + 5, i32::from(ok.topo) + 5, false, false), esperado);
    }

    /// Bancada (2026-10-19, `qemu_dialogo_muitos.txt`, 18 arquivos, 8 visíveis): seta de baixo duas
    /// vezes (primeiro 2, polegar 11 linhas abaixo da seta de cima), trilho abaixo do polegar (9:
    /// uma página de 7), seta de cima (8, polegar 46 abaixo; arredondado) e o clique num item.
    #[test]
    fn lista_com_muitos_arquivos_rola_como_o_original() {
        let c = cfg();
        let i: Option<&Interface> = None;
        let mut d = DialogoArquivo::com_unidades(false, Path::new("nao_existe"), &c, vec![]);
        d.arquivos = (0..18).map(|i| format!("a{i:02}.ant")).collect();
        let a = &c.arquivo;
        let lista = ret(a.lista);
        let p = PartesRolagem::de(lista, a.largura_rolagem);
        let x = i32::from(lista.dir) - 8;
        let (cima, baixo) = (i32::from(lista.topo) + 8, i32::from(lista.base) - 8);
        assert_eq!(d.primeiro_maximo(a), 10);
        assert_eq!(p.linha_polegar(0, 10), p.cima.base);
        for _ in 0..2 {
            d.clicar(&c, i, x, baixo, false, false);
            d.soltar(&c, None);
        }
        assert_eq!(d.primeiro, 2);
        assert_eq!(p.linha_polegar(2, 10) - p.cima.base, 11);
        d.clicar(&c, i, x, i32::from(lista.topo) + 77, false, false);
        d.soltar(&c, None);
        assert_eq!(d.primeiro, 9);
        assert_eq!(p.linha_polegar(9, 10) - p.cima.base, 51);
        d.clicar(&c, i, x, cima, false, false);
        d.soltar(&c, None);
        assert_eq!(d.primeiro, 8);
        assert_eq!(p.linha_polegar(8, 10) - p.cima.base, 46);
        assert_eq!(p.linha_polegar(10, 10) + p.lado() + 1, p.baixo.topo - 1, "no fim, o polegar encosta na seta de baixo");
        d.clicar(&c, i, i32::from(lista.esq) + 40, i32::from(lista.topo) + 1 + 13 + 6, false, false);
        assert_eq!((d.selecionado, d.campo.texto.as_str(), d.foco_no_campo()), (Some(9), "a09.ant", false));
        for _ in 0..5 {
            d.clicar(&c, i, x, baixo, false, false);
            d.soltar(&c, None);
        }
        assert_eq!(d.primeiro, 10, "não passa do fim");
    }

    /// Polegar arrastado, seta segurada (repete depois da espera), roda do mouse e teclado na lista
    /// (bancada g09–g11: três ↓ do primeiro item dão o quarto, End o último, com o nome no campo).
    #[test]
    fn polegar_repeticao_roda_e_teclado_na_lista() {
        let c = cfg();
        let i: Option<&Interface> = None;
        let mut d = DialogoArquivo::com_unidades(false, Path::new("nao_existe"), &c, vec![]);
        d.arquivos = (0..18).map(|i| format!("a{i:02}.ant")).collect();
        let a = &c.arquivo;
        let lista = ret(a.lista);
        let p = PartesRolagem::de(lista, a.largura_rolagem);
        let x = i32::from(lista.dir) - 8;
        // Polegar: pega na linha de cima e leva ao fim do curso.
        let y0 = i32::from(p.cima.base) + 2;
        d.clicar(&c, i, x, y0, false, false);
        d.mover_mouse(&c, i, x, i32::from(p.baixo.topo));
        assert_eq!(d.primeiro, 10);
        d.mover_mouse(&c, i, x, y0 + 23);
        assert_eq!(d.primeiro, 4, "23 de 57 linhas de curso: 4 de 10");
        d.soltar(&c, None);
        // Seta de baixo segurada: um passo já, outro depois da espera e um a cada intervalo.
        d.primeiro = 0;
        d.clicar(&c, i, x, i32::from(lista.base) - 8, false, false);
        assert_eq!(d.primeiro, 1);
        let [espera, intervalo] = a.controles.repeticao_ms;
        d.avancar(&c, espera - 1);
        assert_eq!(d.primeiro, 1);
        d.avancar(&c, 1 + 2 * intervalo);
        assert_eq!(d.primeiro, 4);
        d.soltar(&c, None);
        d.avancar(&c, 10 * intervalo);
        assert_eq!(d.primeiro, 4, "solto, para");
        // Roda: 3 linhas por passo, nas duas listas.
        assert!(d.rolar(&c, i32::from(lista.esq) + 20, i32::from(lista.topo) + 20, -1));
        assert_eq!(d.primeiro, 7);
        assert!(d.rolar(&c, i32::from(lista.esq) + 20, i32::from(lista.topo) + 20, 5));
        assert_eq!(d.primeiro, 0);
        // Teclado.
        d.clicar(&c, i, i32::from(lista.esq) + 20, i32::from(lista.topo) + 4, false, false);
        for _ in 0..3 {
            d.tecla("Down", &c);
        }
        assert_eq!((d.selecionado, d.campo.texto.as_str()), (Some(3), "a03.ant"));
        d.tecla("End", &c);
        assert_eq!((d.selecionado, d.primeiro), (Some(17), 10));
        d.tecla("Home", &c);
        assert_eq!((d.selecionado, d.primeiro), (Some(0), 0));
        d.digitar("a", &c);
        assert_eq!(d.selecionado, Some(1), "a letra vai ao próximo nome com ela");
    }

    /// Listas suspensas (g01–g05): a de tipos tem o filtro só; a das unidades lista as do sistema e
    /// troca de pasta; Esc fecha sem trocar; o clique fora fecha.
    #[test]
    fn listas_suspensas_e_read_only() {
        let c = cfg();
        let i: Option<&Interface> = None;
        let raiz = std::env::temp_dir();
        let unidades = vec![Unidade { letra: 'a', tipo: TipoUnidade::Disquete }, Unidade { letra: 'c', tipo: TipoUnidade::Disco }];
        let mut d = DialogoArquivo::com_unidades(false, &raiz, &c, unidades);
        let a = &c.arquivo;
        let tipo = ret(a.tipo);
        d.clicar(&c, i, i32::from(tipo.dir) - 5, i32::from(tipo.topo) + 5, false, false);
        assert_eq!(d.combo, Some(ComboAberto { qual: Combo::Tipo, realce: 0, primeiro: 0 }));
        let l = DialogoArquivo::lista_do_combo(Combo::Tipo, a);
        assert_eq!((l.topo, l.base - l.topo), (tipo.base - 1, 93), "g01: 7 itens de 13 e o contorno");
        d.clicar(&c, i, i32::from(tipo.dir) - 5, i32::from(tipo.topo) + 5, false, false);
        assert_eq!(d.combo, None, "clicar de novo na caixa fecha");
        let u = ret(a.unidades);
        d.clicar(&c, i, i32::from(u.esq) + 10, i32::from(u.topo) + 5, false, false);
        assert_eq!(d.combo.map(|c| c.qual), Some(Combo::Unidades));
        let l = DialogoArquivo::lista_do_combo(Combo::Unidades, a);
        assert_eq!(l.base - l.topo, 82, "g03: 5 itens de 16 e o contorno");
        d.tecla("Escape", &c);
        assert_eq!(d.combo, None);
        assert_eq!(d.foco, Foco::Unidades);
        // Read Only: o clique marca e desmarca; Alt+R também.
        let (_, [sx, sy]) = a.somente_leitura;
        d.clicar(&c, i, i32::from(sx) + 5, i32::from(sy) + 5, false, false);
        assert!(d.somente_leitura);
        d.tecla_com("R", false, false, true, &c);
        assert!(!d.somente_leitura);
        assert_eq!(d.foco, Foco::SomenteLeitura);
        // Tab percorre os controles na ordem do diálogo.
        d.tecla("Tab", &c);
        assert_eq!(d.foco, Foco::Campo);
        assert_eq!(d.campo.selecao(), (0, d.campo.texto.len()));
        d.tecla_com("Tab", true, false, false, &c);
        assert_eq!(d.foco, Foco::SomenteLeitura);
    }

    /// Arrasto pelo título: o contorno segue o cursor e o diálogo só vai ao soltar (g19, g20); os
    /// cliques seguintes valem no lugar novo.
    #[test]
    fn arrastar_o_dialogo_pelo_titulo() {
        let mut c = cfg();
        c.arquivo.campo_inicial = "*.ant".into();
        let i: Option<&Interface> = None;
        let mut d = DialogoArquivo::com_unidades(false, Path::new("nao_existe"), &c, vec![]);
        let t = DialogoArquivo::titulo(&c.arquivo);
        let (x, y) = (i32::from(t.esq) + 200, i32::from(t.topo) + 8);
        d.clicar(&c, i, x, y, false, false);
        d.mover_mouse(&c, i, x + 60, y + 40);
        assert_eq!(d.deslocamento, (0, 0));
        assert!(d.soltar(&c, None));
        assert_eq!(d.deslocamento, (60, 40));
        assert_eq!(d.externo(&c).esq, ret(c.arquivo.externo).esq + 60);
        let ok = ret(c.arquivo.ok);
        assert_eq!(d.clicar(&c, i, i32::from(ok.esq) + 5, i32::from(ok.topo) + 5, false, false), RespostaArquivo::Nada, "o OK saiu dali");
        assert_eq!(d.clicar(&c, i, i32::from(ok.esq) + 65, i32::from(ok.topo) + 45, false, false), RespostaArquivo::Nada);
        assert_eq!(d.foco, Foco::Ok, "o OK está no lugar novo");
        // Duplo clique na caixa do menu de sistema: Cancel.
        assert_eq!(d.clicar(&c, i, i32::from(t.esq) + 65, i32::from(t.topo) + 45, true, false), RespostaArquivo::Cancelar);
    }

    /// Um nome de pasta no campo e Enter troca de pasta (o original troca de unidade com "a:").
    #[test]
    fn pasta_no_campo_troca_de_pasta() {
        let mut c = cfg();
        c.arquivo.campo_inicial = "*.ant".into();
        let raiz = std::env::temp_dir().join(format!("simant_dlg_{}", std::process::id()));
        std::fs::create_dir_all(raiz.join("sub")).unwrap();
        std::fs::write(raiz.join("sub").join("x.ant"), b"").unwrap();
        let mut d = DialogoArquivo::com_unidades(false, &raiz, &c, vec![]);
        assert!(d.arquivos.is_empty());
        d.digitar("sub", &c);
        assert_eq!(d.tecla("Return", &c), RespostaArquivo::Nada);
        assert_eq!(d.arquivos, vec!["x.ant".to_string()]);
        assert_eq!(d.campo.texto, c.arquivo.campo_inicial, "o campo volta ao texto inicial");
        assert!(d.pastas().iter().any(|p| p.aberta && p.nome == "sub"));
        // Árvore pelo teclado: Home realça a raiz, Enter a abre.
        d.foco = Foco::Pastas;
        d.tecla("Home", &c);
        assert_eq!(d.pasta_realcada, Some(0));
        std::fs::remove_dir_all(&raiz).unwrap();
    }

    #[test]
    fn caixa_de_mensagem_do_load_tem_a_geometria_medida() {
        let c = cfg();
        let m = CaixaMensagem::pergunta_load(&c);
        let b = m.geometria_com(&c.caixa_mensagem, None, |_, _| (0, 0)).botoes;
        assert_eq!(b.len(), 3);
        assert_eq!((b[1].esq - b[0].esq, b[2].esq - b[0].esq), (74, 150));
        assert_eq!(m.tecla("Return"), Some(1));
        assert_eq!(m.tecla("Escape"), Some(2));
        assert_eq!(m.tecla("Y"), Some(0));
        assert_eq!(m.tecla("n"), Some(1));
    }
}
