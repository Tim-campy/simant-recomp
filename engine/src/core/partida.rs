//! Laço de jogo ao vivo, sem janela: guarda o estado da simulação, as regras, a navegação da janela
//! de edição e o relógio, e decide quando rodar um tick.
//!
//! Especificação: docs/spec/regras/simulacao-visao-geral.md (§1–4: `MYTIMERFUNC`, velocidade e
//! pausa) e docs/spec/telas/jogo.md (seção "Simulação ao vivo"). Dados: data/partida.json.
//!
//! O executável chama [`Partida::avancar`] com o tempo real passado; aqui o tempo vira "mensagens
//! de temporizador" (o `SetTimer` de 17 ms, que no Windows 3.1 dispara a cada 55 ms), no laço de
//! `MYTIMERFUNC` (3 mensagens por chamada); cada volta passa pela regra de velocidade e pausa do
//! [`Relogio`]. Entradas do jogador (clique numa célula, tecla de comando, aceleradores) chegam já
//! sem SDL. Os pedidos das regras à interface (mensagens, músicas, fim de jogo) ficam numa fila que a
//! interface esvazia; o pedido de centralizar a vista na formiga amarela é tratado aqui mesmo.
//!
//! Avisos (core/avisos.rs, docs/spec/telas/mensagens.md): os quadros de mensagem e o fim de jogo
//! param a simulação enquanto estão na tela (`_StopSimulation`); a mensagem da faixa segue a pausa,
//! os conselhos de `_Feedback` e o botão de velocidade.

use std::collections::{HashMap, VecDeque};

use serde::Deserialize;

use super::avisos::{Avisos, ConfigAvisos, Modal};
use super::navegacao::{Acao, Evento as EventoTecla, Navegacao};
use super::quadros_animados::TipoAnimado;
use crate::rules::amarela::{
    clicar_no_mapa, comando, tecla_de_comando, ControleJogador, Modificadores, RespostaClique, RespostaTecla,
    TabelasAmarela,
};
use crate::rules::desenho::{self, ContadoresRedesenho, JanelasDesenho};
use crate::rules::estado::{AnimacaoDaAmarela, ConfigSimulacao, Estado, Evento, QuadroAnimacao};
use crate::rules::tick::{tick, Regras, RegrasOriginais, Relogio};
use crate::rules::trapacas::{self, letra_da_tecla, AnelTrapaca, EfeitoTrapaca, TabelasTrapaca};
use crate::scripts::{Gancho, PedidoMod, Scripts};

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_CONFIG: &str = "partida.json";

/// Ritmo das mensagens de temporizador e do redesenho (data/partida.json, "relogio").
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConfigRelogio {
    /// Intervalo pedido ao `SetTimer` com o aplicativo em foco (17 ms).
    pub intervalo_mensagem_ms: u32,
    /// Intervalo sem foco (`WM_ACTIVATEAPP` com wParam = 0: 170 ms).
    pub intervalo_sem_foco_ms: u32,
    /// Resolução do temporizador do Windows 3.1 (55 ms, 18,2 Hz): o intervalo pedido é arredondado
    /// para cima até um múltiplo dela (o de 17 ms vira 55 ms).
    pub resolucao_timer_ms: u32,
    /// Voltas do laço ocupado de `MYTIMERFUNC` entre duas mensagens, por `_GameSpeed` (depende da
    /// CPU no original; os padrões vêm da bancada).
    pub voltas_por_periodo: Vec<u32>,
    /// Máximo de mensagens geradas numa chamada de `avancar`; o resto do tempo é descartado.
    pub max_mensagens_por_avanco: u32,
    /// Redesenho do mapa a cada N ticks, por `_GameSpeed` (`_UpdateWindows`).
    pub redesenho_mapa_a_cada: Vec<u32>,
    /// Sem foco, redesenha só a cada N ticks (`quadros & 7 == 0`).
    pub redesenho_sem_foco_a_cada: u32,
}

/// Estado de uma chamada de `MYTIMERFUNC` em andamento (simulacao-visao-geral.md §2): o laço só
/// devolve o controle ao Windows depois de drenar 2 ou 3 `WM_TIMER` novas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LacoTimer {
    /// Dentro do laço (a mensagem seguinte é drenada, não abre outra chamada).
    dentro: bool,
    /// `WM_TIMER` retiradas por `PeekMessage` nesta chamada.
    drenadas: u32,
    /// Voltas do laço nesta chamada.
    voltas: u32,
    /// Mensagens que chegaram e serão drenadas no fim da volta em curso.
    pendentes: u32,
}

/// Ações dos aceleradores do menu Speed e Options.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcaoPartida {
    /// Speed → Pause (0x41): alterna `_GamePaused`.
    Pausa,
    /// Speed → Slow/Normal/Fast/Ultra (0x43–0x46): `_GameSpeed` = 0..3.
    VelocidadeLenta,
    VelocidadeNormal,
    VelocidadeRapida,
    VelocidadeUltra,
    /// Options → AutoTrack (0x31): alterna `_OptionStates[0]`.
    AutoTrack,
    /// File → Save (5, Ctrl+S) e Save As (6, Ctrl+A): tratados pela interface (arquivo).
    Salvar,
    SalvarComo,
    /// File → Load (`_LoadGame`): pergunta, diálogo e nova sessão, a cargo da interface.
    Carregar,
    /// Window → Edit/Behavior/Caste/History (0x11, 0x13–0x15; Ctrl+E/B/C/H): abre ou traz para a
    /// frente, a cargo da interface.
    JanelaEdicao,
    JanelaComportamento,
    JanelaCastas,
    JanelaHistorico,
    /// Window → Info (0x16): a cargo da interface (Ctrl+I é extensão do port).
    JanelaInfo,
}

/// Resultado de um botão configurável da faixa (`_DoUserButton`, `1000:04c4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RespostaBotao {
    /// Feito aqui (pausa, velocidade, opções, ir até a amarela).
    Feito,
    /// Caso 0xE: salvar o jogo (`_SaveGame(1)`), a cargo da interface.
    Salvar,
    /// Funções que esta camada não trata: 0–3 (abrir Behavior, Caste, History e Info), que a
    /// interface trata antes de chamar aqui (`simant/src/sessao.rs`), ou número desconhecido.
    NaoImplementado(u16),
}

/// Marcadores da faixa (`ribBM1`–`7`, `_DoBookMark`): objetos e bitmaps do marcador posto
/// (`base + índice`, selecionado e normal), som de erro.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConfigMarcadores {
    pub objetos: Vec<String>,
    pub bitmap_posto: [u16; 2],
    pub som_vazio: u16,
}

/// Botões configuráveis da faixa (`ribUserDef1A`–`1H`) e a função de cada um.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct BotoesUsuario {
    pub objetos: Vec<String>,
    pub funcoes: Vec<u16>,
    /// `_SetUserButton`: bitmaps da função f = base + f (0x3889, 0x3857).
    pub bitmap_selecionado: u16,
    pub bitmap_normal: u16,
}

/// Tecla → ação de partida (mesmas regras de `navegacao::Atalho`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AtalhoPartida {
    pub tecla: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    pub acao: AcaoPartida,
}

/// Botão do mouse que uma tecla imita (Espaço/Insert = esquerdo, Delete = direito).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BotaoTeclado {
    Esquerdo,
    Direito,
}

/// Conteúdo de data/partida.json.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConfigPartida {
    pub relogio: ConfigRelogio,
    pub teclas: Vec<AtalhoPartida>,
    /// Nome da tecla no SDL → código de tecla virtual do Windows para `_YellowCommandKey`.
    pub teclas_amarela: HashMap<String, u16>,
    /// Teclas que valem como clique do mouse na posição do cursor.
    pub teclas_clique: HashMap<String, BotaoTeclado>,
    /// Intervalo máximo entre duas dessas teclas para valer como duplo clique.
    pub duplo_clique_teclado_ms: u64,
    /// Espera de `_processEdit` no clique simples, em unidades de `_TickCount` (6).
    pub espera_clique_simples_tickcount: u64,
    pub botoes_usuario: BotoesUsuario,
    /// Botões configuráveis da faixa 35 (`ribUserDef2A`–`2H`, tabela `1048:0022`). Opcional (API
    /// 0.22): sem ele, a faixa do quintal não tem botões configuráveis.
    #[serde(default)]
    pub botoes_usuario_quintal: Option<BotoesUsuario>,
    pub marcadores: ConfigMarcadores,
}

impl ConfigPartida {
    pub fn de_json(texto: &str) -> Result<ConfigPartida, String> {
        serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_CONFIG}: {e}"))
    }

    /// Ação do acelerador ligado à tecla, se houver (a primeira da lista que bater).
    pub fn acao_da_tecla(&self, e: &EventoTecla) -> Option<AcaoPartida> {
        let EventoTecla::Tecla { nome, ctrl, shift, alt } = e;
        self.teclas
            .iter()
            .find(|a| a.tecla.eq_ignore_ascii_case(nome) && a.ctrl == *ctrl && a.shift == *shift && a.alt == *alt)
            .map(|a| a.acao)
    }

    /// Código de tecla virtual da tecla de comando da amarela, se houver.
    pub fn tecla_virtual(&self, nome: &str) -> Option<u16> {
        self.teclas_amarela.iter().find(|(k, _)| k.eq_ignore_ascii_case(nome)).map(|(_, &v)| v)
    }

    /// Função do botão configurável da faixa com esse nome de objeto, se for um.
    pub fn funcao_do_botao(&self, objeto: &str) -> Option<u16> {
        let b = &self.botoes_usuario;
        b.objetos.iter().position(|o| o == objeto).and_then(|i| b.funcoes.get(i).copied())
    }

    /// Botão do mouse imitado pela tecla, se houver.
    pub fn botao_da_tecla(&self, nome: &str) -> Option<BotaoTeclado> {
        self.teclas_clique.iter().find(|(k, _)| k.eq_ignore_ascii_case(nome)).map(|(_, &v)| v)
    }
}

/// `_ELayerMode` que o botão de cheiro da faixa (`_MapMode` n) liga: 4 ninho preto → 1, 5 trilha preta
/// → 2, 6 ninho vermelho → 3, 7 trilha vermelha → 4, 8 alarme → 0 (`_ProcMapRibbonEvent`, `1010:c4f0`).
pub fn camada_do_modo_de_cheiro(n: i16) -> i16 {
    match n {
        4..=7 => n - 3,
        8 => 0,
        _ => -1,
    }
}

/// Resultado de [`Partida::avancar`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Avanco {
    /// Mensagens de temporizador geradas.
    pub mensagens: u32,
    /// Ticks rodados.
    pub ticks: u32,
}

/// Uma partida em andamento.
pub struct Partida {
    pub estado: Estado,
    pub regras: RegrasOriginais,
    pub tabelas_amarela: TabelasAmarela,
    pub controle: ControleJogador,
    /// Plano e vista da janela de edição (a "interface" do original, sem janela).
    pub nav: Navegacao,
    pub cfg_simulacao: ConfigSimulacao,
    pub cfg: ConfigPartida,
    pub relogio: Relogio,
    /// Chamada de `MYTIMERFUNC` em andamento.
    laco: LacoTimer,
    /// Tempo real acumulado que ainda não completou uma mensagem.
    resto_ms: u64,
    /// Tempo real total passado a `avancar` (o `GetTickCount` do port, ms).
    ms_total: u64,
    /// Relógio do Mac no início da partida: o maior dos prazos que o save guarda nesse relógio
    /// (`_CatDelay`, `_DogBarkDelay`, `_BoyMsgCnt`), como num Windows ligado há mais tempo que o
    /// da sessão que salvou (decisão do port; quintal.md, "Relógios do port").
    mac_inicial: u32,
    /// O aplicativo tem o foco (`WM_ACTIVATEAPP`).
    ativo: bool,
    /// Ticks com redesenho (DGROUP `0x02F0`, "quadros").
    quadros: u64,
    /// Contador do redesenho do mapa em fast/ultra (DGROUP `0x9B00`).
    contador_mapa: u32,
    /// Algo mudou na tela desde a última vez que a interface perguntou.
    redesenhar: bool,
    /// O pedido de `_EndGameDialog` já foi repassado (enquanto `_IsGameOver` ≠ 0).
    fim_anunciado: bool,
    eventos: VecDeque<Evento>,
    /// Mensagem da faixa e avisos modais.
    pub avisos: Avisos,

    /// Tempo real em que a simulação fica parada pela espera do clique simples (ms).
    bloqueio_ms: u64,
    /// `_PauseGame(1)` do Load que espera os quadros abertos no `_FullCount` fecharem.
    pausar_ao_fechar: bool,
    /// `_BookMarks` (`8:8aa0`, fora do save): plano e canto da vista de cada marcador.
    pub marcadores: [Option<(i16, i32, i32)>; 7],
    /// Função de cada botão configurável da faixa (`1048:0012`), trocada pelo menu de ícones.
    pub funcoes_botoes: Vec<u16>,
    /// Os da faixa 35 (`_yardUserButton`, `1048:0022`).
    pub funcoes_botoes_quintal: Vec<u16>,
    /// `_showTrails` (botão 4): trilhas na janela do mapa — os rastros das formigas acumulados, não
    /// o feromônio (telas/mapa.md §2.6).
    pub mostrar_trilhas: bool,
    /// Janelas abertas/na frente e minimizada, atualizadas pela interface: decidem os sorteios do
    /// desenho depois de cada tick (`rules::desenho`, rng.md "Sorteios do desenho"). `ativa` vem do
    /// foco (`definir_foco`) e `aranha_visivel` é calculado no tick com a vista.
    pub janelas_desenho: JanelasDesenho,
    /// Contadores de `MYTIMERFUNC`/`_UpdateWindows` (fora do save).
    pub contadores_redesenho: ContadoresRedesenho,
    /// `_SpecialXfer` pedido pelo tick e ainda não escolhido (a simulação fica parada).
    pub escolher_colonia: bool,
    /// A interface está num laço modal do original que para a simulação (`_StopSimulation`), como o
    /// PLACE esperando o clique (`_PlaceQueenInYard`).
    pub parada_pela_interface: bool,
    /// Efeito do último Mystery (para a interface escrever).
    pub ultimo_misterio: Option<crate::rules::misterio::EfeitoMisterio>,
    /// Scripts Lua dos mods (docs/api-mods.md, "Scripts Lua"). `None`: nenhum mod com script, e o
    /// laço é exatamente o de sempre.
    pub scripts: Option<Scripts>,
    /// Linhas de registro dos scripts ainda não retiradas pela interface.
    registro_scripts: Vec<String>,
    /// Códigos de trapaça do original (`data/codigos_trapaca.json`) e o anel de letras (fora do
    /// save; rules/trapacas.rs).
    pub tabelas_trapaca: TabelasTrapaca,
    anel_trapaca: AnelTrapaca,
    /// Ticks pedidos por `avancar_ticks` dos scripts (API 0.17) ainda por rodar.
    ticks_de_mods: u32,
    /// Animações da amarela pedidas no último tick (`_AnimYellowFight`, `_AnimYellowInsane`), em
    /// ordem, e o `ms_total` em que a primeira começou. Enquanto duram, a simulação fica parada
    /// (`bloqueio_ms`) e a janela de edição mostra o quadro da vez (`quadro_animado`).
    animacoes: Vec<AnimacaoDaAmarela>,
    animacao_inicio_ms: u64,
    /// Avisos modais pedidos antes da animação no mesmo tick (o quadro da aranha ou da formiga-leão
    /// vem antes de `_AnimYellowFight`): a animação só começa quando eles fecham.
    modais_antes_da_animacao: usize,
    /// Quadro mostrado no último redesenho (animação, quadro), para só redesenhar quando muda.
    quadro_mostrado: Option<(usize, usize)>,
    /// Rodando os ticks pedidos pelos mods de uma vez: as animações não param nada.
    avanco_rapido: bool,
}

impl Partida {
    pub fn nova(
        estado: Estado,
        nav: Navegacao,
        regras: RegrasOriginais,
        tabelas_amarela: TabelasAmarela,
        cfg_simulacao: ConfigSimulacao,
        cfg: ConfigPartida,
        cfg_avisos: ConfigAvisos,
    ) -> Partida {
        let (b, m) = (&estado.bichos, &estado.menino);
        let mac_inicial = [b.gato_atraso, b.cachorro_atraso_latido, m.boy_msg_cnt].into_iter().max().unwrap_or(0).max(0) as u32;
        let mut p = Partida {
            estado,
            regras,
            tabelas_amarela,
            controle: ControleJogador::default(),
            nav,
            cfg_simulacao,
            cfg,
            relogio: Relogio::default(),
            laco: LacoTimer::default(),
            resto_ms: 0,
            ms_total: 0,
            mac_inicial,
            ativo: true,
            quadros: 0,
            contador_mapa: 0,
            redesenhar: true,
            fim_anunciado: false,
            eventos: VecDeque::new(),
            avisos: Avisos::new(cfg_avisos),

            bloqueio_ms: 0,
            pausar_ao_fechar: false,
            marcadores: [None; 7],
            funcoes_botoes: Vec::new(),
            funcoes_botoes_quintal: Vec::new(),
            mostrar_trilhas: false,
            janelas_desenho: JanelasDesenho { ativa: true, edicao_aberta: true, edicao_na_frente: true, ..Default::default() },
            contadores_redesenho: ContadoresRedesenho::default(),
            ultimo_misterio: None,
            escolher_colonia: false,
            parada_pela_interface: false,
            scripts: None,
            registro_scripts: Vec::new(),
            tabelas_trapaca: TabelasTrapaca::padrao().clone(),
            anel_trapaca: AnelTrapaca::default(),
            ticks_de_mods: 0,
            animacoes: Vec::new(),
            animacao_inicio_ms: 0,
            modais_antes_da_animacao: 0,
            quadro_mostrado: None,
            avanco_rapido: false,
        };
        p.funcoes_botoes = p.cfg.botoes_usuario.funcoes.clone();
        p.funcoes_botoes_quintal = p.cfg.botoes_usuario_quintal.as_ref().map(|b| b.funcoes.clone()).unwrap_or_default();
        p.vistas_para_estado();
        p
    }

    // --- relógio -------------------------------------------------------------------------------

    /// `_GameSpeed` como índice das tabelas (0 slow … 3 ultra; fora disso, o último).
    fn indice_velocidade<T: Copy>(&self, tabela: &[T], padrao: T) -> T {
        usize::try_from(self.estado.partida.velocidade)
            .ok()
            .and_then(|v| tabela.get(v).copied())
            .or(tabela.last().copied())
            .unwrap_or(padrao)
    }

    /// Intervalo atual entre mensagens de temporizador (ms): o pedido ao `SetTimer` (17 ms, ou
    /// 170 ms sem foco) arredondado para cima até a resolução do temporizador do Windows 3.1 (55 ms).
    pub fn intervalo_ms(&self) -> u64 {
        let r = &self.cfg.relogio;
        let pedido = u64::from(if self.ativo { r.intervalo_mensagem_ms } else { r.intervalo_sem_foco_ms }).max(1);
        let resolucao = u64::from(r.resolucao_timer_ms).max(1);
        pedido.div_ceil(resolucao) * resolucao
    }

    /// Foco do aplicativo (`WM_ACTIVATEAPP`): sem foco, o temporizador fica em 170 ms.
    pub fn definir_foco(&mut self, ativo: bool) {
        self.ativo = ativo;
    }

    /// `_TickCount` (unidades de 55 ms) do relógio real da partida.
    pub fn agora(&self) -> u64 {
        self.ms_total / self.avisos.cfg.ms_por_tickcount.max(1)
    }

    /// Passa `ms` de tempo real: gera as mensagens de temporizador (a cada `intervalo_ms`) que couberem (no máximo
    /// `max_mensagens_por_avanco`, descartando o excesso) e roda os ticks que a velocidade e a
    /// pausa liberarem. Com um aviso modal na tela, ou durante a espera do clique simples, o tempo
    /// passa sem mensagens (o original mata o temporizador ou fica num laço de espera).
    pub fn avancar(&mut self, ms: u64) -> Avanco {
        // O GetTickCount continua andando durante quadros modais e esperas.
        self.ms_total += ms;
        if self.animando() {
            // _AnimYellowFight: o programa inteiro espera; um quadro modal pedido no mesmo tick (a
            // morte depois da luta perdida) só aparece depois da animação.
            let espera = ms.min(self.bloqueio_ms);
            self.bloqueio_ms -= espera;
            self.acompanhar_animacao();
            if self.animando() {
                return Avanco::default();
            }
            self.resto_ms = 0;
            return Avanco::default();
        }
        let sobreposta = self.avisos.sobreposta_visivel();
        let animado = self.avisos.animado().cloned();
        if self.avisos.passar_tempo(self.agora()).is_some() {
            self.redesenhar = true;
            self.modal_fechado();
            self.depois_de_fechar();
        }
        self.redesenhar |= sobreposta != self.avisos.sobreposta_visivel() || animado.as_ref() != self.avisos.animado();
        let pedidos = self.avisos.retirar_pedidos();
        self.eventos.extend(pedidos);
        if self.animando() {
            // Fecharam os quadros que vinham antes da animação: ela começa agora.
            return Avanco::default();
        }
        if self.avisos.tem_modal() || self.escolher_colonia || self.parada_pela_interface {
            self.resto_ms = 0;
            self.laco = LacoTimer::default();
            return Avanco::default();
        }
        let espera = ms.min(self.bloqueio_ms);
        self.bloqueio_ms -= espera;
        let ms = ms - espera;
        if self.bloqueio_ms > 0 {
            return Avanco::default();
        }
        if self.ticks_de_mods > 0 {
            return self.rodar_ticks_de_mods();
        }
        let intervalo = self.intervalo_ms();
        self.resto_ms += ms;
        let mut n = self.resto_ms / intervalo;
        self.resto_ms %= intervalo;
        let max = u64::from(self.cfg.relogio.max_mensagens_por_avanco);
        if n > max {
            n = max;
        }
        let mut av = Avanco::default();
        for _ in 0..n {
            av.mensagens += 1;
            av.ticks += self.mensagem_de_timer();
            if self.bloqueio_ms > 0 {
                // Uma animação da amarela parou o programa (_AnimYellowFight): o tempo que sobrou
                // deste avanço não vira mensagens.
                self.resto_ms = 0;
                break;
            }
            if self.avisos.tem_modal() {
                // _StopSimulation: KillTimer, e as WM_TIMER pendentes são descartadas.
                self.resto_ms = 0;
                self.laco = LacoTimer::default();
                self.redesenhar = true;
                break;
            }
        }
        av
    }

    /// Uma mensagem `WM_TIMER` (a cada 55 ms): fora do laço, abre uma chamada de `MYTIMERFUNC`
    /// (atraso + 1); dentro dele, a mensagem é drenada pelo `PeekMessage` no fim da volta em curso
    /// (atraso + 1, drenadas + 1). Em seguida rodam as voltas do laço ocupado até a próxima mensagem
    /// (`voltas_por_periodo`): cada uma soma 1 a `_gameCycles`, roda no máximo um tick (se
    /// `Relogio::liberar_tick` deixar) e termina com o teste de saída do original (drenadas > 1 e
    /// (voltas > 2 ou drenadas > 2)). Devolve os ticks rodados. Ver simulacao-visao-geral.md §2–3.
    pub fn mensagem_de_timer(&mut self) -> u32 {
        if self.laco.dentro {
            self.laco.pendentes += 1;
        } else {
            // Minimizado: o ícone animado sorteia SRand1(7) a cada chamada, se não pausado nem no fim.
            if self.janelas_desenho.minimizada && self.estado.partida.pausado == 0 && self.estado.partida.fim_de_jogo == 0 {
                self.estado.s.rand1(7);
            }
            self.relogio.contar_mensagem();
            self.laco = LacoTimer { dentro: true, ..LacoTimer::default() };
        }
        let voltas = self.indice_velocidade(&self.cfg.relogio.voltas_por_periodo.clone(), 1).max(1);
        let mut ticks = 0;
        let havia_modal = self.avisos.tem_modal();
        for _ in 0..voltas {
            // `_gameCycles` (9:9b2c) conta as voltas do laço, rode ou não o tick. Zero significa
            // "sem relógio do laço" para `quintal::Relogios`: pula o 0 na volta.
            self.estado.ciclos_jogo = self.estado.ciclos_jogo.wrapping_add(1).max(1);
            if self.relogio.liberar_tick(&self.estado, &self.cfg_simulacao) {
                self.rodar_tick();
                ticks += 1;
                if self.bloqueio_ms > 0 {
                    // Animação dentro do tick: o port encerra a chamada (a espera fica no avanço).
                    self.laco = LacoTimer::default();
                    return ticks;
                }
                if !havia_modal && self.avisos.tem_modal() {
                    // _StopSimulation: o quadro para o temporizador; o port encerra a chamada.
                    self.laco = LacoTimer::default();
                    return ticks;
                }
            }
            let l = &mut self.laco;
            for _ in 0..l.pendentes {
                self.relogio.contar_mensagem();
            }
            l.drenadas += std::mem::take(&mut l.pendentes);
            l.voltas += 1;
            if l.drenadas > 1 && (l.voltas > 2 || l.drenadas > 2) {
                l.dentro = false;
                break;
            }
        }
        ticks
    }

    /// Um tick da simulação (`_DoAntSim`), sem olhar relógio nem pausa, e o que o original faz em
    /// seguida na interface (redesenho, vista).
    pub fn rodar_tick(&mut self) {
        self.vistas_para_estado();
        // `_MacTickCount` (1000:9694) = ⌊GetTickCount / 55⌋ × 3: tempo real (quintal.md, "Relógios").
        self.estado.relogio_mac = Some(self.mac_inicial.wrapping_add(((self.ms_total / 55) * 3) as u32));
        if let Some(s) = self.scripts.as_mut().filter(|s| s.tem_gancho(Gancho::AntesDoTick)) {
            s.antes_do_tick(&mut self.estado);
            self.aplicar_pedidos_dos_mods();
        }
        self.preparar_edicao_no_tick();
        tick(&mut self.estado, &mut self.regras as &mut dyn Regras);
        if let Some(s) = self.scripts.as_mut().filter(|s| s.tem_gancho(Gancho::DepoisDoTick)) {
            s.depois_do_tick(&mut self.estado);
            self.aplicar_pedidos_dos_mods();
        }
        self.vistas_do_estado();
        self.tratar_eventos();
        self.conselho_na_faixa();
        // _UpdateWindows: os sorteios do desenho (quintal com chuva/enxame, palpos da aranha).
        let v = &self.nav.vista;
        let aranha = desenho::aranha_visivel(&self.estado, self.nav.plano_mapa, v.x, v.y, v.largura, v.altura);
        let som_cortador = self.regras.quintal.sons.cortador;
        let tick_count = (self.ms_total / 55) as u32;
        let janelas = JanelasDesenho { ativa: self.ativo, aranha_visivel: aranha, som_cortador, tick_count, ..self.janelas_desenho };
        let r = &self.regras;
        let tabelas = desenho::TabelasDesenho { baloes: &r.baloes, d8: (&r.inimigos.dx8, &r.inimigos.dy8) };
        desenho::redesenho_do_timer(&mut self.estado, &janelas, &mut self.contadores_redesenho, &tabelas);
        // _UpdateWindows: em fast/ultra o mapa só é redesenhado a cada 2/4 ticks; sem foco, a cada 8.
        self.contador_mapa = self.contador_mapa.wrapping_add(1);
        self.quadros += 1;
        let a_cada = self.indice_velocidade(&self.cfg.relogio.redesenho_mapa_a_cada.clone(), 1).max(1);
        let mut desenha = self.contador_mapa.is_multiple_of(a_cada);
        if !self.ativo {
            desenha &= self.quadros.is_multiple_of(u64::from(self.cfg.relogio.redesenho_sem_foco_a_cada.max(1)));
        }
        self.redesenhar |= desenha;
    }

    /// A janela de edição para os desenhos de dentro do tick (`desenho::EdicaoNoTick`): aberta ou não
    /// (`janelas_desenho.edicao_aberta`), o tamanho da vista e o `_TickCount` dos balões. `_UpdateEdit`
    /// só confere `_win_IsWinOpen(0)`; com o programa minimizado o port também desenha [hipótese].
    fn preparar_edicao_no_tick(&mut self) {
        if !self.janelas_desenho.edicao_aberta {
            self.estado.edicao_no_tick = None;
            return;
        }
        let (w, h) = (self.nav.vista.largura, self.nav.vista.altura);
        let tick_count = (self.ms_total / 55) as u32;
        let r = &self.regras;
        let j = self.estado.edicao_no_tick.get_or_insert_with(|| {
            let baloes = std::sync::Arc::new(r.baloes.clone());
            Box::new(desenho::EdicaoNoTick::new(w, h, baloes, (r.inimigos.dx8, r.inimigos.dy8)))
        });
        j.largura = w;
        j.altura = h;
        j.tick_count = tick_count;
    }

    /// Botão MATE do quintal (`_ProcYardEvent` 0x2306): arma a revoada preta ou recusa, com a
    /// mensagem na faixa.
    pub fn botao_revoada(&mut self) -> bool {
        let armou = crate::rules::quintal::botao_revoada(&mut self.estado, &self.regras.quintal);
        self.processar_eventos_do_estado();
        armou
    }

    /// Quadro de animação da amarela a mostrar agora na janela de edição: o plano animado e o
    /// quadro (mapa, vida e marca da amarela naquele instante). Cada quadro fica `espera_ms` na tela;
    /// o último aparece só no fim da espera (no original o tick continua logo depois dele).
    pub fn quadro_animado(&self) -> Option<(i16, &QuadroAnimacao)> {
        let (k, q) = self.indice_do_quadro()?;
        let a = &self.animacoes[k];
        Some((a.plano, &a.quadros[q]))
    }

    /// Há uma animação da amarela em andamento (a simulação está parada por ela). Uma animação que
    /// espera os quadros pedidos antes dela ainda não conta.
    pub fn animando(&self) -> bool {
        !self.animacoes.is_empty() && self.modais_antes_da_animacao == 0
    }

    /// Um aviso modal fechou: se a animação esperava por ele, e era o último, ela começa.
    fn modal_fechado(&mut self) {
        if self.modais_antes_da_animacao == 0 {
            return;
        }
        self.modais_antes_da_animacao -= 1;
        if self.modais_antes_da_animacao == 0 && !self.animacoes.is_empty() {
            self.animacao_inicio_ms = self.ms_total;
            self.bloqueio_ms += self.animacoes.iter().map(|a| a.duracao_ms()).sum::<u64>();
            self.quadro_mostrado = None;
            self.redesenhar = true;
        }
    }

    /// Clique ou tecla com um quadro animado na frente (`_SpiderDialog`, `_LionDialog`,
    /// `_DrawSimPayoff`): encerra a espera da vez. Devolve o aviso, se fechou.
    pub fn pular_modal(&mut self) -> Option<Modal> {
        self.redesenhar = true;
        let agora = self.agora();
        let m = self.avisos.pular_animado(agora);
        let pedidos = self.avisos.retirar_pedidos();
        self.eventos.extend(pedidos);
        if m.is_some() {
            self.modal_fechado();
            self.depois_de_fechar();
        }
        m
    }

    fn indice_do_quadro(&self) -> Option<(usize, usize)> {
        if self.modais_antes_da_animacao > 0 {
            return None;
        }
        let mut t = self.ms_total.saturating_sub(self.animacao_inicio_ms);
        for (k, a) in self.animacoes.iter().enumerate() {
            let d = a.duracao_ms();
            if t < d {
                return Some((k, (t / u64::from(a.espera_ms.max(1))) as usize));
            }
            t -= d;
        }
        None
    }

    /// Depois de passar tempo: troca o quadro mostrado (pede redesenho) e, acabadas as esperas,
    /// encerra as animações.
    fn acompanhar_animacao(&mut self) {
        if self.animacoes.is_empty() {
            return;
        }
        let atual = self.indice_do_quadro();
        if atual.is_none() || self.bloqueio_ms == 0 {
            self.animacoes.clear();
            self.quadro_mostrado = None;
            self.redesenhar = true;
        } else if atual != self.quadro_mostrado {
            self.quadro_mostrado = atual;
            self.redesenhar = true;
        }
    }

    /// Devolve se a tela deve ser redesenhada (e limpa o pedido).
    pub fn tomar_redesenho(&mut self) -> bool {
        std::mem::take(&mut self.redesenhar)
    }

    /// Depois de uma mudança no estado feita fora do tick (troca de patch, ferramentas do
    /// Experimental): trata a fila de eventos do estado como depois de um tick e pede redesenho.
    pub fn processar_eventos_do_estado(&mut self) {
        self.tratar_eventos();
        self.redesenhar = true;
    }

    /// Retira os pedidos das regras à interface (mensagens, músicas, fim de jogo), em ordem.
    pub fn retirar_eventos(&mut self) -> Vec<Evento> {
        self.eventos.drain(..).collect()
    }

    // --- vistas: navegação ↔ estado ------------------------------------------------------------

    /// As regras leem o plano mostrado (`_MapPlane`) e os pontos lembrados; a navegação é a dona.
    pub(crate) fn vistas_para_estado(&mut self) {
        let e = &mut self.estado;
        let n = &self.nav;
        let par = |(x, y): (i32, i32)| (x as i16, y as i16);
        // _SetMapPlane termina com _SetMapModeAnt(_MapPlane): trocar de plano volta ao mapa simples
        // dele (telas/mapa.md); no mesmo plano, o modo de feromônio escolhido fica.
        if e.vistas.plano != n.plano_mapa {
            e.vistas.modo_mapa = n.plano_mapa;
        }
        e.vistas.plano = n.plano_mapa;
        e.vistas.ponto = (n.vista.x as i16, n.vista.y as i16);
        e.baloes.vista = (n.vista.largura as i16, n.vista.altura as i16);
        e.vistas.ponto_quintal = par(n.pontos[0]);
        e.vistas.ponto_superficie = par(n.pontos[1]);
        e.preta.vista_ninho = par(n.pontos[2]);
        e.vermelha.vista_ninho = par(n.pontos[3]);
    }

    /// Pontos lembrados que as regras possam ter mudado voltam para a navegação.
    fn vistas_do_estado(&mut self) {
        let e = &self.estado;
        let par = |(x, y): (i16, i16)| (x as i32, y as i32);
        self.nav.pontos = [
            par(e.vistas.ponto_quintal),
            par(e.vistas.ponto_superficie),
            par(e.preta.vista_ninho),
            par(e.vermelha.vista_ninho),
        ];
    }

    /// Esvazia a fila do estado: centralizar é feito aqui (`_GotoMyAnt`); o resto vai para a
    /// interface. `_EndGameDialog` é pedido a cada tick enquanto o jogo acabou: só o primeiro é
    /// repassado (de novo só depois que `_IsGameOver` voltar a 0). Mensagens e fim de jogo também
    /// entram nos avisos (quadros modais).
    fn tratar_eventos(&mut self) {
        for ev in self.estado.retirar_eventos() {
            if let Some(s) = self.scripts.as_mut().filter(|s| s.tem_gancho(Gancho::AoEvento)) {
                s.ao_evento(&ev, &mut self.estado);
            }
            match ev {
                Evento::CentralizarNaAmarela { plano, x, y } => self.ir_para_amarela(plano, x, y),
                Evento::DialogoFimDeJogo => {
                    if !self.fim_anunciado {
                        self.fim_anunciado = true;
                        self.eventos.push_back(ev);
                        // _EndGameDialog: música de vitória ou derrota com Options → Music ligado.
                        if self.estado.partida.opcoes[1] != 0 {
                            let f = &self.avisos.cfg.fim_de_jogo;
                            let m = if self.estado.partida.pretas_venceram != 0 { f.musica_vitoria } else { f.musica_derrota };
                            self.eventos.push_back(Evento::Musica(m));
                        }
                        let agora = self.agora();
                        self.avisos.receber_fim_de_jogo(&self.estado, agora);
                    }
                }
                Evento::MensagemFaixa(m) => {
                    use super::avisos::TextoFaixa;
                    let agora = self.agora();
                    let opcao = self.estado.partida.opcoes[4] != 0;
                    // Lista 0: limpar (`_EditMessage(0, …)`); duração 0xFFFE: permanente.
                    let texto = (m.lista != 0).then_some(TextoFaixa::Lista { lista: m.lista, indice: m.indice as usize });
                    let duracao = (m.duracao != 0xFFFE && m.lista != 0).then_some(u32::from(m.duracao));
                    self.avisos.editar(texto, duracao, m.importante, opcao, agora);
                    self.redesenhar = true;
                }
                Evento::EscolherColonia => {
                    // _SpecialXfer: a interface abre o quintal e espera o clique numa colônia, depois
                    // do quadro 10058; a simulação fica parada até lá.
                    self.escolher_colonia = true;
                    self.eventos.push_back(ev);
                }
                Evento::QuadroAmarela { imagem, indice } => {
                    // _YellowDialog: só com Options → Events (_OptionStates[3]).
                    if self.estado.partida.opcoes[3] != 0 {
                        self.avisos.receber_quadro_amarela(imagem, indice);
                    }
                    self.eventos.push_back(ev);
                }
                Evento::DialogoAranha { texto } => {
                    // _SpiderDialog: as regras já conferiram Events e sortearam o texto; a animação da
                    // aranha (janela 26) e depois o quadro com o texto.
                    self.avisos.receber_animado(TipoAnimado::Aranha, Some(texto));
                    self.eventos.push_back(ev);
                }
                Evento::QuadroFormigaLeao => {
                    self.avisos.receber_animado(TipoAnimado::FormigaLeao, None);
                    self.eventos.push_back(ev);
                }
                Evento::AnimacaoVitoria => {
                    // _DrawSimPayoff: a interface traz o quintal para a frente (_MapToYard, modo 0,
                    // plano 0) e a janela 26 vem por cima.
                    self.avisos.receber_animado(TipoAnimado::Vitoria, None);
                    self.eventos.push_back(ev);
                }
                Evento::Mensagem(id) => {
                    let licoes = &self.regras.quintal.conselhos.licoes;
                    self.avisos.receber_mensagem(id, self.estado.partida.opcoes[3] != 0, licoes);
                    self.eventos.push_back(ev);
                }
                Evento::Animacao(a) => {
                    if !self.avanco_rapido && !a.quadros.is_empty() {
                        if self.animacoes.is_empty() {
                            self.animacao_inicio_ms = self.ms_total;
                            // Os quadros pedidos antes, no mesmo tick (aranha, formiga-leão), vêm
                            // primeiro: a animação espera eles fecharem.
                            self.modais_antes_da_animacao = self.avisos.modais_na_fila();
                        }
                        if self.modais_antes_da_animacao == 0 {
                            self.bloqueio_ms += a.duracao_ms();
                        }
                        self.animacoes.push(*a);
                        self.quadro_mostrado = None;
                        self.redesenhar = true;
                    }
                }
                outro => self.eventos.push_back(outro),
            }
        }
        if self.estado.partida.fim_de_jogo == 0 {
            self.fim_anunciado = false;
        }
        self.aplicar_pedidos_dos_mods();
        self.saidas_dos_scripts();
    }

    // --- scripts dos mods ----------------------------------------------------------------------

    /// Liga os scripts dos mods a esta partida e chama `ao_iniciar`.
    pub fn ligar_scripts(&mut self, mut scripts: Scripts) {
        scripts.ao_iniciar(&mut self.estado);
        self.scripts = Some(scripts);
        self.aplicar_pedidos_dos_mods();
        self.saidas_dos_scripts();
        self.redesenhar = true;
    }

    /// O jogador escolheu o item `indice` (a partir de 1) do menu do mod `nome` (API 0.17):
    /// `ao_menu`, as ações pedidas e os eventos que elas geraram.
    pub fn escolher_no_menu_de_mod(&mut self, nome: &str, indice: usize) {
        self.vistas_para_estado();
        let Some(s) = self.scripts.as_mut() else { return };
        s.ao_menu(nome, indice, &mut self.estado);
        self.aplicar_pedidos_dos_mods();
        self.processar_eventos_do_estado();
    }

    /// Ticks ainda pedidos por `avancar_ticks` (API 0.17).
    pub fn ticks_de_mods_pendentes(&self) -> u32 {
        self.ticks_de_mods
    }

    /// Executa as ações pedidas pelos scripts (`core::acoes_mods`); recusas vão para o registro.
    fn aplicar_pedidos_dos_mods(&mut self) {
        let Some(s) = self.scripts.as_mut() else { return };
        let pedidos = s.retirar_pedidos();
        if pedidos.is_empty() {
            return;
        }
        for (nome, pedido) in pedidos {
            match pedido {
                PedidoMod::AvancarTicks(n) => self.ticks_de_mods = self.ticks_de_mods.saturating_add(n),
                PedidoMod::PararAvanco => self.ticks_de_mods = 0,
                ref outro => {
                    if let Err(motivo) = super::acoes_mods::aplicar(self, outro) {
                        self.registro_scripts.push(format!("[{nome}] {outro:?} recusado: {motivo}"));
                        // A recusa também aparece na faixa, como uma mensagem do script.
                        let agora = self.agora();
                        let d = self.avisos.cfg.faixa.duracao_conselho;
                        let texto = super::avisos::TextoFaixa::Literal(format!("{nome}: {motivo}"));
                        self.avisos.editar(Some(texto), Some(d), true, self.estado.partida.opcoes[4] != 0, agora);
                    }
                }
            }
        }
        self.vistas_do_estado();
        self.redesenhar = true;
    }

    /// Um lote dos ticks pedidos por `avancar_ticks`, sem esperar o relógio (com a pausa ou não);
    /// para no fim do jogo ou num aviso modal.
    fn rodar_ticks_de_mods(&mut self) -> Avanco {
        let mut av = Avanco::default();
        self.avanco_rapido = true;
        let lote = self.ticks_de_mods.min(self.cfg.relogio.max_mensagens_por_avanco.max(1) * 50);
        for _ in 0..lote {
            if self.ticks_de_mods == 0 {
                // `parar_avanco` num gancho do tick anterior.
                break;
            }
            if self.estado.partida.fim_de_jogo != 0 || self.avisos.tem_modal() {
                self.ticks_de_mods = 0;
                break;
            }
            self.rodar_tick();
            self.ticks_de_mods = self.ticks_de_mods.saturating_sub(1);
            av.ticks += 1;
        }
        self.avanco_rapido = false;
        self.redesenhar = true;
        av
    }

    /// Linhas de `registrar` dos scripts (e avisos de script desligado) desde a última retirada.
    pub fn retirar_registro_scripts(&mut self) -> Vec<String> {
        std::mem::take(&mut self.registro_scripts)
    }

    /// Mensagens dos scripts vão para a faixa (como `_EditMessage` importante); o registro fica
    /// guardado para a interface.
    fn saidas_dos_scripts(&mut self) {
        let Some(s) = self.scripts.as_mut() else { return };
        self.registro_scripts.extend(s.retirar_registro());
        let mensagens = s.retirar_mensagens();
        if mensagens.is_empty() {
            return;
        }
        use super::avisos::TextoFaixa;
        let agora = self.agora();
        let opcao = self.estado.partida.opcoes[4] != 0;
        for m in mensagens {
            let d = m.duracao.unwrap_or(self.avisos.cfg.faixa.duracao_conselho);
            self.avisos.editar(Some(TextoFaixa::Literal(m.texto)), Some(d), true, opcao, agora);
        }
        self.redesenhar = true;
    }

    /// Parte de vista de `_GotoMyAnt`: no Experimental só toca o som de erro; senão leva a vista
    /// até a amarela.
    fn ir_para_amarela(&mut self, plano: i16, x: i16, y: i16) {
        if self.estado.partida.tipo_jogo == 3 {
            if let Some(id) = self.tabelas_amarela.sons.erro {
                self.eventos.push_back(Evento::Som(id));
            }
            return;
        }
        self.redesenhar |= self.nav.ir_para_amarela(plano, i32::from(x), i32::from(y));
        self.vistas_para_estado();
    }

    /// Conselho de `_Feedback` na faixa (conselhos.md §2), depois de cada tick.
    fn conselho_na_faixa(&mut self) {
        use super::avisos::TextoFaixa;
        use crate::rules::conselhos::{conselho, Conselho};
        let t = &self.regras.quintal.conselhos;
        let agora = self.agora();
        let p = &self.estado.partida;
        let opcao = p.opcoes[4] != 0;
        match conselho(&self.estado, t) {
            Conselho::Nenhum => {}
            // _SetDefaultWindPrompt(0): o texto de pausa/ferramenta, ou limpar, sem ser importante.
            Conselho::PadraoDaJanela => self.avisos.mensagem_de_pausa(p.pausado != 0, p.ferramenta, false, opcao, agora),
            Conselho::Texto { lista, indice, importante } => {
                let d = self.avisos.cfg.faixa.duracao_conselho;
                self.avisos.editar(Some(TextoFaixa::Lista { lista, indice: indice as usize }), Some(d), importante, opcao, agora);
            }
        }
    }

    /// `_SetPause(pausado)` / Speed → Pause: ao despausar encerra as ferramentas 10 e 11; põe ou
    /// tira a mensagem de pausa da faixa (importante).
    pub fn definir_pausa(&mut self, pausado: bool) {
        let agora = self.agora();
        let p = &mut self.estado.partida;
        if !pausado && (p.ferramenta == 10 || p.ferramenta == 11) {
            p.ferramenta = -1;
        }
        p.pausado = i16::from(pausado);
        let (ferramenta, opcao) = (p.ferramenta, p.opcoes[4] != 0);
        self.avisos.mensagem_de_pausa(pausado, ferramenta, true, opcao, agora);
        self.redesenhar = true;
    }

    /// O que o File → Load faz depois de ler os blocos (`_LoadGame` `1010:9fa0` e
    /// `_DoLoadInitializations` `1010:9dbe`), além da reconstrução da vida e da vista (já feitas
    /// por `Estado::de_save` e `Navegacao::do_save`): `_FullCount` (`_CountAnts` + `_Feedback`),
    /// que recenseia as castas e já detecta a rainha que sumiu (quadros 10010/10011 e fim de jogo
    /// logo no Load, caso C-10), `_SetDefaultWindPrompt(1)` e `_PauseGame(1)`: o jogo carregado
    /// fica pausado com "Game Paused" (bancada-qemu.md, "Carregar um save injetado").
    pub fn ao_carregar(&mut self) {
        // _LoadGame: _TimeTemp = 300 (fora do save; os geradores já vêm de carregar::geradores_depois_do_load).
        self.regras.memoria_tutor.tempo_limite = super::carregar::TEMPO_TUTOR_DEPOIS_DO_LOAD;
        self.vistas_para_estado();
        self.estado.contar_formigas();
        self.tratar_eventos();
        self.conselho_na_faixa();
        // Os quadros do _FullCount são modais: a pausa só vem depois que eles fecham (bancada: com o
        // quadro na tela, a faixa ainda está vazia).
        if self.avisos.tem_modal() {
            self.pausar_ao_fechar = true;
        } else {
            self.definir_pausa(true);
        }
    }

    /// O que espera os avisos modais fecharem (a pausa do Load).
    fn depois_de_fechar(&mut self) {
        if self.pausar_ao_fechar && !self.avisos.tem_modal() {
            self.pausar_ao_fechar = false;
            self.definir_pausa(true);
        }
    }

    // --- avisos modais -------------------------------------------------------------------------

    /// Fecha o aviso modal da frente (clique na área do quadro, Espaço, clique no fim de jogo) e
    /// devolve qual era. Depois do fim de jogo, o original chama `_NewGame(0)`: a interface decide.
    pub fn fechar_modal(&mut self) -> Option<Modal> {
        self.redesenhar = true;
        let m = self.avisos.fechar_modal();
        if m.is_some() {
            self.modal_fechado();
        }
        // _GiveLesson lê o relógio para o prazo da lição só depois de a caixa fechar (conselhos.md §3)
        let relogio = self.mac_inicial.wrapping_add(((self.ms_total / 55) * 3) as u32);
        self.regras.memoria_tutor.caixa_fechada(i64::from(relogio));
        self.depois_de_fechar();
        m
    }

    // --- entradas do jogador -------------------------------------------------------------------

    /// Acelerador de velocidade, pausa ou AutoTrack (salvar fica com a interface).
    pub fn aplicar(&mut self, a: AcaoPartida) {
        let p = &mut self.estado.partida;
        match a {
            AcaoPartida::Pausa => {
                // _ProcMenu 0x41: alterna, com a mensagem de pausa na faixa.
                let novo = p.pausado ^ 1;
                self.definir_pausa(novo != 0);
            }
            AcaoPartida::VelocidadeLenta => p.velocidade = 0,
            AcaoPartida::VelocidadeNormal => p.velocidade = 1,
            AcaoPartida::VelocidadeRapida => p.velocidade = 2,
            AcaoPartida::VelocidadeUltra => p.velocidade = 3,
            AcaoPartida::AutoTrack => p.opcoes[0] = if p.opcoes[0] == 0 { 1 } else { 0 },
            AcaoPartida::Salvar
            | AcaoPartida::SalvarComo
            | AcaoPartida::Carregar
            | AcaoPartida::JanelaEdicao
            | AcaoPartida::JanelaComportamento
            | AcaoPartida::JanelaCastas
            | AcaoPartida::JanelaHistorico
            | AcaoPartida::JanelaInfo => {}
        }
        self.redesenhar = true;
    }

    /// Marcador `i` da faixa (`_DoBookMark`, `1000:08a6`). Com Ctrl, guarda o plano e o canto da
    /// vista e avisa "Bookmark has been placed."; sem Ctrl, vai até ele (`_GotoMapPoint` no centro
    /// da vista guardada) ou, se vazio, som de erro e "Bookmark has not been placed.".
    pub fn marcador(&mut self, i: usize, ctrl: bool) {
        let Some(slot) = self.marcadores.get(i).copied() else { return };
        let agora = self.agora();
        let f = self.avisos.cfg.faixa.clone();
        if ctrl {
            let v = &self.nav.vista;
            self.marcadores[i] = Some((self.nav.plano_mapa, v.x, v.y));
            self.avisos.mensagem_literal(&f.marcador_posto, f.duracao_botao, agora);
        } else if let Some((plano, x, y)) = slot {
            let (w, h) = (self.nav.vista.largura, self.nav.vista.altura);
            self.nav.ir_para_amarela(plano, w / 2 + x, h / 2 + y);
            self.vistas_para_estado();
        } else {
            self.eventos.push_back(Evento::Som(self.cfg.marcadores.som_vazio));
            self.avisos.mensagem_literal(&f.marcador_ausente, f.duracao_botao, agora);
        }
        self.redesenhar = true;
    }

    /// Bitmaps (selecionado, normal) dos marcadores postos, pelo nome do objeto da faixa.
    pub fn bitmaps_dos_marcadores(&self) -> Vec<(String, u16, u16)> {
        let c = &self.cfg.marcadores;
        self.marcadores
            .iter()
            .zip(&c.objetos)
            .enumerate()
            .filter(|(_, (m, _))| m.is_some())
            .map(|(i, (_, nome))| (nome.clone(), c.bitmap_posto[0] + i as u16, c.bitmap_posto[1] + i as u16))
            .collect()
    }

    /// Botão de cheiro da faixa (`ribBNestScent`… → `_SetMapModeAnt(n)`, n de 4 a 8): os feromônios
    /// só existem na superfície (antes, `_SetMapPlane(1)`); o mesmo botão de novo volta ao mapa
    /// simples. A janela de edição mostra a mesma camada (`_ELayerMode`, `_ProcMapRibbonEvent`
    /// `0x220b`–`0x220f`), ou nenhuma quando o botão desliga.
    pub fn botao_de_cheiro(&mut self, n: i16) {
        if self.nav.plano_mapa != 1 {
            self.navegar(Acao::Superficie);
        }
        self.vistas_para_estado();
        let e = &mut self.estado;
        e.vistas.modo_mapa = super::janela_mapa::modo_do_botao_de_cheiro(e.vistas.modo_mapa, n);
        e.camada_edicao = if e.vistas.modo_mapa == n { camada_do_modo_de_cheiro(n) } else { -1 };
        self.redesenhar = true;
    }

    /// Clique simples num botão configurável da faixa (`_DoUserButton`, sem segurar o botão).
    pub fn botao_usuario(&mut self, funcao: u16) -> RespostaBotao {
        let p = &mut self.estado.partida;
        let alternar = |v: &mut i16| *v = i16::from(*v == 0);
        match funcao {
            5 => {
                let novo = p.pausado ^ 1;
                self.definir_pausa(novo != 0);
            }
            6 => {
                p.velocidade = if p.velocidade == 3 { 0 } else { p.velocidade + 1 };
                let v = p.velocidade;
                let agora = self.agora();
                self.avisos.mensagem_de_velocidade(v, agora);
            }
            7 => alternar(&mut p.opcoes[1]),
            4 => {
                // _showTrails: rastros acumulados na janela do mapa (core/janela_mapa.rs).
                self.mostrar_trilhas = !self.mostrar_trilhas;
            }
            9 => alternar(&mut p.opcoes[2]),
            10 => alternar(&mut p.opcoes[5]),
            11 => alternar(&mut p.opcoes[0]),
            12 => alternar(&mut p.opcoes[3]),
            13 => alternar(&mut p.opcoes[4]),
            14 => return RespostaBotao::Salvar,
            15 => {
                let a = &self.estado.amarela;
                let (plano, x, y) = (a.plano, a.x, a.y);
                self.ir_para_amarela(plano, x, y);
            }
            8 => {
                // _MysteryButton: fora do tick, sorteio de R e efeito (rules/misterio.rs).
                self.vistas_para_estado();
                let efeito = crate::rules::misterio::botao_misterio(&mut self.estado, &mut self.regras, &self.tabelas_amarela);
                self.ultimo_misterio = Some(efeito);
                self.processar_eventos_do_estado();
            }
            outro => return RespostaBotao::NaoImplementado(outro),
        }
        self.redesenhar = true;
        RespostaBotao::Feito
    }

    /// Ação de navegação (rolagem, troca de plano).
    pub fn navegar(&mut self, a: Acao) -> bool {
        let mudou = self.nav.aplicar(a);
        self.vistas_para_estado();
        self.redesenhar |= mudou;
        mudou
    }

    /// `_SetMapPlane(0)`: o quintal passa a ser o plano mostrado (`_MapToYard` trocando o mapa pelo
    /// quintal; Ctrl+Y/5/6/7 com `_MapPlane` ≠ 0). A edição fica na superfície sem centralizar
    /// ([`Navegacao::ir_para_quintal`]). `_OldMapPlane` termina em 0: `_SetMapPlane` guarda o plano
    /// antigo e já grava `_MapPlane = 0` antes de `_SetMapPlaneLocation(0, …)`, que guarda de novo
    /// [código, `1000:a5ca` e `1000:a386`]. `_SetMapModeAnt(0)` vem de `vistas_para_estado`.
    pub fn ir_para_quintal(&mut self) -> bool {
        let (px, py) = self.estado.ambiente.patch;
        let mudou = self.nav.ir_para_quintal((i32::from(px), i32::from(py)));
        self.estado.vistas.plano_anterior = 0;
        self.vistas_para_estado();
        self.redesenhar |= mudou;
        mudou
    }

    /// Clique numa célula do mapa mostrado (`_processEdit`). `duplo` = duplo clique (esquerdo ou
    /// direito), sem "segurar o botão" (`segurando` = falso; os menus de segurar vêm de
    /// [`Partida::clicar_depois_da_espera`], janelas.md §4). O clique simples para a simulação por
    /// `espera_clique_simples_tickcount` (o laço de espera do original).
    pub fn clicar(&mut self, x: i32, y: i32, duplo: bool) -> RespostaClique {
        if !duplo {
            self.bloqueio_ms = self.cfg.espera_clique_simples_tickcount * self.avisos.cfg.ms_por_tickcount;
        }
        self.vistas_para_estado();
        let plano = i32::from(self.nav.plano_mapa);
        let r =
            clicar_no_mapa(&mut self.estado, &self.tabelas_amarela, &mut self.controle, plano, x, y, duplo, false);
        self.tratar_eventos();
        self.redesenhar = true;
        r
    }

    /// Começa a espera do clique simples (`_processEdit`): a simulação para por
    /// `espera_clique_simples_tickcount`; a decisão (ordem, menu da amarela, Examine) vem depois,
    /// por [`Partida::clicar_depois_da_espera`].
    pub fn iniciar_espera_do_clique(&mut self) {
        self.bloqueio_ms = self.cfg.espera_clique_simples_tickcount * self.avisos.cfg.ms_por_tickcount;
    }

    /// Fim da espera do clique simples: `segurando` = botão ainda apertado e Shift solto.
    pub fn clicar_depois_da_espera(&mut self, x: i32, y: i32, segurando: bool) -> RespostaClique {
        self.vistas_para_estado();
        let plano = i32::from(self.nav.plano_mapa);
        let r =
            clicar_no_mapa(&mut self.estado, &self.tabelas_amarela, &mut self.controle, plano, x, y, false, segurando);
        self.tratar_eventos();
        self.redesenhar = true;
        r
    }

    /// `_YellowCommand(c)` escolhido no menu da amarela (código 0 = Help, não reproduzido).
    pub fn comando_amarela(&mut self, c: u8) -> bool {
        if c == 0 {
            return false;
        }
        self.vistas_para_estado();
        let ok = comando(&mut self.estado, &self.tabelas_amarela, &mut self.controle, c);
        self.tratar_eventos();
        self.redesenhar = true;
        ok
    }

    /// (faixa, índice) do botão configurável com esse nome de objeto: faixa 0 = 34, 1 = 35.
    fn botao_configuravel(&self, objeto: &str) -> Option<(usize, usize)> {
        if let Some(i) = self.cfg.botoes_usuario.objetos.iter().position(|o| o == objeto) {
            return Some((0, i));
        }
        let q = self.cfg.botoes_usuario_quintal.as_ref()?;
        q.objetos.iter().position(|o| o == objeto).map(|i| (1, i))
    }

    fn funcoes_da_faixa(&mut self, faixa: usize) -> &mut Vec<u16> {
        if faixa == 0 {
            &mut self.funcoes_botoes
        } else {
            &mut self.funcoes_botoes_quintal
        }
    }

    /// Função do botão configurável com esse nome de objeto (faixa 34 ou 35).
    pub fn funcao_do_botao(&self, objeto: &str) -> Option<u16> {
        let (faixa, i) = self.botao_configuravel(objeto)?;
        if faixa == 0 { self.funcoes_botoes.get(i) } else { self.funcoes_botoes_quintal.get(i) }.copied()
    }

    /// Menu de ícones: troca a função do botão (`_SetUserButton`). Com `so_este` (Shift apertado), só
    /// este; senão também o botão de mesmo índice da outra faixa (`_DoUserButton`, `1000:04c4`).
    pub fn definir_funcao_do_botao(&mut self, objeto: &str, f: u16, so_este: bool) {
        let Some((faixa, i)) = self.botao_configuravel(objeto) else { return };
        let faixas: &[usize] = if so_este { &[faixa] } else { &[0, 1] };
        for &k in faixas {
            if let Some(v) = self.funcoes_da_faixa(k).get_mut(i) {
                *v = f;
            }
        }
        self.redesenhar = true;
    }

    /// Estado desenhado dos botões configuráveis (`_SetUserButton`/`_UpdateUserButtons`): nome,
    /// bitmaps (selecionado, normal) da função e, nos botões de alternar, se estão ligados.
    pub fn estado_dos_botoes(&self) -> Vec<(String, u16, u16, Option<bool>)> {
        let mut v = self.estado_dos_botoes_de(&self.cfg.botoes_usuario, &self.funcoes_botoes);
        if let Some(q) = &self.cfg.botoes_usuario_quintal {
            v.extend(self.estado_dos_botoes_de(q, &self.funcoes_botoes_quintal));
        }
        v
    }

    fn estado_dos_botoes_de(&self, b: &BotoesUsuario, funcoes: &[u16]) -> Vec<(String, u16, u16, Option<bool>)> {
        let p = &self.estado.partida;
        b.objetos
            .iter()
            .zip(funcoes)
            .map(|(nome, &f)| {
                let ligado = match f {
                    4 => Some(self.mostrar_trilhas),
                    5 => Some(p.pausado != 0),
                    7 => Some(p.opcoes[1] != 0),
                    9 => Some(p.opcoes[2] != 0),
                    10 => Some(p.opcoes[5] != 0),
                    11 => Some(p.opcoes[0] != 0),
                    12 => Some(p.opcoes[3] != 0),
                    13 => Some(p.opcoes[4] != 0),
                    _ => None,
                };
                (nome.clone(), b.bitmap_selecionado + f, b.bitmap_normal + f, ligado)
            })
            .collect()
    }

    /// Tecla que `_YellowCommandKey` não tratou, para o anel dos códigos de trapaça do original
    /// (`_DoKeyDown`, trapacas.md §1): Enter limpa o anel; letras entram (minúsculas sem Shift,
    /// maiúsculas com Shift) e, se as 4 últimas formam um código, `_CheatKeys` é aplicado. Devolve o
    /// efeito aplicado.
    pub fn tecla_de_trapaca(&mut self, nome: &str, shift: bool) -> Option<EfeitoTrapaca> {
        if nome == "Return" || nome == "Keypad Enter" {
            self.anel_trapaca.limpar();
            return None;
        }
        let letra = letra_da_tecla(nome, shift)?;
        let efeito = self.anel_trapaca.digitar(letra, &self.tabelas_trapaca)?;
        self.vistas_para_estado();
        trapacas::aplicar(&mut self.estado, &self.tabelas_trapaca, &self.tabelas_amarela, efeito);
        self.processar_eventos_do_estado();
        Some(efeito)
    }

    /// Tecla de comando da amarela (`_YellowCommandKey`), com o código de tecla virtual do Windows.
    pub fn tecla_amarela(&mut self, vk: u16, m: Modificadores) -> RespostaTecla {
        self.vistas_para_estado();
        let r = tecla_de_comando(&mut self.estado, &self.tabelas_amarela, &mut self.controle, vk, m);
        self.tratar_eventos();
        if r != RespostaTecla::Ignorada {
            self.redesenhar = true;
        }
        r
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::core::edicao::VistaEdicao;
    use crate::core::mapa::Plano;
    use crate::rules::tick::{RegrasAmarela, RegrasConselhos, RegrasInimigos, RegrasNinho, RegrasQuintal, RegrasSuperficie};

    fn cfg() -> ConfigPartida {
        ConfigPartida::de_json(include_str!("../../../data/partida.json")).unwrap()
    }

    fn cfg_avisos() -> ConfigAvisos {
        ConfigAvisos::de_json(include_str!("../../../data/mensagens.json")).unwrap()
    }

    fn cfg_sim() -> ConfigSimulacao {
        ConfigSimulacao::de_json(include_str!("../../../data/simulacao.json")).unwrap()
    }

    fn nav() -> Navegacao {
        Navegacao {
            plano_mapa: 2,
            vista: VistaEdicao { plano: Plano::NinhoPreto, x: 0, y: 0, largura: 25, altura: 19 },
            pontos: [(0, 0); 4],
        }
    }

    /// Partida sobre um estado vazio (regras originais sobre mapas vazios: nada se mexe, mas o tick
    /// conta o tempo).
    fn partida(velocidade: i16) -> Partida {
        let mut e = Estado::vazio();
        e.partida.velocidade = velocidade;
        e.partida.ferramenta = -1;
        e.amarela.plano = 2;
        Partida::nova(e, nav(), RegrasOriginais::default(), TabelasAmarela::padrao().clone(), cfg_sim(), cfg(), cfg_avisos())
    }

    #[test]
    fn sorteios_do_desenho_depois_do_tick_seguem_as_janelas() {
        // Chuva com o quintal (Yard) aberto na frente: depois do tick, 14 gotas = 28 sorteios de S
        // (rules::desenho); com o quintal fechado, nada. A diferença entre as duas partidas é
        // exatamente o desenho do quintal sobre o estado de depois do tick.
        let nova = |aberto: bool| {
            let mut p = partida(1);
            p.estado.ambiente.chovendo = 1;
            p.estado.ambiente.chuva_contagem = 100;
            p.estado.s = crate::rules::rng::GeradorS::new(0x1234);
            p.estado.vistas.modo_mapa = 1;
            p.janelas_desenho.quintal_aberto = aberto;
            p.janelas_desenho.quintal_na_frente = aberto;
            p
        };
        let (mut fechado, mut aberto) = (nova(false), nova(true));
        fechado.rodar_tick();
        aberto.rodar_tick();
        let mut esperado = fechado.estado.clone();
        let passos = desenho::desenhar_quintal(&mut esperado, 0);
        assert_eq!(passos, 28);
        assert_eq!(aberto.estado.s, esperado.s);
        assert_ne!(aberto.estado.s, fechado.estado.s);
        // Inativa: só 1 tick em 8 chega a _UpdateWindows (`_UDcntr` conta só esses); os contadores
        // ficam na Partida, fora do save.
        let mut inativa = nova(true);
        inativa.definir_foco(false);
        for _ in 0..16 {
            inativa.rodar_tick();
        }
        assert_eq!(inativa.contadores_redesenho.ticks, 16);
        assert_eq!(inativa.contadores_redesenho.chamadas, 2);
    }

    #[test]
    fn minimizada_sorteia_o_icone_a_cada_mensagem() {
        // Minimizada: SRand1(7) antes do laço do temporizador (e nenhum desenho depois do tick).
        let semente = crate::rules::rng::GeradorS::new(0x1234);
        let mut a = partida(1);
        a.estado.s = semente;
        a.janelas_desenho.minimizada = true;
        let mut b = partida(1);
        b.estado.s = semente;
        b.estado.s.rand1(7);
        a.mensagem_de_timer();
        b.mensagem_de_timer();
        assert_eq!(a.estado.s, b.estado.s);
        assert_ne!(a.estado.s, semente);
        // Pausada: o ícone não anima.
        let mut c = partida(1);
        c.estado.partida.pausado = 1;
        c.janelas_desenho.minimizada = true;
        c.estado.s = semente;
        let s = c.estado.s;
        c.mensagem_de_timer();
        assert_eq!(c.estado.s, s);
    }

    #[test]
    fn dados_de_partida_carregam() {
        let c = cfg();
        assert_eq!(c.relogio.intervalo_mensagem_ms, 17);
        assert_eq!(c.relogio.intervalo_sem_foco_ms, 170);
        let t = |nome: &str, shift, ctrl| EventoTecla::Tecla { nome: nome.into(), ctrl, shift, alt: false };
        assert_eq!(c.acao_da_tecla(&t("0", true, false)), Some(AcaoPartida::Pausa));
        assert_eq!(c.acao_da_tecla(&t("3", true, false)), Some(AcaoPartida::VelocidadeRapida));
        assert_eq!(c.acao_da_tecla(&t("Keypad 4", true, false)), Some(AcaoPartida::VelocidadeUltra));
        assert_eq!(c.acao_da_tecla(&t("g", false, true)), Some(AcaoPartida::AutoTrack));
        assert_eq!(c.acao_da_tecla(&t("3", false, false)), None, "3 sem Shift é da amarela");
        assert_eq!(c.tecla_virtual("3"), Some(0x33));
        assert_eq!(c.tecla_virtual("keypad *"), Some(0x6A));
        assert_eq!(c.tecla_virtual("Backspace"), Some(0x08));
        assert_eq!(c.botao_da_tecla("Delete"), None, "Delete é a tecla do modo de ajuda");
        assert_eq!(c.botao_da_tecla("Insert"), Some(BotaoTeclado::Esquerdo));
    }

    #[test]
    fn timer_de_17_ms_vira_55_ms_com_resto() {
        // SetTimer(17 ms) no Windows 3.1: uma WM_TIMER a cada 55 ms (resolução do relógio).
        let mut p = partida(1);
        assert_eq!(p.intervalo_ms(), 55);
        assert_eq!(p.avancar(54).mensagens, 0);
        assert_eq!(p.avancar(1).mensagens, 1);
        assert_eq!(p.avancar(109).mensagens, 1);
        assert_eq!(p.avancar(1).mensagens, 1, "o resto de 54 ms foi guardado");
    }

    #[test]
    fn mensagens_nao_se_acumulam_alem_do_maximo() {
        let mut p = partida(1);
        let av = p.avancar(10_000);
        assert_eq!(av.mensagens, 3);
        assert_eq!(p.avancar(0).mensagens, 0, "o tempo excedente foi descartado");
    }

    /// Ticks em `n` mensagens seguidas.
    #[test]
    fn animacao_da_amarela_para_a_simulacao_e_mostra_os_quadros() {
        // _AnimYellowFight: 6 quadros a cada 165 ms; o programa inteiro espera 825 ms, e a janela de
        // edição mostra o quadro da vez. Depois, os ticks voltam.
        let mut p = partida(1);
        let quadro = |d: i16| crate::rules::estado::QuadroAnimacao {
            mapa: vec![0; 4096],
            vida: vec![0; 4096],
            x: 5,
            y: 6,
            tipo: 0x70,
            dir: d,
            ovo: 0,
            base: 0,
        };
        let a = AnimacaoDaAmarela { codigo: 0, plano: 2, espera_ms: 165, quadros: (0..6).map(quadro).collect() };
        p.estado.eventos.push(Evento::Animacao(Box::new(a)));
        p.processar_eventos_do_estado();
        assert!(p.retirar_eventos().is_empty(), "a animação fica na partida");
        assert!(p.animando());
        let t0 = p.estado.partida.tempo;
        assert_eq!(p.quadro_animado().map(|(pl, q)| (pl, q.dir)), Some((2, 0)));
        assert_eq!(p.avancar(400).ticks, 0);
        assert_eq!(p.quadro_animado().map(|(_, q)| q.dir), Some(2), "400 ms: terceiro quadro");
        assert!(p.tomar_redesenho());
        assert_eq!(p.avancar(400).ticks, 0);
        assert_eq!(p.quadro_animado().map(|(_, q)| q.dir), Some(4));
        assert_eq!(p.avancar(25).ticks, 0, "825 ms: acabou, sem tick neste avanço");
        assert!(!p.animando());
        assert_eq!(p.estado.partida.tempo, t0);
        assert!(p.avancar(2000).ticks > 0, "a simulação volta");
    }

    #[test]
    fn quadro_da_aranha_vem_antes_da_luta_e_a_morte_depois() {
        // _YellowDeath(1) com Events: _SpiderDialog (animação e texto), _AnimYellowFight e
        // _YellowDialog(9100), nessa ordem; a luta espera os dois quadros da aranha fecharem.
        let mut p = partida(1);
        p.estado.partida.opcoes[3] = 1;
        let q = crate::rules::estado::QuadroAnimacao { mapa: vec![0; 4096], vida: vec![0; 4096], x: 5, y: 6, tipo: 0x70, dir: 0, ovo: 0, base: 0 };
        let a = AnimacaoDaAmarela { codigo: 1, plano: 1, espera_ms: 165, quadros: vec![q.clone(), q] };
        p.estado.eventos.push(Evento::DialogoAranha { texto: 10047 });
        p.estado.eventos.push(Evento::Animacao(Box::new(a)));
        p.estado.eventos.push(Evento::QuadroAmarela { imagem: 9100, indice: 0 });
        p.processar_eventos_do_estado();
        assert_eq!(p.avisos.modal(), Some(&Modal::Animado(TipoAnimado::Aranha)));
        assert!(!p.animando(), "a luta espera");
        p.avancar(55);
        assert!(p.retirar_eventos().contains(&Evento::Som(45)), "som do início da aranha");
        assert_eq!(p.pular_modal(), Some(Modal::Animado(TipoAnimado::Aranha)));
        assert!(p.avisos.residual().is_some(), "a janela da aranha fica aberta");
        assert_eq!(p.avisos.modal(), Some(&Modal::Quadro { texto: 10047, imagem: None }));
        assert!(!p.animando());
        assert_eq!(p.fechar_modal(), Some(Modal::Quadro { texto: 10047, imagem: None }));
        assert!(p.animando(), "fechado o texto, a luta começa");
        assert_eq!(p.avisos.modal(), Some(&Modal::Amarela { imagem: 9100, indice: 0 }));
        p.avancar(100);
        assert!(p.animando(), "2 quadros: 165 ms");
        p.avancar(100);
        assert!(!p.animando());
        p.avancar(55);
        assert!(p.avisos.residual().is_none(), "o _YellowDialog(9100) reaproveita a janela");
        assert_eq!(p.fechar_modal(), Some(Modal::Amarela { imagem: 9100, indice: 0 }));
    }

    #[test]
    fn ticks_dos_mods_nao_param_nas_animacoes() {
        let mut p = partida(1);
        p.avanco_rapido = true;
        let q = crate::rules::estado::QuadroAnimacao { mapa: Vec::new(), vida: Vec::new(), x: 0, y: 0, tipo: 0x70, dir: 0, ovo: 0, base: 0 };
        let a = AnimacaoDaAmarela { codigo: 0, plano: 1, espera_ms: 165, quadros: vec![q.clone(), q] };
        p.estado.eventos.push(Evento::Animacao(Box::new(a)));
        p.processar_eventos_do_estado();
        assert!(!p.animando());
    }

    fn ticks_em(p: &mut Partida, n: u32) -> u32 {
        (0..n).map(|_| p.mensagem_de_timer()).sum()
    }

    #[test]
    fn velocidades_seguem_o_laco_de_mytimerfunc() {
        // Cada chamada de MYTIMERFUNC consome 3 mensagens (a dela e 2 drenadas); o atraso é testado
        // na entrada e depois da primeira drenada. Bancada: slow 3,0 ticks/s, normal 6,0, fast
        // 55–63, ultra 91 (18 mensagens = 6 chamadas = 0,99 s; o estado vazio abre um quadro no
        // tick 128, por isso a conta para antes).
        assert_eq!(ticks_em(&mut partida(0), 18), 2, "slow: 1 tick a cada 6 mensagens (330 ms)");
        assert_eq!(ticks_em(&mut partida(1), 18), 6, "normal: 1 tick a cada 3 mensagens (165 ms)");
        assert_eq!(ticks_em(&mut partida(2), 18), 66, "fast: 2 × 5 + 1 por chamada");
        assert_eq!(ticks_em(&mut partida(3), 18), 90, "ultra: 2 × 7 + 1 por chamada");
        let mut p = partida(0);
        assert_eq!(ticks_em(&mut p, 6), 0);
        assert_eq!(ticks_em(&mut p, 1), 1, "a sétima mensagem abre a terceira chamada com atraso 7");
        assert_eq!(p.estado.partida.tempo, 1);
        // _gameCycles conta as voltas do laço: em normal passa de 20 entre dois ticks (o pássaro
        // é testado em todo tick, testes-tick.md); em fast/ultra anda 1 por tick.
        let mut p = partida(1);
        ticks_em(&mut p, 3);
        let antes = p.estado.ciclos_jogo;
        assert_eq!(ticks_em(&mut p, 3), 1);
        assert!(p.estado.ciclos_jogo - antes > 20);
        let mut p = partida(3);
        let t = ticks_em(&mut p, 30);
        assert_eq!(p.estado.ciclos_jogo, t);
    }

    #[test]
    fn pausa_para_e_amarela_em_movimento_continua() {
        let mut p = partida(1);
        p.aplicar(AcaoPartida::Pausa);
        assert_eq!(p.estado.partida.pausado, 1);
        assert_eq!(ticks_em(&mut p, 20), 0);
        // §4: com ordem em andamento e ferramenta < 10, o tick roda mesmo pausado.
        p.estado.amarela.mover = 1;
        assert!(ticks_em(&mut p, 2) >= 1);
        p.estado.amarela.mover = 0;
        p.estado.partida.ferramenta = 10;
        assert_eq!(ticks_em(&mut p, 20), 0);
        // Despausar encerra a ferramenta 10.
        p.aplicar(AcaoPartida::Pausa);
        assert_eq!((p.estado.partida.pausado, p.estado.partida.ferramenta), (0, -1));
        // O atraso continuou contando na pausa: o primeiro tick sai já na mensagem seguinte.
        assert_eq!(ticks_em(&mut p, 1), 1);
    }

    #[test]
    fn alarme_e_botoes_de_cheiro_mostram_a_camada_na_edicao() {
        // Tecla 0 na superfície (_SetAlarmDropState(1)): _ELayerMode = 0; de novo, −1.
        let mut p = partida(1);
        p.estado.amarela.plano = 1;
        p.estado.amarela.modo = 0;
        let sem = Modificadores::default();
        p.tecla_amarela(48, sem);
        assert_eq!((p.estado.amarela.soltando_alarme, p.estado.camada_edicao), (1, 0));
        p.tecla_amarela(48, sem);
        assert_eq!((p.estado.amarela.soltando_alarme, p.estado.camada_edicao), (0, -1));
        // No ninho não liga (som de erro) e a camada não muda.
        p.estado.amarela.plano = 2;
        p.tecla_amarela(48, sem);
        assert_eq!((p.estado.amarela.soltando_alarme, p.estado.camada_edicao), (0, -1));
        // Botões de cheiro: o mesmo botão de novo desliga.
        p.botao_de_cheiro(4);
        assert_eq!(p.estado.camada_edicao, 1);
        p.botao_de_cheiro(8);
        assert_eq!(p.estado.camada_edicao, 0);
        p.botao_de_cheiro(8);
        assert_eq!(p.estado.camada_edicao, -1);
        assert_eq!((5..=7).map(camada_do_modo_de_cheiro).collect::<Vec<_>>(), [2, 3, 4]);
    }

    #[test]
    fn velocidade_e_auto_track_pelos_aceleradores() {
        let mut p = partida(1);
        p.aplicar(AcaoPartida::VelocidadeUltra);
        assert_eq!(p.estado.partida.velocidade, 3);
        p.aplicar(AcaoPartida::VelocidadeLenta);
        assert_eq!(p.estado.partida.velocidade, 0);
        assert_eq!(p.estado.partida.opcoes[0], 0);
        p.aplicar(AcaoPartida::AutoTrack);
        assert_eq!(p.estado.partida.opcoes[0], 1);
        p.aplicar(AcaoPartida::AutoTrack);
        assert_eq!(p.estado.partida.opcoes[0], 0);
    }

    #[test]
    fn sem_foco_o_temporizador_de_170_ms_vira_220_ms() {
        let mut p = partida(1);
        p.definir_foco(false);
        assert_eq!(p.intervalo_ms(), 220);
        assert_eq!(p.avancar(219).mensagens, 0);
        assert_eq!(p.avancar(1).mensagens, 1);
    }

    #[test]
    fn redesenho_do_mapa_em_fast_e_ultra() {
        let mut p = partida(3);
        p.tomar_redesenho();
        let desenhos: Vec<bool> = (0..8)
            .map(|_| {
                p.rodar_tick();
                p.tomar_redesenho()
            })
            .collect();
        assert_eq!(desenhos, [false, false, false, true, false, false, false, true]);
        let mut p = partida(2);
        p.tomar_redesenho();
        let desenhos: Vec<bool> = (0..4)
            .map(|_| {
                p.rodar_tick();
                p.tomar_redesenho()
            })
            .collect();
        assert_eq!(desenhos, [false, true, false, true]);
    }

    /// Regras que só pedem para centralizar (como o AutoTrack de `_DoAntMoveY`) e fim de jogo.
    struct PedeCentralizar;
    impl RegrasQuintal for PedeCentralizar {}
    impl RegrasInimigos for PedeCentralizar {}
    impl RegrasSuperficie for PedeCentralizar {}
    impl RegrasNinho for PedeCentralizar {}
    impl RegrasConselhos for PedeCentralizar {}
    impl RegrasAmarela for PedeCentralizar {
        fn mover_amarela(&mut self, e: &mut Estado) {
            e.amarela.x += 1;
            e.pedir_centralizar_na_amarela();
            e.eventos.push(Evento::Musica(7));
        }
    }

    #[test]
    fn pedido_de_centralizar_move_a_vista_e_fim_de_jogo_nao_repete() {
        let mut p = partida(1);
        p.estado.amarela.plano = 2;
        p.estado.amarela.x = 40;
        p.estado.amarela.y = 30;
        p.estado.partida.fim_de_jogo = 1;
        let mut r = PedeCentralizar;
        for _ in 0..3 {
            tick(&mut p.estado, &mut r);
            p.tratar_eventos();
        }
        // _CenterEdit em (43, 30) com 25 × 19 (_editWidth da bancada): canto (31, 21).
        assert_eq!((p.nav.vista.x, p.nav.vista.y), (31, 21));
        assert_eq!(p.estado.vistas.ponto, (31, 21));
        assert_eq!(
            p.retirar_eventos(),
            vec![Evento::Musica(7), Evento::DialogoFimDeJogo, Evento::Musica(7), Evento::Musica(7)]
        );
    }

    #[test]
    fn centralizar_em_outro_plano_troca_o_plano() {
        let mut p = partida(1);
        p.estado.amarela.plano = 1;
        p.estado.amarela.x = 100;
        p.estado.amarela.y = 10;
        p.estado.pedir_centralizar_na_amarela();
        p.tratar_eventos();
        assert_eq!(p.nav.plano_mapa, 1);
        assert_eq!(p.nav.vista.plano, Plano::Superficie);
        // dx = 88, dy = 1 (meia largura 12): a fração 65536/88 truncada nunca liga o bit 16; y não
        // anda (jogo.md).
        assert_eq!((p.nav.vista.x, p.nav.vista.y), (88, 0));
        assert_eq!(p.estado.vistas.plano, 1);
    }

    fn scripts(codigo: &str) -> Scripts {
        Scripts::de_fontes(
            vec![("teste".into(), Ok(vec![("a.lua".into(), codigo.into())]))],
            crate::rules::experimental::TabelasExperimental::padrao().clone(),
            crate::scripts::Limites::default(),
        )
    }

    #[test]
    fn script_sem_ganchos_de_tick_nao_muda_o_tick() {
        let mut sem = partida(1);
        let mut com = partida(1);
        com.ligar_scripts(scripts("function ao_evento(ev, estado) registrar(ev.tipo) end"));
        for _ in 0..300 {
            sem.rodar_tick();
            com.rodar_tick();
        }
        assert_eq!(sem.estado, com.estado);
    }

    #[test]
    fn mensagem_do_script_vai_para_a_faixa_e_registro_fica_guardado() {
        use crate::core::avisos::TextoFaixa;
        let mut p = partida(1);
        p.ligar_scripts(scripts(
            "function depois_do_tick(e) if e:tempo() == 3 then mostrar_mensagem('olá da faixa'); registrar('t', e:tempo()) end end",
        ));
        for _ in 0..3 {
            p.rodar_tick();
        }
        let agora = p.agora();
        assert_eq!(p.avisos.texto_da_faixa(agora), Some(&TextoFaixa::Literal("olá da faixa".into())));
        assert_eq!(p.retirar_registro_scripts(), vec!["[teste] t\t3".to_string()]);
    }
}
