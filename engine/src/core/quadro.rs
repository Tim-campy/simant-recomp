//! Quadro indexado (um índice de paleta por pixel) e primitivas de desenho usadas pelas janelas.
//! Sem dependência de janela do sistema: a camada de render só converte o quadro com a paleta.

use crate::formats::bitmap::Bitmap;
use crate::formats::fonte::Fonte;
use crate::formats::janela::{Conteudo, Cor, Janela, Retangulo};

/// Cores do relevo 3D [hipótese: branco e cinza escuro; `_GRectPartialMixedOutline` ainda não analisada].
const COR_RELEVO_CLARO: u8 = 15;
const COR_RELEVO_ESCURO: u8 = 8;

pub struct Quadro {
    pub largura: usize,
    pub altura: usize,
    pub pixels: Vec<u8>,
    /// Se definido, só pixels dentro deste retângulo [esq, dir) × [topo, base) são desenhados.
    pub recorte: Option<Retangulo>,
}

impl Quadro {
    pub fn new(largura: usize, altura: usize, fundo: u8) -> Quadro {
        Quadro { largura, altura, pixels: vec![fundo; largura * altura], recorte: None }
    }

    pub(crate) fn ponto(&mut self, x: i32, y: i32, cor: u8) {
        if let Some(r) = self.recorte {
            if x < r.esq as i32 || x >= r.dir as i32 || y < r.topo as i32 || y >= r.base as i32 {
                return;
            }
        }
        if x >= 0 && y >= 0 && (x as usize) < self.largura && (y as usize) < self.altura {
            self.pixels[y as usize * self.largura + x as usize] = cor;
        }
    }

    /// Preenche [esq, dir) × [topo, base). Com `cor2` diferente, usa padrão xadrez [hipótese, ver janela.md].
    pub fn preencher(&mut self, r: Retangulo, cor: u8, cor2: u8) {
        for y in r.topo as i32..r.base as i32 {
            for x in r.esq as i32..r.dir as i32 {
                let c = if (x + y) % 2 == 0 { cor } else { cor2 };
                self.ponto(x, y, c);
            }
        }
    }

    pub fn contorno(&mut self, r: Retangulo, cor: u8) {
        let (e, t, d, b) = (r.esq as i32, r.topo as i32, r.dir as i32, r.base as i32);
        for x in e..=d {
            self.ponto(x, t, cor);
            self.ponto(x, b, cor);
        }
        for y in t..=b {
            self.ponto(e, y, cor);
            self.ponto(d, y, cor);
        }
    }

    pub fn relevo(&mut self, r: Retangulo, espessura: u8, invertido: bool) {
        let (claro, escuro) = if invertido {
            (COR_RELEVO_ESCURO, COR_RELEVO_CLARO)
        } else {
            (COR_RELEVO_CLARO, COR_RELEVO_ESCURO)
        };
        for k in 0..espessura.max(1) as i32 {
            let (e, t, d, b) = (r.esq as i32 + k, r.topo as i32 + k, r.dir as i32 - 1 - k, r.base as i32 - 1 - k);
            for x in e..=d {
                self.ponto(x, t, claro);
                self.ponto(x, b, escuro);
            }
            for y in t..=b {
                self.ponto(e, y, claro);
                self.ponto(d, y, escuro);
            }
        }
    }

    /// Texto num retângulo, como `_gr_JustifyStrInRect`: centralizado (0/3), à esquerda (1) ou à
    /// direita (2), centralizado na vertical, desenhado opaco (tinta em `frente`, fundo em `fundo`).
    pub fn texto(&mut self, r: Retangulo, fonte: &Fonte, texto: &[u8], alinhamento: u16, frente: u8, fundo: u8) {
        let img = fonte.imagem(texto);
        let (l, t, d, b) = (r.esq as i32, r.topo as i32, r.dir as i32, r.base as i32);
        let w = img.largura as i32;
        let x = match alinhamento {
            1 => l + 4,
            2 => d - w - 4,
            _ => (l + d - w).div_euclid(2),
        }
        .max(l);
        let y = (t + b - fonte.altura() as i32).div_euclid(2);
        for j in 0..img.altura {
            for i in 0..img.largura {
                let c = if img.bits[j * img.largura + i] { frente } else { fundo };
                self.ponto(x + i as i32, y + j as i32, c);
            }
        }
        if alinhamento == 3 {
            let h = fonte.altura() as i16;
            self.preencher(Retangulo { esq: r.esq, topo: y as i16, dir: x as i16, base: y as i16 + h }, fundo, fundo);
            self.preencher(Retangulo { esq: (x + w) as i16, topo: y as i16, dir: r.dir, base: y as i16 + h }, fundo, fundo);
        }
    }

    pub fn bitmap(&mut self, bm: &Bitmap, x: i32, y: i32) {
        for j in 0..bm.altura {
            for i in 0..bm.largura {
                self.ponto(x + i as i32, y + j as i32, bm.pixels[j * bm.largura + i]);
            }
        }
    }
}

fn deslocar(r: Retangulo, dx: i16, dy: i16) -> Retangulo {
    Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy }
}

/// Desenha os objetos visíveis 1..N−1 da janela, na ordem do arquivo (`_win_DrawWindow`).
/// `pos`: onde o canto superior esquerdo do objeto 0 cai no quadro. `carregar_bitmap` devolve None se o bitmap faltar.
/// Devolve os nomes dos objetos que não foram desenhados por falta de suporte (texto, por enquanto).
pub fn desenhar_janela(
    q: &mut Quadro,
    janela: &Janela,
    pos: (i16, i16),
    cores: &[Cor],
    fontes: &[Option<Fonte>],
    carregar_bitmap: &mut dyn FnMut(u16) -> Option<Bitmap>,
) -> Vec<String> {
    let base = janela.retangulo();
    let (dx, dy) = (pos.0 - base.esq, pos.1 - base.topo);
    let mut pendentes = Vec::new();
    let cor_de = |i: u8| cores.get(i as usize).copied().unwrap_or(Cor { frente: 0, fundo: 15, fundo2: 15 });
    for ob in janela.objetos.iter().skip(1).filter(|o| o.visivel()) {
        let r = deslocar(ob.retangulo, dx, dy);
        let cor = cor_de(if ob.selecionado() { ob.cor_selecionado } else { ob.cor });
        match &ob.conteudo {
            Conteudo::Preenchimento => q.preencher(r, cor.fundo, cor.fundo2),
            Conteudo::Bitmap { id } => match carregar_bitmap(*id) {
                Some(bm) => q.bitmap(&bm, r.esq as i32, r.topo as i32),
                None => q.preencher(r, cor.fundo, cor.fundo),
            },
            Conteudo::BotaoBitmap { selecionado, normal } => {
                let id = if ob.selecionado() { *selecionado } else { *normal };
                if let Some(bm) = carregar_bitmap(id) {
                    q.bitmap(&bm, r.esq as i32, r.topo as i32);
                }
            }
            Conteudo::Contorno => q.contorno(r, cor.frente),
            Conteudo::Relevo { espessura, invertido, preencher } => {
                if *preencher {
                    q.preencher(r, cor.fundo, cor.fundo2);
                }
                q.relevo(r, *espessura, *invertido);
            }
            Conteudo::Texto { fonte, texto, preencher } => {
                if *preencher {
                    q.preencher(r, cor.fundo, cor.fundo2);
                }
                // _font_SetFont: índices fora de 0..=5 viram 2.
                let i = if *fonte > 5 { 2 } else { *fonte as usize };
                match fontes.get(i).and_then(|f| f.as_ref()) {
                    // Textos com '%' são formatos preenchidos em tempo de execução.
                    Some(f) if !texto.contains('%') => {
                        let alinhamento = (ob.flags & 0x180) >> 7;
                        q.texto(r, f, texto.as_bytes(), alinhamento, cor.frente, cor.fundo);
                    }
                    _ => pendentes.push(ob.nome.clone()),
                }
            }
            Conteudo::Janela | Conteudo::Area | Conteudo::Outro(_) => {}
        }
    }
    pendentes
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn preencher_com_duas_cores_faz_xadrez() {
        let mut q = Quadro::new(2, 2, 0);
        q.preencher(Retangulo { esq: 0, topo: 0, dir: 2, base: 2 }, 7, 8);
        assert_eq!(q.pixels, vec![7, 8, 8, 7]);
    }

    #[test]
    fn recorte_limita_o_desenho() {
        let mut q = Quadro::new(3, 1, 0);
        q.recorte = Some(Retangulo { esq: 1, topo: 0, dir: 2, base: 1 });
        q.preencher(Retangulo { esq: 0, topo: 0, dir: 3, base: 1 }, 5, 5);
        assert_eq!(q.pixels, vec![0, 5, 0]);
    }

    #[test]
    fn desenho_fora_do_quadro_e_recortado() {
        let mut q = Quadro::new(2, 2, 0);
        q.preencher(Retangulo { esq: -5, topo: -5, dir: 10, base: 1 }, 3, 3);
        assert_eq!(q.pixels, vec![3, 3, 0, 0]);
    }
}
