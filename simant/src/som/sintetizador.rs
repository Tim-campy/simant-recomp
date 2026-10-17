//! Sintetizadores MIDI: `rustysynth` com uma SoundFont, ou um sintetizador simples próprio.
//! Decisão em docs/decisoes.md (2026-10-17).

use std::sync::Arc;

/// O que o mixer precisa de um sintetizador: mensagens de canal e blocos de áudio estéreo.
pub trait Sintetizador: Send {
    /// `status` com o canal (0x80–0xEF).
    fn mensagem(&mut self, status: u8, d1: u8, d2: u8);
    /// Soma (não sobrescreve) o áudio em `esq` e `dir`.
    fn somar(&mut self, esq: &mut [f32], dir: &mut [f32]);
    /// Silencia tudo e volta os controladores ao padrão (troca de música).
    fn reiniciar(&mut self);
}

// ---------------------------------------------------------------------------------------------
// rustysynth
// ---------------------------------------------------------------------------------------------

pub struct SintSoundFont {
    synth: rustysynth::Synthesizer,
    tmp_e: Vec<f32>,
    tmp_d: Vec<f32>,
}

impl SintSoundFont {
    pub fn abrir(caminho: &std::path::Path, taxa: u32) -> Result<SintSoundFont, String> {
        let mut f = std::io::BufReader::new(std::fs::File::open(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?);
        let sf = rustysynth::SoundFont::new(&mut f).map_err(|e| format!("{}: {e:?}", caminho.display()))?;
        let cfg = rustysynth::SynthesizerSettings::new(taxa as i32);
        let synth = rustysynth::Synthesizer::new(&Arc::new(sf), &cfg).map_err(|e| format!("rustysynth: {e:?}"))?;
        Ok(SintSoundFont { synth, tmp_e: Vec::new(), tmp_d: Vec::new() })
    }
}

impl Sintetizador for SintSoundFont {
    fn mensagem(&mut self, status: u8, d1: u8, d2: u8) {
        self.synth.process_midi_message((status & 0x0F) as i32, (status & 0xF0) as i32, d1 as i32, d2 as i32);
    }
    fn somar(&mut self, esq: &mut [f32], dir: &mut [f32]) {
        self.tmp_e.resize(esq.len(), 0.0);
        self.tmp_d.resize(dir.len(), 0.0);
        self.synth.render(&mut self.tmp_e, &mut self.tmp_d);
        for (s, t) in esq.iter_mut().zip(&self.tmp_e) {
            *s += t;
        }
        for (s, t) in dir.iter_mut().zip(&self.tmp_d) {
            *s += t;
        }
    }
    fn reiniciar(&mut self) {
        self.synth.reset();
    }
}

// ---------------------------------------------------------------------------------------------
// Sintetizador simples
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Onda {
    Seno,
    Triangulo,
    Quadrada,
    Serra,
}

/// Onda e tempo de decaimento (s) por família de instrumento General MIDI (programa / 8).
fn timbre(programa: u8) -> (Onda, f32) {
    match programa / 8 {
        0 => (Onda::Triangulo, 1.2),  // pianos
        1 => (Onda::Seno, 0.8),       // percussão cromática
        2 => (Onda::Quadrada, 0.0),   // órgãos (sustenta)
        3 => (Onda::Triangulo, 0.9),  // violões e guitarras
        4 => (Onda::Triangulo, 0.0),  // baixos
        5 | 6 => (Onda::Serra, 0.0),  // cordas e ensembles
        7 | 8 => (Onda::Quadrada, 0.0), // metais e palhetas
        9 => (Onda::Seno, 0.0),       // flautas
        10 => (Onda::Quadrada, 0.0),  // solos sintetizados
        _ => (Onda::Seno, 0.0),       // pads e efeitos
    }
}

#[derive(Clone, Copy)]
struct Voz {
    canal: u8,
    nota: u8,
    percussao: bool,
    fase: f32,
    passo: f32,
    amplitude: f32,
    onda: Onda,
    /// Segundos desde o ataque.
    idade: f32,
    decaimento: f32,
    /// Em soltura: nível no início e tempo decorrido.
    soltura: Option<(f32, f32)>,
    ruido: u32,
}

#[derive(Clone, Copy)]
struct Canal {
    programa: u8,
    volume: f32,
    expressao: f32,
    pan: f32,
    /// Em semitons.
    curva: f32,
}

impl Default for Canal {
    fn default() -> Self {
        Canal { programa: 0, volume: 100.0 / 127.0, expressao: 1.0, pan: 0.5, curva: 0.0 }
    }
}

const MAX_VOZES: usize = 32;
const ATAQUE: f32 = 0.005;
const SOLTURA: f32 = 0.12;

/// Ondas básicas por família GM e ruído para a percussão. Serve para ouvir as músicas sem
/// SoundFont; não pretende soar como o original.
pub struct SintSimples {
    taxa: f32,
    canal_percussao: u8,
    canais: [Canal; 16],
    vozes: Vec<Voz>,
    ruido: u32,
}

impl SintSimples {
    pub fn new(taxa: u32, canal_percussao: u8) -> SintSimples {
        SintSimples { taxa: taxa as f32, canal_percussao, canais: [Canal::default(); 16], vozes: Vec::new(), ruido: 0x1234_5678 }
    }

    fn frequencia(nota: u8, curva: f32) -> f32 {
        440.0 * 2f32.powf((nota as f32 + curva - 69.0) / 12.0)
    }

    fn ligar(&mut self, c: u8, nota: u8, vel: u8) {
        self.desligar(c, nota);
        let canal = self.canais[c as usize];
        let percussao = c + 1 == self.canal_percussao;
        let (onda, decaimento) = if percussao { (Onda::Seno, 0.0) } else { timbre(canal.programa) };
        if self.vozes.len() >= MAX_VOZES {
            // Rouba a voz mais velha.
            let i = (0..self.vozes.len()).max_by(|&a, &b| self.vozes[a].idade.total_cmp(&self.vozes[b].idade)).unwrap_or(0);
            self.vozes.swap_remove(i);
        }
        self.ruido = self.ruido.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.vozes.push(Voz {
            canal: c,
            nota,
            percussao,
            fase: 0.0,
            // Na percussão o passo é o período de amostragem; a altura sai da própria nota.
            passo: if percussao { 1.0 / self.taxa } else { Self::frequencia(nota, canal.curva) / self.taxa },
            amplitude: (vel as f32 / 127.0).powi(2),
            onda,
            idade: 0.0,
            decaimento,
            soltura: None,
            ruido: self.ruido | 1,
        });
    }

    fn desligar(&mut self, c: u8, nota: u8) {
        for v in self.vozes.iter_mut().filter(|v| v.canal == c && v.nota == nota && v.soltura.is_none()) {
            let nivel = envelope(v);
            v.soltura = Some((nivel, 0.0));
        }
    }
}

/// Nível do envelope de uma voz que ainda não foi solta.
fn envelope(v: &Voz) -> f32 {
    let ataque = (v.idade / ATAQUE).min(1.0);
    if v.percussao {
        // Bumbo e tons graves duram mais; pratos e chimbal, menos.
        let dur = if v.nota < 50 { 0.25 } else { 0.12 };
        return ataque * (-v.idade / dur).exp();
    }
    if v.decaimento > 0.0 {
        ataque * (-v.idade / v.decaimento).exp().max(0.0)
    } else {
        ataque * 0.8
    }
}

fn amostra(v: &mut Voz) -> f32 {
    if v.percussao {
        if v.nota < 41 {
            // Bumbo: seno grave com queda de altura.
            let f = 60.0 + 80.0 * (-v.idade / 0.04).exp();
            v.fase = (v.fase + f * v.passo).fract();
            return (v.fase * std::f32::consts::TAU).sin();
        }
        v.ruido ^= v.ruido << 13;
        v.ruido ^= v.ruido >> 17;
        v.ruido ^= v.ruido << 5;
        return (v.ruido as f32 / u32::MAX as f32) * 2.0 - 1.0;
    }
    v.fase = (v.fase + v.passo).fract();
    let x = v.fase;
    match v.onda {
        Onda::Seno => (x * std::f32::consts::TAU).sin(),
        Onda::Triangulo => 1.0 - 4.0 * (x - 0.5).abs(),
        Onda::Quadrada => {
            if x < 0.5 {
                0.5
            } else {
                -0.5
            }
        }
        Onda::Serra => x - 0.5,
    }
}

impl Sintetizador for SintSimples {
    fn mensagem(&mut self, status: u8, d1: u8, d2: u8) {
        let c = status & 0x0F;
        match status & 0xF0 {
            0x80 => return self.desligar(c, d1),
            0x90 if d2 == 0 => return self.desligar(c, d1),
            0x90 => return self.ligar(c, d1, d2),
            _ => {}
        }
        let canal = &mut self.canais[c as usize];
        match status & 0xF0 {
            0xB0 => match d1 {
                7 => canal.volume = d2 as f32 / 127.0,
                10 => canal.pan = d2 as f32 / 127.0,
                11 => canal.expressao = d2 as f32 / 127.0,
                120 | 123 => {
                    for v in self.vozes.iter_mut().filter(|v| v.canal == c) {
                        v.soltura.get_or_insert((envelope(v), 0.0));
                    }
                }
                121 => *canal = Canal { programa: canal.programa, ..Canal::default() },
                _ => {}
            },
            0xC0 => canal.programa = d1,
            0xE0 => {
                let valor = ((d2 as i32) << 7 | d1 as i32) - 8192;
                canal.curva = valor as f32 / 8192.0 * 2.0;
                let curva = canal.curva;
                for v in self.vozes.iter_mut().filter(|v| v.canal == c && !v.percussao) {
                    v.passo = Self::frequencia(v.nota, curva) / self.taxa;
                }
            }
            _ => {}
        }
    }

    fn somar(&mut self, esq: &mut [f32], dir: &mut [f32]) {
        let dt = 1.0 / self.taxa;
        let ganho = 0.25;
        for v in self.vozes.iter_mut() {
            let canal = self.canais[v.canal as usize];
            let base = v.amplitude * canal.volume * canal.expressao * ganho;
            let (ge, gd) = ((1.0 - canal.pan).sqrt(), canal.pan.sqrt());
            for (e, d) in esq.iter_mut().zip(dir.iter_mut()) {
                let nivel = match &mut v.soltura {
                    Some((ini, t)) => {
                        *t += dt;
                        *ini * (1.0 - *t / SOLTURA).max(0.0)
                    }
                    None => envelope(v),
                };
                let x = amostra(v) * nivel * base;
                v.idade += dt;
                *e += x * ge;
                *d += x * gd;
            }
        }
        self.vozes.retain(|v| match v.soltura {
            Some((_, t)) => t < SOLTURA,
            None => !(v.percussao || v.decaimento > 0.0) || envelope(v) > 1e-3,
        });
    }

    fn reiniciar(&mut self) {
        self.vozes.clear();
        self.canais = [Canal::default(); 16];
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn nota_soa_e_some_depois_da_soltura() {
        let mut s = SintSimples::new(22050, 10);
        s.mensagem(0xC0, 40, 0);
        s.mensagem(0x90, 69, 100);
        let (mut e, mut d) = (vec![0.0; 2205], vec![0.0; 2205]);
        s.somar(&mut e, &mut d);
        assert!(e.iter().any(|x| x.abs() > 0.01));
        s.mensagem(0x80, 69, 0);
        let (mut e, mut d) = (vec![0.0; 22050], vec![0.0; 22050]);
        s.somar(&mut e, &mut d);
        assert!(s.vozes.is_empty());
        assert!(e[22049].abs() < 1e-6);
    }

    #[test]
    fn percussao_decai_sozinha() {
        let mut s = SintSimples::new(22050, 10);
        s.mensagem(0x99, 38, 120);
        let (mut e, mut d) = (vec![0.0; 44100], vec![0.0; 44100]);
        s.somar(&mut e, &mut d);
        assert!(e[..2000].iter().any(|x| x.abs() > 0.01));
        assert!(s.vozes.is_empty());
    }
}
