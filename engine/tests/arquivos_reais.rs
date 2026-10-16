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

/// Bits de um texto renderizado, como string de '#' e '.' por linha.
fn bits_texto(f: &engine::formats::fonte::Fonte, t: &str) -> String {
    let img = f.imagem(t.as_bytes());
    (0..img.altura)
        .map(|y| (0..img.largura).map(|x| if img.bits[y * img.largura + x] { '#' } else { '.' }).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn fontes_mac_batem_com_o_original() {
    // Validado contra tests/referencias/capturas/apos_titulo.png (janela de registro): o bloco inteiro
    // de cada texto aparece pixel a pixel na captura (font2 em (362,397), font3 em (361,356)).
    use engine::formats::fonte::{Fonte, FonteMac};
    let Some(d) = dir_jogo() else { return };
    let f2 = Fonte::Mac(FonteMac::decodificar(&std::fs::read(d.join("FONT2")).unwrap()).unwrap());
    let f3 = Fonte::Mac(FonteMac::decodificar(&std::fs::read(d.join("FONT3")).unwrap()).unwrap());
    let a = bits_texto(&f2, "Click here to continue");
    let b = bits_texto(&f3, "Customer ID:");
    assert_eq!((f2.largura_texto(b"Click here to continue"), f2.altura()), (84, 9));
    assert_eq!((f3.largura_texto(b"Customer ID:"), f3.altura()), (85, 14));
    assert_eq!(hex(a.as_bytes()), "c07a3a1ee52a5801e6509d70a6578fa6b2b92a959a0addff333e92244ae3947e");
    assert_eq!(hex(b.as_bytes()), "019dcbcbb48bbd4f2272dbd29b6363aa665a5927e2698a60a1fee5ee78902627");
}
