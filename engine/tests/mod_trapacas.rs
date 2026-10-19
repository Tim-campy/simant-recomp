//! O mod de trapaças de exemplo (`exemplos/mods/trapacas`, API 0.17) carregado numa partida: cada
//! item do menu muda o estado do jeito esperado, e sem usar o menu nada muda.

use std::fs;
use std::path::PathBuf;

use engine::core::avisos::ConfigAvisos;
use engine::core::navegacao::Navegacao;
use engine::core::partida::{ConfigPartida, Partida};
use engine::core::tela_jogo::PosicaoVista;
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::rules::amarela::TabelasAmarela;
use engine::rules::casa::TabelasCasa;
use engine::rules::estado::{ConfigSimulacao, Estado, Evento};
use engine::rules::experimental::TabelasExperimental;
use engine::rules::geracao::{Mundo, TabelasGeracao, TipoJogo};
use engine::rules::rng::{GeradorR, GeradorS};
use engine::rules::tick::RegrasOriginais;
use engine::rules::troca_patch::{self, ResultadoRainha};
use engine::scripts::{Limites, Scripts};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// `data/<nome>` com as referências de texto do original marcadas (o teste não depende delas).
fn ler(nome: &str) -> String {
    engine::recursos::marcar_referencias(&fs::read_to_string(raiz().join("data").join(nome)).unwrap())
}

fn mod_trapacas() -> Scripts {
    let codigo = fs::read_to_string(raiz().join("exemplos/mods/trapacas/trapacas.lua")).unwrap();
    Scripts::de_fontes(
        vec![("trapacas".to_string(), Ok(vec![("trapacas.lua".to_string(), codigo)]))],
        TabelasExperimental::padrao().clone(),
        Limites::default(),
    )
}

fn partida_sem_mod() -> Partida {
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

/// QUICK2.ANT (Jogo Rápido) com o mod.
fn partida() -> Partida {
    let mut p = partida_sem_mod();
    p.ligar_scripts(mod_trapacas());
    p
}

fn tabelas_geracao() -> TabelasGeracao {
    let seno: Vec<u8> = (0..64)
        .flat_map(|i| ((((i as f64) * std::f64::consts::PI / 128.0).sin() * 32767.0).round() as i16).to_be_bytes())
        .collect();
    let mut t = TabelasGeracao::de_partes(&ler("geracao.json"), &seno).unwrap();
    t.casa = TabelasCasa::de_json(&ler("casa.json")).unwrap();
    t
}

/// Um Jogo Completo novo gerado pelo port, com o mod e as tabelas da geração (troca de patch).
fn partida_completa() -> Partida {
    let mut p = partida_sem_mod();
    let t = tabelas_geracao();
    let (mut s, mut r) = (GeradorS::new(0), GeradorR::default());
    r.semear_com_ticks(&mut s, 4321, 4321);
    let m = Mundo::novo_jogo(TipoJogo::Completo, &mut r, &mut s, &t).unwrap();
    p.estado = Estado::de_mundo(&m, s, r, &p.cfg_simulacao).unwrap();
    p.regras.geracao = Some(t);
    p.ligar_scripts(mod_trapacas());
    p
}

/// Escolhe o item do menu do mod pelo começo do texto.
fn usar(p: &mut Partida, texto: &str) {
    let menus = p.scripts.as_ref().unwrap().menus();
    let (_, menu) = menus.iter().find(|(n, _)| n == "trapacas").expect("menu do mod");
    let i = menu.itens.iter().position(|t| t.starts_with(texto)).unwrap_or_else(|| panic!("item {texto}")) + 1;
    p.escolher_no_menu_de_mod("trapacas", i);
}

fn rodar(p: &mut Partida, n: u32) {
    for _ in 0..n {
        p.rodar_tick();
    }
}

fn recusas(p: &mut Partida) -> Vec<String> {
    p.retirar_registro_scripts().into_iter().filter(|l| l.contains("recusado")).collect()
}

#[test]
fn menu_em_f12_e_sem_usar_nada_muda() {
    let mut com = partida();
    let menus = com.scripts.as_ref().unwrap().menus();
    assert_eq!(menus[0].1.tecla.texto(), "F12");
    assert!(menus[0].1.itens.len() >= 25);
    let mut sem = partida_sem_mod();
    rodar(&mut com, 300);
    rodar(&mut sem, 300);
    assert!(com.estado == sem.estado, "o mod só lê o estado nos ganchos de tick");
}

#[test]
fn chuva_e_agua_nos_ninhos() {
    let mut p = partida();
    let lencol = p.estado.ambiente.lencol;
    usar(&mut p, "Começar chuva");
    assert_eq!(p.estado.ambiente.chovendo, 1);
    rodar(&mut p, 120);
    assert!(p.estado.ambiente.lencol < lencol, "_WaterTable sobe");
    usar(&mut p, "Parar chuva");
    rodar(&mut p, 1);
    assert_eq!(p.estado.ambiente.chovendo, 0);
    assert!(recusas(&mut p).is_empty());
}

#[test]
fn trapaca_aparece_na_faixa() {
    let mut p = partida();
    usar(&mut p, "Gato aparecer");
    let agora = p.agora();
    let faixa = p.avisos.texto_da_faixa(agora).map(|t| format!("{t:?}")).unwrap_or_default();
    assert!(faixa.contains("Trapaça: Gato aparecer"), "{faixa}");
}

#[test]
fn menino_gato_cachorro_aranha() {
    let mut p = partida();
    p.estado.bichos.gato_ligado = 0;
    p.estado.aranha.ligada = 0;
    usar(&mut p, "Menino: cortar a grama");
    assert_eq!((p.estado.menino.presente, p.estado.menino.cortando_grama), (3, 1));
    rodar(&mut p, 5);
    assert_eq!(p.estado.menino.cortando_grama, 1, "continua cortando no tick");
    usar(&mut p, "Menino: inseticida");
    assert_eq!(p.estado.menino.no_numero, 32);
    usar(&mut p, "Gato aparecer");
    assert_eq!(p.estado.bichos.gato_ligado, 1);
    usar(&mut p, "Aranha aparecer");
    assert_eq!(p.estado.aranha.ligada, 1);
    p.retirar_eventos();
    usar(&mut p, "Cachorro latir");
    assert!(p.retirar_eventos().iter().any(|e| matches!(e, Evento::Som(_))));
}

#[test]
fn guardar_rainha_habilita_o_place() {
    let mut p = partida();
    let patch = p.estado.ambiente.patch;
    assert_eq!(troca_patch::colocar_rainha(&mut p.estado, patch.0, patch.1), ResultadoRainha::SemRainhas);
    usar(&mut p, "Guardar 3 rainhas");
    assert_eq!(p.estado.preta.rainhas_guardadas, 3);
    assert_eq!(troca_patch::colocar_rainha(&mut p.estado, patch.0, patch.1), ResultadoRainha::Colocada { restam: 2 });
}

#[test]
fn revoada_vermelha_decola() {
    let mut p = partida();
    usar(&mut p, "Revoada vermelha");
    assert_eq!(p.estado.vermelha.revoada, 200);
    let mut voando = false;
    for _ in 0..200 {
        p.rodar_tick();
        voando |= p.estado.superficie.lista.ativas().any(|(_, f)| f.tipo & 0xF8 == 0xA0 && f.modo >= 15);
    }
    assert!(voando, "machos em decolagem");
}

#[test]
fn amarela_tempo_fim_e_misterio() {
    let mut p = partida();
    usar(&mut p, "Curar a amarela");
    assert_eq!(p.estado.amarela.saude, 100);
    usar(&mut p, "Comida em volta da amarela");
    let t0 = p.estado.partida.tempo;
    usar(&mut p, "Avançar 100 ticks");
    while p.ticks_de_mods_pendentes() > 0 {
        p.avancar(0);
    }
    assert_eq!(p.estado.partida.tempo, t0 + 100);
    usar(&mut p, "Mystery");
    assert!(p.ultimo_misterio.is_some());
    usar(&mut p, "Tutorial: avançar lição");
    assert!(recusas(&mut p).iter().any(|l| l.contains("só no tutorial")));
    usar(&mut p, "Fim de jogo: derrota");
    assert_eq!((p.estado.partida.fim_de_jogo, p.estado.partida.pretas_venceram), (1, 0));
}

#[test]
fn pular_ate_o_proximo_evento_para_no_evento() {
    let mut p = partida();
    usar(&mut p, "Pular até o próximo evento");
    assert_eq!(p.ticks_de_mods_pendentes(), 5000);
    let t0 = p.estado.partida.tempo;
    while p.ticks_de_mods_pendentes() > 0 {
        p.avancar(0);
    }
    let t = p.estado.partida.tempo;
    assert!(t > t0 && t < t0 + 5000, "parou num evento (tick {t})");
}

#[test]
fn painel_de_depuracao() {
    let mut p = partida();
    usar(&mut p, "Painel de depuração");
    let paineis = p.scripts.as_ref().unwrap().paineis();
    assert_eq!(paineis.len(), 1);
    assert!(paineis[0].1[0].starts_with("GameTime"));
    assert!(paineis[0].1.iter().any(|l| l.contains("_WaterTable")));
    usar(&mut p, "Painel de depuração");
    assert!(p.scripts.as_ref().unwrap().paineis().is_empty());
}

#[test]
fn luta_e_formiga_leao_na_superficie() {
    let mut p = partida();
    // Leva a amarela para a superfície, numa área livre (o teste não depende de onde o save a deixou).
    let (x, y) = (40, 30);
    p.estado.amarela.plano = 1;
    p.estado.amarela.x = x;
    p.estado.amarela.y = y;
    let n0 = p.estado.superficie.lista.indice;
    usar(&mut p, "Luta: soldado vermelho");
    assert!(p.estado.superficie.lista.indice > n0, "soldados vermelhos ao lado da amarela");
    let leoes = p.estado.formigas_leao.quantidade;
    usar(&mut p, "Formiga-leão perto");
    assert!(p.estado.formigas_leao.quantidade == leoes + 1 || !recusas(&mut p).is_empty());
}

/// API 0.26: castas, comportamento, alarme e soldados no ninho; a contagem de trapaças vai para o
/// arquivo lateral do save e volta no Load.
#[test]
fn colonia_e_estado_no_save() {
    let mut p = partida();
    usar(&mut p, "Castas: só soldados");
    assert_eq!(p.estado.controles.castas, [0, 0, 65535]);
    usar(&mut p, "Comportamento: só forragear");
    assert_eq!(p.estado.controles.tarefas, [65535, 0, 0]);
    let n0 = p.estado.ninho_b.lista.ativas().count();
    usar(&mut p, "5 soldados pretos");
    assert_eq!(p.estado.ninho_b.lista.ativas().count(), n0 + 5);
    p.estado.amarela.plano = 1;
    p.estado.amarela.x = 40;
    p.estado.amarela.y = 30;
    usar(&mut p, "Alarme em volta");
    assert!(p.estado.feromonios.alarme.em(20, 15).is_some_and(|v| v > 0));

    let dir = std::env::temp_dir().join(format!("simant_trapacas_save_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let save = dir.join("TRAPACA.ANT");
    p.gravar_estado_dos_mods(&save).unwrap();
    let mut q = partida();
    q.estado.partida.tempo = p.estado.partida.tempo;
    q.ler_estado_dos_mods(&save);
    let agora = q.agora();
    let faixa = q.avisos.texto_da_faixa(agora).map(|t| format!("{t:?}")).unwrap_or_default();
    assert!(faixa.contains("já teve 4 trapaça"), "{faixa}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn patches_da_casa_no_jogo_completo() {
    let mut p = partida_completa();
    // No começo do Jogo Completo o próprio _XferPatch recusa (texto 10002: poucas formigas pretas).
    usar(&mut p, "Ir para o patch da cozinha");
    assert!(recusas(&mut p).iter().any(|l| l.contains("10002")));
    // Com uma colônia crescida (como o jogador terá ao testar), a troca acontece.
    p.estado.preta.populacao = 50;
    usar(&mut p, "Ir para o patch da cozinha");
    assert_eq!(recusas(&mut p), Vec::<String>::new());
    assert_eq!(p.estado.ambiente.patch, (0, 12));
    assert_ne!(p.estado.ambiente.terreno, 0, "patch da casa: outro conjunto de tiles");
    usar(&mut p, "Ir para o próximo patch da casa");
    assert_eq!(p.estado.ambiente.patch, (0, 1));
    usar(&mut p, "Ir para o patch do quintal ao lado");
    assert_eq!(p.estado.ambiente.patch, (1, 1));
    // No Jogo Rápido o próprio _XferPatch recusa (texto 10000).
    let mut q = partida();
    q.regras.geracao = Some(tabelas_geracao());
    usar(&mut q, "Ir para o patch da cozinha");
    assert!(recusas(&mut q).iter().any(|l| l.contains("10000")));
}
