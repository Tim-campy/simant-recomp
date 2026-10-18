//! Executável do port: janela, entrada e desenho sobre SDL2. A lógica fica no crate `engine`; aqui só
//! se traduzem eventos do SDL em eventos do engine e se mostra o quadro composto.
//!
//! Uso:
//!   simant [--jogo DIR] [--dados DIR] [--mods DIR] [--exportar-telas DIR]
//!   simant (--save ARQ.ANT | --novo-jogo [--semente N]) [--acoes a,b,...]
//!          [--ticks N] [--botoes F,F,...] [--fechar-avisos N] [--exportar-tela-jogo ARQ.png]
//!          [--salvar ARQ.ANT]
//!   opções da janela: [--escala N] [--janela LxA] [--sair-apos-ms N [--capturar-janela ARQ.png]]
//!   (padrões em data/video.json: escala 1 = 640×480 como o original; simant/src/video.rs)
//!   som (simant/src/som): [--sem-som] [--soundfont ARQ.sf2] [--exportar-som DIR]
//!   preferências do jogador (simant/src/preferencias.rs): [--preferencias ARQ] [--sem-preferencias]
//!
//! Padrões: --jogo $SIMANT_DIR ou SimAnt-T/MAXIS/ANTWIN, --dados data, --mods mods (relativos à
//! pasta atual). Sem --save nem --novo-jogo, mostra a abertura. Com um deles, a simulação roda ao
//! vivo (engine::core::partida); com --ticks, --exportar-tela-jogo ou --salvar, roda sem janela e
//! sai (--salvar grava o .ANT do estado no fim). Na janela, Ctrl+S salva no arquivo da partida e
//! Ctrl+A ("Save As") num nome novo (o de --salvar, ou SALVO01.ANT, SALVO02.ANT… na pasta atual).

mod arquivos;
mod jogo;
mod opcoes;
mod preferencias;
mod sessao;
mod som;
mod video;

use jogo::*;
use opcoes::*;
use sessao::*;

mod janelas_ui;
mod mods_ui;
mod modos;

use std::path::{Path, PathBuf};
use std::time::Instant;

use engine::core::avisos::{self, ConfigAvisos, Modal};
use engine::core::janelas::{self, Controle, JanelasControle};
use engine::core::menus::{self, ConfigMenus, MenuAberto};
use engine::rules::amarela::RespostaClique;
use engine::core::quadro::Quadro;
use engine::core::navegacao::{Acao, ConfigNavegacao, Evento as EventoNav, Repeticao};
use engine::core::partida::{self, AcaoPartida, BotaoTeclado, ConfigPartida, Partida, RespostaBotao};
use engine::formats::janela::Retangulo;
use engine::core::quintal_mapa::AcaoQuintal;
use engine::core::tela_jogo::{ConfigTelaJogo, PosicaoVista, TelaDeJogo};
use engine::core::telas::{compor, Evento, Interface, Reprodutor, Sequencia};
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::paleta::Paleta;
use engine::recursos::Recursos;
use engine::rules::amarela::{Modificadores, RespostaTecla, TabelasAmarela};
use engine::rules::estado::{ConfigSimulacao, Estado, Evento as EventoRegras};
use engine::rules::geracao::{TabelasGeracao, TipoJogo};
use engine::rules::rng::{GeradorR, GeradorS};
use engine::rules::tick::RegrasOriginais;
use serde::Deserialize;

#[derive(Deserialize)]
struct ConfigJogo {
    bancos: Vec<String>,
    /// Id do objeto tipo 9 com as posições das janelas (docs/spec/formatos/janela.md).
    tabela_posicoes: u16,
}

fn para_rgb(indices: &[u8], cores: &[[u8; 3]]) -> Vec<u8> {
    indices.iter().flat_map(|&i| cores.get(i as usize).copied().unwrap_or([255, 0, 255])).collect()
}

fn salvar_png(caminho: &Path, largura: u32, altura: u32, rgb: &[u8]) -> Result<(), String> {
    if let Some(dir) = caminho.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let arq = std::fs::File::create(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(arq), largura, altura);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(rgb).map_err(|e| e.to_string())
}

/// Compõe a tela e avisa (uma vez por tela) o que ainda não sabemos desenhar.
fn compor_rgb(t: &engine::core::telas::Tela, recursos: &Recursos, interface: &Interface, cores: &[[u8; 3]]) -> Result<Vec<u8>, String> {
    let (quadro, pendentes) = compor(t, recursos, interface)?;
    if !pendentes.is_empty() {
        eprintln!("tela {}: não desenhado ainda: {}", t.nome, pendentes.join(", "));
    }
    Ok(para_rgb(&quadro.pixels, cores))
}

fn exportar(seq: &Sequencia, recursos: &Recursos, interface: &Interface, cores: &[[u8; 3]], dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for t in &seq.telas {
        let rgb = compor_rgb(t, recursos, interface, cores)?;
        let caminho = dir.join(format!("{}.png", t.nome));
        salvar_png(&caminho, t.largura, t.altura, &rgb)?;
        println!("{}", caminho.display());
    }
    Ok(())
}

/// Janela SDL com a resolução lógica (largura × altura) ampliada pela escala de `data/video.json`
/// ou da linha de comando (padrão 1, como o original), centralizada com tarjas quando a janela
/// do sistema não tem a proporção ou um múltiplo exato da resolução.
struct Janela {
    sdl: sdl2::Sdl,
    canvas: sdl2::render::WindowCanvas,
}

fn abrir_janela(titulo: &str, largura: u32, altura: u32, cfg: &video::ConfigVideo) -> Result<Janela, String> {
    // Nitidez: sem ciência de DPI, o Windows com escala de tela (125%, 150%) amplia a janela inteira
    // com filtro bilinear e o jogo fica borrado. Com "permonitorv2" a janela e os limites da tela vêm
    // em pixels físicos, e a ampliação é só a do renderizador: por vizinho mais próximo ("0"),
    // inteira (com tarjas) se `escala_inteira`.
    sdl2::hint::set("SDL_WINDOWS_DPI_AWARENESS", "permonitorv2");
    sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "0");
    let sdl = sdl2::init()?;
    let sistema = sdl.video()?;
    let area = sistema.display_usable_bounds(0).ok().map(|r| (r.width(), r.height()));
    let (escala, (w, h)) = cfg.tamanho_inicial(largura, altura, area);
    let mut construtor = sistema.window(titulo, w, h);
    construtor.position_centered();
    if cfg.redimensionavel {
        construtor.resizable();
    }
    let mut janela = construtor.build().map_err(|e| e.to_string())?;
    // Menor que a resolução do jogo, a escala inteira cairia para zero.
    janela.set_minimum_size(largura, altura).map_err(|e| e.to_string())?;
    let mut canvas = janela.into_canvas().present_vsync().build().map_err(|e| e.to_string())?;
    canvas.set_logical_size(largura, altura).map_err(|e| e.to_string())?;
    canvas.set_integer_scale(cfg.escala_inteira)?;
    let (pw, ph) = canvas.output_size()?;
    println!("janela: {largura}x{altura} em escala {escala} ({pw}x{ph} pixels)");
    Ok(Janela { sdl, canvas })
}

/// Abertura até o "Select a Game"; devolve a escolha feita nele (None = saiu).
fn executar(
    seq: Sequencia,
    recursos: &Recursos,
    interface: &Interface,
    cores: &[[u8; 3]],
    o: &Opcoes,
) -> Result<Option<engine::core::selecao::Escolha>, String> {
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::pixels::PixelFormatEnum;

    let (lw, lh) = seq.telas.first().map(|t| (t.largura, t.altura)).unwrap_or((640, 480));
    let Janela { sdl, mut canvas } = abrir_janela("SimAnt", lw, lh, &o.video)?;
    let criador = canvas.texture_creator();
    let mut eventos = sdl.event_pump()?;
    let selecao = modos::selecao_da_abertura(recursos, interface)?;
    let mut audio = som::Audio::abrir(&sdl, recursos, &o.som);
    audio.musica_abertura();

    let mut rep = Reprodutor::new(seq);
    let mut desenhada: Option<String> = None;
    let mut textura = None;
    let inicio = Instant::now();
    let mut antes = inicio;

    while !rep.terminou() {
        let escolhendo = rep.atual().is_some_and(|t| t.nome == selecao.cfg.tela);
        for ev in eventos.poll_iter() {
            match ev {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return Ok(None),
                Event::KeyDown { keycode: Some(k), keymod, .. } if escolhendo => {
                    if let Some(e) = selecao.cfg.escolha_da_tecla(&evento_de_tecla(k, keymod)) {
                        return Ok(Some(e));
                    }
                }
                Event::MouseButtonDown { x, y, .. } if escolhendo => {
                    if let Some(e) = selecao.escolha_em(x, y) {
                        return Ok(Some(e));
                    }
                }
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => rep.evento(Evento::Espaco),
                Event::KeyDown { .. } => rep.evento(Evento::OutraTecla),
                Event::MouseButtonDown { .. } => rep.evento(Evento::Clique),
                _ => {}
            }
        }
        // WM_ACTIVATEAPP vale também na abertura e no "Select a Game": sem foco, a música para.
        audio.foco(janela_ativa(canvas.window()));
        let agora = Instant::now();
        rep.passar_tempo((agora - antes).as_millis() as u64);
        antes = agora;
        if o.sair_apos_ms.is_some_and(|ms| (agora - inicio).as_millis() as u64 >= ms) {
            return Ok(None);
        }

        let Some(tela) = rep.atual() else { break };
        if desenhada.as_deref() != Some(tela.nome.as_str()) {
            let rgb = compor_rgb(tela, recursos, interface, cores)?;
            let mut t = criador
                .create_texture_static(PixelFormatEnum::RGB24, tela.largura, tela.altura)
                .map_err(|e| e.to_string())?;
            t.update(None, &rgb, tela.largura as usize * 3).map_err(|e| e.to_string())?;
            canvas.set_logical_size(tela.largura, tela.altura).map_err(|e| e.to_string())?;
            textura = Some(t);
            desenhada = Some(tela.nome.clone());
        }
        canvas.clear();
        if let Some(t) = &textura {
            canvas.copy(t, None, None)?;
        }
        canvas.present();
    }
    Ok(None)
}

fn principal() -> Result<(), String> {
    let o = ler_opcoes()?;
    let config: ConfigJogo = serde_json::from_str(
        &std::fs::read_to_string(o.dados.join("jogo.json")).map_err(|e| format!("data/jogo.json: {e}"))?,
    )
    .map_err(|e| format!("data/jogo.json: {e}"))?;
    let recursos = Recursos::abrir(&o.jogo, &config.bancos, &o.dados, Some(&o.mods))?;
    for m in recursos.mods() {
        println!("mod carregado: {} {}", m.manifesto.nome, m.manifesto.versao);
    }
    let cores = recursos.dados::<Paleta>("paleta.json")?.rgb()?;
    if let Some(dir) = &o.som.exportar {
        return som::exportar(&recursos, &o.som, dir);
    }

    let mut o = o;
    // data/video.json (opcional; um mod pode trocá-lo) e, por cima, --escala e --janela.
    let arquivo_video = if recursos.existe_dado(video::ARQUIVO) { recursos.dados(video::ARQUIVO)? } else { video::ConfigVideo::default() };
    o.video = arquivo_video.com_linha_de_comando(o.escala, o.janela);
    if !o.novo_jogo && o.save.is_none() && o.exportar.is_none() {
        // Abertura até o "Select a Game"; a escolha vira o jogo novo.
        let seq: Sequencia = recursos.dados("telas.json")?;
        let interface = Interface::carregar(&recursos, config.tabela_posicoes)?;
        match executar(seq, &recursos, &interface, &cores, &o)? {
            Some(e) => match e.tipo_jogo() {
                Some(tipo) => {
                    println!("Select a Game: {e:?}");
                    o.novo_jogo = true;
                    o.tipo = tipo;
                }
                None => {
                    println!("Select a Game: {e:?} (sem jogo novo)");
                    return Ok(());
                }
            },
            None => return Ok(()),
        }
    }
    // Estado do programa antes de qualquer jogo (o "anterior" de um Load de Tutorial.Ant no começo).
    let mut inicial = Estado::vazio();
    let tut: engine::core::carregar::ConfigTutorialArquivo = recursos.dados("tutorial_arquivo.json")?;
    let origem = match (&o.save, o.novo_jogo) {
        (Some(save), _) => {
            // _initStuff semeia S e R com o relógio; o Load do começo do programa roda _RandYard
            // com o _CurGameType inicial, 0 [hipótese: segmento 9 zerado no carregamento].
            let n = semente(&o);
            let mut s = GeradorS::new(0);
            let mut r = GeradorR::default();
            r.semear_com_ticks(&mut s, n, n);
            if engine::core::carregar::e_o_tutorial(save, &tut) {
                inicial.r = r;
                Some(Origem::Tutorial(&inicial, s.semente()))
            } else {
                Some(Origem::Save(save, r, TipoJogo::Tutorial))
            }
        }
        (None, true) => {
            let n = semente(&o);
            println!("{:?} com semente {n}", o.tipo);
            Some(Origem::Novo(n, o.tipo))
        }
        _ => None,
    };
    if let Some(origem) = origem {
        let de_save = matches!(origem, Origem::Save(..));
        let nome = match &origem {
            Origem::Save(save, ..) => save.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            Origem::Novo(_, tipo) => format!("{tipo:?}"),
            Origem::Tutorial(..) => "Tutorial Game".into(),
            Origem::NovoNaPartida(_, tipo) => format!("{tipo:?}"),
        };
        let mut s = Sessao::abrir(&recursos, &o.dados, origem, cores)?;
        for &a in &o.acoes {
            s.partida.navegar(a);
        }
        if let Some(m) = o.quintal {
            let a = [AcaoQuintal::ModoQuintal, AcaoQuintal::ModoCasa, AcaoQuintal::ModoEstrategico, AcaoQuintal::ModoGrafico];
            let a = *a.get(m as usize).ok_or("--quintal: modo 0 a 3")?;
            modos::aplicar_acao_quintal(&mut s, a, true);
        }
        if let Some(m) = o.janela_mapa {
            s.abrir_mapa();
            match m {
                2 => {
                    s.partida.navegar(Acao::NinhoPreto);
                }
                3 => {
                    s.partida.navegar(Acao::NinhoVermelho);
                }
                // O botão alterna: com o save já nesse modo, não aperta (senão desligaria o feromônio).
                4..=8 if s.partida.estado.vistas.modo_mapa != m => s.partida.botao_de_cheiro(m),
                _ => {}
            }
        }
        s.partida.mostrar_trilhas = o.trilhas;
        if o.trilhas {
            // Trilhas: a primeira geração é a base; os ticks seguintes acumulam.
            s.quadro_rgb()?;
        }
        if let Some(n) = o.ticks {
            let antes = s.partida.estado.partida.tempo;
            for _ in 0..n {
                s.partida.rodar_tick();
                if o.janela_mapa.is_some() {
                    s.mapa.atualizar(&s.partida.estado, s.partida.mostrar_trilhas);
                }
            }
            s.registrar_eventos();
            println!("{n} ticks: GameTime {antes} -> {}", s.partida.estado.partida.tempo);
        }
        for &b in &o.botoes {
            println!("botão {b}: {:?}", s.botao_usuario(b));
        }
        if let Some(n) = o.marcar {
            s.partida.marcador(n.saturating_sub(1), true);
        }
        match o.menu.as_deref().and_then(|m| m.split_once('@')) {
            Some(("amarela", p)) => {
                let (x, y) = p.split_once(',').ok_or("--menu amarela@X,Y")?;
                let (x, y) = (x.parse().map_err(|_| "--menu: x")?, y.parse().map_err(|_| "--menu: y")?);
                s.abrir_menu_da_amarela(x, y);
            }
            Some(("icones", nome)) => {
                let (r, _) = s.tela.objeto_da_faixa(nome).ok_or("--menu: botão desconhecido")?;
                s.abrir_menu_de_icones(i32::from(r.esq) + 2, i32::from(r.topo) + 2);
            }
            Some(("mod", tecla)) => mods_ui::menu_pela_linha_de_comando(&mut s, tecla)?,
            Some(_) => return Err("--menu amarela@X,Y, icones@OBJETO ou mod@TECLA[#ITEM]".into()),
            None => {}
        }
        match o.dialogo.as_deref() {
            Some("load") => s.dialogo = Some(arquivos::Dialogo::Load(s.dialogo_arquivo(false))),
            Some("salvar_como") => s.dialogo = Some(arquivos::Dialogo::SalvarComo(s.dialogo_arquivo(true))),
            Some("pergunta") => {
                s.partida.estado.precisa_salvar = true;
                s.abrir_load();
            }
            Some("salvo") => {
                let m = engine::core::dialogos::CaixaMensagem::salvo(&s.dialogos_cfg, r"C:\MAXIS\ANTWIN\QUICK1.ANT");
                s.dialogo = Some(arquivos::Dialogo::Salvo(m, false));
            }
            Some(d) => return Err(format!("--dialogo {d}: load, pergunta, salvar_como ou salvo")),
            None => {}
        }
        if let Some((x, y)) = o.examinar {
            let plano = i32::from(s.partida.nav.plano_mapa);
            match engine::rules::experimental::examinar(&s.partida.estado, plano, x, y) {
                Some(ex) => s.abrir_exame(ex),
                None => println!("Examine ({x}, {y}): célula sem vida"),
            }
        }
        for _ in 0..o.fechar_avisos {
            s.partida.fechar_modal();
        }
        if let Some(Modal::FimDeJogo { pontos, .. }) = s.partida.avisos.modal() {
            println!("fim de jogo: {pontos} pontos");
        }
        if let Some(png) = &o.exportar_tela_jogo {
            let rgb = s.quadro_rgb()?;
            salvar_png(png, s.tela.cfg.largura, s.tela.cfg.altura, &rgb)?;
            let (p, v) = (&s.partida.nav, &s.partida.nav.vista);
            println!("{} (plano {}, vista em ({}, {}), {} × {} tiles)", png.display(), p.plano_mapa, v.x, v.y, v.largura, v.altura);
        }
        if let Some(arq) = &o.salvar {
            s.partida.estado.para_save(&s.layout)?.gravar(arq)?;
            println!("{} Saved correctly", arq.display());
        }
        if o.ticks.is_some() || o.exportar_tela_jogo.is_some() || o.salvar.is_some() {
            return Ok(());
        }
        let jogo_novo = o.novo_jogo;
        // Preferências do jogador (o `[SimAnt]` do `WIN.INI`): só a partida com janela lê e grava.
        if o.preferencias.is_none() && !o.sem_preferencias {
            o.preferencias = preferencias::Preferencias::caminho_padrao();
        }
        if o.sem_preferencias {
            o.preferencias = None;
        }
        return executar_partida(s, &format!("SimAnt - {nome}"), &o, jogo_novo, de_save);
    }

    let seq: Sequencia = recursos.dados("telas.json")?;
    let interface = Interface::carregar(&recursos, config.tabela_posicoes)?;
    match &o.exportar {
        Some(dir) => exportar(&seq, &recursos, &interface, &cores, dir),
        None => Ok(()),
    }
}

fn main() {
    if let Err(e) = principal() {
        eprintln!("erro: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use sdl2::keyboard::{Keycode, Mod};

    /// `SDL_GetKeyName` devolve um buffer estático: testes que chamam `Keycode::name` em paralelo
    /// podem ler o nome um do outro. Esta trava os serializa.
    static NOMES_SDL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn teclas_do_sdl_viram_acoes_de_data_navegacao() {
        let _trava = NOMES_SDL.lock().unwrap_or_else(|e| e.into_inner());
        let caminho = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/navegacao.json");
        let cfg: ConfigNavegacao = serde_json::from_str(&std::fs::read_to_string(caminho).unwrap()).unwrap();
        let acao = |k, m| cfg.acao_da_tecla(&evento_de_tecla(k, m));
        // Aceleradores do original (Ctrl + seta, Ctrl + 8/9/0), com Ctrl esquerdo ou direito.
        assert_eq!(acao(Keycode::Right, Mod::LCTRLMOD), Some(Acao::LinhaDireita));
        assert_eq!(acao(Keycode::Up, Mod::RCTRLMOD | Mod::NUMMOD), Some(Acao::LinhaAcima), "NumLock não conta");
        assert_eq!(acao(Keycode::Num8, Mod::LCTRLMOD), Some(Acao::Superficie));
        assert_eq!(acao(Keycode::Num9, Mod::RCTRLMOD), Some(Acao::NinhoPreto));
        assert_eq!(acao(Keycode::Num0, Mod::LCTRLMOD), Some(Acao::NinhoVermelho));
        // Extensões do port: setas e PageUp/PageDown sem modificador.
        assert_eq!(acao(Keycode::Left, Mod::NOMOD), Some(Acao::LinhaEsquerda));
        assert_eq!(acao(Keycode::PageDown, Mod::NOMOD), Some(Acao::PaginaAbaixo));
        assert_eq!(acao(Keycode::Num8, Mod::NOMOD), None);
        assert_eq!(acao(Keycode::Down, Mod::LSHIFTMOD), None);
    }

    #[test]
    fn teclas_do_sdl_viram_aceleradores_e_comandos_de_data_partida() {
        let _trava = NOMES_SDL.lock().unwrap_or_else(|e| e.into_inner());
        use engine::core::partida::AcaoPartida;
        let caminho = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/partida.json");
        let cfg = ConfigPartida::de_json(&std::fs::read_to_string(caminho).unwrap()).unwrap();
        let acao = |k, m| cfg.acao_da_tecla(&evento_de_tecla(k, m));
        assert_eq!(acao(Keycode::Num0, Mod::LSHIFTMOD), Some(AcaoPartida::Pausa));
        assert_eq!(acao(Keycode::Num4, Mod::RSHIFTMOD), Some(AcaoPartida::VelocidadeUltra));
        assert_eq!(acao(Keycode::Kp2, Mod::LSHIFTMOD), Some(AcaoPartida::VelocidadeNormal));
        assert_eq!(acao(Keycode::G, Mod::LCTRLMOD), Some(AcaoPartida::AutoTrack));
        assert_eq!(acao(Keycode::Num0, Mod::NOMOD), None);
        assert_eq!(cfg.tecla_virtual(&Keycode::Num1.name()), Some(0x31));
        assert_eq!(cfg.tecla_virtual(&Keycode::KpMultiply.name()), Some(0x6A));
        assert_eq!(cfg.tecla_virtual(&Keycode::Backspace.name()), Some(0x08));
        assert_eq!(cfg.tecla_virtual(&Keycode::F9.name()), Some(0x78));
        assert_eq!(cfg.botao_da_tecla(&Keycode::Space.name()), Some(BotaoTeclado::Esquerdo));
        assert_eq!(cfg.botao_da_tecla(&Keycode::Insert.name()), Some(BotaoTeclado::Esquerdo));
        assert_eq!(cfg.botao_da_tecla(&Keycode::Delete.name()), Some(BotaoTeclado::Direito));
    }

    #[test]
    fn lista_de_acoes_da_linha_de_comando() {
        assert_eq!(ler_acoes("superficie, linha_direita").unwrap(), vec![Acao::Superficie, Acao::LinhaDireita]);
        assert!(ler_acoes("voar").is_err());
    }
}
