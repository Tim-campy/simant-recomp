//! O executável usa a pasta do jogo da configuração do launcher (`--config`), com a linha de comando
//! vencendo, e acha `data/` pela pasta do executável, de qualquer pasta atual (simant/src/pastas.rs).

use std::path::PathBuf;
use std::process::Command;

fn pasta_jogo() -> Option<PathBuf> {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz.join("SimAnt-T/MAXIS/ANTWIN"));
    jogo.join("SHARED.NDX").is_file().then_some(jogo)
}

fn tmp(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("simant_cfg_launcher_{nome}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `simant` com a configuração `cfg` (JSON), rodando de uma pasta vazia e sem `SIMANT_DIR`.
fn rodar(dir: &std::path::Path, cfg: &str, args: &[&str]) -> std::process::Output {
    let arq = dir.join("launcher.json");
    std::fs::write(&arq, cfg).unwrap();
    Command::new(env!("CARGO_BIN_EXE_simant"))
        .arg("--config")
        .arg(&arq)
        .args(args)
        .env_remove("SIMANT_DIR")
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn ticks_com_a_pasta_do_jogo_da_configuracao() {
    let dir = tmp("ticks");
    // Pasta errada na configuração: o erro cita a pasta dela.
    let errada = dir.join("nao-existe");
    let cfg = serde_json::json!({ "jogo": errada }).to_string();
    let s = rodar(&dir, &cfg, &["--novo-jogo", "--ticks", "5", "--sem-som"]);
    let erro = String::from_utf8_lossy(&s.stderr);
    assert_eq!(s.status.code(), Some(1), "{erro}");
    assert!(erro.contains("nao-existe") && erro.contains("SHARED"), "{erro}");

    let Some(jogo) = pasta_jogo() else {
        eprintln!("AVISO: arquivos do jogo não encontrados; parte do teste ignorada");
        return;
    };
    // A pasta da configuração vale (de uma pasta atual sem data/ nem o jogo).
    let cfg = serde_json::json!({ "jogo": jogo, "resolucao": [800, 600], "som": false }).to_string();
    let s = rodar(&dir, &cfg, &["--novo-jogo", "--semente", "1", "--ticks", "5"]);
    let saida = String::from_utf8_lossy(&s.stdout);
    assert!(s.status.success(), "{saida}\n{}", String::from_utf8_lossy(&s.stderr));
    assert!(saida.contains("5 ticks: GameTime 0 -> 5"), "{saida}");

    // A linha de comando vence a configuração.
    let cfg = serde_json::json!({ "jogo": errada }).to_string();
    let s = rodar(&dir, &cfg, &["--novo-jogo", "--semente", "1", "--ticks", "5", "--jogo", jogo.to_str().unwrap()]);
    assert!(String::from_utf8_lossy(&s.stdout).contains("5 ticks: GameTime 0 -> 5"), "{}", String::from_utf8_lossy(&s.stderr));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn configuracao_invalida_e_ignorada_com_aviso() {
    let dir = tmp("invalida");
    let s = rodar(&dir, "{ não é json", &["--novo-jogo", "--ticks", "1", "--sem-som", "--jogo", dir.join("x").to_str().unwrap()]);
    let erro = String::from_utf8_lossy(&s.stderr);
    assert!(erro.contains("configuração do launcher ignorada"), "{erro}");
    assert_eq!(s.status.code(), Some(1), "sem o jogo, termina com erro (sem pânico)");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn captura_do_launcher_sem_janela() {
    let dir = tmp("captura");
    let png = dir.join("launcher.png");
    let s = rodar(&dir, r#"{"inicio": "novo_jogo", "tipo": "completo"}"#, &["--capturar-launcher", png.to_str().unwrap()]);
    assert!(s.status.success(), "{}", String::from_utf8_lossy(&s.stderr));
    let dec = png::Decoder::new(std::fs::File::open(&png).unwrap());
    let info = dec.read_info().unwrap();
    assert_eq!((info.info().width, info.info().height), (640, 480));
    let _ = std::fs::remove_dir_all(&dir);
}
