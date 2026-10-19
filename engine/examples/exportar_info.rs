//! Exporta os cartões do Info (janela 5) em PNG e lista os registros de cada um, para conferência.
//! Ver docs/spec/telas/info.md.
//!
//! Uso: cargo run -p engine --example exportar_info -- <pasta do jogo> <pasta de saída> [opções] [id...]
//!   id: um cartão (127–179); `id:popup` abre também o popup da área de clique de valor −popup.
//!   --tela X Y: desenha numa tela 640 × 442 (área cliente da bancada) com a área do cartão em
//!               (X, Y), para comparar com capturas (tools/comparar_captura.py --mapear).
//!   --conferir: falha (código 1) se algum cartão não tem figura, texto vazio ou bitmap ilegível.
//! Sem ids, exporta os 127–179.

use std::path::Path;

use engine::core::info::{ConfigInfo, JanelaInfo, Registro, TextoCartao};
use engine::core::quadro::Quadro;
use engine::core::telas::Interface;
use engine::formats::banco::Chave;
use engine::formats::bitmap::{Bitmap, TIPO_BITMAP};
use engine::formats::janela::Retangulo;
use engine::recursos::Recursos;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("uso: exportar_info <pasta do jogo> <pasta de saída> [--tela X Y] [--conferir] [id[:popup]...]");
        std::process::exit(2);
    }
    let (jogo, saida) = (Path::new(&a[1]), Path::new(&a[2]));
    std::fs::create_dir_all(saida).unwrap();
    let mut tela: Option<(i16, i16)> = None;
    let mut conferir = false;
    let mut pedidos: Vec<(u16, Option<i16>)> = Vec::new();
    let mut i = 3;
    while i < a.len() {
        match a[i].as_str() {
            "--tela" => {
                tela = Some((a[i + 1].parse().unwrap(), a[i + 2].parse().unwrap()));
                i += 2;
            }
            "--conferir" => conferir = true,
            s => {
                let (id, p) = s.split_once(':').map_or((s, None), |(x, y)| (x, Some(y.parse().unwrap())));
                pedidos.push((id.parse().unwrap(), p));
            }
        }
        i += 1;
    }
    if pedidos.is_empty() {
        pedidos = (127..=179).map(|id| (id, None)).collect();
    }
    let dados = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data");
    let cfg: serde_json::Value = serde_json::from_slice(&std::fs::read(dados.join("jogo.json")).unwrap()).unwrap();
    let bancos: Vec<String> = serde_json::from_value(cfg["bancos"].clone()).unwrap();
    let r = Recursos::abrir(jogo, &bancos, &dados, None).unwrap();
    let rgb = engine::paleta::carregar_rgb(&r).unwrap();
    let interface = Interface::carregar(&r, 0).unwrap();
    let cfg: ConfigInfo = r.dados("info.json").unwrap();
    let mut problemas = Vec::new();
    for (id, popup) in pedidos {
        let mut j = JanelaInfo::new(cfg.clone());
        j.atual = id;
        j.carregar(&r);
        let Some(c) = j.carta.clone() else {
            problemas.push(format!("{id}: cartão não lido"));
            continue;
        };
        println!("== {id} \"{}\"", c.titulo);
        let mut figuras = 0;
        for reg in &c.registros {
            match reg {
                Registro::Figura { bitmap, x, y, areas } => {
                    figuras += 1;
                    let ok = r.objeto(Chave { tipo: TIPO_BITMAP, id: *bitmap }).and_then(|d| Bitmap::decodificar(&d)).is_ok();
                    if !ok {
                        problemas.push(format!("{id}: bitmap {bitmap} ilegível"));
                    }
                    let v: Vec<_> = areas.iter().map(|a| (a.valor, a.ret.esq, a.ret.topo, a.ret.dir, a.ret.base)).collect();
                    println!("  PICT {bitmap} em ({x}, {y}) áreas {v:?}");
                    for a in areas.iter().filter(|a| a.valor <= 0) {
                        match TextoCartao::ler(&r, &cfg, (-a.valor) as u16) {
                            Some(t) if !t.bytes.is_empty() => println!("    popup {}: {:?}", -a.valor, String::from_utf8_lossy(&t.bytes)),
                            _ => problemas.push(format!("{id}: popup {} sem texto", -a.valor)),
                        }
                    }
                }
                Registro::Texto { ret, textos } => {
                    println!("  TEXT ({}, {})–({}, {}) {:?}", ret.esq, ret.topo, ret.dir, ret.base, textos);
                    for t in textos {
                        match TextoCartao::ler(&r, &cfg, *t) {
                            Some(x) if !x.bytes.is_empty() => {
                                println!("    [{t}] {:?} trechos {:?}", String::from_utf8_lossy(&x.bytes), x.trechos)
                            }
                            _ => problemas.push(format!("{id}: texto {t} vazio")),
                        }
                    }
                }
            }
        }
        if figuras == 0 {
            problemas.push(format!("{id}: sem figura"));
        }
        let (mut q, area) = match tela {
            Some((x, y)) => (Quadro::new(640, 442, 7), Retangulo { esq: x, topo: y, dir: x + 320, base: y + 200 }),
            None => (Quadro::new(322, 202, 7), Retangulo { esq: 1, topo: 0, dir: 321, base: 201 }),
        };
        if let Some(p) = popup {
            let alvo = c.registros.iter().find_map(|reg| match reg {
                Registro::Figura { areas, .. } => areas.iter().find(|a| a.valor == -p),
                _ => None,
            });
            if let Some(a) = alvo {
                let (cx, cy) = ((a.ret.esq + a.ret.dir) / 2, (a.ret.topo + a.ret.base) / 2);
                j.clicar(i32::from(cx), i32::from(cy));
            }
        }
        j.desenhar(&mut q, &r, &interface, area);
        let nome = match popup {
            Some(p) => format!("info_{id}_{p}.png"),
            None => format!("info_{id}.png"),
        };
        let arq = std::fs::File::create(saida.join(nome)).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(arq), q.largura as u32, q.altura as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let px: Vec<u8> = q.pixels.iter().flat_map(|&i| rgb[i as usize]).collect();
        enc.write_header().unwrap().write_image_data(&px).unwrap();
    }
    for p in &problemas {
        println!("PROBLEMA {p}");
    }
    if conferir && !problemas.is_empty() {
        std::process::exit(1);
    }
}
