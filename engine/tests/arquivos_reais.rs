//! Testes com os arquivos originais do usuário (não versionados).
//! Pasta do jogo: variável SIMANT_DIR ou, por padrão, ../SimAnt-T/MAXIS/ANTWIN.
//! Sem os arquivos, os testes avisam e passam sem verificar nada.

use std::path::PathBuf;

use engine::formats::banco::{Banco, Chave};
use engine::formats::bitmap::{Bitmap, TIPO_BITMAP};
use sha2::{Digest, Sha256};

fn dir_jogo() -> Option<PathBuf> {
    let d = std::env::var_os("SIMANT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../SimAnt-T/MAXIS/ANTWIN"));
    if d.join("SHARED.NDX").is_file() {
        Some(d)
    } else {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", d.display());
        None
    }
}

fn hex(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn bancos_abrem_com_o_numero_de_entradas_esperado() {
    let Some(d) = dir_jogo() else { return };
    for (nome, n) in [("SHARED", 518), ("WINGANT", 406), ("MWINNT", 402), ("SOUND", 182)] {
        let b = Banco::abrir(&d.join(nome)).unwrap();
        assert_eq!(b.entradas().len(), n, "{nome}");
    }
}

#[test]
fn todos_os_objetos_descomprimem() {
    let Some(d) = dir_jogo() else { return };
    for nome in ["SHARED", "WINGANT", "MWINNT", "SOUND"] {
        let b = Banco::abrir(&d.join(nome)).unwrap();
        for e in b.entradas() {
            b.objeto(e.chave).unwrap().unwrap_or_else(|err| panic!("{err}"));
        }
    }
}

#[test]
fn lzss_bate_com_a_ferramenta_python() {
    // Hashes gerados por tools/lzss_probe.py (janela inicial com espaços), ver docs/spec/formatos/banco-dat-ndx.md.
    let Some(d) = dir_jogo() else { return };
    let w = Banco::abrir(&d.join("WINGANT")).unwrap();
    let obj = w.objeto(Chave { tipo: 2, id: 9111 }).unwrap().unwrap();
    assert_eq!(obj.len(), 3572);
    assert_eq!(hex(&obj), "3616be2e3feca5aa66101c3ebf8667de00b66a9d2a7a46d78436ce700a32cd74");
    let s = Banco::abrir(&d.join("SHARED")).unwrap();
    let obj = s.objeto(Chave { tipo: 4, id: 1000 }).unwrap().unwrap();
    assert_eq!(hex(&obj), "2c5d9f82457d79b2b11f4b5e0e38f4b4a06b25c87ce27e95417723299675cdc1");
}

#[test]
fn bitmaps_conhecidos_tem_as_dimensoes_certas() {
    let Some(d) = dir_jogo() else { return };
    let w = Banco::abrir(&d.join("WINGANT")).unwrap();
    for (id, larg, alt) in [(2000, 512, 168), (2001, 512, 174), (2500, 318, 259), (3000, 320, 200), (28000, 236, 197)] {
        let bm = Bitmap::decodificar(&w.objeto(Chave { tipo: TIPO_BITMAP, id }).unwrap().unwrap()).unwrap();
        assert_eq!((bm.largura, bm.altura), (larg, alt), "bitmap {id}");
    }
}

#[test]
fn todos_os_bitmaps_coloridos_decodificam() {
    let Some(d) = dir_jogo() else { return };
    let w = Banco::abrir(&d.join("WINGANT")).unwrap();
    let mut falhas = Vec::new();
    let mut total = 0;
    for e in w.entradas().iter().filter(|e| e.chave.tipo == TIPO_BITMAP) {
        total += 1;
        let dados = w.objeto(e.chave).unwrap().unwrap();
        if Bitmap::decodificar(&dados).is_err() {
            falhas.push(e.chave.id);
        }
    }
    eprintln!("WINGANT: {} de {} bitmaps não decodificam: {:?}", falhas.len(), total, falhas);
    assert_eq!(total, 352);
    // Decodificar sem erro não garante imagem correta: o grupo ~7100 sai com faixas (bitmap.md, Em aberto).
    assert!(falhas.is_empty(), "{falhas:?}");
}
