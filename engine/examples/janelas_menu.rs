//! Desenha as janelas abertas pela barra de menus (Status e About) na Ã¡rea cliente (640 Ã— 442) e grava
//! em PNG com a cor 255 (magenta) fora da janela, para comparar com as capturas da bancada (Ã¡rea cliente
//! a partir da linha 38). Ver docs/spec/telas/menus.md.
//!
//! Uso: cargo run -p engine --example janelas_menu -- <pasta do jogo> <saÃ­da.png> status SAVE.ANT [NOTA...]
//!      cargo run -p engine --example janelas_menu -- <pasta do jogo> <saÃ­da.png> sobre [DESLOCAMENTO]
//!   status: as notas da coluna COLONY podem ser dadas (ex.: 97 0 100 100) e a pontuaÃ§Ã£o depois delas.

use std::path::Path;

use engine::core::barra_menus::ConfigBarraMenus;
use engine::core::quadro::Quadro;
use engine::core::sobre::{Fase, Sobre};
use engine::core::telas::Interface;
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::recursos::Recursos;
use engine::rules::estado::Estado;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let bancos = ["SHARED", "WINGANT", "SOUND"].map(String::from);
    let r = Recursos::abrir(Path::new(&a[1]), &bancos, Path::new("data"), None).unwrap();
    let interface = Interface::carregar(&r, 0).unwrap();
    let cores = engine::paleta::carregar_rgb(&r).unwrap();
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
        "medir_sobre" => {
            // Deslocamento da rolagem dos crÃ©ditos que mais bate com cada captura (texto preto contra o
            // resto, sÃ³ na Ã¡rea dos crÃ©ditos; captura da bancada com a Ã¡rea cliente na linha 38).
            let s0 = Sobre::abrir(&r, &interface, &cfg.sobre, 0, false);
            let janela = engine::core::telas::carregar_janela(&r, cfg.sobre.janela).unwrap();
            let o = janela.objetos.iter().find(|o| o.nome == cfg.sobre.objeto_figura).unwrap().retangulo;
            let g = interface.posicionar(cfg.sobre.janela, &janela).unwrap();
            let b = janela.retangulo();
            let (dx, dy) = (g.origem.0 - b.esq + cfg.sobre.ajuste[0], g.origem.1 - b.topo + cfg.sobre.ajuste[1]);
            let area = (o.esq + dx, o.topo + dy, o.dir + dx, o.base + dy);
            for cap in &a[4..] {
                let dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(cap).unwrap()));
                let mut leitor = dec.read_info().unwrap();
                let mut buf = vec![0; leitor.output_buffer_size()];
                let info = leitor.next_frame(&mut buf).unwrap();
                let bpp = info.color_type.samples();
                let preto = |x: i16, y: i16| {
                    let i = ((y as usize + 38) * 640 + x as usize) * bpp;
                    buf[i] == 0 && buf[i + 1] == 0 && buf[i + 2] == 0
                };
                let mut melhor = (0usize, 0u32);
                for d in 0..1600 {
                    let mut s = s0.clone();
                    s.fase = Fase::Rolando { deslocamento: d, desde: 0 };
                    let mut q = Quadro::new(640, 442, 255);
                    s.desenhar(&mut q, &r, &interface, &cfg.sobre).unwrap();
                    let mut ig = 0;
                    for y in area.1 + 1..area.3 - 1 {
                        for x in area.0 + 1..area.2 - 1 {
                            let p = q.pixels[y as usize * 640 + x as usize] == 0;
                            ig += usize::from(p == preto(x, y));
                        }
                    }
                    if ig > melhor.0 {
                        melhor = (ig, d);
                    }
                }
                let tot = (area.2 - area.0 - 2) as usize * (area.3 - area.1 - 2) as usize;
                println!("{cap}: deslocamento {} ({:.2}%)", melhor.1, 100.0 * melhor.0 as f64 / tot as f64);
            }
            return;
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
