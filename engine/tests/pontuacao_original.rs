//! Pontuação do GAME OVER (`_CalcScore`, `1000:5530`) contra o original: sete fins de jogo provocados
//! na bancada QEMU (`tools/cenarios_fecha.py gravar pontuacao`, snapshots `rico` e `full`):
//! `_GameTime`, `_BlackLost`, `_RedLost` e `_MeHealth` escritos e `_IsGameOver` = 1 na entrada de um
//! tick; o fim de `_DoAntSim` abre o GAME OVER. Os estados foram lidos com a caixa aberta
//! (`tests/referencias/saves/FIM_*.ANT`) e os pontos e a avaliação são os da tela capturada.
//! Confere o retorno de `_CalcScore` (`_GameTime` + soma, em 32 bits: pontos acima de 65535), o
//! sentido da nota 3 (`_BlackLost` × 100 / (`_BlackLost` + `_RedLost`): sem pretas perdidas, 0) e
//! os dois ramos de tempo (Quick: × 29 / 10 e < 4100; Full: < 8100 e × 5).
//! Especificação: docs/spec/telas/mensagens.md §4.1, docs/spec/regras/simulacao-visao-geral.md.

use std::path::PathBuf;

use engine::core::avisos::{calcular_pontuacao, indice_avaliacao, linhas_do_texto, parciais, ConfigAvisos};
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::recursos::Recursos;
use engine::rules::estado::Estado;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// (arquivo, nota 3 esperada, pontos e avaliação mostrados pelo original).
const CASOS: [(&str, i64, i64, &str); 7] = [
    ("FIM_Q_TEMPO_LONGO", 75, 902_307, "Llama Repair Specialist"),
    ("FIM_Q_TEMPO_CURTO", 10, 331_894, "Weak Link in the Food Chain"),
    ("FIM_Q_SEM_BAIXAS_PRETAS", 0, 646_424, "Aphid Herder"),
    ("FIM_Q_32BITS", 75, 927_165, "Llama Repair Specialist"),
    ("FIM_F_TEMPO_LONGO", 25, 1_800_726, "Weak Link in the Food Chain"),
    ("FIM_F_TEMPO_CURTO", 100, 829_571, "Aardvark Fodder"),
    ("FIM_F_8100", 50, 1_946_265, "Weak Link in the Food Chain"),
];

#[test]
fn pontos_e_avaliacao_do_game_over_iguais_ao_original() {
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    let cfg = ConfigAvisos::de_json(&engine::recursos::marcar_referencias(&std::fs::read_to_string(raiz().join("data/mensagens.json")).unwrap())).unwrap();
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    let textos = jogo.join("SHARED.NDX").is_file().then(|| {
        let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
        linhas_do_texto(&r, cfg.fim_de_jogo.lista_avaliacoes)
    });
    for (arquivo, nota3, pontos, avaliacao) in CASOS {
        let j = JogoSalvo::abrir(&layout, &raiz().join(format!("tests/referencias/saves/{arquivo}.ANT"))).unwrap();
        let e = Estado::de_save(&j).unwrap();
        assert_eq!(e.partida.fim_de_jogo, 1, "{arquivo}: _IsGameOver");
        assert_eq!(parciais(&e, &cfg.pontuacao)[3], nota3, "{arquivo}: nota 3");
        let p = calcular_pontuacao(&e, &cfg.pontuacao);
        assert_eq!(p, pontos, "{arquivo}: pontos");
        // GAME OVER: as pretas não venceram.
        let i = indice_avaliacao(&cfg.fim_de_jogo, e.partida.tipo_jogo, false, p);
        if let Some(t) = &textos {
            assert_eq!(String::from_utf8_lossy(&t[i]), avaliacao, "{arquivo}: avaliação {i}");
        }
    }
}
