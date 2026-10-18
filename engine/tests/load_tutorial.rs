//! Load de "Tutorial.Ant" (`tests/referencias/load_tutorial/`, gravado na bancada por
//! `cenarios_tick.py load_tutorial`): o original não lê o arquivo e chama `_GenerateTutorial` com o R
//! da sessão. O port (`carregar::estado_do_tutorial`) parte do estado de antes e tem de chegar ao de
//! depois, bloco a bloco, com R igual; S vem de `_SeedSRand` (relógio), então usa o valor gravado.
//! Ver docs/spec/telas/arquivos.md.

use std::path::PathBuf;

use engine::core::carregar::{estado_do_tutorial, ConfigTutorialArquivo};
use engine::formats::save::LayoutSave;
use engine::recursos::Recursos;
use engine::rules::estado::ConfigSimulacao;
use engine::rules::geracao::TabelasGeracao;
use engine::rules::tick::RegrasOriginais;
use engine::rules::vetor_tick::{comparar_com, VetorTick};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn load_de_tutorial_ant_gera_o_tutorial_do_original() {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
        return;
    }
    let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    let tab = TabelasGeracao::carregar(&r).unwrap();
    let cfg = ConfigSimulacao::de_json(&std::fs::read_to_string(raiz().join("data/simulacao.json")).unwrap()).unwrap();
    let tut: ConfigTutorialArquivo = r.dados("tutorial_arquivo.json").unwrap();
    let mut regras = RegrasOriginais::carregar_sem_banco(&r).unwrap();
    let quintal = regras.quintal.clone();
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let d = raiz().join("tests/referencias/load_tutorial");
    let v = VetorTick::ler(&d, &layout).unwrap();
    let s_depois = v.extras_depois.semente_s;
    let res = comparar_com(&v, &layout, |e| {
        let antes = e.clone();
        *e = estado_do_tutorial(&antes, s_depois, &tab, &cfg, &quintal, &tut, &mut regras, (25, 19))?;
        Ok(())
    })
    .unwrap();
    println!("{}", res.placar());
    let mut falhas: Vec<String> = res.diferentes().map(|b| format!("{}: {:?}", b.simbolo, b.detalhes)).collect();
    for x in res.extras.iter().filter(|x| x.original != x.port && x.nome != "gameCycles") {
        falhas.push(format!("{}: original {}, port {}", x.nome, x.original, x.port));
    }
    assert!(falhas.is_empty(), "port difere do original:\n{}", falhas.join("\n"));
}
