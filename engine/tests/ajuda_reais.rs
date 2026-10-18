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
