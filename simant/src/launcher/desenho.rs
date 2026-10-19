//! Desenho por software do launcher (RGB de 24 bits), no estilo dos diálogos do Windows 3.1:
//! cinza com relevo, campos brancos com contorno preto, botões com cantos cortados. Não usa nada
//! do jogo original (a fonte é a de `fonte8x8.rs`), para funcionar antes de escolher a pasta.

use super::fonte8x8;

pub type Cor = [u8; 3];

pub const FACE: Cor = [192, 192, 192];
pub const BRANCO: Cor = [255, 255, 255];
pub const PRETO: Cor = [0, 0, 0];
pub const CINZA: Cor = [128, 128, 128];
pub const AZUL: Cor = [0, 0, 128];
pub const VERDE: Cor = [0, 112, 0];
pub const VERMELHO: Cor = [176, 0, 0];

/// Largura e altura de um caractere.
pub const CAR: i32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ret {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Ret {
    pub fn contem(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

pub struct Tela {
    pub largura: i32,
    pub altura: i32,
    pub rgb: Vec<u8>,
}

impl Tela {
    pub fn nova(largura: i32, altura: i32) -> Tela {
        Tela { largura, altura, rgb: FACE.repeat((largura * altura) as usize) }
    }

    pub fn ponto(&mut self, x: i32, y: i32, c: Cor) {
        if x >= 0 && y >= 0 && x < self.largura && y < self.altura {
            let i = ((y * self.largura + x) * 3) as usize;
            self.rgb[i..i + 3].copy_from_slice(&c);
        }
    }

    pub fn cheio(&mut self, r: Ret, c: Cor) {
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                self.ponto(x, y, c);
            }
        }
    }

    pub fn horizontal(&mut self, x: i32, y: i32, w: i32, c: Cor) {
        self.cheio(Ret { x, y, w, h: 1 }, c);
    }

    pub fn vertical(&mut self, x: i32, y: i32, h: i32, c: Cor) {
        self.cheio(Ret { x, y, w: 1, h }, c);
    }

    pub fn contorno(&mut self, r: Ret, c: Cor) {
        self.horizontal(r.x, r.y, r.w, c);
        self.horizontal(r.x, r.y + r.h - 1, r.w, c);
        self.vertical(r.x, r.y, r.h, c);
        self.vertical(r.x + r.w - 1, r.y, r.h, c);
    }

    /// Relevo de `espessura` pixels: claro em cima e à esquerda (ou escuro, se `afundado`).
    pub fn relevo(&mut self, r: Ret, afundado: bool, espessura: i32) {
        let (luz, sombra) = if afundado { (CINZA, BRANCO) } else { (BRANCO, CINZA) };
        for i in 0..espessura {
            self.horizontal(r.x + i, r.y + i, r.w - 2 * i, luz);
            self.vertical(r.x + i, r.y + i, r.h - 2 * i, luz);
            self.horizontal(r.x + i, r.y + r.h - 1 - i, r.w - 2 * i, sombra);
            self.vertical(r.x + r.w - 1 - i, r.y + i, r.h - 2 * i, sombra);
        }
    }

    /// Texto na fonte 8 × 8 ampliada `escala` vezes; devolve a largura desenhada.
    pub fn texto(&mut self, x: i32, y: i32, s: &str, c: Cor, escala: i32) -> i32 {
        let mut cx = x;
        for ch in s.chars() {
            let g = fonte8x8::glifo(ch);
            for (ly, linha) in g.iter().enumerate() {
                for lx in 0..8 {
                    if linha >> lx & 1 == 1 {
                        self.cheio(Ret { x: cx + lx * escala, y: y + ly as i32 * escala, w: escala, h: escala }, c);
                    }
                }
            }
            cx += CAR * escala;
        }
        cx - x
    }

    /// Texto "negrito" (desenhado duas vezes, um pixel ao lado).
    pub fn negrito(&mut self, x: i32, y: i32, s: &str, c: Cor) -> i32 {
        self.texto(x + 1, y, s, c, 1);
        self.texto(x, y, s, c, 1) + 1
    }

    /// Botão do Windows 3.1: contorno preto com cantos cortados, relevo de 2 pixels (o padrão, com
    /// contorno mais grosso), afundado quando apertado.
    pub fn botao(&mut self, r: Ret, rotulo: &str, padrao: bool, apertado: bool) {
        self.cheio(Ret { x: r.x + 1, y: r.y + 1, w: r.w - 2, h: r.h - 2 }, FACE);
        self.horizontal(r.x + 1, r.y, r.w - 2, PRETO);
        self.horizontal(r.x + 1, r.y + r.h - 1, r.w - 2, PRETO);
        self.vertical(r.x, r.y + 1, r.h - 2, PRETO);
        self.vertical(r.x + r.w - 1, r.y + 1, r.h - 2, PRETO);
        let mut dentro = Ret { x: r.x + 1, y: r.y + 1, w: r.w - 2, h: r.h - 2 };
        if padrao {
            self.contorno(dentro, PRETO);
            dentro = Ret { x: dentro.x + 1, y: dentro.y + 1, w: dentro.w - 2, h: dentro.h - 2 };
        }
        if apertado {
            self.horizontal(dentro.x, dentro.y, dentro.w, CINZA);
            self.vertical(dentro.x, dentro.y, dentro.h, CINZA);
        } else {
            self.relevo(dentro, false, 2);
        }
        let w = rotulo.chars().count() as i32 * CAR;
        let d = i32::from(apertado);
        self.texto(r.x + (r.w - w) / 2 + d, r.y + (r.h - CAR) / 2 + d, rotulo, PRETO, 1);
    }

    /// Caixa de marcar do Windows 3.1 (10 × 10, com um X).
    pub fn caixa(&mut self, x: i32, y: i32, marcada: bool, ativa: bool) {
        let r = Ret { x, y, w: 10, h: 10 };
        self.cheio(r, BRANCO);
        self.contorno(r, if ativa { PRETO } else { CINZA });
        if marcada {
            let c = if ativa { PRETO } else { CINZA };
            for i in 2..8 {
                self.ponto(x + i, y + i, c);
                self.ponto(x + 9 - i, y + i, c);
            }
        }
    }

    /// Botão de opção (círculo de 10 × 10, com o miolo preto quando escolhido).
    pub fn opcao(&mut self, x: i32, y: i32, marcada: bool, ativa: bool) {
        const CIRCULO: [&str; 10] = ["...####...", ".##....##.", ".#......#.", "#........#", "#........#", "#........#", "#........#", ".#......#.", ".##....##.", "...####..."];
        const DENTRO: [&str; 10] = ["..........", "...####...", "..######..", ".########.", ".########.", ".########.", ".########.", "..######..", "...####...", ".........."];
        let c = if ativa { PRETO } else { CINZA };
        for (ly, (borda, miolo)) in CIRCULO.iter().zip(DENTRO).enumerate() {
            for (lx, (b, m)) in borda.bytes().zip(miolo.bytes()).enumerate() {
                let (px, py) = (x + lx as i32, y + ly as i32);
                if b == b'#' {
                    self.ponto(px, py, c);
                } else if m == b'#' {
                    self.ponto(px, py, BRANCO);
                }
            }
        }
        if marcada {
            self.cheio(Ret { x: x + 3, y: y + 3, w: 4, h: 4 }, c);
        }
    }
}
