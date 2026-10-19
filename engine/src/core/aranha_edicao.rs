//! A aranha na janela de edição (`_PreDrawSpider` `1010:1c18`, `_DrawSpider` `1010:1d6a`, `_DrawLegs`
//! `1010:2be0`, `_DrawPalps` `1010:2594`): o corpo é um sprite do banco; pernas e palpos são retas
//! desenhadas por cima. Especificação: docs/spec/telas/jogo.md, "Aranha na janela de edição". Tabelas:
//! `data/desenho_aranha.json`.
//!
//! O sorteio dos palpos (`S1(4)`) é das regras (`rules::desenho::desenhar_palpos`, que guarda o
//! índice em `Estado::palpos_aranha`); aqui só se desenha com o índice recebido.

use serde::Deserialize;

use super::edicao::VistaEdicao;
use super::mapa::Plano;
use super::quadro::Quadro;
use crate::formats::banco::Chave;
use crate::formats::bitmap::{Bitmap, TIPO_BITMAP};
use crate::formats::janela::Retangulo;
use crate::formats::tiles::LADO;
use crate::recursos::Recursos;

/// Arquivo de dados com as tabelas do desenho.
pub const ARQUIVO: &str = "desenho_aranha.json";

/// Pernas de um lado para a direção-base (0 nas retas, 1 nas diagonais): onde cada uma das 4 sai do
/// corpo e, por quadro de animação (0–7), o joelho e o pé.
#[derive(Clone, Debug, Deserialize)]
pub struct Pernas {
    pub inicio_x: [i8; 4],
    pub inicio_y: [i8; 4],
    pub joelho_x: [[i8; 8]; 4],
    pub joelho_y: [[i8; 8]; 4],
    pub pe_x: [[i8; 8]; 4],
    pub pe_y: [[i8; 8]; 4],
}

/// Um palpo da direção-base: sai de `inicio` e passa pelo joelho até a ponta; o índice das tabelas é o
/// `S1(4)` do desenho.
#[derive(Clone, Debug, Deserialize)]
pub struct Palpos {
    pub inicio: [i8; 2],
    pub joelho_x: [i8; 4],
    pub joelho_y: [i8; 4],
    pub ponta_x: [i8; 4],
    pub ponta_y: [i8; 4],
}

/// Conteúdo de `data/desenho_aranha.json`.
#[derive(Clone, Debug, Deserialize)]
pub struct TabelasDesenhoAranha {
    /// Sprites (objeto tipo 2): corpo = `sprite_corpo + _SpidDir`; agonia = `sprite_agonia + _Scycle`.
    pub sprite_corpo: u16,
    pub sprite_agonia: u16,
    /// Índice que não é copiado do sprite (`_CopyMaskBitmap2`).
    pub cor_transparente: u8,
    pub cor_pernas: u8,
    pub cor_palpos: u8,
    /// Palpos da aranha controlada pelo jogador (`_MeMode` = 1).
    pub cor_palpos_jogador: u8,
    /// Canto do sprite em relação ao centro da aranha (`_BodX/_BodY` por direção; `_DBodX/_DBodY` por
    /// quadro da agonia).
    pub corpo_x: [i8; 8],
    pub corpo_y: [i8; 8],
    pub agonia_x: [i8; 4],
    pub agonia_y: [i8; 4],
    pub pernas_reto: Pernas,
    pub pernas_diagonal: Pernas,
    pub palpos_reto: Palpos,
    pub palpos_diagonal: Palpos,
}

/// O que o desenho lê do estado: `_SpidOn`, `_SpidX/_SpidY` (pixels do mapa, 16 por célula),
/// `_SpidDir`, `_SMode`, `_Scycle`, `_MeMode` = 1 e o índice dos palpos do último desenho.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AranhaNaEdicao {
    pub ligada: bool,
    pub x: i32,
    pub y: i32,
    pub dir: i32,
    pub estado: i32,
    pub ciclo: i32,
    pub jogador: bool,
    pub palpos: u8,
}

impl AranhaNaEdicao {
    /// `_PreDrawSpider`: ligada, plano 0 ou 1 e o centro dentro da vista (bordas incluídas, em pixels).
    pub fn visivel(&self, vista: &VistaEdicao) -> bool {
        let lado = LADO as i32;
        let (x0, y0) = (vista.x * lado, vista.y * lado);
        self.ligada
            && vista.plano == Plano::Superficie
            && (x0..=x0 + vista.largura * lado).contains(&self.x)
            && (y0..=y0 + vista.altura * lado).contains(&self.y)
    }
}

/// Tabelas e sprites carregados.
#[derive(Clone, Debug)]
pub struct DesenhoAranha {
    pub tabelas: TabelasDesenhoAranha,
    corpo: Vec<Option<Bitmap>>,
    agonia: Vec<Option<Bitmap>>,
}

fn sprite(recursos: &Recursos, id: u16) -> Option<Bitmap> {
    recursos.objeto(Chave { tipo: TIPO_BITMAP, id }).ok().and_then(|d| Bitmap::decodificar(&d).ok())
}

/// Giro/espelho da direção `d` aplicado a um ponto da direção-base (0 nas retas, 1 nas diagonais).
fn girar(d: i32, (x, y): (i32, i32)) -> (i32, i32) {
    match d & 7 {
        0 | 1 => (x, y),
        2 => (-y, x),
        3 => (x, -y),
        4 | 5 => (-x, -y),
        6 => (y, -x),
        _ => (-x, y),
    }
}

/// O lado oposto do corpo na direção-base: espelho no eixo do corpo (x → −x nas retas; nas diagonais,
/// (x, y) → (−y, −x)).
fn espelhar(d: i32, (x, y): (i32, i32)) -> (i32, i32) {
    if d & 1 == 0 {
        (-x, y)
    } else {
        (-y, -x)
    }
}

/// `_PlotLine` (`1018:6fbe`): reta por acumulador de 16 bits que começa em 0xFFFF (o eixo menor anda
/// já no primeiro passo). Eixo y maior: |dy| + 1 pontos; eixo x maior: |dx| pontos (sem o último);
/// com |dx| = |dy|, eixo x com |dx| + 1. Sempre percorrida no sentido crescente do eixo maior.
pub fn reta_simant(q: &mut Quadro, x0: i32, y0: i32, x1: i32, y1: i32, cor: u8) {
    let (mut dx, dy) = ((x1 - x0).unsigned_abs(), (y1 - y0).unsigned_abs());
    let (x_desce, y_desce) = (x1 < x0, y1 < y0);
    if dx == dy {
        dx += 1;
    } else if dx < dy {
        let mut passo = if x_desce { -1 } else { 1 };
        let (mut x, mut y) = (x0, y0);
        if y_desce {
            passo = -passo;
            (x, y) = (x1, y1);
        }
        let fracao = ((dx << 16) / dy) as u16;
        let mut acum: u16 = 0xFFFF;
        for _ in 0..=dy {
            q.ponto(x, y, cor);
            y += 1;
            let (n, vai_um) = acum.overflowing_add(fracao);
            acum = n;
            if vai_um {
                x += passo;
            }
        }
        return;
    }
    let mut passo = if y_desce { -1 } else { 1 };
    let (mut x, mut y) = (x0, y0);
    if x_desce {
        passo = -passo;
        (x, y) = (x1, y1);
    }
    let fracao = ((dy << 16) / dx) as u16;
    let mut acum: u16 = 0xFFFF;
    for _ in 0..dx {
        q.ponto(x, y, cor);
        x += 1;
        let (n, vai_um) = acum.overflowing_add(fracao);
        acum = n;
        if vai_um {
            y += passo;
        }
    }
}

/// `_ed_LineTo` (`1010:22fa`): a reta com 2 pixels de espessura — repetida 1 pixel abaixo se o eixo x
/// é o maior, senão 1 pixel à direita.
fn reta_grossa(q: &mut Quadro, (x0, y0): (i32, i32), (x1, y1): (i32, i32), cor: u8) {
    reta_simant(q, x0, y0, x1, y1, cor);
    if (y0 - y1).abs() < (x0 - x1).abs() {
        reta_simant(q, x0, y0 + 1, x1, y1 + 1, cor);
    } else {
        reta_simant(q, x0 + 1, y0, x1 + 1, y1, cor);
    }
}

impl DesenhoAranha {
    pub fn carregar(recursos: &Recursos) -> Result<DesenhoAranha, String> {
        let tabelas: TabelasDesenhoAranha = recursos.dados(ARQUIVO)?;
        let corpo = (0..8).map(|d| sprite(recursos, tabelas.sprite_corpo + d)).collect();
        let agonia = (0..4).map(|q| sprite(recursos, tabelas.sprite_agonia + q)).collect();
        Ok(DesenhoAranha { tabelas, corpo, agonia })
    }

    /// Desenha a aranha na vista (`_DrawSpider`), se visível. `origem` = canto da vista no quadro; o
    /// desenho fica dentro do buffer da vista (`largura × altura` células) e do recorte do quadro.
    /// Devolve se desenhou.
    pub fn desenhar(&self, q: &mut Quadro, a: &AranhaNaEdicao, vista: &VistaEdicao, origem: (i32, i32)) -> bool {
        if !a.visivel(vista) {
            return false;
        }
        let lado = LADO as i32;
        let buffer = Retangulo {
            esq: origem.0 as i16,
            topo: origem.1 as i16,
            dir: (origem.0 + vista.largura * lado) as i16,
            base: (origem.1 + vista.altura * lado) as i16,
        };
        let antes = q.recorte;
        q.recorte = Some(match antes {
            Some(r) => Retangulo {
                esq: r.esq.max(buffer.esq),
                topo: r.topo.max(buffer.topo),
                dir: r.dir.min(buffer.dir),
                base: r.base.min(buffer.base),
            },
            None => buffer,
        });
        // centro da aranha no quadro
        let c = (origem.0 + a.x - vista.x * lado, origem.1 + a.y - vista.y * lado);
        let t = &self.tabelas;
        let d = a.dir & 7;
        if a.estado == 5 {
            let k = (a.ciclo & 3) as usize;
            if let Some(bm) = self.agonia.get(k).and_then(Option::as_ref) {
                let (dx, dy) = (i32::from(t.agonia_x[k]), i32::from(t.agonia_y[k]));
                q.bitmap_transparente(bm, c.0 + dx, c.1 + dy, t.cor_transparente);
            }
        } else {
            if let Some(bm) = self.corpo.get(d as usize).and_then(Option::as_ref) {
                let (dx, dy) = (i32::from(t.corpo_x[d as usize]), i32::from(t.corpo_y[d as usize]));
                q.bitmap_transparente(bm, c.0 + dx, c.1 + dy, t.cor_transparente);
            }
            self.desenhar_pernas(q, c, d, a.ciclo & 7);
            self.desenhar_palpos(q, c, d, a);
        }
        q.recorte = antes;
        true
    }

    /// `_DrawLegs`: 4 pernas de cada lado, cada uma em duas retas (corpo → joelho → pé); um lado com o
    /// quadro `_Scycle & 7`, o outro com o quadro + 4 (o passo alternado).
    fn desenhar_pernas(&self, q: &mut Quadro, c: (i32, i32), d: i32, quadro: i32) {
        let t = &self.tabelas;
        let p = if d & 1 == 0 { &t.pernas_reto } else { &t.pernas_diagonal };
        for (lado, f) in [(false, quadro), (true, (quadro + 4) & 7)] {
            let f = f as usize;
            for k in 0..4 {
                let pontos = [
                    (p.inicio_x[k], p.inicio_y[k]),
                    (p.joelho_x[k][f], p.joelho_y[k][f]),
                    (p.pe_x[k][f], p.pe_y[k][f]),
                ]
                .map(|(x, y)| {
                    let b = (i32::from(x), i32::from(y));
                    let (x, y) = girar(d, if lado { espelhar(d, b) } else { b });
                    (c.0 + x, c.1 + y)
                });
                reta_grossa(q, pontos[0], pontos[1], t.cor_pernas);
                reta_grossa(q, pontos[1], pontos[2], t.cor_pernas);
            }
        }
    }

    /// `_DrawPalps`: dois palpos (início → joelho → ponta), espelhados; o índice das tabelas é o `S1(4)`
    /// do desenho (0 quando não há sorteio).
    fn desenhar_palpos(&self, q: &mut Quadro, c: (i32, i32), d: i32, a: &AranhaNaEdicao) {
        let t = &self.tabelas;
        let p = if d & 1 == 0 { &t.palpos_reto } else { &t.palpos_diagonal };
        let r = usize::from(a.palpos & 3);
        let cor = if a.jogador { t.cor_palpos_jogador } else { t.cor_palpos };
        for lado in [false, true] {
            let pontos = [(p.inicio[0], p.inicio[1]), (p.joelho_x[r], p.joelho_y[r]), (p.ponta_x[r], p.ponta_y[r])].map(|(x, y)| {
                let b = (i32::from(x), i32::from(y));
                let (x, y) = girar(d, if lado { espelhar(d, b) } else { b });
                (c.0 + x, c.1 + y)
            });
            reta_grossa(q, pontos[0], pontos[1], cor);
            reta_grossa(q, pontos[1], pontos[2], cor);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pontos(q: &Quadro) -> Vec<(usize, usize)> {
        (0..q.altura).flat_map(|y| (0..q.largura).map(move |x| (x, y))).filter(|&(x, y)| q.pixels[y * q.largura + x] == 1).collect()
    }

    #[test]
    fn reta_com_eixo_y_maior_tem_dy_mais_1_pontos_e_anda_x_no_primeiro_passo() {
        let mut q = Quadro::new(10, 10, 0);
        reta_simant(&mut q, 2, 1, 3, 4, 1);
        assert_eq!(pontos(&q), vec![(2, 1), (3, 2), (3, 3), (3, 4)]);
    }

    #[test]
    fn reta_com_eixo_x_maior_nao_desenha_o_ultimo_ponto() {
        let mut q = Quadro::new(10, 10, 0);
        reta_simant(&mut q, 5, 2, 1, 2, 1);
        assert_eq!(pontos(&q), vec![(1, 2), (2, 2), (3, 2), (4, 2)]);
    }

    #[test]
    fn diagonal_exata_vira_eixo_x_com_um_ponto_a_mais() {
        let mut q = Quadro::new(10, 10, 0);
        reta_simant(&mut q, 0, 0, 2, 2, 1);
        assert_eq!(pontos(&q).len(), 3);
    }

    #[test]
    fn giros_levam_o_norte_para_cada_direcao_reta() {
        // um ponto à frente da aranha (0, −10) na direção 0
        assert_eq!(girar(0, (0, -10)), (0, -10));
        assert_eq!(girar(2, (0, -10)), (10, 0));
        assert_eq!(girar(4, (0, -10)), (0, 10));
        assert_eq!(girar(6, (0, -10)), (-10, 0));
        // nas diagonais, a base é a direção 1 (nordeste)
        assert_eq!(girar(3, (10, -10)), (10, 10));
        assert_eq!(girar(5, (10, -10)), (-10, 10));
        assert_eq!(girar(7, (10, -10)), (-10, -10));
    }
}
