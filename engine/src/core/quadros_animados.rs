//! Quadros animados da janela 26 (`dialogPicWin`): `_SpiderDialog` (`1000:673a`, a aranha come a
//! amarela), `_LionDialog` (`1000:80a0`, a formiga-leão) e `_DrawSimPayoff` (`1000:cda8`, vitória do
//! Jogo Completo e trapaça `fred`).
//!
//! Especificação: docs/spec/telas/mensagens.md §11. Dados: data/mensagens.json → `quadros_animados`.
//!
//! Os três seguem o mesmo roteiro, com números diferentes: (espera antes de abrir), a janela abre
//! com um bitmap, (som e músicas), (espera o som acabar), e um laço que, a cada `intervalo_mac`
//! unidades do relógio do Mac, desenha o quadro seguinte de uma lista em uma ou mais posições
//! (com som em alguns quadros), até o prazo (`_DialogWaitInit` ou duração no relógio do Mac), um
//! clique ou uma tecla; depois, (espera final) e a janela fecha (ou fica, na aranha). Aqui só fica
//! o estado do roteiro, no relógio `_TickCount` da partida; o desenho é de `avisos.rs`.

use std::collections::HashMap;

use serde::Deserialize;

use crate::rules::estado::Evento;

/// Qual quadro animado.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TipoAnimado {
    /// `_SpiderDialog`.
    Aranha,
    /// `_LionDialog`.
    FormigaLeao,
    /// `_DrawSimPayoff`.
    Vitoria,
}

/// Roteiro de um quadro animado (data/mensagens.json → `quadros_animados.<nome>`). Tempos em
/// unidades de `_TickCount` (55 ms) ou do relógio do Mac (`_MacTickCount` = 3 por `_TickCount`).
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigAnimado {
    /// Bitmap posto no objeto `dialogPic` ao abrir (`_win_SetObjBitmap`).
    pub bitmap: u16,
    /// `_myDelay` antes de abrir a janela (vitória: 150, com o quintal já na frente).
    #[serde(default)]
    pub espera_antes_tickcount: u64,
    /// `_myBeginSound` ao abrir.
    #[serde(default)]
    pub som_inicio: Option<u16>,
    /// `_myBeginSong` em sequência durante o laço (vitória: 20003 e 20004).
    #[serde(default)]
    pub musicas: Vec<u16>,
    /// Espera o som do início acabar antes do laço (`_mySoundIsDone`), em ms (duração do efeito).
    #[serde(default)]
    pub espera_som_ms: u64,
    /// Quadros do laço (bitmaps), na ordem.
    pub quadros: Vec<u16>,
    /// Índice do primeiro quadro desenhado.
    #[serde(default)]
    pub primeiro: usize,
    /// Depois do último quadro, volta para este índice.
    #[serde(default)]
    pub volta_para: usize,
    /// Onde cada quadro é desenhado, a partir do canto do bitmap da janela (vários = o mesmo quadro
    /// em todas as posições).
    pub posicoes: Vec<[i16; 2]>,
    /// Índice do quadro → efeito tocado ao desenhá-lo.
    #[serde(default)]
    pub sons_quadro: HashMap<String, u16>,
    /// Intervalo entre quadros no relógio do Mac.
    pub intervalo_mac: u64,
    /// Duração do laço no relógio do Mac (sem ela, só o prazo em `_TickCount`).
    #[serde(default)]
    pub duracao_mac: Option<u64>,
    /// Prazo do laço em `_TickCount` (`_DialogWaitInit(n)` = 18 × n).
    #[serde(default)]
    pub limite_tickcount: Option<u64>,
    /// `_myDelay` depois do laço, com a janela ainda aberta.
    #[serde(default)]
    pub espera_final_tickcount: u64,
    /// A janela continua aberta depois (a aranha não chama `_win_Close`; o quadro de texto vem por
    /// cima e o `_YellowDialog(9100)` reaproveita a janela).
    #[serde(default)]
    pub fica_aberta: bool,
}

/// `quadros_animados` de data/mensagens.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigQuadrosAnimados {
    /// `_MacTickCount` por `_TickCount` (3: `_MacTickCount` = `_TickCount` × 3).
    pub mac_por_tickcount: u64,
    pub aranha: Option<ConfigAnimado>,
    pub formiga_leao: Option<ConfigAnimado>,
    pub vitoria: Option<ConfigAnimado>,
}

impl ConfigQuadrosAnimados {
    pub fn de(&self, tipo: TipoAnimado) -> Option<&ConfigAnimado> {
        match tipo {
            TipoAnimado::Aranha => self.aranha.as_ref(),
            TipoAnimado::FormigaLeao => self.formiga_leao.as_ref(),
            TipoAnimado::Vitoria => self.vitoria.as_ref(),
        }
    }
}

/// Fase do roteiro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fase {
    /// Antes de abrir a janela.
    Antes,
    /// Janela aberta, esperando o som do início.
    Som,
    /// O laço dos quadros.
    Laco,
    /// Depois do laço.
    Final,
    /// Acabou (a janela fecha, ou fica se `fica_aberta`).
    Fim,
}

/// Estado de um quadro animado em andamento.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Animado {
    pub tipo: TipoAnimado,
    pub fase: Fase,
    /// Início da fase atual (`_TickCount`).
    inicio_fase: u64,
    /// Início do laço no relógio do Mac (para a duração em `_MacTickCount`).
    inicio_laco_mac: u64,
    /// Próximo quadro no relógio do Mac.
    proximo_mac: u64,
    /// Contador de quadros do laço (índice na lista do próximo quadro a desenhar).
    indice: usize,
    /// Quadro desenhado por último (bitmap), se algum.
    pub quadro: Option<u16>,
    /// Músicas já pedidas.
    musicas_pedidas: usize,
}

/// Unidades de `_TickCount` até `ms` ter passado.
fn ms_em_tickcount(ms: u64, ms_por_tickcount: u64) -> u64 {
    ms.div_ceil(ms_por_tickcount.max(1))
}

impl Animado {
    /// Começa o roteiro no instante `agora` (`_TickCount`). Devolve o estado e os pedidos à interface.
    pub fn novo(tipo: TipoAnimado, c: &ConfigAnimado, cfg: &ConfigQuadrosAnimados, agora: u64, ms_por_tickcount: u64) -> (Animado, Vec<Evento>) {
        let mut a = Animado {
            tipo,
            fase: Fase::Antes,
            inicio_fase: agora,
            inicio_laco_mac: 0,
            proximo_mac: 0,
            indice: c.primeiro,
            quadro: None,
            musicas_pedidas: 0,
        };
        let mut pedidos = Vec::new();
        if c.espera_antes_tickcount == 0 {
            a.abrir(c, cfg, agora, ms_por_tickcount, &mut pedidos);
        }
        (a, pedidos)
    }

    /// A janela já está na tela.
    pub fn janela_visivel(&self) -> bool {
        !matches!(self.fase, Fase::Antes)
    }

    fn abrir(&mut self, c: &ConfigAnimado, cfg: &ConfigQuadrosAnimados, agora: u64, ms_por_tickcount: u64, pedidos: &mut Vec<Evento>) {
        self.fase = Fase::Som;
        self.inicio_fase = agora;
        pedidos.extend(c.som_inicio.map(Evento::Som));
        if ms_em_tickcount(c.espera_som_ms, ms_por_tickcount) == 0 {
            self.comecar_laco(c, cfg, agora);
        }
    }

    fn comecar_laco(&mut self, c: &ConfigAnimado, cfg: &ConfigQuadrosAnimados, agora: u64) {
        self.fase = Fase::Laco;
        self.inicio_fase = agora;
        self.inicio_laco_mac = agora * cfg.mac_por_tickcount;
        self.proximo_mac = self.inicio_laco_mac + c.intervalo_mac;
    }

    fn terminar_laco(&mut self, c: &ConfigAnimado, agora: u64) {
        self.fase = if c.espera_final_tickcount > 0 { Fase::Final } else { Fase::Fim };
        self.inicio_fase = agora;
    }

    /// Passa o relógio até `agora` (`_TickCount`). Devolve os pedidos (sons, músicas) e se a tela
    /// mudou.
    pub fn passar_tempo(&mut self, c: &ConfigAnimado, cfg: &ConfigQuadrosAnimados, agora: u64, ms_por_tickcount: u64) -> (Vec<Evento>, bool) {
        let mut pedidos = Vec::new();
        let mut mudou = false;
        loop {
            match self.fase {
                Fase::Antes => {
                    if agora < self.inicio_fase + c.espera_antes_tickcount {
                        break;
                    }
                    let t = self.inicio_fase + c.espera_antes_tickcount;
                    self.abrir(c, cfg, t, ms_por_tickcount, &mut pedidos);
                    mudou = true;
                }
                Fase::Som => {
                    let fim = self.inicio_fase + ms_em_tickcount(c.espera_som_ms, ms_por_tickcount);
                    if agora < fim {
                        break;
                    }
                    self.comecar_laco(c, cfg, fim);
                }
                Fase::Laco => {
                    // Músicas: `_mySongIsDone` é imediato no MCI do Windows (som.md), então cada
                    // volta do laço pede a seguinte e a última fica.
                    while self.musicas_pedidas < c.musicas.len() {
                        pedidos.push(Evento::Musica(c.musicas[self.musicas_pedidas]));
                        self.musicas_pedidas += 1;
                    }
                    // O relógio do Mac anda de `mac_por_tickcount` em `mac_por_tickcount`: cada prazo vence
                    // no primeiro `_TickCount` em que o relógio o alcançou. O teste do prazo do laço vem
                    // antes do desenho, a cada volta.
                    let m = cfg.mac_por_tickcount.max(1);
                    let fim_mac = [c.limite_tickcount.map(|l| (self.inicio_fase + l) * m), c.duracao_mac.map(|d| self.inicio_laco_mac + d)]
                        .into_iter()
                        .flatten()
                        .min();
                    let fim_tc = fim_mac.map(|f| f.div_ceil(m));
                    let tem_quadros = !c.quadros.is_empty() && c.intervalo_mac > 0;
                    let quadro_tc = tem_quadros.then(|| self.proximo_mac.div_ceil(m));
                    if let Some(f) = fim_tc.filter(|&f| quadro_tc.is_none_or(|q| f <= q)) {
                        if agora < f {
                            break;
                        }
                        self.terminar_laco(c, f);
                        mudou = true;
                        continue;
                    }
                    let Some(quadro_tc) = quadro_tc else { break };
                    if agora < quadro_tc {
                        break;
                    }
                    let quadro_mac = quadro_tc * m;
                    let i = self.indice.min(c.quadros.len() - 1);
                    self.quadro = Some(c.quadros[i]);
                    if let Some(&s) = c.sons_quadro.get(&i.to_string()) {
                        pedidos.push(Evento::Som(s));
                    }
                    self.indice = i + 1;
                    if self.indice >= c.quadros.len() {
                        self.indice = c.volta_para.min(c.quadros.len() - 1);
                    }
                    self.proximo_mac = quadro_mac + c.intervalo_mac;
                    mudou = true;
                }
                Fase::Final => {
                    if agora < self.inicio_fase + c.espera_final_tickcount {
                        break;
                    }
                    self.fase = Fase::Fim;
                    mudou = true;
                }
                Fase::Fim => break,
            }
        }
        (pedidos, mudou)
    }

    /// Clique ou tecla: encerra a espera da vez (`_myDelay` e `_DialogAbortOrCont` saem com um
    /// evento). A espera do som do leão não é interrompida (`_mySoundIsDone` num laço sem eventos).
    pub fn pular(&mut self, c: &ConfigAnimado, cfg: &ConfigQuadrosAnimados, agora: u64, ms_por_tickcount: u64) -> Vec<Evento> {
        let mut pedidos = Vec::new();
        match self.fase {
            Fase::Antes => self.abrir(c, cfg, agora, ms_por_tickcount, &mut pedidos),
            Fase::Som => {}
            Fase::Laco => self.terminar_laco(c, agora),
            Fase::Final => self.fase = Fase::Fim,
            Fase::Fim => {}
        }
        pedidos
    }

    pub fn acabou(&self) -> bool {
        self.fase == Fase::Fim
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cfg() -> ConfigQuadrosAnimados {
        let v: serde_json::Value = serde_json::from_str(include_str!("../../../data/mensagens.json")).unwrap();
        serde_json::from_value(v["quadros_animados"].clone()).unwrap()
    }

    type Rodada = (Vec<(u64, u16)>, Vec<(u64, Evento)>, Option<u64>);

    fn rodar(tipo: TipoAnimado, ate: u64) -> Rodada {
        let cfg = cfg();
        let c = cfg.de(tipo).unwrap().clone();
        let (mut a, p) = Animado::novo(tipo, &c, &cfg, 1000, 55);
        let mut quadros = Vec::new();
        let mut pedidos: Vec<(u64, Evento)> = p.into_iter().map(|e| (1000, e)).collect();
        let mut fim = None;
        for t in 1000..=1000 + ate {
            let antes = a.quadro;
            let indice_antes = a.indice;
            let (p, _) = a.passar_tempo(&c, &cfg, t, 55);
            pedidos.extend(p.into_iter().map(|e| (t, e)));
            if let Some(q) = a.quadro.filter(|_| a.quadro != antes || a.indice != indice_antes) {
                quadros.push((t - 1000, q));
            }
            if a.acabou() && fim.is_none() {
                fim = Some(t - 1000);
            }
        }
        (quadros, pedidos, fim)
    }

    #[test]
    fn formiga_leao_espera_o_som_e_troca_de_quadro_a_cada_30_do_mac() {
        // Som 38 (703 ms = 13 unidades), depois um quadro a cada 10 unidades: 9201, 9202, 9203, 9202…
        // até 100 unidades de laço (300 do Mac): 9 quadros; som 37 nos quadros de índice 0 e 2.
        let (q, p, fim) = rodar(TipoAnimado::FormigaLeao, 400);
        assert_eq!(q.first(), Some(&(23, 9201)));
        assert_eq!(q.iter().map(|x| x.1).collect::<Vec<_>>(), vec![9201, 9202, 9203, 9202, 9203, 9202, 9203, 9202, 9203]);
        assert_eq!(fim, Some(113));
        let sons: Vec<u16> = p.iter().filter_map(|(_, e)| if let Evento::Som(s) = e { Some(*s) } else { None }).collect();
        assert_eq!(sons, vec![38, 37, 37, 37, 37, 37]);
    }

    #[test]
    fn aranha_cicla_quatro_quadros_com_som_a_cada_volta() {
        let (q, p, fim) = rodar(TipoAnimado::Aranha, 400);
        assert_eq!(&q.iter().map(|x| x.1).take(6).collect::<Vec<_>>(), &[12002, 12003, 12002, 12001, 12002, 12003]);
        assert_eq!(q[0].0, 3);
        assert_eq!(fim, Some(144));
        let sons = p.iter().filter(|(_, e)| *e == Evento::Som(45)).count();
        assert_eq!(sons, 1 + q.iter().filter(|x| x.1 == 12001).count());
    }

    #[test]
    fn vitoria_dura_o_dialog_wait_e_pede_as_duas_musicas() {
        // _DialogWaitInit(10) = 180 vence antes das 600 do Mac; os _myDelay voltam logo (bancada).
        let (q, p, fim) = rodar(TipoAnimado::Vitoria, 1000);
        assert_eq!(q[0], (3, 16211));
        assert_eq!(&q.iter().map(|x| x.1).take(4).collect::<Vec<_>>(), &[16211, 16212, 16210, 16211]);
        assert_eq!(fim, Some(180));
        let musicas: Vec<u16> = p.iter().filter_map(|(_, e)| if let Evento::Musica(m) = e { Some(*m) } else { None }).collect();
        assert_eq!(musicas, vec![20003, 20004]);
    }

    #[test]
    fn clique_encerra_a_espera_da_vez() {
        let cfg = cfg();
        let mut c = cfg.vitoria.clone().unwrap();
        c.espera_antes_tickcount = 150;
        c.espera_final_tickcount = 300;
        let (mut a, _) = Animado::novo(TipoAnimado::Vitoria, &c, &cfg, 0, 55);
        assert!(!a.janela_visivel());
        a.pular(&c, &cfg, 10, 55);
        assert!(a.janela_visivel());
        a.passar_tempo(&c, &cfg, 11, 55);
        assert_eq!(a.fase, Fase::Laco);
        a.pular(&c, &cfg, 12, 55);
        assert_eq!(a.fase, Fase::Final);
        a.pular(&c, &cfg, 13, 55);
        assert!(a.acabou());
    }
}
