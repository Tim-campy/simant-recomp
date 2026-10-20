//! Menus de "segurar o botão": o menu da formiga amarela (`_AntMenu`, janelas 7, 8 e 32) e o
//! menu de ícones dos botões da faixa (Ctrl+clique, `_DoUserButton`, janela 36).
//!
//! Especificação: docs/spec/telas/janelas.md, "Menus". Dados: data/menus.json. O menu abre num
//! ponto, segue o cursor realçando o item (`__win_SetProxItem`, `_GRectInv`) e devolve o item sob o
//! cursor quando o botão é solto (`_win_DoProxMenu`).

use serde::Deserialize;

use super::quadro::Quadro;
use super::telas::{carregar_janela, desenhar_objetos, Interface};
use crate::formats::janela::Retangulo;
use crate::recursos::Recursos;

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_CONFIG: &str = "menus.json";

/// Um menu: janela, objetos dos itens na ordem do índice devolvido e o código de cada item.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigMenu {
    pub janela: u16,
    pub itens: Vec<String>,
    /// `_YellowCommand(código)` de cada item (menu da formiga) ou função do botão (menu de ícones).
    pub codigos: Vec<u8>,
    /// Canto externo em relação ao clique, quando difere de `deslocamento_formiga` [medido].
    #[serde(default)]
    pub deslocamento: Option<[i16; 2]>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigMenus {
    /// `_AntMenu` com a amarela formiga (janela 7), rainha sem ninho (8) e aranha (32).
    pub formiga: ConfigMenu,
    pub rainha: ConfigMenu,
    pub aranha: ConfigMenu,
    /// Menu de ícones dos botões da faixa (janela 36).
    pub icones: ConfigMenu,
    /// Canto externo do menu da formiga em relação ao clique (área cliente) [medido].
    pub deslocamento_formiga: [i16; 2],
    /// Canto externo do menu de ícones: x do botão + [0], y = [1] [medido].
    pub deslocamento_icones: [i16; 2],
    /// Recrutar 10 (código 2) com Shift vira "todas" (3) (`_AntMenu`).
    pub codigo_com_shift: [u8; 2],
}

/// Um menu aberto.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuAberto {
    pub janela: u16,
    pub externo: Retangulo,
    pub interior: Retangulo,
    /// Retângulo de cada item (área cliente).
    pub itens: Vec<Retangulo>,
    pub codigos: Vec<u8>,
    pub realcado: Option<usize>,
}

fn contem(r: Retangulo, x: i32, y: i32) -> bool {
    x >= i32::from(r.esq) && x < i32::from(r.dir) && y >= i32::from(r.topo) && y < i32::from(r.base)
}

impl MenuAberto {
    /// Abre o menu `cfg` com o canto externo em (x, y), mantido dentro de `limite`
    /// (`_win_DoProxMenu`: se passa da direita ou de baixo, volta pelo tamanho do menu).
    pub fn abrir(recursos: &Recursos, interface: &Interface, cfg: &ConfigMenu, x: i32, y: i32, limite: Retangulo) -> Result<MenuAberto, String> {
        let j = carregar_janela(recursos, cfg.janela)?;
        let base = j.retangulo();
        let b = interface.moldura.borda.largura as i32;
        let (w, h) = (i32::from(base.dir - base.esq) + 2 * b, i32::from(base.base - base.topo) + 2 * b);
        let mut x = x;
        let mut y = y;
        if x + w > i32::from(limite.dir) {
            x = (x - w).max(i32::from(limite.esq));
        }
        if y + h > i32::from(limite.base) {
            y = (y - h).max(i32::from(limite.topo));
        }
        let externo = Retangulo { esq: x as i16, topo: y as i16, dir: (x + w) as i16, base: (y + h) as i16 };
        let interior = Retangulo { esq: (x + b) as i16, topo: (y + b) as i16, dir: (x + w - b) as i16, base: (y + h - b) as i16 };
        let (dx, dy) = (interior.esq - base.esq, interior.topo - base.topo);
        let itens = cfg
            .itens
            .iter()
            .map(|n| {
                j.objetos.iter().find(|o| &o.nome == n).map(|o| {
                    let r = o.retangulo;
                    Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy }
                })
            })
            .collect::<Option<Vec<_>>>()
            .ok_or(format!("window {}: menu item missing", cfg.janela))?;
        Ok(MenuAberto { janela: cfg.janela, externo, interior, itens, codigos: cfg.codigos.clone(), realcado: None })
    }

    /// Cursor em (x, y) com o botão apertado: realça o item sob ele. Devolve se mudou.
    pub fn mover(&mut self, x: i32, y: i32) -> bool {
        let novo = self.itens.iter().position(|r| contem(*r, x, y));
        std::mem::replace(&mut self.realcado, novo) != novo
    }

    /// Botão solto em (x, y): o código do item sob o cursor, se houver.
    pub fn soltar(&self, x: i32, y: i32) -> Option<u8> {
        let i = self.itens.iter().position(|r| contem(*r, x, y))?;
        self.codigos.get(i).copied()
    }

    /// Desenha o menu: moldura, bitmap do layout e o item realçado invertido (`_GRectInv`).
    pub fn desenhar(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface) -> Result<Vec<String>, String> {
        let j = carregar_janela(recursos, self.janela)?;
        let b = &interface.moldura.borda;
        q.preencher(self.externo, b.cor, b.cor);
        q.contorno(Retangulo { dir: self.externo.dir - 1, base: self.externo.base - 1, ..self.externo }, b.cor_externa);
        let p = desenhar_objetos(q, recursos, interface, &j, (self.interior.esq, self.interior.topo), Some(self.interior));
        if let Some(r) = self.realcado.and_then(|i| self.itens.get(i)) {
            q.inverter(*r);
        }
        Ok(p)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn menu() -> MenuAberto {
        let r = |y: i16| Retangulo { esq: 10, topo: y, dir: 50, base: y + 10 };
        MenuAberto {
            janela: 7,
            externo: Retangulo { esq: 0, topo: 0, dir: 60, base: 40 },
            interior: Retangulo { esq: 5, topo: 5, dir: 55, base: 35 },
            itens: vec![r(5), r(15), r(25)],
            codigos: vec![1, 2, 0],
            realcado: None,
        }
    }

    #[test]
    fn realca_e_escolhe_o_item_sob_o_cursor() {
        let mut m = menu();
        assert!(m.mover(20, 17));
        assert_eq!(m.realcado, Some(1));
        assert!(!m.mover(21, 18));
        assert_eq!(m.soltar(20, 17), Some(2));
        assert_eq!(m.soltar(200, 17), None);
        assert!(m.mover(200, 17));
        assert_eq!(m.realcado, None);
    }
}
