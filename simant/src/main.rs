//! Executável do port: janela, entrada e desenho sobre SDL2. A lógica fica no crate `engine`.
//!
//! Uso:
//!   simant [--jogo DIR] [--dados DIR] [--mods DIR] [--exportar-telas DIR]
//!
//! Padrões: --jogo SimAnt-T/MAXIS/ANTWIN, --dados data, --mods mods (relativos à pasta atual).

use std::path::{Path, PathBuf};
use std::time::Instant;

use engine::core::telas::{compor, Evento, Reprodutor, Sequencia};
use engine::paleta::Paleta;
use engine::recursos::Recursos;
use serde::Deserialize;

#[derive(Deserialize)]
struct ConfigJogo {
    bancos: Vec<String>,
}

struct Opcoes {
    jogo: PathBuf,
    dados: PathBuf,
    mods: PathBuf,
    exportar: Option<PathBuf>,
}

fn ler_opcoes() -> Result<Opcoes, String> {
    let mut o = Opcoes {
        jogo: PathBuf::from("SimAnt-T/MAXIS/ANTWIN"),
        dados: PathBuf::from("data"),
        mods: PathBuf::from("mods"),
        exportar: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut valor = || args.next().map(PathBuf::from).ok_or(format!("falta o valor de {a}"));
        match a.as_str() {
            "--jogo" => o.jogo = valor()?,
            "--dados" => o.dados = valor()?,
            "--mods" => o.mods = valor()?,
            "--exportar-telas" => o.exportar = Some(valor()?),
            _ => return Err(format!("opção desconhecida: {a}")),
        }
    }
    Ok(o)
}

fn para_rgb(indices: &[u8], cores: &[[u8; 3]]) -> Vec<u8> {
    indices.iter().flat_map(|&i| cores[i as usize & 15]).collect()
}

fn salvar_png(caminho: &Path, largura: u32, altura: u32, rgb: &[u8]) -> Result<(), String> {
    let arq = std::fs::File::create(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(arq), largura, altura);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(rgb).map_err(|e| e.to_string())
}

fn exportar(seq: &Sequencia, recursos: &Recursos, cores: &[[u8; 3]], dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for t in &seq.telas {
        let rgb = para_rgb(&compor(t, recursos)?, cores);
        let caminho = dir.join(format!("{}.png", t.nome));
        salvar_png(&caminho, t.largura, t.altura, &rgb)?;
        println!("{}", caminho.display());
    }
    Ok(())
}

fn executar(seq: Sequencia, recursos: &Recursos, cores: &[[u8; 3]]) -> Result<(), String> {
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::pixels::PixelFormatEnum;

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let (lw, lh) = seq.telas.first().map(|t| (t.largura, t.altura)).unwrap_or((640, 480));
    let janela = video
        .window("SimAnt", lw * 2, lh * 2)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = janela.into_canvas().present_vsync().build().map_err(|e| e.to_string())?;
    let criador = canvas.texture_creator();
    let mut eventos = sdl.event_pump()?;

    let mut rep = Reprodutor::new(seq);
    let mut desenhada: Option<String> = None;
    let mut textura = None;
    let mut antes = Instant::now();

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

        let Some(tela) = rep.atual() else { break };
        if desenhada.as_deref() != Some(tela.nome.as_str()) {
            let rgb = para_rgb(&compor(tela, recursos)?, cores);
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
    let seq: Sequencia = recursos.dados("telas.json")?;
    match &o.exportar {
        Some(dir) => exportar(&seq, &recursos, &cores, dir),
        None => executar(seq, &recursos, &cores),
    }
}

fn main() {
    if let Err(e) = principal() {
        eprintln!("erro: {e}");
        std::process::exit(1);
    }
}
