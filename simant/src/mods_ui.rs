//! Menus e painéis dos scripts de mods na tela de jogo (API 0.17; engine/src/core/menus_mods.rs).
//! Extensão do port: sem mod com `definir_menu`/`definir_painel`, nada aqui aparece nem muda.

use super::*;
use engine::core::menus_mods::{desenhar_paineis, MenuModAberto, RespostaMenuMod};

/// Área cliente abaixo da faixa (onde o menu e o painel ficam).
fn area(s: &Sessao) -> Retangulo {
    Retangulo { esq: 0, topo: s.interface.moldura.faixa.altura, dir: s.tela.cfg.largura as i16, base: s.tela.cfg.altura as i16 }
}

/// Tecla que nenhum acelerador do jogo pegou: abre o menu do mod dono dela, se houver.
pub(crate) fn tecla_abre_menu(s: &mut Sessao, ev: &EventoNav) -> bool {
    let Some(scripts) = s.partida.scripts.as_ref() else { return false };
    let Some((dono, menu)) = scripts.menus().into_iter().find(|(_, m)| m.tecla.casa(ev)) else { return false };
    let a = area(s);
    s.menu_mod = Some(MenuModAberto::abrir(&s.menus_mods_cfg, &s.interface.fontes, &dono, &menu, a));
    true
}

/// `--menu mod@TECLA[#ITEM]` (com `--exportar-tela-jogo`): abre o menu do mod como se a tecla fosse
/// apertada; com `#ITEM`, escolhe o item (a partir de 1) e fecha o menu.
pub(crate) fn menu_pela_linha_de_comando(s: &mut Sessao, arg: &str) -> Result<(), String> {
    let (tecla, item) = match arg.split_once('#') {
        Some((t, i)) => (t, Some(i.parse::<usize>().map_err(|_| format!("--menu mod@{arg}: item inválido"))?)),
        None => (arg, None),
    };
    let t = engine::scripts::TeclaMod::de_texto(tecla)?;
    let ev = EventoNav::Tecla { nome: t.nome, ctrl: t.ctrl, shift: t.shift, alt: t.alt };
    if !tecla_abre_menu(s, &ev) {
        return Err(format!("--menu mod@{tecla}: nenhum menu de mod nessa tecla"));
    }
    if let Some(i) = item {
        let dono = s.menu_mod.take().map(|m| m.dono).unwrap_or_default();
        s.partida.escolher_no_menu_de_mod(&dono, i);
        for l in s.partida.retirar_registro_scripts() {
            println!("[script] {l}");
        }
    }
    Ok(())
}

/// Avisa no console quando a tecla de um menu de mod já é de um acelerador do jogo (o jogo vence).
pub(crate) fn avisar_teclas_ocupadas(s: &Sessao) {
    let Some(scripts) = s.partida.scripts.as_ref() else { return };
    for (dono, m) in scripts.menus() {
        let t = &m.tecla;
        let ev = EventoNav::Tecla { nome: t.nome.clone(), ctrl: t.ctrl, shift: t.shift, alt: t.alt };
        let ocupada = s.partida.cfg.acao_da_tecla(&ev).is_some()
            || s.nav_cfg.acao_da_tecla(&ev).is_some()
            || s.janelas.cfg.acao_da_tecla(&t.nome, t.ctrl, t.shift, t.alt).is_some()
            || s.ajuda.cfg.tecla(&ev).is_some()
            || (!t.alt && s.menus.barra.cfg.acelerador(&t.nome, t.ctrl, t.shift).is_some());
        if ocupada {
            eprintln!("[script] [{dono}] a tecla {} do menu já é um atalho do jogo; o menu não abre por ela", t.texto());
        } else {
            println!("[script] [{dono}] menu \"{}\": tecla {}", m.titulo, t.texto());
        }
    }
}

/// Com o menu de mod aberto, ele recebe toda a entrada. Devolve se o evento foi tratado (sempre,
/// com o menu aberto) e se a tela mudou.
pub(crate) fn evento(s: &mut Sessao, ev: &sdl2::event::Event) -> Option<bool> {
    use sdl2::event::Event;
    use sdl2::mouse::MouseButton;
    let m = s.menu_mod.as_mut()?;
    let resposta = match ev {
        Event::MouseMotion { x, y, .. } => return Some(m.mover(*x, *y)),
        Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => m.clicar(*x, *y),
        Event::KeyDown { keycode: Some(k), .. } => m.tecla(&k.name()),
        _ => return Some(false),
    };
    match resposta {
        RespostaMenuMod::Nada => {}
        RespostaMenuMod::Fechar => s.menu_mod = None,
        RespostaMenuMod::Escolher(i) => {
            let dono = m.dono.clone();
            let texto = m.menu.itens.get(i - 1).cloned().unwrap_or_default();
            s.menu_mod = None;
            println!("[script] [{dono}] menu: {texto}");
            s.partida.escolher_no_menu_de_mod(&dono, i);
        }
    }
    Some(true)
}

/// Painéis (por baixo) e o menu aberto (por cima de tudo).
pub(crate) fn desenhar(s: &Sessao, q: &mut Quadro) {
    let Some(scripts) = s.partida.scripts.as_ref() else { return };
    let paineis = scripts.paineis();
    if !paineis.is_empty() {
        desenhar_paineis(q, &s.interface.fontes, &s.menus_mods_cfg, &paineis, area(s));
    }
    if let Some(m) = &s.menu_mod {
        m.desenhar(q, &s.interface.fontes, &s.menus_mods_cfg);
    }
}
