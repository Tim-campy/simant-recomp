//! Laço de jogo (`core::partida`) sobre o `QUICK2.ANT`: regressão de "roda sem quebrar".
//! Especificação: docs/spec/telas/jogo.md, seção "Simulação ao vivo".

use std::path::PathBuf;

use engine::core::avisos::ConfigAvisos;
use engine::core::navegacao::Navegacao;
use engine::core::partida::{ConfigPartida, Partida};
use engine::core::tela_jogo::PosicaoVista;
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::rules::amarela::TabelasAmarela;
use engine::rules::estado::{ConfigSimulacao, Estado};
use engine::rules::tick::RegrasOriginais;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn ler(nome: &str) -> String {
    std::fs::read_to_string(raiz().join("data").join(nome)).unwrap()
}

fn partida() -> Partida {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let jogo = JogoSalvo::abrir(&layout, &raiz().join("tests/referencias/saves/QUICK2.ANT")).unwrap();
    let estado = Estado::de_save(&jogo).unwrap();
    let nav = Navegacao::do_save(&jogo, PosicaoVista::AoCarregar, 25, 19).unwrap();
    Partida::nova(
        estado,
        nav,
        RegrasOriginais::default(),
        TabelasAmarela::padrao().clone(),
        ConfigSimulacao::de_json(&ler("simulacao.json")).unwrap(),
        ConfigPartida::de_json(&ler("partida.json")).unwrap(),
        ConfigAvisos::de_json(&ler("mensagens.json")).unwrap(),
    )
}

#[test]
fn cinquenta_ticks_do_quick2() {
    let mut p = partida();
    let antes = p.estado.partida.tempo;
    for _ in 0..50 {
        p.rodar_tick();
    }
    assert_eq!(p.estado.partida.tempo, antes + 50);
    // A vista continua no ninho preto, dentro do mapa.
    assert_eq!(p.nav.plano_mapa, 2);
    assert!(p.nav.vista.x >= 0 && p.nav.vista.x + p.nav.vista.largura <= 64);
}

#[test]
fn relogio_em_tempo_real_no_quick2() {
    // QUICK2: _GameSpeed = 1 (normal), não pausado: 1 tick a cada 2 mensagens de 17 ms.
    let mut p = partida();
    assert_eq!(p.estado.partida.velocidade, 1);
    assert_eq!(p.estado.partida.pausado, 0);
    let antes = p.estado.partida.tempo;
    let mut ticks = 0;
    for _ in 0..100 {
        ticks += p.avancar(17).ticks;
    }
    assert_eq!(ticks, 50);
    assert_eq!(p.estado.partida.tempo, antes + 50);
    assert_eq!(p.estado.ciclos_jogo, 100, "_gameCycles conta toda mensagem");
}

/// Salvar (`Estado::para_save`, o que Ctrl+S grava) e recarregar dá o mesmo estado, depois de 50
/// ticks de simulação ao vivo (o save do port também foi carregado pelo original: mensagens.md).
#[test]
fn salvar_e_recarregar_da_o_mesmo_estado() {
    let mut p = partida();
    for _ in 0..50 {
        p.rodar_tick();
    }
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let salvo = p.estado.para_save(&layout).unwrap();
    let dir = std::env::temp_dir().join(format!("simant_salvar_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let arq = dir.join("TESTE.ANT");
    salvo.gravar(&arq).unwrap();
    let relido = JogoSalvo::abrir(&layout, &arq).unwrap();
    assert_eq!(relido.bytes(), salvo.bytes());
    assert_eq!(relido.bytes().len(), 48_386);
    let e2 = Estado::de_save(&relido).unwrap();
    assert_eq!(e2.para_save(&layout).unwrap().bytes(), salvo.bytes(), "ida e volta byte a byte");
    // Os campos do save batem; o que não está no save (geradores, ciclos do laço) não é comparado.
    assert_eq!(e2.partida, p.estado.partida);
    assert_eq!(e2.amarela, p.estado.amarela);
    assert_eq!(e2.superficie.lista, p.estado.superficie.lista);
    assert_eq!(e2.ninho_b.vida, p.estado.ninho_b.vida);
    assert_eq!(e2.vistas, p.estado.vistas);
    std::fs::remove_dir_all(&dir).ok();
}

/// QUICK2 sem a cabeça da rainha preta (como o caso C-10): no tick 128 saem os quadros 10010 e
/// 10011 e o fim de jogo; a simulação para no tick em que o primeiro aparece e só volta quando os
/// três avisos são fechados.
#[test]
fn quadros_param_a_simulacao_ate_fechar() {
    use engine::core::avisos::Modal;
    let mut p = partida();
    p.estado.ninho_b.lista.vagas[0].tipo = 0;
    let mut n = 0;
    while !p.avisos.tem_modal() && n < 2000 {
        p.avancar(17);
        n += 1;
    }
    let tempo = p.estado.partida.tempo;
    assert_eq!(tempo, 128);
    assert_eq!(p.avisos.modal(), Some(&Modal::Quadro { texto: 10010, imagem: None }));
    // Parada: 5 s de tempo real sem tick.
    for _ in 0..300 {
        assert_eq!(p.avancar(17).ticks, 0);
    }
    assert_eq!(p.estado.partida.tempo, tempo);
    // _DialogWaitInit(0xf): o quadro fecha sozinho depois de 270 × 55 ms.
    for _ in 0..600 {
        p.avancar(17);
    }
    assert_eq!(p.avisos.modal(), Some(&Modal::Quadro { texto: 10011, imagem: None }));
    p.fechar_modal();
    assert!(matches!(p.avisos.modal(), Some(Modal::FimDeJogo { tipo_jogo: 1, pretas_venceram: false, .. })));
    for _ in 0..2000 {
        p.avancar(17);
    }
    assert!(p.avisos.tem_modal(), "o fim de jogo não tem prazo");
    assert_eq!(p.estado.partida.tempo, tempo);
}

/// Botão de velocidade (`_DoUserButton` caso 6): cicla e põe "Game speed now …" na faixa; o clique
/// simples no mapa para a simulação por 6 × 55 ms.
#[test]
fn botao_de_velocidade_e_espera_do_clique() {
    use engine::core::avisos::TextoFaixa;
    use engine::core::partida::RespostaBotao;
    let mut p = partida();
    assert_eq!(p.cfg.funcao_do_botao("ribUserDef1F"), Some(6));
    assert_eq!(p.botao_usuario(6), RespostaBotao::Feito);
    assert_eq!(p.estado.partida.velocidade, 2);
    let agora = p.agora();
    assert_eq!(p.avisos.texto_da_faixa(agora), Some(&TextoFaixa::Literal("Game speed now fast.".into())));
    for _ in 0..2 {
        p.botao_usuario(6);
    }
    assert_eq!(p.estado.partida.velocidade, 0, "ultra → slow");
    assert_eq!(p.botao_usuario(14), RespostaBotao::Salvar);
    // Pausa pelo botão 5: "Game Paused" (lista 1010, índice 2), permanente.
    p.botao_usuario(5);
    assert_eq!(p.estado.partida.pausado, 1);
    let agora = p.agora();
    assert_eq!(p.avisos.texto_da_faixa(agora), Some(&TextoFaixa::Lista { lista: 1010, indice: 2 }));
    p.botao_usuario(5);
    p.aplicar(engine::core::partida::AcaoPartida::VelocidadeNormal);
    // Espera do clique simples: 330 ms sem mensagens de temporizador; o duplo não espera.
    let (x, y) = (p.estado.amarela.x as i32, p.estado.amarela.y as i32);
    p.clicar(x, y + 1, false);
    assert_eq!(p.avancar(329).mensagens, 0);
    assert_eq!(p.avancar(1 + 17).mensagens, 1);
    p.clicar(x, y + 1, true);
    assert_eq!(p.avancar(17).mensagens, 1);
}

/// File → Load (`_DoLoadInitializations`): `_FullCount` já detecta a rainha que sumiu (C-10 na
/// bancada: o quadro 10010 aparece logo no Load, `GameTime` 78) e o jogo fica pausado depois que os
/// quadros fecham.
#[test]
fn load_conta_as_formigas_e_pausa_depois_dos_quadros() {
    use engine::core::avisos::{Modal, TextoFaixa};
    let mut p = partida();
    p.estado.ninho_b.lista.vagas[0].tipo = 0;
    p.ao_carregar();
    assert_eq!(p.estado.partida.tempo, 78);
    assert_eq!(p.avisos.modal(), Some(&Modal::Quadro { texto: 10010, imagem: None }));
    assert_eq!(p.estado.partida.pausado, 0, "a pausa espera os quadros");
    let agora = p.agora();
    assert_eq!(p.avisos.texto_da_faixa(agora), None);
    p.fechar_modal();
    assert_eq!(p.avisos.modal(), Some(&Modal::Quadro { texto: 10011, imagem: None }));
    p.fechar_modal();
    assert!(!p.avisos.tem_modal());
    assert_eq!(p.estado.partida.pausado, 1);
    let agora = p.agora();
    assert_eq!(p.avisos.texto_da_faixa(agora), Some(&TextoFaixa::Lista { lista: 1010, indice: 2 }));
    // QUICK2 sem mudança: só recontagem (as castas do save batem com a bancada) e a pausa.
    let mut q = partida();
    q.ao_carregar();
    assert!(!q.avisos.tem_modal());
    assert_eq!(q.estado.partida.pausado, 1);
    assert_eq!(q.estado.preta.castas, [3, 2, 0, 0, 0, 1], "_CastePopB recontado no Load (bancada-qemu.md)");
}

/// `_DoBookMark`: Ctrl+clique põe o marcador (vista atual), clique volta a ele; vazio avisa.
#[test]
fn marcadores_da_faixa() {
    use engine::core::avisos::TextoFaixa;
    use engine::core::navegacao::Acao;
    let mut p = partida();
    p.marcador(1, false);
    let agora = p.agora();
    assert_eq!(p.avisos.texto_da_faixa(agora), Some(&TextoFaixa::Literal("Bookmark has not been placed.".into())));
    assert!(p.retirar_eventos().contains(&engine::rules::estado::Evento::Som(1)));
    let vista = (p.nav.plano_mapa, p.nav.vista.x, p.nav.vista.y);
    p.marcador(1, true);
    assert_eq!(p.marcadores[1], Some(vista));
    assert_eq!(p.bitmaps_dos_marcadores(), vec![("ribBM2".to_string(), 14558, 14508)]);
    let agora = p.agora();
    assert_eq!(p.avisos.texto_da_faixa(agora), Some(&TextoFaixa::Literal("Bookmark has been placed.".into())));
    p.navegar(Acao::Superficie);
    assert_eq!(p.nav.plano_mapa, 1);
    p.marcador(1, false);
    // _GotoMapPoint → _SetMapPlaneLocation centraliza uma vez só (jogo.md): o eixo menor pode ficar
    // um passo atrás.
    assert_eq!((p.nav.plano_mapa, p.nav.vista.x), (vista.0, vista.1));
    assert!((p.nav.vista.y - vista.2).abs() <= 1);
}

#[test]
fn ordem_de_andar_pelo_clique() {
    // Clique simples numa célula vizinha da amarela dá a ordem de ir (`_MeMoveMe` = 1).
    let mut p = partida();
    let (x, y) = (p.estado.amarela.x as i32, p.estado.amarela.y as i32);
    p.clicar(x, y + 1, false);
    assert_eq!(p.estado.amarela.mover, 1);
    assert_eq!((p.estado.amarela.alvo_x as i32, p.estado.amarela.alvo_y as i32), (x, y + 1));
}
