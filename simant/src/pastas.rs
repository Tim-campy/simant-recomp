//! Onde ficam as pastas do port: a pasta base (a que tem `data/`), procurada ao lado do executável
//! antes da pasta atual, para o pacote funcionar de qualquer lugar e pelo duplo clique; e a pasta de
//! dados do usuário (`SDL_GetPrefPath`), com as preferências, a configuração do launcher e o log.

use std::path::{Path, PathBuf};

/// Organização e aplicação do `SDL_GetPrefPath` (no Windows, `%APPDATA%\SimAnt recomp\SimAnt\`).
pub const ORGANIZACAO: &str = "SimAnt recomp";
pub const APLICACAO: &str = "SimAnt";

/// Pasta base do port: a primeira com `data/` entre a do executável, a atual e (para quem roda de
/// `target/<perfil>/` no repositório) até três pastas acima do executável. Sem nenhuma, a atual.
pub fn base() -> PathBuf {
    let exe = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
    let atual = std::env::current_dir().ok();
    base_entre(exe.as_deref(), atual.as_deref())
}

pub fn base_entre(exe: Option<&Path>, atual: Option<&Path>) -> PathBuf {
    let tem_dados = |p: &Path| p.join("data").is_dir();
    if let Some(e) = exe.filter(|e| tem_dados(e)) {
        return e.to_path_buf();
    }
    if let Some(a) = atual.filter(|a| tem_dados(a)) {
        return a.to_path_buf();
    }
    if let Some(e) = exe.and_then(|e| e.ancestors().skip(1).take(3).find(|p| tem_dados(p))) {
        return e.to_path_buf();
    }
    atual.map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."))
}

/// Um caminho da linha de comando ou da configuração: absoluto fica como está; relativo vale a partir
/// da pasta atual se existir lá, senão a partir da pasta base (`--mods exemplos/mods` funciona de
/// qualquer pasta).
pub fn resolver(p: &Path, base: &Path) -> PathBuf {
    if p.is_absolute() || p.exists() {
        return p.to_path_buf();
    }
    let na_base = base.join(p);
    if na_base.exists() {
        na_base
    } else {
        p.to_path_buf()
    }
}

/// Pasta de dados do usuário (criada se preciso); `None` se o sistema não der uma.
pub fn usuario() -> Option<PathBuf> {
    sdl2::filesystem::pref_path(ORGANIZACAO, APLICACAO).ok().map(PathBuf::from)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn base_prefere_a_pasta_do_executavel() {
        let raiz = std::env::temp_dir().join(format!("simant_pastas_{}", std::process::id()));
        let (exe, atual) = (raiz.join("pacote"), raiz.join("outra"));
        std::fs::create_dir_all(exe.join("data")).unwrap();
        std::fs::create_dir_all(atual.join("data")).unwrap();
        assert_eq!(base_entre(Some(&exe), Some(&atual)), exe);
        // Sem data/ ao lado do executável: a pasta atual (cargo run na raiz do repositório).
        let alvo = raiz.join("repo").join("target").join("debug");
        std::fs::create_dir_all(&alvo).unwrap();
        assert_eq!(base_entre(Some(&alvo), Some(&atual)), atual);
        // Nem na atual: subindo do executável (target/debug -> raiz do repositório).
        std::fs::create_dir_all(raiz.join("repo").join("data")).unwrap();
        let vazia = raiz.join("vazia");
        std::fs::create_dir_all(&vazia).unwrap();
        assert_eq!(base_entre(Some(&alvo), Some(&vazia)), raiz.join("repo"));
        // Caminho relativo: da pasta atual se existir, senão da base.
        std::fs::create_dir_all(exe.join("exemplos").join("mods")).unwrap();
        let rel = Path::new("exemplos").join("mods");
        assert_eq!(resolver(&rel, &exe), exe.join("exemplos").join("mods"));
        assert_eq!(resolver(Path::new("nao-existe"), &exe), PathBuf::from("nao-existe"));
        let _ = std::fs::remove_dir_all(&raiz);
    }
}
