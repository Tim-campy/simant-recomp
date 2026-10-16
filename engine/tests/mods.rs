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
