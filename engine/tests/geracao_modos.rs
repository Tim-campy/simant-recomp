//! Jogo novo nos tipos Full Game, Experimental e Tutorial (do menu) contra o original.
//! Estados gravados na bancada QEMU parando no CALL do primeiro `_DoAntSim` depois do clique no
//! "Select a Game" da abertura (`tests/referencias/roteiros/qemu_geracao_modos.txt`; `GameTime` = 0:
//! nenhum tick). O port regera o patch (11, 8) com a semente `_YMapSeeds[8 * 12 + 11]` do próprio
//! estado (o estado de R no clique não é conhecido), monta o estado como a interface faz no jogo novo
//! (`carregar::estado_do_mundo_novo` e `centrar_na_amarela`) e compara os 307 blocos.
//! Especificação: docs/spec/regras/geracao-mundo.md, "Tipos de jogo no original".

use std::path::PathBuf;

use engine::core::carregar::{centrar_na_amarela, estado_do_mundo_novo};
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::recursos::Recursos;
use engine::rules::estado::ConfigSimulacao;
use engine::rules::geracao::{Mundo, TabelasGeracao, TipoJogo, PATCH_INICIAL};
use engine::rules::quintal::TabelasQuintal;
use engine::rules::rng::{GeradorR, GeradorS};
use engine::rules::tick::RegrasOriginais;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn tabelas() -> Option<(TabelasGeracao, TabelasQuintal)> {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
        return None;
    }
    let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    Some((TabelasGeracao::carregar(&r).unwrap(), RegrasOriginais::carregar_sem_banco(&r).unwrap().quintal))
}

/// Nomes dos blocos em que o estado do port difere do original. `vista` é a janela de edição da
/// bancada em células (25 × 19; no Tutorial ela abre maximizada: 39 × 21, telas/janelas.md §8).
fn diferencas(
    arquivo: &str,
    tipo: TipoJogo,
    vista: (i32, i32),
    (tab, quintal): &(TabelasGeracao, TabelasQuintal),
) -> Vec<String> {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let orig = JogoSalvo::abrir(&layout, &raiz().join(format!("tests/referencias/saves/{arquivo}.ANT"))).unwrap();
    assert_eq!(orig.i32("GameTime").unwrap(), 0, "{arquivo}: estado antes do primeiro tick");
    let mut m = Mundo::vazio(tipo);
    for (d, o) in m.sementes.iter_mut().zip(orig.vetor_i16("YMapSeeds").unwrap()) {
        *d = o as u16;
    }
    m.ninho_iniciado = tipo != TipoJogo::Completo;
    let (px, py) = PATCH_INICIAL;
    let semente = m.sementes[(py * 12 + px) as usize];
    let pretas = if tipo != TipoJogo::Completo { 1 } else { 0 };
    let mut s = GeradorS::new(0);
    m.gerar_patch(&mut s, tab, semente, pretas, 1, px, py).unwrap();
    let cfg = ConfigSimulacao::de_json(&std::fs::read_to_string(raiz().join("data/simulacao.json")).unwrap()).unwrap();
    let mut e = estado_do_mundo_novo(&m, s, GeradorR::default(), &cfg, quintal).unwrap();
    centrar_na_amarela(&mut e, vista);
    let port = e.para_save(&layout).unwrap();
    orig.simbolos()
        .filter(|nome| orig.bloco(nome) != port.bloco(nome))
        .map(|nome| {
            let (a, b) = (orig.bloco(nome).unwrap(), port.bloco(nome).unwrap());
            if a.len() <= 16 {
                eprintln!("{arquivo} {nome}: original {a:02x?}, port {b:02x?}");
            }
            nome.to_string()
        })
        .collect()
}

#[test]
fn full_experimental_e_tutorial_iguais_ao_original() {
    let Some(tab) = tabelas() else { return };
    let mut todas = Vec::new();
    for (arquivo, tipo, vista) in [
        ("GERACAO_FULL", TipoJogo::Completo, (25, 19)),
        ("GERACAO_EXP", TipoJogo::Experimental, (25, 19)),
        ("GERACAO_TUT", TipoJogo::Tutorial, (39, 21)),
    ] {
        let d = diferencas(arquivo, tipo, vista, &tab);
        if !d.is_empty() {
            todas.push(format!("{arquivo}: {d:?}"));
        }
    }
    assert!(todas.is_empty(), "blocos diferentes:\n{}", todas.join("\n"));
}

/// Jogo novo escolhido **durante a partida** (File → New, Ctrl+N; roteiros `qemu_novo_na_partida.txt`
/// e `qemu_novo_na_partida2.txt`): do Quick Game do snapshot `quick`, pausado pelo menu, o estado e
/// as sementes gravados (`NOVO_*_ANTES.ANT`; S e R lidos na bancada, iguais aos do instante do clique
/// no "Select a Game"), depois o tipo escolhido. O original não ressemeia: o port gera com
/// `carregar::jogo_novo_com_geradores` a partir do estado anterior e compara os 307 blocos com o
/// estado logo depois do clique (`GameTime` = 0: o jogo novo continua pausado). No Tutorial a
/// edição abre maximizada (39 × 21).
#[test]
fn jogo_novo_durante_a_partida_igual_ao_original() {
    use engine::core::carregar::jogo_novo_com_geradores;
    use engine::rules::estado::Estado;
    let Some((tab, quintal)) = tabelas() else { return };
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let abrir = |n: &str| JogoSalvo::abrir(&layout, &raiz().join(format!("tests/referencias/saves/{n}.ANT"))).unwrap();
    let cfg = ConfigSimulacao::de_json(&std::fs::read_to_string(raiz().join("data/simulacao.json")).unwrap()).unwrap();
    let mut todas = Vec::new();
    for (antes, depois, tipo, vista, s, r) in [
        ("NOVO_ANTES", "NOVO_FULL", TipoJogo::Completo, (25, 19), 9669, 805_219_728),
        ("NOVO_EXP_ANTES", "NOVO_EXP", TipoJogo::Experimental, (25, 19), 46275, 1_451_923_833),
        ("NOVO_TUT_ANTES", "NOVO_TUT", TipoJogo::Tutorial, (39, 21), 60646, 4_224_699_591),
        ("NOVO_QUICK_ANTES", "NOVO_QUICK", TipoJogo::Rapido, (25, 19), 46275, 1_451_923_833),
    ] {
        let orig = abrir(depois);
        assert_eq!(orig.i32("GameTime").unwrap(), 0, "{depois}");
        let mut anterior = Estado::de_save(&abrir(antes)).unwrap();
        anterior.s = GeradorS::new(s);
        anterior.r = GeradorR::de_estado(r);
        let mut e = jogo_novo_com_geradores(tipo, &anterior, &tab, &cfg, &quintal, (25, 19)).unwrap();
        if vista != (25, 19) {
            // A interface maximiza a edição (o `_win_Zoom(0)` do Tutorial; `_win_EditChanged`
            // limita a vista) e o `_CenterEdit` do fim de `_NewGame` anda até a amarela pelo passo
            // de reta (o eixo menor pode ficar um passo atrás): (26, 2) → (25, 2) → (19, 2).
            let plano = engine::core::vida::plano_de(e.vistas.plano).unwrap();
            let (x, y) = (i32::from(e.vistas.ponto.0), i32::from(e.vistas.ponto.1));
            let mut v = engine::core::edicao::VistaEdicao { plano, x, y, largura: vista.0, altura: vista.1 };
            v.limitar();
            v.centralizar(i32::from(e.amarela.x), i32::from(e.amarela.y));
            e.vistas.ponto = (v.x as i16, v.y as i16);
        }
        let port = e.para_save(&layout).unwrap();
        let dif: Vec<String> = orig
            .simbolos()
            .filter(|nome| orig.bloco(nome) != port.bloco(nome))
            .map(|nome| {
                let (a, b) = (orig.bloco(nome).unwrap(), port.bloco(nome).unwrap());
                if a.len() <= 16 {
                    eprintln!("{depois} {nome}: original {a:02x?}, port {b:02x?}");
                }
                nome.to_string()
            })
            .collect();
        if !dif.is_empty() {
            todas.push(format!("{depois}: {dif:?}"));
        }
    }
    assert!(todas.is_empty(), "blocos diferentes:\n{}", todas.join("\n"));
}
