//! Port × original, um tick por vez, com os vetores de `tests/referencias/ticks/`: os ticks 79–83 do
//! snapshot `quick` na raiz e os cenários das subpastas listadas em `"cenarios"` do `indice.json` da raiz
//! (cada uma com o seu `indice.json`). Protocolo e placar: docs/spec/regras/testes-tick.md.
//!
//! Desde 2026-10-17 (todas as partes do tick implementadas) o teste exige igualdade TOTAL: os 307
//! blocos do save, os extras (S, R, `_gameCycles`, `_Tindex`, `_GlobalKey`) e a vida. Exceções, no
//! `indice.json` de cada cenário, sempre com o motivo:
//! - `"pendentes"`: {tick: motivo} — vetores em que o port ainda diverge; só se exige que rodem e que a
//!   ida e volta do save funcione. Quem corrigir a regra tira o tick da lista.
//! - `"pela_interface"`: {tick: onde} — ticks em que o original passa pela interface DENTRO do tick
//!   (ex.: GAME OVER e "Select a Game"); o tick sozinho não chega ao estado de depois, e o vetor é
//!   conferido inteiro (307/307, S e R) pelo teste da interface indicado.
//! - `"vida_nao_reconstruivel"`: {tick: motivo} — a vida do original não é reconstruível das listas
//!   (duas ocupantes na mesma célula); não se confere a reconstrução, só a vida depois do tick.
//!   A chave `"*"` vale para todos os ticks do cenário.
//! - `"gamecycles_fora_do_tick"`: {tick: motivo} — o tick abre uma caixa que não para o temporizador
//!   (`_YellowDialog` da trofalaxia): enquanto ela está aberta, o laço do `MYTIMERFUNC` conta
//!   `_gameCycles` em tempo real; só esse extra deixa de ser exigido.
//!
//! Para ver o relatório de uma falha:
//! `cargo run -p engine --example comparar_tick -- tests/referencias/ticks[/CENÁRIO]`.

use std::path::{Path, PathBuf};

use engine::formats::save::LayoutSave;
use engine::rules::vetor_tick::{comparar_tick, regras_do_vetor, RelatorioTick, VetorTick};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn ler_indice(dir: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join("indice.json")).unwrap()).unwrap()
}

/// Um vetor comparado: (cenário/tick, relatório, motivo de pendência, motivo de vida não reconstruível,
/// diferença em `_TimeTemp`).
type Caso = (String, RelatorioTick, Option<String>, Option<String>, Option<String>, bool);

fn casos() -> Vec<Caso> {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let base = raiz().join("tests/referencias/ticks");
    let indice_raiz = ler_indice(&base);
    let mut dirs = vec![(String::new(), base.clone())];
    for c in indice_raiz["cenarios"].as_array().into_iter().flatten() {
        let c = c.as_str().unwrap();
        dirs.push((format!("{c}/"), base.join(c)));
    }
    let mut out = Vec::new();
    for (prefixo, dir) in dirs {
        let indice = ler_indice(&dir);
        // "*" vale para todos os ticks do cenário
        let motivo = |chave: &str, t: &str| {
            indice[chave][t].as_str().or_else(|| indice[chave]["*"].as_str()).map(str::to_owned)
        };
        let ticks = indice["ticks"].as_array().expect("indice.json sem \"ticks\"");
        assert!(!ticks.is_empty(), "{}: sem ticks", dir.display());
        for t in ticks {
            let t = t.as_str().unwrap();
            let v = VetorTick::ler(&dir.join(t), &layout).unwrap();
            let mut regras = regras_do_vetor(&v);
            let r = comparar_tick(&v, &layout, &mut regras).unwrap();
            // `_TimeTemp` (memória do tutorial, fora do save) depois do tick, quando gravado.
            let tutor = v
                .extras_depois
                .time_temp
                .filter(|&t| regras.memoria_tutor.tempo_limite != i64::from(t))
                .map(|t| format!("_TimeTemp: original {t}, port {}", regras.memoria_tutor.tempo_limite));
            let pendente = motivo("pendentes", t).or_else(|| motivo("pela_interface", t).map(|m| format!("conferido pela interface: {m}")));
            let sem_ciclos = motivo("gamecycles_fora_do_tick", t).is_some();
            out.push((format!("{prefixo}{t}"), r, pendente, motivo("vida_nao_reconstruivel", t), tutor, sem_ciclos));
        }
    }
    out
}

#[test]
fn ticks_do_original() {
    let mut falhas = Vec::new();
    for (nome, r, pendente, vida_irreconstruivel, tutor, sem_ciclos) in casos() {
        println!("{nome}: {}", r.placar());
        assert!(r.ida_e_volta.is_empty(), "{nome}: ida e volta do save falhou em {:?}", r.ida_e_volta);
        if vida_irreconstruivel.is_none() {
            // A reconstrução da vida a partir das listas bate com o original.
            assert!(r.vida_reconstruida.is_empty(), "{nome}: vida reconstruída ≠ vida_antes");
            assert!(r.vida_reconstruida_depois.is_empty(), "{nome}: vida reconstruída ≠ vida_depois");
        }
        if let Some(m) = pendente {
            println!("  pendente: {m}");
            continue;
        }
        for b in r.diferentes() {
            falhas.push(format!("{nome} {}: {:?}", b.simbolo, b.detalhes));
        }
        falhas.extend(tutor.map(|t| format!("{nome} {t}")));
        for x in r.extras.iter().filter(|x| x.original != x.port && !(sem_ciclos && x.nome == "gameCycles")) {
            falhas.push(format!("{nome} {}: original {}, port {}", x.nome, x.original, x.port));
        }
        if !r.vida.is_empty() {
            falhas.push(format!("{nome}: {} células de vida diferentes", r.vida.len()));
        }
    }
    assert!(falhas.is_empty(), "port difere do original:\n{}", falhas.join("\n"));
}
