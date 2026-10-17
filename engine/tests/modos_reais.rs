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
    m.abrir(Retangulo { esq: 100, topo: 40, dir: 125, base: 60 });
    // Itens de 17 pixels a partir de (3, 3) do objeto 0 (janela 9 do WINGANT).
    let (ox, oy) = (i32::from(m.origem.0), i32::from(m.origem.1));
    assert_eq!(m.item_em(ox + 10, oy + 5), Some(ItemMenu::Ferramenta(Ferramenta::Info)));
    assert!(m.submenu.is_none());
    assert_eq!(m.item_em(ox + 10, oy + 3 + 17 * 6 + 2), Some(ItemMenu::Ferramenta(Ferramenta::Inseticida)));
    let (_, (sx, sy)) = m.submenu.unwrap();
    // Submenu cideMenu (janela 14): toolSpray e toolStream.
    assert_eq!(m.item_em(i32::from(sx) + 5, i32::from(sy) + 16), Some(ItemMenu::Subopcao(Ferramenta::Inseticida, 1)));
    assert!(!m.contem(ox - 5, oy - 5));
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
