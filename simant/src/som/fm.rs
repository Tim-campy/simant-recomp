//! Sintetizador FM no estilo do OPL2/OPL3 (Yamaha YM3812/YMF262), a placa em que o Windows 3.1 com
//! Sound Blaster tocava o MIDI do SimAnt (MIDI Mapper → driver FM). Implementação própria, a partir
//! do comportamento documentado do chip (fichas técnicas da Yamaha e descrições públicas): dois
//! operadores por voz, multiplicadores, KSL, KSR, envelopes ADSR com as tabelas de tempo do chip,
//! 8 formas de onda, realimentação, conexão FM ou aditiva, LFO de tremolo e vibrato. Não é uma
//! emulação ciclo a ciclo: as amplitudes são calculadas em ponto flutuante e não há modo de
//! 4 operadores nem modo de ritmo. O "driver" (MIDI → vozes) também é próprio: 18 vozes (as do OPL3
//! em modo de 2 operadores), um timbre por programa General MIDI e um por nota de percussão
//! (`banco_fm.rs`). Decisão em docs/decisoes.md; comparação com o original em docs/spec/regras/som.md.

use std::f64::consts::TAU;

use super::banco_fm::{Banco, Operador, Timbre};
use super::sintetizador::Sintetizador;

/// Taxa interna do OPL (14,31818 MHz / 288).
pub const TAXA_OPL: f64 = 49_716.0;
/// Vozes simultâneas (OPL3 com 18 canais de 2 operadores).
pub const VOZES: usize = 18;
/// Multiplicadores de frequência do OPL (registrador MULT).
const MULT: [f64; 16] = [0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 10.0, 12.0, 12.0, 15.0, 15.0];
/// Base do KSL por F-number (4 bits altos), em unidades de 0,1875 dB × 4 (tabela do chip).
const KSL_BASE: [i32; 16] = [0, 32, 40, 45, 48, 51, 53, 55, 56, 58, 59, 60, 61, 62, 63, 64];
/// Unidade de atenuação do envelope: 0,1875 dB; 511 ≈ 96 dB (silêncio).
const DB_POR_UNIDADE: f64 = 0.1875;
const SILENCIO: f64 = 511.0;
/// Tempos do chip para a taxa 1 (ficha do YM3812): ataque 0→100 % e decaimento 0→96 dB.
const ATAQUE_TAXA1_S: f64 = 2.82624;
const DECAIMENTO_TAXA1_S: f64 = 39.28064;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Estagio {
    Ataque,
    Decaimento,
    Sustentacao,
    Soltura,
}

/// Estado de um operador numa voz.
#[derive(Clone, Copy, Debug)]
struct EstadoOp {
    p: Operador,
    /// Fase em ciclos (0..1).
    fase: f64,
    /// Incremento de fase por amostra do chip (sem vibrato).
    passo: f64,
    /// Atenuação do envelope, em unidades de 0,1875 dB (0 = máximo, 511 = silêncio).
    env: f64,
    estagio: Estagio,
    /// Atenuação fixa: TL + KSL + volume da nota (unidades de 0,1875 dB).
    fixa: f64,
    /// Constantes por amostra do chip: ataque (fração), decaimento e soltura (unidades).
    k_ataque: f64,
    d_decaimento: f64,
    d_soltura: f64,
    /// Saídas anteriores (realimentação).
    saida: [f64; 2],
}

/// Tempo de escala para a taxa efetiva `p` (0–63): cada 4 passos dividem o tempo por 2.
fn escala_taxa(p: u32) -> f64 {
    2f64.powf(-((p as f64) - 4.0) / 4.0)
}

impl EstadoOp {
    fn novo(p: Operador) -> EstadoOp {
        EstadoOp {
            p,
            fase: 0.0,
            passo: 0.0,
            env: SILENCIO,
            estagio: Estagio::Soltura,
            fixa: 0.0,
            k_ataque: 0.0,
            d_decaimento: 0.0,
            d_soltura: 0.0,
            saida: [0.0; 2],
        }
    }

    /// Prepara frequência, KSL e taxas para uma nota de frequência `hz` (bloco/F-number do OPL).
    fn afinar(&mut self, hz: f64, bloco: u32, fnum: u32, extra: f64) {
        self.passo = hz * MULT[self.p.mult as usize] / TAXA_OPL;
        // KSL: tabela por F-number menos 8 unidades × 4 por oitava abaixo do bloco 8.
        let ksl = (KSL_BASE[(fnum >> 6) as usize & 15] * 4 - ((8 - bloco as i32) << 5)).max(0);
        let ksl = match self.p.ksl {
            0 => 0,
            1 => ksl >> 1,
            2 => ksl >> 2,
            _ => ksl,
        };
        self.fixa = self.p.tl as f64 * 4.0 + ksl as f64 + extra;
        // KSR: bloco e bit alto do F-number.
        let ksr_val = bloco * 2 + ((fnum >> 9) & 1);
        let deslocamento = if self.p.ksr { ksr_val } else { ksr_val >> 2 };
        let efetiva = |r: u8| if r == 0 { 0 } else { (4 * r as u32 + deslocamento).min(63) };
        let (a, d, s) = (efetiva(self.p.ar), efetiva(self.p.dr), efetiva(self.p.rr));
        // Ataque exponencial: de 511 a 0 em T (aproximação: −k·(env+16) por amostra).
        self.k_ataque = if a >= 60 {
            1.0
        } else if a == 0 {
            0.0
        } else {
            (527.0f64 / 16.0).ln() / (ATAQUE_TAXA1_S * escala_taxa(a) * TAXA_OPL)
        };
        let linear = |p: u32| if p == 0 { 0.0 } else { 512.0 / (DECAIMENTO_TAXA1_S * escala_taxa(p) * TAXA_OPL) };
        self.d_decaimento = linear(d);
        self.d_soltura = linear(s);
    }

    fn ligar(&mut self) {
        self.estagio = Estagio::Ataque;
        self.fase = 0.0;
        if self.k_ataque >= 1.0 {
            self.env = 0.0;
            self.estagio = Estagio::Decaimento;
        }
    }

    fn soltar(&mut self) {
        self.estagio = Estagio::Soltura;
    }

    fn nivel_sustentacao(&self) -> f64 {
        if self.p.sl >= 15 {
            496.0
        } else {
            self.p.sl as f64 * 16.0
        }
    }

    /// Avança o envelope uma amostra do chip.
    fn envelope(&mut self) {
        match self.estagio {
            Estagio::Ataque => {
                self.env -= self.k_ataque * (self.env + 16.0);
                if self.env <= 0.0 {
                    self.env = 0.0;
                    self.estagio = Estagio::Decaimento;
                }
            }
            Estagio::Decaimento => {
                self.env += self.d_decaimento;
                let sl = self.nivel_sustentacao();
                if self.env >= sl {
                    self.env = sl;
                    // Sem EG-TYP, depois do decaimento vem a soltura, com a tecla ainda apertada.
                    self.estagio = if self.p.sustenta { Estagio::Sustentacao } else { Estagio::Soltura };
                }
            }
            Estagio::Sustentacao => {}
            Estagio::Soltura => {
                self.env = (self.env + self.d_soltura).min(SILENCIO);
            }
        }
    }

    /// Saída (−1..1) para a fase atual mais `modulacao` (em ciclos).
    fn saida(&self, modulacao: f64, tremolo: f64) -> f64 {
        let at = self.env + self.fixa + if self.p.am { tremolo } else { 0.0 };
        if at >= SILENCIO {
            return 0.0;
        }
        let x = (self.fase + modulacao).rem_euclid(1.0);
        onda(self.p.onda, x) * 10f64.powf(-at * DB_POR_UNIDADE / 20.0)
    }

    fn silenciado(&self) -> bool {
        self.estagio == Estagio::Soltura && self.env >= SILENCIO
    }
}

/// As 8 formas de onda do OPL3, para a fase `x` em ciclos (0..1).
fn onda(n: u8, x: f64) -> f64 {
    let s = (TAU * x).sin();
    match n {
        0 => s,
        1 => s.max(0.0),
        2 => s.abs(),
        3 => {
            if x.rem_euclid(0.5) < 0.25 {
                s.abs()
            } else {
                0.0
            }
        }
        4 => {
            if x < 0.5 {
                (2.0 * TAU * x).sin()
            } else {
                0.0
            }
        }
        5 => {
            if x < 0.5 {
                (2.0 * TAU * x).sin().abs()
            } else {
                0.0
            }
        }
        6 => {
            if x < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        _ => {
            // "Quadrada derivada": rampa exponencial (2^(−p/32), p = posição em 1/1024 de ciclo).
            let p = x * 1024.0;
            if p < 512.0 {
                2f64.powf(-p / 32.0)
            } else {
                -(2f64.powf(-(1023.0 - p) / 32.0))
            }
        }
    }
}

/// Bloco e F-number do OPL para a frequência `hz` (o menor bloco em que o F-number cabe em 10 bits).
fn bloco_fnum(hz: f64) -> (u32, u32) {
    for bloco in 0..8u32 {
        let fnum = hz * 2f64.powi(20 - bloco as i32) / TAXA_OPL;
        if fnum < 1024.0 {
            return (bloco, fnum.round() as u32);
        }
    }
    (7, 1023)
}

#[derive(Clone, Debug)]
struct Voz {
    canal: u8,
    /// Nota recebida (para o note-off) e nota tocada (com transposição ou nota fixa).
    nota: u8,
    tocada: f64,
    /// Atenuação da nota (velocidade × volume × expressão), em unidades de 0,1875 dB.
    volume: f64,
    velocidade: u8,
    aditivo: bool,
    realimentacao: u8,
    ops: [EstadoOp; 2],
    /// Tecla solta, mas o pedal de sustentação segura.
    no_pedal: bool,
    solta: bool,
    idade: u64,
}

#[derive(Clone, Copy, Debug)]
struct CanalMidi {
    programa: u8,
    volume: u8,
    expressao: u8,
    pan: u8,
    /// Curvatura em semitons.
    curva: f64,
    pedal: bool,
}

impl Default for CanalMidi {
    fn default() -> Self {
        CanalMidi { programa: 0, volume: 100, expressao: 127, pan: 64, curva: 0.0, pedal: false }
    }
}

/// Sintetizador FM com o banco dado. Gera a 49 716 Hz e reamostra (linear) para a taxa de saída.
pub struct SintFm {
    banco: Banco,
    taxa: f64,
    canal_percussao: u8,
    canais: [CanalMidi; 16],
    vozes: Vec<Voz>,
    relogio: u64,
    /// Fase dos LFOs em amostras do chip.
    lfo: u64,
    /// Reamostragem: posição fracionária e as duas últimas amostras do chip (E, D).
    pos: f64,
    ant: (f64, f64),
    atual: (f64, f64),
}

impl SintFm {
    pub fn new(banco: Banco, taxa: u32, canal_percussao: u8) -> SintFm {
        SintFm {
            banco,
            taxa: taxa as f64,
            canal_percussao,
            canais: [CanalMidi::default(); 16],
            vozes: Vec::new(),
            relogio: 0,
            lfo: 0,
            pos: 0.0,
            ant: (0.0, 0.0),
            atual: (0.0, 0.0),
        }
    }

    fn timbre(&self, c: u8, nota: u8) -> Option<Timbre> {
        if c + 1 == self.canal_percussao {
            self.banco.percussao.get(&nota).cloned()
        } else {
            self.banco.melodicos.get(self.canais[c as usize].programa as usize).cloned()
        }
    }

    /// Atenuação (unidades de 0,1875 dB) de velocidade × volume × expressão.
    fn atenuacao(&self, c: u8, vel: u8) -> f64 {
        let k = &self.canais[c as usize];
        let nivel = (vel as f64 / 127.0) * (k.volume as f64 / 127.0) * (k.expressao as f64 / 127.0);
        if nivel <= 0.0 {
            return SILENCIO;
        }
        (-(self.banco.curva_volume_db as f64) * nivel.log10() / DB_POR_UNIDADE).min(SILENCIO)
    }

    fn afinar(&self, v: &mut Voz) {
        let curva = if v.canal + 1 == self.canal_percussao { 0.0 } else { self.canais[v.canal as usize].curva };
        let hz = 440.0 * 2f64.powf((v.tocada + curva - 69.0) / 12.0);
        let (bloco, fnum) = bloco_fnum(hz);
        let volume = v.volume;
        // Na conexão aditiva o modulador também soa: o volume da nota vale para os dois.
        let mod_extra = if v.aditivo { volume } else { 0.0 };
        v.ops[0].afinar(hz, bloco, fnum, mod_extra);
        v.ops[1].afinar(hz, bloco, fnum, volume);
    }

    fn ligar(&mut self, c: u8, nota: u8, vel: u8) {
        self.desligar(c, nota, true);
        let Some(t) = self.timbre(c, nota) else { return };
        let tocada = match t.nota_fixa {
            Some(n) => n as f64,
            None => nota as f64 + t.transposicao as f64,
        };
        self.relogio += 1;
        let mut v = Voz {
            canal: c,
            nota,
            tocada,
            volume: self.atenuacao(c, vel),
            velocidade: vel,
            aditivo: t.aditivo,
            realimentacao: t.realimentacao,
            ops: [EstadoOp::novo(t.modulador), EstadoOp::novo(t.portadora)],
            no_pedal: false,
            solta: false,
            idade: self.relogio,
        };
        self.afinar(&mut v);
        v.ops[0].ligar();
        v.ops[1].ligar();
        if self.vozes.len() >= VOZES {
            // Rouba uma voz: a solta mais silenciosa; sem solta, a mais antiga.
            let i = (0..self.vozes.len())
                .max_by(|&a, &b| {
                    let (va, vb) = (&self.vozes[a], &self.vozes[b]);
                    (va.solta, va.ops[1].env, std::cmp::Reverse(va.idade))
                        .partial_cmp(&(vb.solta, vb.ops[1].env, std::cmp::Reverse(vb.idade)))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0);
            self.vozes.swap_remove(i);
        }
        self.vozes.push(v);
    }

    /// Solta as vozes de `nota` no canal (`forcar` ignora o pedal: nota repetida).
    fn desligar(&mut self, c: u8, nota: u8, forcar: bool) {
        let pedal = self.canais[c as usize].pedal && !forcar;
        for v in self.vozes.iter_mut().filter(|v| v.canal == c && v.nota == nota && !v.solta) {
            if pedal {
                v.no_pedal = true;
            } else {
                v.solta = true;
                v.ops[0].soltar();
                v.ops[1].soltar();
            }
        }
    }

    fn soltar_canal(&mut self, c: u8) {
        for v in self.vozes.iter_mut().filter(|v| v.canal == c) {
            v.solta = true;
            v.ops[0].soltar();
            v.ops[1].soltar();
        }
    }

    fn reafinar_canal(&mut self, c: u8, volume: bool) {
        let mut vozes = std::mem::take(&mut self.vozes);
        for v in vozes.iter_mut().filter(|v| v.canal == c) {
            if volume {
                v.volume = self.atenuacao(c, v.velocidade);
            }
            self.afinar(v);
        }
        self.vozes = vozes;
    }

    /// Uma amostra do chip (E, D).
    fn amostra_chip(&mut self) -> (f64, f64) {
        // LFOs: tremolo ~3,7 Hz (13 432 amostras), vibrato ~6,1 Hz (8 192 amostras), triangulares.
        let t = (self.lfo % 13_432) as f64 / 13_432.0;
        let tri_t = 1.0 - (2.0 * t - 1.0).abs();
        let tremolo = tri_t * if self.banco.tremolo_profundo { 4.8 } else { 1.0 } / DB_POR_UNIDADE;
        let v = (self.lfo % 8_192) as f64 / 8_192.0;
        let tri_v = if v < 0.25 { 4.0 * v } else if v < 0.75 { 2.0 - 4.0 * v } else { 4.0 * v - 4.0 };
        let cents = if self.banco.vibrato_profundo { 14.0 } else { 7.0 };
        let vibrato = 2f64.powf(tri_v * cents / 1200.0);
        self.lfo += 1;
        let (mut e, mut d) = (0.0, 0.0);
        for voz in &mut self.vozes {
            let fb = if voz.realimentacao > 0 {
                (voz.ops[0].saida[0] + voz.ops[0].saida[1]) * 4.0 / 2f64.powi(9 - voz.realimentacao as i32)
            } else {
                0.0
            };
            let m = voz.ops[0].saida(fb, tremolo);
            voz.ops[0].saida = [m, voz.ops[0].saida[0]];
            let s = if voz.aditivo { m + voz.ops[1].saida(0.0, tremolo) } else { voz.ops[1].saida(m * 4.0, tremolo) };
            for op in &mut voz.ops {
                let passo = if op.p.vib { op.passo * vibrato } else { op.passo };
                op.fase = (op.fase + passo).fract();
                op.envelope();
            }
            let pan = self.canais[voz.canal as usize].pan as f64 / 127.0;
            e += s * (1.0 - pan).sqrt() * std::f64::consts::SQRT_2;
            d += s * pan.sqrt() * std::f64::consts::SQRT_2;
        }
        self.vozes.retain(|v| !(v.ops[1].silenciado() && (!v.aditivo || v.ops[0].silenciado())));
        // Um canal no máximo = 4096/32768 da escala, como na saída do chip.
        let g = self.banco.ganho as f64 / 8.0;
        (e * g, d * g)
    }
}

impl Sintetizador for SintFm {
    fn mensagem(&mut self, status: u8, d1: u8, d2: u8) {
        let c = status & 0x0F;
        match status & 0xF0 {
            0x80 => self.desligar(c, d1, false),
            0x90 if d2 == 0 => self.desligar(c, d1, false),
            0x90 => self.ligar(c, d1, d2),
            0xB0 => match d1 {
                7 => {
                    self.canais[c as usize].volume = d2;
                    self.reafinar_canal(c, true);
                }
                10 => self.canais[c as usize].pan = d2,
                11 => {
                    self.canais[c as usize].expressao = d2;
                    self.reafinar_canal(c, true);
                }
                64 => {
                    let ligado = d2 >= 64;
                    self.canais[c as usize].pedal = ligado;
                    if !ligado {
                        for v in self.vozes.iter_mut().filter(|v| v.canal == c && v.no_pedal) {
                            v.no_pedal = false;
                            v.solta = true;
                            v.ops[0].soltar();
                            v.ops[1].soltar();
                        }
                    }
                }
                120 | 123 => self.soltar_canal(c),
                121 => {
                    let programa = self.canais[c as usize].programa;
                    self.canais[c as usize] = CanalMidi { programa, ..CanalMidi::default() };
                    self.reafinar_canal(c, true);
                }
                _ => {}
            },
            0xC0 => self.canais[c as usize].programa = d1 & 0x7F,
            0xE0 => {
                let valor = ((d2 as i32) << 7 | d1 as i32) - 8192;
                self.canais[c as usize].curva = valor as f64 / 8192.0 * 2.0;
                self.reafinar_canal(c, false);
            }
            _ => {}
        }
    }

    fn somar(&mut self, esq: &mut [f32], dir: &mut [f32]) {
        let passo = TAXA_OPL / self.taxa;
        for (e, d) in esq.iter_mut().zip(dir.iter_mut()) {
            self.pos += passo;
            while self.pos >= 1.0 {
                self.pos -= 1.0;
                self.ant = self.atual;
                self.atual = self.amostra_chip();
            }
            let f = self.pos;
            *e += (self.ant.0 + (self.atual.0 - self.ant.0) * f) as f32;
            *d += (self.ant.1 + (self.atual.1 - self.ant.1) * f) as f32;
        }
    }

    fn reiniciar(&mut self) {
        self.vozes.clear();
        self.canais = [CanalMidi::default(); 16];
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::som::banco_fm::Operador;

    fn op(mult: u8, tl: u8, ar: u8, dr: u8, sl: u8, rr: u8, sustenta: bool) -> Operador {
        Operador { mult, tl, ar, dr, sl, rr, sustenta, ..Operador::default() }
    }

    /// Banco de teste: todo programa é um seno puro (modulador mudo) e a nota 36 da percussão é fixa em 48.
    fn banco_seno() -> Banco {
        let seno = Timbre {
            nome: "seno".into(),
            modulador: op(1, 63, 15, 0, 0, 15, true),
            portadora: op(1, 0, 15, 0, 0, 8, true),
            ..Timbre::default()
        };
        let mut percussao = std::collections::BTreeMap::new();
        percussao.insert(36, Timbre { nota_fixa: Some(48), portadora: op(1, 0, 15, 6, 15, 6, false), ..seno.clone() });
        Banco { tremolo_profundo: true, vibrato_profundo: true, curva_volume_db: 40.0, ganho: 1.0, melodicos: vec![seno; 128], percussao }
    }

    fn render(s: &mut SintFm, n: usize) -> Vec<f32> {
        let (mut e, mut d) = (vec![0.0; n], vec![0.0; n]);
        s.somar(&mut e, &mut d);
        e
    }

    fn frequencia(x: &[f32], taxa: f64) -> f64 {
        let cruzamentos = x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        cruzamentos as f64 * taxa / x.len() as f64
    }

    #[test]
    fn seno_na_altura_certa_e_volume_do_chip() {
        let mut s = SintFm::new(banco_seno(), 44_100, 10);
        s.mensagem(0x90, 69, 127);
        s.mensagem(0xB0, 7, 127);
        let x = render(&mut s, 44_100);
        let f = frequencia(&x[4410..], 44_100.0);
        assert!((f - 440.0).abs() < 2.0, "{f} Hz");
        let pico = x[4410..].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        // TL 0, velocidade e volume máximos: 1/8 da escala (pan central: ×√2·√½ = 1).
        assert!((pico - 0.125).abs() < 0.005, "pico {pico}");
    }

    #[test]
    fn nota_solta_some_e_libera_a_voz() {
        let mut s = SintFm::new(banco_seno(), 22_050, 10);
        s.mensagem(0x90, 60, 100);
        render(&mut s, 2_000);
        assert_eq!(s.vozes.len(), 1);
        s.mensagem(0x80, 60, 0);
        // RR 8: 96 dB em ~153 ms (taxa efetiva 32 + KSR).
        render(&mut s, 22_050);
        assert!(s.vozes.is_empty());
    }

    #[test]
    fn percussao_usa_nota_fixa_e_decai_sozinha() {
        let mut s = SintFm::new(banco_seno(), 44_100, 10);
        s.mensagem(0x99, 36, 120);
        let x = render(&mut s, 8_820);
        let f = frequencia(&x[100..4000], 44_100.0);
        assert!((f - 130.8).abs() < 6.0, "{f} Hz");
        // Nota sem timbre na percussão: nada toca.
        s.mensagem(0x99, 90, 120);
        assert_eq!(s.vozes.len(), 1);
        render(&mut s, 44_100 * 2);
        assert!(s.vozes.is_empty(), "sem EG-TYP o envelope solta sozinho");
    }

    #[test]
    fn volume_segue_a_curva() {
        let mut a = SintFm::new(banco_seno(), 44_100, 10);
        a.mensagem(0x90, 69, 127);
        let mut b = SintFm::new(banco_seno(), 44_100, 10);
        b.mensagem(0x90, 69, 64);
        let (xa, xb) = (render(&mut a, 22_050), render(&mut b, 22_050));
        let rms = |x: &[f32]| (x[2205..].iter().map(|v| (v * v) as f64).sum::<f64>() / (x.len() - 2205) as f64).sqrt();
        let db = 20.0 * (rms(&xb) / rms(&xa)).log10();
        let esperado = 40.0 * (64.0f64 / 127.0).log10();
        assert!((db - esperado).abs() < 0.5, "{db} dB, esperado {esperado}");
    }

    #[test]
    fn no_maximo_18_vozes() {
        let mut s = SintFm::new(banco_seno(), 22_050, 10);
        for n in 40..70 {
            s.mensagem(0x90, n, 100);
        }
        assert_eq!(s.vozes.len(), VOZES);
        assert!(s.vozes.iter().any(|v| v.nota == 69));
    }

    #[test]
    fn bloco_e_fnum_como_no_opl() {
        // Lá 440 Hz: bloco 4, F-number ≈ 580 (440 · 2^16 / 49716).
        assert_eq!(bloco_fnum(440.0), (4, 580));
        assert_eq!(bloco_fnum(20.0).0, 0);
    }

    #[test]
    fn ondas_basicas() {
        assert!((onda(0, 0.25) - 1.0).abs() < 1e-9);
        assert_eq!(onda(1, 0.75), 0.0);
        assert!((onda(2, 0.75) - 1.0).abs() < 1e-9);
        assert_eq!(onda(3, 0.3), 0.0);
        assert_eq!(onda(6, 0.6), -1.0);
        assert!((onda(7, 0.0) - 1.0).abs() < 1e-9);
    }
}
