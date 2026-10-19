//! Mini mapa na interface SDL (`engine::core::mini_mapa`): o botão `ribToYellow` da faixa 34 o abre
//! (`_OpenMiniMapWin`); aberto, é modal e recebe todo o mouse; o jogo fica pausado. Especificação:
//! docs/spec/telas/mapa.md §7.

use super::*;
use engine::core::mini_mapa::AcaoMini;
use sdl2::event::Event;
use sdl2::mouse::MouseButton;

/// Botão apertado em (x, y) da faixa 34: se é o do mini mapa, abre (`_SetPause(1)`). Devolve se
/// abriu.
pub(crate) fn abrir_pelo_botao(s: &mut Sessao, x: i32, y: i32) -> bool {
    if s.modos.janela.na_frente || !s.tela.objetos_da_faixa_em(x, y).contains(&s.mini.cfg.botao.as_str()) {
        return false;
    }
    let Some(botao) = s.tela.retangulo_da_faixa(&s.mini.cfg.botao) else { return false };
    let pausado = s.partida.estado.partida.pausado != 0;
    s.mini.abrir(botao, &s.partida.estado, &s.mapa, s.partida.mostrar_trilhas, true, pausado);
    s.partida.definir_pausa(true);
    println!("mini mapa aberto (plano {})", s.partida.nav.plano_mapa);
    true
}

/// Fecha e volta à pausa de antes (`_SetPause(local_8)`).
fn fechar(s: &mut Sessao) {
    s.mini.fechar();
    let antes = s.mini.pausa_antes;
    s.partida.definir_pausa(antes);
    println!("mini mapa fechado");
}

fn aplicar(s: &mut Sessao, a: AcaoMini) -> bool {
    match a {
        AcaoMini::Nada => false,
        AcaoMini::Centralizar(cx, cy) => {
            s.centralizar_vista(cx, cy);
            true
        }
        AcaoMini::Fechar => {
            fechar(s);
            true
        }
        AcaoMini::FecharEAbrirMapa => {
            fechar(s);
            s.abrir_mapa_ou_quintal();
            true
        }
    }
}

/// Com o mini mapa aberto, o evento vai para ele (o laço modal de `_OpenMiniMapWin` só trata o
/// mouse; as teclas são descartadas, como no original: bancada `mini_teclas`, telas/mapa.md §7).
/// Devolve `None` com ele fechado; senão, se é preciso redesenhar.
pub(crate) fn evento(s: &mut Sessao, ev: &Event) -> Option<bool> {
    if !s.mini.aberto() {
        return None;
    }
    let a = match *ev {
        Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => s.mini.apertar(x, y),
        Event::MouseMotion { x, y, mousestate, .. } if mousestate.left() => s.mini.mover(x, y),
        Event::MouseButtonUp { mouse_btn: MouseButton::Left, x, y, .. } => s.mini.soltar(x, y),
        _ => AcaoMini::Nada,
    };
    Some(aplicar(s, a))
}

pub(crate) fn desenhar(s: &Sessao, q: &mut Quadro) {
    if s.mini.aberto() {
        s.mini.desenhar(q, &s.mapa, &s.partida.nav);
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use engine::rules::rng::GeradorR;

    fn raiz() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    fn com_sessao(f: impl FnOnce(&mut Sessao)) {
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SHARED.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
            return;
        }
        let dados = raiz().join("data");
        let cfg: ConfigJogo = serde_json::from_str(&std::fs::read_to_string(dados.join("jogo.json")).unwrap()).unwrap();
        let recursos = Recursos::abrir(&jogo, &cfg.bancos, &dados, None).unwrap();
        let cores = recursos.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
        let save = raiz().join("tests/referencias/saves/QUICK2.ANT");
        let mut s = Sessao::abrir(&recursos, &dados, Origem::Save(&save, GeradorR::default(), TipoJogo::Rapido), cores).unwrap();
        f(&mut s);
    }

    fn botao(s: &Sessao) -> (i32, i32) {
        let r = s.tela.retangulo_da_faixa(&s.mini.cfg.botao).unwrap();
        (i32::from(r.esq + r.dir) / 2, i32::from(r.topo + r.base) / 2)
    }

    fn ev(baixo: bool, x: i32, y: i32) -> Event {
        let b = MouseButton::Left;
        if baixo {
            Event::MouseButtonDown { timestamp: 0, window_id: 0, which: 0, mouse_btn: b, clicks: 1, x, y }
        } else {
            Event::MouseButtonUp { timestamp: 0, window_id: 0, which: 0, mouse_btn: b, clicks: 1, x, y }
        }
    }

    /// Bancada (mini2): abre pausando, o clique na área centraliza, o clique fora fecha e volta à
    /// pausa de antes; o botão solto dentro da área abre o mapa (m15).
    #[test]
    fn mini_mapa_pausa_centraliza_fecha_e_abre_o_mapa() {
        com_sessao(|s| {
            s.partida.definir_pausa(false);
            s.partida.navegar(Acao::Superficie);
            let (bx, by) = botao(s);
            assert!(abrir_pelo_botao(s, bx, by));
            assert_eq!(s.partida.estado.partida.pausado, 1);
            evento(s, &ev(false, bx, 400));
            assert!(s.mini.aberto(), "solto fora: fica aberto");
            let i = s.mini.interior().unwrap();
            let (ax, ay) = (i32::from(i.esq), i32::from(i.topo));
            evento(s, &ev(true, ax + 70, ay + 32));
            evento(s, &ev(false, ax + 70, ay + 32));
            assert_eq!((s.partida.nav.vista.x, s.partida.nav.vista.y), (70 - 12, 32 - 9), "m12: _CenterEdit(70, 32)");
            evento(s, &ev(true, 400, 300));
            assert!(!s.mini.aberto());
            assert_eq!(s.partida.estado.partida.pausado, 0, "m14: a pausa de antes");
            // Apertar o botão e soltar dentro da área: fecha e abre o mapa (_OpenMapYard).
            assert!(abrir_pelo_botao(s, bx, by));
            let i = s.mini.interior().unwrap();
            assert_eq!(i32::from(i.esq), ax + 1, "mini3: da segunda abertura em diante, uma coluna à direita");
            let mut q = Quadro::new(640, 442, 0);
            desenhar(s, &mut q);
            evento(s, &ev(false, ax + 60, ay + 30));
            assert!(!s.mini.aberto());
            assert_eq!(s.janelas.gerente.topo(), Some(s.janelas.cfg.id(Controle::Mapa)));
        });
    }

    /// Bancada (mapa1, p06–p08): o mapa arrastado pelo título e trocado pelo quintal (ribToYard) fica
    /// no mesmo lugar; o Patch devolve o mapa ao mesmo lugar (`_win_Swap`).
    #[test]
    fn troca_com_o_quintal_no_lugar_do_mapa_movido() {
        com_sessao(|s| {
            s.abrir_mapa();
            let mapa = s.janelas.cfg.id(Controle::Mapa);
            let quintal = s.janelas.cfg.quintal;
            let antes = s.janelas.gerente.janela(mapa).unwrap().externo;
            let (x, y) = (300, i32::from(antes.topo) + 8);
            s.clicar_nas_janelas(x, y, false);
            s.mover_nas_janelas(x - 50, y + 34);
            s.soltar_nas_janelas(x - 50, y + 34);
            let movido = s.janelas.gerente.janela(mapa).unwrap().externo;
            assert_eq!((movido.esq - antes.esq, movido.topo - antes.topo), (-50, 34));
            modos::aplicar_acao_quintal(s, AcaoQuintal::Abrir, true);
            assert!(!s.janelas.aberta(Controle::Mapa));
            assert_eq!(s.janelas.gerente.janela(quintal).unwrap().externo, movido, "p07");
            modos::aplicar_acao_quintal(s, AcaoQuintal::ParaMapa, true);
            assert_eq!(s.janelas.gerente.janela(mapa).unwrap().externo, movido, "p08");
            assert!(!s.janelas.aberta(Controle::Quintal));
        });
    }

    /// Bancada (mapa1/mapa2): Shift+clique no mapa dá a ordem sem centralizar; o clique e o arrasto
    /// centralizam; o duplo clique traz a janela de edição; nas faixas laterais do ninho o
    /// Shift+clique não faz nada.
    #[test]
    fn cliques_no_mapa() {
        com_sessao(|s| {
            s.abrir_mapa();
            s.partida.navegar(Acao::Superficie);
            let a = s.janelas.area_do_mapa(s.recursos).unwrap();
            let celula = |cx: i32, cy: i32| (i32::from(a.esq) + 4 * cx + 2, i32::from(a.topo) + 4 * cy + 2);
            let vista = |s: &Sessao| (s.partida.nav.vista.x, s.partida.nav.vista.y);
            let antes = vista(s);
            let (x, y) = celula(60, 30);
            s.janelas.shift = true;
            s.clicar_nas_janelas(x, y, false);
            s.janelas.shift = false;
            let am = &s.partida.estado.amarela;
            assert_eq!((am.alvo_x, am.alvo_y, am.alvo_plano, am.mover), (60, 30, 1, 1), "q01");
            assert_eq!(vista(s), antes, "Shift+clique não centraliza");
            let (x, y) = celula(30, 40);
            s.clicar_nas_janelas(x, y, false);
            s.janelas.soltar();
            assert_eq!(vista(s), (18, 30), "p02");
            // Duplo clique = um clique simples e depois o duplo: dois _CenterEdit (o primeiro deixa o
            // eixo menor um passo antes, o segundo completa).
            let (x, y) = celula(90, 20);
            s.clicar_nas_janelas(x, y, false);
            s.janelas.soltar();
            assert_eq!(s.janelas.gerente.topo(), Some(s.janelas.cfg.id(Controle::Mapa)));
            s.clicar_nas_janelas(x, y, true);
            s.janelas.soltar();
            assert_eq!(vista(s), (78, 11), "p03");
            assert_eq!(s.janelas.gerente.topo(), Some(janelas::ID_EDICAO), "p03: a janela de edição na frente");
            // Arrasto (p04): a vista vai junto, antes de soltar.
            s.janelas.trazer_para_frente(Some(Controle::Mapa));
            let (x, y) = celula(30, 30);
            s.clicar_nas_janelas(x, y, false);
            let (x2, y2) = celula(100, 50);
            s.janelas.mover(s.recursos, &mut s.partida.estado, x2, y2).unwrap();
            s.centralizar_pelo_mapa();
            assert_eq!(vista(s), (88, 40), "p04: a vista anda durante o arrasto");
            s.janelas.soltar();
            // Ninho: Shift+clique na faixa lateral não faz nada; no meio, a ordem no ninho.
            s.partida.navegar(Acao::NinhoPreto);
            s.janelas.shift = true;
            let (x, y) = celula(10, 30);
            s.clicar_nas_janelas(x, y, false);
            assert_eq!(s.partida.estado.amarela.alvo_x, 60, "q03");
            let (x, y) = celula(32 + 20, 30);
            s.clicar_nas_janelas(x, y, false);
            s.janelas.shift = false;
            let am = &s.partida.estado.amarela;
            assert_eq!((am.alvo_x, am.alvo_y, am.alvo_plano), (20, 30, 2), "q04");
        });
    }
}
