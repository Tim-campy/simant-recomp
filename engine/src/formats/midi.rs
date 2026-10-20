//! MIDI padrão (SMF formatos 0 e 1): `SOUND\*.MID` e objetos tipos 20/50 do banco.
//! Especificação: docs/spec/formatos/som.md. Só o necessário para tocar: mensagens de canal e
//! andamento; sysex e os demais metaeventos são lidos e descartados.

/// Mensagem de canal com o instante em ticks da trilha.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Bruto {
    tick: u64,
    /// Ordem de leitura (desempate estável entre trilhas).
    ordem: usize,
    conteudo: Conteudo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Conteudo {
    Canal { status: u8, d1: u8, d2: u8 },
    /// Microssegundos por semínima (`FF 51`).
    Andamento(u32),
}

/// Um arquivo MIDI lido.
#[derive(Clone, Debug)]
pub struct Midi {
    pub formato: u16,
    pub trilhas: u16,
    /// Divisão do cabeçalho (ticks por semínima, ou SMPTE com o bit 15).
    pub divisao: u16,
    eventos: Vec<Bruto>,
}

/// Mensagem de canal pronta para o sintetizador, no tempo absoluto.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventoMidi {
    pub segundos: f64,
    /// Status com o canal (0x80–0xEF); canal 0 = canal MIDI 1.
    pub status: u8,
    pub d1: u8,
    pub d2: u8,
}

impl EventoMidi {
    /// Canal MIDI de 1 a 16.
    pub fn canal(&self) -> u8 {
        (self.status & 0x0F) + 1
    }
    pub fn comando(&self) -> u8 {
        self.status & 0xF0
    }
}

struct Leitor<'a> {
    b: &'a [u8],
    i: usize,
}

impl Leitor<'_> {
    fn byte(&mut self) -> Result<u8, String> {
        let v = *self.b.get(self.i).ok_or_else(|| format!("MIDI truncated at 0x{:x}", self.i))?;
        self.i += 1;
        Ok(v)
    }
    fn vlq(&mut self) -> Result<u32, String> {
        let mut v: u32 = 0;
        for _ in 0..4 {
            let c = self.byte()?;
            v = (v << 7) | (c & 0x7F) as u32;
            if c & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err(format!("variable-length quantity too long at 0x{:x}", self.i))
    }
    fn pular(&mut self, n: usize) -> Result<(), String> {
        if self.i + n > self.b.len() {
            return Err(format!("MIDI truncated at 0x{:x}", self.i));
        }
        self.i += n;
        Ok(())
    }
}

fn be16(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}

fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

impl Midi {
    pub fn de_bytes(b: &[u8]) -> Result<Midi, String> {
        if b.len() < 14 || &b[0..4] != b"MThd" {
            return Err("not MIDI (missing MThd)".into());
        }
        let tam = be32(b, 4) as usize;
        if tam < 6 || 8 + tam > b.len() {
            return Err("invalid MThd header".into());
        }
        let formato = be16(b, 8);
        let trilhas = be16(b, 10);
        let divisao = be16(b, 12);
        if divisao == 0 {
            return Err("division 0 in the MIDI header".into());
        }
        let mut eventos = Vec::new();
        let mut pos = 8 + tam;
        for t in 0..trilhas {
            if pos + 8 > b.len() || &b[pos..pos + 4] != b"MTrk" {
                return Err(format!("track {t}: missing MTrk at 0x{pos:x}"));
            }
            let fim = pos + 8 + be32(b, pos + 4) as usize;
            if fim > b.len() {
                return Err(format!("track {t} runs past the end of the file"));
            }
            ler_trilha(&b[pos + 8..fim], &mut eventos)?;
            pos = fim;
        }
        // Formato 1: trilhas simultâneas. A ordenação é estável (tick, ordem de leitura).
        eventos.sort_by_key(|e| (e.tick, e.ordem));
        Ok(Midi { formato, trilhas, divisao, eventos })
    }

    /// Mensagens de canal no tempo absoluto, só dos `canais` (1–16), com as notas transpostas em
    /// `transposicao` semitons (exceto no canal `percussao`, que não tem altura).
    pub fn sequencia(&self, canais: &[u8], transposicao: i16, percussao: u8) -> Vec<EventoMidi> {
        let mut saida = Vec::new();
        // Andamento padrão: 120 semínimas por minuto.
        let mut us_por_seminima: f64 = 500_000.0;
        let (mut tick_ant, mut seg_ant) = (0u64, 0f64);
        let seg_por_tick_smpte = if self.divisao & 0x8000 != 0 {
            let fps = -((self.divisao >> 8) as i8) as f64;
            let por_quadro = (self.divisao & 0xFF) as f64;
            Some(1.0 / (fps * por_quadro).max(1.0))
        } else {
            None
        };
        for e in &self.eventos {
            let dt = (e.tick - tick_ant) as f64;
            let seg = seg_ant
                + match seg_por_tick_smpte {
                    Some(s) => dt * s,
                    None => dt * us_por_seminima / 1e6 / self.divisao as f64,
                };
            tick_ant = e.tick;
            seg_ant = seg;
            match e.conteudo {
                Conteudo::Andamento(us) => us_por_seminima = us.max(1) as f64,
                Conteudo::Canal { status, d1, mut d2 } => {
                    let canal = (status & 0x0F) + 1;
                    if !canais.contains(&canal) {
                        continue;
                    }
                    let cmd = status & 0xF0;
                    let mut nota = d1;
                    if (cmd == 0x80 || cmd == 0x90) && canal != percussao && transposicao != 0 {
                        nota = (d1 as i16 + transposicao).clamp(0, 127) as u8;
                    }
                    if cmd == 0x80 {
                        d2 = 0;
                    }
                    // Nota com velocidade 0 = desligar; normaliza para 0x80.
                    let status = if cmd == 0x90 && d2 == 0 { 0x80 | (status & 0x0F) } else { status };
                    saida.push(EventoMidi { segundos: seg, status, d1: nota, d2 });
                }
            }
        }
        saida
    }

    /// Número de mensagens de canal (todas as trilhas e canais).
    pub fn mensagens_de_canal(&self) -> usize {
        self.eventos.iter().filter(|e| matches!(e.conteudo, Conteudo::Canal { .. })).count()
    }
}

fn ler_trilha(b: &[u8], eventos: &mut Vec<Bruto>) -> Result<(), String> {
    let mut l = Leitor { b, i: 0 };
    let mut tick: u64 = 0;
    let mut corrente: u8 = 0;
    while l.i < b.len() {
        tick += l.vlq()? as u64;
        let mut status = l.byte()?;
        let ordem = eventos.len();
        match status {
            0xFF => {
                let tipo = l.byte()?;
                let n = l.vlq()? as usize;
                let ini = l.i;
                l.pular(n)?;
                if tipo == 0x51 && n == 3 {
                    let us = (b[ini] as u32) << 16 | (b[ini + 1] as u32) << 8 | b[ini + 2] as u32;
                    eventos.push(Bruto { tick, ordem, conteudo: Conteudo::Andamento(us) });
                }
                if tipo == 0x2F {
                    break;
                }
            }
            0xF0 | 0xF7 => {
                let n = l.vlq()? as usize;
                l.pular(n)?;
            }
            _ => {
                // Status corrente: um byte de dados no lugar do status repete o anterior.
                let d1 = if status & 0x80 == 0 {
                    if corrente == 0 {
                        return Err(format!("data byte without status at 0x{:x}", l.i - 1));
                    }
                    let d = status;
                    status = corrente;
                    d
                } else {
                    if status >= 0xF0 {
                        return Err(format!("status 0x{status:02X} not supported at 0x{:x}", l.i - 1));
                    }
                    corrente = status;
                    l.byte()?
                };
                let d2 = if matches!(status & 0xF0, 0xC0 | 0xD0) { 0 } else { l.byte()? };
                eventos.push(Bruto { tick, ordem, conteudo: Conteudo::Canal { status, d1, d2 } });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    /// MIDI formato 1, 240 ticks/semínima, andamento de 0,25 s por semínima, duas trilhas.
    fn exemplo() -> Vec<u8> {
        let mut b = b"MThd\0\0\0\x06\0\x01\0\x02\0\xF0".to_vec();
        let t1: &[u8] = &[0x00, 0xFF, 0x51, 0x03, 0x03, 0xD0, 0x90, 0x00, 0xFF, 0x2F, 0x00];
        // canal 3: nota 60 em 0, status corrente em 240 (vel 0 = desliga); canal 13 igual.
        let t2: &[u8] = &[
            0x00, 0x92, 60, 100, 0x81, 0x70, 60, 0, 0x00, 0x9C, 60, 100, 0x00, 0xCC, 5, 0x00, 0xFF, 0x2F, 0x00,
        ];
        for t in [t1, t2] {
            b.extend_from_slice(b"MTrk");
            b.extend_from_slice(&(t.len() as u32).to_be_bytes());
            b.extend_from_slice(t);
        }
        b
    }

    #[test]
    fn le_andamento_status_corrente_e_filtra_canais() {
        let m = Midi::de_bytes(&exemplo()).unwrap();
        assert_eq!((m.formato, m.trilhas, m.divisao), (1, 2, 240));
        assert_eq!(m.mensagens_de_canal(), 4);
        let s = m.sequencia(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 0, 10);
        assert_eq!(s.len(), 2, "o canal 13 fica de fora");
        assert_eq!((s[0].segundos, s[0].status, s[0].d1, s[0].d2), (0.0, 0x92, 60, 100));
        assert!((s[1].segundos - 0.25).abs() < 1e-9);
        assert_eq!((s[1].status, s[1].canal(), s[1].comando()), (0x82, 3, 0x80));
    }

    #[test]
    fn transpoe_notas_menos_na_percussao() {
        let m = Midi::de_bytes(&exemplo()).unwrap();
        let s = m.sequencia(&[3, 10], 12, 3);
        assert_eq!(s[0].d1, 60, "canal 3 marcado como percussão");
        let s = m.sequencia(&[3], 12, 10);
        assert_eq!(s[0].d1, 72);
        assert_eq!(s[1].d1, 72);
    }

    #[test]
    fn rejeita_arquivo_sem_cabecalho() {
        assert!(Midi::de_bytes(b"RIFF....").is_err());
        let mut b = exemplo();
        b.truncate(30);
        assert!(Midi::de_bytes(&b).is_err());
    }
}
