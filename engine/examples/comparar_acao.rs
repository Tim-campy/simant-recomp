//! Compara uma ação da interface gravada no original com o jogo pausado (tools/cenarios_tick.py:
//! `xfer`, `place`, ...) com a mesma ação no port (`vetor_tick::aplicar_acao`). O diretório tem o
//! formato de um vetor de tick e `extras.json["acao"]` diz a ação.
//! Uso: SIMANT_DIR=<pasta do jogo> cargo run -p engine --example comparar_acao -- DIR [DIR ...]
use std::path::{Path, PathBuf};

use engine::formats::save::LayoutSave;
use engine::recursos::Recursos;
use engine::rules::estado::ConfigSimulacao;
use engine::rules::geracao::TabelasGeracao;
use engine::rules::vetor_tick::{aplicar_acao, comparar_com, ler_acao, VetorTick};

fn main() {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let layout = LayoutSave::carregar(&raiz.join("data/save_layout.json")).expect("layout");
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz.join("SimAnt-T/MAXIS/ANTWIN"));
    let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz.join("data"), None).expect("recursos (SIMANT_DIR)");
    let tab = TabelasGeracao::carregar(&r).expect("tabelas");
    let cfg = ConfigSimulacao::de_json(&std::fs::read_to_string(raiz.join("data/simulacao.json")).unwrap()).unwrap();
    for a in std::env::args().skip(1) {
        let d = Path::new(&a);
        let v = VetorTick::ler(d, &layout).unwrap_or_else(|e| panic!("{a}: {e}"));
        let acao = ler_acao(d).unwrap();
        let r = comparar_com(&v, &layout, |e| {
            println!("{acao}: {}", aplicar_acao(e, &acao, &tab, &cfg)?);
            println!("eventos: {:?}", e.retirar_eventos());
            Ok(())
        })
        .unwrap_or_else(|e| panic!("{a}: {e}"));
        println!("{r}");
    }
}
