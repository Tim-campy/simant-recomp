//! Área das barras da faixa (`mapRibCaste` 0x2227 da faixa 34, `yardRibCaste` 0x2324 da 35): o clique
//! na barra de saúde da amarela ou da colônia preta muda o limiar de aviso (`_DoHealthSetY`,
//! `_DoWarnSetB`); no resto da área, abre o quadro das castas (`_DrawCastePopUp` `1000:5d9e`, janela
//! 23 `antPopWin`) enquanto o botão fica apertado. Sem SDL.
//!
//! Especificação: docs/spec/telas/mensagens.md §9.1; dados: data/popup_castas.json.

use serde::Deserialize;

use super::quadro::Quadro;
use super::telas::{carregar_janela, desenhar_objetos, Interface};
use crate::formats::janela::{Cor, Janela, Retangulo};
use crate::recursos::Recursos;
use crate::rules::estado::Estado;

pub const ARQUIVO_CONFIG: &str = "popup_castas.json";

/// Uma faixa (34 ou 35): a área que recebe o clique e as duas barras de saúde.
#[derive(Clone, Debug, Deserialize)]
pub struct AreaFaixa {
    /// Objeto da área (`mapRibCaste` / `yardRibCaste`).
    pub area: String,
    /// Barra de saúde da amarela (`_DoHealthSetY`) e da colônia preta (`_DoWarnSetB`).
    pub saude_amarela: String,
    pub saude_preta: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigPopupCastas {
    pub faixa_mapa: AreaFaixa,
    pub faixa_quintal: AreaFaixa,
    /// Colunas [esq, dir) usadas na conta do limiar: o original usa os retângulos de `ribYellowH2` e
    /// `ribBlackH2` da faixa 35 nas duas faixas [código + bancada].
    pub escala_x: [i32; 2],
    /// Janela 23 e os objetos: área do gráfico, cor dos números pretos e dos vermelhos.
    pub janela: u16,
    pub objeto_grafico: String,
    pub objeto_preto: String,
    pub objeto_vermelho: String,
    /// `_font_SetFont(2)`.
    pub fonte: usize,
    /// Contorno externo: largura e altura, distância da borda direita da área cliente e topo.
    pub externo: [i16; 2],
    pub margem_direita: i16,
    pub topo: i16,
    /// Moldura: 1 linha preta (`cor_contorno`) e o resto na `cor_borda`, `largura_borda` no total.
    pub largura_borda: i16,
    pub cor_borda: u8,
    pub cor_contorno: u8,
    /// Área do objeto 0 (a linha branca em volta do bitmap).
    pub cor_fundo: u8,
    /// Canto do objeto 0 em relação ao canto de dentro da moldura [medido: (−2, −2), o conteúdo
    /// começa embaixo da moldura].
    #[serde(default)]
    pub ajuste_conteudo: [i16; 2],
    /// Ajustes medidos na bancada: posição dos números e das barras em relação à conta do código.
    #[serde(default)]
    pub ajuste_numeros: [i32; 2],
    #[serde(default)]
    pub ajuste_barras: [i32; 2],
}

/// O que um clique na área das barras faz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliqueBarras {
    /// `_MeWarnHealth` = valor.
    AvisoAmarela(i16),
    /// `_BlkWarnHealth` = valor.
    AvisoColonia(i16),
    /// Abre o quadro das castas até soltar o botão.
    Castas,
}

/// `_DoHealthSetY` / `_DoWarnSetB` (barra horizontal): `(x − esq) × 100 / (dir − esq)`, com o C
/// truncando para zero.
pub fn limiar_do_clique(x: i32, escala: [i32; 2]) -> i16 {
    let [esq, dir] = escala;
    let largura = (dir - esq).max(1);
    ((x - esq) * 100 / largura) as i16
}

pub struct PopupCastas {
    pub cfg: ConfigPopupCastas,
    janela: Janela,
}

impl PopupCastas {
    pub fn carregar(recursos: &Recursos) -> Result<PopupCastas, String> {
        let cfg: ConfigPopupCastas = recursos.dados(ARQUIVO_CONFIG)?;
        let janela = carregar_janela(recursos, cfg.janela)?;
        Ok(PopupCastas { cfg, janela })
    }

    /// O clique em (x, y) cai na área das barras da faixa? `objeto(nome)` dá o retângulo do objeto
    /// da faixa na área cliente; `quintal` escolhe a faixa 35.
    pub fn clique(&self, x: i32, y: i32, quintal: bool, objeto: &dyn Fn(&str) -> Option<Retangulo>) -> Option<CliqueBarras> {
        let a = if quintal { &self.cfg.faixa_quintal } else { &self.cfg.faixa_mapa };
        let dentro = |nome: &str| objeto(nome).is_some_and(|r| x >= i32::from(r.esq) && x < i32::from(r.dir) && y >= i32::from(r.topo) && y < i32::from(r.base));
        if !dentro(&a.area) {
            return None;
        }
        Some(if dentro(&a.saude_amarela) {
            CliqueBarras::AvisoAmarela(limiar_do_clique(x, self.cfg.escala_x))
        } else if dentro(&a.saude_preta) {
            CliqueBarras::AvisoColonia(limiar_do_clique(x, self.cfg.escala_x))
        } else {
            CliqueBarras::Castas
        })
    }

    /// Contorno externo na área cliente de largura `largura_tela`.
    pub fn externo(&self, largura_tela: i32) -> Retangulo {
        let [w, h] = self.cfg.externo;
        let esq = largura_tela as i16 - self.cfg.margem_direita - w;
        Retangulo { esq, topo: self.cfg.topo, dir: esq + w, base: self.cfg.topo + h }
    }

    fn retangulo(&self, nome: &str) -> Option<Retangulo> {
        self.janela.objetos.iter().find(|o| o.nome == nome).map(|o| o.retangulo)
    }

    fn cor(&self, interface: &Interface, nome: &str) -> u8 {
        let c = self.janela.objetos.iter().find(|o| o.nome == nome).map_or(0, |o| o.cor);
        interface.cores.get(usize::from(c)).copied().unwrap_or(Cor { frente: 0, fundo: 15, fundo2: 15 }).frente
    }

    /// Desenha o quadro com os números `_CastePopB/R` de cada casta e as barras proporcionais ao
    /// maior número das castas 1..5 (`_DrawCastePopUp`).
    pub fn desenhar(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface, e: &Estado, largura_tela: i32) -> Vec<String> {
        let c = &self.cfg;
        let ext = self.externo(largura_tela);
        let b = c.largura_borda;
        let interno = Retangulo { esq: ext.esq + b, topo: ext.topo + b, dir: ext.dir - b, base: ext.base - b };
        q.preencher(ext, c.cor_borda, c.cor_borda);
        q.contorno(Retangulo { dir: ext.dir - 1, base: ext.base - 1, ..ext }, c.cor_contorno);
        q.preencher(interno, c.cor_fundo, c.cor_fundo);
        let ob0 = self.janela.objetos[0].retangulo;
        let origem = (interno.esq + c.ajuste_conteudo[0], interno.topo + c.ajuste_conteudo[1]);
        let mut pendentes = desenhar_objetos(q, recursos, interface, &self.janela, origem, Some(interno));
        let Some(g) = self.retangulo(&c.objeto_grafico) else {
            pendentes.push(c.objeto_grafico.clone());
            return pendentes;
        };
        let Some(f) = interface.fontes.get(c.fonte).and_then(|f| f.as_ref()) else {
            pendentes.push(format!("fonte {}", c.fonte));
            return pendentes;
        };
        // Área do gráfico na área cliente da janela (origem no objeto 0).
        let (ox, oy) = (i32::from(origem.0 - ob0.esq), i32::from(origem.1 - ob0.topo));
        // A direita e a base contam 1 a menos que o retângulo do layout [medido: com 160 as linhas 4 e
        // 5 ficavam 1 pixel abaixo da captura, e as barras 1 pixel mais compridas].
        let (esq, topo, dir, base) = (i32::from(g.esq), i32::from(g.topo), i32::from(g.dir) - 1, i32::from(g.base) - 1);
        let digito = (b'0'..=b'9').map(|d| f.largura_texto(&[d]) as i32).max().unwrap_or(0);
        let x_barra = digito * 4 + esq + 2;
        let comprimento = dir - x_barra;
        let fh = f.altura() as i32;
        let sobra = base - topo - fh * 12 + 3;
        let (resto, folga) = (sobra % 5, sobra / 5);
        let pretas = e.preta.castas;
        let vermelhas = e.vermelha.castas;
        let maior = (1..6).map(|i| i32::from(pretas[i]).max(i32::from(vermelhas[i]))).max().unwrap_or(1).max(1);
        let (cor_p, cor_v) = (self.cor(interface, &c.objeto_preto), self.cor(interface, &c.objeto_vermelho));
        let antes = q.recorte.replace(interno);
        let mut y = topo;
        let mut acumulado = 0;
        for i in 0..6 {
            acumulado += resto;
            if acumulado > 5 {
                y += 1;
                acumulado -= 5;
            }
            if i == 5 {
                y -= 1;
            }
            for (k, (valor, cor)) in [(pretas[i], cor_p), (vermelhas[i], cor_v)].into_iter().enumerate() {
                let yl = y + fh * k as i32;
                let [nx, ny] = c.ajuste_numeros;
                escrever(q, f, ox + esq + nx, oy + yl + ny, valor.to_string().as_bytes(), cor);
                let w = i32::from(valor) * comprimento / maior;
                if i != 0 && w != 0 {
                    let [bx, by] = c.ajuste_barras;
                    let r = Retangulo {
                        esq: (ox + x_barra + bx) as i16,
                        topo: (oy + yl + 1 + by) as i16,
                        dir: (ox + x_barra + w + bx + 1) as i16,
                        base: (oy + yl + fh - 3 + by + 1) as i16,
                    };
                    q.preencher(r, cor, cor);
                }
            }
            y += fh * 2 + folga;
        }
        q.recorte = antes;
        pendentes
    }
}

/// Texto transparente com a célula em (x, y) (`_font_PrintStr`).
fn escrever(q: &mut Quadro, f: &crate::formats::fonte::Fonte, x: i32, y: i32, texto: &[u8], cor: u8) {
    let img = f.imagem(texto);
    for j in 0..img.altura {
        for i in 0..img.largura {
            if img.bits[j * img.largura + i] {
                q.ponto(x + i as i32, y + j as i32, cor);
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Bancada (qemu_ui_barras_direito.txt): x = 580 na barra preta deu `_BlkWarnHealth` 20; x = 620
    /// na da amarela deu `_MeWarnHealth` 82; x = 590 deu 35.
    #[test]
    fn limiar_como_na_bancada() {
        let escala = [567, 631];
        assert_eq!(limiar_do_clique(580, escala), 20);
        assert_eq!(limiar_do_clique(620, escala), 82);
        assert_eq!(limiar_do_clique(590, escala), 35);
    }
}
