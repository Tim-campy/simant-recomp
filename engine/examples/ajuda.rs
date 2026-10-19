//! Desenha o visor de ajuda (engine::core::ajuda) numa tela de 640 × 442 e grava em PNG, para comparar
//! com as capturas do WinHelp do original. Ver docs/spec/telas/ajuda.md.
//!
//! Uso: cargo run -p engine --example ajuda -- <pasta do jogo> <saída.png> [contexto] [opções]
//!   contexto: número (0x2200, 4…); sem ele, o índice.
//!   --topico N: abre o tópico N (índice da lista) em vez de um contexto.
//!   --rolar PIXELS, --clicar X Y (repetível): entrada depois de abrir.
//!   --tecla NOME, --alt LETRA, --shift NOME, --digitar TEXTO: teclas (nomes do SDL) depois de abrir.
//!   --listar: imprime os tópicos (índice, posição e título).
//!   --externo E,T,D,B: contorno da janela (área cliente de 640 × 442); --maximizada; --inativa;
//!   --menu-sistema: o menu de sistema aberto (gerenciador de janelas com `ajuda.json` → `janela`);
//!   --arrasto E,T,D,B: o contorno pontilhado de um arrasto nesse retângulo.

use std::path::Path;

use engine::core::ajuda::VisorAjuda;
use engine::core::quadro::Quadro;
use engine::core::telas::Interface;
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
    let mut menu_sistema = false;
    let mut arrasto: Option<[i16; 4]> = None;
    let ret4 = |s: &str| -> [i16; 4] {
        let v: Vec<i16> = s.split(',').map(|p| p.trim().parse().unwrap()).collect();
        [v[0], v[1], v[2], v[3]]
    };
    while i < a.len() {
        match a[i].as_str() {
            "--externo" => {
                v.cfg.externo = ret4(&a[i + 1]);
                i += 1;
            }
            "--maximizada" => v.maximizada = true,
            "--inativa" => v.ativa_no_sistema = false,
            "--menu-sistema" => menu_sistema = true,
            "--arrasto" => {
                arrasto = Some(ret4(&a[i + 1]));
                i += 1;
            }
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
    if menu_sistema || arrasto.is_some() {
        use engine::core::gerenciador::{Arrasto, Estilo, Gerenciador, Metricas};
        use engine::formats::janela::Retangulo;
        let mut cfg = r.dados::<engine::core::janelas::ConfigJanelas>(engine::core::janelas::ARQUIVO_CONFIG).unwrap().gerenciador;
        if let Some(j) = &v.cfg.janela {
            j.aplicar(&mut cfg);
        }
        let mut g = Gerenciador::new(cfg, Retangulo { esq: 0, topo: 0, dir: 640, base: 442 });
        g.fase_pontilhado = v.cfg.janela.as_ref().map_or(0, |j| j.fase_pontilhado);
        let estilo = Estilo { titulo: true, menu_sistema: true, minimizar: true, maximizar: true, moldura_grossa: true, rolagem: false, modal: false };
        let m = Metricas { espessura: v.cfg.espessura_moldura, altura_titulo: v.cfg.altura_titulo, barra_rolagem: v.cfg.largura_barra };
        g.abrir(0xFFFE, v.externo(), estilo, m);
        if menu_sistema {
            g.abrir_menu(0xFFFE);
        }
        if let Some([e, t, d, b]) = arrasto {
            let contorno = Retangulo { esq: e, topo: t, dir: d, base: b };
            g.arrasto = Some(Arrasto::Mover { id: 0xFFFE, inicio: (0, 0), contorno, por_teclado: false });
        }
        let fonte = g.cfg.menu_sistema.fontes.iter().find_map(|&k| interface.fontes.get(k).and_then(|f| f.as_ref()));
        g.desenhar_por_cima(&mut q, fonte, false);
    }

    let arq = std::fs::File::create(&a[2]).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(arq), q.largura as u32, q.altura as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let rgb: Vec<u8> = q.pixels.iter().flat_map(|&c| cores.get(c as usize).copied().unwrap_or([255, 0, 255])).collect();
    enc.write_header().unwrap().write_image_data(&rgb).unwrap();
}
