//! Ferramenta de validação: pontuação de fim de jogo (`_CalcScore`) e parciais de um save.
//! Uso: cargo run -p engine --example pontuacao -- <save .ANT> [tempo]
use engine::core::avisos::{calcular_pontuacao, parciais, ConfigAvisos};
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::rules::estado::Estado;
use std::path::Path;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let layout = LayoutSave::carregar(Path::new("data/save_layout.json")).unwrap();
    let mut e = Estado::de_save(&JogoSalvo::abrir(&layout, Path::new(&a[1])).unwrap()).unwrap();
    if let Some(t) = a.get(2) {
        e.partida.tempo = t.parse().unwrap();
    }
    let cfg = ConfigAvisos::de_json(&std::fs::read_to_string("data/mensagens.json").unwrap()).unwrap();
    let c = &cfg.pontuacao;
    println!("GameTime {} MeHealth {} tipo {}", e.partida.tempo, e.amarela.saude, e.partida.tipo_jogo);
    println!("parciais {:?}", parciais(&e, c));
    println!("pontos {}", calcular_pontuacao(&e, c));
}
