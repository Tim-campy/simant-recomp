//! Regras do ninho e do ciclo de vida (engine/src/rules/ninho.rs).
//! Especificação: docs/spec/regras/ninho.md e ciclo-de-vida.md.
//!
//! Três grupos de testes:
//! 1. casos montados à mão a partir da spec (estado pequeno, sementes conhecidas);
//! 2. vetores de tick do original (tests/referencias/ticks, docs/spec/regras/testes-tick.md):
//!    o ninho do port reproduz o `estado_depois` e o gerador S termina no mesmo valor;
//! 3. casos da bancada (tests/referencias/casos/N-*.json, C-1*.json): o port parte do estado que o
//!    original tinha logo depois do load (`*_carregado.ANT`), roda até o mesmo `GameTime` e as mesmas
//!    conferências do caso são avaliadas no estado do port.

use std::path::{Path, PathBuf};

use engine::formats::save::{Formiga, JogoSalvo, LayoutSave, Lista};
use engine::rules::estado::Estado;
use engine::rules::geracao::Cor;
use engine::rules::ninho::{simular_ninho, vencedor_da_luta, TabelasNinho};
use engine::rules::rng::GeradorS;
use engine::rules::tick::{tick, RegrasOriginais};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn layout() -> LayoutSave {
    LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap()
}

fn abrir(caminho: &Path) -> JogoSalvo {
    JogoSalvo::abrir(&layout(), caminho).unwrap()
}

fn tab() -> &'static TabelasNinho {
    TabelasNinho::padrao()
}

// ---------------------------------------------------------------------------------------------
// 1. Casos montados à mão
// ---------------------------------------------------------------------------------------------

/// Ninho preto todo de terra maciça, com um túnel vertical na coluna 10 (y = 1..=20), linha 0
/// aberta em (10, 0) e mapa de saída decrescente para baixo (0xFF na linha 0).
fn ninho_de_teste() -> Estado {
    let mut e = Estado::vazio();
    e.preta.saude = 100;
    e.vermelha.saude = 100;
    e.partida.ciclo = 1;
    for n in [&mut e.ninho_b, &mut e.ninho_r] {
        n.mapa.dados.fill(0x2E);
        for x in 0..64 {
            n.mapa.definir(x, 0, 0x1C);
            n.saida.definir(x, 0, 0xFF);
        }
        n.mapa.definir(10, 0, 0x18);
        for y in 1..=20 {
            n.mapa.definir(10, y, 0x03);
            n.saida.definir(10, y, 0xFF - y as u8);
        }
    }
    e
}

fn formiga(x: u8, y: u8, modo: u8, tipo: u8, s: u8) -> Formiga {
    Formiga { x, y, modo, tipo, s }
}

#[test]
fn vaga_vazia_nao_sorteia() {
    let mut e = ninho_de_teste();
    e.ninho_b.lista.adicionar(formiga(10, 5, 2, 0, 0));
    e.s.definir_semente(0x1234);
    simular_ninho(&mut e, Cor::Preta, tab());
    assert_eq!(e.s.semente(), 0x1234);
}

#[test]
fn operaria_sobe_pelo_mapa_de_saida_e_sai_pelo_buraco() {
    // ninho.md §4.5, §4.6, §7.2 (caso N-03 em miniatura).
    let mut e = ninho_de_teste();
    e.ambiente.terreno = 0;
    e.preta.buracos[10] = 20; // buraco na superfície em (20, 10)
    e.adicionar_formiga(Lista::B, formiga(10, 3, 2, 0x10, 0));
    e.s.definir_semente(0x1234);
    for esperado in [(10, 2), (10, 1)] {
        simular_ninho(&mut e, Cor::Preta, tab());
        let f = e.ninho_b.lista.vagas[0];
        assert_eq!((f.x, f.y, f.tipo & 0xF8), (esperado.0, esperado.1, 0x10));
        assert_eq!(e.ninho_b.vida.em(esperado.0 as i32, esperado.1 as i32), Some(f.tipo));
    }
    simular_ninho(&mut e, Cor::Preta, tab());
    assert_eq!(e.ninho_b.lista.vagas[0].tipo, 0, "saiu da lista B");
    assert_eq!(e.ninho_b.vida.em(10, 1), Some(0));
    let a = e.superficie.lista.vagas[0];
    assert_eq!(e.superficie.lista.indice, 1);
    // Primeira vizinha (direção 0) do buraco (20, 10): (20, 9); M mantido, S = 120 (lado preto).
    assert_eq!((a.x, a.y, a.modo, a.s, a.tipo & 0xF8), (20, 9, 2, 120, 0x10));
}

#[test]
fn morte_natural_respeita_a_saude() {
    // §3.1: com Sk(256) = 0, morre se Sk(32) > saúde. Procura uma semente cujo primeiro sorteio dá 0.
    let semente = (1..=u16::MAX).find(|&s| GeradorS::new(s).mascara(256) == 0).unwrap();
    let mut seg = GeradorS::new(semente);
    seg.mascara(256);
    let b = seg.mascara(32);
    for (saude, morre) in [(b as i16 - 1, true), (b as i16, false)] {
        let mut e = ninho_de_teste();
        e.preta.saude = saude;
        e.adicionar_formiga(Lista::B, formiga(10, 5, 13, 0x12, 0));
        e.s.definir_semente(semente);
        simular_ninho(&mut e, Cor::Preta, tab());
        assert_eq!(e.ninho_b.lista.vagas[0].tipo == 0, morre, "saúde {saude}, Sk(32) = {b}");
        assert_eq!(e.preta.mortes_naturais, morre as i32);
    }
    // A rainha (M = 9) não morre assim.
    let mut e = ninho_de_teste();
    e.preta.saude = 0;
    e.adicionar_formiga(Lista::B, formiga(10, 5, 9, 0x6C, 0));
    e.s.definir_semente(semente);
    simular_ninho(&mut e, Cor::Preta, tab());
    assert_eq!(e.preta.mortes_naturais, 0);
}

#[test]
fn ama_preta_pega_ovo_guarda_estagio_e_larga_no_fundo() {
    // §5.1, §5.2 e ciclo-de-vida.md §4 (caso C-11 em miniatura).
    let mut e = ninho_de_teste();
    e.adicionar_formiga(Lista::B, formiga(10, 18, 8, 5, 0)); // ovo, índice menor: fica em Vida
    e.ninho_b.lista.adicionar(formiga(10, 18, 1, 0x12, 0));
    e.s.definir_semente(0x4321);
    simular_ninho(&mut e, Cor::Preta, tab());
    let ama = e.ninho_b.lista.vagas[1];
    assert_eq!((ama.tipo, ama.s, ama.x, ama.y), (0x0A, 5, 10, 18), "pega o ovo e não anda");
    assert_eq!(e.ninho_b.lista.vagas[0].tipo, 0);
    // Desce até o fim do túnel (10, 20), marca o bit 3 e, no tick seguinte, larga o ovo com o mesmo
    // estágio. Virada para cima (T = 0x08) ela não consegue largar (defeito de §5.1): algumas sementes
    // caem nisso, por isso procura uma semente em que a ama larga em até 25 ticks.
    let depois_de_pegar = e.clone();
    for k in 0..100u16 {
        // Sementes espalhadas (as pequenas, potências de 2, dão Sk(8) = 0 várias vezes seguidas).
        let semente = 0x1234u16.wrapping_add(k.wrapping_mul(0x9E37));
        let mut e = depois_de_pegar.clone();
        e.s.definir_semente(semente);
        for _ in 0..25 {
            simular_ninho(&mut e, Cor::Preta, tab());
            if e.ninho_b.lista.vagas[1].tipo & 0xF8 == 0x10 {
                break;
            }
        }
        let ama = e.ninho_b.lista.vagas[1];
        if ama.tipo & 0xF8 != 0x10 {
            continue;
        }
        assert!(ama.s == 8 || ama.s == 0, "largou: grupo 2 e S = 8 (ou 0 depois do Sk(8))");
        let ovos: Vec<Formiga> = e.ninho_b.lista.ativas().map(|(_, f)| *f).filter(|f| f.modo == 8).collect();
        assert_eq!(ovos.len(), 1);
        assert_eq!(ovos[0].tipo, 5, "o ovo largado tem o estágio guardado");
        return;
    }
    panic!("nenhuma semente fez a ama largar o ovo");
}

#[test]
fn comida_dobrada() {
    // §6.2: cada carga trazida rende duas unidades (caso N-06 em miniatura).
    let mut e = ninho_de_teste();
    e.adicionar_formiga(Lista::B, formiga(10, 3, 3, 0x1A, 0));
    e.s.definir_semente(0x0BAD);
    for _ in 0..60 {
        simular_ninho(&mut e, Cor::Preta, tab());
    }
    let unidades: i32 = e.ninho_b.mapa.dados.iter().filter(|v| (0x10..=0x13).contains(*v)).map(|&v| v as i32 - 0x0F).sum();
    assert_eq!(e.preta.comida, 2);
    assert_eq!(unidades, 2);
}

#[test]
fn saida_decrementada_sem_teste_no_preto_e_com_teste_no_vermelho() {
    // §7.2 (caso N-09 em miniatura): célula de túnel cercada de terra inundada, saída 0.
    for cor in [Cor::Preta, Cor::Vermelha] {
        let mut e = ninho_de_teste();
        let n = e.ninho_mut(cor);
        for x in 29..=31 {
            for y in 39..=41 {
                n.mapa.definir(x, y, 0x5D);
            }
        }
        n.mapa.definir(30, 40, 0x02);
        let (lista, t) = if cor == Cor::Preta { (Lista::B, 0x12) } else { (Lista::R, 0x92) };
        e.adicionar_formiga(lista, formiga(30, 40, 2, t, 0));
        e.s.definir_semente(0x7777);
        for _ in 0..3 {
            simular_ninho(&mut e, cor, tab());
        }
        let esperado = if cor == Cor::Preta { 0xFF } else { 0 };
        assert_eq!(e.ninho(cor).saida.em(30, 40), Some(esperado), "{cor:?}");
        assert_eq!(e.ninho(cor).lista.vagas[0].modo, 4, "passa a cavar");
    }
}

#[test]
fn luta_nao_remove_a_checadora() {
    // §4.3 (caso N-12 em miniatura): a vermelha (índice menor, em Vida) vira nuvem; a preta fica.
    let mut e = ninho_de_teste();
    e.adicionar_formiga(Lista::B, formiga(10, 8, 0, 0x92, 0));
    e.ninho_b.lista.adicionar(formiga(10, 8, 13, 0x12, 0));
    e.s.definir_semente(0x2468);
    simular_ninho(&mut e, Cor::Preta, tab());
    let (v, p) = (e.ninho_b.lista.vagas[0], e.ninho_b.lista.vagas[1]);
    assert_eq!((p.tipo, p.modo), (0x12, 13));
    assert_eq!(v.modo, 10);
    assert_eq!(v.tipo & 0x78, 0x70);
    assert!(v.s == 0x92 || v.s == 0x12);
    assert_eq!(e.preta.mortas_em_luta + e.vermelha.mortas_em_luta, 1);
}

#[test]
fn vencedor_usa_o_gerador_r_e_a_tabela() {
    let mut e = Estado::vazio();
    let mut r = e.r;
    let w = vencedor_da_luta(&mut e, tab(), 0x32, 0x92); // soldado preto contra operária vermelha
    let esperado = if r.rrand(10) < 8 { 0x32 } else { 0x92 };
    assert_eq!(w, esperado);
    e.partida.invencivel = 1;
    let r0 = e.r;
    assert_eq!(vencedor_da_luta(&mut e, tab(), 0x92, 0x12), 0x12);
    assert_eq!(e.r, r0, "invencível não sorteia");
}

#[test]
fn rainha_atacada_some_e_a_cauda_vai_junto() {
    // §3.2 e ciclo-de-vida.md §5.4 (caso N-13 em miniatura).
    let mut e = ninho_de_teste();
    e.preta.rainhas = 1;
    e.adicionar_formiga(Lista::B, formiga(10, 10, 9, 0x64, 0)); // cabeça virada para baixo
    e.adicionar_formiga(Lista::B, formiga(10, 9, 9, 0x6C, 0)); // cauda acima
    e.ninho_b.lista.adicionar(formiga(10, 10, 7, 0x92, 0)); // invasora na célula da cabeça
    e.s.definir_semente(0x1357);
    simular_ninho(&mut e, Cor::Preta, tab());
    let l = &e.ninho_b.lista.vagas;
    assert_eq!(l[2].tipo, 0, "a invasora some");
    assert_eq!(l[0].modo, 10, "a cabeça vira nuvem");
    assert!(l[0].s == 0 || l[0].s == 0x92);
    assert_eq!(l[1].tipo, 0, "a cauda some no mesmo tick");
    assert_eq!(e.preta.rainhas, 0);
}

#[test]
fn ovo_avanca_so_no_ciclo_do_periodo_e_eclode() {
    // ciclo-de-vida.md §3.1: P = 32 com _BpopT < 3; eclode como _MakeMe × 8 + 2.
    let mut e = ninho_de_teste();
    e.preta.populacao = 2;
    e.controles.modo_automatico = 1;
    e.controles.proxima_casta = 2;
    e.adicionar_formiga(Lista::B, formiga(10, 5, 8, 6, 0));
    e.partida.ciclo = 31;
    simular_ninho(&mut e, Cor::Preta, tab());
    assert_eq!(e.ninho_b.lista.vagas[0].tipo, 6);
    e.partida.ciclo = 32;
    simular_ninho(&mut e, Cor::Preta, tab());
    assert_eq!(e.ninho_b.lista.vagas[0].tipo, 7);
    e.partida.ciclo = 64;
    simular_ninho(&mut e, Cor::Preta, tab());
    let f = e.ninho_b.lista.vagas[0];
    assert_eq!((f.tipo, f.modo), (0x12, 1), "operária virada para a direita, M = 1");
}

#[test]
fn cauda_poe_ovo_atras_e_cobra_a_saude() {
    // ciclo-de-vida.md §2.
    let mut e = ninho_de_teste();
    e.preta.rainhas = 1;
    e.preta.reserva_comer = 0;
    e.preta.populacao = 3;
    e.adicionar_formiga(Lista::B, formiga(10, 10, 9, 0x64, 0));
    e.adicionar_formiga(Lista::B, formiga(10, 9, 9, 0x6C, 0));
    e.partida.ciclo = 16;
    // Semente que dá Sk(64) ≠ 0 na cabeça (não anda) e Sk(128) ≤ 100 na cauda. A cauda é processada
    // primeiro (índice maior): Sk(256) dela, Sk(128) da postura, cavar (sorteios do _DigTileB), e a cabeça.
    let semente = (1..=u16::MAX)
        .find(|&s| {
            let mut g = GeradorS::new(s);
            g.mascara(256);
            g.mascara(128) <= 100
        })
        .unwrap();
    e.s.definir_semente(semente);
    simular_ninho(&mut e, Cor::Preta, tab());
    let ovo = e.ninho_b.lista.vagas[2];
    assert_eq!((ovo.x, ovo.y, ovo.tipo, ovo.modo), (10, 8, 1, 8));
    assert_eq!(e.preta.ultimo_ovo, (10, 8));
    assert_eq!(e.partida.ovos_postos_b, 1);
    assert_eq!((e.preta.reserva_comer, e.preta.saude), (0, 99), "reserva negativa: volta a 3 >> 5 = 0 e saúde −1");
}

#[test]
fn passagem_leva_a_vermelha_para_o_ninho_preto() {
    // §7.7 (caso N-14 em miniatura).
    let mut e = ninho_de_teste();
    e.ninho_b.mapa.definir(10, 21, 0x14);
    e.ninho_r.mapa.definir(10, 21, 0x14);
    e.ninho_r.saida.definir(10, 21, 0x01);
    e.adicionar_formiga(Lista::R, formiga(10, 20, 4, 0x94, 0));
    for semente in 1..200u16 {
        let mut e2 = e.clone();
        e2.s.definir_semente(semente);
        simular_ninho(&mut e2, Cor::Vermelha, tab());
        if e2.ninho_r.lista.vagas[0].tipo == 0 {
            let b = e2.ninho_b.lista.vagas[0];
            assert_eq!(e2.ninho_b.lista.indice, 1);
            assert_eq!((b.x, b.y, b.modo, b.s, b.tipo & 0xF8), (10, 21, 3, 0, 0x90));
            assert_eq!(e2.ninho_b.vida.em(10, 21), Some(b.tipo));
            return;
        }
    }
    panic!("nenhuma semente levou a vermelha para a passagem");
}

// ---------------------------------------------------------------------------------------------
// 2. Vetores de tick do original
// ---------------------------------------------------------------------------------------------

/// Passo inverso do LFSR do gerador S (o bit 0 do novo estado é o bit que saiu).
fn passo_inverso(s: u16) -> u16 {
    if s & 1 != 0 {
        ((s ^ 0x1BF5) >> 1) | 0x8000
    } else {
        s >> 1
    }
}

fn extras(dir: &Path, quando: &str, campo: &str) -> u64 {
    let texto = std::fs::read_to_string(dir.join("extras.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&texto).unwrap();
    v[quando][campo].as_u64().unwrap()
}

/// O que o ninho mexe e o vetor guarda: mapas, listas, vida e os contadores das colônias.
fn resumo_do_ninho(e: &Estado) -> Vec<String> {
    let mut v = Vec::new();
    for (nome, n) in [("B", &e.ninho_b), ("R", &e.ninho_r)] {
        v.push(format!("{nome} mapa {:?}", n.mapa.dados));
        v.push(format!("{nome} saida {:?}", n.saida.dados));
        v.push(format!("{nome} vida {:?}", n.vida.dados));
        v.push(format!("{nome} lista {} {:?}", n.lista.indice, &n.lista.vagas[..n.lista.usadas()]));
    }
    for (nome, c) in [("preta", &e.preta), ("vermelha", &e.vermelha)] {
        v.push(format!(
            "{nome} saude {} comida {} reserva {} rainhas {} escav {} {} {} {} {} ultimo_ovo {:?} buracos {:?} mortes {} {} {}",
            c.saude,
            c.comida,
            c.reserva_comer,
            c.rainhas,
            c.tiles_escavados,
            c.soma_x_escavada,
            c.soma_y_escavada,
            c.centro_x,
            c.centro_y,
            c.ultimo_ovo,
            c.buracos,
            c.mortes_naturais,
            c.perdas,
            c.mortas_em_luta
        ));
    }
    v.push(format!("ovos {} {}", e.partida.ovos_postos_b, e.partida.ovos_perdidos_b));
    v
}

#[test]
fn vetores_de_tick_do_original_no_ninho() {
    // O tick inteiro consome S antes do ninho (quintal, formigas-leão, aranha, estratégia...), o que o
    // port ainda não tem. Mas depois de `_DoAntSimR` nada mais sorteia S nestes ticks [verificado
    // abaixo: o S final bate], então a semente na entrada do ninho é o S final do vetor andado k
    // passos para trás, para algum k. Para cada k, roda o ninho e exige o estado depois e S final.
    let dir = raiz().join("tests/referencias/ticks");
    if !dir.join("indice.json").exists() {
        eprintln!("sem vetores de tick em {}; teste não verificado", dir.display());
        return;
    }
    for t in 79..=83 {
        let d = dir.join(format!("tick_{t:06}"));
        let antes = abrir(&d.join("estado_antes.bin"));
        let depois = Estado::de_save(&abrir(&d.join("estado_depois.bin"))).unwrap();
        let vida_antes = std::fs::read(d.join("vida_antes.bin")).unwrap();
        let vida_depois = std::fs::read(d.join("vida_depois.bin")).unwrap();
        let mut base = Estado::de_save(&antes).unwrap();
        // A vida reconstruída das listas é a do original (hipótese de testes-tick.md, conferida aqui).
        assert_eq!(base.ninho_b.vida.dados, vida_antes[8192..12288], "tick {t}: _LifeB reconstruído");
        assert_eq!(base.ninho_r.vida.dados, vida_antes[12288..], "tick {t}: _LifeR reconstruído");
        // O que o tick faz antes do ninho: _Cycle/_GameTime e _GetStrategy.
        base.partida.ciclo = depois.partida.ciclo;
        base.partida.tempo = depois.partida.tempo;
        base.preta.modo_estrategico = depois.preta.modo_estrategico;
        base.vermelha.modo_estrategico = depois.vermelha.modo_estrategico;
        base.controles.proxima_casta = depois.controles.proxima_casta;
        base.controles.proximo_modo = depois.controles.proximo_modo;
        base.r = engine::rules::rng::GeradorR::default();
        let s_final = extras(&d, "depois", "SementeS") as u16;
        let mut esperado = depois.clone();
        esperado.ninho_b.vida.dados = vida_depois[8192..12288].to_vec();
        esperado.ninho_r.vida.dados = vida_depois[12288..].to_vec();
        let alvo = resumo_do_ninho(&esperado);

        let mut s0 = s_final;
        let mut achou = None;
        for k in 0..400 {
            let mut e = base.clone();
            e.s.definir_semente(s0);
            simular_ninho(&mut e, Cor::Preta, tab());
            simular_ninho(&mut e, Cor::Vermelha, tab());
            if e.s.semente() == s_final && resumo_do_ninho(&e) == alvo {
                achou = Some(k);
                break;
            }
            s0 = passo_inverso(s0);
        }
        let k = achou.unwrap_or_else(|| panic!("tick {t}: nenhuma semente reproduz o ninho do original"));
        eprintln!("tick {t}: ninho reproduzido com {k} sorteios de S");
    }
}

// ---------------------------------------------------------------------------------------------
// 3. Casos da bancada
// ---------------------------------------------------------------------------------------------

/// Avalia uma conferência de `tools/caso_ant.py` no estado do port (gravado como `.ANT`).
fn conferir(j: &JogoSalvo, c: &serde_json::Value) -> Result<(), String> {
    let lista = |l: &str| match l {
        "A" => Lista::A,
        "B" => Lista::B,
        _ => Lista::R,
    };
    let campo = |f: &Formiga, c: &str| -> i64 {
        (match c {
            "X" => f.x,
            "Y" => f.y,
            "M" => f.modo,
            "T" => f.tipo,
            _ => f.s,
        }) as i64
    };
    let bate = |f: &Formiga, onde: &serde_json::Value| {
        onde.as_object().is_none_or(|o| o.iter().all(|(k, v)| campo(f, k) == v.as_i64().unwrap()))
    };
    let formigas = |l: &str| j.formigas(lista(l)).unwrap();
    let mascara = |v: i64| c.get("mascara").and_then(|m| m.as_i64()).map_or(v, |m| v & m);
    let valor = &c["valor"];
    let op = c.get("op").and_then(|o| o.as_str()).unwrap_or("==");
    let comparar = |v: i64| -> bool {
        match op {
            "em" => valor.as_array().unwrap().iter().any(|x| x.as_i64() == Some(v)),
            _ => {
                let w = valor.as_i64().unwrap();
                match op {
                    "==" => v == w,
                    "!=" => v != w,
                    ">=" => v >= w,
                    "<=" => v <= w,
                    ">" => v > w,
                    "<" => v < w,
                    _ => panic!("op {op}"),
                }
            }
        }
    };
    let ok = if let Some(nome) = c.get("nome").and_then(|n| n.as_str()) {
        let b = j.bloco(nome).unwrap();
        let v = match c.get("indice").and_then(|i| i.as_u64()) {
            Some(i) => {
                let w = c.get("largura").and_then(|l| l.as_u64()).unwrap_or(1) as usize;
                let o = i as usize * w;
                b[o..o + w].iter().rev().fold(0i64, |a, &x| (a << 8) | x as i64)
            }
            None => b.iter().rev().fold(0i64, |a, &x| (a << 8) | x as i64),
        };
        comparar(mascara(v))
    } else if let Some(l) = c.get("formiga").and_then(|l| l.as_str()) {
        let i = c["indice"].as_u64().unwrap() as usize;
        let f = j.formiga(lista(l), i).unwrap();
        comparar(mascara(campo(&f, c["campo"].as_str().unwrap())))
    } else if let Some(l) = c.get("formigas").and_then(|l| l.as_str()) {
        let vs: Vec<i64> =
            formigas(l).iter().filter(|f| bate(f, &c["onde"])).map(|f| mascara(campo(f, c["campo"].as_str().unwrap()))).collect();
        match op {
            "todos==" => !vs.is_empty() && vs.iter().all(|&v| Some(v) == valor.as_i64()),
            "todos_em" => {
                !vs.is_empty() && vs.iter().all(|v| valor.as_array().unwrap().iter().any(|x| x.as_i64() == Some(*v)))
            }
            _ => panic!("op {op} para 'formigas'"),
        }
    } else if let Some(l) = c.get("contar").and_then(|l| l.as_str()) {
        comparar(formigas(l).iter().filter(|f| bate(f, &c["onde"])).count() as i64)
    } else {
        panic!("conferência sem alvo: {c}");
    };
    if ok {
        Ok(())
    } else {
        Err(format!("falhou: {c}"))
    }
}

/// Roda o port a partir do estado carregado pelo original (`saves/<ID>_carregado.ANT`) até o
/// `GameTime` alvo do caso, com várias sementes S, e confere o caso em cada execução. Só as regras
/// do ninho estão no port; as demais partes do tick ficam vazias.
fn rodar_caso(id: &str) {
    let caso_p = raiz().join(format!("tests/referencias/casos/{id}.json"));
    let caso: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&caso_p).unwrap()).unwrap();
    let nome = id.replace('-', "");
    let entrada = raiz().join(format!("tests/referencias/saves/{nome}_carregado.ANT"));
    let jogo = abrir(&entrada);
    let alvo = caso["rodar"]["ate_gametime"].as_u64().unwrap() as u32;
    let l = layout();
    for semente in [1u16, 0x1234, 0x8a5e, 0xBEEF, 0x0F0F, 0x7777] {
        let mut e = Estado::de_save(&jogo).unwrap();
        e.partida.pausado = 0;
        e.s.definir_semente(semente);
        let mut regras = RegrasOriginais::default();
        while e.partida.tempo < alvo {
            tick(&mut e, &mut regras);
        }
        let fim = e.para_save(&l).unwrap();
        for c in caso["conferir"].as_array().unwrap() {
            if c.get("so_original").is_some() {
                continue;
            }
            if let Err(m) = conferir(&fim, c) {
                panic!("{id}, semente {semente:#06x}: {m}
lista B: {:?}", fim.formigas(Lista::B).unwrap());
            }
        }
    }
}

macro_rules! caso_da_bancada {
    ($nome:ident, $id:literal) => {
        #[test]
        fn $nome() {
            rodar_caso($id);
        }
    };
}

caso_da_bancada!(caso_n03_sair_do_ninho, "N-03");
caso_da_bancada!(caso_n06_comida_dobrada, "N-06");
caso_da_bancada!(caso_n09_saida_sem_teste, "N-09");
caso_da_bancada!(caso_n13_rainha_atacada, "N-13");
caso_da_bancada!(caso_c11_ama_guarda_estagio, "C-11");
