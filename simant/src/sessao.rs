//! Sessão de jogo ao vivo: tela carregada, navegação, partida e desenho do quadro.

use super::*;

/// De onde vem a partida.
pub(crate) enum Origem<'o> {
    /// File → Load: o arquivo, o gerador R da sessão antes do Load e o `_CurGameType` de antes
    /// (`_LoadGame` roda `_RandYard` com eles antes de ler os blocos; telas/arquivos.md).
    Save(&'o Path, GeradorR, TipoJogo),
    /// Jogo novo gerado pelo port com a semente dada.
    Novo(u32, TipoJogo),
    /// Load de um arquivo com o nome do tutorial (`Tutorial.Ant`): `_GenerateTutorial` sobre o
    /// estado da partida aberta, com a semente de S de `_SeedSRand` (telas/arquivos.md).
    Tutorial(&'o Estado, u16),
    /// Jogo novo escolhido durante a partida (Ctrl+N ou depois do GAME OVER): `_NewGame` sem
    /// ressemear, com os geradores e o que `_RandYard` não escreve vindos da partida aberta.
    NovoNaPartida(&'o Estado, TipoJogo),
}

/// Tudo o que a tela de jogo ao vivo precisa: a tela carregada (layouts e tilesets), a configuração
/// de navegação e a partida (estado, regras, relógio e vista).
pub(crate) struct Sessao<'a> {
    pub(crate) recursos: &'a Recursos,
    pub(crate) interface: Interface,
    pub(crate) tela: TelaDeJogo,
    pub(crate) nav_cfg: ConfigNavegacao,
    pub(crate) partida: Partida,
    pub(crate) cores: Vec<[u8; 3]>,
    /// Janela do quintal e "Select a Game" durante a partida (simant/src/modos.rs).
    pub(crate) modos: modos::Modos,
    pub(crate) layout: LayoutSave,
    /// Arquivo da partida (o do --save ou o último salvo), alvo do File → Save.
    pub(crate) arquivo: Option<PathBuf>,
    /// Retângulo que fecha o aviso modal da frente, como desenhado no último quadro.
    pub(crate) clique_modal: Option<Retangulo>,
    /// Janelas de controle (Caste/Behavior) e a ordem delas em relação à janela de edição.
    pub(crate) janelas: JanelasControle,
    /// Menus de segurar o botão (data/menus.json) e o menu aberto agora, com o seu alvo.
    pub(crate) menus_cfg: ConfigMenus,
    pub(crate) menu: Option<(MenuAberto, AlvoMenu)>,
    /// Clique simples no mapa esperando o fim da espera de `_processEdit`: instante, ponto, célula.
    pub(crate) clique_pendente: Option<(Instant, i32, i32, (i32, i32))>,
    /// Diálogos do Windows (data/dialogos.json) e o aberto agora (simant/src/arquivos.rs).
    pub(crate) dialogos_cfg: engine::core::dialogos::ConfigDialogos,
    pub(crate) dialogo: Option<arquivos::Dialogo>,
    /// Janela do mapa (core/janela_mapa.rs): buffer das células e trilhas.
    pub(crate) mapa: engine::core::janela_mapa::JanelaMapa,
    /// Mini mapa (core/mini_mapa.rs, simant/src/mini_ui.rs): aberto pelo `ribToYellow`, modal.
    pub(crate) mini: engine::core::mini_mapa::MiniMapa,
    /// Examine (janela 29) e o exame aberto agora, com o canto da janela.
    pub(crate) janela_exame: engine::core::exame::JanelaExame,
    pub(crate) exame: Option<(engine::rules::experimental::Exame, (i32, i32))>,
    /// Janela Info (core/info.rs): cartão atual e histórico.
    pub(crate) info: engine::core::info::JanelaInfo,
    /// Menus e painéis dos scripts de mods (API 0.17, simant/src/mods_ui.rs) e o menu aberto agora.
    pub(crate) menus_mods_cfg: engine::core::menus_mods::ConfigMenusMods,
    pub(crate) menu_mod: Option<engine::core::menus_mods::MenuModAberto>,
    /// Ajuda: modo de ajuda, contextos e cursores (simant/src/ajuda_ui.rs).
    pub(crate) ajuda: ajuda_ui::AjudaUi,
    /// Barra de menus e as janelas abertas por ela (simant/src/menus_ui.rs).
    pub(crate) menus: menus_ui::MenusUi,
}

/// Para quem vai a escolha do menu aberto.
#[derive(Clone, Debug)]
pub(crate) enum AlvoMenu {
    /// `_AntMenu`: `_YellowCommand(código)`.
    Amarela,
    /// Menu de ícones do botão da faixa com esse nome de objeto.
    Botao(String),
}

impl Sessao<'_> {
    /// Sessão com a tela de `data/tela_jogo.json` como está (640×442, a bancada): a dos testes.
    #[cfg(test)]
    pub(crate) fn abrir<'a>(recursos: &'a Recursos, dados: &Path, origem: Origem, cores: Vec<[u8; 3]>) -> Result<Sessao<'a>, String> {
        Self::abrir_na_tela(recursos, dados, origem, cores, None)
    }

    /// Sessão na tela virtual dada (docs/spec/telas/resolucao.md): tamanho da área cliente e tabela
    /// de posições; `None` = os de `data/tela_jogo.json`.
    pub(crate) fn abrir_na_tela<'a>(
        recursos: &'a Recursos,
        dados: &Path,
        origem: Origem,
        cores: Vec<[u8; 3]>,
        tela_virtual: Option<engine::core::resolucao::TelaVirtual>,
    ) -> Result<Sessao<'a>, String> {
        let mut cfg: ConfigTelaJogo = recursos.dados("tela_jogo.json")?;
        if let Some(t) = tela_virtual {
            t.aplicar_tela_jogo(&mut cfg);
        }
        let nav_cfg: ConfigNavegacao = recursos.dados("navegacao.json")?;
        let cfg_sim = ConfigSimulacao::carregar(recursos)?;
        let cfg_partida: ConfigPartida = recursos.dados(partida::ARQUIVO_CONFIG)?;
        let interface = Interface::carregar(recursos, cfg.tabela_posicoes)?;
        let layout = LayoutSave::carregar(&dados.join("save_layout.json"))?;
        let geracao = TabelasGeracao::carregar(recursos)?;
        let (jogo, estado) = match origem {
            Origem::Save(save, r, tipo) => {
                let jogo = JogoSalvo::abrir(&layout, save)?;
                let mut estado = Estado::de_save(&jogo)?;
                let (s, r) = engine::core::carregar::geradores_depois_do_load(r, tipo, &geracao)?;
                estado.s = s;
                estado.r = r;
                (jogo, estado)
            }
            Origem::Novo(semente, tipo) => {
                let quintal = RegrasOriginais::carregar_sem_banco(recursos)?.quintal;
                let estado = engine::core::carregar::estado_de_jogo_novo(tipo, semente, &geracao, &cfg_sim, &quintal)?;
                (estado.para_save(&layout)?, estado)
            }
            Origem::NovoNaPartida(anterior, tipo) => {
                let quintal = RegrasOriginais::carregar_sem_banco(recursos)?.quintal;
                // `_MapPnt` provisório (25 × 19); a navegação recentra com a vista real.
                let estado = engine::core::carregar::jogo_novo_com_geradores(tipo, anterior, &geracao, &cfg_sim, &quintal, (25, 19))?;
                (estado.para_save(&layout)?, estado)
            }
            Origem::Tutorial(anterior, semente_s) => {
                let mut regras = RegrasOriginais::carregar_sem_banco(recursos)?;
                let quintal = regras.quintal.clone();
                let tut: engine::core::carregar::ConfigTutorialArquivo = recursos.dados("tutorial_arquivo.json")?;
                // `_MapPnt` provisório (25 × 19, a vista da bancada); a navegação recentra com a vista real.
                let vista = (25, 19);
                let estado = engine::core::carregar::estado_do_tutorial(
                    anterior, semente_s, &geracao, &cfg_sim, &quintal, &tut, &mut regras, vista,
                )?;
                (estado.para_save(&layout)?, estado)
            }
        };
        let mut tela = TelaDeJogo::carregar(&cfg, recursos, &interface, &jogo)?;
        // _SetEditWinTitle: no Load, o nome do arquivo (em maiúsculas, como o diálogo devolve); num
        // jogo novo, "SimAnt - " e o nome do tipo (lista 4/1001) [medido no Quick Game].
        tela.titulo = match origem {
            Origem::Save(p, ..) => p.file_name().map(|n| n.to_string_lossy().to_uppercase()).unwrap_or_default(),
            Origem::Tutorial(..) => {
                let tut: engine::core::carregar::ConfigTutorialArquivo = recursos.dados("tutorial_arquivo.json")?;
                tut.titulo
            }
            Origem::Novo(_, tipo) | Origem::NovoNaPartida(_, tipo) => {
                let nomes = avisos::linhas_do_texto(recursos, 1001);
                let nome = nomes.get(tipo as usize).map(|l| l.iter().map(|&b| b as char).collect::<String>()).unwrap_or_default();
                format!("SimAnt - {nome}")
            }
        };
        tela.atualizar(&estado);
        // Jogo novo: _NewGame faz _SetMapPlane(_MePlane) e _CenterEdit na amarela, o mesmo que a
        // posição "ao carregar"; um save segue data/navegacao.json.
        let posicao = if matches!(origem, Origem::Novo(..) | Origem::NovoNaPartida(..) | Origem::Tutorial(..)) { PosicaoVista::AoCarregar } else { nav_cfg.posicao_inicial };
        let nav = tela.navegacao(&jogo, posicao)?;
        // Tabelas de todas as partes com mods (data/*.json); o seno da geração já foi lido do banco.
        let regras = RegrasOriginais { geracao: Some(geracao), ..RegrasOriginais::carregar_sem_banco(recursos)? };
        let cfg_avisos: ConfigAvisos = recursos.dados(avisos::ARQUIVO_CONFIG)?;
        let mut partida =
            Partida::nova(estado, nav, regras, TabelasAmarela::carregar(recursos)?, cfg_sim, cfg_partida, cfg_avisos);
        partida.tabelas_trapaca = engine::rules::trapacas::TabelasTrapaca::carregar(recursos)?;
        if matches!(origem, Origem::Save(..) | Origem::Tutorial(..)) {
            // File → Load: _FullCount e o jogo pausado (mensagens.md §6).
            partida.ao_carregar();
        }
        // Scripts Lua dos mods ativos (docs/api-mods.md, "Scripts Lua"): recomeçam a cada partida.
        if let Some(scripts) = engine::scripts::Scripts::carregar(recursos, Default::default())? {
            partida.ligar_scripts(scripts);
            // Load: o estado que os scripts gravaram em NOME.ANT.mods.json (API 0.26), se houver.
            if let Origem::Save(p, ..) = origem {
                partida.ler_estado_dos_mods(p);
            }
        }
        let arquivo = match origem {
            Origem::Save(p, ..) => Some(p.to_path_buf()),
            Origem::Novo(..) | Origem::NovoNaPartida(..) | Origem::Tutorial(..) => None,
        };
        let modos = modos::Modos::carregar(recursos, &interface)?;
        // Gerenciador de janelas: a de edição e a área cliente embaixo da faixa (janelas.md §8).
        let area = Retangulo { esq: 0, topo: interface.moldura.faixa.altura, dir: tela.cfg.largura as i16, base: tela.cfg.altura as i16 };
        let moldura = &tela.cfg.edicao.moldura;
        let mut janelas =
            JanelasControle::new(recursos.dados(janelas::ARQUIVO_CONFIG)?, &interface, recursos, (moldura.retangulo(), moldura.metricas()), area)?;
        let [qx, qy, qw, qh] = modos.quintal.cfg.externo;
        janelas.registrar_quintal(Retangulo { esq: qx, topo: qy, dir: qx + qw, base: qy + qh });
        // _SetDefaultWindows no começo do jogo e, num Load, de novo (o Load roda depois de um jogo).
        let plano = partida.nav.plano_mapa;
        let mut abertas = vec![janelas.janelas_padrao(plano)];
        if matches!(origem, Origem::Save(..) | Origem::Tutorial(..)) {
            abertas.push(janelas.janelas_padrao(plano));
        }
        let menus_cfg: ConfigMenus = recursos.dados(menus::ARQUIVO_CONFIG)?;
        let dialogos_cfg = recursos.dados(engine::core::dialogos::ARQUIVO_CONFIG)?;
        let mapa = engine::core::janela_mapa::JanelaMapa::carregar(recursos)?;
        let janela_exame = engine::core::exame::JanelaExame::carregar(recursos)?;
        let info = engine::core::info::JanelaInfo::new(recursos.dados(engine::core::info::ARQUIVO_CONFIG)?);
        let mut s = Sessao {
            recursos,
            interface,
            tela,
            nav_cfg,
            partida,
            cores,
            layout,
            arquivo,
            clique_modal: None,
            modos,
            janelas,
            menus_cfg,
            menu: None,
            clique_pendente: None,
            dialogos_cfg,
            dialogo: None,
            mapa,
            mini: engine::core::mini_mapa::MiniMapa::carregar(recursos)?,
            janela_exame,
            exame: None,
            info,
            menus_mods_cfg: recursos.dados(engine::core::menus_mods::ARQUIVO_CONFIG)?,
            menu_mod: None,
            ajuda: ajuda_ui::AjudaUi::carregar(recursos)?,
            menus: menus_ui::MenusUi::carregar(recursos)?,
        };
        for a in abertas.into_iter().flatten() {
            s.depois_de_abrir(a);
        }
        s.sincronizar_janelas();
        // _NewGame (`1000:d6dc`): no Tutorial (tipo 0, escolhido no "Select a Game"; não no Load de
        // Tutorial.Ant) a janela de edição é maximizada (`_win_Zoom(0)` = ShowWindow SW_MAXIMIZE) antes
        // do _CenterEdit na amarela (telas/janelas.md §8).
        if matches!(origem, Origem::Novo(_, TipoJogo::Tutorial) | Origem::NovoNaPartida(_, TipoJogo::Tutorial))
            && !s.janelas.gerente.janela(janelas::ID_EDICAO).is_some_and(|j| j.maximizada())
            && s.janelas.gerente.maximizar(janelas::ID_EDICAO)
        {
            s.sincronizar_janelas();
            s.partida.centralizar_edicao_na_amarela();
            s.sincronizar_janelas();
        }
        mods_ui::avisar_teclas_ocupadas(&s);
        Ok(s)
    }

    /// File → Save (`_SaveGame(1)`): grava no arquivo da partida; sem arquivo, ou em "Save As"
    /// (`_SaveGame(0)`), num nome novo. O original pergunta o nome num diálogo comum do Windows.
    pub(crate) fn salvar(&mut self, como: bool, destino_como: Option<&Path>) -> Result<PathBuf, String> {
        let caminho = match (&self.arquivo, como, destino_como) {
            (Some(a), false, _) => a.clone(),
            (_, _, Some(d)) => d.to_path_buf(),
            _ => (1..1000)
                .map(|n| PathBuf::from(format!("SALVO{n:02}.ANT")))
                .find(|p| !p.exists())
                .ok_or("nenhum nome livre para salvar")?,
        };
        self.partida.estado.para_save(&self.layout)?.gravar(&caminho)?;
        // Estado dos scripts dos mods no arquivo lateral (API de mods 0.26); o .ANT não muda.
        self.partida.gravar_estado_dos_mods(&caminho)?;
        self.arquivo = Some(caminho.clone());
        Ok(caminho)
    }

    pub(crate) fn quadro_rgb(&mut self) -> Result<Vec<u8>, String> {
        self.tela.atualizar(&self.partida.estado);
        if let Some((plano, q)) = self.partida.quadro_animado() {
            self.tela.mostrar_quadro_animado(plano, q);
        }
        self.tela.bitmaps_da_faixa = self.partida.bitmaps_dos_marcadores();
        self.tela.selecao_da_faixa.clear();
        for (nome, s, n, ligado) in self.partida.estado_dos_botoes() {
            self.tela.bitmaps_da_faixa.push((nome.clone(), s, n));
            self.tela.selecao_da_faixa.push((nome, ligado.unwrap_or(false)));
        }
        // Botões de cheiro: o do _MapMode atual fica selecionado (grupo 1 da faixa).
        let modo = self.partida.estado.vistas.modo_mapa;
        for (nome, &n) in &self.mapa.cfg.botoes_cheiro {
            self.tela.selecao_da_faixa.push((nome.clone(), n == modo));
        }
        self.sincronizar_janelas();
        if self.janelas.aberta(Controle::Info) {
            self.info.carregar(self.recursos);
        }
        if self.janelas.aberta(Controle::Mapa) {
            let na_tela = self.partida.tomar_mapa_na_tela();
            self.mapa.atualizar_no_timer(&self.partida.estado, self.partida.mostrar_trilhas, na_tela);
        }
        // Janelas na ordem do gerenciador (janelas.md §8): as de baixo da de edição são desenhadas
        // entre a faixa e ela, as outras depois. As janelas filhas ficam dentro da área cliente da
        // janela-mãe: a faixa (outra janela) é refeita por cima delas.
        let ordem = self.janelas.gerente.ordem();
        let edicao = self.janelas.gerente.aberta(janelas::ID_EDICAO);
        let i = ordem.iter().position(|&id| id == janelas::ID_EDICAO).unwrap_or(ordem.len());
        let (atras, frente) = (&ordem[..i], if edicao { &ordem[(i + 1).min(ordem.len())..] } else { &ordem[..0] });
        let largura = self.tela.cfg.largura as usize;
        let linhas_faixa = (self.interface.moldura.faixa.altura.max(0) as usize).min(self.tela.cfg.altura as usize);
        let mut faixa_salva = Vec::new();
        let (mut q, mut pendentes) = {
            let s: &Sessao = self;
            s.tela.compor_com(s.recursos, &s.interface, &s.partida.nav, &mut |q| {
                faixa_salva = q.pixels[..largura * linhas_faixa].to_vec();
                s.desenhar_janelas(q, atras)
            })?
        };
        pendentes.extend(self.desenhar_janelas(&mut q, frente)?);
        if faixa_salva.len() == largura * linhas_faixa {
            q.pixels[..largura * linhas_faixa].copy_from_slice(&faixa_salva);
        }
        let agora = self.partida.agora();
        // Faixa do momento (35 no lugar da 34 com o quintal ativado por último; a mensagem só na 34).
        pendentes.extend(modos::desenhar_faixa(self, &mut q));
        if !self.modos.janela.na_frente {
            self.tela.desenhar_mensagem_da_faixa(&mut q, self.recursos, &self.interface, &mut self.partida.avisos, agora)?;
        }
        pendentes.extend(modos::desenhar(self, &mut q)?);
        mini_ui::desenhar(self, &mut q);
        // O aviso modal fica por cima de todas as janelas (e do resto dos modos): o quadro de recusa
        // da troca de patch (10000/10002/10004) sai de um duplo clique no quintal.
        // Durante uma animação da amarela o quadro modal pedido no mesmo tick ainda não apareceu.
        if !self.partida.animando() {
            let (clique, p) = avisos::desenhar_modal(&mut q, self.recursos, &self.interface, &self.partida.avisos)?;
            self.clique_modal = clique;
            pendentes.extend(p);
        } else {
            // A janela que _SpiderDialog deixou aberta continua por cima durante a luta.
            avisos::desenhar_residual(&mut q, self.recursos, &self.partida.avisos)?;
            self.clique_modal = None;
        }
        if let Some((m, _)) = &self.menu {
            pendentes.extend(m.desenhar(&mut q, self.recursos, &self.interface)?);
        }
        if let Some((ex, canto)) = &self.exame {
            self.janela_exame.desenhar(&mut q, self.recursos, &self.interface, ex, *canto);
        }
        // Contorno do arrasto (mover, redimensionar, polegar) e menu de sistema.
        self.desenhar_gerente(&mut q);
        ajuda_ui::desenhar(self, &mut q);
        mods_ui::desenhar(self, &mut q);
        pendentes.extend(menus_ui::desenhar_no_cliente(self, &mut q));
        self.desenhar_dialogo(&mut q);
        if !pendentes.is_empty() {
            eprintln!("tela de jogo: não desenhado ainda: {}", pendentes.join(", "));
        }
        Ok(para_rgb(&q.pixels, &self.cores))
    }

    /// `_DoUserButton`: as funções de janela que o port já tem (0 Behavior, 1 Caste) abrem aqui;
    /// o resto vai para a `Partida`.
    pub(crate) fn botao_usuario(&mut self, f: u16) -> RespostaBotao {
        match f {
            0 => self.janelas.trazer_para_frente(Some(Controle::Tarefas)),
            1 => self.janelas.trazer_para_frente(Some(Controle::Castas)),
            2 => self.janelas.trazer_para_frente(Some(Controle::Historico)),
            3 => self.abrir_info(),
            15 => {
                // _GotoMyAnt depois de abrir a janela de edição e trazê-la para a frente.
                self.janelas.trazer_para_frente(None);
                return self.partida.botao_usuario(f);
            }
            _ => return self.partida.botao_usuario(f),
        }
        RespostaBotao::Feito
    }

    /// Marcador da faixa (`ribBM1`–`7`) sob (x, y).
    pub(crate) fn marcador_em(&self, x: i32, y: i32) -> Option<usize> {
        let nomes = &self.partida.cfg.marcadores.objetos;
        self.tela.objetos_da_faixa_em(x, y).iter().find_map(|n| nomes.iter().position(|m| m == n))
    }

    /// Depois de um clique na área de uma janela de controle: o mapa centraliza a vista e o Info
    /// troca de cartão.
    pub(crate) fn depois_do_clique_em_controle(&mut self) {
        self.centralizar_pelo_mapa();
        if let Some((x, y)) = self.janelas.clique_info.take() {
            if let Some(a) = self.janelas.area_do_info(self.recursos) {
                if self.info.clicar(x - i32::from(a.esq), y - i32::from(a.topo)) {
                    println!("info: cartão {}", self.info.atual);
                }
            }
        }
    }

    /// `_OpenInfoWindow`: com a janela fechada, volta ao cartão inicial; sempre a traz para a frente.
    pub(crate) fn abrir_info(&mut self) {
        if !self.janelas.aberta(Controle::Info) {
            self.info.reiniciar();
        }
        self.janelas.trazer_para_frente(Some(Controle::Info));
    }

    /// `_MapAreaEvent`: o clique ou o arrasto na área do mapa centraliza a vista de edição na célula;
    /// no duplo clique, a janela de edição vem depois para a frente (`_OpenEditWindow`). Shift+clique
    /// é o clique da janela de edição naquela célula (`_processEdit`), sem centralizar.
    pub(crate) fn centralizar_pelo_mapa(&mut self) {
        if let Some((x, y, duplo)) = self.janelas.ordem_mapa.take() {
            self.ordem_pelo_mapa(x, y, duplo);
        }
        let Some((x, y)) = self.janelas.clique_mapa.take() else { return };
        let Some(area) = self.janelas.area_do_mapa(self.recursos) else { return };
        if let Some((cx, cy)) = self.mapa.celula_em(area, &self.partida.estado, x, y) {
            self.centralizar_vista(cx, cy);
        }
        if std::mem::take(&mut self.janelas.clique_mapa_duplo) {
            self.janelas.trazer_para_frente(None);
        }
    }

    /// `_CenterEdit(cx, cy)`: a vista de edição vai até a célula.
    pub(crate) fn centralizar_vista(&mut self, cx: i32, cy: i32) {
        if self.partida.nav.vista.centralizar(cx, cy) {
            self.partida.processar_eventos_do_estado();
        }
    }

    /// Shift+clique no mapa (`_MapAreaEvent` → `_processEdit` com o ponto convertido para a célula):
    /// no Experimental, a ferramenta; senão a ordem à amarela (ou a ferramenta 10/11). A janela de
    /// edição não está na frente, então não há a espera do clique simples, e com Shift apertado não
    /// há menu da amarela nem Examine [código `1000:6a38`; bancada q01/q04 e p05].
    fn ordem_pelo_mapa(&mut self, x: i32, y: i32, duplo: bool) {
        let Some(area) = self.janelas.area_do_mapa(self.recursos) else { return };
        let Some((cx, cy)) = self.mapa.celula_em(area, &self.partida.estado, x, y) else { return };
        if self.partida.estado.partida.tipo_jogo == 3 {
            modos::ferramenta_na_celula(self, cx, cy, self.janelas.ctrl, false);
            return;
        }
        let r = if duplo { self.partida.clicar(cx, cy, true) } else { self.partida.clicar_depois_da_espera(cx, cy, false) };
        println!("Shift+clique {} no mapa em ({cx}, {cy}), plano {}: {r:?}", if duplo { "duplo" } else { "simples" }, self.partida.nav.plano_mapa);
    }

    /// `_OpenMapYard` (Window → Map/Yard, Ctrl+M, mini mapa): com o quintal aberto, ele vem para a
    /// frente; senão a janela do mapa (aberta na superfície, se estava fechada).
    pub(crate) fn abrir_mapa_ou_quintal(&mut self) {
        if self.janelas.aberta(Controle::Quintal) {
            modos::aplicar_acao_quintal(self, AcaoQuintal::Abrir, true);
        } else {
            self.abrir_mapa();
        }
    }

    /// Botão de cheiro da faixa sob (x, y): muda o `_MapMode` (`_ProcMapRibbonEvent`).
    pub(crate) fn botao_de_cheiro_em(&mut self, x: i32, y: i32) -> bool {
        let objetos = self.tela.objetos_da_faixa_em(x, y);
        let Some(n) = objetos.iter().find_map(|o| self.mapa.cfg.botoes_cheiro.get(*o).copied()) else { return false };
        self.partida.botao_de_cheiro(n);
        println!("mapa: modo {}", self.partida.estado.vistas.modo_mapa);
        true
    }

    /// Window -> Map (`_OpenMapYard`): com o quintal fechado, abre o mapa (ou o traz para a
    /// frente) na superfície, como `_YardToMap`.
    pub(crate) fn abrir_mapa(&mut self) {
        if !self.janelas.aberta(Controle::Mapa) && self.partida.nav.plano_mapa != 1 {
            self.partida.navegar(Acao::Superficie);
        }
        self.janelas.trazer_para_frente(Some(Controle::Mapa));
        // _YardToMap → _DrawMap: a geração da abertura, com o modo de agora.
        self.mapa.atualizar(&self.partida.estado, self.partida.mostrar_trilhas);
    }

    /// Botão configurável da faixa sob (x, y): a função dele (`_DoUserButton`).
    pub(crate) fn botao_usuario_em(&self, x: i32, y: i32) -> Option<u16> {
        self.objetos_da_faixa_visivel_em(x, y).iter().find_map(|n| self.partida.funcao_do_botao(n))
    }

    /// Objetos da faixa que está na tela (a 35 com o quintal ativado por último, senão a 34) sob (x, y).
    fn objetos_da_faixa_visivel_em(&self, x: i32, y: i32) -> Vec<String> {
        let objetos = if self.modos.janela.na_frente { self.modos.quintal.objetos_da_faixa_em(x, y) } else { self.tela.objetos_da_faixa_em(x, y) };
        objetos.into_iter().map(String::from).collect()
    }

    /// Ctrl+clique num botão configurável: abre o menu de ícones (`_DoUserButton`, janela 36).
    pub(crate) fn abrir_menu_de_icones(&mut self, x: i32, y: i32) -> bool {
        let Some(nome) = self.objetos_da_faixa_visivel_em(x, y).into_iter().find(|n| self.partida.funcao_do_botao(n).is_some()) else {
            return false;
        };
        let r = if self.modos.janela.na_frente { self.modos.quintal.retangulo_da_faixa(&nome) } else { self.tela.objeto_da_faixa(&nome).map(|(r, _)| r) };
        let Some(r) = r else { return false };
        let [dx, dy] = self.menus_cfg.deslocamento_icones;
        let limite = Retangulo { esq: 0, topo: 0, dir: self.tela.cfg.largura as i16, base: self.tela.cfg.altura as i16 };
        match MenuAberto::abrir(self.recursos, &self.interface, &self.menus_cfg.icones, i32::from(r.esq + dx), i32::from(dy), limite) {
            Ok(mut m) => {
                // __win_SetProxItem(função atual + 0x2402): o item da função atual já vem realçado.
                m.realcado = self.partida.funcao_do_botao(&nome).map(usize::from);
                self.menu = Some((m, AlvoMenu::Botao(nome)));
                true
            }
            Err(e) => {
                eprintln!("menu de ícones: {e}");
                false
            }
        }
    }

    /// `_AntMenu`: o menu certo para a amarela (formiga, rainha alada sem colônia, aranha) no ponto.
    pub(crate) fn abrir_menu_da_amarela(&mut self, x: i32, y: i32) {
        let a = &self.partida.estado.amarela;
        let cfg = if a.modo == 1 {
            &self.menus_cfg.aranha
        } else if a.tipo == 0x40 && a.ninho_iniciado == 0 {
            &self.menus_cfg.rainha
        } else {
            &self.menus_cfg.formiga
        };
        let [dx, dy] = cfg.deslocamento.unwrap_or(self.menus_cfg.deslocamento_formiga);
        let limite = Retangulo { esq: 0, topo: 0, dir: self.tela.cfg.largura as i16, base: self.tela.cfg.altura as i16 };
        match MenuAberto::abrir(self.recursos, &self.interface, cfg, x + i32::from(dx), y + i32::from(dy), limite) {
            Ok(m) => self.menu = Some((m, AlvoMenu::Amarela)),
            Err(e) => eprintln!("menu da amarela: {e}"),
        }
    }

    /// Botão solto com um menu aberto: aplica a escolha.
    pub(crate) fn soltar_menu(&mut self, x: i32, y: i32, shift: bool) {
        let Some((m, alvo)) = self.menu.take() else { return };
        let Some(mut c) = m.soltar(x, y) else { return };
        match alvo {
            AlvoMenu::Amarela => {
                let [de, para] = self.menus_cfg.codigo_com_shift;
                if shift && c == de {
                    c = para;
                }
                let ok = self.partida.comando_amarela(c);
                println!("menu da amarela: comando {c}{}", if ok { "" } else { " (erro)" });
            }
            AlvoMenu::Botao(nome) => {
                self.partida.definir_funcao_do_botao(&nome, u16::from(c), shift);
                println!("botão {nome}: função {c}");
            }
        }
    }

    /// Fim da espera do clique simples no mapa: com o botão ainda apertado (e Shift solto), menu
    /// da amarela ou Examine; senão, a ordem de sempre.
    pub(crate) fn decidir_clique_pendente(&mut self, segurando: bool) {
        let Some((_, x, y, (cx, cy))) = self.clique_pendente.take() else { return };
        let r = self.partida.clicar_depois_da_espera(cx, cy, segurando);
        println!("clique simples em ({cx}, {cy}), plano {}{}: {r:?}", self.partida.nav.plano_mapa, if segurando { " segurando" } else { "" });
        match r {
            RespostaClique::MenuAmarela => self.abrir_menu_da_amarela(x, y),
            RespostaClique::Examinar => {
                let plano = i32::from(self.partida.nav.plano_mapa);
                if let Some(ex) = engine::rules::experimental::examinar(&self.partida.estado, plano, cx, cy) {
                    self.abrir_exame(ex);
                }
            }
            _ => {}
        }
    }

    /// `_MagnifyMenu`: abre a janela 29 junto da célula examinada; fica aberta enquanto o botão
    /// estiver apertado (a interface fecha ao soltar) e a simulação não anda.
    pub(crate) fn abrir_exame(&mut self, ex: engine::rules::experimental::Exame) {
        let v = &self.partida.nav.vista;
        let celula = ((ex.x - v.x).rem_euclid(128), ex.y - v.y);
        let [ax, ay] = self.tela.cfg.edicao.area_na_tela;
        let tela = (self.tela.cfg.largura as i32, self.tela.cfg.altura as i32);
        let canto = self.janela_exame.canto_ao_abrir(celula, (i32::from(ax), i32::from(ay)), tela);
        let textos = self.janela_exame.textos(&ex);
        let t: Vec<String> = textos.iter().map(|l| l.iter().map(|&b| b as char).collect()).collect();
        println!("Examine ({}, {}): {}", ex.x, ex.y, t.join(" / "));
        self.exame = Some((ex, canto));
    }

    /// Ação do clique esquerdo em (x, y) da área cliente: seta da barra de rolagem ou botão da faixa.
    pub(crate) fn acao_do_clique(&self, x: i32, y: i32) -> Option<Acao> {
        self.tela
            .seta_em(x, y)
            .or_else(|| self.tela.objetos_da_faixa_em(x, y).iter().find_map(|n| self.nav_cfg.botoes_faixa.get(*n).copied()))
    }

    /// Clique (ou tecla que imita o mouse) em (x, y): se cai numa célula do mapa, vira ordem para a
    /// formiga amarela.
    pub(crate) fn clicar_no_mapa(&mut self, x: i32, y: i32, duplo: bool) {
        if let Some((cx, cy)) = self.tela.celula_em(x, y, &self.partida.nav) {
            let r = self.partida.clicar(cx, cy, duplo);
            println!("clique {} em ({cx}, {cy}), plano {}: {r:?}", if duplo { "duplo" } else { "simples" }, self.partida.nav.plano_mapa);
        }
    }

    /// Escreve os pedidos das regras à interface (mensagens, músicas, fim de jogo) e os devolve
    /// (o som é tocado por `som::Audio`); por enquanto sem texto nem diálogo.
    pub(crate) fn registrar_eventos(&mut self) -> Vec<EventoRegras> {
        for linha in self.partida.retirar_registro_scripts() {
            println!("[script] {linha}");
        }
        let eventos = self.partida.retirar_eventos();
        for ev in &eventos {
            let t = self.partida.estado.partida.tempo;
            match *ev {
                EventoRegras::Mensagem(id) => {
                    let texto: Vec<String> =
                        avisos::linhas_do_texto(self.recursos, id).iter().map(|l| l.iter().map(|&b| b as char).collect()).collect();
                    println!("[tick {t}] mensagem {id}: {}", texto.join(" / "));
                }
                EventoRegras::DialogoAranha { texto } => println!("[tick {t}] quadro da aranha, texto {texto} (_SpiderDialog)"),
                EventoRegras::Musica(id) => println!("[tick {t}] música {id}"),
                EventoRegras::Som(_) | EventoRegras::TemaDoNinho | EventoRegras::Espera(_) => {} // tocados por som::Audio
                EventoRegras::Animacao(_) => {} // a Partida guarda e mostra os quadros
                EventoRegras::DialogoFimDeJogo => println!("[tick {t}] fim de jogo (_EndGameDialog)"),
                EventoRegras::EscolherColonia => println!("[tick {t}] escolher outra colônia (_SpecialXfer)"),
                EventoRegras::QuadroAmarela { imagem, .. } => println!("[tick {t}] quadro da amarela {imagem} (_YellowDialog)"),
                EventoRegras::QuadroFormigaLeao => println!("[tick {t}] quadro da formiga-leão (_LionDialog)"),
                EventoRegras::AnimacaoVitoria => println!("[tick {t}] animação da vitória (_DrawSimPayoff)"),
                EventoRegras::CentralizarNaAmarela { .. } | EventoRegras::MensagemFaixa(_) => {} // a faixa é da Partida
            }
        }
        eventos
    }

    /// Globais do original que um jogo novo ou um Load não refazem: `_OptionStates` (`_NewGame` não
    /// o toca; o `_GenerateTutorial` do Load de `Tutorial.Ant` também não) e as funções dos botões
    /// configuráveis (fora do save). `do_save`: Load de um `.ANT`, que traz as opções dele.
    pub(crate) fn herdar_opcoes(&mut self, antiga: &Sessao, do_save: bool) {
        if !do_save {
            self.partida.estado.partida.opcoes = antiga.partida.estado.partida.opcoes;
        }
        self.partida.funcoes_botoes = antiga.partida.funcoes_botoes.clone();
        self.partida.funcoes_botoes_quintal = antiga.partida.funcoes_botoes_quintal.clone();
    }

    /// Diz à partida quais janelas estão abertas e na frente (sorteios do desenho depois de cada
    /// tick, `rules::desenho`): "na frente" é `_win_IsWinInFront`, a janela de cima do gerenciador.
    pub(crate) fn atualizar_janelas_desenho(&mut self, minimizada: bool) {
        let g = &self.janelas.gerente;
        let quintal = self.janelas.cfg.quintal;
        let topo = g.topo();
        let j = &mut self.partida.janelas_desenho;
        j.minimizada = minimizada;
        j.quintal_aberto = g.aberta(quintal);
        j.quintal_na_frente = topo == Some(quintal);
        j.edicao_aberta = g.aberta(janelas::ID_EDICAO);
        let mapa = self.janelas.cfg.id(Controle::Mapa);
        self.partida.mapa_aberto = g.aberta(mapa);
        self.partida.mapa_na_frente = topo == Some(mapa);
        j.edicao_na_frente = topo == Some(janelas::ID_EDICAO);
        // `_win_IsWinInFront(0)` das animações de morte e de loucura da amarela (quantos quadros e
        // sorteios): a ordem real do gerenciador, no lugar do valor fixo de amarela.json.
        let frente = j.edicao_na_frente;
        self.partida.regras.amarela.animacao.janela_edicao_na_frente = frente;
        self.partida.tabelas_amarela.animacao.janela_edicao_na_frente = frente;
        // O tutorial pergunta `_win_IsWinInFront(janela)`: o número da janela de cima (prop INDEX).
        self.partida.regras.interface_tutor.janelas = topo.map(|t| vec![t << 8]).unwrap_or_default();
    }

    /// Velocidade e pausa, para o título da janela.
    pub(crate) fn situacao(&self) -> String {
        let p = &self.partida.estado.partida;
        let vel = ["lenta", "normal", "rápida", "ultra"].get(p.velocidade as usize).copied().unwrap_or("?");
        let pausa = if p.pausado != 0 { ", pausado" } else { "" };
        let track = if p.opcoes[0] != 0 { ", AutoTrack" } else { "" };
        format!("{vel}{pausa}{track}")
    }
}

#[cfg(test)]
mod testes_tela_virtual {
    use super::*;
    use engine::core::resolucao::{ConfigResolucao, ARQUIVO};

    fn raiz() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    /// QUICK2.ANT aberto na tela virtual dada (sem os arquivos do jogo, o teste é ignorado).
    fn quick2(resolucao: [u32; 2], f: impl FnOnce(&mut Sessao)) {
        abrir(resolucao, None, f)
    }

    /// Jogo novo do tipo dado (semente 1), ou QUICK2.ANT com `None`, na tela virtual dada.
    fn abrir(resolucao: [u32; 2], novo: Option<TipoJogo>, f: impl FnOnce(&mut Sessao)) {
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SHARED.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
            return;
        }
        let dados = raiz().join("data");
        let cfg: ConfigJogo = serde_json::from_str(&std::fs::read_to_string(dados.join("jogo.json")).unwrap()).unwrap();
        let recursos = Recursos::abrir(&jogo, &cfg.bancos, &dados, None).unwrap();
        let cores = recursos.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
        let tela = recursos.dados::<ConfigResolucao>(ARQUIVO).unwrap().tela(resolucao).unwrap();
        let save = raiz().join("tests/referencias/saves/QUICK2.ANT");
        let origem = match novo {
            Some(tipo) => Origem::Novo(1, tipo),
            None => Origem::Save(&save, GeradorR::default(), TipoJogo::Rapido),
        };
        let mut s = Sessao::abrir_na_tela(&recursos, &dados, origem, cores, Some(tela)).unwrap();
        f(&mut s);
    }

    #[test]
    fn tutorial_abre_a_janela_de_edicao_maximizada_na_tela_virtual() {
        // _NewGame: no Tutorial, _win_Zoom(0). Em 640×480 a vista fica com 39 × 21 células, como a
        // _editWidth × _editHeight gravada pela bancada nos vetores do tutorial.
        for (resolucao, vista) in [([640, 480], Some((39, 21))), ([800, 600], None)] {
            abrir(resolucao, Some(TipoJogo::Tutorial), |s| {
                let j = *s.janelas.gerente.janela(janelas::ID_EDICAO).unwrap();
                assert!(j.maximizada(), "{resolucao:?}");
                // maximizada, a moldura fica fora da área (como no Windows): cobre a largura toda da tela virtual
                assert!(j.externo.esq <= 0 && j.externo.dir >= s.tela.cfg.largura as i16, "{resolucao:?}: {:?}", j.externo);
                if let Some((w, h)) = vista {
                    assert_eq!((s.partida.nav.vista.largura, s.partida.nav.vista.altura), (w, h));
                }
            });
        }
        // Os outros tipos não maximizam.
        abrir([640, 480], Some(TipoJogo::Rapido), |s| {
            assert!(!s.janelas.gerente.janela(janelas::ID_EDICAO).unwrap().maximizada());
        });
    }

    fn externo(s: &Sessao, c: Controle) -> Retangulo {
        s.janelas.gerente.janela(s.janelas.cfg.id(c)).unwrap().externo
    }

    #[test]
    fn tela_virtual_640_e_a_de_tela_jogo_json() {
        // 640×480 dá a mesma tela de data/tela_jogo.json (a da bancada): os testes de tela não mudam.
        quick2([640, 480], |s| {
            assert_eq!((s.tela.cfg.largura, s.tela.cfg.altura, s.tela.cfg.tabela_posicoes), (640, 442, 0));
            assert_eq!(s.interface.posicoes[19][..2], [342, 158], "Caste na tabela 9/0");
        });
    }

    #[test]
    fn tela_virtual_800_e_1024_usam_a_tabela_8() {
        let mut em_640 = Vec::new();
        quick2([640, 480], |s| em_640 = vec![externo(s, Controle::Castas), externo(s, Controle::Tarefas)]);
        if em_640.is_empty() {
            return;
        }
        for (r, l, a) in [([800, 600], 800, 562), ([1024, 768], 1024, 730)] {
            quick2(r, |s| {
                assert_eq!((s.tela.cfg.largura, s.tela.cfg.altura, s.tela.cfg.tabela_posicoes), (l, a, 8));
                // Caste e Behavior descem 127 linhas (9/0: y = 158; 9/8: y = 285), para baixo da edição.
                let castas = externo(s, Controle::Castas);
                let tarefas = externo(s, Controle::Tarefas);
                assert_eq!((castas.esq, castas.topo), (em_640[0].esq, em_640[0].topo + 127));
                assert_eq!((tarefas.esq, tarefas.topo), (em_640[1].esq, em_640[1].topo + 127));
                // A faixa ocupa a largura toda; o quadro tem o tamanho da área cliente.
                let rgb = s.quadro_rgb().unwrap();
                assert_eq!(rgb.len(), (l * a) as usize * 3);
                let px = |x: u32, y: u32| rgb[((y * l + x) * 3) as usize..((y * l + x) * 3 + 3) as usize].to_vec();
                assert_eq!(px(l - 1, 30), vec![0xc0, 0xc0, 0xc0], "fundo cinza da faixa até a borda direita");
                assert_eq!(px(l - 1, a - 1), vec![0xff, 0xfb, 0xf0], "fundo da área cliente");
            });
        }
    }
}
