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

/// `data/<nome>` com as referências de texto do original marcadas (`recursos::marcar_referencias`):
/// os textos da faixa que vêm do SIMANTW.EXE são conferidos pela referência, não pelo texto
/// (o texto em si: tests/textos_originais.rs, com a instalação).
fn ler(nome: &str) -> String {
    engine::recursos::marcar_referencias(&std::fs::read_to_string(raiz().join("data").join(nome)).unwrap())
}

/// Texto da faixa de `mensagens.json` → `faixa` (já marcado por `ler`).
fn texto_faixa(f: impl Fn(&engine::core::avisos::ConfigFaixa) -> String) -> engine::core::avisos::TextoFaixa {
    engine::core::avisos::TextoFaixa::Literal(f(&ConfigAvisos::de_json(&ler("mensagens.json")).unwrap().faixa))
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
    // QUICK2: _GameSpeed = 1 (normal), não pausado: o timer de 17 ms dá uma WM_TIMER a cada 55 ms e
    // o laço de MYTIMERFUNC roda 1 tick a cada 3 delas: 6 ticks/s, como na bancada.
    let mut p = partida();
    assert_eq!(p.estado.partida.velocidade, 1);
    assert_eq!(p.estado.partida.pausado, 0);
    let antes = p.estado.partida.tempo;
    let mut ticks = 0;
    for _ in 0..100 {
        ticks += p.avancar(33).ticks;
    }
    assert_eq!(ticks, 20, "3,3 s de tempo real");
    assert_eq!(p.estado.partida.tempo, antes + 20);
    assert!(p.estado.ciclos_jogo > 20 * 20, "_gameCycles conta as voltas do laço ocupado");
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
    assert_eq!(p.avisos.texto_da_faixa(agora), Some(&texto_faixa(|f| f.velocidade[2].clone())));
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
    assert_eq!(p.avancar(1 + 55).mensagens, 1);
    p.clicar(x, y + 1, true);
    assert_eq!(p.avancar(55).mensagens, 1);
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
    use engine::core::navegacao::Acao;
    let mut p = partida();
    p.marcador(1, false);
    let agora = p.agora();
    assert_eq!(p.avisos.texto_da_faixa(agora), Some(&texto_faixa(|f| f.marcador_ausente.clone())));
    assert!(p.retirar_eventos().contains(&engine::rules::estado::Evento::Som(1)));
    let vista = (p.nav.plano_mapa, p.nav.vista.x, p.nav.vista.y);
    p.marcador(1, true);
    assert_eq!(p.marcadores[1], Some(vista));
    assert_eq!(p.bitmaps_dos_marcadores(), vec![("ribBM2".to_string(), 14558, 14508)]);
    let agora = p.agora();
    assert_eq!(p.avisos.texto_da_faixa(agora), Some(&texto_faixa(|f| f.marcador_posto.clone())));
    p.navegar(Acao::Superficie);
    assert_eq!(p.nav.plano_mapa, 1);
    p.marcador(1, false);
    // _GotoMapPoint → _SetMapPlaneLocation centraliza uma vez só (jogo.md): o eixo menor pode ficar
    // um passo atrás.
    assert_eq!((p.nav.plano_mapa, p.nav.vista.x), (vista.0, vista.1));
    assert!((p.nav.vista.y - vista.2).abs() <= 1);
}

/// Botões configuráveis: bitmaps e estado de `_SetUserButton`/`_UpdateUserButtons`; o menu de
/// ícones troca a função; o menu da amarela manda `_YellowCommand` e segurar o botão na amarela
/// pede o menu.
#[test]
fn botoes_da_faixa_e_menu_da_amarela() {
    use engine::rules::amarela::RespostaClique;
    let mut p = partida();
    let b = p.estado_dos_botoes();
    // 1G = pausa (5): bitmaps 14473 + 5 / 14423 + 5, desligada; 1A = ir até a amarela (15), sem estado.
    assert_eq!(b[6], ("ribUserDef1G".to_string(), 14478, 14428, Some(false)));
    assert_eq!(b[0].3, None);
    p.botao_usuario(5);
    assert_eq!(p.estado_dos_botoes()[6].3, Some(true));
    p.definir_funcao_do_botao("ribUserDef1A", 11, true);
    assert_eq!(p.funcao_do_botao("ribUserDef1A"), Some(11));
    assert_eq!(p.estado_dos_botoes()[0], ("ribUserDef1A".to_string(), 14484, 14434, Some(false)));
    // Faixa 35: a mesma tabela padrão; com Shift só o botão escolhido muda, sem Shift os dois de
    // mesmo índice (_DoUserButton).
    assert_eq!(p.funcao_do_botao("ribUserDef2A"), Some(15));
    assert_eq!(p.estado_dos_botoes()[8].0, "ribUserDef2A");
    p.definir_funcao_do_botao("ribUserDef2B", 8, false);
    assert_eq!((p.funcao_do_botao("ribUserDef2B"), p.funcao_do_botao("ribUserDef1B")), (Some(8), Some(8)));
    p.definir_funcao_do_botao("ribUserDef2B", 9, true);
    assert_eq!((p.funcao_do_botao("ribUserDef2B"), p.funcao_do_botao("ribUserDef1B")), (Some(9), Some(8)));
    // Segurando o botão na própria amarela: menu; o item "Recruit 5" (código 1) recruta.
    let (x, y) = (p.estado.amarela.x as i32, p.estado.amarela.y as i32);
    p.iniciar_espera_do_clique();
    assert_eq!(p.clicar_depois_da_espera(x, y, true), RespostaClique::MenuAmarela);
    let antes = p.estado.preta.modos[5];
    assert!(p.comando_amarela(1));
    assert!(p.estado.preta.modos[5] >= antes);
    assert!(!p.comando_amarela(0), "Help não faz nada no port");
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

#[test]
fn clique_do_place_tambem_grava_o_patch_escolhido() {
    // No original, o clique que encerra o PLACE também passa por _YardArea: _YMapPnt = patch
    // clicado, dentro ou fora do alcance (bancada: acoes/place e place_fora; telas/quintal.md §8).
    use engine::core::quintal_mapa::{clicar, ConfigQuintalMapa, JanelaQuintal};
    let cfg = ConfigQuintalMapa::de_json(&ler("quintal_mapa.json")).unwrap();
    let ponto = (0..640)
        .flat_map(|x| (0..442).map(move |y| (x, y)))
        .find(|&(x, y)| cfg.patch_em(x, y) == Some((5, 8)))
        .unwrap();
    let mut p = partida();
    let mut j = JanelaQuintal { aberta: true, na_frente: true, colocando_rainha: true, ..Default::default() };
    clicar(&mut j, &cfg, &mut p, ponto.0, ponto.1, false, false).unwrap();
    assert!(!j.colocando_rainha);
    assert_eq!(p.estado.vistas.ponto_quintal, (5, 8));
    assert_eq!(p.nav.pontos[0], (5, 8));
}

/// `_PlaceQueenInYard`: a simulação para até o clique; com Shift o laço continua enquanto houver
/// rainhas guardadas; fora do alcance: "That location is out of bounds" (1010[10]).
#[test]
fn place_para_a_simulacao_e_shift_repete() {
    use engine::core::avisos::TextoFaixa;
    use engine::core::quintal_mapa::{clicar, AcaoQuintal, ConfigQuintalMapa, JanelaQuintal};
    let cfg = ConfigQuintalMapa::de_json(&ler("quintal_mapa.json")).unwrap();
    let ponto = |alvo| (0..640).flat_map(|x| (0..442).map(move |y| (x, y))).find(|&(x, y)| cfg.patch_em(x, y) == Some(alvo)).unwrap();
    let mut p = partida();
    p.estado.preta.rainhas_guardadas = 2;
    let mut j = JanelaQuintal::default();
    j.aplicar(AcaoQuintal::ColocarRainha, &mut p.estado);
    p.parada_pela_interface = j.colocando_rainha;
    p.processar_eventos_do_estado();
    assert!(j.colocando_rainha);
    let lista = |i| Some(TextoFaixa::Lista { lista: 1010, indice: i });
    assert_eq!(p.avisos.texto_vigente(p.agora()).cloned(), lista(8));
    let tempo = p.estado.partida.tempo;
    p.avancar(2000);
    assert_eq!(p.estado.partida.tempo, tempo, "a simulação para durante o PLACE");
    // Fora do alcance (patch atual 11, 8; (0, 0) a mais de 3) com Shift: continua.
    let (x, y) = ponto((0, 0));
    clicar(&mut j, &cfg, &mut p, x, y, false, true).unwrap();
    assert!(j.colocando_rainha);
    let (x, y) = ponto((10, 8));
    clicar(&mut j, &cfg, &mut p, x, y, false, true).unwrap();
    assert!(j.colocando_rainha, "com Shift e outra rainha guardada, continua");
    assert_eq!(p.estado.preta.rainhas_guardadas, 1);
    clicar(&mut j, &cfg, &mut p, x, y, false, false).unwrap();
    assert!(!j.colocando_rainha && !p.parada_pela_interface);
    assert_eq!(p.estado.preta.rainhas_guardadas, 0);
    assert_eq!(p.avisos.texto_vigente(p.agora()).cloned(), lista(9), "colônia nova");
    p.avancar(2000);
    assert!(p.estado.partida.tempo > tempo);
}

/// `_SpecialXfer` ao vivo: o tick pede a escolha; fechado o quadro 10058, o quintal fica na Strategic
/// View com "Click on a black colony to continue" e a simulação parada; um patch sem colônia preta é
/// recusado; o de uma colônia troca de patch.
#[test]
fn transferencia_especial_ao_vivo() {
    use engine::core::quintal_mapa::{clicar, iniciar_escolha_de_colonia, ConfigQuintalMapa, JanelaQuintal, RespostaQuintal};
    use engine::rules::troca_patch::ResultadoColonia;
    let cfg = ConfigQuintalMapa::de_json(&ler("quintal_mapa.json")).unwrap();
    let ponto = |alvo| (0..640).flat_map(|x| (0..442).map(move |y| (x, y))).find(|&(x, y)| cfg.patch_em(x, y) == Some(alvo)).unwrap();
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
        return;
    }
    let r = engine::recursos::Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    let mut p = partida();
    p.regras.geracao = Some(engine::rules::geracao::TabelasGeracao::carregar(&r).unwrap());
    p.estado.partida.tipo_jogo = 2;
    p.estado.preta.populacao_quintal.0[5 * 16 + 8] = 9;
    p.estado.eventos.push(engine::rules::estado::Evento::EscolherColonia);
    p.processar_eventos_do_estado();
    assert!(p.escolher_colonia);
    let mut j = JanelaQuintal::default();
    assert!(iniciar_escolha_de_colonia(&mut j, &mut p));
    assert_eq!(p.estado.vistas.modo_quintal, 2);
    let tempo = p.estado.partida.tempo;
    p.avancar(2000);
    assert_eq!(p.estado.partida.tempo, tempo, "parada durante a escolha");
    let (x, y) = ponto((4, 8));
    let r = clicar(&mut j, &cfg, &mut p, x, y, false, false).unwrap();
    assert_eq!(r, RespostaQuintal::Colonia(ResultadoColonia::SemColonia));
    let (x, y) = ponto((5, 8));
    let r = clicar(&mut j, &cfg, &mut p, x, y, false, false).unwrap();
    assert!(matches!(r, RespostaQuintal::Colonia(ResultadoColonia::Trocou(_))), "{r:?}");
    assert!(!p.escolher_colonia && !j.escolhendo_colonia);
    assert_eq!(p.estado.ambiente.patch, (5, 8));
}

/// `_SpecialXfer` DENTRO do tick ao vivo (telas/quintal.md §8): a partir do vetor `acoes/special_xfer`
/// (a amarela morre sem corpo no Jogo Completo com duas colônias), o tick para na escolha; o clique na
/// colônia roda o tick de novo com a escolha. O resultado é o mesmo de um tick que já sabia a escolha
/// (a troca no ponto do original e o resto do tick no patch novo), e só um tick passa.
#[test]
fn transferencia_especial_dentro_do_tick_ao_vivo() {
    use engine::core::quintal_mapa::{clicar, iniciar_escolha_de_colonia, ConfigQuintalMapa, JanelaQuintal, RespostaQuintal};
    use engine::rules::troca_patch::{ResultadoColonia, TransferenciaNoTick};
    use engine::rules::vetor_tick::{montar_entrada, VetorTick};
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
        return;
    }
    let r = engine::recursos::Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    let geracao = engine::rules::geracao::TabelasGeracao::carregar(&r).unwrap();
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let v = VetorTick::ler(&raiz().join("tests/referencias/acoes/special_xfer"), &layout).unwrap();
    let nova = || {
        let e = montar_entrada(&v, &layout).unwrap().estado;
        let nav = Navegacao::do_save(&v.antes, PosicaoVista::AoCarregar, 25, 19).unwrap();
        let mut p = Partida::nova(
            e,
            nav,
            RegrasOriginais { geracao: Some(geracao.clone()), ..RegrasOriginais::default() },
            TabelasAmarela::padrao().clone(),
            ConfigSimulacao::de_json(&ler("simulacao.json")).unwrap(),
            ConfigPartida::de_json(&ler("partida.json")).unwrap(),
            ConfigAvisos::de_json(&ler("mensagens.json")).unwrap(),
        );
        p.janelas_desenho = Default::default();
        p
    };
    let tempo = nova().estado.partida.tempo;

    // Referência: o tick que já sabe a escolha (10, 8).
    let mut b = nova();
    b.estado.transferencia_no_tick = Some(Box::new(TransferenciaNoTick {
        patch: (10, 8),
        geracao: std::sync::Arc::new(geracao.clone()),
        cfg: ConfigSimulacao::de_json(&ler("simulacao.json")).unwrap(),
        resultado: None,
    }));
    b.rodar_tick();
    let feita = b.estado.transferencia_no_tick.take().and_then(|t| t.resultado);
    assert!(matches!(feita, Some(Ok(_))), "{feita:?}");
    assert_eq!(b.estado.ambiente.patch, (10, 8));

    // Ao vivo: o tick para na escolha.
    let mut a = nova();
    a.rodar_tick();
    assert!(a.escolher_colonia && a.tick_esperando_colonia());
    let mut j = JanelaQuintal::default();
    assert!(iniciar_escolha_de_colonia(&mut j, &mut a) || a.avisos.tem_modal());
    while a.avisos.tem_modal() {
        a.pular_modal();
        a.fechar_modal();
    }
    iniciar_escolha_de_colonia(&mut j, &mut a);
    assert!(j.escolhendo_colonia);
    let cfg = ConfigQuintalMapa::de_json(&ler("quintal_mapa.json")).unwrap();
    let ponto = |alvo| (0..640).flat_map(|x| (0..442).map(move |y| (x, y))).find(|&(x, y)| cfg.patch_em(x, y) == Some(alvo)).unwrap();
    let (x, y) = ponto((10, 8));
    let r = clicar(&mut j, &cfg, &mut a, x, y, false, false).unwrap();
    assert!(matches!(r, RespostaQuintal::Colonia(ResultadoColonia::Trocou(_))), "{r:?}");
    assert!(!a.escolher_colonia && !a.tick_esperando_colonia() && !j.escolhendo_colonia);
    assert_eq!(a.estado.ambiente.patch, (10, 8));
    assert_eq!(a.estado.partida.tempo, tempo + 1, "um tick só");
    a.estado.eventos.clear();
    b.estado.eventos.clear();
    // Ao vivo a vista ainda vai até a amarela depois da troca (decisão do port, `_GotoMyAnt`: plano e canto da vista).
    a.estado.vistas = b.estado.vistas.clone();
    assert!(a.estado == b.estado, "o tick repetido com a escolha é igual ao tick que já a sabia");
}

/// O relógio do Mac da partida começa no maior prazo guardado no save (`_CatDelay` 2669 no QUICK2):
/// o gato é testado já no primeiro tick, como num Windows ligado há mais tempo que a sessão que
/// salvou (quintal.md, "Relógios do port").
#[test]
fn relogio_do_mac_comeca_nos_prazos_do_save() {
    let mut p = partida();
    let atraso = p.estado.bichos.gato_atraso;
    assert_eq!(atraso, 2669);
    p.rodar_tick();
    assert!(p.estado.relogio_mac.unwrap() >= atraso as u32);
}

/// "You have entered the red nest" (1010[4]) só pela passagem entre os ninhos (tile 0x14): a amarela
/// do QUICK2 (ninho preto) recebe a ordem de ir para a célula de baixo, que vira passagem; ao chegar,
/// passa para o ninho vermelho e a faixa mostra a mensagem (mensagens.md §2.3).
#[test]
fn mensagem_do_ninho_vermelho_so_pela_passagem() {
    use engine::core::avisos::TextoFaixa;
    let mut p = partida();
    assert_eq!(p.estado.amarela.plano, 2);
    p.estado.partida.opcoes[4] = 1;
    let (x, y) = (p.estado.amarela.x as i32, p.estado.amarela.y as i32);
    p.estado.ninho_b.mapa.definir(x, y + 1, 0x14);
    p.clicar(x, y + 1, true);
    let mut n = 0;
    while p.estado.amarela.plano == 2 && n < 200 {
        p.rodar_tick();
        n += 1;
    }
    assert_eq!(p.estado.amarela.plano, 3, "atravessou");
    let agora = p.agora();
    assert_eq!(p.avisos.texto_da_faixa(agora), Some(&TextoFaixa::Lista { lista: 1010, indice: 4 }));
}
