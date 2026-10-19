//! Decisões do som sem SDL: opções Music/Sound Effects, foco da janela (`WM_ACTIVATEAPP`) e a linha
//! do tempo dos efeitos. `Audio` (mod.rs) só executa os comandos devolvidos aqui no dispositivo.
//! Especificação: docs/spec/regras/som.md ("Opções do jogador", "Janela inativa" e "Linha do tempo
//! dos efeitos").

use std::collections::VecDeque;

use engine::rules::estado::Evento;

/// O que fazer no dispositivo de som.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comando {
    TocarMusica(u16),
    PararMusica,
    TocarEfeito(u16),
    PararEfeito,
}

/// Estado das decisões. Tempos em milissegundos desde a abertura do som.
#[derive(Debug)]
pub struct ControleSom {
    /// `_OptionStates[1]` e `[2]` vistos por último (`_songsOnFlag`, `_effectsOnFlag`).
    musica_ligada: bool,
    efeitos_ligados: bool,
    /// A janela está ativa (tem o foco e não está minimizada).
    foco: bool,
    /// Música pedida ao abrir a janela antes de ela ganhar o foco pela primeira vez: toca quando o
    /// foco chegar (o SDL só entrega o foco alguns quadros depois de criar a janela).
    musica_inicial: Option<u16>,
    ja_teve_foco: bool,
    /// Instante a partir do qual o próximo efeito pode sair (fim das esperas já agendadas).
    cursor_ms: u64,
    /// Efeitos agendados, em ordem de instante.
    fila: VecDeque<(u64, u16)>,
    /// Instante em que começou (ou começará, se agendado) o último efeito aceito: um pedido no
    /// mesmo milissegundo é descartado (`_myBeginSound`, regras/som.md "Um efeito por vez").
    ultimo_inicio: Option<u64>,
}

impl Default for ControleSom {
    fn default() -> Self {
        ControleSom {
            musica_ligada: true,
            efeitos_ligados: true,
            foco: false,
            musica_inicial: None,
            ja_teve_foco: false,
            cursor_ms: 0,
            fila: VecDeque::new(),
            ultimo_inicio: None,
        }
    }
}

impl ControleSom {
    /// Música da abertura ou do jogo novo, pedida ao abrir a janela. Com a janela já ativa, toca;
    /// antes do primeiro foco, fica guardada até ele.
    pub fn musica_inicial(&mut self, id: u16) -> Vec<Comando> {
        if self.ja_teve_foco {
            self.musica(id)
        } else {
            self.musica_inicial = Some(id);
            Vec::new()
        }
    }

    /// `_myBeginSong`: só toca com a música ligada e a janela ativa.
    pub fn musica(&mut self, id: u16) -> Vec<Comando> {
        if self.musica_ligada && self.foco {
            vec![Comando::TocarMusica(id)]
        } else {
            Vec::new()
        }
    }

    /// Opções Music/Sound Effects da partida: desligar a música para a que toca; desligar os
    /// efeitos corta o que toca e esquece os agendados.
    pub fn opcoes(&mut self, musica: bool, efeitos: bool) -> Vec<Comando> {
        let mut c = Vec::new();
        if self.musica_ligada && !musica {
            c.push(Comando::PararMusica);
        }
        if self.efeitos_ligados && !efeitos {
            c.push(Comando::PararEfeito);
            self.fila.clear();
            self.ultimo_inicio = None;
        }
        self.musica_ligada = musica;
        self.efeitos_ligados = efeitos;
        c
    }

    /// `WM_ACTIVATEAPP` (`MAINWNDPROC`, `1000:2930`): ao perder o foco, `_StopSong`, efeito cortado e
    /// silêncio até voltar; ao voltar, nada recomeça (a não ser a música inicial ainda não tocada).
    pub fn foco(&mut self, ativo: bool, agora_ms: u64) -> Vec<Comando> {
        let mut c = Vec::new();
        if self.foco && !ativo {
            c.push(Comando::PararMusica);
            c.push(Comando::PararEfeito);
            self.fila.clear();
            self.ultimo_inicio = None;
            self.cursor_ms = agora_ms;
        }
        self.foco = ativo;
        if ativo && !self.ja_teve_foco {
            self.ja_teve_foco = true;
            if let Some(id) = self.musica_inicial.take() {
                c.extend(self.musica(id));
            }
        }
        c
    }

    /// Pedidos das regras de um quadro, na ordem. Os efeitos sem espera antes deles saem já (um
    /// corta o anterior, como no `waveOut` único do original), mas um efeito pedido no mesmo
    /// milissegundo em que outro começou é descartado (`_myBeginSound`: `GetTickCount() ≤` início do
    /// anterior + posição dele, ~0); `Evento::Espera` empurra os
    /// seguintes para mais tarde, e a fila do quadro seguinte começa depois das esperas
    /// agendadas (no original nada roda durante a animação). Devolve o que tocar agora.
    pub fn receber(&mut self, eventos: &[Evento], agora_ms: u64) -> Vec<Comando> {
        let mut c = Vec::new();
        let mut t = self.cursor_ms.max(agora_ms);
        for ev in eventos {
            match ev {
                Evento::Musica(id) => c.extend(self.musica(*id)),
                Evento::Espera(ms) => t += u64::from(*ms),
                Evento::Som(id) if self.efeitos_ligados && self.foco => {
                    if self.ultimo_inicio == Some(t) {
                        continue;
                    }
                    self.ultimo_inicio = Some(t);
                    if t <= agora_ms {
                        c.push(Comando::TocarEfeito(*id));
                    } else {
                        self.fila.push_back((t, *id));
                    }
                }
                _ => {}
            }
        }
        self.cursor_ms = t;
        c.extend(self.vencidos(agora_ms));
        c
    }

    /// Efeitos agendados cujo instante chegou.
    pub fn vencidos(&mut self, agora_ms: u64) -> Vec<Comando> {
        let mut c = Vec::new();
        while self.fila.front().is_some_and(|&(t, _)| t <= agora_ms) {
            if let Some((_, id)) = self.fila.pop_front() {
                c.push(Comando::TocarEfeito(id));
            }
        }
        c
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use Comando::*;

    fn ativo() -> ControleSom {
        let mut c = ControleSom::default();
        c.foco(true, 0);
        c
    }

    #[test]
    fn perder_o_foco_para_musica_e_efeito_e_silencia_ate_voltar() {
        let mut c = ativo();
        assert_eq!(c.musica(10001), vec![TocarMusica(10001)]);
        assert_eq!(c.foco(false, 100), vec![PararMusica, PararEfeito]);
        assert!(c.musica(10003).is_empty(), "sem foco, nenhuma música nova");
        assert!(c.receber(&[Evento::Som(37)], 200).is_empty(), "sem foco, nenhum efeito");
        assert!(c.foco(false, 300).is_empty(), "perder de novo não repete os comandos");
        // Ao voltar, a música interrompida não recomeça sozinha.
        assert!(c.foco(true, 400).is_empty());
        assert_eq!(c.receber(&[Evento::Som(37)], 500), vec![TocarEfeito(37)]);
    }

    #[test]
    fn perder_o_foco_esquece_os_efeitos_agendados() {
        let mut c = ativo();
        let ev = [Evento::Som(37), Evento::Espera(165), Evento::Som(37)];
        assert_eq!(c.receber(&ev, 0), vec![TocarEfeito(37)]);
        c.foco(false, 50);
        c.foco(true, 60);
        assert!(c.vencidos(1000).is_empty());
    }

    #[test]
    fn musica_inicial_espera_o_primeiro_foco() {
        let mut c = ControleSom::default();
        assert!(c.musica_inicial(10001).is_empty(), "janela recém-criada, ainda sem foco");
        assert_eq!(c.foco(true, 30), vec![TocarMusica(10001)]);
        // Depois do primeiro foco, perder e voltar não toca de novo.
        c.foco(false, 40);
        assert!(c.foco(true, 50).is_empty());
        // Com a janela ativa, a música inicial toca na hora.
        assert_eq!(c.musica_inicial(10001), vec![TocarMusica(10001)]);
    }

    #[test]
    fn opcoes_desligadas_param_e_bloqueiam() {
        let mut c = ativo();
        assert_eq!(c.opcoes(false, true), vec![PararMusica]);
        assert!(c.musica(10001).is_empty());
        assert_eq!(c.opcoes(false, false), vec![PararEfeito]);
        assert!(c.receber(&[Evento::Som(1)], 0).is_empty());
        assert!(c.opcoes(true, true).is_empty());
        assert_eq!(c.receber(&[Evento::Som(1)], 0), vec![TocarEfeito(1)]);
    }

    #[test]
    fn esperas_da_luta_espalham_os_efeitos_no_tempo() {
        // _AnimYellowFight: 6 quadros a 165 ms; som 0x25 nos quadros 0, 2 e 5; depois da animação,
        // no mesmo tick, a escavação (0x11).
        let mut c = ativo();
        const E: Evento = Evento::Espera(165);
        // A escavação é pedida no mesmo milissegundo do último 0x25: descartada (`_myBeginSound`).
        let ev = [Evento::Som(37), E, E, Evento::Som(37), E, E, E, Evento::Som(37), Evento::Som(17)];
        assert_eq!(c.receber(&ev, 1000), vec![TocarEfeito(37)]);
        assert!(c.vencidos(1329).is_empty());
        assert_eq!(c.vencidos(1330), vec![TocarEfeito(37)]);
        assert_eq!(c.vencidos(1824), vec![]);
        assert_eq!(c.vencidos(1825), vec![TocarEfeito(37)]);
        // Um tick seguinte (165 ms depois do primeiro) só sai depois da animação inteira.
        let mut c = ativo();
        c.receber(&ev, 1000);
        assert!(c.receber(&[Evento::Som(30)], 1165).is_empty());
        assert_eq!(c.vencidos(1824), vec![TocarEfeito(37)], "o do quadro 2");
        // O do tick seguinte começaria no mesmo instante do último da luta: descartado também.
        assert_eq!(c.vencidos(1825), vec![TocarEfeito(37)]);
        // Sem esperas pendentes, o efeito sai na hora.
        assert_eq!(c.receber(&[Evento::Som(30)], 5000), vec![TocarEfeito(30)]);
    }

    #[test]
    fn efeito_no_mesmo_milissegundo_e_descartado() {
        let mut c = ativo();
        // Dois pedidos no mesmo instante: só o primeiro toca (o original descarta o segundo).
        assert_eq!(c.receber(&[Evento::Som(19), Evento::Som(28)], 100), vec![TocarEfeito(19)]);
        assert!(c.receber(&[Evento::Som(28)], 100).is_empty(), "outro quadro, mesmo milissegundo");
        // Um milissegundo depois, o novo corta o anterior.
        assert_eq!(c.receber(&[Evento::Som(28)], 101), vec![TocarEfeito(28)]);
        // Depois de parar (opção desligada), nada a descartar.
        c.opcoes(true, false);
        c.opcoes(true, true);
        assert_eq!(c.receber(&[Evento::Som(5)], 101), vec![TocarEfeito(5)]);
    }
}
