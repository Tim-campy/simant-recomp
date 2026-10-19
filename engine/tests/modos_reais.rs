//! "Select a Game", menu de ferramentas do Experimental e janela do quintal com os bancos do jogo
//! (SIMANT_DIR ou ../SimAnt-T/MAXIS/ANTWIN; sem eles, avisa e passa sem verificar).
//! Especificação: docs/spec/telas/abertura.md, experimental.md e quintal.md.

use std::path::PathBuf;

use engine::core::menu_ferramentas::{ItemMenu, MenuFerramentas};
use engine::core::quadro::Quadro;
use engine::core::quintal_mapa::{ConfigQuintalMapa, DesenhoQuintal};
use engine::core::selecao::{ConfigModos, Escolha, SelecaoDeJogo};
use engine::core::telas::Interface;
use engine::formats::janela::Retangulo;
use engine::recursos::Recursos;
use engine::rules::estado::Estado;
use engine::rules::experimental::Ferramenta;

fn sha256(t: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(t.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
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

/// Na tela de jogo (tabela de posições 0, 640×480) os cliques da bancada QEMU escolhem os jogos:
/// Quick Game em (345, 228) e Tutorial em (185, 228) na tela (roteiros qemu_snapshot_quick.txt e
/// qemu_tutorial.txt), Full Game em (280, 290) (re_trabalho/modos/r3.txt). Área cliente = tela − 38.
#[test]
fn select_a_game_bate_com_os_cliques_da_bancada() {
    let Some(r) = recursos() else { return };
    let interface = Interface::carregar(&r, 0).unwrap();
    let sel = SelecaoDeJogo::carregar(r.dados::<ConfigModos>("modos.json").unwrap(), &r, &interface).unwrap();
    assert_eq!(sel.escolha_em(345, 228 - 38), Some(Escolha::Rapido));
    assert_eq!(sel.escolha_em(185, 228 - 38), Some(Escolha::Tutorial));
    assert_eq!(sel.escolha_em(280, 290 - 38), Some(Escolha::Completo));
    assert_eq!(sel.escolha_em(185, 375 - 38), Some(Escolha::Experimental));
    assert_eq!(sel.escolha_em(345, 375 - 38), Some(Escolha::Carregar));
    assert_eq!(sel.escolha_em(5, 5), None);
    for e in [Escolha::Tutorial, Escolha::Rapido, Escolha::Completo, Escolha::Experimental, Escolha::Carregar, Escolha::Cancelar] {
        let (x, y) = sel.centro(e).unwrap();
        assert_eq!(sel.escolha_em(x, y), Some(e));
    }
}

#[test]
fn menu_de_ferramentas_e_submenus() {
    let Some(r) = recursos() else { return };
    let mut m = MenuFerramentas::carregar(&r).unwrap();
    let vga = engine::core::resolucao::PerfilDriver { alinhamento_x: 8, ..Default::default() };
    m.abrir(Retangulo { esq: 100, topo: 40, dir: 125, base: 60 }, None, &vga);
    // Área de dentro da moldura no byte (VGA de 16 cores), como nas capturas i05/q01/f01.
    assert_eq!(m.origem.0 % 8, 0);
    // Itens de 17 pixels a partir de (3, 3) do objeto 0 (janela 9 do WINGANT).
    let (ox, oy) = (i32::from(m.origem.0), i32::from(m.origem.1));
    assert_eq!(m.item_em(ox + 10, oy + 5), Some(ItemMenu::Ferramenta(Ferramenta::Info)));
    assert!(m.submenu.is_none());
    assert_eq!(m.item_em(ox + 10, oy + 3 + 17 * 6 + 2), Some(ItemMenu::Ferramenta(Ferramenta::Inseticida)));
    let (_, (sx, sy)) = m.submenu.unwrap();
    // Submenu cideMenu (janela 14): toolSpray e toolStream.
    assert_eq!(m.item_em(i32::from(sx) + 5, i32::from(sy) + 16), Some(ItemMenu::Subopcao(Ferramenta::Inseticida, 1)));
    // A moldura (5 pixels) é do menu; fora dela, não.
    assert!(m.contem(ox - 5, oy - 5));
    assert!(!m.contem(ox - 6, oy - 6));
}

#[test]
fn janela_do_quintal_desenha_os_quatro_modos() {
    let Some(r) = recursos() else { return };
    let interface = Interface::carregar(&r, 0).unwrap();
    let d = DesenhoQuintal::carregar(ConfigQuintalMapa::carregar(&r).unwrap(), &r, &interface).unwrap();
    let mut e = Estado::vazio();
    e.ambiente.patch = (11, 8);
    e.preta.populacao_quintal.0[2 * 16 + 2] = 100;
    for modo in 0..4 {
        e.vistas.modo_quintal = modo;
        let mut q = Quadro::new(640, 442, 17);
        d.desenhar_janela(&mut q, &e);
        let px = |x: usize, y: usize| q.pixels[y * 640 + x];
        // Canto superior esquerdo do cursor do patch (11, 8): (517, 304) na tela (yard_strat.png),
        // menos 1 em x nos modos 0 e 1.
        let x = if modo < 2 { 516 } else { 517 };
        assert_eq!(px(x, 304 - 38), if modo < 2 { 13 } else { 7 }, "modo {modo}");
        if modo == 2 {
            // Colônia preta em (2, 2): polígono em (120 + 56 − 20 + 169 + 5, 157 + 20 + 67 + 3) − 38.
            assert_eq!(px(330, 209), 0);
        }
    }
}

/// Pixels que mudam na janela do quintal entre dois estados, e a caixa que os contém.
fn caixa_da_diferenca(d: &DesenhoQuintal, a: &Estado, b: &Estado) -> Option<(i32, i32, i32, i32)> {
    let (mut qa, mut qb) = (Quadro::new(640, 442, 17), Quadro::new(640, 442, 17));
    d.desenhar_janela(&mut qa, a);
    d.desenhar_janela(&mut qb, b);
    let mut caixa: Option<(i32, i32, i32, i32)> = None;
    for (i, (x, y)) in qa.pixels.iter().zip(&qb.pixels).enumerate() {
        if x != y {
            let (px, py) = ((i % 640) as i32, (i / 640) as i32);
            caixa = Some(caixa.map_or((px, py, px, py), |(x0, y0, x1, y1)| (x0.min(px), y0.min(py), x1.max(px), y1.max(py))));
        }
    }
    caixa
}

/// Gato (`_Draw_SimYard`, bitmap 1300 + `_CatFrame` em (`_CatX`, `_CatY`) + deslocamento dos
/// quadros 10–29): aparece nos modos 0 e 1 com `_CatOn`; no ponto de entrada (0xFC, 0x19), andando
/// 1 pixel abaixo (`ajuste_medido`, bancada).
#[test]
fn gato_aparece_no_quintal() {
    let Some(r) = recursos() else { return };
    let interface = Interface::carregar(&r, 0).unwrap();
    let d = DesenhoQuintal::carregar(ConfigQuintalMapa::carregar(&r).unwrap(), &r, &interface).unwrap();
    let (ox, oy) = d.cfg.origem();
    for modo in 0..2 {
        let mut sem = Estado::vazio();
        sem.vistas.modo_quintal = modo;
        sem.ambiente.casa_a_venda = 1;
        let mut com = sem.clone();
        com.bichos.gato_ligado = 1;
        com.bichos.gato_x = 0xFC;
        com.bichos.gato_y = 0x19;
        for quadro in [0, 1, 2, 4, 5, 11, 12, 20, 25, 29] {
            com.bichos.gato_quadro = quadro;
            let caixa = caixa_da_diferenca(&d, &sem, &com);
            let Some((x0, y0, ..)) = caixa else { panic!("modo {modo}: gato no quadro {quadro} não desenhado") };
            if quadro < 10 {
                assert_eq!((x0, y0), (ox + 0xFC, oy + 0x19 + 1), "modo {modo} quadro {quadro}: {caixa:?}");
            }
        }
    }
}

/// Menino na poltrona (quadro 200), na geladeira (201) e na porta (900–903): `_DrawSimKid` usa o
/// bitmap 8000 + quadro para qualquer quadro ≥ 0, sem deslocamento a partir de 200.
#[test]
fn menino_na_poltrona_na_geladeira_e_na_porta() {
    let Some(r) = recursos() else { return };
    let interface = Interface::carregar(&r, 0).unwrap();
    let d = DesenhoQuintal::carregar(ConfigQuintalMapa::carregar(&r).unwrap(), &r, &interface).unwrap();
    let (ox, oy) = d.cfg.origem();
    let mut sem = Estado::vazio();
    sem.vistas.modo_quintal = 1;
    sem.menino.quadro = -1;
    for (quadro, x, y) in [(200, 45, 166), (201, 207, 47), (900, 172, 73), (901, 172, 73), (902, 172, 73), (903, 172, 73)] {
        let mut com = sem.clone();
        com.menino.quadro = quadro;
        com.menino.x = x;
        com.menino.y = y;
        let caixa = caixa_da_diferenca(&d, &sem, &com);
        let Some((x0, y0, ..)) = caixa else { panic!("menino no quadro {quadro} não desenhado") };
        assert!(x0 >= ox + i32::from(x) && y0 >= oy + i32::from(y), "quadro {quadro}: {caixa:?}");
    }
}

/// Balão do menino (`_DrawSimKid` + `_MakeBalloon`): com `_BoyMessOn` aparece acima do menino, com o
/// contorno preto e o fundo branco das peças; sem ele, nada muda.
#[test]
fn balao_do_menino_aparece_acima_dele() {
    let Some(r) = recursos() else { return };
    let interface = Interface::carregar(&r, 0).unwrap();
    let d = DesenhoQuintal::carregar(ConfigQuintalMapa::carregar(&r).unwrap(), &r, &interface).unwrap();
    let fonte = interface.fontes.get(d.cfg.balao_menino.fonte).and_then(|f| f.as_ref());
    assert!(fonte.is_some(), "fonte do balão");
    let mut e = Estado::vazio();
    e.vistas.modo_quintal = 0;
    e.menino.presente = 1;
    e.menino.quadro = 0;
    e.menino.x = 0xB4;
    e.menino.y = 0x49;
    let mut sem = Quadro::new(640, 442, 17);
    d.desenhar_janela(&mut sem, &e);
    let mut com = Quadro::new(640, 442, 17);
    d.desenhar_janela(&mut com, &e);
    d.desenhar_balao_do_menino(&mut com, &e, fonte);
    assert_eq!(sem.pixels, com.pixels, "sem _BoyMessOn não há balão");
    e.menino.mensagem_ligada = 1;
    e.menino.mensagem_offset = 0;
    d.desenhar_balao_do_menino(&mut com, &e, fonte);
    let (ox, oy) = d.cfg.origem();
    let mudados: Vec<(i32, i32)> = (0..442)
        .flat_map(|y| (0..640).map(move |x| (x, y)))
        .filter(|&(x, y)| sem.pixels[(y * 640 + x) as usize] != com.pixels[(y * 640 + x) as usize])
        .collect();
    assert!(!mudados.is_empty());
    // Tudo acima do pé do menino e à direita do canto dele.
    assert!(mudados.iter().all(|&(x, y)| y < oy + 0x49 + 8 && x >= ox + 0xB4 - 16), "{:?}", mudados.first());
}

/// Examine (`_MagnifyMenu`): canto uma célula à direita e uma acima, virando para dentro da tela, e
/// os textos das listas 1100–1103 (QUICK2: a rainha em (35, 9) do ninho preto).
#[test]
fn examine_canto_e_textos() {
    use engine::core::exame::JanelaExame;
    use engine::formats::save::{JogoSalvo, LayoutSave};
    let Some(r) = recursos() else { return };
    let j = JanelaExame::carregar(&r).unwrap();
    assert_eq!(j.canto((13, 9), (24, 60), (640, 442)), (14 * 16 + 24, 8 * 16 + 60));
    // Perto da direita e da base: vai para a esquerda e para cima a própria largura/altura.
    let (x, y) = j.canto((38, 24), (24, 60), (640, 442));
    assert_eq!((x, y), (39 * 16 + 24 - 92, 23 * 16 + 60 - 87));
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let jogo = JogoSalvo::abrir(&layout, &raiz().join("tests/referencias/saves/QUICK2.ANT")).unwrap();
    let e = Estado::de_save(&jogo).unwrap();
    let ex = engine::rules::experimental::examinar(&e, 2, 35, 9).unwrap();
    let t: Vec<String> = j.textos(&ex).iter().map(|l| String::from_utf8_lossy(l).into_owned()).collect();
    assert_eq!(t, ["Queen", "Egg Laying", "Nothing"]);
}

#[test]
fn info_cartao_128_tem_as_figuras() {
    use engine::core::info::{Carta, ConfigInfo, Registro};
    use engine::formats::banco::Chave;
    use engine::formats::bitmap::{Bitmap, TIPO_BITMAP};
    let Some(r) = recursos() else { return };
    let cfg: ConfigInfo = r.dados("info.json").unwrap();
    let c = Carta::ler(&r, &cfg, 128).unwrap();
    assert_eq!(c.titulo, "Top Card");
    let figuras: Vec<u16> = c.registros.iter().filter_map(|x| if let Registro::Figura { bitmap, .. } = x { Some(*bitmap) } else { None }).collect();
    assert_eq!(figuras, [21005, 151, 150]);
    for id in figuras {
        let b = r.objeto(Chave { tipo: TIPO_BITMAP, id }).and_then(|d| Bitmap::decodificar(&d));
        assert!(b.is_ok(), "bitmap {id}: {:?}", b.err());
    }
}

/// Os 53 cartões (127–179): todos com figura, textos não vazios e popups com texto; os textos com
/// um 0 no meio (Ants, Soldier, Mouth) saem inteiros depois da limpeza de `_DisplayCard`.
#[test]
fn info_todos_os_cartoes_tem_figura_e_texto_inteiro() {
    use engine::core::info::{Carta, ConfigInfo, Registro, TextoCartao};
    let Some(r) = recursos() else { return };
    let cfg: ConfigInfo = r.dados("info.json").unwrap();
    for id in 127..=179u16 {
        let c = Carta::ler(&r, &cfg, id).unwrap();
        assert!(c.registros.iter().any(|x| matches!(x, Registro::Figura { .. })), "cartão {id} sem figura");
        for reg in &c.registros {
            let ids: Vec<u16> = match reg {
                Registro::Texto { textos, .. } => textos.clone(),
                Registro::Figura { areas, .. } => areas.iter().filter(|a| a.valor <= 0).map(|a| (-a.valor) as u16).collect(),
            };
            for t in ids {
                let x = TextoCartao::ler(&r, &cfg, t).unwrap_or_else(|| panic!("cartão {id}: texto {t}"));
                assert!(!x.bytes.is_empty() && x.bytes.iter().all(|&b| b != 0), "cartão {id}: texto {t}");
            }
        }
    }
    let texto = |id| String::from_utf8_lossy(&TextoCartao::ler(&r, &cfg, id).unwrap().bytes).into_owned();
    // Textos inteiros de três cartões, conferidos pelo SHA-256 (o repositório não guarda o texto do
    // original; antes de 2026-10-19 o teste comparava trechos de cada um).
    for (id, tamanho, sha) in [
        (310u16, 622, "9673058859fa28554e6e4950d99929110a31c317bde59ee11275733f67db6a9e"),
        (420, 642, "76417dcc670187c8c6b0ea218b9ffd6a10b19f8dd0012306837ddee0b44d236c"),
        (212, 553, "c1a280d550d3d0733f6bfe53f5f26b322366ed62a1abd2d06fb2852e7e9f1995"),
    ] {
        assert_eq!((texto(id).len(), sha256(&texto(id)).as_str()), (tamanho, sha), "texto {id}");
    }
}
