//! Modos de jogo na interface SDL: escolha no "Select a Game" (na abertura e com File → New Game,
//! Ctrl+N, durante a partida), janela do quintal (Yard View, faixa 35) com a escolha e a troca de
//! patch. A lógica fica no engine (`core::selecao`, `core::quintal_mapa`, `rules::troca_patch`);
//! aqui só se liga a entrada do SDL e o desenho por cima da tela de jogo.
//!
//! Especificação: docs/spec/telas/abertura.md ("Escolha do jogo") e docs/spec/telas/quintal.md.

use engine::core::menu_ferramentas::{ItemMenu, MenuFerramentas};
use engine::core::navegacao::Evento as EventoNav;
use engine::core::quadro::Quadro;
use engine::core::janelas::Controle;
use engine::core::quintal_mapa::{self, AcaoQuintal, ConfigQuintalMapa, DesenhoQuintal, JanelaQuintal};
use engine::core::selecao::{ConfigModos, Escolha, SelecaoDeJogo};
use engine::core::telas::{desenhar_janela_na_tela, Interface};
use engine::recursos::Recursos;
use engine::rules::estado::Evento;
use engine::rules::experimental::{self, Contexto, ControleExperimental, Efeito, Ferramenta, TabelasExperimental};

use super::Sessao;

/// Estado dos modos na tela de jogo: janela do quintal e "Select a Game" aberto pelo Ctrl+N.
pub struct Modos {
    pub quintal: DesenhoQuintal,
    pub janela: JanelaQuintal,
    pub selecao: SelecaoDeJogo,
    /// O "Select a Game" está aberto sobre a partida (a simulação fica pausada).
    pub escolhendo: bool,
    /// Menu de ferramentas do Experimental (botão ribTools).
    pub menu: MenuFerramentas,
    pub menu_aberto: bool,
    pub tabelas_exp: TabelasExperimental,
    /// `_ExpSubStates` e o arrasto de `_processExp`.
    pub controle_exp: ControleExperimental,
}

impl Modos {
    pub fn carregar(recursos: &Recursos, interface: &Interface) -> Result<Modos, String> {
        let cfg = ConfigQuintalMapa::carregar(recursos)?;
        let quintal = DesenhoQuintal::carregar(cfg, recursos, interface)?;
        let selecao = SelecaoDeJogo::carregar(recursos.dados::<ConfigModos>(engine::core::selecao::ARQUIVO_CONFIG)?, recursos, interface)?;
        let tabelas_exp = TabelasExperimental::carregar(recursos)?;
        let controle_exp = ControleExperimental::novo(&tabelas_exp);
        let menu = MenuFerramentas::carregar(recursos)?;
        Ok(Modos { quintal, janela: JanelaQuintal::default(), selecao, escolhendo: false, menu, menu_aberto: false, tabelas_exp, controle_exp })
    }
}

/// "Select a Game" da abertura (com a interface da abertura).
pub fn selecao_da_abertura(recursos: &Recursos, interface: &Interface) -> Result<SelecaoDeJogo, String> {
    SelecaoDeJogo::carregar(recursos.dados::<ConfigModos>(engine::core::selecao::ARQUIVO_CONFIG)?, recursos, interface)
}

/// A faixa do momento: com o quintal ativado por último, a 35 no lugar da 34; senão, na 34, o botão
/// de ferramentas do Experimental. Devolve o que não foi desenhado.
pub fn desenhar_faixa(s: &Sessao, q: &mut Quadro) -> Vec<String> {
    let m = &s.modos;
    if m.janela.na_frente {
        return m.quintal.desenhar_faixa(q, s.recursos, &s.interface, &s.partida.estado);
    }
    // _NewGame: no Experimental o botão de ferramentas da faixa usa outros bitmaps.
    if s.partida.estado.partida.tipo_jogo == 3 {
        if let Some(r) = s.tela.retangulo_da_faixa(&m.menu.cfg.botao) {
            let id = m.menu.cfg.bitmaps_botao_experimental[1];
            if let Ok(b) = s.recursos.objeto(engine::formats::banco::Chave { tipo: engine::formats::bitmap::TIPO_BITMAP, id }).and_then(|d| engine::formats::bitmap::Bitmap::decodificar(&d)) {
                q.bitmap(&b, i32::from(r.esq), i32::from(r.topo));
            }
        }
    }
    Vec::new()
}

/// Desenha, por cima de tudo, o menu de ferramentas do Experimental e o "Select a Game" do Ctrl+N.
/// (O quintal é uma janela do gerenciador: `Sessao::desenhar_janelas`.) Devolve o que não foi
/// desenhado.
pub fn desenhar(s: &Sessao, q: &mut Quadro) -> Result<Vec<String>, String> {
    let m = &s.modos;
    let mut pendentes = Vec::new();
    if m.menu_aberto {
        let atual = Ferramenta::de_i16(s.partida.estado.partida.ferramenta_experimental);
        pendentes.extend(m.menu.desenhar(q, s.recursos, &s.interface, atual));
    }
    if m.escolhendo {
        pendentes.extend(desenhar_janela_na_tela(q, s.recursos, &s.interface, m.selecao.cfg.janela)?);
    }
    Ok(pendentes)
}

/// O que fazer depois de uma entrada tratada aqui.
pub enum Resposta {
    /// Não era para os modos: segue o tratamento normal da tela de jogo.
    Seguir,
    /// Tratada.
    Tratada,
    /// O jogador escolheu um jogo novo no "Select a Game" do Ctrl+N.
    NovoJogo(Escolha),
}

/// Tecla na tela de jogo: Ctrl+N (File → New Game), teclas do "Select a Game" aberto e atalhos do
/// quintal (Ctrl+Y/5/6/7, Ctrl+M).
pub fn tecla(s: &mut Sessao, ev: &EventoNav) -> Resposta {
    let m = &mut s.modos;
    if m.escolhendo {
        return match m.selecao.cfg.escolha_da_tecla(ev) {
            Some(e) => escolher(s, e),
            None => Resposta::Tratada,
        };
    }
    if m.selecao.cfg.novo_jogo.bate(ev) {
        m.escolhendo = true;
        println!("File > New Game: Select a Game");
        return Resposta::Tratada;
    }
    if let Some(a) = m.quintal.cfg.acao_da_tecla(ev) {
        if a == AcaoQuintal::Abrir && !s.janelas.aberta(Controle::Quintal) {
            // _OpenMapYard: sem o quintal aberto, Ctrl+M abre a janela do mapa (_YardToMap).
            s.abrir_mapa();
            println!("mapa aberto (modo {})", s.partida.estado.vistas.modo_mapa);
            return Resposta::Tratada;
        }
        aplicar_acao_quintal(s, a, true);
        return Resposta::Tratada;
    }
    Resposta::Seguir
}

fn escolher(s: &mut Sessao, e: Escolha) -> Resposta {
    s.modos.escolhendo = false;
    match e {
        Escolha::Cancelar => Resposta::Tratada,
        Escolha::Carregar => {
            // Load Game ainda não existe no port (diálogo de arquivo): fica como estava.
            println!("Load Game ainda não implementado no port");
            Resposta::Tratada
        }
        novo => Resposta::NovoJogo(novo),
    }
}

/// Os efeitos das ferramentas viram pedidos da fila (som, música) ou linhas no terminal.
fn tratar_efeitos(s: &mut Sessao, efeitos: Vec<Efeito>) {
    for e in efeitos {
        match e {
            Efeito::Som { id, .. } => s.partida.estado.eventos.push(Evento::Som(id)),
            Efeito::Musica(id) => s.partida.estado.eventos.push(Evento::Musica(id)),
            Efeito::Aviso { lista, indice } => println!("aviso: texto {indice} da lista {lista}"),
            Efeito::Examinar(x) => s.abrir_exame(x),
        }
    }
    s.partida.processar_eventos_do_estado();
}

/// Botão de ferramentas (`_RibbonToolsMenu`): Quick Game → Experimental; no Experimental abre o
/// menu; no Full Game e no Tutorial, nada (o original mostra só uma figura no Full Game).
fn botao_ferramentas(s: &mut Sessao) {
    match s.partida.estado.partida.tipo_jogo {
        1 => {
            let c = Contexto { tabelas: &s.modos.tabelas_exp, amarela: &s.partida.tabelas_amarela, inimigos: &s.partida.regras.inimigos };
            let e = &mut s.partida.estado;
            let efeitos = experimental::quick_para_experimental(e, c);
            println!("Quick Game -> Experimental");
            tratar_efeitos(s, efeitos);
        }
        3 => {
            if let Some(r) = s.tela.retangulo_da_faixa(&s.modos.menu.cfg.botao) {
                s.modos.menu.abrir(r);
                s.modos.menu_aberto = true;
            }
        }
        _ => println!("ferramentas: nada neste tipo de jogo"),
    }
}

fn clique_no_menu(s: &mut Sessao, x: i32, y: i32) {
    if !s.modos.menu.contem(x, y) {
        s.modos.menu_aberto = false;
        return;
    }
    let Some(item) = s.modos.menu.item_em(x, y) else { return };
    let plano = i32::from(s.partida.nav.plano_mapa);
    let (f, sub, fecha) = match item {
        ItemMenu::Ferramenta(f) => (f, None, s.modos.menu.submenu.is_none()),
        ItemMenu::Subopcao(f, i) => (f, Some(i), true),
    };
    let mut ctl = s.modos.controle_exp.clone();
    let c = Contexto { tabelas: &s.modos.tabelas_exp, amarela: &s.partida.tabelas_amarela, inimigos: &s.partida.regras.inimigos };
    let e = &mut s.partida.estado;
    let efeitos = experimental::escolher_ferramenta(e, c, &mut ctl, f, sub, plano);
    s.modos.controle_exp = ctl;
    println!("ferramenta {f:?} (subopção {:?})", sub.unwrap_or(s.modos.controle_exp.subopcao(f)));
    if fecha {
        s.modos.menu_aberto = false;
    }
    tratar_efeitos(s, efeitos);
}

/// Experimental: botão apertado (`pressionar`) ou arrasto (`arrastar`) numa célula do mapa.
fn ferramenta_no_mapa(s: &mut Sessao, x: i32, y: i32, ctrl: bool, arrasto: bool) -> bool {
    let Some((cx, cy)) = s.tela.celula_em(x, y, &s.partida.nav) else { return false };
    let plano = i32::from(s.partida.nav.plano_mapa);
    let mut ctl = s.modos.controle_exp.clone();
    let c = Contexto { tabelas: &s.modos.tabelas_exp, amarela: &s.partida.tabelas_amarela, inimigos: &s.partida.regras.inimigos };
    let e = &mut s.partida.estado;
    let efeitos = if arrasto {
        experimental::arrastar(e, c, &mut ctl, plano, cx, cy, ctrl)
    } else {
        experimental::pressionar(e, c, &mut ctl, plano, cx, cy, ctrl)
    };
    s.modos.controle_exp = ctl;
    tratar_efeitos(s, efeitos);
    true
}

/// Mouse andando com o botão esquerdo apertado: arrasto da ferramenta do Experimental.
pub fn mover(s: &mut Sessao, x: i32, y: i32, ctrl: bool) -> bool {
    if s.modos.controle_exp.atual.is_none() || s.partida.estado.partida.tipo_jogo != 3 {
        return false;
    }
    ferramenta_no_mapa(s, x, y, ctrl, true)
}

/// Botão solto: fim do arrasto.
pub fn soltar(s: &mut Sessao) {
    experimental::soltar(&mut s.modos.controle_exp);
}

/// Clique com um "popup" dos modos aberto ("Select a Game" do Ctrl+N, menu de ferramentas do
/// Experimental): eles ficam por cima das janelas e recebem o clique antes delas.
pub fn clique_popup(s: &mut Sessao, x: i32, y: i32) -> Resposta {
    if s.modos.escolhendo {
        return match s.modos.selecao.escolha_em(x, y) {
            Some(e) => escolher(s, e),
            None => Resposta::Tratada,
        };
    }
    if s.modos.menu_aberto {
        clique_no_menu(s, x, y);
        return Resposta::Tratada;
    }
    Resposta::Seguir
}

/// Uma ação do quintal (tecla ou botão da faixa) com o gerenciador de janelas: o Patch é
/// `_YardToMap`; abrir ou trocar de modo por tecla traz o quintal para a frente (`_MapToYard`).
pub fn aplicar_acao_quintal(s: &mut Sessao, a: AcaoQuintal, trazer: bool) {
    match a {
        AcaoQuintal::ParaMapa => s.quintal_para_mapa(),
        AcaoQuintal::Fechar => {
            let id = s.janelas.cfg.quintal;
            s.fechar_janela(id, false);
        }
        _ => {
            let aberta = s.janelas.aberta(Controle::Quintal);
            s.modos.janela.aplicar(a, &mut s.partida.estado);
            let abre = matches!(
                a,
                AcaoQuintal::Abrir | AcaoQuintal::ModoQuintal | AcaoQuintal::ModoCasa | AcaoQuintal::ModoEstrategico | AcaoQuintal::ModoGrafico | AcaoQuintal::AlternarCasa
            );
            if abre && (trazer || !aberta) {
                s.abrir_quintal();
            }
        }
    }
    println!("quintal: {a:?} (modo {})", s.partida.estado.vistas.modo_quintal);
}

/// Clique fora das janelas (na faixa): botão de ferramentas e botão Yard na faixa 34; os botões da
/// faixa 35 quando ela está na tela.
pub fn clique_na_faixa(s: &mut Sessao, x: i32, y: i32) -> Resposta {
    if s.modos.janela.na_frente {
        // Faixa 35 no lugar da 34.
        if y < i32::from(s.interface.moldura.faixa.altura) {
            if let Some(a) = s.modos.quintal.acao_da_faixa_em(x, y) {
                aplicar_acao_quintal(s, a, false);
            }
            return Resposta::Tratada;
        }
        return Resposta::Seguir;
    }
    let objetos = s.tela.objetos_da_faixa_em(x, y);
    if objetos.contains(&s.modos.menu.cfg.botao.as_str()) {
        botao_ferramentas(s);
        return Resposta::Tratada;
    }
    if objetos.contains(&s.modos.quintal.cfg.botao_abrir.as_str()) {
        aplicar_acao_quintal(s, AcaoQuintal::Abrir, true);
        return Resposta::Tratada;
    }
    Resposta::Seguir
}

/// Clique na área cliente da janela de edição: no Experimental, vai para a ferramenta
/// (`_processExp`). Devolve se tratou.
pub fn clique_na_edicao(s: &mut Sessao, x: i32, y: i32, ctrl: bool) -> bool {
    s.partida.estado.partida.tipo_jogo == 3 && ferramenta_no_mapa(s, x, y, ctrl, false)
}

/// Clique na área cliente do quintal (que é a janela da frente): patch, troca de patch, PLACE.
pub fn clique_no_quintal(s: &mut Sessao, x: i32, y: i32, duplo: bool) {
    let m = &mut s.modos;
    match quintal_mapa::clicar(&mut m.janela, &m.quintal.cfg, &mut s.partida, x, y, duplo) {
        Ok(r) => println!("quintal: clique {} em ({x}, {y}): {r:?}", if duplo { "duplo" } else { "simples" }),
        Err(e) => eprintln!("quintal: {e}"),
    }
}
