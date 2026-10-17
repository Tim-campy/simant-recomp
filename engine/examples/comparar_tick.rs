//! Comparador de tick: roda um tick do port sobre um vetor gravado no original e imprime o relatório.
//! Uso: cargo run -p engine --example comparar_tick -- tests/referencias/ticks/tick_000079 [outro_dir ...]
//! Um diretório que contenha `tick_*` (ex.: tests/referencias/ticks) roda todos e imprime o placar.
//! Protocolo: docs/spec/regras/testes-tick.md.
use std::path::{Path, PathBuf};

use engine::formats::save::LayoutSave;
use engine::rules::vetor_tick::{comparar_tick, regras_do_vetor, VetorTick};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("uso: comparar_tick <dir tick_NNNNNN | dir com tick_*> ...");
        std::process::exit(2);
    }
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let layout = LayoutSave::carregar(&raiz.join("data/save_layout.json")).expect("layout");
    let mut dirs: Vec<PathBuf> = Vec::new();
    for a in &args {
        let p = Path::new(a);
        if p.join("extras.json").exists() {
            dirs.push(p.to_path_buf());
        } else {
            let mut v: Vec<PathBuf> = std::fs::read_dir(p)
                .unwrap_or_else(|e| panic!("{a}: {e}"))
                .flatten()
                .map(|e| e.path())
                .filter(|q| q.join("extras.json").exists())
                .collect();
            v.sort();
            dirs.extend(v);
        }
    }
    let mut placar = Vec::new();
    for d in &dirs {
        let v = VetorTick::ler(d, &layout).unwrap_or_else(|e| panic!("{}: {e}", d.display()));
        let r = comparar_tick(&v, &layout, &mut regras_do_vetor(&v)).expect("comparar");
        println!("{r}");
        placar.push(r.placar());
    }
    if placar.len() > 1 {
        println!("== Placar ==");
        for p in placar {
            println!("{p}");
        }
    }
}
