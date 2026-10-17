//! Regressão longa: sequências de ticks consecutivos do original (`tests/referencias/ticks_longos/*/`,
//! gravadas por `tools/cenarios_tick.py gravar longo`) rodadas encadeadas no port. Confere S e R depois de
//! cada tick e o save inteiro e a vida a cada ponto gravado (`depois_NNNN/`). Ver
//! docs/spec/regras/testes-tick.md, "Sequência longa".

use std::path::PathBuf;

use engine::formats::save::LayoutSave;
use engine::rules::tick::RegrasOriginais;
use engine::rules::vetor_tick::rodar_sequencia;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn sequencias_longas_do_original() {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let base = raiz().join("tests/referencias/ticks_longos");
    let mut dirs: Vec<PathBuf> =
        std::fs::read_dir(&base).unwrap().flatten().map(|d| d.path()).filter(|p| p.join("extras.json").exists()).collect();
    dirs.sort();
    assert!(!dirs.is_empty(), "nenhuma sequência em {}", base.display());
    let mut falhas = Vec::new();
    for d in dirs {
        let nome = d.file_name().unwrap().to_string_lossy().to_string();
        let r = rodar_sequencia(&d, &layout, &mut RegrasOriginais::default()).unwrap();
        println!("{nome}: {} ticks, {} pontos, S/R diferentes a partir de {:?}", r.ticks, r.pontos.len(), r.primeiro_sorteio_diferente);
        if let Some(k) = r.primeiro_sorteio_diferente {
            falhas.push(format!("{nome}: S ou R diferem depois do tick {k}"));
        }
        for p in &r.pontos {
            for b in &p.blocos {
                falhas.push(format!("{nome} depois de {} ticks: {} {:?}", p.ticks, b.simbolo, b.detalhes));
            }
            if !p.vida.is_empty() {
                falhas.push(format!("{nome} depois de {} ticks: {} células de vida diferentes", p.ticks, p.vida.len()));
            }
        }
    }
    assert!(falhas.is_empty(), "port difere do original:\n{}", falhas.join("\n"));
}
