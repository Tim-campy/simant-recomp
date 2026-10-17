//! Efeitos sonoros e tema do ninho pedidos pelas regras (docs/spec/regras/som.md, "O que está ligado").
//!
//! Cada caso roda a mesma regra duas vezes, com as tabelas do port e com a seção `sons` vazia, e
//! confere que: (1) com os ids, o `Evento::Som` esperado aparece; (2) sem os ids, nenhum aparece; e
//! (3) fora a fila de eventos, os dois estados (inclusive os geradores) são idênticos. Ou seja, pedir
//! um som não sorteia nem muda a simulação.

use std::path::PathBuf;

use engine::formats::save::{Formiga, JogoSalvo, LayoutSave, Lista};
use engine::rules::amarela::{self, ControleJogador, Modificadores, SonsAmarela, TabelasAmarela};
use engine::rules::estado::{Estado, Evento};
use engine::rules::geracao::Cor;
use engine::rules::inimigos::{formigas_leao, SonsInimigos, TabelasInimigos};
use engine::rules::ninho::{simular_ninho, SonsNinho, TabelasNinho};
use engine::rules::quintal::{simular_agua, SonsQuintal, TabelasQuintal};
use engine::rules::superficie::SonsSuperficie;
use engine::rules::tick::{tick, RegrasOriginais, RegrasSuperficie};

fn sons(ev: &[Evento]) -> Vec<u16> {
    ev.iter().filter_map(|e| if let Evento::Som(id) = e { Some(*id) } else { None }).collect()
}

/// Roda `f` em `e` com as duas versões das tabelas e devolve os sons pedidos na versão com ids.
fn comparar<T>(e: &Estado, com: &T, sem: &T, f: impl Fn(&mut Estado, &T)) -> Vec<u16> {
    let (mut a, mut b) = (e.clone(), e.clone());
    f(&mut a, com);
    f(&mut b, sem);
    let (ea, eb) = (a.retirar_eventos(), b.retirar_eventos());
    assert!(sons(&eb).is_empty(), "sem ids, nenhum som: {eb:?}");
    let resto = |v: &[Evento]| v.iter().filter(|x| !matches!(x, Evento::Som(_))).cloned().collect::<Vec<_>>();
    assert_eq!(resto(&ea), resto(&eb), "os outros eventos não mudam");
    assert_eq!(a, b, "pedir som não muda o estado nem os sorteios");
    sons(&ea)
}

#[test]
fn quintal_chuva_pede_0x29() {
    let com = TabelasQuintal::padrao().clone();
    let sem = TabelasQuintal { sons: SonsQuintal::default(), ..com.clone() };
    let mut achou = false;
    for semente in 1..400u16 {
        let mut e = Estado::vazio();
        e.ambiente.chovendo = 1;
        e.s.definir_semente(semente);
        let s = comparar(&e, &com, &sem, simular_agua);
        achou |= s == vec![41];
        assert!(s.is_empty() || s == vec![41], "{s:?}");
    }
    assert!(achou, "S(50) = 0 em alguma semente");
}

#[test]
fn ninho_cavar_pede_0x11_no_preto() {
    let com = TabelasNinho::padrao().clone();
    let sem = TabelasNinho { sons: SonsNinho::default(), ..com.clone() };
    let ta = TabelasAmarela::padrao();
    let mut achou = false;
    for i in 1..300u16 {
        // Sementes espalhadas: as pequenas e seguidas dão os mesmos primeiros sorteios do LFSR.
        let semente = i.wrapping_mul(40503);
        let mut e = Estado::vazio();
        e.preta.saude = 100;
        e.partida.ciclo = 1;
        e.ninho_b.mapa.dados.fill(0x2E);
        // Uma célula aberta no meio da terra; a operária (grupo 2) cava para dentro (M = 4).
        e.ninho_b.mapa.definir(10, 30, 0x03);
        e.adicionar_formiga(Lista::B, Formiga { x: 10, y: 30, modo: 4, tipo: 0x14, s: 0 });
        e.s.definir_semente(semente);
        let s = comparar(&e, &com, &sem, |e, t| simular_ninho(e, Cor::Preta, t, ta));
        achou |= s.contains(&17);
        assert!(s.iter().all(|&id| id == 17), "{s:?}");
    }
    assert!(achou, "alguma semente cava");
}

#[test]
fn superficie_comida_nova_pede_0x20() {
    let mut com = RegrasOriginais::default();
    let mut sem = RegrasOriginais::default();
    sem.superficie.sons = SonsSuperficie::default();
    let e = Estado::vazio();
    let (mut a, mut b) = (e.clone(), e.clone());
    com.adicionar_comida(&mut a, 150, true);
    sem.adicionar_comida(&mut b, 150, true);
    assert_eq!(sons(&a.retirar_eventos()), vec![32]);
    assert!(sons(&b.retirar_eventos()).is_empty());
    assert_eq!(a, b);
    // `_AddFood(n, 0)` (comida do jogo novo) não toca.
    let mut c = e.clone();
    com.adicionar_comida(&mut c, 150, false);
    assert!(sons(&c.retirar_eventos()).is_empty());
}

#[test]
fn inimigos_formiga_leao_cospe_pedrinha_pede_0x1e() {
    let com = TabelasInimigos::padrao().clone();
    let sem = TabelasInimigos { sons: SonsInimigos::default(), ..com.clone() };
    let ta = TabelasAmarela::padrao();
    let mut achou = false;
    for semente in 1..4000u16 {
        let mut e = Estado::vazio();
        e.formigas_leao.quantidade = 1;
        e.formigas_leao.x[0] = 40;
        e.formigas_leao.y[0] = 30;
        e.superficie.mapa.definir(40, 30, 0x51); // pedrinha no centro
        e.s.definir_semente(semente);
        let s = comparar(&e, &com, &sem, |e, t| formigas_leao(e, t, ta));
        achou |= s == vec![30];
        assert!(s.is_empty() || s == vec![30], "{s:?}");
    }
    assert!(achou, "S1(512) = 0 em alguma semente");
}

#[test]
fn amarela_comandos_pedem_ok_erro_e_aranha() {
    let com = TabelasAmarela::padrao().clone();
    let sem = TabelasAmarela { sons: SonsAmarela::default(), ..com.clone() };
    let mut e = Estado::vazio();
    e.amarela.plano = 2; // no ninho: o alarme (comando 7) é impossível
    let cmd = |c: u8| move |e: &mut Estado, t: &TabelasAmarela| {
        amarela::comando(e, t, &mut ControleJogador::default(), c);
    };
    assert_eq!(comparar(&e, &com, &sem, cmd(7)), vec![1], "alarme fora da superfície: erro");
    e.amarela.plano = 1;
    assert_eq!(comparar(&e, &com, &sem, cmd(7)), vec![15], "alarme ligado: ok");
    e.amarela.modo = 1; // aranha
    assert_eq!(comparar(&e, &com, &sem, cmd(11)), vec![2]);
    assert_eq!(comparar(&e, &com, &sem, cmd(12)), vec![41], "liga o laser");
    e.amarela.modo_aranha = 8;
    assert_eq!(comparar(&e, &com, &sem, cmd(12)), vec![2], "desliga o laser");
    let home = |e: &mut Estado, t: &TabelasAmarela| {
        let m = Modificadores { shift: false, ctrl: false, tab: false };
        amarela::tecla_de_comando(e, t, &mut ControleJogador::default(), amarela::vk::HOME, m);
    };
    assert_eq!(comparar(&e, &com, &sem, home), vec![15]);
}

/// Partidas inteiras: 400 ticks de vários saves com e sem os ids de todas as partes dão o mesmo estado.
#[test]
fn ticks_com_e_sem_sons_dao_o_mesmo_estado() {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let layout = LayoutSave::carregar(&raiz.join("data/save_layout.json")).unwrap();
    let mut total = 0;
    for nome in ["QUICK2.ANT", "Y01_CARREGADO.ANT", "Y03_CARREGADO.ANT", "SUP01_CARREGADO.ANT", "N03_carregado.ANT", "Q01_T140.ANT"] {
        let jogo = JogoSalvo::abrir(&layout, &raiz.join("tests/referencias/saves").join(nome)).unwrap();
        let mut a = Estado::de_save(&jogo).unwrap();
        let mut b = a.clone();
        let mut com = RegrasOriginais::default();
        let mut sem = RegrasOriginais::default();
        sem.quintal.sons = SonsQuintal::default();
        sem.ninho.sons = SonsNinho::default();
        sem.inimigos.sons = SonsInimigos::default();
        sem.superficie.sons = SonsSuperficie::default();
        sem.amarela.sons = SonsAmarela::default();
        let mut pedidos = std::collections::BTreeMap::new();
        for _ in 0..400 {
            tick(&mut a, &mut com);
            tick(&mut b, &mut sem);
            for id in sons(&a.retirar_eventos()) {
                *pedidos.entry(id).or_insert(0) += 1;
            }
            assert!(sons(&b.retirar_eventos()).is_empty());
            assert_eq!(a, b, "{nome}");
        }
        eprintln!("{nome}: sons pedidos (id: vezes) {pedidos:?}");
        total += pedidos.values().sum::<i32>();
    }
    assert!(total > 0);
}
