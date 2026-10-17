//! Ferramentas do modo Experimental e trocas de modo (docs/spec/telas/experimental.md, casos E-U1 a
//! E-U12). Entradas e resultados esperados vêm do pseudocódigo (`_DoTool` e rotinas chamadas); os que
//! partem do `QUICK2.ANT` usam o estado salvo pelo original na bancada QEMU.

use std::path::PathBuf;

use engine::formats::save::{Formiga, JogoSalvo, LayoutSave};
use engine::rules::estado::Estado;
use engine::rules::experimental::{
    self, arrastar, escolher_ferramenta, por_comida, pressionar, quick_para_experimental, soltar,
    subopcao, Contexto, ControleExperimental, Efeito, Ferramenta, TabelasExperimental,
};
use engine::rules::rng::GeradorS;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn quick2() -> Estado {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let jogo = JogoSalvo::abrir(&layout, &raiz().join("tests/referencias/saves/QUICK2.ANT")).unwrap();
    Estado::de_save(&jogo).unwrap()
}

/// Quintal vazio (chão 0) no Experimental, com a ferramenta dada e o gerador S na semente dada.
fn experimental(f: Ferramenta, semente: u16) -> Estado {
    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 3;
    e.partida.ferramenta_experimental = f as i16;
    e.s = GeradorS::new(semente);
    e
}

fn c() -> Contexto<'static> {
    Contexto::padrao()
}

fn ctl() -> ControleExperimental {
    ControleExperimental::novo(TabelasExperimental::padrao())
}

#[test]
fn e_u1_quick_para_experimental_no_quick2() {
    let mut e = quick2();
    assert_eq!((e.partida.tipo_jogo, e.amarela.plano, e.amarela.x, e.amarela.y), (1, 2, 34, 9));
    assert_eq!(e.ninho_b.vida.em(34, 9), Some(0xFF));
    let ef = quick_para_experimental(&mut e, c());
    assert_eq!(ef, vec![Efeito::Som { id: 2, parametro: 0 }]);
    assert_eq!(e.partida.tipo_jogo, 3);
    // _ClearMyLife apaga a marca 0xFF (a rainha que dividia a célula some da camada de vida).
    assert_eq!(e.ninho_b.vida.em(34, 9), Some(0));
    let a = &e.amarela;
    assert_eq!((a.plano, a.x, a.y, a.tipo, a.modo, a.saude, a.ovo_carregado), (1, 64, 32, 0x10, 2, 100, 0xFD));
    assert_eq!((a.alvo_x, a.alvo_y, a.mover, a.comando), (64, 32, 0, 0), "_ResetYellowVars");
    assert_eq!(e.superficie.vida.em(64, 32), Some(0), "no Experimental a amarela não é marcada");
    let s = &e.aranha;
    assert_eq!((s.x, s.y, s.dir, s.ligada, s.alvo, s.alvo_tipo, s.arrotos, s.estado), (0x400, 0x200, 0, 1, -2, -1, 10, 0));
    assert_eq!((s.destino_x, s.destino_y), (0x40, 0x40));
}

#[test]
fn e_u2_comecar_jogo_volta_ao_quick_game_no_ninho_preto() {
    let mut e = quick2();
    quick_para_experimental(&mut e, c());
    let (rx, ry) = (e.amarela.renascer_x as i32, e.amarela.renascer_y as i32);
    let mut k = ctl();
    let ef = escolher_ferramenta(&mut e, c(), &mut k, Ferramenta::ComecarJogo, None, 2);
    assert_eq!(ef, vec![Efeito::Musica(11003)]);
    assert_eq!(e.partida.tipo_jogo, 1);
    assert_eq!(e.partida.ferramenta_experimental, 7);
    let a = &e.amarela;
    assert_eq!((a.plano, a.x as i32, a.y as i32, a.tipo, a.dir, a.modo), (2, rx, ry, 0x10, 2, 0));
    assert_eq!(e.ninho_b.vida.em(rx, ry), Some(0xFF));
    // De novo fora do Experimental: o menu não faz nada.
    let antes = e.clone();
    assert!(escolher_ferramenta(&mut e, c(), &mut k, Ferramenta::Comida, Some(1), 2).is_empty());
    assert_eq!(e, antes);
}

#[test]
fn e_u3_quick_para_experimental_so_no_quick_game() {
    let mut e = quick2();
    e.partida.tipo_jogo = 2;
    let antes = e.clone();
    assert!(quick_para_experimental(&mut e, c()).is_empty());
    assert_eq!(e, antes, "Full Game: só a figura de editNoExpMenu");
}

#[test]
fn e_u4_comida_na_superficie_sobe_ate_quatro_e_ctrl_tira() {
    let mut e = experimental(Ferramenta::Comida, 0x1234);
    let cx = c();
    for esperado in [0x48, 0x49, 0x4A, 0x4B] {
        assert!(por_comida(&mut e, cx, 1, 5, 5, false));
        assert_eq!(e.superficie.mapa.em(5, 5), Some(esperado));
    }
    assert!(!por_comida(&mut e, cx, 1, 5, 5, false), "0x4B > 0x48 + 2");
    assert_eq!(e.ambiente.comida_superficie, 4);
    // Objeto (0x20) entre 0x18 e o limiar: fica como está, mas conta (fato do código).
    e.superficie.mapa.definir(6, 6, 0x20);
    assert!(por_comida(&mut e, cx, 1, 6, 6, false));
    assert_eq!((e.superficie.mapa.em(6, 6), e.ambiente.comida_superficie), (Some(0x20), 5));
    // Ctrl: 0x4B vira chão SRand1(8).
    let mut s = e.s;
    assert!(por_comida(&mut e, cx, 1, 5, 5, true));
    assert_eq!(e.superficie.mapa.em(5, 5), Some(s.rand1(8) as u8));
    assert_eq!(e.ambiente.comida_superficie, 4);
}

#[test]
fn e_u5_comida_no_ninho_conta_na_colonia_mas_ctrl_nao_desconta() {
    let mut e = experimental(Ferramenta::Comida, 7);
    let cx = c();
    e.ninho_b.mapa.definir(10, 10, 3);
    assert!(por_comida(&mut e, cx, 2, 10, 10, false));
    assert_eq!((e.ninho_b.mapa.em(10, 10), e.preta.comida), (Some(0x10), 1));
    assert!(por_comida(&mut e, cx, 2, 10, 10, true));
    assert!(e.ninho_b.mapa.em(10, 10).unwrap() < 8);
    assert_eq!(e.preta.comida, 1, "no ninho o Ctrl não mexe em _FoodB");
}

#[test]
fn e_u6_vinte_pedacos_espalhados_com_srand9() {
    let mut e = experimental(Ferramenta::Comida, 0x4321);
    let mut k = ctl();
    k.subopcoes[Ferramenta::Comida.indice()] = subopcao::VINTE_PEDACOS;
    let mut s = e.s;
    let ef = pressionar(&mut e, c(), &mut k, 1, 50, 30, false);
    assert_eq!(ef, vec![Efeito::Som { id: 32, parametro: 0 }]);
    let mut esperado = Estado::vazio();
    for _ in 0..20 {
        let x = s.rand1(9) + 50 - 4;
        let y = s.rand1(9) + 30 - 4;
        let v = esperado.superficie.mapa.em(x, y).unwrap();
        if v <= 0x4A {
            let novo = if v >= 0x48 { v + 1 } else { 0x48 };
            esperado.superficie.mapa.definir(x, y, novo);
        }
    }
    assert_eq!(e.superficie.mapa, esperado.superficie.mapa);
    assert_eq!(e.s, s);
}

#[test]
fn e_u7_formiga_na_superficie_e_no_ninho_alheio() {
    let mut e = experimental(Ferramenta::Formiga, 99);
    let mut k = ctl();
    let mut s = e.s;
    let ef = pressionar(&mut e, c(), &mut k, 1, 20, 20, false);
    assert_eq!(ef, vec![Efeito::Som { id: 28, parametro: 0 }]);
    let t = 0x10 + s.rand1(8) as u8;
    assert_eq!(e.superficie.lista.indice, 1);
    assert_eq!(e.superficie.lista.vagas[0], Formiga { x: 20, y: 20, modo: 2, tipo: t, s: 0 });
    assert_eq!(e.superficie.vida.em(20, 20), Some(t));
    // Célula ocupada: sorteia T mesmo assim, mas não põe nem toca som.
    assert!(pressionar(&mut e, c(), &mut k, 1, 20, 20, false).is_empty());
    assert_eq!(e.superficie.lista.indice, 1);
    // Vermelha no ninho preto: M = 7; linha 0 recusada.
    k.subopcoes[Ferramenta::Formiga.indice()] = subopcao::VERMELHA;
    e.ninho_b.mapa.definir(30, 10, 2);
    pressionar(&mut e, c(), &mut k, 2, 30, 10, false);
    let f = e.ninho_b.lista.vagas[0];
    assert_eq!((f.x, f.y, f.modo), (30, 10, 7));
    assert!((0x90..0x98).contains(&f.tipo));
    assert!(pressionar(&mut e, c(), &mut k, 2, 31, 0, false).is_empty());
}

#[test]
fn e_u8_inseticida_em_jato_vira_cadaver_sem_tocar_a_amarela() {
    let mut e = quick2();
    e.partida.tipo_jogo = 3;
    e.partida.ferramenta_experimental = Ferramenta::Inseticida as i16;
    let mut k = ctl();
    k.subopcoes[Ferramenta::Inseticida.indice()] = subopcao::JATO;
    let (i, f) = e
        .superficie
        .lista
        .ativas()
        .map(|(i, f)| (i, *f))
        .find(|(_, f)| e.superficie.mapa.em(f.x as i32, f.y as i32).unwrap() <= 0x17)
        .expect("formiga da lista A em chão");
    let (x, y) = (f.x as i32, f.y as i32);
    let chao = e.superficie.mapa.em(x, y).unwrap();
    let ef = pressionar(&mut e, c(), &mut k, 1, x, y, false);
    assert_eq!(ef.len(), 8, "oito jatos, um som cada");
    assert_eq!(e.superficie.lista.vagas[i].tipo, 0);
    assert_eq!(e.superficie.vida.em(x, y), Some(0));
    let base = if f.tipo & 0x80 != 0 { 0x14 } else { 0x10 };
    assert!(chao <= 0x17);
    assert!((base..base + 4).contains(&e.superficie.mapa.em(x, y).unwrap()), "cadáver da cor");
}

#[test]
fn e_u9_cheiro_soma_70_e_suaviza() {
    let mut e = experimental(Ferramenta::Cheiro, 1);
    let mut k = ctl();
    k.subopcoes[Ferramenta::Cheiro.indice()] = subopcao::ALARME;
    let ef = pressionar(&mut e, c(), &mut k, 1, 10, 11, false);
    assert_eq!(ef, vec![Efeito::Som { id: 27, parametro: 0 }]);
    let g = &e.feromonios.alarme;
    // (5, 5): 70 → 35 → 19; as quatro vizinhas: 35 / 4 = 8 → 4.
    assert_eq!(g.em(5, 5), Some(19));
    for (x, y) in [(5, 4), (6, 5), (5, 6), (4, 5)] {
        assert_eq!(g.em(x, y), Some(4), "({x}, {y})");
    }
    // Ninho: nada.
    assert!(pressionar(&mut e, c(), &mut k, 2, 10, 11, false).is_empty());
}

#[test]
fn e_u10_encher_no_ninho_desconta_a_escavacao() {
    let mut e = quick2();
    e.partida.tipo_jogo = 3;
    e.partida.ferramenta_experimental = Ferramenta::Cavar as i16;
    let mut k = ctl();
    k.subopcoes[Ferramenta::Cavar.indice()] = subopcao::ENCHER;
    // Uma célula de túnel do ninho preto do QUICK2 sem formiga.
    let (x, y) = (2..64)
        .flat_map(|y| (1..63).map(move |x| (x, y)))
        .find(|&(x, y)| e.ninho_b.mapa.em(x, y).unwrap() < 8 && e.ninho_b.vida.em(x, y) == Some(0))
        .unwrap();
    let antes = e.preta.clone();
    let ef = pressionar(&mut e, c(), &mut k, 2, x, y, false);
    assert_eq!(ef, vec![Efeito::Som { id: 18, parametro: 0 }]);
    assert_eq!(e.ninho_b.mapa.em(x, y), Some(0x2E));
    assert_eq!(e.ninho_b.saida.em(x, y), Some(0));
    assert_eq!(e.preta.tiles_escavados, antes.tiles_escavados - 1);
    assert_eq!(e.preta.soma_x_escavada, antes.soma_x_escavada - x);
    assert_eq!(e.preta.soma_y_escavada, antes.soma_y_escavada - y);
    assert_eq!((e.preta.centro_x, e.preta.centro_y), (antes.centro_x, antes.centro_y), "centro não recalculado");
    // Ctrl inverte: cava de volta (terra maciça é terra).
    let ef = pressionar(&mut e, c(), &mut k, 2, x, y, true);
    assert_eq!(ef, vec![Efeito::Som { id: 19, parametro: 0 }]);
    assert!(e.ninho_b.mapa.em(x, y).unwrap() < 0x2E);
    // Encher na superfície: aviso 21 da lista 1010.
    let ef = pressionar(&mut e, c(), &mut k, 1, 5, 5, false);
    assert_eq!(ef[1], Efeito::Aviso { lista: 1010, indice: 21 });
}

#[test]
fn e_u11_pedrinhas_e_apagar_com_ctrl() {
    let mut e = experimental(Ferramenta::Barreira, 0x0BEE);
    let mut k = ctl();
    k.subopcoes[Ferramenta::Barreira.indice()] = subopcao::PEDRAS;
    let mut s = e.s;
    let ef = pressionar(&mut e, c(), &mut k, 1, 3, 3, false);
    let pedra = 0x51 + s.rand1(3) as u8;
    let som = s.rand1(10000) + 2000;
    assert_eq!(e.superficie.mapa.em(3, 3), Some(pedra));
    assert_eq!(ef, vec![Efeito::Som { id: 10, parametro: som }]);
    // Ctrl apaga pedra/muro (0x50 < v < 0x68) com SRand1(16), sem som.
    let ef = pressionar(&mut e, c(), &mut k, 1, 3, 3, true);
    assert!(ef.is_empty());
    assert_eq!(e.superficie.mapa.em(3, 3), Some(s.rand1(16) as u8));
    // Muro no plano do ninho: nada.
    k.subopcoes[Ferramenta::Barreira.indice()] = subopcao::MUROS;
    assert!(pressionar(&mut e, c(), &mut k, 2, 3, 3, false).is_empty());
}

#[test]
fn e_u12_arrasto_repete_cheiro_e_inseticida_mas_nao_comida() {
    let mut e = experimental(Ferramenta::Comida, 5);
    let mut k = ctl();
    pressionar(&mut e, c(), &mut k, 1, 8, 8, false);
    assert_eq!(e.superficie.mapa.em(8, 8), Some(0x48));
    // Mesma célula: comida (4) não repete.
    assert!(arrastar(&mut e, c(), &mut k, 1, 8, 8, false).is_empty());
    assert_eq!(e.superficie.mapa.em(8, 8), Some(0x48));
    soltar(&mut k);
    e.partida.ferramenta_experimental = Ferramenta::Cheiro as i16;
    pressionar(&mut e, c(), &mut k, 1, 8, 8, false);
    let um = e.feromonios.ninho_b.em(4, 4).unwrap();
    assert_eq!(arrastar(&mut e, c(), &mut k, 1, 8, 8, false).len(), 1, "cheiro repete parado");
    assert!(e.feromonios.ninho_b.em(4, 4).unwrap() > um);
    // Validade: fora do mapa, nada.
    assert!(experimental::posicao_valida(1, 127, 63));
    assert!(!experimental::posicao_valida(2, 64, 0));
}

/// Caso E-01 no original (bancada QEMU, 2026-10-17; roteiro em docs/spec/telas/experimental.md):
/// Experimental novo, pausado; com `_CurExpTool` e `_ExpSubStates` escritos na memória, cliques nas
/// células (57, 28) comida (é pedra 0x57: nada muda), (59, 28) formiga preta, (61, 28) muro, (62, 28)
/// pedrinha (pedra 0x59: só o sorteio do som) e (59, 28) inseticida em jato. O gerador S lido antes
/// do primeiro clique era 0x7CFE e depois do último 0xCBEF (três passos: formiga, som da pedrinha,
/// cadáver). O port, a partir do mesmo estado, chega ao mesmo save byte a byte.
#[test]
fn e_01_sequencia_de_ferramentas_igual_ao_original() {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let ler = |n: &str| JogoSalvo::abrir(&layout, &raiz().join("tests/referencias/saves").join(n)).unwrap();
    let mut e = Estado::de_save(&ler("E01_ANTES.ANT")).unwrap();
    e.s = GeradorS::new(0x7CFE);
    let mut k = ctl();
    let passos = [
        (Ferramenta::Comida, subopcao::UM_PEDACO, 57),
        (Ferramenta::Formiga, subopcao::PRETA, 59),
        (Ferramenta::Barreira, subopcao::MUROS, 61),
        (Ferramenta::Barreira, subopcao::PEDRAS, 62),
        (Ferramenta::Inseticida, subopcao::JATO, 59),
    ];
    for (f, sub, x) in passos {
        e.partida.ferramenta_experimental = f as i16;
        k.subopcoes[f.indice()] = sub;
        pressionar(&mut e, c(), &mut k, 1, x, 28, false);
        soltar(&mut k);
    }
    assert_eq!(e.s.semente(), 0xCBEF);
    let esperado = std::fs::read(raiz().join("tests/referencias/saves/E01_DEPOIS.ANT")).unwrap();
    let obtido = e.para_save(&layout).unwrap();
    assert!(obtido.bytes() == esperado.as_slice(), "save do port difere do original");
}

/// Caso E-02 no original (bancada QEMU, 2026-10-17): Experimental novo e pausado. (a) Vinte pedaços
/// de comida em (64, 31) e spray em (55, 26), com S = 0xC14E antes: o original chega a S = 0x0E20 e
/// `_FoodA` = 19 (15 células). (b) No ninho preto (vista em (20, 0)), com S = 0x0E20: cavar (28, 4),
/// encher (28, 4) e encher (22, 6) (terra: não muda). O port reproduz os dois trechos byte a byte.
#[test]
fn e_02_comida_espalhada_spray_e_cavar_encher_iguais_ao_original() {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let dir = raiz().join("tests/referencias/saves");
    let ler = |n: &str| Estado::de_save(&JogoSalvo::abrir(&layout, &dir.join(n)).unwrap()).unwrap();
    let bytes = |n: &str| std::fs::read(dir.join(n)).unwrap();
    let mut k = ctl();

    let mut e = ler("E02_0.ANT");
    e.s = GeradorS::new(0xC14E);
    e.partida.ferramenta_experimental = Ferramenta::Comida as i16;
    k.subopcoes[Ferramenta::Comida.indice()] = subopcao::VINTE_PEDACOS;
    pressionar(&mut e, c(), &mut k, 1, 64, 31, false);
    soltar(&mut k);
    e.partida.ferramenta_experimental = Ferramenta::Inseticida as i16;
    k.subopcoes[Ferramenta::Inseticida.indice()] = subopcao::SPRAY;
    pressionar(&mut e, c(), &mut k, 1, 55, 26, false);
    soltar(&mut k);
    assert_eq!(e.s.semente(), 0x0E20);
    assert!(e.para_save(&layout).unwrap().bytes() == bytes("E02_2.ANT").as_slice(), "trecho (a)");

    let mut e = ler("E02_3.ANT");
    e.s = GeradorS::new(0x0E20);
    e.partida.ferramenta_experimental = Ferramenta::Cavar as i16;
    k.subopcoes[Ferramenta::Cavar.indice()] = subopcao::CAVAR;
    pressionar(&mut e, c(), &mut k, 2, 28, 4, false);
    soltar(&mut k);
    k.subopcoes[Ferramenta::Cavar.indice()] = subopcao::ENCHER;
    for (x, y) in [(28, 4), (22, 6)] {
        pressionar(&mut e, c(), &mut k, 2, x, y, false);
        soltar(&mut k);
    }
    assert_eq!(e.s.semente(), 0x1C40);
    assert!(e.para_save(&layout).unwrap().bytes() == bytes("E02_5.ANT").as_slice(), "trecho (b)");
}
