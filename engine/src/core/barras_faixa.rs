//! Barras e números da faixa (`_DrawMapData`, `1010:cb60`): saúde da amarela, saúde e população das
//! pretas e das vermelhas, com a marca do limiar de aviso. Sem janela: desenha num `Quadro`.
//!
//! Especificação: docs/spec/telas/mensagens.md §9 ("Barras e números da faixa"). Dados: data/tela_jogo.json,
//! `barras_faixa` (objetos, valor de cada um e fonte dos números).

use serde::Deserialize;

use super::quadro::Quadro;
use super::telas::Interface;
use crate::formats::janela::{Cor, Retangulo};
use crate::rules::estado::Estado;

/// O que uma barra ou um número mostra.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValorFaixa {
    /// `_MeHealth` / 100.
    SaudeAmarela,
    /// `_HealthB` / 100.
    SaudePreta,
    /// `_HealthR` / 100, ou 0 sem vermelhas (`_RpopT` = 0).
    SaudeVermelha,
    /// `_BpopT` / max(1, `_BpopT`, `_RpopT`).
    PopulacaoPreta,
    /// `_RpopT` / max(1, `_BpopT`, `_RpopT`).
    PopulacaoVermelha,
}

/// Limiar marcado com uma coluna invertida (`_GInvBox`).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarcaFaixa {
    /// `_MeWarnHealth`.
    AvisoSaudeAmarela,
    /// `_BlkWarnHealth`.
    AvisoSaudeColonia,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct BarraFaixa {
    pub objeto: String,
    pub valor: ValorFaixa,
    #[serde(default)]
    pub marca: Option<MarcaFaixa>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct NumeroFaixa {
    pub objeto: String,
    pub valor: ValorFaixa,
}

/// data/tela_jogo.json, `barras_faixa`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct ConfigBarrasFaixa {
    /// Barras horizontais (`_win_DrawHBar`).
    pub barras: Vec<BarraFaixa>,
    /// Números (`_win_PrintfAtObj` com "%d").
    pub numeros: Vec<NumeroFaixa>,
    /// Fonte dos números (`_font_SetFont(3)`).
    pub fonte_numeros: usize,
    /// A área pintada da barra vai de `esq` a `dir − encolher` (inclusive), e a divisão usa a largura
    /// `dir − encolher − esq` [medido na bancada: 64 pixels pintados e divisão por 63].
    pub encolher: i16,
}

/// Os números do estado que a faixa mostra (lidos a cada redesenho).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ValoresFaixa {
    pub saude_amarela: i32,
    pub aviso_amarela: i32,
    pub saude_preta: i32,
    pub aviso_colonia: i32,
    pub saude_vermelha: i32,
    pub populacao_preta: i32,
    pub populacao_vermelha: i32,
}

impl ValoresFaixa {
    pub fn do_estado(e: &Estado) -> ValoresFaixa {
        ValoresFaixa {
            saude_amarela: i32::from(e.amarela.saude),
            aviso_amarela: i32::from(e.amarela.aviso_saude),
            saude_preta: i32::from(e.preta.saude),
            aviso_colonia: i32::from(e.partida.aviso_saude_colonia),
            saude_vermelha: i32::from(e.vermelha.saude),
            populacao_preta: i32::from(e.preta.populacao),
            populacao_vermelha: i32::from(e.vermelha.populacao),
        }
    }

    /// Valor inteiro mostrado (os números da faixa).
    pub fn inteiro(&self, v: ValorFaixa) -> i32 {
        match v {
            ValorFaixa::SaudeAmarela => self.saude_amarela,
            ValorFaixa::SaudePreta => self.saude_preta,
            ValorFaixa::SaudeVermelha => self.saude_vermelha,
            ValorFaixa::PopulacaoPreta => self.populacao_preta,
            ValorFaixa::PopulacaoVermelha => self.populacao_vermelha,
        }
    }

    /// Fração da barra em 1/65536 (`__aFldiv(v << 16, divisor)`).
    pub fn fracao(&self, v: ValorFaixa) -> i64 {
        let maior = 1.max(self.populacao_preta).max(self.populacao_vermelha);
        let (n, d) = match v {
            ValorFaixa::SaudeAmarela => (self.saude_amarela, 100),
            ValorFaixa::SaudePreta => (self.saude_preta, 100),
            ValorFaixa::SaudeVermelha => (if self.populacao_vermelha == 0 { 0 } else { self.saude_vermelha }, 100),
            ValorFaixa::PopulacaoPreta => (self.populacao_preta, maior),
            ValorFaixa::PopulacaoVermelha => (self.populacao_vermelha, maior),
        };
        (i64::from(n) << 16) / i64::from(d)
    }

    pub fn limiar(&self, m: MarcaFaixa) -> i32 {
        match m {
            MarcaFaixa::AvisoSaudeAmarela => self.aviso_amarela,
            MarcaFaixa::AvisoSaudeColonia => self.aviso_colonia,
        }
    }
}

/// `_win_DrawHBar`: com `r` o retângulo do objeto na tela, a barra ocupa as colunas `esq..=fim`
/// (`fim = dir − encolher`) e as linhas `topo..base`. A divisão `s = esq + ⌊largura × fração / 65536⌋`
/// (largura = `fim − esq`): cor de frente em `esq..=s` e de fundo em `s..=fim` (a segunda pintura cobre
/// a coluna `s`; com a barra cheia, `s = fim` e só a primeira acontece).
pub fn desenhar_barra(q: &mut Quadro, r: Retangulo, cor: Cor, fracao: i64, encolher: i16) {
    let (esq, fim) = (i32::from(r.esq), i32::from(r.dir - encolher));
    let largura = i64::from(fim - esq);
    let s = esq + ((largura * fracao) >> 16) as i32;
    let faixa = |x0: i32, x1: i32| Retangulo { esq: x0 as i16, topo: r.topo, dir: (x1 + 1) as i16, base: r.base };
    if esq < s {
        q.preencher(faixa(esq, s), cor.frente, cor.frente);
    }
    if s < fim {
        q.preencher(faixa(s, fim), cor.fundo, cor.fundo);
    }
}

/// `_GInvBox` na coluna `esq + ⌊largura × limiar / 100⌋`, nas linhas da barra.
pub fn desenhar_marca(q: &mut Quadro, r: Retangulo, limiar: i32, encolher: i16) {
    let esq = i32::from(r.esq);
    let largura = i32::from(r.dir - encolher) - esq;
    let x = esq + largura * limiar / 100;
    q.inverter(Retangulo { esq: x as i16, topo: r.topo, dir: (x + 1) as i16, base: r.base });
}

/// Desenha tudo: `objeto(nome)` dá o retângulo na tela e o índice de cor de cada objeto da faixa.
/// Devolve os objetos que não existem na faixa.
pub fn desenhar(
    q: &mut Quadro,
    interface: &Interface,
    cfg: &ConfigBarrasFaixa,
    v: &ValoresFaixa,
    objeto: &dyn Fn(&str) -> Option<(Retangulo, u8)>,
) -> Vec<String> {
    let mut faltam = Vec::new();
    let cor_de = |i: u8| interface.cores.get(usize::from(i)).copied().unwrap_or(Cor { frente: 0, fundo: 15, fundo2: 15 });
    for b in &cfg.barras {
        let Some((r, c)) = objeto(&b.objeto) else {
            faltam.push(b.objeto.clone());
            continue;
        };
        desenhar_barra(q, r, cor_de(c), v.fracao(b.valor), cfg.encolher);
        if let Some(m) = b.marca {
            desenhar_marca(q, r, v.limiar(m), cfg.encolher);
        }
    }
    let fonte = interface.fontes.get(cfg.fonte_numeros).and_then(|f| f.as_ref());
    for n in &cfg.numeros {
        let Some((r, c)) = objeto(&n.objeto) else {
            faltam.push(n.objeto.clone());
            continue;
        };
        let Some(f) = fonte else {
            faltam.push(format!("fonte {}", cfg.fonte_numeros));
            continue;
        };
        let c = cor_de(c);
        q.preencher(r, c.fundo, c.fundo);
        // Centralizado como a mensagem da faixa (`avisos::desenhar_faixa`): na largura `dir − 1 − esq`.
        let texto = Retangulo { base: r.base, ..r.do_recalc() };
        q.texto(texto, f, v.inteiro(n.valor).to_string().as_bytes(), 0, c.frente, c.fundo);
    }
    faltam
}

#[cfg(test)]
mod testes {
    use super::*;

    fn quadro() -> Quadro {
        Quadro::new(700, 120, 8)
    }

    const COR: Cor = Cor { frente: 11, fundo: 8, fundo2: 8 };
    const R: Retangulo = Retangulo { esq: 566, topo: 47, dir: 630, base: 52 };

    /// Primeira coluna com a cor de fundo na linha 48.
    fn divisao(q: &Quadro) -> usize {
        (566..630).find(|&x| q.pixels[48 * q.largura + x] == 8).unwrap_or(630)
    }

    #[test]
    fn barras_batem_com_a_bancada() {
        // barras_1..3 (qemu_barras_faixa.txt): valor → primeira coluna de fundo.
        for (n, d, s) in [(99, 100, 628), (94, 100, 625), (79, 100, 615), (35, 100, 588), (14, 100, 574), (2, 3, 607), (18, 25, 611), (17, 20, 619)] {
            let mut q = quadro();
            desenhar_barra(&mut q, R, COR, (i64::from(n) << 16) / d, 1);
            assert_eq!(divisao(&q), s, "{n}/{d}");
        }
        // Cheia: pinta até a coluna 629; a 630 é a moldura do bitmap.
        let mut q = quadro();
        desenhar_barra(&mut q, R, COR, 1 << 16, 1);
        assert_eq!(divisao(&q), 630);
        assert_eq!(q.pixels[48 * q.largura + 630], 8, "fora da barra");
        // Linhas 47..51.
        assert_eq!(q.pixels[46 * q.largura + 570], 8);
        assert_eq!(q.pixels[51 * q.largura + 570], 11);
        assert_eq!(q.pixels[52 * q.largura + 570], 8);
    }

    #[test]
    fn marca_do_aviso_na_coluna_584() {
        let mut q = quadro();
        desenhar_barra(&mut q, R, COR, (99 << 16) / 100, 1);
        desenhar_marca(&mut q, R, 30, 1);
        assert_eq!(q.pixels[48 * q.largura + 584], 11 ^ 15, "amarelo invertido = azul escuro");
        assert_eq!(q.pixels[48 * q.largura + 583], 11);
    }

    #[test]
    fn vermelha_sem_populacao_tem_saude_zero() {
        let v = ValoresFaixa { saude_vermelha: 80, populacao_vermelha: 0, populacao_preta: 3, ..Default::default() };
        assert_eq!(v.fracao(ValorFaixa::SaudeVermelha), 0);
        assert_eq!(v.fracao(ValorFaixa::PopulacaoPreta), 1 << 16);
        let v = ValoresFaixa::default();
        assert_eq!(v.fracao(ValorFaixa::PopulacaoPreta), 0, "divisor mínimo 1");
    }
}
