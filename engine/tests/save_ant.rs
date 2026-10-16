//! Testes do carregador de `.ANT` com `tests/referencias/saves/QUICK2.ANT` (Quick Game salvo pelo original).
//! Especificação: docs/spec/formatos/save-ant.md.
//! O teste contra o executável usa SIMANT_DIR (ou ../SimAnt-T/MAXIS/ANTWIN) e é ignorado sem ele.

use std::path::PathBuf;

use engine::formats::save::{Formiga, JogoSalvo, LayoutSave, Lista};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn layout() -> LayoutSave {
    LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap()
}

fn quick2() -> JogoSalvo {
    JogoSalvo::abrir(&layout(), &raiz().join("tests/referencias/saves/QUICK2.ANT")).unwrap()
}

#[test]
fn layout_tem_307_blocos_e_48386_bytes() {
    let l = layout();
    assert_eq!(l.blocos.len(), 307);
    assert_eq!(l.total_bytes, 48386);
    assert_eq!(l.blocos[0].simbolo, "MapA");
    assert_eq!(l.blocos[306].simbolo, "ShowPercentMode");
}

#[test]
fn tamanho_errado_e_recusado() {
    let mut d = std::fs::read(raiz().join("tests/referencias/saves/QUICK2.ANT")).unwrap();
    d.pop();
    assert!(JogoSalvo::ler(&layout(), d).is_err());
}

#[test]
fn round_trip_byte_a_byte() {
    let original = std::fs::read(raiz().join("tests/referencias/saves/QUICK2.ANT")).unwrap();
    let mut s = quick2();
    assert_eq!(s.bytes(), &original[..]);

    // Alterar e desfazer pela API tem que voltar ao arquivo original.
    let t = s.i32("GameTime").unwrap();
    s.definir_i32("GameTime", t + 1).unwrap();
    assert_ne!(s.bytes(), &original[..]);
    s.definir_i32("GameTime", t).unwrap();
    let h = s.i16("HealthB").unwrap();
    s.definir_i16("HealthB", h).unwrap();
    assert_eq!(s.bytes(), &original[..]);

    let tmp = std::env::temp_dir().join(format!("simant_round_trip_{}.ANT", std::process::id()));
    s.gravar(&tmp).unwrap();
    let relido = std::fs::read(&tmp).unwrap();
    std::fs::remove_file(&tmp).ok();
    assert_eq!(relido, original);
}

#[test]
fn blocos_por_nome_seguem_o_layout() {
    let s = quick2();
    assert_eq!(s.simbolos().count(), 307);
    for (nome, n) in [("MapA", 8192), ("MapB", 4096), ("MapR", 4096), ("AlistX", 1000), ("BlistT", 500), ("PherMapA", 2048)] {
        assert_eq!(s.bloco(nome).unwrap().len(), n, "{nome}");
    }
    assert!(s.bloco("NaoExiste").is_none());
    assert!(s.i16("MapA").is_err(), "escalar de 2 bytes não pode ler bloco de 8192");
}

#[test]
fn escalares_do_quick_game() {
    // Valores lidos do QUICK2.ANT (Quick Game recém-iniciado, ver docs/spec/formatos/save-ant.md).
    let s = quick2();
    assert_eq!(s.i16("CurGameType").unwrap(), 1);
    assert_eq!(s.u32("GameTime").unwrap(), 78);
    assert_eq!(s.par_i16("CurYardPnt").unwrap(), (11, 8));
    assert_eq!(s.i16("WaterTable").unwrap(), 64);
    assert_eq!(s.vetor_i16("CastePopB").unwrap().len(), 6);
}

#[test]
fn mapas_dos_ninhos_ficam_na_faixa_observada() {
    // Ninho seco (WaterTable = 64): nenhum valor inundado (>= 0x4e, ver _DoWater na spec).
    let s = quick2();
    for m in [s.mapa_b().unwrap(), s.mapa_r().unwrap()] {
        assert_eq!((m.largura, m.altura), (64, 64));
        assert!(m.dados.iter().all(|&v| v <= 0x2e), "valor de ninho fora da faixa");
    }
    let a = s.mapa_a().unwrap();
    assert_eq!((a.largura, a.altura), (128, 64));
    assert!(a.em(128, 0).is_none() && a.em(0, 64).is_none());
}

/// Valor mais comum de um mapa (a terra/grama de fundo).
fn moda(dados: &[u8]) -> u8 {
    let mut c = [0usize; 256];
    for &v in dados {
        c[v as usize] += 1;
    }
    (0..=255u8).max_by_key(|&v| c[v as usize]).unwrap()
}

#[test]
fn formigas_dos_ninhos_estao_em_tuneis_coluna_major() {
    // Confirma a indexação x * 64 + y: cada formiga do ninho está numa célula escavada
    // (diferente da terra maciça, o valor mais comum). Com y * 64 + x isso falha.
    let s = quick2();
    for lista in [Lista::B, Lista::R] {
        let m = s.mapa_da_lista(lista).unwrap();
        let terra = moda(m.dados);
        let fs = s.formigas(lista).unwrap();
        assert!(!fs.is_empty());
        for f in fs {
            let v = m.em(f.x as usize, f.y as usize).unwrap();
            assert_ne!(v, terra, "formiga {f:?} da lista {lista:?} dentro da terra");
        }
    }
}

#[test]
fn listas_usam_listindex_e_posicoes_validas() {
    let s = quick2();
    for (lista, cap, usadas, w, h) in [(Lista::A, 1000, 2, 128, 64), (Lista::B, 500, 5, 64, 64), (Lista::R, 500, 6, 64, 64)] {
        assert_eq!(s.capacidade(lista).unwrap(), cap);
        assert_eq!(s.usadas(lista).unwrap(), usadas);
        let fs = s.formigas(lista).unwrap();
        assert_eq!(fs.len(), usadas, "sem buracos (tipo 0) dentro de 0..ListIndex");
        for f in &fs {
            assert!((f.x as usize) < w && (f.y as usize) < h, "{f:?}");
        }
        // Observado no QUICK2: depois de ListIndex tudo é zero.
        for i in usadas..cap {
            assert_eq!(s.formiga(lista, i).unwrap().tipo, 0);
        }
    }
}

/// Recontagem por casta como em `_CountAnts` (1020:04de): classe = tipo >> 3 (sem o bit 7),
/// agrupada em [cria, 1+2+3+5, 6+7+9, 4, 8, 12].
fn castas(fs: &[Formiga], vermelha: bool, extra: Option<u8>) -> [i16; 6] {
    let mut por_classe = [0i16; 16];
    for f in fs.iter().filter(|f| f.vermelha() == vermelha) {
        por_classe[f.classe() as usize] += 1;
    }
    if let Some(k) = extra {
        por_classe[k as usize] += 1;
    }
    let c = por_classe;
    [c[0], c[1] + c[2] + c[3] + c[5], c[6] + c[7] + c[9], c[4], c[8], c[12]]
}

#[test]
fn contadores_de_populacao_batem_com_as_listas() {
    let s = quick2();
    let todas: Vec<Formiga> = Lista::TODAS.iter().flat_map(|&l| s.formigas(l).unwrap()).collect();
    // Formiga amarela (MeMode = 0): entra na contagem com a classe de MeType. No QUICK2 ela é preta.
    assert_eq!(s.i16("MeMode").unwrap(), 0);
    let me = Some(((s.i16("MeType").unwrap() as u8) >> 3) & 0x0f);

    for (vermelha, casta, total, extra) in [(false, "CastePopB", "BpopT", me), (true, "CastePopR", "RpopT", None)] {
        let gravado = s.vetor_i16(casta).unwrap();
        let recontado = castas(&todas, vermelha, extra);
        // Índices 1..5 (adultos) batem; o 0 (crias) pode estar defasado, pois _CountAnts só
        // roda a cada 128 ciclos e há ovos postos depois da última contagem.
        assert_eq!(&gravado[1..], &recontado[1..], "{casta}");
        assert!(recontado[0] >= gravado[0], "{casta}[0]");
        assert_eq!(s.i16(total).unwrap(), recontado[1..].iter().sum::<i16>(), "{total}");
    }
}

#[test]
fn populacao_do_patch_atual_no_mapa_do_quintal() {
    // YMapPopB/R: 12 × 16 patches, índice x * 16 + y (fato: _SimColonies 1030:35f8, ver spec).
    let s = quick2();
    let (x, y) = s.par_i16("CurYardPnt").unwrap();
    let i = x as usize * 16 + y as usize;
    assert_eq!(s.bloco("YMapPopB").unwrap()[i] as i16, s.i16("BpopT").unwrap());
    assert_eq!(s.bloco("YMapPopR").unwrap()[i] as i16, s.i16("RpopT").unwrap());
}

#[test]
fn feromonio_tem_64_por_32() {
    let s = quick2();
    for nome in ["PherMapA", "PherMapBN", "PherMapBT", "PherMapRN", "PherMapRT"] {
        let m = s.feromonio(nome).unwrap();
        assert_eq!((m.largura, m.altura), (64, 32));
    }
}

/// Lê a tabela de blocos direto do SIMANTW.EXE (segmento 8, offset 0x9570): só count e tamanho,
/// que não dependem de relocação. Formato NE: cabeçalho em [0x3c], tabela de segmentos em +0x22,
/// deslocamento de alinhamento em +0x32, 8 bytes por segmento (setor, tamanho, flags, alocação).
fn tabela_do_exe(exe: &[u8]) -> Vec<(usize, usize)> {
    let u16_em = |o: usize| u16::from_le_bytes([exe[o], exe[o + 1]]) as usize;
    let ne = u16_em(0x3c);
    assert_eq!(&exe[ne..ne + 2], b"NE");
    let tab_seg = ne + u16_em(ne + 0x22);
    let shift = u16_em(ne + 0x32);
    let seg8 = tab_seg + (8 - 1) * 8;
    let base = u16_em(seg8) << shift;
    let mut v = Vec::new();
    let mut o = base + 0x9570;
    while u16_em(o) != 0 {
        v.push((u16_em(o), u16_em(o + 2)));
        o += 8;
    }
    v
}

#[test]
fn layout_bate_com_a_tabela_do_executavel() {
    let d = std::env::var_os("SIMANT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| raiz().join("../SimAnt-T/MAXIS/ANTWIN"));
    let caminho = d.join("SIMANTW.EXE");
    let Ok(exe) = std::fs::read(&caminho) else {
        eprintln!("AVISO: {} não encontrado; teste ignorado", caminho.display());
        return;
    };
    let tabela = tabela_do_exe(&exe);
    let l = layout();
    assert_eq!(tabela.len(), l.blocos.len());
    for (b, &(count, tamanho)) in l.blocos.iter().zip(&tabela) {
        assert_eq!((b.count, b.tamanho), (count, tamanho), "{}", b.simbolo);
    }
}
