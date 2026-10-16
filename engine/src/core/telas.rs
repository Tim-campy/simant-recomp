//! Sequência de telas estáticas (abertura). Especificação: docs/spec/telas/abertura.md.
//! Sem dependência de janela: quem desenha é a camada de render.

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct BitmapNaTela {
    pub id: u16,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Gatilho {
    Espaco,
    Clique,
    QualquerTecla,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Tela {
    pub nome: String,
    pub largura: u32,
    pub altura: u32,
    /// Índice de paleta do fundo.
    pub fundo: u8,
    pub bitmaps: Vec<BitmapNaTela>,
    pub duracao_ticks: u32,
    pub fechar_com: Vec<Gatilho>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Sequencia {
    pub ms_por_tick: u32,
    pub telas: Vec<Tela>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evento {
    Espaco,
    Clique,
    OutraTecla,
}

/// Estado de execução de uma `Sequencia`.
pub struct Reprodutor {
    seq: Sequencia,
    indice: usize,
    ms_na_tela: u64,
}

impl Reprodutor {
    pub fn new(seq: Sequencia) -> Reprodutor {
        Reprodutor { seq, indice: 0, ms_na_tela: 0 }
    }

    pub fn atual(&self) -> Option<&Tela> {
        self.seq.telas.get(self.indice)
    }

    pub fn terminou(&self) -> bool {
        self.indice >= self.seq.telas.len()
    }

    fn proxima(&mut self) {
        self.indice += 1;
        self.ms_na_tela = 0;
    }

    pub fn passar_tempo(&mut self, ms: u64) {
        let Some(t) = self.atual() else { return };
        let limite = t.duracao_ticks as u64 * self.seq.ms_por_tick as u64;
        self.ms_na_tela += ms;
        if self.ms_na_tela >= limite {
            self.proxima();
        }
    }

    pub fn evento(&mut self, e: Evento) {
        let Some(t) = self.atual() else { return };
        let fecha = t.fechar_com.iter().any(|g| {
            matches!(
                (g, e),
                (Gatilho::QualquerTecla, Evento::Espaco | Evento::OutraTecla)
                    | (Gatilho::Espaco, Evento::Espaco)
                    | (Gatilho::Clique, Evento::Clique)
            )
        });
        if fecha {
            self.proxima();
        }
    }
}

/// Monta a tela como índices de paleta (largura × altura), desenhando os bitmaps na ordem da lista.
pub fn compor(
    tela: &Tela,
    recursos: &crate::recursos::Recursos,
) -> Result<Vec<u8>, String> {
    use crate::formats::banco::Chave;
    use crate::formats::bitmap::{Bitmap, TIPO_BITMAP};

    let (lw, lh) = (tela.largura as i32, tela.altura as i32);
    let mut buf = vec![tela.fundo; (lw * lh) as usize];
    for b in &tela.bitmaps {
        let bm = Bitmap::decodificar(&recursos.objeto(Chave { tipo: TIPO_BITMAP, id: b.id })?)?;
        for y in 0..bm.altura as i32 {
            for x in 0..bm.largura as i32 {
                let (dx, dy) = (b.x + x, b.y + y);
                if (0..lw).contains(&dx) && (0..lh).contains(&dy) {
                    buf[(dy * lw + dx) as usize] = bm.pixels[(y * bm.largura as i32 + x) as usize];
                }
            }
        }
    }
    Ok(buf)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn seq() -> Sequencia {
        serde_json::from_str(
            r#"{"ms_por_tick": 55, "telas": [
                {"nome":"a","largura":10,"altura":10,"fundo":0,"bitmaps":[],"duracao_ticks":720,"fechar_com":["espaco","clique"]},
                {"nome":"b","largura":10,"altura":10,"fundo":0,"bitmaps":[],"duracao_ticks":10,"fechar_com":[]}
            ]}"#,
        )
        .unwrap()
    }

    #[test]
    fn abertura_dura_720_ticks_de_55ms() {
        // docs/spec/telas/abertura.md: 40 * 18 ticks de 55 ms ≈ 39,6 s
        let mut r = Reprodutor::new(seq());
        r.passar_tempo(720 * 55 - 1);
        assert_eq!(r.atual().unwrap().nome, "a");
        r.passar_tempo(1);
        assert_eq!(r.atual().unwrap().nome, "b");
    }

    #[test]
    fn espaco_fecha_mas_outra_tecla_nao() {
        let mut r = Reprodutor::new(seq());
        r.evento(Evento::OutraTecla);
        assert_eq!(r.atual().unwrap().nome, "a");
        r.evento(Evento::Espaco);
        assert_eq!(r.atual().unwrap().nome, "b");
    }

    #[test]
    fn termina_depois_da_ultima_tela() {
        let mut r = Reprodutor::new(seq());
        r.evento(Evento::Clique);
        r.passar_tempo(10 * 55);
        assert!(r.terminou());
    }
}
