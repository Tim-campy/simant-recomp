//! Troca de patch (`_XferPatch`) e colocação de rainhas (`_PlaceQueenInYard`) sobre um Jogo Completo
//! gerado pelo port. Especificação: docs/spec/telas/quintal.md §5. Os esperados vêm da spec; não
//! precisa dos arquivos do jogo (seno sintético, tabelas de data/).

use std::path::PathBuf;

use engine::rules::casa::TabelasCasa;
use engine::rules::estado::{ConfigSimulacao, Estado, Evento};
use engine::rules::geracao::{Mundo, TabelasGeracao, TipoJogo};
use engine::rules::rng::{GeradorR, GeradorS};
use engine::rules::troca_patch::{ids, trocar_patch, ResultadoTroca};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn tabelas() -> TabelasGeracao {
    let seno: Vec<u8> = (0..64)
        .flat_map(|i| ((((i as f64) * std::f64::consts::PI / 128.0).sin() * 32767.0).round() as i16).to_be_bytes())
        .collect();
    let mut t = TabelasGeracao::de_partes(&std::fs::read_to_string(raiz().join("data/geracao.json")).unwrap(), &seno).unwrap();
    t.casa = TabelasCasa::de_json(&std::fs::read_to_string(raiz().join("data/casa.json")).unwrap()).unwrap();
    t
}

fn cfg() -> ConfigSimulacao {
    ConfigSimulacao::de_json(&std::fs::read_to_string(raiz().join("data/simulacao.json")).unwrap()).unwrap()
}

fn jogo(tipo: TipoJogo) -> Estado {
    let (mut s, mut r) = (GeradorS::new(0), GeradorR::default());
    r.semear_com_ticks(&mut s, 4321, 4321);
    let m = Mundo::novo_jogo(tipo, &mut r, &mut s, &tabelas()).unwrap();
    Estado::de_mundo(&m, s, r, &cfg()).unwrap()
}

fn i(x: usize, y: usize) -> usize {
    x * 16 + y
}

#[test]
fn recusas_do_xfer_patch() {
    let t = tabelas();
    // Mesmo patch: nada.
    let mut e = jogo(TipoJogo::Completo);
    assert_eq!(trocar_patch(&mut e, &t, &cfg()).unwrap(), ResultadoTroca::MesmoPatch);
    // Quick Game: só no Jogo Completo.
    let mut e = jogo(TipoJogo::Rapido);
    e.vistas.ponto_quintal = (5, 5);
    assert_eq!(trocar_patch(&mut e, &t, &cfg()).unwrap(), ResultadoTroca::Recusada(ids::SO_NO_JOGO_COMPLETO));
    // Full Game no começo: menos de 2 colônias e menos de 2 formigas pretas.
    let mut e = jogo(TipoJogo::Completo);
    e.vistas.ponto_quintal = (5, 5);
    assert_eq!(trocar_patch(&mut e, &t, &cfg()).unwrap(), ResultadoTroca::Recusada(ids::POUCAS_FORMIGAS));
    // Com formigas, mas sem colônia preta no destino (e a amarela já com ninho).
    e.preta.populacao = 50;
    e.amarela.ninho_iniciado = 1;
    assert_eq!(trocar_patch(&mut e, &t, &cfg()).unwrap(), ResultadoTroca::Recusada(ids::SEM_COLONIA_PRETA));
    assert_eq!(e.ambiente.patch, (11, 8), "a recusa não muda o patch");
    assert_eq!(e.retirar_eventos().last(), Some(&Evento::Mensagem(ids::SEM_COLONIA_PRETA)));
}

#[test]
fn troca_para_o_quintal_gera_o_patch_da_semente() {
    let t = tabelas();
    let mut e = jogo(TipoJogo::Completo);
    e.preta.populacao = 50;
    e.amarela.ninho_iniciado = 1;
    e.preta.populacao_quintal.0[i(6, 9)] = 3;
    e.vermelha.populacao_quintal.0[i(6, 9)] = 0;
    e.preta.rainhas_guardadas = 2;
    e.menino.x = 123;
    let tempo = e.partida.tempo;
    e.vistas.ponto_quintal = (6, 9);
    let r = trocar_patch(&mut e, &t, &cfg()).unwrap();
    assert_eq!(r, ResultadoTroca::Trocou { mensagem: ids::MUDOU_QUINTAL, casa: false });
    assert_eq!(e.ambiente.patch, (6, 9));
    // _JustXfered = 1 vem antes de _RandWorld, cujo _CountAnts o consome (volta a 0).
    assert_eq!((e.ambiente.terreno, e.partida.acabou_de_trocar), (0, 0));
    assert_eq!(e.preta.rainhas_guardadas, 0);
    assert_eq!(e.vermelha.saude, 0, "sem vermelhas no destino: _HealthR = 0");
    assert_eq!(e.menino.x, 123, "o quintal não é refeito");
    assert_eq!(e.partida.tempo, tempo);
    // O patch novo é o de _RandWorld com a semente do patch: mesmo mapa de superfície.
    let mut m = Mundo::vazio(TipoJogo::Completo);
    m.ninho_iniciado = true;
    let mut s = GeradorS::new(0);
    m.gerar_patch(&mut s, &t, e.ambiente.sementes.0[9 * 12 + 6], 3, 0, 6, 9).unwrap();
    assert_eq!(e.superficie.mapa.dados, m.mapa_a);
    assert_eq!(e.ninho_b.mapa.dados, m.mapa_b);
    assert_eq!(e.amarela.plano, 2, "operária no ninho preto");
    let ev = e.retirar_eventos();
    assert_eq!(ev, vec![Evento::Musica(ids::MUSICA_TROCA), Evento::Mensagem(ids::MUDOU_QUINTAL)]);
}

#[test]
fn troca_para_a_casa_usa_o_terreno_da_casa() {
    let t = tabelas();
    let mut e = jogo(TipoJogo::Completo);
    e.preta.total_colonias = 3;
    e.amarela.ninho_iniciado = 1;
    e.preta.populacao_quintal.0[i(0, 4)] = 2;
    e.vermelha.populacao_quintal.0[i(0, 4)] = 1;
    e.vistas.ponto_quintal = (0, 4);
    let r = trocar_patch(&mut e, &t, &cfg()).unwrap();
    assert_eq!(r, ResultadoTroca::Trocou { mensagem: ids::MUDOU_CASA, casa: true });
    assert_eq!((e.ambiente.terreno, e.ambiente.barreira), (1, cfg().inicial.barreira_casa));
    assert_eq!(e.vermelha.saude, 100);
}
