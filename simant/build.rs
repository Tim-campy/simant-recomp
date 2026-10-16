// Liga com o SDL2 pré-compilado em vendor/ (ver docs/decisoes.md) e copia o SDL2.dll para junto do executável.
use std::path::PathBuf;

fn main() {
    let raiz = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("..");
    let lib = raiz.join("vendor/SDL2-2.30.9/lib/x64");
    println!("cargo:rerun-if-changed=build.rs");
    if lib.is_dir() {
        println!("cargo:rustc-link-search=native={}", lib.display());
        // target/<perfil>/ fica três níveis acima de OUT_DIR/../../..
        let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        if let Some(perfil) = out.ancestors().nth(3) {
            let _ = std::fs::copy(lib.join("SDL2.dll"), perfil.join("SDL2.dll"));
        }
    } else {
        println!("cargo:warning=vendor/SDL2-2.30.9 não encontrado; o link com SDL2 pode falhar");
    }
}
