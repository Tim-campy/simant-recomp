//! Formiga amarela e comandos do jogador. Especificação: docs/spec/regras/formiga-amarela.md
//! (casos Y1–Y17 da seção 13 onde dá para montar sem interface) e comparação com o original
//! (vetores de tick em tests/referencias/ticks e casos Y-01..Y-04 da bancada).

use std::path::{Path, PathBuf};

use engine::formats::save::{Formiga, JogoSalvo, LayoutSave, Lista};
use engine::rules::amarela::{
    self, clicar_no_mapa, comando, lutar_com_amarela, matar_amarela, ordem_agir, ordem_ir, tecla_de_comando,
    trocar_de_corpo, trofalaxia, vk, CausaMorte, ControleJogador, Modificadores, RespostaClique, RespostaTecla,
    TabelasAmarela,
};
use engine::rules::estado::{Estado, Evento};
use engine::rules::tick::{tick, RegrasOriginais};

fn t() -> &'static TabelasAmarela {
    TabelasAmarela::padrao()
}

/// Superfície de chão livre (0), ninhos de túnel (0) e a amarela operária em (x, y) do plano.
fn estado(plano: i16, x: i16, y: i16) -> Estado {
    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 1;
    e.partida.ferramenta = -1;
    let a = &mut e.amarela;
    a.plano = plano;
    a.x = x;
    a.y = y;
    a.tipo = 0x10;
    a.dir = 2;
    a.saude = 100;
    a.aviso_saude = 30;
    a.comeu_tipo = -1;
    a.ovo_carregado = 0xFD;
    a.contorno_contagem = -2;
    a.ninho_iniciado = 1;
    e.preta.ultimo_buraco = (3, 32);
    e.preta.ultima_saida = (32, 0);
    e.partida.ciclo = 1;
    amarela::simular_amarela(&mut e, t());
    e
}

/// A parte da amarela de um tick (passos 1, 17 e 20 de `_DoAntSim`).
fn passo(e: &mut Estado) {
    e.partida.ciclo += 1;
    e.partida.tempo += 1;
    amarela::simular_amarela(e, t());
    amarela::mover_amarela(e, t());
}

fn andar_ate_parar(e: &mut Estado, limite: usize) -> usize {
    for n in 0..limite {
        if e.amarela.mover == 0 {
            return n;
        }
        passo(e);
    }
    limite
}

fn pos(e: &Estado) -> (i16, i16, i16) {
    (e.amarela.plano, e.amarela.x, e.amarela.y)
}

#[test]
fn y1_vida_por_tempo_e_por_passo() {
    let mut e = estado(1, 10, 10);
    e.partida.ciclo = 60;
    for _ in 0..8 {
        passo(&mut e);
    }
    assert_eq!(e.amarela.saude, 99, "parada: −1 a cada 64 ticks (ciclo 64)");
    ordem_ir(&mut e, 1, 20, 10);
    for _ in 0..10 {
        passo(&mut e);
    }
    assert_eq!(e.amarela.x, 20);
    assert_eq!(e.amarela.saude, 94, "andando: −1 a cada 2 passos");
    assert_eq!(e.superficie.vida.em(20, 10), Some(0xFF));
    assert_eq!(e.superficie.vida.em(19, 10), Some(0));
}

#[test]
fn y2_ordem_simples() {
    let mut e = estado(1, 10, 10);
    assert_eq!(ordem_ir(&mut e, 1, 15, 13), RespostaClique::Ordem);
    let a = &e.amarela;
    assert_eq!((a.mover, a.comando, a.contorno_contagem), (1, 0, -2));
    assert_eq!((a.alvo_plano, a.alvo_x, a.alvo_y), (1, 15, 13));
    let n = andar_ate_parar(&mut e, 50);
    assert_eq!(n, 5, "3 diagonais e 2 retos");
    assert_eq!(pos(&e), (1, 15, 13));
    assert_eq!((e.amarela.passos, e.amarela.alvo_x), (0, 15), "ordem encerrada zera os passos");
}

#[test]
fn y3_contorno_de_parede() {
    let mut e = estado(1, 25, 20);
    for y in 15..=25 {
        e.superficie.mapa.definir(30, y, 0x5C);
    }
    ordem_ir(&mut e, 1, 35, 20);
    let mut contorno_max = -2;
    for _ in 0..60 {
        if e.amarela.mover == 0 {
            break;
        }
        passo(&mut e);
        contorno_max = contorno_max.max(e.amarela.contorno_contagem);
        assert_ne!(e.superficie.mapa.em(e.amarela.x as i32, e.amarela.y as i32), Some(0x5C));
    }
    assert_eq!(pos(&e), (1, 35, 20));
    assert!((14..=16).contains(&contorno_max), "contorno começou com 16: {contorno_max}");
}

#[test]
fn y4_pegar_e_largar_comida() {
    let mut e = estado(1, 10, 10);
    e.superficie.mapa.definir(12, 10, 0x4A);
    e.ambiente.comida_superficie = 3;
    assert_eq!(ordem_agir(&mut e, 1, 12, 10), RespostaClique::Ordem);
    assert_eq!(e.amarela.comando, 1);
    andar_ate_parar(&mut e, 10);
    assert_eq!(pos(&e), (1, 11, 10), "pega do lado, sem entrar na célula");
    assert_eq!(e.amarela.tipo, 0x18);
    assert_eq!(e.amarela.rastro, 200);
    assert_eq!(e.superficie.mapa.em(12, 10), Some(0x49));
    assert_eq!(e.ambiente.comida_superficie, 2);
    assert_eq!(e.feromonios.trilha_b.em(6, 5), Some(200), "trilha preta 200 na célula da comida");
    // Andar deixa trilha decrescente.
    ordem_ir(&mut e, 1, 11, 20);
    let n = andar_ate_parar(&mut e, 20);
    assert_eq!((n, pos(&e)), (10, (1, 11, 20)));
    assert_eq!(e.amarela.rastro, 190);
    assert_eq!(e.feromonios.trilha_b.em(5, 10), Some(191), "deposita antes de diminuir");
    // Largar no chão: duplo clique numa célula vizinha.
    ordem_agir(&mut e, 1, 12, 20);
    andar_ate_parar(&mut e, 5);
    assert_eq!(e.amarela.tipo, 0x10);
    assert_eq!(e.superficie.mapa.em(12, 20), Some(0x48));
    assert_eq!(e.ambiente.comida_superficie, 3);
}

#[test]
fn y5_comer_com_fome() {
    let mut e = estado(1, 10, 10);
    e.superficie.mapa.definir(11, 10, 0x49);
    e.amarela.saude = 40;
    e.amarela.aviso_saude = 50;
    e.amarela.quer_comida = 1;
    e.preta.populacao = 3;
    e.preta.saude = 50;
    ordem_agir(&mut e, 1, 11, 10);
    passo(&mut e);
    assert_eq!(e.amarela.saude, 100);
    assert_eq!(e.preta.saude, 50 + 40 / 2);
    assert_eq!(e.amarela.tipo, 0x10, "come na hora, não carrega");
    assert_eq!(e.superficie.mapa.em(11, 10), Some(0x48));
    assert_eq!(e.amarela.mover, 0);
    assert_eq!(e.amarela.comeu_tipo, -1);
}

#[test]
fn y6_trofalaxia() {
    let mut e = estado(1, 10, 10);
    e.amarela.saude = 5;
    e.amarela.quer_comida = 1;
    // Formiga em (9, 10) andando para a direita (2) entra na célula da amarela.
    trofalaxia(&mut e, t(), 9, 10, 2);
    assert_eq!(e.amarela.saude, 100);
    assert_eq!(e.amarela.dir, 6, "virada para a formiga");
    assert_eq!(e.amarela.quer_comida, 0);
}

fn com_operarias(e: &mut Estado, n: u8) {
    for i in 0..n {
        e.adicionar_formiga(Lista::A, Formiga { x: 40 + i, y: 40, modo: 2, tipo: 0x12, s: 0 });
    }
    e.adicionar_formiga(Lista::A, Formiga { x: 60, y: 40, modo: 2, tipo: 0x92, s: 0 });
}

#[test]
fn y7_recrutar_e_liberar() {
    let mut e = estado(1, 10, 10);
    com_operarias(&mut e, 20);
    let mut c = ControleJogador::default();
    let defesa = |e: &Estado| e.superficie.lista.ativas().filter(|(_, f)| f.modo == 6).count();
    assert_eq!(tecla_de_comando(&mut e, t(), &mut c, vk::DIGITO_1, Modificadores::default()), RespostaTecla::Tratada);
    assert_eq!(defesa(&e), 5);
    assert!(e.eventos.contains(&Evento::Musica(0x2B05)));
    tecla_de_comando(&mut e, t(), &mut c, vk::NUMPAD_2, Modificadores::default());
    assert_eq!(defesa(&e), 15);
    tecla_de_comando(&mut e, t(), &mut c, vk::DIGITO_2, Modificadores { ctrl: true, ..Default::default() });
    assert_eq!(defesa(&e), 20, "Ctrl+2 recruta todas as pretas");
    // Liberar usa a contagem de Defense do último tick.
    e.modos_do_tick[0][6] = 20;
    tecla_de_comando(&mut e, t(), &mut c, vk::DIGITO_3, Modificadores::default());
    assert_eq!(defesa(&e), 10);
    tecla_de_comando(&mut e, t(), &mut c, vk::DIGITO_4, Modificadores::default());
    assert_eq!(defesa(&e), 0);
    assert_eq!(e.superficie.lista.vagas[20].modo, 2, "a vermelha não é recrutada");
}

#[test]
fn y8_alarme() {
    let mut e = estado(1, 10, 10);
    let mut c = ControleJogador::default();
    tecla_de_comando(&mut e, t(), &mut c, vk::DIGITO_0, Modificadores::default());
    assert_eq!(e.amarela.soltando_alarme, 1);
    ordem_ir(&mut e, 1, 10, 14);
    andar_ate_parar(&mut e, 10);
    // Células (5, 5), (5, 6), (5, 7): passos em y = 11 | 12, 13 | 14.
    assert_eq!(e.feromonios.alarme.em(5, 5), Some(50));
    assert_eq!(e.feromonios.alarme.em(5, 6), Some(100));
    assert_eq!(e.feromonios.alarme.em(5, 7), Some(50));
    // No ninho não liga.
    let mut e = estado(2, 10, 10);
    assert!(!comando(&mut e, t(), &mut c, 7));
    assert_eq!(e.amarela.soltando_alarme, 0);
}

#[test]
fn y9_entrar_e_sair_do_ninho() {
    let mut e = estado(1, 8, 32);
    e.superficie.mapa.definir(3, 32, 0x50);
    e.preta.buracos[32] = 3;
    // Ninho preto: linha 0 fechada, menos a abertura da coluna 32; terra embaixo, menos o túnel.
    for x in 0..64 {
        e.ninho_b.mapa.definir(x, 0, if x == 32 { 0x18 } else { 0x1E });
        for y in 1..64 {
            e.ninho_b.mapa.definir(x, y, 0x2E);
        }
    }
    for y in 1..6 {
        e.ninho_b.mapa.definir(32, y, 3);
    }
    ordem_ir(&mut e, 1, 3, 32);
    ordem_agir(&mut e, 2, 32, 5); // cavar não, é túnel: ordem 2 (cavar) só em terra
    e.amarela.comando = 0;
    let mut entrou = None;
    for _ in 0..30 {
        let antes = e.amarela.plano;
        passo(&mut e);
        if antes == 1 && e.amarela.plano == 2 {
            entrou = Some((e.amarela.x, e.amarela.y, e.amarela.dir));
        }
        if e.amarela.mover == 0 {
            break;
        }
    }
    assert_eq!(entrou, Some((32, 1, 4)));
    assert_eq!(pos(&e), (2, 32, 5));
    assert_eq!(e.ninho_b.vida.em(32, 5), Some(0xFF));
    // Sair: destino na superfície à direita do buraco.
    ordem_ir(&mut e, 1, 12, 32);
    let mut saiu = None;
    for _ in 0..40 {
        let antes = e.amarela.plano;
        passo(&mut e);
        if antes == 2 && e.amarela.plano == 1 {
            saiu = Some((e.amarela.x, e.amarela.y, e.amarela.dir));
        }
        if e.amarela.mover == 0 {
            break;
        }
    }
    assert_eq!(saiu, Some((4, 32, 2)), "sai na vizinha do buraco na direção do destino");
    assert_eq!(pos(&e), (1, 12, 32));
}

#[test]
fn y10_coluna_64_vai_para_o_ninho_preto() {
    let mut e = estado(1, 66, 20);
    e.superficie.mapa.definir(64, 20, 0x50);
    ordem_ir(&mut e, 1, 64, 20);
    while e.amarela.plano == 1 && e.partida.tempo < 10 {
        passo(&mut e);
    }
    assert_eq!(pos(&e).0, 2, "peculiaridade A1: x < 65 entra no preto");
    assert_eq!((e.amarela.x, e.amarela.y), (20, 1));
}

#[test]
fn y11_luta_por_ordem() {
    let mut e = estado(1, 10, 10);
    e.partida.invencivel = 1;
    e.adicionar_formiga(Lista::A, Formiga { x: 14, y: 10, modo: 2, tipo: 0x96, s: 0 });
    assert_eq!(ordem_agir(&mut e, 1, 14, 10), RespostaClique::Ordem);
    assert_eq!((e.amarela.comando, e.amarela.alvo_indice), (3, 0));
    e.s.definir_semente(0x1234);
    let s0 = e.s;
    andar_ate_parar(&mut e, 10);
    assert_eq!(e.superficie.lista.vagas[0].tipo, 0, "a vermelha morreu");
    assert!((0x14..=0x17).contains(&e.superficie.mapa.em(14, 10).unwrap()), "cadáver vermelho");
    assert_eq!(e.amarela.mover, 0);
    assert_eq!(e.superficie.vida.em(e.amarela.x as i32, 10), Some(0xFF));
    assert_eq!(e.vermelha.mortas_em_luta, 1);
    assert_ne!(e.s, s0, "a animação da luta consome S2 e S8 por quadro");
    // Sorteios: 6 quadros × (S2 + S8) + S4 do cadáver.
    let mut s = s0;
    for _ in 0..13 {
        s.passo();
    }
    assert_eq!(e.s, s);
    // Linha do tempo da luta (regras/som.md): 0x25 nos quadros com S2 ≠ 0 e uma espera de 165 ms
    // entre dois quadros (5 esperas para 6 quadros), na ordem em que o original toca.
    let linha: Vec<&Evento> = e.eventos.iter().filter(|ev| matches!(ev, Evento::Som(37) | Evento::Espera(_))).collect();
    assert_eq!(linha.iter().filter(|ev| matches!(ev, Evento::Espera(165))).count(), 5);
    assert!(linha.iter().any(|ev| matches!(ev, Evento::Som(37))), "S2 ≠ 0 em algum quadro com essa semente");
    assert!(!matches!(linha.last(), Some(Evento::Espera(_))), "sem espera depois do último quadro");
    // Os quadros desenhados (_DoEditUpdateDraw depois de cada _MoveMyLife): a nuvem de luta (tipo
    // 0x70) na célula da amarela, sem a vermelha (apagada antes da animação), pedidos antes da
    // primeira espera para a interface parar a simulação desde o primeiro quadro.
    let i = e.eventos.iter().position(|ev| matches!(ev, Evento::Animacao(_))).expect("animação da luta");
    let primeira_espera = e.eventos.iter().position(|ev| matches!(ev, Evento::Espera(_))).unwrap();
    assert!(i < primeira_espera);
    let Evento::Animacao(a) = &e.eventos[i] else { unreachable!() };
    assert_eq!((a.codigo, a.plano, a.espera_ms, a.quadros.len()), (0, 1, 165, 6));
    assert_eq!(a.duracao_ms(), 825);
    for q in &a.quadros {
        assert_eq!(q.tipo, 0x70);
        assert!((0..8).contains(&q.dir));
        assert_eq!(q.vida[q.x as usize * 64 + q.y as usize], 0xFF);
        assert_eq!(q.vida[14 * 64 + 10], 0, "a vermelha sai do mapa de vida durante a luta");
    }
}

#[test]
fn morte_com_musica_sai_no_primeiro_quadro_no_caminho_mci() {
    // _AnimYellowFight com código 1–9 e música ligada sai quando _mySongIsDone, sempre verdadeiro no
    // MCI: nenhum quadro desenhado, nenhum S8. Com a música desligada, 32 quadros de 165 ms.
    for (musica, quadros) in [(1, 0usize), (0, 32)] {
        let mut e = estado(1, 10, 10);
        e.partida.opcoes[1] = musica;
        e.s.definir_semente(0x4321);
        matar_amarela(&mut e, t(), CausaMorte::Aranha);
        let a = e.eventos.iter().find_map(|ev| if let Evento::Animacao(a) = ev { Some(a) } else { None });
        assert_eq!(a.map_or(0, |a| a.quadros.len()), quadros, "música {musica}");
        if musica != 0 {
            assert_eq!(e.eventos.iter().filter(|ev| matches!(ev, Evento::Espera(_))).count(), 0);
        } else {
            assert_eq!(e.eventos.iter().filter(|ev| matches!(ev, Evento::Espera(165))).count(), 31);
        }
    }
}

#[test]
fn y11b_derrota_mata_e_reencarna_em_adulto() {
    let mut e = estado(1, 10, 10);
    e.partida.opcoes[3] = 1;
    e.adicionar_formiga(Lista::A, Formiga { x: 11, y: 10, modo: 2, tipo: 0xB6, s: 0 });
    e.adicionar_formiga(Lista::A, Formiga { x: 30, y: 30, modo: 2, tipo: 0x13, s: 0 });
    // Procura uma semente de R em que o soldado vermelho vence.
    let mut semente = 0;
    loop {
        let mut r = engine::rules::rng::GeradorR::new(semente);
        if r.rrand(10) >= 2 {
            break;
        }
        semente += 1;
    }
    e.r = engine::rules::rng::GeradorR::new(semente);
    lutar_com_amarela(&mut e, t(), 1, 0);
    assert_eq!(e.preta.mortas_em_luta, 1);
    assert!(e.eventos.contains(&Evento::Mensagem(10032)), "atacada (não começou a luta)");
    assert!(e.eventos.contains(&Evento::Mensagem(10059)), "transferida para um adulto");
    assert_eq!(pos(&e), (1, 30, 30));
    assert_eq!(e.amarela.tipo, 0x10);
    assert_eq!(e.superficie.lista.vagas[1].tipo, 0);
    assert_eq!(e.superficie.vida.em(30, 30), Some(0x10 + e.amarela.dir as u8), "peculiaridade A4");
    assert_eq!(e.superficie.lista.vagas[0].tipo, 0xB6, "o soldado volta à lista");
}

#[test]
fn y12_troca_de_corpo() {
    let mut e = estado(1, 10, 10);
    e.adicionar_formiga(Lista::A, Formiga { x: 20, y: 20, modo: 2, tipo: 0x33, s: 0 });
    let mut c = ControleJogador::default();
    tecla_de_comando(&mut e, t(), &mut c, vk::X, Modificadores::default());
    assert_eq!((e.partida.ferramenta, e.partida.pausado), (10, 1));
    let r = clicar_no_mapa(&mut e, t(), &mut c, 1, 20, 20, false, false);
    assert_eq!(r, RespostaClique::TrocaDeCorpo(true));
    assert_eq!((e.partida.ferramenta, e.partida.pausado), (-1, 0));
    assert_eq!((e.amarela.tipo, e.amarela.dir), (0x30, 3));
    assert_eq!(pos(&e), (1, 20, 20));
    assert_eq!(e.superficie.vida.em(20, 20), Some(0xFF));
    let velho = e.superficie.lista.vagas[1];
    assert_eq!((velho.x, velho.y, velho.tipo, velho.modo), (10, 10, 0x12, 0));
    assert_eq!(e.superficie.vida.em(10, 10), Some(0x12));
    assert_eq!(e.superficie.lista.vagas[0].tipo, 0);
    // Vermelha recusada.
    e.adicionar_formiga(Lista::A, Formiga { x: 40, y: 20, modo: 2, tipo: 0x92, s: 0 });
    assert!(!trocar_de_corpo(&mut e, t(), &mut c, 1, 40, 20));
}

#[test]
fn y13_virar_aranha() {
    let mut e = estado(1, 10, 10);
    e.aranha.x = 64 * 16;
    e.aranha.y = 32 * 16;
    let mut c = ControleJogador::default();
    assert!(trocar_de_corpo(&mut e, t(), &mut c, 1, 64, 32));
    assert_eq!(e.amarela.modo, 1);
    assert_eq!(pos(&e), (1, 64, 32));
    assert_eq!(e.aranha.alvo, -2);
    let velho = e.superficie.lista.vagas[0];
    assert_eq!((velho.x, velho.y, velho.tipo), (10, 10, 0x12));
    assert_eq!(e.superficie.vida.em(10, 10), Some(0), "o corpo devolvido fica sem marca");
    // Clique simples comanda a aranha.
    assert_eq!(ordem_ir(&mut e, 1, 70, 40), RespostaClique::Ordem);
    assert_eq!((e.aranha.destino_x, e.aranha.destino_y, e.aranha.estado), (70, 40, 0));
}

#[test]
fn y14_morte_de_fome_e_renascer_como_ovo() {
    let mut e = estado(1, 10, 10);
    e.partida.opcoes[3] = 1;
    e.ninho_b.lista.adicionar(Formiga { x: 30, y: 10, modo: 8, tipo: 3, s: 0 });
    e.ninho_b.vida.definir(30, 10, 3);
    e.amarela.saude = 0;
    for _ in 0..100 {
        passo(&mut e);
    }
    // Morre quando a contagem passa de 99 (centésimo tick com vida 0).
    assert!(e.eventos.contains(&Evento::Mensagem(10040)));
    assert_eq!(e.preta.mortes_naturais, 1);
    assert_eq!(pos(&e), (2, 30, 10));
    assert_eq!((e.amarela.ovo, e.amarela.tipo, e.amarela.dir), (7, 0x10, 6));
    assert_eq!(e.amarela.saude, 100);
    assert_eq!(e.ninho_b.lista.vagas[0].tipo, 0);
    assert_eq!(e.ninho_b.vida.em(30, 10), Some(0xFF));
    assert_eq!(e.superficie.vida.em(10, 10), Some(0));
    assert_eq!((e.amarela.renascer_x, e.amarela.renascer_y), (30, 10));
}

#[test]
fn quadros_da_morte_pela_aranha_e_pela_formiga_leao() {
    // _YellowDeath(1/2) com Events: _SpiderDialog (sorteia S1(2) para o texto) ou _LionDialog antes
    // da música e da luta; _YellowDialog(9100, 0) depois. Sem Events: nenhum quadro e nenhum S1(2).
    for (causa, eventos) in [(CausaMorte::Aranha, 1), (CausaMorte::Aranha, 0), (CausaMorte::FormigaLeao, 1)] {
        let mut e = estado(1, 10, 10);
        e.partida.opcoes[3] = eventos;
        e.s.definir_semente(0x4321);
        let mut s = e.s;
        let r = if causa == CausaMorte::Aranha && eventos != 0 { s.rand1(2) } else { -1 };
        let musica = s.rand1(5);
        matar_amarela(&mut e, t(), causa);
        let ev = &e.eventos;
        let quadro = ev.iter().position(|x| matches!(x, Evento::DialogoAranha { .. } | Evento::QuadroFormigaLeao));
        let m = ev.iter().position(|x| matches!(x, Evento::Musica(_))).unwrap();
        assert_eq!(ev[m], Evento::Musica(t().musicas.morte_base + musica as u16), "{causa:?} {eventos}");
        let morte = ev.iter().position(|x| *x == Evento::QuadroAmarela { imagem: 9100, indice: 0 }).unwrap();
        assert!(morte > m);
        match (causa, eventos) {
            (CausaMorte::Aranha, 1) => {
                assert_eq!(ev[quadro.unwrap()], Evento::DialogoAranha { texto: if r == 0 { 10047 } else { 10048 } });
                assert!(quadro.unwrap() < m);
            }
            (CausaMorte::FormigaLeao, _) => assert!(quadro.unwrap() < m),
            _ => assert_eq!(quadro, None),
        }
    }
}

#[test]
fn renascer_de_ovo_pede_o_quadro_9101() {
    let mut e = estado(1, 10, 10);
    e.ninho_b.lista.adicionar(Formiga { x: 30, y: 10, modo: 8, tipo: 3, s: 0 });
    e.ninho_b.vida.definir(30, 10, 3);
    matar_amarela(&mut e, t(), CausaMorte::Pisada);
    let ev = &e.eventos;
    let musica = ev.iter().position(|x| *x == Evento::Musica(t().musicas.renascer)).unwrap();
    assert_eq!(ev.last(), Some(&Evento::QuadroAmarela { imagem: 9101, indice: 1 }));
    assert!(ev.len() - 1 > musica);
}

#[test]
fn y14b_sem_corpo_no_quick_e_derrota() {
    let mut e = estado(1, 10, 10);
    matar_amarela(&mut e, t(), CausaMorte::Pisada);
    assert_eq!((e.partida.fim_de_jogo, e.partida.pretas_venceram), (1, 0));
    assert_eq!(e.preta.mortas_em_luta, 1);
}

#[test]
fn y15_cavar_ninho_e_fundar_colonia() {
    let mut e = estado(1, 20, 20);
    e.amarela.tipo = 0x40;
    e.amarela.ninho_iniciado = 0;
    for x in 0..64 {
        for y in 1..64 {
            e.ninho_b.mapa.definir(x, y, 0x2E);
        }
        e.ninho_b.mapa.definir(x, 0, 0x1E);
    }
    let mut c = ControleJogador::default();
    // Tecla 1 da rainha alada: cava o ninho (comando 8).
    tecla_de_comando(&mut e, t(), &mut c, vk::DIGITO_1, Modificadores::default());
    assert_eq!(e.superficie.mapa.em(20, 20), Some(0x50));
    assert_eq!(e.preta.buracos[20], 20);
    assert_eq!(pos(&e), (2, 20, 1));
    // Cava para baixo até y = 5 (ordem 2) e funda.
    ordem_agir(&mut e, 2, 20, 5);
    assert_eq!(e.amarela.comando, 2);
    andar_ate_parar(&mut e, 20);
    assert_eq!(pos(&e), (2, 20, 5));
    assert!(e.ninho_b.mapa.em(20, 4).unwrap() < 8, "cavou o túnel");
    e.amarela.dir = 4;
    tecla_de_comando(&mut e, t(), &mut c, vk::DIGITO_2, Modificadores::default());
    assert_eq!(e.amarela.tipo, 0x10, "virou operária");
    assert_eq!(pos(&e), (2, 20, 3), "na célula do ovo, 2 atrás");
    assert_eq!(e.amarela.dir, 0);
    let rainha: Vec<_> = e.ninho_b.lista.ativas().map(|(_, f)| (f.x, f.y, f.tipo, f.modo)).collect();
    assert_eq!(rainha, vec![(20, 5, 0x64, 9), (20, 4, 0x6C, 9)]);
    assert_eq!(e.amarela.ninho_iniciado, 1);
    assert_eq!(e.preta.rainhas, 1);
}

#[test]
fn rainha_amarela_poe_ovo() {
    let mut e = estado(2, 30, 10);
    e.amarela.tipo = 0x60;
    e.amarela.dir = 2;
    e.amarela.saude = 100;
    e.partida.ciclo = 15;
    e.s.definir_semente(1);
    passo(&mut e);
    // S128 = 2 ≤ 100: ovo 2 células atrás (dir 6 → x − 2).
    let ovo = e.ninho_b.lista.vagas[0];
    assert_eq!((ovo.x, ovo.y, ovo.tipo, ovo.modo), (28, 10, 1, 8));
    assert_eq!(e.amarela.saude, 95);
    assert_eq!(e.ninho_b.vida.em(29, 10), Some(0xFE), "cauda");
}

#[test]
fn backspace_para_a_ordem() {
    let mut e = estado(1, 10, 10);
    ordem_ir(&mut e, 1, 30, 10);
    passo(&mut e);
    let mut c = ControleJogador::default();
    assert_eq!(tecla_de_comando(&mut e, t(), &mut c, vk::BACKSPACE, Modificadores::default()), RespostaTecla::Tratada);
    assert_eq!((e.amarela.mover, e.amarela.alvo_x), (0, 11));
    assert_eq!(
        tecla_de_comando(&mut e, t(), &mut c, vk::BACKSPACE, Modificadores::default()),
        RespostaTecla::Centralizar
    );
    assert_eq!(tecla_de_comando(&mut e, t(), &mut c, 0x41, Modificadores::default()), RespostaTecla::Ignorada);
}

#[test]
fn clique_na_propria_amarela_centraliza() {
    let mut e = estado(1, 10, 10);
    let mut c = ControleJogador::default();
    assert_eq!(clicar_no_mapa(&mut e, t(), &mut c, 1, 10, 10, false, false), RespostaClique::Centralizar);
    assert_eq!(clicar_no_mapa(&mut e, t(), &mut c, 1, 10, 10, false, true), RespostaClique::MenuAmarela);
    assert_eq!(e.amarela.mover, 0);
    // Ninho: destino nas linhas 0–1 tem x limitado a 1..62.
    let mut e = estado(2, 10, 10);
    ordem_ir(&mut e, 2, 63, 1);
    assert_eq!(e.amarela.alvo_x, 62);
}

#[test]
fn tabelas_do_arquivo_de_dados() {
    let texto = std::fs::read_to_string(raiz().join("data/amarela.json")).unwrap();
    let t = TabelasAmarela::de_json(&texto).unwrap();
    assert_eq!(t.matriz_luta[1], [8, 5, 9, 4]);
    assert_eq!(t.borda_buraco, [25, 26, 28, 31, 30, 29, 27, 24]);
}

// ---------------------------------------------------------------------------------------------
// Contra o original
// ---------------------------------------------------------------------------------------------

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn layout() -> LayoutSave {
    LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap()
}

fn abrir(caminho: &Path) -> Option<Estado> {
    let j = JogoSalvo::abrir(&layout(), caminho).ok()?;
    Some(Estado::de_save(&j).unwrap())
}

/// Campos da amarela que a comparação confere.
fn resumo(e: &Estado) -> Vec<(&'static str, i16)> {
    let a = &e.amarela;
    vec![
        ("MePlane", a.plano),
        ("MeLocX", a.x),
        ("MeLocY", a.y),
        ("MeType", a.tipo),
        ("MeDir", a.dir),
        ("MeHealth", a.saude),
        ("MeHealthCntDwn", a.saude_contagem),
        ("MeWantFood", a.quer_comida),
        ("MeMoveMe", a.mover),
        ("MeCmd", a.comando),
        ("MeGoalPlane", a.alvo_plano),
        ("MeGoalX", a.alvo_x),
        ("MeGoalY", a.alvo_y),
        ("MeLastX", a.ultimo_x),
        ("MeLastY", a.ultimo_y),
        ("MeSteps", a.passos),
        ("MeCrazyCnt", a.contorno_contagem),
        ("MeCrazyDir", a.contorno_dir),
        ("MeCrazyRot", a.contorno_rot),
        ("MeScent", a.rastro),
        ("MeFoodInType", a.comeu_tipo),
    ]
}

/// Vetores de tick do original (tests/referencias/ticks, docs/spec/regras/testes-tick.md): a amarela
/// parada no ninho preto. Só a parte da amarela é comparada (as outras regras ainda são vazias).
#[test]
fn vetores_de_tick_do_original() {
    let dir = raiz().join("tests/referencias/ticks");
    let mut n = 0;
    for tick in 79..=83 {
        let d = dir.join(format!("tick_{tick:06}"));
        let (Some(mut antes), Some(depois)) = (abrir(&d.join("estado_antes.bin")), abrir(&d.join("estado_depois.bin")))
        else {
            continue;
        };
        passo(&mut antes);
        assert_eq!(resumo(&antes), resumo(&depois), "tick {tick}");
        let (x, y) = (depois.amarela.x as i32, depois.amarela.y as i32);
        assert_eq!(antes.ninho_b.vida.em(x, y), Some(0xFF));
        n += 1;
    }
    assert!(n == 0 || n == 5);
}

/// Casos Y-01..Y-04 da bancada (tests/referencias/casos). No original, a amarela com ordem anda mesmo com
/// o jogo pausado depois do load, então boa parte do caminho acontece antes do primeiro estado lido
/// (`Yxx_CARREGADO.ANT`). Por isso o port parte do arquivo injetado (`Yxx_ENTRADA.ANT`, GameTime 78) e
/// roda ticks completos, conferindo a amarela no GameTime do estado carregado e no final (`Yxx_FIM.ANT`).
/// As regras dos outros assuntos ainda são vazias: os casos foram montados longe das outras formigas e
/// com a aranha desligada, e só a amarela (mais a saúde da colônia no Y-03) é comparada.
#[test]
fn casos_da_bancada() {
    let saves = raiz().join("tests/referencias/saves");
    for caso in ["Y01", "Y02", "Y03", "Y04"] {
        let arq = |k: &str| saves.join(format!("{caso}_{k}.ANT"));
        let (Some(mut e), Some(carregado), Some(fim)) = (abrir(&arq("ENTRADA")), abrir(&arq("CARREGADO")), abrir(&arq("FIM")))
        else {
            eprintln!("{caso}: saves da bancada ausentes, caso não conferido");
            continue;
        };
        let mut regras = RegrasOriginais::default();
        for alvo in [&carregado, &fim] {
            while e.partida.tempo < alvo.partida.tempo {
                tick(&mut e, &mut regras);
            }
            assert_eq!(resumo(&e), resumo(alvo), "{caso} no GameTime {}", alvo.partida.tempo);
        }
        if caso == "Y03" {
            // O load do original reconta as castas pelas listas (docs/bancada-qemu.md): `_BpopT` = 2.
            let mut e = abrir(&arq("ENTRADA")).unwrap();
            e.contar_formigas();
            while e.partida.tempo < carregado.partida.tempo {
                tick(&mut e, &mut regras);
            }
            // No tick 80 a rainha preta pôs um ovo com `_EatCountB` = 0 (−1 de `_HealthB`, ninho.md 6.1);
            // enquanto as regras do ninho não estiverem neste ramo, o port não tem esse ovo.
            let ovos_faltando = (carregado.partida.ovos_postos_b - e.partida.ovos_postos_b) as i16;
            assert_eq!(e.preta.saude - ovos_faltando, carregado.preta.saude, "Y03: 40 + 49 / (2 − 1)");
            assert_eq!(e.superficie.mapa.em(62, 36), carregado.superficie.mapa.em(62, 36));
        }
    }
}
