//! Exporta em PNG a camada de chão dos mapas de um save `.ANT` (superfície 128×64 tiles e ninhos
//! preto e vermelho 64×64), mais folhas de contato dos tilesets, para conferência visual.
//! Uso: cargo run -p engine --example exportar_mapa -- <pasta do jogo> <save .ANT> <pasta de saída>
//! Ver docs/spec/formatos/tiles.md.

use std::path::Path;

use engine::core::mapa::{desenhar_tile, MapaChao, Plano};
use engine::core::quadro::Quadro;
use engine::formats::banco::Banco;
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::formats::tiles::{self, Tileset, LADO};
use engine::paleta::Paleta;


fn salvar_png(q: &Quadro, rgb: &[[u8; 3]], caminho: &Path) {
    let arq = std::fs::File::create(caminho).unwrap_or_else(|e| panic!("{}: {e}", caminho.display()));
    let mut enc = png::Encoder::new(std::io::BufWriter::new(arq), q.largura as u32, q.altura as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let dados: Vec<u8> = q.pixels.iter().flat_map(|&i| rgb[i as usize]).collect();
    enc.write_header().unwrap().write_image_data(&dados).unwrap();
    println!("gravado {}", caminho.display());
}

/// Folha de contato: 16 tiles por linha, 1 pixel de separação; `fundo` aparece onde for transparente.
fn folha(ts: &Tileset, transparente: Option<u8>, fundo: u8) -> Quadro {
    let linhas = ts.len().div_ceil(16);
    let mut q = Quadro::new(16 * (LADO + 1), linhas * (LADO + 1), fundo);
    for (i, t) in ts.tiles.iter().enumerate() {
        let (x, y) = ((i % 16) * (LADO + 1), (i / 16) * (LADO + 1));
        desenhar_tile(&mut q, t, x as i32, y as i32, transparente);
    }
    q
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 4 {
        eprintln!("uso: exportar_mapa <pasta do jogo> <save .ANT> <pasta de saída>");
        std::process::exit(2);
    }
    let (jogo, save, saida) = (Path::new(&a[1]), Path::new(&a[2]), Path::new(&a[3]));
    std::fs::create_dir_all(saida).unwrap();

    let caminho_paleta = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/paleta.json");
    let paleta: Paleta = serde_json::from_slice(&std::fs::read(caminho_paleta).unwrap()).unwrap();
    let rgb = paleta.rgb().unwrap();

    let banco = Banco::abrir(&jogo.join("WINGANT")).unwrap();
    let layout = LayoutSave::carregar(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/save_layout.json")).unwrap();
    let jogo = JogoSalvo::abrir(&layout, save).unwrap();
    let parte = |simbolo: &str| jogo.bloco(simbolo).unwrap_or_else(|| panic!("bloco {simbolo} ausente"));

    // _TERRAINset: 0 = quintal (9/10), 1 = casa (9/9).
    let terreno = jogo.u16("TERRAINset").unwrap();
    let id_chao = if terreno == 1 { tiles::ID_CHAO_CASA } else { tiles::ID_CHAO_QUINTAL };
    println!("_TERRAINset = {terreno}: tileset de chão 9/{id_chao}");
    let chao = tiles::carregar_chao(&banco, id_chao).unwrap();

    let mapas = [
        ("mapa_superficie.png", MapaChao::new(Plano::Superficie, parte("MapA")).unwrap()),
        // Com a camada _PherMapA (no QUICK2 ela é toda zero, então sai igual à anterior).
        (
            "mapa_superficie_feromonio.png",
            MapaChao::new(Plano::Superficie, parte("MapA")).unwrap().com_feromonio(parte("PherMapA")).unwrap(),
        ),
        ("mapa_ninho_preto.png", MapaChao::new(Plano::NinhoPreto, parte("MapB")).unwrap()),
        ("mapa_ninho_vermelho.png", MapaChao::new(Plano::NinhoVermelho, parte("MapR")).unwrap()),
    ];
    for (nome, m) in &mapas {
        let (w, h) = (m.plano.largura(), engine::core::mapa::ALTURA);
        let mut q = Quadro::new(w * LADO, h * LADO, 0);
        m.desenhar(&mut q, &chao, (0, 0, w, h), (0, 0)).unwrap();
        salvar_png(&q, &rgb, &saida.join(nome));
    }

    salvar_png(&folha(&chao, None, 0), &rgb, &saida.join(format!("tiles_chao_9_{id_chao}.png")));
    for (nome, ids) in [("superficie", tiles::IDS_VIDA_SUPERFICIE), ("ninho", tiles::IDS_VIDA_NINHO)] {
        let vida = tiles::carregar_vida(&banco, &ids).unwrap();
        // Fundo 7 (cinza escuro) para destacar o que é transparente.
        salvar_png(&folha(&vida, Some(tiles::TRANSPARENTE), 7), &rgb, &saida.join(format!("tiles_vida_{nome}.png")));
    }
}
