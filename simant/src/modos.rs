//! Modos de jogo na interface SDL: escolha no "Select a Game" (na abertura e com File → New Game,
//! Ctrl+N, durante a partida), janela do quintal (Yard View, faixa 35) com a escolha e a troca de
//! patch. A lógica fica no engine (`core::selecao`, `core::quintal_mapa`, `rules::troca_patch`);
//! aqui só se liga a entrada do SDL e o desenho por cima da tela de jogo.
//!
//! Especificação: docs/spec/telas/abertura.md ("Escolha do jogo") e docs/spec/telas/quintal.md.

use engine::core::menu_ferramentas::{ItemMenu, MenuFerramentas};
use engine::core::navegacao::Evento as EventoNav;
use engine::core::quadro::Quadro;
use engine::core::quintal_mapa::{self, ConfigQuintalMapa, DesenhoQuintal, JanelaQuintal};
use engine::core::selecao::{ConfigModos, Escolha, SelecaoDeJogo};
use engine::core::telas::{desenhar_janela_na_tela, Interface};
use engine::formats::janela::Retangulo;
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

/// Desenha, por cima da tela de jogo já composta, a faixa 35 e a janela do quintal (na frente da
/// janela de edição ou atrás dela) e o "Select a Game" do Ctrl+N. Devolve o que não foi desenhado.
pub fn desenhar(s: &Sessao, q: &mut Quadro) -> Result<Vec<String>, String> {
    let m = &s.modos;
    let mut pendentes = Vec::new();
    if m.janela.aberta {
        pendentes.extend(m.quintal.desenhar_faixa(q, s.recursos, &s.interface, &s.partida.estado));
        let fonte_balao = s.interface.fontes.get(m.quintal.cfg.balao_menino.fonte).and_then(|f| f.as_ref());
        if m.janela.na_frente {
            m.quintal.desenhar_janela(q, &s.partida.estado);
            m.quintal.desenhar_balao_do_menino(q, &s.partida.estado, fonte_balao);
        } else {
            // Atrás da janela de edição: só a parte fora dela aparece.
            let [x, y, w, h] = s.tela.cfg.edicao.moldura.externo;
            let edicao = Retangulo { esq: x, topo: y, dir: x + w, base: y + h };
            let guardado = q.pixels.clone();
            m.quintal.desenhar_janela(q, &s.partida.estado);
            m.quintal.desenhar_balao_do_menino(q, &s.partida.estado, fonte_balao);
            for yy in edicao.topo.max(0) as usize..(edicao.base.max(0) as usize).min(q.altura) {
                for xx in edicao.esq.max(0) as usize..(edicao.dir.max(0) as usize).min(q.largura) {
                    let i = yy * q.largura + xx;
                    q.pixels[i] = guardado[i];
                }
            }
        }
    }
    // _NewGame: no Experimental o botão de ferramentas da faixa usa outros bitmaps.
    if s.partida.estado.partida.tipo_jogo == 3 && !m.janela.aberta {
        if let Some(r) = s.tela.retangulo_da_faixa(&m.menu.cfg.botao) {
            let id = m.menu.cfg.bitmaps_botao_experimental[1];
            if let Ok(b) = s.recursos.objeto(engine::formats::banco::Chave { tipo: engine::formats::bitmap::TIPO_BITMAP, id }).and_then(|d| engine::formats::bitmap::Bitmap::decodificar(&d)) {
                q.bitmap(&b, i32::from(r.esq), i32::from(r.topo));
            }
        }
    }
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
        if a == quintal_mapa::AcaoQuintal::Abrir && !m.janela.aberta {
            // _OpenMapYard: sem o quintal aberto, Ctrl+M abre a janela do mapa (_YardToMap).
            s.abrir_mapa();
            println!("mapa aberto (modo {})", s.partida.estado.vistas.modo_mapa);
            return Resposta::Tratada;
        }
        m.janela.aplicar(a, &mut s.partida.estado);
        println!("quintal: {a:?} (modo {})", s.partida.estado.vistas.modo_quintal);
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

/// Clique (esquerdo ou direito) em (x, y) da área cliente. `duplo`: duplo clique do SDL; `ctrl`: Ctrl
/// apertado (inverte muro, cavar, comida e cheiro no Experimental).
pub fn clique(s: &mut Sessao, x: i32, y: i32, duplo: bool, ctrl: bool) -> Resposta {
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
    if !s.modos.janela.aberta && s.tela.objetos_da_faixa_em(x, y).contains(&s.modos.menu.cfg.botao.as_str()) {
        botao_ferramentas(s);
        return Resposta::Tratada;
    }
    let m = &mut s.modos;
    if !m.janela.aberta {
        let abre = s.tela.objetos_da_faixa_em(x, y).contains(&m.quintal.cfg.botao_abrir.as_str());
        if abre {
            m.janela.aplicar(quintal_mapa::AcaoQuintal::Abrir, &mut s.partida.estado);
            println!("quintal aberto (modo {})", s.partida.estado.vistas.modo_quintal);
            return Resposta::Tratada;
        }
        if s.partida.estado.partida.tipo_jogo == 3 && ferramenta_no_mapa(s, x, y, ctrl, false) {
            return Resposta::Tratada;
        }
        return Resposta::Seguir;
    }
    // Faixa 35 no lugar da 34.
    if y < i32::from(s.interface.moldura.faixa.altura) {
        if let Some(a) = m.quintal.acao_da_faixa_em(x, y) {
            m.janela.aplicar(a, &mut s.partida.estado);
            println!("quintal: {a:?} (modo {})", s.partida.estado.vistas.modo_quintal);
        }
        return Resposta::Tratada;
    }
    let cfg = &m.quintal.cfg;
    let [ex, ey, ew, eh] = s.tela.cfg.edicao.moldura.externo;
    let na_edicao = x >= i32::from(ex) && x < i32::from(ex + ew) && y >= i32::from(ey) && y < i32::from(ey + eh);
    let no_quintal = cfg.contem(x, y) && (m.janela.na_frente || !na_edicao);
    if !no_quintal {
        // Clique fora: a janela de edição (se for ela) vem para a frente e recebe o clique.
        if na_edicao {
            m.janela.na_frente = false;
        }
        // _processExp: no Experimental, todo clique no mapa vai para a ferramenta.
        if s.partida.estado.partida.tipo_jogo == 3 && ferramenta_no_mapa(s, x, y, ctrl, false) {
            return Resposta::Tratada;
        }
        return Resposta::Seguir;
    }
    match quintal_mapa::clicar(&mut m.janela, cfg, &mut s.partida, x, y, duplo) {
        Ok(r) => println!("quintal: clique {} em ({x}, {y}): {r:?}", if duplo { "duplo" } else { "simples" }),
        Err(e) => eprintln!("quintal: {e}"),
    }
    Resposta::Tratada
}
