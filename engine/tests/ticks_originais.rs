//! Port × original, um tick por vez, com os vetores de `tests/referencias/ticks/` (ticks 79–83 do
//! snapshot `quick`). Protocolo e placar: docs/spec/regras/testes-tick.md.
//!
//! Desde 2026-10-17 (todas as partes do tick implementadas) o teste exige igualdade TOTAL: os 307
//! blocos do save, os extras (S, R, `_gameCycles`, `_Tindex`, `_GlobalKey`) e a vida. Para ver o
//! relatório de uma falha: `cargo run -p engine --example comparar_tick -- tests/referencias/ticks`.

use std::path::PathBuf;

use engine::formats::save::LayoutSave;
use engine::rules::tick::RegrasOriginais;
use engine::rules::vetor_tick::{comparar_tick, RelatorioTick, VetorTick};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn relatorios() -> Vec<RelatorioTick> {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let dir = raiz().join("tests/referencias/ticks");
    let indice: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("indice.json")).unwrap()).unwrap();
    let ticks = indice["ticks"].as_array().expect("indice.json sem \"ticks\"");
    assert!(!ticks.is_empty());
    ticks
        .iter()
        .map(|t| {
            let v = VetorTick::ler(&dir.join(t.as_str().unwrap()), &layout).unwrap();
            comparar_tick(&v, &layout, &mut RegrasOriginais::default()).unwrap()
        })
        .collect()
}

#[test]
fn ticks_do_original() {
    let mut falhas = Vec::new();
    for r in relatorios() {
        println!("{}", r.placar());
        assert!(r.ida_e_volta.is_empty(), "{}: ida e volta do save falhou em {:?}", r.nome, r.ida_e_volta);
        // A reconstrução da vida a partir das listas bate com o original (conferido nos 5 vetores).
        assert!(r.vida_reconstruida.is_empty(), "{}: vida reconstruída ≠ vida_antes", r.nome);
        assert!(r.vida_reconstruida_depois.is_empty(), "{}: vida reconstruída ≠ vida_depois", r.nome);
        for b in r.diferentes() {
            falhas.push(format!("{} {}: {:?}", r.nome, b.simbolo, b.detalhes));
        }
        for x in r.extras.iter().filter(|x| x.original != x.port) {
            falhas.push(format!("{} {}: original {}, port {}", r.nome, x.nome, x.original, x.port));
        }
        if !r.vida.is_empty() {
            falhas.push(format!("{}: {} células de vida diferentes", r.nome, r.vida.len()));
        }
    }
    assert!(falhas.is_empty(), "port difere do original:\n{}", falhas.join("\n"));
}
