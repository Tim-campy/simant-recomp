//! Ajuda na interface SDL: modo de ajuda (`_bHelp`) pelo clique direito, Shift+F1 ou Delete; o
//! clique seguinte num objeto abre a ajuda do contexto dele; F1 abre o índice; os botões "?" das
//! faixas (`ribHelp`, `ribHelp2`) abrem a ajuda da faixa; e os cursores do executável (modo de ajuda
//! e ferramentas do Experimental). A tabela de contextos está em `engine::core::ajuda_contexto`.
//!
//! Especificação: docs/spec/telas/ajuda-contexto.md.

use std::collections::HashMap;

use engine::core::ajuda::{RespostaAjuda, VisorAjuda};
use engine::core::ajuda_contexto::{ConfigAjudaContexto, TeclaAjuda};
use engine::core::quadro::Quadro;
use engine::core::janelas::ID_EDICAO;
use engine::core::navegacao::Evento as EventoNav;
use engine::formats::ne;

use super::Sessao;

pub struct AjudaUi {
    pub cfg: ConfigAjudaContexto,
    /// `_bHelp`: o próximo clique num objeto abre a ajuda dele.
    pub modo: bool,
    /// Cursores lidos do executável (vazio se ele não existe).
    pub cursores: HashMap<String, ne::Cursor>,
    /// Último contexto pedido (para os testes e o terminal).
    pub ultimo_contexto: Option<u32>,
    /// Visor de ajuda (core/ajuda.rs), carregado na primeira vez; `aberto` = na tela.
    pub visor: Option<VisorAjuda>,
    pub aberto: bool,
    /// O visor recebe as teclas (foi aberto ou clicado por último), como a janela do WinHelp com o
    /// foco.
    pub foco: bool,
}

impl AjudaUi {
    pub fn carregar(recursos: &engine::recursos::Recursos) -> Result<AjudaUi, String> {
        let cfg: ConfigAjudaContexto = recursos.dados(engine::core::ajuda_contexto::ARQUIVO_CONFIG)?;
        let cursores = match recursos.arquivo(&cfg.cursores.executavel).and_then(|d| ne::cursores(&d)) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("cursores: {e}");
                HashMap::new()
            }
        };
        Ok(AjudaUi { cfg, modo: false, cursores, ultimo_contexto: None, visor: None, aberto: false, foco: false })
    }
}

/// Abre a ajuda no tópico do contexto `ctx` (`WinHelp(HELP_CONTEXT)`). Um contexto que o arquivo
/// não tem (0x1900 do `yardHelpBut`) abre o índice [hipótese: o WinHelp avisa e mostra o índice].
pub fn abrir_contexto(s: &mut Sessao, ctx: u32) {
    s.ajuda.ultimo_contexto = Some(ctx);
    if s.ajuda.visor.is_none() {
        match VisorAjuda::carregar(s.recursos, &s.cores) {
            Ok(v) => s.ajuda.visor = Some(v),
            Err(e) => {
                eprintln!("ajuda: {e}");
                return;
            }
        }
    }
    let Some(v) = s.ajuda.visor.as_mut() else { return };
    if !v.abrir_contexto(ctx) {
        println!("ajuda: contexto 0x{ctx:X} não existe no arquivo; índice");
        v.abrir_indice();
    }
    println!("ajuda: contexto 0x{ctx:X}: {}", v.topico_atual().titulo);
    s.ajuda.aberto = true;
    s.ajuda.foco = true;
}

/// Clique esquerdo com o visor aberto: dentro dele, vai para o visor (e ele fica com o foco);
/// fora, o foco volta ao jogo e o clique segue. Devolve se tratou.
pub fn clique_no_visor(s: &mut Sessao, x: i32, y: i32, duplo: bool) -> bool {
    if !s.ajuda.aberto {
        return false;
    }
    let Some(v) = s.ajuda.visor.as_mut() else { return false };
    match v.clicar(&s.interface, x, y, duplo) {
        RespostaAjuda::Fora => {
            s.ajuda.foco = false;
            false
        }
        RespostaAjuda::Nada => {
            s.ajuda.foco = true;
            true
        }
        RespostaAjuda::Fechar => {
            fechar(s);
            true
        }
    }
}

/// Tecla com o visor aberto e com o foco (nome da tecla do SDL). Devolve se tratou.
pub fn tecla_no_visor(s: &mut Sessao, nome: &str) -> bool {
    if !(s.ajuda.aberto && s.ajuda.foco) {
        return false;
    }
    let Some(v) = s.ajuda.visor.as_mut() else { return false };
    match v.tecla(&s.interface, nome) {
        RespostaAjuda::Fechar => fechar(s),
        RespostaAjuda::Nada | RespostaAjuda::Fora => {}
    }
    true
}

/// Roda do mouse sobre o visor.
pub fn rolar_visor(s: &mut Sessao, x: i32, y: i32, linhas: i32) -> bool {
    let Some(v) = s.ajuda.visor.as_mut().filter(|_| s.ajuda.aberto) else { return false };
    if !v.contem(x, y) {
        return false;
    }
    v.rolar(&s.interface, -linhas * 3 * 13);
    true
}

/// O ponto está sobre a janela de ajuda aberta (outra aplicação no original: o jogo não recebe).
pub fn sobre_visor(s: &Sessao, x: i32, y: i32) -> bool {
    s.ajuda.aberto && s.ajuda.visor.as_ref().is_some_and(|v| v.contem(x, y))
}

pub fn fechar(s: &mut Sessao) {
    s.ajuda.aberto = false;
    s.ajuda.foco = false;
}

/// Desenha o visor por cima do jogo, se aberto.
pub fn desenhar(s: &Sessao, q: &mut Quadro) {
    if let Some(v) = s.ajuda.visor.as_ref().filter(|_| s.ajuda.aberto) {
        v.desenhar(q, &s.interface);
    }
}

/// Botão "?" da faixa sob o ponto (`ribHelp` 0x2208 na 34, `ribHelp2` 0x2308 na 35): abre a ajuda
/// da faixa (`_DoWinHelp`). Devolve se tratou.
pub fn botao_da_faixa(s: &mut Sessao, x: i32, y: i32) -> bool {
    let nomes: Vec<String> = if s.modos.janela.na_frente {
        s.modos.quintal.objetos_da_faixa_em(x, y).into_iter().map(String::from).collect()
    } else {
        s.tela.objetos_da_faixa_em(x, y).into_iter().map(String::from).collect()
    };
    let Some(b) = s.ajuda.cfg.botoes_ajuda.iter().find(|b| nomes.contains(&b.objeto)).cloned() else { return false };
    if b.licao {
        s.partida.estado.tutorial.temporario = 1;
    }
    abrir_contexto(s, b.contexto);
    true
}

/// Botão "?" de uma janela de controle (Caste, Behavior, History, quintal) sob o ponto, na janela da
/// frente: abre a ajuda dela (`_DoWinHelp`). Devolve se tratou.
pub fn botao_de_janela(s: &mut Sessao, x: i32, y: i32) -> bool {
    let Some(id) = s.janelas.gerente.janela_em(x, y) else { return false };
    if s.janelas.gerente.topo() != Some(id) || id == ID_EDICAO {
        return false;
    }
    let Some(obj) = s.janelas.objeto_em(s.recursos, id, x, y) else { return false };
    let Some(&[_, ctx]) = s.ajuda.cfg.botoes_janela.iter().find(|[o, _]| *o == u32::from(obj)) else { return false };
    abrir_contexto(s, ctx);
    true
}

/// Liga ou desliga o modo de ajuda.
pub fn alternar_modo(s: &mut Sessao) {
    s.ajuda.modo = !s.ajuda.modo;
    println!("modo de ajuda {}", if s.ajuda.modo { "ligado" } else { "desligado" });
}

/// Teclas da ajuda (`_HelpKeyDown`): F1 abre o índice; Shift+F1 e Delete alternam o modo; Esc o
/// desliga (só quando ligado). Devolve se tratou.
pub fn tecla(s: &mut Sessao, ev: &EventoNav) -> bool {
    match s.ajuda.cfg.tecla(ev) {
        Some(TeclaAjuda::Indice) => {
            let c = s.ajuda.cfg.contexto_indice;
            abrir_contexto(s, c);
            true
        }
        Some(TeclaAjuda::Alternar) => {
            alternar_modo(s);
            true
        }
        Some(TeclaAjuda::Sair) if s.ajuda.modo => {
            s.ajuda.modo = false;
            true
        }
        _ => false,
    }
}

/// Id do objeto (janela × 256 + índice) sob o ponto, como `_win_FindObject`: a janela da frente sob
/// o cursor, senão a faixa. `None` fora de tudo.
pub fn objeto_em(s: &Sessao, x: i32, y: i32) -> Option<u16> {
    if let Some(id) = s.janelas.gerente.janela_em(x, y) {
        if id == ID_EDICAO {
            return Some(0);
        }
        return s.janelas.objeto_em(s.recursos, id, x, y).or(Some(id << 8));
    }
    if y < i32::from(s.interface.moldura.faixa.altura) {
        let c = &s.ajuda.cfg;
        return Some(if s.modos.janela.na_frente {
            (c.faixa_quintal << 8) | s.modos.quintal.indice_da_faixa_em(x, y) as u16
        } else {
            (c.faixa_mapa << 8) | s.tela.indice_da_faixa_em(x, y) as u16
        });
    }
    None
}

/// Clique esquerdo com o modo de ajuda ligado (`_DoEvent` com `_bHelp`): o modo desliga e, se o
/// objeto tem ajuda, ela abre no contexto dele. Fora de qualquer objeto, nada acontece e o modo
/// continua. Devolve se tratou o clique.
pub fn clique(s: &mut Sessao, x: i32, y: i32) -> bool {
    if !s.ajuda.modo {
        return false;
    }
    let Some(id) = objeto_em(s, x, y) else { return true };
    s.ajuda.modo = false;
    match s.ajuda.cfg.contexto(id, &s.partida.funcoes_botoes) {
        Some(ctx) => abrir_contexto(s, ctx),
        None => println!("ajuda: objeto 0x{id:04X} sem ajuda"),
    }
    true
}

/// Nome do cursor que o original mostraria em (x, y) (`WM_SETCURSOR` em `MAINWNDPROC`): o de ajuda
/// no modo de ajuda; no Experimental, o da ferramenta sobre a área cliente da janela de edição quando
/// ela é a da frente; senão a seta (`None`).
pub fn cursor_em(s: &Sessao, x: i32, y: i32) -> Option<String> {
    let c = &s.ajuda.cfg.cursores;
    if s.ajuda.aberto && s.ajuda.visor.as_ref().is_some_and(|v| v.contem(x, y)) {
        return None;
    }
    if s.ajuda.modo {
        return Some(c.ajuda.clone());
    }
    if s.partida.estado.partida.tipo_jogo != 3 || s.janelas.gerente.topo() != Some(ID_EDICAO) {
        return None;
    }
    let cliente = s.janelas.gerente.janela(ID_EDICAO).is_some_and(|j| {
        s.janelas.gerente.janela_em(x, y) == Some(ID_EDICAO) && engine::core::gerenciador::parte_em(j, x, y) == Some(engine::core::gerenciador::Parte::Cliente)
    });
    if !cliente {
        return None;
    }
    let f = usize::try_from(s.partida.estado.partida.ferramenta_experimental).ok()?;
    c.ferramentas.get(f).cloned()
}

/// Cursores do SDL criados a partir dos do executável, e o que está na tela.
pub struct CursoresSdl {
    mapa: HashMap<String, sdl2::mouse::Cursor>,
    seta: Option<sdl2::mouse::Cursor>,
    atual: Option<Option<String>>,
}

fn criar(c: &ne::Cursor) -> Result<sdl2::mouse::Cursor, String> {
    // AND 1 + XOR 0 = transparente; AND 0: XOR 0 preto, 1 branco; AND 1 + XOR 1 (inverter) = preto.
    let mut px = vec![0u8; c.largura * c.altura * 4];
    for i in 0..c.largura * c.altura {
        let (r, a) = match (c.e[i], c.x[i]) {
            (true, false) => (0, 0),
            (false, false) | (true, true) => (0, 255),
            (false, true) => (255, 255),
        };
        px[i * 4..i * 4 + 4].copy_from_slice(&[r, r, r, a]);
    }
    let sup = sdl2::surface::Surface::from_data(&mut px, c.largura as u32, c.altura as u32, (c.largura * 4) as u32, sdl2::pixels::PixelFormatEnum::ABGR8888)?;
    sdl2::mouse::Cursor::from_surface(sup, i32::from(c.quente.0), i32::from(c.quente.1))
}

impl CursoresSdl {
    pub fn new(s: &Sessao) -> CursoresSdl {
        let mut mapa = HashMap::new();
        for (n, c) in &s.ajuda.cursores {
            match criar(c) {
                Ok(k) => {
                    mapa.insert(n.clone(), k);
                }
                Err(e) => eprintln!("cursor {n}: {e}"),
            }
        }
        let seta = sdl2::mouse::Cursor::from_system(sdl2::mouse::SystemCursor::Arrow).ok();
        CursoresSdl { mapa, seta, atual: None }
    }

    /// Troca o cursor se o desejado em (x, y) mudou.
    pub fn atualizar(&mut self, s: &Sessao, x: i32, y: i32) {
        let quer = cursor_em(s, x, y).filter(|n| self.mapa.contains_key(n));
        if self.atual.as_ref() == Some(&quer) {
            return;
        }
        match quer.as_ref().and_then(|n| self.mapa.get(n)).or(self.seta.as_ref()) {
            Some(c) => c.set(),
            None => return,
        }
        self.atual = Some(quer);
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::*;

    fn tecla(nome: &str, shift: bool) -> EventoNav {
        EventoNav::Tecla { nome: nome.into(), ctrl: false, shift, alt: false }
    }

    /// Bancada (qemu_ui_ajuda.txt, qemu_ui_barras_direito.txt, 2026-10-18): o clique direito liga e
    /// desliga `_bHelp`; com ele ligado, o clique no mapa da edição abre "Edit Window" (4) e o modo
    /// desliga; o botão de estatísticas abre "Status Window Button" (0x2209); o "?" da faixa abre
    /// "Patch Ribbon Bar" (0x2200); F1 abre o índice (0x100). Tela − 37 = área cliente do port.
    #[test]
    fn ajuda_por_contexto_como_no_original() {
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz.join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SHARED.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
            return;
        }
        let dados = raiz.join("data");
        let cfg: ConfigJogo = serde_json::from_str(&std::fs::read_to_string(dados.join("jogo.json")).unwrap()).unwrap();
        let recursos = Recursos::abrir(&jogo, &cfg.bancos, &dados, None).unwrap();
        let cores = recursos.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
        let save = raiz.join("tests/referencias/saves/QUICK2.ANT");
        let mut s = Sessao::abrir(&recursos, &dados, Origem::Save(&save, GeradorR::default(), TipoJogo::Rapido), cores).unwrap();
        assert!(!clique(&mut s, 192, 278 - 37), "sem o modo, o clique segue o caminho normal");
        alternar_modo(&mut s);
        assert_eq!(cursor_em(&s, 10, 10).as_deref(), Some("HELPCURSOR"));
        assert!(clique(&mut s, 192, 278 - 37));
        assert_eq!(s.ajuda.ultimo_contexto, Some(4));
        assert!(!s.ajuda.modo);
        alternar_modo(&mut s);
        assert!(clique(&mut s, 320, 70 - 37));
        assert_eq!(s.ajuda.ultimo_contexto, Some(0x2209));
        assert!(botao_da_faixa(&mut s, 287, 70 - 37));
        assert_eq!(s.ajuda.ultimo_contexto, Some(0x2200));
        assert_eq!(s.partida.estado.tutorial.temporario, 1);
        assert!(super::tecla(&mut s, &tecla("F1", false)));
        assert_eq!(s.ajuda.ultimo_contexto, Some(0x100));
        assert!(super::tecla(&mut s, &tecla("Delete", false)));
        assert!(s.ajuda.modo);
        assert!(super::tecla(&mut s, &tecla("Escape", false)));
        assert!(!s.ajuda.modo);
        assert!(!super::tecla(&mut s, &tecla("Escape", false)), "Esc sem o modo segue adiante");
        assert!(super::tecla(&mut s, &tecla("F1", true)));
        assert!(s.ajuda.modo);
        // O visor abriu no último contexto pedido e fecha com Esc (tem o foco).
        assert!(s.ajuda.aberto);
        abrir_contexto(&mut s, 0x2200);
        assert_eq!(s.ajuda.visor.as_ref().unwrap().topico_atual().titulo, "Patch Ribbon Bar");
        let rgb = s.quadro_rgb().unwrap();
        if let Some(saida) = std::env::var_os("SIMANT_SAIDA_TESTE") {
            crate::salvar_png(&PathBuf::from(saida).join("ajuda_ribhelp.png"), s.tela.cfg.largura, s.tela.cfg.altura, &rgb).unwrap();
        }
        assert!(tecla_no_visor(&mut s, "Escape"));
        assert!(!s.ajuda.aberto);
    }
}
