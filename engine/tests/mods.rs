//! Mods sobrescrevem objetos por (tipo, id) e arquivos de dados por nome, sem tocar no original.

use std::fs;
use std::path::{Path, PathBuf};

use engine::formats::banco::Chave;
use engine::recursos::{carregar_mods, Recursos};

fn tmp(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("simant_teste_{nome}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn criar_mod(dir: &Path, nome: &str, deps: &[&str]) -> PathBuf {
    let d = dir.join(nome);
    fs::create_dir_all(&d).unwrap();
    let deps: Vec<String> = deps.iter().map(|x| format!("\"{x}\"")).collect();
    fs::write(d.join("mod.json"), format!(r#"{{"nome":"{nome}","versao":"1.0","dependencias":[{}]}}"#, deps.join(","))).unwrap();
    d
}

/// Banco mínimo com um objeto (tipo 4, id 7) sem compressão.
fn banco_minimo(dir: &Path) {
    let conteudo = b"original";
    let mut dat = Vec::new();
    dat.extend_from_slice(&0x1234_5678u32.to_le_bytes());
    dat.extend_from_slice(&[0; 10]);
    dat.extend_from_slice(&0x1234_5678u32.to_le_bytes());
    dat.extend_from_slice(&0u16.to_le_bytes());
    dat.extend_from_slice(&(conteudo.len() as u32).to_le_bytes());
    dat.extend_from_slice(conteudo);
    let mut ndx = Vec::new();
    ndx.extend_from_slice(&1u32.to_le_bytes());
    ndx.extend_from_slice(&[0; 16]);
    ndx.extend_from_slice(&0u32.to_le_bytes());
    ndx.extend_from_slice(&7u16.to_le_bytes());
    ndx.extend_from_slice(&[4, 0]);
    fs::write(dir.join("TESTE.NDX"), ndx).unwrap();
    fs::write(dir.join("TESTE.DAT"), dat).unwrap();
}

#[test]
fn mod_sobrescreve_objeto_e_dados() {
    let raiz = tmp("sobrescreve");
    let (jogo, dados, mods) = (raiz.join("jogo"), raiz.join("data"), raiz.join("mods"));
    for d in [&jogo, &dados, &mods] {
        fs::create_dir_all(d).unwrap();
    }
    banco_minimo(&jogo);
    fs::write(dados.join("x.json"), r#"{"v": 1}"#).unwrap();

    let r = Recursos::abrir(&jogo, &["TESTE".into()], &dados, Some(&mods)).unwrap();
    assert_eq!(r.objeto(Chave { tipo: 4, id: 7 }).unwrap(), b"original");
    assert_eq!(r.dados::<serde_json::Value>("x.json").unwrap()["v"], 1);

    let m = criar_mod(&mods, "meu-mod", &[]);
    fs::create_dir_all(m.join("objetos/4")).unwrap();
    fs::write(m.join("objetos/4/7.bin"), b"do mod").unwrap();
    fs::create_dir_all(m.join("data")).unwrap();
    fs::write(m.join("data/x.json"), r#"{"v": 2}"#).unwrap();

    let r = Recursos::abrir(&jogo, &["TESTE".into()], &dados, Some(&mods)).unwrap();
    assert_eq!(r.objeto(Chave { tipo: 4, id: 7 }).unwrap(), b"do mod");
    assert_eq!(r.dados::<serde_json::Value>("x.json").unwrap()["v"], 2);
    // o original não foi tocado
    assert!(fs::read(jogo.join("TESTE.DAT")).unwrap().ends_with(b"original"));
}

#[test]
fn ordem_respeita_dependencias() {
    let mods = tmp("ordem");
    criar_mod(&mods, "a-depende-de-z", &["z-base"]);
    criar_mod(&mods, "z-base", &[]);
    let ordem: Vec<String> = carregar_mods(&mods).unwrap().into_iter().map(|m| m.manifesto.nome).collect();
    assert_eq!(ordem, vec!["z-base", "a-depende-de-z"]);
}

#[test]
fn dependencia_ausente_e_erro() {
    let mods = tmp("ausente");
    criar_mod(&mods, "orfao", &["nao-existe"]);
    assert!(carregar_mods(&mods).is_err());
}

/// Banco com um único objeto (tipo, id) sem compressão, no mesmo formato de [`banco_minimo`].
fn banco_com(dir: &Path, nome: &str, tipo: u8, id: u16, conteudo: &[u8]) {
    let mut dat = Vec::new();
    dat.extend_from_slice(&0x1234_5678u32.to_le_bytes());
    dat.extend_from_slice(&[0; 10]);
    dat.extend_from_slice(&0x1234_5678u32.to_le_bytes());
    dat.extend_from_slice(&0u16.to_le_bytes());
    dat.extend_from_slice(&(conteudo.len() as u32).to_le_bytes());
    dat.extend_from_slice(conteudo);
    let mut ndx = Vec::new();
    ndx.extend_from_slice(&1u32.to_le_bytes());
    ndx.extend_from_slice(&[0; 16]);
    ndx.extend_from_slice(&0u32.to_le_bytes());
    ndx.extend_from_slice(&id.to_le_bytes());
    ndx.extend_from_slice(&[tipo, 0]);
    fs::write(dir.join(format!("{nome}.NDX")), ndx).unwrap();
    fs::write(dir.join(format!("{nome}.DAT")), dat).unwrap();
}

fn dados_do_port() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data")
}

/// Cópia de `data/<arquivo>` do port com uma alteração, gravada em `<mod>/data/<arquivo>`.
fn sobrescrever(m: &Path, arquivo: &str, mudar: impl FnOnce(&mut serde_json::Value)) {
    let texto = fs::read_to_string(dados_do_port().join(arquivo)).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&texto).unwrap();
    mudar(&mut v);
    fs::create_dir_all(m.join("data")).unwrap();
    fs::write(m.join("data").join(arquivo), v.to_string()).unwrap();
}

/// API de mods 0.5: os `data/*.json` das regras do tick, sobrescritos por um mod, chegam a
/// `RegrasOriginais` (e daí ao tick).
#[test]
fn mod_das_regras_chega_ao_tick() {
    use engine::rules::estado::Estado;
    use engine::rules::tick::{tick, RegrasOriginais};

    let raiz = tmp("regras");
    let (jogo, mods) = (raiz.join("jogo"), raiz.join("mods"));
    fs::create_dir_all(&jogo).unwrap();
    // O seno da geração (objeto 9/1000: 64 palavras big-endian).
    let seno: Vec<u8> = (0..64)
        .flat_map(|i| (((i as f64 * std::f64::consts::PI / 128.0).sin() * 32767.0) as i16).to_be_bytes())
        .collect();
    banco_com(&jogo, "SENO", 9, 1000, &seno);
    let bancos = ["SENO".to_string()];

    // Sem mod: as tabelas são as de `data/`, iguais às embutidas.
    let r = Recursos::abrir(&jogo, &bancos, &dados_do_port(), Some(&mods)).unwrap();
    let regras = RegrasOriginais::carregar(&r).unwrap();
    let padrao = RegrasOriginais::default();
    assert_eq!(regras.superficie.casta_por_escolha, padrao.superficie.casta_por_escolha);
    assert!(regras.geracao.is_some());

    let m = criar_mod(&mods, "regras-mod", &[]);
    sobrescrever(&m, "superficie.json", |v| v["casta_por_escolha"][0] = 5.into());
    sobrescrever(&m, "ninho.json", |v| v["vitoria_vermelha"][0] = 7.into());
    sobrescrever(&m, "quintal.json", |v| v["regras"]["gato_chance"] = 64.into());
    sobrescrever(&m, "inimigos.json", |v| v["regras"]["aranha_agonia"] = 123.into());
    sobrescrever(&m, "amarela.json", |v| v["regras"]["vida_maxima"] = 90.into());
    sobrescrever(&m, "feromonios.json", |v| v["regras"]["limiar_trilha"] = 9.into());
    let r = Recursos::abrir(&jogo, &bancos, &dados_do_port(), Some(&mods)).unwrap();
    let mut regras = RegrasOriginais::carregar(&r).unwrap();
    assert_eq!(regras.superficie.casta_por_escolha[0], 5);
    assert_eq!(regras.ninho.vitoria_vermelha[0], 7);
    assert_eq!(regras.quintal.regras.gato_chance, 64);
    assert_eq!(regras.inimigos.regras.aranha_agonia, 123);
    assert_eq!(regras.amarela.regras.vida_maxima, 90);
    assert_eq!(regras.feromonios.regras.limiar_trilha, 9);

    // No tick: `_SetCasteProd` escolhe a entrada 0 de `casta_por_escolha` num estado vazio.
    let mut e = Estado::vazio();
    tick(&mut e, &mut regras);
    assert_eq!(e.controles.proxima_casta, 5);
    let mut e = Estado::vazio();
    tick(&mut e, &mut RegrasOriginais::default());
    assert_eq!(e.controles.proxima_casta, padrao.superficie.casta_por_escolha[0]);
}

/// API de mods 0.20: as tabelas que o original tem uma vez só ficam em `data/comum.json`. Um mod que o
/// troca muda a tabela para todas as partes; um mod antigo que ainda traz a cópia no arquivo de uma
/// parte muda só aquela parte (com aviso no console), como antes.
#[test]
fn mod_de_comum_json_vale_para_todas_as_partes() {
    use engine::rules::tick::RegrasOriginais;

    let raiz = tmp("comum");
    let (jogo, mods) = (raiz.join("jogo"), raiz.join("mods"));
    fs::create_dir_all(&jogo).unwrap();
    let r = Recursos::abrir(&jogo, &[], &dados_do_port(), Some(&mods)).unwrap();
    let padrao = RegrasOriginais::carregar_sem_banco(&r).unwrap();
    assert_eq!(padrao.ninho.classe_luta, padrao.superficie.classe_luta);
    assert_eq!(padrao.ninho.chance_vitoria, padrao.amarela.matriz_luta);
    assert_eq!(padrao.amarela.descarregar_grupo.map(|g| g as i8), padrao.superficie.grupo_sem_carga);

    // Mod novo: comum.json com a classe de luta do grupo 0 trocada.
    let m = criar_mod(&mods, "comum-mod", &[]);
    sobrescrever(&m, "comum.json", |v| v["classe_luta"][0] = 3.into());
    let r = Recursos::abrir(&jogo, &[], &dados_do_port(), Some(&mods)).unwrap();
    let regras = RegrasOriginais::carregar_sem_banco(&r).unwrap();
    assert_eq!(regras.superficie.classe_luta[0], 3);
    assert_eq!(regras.ninho.classe_luta[0], 3);
    assert_eq!(regras.amarela.classe_luta[0], 3);

    // Mod antigo (API ≤ 0.19): a cópia de `turn_tab` no ninho.json vale só para o ninho.
    fs::remove_file(m.join("data/comum.json")).unwrap();
    sobrescrever(&m, "ninho.json", |v| {
        v["turn_tab"] = serde_json::to_value([[1u8; 8]; 8]).unwrap();
    });
    let r = Recursos::abrir(&jogo, &[], &dados_do_port(), Some(&mods)).unwrap();
    let regras = RegrasOriginais::carregar_sem_banco(&r).unwrap();
    assert_eq!(regras.ninho.turn_tab, [[1; 8]; 8]);
    assert_eq!(regras.inimigos.turn_tab, padrao.inimigos.turn_tab);
    assert_eq!(regras.feromonios.turn_tab, padrao.feromonios.turn_tab);

    // API 0.26: um comum.json de mod sem uma tabela herda a do port (mesclagem por chave)...
    fs::remove_file(m.join("data/ninho.json")).unwrap();
    sobrescrever(&m, "comum.json", |v| {
        v.as_object_mut().unwrap().remove("borda_buraco");
    });
    let r = Recursos::abrir(&jogo, &[], &dados_do_port(), Some(&mods)).unwrap();
    let regras = RegrasOriginais::carregar_sem_banco(&r).unwrap();
    assert_eq!(regras.ninho.borda_buraco, padrao.ninho.borda_buraco);
    // ... e, com "_substituir", o arquivo do mod vale inteiro: a tabela que falta é erro, com o nome
    // do arquivo.
    sobrescrever(&m, "comum.json", |v| {
        v.as_object_mut().unwrap().remove("borda_buraco");
        v["_substituir"] = true.into();
    });
    let r = Recursos::abrir(&jogo, &[], &dados_do_port(), Some(&mods)).unwrap();
    let erro = RegrasOriginais::carregar_sem_banco(&r).unwrap_err();
    assert!(erro.contains("comum.json") && erro.contains("borda_buraco"), "{erro}");
}

/// API de mods 0.26: os `data/*.json` dos mods são mesclados por chave sobre os do port, na ordem
/// dos mods: objetos mesclam, listas e valores substituem, `"_substituir": true` troca o objeto
/// inteiro. Um mod que traz o arquivo inteiro dá o mesmo resultado de antes.
#[test]
fn dados_de_mods_sao_mesclados_por_chave() {
    use serde_json::{json, Value};

    let raiz = tmp("mesclar");
    let (jogo, dados, mods) = (raiz.join("jogo"), raiz.join("data"), raiz.join("mods"));
    for d in [&jogo, &dados, &mods] {
        fs::create_dir_all(d).unwrap();
    }
    let base = json!({
        "a": 1,
        "lista": [1, 2, 3],
        "obj": { "x": 1, "y": 2, "fundo": { "p": true, "q": false } },
        "mapa": { "F1": "ajuda", "F2": "pausa" }
    });
    fs::write(dados.join("x.json"), base.to_string()).unwrap();
    let a = criar_mod(&mods, "a-parcial", &[]);
    fs::create_dir_all(a.join("data")).unwrap();
    fs::write(a.join("data/x.json"), r#"{"obj": {"y": 20, "fundo": {"q": true}, "novo": "n"}, "lista": [9]}"#).unwrap();
    let b = criar_mod(&mods, "b-substitui", &["a-parcial"]);
    fs::create_dir_all(b.join("data")).unwrap();
    fs::write(b.join("data/x.json"), r#"{"mapa": {"_substituir": true, "F3": "mapa"}, "a": null}"#).unwrap();

    let r = Recursos::abrir(&jogo, &[], &dados, Some(&mods)).unwrap();
    let v: Value = r.dados("x.json").unwrap();
    assert_eq!(
        v,
        json!({
            "a": null,
            "lista": [9],
            "obj": { "x": 1, "y": 20, "fundo": { "p": true, "q": true }, "novo": "n" },
            "mapa": { "F3": "mapa" }
        })
    );

    // Um mod que traz o arquivo inteiro (como até a 0.25) dá exatamente o arquivo do mod.
    fs::remove_dir_all(&b).unwrap();
    let inteiro = json!({ "a": 5, "lista": [], "obj": { "x": 0, "y": 0, "fundo": { "p": false, "q": false } }, "mapa": { "F1": "x", "F2": "y" } });
    fs::write(a.join("data/x.json"), inteiro.to_string()).unwrap();
    let r = Recursos::abrir(&jogo, &[], &dados, Some(&mods)).unwrap();
    assert_eq!(r.dados::<Value>("x.json").unwrap(), inteiro);

    // Um arquivo que só existe no mod entra inteiro (sem as marcas).
    fs::write(a.join("data/so_do_mod.json"), r#"{"k": {"_substituir": true, "v": 1}}"#).unwrap();
    let r = Recursos::abrir(&jogo, &[], &dados, Some(&mods)).unwrap();
    assert_eq!(r.dados::<Value>("so_do_mod.json").unwrap(), json!({ "k": { "v": 1 } }));

    // Um mod parcial de verdade: só a chance do gato em quintal.json.
    let m = criar_mod(&mods, "c-gato", &[]);
    fs::create_dir_all(m.join("data")).unwrap();
    fs::write(m.join("data/quintal.json"), r#"{"regras": {"gato_chance": 64}}"#).unwrap();
    let r = Recursos::abrir(&jogo, &[], &dados_do_port(), Some(&mods)).unwrap();
    let regras = engine::rules::tick::RegrasOriginais::carregar_sem_banco(&r).unwrap();
    let padrao = engine::rules::tick::RegrasOriginais::default();
    assert_eq!(regras.quintal.regras.gato_chance, 64);
    assert_eq!(regras.quintal.regras.chuva_chance_1, padrao.quintal.regras.chuva_chance_1);
    // Erro depois da mesclagem cita o arquivo e os mods.
    fs::write(m.join("data/quintal.json"), r#"{"regras": {"gato_chance": "muito"}}"#).unwrap();
    let r = Recursos::abrir(&jogo, &[], &dados_do_port(), Some(&mods)).unwrap();
    let erro = engine::rules::tick::RegrasOriginais::carregar_sem_banco(&r).unwrap_err();
    assert!(erro.contains("quintal.json") && erro.contains("c-gato"), "{erro}");
}
