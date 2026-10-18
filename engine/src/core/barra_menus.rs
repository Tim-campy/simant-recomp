//! Barra de menus da janela principal (File, Window, View, Options, Speed), como o Windows 3.1 a desenha
//! e trata: títulos com o mnemônico sublinhado, menus suspensos com o atalho numa coluna, marcas de
//! seleção (`CheckMenuItem` de `_SetMenuEntries`), navegação pelo mouse (apertar, arrastar, soltar) e
//! pelo teclado (Alt, F10, Alt+letra, setas, Enter, Esc). Os textos vêm do objeto 6/0 do banco
//! (`_InitMenu`, formats/menu.rs); o que cada comando faz é `_ProcMenu` (a interface executa).
//!
//! Coordenadas: as da janela do port, com a barra em cima (linhas 0..`altura_barra`), a linha preta
//! embaixo dela e a área cliente a partir de `altura_total()`. Especificação: docs/spec/telas/menus.md.

use std::collections::HashMap;

use serde::Deserialize;

use super::gerenciador::{mnemonico, texto_a_esquerda};
use super::quadro::Quadro;
use crate::formats::fonte::Fonte;
use crate::formats::janela::Retangulo;
use crate::formats::menu::{comando, MenuBanco, SEPARADOR};

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_CONFIG: &str = "barra_menus.json";

/// O que um item faz (`_ProcMenu`, pelo número do comando; a tabela fica em data/barra_menus.json).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ComandoMenu {
    Sobre,
    Novo,
    Carregar,
    Salvar,
    SalvarComo,
    Sair,
    Edicao,
    Mapa,
    Comportamento,
    Castas,
    Historico,
    Info,
    Status,
    Ajuda,
    Quintal,
    Casa,
    Estrategico,
    Grafico,
    Superficie,
    NinhoPreto,
    NinhoVermelho,
    AutoTrack,
    Musica,
    Efeitos,
    Eventos,
    Mensagens,
    Silly,
    Pausa,
    Lenta,
    Normal,
    Rapida,
    Ultra,
}

impl ComandoMenu {
    /// Índice em `_OptionStates` dos itens do Options (`0xFD31`–`0xFD36`).
    pub fn opcao(self) -> Option<usize> {
        use ComandoMenu::*;
        [AutoTrack, Musica, Efeitos, Eventos, Mensagens, Silly].iter().position(|&c| c == self)
    }

    /// Velocidade (`_SimSpeed`) dos itens do Speed (`0xFD43`–`0xFD46`).
    pub fn velocidade(self) -> Option<i16> {
        use ComandoMenu::*;
        [Lenta, Normal, Rapida, Ultra].iter().position(|&c| c == self).map(|v| v as i16)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CoresMenu {
    pub fundo: u8,
    pub texto: u8,
    pub realce: u8,
    pub texto_realce: u8,
    pub borda: u8,
}

/// Tecla (nome do SDL) com Ctrl/Shift que dispara um comando do menu.
#[derive(Clone, Debug, Deserialize)]
pub struct AceleradorMenu {
    pub tecla: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    pub comando: ComandoMenu,
}

impl ConfigBarraMenus {
    /// Comando do acelerador da tecla (sem Alt).
    pub fn acelerador(&self, nome: &str, ctrl: bool, shift: bool) -> Option<ComandoMenu> {
        self.aceleradores.iter().find(|a| a.tecla.eq_ignore_ascii_case(nome) && a.ctrl == ctrl && a.shift == shift).map(|a| a.comando)
    }
}

/// Conteúdo de data/barra_menus.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigBarraMenus {
    /// Id do objeto tipo 6 (`_InitMenu(0)`).
    pub objeto_menu: u16,
    /// Índices de `interface.json`, o primeiro que existir (6 = System do Windows).
    pub fontes: Vec<usize>,
    /// Linhas da barra (sem a linha preta de baixo).
    pub altura_barra: i16,
    /// Distância do texto do título às bordas do realce.
    pub margem_titulo: i16,
    pub altura_item: i16,
    pub altura_separador: i16,
    pub linha_separador: i16,
    /// Distância do texto do item à borda esquerda do menu.
    pub margem_texto: i16,
    /// Folga entre o texto mais largo (antes do tab) e a coluna dos atalhos.
    pub espaco_atalho: i16,
    /// Folga depois do atalho mais largo (ou do texto, sem atalhos) até a borda direita (inclusive).
    pub margem_direita: i16,
    /// Marca de seleção: linhas de `#` e `.`, com o canto em (`marca_x`, `marca_y`) do item.
    pub marca: Vec<String>,
    pub marca_x: i16,
    pub marca_y: i16,
    pub cores: CoresMenu,
    /// Texto do item Pause sem e com o jogo pausado (`_SetMenuOptionText(0x41, …)`, DGROUP 0x911 e 0x900).
    pub texto_pausa: [String; 2],
    /// Comando (hexadecimal, "0xFD01") → o que faz.
    pub comandos: HashMap<String, ComandoMenu>,
    /// Aceleradores da ACCELERATORTABLE que só existem pelo menu (os outros ficam em partida.json,
    /// navegacao.json, modos.json e quintal_mapa.json).
    #[serde(default)]
    pub aceleradores: Vec<AceleradorMenu>,
    /// Música do File → New (`_myBeginSong(0x2711)` antes de `_NewGame(0)`).
    #[serde(default)]
    pub musica_novo: Option<u16>,
    /// Window → Status (`_ScoreDialog`).
    pub status: super::status::ConfigStatus,
    /// File → About (`_AboutDialog`).
    pub sobre: super::sobre::ConfigSobre,
}

impl ConfigBarraMenus {
    /// O que faz o comando `c` (None para os separadores e números sem uso).
    pub fn comando(&self, c: u16) -> Option<ComandoMenu> {
        self.comandos.iter().find(|(k, _)| u16::from_str_radix(k.trim_start_matches("0x").trim_start_matches("0X"), 16).ok() == Some(c)).map(|(_, &v)| v)
    }
}

/// Estado do jogo que o menu mostra (`_SetMenuEntries`): marcas do Options e do Speed e o texto do Pause.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EstadoMenus {
    pub opcoes: [bool; 6],
    pub velocidade: i16,
    pub pausado: bool,
}

/// Onde está a barra.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estado {
    Fechado,
    /// Título realçado sem menu aberto (Alt ou F10, ou Esc com um menu aberto).
    Barra(usize),
    /// Menu aberto, com o item realçado (se houver).
    Aberto { menu: usize, item: Option<usize> },
}

/// O que a interface faz depois de uma entrada na barra.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resposta {
    /// A entrada não era da barra (segue para o jogo).
    Ignorada,
    /// Tratada; `bool` = precisa redesenhar.
    Tratada(bool),
    /// Item escolhido (a barra já fechou).
    Comando(ComandoMenu),
}

/// A barra de menus: configuração, textos do banco e estado.
#[derive(Clone, Debug)]
pub struct BarraMenus {
    pub cfg: ConfigBarraMenus,
    pub menu: MenuBanco,
    pub estado: Estado,
    /// Botão do mouse apertado com a barra ativa (arrasto pelos itens).
    arrastando: bool,
    /// O menu já estava aberto quando o título foi apertado (o soltar no título fecha).
    reapertou: bool,
}

fn contem(r: Retangulo, x: i32, y: i32) -> bool {
    x >= i32::from(r.esq) && x < i32::from(r.dir) && y >= i32::from(r.topo) && y < i32::from(r.base)
}

/// Texto sem `&`, como a fonte o desenha.
fn sem_e(texto: &str) -> Vec<u8> {
    texto.bytes().filter(|&b| b != b'&').collect()
}

impl BarraMenus {
    pub fn new(cfg: ConfigBarraMenus, menu: MenuBanco) -> BarraMenus {
        BarraMenus { cfg, menu, estado: Estado::Fechado, arrastando: false, reapertou: false }
    }

    /// Linhas ocupadas em cima da área cliente: a barra e a linha preta.
    pub fn altura_total(&self) -> i16 {
        self.cfg.altura_barra + 1
    }

    pub fn ativa(&self) -> bool {
        self.estado != Estado::Fechado
    }

    /// Texto do item (o Pause muda com o jogo pausado).
    pub fn texto_item(&self, m: usize, i: usize, e: &EstadoMenus) -> String {
        let t = self.menu.itens[m][i].clone();
        if self.cfg.comando(comando(m, i)) == Some(ComandoMenu::Pausa) {
            return self.cfg.texto_pausa[usize::from(e.pausado)].clone();
        }
        t
    }

    fn separador(&self, m: usize, i: usize) -> bool {
        self.menu.itens[m][i] == SEPARADOR
    }

    /// O item tem marca de seleção.
    pub fn marcado(&self, m: usize, i: usize, e: &EstadoMenus) -> bool {
        match self.cfg.comando(comando(m, i)) {
            Some(c) => c.opcao().map(|k| e.opcoes[k]).or(c.velocidade().map(|v| v == e.velocidade)).unwrap_or(false),
            None => false,
        }
    }

    /// Retângulo do realce de cada título (a largura do texto mais as margens, um depois do outro).
    pub fn titulos(&self, f: &Fonte) -> Vec<Retangulo> {
        let mut x = 0i16;
        self.menu
            .titulos
            .iter()
            .map(|t| {
                let w = f.largura_texto(&sem_e(t)) as i16 + 2 * self.cfg.margem_titulo;
                let r = Retangulo { esq: x, topo: 0, dir: x + w, base: self.cfg.altura_barra };
                x += w;
                r
            })
            .collect()
    }

    /// Larguras do texto antes e depois do tab dos itens do menu `m`.
    fn colunas(&self, m: usize, f: &Fonte, e: &EstadoMenus) -> (i16, i16) {
        let (mut l, mut r) = (0, 0);
        for i in 0..self.menu.itens[m].len() {
            if self.separador(m, i) {
                continue;
            }
            let t = self.texto_item(m, i, e);
            let (a, b) = t.split_once('\t').unwrap_or((&t, ""));
            l = l.max(f.largura_texto(&sem_e(a)) as i16);
            r = r.max(f.largura_texto(&sem_e(b)) as i16);
        }
        (l, r)
    }

    /// Contorno do menu `m` (borda incluída), a coluna dos atalhos (relativa à borda esquerda) e o
    /// topo de cada item (relativo ao topo do contorno).
    pub fn geometria_menu(&self, m: usize, f: &Fonte, e: &EstadoMenus) -> (Retangulo, i16, Vec<(i16, i16)>) {
        let c = &self.cfg;
        let titulo = self.titulos(f)[m];
        let (l, r) = self.colunas(m, f, e);
        let coluna = c.margem_texto + l + c.espaco_atalho;
        let largura = if r > 0 { coluna + r + c.margem_direita } else { c.margem_texto + l + c.margem_direita };
        let mut y = 1;
        let mut itens = Vec::new();
        for i in 0..self.menu.itens[m].len() {
            let h = if self.separador(m, i) { c.altura_separador } else { c.altura_item };
            itens.push((y, h));
            y += h;
        }
        let topo = c.altura_barra;
        let ret = Retangulo { esq: titulo.esq, topo, dir: titulo.esq + largura, base: topo + y + 1 };
        (ret, coluna, itens)
    }

    fn primeiro_item(&self, m: usize) -> Option<usize> {
        (0..self.menu.itens[m].len()).find(|&i| !self.separador(m, i))
    }

    fn abrir(&mut self, m: usize, realcar: bool) {
        let item = if realcar { self.primeiro_item(m) } else { None };
        self.estado = Estado::Aberto { menu: m, item };
    }

    fn fechar(&mut self) {
        self.estado = Estado::Fechado;
        self.arrastando = false;
    }

    fn titulo_em(&self, f: &Fonte, x: i32, y: i32) -> Option<usize> {
        self.titulos(f).iter().position(|r| contem(*r, x, y))
    }

    /// Item do menu aberto sob (x, y) (separadores contam como "dentro, sem item").
    fn item_em(&self, f: &Fonte, e: &EstadoMenus, x: i32, y: i32) -> Option<Option<usize>> {
        let Estado::Aberto { menu, .. } = self.estado else { return None };
        let (r, _, itens) = self.geometria_menu(menu, f, e);
        if !contem(r, x, y) {
            return None;
        }
        let dy = (y - i32::from(r.topo)) as i16;
        Some(itens.iter().position(|&(t, h)| dy >= t && dy < t + h).filter(|&i| !self.separador(menu, i)))
    }

    /// Botão esquerdo apertado em (x, y) da janela.
    pub fn apertar(&mut self, f: &Fonte, e: &EstadoMenus, x: i32, y: i32) -> Resposta {
        if let Some(m) = self.titulo_em(f, x, y) {
            self.reapertou = matches!(self.estado, Estado::Aberto { menu, .. } if menu == m);
            self.abrir(m, false);
            self.arrastando = true;
            return Resposta::Tratada(true);
        }
        if !self.ativa() {
            return Resposta::Ignorada;
        }
        if let Some(item) = self.item_em(f, e, x, y) {
            if let Estado::Aberto { menu, .. } = self.estado {
                self.estado = Estado::Aberto { menu, item };
            }
            self.arrastando = true;
            self.reapertou = false;
            return Resposta::Tratada(true);
        }
        // Fora da barra e do menu: fecha, e o clique não vai para o jogo.
        self.fechar();
        Resposta::Tratada(true)
    }

    /// Mouse andando: com o botão apertado, o realce segue o cursor (título ou item).
    pub fn mover(&mut self, f: &Fonte, e: &EstadoMenus, x: i32, y: i32) -> bool {
        if !self.arrastando {
            return false;
        }
        let antes = self.estado;
        if let Some(m) = self.titulo_em(f, x, y) {
            if !matches!(self.estado, Estado::Aberto { menu, .. } if menu == m) {
                self.reapertou = false;
                self.abrir(m, false);
            }
        } else if let Estado::Aberto { menu, .. } = self.estado {
            let item = self.item_em(f, e, x, y).flatten();
            self.estado = Estado::Aberto { menu, item };
        }
        self.estado != antes
    }

    /// Botão solto: num item, escolhe; no título, o menu fica aberto com o primeiro item realçado
    /// (ou fecha, se já estava aberto antes do clique); fora, fecha.
    pub fn soltar(&mut self, f: &Fonte, e: &EstadoMenus, x: i32, y: i32) -> Resposta {
        if !self.arrastando {
            return Resposta::Ignorada;
        }
        self.arrastando = false;
        if let Some(m) = self.titulo_em(f, x, y) {
            if self.reapertou {
                self.fechar();
            } else {
                self.abrir(m, true);
            }
            return Resposta::Tratada(true);
        }
        match self.item_em(f, e, x, y) {
            Some(Some(i)) => self.escolher(i),
            Some(None) => Resposta::Tratada(false),
            None => {
                self.fechar();
                Resposta::Tratada(true)
            }
        }
    }

    fn escolher(&mut self, i: usize) -> Resposta {
        let Estado::Aberto { menu, .. } = self.estado else { return Resposta::Tratada(false) };
        self.fechar();
        match self.cfg.comando(comando(menu, i)) {
            Some(c) => Resposta::Comando(c),
            None => Resposta::Tratada(true),
        }
    }

    /// Alt (ou F10) apertado e solto sozinho: liga ou desliga a barra (título File realçado).
    pub fn alternar(&mut self) -> Resposta {
        if self.ativa() {
            self.fechar();
        } else {
            self.estado = Estado::Barra(0);
        }
        Resposta::Tratada(true)
    }

    /// Menu do título com a letra sublinhada `letra` (Alt+letra, ou a letra com a barra ativa).
    fn menu_da_letra(&self, letra: &str) -> Option<usize> {
        let l = letra.to_ascii_lowercase();
        self.menu.titulos.iter().position(|t| mnemonico(t).is_some_and(|c| l.len() == 1 && l.starts_with(c)))
    }

    /// Tecla (nome do SDL). `alt`: Alt apertado junto.
    pub fn tecla(&mut self, nome: &str, alt: bool) -> Resposta {
        let n = self.menu.titulos.len();
        if n == 0 {
            return Resposta::Ignorada;
        }
        let lower = nome.to_ascii_lowercase();
        if alt && !matches!(lower.as_str(), "left alt" | "right alt") {
            // Alt+letra abre o menu com o primeiro item realçado; outras combinações com Alt não são da barra.
            return match self.menu_da_letra(nome) {
                Some(m) => {
                    self.abrir(m, true);
                    Resposta::Tratada(true)
                }
                None => {
                    if self.ativa() {
                        self.fechar();
                        return Resposta::Tratada(true);
                    }
                    Resposta::Ignorada
                }
            };
        }
        match self.estado {
            Estado::Fechado => {
                if lower == "f10" {
                    return self.alternar();
                }
                Resposta::Ignorada
            }
            Estado::Barra(t) => {
                match lower.as_str() {
                    "left" => self.estado = Estado::Barra((t + n - 1) % n),
                    "right" => self.estado = Estado::Barra((t + 1) % n),
                    "down" | "up" | "return" | "keypad enter" => self.abrir(t, true),
                    "escape" | "f10" => self.fechar(),
                    _ => {
                        if let Some(m) = self.menu_da_letra(nome) {
                            self.abrir(m, true);
                        }
                    }
                }
                Resposta::Tratada(true)
            }
            Estado::Aberto { menu, item } => {
                let validos: Vec<usize> = (0..self.menu.itens[menu].len()).filter(|&i| !self.separador(menu, i)).collect();
                let k = validos.len();
                let atual = item.and_then(|i| validos.iter().position(|&v| v == i));
                match lower.as_str() {
                    "up" | "down" if k > 0 => {
                        let novo = match (atual, lower == "up") {
                            (Some(a), true) => (a + k - 1) % k,
                            (Some(a), false) => (a + 1) % k,
                            (None, true) => k - 1,
                            (None, false) => 0,
                        };
                        self.estado = Estado::Aberto { menu, item: Some(validos[novo]) };
                    }
                    "left" => self.abrir((menu + n - 1) % n, true),
                    "right" => self.abrir((menu + 1) % n, true),
                    "return" | "keypad enter" => {
                        if let Some(i) = item {
                            return self.escolher(i);
                        }
                    }
                    "escape" => self.estado = Estado::Barra(menu),
                    "f10" => self.fechar(),
                    _ => {
                        let l = lower.clone();
                        let achado = (0..self.menu.itens[menu].len())
                            .find(|&i| !self.separador(menu, i) && mnemonico(&self.menu.itens[menu][i]).is_some_and(|c| l.len() == 1 && l.starts_with(c)));
                        if let Some(i) = achado {
                            return self.escolher(i);
                        }
                    }
                }
                Resposta::Tratada(true)
            }
        }
    }

    /// Desenha a barra (linhas 0..`altura_total`) e o menu aberto, por cima do que já está no quadro
    /// (a área cliente começa em `altura_total`).
    pub fn desenhar(&self, q: &mut Quadro, f: Option<&Fonte>, e: &EstadoMenus) {
        let c = &self.cfg;
        let largura = q.largura as i16;
        let h = c.altura_barra;
        q.preencher(Retangulo { esq: 0, topo: 0, dir: largura, base: h }, c.cores.fundo, c.cores.fundo);
        q.preencher(Retangulo { esq: 0, topo: h, dir: largura, base: h + 1 }, c.cores.borda, c.cores.borda);
        let Some(f) = f else { return };
        let realce = match self.estado {
            Estado::Barra(t) => Some(t),
            Estado::Aberto { menu, .. } => Some(menu),
            Estado::Fechado => None,
        };
        for (i, (r, t)) in self.titulos(f).iter().zip(&self.menu.titulos).enumerate() {
            let ativo = realce == Some(i);
            let (frente, fundo) = if ativo { (c.cores.texto_realce, c.cores.realce) } else { (c.cores.texto, c.cores.fundo) };
            if ativo {
                q.preencher(*r, fundo, fundo);
            }
            texto_a_esquerda(q, f, t, r.esq + c.margem_titulo, r.topo, frente, fundo);
        }
        if let Estado::Aberto { menu, item } = self.estado {
            self.desenhar_menu(q, f, e, menu, item);
        }
    }

    fn desenhar_menu(&self, q: &mut Quadro, f: &Fonte, e: &EstadoMenus, m: usize, realcado: Option<usize>) {
        let c = &self.cfg;
        let (r, coluna, itens) = self.geometria_menu(m, f, e);
        q.preencher(r, c.cores.fundo, c.cores.fundo);
        q.contorno(Retangulo { dir: r.dir - 1, base: r.base - 1, ..r }, c.cores.borda);
        for (i, &(t, h)) in itens.iter().enumerate() {
            let topo = r.topo + t;
            if self.separador(m, i) {
                let y = topo + c.linha_separador;
                q.preencher(Retangulo { esq: r.esq, topo: y, dir: r.dir, base: y + 1 }, c.cores.borda, c.cores.borda);
                continue;
            }
            let ativo = realcado == Some(i);
            let (frente, fundo) = if ativo { (c.cores.texto_realce, c.cores.realce) } else { (c.cores.texto, c.cores.fundo) };
            let faixa = Retangulo { esq: r.esq + 1, topo, dir: r.dir - 1, base: topo + h };
            if ativo {
                q.preencher(faixa, fundo, fundo);
            }
            let antes = q.recorte.replace(faixa);
            let texto = self.texto_item(m, i, e);
            let (a, b) = texto.split_once('\t').unwrap_or((&texto, ""));
            texto_a_esquerda(q, f, a, r.esq + c.margem_texto, topo, frente, fundo);
            if !b.is_empty() {
                texto_a_esquerda(q, f, b, r.esq + coluna, topo, frente, fundo);
            }
            if self.marcado(m, i, e) {
                for (dy, linha) in c.marca.iter().enumerate() {
                    for (dx, ch) in linha.chars().enumerate() {
                        if ch == '#' {
                            q.ponto(i32::from(r.esq + c.marca_x) + dx as i32, i32::from(topo + c.marca_y) + dy as i32, frente);
                        }
                    }
                }
            }
            q.recorte = antes;
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn barra() -> BarraMenus {
        let cfg: ConfigBarraMenus = serde_json::from_str(include_str!("../../../data/barra_menus.json")).unwrap();
        let menu = MenuBanco {
            titulos: vec!["&File".into(), "&Speed".into()],
            itens: vec![
                vec!["&About".into(), SEPARADOR.into(), "&New\tCtrl+N".into(), "&Quit\tCtrl+Q".into()],
                vec!["&Pause   \tShift+0".into(), SEPARADOR.into(), "&Slow\tShift+1".into(), "&Normal\tShift+2".into()],
            ],
        };
        BarraMenus::new(cfg, menu)
    }

    #[test]
    fn comandos_e_marcas() {
        let b = barra();
        assert_eq!(b.cfg.comando(0xFD08), Some(ComandoMenu::Sair));
        assert_eq!(b.cfg.comando(0xFD17), Some(ComandoMenu::Status));
        assert_eq!(b.cfg.comando(0xFD02), None, "separador");
        let e = EstadoMenus { velocidade: 1, pausado: true, ..Default::default() };
        // No menu de teste, o Speed é o menu 1: Pause = 0xFD11 não é o Pause; usa o texto do banco.
        assert_eq!(b.texto_item(1, 0, &e), "&Pause   \tShift+0");
        assert_eq!(ComandoMenu::Silly.opcao(), Some(5));
        assert_eq!(ComandoMenu::Ultra.velocidade(), Some(3));
    }

    #[test]
    fn teclado_alt_setas_letra_e_esc() {
        let mut b = barra();
        assert_eq!(b.tecla("X", false), Resposta::Ignorada);
        b.alternar();
        assert_eq!(b.estado, Estado::Barra(0));
        b.tecla("Right", false);
        assert_eq!(b.estado, Estado::Barra(1));
        b.tecla("Down", false);
        assert_eq!(b.estado, Estado::Aberto { menu: 1, item: Some(0) });
        b.tecla("Down", false);
        assert_eq!(b.estado, Estado::Aberto { menu: 1, item: Some(2) }, "pula o separador");
        b.tecla("Left", false);
        assert_eq!(b.estado, Estado::Aberto { menu: 0, item: Some(0) });
        b.tecla("Escape", false);
        assert_eq!(b.estado, Estado::Barra(0));
        b.tecla("Escape", false);
        assert_eq!(b.estado, Estado::Fechado);
        // Alt+F e Q: o Quit.
        b.tecla("F", true);
        assert_eq!(b.tecla("Q", false), Resposta::Comando(ComandoMenu::Sair));
        assert_eq!(b.estado, Estado::Fechado);
    }
}
