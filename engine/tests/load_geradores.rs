//! File → Load: os geradores S e R depois de `_LoadGame` (`_RandYard` antes de ler os blocos),
//! conferidos na bancada QEMU (roteiro tests/referencias/roteiros/qemu_load_rng.txt,
//! docs/spec/telas/arquivos.md).

use std::path::PathBuf;

use engine::core::carregar::geradores_depois_do_load;
use engine::recursos::Recursos;
use engine::rules::geracao::{TabelasGeracao, TipoJogo};
use engine::rules::rng::GeradorR;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn tabelas() -> Option<TabelasGeracao> {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
        return None;
    }
    let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    Some(TabelasGeracao::carregar(&r).unwrap())
}

/// Bancada (2026-10-18): Quick Game do snapshot "quick" pausado, R = 0x568a9979 antes do Load de
/// `PORT50.ANT`; logo depois do Load (ainda pausado), S = 0x98b9 e R = 0x7e80df39.
#[test]
fn geradores_do_load_batem_com_a_bancada() {
    let Some(tab) = tabelas() else { return };
    let (s, r) = geradores_depois_do_load(GeradorR::de_estado(0x568a_9979), TipoJogo::Rapido, &tab).unwrap();
    assert_eq!(r.estado(), 0x7e80_df39);
    assert_eq!(s.semente(), 0x98b9);
}
