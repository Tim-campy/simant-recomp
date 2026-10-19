//! Procura sementes S de entrada com as quais um tick do port, partindo de um vetor gravado no
//! original, deixa um bloco do save com um valor pedido. Serve para preparar na bancada um ramo raro
//! que depende de sorteio (a semente encontrada é escrita na entrada do mesmo tick, `SementeS`, e o
//! tick é gravado de novo: o vetor confere o ramo no original).
//! Uso: cargo run -p engine --release --example buscar_semente -- DIR_TICK SIMBOLO [VALOR|nz] [MAX]
//! Ex.: buscar_semente re_trabalho/b/tick_001711 JustToBeMean nz 5
use std::path::Path;

use engine::formats::save::LayoutSave;
use engine::rules::tick::tick;
use engine::rules::vetor_tick::{montar_entrada, regras_do_vetor, VetorTick};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 2 {
        eprintln!("uso: buscar_semente DIR_TICK SIMBOLO [VALOR|nz] [MAX]");
        std::process::exit(2);
    }
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let layout = LayoutSave::carregar(&raiz.join("data/save_layout.json")).expect("layout");
    let v = VetorTick::ler(Path::new(&a[0]), &layout).expect("vetor");
    let alvo = a.get(2).map(String::as_str).unwrap_or("nz");
    let max: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(10);
    let base = montar_entrada(&v, &layout).expect("entrada").estado;
    let mut achadas = 0;
    for s in 1..=0xFFFFu16 {
        let mut e = base.clone();
        e.s.definir_semente(s);
        let mut r = regras_do_vetor(&v);
        tick(&mut e, &mut r);
        let save = e.para_save(&layout).expect("save");
        let b = save.bloco(&a[1]).expect("símbolo");
        let valor = b.iter().rev().fold(0u64, |acc, &x| acc << 8 | u64::from(x));
        let ok = if alvo == "nz" { valor != 0 } else { valor == alvo.parse::<u64>().unwrap_or(0) };
        if ok {
            println!("semente {s:#06x}: {} = {valor}", a[1]);
            achadas += 1;
            if achadas >= max {
                break;
            }
        }
    }
    println!("{achadas} semente(s)");
}
