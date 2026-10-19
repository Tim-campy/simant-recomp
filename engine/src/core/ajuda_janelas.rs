//! Janelas pequenas do WinHelp 3.1 em volta do visor de ajuda (`core/ajuda.rs`): os menus File,
//! Edit, Bookmark e Help, o diálogo Search (palavras-chave do `|KWBTREE`), a janela History e os
//! popups (saltos 0xE0). Estado, entrada e desenho; sem SDL.
//!
//! Especificação: docs/spec/telas/ajuda.md §4. Dados: data/ajuda.json → `janelas` (medidas da
//! bancada, relativas ao canto externo da janela de ajuda).

use serde::Deserialize;

use super::ajuda::{contem, escrever, escrever_mnemonico, largura_sem_e, ret, VisorAjuda};
use super::gerenciador::{desenhar_seta, Seta};
use super::quadro::Quadro;
use super::telas::Interface;
use crate::formats::fonte::Fonte;
use crate::formats::janela::Retangulo;

/// O que um item de menu faz no port.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcaoMenu {
    /// Só fecha o menu (o port não tem o diálogo do item).
    #[default]
    SemEfeito,
    /// File → Exit: fecha a ajuda.
    Sair,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ItemMenu {
    /// Texto com `&`; "-" = separador.
    pub texto: String,
    #[serde(default)]
    pub acao: AcaoMenu,
}

impl ItemMenu {
    fn separador(&self) -> bool {
        self.texto == "-"
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct MenuAjuda {
    /// Colunas [x0, x1] do item destacado na barra.
    pub destaque: [i16; 2],
    /// Largura da caixa (com o contorno).
    pub largura: i16,
    pub itens: Vec<ItemMenu>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GeometriaBusca {
    pub externo: [i16; 4],
    pub titulo: String,
    pub rotulo_palavra: Vec<String>,
    pub rotulo_palavra_em: [i16; 2],
    pub altura_rotulo: i16,
    pub caixa_texto: [i16; 4],
    pub lista_palavras: [i16; 4],
    pub botao_cancelar: [i16; 4],
    pub botao_mostrar: [i16; 4],
    pub botao_ir: [i16; 4],
    pub separador: [i16; 4],
    pub rotulo_topicos: String,
    pub rotulo_topicos_em: [i16; 2],
    pub lista_topicos: [i16; 4],
    pub cancelar: String,
    pub fechar: String,
    pub mostrar: String,
    pub ir: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GeometriaHistorico {
    /// Retângulo [x0, y0, x1, y1] (inclusivo) relativo ao canto do visor, usado quando a tela não é
    /// conhecida ou sem `divisor_tela`.
    pub externo: [i16; 4],
    /// Lugar padrão do WinHelp para a History, sem `H_WindowPosition`: x = L / d, y = A / d, tamanho
    /// L / d × A / d da tela (213, 160, 213 × 160 em 640×480, medido). Opcional (API 0.32).
    #[serde(default)]
    pub divisor_tela: Option<i16>,
    /// Linha da tela em que o WinHelp põe o topo do visor (2): o port desenha o visor a partir da
    /// linha 0 da área cliente e desloca a History do mesmo tanto.
    #[serde(default)]
    pub topo_do_visor_na_tela: i16,
    pub titulo: String,
    /// Entradas guardadas [hipótese].
    pub maximo: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GeometriaPopup {
    /// Largura externa (contorno incluído, sem a sombra) [hipótese: uma amostra].
    pub largura: i16,
    pub borda: i16,
    pub sombra: i16,
    /// Margens do texto dentro do contorno: esquerda, topo, direita, base.
    pub margem: [i16; 4],
    /// Distância do topo do popup à base do ponto quente [hipótese: uma amostra].
    pub abaixo_do_ponto: i16,
    /// Área onde o popup tem de caber (a área cliente do port).
    pub tela: [i16; 2],
}

#[derive(Clone, Debug, Deserialize)]
pub struct CoresJanelas {
    pub linha: u8,
    pub selecao: u8,
    pub texto_selecao: u8,
    pub sombra_menu: u8,
    pub face: u8,
    pub luz: u8,
    pub sombra: u8,
    pub fundo: u8,
    pub texto: u8,
    pub borda_dialogo: u8,
    pub titulo_inativo: u8,
    pub texto_titulo_inativo: u8,
    pub foco: u8,
}

/// `janelas` de data/ajuda.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigJanelasAjuda {
    pub fonte_menu: usize,
    pub fonte_dialogo: usize,
    pub altura_item_menu: i16,
    pub altura_separador: i16,
    pub x_texto_menu: i16,
    pub menus: Vec<MenuAjuda>,
    pub busca: GeometriaBusca,
    pub historico: GeometriaHistorico,
    pub popup: GeometriaPopup,
    pub altura_item_lista: i16,
    pub recuo_texto_lista: i16,
    pub largura_barra_lista: i16,
    pub cores: CoresJanelas,
}

/// Diálogo Search aberto.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Busca {
    /// Texto da caixa.
    pub texto: String,
    /// Palavra-chave selecionada e a primeira visível.
    pub palavra: usize,
    pub topo: usize,
    /// O jogador já digitou ou escolheu (Show Topics habilitado).
    pub escolheu: bool,
    /// Tópicos da palavra (depois de Show Topics) e o selecionado.
    pub topicos: Option<Vec<usize>>,
    pub topico: usize,
}

/// Janela por cima do visor que recebe a entrada.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sobreposta {
    /// Menu da barra aberto; `item` destacado.
    Menu { menu: usize, item: Option<usize> },
    Busca(Busca),
    /// Popup do tópico, com o contorno externo (sem a sombra).
    Popup { topico: usize, caixa: Retangulo },
}

/// Janela History aberta: item selecionado e o primeiro visível; `foco` = é a janela ativa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JanelaHistorico {
    pub sel: usize,
    pub topo: usize,
    pub foco: bool,
}

fn r4(base: Retangulo, v: [i16; 4]) -> Retangulo {
    // [x0, y0, x1, y1] inclusivos relativos a `base` → retângulo com dir/base exclusivos.
    ret(base.esq + v[0], base.topo + v[1], base.esq + v[2] + 1, base.topo + v[3] + 1)
}

/// Letra depois do `&`, minúscula.
fn mnemonico(t: &str) -> Option<char> {
    t.find('&').and_then(|p| t[p + 1..].chars().next()).map(|c| c.to_ascii_lowercase())
}

impl VisorAjuda {
    pub(crate) fn cj(&self) -> Option<&ConfigJanelasAjuda> {
        self.cfg.janelas.as_ref()
    }

    /// O menu, Search ou popup aberto.
    pub fn sobreposta(&self) -> Option<&Sobreposta> {
        self.sobreposta.as_ref()
    }

    /// A janela History aberta.
    pub fn janela_historico(&self) -> Option<&JanelaHistorico> {
        self.janela_historico.as_ref()
    }

    /// A janela principal do WinHelp está ativa (sem diálogo, popup ou History com o foco).
    pub fn janela_ativa(&self) -> bool {
        !matches!(self.sobreposta, Some(Sobreposta::Busca(_)) | Some(Sobreposta::Popup { .. })) && !self.janela_historico.as_ref().is_some_and(|h| h.foco)
    }

    /// Registra o tópico mostrado na lista do History.
    pub(crate) fn visitar(&mut self, i: usize) {
        self.visitados.push(i);
        let max = self.cj().map_or(40, |c| c.historico.maximo.max(1));
        if self.visitados.len() > max {
            let sobra = self.visitados.len() - max;
            self.visitados.drain(..sobra);
        }
    }

    /// Itens do History: o mais recente primeiro (com repetições e o atual).
    pub fn itens_historico(&self) -> Vec<usize> {
        self.visitados.iter().rev().copied().collect()
    }

    // --- menus -------------------------------------------------------------------------------

    /// Faixa realçada do item `k` da barra de menus (a célula dele, na linha em que ele está).
    fn retangulo_destaque(&self, k: usize) -> Option<Retangulo> {
        self.cj()?.menus.get(k)?;
        self.celulas_menu().get(k).map(|c| c.0)
    }

    /// Caixa do menu `k` (com o contorno) e o retângulo de cada item.
    fn caixa_menu(&self, k: usize) -> Option<(Retangulo, Vec<Retangulo>)> {
        let c = self.cj()?;
        let m = c.menus.get(k)?;
        let d = self.retangulo_destaque(k)?;
        // A linha de baixo da célula (na última linha da barra, a linha preta) é o contorno de cima.
        let topo = d.base;
        let mut y = topo + 1;
        let mut itens = Vec::new();
        for it in &m.itens {
            let h = if it.separador() { c.altura_separador } else { c.altura_item_menu };
            itens.push(ret(d.esq + 1, y, d.esq + m.largura - 1, y + h));
            y += h;
        }
        Some((ret(d.esq, topo, d.esq + m.largura, y + 1), itens))
    }

    /// Abre o menu `k` da barra (pelo teclado o primeiro item já vem destacado).
    pub fn abrir_menu(&mut self, k: usize, pelo_teclado: bool) {
        if self.cj().is_some_and(|c| k < c.menus.len()) {
            self.sobreposta = Some(Sobreposta::Menu { menu: k, item: pelo_teclado.then_some(0) });
        }
    }

    fn escolher_item(&mut self, menu: usize, item: usize) -> bool {
        self.sobreposta = None;
        let acao = self.cj().and_then(|c| c.menus.get(menu)).and_then(|m| m.itens.get(item)).map(|i| i.acao);
        acao == Some(AcaoMenu::Sair)
    }

    fn proximo_item(&self, menu: usize, de: Option<usize>, passo: isize) -> Option<usize> {
        let itens = &self.cj()?.menus.get(menu)?.itens;
        let n = itens.len() as isize;
        let mut i = de.map_or(if passo > 0 { -1 } else { n }, |i| i as isize);
        for _ in 0..n {
            i = (i + passo).rem_euclid(n);
            if !itens[i as usize].separador() {
                return Some(i as usize);
            }
        }
        None
    }

    // --- Search -----------------------------------------------------------------------------

    /// Abre o diálogo Search (botão Search, Alt+S): caixa vazia, a primeira palavra selecionada.
    pub fn abrir_busca(&mut self) {
        if self.cj().is_some() && !self.palavras.is_empty() {
            self.sobreposta = Some(Sobreposta::Busca(Busca { texto: String::new(), palavra: 0, topo: 0, escolheu: false, topicos: None, topico: 0 }));
        }
    }

    fn visiveis_palavras(&self) -> usize {
        let Some(c) = self.cj() else { return 1 };
        let l = c.busca.lista_palavras;
        (((l[3] - l[1] - 1) / c.altura_item_lista).max(1)) as usize
    }

    /// Digitar na caixa: seleciona a primeira palavra que começa com o texto e a põe no topo.
    fn digitar(&mut self, texto: String) {
        let n_vis = self.visiveis_palavras();
        let total = self.palavras.len();
        let achada = self.palavras.iter().position(|p| p.palavra.to_lowercase().starts_with(&texto.to_lowercase()));
        if let Some(Sobreposta::Busca(b)) = self.sobreposta.as_mut() {
            b.texto = texto;
            b.escolheu = !b.texto.is_empty();
            b.topicos = None;
            if let Some(i) = achada {
                b.palavra = i;
                b.topo = i.min(total.saturating_sub(n_vis));
            }
        }
    }

    /// Show Topics: a caixa recebe a grafia da palavra e a lista de baixo os tópicos dela.
    fn mostrar_topicos(&mut self) {
        let Some(Sobreposta::Busca(b)) = self.sobreposta.as_ref() else { return };
        if !b.escolheu {
            return;
        }
        let Some(p) = self.palavras.get(b.palavra) else { return };
        let mut topicos: Vec<usize> = p.topicos.iter().filter_map(|&pos| self.arquivo.topico_da_posicao(pos)).collect();
        topicos.dedup();
        let palavra = p.palavra.clone();
        if let Some(Sobreposta::Busca(b)) = self.sobreposta.as_mut() {
            b.texto = palavra;
            b.topicos = Some(topicos);
            b.topico = 0;
        }
    }

    /// Go To: fecha o diálogo e vai ao tópico escolhido.
    fn ir_ao_topico(&mut self) {
        let Some(Sobreposta::Busca(b)) = self.sobreposta.as_ref() else { return };
        let Some(i) = b.topicos.as_ref().and_then(|t| t.get(b.topico)).copied() else { return };
        self.sobreposta = None;
        self.ir_para(i);
    }

    // --- History ---------------------------------------------------------------------------

    /// Abre (ou traz para a frente) a janela History, com o tópico anterior selecionado.
    pub fn abrir_historico(&mut self) {
        if self.cj().is_none() {
            return;
        }
        let sel = usize::from(self.visitados.len() > 1);
        self.janela_historico = Some(JanelaHistorico { sel, topo: 0, foco: true });
    }

    fn visiveis_historico(&self) -> usize {
        let Some(c) = self.cj() else { return 1 };
        (self.cliente_historico().map_or(13, |r| i32::from(r.base - r.topo)) / i32::from(c.altura_item_lista).max(1)).max(1) as usize
    }

    fn externo_historico(&self) -> Option<Retangulo> {
        let h = &self.cj()?.historico;
        let padrao = match (self.tela, h.divisor_tela) {
            (Some((l, a)), Some(d)) if d > 0 => Some([l / d, a / d, l / d, a / d]),
            _ => None,
        };
        // `H_WindowPosition` (WIN.INI) vale sobre o padrão: [x, y, largura, altura] na tela.
        match self.historico_guardado.filter(|_| self.tela.is_some()).or(padrao) {
            Some([x, y, w, alt]) => {
                let y = y - h.topo_do_visor_na_tela + self.externo().topo;
                Some(ret(x, y, x + w, y + alt))
            }
            None => Some(r4(self.externo(), h.externo)),
        }
    }

    /// Retângulo externo da janela History (para os testes e a interface).
    pub fn retangulo_historico(&self) -> Option<Retangulo> {
        self.externo_historico()
    }

    fn cliente_historico(&self) -> Option<Retangulo> {
        let e = self.externo_historico()?;
        let k = self.cfg.espessura_moldura;
        Some(ret(e.esq + k, e.topo + k + self.cfg.altura_titulo + 1, e.dir - k, e.base - k))
    }

    fn ir_do_historico(&mut self) {
        let itens = self.itens_historico();
        if let Some(h) = self.janela_historico.as_ref() {
            if let Some(&i) = itens.get(h.sel) {
                self.ir_para(i);
            }
        }
    }

    // --- popup -----------------------------------------------------------------------------

    /// Abre o popup do tópico `topico` para o ponto quente `quente` (área cliente).
    pub(crate) fn abrir_popup(&mut self, interface: &Interface, topico: usize, quente: Retangulo) {
        let Some(c) = self.cj().map(|c| c.popup.clone()) else {
            self.ir_para(topico);
            return;
        };
        let [me, mt, md, mb] = c.margem;
        let [largura_tela, altura_tela] = c.tela;
        let util = i32::from(c.largura - 2 * c.borda - me - md);
        let linhas = self.layout_topico(interface, topico, 0, util);
        let alto = linhas.last().map_or(0, |l| l.topo + l.altura) as i16;
        let altura = alto + mt + mb + 2 * c.borda;
        // Centrado (com a sombra) no ponto quente, logo abaixo dele [hipótese: uma amostra].
        let total = c.largura + c.sombra;
        let mut x = (quente.esq + quente.dir) / 2 - total / 2;
        let mut y = quente.base + c.abaixo_do_ponto;
        x = x.clamp(0, (largura_tela - total).max(0));
        // Sem espaço embaixo, sobe o necessário [hipótese: a regra do WinHelp não foi medida].
        if y + altura + c.sombra > altura_tela {
            y = (altura_tela - altura - c.sombra).max(0);
        }
        self.sobreposta = Some(Sobreposta::Popup { topico, caixa: ret(x, y, x + c.largura, y + altura) });
    }

    /// A janela History, se aberta, perde o foco (clique na moldura da janela principal).
    pub fn tirar_foco_do_historico(&mut self) {
        if let Some(h) = self.janela_historico.as_mut() {
            h.foco = false;
        }
    }

    /// O ponto está numa janela pequena (popup, Search, History) — que pode sair da janela de ajuda.
    pub fn contem_janelas(&self, x: i32, y: i32) -> bool {

        let Some(c) = self.cj() else { return false };
        let dentro = match &self.sobreposta {
            Some(Sobreposta::Popup { caixa, .. }) => contem(*caixa, x, y),
            Some(Sobreposta::Busca(_)) => contem(r4(self.externo(), c.busca.externo), x, y),
            Some(Sobreposta::Menu { menu, .. }) => self.caixa_menu(*menu).is_some_and(|(r, _)| contem(r, x, y)),
            None => false,
        };
        dentro || (self.janela_historico.is_some() && self.externo_historico().is_some_and(|r| contem(r, x, y)))
    }

    // --- entrada -----------------------------------------------------------------------------

    /// Clique com uma janela pequena aberta ou na barra de menus. `None`: não era com elas (o visor
    /// trata); `Some(sair)`: tratado (sair = File → Exit).
    pub(crate) fn clicar_janelas(&mut self, x: i32, y: i32, duplo: bool) -> Option<bool> {
        self.cj()?;
        match self.sobreposta.clone() {
            Some(Sobreposta::Popup { .. }) => {
                // Qualquer clique fecha o popup e não segue (bancada p04).
                self.sobreposta = None;
                return Some(false);
            }
            Some(Sobreposta::Menu { menu, .. }) => {
                if let Some((_, itens)) = self.caixa_menu(menu) {
                    if let Some(k) = itens.iter().position(|r| contem(*r, x, y)) {
                        let separador = self.cj().and_then(|c| c.menus[menu].itens.get(k)).is_some_and(|i| i.separador());
                        if separador {
                            return Some(false);
                        }
                        return Some(self.escolher_item(menu, k));
                    }
                }
                let n = self.cj().map_or(0, |c| c.menus.len());
                let outro = (0..n).find(|&k| self.retangulo_destaque(k).is_some_and(|r| contem(r, x, y)));
                self.sobreposta = match outro {
                    Some(k) if k != menu => Some(Sobreposta::Menu { menu: k, item: None }),
                    _ => None,
                };
                return Some(false);
            }
            Some(Sobreposta::Busca(b)) => {
                self.clicar_busca(b, x, y, duplo);
                return Some(false);
            }
            None => {}
        }
        // History: dentro dela, ela fica com o foco; fora, o foco volta ao visor.
        if let Some(e) = self.externo_historico().filter(|_| self.janela_historico.is_some()) {
            if contem(e, x, y) {
                self.clicar_historico(x, y, duplo);
                return Some(false);
            }
            if let Some(h) = self.janela_historico.as_mut() {
                h.foco = false;
            }
        }
        let n = self.cj().map_or(0, |c| c.menus.len());
        if let Some(k) = (0..n).find(|&k| self.retangulo_destaque(k).is_some_and(|r| contem(r, x, y))) {
            self.abrir_menu(k, false);
            return Some(false);
        }
        None
    }

    fn clicar_busca(&mut self, b: Busca, x: i32, y: i32, duplo: bool) {
        let Some(c) = self.cj().cloned() else { return };
        let e = self.externo();
        let g = &c.busca;
        let em = |v: [i16; 4]| r4(e, v);
        if contem(em(g.botao_cancelar), x, y) {
            self.sobreposta = None;
        } else if contem(em(g.botao_mostrar), x, y) {
            self.mostrar_topicos();
        } else if contem(em(g.botao_ir), x, y) {
            self.ir_ao_topico();
        } else if contem(em(g.lista_palavras), x, y) {
            let l = em(g.lista_palavras);
            if x >= i32::from(l.dir - c.largura_barra_lista) {
                // Barra de rolagem: setas e trilho (uma página).
                let n_vis = self.visiveis_palavras();
                let max = self.palavras.len().saturating_sub(n_vis);
                let seta = i32::from(c.largura_barra_lista);
                let topo = if y < i32::from(l.topo) + seta {
                    b.topo.saturating_sub(1)
                } else if y >= i32::from(l.base) - seta {
                    (b.topo + 1).min(max)
                } else if y < (i32::from(l.topo) + i32::from(l.base)) / 2 {
                    b.topo.saturating_sub(n_vis)
                } else {
                    (b.topo + n_vis).min(max)
                };
                if let Some(Sobreposta::Busca(bb)) = self.sobreposta.as_mut() {
                    bb.topo = topo;
                }
                return;
            }
            let k = ((y - i32::from(l.topo) - 1) / i32::from(c.altura_item_lista)).max(0) as usize + b.topo;
            if let Some(p) = self.palavras.get(k) {
                let palavra = p.palavra.clone();
                if let Some(Sobreposta::Busca(bb)) = self.sobreposta.as_mut() {
                    bb.palavra = k;
                    bb.texto = palavra;
                    bb.escolheu = true;
                    bb.topicos = None;
                }
                if duplo {
                    self.mostrar_topicos();
                }
            }
        } else if contem(em(g.lista_topicos), x, y) {
            let l = em(g.lista_topicos);
            let k = ((y - i32::from(l.topo) - 1) / i32::from(c.altura_item_lista)).max(0) as usize;
            if b.topicos.as_ref().is_some_and(|t| k < t.len()) {
                if let Some(Sobreposta::Busca(bb)) = self.sobreposta.as_mut() {
                    bb.topico = k;
                }
                if duplo {
                    self.ir_ao_topico();
                }
            }
        }
    }

    fn clicar_historico(&mut self, x: i32, y: i32, duplo: bool) {
        let Some(c) = self.cj().cloned() else { return };
        let Some(e) = self.externo_historico() else { return };
        let caixa = ret(e.esq + self.cfg.espessura_moldura, e.topo + self.cfg.espessura_moldura, e.esq + self.cfg.espessura_moldura + self.cfg.altura_titulo, e.topo + self.cfg.espessura_moldura + self.cfg.altura_titulo);
        if contem(caixa, x, y) {
            if duplo {
                self.janela_historico = None;
            } else if let Some(h) = self.janela_historico.as_mut() {
                h.foco = true;
            }
            return;
        }
        let n = self.visitados.len();
        let Some(cl) = self.cliente_historico() else { return };
        if let Some(h) = self.janela_historico.as_mut() {
            h.foco = true;
            if contem(cl, x, y) {
                let k = ((y - i32::from(cl.topo)) / i32::from(c.altura_item_lista)) as usize + h.topo;
                if k < n {
                    h.sel = k;
                    if duplo {
                        self.ir_do_historico();
                    }
                }
            }
        }
    }

    /// Tecla com uma janela pequena aberta (ou Alt+letra de um menu). `None`: o visor trata;
    /// `Some(sair)`: tratada.
    pub(crate) fn tecla_janelas(&mut self, nome: &str, alt: bool, shift: bool) -> Option<bool> {
        self.cj()?;
        match self.sobreposta.clone() {
            Some(Sobreposta::Popup { .. }) => {
                self.sobreposta = None;
                return Some(false);
            }
            Some(Sobreposta::Menu { menu, item }) => {
                let n = self.cj().map_or(1, |c| c.menus.len());
                match nome {
                    "Escape" => self.sobreposta = None,
                    "Left" | "Right" => {
                        let k = if nome == "Left" { (menu + n - 1) % n } else { (menu + 1) % n };
                        self.abrir_menu(k, true);
                    }
                    "Up" | "Down" => {
                        let i = self.proximo_item(menu, item, if nome == "Up" { -1 } else { 1 });
                        self.sobreposta = Some(Sobreposta::Menu { menu, item: i });
                    }
                    "Return" | "Keypad Enter" => {
                        if let Some(i) = item {
                            return Some(self.escolher_item(menu, i));
                        }
                    }
                    outro => {
                        let letra = outro.to_lowercase();
                        let achado = self.cj().and_then(|c| c.menus[menu].itens.iter().position(|it| mnemonico(&it.texto).is_some_and(|ch| ch.to_string() == letra)));
                        if let Some(i) = achado {
                            return Some(self.escolher_item(menu, i));
                        }
                    }
                }
                return Some(false);
            }
            Some(Sobreposta::Busca(b)) => {
                self.tecla_busca(b, nome, alt, shift);
                return Some(false);
            }
            None => {}
        }
        if let Some(h) = self.janela_historico.clone().filter(|h| h.foco) {
            let n = self.visitados.len();
            let vis = self.visiveis_historico();
            let mut h2 = h.clone();
            match nome {
                "Up" => h2.sel = h.sel.saturating_sub(1),
                "Down" => h2.sel = (h.sel + 1).min(n.saturating_sub(1)),
                "Return" | "Keypad Enter" => {
                    self.ir_do_historico();
                    return Some(false);
                }
                // Esc e Alt+S não fazem nada com o History na frente (bancada s08, s09).
                _ => return Some(false),
            }
            if h2.sel < h2.topo {
                h2.topo = h2.sel;
            } else if h2.sel >= h2.topo + vis {
                h2.topo = h2.sel + 1 - vis;
            }
            self.janela_historico = Some(h2);
            return Some(false);
        }
        if alt {
            let letra = nome.to_lowercase();
            let n = self.cj().map_or(0, |c| c.menus.len());
            let menu = self.cfg.menu.iter().take(n).position(|it| mnemonico(&it.texto).is_some_and(|c| c.to_string() == letra));
            if let Some(k) = menu {
                self.abrir_menu(k, true);
                return Some(false);
            }
        }
        None
    }

    fn tecla_busca(&mut self, b: Busca, nome: &str, alt: bool, shift: bool) {
        let Some(c) = self.cj().cloned() else { return };
        let g = &c.busca;
        let letra = nome.chars().next().filter(|_| nome.chars().count() == 1).map(|c| c.to_ascii_lowercase());
        if alt {
            if letra == mnemonico(&g.mostrar) {
                self.mostrar_topicos();
            } else if letra == mnemonico(&g.ir) {
                self.ir_ao_topico();
            }
            return;
        }
        let n_vis = self.visiveis_palavras();
        match nome {
            "Escape" => self.sobreposta = None,
            "Return" | "Keypad Enter" => {
                // O botão padrão: Go To depois de Show Topics, senão Show Topics.
                if b.topicos.is_some() {
                    self.ir_ao_topico();
                } else {
                    self.mostrar_topicos();
                }
            }
            "Backspace" => {
                let mut t = b.texto.clone();
                t.pop();
                self.digitar(t);
            }
            "Up" | "Down" => {
                let total = self.palavras.len();
                let k = if nome == "Up" { b.palavra.saturating_sub(1) } else { (b.palavra + 1).min(total.saturating_sub(1)) };
                let palavra = self.palavras.get(k).map(|p| p.palavra.clone()).unwrap_or_default();
                if let Some(Sobreposta::Busca(bb)) = self.sobreposta.as_mut() {
                    bb.palavra = k;
                    bb.texto = palavra;
                    bb.escolheu = true;
                    bb.topicos = None;
                    if k < bb.topo {
                        bb.topo = k;
                    } else if k >= bb.topo + n_vis {
                        bb.topo = k + 1 - n_vis;
                    }
                }
            }
            "Space" => self.digitar(format!("{} ", b.texto)),
            _ => {
                if let Some(l) = letra.filter(|c| c.is_ascii_alphanumeric()) {
                    let l = if shift { l.to_ascii_uppercase() } else { l };
                    self.digitar(format!("{}{l}", b.texto));
                }
            }
        }
    }

    // --- desenho ---------------------------------------------------------------------------

    /// Desenha a barra de menus destacada, o menu aberto, o History, o Search e o popup.
    pub(crate) fn desenhar_janelas(&self, q: &mut Quadro, interface: &Interface) {
        let Some(c) = self.cj() else { return };
        if let Some(Sobreposta::Menu { menu, item }) = &self.sobreposta {
            self.desenhar_menu(q, interface, c, *menu, *item);
        }
        if self.janela_historico.is_some() {
            self.desenhar_historico(q, interface, c);
        }
        match &self.sobreposta {
            Some(Sobreposta::Busca(b)) => self.desenhar_busca(q, interface, c, b),
            Some(Sobreposta::Popup { topico, caixa }) => self.desenhar_popup(q, interface, c, *topico, *caixa),
            _ => {}
        }
    }

    fn desenhar_menu(&self, q: &mut Quadro, interface: &Interface, c: &ConfigJanelasAjuda, menu: usize, item: Option<usize>) {
        let cr = &c.cores;
        let Some(f) = self.fonte(interface, c.fonte_menu) else { return };
        let (Some(d), Some(m), Some(mb)) = (self.retangulo_destaque(menu), c.menus.get(menu), self.cfg.menu.get(menu)) else { return };
        // Item da barra destacado.
        q.preencher(d, cr.selecao, cr.selecao);
        let tx = self.celulas_menu().get(menu).map_or(mb.x, |c| c.2);
        escrever_mnemonico(q, f, i32::from(d.esq + tx), i32::from(d.topo), &mb.texto, cr.texto_selecao, false);

        let Some((caixa, itens)) = self.caixa_menu(menu) else { return };
        let (x0, y0, x1, y1) = (i32::from(caixa.esq), i32::from(caixa.topo), i32::from(caixa.dir) - 1, i32::from(caixa.base) - 1);
        q.preencher(caixa, cr.fundo, cr.fundo);
        q.contorno(ret(caixa.esq, caixa.topo, caixa.dir - 1, caixa.base - 1), cr.linha);
        // Sombra de 1 pixel à direita e embaixo.
        q.linha(x1 + 1, y0 + 1, x1 + 1, y1 + 1, cr.sombra_menu);
        q.linha(x0 + 1, y1 + 1, x1 + 1, y1 + 1, cr.sombra_menu);
        for (k, (it, r)) in m.itens.iter().zip(&itens).enumerate() {
            if it.separador() {
                let ym = i32::from(r.topo) + i32::from(c.altura_separador) / 2;
                q.linha(x0, ym, x1, ym, cr.linha);
                continue;
            }
            let destacado = item == Some(k);
            if destacado {
                q.preencher(*r, cr.selecao, cr.selecao);
            }
            let cor = if destacado { cr.texto_selecao } else { cr.texto };
            escrever_mnemonico(q, f, x0 + i32::from(c.x_texto_menu), i32::from(r.topo), &it.texto, cor, false);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn desenhar_titulo(&self, q: &mut Quadro, f: Option<&Fonte>, t: Retangulo, caixa: Retangulo, texto: &str, ativo: bool, c: &ConfigJanelasAjuda) {
        let cr = &c.cores;
        let (fundo, frente) = if ativo { (self.cfg.cores.titulo, self.cfg.cores.texto_titulo) } else { (cr.titulo_inativo, cr.texto_titulo_inativo) };
        q.preencher(t, fundo, fundo);
        q.preencher(caixa, cr.face, cr.face);
        q.linha(i32::from(caixa.dir), i32::from(t.topo), i32::from(caixa.dir), i32::from(t.base) - 1, cr.linha);
        let (bx, by) = (caixa.esq + 2, t.topo + 7);
        q.contorno(ret(bx, by, bx + 12, by + 2), cr.linha);
        q.preencher(ret(bx + 1, by + 1, bx + 12, by + 2), cr.fundo, cr.fundo);
        if let Some(f) = f {
            let w = f.largura_texto(texto.as_bytes()) as i32;
            let (xa, xb) = (i32::from(caixa.dir) + 1, i32::from(t.dir));
            let antes = q.recorte.replace(ret(xa as i16, t.topo, xb as i16, t.base));
            escrever(q, f, (xa + xb - w) / 2, i32::from(t.topo) + (i32::from(t.base - t.topo) - f.altura() as i32) / 2, texto.as_bytes(), frente);
            q.recorte = antes;
        }
    }

    fn desenhar_historico(&self, q: &mut Quadro, interface: &Interface, c: &ConfigJanelasAjuda) {
        let (Some(e), Some(cl), Some(h)) = (self.externo_historico(), self.cliente_historico(), self.janela_historico.as_ref()) else { return };
        let cr = &c.cores;
        let k = self.cfg.espessura_moldura;
        // Moldura grossa como a do visor, com as marcas dos cantos.
        q.preencher(e, self.cfg.cores.moldura, self.cfg.cores.moldura);
        q.contorno(ret(e.esq, e.topo, e.dir - 1, e.base - 1), cr.linha);
        q.contorno(ret(e.esq + k - 1, e.topo + k - 1, e.dir - k, e.base - k), cr.linha);
        let (ex, ey, ed, eb) = (i32::from(e.esq), i32::from(e.topo), i32::from(e.dir) - 1, i32::from(e.base) - 1);
        let d = i32::from(k + self.cfg.altura_titulo);
        for x in [ex + d, ed - d] {
            q.linha(x, ey, x, ey + i32::from(k) - 1, cr.linha);
            q.linha(x, eb - i32::from(k) + 1, x, eb, cr.linha);
        }
        for y in [ey + d, eb - d] {
            q.linha(ex, y, ex + i32::from(k) - 1, y, cr.linha);
            q.linha(ed - i32::from(k) + 1, y, ed, y, cr.linha);
        }
        let t = ret(e.esq + k, e.topo + k, e.dir - k, e.topo + k + self.cfg.altura_titulo);
        let caixa = ret(t.esq, t.topo, t.esq + self.cfg.altura_titulo, t.base);
        self.desenhar_titulo(q, self.fonte(interface, self.cfg.fonte_sistema), t, caixa, &c.historico.titulo, h.foco, c);
        q.linha(i32::from(t.esq), i32::from(t.base), i32::from(t.dir) - 1, i32::from(t.base), cr.linha);
        q.preencher(cl, cr.fundo, cr.fundo);
        let Some(f) = self.fonte(interface, c.fonte_dialogo) else { return };
        let antes = q.recorte.replace(cl);
        let ai = i32::from(c.altura_item_lista);
        for (n, i) in self.itens_historico().into_iter().enumerate().skip(h.topo) {
            let y = i32::from(cl.topo) + (n - h.topo) as i32 * ai;
            if y >= i32::from(cl.base) {
                break;
            }
            let r = ret(cl.esq, y as i16, cl.dir, (y + ai) as i16);
            let sel = n == h.sel;
            if sel {
                q.preencher(r, cr.selecao, cr.selecao);
                if h.foco {
                    foco(q, r, cr.foco);
                }
            }
            let titulo = self.arquivo.topicos[i].titulo_bruto.as_bytes().to_vec();
            escrever(q, f, i32::from(cl.esq) + 2, y + (ai - f.altura() as i32) / 2, &titulo, if sel { cr.texto_selecao } else { cr.texto });
        }
        q.recorte = antes;
    }

    fn desenhar_busca(&self, q: &mut Quadro, interface: &Interface, c: &ConfigJanelasAjuda, b: &Busca) {
        let cr = &c.cores;
        let g = &c.busca;
        let e0 = self.externo();
        let em = |v: [i16; 4]| r4(e0, v);
        let e = em(g.externo);
        // Moldura de diálogo: 1 pixel preto, 4 na cor da borda ativa (3 em cima), 1 branco por dentro.
        q.preencher(e, cr.borda_dialogo, cr.borda_dialogo);
        q.contorno(ret(e.esq, e.topo, e.dir - 1, e.base - 1), cr.linha);
        let dentro = ret(e.esq + 5, e.topo + 4, e.dir - 5, e.base - 5);
        q.contorno(ret(dentro.esq, dentro.topo, dentro.dir - 1, dentro.base - 1), cr.fundo);
        let cliente = ret(dentro.esq + 1, dentro.topo + 1, dentro.dir - 1, dentro.base - 1);
        let t = ret(cliente.esq, cliente.topo, cliente.dir, cliente.topo + self.cfg.altura_titulo);
        let caixa = ret(t.esq, t.topo, t.esq + self.cfg.altura_titulo, t.base);
        self.desenhar_titulo(q, self.fonte(interface, self.cfg.fonte_sistema), t, caixa, &g.titulo, true, c);
        q.linha(i32::from(t.esq), i32::from(t.base), i32::from(t.dir) - 1, i32::from(t.base), cr.linha);
        q.preencher(ret(cliente.esq, t.base + 1, cliente.dir, cliente.base), cr.fundo, cr.fundo);
        let Some(f) = self.fonte(interface, c.fonte_dialogo) else { return };
        let topo_letras = 2; // do topo da célula ao topo das maiúsculas na fonte do diálogo [medido]
        for (k, l) in g.rotulo_palavra.iter().enumerate() {
            let y = i32::from(e0.topo + g.rotulo_palavra_em[1] + k as i16 * g.altura_rotulo) - topo_letras;
            escrever_mnemonico(q, f, i32::from(e0.esq + g.rotulo_palavra_em[0]), y, l, cr.texto, false);
        }
        // Caixa de texto.
        let ct = em(g.caixa_texto);
        q.preencher(ct, cr.fundo, cr.fundo);
        q.contorno(ret(ct.esq, ct.topo, ct.dir - 1, ct.base - 1), cr.linha);
        let yt = i32::from(ct.topo) + (i32::from(ct.base - ct.topo) - f.altura() as i32) / 2;
        let bytes: Vec<u8> = b.texto.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect();
        escrever(q, f, i32::from(ct.esq) + 3, yt, &bytes, cr.texto);
        // Cursor de texto enquanto a caixa tem o foco (até Show Topics, que o passa à lista).
        if b.topicos.is_none() {
            let xc = i32::from(ct.esq) + 3 + f.largura_texto(&bytes) as i32;
            q.linha(xc, i32::from(ct.topo) + 3, xc, i32::from(ct.base) - 8, cr.linha);
        }
        // Lista de palavras com a barra de rolagem.
        let lp = em(g.lista_palavras);
        let nomes: Vec<Vec<u8>> = self.palavras.iter().map(|p| p.palavra.chars().map(|c| c as u8).collect()).collect();
        let barra = ret(lp.dir - c.largura_barra_lista, lp.topo, lp.dir, lp.base);
        self.desenhar_lista(q, f, c, lp, Some(barra), &nomes, b.topo, Some(b.palavra), false);
        // Botões.
        let fechar = if b.topicos.is_some() { &g.fechar } else { &g.cancelar };
        botao(q, f, cr, em(g.botao_cancelar), fechar, false, true);
        botao(q, f, cr, em(g.botao_mostrar), &g.mostrar, b.escolheu && b.topicos.is_none(), b.escolheu);
        botao(q, f, cr, em(g.botao_ir), &g.ir, b.topicos.is_some(), b.topicos.as_ref().is_some_and(|t| !t.is_empty()));
        q.preencher(em(g.separador), cr.linha, cr.linha);
        let y2 = i32::from(e0.topo + g.rotulo_topicos_em[1]) - topo_letras;
        escrever_mnemonico(q, f, i32::from(e0.esq + g.rotulo_topicos_em[0]), y2, &g.rotulo_topicos, cr.texto, false);
        let lt = em(g.lista_topicos);
        let titulos: Vec<Vec<u8>> = b.topicos.iter().flatten().map(|&i| self.arquivo.topicos[i].titulo_bruto.as_bytes().to_vec()).collect();
        self.desenhar_lista(q, f, c, lt, None, &titulos, 0, b.topicos.as_ref().map(|_| b.topico), true);
    }

    #[allow(clippy::too_many_arguments)]
    fn desenhar_lista(&self, q: &mut Quadro, f: &Fonte, c: &ConfigJanelasAjuda, r: Retangulo, barra: Option<Retangulo>, itens: &[Vec<u8>], topo: usize, sel: Option<usize>, com_foco: bool) {
        let cr = &c.cores;
        q.preencher(r, cr.fundo, cr.fundo);
        q.contorno(ret(r.esq, r.topo, r.dir - 1, r.base - 1), cr.linha);
        let dir_itens = barra.map_or(r.dir - 1, |b| b.esq);
        let area = ret(r.esq + 1, r.topo + 1, dir_itens, r.base - 1);
        let ai = i32::from(c.altura_item_lista);
        let antes = q.recorte.replace(area);
        for (n, t) in itens.iter().enumerate().skip(topo) {
            let y = i32::from(area.topo) + (n - topo) as i32 * ai;
            if y >= i32::from(area.base) {
                break;
            }
            let ri = ret(area.esq, y as i16, area.dir, (y + ai) as i16);
            let s = sel == Some(n);
            if s {
                q.preencher(ri, cr.selecao, cr.selecao);
                if com_foco {
                    foco(q, ri, cr.foco);
                }
            }
            escrever(q, f, i32::from(r.esq + c.recuo_texto_lista), y + (ai - f.altura() as i32) / 2, t, if s { cr.texto_selecao } else { cr.texto });
        }
        q.recorte = antes;
        if let Some(b) = barra {
            let lado = c.largura_barra_lista;
            q.preencher(b, cr.face, cr.face);
            q.contorno(ret(b.esq, b.topo, b.dir - 1, b.base - 1), cr.linha);
            let cima = ret(b.esq + 1, b.topo + 1, b.dir - 1, b.topo + lado - 1);
            let baixo = ret(b.esq + 1, b.base - lado + 1, b.dir - 1, b.base - 1);
            for (cel, s) in [(cima, Seta::Cima), (baixo, Seta::Baixo)] {
                relevo_fino(q, cr, cel);
                desenhar_seta(q, cel, s, cr.linha);
            }
            q.linha(i32::from(b.esq), i32::from(b.topo + lado - 1), i32::from(b.dir) - 1, i32::from(b.topo + lado - 1), cr.linha);
            q.linha(i32::from(b.esq), i32::from(b.base - lado), i32::from(b.dir) - 1, i32::from(b.base - lado), cr.linha);
            let n_vis = ((i32::from(area.base - area.topo)) / ai).max(1) as usize;
            let max = itens.len().saturating_sub(n_vis);
            if max > 0 {
                let ini = i32::from(b.topo + lado - 1);
                let curso = i32::from(b.base - lado) - ini - i32::from(lado);
                // Arredondado (como o MulDiv do Windows).
                let p = ini + (2 * curso * topo.min(max) as i32 + max as i32) / (2 * max as i32);
                let pol = ret(b.esq, p as i16, b.dir, (p + i32::from(lado)) as i16);
                q.contorno(ret(pol.esq, pol.topo, pol.dir - 1, pol.base - 1), cr.linha);
                relevo_fino(q, cr, ret(pol.esq + 1, pol.topo + 1, pol.dir - 1, pol.base - 1));
            }
        }
    }

    fn desenhar_popup(&self, q: &mut Quadro, interface: &Interface, c: &ConfigJanelasAjuda, topico: usize, caixa: Retangulo) {
        let cr = &c.cores;
        let p = &c.popup;
        // Sombra em xadrez, deslocada.
        let s = p.sombra;
        for y in caixa.topo + s..caixa.base + s {
            for x in caixa.esq + s..caixa.dir + s {
                let fora = x >= caixa.dir || y >= caixa.base;
                if fora && (x + y).rem_euclid(2) == 0 {
                    q.ponto(i32::from(x), i32::from(y), cr.linha);
                }
            }
        }
        q.preencher(caixa, cr.linha, cr.linha);
        let dentro = ret(caixa.esq + p.borda, caixa.topo + p.borda, caixa.dir - p.borda, caixa.base - p.borda);
        q.preencher(dentro, cr.fundo, cr.fundo);
        let [me, mt, md, _] = p.margem;
        let util = i32::from(p.largura - 2 * p.borda - me - md);
        let linhas = self.layout_topico(interface, topico, 0, util);
        let antes = q.recorte.replace(dentro);
        self.desenhar_linhas(q, interface, &linhas, i32::from(dentro.esq + me), i32::from(dentro.topo + mt), dentro);
        q.recorte = antes;
    }
}

/// Retângulo de foco pontilhado (pixels alternados) por dentro de `r`.
fn foco(q: &mut Quadro, r: Retangulo, cor: u8) {
    let (x0, y0, x1, y1) = (i32::from(r.esq), i32::from(r.topo), i32::from(r.dir) - 1, i32::from(r.base) - 1);
    for x in x0..=x1 {
        if (x - x0) % 2 == 0 {
            q.ponto(x, y0, cor);
            q.ponto(x, y1, cor);
        }
    }
    for y in y0..=y1 {
        if (y - y0) % 2 == 0 {
            q.ponto(x0, y, cor);
            q.ponto(x1, y, cor);
        }
    }
}

/// Célula com 1 pixel de luz e 2 de sombra (setas e polegar da barra).
fn relevo_fino(q: &mut Quadro, cr: &CoresJanelas, r: Retangulo) {
    q.preencher(r, cr.face, cr.face);
    let (x0, y0, x1, y1) = (i32::from(r.esq), i32::from(r.topo), i32::from(r.dir) - 1, i32::from(r.base) - 1);
    q.linha(x0, y0, x1, y0, cr.luz);
    q.linha(x0, y0, x0, y1, cr.luz);
    for k in 0..2 {
        q.linha(x0 + k, y1 - k, x1, y1 - k, cr.sombra);
        q.linha(x1 - k, y0 + k, x1 - k, y1, cr.sombra);
    }
}

/// Botão de diálogo do Windows 3.1: contorno preto sem os cantos (2 pixels no padrão), 2 de luz,
/// 2 de sombra, face cinza e o texto centrado (acinzentado se desabilitado).
fn botao(q: &mut Quadro, f: &Fonte, cr: &CoresJanelas, r: Retangulo, texto: &str, padrao: bool, habilitado: bool) {
    let (mut x0, mut y0, mut x1, mut y1) = (i32::from(r.esq), i32::from(r.topo), i32::from(r.dir) - 1, i32::from(r.base) - 1);
    q.preencher(r, cr.face, cr.face);
    for (cx, cy) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
        q.ponto(cx, cy, cr.fundo);
    }
    q.linha(x0 + 1, y0, x1 - 1, y0, cr.linha);
    q.linha(x0 + 1, y1, x1 - 1, y1, cr.linha);
    q.linha(x0, y0 + 1, x0, y1 - 1, cr.linha);
    q.linha(x1, y0 + 1, x1, y1 - 1, cr.linha);
    if padrao {
        x0 += 1;
        y0 += 1;
        x1 -= 1;
        y1 -= 1;
        q.contorno(ret(x0 as i16, y0 as i16, x1 as i16, y1 as i16), cr.linha);
    }
    for k in 1..=2 {
        q.linha(x0 + 1, y0 + k, x1 - 1 - k, y0 + k, cr.luz);
        q.linha(x0 + k, y0 + 1, x0 + k, y1 - 1 - k, cr.luz);
        q.linha(x0 + k, y1 - k, x1 - 1, y1 - k, cr.sombra);
        q.linha(x1 - k, y0 + k, x1 - k, y1 - 1, cr.sombra);
    }
    let w = largura_sem_e(f, texto);
    // Centrado, 1 pixel à esquerda do centro da conta [medido nos botões do Search].
    let x = (i32::from(r.esq) + i32::from(r.dir) - 1 - w - i32::from(f.negrito())) / 2 - 1;
    let y = (i32::from(r.topo) + i32::from(r.base) - f.altura() as i32) / 2;
    escrever_mnemonico(q, f, x, y, texto, cr.texto, !habilitado);
}
