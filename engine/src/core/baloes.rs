//! Desenho dos balões de fala (`_MakeBalloon`, `1008:59e4`): o do menino no quintal e os da opção
//! "Silly" na janela de edição (`_DrawBalloons`, `1010:57aa`). Sem janela: desenha num `Quadro`.
//!
//! Especificação: docs/spec/telas/baloes.md (forma e posição) e telas/quintal.md §2.3 (menino).
//! As peças 8×8 do DGROUP (`1048:1c6e`…) estão em data/quintal_mapa.json (`balao_menino.pecas`).

use std::collections::HashMap;

use super::quadro::Quadro;
use crate::formats::fonte::Fonte;

/// Medidas de um balão para um texto (linhas separadas pelos dois caracteres '\' 'n').
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormaBalao<'t> {
    pub linhas: Vec<&'t [u8]>,
    /// Colunas de 8 pixels: (maior linha + 23) / 8.
    pub colunas: i32,
    pub altura_fonte: i32,
    /// Linha das peças de baixo: `altura_fonte × n + 6`.
    pub y_baixo: i32,
    /// Altura registrada da imagem: `altura_fonte × n + 15`, uma a menos com uma linha.
    pub altura: i32,
}

impl<'t> FormaBalao<'t> {
    pub fn medir(texto: &'t [u8], f: &Fonte) -> FormaBalao<'t> {
        let mut linhas: Vec<&[u8]> = Vec::new();
        let mut resto: &[u8] = texto;
        while let Some(p) = resto.windows(2).position(|w| w == b"\\n") {
            linhas.push(&resto[..p]);
            resto = &resto[p + 2..];
        }
        linhas.push(resto);
        let largura_texto = linhas.iter().map(|l| f.largura_texto(l) as i32).max().unwrap_or(0);
        let colunas = ((largura_texto + 23) >> 3).max(3);
        let altura_fonte = f.altura() as i32;
        let n = linhas.len() as i32;
        FormaBalao { colunas, altura_fonte, y_baixo: altura_fonte * n + 6, altura: altura_fonte * n + 15 - i32::from(n <= 1), linhas }
    }

    /// Largura em pixels.
    pub fn largura(&self) -> i32 {
        self.colunas * 8
    }
}

/// Peças e cores do balão.
#[derive(Clone, Copy, Debug)]
pub struct PecasBalao<'a> {
    /// Peças 8×8 por nome: 8 linhas de 8 caracteres ('.' transparente, '#' tinta, 'o' fundo).
    pub pecas: &'a HashMap<String, Vec<String>>,
    pub cor_tinta: u8,
    pub cor_fundo: u8,
}

/// `_MakeBalloon` com a cunha da esquerda, desenhado com o canto em (x0, y0): bordas, centro, peças
/// de baixo e cunha; cada linha do texto em `altura_fonte × i + 6`, centrada [hipótese], com a tinta.
/// O recorte do quadro limita o que aparece.
pub fn desenhar(q: &mut Quadro, forma: &FormaBalao, f: &Fonte, p: PecasBalao, x0: i32, y0: i32) {
    let colunas = forma.colunas;
    let peca = |q: &mut Quadro, nome: &str, coluna: i32, py: i32| {
        let Some(linhas) = p.pecas.get(nome) else { return };
        for (j, l) in linhas.iter().enumerate() {
            for (i, c) in l.bytes().enumerate() {
                let cor = match c {
                    b'#' => p.cor_tinta,
                    b'o' => p.cor_fundo,
                    _ => continue,
                };
                q.ponto(x0 + coluna * 8 + i as i32, y0 + py + j as i32, cor);
            }
        }
    };
    peca(q, "canto_sup_esq", 0, 0);
    for c in 1..colunas - 1 {
        peca(q, "borda_cima", c, 0);
    }
    peca(q, "canto_sup_dir", colunas - 1, 0);
    let mut y = 8;
    while y < forma.y_baixo {
        peca(q, "borda_esq", 0, y);
        for c in 1..colunas - 1 {
            peca(q, "centro", c, y);
        }
        peca(q, "borda_dir", colunas - 1, y);
        y += 8;
    }
    peca(q, "canto_inf_esq", 0, forma.y_baixo);
    for c in 1..colunas - 1 {
        peca(q, "borda_baixo", c, forma.y_baixo);
    }
    peca(q, "canto_inf_dir", colunas - 1, forma.y_baixo);
    peca(q, "cunha_esq", 1, forma.y_baixo);
    for (i, l) in forma.linhas.iter().enumerate() {
        let img = f.imagem(l);
        let tx = x0 + (forma.largura() - img.largura as i32) / 2;
        let ty = y0 + forma.altura_fonte * i as i32 + 6;
        for j in 0..img.altura {
            for k in 0..img.largura {
                if img.bits[j * img.largura + k] {
                    q.ponto(tx + k as i32, ty + j as i32, p.cor_tinta);
                }
            }
        }
    }
}
