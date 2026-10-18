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

/// Jogo novo escolhido durante a partida (`_NewGame` do "Select a Game", também o que vem depois do
/// GAME OVER): o original **não** ressemeia S nem R; `_RandYard` gera o mundo com o R da partida
/// que estava aberta (geracao-mundo.md §1).
pub fn jogo_novo_com_geradores(
    tipo: TipoJogo,
    anterior: &Estado,
    tab: &TabelasGeracao,
    cfg: &ConfigSimulacao,
    quintal: &TabelasQuintal,
    vista: (i32, i32),
) -> Result<Estado, String> {
    let mut r = anterior.r;
    let mut s = anterior.s;
    let m = Mundo::novo_jogo(tipo, &mut r, &mut s, tab)?;
    let mut e = Estado::de_mundo(&m, s, r, cfg)?;
    iniciar_quintal(&mut e, quintal);
    manter_do_anterior(&mut e, anterior);
    // _NewGame: a lição só recomeça no Tutorial (tipo 0); `_GotoMyAnt` põe `_LessonTemp = 1`.
    if tipo != TipoJogo::Tutorial {
        e.tutorial.licao = anterior.tutorial.licao;
        e.tutorial.dada = anterior.tutorial.dada;
    }
    e.tutorial.temporario = 1;
    centrar_na_amarela(&mut e, vista);
    Ok(e)
}

/// `_SetMapPlane(_MePlane)` e `_CenterEdit`: o plano da amarela e `_MapPnt` com a vista de
/// `largura × altura` células centrada nela (a navegação da interface recentra com a vista real).
pub fn centrar_na_amarela(e: &mut Estado, (w, h): (i32, i32)) {
    let plano = e.amarela.plano;
    let largura_plano = if plano < 2 { 128 } else { 64 };
    e.vistas.plano = plano;
    e.vistas.modo_mapa = plano;
    let cx = (i32::from(e.amarela.x) - w / 2).clamp(0, largura_plano - w);
    let cy = (i32::from(e.amarela.y) - h / 2).clamp(0, 64 - h);
    e.vistas.ponto = (cx as i16, cy as i16);
}

/// Os 30 patches de `_GenerateTutorial`: (x, y, pretas/32, vermelhas/32) (tabelas `DGROUP:25f0`,
/// `260e`, `262c`, `264a`; geracao-mundo.md §6).
#[derive(Clone, Debug, serde::Deserialize)]
pub struct ConfigTutorialArquivo {
    /// Nome do arquivo (sem a pasta, sem diferenciar maiúsculas) que gera o tutorial (string 1800).
    pub nome: String,
    pub titulo: String,
    pub patches: Vec<[u8; 4]>,
    /// Plano da amarela depois (`_SetMapPlane(_MePlane)`) fica a cargo da interface.
    pub velocidade: i16,
    pub aviso_saude: i16,
}

/// `_GenerateTutorial` (`1030:6262`) como o Load de `Tutorial.Ant` a chama (o arquivo não é lido):
/// `_CurGameType = 1` e `_RandYard` com o R da partida anterior; 32 pretas e 32 vermelhas novas;
/// `_FullCount`; `_CurGameType = 2`; as populações dos 30 patches; depois `_InitSimVars` e
/// `_SeedSRand` (`semente_s`, do relógio). Devolve o estado.
#[allow(clippy::too_many_arguments)]
pub fn estado_do_tutorial(
    anterior: &Estado,
    semente_s: u16,
    tab: &TabelasGeracao,
    cfg: &ConfigSimulacao,
    quintal: &TabelasQuintal,
    tut: &ConfigTutorialArquivo,
    regras: &mut dyn crate::rules::tick::Regras,
    vista: (i32, i32),
) -> Result<Estado, String> {
    use crate::rules::geracao::Cor;
    let mut r = anterior.r;
    let mut s = GeradorS::new(0);
    let m = Mundo::novo_jogo(TipoJogo::Rapido, &mut r, &mut s, tab)?;
    let mut e = Estado::de_mundo(&m, s, r, cfg)?;
    iniciar_quintal(&mut e, quintal);
    crate::rules::misterio::adicionar_formigas(&mut e, Cor::Preta, 32);
    crate::rules::misterio::adicionar_formigas(&mut e, Cor::Vermelha, 32);
    e.contar_formigas();
    regras.dar_conselhos(&mut e);
    e.partida.tipo_jogo = 2;
    for &[x, y, p, v] in &tut.patches {
        let i = usize::from(x) * 16 + usize::from(y);
        if let Some(c) = e.preta.populacao_quintal.0.get_mut(i) {
            *c = p.wrapping_shl(5);
        }
        if let Some(c) = e.vermelha.populacao_quintal.0.get_mut(i) {
            *c = v.wrapping_shl(5);
        }
    }
    // _InitSimVars.
    e.partida.velocidade = tut.velocidade;
    e.partida.aviso_saude_colonia = tut.aviso_saude;
    e.partida.ferramenta_experimental = 0;
    e.preta.enxame_contagem = 0;
    e.vermelha.enxame_contagem = 0;
    e.preta.rainhas_guardadas = 0;
    e.vermelha.rainhas_guardadas = 0;
    e.tutorial.licao = 1;
    e.tutorial.dada = 0;
    e.s = GeradorS::new(semente_s);
    manter_do_anterior(&mut e, anterior);
    centrar_na_amarela(&mut e, vista);
    Ok(e)
}

/// O que o Load do tutorial **não** escreve (não passa por `_NewGame`): fica como estava na partida
/// anterior [medido na bancada, `tests/referencias/load_tutorial`]. E o que `_RandYard` põe na
/// amarela e o port ainda não põe em `Estado::de_mundo` (alvo e última posição = a posição dela).
fn manter_do_anterior(e: &mut Estado, a: &Estado) {
    for (n, v) in [(&mut e.preta, &a.preta), (&mut e.vermelha, &a.vermelha)] {
        n.modos = v.modos;
        n.colonias_fundadas = v.colonias_fundadas;
        n.total_colonias = v.total_colonias;
        n.modo_estrategico = v.modo_estrategico;
    }
    // Vagas além do fim das listas dos ninhos: X e Y ficam os de antes (T e M zerados).
    for (n, v) in [(&mut e.ninho_b.lista, &a.ninho_b.lista), (&mut e.ninho_r.lista, &a.ninho_r.lista)] {
        let usadas = n.usadas();
        for i in usadas..n.vagas.len().min(v.vagas.len()) {
            n.vagas[i].x = v.vagas[i].x;
            n.vagas[i].y = v.vagas[i].y;
        }
    }
    e.lagarta_e_bichos.bicho_dir[0] = a.lagarta_e_bichos.bicho_dir[0];
    let (b, ab) = (&mut e.bichos, &a.bichos);
    b.gato_dir = ab.gato_dir;
    b.gato_x = ab.gato_x;
    b.gato_y = ab.gato_y;
    b.cachorro_px = ab.cachorro_px;
    b.cachorro_py = ab.cachorro_py;
    e.ambiente.atualizar_colonias = a.ambiente.atualizar_colonias;
    e.partida.modo_edicao = a.partida.modo_edicao;
    e.partida.pausado = a.partida.pausado;
    let (m, am) = (&mut e.amarela, &a.amarela);
    m.difusa_x = am.difusa_x;
    m.difusa_y = am.difusa_y;
    m.contorno_contagem = am.contorno_contagem;
    m.comeu_tipo = am.comeu_tipo;
    m.alvo_plano = am.alvo_plano;
    m.alvo_x = m.x;
    m.alvo_y = m.y;
    m.ultimo_x = m.x;
    m.ultimo_y = m.y;
    e.controles.proxima_casta = a.controles.proxima_casta;
    e.controles.proximo_modo = a.controles.proximo_modo;
    e.aranha.alvo_tipo = a.aranha.alvo_tipo;
    e.aranha.destino_x = a.aranha.destino_x;
    e.aranha.destino_y = a.aranha.destino_y;
}

/// O nome escolhido no Load é o do tutorial?
pub fn e_o_tutorial(caminho: &Path, tut: &ConfigTutorialArquivo) -> bool {
    caminho.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(&tut.nome))
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
