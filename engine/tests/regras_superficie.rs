//! Regras da superfície (`rules/superficie.rs`) e etapas de feromônio do tick.
//! Especificação: docs/spec/regras/superficie.md (casos T1–T12) e feromonios.md.
//!
//! Os testes montam o estado à mão e conferem resultado e sorteios. Os sorteios esperados são
//! obtidos repetindo, num `GeradorS` copiado, a mesma ordem que a spec descreve.
//! `vetores_de_tick_do_original` compara a superfície com os ticks reais do original em
//! `tests/referencias/ticks/` (docs/spec/regras/testes-tick.md).

use std::path::PathBuf;

use engine::formats::save::{Formiga, JogoSalvo, LayoutSave, Lista};
use engine::rules::estado::Estado;
use engine::rules::geracao::TabelasGeracao;
use engine::rules::rng::{GeradorR, GeradorS};
use engine::rules::superficie::{
    adicionar_comida, criar_lider_vermelha, desrecrutar, escolher_estrategia, recrutar, simular_superficie,
    tabelas_padrao,
};
use engine::rules::tick::{tick, RegrasOriginais};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Quintal vazio: chão 0, barreira 0x50, saúde 100 (ninguém morre de velhice: S32 ≤ 31).
fn quintal() -> Estado {
    let mut e = Estado::vazio();
    e.ambiente.barreira = 0x50;
    e.preta.saude = 100;
    e.vermelha.saude = 100;
    e.controles.modo_automatico = 1;
    e.preta.modo_estrategico = 1;
    e.vermelha.modo_estrategico = 1;
    e.amarela.plano = 2;
    e.s.definir_semente(0x1234);
    e
}

fn por(e: &mut Estado, x: u8, y: u8, modo: u8, tipo: u8, s: u8) -> usize {
    e.adicionar_formiga(Lista::A, Formiga { x, y, modo, tipo, s });
    e.superficie.lista.usadas() - 1
}

fn rodar(e: &mut Estado) {
    let (t, f) = tabelas_padrao();
    simular_superficie(e, t, f);
}

fn vaga(e: &Estado, i: usize) -> Formiga {
    e.superficie.lista.vagas[i]
}

fn vida(e: &Estado, x: i32, y: i32) -> u8 {
    e.superficie.vida.em(x, y).unwrap()
}

/// Gradiente de cheiro para leste em volta da célula (20, 10) (tiles 40–41, 20–21): a maior vizinha
/// é a leste, então uma formiga virada para 2 vai para 2 seja qual for o `S2()` de `_GetNestDir`.
fn gradiente_leste(g: &mut engine::rules::estado::Grade) {
    g.definir(20, 10, 50);
    g.definir(21, 10, 100);
}

/// Primeira semente (a partir de 1) para a qual `pred` vale.
fn semente_que(pred: impl Fn(&mut GeradorS) -> bool) -> u16 {
    (1..=u16::MAX).find(|&s| pred(&mut GeradorS::new(s))).expect("nenhuma semente serve")
}

// --- seção 1: o laço ---

#[test]
fn laco_sorteia_s256_para_toda_vaga() {
    let mut e = quintal();
    for k in 0..3 {
        por(&mut e, 10 + k, 10, 17, 0, 0); // vagas vazias (T = 0)
    }
    let mut s = e.s;
    rodar(&mut e);
    for _ in 0..3 {
        s.mascara(256);
    }
    assert_eq!(e.s, s, "um S256 por vaga, mesmo vazia (P8)");
}

#[test]
fn modos_17_e_18_nao_fazem_nada_e_contam() {
    let mut e = quintal();
    let i = por(&mut e, 30, 30, 17, 0x12, 9);
    let j = por(&mut e, 31, 30, 18, 0x92, 9);
    let antes = e.superficie.lista.clone();
    rodar(&mut e);
    assert_eq!(e.superficie.lista, antes);
    assert_eq!(e.modos_do_tick[0][17], 1);
    assert_eq!(e.modos_do_tick[1][18], 1);
    let _ = (i, j);
}

#[test]
fn t1_morte_natural_e_cadaver() {
    // S256 = 0 e saúde < S32: a formiga vira cadáver; conta em _BAntsExpired mesmo vermelha (P6).
    for (tipo, faixa) in [(0x12u8, 0x10..=0x13u8), (0x92, 0x14..=0x17)] {
        let semente = semente_que(|g| g.mascara(256) == 0 && g.mascara(32) > 5);
        let mut e = quintal();
        e.preta.saude = 5;
        e.vermelha.saude = 5;
        e.s.definir_semente(semente);
        let i = por(&mut e, 40, 40, 17, tipo, 0);
        rodar(&mut e);
        assert_eq!(vaga(&e, i).tipo, 0);
        assert!(faixa.contains(&e.superficie.mapa.em(40, 40).unwrap()));
        assert_eq!(vida(&e, 40, 40), 0);
        assert_eq!(e.preta.mortes_naturais, 1);
        assert_eq!(e.vermelha.mortes_naturais, 0);
        assert_eq!((e.partida.cadaver_ponteiro, e.partida.cadaveres_x[1], e.partida.cadaveres_y[1]), (1, 40, 40));
    }
}

// --- seções 2–3: passo e modos ---

#[test]
fn t2_pegar_comida() {
    let mut e = quintal();
    gradiente_leste(&mut e.feromonios.ninho_b);
    e.superficie.mapa.definir(41, 20, 0x4A);
    e.ambiente.comida_superficie = 3;
    let i = por(&mut e, 40, 20, 1, 0x12, 0);
    rodar(&mut e);
    let f = vaga(&e, i);
    assert_eq!((f.x, f.y, f.tipo, f.modo, f.s), (40, 20, 0x1A, 3, 200), "não anda; grupo 2 → 3");
    assert_eq!(vida(&e, 40, 20), 0x1A);
    assert_eq!(e.superficie.mapa.em(41, 20), Some(0x49));
    assert_eq!(e.ambiente.comida_superficie, 2);
}

#[test]
fn t3_t4_trilha_de_volta_atravessa_outras_formigas() {
    let mut e = quintal();
    gradiente_leste(&mut e.feromonios.ninho_b);
    let ocupante = por(&mut e, 41, 20, 17, 0x92, 0); // vermelha no caminho
    let i = por(&mut e, 40, 20, 3, 0x1A, 200);
    rodar(&mut e);
    let f = vaga(&e, i);
    assert_eq!((f.x, f.y, f.tipo, f.s), (41, 20, 0x1A, 199));
    assert_eq!(vida(&e, 41, 20), 0x1A, "sobrescreve a vida da outra (P2)");
    assert_eq!(vida(&e, 40, 20), 0);
    assert_eq!(vaga(&e, ocupante).tipo, 0x92, "sem luta");
    assert_eq!(e.feromonios.trilha_b.em(20, 10), Some(199));
}

#[test]
fn passeio_deposita_ninho_gasta_trilha_e_pode_virar_forrageira() {
    // Modo 0: direção _GetRandDir (Giro[dir][S8]); depois do passo, S8 == 0 → Foraging.
    let semente = semente_que(|g| {
        if g.mascara(256) == 0 {
            g.mascara(32);
        }
        // Giro[0][0] = 0 (norte). (Com o LFSR, dois S8 seguidos não podem dar 2 e 0.)
        g.mascara(8) == 0 && g.mascara(8) == 0
    });
    let mut e = quintal();
    e.s.definir_semente(semente);
    e.feromonios.trilha_b.definir(20, 9, 9);
    let i = por(&mut e, 40, 20, 0, 0x10, 50);
    rodar(&mut e);
    let f = vaga(&e, i);
    assert_eq!((f.x, f.y, f.tipo, f.modo, f.s), (40, 19, 0x10, 2, 49));
    assert_eq!(e.feromonios.ninho_b.em(20, 9), Some(49));
    assert_eq!(e.feromonios.trilha_b.em(20, 9), Some(8), "_DecTSmell no destino");
}

#[test]
fn barreira_gira_sem_andar() {
    let semente = semente_que(|g| {
        g.mascara(256);
        g.mascara(8) == 2
    });
    let mut e = quintal();
    e.s.definir_semente(semente);
    e.superficie.mapa.definir(41, 20, 0x54); // pedra
    let i = por(&mut e, 40, 20, 0, 0x12, 0);
    let mut s = e.s;
    rodar(&mut e);
    s.mascara(256);
    s.mascara(8);
    let k = s.mascara(8) as usize;
    let novo = 0x10 | tabelas_padrao().1.turn_tab[2][k];
    assert_eq!((vaga(&e, i).x, vaga(&e, i).tipo), (40, novo));
    assert_eq!(vida(&e, 40, 20), novo);
    assert_eq!(e.s, s);
}

#[test]
fn t5_luta_comeca_e_atacante_some() {
    let mut e = quintal();
    gradiente_leste(&mut e.feromonios.ninho_r); // o atacante preto segue o ninho vermelho
    e.r = GeradorR::new(7);
    // A atacante fica na vaga mais baixa: a defensora (vaga 1) é processada antes e não vira
    // nuvem de novo neste tick.
    let atacante = por(&mut e, 40, 20, 7, 0x12, 0);
    let defensora = por(&mut e, 41, 20, 17, 0x92, 0);
    let mut r = e.r;
    let v = if r.rrand(10) < 5 { 0x92 } else { 0x12 };
    rodar(&mut e);
    assert_eq!(vaga(&e, atacante).tipo, 0);
    assert_eq!(vida(&e, 40, 20), 0);
    let d = vaga(&e, defensora);
    assert_eq!((d.tipo, d.modo, d.s), (0x70 + (v & 0x80), 10, v));
    assert_eq!(vida(&e, 41, 20), d.tipo);
    assert_eq!(e.feromonios.alarme.em(20, 10), Some(40));
    assert_eq!(e.r, r, "um RRand(10) (P10)");
    if v == 0x12 {
        assert_eq!((e.vermelha.perdas, e.vermelha.mortas_em_luta), (1, 1));
    } else {
        assert_eq!((e.preta.perdas, e.preta.mortas_em_luta), (1, 1));
    }
}

#[test]
fn luta_invencivel_sem_sorteio_e_ataque_larga_a_carga() {
    let mut e = quintal();
    e.partida.invencivel = 1;
    gradiente_leste(&mut e.feromonios.ninho_r);
    por(&mut e, 40, 20, 7, 0x1A, 0); // carregando comida (grupo 3)
    let defensora = por(&mut e, 41, 20, 17, 0x92, 0);
    let r = e.r;
    rodar(&mut e);
    assert_eq!(e.r, r);
    let d = vaga(&e, defensora);
    assert_eq!((d.tipo, d.s), (0x70, 0x1A), "S guarda o T gravado na vaga, com a carga");
    assert_eq!(e.vermelha.mortas_em_luta, 1);
}

#[test]
fn nuvem_de_luta_termina_com_cadaver_da_cor_do_vencedor() {
    let t = &tabelas_padrao().0;
    let semente = semente_que(|g| {
        if g.mascara(256) == 0 {
            g.mascara(32);
        }
        g.rand1(7);
        g.mascara(16) == 0
    });
    let mut e = quintal();
    e.s.definir_semente(semente);
    let i = por(&mut e, 50, 30, 10, 0xF0, 0x12); // nuvem vermelha... vencida pela preta 0x12
    let mut g = e.s;
    if g.mascara(256) == 0 {
        g.mascara(32);
    }
    g.rand1(7);
    g.mascara(16);
    let modo = t.modos_operaria[1][g.mascara(8) as usize];
    rodar(&mut e);
    let f = vaga(&e, i);
    assert_eq!((f.tipo, f.modo, f.s), (0x12, modo, 0));
    assert!((0x10..=0x13).contains(&e.superficie.mapa.em(50, 30).unwrap()), "cadáver preto (P3)");
    assert_eq!(vida(&e, 50, 30), 0, "vida zerada sob o vencedor (P3)");
}

#[test]
fn t7_entrada_no_ninho_pela_coluna_do_buraco() {
    let mut e = quintal();
    e.superficie.mapa.definir(3, 32, 0x50);
    e.preta.buracos[32] = 3;
    e.ninho_b.mapa.definir(32, 1, 0x2E);
    let i = por(&mut e, 3, 32, 0, 0x13, 77);
    rodar(&mut e);
    assert_eq!(vaga(&e, i).tipo, 0);
    assert_eq!(vida(&e, 3, 32), 0);
    let b = e.ninho_b.lista.vagas[0];
    assert_eq!((e.ninho_b.lista.indice, b.x, b.y, b.tipo, b.modo, b.s), (1, 32, 1, 0x14, 0, 77));
    assert!(e.ninho_b.mapa.em(32, 1).unwrap() < 8, "_DigTileB cavou a entrada");
    assert_eq!(e.preta.tiles_escavados, 1);

    // Uma preta num buraco do lado direito entra no ninho vermelho.
    let mut e = quintal();
    e.superficie.mapa.definir(100, 10, 0x50);
    por(&mut e, 100, 10, 2, 0x12, 0);
    rodar(&mut e);
    assert_eq!(e.ninho_r.lista.vagas[0].tipo, 0x14);
}

#[test]
fn modo_5_larga_a_terra() {
    let semente = semente_que(|g| {
        g.mascara(256);
        g.mascara(8);
        g.mascara(8) == 0
    });
    let mut e = quintal();
    e.s.definir_semente(semente);
    let i = por(&mut e, 40, 20, 5, 0x2C, 5); // grupo 5
    rodar(&mut e);
    let f = vaga(&e, i);
    assert_eq!((f.tipo, f.modo, f.s), (0x14, 5, 0), "grupo 5 → 2; tabela devolve 5 (corrige no próximo tick)");
    assert_eq!(vida(&e, 40, 20), 0x14);
}

#[test]
fn modo_8_ovo_e_modo_9_cauda_sem_cabeca() {
    let mut e = quintal();
    let cauda = por(&mut e, 20, 20, 9, 0x6A, 0); // cauda, direção 2: cabeça esperada em (21, 20)
    let cabeca = por(&mut e, 30, 20, 9, 0x62, 0); // cabeça sozinha: nunca removida
    rodar(&mut e);
    assert_eq!(vaga(&e, cauda).tipo, 0);
    assert_eq!(vaga(&e, cabeca).tipo, 0x62);

    let semente = semente_que(|g| {
        g.mascara(256);
        g.rand1(200) == 0
    });
    let mut e = quintal();
    e.s.definir_semente(semente);
    let ovo = por(&mut e, 20, 20, 8, 1, 0);
    rodar(&mut e);
    assert_eq!(vaga(&e, ovo).tipo, 0);
}

#[test]
fn t8_decolar_e_voar() {
    let mut e = quintal();
    e.preta.revoada = 1;
    e.feromonios.ninho_b.definir(20, 10, 200); // ≥ 100: passeio mudo
    let i = por(&mut e, 40, 20, 15, 0x40, 0);
    rodar(&mut e);
    assert_eq!(vaga(&e, i).modo, 16, "F = 1 → voo sem sorteio");

    let semente = semente_que(|g| {
        g.mascara(256);
        g.mascara(32) == 0 && g.mascara(16) == 0
    });
    let mut e = quintal();
    e.partida.tipo_jogo = 2;
    e.s.definir_semente(semente);
    let i = por(&mut e, 40, 20, 16, 0x40, 0);
    rodar(&mut e);
    assert_eq!(vaga(&e, i).tipo, 0);
    assert_eq!((e.preta.enxame_contagem, e.preta.rainhas_guardadas), (1, 1));
}

#[test]
fn modo_2_alarme_e_descanso() {
    let semente = semente_que(|g| {
        g.mascara(256);
        g.mascara(32) != 0
    });
    let mut e = quintal();
    e.s.definir_semente(semente);
    e.feromonios.alarme.definir(20, 10, 1);
    let i = por(&mut e, 40, 20, 2, 0x12, 0);
    rodar(&mut e);
    assert_eq!(vaga(&e, i).modo, 11);
}

// --- seção 9: estratégia, líder, recrutamento ---

#[test]
fn t9_estrategia_vermelha_malvada_e_revoada() {
    let t = &tabelas_padrao().0;
    let mut e = quintal();
    e.vermelha.saude = 5;
    e.vermelha.populacao = 100;
    e.preta.populacao = 10;
    e.preta.rainhas = 1;
    e.vermelha.castas[3] = 21;
    let mut s = e.s;
    escolher_estrategia(&mut e, t);
    assert_eq!(e.vermelha.modo_estrategico, 0);
    assert_eq!(e.lider_vermelha.malvadeza, s.rand1(100) as i16 + 30);
    assert_eq!(e.vermelha.revoada, 200);
    assert_eq!(e.retirar_eventos(), vec![engine::rules::estado::Evento::Musica(0x2B0A)]);
    let m = e.lider_vermelha.malvadeza;
    escolher_estrategia(&mut e, t);
    assert_eq!((e.vermelha.modo_estrategico, e.lider_vermelha.malvadeza), (0, m - 1));
}

#[test]
fn estrategia_preta_escada_e_posicao_difusa() {
    let t = &tabelas_padrao().0;
    let mut e = quintal();
    e.vermelha.saude = 100;
    e.vermelha.populacao = 10;
    e.vermelha.tiles_escavados = 100;
    for (saude, dug, esperado) in [(20, 100, 5), (40, 100, 4), (90, 5, 3), (90, 15, 2), (90, 100, 1)] {
        e.preta.saude = saude;
        e.preta.populacao = 10;
        e.preta.tiles_escavados = dug;
        escolher_estrategia(&mut e, t);
        assert_eq!(e.preta.modo_estrategico, esperado, "saúde {saude}, cavados {dug}");
    }
    // Amarela na superfície: dois S1(5); X é sobrescrito por _MeLocX.
    e.amarela.plano = 1;
    e.amarela.x = 10;
    e.amarela.y = 0;
    let mut s = e.s;
    escolher_estrategia(&mut e, t);
    s.rand1(5);
    let dy = s.rand1(5);
    assert_eq!((e.amarela.difusa_x, e.amarela.difusa_y), (10, (dy - 2).max(0) as i16));
}

#[test]
fn castas_e_tarefas_do_proximo_ovo() {
    let t = &tabelas_padrao().0;
    let mut e = quintal();
    e.preta.castas = [0, 10, 0, 0, 0, 1];
    e.controles.castas_ideais = [60, 40, 0, 0];
    e.preta.modos = [0, 0, 10, 0, 0, 0];
    e.controles.tarefas = [39321, 13107, 13107];
    escolher_estrategia(&mut e, t);
    assert_eq!(e.controles.proxima_casta, 6, "faltam soldados");
    assert_eq!(e.controles.proximo_modo, 2, "falta forragear");
}

#[test]
fn t10_lider_vermelha_criada_e_andando() {
    let t = &tabelas_padrao().0;
    let mut e = quintal();
    e.preta.populacao = 29;
    por(&mut e, 100, 30, 2, 0x92, 0);
    criar_lider_vermelha(&mut e, t);
    assert_eq!((e.lider_vermelha.ligada, vaga(&e, 0).modo), (0, 2));

    e.preta.populacao = 30;
    por(&mut e, 90, 30, 17, 0x92, 0);
    por(&mut e, 80, 30, 17, 0x12, 0);
    e.lider_vermelha.tarefa = 5;
    criar_lider_vermelha(&mut e, t);
    let f = vaga(&e, 1);
    assert_eq!((f.tipo, f.modo, f.s), (0xB0, 19, 0));
    assert_eq!((e.lider_vermelha.ligada, e.lider_vermelha.tarefa), (1, 0));
    assert_eq!(vida(&e, 90, 30), 0x92, "não atualiza a vida (P4)");

    // _DoRedInitiator: destino = _LastFoodDrop (64, 38) − 5 em x; recruta (8 >> 2) = 2.
    e.superficie.lista.vagas[1].x = 100;
    e.superficie.lista.vagas[1].y = 30;
    e.superficie.vida.definir(90, 30, 0);
    e.superficie.vida.definir(100, 30, 0xB0);
    e.superficie.lista.vagas[0] = Formiga { x: 110, y: 10, modo: 17, tipo: 0x92, s: 0 };
    e.ambiente.ultima_comida = (64, 38);
    e.vermelha.castas[1] = 8;
    rodar(&mut e);
    assert_eq!((e.lider_vermelha.destino_x, e.lider_vermelha.destino_y, e.lider_vermelha.tarefa), (59, 38, 1));
    let l = vaga(&e, 1);
    assert_eq!((l.x, l.y, l.tipo), (99, 31, 0xB5), "_GetRedBestDirs: sudoeste");
    assert_eq!(vaga(&e, 0).modo, 6, "recrutada");
    assert_eq!((e.lider_vermelha.x, e.lider_vermelha.y, e.lider_vermelha.plano), (100, 30, 1));
}

#[test]
fn t11_recrutamento() {
    let mut e = quintal();
    for k in 0..4 {
        por(&mut e, 10 + k, 10, 2, if k == 1 { 0x92 } else { 0x12 }, 9);
    }
    recrutar(&mut e, 2);
    let modos: Vec<u8> = (0..4).map(|i| vaga(&e, i).modo).collect();
    assert_eq!(modos, vec![2, 2, 6, 6], "de trás para frente, só pretas");
    e.modos_do_tick[0][6] = 2;
    desrecrutar(&mut e, 0);
    let modos: Vec<u8> = (0..4).map(|i| vaga(&e, i).modo).collect();
    assert_eq!(modos, vec![2, 2, 6, 0], "quota = metade");
}

// --- seção 8.3: comida ---

fn tabelas_geracao() -> TabelasGeracao {
    let seno: Vec<u8> = (0..64)
        .flat_map(|i| ((((i as f64) * std::f64::consts::PI / 128.0).sin() * 32767.0).round() as i16).to_be_bytes())
        .collect();
    TabelasGeracao::de_partes(&std::fs::read_to_string(raiz().join("data/geracao.json")).unwrap(), &seno).unwrap()
}

#[test]
fn adicionar_comida_sorteia_centro_e_espalha() {
    let g = tabelas_geracao();
    let mut e = quintal();
    let mut s = e.s;
    adicionar_comida(&mut e, &g, 150, true);
    let (cx, cy) = (s.mascara(128), s.mascara(64));
    assert_eq!(e.ambiente.ultima_comida, (cx as i16, cy as i16));
    let mut unidades = 0;
    for v in &e.superficie.mapa.dados {
        if (0x48..=0x4B).contains(v) {
            unidades += (v - 0x47) as i16;
        }
    }
    assert_eq!(e.ambiente.comida_superficie, unidades);
    assert!(unidades > 0 && unidades <= 150);
    // raio S8 + 5, depois 2 sorteios por unidade.
    let raio = s.mascara(8) as u16 + 5;
    for _ in 0..150 {
        s.mascara(256);
        s.rand1(raio);
    }
    assert_eq!(e.s, s);
}

// --- feromônios no tick ---

#[test]
fn etapas_de_feromonio_no_tick() {
    let mut e = quintal();
    let mut r = RegrasOriginais::default();
    e.superficie.mapa.definir(3, 32, 0x50);
    e.preta.buracos[32] = 3;
    e.vermelha.buracos[32] = 118;
    e.feromonios.trilha_b.definir(10, 5, 210);
    e.feromonios.trilha_r.definir(10, 5, 7);
    e.feromonios.alarme.definir(30, 20, 200);
    e.partida.ciclo = 31;
    tick(&mut e, &mut r); // 32: pretos
    assert_eq!(e.feromonios.ninho_b.em(1, 16), Some(254));
    assert_eq!(e.feromonios.trilha_b.em(10, 5), Some(105));
    assert_eq!(e.feromonios.trilha_r.em(10, 5), Some(7));
    e.partida.ciclo = 95;
    tick(&mut e, &mut r); // 96: vermelhos
    assert_eq!(e.feromonios.ninho_r.em(59, 16), Some(254));
    assert_eq!(e.feromonios.trilha_r.em(10, 5), Some(0));
    e.partida.ciclo = 127;
    tick(&mut e, &mut r); // 128: suavização do alarme
    assert_eq!(e.feromonios.alarme.em(30, 20), Some(100));
    assert_eq!(e.feromonios.alarme.em(31, 20), Some(25));
}

// --- vetores do original (testes-tick.md) ---

/// Para cada tick gravado do original: parte do estado de entrada, acha quantos sorteios S as
/// sub-rotinas anteriores (quintal, água, formiga-leão, aranha, estratégia) consumiram — ainda não
/// portadas por inteiro — e confere que `_DoAntSimA` a partir daí reproduz a lista A, a vida da
/// superfície nas células das formigas e os mapas de cheiro do estado de saída.
#[test]
fn vetores_de_tick_do_original() {
    let dir = raiz().join("tests/referencias/ticks");
    let Ok(entradas) = std::fs::read_dir(&dir) else {
        eprintln!("sem {}: teste não verificado", dir.display());
        return;
    };
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let (t, f) = tabelas_padrao();
    let mut dirs: Vec<PathBuf> =
        entradas.flatten().map(|d| d.path()).filter(|p| p.join("estado_antes.bin").exists()).collect();
    dirs.sort();
    assert!(!dirs.is_empty());
    for d in dirs {
        let ler = |n: &str| std::fs::read(d.join(n)).unwrap();
        let extras: serde_json::Value = serde_json::from_slice(&ler("extras.json")).unwrap();
        let semente = |p: &str| extras[p]["SementeS"].as_u64().unwrap() as u16;
        let antes = JogoSalvo::ler(&layout, ler("estado_antes.bin")).unwrap();
        let depois = Estado::de_save(&JogoSalvo::ler(&layout, ler("estado_depois.bin")).unwrap()).unwrap();
        let vida_depois = ler("vida_depois.bin");
        let mut base = Estado::de_save(&antes).unwrap();
        base.superficie.vida.dados.copy_from_slice(&ler("vida_antes.bin")[..8192]);
        base.partida.ciclo += 1;
        base.partida.tempo += 1;
        // A estratégia roda antes da superfície; os modos estratégicos de saída valem no laço.
        let mut est = base.clone();
        escolher_estrategia(&mut est, t);
        assert_eq!(
            (est.preta.modo_estrategico, est.vermelha.modo_estrategico, est.controles.proxima_casta, est.controles.proximo_modo),
            (depois.preta.modo_estrategico, depois.vermelha.modo_estrategico, depois.controles.proxima_casta, depois.controles.proximo_modo),
            "{}: estratégia",
            d.display()
        );
        base.preta.modo_estrategico = depois.preta.modo_estrategico;
        base.vermelha.modo_estrategico = depois.vermelha.modo_estrategico;
        base.controles.proximo_modo = depois.controles.proximo_modo;
        let n = base.superficie.lista.usadas();
        let mut s = GeradorS::new(semente("antes"));
        let mut achou = None;
        for k in 0..4000 {
            let mut e = base.clone();
            e.s = s;
            simular_superficie(&mut e, t, f);
            let lista_ok = e.superficie.lista.vagas[..n] == depois.superficie.lista.vagas[..n];
            let cheiro_ok = e.feromonios == depois.feromonios;
            let vida_ok = e.superficie.lista.vagas[..n]
                .iter()
                .filter(|a| a.tipo != 0)
                .all(|a| vida_depois[a.x as usize * 64 + a.y as usize] == a.tipo);
            if lista_ok && cheiro_ok && vida_ok {
                // O resto do tick (ninhos, amarela...) leva S de onde a superfície parou até o fim.
                let mut resto = e.s;
                let fim = semente("depois");
                let alcanca = (0..20000).any(|_| resto.passo() == fim) || e.s.semente() == fim;
                if alcanca {
                    achou = Some(k);
                    break;
                }
            }
            s.passo();
        }
        let k = achou.unwrap_or_else(|| panic!("{}: nenhum ponto de partida de S reproduz a superfície", d.display()));
        eprintln!("{}: superfície reproduzida com {k} sorteios S antes de _DoAntSimA", d.display());
    }
}

// --- casos da bancada (tests/referencias/casos/SUP-0*.json) ---

/// Carrega o estado logo depois do load no original (`SUPnn_CARREGADO.ANT`), roda o port até o
/// `GameTime` em que o original parou (`SUPnn_FIM.ANT`) e devolve (port, original). As sementes do
/// original depois do load não são conhecidas: os casos só comparam o que não depende delas.
fn caso_da_bancada(n: u8) -> (Estado, Estado) {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let abrir = |nome: String| {
        Estado::de_save(&JogoSalvo::abrir(&layout, &raiz().join("tests/referencias/saves").join(nome)).unwrap())
            .unwrap()
    };
    let mut e = abrir(format!("SUP0{n}_CARREGADO.ANT"));
    let fim = abrir(format!("SUP0{n}_FIM.ANT"));
    let mut r = RegrasOriginais::default();
    while e.partida.tempo < fim.partida.tempo {
        tick(&mut e, &mut r);
    }
    (e, fim)
}

#[test]
fn sup01_carregadora_igual_ao_original() {
    let (e, o) = caso_da_bancada(1);
    assert_eq!(vaga(&e, 2), vaga(&o, 2));
    for cx in 5..=20 {
        assert_eq!(e.feromonios.trilha_b.em(cx, 20), o.feromonios.trilha_b.em(cx, 20), "trilha cx {cx}");
        assert_eq!(e.feromonios.ninho_b.em(cx, 20), o.feromonios.ninho_b.em(cx, 20), "ninho cx {cx}");
    }
}

#[test]
fn sup02_entrada_no_ninho_igual_ao_original() {
    let (e, o) = caso_da_bancada(2);
    assert_eq!((vaga(&e, 2).tipo, vaga(&o, 2).tipo), (0, 0));
    let (pb, ob) = (e.ninho_b.lista.vagas[5], o.ninho_b.lista.vagas[5]);
    assert_eq!((pb.tipo & 0xF8, pb.s), (ob.tipo & 0xF8, ob.s));
    assert!(o.ninho_b.mapa.em(32, 1).unwrap() < 0x20 && e.ninho_b.mapa.em(32, 1).unwrap() < 0x20, "entrada escavada");
}

#[test]
fn sup03_lider_vermelha_igual_ao_original() {
    let (e, o) = caso_da_bancada(3);
    assert_eq!(vaga(&e, 32), vaga(&o, 32));
    let (l, lo) = (&e.lider_vermelha, &o.lider_vermelha);
    assert_eq!(
        (l.destino_x, l.destino_y, l.x, l.y, l.plano, l.tarefa, l.ligada),
        (lo.destino_x, lo.destino_y, lo.x, lo.y, lo.plano, lo.tarefa, lo.ligada)
    );
}

#[test]
fn sup04_feromonios_iguais_ao_original() {
    let (e, o) = caso_da_bancada(4);
    assert_eq!(vaga(&e, 2), vaga(&o, 2));
    let f = |e: &Estado| {
        (
            e.feromonios.trilha_r.em(40, 5),
            e.feromonios.trilha_b.em(45, 5),
            [(50, 25), (49, 25), (51, 25), (50, 24), (50, 26)].map(|(x, y)| e.feromonios.alarme.em(x, y)),
            e.feromonios.ninho_r.em(59, 16),
            e.feromonios.ninho_b.em(1, 16),
        )
    };
    assert_eq!(f(&e), f(&o));
}
