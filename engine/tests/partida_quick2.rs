//! Laço de jogo (`core::partida`) sobre o `QUICK2.ANT`: regressão de "roda sem quebrar".
//! Especificação: docs/spec/telas/jogo.md, seção "Simulação ao vivo".

use std::path::PathBuf;

use engine::core::navegacao::Navegacao;
use engine::core::partida::{ConfigPartida, Partida};
use engine::core::tela_jogo::PosicaoVista;
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::rules::amarela::TabelasAmarela;
use engine::rules::estado::{ConfigSimulacao, Estado};
use engine::rules::tick::RegrasOriginais;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn ler(nome: &str) -> String {
    std::fs::read_to_string(raiz().join("data").join(nome)).unwrap()
}

fn partida() -> Partida {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let jogo = JogoSalvo::abrir(&layout, &raiz().join("tests/referencias/saves/QUICK2.ANT")).unwrap();
    let estado = Estado::de_save(&jogo).unwrap();
    let nav = Navegacao::do_save(&jogo, PosicaoVista::AoCarregar, 26, 19).unwrap();
    Partida::nova(
        estado,
        nav,
        RegrasOriginais::default(),
        TabelasAmarela::padrao().clone(),
        ConfigSimulacao::de_json(&ler("simulacao.json")).unwrap(),
        ConfigPartida::de_json(&ler("partida.json")).unwrap(),
    )
}

#[test]
fn cinquenta_ticks_do_quick2() {
    let mut p = partida();
    let antes = p.estado.partida.tempo;
    for _ in 0..50 {
        p.rodar_tick();
    }
    assert_eq!(p.estado.partida.tempo, antes + 50);
    // A vista continua no ninho preto, dentro do mapa.
    assert_eq!(p.nav.plano_mapa, 2);
    assert!(p.nav.vista.x >= 0 && p.nav.vista.x + p.nav.vista.largura <= 64);
}

#[test]
fn relogio_em_tempo_real_no_quick2() {
    // QUICK2: _GameSpeed = 1 (normal), não pausado: 1 tick a cada 2 mensagens de 17 ms.
    let mut p = partida();
    assert_eq!(p.estado.partida.velocidade, 1);
    assert_eq!(p.estado.partida.pausado, 0);
    let antes = p.estado.partida.tempo;
    let mut ticks = 0;
    for _ in 0..100 {
        ticks += p.avancar(17).ticks;
    }
    assert_eq!(ticks, 50);
    assert_eq!(p.estado.partida.tempo, antes + 50);
    assert_eq!(p.estado.ciclos_jogo, 100, "_gameCycles conta toda mensagem");
}

#[test]
fn ordem_de_andar_pelo_clique() {
    // Clique simples numa célula vizinha da amarela dá a ordem de ir (`_MeMoveMe` = 1).
    let mut p = partida();
    let (x, y) = (p.estado.amarela.x as i32, p.estado.amarela.y as i32);
    p.clicar(x, y + 1, false);
    assert_eq!(p.estado.amarela.mover, 1);
    assert_eq!((p.estado.amarela.alvo_x as i32, p.estado.amarela.alvo_y as i32), (x, y + 1));
}
