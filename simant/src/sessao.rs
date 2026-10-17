//! Sessão de jogo ao vivo: tela carregada, navegação, partida e desenho do quadro.

use super::*;

/// De onde vem a partida.
pub(crate) enum Origem<'o> {
    Save(&'o Path),
    /// Quick Game gerado pelo port com a semente dada.
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
    pub(crate) layout: LayoutSave,
    /// Arquivo da partida (o do --save ou o último salvo), alvo do File → Save.
    pub(crate) arquivo: Option<PathBuf>,
    /// Retângulo que fecha o aviso modal da frente, como desenhado no último quadro.
    pub(crate) clique_modal: Option<Retangulo>,
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
            Origem::Save(save) => {
                let jogo = JogoSalvo::abrir(&layout, save)?;
                let estado = Estado::de_save(&jogo)?;
                (jogo, estado)
            }
            Origem::Novo(semente, tipo) => {
                // _initStuff: _SeedSRand e _SeedRRand com o relógio; o port usa a semente nas duas
                // leituras do relógio. Depois _NewGame → _RandYard (geracao-mundo.md).
                let mut s = GeradorS::new(0);
                let mut r = GeradorR::default();
                r.semear_com_ticks(&mut s, semente, semente);
                let m = Mundo::novo_jogo(tipo, &mut r, &mut s, &geracao)?;
                let estado = Estado::de_mundo(&m, s, r, &cfg_sim)?;
                (estado.para_save(&layout)?, estado)
            }
        };
        let mut tela = TelaDeJogo::carregar(&cfg, recursos, &interface, &jogo)?;
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
        if matches!(origem, Origem::Save(_)) {
            // File → Load: _FullCount e o jogo pausado (mensagens.md §6).
            partida.ao_carregar();
        }
        let arquivo = match origem {
            Origem::Save(p) => Some(p.to_path_buf()),
            Origem::Novo(..) => None,
        };
        Ok(Sessao { recursos, interface, tela, nav_cfg, partida, cores, layout, arquivo, clique_modal: None })
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
        let (mut q, mut pendentes) = self.tela.compor(self.recursos, &self.interface, &self.partida.nav)?;
        let agora = self.partida.agora();
        let (clique, p) = self.tela.desenhar_avisos(&mut q, self.recursos, &self.interface, &mut self.partida.avisos, agora)?;
        self.clique_modal = clique;
        pendentes.extend(p);
        if !pendentes.is_empty() {
            eprintln!("tela de jogo: não desenhado ainda: {}", pendentes.join(", "));
        }
        Ok(para_rgb(&q.pixels, &self.cores))
    }

    /// Botão configurável da faixa sob (x, y): a função dele (`_DoUserButton`).
    /// Marcador da faixa (`ribBM1`–`7`) sob (x, y).
    pub(crate) fn marcador_em(&self, x: i32, y: i32) -> Option<usize> {
        let nomes = &self.partida.cfg.marcadores.objetos;
        self.tela.objetos_da_faixa_em(x, y).iter().find_map(|n| nomes.iter().position(|m| m == n))
    }

    pub(crate) fn botao_usuario_em(&self, x: i32, y: i32) -> Option<u16> {
        self.tela.objetos_da_faixa_em(x, y).iter().find_map(|n| self.partida.cfg.funcao_do_botao(n))
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
