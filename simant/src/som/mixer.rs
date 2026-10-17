//! Mistura de uma música (sequência MIDI + sintetizador) e um efeito (PCM), em blocos estéreo.
//! Não depende do SDL: o mesmo código serve à reprodução e à exportação para WAV.

use engine::formats::midi::EventoMidi;

use super::sintetizador::Sintetizador;

/// Resolução do sequenciador: no máximo esse número de quadros entre duas entregas de eventos.
const BLOCO: usize = 64;

struct Reproducao {
    eventos: Vec<EventoMidi>,
    proximo: usize,
    segundos: f64,
}

pub struct Mixer {
    taxa: u32,
    sint: Box<dyn Sintetizador>,
    musica: Option<Reproducao>,
    /// Amostras mono (−1..1) já na taxa de saída e a posição atual.
    efeito: Option<(Vec<f32>, usize)>,
    pub volume_musica: f32,
    pub volume_efeitos: f32,
    esq: Vec<f32>,
    dir: Vec<f32>,
}

impl Mixer {
    pub fn new(taxa: u32, sint: Box<dyn Sintetizador>, volume_musica: f32, volume_efeitos: f32) -> Mixer {
        Mixer { taxa, sint, musica: None, efeito: None, volume_musica, volume_efeitos, esq: vec![0.0; BLOCO], dir: vec![0.0; BLOCO] }
    }

    /// Começa uma música do início; a anterior é cortada (como ao abrir outro MIDI no MCI).
    pub fn tocar_musica(&mut self, eventos: Vec<EventoMidi>) {
        self.sint.reiniciar();
        self.musica = Some(Reproducao { eventos, proximo: 0, segundos: 0.0 });
    }

    pub fn parar_musica(&mut self) {
        self.sint.reiniciar();
        self.musica = None;
    }

    /// Verdadeiro enquanto há eventos da música por entregar.
    pub fn musica_tocando(&self) -> bool {
        self.musica.as_ref().is_some_and(|m| m.proximo < m.eventos.len())
    }

    /// Começa um efeito; o anterior é cortado (um só `waveOut` no original).
    pub fn tocar_efeito(&mut self, amostras: Vec<f32>) {
        self.efeito = Some((amostras, 0));
    }

    pub fn parar_efeito(&mut self) {
        self.efeito = None;
    }

    #[cfg(test)]
    pub fn efeito_tocando(&self) -> bool {
        self.efeito.is_some()
    }

    /// Preenche `saida` (estéreo intercalado E, D), sobrescrevendo.
    pub fn render(&mut self, saida: &mut [f32]) {
        for bloco in saida.chunks_mut(BLOCO * 2) {
            let n = bloco.len() / 2;
            self.entregar_eventos();
            self.esq[..n].fill(0.0);
            self.dir[..n].fill(0.0);
            self.sint.somar(&mut self.esq[..n], &mut self.dir[..n]);
            if let Some(m) = &mut self.musica {
                m.segundos += n as f64 / self.taxa as f64;
            }
            let efeito = self.efeito_proximo(n);
            for i in 0..n {
                let e = efeito.get(i).copied().unwrap_or(0.0) * self.volume_efeitos;
                bloco[2 * i] = (self.esq[i] * self.volume_musica + e).clamp(-1.0, 1.0);
                bloco[2 * i + 1] = (self.dir[i] * self.volume_musica + e).clamp(-1.0, 1.0);
            }
        }
    }

    fn entregar_eventos(&mut self) {
        let Some(m) = &mut self.musica else { return };
        while let Some(ev) = m.eventos.get(m.proximo) {
            if ev.segundos > m.segundos {
                break;
            }
            self.sint.mensagem(ev.status, ev.d1, ev.d2);
            m.proximo += 1;
        }
    }

    fn efeito_proximo(&mut self, n: usize) -> Vec<f32> {
        let Some((amostras, pos)) = &mut self.efeito else { return Vec::new() };
        let fim = (*pos + n).min(amostras.len());
        let trecho = amostras[*pos..fim].to_vec();
        *pos = fim;
        if fim >= amostras.len() {
            self.efeito = None;
        }
        trecho
    }
}

/// PCM de 8 bits sem sinal em `de` Hz → mono em ponto flutuante a `para` Hz (interpolação linear).
pub fn reamostrar(pcm: &[u8], de: u32, para: u32) -> Vec<f32> {
    if pcm.is_empty() || de == 0 || para == 0 {
        return Vec::new();
    }
    let x: Vec<f32> = pcm.iter().map(|&b| (b as f32 - 128.0) / 128.0).collect();
    let n = (x.len() as u64 * para as u64 / de as u64).max(1) as usize;
    let razao = de as f64 / para as f64;
    (0..n)
        .map(|i| {
            let p = i as f64 * razao;
            let k = p as usize;
            let f = (p - k as f64) as f32;
            let a = x[k.min(x.len() - 1)];
            let b = x[(k + 1).min(x.len() - 1)];
            a + (b - a) * f
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::super::sintetizador::SintSimples;
    use super::*;

    #[test]
    fn reamostra_mantendo_a_duracao() {
        let r = reamostrar(&[128, 255, 0, 128], 4096, 8192);
        assert_eq!(r.len(), 8);
        assert_eq!(r[0], 0.0);
        assert!((r[2] - 127.0 / 128.0).abs() < 1e-6);
        assert!(r[1] > 0.0 && r[1] < r[2], "meio do caminho");
    }

    #[test]
    fn efeito_toca_uma_vez_e_o_novo_corta_o_anterior() {
        let mut m = Mixer::new(1000, Box::new(SintSimples::new(1000, 10)), 1.0, 1.0);
        m.tocar_efeito(vec![0.5; 100]);
        let mut s = vec![0.0; 2 * 60];
        m.render(&mut s);
        assert_eq!((s[0], s[1]), (0.5, 0.5));
        m.tocar_efeito(vec![-0.25; 10]);
        m.render(&mut s);
        assert_eq!(s[0], -0.25);
        assert_eq!(s[2 * 10], 0.0, "acabou depois de 10 amostras");
        assert!(!m.efeito_tocando());
    }

    #[test]
    fn musica_entrega_eventos_no_tempo() {
        let mut m = Mixer::new(1000, Box::new(SintSimples::new(1000, 10)), 1.0, 0.0);
        let ev = |s: f64, st: u8, n: u8, v: u8| EventoMidi { segundos: s, status: st, d1: n, d2: v };
        m.tocar_musica(vec![ev(0.0, 0x90, 60, 100), ev(0.5, 0x80, 60, 0)]);
        let mut s = vec![0.0; 2 * 400];
        m.render(&mut s);
        assert!(m.musica_tocando());
        assert!(s.iter().any(|x| x.abs() > 0.01));
        m.render(&mut s);
        assert!(!m.musica_tocando());
    }
}
