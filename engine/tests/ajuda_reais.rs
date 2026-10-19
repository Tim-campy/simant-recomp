//! Leitor do SIMANT.HLP com o arquivo original do usuário (não versionado).
//! Pasta do jogo: SIMANT_DIR ou ../SimAnt-T/MAXIS/ANTWIN. Sem o arquivo, o teste avisa e passa.

use std::path::PathBuf;

use engine::formats::hlp::{ArquivoAjuda, Item};

fn ajuda() -> Option<ArquivoAjuda> {
    let d = std::env::var_os("SIMANT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../SimAnt-T/MAXIS/ANTWIN"));
    let f = d.join("SIMANT.HLP");
    if !f.is_file() {
        eprintln!("AVISO: {} não encontrado; teste ignorado", f.display());
        return None;
    }
    Some(ArquivoAjuda::ler(std::fs::read(f).unwrap()).unwrap())
}

#[test]
fn topicos_e_contextos_do_simant_hlp() {
    let Some(a) = ajuda() else { return };
    assert_eq!(a.titulo, "SimAnt");
    assert_eq!(a.topicos.len(), 344);
    // Contextos conferidos com o WinHelp do original na bancada (a02, a04, a05, a06) e pelo |CTXOMAP.
    for (ctx, titulo) in [
        (0x2200, "Patch Ribbon Bar"),
        (0x2209, "Status Window Button"),
        (0x4, "Edit Window"),
        (0x102, "Map Window"),
        (0x1300, "Caste Control Window"),
        (0x2308, "Help Button"),
        (0xFD48, "Bookmarks"),
        (0x100, "SimAnt"),
    ] {
        let i = a.topico_do_contexto(ctx).unwrap_or_else(|| panic!("contexto {ctx:#x}"));
        assert_eq!(a.topicos[i].titulo, titulo, "contexto {ctx:#x}");
    }
    // Índice: título, "Welcome to SimAnt" e 14 saltos (o primeiro para Introduction).
    let indice = &a.topicos[a.topico_do_contexto(0x100).unwrap()];
    let itens: Vec<&Item> = indice.paragrafos.iter().flat_map(|p| p.itens.iter()).collect();
    let textos: Vec<String> = itens
        .iter()
        .filter_map(|i| match i {
            Item::Texto { texto, .. } => Some(String::from_utf8_lossy(texto).into_owned()),
            _ => None,
        })
        .collect();
    assert_eq!(&textos[..3], ["SimAnt", "Welcome to SimAnt", "Introduction"]);
    let saltos: Vec<_> = itens
        .iter()
        .filter_map(|i| match i {
            Item::Texto { salto: Some(s), .. } => Some(*s),
            _ => None,
        })
        .collect();
    assert_eq!(saltos.len(), 14);
    assert_eq!(a.topicos[a.topico_do_numero(saltos[0].numero_topico).unwrap()].titulo, "Introduction");
    assert!(!saltos[0].popup);
    // Fontes: 0 = Helv 12 pt negrito, 1 = Helv 10 pt.
    assert_eq!((a.fontes[0].face.as_str(), a.fontes[0].meios_pontos, a.fontes[0].negrito), ("Helv", 24, true));
    assert_eq!((a.fontes[1].face.as_str(), a.fontes[1].meios_pontos, a.fontes[1].negrito), ("Helv", 20, false));
}

#[test]
fn todas_as_figuras_decodificam() {
    let Some(a) = ajuda() else { return };
    let mut n = 0;
    for t in &a.topicos {
        for p in &t.paragrafos {
            for i in &p.itens {
                if let Item::Figura { numero, .. } = i {
                    let f = a.figura(*numero).unwrap_or_else(|e| panic!("{}: {e}", t.titulo));
                    assert!(f.largura > 0 && f.altura > 0);
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 65);
    let f = a.figura(0).unwrap();
    assert_eq!((f.largura, f.altura), (75, 75));
}

fn visor() -> Option<(engine::core::ajuda::VisorAjuda, engine::core::telas::Interface)> {
    use engine::core::telas::Interface;
    use engine::paleta::Paleta;
    use engine::recursos::Recursos;
    let d = std::env::var_os("SIMANT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../SimAnt-T/MAXIS/ANTWIN"));
    if !d.join("SIMANT.HLP").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
        return None;
    }
    let dados = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data");
    let bancos = ["SHARED", "WINGANT", "SOUND"].map(String::from);
    let r = Recursos::abrir(&d, &bancos, &dados, None).unwrap();
    let interface = Interface::carregar(&r, 0).unwrap();
    let cores = r.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
    Some((engine::core::ajuda::VisorAjuda::carregar(&r, &cores).unwrap(), interface))
}

/// Janela History do WinHelp (telas/ajuda.md, "History"): sem `H_WindowPosition`, um terço da tela
/// (213, 160, 213 × 160 em 640×480: o lugar medido na bancada VGA, igual ao `externo` antigo); com a
/// chave, o lugar guardado (bancada SVGA, `[100,50,300,250,0]` → 100, 50, 300 × 250). O visor do port
/// começa na linha 0 da área cliente, 2 acima do WinHelp: a History sobe junto.
#[test]
fn janela_history_no_lugar_do_winhelp() {
    use engine::formats::janela::Retangulo;
    let Some((mut v, _)) = visor() else { return };
    v.na_tela(640, 442);
    assert_eq!(v.retangulo_historico(), Some(Retangulo { esq: 213, topo: 158, dir: 426, base: 318 }), "sem a tela, o externo antigo");
    v.definir_tela(640, 480, None);
    assert_eq!(v.retangulo_historico(), Some(Retangulo { esq: 213, topo: 158, dir: 426, base: 318 }));
    v.definir_tela(640, 480, Some([100, 50, 300, 250]));
    assert_eq!(v.retangulo_historico(), Some(Retangulo { esq: 100, topo: 48, dir: 400, base: 298 }));
    // A chave da instalação do usuário (de uma sessão em 800×600) é o próprio lugar padrão de lá.
    v.na_tela(800, 562);
    v.definir_tela(800, 600, None);
    let padrao = v.retangulo_historico();
    v.definir_tela(800, 600, Some([266, 200, 266, 200]));
    assert_eq!(v.retangulo_historico(), padrao);
}

/// Cliques no visor como no WinHelp do original (posições do port: janela de y 0 a 441, 2 linhas
/// acima da janela do original em a04): salto, Back, Contents, Esc, clique fora.
#[test]
fn visor_navega_por_saltos_e_botoes() {
    use engine::core::ajuda::RespostaAjuda;
    use engine::core::quadro::Quadro;
    let Some((mut v, i)) = visor() else { return };
    assert!(v.abrir_contexto(0x2200));
    assert_eq!(v.topico_atual().titulo, "Patch Ribbon Bar");
    // "General Info": linha 101..116 da tela do original (a04) = 99..114 no port.
    assert_eq!(v.clicar(&i, 240, 105, false), RespostaAjuda::Nada);
    assert_eq!(v.topico_atual().titulo, "General Info");
    // Back (botão 3: x 335..393).
    v.clicar(&i, 360, 50, false);
    assert_eq!(v.topico_atual().titulo, "Patch Ribbon Bar");
    // Contents (botão 1).
    v.clicar(&i, 240, 50, false);
    assert_eq!(v.topico_atual().titulo, "SimAnt");
    assert_eq!(v.clicar(&i, 100, 100, false), RespostaAjuda::Fora);
    assert_eq!(v.clicar(&i, 225, 10, true), RespostaAjuda::Fechar);
    assert_eq!(v.tecla(&i, "Escape"), RespostaAjuda::Fechar);
    // Todos os tópicos se desenham sem pânico; algum precisa de rolagem e rola.
    let mut rolou = false;
    for k in 0..v.arquivo.topicos.len() {
        v.abrir_topico(k);
        let mut q = Quadro::new(640, 442, 0);
        v.desenhar(&mut q, &i);
        let antes = q.pixels.clone();
        v.tecla(&i, "PageDown");
        v.desenhar(&mut q, &i);
        rolou |= q.pixels != antes;
    }
    assert!(rolou, "nenhum tópico rolou");
}

#[test]
fn palavras_chave_do_search() {
    let Some(a) = ajuda() else { return };
    let k = a.palavras_chave().unwrap();
    assert_eq!(k.len(), 128);
    assert_eq!(k[0].palavra, "About");
    let q = k.iter().find(|p| p.palavra == "Queen").unwrap();
    let t: Vec<&str> = q.topicos.iter().map(|&p| a.topicos[a.topico_da_posicao(p).unwrap()].titulo_bruto.as_str()).collect();
    // Bancada s03: "Queen" → "Queens (id=0x19b)".
    assert_eq!(t, ["Queens (id=0x19b)"]);
}

/// Search, History, menus e popup como na bancada (re_trabalho/agente_ui/winhelp, medidas.md).
#[test]
fn janelas_do_winhelp_como_na_bancada() {
    use engine::core::ajuda::RespostaAjuda;
    use engine::core::ajuda_janelas::Sobreposta;
    use engine::formats::janela::Retangulo;
    let Some((mut v, i)) = visor() else { return };
    // Alt+F abre o File com o primeiro item; seta para baixo pula o separador; Exit fecha a ajuda.
    v.tecla_com(&i, "F", true, false);
    assert_eq!(v.sobreposta(), Some(&Sobreposta::Menu { menu: 0, item: Some(0) }));
    v.tecla_com(&i, "Down", false, false);
    assert_eq!(v.sobreposta(), Some(&Sobreposta::Menu { menu: 0, item: Some(2) }));
    assert_eq!(v.tecla_com(&i, "X", false, false), RespostaAjuda::Fechar);
    // Search: "queen" seleciona Queen; Enter = Show Topics; Enter = Go To abre "Queens".
    v.tecla(&i, "S");
    assert!(!v.janela_ativa(), "o diálogo deixa a janela de ajuda inativa");
    for c in ["Q", "U", "E", "E", "N"] {
        v.tecla(&i, c);
    }
    v.tecla(&i, "Return");
    match v.sobreposta() {
        Some(Sobreposta::Busca(b)) => assert_eq!((b.texto.as_str(), b.topicos.as_ref().map(Vec::len)), ("Queen", Some(1))),
        outro => panic!("{outro:?}"),
    }
    v.tecla(&i, "Return");
    assert_eq!(v.topico_atual().titulo, "Queens");
    assert!(v.sobreposta().is_none());
    // Popup da figura (0xE0): "Queen Info", 238 de largura, centrado na figura (bancada p03: x 125–362).
    v.clicar(&i, 250, 283, false);
    match v.sobreposta() {
        Some(Sobreposta::Popup { topico, caixa }) => {
            assert_eq!(v.arquivo.topicos[*topico].titulo, "Queen Info");
            assert_eq!((caixa.esq, caixa.dir), (125, 363));
            let _: Retangulo = *caixa;
        }
        outro => panic!("{outro:?}"),
    }
    v.clicar(&i, 400, 400, false);
    assert!(v.sobreposta().is_none(), "qualquer clique fecha o popup sem seguir");
    assert_eq!(v.topico_atual().titulo, "Queens");
    // History: o mais recente em cima, com o anterior selecionado.
    v.tecla(&i, "T");
    let h = v.itens_historico();
    let titulos: Vec<&str> = h.iter().map(|&k| v.arquivo.topicos[k].titulo.as_str()).collect();
    assert_eq!(titulos, ["Queens", "SimAnt"]);
    assert_eq!(v.janela_historico().map(|h| h.sel), Some(1));
    v.tecla(&i, "Return");
    assert_eq!(v.topico_atual().titulo, "SimAnt");
}
