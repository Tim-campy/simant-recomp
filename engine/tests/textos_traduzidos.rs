//! Varredura do frame de idiomas (docs/i18n.md): nenhum texto em português nos literais do código
//! fora dos testes de `engine/src` e `simant/src`. Os textos que o port mostra vêm de
//! `data/idiomas/` (chaves usadas com `tr!`/`Textos::t`), e os diagnósticos de desenvolvedor estão em
//! inglês no código. Comentários, módulos `#[cfg(test)]`, chaves de dados e nomes de opções
//! (`--sem-som`, `ninho_preto`...) não contam.
//!
//! A heurística: um literal com letra acentuada do português (ã, õ, ç, á, é, í, ó, ú, â, ê, ô, à)
//! ou, se tiver mais de uma palavra, com uma palavra comum do português que não existe em inglês.
//! Placeholders `{...}`, opções `--x` e identificadores com `_`, `.`, `/` ou `@` são ignorados.

use std::path::{Path, PathBuf};

const ACENTOS: &str = "ãõçáéíóúâêôàÃÕÇÁÉÍÓÚÂÊÔÀ";

/// Palavras do português que não são palavras do inglês (em minúsculas).
const PALAVRAS: &[&str] = &[
    "nao", "sem", "com", "para", "uma", "um", "ou", "esta", "pelo", "pela", "dos", "das", "ainda", "fora", "falta", "faltam", "arquivo",
    "pasta", "janela", "objeto", "tabela", "erro", "ligado", "desligado", "quintal", "ninho", "formiga", "jogo", "tecla", "deve",
    "precisa", "existe", "tem", "nenhum", "nenhuma", "aberto", "fechado", "clique", "plano", "vista", "modo", "botao", "lista",
    "bloco", "tamanho", "esperava", "esperado", "mapa", "som", "musica", "efeito", "cor", "cores", "linha", "linhas", "quadro",
    "desenhado", "ainda", "depois", "antes", "entre", "muito", "demais", "carregado", "gravado", "ignorado",
];

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn arquivos(dir: &Path, saida: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            arquivos(&p, saida);
        } else if p.extension().is_some_and(|x| x == "rs") {
            saida.push(p);
        }
    }
}

/// Os literais de texto (`"..."`) do código antes do primeiro `#[cfg(test)]`, com a linha, sem
/// comentários nem atributos.
fn literais(codigo: &str) -> Vec<(usize, String)> {
    let codigo = codigo.split("#[cfg(test)]").next().unwrap_or_default();
    let c: Vec<char> = codigo.chars().collect();
    let mut v = Vec::new();
    let (mut i, mut linha) = (0, 1);
    let mut inicio_da_linha = true;
    let mut em_atributo = false;
    while i < c.len() {
        let ch = c[i];
        if ch == '\n' {
            linha += 1;
            inicio_da_linha = true;
            em_atributo = false;
            i += 1;
            continue;
        }
        if inicio_da_linha && !ch.is_whitespace() {
            inicio_da_linha = false;
            em_atributo = ch == '#';
        }
        if ch == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if ch == '/' && c.get(i + 1) == Some(&'*') {
            while i + 1 < c.len() && !(c[i] == '*' && c[i + 1] == '/') {
                linha += usize::from(c[i] == '\n');
                i += 1;
            }
            i += 2;
            continue;
        }
        // Caractere entre aspas simples ('"', '\'', '\\'), para não abrir um texto.
        if ch == '\'' {
            if c.get(i + 1) == Some(&'\\') && c.get(i + 3) == Some(&'\'') {
                i += 4;
                continue;
            }
            if c.get(i + 2) == Some(&'\'') {
                i += 3;
                continue;
            }
        }
        if ch == '"' {
            let bruto = i > 0 && (c[i - 1] == 'r' || c[i - 1] == '#');
            let (comeco, mut s) = (linha, String::new());
            i += 1;
            while i < c.len() && c[i] != '"' {
                if c[i] == '\\' && !bruto {
                    i += 1;
                }
                if c.get(i) == Some(&'\n') {
                    linha += 1;
                }
                if let Some(&x) = c.get(i) {
                    s.push(x);
                }
                i += 1;
            }
            i += 1;
            if !em_atributo {
                v.push((comeco, s));
            }
            continue;
        }
        i += 1;
    }
    v
}

/// O literal parece texto em português?
fn portugues(s: &str) -> bool {
    // Sem placeholders.
    let mut limpo = String::new();
    let mut fundo = 0;
    for ch in s.chars() {
        match ch {
            '{' => fundo += 1,
            '}' => fundo -= 1,
            _ if fundo == 0 => limpo.push(ch),
            _ => {}
        }
    }
    let palavras: Vec<&str> = limpo
        .split(|c: char| c.is_whitespace() || ",;:()[]!?\"'".contains(c))
        .filter(|p| !p.is_empty() && !p.starts_with('-') && !p.contains(['_', '.', '/', '@', '\\']))
        .collect();
    if palavras.iter().any(|p| p.chars().any(|c| ACENTOS.contains(c))) {
        return true;
    }
    palavras.len() > 1 && palavras.iter().any(|p| PALAVRAS.contains(&p.to_lowercase().as_str()))
}

#[test]
fn nenhum_texto_em_portugues_no_codigo() {
    let mut todos = Vec::new();
    for d in ["engine/src", "simant/src"] {
        arquivos(&raiz().join(d), &mut todos);
    }
    assert!(todos.len() > 50, "achou os fontes: {}", todos.len());
    let mut achados = Vec::new();
    for arq in &todos {
        let codigo = std::fs::read_to_string(arq).unwrap();
        for (linha, s) in literais(&codigo) {
            if portugues(&s) {
                achados.push(format!("{}:{linha}: \"{s}\"", arq.strip_prefix(raiz()).unwrap_or(arq).display()));
            }
        }
    }
    assert!(
        achados.is_empty(),
        "textos em português no código (use data/idiomas/ e tr!, ou inglês para diagnósticos):\n{}",
        achados.join("\n")
    );
}

#[test]
fn a_heuristica_acha_o_que_deve() {
    assert!(portugues("arquivo não encontrado"));
    assert!(portugues("Máxima"));
    assert!(portugues("sem lugar livre em ({x}, {y})"));
    assert!(!portugues("--sem-som"));
    assert!(!portugues("ninho_preto"));
    assert!(!portugues("superficie"));
    assert!(!portugues("{origem} (with command line options)"));
    assert!(!portugues("mods.recusa.ja_chove"));
    let l = literais("// \"não\"\n#[serde(rename = \"ação\")]\nlet a = '\"'; let b = \"texto ok\";\n#[cfg(test)]\n\"não\"");
    assert_eq!(l, vec![(3, "texto ok".to_string())]);
}

/// Toda chave do frame citada no código (`"launcher.x"`, `"jogo.x"`, `"mods.x"`, `"lua.x"`,
/// `"cli.x"`, `"programa.x"`) existe em `data/idiomas/en.json`.
#[test]
fn chaves_usadas_existem_em_ingles() {
    let en: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(raiz().join("data/idiomas/en.json")).unwrap()).unwrap();
    let mut todos = Vec::new();
    for d in ["engine/src", "simant/src"] {
        arquivos(&raiz().join(d), &mut todos);
    }
    let prefixos = ["launcher.", "jogo.", "mods.", "lua.", "cli.", "programa."];
    let mut faltam = Vec::new();
    let mut usadas = 0;
    for arq in &todos {
        for (linha, s) in literais(&std::fs::read_to_string(arq).unwrap()) {
            let chave = prefixos.iter().any(|p| s.starts_with(p))
                && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
                && !s.ends_with('.')
                && !s.ends_with(".json");
            if chave {
                usadas += 1;
                if en.get(&s).is_none() {
                    faltam.push(format!("{}:{linha}: {s}", arq.display()));
                }
            }
        }
    }
    assert!(usadas > 100, "chaves achadas: {usadas}");
    assert!(faltam.is_empty(), "chaves sem texto em data/idiomas/en.json:\n{}", faltam.join("\n"));
}
