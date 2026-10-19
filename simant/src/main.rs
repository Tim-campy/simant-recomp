//! Executável do port: janela, entrada e desenho sobre SDL2. A lógica fica no crate `engine`; aqui só
//! se traduzem eventos do SDL em eventos do engine e se mostra o quadro composto.
//!
//! Uso:
//!   simant                     sem opções: abre o launcher (opções de inicialização, launcher/), ou
//!                              vai direto ao jogo se a configuração dele pedir (`mostrar_launcher`)
//!   simant [--jogo DIR] [--dados DIR] [--mods DIR] [--desligar-mods A,B] [--exportar-telas DIR]
//!   simant (--save ARQ.ANT | --novo-jogo [--semente N]) [--acoes a,b,...]
//!          [--ticks N] [--avancar-ms MS] [--botoes F,F,...] [--historico B,B,...] [--fechar-avisos N] [--exportar-tela-jogo ARQ.png]
//!          [--salvar ARQ.ANT]
//!   opções da janela: [--resolucao LxA] [--escala N] [--janela LxA] [--moldura | --sem-moldura]
//!     [--moldura-do-sistema | --moldura-sem-sistema]
//!   ritmo (data/partida.json): [--velocidades proporcional|original] [--sem-foco lenta|original]
//!     [--perfil-driver vga|svga256|s3_dosbox] [--sair-apos-ms N [--capturar-janela ARQ.png]]
//!   (padrões em data/video.json: tela virtual 640×480 em escala 1, como a bancada; simant/src/video.rs;
//!   --resolucao 800x600 ou 1024x768 muda o layout como o original, docs/spec/telas/resolucao.md)
//!   som (simant/src/som): [--sem-som | --com-som] [--soundfont ARQ.sf2] [--sintetizador simples|soundfont|fm]
//!     [--exportar-som DIR [--exportar-musica ID]]
//!   preferências do jogador (simant/src/preferencias.rs): [--preferencias ARQ] [--sem-preferencias]
//!   [--win-ini ARQ] (origem enquanto não há preferências gravadas; padrão: WINDOWS/WIN.INI acima do jogo)
//!   launcher e saída (config_launcher.rs, registro.rs): [--launcher] [--config ARQ | --sem-config]
//!     [--console] [--capturar-launcher ARQ.png] [--launcher-jogar-apos-ms N (teste)]
//!
//! Padrões: --jogo (a configuração do launcher, `SIMANT_DIR` ou SimAnt-T/MAXIS/ANTWIN), --dados data,
//! --mods mods, relativos à pasta base do port: a do executável, se tiver `data/`, senão a atual
//! (pastas.rs). A configuração do launcher (`launcher.json` na pasta de dados do usuário) entra antes
//! das opções da linha de comando, que vencem (config_launcher::combinar). Sem --save nem
//! --novo-jogo, mostra a abertura. Com um deles, a simulação roda ao
//! vivo (engine::core::partida); com --ticks, --exportar-tela-jogo ou --salvar, roda sem janela e
//! sai (--salvar grava o .ANT do estado no fim). Na janela, Ctrl+S salva no arquivo da partida e
//! Ctrl+A ("Save As") num nome novo (o de --salvar, ou SALVO01.ANT, SALVO02.ANT… na pasta atual).

// Sem console no Windows no executável de lançamento (o de depuração, do `cargo run`, mantém o
// console); a saída vai para o log (registro.rs) e `--console` liga um. docs/decisoes.md, 2026-10-19.
#![cfg_attr(all(windows, not(debug_assertions), not(test)), windows_subsystem = "windows")]

mod ajuda_ui;
mod arquivos;
mod config_launcher;
mod jogo;
mod launcher;
mod opcoes;
mod pastas;
mod preferencias;
mod registro;
mod sessao;
mod som;
mod video;

use jogo::*;
use opcoes::*;
use sessao::*;

mod janelas_ui;
mod menus_ui;
mod mini_ui;
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
    // `tabela_posicoes` (id do objeto tipo 9) não é mais lida pelo executável: a tabela vem da tela
    // virtual (data/resolucao.json). Fica no arquivo para os testes e exemplos.
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

/// `--exportar-telas`: a área cliente de cada tela da abertura; com `--moldura`, a janela inteira
/// (título, barra de menus e área cliente: a tela inteira da bancada, telas/menus.md §8).
fn exportar(seq: &Sequencia, recursos: &Recursos, interface: &Interface, cores: &[[u8; 3]], dir: &Path, moldura: bool) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for t in &seq.telas {
        let mut rgb = compor_rgb(t, recursos, interface, cores)?;
        let mut altura = t.altura;
        if moldura {
            let mut menus = menus_ui::MenusUi::carregar(recursos)?;
            menus.ligar_moldura(recursos, t.largura)?;
            let titulo = engine::core::moldura_principal::EstadoTitulo { ativa: true, ..Default::default() };
            let estado = engine::core::barra_menus::EstadoMenus::default();
            rgb = menus_ui::compor_barras(&menus, interface, cores, &titulo, &estado, &rgb, t.largura as usize, t.altura as usize);
            altura += menus.altura() as u32;
        }
        let caminho = dir.join(format!("{}.png", t.nome));
        salvar_png(&caminho, t.largura, altura, &rgb)?;
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

/// O que a abertura decidiu.
enum SaidaAbertura {
    /// Jogo novo do tipo escolhido no "Select a Game".
    Jogo(engine::core::selecao::Escolha),
    /// "Load Game" e o arquivo escolhido no "Load Ant Hill".
    Carregar(PathBuf),
}

/// Abertura até o "Select a Game"; devolve a escolha feita nele (None = saiu).
fn executar(seq: Sequencia, recursos: &Recursos, interface: &Interface, cores: &[[u8; 3]], o: &Opcoes) -> Result<Option<SaidaAbertura>, String> {
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::pixels::PixelFormatEnum;

    let (lw, lh) = seq.telas.first().map(|t| (t.largura, t.altura)).unwrap_or((640, 480));
    // O original já mostra a janela principal maximizada na abertura: a barra de menus (e o título,
    // com `moldura_windows31`) em cima da área cliente, paradas: nem o Alt+F, nem o clique no File,
    // na caixa ou no minimizar fazem nada no "Select a Game" (bancada, qemu_selecao_moldura.txt;
    // telas/menus.md §8).
    let mut menus = menus_ui::MenusUi::carregar(recursos)?;
    if o.video.moldura_windows31 {
        menus.ligar_moldura(recursos, lw)?;
    }
    let barras = menus.altura();
    let Janela { sdl, mut canvas } = abrir_janela("SimAnt", lw, lh + barras as u32, &o.video)?;
    let criador = canvas.texture_creator();
    let mut eventos = sdl.event_pump()?;
    let selecao = modos::selecao_da_abertura(recursos, interface)?;
    let mut audio = som::Audio::abrir(&sdl, recursos, &o.som);
    audio.musica_abertura();

    let mut rep = Reprodutor::new(seq);
    let mut desenhada: Option<String> = None;
    let mut ativa_desenhada = true;
    let mut textura = None;
    let inicio = Instant::now();
    let mut antes = inicio;

    // "Load Ant Hill" aberto pelo Load Game (arquivos.rs).
    let mut load: Option<arquivos::LoadNaAbertura> = None;
    let escolher = |e: engine::core::selecao::Escolha, load: &mut Option<arquivos::LoadNaAbertura>| -> Result<Option<SaidaAbertura>, String> {
        match e {
            engine::core::selecao::Escolha::Carregar => {
                *load = Some(arquivos::LoadNaAbertura::abrir(recursos)?);
                Ok(None)
            }
            e => Ok(Some(SaidaAbertura::Jogo(e))),
        }
    };

    while !rep.terminou() {
        let escolhendo = rep.atual().is_some_and(|t| t.nome == selecao.cfg.tela);
        let lote: Vec<Event> = eventos.poll_iter().collect();
        for ev in lote {
            // Coordenadas da janela → área cliente (as barras ficam em cima e não respondem).
            let ev = menus_ui::deslocar(ev, barras);
            // Clique nas barras: nada (medido no "Select a Game"; no título, [hipótese] o mesmo).
            if let Event::MouseButtonDown { y, .. } | Event::MouseButtonUp { y, .. } = ev {
                if y < 0 {
                    continue;
                }
            }
            if matches!(ev, Event::Quit { .. }) {
                return Ok(None);
            }
            if let Some(l) = load.as_mut() {
                match l.evento(&ev) {
                    Some(engine::core::dialogos::RespostaArquivo::Escolher(p)) => return Ok(Some(SaidaAbertura::Carregar(p))),
                    Some(_) => load = None,
                    None => {}
                }
                desenhada = None;
                continue;
            }
            match ev {
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return Ok(None),
                Event::KeyDown { keycode: Some(k), keymod, .. } if escolhendo => {
                    if let Some(e) = selecao.cfg.escolha_da_tecla(&evento_de_tecla(k, keymod)) {
                        if let Some(s) = escolher(e, &mut load)? {
                            return Ok(Some(s));
                        }
                        desenhada = None;
                    }
                }
                Event::MouseButtonDown { x, y, .. } if escolhendo => {
                    if let Some(e) = selecao.escolha_em(x, y) {
                        if let Some(s) = escolher(e, &mut load)? {
                            return Ok(Some(s));
                        }
                        desenhada = None;
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

        let Some(tela) = rep.atual() else { break };
        // O título fica inativo sem o foco do sistema ou com o "Load Ant Hill" (diálogo) por cima.
        let ativa = janela_ativa(canvas.window()) && load.is_none();
        if desenhada.as_deref() != Some(tela.nome.as_str()) || ativa != ativa_desenhada {
            let rgb = match &load {
                Some(l) => {
                    // O original fecha o "Select a Game" antes de abrir o "Load Ant Hill" (bancada,
                    // 2026-10-19, re_trabalho/raros/load/a2_dialogo.png): o diálogo fica sobre o fundo.
                    let mut fundo = tela.clone();
                    fundo.janelas.retain(|j| j.id != selecao.cfg.janela);
                    let (mut q, _) = compor(&fundo, recursos, interface)?;
                    l.desenhar(&mut q, interface);
                    para_rgb(&q.pixels, cores)
                }
                None => compor_rgb(tela, recursos, interface, cores)?,
            };
            let titulo = engine::core::moldura_principal::EstadoTitulo { ativa, ..Default::default() };
            let estado = engine::core::barra_menus::EstadoMenus::default();
            let rgb = menus_ui::compor_barras(&menus, interface, cores, &titulo, &estado, &rgb, tela.largura as usize, tela.altura as usize);
            let altura = tela.altura + barras as u32;
            let mut t = criador.create_texture_static(PixelFormatEnum::RGB24, tela.largura, altura).map_err(|e| e.to_string())?;
            t.update(None, &rgb, tela.largura as usize * 3).map_err(|e| e.to_string())?;
            canvas.set_logical_size(tela.largura, altura).map_err(|e| e.to_string())?;
            textura = Some(t);
            desenhada = Some(tela.nome.clone());
            ativa_desenhada = ativa;
        }
        canvas.clear();
        if let Some(t) = &textura {
            canvas.copy(t, None, None)?;
        }
        if o.sair_apos_ms.is_some_and(|ms| (agora - inicio).as_millis() as u64 >= ms) {
            // Como na partida (jogo.rs): o quadro da abertura lido antes do present.
            if let Some(c) = &o.capturar_janela {
                let (w, h) = canvas.output_size()?;
                let px = canvas.read_pixels(None, PixelFormatEnum::RGB24)?;
                salvar_png(c, w, h, &px)?;
                println!("{} ({w} × {h})", c.display());
            }
            return Ok(None);
        }
        canvas.present();
    }
    Ok(None)
}

fn principal(o: Opcoes) -> Result<(), String> {
    // data/jogo.json diz quais bancos abrir; um mod pode trocá-lo (API 0.20), então ele é lido com os
    // mods antes de abrir os bancos.
    let config: ConfigJogo = Recursos::abrir_sem(&o.jogo, &[], &o.dados, Some(&o.mods), &o.mods_desligados)?.dados("jogo.json")?;
    let mut recursos = Recursos::abrir_sem(&o.jogo, &config.bancos, &o.dados, Some(&o.mods), &o.mods_desligados)?;
    if let Some(p) = &o.perfil_driver {
        // --perfil-driver: por cima de data/resolucao.json (e dos mods), conferido já aqui.
        recursos.sobrepor_dados(engine::core::resolucao::ARQUIVO, serde_json::json!({ "perfil_driver": p }));
        recursos.dados::<engine::core::resolucao::ConfigResolucao>(engine::core::resolucao::ARQUIVO)?.perfil(None)?;
    }
    if let Some(v) = sobreposicao_partida(o.velocidades.as_deref(), o.sem_foco.as_deref()) {
        // --velocidades e --sem-foco: por cima de data/partida.json (e dos mods).
        recursos.sobrepor_dados(engine::core::partida::ARQUIVO_CONFIG, v);
    }
    for m in recursos.mods() {
        println!("mod carregado: {} {}", m.manifesto.nome, m.manifesto.versao);
    }
    let cores = engine::paleta::carregar_rgb(&recursos)?;
    if let Some(dir) = &o.som.exportar {
        return som::exportar(&recursos, &o.som, dir);
    }

    let mut o = o;
    // data/video.json (opcional; um mod pode trocá-lo) e, por cima, --escala e --janela.
    let arquivo_video = if recursos.existe_dado(video::ARQUIVO) { recursos.dados(video::ARQUIVO)? } else { video::ConfigVideo::default() };
    o.video = arquivo_video.com_linha_de_comando(o.resolucao, o.escala, o.janela);
    if let Some(m) = o.moldura {
        o.video.moldura_windows31 = m;
    }
    // Tela virtual: tamanho da área cliente e tabela de posições da resolução imitada, pela regra
    // do original em data/resolucao.json (docs/spec/telas/resolucao.md).
    let regra: engine::core::resolucao::ConfigResolucao = recursos.dados(engine::core::resolucao::ARQUIVO)?;
    let tela_virtual = regra.tela(o.video.resolucao_virtual)?;
    o.tela_virtual = Some(tela_virtual);
    if !o.novo_jogo && o.save.is_none() && o.exportar.is_none() {
        // Abertura até o "Select a Game"; a escolha vira o jogo novo.
        let mut seq: Sequencia = recursos.dados("telas.json")?;
        tela_virtual.aplicar_sequencia(&mut seq);
        let mut interface = Interface::carregar(&recursos, tela_virtual.tabela_posicoes)?;
        interface.area = [tela_virtual.largura as i16, tela_virtual.altura as i16];
        match executar(seq, &recursos, &interface, &cores, &o)? {
            Some(SaidaAbertura::Carregar(p)) => {
                // Load Game na abertura: o Load do começo do programa (como --save).
                println!("Select a Game: Load Game {}", p.display());
                o.save = Some(p);
            }
            Some(SaidaAbertura::Jogo(e)) => match e.tipo_jogo() {
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
        let mut s = Sessao::abrir_na_tela(&recursos, &o.dados, origem, cores, o.tela_virtual)?;
        if o.ticks.is_some() || o.exportar_tela_jogo.is_some() || o.salvar.is_some() {
            // Sem janela, as preferências só valem se pedidas (--preferencias/--win-ini).
            jogo::aplicar_preferencias(&mut s, &o, de_save);
        }
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
        for k in 0..o.mini_mapa {
            // O clique no ribToYellow, solto fora do mini mapa: fica aberto; antes de reabrir, um
            // clique fora fecha.
            if k > 0 {
                mini_ui::evento(&mut s, &sdl2::event::Event::MouseButtonDown {
                    timestamp: 0,
                    window_id: 0,
                    which: 0,
                    mouse_btn: sdl2::mouse::MouseButton::Left,
                    clicks: 1,
                    x: -1,
                    y: -1,
                });
            }
            let r = s.tela.retangulo_da_faixa(&s.mini.cfg.botao).ok_or("--mini-mapa: botão sem retângulo")?;
            let (x, y) = (i32::from(r.esq + r.dir) / 2, i32::from(r.topo + r.base) / 2);
            if !mini_ui::abrir_pelo_botao(&mut s, x, y) {
                return Err("--mini-mapa: o botão não abriu o mini mapa".into());
            }
            s.mini.soltar(-1, -1);
        }
        if let Some(ms) = o.avancar_ms {
            let av = s.partida.avancar(ms);
            s.registrar_eventos();
            let quadro = s.partida.quadro_animado().map(|(p, q)| format!(", animação no plano {p}: tipo {:#x} dir {}", q.tipo, q.dir));
            println!("{ms} ms: {} ticks{}", av.ticks, quadro.unwrap_or_default());
        }
        for &b in &o.botoes {
            println!("botão {b}: {:?}", s.botao_usuario(b));
        }
        if !o.historico.is_empty() {
            s.botao_usuario(2);
            for &b in &o.historico {
                s.janelas.historico.alternar(b);
            }
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
            Some(("sistema", id)) => {
                // Menu de sistema da janela ID (aberta se preciso), para comparar com a bancada (w04).
                let id: u16 = id.parse().map_err(|_| "--menu sistema@ID")?;
                if id == s.janelas.cfg.id(Controle::Castas) {
                    s.botao_usuario(1);
                }
                if !s.janelas.gerente.abrir_menu(id) {
                    return Err(format!("--menu sistema@{id}: janela fechada"));
                }
            }
            Some(("barra", m)) => {
                // Barra de menus (menus_ui.rs): menu M aberto, com o item I realçado.
                let (m, i) = m.split_once(',').map_or((m, None), |(m, i)| (m, i.parse().ok()));
                let menu = m.parse().map_err(|_| "--menu barra@M[,ITEM]")?;
                s.menus.barra.estado = engine::core::barra_menus::Estado::Aberto { menu, item: i };
            }
            Some(_) => return Err("--menu amarela@X,Y, icones@OBJETO, mod@TECLA[#ITEM], sistema@ID ou barra@M[,ITEM]".into()),
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
            Some("sair") => s.abrir_pergunta_sair(),
            Some("sobre") => {
                menus_ui::executar(&mut s, engine::core::barra_menus::ComandoMenu::Sobre, None);
            }
            Some("status") => s.menus.status = true,
            Some(d) => return Err(format!("--dialogo {d}: load, pergunta, salvar_como, salvo, sair, sobre ou status")),
            None => {}
        }
        for &(x, y, duplo) in &o.cliques_dialogo {
            s.clicar_no_dialogo(x, y, duplo);
        }
        if let Some((x, y)) = o.examinar {
            s.janela_exame.ja_aberta = o.exame_ja_aberto;
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
            s.partida.gravar_estado_dos_mods(arq)?;
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
        if o.win_ini.is_none() && !o.sem_preferencias {
            o.win_ini = preferencias::procurar_win_ini(&o.jogo);
        }
        if o.sem_preferencias {
            o.preferencias = None;
            o.win_ini = None;
        }
        return executar_partida(s, &format!("SimAnt - {nome}"), &o, jogo_novo, de_save);
    }

    let mut seq: Sequencia = recursos.dados("telas.json")?;
    tela_virtual.aplicar_sequencia(&mut seq);
    let mut interface = Interface::carregar(&recursos, tela_virtual.tabela_posicoes)?;
    interface.area = [tela_virtual.largura as i16, tela_virtual.altura as i16];
    match &o.exportar {
        Some(dir) => exportar(&seq, &recursos, &interface, &cores, dir, o.video.moldura_windows31),
        None => Ok(()),
    }
}

/// Opções do programa em si (launcher, configuração, saída), tiradas antes das do jogo.
#[derive(Debug, Default, PartialEq)]
struct OpcoesPrograma {
    config: Option<PathBuf>,
    sem_config: bool,
    launcher: bool,
    console: bool,
    capturar_launcher: Option<PathBuf>,
    /// Teste: abre o launcher e aperta Jogar depois de N ms.
    launcher_jogar_apos_ms: Option<u64>,
}

fn separar_opcoes(args: Vec<String>) -> Result<(OpcoesPrograma, Vec<String>), String> {
    let mut p = OpcoesPrograma::default();
    let mut resto = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--config" => p.config = Some(it.next().ok_or("falta o valor de --config")?.into()),
            "--sem-config" => p.sem_config = true,
            "--launcher" => p.launcher = true,
            "--console" => p.console = true,
            "--capturar-launcher" => p.capturar_launcher = Some(it.next().ok_or("falta o valor de --capturar-launcher")?.into()),
            "--launcher-jogar-apos-ms" => {
                let v = it.next().ok_or("falta o valor de --launcher-jogar-apos-ms")?;
                p.launcher_jogar_apos_ms = Some(v.parse().map_err(|_| "--launcher-jogar-apos-ms: número inválido")?);
                p.launcher = true;
            }
            _ => resto.push(a),
        }
    }
    Ok((p, resto))
}

/// A pasta do jogo que a configuração do launcher faria o jogo usar tem os arquivos do SimAnt?
fn jogo_da_config_valido(c: &config_launcher::ConfigLauncher, base: &Path) -> bool {
    let pasta = match &c.jogo {
        Some(j) => pastas::resolver(Path::new(j), base),
        None => pasta_jogo_padrao(base),
    };
    config_launcher::validar_pasta_jogo(&pasta).ok
}

/// Configuração do launcher, launcher (se for o caso) e o jogo.
fn iniciar(p: OpcoesPrograma, cli: Vec<String>) -> Result<(), String> {
    use config_launcher::ConfigLauncher;
    let caminho = if p.sem_config { None } else { p.config.clone().or_else(ConfigLauncher::caminho_padrao) };
    let (cfg, mut aviso) = match caminho.as_deref().map(ConfigLauncher::ler) {
        Some(Ok(c)) => (c, None),
        Some(Err(e)) => {
            eprintln!("aviso: configuração do launcher ignorada: {e}");
            (None, Some(format!("Configuração ignorada (inválida): {e}")))
        }
        None => (None, None),
    };
    let base = pastas::base();
    if let Some(png) = &p.capturar_launcher {
        let mut e = launcher::Estado::novo(cfg, caminho, base);
        launcher::capturar(&mut e, png)?;
        println!("{}", png.display());
        return Ok(());
    }
    // Sem opções de jogo: o launcher, salvo se a configuração pede o jogo direto e a pasta vale.
    let sem_opcoes = cli.is_empty() && !p.sem_config;
    let direto = cfg.as_ref().is_some_and(|c| !c.mostrar_launcher && jogo_da_config_valido(c, &base));
    if p.launcher || (sem_opcoes && !direto) {
        if cfg.as_ref().is_some_and(|c| !c.mostrar_launcher) && !direto {
            aviso.get_or_insert("A pasta do jogo da configuração não tem os arquivos do SimAnt; escolha outra.".into());
        }
        let mut e = launcher::Estado::novo(cfg, caminho, base);
        return match launcher::executar(&mut e, aviso, p.launcher_jogar_apos_ms)? {
            launcher::Saida::Jogar(mut args) => {
                println!("launcher: simant {}", config_launcher::formatar_linha(&args));
                // Opções dadas junto com --launcher vêm depois (e vencem).
                args.extend(cli);
                principal(ler_opcoes(args)?)
            }
            launcher::Saida::Sair => Ok(()),
        };
    }
    principal(ler_opcoes(config_launcher::combinar(cfg.as_ref(), &cli))?)
}

fn main() {
    let (p, cli) = match separar_opcoes(std::env::args().skip(1).collect()) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("erro: {e}");
            std::process::exit(1);
        }
    };
    let saida = registro::preparar(p.console, cli.is_empty());
    registro::avisar_panico(&saida);
    if let Err(e) = iniciar(p, cli) {
        registro::erro_fatal(&saida, &e);
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
        assert_eq!(cfg.botao_da_tecla(&Keycode::Delete.name()), None, "Delete é a tecla do modo de ajuda");
    }

    #[test]
    fn lista_de_acoes_da_linha_de_comando() {
        assert_eq!(ler_acoes("superficie, linha_direita").unwrap(), vec![Acao::Superficie, Acao::LinhaDireita]);
        assert!(ler_acoes("voar").is_err());
    }

    /// Com `moldura_windows31`, a abertura exportada é a janela inteira da bancada (640 × 480: título,
    /// barra de menus e área cliente); sem ela, só a área cliente (as comparações antigas).
    #[test]
    fn abertura_com_a_moldura_tem_a_tela_inteira() {
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz.join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SHARED.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
            return;
        }
        let dados = raiz.join("data");
        let cfg: ConfigJogo = serde_json::from_str(&std::fs::read_to_string(dados.join("jogo.json")).unwrap()).unwrap();
        let recursos = Recursos::abrir(&jogo, &cfg.bancos, &dados, None).unwrap();
        let cores = engine::paleta::carregar_rgb(&recursos).unwrap();
        let regra: engine::core::resolucao::ConfigResolucao = recursos.dados(engine::core::resolucao::ARQUIVO).unwrap();
        let tv = regra.tela([640, 480]).unwrap();
        let mut seq: Sequencia = recursos.dados("telas.json").unwrap();
        tv.aplicar_sequencia(&mut seq);
        let mut interface = Interface::carregar(&recursos, tv.tabela_posicoes).unwrap();
        interface.area = [tv.largura as i16, tv.altura as i16];
        let dir = std::env::temp_dir().join(format!("simant_abertura_moldura_{}", std::process::id()));
        for (moldura, altura) in [(false, 442), (true, 480)] {
            exportar(&seq, &recursos, &interface, &cores, &dir, moldura).unwrap();
            let dec = png::Decoder::new(std::fs::File::open(dir.join("selecao.png")).unwrap());
            let info = dec.read_info().unwrap();
            assert_eq!((info.info().width, info.info().height), (640, altura));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
