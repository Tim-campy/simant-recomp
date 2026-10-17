//! Botão "Mystery" (`_MysteryButton`, `1000:b0bc`): um efeito sorteado com o gerador R, fora do
//! tick (função 8 dos botões configuráveis e objeto 0x112 da janela do mapa). Especificação:
//! docs/spec/regras/misterio.md.

use super::amarela::TabelasAmarela;
use super::comum;
use super::geracao::Cor;
use super::estado::{Estado, Evento};
use crate::formats::save::{Formiga, Lista};
use super::tick::Regras;

/// Efeitos do Mystery, pelo número sorteado (`_RRand(15)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EfeitoMisterio {
    /// 0, 1: vermelhas novas; 2: pretas novas.
    FormigasVermelhas,
    FormigasPretas,
    /// 3: mata vermelhas; 4: mata pretas.
    MataVermelhas,
    MataPretas,
    /// 5: `_AddFood(150, 1)`.
    Comida,
    /// 6: tira a comida da superfície.
    TiraComida,
    /// 7: lista de sons vazia no Windows.
    Nada,
    /// 8 / 9: buracos novos vermelhos / pretos.
    BuracosVermelhos,
    BuracosPretos,
    /// 10: saúde das duas colônias a zero.
    SaudeZero,
    /// 11: alterna `_Invincible`; 12: alterna `_RevSpider`.
    Invencivel,
    AranhaAoContrario,
    /// 13: a piada (quadros 10022 e 10024).
    Piada,
    /// 14: `_MeSMode = 8`.
    Laser,
}

impl EfeitoMisterio {
    pub fn do_sorteio(k: i32) -> EfeitoMisterio {
        use EfeitoMisterio::*;
        match k {
            0 | 1 => FormigasVermelhas,
            2 => FormigasPretas,
            3 => MataVermelhas,
            4 => MataPretas,
            5 => Comida,
            6 => TiraComida,
            8 => BuracosVermelhos,
            9 => BuracosPretos,
            10 => SaudeZero,
            11 => Invencivel,
            12 => AranhaAoContrario,
            13 => Piada,
            14 => Laser,
            _ => Nada,
        }
    }
}

/// `_FullCount` (`1020:07b4`): `_CountAnts` e `_Feedback`.
fn contagem_completa(e: &mut Estado, regras: &mut dyn Regras) {
    e.contar_formigas();
    regras.dar_conselhos(e);
}

/// `_AddBlackAnts(n)` (`1030:6c5a`) e `_AddRedAnts(n)` (`1030:6cfe`): formigas novas na superfície,
/// nas células com `_MapA` < 0x50 e sem vida, y de 16 a 47; as pretas com x de 0 a 63, as vermelhas
/// com x de 127 descendo a 64. Cada uma: `SRand1(10)` < 4 → tipo base 0x30 e modo 2, senão 0x10 e
/// modo 4; tipo = `SRand8()` + base (+ 0x80 nas vermelhas). Para depois de `n` ou com a lista A
/// passando de 999.
pub fn adicionar_formigas(e: &mut Estado, cor: Cor, mut n: i32) {
    let colunas: Vec<i32> = match cor {
        Cor::Preta => (0..64).collect(),
        Cor::Vermelha => (64..128).rev().collect(),
    };
    for x in colunas {
        for y in 16..48 {
            let livre = e.superficie.mapa.em(x, y).unwrap_or(0xFF) < 0x50 && e.superficie.vida.em(x, y) == Some(0);
            if !livre {
                continue;
            }
            let (base, modo) = if (0..=3).contains(&e.s.rand1(10)) { (0x30, 2) } else { (0x10, 4) };
            let mut tipo = e.s.mascara(8) + base;
            if cor == Cor::Vermelha {
                tipo += 0x80;
            }
            let tipo = tipo as u8;
            // O original grava a vida antes de tentar pôr na lista.
            e.superficie.vida.definir(x, y, tipo);
            e.adicionar_formiga(Lista::A, Formiga { x: x as u8, y: y as u8, modo, tipo, s: 0 });
            n -= 1;
            if n < 1 || e.superficie.lista.indice > 999 {
                return;
            }
        }
    }
}

/// `_IsItYellow(1, x, y)` (`1020:96b6`): a amarela está no plano 1, não é a aranha e a vida da
/// célula é a marca dela.
fn e_a_amarela_na_superficie(e: &Estado, x: i32, y: i32) -> bool {
    e.amarela.plano == 1 && e.amarela.modo != 1 && e.superficie.vida.em(x, y).is_some_and(comum::e_amarela)
}

/// `_KillSomeAnts(cor)` (`1000:9d5e`): compacta a lista A e recenseia; depois, do fim da lista para
/// o começo, mata as formigas da cor (vermelhas: tipo com o bit 7, menos a amarela; pretas: sem o
/// bit 7) com `_DeadAntHere` e zera o tipo, até 51 mortes; compacta e recenseia de novo.
pub fn matar_formigas(e: &mut Estado, regras: &mut dyn Regras, cor: Cor) {
    e.superficie.lista.compactar();
    contagem_completa(e, regras);
    let mut mortas = 0;
    let mut i = e.superficie.lista.indice;
    while i > 0 {
        i -= 1;
        let f = e.superficie.lista.vagas[i as usize];
        let (x, y) = (i32::from(f.x), i32::from(f.y));
        let vermelha = f.tipo & 0x80 != 0;
        let alvo = match cor {
            Cor::Preta => !vermelha,
            Cor::Vermelha => vermelha && !e_a_amarela_na_superficie(e, x, y),
        };
        if !alvo {
            continue;
        }
        comum::cadaver_aqui(e, x, y, cor == Cor::Vermelha);
        e.superficie.lista.vagas[i as usize].tipo = 0;
        mortas += 1;
        if mortas > 50 {
            break;
        }
    }
    e.superficie.lista.compactar();
    contagem_completa(e, regras);
}

/// Número de formigas que o Mystery 0–2 acrescenta: com `_CommandKey` = 1 completa a lista até
/// 1000; o port não tem a tecla de comando, então 32.
fn quantas(e: &Estado) -> Option<i32> {
    (e.superficie.lista.indice < 1000).then_some(32)
}

/// `_MysteryButton`: sorteia `_RRand(15)` e aplica o efeito. Devolve o efeito.
pub fn botao_misterio(e: &mut Estado, regras: &mut dyn Regras, t: &TabelasAmarela) -> EfeitoMisterio {
    let efeito = EfeitoMisterio::do_sorteio(e.r.rrand(15));
    use EfeitoMisterio::*;
    match efeito {
        FormigasVermelhas | FormigasPretas => {
            let cor = if efeito == FormigasVermelhas { Cor::Vermelha } else { Cor::Preta };
            for _ in 0..2 {
                if let Some(n) = quantas(e) {
                    adicionar_formigas(e, cor, n);
                }
                contagem_completa(e, regras);
            }
        }
        MataVermelhas | MataPretas => {
            let cor = if efeito == MataVermelhas { Cor::Vermelha } else { Cor::Preta };
            matar_formigas(e, regras, cor);
            matar_formigas(e, regras, cor);
        }
        Comida => regras.adicionar_comida(e, 150, true),
        TiraComida => {
            e.pedir_som(Some(0x20));
            let casa = comum::em_casa(e);
            for x in 0..128 {
                for y in 0..64 {
                    if e.superficie.mapa.em(x, y).is_some_and(|v| comum::e_comida_superficie(casa, v)) {
                        let v = e.s.mascara(16) as u8;
                        e.superficie.mapa.definir(x, y, v);
                    }
                }
            }
            e.ambiente.comida_superficie = 0;
        }
        Nada => {}
        BuracosVermelhos | BuracosPretos => {
            let cor = if efeito == BuracosVermelhos { Cor::Vermelha } else { Cor::Preta };
            for c in (0..64).step_by(2) {
                comum::novo_buraco(e, t, &t.borda_buraco, cor, c);
            }
        }
        SaudeZero => {
            e.partida.sempre_saudavel = 0;
            e.preta.saude = 0;
            e.vermelha.saude = 0;
        }
        Invencivel => {
            let t = &mut e.partida.invencivel;
            *t = i16::from(*t == 0);
            let som = if *t != 0 { 2 } else { 1 };
            e.pedir_som(Some(som));
        }
        AranhaAoContrario => {
            let t = &mut e.aranha.de_re;
            *t = i16::from(*t == 0);
        }
        Piada => {
            // Quadros não forçados: só com a opção Events (data/mensagens.json, `nao_forcadas`).
            e.eventos.push(Evento::Mensagem(10022));
            e.eventos.push(Evento::Mensagem(10024));
        }
        Laser => {
            e.amarela.modo_aranha = 8;
            e.pedir_som(Some(2));
        }
    }
    efeito
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::rules::rng::{GeradorR, GeradorS};
    use crate::rules::tick::{RegrasAmarela, RegrasConselhos, RegrasInimigos, RegrasNinho, RegrasQuintal, RegrasSuperficie};

    struct SemRegras;
    impl RegrasAmarela for SemRegras {}
    impl RegrasNinho for SemRegras {}
    impl RegrasQuintal for SemRegras {}
    impl RegrasSuperficie for SemRegras {}
    impl RegrasInimigos for SemRegras {}
    impl RegrasConselhos for SemRegras {}

    fn estado() -> Estado {
        let mut e = Estado::vazio();
        e.s = GeradorS::new(0x1234);
        e.r = GeradorR::default();
        e
    }

    #[test]
    fn vermelhas_novas_da_direita_para_a_esquerda_com_o_bit_7() {
        let mut e = estado();
        adicionar_formigas(&mut e, Cor::Vermelha, 32);
        let l = &e.superficie.lista;
        assert_eq!(l.indice, 32);
        let primeira = l.vagas[0];
        assert_eq!((primeira.x, primeira.y), (127, 16));
        assert!(l.vagas[..32].iter().all(|f| f.tipo & 0x80 != 0 && f.x == 127 && (16..48).contains(&f.y)));
        assert_eq!(e.superficie.vida.em(127, 16), Some(primeira.tipo));
        // Dois sorteios de S por formiga.
        let mut s = GeradorS::new(0x1234);
        for _ in 0..32 {
            s.rand1(10);
            s.mascara(8);
        }
        assert_eq!(e.s, s);
    }

    #[test]
    fn mata_no_maximo_51_do_fim_da_lista_e_poupa_a_amarela() {
        let mut e = estado();
        adicionar_formigas(&mut e, Cor::Vermelha, 60);
        // A amarela por cima da última vermelha da lista (só para o teste de `_IsItYellow`).
        let ultima = e.superficie.lista.vagas[59];
        e.amarela.plano = 1;
        e.superficie.vida.definir(i32::from(ultima.x), i32::from(ultima.y), 0xFF);
        matar_formigas(&mut e, &mut SemRegras, Cor::Vermelha);
        assert_eq!(e.superficie.lista.indice, 60 - 51);
        assert!(e.superficie.lista.vagas[..9].iter().any(|f| (f.x, f.y) == (ultima.x, ultima.y)), "a amarela fica");
        matar_formigas(&mut e, &mut SemRegras, Cor::Preta);
        assert_eq!(e.superficie.lista.indice, 9, "pretas: não há");
    }

    #[test]
    fn um_sorteio_de_r_escolhe_o_efeito() {
        let mut e = estado();
        let mut r = e.r;
        let k = r.rrand(15);
        let efeito = botao_misterio(&mut e, &mut SemRegras, TabelasAmarela::padrao());
        assert_eq!(efeito, EfeitoMisterio::do_sorteio(k));
        assert_eq!(e.r, r, "só um _RRand(15)");
    }
}
