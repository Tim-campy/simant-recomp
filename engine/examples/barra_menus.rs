//! Desenha a barra de menus (engine::core::barra_menus) num quadro do tamanho da tela da bancada (640 × 480)
//! com a barra na linha 19, como no original, e grava em PNG; fora da barra e do menu aberto, o quadro fica
//! na cor 255 (ignorada pela comparação). Ver docs/spec/telas/menus.md.
//!
//! Uso: cargo run -p engine --example barra_menus -- <pasta do jogo> <saída.png> [estado] [opções]
//!   estado: "fechado", "barra N" (título N realçado) ou "menu N [ITEM]" (menu N aberto, item realçado)
//!   --opcoes 0,1,1,1,1,0 (marcas do Options), --velocidade V, --pausado
//!   --moldura: desenha também a barra de título da janela principal (engine::core::moldura_principal),
//!   com o menu de sistema como o menu 5; --inativa (título inativo), --apertado min|rest (botão apertado)

use std::path::Path;

use engine::core::barra_menus::{BarraMenus, ConfigBarraMenus, Estado, EstadoMenus};
use engine::core::moldura_principal::{ConfigMolduraPrincipal, EstadoTitulo, ParteTitulo};
use engine::core::quadro::Quadro;
use engine::core::telas::Interface;
use engine::formats::banco::Chave;
use engine::formats::menu::{MenuBanco, TIPO_MENU};
use engine::recursos::Recursos;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("uso: barra_menus <pasta do jogo> <saída.png> [fechado | barra N | menu N [ITEM]] [--opcoes ...] [--velocidade V] [--pausado] [--moldura [--inativa] [--apertado min|rest]]");
        std::process::exit(2);
    }
    let bancos = ["SHARED", "WINGANT", "SOUND"].map(String::from);
    let r = Recursos::abrir(Path::new(&a[1]), &bancos, Path::new("data"), None).unwrap();
    let interface = Interface::carregar(&r, 0).unwrap();
    let cores = engine::paleta::carregar_rgb(&r).unwrap();
    let cfg: ConfigBarraMenus = r.dados(engine::core::barra_menus::ARQUIVO_CONFIG).unwrap();
    let menu = MenuBanco::decodificar(&r.objeto(Chave { tipo: TIPO_MENU, id: cfg.objeto_menu }).unwrap()).unwrap();
    let fonte = cfg.fontes.iter().find_map(|&i| interface.fontes.get(i).and_then(|f| f.as_ref()));
    let mut b = BarraMenus::new(cfg, menu);
    let mut e = EstadoMenus { opcoes: [false, true, true, true, true, false], velocidade: 1, pausado: false };
    let mut moldura: Option<(ConfigMolduraPrincipal, EstadoTitulo)> = None;
    let mut i = 3;
    while i < a.len() {
        match a[i].as_str() {
            "fechado" => {}
            "barra" => {
                b.estado = Estado::Barra(a[i + 1].parse().unwrap());
                i += 1;
            }
            "menu" => {
                let menu = a[i + 1].parse().unwrap();
                let item = a.get(i + 2).and_then(|s| s.parse().ok());
                b.estado = Estado::Aberto { menu, item };
                i += 1 + usize::from(item.is_some());
            }
            "--opcoes" => {
                for (k, v) in a[i + 1].split(',').enumerate() {
                    e.opcoes[k] = v == "1";
                }
                i += 1;
            }
            "--velocidade" => {
                e.velocidade = a[i + 1].parse().unwrap();
                i += 1;
            }
            "--pausado" => e.pausado = true,
            "--moldura" => {
                let m: ConfigMolduraPrincipal = r.dados(engine::core::moldura_principal::ARQUIVO).unwrap();
                b.ligar_sistema(&m, 640);
                moldura = Some((m, EstadoTitulo { ativa: true, ..Default::default() }));
            }
            "--inativa" => moldura.as_mut().expect("--moldura antes").1.ativa = false,
            "--apertado" => {
                let p = if a[i + 1] == "min" { ParteTitulo::Minimizar } else { ParteTitulo::Restaurar };
                moldura.as_mut().expect("--moldura antes").1.apertado = Some(p);
                i += 1;
            }
            s => panic!("argumento {s}"),
        }
        i += 1;
    }
    let rgb: Vec<u8> = match &moldura {
        Some((m, et)) => {
            // Janela inteira: título nas linhas 0–18, barra a partir da 19.
            let mut q = Quadro::new(640, 480, 255);
            let et = EstadoTitulo { caixa_realcada: b.sistema_realcado(), ..*et };
            let ft = m.fontes.iter().find_map(|&i| interface.fontes.get(i).and_then(|f| f.as_ref()));
            m.desenhar(&mut q, ft, &et);
            b.desenhar(&mut q, fonte, &e);
            q.pixels.iter().flat_map(|&c| if c == 255 { [255, 0, 255] } else { cores[c as usize] }).collect()
        }
        None => {
            // A barra começa na linha 19 da tela da bancada (abaixo do título da janela principal).
            let topo = 19;
            let mut q = Quadro::new(640, 480 - topo, 255);
            b.desenhar(&mut q, fonte, &e);
            let mut rgb: Vec<u8> = [255u8, 0, 255].repeat(640 * topo);
            rgb.extend(q.pixels.iter().flat_map(|&c| if c == 255 { [255, 0, 255] } else { cores[c as usize] }));
            rgb
        }
    };
    let arq = std::fs::File::create(&a[2]).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(arq), 640, 480);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&rgb).unwrap();
}
