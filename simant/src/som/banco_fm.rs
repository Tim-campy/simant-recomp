//! Banco de timbres do sintetizador FM (`fm.rs`): um timbre de dois operadores por programa General
//! MIDI e um por nota de percussão. Formatos aceitos:
//!
//! - JSON do port (`data/opl_banco.json`, banco próprio e livre, gerado por
//!   `tools/gerar_banco_opl.py`; sobrescrevível por mod);
//! - `.op2` (formato "GENMIDI" do DMX, 175 instrumentos), lido em tempo de execução de um arquivo
//!   que o usuário indicar. O port não traz nenhum `.op2`.
//!
//! Ver docs/spec/formatos/som.md, "Sintetizador FM", e docs/decisoes.md.

use std::collections::BTreeMap;

use serde::Deserialize;

/// Parâmetros de um operador, com os mesmos campos e faixas dos registradores do OPL
/// (0x20: AM, VIB, EG-TYP, KSR, MULT; 0x40: KSL, TL; 0x60: AR, DR; 0x80: SL, RR; 0xE0: WS).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Operador {
    /// Tremolo (variação de volume do LFO).
    #[serde(default)]
    pub am: bool,
    /// Vibrato (variação de altura do LFO).
    #[serde(default)]
    pub vib: bool,
    /// EG-TYP: segura o nível de sustentação até soltar a tecla; falso = decai direto.
    #[serde(default)]
    pub sustenta: bool,
    /// KSR: envelope mais rápido nas notas agudas.
    #[serde(default)]
    pub ksr: bool,
    /// Multiplicador de frequência (0–15; 0 = ½).
    pub mult: u8,
    /// Atenuação por altura (0–3, na codificação do OPL: 0 nada, 1 = 3 dB/oitava, 2 = 1,5, 3 = 6).
    #[serde(default)]
    pub ksl: u8,
    /// Atenuação total (0–63, passos de 0,75 dB).
    pub tl: u8,
    /// Taxas de ataque, decaimento e soltura (0–15) e nível de sustentação (0–15, passos de 3 dB).
    pub ar: u8,
    pub dr: u8,
    pub sl: u8,
    pub rr: u8,
    /// Forma de onda (0–7; 4–7 só no OPL3).
    #[serde(default)]
    pub onda: u8,
}

impl Operador {
    /// A partir dos bytes dos registradores 0x20, 0x40, 0x60, 0x80 e 0xE0.
    pub fn de_registros(r20: u8, r40: u8, r60: u8, r80: u8, re0: u8) -> Operador {
        Operador {
            am: r20 & 0x80 != 0,
            vib: r20 & 0x40 != 0,
            sustenta: r20 & 0x20 != 0,
            ksr: r20 & 0x10 != 0,
            mult: r20 & 0x0F,
            ksl: r40 >> 6,
            tl: r40 & 0x3F,
            ar: r60 >> 4,
            dr: r60 & 0x0F,
            sl: r80 >> 4,
            rr: r80 & 0x0F,
            onda: re0 & 0x07,
        }
    }

    pub fn validar(&self) -> Result<(), String> {
        let faixas = [(self.mult, 15), (self.ksl, 3), (self.tl, 63), (self.ar, 15), (self.dr, 15), (self.sl, 15), (self.rr, 15), (self.onda, 7)];
        if faixas.iter().any(|&(v, max)| v > max) {
            return Err(format!("operator outside the OPL ranges: {self:?}"));
        }
        Ok(())
    }
}

/// Um timbre de dois operadores (um canal do OPL em modo de 2 operadores).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Timbre {
    #[serde(default)]
    pub nome: String,
    pub modulador: Operador,
    pub portadora: Operador,
    /// Realimentação do modulador (0–7).
    #[serde(default)]
    pub realimentacao: u8,
    /// Conexão: falso = FM (o modulador modula a portadora), verdadeiro = aditiva (os dois soam).
    #[serde(default)]
    pub aditivo: bool,
    /// Semitons somados à nota.
    #[serde(default)]
    pub transposicao: i8,
    /// Percussão: toca sempre esta nota, qualquer que seja a recebida.
    #[serde(default)]
    pub nota_fixa: Option<u8>,
}

/// Banco completo.
#[derive(Clone, Debug, Deserialize)]
pub struct Banco {
    /// Profundidade global do LFO (registrador 0xBD do OPL): tremolo de 4,8 dB (senão 1 dB) e
    /// vibrato de 14 cents (senão 7).
    #[serde(default = "verdadeiro")]
    pub tremolo_profundo: bool,
    #[serde(default = "verdadeiro")]
    pub vibrato_profundo: bool,
    /// Curva de volume: atenuação = −curva × log10(velocidade × volume × expressão), em dB.
    #[serde(default = "quarenta")]
    pub curva_volume_db: f32,
    /// Ganho da saída (1,0 = um canal do OPL em volume máximo dá 1/8 da escala, como no chip).
    #[serde(default = "um")]
    pub ganho: f32,
    /// 128 timbres, um por programa General MIDI (0–127).
    pub melodicos: Vec<Timbre>,
    /// Nota do canal de percussão (decimal) → timbre com `nota_fixa`. Nota ausente = silêncio.
    #[serde(default)]
    pub percussao: BTreeMap<u8, Timbre>,
}

fn verdadeiro() -> bool {
    true
}
fn quarenta() -> f32 {
    40.0
}
fn um() -> f32 {
    1.0
}

impl Banco {
    /// Lê o JSON do port e confere as faixas (o executável lê pelo `Recursos::dados`, com mods).
    #[cfg(test)]
    pub fn de_json(bytes: &[u8]) -> Result<Banco, String> {
        let b: Banco = serde_json::from_slice(bytes).map_err(|e| format!("FM bank: {e}"))?;
        b.validar()?;
        Ok(b)
    }

    pub fn validar(&self) -> Result<(), String> {
        if self.melodicos.len() != 128 {
            return Err(format!("FM bank: {} melodic instruments (needs 128)", self.melodicos.len()));
        }
        for t in self.melodicos.iter().chain(self.percussao.values()) {
            t.modulador.validar()?;
            t.portadora.validar()?;
            if t.realimentacao > 7 {
                return Err(format!("FM bank: feedback {} in '{}'", t.realimentacao, t.nome));
            }
        }
        Ok(())
    }

    /// Lê um banco no formato `.op2` do DMX ("#OPL_II#", 175 instrumentos de 36 bytes e 175
    /// nomes de 32 bytes): 128 melódicos e as notas de percussão 35–81. Só a primeira voz de cada
    /// instrumento é usada (o port toca um canal do OPL por nota).
    pub fn de_op2(bytes: &[u8]) -> Result<Banco, String> {
        const CAB: &[u8] = b"#OPL_II#";
        const N: usize = 175;
        if bytes.len() < CAB.len() + N * 36 || &bytes[..CAB.len()] != CAB {
            return Err(".op2 bank: #OPL_II# header missing or file too short".into());
        }
        let nome = |i: usize| -> String {
            let o = CAB.len() + N * 36 + i * 32;
            bytes.get(o..o + 32).map(|n| String::from_utf8_lossy(n).trim_end_matches('\0').to_string()).unwrap_or_default()
        };
        let timbre = |i: usize| -> Timbre {
            let d = &bytes[CAB.len() + i * 36..CAB.len() + (i + 1) * 36];
            let flags = u16::from_le_bytes([d[0], d[1]]);
            let fixa = flags & 1 != 0;
            let v = &d[4..20];
            // Voz: modulador (0x20, 0x60, 0x80, 0xE0, KSL, nível), 0xC0, portadora (idem), livre, nota (i16).
            let op = |o: &[u8]| Operador::de_registros(o[0], (o[4] & 0xC0) | (o[5] & 0x3F), o[1], o[2], o[3]);
            let deslocamento = i16::from_le_bytes([v[14], v[15]]);
            Timbre {
                nome: nome(i),
                modulador: op(&v[0..6]),
                portadora: op(&v[7..13]),
                realimentacao: (v[6] >> 1) & 7,
                aditivo: v[6] & 1 != 0,
                transposicao: deslocamento.clamp(-48, 48) as i8,
                nota_fixa: fixa.then_some(d[3]),
            }
        };
        let melodicos = (0..128).map(timbre).collect();
        let percussao = (0..47).map(|k| (35 + k as u8, timbre(128 + k))).collect();
        let b = Banco {
            tremolo_profundo: true,
            vibrato_profundo: true,
            curva_volume_db: 40.0,
            ganho: 1.0,
            melodicos,
            percussao,
        };
        b.validar()?;
        Ok(b)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn registradores_viram_campos() {
        let op = Operador::de_registros(0xA1, 0x8F, 0xF2, 0x53, 0x05);
        assert_eq!(
            op,
            Operador { am: true, vib: false, sustenta: true, ksr: false, mult: 1, ksl: 2, tl: 15, ar: 15, dr: 2, sl: 5, rr: 3, onda: 5 }
        );
    }

    #[test]
    fn op2_sintetico() {
        let mut d = b"#OPL_II#".to_vec();
        for i in 0..175u16 {
            let mut inst = vec![0u8; 36];
            if i >= 128 {
                inst[0] = 1; // nota fixa
                inst[3] = 60;
            }
            // modulador: 0x20=0x21, 0x60=0xF0, 0x80=0x0F, 0xE0=1, KSL=0x40, nível=0x10
            inst[4..10].copy_from_slice(&[0x21, 0xF0, 0x0F, 0x01, 0x40, 0x10]);
            inst[10] = 0x0E; // realimentação 7, FM
            inst[11..17].copy_from_slice(&[0x01, 0xF4, 0x26, 0x00, 0x00, 0x00]);
            inst[18..20].copy_from_slice(&(-12i16).to_le_bytes());
            d.extend_from_slice(&inst);
        }
        for _ in 0..175 {
            d.extend_from_slice(&[b'x'; 32]);
        }
        let b = Banco::de_op2(&d).unwrap();
        assert_eq!(b.melodicos.len(), 128);
        let t = &b.melodicos[5];
        assert_eq!((t.modulador.mult, t.modulador.ksl, t.modulador.tl, t.modulador.onda), (1, 1, 0x10, 1));
        assert_eq!((t.realimentacao, t.aditivo, t.transposicao), (7, false, -12));
        assert_eq!(b.percussao[&35].nota_fixa, Some(60));
        assert_eq!(b.percussao.len(), 47);
    }

    #[test]
    fn banco_do_port_carrega() {
        let arq = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/opl_banco.json");
        let b = Banco::de_json(&std::fs::read(arq).unwrap()).unwrap();
        assert_eq!(b.melodicos.len(), 128);
        // Notas que não soam no original (medido nas gravações do DOSBox-X, docs/spec/regras/som.md,
        // "Percussão"): 59, 67, 70, 77 e 80 do mapa GM e as 85-95 de fora dele.
        let mudas = [59, 67, 70, 77, 80];
        assert!((35..=81).filter(|n| !mudas.contains(n)).all(|n| b.percussao.get(&n).is_some_and(|t| t.nota_fixa.is_some())));
        assert!(mudas.iter().chain(&[85, 86, 89, 91, 95]).all(|n| !b.percussao.contains_key(n)));
    }

    #[test]
    fn json_rejeita_faixa_invalida() {
        let op = r#"{"mult":1,"tl":0,"ar":15,"dr":0,"sl":0,"rr":0}"#;
        let ruim = r#"{"mult":16,"tl":0,"ar":15,"dr":0,"sl":0,"rr":0}"#;
        let banco = |m: &str| format!(r#"{{"melodicos":[{}]}}"#, vec![format!(r#"{{"modulador":{m},"portadora":{op}}}"#); 128].join(","));
        assert!(Banco::de_json(banco(op).as_bytes()).is_ok());
        assert!(Banco::de_json(banco(ruim).as_bytes()).is_err());
    }
}
