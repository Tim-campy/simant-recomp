//! Sessão de jogo ao vivo: tela carregada, navegação, partida e desenho do quadro.

use super::*;

/// De onde vem a partida.
pub(crate) enum Origem<'o> {
    /// File → Load: o arquivo, o gerador R da sessão antes do Load e o `_CurGameType` de antes
    /// (`_LoadGame` roda `_RandYard` com eles antes de ler os blocos; telas/arquivos.md).
    Save(&'o Path, GeradorR, TipoJogo),
    /// Jogo novo gerado pelo port com a semente dada.
    Novo(u32, TipoJogo),
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
    pub(crate) fn abrir<'a>(recursos: &'a Recursos, dados: &Path, origem: Origem, cores: Vec<[u8; 3]>) -> Result<Sessao<'a>, String> {
        let cfg: ConfigTelaJogo = recursos.dados("tela_jogo.json")?;
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
                // _initStuff: _SeedSRand e _SeedRRand com o relógio; o port usa a semente nas duas
                // leituras do relógio. Depois _NewGame → _RandYard (geracao-mundo.md).
                let mut s = GeradorS::new(0);
                let mut r = GeradorR::default();
                r.semear_com_ticks(&mut s, semente, semente);
                let m = Mundo::novo_jogo(tipo, &mut r, &mut s, &geracao)?;
                let mut estado = Estado::de_mundo(&m, s, r, &cfg_sim)?;
                // _RandYard passo 3: _InitSimYard (menino, cachorro...), que Estado::de_mundo não faz.
                let quintal = RegrasOriginais::carregar_sem_banco(recursos)?.quintal;
                engine::rules::quintal::iniciar_quintal(&mut estado, &quintal);
                (estado.para_save(&layout)?, estado)
            }
        };
        let mut tela = TelaDeJogo::carregar(&cfg, recursos, &interface, &jogo)?;
        // _SetEditWinTitle: no Load, o nome do arquivo (em maiúsculas, como o diálogo devolve); num
        // jogo novo, "SimAnt - " e o nome do tipo (lista 4/1001) [medido no Quick Game].
        tela.titulo = match origem {
            Origem::Save(p, ..) => p.file_name().map(|n| n.to_string_lossy().to_uppercase()).unwrap_or_default(),
            Origem::Novo(_, tipo) => {
                let nomes = avisos::linhas_do_texto(recursos, 1001);
                let nome = nomes.get(tipo as usize).map(|l| l.iter().map(|&b| b as char).collect::<String>()).unwrap_or_default();
                format!("SimAnt - {nome}")
            }
        };
        tela.atualizar(&estado);
        // Jogo novo: _NewGame faz _SetMapPlane(_MePlane) e _CenterEdit na amarela, o mesmo que a
        // posição "ao carregar"; um save segue data/navegacao.json.
        let posicao = if matches!(origem, Origem::Novo(..)) { PosicaoVista::AoCarregar } else { nav_cfg.posicao_inicial };
        let nav = tela.navegacao(&jogo, posicao)?;
        // Tabelas de todas as partes com mods (data/*.json); o seno da geração já foi lido do banco.
        let regras = RegrasOriginais { geracao: Some(geracao), ..RegrasOriginais::carregar_sem_banco(recursos)? };
        let cfg_avisos: ConfigAvisos = recursos.dados(avisos::ARQUIVO_CONFIG)?;
        let mut partida =
            Partida::nova(estado, nav, regras, TabelasAmarela::carregar(recursos)?, cfg_sim, cfg_partida, cfg_avisos);
        if matches!(origem, Origem::Save(..)) {
            // File → Load: _FullCount e o jogo pausado (mensagens.md §6).
            partida.ao_carregar();
        }
        let arquivo = match origem {
            Origem::Save(p, ..) => Some(p.to_path_buf()),
            Origem::Novo(..) => None,
        };
        let modos = modos::Modos::carregar(recursos, &interface)?;
        let janelas = JanelasControle::new(recursos.dados(janelas::ARQUIVO_CONFIG)?, &interface);
        let menus_cfg: ConfigMenus = recursos.dados(menus::ARQUIVO_CONFIG)?;
        let dialogos_cfg = recursos.dados(engine::core::dialogos::ARQUIVO_CONFIG)?;
        Ok(Sessao {
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
        })
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
        self.arquivo = Some(caminho.clone());
        Ok(caminho)
    }

    pub(crate) fn quadro_rgb(&mut self) -> Result<Vec<u8>, String> {
        self.tela.atualizar(&self.partida.estado);
        self.tela.bitmaps_da_faixa = self.partida.bitmaps_dos_marcadores();
        self.tela.selecao_da_faixa.clear();
        for (nome, s, n, ligado) in self.partida.estado_dos_botoes() {
            self.tela.bitmaps_da_faixa.push((nome.clone(), s, n));
            self.tela.selecao_da_faixa.push((nome, ligado.unwrap_or(false)));
        }
        // Janelas de controle: as que estão antes da de edição na pilha ficam atrás dela.
        let edicao = self.janelas.pilha.iter().position(|j| j.is_none()).unwrap_or(self.janelas.pilha.len());
        let (atras, frente) = self.janelas.pilha.split_at(edicao);
        let (recursos, interface, estado, janelas) = (self.recursos, &self.interface, &self.partida.estado, &self.janelas);
        let desenhar = |q: &mut Quadro, lista: &[Option<Controle>]| -> Result<Vec<String>, String> {
            let mut p = Vec::new();
            for c in lista.iter().flatten() {
                p.extend(janelas.desenhar(q, recursos, interface, estado, *c)?);
            }
            Ok(p)
        };
        let (mut q, mut pendentes) =
            self.tela.compor_com(self.recursos, &self.interface, &self.partida.nav, &mut |q| desenhar(q, atras))?;
        pendentes.extend(desenhar(&mut q, frente)?);
        let agora = self.partida.agora();
        let (clique, p) = self.tela.desenhar_avisos(&mut q, self.recursos, &self.interface, &mut self.partida.avisos, agora)?;
        self.clique_modal = clique;
        pendentes.extend(p);
        pendentes.extend(modos::desenhar(self, &mut q)?);
        if let Some((m, _)) = &self.menu {
            pendentes.extend(m.desenhar(&mut q, self.recursos, &self.interface)?);
        }
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

    /// Clique esquerdo nas janelas de controle (Caste/Behavior); `true` se foi tratado.
    pub(crate) fn clicar_nas_janelas(&mut self, x: i32, y: i32) -> bool {
        let edicao = self.tela.retangulo_da_edicao();
        match self.janelas.clicar(self.recursos, &mut self.partida.estado, edicao, x, y) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("janelas: {e}");
                false
            }
        }
    }

    /// Botão configurável da faixa sob (x, y): a função dele (`_DoUserButton`).
    pub(crate) fn botao_usuario_em(&self, x: i32, y: i32) -> Option<u16> {
        self.tela.objetos_da_faixa_em(x, y).iter().find_map(|n| self.partida.funcao_do_botao(n))
    }

    /// Ctrl+clique num botão configurável: abre o menu de ícones (`_DoUserButton`, janela 36).
    pub(crate) fn abrir_menu_de_icones(&mut self, x: i32, y: i32) -> bool {
        let Some(nome) = self.tela.objetos_da_faixa_em(x, y).into_iter().find(|n| self.partida.funcao_do_botao(n).is_some()) else {
            return false;
        };
        let nome = nome.to_string();
        let Some((r, _)) = self.tela.objeto_da_faixa(&nome) else { return false };
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
        let [dx, dy] = self.menus_cfg.deslocamento_formiga;
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
                self.partida.definir_funcao_do_botao(&nome, u16::from(c));
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
            RespostaClique::Examinar => println!("Examine (_MagnifyMenu): a janela 29 ainda não existe no port"),
            _ => {}
        }
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
        let eventos = self.partida.retirar_eventos();
        for ev in &eventos {
            let t = self.partida.estado.partida.tempo;
            match *ev {
                EventoRegras::Mensagem(id) => {
                    let texto: Vec<String> =
                        avisos::linhas_do_texto(self.recursos, id).iter().map(|l| l.iter().map(|&b| b as char).collect()).collect();
                    println!("[tick {t}] mensagem {id}: {}", texto.join(" / "));
                }
                EventoRegras::Musica(id) => println!("[tick {t}] música {id}"),
                EventoRegras::Som(_) | EventoRegras::TemaDoNinho => {} // tocados por som::Audio
                EventoRegras::DialogoFimDeJogo => println!("[tick {t}] fim de jogo (_EndGameDialog)"),
                EventoRegras::CentralizarNaAmarela | EventoRegras::MensagemFaixa(_) => {} // a faixa é da Partida
            }
        }
        eventos
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
