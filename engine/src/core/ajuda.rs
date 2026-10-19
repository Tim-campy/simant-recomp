//! Visor de ajuda (o WinHelp 3.x que o original chama com `WinHelp(…, "SIMANT.HLP", …)`), sem SDL:
//! tópico atual, histórico (Back), sequência de navegação (<< >>), rolagem, layout dos parágrafos
//! com as fontes do port, desenho num `Quadro` e teste de clique.
//!
//! Especificação: docs/spec/telas/ajuda.md. Formato do arquivo: docs/spec/formatos/hlp.md
//! (`formats::hlp`). Aparência e textos: data/ajuda.json.

use std::collections::HashMap;

use serde::Deserialize;

use super::gerenciador::{desenhar_seta, inicio_do_polegar, parte_da_barra, ParteBarra, Rolagem, Seta};
use super::quadro::Quadro;
use super::telas::Interface;
use crate::formats::bitmap::Bitmap;
use crate::formats::fonte::Fonte;
use super::ajuda_janelas::{JanelaHistorico, Sobreposta};
use crate::formats::hlp::{Alinhamento, ArquivoAjuda, Item, PalavraChave, Salto, Topico};
use crate::formats::janela::Retangulo;
use crate::recursos::Recursos;

pub const ARQUIVO_CONFIG: &str = "ajuda.json";

/// Fonte do arquivo de ajuda (meios pontos e negrito) → índice de fonte do port (`interface.json`).
#[derive(Clone, Debug, Deserialize)]
pub struct MapaFonte {
    pub meios_pontos: u8,
    pub negrito: bool,
    pub fonte: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcaoBotao {
    Conteudo,
    Busca,
    Voltar,
    Historico,
    Anterior,
    Seguinte,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BotaoAjuda {
    /// Texto com `&` antes da letra sublinhada.
    pub texto: String,
    pub acao: AcaoBotao,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ItemMenuAjuda {
    pub texto: String,
    /// Coluna da célula do texto a partir da borda interna esquerda.
    pub x: i16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CoresAjuda {
    pub linha: u8,
    pub moldura: u8,
    pub titulo: u8,
    pub texto_titulo: u8,
    pub menu: u8,
    pub texto_menu: u8,
    pub botao: u8,
    pub relevo_claro: u8,
    pub relevo_escuro: u8,
    pub fundo_botoes: u8,
    pub fundo: u8,
    pub texto: u8,
    pub salto: u8,
    pub trilho: u8,
}

/// Conteúdo de data/ajuda.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigAjuda {
    /// Arquivo de ajuda na pasta do jogo.
    pub arquivo: String,
    /// Texto do título da janela.
    pub titulo: String,
    /// Contorno externo da janela na área cliente do port [esq, topo, dir, base) .
    pub externo: [i16; 4],
    pub espessura_moldura: i16,
    pub altura_titulo: i16,
    pub altura_menu: i16,
    /// Altura da barra de botões sem a linha preta de baixo.
    pub altura_botoes: i16,
    pub largura_botao: i16,
    /// Topo da célula do texto dos botões a partir do topo do botão.
    pub topo_texto_botao: i16,
    pub menu: Vec<ItemMenuAjuda>,
    pub botoes: Vec<BotaoAjuda>,
    /// Coluna do texto do tópico a partir da borda interna esquerda.
    pub margem_texto: i16,
    /// Passo das tabulações sem tabulação definida no parágrafo [hipótese].
    pub passo_tab: i32,
    /// Largura da barra de rolagem vertical (com a linha preta da esquerda).
    pub largura_barra: i16,
    /// Pixels por clique nas setas da barra e por linha das teclas.
    pub passo_rolagem: i32,
    /// Fonte do título e do menu (System do Windows), dos botões e de reserva.
    pub fonte_sistema: usize,
    pub fonte_botoes: usize,
    pub fonte_padrao: usize,
    pub fontes: Vec<MapaFonte>,
    /// `WinHelp(…, HELP_CONTEXT, n)` do índice (F1 no original abre o índice com HELP_INDEX).
    pub contexto_indice: u32,
    pub cores: CoresAjuda,
    /// Menus, Search, History e popups (core/ajuda_janelas.rs). Opcional (API 0.24): sem ele, os
    /// botões Search e History não fazem nada e os popups trocam o tópico, como antes.
    #[serde(default)]
    pub janelas: Option<super::ajuda_janelas::ConfigJanelasAjuda>,
}

/// O que a interface faz depois de uma entrada no visor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RespostaAjuda {
    /// O ponto não é da janela de ajuda.
    Fora,
    /// Tratado (redesenhar).
    Nada,
    /// Fechar a janela de ajuda.
    Fechar,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Conteudo {
    Texto { fonte: usize, bytes: Vec<u8>, sublinhado: i32 },
    Figura(u16),
}

#[derive(Clone, Debug)]
pub(crate) struct Peca {
    pub(crate) x: i32,
    pub(crate) largura: i32,
    conteudo: Conteudo,
    pub(crate) salto: Option<Salto>,
}

#[derive(Clone, Debug)]
pub(crate) struct Linha {
    pub(crate) topo: i32,
    pub(crate) altura: i32,
    pub(crate) base: i32,
    pub(crate) pecas: Vec<Peca>,
}

/// O visor aberto: o arquivo lido, o tópico e a rolagem.
pub struct VisorAjuda {
    pub cfg: ConfigAjuda,
    pub arquivo: ArquivoAjuda,
    pub(crate) topico: usize,
    pub(crate) historico: Vec<usize>,
    pub(crate) rolagem: i32,
    figuras: HashMap<u16, Bitmap>,
    /// Palavras-chave da busca (|KWBTREE).
    pub(crate) palavras: Vec<PalavraChave>,
    /// Menu, Search ou popup aberto (recebe a entrada).
    pub(crate) sobreposta: Option<Sobreposta>,
    /// Janela History aberta.
    pub(crate) janela_historico: Option<JanelaHistorico>,
    /// Tópicos mostrados, do mais antigo ao mais recente (a lista do History).
    pub(crate) visitados: Vec<usize>,
}

pub(crate) fn ret(e: i16, t: i16, d: i16, b: i16) -> Retangulo {
    Retangulo { esq: e, topo: t, dir: d, base: b }
}

pub(crate) fn contem(r: Retangulo, x: i32, y: i32) -> bool {
    x >= i32::from(r.esq) && x < i32::from(r.dir) && y >= i32::from(r.topo) && y < i32::from(r.base)
}

fn cor_mais_proxima(c: [u8; 3], cores: &[[u8; 3]]) -> u8 {
    let d = |p: &[u8; 3]| (0..3).map(|k| (i32::from(c[k]) - i32::from(p[k])).pow(2)).sum::<i32>();
    // Só as 16 cores do Windows (as seguintes são cores do sistema do port).
    cores.iter().take(16).enumerate().min_by_key(|(_, p)| d(p)).map_or(0, |(i, _)| i as u8)
}

/// Texto transparente com a célula em (x, y).
pub(crate) fn escrever(q: &mut Quadro, f: &Fonte, x: i32, y: i32, texto: &[u8], cor: u8) {
    let img = f.imagem(texto);
    for j in 0..img.altura {
        for i in 0..img.largura {
            if img.bits[j * img.largura + i] {
                q.ponto(x + i as i32, y + j as i32, cor);
            }
        }
    }
}

/// Texto com `&`: escreve sem o `&` e sublinha a letra seguinte (linha de base + 2). Acinzentado
/// (`GrayString`): só os pixels com x + y par [medido no Back
/// desabilitado de a06].
pub(crate) fn escrever_mnemonico(q: &mut Quadro, f: &Fonte, x: i32, y: i32, texto: &str, cor: u8, cinza: bool) {
    let bytes: Vec<u8> = texto.bytes().filter(|&b| b != b'&').collect();
    let img = f.imagem(&bytes);
    let ponto = |q: &mut Quadro, px: i32, py: i32| {
        if !cinza || (px + py).rem_euclid(2) == 0 {
            q.ponto(px, py, cor);
        }
    };
    for j in 0..img.altura {
        for i in 0..img.largura {
            if img.bits[j * img.largura + i] {
                ponto(q, x + i as i32, y + j as i32);
            }
        }
    }
    if let Some(p) = texto.find('&') {
        let antes = f.largura_texto(&texto.as_bytes()[..p]) as i32;
        let letra = texto.as_bytes().get(p + 1).copied().unwrap_or(b' ');
        // Largura da letra, mais o pixel do negrito simulado.
        let w = f.largura_texto(&[letra]) as i32 + i32::from(f.negrito());
        let yb = y + f.ascendente() as i32 + 1;
        for k in 0..w {
            ponto(q, x + antes + k, yb);
        }
    }
}

pub(crate) fn largura_sem_e(f: &Fonte, texto: &str) -> i32 {
    let bytes: Vec<u8> = texto.bytes().filter(|&b| b != b'&').collect();
    f.largura_texto(&bytes) as i32
}

impl VisorAjuda {
    /// Lê `data/ajuda.json` e o arquivo de ajuda; as figuras viram índices da paleta `cores`.
    pub fn carregar(recursos: &Recursos, cores: &[[u8; 3]]) -> Result<VisorAjuda, String> {
        let cfg: ConfigAjuda = recursos.dados(ARQUIVO_CONFIG)?;
        let arquivo = ArquivoAjuda::ler(recursos.arquivo(&cfg.arquivo)?)?;
        VisorAjuda::novo(cfg, arquivo, cores)
    }

    pub fn novo(cfg: ConfigAjuda, arquivo: ArquivoAjuda, cores: &[[u8; 3]]) -> Result<VisorAjuda, String> {
        let mut figuras = HashMap::new();
        for t in &arquivo.topicos {
            for p in &t.paragrafos {
                for i in &p.itens {
                    if let Item::Figura { numero, .. } = i {
                        if figuras.contains_key(numero) {
                            continue;
                        }
                        let f = arquivo.figura(*numero)?;
                        let pixels = f.rgb.iter().map(|&c| cor_mais_proxima(c, cores)).collect();
                        figuras.insert(*numero, Bitmap { largura: f.largura, altura: f.altura, pixels });
                    }
                }
            }
        }
        let indice = arquivo.topico_do_contexto(cfg.contexto_indice).unwrap_or(0);
        let palavras = arquivo.palavras_chave()?;
        Ok(VisorAjuda {
            cfg,
            arquivo,
            topico: indice,
            historico: Vec::new(),
            rolagem: 0,
            figuras,
            palavras,
            sobreposta: None,
            janela_historico: None,
            visitados: vec![indice],
        })
    }

    pub fn topico_atual(&self) -> &Topico {
        &self.arquivo.topicos[self.topico]
    }

    pub(crate) fn ir_para(&mut self, i: usize) {
        if i != self.topico {
            self.historico.push(self.topico);
        }
        self.topico = i;
        self.rolagem = 0;
        self.visitar(i);
    }

    /// `HELP_CONTEXT n`: vai para o tópico do contexto (false se o contexto não existe; fica onde
    /// estava). O histórico é zerado, como numa chamada nova do WinHelp [hipótese].
    pub fn abrir_contexto(&mut self, n: u32) -> bool {
        match self.arquivo.topico_do_contexto(n) {
            Some(i) => {
                self.topico = i;
                self.historico.clear();
                self.rolagem = 0;
                self.sobreposta = None;
                self.visitar(i);
                true
            }
            None => false,
        }
    }

    /// Vai para o tópico `i` da lista (como um salto: entra no histórico).
    pub fn abrir_topico(&mut self, i: usize) {
        if i < self.arquivo.topicos.len() {
            self.ir_para(i);
        }
    }

    /// `HELP_INDEX` (F1): o índice.
    pub fn abrir_indice(&mut self) {
        self.abrir_contexto(self.cfg.contexto_indice);
    }

    fn habilitado(&self, a: AcaoBotao) -> bool {
        let t = self.topico_atual();
        match a {
            AcaoBotao::Conteudo => true,
            AcaoBotao::Voltar => !self.historico.is_empty(),
            AcaoBotao::Anterior => t.anterior.and_then(|n| self.arquivo.topico_do_numero(n)).is_some(),
            AcaoBotao::Seguinte => t.seguinte.and_then(|n| self.arquivo.topico_do_numero(n)).is_some(),
            AcaoBotao::Busca | AcaoBotao::Historico => true,
        }
    }

    pub(crate) fn acionar(&mut self, a: AcaoBotao) {
        if !self.habilitado(a) {
            return;
        }
        let t = (self.topico_atual().anterior, self.topico_atual().seguinte);
        match a {
            AcaoBotao::Conteudo => {
                if let Some(i) = self.arquivo.topico_do_contexto(self.cfg.contexto_indice) {
                    self.ir_para(i);
                }
            }
            AcaoBotao::Voltar => {
                if let Some(i) = self.historico.pop() {
                    self.topico = i;
                    self.rolagem = 0;
                    self.visitar(i);
                }
            }
            AcaoBotao::Anterior | AcaoBotao::Seguinte => {
                let n = if a == AcaoBotao::Anterior { t.0 } else { t.1 };
                if let Some(i) = n.and_then(|n| self.arquivo.topico_do_numero(n)) {
                    self.ir_para(i);
                }
            }
            AcaoBotao::Busca => self.abrir_busca(),
            AcaoBotao::Historico => self.abrir_historico(),
        }
    }

    // ------------------------------------------------------------------------------------------
    // Geometria
    // ------------------------------------------------------------------------------------------

    pub fn externo(&self) -> Retangulo {
        let [e, t, d, b] = self.cfg.externo;
        ret(e, t, d, b)
    }

    pub(crate) fn interno(&self) -> Retangulo {
        let e = self.externo();
        let k = self.cfg.espessura_moldura;
        ret(e.esq + k, e.topo + k, e.dir - k, e.base - k)
    }

    pub(crate) fn faixa_titulo(&self) -> Retangulo {
        let i = self.interno();
        ret(i.esq, i.topo, i.dir, i.topo + self.cfg.altura_titulo)
    }

    pub(crate) fn faixa_menu(&self) -> Retangulo {
        let t = self.faixa_titulo();
        ret(t.esq, t.base + 1, t.dir, t.base + 1 + self.cfg.altura_menu)
    }

    /// Faixa dos botões, da linha preta de cima (topo) até a de baixo (base − 1).
    fn faixa_botoes(&self) -> Retangulo {
        let m = self.faixa_menu();
        ret(m.esq, m.base + 1, m.dir, m.base + 1 + self.cfg.altura_botoes + 1)
    }

    fn botao(&self, k: usize) -> Retangulo {
        let f = self.faixa_botoes();
        let x0 = f.esq + k as i16 * self.cfg.largura_botao;
        ret(x0, f.topo, x0 + self.cfg.largura_botao, f.base)
    }

    /// Área do texto do tópico (com a barra, se houver).
    pub(crate) fn area_texto(&self) -> Retangulo {
        let b = self.faixa_botoes();
        let i = self.interno();
        ret(i.esq, b.base, i.dir, i.base)
    }

    pub(crate) fn caixa_menu_sistema(&self) -> Retangulo {
        let t = self.faixa_titulo();
        ret(t.esq, t.topo, t.esq + self.cfg.altura_titulo, t.base)
    }

    /// O ponto está na janela de ajuda ou numa janela dela (popup, Search, History).
    pub fn contem(&self, x: i32, y: i32) -> bool {
        contem(self.externo(), x, y) || self.contem_janelas(x, y)
    }

    pub(crate) fn fonte<'a>(&self, interface: &'a Interface, i: usize) -> Option<&'a Fonte> {
        interface
            .fontes
            .get(i)
            .and_then(|f| f.as_ref())
            .or_else(|| interface.fontes.get(self.cfg.fonte_padrao).and_then(|f| f.as_ref()))
            .or_else(|| interface.fontes.iter().flatten().next())
    }

    fn fonte_do_arquivo(&self, n: u16) -> usize {
        let Some(f) = self.arquivo.fontes.get(n as usize) else { return self.cfg.fonte_padrao };
        self.cfg
            .fontes
            .iter()
            .find(|m| m.meios_pontos == f.meios_pontos && m.negrito == f.negrito)
            .or_else(|| self.cfg.fontes.iter().filter(|m| m.negrito == f.negrito).min_by_key(|m| (i32::from(m.meios_pontos) - i32::from(f.meios_pontos)).abs()))
            .map_or(self.cfg.fonte_padrao, |m| m.fonte)
    }

    // ------------------------------------------------------------------------------------------
    // Layout
    // ------------------------------------------------------------------------------------------

    /// Linhas do tópico atual para a largura útil `largura` (a partir da margem).
    fn layout(&self, interface: &Interface, largura: i32) -> Vec<Linha> {
        self.layout_topico(interface, self.topico, i32::from(self.cfg.margem_texto), largura)
    }

    /// Linhas do tópico `topico` com a margem e a largura útil dadas.
    pub(crate) fn layout_topico(&self, interface: &Interface, topico: usize, margem: i32, largura: i32) -> Vec<Linha> {
        let mut linhas: Vec<Linha> = Vec::new();
        let mut y = 0;
        let metricas = |fi: usize| self.fonte(interface, fi).map_or((16, 13), |f| (f.altura() as i32, f.ascendente() as i32));
        for p in &self.arquivo.topicos[topico].paragrafos {
            let mut atual: Vec<Peca> = Vec::new();
            let mut x = margem;
            let mut fonte_linha = self.fonte_do_arquivo(p.fonte_final);
            let fechar = |pecas: &mut Vec<Peca>, y: &mut i32, linhas: &mut Vec<Linha>, fonte_vazia: usize| {
                let (mut alt, mut base) = (0, 0);
                for pc in pecas.iter() {
                    match &pc.conteudo {
                        Conteudo::Texto { fonte, .. } => {
                            let (h, a) = metricas(*fonte);
                            base = base.max(a);
                            alt = alt.max(h - a);
                        }
                        Conteudo::Figura(n) => {
                            let h = self.figuras.get(n).map_or(0, |b| b.altura as i32);
                            base = base.max(h);
                        }
                    }
                }
                if pecas.is_empty() {
                    let (h, a) = metricas(fonte_vazia);
                    base = a;
                    alt = h - a;
                }
                if p.alinhamento != Alinhamento::Esquerda {
                    let usado = pecas.last().map_or(margem, |pc| pc.x + pc.largura) - margem;
                    let folga = (largura - usado).max(0);
                    let d = if p.alinhamento == Alinhamento::Centro { folga / 2 } else { folga };
                    for pc in pecas.iter_mut() {
                        pc.x += d;
                    }
                }
                linhas.push(Linha { topo: *y, altura: base + alt, base, pecas: std::mem::take(pecas) });
                *y += base + alt;
            };
            for item in &p.itens {
                match item {
                    Item::Texto { fonte, texto, salto } => {
                        let fi = self.fonte_do_arquivo(*fonte);
                        fonte_linha = fi;
                        let Some(f) = self.fonte(interface, fi) else { continue };
                        for palavra in palavras(texto) {
                            let sem_espaco = palavra.iter().rposition(|&c| c != b' ').map_or(0, |k| k + 1);
                            let w = f.largura_texto(palavra) as i32;
                            let w_vis = f.largura_texto(&palavra[..sem_espaco]) as i32;
                            if x + w_vis > margem + largura && !atual.is_empty() {
                                fechar(&mut atual, &mut y, &mut linhas, fi);
                                x = margem;
                                if sem_espaco == 0 {
                                    continue;
                                }
                            }
                            match atual.last_mut() {
                                Some(Peca { conteudo: Conteudo::Texto { fonte: pf, bytes, sublinhado }, largura: pw, salto: ps, x: px })
                                    if *pf == fi && *ps == *salto && *px + *pw == x =>
                                {
                                    bytes.extend_from_slice(palavra);
                                    *sublinhado = *pw + w_vis;
                                    *pw += w;
                                }
                                _ => atual.push(Peca { x, largura: w, conteudo: Conteudo::Texto { fonte: fi, bytes: palavra.to_vec(), sublinhado: w_vis }, salto: *salto }),
                            }
                            x += w;
                        }
                    }
                    Item::Quebra => {
                        fechar(&mut atual, &mut y, &mut linhas, fonte_linha);
                        x = margem;
                    }
                    Item::Tab => {
                        let passo = self.cfg.passo_tab.max(1);
                        let tabs: Vec<i32> = p.tabs.iter().map(|&t| i32::from(t)).collect();
                        x = tabs.iter().map(|&t| margem + t).find(|&t| t > x).unwrap_or(margem + ((x - margem) / passo + 1) * passo);
                    }
                    Item::Figura { numero, salto, .. } => {
                        let w = self.figuras.get(numero).map_or(0, |b| b.largura as i32);
                        if x + w > margem + largura && !atual.is_empty() {
                            fechar(&mut atual, &mut y, &mut linhas, fonte_linha);
                            x = margem;
                        }
                        atual.push(Peca { x, largura: w, conteudo: Conteudo::Figura(*numero), salto: *salto });
                        x += w;
                    }
                }
            }
            fechar(&mut atual, &mut y, &mut linhas, fonte_linha);
        }
        linhas
    }

    /// Linhas e se a barra de rolagem aparece (o texto não cabe na área).
    fn layout_com_barra(&self, interface: &Interface) -> (Vec<Linha>, bool) {
        let a = self.area_texto();
        let largura = i32::from(a.dir - a.esq) - 2 * i32::from(self.cfg.margem_texto);
        let linhas = self.layout(interface, largura);
        let total = linhas.last().map_or(0, |l| l.topo + l.altura);
        if total <= i32::from(a.base - a.topo) {
            return (linhas, false);
        }
        (self.layout(interface, largura - i32::from(self.cfg.largura_barra)), true)
    }

    fn barra(&self) -> Retangulo {
        let a = self.area_texto();
        ret(a.dir - self.cfg.largura_barra + 1, a.topo, a.dir, a.base)
    }

    fn rolagem_maxima(&self, linhas: &[Linha]) -> i32 {
        let a = self.area_texto();
        let total = linhas.last().map_or(0, |l| l.topo + l.altura);
        (total - i32::from(a.base - a.topo)).max(0)
    }

    /// Rola `pixels` (positivo = para baixo), dentro do texto.
    pub fn rolar(&mut self, interface: &Interface, pixels: i32) {
        let (linhas, _) = self.layout_com_barra(interface);
        let max = self.rolagem_maxima(&linhas);
        self.rolagem = (self.rolagem + pixels).clamp(0, max);
    }

    // ------------------------------------------------------------------------------------------
    // Entrada
    // ------------------------------------------------------------------------------------------

    /// Clique esquerdo em (x, y) da área cliente do port.
    pub fn clicar(&mut self, interface: &Interface, x: i32, y: i32, duplo: bool) -> RespostaAjuda {
        // Com popup ou menu aberto, qualquer clique é deles (fecham sem seguir o clique).
        let prende = matches!(self.sobreposta, Some(Sobreposta::Popup { .. }) | Some(Sobreposta::Menu { .. }));
        if !self.contem(x, y) && !prende {
            if let Some(h) = self.janela_historico.as_mut() {
                h.foco = false;
            }
            return RespostaAjuda::Fora;
        }
        if let Some(sair) = self.clicar_janelas(x, y, duplo) {
            return if sair { RespostaAjuda::Fechar } else { RespostaAjuda::Nada };
        }
        if matches!(self.sobreposta, Some(Sobreposta::Busca(_))) || !contem(self.externo(), x, y) {
            return RespostaAjuda::Nada;
        }
        if contem(self.caixa_menu_sistema(), x, y) {
            // Duplo clique na caixa do menu de sistema fecha (o menu em si não é desenhado).
            return if duplo { RespostaAjuda::Fechar } else { RespostaAjuda::Nada };
        }
        for (k, b) in self.cfg.botoes.clone().iter().enumerate() {
            if contem(self.botao(k), x, y) {
                self.acionar(b.acao);
                return RespostaAjuda::Nada;
            }
        }
        let a = self.area_texto();
        if !contem(a, x, y) {
            return RespostaAjuda::Nada;
        }
        let (linhas, com_barra) = self.layout_com_barra(interface);
        if com_barra && contem(self.barra(), x, y) {
            let max = self.rolagem_maxima(&linhas);
            let pagina = i32::from(a.base - a.topo);
            let r = Rolagem { pos: self.rolagem, max };
            let lado = self.cfg.largura_barra - 1;
            let passo = match parte_da_barra(self.barra(), false, lado, r, y) {
                ParteBarra::SetaMenos => -self.cfg.passo_rolagem,
                ParteBarra::SetaMais => self.cfg.passo_rolagem,
                ParteBarra::PaginaMenos => -pagina,
                ParteBarra::PaginaMais => pagina,
                ParteBarra::Polegar => 0,
            };
            self.rolagem = (self.rolagem + passo).clamp(0, max);
            return RespostaAjuda::Nada;
        }
        let dy = y - i32::from(a.topo) + self.rolagem;
        let dx = x - i32::from(a.esq);
        let alvo = linhas.iter().find(|l| dy >= l.topo && dy < l.topo + l.altura).and_then(|l| {
            l.pecas.iter().find(|p| p.salto.is_some() && dx >= p.x && dx < p.x + p.largura).and_then(|p| {
                // Ponto quente: a peça; uma figura vai até a linha de base (onde ela termina).
                let altura = if matches!(p.conteudo, Conteudo::Figura(_)) { l.base } else { l.altura };
                p.salto.map(|s| (s, l.topo, altura, p.x, p.largura))
            })
        });
        if let Some((s, topo, altura, px, pl)) = alvo {
            if let Some(i) = self.arquivo.topico_do_numero(s.numero_topico) {
                if s.popup && self.cfg.janelas.is_some() {
                    // Popup (0xE0): janela pequena abaixo do ponto quente (a peça na linha).
                    let (ax, ay) = (i32::from(a.esq), i32::from(a.topo) - self.rolagem);
                    let quente = ret((ax + px) as i16, (ay + topo) as i16, (ax + px + pl) as i16, (ay + topo + altura) as i16);
                    self.abrir_popup(interface, i, quente);
                } else {
                    self.ir_para(i);
                }
            }
        }
        RespostaAjuda::Nada
    }

    /// Tecla (nome do SDL). Esc fecha; setas, PageUp/PageDown, Home/End rolam; Backspace = Back;
    /// a letra sublinhada de um botão (com ou sem Alt) aciona o botão.
    pub fn tecla(&mut self, interface: &Interface, nome: &str) -> RespostaAjuda {
        self.tecla_com(interface, nome, false, false)
    }

    /// Como [`VisorAjuda::tecla`], com Alt (abre os menus pela letra) e Shift (maiúsculas no Search).
    pub fn tecla_com(&mut self, interface: &Interface, nome: &str, alt: bool, shift: bool) -> RespostaAjuda {
        if let Some(sair) = self.tecla_janelas(nome, alt, shift) {
            return if sair { RespostaAjuda::Fechar } else { RespostaAjuda::Nada };
        }
        let a = self.area_texto();
        let pagina = i32::from(a.base - a.topo);
        match nome {
            "Escape" => return RespostaAjuda::Fechar,
            "Up" => self.rolar(interface, -self.cfg.passo_rolagem),
            "Down" => self.rolar(interface, self.cfg.passo_rolagem),
            "PageUp" => self.rolar(interface, -pagina),
            "PageDown" => self.rolar(interface, pagina),
            "Home" => self.rolagem = 0,
            "End" => self.rolar(interface, i32::MAX / 2),
            "Backspace" => self.acionar(AcaoBotao::Voltar),
            outro => {
                let tecla = outro.to_lowercase();
                let botao = self.cfg.botoes.iter().find(|b| {
                    b.texto.find('&').and_then(|p| b.texto[p + 1..].chars().next()).is_some_and(|c| c.to_lowercase().to_string() == tecla)
                });
                if let Some(b) = botao.map(|b| b.acao) {
                    self.acionar(b);
                }
            }
        }
        RespostaAjuda::Nada
    }

    // ------------------------------------------------------------------------------------------
    // Desenho
    // ------------------------------------------------------------------------------------------

    pub fn desenhar(&self, q: &mut Quadro, interface: &Interface) {
        let c = &self.cfg.cores;
        let antes = q.recorte.take();
        let e = self.externo();
        let k = self.cfg.espessura_moldura;
        let i = self.interno();
        let (ex, ey, ed, eb) = (i32::from(e.esq), i32::from(e.topo), i32::from(e.dir) - 1, i32::from(e.base) - 1);
        // Moldura grossa: contorno preto, cinza e linha preta por dentro, com as marcas dos cantos.
        q.preencher(e, c.moldura, c.moldura);
        q.contorno(ret(e.esq, e.topo, e.dir - 1, e.base - 1), c.linha);
        q.contorno(ret(e.esq + k - 1, e.topo + k - 1, e.dir - k, e.base - k), c.linha);
        let d = i32::from(k + self.cfg.altura_titulo);
        for x in [ex + d, ed - d] {
            q.linha(x, ey, x, ey + i32::from(k) - 1, c.linha);
            q.linha(x, eb - i32::from(k) + 1, x, eb, c.linha);
        }
        for y in [ey + d, eb - d] {
            q.linha(ex, y, ex + i32::from(k) - 1, y, c.linha);
            q.linha(ed - i32::from(k) + 1, y, ed, y, c.linha);
        }
        let sistema = self.fonte(interface, self.cfg.fonte_sistema);
        // Título: caixa do menu de sistema, faixa azul com o nome, botões de minimizar e maximizar.
        let t = self.faixa_titulo();
        let inativas = self.cfg.janelas.as_ref().map(|j| (j.cores.titulo_inativo, j.cores.texto_titulo_inativo));
        let (cor_titulo, cor_texto_titulo) = match inativas {
            Some(par) if !self.janela_ativa() => par,
            _ => (c.titulo, c.texto_titulo),
        };
        q.preencher(t, cor_titulo, cor_titulo);
        let h = self.cfg.altura_titulo;
        let caixa = self.caixa_menu_sistema();
        q.preencher(caixa, c.botao, c.botao);
        q.linha(i32::from(caixa.dir), i32::from(t.topo), i32::from(caixa.dir), i32::from(t.base) - 1, c.linha);
        let (bx, by) = (i32::from(caixa.esq) + 2, i32::from(t.topo) + 7);
        q.contorno(ret(bx as i16, by as i16, bx as i16 + 12, by as i16 + 2), c.linha);
        q.preencher(ret(bx as i16 + 1, by as i16 + 1, bx as i16 + 12, by as i16 + 2), c.menu, c.menu);
        let max = ret(t.dir - h, t.topo, t.dir, t.base);
        let min = ret(t.dir - 2 * h - 1, t.topo, t.dir - h - 1, t.base);
        for (b, cima) in [(min, false), (max, true)] {
            self.desenhar_botao_titulo(q, b);
            let cx = i32::from(b.esq) + 8;
            let y0 = i32::from(b.topo) + if cima { 6 } else { 7 };
            for kk in 0..4 {
                let w = if cima { kk } else { 3 - kk };
                q.linha(cx - w, y0 + kk, cx + w, y0 + kk, c.linha);
            }
        }
        q.linha(i32::from(min.esq) - 1, i32::from(t.topo), i32::from(min.esq) - 1, i32::from(t.base) - 1, c.linha);
        q.linha(i32::from(max.esq) - 1, i32::from(t.topo), i32::from(max.esq) - 1, i32::from(t.base) - 1, c.linha);
        if let Some(f) = sistema {
            let w = f.largura_texto(self.cfg.titulo.as_bytes()) as i32;
            let (x0, x1) = (i32::from(caixa.dir) + 1, i32::from(min.esq) - 1);
            let x = (x0 + x1 - w) / 2;
            q.recorte = Some(ret(x0 as i16, t.topo, x1 as i16, t.base));
            escrever(q, f, x, i32::from(t.topo) + (i32::from(h) - f.altura() as i32) / 2, self.cfg.titulo.as_bytes(), cor_texto_titulo);
            q.recorte = None;
        }
        q.linha(i32::from(i.esq), i32::from(t.base), i32::from(i.dir) - 1, i32::from(t.base), c.linha);
        // Barra de menus (só desenho).
        let m = self.faixa_menu();
        q.preencher(m, c.menu, c.menu);
        if let Some(f) = sistema {
            for it in &self.cfg.menu {
                escrever_mnemonico(q, f, i32::from(m.esq + it.x), i32::from(m.topo), &it.texto, c.texto_menu, false);
            }
        }
        q.linha(i32::from(i.esq), i32::from(m.base), i32::from(i.dir) - 1, i32::from(m.base), c.linha);
        // Barra de botões: fundo escuro, botões com relevo, linha preta embaixo.
        let fb = self.faixa_botoes();
        q.preencher(fb, c.fundo_botoes, c.fundo_botoes);
        q.linha(i32::from(i.esq), i32::from(fb.base) - 1, i32::from(i.dir) - 1, i32::from(fb.base) - 1, c.linha);
        for (n, b) in self.cfg.botoes.iter().enumerate() {
            let r = self.botao(n);
            let (x0, y0, x1, y1) = (i32::from(r.esq), i32::from(r.topo), i32::from(r.dir) - 1, i32::from(r.base) - 1);
            q.preencher(r, c.botao, c.botao);
            for (cx, cy) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
                q.ponto(cx, cy, c.fundo_botoes);
            }
            // Contorno preto sem os quatro cantos (botão do Windows 3.1).
            q.linha(x0 + 1, y0, x1 - 1, y0, c.linha);
            q.linha(x0 + 1, y1, x1 - 1, y1, c.linha);
            q.linha(x0, y0 + 1, x0, y1 - 1, c.linha);
            q.linha(x1, y0 + 1, x1, y1 - 1, c.linha);
            for kk in 1..=2 {
                q.linha(x0 + 1, y0 + kk, x1 - 1 - kk, y0 + kk, c.relevo_claro);
                q.linha(x0 + kk, y0 + 1, x0 + kk, y1 - 1 - kk, c.relevo_claro);
                q.linha(x0 + kk, y1 - kk, x1 - 1, y1 - kk, c.relevo_escuro);
                q.linha(x1 - kk, y0 + kk, x1 - kk, y1 - 1, c.relevo_escuro);
            }
            if let Some(f) = self.fonte(interface, self.cfg.fonte_botoes) {
                let w = largura_sem_e(f, &b.texto);
                // Centrado com o pixel a mais do negrito simulado (o "overhang" do Windows)
                // [medido nos seis botões de a06].
                let x = (x0 + x1 - 1 - w - i32::from(f.negrito())) / 2;
                escrever_mnemonico(q, f, x, y0 + i32::from(self.cfg.topo_texto_botao), &b.texto, c.linha, !self.habilitado(b.acao));
            }
        }
        // Texto do tópico.
        let a = self.area_texto();
        q.preencher(a, c.fundo, c.fundo);
        let (linhas, com_barra) = self.layout_com_barra(interface);
        let area_texto = if com_barra { ret(a.esq, a.topo, self.barra().esq - 1, a.base) } else { a };
        q.recorte = Some(area_texto);
        let (ax, ay) = (i32::from(a.esq), i32::from(a.topo) - self.rolagem);
        self.desenhar_linhas(q, interface, &linhas, ax, ay, a);
        q.recorte = None;
        if com_barra {
            self.desenhar_barra(q, &linhas);
        }
        self.desenhar_janelas(q, interface);
        q.recorte = antes;
    }

    /// Desenha as linhas com a origem do texto em (ax, ay), pulando as que ficam fora de `area`.
    pub(crate) fn desenhar_linhas(&self, q: &mut Quadro, interface: &Interface, linhas: &[Linha], ax: i32, ay: i32, area: Retangulo) {
        let c = &self.cfg.cores;
        for l in linhas {
            if ay + l.topo > i32::from(area.base) || ay + l.topo + l.altura < i32::from(area.topo) {
                continue;
            }
            for p in &l.pecas {
                match &p.conteudo {
                    Conteudo::Texto { fonte, bytes, sublinhado } => {
                        let Some(f) = self.fonte(interface, *fonte) else { continue };
                        let y = ay + l.topo + l.base - f.ascendente() as i32;
                        let cor = if p.salto.is_some() { c.salto } else { c.texto };
                        escrever(q, f, ax + p.x, y, bytes, cor);
                        if p.salto.is_some() && *sublinhado > 0 {
                            let yb = y + f.ascendente() as i32 + 1;
                            q.linha(ax + p.x, yb, ax + p.x + sublinhado - 1, yb, cor);
                        }
                    }
                    Conteudo::Figura(n) => {
                        if let Some(b) = self.figuras.get(n) {
                            q.bitmap(b, ax + p.x, ay + l.topo + l.base - b.altura as i32);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn desenhar_botao_titulo(&self, q: &mut Quadro, b: Retangulo) {
        let c = &self.cfg.cores;
        q.preencher(b, c.botao, c.botao);
        let (x0, y0, x1, y1) = (i32::from(b.esq), i32::from(b.topo), i32::from(b.dir) - 1, i32::from(b.base) - 1);
        q.linha(x0, y0, x1, y0, c.relevo_claro);
        q.linha(x0, y0, x0, y1, c.relevo_claro);
        for kk in 0..2 {
            q.linha(x0 + kk, y1 - kk, x1, y1 - kk, c.relevo_escuro);
            q.linha(x1 - kk, y0 + kk, x1 - kk, y1, c.relevo_escuro);
        }
    }

    fn desenhar_barra(&self, q: &mut Quadro, linhas: &[Linha]) {
        let c = &self.cfg.cores;
        let v = self.barra();
        let lado = self.cfg.largura_barra - 1;
        q.preencher(v, c.trilho, c.trilho);
        q.linha(i32::from(v.esq) - 1, i32::from(v.topo), i32::from(v.esq) - 1, i32::from(v.base) - 1, c.linha);
        let cima = ret(v.esq, v.topo, v.dir, v.topo + lado);
        let baixo = ret(v.esq, v.base - lado, v.dir, v.base);
        for (b, s) in [(cima, Seta::Cima), (baixo, Seta::Baixo)] {
            self.desenhar_botao_titulo(q, b);
            desenhar_seta(q, b, s, c.linha);
        }
        q.linha(i32::from(v.esq), i32::from(cima.base), i32::from(v.dir) - 1, i32::from(cima.base), c.linha);
        q.linha(i32::from(v.esq), i32::from(baixo.topo) - 1, i32::from(v.dir) - 1, i32::from(baixo.topo) - 1, c.linha);
        let r = Rolagem { pos: self.rolagem, max: self.rolagem_maxima(linhas) };
        if let Some(p) = inicio_do_polegar(v, false, lado, r) {
            let p = p as i16;
            q.linha(i32::from(v.esq), i32::from(p), i32::from(v.dir) - 1, i32::from(p), c.linha);
            self.desenhar_botao_titulo(q, ret(v.esq, p + 1, v.dir, p + 1 + lado));
            q.linha(i32::from(v.esq), i32::from(p + lado + 1), i32::from(v.dir) - 1, i32::from(p + lado + 1), c.linha);
        }
    }
}

/// Palavras com os espaços que as seguem ("ab  cd" → "ab  ", "cd"); espaços no início formam uma
/// palavra própria.
fn palavras(t: &[u8]) -> Vec<&[u8]> {
    let mut v = Vec::new();
    let mut i = 0;
    while i < t.len() {
        let ini = i;
        while i < t.len() && t[i] != b' ' {
            i += 1;
        }
        while i < t.len() && t[i] == b' ' {
            i += 1;
        }
        v.push(&t[ini..i]);
    }
    v
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn palavras_guardam_os_espacos() {
        assert_eq!(palavras(b"ab  cd"), vec![&b"ab  "[..], &b"cd"[..]]);
        assert_eq!(palavras(b" x"), vec![&b" "[..], &b"x"[..]]);
    }
}
