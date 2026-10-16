//! Vista da janela de edição (janela 0, `editWin`): qual mapa, a partir de que célula e com quantas
//! células, e o desenho de chão + vida. Especificação: docs/spec/telas/jogo.md.

use super::mapa::{desenhar_tile, desenhar_tile_de_vida, MapaChao, Plano, ALTURA};
use super::quadro::Quadro;
use super::vida::{tile_de_vida, CamadasVida, FormigaAmarela};
use crate::formats::save::JogoSalvo;
use crate::formats::tiles::{Tileset, LADO};

/// Estado da vista: `_MapPlane`, `_MapPnt` (célula do canto superior esquerdo) e `_editWidth/_editHeight`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VistaEdicao {
    pub plano: Plano,
    pub x: i32,
    pub y: i32,
    pub largura: i32,
    pub altura: i32,
}

/// Número de tiles que cobrem `pixels` (`(pixels + tile − 1) / tile`, como em `_win_EditChanged`).
pub fn tiles_para(pixels: i32) -> i32 {
    (pixels + LADO as i32 - 1) / LADO as i32
}

impl VistaEdicao {
    /// Vista com o `_MapPlane` e o `_MapPnt` salvos, já limitada.
    pub fn do_save(jogo: &JogoSalvo, largura: i32, altura: i32) -> Result<VistaEdicao, String> {
        let n = jogo.i16("MapPlane")?;
        let plano = super::vida::plano_de(n).ok_or(format!("_MapPlane = {n} desconhecido"))?;
        let (x, y) = jogo.par_i16("MapPnt")?;
        let mut v = VistaEdicao { plano, x: x as i32, y: y as i32, largura, altura };
        v.limitar();
        Ok(v)
    }

    /// `_BoundEditPosition`: mantém a vista dentro do mapa do plano.
    pub fn limitar(&mut self) {
        let largura_mapa = self.plano.largura() as i32;
        if self.x < 0 {
            self.x = 0;
        } else if self.x + self.largura > largura_mapa {
            self.x = largura_mapa - self.largura;
        }
        if self.y < 0 {
            self.y = 0;
        } else if self.y + self.altura > ALTURA as i32 {
            self.y = ALTURA as i32 - self.altura;
        }
    }

    /// `_CenterEdit`: anda a vista para centralizar (cx, cy), com o passo de reta em ponto fixo de
    /// `FUN_1010_616c` (o eixo menor pode andar um passo a menos). Devolve false se já estava lá.
    pub fn centralizar(&mut self, cx: i32, cy: i32) -> bool {
        let dx = cx - self.x - self.largura / 2;
        let dy = cy - self.y - self.altura / 2;
        if dx == 0 && dy == 0 {
            return false;
        }
        let (passo_x, passo_y) = (dx.signum(), dy.signum());
        let (ax, ay) = (dx.unsigned_abs(), dy.unsigned_abs());
        // Eixo maior: |d| passos; eixo menor: acumula (menor << 16) / maior e anda quando o bit 16 liga.
        let x_maior = ay < ax;
        let (maior, menor) = if x_maior { (ax, ay) } else { (ay, ax) };
        let fracao = (menor << 16) / maior;
        let mut acumulador: u32 = 0;
        for _ in 0..maior {
            acumulador += fracao;
            let anda_menor = acumulador & 0x1_0000 != 0;
            acumulador &= !0x1_0000;
            if x_maior {
                self.x += passo_x;
                if anda_menor {
                    self.y += passo_y;
                }
            } else {
                self.y += passo_y;
                if anda_menor {
                    self.x += passo_x;
                }
            }
        }
        self.limitar();
        true
    }

    /// Célula do mapa mostrada na posição (col, lin) da vista (`FUN_1010_6250`): passando da
    /// linha 63, soma 64 a x e dá a volta em y.
    pub fn celula(&self, col: i32, lin: i32) -> (usize, usize) {
        let mut x = self.x + col;
        let mut y = self.y + lin;
        if y > 63 {
            x += 64;
            y &= 63;
        }
        // _DoCalcTile mascara x pela largura do plano (x & 0x7f ou x & 0x3f).
        let x = (x as usize) & (self.plano.largura() - 1);
        (x, y as usize)
    }
}

/// O que é preciso para desenhar a vista: mapa de chão, vida e tilesets.
pub struct CenaEdicao<'a> {
    pub chao: MapaChao<'a>,
    pub vida: &'a CamadasVida,
    pub amarela: FormigaAmarela,
    pub tiles_chao: &'a Tileset,
    /// Buffer de vida do plano: objetos 9/15–17 (superfície) ou 9/18–20 (ninhos).
    pub tiles_vida: &'a Tileset,
}

/// Desenha as `largura × altura` células da vista com o canto em `destino` (pixels), como
/// `FUN_1010_16d4`: chão e, se houver, a vida por cima com transparência. O recorte do quadro limita
/// o que aparece. Devolve erro se algum tile não existir no tileset.
pub fn desenhar_vista(q: &mut Quadro, vista: &VistaEdicao, cena: &CenaEdicao, destino: (i32, i32)) -> Result<(), String> {
    if cena.chao.plano != vista.plano {
        return Err(format!("mapa do plano {:?} para a vista do plano {:?}", cena.chao.plano, vista.plano));
    }
    for lin in 0..vista.altura {
        for col in 0..vista.largura {
            let (x, y) = vista.celula(col, lin);
            let px = destino.0 + col * LADO as i32;
            let py = destino.1 + lin * LADO as i32;
            let i = cena.chao.tile(x, y);
            let t = cena.tiles_chao.tile(i as usize).ok_or_else(|| format!("tile de chão {i} ausente em ({x}, {y})"))?;
            desenhar_tile(q, t, px, py, None);
            let v = cena.vida.valor(vista.plano, x, y);
            if let Some(iv) = tile_de_vida(v, &cena.amarela) {
                let t = cena
                    .tiles_vida
                    .tile(iv)
                    .ok_or_else(|| format!("tile de vida {iv} (byte {v}) ausente em ({x}, {y})"))?;
                desenhar_tile_de_vida(q, t, px, py);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::formats::tiles::TRANSPARENTE;

    fn vista(x: i32, y: i32) -> VistaEdicao {
        VistaEdicao { plano: Plano::NinhoPreto, x, y, largura: 26, altura: 19 }
    }

    #[test]
    fn limites_do_ninho() {
        let mut v = vista(50, 60);
        v.limitar();
        assert_eq!((v.x, v.y), (38, 45));
        let mut v = vista(-3, -1);
        v.limitar();
        assert_eq!((v.x, v.y), (0, 0));
        let mut v = VistaEdicao { plano: Plano::Superficie, ..vista(110, 0) };
        v.limitar();
        assert_eq!(v.x, 102);
    }

    #[test]
    fn centralizar_no_quick2_ao_carregar() {
        // jogo.md: _MapPnt (22, 0) salvo, formiga amarela em (34, 9), 26 × 19 tiles -> x = 21.
        let mut v = vista(22, 0);
        assert!(v.centralizar(34, 9));
        assert_eq!((v.x, v.y), (21, 0));
        assert!(!v.centralizar(34, 9));
    }

    #[test]
    fn centralizar_trunca_o_eixo_menor() {
        // dx = 3, dy = 1: fração 21845, soma 65535 < 65536, y não anda.
        let mut v = vista(10, 10);
        v.centralizar(10 + 13 + 3, 10 + 9 + 1);
        assert_eq!((v.x, v.y), (13, 10));
        // dx = 2, dy = 1: fração 32768, y anda no segundo passo.
        let mut v = vista(10, 10);
        v.centralizar(10 + 13 + 2, 10 + 9 + 1);
        assert_eq!((v.x, v.y), (12, 11));
        // Eixo y maior, sentido negativo.
        let mut v = vista(20, 20);
        v.centralizar(20 + 13 - 1, 20 + 9 - 4);
        assert_eq!((v.x, v.y), (19, 16));
    }

    #[test]
    fn celula_da_volta_depois_da_linha_63() {
        let v = VistaEdicao { plano: Plano::Superficie, x: 5, y: 60, largura: 26, altura: 19 };
        assert_eq!(v.celula(0, 3), (5, 63));
        assert_eq!(v.celula(0, 4), (69, 0));
    }

    #[test]
    fn vida_por_cima_do_chao_com_transparencia() {
        let chao_bytes = vec![0u8; 64 * 64]; // tile de chão 0x90
        let mut tiles_chao = Tileset { tiles: vec![[1; LADO * LADO]; 256] };
        tiles_chao.tiles[0x90] = [2; LADO * LADO];
        let mut formiga = [TRANSPARENTE; LADO * LADO];
        formiga[0] = 9;
        let mut tiles_vida = Tileset { tiles: vec![[TRANSPARENTE; LADO * LADO]; 384] };
        tiles_vida.tiles[5] = formiga;
        let mut vida = CamadasVida::vazia();
        vida.definir(Plano::NinhoPreto, 1, 0, 5);
        let cena = CenaEdicao {
            chao: MapaChao::new(Plano::NinhoPreto, &chao_bytes).unwrap(),
            vida: &vida,
            amarela: FormigaAmarela { modo: 0, plano: 2, x: 0, y: 0, tipo: 16, dir: 0, ovo: 0, base: 0 },
            tiles_chao: &tiles_chao,
            tiles_vida: &tiles_vida,
        };
        let v = VistaEdicao { plano: Plano::NinhoPreto, x: 0, y: 0, largura: 2, altura: 1 };
        let mut q = Quadro::new(32, 16, 0);
        desenhar_vista(&mut q, &v, &cena, (0, 0)).unwrap();
        assert_eq!(q.pixels[16], 9);
        assert_eq!(q.pixels[17], 2);
        assert_eq!(q.pixels[0], 2);
    }
}
