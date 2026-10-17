//! Executável do port: janela, entrada e desenho sobre SDL2. A lógica fica no crate `engine`; aqui só
//! se traduzem eventos do SDL em eventos do engine e se mostra o quadro composto.
//!
//! Uso:
//!   simant [--jogo DIR] [--dados DIR] [--mods DIR] [--exportar-telas DIR]
//!   simant (--save ARQ.ANT | --novo-jogo [--semente N]) [--acoes a,b,...]
//!          [--ticks N] [--botoes F,F,...] [--fechar-avisos N] [--exportar-tela-jogo ARQ.png]
//!          [--salvar ARQ.ANT]
//!   opções da janela: [--escala N] [--sair-apos-ms N [--capturar-janela ARQ.png]]
//!   som (simant/src/som): [--sem-som] [--soundfont ARQ.sf2] [--exportar-som DIR]
//!
//! Padrões: --jogo $SIMANT_DIR ou SimAnt-T/MAXIS/ANTWIN, --dados data, --mods mods (relativos à
//! pasta atual). Sem --save nem --novo-jogo, mostra a abertura. Com um deles, a simulação roda ao
//! vivo (engine::core::partida); com --ticks, --exportar-tela-jogo ou --salvar, roda sem janela e
//! sai (--salvar grava o .ANT do estado no fim). Na janela, Ctrl+S salva no arquivo da partida e
//! Ctrl+A ("Save As") num nome novo (o de --salvar, ou SALVO01.ANT, SALVO02.ANT… na pasta atual).

mod jogo;
mod opcoes;
mod sessao;
mod som;

use jogo::*;
use opcoes::*;
use sessao::*;

use std::path::{Path, PathBuf};
use std::time::Instant;

use engine::core::avisos::{self, ConfigAvisos, Escolha, Modal};
use engine::core::navegacao::{Acao, ConfigNavegacao, Evento as EventoNav, Repeticao};
use engine::core::partida::{self, AcaoPartida, BotaoTeclado, ConfigPartida, Partida, RespostaBotao};
use engine::formats::janela::Retangulo;
use engine::core::tela_jogo::{ConfigTelaJogo, PosicaoVista, TelaDeJogo};
use engine::core::telas::{compor, Evento, Interface, Reprodutor, Sequencia};
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::paleta::Paleta;
use engine::recursos::Recursos;
use engine::rules::amarela::{Modificadores, RespostaTecla, TabelasAmarela};
use engine::rules::estado::{ConfigSimulacao, Estado, Evento as EventoRegras};
use engine::rules::geracao::{Mundo, TabelasGeracao, TipoJogo};
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

/// Janela SDL com a resolução lógica (largura × altura) ampliada pela maior escala inteira que cabe
/// na tela (no mínimo 1), e mantida em escala inteira ao redimensionar.
struct Janela {
    sdl: sdl2::Sdl,
    canvas: sdl2::render::WindowCanvas,
}

fn abrir_janela(titulo: &str, largura: u32, altura: u32, escala: Option<u32>) -> Result<Janela, String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let escala = escala.unwrap_or_else(|| {
        video
            .display_usable_bounds(0)
            .map(|r| (r.width() / largura).min(r.height() / altura))
            .unwrap_or(2)
            .clamp(1, 4)
    });
    let janela = video
        .window(titulo, largura * escala, altura * escala)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = janela.into_canvas().present_vsync().build().map_err(|e| e.to_string())?;
    canvas.set_logical_size(largura, altura).map_err(|e| e.to_string())?;
    canvas.set_integer_scale(true)?;
    Ok(Janela { sdl, canvas })
}

fn executar(seq: Sequencia, recursos: &Recursos, interface: &Interface, cores: &[[u8; 3]], o: &Opcoes) -> Result<(), String> {
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::pixels::PixelFormatEnum;

    let (lw, lh) = seq.telas.first().map(|t| (t.largura, t.altura)).unwrap_or((640, 480));
    let Janela { sdl, mut canvas } = abrir_janela("SimAnt", lw, lh, o.escala)?;
    let criador = canvas.texture_creator();
    let mut eventos = sdl.event_pump()?;
    let mut audio = som::Audio::abrir(&sdl, recursos, &o.som);
    audio.musica_abertura();

    let mut rep = Reprodutor::new(seq);
    let mut desenhada: Option<String> = None;
    let mut textura = None;
    let inicio = Instant::now();
    let mut antes = inicio;

    while !rep.terminou() {
        for ev in eventos.poll_iter() {
            match ev {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return Ok(()),
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => rep.evento(Evento::Espaco),
                Event::KeyDown { .. } => rep.evento(Evento::OutraTecla),
                Event::MouseButtonDown { .. } => rep.evento(Evento::Clique),
                _ => {}
            }
        }
        let agora = Instant::now();
        rep.passar_tempo((agora - antes).as_millis() as u64);
        antes = agora;
        if o.sair_apos_ms.is_some_and(|ms| (agora - inicio).as_millis() as u64 >= ms) {
            return Ok(());
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
    Ok(())
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

    let origem = match (&o.save, o.novo_jogo) {
        (Some(save), _) => Some(Origem::Save(save)),
        (None, true) => {
            let n = semente(&o);
            println!("Quick Game com semente {n}");
            Some(Origem::Novo(n, TipoJogo::Rapido))
        }
        _ => None,
    };
    if let Some(origem) = origem {
        let nome = match &origem {
            Origem::Save(save) => save.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            Origem::Novo(..) => "Quick Game".to_string(),
        };
        let mut s = Sessao::abrir(&recursos, &o.dados, origem, cores)?;
        for &a in &o.acoes {
            s.partida.navegar(a);
        }
        if let Some(n) = o.ticks {
            let antes = s.partida.estado.partida.tempo;
            for _ in 0..n {
                s.partida.rodar_tick();
            }
            s.registrar_eventos();
            println!("{n} ticks: GameTime {antes} -> {}", s.partida.estado.partida.tempo);
        }
        for &b in &o.botoes {
            println!("botão {b}: {:?}", s.partida.botao_usuario(b));
        }
        for _ in 0..o.fechar_avisos {
            if let Some(Modal::FimDeJogo { .. }) = s.partida.fechar_modal() {
                s.partida.avisos.abrir_selecao_de_jogo();
            }
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
        return executar_partida(s, &format!("SimAnt - {nome}"), &o, jogo_novo);
    }

    let seq: Sequencia = recursos.dados("telas.json")?;
    let interface = Interface::carregar(&recursos, config.tabela_posicoes)?;
    match &o.exportar {
        Some(dir) => exportar(&seq, &recursos, &interface, &cores, dir),
        None => executar(seq, &recursos, &interface, &cores, &o),
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

    #[test]
    fn teclas_do_sdl_viram_acoes_de_data_navegacao() {
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
