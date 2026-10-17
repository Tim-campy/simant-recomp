//! Exporta bitmaps do banco (objeto tipo 2) em PNG, com a paleta de data/paleta.json, para
//! conferência visual. Ids sem bitmap são avisados e pulados.
//! Uso: cargo run -p engine --example exportar_bitmaps -- <pasta do jogo> <pasta de saída> <id> [id...]
//! Um id pode ser uma faixa `A-B` (inclusive). Bancos: os de data/jogo.json.

use std::path::Path;

use engine::formats::banco::Chave;
use engine::formats::bitmap::{Bitmap, TIPO_BITMAP};
use engine::paleta::Paleta;
use engine::recursos::Recursos;

fn ids(arg: &str) -> Vec<u16> {
    match arg.split_once('-') {
        Some((a, b)) => (a.parse().unwrap()..=b.parse().unwrap()).collect(),
        None => vec![arg.parse().unwrap()],
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        eprintln!("uso: exportar_bitmaps <pasta do jogo> <pasta de saída> <id|A-B> ...");
        std::process::exit(2);
    }
    let (jogo, saida) = (Path::new(&a[1]), Path::new(&a[2]));
    std::fs::create_dir_all(saida).unwrap();
    let dados = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data");
    let cfg: serde_json::Value = serde_json::from_slice(&std::fs::read(dados.join("jogo.json")).unwrap()).unwrap();
    let bancos: Vec<String> = serde_json::from_value(cfg["bancos"].clone()).unwrap();
    let r = Recursos::abrir(jogo, &bancos, &dados, None).unwrap();
    let rgb = r.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
    for id in a[3..].iter().flat_map(|s| ids(s)) {
        let Ok(bm) = r.objeto(Chave { tipo: TIPO_BITMAP, id }).and_then(|d| Bitmap::decodificar(&d)) else {
            continue;
        };
        let caminho = saida.join(format!("bmp_{id}.png"));
        let arq = std::fs::File::create(&caminho).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(arq), bm.largura as u32, bm.altura as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let px: Vec<u8> = bm.pixels.iter().flat_map(|&i| rgb[i as usize]).collect();
        enc.write_header().unwrap().write_image_data(&px).unwrap();
        println!("{} ({} × {})", caminho.display(), bm.largura, bm.altura);
    }
}
