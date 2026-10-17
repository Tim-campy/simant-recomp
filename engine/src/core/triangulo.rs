//! Controle triangular das janelas de casta e de comportamento (`_casteLevels`, `_modeLevels`).
//! Especificação: docs/spec/telas/janelas.md, "Triângulo". Sem janela: só a conta entre o ponto
//! do botão (knob) e os três níveis (0..65535, somando 65535).
//!
//! Vértices (`_InitTriVars`, `1000:b324`): com o retângulo `[esq, dir) × [topo, base)` do objeto
//! do triângulo, o vértice de cima fica em (esq + largura/2, topo) e os de baixo em (esq, base) e
//! (dir, base). O nível 0 é o do vértice de cima; o 2 cresce para a direita; o 1 é o resto.

/// Triângulo isósceles de um objeto (coordenadas da área cliente).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Triangulo {
    pub esq: i32,
    pub topo: i32,
    pub dir: i32,
    pub base: i32,
}

/// Divisão inteira truncada para zero, como o `__aFldiv` do runtime C.
fn div(a: i64, b: i64) -> i64 {
    if b == 0 {
        0
    } else {
        a / b
    }
}

impl Triangulo {
    /// `_triWidth`.
    fn largura(&self) -> i32 {
        self.dir - self.esq
    }
    /// `_triWidthL` = `_triWidthR` = metade (deslocamento à direita).
    fn meia(&self) -> i32 {
        ((self.dir - self.esq) as u16 >> 1) as i32
    }
    /// `_triHeight`.
    fn altura(&self) -> i32 {
        self.base - self.topo
    }
    fn apice_x(&self) -> i32 {
        self.meia() + self.esq
    }

    /// Ponto dentro do triângulo, como o teste do clique de `_ProcCasteEvent`/`_ProcModeEvent`
    /// (o mesmo de `_IsPointInIsoTri`).
    pub fn contem(&self, x: i32, y: i32) -> bool {
        let (l, t, r, b) = (self.esq as i64, self.topo as i64, self.dir as i64, self.base as i64);
        let meio = (r + l) / 2;
        let (x, y) = (x as i64, y as i64);
        if !(t <= y && y < b) {
            return false;
        }
        let lim_esq = div((l - meio) * (y - b), b - t) + l;
        let lim_dir = div((meio - r) * (y - t), t - b) + meio;
        x >= lim_esq && x <= lim_dir
    }

    /// `_BoundPointToTri` (`1000:c438`): leva o ponto para dentro do triângulo (base − 1 em y; os
    /// lados na linha do ponto em x).
    pub fn limitar(&self, x: i32, y: i32) -> (i32, i32) {
        let (l, t, r, b1) = (self.esq as i64, self.topo as i64, self.dir as i64, (self.base - 1) as i64);
        let meio = (r + l) / 2;
        let y = (y as i64).clamp(t.min(b1), b1);
        let y = if y < t { t } else { y };
        let x = x as i64;
        let min_x = div((l - meio) * (y - b1), b1 - t) + l;
        let x = if min_x <= x {
            let max_x = div((meio - r) * (y - t), t - b1) + meio;
            x.min(max_x)
        } else {
            min_x
        };
        (x as i32, y as i32)
    }

    /// `_GetTriLatDist` (`1000:ca7a`): níveis do ponto (já limitado).
    pub fn niveis(&self, x: i32, y: i32) -> [u16; 3] {
        let h = self.altura();
        let dy = (y - self.topo) as u16;
        let n0: u16 = if ((h - 2) as u16) < dy {
            0
        } else {
            div(((h as i64) - dy as i64 - 2) * 0xFFFF, (h - 2) as i64) as u16
        };
        let w0 = ((n0 as u32 * self.meia() as u32) / 0xFFFF) as i32;
        let linha = self.largura() - 2 * w0;
        if linha < 3 {
            return [n0, 0, 0];
        }
        let dx = (x - self.esq) - w0;
        let n2: u16 = if dx < linha - 2 {
            if dx < 3 {
                0
            } else {
                div((0xFFFF - n0 as i64) * dx as i64, (linha - 2) as i64) as u16
            }
        } else {
            0xFFFF - n0
        };
        let n1 = 0xFFFFu16.wrapping_sub(n2).wrapping_sub(n0);
        [n0, n1, n2]
    }

    /// Posição do botão (centro) para os níveis (`_UpdateCasteWindow`, `_SetTriLatPoint`).
    pub fn ponto(&self, n: [u16; 3]) -> (i32, i32) {
        let y = (((self.altura() - 2) as u32 * (0xFFFF - n[0] as u32)) / 0xFFFF) as i32 + self.topo;
        let w0 = ((self.meia() as u32 * n[0] as u32) / 0xFFFF) as i32;
        let linha = self.largura() - 2 * w0;
        let x = if n[0] == 0xFFFF || linha < 3 {
            self.apice_x()
        } else {
            div((linha - 3) as i64 * n[2] as i64, 0xFFFF - n[0] as i64) as i32 + self.esq + w0
        };
        (x + 2, y)
    }
}

/// `_cvtLevels2IdealCaste` (`1000:c9de`): `_IdealCaste` (%) = operárias, soldados, machos, rainhas.
pub fn castas_ideais(n: [u16; 3]) -> [i16; 4] {
    let p = |v: u16, k: u32| ((v as u32 * k + 0x3FFF) / 0xFFFF) as i16;
    [p(n[1], 100), p(n[2], 100), p(n[0], 50), p(n[0], 50)]
}

/// Texto do nível (`_DrawControlLevels`): porcentagem ("%-ld%%") ou o nível vezes o total
/// ("%-ld").
pub fn texto_do_nivel(nivel: u16, porcentagem: bool, total: i32) -> String {
    if porcentagem {
        format!("{}%", (nivel as u64 * 100 + 0x3FFF) / 0xFFFF)
    } else {
        format!("{}", (nivel as i64 * total as i64 + 0x3FFF).div_euclid(0xFFFF))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn t() -> Triangulo {
        // casteTriangle (392, 342)–(501, 438) no layout.
        Triangulo { esq: 392, topo: 342, dir: 501, base: 438 }
    }

    #[test]
    fn apice_e_base() {
        let t = t();
        assert_eq!(t.niveis(446, 342), [0xFFFF, 0, 0]);
        let [n0, n1, n2] = t.niveis(392, 437);
        assert_eq!((n0, n2), (0, 0));
        assert_eq!(n1, 0xFFFF);
        assert_eq!(t.niveis(501, 437)[2], 0xFFFF);
    }

    #[test]
    fn niveis_somam_65535() {
        let t = t();
        for (x, y) in [(446, 400), (420, 430), (470, 420), (446, 360)] {
            let (x, y) = t.limitar(x, y);
            let n = t.niveis(x, y);
            assert_eq!(n.iter().map(|&v| v as u32).sum::<u32>(), 0xFFFF, "{x},{y}");
        }
    }

    #[test]
    fn limitar_prende_nos_lados() {
        let t = t();
        assert_eq!(t.limitar(446, 300), (446, 342));
        assert_eq!(t.limitar(100, 437).0, 392);
        assert_eq!(t.limitar(446, 500).1, 437);
        assert!(t.contem(446, 400));
        assert!(!t.contem(395, 345));
    }

    #[test]
    fn ponto_e_niveis_ida_e_volta_aproximada() {
        let t = t();
        // QUICK2: casteLevels 39321, 13107, 13107 (60%, 20%, 20%).
        let (x, y) = t.ponto([39321, 13107, 13107]);
        let n = t.niveis(x - 2, y);
        assert!((n[0] as i32 - 39321).abs() < 1500, "{n:?}");
        assert_eq!(castas_ideais([39321, 13107, 13107]), [20, 20, 30, 30]);
        assert_eq!(texto_do_nivel(39321, true, 0), "60%");
        assert_eq!(texto_do_nivel(39321, false, 10), "6");
    }
}
