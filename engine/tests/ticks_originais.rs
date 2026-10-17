//! Port × original, um tick por vez, com os vetores de `tests/referencias/ticks/` (ticks 79–83 do
//! snapshot `quick`). Protocolo e placar: docs/spec/regras/testes-tick.md.
//!
//! Por enquanto o teste exige só que nada quebre e que os blocos de [`ESPERADOS_IGUAIS`] batam.
//! Quem implementar uma parte do tick acrescenta à lista os blocos que passaram a bater (rode
//! `cargo run -p engine --example comparar_tick -- tests/referencias/ticks` para ver o relatório)
//! e atualiza a seção "Placar do port" da spec.

use std::path::PathBuf;

use engine::formats::save::LayoutSave;
use engine::rules::tick::RegrasOriginais;
use engine::rules::vetor_tick::{comparar_tick, RelatorioTick, VetorTick};

/// Blocos do save que o port já reproduz em todos os vetores.
///
/// - `GameTime`, `Cycle`: contadores do tick.
/// - `FlyAwayB/R`: contagem regressiva da revoada (o tick a decrementa quando ≠ 0).
/// - `CastePopB/R`, `BpopT`, `RpopT`: contagem (`_CountAnts`), só nos ticks ≡ 0 e 64 (mod 128);
///   nos ticks 79–83 conferem que o port não as mexe fora da fase.
const ESPERADOS_IGUAIS: &[&str] =
    &["GameTime", "Cycle", "FlyAwayB", "FlyAwayR", "CastePopB", "CastePopR", "BpopT", "RpopT"];

/// Variáveis fora do save que o port já reproduz.
const EXTRAS_ESPERADOS_IGUAIS: &[&str] = &["gameCycles", "Tindex", "GlobalKey"];

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
        for s in ESPERADOS_IGUAIS {
            let b = r.bloco(s).unwrap_or_else(|| panic!("bloco {s} não existe no layout"));
            if !b.igual() {
                falhas.push(format!("{} {s}: {:?}", r.nome, b.detalhes));
            }
        }
        for n in EXTRAS_ESPERADOS_IGUAIS {
            let x = r.extras.iter().find(|x| x.nome == *n).unwrap();
            if x.original != x.port {
                falhas.push(format!("{} {n}: original {}, port {}", r.nome, x.original, x.port));
            }
        }
    }
    assert!(falhas.is_empty(), "blocos esperados iguais diferem:\n{}", falhas.join("\n"));
}
