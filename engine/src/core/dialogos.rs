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

/// "Load Ant Hill" / "Save Ant Hill As" (`_FileSelect`, diálogo comum do Windows).
#[derive(Clone, Debug)]
pub struct DialogoArquivo {
    pub salvar: bool,
    pub pasta: PathBuf,
    pub campo: String,
    pub arquivos: Vec<String>,
    pub selecionado: Option<usize>,
    pub primeiro: usize,
    /// Primeiro item visível da árvore de pastas (rola como a lista; [hipótese]: só a lista de
    /// arquivos foi rolada na bancada).
    pub primeira_pasta: usize,
    /// Item realçado na árvore de pastas (o clique; depois do clique duplo, a pasta nova).
    pub pasta_realcada: Option<usize>,
    /// O texto inicial do campo aparece selecionado só com o foco nele (ao abrir e depois de trocar
    /// de pasta pelo campo); depois de um clique na lista ou na árvore, não [bancada].
    pub foco_no_campo: bool,
}

impl DialogoArquivo {
    pub fn new(salvar: bool, pasta: &Path, cfg: &ConfigDialogos) -> DialogoArquivo {
        let mut d = DialogoArquivo {
            salvar,
            pasta: pasta.to_path_buf(),
            campo: String::new(),
            arquivos: Vec::new(),
            selecionado: None,
            primeiro: 0,
            primeira_pasta: 0,
            pasta_realcada: None,
            foco_no_campo: true,
        };
        d.trocar_pasta(pasta, cfg);
        d
    }

    /// Lê os `.ANT` da pasta e volta a lista ao começo, com o campo no texto inicial (`*.ant`).
    fn trocar_pasta(&mut self, pasta: &Path, cfg: &ConfigDialogos) {
        self.pasta = pasta.to_path_buf();
        self.arquivos = arquivos_ant(pasta)
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
            .collect();
        self.campo = cfg.arquivo.campo_inicial.clone();
        self.selecionado = None;
        self.primeiro = 0;
        self.primeira_pasta = 0;
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
        let t = self.campo.trim();
        if t.is_empty() || t.contains('*') || t.contains('?') {
            return None;
        }
        // "a:" sozinho é a raiz da unidade (o original volta à pasta corrente dela; o port não a guarda).
        let t = if t.len() == 2 && t.ends_with(':') { format!("{t}\\") } else { t.to_string() };
        let p = self.pasta.join(&t);
        p.is_dir().then(|| std::path::absolute(&p).unwrap_or(p))
    }

    /// OK: uma pasta no campo troca de pasta; senão o nome vira caminho (com curinga, nada).
    fn ok(&mut self, cfg: &ConfigDialogos) -> RespostaArquivo {
        if let Some(p) = self.pasta_do_campo() {
            self.trocar_pasta(&p, cfg);
            self.pasta_realcada = None;
            self.foco_no_campo = true;
            return RespostaArquivo::Nada;
        }
        if self.campo.contains('*') || self.campo.contains('?') {
            return RespostaArquivo::Nada;
        }
        match caminho_do_nome(&self.pasta, &self.campo) {
            Some(p) => RespostaArquivo::Escolher(p),
            None => RespostaArquivo::Nada,
        }
    }

    /// Clique em (x, y); `duplo` = clique duplo (na lista, escolhe o arquivo; na árvore, abre a
    /// pasta). Setas e trilho da barra da lista rolam um item ou uma página (os visíveis − 1).
    pub fn clicar(&mut self, cfg: &ConfigDialogos, x: i32, y: i32, duplo: bool) -> RespostaArquivo {
        let a = &cfg.arquivo;
        if contem(ret(a.ok), x, y) {
            return self.ok(cfg);
        }
        if contem(ret(a.cancelar), x, y) {
            return RespostaArquivo::Cancelar;
        }
        let lista = ret(a.lista);
        let barra = Retangulo { esq: lista.dir - a.largura_rolagem, ..lista };
        let max = self.primeiro_maximo(a);
        if contem(barra, x, y) && max > 0 {
            let p = PartesRolagem::de(lista, a.largura_rolagem);
            self.primeiro = p.rolar(self.primeiro, max, self.linhas_visiveis(a), y);
            return RespostaArquivo::Nada;
        }
        let pastas = ret(a.pastas);
        let max_pastas = self.pastas().len().saturating_sub(self.pastas_visiveis(a));
        if contem(Retangulo { esq: pastas.dir - a.largura_rolagem, ..pastas }, x, y) && max_pastas > 0 {
            let p = PartesRolagem::de(pastas, a.largura_rolagem);
            self.primeira_pasta = p.rolar(self.primeira_pasta, max_pastas, self.pastas_visiveis(a), y);
            return RespostaArquivo::Nada;
        }
        if contem(Retangulo { dir: lista.dir - a.largura_rolagem, ..lista }, x, y) {
            let k = ((y - i32::from(lista.topo) - 1) / i32::from(a.altura_item)).max(0) as usize;
            let i = k + self.primeiro;
            if i < self.arquivos.len() && k < self.linhas_visiveis(a) {
                self.selecionado = Some(i);
                self.campo = self.arquivos[i].clone();
                self.foco_no_campo = false;
                if duplo {
                    return self.ok(cfg);
                }
            }
        }
        if contem(Retangulo { dir: pastas.dir - a.largura_rolagem, ..pastas }, x, y) {
            let v = ((y - i32::from(pastas.topo) - 1) / i32::from(a.altura_item_pasta)).max(0) as usize;
            let k = v + self.primeira_pasta;
            let lista_pastas = self.pastas();
            if let Some(item) = lista_pastas.get(k).filter(|_| v < self.pastas_visiveis(a)) {
                self.pasta_realcada = Some(k);
                self.foco_no_campo = false;
                if duplo {
                    let destino = item.caminho.clone();
                    self.trocar_pasta(&destino, cfg);
                    // A pasta aberta fica realçada: é o último ancestral da lista nova.
                    self.pasta_realcada = self.pastas().iter().rposition(|p| p.aberta);
                }
            }
        }
        RespostaArquivo::Nada
    }

    /// Tecla: Enter = OK, Esc = Cancelar, Backspace apaga.
    pub fn tecla(&mut self, nome: &str, cfg: &ConfigDialogos) -> RespostaArquivo {
        match nome {
            "Return" | "Keypad Enter" => return self.ok(cfg),
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
        self.foco_no_campo = true;
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
        // Campo do nome; o texto inicial vem selecionado (com o foco no campo).
        let campo = ret(a.campo);
        caixa_branca(q, c, campo);
        let (tx, ty) = (i32::from(campo.esq) + 3, (i32::from(campo.topo) + i32::from(campo.base) - f.altura() as i32) / 2);
        if self.campo == a.campo_inicial && self.foco_no_campo {
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
        rolagem(q, c, lista, a.largura_rolagem, Some((self.primeiro, self.primeiro_maximo(a))));
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
        }
        q.recorte = antes;
        // Combinações: tipo e unidade (a letra da unidade da pasta, quando o caminho tem uma).
        let unidade = match self.caminho_absoluto().components().next() {
            Some(std::path::Component::Prefix(p)) => p.as_os_str().to_string_lossy().to_lowercase(),
            _ => a.texto_unidade.clone(),
        };
        for (caixa, texto) in [(a.tipo, &a.texto_tipo), (a.unidades, &unidade)] {
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
        serde_json::from_str(&crate::recursos::marcar_referencias(include_str!("../../../data/dialogos.json"))).unwrap()
    }

    #[test]
    fn ok_com_curinga_nao_escolhe_e_nome_vira_caminho() {
        // O campo inicial vem do SIMANTW.EXE (referência); aqui, um curinga qualquer.
        let mut c = cfg();
        c.arquivo.campo_inicial = "*.ant".into();
        let mut d = DialogoArquivo::new(false, Path::new("nao_existe"), &c);
        assert_eq!(d.campo, c.arquivo.campo_inicial, "o campo volta ao texto inicial");
        assert_eq!(d.tecla("Return", &c), RespostaArquivo::Nada);
        d.digitar("TESTE", &c);
        assert_eq!(d.campo, "TESTE");
        let esperado = RespostaArquivo::Escolher(Path::new("nao_existe").join("TESTE.ANT"));
        assert_eq!(d.tecla("Return", &c), esperado);
        assert_eq!(d.tecla("Escape", &c), RespostaArquivo::Cancelar);
        let ok = ret(c.arquivo.ok);
        assert_eq!(d.clicar(&c, i32::from(ok.esq) + 5, i32::from(ok.topo) + 5, false), esperado);
    }

    /// Bancada (2026-10-19, `qemu_dialogo_muitos.txt`, 18 arquivos, 8 visíveis): seta de baixo duas
    /// vezes (primeiro 2, polegar 11 linhas abaixo da seta de cima), trilho abaixo do polegar (9:
    /// uma página de 7), seta de cima (8, polegar 46 abaixo; arredondado) e o clique num item.
    #[test]
    fn lista_com_muitos_arquivos_rola_como_o_original() {
        let c = cfg();
        let mut d = DialogoArquivo::new(false, Path::new("nao_existe"), &c);
        d.arquivos = (0..18).map(|i| format!("a{i:02}.ant")).collect();
        let a = &c.arquivo;
        let lista = ret(a.lista);
        let p = PartesRolagem::de(lista, a.largura_rolagem);
        let x = i32::from(lista.dir) - 8;
        let (cima, baixo) = (i32::from(lista.topo) + 8, i32::from(lista.base) - 8);
        assert_eq!(d.primeiro_maximo(a), 10);
        assert_eq!(p.linha_polegar(0, 10), p.cima.base);
        d.clicar(&c, x, baixo, false);
        d.clicar(&c, x, baixo, false);
        assert_eq!(d.primeiro, 2);
        assert_eq!(p.linha_polegar(2, 10) - p.cima.base, 11);
        d.clicar(&c, x, i32::from(lista.topo) + 77, false);
        assert_eq!(d.primeiro, 9);
        assert_eq!(p.linha_polegar(9, 10) - p.cima.base, 51);
        d.clicar(&c, x, cima, false);
        assert_eq!(d.primeiro, 8);
        assert_eq!(p.linha_polegar(8, 10) - p.cima.base, 46);
        assert_eq!(p.linha_polegar(10, 10) + p.lado() + 1, p.baixo.topo - 1, "no fim, o polegar encosta na seta de baixo");
        d.clicar(&c, i32::from(lista.esq) + 40, i32::from(lista.topo) + 1 + 13 + 6, false);
        assert_eq!((d.selecionado, d.campo.as_str(), d.foco_no_campo), (Some(9), "a09.ant", false));
        for _ in 0..5 {
            d.clicar(&c, x, baixo, false);
        }
        assert_eq!(d.primeiro, 10, "não passa do fim");
    }

    /// Um nome de pasta no campo e Enter troca de pasta (o original troca de unidade com "a:").
    #[test]
    fn pasta_no_campo_troca_de_pasta() {
        let c = cfg();
        let raiz = std::env::temp_dir().join(format!("simant_dlg_{}", std::process::id()));
        std::fs::create_dir_all(raiz.join("sub")).unwrap();
        std::fs::write(raiz.join("sub").join("x.ant"), b"").unwrap();
        let mut d = DialogoArquivo::new(false, &raiz, &c);
        assert!(d.arquivos.is_empty());
        d.digitar("sub", &c);
        assert_eq!(d.tecla("Return", &c), RespostaArquivo::Nada);
        assert_eq!(d.arquivos, vec!["x.ant".to_string()]);
        assert_eq!(d.campo, c.arquivo.campo_inicial, "o campo volta ao texto inicial");
        assert!(d.pastas().iter().any(|p| p.aberta && p.nome == "sub"));
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
