//! Quintal, água e colônias contra o original (docs/spec/regras/quintal.md §13).
//!
//! - Q-02 (bancada, `tests/referencias/casos/Q-02.json`): uma passada de `_SimColonies`; o estado
//!   final do original está em `saves/Q02_T100.ANT`.
//! - Q-01 (bancada): chuva por 62 ticks; estado final em `saves/Q01_T140.ANT`.
//! - Vetores de tick do original (`tests/referencias/ticks/`, ticks 79–83; testes-tick.md).

use std::path::PathBuf;

use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::rules::amarela::TabelasAmarela;
use engine::rules::estado::{Estado, Evento};
use engine::rules::geracao::Cor;
use engine::rules::inimigos::TabelasInimigos;
use engine::rules::quintal::{
    placar_vizinhos, reproduzir, simular_agua, simular_colonias, simular_quintal_com, Relogios, TabelasQuintal, Vitimas,
};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn layout() -> LayoutSave {
    LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap()
}

fn abrir(caminho: &str) -> JogoSalvo {
    JogoSalvo::abrir(&layout(), &raiz().join(caminho)).unwrap()
}

fn tabelas() -> TabelasQuintal {
    TabelasQuintal::de_json(&std::fs::read_to_string(raiz().join("data/quintal.json")).unwrap()).unwrap()
}

fn i(x: usize, y: usize) -> usize {
    x * 16 + y
}

#[test]
fn json_do_repositorio_igual_ao_embutido() {
    let t = tabelas();
    let p = TabelasQuintal::padrao();
    assert_eq!(t.casa.duracao, p.casa.duracao);
    assert_eq!(t.colonias.vizinho_dy, p.colonias.vizinho_dy);
}

#[test]
fn placar_ignora_vizinhos_de_cima_e_da_esquerda() {
    let t = tabelas();
    let mut e = Estado::vazio();
    e.preta.populacao_quintal[i(5, 5)] = 10;
    assert_eq!(placar_vizinhos(&e, &t, 5, 5), 6, "o próprio patch conta duas vezes");
    e.preta.populacao_quintal[i(6, 5)] = 10;
    assert_eq!(placar_vizinhos(&e, &t, 5, 5), 9, "vizinho em x + 1");
    assert_eq!(placar_vizinhos(&e, &t, 6, 5), 6, "vizinho em x − 1 não conta");
    e.vermelha.populacao_quintal[i(5, 6)] = 1;
    assert_eq!(placar_vizinhos(&e, &t, 5, 5), 6, "vermelha em y + 1 tira 3");
}

/// Q-U2: a passada do caso Q-02 dá os mesmos dois mapas que o original.
#[test]
fn colonias_iguais_ao_original_q02() {
    let t = tabelas();
    let mut e = Estado::de_save(&abrir("tests/referencias/saves/QUICK2.ANT")).unwrap();
    let fim = Estado::de_save(&abrir("tests/referencias/saves/Q02_T100.ANT")).unwrap();
    for (idx, b, r) in [(34, 10, 0), (85, 10, 0), (101, 20, 0), (138, 0, 10), (156, 5, 5), (29, 3, 0), (30, 0, 10)] {
        if b != 0 {
            e.preta.populacao_quintal[idx] = b;
        }
        if r != 0 {
            e.vermelha.populacao_quintal[idx] = r;
        }
    }
    e.preta.enxame_contagem = 40;
    e.vermelha.enxame_contagem = 3;
    // O load do original recontou _BpopT/_RpopT; o patch atual recebe esses valores.
    e.preta.populacao = fim.preta.populacao;
    e.vermelha.populacao = fim.vermelha.populacao;
    e.ambiente.ciclo_quintal = 96;
    simular_colonias(&mut e, &t);
    assert_eq!(*e.preta.populacao_quintal, *fim.preta.populacao_quintal);
    assert_eq!(*e.vermelha.populacao_quintal, *fim.vermelha.populacao_quintal);
    assert_eq!((e.preta.total_colonias, e.vermelha.total_colonias), (6, 4));
    assert_eq!((e.preta.enxame_contagem, e.vermelha.enxame_contagem), (30, 2));
    assert_eq!((e.preta.enxame_atraso, e.vermelha.enxame_atraso), (4, 4));
    assert_eq!((fim.preta.total_colonias, fim.vermelha.total_colonias), (6, 4));
}

/// Q-U3: patch no teto reproduz para um vizinho a até ±3.
#[test]
fn reproducao_no_teto() {
    let t = tabelas();
    let mut e = Estado::vazio();
    e.s.definir_semente(0x1234);
    for _ in 0..50 {
        reproduzir(&mut e, &t, 5, 5, Cor::Preta);
    }
    let total: u32 = e.preta.populacao_quintal.iter().map(|&v| v as u32).sum();
    let ocupados = e.preta.populacao_quintal.iter().filter(|&&v| v != 0).count();
    assert_eq!(e.preta.colonias_fundadas as usize, ocupados, "fundada = patch que estava vazio");
    assert!(total as usize >= ocupados);
    for x in 0..12 {
        for y in 0..16 {
            if e.preta.populacao_quintal[i(x, y)] != 0 {
                assert!((x as i32 - 5).abs() <= 3 && (y as i32 - 5).abs() <= 3, "({x},{y}) longe demais");
            }
        }
    }
    assert_eq!(e.preta.populacao_quintal[i(5, 5)], 0, "nunca no próprio patch");
    assert!(e.preta.colonias_fundadas > 0);
}

/// Q-U4: vermelha extinta em px = 11 cria uma vermelha nova em (2 + S(6), 0).
#[test]
fn vermelha_extinta_na_coluna_11() {
    let t = tabelas();
    let mut e = Estado::vazio();
    e.ambiente.patch = (0, 15);
    e.vermelha.populacao_quintal[i(11, 3)] = 1;
    // Placar de (11, 3): +6 (preta aqui) − 6 (vermelha aqui) + 3 (preta em (11, 4)) = 3 → vermelha 1 − 3 < 1.
    e.preta.populacao_quintal[i(11, 3)] = 1;
    e.preta.populacao_quintal[i(11, 4)] = 5;
    simular_colonias(&mut e, &t);
    assert_eq!(e.vermelha.populacao_quintal[i(11, 3)], 0);
    assert_eq!(e.vermelha.colonias_extintas, 1);
    let novas: Vec<usize> = (2..8).filter(|&x| e.vermelha.populacao_quintal[i(x, 0)] == 20).collect();
    assert_eq!(novas.len(), 1);
    // O laço pula para py = 9: (11, 4) não é processado nessa passada.
    assert_eq!(e.preta.populacao_quintal[i(11, 4)], 5);
}

/// Q-U5: o cortador fora do patch atual tira um quarto da colônia.
#[test]
fn cortador_fora_do_patch_atual() {
    let t = tabelas();
    let mut e = Estado::vazio();
    e.menino.no_numero = 0x26;
    e.menino.presente = 3;
    e.menino.cortando_grama = 1;
    e.menino.px = 5;
    e.menino.py = 5;
    (e.menino.ultimo_corte_x, e.menino.ultimo_corte_y) = (5, 5);
    // Posição em pixels que cai em (5, 5) depois do passo: X + Y − 200 = 5·28 + 2, Y − 38 = 55.
    e.menino.y = 93;
    e.menino.x = 249;
    e.menino.dir = 1;
    e.ambiente.patch = (11, 8);
    e.ambiente.ciclo_quintal = 4;
    e.preta.populacao_quintal[i(5, 5)] = 100;
    e.bichos.passaro_atraso = i32::MAX;
    let rel = Relogios { ciclos: 0, mac: 0 };
    simular_quintal_com(&mut e, &t, Vitimas::de(TabelasAmarela::padrao(), TabelasInimigos::padrao()), rel);
    assert_eq!((e.menino.px, e.menino.py), (5, 5));
    assert_eq!(e.preta.populacao_quintal[i(5, 5)], 75);
}

/// Q-U6: roteiro da casa; QUICK2 tem `_NodeNum = 3`, `_NodeCnt = 10` em (126, 92).
#[test]
fn roteiro_da_casa_bate_com_o_quick2() {
    let t = tabelas();
    let mut e = Estado::vazio();
    e.menino.no_numero = 3;
    e.menino.no_contagem = 9;
    e.ambiente.patch = (11, 8);
    e.ambiente.ciclo_quintal = 0;
    e.bichos.passaro_atraso = i32::MAX;
    simular_quintal_com(&mut e, &t, Vitimas::de(TabelasAmarela::padrao(), TabelasInimigos::padrao()), Relogios { ciclos: 0, mac: 0 });
    assert_eq!((e.menino.x, e.menino.y, e.menino.no_contagem), (126, 92, 10));
    assert_eq!((e.menino.px, e.menino.py, e.menino.presente), (0, 5, 0));
}

/// Q-U7: a tecla 0xA8 faz chover: `_RainCnt` em 150..299 e 100 gotas pelo gerador R.
#[test]
fn tecla_faz_chover() {
    let t = tabelas();
    let mut e = Estado::vazio();
    e.tecla_global = 0xA8;
    e.menino.no_numero = 0x26;
    e.bichos.passaro_atraso = i32::MAX;
    let r_antes = e.r;
    simular_quintal_com(&mut e, &t, Vitimas::de(TabelasAmarela::padrao(), TabelasInimigos::padrao()), Relogios { ciclos: 0, mac: 0 });
    assert_eq!(e.ambiente.chovendo, 1);
    assert!((150..300).contains(&e.ambiente.chuva_contagem));
    assert_ne!(e.r, r_antes);
    let mut r = r_antes;
    let (x0, y0) = (r.rrand(128), r.rrand(64));
    assert_eq!((e.ambiente.gotas_x[0] as i32, e.ambiente.gotas_y[0] as i32), (x0, y0));
    assert_eq!((e.menino.mensagem_ligada, e.menino.mensagem_offset), (1, 0));
}

/// Q-U8: água do caso Q-01: as linhas 61–63 inundadas no original são as do QUICK2 transformadas.
#[test]
fn inundacao_igual_ao_original_q01() {
    let antes = Estado::de_save(&abrir("tests/referencias/saves/QUICK2.ANT")).unwrap();
    let fim = Estado::de_save(&abrir("tests/referencias/saves/Q01_T140.ANT")).unwrap();
    assert_eq!(fim.ambiente.lencol, 61);
    assert_eq!(fim.ambiente.chuva_contagem, 538);
    let mut e = antes.clone();
    e.ambiente.chovendo = 1;
    // Sobe a água até 61 pelo próprio `_DoWater` (S(10) == 0 três vezes).
    let mut ticks = 0;
    while e.ambiente.lencol > 61 {
        simular_agua(&mut e, TabelasQuintal::padrao());
        ticks += 1;
        assert!(ticks < 10_000);
    }
    assert_eq!(e.ambiente.limpar_chuva, 1);
    for (cor, nome) in [(Cor::Preta, "MapB"), (Cor::Vermelha, "MapR")] {
        for y in 61..64 {
            for x in 0..64 {
                assert_eq!(e.ninho(cor).mapa.em(x, y), fim.ninho(cor).mapa.em(x, y), "{nome} ({x}, {y})");
            }
        }
        for x in 0..64 {
            assert_eq!(e.ninho(cor).mapa.em(x, 60), antes.ninho(cor).mapa.em(x, 60));
        }
    }
    // Secando: as linhas voltam (0x4E vira túnel S(8); o resto −0x2F) e o lençol desce.
    e.ambiente.chovendo = 0;
    while e.ambiente.lencol < 64 {
        simular_agua(&mut e, TabelasQuintal::padrao());
    }
    for x in 0..64 {
        for y in 61..64 {
            let v = antes.ninho_b.mapa.em(x, y).unwrap();
            let w = e.ninho_b.mapa.em(x, y).unwrap();
            if v < 0x20 {
                assert!(w < 8, "túnel volta como S(8)");
            } else {
                assert_eq!(w, v);
            }
        }
    }
}

/// Q-U9: vitória e derrota do Jogo Completo.
#[test]
fn fim_do_jogo_completo() {
    let t = tabelas();
    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 2;
    e.ambiente.casa_a_venda = 1;
    e.preta.populacao = 10;
    e.preta.castas = [0, 10, 0, 0, 0, 1];
    simular_colonias(&mut e, &t);
    assert_eq!((e.partida.fim_de_jogo, e.partida.pretas_venceram), (1, 1));
    assert!(e.eventos.contains(&Evento::Mensagem(10057)));

    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 2;
    e.vermelha.populacao = 5;
    e.preta.populacao = 3;
    e.preta.castas = [0, 3, 0, 0, 0, 0];
    simular_colonias(&mut e, &t);
    assert_eq!((e.partida.fim_de_jogo, e.partida.pretas_venceram), (1, 0));
    assert_eq!(e.eventos, vec![Evento::Mensagem(10056)]);

    // Vermelhas eliminadas (havia vermelha na passada anterior) e casa tomada.
    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 2;
    e.vermelha.total_colonias = 1;
    e.preta.populacao = 3;
    e.preta.castas = [1, 3, 0, 0, 0, 1];
    for x in 0..2 {
        for y in 0..13 {
            e.preta.populacao_quintal[i(x, y)] = 1;
        }
    }
    e.ambiente.patch = (11, 8);
    simular_colonias(&mut e, &t);
    assert_eq!(e.ambiente.colonias_na_casa, 26);
    assert_eq!(e.ambiente.casa_a_venda, 1);
    assert_eq!(e.menino.quadro, -1);
    assert!(e.eventos.contains(&Evento::Mensagem(10052)));
    assert!(e.eventos.contains(&Evento::Mensagem(10055)));
    assert!(e.eventos.contains(&Evento::Mensagem(10054)));
    assert!(e.eventos.contains(&Evento::Mensagem(10057)), "sem vermelhas e casa à venda: vitória");
}

/// `_LastColonyPopR` (fora do save): 1 depois de `_InitSimYard` (jogo novo e Load), então a primeira
/// passada do Jogo Completo sem vermelhas mostra a 10052 mesmo que a passada anterior (de antes do
/// Load) já não tivesse vermelhas; a segunda não repete. Fora do Jogo Completo nada é gravado.
#[test]
fn ultima_populacao_vermelha_comeca_em_1() {
    let t = tabelas();
    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 2;
    e.preta.populacao = 3;
    e.preta.castas = [1, 3, 0, 0, 0, 1];
    e.ultimas_colonias_vermelhas = Some(1);
    simular_colonias(&mut e, &t);
    assert!(e.eventos.contains(&Evento::Mensagem(10052)));
    assert_eq!(e.ultimas_colonias_vermelhas, Some(0));
    e.eventos.clear();
    simular_colonias(&mut e, &t);
    assert!(!e.eventos.contains(&Evento::Mensagem(10052)));

    let mut q = Estado::vazio();
    q.partida.tipo_jogo = 1;
    q.ultimas_colonias_vermelhas = Some(1);
    simular_colonias(&mut q, &t);
    assert_eq!(q.ultimas_colonias_vermelhas, Some(1));
}

/// Q-U10: vetores de tick do original. Até `_DoSimYard` os ticks 79–83 não sorteiam nada (nenhum é
/// múltiplo de 32 nem de 64), então o S da entrada do tick é o S de `_DoSimYard`. Os campos do
/// quintal depois do tick devem bater com o original.
#[test]
fn vetores_de_tick_do_original() {
    let t = tabelas();
    for tick in 79..=83 {
        let dir = raiz().join(format!("tests/referencias/ticks/tick_{tick:06}"));
        let extras: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("extras.json")).unwrap()).unwrap();
        let antes = &extras["antes"];
        let l = layout();
        let ja = JogoSalvo::ler(&l, std::fs::read(dir.join("estado_antes.bin")).unwrap()).unwrap();
        let jd = JogoSalvo::ler(&l, std::fs::read(dir.join("estado_depois.bin")).unwrap()).unwrap();
        let mut e = Estado::de_save(&ja).unwrap();
        let fim = Estado::de_save(&jd).unwrap();
        e.s.definir_semente(antes["SementeS"].as_u64().unwrap() as u16);
        e.tecla_global = antes["GlobalKey"].as_i64().unwrap() as i16;
        let vida = std::fs::read(dir.join("vida_antes.bin")).unwrap();
        e.superficie.vida.dados.copy_from_slice(&vida[..8192]);
        let rel = Relogios { ciclos: antes["gameCycles"].as_i64().unwrap(), mac: 0 };
        simular_quintal_com(&mut e, &t, Vitimas::de(TabelasAmarela::padrao(), TabelasInimigos::padrao()), rel);
        let mut m = e.menino.clone();
        m.boy_msg_cnt = fim.menino.boy_msg_cnt; // relógio do Mac (não capturado)
        assert_eq!(m, fim.menino, "tick {tick}: menino");
        let mut b = e.bichos.clone();
        b.gato_atraso = fim.bichos.gato_atraso;
        b.cachorro_atraso_latido = fim.bichos.cachorro_atraso_latido;
        assert_eq!(b, fim.bichos, "tick {tick}: gato, cachorro, pássaro");
        assert_eq!(e.ambiente.ciclo_quintal, fim.ambiente.ciclo_quintal);
        assert_eq!(e.ambiente.chovendo, fim.ambiente.chovendo);
        assert_eq!(*e.preta.populacao_quintal, *fim.preta.populacao_quintal);
    }
}

#[test]
fn botao_mate_arma_a_revoada_so_no_jogo_completo_com_reprodutores() {
    use engine::rules::estado::MensagemFaixa;
    use engine::rules::quintal::botao_revoada;
    let t = tabelas();
    let indice = |e: &Estado| {
        e.eventos.iter().find_map(|ev| match ev {
            Evento::MensagemFaixa(MensagemFaixa { lista: 1010, indice, importante: true, .. }) => Some(*indice),
            _ => None,
        })
    };
    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 1;
    assert!(!botao_revoada(&mut e, &t));
    assert_eq!(indice(&e), Some(15), "Fertilize queens in Full Game only");
    assert!(e.eventos.contains(&Evento::Som(1)));
    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 2;
    e.preta.castas[4] = 1;
    e.amarela.tipo = 0x40; // a amarela rainha alada é a única: não conta
    assert!(!botao_revoada(&mut e, &t));
    assert_eq!(indice(&e), Some(5), "Produce males and new queens first");
    assert_eq!(e.preta.revoada, 0);
    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 2;
    e.preta.castas[3] = 2;
    assert!(botao_revoada(&mut e, &t));
    assert_eq!(e.preta.revoada, 200);
    assert_eq!(indice(&e), Some(6), "You have started the mating cycle");
    assert!(!e.eventos.contains(&Evento::Som(1)));
}

#[test]
fn cortador_toca_no_desenho_do_quintal_e_gato_fugindo_tem_som() {
    use engine::rules::desenho::desenhar_cortador;
    let mut e = Estado::vazio();
    e.vistas.modo_quintal = 0;
    for (presente, pausado, esperado) in [(3, 0, true), (4, 0, true), (1, 0, false), (3, 1, false)] {
        e.eventos.clear();
        e.menino.presente = presente;
        e.partida.pausado = pausado;
        desenhar_cortador(&mut e, Some(36));
        assert_eq!(e.eventos.contains(&Evento::Som(36)), esperado, "_BoyHere {presente}, pausado {pausado}");
    }
    assert_eq!(tabelas().sons.gato_fugindo, Some(14));
    assert_eq!(tabelas().sons.cortador, Some(36));
}
