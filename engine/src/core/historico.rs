//! Janela History (janela 21, `_OpenHistoryWindow`): até 4 gráficos das séries do histórico
//! (`_drawHistGraph`), ligados e desligados pelos 10 botões (`_ToggleHistButton`).
//!
//! Especificação: docs/spec/telas/janelas.md, "History". Dados: bloco `historico` de
//! data/janelas.json.

use serde::Deserialize;

use super::janelas::GeometriaControle;
use super::quadro::Quadro;
use super::telas::Interface;
use crate::formats::janela::{Janela, Retangulo};
use crate::recursos::Recursos;
use crate::rules::estado::Estado;

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigHistorico {
    pub janela: u16,
    pub titulo: String,
    pub objeto_grafico: String,
    /// Botões na ordem do id (0x1503…0x150c): o gráfico do botão b é a série
    /// `(b & 1) × 5 + b / 2` (`_drawHistGraph`).
    pub botoes: Vec<String>,
    /// Séries (`_HistGraphs`, `8:8672`) e a parceira que entra na escala (`_AltHistGraphs`), por
    /// símbolo do save.
    pub series: Vec<String>,
    pub parceiras: Vec<String>,
    /// Lista SHARED 4 com o nome de cada série (`8:72e8` ← 4/1050).
    pub lista_nomes: u16,
    /// Cor (índice da paleta) de cada um dos 4 lugares de gráfico.
    pub cores: [u8; 4],
    pub cor_fundo: u8,
    pub fonte: usize,
    /// Deslocamento (x, y) das linhas e da caixa do nome em relação ao retângulo do objeto (medido na
    /// bancada, telas/janelas.md §3: (−1, −1)).
    #[serde(default)]
    pub ajuste_grafico: [i16; 2],
}

/// Estado da janela (fora do save, como no original: `8:8e54`, `8e10`, `8e5c`, `8e38`).
#[derive(Clone, Debug)]
pub struct JanelaHistorico {
    pub cfg: ConfigHistorico,
    /// Gráficos ligados, do mais novo ao mais velho (`8:8e54`): índices de botão.
    pub ativos: [Option<usize>; 4],
    /// Lugar (cor) de cada botão (`8:8e10`).
    lugar: [usize; 10],
    /// Lugares livres (`8:8e5c`, bits 0–3).
    livres: u8,
}

fn contem(r: Retangulo, x: i32, y: i32) -> bool {
    x >= i32::from(r.esq) && x < i32::from(r.dir) && y >= i32::from(r.topo) && y < i32::from(r.base)
}

/// Série do histórico pelo símbolo do save.
fn serie<'e>(e: &'e Estado, simbolo: &str) -> Option<&'e [i16; 64]> {
    let h = &e.historico;
    Some(match simbolo {
        "H_BPop" => &h.populacao_b,
        "H_RPop" => &h.populacao_r,
        "H_BFood" => &h.comida_b,
        "H_RFood" => &h.comida_r,
        "H_BHeal" => &h.saude_b,
        "H_RHeal" => &h.saude_r,
        "H_FoodA" => &h.comida_superficie,
        "H_ADead" => &h.mortes_naturais,
        "H_PDead" => &h.mortes_por_predadores,
        "H_FDead" => &h.mortes_em_luta,
        _ => return None,
    })
}

impl JanelaHistorico {
    pub fn new(cfg: ConfigHistorico) -> JanelaHistorico {
        JanelaHistorico { cfg, ativos: [None; 4], lugar: [0; 10], livres: 0x0F }
    }

    /// `_ToggleHistButton(b)`: liga (entra na frente da lista, empurrando o mais velho para fora,
    /// com o primeiro lugar livre) ou desliga o gráfico do botão.
    pub fn alternar(&mut self, b: usize) {
        if b >= 10 {
            return;
        }
        if let Some(i) = self.ativos.iter().position(|a| *a == Some(b)) {
            for k in i..3 {
                self.ativos[k] = self.ativos[k + 1];
            }
            self.ativos[3] = None;
            self.livres |= 1 << self.lugar[b];
            return;
        }
        if let Some(velho) = self.ativos[3] {
            self.livres |= 1 << self.lugar[velho];
        }
        self.ativos.copy_within(0..3, 1);
        self.ativos[0] = Some(b);
        if let Some(k) = (0..4).find(|k| self.livres & (1 << k) != 0) {
            self.livres &= !(1 << k);
            self.lugar[b] = k;
        }
    }

    pub fn ligado(&self, b: usize) -> bool {
        self.ativos.contains(&Some(b))
    }

    /// Clique na janela: botão de série alterna.
    pub fn clicar(&mut self, j: &Janela, g: &GeometriaControle, x: i32, y: i32) {
        for (b, nome) in self.cfg.botoes.clone().iter().enumerate() {
            if let Some(o) = j.objetos.iter().find(|o| &o.nome == nome) {
                let r = o.retangulo;
                let r = Retangulo { esq: r.esq + g.dx, topo: r.topo + g.dy, dir: r.dir + g.dx, base: r.base + g.dy };
                if contem(r, x, y) {
                    self.alternar(b);
                    return;
                }
            }
        }
    }

    /// Marca nos botões o estado ligado.
    pub fn marcar_botoes(&self, j: &mut Janela) {
        for (b, nome) in self.cfg.botoes.iter().enumerate() {
            if let Some(o) = j.objetos.iter_mut().find(|o| &o.nome == nome) {
                if self.ligado(b) {
                    o.flags |= 4;
                } else {
                    o.flags &= !4;
                }
            }
        }
    }

    /// `_win_DrawHistoryWindow`: área do gráfico em branco e os gráficos ligados.
    pub fn desenhar_graficos(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface, j: &Janela, g: &GeometriaControle, e: &Estado) {
        let Some(o) = j.objetos.iter().find(|o| o.nome == self.cfg.objeto_grafico) else { return };
        let r = o.retangulo;
        // _win_GetObjRect com faixa soma 1 à direita e embaixo; medido (j04): o preenchimento não
        // passa da base do objeto.
        let area = Retangulo { esq: r.esq + g.dx, topo: r.topo + g.dy, dir: r.dir + g.dx + 1, base: r.base + g.dy };
        q.preencher(area, self.cfg.cor_fundo, self.cfg.cor_fundo);
        let nomes = super::avisos::linhas_do_texto(recursos, self.cfg.lista_nomes);
        let fonte = interface.fontes.get(self.cfg.fonte).and_then(|f| f.as_ref());
        let antes = q.recorte.replace(area);
        let [ax, ay] = self.cfg.ajuste_grafico;
        // As linhas usam o retângulo do objeto sem o pixel a mais da direita, deslocado (medido).
        let linhas = Retangulo { esq: area.esq + ax, topo: area.topo + ay, dir: area.dir - 1 + ax, base: area.base + ay };
        for (i, b) in self.ativos.iter().enumerate() {
            let Some(b) = *b else { continue };
            let gi = (b & 1) * 5 + b / 2;
            let (Some(s), Some(p)) = (
                self.cfg.series.get(gi).and_then(|n| serie(e, n)),
                self.cfg.parceiras.get(gi).and_then(|n| serie(e, n)),
            ) else {
                continue;
            };
            let cor = self.cfg.cores[self.lugar[b]];
            let nome = nomes.get(gi).cloned().unwrap_or_default();
            desenhar_grafico(q, linhas, s, p, &e.historico, i as i32, cor, self.cfg.cor_fundo, fonte, &nome);
        }
        q.recorte = antes;
    }
}

/// `_drawHistGraph`: 64 amostras do anel (a partir de `_HistStart`) em segmentos de reta, com a
/// escala pelo maior valor da série e da parceira nas `_HistCnt` amostras; o nome numa caixa no fim.
#[allow(clippy::too_many_arguments)]
fn desenhar_grafico(
    q: &mut Quadro,
    area: Retangulo,
    s: &[i16; 64],
    p: &[i16; 64],
    h: &crate::rules::estado::Historico,
    deslocamento: i32,
    cor: u8,
    fundo: u8,
    fonte: Option<&crate::formats::fonte::Fonte>,
    nome: &[u8],
) {
    let (l, r) = (i32::from(area.esq), i32::from(area.dir));
    let passo = (r - l) - 64;
    let topo = i32::from(area.topo) + 8;
    let base = i32::from(area.base) - 8;
    let alt = base - topo;
    let inicio = i32::from(h.inicio);
    let n = i32::from(h.contagem);
    let mut maior = 0i32;
    let mut k = (inicio - n) & 0x3F;
    for _ in 0..n.max(0) {
        maior = maior.max(i32::from(s[k as usize])).max(i32::from(p[k as usize]));
        k = (k + 1) & 0x3F;
    }
    // Escala: multiplica por `mult` enquanto cabe, ou divide por `div` se nem 1 vez cabe.
    let (mut mult, mut div) = (1, 1);
    if maior > 0 {
        let mut v = maior;
        while v < alt {
            mult += 1;
            v += maior;
        }
        if mult < 2 {
            let mut v = maior;
            while alt < v {
                div += 1;
                v = maior / div;
            }
        } else {
            mult -= 1;
        }
    }
    let y = |v: i16| base - if div == 1 { i32::from(v) * mult } else { i32::from(v) / div };
    let mut idx = inicio & 0x3F;
    let mut ant = (l, y(s[(inicio as usize) & 0x3F]));
    let mut acum = passo;
    for _ in 0..63 {
        idx = (idx + 1) & 0x3F;
        let x = (acum >> 6) + l + deslocamento;
        let novo = (x, y(s[idx as usize]));
        q.linha(ant.0, ant.1, novo.0, novo.1, cor);
        ant = novo;
        acum += passo;
    }
    if let Some(f) = fonte {
        let w = f.largura_texto(nome) as i32;
        let hf = f.altura() as i32;
        let (x, yt) = (ant.0, ant.1 - hf / 2);
        let r = Retangulo { esq: x as i16, topo: yt as i16, dir: (x + w) as i16, base: (yt + hf) as i16 };
        q.texto(r, f, nome, 0, cor, fundo);
        q.contorno(Retangulo { esq: r.esq - 1, topo: r.topo - 1, dir: r.dir, base: r.base }, cor);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cfg() -> ConfigHistorico {
        let t: serde_json::Value = serde_json::from_str(&crate::recursos::marcar_referencias(include_str!("../../../data/janelas.json"))).unwrap();
        serde_json::from_value(t["historico"].clone()).unwrap()
    }

    #[test]
    fn liga_quatro_e_empurra_o_mais_velho() {
        let mut h = JanelaHistorico::new(cfg());
        for b in [0, 1, 2, 3] {
            h.alternar(b);
        }
        assert_eq!(h.ativos, [Some(3), Some(2), Some(1), Some(0)]);
        assert_eq!(h.lugar[..4], [0, 1, 2, 3]);
        h.alternar(5);
        assert_eq!(h.ativos, [Some(5), Some(3), Some(2), Some(1)]);
        assert_eq!(h.lugar[5], 0, "o lugar do que saiu fica livre");
        h.alternar(2);
        assert_eq!(h.ativos, [Some(5), Some(3), Some(1), None]);
        assert!(!h.ligado(2));
    }
}
