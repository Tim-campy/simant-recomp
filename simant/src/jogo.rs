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
        let janela = match a {
            AcaoPartida::Salvar | AcaoPartida::SalvarComo => {
                s.salvar_com_dialogo(a == AcaoPartida::SalvarComo, o.salvar.as_deref());
                return None;
            }
            AcaoPartida::Carregar => {
                s.abrir_load();
                return None;
            }
            AcaoPartida::JanelaEdicao => Some(None),
            AcaoPartida::JanelaComportamento => Some(Some(Controle::Tarefas)),
            AcaoPartida::JanelaCastas => Some(Some(Controle::Castas)),
            AcaoPartida::JanelaHistorico => Some(Some(Controle::Historico)),
            AcaoPartida::JanelaInfo => {
                s.abrir_info();
                return None;
            }
            _ => None,
        };
        if let Some(j) = janela {
            s.janelas.trazer_para_frente(j);
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

/// Entrada com um aviso modal na tela (`_PictureDialog`, `_EndGameDialog`): só o clique no
/// retângulo do aviso e (no quadro) Espaço o fecham. Devolve o aviso fechado.
pub(crate) fn tratar_modal(s: &mut Sessao, ev: &sdl2::event::Event) -> Option<Modal> {
    use sdl2::event::Event;
    use sdl2::mouse::MouseButton;
    let r = s.clique_modal?;
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
        s.partida.fechar_modal()
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
    let mut minimizada = false;
    let inicio = Instant::now();
    let mut antes = inicio;
    println!("partida: {} (Shift+0 pausa, Shift+1..4 velocidade, Ctrl+G AutoTrack)", s.situacao());

    loop {
        let lote: Vec<Event> = eventos.poll_iter().collect();
        for ev in lote {
            if s.exame.is_some() {
                // _MagnifyMenu: a janela fica enquanto o botão (ou Espaço/Insert/Delete) estiver
                // apertado; o laço do original não trata mensagens (a simulação não anda).
                let solta = matches!(ev, Event::MouseButtonUp { .. } | Event::KeyUp { .. });
                if solta || matches!(ev, Event::Quit { .. }) {
                    s.exame = None;
                    redesenhar = true;
                }
                if matches!(ev, Event::Quit { .. }) {
                    return Ok(());
                }
                continue;
            }
            if s.dialogo.is_some() && !matches!(ev, Event::Window { .. }) {
                // Diálogo do Windows aberto: modal, recebe toda a entrada.
                let r = match ev {
                    Event::Quit { .. } => return Ok(()),
                    Event::MouseMotion { x, y, .. } => {
                        cursor = (x, y);
                        continue;
                    }
                    Event::MouseButtonDown { mouse_btn: MouseButton::Left, clicks, x, y, .. } => s.clicar_no_dialogo(x, y, clicks >= 2),
                    Event::KeyDown { keycode: Some(k), .. } => s.tecla_no_dialogo(&k.name()),
                    Event::TextInput { text, .. } => {
                        s.digitar_no_dialogo(&text);
                        arquivos::RespostaDialogo::Nada
                    }
                    _ => continue,
                };
                if let arquivos::RespostaDialogo::Carregar(p) = r {
                    match arquivos::carregar(&s, &p, o) {
                        Ok(nova) => s = nova,
                        Err(e) => eprintln!("erro no Load: {e}"),
                    }
                }
                redesenhar = true;
                continue;
            }
            if s.partida.avisos.tem_modal() && !matches!(ev, Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } | Event::Window { .. }) {
                if let Event::MouseMotion { x, y, .. } = ev {
                    cursor = (x, y);
                }
                if tratar_modal(&mut s, &ev).is_some_and(|m| m.abre_selecao_de_jogo()) {
                    // _EndGameDialog → _NewGame(0): abre o "Select a Game" (simant/src/modos.rs).
                    println!("fim de jogo: Select a Game");
                    s.modos.escolhendo = true;
                }
                redesenhar = true;
                continue;
            }
            match ev {
                Event::KeyDown { keycode: Some(Keycode::Escape), .. }
                    if s.janelas.gerente.menu.is_some() || s.janelas.gerente.arrasto.is_some() =>
                {
                    // Esc fecha o menu de sistema ou cancela o Move/Size pelo teclado.
                    if !s.tecla_de_janela(&EventoNav::Tecla { nome: "Escape".into(), ctrl: false, shift: false, alt: false }) {
                        s.janelas.gerente.arrasto = None;
                    }
                    redesenhar = true;
                }
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
                Event::Window { win_event: WindowEvent::Minimized, .. } => minimizada = true,
                Event::Window { win_event: WindowEvent::Restored | WindowEvent::Maximized, .. } => {
                    minimizada = false;
                    redesenhar = true;
                }
                Event::MouseMotion { x, y, mousestate, .. } => {
                    cursor = (x, y);
                    if let Some((m, _)) = &mut s.menu {
                        redesenhar |= m.mover(x, y);
                        continue;
                    }
                    // Arrasto da moldura (mover, redimensionar, polegar) e realce do menu de sistema.
                    if s.janelas.gerente.arrasto.is_some() || s.janelas.gerente.menu.is_some() {
                        redesenhar |= s.mover_nas_janelas(x, y);
                        continue;
                    }
                    let nas_janelas = mousestate.left() && s.janelas.mover(s.recursos, &mut s.partida.estado, x, y).unwrap_or(false);
                    if nas_janelas || (mousestate.left() && modos::mover(&mut s, x, y, ctrl_apertado(&eventos))) {
                        redesenhar = true;
                    }
                }
                Event::KeyDown { keycode: Some(k), keymod, .. } => {
                    let tab = eventos.keyboard_state().is_scancode_pressed(Scancode::Tab);
                    let ev = evento_de_tecla(k, keymod);
                    // Menu de sistema aberto e aceleradores de janela (Ctrl+F4, Ctrl+F6…).
                    if s.tecla_de_janela(&ev) {
                        redesenhar = true;
                        continue;
                    }
                    match modos::tecla(&mut s, &ev) {
                        modos::Resposta::Seguir => {}
                        modos::Resposta::Tratada => {
                            redesenhar = true;
                            continue;
                        }
                        modos::Resposta::NovoJogo(e) if e.tipo_jogo().is_none() => {
                            // "Load Game" do Select a Game: o File → Load.
                            s.modos.escolhendo = false;
                            s.abrir_load();
                            redesenhar = true;
                            continue;
                        }
                        modos::Resposta::NovoJogo(e) => {
                            s = novo_jogo(&s, e, o)?;
                            audio.musica_jogo_novo();
                            redesenhar = true;
                            continue;
                        }
                    }
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
                    let ctrl = ctrl_apertado(&eventos);
                    // 1. "Popups" dos modos por cima de tudo (Select a Game, menu de ferramentas).
                    // 2. Janelas (gerenciador, janelas.md §8): só a da frente sob o cursor recebe.
                    // 3. Fora das janelas: faixa.
                    let popup = s.modos.escolhendo || s.modos.menu_aberto;
                    let rota = if popup { janelas_ui::Roteamento::Fora } else { s.clicar_nas_janelas(x, y, clicks >= 2) };
                    let resposta = match rota {
                        _ if popup => modos::clique_popup(&mut s, x, y),
                        janelas_ui::Roteamento::Tratado => modos::Resposta::Tratada,
                        janelas_ui::Roteamento::Linha(a) => {
                            // Seta da barra: repete enquanto o botão fica apertado.
                            repeticao.pressionar(a);
                            modos::Resposta::Tratada
                        }
                        janelas_ui::Roteamento::Quintal => {
                            modos::clique_no_quintal(&mut s, x, y, clicks >= 2);
                            modos::Resposta::Tratada
                        }
                        janelas_ui::Roteamento::Edicao => {
                            if modos::clique_na_edicao(&mut s, x, y, ctrl) {
                                modos::Resposta::Tratada
                            } else {
                                if clicks >= 2 {
                                    s.clique_pendente = None;
                                    s.clicar_no_mapa(x, y, true);
                                } else if let Some(c) = s.tela.celula_em(x, y, &s.partida.nav) {
                                    // _processEdit: espera ~6 unidades de _TickCount e só então decide.
                                    s.partida.iniciar_espera_do_clique();
                                    s.clique_pendente = Some((Instant::now(), x, y, c));
                                }
                                modos::Resposta::Tratada
                            }
                        }
                        janelas_ui::Roteamento::Fora => {
                            if !s.modos.janela.na_frente && s.botao_de_cheiro_em(x, y) {
                                modos::Resposta::Tratada
                            } else if let Some(i) = s.marcador_em(x, y).filter(|_| !s.modos.janela.na_frente) {
                                s.partida.marcador(i, ctrl);
                                modos::Resposta::Tratada
                            } else {
                                modos::clique_na_faixa(&mut s, x, y)
                            }
                        }
                    };
                    match resposta {
                        modos::Resposta::Seguir => {}
                        modos::Resposta::Tratada => {
                            redesenhar = true;
                            continue;
                        }
                        modos::Resposta::NovoJogo(e) if e.tipo_jogo().is_none() => {
                            // "Load Game" do Select a Game: o File → Load.
                            s.modos.escolhendo = false;
                            s.abrir_load();
                            redesenhar = true;
                            continue;
                        }
                        modos::Resposta::NovoJogo(e) => {
                            s = novo_jogo(&s, e, o)?;
                            audio.musica_jogo_novo();
                            redesenhar = true;
                            continue;
                        }
                    }
                    // Faixa 34 (fora das janelas).
                    if ctrl && s.abrir_menu_de_icones(x, y) {
                        redesenhar = true;
                        continue;
                    }
                    if let Some(f) = s.botao_usuario_em(x, y) {
                        match s.botao_usuario(f) {
                            RespostaBotao::Feito => println!("botão {f}: {}", s.situacao()),
                            RespostaBotao::Salvar => s.salvar_com_dialogo(false, o.salvar.as_deref()),
                            RespostaBotao::NaoImplementado(n) => println!("botão {n}: janela ainda não implementada"),
                        }
                        redesenhar = true;
                    } else if let Some(a) = s.acao_do_clique(x, y) {
                        s.partida.navegar(a);
                        repeticao.pressionar(a);
                    }
                }
                Event::MouseButtonDown { mouse_btn: MouseButton::Right, clicks, x, y, .. } => {
                    if s.modos.escolhendo || s.modos.menu_aberto {
                        continue;
                    }
                    // WM_MOUSEACTIVATE com WM_RBUTTONDOWN: a janela vem para a frente e recebe o clique
                    // (só o esquerdo é comido, `MAINWNDPROC`); o direito só vale na área do mapa.
                    if s.clicar_direito_nas_janelas(x, y) {
                        // Experimental: _processExp recebe também o clique direito.
                        if !modos::clique_na_edicao(&mut s, x, y, ctrl_apertado(&eventos)) {
                            // _processEdit: clique direito simples = simples; duplo direito = duplo.
                            s.clicar_no_mapa(x, y, clicks >= 2);
                        }
                    }
                    redesenhar = true;
                }
                Event::MouseButtonUp { mouse_btn: MouseButton::Left, x, y, .. } => {
                    if s.menu.is_some() {
                        s.soltar_menu(x, y, shift_apertado(&eventos));
                        redesenhar = true;
                    }
                    redesenhar |= s.soltar_nas_janelas(x, y);
                    repeticao.soltar();
                    s.janelas.soltar();
                    modos::soltar(&mut s);
                }
                Event::MouseButtonUp { mouse_btn: MouseButton::Right, .. } => modos::soltar(&mut s),
                _ => {}
            }
        }
        let agora = Instant::now();
        let ms = (agora - antes).as_millis() as u64;
        antes = agora;
        // Fim da espera do clique simples (6 × 55 ms): botão ainda apertado e Shift solto = segurar.
        if let Some((t0, ..)) = s.clique_pendente {
            let espera = s.partida.cfg.espera_clique_simples_tickcount * s.partida.avisos.cfg.ms_por_tickcount;
            if (Instant::now() - t0).as_millis() as u64 >= espera {
                let segurando = eventos.mouse_state().left() && !shift_apertado(&eventos);
                s.decidir_clique_pendente(segurando);
                redesenhar = true;
            }
        }
        if let Some((a, n)) = repeticao.passar_tempo(ms, &s.nav_cfg.repeticao) {
            for _ in 0..n {
                s.partida.navegar(a);
            }
        }
        // Com o "Select a Game" aberto (Ctrl+N), a simulação fica parada (_SetPause(1) em _NewGame).
        s.atualizar_janelas_desenho(minimizada);
        if !s.modos.escolhendo && s.dialogo.is_none() && s.exame.is_none() {
            s.partida.avancar(ms);
        }
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

/// Shift apertado agora (`GetAsyncKeyState(VK_SHIFT)` de `_processEdit` e `_AntMenu`).
pub(crate) fn shift_apertado(eventos: &sdl2::EventPump) -> bool {
    use sdl2::keyboard::Scancode;
    let k = eventos.keyboard_state();
    k.is_scancode_pressed(Scancode::LShift) || k.is_scancode_pressed(Scancode::RShift)
}

/// Ctrl apertado agora (`GetAsyncKeyState(VK_CONTROL)` das ferramentas do Experimental).
pub(crate) fn ctrl_apertado(eventos: &sdl2::EventPump) -> bool {
    use sdl2::keyboard::Scancode;
    let k = eventos.keyboard_state();
    k.is_scancode_pressed(Scancode::LCtrl) || k.is_scancode_pressed(Scancode::RCtrl)
}

/// Jogo novo escolhido no "Select a Game" do Ctrl+N: nova sessão com a semente do relógio.
pub(crate) fn novo_jogo<'a>(s: &Sessao<'a>, e: engine::core::selecao::Escolha, o: &Opcoes) -> Result<Sessao<'a>, String> {
    let tipo = e.tipo_jogo().ok_or("escolha sem jogo novo")?;
    // _NewGame não ressemeia: o mundo sai do R da partida aberta (geracao-mundo.md §1; conferido no
    // vetor inseticida_morte/21, engine/tests/fim_de_jogo_interface.rs).
    println!("novo jogo {e:?} com R = {:#010x}", s.partida.estado.r.estado());
    let mut nova = Sessao::abrir(s.recursos, &o.dados, Origem::NovoNaPartida(&s.partida.estado, tipo), s.cores.clone())?;
    nova.herdar_janelas(s);
    Ok(nova)
}
