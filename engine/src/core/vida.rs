//! Camada de vida (`_LifeA/B/R`): o que está em cada célula do mapa (formiga, ovo, formiga amarela).
//! Os três mapas não são salvos; são reconstruídos a partir das listas de formigas ao carregar.
//! Especificação: docs/spec/telas/jogo.md, seção "Camada de vida".

use super::mapa::{Plano, ALTURA};
use crate::formats::save::{JogoSalvo, Lista};

/// Byte de vida da formiga amarela.
pub const AMARELA: u8 = 0xFF;
/// Byte da segunda metade da formiga amarela quando ela é rainha.
pub const AMARELA_METADE: u8 = 0xFE;
/// `_MeType` da rainha: `_SetMyLife` grava também a segunda metade.
const TIPO_RAINHA: i16 = 0x60;
/// Primeiro índice da formiga amarela no buffer de vida (já descontado o deslocamento de `FUN_1010_63fe`).
const BASE_AMARELA: i32 = 0x100;
/// Primeiro índice da segunda metade (`0x308 − 0x200` ou `0x388 − 0x280`).
const BASE_AMARELA_METADE: i32 = 0x108;

/// Estado da formiga amarela usado no desenho e na reconstrução (blocos `_Me*` do save).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormigaAmarela {
    pub modo: i16,
    pub plano: i16,
    pub x: i16,
    pub y: i16,
    pub tipo: i16,
    pub dir: i16,
    pub ovo: i16,
    pub base: i16,
}

impl FormigaAmarela {
    pub fn do_save(jogo: &JogoSalvo) -> Result<FormigaAmarela, String> {
        Ok(FormigaAmarela {
            modo: jogo.i16("MeMode")?,
            plano: jogo.i16("MePlane")?,
            x: jogo.i16("MeLocX")?,
            y: jogo.i16("MeLocY")?,
            tipo: jogo.i16("MeType")?,
            dir: jogo.i16("MeDir")?,
            ovo: jogo.i16("MeEgg")?,
            base: jogo.i16("MeBase")?,
        })
    }

    /// Índice no buffer de vida para os bytes 0xFF e 0xFE (`_DoCalcTile`).
    fn tile(&self, valor: u8) -> i32 {
        let (base, tipo, dir, ovo) = (self.base as i32, self.tipo as i32, self.dir as i32, self.ovo as i32);
        if valor == AMARELA_METADE {
            BASE_AMARELA_METADE + base + tipo + dir
        } else if tipo < 8 {
            BASE_AMARELA + base + tipo + ovo
        } else {
            BASE_AMARELA + base + tipo + dir
        }
    }
}

/// Plano de `_MapPlane`/`_MePlane` (0 e 1 = superfície, 2 = ninho preto, 3 = ninho vermelho).
pub fn plano_de(n: i16) -> Option<Plano> {
    match n {
        0 | 1 => Some(Plano::Superficie),
        2 => Some(Plano::NinhoPreto),
        3 => Some(Plano::NinhoVermelho),
        _ => None,
    }
}

fn plano_da_lista(l: Lista) -> Plano {
    match l {
        Lista::A => Plano::Superficie,
        Lista::B => Plano::NinhoPreto,
        Lista::R => Plano::NinhoVermelho,
    }
}

/// `_LifeA` (128 × 64), `_LifeB` e `_LifeR` (64 × 64), em colunas (índice x × 64 + y).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CamadasVida {
    pub a: Vec<u8>,
    pub b: Vec<u8>,
    pub r: Vec<u8>,
}

impl CamadasVida {
    pub fn vazia() -> CamadasVida {
        CamadasVida {
            a: vec![0; Plano::Superficie.bytes()],
            b: vec![0; Plano::NinhoPreto.bytes()],
            r: vec![0; Plano::NinhoVermelho.bytes()],
        }
    }

    pub fn do_plano(&self, p: Plano) -> &[u8] {
        match p {
            Plano::Superficie => &self.a,
            Plano::NinhoPreto => &self.b,
            Plano::NinhoVermelho => &self.r,
        }
    }

    fn do_plano_mut(&mut self, p: Plano) -> &mut Vec<u8> {
        match p {
            Plano::Superficie => &mut self.a,
            Plano::NinhoPreto => &mut self.b,
            Plano::NinhoVermelho => &mut self.r,
        }
    }

    pub fn valor(&self, p: Plano, x: usize, y: usize) -> u8 {
        self.do_plano(p)[x * ALTURA + y]
    }

    /// Grava `valor` em (x, y) se a célula existir no plano (como `_SetLife`); devolve se gravou.
    pub fn definir(&mut self, p: Plano, x: i32, y: i32, valor: u8) -> bool {
        if x < 0 || y < 0 || x as usize >= p.largura() || y as usize >= ALTURA {
            return false;
        }
        self.do_plano_mut(p)[x as usize * ALTURA + y as usize] = valor;
        true
    }
}

/// Resultado da reconstrução: as camadas e o que não foi reproduzido.
pub struct Reconstrucao {
    pub camadas: CamadasVida,
    pub pendencias: Vec<String>,
}

/// Reconstrói `_LifeA/B/R` como `_DoLoadInitializations`: zera, percorre cada lista de `_ListIndex?`
/// até 0 (inclusive, decrescendo; a entrada de menor índice vence) e grava a formiga amarela por cima
/// quando `_MeMode` = 0.
pub fn reconstruir(jogo: &JogoSalvo) -> Result<Reconstrucao, String> {
    let mut c = CamadasVida::vazia();
    let mut pendencias = Vec::new();
    for lista in Lista::TODAS {
        let plano = plano_da_lista(lista);
        let contador = jogo.i16(&lista.simbolo_contador())?;
        let cap = jogo.capacidade(lista)?;
        if contador < 0 {
            continue;
        }
        // O original também lê a entrada de índice ListIndex; com ListIndex = capacidade ela cairia
        // no vetor seguinte da memória, o que o port não reproduz.
        let ultimo = (contador as usize).min(cap - 1);
        if contador as usize >= cap {
            pendencias.push(format!("lista {}: ListIndex = {contador} na capacidade", lista.letra()));
        }
        for i in (0..=ultimo).rev() {
            let f = jogo.formiga(lista, i)?;
            if !c.definir(plano, f.x as i32, f.y as i32, f.tipo) {
                pendencias.push(format!("lista {} [{i}]: ({}, {}) fora do mapa", lista.letra(), f.x, f.y));
            }
        }
    }
    let me = FormigaAmarela::do_save(jogo)?;
    if me.modo == 0 {
        definir_amarela(&mut c, &me, &mut pendencias);
    }
    Ok(Reconstrucao { camadas: c, pendencias })
}

/// `_SetMyLife(_MePlane, _MeLocX, _MeLocY, _MeType, _MeDir, 0xFF)`.
fn definir_amarela(c: &mut CamadasVida, me: &FormigaAmarela, pendencias: &mut Vec<String>) {
    let Some(plano) = plano_de(me.plano) else {
        pendencias.push(format!("formiga amarela no plano {} desconhecido", me.plano));
        return;
    };
    if !c.definir(plano, me.x as i32, me.y as i32, AMARELA) {
        return;
    }
    if me.tipo == TIPO_RAINHA {
        // A segunda metade (0xFE) não é gravada aqui: jogo.md (mapa de vida) diz que a tabela de direções
        // não foi localizada. As regras do tick já põem a cauda em `_Dx8/_Dy8[_MeDir ^ 4]`
        // (`rules/amarela.rs`, `cauda`; formiga-amarela.md, `_DoLifeExchange`), que é provavelmente a
        // mesma tabela [hipótese: não conferido contra uma captura da rainha amarela].
        pendencias.push("segunda metade da formiga amarela rainha (tabela de direções em aberto)".into());
    }
}

/// Índice no buffer de vida do plano para um byte de vida, ou None se a célula está vazia.
pub fn tile_de_vida(valor: u8, me: &FormigaAmarela) -> Option<usize> {
    match valor {
        0 => None,
        AMARELA | AMARELA_METADE => usize::try_from(me.tile(valor)).ok(),
        v => Some(v as usize),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn me(tipo: i16, dir: i16, ovo: i16) -> FormigaAmarela {
        FormigaAmarela { modo: 0, plano: 2, x: 0, y: 0, tipo, dir, ovo, base: 0 }
    }

    #[test]
    fn amarela_adulta_usa_direcao_e_cria_usa_ovo() {
        assert_eq!(tile_de_vida(AMARELA, &me(16, 2, 5)), Some(0x112));
        assert_eq!(tile_de_vida(AMARELA, &me(3, 2, 5)), Some(0x108));
        assert_eq!(tile_de_vida(AMARELA_METADE, &me(0x60, 1, 0)), Some(0x108 + 0x61));
        assert_eq!(tile_de_vida(0, &me(16, 2, 0)), None);
        assert_eq!(tile_de_vida(102, &me(16, 2, 0)), Some(102));
    }

    #[test]
    fn definir_respeita_o_tamanho_do_plano() {
        let mut c = CamadasVida::vazia();
        assert!(c.definir(Plano::Superficie, 100, 63, 7));
        assert!(!c.definir(Plano::NinhoPreto, 100, 3, 7));
        assert!(!c.definir(Plano::NinhoPreto, 3, 64, 7));
        assert_eq!(c.valor(Plano::Superficie, 100, 63), 7);
    }
}
