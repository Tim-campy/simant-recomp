//! Conselhos e tutorial (`_Feedback`, `1030:78ca`; `_GiveLesson` `1030:7b4e`; `_LessonDone` `1030:7f02`).
//! Especificação: docs/spec/regras/conselhos.md.
//!
//! - A mensagem de conselho da faixa é **estado derivado** ([`conselho`]): o original a refaz em
//!   todo tick, então ela não vira evento; a interface a consulta depois de cada tick.
//! - Os quadros das lições do tutorial saem como `Evento::Mensagem(texto)`.
//! - Tabelas (lista 4/1020, lições) no bloco `conselhos` de `data/quintal.json`.

use serde::Deserialize;

use super::estado::Estado;
use super::quintal::{Relogios, TabelasQuintal};
use super::tick::{RegrasConselhos, RegrasOriginais};
use crate::formats::save::{Formiga, Lista};

/// O que a lição faz ao ser dada (conselhos.md §3, "Ações").
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum AoDar {
    Nada,
    /// `_TimeTemp = _TilesDugB + soma`.
    MarcoEscavacao { soma: i64 },
    /// `_LessonTemp = _MeLocY + _MeLocX`; `_TimeTemp = relógio + espera`.
    MarcarPosicaoEEsperar { espera: i64 },
    /// `_LessonTemp = _MapPnt.y + _MapPnt.x`; `_TimeTemp = relógio + espera`.
    MarcarVistaEEsperar { espera: i64 },
    /// `_LessonTemp = _MeLocY + _MeLocX`.
    MarcarPosicao,
    /// `_LessonTemp = _modeLevels[1] + _modeLevels[0]`.
    MarcarTarefas,
    /// `_LessonTemp = 0`.
    ZerarTemp,
    /// `_SetMyHealth(valor)`.
    SaudeAmarela { valor: i16 },
    /// `_AddBlackAnts(n)` antes do quadro.
    FormigasPretas { n: i32 },
    /// Quadro, `_AddBlackAnts(n)` e `_PherMapA` zerado.
    FormigasPretasELimparAlarme { n: i32 },
    /// `_TutLesson += 1` (mostra o texto da lição seguinte).
    PularLicao,
}

/// Quando a lição termina (conselhos.md §3, "Condições").
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Concluida {
    Sempre,
    Nunca,
    EscavouAlemDoMarco,
    NoPlano { plano: i16 },
    AndouEEsperou,
    VistaMudouOuEsperou,
    JanelaNaFrente { janela: u16 },
    SuperficieAcimaDe { tile: u8 },
    SaudeAcimaDe { valor: i16 },
    TipoAmarela { tipo_formiga: i16 },
    DefesaAcimaDe { valor: i16 },
    AndouSoltandoAlarme,
    TempMarcadoDesligaAlarme,
    TempMarcado,
    PlanoDoMapa { plano: i16 },
    ModoQuintal { modo: i16 },
    TarefasMudaram,
    EntrouNumNinho,
}

/// Uma lição do tutorial.
#[derive(Clone, Debug, Deserialize)]
pub struct Licao {
    pub n: i16,
    /// Texto SHARED 4 do quadro.
    pub texto: u16,
    /// Texto usado em vez de `texto` quando `_LessonTemp ≠ 0` (lição 0x37).
    #[serde(default)]
    pub texto_se_temp: Option<u16>,
    /// Imagem do quadro (0 = nenhuma) [em aberto: recurso].
    pub imagem: u16,
    pub ao_dar: AoDar,
    pub concluida: Concluida,
}

/// Bloco `conselhos` de `data/quintal.json`.
#[derive(Clone, Debug, Deserialize)]
pub struct TabelasConselhos {
    /// Lista SHARED 4 com os textos dos conselhos (1020).
    pub lista: u16,
    /// Vida abaixo da qual o conselho é "crítico" (10).
    pub saude_critica: i16,
    /// `_TutHowLong` mínimo para repetir uma lição não concluída (16).
    pub licao_minima_ticks: i16,
    pub licoes: Vec<Licao>,
}

impl TabelasConselhos {
    fn licao(&self, n: i16) -> Option<&Licao> {
        self.licoes.iter().find(|l| l.n == n)
    }
}

/// A mensagem da faixa pedida por `_Feedback` (conselhos.md §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conselho {
    /// Opção de conselhos desligada (`_OptionStates[4] == 0`): nada.
    Nenhum,
    /// `_SetDefaultWindPrompt(0)`: texto padrão da janela, conforme a ferramenta.
    PadraoDaJanela,
    /// Entrada `indice` da lista SHARED 4/`lista`; `importante` = mostrar mesmo com outra mensagem.
    Texto { lista: u16, indice: u8, importante: bool },
}

/// `_TimeTemp` (`9:9b10`, fora do save): prazo no relógio do Mac ou marco de escavação.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemoriaTutor {
    pub tempo_limite: i64,
}

/// O que o tutorial precisa saber da interface.
pub trait InterfaceTutor {
    /// `_win_IsWinInFront(janela)`.
    fn janela_na_frente(&self, janela: u16) -> bool;
}

/// Sem interface: nenhuma janela na frente.
pub struct SemInterface;

impl InterfaceTutor for SemInterface {
    fn janela_na_frente(&self, _janela: u16) -> bool {
        false
    }
}

impl RegrasConselhos for RegrasOriginais {
    fn dar_conselhos(&mut self, e: &mut Estado) {
        // TODO(integração): guardar `MemoriaTutor` e a interface em `RegrasOriginais` (conselhos.md, "Lacunas").
        let t = TabelasQuintal::padrao();
        let rel = Relogios::do_port(e, t);
        executar_tutor(e, &t.conselhos, &mut MemoriaTutor::default(), &SemInterface, rel.mac);
    }
}

/// Parte de conselho de `_Feedback`.
pub fn conselho(e: &Estado, t: &TabelasConselhos) -> Conselho {
    if e.partida.opcoes[4] == 0 {
        return Conselho::Nenhum;
    }
    let texto = |indice, importante| Conselho::Texto { lista: t.lista, indice, importante };
    let a = &e.amarela;
    if a.modo == 0 && (a.saude < a.aviso_saude || a.saude < t.saude_critica) {
        return if a.saude < t.saude_critica { texto(0, true) } else { texto(1, false) };
    }
    if e.preta.saude < e.partida.aviso_saude_colonia {
        return texto(2, false);
    }
    let dug = e.preta.tiles_escavados;
    if e.preta.populacao <= dug {
        return Conselho::PadraoDaJanela;
    }
    if a.modo == 0 && a.ninho_iniciado == 0 && dug == 0 {
        texto(3, false)
    } else {
        texto(4, false)
    }
}

/// Parte do tutorial de `_Feedback` (`_RunTutor`), com memória, interface e relógio do Mac explícitos.
pub fn executar_tutor(
    e: &mut Estado,
    t: &TabelasConselhos,
    mem: &mut MemoriaTutor,
    ui: &dyn InterfaceTutor,
    relogio: i64,
) {
    if e.partida.tipo_jogo != 0 {
        return;
    }
    let tu = &mut e.tutorial;
    tu.contagem += 1;
    if tu.contagem > 0x400 {
        tu.contagem = 0;
    }
    if tu.contagem & 7 != 0 {
        return;
    }
    tu.duracao += 1;
    if tu.dada == 0 {
        let n = tu.licao;
        dar_licao(e, t, mem, relogio, n);
        e.tutorial.dada = 1;
        e.tutorial.duracao = 0;
    } else if licao_concluida(e, t, mem, ui, relogio, e.tutorial.licao) {
        e.tutorial.licao += 1;
        e.tutorial.dada = 0;
    } else if e.tutorial.duracao >= t.licao_minima_ticks && relogio > mem.tempo_limite {
        e.tutorial.dada = 0;
    }
}

/// `_GiveLesson(n)`.
pub fn dar_licao(e: &mut Estado, t: &TabelasConselhos, mem: &mut MemoriaTutor, relogio: i64, n: i16) {
    let Some(l) = t.licao(n) else { return };
    let soma_posicao = e.amarela.y as i32 + e.amarela.x as i32;
    match l.ao_dar {
        AoDar::Nada => {}
        AoDar::MarcoEscavacao { soma } => mem.tempo_limite = e.preta.tiles_escavados as i64 + soma,
        AoDar::MarcarPosicaoEEsperar { espera } => {
            e.tutorial.temporario = soma_posicao as i16;
            mem.tempo_limite = relogio + espera;
        }
        AoDar::MarcarVistaEEsperar { espera } => {
            e.tutorial.temporario = (e.vistas.ponto.1 as i32 + e.vistas.ponto.0 as i32) as i16;
            mem.tempo_limite = relogio + espera;
        }
        AoDar::MarcarPosicao => e.tutorial.temporario = soma_posicao as i16,
        AoDar::MarcarTarefas => {
            let m = &e.controles.tarefas;
            e.tutorial.temporario = m[1].wrapping_add(m[0]) as i16;
        }
        AoDar::ZerarTemp => e.tutorial.temporario = 0,
        AoDar::SaudeAmarela { valor } => definir_saude_amarela(e, valor),
        AoDar::FormigasPretas { n } => adicionar_formigas_pretas(e, n),
        AoDar::FormigasPretasELimparAlarme { .. } => {}
        AoDar::PularLicao => e.tutorial.licao += 1,
    }
    let texto = match l.texto_se_temp {
        Some(alt) if e.tutorial.temporario != 0 => alt,
        _ => l.texto,
    };
    e.eventos.push(super::estado::Evento::Mensagem(texto));
    if let AoDar::FormigasPretasELimparAlarme { n } = l.ao_dar {
        adicionar_formigas_pretas(e, n);
        e.feromonios.alarme.dados.fill(0);
    }
}

/// `_LessonDone(n)`.
pub fn licao_concluida(
    e: &mut Estado,
    t: &TabelasConselhos,
    mem: &MemoriaTutor,
    ui: &dyn InterfaceTutor,
    relogio: i64,
    n: i16,
) -> bool {
    let Some(l) = t.licao(n) else { return false };
    let a = &e.amarela;
    let posicao_mudou = a.y as i32 + a.x as i32 != e.tutorial.temporario as i32;
    match l.concluida {
        Concluida::Sempre => true,
        Concluida::Nunca => false,
        Concluida::EscavouAlemDoMarco => e.preta.tiles_escavados as i64 > mem.tempo_limite && a.mover == 0,
        Concluida::NoPlano { plano } => a.plano == plano,
        Concluida::AndouEEsperou => posicao_mudou && a.dis == 0 && relogio > mem.tempo_limite,
        Concluida::VistaMudouOuEsperou => {
            let v = e.vistas.ponto.1 as i32 + e.vistas.ponto.0 as i32;
            v != e.tutorial.temporario as i32 || relogio > mem.tempo_limite
        }
        Concluida::JanelaNaFrente { janela } => ui.janela_na_frente(janela),
        Concluida::SuperficieAcimaDe { tile } => {
            a.plano == 1 && e.superficie.mapa.em(a.x as i32, a.y as i32).unwrap_or(0) > tile
        }
        Concluida::SaudeAcimaDe { valor } => a.saude > valor,
        Concluida::TipoAmarela { tipo_formiga } => a.tipo == tipo_formiga,
        Concluida::DefesaAcimaDe { valor } => e.preta.modos[5] > valor,
        Concluida::AndouSoltandoAlarme => posicao_mudou && a.soltando_alarme != 0 && a.dis == 0,
        Concluida::TempMarcadoDesligaAlarme => {
            if a.soltando_alarme != 0 {
                e.amarela.soltando_alarme = 0; // `_SetAlarmDropState(0, 1)`
            }
            e.tutorial.temporario != 0
        }
        Concluida::TempMarcado => e.tutorial.temporario != 0,
        Concluida::PlanoDoMapa { plano } => e.vistas.plano == plano,
        Concluida::ModoQuintal { modo } => e.vistas.modo_quintal == modo,
        Concluida::TarefasMudaram => {
            let m = &e.controles.tarefas;
            m[1].wrapping_add(m[0]) as i16 != e.tutorial.temporario
        }
        Concluida::EntrouNumNinho => match a.plano {
            3 => {
                e.tutorial.temporario = 0;
                true
            }
            2 => {
                e.tutorial.temporario = 1;
                true
            }
            _ => false,
        },
    }
}

/// `_SetMyHealth(v)` (formiga-amarela.md, `1020:8c70`).
fn definir_saude_amarela(e: &mut Estado, v: i16) {
    let v = if e.partida.nunca_com_fome != 0 { 100 } else { v };
    let a = &mut e.amarela;
    if v > 0 {
        a.saude_contagem = 0;
    }
    a.saude = v.clamp(0, 100);
    a.quer_comida = if a.saude > a.aviso_saude && a.saude > 9 { 0 } else { 1 };
}

/// `_AddBlackAnts(n)` (`1030:6c5a`, geracao-mundo.md): pretas novas na superfície, x de 0 a 63, y de 16 a 47.
fn adicionar_formigas_pretas(e: &mut Estado, mut n: i32) {
    for x in 0..64 {
        for y in 16..48 {
            let livre = e.superficie.mapa.em(x, y).unwrap_or(0xFF) < 0x50 && e.superficie.vida.em(x, y) == Some(0);
            if !livre {
                continue;
            }
            let (base, modo) = if (0..=3).contains(&e.s.rand1(10)) { (0x30, 2) } else { (0x10, 4) };
            let tipo = (e.s.mascara(8) + base) as u8;
            e.adicionar_formiga(Lista::A, Formiga { x: x as u8, y: y as u8, modo, tipo, s: 0 });
            n -= 1;
            if n < 1 || e.superficie.lista.indice > 999 {
                return;
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn t() -> &'static TabelasConselhos {
        &TabelasQuintal::padrao().conselhos
    }

    fn base() -> Estado {
        let mut e = Estado::vazio();
        e.partida.opcoes[4] = 1;
        e.amarela.saude = 80;
        e.amarela.aviso_saude = 30;
        e.preta.saude = 80;
        e.partida.aviso_saude_colonia = 30;
        e.preta.populacao = 5;
        e
    }

    fn texto(i: u8, imp: bool) -> Conselho {
        Conselho::Texto { lista: 1020, indice: i, importante: imp }
    }

    #[test]
    fn conselhos_da_faixa() {
        let mut e = base();
        e.amarela.saude = 5;
        assert_eq!(conselho(&e, t()), texto(0, true));
        e.amarela.saude = 20;
        assert_eq!(conselho(&e, t()), texto(1, false));
        e.amarela.saude = 80;
        e.preta.saude = 10;
        assert_eq!(conselho(&e, t()), texto(2, false));
        e.preta.saude = 80;
        e.preta.tiles_escavados = 10;
        assert_eq!(conselho(&e, t()), Conselho::PadraoDaJanela);
        e.preta.tiles_escavados = 0;
        assert_eq!(conselho(&e, t()), texto(3, false));
        e.amarela.ninho_iniciado = 1;
        assert_eq!(conselho(&e, t()), texto(4, false));
        e.partida.opcoes[4] = 0;
        assert_eq!(conselho(&e, t()), Conselho::Nenhum);
        // Amarela como aranha (`_MeMode` = 1): a vida dela não conta.
        let mut e = base();
        e.amarela.modo = 1;
        e.amarela.saude = 5;
        e.amarela.ninho_iniciado = 1;
        assert_eq!(conselho(&e, t()), texto(4, false));
    }

    #[test]
    fn tutor_da_e_avanca_licoes() {
        let mut e = Estado::vazio();
        e.tutorial.licao = 1;
        e.tutorial.contagem = 7;
        let mut m = MemoriaTutor::default();
        executar_tutor(&mut e, t(), &mut m, &SemInterface, 0);
        assert_eq!(e.retirar_eventos(), vec![crate::rules::estado::Evento::Mensagem(11000)]);
        assert_eq!((e.tutorial.dada, e.tutorial.duracao), (1, 0));
        for _ in 0..8 {
            executar_tutor(&mut e, t(), &mut m, &SemInterface, 0);
        }
        assert_eq!((e.tutorial.licao, e.tutorial.dada), (2, 0));
        for _ in 0..8 {
            executar_tutor(&mut e, t(), &mut m, &SemInterface, 0);
        }
        assert_eq!(e.retirar_eventos(), vec![crate::rules::estado::Evento::Mensagem(11002)]);
    }

    #[test]
    fn licao_5_espera_a_superficie_e_0x2b_pula() {
        let mut e = Estado::vazio();
        let m = MemoriaTutor::default();
        e.amarela.plano = 2;
        assert!(!licao_concluida(&mut e, t(), &m, &SemInterface, 0, 5));
        e.amarela.plano = 1;
        assert!(licao_concluida(&mut e, t(), &m, &SemInterface, 0, 5));
        e.tutorial.licao = 0x2B;
        dar_licao(&mut e, t(), &mut MemoriaTutor::default(), 0, 0x2B);
        assert_eq!(e.tutorial.licao, 0x2C);
        assert_eq!(e.retirar_eventos(), vec![crate::rules::estado::Evento::Mensagem(11086)]);
    }

    #[test]
    fn tutor_so_no_tutorial() {
        let mut e = Estado::vazio();
        e.partida.tipo_jogo = 1;
        e.tutorial.contagem = 7;
        executar_tutor(&mut e, t(), &mut MemoriaTutor::default(), &SemInterface, 0);
        assert_eq!(e.tutorial.contagem, 7);
    }
}
