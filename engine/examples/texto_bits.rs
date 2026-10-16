//! Ferramenta de validação: imprime a imagem de 1 bit de um texto numa fonte do jogo ('#' = tinta).
//! Uso: cargo run -p engine --example texto_bits -- <pasta do jogo> <fonte: FONTRES.FON#Gaia8x14 | font2> <texto>
use engine::formats::fonte::{fontes_de_fon, Fonte, FonteMac};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = std::path::Path::new(&a[1]);
    let ler = |n: &str| {
        std::fs::read_dir(dir).unwrap().flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(n))
            .map(|e| std::fs::read(e.path()).unwrap()).expect("arquivo")
    };
    let fonte = match a[2].split_once('#') {
        Some((arq, face)) => Fonte::Fnt(fontes_de_fon(&ler(arq)).unwrap().into_iter().find(|f| f.nome == face).unwrap()),
        None => Fonte::Mac(FonteMac::decodificar(&ler(&a[2])).unwrap()),
    };
    let img = fonte.imagem(a[3].as_bytes());
    for y in 0..img.altura {
        let l: String = (0..img.largura).map(|x| if img.bits[y * img.largura + x] { '#' } else { '.' }).collect();
        println!("{l}");
    }
}
