//! Códigos de trapaça do original (`_CheatKeys`, docs/spec/regras/trapacas.md) sobre um Jogo Rápido
//! gerado pelo port. Os esperados vêm da spec (código e bancada); não precisa dos arquivos do jogo.

use std::path::PathBuf;

use engine::rules::amarela::TabelasAmarela;
use engine::rules::casa::TabelasCasa;
use engine::rules::estado::{ConfigSimulacao, Estado, Evento};
use engine::rules::geracao::{Mundo, TabelasGeracao, TipoJogo};
use engine::rules::rng::{GeradorR, GeradorS};
use engine::rules::trapacas::{aplicar, AnelTrapaca, EfeitoTrapaca as Ef, TabelasTrapaca};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn jogo() -> Estado {
    let seno: Vec<u8> = (0..64)
        .flat_map(|i| ((((i as f64) * std::f64::consts::PI / 128.0).sin() * 32767.0).round() as i16).to_be_bytes())
        .collect();
    let mut t = TabelasGeracao::de_partes(&std::fs::read_to_string(raiz().join("data/geracao.json")).unwrap(), &seno).unwrap();
    t.casa = TabelasCasa::de_json(&std::fs::read_to_string(raiz().join("data/casa.json")).unwrap()).unwrap();
    let cfg = ConfigSimulacao::de_json(&std::fs::read_to_string(raiz().join("data/simulacao.json")).unwrap()).unwrap();
    let (mut s, mut r) = (GeradorS::new(0), GeradorR::default());
    r.semear_com_ticks(&mut s, 4321, 4321);
    let m = Mundo::novo_jogo(TipoJogo::Rapido, &mut r, &mut s, &t).unwrap();
    Estado::de_mundo(&m, s, r, &cfg).unwrap()
}

fn tt() -> &'static TabelasTrapaca {
    TabelasTrapaca::padrao()
}

fn ta() -> &'static TabelasAmarela {
    TabelasAmarela::padrao()
}

fn sons(e: &mut Estado) -> Vec<u16> {
    e.retirar_eventos().into_iter().filter_map(|ev| if let Evento::Som(id) = ev { Some(id) } else { None }).collect()
}

#[test]
fn digitar_o_codigo_escolhe_o_efeito() {
    let mut anel = AnelTrapaca::default();
    let efeitos: Vec<Ef> = b"qeenoopsHEALsusi".iter().filter_map(|&b| anel.digitar(b, tt())).collect();
    assert_eq!(efeitos, vec![Ef::RainhaPreta, Ef::RainhasGuardadasVermelhas, Ef::SaudeVermelhaCheia, Ef::SempreSaudavel]);
}

#[test]
fn rainhas_guardadas_e_saude_das_colonias() {
    let mut e = jogo();
    e.preta.saude = 40;
    e.vermelha.saude = 40;
    aplicar(&mut e, tt(), ta(), Ef::RainhasGuardadasPretas);
    aplicar(&mut e, tt(), ta(), Ef::RainhasGuardadasVermelhas);
    assert_eq!((e.preta.rainhas_guardadas, e.vermelha.rainhas_guardadas), (10, 10));
    assert_eq!(sons(&mut e), vec![41, 10]);
    aplicar(&mut e, tt(), ta(), Ef::SaudePretaCheia);
    aplicar(&mut e, tt(), ta(), Ef::SaudeVermelhaZero);
    assert_eq!((e.preta.saude, e.vermelha.saude), (100, 0));
    aplicar(&mut e, tt(), ta(), Ef::SempreSaudavel);
    assert_eq!(e.partida.sempre_saudavel, 1);
    aplicar(&mut e, tt(), ta(), Ef::SaudePretaZero);
    assert_eq!((e.partida.sempre_saudavel, e.preta.saude), (0, 0), "ZHEA desliga o AlwaysHealthy");
}

#[test]
fn alternancias_tocam_ligou_e_desligou() {
    let mut e = jogo();
    for ef in [Ef::Invencivel, Ef::Invencivel, Ef::AranhaDeRe, Ef::NuncaComFome] {
        aplicar(&mut e, tt(), ta(), ef);
    }
    assert_eq!((e.partida.invencivel, e.aranha.de_re, e.partida.nunca_com_fome), (0, 1, 1));
    assert_eq!(e.amarela.saude, 100);
    assert_eq!(sons(&mut e), vec![2, 1, 2, 2]);
}

#[test]
fn rainha_nova_no_ninho_preto() {
    let mut e = jogo();
    let (antes, lista) = (e.preta.rainhas, e.ninho_b.lista.indice);
    e.amarela.x = 20;
    e.amarela.y = 20;
    aplicar(&mut e, tt(), ta(), Ef::RainhaPreta);
    assert_eq!(e.preta.rainhas, antes + 1);
    assert_eq!(e.ninho_b.lista.indice, lista + 2, "cabeça e cauda");
    let cabeca = e.ninho_b.lista.vagas[lista as usize];
    assert_eq!((cabeca.x, cabeca.y, cabeca.tipo, cabeca.modo), (22, 20, 0x62, 9));
}

#[test]
fn ovos_em_volta_da_amarela_com_os_sorteios_do_original() {
    let mut e = jogo();
    e.amarela.x = 30;
    e.amarela.y = 30;
    let (s0, r0, n0) = (e.s, e.r, e.ninho_b.lista.indice);
    aplicar(&mut e, tt(), ta(), Ef::OvosPretos);
    let (mut s, mut r) = (s0, r0);
    for _ in 0..16 {
        r.rrand(6);
        s.mascara(8);
        s.mascara(8);
    }
    // _DigTileB não sorteia em células já cavadas; aqui as 9 em volta podem ser terra: confere R exato.
    assert_eq!(e.r, r, "16 RRand(6)");
    let novos = &e.ninho_b.lista.vagas[n0 as usize..e.ninho_b.lista.indice as usize];
    assert_eq!(novos.len(), 16);
    assert!(novos.iter().all(|f| (29..=31).contains(&f.x) && (29..=31).contains(&f.y) && (1..=6).contains(&f.tipo) && f.modo == 8));
}

#[test]
fn colonias_no_quintal() {
    let mut e = jogo();
    let (pb, fb) = (e.preta.populacao_quintal.0, e.preta.colonias_fundadas);
    aplicar(&mut e, tt(), ta(), Ef::ColoniasPretasNoQuintal);
    assert!(e.preta.populacao_quintal.0.iter().zip(pb).all(|(&d, a)| d == a.wrapping_add(4)));
    assert_eq!(e.preta.colonias_fundadas, fb + 192);
    let s0 = e.s;
    let (b0, r0) = (e.preta.colonias_fundadas, e.vermelha.colonias_fundadas);
    aplicar(&mut e, tt(), ta(), Ef::ColoniasNoQuintal);
    assert_eq!((e.preta.colonias_fundadas - b0) + (e.vermelha.colonias_fundadas - r0), 192);
    let mut s = s0;
    let pretas = (0..192).filter(|_| s.mascara(2) == 0).count() as i16;
    assert_eq!(e.preta.colonias_fundadas - b0, pretas);
    assert_eq!(e.s, s, "um SRand2 por patch");
}

#[test]
fn sem_equivalente_nao_muda_nada() {
    let mut e = jogo();
    let antes = e.clone();
    for ef in [Ef::BancoDeDados, Ef::RedeServidor, Ef::RedeCliente] {
        assert!(ef.sem_equivalente());
        aplicar(&mut e, tt(), ta(), ef);
    }
    assert!(e == antes);
}

#[test]
fn fred_pede_a_animacao_da_vitoria_e_nao_muda_o_estado() {
    // `_CheatKeys` caso 0x14: só `_DrawSimPayoff` (desenho e música).
    let mut e = jogo();
    let antes = e.clone();
    assert!(!Ef::AnimacaoVitoria.sem_equivalente());
    aplicar(&mut e, tt(), ta(), Ef::AnimacaoVitoria);
    assert_eq!(e.retirar_eventos(), vec![engine::rules::estado::Evento::AnimacaoVitoria]);
    assert!(e == antes);
}
