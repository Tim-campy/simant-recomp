//! Barra de título da janela principal do SimAnt como o Windows 3.1 a desenha com a janela
//! maximizada (`_InitInstance` a abre com `SW_SHOWMAXIMIZED`): caixa do menu de sistema à esquerda,
//! o nome centrado na faixa azul (branca com a janela inativa), os botões de minimizar e de
//! restaurar à direita e a linha preta embaixo. É opcional no port (`data/video.json` →
//! `moldura_windows31`): a janela do sistema atual já tem título.
//!
//! O menu de sistema da janela principal é desenhado e tratado pela barra de menus
//! (`barra_menus::BarraMenus`, como um menu antes do File), porque o Windows o percorre junto com
//! ela pelo teclado (← no File, → no Speed). Especificação: docs/spec/telas/menus.md §8. Medidas:
//! capturas da bancada VGA (`re_trabalho/agente_menus/m00`, `m05b`; `re_trabalho/moldura/s00`–`s16`,
//! roteiro `tests/referencias/roteiros/qemu_ui_moldura_principal.txt`).

use serde::Deserialize;

use super::gerenciador::{desenhar_glifo_restaurar, inverter, texto_a_esquerda};
use super::quadro::Quadro;
use crate::formats::fonte::Fonte;
use crate::formats::janela::Retangulo;

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO: &str = "moldura_principal.json";

/// O que um item do menu de sistema da janela principal faz (`WM_SYSCOMMAND` na `_mainRootWnd`).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComandoPrincipal {
    /// `SC_RESTORE`: no original a janela deixa de ser maximizada. No port, restaura a janela do
    /// sistema se ela estiver maximizada; senão, nada.
    Restaurar,
    /// `SC_MOVE`, `SC_SIZE`, `SC_MAXIMIZE`: acinzentados com a janela maximizada.
    Mover,
    Tamanho,
    Maximizar,
    /// `SC_MINIMIZE`: minimiza a janela do sistema.
    Minimizar,
    /// `SC_CLOSE`: `MAINWNDPROC` chama `_MenuQuit` (a pergunta do Quit).
    Fechar,
    /// `SC_TASKLIST` ("Switch To…"): abre a Task List do Windows; no port, nada.
    Alternar,
}

/// Item do menu de sistema (texto com `&` e, depois do tab, o atalho; `null` = separador).
#[derive(Clone, Debug, Deserialize)]
pub struct ItemPrincipal {
    #[serde(default)]
    pub texto: Option<String>,
    #[serde(default)]
    pub comando: Option<ComandoPrincipal>,
    /// O Windows acinzenta o item com a janela maximizada [medido em m05b e s01].
    #[serde(default = "verdadeiro")]
    pub habilitado: bool,
}

fn verdadeiro() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize)]
pub struct CoresTitulo {
    /// [fundo, texto] da faixa com a janela ativa e inativa.
    pub ativo: [u8; 2],
    pub inativo: [u8; 2],
    pub linha: u8,
    pub botao: u8,
    pub claro: u8,
    pub escuro: u8,
    pub glifo: u8,
}

/// Conteúdo de data/moldura_principal.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigMolduraPrincipal {
    /// Texto do título (o nome da janela principal).
    pub texto: String,
    /// Linhas da faixa do título, sem a linha preta de baixo (`SM_CYCAPTION` − 1).
    pub altura: i16,
    /// Lado da caixa do menu de sistema e dos botões (a linha preta que os separa fica depois).
    pub lado_botao: i16,
    /// Índices de interface.json (6 = System do Windows; 0 = reserva).
    pub fontes: Vec<usize>,
    pub cores: CoresTitulo,
    /// Deslocamento do glifo com o botão apertado (o Windows troca o bitmap) [medido em s08 e s10].
    pub glifo_apertado: [i16; 2],
    /// Itens do menu de sistema.
    pub itens: Vec<ItemPrincipal>,
}

/// Partes da barra de título.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParteTitulo {
    Caixa,
    Minimizar,
    Restaurar,
    /// A faixa com o nome.
    Faixa,
}

/// O que a barra mostra além da configuração.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EstadoTitulo {
    /// A janela principal é a ativa (sem diálogo do Windows por cima e com o foco do sistema).
    pub ativa: bool,
    /// Caixa do menu de sistema realçada (menu aberto ou caixa escolhida pelo teclado).
    pub caixa_realcada: bool,
    /// Botão apertado com o cursor em cima.
    pub apertado: Option<ParteTitulo>,
}

impl ConfigMolduraPrincipal {
    /// Linhas ocupadas: a faixa e a linha preta.
    pub fn altura_total(&self) -> i16 {
        self.altura + 1
    }

    /// Retângulo de cada parte numa janela de `largura` pixels.
    pub fn retangulo(&self, parte: ParteTitulo, largura: i16) -> Retangulo {
        let (h, l) = (self.altura, self.lado_botao);
        match parte {
            ParteTitulo::Caixa => Retangulo { esq: 0, topo: 0, dir: l, base: h },
            ParteTitulo::Restaurar => Retangulo { esq: largura - l, topo: 0, dir: largura, base: h },
            ParteTitulo::Minimizar => Retangulo { esq: largura - 2 * l - 1, topo: 0, dir: largura - l - 1, base: h },
            ParteTitulo::Faixa => Retangulo { esq: l + 1, topo: 0, dir: largura - 2 * l - 2, base: h },
        }
    }

    /// Parte sob (x, y), se o ponto está na faixa do título (a linha preta não conta).
    pub fn parte_em(&self, largura: i16, x: i32, y: i32) -> Option<ParteTitulo> {
        if y < 0 || y >= i32::from(self.altura) || x < 0 || x >= i32::from(largura) {
            return None;
        }
        [ParteTitulo::Caixa, ParteTitulo::Minimizar, ParteTitulo::Restaurar]
            .into_iter()
            .find(|&p| super::gerenciador::contem(self.retangulo(p, largura), x, y))
            .or(Some(ParteTitulo::Faixa))
    }

    /// Desenha a barra de título nas linhas 0..`altura_total` de `q`.
    pub fn desenhar(&self, q: &mut Quadro, fonte: Option<&Fonte>, e: &EstadoTitulo) {
        let c = &self.cores;
        let largura = q.largura as i16;
        let h = self.altura;
        let [fundo, texto] = if e.ativa { c.ativo } else { c.inativo };
        q.preencher(Retangulo { esq: 0, topo: 0, dir: largura, base: h }, fundo, fundo);
        q.preencher(Retangulo { esq: 0, topo: h, dir: largura, base: h + 1 }, c.linha, c.linha);
        // Caixa do menu de sistema: o traço 13 × 3 com sombra, como o das janelas filhas [medido em m00].
        let caixa = self.retangulo(ParteTitulo::Caixa, largura);
        q.preencher(caixa, c.botao, c.botao);
        linha_v(q, caixa.dir, 0, h, c.linha);
        let (cx, cy) = (caixa.esq + 2, caixa.topo + 7);
        q.preencher(Retangulo { esq: cx + 1, topo: cy + 3, dir: cx + 14, base: cy + 4 }, c.escuro, c.escuro);
        q.preencher(Retangulo { esq: cx + 13, topo: cy + 1, dir: cx + 14, base: cy + 4 }, c.escuro, c.escuro);
        q.contorno(Retangulo { esq: cx, topo: cy, dir: cx + 12, base: cy + 2 }, c.linha);
        q.preencher(Retangulo { esq: cx + 1, topo: cy + 1, dir: cx + 12, base: cy + 2 }, c.claro, c.claro);
        if e.caixa_realcada {
            // O Windows inverte a caixa (no VGA de 16 cores, o índice vira o complementar) [m05b, s01].
            inverter(q, caixa);
        }
        // Botões de minimizar e de restaurar, com a linha preta antes de cada um.
        for p in [ParteTitulo::Minimizar, ParteTitulo::Restaurar] {
            let r = self.retangulo(p, largura);
            linha_v(q, r.esq - 1, 0, h, c.linha);
            let apertado = e.apertado == Some(p);
            botao(q, r, c, apertado);
            let (dx, dy) = if apertado { (self.glifo_apertado[0], self.glifo_apertado[1]) } else { (0, 0) };
            let g = Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy };
            if p == ParteTitulo::Restaurar {
                desenhar_glifo_restaurar(q, g, c.glifo);
            } else {
                // Seta para baixo de 7, 5, 3 e 1 pixels nas linhas 7–10 do botão, centrada na coluna 8 [m00].
                let (ax, ay) = (i32::from(g.esq) + 8, i32::from(g.topo) + 7);
                for k in 0..4 {
                    for d in -(3 - k)..=(3 - k) {
                        q.ponto(ax + d, ay + k, c.glifo);
                    }
                }
            }
        }
        // O nome, centrado na faixa entre a caixa e os botões (`DrawText` com `DT_CENTER`).
        if let Some(f) = fonte {
            let faixa = self.retangulo(ParteTitulo::Faixa, largura);
            let w = f.largura_texto(self.texto.as_bytes()) as i16;
            let x = faixa.esq + (faixa.dir - faixa.esq - w) / 2;
            let topo = (h - f.altura() as i16) / 2;
            let antes = q.recorte.replace(faixa);
            texto_a_esquerda(q, f, &self.texto, x, topo, texto, fundo);
            q.recorte = antes;
        }
    }
}

/// Cor complementar no VGA de 16 cores (o `DSTINVERT` inverte os 4 bits do índice). A caixa só tem
/// cores de 0 a 15; as outras ficam como estão.
pub fn complementar(c: u8) -> u8 {
    if c < 16 {
        15 - c
    } else {
        c
    }
}

fn linha_v(q: &mut Quadro, x: i16, y0: i16, y1: i16, cor: u8) {
    q.preencher(Retangulo { esq: x, topo: y0, dir: x + 1, base: y1 }, cor, cor);
}

/// Botão do título do Windows 3.1: em relevo (1 pixel claro em cima e à esquerda, 2 escuros embaixo e
/// à direita) ou apertado (1 pixel escuro em cima e à esquerda, sem relevo) [medido em m00, s08, s10].
fn botao(q: &mut Quadro, r: Retangulo, c: &CoresTitulo, apertado: bool) {
    q.preencher(r, c.botao, c.botao);
    let (x0, y0, x1, y1) = (i32::from(r.esq), i32::from(r.topo), i32::from(r.dir) - 1, i32::from(r.base) - 1);
    if apertado {
        for x in x0..=x1 {
            q.ponto(x, y0, c.escuro);
        }
        for y in y0..=y1 {
            q.ponto(x0, y, c.escuro);
        }
        return;
    }
    for x in x0..x1 {
        q.ponto(x, y0, c.claro);
    }
    for y in y0..y1 {
        q.ponto(x0, y, c.claro);
    }
    for k in 0..2 {
        for x in x0 + k..=x1 {
            q.ponto(x, y1 - k, c.escuro);
        }
        for y in y0 + k..=y1 {
            q.ponto(x1 - k, y, c.escuro);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cfg() -> ConfigMolduraPrincipal {
        serde_json::from_str(&crate::recursos::marcar_referencias(include_str!("../../../data/moldura_principal.json"))).unwrap()
    }

    /// Geometria medida em m00 (640 de largura): caixa 0–17 e linha em 18; minimizar 603–620,
    /// linha em 621, restaurar 622–639, linha antes do minimizar em 602.
    #[test]
    fn partes_da_barra() {
        let c = cfg();
        assert_eq!(c.altura_total(), 19);
        assert_eq!(c.retangulo(ParteTitulo::Caixa, 640), Retangulo { esq: 0, topo: 0, dir: 18, base: 18 });
        assert_eq!(c.retangulo(ParteTitulo::Minimizar, 640), Retangulo { esq: 603, topo: 0, dir: 621, base: 18 });
        assert_eq!(c.retangulo(ParteTitulo::Restaurar, 640), Retangulo { esq: 622, topo: 0, dir: 640, base: 18 });
        assert_eq!(c.parte_em(640, 5, 5), Some(ParteTitulo::Caixa));
        assert_eq!(c.parte_em(640, 611, 8), Some(ParteTitulo::Minimizar));
        assert_eq!(c.parte_em(640, 630, 17), Some(ParteTitulo::Restaurar));
        assert_eq!(c.parte_em(640, 300, 8), Some(ParteTitulo::Faixa));
        assert_eq!(c.parte_em(640, 300, 18), None, "a linha preta");
        assert_eq!(complementar(7), 8);
        assert_eq!(complementar(0), 15);
    }

    #[test]
    fn botao_apertado_sem_relevo() {
        let c = cfg();
        let mut q = Quadro::new(640, 19, 255);
        c.desenhar(&mut q, None, &EstadoTitulo { ativa: true, caixa_realcada: false, apertado: Some(ParteTitulo::Minimizar) });
        let px = |x: usize, y: usize| q.pixels[y * 640 + x];
        assert_eq!((px(603, 0), px(603, 5), px(620, 5), px(620, 17)), (c.cores.escuro, c.cores.escuro, c.cores.botao, c.cores.botao));
        // Seta deslocada (+1, +2): a linha de 7 pixels na linha 9, de 609 a 615.
        assert!((609..=615).all(|x| px(x, 9) == c.cores.glifo));
        assert_eq!(px(608, 9), c.cores.botao);
        // O de restaurar, solto: relevo.
        assert_eq!((px(622, 0), px(639, 0), px(639, 17)), (c.cores.claro, c.cores.escuro, c.cores.escuro));
    }
}
