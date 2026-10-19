//! Gerenciador de janelas na interface SDL: liga `engine::core::gerenciador` (por `JanelasControle`)
//! à sessão — roteamento do clique só para a janela da frente, moldura (mover, redimensionar,
//! maximizar, menu de sistema, fechar), barras de rolagem da janela de edição, mapa ↔ quintal e a
//! faixa que acompanha a janela ativa. Especificação: docs/spec/telas/janelas.md §8.

use super::*;
use engine::core::gerenciador::{Barra, Clique, ComandoRolagem, Rolagem};
use engine::core::janelas::{AcaoJanela, ID_EDICAO};

/// Para onde vai o clique esquerdo depois do gerenciador.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Roteamento {
    /// Tratado (moldura, conteúdo de janela de controle, janela de trás trazida para a frente).
    Tratado,
    /// Fora das janelas: faixa ou fundo.
    Fora,
    /// Área cliente da janela de edição, que é a da frente.
    Edicao,
    /// Área cliente do quintal, que é a da frente.
    Quintal,
    /// Seta da barra da janela de edição: a ação, que se repete com o botão apertado.
    Linha(Acao),
}

impl Sessao<'_> {
    /// Uma janela aberta por `_SetDefaultWindows`: o mapa gera o buffer; o quintal fica aberto.
    pub(crate) fn depois_de_abrir(&mut self, c: Controle) {
        match c {
            Controle::Mapa => self.mapa.atualizar(&self.partida.estado, self.partida.mostrar_trilhas),
            Controle::Quintal => self.modos.janela.aberta = true,
            _ => {}
        }
    }

    /// Jogo novo ou Load durante a partida: as janelas do jogo anterior ficam onde estavam
    /// (`_NewGame`/`_LoadGame` não as fecham) e `_SetDefaultWindows` roda de novo.
    pub(crate) fn herdar_janelas(&mut self, antiga: &Sessao) {
        self.janelas.gerente = antiga.janelas.gerente.clone();
        self.janelas.gerente.arrasto = None;
        self.janelas.gerente.menu = None;
        self.modos.janela.na_frente = antiga.modos.janela.na_frente;
        self.mini.reaberto = antiga.mini.reaberto;
        if let Some(c) = self.janelas.janelas_padrao(self.partida.nav.plano_mapa) {
            self.depois_de_abrir(c);
        }
        self.sincronizar_janelas();
    }

    /// O gerenciador manda: a moldura da janela de edição (e a vista, se o tamanho mudou), os
    /// polegares, o contorno do quintal e o "aberto" dele.
    pub(crate) fn sincronizar_janelas(&mut self) {
        if let Some(j) = self.janelas.gerente.janela(ID_EDICAO).copied() {
            if j.externo != self.tela.cfg.edicao.moldura.retangulo() {
                let (w, h) = self.tela.ajustar_moldura(j.externo);
                if self.partida.nav.redimensionar_vista(w, h) {
                    println!("janela de edição: vista de {w} × {h}");
                }
            }
        }
        let (mx, my) = self.partida.nav.maximo();
        let v = self.partida.nav.vista;
        let (h, vv) = (Rolagem { pos: v.x, max: mx }, Rolagem { pos: v.y, max: my });
        self.janelas.gerente.definir_rolagem(ID_EDICAO, h, vv);
        let g = &self.janelas.gerente;
        let edicao = g.janela(ID_EDICAO);
        self.tela.estado_moldura = engine::core::tela_jogo::EstadoMoldura {
            aberta: edicao.is_some(),
            ativa: g.topo() == Some(ID_EDICAO),
            maximizada: edicao.is_some_and(|j| j.maximizada()),
            rolagem: [h, vv],
        };
        let quintal = self.janelas.cfg.quintal;
        self.modos.janela.aberta = g.aberta(quintal);
        if let Some(j) = g.janela(quintal) {
            let e = j.externo;
            self.modos.quintal.cfg.externo = [e.esq, e.topo, e.dir - e.esq, e.base - e.topo];
        }
    }

    /// Depois de um evento do gerenciador: a faixa segue a janela ativada (`WM_NCACTIVATE` em
    /// `MAINWNDPROC`: a de edição ativa troca 35 → 34, o quintal ativo troca 34 → 35) e o resto
    /// acompanha.
    fn depois_do_gerente(&mut self, topo_antes: Option<u16>) {
        let topo = self.janelas.gerente.topo();
        if topo != topo_antes {
            if topo == Some(ID_EDICAO) {
                self.modos.janela.na_frente = false;
            } else if topo == Some(self.janelas.cfg.quintal) {
                self.modos.janela.na_frente = true;
            }
        }
        self.sincronizar_janelas();
    }

    /// Clique esquerdo: o gerenciador decide quem recebe (janelas.md §8.3).
    pub(crate) fn clicar_nas_janelas(&mut self, x: i32, y: i32, duplo: bool) -> Roteamento {
        self.sincronizar_janelas();
        let antes = self.janelas.gerente.topo();
        let c = match self.janelas.clicar(self.recursos, &mut self.partida.estado, x, y, duplo) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("janelas: {e}");
                Clique::Nada
            }
        };
        let r = self.tratar_clique_do_gerente(c);
        self.depois_do_gerente(antes);
        r
    }

    /// Clique direito: a janela sob o cursor vem para a frente e recebe o clique (`WM_MOUSEACTIVATE`
    /// só come o esquerdo). Devolve se caiu na área do mapa da janela de edição.
    pub(crate) fn clicar_direito_nas_janelas(&mut self, x: i32, y: i32) -> bool {
        self.sincronizar_janelas();
        let antes = self.janelas.gerente.topo();
        if self.janelas.gerente.menu.take().is_some() {
            self.depois_do_gerente(antes);
            return false;
        }
        let Some(id) = self.janelas.gerente.janela_em(x, y) else { return false };
        self.janelas.gerente.trazer_para_frente(id);
        let cliente = self
            .janelas
            .gerente
            .janela(id)
            .is_some_and(|j| engine::core::gerenciador::parte_em(j, x, y) == Some(engine::core::gerenciador::Parte::Cliente));
        self.depois_do_gerente(antes);
        id == ID_EDICAO && cliente
    }

    fn tratar_clique_do_gerente(&mut self, c: Clique) -> Roteamento {
        let quintal = self.janelas.cfg.quintal;
        match c {
            Clique::Fora => Roteamento::Fora,
            Clique::Cliente(ID_EDICAO) => Roteamento::Edicao,
            Clique::Cliente(id) if id == quintal => Roteamento::Quintal,
            Clique::Cliente(_) => {
                self.depois_do_clique_em_controle();
                Roteamento::Tratado
            }
            Clique::Fechar(id) => {
                self.fechar_janela(id, true);
                Roteamento::Tratado
            }
            Clique::Rolar(ID_EDICAO, barra, cmd) => match self.rolar_edicao(barra, cmd) {
                Some(a) if a.repete() => Roteamento::Linha(a),
                _ => Roteamento::Tratado,
            },
            Clique::Ativou(id) => {
                println!("janela {id} na frente");
                Roteamento::Tratado
            }
            Clique::Tamanho(_) | Clique::Moveu(_) | Clique::Nada | Clique::Rolar(..) => Roteamento::Tratado,
        }
    }

    /// Mensagem de rolagem para a janela de edição (`_DoEditScroll`). Devolve a ação de linha ou de
    /// página aplicada (a de linha se repete enquanto o botão fica apertado).
    fn rolar_edicao(&mut self, barra: Barra, cmd: ComandoRolagem) -> Option<Acao> {
        let h = barra == Barra::Horizontal;
        let acao = match cmd {
            ComandoRolagem::LinhaMenos => Some(if h { Acao::LinhaEsquerda } else { Acao::LinhaAcima }),
            ComandoRolagem::LinhaMais => Some(if h { Acao::LinhaDireita } else { Acao::LinhaAbaixo }),
            ComandoRolagem::PaginaMenos => Some(if h { Acao::PaginaEsquerda } else { Acao::PaginaAcima }),
            ComandoRolagem::PaginaMais => Some(if h { Acao::PaginaDireita } else { Acao::PaginaAbaixo }),
            ComandoRolagem::Posicao(p) => {
                self.partida.nav.ir_para_posicao(h, p);
                None
            }
            ComandoRolagem::Acompanhar(p) => {
                // SB_THUMBTRACK: a vista só segue o polegar com AutoTrack (`_OptionStates[0]`) [código
                // + bancada x01].
                if self.partida.estado.partida.opcoes[0] != 0 {
                    self.partida.nav.ir_para_posicao(h, p);
                }
                None
            }
        };
        if let Some(a) = acao {
            self.partida.navegar(a);
        }
        acao
    }

    /// Mouse andando: arrastos e realce do menu do gerenciador. Devolve se é preciso redesenhar.
    pub(crate) fn mover_nas_janelas(&mut self, x: i32, y: i32) -> bool {
        if self.janelas.gerente.arrasto.is_none() && self.janelas.gerente.menu.is_none() {
            return false;
        }
        let (redesenha, msg) = self.janelas.gerente.mover_mouse(x, y);
        if let Some(Clique::Rolar(ID_EDICAO, barra, cmd)) = msg {
            self.rolar_edicao(barra, cmd);
        }
        self.sincronizar_janelas();
        redesenha || msg.is_some()
    }

    /// Botão solto: termina o arrasto ou escolhe no menu de sistema. Devolve se algo mudou.
    pub(crate) fn soltar_nas_janelas(&mut self, x: i32, y: i32) -> bool {
        let g = &self.janelas.gerente;
        if g.arrasto.is_none() && g.menu.is_none() {
            return false;
        }
        let antes = g.topo();
        let c = self.janelas.gerente.soltar(x, y);
        self.tratar_clique_do_gerente(c);
        self.depois_do_gerente(antes);
        true
    }

    /// Tecla: com o menu de sistema aberto (ou num Move/Size pelo teclado), vai para ele; senão os
    /// aceleradores de janela (Ctrl+F4, Ctrl+F6, Ctrl+Espaço, Ctrl+P, Ctrl+Z). Devolve se tratou.
    pub(crate) fn tecla_de_janela(&mut self, ev: &EventoNav) -> bool {
        let EventoNav::Tecla { nome, ctrl, shift, alt } = ev;
        let antes = self.janelas.gerente.topo();
        if let Some(c) = self.janelas.gerente.tecla(nome) {
            self.tratar_clique_do_gerente(c);
            self.depois_do_gerente(antes);
            return true;
        }
        let Some(a) = self.janelas.cfg.acao_da_tecla(nome, *ctrl, *shift, *alt) else { return false };
        let g = &mut self.janelas.gerente;
        match a {
            AcaoJanela::FecharDeCima => {
                if let Some(id) = g.topo() {
                    // 0xFDA0: WM_CLOSE direto (sem o SC_CLOSE, então o quintal não vira mapa).
                    self.fechar_janela(id, false);
                }
            }
            AcaoJanela::MenuSistema => {
                if let Some(id) = g.topo() {
                    g.abrir_menu(id);
                }
            }
            AcaoJanela::Proxima => {
                g.proxima();
            }
            AcaoJanela::Mover => {
                if let Some(id) = g.topo() {
                    g.comando(id, engine::core::gerenciador::ComandoSistema::Mover);
                }
            }
            AcaoJanela::MaximizarEdicao => {
                if g.topo() == Some(ID_EDICAO) {
                    g.alternar_maximizar(ID_EDICAO);
                }
            }
        }
        println!("janela: {a:?}");
        self.depois_do_gerente(antes);
        true
    }

    /// Fecha a janela `id`. `sistema`: pelo `SC_CLOSE` (duplo clique na caixa, Close do menu), que
    /// no quintal chama `_YardToMap` antes (`MAINWNDPROC`): o quintal vira o mapa, que é fechado.
    pub(crate) fn fechar_janela(&mut self, id: u16, sistema: bool) {
        let antes = self.janelas.gerente.topo();
        let mut alvo = id;
        if sistema && id == self.janelas.cfg.quintal {
            self.quintal_para_mapa();
            alvo = self.janelas.cfg.id(Controle::Mapa);
        }
        self.janelas.gerente.fechar(alvo);
        println!("janela {id} fechada");
        self.depois_do_gerente(antes);
    }

    /// `_MapToYard` (botão Yard, Ctrl+Y/5/6/7): com o mapa aberto e o quintal não, `_SetMapPlane(0)`
    /// (a edição passa à superfície, sem centralizar) e o quintal toma o lugar do mapa
    /// (`_win_Swap`); senão só abre o quintal. Vem para a frente com a faixa 35 [medido em w20, p01].
    pub(crate) fn abrir_quintal(&mut self) {
        let antes = self.janelas.gerente.topo();
        if !self.janelas.aberta(Controle::Quintal) && self.janelas.aberta(Controle::Mapa) {
            self.partida.ir_para_quintal();
            self.janelas.trocar(Controle::Mapa, Controle::Quintal);
        }
        self.janelas.abrir(Controle::Quintal);
        self.modos.janela.na_frente = true;
        self.depois_do_gerente(antes);
    }

    /// `_YardToMap` (botão Patch da faixa 35, `SC_CLOSE` do quintal): com o quintal aberto e o mapa
    /// não, o mapa toma o lugar dele (`_win_Swap`), `_SetMapPlane(1)` e a faixa 34; senão abre o mapa.
    pub(crate) fn quintal_para_mapa(&mut self) {
        let antes = self.janelas.gerente.topo();
        if self.janelas.aberta(Controle::Quintal) && !self.janelas.aberta(Controle::Mapa) {
            self.janelas.trocar(Controle::Quintal, Controle::Mapa);
            self.partida.navegar(Acao::Superficie);
            self.modos.janela.na_frente = false;
        } else {
            self.janelas.abrir(Controle::Mapa);
        }
        self.mapa.atualizar(&self.partida.estado, self.partida.mostrar_trilhas);
        println!("Patch: mapa da superfície (plano {})", self.partida.nav.plano_mapa);
        self.depois_do_gerente(antes);
    }

    /// Desenha as janelas `ids` (de baixo para cima), menos a de edição, que é da `TelaDeJogo`.
    pub(crate) fn desenhar_janelas(&self, q: &mut Quadro, ids: &[u16]) -> Result<Vec<String>, String> {
        let (recursos, interface, estado, janelas) = (self.recursos, &self.interface, &self.partida.estado, &self.janelas);
        let mut p = Vec::new();
        for &id in ids {
            let Some(c) = janelas.controle(id) else { continue };
            let ativa = janelas.gerente.topo() == Some(id);
            match c {
                Controle::Info => {
                    let g = janelas.geometria(recursos, c)?;
                    let titulo = self.info.carta.as_ref().map(|k| k.titulo.clone()).unwrap_or_default();
                    janelas::desenhar_moldura_com_titulo(q, interface, &janelas.cfg, &g, &titulo, ativa);
                    if let Some(area) = janelas.area_do_info(recursos) {
                        self.info.desenhar(q, recursos, interface, area);
                    }
                }
                Controle::Mapa => {
                    let g = janelas.geometria(recursos, c)?;
                    janelas::desenhar_moldura_com_titulo(q, interface, &janelas.cfg, &g, &self.mapa.titulo(estado), ativa);
                    if let Some(area) = janelas.area_do_mapa(recursos) {
                        self.mapa.desenhar(q, area, estado, &self.partida.nav, self.partida.mostrar_trilhas);
                    }
                }
                Controle::Quintal => {
                    let g = janelas.geometria(recursos, c)?;
                    let m = &self.modos;
                    m.quintal.desenhar_janela(q, estado);
                    let fonte_balao = interface.fontes.get(m.quintal.cfg.balao_menino.fonte).and_then(|f| f.as_ref());
                    m.quintal.desenhar_balao_do_menino(q, estado, fonte_balao);
                    // _UpdateYardMessage: a mensagem da faixa também no rodapé do quintal.
                    let texto = self.partida.avisos.texto_vigente(self.partida.agora()).map(|t| match t {
                        engine::core::avisos::TextoFaixa::Literal(s) => m
                            .quintal
                            .cfg
                            .mensagem
                            .as_ref()
                            .and_then(|c| interface.fontes.get(c.fonte).and_then(|f| f.as_ref()))
                            .map(|f| f.codificar(s))
                            .unwrap_or_default(),
                        outro => engine::core::avisos::bytes_da_faixa(recursos, outro),
                    });
                    m.quintal.desenhar_mensagem(q, interface, texto.as_deref());
                    // Título do Windows por cima do desenhado pelo quintal: ativo ou não, com o nome
                    // do modo (`_SetMapTitle`, lista 4/1000 itens 9..12).
                    let modo = estado.vistas.modo_quintal.clamp(0, 3) as usize;
                    let titulo = m.quintal.cfg.titulos.get(modo).cloned().unwrap_or_default();
                    janelas::desenhar_titulo(q, interface, &janelas.cfg, &g, &titulo, ativa);
                }
                _ => p.extend(janelas.desenhar(q, recursos, interface, estado, c)?),
            }
        }
        Ok(p)
    }

    /// Contorno do arrasto e menu de sistema por cima de tudo.
    pub(crate) fn desenhar_gerente(&self, q: &mut Quadro) {
        let fontes = &self.janelas.cfg.gerenciador.menu_sistema.fontes;
        let fonte = fontes.iter().find_map(|&i| self.interface.fontes.get(i).and_then(|f| f.as_ref()));
        let acompanhando = self.partida.estado.partida.opcoes[0] != 0;
        self.janelas.gerente.desenhar_por_cima(q, fonte, acompanhando);
    }
}
