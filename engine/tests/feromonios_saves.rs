//! Regras de feromônio com saves reais do original. Especificação: docs/spec/regras/feromonios.md.
//!
//! - `QUICK2.ANT` (tick 78) e `QUICK3.ANT` (tick 92): leitura e gravação dos cinco mapas e avanço
//!   só das etapas de cheiro (sem formigas), conferindo invariantes.
//! - `tests/referencias/ticks/`: pares "antes/depois de um tick" tirados do original. Sem os
//!   arquivos, o teste avisa e passa sem verificar nada.

use std::path::{Path, PathBuf};

use engine::formats::save::{JogoSalvo, LayoutSave, Lista};
use engine::rules::feromonios::{
    celula_do_tile, fase_do_ciclo, Camada, FaseCheiro, Feromonios, TabelasFeromonio, ALTURA,
    ARQUIVO_TABELAS, CELULAS,
};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn layout() -> LayoutSave {
    LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap()
}

fn tabelas() -> TabelasFeromonio {
    TabelasFeromonio::de_json(
        &std::fs::read_to_string(raiz().join("data").join(ARQUIVO_TABELAS)).unwrap(),
    )
    .unwrap()
}

fn save(nome: &str) -> Option<JogoSalvo> {
    let p = raiz().join("tests/referencias/saves").join(nome);
    if !p.is_file() {
        eprintln!("AVISO: {} não encontrado; teste ignorado", p.display());
        return None;
    }
    Some(JogoSalvo::abrir(&layout(), &p).unwrap())
}

/// Avança só a etapa de cheiro de `_DoAntSim`, de `_Cycle` = `de` + 1 até `ate`, inclusive.
fn avancar(f: &mut Feromonios, s: &JogoSalvo, de: u16, ate: u16) -> Vec<(u16, FaseCheiro)> {
    let tab = tabelas();
    let mapa_a = s.mapa_a().unwrap();
    let hb = s.bloco("HoleMapB").unwrap();
    let hr = s.bloco("HoleMapR").unwrap();
    let mut fases = Vec::new();
    let mut ciclo = de;
    while ciclo != ate {
        ciclo = if ciclo >= 0x1000 { 0 } else { ciclo + 1 };
        if let Some(fase) = f.etapa_do_tick(ciclo, hb, hr, &mapa_a, &tab.regras) {
            fases.push((ciclo, fase));
        }
    }
    fases
}

#[test]
fn leitura_e_gravacao_preservam_o_save() {
    for nome in ["QUICK2.ANT", "QUICK3.ANT"] {
        let Some(mut s) = save(nome) else { continue };
        let original = s.bytes().to_vec();
        let f = Feromonios::do_save(&s).unwrap();
        f.gravar_no_save(&mut s).unwrap();
        assert_eq!(s.bytes(), &original[..], "{nome}");

        // Alterar e gravar muda só o bloco certo.
        let mut g = f.clone();
        g.trilha_vermelha.depositar(64, 32, 99);
        g.gravar_no_save(&mut s).unwrap();
        let diferentes: Vec<_> = s
            .bytes()
            .iter()
            .zip(&original)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .collect();
        assert_eq!(diferentes.len(), 1, "{nome}");
        assert_eq!(Feromonios::do_save(&s).unwrap(), g);
    }
}

#[test]
fn estado_dos_saves_quick() {
    // Spec, seção 5: só a célula do buraco preto tem cheiro (254 = 255 do tick 32 menos 1).
    for (nome, buraco_preto) in [("QUICK2.ANT", (3, 32)), ("QUICK3.ANT", (2, 32))] {
        let Some(s) = save(nome) else { continue };
        let f = Feromonios::do_save(&s).unwrap();
        let i = celula_do_tile(buraco_preto.0, buraco_preto.1).unwrap();
        assert_eq!(f.ninho_preto.celulas[i], 254, "{nome}");
        assert_eq!(
            f.ninho_preto.celulas.iter().filter(|v| **v != 0).count(),
            1,
            "{nome}"
        );
        for c in [
            Camada::Alarme,
            Camada::TrilhaPreta,
            Camada::NinhoVermelho,
            Camada::TrilhaVermelha,
        ] {
            assert!(f.camada(c).celulas.iter().all(|v| *v == 0), "{nome} {c:?}");
        }
    }
}

#[test]
fn quick2_avancado_so_com_cheiro() {
    let Some(s) = save("QUICK2.ANT") else { return };
    let mut f = Feromonios::do_save(&s).unwrap();
    assert_eq!(s.i16("Cycle").unwrap(), 78);
    let preto = celula_do_tile(3, 32).unwrap();
    let vermelho = celula_do_tile(118, 32).unwrap();

    // Ticks 79..=96: a fase vermelha roda no 96 (buraco = 255, menos 1).
    let fases = avancar(&mut f, &s, 78, 96);
    assert_eq!(fases, vec![(96, FaseCheiro::Vermelhos)]);
    assert_eq!(f.ninho_vermelho.celulas[vermelho], 254);
    assert_eq!(f.ninho_preto.celulas[preto], 254);

    // Até 160: alarme no 128, pretos no 160 (o buraco volta a 255 e perde 1).
    let fases = avancar(&mut f, &s, 96, 160);
    assert_eq!(
        fases,
        vec![(128, FaseCheiro::Alarme), (160, FaseCheiro::Pretos)]
    );
    assert_eq!(f.ninho_preto.celulas[preto], 254);

    // Sem formigas, nada além das duas células aparece, mesmo dando a volta no _Cycle.
    let fases = avancar(&mut f, &s, 160, 0x40);
    assert!(
        fases.contains(&(0x1000, FaseCheiro::Alarme)) && fases.contains(&(0, FaseCheiro::Alarme))
    );
    let nao_nulas: usize = Camada::TODAS
        .iter()
        .map(|c| f.camada(*c).celulas.iter().filter(|v| **v != 0).count())
        .sum();
    assert_eq!(nao_nulas, 2);
}

#[test]
fn invariantes_do_decaimento_com_mapas_cheios() {
    // Mapas preenchidos com valores pseudoaleatórios a partir do QUICK2: cada fase só diminui
    // ninho e trilha (fora dos buracos) e o alarme suavizado nunca passa do maior valor anterior.
    let Some(s) = save("QUICK2.ANT") else { return };
    let tab = tabelas();
    let mut f = Feromonios::do_save(&s).unwrap();
    let mut g = engine::rules::rng::GeradorS::new(0xACE1);
    for c in Camada::TODAS {
        for v in f.camada_mut(c).celulas.iter_mut() {
            *v = g.mascara(256) as u8;
        }
    }
    let mapa_a = s.mapa_a().unwrap();
    let hb = s.bloco("HoleMapB").unwrap();
    let hr = s.bloco("HoleMapR").unwrap();
    let buracos = [
        celula_do_tile(3, 32).unwrap(),
        celula_do_tile(118, 32).unwrap(),
    ];
    for ciclo in [32u16, 96, 64] {
        let antes = f.clone();
        let fase = f
            .etapa_do_tick(ciclo, hb, hr, &mapa_a, &tab.regras)
            .unwrap();
        for c in [
            Camada::NinhoPreto,
            Camada::TrilhaPreta,
            Camada::NinhoVermelho,
            Camada::TrilhaVermelha,
        ] {
            for i in 0..CELULAS {
                if buracos.contains(&i) {
                    continue;
                }
                assert!(
                    f.camada(c).celulas[i] <= antes.camada(c).celulas[i],
                    "{fase:?} {c:?} {i}"
                );
            }
        }
        if fase == FaseCheiro::Alarme {
            let max_antes = *antes.alarme.celulas.iter().max().unwrap();
            assert!(f.alarme.celulas.iter().all(|v| *v <= max_antes));
            assert!(f
                .alarme
                .celulas
                .iter()
                .all(|v| *v == 0 || *v >= tab.regras.limiar_alarme));
        } else {
            assert_eq!(f.alarme, antes.alarme);
        }
        if fase == FaseCheiro::Pretos {
            assert!(f.trilha_preta.celulas.iter().all(|v| *v == 0 || *v >= 4));
            assert_eq!(f.trilha_vermelha, antes.trilha_vermelha);
        }
    }
}

// --- vetores "estado antes/depois de 1 tick" do original ---

/// Pares `(antes, depois)` em `tests/referencias/ticks/`: qualquer arquivo `.ANT` cujo nome contém
/// "antes", com o par de mesmo nome trocando "antes" por "depois" (em qualquer subpasta).
fn pares_de_tick(dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut v = Vec::new();
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return v;
    };
    for e in entradas.flatten() {
        let p = e.path();
        if p.is_dir() {
            v.extend(pares_de_tick(&p));
            continue;
        }
        let nome = p.file_name().unwrap().to_string_lossy().to_string();
        if !nome.to_ascii_lowercase().ends_with(".ant") || !nome.contains("antes") {
            continue;
        }
        let depois = p.with_file_name(nome.replace("antes", "depois"));
        if depois.is_file() {
            v.push((p, depois));
        }
    }
    v.sort();
    v
}

/// Células que as formigas podem ter tocado no mesmo tick (depósito, desgaste, alarme de luta):
/// as dos tiles em volta (±1) de cada formiga da lista A, antes e depois, e da formiga amarela.
fn celulas_tocadas_por_formigas(antes: &JogoSalvo, depois: &JogoSalvo) -> Vec<bool> {
    let mut m = vec![false; CELULAS];
    let mut marcar = |x: i32, y: i32| {
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(i) = celula_do_tile(x + dx, y + dy) {
                    m[i] = true;
                }
            }
        }
    };
    for s in [antes, depois] {
        for f in s.formigas(Lista::A).unwrap() {
            marcar(f.x as i32, f.y as i32);
        }
        if s.i16("MePlane").unwrap() == 1 {
            marcar(
                s.i16("MeLocX").unwrap() as i32,
                s.i16("MeLocY").unwrap() as i32,
            );
        }
    }
    m
}

#[test]
fn vetores_de_tick_do_original() {
    let dir = raiz().join("tests/referencias/ticks");
    let pares = pares_de_tick(&dir);
    if pares.is_empty() {
        eprintln!(
            "AVISO: nenhum par antes/depois em {}; teste ignorado",
            dir.display()
        );
        return;
    }
    let tab = tabelas();
    let l = layout();
    for (pa, pd) in pares {
        let antes = JogoSalvo::abrir(&l, &pa).unwrap();
        let depois = JogoSalvo::abrir(&l, &pd).unwrap();
        let ciclo = depois.i16("Cycle").unwrap() as u16;
        let mut f = Feromonios::do_save(&antes).unwrap();
        let fase = f.etapa_do_tick(
            ciclo,
            antes.bloco("HoleMapB").unwrap(),
            antes.bloco("HoleMapR").unwrap(),
            &antes.mapa_a().unwrap(),
            &tab.regras,
        );
        let esperado = Feromonios::do_save(&depois).unwrap();
        let tocadas = celulas_tocadas_por_formigas(&antes, &depois);
        let mut ignoradas = 0;
        let mut erros = Vec::new();
        for c in Camada::TODAS {
            for (i, &tocada) in tocadas.iter().enumerate() {
                let (a, b) = (f.camada(c).celulas[i], esperado.camada(c).celulas[i]);
                if a == b {
                    continue;
                }
                if tocada {
                    ignoradas += 1;
                } else {
                    erros.push(format!(
                        "{c:?} célula ({}, {}): port {a}, original {b}",
                        i / ALTURA,
                        i % ALTURA
                    ));
                }
            }
        }
        eprintln!(
            "{}: _Cycle {ciclo}, fase {fase:?} ({:?}), {ignoradas} diferenças perto de formigas ignoradas",
            pd.display(),
            fase_do_ciclo(ciclo)
        );
        assert!(
            erros.is_empty(),
            "{}: {} diferenças\n{}",
            pd.display(),
            erros.len(),
            erros.join("\n")
        );
    }
}
