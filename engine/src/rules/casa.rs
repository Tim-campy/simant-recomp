//! Patches da casa (`_MakeHousePatch`, `1020:3120`): o chão da casa é montado por uma receita de
//! operações por patch, lida de `data/casa.json` (gerado por `tools/extrair_casa.py`).
//! Especificação: docs/spec/regras/geracao-mundo.md §5.
//!
//! Só o `_MapA` e o `_DROPdir` mudam aqui; o resto de `_RandWorld` (ninhos, formigas, comida) é o
//! mesmo do quintal (`geracao.rs`). As operações rodam em ordem, e os sorteios do gerador S saem na
//! ordem em que as operações `sortear` os pedem.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use super::geracao::{LADO, LARGURA_SUPERFICIE};
use super::rng::GeradorS;
use crate::recursos::Recursos;

/// Arquivo de dados com as formas e as receitas.
pub const ARQUIVO_TABELAS: &str = "casa.json";
/// Patches da casa: `n = px * 16 + py` de 0 a 37 (`_MakeMap`).
pub const PATCHES_CASA: usize = 38;
/// Profundidade máxima de rotinas chamadas por rotinas (evita laço num mod mal feito).
const PROFUNDIDADE_MAXIMA: usize = 8;

/// Desenho fixo copiado para o mapa: célula `(x + i, y + j)` recebe `celulas[j * largura + i]`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Forma {
    pub largura: i32,
    pub altura: i32,
    /// Zeros não são copiados (`_MakeClip`).
    #[serde(default)]
    pub transparente: bool,
    pub celulas: Vec<u8>,
}

/// Uma operação sobre o `_MapA` (geracao-mundo.md §5.1). Intervalos `[a, b]` são inclusivos.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operacao {
    /// Retângulo inteiro com `v`.
    Preencher { x: [i32; 2], y: [i32; 2], v: u8 },
    /// Uma célula.
    Celula { x: i32, y: i32, v: u8 },
    /// Para cada célula (x por fora, y por dentro): se `S(chance) = 0`, `base + S(variantes)`.
    Sortear { x: [i32; 2], y: [i32; 2], chance: u16, base: u8, variantes: u16 },
    /// Moldura (`_TileFrame1/2`): linha y1, linha y2, coluna x1, coluna x2, depois os cantos
    /// (x1, y1), (x2, y1), (x1, y2), (x2, y2).
    Moldura { x: [i32; 2], y: [i32; 2], y1: u8, y2: u8, x1: u8, x2: u8, cantos: [u8; 4] },
    /// Nas colunas x1, x1 + passo, ... ≤ x2 e linhas y1..y2: `se` vira `entao`; qualquer outro valor
    /// vira `senao`.
    Trocar { x: [i32; 2], y: [i32; 2], passo_x: i32, se: u8, entao: u8, senao: u8 },
    /// Copia a forma com o canto em (x, y).
    Forma { nome: String, x: i32, y: i32 },
    /// Ladrilhos do mapa inteiro (`_FloorTiles`), com quadrados de `lado` (potência de 2).
    Ladrilhos { lado: i32, par: u8, impar: u8, junta_y: [u8; 2], junta_x: [u8; 2], cruzamento: [u8; 2] },
    /// `_DROPdir` = valor.
    Queda { valor: u8 },
    /// Roda as operações da rotina com esse nome.
    Rotina { nome: String },
}

/// Conteúdo de `data/casa.json`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct TabelasCasa {
    pub formas: HashMap<String, Forma>,
    pub rotinas: HashMap<String, Vec<Operacao>>,
    /// Receita de cada patch `n` = 0..37.
    pub patches: Vec<Vec<Operacao>>,
    /// Passo da comida que cai (`_DropFoodA`): tabelas `10:22be` (x) e `10:22c2` (y) indexadas por
    /// `_DROPdir`, somadas **sem sinal** (0xFF = +255 sai do mapa no primeiro passo).
    #[serde(default = "direcoes_queda_padrao")]
    pub direcoes_queda: DirecoesQueda,
}

/// `dx` e `dy` (bytes) por `_DROPdir` 0..3.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct DirecoesQueda {
    pub dx: [u8; 4],
    pub dy: [u8; 4],
}

fn direcoes_queda_padrao() -> DirecoesQueda {
    DirecoesQueda { dx: [0, 1, 0, 255], dy: [255, 0, 1, 0] }
}

impl TabelasCasa {
    /// Passo (dx, dy) da comida que cai com este `_DROPdir` (fora de 0..3: o de 0).
    pub fn passo_queda(&self, dir: u8) -> (u8, u8) {
        let d = &self.direcoes_queda;
        let i = usize::from(dir).min(3);
        (d.dx[i], d.dy[i])
    }
}

impl Default for TabelasCasa {
    /// O padrão do port: o `data/casa.json` embutido.
    fn default() -> TabelasCasa {
        TabelasCasa::padrao().clone()
    }
}

/// Mapa de superfície em que a casa é desenhada (`_MapA`, índice `x * 64 + y`) e o `_DROPdir`.
pub struct ChaoDaCasa<'a> {
    pub mapa_a: &'a mut [u8],
    pub queda: &'a mut u8,
}

fn dentro(x: i32, y: i32) -> bool {
    (0..LARGURA_SUPERFICIE).contains(&x) && (0..LADO).contains(&y)
}

fn intervalo_valido(r: [i32; 2], max: i32) -> bool {
    0 <= r[0] && r[0] <= r[1] && r[1] < max
}

impl TabelasCasa {
    pub fn carregar(r: &Recursos) -> Result<TabelasCasa, String> {
        let t: TabelasCasa = r.dados(ARQUIVO_TABELAS)?;
        t.validar()?;
        Ok(t)
    }

    pub fn de_json(texto: &str) -> Result<TabelasCasa, String> {
        let t: TabelasCasa = serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_TABELAS}: {e}"))?;
        t.validar()?;
        Ok(t)
    }

    /// `data/casa.json` embutido no executável.
    pub fn padrao() -> &'static TabelasCasa {
        static TABELAS: OnceLock<TabelasCasa> = OnceLock::new();
        TABELAS.get_or_init(|| TabelasCasa::de_json(include_str!("../../../data/casa.json")).expect("invalid data/casa.json"))
    }

    /// Confere tamanhos, nomes e que nenhuma operação escreve fora do mapa.
    fn validar(&self) -> Result<(), String> {
        let erro = |m: String| Err(format!("{ARQUIVO_TABELAS}: {m}"));
        if self.patches.len() != PATCHES_CASA {
            return erro(format!("expected {PATCHES_CASA} patches, got {}", self.patches.len()));
        }
        for (nome, f) in &self.formas {
            if f.largura <= 0 || f.altura <= 0 || f.celulas.len() != (f.largura * f.altura) as usize {
                return erro(format!("shape {nome}: {} cells for {} × {}", f.celulas.len(), f.largura, f.altura));
            }
        }
        let ops = self.rotinas.iter().flat_map(|(n, v)| v.iter().map(move |o| (format!("routine {n}"), o)));
        let ops = ops.chain(self.patches.iter().enumerate().flat_map(|(n, v)| v.iter().map(move |o| (format!("patch {n}"), o))));
        for (onde, op) in ops {
            if let Err(m) = self.validar_operacao(op) {
                return erro(format!("{onde}: {m}"));
            }
        }
        // Rotinas que se chamam em laço falham aqui, e não no meio de uma geração.
        for nome in self.rotinas.keys() {
            self.profundidade(nome, 0).map_err(|m| format!("{ARQUIVO_TABELAS}: {m}"))?;
        }
        Ok(())
    }

    fn profundidade(&self, nome: &str, nivel: usize) -> Result<(), String> {
        if nivel > PROFUNDIDADE_MAXIMA {
            return Err(format!("routine {nome}: routines nested too deep (loop?)"));
        }
        for op in self.rotinas.get(nome).into_iter().flatten() {
            if let Operacao::Rotina { nome } = op {
                self.profundidade(nome, nivel + 1)?;
            }
        }
        Ok(())
    }

    fn validar_operacao(&self, op: &Operacao) -> Result<(), String> {
        let (w, h) = (LARGURA_SUPERFICIE, LADO);
        let ret = |x: [i32; 2], y: [i32; 2]| {
            if intervalo_valido(x, w) && intervalo_valido(y, h) {
                Ok(())
            } else {
                Err(format!("rectangle x {x:?}, y {y:?} outside the {w} × {h} map"))
            }
        };
        match op {
            Operacao::Preencher { x, y, .. } | Operacao::Moldura { x, y, .. } => ret(*x, *y),
            Operacao::Sortear { x, y, chance, variantes, .. } => {
                if *chance == 0 || *variantes == 0 {
                    return Err("sortear with chance or variantes 0 (S(0) divides by zero in the original)".into());
                }
                ret(*x, *y)
            }
            Operacao::Trocar { x, y, passo_x, .. } => {
                if *passo_x <= 0 {
                    return Err("trocar with passo_x ≤ 0".into());
                }
                ret(*x, *y)
            }
            Operacao::Celula { x, y, .. } => {
                if dentro(*x, *y) {
                    Ok(())
                } else {
                    Err(format!("cell ({x}, {y}) outside the map"))
                }
            }
            Operacao::Forma { nome, x, y } => {
                let f = self.formas.get(nome).ok_or(format!("shape {nome} does not exist"))?;
                ret([*x, x + f.largura - 1], [*y, y + f.altura - 1])
            }
            Operacao::Ladrilhos { lado, .. } => {
                if *lado > 0 && (*lado as u32).is_power_of_two() {
                    Ok(())
                } else {
                    Err(format!("ladrilhos with side {lado} (must be a power of 2)"))
                }
            }
            Operacao::Rotina { nome } => self.rotinas.get(nome).map(|_| ()).ok_or(format!("routine {nome} does not exist")),
            Operacao::Queda { .. } => Ok(()),
        }
    }

    /// Monta o chão do patch `n` (0..37). Não mexe em `_TERRAINset`, formigas-leão nem bichos.
    pub fn gerar(&self, n: usize, chao: &mut ChaoDaCasa, s: &mut GeradorS) -> Result<(), String> {
        let receita = self.patches.get(n).ok_or(format!("house patch {n} out of 0..{}", PATCHES_CASA - 1))?;
        self.executar(receita, chao, s, 0)
    }

    fn executar(&self, ops: &[Operacao], c: &mut ChaoDaCasa, s: &mut GeradorS, nivel: usize) -> Result<(), String> {
        if nivel > PROFUNDIDADE_MAXIMA {
            return Err("routines nested too deep".into());
        }
        let ix = |x: i32, y: i32| (x * LADO + y) as usize;
        for op in ops {
            match op {
                Operacao::Preencher { x, y, v } => {
                    for cx in x[0]..=x[1] {
                        for cy in y[0]..=y[1] {
                            c.mapa_a[ix(cx, cy)] = *v;
                        }
                    }
                }
                Operacao::Celula { x, y, v } => c.mapa_a[ix(*x, *y)] = *v,
                Operacao::Sortear { x, y, chance, base, variantes } => {
                    for cx in x[0]..=x[1] {
                        for cy in y[0]..=y[1] {
                            if s.rand1(*chance) == 0 {
                                c.mapa_a[ix(cx, cy)] = base.wrapping_add(s.rand1(*variantes) as u8);
                            }
                        }
                    }
                }
                Operacao::Moldura { x, y, y1, y2, x1, x2, cantos } => {
                    for cx in x[0]..=x[1] {
                        c.mapa_a[ix(cx, y[0])] = *y1;
                    }
                    for cx in x[0]..=x[1] {
                        c.mapa_a[ix(cx, y[1])] = *y2;
                    }
                    for cy in y[0]..=y[1] {
                        c.mapa_a[ix(x[0], cy)] = *x1;
                    }
                    for cy in y[0]..=y[1] {
                        c.mapa_a[ix(x[1], cy)] = *x2;
                    }
                    c.mapa_a[ix(x[0], y[0])] = cantos[0];
                    c.mapa_a[ix(x[1], y[0])] = cantos[1];
                    c.mapa_a[ix(x[0], y[1])] = cantos[2];
                    c.mapa_a[ix(x[1], y[1])] = cantos[3];
                }
                Operacao::Trocar { x, y, passo_x, se, entao, senao } => {
                    for cx in (x[0]..=x[1]).step_by(*passo_x as usize) {
                        for cy in y[0]..=y[1] {
                            let v = &mut c.mapa_a[ix(cx, cy)];
                            *v = if *v == *se { *entao } else { *senao };
                        }
                    }
                }
                Operacao::Forma { nome, x, y } => {
                    let f = self.formas.get(nome).ok_or(format!("shape {nome} does not exist"))?;
                    for j in 0..f.altura {
                        for i in 0..f.largura {
                            let v = f.celulas[(j * f.largura + i) as usize];
                            if v != 0 || !f.transparente {
                                c.mapa_a[ix(x + i, y + j)] = v;
                            }
                        }
                    }
                }
                Operacao::Ladrilhos { lado, par, impar, junta_y, junta_x, cruzamento } => {
                    let l = *lado;
                    for cx in 0..LARGURA_SUPERFICIE {
                        for cy in 0..LADO {
                            let claro = (cx + cy) & l == 0;
                            let i = if claro { 0 } else { 1 };
                            c.mapa_a[ix(cx, cy)] = match (cx % l == 0, cy % l == 0) {
                                (true, true) => cruzamento[i],
                                (true, false) => junta_x[i],
                                (false, true) => junta_y[i],
                                (false, false) => {
                                    if (cx / l + cy / l) % 2 == 0 {
                                        *par
                                    } else {
                                        *impar
                                    }
                                }
                            };
                        }
                    }
                }
                Operacao::Queda { valor } => *c.queda = *valor,
                Operacao::Rotina { nome } => {
                    let r = self.rotinas.get(nome).ok_or(format!("routine {nome} does not exist"))?;
                    self.executar(r, c, s, nivel + 1)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn gerar(n: usize, semente: u16) -> (Vec<u8>, u8, GeradorS) {
        let mut mapa = vec![0xAAu8; (LARGURA_SUPERFICIE * LADO) as usize];
        let mut queda = 0xFF;
        let mut s = GeradorS::new(semente);
        TabelasCasa::padrao().gerar(n, &mut ChaoDaCasa { mapa_a: &mut mapa, queda: &mut queda }, &mut s).unwrap();
        (mapa, queda, s)
    }

    fn em(m: &[u8], x: i32, y: i32) -> u8 {
        m[(x * LADO + y) as usize]
    }

    #[test]
    fn dados_embutidos_validos() {
        let t = TabelasCasa::padrao();
        assert_eq!(t.patches.len(), 38);
        assert_eq!(t.formas["botao"].celulas[12], 0x4C);
    }

    #[test]
    fn ladrilhos_como_floor_tiles() {
        // _FloorTiles: passos 1 a 3 da spec, conferidos célula a célula.
        let (m, queda, _) = gerar(2, 1);
        assert_eq!(queda, 0);
        for x in 0..128 {
            for y in 0..64 {
                let mut v = if ((y >> 4) + (x >> 4)) & 1 == 0 { 1 } else { 0 };
                if y % 16 == 0 {
                    v = if (x + y) & 0x10 == 0 { 0x61 } else { 0x60 };
                }
                if x % 16 == 0 {
                    v = match (v < 2, (x + y) & 0x10 == 0) {
                        (true, true) => 0x5F,
                        (true, false) => 0x5E,
                        (false, true) => 0x5C,
                        (false, false) => 0x5D,
                    };
                }
                assert_eq!(em(&m, x, y), v, "({x}, {y})");
            }
        }
    }

    #[test]
    fn todo_patch_cobre_o_mapa_inteiro() {
        for n in 0..38 {
            let (m, queda, _) = gerar(n, 0x1234);
            assert!(!m.contains(&0xAA), "patch {n} deixou célula sem escrever");
            assert!(queda <= 3, "patch {n}: queda {queda}");
        }
    }

    #[test]
    fn mod_com_rotina_em_laco_e_recusado() {
        let mut j: serde_json::Value = serde_json::from_str(include_str!("../../../data/casa.json")).unwrap();
        j["rotinas"]["pia"] = serde_json::json!([{"op": "rotina", "nome": "pia"}]);
        assert!(TabelasCasa::de_json(&j.to_string()).is_err());
        j["rotinas"]["pia"] = serde_json::json!([{"op": "celula", "x": 128, "y": 0, "v": 1}]);
        assert!(TabelasCasa::de_json(&j.to_string()).is_err());
    }
}
