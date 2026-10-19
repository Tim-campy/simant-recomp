//! Fim de jogo pela interface (docs/spec/regras/testes-tick.md, pendente `inseticida_morte/21`): no
//! original a amarela morre pelo inseticida no Jogo Completo sem corpo e sem outra colônia preta, o
//! tick abre "GAME OVER" (pontuação 27572, captura `re_trabalho/cen/dialogo_21_1.png`) e, depois do
//! clique, começa um Jogo Completo novo. Aqui: o mesmo tick numa `core::partida::Partida` (com o
//! relógio do Mac do vetor), o pedido do diálogo uma só vez, o quadro modal com a pontuação do
//! original, e o jogo novo com a forma do que o original começou.

use std::path::PathBuf;

use engine::core::avisos::{ConfigAvisos, Modal};
use engine::core::navegacao::Navegacao;
use engine::core::partida::{ConfigPartida, Partida};
use engine::core::tela_jogo::PosicaoVista;
use engine::formats::save::LayoutSave;
use engine::recursos::Recursos;
use engine::rules::amarela::TabelasAmarela;
use engine::rules::estado::{ConfigSimulacao, Estado, Evento};
use engine::rules::geracao::{Mundo, TabelasGeracao, TipoJogo};
use engine::rules::rng::{GeradorR, GeradorS};
use engine::rules::tick::{tick, RegrasOriginais};
use engine::rules::vetor_tick::{montar_entrada, VetorTick};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn ler(p: &str) -> String {
    std::fs::read_to_string(raiz().join(p)).unwrap()
}

fn vetor() -> (VetorTick, LayoutSave) {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let v = VetorTick::ler(&raiz().join("tests/referencias/ticks/inseticida_morte/tick_000021"), &layout).unwrap();
    (v, layout)
}

/// A partida no instante de entrada do tick 21, com S, R, `_gameCycles` e o relógio do vetor.
fn partida(v: &VetorTick, layout: &LayoutSave) -> Partida {
    let e = montar_entrada(v, layout).unwrap().estado;
    let nav = Navegacao::do_save(&v.antes, PosicaoVista::Salva, 26, 19).unwrap();
    Partida::nova(
        e,
        nav,
        RegrasOriginais::default(),
        TabelasAmarela::padrao().clone(),
        ConfigSimulacao::de_json(&ler("data/simulacao.json")).unwrap(),
        ConfigPartida::de_json(&ler("data/partida.json")).unwrap(),
        ConfigAvisos::de_json(&engine::recursos::marcar_referencias(&ler("data/mensagens.json"))).unwrap(),
    )
}

#[test]
fn morte_pelo_inseticida_pede_o_fim_de_jogo_uma_vez_com_a_pontuacao_do_original() {
    let (v, layout) = vetor();
    let mut p = partida(&v, &layout);
    // `Partida::rodar_tick` põe o relógio do Mac pelo tempo da partida; aqui vale o lido no original
    // (o gato sorteia conforme o relógio), então o tick roda como em `rodar_tick`, sem essa troca.
    // Tick 21: a morte. A apresentação do tick, na ordem do original: a loucura (_AnimYellowInsane)
    // segura o resto.
    tick(&mut p.estado, &mut p.regras);
    p.processar_eventos_do_estado();
    assert_eq!(p.estado.partida.fim_de_jogo, 1, "_IsGameOver");
    assert_eq!(p.estado.partida.pretas_venceram, 0);
    assert!(p.animando());
    p.avancar(10_000);
    assert!(!p.animando());
    // Como no original: a caixa da morte ("You are covered with insecticide!", 10042, captura
    // dialogo_21_0.png), a de derrota sem corpo (`_PictStrnDialog(0, 10056, 1)` em `_YellowDeath`,
    // `1000:7746`; na bancada fechada pelo Enter logo depois do clique, sem captura) e o GAME OVER
    // (dialogo_21_1.png), pedido pelos ticks seguintes (o segundo pedido fica atrás do quadro).
    assert!(matches!(p.fechar_modal(), Some(Modal::Quadro { texto: 10042, .. })));
    assert!(matches!(p.fechar_modal(), Some(Modal::Quadro { texto: 10056, .. })));
    for _ in 0..2 {
        tick(&mut p.estado, &mut p.regras);
        p.processar_eventos_do_estado();
    }
    match p.avisos.modal() {
        Some(Modal::FimDeJogo { tipo_jogo, pretas_venceram, pontos }) => {
            assert_eq!((*tipo_jogo, *pretas_venceram), (2, false));
            assert_eq!(*pontos, 27572, "pontuação mostrada pelo original no GAME OVER");
        }
        outro => panic!("esperava o quadro de fim de jogo, veio {outro:?}"),
    }
    assert!(matches!(p.fechar_modal(), Some(Modal::FimDeJogo { .. })));
    let eventos = p.retirar_eventos();
    assert_eq!(eventos.iter().filter(|e| **e == Evento::DialogoFimDeJogo).count(), 1, "{eventos:?}");
}

/// Depois do clique no "GAME OVER", o original chama `_NewGame(0)`: a tela "Select a Game" e, com
/// Full Game, um jogo novo. A forma do jogo novo é a do `estado_depois` do vetor (o original o começou
/// dentro do tick): patch (11, 8), rainha alada amarela na superfície, sem ninho, colônia vermelha.
/// Precisa das tabelas do jogo (SIMANT_DIR); sem elas, avisa e passa.
#[test]
fn jogo_completo_novo_tem_a_forma_do_que_o_original_comecou() {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
        return;
    }
    let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    let tab = TabelasGeracao::carregar(&r).unwrap();
    let cfg = ConfigSimulacao::de_json(&ler("data/simulacao.json")).unwrap();
    let (v, _) = vetor();
    let original = Estado::de_save(&v.depois).unwrap();
    let (mut s, mut r) = (GeradorS::new(0), GeradorR::default());
    r.semear_com_ticks(&mut s, 1234, 1234);
    let m = Mundo::novo_jogo(TipoJogo::Completo, &mut r, &mut s, &tab).unwrap();
    let novo = Estado::de_mundo(&m, s, r, &cfg).unwrap();
    for e in [&original, &novo] {
        assert_eq!(e.partida.tipo_jogo, 2);
        assert_eq!(e.ambiente.patch, (11, 8));
        assert_eq!((e.amarela.tipo, e.amarela.plano, e.amarela.ninho_iniciado), (0x40, 1, 0));
        assert_eq!(e.partida.fim_de_jogo, 0);
        assert!(e.vermelha.populacao > 0, "colônia vermelha no patch inicial");
    }
}

/// O caminho inteiro pela interface: `Partida::fechar_modal` devolve o quadro de fim de jogo, que
/// abre o "Select a Game" (`Modal::abre_selecao_de_jogo`, `_NewGame(0)`); com Full Game, a interface
/// cria o jogo novo por `carregar::estado_de_jogo_novo` (o mesmo que simant/src/sessao.rs usa).
#[test]
fn fim_de_jogo_volta_a_selecao_e_comeca_jogo_completo() {
    use engine::core::carregar::estado_de_jogo_novo;
    use engine::core::selecao::Escolha;
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
        return;
    }
    let (v, layout) = vetor();
    let mut p = partida(&v, &layout);
    for _ in 0..3 {
        tick(&mut p.estado, &mut p.regras);
        p.processar_eventos_do_estado();
    }
    p.avancar(10_000); // a animação da loucura, antes dos quadros
    let mut fechados = Vec::new();
    while let Some(m) = p.fechar_modal() {
        fechados.push(m);
    }
    assert!(fechados.last().is_some_and(Modal::abre_selecao_de_jogo), "{fechados:?}");
    assert!(fechados[..fechados.len() - 1].iter().all(|m| !m.abre_selecao_de_jogo()));
    // "Select a Game" → Full Game.
    let tipo = Escolha::Completo.tipo_jogo().unwrap();
    let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    let tab = TabelasGeracao::carregar(&r).unwrap();
    let cfg = ConfigSimulacao::de_json(&ler("data/simulacao.json")).unwrap();
    let quintal = RegrasOriginais::carregar_sem_banco(&r).unwrap().quintal;
    let novo = estado_de_jogo_novo(tipo, 1234, &tab, &cfg, &quintal).unwrap();
    let original = Estado::de_save(&v.depois).unwrap();
    for e in [&original, &novo] {
        assert_eq!(e.partida.tipo_jogo, 2);
        assert_eq!(e.ambiente.patch, (11, 8));
        assert_eq!((e.amarela.tipo, e.amarela.plano, e.amarela.ninho_iniciado), (0x40, 1, 0));
        assert_eq!(e.partida.fim_de_jogo, 0);
    }
    // O menino e o cachorro postos por _InitSimYard (quintal.md: _BoyX/_BoyY = 0xB4/0x49).
    assert_eq!((novo.menino.x, novo.menino.y), (0xB4, 0x49));
}

/// O vetor regravado: o tick 21 abre as caixas e o "Select a Game" DENTRO do tick e volta com um
/// Jogo Completo novo (GameTime 0), gerado com os geradores do momento (o novo jogo não ressemeia).
#[test]
fn tick_21_volta_com_o_jogo_completo_novo_do_original() {
    use engine::core::carregar::jogo_novo_com_geradores;
    use engine::rules::vetor_tick::{comparar_com, regras_do_vetor};
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
        return;
    }
    let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    let tab = TabelasGeracao::carregar(&r).unwrap();
    let cfg = ConfigSimulacao::de_json(&ler("data/simulacao.json")).unwrap();
    let quintal = RegrasOriginais::carregar_sem_banco(&r).unwrap().quintal;
    let (v, layout) = vetor();
    let mut regras = regras_do_vetor(&v);
    let res = comparar_com(&v, &layout, |e| {
        tick(e, &mut regras);
        assert_eq!(e.partida.fim_de_jogo, 1);
        let anterior = e.clone();
        *e = jogo_novo_com_geradores(TipoJogo::Completo, &anterior, &tab, &cfg, &quintal, (25, 19))?;
        Ok(())
    })
    .unwrap();
    println!("{}", res.placar());
    let falhas: Vec<String> = res.diferentes().map(|b| format!("{}: {:?}", b.simbolo, b.detalhes)).collect();
    for x in &res.extras {
        println!("{} original {} port {}", x.nome, x.original, x.port);
    }
    assert!(falhas.is_empty(), "port difere do original:\n{}", falhas.join("\n"));
}
