//! Modos de jogo na interface SDL: escolha no "Select a Game" (na abertura e com File → New Game,
//! Ctrl+N, durante a partida), janela do quintal (Yard View, faixa 35) com a escolha e a troca de
//! patch. A lógica fica no engine (`core::selecao`, `core::quintal_mapa`, `rules::troca_patch`);
//! aqui só se liga a entrada do SDL e o desenho por cima da tela de jogo.
//!
//! Especificação: docs/spec/telas/abertura.md ("Escolha do jogo") e docs/spec/telas/quintal.md.

use engine::core::navegacao::Evento as EventoNav;
use engine::core::quadro::Quadro;
use engine::core::quintal_mapa::{self, ConfigQuintalMapa, DesenhoQuintal, JanelaQuintal};
use engine::core::selecao::{ConfigModos, Escolha, SelecaoDeJogo};
use engine::core::telas::{desenhar_janela_na_tela, Interface};
use engine::formats::janela::Retangulo;
use engine::recursos::Recursos;

use super::Sessao;

/// Estado dos modos na tela de jogo: janela do quintal e "Select a Game" aberto pelo Ctrl+N.
pub struct Modos {
    pub quintal: DesenhoQuintal,
    pub janela: JanelaQuintal,
    pub selecao: SelecaoDeJogo,
    /// O "Select a Game" está aberto sobre a partida (a simulação fica pausada).
    pub escolhendo: bool,
}

impl Modos {
    pub fn carregar(recursos: &Recursos, interface: &Interface) -> Result<Modos, String> {
        let cfg = ConfigQuintalMapa::carregar(recursos)?;
        let quintal = DesenhoQuintal::carregar(cfg, recursos, interface)?;
        let selecao = SelecaoDeJogo::carregar(recursos.dados::<ConfigModos>(engine::core::selecao::ARQUIVO_CONFIG)?, recursos, interface)?;
        Ok(Modos { quintal, janela: JanelaQuintal::default(), selecao, escolhendo: false })
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
        if m.janela.na_frente {
            m.quintal.desenhar_janela(q, &s.partida.estado);
        } else {
            // Atrás da janela de edição: só a parte fora dela aparece.
            let [x, y, w, h] = s.tela.cfg.edicao.moldura.externo;
            let edicao = Retangulo { esq: x, topo: y, dir: x + w, base: y + h };
            let guardado = q.pixels.clone();
            m.quintal.desenhar_janela(q, &s.partida.estado);
            for yy in edicao.topo.max(0) as usize..(edicao.base.max(0) as usize).min(q.altura) {
                for xx in edicao.esq.max(0) as usize..(edicao.dir.max(0) as usize).min(q.largura) {
                    let i = yy * q.largura + xx;
                    q.pixels[i] = guardado[i];
                }
            }
        }
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

/// Clique esquerdo em (x, y) da área cliente. `duplo`: duplo clique do SDL.
pub fn clique(s: &mut Sessao, x: i32, y: i32, duplo: bool) -> Resposta {
    if s.modos.escolhendo {
        return match s.modos.selecao.escolha_em(x, y) {
            Some(e) => escolher(s, e),
            None => Resposta::Tratada,
        };
    }
    let m = &mut s.modos;
    if !m.janela.aberta {
        let abre = s.tela.objetos_da_faixa_em(x, y).contains(&m.quintal.cfg.botao_abrir.as_str());
        if abre {
            m.janela.aplicar(quintal_mapa::AcaoQuintal::Abrir, &mut s.partida.estado);
            println!("quintal aberto (modo {})", s.partida.estado.vistas.modo_quintal);
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
        return Resposta::Seguir;
    }
    match quintal_mapa::clicar(&mut m.janela, cfg, &mut s.partida, x, y, duplo) {
        Ok(r) => println!("quintal: clique {} em ({x}, {y}): {r:?}", if duplo { "duplo" } else { "simples" }),
        Err(e) => eprintln!("quintal: {e}"),
    }
    Resposta::Tratada
}
