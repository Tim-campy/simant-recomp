//! Desenha as janelas abertas pela barra de menus (Status e About) na área cliente (640 × 442) e grava
//! em PNG com a cor 255 (magenta) fora da janela, para comparar com as capturas da bancada (área cliente
//! a partir da linha 38). Ver docs/spec/telas/menus.md.
//!
//! Uso: cargo run -p engine --example janelas_menu -- <pasta do jogo> <saída.png> status SAVE.ANT [NOTA...]
//!      cargo run -p engine --example janelas_menu -- <pasta do jogo> <saída.png> sobre [DESLOCAMENTO]
//!   status: as notas da coluna COLONY podem ser dadas (ex.: 97 0 100 100) e a pontuação depois delas.

use std::path::Path;

use engine::core::barra_menus::ConfigBarraMenus;
use engine::core::quadro::Quadro;
use engine::core::sobre::{Fase, Sobre};
use engine::core::telas::Interface;
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::paleta::Paleta;
use engine::recursos::Recursos;
use engine::rules::estado::Estado;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let bancos = ["SHARED", "WINGANT", "SOUND"].map(String::from);
    let r = Recursos::abrir(Path::new(&a[1]), &bancos, Path::new("data"), None).unwrap();
    let interface = Interface::carregar(&r, 0).unwrap();
    let cores = r.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
    let cfg: ConfigBarraMenus = r.dados(engine::core::barra_menus::ARQUIVO_CONFIG).unwrap();
    let mut q = Quadro::new(640, 442, 255);
    match a[3].as_str() {
        "status" => {
            let layout = LayoutSave::carregar(Path::new("data/save_layout.json")).unwrap();
            let e = Estado::de_save(&JogoSalvo::abrir(&layout, Path::new(&a[4])).unwrap()).unwrap();
            let avisos: engine::core::avisos::ConfigAvisos = r.dados(engine::core::avisos::ARQUIVO_CONFIG).unwrap();
            let mut t = engine::core::status::textos(&r, &cfg.status, &avisos.pontuacao, &e);
            for (i, v) in a.iter().skip(5).enumerate() {
                let i = if i < 4 { i } else { 9 };
                t[i].1 = if i < 4 { format!("{v}%").into_bytes() } else { v.clone().into_bytes() };
            }
            for (n, v) in &t {
                println!("{n}: {}", String::from_utf8_lossy(v));
            }
            println!("{:?}", engine::core::status::desenhar(&mut q, &r, &interface, &cfg.status, &t).unwrap());
        }
        "sobre" => {
            let mut s = Sobre::abrir(&r, &interface, &cfg.sobre, 0, false);
            if let Some(d) = a.get(4) {
                s.fase = Fase::Rolando { deslocamento: d.parse().unwrap(), desde: 0 };
            }
            println!("{:?}", s.desenhar(&mut q, &r, &interface, &cfg.sobre).unwrap());
        }
        o => panic!("{o}"),
    }
    let mut rgb: Vec<u8> = [255u8, 0, 255].repeat(640 * 38);
    rgb.extend(q.pixels.iter().flat_map(|&c| if c == 255 { [255, 0, 255] } else { cores[c as usize] }));
    let arq = std::fs::File::create(&a[2]).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(arq), 640, 480);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&rgb).unwrap();
}
