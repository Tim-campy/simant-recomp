//! Telas compostas com os arquivos reais. O hash de referência foi gerado depois de comparar a saída
//! com a captura do original no DOSBox-X: 99,98% dos pixels iguais, e a única diferença é o cursor
//! do mouse (tools/comparar_captura.py, tests/referencias/README.md).

use std::path::PathBuf;

use engine::core::telas::{compor, Interface, Sequencia};
use engine::recursos::Recursos;
use sha2::{Digest, Sha256};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn recursos() -> Option<Recursos> {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
        return None;
    }
    let bancos = ["SHARED", "WINGANT", "SOUND"].map(String::from);
    Some(Recursos::abrir(&jogo, &bancos, &raiz().join("data"), None).unwrap())
}

#[test]
fn titulo_igual_ao_validado_contra_o_original() {
    let Some(r) = recursos() else { return };
    let interface = Interface::carregar(&r, 8).unwrap();
    let seq: Sequencia = r.dados("telas.json").unwrap();
    let tela = seq.telas.iter().find(|t| t.nome == "titulo").unwrap();
    let (q, pendentes) = compor(tela, &r, &interface).unwrap();
    assert!(pendentes.is_empty(), "{pendentes:?}");
    assert_eq!((q.largura, q.altura), (800, 562));
    // Pontos medidos na captura: borda externa preta em (64,145), cor da borda em (65,146),
    // primeiro pixel do bitmap 2000 em (69,150), fundo da aplicação fora das janelas.
    let px = |x: usize, y: usize| q.pixels[y * q.largura + x];
    assert_eq!(px(64, 145), 0);
    assert_eq!(px(65, 146), 16);
    assert_eq!(px(700, 400), 17);
    assert_eq!(px(645, 30), 8);
    let hash: String = Sha256::digest(&q.pixels).iter().map(|b| format!("{b:02x}")).collect();
    println!("hash do título: {hash}");
    assert_eq!(hash, HASH_TITULO);
}

#[test]
fn selecao_igual_a_validada_contra_o_original() {
    // Comparada com tests/referencias/capturas/novo_jogo.png: 99,87% iguais; diferem só o cursor e os
    // botões configuráveis ribUserDef1E/1G da faixa (estado definido por _SetUserButton em tempo de execução).
    let Some(r) = recursos() else { return };
    let interface = Interface::carregar(&r, 8).unwrap();
    let seq: Sequencia = r.dados("telas.json").unwrap();
    let tela = seq.telas.iter().find(|t| t.nome == "selecao").unwrap();
    let (q, pendentes) = compor(tela, &r, &interface).unwrap();
    assert!(pendentes.is_empty(), "{pendentes:?}");
    // Borda externa da janela 2 em (138,116) e bitmap 2500 recortado pelo interior a partir de (143,121).
    let px = |x: usize, y: usize| q.pixels[y * q.largura + x];
    assert_eq!(px(138, 116), 0);
    assert_eq!(px(139, 117), 16);
    let hash: String = Sha256::digest(&q.pixels).iter().map(|b| format!("{b:02x}")).collect();
    println!("hash da seleção: {hash}");
    assert_eq!(hash, HASH_SELECAO);
}

const HASH_SELECAO: &str = "5cdcef65aab3b47ba9e9c64afdd5965b17fe8250ad85ec14d0f2ed855bb757fb";

const HASH_TITULO: &str = "678a9c6bf41a778f66acf51c5454abe78ca071a9f5fa84a53574c29df69ac21c";
