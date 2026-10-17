//! File → Load e Save As: o que `_LoadGame` (`1010:9fa0`) faz fora dos blocos do save e a lista
//! de arquivos dos diálogos. Especificação: docs/spec/telas/arquivos.md.

use std::path::{Path, PathBuf};

use crate::rules::estado::{ConfigSimulacao, Estado};
use crate::rules::geracao::{Mundo, TabelasGeracao, TipoJogo};
use crate::rules::quintal::{iniciar_quintal, TabelasQuintal};
use crate::rules::rng::{GeradorR, GeradorS};

/// Jogo novo (`_NewGame` do "Select a Game"): `_initStuff` semeia S e R com o relógio (o port usa
/// `semente` nas duas leituras), `_RandYard` gera o mundo e `_InitSimYard` põe menino, cachorro etc.
/// (geracao-mundo.md). Usado pela interface depois do "Select a Game" (também no fim de jogo).
pub fn estado_de_jogo_novo(
    tipo: TipoJogo,
    semente: u32,
    tab: &TabelasGeracao,
    cfg: &ConfigSimulacao,
    quintal: &TabelasQuintal,
) -> Result<Estado, String> {
    let mut s = GeradorS::new(0);
    let mut r = GeradorR::default();
    r.semear_com_ticks(&mut s, semente, semente);
    let m = Mundo::novo_jogo(tipo, &mut r, &mut s, tab)?;
    let mut e = Estado::de_mundo(&m, s, r, cfg)?;
    iniciar_quintal(&mut e, quintal);
    Ok(e)
}

/// `_LoadGame` chama `_RandYard` antes de ler os blocos: 192 `_RRand` para as sementes dos patches
/// e a geração do patch (11, 8) com S semeado pela semente nova. Os blocos lidos depois
/// sobrescrevem o mundo, mas os geradores ficam como `_RandYard` os deixou. `r` é o gerador R da
/// sessão antes do Load; `tipo` é o `_CurGameType` de antes (o do jogo que estava aberto; 0 no
/// começo do programa). Devolve S e R depois do Load.
pub fn geradores_depois_do_load(r: GeradorR, tipo: TipoJogo, tab: &TabelasGeracao) -> Result<(GeradorS, GeradorR), String> {
    let mut r = r;
    let mut s = GeradorS::new(0);
    Mundo::novo_jogo(tipo, &mut r, &mut s, tab)?;
    Ok((s, r))
}

/// `_TimeTemp` (`9:9b10`) depois do Load: `_LoadGame` grava 300 (o `_RandYard` zera e o valor
/// gravado antes fica de fora).
pub const TEMPO_TUTOR_DEPOIS_DO_LOAD: i64 = 300;

/// Arquivos `.ANT` de uma pasta, em ordem alfabética sem diferenciar maiúsculas (a lista do
/// diálogo comum do Windows, filtro `*.ANT`).
pub fn arquivos_ant(pasta: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(pasta)
        .map(|it| {
            it.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ant")))
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
    v
}

/// Nome digitado no diálogo → caminho: relativo à pasta do diálogo; sem extensão, `.ANT` (como o
/// diálogo comum com a extensão padrão).
pub fn caminho_do_nome(pasta: &Path, nome: &str) -> Option<PathBuf> {
    let nome = nome.trim();
    if nome.is_empty() {
        return None;
    }
    let mut p = PathBuf::from(nome);
    if p.extension().is_none() {
        p.set_extension("ANT");
    }
    Some(if p.is_absolute() { p } else { pasta.join(p) })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn nome_ganha_extensao_e_pasta() {
        let p = caminho_do_nome(Path::new("saves"), "TESTE").unwrap();
        assert_eq!(p, Path::new("saves").join("TESTE.ANT"));
        assert_eq!(caminho_do_nome(Path::new("x"), "a.ant").unwrap(), Path::new("x").join("a.ant"));
        assert_eq!(caminho_do_nome(Path::new("x"), "  "), None);
    }

    #[test]
    fn lista_so_ant_em_ordem() {
        let d = std::env::temp_dir().join(format!("simant_lista_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        for n in ["b.ANT", "A.ant", "c.txt"] {
            std::fs::write(d.join(n), b"x").unwrap();
        }
        let v: Vec<String> = arquivos_ant(&d).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(v, ["A.ant", "b.ANT"]);
        std::fs::remove_dir_all(&d).ok();
    }
}
