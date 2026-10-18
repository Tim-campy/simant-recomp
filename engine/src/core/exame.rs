//! Examine (`_MagnifyMenu` `1010:6b4c`, `_win_DrawExamineWindow` `1010:69c0`): a janela 29
//! (`examineWin`) aberta enquanto o botão fica apertado numa célula com formiga, com o bitmap de
//! fundo e três linhas de texto (estágio ou casta, modo, carga). Especificação:
//! docs/spec/telas/janelas.md §4.3; dados: data/exame.json.

use serde::Deserialize;

use super::quadro::Quadro;
use super::telas::{carregar_janela, desenhar_objetos, Interface};
use crate::formats::janela::{Janela, Retangulo};
use crate::recursos::Recursos;
use crate::rules::experimental::Exame;

pub const ARQUIVO_CONFIG: &str = "exame.json";

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigExame {
    pub janela: u16,
    pub fonte: usize,
    /// Cor de cada linha: na bancada, a 1ª (casta/estágio) em preto e as outras duas em azul (12).
    pub cores_texto: [u8; 3],
    /// Cor da moldura (na bancada, azul-escuro, índice 4; as janelas de controle usam outra).
    pub cor_borda: u8,
    /// Objetos das três linhas (`examStr1..3`).
    pub objetos: [String; 3],
    /// Listas SHARED 4: estágio do ovo, casta (grupo), modo, carga.
    pub estagios: u16,
    pub grupos: u16,
    pub modos: u16,
    pub cargas: u16,
    /// Lado do tile da janela de edição (`_tileWidth`/`_tileHeight`).
    pub lado_tile: i32,
    /// Medido na bancada (2026-10-18): deslocamento do canto calculado até o contorno preto da janela,
    /// largura da moldura azul e deslocamento do conteúdo (bitmap) dentro dela.
    #[serde(default)]
    pub ajuste_canto: [i32; 2],
    #[serde(default)]
    pub largura_borda: Option<i16>,
    #[serde(default)]
    pub ajuste_conteudo: [i16; 2],
    /// Deslocamento (x, y) de cada linha em relação ao topo do objeto [medido em examine_0..2].
    #[serde(default)]
    pub deslocamento_linhas: [[i32; 2]; 3],
    /// Cor da área do objeto 0 (a linha branca em volta do bitmap).
    #[serde(default = "branco")]
    pub cor_fundo: u8,
}

fn branco() -> u8 {
    15
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
        (x + self.cfg.ajuste_canto[0], y + self.cfg.ajuste_canto[1])
    }

    /// Textos das três linhas: estágio (ovo) ou casta; modo; carga.
    pub fn textos(&self, ex: &Exame) -> [Vec<u8>; 3] {
        let pega = |l: usize, i: u8| self.listas[l].get(usize::from(i)).cloned().unwrap_or_default();
        let primeira = if ex.grupo == 0 { pega(0, ex.estagio) } else { pega(1, ex.grupo) };
        [primeira, pega(2, ex.modo), pega(3, ex.grupo)]
    }

    /// Desenha a janela com o canto em `(x, y)` da área cliente: contorno preto e moldura azul
    /// (`largura_borda` pixels, contorno incluído), a área do objeto 0 em branco (a linha branca
    /// em volta do bitmap) e os objetos [medido na bancada: examine_0..2].
    pub fn desenhar(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface, ex: &Exame, (x, y): (i32, i32)) {
        let m = &interface.moldura;
        let ob0 = &self.janela.objetos[0];
        let (w, h) = self.tamanho();
        let (w, h) = (w as i16, h as i16);
        let b = self.cfg.largura_borda.unwrap_or(m.borda.largura);
        // O canto já vem com `ajuste_canto` (de `canto`).
        let (ex0, ey0) = (x as i16, y as i16);
        let externo = Retangulo { esq: ex0, topo: ey0, dir: ex0 + w + 2 * b, base: ey0 + h + 2 * b };
        let interno = Retangulo { esq: ex0 + b, topo: ey0 + b, dir: ex0 + b + w, base: ey0 + b + h };
        q.preencher(externo, self.cfg.cor_borda, self.cfg.cor_borda);
        q.contorno(Retangulo { dir: externo.dir - 1, base: externo.base - 1, ..externo }, m.borda.cor_externa);
        q.preencher(interno, self.cfg.cor_fundo, self.cfg.cor_fundo);
        let origem = (interno.esq + self.cfg.ajuste_conteudo[0], interno.topo + self.cfg.ajuste_conteudo[1]);
        let (o0x, o0y) = (ob0.retangulo.esq, ob0.retangulo.topo);
        desenhar_objetos(q, recursos, interface, &self.janela, origem, Some(interno));
        let Some(f) = interface.fontes.get(self.cfg.fonte).and_then(|f| f.as_ref()) else { return };
        let antes = q.recorte.replace(interno);
        for (((nome, texto), cor), [dx, dy]) in
            self.cfg.objetos.iter().zip(self.textos(ex)).zip(self.cfg.cores_texto).zip(self.cfg.deslocamento_linhas)
        {
            let Some(o) = self.janela.objetos.iter().find(|o| &o.nome == nome) else { continue };
            let r = o.retangulo;
            // Centrado na largura do objeto, a partir do topo dele [hipótese: sem captura].
            let img = f.imagem(&texto);
            let largura = i32::from(r.dir - r.esq);
            let tx = i32::from(origem.0 + r.esq - o0x) + (largura - img.largura as i32) / 2 + dx;
            let ty = i32::from(origem.1 + r.topo - o0y) + dy;
            for j in 0..img.altura {
                for i in 0..img.largura {
                    if img.bits[j * img.largura + i] {
                        q.ponto(tx + i as i32, ty + j as i32, cor);
                    }
                }
            }
        }
        q.recorte = antes;
    }
}
