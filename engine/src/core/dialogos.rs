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
}

/// Estilo da caixa: espessura da moldura (contorno incluído: esquerda, cima, direita, baixo), a
/// margem branca em volta do título e o centro na área cliente [medido: a pergunta do Load tem só o
/// contorno; a "Message" do `_PopMsg`, a moldura de diálogo azul].
#[derive(Clone, Debug, Deserialize)]
pub struct EstiloCaixa {
    pub borda: [i16; 4],
    pub margem_titulo: i16,
    pub centro: [i16; 2],
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
    pub titulo_mensagem: String,
    pub salvo: String,
    pub sim: String,
    pub nao: String,
    pub cancelar: String,
    pub ok: String,
    pub padrao_load: usize,
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
}

/// Conteúdo de data/dialogos.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigDialogos {
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

/// Barra de rolagem vertical sem polegar (a lista cabe), à direita de `r` (contorno incluído).
fn rolagem(q: &mut Quadro, c: &Cores, r: Retangulo, largura: i16) {
    let x = r.dir - largura;
    q.preencher(Retangulo { esq: x, dir: x + 1, ..r }, c.linha, c.linha);
    let lado = largura - 2;
    let cima = Retangulo { esq: x + 1, topo: r.topo + 1, dir: r.dir - 1, base: r.topo + 1 + lado };
    relevo(q, c, cima, 1);
    seta(q, c, cima, false, false, true);
    q.linha(i32::from(x), i32::from(cima.base), i32::from(r.dir) - 1, i32::from(cima.base), c.linha);
    let baixo = Retangulo { topo: r.base - 1 - lado, base: r.base - 1, ..cima };
    relevo(q, c, baixo, 1);
    seta(q, c, baixo, true, false, true);
    q.linha(i32::from(x), i32::from(baixo.topo) - 1, i32::from(r.dir) - 1, i32::from(baixo.topo) - 1, c.linha);
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
    fn geometria(&self, cfg: &ConfigCaixaMensagem, f: Option<&Fonte>) -> GeometriaCaixa {
        let est = cfg.estilos.get(self.estilo).or(cfg.estilos.first()).cloned().unwrap_or(EstiloCaixa { borda: [1; 4], margem_titulo: 0, centro: [320, 221] });
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
        let (x, y) = (est.centro[0] - w / 2, est.centro[1] - h / 2);
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
        let g = self.geometria(m, f);
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

    /// Botão sob (x, y).
    pub fn botao_em(&self, interface: &Interface, cfg: &ConfigDialogos, x: i32, y: i32) -> Option<usize> {
        let g = self.geometria(&cfg.caixa_mensagem, fonte(&cfg.caixa_mensagem.fontes, interface));
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

/// "Load Ant Hill" / "Save Ant Hill As" (`_FileSelect`, diálogo comum do Windows).
#[derive(Clone, Debug)]
pub struct DialogoArquivo {
    pub salvar: bool,
    pub pasta: PathBuf,
    pub campo: String,
    pub arquivos: Vec<String>,
    pub selecionado: Option<usize>,
    pub primeiro: usize,
}

impl DialogoArquivo {
    pub fn new(salvar: bool, pasta: &Path, cfg: &ConfigDialogos) -> DialogoArquivo {
        let arquivos = arquivos_ant(pasta)
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
            .collect();
        DialogoArquivo {
            salvar,
            pasta: pasta.to_path_buf(),
            campo: cfg.arquivo.campo_inicial.clone(),
            arquivos,
            selecionado: None,
            primeiro: 0,
        }
    }

    fn linhas_visiveis(&self, cfg: &ConfigDialogoArquivo) -> usize {
        ((cfg.lista[3] - 2) / cfg.altura_item).max(1) as usize
    }

    /// OK: o nome do campo vira caminho (com curinga, nada).
    fn ok(&self) -> RespostaArquivo {
        if self.campo.contains('*') || self.campo.contains('?') {
            return RespostaArquivo::Nada;
        }
        match caminho_do_nome(&self.pasta, &self.campo) {
            Some(p) => RespostaArquivo::Escolher(p),
            None => RespostaArquivo::Nada,
        }
    }

    /// Clique em (x, y); `duplo` = clique duplo (na lista, escolhe o arquivo).
    pub fn clicar(&mut self, cfg: &ConfigDialogos, x: i32, y: i32, duplo: bool) -> RespostaArquivo {
        let a = &cfg.arquivo;
        if contem(ret(a.ok), x, y) {
            return self.ok();
        }
        if contem(ret(a.cancelar), x, y) {
            return RespostaArquivo::Cancelar;
        }
        let lista = ret(a.lista);
        if contem(Retangulo { dir: lista.dir - a.largura_rolagem, ..lista }, x, y) {
            let k = ((y - i32::from(lista.topo) - 1) / i32::from(a.altura_item)).max(0) as usize;
            let i = k + self.primeiro;
            if i < self.arquivos.len() && k < self.linhas_visiveis(a) {
                self.selecionado = Some(i);
                self.campo = self.arquivos[i].clone();
                if duplo {
                    return self.ok();
                }
            }
        }
        RespostaArquivo::Nada
    }

    /// Tecla: Enter = OK, Esc = Cancelar, Backspace apaga.
    pub fn tecla(&mut self, nome: &str, cfg: &ConfigDialogos) -> RespostaArquivo {
        match nome {
            "Return" | "Keypad Enter" => return self.ok(),
            "Escape" => return RespostaArquivo::Cancelar,
            "Backspace" => {
                if self.campo == cfg.arquivo.campo_inicial {
                    self.campo.clear();
                }
                self.campo.pop();
            }
            _ => {}
        }
        RespostaArquivo::Nada
    }

    /// Texto digitado (eventos de texto do SDL). O texto inicial vem selecionado: digitar o troca.
    pub fn digitar(&mut self, texto: &str, cfg: &ConfigDialogos) {
        if self.campo == cfg.arquivo.campo_inicial {
            self.campo.clear();
        }
        for c in texto.chars().filter(|c| c.is_ascii_graphic()) {
            if self.campo.len() < 64 {
                self.campo.push(c);
            }
        }
    }

    fn caminho_absoluto(&self) -> PathBuf {
        std::path::absolute(&self.pasta).unwrap_or_else(|_| self.pasta.clone())
    }

    /// Pastas mostradas: os ancestrais da pasta (da raiz até ela, abertos) e as subpastas dela.
    fn pastas(&self) -> Vec<(usize, String, bool)> {
        let caminho = self.caminho_absoluto();
        let mut v: Vec<(usize, String, bool)> = Vec::new();
        for comp in caminho.components() {
            let nome = match comp {
                std::path::Component::Prefix(p) => format!("{}\\", p.as_os_str().to_string_lossy()),
                std::path::Component::RootDir if !v.is_empty() => continue,
                std::path::Component::RootDir => "\\".into(),
                c => c.as_os_str().to_string_lossy().into_owned(),
            };
            v.push((v.len(), nome.to_lowercase(), true));
        }
        let nivel = v.len();
        let mut sub: Vec<String> = std::fs::read_dir(&caminho)
            .map(|it| it.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().to_lowercase()).collect())
            .unwrap_or_default();
        sub.sort();
        v.extend(sub.into_iter().map(|n| (nivel, n, false)));
        v
    }

    pub fn desenhar(&self, q: &mut Quadro, interface: &Interface, cfg: &ConfigDialogos) {
        let c = &cfg.cores;
        let a = &cfg.arquivo;
        let e = ret(a.externo);
        // Moldura de diálogo: contorno preto, borda azul e o interior branco.
        q.preencher(e, c.moldura, c.moldura);
        q.contorno(Retangulo { dir: e.dir - 1, base: e.base - 1, ..e }, c.linha);
        let [me, mt, md, mb] = a.moldura;
        let i = Retangulo { esq: e.esq + me, topo: e.topo + mt, dir: e.dir - md, base: e.base - mb };
        q.preencher(i, c.fundo, c.fundo);
        let t = Retangulo { esq: i.esq + 1, topo: i.topo + 1, dir: i.dir - 1, base: i.topo + 1 + a.altura_titulo };
        titulo(q, fonte(&cfg.fontes_titulo, interface), c, t, if self.salvar { &a.titulo_salvar } else { &a.titulo_abrir });
        let Some(f) = fonte(&a.fontes, interface) else { return };
        let rotulos = if self.salvar { &a.rotulos_salvar } else { &a.rotulos_abrir };
        for (texto, [x, y]) in rotulos {
            escrever(q, f, i32::from(*x), i32::from(*y), texto, c.texto);
        }
        // Campo do nome; o texto inicial vem selecionado.
        let campo = ret(a.campo);
        caixa_branca(q, c, campo);
        let (tx, ty) = (i32::from(campo.esq) + 3, (i32::from(campo.topo) + i32::from(campo.base) - f.altura() as i32) / 2);
        if self.campo == a.campo_inicial {
            let w = largura(f, &self.campo);
            let r = Retangulo { esq: tx as i16, topo: ty as i16, dir: (tx + w) as i16, base: (ty + f.altura() as i32) as i16 + 1 };
            q.preencher(r, c.selecao, c.selecao);
            escrever(q, f, tx, ty, &self.campo, c.texto_selecao);
        } else {
            escrever(q, f, tx, ty, &self.campo, c.texto);
        }
        // Lista de arquivos (cinza no Save As, como no original).
        let lista = ret(a.lista);
        caixa_branca(q, c, lista);
        rolagem(q, c, lista, a.largura_rolagem);
        let cor = if self.salvar { c.desabilitado } else { c.texto };
        for (k, nome) in self.arquivos.iter().skip(self.primeiro).take(self.linhas_visiveis(a)).enumerate() {
            let y = i32::from(lista.topo) + 1 + k as i32 * i32::from(a.altura_item);
            let x = i32::from(lista.esq) + 5;
            if Some(k + self.primeiro) == self.selecionado {
                let r = Retangulo { esq: lista.esq + 1, topo: y as i16, dir: lista.dir - a.largura_rolagem, base: y as i16 + a.altura_item };
                q.preencher(r, c.selecao, c.selecao);
                escrever(q, f, x, y, nome, c.texto_selecao);
            } else {
                escrever(q, f, x, y, nome, cor);
            }
        }
        // Pastas: o caminho e a árvore (ícones de pasta próprios, simplificados).
        let [cx, cy] = a.caminho;
        let caminho = self.caminho_absoluto().display().to_string().to_lowercase();
        let pastas = ret(a.pastas);
        let antes = q.recorte.replace(Retangulo { esq: cx, topo: cy, dir: pastas.dir, base: pastas.topo });
        escrever(q, f, i32::from(cx), i32::from(cy), &caminho, c.texto);
        q.recorte = antes;
        caixa_branca(q, c, pastas);
        rolagem(q, c, pastas, a.largura_rolagem);
        let dentro = Retangulo { esq: pastas.esq + 1, topo: pastas.topo + 1, dir: pastas.dir - a.largura_rolagem, base: pastas.base - 1 };
        let antes = q.recorte.replace(dentro);
        for (k, (nivel, nome, aberta)) in self.pastas().iter().enumerate() {
            let y = pastas.topo + 1 + k as i16 * a.altura_item_pasta;
            let x = pastas.esq + 5 + *nivel as i16 * a.recuo_pasta;
            let r = Retangulo { esq: x, topo: y + 3, dir: x + 16, base: y + 13 };
            q.preencher(r, c.pasta, c.pasta);
            q.contorno(Retangulo { dir: r.dir - 1, base: r.base - 1, ..r }, c.linha);
            if *aberta {
                q.linha(i32::from(x) + 2, i32::from(y) + 7, i32::from(x) + 15, i32::from(y) + 7, c.linha);
            }
            escrever(q, f, i32::from(x + a.texto_pasta), i32::from(y), nome, c.texto);
        }
        q.recorte = antes;
        // Combinações: tipo e unidade.
        for (caixa, texto) in [(a.tipo, &a.texto_tipo), (a.unidades, &a.texto_unidade)] {
            let r = ret(caixa);
            caixa_branca(q, c, r);
            let x = r.dir - a.largura_rolagem;
            q.preencher(Retangulo { esq: x, dir: x + 1, ..r }, c.linha, c.linha);
            let b = Retangulo { esq: x + 1, topo: r.topo + 1, dir: r.dir - 1, base: r.base - 1 };
            relevo(q, c, b, 1);
            seta(q, c, b, true, true, false);
            escrever(q, f, i32::from(r.esq) + 3, (i32::from(r.topo) + i32::from(r.base) - f.altura() as i32) / 2, texto, c.texto);
        }
        botao(q, Some(f), c, ret(a.ok), &a.texto_ok, true);
        botao(q, Some(f), c, ret(a.cancelar), &a.texto_cancelar, false);
        let (texto, [x, y]) = &a.somente_leitura;
        q.preencher(Retangulo { esq: *x, topo: *y, dir: x + 13, base: y + 13 }, c.campo, c.campo);
        q.contorno(Retangulo { esq: *x, topo: *y, dir: x + 12, base: y + 12 }, c.linha);
        escrever(q, f, i32::from(*x) + 18, i32::from(*y) + 1, texto, c.texto);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cfg() -> ConfigDialogos {
        serde_json::from_str(include_str!("../../../data/dialogos.json")).unwrap()
    }

    #[test]
    fn ok_com_curinga_nao_escolhe_e_nome_vira_caminho() {
        let c = cfg();
        let mut d = DialogoArquivo::new(false, Path::new("nao_existe"), &c);
        assert_eq!(d.campo, "*.ant");
        assert_eq!(d.tecla("Return", &c), RespostaArquivo::Nada);
        d.digitar("TESTE", &c);
        assert_eq!(d.campo, "TESTE");
        let esperado = RespostaArquivo::Escolher(Path::new("nao_existe").join("TESTE.ANT"));
        assert_eq!(d.tecla("Return", &c), esperado);
        assert_eq!(d.tecla("Escape", &c), RespostaArquivo::Cancelar);
        let ok = ret(c.arquivo.ok);
        assert_eq!(d.clicar(&c, i32::from(ok.esq) + 5, i32::from(ok.topo) + 5, false), esperado);
    }

    #[test]
    fn caixa_de_mensagem_do_load_tem_a_geometria_medida() {
        let c = cfg();
        let m = CaixaMensagem::pergunta_load(&c);
        let b = m.geometria(&c.caixa_mensagem, None).botoes;
        assert_eq!(b.len(), 3);
        assert_eq!((b[1].esq - b[0].esq, b[2].esq - b[0].esq), (74, 150));
        assert_eq!(m.tecla("Return"), Some(1));
        assert_eq!(m.tecla("Escape"), Some(2));
        assert_eq!(m.tecla("Y"), Some(0));
        assert_eq!(m.tecla("n"), Some(1));
    }
}
