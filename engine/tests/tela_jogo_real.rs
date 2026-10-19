//! Tela de jogo do `QUICK2.ANT` (faixa + janela de edição com chão e vida). Especificação:
//! docs/spec/telas/jogo.md. O save está no repositório; os bancos do jogo vêm de SIMANT_DIR (ou
//! ../SimAnt-T/MAXIS/ANTWIN) e, sem eles, os testes que desenham avisam e passam sem verificar nada.
//!
//! O hash foi gravado depois de comparar a saída com re_trabalho/qi_14_salvo.png (exemplo
//! `tela_jogo`): 100% dos pixels do mapa visíveis fora do diálogo iguais, 99,33% da janela de edição.

use std::path::PathBuf;

use engine::core::mapa::Plano;
use engine::core::tela_jogo::{compor_tela_de_jogo, vista_do_save, ConfigTelaJogo, PosicaoVista};
use engine::core::telas::Interface;
use engine::core::vida::{reconstruir, AMARELA};
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::recursos::Recursos;
use sha2::{Digest, Sha256};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn quick2() -> JogoSalvo {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    JogoSalvo::abrir(&layout, &raiz().join("tests/referencias/saves/QUICK2.ANT")).unwrap()
}

fn recursos() -> Option<Recursos> {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
        return None;
    }
    let bancos = ["SHARED", "WINGANT", "SOUND"].map(String::from);
    Some(Recursos::abrir(&jogo, &bancos, &raiz().join("data"), None).unwrap())
}

#[test]
fn vida_do_quick2_reconstruida_das_listas() {
    let r = reconstruir(&quick2()).unwrap();
    assert!(r.pendencias.is_empty(), "{:?}", r.pendencias);
    let c = &r.camadas;
    // Lista A: operária preta (T = 17) e vermelha (T = 150).
    assert_eq!(c.valor(Plano::Superficie, 59, 31), 17);
    assert_eq!(c.valor(Plano::Superficie, 123, 44), 150);
    // Lista B: a formiga amarela (_MeMode = 0) cobre a rainha preta e um ovo em (34, 9);
    // em (36, 9) há dois ovos e vence o de menor índice.
    assert_eq!(c.valor(Plano::NinhoPreto, 34, 9), AMARELA);
    assert_eq!(c.valor(Plano::NinhoPreto, 35, 9), 110);
    assert_eq!(c.valor(Plano::NinhoPreto, 36, 9), 1);
    // Lista R: rainha vermelha, sua metade e quatro ovos na mesma célula.
    assert_eq!(c.valor(Plano::NinhoVermelho, 41, 11), 226);
    assert_eq!(c.valor(Plano::NinhoVermelho, 40, 11), 234);
    assert_eq!(c.valor(Plano::NinhoVermelho, 39, 11), 129);
    let ocupadas = |v: &[u8]| v.iter().filter(|&&b| b != 0).count();
    assert_eq!((ocupadas(&c.a), ocupadas(&c.b), ocupadas(&c.r)), (2, 3, 3));
}

#[test]
fn vista_do_quick2_salva_e_ao_carregar() {
    let j = quick2();
    let salva = vista_do_save(&j, PosicaoVista::Salva, 26, 19).unwrap();
    assert_eq!((salva.plano, salva.x, salva.y), (Plano::NinhoPreto, 22, 0));
    // Ao carregar, centraliza em (_MeLocX, _MeLocY) = (34, 9): x = 34 − 25/2 = 22 (bancada, jogo.md).
    let carregada = vista_do_save(&j, PosicaoVista::AoCarregar, 25, 19).unwrap();
    assert_eq!((carregada.x, carregada.y), (22, 0));
}

#[test]
fn tela_de_jogo_do_quick2_igual_a_validada() {
    let Some(r) = recursos() else { return };
    let cfg: ConfigTelaJogo = r.dados("tela_jogo.json").unwrap();
    let interface = Interface::carregar(&r, cfg.tabela_posicoes).unwrap();
    let (q, pendentes) = compor_tela_de_jogo(&cfg, &r, &interface, &quick2()).unwrap();
    assert!(pendentes.is_empty(), "{pendentes:?}");
    assert_eq!((q.largura, q.altura), (640, 442));
    let px = |x: usize, y: usize| q.pixels[y * q.largura + x];
    // Contorno da janela de edição em (20, 97) e moldura cinza (índice 8) logo dentro.
    assert_eq!(px(20, 97), 0);
    assert_eq!(px(21, 98), 8);
    // Formiga amarela na célula (34, 9) do ninho, coluna 12 da vista: pixels amarelos (índice 11).
    let (x0, y0) = (24 + 12 * 16, 120 + 9 * 16);
    let amarelos = (0..16).flat_map(|j| (0..16).map(move |i| (x0 + i, y0 + j))).filter(|&(x, y)| px(x, y) == 11).count();
    assert!(amarelos > 0);
    // A barra de rolagem vertical recorta a 26.ª coluna da vista.
    assert_eq!(px(424, 200), 0);
    let hash: String = Sha256::digest(&q.pixels).iter().map(|b| format!("{b:02x}")).collect();
    println!("hash da tela de jogo: {hash}");
    assert_eq!(hash, HASH_TELA_JOGO);
}

#[test]
fn navegacao_do_quick2_ao_carregar() {
    use engine::core::navegacao::{Acao, ConfigNavegacao, Navegacao};
    let texto = std::fs::read_to_string(raiz().join("data/navegacao.json")).unwrap();
    let cfg: ConfigNavegacao = serde_json::from_str(&texto).unwrap();
    assert_eq!(cfg.posicao_inicial, PosicaoVista::AoCarregar);
    let mut n = Navegacao::do_save(&quick2(), PosicaoVista::AoCarregar, 25, 19).unwrap();
    assert_eq!((n.plano_mapa, n.vista.x, n.vista.y), (2, 22, 0));
    // _DoLoadInitializations grava (_MeLocX, _MeLocY) = (34, 9) no ponto do ninho preto.
    assert_eq!(n.pontos[2], (34, 9));
    assert!(n.aplicar(Acao::Superficie));
    assert_eq!(n.vista.plano, Plano::Superficie);
    assert!(n.aplicar(Acao::NinhoVermelho));
    assert_eq!(n.vista.plano, Plano::NinhoVermelho);
}

#[test]
fn botoes_da_faixa_e_outros_planos_do_quick2() {
    use engine::core::navegacao::{Acao, ConfigNavegacao};
    use engine::core::tela_jogo::TelaDeJogo;
    let Some(r) = recursos() else { return };
    let cfg: ConfigTelaJogo = r.dados("tela_jogo.json").unwrap();
    let nav_cfg: ConfigNavegacao = r.dados("navegacao.json").unwrap();
    let interface = Interface::carregar(&r, cfg.tabela_posicoes).unwrap();
    let j = quick2();
    let tela = TelaDeJogo::carregar(&cfg, &r, &interface, &j).unwrap();
    assert_eq!((tela.largura_vista, tela.altura_vista), (25, 19));
    // Cada botão configurado existe na faixa e é encontrado pelo clique no seu centro.
    let id_faixa = interface.moldura.faixa.janela;
    let faixa = engine::core::telas::carregar_janela(&r, id_faixa).unwrap();
    let origem = interface.posicionar(id_faixa, &faixa).unwrap().origem;
    let base = faixa.retangulo();
    for nome in nav_cfg.botoes_faixa.keys() {
        let ob = faixa.objetos.iter().find(|o| &o.nome == nome).unwrap_or_else(|| panic!("{nome} não está na faixa"));
        let x = (ob.retangulo.esq + ob.retangulo.dir) / 2 + origem.0 - base.esq;
        let y = (ob.retangulo.topo + ob.retangulo.base) / 2 + origem.1 - base.topo;
        assert!(tela.objetos_da_faixa_em(x as i32, y as i32).contains(&nome.as_str()), "{nome}");
    }
    // Os três planos desenham sem pendências.
    let mut nav = tela.navegacao(&j, PosicaoVista::AoCarregar).unwrap();
    for a in [Acao::Superficie, Acao::NinhoVermelho, Acao::NinhoPreto] {
        nav.aplicar(a);
        let (q, pendentes) = tela.compor(&r, &interface, &nav).unwrap();
        assert!(pendentes.is_empty(), "{a:?}: {pendentes:?}");
        assert_eq!((q.largura, q.altura), (640, 442));
    }
    assert_eq!(tela.seta_em(430, 125), Some(Acao::LinhaAcima));
    assert_eq!(tela.seta_em(200, 200), None);
}

// Atualizado em 2026-10-18: traço do menu de sistema e seta de maximizar na moldura (medidos em j00).
/// `_ELayerMode` (alarme da amarela, botões de cheiro): na superfície, a camada de feromônio entra no
/// lugar do chão (`_DoCalcTile`). No QUICK2 o `_PherMapBN` só tem a célula (1, 16) = 254, o buraco
/// preto: com a camada 1 mudam exatamente os tiles (2..3, 32..33) da vista.
#[test]
fn camada_de_cheiro_na_janela_de_edicao() {
    use engine::core::navegacao::Acao;
    use engine::core::tela_jogo::TelaDeJogo;
    use engine::rules::estado::Estado;
    let Some(r) = recursos() else { return };
    let cfg: ConfigTelaJogo = r.dados("tela_jogo.json").unwrap();
    let interface = Interface::carregar(&r, cfg.tabela_posicoes).unwrap();
    let j = quick2();
    let mut tela = TelaDeJogo::carregar(&cfg, &r, &interface, &j).unwrap();
    let mut nav = tela.navegacao(&j, PosicaoVista::AoCarregar).unwrap();
    nav.aplicar(Acao::Superficie);
    (nav.vista.x, nav.vista.y) = (0, 23);
    let mut e = Estado::de_save(&j).unwrap();
    tela.atualizar(&e);
    let (sem, _) = tela.compor(&r, &interface, &nav).unwrap();
    e.camada_edicao = 1;
    tela.atualizar(&e);
    let (com, _) = tela.compor(&r, &interface, &nav).unwrap();
    let diferentes: Vec<(usize, usize)> = (0..sem.pixels.len())
        .filter(|&i| sem.pixels[i] != com.pixels[i])
        .map(|i| (i % sem.largura, i / sem.largura))
        .collect();
    assert!(!diferentes.is_empty());
    // Canto da área de tiles (24, 120); célula (2, 32) = coluna 2, linha 9 da vista.
    let (x0, y0) = (24 + 2 * 16, 120 + 9 * 16);
    assert!(diferentes.iter().all(|&(x, y)| (x0..x0 + 32).contains(&x) && (y0..y0 + 32).contains(&y)), "{:?}", &diferentes[..5]);
}

// 2026-10-18: com as barras e os números da faixa (core/barras_faixa.rs), conferidos contra a bancada
// (mensagens.md §9); o resto da tela não mudou.
// 2026-10-18: setas e polegares das barras de rolagem (gerenciador de janelas, janelas.md §8);
// a região da janela de edição dá 99,59% contra j00 (resta o texto do título).
const HASH_TELA_JOGO: &str = "1274ad6e2390bd2b28b41185e66c8d0d39f8d1fb4fdfe150516a6d6fbd3a1f8e";

/// Janelas de controle (janelas.md): o clique no triângulo do Caste Control (na frente) muda
/// `_casteLevels`, passa para Manual e recalcula `_IdealCaste`; Auto e os conjuntos.
#[test]
fn janela_de_castas_muda_os_niveis() {
    use engine::core::janelas::{Controle, JanelasControle};
    use engine::rules::estado::Estado;
    let Some(r) = recursos() else { return };
    let mut e = Estado::de_save(&quick2()).unwrap();
    let cfg: ConfigTelaJogo = r.dados("tela_jogo.json").unwrap();
    let interface = Interface::carregar(&r, cfg.tabela_posicoes).unwrap();
    use engine::core::gerenciador::Clique;
    let area = engine::formats::janela::Retangulo { esq: 0, topo: 73, dir: 640, base: 442 };
    let m = &cfg.edicao.moldura;
    let mut j = JanelasControle::new(r.dados("janelas.json").unwrap(), &interface, &r, (m.retangulo(), m.metricas()), area).unwrap();
    // _SetDefaultWindows: Caste, Behavior, mapa e a de edição por cima.
    assert_eq!(j.janelas_padrao(2), Some(Controle::Mapa));
    assert_eq!(j.gerente.ordem(), vec![19, 18, 1, 0]);
    j.trazer_para_frente(Some(Controle::Castas));
    // Ápice do triângulo (casteTriangle (392, 342) no layout, deslocamento (0, −52); largura 108 no
    // triângulo do clique, que é o do botão): reprodutores 100%.
    assert_eq!(j.clicar(&r, &mut e, 446, 290, false).unwrap(), Clique::Cliente(19));
    assert!(j.soltar());
    assert_eq!(e.controles.castas[0], 0xFFFF);
    assert_eq!(e.controles.casta_automatica, 0);
    assert_eq!(e.controles.castas_ideais, [0, 0, 50, 50]);
    assert_eq!(e.controles.castas_memorizadas[..3], [0xFFFF, 0, 0]);
    // Set 2 (348, 359)–(391, 373): guarda o atual no conjunto 0 e carrega o 1.
    assert_eq!(j.clicar(&r, &mut e, 360, 359 - 52 + 5, false).unwrap(), Clique::Cliente(19));
    assert_eq!(j.conjunto[0], 1);
    assert_eq!(e.controles.castas[..], e.controles.castas_memorizadas[3..6]);
    // Auto (348, 309)–(390, 323).
    assert_eq!(j.clicar(&r, &mut e, 360, 309 - 52 + 5, false).unwrap(), Clique::Cliente(19));
    assert_eq!(e.controles.casta_automatica, 1);
    // Na janela de edição (atrás): ela vem para a frente e o clique é comido.
    assert_eq!(j.clicar(&r, &mut e, 100, 200, false).unwrap(), Clique::Ativou(0));
    assert_eq!(j.ativa(), None);
    // Agora o mesmo ponto do Auto é da edição, que cobre a Caste: o Manual não é apertado.
    assert_eq!(j.clicar(&r, &mut e, 360, 323 - 52 + 5, false).unwrap(), Clique::Cliente(0));
    assert_eq!(e.controles.casta_automatica, 1);
}

/// Arrasto no triângulo das castas conferido na bancada (2026-10-18, `cenarios_tick.py gravar
/// triangulo`, snapshot rico, jogo pausado): botão apertado em (435, 421) da tela, passando por
/// (436, 370) e (460, 400), solto em (420, 395) (área cliente = tela − 38 linhas). O original leu
/// `_casteLevels` = (18321, 39138, 8076), `_CasteAuto` = 0 e `_IdealCaste` = (59, 12, 14, 14).
#[test]
fn arrasto_no_triangulo_igual_a_bancada() {
    use engine::core::janelas::{Controle, JanelasControle};
    use engine::rules::estado::Estado;
    let Some(r) = recursos() else { return };
    let mut e = Estado::de_save(&quick2()).unwrap();
    let cfg: ConfigTelaJogo = r.dados("tela_jogo.json").unwrap();
    let interface = Interface::carregar(&r, cfg.tabela_posicoes).unwrap();
    let area = engine::formats::janela::Retangulo { esq: 0, topo: 73, dir: 640, base: 442 };
    let m = &cfg.edicao.moldura;
    let mut j = JanelasControle::new(r.dados("janelas.json").unwrap(), &interface, &r, (m.retangulo(), m.metricas()), area).unwrap();
    j.janelas_padrao(2);
    j.trazer_para_frente(Some(Controle::Castas));
    j.clicar(&r, &mut e, 435, 421 - 38, false).unwrap();
    for (x, y) in [(436, 370), (460, 400), (420, 395)] {
        j.mover(&r, &mut e, x, y - 38).unwrap();
    }
    j.soltar();
    assert_eq!(e.controles.castas, [18321, 39138, 8076]);
    assert_eq!(e.controles.casta_automatica, 0);
    assert_eq!(e.controles.castas_ideais, [59, 12, 14, 14]);
}

/// Navegação conferida na bancada (2026-10-18, `cenarios_tick.py gravar navegacao`, snapshot rico,
/// jogo pausado): o estado lido depois de cada tecla. Uma Ctrl+seta mandada pela bancada fica
/// apertada ~150 ms e o laço de `MAINWNDPROC` repete o passo enquanto ela está apertada, por isso o
/// número de passos por tecla variou de 1 a 3 (o de cada vista vem do `_MapPnt` lido).
#[test]
fn navegacao_igual_a_bancada() {
    use engine::core::navegacao::{Acao, Navegacao};
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let j = JogoSalvo::abrir(&layout, &raiz().join("tests/referencias/saves/NAV_RICO.ANT")).unwrap();
    let mut n = Navegacao::do_save(&j, PosicaoVista::Salva, 25, 19).unwrap();
    assert_eq!((n.plano_mapa, n.vista.x, n.vista.y), (1, 52, 23));
    let pontos = n.pontos;
    let passos: [(Acao, usize, (i16, i32, i32)); 11] = [
        (Acao::LinhaDireita, 1, (1, 53, 23)),
        (Acao::LinhaDireita, 2, (1, 55, 23)),
        (Acao::LinhaAbaixo, 1, (1, 55, 24)),
        (Acao::LinhaEsquerda, 1, (1, 54, 24)),
        (Acao::LinhaAcima, 1, (1, 54, 23)),
        // troca de plano: _CenterEdit no ponto lembrado (ninho preto (36, 9), vermelho (32, 1))
        (Acao::NinhoPreto, 1, (2, 24, 0)),
        (Acao::LinhaDireita, 3, (2, 27, 0)),
        (Acao::NinhoVermelho, 1, (3, 20, 0)),
        (Acao::Superficie, 1, (1, 52, 23)),
        (Acao::LinhaAbaixo, 5, (1, 52, 28)),
        // o plano que já está na tela também centraliza: a rolagem não muda o ponto lembrado
        (Acao::Superficie, 1, (1, 52, 23)),
    ];
    for (k, (a, vezes, esperado)) in passos.into_iter().enumerate() {
        for _ in 0..vezes {
            n.aplicar(a);
        }
        assert_eq!((n.plano_mapa, n.vista.x, n.vista.y), esperado, "passo {k} ({a:?})");
    }
    assert_eq!(n.pontos, pontos, "a rolagem e a troca de plano não mudam os pontos lembrados");
}
