//! Ajuda na interface SDL: modo de ajuda (`_bHelp`) pelo clique direito, Shift+F1 ou Delete; o
//! clique seguinte num objeto abre a ajuda do contexto dele; F1 abre o índice; os botões "?" das
//! faixas (`ribHelp`, `ribHelp2`) abrem a ajuda da faixa; e os cursores do executável (modo de ajuda
//! e ferramentas do Experimental). A tabela de contextos está em `engine::core::ajuda_contexto`.
//!
//! Especificação: docs/spec/telas/ajuda-contexto.md.

use std::collections::HashMap;

use engine::core::ajuda::{RespostaAjuda, VisorAjuda};
use engine::core::ajuda_contexto::{ConfigAjudaContexto, TeclaAjuda};
use engine::core::gerenciador::{self, Clique, Estilo, Gerenciador, Metricas, Parte};
use engine::core::quadro::Quadro;
use engine::formats::janela::Retangulo;
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
    /// Posição guardada da janela History do WinHelp (`WIN.INI` do usuário, `[Windows Help]
    /// H_WindowPosition`), lida ao abrir a partida.
    pub historico_guardado: Option<[i16; 4]>,
    /// O visor recebe as teclas (foi aberto ou clicado por último), como a janela do WinHelp com o
    /// foco.
    pub foco: bool,
    /// A janela do WinHelp no gerenciador de janelas (`core/gerenciador.rs`), como janela de nível
    /// superior: menu de sistema, mover, redimensionar, minimizar e maximizar (telas/ajuda.md §2.1).
    /// A "área de trabalho" é a área cliente do port. Criado com o visor.
    pub gerente: Option<Gerenciador>,
}

/// Número da janela do WinHelp no gerenciador próprio dela.
pub const ID_AJUDA: u16 = 0xFFFE;

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
        Ok(AjudaUi { cfg, modo: false, cursores, ultimo_contexto: None, visor: None, aberto: false, historico_guardado: None, foco: false, gerente: None })
    }

    /// A janela do WinHelp minimizada (um ícone escondido atrás do SimAnt maximizado: bancada h11).
    pub fn minimizada(&self) -> bool {
        self.gerente.as_ref().and_then(|g| g.janela(ID_AJUDA)).is_some_and(|j| j.minimizada)
    }

    /// A janela de ajuda está na tela (aberta e não minimizada).
    pub fn na_tela(&self) -> bool {
        self.aberto && !self.minimizada()
    }

    /// Copia para o visor o lugar, o tamanho e o estado que o gerenciador decidiu.
    fn sincronizar(&mut self) {
        let (Some(v), Some(g)) = (self.visor.as_mut(), self.gerente.as_ref()) else { return };
        if let Some(j) = g.janela(ID_AJUDA) {
            let e = j.externo;
            v.cfg.externo = [e.esq, e.topo, e.dir, e.base];
            v.maximizada = j.maximizada();
        }
        v.ativa_no_sistema = self.foco;
    }
}

/// O gerenciador com a janela do WinHelp: o menu e o tamanho mínimo de `ajuda.json` → `janela`
/// por cima da configuração das janelas filhas.
fn criar_gerente(s: &Sessao, v: &VisorAjuda) -> Gerenciador {
    let mut cfg = s.janelas.gerente.cfg.clone();
    if let Some(j) = &v.cfg.janela {
        j.aplicar(&mut cfg);
    }
    let area = Retangulo { esq: 0, topo: 0, dir: s.tela.cfg.largura as i16, base: s.tela.cfg.altura as i16 };
    let mut g = Gerenciador::new(cfg, area);
    g.fase_pontilhado = v.cfg.janela.as_ref().map_or(0, |j| j.fase_pontilhado);
    let estilo = Estilo { titulo: true, menu_sistema: true, minimizar: true, maximizar: true, moldura_grossa: true, rolagem: false, modal: false };
    let metricas = Metricas { espessura: v.cfg.espessura_moldura, altura_titulo: v.cfg.altura_titulo, barra_rolagem: v.cfg.largura_barra };
    g.abrir(ID_AJUDA, v.externo(), estilo, metricas);
    g
}

/// Abre a ajuda no tópico do contexto `ctx` (`WinHelp(HELP_CONTEXT)`). Um contexto que o arquivo
/// não tem (0x1900 do `yardHelpBut`) abre o índice [hipótese: o WinHelp avisa e mostra o índice].
pub fn abrir_contexto(s: &mut Sessao, ctx: u32) {
    s.ajuda.ultimo_contexto = Some(ctx);
    if s.ajuda.visor.is_none() {
        match VisorAjuda::carregar(s.recursos, &s.cores) {
            Ok(mut v) => {
                v.na_tela(s.tela.cfg.largura as i16, s.tela.cfg.altura as i16);
                v.definir_tela(s.tela.cfg.largura as i16, s.tela.cfg.altura as i16 + s.interface.linhas_acima, s.ajuda.historico_guardado);
                s.ajuda.gerente = Some(criar_gerente(s, &v));
                s.ajuda.visor = Some(v)
            }
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
    // Um pedido novo ao WinHelp minimizado o restaura no lugar de antes (bancada h14). Fechado e
    // aberto de novo, ele volta no lugar e no tamanho em que estava (k03).
    if let Some(g) = s.ajuda.gerente.as_mut() {
        if g.janela(ID_AJUDA).is_some_and(|j| j.minimizada) {
            g.restaurar(ID_AJUDA);
        }
    }
    s.ajuda.aberto = true;
    s.ajuda.foco = true;
    s.ajuda.sincronizar();
}

/// O que o gerenciador decidiu para a janela do WinHelp.
fn tratar_clique(s: &mut Sessao, c: Clique) {
    match c {
        Clique::Fechar(_) => fechar(s),
        Clique::Minimizou(_) => {
            // O SimAnt fica com o foco (h11).
            s.ajuda.foco = false;
            println!("ajuda minimizada");
        }
        _ => {}
    }
    s.ajuda.sincronizar();
}

/// Clique esquerdo com o visor aberto: dentro dele, vai para o visor (e ele fica com o foco);
/// fora, o foco volta ao jogo e o clique segue. A moldura (caixa e menu de sistema, título,
/// bordas, botões de minimizar e maximizar) é do gerenciador. Devolve se tratou.
pub fn clique_no_visor(s: &mut Sessao, x: i32, y: i32, duplo: bool) -> bool {
    if !s.ajuda.na_tela() {
        return false;
    }
    if let Some(g) = s.ajuda.gerente.as_mut() {
        // Com o menu de sistema aberto, o clique é dele (fora dele, fecha o menu).
        if g.menu.is_some() || g.arrasto.is_some() {
            let c = g.pressionar(x, y, duplo);
            s.ajuda.foco = true;
            tratar_clique(s, c);
            return true;
        }
    }
    let Some(v) = s.ajuda.visor.as_mut() else { return false };
    // Menus, Search, popup e History do WinHelp recebem o clique antes da moldura (o Search é modal).
    let da_moldura = v.sobreposta().is_none() && !v.contem_janelas(x, y);
    if da_moldura {
        if let Some(g) = s.ajuda.gerente.as_mut() {
            let parte = g.janela(ID_AJUDA).and_then(|j| gerenciador::parte_em(j, x, y));
            if parte.is_some_and(|p| p != Parte::Cliente) {
                let c = g.pressionar(x, y, duplo);
                s.ajuda.foco = true;
                if let Some(v) = s.ajuda.visor.as_mut() {
                    v.tirar_foco_do_historico();
                }
                tratar_clique(s, c);
                return true;
            }
        }
    }
    let Some(v) = s.ajuda.visor.as_mut() else { return false };
    match v.clicar(&s.interface, x, y, duplo) {
        RespostaAjuda::Fora => {
            s.ajuda.foco = false;
            s.ajuda.sincronizar();
            false
        }
        RespostaAjuda::Nada => {
            s.ajuda.foco = true;
            s.ajuda.sincronizar();
            true
        }
        RespostaAjuda::Fechar => {
            fechar(s);
            true
        }
    }
}

/// Mouse andando: o arrasto (mover, redimensionar) e o realce do menu de sistema da ajuda.
/// Devolve se tratou (e é preciso redesenhar).
pub fn mover_mouse(s: &mut Sessao, x: i32, y: i32) -> bool {
    let Some(g) = s.ajuda.gerente.as_mut().filter(|g| g.arrasto.is_some() || g.menu.is_some()) else { return false };
    g.mover_mouse(x, y);
    true
}

/// Botão solto: termina o arrasto da janela de ajuda (ou escolhe o item do menu de sistema).
/// Devolve se tratou.
pub fn soltar(s: &mut Sessao, x: i32, y: i32) -> bool {
    let Some(g) = s.ajuda.gerente.as_mut().filter(|g| g.arrasto.is_some() || g.menu.is_some()) else { return false };
    // Um Move/Size escolhido no menu termina no clique, não ao soltar o botão do item.
    if g.arrasto.is_some_and(|a| matches!(a, gerenciador::Arrasto::Mover { por_teclado: true, .. } | gerenciador::Arrasto::Redimensionar { por_teclado: true, .. })) {
        return true;
    }
    let c = g.soltar(x, y);
    tratar_clique(s, c);
    true
}

/// Tecla com o visor aberto e com o foco (nome da tecla do SDL). Devolve se tratou.
#[cfg(test)]
pub fn tecla_no_visor(s: &mut Sessao, nome: &str) -> bool {
    tecla_no_visor_com(s, nome, false, false)
}

/// Como [`tecla_no_visor`], com Alt (menus do WinHelp) e Shift (maiúsculas no Search).
pub fn tecla_no_visor_com(s: &mut Sessao, nome: &str, alt: bool, shift: bool) -> bool {
    if !(s.ajuda.na_tela() && s.ajuda.foco) {
        return false;
    }
    if let Some(g) = s.ajuda.gerente.as_mut() {
        // Menu de sistema aberto, ou Move/Size pelo teclado: as teclas são do gerenciador.
        if g.menu.is_some() || g.arrasto.is_some() {
            if let Some(c) = g.tecla(nome) {
                tratar_clique(s, c);
            }
            return true;
        }
        // Alt+Espaço abre o menu de sistema; Alt+F4 fecha (os atalhos da janela de nível superior).
        if alt && nome == "Space" {
            g.abrir_menu(ID_AJUDA);
            return true;
        }
        if alt && nome == "F4" {
            fechar(s);
            return true;
        }
    }
    let Some(v) = s.ajuda.visor.as_mut() else { return false };
    match v.tecla_com(&s.interface, nome, alt, shift) {
        RespostaAjuda::Fechar => fechar(s),
        RespostaAjuda::Nada | RespostaAjuda::Fora => {}
    }
    true
}

/// Roda do mouse sobre o visor.
pub fn rolar_visor(s: &mut Sessao, x: i32, y: i32, linhas: i32) -> bool {
    let na_tela = s.ajuda.na_tela();
    let Some(v) = s.ajuda.visor.as_mut().filter(|_| na_tela) else { return false };
    if !v.contem(x, y) {
        return false;
    }
    v.rolar(&s.interface, -linhas * 3 * 13);
    true
}

/// O ponto está sobre a janela de ajuda aberta (outra aplicação no original: o jogo não recebe).
pub fn sobre_visor(s: &Sessao, x: i32, y: i32) -> bool {
    s.ajuda.na_tela() && s.ajuda.visor.as_ref().is_some_and(|v| v.contem(x, y))
}

/// Fecha o WinHelp. O lugar e o tamanho da janela ficam para a próxima abertura (bancada k03).
pub fn fechar(s: &mut Sessao) {
    s.ajuda.aberto = false;
    s.ajuda.foco = false;
    if let Some(g) = s.ajuda.gerente.as_mut() {
        g.menu = None;
        g.arrasto = None;
    }
}

/// Desenha o visor por cima do jogo, se aberto e não minimizado, com o menu de sistema e o
/// contorno do arrasto da janela dele.
pub fn desenhar(s: &Sessao, q: &mut Quadro) {
    if !s.ajuda.na_tela() {
        return;
    }
    if let Some(v) = s.ajuda.visor.as_ref() {
        v.desenhar(q, &s.interface);
    }
    if let Some(g) = &s.ajuda.gerente {
        let fonte = g.cfg.menu_sistema.fontes.iter().find_map(|&i| s.interface.fontes.get(i).and_then(|f| f.as_ref()));
        g.desenhar_por_cima(q, fonte, false);
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
    if s.ajuda.na_tela() && s.ajuda.visor.as_ref().is_some_and(|v| v.contem(x, y)) {

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
    /// Cursores com pixels "inverter" (AND 1 + XOR 1: a lente do `MAGCURSOR`): o cursor do sistema
    /// não inverte o que está embaixo, então o port esconde o do sistema e desenha este no quadro.
    software: HashMap<String, ne::Cursor>,
    /// Cursor do sistema transparente (para esconder o do sistema).
    vazio: Option<sdl2::mouse::Cursor>,
}

/// O cursor tem pixels que invertem a tela (AND 1 e XOR 1).
pub fn tem_inversao(c: &ne::Cursor) -> bool {
    c.e.iter().zip(&c.x).any(|(&e, &x)| e && x)
}

/// Inverte um pixel como o XOR do cursor no driver de 16 cores: o índice i vira i ^ 15, que na
/// paleta padrão é o complemento RGB, menos os dois cinzas (7 ↔ 8).
fn inverter_rgb(p: [u8; 3]) -> [u8; 3] {
    match p {
        [128, 128, 128] => [192, 192, 192],
        [192, 192, 192] => [128, 128, 128],
        [r, g, b] => [255 - r, 255 - g, 255 - b],
    }
}

/// Desenha o cursor `c` com o ponto quente em (x, y) num quadro RGB `largura` × `altura`, como o
/// Windows: AND 1 + XOR 0 transparente, AND 0 preto ou branco, AND 1 + XOR 1 inverte.
pub fn desenhar_cursor(c: &ne::Cursor, rgb: &mut [u8], largura: usize, altura: usize, x: i32, y: i32) {
    let (x0, y0) = (x - i32::from(c.quente.0), y - i32::from(c.quente.1));
    for j in 0..c.altura {
        for i in 0..c.largura {
            let (px, py) = (x0 + i as i32, y0 + j as i32);
            if px < 0 || py < 0 || px as usize >= largura || py as usize >= altura {
                continue;
            }
            let k = j * c.largura + i;
            let o = (py as usize * largura + px as usize) * 3;
            let atual = [rgb[o], rgb[o + 1], rgb[o + 2]];
            let novo = match (c.e[k], c.x[k]) {
                (true, false) => continue,
                (false, false) => [0, 0, 0],
                (false, true) => [255, 255, 255],
                (true, true) => inverter_rgb(atual),
            };
            rgb[o..o + 3].copy_from_slice(&novo);
        }
    }
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
        let software = s.ajuda.cursores.iter().filter(|(_, c)| tem_inversao(c)).map(|(n, c)| (n.clone(), c.clone())).collect();
        let vazio = criar(&ne::Cursor { largura: 32, altura: 32, quente: (0, 0), e: vec![true; 32 * 32], x: vec![false; 32 * 32] }).ok();
        CursoresSdl { mapa, seta, atual: None, software, vazio }
    }

    /// Troca o cursor se o desejado em (x, y) mudou. Devolve se a tela precisa ser redesenhada (o
    /// cursor desenhado pelo port entrou ou saiu).
    pub fn atualizar(&mut self, s: &Sessao, x: i32, y: i32) -> bool {
        let quer = cursor_em(s, x, y).filter(|n| self.mapa.contains_key(n));
        if self.atual.as_ref() == Some(&quer) {
            return false;
        }
        let antes = self.por_software().is_some();
        let soft = quer.as_ref().is_some_and(|n| self.software.contains_key(n));
        let c = if soft { self.vazio.as_ref() } else { quer.as_ref().and_then(|n| self.mapa.get(n)).or(self.seta.as_ref()) };
        match c {
            Some(c) => c.set(),
            None => return false,
        }
        self.atual = Some(quer);
        antes || soft
    }

    /// O cursor que o port desenha no quadro, se houver.
    pub fn por_software(&self) -> Option<&ne::Cursor> {
        self.atual.as_ref()?.as_ref().and_then(|n| self.software.get(n))
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
        let cores = engine::paleta::carregar_rgb(&recursos).unwrap();
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

    fn sessao() -> Option<(Recursos, PathBuf)> {
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz.join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SHARED.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
            return None;
        }
        let dados = raiz.join("data");
        let cfg: ConfigJogo = serde_json::from_str(&std::fs::read_to_string(dados.join("jogo.json")).unwrap()).unwrap();
        Some((Recursos::abrir(&jogo, &cfg.bancos, &dados, None).unwrap(), raiz))
    }

    fn externo(s: &Sessao) -> [i16; 4] {
        s.ajuda.visor.as_ref().unwrap().cfg.externo
    }

    /// A janela do WinHelp como janela de nível superior (bancada qemu_ajuda_janela*.txt): clique na
    /// caixa abre o menu de sistema (Esc fecha), duplo clique fecha; o título arrasta (o contorno
    /// segue e a janela só vai ao soltar); a borda redimensiona até o mínimo de 103 × 26; maximizar
    /// ocupa a área com a moldura de fora e o duplo clique no título restaura; minimizar a esconde, e
    /// um pedido novo de ajuda a restaura no mesmo lugar; fechada e aberta de novo, fica onde estava.
    #[test]
    fn janela_de_ajuda_move_redimensiona_minimiza_e_maximiza() {
        let Some((recursos, raiz)) = sessao() else { return };
        let cores = engine::paleta::carregar_rgb(&recursos).unwrap();
        let save = raiz.join("tests/referencias/saves/QUICK2.ANT");
        let mut s = Sessao::abrir(&recursos, &raiz.join("data"), Origem::Save(&save, GeradorR::default(), TipoJogo::Rapido), cores).unwrap();
        abrir_contexto(&mut s, 0x100);
        let [e, t, d, b] = externo(&s);
        assert_eq!((e, t, d, b), (213, 0, 638, 442));
        // Caixa do menu de sistema: (217, 4)–(235, 22).
        assert!(clique_no_visor(&mut s, 225, 12, false));
        let m = s.ajuda.gerente.as_ref().unwrap().menu.expect("o clique simples abre o menu");
        assert_eq!((m.x, m.y), (217, 22), "o menu abre com o canto na caixa, embaixo do título");
        assert!(tecla_no_visor(&mut s, "Escape"));
        assert!(s.ajuda.gerente.as_ref().unwrap().menu.is_none() && s.ajuda.aberto);
        // Arrastar o título: (400, 12) → (340, 42).
        assert!(clique_no_visor(&mut s, 400, 12, false));
        assert!(mover_mouse(&mut s, 340, 42));
        assert_eq!(externo(&s), [213, 0, 638, 442], "a janela só vai ao soltar");
        let rgb = s.quadro_rgb().unwrap();
        assert!(!rgb.is_empty());
        assert!(soltar(&mut s, 340, 42));
        assert_eq!(externo(&s), [153, 30, 578, 472]);
        // Borda esquerda para a direita até o mínimo de largura.
        assert!(clique_no_visor(&mut s, 154, 200, false));
        mover_mouse(&mut s, 620, 200);
        soltar(&mut s, 620, 200);
        assert_eq!(externo(&s)[2] - externo(&s)[0], 102);
        assert_eq!(externo(&s)[2], 578, "o lado oposto fica");
        let [e, t, d, _] = externo(&s);
        // Maximizar (botão da direita do título).
        let (mx, my) = (i32::from(d) - 4 - 9, i32::from(t) + 4 + 9);
        assert!(clique_no_visor(&mut s, mx, my, false));
        assert!(soltar(&mut s, mx, my));
        assert_eq!(externo(&s), [-4, -4, 644, 446]);
        assert!(s.ajuda.visor.as_ref().unwrap().maximizada);
        s.quadro_rgb().unwrap();
        // Duplo clique no título restaura.
        assert!(clique_no_visor(&mut s, 300, 9, false));
        soltar(&mut s, 300, 9);
        assert!(clique_no_visor(&mut s, 300, 9, true));
        assert_eq!(externo(&s)[0], e);
        // Minimizar (botão à esquerda do de maximizar): some, e o pedido de ajuda a traz de volta.
        let (nx, ny) = (i32::from(d) - 4 - 18 - 1 - 9, i32::from(t) + 4 + 9);
        assert!(clique_no_visor(&mut s, nx, ny, false));
        assert!(soltar(&mut s, nx, ny));
        assert!(s.ajuda.minimizada() && !s.ajuda.na_tela() && !s.ajuda.foco);
        assert!(!clique_no_visor(&mut s, nx, ny, false), "minimizada, não recebe cliques");
        abrir_contexto(&mut s, 0x2200);
        assert!(s.ajuda.na_tela());
        assert_eq!(externo(&s)[0], e);
        // Duplo clique na caixa fecha; aberta de novo, no mesmo lugar e tamanho.
        let caixa = (i32::from(e) + 4 + 9, i32::from(t) + 4 + 9);
        assert!(clique_no_visor(&mut s, caixa.0, caixa.1, false));
        assert!(clique_no_visor(&mut s, caixa.0, caixa.1, true));
        assert!(!s.ajuda.aberto);
        abrir_contexto(&mut s, 0x100);
        assert_eq!(externo(&s)[0], e);
        // Alt+Espaço abre o menu; "n" (Minimize) minimiza.
        assert!(tecla_no_visor_com(&mut s, "Space", true, false));
        assert!(tecla_no_visor_com(&mut s, "n", false, false));
        assert!(s.ajuda.minimizada());
    }

    /// AND/XOR como o Windows: transparente, preto, branco e invertido (a lente do `MAGCURSOR`, que
    /// o cursor do sistema do port pintava de preto). Dos cursores do jogo, só a lupa inverte.
    #[test]
    fn cursor_com_lente_invertida() {
        let c = ne::Cursor { largura: 4, altura: 1, quente: (1, 0), e: vec![true, false, false, true], x: vec![false, false, true, true] };
        let mut rgb = vec![128u8; 6 * 3];
        rgb[3 * 3..3 * 3 + 3].copy_from_slice(&[128, 128, 0]);
        desenhar_cursor(&c, &mut rgb, 6, 1, 2, 0);
        assert_eq!(&rgb[3..6], &[128, 128, 128], "transparente");
        assert_eq!(&rgb[6..9], &[0, 0, 0]);
        assert_eq!(&rgb[9..12], &[255, 255, 255]);
        assert_eq!(&rgb[12..15], &[192, 192, 192], "cinza escuro invertido = cinza claro (7 ^ 15 = 8)");
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz.join("SimAnt-T/MAXIS/ANTWIN"));
        let Ok(exe) = std::fs::read(jogo.join("SIMANTW.EXE")) else { return };
        let todos = ne::cursores(&exe).unwrap();
        let mut com: Vec<&str> = todos.iter().filter(|(_, c)| tem_inversao(c)).map(|(n, _)| n.as_str()).collect();
        com.sort();
        assert_eq!(com, ["MAGCURSOR"]);
    }
}

