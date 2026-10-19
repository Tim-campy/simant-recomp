//! Mini mapa na interface SDL (`engine::core::mini_mapa`): o botão `ribToYellow` da faixa 34 o abre
//! (`_OpenMiniMapWin`); aberto, é modal e recebe todo o mouse; o jogo fica pausado. Especificação:
//! docs/spec/telas/mapa.md §7.

use super::*;
use engine::core::mini_mapa::AcaoMini;
use sdl2::event::Event;
use sdl2::mouse::MouseButton;

/// Botão apertado em (x, y) da faixa 34: se é o do mini mapa, abre (`_SetPause(1)`). Devolve se
/// abriu.
pub(crate) fn abrir_pelo_botao(s: &mut Sessao, x: i32, y: i32) -> bool {
    if s.modos.janela.na_frente || !s.tela.objetos_da_faixa_em(x, y).contains(&s.mini.cfg.botao.as_str()) {
        return false;
    }
    let Some(botao) = s.tela.retangulo_da_faixa(&s.mini.cfg.botao) else { return false };
    let pausado = s.partida.estado.partida.pausado != 0;
    s.mini.abrir(botao, &s.partida.estado, &s.mapa, s.partida.mostrar_trilhas, true, pausado);
    s.partida.definir_pausa(true);
    println!("mini mapa aberto (plano {})", s.partida.nav.plano_mapa);
    true
}

/// Fecha e volta à pausa de antes (`_SetPause(local_8)`).
fn fechar(s: &mut Sessao) {
    s.mini.fechar();
    let antes = s.mini.pausa_antes;
    s.partida.definir_pausa(antes);
    println!("mini mapa fechado");
}

fn aplicar(s: &mut Sessao, a: AcaoMini) -> bool {
    match a {
        AcaoMini::Nada => false,
        AcaoMini::Centralizar(cx, cy) => {
            s.centralizar_vista(cx, cy);
            true
        }
        AcaoMini::Fechar => {
            fechar(s);
            true
        }
        AcaoMini::FecharEAbrirMapa => {
            fechar(s);
            s.abrir_mapa_ou_quintal();
            true
        }
    }
}

/// Com o mini mapa aberto, o evento vai para ele (o laço modal de `_OpenMiniMapWin` só trata o
/// mouse; as teclas ficam sem efeito no port [hipótese: no original o Windows ainda despacha as
/// mensagens de teclado]). Devolve `None` com ele fechado; senão, se é preciso redesenhar.
pub(crate) fn evento(s: &mut Sessao, ev: &Event) -> Option<bool> {
    if !s.mini.aberto() {
        return None;
    }
    let a = match *ev {
        Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => s.mini.apertar(x, y),
        Event::MouseMotion { x, y, mousestate, .. } if mousestate.left() => s.mini.mover(x, y),
        Event::MouseButtonUp { mouse_btn: MouseButton::Left, x, y, .. } => s.mini.soltar(x, y),
        _ => AcaoMini::Nada,
    };
    Some(aplicar(s, a))
}

pub(crate) fn desenhar(s: &Sessao, q: &mut Quadro) {
    if s.mini.aberto() {
        s.mini.desenhar(q, &s.mapa, &s.partida.nav);
    }
}
