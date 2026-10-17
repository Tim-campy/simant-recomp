//! Ações da interface gravadas no original com o jogo pausado (`tests/referencias/acoes/*/`, gravadas
//! por `tools/cenarios_tick.py`): o port aplica a mesma ação (`vetor_tick::aplicar_acao`) ao estado de
//! antes e o resultado tem de ser o estado de depois, bloco a bloco, com S e R iguais. `_gameCycles`
//! não conta (o relógio anda com o jogo pausado). Precisa dos arquivos do jogo (SIMANT_DIR ou
//! ../SimAnt-T/MAXIS/ANTWIN) para as tabelas de geração; sem eles, avisa e passa.
//! Ver docs/spec/telas/quintal.md §8.

use std::path::PathBuf;

use engine::formats::save::LayoutSave;
use engine::recursos::Recursos;
use engine::rules::estado::ConfigSimulacao;
use engine::rules::geracao::TabelasGeracao;
use engine::rules::vetor_tick::{aplicar_acao, comparar_com, ler_acao, VetorTick};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn acoes_do_original() {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
        return;
    }
    let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    let tab = TabelasGeracao::carregar(&r).unwrap();
    let cfg = ConfigSimulacao::de_json(&std::fs::read_to_string(raiz().join("data/simulacao.json")).unwrap()).unwrap();
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(raiz().join("tests/referencias/acoes"))
        .unwrap()
        .flatten()
        .map(|d| d.path())
        .filter(|p| p.join("extras.json").exists())
        .collect();
    dirs.sort();
    assert!(!dirs.is_empty());
    let mut falhas = Vec::new();
    for d in dirs {
        let nome = d.file_name().unwrap().to_string_lossy().to_string();
        let v = VetorTick::ler(&d, &layout).unwrap();
        let acao = ler_acao(&d).unwrap();
        let r = comparar_com(&v, &layout, |e| aplicar_acao(e, &acao, &tab, &cfg).map(|_| ())).unwrap();
        println!("{nome}: {}", r.placar());
        for b in r.diferentes() {
            falhas.push(format!("{nome} {}: {:?}", b.simbolo, b.detalhes));
        }
        for x in r.extras.iter().filter(|x| x.original != x.port && x.nome != "gameCycles") {
            falhas.push(format!("{nome} {}: original {}, port {}", x.nome, x.original, x.port));
        }
        if !r.vida.is_empty() {
            falhas.push(format!("{nome}: {} células de vida diferentes", r.vida.len()));
        }
    }
    assert!(falhas.is_empty(), "port difere do original:\n{}", falhas.join("\n"));
}
