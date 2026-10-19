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
    // Menu de um script de mod (API 0.17): só teclas que o jogo não usa chegam aqui.
    if mods_ui::tecla_abre_menu(s, ev) {
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
    // _DoKeyDown: Enter e letras vão para o anel dos códigos de trapaça do original (trapacas.md).
    if let Some(ef) = s.partida.tecla_de_trapaca(nome, *shift) {
        let falta = if ef.sem_equivalente() { " (sem equivalente no port)" } else { "" };
        println!("código de trapaça do original: {ef:?}{falta}");
        return None;
    }
    s.partida.cfg.botao_da_tecla(nome)
}

/// Entrada com um aviso modal na tela (`_PictureDialog`, `_EndGameDialog`): só o clique no
/// retângulo do aviso e (no quadro) Espaço o fecham. Devolve o aviso fechado.
pub(crate) fn tratar_modal(s: &mut Sessao, ev: &sdl2::event::Event) -> Option<Modal> {
    use sdl2::event::Event;
    use sdl2::mouse::MouseButton;
    let r = s.clique_modal?;
    if matches!(s.partida.avisos.modal(), Some(Modal::Animado(_))) {
        // _SpiderDialog/_LionDialog/_DrawSimPayoff: um clique em qualquer lugar ou uma tecla
        // (_win_Events, _DialogAbortOrCont) encerra a espera da vez.
        let pula = matches!(ev, Event::MouseButtonDown { .. } | Event::KeyDown { .. });
        return if pula { s.partida.pular_modal() } else { None };
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
        s.partida.fechar_modal()
    } else {
        None
    }
}

/// Partida com janela. Preferências do jogador (o `[SimAnt]` do `WIN.INI` do original,
/// simant/src/preferencias.rs): lidas ao abrir — as opções só fora de um `.ANT` (`de_save`), que
/// traz as suas; os botões configuráveis sempre — e gravadas ao sair com o que estiver valendo.
pub(crate) fn executar_partida(mut s: Sessao, titulo: &str, o: &Opcoes, jogo_novo: bool, de_save: bool) -> Result<(), String> {
    let prefs = o.preferencias.as_deref().map(preferencias::Preferencias::ler);
    if let Some(p) = &prefs {
        if !de_save {
            p.aplicar_opcoes(&mut s.partida.estado.partida.opcoes);
        }
        p.aplicar_botoes(&mut s.partida.funcoes_botoes);
        p.aplicar_botoes_quintal(&mut s.partida.funcoes_botoes_quintal);
    }
    let r = laco_partida(&mut s, titulo, o, jogo_novo);
    if let Some(caminho) = &o.preferencias {
        let p = preferencias::Preferencias::do_momento(&s.partida.estado.partida.opcoes, &s.partida.funcoes_botoes, &s.partida.funcoes_botoes_quintal);
        match p.gravar(caminho) {
            Ok(()) => println!("preferências gravadas em {}", caminho.display()),
            Err(e) => eprintln!("preferências: {e}"),
        }
    }
    r
}

fn laco_partida(s: &mut Sessao, titulo: &str, o: &Opcoes, jogo_novo: bool) -> Result<(), String> {
    use sdl2::event::{Event, WindowEvent};
    use sdl2::keyboard::{Keycode, Scancode};
    use sdl2::mouse::MouseButton;
    use sdl2::pixels::PixelFormatEnum;

    // A janela tem a barra de menus em cima da área cliente (simant/src/menus_ui.rs).
    let (lw, lh) = (s.tela.cfg.largura, s.tela.cfg.altura + s.menus.altura() as u32);
    let Janela { sdl, mut canvas } = abrir_janela(titulo, lw, lh, &o.video)?;
    let criador = canvas.texture_creator();
    let mut textura = criador.create_texture_static(PixelFormatEnum::RGB24, lw, lh).map_err(|e| e.to_string())?;
    let mut eventos = sdl.event_pump()?;
    let mut cursores = ajuda_ui::CursoresSdl::new(s);
    let mut audio = som::Audio::abrir(&sdl, s.recursos, &o.som);
    if jogo_novo {
        audio.musica_jogo_novo(true);
    }
    let mut repeticao = Repeticao::default();
    let mut redesenhar = true;
    // Posição do cursor em coordenadas lógicas (para Espaço/Insert/Delete) e a última tecla-clique.
    let mut cursor = (0, 0);
    let mut ultima_tecla_clique: Option<(BotaoTeclado, Instant)> = None;
    let mut situacao = String::new();
    let mut faixa = None;
    let mut minimizada = false;
    let mut foco: Option<bool> = None;
    // _DrawSimPayoff na tela: ao fechar, a faixa volta à da janela de edição (bancada, cenário payoff).
    let mut vitoria_na_tela = false;
    let inicio = Instant::now();
    let mut antes = inicio;
    println!("partida: {} (Shift+0 pausa, Shift+1..4 velocidade, Ctrl+G AutoTrack)", s.situacao());

    loop {
        let lote: Vec<Event> = eventos.poll_iter().collect();
        for ev in lote {
            // Barra de menus (e About/Status) primeiro; o resto segue com y na área cliente.
            let modal = s.dialogo.is_some()
                || s.partida.avisos.tem_modal()
                || s.menu.is_some()
                || s.menu_mod.is_some()
                || s.modos.escolhendo
                || s.modos.menu_aberto
                || s.exame.is_some()
                || s.mini.aberto()
                || modos::segurando(s);
            let ev = match menus_ui::evento(s, ev, modal) {
                menus_ui::Filtro::Passar(ev) => ev,
                menus_ui::Filtro::Tratada(r) => {
                    redesenhar |= r;
                    continue;
                }
                menus_ui::Filtro::Comando(c) => {
                    redesenhar = true;
                    if menus_ui::executar(s, c, o.salvar.as_deref()) == menus_ui::Depois::Sair {
                        return Ok(());
                    }
                    continue;
                }
            };
            if s.exame.is_some() || modos::segurando(s) {
                // _MagnifyMenu e _DrawCastePopUp: a janela fica enquanto o botão (ou Espaço/Insert)
                // estiver apertado; a simulação não anda.
                let solta = matches!(ev, Event::MouseButtonUp { .. } | Event::KeyUp { .. });
                if solta || matches!(ev, Event::Quit { .. }) {
                    s.exame = None;
                    modos::soltar_segurado(s);
                    redesenhar = true;
                }
                if matches!(ev, Event::Quit { .. }) {
                    return Ok(());
                }
                continue;
            }
            if !matches!(ev, Event::Quit { .. } | Event::Window { .. }) {
                if let Some(r) = mini_ui::evento(s, &ev) {
                    // Mini mapa aberto (_OpenMiniMapWin): modal, recebe todo o mouse.
                    if let Event::MouseMotion { x, y, .. } = ev {
                        cursor = (x, y);
                    }
                    redesenhar |= r;
                    continue;
                }
            }
            if matches!(ev, Event::Quit { .. }) && s.menu_mod.is_some() {
                return Ok(());
            }
            if let Some(mudou) = mods_ui::evento(s, &ev) {
                // Menu de um script de mod aberto (API 0.17): recebe toda a entrada.
                redesenhar |= mudou;
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
                match r {
                    arquivos::RespostaDialogo::Carregar(p) => match arquivos::carregar(s, &p, o) {
                        Ok(nova) => *s = nova,
                        Err(e) => eprintln!("erro no Load: {e}"),
                    },
                    arquivos::RespostaDialogo::Sair => return Ok(()),
                    arquivos::RespostaDialogo::Nada => {}
                }
                redesenhar = true;
                continue;
            }
            if s.partida.avisos.tem_modal() && !matches!(ev, Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } | Event::Window { .. }) {
                if let Event::MouseMotion { x, y, .. } = ev {
                    cursor = (x, y);
                }
                if tratar_modal(s, &ev).is_some_and(|m| m.abre_selecao_de_jogo()) {
                    // _EndGameDialog → _NewGame(0): abre o "Select a Game" (simant/src/modos.rs).
                    println!("fim de jogo: Select a Game");
                    s.modos.escolhendo = true;
                }
                redesenhar = true;
                continue;
            }
            match ev {
                // A janela de ajuda com o foco recebe as teclas; depois, _HelpKeyDown: F1 (índice),
                // Shift+F1 e Delete (modo de ajuda), Esc (sai do modo).
                Event::KeyDown { keycode: Some(k), keymod, .. }
                    if ajuda_ui::tecla_no_visor(s, &k.name()) || ajuda_ui::tecla(s, &evento_de_tecla(k, keymod)) =>
                {
                    redesenhar = true
                }
                Event::MouseWheel { y: dy, .. } if ajuda_ui::rolar_visor(s, cursor.0, cursor.1, dy) => redesenhar = true,
                Event::KeyDown { keycode: Some(Keycode::Escape), .. }
                    if s.janelas.gerente.menu.is_some() || s.janelas.gerente.arrasto.is_some() =>
                {
                    // Esc fecha o menu de sistema ou cancela o Move/Size pelo teclado.
                    if !s.tecla_de_janela(&EventoNav::Tecla { nome: "Escape".into(), ctrl: false, shift: false, alt: false }) {
                        s.janelas.gerente.arrasto = None;
                    }
                    redesenhar = true;
                }
                // Fechar a janela (SC_CLOSE/WM_QUERYENDSESSION → _MenuQuit) e Esc (extensão do port):
                // a pergunta do Quit.
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    s.abrir_pergunta_sair();
                    redesenhar = true;
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
                    if mousestate.left() && modos::mover_no_menu(s, x, y) {
                        // Menu de ferramentas do Experimental: realce segue o mouse.
                        redesenhar = true;
                        continue;
                    }
                    let nas_janelas = mousestate.left() && s.janelas.mover(s.recursos, &mut s.partida.estado, x, y).unwrap_or(false);
                    if nas_janelas {
                        // _MapAreaEvent: o arrasto na área do mapa leva a vista junto.
                        s.centralizar_pelo_mapa();
                    }
                    if nas_janelas || (mousestate.left() && modos::mover(s, x, y, ctrl_apertado(&eventos))) {
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
                    match modos::tecla(s, &ev) {
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
                            *s = novo_jogo(s, e, o)?;
                            audio.musica_jogo_novo(false);
                            redesenhar = true;
                            continue;
                        }
                    }
                    if let Some(b) = tratar_tecla(s, &ev, tab, o) {
                        let agora = Instant::now();
                        let duplo = ultima_tecla_clique.is_some_and(|(ub, t)| {
                            ub == b && (agora - t).as_millis() as u64 <= s.partida.cfg.duplo_clique_teclado_ms
                        });
                        ultima_tecla_clique = if duplo { None } else { Some((b, agora)) };
                        s.clicar_no_mapa(cursor.0, cursor.1, duplo);
                    }
                }
                Event::MouseButtonDown { mouse_btn: MouseButton::Left, clicks, x, y, .. }
                    if ajuda_ui::clique_no_visor(s, x, y, clicks >= 2)
                        || ajuda_ui::clique(s, x, y)
                        || (!s.modos.escolhendo && !s.modos.menu_aberto && ajuda_ui::botao_de_janela(s, x, y)) =>
                {
                    // Modo de ajuda: o clique num objeto abre a ajuda dele (_DoEvent com _bHelp).
                    cursor = (x, y);
                    redesenhar = true;
                }
                Event::MouseButtonDown { mouse_btn: MouseButton::Left, clicks, x, y, .. } => {
                    cursor = (x, y);
                    let ctrl = ctrl_apertado(&eventos);
                    // 1. "Popups" dos modos por cima de tudo (Select a Game, menu de ferramentas).
                    // 2. Janelas (gerenciador, janelas.md §8): só a da frente sob o cursor recebe.
                    // 3. Fora das janelas: faixa.
                    let popup = s.modos.escolhendo || s.modos.menu_aberto;
                    s.janelas.shift = shift_apertado(&eventos);
                    s.janelas.ctrl = ctrl;
                    let rota = if popup { janelas_ui::Roteamento::Fora } else { s.clicar_nas_janelas(x, y, clicks >= 2) };
                    let resposta = match rota {
                        _ if popup => modos::clique_popup(s, x, y),
                        janelas_ui::Roteamento::Tratado => modos::Resposta::Tratada,
                        janelas_ui::Roteamento::Linha(a) => {
                            // Seta da barra: repete enquanto o botão fica apertado.
                            repeticao.pressionar(a);
                            modos::Resposta::Tratada
                        }
                        janelas_ui::Roteamento::Quintal => {
                            modos::clique_no_quintal(s, x, y, clicks >= 2, shift_apertado(&eventos));
                            modos::Resposta::Tratada
                        }
                        janelas_ui::Roteamento::Edicao => {
                            if modos::clique_na_edicao(s, x, y, ctrl) {
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
                            if (!s.modos.janela.na_frente && s.botao_de_cheiro_em(x, y)) || mini_ui::abrir_pelo_botao(s, x, y) {
                                // Botão de cheiro ou o do mini mapa (ribToYellow, _OpenMiniMapWin).
                                modos::Resposta::Tratada
                            } else if let Some(i) = s.marcador_em(x, y).filter(|_| !s.modos.janela.na_frente) {
                                s.partida.marcador(i, ctrl);
                                modos::Resposta::Tratada
                            } else {
                                modos::clique_na_faixa(s, x, y)
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
                            *s = novo_jogo(s, e, o)?;
                            audio.musica_jogo_novo(false);
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
                    if s.modos.escolhendo || s.modos.menu_aberto || ajuda_ui::sobre_visor(s, x, y) {
                        continue;
                    }
                    // WM_RBUTTONDOWN em MAINWNDPROC (todas as janelas do jogo e a faixa): liga ou
                    // desliga o modo de ajuda (_bHelp). A janela sob o cursor vem para a frente
                    // (WM_MOUSEACTIVATE só come o esquerdo). O segundo clique de um duplo é
                    // WM_RBUTTONDBLCLK, que o jogo não trata.
                    s.clicar_direito_nas_janelas(x, y);
                    if clicks < 2 {
                        ajuda_ui::alternar_modo(s);
                    }
                    redesenhar = true;
                }
                Event::MouseButtonUp { mouse_btn: MouseButton::Left, x, y, .. } => {
                    redesenhar |= modos::soltar_no_menu(s, x, y);
                    if s.menu.is_some() {
                        s.soltar_menu(x, y, shift_apertado(&eventos));
                        redesenhar = true;
                    }
                    redesenhar |= s.soltar_nas_janelas(x, y);
                    repeticao.soltar();
                    s.janelas.soltar();
                    // Popup de definição do Info: só enquanto o botão fica apertado.
                    redesenhar |= s.info.soltar();
                    modos::soltar(s);
                }
                Event::MouseButtonUp { mouse_btn: MouseButton::Right, .. } => modos::soltar(s),
                _ => {}
            }
        }
        // WM_SETCURSOR: cursor do modo de ajuda e das ferramentas do Experimental.
        cursores.atualizar(s, cursor.0, cursor.1);
        // WM_ACTIVATEAPP: o estado da janela é lido a cada quadro (não só pelos eventos de foco, que
        // podem ser engolidos por um laço modal como o do Examine).
        let ativa = janela_ativa(canvas.window());
        if foco != Some(ativa) {
            foco = Some(ativa);
            println!("janela {}", if ativa { "ativa" } else { "inativa: som desligado" });
            s.partida.definir_foco(ativa);
            audio.foco(ativa);
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
        // Menu da barra aberto: o laço de menu do Windows não deixa o jogo andar (bancada).
        if !s.modos.escolhendo
            && !s.modos.menu_aberto
            && s.dialogo.is_none()
            && s.exame.is_none()
            && !modos::segurando(s)
            && s.menu_mod.is_none()
            && !s.menus.prende_simulacao()
        {
            s.partida.avancar(ms);
        }
        // _SpecialXfer: fechado o quadro 10058, o quintal vem para a frente na Strategic View e
        // espera o clique numa colônia preta.
        if engine::core::quintal_mapa::iniciar_escolha_de_colonia(&mut s.modos.janela, &mut s.partida) {
            s.abrir_quintal();
            redesenhar = true;
        }
        // PLACE: o laço acaba se o quintal deixa de ser a janela da frente (_win_IsWinInFront).
        if s.modos.janela.colocando_rainha && !s.janelas.aberta(Controle::Quintal) {
            engine::core::quintal_mapa::cancelar_colocacao(&mut s.modos.janela, &mut s.partida);
        }
        redesenhar |= menus_ui::passar_tempo(s);
        let pedidos = s.registrar_eventos();
        if pedidos.iter().any(|e| matches!(e, engine::rules::estado::Evento::AnimacaoVitoria)) {
            // _DrawSimPayoff: _MapToYard, _SetYardMode(0), _SetMapPlane(0) e _DrawYard antes da janela.
            modos::aplicar_acao_quintal(s, engine::core::quintal_mapa::AcaoQuintal::ModoQuintal, true);
            redesenhar = true;
        }
        audio.tratar(&pedidos, &s.partida.estado.partida.opcoes);
        let vitoria = matches!(s.partida.avisos.modal(), Some(Modal::Animado(engine::core::quadros_animados::TipoAnimado::Vitoria)));
        if vitoria_na_tela && !vitoria {
            s.modos.janela.na_frente = false;
            redesenhar = true;
        }
        vitoria_na_tela = vitoria;
        redesenhar |= s.partida.tomar_redesenho();
        // _DrawRibbonMessage: a mensagem vencida some da faixa.
        let agora_faixa = s.partida.agora();
        let nova_faixa = s.partida.avisos.texto_da_faixa(agora_faixa).cloned();
        if nova_faixa != faixa {
            faixa = nova_faixa;
            redesenhar = true;
        }
        if redesenhar {
            let cliente = s.quadro_rgb()?;
            let rgb = menus_ui::compor(s, &cliente);
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

/// A janela do port está ativa (`WM_ACTIVATEAPP` com o aplicativo ativo): tem o foco do teclado e
/// não está minimizada.
pub(crate) fn janela_ativa(w: &sdl2::video::Window) -> bool {
    w.has_input_focus() && !w.is_minimized()
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
    nova.herdar_opcoes(s, false);
    Ok(nova)
}
