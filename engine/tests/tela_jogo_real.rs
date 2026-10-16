//! Tela de jogo do `QUICK2.ANT` (faixa + janela de edição com chão e vida). Especificação:
//! docs/spec/telas/jogo.md. O save está no repositório; os bancos do jogo vêm de SIMANT_DIR (ou
//! ../SimAnt-T/MAXIS/ANTWIN) e, sem eles, os testes que desenham avisam e passam sem verificar nada.
//!
//! O hash foi gravado depois de comparar a saída com re_trabalho/qi_14_salvo.png (exemplo
//! `tela_jogo`): 100% dos pixels do mapa visíveis fora do diálogo iguais, 99,33% da janela de edição.

use std::path::PathBuf;

use engine::core::mapa::Plano;
use engine::core::tela_jogo::{compor_tela_de_jogo, vista_do_save, ConfigTelaJogo, PosicaoVista};
use engine::core::telas::Interface;
use engine::core::vida::{reconstruir, AMARELA};
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::recursos::Recursos;
use sha2::{Digest, Sha256};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn quick2() -> JogoSalvo {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    JogoSalvo::abrir(&layout, &raiz().join("tests/referencias/saves/QUICK2.ANT")).unwrap()
}

fn recursos() -> Option<Recursos> {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
        return None;
    }
    let bancos = ["SHARED", "WINGANT", "SOUND"].map(String::from);
    Some(Recursos::abrir(&jogo, &bancos, &raiz().join("data"), None).unwrap())
}

#[test]
fn vida_do_quick2_reconstruida_das_listas() {
    let r = reconstruir(&quick2()).unwrap();
    assert!(r.pendencias.is_empty(), "{:?}", r.pendencias);
    let c = &r.camadas;
    // Lista A: operária preta (T = 17) e vermelha (T = 150).
    assert_eq!(c.valor(Plano::Superficie, 59, 31), 17);
    assert_eq!(c.valor(Plano::Superficie, 123, 44), 150);
    // Lista B: a formiga amarela (_MeMode = 0) cobre a rainha preta e um ovo em (34, 9);
    // em (36, 9) há dois ovos e vence o de menor índice.
    assert_eq!(c.valor(Plano::NinhoPreto, 34, 9), AMARELA);
    assert_eq!(c.valor(Plano::NinhoPreto, 35, 9), 110);
    assert_eq!(c.valor(Plano::NinhoPreto, 36, 9), 1);
    // Lista R: rainha vermelha, sua metade e quatro ovos na mesma célula.
    assert_eq!(c.valor(Plano::NinhoVermelho, 41, 11), 226);
    assert_eq!(c.valor(Plano::NinhoVermelho, 40, 11), 234);
    assert_eq!(c.valor(Plano::NinhoVermelho, 39, 11), 129);
    let ocupadas = |v: &[u8]| v.iter().filter(|&&b| b != 0).count();
    assert_eq!((ocupadas(&c.a), ocupadas(&c.b), ocupadas(&c.r)), (2, 3, 3));
}

#[test]
fn vista_do_quick2_salva_e_ao_carregar() {
    let j = quick2();
    let salva = vista_do_save(&j, PosicaoVista::Salva, 26, 19).unwrap();
    assert_eq!((salva.plano, salva.x, salva.y), (Plano::NinhoPreto, 22, 0));
    // Ao carregar, centraliza em (_MeLocX, _MeLocY) = (34, 9): x = 34 − 26/2 = 21 [hipótese, jogo.md].
    let carregada = vista_do_save(&j, PosicaoVista::AoCarregar, 26, 19).unwrap();
    assert_eq!((carregada.x, carregada.y), (21, 0));
}

#[test]
fn tela_de_jogo_do_quick2_igual_a_validada() {
    let Some(r) = recursos() else { return };
    let cfg: ConfigTelaJogo = r.dados("tela_jogo.json").unwrap();
    let interface = Interface::carregar(&r, cfg.tabela_posicoes).unwrap();
    let (q, pendentes) = compor_tela_de_jogo(&cfg, &r, &interface, &quick2()).unwrap();
    assert!(pendentes.is_empty(), "{pendentes:?}");
    assert_eq!((q.largura, q.altura), (640, 442));
    let px = |x: usize, y: usize| q.pixels[y * q.largura + x];
    // Contorno da janela de edição em (20, 97) e moldura cinza (índice 8) logo dentro.
    assert_eq!(px(20, 97), 0);
    assert_eq!(px(21, 98), 8);
    // Formiga amarela na célula (34, 9) do ninho, coluna 12 da vista: pixels amarelos (índice 11).
    let (x0, y0) = (24 + 12 * 16, 120 + 9 * 16);
    let amarelos = (0..16).flat_map(|j| (0..16).map(move |i| (x0 + i, y0 + j))).filter(|&(x, y)| px(x, y) == 11).count();
    assert!(amarelos > 0);
    // A barra de rolagem vertical recorta a 26.ª coluna da vista.
    assert_eq!(px(424, 200), 0);
    let hash: String = Sha256::digest(&q.pixels).iter().map(|b| format!("{b:02x}")).collect();
    println!("hash da tela de jogo: {hash}");
    assert_eq!(hash, HASH_TELA_JOGO);
}

const HASH_TELA_JOGO: &str = "2bdd46f9280642b70c36b1fd5f74aa78efad63dc208760e5b87a3cd19312c1c8";
