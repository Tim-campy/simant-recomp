//! Menu e painel de texto dos scripts de mods (API 0.17, docs/api-mods.md, "Menu e painel").
//! Extensão do port: o original não tem nada parecido. Geometria e cores em data/menus_mods.json.
//!
//! O menu abre pela tecla que o mod escolheu (`definir_menu`), fica por cima de tudo e recebe toda
//! a entrada até ser fechado (a simulação fica parada, como com um menu do Windows aberto); o item
//! escolhido vai para o gancho `ao_menu` do mod. O painel (`definir_painel`) é só texto num canto.

use serde::Deserialize;

use super::quadro::Quadro;
use crate::formats::fonte::Fonte;
use crate::formats::janela::Retangulo;
use crate::scripts::MenuMod;

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_CONFIG: &str = "menus_mods.json";

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigMenu {
    pub fontes: Vec<usize>,
    pub fontes_titulo: Vec<usize>,
    pub topo: i16,
    pub altura_titulo: i16,
    pub altura_item: i16,
    pub altura_separador: i16,
    pub margem_x: i16,
    pub largura_minima: i16,
    pub cor_fundo: u8,
    pub cor_texto: u8,
    pub cor_borda: u8,
    pub cor_realce: u8,
    pub cor_texto_realce: u8,
    pub cor_separador: u8,
    pub cor_titulo: u8,
    pub cor_texto_titulo: u8,
    pub cor_dica: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Canto {
    CimaDireita,
    CimaEsquerda,
    BaixoDireita,
    BaixoEsquerda,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigPainel {
    pub fontes: Vec<usize>,
    pub canto: Canto,
    pub margem: i16,
    pub folga: i16,
    pub altura_linha: i16,
    pub cor_fundo: u8,
    pub cor_texto: u8,
    pub cor_titulo: u8,
    pub cor_borda: u8,
}

/// Conteúdo de `data/menus_mods.json`.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigMenusMods {
    pub menu: ConfigMenu,
    pub painel: ConfigPainel,
}

impl ConfigMenusMods {
    pub fn de_json(texto: &str) -> Result<ConfigMenusMods, String> {
        serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_CONFIG}: {e}"))
    }
}

/// Resposta a uma tecla com o menu aberto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RespostaMenuMod {
    /// O menu continua aberto (mudou o realce ou nada).
    Nada,
    /// Fechar sem escolher.
    Fechar,
    /// Item escolhido (índice a partir de 1, como o Lua vê).
    Escolher(usize),
}

/// Um menu de mod aberto na tela.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuModAberto {
    /// Nome do mod dono do menu.
    pub dono: String,
    pub menu: MenuMod,
    pub externo: Retangulo,
    pub titulo: Retangulo,
    /// Retângulo de cada item (os separadores também, para o desenho).
    pub itens: Vec<Retangulo>,
    pub realcado: Option<usize>,
}

fn e_separador(texto: &str) -> bool {
    texto.trim() == "-"
}

fn contem(r: Retangulo, x: i32, y: i32) -> bool {
    x >= i32::from(r.esq) && x < i32::from(r.dir) && y >= i32::from(r.topo) && y < i32::from(r.base)
}

fn fonte<'i>(lista: &[usize], fontes: &'i [Option<Fonte>]) -> Option<&'i Fonte> {
    lista.iter().find_map(|&i| fontes.get(i).and_then(|f| f.as_ref()))
}

/// Texto do Lua (UTF-8) nos bytes da fonte do Windows (Latin-1; o resto vira '?').
fn bytes(texto: &str) -> Vec<u8> {
    texto.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect()
}

fn largura(f: Option<&Fonte>, texto: &str) -> i32 {
    f.map_or(texto.chars().count() as i32 * 8, |f| f.largura_texto(&bytes(texto)) as i32)
}

fn escrever(q: &mut Quadro, f: Option<&Fonte>, x: i32, y: i32, texto: &str, cor: u8) {
    let Some(f) = f else { return };
    let img = f.imagem(&bytes(texto));
    for j in 0..img.altura {
        for i in 0..img.largura {
            if img.bits[j * img.largura + i] {
                q.ponto(x + i as i32, y + j as i32, cor);
            }
        }
    }
}

fn altura_fonte(f: Option<&Fonte>) -> i32 {
    f.map_or(12, |f| f.altura() as i32)
}

impl MenuModAberto {
    /// Abre o menu centrado em x dentro de `area`, a `topo` pixels do alto dela.
    pub fn abrir(cfg: &ConfigMenusMods, fontes: &[Option<Fonte>], dono: &str, menu: &MenuMod, area: Retangulo) -> MenuModAberto {
        let c = &cfg.menu;
        let f = fonte(&c.fontes, fontes);
        let ft = fonte(&c.fontes_titulo, fontes);
        let titulo = format!("{} ({})", menu.titulo, menu.tecla.texto());
        let mut w = largura(ft, &titulo) + 2 * i32::from(c.margem_x);
        for item in &menu.itens {
            w = w.max(largura(f, item) + 2 * i32::from(c.margem_x));
        }
        let w = w.max(i32::from(c.largura_minima)) as i16;
        let altura_itens: i16 = menu.itens.iter().map(|i| if e_separador(i) { c.altura_separador } else { c.altura_item }).sum();
        let h = c.altura_titulo + altura_itens + 3;
        let esq = area.esq + ((area.dir - area.esq - w) / 2).max(0);
        let topo = (area.topo + c.topo).min((area.base - h).max(area.topo));
        let externo = Retangulo { esq, topo, dir: esq + w, base: topo + h };
        let titulo_r = Retangulo { esq: esq + 1, topo: topo + 1, dir: esq + w - 1, base: topo + 1 + c.altura_titulo };
        let mut y = titulo_r.base + 1;
        let itens = menu
            .itens
            .iter()
            .map(|i| {
                let a = if e_separador(i) { c.altura_separador } else { c.altura_item };
                let r = Retangulo { esq: esq + 1, topo: y, dir: esq + w - 1, base: y + a };
                y += a;
                r
            })
            .collect();
        MenuModAberto { dono: dono.to_string(), menu: menu.clone(), externo, titulo: titulo_r, itens, realcado: None }
    }

    /// Item (índice a partir de 0) sob o ponto, se não for separador.
    pub fn item_em(&self, x: i32, y: i32) -> Option<usize> {
        self.itens.iter().position(|r| contem(*r, x, y)).filter(|&i| !e_separador(&self.menu.itens[i]))
    }

    /// O cursor passou por (x, y): realça o item. Devolve se mudou.
    pub fn mover(&mut self, x: i32, y: i32) -> bool {
        let novo = self.item_em(x, y);
        let mudou = novo != self.realcado && novo.is_some();
        if novo.is_some() {
            self.realcado = novo;
        }
        mudou
    }

    /// Clique em (x, y): dentro de um item escolhe; fora do menu fecha.
    pub fn clicar(&self, x: i32, y: i32) -> RespostaMenuMod {
        match self.item_em(x, y) {
            Some(i) => RespostaMenuMod::Escolher(i + 1),
            None if contem(self.externo, x, y) => RespostaMenuMod::Nada,
            None => RespostaMenuMod::Fechar,
        }
    }

    /// Setas sobem e descem (pulando separadores, dando a volta), Enter escolhe, Esc fecha.
    pub fn tecla(&mut self, nome: &str) -> RespostaMenuMod {
        let n = self.menu.itens.len();
        let passo = match nome {
            "Escape" => return RespostaMenuMod::Fechar,
            "Return" | "Keypad Enter" => {
                return self.realcado.map_or(RespostaMenuMod::Nada, |i| RespostaMenuMod::Escolher(i + 1));
            }
            "Down" => 1,
            "Up" => n - 1,
            _ => return RespostaMenuMod::Nada,
        };
        let mut i = self.realcado.unwrap_or(if passo == 1 { n - 1 } else { 0 });
        for _ in 0..n {
            i = (i + passo) % n;
            if !e_separador(&self.menu.itens[i]) {
                self.realcado = Some(i);
                break;
            }
        }
        RespostaMenuMod::Nada
    }

    pub fn desenhar(&self, q: &mut Quadro, fontes: &[Option<Fonte>], cfg: &ConfigMenusMods) {
        let c = &cfg.menu;
        let f = fonte(&c.fontes, fontes);
        let ft = fonte(&c.fontes_titulo, fontes);
        let e = self.externo;
        q.preencher(e, c.cor_fundo, c.cor_fundo);
        q.contorno(Retangulo { dir: e.dir - 1, base: e.base - 1, ..e }, c.cor_borda);
        let t = self.titulo;
        q.preencher(t, c.cor_titulo, c.cor_titulo);
        q.linha(i32::from(e.esq), i32::from(t.base), i32::from(e.dir) - 1, i32::from(t.base), c.cor_borda);
        let titulo = format!("{} ({})", self.menu.titulo, self.menu.tecla.texto());
        let yt = i32::from(t.topo) + (i32::from(t.base - t.topo) - altura_fonte(ft)) / 2;
        escrever(q, ft, i32::from(t.esq) + i32::from(c.margem_x) - 1, yt, &titulo, c.cor_texto_titulo);
        for (i, (r, texto)) in self.itens.iter().zip(&self.menu.itens).enumerate() {
            if e_separador(texto) {
                let y = i32::from(r.topo + r.base) / 2;
                q.linha(i32::from(r.esq) + 1, y, i32::from(r.dir) - 2, y, c.cor_separador);
                continue;
            }
            let realce = self.realcado == Some(i);
            if realce {
                q.preencher(*r, c.cor_realce, c.cor_realce);
            }
            let y = i32::from(r.topo) + (i32::from(r.base - r.topo) - altura_fonte(f)) / 2;
            escrever(q, f, i32::from(r.esq) + i32::from(c.margem_x) - 1, y, texto, if realce { c.cor_texto_realce } else { c.cor_texto });
        }
    }
}

/// Os painéis dos mods, um embaixo do outro, no canto de `area`. Cada um: nome do mod e as linhas.
pub fn desenhar_paineis(q: &mut Quadro, fontes: &[Option<Fonte>], cfg: &ConfigMenusMods, paineis: &[(String, Vec<String>)], area: Retangulo) {
    let c = &cfg.painel;
    let f = fonte(&c.fontes, fontes);
    let lh = i32::from(c.altura_linha);
    let folga = i32::from(c.folga);
    let mut deslocamento = 0;
    for (nome, linhas) in paineis {
        let cabecalho = format!("[{nome}]");
        let w = std::iter::once(&cabecalho).chain(linhas).map(|l| largura(f, l)).max().unwrap_or(0) + 2 * folga;
        let h = (linhas.len() as i32 + 1) * lh + 2 * folga;
        let m = i32::from(c.margem);
        let x = match c.canto {
            Canto::CimaDireita | Canto::BaixoDireita => i32::from(area.dir) - m - w,
            _ => i32::from(area.esq) + m,
        };
        let y = match c.canto {
            Canto::CimaDireita | Canto::CimaEsquerda => i32::from(area.topo) + m + deslocamento,
            _ => i32::from(area.base) - m - h - deslocamento,
        };
        let r = Retangulo { esq: x as i16, topo: y as i16, dir: (x + w) as i16, base: (y + h) as i16 };
        q.preencher(r, c.cor_fundo, c.cor_fundo);
        q.contorno(Retangulo { dir: r.dir - 1, base: r.base - 1, ..r }, c.cor_borda);
        escrever(q, f, x + folga, y + folga, &cabecalho, c.cor_titulo);
        for (k, l) in linhas.iter().enumerate() {
            escrever(q, f, x + folga, y + folga + (k as i32 + 1) * lh, l, c.cor_texto);
        }
        deslocamento += h + m;
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::scripts::TeclaMod;

    fn menu() -> MenuMod {
        MenuMod { titulo: "T".into(), tecla: TeclaMod::de_texto("F12").unwrap(), itens: vec!["a".into(), "-".into(), "b".into()] }
    }

    fn aberto() -> MenuModAberto {
        let cfg = ConfigMenusMods::de_json(include_str!("../../../data/menus_mods.json")).unwrap();
        let fontes: Vec<Option<Fonte>> = Vec::new();
        MenuModAberto::abrir(&cfg, &fontes, "m", &menu(), Retangulo { esq: 0, topo: 40, dir: 640, base: 442 })
    }

    #[test]
    fn setas_pulam_o_separador_e_enter_escolhe() {
        let mut m = aberto();
        assert_eq!(m.tecla("Return"), RespostaMenuMod::Nada, "nada realçado");
        m.tecla("Down");
        assert_eq!(m.realcado, Some(0));
        m.tecla("Down");
        assert_eq!(m.realcado, Some(2));
        assert_eq!(m.tecla("Return"), RespostaMenuMod::Escolher(3));
        m.tecla("Down");
        assert_eq!(m.realcado, Some(0), "dá a volta");
        assert_eq!(m.tecla("Escape"), RespostaMenuMod::Fechar);
    }

    #[test]
    fn clique_no_item_escolhe_e_fora_fecha() {
        let m = aberto();
        let r = m.itens[2];
        assert_eq!(m.clicar(i32::from(r.esq) + 2, i32::from(r.topo) + 2), RespostaMenuMod::Escolher(3));
        let s = m.itens[1];
        assert_eq!(m.clicar(i32::from(s.esq) + 2, i32::from(s.topo) + 2), RespostaMenuMod::Nada, "separador");
        assert_eq!(m.clicar(0, 441), RespostaMenuMod::Fechar);
    }
}
