//! A barra de título do Windows 3.1 como moldura de verdade da janela do port (extensão do port,
//! escolha do usuário; docs/decisoes.md, 2026-10-19, "Moldura do Windows 3.1 no lugar da do
//! sistema"). Com `data/video.json` → `moldura_windows31` e `substituir_moldura_do_sistema`, a
//! janela SDL fica sem a borda do sistema (`SDL_WINDOW_BORDERLESS`) e um `SDL_SetWindowHitTest` diz
//! ao sistema onde fica a faixa do título (arrastar move a janela; duplo clique maximiza ou
//! restaura) e, com a janela restaurada, a moldura grossa do Windows 3.1 em volta (redimensiona).
//! Os botões e o menu de sistema desenhados continuam com o jogo (menus_ui.rs), que os manda para
//! a janela do sistema (jogo.rs).
//!
//! A ampliação fica aqui, não na tela lógica do SDL: maximizada, a janela sem borda do Windows
//! passa da área útil do monitor pela espessura da moldura invisível (como toda janela maximizada
//! com `WS_THICKFRAME`), e o jogo tem de caber na parte visível. A escala é inteira (com tarjas) ou
//! a maior que cabe, como `escala_inteira`; os eventos do mouse voltam às coordenadas da janela do
//! jogo por [`MolduraJanela::deslocar`]. A geometria e as partes vêm de
//! `engine::core::moldura_principal`.

use std::sync::Mutex;

use engine::core::moldura_principal::{ConfigMolduraPrincipal, ParteJanela};
use sdl2::event::Event;
use sdl2::rect::Rect;
use sdl2::render::{Texture, WindowCanvas};

/// Onde o jogo e a moldura ficam na janela, em pixels da saída. Restaurada, a moldura grossa fica
/// rente às bordas da janela (é a borda dela, com qualquer tamanho que o jogador der) e o jogo fica
/// centrado dentro dela, com tarjas pretas entre os dois se o tamanho não for um múltiplo exato.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Posicao {
    /// Tamanho lógico do jogo (barras e área cliente), sem a moldura.
    largura: i32,
    altura: i32,
    /// Espessura lógica da moldura (0 = maximizada).
    borda: i32,
    /// Pixels da saída por pixel lógico e canto (0, 0) do jogo na saída.
    escala: f32,
    origem: (f32, f32),
    /// Parte visível da janela na saída (a moldura fica rente a ela): canto e tamanho.
    visivel: (f32, f32, f32, f32),
    /// Pixels da saída por unidade de coordenada da janela (DPI).
    saida_por_janela: (f32, f32),
}

impl Posicao {
    /// Ponto em coordenadas da janela → pixel da saída.
    fn na_saida(&self, x: f32, y: f32) -> (f32, f32) {
        (x * self.saida_por_janela.0, y * self.saida_por_janela.1)
    }

    /// Ponto em coordenadas da janela → pixel lógico do jogo (sem a moldura).
    fn logico(&self, x: f32, y: f32) -> (i32, i32) {
        let (sx, sy) = self.na_saida(x, y);
        (((sx - self.origem.0) / self.escala).floor() as i32, ((sy - self.origem.1) / self.escala).floor() as i32)
    }

    /// Tamanho lógico da moldura: a parte visível na escala do jogo.
    fn tamanho_moldura(&self) -> (i32, i32) {
        ((self.visivel.2 / self.escala).floor() as i32, (self.visivel.3 / self.escala).floor() as i32)
    }

    /// Coordenada lógica da moldura → saída: até o meio, contada da borda de cima (ou da esquerda);
    /// depois, da de baixo (ou da direita): as duas bordas ficam rentes à janela.
    fn moldura_na_saida(&self, v: i32, total: i32, ini: f32, tam: f32) -> i32 {
        if 2 * v <= total {
            (ini + v as f32 * self.escala).round() as i32
        } else {
            (ini + tam - (total - v) as f32 * self.escala).round() as i32
        }
    }

    /// Ponto em coordenadas da janela → pixel lógico da moldura (o inverso de `moldura_na_saida`).
    fn logico_moldura(&self, x: f32, y: f32) -> (i32, i32) {
        let (sx, sy) = self.na_saida(x, y);
        let (fw, fh) = self.tamanho_moldura();
        let (x0, y0, w, h) = self.visivel;
        let eixo = |s: f32, ini: f32, tam: f32, total: i32| {
            let d = s - ini;
            if 2.0 * d <= tam {
                (d / self.escala).floor() as i32
            } else {
                total - 1 - ((ini + tam - 1.0 - s) / self.escala).floor() as i32
            }
        };
        (eixo(sx, x0, w, fw), eixo(sy, y0, h, fh))
    }

    /// Retângulo lógico da moldura → retângulo na saída.
    fn retangulo_moldura(&self, esq: i32, topo: i32, dir: i32, base: i32) -> Option<Rect> {
        let (fw, fh) = self.tamanho_moldura();
        let (x0, y0, w, h) = self.visivel;
        let (sx0, sx1) = (self.moldura_na_saida(esq, fw, x0, w), self.moldura_na_saida(dir, fw, x0, w));
        let (sy0, sy1) = (self.moldura_na_saida(topo, fh, y0, h), self.moldura_na_saida(base, fh, y0, h));
        (sx1 > sx0 && sy1 > sy0).then(|| Rect::new(sx0, sy0, (sx1 - sx0) as u32, (sy1 - sy0) as u32))
    }

    /// Retângulo do jogo na saída.
    fn retangulo_jogo(&self) -> Option<Rect> {
        let (x0, y0) = (self.origem.0.round() as i32, self.origem.1.round() as i32);
        let x1 = (self.origem.0 + self.largura as f32 * self.escala).round() as i32;
        let y1 = (self.origem.1 + self.altura as f32 * self.escala).round() as i32;
        (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, (x1 - x0) as u32, (y1 - y0) as u32))
    }

    /// O que fica sob o ponto (coordenadas da janela): a moldura grossa (rente às bordas), a faixa
    /// do título do jogo ou o resto (as tarjas entre a moldura e o jogo não fazem nada).
    fn parte(&self, cfg: &ConfigMolduraPrincipal, redimensionavel: bool, x: f32, y: f32) -> ParteJanela {
        if self.borda > 0 {
            let (fw, fh) = self.tamanho_moldura();
            let (mx, my) = self.logico_moldura(x, y);
            if let r @ ParteJanela::Redimensionar { .. } = cfg.parte_da_janela(fw, fh, self.borda, redimensionavel, mx, my) {
                return r;
            }
            let (esq, dir, topo, base) = (mx < self.borda, mx >= fw - self.borda, my < self.borda, my >= fh - self.borda);
            if esq || dir || topo || base {
                return ParteJanela::Normal;
            }
        }
        let (gx, gy) = self.logico(x, y);
        cfg.parte_da_janela(self.largura, self.altura, 0, redimensionavel, gx, gy)
    }
}

/// O que o teste de clique (chamado pelo SDL no meio do tratamento de mensagens) precisa saber.
struct Geometria {
    cfg: ConfigMolduraPrincipal,
    redimensionavel: bool,
    posicao: Posicao,
}

/// `SDL_HitTest`: o ponto em coordenadas da janela vira lógico e a parte, um `SDL_HitTestResult`.
unsafe extern "C" fn teste_de_clique(
    _janela: *mut sdl2::sys::SDL_Window,
    ponto: *const sdl2::sys::SDL_Point,
    dados: *mut std::ffi::c_void,
) -> sdl2::sys::SDL_HitTestResult {
    use sdl2::sys::SDL_HitTestResult as R;
    // SAFETY: `dados` é o `Mutex<Geometria>` vazado em `MolduraJanela::ligar` (vive até o fim do
    // programa) e `ponto` vem do SDL, válido durante a chamada.
    let (g, p) = unsafe { (&*(dados as *const Mutex<Geometria>), *ponto) };
    let Ok(g) = g.lock() else { return R::SDL_HITTEST_NORMAL };
    match g.posicao.parte(&g.cfg, g.redimensionavel, p.x as f32, p.y as f32) {
        ParteJanela::Normal => R::SDL_HITTEST_NORMAL,
        ParteJanela::Arrastar => R::SDL_HITTEST_DRAGGABLE,
        ParteJanela::Redimensionar { esq, dir, topo, base } => match (esq, dir, topo, base) {
            (true, _, true, _) => R::SDL_HITTEST_RESIZE_TOPLEFT,
            (_, true, true, _) => R::SDL_HITTEST_RESIZE_TOPRIGHT,
            (true, _, _, true) => R::SDL_HITTEST_RESIZE_BOTTOMLEFT,
            (_, true, _, true) => R::SDL_HITTEST_RESIZE_BOTTOMRIGHT,
            (true, ..) => R::SDL_HITTEST_RESIZE_LEFT,
            (_, true, ..) => R::SDL_HITTEST_RESIZE_RIGHT,
            (_, _, true, _) => R::SDL_HITTEST_RESIZE_TOP,
            _ => R::SDL_HITTEST_RESIZE_BOTTOM,
        },
    }
}

/// A moldura da janela do port: desligada (a do sistema) ou a do Windows 3.1 no lugar dela.
pub struct MolduraJanela {
    /// Geometria compartilhada com o teste de clique (`None` = a janela tem a borda do sistema).
    geometria: Option<&'static Mutex<Geometria>>,
    cfg: Option<ConfigMolduraPrincipal>,
    /// Tamanho lógico da janela do jogo (barras e área cliente), sem a moldura.
    largura: u32,
    altura: u32,
    escala_inteira: bool,
    posicao: Posicao,
    /// Cores RGB da paleta (a moldura é desenhada com o SDL).
    cores: Vec<[u8; 3]>,
}

impl MolduraJanela {
    /// A janela fica com a borda do sistema.
    pub fn desligada() -> MolduraJanela {
        let posicao = Posicao { largura: 0, altura: 0, borda: 0, escala: 1.0, origem: (0.0, 0.0), visivel: (0.0, 0.0, 1.0, 1.0), saida_por_janela: (1.0, 1.0) };
        MolduraJanela { geometria: None, cfg: None, largura: 0, altura: 0, escala_inteira: true, posicao, cores: Vec::new() }
    }

    /// Pixels de moldura de cada lado na janela restaurada (o tamanho inicial da janela os inclui).
    pub fn espessura(cfg: Option<&ConfigMolduraPrincipal>) -> u32 {
        cfg.map_or(0, |c| c.borda.espessura.max(0) as u32)
    }

    /// Liga a moldura do Windows 3.1 na janela (já criada sem borda) com o jogo de `largura` ×
    /// `altura` pixels lógicos. Se o sistema não aceitar o teste de clique, a janela volta a ter
    /// borda (o jogo continua com a barra desenhada, como sem esta opção).
    pub fn ligar(
        canvas: &mut WindowCanvas,
        cfg: &ConfigMolduraPrincipal,
        largura: u32,
        altura: u32,
        redimensionavel: bool,
        escala_inteira: bool,
        cores: &[[u8; 3]],
    ) -> MolduraJanela {
        let mut m = MolduraJanela::desligada();
        let g = Geometria { cfg: cfg.clone(), redimensionavel, posicao: m.posicao };
        // Vazado de propósito: o SDL pode chamar o teste até a janela fechar, e são poucos bytes
        // por janela aberta (a abertura e a partida).
        let g: &'static Mutex<Geometria> = Box::leak(Box::new(Mutex::new(g)));
        // SAFETY: a janela é válida (é a do canvas) e `g` vive até o fim do programa.
        let r = unsafe {
            sdl2::sys::SDL_SetWindowHitTest(canvas.window().raw(), Some(teste_de_clique), g as *const Mutex<Geometria> as *mut std::ffi::c_void)
        };
        if r != 0 {
            eprintln!("{}", engine::tr!("jogo.moldura.sem_teste_de_clique", erro = sdl2::get_error()));
            canvas.window_mut().set_bordered(true);
            return m;
        }
        // A ampliação passa a ser feita aqui (sem a tela lógica do SDL; ver o começo do arquivo).
        let _ = canvas.set_logical_size(0, 0);
        m.geometria = Some(g);
        m.cfg = Some(cfg.clone());
        m.largura = largura;
        m.altura = altura;
        m.escala_inteira = escala_inteira;
        m.cores = cores.to_vec();
        m.posicao.borda = -1;
        m.atualizar(canvas);
        m
    }

    /// A moldura do Windows 3.1 substitui a do sistema.
    pub fn ligada(&self) -> bool {
        self.geometria.is_some()
    }

    /// A janela do sistema não está maximizada (o título mostra o botão de maximizar).
    pub fn restaurada(&self) -> bool {
        self.ligada() && self.posicao.borda > 0
    }

    /// A cada quadro: com a janela maximizada, sem moldura e só na área útil do monitor; restaurada,
    /// com a moldura grossa. Recalcula escala e posição e as passa ao teste de clique.
    pub fn atualizar(&mut self, canvas: &mut WindowCanvas) {
        let (Some(g), Some(cfg)) = (self.geometria, &self.cfg) else { return };
        let maximizada = canvas.window().is_maximized();
        let borda = if maximizada { 0 } else { i32::from(cfg.borda.espessura.max(0)) };
        let (lw, lh) = (self.largura as i32 + 2 * borda, self.altura as i32 + 2 * borda);
        if borda != self.posicao.borda {
            let _ = canvas.window_mut().set_minimum_size(lw as u32, lh as u32);
        }
        let janela = canvas.window();
        let (ow, oh) = canvas.output_size().unwrap_or((lw as u32, lh as u32));
        let (ww, wh) = janela.size();
        let saida_por_janela = (ow as f32 / ww.max(1) as f32, oh as f32 / wh.max(1) as f32);
        // Parte visível da janela, em coordenadas da janela: maximizada, a interseção com a área útil
        // do monitor (o Windows põe a janela maximizada além dela pela moldura invisível).
        let (mut vx0, mut vy0, mut vx1, mut vy1) = (0, 0, ww as i32, wh as i32);
        if maximizada {
            let (x, y) = janela.position();
            let util = janela.display_index().and_then(|d| janela.subsystem().display_usable_bounds(d));
            if let Ok(u) = util {
                vx0 = vx0.max(u.x() - x);
                vy0 = vy0.max(u.y() - y);
                vx1 = vx1.min(u.x() + u.width() as i32 - x);
                vy1 = vy1.min(u.y() + u.height() as i32 - y);
            }
        }
        let (vx0, vy0) = (vx0 as f32 * saida_por_janela.0, vy0 as f32 * saida_por_janela.1);
        let (vw, vh) = ((vx1 as f32 * saida_por_janela.0 - vx0).max(1.0), (vy1 as f32 * saida_por_janela.1 - vy0).max(1.0));
        let mut escala = (vw / lw as f32).min(vh / lh as f32);
        if self.escala_inteira && escala >= 1.0 {
            escala = escala.floor();
        }
        // O jogo fica centrado dentro da moldura (que fica rente às bordas da parte visível).
        let b = borda as f32 * escala;
        let (iw, ih) = (vw - 2.0 * b, vh - 2.0 * b);
        let (jw, jh) = (self.largura as f32 * escala, self.altura as f32 * escala);
        let origem = ((vx0 + b + (iw - jw) / 2.0).floor(), (vy0 + b + (ih - jh) / 2.0).floor());
        self.posicao = Posicao {
            largura: self.largura as i32,
            altura: self.altura as i32,
            borda,
            escala,
            origem,
            visivel: (vx0, vy0, vw, vh),
            saida_por_janela,
        };
        if let Ok(mut g) = g.lock() {
            g.posicao = self.posicao;
        }
    }

    /// Eventos do mouse em coordenadas da janela do jogo (sem a moldura nem a ampliação).
    pub fn deslocar(&self, ev: Event) -> Event {
        if !self.ligada() {
            return ev;
        }
        let p = &self.posicao;
        let ponto = |x: i32, y: i32| p.logico(x as f32, y as f32);
        match ev {
            Event::MouseMotion { timestamp, window_id, which, mousestate, x, y, xrel, yrel } => {
                let (x, y) = ponto(x, y);
                let r = |d: i32, k: f32| (d as f32 * k / p.escala).round() as i32;
                let (xrel, yrel) = (r(xrel, p.saida_por_janela.0), r(yrel, p.saida_por_janela.1));
                Event::MouseMotion { timestamp, window_id, which, mousestate, x, y, xrel, yrel }
            }
            Event::MouseButtonDown { timestamp, window_id, which, mouse_btn, clicks, x, y } => {
                let (x, y) = ponto(x, y);
                Event::MouseButtonDown { timestamp, window_id, which, mouse_btn, clicks, x, y }
            }
            Event::MouseButtonUp { timestamp, window_id, which, mouse_btn, clicks, x, y } => {
                let (x, y) = ponto(x, y);
                Event::MouseButtonUp { timestamp, window_id, which, mouse_btn, clicks, x, y }
            }
            outro => outro,
        }
    }

    /// Copia a janela do jogo (`textura`) para a tela: ampliada, dentro da moldura grossa (se a
    /// janela estiver restaurada). Sem esta moldura, como sempre (a tela lógica do SDL amplia).
    pub fn copiar(&self, canvas: &mut WindowCanvas, textura: &Texture) -> Result<(), String> {
        let Some(cfg) = self.cfg.as_ref().filter(|_| self.ligada()) else { return canvas.copy(textura, None, None) };
        let p = &self.posicao;
        if p.borda > 0 {
            let (fw, fh) = p.tamanho_moldura();
            let lim = |v: i32| v.clamp(0, i32::from(i16::MAX)) as i16;
            for (r, cor) in cfg.retangulos_da_borda(lim(fw), lim(fh)) {
                let [cr, cg, cb] = self.cores.get(cor as usize).copied().unwrap_or([255, 0, 255]);
                canvas.set_draw_color(sdl2::pixels::Color::RGB(cr, cg, cb));
                if let Some(r) = p.retangulo_moldura(i32::from(r.esq), i32::from(r.topo), i32::from(r.dir), i32::from(r.base)) {
                    canvas.fill_rect(r)?;
                }
            }
            canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
        }
        match p.retangulo_jogo() {
            Some(r) => canvas.copy(textura, None, r),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cfg() -> ConfigMolduraPrincipal {
        serde_json::from_str(&engine::recursos::marcar_referencias(include_str!("../../data/moldura_principal.json"))).unwrap()
    }

    #[test]
    fn coordenadas_da_janela_e_do_jogo() {
        // Escala 2, janela de 1316 × 996 (20 pixels a mais que 2 × 648 × 488 nos dois eixos): a
        // moldura fica rente às bordas e o jogo, centrado dentro dela (tarjas de 20 / 2 = 10).
        let p = Posicao {
            largura: 640,
            altura: 480,
            borda: 4,
            escala: 2.0,
            origem: (8.0 + 10.0, 8.0 + 10.0),
            visivel: (0.0, 0.0, 1316.0, 996.0),
            saida_por_janela: (1.0, 1.0),
        };
        assert_eq!(p.logico(18.0, 18.0), (0, 0));
        assert_eq!(p.logico(17.0, 19.0), (-1, 0));
        assert_eq!(p.retangulo_jogo(), Some(Rect::new(18, 18, 1280, 960)));
        // Moldura de 658 × 498 lógicos: a linha de fora, à esquerda e à direita, rente às bordas.
        assert_eq!(p.tamanho_moldura(), (658, 498));
        assert_eq!(p.retangulo_moldura(0, 0, 1, 498), Some(Rect::new(0, 0, 2, 996)));
        assert_eq!(p.retangulo_moldura(657, 0, 658, 498), Some(Rect::new(1314, 0, 2, 996)));
        assert_eq!(p.logico_moldura(1315.0, 995.0), (657, 497));
        assert_eq!(p.logico_moldura(0.0, 7.0), (0, 3));
        // DPI: a janela em pontos, a saída em pixels (1,25).
        let q = Posicao { saida_por_janela: (1.25, 1.25), escala: 1.25, origem: (0.0, 0.0), ..p };
        assert_eq!(q.logico(100.0, 8.0), (100, 8));
    }

    /// Restaurada com qualquer tamanho, a moldura grossa fica nas bordas da janela e redimensiona;
    /// a faixa do título (dentro, onde o jogo está) arrasta; as tarjas não fazem nada.
    #[test]
    fn partes_com_a_moldura_rente_as_bordas() {
        let c = cfg();
        let p = Posicao {
            largura: 640,
            altura: 480,
            borda: 4,
            escala: 1.0,
            origem: (4.0 + 100.0, 4.0 + 50.0),
            visivel: (0.0, 0.0, 848.0, 588.0),
            saida_por_janela: (1.0, 1.0),
        };
        let lados = |esq, dir, topo, base| ParteJanela::Redimensionar { esq, dir, topo, base };
        assert_eq!(p.parte(&c, true, 1.0, 300.0), lados(true, false, false, false));
        assert_eq!(p.parte(&c, true, 846.0, 300.0), lados(false, true, false, false));
        assert_eq!(p.parte(&c, true, 846.0, 587.0), lados(false, true, false, true));
        assert_eq!(p.parte(&c, true, 50.0, 300.0), ParteJanela::Normal, "tarja");
        assert_eq!(p.parte(&c, true, 104.0 + 300.0, 54.0 + 5.0), ParteJanela::Arrastar, "faixa do título");
        assert_eq!(p.parte(&c, true, 104.0 + 5.0, 54.0 + 5.0), ParteJanela::Normal, "caixa do menu de sistema");
        assert_eq!(p.parte(&c, false, 1.0, 300.0), ParteJanela::Normal, "sem redimensionar");
        // Maximizada: sem moldura.
        let m = Posicao { borda: 0, origem: (0.0, 0.0), visivel: (0.0, 0.0, 640.0, 480.0), ..p };
        assert_eq!(m.parte(&c, true, 300.0, 5.0), ParteJanela::Arrastar);
        assert_eq!(m.parte(&c, true, 0.0, 300.0), ParteJanela::Normal);
    }
}
