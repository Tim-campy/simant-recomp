//! Barra de menus da janela principal na interface SDL (File, Window, View, Options, Speed): a barra
//! fica em cima da área cliente (a janela do port ganha `altura_total` linhas), recebe o mouse e o
//! teclado antes do jogo, e os itens fazem o que `_ProcMenu`/`_DoMenuEntry` fazem, pelos mesmos
//! caminhos dos aceleradores. Também as janelas abertas pelos menus: About (`_AboutDialog`) e Status
//! (`_ScoreDialog`). Lógica e desenho em `engine::core::barra_menus`, `sobre` e `status`.
//!
//! Especificação: docs/spec/telas/menus.md.

use std::time::Instant;

use engine::core::barra_menus::{self, BarraMenus, ComandoMenu, ConfigBarraMenus, EstadoMenus, Resposta};
use engine::core::janelas::Controle;
use engine::core::navegacao::Acao;
use engine::core::partida::AcaoPartida;
use engine::core::quadro::Quadro;
use engine::core::quintal_mapa::AcaoQuintal;
use engine::core::sobre::Sobre;
use engine::core::telas::Interface;
use engine::formats::banco::Chave;
use engine::formats::fonte::Fonte;
use engine::formats::menu::{MenuBanco, TIPO_MENU};
use engine::recursos::Recursos;
use engine::rules::estado::Evento;
use sdl2::event::Event;

use super::Sessao;

/// Estado da barra e das janelas dos menus na sessão.
pub struct MenusUi {
    pub barra: BarraMenus,
    /// Alt apertado sem outra tecla depois (Alt sozinho liga a barra ao ser solto).
    alt_sozinho: bool,
    /// About aberto.
    pub sobre: Option<Sobre>,
    /// Status aberto.
    pub status: bool,
    relogio: Instant,
}

impl MenusUi {
    pub fn carregar(recursos: &Recursos) -> Result<MenusUi, String> {
        let cfg: ConfigBarraMenus = recursos.dados(barra_menus::ARQUIVO_CONFIG)?;
        let menu = MenuBanco::decodificar(&recursos.objeto(Chave { tipo: TIPO_MENU, id: cfg.objeto_menu })?)?;
        Ok(MenusUi { barra: BarraMenus::new(cfg, menu), alt_sozinho: false, sobre: None, status: false, relogio: Instant::now() })
    }

    /// Linhas em cima da área cliente (barra e linha preta).
    pub fn altura(&self) -> i32 {
        i32::from(self.barra.altura_total())
    }

    fn agora_ms(&self) -> u64 {
        self.relogio.elapsed().as_millis() as u64
    }

    /// A barra ou uma janela dos menus prende a simulação (menu aberto: bancada, GameTime parado com o
    /// File aberto; o About pausa pelo `_SetPause`; o Status não para).
    pub fn prende_simulacao(&self) -> bool {
        self.barra.ativa()
    }
}

fn fonte<'i>(cfg: &ConfigBarraMenus, interface: &'i Interface) -> Option<&'i Fonte> {
    cfg.fontes.iter().find_map(|&i| interface.fontes.get(i).and_then(|f| f.as_ref()))
}

/// Marcas e textos do momento (`_SetMenuEntries`).
pub fn estado_menus(s: &Sessao) -> EstadoMenus {
    let p = &s.partida.estado.partida;
    let mut opcoes = [false; 6];
    for (o, v) in opcoes.iter_mut().zip(p.opcoes.iter()) {
        *o = *v != 0;
    }
    EstadoMenus { opcoes, velocidade: p.velocidade, pausado: p.pausado != 0 }
}

/// O que o laço da janela faz depois de uma entrada.
#[derive(Debug)]
pub enum Filtro {
    /// Tratada aqui (ou por um item do menu); `bool` = redesenhar.
    Tratada(bool),
    /// Item escolhido: o laço executa (`executar`).
    Comando(ComandoMenu),
    /// Segue para o jogo, com as coordenadas já na área cliente.
    Passar(Event),
}

/// Evento com o y deslocado (coordenadas da janela → área cliente).
fn deslocar(ev: Event, dy: i32) -> Event {
    match ev {
        Event::MouseMotion { timestamp, window_id, which, mousestate, x, y, xrel, yrel } => {
            Event::MouseMotion { timestamp, window_id, which, mousestate, x, y: y - dy, xrel, yrel }
        }
        Event::MouseButtonDown { timestamp, window_id, which, mouse_btn, clicks, x, y } => {
            Event::MouseButtonDown { timestamp, window_id, which, mouse_btn, clicks, x, y: y - dy }
        }
        Event::MouseButtonUp { timestamp, window_id, which, mouse_btn, clicks, x, y } => {
            Event::MouseButtonUp { timestamp, window_id, which, mouse_btn, clicks, x, y: y - dy }
        }
        outro => outro,
    }
}

fn resposta(r: Resposta) -> Option<Filtro> {
    match r {
        Resposta::Ignorada => None,
        Resposta::Tratada(m) => Some(Filtro::Tratada(m)),
        Resposta::Comando(c) => Some(Filtro::Comando(c)),
    }
}

/// Primeira parada de cada evento da partida. `modal`: há um diálogo, aviso ou menu do jogo aberto
/// (a barra não abre por cima deles, como o Windows com a captura do mouse do jogo).
pub fn evento(s: &mut Sessao, ev: Event, modal: bool) -> Filtro {
    use sdl2::keyboard::Keycode;
    use sdl2::mouse::MouseButton;
    let h = s.menus.altura();
    // Janelas modais dos menus: About e Status recebem tudo.
    if s.menus.sobre.is_some() || s.menus.status {
        return modal_dos_menus(s, ev);
    }
    let e = estado_menus(s);
    let Some(f) = fonte(&s.menus.barra.cfg, &s.interface) else { return Filtro::Passar(deslocar(ev, h)) };
    let barra = &mut s.menus.barra;
    let livre = !modal;
    match &ev {
        Event::KeyDown { keycode: Some(k), keymod, repeat, .. } => {
            let nome = k.name();
            let alt = keymod.intersects(sdl2::keyboard::Mod::LALTMOD | sdl2::keyboard::Mod::RALTMOD);
            if matches!(*k, Keycode::LAlt | Keycode::RAlt) {
                if !repeat {
                    s.menus.alt_sozinho = livre || barra.ativa();
                }
                if barra.ativa() {
                    return Filtro::Tratada(false);
                }
                return Filtro::Passar(ev);
            }
            s.menus.alt_sozinho = false;
            let ctrl = keymod.intersects(sdl2::keyboard::Mod::LCTRLMOD | sdl2::keyboard::Mod::RCTRLMOD);
            let shift = keymod.intersects(sdl2::keyboard::Mod::LSHIFTMOD | sdl2::keyboard::Mod::RSHIFTMOD);
            if livre && !alt && !barra.ativa() {
                if let Some(c) = barra.cfg.acelerador(&nome, ctrl, shift) {
                    return Filtro::Comando(c);
                }
            }
            if livre || barra.ativa() {
                if let Some(r) = resposta(barra.tecla(&nome, alt)) {
                    return r;
                }
            }
            if barra.ativa() {
                return Filtro::Tratada(false);
            }
        }
        Event::KeyUp { keycode: Some(Keycode::LAlt | Keycode::RAlt), .. } => {
            if std::mem::take(&mut s.menus.alt_sozinho) && (livre || barra.ativa()) {
                return Filtro::Tratada(matches!(barra.alternar(), Resposta::Tratada(true)));
            }
        }
        Event::KeyUp { .. } | Event::TextInput { .. } if barra.ativa() => return Filtro::Tratada(false),
        Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => {
            s.menus.alt_sozinho = false;
            if livre || barra.ativa() {
                if let Some(r) = resposta(barra.apertar(f, &e, *x, *y)) {
                    return r;
                }
            }
            if *y < h {
                return Filtro::Tratada(false);
            }
        }
        Event::MouseButtonDown { y, .. } => {
            if barra.ativa() {
                return Filtro::Tratada(false);
            }
            if *y < h {
                return Filtro::Tratada(false);
            }
        }
        Event::MouseMotion { x, y, .. } => {
            if barra.ativa() {
                return Filtro::Tratada(barra.mover(f, &e, *x, *y));
            }
        }
        Event::MouseButtonUp { mouse_btn: MouseButton::Left, x, y, .. } => {
            if let Some(r) = resposta(barra.soltar(f, &e, *x, *y)) {
                return r;
            }
            if barra.ativa() {
                return Filtro::Tratada(false);
            }
        }
        _ => {}
    }
    Filtro::Passar(deslocar(ev, h))
}

/// Entrada com o About ou o Status aberto: clique, Espaço ou Insert (`_win_Events`); Esc também fecha o
/// Status (`_DialogAbort`). O fechamento da janela do sistema passa adiante.
fn modal_dos_menus(s: &mut Sessao, ev: Event) -> Filtro {
    use sdl2::keyboard::Keycode;
    let fecha = match &ev {
        Event::MouseButtonDown { .. } => true,
        Event::KeyDown { keycode: Some(Keycode::Space | Keycode::Insert), .. } => true,
        Event::KeyDown { keycode: Some(Keycode::Escape), .. } => s.menus.status,
        Event::Quit { .. } | Event::Window { .. } => return Filtro::Passar(ev),
        _ => false,
    };
    if !fecha {
        return Filtro::Tratada(false);
    }
    if s.menus.status {
        s.menus.status = false;
        return Filtro::Tratada(true);
    }
    let agora = s.menus.agora_ms();
    let cfg = s.menus.barra.cfg.sobre.clone();
    if let Some(sobre) = s.menus.sobre.as_mut() {
        if sobre.clicar(&cfg, agora) {
            let antes = sobre.pausado_antes;
            s.menus.sobre = None;
            // _SetPause(o de antes).
            s.partida.definir_pausa(antes);
        }
    }
    Filtro::Tratada(true)
}

/// Passa o relógio do About (a rolagem). Devolve se mudou o desenho.
pub fn passar_tempo(s: &mut Sessao) -> bool {
    let agora = s.menus.agora_ms();
    let cfg = &s.menus.barra.cfg.sobre;
    match s.menus.sobre.as_mut() {
        Some(sobre) => sobre.passar_tempo(cfg, agora),
        // O Status mostra as notas do momento: a simulação continua andando.
        None => s.menus.status,
    }
}

/// O que o laço da janela faz depois de um comando.
#[derive(Debug, PartialEq, Eq)]
pub enum Depois {
    Nada,
    /// File → Quit respondido (ou sem pergunta): sair do laço.
    Sair,
}

/// `_ProcMenu`: executa o item escolhido (ou o acelerador equivalente).
pub fn executar(s: &mut Sessao, c: ComandoMenu, destino_salvar: Option<&std::path::Path>) -> Depois {
    println!("menu: {c:?}");
    // _DoMenuEntry com o modo de ajuda ligado: a ajuda do item (WinHelp HELP_CONTEXT, o número do comando).
    if s.ajuda.modo {
        s.ajuda.modo = false;
        if let Some(n) = comando_de(s, c) {
            super::ajuda_ui::abrir_contexto(s, u32::from(n));
        }
        return Depois::Nada;
    }
    use ComandoMenu::*;
    match c {
        Sobre => {
            let pausado = s.partida.estado.partida.pausado != 0;
            s.partida.definir_pausa(true);
            let agora = s.menus.agora_ms();
            s.menus.sobre = Some(engine::core::sobre::Sobre::abrir(s.recursos, &s.interface, &s.menus.barra.cfg.sobre, agora, pausado));
        }
        Novo => {
            // _myBeginSong(0x2711) e _NewGame(0): o "Select a Game" (simant/src/modos.rs).
            if let Some(id) = s.menus.barra.cfg.musica_novo {
                s.partida.estado.eventos.push(Evento::Musica(id));
                s.partida.processar_eventos_do_estado();
            }
            s.modos.escolhendo = true;
        }
        Carregar => s.abrir_load(),
        Salvar => s.salvar_com_dialogo(false, destino_salvar),
        SalvarComo => s.salvar_com_dialogo(true, destino_salvar),
        Sair => s.abrir_pergunta_sair(),
        Edicao => s.janelas.trazer_para_frente(None),
        Mapa => {
            // _OpenMapYard: sem o quintal aberto, a janela do mapa; com ele, o quintal (_MapToYard).
            if s.janelas.aberta(Controle::Quintal) {
                super::modos::aplicar_acao_quintal(s, AcaoQuintal::Abrir, true);
            } else {
                s.abrir_mapa();
            }
        }
        Comportamento => s.janelas.trazer_para_frente(Some(Controle::Tarefas)),
        Castas => s.janelas.trazer_para_frente(Some(Controle::Castas)),
        Historico => s.janelas.trazer_para_frente(Some(Controle::Historico)),
        Info => s.abrir_info(),
        Status => s.menus.status = true,
        Ajuda => {
            let c = s.ajuda.cfg.contexto_indice;
            super::ajuda_ui::abrir_contexto(s, c);
        }
        Quintal => super::modos::aplicar_acao_quintal(s, AcaoQuintal::ModoQuintal, true),
        Casa => super::modos::aplicar_acao_quintal(s, AcaoQuintal::ModoCasa, true),
        Estrategico => super::modos::aplicar_acao_quintal(s, AcaoQuintal::ModoEstrategico, true),
        Grafico => super::modos::aplicar_acao_quintal(s, AcaoQuintal::ModoGrafico, true),
        Superficie | NinhoPreto | NinhoVermelho => {
            let a = match c {
                Superficie => Acao::Superficie,
                NinhoPreto => Acao::NinhoPreto,
                _ => Acao::NinhoVermelho,
            };
            s.partida.navegar(a);
            // _MakeEditOpen: a janela de edição aberta.
            if !s.janelas.gerente.aberta(engine::core::janelas::ID_EDICAO) {
                s.janelas.trazer_para_frente(None);
            }
        }
        AutoTrack => s.partida.aplicar(AcaoPartida::AutoTrack),
        Musica | Efeitos | Eventos | Mensagens | Silly => {
            let k = c.opcao().unwrap_or(0);
            let o = &mut s.partida.estado.partida.opcoes[k];
            *o = i16::from(*o == 0);
        }
        Pausa => s.partida.aplicar(AcaoPartida::Pausa),
        Lenta => s.partida.aplicar(AcaoPartida::VelocidadeLenta),
        Normal => s.partida.aplicar(AcaoPartida::VelocidadeNormal),
        Rapida => s.partida.aplicar(AcaoPartida::VelocidadeRapida),
        Ultra => s.partida.aplicar(AcaoPartida::VelocidadeUltra),
    }
    Depois::Nada
}

/// Número do comando (`0xFDxx`) de um item.
fn comando_de(s: &Sessao, c: ComandoMenu) -> Option<u16> {
    let b = &s.menus.barra;
    (0..b.menu.titulos.len())
        .flat_map(|m| (0..b.menu.itens[m].len()).map(move |i| engine::formats::menu::comando(m, i)))
        .find(|&n| b.cfg.comando(n) == Some(c))
}

/// Desenha o About ou o Status na área cliente (antes dos diálogos do Windows).
pub fn desenhar_no_cliente(s: &Sessao, q: &mut Quadro) -> Vec<String> {
    let cfg = &s.menus.barra.cfg;
    let r = if let Some(sobre) = &s.menus.sobre {
        sobre.desenhar(q, s.recursos, &s.interface, &cfg.sobre)
    } else if s.menus.status {
        let t = engine::core::status::textos(s.recursos, &cfg.status, &s.partida.avisos.cfg.pontuacao, &s.partida.estado);
        engine::core::status::desenhar(q, s.recursos, &s.interface, &cfg.status, &t)
    } else {
        return Vec::new();
    };
    r.unwrap_or_else(|e| vec![e])
}

/// A janela inteira (barra + área cliente) em RGB: `cliente` é o quadro da área cliente já convertido.
pub fn compor(s: &Sessao, cliente: &[u8]) -> Vec<u8> {
    let h = s.menus.altura() as usize;
    let (w, hc) = (s.tela.cfg.largura as usize, s.tela.cfg.altura as usize);
    // Índice 255 = transparente (mostra a área cliente).
    let mut q = Quadro::new(w, h + hc, 255);
    let e = estado_menus(s);
    s.menus.barra.desenhar(&mut q, fonte(&s.menus.barra.cfg, &s.interface), &e);
    let mut rgb = Vec::with_capacity(w * (h + hc) * 3);
    for (i, &c) in q.pixels.iter().enumerate() {
        let (y, x) = (i / w, i % w);
        if c == 255 && y >= h {
            let k = ((y - h) * w + x) * 3;
            rgb.extend_from_slice(&cliente[k..k + 3]);
        } else {
            rgb.extend_from_slice(&s.cores.get(c as usize).copied().unwrap_or([255, 0, 255]));
        }
    }
    rgb
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::{ConfigJogo, Origem};
    use engine::paleta::Paleta;
    use engine::rules::geracao::TipoJogo;
    use engine::rules::rng::GeradorR;
    use std::path::PathBuf;
    use engine::core::barra_menus::Estado;

    fn raiz() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    fn com_sessao(f: impl FnOnce(&mut Sessao)) {
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SHARED.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
            return;
        }
        let dados = raiz().join("data");
        let cfg: ConfigJogo = serde_json::from_str(&std::fs::read_to_string(dados.join("jogo.json")).unwrap()).unwrap();
        let recursos = Recursos::abrir(&jogo, &cfg.bancos, &dados, None).unwrap();
        let cores = recursos.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
        let save = raiz().join("tests/referencias/saves/QUICK2.ANT");
        let mut s = Sessao::abrir(&recursos, &dados, Origem::Save(&save, GeradorR::default(), TipoJogo::Rapido), cores).unwrap();
        f(&mut s);
    }

    fn tecla(k: sdl2::keyboard::Keycode, m: sdl2::keyboard::Mod) -> Event {
        Event::KeyDown { timestamp: 0, window_id: 0, keycode: Some(k), scancode: None, keymod: m, repeat: false }
    }

    fn clique(x: i32, y: i32) -> Event {
        Event::MouseButtonDown { timestamp: 0, window_id: 0, which: 0, mouse_btn: sdl2::mouse::MouseButton::Left, clicks: 1, x, y }
    }

    fn soltar(x: i32, y: i32) -> Event {
        Event::MouseButtonUp { timestamp: 0, window_id: 0, which: 0, mouse_btn: sdl2::mouse::MouseButton::Left, clicks: 1, x, y }
    }

    /// Itens pelo mouse e pelo teclado, como na bancada (m01–m11, n05): o mouse apertado no título,
    /// arrastado até o item e solto escolhe; Alt+O e a letra sublinhada também.
    #[test]
    fn itens_pelo_mouse_e_pelo_teclado() {
        use sdl2::keyboard::{Keycode, Mod};
        com_sessao(|s| {
            let silly = s.partida.estado.partida.opcoes[5];
            // Options (título em x 154–219) → Silly (6.º item: topo 18 + 1 + 5 × 18).
            assert!(matches!(evento(s, clique(170, 5), false), Filtro::Tratada(true)));
            assert!(s.menus.prende_simulacao());
            let y = 18 + 1 + 5 * 18 + 5;
            assert!(matches!(evento(s, Event::MouseMotion { timestamp: 0, window_id: 0, which: 0, mousestate: sdl2::mouse::MouseState::from_sdl_state(1), x: 170, y, xrel: 0, yrel: 0 }, false), Filtro::Tratada(true)));
            match evento(s, soltar(170, y), false) {
                Filtro::Comando(c) => assert_eq!(executar(s, c, None), Depois::Nada),
                f => panic!("{f:?}"),
            }
            assert_eq!(s.partida.estado.partida.opcoes[5], i16::from(silly == 0));
            assert_eq!(s.menus.barra.estado, Estado::Fechado);
            // Alt+S e Enter: o Pause (primeiro item) pausa, e o texto vira "Un&pause".
            let pausado = s.partida.estado.partida.pausado;
            evento(s, tecla(Keycode::S, Mod::LALTMOD), false);
            assert_eq!(s.menus.barra.estado, Estado::Aberto { menu: 4, item: Some(0) });
            match evento(s, tecla(Keycode::Return, Mod::NOMOD), false) {
                Filtro::Comando(c) => {
                    executar(s, c, None);
                }
                f => panic!("{f:?}"),
            }
            assert_ne!(s.partida.estado.partida.pausado, pausado);
            let e = estado_menus(s);
            assert_eq!(s.menus.barra.texto_item(4, 0, &e), if e.pausado { "Un&pause\tShift+0" } else { "&Pause\tShift+0" });
            // Fora da barra, o clique vai para o jogo com o y da área cliente.
            match evento(s, clique(300, 200), false) {
                Filtro::Passar(Event::MouseButtonDown { y, .. }) => assert_eq!(y, 200 - s.menus.altura()),
                f => panic!("{f:?}"),
            }
        });
    }

    /// Ctrl+T abre o Status (a simulação continua); clique fecha. About pausa e devolve a pausa.
    /// Ctrl+Q pergunta; No sai.
    #[test]
    fn status_about_e_quit() {
        use sdl2::keyboard::{Keycode, Mod};
        com_sessao(|s| {
            match evento(s, tecla(Keycode::T, Mod::LCTRLMOD), false) {
                Filtro::Comando(c) => {
                    executar(s, c, None);
                }
                f => panic!("{f:?}"),
            }
            assert!(s.menus.status && !s.menus.prende_simulacao());
            s.quadro_rgb().unwrap();
            assert!(matches!(evento(s, clique(10, 300), false), Filtro::Tratada(true)));
            assert!(!s.menus.status);
            s.partida.definir_pausa(false);
            executar(s, ComandoMenu::Sobre, None);
            assert_eq!(s.partida.estado.partida.pausado, 1);
            s.quadro_rgb().unwrap();
            // O primeiro clique só começa a rolagem; depois de alguns passos, fecha e despausa.
            evento(s, clique(300, 300), false);
            assert!(s.menus.sobre.is_some());
            for _ in 0..5 {
                let cfg = s.menus.barra.cfg.sobre.clone();
                let sobre = s.menus.sobre.as_mut().unwrap();
                sobre.passos += 1;
                sobre.passar_tempo(&cfg, 0);
            }
            evento(s, clique(300, 300), false);
            assert!(s.menus.sobre.is_none());
            assert_eq!(s.partida.estado.partida.pausado, 0);
            // Quit: a pergunta; No = sair.
            match evento(s, tecla(Keycode::Q, Mod::LCTRLMOD), false) {
                Filtro::Comando(c) => {
                    executar(s, c, None);
                }
                f => panic!("{f:?}"),
            }
            assert!(matches!(s.dialogo, Some(crate::arquivos::Dialogo::PerguntaSair(_))));
            assert_eq!(s.tecla_no_dialogo("N"), crate::arquivos::RespostaDialogo::Sair);
        });
    }

    /// "Load Game" do "Select a Game" na partida abre o File → Load (a pergunta, com a partida mudada).
    #[test]
    fn load_game_do_select_a_game() {
        com_sessao(|s| {
            s.modos.escolhendo = true;
            match crate::modos::escolher(s, engine::core::selecao::Escolha::Carregar) {
                crate::modos::Resposta::NovoJogo(e) => assert_eq!(e, engine::core::selecao::Escolha::Carregar),
                _ => panic!("Load Game deve chegar ao laço da janela"),
            }
            assert!(!s.modos.escolhendo);
        });
    }
}
