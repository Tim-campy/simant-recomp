//! Examine (`_MagnifyMenu` `1010:6b4c`, `_win_DrawExamineWindow` `1010:69c0`): a janela 29
//! (`examineWin`) aberta enquanto o botão fica apertado numa célula com formiga, com o bitmap de
//! fundo e três linhas de texto (estágio ou casta, modo, carga). Especificação:
//! docs/spec/telas/janelas.md §4.3; dados: data/exame.json.

use serde::Deserialize;

use super::quadro::Quadro;
use super::telas::{carregar_janela, desenhar_objetos, Interface};
use crate::formats::janela::{Janela, Retangulo, TIPO_CONTORNO};
use crate::recursos::Recursos;
use crate::rules::experimental::Exame;

pub const ARQUIVO_CONFIG: &str = "exame.json";

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigExame {
    pub janela: u16,
    pub fonte: usize,
    pub cor_texto: u8,
    /// Objetos das três linhas (`examStr1..3`).
    pub objetos: [String; 3],
    /// Listas SHARED 4: estágio do ovo, casta (grupo), modo, carga.
    pub estagios: u16,
    pub grupos: u16,
    pub modos: u16,
    pub cargas: u16,
    /// Lado do tile da janela de edição (`_tileWidth`/`_tileHeight`).
    pub lado_tile: i32,
}

/// Janela 29 carregada, com as listas de texto.
pub struct JanelaExame {
    pub cfg: ConfigExame,
    janela: Janela,
    listas: [Vec<Vec<u8>>; 4],
}

impl JanelaExame {
    pub fn carregar(recursos: &Recursos) -> Result<JanelaExame, String> {
        let cfg: ConfigExame = recursos.dados(ARQUIVO_CONFIG)?;
        let janela = carregar_janela(recursos, cfg.janela)?;
        let l = |id| super::avisos::linhas_do_texto(recursos, id);
        let listas = [l(cfg.estagios), l(cfg.grupos), l(cfg.modos), l(cfg.cargas)];
        Ok(JanelaExame { cfg, janela, listas })
    }

    /// Tamanho (largura, altura) do objeto 0.
    fn tamanho(&self) -> (i32, i32) {
        let r = self.janela.objetos[0].retangulo;
        (i32::from(r.dir - r.esq), i32::from(r.base - r.topo))
    }

    /// Canto da janela (`_MagnifyMenu`): uma célula à direita e uma acima da clicada; se passa da
    /// base da área cliente, sobe a própria altura (sem passar do topo); se passa da direita, vai a
    /// própria largura para a esquerda (sem passar da esquerda). `celula` é a posição da célula na
    /// vista (coluna, linha) e `area` o canto da área de tiles da janela de edição.
    pub fn canto(&self, celula: (i32, i32), area: (i32, i32), tela: (i32, i32)) -> (i32, i32) {
        let lado = self.cfg.lado_tile;
        let (w, h) = self.tamanho();
        let mut x = (celula.0 + 1) * lado + area.0;
        let mut y = (celula.1 - 1) * lado + area.1;
        if y + h > tela.1 {
            y = (y - h).max(0);
        }
        if x + w > tela.0 {
            x = (x - w).max(0);
        }
        (x, y)
    }

    /// Textos das três linhas: estágio (ovo) ou casta; modo; carga.
    pub fn textos(&self, ex: &Exame) -> [Vec<u8>; 3] {
        let pega = |l: usize, i: u8| self.listas[l].get(usize::from(i)).cloned().unwrap_or_default();
        let primeira = if ex.grupo == 0 { pega(0, ex.estagio) } else { pega(1, ex.grupo) };
        [primeira, pega(2, ex.modo), pega(3, ex.grupo)]
    }

    /// Desenha a janela com o canto em `(x, y)` da área cliente.
    pub fn desenhar(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface, ex: &Exame, (x, y): (i32, i32)) {
        let m = &interface.moldura;
        let ob0 = &self.janela.objetos[0];
        let t = if ob0.tipo == TIPO_CONTORNO { ob0.byte_28 as i16 } else { 0 };
        let (w, h) = self.tamanho();
        let (w, h) = (w as i16, h as i16);
        let (lw, lh) = if t > 0 { (w - 2 * t - 1, h - 2 * t - 1) } else { (w, h) };
        let b = m.borda.largura;
        let (ex0, ey0) = (x as i16, y as i16);
        let externo = Retangulo { esq: ex0, topo: ey0, dir: ex0 + lw + 2 * b, base: ey0 + lh + 2 * b };
        let interno = Retangulo { esq: ex0 + b, topo: ey0 + b, dir: ex0 + b + lw, base: ey0 + b + lh };
        q.preencher(externo, m.borda.cor, m.borda.cor);
        q.contorno(Retangulo { dir: externo.dir - 1, base: externo.base - 1, ..externo }, m.borda.cor_externa);
        let origem = (interno.esq - t + m.borda.ajuste_x_contorno * i16::from(t > 0), interno.topo - t);
        let (o0x, o0y) = (ob0.retangulo.esq, ob0.retangulo.topo);
        desenhar_objetos(q, recursos, interface, &self.janela, origem, Some(interno));
        let Some(f) = interface.fontes.get(self.cfg.fonte).and_then(|f| f.as_ref()) else { return };
        let antes = q.recorte.replace(interno);
        for (nome, texto) in self.cfg.objetos.iter().zip(self.textos(ex)) {
            let Some(o) = self.janela.objetos.iter().find(|o| &o.nome == nome) else { continue };
            let r = o.retangulo;
            // Centrado na largura do objeto, a partir do topo dele [hipótese: sem captura].
            let img = f.imagem(&texto);
            let largura = i32::from(r.dir - r.esq);
            let tx = i32::from(origem.0 + r.esq - o0x) + (largura - img.largura as i32) / 2;
            let ty = i32::from(origem.1 + r.topo - o0y);
            for j in 0..img.altura {
                for i in 0..img.largura {
                    if img.bits[j * img.largura + i] {
                        q.ponto(tx + i as i32, ty + j as i32, self.cfg.cor_texto);
                    }
                }
            }
        }
        q.recorte = antes;
    }
}
