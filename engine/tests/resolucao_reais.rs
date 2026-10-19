//! Tela virtual (docs/spec/telas/resolucao.md): a regra de data/resolucao.json aplicada à abertura
//! reproduz, em 800×600, exatamente as telas validadas contra o DOSBox-X (telas_reais.rs: título
//! 99,98% e "Select a Game" 99,87% dos pixels iguais) e, em 640×480, usa a tabela 9/0 da bancada.

use std::path::PathBuf;

use engine::core::resolucao::{ConfigResolucao, ARQUIVO};
use engine::core::telas::{compor, Interface, Sequencia};
use engine::recursos::Recursos;

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
fn abertura_em_800x600_pela_regra_e_a_validada_no_dosbox() {
    let Some(r) = recursos() else { return };
    let tela = r.dados::<ConfigResolucao>(ARQUIVO).unwrap().tela([800, 600]).unwrap();
    let mut seq: Sequencia = r.dados("telas.json").unwrap();
    let validada = seq.clone();
    tela.aplicar_sequencia(&mut seq);
    // As telas validadas (telas.json, tabela 8) são as do DOSBox-X a 800×600.
    let ref_interface = Interface::carregar(&r, 8).unwrap();
    let interface = Interface::carregar(&r, tela.tabela_posicoes).unwrap();
    for (t, v) in seq.telas.iter().zip(&validada.telas) {
        let (q, _) = compor(t, &r, &interface).unwrap();
        let (q_ref, _) = compor(v, &r, &ref_interface).unwrap();
        assert_eq!((q.largura, q.altura), (q_ref.largura, q_ref.altura), "{}", t.nome);
        assert!(q.pixels == q_ref.pixels, "tela {} diferente da validada", t.nome);
    }
}

#[test]
fn abertura_em_640x480_usa_a_tabela_0() {
    let Some(r) = recursos() else { return };
    let tela = r.dados::<ConfigResolucao>(ARQUIVO).unwrap().tela([640, 480]).unwrap();
    let mut seq: Sequencia = r.dados("telas.json").unwrap();
    tela.aplicar_sequencia(&mut seq);
    let interface = Interface::carregar(&r, tela.tabela_posicoes).unwrap();
    let titulo = seq.telas.iter().find(|t| t.nome == "titulo").unwrap();
    let (q, pendentes) = compor(titulo, &r, &interface).unwrap();
    assert!(pendentes.is_empty(), "{pendentes:?}");
    assert_eq!((q.largura, q.altura), (640, 442));
    let px = |x: usize, y: usize| q.pixels[y * q.largura + x];
    // Janela 3 na tabela 9/0: (64, 3) + faixa (73). Na de 800×600 (9/8) ficaria em (64, 145).
    assert_eq!(px(64, 76), 0, "borda externa");
    assert_eq!(px(65, 77), 16, "cor da borda");
    assert_eq!(px(64, 75), 17, "fundo acima da janela");
    assert_eq!(px(639, 30), 8, "faixa até a borda direita");
}
