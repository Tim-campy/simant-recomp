//! Estado da simulação e tick contra os saves do original (`QUICK2.ANT`, tick 78; `QUICK3.ANT`, tick 92).
//! Especificação: docs/spec/regras/simulacao-visao-geral.md, seção "Implementação no port".
//! O teste de `Estado::de_mundo` precisa da pasta do jogo (SIMANT_DIR ou ../SimAnt-T/MAXIS/ANTWIN);
//! sem ela, avisa e passa sem verificar.

use std::collections::BTreeSet;
use std::path::PathBuf;

use engine::formats::save::{JogoSalvo, LayoutSave, Lista};
use engine::recursos::Recursos;
use engine::rules::estado::{ConfigSimulacao, Estado};
use engine::rules::geracao::{Mundo, TabelasGeracao, TipoJogo, PATCH_INICIAL};
use engine::rules::rng::{GeradorR, GeradorS};
use engine::rules::tick::{
    tick, RegrasAmarela, RegrasConselhos, RegrasInimigos, RegrasNinho, RegrasQuintal,
    RegrasSuperficie,
};

/// Regras com todas as sub-rotinas vazias: isola o esqueleto do tick (contadores, periodicidade,
/// contagem, compactação) das partes já implementadas em `RegrasOriginais`.
#[derive(Default)]
struct RegrasVazias;
impl RegrasQuintal for RegrasVazias {}
impl RegrasInimigos for RegrasVazias {}
impl RegrasSuperficie for RegrasVazias {}
impl RegrasNinho for RegrasVazias {}
impl RegrasAmarela for RegrasVazias {}
impl RegrasConselhos for RegrasVazias {}

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn layout() -> LayoutSave {
    LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap()
}

fn save(nome: &str) -> JogoSalvo {
    JogoSalvo::abrir(&layout(), &raiz().join("tests/referencias/saves").join(nome)).unwrap()
}

fn config() -> ConfigSimulacao {
    ConfigSimulacao::de_json(&std::fs::read_to_string(raiz().join("data/simulacao.json")).unwrap()).unwrap()
}

#[test]
fn estado_cobre_todos_os_blocos_do_save() {
    let cobertos: Vec<String> = Estado::simbolos_cobertos();
    let unicos: BTreeSet<&str> = cobertos.iter().map(String::as_str).collect();
    assert_eq!(unicos.len(), cobertos.len(), "símbolo coberto duas vezes");
    let layout: BTreeSet<String> = layout().blocos.iter().map(|b| b.simbolo.clone()).collect();
    let cobertos: BTreeSet<String> = cobertos.into_iter().collect();
    assert_eq!(cobertos, layout);
}

#[test]
fn ida_e_volta_byte_a_byte() {
    let l = layout();
    for nome in ["QUICK2.ANT", "QUICK3.ANT"] {
        let original = save(nome);
        let e = Estado::de_save(&original).unwrap();
        let regravado = e.para_save(&l).unwrap();
        let dif: Vec<&str> = original.simbolos().filter(|s| original.bloco(s) != regravado.bloco(s)).collect();
        assert!(dif.is_empty(), "{nome}: blocos diferentes: {dif:?}");
        assert_eq!(original.bytes(), regravado.bytes(), "{nome}");
    }
}

#[test]
fn campos_tipados_do_quick2() {
    let e = Estado::de_save(&save("QUICK2.ANT")).unwrap();
    assert_eq!((e.partida.tempo, e.partida.ciclo, e.ambiente.ciclo_quintal), (78, 78, 77));
    assert_eq!(e.partida.tipo_jogo, 1);
    assert_eq!(e.ambiente.patch, (11, 8));
    assert_eq!(e.ambiente.sementes[8 * 12 + 11], 0x3332);
    assert_eq!((e.preta.buracos[32], e.vermelha.buracos[32]), (3, 118));
    assert_eq!(e.preta.castas, [2, 2, 0, 0, 0, 1]);
    assert_eq!(e.controles.tarefas, [39321, 13107, 13107]);
    assert_eq!((e.historico.inicio, e.historico.contagem), (0, 1));
    assert_eq!((e.amarela.plano, e.amarela.x, e.amarela.y, e.amarela.tipo), (2, 34, 9, 0x10));
    assert_eq!(e.superficie.lista.indice, 2);
    // Vida reconstruída: no save, a amarela e a cabeça da rainha preta estão ambas em (34, 9); a
    // reconstrução grava a amarela por último (_SetMyLife depois das listas).
    assert_eq!(e.ninho_b.vida.em(34, 9), Some(0xFF));
    // Geradores: S na semente do patch atual (convenção do port; o original não salva S).
    assert_eq!(e.s.semente(), 0x3332);
}

#[test]
fn contagem_bate_com_o_save_respeitando_a_defasagem() {
    // A última contagem do original foi no tick 64; ovos postos depois disso não estão em
    // CastePop[0]. Os adultos (índices 1..5) e os totais batem.
    for nome in ["QUICK2.ANT", "QUICK3.ANT"] {
        let mut e = Estado::de_save(&save(nome)).unwrap();
        let (antes_b, antes_r) = (e.preta.clone(), e.vermelha.clone());
        e.contar_formigas();
        for (cor, antes, depois) in [("preta", &antes_b, &e.preta), ("vermelha", &antes_r, &e.vermelha)] {
            assert_eq!(&depois.castas[1..], &antes.castas[1..], "{nome} {cor}");
            assert!(depois.castas[0] >= antes.castas[0], "{nome} {cor}: ovos");
            assert_eq!(depois.populacao, antes.populacao, "{nome} {cor}: população");
        }
        assert_eq!(e.partida.fim_de_jogo, 0, "{nome}: rainhas vivas");
        assert!(e.eventos.is_empty());
    }
}

#[test]
fn tick_vazio_sobre_o_quick2() {
    let l = layout();
    let original = save("QUICK2.ANT");
    let mut e = Estado::de_save(&original).unwrap();
    let mut regras = RegrasVazias;
    tick(&mut e, &mut regras);
    assert_eq!((e.partida.tempo, e.partida.ciclo), (79, 79));
    // Com as sub-rotinas ainda vazias, só mudam _GameTime, _Cycle e o resumo de modos (_TallyModePop
    // roda todo tick e, sem formigas simuladas, a contagem do tick é zero). O tick 79 não é periódico.
    let novo = e.para_save(&l).unwrap();
    let dif: Vec<&str> = original.simbolos().filter(|s| original.bloco(s) != novo.bloco(s)).collect();
    assert_eq!(dif, vec!["GameTime", "ModePopB", "ModePopR", "Cycle"]);
    assert_eq!((e.preta.modos, e.vermelha.modos), ([0; 6], [0; 6]));
    // Até o tick 128: compactação de R (96) e de A (128), saúde −1 e contagem completa (128).
    // As listas do QUICK2 não têm vagas com T = 0, então os índices não mudam.
    for _ in 79..128 {
        tick(&mut e, &mut regras);
    }
    assert_eq!(e.partida.ciclo, 128);
    assert_eq!(e.historico.contagem, 2);
    assert_eq!(e.preta.saude, original.i16("HealthB").unwrap() - 1);
    assert_eq!(e.ninho_r.lista.indice, original.i16("ListIndexR").unwrap());
}

/// `Estado::de_mundo` a partir da geração do patch do QUICK2: os campos iniciais que não mudam em
/// 78 ticks batem com o save.
#[test]
fn estado_do_mundo_gerado_bate_com_o_quick2() {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
        return;
    }
    let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    let tab = TabelasGeracao::carregar(&r).unwrap();
    let q2 = Estado::de_save(&save("QUICK2.ANT")).unwrap();

    let mut m = Mundo::vazio(TipoJogo::Rapido);
    m.sementes = q2.ambiente.sementes.0;
    m.ninho_iniciado = true;
    let (px, py) = PATCH_INICIAL;
    let mut s = GeradorS::new(0);
    m.gerar_patch(&mut s, &tab, m.sementes[(py * 12 + px) as usize], 1, 1, px, py).unwrap();
    let e = Estado::de_mundo(&m, s, GeradorR::default(), &config()).unwrap();

    assert_eq!((e.partida.tempo, e.partida.ciclo, e.ambiente.ciclo_quintal), (0, 0, -1));
    // _MakeMe e _ModeMe são escolhidos pela estratégia a cada tick; o resto vem de _initControls.
    let sem_escolhas = |c: &engine::rules::estado::Controles| engine::rules::estado::Controles {
        proxima_casta: 0,
        proximo_modo: 0,
        ..c.clone()
    };
    assert_eq!(sem_escolhas(&e.controles), sem_escolhas(&q2.controles));
    assert_eq!(e.partida.opcoes, q2.partida.opcoes);
    assert_eq!(e.partida.velocidade, q2.partida.velocidade);
    assert_eq!(e.partida.ferramenta, q2.partida.ferramenta);
    assert_eq!(e.partida.acabou_de_trocar, 0);
    assert_eq!(e.historico.botoes, q2.historico.botoes);
    assert_eq!((e.historico.inicio, e.historico.contagem), (63, 0));
    assert_eq!(e.ambiente.barreira, q2.ambiente.barreira);
    assert_eq!(e.ambiente.proxima_comida, q2.ambiente.proxima_comida);
    assert_eq!(e.ambiente.patch, q2.ambiente.patch);
    assert_eq!(e.ambiente.sementes, q2.ambiente.sementes);
    assert_eq!((e.vistas.plano, e.vistas.modo_mapa, e.vistas.ponto_quintal), (2, 2, (11, 8)));
    assert_eq!(e.vistas.plano, q2.vistas.plano);
    assert_eq!(e.amarela.aviso_saude, q2.amarela.aviso_saude);
    assert_eq!(e.partida.aviso_saude_colonia, q2.partida.aviso_saude_colonia);
    assert_eq!(e.ambiente.comida_superficie, q2.ambiente.comida_superficie);
    for (cor, a, b) in [("preta", &e.preta, &q2.preta), ("vermelha", &e.vermelha, &q2.vermelha)] {
        assert_eq!(a.buracos, b.buracos, "{cor}");
        assert_eq!(a.ultimo_buraco, b.ultimo_buraco, "{cor}");
        assert_eq!(a.ultima_saida, b.ultima_saida, "{cor}");
        assert_eq!(a.rainhas, b.rainhas, "{cor}");
        assert_eq!(a.tiles_escavados, b.tiles_escavados, "{cor}");
        // Na geração: só a rainha (as operárias da superfície e a amarela entram no censo).
        assert_eq!(a.castas[5], 1, "{cor}");
    }
    assert_eq!(e.lista(Lista::A).indice, q2.lista(Lista::A).indice);
    assert_eq!(e.aranha.arrotos, q2.aranha.arrotos);
    assert_eq!(e.aranha.alvo, q2.aranha.alvo);
    assert_eq!(e.formigas_leao.quantidade, q2.formigas_leao.quantidade);
    assert_eq!(e.amarela.renascer_x, q2.amarela.renascer_x);
    // Gera um .ANT válido.
    e.para_save(&layout()).unwrap();
}
