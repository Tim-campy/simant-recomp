//! Formiga-leão, aranha, lagarta e bicho-de-conta (docs/spec/regras/inimigos.md, casos I1–I12).
//!
//! Os testes de unidade montam um `Estado` vazio (quintal, chão 0) e chamam as funções de
//! `rules::inimigos` diretamente. Os testes "contra o original" partem dos saves da bancada QEMU
//! (`tests/referencias/saves/I*.ANT`: o estado logo depois do load e o estado final no original) e
//! comparam só o que não depende do gerador S (ver tests/referencias/README.md, casos I-01 a I-03).

use std::path::PathBuf;

use engine::formats::save::{Formiga, JogoSalvo, LayoutSave};
use engine::recursos::Recursos;
use engine::rules::amarela::TabelasAmarela;
use engine::rules::estado::{Estado, Evento};
use engine::rules::geracao::TabelasGeracao;
use engine::rules::inimigos::{self, seno_calculado, TabelasInimigos};
use engine::rules::rng::GeradorS;
use engine::rules::tick::{tick, RegrasOriginais};

fn t() -> &'static TabelasInimigos {
    TabelasInimigos::padrao()
}

/// Quintal vazio: chão 0, Quick Game, gerador S com a semente dada.
fn quintal(semente: u16) -> Estado {
    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 1;
    e.ambiente.terreno = 0;
    e.ambiente.barreira = 0x50;
    e.s = GeradorS::new(semente);
    e.aranha.alvo = -2;
    e.aranha.arrotos = 10;
    e
}

fn formiga(x: u8, y: u8, tipo: u8) -> Formiga {
    Formiga {
        x,
        y,
        modo: 2,
        tipo,
        s: 0,
    }
}

fn mapa(e: &Estado, x: i32, y: i32) -> u8 {
    e.superficie.mapa.em(x, y).unwrap()
}

/// Uma formiga-leão parada (estado 0) na vaga k.
fn por_formiga_leao(e: &mut Estado, k: usize, x: u8, y: u8) {
    let fl = &mut e.formigas_leao;
    fl.x[k] = x;
    fl.y[k] = y;
    fl.m[k] = 0;
    fl.s[k] = 0;
    fl.t[k] = 0;
    fl.quantidade = fl.quantidade.max(k as i16 + 1);
    fl.iniciais = fl.quantidade;
    e.superficie.mapa.definir(x as i32, y as i32, 0x38);
}

// ---------------------------------------------------------------------------------------------
// Bicho-de-conta (I1, I2)
// ---------------------------------------------------------------------------------------------

/// Semente em que a primeira vaga sorteia S4 ≠ 0, depois S4 = 0 (vira) e S1(3) = `giro`.
fn semente_para_virar(giro: i32) -> u16 {
    (1..=u16::MAX)
        .find(|&s| {
            let mut g = GeradorS::new(s);
            g.mascara(4) != 0 && g.mascara(4) == 0 && g.rand1(3) == giro
        })
        .unwrap()
}

#[test]
fn i1_vaga_0_e_um_bicho_fantasma_em_0_0() {
    // Vaga 0 nunca inicializada (0, 0, dir 0, chão 0); as outras vagas não importam aqui.
    let mut e = quintal(semente_para_virar(2));
    e.lagarta_e_bichos.lagarta_estado = 1; // sem lagarta nova
    e.lagarta_e_bichos.lagarta_dir = 9; // direção inválida: a lagarta não faz nada
    inimigos::bichos_de_conta(&mut e, t());
    // Virou de 0 para 1: redesenha 0x73 em (0, 0); o passo (1, −1) sai do mapa e ele fica.
    let lb = &e.lagarta_e_bichos;
    assert_eq!((lb.bicho_x[0], lb.bicho_y[0], lb.bicho_dir[0]), (0, 0, 1));
    assert_eq!(mapa(&e, 0, 0), 0x73);
}

#[test]
fn i1_quick2_tem_o_fantasma_desenhado() {
    let Some(j) = save_ref("QUICK2.ANT") else {
        return;
    };
    let e = Estado::de_save(&j).unwrap();
    let lb = &e.lagarta_e_bichos;
    assert_eq!(
        (lb.bicho_x, lb.bicho_y, lb.bicho_dir),
        ([0, 32, 49], [0, 46, 8], [0, 6, 2])
    );
    assert_eq!(mapa(&e, 0, 0), t().sow_tab[0]);
    assert_eq!(mapa(&e, 32, 46), t().sow_tab[6]);
    assert_eq!(mapa(&e, 49, 8), t().sow_tab[2]);
}

#[test]
fn i2_bicho_anda_uma_celula_e_repoe_o_chao() {
    let mut e = quintal(0x2468);
    // Chão variado para ver o _SowSave voltar.
    for (i, c) in e.superficie.mapa.dados.iter_mut().enumerate() {
        *c = (i % 16) as u8;
    }
    let lb = &mut e.lagarta_e_bichos;
    for (v, (x, y, d)) in [(60, 30, 2), (20, 10, 6), (90, 50, 0)]
        .into_iter()
        .enumerate()
    {
        lb.bicho_x[v] = x;
        lb.bicho_y[v] = y;
        lb.bicho_dir[v] = d;
        lb.bicho_chao[v] = ((x as usize * 64 + y as usize) % 16) as i16;
    }
    for v in 0..3 {
        let (x, y, d) = (lb.bicho_x[v], lb.bicho_y[v], lb.bicho_dir[v]);
        e.superficie
            .mapa
            .definir(x as i32, y as i32, t().sow_tab[d as usize]);
    }
    let mut andou = 0;
    for _ in 0..300 {
        let antes = e.lagarta_e_bichos.clone();
        inimigos::bichos_de_conta(&mut e, t());
        let lb = &e.lagarta_e_bichos;
        for v in 0..3 {
            let (x0, y0) = (antes.bicho_x[v] as i32, antes.bicho_y[v] as i32);
            let (x, y) = (lb.bicho_x[v] as i32, lb.bicho_y[v] as i32);
            assert!(
                (x - x0).abs() <= 1 && (y - y0).abs() <= 1,
                "no máximo 1 célula por chamada"
            );
            assert_eq!(
                mapa(&e, x, y),
                t().sow_tab[lb.bicho_dir[v] as usize],
                "tile = _SowTab[dir]"
            );
            if (x, y) != (x0, y0) {
                andou += 1;
                assert_eq!(
                    mapa(&e, x0, y0) as i16,
                    antes.bicho_chao[v],
                    "o chão anterior volta"
                );
                assert_eq!(
                    lb.bicho_chao[v],
                    ((x * 64 + y) % 16) as i16,
                    "guarda o chão novo"
                );
                assert_eq!(
                    (x - x0, y - y0),
                    (
                        t().dx8[lb.bicho_dir[v] as usize] as i32,
                        t().dy8[lb.bicho_dir[v] as usize] as i32
                    )
                );
            }
        }
    }
    assert!(andou > 100, "os bichos andaram {andou} vezes");
}

#[test]
fn bicho_nao_entra_em_formiga_nem_em_obstaculo() {
    let mut e = quintal(0x1111);
    let lb = &mut e.lagarta_e_bichos;
    lb.bicho_x = [10, 30, 50];
    lb.bicho_y = [10, 30, 50];
    lb.bicho_dir = [2, 2, 2];
    // Cercados: formiga na vaga 0, pedrinha nas vizinhas das vagas 1 e 2.
    for v in 0..3 {
        let (x, y) = (lb.bicho_x[v] as i32, lb.bicho_y[v] as i32);
        for d in 0..8 {
            let (vx, vy) = (x + t().dx8[d] as i32, y + t().dy8[d] as i32);
            if v == 0 {
                e.superficie.vida.definir(vx, vy, 0x12);
            } else {
                e.superficie.mapa.definir(vx, vy, 0x51);
            }
        }
    }
    for _ in 0..100 {
        inimigos::bichos_de_conta(&mut e, t());
    }
    assert_eq!(e.lagarta_e_bichos.bicho_x, [10, 30, 50]);
    assert_eq!(e.lagarta_e_bichos.bicho_y, [10, 30, 50]);
}

// ---------------------------------------------------------------------------------------------
// Lagarta (I3, I4)
// ---------------------------------------------------------------------------------------------

/// Lagarta como no QUICK2.ANT: cabeça (7, 58), direção 1 (direita), fase 1.
fn lagarta_quick2(e: &mut Estado) {
    let lb = &mut e.lagarta_e_bichos;
    lb.lagarta_estado = 1;
    lb.lagarta_x = 7;
    lb.lagarta_y = 58;
    lb.lagarta_dir = 1;
    lb.lagarta_fase = 1;
    lb.lagarta_chao = [1, 2, 3, 4, 5, 6];
    // Bichos-de-conta longe, cercados por pedrinhas (não interferem).
    lb.bicho_x = [100, 100, 100];
    lb.bicho_y = [5, 5, 5];
    for x in 99..=101 {
        for y in 4..=6 {
            e.superficie.mapa.definir(x, y, 0x51);
        }
    }
    e.superficie.mapa.definir(7, 58, 0x6B);
    e.superficie.mapa.definir(6, 58, 0x6A);
    for x in 3..=5 {
        e.superficie.mapa.definir(x, 58, 0x69);
    }
}

fn linha(e: &Estado, y: i32, xs: std::ops::RangeInclusive<i32>) -> Vec<u8> {
    xs.map(|x| mapa(e, x, y)).collect()
}

#[test]
fn i3_lagarta_avanca_uma_celula_a_cada_cinco_chamadas() {
    let mut e = quintal(0x4321);
    lagarta_quick2(&mut e);
    // s = 0: a cabeça avança para (8, 58).
    inimigos::lagarta(&mut e, t());
    let lb = &e.lagarta_e_bichos;
    assert_eq!((lb.lagarta_x, lb.lagarta_y, lb.lagarta_fase), (8, 58, 5));
    assert_eq!(
        linha(&e, 58, 2..=9),
        [0, 0x69, 0x69, 0x69, 0x69, 0x69, 0x6B, 0]
    );
    assert_eq!(
        e.lagarta_e_bichos.lagarta_chao[8 % 6],
        0,
        "chão sob a cabeça nova guardado"
    );
    // s = 4: repõe o chão em atrás(5) = (3, 58) com _PillarMap[3 % 6] e grava 0x6A em atrás(4).
    inimigos::lagarta(&mut e, t());
    assert_eq!(
        linha(&e, 58, 2..=9),
        [0, 4, 0x6A, 0x69, 0x69, 0x69, 0x6B, 0]
    );
    // s = 3, 2, 1: o 0x6A anda para a frente.
    inimigos::lagarta(&mut e, t());
    assert_eq!(
        linha(&e, 58, 2..=9),
        [0, 4, 0x69, 0x6A, 0x69, 0x69, 0x6B, 0]
    );
    inimigos::lagarta(&mut e, t());
    inimigos::lagarta(&mut e, t());
    assert_eq!(
        linha(&e, 58, 2..=9),
        [0, 4, 0x69, 0x69, 0x69, 0x6A, 0x6B, 0]
    );
    assert_eq!(e.lagarta_e_bichos.lagarta_x, 8);
    // s = 0 de novo: (9, 58).
    inimigos::lagarta(&mut e, t());
    assert_eq!(e.lagarta_e_bichos.lagarta_x, 9);
    assert_eq!(
        linha(&e, 58, 2..=10),
        [0, 4, 0x69, 0x69, 0x69, 0x69, 0x69, 0x6B, 0]
    );
}

#[test]
fn i3_lagarta_sai_do_quintal_e_outra_nasce_na_borda() {
    let mut e = quintal(0x0BAD);
    let lb = &mut e.lagarta_e_bichos;
    lb.lagarta_estado = 1;
    lb.lagarta_x = 133;
    lb.lagarta_y = 20;
    lb.lagarta_dir = 1;
    lb.lagarta_fase = 1;
    lb.bicho_dir = [0; 3];
    inimigos::lagarta(&mut e, t());
    assert_eq!(e.lagarta_e_bichos.lagarta_estado, 1, "x = 134 ainda vale");
    for _ in 0..5 {
        inimigos::lagarta(&mut e, t());
    }
    assert_eq!(
        e.lagarta_e_bichos.lagarta_estado, 0,
        "x = 135 passou do limite"
    );
    // Próxima chamada: _MakeAPill. Repete os sorteios dos bichos-de-conta num gerador espelho.
    let mut espelho = e.clone();
    inimigos::bichos_de_conta(&mut espelho, t());
    let dir = espelho.s.rand1(4) as i16;
    let outro = match dir {
        0 | 2 => espelho.s.rand1(128),
        _ => espelho.s.rand1(64),
    } as i16;
    inimigos::lagarta(&mut e, t());
    let lb = &e.lagarta_e_bichos;
    assert_eq!(
        (lb.lagarta_estado, lb.lagarta_fase, lb.lagarta_dir),
        (1, 4, dir)
    );
    let esperado = match dir {
        0 => (outro, 63),
        1 => (0, outro),
        2 => (outro, 0),
        _ => (127, outro),
    };
    assert_eq!((lb.lagarta_x, lb.lagarta_y), esperado);
    assert_eq!(
        mapa(&e, esperado.0 as i32, esperado.1 as i32),
        [0x6C, 0x6B, 0x6F, 0x68][dir as usize]
    );
    assert_eq!(
        e.s, espelho.s,
        "nenhum sorteio além de S1(4) e um S1(128)/S1(64)"
    );
}

#[test]
fn i4_lagarta_cercada_vira_comida() {
    let mut e = quintal(0x7777);
    lagarta_quick2(&mut e);
    e.superficie.mapa.definir(4, 58, 0x30); // chão "alto" (borda de armadilha): não vira comida
    e.lagarta_e_bichos.lagarta_chao[4] = 0x30;
    // Formiga na frente (8, 58) e mais 5 no 3 × 3 da cabeça.
    for (x, y) in [(8, 58), (8, 57), (8, 59), (7, 57), (7, 59), (6, 57)] {
        e.superficie.vida.definir(x, y, 0x12);
    }
    inimigos::lagarta(&mut e, t());
    assert_eq!(e.lagarta_e_bichos.lagarta_estado, 0);
    // Corpo: cabeça (7) e 5 atrás (2..6), o chão volta e vira 0x4B se < 0x18.
    assert_eq!(
        linha(&e, 58, 1..=8),
        [0, 0x4B, 0x4B, 0x30, 0x4B, 0x4B, 0x4B, 0]
    );
}

#[test]
fn lagarta_bloqueada_com_poucas_formigas_so_espera() {
    let mut e = quintal(0x7777);
    lagarta_quick2(&mut e);
    for (x, y) in [(8, 58), (8, 57)] {
        e.superficie.vida.definir(x, y, 0x12);
    }
    let antes = e.superficie.mapa.clone();
    for _ in 0..20 {
        inimigos::lagarta(&mut e, t());
    }
    let lb = &e.lagarta_e_bichos;
    assert_eq!(
        (lb.lagarta_estado, lb.lagarta_x, lb.lagarta_fase),
        (1, 7, 1)
    );
    assert_eq!(
        linha(&e, 58, 2..=9),
        antes
            .dados
            .iter()
            .skip(2 * 64 + 58)
            .step_by(64)
            .take(8)
            .copied()
            .collect::<Vec<_>>()
    );
}

#[test]
fn lagarta_nao_existe_na_casa() {
    let mut e = quintal(0x1234);
    e.ambiente.terreno = 1;
    let s = e.s;
    inimigos::lagarta(&mut e, t());
    assert_eq!(e.s, s, "na casa nem os bichos-de-conta sorteiam");
    assert_eq!(e.lagarta_e_bichos.lagarta_estado, 0);
}

// ---------------------------------------------------------------------------------------------
// Formiga-leão (I5, I6, I7)
// ---------------------------------------------------------------------------------------------

#[test]
fn i5_formiga_leao_come_a_formiga_do_centro() {
    let mut e = quintal(0x5A5A);
    por_formiga_leao(&mut e, 0, 40, 20);
    e.adicionar_formiga(engine::formats::save::Lista::A, formiga(10, 10, 0x12));
    e.adicionar_formiga(engine::formats::save::Lista::A, formiga(40, 20, 0x14));
    e.adicionar_formiga(engine::formats::save::Lista::A, formiga(11, 11, 0x93));
    let mut s = e.s;
    inimigos::formigas_leao(&mut e, t(), TabelasAmarela::padrao());
    s.mascara(8); // só o sorteio do som
    assert_eq!(
        e.s, s,
        "nada de S1(1024) (há formigas-leão) nem de S1(8) (o centro já tinha presa)"
    );
    let l = &e.superficie.lista;
    assert_eq!(l.indice, 2);
    assert_eq!(
        (l.vagas[0].tipo, l.vagas[1].tipo, l.vagas[2].tipo),
        (0x12, 0x93, 0x93),
        "deslocou a 3ª; a última fica repetida"
    );
    assert_eq!(e.superficie.vida.em(40, 20), Some(0));
    assert_eq!(e.preta.comidas_por_predadores, 1);
    let fl = &e.formigas_leao;
    assert_eq!((fl.m[0], fl.s[0], fl.t[0]), (1, 50, 1));
    assert_eq!(mapa(&e, 40, 20), 0x39);

    // Ciclo completo: T sobe até 4, S desce até 0 (com quadros sorteados), volta e conta a refeição.
    let mut chamadas = 1;
    while e.formigas_leao.m[0] != 0 {
        inimigos::formigas_leao(&mut e, t(), TabelasAmarela::padrao());
        chamadas += 1;
        assert!((0x38..=0x3E).contains(&mapa(&e, 40, 20)));
        assert!(chamadas < 200);
    }
    // 1 (pega) + 3 (T 2..4) + 50 (S) + 1 (vira estado 2) + 5 (T 3..0 e volta) = 60, mais uma chamada
    // por quadro sorteado igual a 3 (o T volta a subir).
    assert!((60..=85).contains(&chamadas), "{chamadas}");
    assert_eq!(e.partida.comidas_por_formiga_leao, 1);
    assert_eq!(mapa(&e, 40, 20), 0x38);
}

#[test]
fn i5_formiga_leao_olha_uma_vizinha_sorteada() {
    // Presa vermelha na vizinha sorteada pelo primeiro S1(8).
    let semente = 0x3C3C;
    let mut g = GeradorS::new(semente);
    let k = g.rand1(8) as usize + 1;
    let (dx, dy) = (t().dx9[k] as i32, t().dy9[k] as i32);
    let mut e = quintal(semente);
    por_formiga_leao(&mut e, 0, 40, 20);
    let (px, py) = ((40 + dx) as u8, (20 + dy) as u8);
    e.adicionar_formiga(engine::formats::save::Lista::A, formiga(px, py, 0x95));
    inimigos::formigas_leao(&mut e, t(), TabelasAmarela::padrao());
    assert_eq!(e.superficie.lista.indice, 0);
    assert_eq!(e.vermelha.comidas_por_predadores, 1);
    assert_eq!(e.formigas_leao.m[0], 1);
    // Sem presa: dois S1(8) por tick (o segundo é sorteado mesmo sem uso).
    let mut e = quintal(semente);
    por_formiga_leao(&mut e, 0, 40, 20);
    let mut s = e.s;
    inimigos::formigas_leao(&mut e, t(), TabelasAmarela::padrao());
    s.rand1(8);
    s.rand1(8);
    assert_eq!(e.s, s);
}

#[test]
fn i5_formiga_leao_pega_a_amarela() {
    let mut e = quintal(0x1357);
    por_formiga_leao(&mut e, 0, 40, 20);
    e.amarela.plano = 1;
    e.amarela.x = 40;
    e.amarela.y = 20;
    e.amarela.tipo = 0x10;
    e.superficie.vida.definir(40, 20, 0xFF);
    inimigos::formigas_leao(&mut e, t(), TabelasAmarela::padrao());
    assert_eq!(e.formigas_leao.m[0], 1);
    assert_eq!(
        e.preta.comidas_por_predadores, 1,
        "_YellowDeath(2) conta em _BAntsEaten"
    );
    assert_eq!(e.superficie.vida.em(40, 20), Some(0));
}

#[test]
fn i6_formiga_leao_cercada_morre_e_a_seguinte_pula_o_tick() {
    let mut e = quintal(0x2222);
    for (k, x) in [(0, 20u8), (1, 50), (2, 80)] {
        por_formiga_leao(&mut e, k, x, 30);
        e.formigas_leao.m[k] = 1;
        e.formigas_leao.s[k] = 10;
        e.formigas_leao.t[k] = 2;
    }
    for d in 0..7 {
        e.superficie
            .vida
            .definir(20 + t().dx8[d] as i32, 30 + t().dy8[d] as i32, 0x12);
    }
    inimigos::formigas_leao(&mut e, t(), TabelasAmarela::padrao());
    assert_eq!(e.retirar_eventos(), vec![Evento::Mensagem(10050)]);
    assert_eq!(mapa(&e, 20, 30), 0x3F);
    let fl = &e.formigas_leao;
    assert_eq!(fl.quantidade, 2);
    assert_eq!(
        (fl.x[0], fl.t[0]),
        (50, 2),
        "a de x = 50 foi para a vaga 0 e não rodou"
    );
    assert_eq!((fl.x[1], fl.t[1]), (80, 3), "a de x = 80 rodou (T 2 → 3)");
}

#[test]
fn i7_pedrinha_na_armadilha_e_cuspida() {
    // Semente com S1(512) = 0 na primeira chamada.
    let semente = (1..=u16::MAX)
        .find(|&s| GeradorS::new(s).rand1(512) == 0)
        .unwrap();
    let mut g = GeradorS::new(semente);
    g.rand1(512);
    let d = g.mascara(8) as usize;
    let mut e = quintal(semente);
    por_formiga_leao(&mut e, 0, 40, 20);
    e.superficie.mapa.definir(40, 20, 0x51);
    e.superficie.vida.definir(40, 20, 0x12); // formiga no centro: ignorada enquanto há pedrinha
    inimigos::formigas_leao(&mut e, t(), TabelasAmarela::padrao());
    let (ax, ay) = (40 + 2 * t().dx8[d] as i32, 20 + 2 * t().dy8[d] as i32);
    assert_eq!(mapa(&e, ax, ay), 0x51);
    assert_eq!(mapa(&e, 40, 20), 0x39);
    let fl = &e.formigas_leao;
    assert_eq!((fl.m[0], fl.s[0], fl.t[0]), (1, 25, 1));
    assert_eq!(e.preta.comidas_por_predadores, 0);

    // Sem o S1(512) = 0, nada acontece e só um sorteio é feito.
    let semente = (1..=u16::MAX)
        .find(|&s| GeradorS::new(s).rand1(512) != 0)
        .unwrap();
    let mut e = quintal(semente);
    por_formiga_leao(&mut e, 0, 40, 20);
    e.superficie.mapa.definir(40, 20, 0x4A);
    let mut s = e.s;
    inimigos::formigas_leao(&mut e, t(), TabelasAmarela::padrao());
    s.rand1(512);
    assert_eq!(e.s, s);
    assert_eq!((e.formigas_leao.m[0], mapa(&e, 40, 20)), (0, 0x4A));
}

#[test]
fn formiga_leao_nova_quando_nao_resta_nenhuma() {
    let semente = (1..=u16::MAX)
        .find(|&s| GeradorS::new(s).rand1(1024) == 0)
        .unwrap();
    let mut e = quintal(semente);
    e.formigas_leao.iniciais = 2;
    inimigos::formigas_leao(&mut e, t(), TabelasAmarela::padrao());
    let fl = &e.formigas_leao;
    assert_eq!(fl.quantidade, 1);
    let (x, y) = (fl.x[0] as i32, fl.y[0] as i32);
    assert_eq!(mapa(&e, x, y), 0x38);
    let bordas: Vec<u8> = (0..8)
        .map(|d| mapa(&e, x + t().dx8[d] as i32, y + t().dy8[d] as i32))
        .collect();
    assert_eq!(bordas, [0x31, 0x32, 0x34, 0x37, 0x36, 0x35, 0x33, 0x30]);
    // Sem formigas-leão iniciais (casa), nem o S1(1024) é sorteado.
    let mut e = quintal(semente);
    let s = e.s;
    inimigos::formigas_leao(&mut e, t(), TabelasAmarela::padrao());
    assert_eq!(e.s, s);
}

// ---------------------------------------------------------------------------------------------
// Aranha (I8–I11)
// ---------------------------------------------------------------------------------------------

/// Aranha ligada no centro da célula (cx, cy), estado `estado`, direção `dir`.
fn por_aranha(e: &mut Estado, cx: i16, cy: i16, estado: i16, dir: i16) {
    let a = &mut e.aranha;
    a.x = cx * 16 + 8;
    a.y = cy * 16 + 8;
    a.ligada = 1;
    a.estado = estado;
    a.dir = dir;
    a.alvo = -2;
}

#[test]
fn i8_aranha_aparece_por_cima_ou_por_baixo() {
    let mut e = quintal(0x9ABC);
    let mut n = 0;
    while e.aranha.ligada == 0 {
        inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
        n += 1;
        assert!(n < 10_000);
    }
    let a = &e.aranha;
    assert_eq!(a.estado, 1);
    assert!((512..1536).contains(&a.x));
    assert!(
        matches!((a.y, a.dir), (1, 4) | (0x3FF, 0)),
        "{:?}",
        (a.y, a.dir)
    );
    // No Tutorial, nunca (e sem sorteio).
    let mut e = quintal(0x9ABC);
    e.partida.tipo_jogo = 0;
    let s = e.s;
    for _ in 0..2000 {
        inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
    }
    assert_eq!((e.aranha.ligada, e.s), (0, s));
}

#[test]
fn aranha_sai_pela_borda_e_desliga() {
    let mut e = quintal(0x1);
    por_aranha(&mut e, 64, 0, 1, 0);
    e.aranha.y = 2;
    inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
    assert_eq!(e.aranha.y, -2);
    assert_eq!(e.aranha.ligada, 0);
}

#[test]
fn i9_aranha_ve_a_formiga_a_frente_persegue_e_come() {
    let mut e = quintal(0x0F0F);
    por_aranha(&mut e, 40, 20, 0, 2);
    e.adicionar_formiga(engine::formats::save::Lista::A, formiga(45, 20, 0x12));
    inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
    assert_eq!(
        (e.aranha.estado, e.aranha.alvo, e.aranha.alvo_tipo),
        (2, 0, 0x12)
    );
    let mut n = 0;
    while e.aranha.estado == 2 {
        inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
        n += 1;
        assert!(n < 20);
    }
    assert_eq!(e.aranha.estado, 3);
    assert_eq!(e.superficie.lista.vagas[0].tipo, 0, "presa com T = 0");
    assert_eq!(e.superficie.vida.em(45, 20), Some(0));
    assert_eq!(e.preta.comidas_por_predadores, 1);
    assert_eq!(e.aranha.mastigando, 49, "50, menos a primeira mastigada");
    let (cx, cy) = ((e.aranha.x >> 4) as i32, (e.aranha.y >> 4) as i32);
    assert_eq!(
        (e.aranha.x & 15, e.aranha.y & 15),
        (8, 8),
        "centralizada na célula"
    );
    let d = e.aranha.dir as usize;
    let boca = (cx + t().dx8[d] as i32, cy + t().dy8[d] as i32);
    assert!(
        (0x10..=0x13).contains(&mapa(&e, boca.0, boca.1)),
        "restos pretos na boca"
    );
    while e.aranha.estado == 3 {
        inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
    }
    assert_eq!(e.aranha.mastigando, 0);
    assert_eq!(e.aranha.arrotos, 9);
    assert_eq!(e.partida.cadaver_ponteiro, 1);
    assert_eq!(
        (
            e.partida.cadaveres_x[1] as i32,
            e.partida.cadaveres_y[1] as i32
        ),
        boca
    );
}

#[test]
fn aranha_faro_acha_formiga_atras() {
    let mut e = quintal(0x0F0F);
    por_aranha(&mut e, 40, 20, 0, 2);
    e.amarela.modo_aranha = 7;
    e.adicionar_formiga(engine::formats::save::Lista::A, formiga(30, 20, 0x92)); // atrás, dist² 100
    e.adicionar_formiga(engine::formats::save::Lista::A, formiga(100, 20, 0x12)); // longe
    inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
    assert_eq!(
        (e.aranha.estado, e.aranha.alvo, e.aranha.alvo_tipo),
        (2, 0, 0x92)
    );
}

#[test]
fn aranha_desiste_se_a_presa_muda() {
    let mut e = quintal(0x0F0F);
    por_aranha(&mut e, 40, 20, 2, 2);
    e.adicionar_formiga(engine::formats::save::Lista::A, formiga(45, 20, 0x32)); // virou soldado (grupo 6): outro T
    e.aranha.alvo = 0;
    e.aranha.alvo_tipo = 0x12;
    inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
    assert_eq!((e.aranha.estado, e.aranha.alvo), (0, -2));
}

#[test]
fn i10_cerco_mata_a_aranha_e_arma_a_vinganca() {
    for (vinganca, modo) in [(0, 0), (2, 7), (4, 8)] {
        let mut e = quintal(0x4444);
        por_aranha(&mut e, 40, 20, 1, 2);
        e.aranha.ciclo = 3; // o próximo tick é múltiplo de 4
        e.aranha.vinganca = vinganca;
        for dx in -1..=1 {
            for dy in -1..=1 {
                e.superficie.vida.definir(40 + dx, 20 + dy, 0x12);
            }
        }
        let s = e.s;
        inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
        let a = &e.aranha;
        assert_eq!(
            (a.estado, a.agonia, a.ciclo_pernas, a.vinganca),
            (5, 500, 0, vinganca + 1)
        );
        assert_eq!(e.amarela.modo_aranha, modo);
        assert_eq!(e.retirar_eventos(), vec![Evento::Mensagem(10046)]);
        assert_eq!(e.s, s, "o cerco não sorteia");
    }
}

#[test]
fn cerco_com_cinco_formigas_faz_fugir() {
    let mut e = quintal(0x4444);
    por_aranha(&mut e, 40, 20, 0, 2);
    e.aranha.ciclo = 3;
    for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (0, 1)] {
        e.superficie.vida.definir(40 + dx, 20 + dy, 0x12);
    }
    inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
    assert_eq!(e.aranha.estado, 4);
    // Sem múltiplo de 4, nada de cerco.
    let mut e2 = quintal(0x4444);
    por_aranha(&mut e2, 40, 20, 0, 2);
    e2.aranha.ciclo = 4;
    for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (0, 1)] {
        e2.superficie.vida.definir(40 + dx, 20 + dy, 0x12);
    }
    inimigos::aranha(&mut e2, t(), TabelasAmarela::padrao(), None);
    assert_ne!(e2.aranha.estado, 4);
}

#[test]
fn aranha_morta_vira_tres_comidas() {
    let mut e = quintal(0x4444);
    por_aranha(&mut e, 40, 20, 5, 2);
    e.aranha.agonia = 3;
    e.ambiente.comida_superficie = 10;
    for _ in 0..3 {
        inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
    }
    assert_eq!(
        (e.aranha.ligada, e.aranha.estado, e.aranha.agonia),
        (0, 0, 0)
    );
    assert_eq!(mapa(&e, 40, 20), 0x4A);
    assert_eq!(e.ambiente.comida_superficie, 13);
}

#[test]
fn matar_aranha_poe_em_agonia() {
    let mut e = quintal(1);
    por_aranha(&mut e, 40, 20, 1, 2);
    inimigos::matar_aranha(&mut e, t());
    assert_eq!(
        (e.aranha.estado, e.aranha.agonia, e.aranha.ciclo_pernas),
        (5, 500, 0)
    );
}

#[test]
fn i11_laser_acerta_formiga_no_cone() {
    let mut e = quintal(0x6666);
    por_aranha(&mut e, 40, 20, 0, 2);
    e.amarela.modo_aranha = 8;
    // Formiga a 3 células, na direção da aranha (dir 2 = direita; laser varre 0 ± 32 = leste).
    e.adicionar_formiga(engine::formats::save::Lista::A, formiga(43, 20, 0x12));
    let mut atingida = false;
    for _ in 0..50 {
        inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
        if e.superficie.lista.vagas[0].tipo == 0 {
            atingida = true;
            break;
        }
        // Mantém a aranha parada no lugar para o teste.
        por_aranha(&mut e, 40, 20, 0, 2);
    }
    assert!(atingida, "o laser deveria matar a formiga em algum tick");
    assert!((0x10..=0x13).contains(&mapa(&e, 43, 20)) || e.superficie.vida.em(43, 20) == Some(0));
}

#[test]
fn aranha_do_jogador_anda_para_o_destino() {
    let mut e = quintal(0x1);
    por_aranha(&mut e, 40, 20, 0, 0);
    e.amarela.modo = 1;
    e.amarela.x = 40;
    e.amarela.y = 20;
    e.aranha.destino_x = 50;
    e.aranha.destino_y = 20;
    e.amarela.comando = 1;
    let s = e.s;
    inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
    // Gira de 0 para 1 (no máximo 45°) e corre: 5 × (3, −3).
    assert_eq!(e.aranha.dir, 1);
    assert_eq!(
        (e.aranha.x, e.aranha.y),
        (40 * 16 + 8 + 15, 20 * 16 + 8 - 15)
    );
    assert_eq!(
        (e.amarela.x, e.amarela.y),
        (e.aranha.x >> 4, e.aranha.y >> 4)
    );
    assert_eq!(e.s, s, "o jogador não sorteia");
    // No destino: só o quadro das pernas.
    e.amarela.x = 50;
    e.amarela.y = 20;
    inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
    assert_eq!(e.aranha.ciclo_pernas, 2);
}

// ---------------------------------------------------------------------------------------------
// Tabelas e seno
// ---------------------------------------------------------------------------------------------

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn json_do_repositorio_igual_ao_embutido() {
    let texto = std::fs::read_to_string(raiz().join("data/inimigos.json")).unwrap();
    let t2 = TabelasInimigos::de_json(&texto).unwrap();
    assert_eq!(t2.turn_tab, t().turn_tab);
    // As tabelas comuns vêm de data/comum.json (API 0.20) e não estão mais no inimigos.json.
    let parte: serde_json::Value = serde_json::from_str(&texto).unwrap();
    assert!(parte.get("sow_tab").is_none() && parte.get("dx8").is_none());
    let g: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(raiz().join("data/comum.json")).unwrap())
            .unwrap();
    assert_eq!(g["sow_tab"].as_array().unwrap().len(), 8);
    for i in 0..8 {
        assert_eq!(g["sow_tab"][i].as_u64().unwrap() as u8, t().sow_tab[i]);
        assert_eq!(
            g["borda_formiga_leao"][i].as_u64().unwrap() as u8,
            t().borda_formiga_leao[i]
        );
    }
}

fn tabelas_geracao() -> Option<TabelasGeracao> {
    let jogo = std::env::var_os("SIMANT_DIR")
        .map(PathBuf::from)
        .unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!(
            "AVISO: arquivos do jogo não encontrados em {}; teste ignorado",
            jogo.display()
        );
        return None;
    }
    let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    Some(TabelasGeracao::carregar(&r).unwrap())
}

#[test]
fn seno_calculado_igual_ao_do_banco() {
    let Some(g) = tabelas_geracao() else { return };
    let calculado: Vec<i16> = (0..64).map(seno_calculado).collect();
    assert_eq!(calculado, g.seno);
}

// ---------------------------------------------------------------------------------------------
// Contra o original (bancada QEMU): casos I-01 a I-03
// ---------------------------------------------------------------------------------------------

fn save_ref(nome: &str) -> Option<JogoSalvo> {
    let p = raiz().join("tests/referencias/saves").join(nome);
    if !p.is_file() {
        eprintln!("AVISO: {} não encontrado; teste ignorado", p.display());
        return None;
    }
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    Some(JogoSalvo::abrir(&layout, &p).unwrap())
}

/// Roda o port (só os inimigos; as outras partes do tick ainda são vazias) do estado `inicio` até o
/// `_GameTime` de `fim`.
fn rodar_ate(inicio: &str, fim: &str) -> Option<(Estado, Estado)> {
    let (a, b) = (save_ref(inicio)?, save_ref(fim)?);
    let mut e = Estado::de_save(&a).unwrap();
    let alvo = Estado::de_save(&b).unwrap();
    let mut r = RegrasOriginais { geracao: tabelas_geracao(), ..RegrasOriginais::default() };
    assert!(alvo.partida.tempo > e.partida.tempo);
    while e.partida.tempo < alvo.partida.tempo {
        tick(&mut e, &mut r);
    }
    Some((e, alvo))
}

#[test]
fn caso_i01_lagarta_igual_ao_original() {
    let Some((port, orig)) = rodar_ate("I01_INICIO.ANT", "I01_FIM.ANT") else {
        return;
    };
    let (p, o) = (&port.lagarta_e_bichos, &orig.lagarta_e_bichos);
    assert_eq!(
        (
            p.lagarta_estado,
            p.lagarta_x,
            p.lagarta_y,
            p.lagarta_dir,
            p.lagarta_fase,
            p.lagarta_chao
        ),
        (
            o.lagarta_estado,
            o.lagarta_x,
            o.lagarta_y,
            o.lagarta_dir,
            o.lagarta_fase,
            o.lagarta_chao
        )
    );
    let y = o.lagarta_y as i32;
    for x in 0..128 {
        assert_eq!(mapa(&port, x, y), mapa(&orig, x, y), "MapA({x}, {y})");
    }
}

#[test]
fn caso_i02_formiga_leao_igual_ao_original() {
    let Some((port, orig)) = rodar_ate("I02_INICIO.ANT", "I02_FIM.ANT") else {
        return;
    };
    let (p, o) = (&port.formigas_leao, &orig.formigas_leao);
    assert_eq!(
        (p.quantidade, p.x, p.y, p.m[0], p.t[0]),
        (o.quantidade, o.x, o.y, o.m[0], o.t[0])
    );
    assert_eq!(
        port.partida.comidas_por_formiga_leao,
        orig.partida.comidas_por_formiga_leao
    );
    // _BAntsEaten também conta presas da aranha, que dependem do movimento das formigas: só o mínimo.
    assert!(port.preta.comidas_por_predadores >= 1 && orig.preta.comidas_por_predadores >= 1);
    assert!(port
        .superficie
        .lista
        .ativas()
        .all(|(_, f)| (f.x, f.y) != (71, 19)));
    let (x, y) = (o.x[0] as i32, o.y[0] as i32);
    assert_eq!(mapa(&port, x, y), mapa(&orig, x, y));
}

#[test]
fn caso_i03_aranha_morta_igual_ao_original() {
    let Some((port, orig)) = rodar_ate("I03_INICIO.ANT", "I03_FIM.ANT") else {
        return;
    };
    // Depois da agonia a aranha pode reaparecer (S1(300)): só a agonia e a célula da comida.
    assert_eq!(port.aranha.agonia, orig.aranha.agonia);
    assert_eq!((mapa(&port, 100, 10), mapa(&orig, 100, 10)), (0x4A, 0x4A));
}

// ---------------------------------------------------------------------------------------------
// Vetores de tick do original (tests/referencias/ticks, docs/spec/regras/testes-tick.md)
// ---------------------------------------------------------------------------------------------

/// Para cada vetor: parte do "estado antes", roda só as partes dos inimigos (com o `_Cycle` já
/// incrementado, como o tick faz antes delas) e compara com o "estado depois" o que não depende da
/// posição exata no gerador S: lagarta (sem sorteio fora da criação), formigas-leão e a aranha
/// andando (nesses ticks ela não vira; o teste usa a semente S da entrada do tick, que não é a do
/// momento da aranha, porque quintal e água sorteiam antes).
#[test]
fn vetores_de_tick_do_original() {
    let dir = raiz().join("tests/referencias/ticks");
    let Ok(lista) = std::fs::read_dir(&dir) else {
        eprintln!("AVISO: {} não encontrado; teste ignorado", dir.display());
        return;
    };
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let mut n = 0;
    let mut nomes: Vec<PathBuf> = lista
        .filter_map(|d| d.ok())
        .map(|d| d.path())
        // Só os vetores da raiz (ticks 79–83); as subpastas de cenário têm o seu próprio índice.
        .filter(|p| p.join("estado_antes.bin").is_file())
        .collect();
    nomes.sort();
    for p in nomes {
        let antes = JogoSalvo::abrir(&layout, &p.join("estado_antes.bin")).unwrap();
        let depois = JogoSalvo::abrir(&layout, &p.join("estado_depois.bin")).unwrap();
        let extras: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(p.join("extras.json")).unwrap()).unwrap();
        let mut e = Estado::de_save(&antes).unwrap();
        let o = Estado::de_save(&depois).unwrap();
        e.s = GeradorS::new(extras["antes"]["SementeS"].as_u64().unwrap() as u16);
        e.partida.ciclo += 1;
        inimigos::formigas_leao(&mut e, t(), TabelasAmarela::padrao());
        inimigos::aranha(&mut e, t(), TabelasAmarela::padrao(), None);
        if e.partida.ciclo & 1 != 0 {
            inimigos::lagarta(&mut e, t());
        }
        let nome = p.file_name().unwrap().to_string_lossy().to_string();
        let (a, b) = (&e.aranha, &o.aranha);
        assert_eq!(
            (
                a.x,
                a.y,
                a.dir,
                a.ligada,
                a.estado,
                a.ciclo,
                a.ciclo_pernas,
                a.alvo
            ),
            (
                b.x,
                b.y,
                b.dir,
                b.ligada,
                b.estado,
                b.ciclo,
                b.ciclo_pernas,
                b.alvo
            ),
            "aranha em {nome}"
        );
        let (fa, fb) = (&e.formigas_leao, &o.formigas_leao);
        assert_eq!(
            (fa.quantidade, fa.m, fa.s, fa.t),
            (fb.quantidade, fb.m, fb.s, fb.t),
            "formigas-leão em {nome}"
        );
        let (la, lb) = (&e.lagarta_e_bichos, &o.lagarta_e_bichos);
        assert_eq!(
            (
                la.lagarta_estado,
                la.lagarta_x,
                la.lagarta_y,
                la.lagarta_dir,
                la.lagarta_fase,
                la.lagarta_chao
            ),
            (
                lb.lagarta_estado,
                lb.lagarta_x,
                lb.lagarta_y,
                lb.lagarta_dir,
                lb.lagarta_fase,
                lb.lagarta_chao
            ),
            "lagarta em {nome}"
        );
        // Corpo da lagarta (6 células para trás da cabeça, mais a frente) no mapa.
        let (hx, hy) = (lb.lagarta_x as i32, lb.lagarta_y as i32);
        let (px, py) = [(0, -1), (1, 0), (0, 1), (-1, 0)][(lb.lagarta_dir & 3) as usize];
        for k in -1..=6 {
            let (x, y) = (hx - k * px, hy - k * py);
            if let (Some(va), Some(vb)) = (e.superficie.mapa.em(x, y), o.superficie.mapa.em(x, y)) {
                if !(0x70..=0x73).contains(&vb) {
                    assert_eq!(va, vb, "MapA({x}, {y}) em {nome}");
                }
            }
        }
        n += 1;
    }
    assert!(n > 0);
}
