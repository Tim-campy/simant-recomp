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
use std::path::Path;

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
use crate::rules::troca_patch::{ResultadoTroca, TransferenciaNoTick};
use crate::rules::trapacas::{self, letra_da_tecla, AnelTrapaca, EfeitoTrapaca, TabelasTrapaca};
use crate::scripts::{Gancho, PedidoMod, RegrasComScripts, Scripts};

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_CONFIG: &str = "partida.json";

/// Como Fast e Ultra viram ritmo (data/partida.json, `relogio.ritmo_velocidades`).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModoRitmo {
    /// O laço ocupado de `MYTIMERFUNC` com `voltas_por_periodo` (o 486 da bancada: ~66,7 e ~90,9
    /// ticks/s). É o original.
    #[default]
    Original,
    /// Extensão do port (escolha do usuário, 2026-10-19): as velocidades com multiplicador andam a
    /// `multiplicador × ticks_por_segundo_normal`, como num processador mais lento; o tick é o mesmo.
    Proporcional,
}

/// Ritmo das velocidades (extensão do port; docs/decisoes.md, 2026-10-19, "Ritmo das velocidades").
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct RitmoVelocidades {
    #[serde(default)]
    pub modo: ModoRitmo,
    /// Ticks por segundo da Normal no original (1 a cada 3 mensagens de 55 ms: 6,06).
    #[serde(default)]
    pub ticks_por_segundo_normal: f64,
    /// Por `_GameSpeed` (slow, normal, fast, ultra): múltiplo de `ticks_por_segundo_normal`, ou
    /// `null` = a regra do original para essa velocidade.
    #[serde(default)]
    pub multiplicadores: Vec<Option<f64>>,
}

/// Ritmo das mensagens de temporizador e do redesenho (data/partida.json, "relogio").
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ConfigRelogio {
    /// Intervalo pedido ao `SetTimer` com o aplicativo em foco (17 ms).
    pub intervalo_mensagem_ms: u32,
    /// Intervalo sem foco (`WM_ACTIVATEAPP` com wParam = 0: o código pede 170 ms; o dado é 17, o ritmo
    /// medido sem foco).
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
    /// Ritmo de Fast/Ultra (ausente = o original).
    #[serde(default)]
    pub ritmo_velocidades: RitmoVelocidades,
    /// Extensão do port (escolha do usuário, 2026-10-19): sem o foco do sistema, a cadência é a
    /// desta velocidade (0 = Slow), ou a escolhida se for mais lenta; volta sozinha com o foco. Só
    /// a cadência: `_GameSpeed` (o save, a marca do menu Speed, o desenho) não muda. `null`/ausente
    /// = o original (o ritmo não muda sem foco).
    #[serde(default)]
    pub sem_foco_velocidade: Option<i16>,
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
#[derive(Clone, Debug, Deserialize, PartialEq)]
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

/// Valor de uma tabela por `_GameSpeed` (0 slow … 3 ultra; fora disso, o último).
fn na_velocidade<T: Copy>(tabela: &[T], v: i16, padrao: T) -> T {
    usize::try_from(v).ok().and_then(|i| tabela.get(i).copied()).or(tabela.last().copied()).unwrap_or(padrao)
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

/// O começo de um tick que parou na escolha de `_SpecialXfer`.
struct FotoColonia {
    estado: Estado,
    memoria_tutor: crate::rules::conselhos::MemoriaTutor,
    /// Pedidos do tick até a escolha (já entregues à interface na primeira passada).
    eventos: usize,
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
    /// Bit 0 de `GetAsyncKeyState(VK_LBUTTON)`: o botão esquerdo foi apertado desde a última
    /// consulta (simulacao-visao-geral.md §2).
    clique_esquerdo_pendente: bool,
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
    /// A janela do mapa aberta e na frente (a interface informa, como `janelas_desenho`).
    pub mapa_aberto: bool,
    pub mapa_na_frente: bool,
    /// `DGROUP:0x72f0`: alternância `_DrawMap`/`_DrawMapData` com o mapa atrás; os `_Goto*` no mesmo
    /// plano a zeram.
    alterna_mapa: bool,
    /// Algum tick desde o último quadro mostrou o mapa (`_DrawMap`).
    mapa_na_tela: bool,
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
    /// O tick que pediu a escolha de `_SpecialXfer`, guardado do começo: com a colônia escolhida, ele
    /// roda de novo com a escolha e a troca acontece DENTRO dele (`concluir_transferencia_especial`).
    foto_colonia: Option<Box<FotoColonia>>,
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
    /// A animação da amarela em andamento (`_AnimYellowFight`, `_AnimYellowInsane`, os estágios do
    /// ovo de `_YellowBirth`) e o `ms_total` em que começou. Enquanto dura, a simulação fica parada
    /// (`bloqueio_ms`) e a janela de edição mostra o quadro da vez (`quadro_animado`).
    animacoes: Vec<AnimacaoDaAmarela>,
    animacao_inicio_ms: u64,
    /// Linha do tempo da apresentação: os pedidos do tick ainda não apresentados. O original faz
    /// tudo dentro do tick, em sequência e parado em cada quadro modal e em cada animação; o port
    /// roda o tick inteiro e apresenta os pedidos na mesma ordem, um depois do outro: uma animação
    /// ou um aviso modal seguram o resto (centralizações, outros quadros, músicas) até acabarem, e
    /// nenhum tick roda enquanto sobra algo aqui (regras/formiga-amarela.md §9.4).
    apresentacao: VecDeque<Evento>,
    /// O último instantâneo da janela de edição apresentado (`Evento::Instantaneo`, ou o último
    /// quadro de uma animação): enquanto a apresentação não acaba, a janela de edição mostra isto
    /// no lugar do estado (que já é o do fim do tick).
    quadro_congelado: Option<(i16, QuadroAnimacao)>,
    /// Quadro mostrado no último redesenho (animação, quadro), para só redesenhar quando muda.
    quadro_mostrado: Option<(usize, usize)>,
    /// Rodando os ticks pedidos pelos mods de uma vez: as animações não param nada.
    avanco_rapido: bool,
    /// Ritmo proporcional (`ritmo_velocidades`): voltas do laço de `MYTIMERFUNC` ainda por dar,
    /// acumuladas a cada mensagem (a fração de volta fica guardada).
    credito_voltas: f64,
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
            clique_esquerdo_pendente: false,
            resto_ms: 0,
            ms_total: 0,
            mac_inicial,
            ativo: true,
            quadros: 0,
            contador_mapa: 0,
            mapa_aberto: false,
            mapa_na_frente: false,
            alterna_mapa: false,
            mapa_na_tela: false,
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
            foto_colonia: None,
            parada_pela_interface: false,
            scripts: None,
            registro_scripts: Vec::new(),
            tabelas_trapaca: TabelasTrapaca::padrao().clone(),
            anel_trapaca: AnelTrapaca::default(),
            ticks_de_mods: 0,
            animacoes: Vec::new(),
            animacao_inicio_ms: 0,
            apresentacao: VecDeque::new(),
            quadro_congelado: None,
            quadro_mostrado: None,
            avanco_rapido: false,
            credito_voltas: 0.0,
        };
        p.funcoes_botoes = p.cfg.botoes_usuario.funcoes.clone();
        p.funcoes_botoes_quintal = p.cfg.botoes_usuario_quintal.as_ref().map(|b| b.funcoes.clone()).unwrap_or_default();
        p.vistas_para_estado();
        p
    }

    // --- relógio -------------------------------------------------------------------------------

    /// `_GameSpeed` como índice das tabelas (0 slow … 3 ultra; fora disso, o último).
    fn indice_velocidade<T: Copy>(&self, tabela: &[T], padrao: T) -> T {
        na_velocidade(tabela, self.estado.partida.velocidade, padrao)
    }

    /// Velocidade da cadência: `_GameSpeed`, ou, sem foco e com `sem_foco_velocidade`, a mais lenta
    /// das duas (extensão do port). Só decide o ritmo dos ticks (voltas e atraso), nunca o estado.
    pub fn velocidade_efetiva(&self) -> i16 {
        let v = self.estado.partida.velocidade;
        match self.cfg.relogio.sem_foco_velocidade {
            Some(lenta) if !self.ativo => v.min(lenta),
            _ => v,
        }
    }

    /// Ritmo proporcional: voltas (= ticks em Fast/Ultra) por mensagem de temporizador na velocidade
    /// `v`, ou `None` se ela segue o laço do original.
    fn voltas_por_mensagem(&self, v: i16) -> Option<f64> {
        let ritmo = &self.cfg.relogio.ritmo_velocidades;
        if ritmo.modo != ModoRitmo::Proporcional {
            return None;
        }
        let m = usize::try_from(v).ok().and_then(|i| ritmo.multiplicadores.get(i).copied().flatten())?;
        Some(m * ritmo.ticks_por_segundo_normal * self.intervalo_ms() as f64 / 1000.0)
    }

    /// Ticks por segundo da velocidade `v` no ritmo proporcional (`None` = a do laço do original).
    pub fn ticks_por_segundo(&self, v: i16) -> Option<f64> {
        self.voltas_por_mensagem(v).map(|n| n * 1000.0 / self.intervalo_ms() as f64)
    }

    /// Intervalo atual entre mensagens de temporizador (ms): o pedido ao `SetTimer` (17 ms, ou
    /// 170 ms sem foco) arredondado para cima até a resolução do temporizador do Windows 3.1 (55 ms).
    pub fn intervalo_ms(&self) -> u64 {
        let r = &self.cfg.relogio;
        let pedido = u64::from(if self.ativo { r.intervalo_mensagem_ms } else { r.intervalo_sem_foco_ms }).max(1);
        let resolucao = u64::from(r.resolucao_timer_ms).max(1);
        pedido.div_ceil(resolucao) * resolucao
    }

    /// Foco do aplicativo (`WM_ACTIVATEAPP`): sem foco, o código pede 170 ms, mas o ritmo medido não
    /// muda (`data/partida.json`, `intervalo_sem_foco_ms` = 17); só o redesenho fica mais raro. Com
    /// `sem_foco_velocidade` (extensão do port), a cadência cai para a lenta até o foco voltar.
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
            // _AnimYellowFight: o programa inteiro espera; o que o tick pediu depois da animação (a
            // morte depois da luta perdida, o renascimento) só é apresentado quando ela acaba.
            let espera = ms.min(self.bloqueio_ms);
            self.bloqueio_ms -= espera;
            self.acompanhar_animacao();
            if !self.animando() {
                self.seguir_apresentacao();
            }
            self.resto_ms = 0;
            return Avanco::default();
        }
        if !self.apresentacao.is_empty() {
            self.seguir_apresentacao();
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
            // Fechou o quadro que vinha antes de uma animação: ela começa agora.
            return Avanco::default();
        }
        if self.avisos.tem_modal() || self.escolher_colonia || self.parada_pela_interface || !self.apresentacao.is_empty() {
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
            if self.avisos.tem_modal() || !self.apresentacao.is_empty() {
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
        let velocidade = self.velocidade_efetiva();
        let por_mensagem = self.voltas_por_mensagem(velocidade);
        let voltas = match por_mensagem {
            // Ritmo proporcional: as voltas que couberem no crédito (pode ser nenhuma, como num
            // processador em que uma volta leva mais que uma mensagem: as mensagens se acumulam e
            // são drenadas juntas). O teto evita uma rajada depois de uma parada.
            Some(n) => {
                self.credito_voltas = (self.credito_voltas + n).min(3.0 * n + 1.0);
                self.credito_voltas.floor() as u32
            }
            None => {
                self.credito_voltas = 0.0;
                na_velocidade(&self.cfg.relogio.voltas_por_periodo, velocidade, 1).max(1)
            }
        };
        let mut ticks = 0;
        let havia_modal = self.avisos.tem_modal();
        for _ in 0..voltas {
            // `_gameCycles` (9:9b2c) conta as voltas do laço, rode ou não o tick. Zero significa
            // "sem relógio do laço" para `quintal::Relogios`: pula o 0 na volta.
            self.estado.ciclos_jogo = self.estado.ciclos_jogo.wrapping_add(1).max(1);
            if por_mensagem.is_some() {
                self.credito_voltas -= 1.0;
            }
            if self.relogio.liberar_tick_na_velocidade(&self.estado, &self.cfg_simulacao, velocidade) {
                self.rodar_tick();
                ticks += 1;
                if self.bloqueio_ms > 0 {
                    // Animação dentro do tick: o port encerra a chamada (a espera fica no avanço).
                    self.laco = LacoTimer::default();
                    return ticks;
                }
                if (!havia_modal && self.avisos.tem_modal()) || !self.apresentacao.is_empty() {
                    // _StopSimulation: o quadro para o temporizador; o port encerra a chamada. O
                    // resto da apresentação do tick também segura os ticks seguintes.
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
            // Teste de saída do original, na mesma ordem: o botão esquerdo só é consultado (e o
            // bit "apertado desde a última consulta" consumido) se as mensagens não bastaram.
            if (l.drenadas > 1 && (l.voltas > 2 || l.drenadas > 2)) || std::mem::take(&mut self.clique_esquerdo_pendente) {
                self.laco.dentro = false;
                break;
            }
        }
        ticks
    }

    /// O jogador apertou o botão esquerdo do mouse (em qualquer lugar): o laço de `MYTIMERFUNC`
    /// em andamento devolve o controle no fim da volta seguinte (`GetAsyncKeyState(VK_LBUTTON)`).
    pub fn botao_esquerdo_apertado(&mut self) {
        self.clique_esquerdo_pendente = true;
    }

    /// Um tick da simulação (`_DoAntSim`), sem olhar relógio nem pausa, e o que o original faz em
    /// seguida na interface (redesenho, vista).
    ///
    /// `_SpecialXfer` (telas/quintal.md §8): o original espera a escolha da colônia DENTRO do tick. No
    /// Jogo Completo com mais de uma colônia preta o port guarda o estado do começo do tick; se o tick
    /// pedir a escolha, só os pedidos até ela vão para a interface e o resto do tick fica esperando:
    /// escolhida a colônia, o tick roda de novo a partir da foto, com a escolha, e a troca acontece no
    /// mesmo ponto (os pedidos repetidos até ali são descartados).
    pub fn rodar_tick(&mut self) {
        self.vistas_para_estado();
        // `_MacTickCount` (1000:9694) = ⌊GetTickCount / 55⌋ × 3: tempo real (quintal.md, "Relógios").
        self.estado.relogio_mac = Some(self.mac_inicial.wrapping_add(((self.ms_total / 55) * 3) as u32));
        if let Some(s) = self.scripts.as_mut().filter(|s| s.tem_gancho(Gancho::AntesDoTick)) {
            s.antes_do_tick(&mut self.estado);
            self.aplicar_pedidos_dos_mods();
        }
        self.preparar_edicao_no_tick();
        let e = &self.estado;
        let foto = (e.partida.tipo_jogo == 2 && e.preta.total_colonias > 1)
            .then(|| (e.clone(), self.regras.memoria_tutor));
        self.nucleo_do_tick();
        if let Some((estado, memoria_tutor)) = foto {
            if let Some(n) = self.estado.eventos.iter().position(|ev| matches!(ev, Evento::EscolherColonia)) {
                self.estado.eventos.truncate(n + 1);
                self.foto_colonia = Some(Box::new(FotoColonia { estado, memoria_tutor, eventos: n }));
                self.vistas_do_estado();
                self.tratar_eventos();
                return;
            }
        }
        self.terminar_tick();
    }

    /// Há um tick parado na escolha de `_SpecialXfer` (guardado por `rodar_tick`).
    pub fn tick_esperando_colonia(&self) -> bool {
        self.foto_colonia.is_some()
    }

    /// A colônia de `_SpecialXfer` foi escolhida (o patch já conferido com
    /// `troca_patch::conferir_colonia`): o tick parado roda de novo da foto, com a escolha, e a troca
    /// acontece dentro dele. Devolve o que `_XferPatch` fez; `None` se não havia tick parado.
    pub fn concluir_transferencia_especial(&mut self, x: i16, y: i16) -> Option<Result<ResultadoTroca, String>> {
        let foto = self.foto_colonia.take()?;
        let Some(geracao) = self.regras.geracao.clone() else {
            return Some(Err("generation tables not loaded (patch change)".into()));
        };
        let FotoColonia { estado, memoria_tutor, eventos } = *foto;
        self.estado = estado;
        self.regras.memoria_tutor = memoria_tutor;
        self.estado.transferencia_no_tick = Some(Box::new(TransferenciaNoTick {
            patch: (x, y),
            geracao: std::sync::Arc::new(geracao),
            cfg: self.cfg_simulacao.clone(),
            resultado: None,
        }));
        self.nucleo_do_tick();
        let r = self.estado.transferencia_no_tick.take().and_then(|t| t.resultado);
        let n = eventos.min(self.estado.eventos.len());
        self.estado.eventos.drain(..n);
        self.escolher_colonia = false;
        if let Some(Ok(ResultadoTroca::Trocou { .. })) = r {
            // _RandWorld passo 18: pontos lembrados; o port leva a vista até a amarela [decisão do port].
            self.estado.pedir_centralizar_na_amarela();
        }
        self.terminar_tick();
        self.redesenhar = true;
        Some(r.unwrap_or_else(|| Err("the repeated tick did not reach _SpecialXfer".into())))
    }

    /// O tick pelas regras, com os ganchos de parte dos scripts quando há.
    fn nucleo_do_tick(&mut self) {
        match self.scripts.as_mut().filter(|s| s.tem_ganchos_do_tick()) {
            // Ganchos de parte (API 0.26): as mesmas regras, com os ganchos entre as partes; as
            // ações pedidas neles rodam logo depois do tick.
            Some(s) => {
                let mut r = RegrasComScripts { regras: &mut self.regras, scripts: s };
                tick(&mut self.estado, &mut r as &mut dyn Regras);
                self.aplicar_pedidos_dos_mods();
            }
            None => tick(&mut self.estado, &mut self.regras as &mut dyn Regras),
        }
    }

    /// O que a interface faz depois do tick: gancho "depois", vistas, pedidos e redesenho.
    fn terminar_tick(&mut self) {
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
        // _UpdateWindows com o mapa aberto: na frente (ou em ultra), _DrawMap e a alternância zera;
        // atrás, _DrawMap e _DrawMapData se revezam (telas/mapa.md §4).
        if desenha && self.mapa_aberto {
            if self.mapa_na_frente || self.estado.partida.velocidade == 3 {
                self.mapa_na_tela = true;
                self.alterna_mapa = false;
            } else {
                self.mapa_na_tela |= self.alterna_mapa;
                self.alterna_mapa = !self.alterna_mapa;
            }
        }
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
    ///
    /// Sem animação, enquanto a apresentação do tick não acabou (um aviso modal na frente), o
    /// último instantâneo da janela de edição (`Evento::Instantaneo`): o original fica parado no
    /// meio do tick com a janela desenhada até ali.
    pub fn quadro_animado(&self) -> Option<(i16, &QuadroAnimacao)> {
        if let Some((k, q)) = self.indice_do_quadro() {
            let a = &self.animacoes[k];
            return Some((a.plano, &a.quadros[q]));
        }
        if let Some(a) = self.animacoes.last() {
            if let Some(q) = a.quadros.last() {
                return Some((a.plano, q));
            }
        }
        self.quadro_congelado.as_ref().map(|(p, q)| (*p, q))
    }

    /// Há uma animação da amarela em andamento (a simulação está parada por ela).
    pub fn animando(&self) -> bool {
        !self.animacoes.is_empty()
    }

    /// A apresentação do último tick ainda não acabou (há pedidos esperando um aviso modal ou uma
    /// animação acabar).
    pub fn apresentando(&self) -> bool {
        !self.apresentacao.is_empty()
    }

    /// Um aviso modal fechou: a apresentação do tick continua de onde parou.
    fn modal_fechado(&mut self) {
        self.seguir_apresentacao();
    }

    /// O que segura a apresentação: uma animação, um aviso modal ou a escolha de `_SpecialXfer`.
    fn apresentacao_parada(&self) -> bool {
        self.animando() || self.avisos.tem_modal() || self.escolher_colonia
    }

    /// Apresenta os pedidos do tick em ordem até um que segure a apresentação (ou até o fim). No
    /// fim, sem nada segurando, a janela de edição volta a mostrar o estado.
    fn seguir_apresentacao(&mut self) {
        while !self.apresentacao_parada() {
            let Some(ev) = self.apresentacao.pop_front() else { break };
            self.tratar_evento(ev);
        }
        if self.apresentacao.is_empty() && !self.apresentacao_parada() && self.quadro_congelado.take().is_some() {
            self.redesenhar = true;
        }
        if self.estado.partida.fim_de_jogo == 0 {
            self.fim_anunciado = false;
        }
        self.aplicar_pedidos_dos_mods();
        self.saidas_dos_scripts();
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
            // Acabou: a janela fica com o último quadro até o próximo desenho.
            if let Some(a) = self.animacoes.last() {
                self.quadro_congelado = a.quadros.last().map(|q| (a.plano, q.clone()));
            }
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

    /// Esvazia a fila do estado na linha do tempo da apresentação e apresenta o que der
    /// ([`Partida::seguir_apresentacao`]).
    fn tratar_eventos(&mut self) {
        let novos = self.estado.retirar_eventos();
        self.apresentacao.extend(novos);
        self.seguir_apresentacao();
    }

    /// Apresenta um pedido: centralizar é feito aqui (`_GotoMyAnt`); o resto vai para a interface.
    /// `_EndGameDialog` é pedido a cada tick enquanto o jogo acabou: só o primeiro é repassado (de
    /// novo só depois que `_IsGameOver` voltar a 0). Mensagens e fim de jogo também entram nos
    /// avisos (quadros modais).
    fn tratar_evento(&mut self, ev: Evento) {
        if let Evento::Instantaneo { plano, quadro } = ev {
            self.quadro_congelado = Some((plano, *quadro));
            self.redesenhar = true;
            return;
        }
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
                    // A animação começa agora (o que veio antes no tick, como o quadro da aranha,
                    // já foi apresentado) e segura o resto da apresentação.
                    self.animacao_inicio_ms = self.ms_total;
                    self.bloqueio_ms += a.duracao_ms();
                    self.animacoes.push(*a);
                    self.quadro_mostrado = None;
                    self.redesenhar = true;
                    // Os sons e as esperas entre os quadros vêm logo depois do pedido da animação:
                    // vão já para a interface, que os espalha no tempo dos quadros.
                    while matches!(self.apresentacao.front(), Some(Evento::Som(_) | Evento::Espera(_))) {
                        if let Some(ev) = self.apresentacao.pop_front() {
                            self.tratar_evento(ev);
                        }
                    }
                }
            }
            outro => self.eventos.push_back(outro),
        }
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

    /// Depois de gravar o `.ANT` em `save`: `ao_salvar` dos scripts e o arquivo lateral
    /// `save.mods.json` (API 0.26, `scripts::arquivo_lateral`). O `.ANT` não muda. Sem nenhum dado de
    /// mod, um arquivo lateral antigo com esse nome é apagado (ele seria de outra gravação).
    pub fn gravar_estado_dos_mods(&mut self, save: &Path) -> Result<(), String> {
        let lateral = crate::scripts::arquivo_lateral(save);
        let dados = match self.scripts.as_mut() {
            Some(s) => s.ao_salvar(&mut self.estado),
            None => Vec::new(),
        };
        self.saidas_dos_scripts();
        if dados.is_empty() {
            if lateral.is_file() {
                std::fs::remove_file(&lateral).map_err(|e| format!("{}: {e}", lateral.display()))?;
            }
            return Ok(());
        }
        let json = serde_json::json!({
            "formato": crate::scripts::FORMATO_ARQUIVO_LATERAL,
            "api": crate::scripts::VERSAO_API,
            "tempo": self.estado.partida.tempo,
            "mods": dados.into_iter().collect::<serde_json::Map<_, _>>(),
        });
        let texto = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
        std::fs::write(&lateral, texto).map_err(|e| format!("{}: {e}", lateral.display()))
    }

    /// Depois do Load de `save` (com os scripts já ligados): lê `save.mods.json`, se existir, e chama
    /// `ao_carregar` dos mods que têm dados nele (API 0.26). Um arquivo lateral inválido, ou de outra
    /// gravação (o `tempo` não bate com o `_GameTime` do `.ANT`, por exemplo porque o original gravou
    /// por cima), é ignorado com uma linha no registro dos scripts.
    pub fn ler_estado_dos_mods(&mut self, save: &Path) {
        let lateral = crate::scripts::arquivo_lateral(save);
        if !lateral.is_file() {
            return;
        }
        let Some(s) = self.scripts.as_mut() else { return };
        let lido = std::fs::read_to_string(&lateral)
            .map_err(|e| e.to_string())
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).map_err(|e| e.to_string()));
        let aviso = |m: String| crate::tr!("mods.lateral.ignorado", arquivo = lateral.display(), motivo = m);
        let v = match lido {
            Ok(v) => v,
            Err(e) => return self.registro_scripts.push(aviso(e)),
        };
        if v.get("formato").and_then(|f| f.as_str()) != Some(crate::scripts::FORMATO_ARQUIVO_LATERAL) {
            return self.registro_scripts.push(aviso(crate::tr!("mods.lateral.formato")));
        }
        let tempo = v.get("tempo").and_then(|t| t.as_u64());
        if tempo != Some(u64::from(self.estado.partida.tempo)) {
            return self.registro_scripts.push(aviso(crate::tr!(
                "mods.lateral.outra_gravacao",
                tempo = format!("{tempo:?}"),
                tempo_save = self.estado.partida.tempo
            )));
        }
        let Some(por_mod) = v.get("mods").and_then(|m| m.as_object()) else {
            return self.registro_scripts.push(aviso(crate::tr!("mods.lateral.sem_mods")));
        };
        s.ao_carregar(por_mod, &mut self.estado);
        self.aplicar_pedidos_dos_mods();
        self.saidas_dos_scripts();
        self.redesenhar = true;
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
                        self.registro_scripts.push(format!("[{nome}] {}", crate::tr!("mods.recusado", acao = format!("{outro:?}"), motivo = motivo)));
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

    /// `_CenterEdit(_MeLocX, _MeLocY)` do fim de `_NewGame`, depois de a interface mudar o tamanho da
    /// janela de edição (no Tutorial ela é maximizada): a vista volta a centralizar na amarela.
    pub fn centralizar_edicao_na_amarela(&mut self) {
        let a = &self.estado.amarela;
        let (plano, x, y) = (a.plano, a.x, a.y);
        self.ir_para_amarela(plano, x, y);
    }

    /// `_GotoSpider` (`1000:a844`), o botão `ribToSpider` da faixa 34 (id 0x2207 em
    /// `_ProcMapRibbonEvent`): sem a aranha no mapa (`_SpidOn` = 0, como no Tutorial ou depois de ela
    /// morrer) só toca o som de erro (`_myBeginSound(1, 0, 0x7e)`); senão a vista vai à célula dela
    /// (`_SpidX >> 4`, `_SpidY >> 4`) na superfície: com a superfície na tela, `_CenterEdit` e a
    /// alternância do mapa zerada; noutro plano, `_SetMapPlaneLocation(1, x, y)` (troca de plano
    /// centralizando uma vez, sem mexer no ponto lembrado). Devolve se foi.
    pub fn ir_para_aranha(&mut self) -> bool {
        let a = &self.estado.aranha;
        if a.ligada == 0 {
            if let Some(id) = self.tabelas_amarela.sons.erro {
                self.eventos.push_back(Evento::Som(id));
            }
            return false;
        }
        let (x, y) = (i32::from(a.x >> 4), i32::from(a.y >> 4));
        if self.nav.plano_mapa == 1 {
            self.alterna_mapa = false;
            let antes = self.nav.vista;
            self.nav.vista.centralizar(x, y);
            self.nav.vista.limitar();
            self.redesenhar |= antes != self.nav.vista;
        } else {
            self.redesenhar |= self.nav.ir_para_ponto(1, x, y);
        }
        self.vistas_para_estado();
        true
    }

    /// Se algum `_UpdateWindows` desde a última chamada mostrou o mapa (`_DrawMap`), e zera.
    pub fn tomar_mapa_na_tela(&mut self) -> bool {
        std::mem::take(&mut self.mapa_na_tela)
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
        // _GotoMyAnt zera a alternância do mapa quando a amarela está no plano mostrado.
        if plano > 0 && plano == self.nav.plano_mapa {
            self.alterna_mapa = false;
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
        // O `_RandYard` de antes da leitura chama `_InitSimYard`: `_LastColonyPopR` = 1 (fora do save).
        self.estado.ultimas_colonias_vermelhas = Some(1);
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
            // _GotoMapPoint: no plano mostrado, zera a alternância do mapa (DGROUP 0x72f0)
            if plano == self.nav.plano_mapa {
                self.alterna_mapa = false;
            }
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
        ConfigAvisos::de_json(&crate::recursos::marcar_referencias(include_str!("../../../data/mensagens.json"))).unwrap()
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

    /// Partida com o ritmo do original (o laço de `MYTIMERFUNC` do 486 da bancada, sem a lenta sem
    /// foco): as extensões do port desligadas.
    fn partida_original(velocidade: i16) -> Partida {
        let mut p = partida(velocidade);
        p.cfg.relogio.ritmo_velocidades.modo = ModoRitmo::Original;
        p.cfg.relogio.sem_foco_velocidade = None;
        p
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
        assert_eq!(c.relogio.intervalo_sem_foco_ms, 17, "sem foco, o ritmo medido é o mesmo");
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
        // _YellowDialog(9100), nessa ordem; a luta espera os dois quadros da aranha fecharem, e o
        // 9100 espera a luta acabar.
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
        assert_eq!(p.avisos.modal(), None, "o 9100 só depois da luta");
        p.avancar(100);
        assert!(p.animando(), "2 quadros: 165 ms");
        p.avancar(100);
        assert!(!p.animando());
        assert_eq!(p.avisos.modal(), Some(&Modal::Amarela { imagem: 9100, indice: 0 }));
        assert_eq!(p.quadro_animado().map(|(pl, q)| (pl, q.tipo)), Some((1, 0x70)), "atrás do 9100, o último quadro da luta");
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
    fn clique_esquerdo_encerra_o_laco_de_mytimerfunc() {
        // Fast: sem clique a primeira mensagem roda as 5 voltas (5 ticks); com o botão esquerdo
        // apertado antes, o teste de saída da primeira volta consulta o botão e devolve o controle.
        let mut sem = partida_original(2);
        assert_eq!(sem.mensagem_de_timer(), 5);
        let mut com = partida_original(2);
        com.botao_esquerdo_apertado();
        assert_eq!(com.mensagem_de_timer(), 1);
        // O bit foi consumido: a mensagem seguinte abre outra chamada com as 5 voltas.
        assert_eq!(com.mensagem_de_timer(), 5);
    }

    #[test]
    fn velocidades_seguem_o_laco_de_mytimerfunc() {
        // Cada chamada de MYTIMERFUNC consome 3 mensagens (a dela e 2 drenadas); o atraso é testado
        // na entrada e depois da primeira drenada. Bancada: slow 3,0 ticks/s, normal 6,0, fast
        // 55–63, ultra 91 (18 mensagens = 6 chamadas = 0,99 s; o estado vazio abre um quadro no
        // tick 128, por isso a conta para antes).
        assert_eq!(ticks_em(&mut partida(0), 18), 2, "slow: 1 tick a cada 6 mensagens (330 ms)");
        assert_eq!(ticks_em(&mut partida(1), 18), 6, "normal: 1 tick a cada 3 mensagens (165 ms)");
        assert_eq!(ticks_em(&mut partida_original(2), 18), 66, "fast: 2 × 5 + 1 por chamada");
        assert_eq!(ticks_em(&mut partida_original(3), 18), 90, "ultra: 2 × 7 + 1 por chamada");
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
        let mut p = partida_original(3);
        let t = ticks_em(&mut p, 30);
        assert_eq!(p.estado.ciclos_jogo, t);
    }

    /// Ticks em `ms` de tempo real, em avanços de 16 ms (um quadro a 60 Hz).
    fn ticks_em_ms(p: &mut Partida, ms: u64) -> u32 {
        (0..ms / 16).map(|_| p.avancar(16).ticks).sum()
    }

    #[test]
    fn ritmo_proporcional_de_fast_e_ultra() {
        // Escolha do usuário (2026-10-19): Fast ≈ 2,5 × Normal e Ultra ≈ 5 × Normal (data/partida.json).
        let p = partida(1);
        assert_eq!(p.cfg.relogio.ritmo_velocidades.modo, ModoRitmo::Proporcional, "padrão de data/partida.json");
        assert_eq!(p.ticks_por_segundo(1), None, "Normal segue o laço do original");
        let fast = p.ticks_por_segundo(2).unwrap();
        let ultra = p.ticks_por_segundo(3).unwrap();
        assert!((fast - 15.15).abs() < 0.01 && (ultra - 30.3).abs() < 0.01, "{fast} {ultra}");
        // ~90 ticks de cada (o estado vazio abre um quadro no tick 100, que para a simulação).
        let normal = ticks_em_ms(&mut partida(1), 15_000);
        let fast = ticks_em_ms(&mut partida(2), 6_000);
        let ultra = ticks_em_ms(&mut partida(3), 3_000);
        assert!((89..=92).contains(&normal), "normal: {normal} em 15 s");
        assert!((89..=92).contains(&fast), "fast: {fast} em 6 s (15,15/s)");
        assert!((89..=92).contains(&ultra), "ultra: {ultra} em 3 s (30,3/s)");
        // Como no original em Fast/Ultra, cada volta do laço roda um tick: _gameCycles = ticks.
        let mut p = partida(3);
        let t = ticks_em_ms(&mut p, 2_000);
        assert_eq!(p.estado.ciclos_jogo, t);
        // Pausado, o crédito não se acumula numa rajada: ao despausar, o ritmo segue o mesmo.
        let mut p = partida(3);
        p.aplicar(AcaoPartida::Pausa);
        assert_eq!(ticks_em_ms(&mut p, 5_000), 0);
        p.aplicar(AcaoPartida::Pausa);
        assert!(ticks_em(&mut p, 1) <= 2, "sem rajada depois da pausa");
    }

    #[test]
    fn os_ticks_sao_os_mesmos_em_qualquer_ritmo() {
        // O ritmo só muda quando os ticks rodam: a sequência de estados é a mesma nos dois modos.
        let mut orig = partida_original(2);
        let mut prop = partida(2);
        let n = 60;
        while orig.estado.partida.tempo < n {
            orig.mensagem_de_timer();
        }
        while prop.estado.partida.tempo < n {
            prop.mensagem_de_timer();
        }
        assert_eq!(orig.estado.partida.tempo, prop.estado.partida.tempo);
        assert_eq!(orig.estado.s, prop.estado.s);
        assert_eq!(orig.estado.ciclos_jogo, prop.estado.ciclos_jogo);
    }

    #[test]
    fn sem_foco_a_cadencia_cai_para_a_lenta_e_volta() {
        // Escolha do usuário (2026-10-19): sem foco, a Slow; com o foco, a escolhida. _GameSpeed
        // (save, menu Speed) não muda.
        let mut p = partida(3);
        p.definir_foco(false);
        assert_eq!((p.velocidade_efetiva(), p.estado.partida.velocidade), (0, 3));
        let lenta = ticks_em_ms(&mut p, 10_000);
        assert!((29..=32).contains(&lenta), "slow: {lenta} em 10 s");
        assert_eq!(p.estado.partida.velocidade, 3, "o _GameSpeed fica");
        p.definir_foco(true);
        assert_eq!(p.velocidade_efetiva(), 3);
        assert!(ticks_em_ms(&mut p, 1_000) >= 28, "de volta à Ultra");
        // Já na Slow, nada muda; sem a opção, o ritmo do original (o mesmo sem foco).
        let mut p = partida(0);
        p.definir_foco(false);
        assert_eq!(p.velocidade_efetiva(), 0);
        let mut p = partida_original(1);
        p.definir_foco(false);
        assert_eq!(p.velocidade_efetiva(), 1);
        assert_eq!(ticks_em(&mut p, 18), 6, "normal sem foco no original");
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
    fn sem_foco_o_ritmo_nao_muda() {
        // Bancada (2026-10-19): 61 ticks em 10 s ativa, com a Task List na frente e minimizada.
        let mut p = partida(1);
        p.definir_foco(false);
        assert_eq!(p.intervalo_ms(), 55);
        assert_eq!(p.avancar(54).mensagens, 0);
        assert_eq!(p.avancar(1).mensagens, 1);
        // Um mod que volte aos 170 ms do código tem o arredondamento para 220 ms.
        p.cfg.relogio.intervalo_sem_foco_ms = 170;
        assert_eq!(p.intervalo_ms(), 220);
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
            // O fim de jogo é modal: segura a apresentação até fechar.
            p.fechar_modal();
        }
        // _CenterEdit em (43, 30) com 25 × 19 (_editWidth da bancada): canto (31, 21).
        assert_eq!((p.nav.vista.x, p.nav.vista.y), (31, 21));
        assert_eq!(p.estado.vistas.ponto, (31, 21));
        assert_eq!(
            p.retirar_eventos(),
            vec![Evento::Musica(7), Evento::DialogoFimDeJogo, Evento::Musica(7), Evento::Musica(7)]
        );
    }

    /// Regras em que a amarela, na superfície em (10, 10), perde a luta para um soldado vermelho em
    /// (11, 10) e renasce do ovo preto em (30, 10) do ninho preto (as regras de verdade:
    /// `_YellowFight` → `_YellowDeath(0)` → `_YellowBirth`).
    struct LutaPerdida;
    impl RegrasQuintal for LutaPerdida {}
    impl RegrasInimigos for LutaPerdida {}
    impl RegrasSuperficie for LutaPerdida {}
    impl RegrasNinho for LutaPerdida {}
    impl RegrasConselhos for LutaPerdida {}
    impl RegrasAmarela for LutaPerdida {
        fn mover_amarela(&mut self, e: &mut Estado) {
            crate::rules::amarela::lutar_com_amarela(e, TabelasAmarela::padrao(), 1, 0);
        }
    }

    fn partida_da_luta_perdida() -> Partida {
        use crate::formats::save::{Formiga, Lista};
        let mut p = partida(1);
        let e = &mut p.estado;
        e.partida.tipo_jogo = 1;
        e.partida.opcoes[1] = 0; // sem música: os quadros da luta correm
        e.partida.opcoes[3] = 1; // Events
        let a = &mut e.amarela;
        (a.plano, a.x, a.y, a.tipo, a.dir, a.saude, a.aviso_saude) = (1, 10, 10, 0x10, 2, 100, 30);
        (a.comeu_tipo, a.ovo_carregado, a.contorno_contagem, a.ninho_iniciado) = (-1, 0xFD, -2, 1);
        crate::rules::amarela::simular_amarela(e, TabelasAmarela::padrao());
        e.adicionar_formiga(Lista::A, Formiga { x: 11, y: 10, modo: 2, tipo: 0xB6, s: 0 });
        e.ninho_b.lista.adicionar(Formiga { x: 30, y: 10, modo: 8, tipo: 3, s: 0 });
        e.ninho_b.vida.definir(30, 10, 3);
        // Uma semente de R em que o soldado vermelho vence.
        let semente = (0..).find(|&s| crate::rules::rng::GeradorR::new(s).rrand(10) >= 2).unwrap();
        e.r = crate::rules::rng::GeradorR::new(semente);
        p
    }

    #[test]
    fn morte_na_luta_e_apresentada_na_ordem_do_original() {
        // _YellowFight perdido, dentro de um tick: _GotoMyAnt no lugar da luta, _AnimYellowFight ali,
        // o quadro 10032 com a vista ainda no lugar da morte (a amarela já fora do mapa de vida),
        // e só depois _YellowBirth: _GotoMyAnt no ovo, os estágios do ovo e o quadro 9101.
        let mut p = partida_da_luta_perdida();
        let estado_final = {
            let mut q = partida_da_luta_perdida();
            q.preparar_edicao_no_tick();
            tick(&mut q.estado, &mut LutaPerdida);
            q.estado.eventos.clear();
            q.estado
        };
        p.preparar_edicao_no_tick();
        tick(&mut p.estado, &mut LutaPerdida);
        p.tratar_eventos();
        // O estado já é o do fim do tick (a apresentação não roda regras de novo).
        assert_eq!((p.estado.amarela.plano, p.estado.amarela.x, p.estado.amarela.y), (2, 30, 10));
        // 1. A luta, no lugar dela.
        assert!(p.animando());
        assert_eq!(p.nav.plano_mapa, 1, "a vista foi para a luta, na superfície");
        let v = &p.nav.vista;
        assert!((v.x..v.x + v.largura).contains(&10) && (v.y..v.y + v.altura).contains(&10));
        assert_eq!(p.quadro_animado().map(|(pl, q)| (pl, q.tipo, q.x, q.y)), Some((1, 0x70, 10, 10)));
        assert_eq!(p.avisos.modal(), None, "o quadro da morte só depois da luta");
        assert_eq!(p.avancar(5000).ticks, 0);
        // 2. A morte: o quadro com a vista ainda na superfície e a janela de edição como o original a
        // desenhou em _YellowDeath (sem a amarela; o cadáver no lugar dela).
        assert!(!p.animando());
        assert!(matches!(p.avisos.modal(), Some(Modal::Quadro { texto: 10032, .. })), "{:?}", p.avisos.modal());
        assert_eq!(p.nav.plano_mapa, 1);
        let (pl, q) = p.quadro_animado().expect("a janela de edição parada no instante da morte");
        assert_eq!(pl, 1);
        assert_ne!(q.vida[10 * 64 + 10], 0xFF, "a amarela já saiu do mapa de vida");
        assert!(p.apresentando());
        assert_eq!(p.avancar(5000).ticks, 0, "nenhum tick com a apresentação parada");
        // 3. O renascimento: a vista vai ao ovo, no ninho preto, e os estágios do ovo.
        p.fechar_modal();
        assert_eq!(p.nav.plano_mapa, 2, "a câmera vai ao corpo novo só depois do quadro");
        assert!(p.animando(), "os estágios do ovo");
        assert_eq!(p.quadro_animado().map(|(pl, q)| (pl, q.x, q.y, q.ovo, q.tipo)), Some((2, 30, 10, 2, 0)));
        assert_eq!(p.avisos.modal(), None);
        p.avancar(6 * 220);
        // 4. "You have been reborn!" com a formiga pronta na tela.
        assert!(!p.animando());
        assert_eq!(p.avisos.modal(), Some(&Modal::Amarela { imagem: 9101, indice: 1 }));
        assert_eq!(p.quadro_animado().map(|(pl, q)| (pl, q.tipo)), Some((2, 0x10)));
        p.fechar_modal();
        assert!(!p.apresentando());
        assert!(p.quadro_animado().is_none(), "acabada a apresentação, a janela mostra o estado");
        // A apresentação não muda o estado do fim do tick (geradores, amarela, mapas).
        assert_eq!((p.estado.s, p.estado.r), (estado_final.s, estado_final.r));
        assert_eq!(format!("{:?}", p.estado.amarela), format!("{:?}", estado_final.amarela));
        assert_eq!(p.estado.superficie.vida.dados, estado_final.superficie.vida.dados);
        assert_eq!(p.estado.ninho_b.vida.dados, estado_final.ninho_b.vida.dados);
        assert!(p.avancar(2000).ticks > 0, "a simulação volta");
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
