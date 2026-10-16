//! Tilesets e composição de mapas com os arquivos originais (não versionados).
//! Pasta do jogo: SIMANT_DIR ou ../SimAnt-T/MAXIS/ANTWIN; save: tests/referencias/saves/QUICK2.ANT.
//! Sem os arquivos, os testes avisam e passam sem verificar nada. Ver docs/spec/formatos/tiles.md.

use std::path::PathBuf;

use engine::formats::save::{JogoSalvo, LayoutSave};

use engine::core::mapa::{MapaChao, Plano, ALTURA};
use engine::core::quadro::Quadro;
use engine::formats::banco::Banco;
use engine::formats::tiles::{self, LADO, TRANSPARENTE};
use sha2::{Digest, Sha256};

fn wingant() -> Option<Banco> {
    let d = std::env::var_os("SIMANT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../SimAnt-T/MAXIS/ANTWIN"));
    if d.join("WINGANT.NDX").is_file() {
        Some(Banco::abrir(&d.join("WINGANT")).unwrap())
    } else {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", d.display());
        None
    }
}

fn quick2() -> Option<Vec<u8>> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/referencias/saves/QUICK2.ANT");
    match std::fs::read(&p) {
        Ok(d) => Some(d),
        Err(_) => {
            eprintln!("AVISO: {} não encontrado; teste ignorado", p.display());
            None
        }
    }
}

fn hex(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn tilesets_de_chao_tem_256_tiles() {
    let Some(b) = wingant() else { return };
    for id in [tiles::ID_CHAO_QUINTAL, tiles::ID_CHAO_CASA] {
        assert_eq!(tiles::carregar_chao(&b, id).unwrap().len(), 256, "9/{id}");
    }
}

#[test]
fn tilesets_de_vida_tem_384_tiles_com_transparencia() {
    let Some(b) = wingant() else { return };
    for ids in [tiles::IDS_VIDA_SUPERFICIE, tiles::IDS_VIDA_NINHO] {
        let t = tiles::carregar_vida(&b, &ids).unwrap();
        assert_eq!(t.len(), 384, "{ids:?}");
        // Cada parte do banco já tem exatamente 128 tiles (nada foi preenchido).
        for id in ids {
            let n = b.objeto(engine::formats::banco::Chave { tipo: tiles::TIPO_TILES, id }).unwrap().unwrap().len();
            assert_eq!(n, 128 * tiles::BYTES_POR_TILE, "9/{id}");
        }
        // O tile 1 (primeira formiga preta; o valor 0 de _Life* é "vazio") mistura transparentes e opacos.
        let ant = t.tile(1).unwrap();
        assert!(ant.contains(&TRANSPARENTE));
        assert!(ant.iter().any(|&c| c != TRANSPARENTE));
    }
}

#[test]
fn ninho_preto_do_quick2_bate_com_a_referencia() {
    // Conferido visualmente contra re_trabalho/qi_14_salvo.png (mesma sessão do save): grama, entrada
    // e túnel batem pixel a pixel onde a captura não está coberta pelo diálogo. Ver tiles.md.
    let (Some(b), Some(save)) = (wingant(), quick2()) else { return };
    let layout = LayoutSave::carregar(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/save_layout.json")).unwrap();
    let jogo = JogoSalvo::ler(&layout, save).unwrap();
    let mapa_b = jogo.bloco("MapB").unwrap();
    let chao = tiles::carregar_chao(&b, tiles::ID_CHAO_QUINTAL).unwrap();
    let m = MapaChao::new(Plano::NinhoPreto, mapa_b).unwrap();
    let mut q = Quadro::new(64 * LADO, ALTURA * LADO, 0);
    m.desenhar(&mut q, &chao, (0, 0, 64, ALTURA), (0, 0)).unwrap();
    assert_eq!(hex(&q.pixels), "b61b4e5261ea027f70e223b270d187a7814e1278e85ecf8604af6997f2e6f2d2");
}
