//! Menu de ferramentas do Experimental (janela 9 `editToolsMenu`) e os submenus das ferramentas
//! (janelas 12–17), sem SDL: posição, desenho e qual item está sob o mouse. O mesmo botão abre, no
//! Quick Game, o menu de um item `editToExp` (janela 10) e, no Jogo Completo, a figura
//! `editNoExpMenu` (janela 11) (`_RibbonToolsMenu` `1010:de76`).
//!
//! Especificação: docs/spec/telas/experimental.md §2 (`_DoExpMenu` `1010:6ee2`, `_win_DoProxMenu`).
//! As regras das ferramentas estão em `rules::experimental`. Dados: data/menu_ferramentas.json.

use serde::Deserialize;

use super::quadro::Quadro;
use super::telas::{carregar_janela, desenhar_objetos, Interface};
use crate::formats::janela::{Janela, Retangulo};
use crate::recursos::Recursos;
use crate::rules::experimental::Ferramenta;

/// Nome do arquivo de dados.
pub const ARQUIVO_CONFIG: &str = "menu_ferramentas.json";

/// Conteúdo de data/menu_ferramentas.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigMenuFerramentas {
    /// Botão da faixa 34 que abre o menu (`ribTools`, `_RibbonToolsMenu`).
    pub botao: String,
    /// Bitmaps (selecionado, normal) do botão no Experimental (`_NewGame`: 14550/14500).
    pub bitmaps_botao_experimental: [u16; 2],
    /// Janela do menu (9) e primeiro objeto de item (2 = `editExpInfo`).
    pub janela: u16,
    pub primeiro_item: usize,
    /// Janela do submenu de cada ferramenta 0–7 (0 = sem submenu).
    pub submenus: [u16; 8],
    /// Primeiro objeto de item dos submenus (2).
    pub primeiro_item_submenu: usize,
    /// Canto do menu relativo ao canto inferior esquerdo do botão. Só vale sem `moldura`: com ela, a
    /// posição é a medida na bancada (centrado no botão, i05), e este valor fica sem uso.
    pub deslocamento: [i16; 2],
    /// Cor do contorno do item atual (`__win_SetProxItem`); sem uso desde que o realce passou a
    /// inverter o item, como no original.
    #[serde(default)]
    pub cor_item_atual: u8,
    /// Moldura e posição medidas na bancada (i05). Sem ela, o menu fica em `deslocamento` do botão,
    /// sem moldura (o desenho anterior do port).
    #[serde(default)]
    pub moldura: Option<MolduraMenu>,
    /// Janela do botão no Quick Game (10 `editToExp`, um item: vira Experimental) e no Jogo Completo
    /// (11 `editNoExpMenu`, só a figura). 0 = o botão não abre nada naquele tipo de jogo.
    #[serde(default)]
    pub janela_quick: u16,
    #[serde(default)]
    pub janela_completo: u16,
}

/// Qual dos menus do botão de ferramentas está aberto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualMenu {
    /// Janela 9 com os submenus (Experimental).
    Experimental,
    /// Janela 10, um item (Quick Game).
    Quick,
    /// Janela 11, sem item (Jogo Completo).
    Completo,
}

/// `_RibbonToolsMenu` abre a janela 9 em x = (direita do botão − largura + esquerda) / 2; a janela tem
/// contorno preto e 4 linhas azuis em volta da área do objeto 0, que começa no canto de dentro
/// [medido em i05: contorno em (307, 74) da área cliente; o port bate 100% com a captura].
#[derive(Clone, Debug, Deserialize)]
pub struct MolduraMenu {
    /// Linha do contorno na área cliente.
    pub topo: i16,
    /// Soma à coluna calculada pela conta do original.
    pub ajuste_x: i16,
    pub largura_borda: i16,
    pub cor_borda: u8,
    pub cor_contorno: u8,
    /// Canto do objeto 0 em relação ao canto de dentro da moldura.
    pub ajuste_conteudo: [i16; 2],
}

/// Item escolhido.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemMenu {
    Ferramenta(Ferramenta),
    /// Subopção da ferramenta (índice do item no submenu).
    Subopcao(Ferramenta, u8),
    /// O item `editToExpMode` da janela 10: o Quick Game vira Experimental.
    ParaExperimental,
}

/// O menu aberto: onde está e qual submenu está aberto.
pub struct MenuFerramentas {
    pub cfg: ConfigMenuFerramentas,
    menu: Janela,
    submenus: Vec<Option<Janela>>,
    /// Janelas 10 e 11 (`None` sem a janela no `cfg`).
    quick: Option<Janela>,
    completo: Option<Janela>,
    /// Qual menu foi aberto por último.
    pub qual: QualMenu,
    /// Janela 10: o item está realçado (o mouse sobre ele).
    pub realce_quick: bool,
    /// Canto do objeto 0 do menu na área cliente.
    pub origem: (i16, i16),
    /// Submenu aberto (ferramenta) e o canto dele.
    pub submenu: Option<(Ferramenta, (i16, i16))>,
    /// Item realçado (`__win_SetProxItem`): ao abrir, o da ferramenta atual; depois, o que está sob
    /// o mouse.
    pub realce: Option<Ferramenta>,
    /// Contorno externo da moldura, quando há (`cfg.moldura`).
    pub externo: Option<Retangulo>,
}

fn deslocar(r: Retangulo, base: Retangulo, origem: (i16, i16)) -> Retangulo {
    let (dx, dy) = (origem.0 - base.esq, origem.1 - base.topo);
    Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy }
}

fn contem(r: Retangulo, x: i32, y: i32) -> bool {
    x >= i32::from(r.esq) && x < i32::from(r.dir) && y >= i32::from(r.topo) && y < i32::from(r.base)
}

impl MenuFerramentas {
    pub fn carregar(recursos: &Recursos) -> Result<MenuFerramentas, String> {
        let cfg: ConfigMenuFerramentas = recursos.dados(ARQUIVO_CONFIG)?;
        let menu = carregar_janela(recursos, cfg.janela)?;
        let submenus = cfg.submenus.iter().map(|&id| if id == 0 { Ok(None) } else { carregar_janela(recursos, id).map(Some) }).collect::<Result<_, _>>()?;
        let opcional = |id: u16| if id == 0 { Ok(None) } else { carregar_janela(recursos, id).map(Some) };
        let (quick, completo) = (opcional(cfg.janela_quick)?, opcional(cfg.janela_completo)?);
        Ok(MenuFerramentas {
            cfg,
            menu,
            submenus,
            quick,
            completo,
            qual: QualMenu::Experimental,
            realce_quick: false,
            origem: (0, 0),
            submenu: None,
            realce: None,
            externo: None,
        })
    }

    /// A janela do menu aberto.
    fn janela(&self) -> &Janela {
        match self.qual {
            QualMenu::Experimental => &self.menu,
            QualMenu::Quick => self.quick.as_ref().unwrap_or(&self.menu),
            QualMenu::Completo => self.completo.as_ref().unwrap_or(&self.menu),
        }
    }

    /// Abre o menu do Quick Game (janela 10) ou do Jogo Completo (janela 11) junto ao botão, na mesma
    /// conta de `_RibbonToolsMenu` e com a mesma moldura da janela 9 (as três têm os mesmos bits de
    /// estilo, 0x41/0x18). Devolve false se a janela não está no `cfg` (ou com `Experimental`, que
    /// abre por [`MenuFerramentas::abrir`]).
    pub fn abrir_por_tipo(&mut self, qual: QualMenu, botao: Retangulo) -> bool {
        let existe = match qual {
            QualMenu::Experimental => false,
            QualMenu::Quick => self.quick.is_some(),
            QualMenu::Completo => self.completo.is_some(),
        };
        if !existe {
            return false;
        }
        self.qual = qual;
        self.submenu = None;
        self.realce = None;
        self.realce_quick = false;
        self.posicionar(botao);
        true
    }

    /// Abre o menu junto ao botão (retângulo na área cliente), com o item da ferramenta `atual`
    /// realçado.
    pub fn abrir(&mut self, botao: Retangulo, atual: Option<Ferramenta>) {
        self.qual = QualMenu::Experimental;
        self.submenu = None;
        self.realce = atual;
        self.posicionar(botao);
    }

    /// `_RibbonToolsMenu`: x = (direita do botão − largura + esquerda) / 2 (com a moldura medida).
    fn posicionar(&mut self, botao: Retangulo) {
        match &self.cfg.moldura {
            Some(m) => {
                let r = self.janela().objetos[0].retangulo;
                let (w, h) = (r.dir - r.esq, r.base - r.topo);
                let b = m.largura_borda;
                let esq = (botao.dir - w + botao.esq) / 2 + m.ajuste_x;
                let ext = Retangulo { esq, topo: m.topo, dir: esq + w + 2 * b, base: m.topo + h + 2 * b };
                self.externo = Some(ext);
                self.origem = (ext.esq + b + m.ajuste_conteudo[0], ext.topo + b + m.ajuste_conteudo[1]);
            }
            None => {
                self.externo = None;
                self.origem = (botao.esq + self.cfg.deslocamento[0], botao.base + self.cfg.deslocamento[1]);
            }
        }
    }

    fn itens(&self) -> impl Iterator<Item = (Ferramenta, Retangulo)> + '_ {
        let base = self.menu.retangulo();
        self.menu
            .objetos
            .iter()
            .skip(self.cfg.primeiro_item)
            .zip(Ferramenta::TODAS)
            .map(move |(o, f)| (f, deslocar(o.retangulo, base, self.origem)))
    }

    fn itens_submenu(&self) -> Vec<(u8, Retangulo)> {
        let Some((f, origem)) = self.submenu else { return Vec::new() };
        let Some(Some(j)) = self.submenus.get(f.indice()) else { return Vec::new() };
        let base = j.retangulo();
        j.objetos.iter().skip(self.cfg.primeiro_item_submenu).enumerate().map(|(i, o)| (i as u8, deslocar(o.retangulo, base, origem))).collect()
    }

    /// O item da janela 10 (o objeto `primeiro_item`), na tela.
    fn item_quick(&self) -> Option<Retangulo> {
        let j = self.quick.as_ref()?;
        let base = j.retangulo();
        j.objetos.get(self.cfg.primeiro_item).map(|o| deslocar(o.retangulo, base, self.origem))
    }

    /// Item sob o ponto. Um item com submenu abre o submenu ao lado e devolve a ferramenta.
    pub fn item_em(&mut self, x: i32, y: i32) -> Option<ItemMenu> {
        match self.qual {
            QualMenu::Experimental => {}
            QualMenu::Quick => {
                self.realce_quick = self.item_quick().is_some_and(|r| contem(r, x, y));
                return self.realce_quick.then_some(ItemMenu::ParaExperimental);
            }
            // `_win_DoProxMenu` só devolve objetos a partir do 2: a figura não é item.
            QualMenu::Completo => return None,
        }
        if let Some((f, _)) = self.submenu {
            if let Some((i, _)) = self.itens_submenu().into_iter().find(|(_, r)| contem(*r, x, y)) {
                return Some(ItemMenu::Subopcao(f, i));
            }
        }
        let (f, r) = self.itens().find(|(_, r)| contem(*r, x, y))?;
        self.submenu = (self.cfg.submenus[f.indice()] != 0).then_some((f, (r.dir, r.topo)));
        self.realce = Some(f);
        Some(ItemMenu::Ferramenta(f))
    }

    /// O ponto cai no menu ou no submenu aberto.
    pub fn contem(&self, x: i32, y: i32) -> bool {
        let base = self.janela().retangulo();
        self.externo.is_some_and(|r| contem(r, x, y))
            || contem(deslocar(base, base, self.origem), x, y)
            || self.submenu.is_some_and(|(f, o)| {
                self.submenus.get(f.indice()).and_then(|j| j.as_ref()).is_some_and(|j| contem(deslocar(j.retangulo(), j.retangulo(), o), x, y))
            })
    }

    /// Desenha o menu (e o submenu), com o item realçado invertido (na bancada, i05: o item da
    /// ferramenta atual aparece com as cores invertidas).
    pub fn desenhar(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface) -> Vec<String> {
        let interno = match (&self.cfg.moldura, self.externo) {
            (Some(m), Some(ext)) => {
                let b = m.largura_borda;
                q.preencher(ext, m.cor_borda, m.cor_borda);
                q.contorno(Retangulo { dir: ext.dir - 1, base: ext.base - 1, ..ext }, m.cor_contorno);
                Some(Retangulo { esq: ext.esq + b, topo: ext.topo + b, dir: ext.dir - b, base: ext.base - b })
            }
            _ => None,
        };
        let mut pendentes = desenhar_objetos(q, recursos, interface, self.janela(), self.origem, interno);
        let realcado = match self.qual {
            QualMenu::Experimental => self.itens().find(|(f, _)| Some(*f) == self.realce).map(|(_, r)| r),
            QualMenu::Quick => self.item_quick().filter(|_| self.realce_quick),
            QualMenu::Completo => None,
        };
        if let Some(r) = realcado {
            let antes = q.recorte;
            if let Some(i) = interno {
                q.recorte = Some(i);
            }
            q.inverter(r);
            q.recorte = antes;
        }
        if let Some((f, o)) = self.submenu {
            if let Some(Some(j)) = self.submenus.get(f.indice()) {
                pendentes.extend(desenhar_objetos(q, recursos, interface, j, o, None));
            }
        }
        pendentes
    }
}
