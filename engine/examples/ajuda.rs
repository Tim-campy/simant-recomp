//! Desenha o visor de ajuda (engine::core::ajuda) numa tela de 640 × 442 e grava em PNG, para comparar
//! com as capturas do WinHelp do original. Ver docs/spec/telas/ajuda.md.
//!
//! Uso: cargo run -p engine --example ajuda -- <pasta do jogo> <saída.png> [contexto] [opções]
//!   contexto: número (0x2200, 4…); sem ele, o índice.
//!   --topico N: abre o tópico N (índice da lista) em vez de um contexto.
//!   --rolar PIXELS, --clicar X Y (repetível): entrada depois de abrir.
//!   --tecla NOME, --alt LETRA, --shift NOME, --digitar TEXTO: teclas (nomes do SDL) depois de abrir.
//!   --listar: imprime os tópicos (índice, posição e título).

use std::path::Path;

use engine::core::ajuda::VisorAjuda;
use engine::core::quadro::Quadro;
use engine::core::telas::Interface;
use engine::paleta::Paleta;
use engine::recursos::Recursos;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("uso: ajuda <pasta do jogo> <saída.png> [contexto] [--topico N] [--rolar P] [--clicar X Y] [--listar]");
        std::process::exit(2);
    }
    let bancos = ["SHARED", "WINGANT", "SOUND"].map(String::from);
    let r = Recursos::abrir(Path::new(&a[1]), &bancos, Path::new("data"), None).unwrap();
    let interface = Interface::carregar(&r, 0).unwrap();
    let cores = engine::paleta::carregar_rgb(&r).unwrap();
    let mut v = VisorAjuda::carregar(&r, &cores).unwrap();
    let num = |s: &str| s.strip_prefix("0x").map_or_else(|| s.parse::<i64>().unwrap(), |h| i64::from_str_radix(h, 16).unwrap());
    let mut i = 3;
    while i < a.len() {
        match a[i].as_str() {
            "--topico" => {
                v.abrir_topico(a[i + 1].parse::<usize>().unwrap());
                i += 1;
            }
            "--rolar" => {
                v.rolar(&interface, num(&a[i + 1]) as i32);
                i += 1;
            }
            "--clicar" => {
                let (x, y) = (num(&a[i + 1]) as i32, num(&a[i + 2]) as i32);
                println!("clique ({x}, {y}): {:?}", v.clicar(&interface, x, y, false));
                i += 2;
            }
            "--tecla" | "--alt" | "--shift" => {
                let (alt, shift) = (a[i] == "--alt", a[i] == "--shift");
                println!("tecla {}: {:?}", a[i + 1], v.tecla_com(&interface, &a[i + 1], alt, shift));
                i += 1;
            }
            "--digitar" => {
                for c in a[i + 1].chars() {
                    let nome = if c == ' ' { "Space".to_string() } else { c.to_ascii_uppercase().to_string() };
                    v.tecla_com(&interface, &nome, false, c.is_ascii_uppercase());
                }
                i += 1;
            }
            "--altura-popup" => {
                if let Some(j) = v.cfg.janelas.as_mut() {
                    j.popup.tela[1] = num(&a[i + 1]) as i16;
                }
                i += 1;
            }
            "--listar" => {
                for (k, t) in v.arquivo.topicos.iter().enumerate() {
                    println!("{k:4} {:#7x} {}", t.posicao, t.titulo_bruto);
                }
            }
            s => {
                let n = num(s) as u32;
                if !v.abrir_contexto(n) {
                    eprintln!("contexto {n:#x} não existe");
                }
            }
        }
        i += 1;
    }
    println!("tópico: {}", v.topico_atual().titulo);
    let mut q = Quadro::new(640, 442, 17);
    v.desenhar(&mut q, &interface);
    let arq = std::fs::File::create(&a[2]).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(arq), q.largura as u32, q.altura as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let rgb: Vec<u8> = q.pixels.iter().flat_map(|&c| cores.get(c as usize).copied().unwrap_or([255, 0, 255])).collect();
    enc.write_header().unwrap().write_image_data(&rgb).unwrap();
}
