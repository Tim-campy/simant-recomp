//! Menu de ferramentas do Experimental (janela 9 `editToolsMenu`) e os submenus das ferramentas
//! (janelas 12–17), sem SDL: posição, desenho e qual item está sob o mouse.
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
    /// Canto do menu relativo ao canto inferior esquerdo do botão [hipótese: `_win_DoProxMenu`
    /// abre o menu junto ao botão; posição não medida].
    pub deslocamento: [i16; 2],
    /// Cor do contorno do item atual (`__win_SetProxItem`).
    pub cor_item_atual: u8,
}

/// Item escolhido.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemMenu {
    Ferramenta(Ferramenta),
    /// Subopção da ferramenta (índice do item no submenu).
    Subopcao(Ferramenta, u8),
}

/// O menu aberto: onde está e qual submenu está aberto.
pub struct MenuFerramentas {
    pub cfg: ConfigMenuFerramentas,
    menu: Janela,
    submenus: Vec<Option<Janela>>,
    /// Canto do objeto 0 do menu na área cliente.
    pub origem: (i16, i16),
    /// Submenu aberto (ferramenta) e o canto dele.
    pub submenu: Option<(Ferramenta, (i16, i16))>,
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
        Ok(MenuFerramentas { cfg, menu, submenus, origem: (0, 0), submenu: None })
    }

    /// Abre o menu junto ao botão (retângulo na área cliente).
    pub fn abrir(&mut self, botao: Retangulo) {
        self.origem = (botao.esq + self.cfg.deslocamento[0], botao.base + self.cfg.deslocamento[1]);
        self.submenu = None;
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

    /// Item sob o ponto. Um item com submenu abre o submenu ao lado e devolve a ferramenta.
    pub fn item_em(&mut self, x: i32, y: i32) -> Option<ItemMenu> {
        if let Some((f, _)) = self.submenu {
            if let Some((i, _)) = self.itens_submenu().into_iter().find(|(_, r)| contem(*r, x, y)) {
                return Some(ItemMenu::Subopcao(f, i));
            }
        }
        let (f, r) = self.itens().find(|(_, r)| contem(*r, x, y))?;
        self.submenu = (self.cfg.submenus[f.indice()] != 0).then_some((f, (r.dir, r.topo)));
        Some(ItemMenu::Ferramenta(f))
    }

    /// O ponto cai no menu ou no submenu aberto.
    pub fn contem(&self, x: i32, y: i32) -> bool {
        let base = self.menu.retangulo();
        contem(deslocar(base, base, self.origem), x, y)
            || self.submenu.is_some_and(|(f, o)| {
                self.submenus.get(f.indice()).and_then(|j| j.as_ref()).is_some_and(|j| contem(deslocar(j.retangulo(), j.retangulo(), o), x, y))
            })
    }

    /// Desenha o menu (e o submenu), com um contorno no item da ferramenta atual.
    pub fn desenhar(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface, atual: Option<Ferramenta>) -> Vec<String> {
        let mut pendentes = desenhar_objetos(q, recursos, interface, &self.menu, self.origem, None);
        if let Some((_, r)) = self.itens().find(|(f, _)| Some(*f) == atual) {
            q.contorno(Retangulo { dir: r.dir - 1, base: r.base - 1, ..r }, self.cfg.cor_item_atual);
        }
        if let Some((f, o)) = self.submenu {
            if let Some(Some(j)) = self.submenus.get(f.indice()) {
                pendentes.extend(desenhar_objetos(q, recursos, interface, j, o, None));
            }
        }
        pendentes
    }
}
