//! Gera a tela de jogo (faixa + janela de edição com chão e vida) de um save `.ANT` e, opcionalmente,
//! compara com uma captura 640×480 da bancada QEMU.
//! Uso: cargo run -p engine --example tela_jogo -- <pasta do jogo> <save .ANT> <pasta de saída> [captura.png]
//! Ver docs/spec/telas/jogo.md, seção "Verificação".
//!
//! A comparação mapeia cada índice de cor do port para a cor RGB mais frequente da captura nos pixels
//! com esse índice (a QEMU mostra algumas cores da paleta com outros valores) e ignora o diálogo
//! "Saved correctly" de `qi_14_salvo.png`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use engine::core::gerenciador::Rolagem;
use engine::core::quadro::Quadro;
use engine::core::tela_jogo::{compor_tela_de_jogo, ConfigTelaJogo, TelaDeJogo};
use engine::core::telas::Interface;
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::paleta::Paleta;
use engine::recursos::Recursos;

/// Linha da captura onde começa a área cliente da janela principal (abaixo do menu).
const TOPO: usize = 38;
/// Diálogo "Saved correctly" de qi_14_salvo.png, em coordenadas da tela: (x0, y0, x1, y1) exclusivo.
const DIALOGO: (usize, usize, usize, usize) = (176, 171, 472, 310);

type Rgb = [u8; 3];

fn ler_png(caminho: &Path) -> (usize, usize, Vec<Rgb>) {
    let dec = png::Decoder::new(std::fs::File::open(caminho).unwrap_or_else(|e| panic!("{}: {e}", caminho.display())));
    let mut r = dec.read_info().unwrap();
    let mut buf = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).unwrap();
    let bpp = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        t => panic!("PNG {t:?} não suportado"),
    };
    let px = buf[..info.buffer_size()].chunks(bpp).map(|c| [c[0], c[1], c[2]]).collect();
    (info.width as usize, info.height as usize, px)
}

fn salvar_png(largura: usize, altura: usize, px: &[Rgb], caminho: &Path) {
    let arq = std::fs::File::create(caminho).unwrap_or_else(|e| panic!("{}: {e}", caminho.display()));
    let mut enc = png::Encoder::new(std::io::BufWriter::new(arq), largura as u32, altura as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let dados: Vec<u8> = px.iter().flatten().copied().collect();
    enc.write_header().unwrap().write_image_data(&dados).unwrap();
    println!("gravado {}", caminho.display());
}

/// Recorte ampliado (`escala` vezes) de uma imagem RGB.
fn ampliar(px: &[Rgb], largura: usize, r: (usize, usize, usize, usize), escala: usize) -> (usize, usize, Vec<Rgb>) {
    let (x0, y0, x1, y1) = r;
    let (w, h) = ((x1 - x0) * escala, (y1 - y0) * escala);
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            out.push(px[(y0 + y / escala) * largura + x0 + x / escala]);
        }
    }
    (w, h, out)
}

struct Regiao {
    nome: &'static str,
    /// (x0, y0, x1, y1) exclusivo, em coordenadas da tela.
    caixa: (usize, usize, usize, usize),
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        eprintln!("uso: tela_jogo <pasta do jogo> <save .ANT> <pasta de saída> [captura.png]");
        std::process::exit(2);
    }
    let (dir_jogo, save, saida) = (Path::new(&a[1]), Path::new(&a[2]), Path::new(&a[3]));
    std::fs::create_dir_all(saida).unwrap();
    let dados = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data");

    let jogo_json: serde_json::Value = serde_json::from_slice(&std::fs::read(dados.join("jogo.json")).unwrap()).unwrap();
    let bancos: Vec<String> = serde_json::from_value(jogo_json["bancos"].clone()).unwrap();
    let recursos = Recursos::abrir(dir_jogo, &bancos, &dados, None).unwrap();
    let cfg: ConfigTelaJogo = recursos.dados("tela_jogo.json").unwrap();
    let interface = Interface::carregar(&recursos, cfg.tabela_posicoes).unwrap();
    let layout = LayoutSave::carregar(&dados.join("save_layout.json")).unwrap();
    let jogo = JogoSalvo::abrir(&layout, save).unwrap();

    // PALPOS=k: índice dos palpos da aranha (o S1(4) do último redesenho do original).
    let (q, pendentes) = match std::env::var("PALPOS").ok().and_then(|v| v.parse::<u8>().ok()) {
        None => compor_tela_de_jogo(&cfg, &recursos, &interface, &jogo).unwrap(),
        Some(k) => {
            let mut tela = TelaDeJogo::carregar(&cfg, &recursos, &interface, &jogo).unwrap();
            tela.aranha.palpos = k;
            let nav = tela.navegacao(&jogo, cfg.edicao.posicao).unwrap();
            let (mx, my) = nav.maximo();
            tela.estado_moldura.rolagem = [Rolagem { pos: nav.vista.x, max: mx }, Rolagem { pos: nav.vista.y, max: my }];
            tela.compor(&recursos, &interface, &nav).unwrap()
        }
    };
    println!("pendências: {pendentes:?}");
    let paleta: Paleta = recursos.dados("paleta.json").unwrap();
    let rgb = paleta.rgb().unwrap();
    let px: Vec<Rgb> = q.pixels.iter().map(|&i| rgb[i as usize]).collect();
    salvar_png(q.largura, q.altura, &px, &saida.join("tela_jogo.png"));

    if let Some(captura) = a.get(4) {
        // região da aranha: ±48 pixels em volta do centro (_SpidX/_SpidY na vista do save)
        let v = |n: &str| jogo.i16(n).unwrap() as i32;
        let (mx, my) = jogo.par_i16("MapPnt").unwrap();
        let (cx, cy) = (24 + v("SpidX") - 16 * mx as i32, 158 + v("SpidY") - 16 * my as i32);
        let caixa = ((cx - 48).max(24) as usize, (cy - 48).max(158) as usize, (cx + 48).min(424) as usize, (cy + 48).min(462) as usize);
        comparar(&q, Path::new(captura), saida, caixa);
    }
}

fn comparar(q: &Quadro, captura: &Path, saida: &Path, aranha: (usize, usize, usize, usize)) {
    let (cw, ch, c) = ler_png(captura);
    assert!(cw == q.largura && ch == q.altura + TOPO, "captura {cw}x{ch} não bate com a tela {}x{}", q.largura, q.altura);
    let no_dialogo = |x: usize, y: usize| x >= DIALOGO.0 && x < DIALOGO.2 && y >= DIALOGO.1 && y < DIALOGO.3;
    let cap = |x: usize, y: usize| c[(y + TOPO) * cw + x];

    // Mapeamento índice -> cor mais frequente da captura (fora do diálogo).
    let mut contagem: HashMap<(u8, Rgb), usize> = HashMap::new();
    for y in 0..q.altura {
        for x in 0..q.largura {
            if !no_dialogo(x, y + TOPO) {
                *contagem.entry((q.pixels[y * q.largura + x], cap(x, y))).or_default() += 1;
            }
        }
    }
    let mut mapa: HashMap<u8, (Rgb, usize, usize)> = HashMap::new();
    for (&(i, cor), &n) in &contagem {
        let e = mapa.entry(i).or_insert((cor, 0, 0));
        e.2 += n;
        if n > e.1 {
            e.0 = cor;
            e.1 = n;
        }
    }
    let mut indices: Vec<_> = mapa.iter().collect();
    indices.sort();
    println!("mapeamento índice do port -> cor da captura (pixels com a cor / pixels com o índice):");
    for (i, (cor, n, total)) in indices {
        println!("  {i:2} -> {cor:?}  {n}/{total}");
    }

    let regioes = [
        Regiao { nome: "faixa (janela 34)", caixa: (0, TOPO, 640, TOPO + 73) },
        Regiao { nome: "janela de edição (moldura + mapa)", caixa: (20, 135, 444, 480) },
        Regiao { nome: "mapa (área visível do editTileArea)", caixa: (24, 158, 424, 462) },
        Regiao { nome: "tela inteira", caixa: (0, TOPO, 640, 480) },
        Regiao { nome: "aranha (±48 pixels do centro)", caixa: aranha },
    ];
    let igual = |x: usize, y: usize| mapa.get(&q.pixels[(y - TOPO) * q.largura + x]).map(|m| m.0) == Some(cap(x, y - TOPO));
    for r in &regioes {
        let (mut iguais, mut total) = (0usize, 0usize);
        for y in r.caixa.1..r.caixa.3 {
            for x in r.caixa.0..r.caixa.2 {
                if !no_dialogo(x, y) {
                    total += 1;
                    iguais += igual(x, y) as usize;
                }
            }
        }
        println!("{}: {:.2}% iguais ({iguais}/{total}, fora do diálogo)", r.nome, 100.0 * iguais as f64 / total as f64);
    }

    // Imagens para conferência: port com o mapeamento, diferenças em vermelho e recortes ampliados.
    let mut tela = c.clone();
    let mut diff = c.clone();
    for y in TOPO..ch {
        for x in 0..cw {
            let i = q.pixels[(y - TOPO) * q.largura + x];
            tela[y * cw + x] = mapa.get(&i).map(|m| m.0).unwrap_or([255, 0, 255]);
            let d = &mut diff[y * cw + x];
            *d = if no_dialogo(x, y) {
                [d[0] / 4, d[1] / 4, d[2] / 4]
            } else if igual(x, y) {
                [d[0] / 3, d[1] / 3, d[2] / 3]
            } else {
                [255, 0, 0]
            };
        }
    }
    salvar_png(cw, ch, &tela, &saida.join("tela_jogo_640x480_mapeada.png"));
    salvar_png(cw, ch, &diff, &saida.join("diferencas.png"));
    let recortes = [("formigas", (200, 290, 280, 330), 6), ("faixa", (0, TOPO, 320, TOPO + 73), 3), ("aranha", aranha, 5)];
    for (nome, caixa, escala) in recortes {
        for (sufixo, img) in [("port", &tela), ("captura", &c)] {
            let (w, h, a) = ampliar(img, cw, caixa, escala);
            salvar_png(w, h, &a, &saida.join(format!("{nome}_{sufixo}.png")));
        }
    }
}
