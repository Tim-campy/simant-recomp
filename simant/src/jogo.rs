//! Laço da janela da partida: eventos do SDL, teclas, cliques, avisos modais e salvar.

use super::*;

/// Tecla do SDL → evento do engine (nome da tecla no SDL e modificadores, esquerdo ou direito).
pub(crate) fn evento_de_tecla(k: sdl2::keyboard::Keycode, m: sdl2::keyboard::Mod) -> EventoNav {
    use sdl2::keyboard::Mod;
    EventoNav::Tecla {
        nome: k.name(),
        ctrl: m.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
        shift: m.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
        alt: m.intersects(Mod::LALTMOD | Mod::RALTMOD),
    }
}

/// Uma tecla na tela de jogo, na ordem do original: aceleradores (Speed/Options e navegação),
/// depois `_YellowCommandKey`, depois as teclas que imitam o mouse. Devolve o botão imitado, se for
/// o caso.
pub(crate) fn tratar_tecla(s: &mut Sessao, ev: &EventoNav, tab: bool, o: &Opcoes) -> Option<BotaoTeclado> {
    if let Some(a) = s.partida.cfg.acao_da_tecla(ev) {
        if let AcaoPartida::Salvar | AcaoPartida::SalvarComo = a {
            salvar_e_avisar(s, a == AcaoPartida::SalvarComo, o);
            return None;
        }
        s.partida.aplicar(a);
        println!("{a:?}: {}", s.situacao());
        return None;
    }
    if let Some(a) = s.nav_cfg.acao_da_tecla(ev) {
        s.partida.navegar(a);
        return None;
    }
    let EventoNav::Tecla { nome, ctrl, shift, alt } = ev;
    if *alt {
        return None;
    }
    if let Some(vk) = s.partida.cfg.tecla_virtual(nome) {
        let r = s.partida.tecla_amarela(vk, Modificadores { shift: *shift, ctrl: *ctrl, tab });
        if r != RespostaTecla::Ignorada {
            println!("tecla {nome} (VK 0x{vk:02X}): {r:?}");
            return None;
        }
    }
    s.partida.cfg.botao_da_tecla(nome)
}

/// Salva e escreve o resultado ("%s Saved correctly", `_PopMsg`, no original uma caixa de mensagem).
pub(crate) fn salvar_e_avisar(s: &mut Sessao, como: bool, o: &Opcoes) {
    match s.salvar(como, o.salvar.as_deref()) {
        Ok(p) => println!("{} Saved correctly", p.display()),
        Err(e) => eprintln!("erro ao salvar: {e}"),
    }
}

/// Entrada com um aviso modal na tela (`_PictureDialog`, `_EndGameDialog`): só o clique no
/// retângulo do aviso e (no quadro) Espaço o fecham. Devolve o aviso fechado.
pub(crate) enum Fechado {
    Aviso(Modal),
    Escolha(Escolha),
}

pub(crate) fn tratar_modal(s: &mut Sessao, ev: &sdl2::event::Event) -> Option<Fechado> {
    use sdl2::event::Event;
    use sdl2::mouse::MouseButton;
    let r = s.clique_modal?;
    if let Some(Modal::SelecaoDeJogo) = s.partida.avisos.modal() {
        // _DoScenario: clique num objeto, Enter (Full Game) ou Esc (Cancel).
        let cfg = s.partida.avisos.cfg.selecao_de_jogo.clone();
        let escolha = match ev {
            Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => {
                avisos::escolha_em(s.recursos, &s.interface, &cfg, *x, *y).ok().flatten()
            }
            Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Return), .. } => Some(cfg.tecla_enter),
            Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Escape), .. } => Some(cfg.tecla_esc),
            _ => None,
        }?;
        if escolha != Escolha::Carregar {
            s.partida.fechar_modal();
        }
        return Some(Fechado::Escolha(escolha));
    }
    let tecla = s.partida.avisos.cfg.quadro.tecla_continuar.clone();
    let quadro = matches!(s.partida.avisos.modal(), Some(Modal::Quadro { .. }));
    let fecha = match ev {
        Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => {
            let dentro = *x >= r.esq as i32 && *x < r.dir as i32 && *y >= r.topo as i32 && *y < r.base as i32;
            if !dentro {
                // O original toca MessageBeep.
                println!("(bip)");
            }
            dentro
        }
        Event::KeyDown { keycode: Some(k), .. } => quadro && k.name().eq_ignore_ascii_case(&tecla),
        _ => false,
    };
    if fecha {
        s.partida.fechar_modal().map(Fechado::Aviso)
    } else {
        None
    }
}

pub(crate) fn executar_partida(mut s: Sessao, titulo: &str, o: &Opcoes, jogo_novo: bool) -> Result<(), String> {
    use sdl2::event::{Event, WindowEvent};
    use sdl2::keyboard::{Keycode, Scancode};
    use sdl2::mouse::MouseButton;
    use sdl2::pixels::PixelFormatEnum;

    let (lw, lh) = (s.tela.cfg.largura, s.tela.cfg.altura);
    let Janela { sdl, mut canvas } = abrir_janela(titulo, lw, lh, o.escala)?;
    let criador = canvas.texture_creator();
    let mut textura = criador.create_texture_static(PixelFormatEnum::RGB24, lw, lh).map_err(|e| e.to_string())?;
    let mut eventos = sdl.event_pump()?;
    let mut audio = som::Audio::abrir(&sdl, s.recursos, &o.som);
    if jogo_novo {
        audio.musica_jogo_novo();
    }
    let mut repeticao = Repeticao::default();
    let mut redesenhar = true;
    // Posição do cursor em coordenadas lógicas (para Espaço/Insert/Delete) e a última tecla-clique.
    let mut cursor = (0, 0);
    let mut ultima_tecla_clique: Option<(BotaoTeclado, Instant)> = None;
    let mut situacao = String::new();
    let mut faixa = None;
    let inicio = Instant::now();
    let mut antes = inicio;
    println!("partida: {} (Shift+0 pausa, Shift+1..4 velocidade, Ctrl+G AutoTrack)", s.situacao());

    loop {
        let lote: Vec<Event> = eventos.poll_iter().collect();
        for ev in lote {
            let selecao = matches!(s.partida.avisos.modal(), Some(Modal::SelecaoDeJogo));
            let escape = matches!(ev, Event::KeyDown { keycode: Some(Keycode::Escape), .. });
            if s.partida.avisos.tem_modal() && !matches!(ev, Event::Quit { .. } | Event::Window { .. }) && (selecao || !escape) {
                if let Event::MouseMotion { x, y, .. } = ev {
                    cursor = (x, y);
                }
                match tratar_modal(&mut s, &ev) {
                    // _EndGameDialog → _NewGame(0): o "Select a Game".
                    Some(Fechado::Aviso(Modal::FimDeJogo { .. })) => s.partida.avisos.abrir_selecao_de_jogo(),
                    Some(Fechado::Escolha(e)) => {
                        let (recursos, cores) = (s.recursos, s.cores.clone());
                        let tipo = match e {
                            // Cancel depois do fim de jogo: _CleanUp e PostQuitMessage.
                            Escolha::Cancelar => return Ok(()),
                            Escolha::Carregar => {
                                // Sem o diálogo de arquivo: recarrega o arquivo da partida, se houver.
                                match s.arquivo.clone() {
                                    Some(arq) => {
                                        s.partida.fechar_modal();
                                        s = Sessao::abrir(recursos, &o.dados, Origem::Save(&arq), cores)?;
                                        println!("Load: {}", arq.display());
                                    }
                                    None => println!("Load: o port não tem o diálogo de arquivo; use --save"),
                                }
                                redesenhar = true;
                                continue;
                            }
                            Escolha::Tutorial => TipoJogo::Tutorial,
                            Escolha::Rapido => TipoJogo::Rapido,
                            Escolha::Completo => TipoJogo::Completo,
                            Escolha::Experimental => TipoJogo::Experimental,
                        };
                        let n = semente(o);
                        println!("novo jogo {tipo:?} com semente {n}");
                        s = Sessao::abrir(recursos, &o.dados, Origem::Novo(n, tipo), cores)?;
                        audio.musica_jogo_novo();
                    }
                    _ => {}
                }
                redesenhar = true;
                continue;
            }
            match ev {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return Ok(()),
                Event::Window { win_event: WindowEvent::FocusGained, .. } => {
                    s.partida.definir_foco(true);
                    audio.foco(true);
                }
                Event::Window { win_event: WindowEvent::FocusLost, .. } => {
                    s.partida.definir_foco(false);
                    audio.foco(false);
                }
                Event::Window { win_event: WindowEvent::Exposed, .. } => redesenhar = true,
                Event::MouseMotion { x, y, .. } => cursor = (x, y),
                Event::KeyDown { keycode: Some(k), keymod, .. } => {
                    let tab = eventos.keyboard_state().is_scancode_pressed(Scancode::Tab);
                    let ev = evento_de_tecla(k, keymod);
                    if let Some(b) = tratar_tecla(&mut s, &ev, tab, o) {
                        let agora = Instant::now();
                        let duplo = ultima_tecla_clique.is_some_and(|(ub, t)| {
                            ub == b && (agora - t).as_millis() as u64 <= s.partida.cfg.duplo_clique_teclado_ms
                        });
                        ultima_tecla_clique = if duplo { None } else { Some((b, agora)) };
                        s.clicar_no_mapa(cursor.0, cursor.1, duplo);
                    }
                }
                Event::MouseButtonDown { mouse_btn: MouseButton::Left, clicks, x, y, .. } => {
                    cursor = (x, y);
                    let ctrl = sdl.keyboard().mod_state().intersects(sdl2::keyboard::Mod::LCTRLMOD | sdl2::keyboard::Mod::RCTRLMOD);
                    if let Some(i) = s.marcador_em(x, y) {
                        s.partida.marcador(i, ctrl);
                    } else if let Some(f) = s.botao_usuario_em(x, y) {
                        match s.botao_usuario(f) {
                            RespostaBotao::Feito => println!("botão {f}: {}", s.situacao()),
                            RespostaBotao::Salvar => salvar_e_avisar(&mut s, false, o),
                            RespostaBotao::NaoImplementado(n) => println!("botão {n}: janela ainda não implementada"),
                        }
                    } else if let Some(a) = s.acao_do_clique(x, y) {
                        s.partida.navegar(a);
                        repeticao.pressionar(a);
                    } else {
                        s.clicar_no_mapa(x, y, clicks >= 2);
                    }
                }
                Event::MouseButtonDown { mouse_btn: MouseButton::Right, clicks, x, y, .. } => {
                    // _processEdit: clique direito simples = simples; duplo direito = duplo.
                    s.clicar_no_mapa(x, y, clicks >= 2);
                }
                Event::MouseButtonUp { mouse_btn: MouseButton::Left, .. } => repeticao.soltar(),
                _ => {}
            }
        }
        let agora = Instant::now();
        let ms = (agora - antes).as_millis() as u64;
        antes = agora;
        if let Some((a, n)) = repeticao.passar_tempo(ms, &s.nav_cfg.repeticao) {
            for _ in 0..n {
                s.partida.navegar(a);
            }
        }
        s.partida.avancar(ms);
        let pedidos = s.registrar_eventos();
        audio.tratar(&pedidos, &s.partida.estado.partida.opcoes);
        redesenhar |= s.partida.tomar_redesenho();
        // _DrawRibbonMessage: a mensagem vencida some da faixa.
        let agora_faixa = s.partida.agora();
        let nova_faixa = s.partida.avisos.texto_da_faixa(agora_faixa).cloned();
        if nova_faixa != faixa {
            faixa = nova_faixa;
            redesenhar = true;
        }
        if redesenhar {
            let rgb = s.quadro_rgb()?;
            textura.update(None, &rgb, lw as usize * 3).map_err(|e| e.to_string())?;
            redesenhar = false;
        }
        let nova = s.situacao();
        if nova != situacao {
            canvas.window_mut().set_title(&format!("{titulo} [{nova}]")).map_err(|e| e.to_string())?;
            situacao = nova;
        }
        canvas.clear();
        canvas.copy(&textura, None, None)?;
        if o.sair_apos_ms.is_some_and(|ms| (agora - inicio).as_millis() as u64 >= ms) {
            // Lê o quadro antes do present (depois dele o conteúdo do buffer é indefinido). Sem
            // redimensionar, a janela tem exatamente a resolução lógica × escala.
            if let Some(c) = &o.capturar_janela {
                let (w, h) = canvas.output_size()?;
                let px = canvas.read_pixels(None, PixelFormatEnum::RGB24)?;
                salvar_png(c, w, h, &px)?;
                println!("{} ({w} × {h})", c.display());
            }
            println!("saindo: GameTime = {}, {}", s.partida.estado.partida.tempo, s.situacao());
            return Ok(());
        }
        canvas.present();
    }
}
