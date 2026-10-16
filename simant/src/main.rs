//! Executável do port: janela, entrada e desenho sobre SDL2. A lógica fica no crate `engine`; aqui só
//! se traduzem eventos do SDL em eventos do engine e se mostra o quadro composto.
//!
//! Uso:
//!   simant [--jogo DIR] [--dados DIR] [--mods DIR] [--exportar-telas DIR]
//!   simant --save ARQ.ANT [--exportar-tela-jogo ARQ.png] [--acoes a,b,...]
//!   opções da janela: [--escala N] [--sair-apos-ms N [--capturar-janela ARQ.png]]
//!
//! Padrões: --jogo $SIMANT_DIR ou SimAnt-T/MAXIS/ANTWIN, --dados data, --mods mods (relativos à
//! pasta atual). Sem --save, mostra a abertura; com --save, a tela de jogo do save.

use std::path::{Path, PathBuf};
use std::time::Instant;

use engine::core::navegacao::{Acao, ConfigNavegacao, Evento as EventoNav, Navegacao, Repeticao};
use engine::core::tela_jogo::{ConfigTelaJogo, TelaDeJogo};
use engine::core::telas::{compor, Evento, Interface, Reprodutor, Sequencia};
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::paleta::Paleta;
use engine::recursos::Recursos;
use serde::Deserialize;

#[derive(Deserialize)]
struct ConfigJogo {
    bancos: Vec<String>,
    /// Id do objeto tipo 9 com as posições das janelas (docs/spec/formatos/janela.md).
    tabela_posicoes: u16,
}

struct Opcoes {
    jogo: PathBuf,
    dados: PathBuf,
    mods: PathBuf,
    exportar: Option<PathBuf>,
    save: Option<PathBuf>,
    exportar_tela_jogo: Option<PathBuf>,
    /// Ações aplicadas à navegação antes de mostrar ou exportar (nomes de data/navegacao.json).
    acoes: Vec<Acao>,
    /// Fecha a janela sozinha depois desse tempo (para testes sem interação).
    sair_apos_ms: Option<u64>,
    /// Com --sair-apos-ms, grava o que a janela mostra (resolução da janela) antes de fechar.
    capturar_janela: Option<PathBuf>,
    /// Escala inteira da janela; sem ela, a maior que cabe na área útil do monitor.
    escala: Option<u32>,
}

fn ler_acoes(texto: &str) -> Result<Vec<Acao>, String> {
    texto
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| serde_json::from_value(serde_json::Value::String(s.trim().into())).map_err(|_| format!("ação desconhecida: {s}")))
        .collect()
}

fn ler_opcoes() -> Result<Opcoes, String> {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(PathBuf::from("SimAnt-T/MAXIS/ANTWIN"));
    let mut o = Opcoes {
        jogo,
        dados: PathBuf::from("data"),
        mods: PathBuf::from("mods"),
        exportar: None,
        save: None,
        exportar_tela_jogo: None,
        acoes: Vec::new(),
        sair_apos_ms: None,
        capturar_janela: None,
        escala: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut valor = || args.next().ok_or(format!("falta o valor de {a}"));
        match a.as_str() {
            "--jogo" => o.jogo = valor()?.into(),
            "--dados" => o.dados = valor()?.into(),
            "--mods" => o.mods = valor()?.into(),
            "--exportar-telas" => o.exportar = Some(valor()?.into()),
            "--save" => o.save = Some(valor()?.into()),
            "--exportar-tela-jogo" => o.exportar_tela_jogo = Some(valor()?.into()),
            "--acoes" => o.acoes = ler_acoes(&valor()?)?,
            "--sair-apos-ms" => o.sair_apos_ms = Some(valor()?.parse().map_err(|_| format!("{a}: número inválido"))?),
            "--escala" => o.escala = Some(valor()?.parse().ok().filter(|&e| e > 0).ok_or(format!("{a}: inteiro positivo"))?),
            "--capturar-janela" => o.capturar_janela = Some(valor()?.into()),
            _ => return Err(format!("opção desconhecida: {a}")),
        }
    }
    if o.save.is_none() && (o.exportar_tela_jogo.is_some() || !o.acoes.is_empty()) {
        return Err("--exportar-tela-jogo e --acoes precisam de --save".into());
    }
    Ok(o)
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

/// Tudo o que a tela de jogo precisa: o save, a tela carregada e a configuração de navegação.
struct Partida<'a> {
    recursos: &'a Recursos,
    interface: Interface,
    tela: TelaDeJogo,
    nav_cfg: ConfigNavegacao,
    nav: Navegacao,
    cores: Vec<[u8; 3]>,
}

impl Partida<'_> {
    fn abrir<'a>(recursos: &'a Recursos, dados: &Path, save: &Path, cores: Vec<[u8; 3]>) -> Result<Partida<'a>, String> {
        let cfg: ConfigTelaJogo = recursos.dados("tela_jogo.json")?;
        let nav_cfg: ConfigNavegacao = recursos.dados("navegacao.json")?;
        let interface = Interface::carregar(recursos, cfg.tabela_posicoes)?;
        let layout = LayoutSave::carregar(&dados.join("save_layout.json"))?;
        let jogo = JogoSalvo::abrir(&layout, save)?;
        let tela = TelaDeJogo::carregar(&cfg, recursos, &interface, &jogo)?;
        let nav = tela.navegacao(&jogo, nav_cfg.posicao_inicial)?;
        Ok(Partida { recursos, interface, tela, nav_cfg, nav, cores })
    }

    fn quadro_rgb(&self) -> Result<Vec<u8>, String> {
        let (q, pendentes) = self.tela.compor(self.recursos, &self.interface, &self.nav)?;
        if !pendentes.is_empty() {
            eprintln!("tela de jogo: não desenhado ainda: {}", pendentes.join(", "));
        }
        Ok(para_rgb(&q.pixels, &self.cores))
    }

    /// Ação do clique esquerdo em (x, y) da área cliente: seta da barra de rolagem ou botão da faixa.
    fn acao_do_clique(&self, x: i32, y: i32) -> Option<Acao> {
        self.tela
            .seta_em(x, y)
            .or_else(|| self.tela.objetos_da_faixa_em(x, y).iter().find_map(|n| self.nav_cfg.botoes_faixa.get(*n).copied()))
    }
}

/// Tecla do SDL → evento do engine (nome da tecla no SDL e modificadores, esquerdo ou direito).
fn evento_de_tecla(k: sdl2::keyboard::Keycode, m: sdl2::keyboard::Mod) -> EventoNav {
    use sdl2::keyboard::Mod;
    EventoNav::Tecla {
        nome: k.name(),
        ctrl: m.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
        shift: m.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
        alt: m.intersects(Mod::LALTMOD | Mod::RALTMOD),
    }
}

fn executar_partida(mut p: Partida, titulo: &str, o: &Opcoes) -> Result<(), String> {
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::mouse::MouseButton;
    use sdl2::pixels::PixelFormatEnum;

    let (lw, lh) = (p.tela.cfg.largura, p.tela.cfg.altura);
    let Janela { sdl, mut canvas } = abrir_janela(titulo, lw, lh, o.escala)?;
    let criador = canvas.texture_creator();
    let mut textura = criador.create_texture_static(PixelFormatEnum::RGB24, lw, lh).map_err(|e| e.to_string())?;
    let mut eventos = sdl.event_pump()?;
    let mut repeticao = Repeticao::default();
    let mut redesenhar = true;
    let inicio = Instant::now();
    let mut antes = inicio;

    loop {
        for ev in eventos.poll_iter() {
            match ev {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return Ok(()),
                Event::KeyDown { keycode: Some(k), keymod, .. } => {
                    if let Some(a) = p.nav_cfg.acao_da_tecla(&evento_de_tecla(k, keymod)) {
                        redesenhar |= p.nav.aplicar(a);
                    }
                }
                Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => {
                    if let Some(a) = p.acao_do_clique(x, y) {
                        redesenhar |= p.nav.aplicar(a);
                        repeticao.pressionar(a);
                    }
                }
                Event::MouseButtonUp { mouse_btn: MouseButton::Left, .. } => repeticao.soltar(),
                _ => {}
            }
        }
        let agora = Instant::now();
        if let Some((a, n)) = repeticao.passar_tempo((agora - antes).as_millis() as u64, &p.nav_cfg.repeticao) {
            for _ in 0..n {
                redesenhar |= p.nav.aplicar(a);
            }
        }
        antes = agora;
        if redesenhar {
            let rgb = p.quadro_rgb()?;
            textura.update(None, &rgb, lw as usize * 3).map_err(|e| e.to_string())?;
            redesenhar = false;
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
            return Ok(());
        }
        canvas.present();
    }
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

    if let Some(save) = &o.save {
        let mut p = Partida::abrir(&recursos, &o.dados, save, cores)?;
        for &a in &o.acoes {
            p.nav.aplicar(a);
        }
        if let Some(png) = &o.exportar_tela_jogo {
            let rgb = p.quadro_rgb()?;
            salvar_png(png, p.tela.cfg.largura, p.tela.cfg.altura, &rgb)?;
            let v = &p.nav.vista;
            println!("{} (plano {}, vista em ({}, {}), {} × {} tiles)", png.display(), p.nav.plano_mapa, v.x, v.y, v.largura, v.altura);
            return Ok(());
        }
        let nome = save.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        return executar_partida(p, &format!("SimAnt - {nome}"), &o);
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
    fn lista_de_acoes_da_linha_de_comando() {
        assert_eq!(ler_acoes("superficie, linha_direita").unwrap(), vec![Acao::Superficie, Acao::LinhaDireita]);
        assert!(ler_acoes("voar").is_err());
    }
}
