//! Laço de jogo ao vivo, sem janela: guarda o estado da simulação, as regras, a navegação da janela
//! de edição e o relógio, e decide quando rodar um tick.
//!
//! Especificação: docs/spec/regras/simulacao-visao-geral.md (§1–4: `MYTIMERFUNC`, velocidade e
//! pausa) e docs/spec/telas/jogo.md (seção "Simulação ao vivo"). Dados: data/partida.json.
//!
//! O executável chama [`Partida::avancar`] com o tempo real passado; aqui o tempo vira "mensagens
//! de temporizador" (17 ms no original), e cada mensagem passa pela regra de velocidade e pausa do
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
use crate::rules::amarela::{
    clicar_no_mapa, tecla_de_comando, ControleJogador, Modificadores, RespostaClique, RespostaTecla, TabelasAmarela,
};
use crate::rules::estado::{ConfigSimulacao, Estado, Evento};
use crate::rules::tick::{tick, Regras, RegrasOriginais, Relogio};

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_CONFIG: &str = "partida.json";

/// Ritmo das mensagens de temporizador e do redesenho (data/partida.json, "relogio").
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConfigRelogio {
    /// Intervalo do `SetTimer` com o aplicativo em foco (17 ms).
    pub intervalo_mensagem_ms: u32,
    /// Intervalo sem foco (`WM_ACTIVATEAPP` com wParam = 0: 170 ms).
    pub intervalo_sem_foco_ms: u32,
    /// Voltas do laço de `MYTIMERFUNC` por mensagem, por `_GameSpeed`.
    pub iteracoes_por_mensagem: Vec<u32>,
    /// Máximo de mensagens geradas numa chamada de `avancar`; o resto do tempo é descartado.
    pub max_mensagens_por_avanco: u32,
    /// Redesenho do mapa a cada N ticks, por `_GameSpeed` (`_UpdateWindows`).
    pub redesenho_mapa_a_cada: Vec<u32>,
    /// Sem foco, redesenha só a cada N ticks (`quadros & 7 == 0`).
    pub redesenho_sem_foco_a_cada: u32,
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
}

/// Resultado de um botão configurável da faixa (`_DoUserButton`, `1000:04c4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RespostaBotao {
    /// Feito aqui (pausa, velocidade, opções, ir até a amarela).
    Feito,
    /// Caso 0xE: salvar o jogo (`_SaveGame(1)`), a cargo da interface.
    Salvar,
    /// Abre uma janela que o port ainda não tem (0–4, 8) ou número desconhecido.
    NaoImplementado(u16),
}

/// Botões configuráveis da faixa (`ribUserDef1A`–`1H`) e a função de cada um.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct BotoesUsuario {
    pub objetos: Vec<String>,
    pub funcoes: Vec<u16>,
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
    /// Tempo real acumulado que ainda não completou uma mensagem.
    resto_ms: u64,
    /// Tempo real total passado a `avancar` (o `GetTickCount` do port, ms).
    ms_total: u64,
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
        let mut p = Partida {
            estado,
            regras,
            tabelas_amarela,
            controle: ControleJogador::default(),
            nav,
            cfg_simulacao,
            cfg,
            relogio: Relogio::default(),
            resto_ms: 0,
            ms_total: 0,
            ativo: true,
            quadros: 0,
            contador_mapa: 0,
            redesenhar: true,
            fim_anunciado: false,
            eventos: VecDeque::new(),
            avisos: Avisos::new(cfg_avisos),

            bloqueio_ms: 0,
        };
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

    /// Intervalo atual entre mensagens de temporizador (ms).
    pub fn intervalo_ms(&self) -> u64 {
        let r = &self.cfg.relogio;
        u64::from(if self.ativo { r.intervalo_mensagem_ms } else { r.intervalo_sem_foco_ms }).max(1)
    }

    /// Foco do aplicativo (`WM_ACTIVATEAPP`): sem foco, o temporizador fica em 170 ms.
    pub fn definir_foco(&mut self, ativo: bool) {
        self.ativo = ativo;
    }

    /// `_TickCount` (unidades de 55 ms) do relógio real da partida.
    pub fn agora(&self) -> u64 {
        self.ms_total / self.avisos.cfg.ms_por_tickcount.max(1)
    }

    /// Passa `ms` de tempo real: gera as mensagens de temporizador que couberem (no máximo
    /// `max_mensagens_por_avanco`, descartando o excesso) e roda os ticks que a velocidade e a
    /// pausa liberarem. Com um aviso modal na tela, ou durante a espera do clique simples, o tempo
    /// passa sem mensagens (o original mata o temporizador ou fica num laço de espera).
    pub fn avancar(&mut self, ms: u64) -> Avanco {
        // O GetTickCount continua andando durante quadros modais e esperas.
        self.ms_total += ms;
        if self.avisos.passar_tempo(self.agora()).is_some() {
            self.redesenhar = true;
        }
        if self.avisos.tem_modal() {
            self.resto_ms = 0;
            return Avanco::default();
        }
        let espera = ms.min(self.bloqueio_ms);
        self.bloqueio_ms -= espera;
        let ms = ms - espera;
        if self.bloqueio_ms > 0 {
            return Avanco::default();
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
            if self.avisos.tem_modal() {
                // _StopSimulation: KillTimer, e as WM_TIMER pendentes são descartadas.
                self.resto_ms = 0;
                self.redesenhar = true;
                break;
            }
        }
        av
    }

    /// Uma chamada de `MYTIMERFUNC` com uma mensagem: atraso + 1 e as voltas do laço (cada uma
    /// roda no máximo um tick, se `Relogio::liberar_tick` deixar). Devolve os ticks rodados.
    pub fn mensagem_de_timer(&mut self) -> u32 {
        // `_gameCycles` (9:9b2c) conta o laço do temporizador mesmo sem tick (pausa, velocidade).
        // O port conta uma vez por mensagem: o quintal o usa também como relógio do Mac (1/60 s ≈
        // 17 ms) [hipótese; o original soma 1 por volta do laço, ver jogo.md "Simulação ao vivo"].
        // Zero significa "sem relógio do laço" para `quintal::Relogios`: pula o 0 na volta.
        self.estado.ciclos_jogo = self.estado.ciclos_jogo.wrapping_add(1).max(1);
        self.relogio.contar_mensagem();
        let voltas = self.indice_velocidade(&self.cfg.relogio.iteracoes_por_mensagem.clone(), 1).max(1);
        let mut ticks = 0;
        for _ in 0..voltas {
            if self.relogio.liberar_tick(&self.estado, &self.cfg_simulacao) {
                self.rodar_tick();
                ticks += 1;
            }
        }
        ticks
    }

    /// Um tick da simulação (`_DoAntSim`), sem olhar relógio nem pausa, e o que o original faz em
    /// seguida na interface (redesenho, vista).
    pub fn rodar_tick(&mut self) {
        self.vistas_para_estado();
        // `_MacTickCount` (1000:9694) = ⌊GetTickCount / 55⌋ × 3: tempo real (quintal.md, "Relógios").
        self.estado.relogio_mac = Some(((self.ms_total / 55) * 3) as u32);
        tick(&mut self.estado, &mut self.regras as &mut dyn Regras);
        self.vistas_do_estado();
        self.tratar_eventos();
        self.conselho_na_faixa();
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

    /// Devolve se a tela deve ser redesenhada (e limpa o pedido).
    pub fn tomar_redesenho(&mut self) -> bool {
        std::mem::take(&mut self.redesenhar)
    }

    /// Retira os pedidos das regras à interface (mensagens, músicas, fim de jogo), em ordem.
    pub fn retirar_eventos(&mut self) -> Vec<Evento> {
        self.eventos.drain(..).collect()
    }

    // --- vistas: navegação ↔ estado ------------------------------------------------------------

    /// As regras leem o plano mostrado (`_MapPlane`) e os pontos lembrados; a navegação é a dona.
    fn vistas_para_estado(&mut self) {
        let e = &mut self.estado;
        let n = &self.nav;
        let par = |(x, y): (i32, i32)| (x as i16, y as i16);
        e.vistas.plano = n.plano_mapa;
        e.vistas.ponto = (n.vista.x as i16, n.vista.y as i16);
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
            match ev {
                Evento::CentralizarNaAmarela => self.ir_para_amarela(),
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
                Evento::Mensagem(id) => {
                    let licoes = &self.regras.quintal.conselhos.licoes;
                    self.avisos.receber_mensagem(id, self.estado.partida.opcoes[3] != 0, licoes);
                    self.eventos.push_back(ev);
                }
                outro => self.eventos.push_back(outro),
            }
        }
        if self.estado.partida.fim_de_jogo == 0 {
            self.fim_anunciado = false;
        }
    }

    /// Parte de vista de `_GotoMyAnt`: no Experimental só toca o som de erro; senão leva a vista
    /// até a amarela.
    fn ir_para_amarela(&mut self) {
        if self.estado.partida.tipo_jogo == 3 {
            if let Some(id) = self.tabelas_amarela.sons.erro {
                self.eventos.push_back(Evento::Som(id));
            }
            return;
        }
        let a = &self.estado.amarela;
        let (plano, x, y) = (a.plano, a.x as i32, a.y as i32);
        self.redesenhar |= self.nav.ir_para_amarela(plano, x, y);
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
    fn definir_pausa(&mut self, pausado: bool) {
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

    // --- avisos modais -------------------------------------------------------------------------

    /// Fecha o aviso modal da frente (clique na área do quadro, Espaço, clique no fim de jogo) e
    /// devolve qual era. Depois do fim de jogo, o original chama `_NewGame(0)`: a interface decide.
    pub fn fechar_modal(&mut self) -> Option<Modal> {
        self.redesenhar = true;
        self.avisos.fechar_modal()
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
            AcaoPartida::Salvar | AcaoPartida::SalvarComo => {}
        }
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
            9 => alternar(&mut p.opcoes[2]),
            10 => alternar(&mut p.opcoes[5]),
            11 => alternar(&mut p.opcoes[0]),
            12 => alternar(&mut p.opcoes[3]),
            13 => alternar(&mut p.opcoes[4]),
            14 => return RespostaBotao::Salvar,
            15 => self.ir_para_amarela(),
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

    /// Clique numa célula do mapa mostrado (`_processEdit`). `duplo` = duplo clique (esquerdo ou
    /// direito). Menus de "segurar o botão" ainda não existem no port (`segurando` = falso). O clique
    /// simples para a simulação por `espera_clique_simples_tickcount` (o laço de espera do original).
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
            vista: VistaEdicao { plano: Plano::NinhoPreto, x: 0, y: 0, largura: 26, altura: 19 },
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
        assert_eq!(c.botao_da_tecla("Delete"), Some(BotaoTeclado::Direito));
    }

    #[test]
    fn mensagens_a_cada_17_ms_com_resto() {
        let mut p = partida(1);
        assert_eq!(p.avancar(16).mensagens, 0);
        assert_eq!(p.avancar(1).mensagens, 1);
        assert_eq!(p.avancar(33).mensagens, 1);
        assert_eq!(p.avancar(1).mensagens, 1, "o resto de 16 ms foi guardado");
    }

    #[test]
    fn mensagens_nao_se_acumulam_alem_do_maximo() {
        let mut p = partida(1);
        let av = p.avancar(10_000);
        assert_eq!(av.mensagens, 3);
        assert_eq!(p.avancar(0).mensagens, 0, "o tempo excedente foi descartado");
    }

    /// Ticks em `n` mensagens seguidas.
    fn ticks_em(p: &mut Partida, n: u32) -> u32 {
        (0..n).map(|_| p.mensagem_de_timer()).sum()
    }

    #[test]
    fn velocidades_seguem_speed_delay_vals() {
        // slow: atraso 6 → 1 tick a cada 6 mensagens; normal: a cada 2; fast e ultra: toda volta.
        assert_eq!(ticks_em(&mut partida(0), 60), 10);
        assert_eq!(ticks_em(&mut partida(1), 60), 30);
        assert_eq!(ticks_em(&mut partida(2), 60), 120);
        assert_eq!(ticks_em(&mut partida(3), 60), 120);
        let mut p = partida(0);
        assert_eq!(ticks_em(&mut p, 5), 0);
        assert_eq!(ticks_em(&mut p, 1), 1, "o sexto libera");
        assert_eq!(p.estado.partida.tempo, 1);
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
    fn sem_foco_o_temporizador_fica_em_170_ms() {
        let mut p = partida(1);
        p.definir_foco(false);
        assert_eq!(p.avancar(169).mensagens, 0);
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
            e.eventos.push(Evento::CentralizarNaAmarela);
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
        // _CenterEdit em (43, 30) com 26 × 19: canto (30, 21).
        assert_eq!((p.nav.vista.x, p.nav.vista.y), (30, 21));
        assert_eq!(p.estado.vistas.ponto, (30, 21));
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
        p.estado.eventos.push(Evento::CentralizarNaAmarela);
        p.tratar_eventos();
        assert_eq!(p.nav.plano_mapa, 1);
        assert_eq!(p.nav.vista.plano, Plano::Superficie);
        // dx = 87, dy = 1: a fração 65536/87 truncada nunca liga o bit 16; y não anda (jogo.md).
        assert_eq!((p.nav.vista.x, p.nav.vista.y), (87, 0));
        assert_eq!(p.estado.vistas.plano, 1);
    }
}
