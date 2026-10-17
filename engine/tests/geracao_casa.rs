//! Patches da casa (`_MakeHousePatch`, docs/spec/regras/geracao-mundo.md §5). Os valores esperados
//! foram tirados à mão da especificação (e do pseudocódigo/desmontagem citados nela), não do port.
//! Não precisa dos arquivos do jogo: a tabela de seno é sintética e as tabelas vêm de data/.

use std::path::PathBuf;

use engine::rules::casa::{ChaoDaCasa, TabelasCasa};
use engine::rules::estado::{ConfigSimulacao, Estado};
use engine::rules::geracao::{Mundo, TabelasGeracao, TipoJogo};
use engine::rules::rng::{GeradorR, GeradorS};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn tabelas() -> TabelasGeracao {
    let seno: Vec<u8> = (0..64)
        .flat_map(|i| ((((i as f64) * std::f64::consts::PI / 128.0).sin() * 32767.0).round() as i16).to_be_bytes())
        .collect();
    let mut t = TabelasGeracao::de_partes(&std::fs::read_to_string(raiz().join("data/geracao.json")).unwrap(), &seno)
        .unwrap();
    t.casa = TabelasCasa::de_json(&std::fs::read_to_string(raiz().join("data/casa.json")).unwrap()).unwrap();
    t
}

/// Só o chão do patch `n` (sem o resto de `_RandWorld`).
fn chao(n: usize, semente: u16) -> (Vec<u8>, u8, GeradorS) {
    let mut mapa = vec![0u8; 128 * 64];
    let mut queda = 0xFF;
    let mut s = GeradorS::new(semente);
    tabelas().casa.gerar(n, &mut ChaoDaCasa { mapa_a: &mut mapa, queda: &mut queda }, &mut s).unwrap();
    (mapa, queda, s)
}

fn em(m: &[u8], x: i32, y: i32) -> u8 {
    m[(x * 64 + y) as usize]
}

/// Combinações que o original produz: `_XferPatch` só existe no Jogo Completo (com ou sem o ninho
/// da amarela iniciado); nos outros tipos, ao menos uma colônia preta (sem ela, a operária amarela
/// nasceria em (0, 0) do ninho, em terra: `_DigMyTile` não especificado, lacuna anterior à casa).
#[test]
fn todos_os_patches_da_casa_geram_em_todos_os_tipos() {
    let t = tabelas();
    let casos = [
        (TipoJogo::Completo, false, 0, 0),
        (TipoJogo::Completo, false, 1, 1),
        (TipoJogo::Completo, true, 1, 0),
        (TipoJogo::Completo, true, 8, 3),
        (TipoJogo::Tutorial, true, 1, 1),
        (TipoJogo::Rapido, true, 1, 1),
        (TipoJogo::Experimental, true, 0, 2),
    ];
    for (tipo, ninho, pretas, vermelhas) in casos {
        for n in 0..38 {
            let (px, py) = (n / 16, n % 16);
            {
                let mut m = Mundo::vazio(tipo);
                m.ninho_iniciado = ninho;
                let mut s = GeradorS::new(0x2000 + n as u16);
                m.gerar_patch(&mut s, &t, (0x0111u16.wrapping_mul(n as u16 + 1)) & 0x7FFF, pretas, vermelhas, px, py)
                    .unwrap_or_else(|e| panic!("{tipo:?} patch ({px}, {py}) = {n}, {pretas}/{vermelhas}: {e}"));
                assert_eq!(m.terreno, 1, "patch {n}");
                assert!(m.formigas_leao.is_empty() && m.formigas_leao_iniciais == 0, "_InitAntLions(0)");
                assert!(m.bichos_de_conta.iter().all(|b| b.x == 0 && b.y == 0), "sem bichos-de-conta na casa");
            }
        }
    }
}

#[test]
fn mesma_semente_mesma_casa() {
    let t = tabelas();
    let gerar = || {
        let mut m = Mundo::vazio(TipoJogo::Completo);
        let mut s = GeradorS::new(7);
        m.gerar_patch(&mut s, &t, 0x0ABC, 2, 2, 0, 4).unwrap();
        (m.mapa_a.clone(), m.mapa_b.clone(), s)
    };
    assert_eq!(gerar(), gerar());
}

#[test]
fn estado_da_casa_tem_barreira_0x90() {
    let t = tabelas();
    let cfg = ConfigSimulacao::de_json(&std::fs::read_to_string(raiz().join("data/simulacao.json")).unwrap()).unwrap();
    let mut m = Mundo::vazio(TipoJogo::Completo);
    let mut s = GeradorS::new(3);
    m.gerar_patch(&mut s, &t, 0x0123, 1, 1, 1, 2).unwrap();
    let e = Estado::de_mundo(&m, s, GeradorR::default(), &cfg).unwrap();
    assert_eq!(e.ambiente.terreno, 1);
    assert_eq!(e.ambiente.barreira, 0x90);
}

#[test]
fn parede_da_cozinha_e_pia_no_patch_0() {
    let (m, queda, s) = chao(0, 0x55);
    assert_eq!(queda, 2);
    assert_eq!(s, GeradorS::new(0x55), "a cozinha não sorteia");
    // _MakeKitchenWall: linhas 0, 8 e 16 = 0x68; colunas múltiplas de 8 trocadas (0x62 → 0x66, outro → 0x67);
    // linha 23: 0x62 → 0x68, outro → 0x69. Abaixo da linha 23, 0.
    assert_eq!(em(&m, 1, 0), 0x68);
    assert_eq!(em(&m, 0, 0), 0x67);
    assert_eq!(em(&m, 0, 1), 0x66);
    assert_eq!(em(&m, 1, 23), 0x68);
    assert_eq!(em(&m, 8, 23), 0x69);
    assert_eq!(em(&m, 3, 5), 0x62);
    assert_eq!(em(&m, 0, 24), 0);
    assert_eq!(em(&m, 127, 63), 0);
    // _MakeOutletV(0x24, 2): moldura 1 em x 36..44, y 2..14; centro (40, 8) = 0x65; tomada em (38, 4).
    assert_eq!(em(&m, 36, 2), 0x53);
    assert_eq!(em(&m, 44, 14), 0x52);
    assert_eq!(em(&m, 40, 8), 0x65);
    assert_eq!(em(&m, 38, 4), 0x6B);
    assert_eq!(em(&m, 39, 5), 0x78);
    assert_eq!(em(&m, 88, 8), 0x65, "segunda tomada (0x54, 2)");
    // _MakeSink: moldura 1 por fora (37..91, 26..56), cubas, torneira e botões.
    assert_eq!(em(&m, 37, 26), 0x53);
    assert_eq!(em(&m, 91, 56), 0x52);
    assert_eq!(em(&m, 38, 27), 0x01);
    assert_eq!(em(&m, 40, 32), 0x56, "moldura 2 da cuba esquerda");
    assert_eq!(em(&m, 50, 45), 0xC1);
    assert_eq!(em(&m, 41, 34), 0xC2);
    assert_eq!(em(&m, 44, 27), 0x40, "botão (44, 27)");
    assert_eq!(em(&m, 46, 29), 0x4C, "centro do botão");
    assert_eq!(em(&m, 62, 29), 0x4D, "centro do botão (60, 27) trocado");
    assert_eq!(em(&m, 52, 30), 0x43);
    assert_eq!(em(&m, 54, 30), 0x4E);
    assert_eq!(em(&m, 56, 30), 0x44);
    assert_eq!((em(&m, 52, 40), em(&m, 56, 40)), (0x45, 0x48));
    assert_eq!(em(&m, 55, 42), 0xC2);
    assert_eq!(em(&m, 58, 35), 0xC2);
}

#[test]
fn carpete_esquerdo_e_seus_fiapos() {
    let semente = 0x3C3C;
    let (m, queda, s) = chao(4, semente);
    assert_eq!(queda, 1);
    // Sorteios refeitos à mão: x 23..127 por fora, y 0..63 por dentro, S(200) e, se 0, S(2).
    let mut r = GeradorS::new(semente);
    let mut fiapos = vec![None; 128 * 64];
    for x in 23..128 {
        for y in 0..64 {
            if r.rand1(200) == 0 {
                fiapos[x * 64 + y] = Some(0x3E + r.rand1(2) as u8);
            }
        }
    }
    assert_eq!(s, r, "mesma quantidade de sorteios");
    let tomada = |x: i32, y: i32| (2..=14).contains(&x) && (25..=33).contains(&y);
    for x in 0..128i32 {
        for y in 0..64i32 {
            let v = em(&m, x, y);
            if tomada(x, y) {
                continue;
            }
            let esperado = match x {
                0..=20 => 0x64,
                21 => 0x7A,
                22 => 0x7B,
                _ => fiapos[(x * 64 + y) as usize].unwrap_or(0x03),
            };
            assert_eq!(v, esperado, "({x}, {y})");
        }
    }
    // _MakeOutletH(2, 0x19): moldura 1 em x 2..14, y 25..33, centro (8, 29).
    assert_eq!(em(&m, 2, 25), 0x53);
    assert_eq!(em(&m, 8, 29), 0x65);
    assert_eq!(em(&m, 4, 27), 0x6B);
    assert_eq!(em(&m, 5, 28), 0x76);
}

#[test]
fn carpete_direito_coluna_106_fica_0x64() {
    let (m, queda, _) = chao(20, 0x0101);
    assert_eq!(queda, 3);
    for y in 0..64 {
        assert_eq!(em(&m, 104, y), 0x7B);
        assert_eq!(em(&m, 105, y), 0x7B);
        assert_eq!(em(&m, 106, y), 0x64, "0x7C sobrescrito (desmontagem 1020:3639–367b)");
    }
    assert_eq!(em(&m, 119, 39), 0x65, "_MakeOutletH(0x71, 0x23)");
}

#[test]
fn patches_lisos_e_pernas_de_movel() {
    let (m, queda, _) = chao(9, 1);
    assert!(m.iter().all(|&v| v == 3) && queda == 2);
    let (m, queda, _) = chao(36, 1);
    assert!(m.iter().all(|&v| v == 0) && queda == 2);
    // Patch 12: _FillMapLegs(0x2D, 0x5F, 8, 0x2E, 2): cantos 4 × 4 = 0xC0; dentro, 2 ou fiapo 0x38..0x3C.
    let (m, _, _) = chao(12, 0x4444);
    for (x, y) in [(45, 8), (48, 11), (95, 8), (92, 11), (45, 46), (48, 43), (95, 46), (92, 43)] {
        assert_eq!(em(&m, x, y), 0xC0, "({x}, {y})");
    }
    let dentro: Vec<u8> = (50..90).flat_map(|x| (15..40).map(move |y| (x, y))).map(|(x, y)| em(&m, x, y)).collect();
    assert!(dentro.iter().all(|&v| v == 2 || (0x38..=0x3C).contains(&v)));
    assert!(dentro.iter().any(|&v| v != 2), "algum pé/fiapo em 1/20 das células");
    // Patch 6: moeda em (32, 20) e clipe (zeros transparentes) em (38, 55) sobre o 2.
    let (m, _, _) = chao(6, 9);
    assert_eq!(em(&m, 32, 20), 0x28);
    assert_eq!(em(&m, 34, 22), 0x30);
    assert_eq!(em(&m, 38, 55), 2, "canto do clipe é transparente");
    assert_eq!(em(&m, 39, 55), 0x31);
}

// --- comparação com o original (bancada QEMU, 2026-10-17) -------------------------------------

/// Estados gravados pela bancada logo depois de `_XferPatch` para um patch da casa (Quick Game com
/// `_CurGameType` = 2 e uma colônia preta escrita em `_YMapPopB`; ver geracao-mundo.md §5.3). Com a
/// semente `_YMapSeeds` do próprio estado, pretas = `_YMapPopB[n]` e vermelhas = `_YMapPopR[n]`, o
/// port deve dar `_MapA`, `_MapB`, `_MapR` e `_ExitMapB/R` idênticos. Precisa do banco SHARED (seno
/// de `_AddFood`): sem ele, avisa e passa sem verificar.
#[test]
fn casa_igual_ao_original_depois_de_xfer_patch() {
    use engine::formats::save::{JogoSalvo, LayoutSave};
    use engine::recursos::Recursos;
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
        return;
    }
    let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    let t = TabelasGeracao::carregar(&r).unwrap();
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    for (arquivo, px, py) in [("CASA_0_0", 0, 0), ("CASA_0_4", 0, 4), ("CASA_0_12", 0, 12), ("CASA_1_4", 1, 4)] {
        let j = JogoSalvo::abrir(&layout, &raiz().join(format!("tests/referencias/saves/{arquivo}.ANT"))).unwrap();
        let bloco = |s: &str| j.bloco(s).unwrap().to_vec();
        assert_eq!(j.u16("TERRAINset").unwrap(), 1, "{arquivo}");
        let n = (px * 16 + py) as usize;
        let k = (py * 12 + px) as usize;
        let sementes = bloco("YMapSeeds");
        let semente = u16::from_le_bytes([sementes[2 * k], sementes[2 * k + 1]]);
        let (pretas, vermelhas) = (bloco("YMapPopB")[n] as i32, bloco("YMapPopR")[n] as i32);
        let mut m = Mundo::vazio(TipoJogo::Completo);
        m.ninho_iniciado = true; // _MeNestStarted = 1 no Quick Game de onde a bancada parte
        let mut s = GeradorS::new(0);
        m.gerar_patch(&mut s, &t, semente, pretas, vermelhas, px, py).unwrap();
        for (nome, port) in [
            ("MapA", &m.mapa_a),
            ("MapB", &m.mapa_b),
            ("MapR", &m.mapa_r),
            ("ExitMapB", &m.saida_b),
            ("ExitMapR", &m.saida_r),
        ] {
            let orig = bloco(nome);
            let dif = orig.iter().zip(port.iter()).filter(|(a, b)| a != b).count();
            assert_eq!(dif, 0, "{arquivo}: {nome} com {dif} células diferentes");
        }
    }
}
