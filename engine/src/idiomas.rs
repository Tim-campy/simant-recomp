//! Frame de idiomas (i18n) do port: os textos que o **port** mostra (launcher, mensagens, avisos,
//! menus dos mods, linha de comando) vêm de `data/idiomas/<código>.json`, nunca do código. Os textos
//! do SimAnt original continuam vindo da instalação do usuário (já em inglês). Formato e regras em
//! docs/i18n.md.
//!
//! - `data/idiomas/idiomas.json` (o índice): cada idioma, por código (`en`, `pt-BR`...), com o nome
//!   mostrado e os **contextos** em que ele está disponível (`launcher`, `jogo`). Objeto por código
//!   para que um mod acrescente um idioma mesclando por chave, sem repetir os outros.
//! - `data/idiomas/<código>.json`: objeto plano `"chave": "texto"`, com parâmetros `{nome}`. Chaves
//!   que começam com `_` são comentários.
//! - Inglês (`en`) é a reserva: uma chave ausente no idioma escolhido usa o texto em inglês, com um
//!   aviso no log (uma vez por chave); ausente também em inglês, aparece a própria chave.
//! - Mods trazem `data/idiomas/<código>.json` (e o índice) e são mesclados por chave como os outros
//!   dados ([`crate::recursos::Recursos::dados`]).
//!
//! O jogo usa um [`Textos`] global ([`atual`], [`instalar`], macro [`tr!`](crate::tr)); antes de
//! [`instalar`], vale o inglês embutido (o `data/idiomas/en.json` do port, compilado junto), para
//! que testes e ferramentas tenham os textos. O launcher usa o seu próprio [`Textos`].

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Display;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

use serde::Deserialize;

use crate::recursos::{mesclar, Recursos};

/// O idioma de reserva (e o padrão).
pub const INGLES: &str = "en";
/// Pasta dos arquivos de idioma dentro de `data/`.
pub const PASTA: &str = "idiomas";
/// O índice dos idiomas.
pub const ARQUIVO_INDICE: &str = "idiomas.json";
/// Contexto do launcher.
pub const CONTEXTO_LAUNCHER: &str = "launcher";
/// Contexto do jogo (tudo depois do launcher).
pub const CONTEXTO_JOGO: &str = "jogo";

const EN_EMBUTIDO: &str = include_str!("../../data/idiomas/en.json");

/// Um idioma do índice.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct InfoIdioma {
    /// Nome mostrado no seletor (no próprio idioma: "English", "Português (Brasil)").
    pub nome: String,
    /// Onde o idioma pode ser escolhido (`launcher`, `jogo`).
    #[serde(default)]
    pub contextos: Vec<String>,
}

/// `data/idiomas/idiomas.json`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Indice {
    #[serde(default)]
    pub idiomas: BTreeMap<String, InfoIdioma>,
}

impl Indice {
    pub fn de_valor(v: serde_json::Value) -> Result<Indice, String> {
        serde_json::from_value(v).map_err(|e| format!("{PASTA}/{ARQUIVO_INDICE}: {e}"))
    }

    /// Códigos e nomes dos idiomas disponíveis em `contexto`, inglês primeiro e os outros por código.
    /// Inglês está sempre na lista (é a reserva), mesmo sem estar no índice.
    pub fn disponiveis(&self, contexto: &str) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = self
            .idiomas
            .iter()
            .filter(|(_, i)| i.contextos.iter().any(|c| c == contexto))
            .map(|(c, i)| (c.clone(), i.nome.clone()))
            .collect();
        if !v.iter().any(|(c, _)| c == INGLES) {
            v.push((INGLES.into(), "English".into()));
        }
        v.sort_by_key(|(c, _)| (c != INGLES, c.clone()));
        v
    }

    pub fn codigos(&self, contexto: &str) -> Vec<String> {
        self.disponiveis(contexto).into_iter().map(|(c, _)| c).collect()
    }
}

/// Os textos de um idioma, com o inglês de reserva.
#[derive(Debug)]
pub struct Textos {
    codigo: String,
    textos: HashMap<String, String>,
    reserva: HashMap<String, String>,
    /// Chaves já avisadas no log (ausentes no idioma ou em todos).
    avisadas: Mutex<HashSet<String>>,
}

/// Objeto JSON plano → mapa (sem as chaves `_comentário`; valores que não são texto são ignorados).
fn mapa(v: &serde_json::Value) -> HashMap<String, String> {
    v.as_object()
        .map(|o| o.iter().filter(|(k, _)| !k.starts_with('_')).filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string()))).collect())
        .unwrap_or_default()
}

impl Textos {
    /// `codigo` com os textos `atual` e o inglês `reserva` (objetos JSON planos).
    pub fn de_valores(codigo: &str, atual: &serde_json::Value, reserva: &serde_json::Value) -> Textos {
        let reserva = mapa(reserva);
        let textos = if codigo == INGLES { reserva.clone() } else { mapa(atual) };
        Textos { codigo: codigo.to_string(), textos, reserva, avisadas: Mutex::new(HashSet::new()) }
    }

    /// O inglês embutido no executável (o `data/idiomas/en.json` do port na compilação).
    pub fn embutido() -> Textos {
        let v: serde_json::Value = serde_json::from_str(EN_EMBUTIDO).expect("data/idiomas/en.json");
        Textos::de_valores(INGLES, &v, &v)
    }

    /// Lê `codigo` de `dir_dados/idiomas/` e das pastas `dirs_mods` (cada uma com `data/idiomas/`,
    /// mescladas por chave na ordem), com o inglês das mesmas pastas por cima do embutido.
    pub fn de_pastas(dir_dados: &Path, dirs_mods: &[&Path], codigo: &str) -> Textos {
        let ler = |c: &str| {
            let nome = format!("{c}.json");
            let mut v = serde_json::json!({});
            let caminhos = std::iter::once(dir_dados.join(PASTA).join(&nome)).chain(dirs_mods.iter().map(|m| m.join("data").join(PASTA).join(&nome)));
            for p in caminhos {
                match std::fs::read_to_string(&p).map(|t| serde_json::from_str::<serde_json::Value>(&t)) {
                    Ok(Ok(x)) => mesclar(&mut v, x),
                    Ok(Err(e)) => eprintln!("{}: {e}", p.display()),
                    Err(_) => {}
                }
            }
            v
        };
        Textos::montar(codigo, ler(codigo), ler(INGLES))
    }

    /// Lê `codigo` pelos dados do jogo (`data/idiomas/<código>.json` do port com os mods por cima,
    /// [`Recursos::dados`]), com o inglês de reserva.
    pub fn do_jogo(r: &Recursos, codigo: &str) -> Textos {
        let ler = |c: &str| {
            let nome = format!("{PASTA}/{c}.json");
            if !r.existe_dado(&nome) {
                return serde_json::json!({});
            }
            r.dados::<serde_json::Value>(&nome).unwrap_or_else(|e| {
                eprintln!("{e}");
                serde_json::json!({})
            })
        };
        Textos::montar(codigo, ler(codigo), ler(INGLES))
    }

    fn montar(codigo: &str, atual: serde_json::Value, ingles: serde_json::Value) -> Textos {
        let mut reserva: serde_json::Value = serde_json::from_str(EN_EMBUTIDO).unwrap_or_else(|_| serde_json::json!({}));
        mesclar(&mut reserva, ingles);
        let atual = if codigo == INGLES { reserva.clone() } else { atual };
        Textos::de_valores(codigo, &atual, &reserva)
    }

    /// O código do idioma (`en`, `pt-BR`...).
    pub fn codigo(&self) -> &str {
        &self.codigo
    }

    /// Há texto para `chave` (no idioma ou na reserva)?
    pub fn tem(&self, chave: &str) -> bool {
        self.textos.contains_key(chave) || self.reserva.contains_key(chave)
    }

    /// Há texto para `chave` no próprio idioma (sem a reserva)?
    pub fn tem_no_idioma(&self, chave: &str) -> bool {
        self.textos.contains_key(chave)
    }

    /// Todas as chaves do idioma (para conferências).
    pub fn chaves(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.textos.keys().map(String::as_str).collect();
        v.sort_unstable();
        v
    }

    fn avisar(&self, chave: &str, mensagem: impl FnOnce() -> String) {
        if let Ok(mut a) = self.avisadas.lock() {
            if a.insert(chave.to_string()) {
                eprintln!("{}", mensagem());
            }
        }
    }

    /// O texto cru de `chave`: o do idioma, senão o inglês (com aviso), senão a própria chave.
    pub fn bruto<'a>(&'a self, chave: &'a str) -> &'a str {
        if let Some(t) = self.textos.get(chave) {
            return t;
        }
        if let Some(t) = self.reserva.get(chave) {
            if self.codigo != INGLES {
                self.avisar(chave, || format!("i18n: '{chave}' missing in {}; using English", self.codigo));
            }
            return t;
        }
        self.avisar(chave, || format!("i18n: '{chave}' missing in {} and in English", self.codigo));
        chave
    }

    /// O texto de `chave`.
    pub fn t(&self, chave: &str) -> String {
        self.bruto(chave).to_string()
    }

    /// O texto de `chave` com os parâmetros `{nome}` trocados.
    pub fn f(&self, chave: &str, params: &[(&str, &dyn Display)]) -> String {
        formatar(self.bruto(chave), params)
    }

    /// Como [`Textos::f`], com os parâmetros já em texto (a API Lua).
    pub fn f_texto(&self, chave: &str, params: &[(String, String)]) -> String {
        let p: Vec<(&str, &dyn Display)> = params.iter().map(|(k, v)| (k.as_str(), v as &dyn Display)).collect();
        self.f(chave, &p)
    }
}

/// Troca `{nome}` pelos parâmetros; `{{` e `}}` viram `{` e `}`; um `{nome}` sem parâmetro fica como
/// está.
pub fn formatar(modelo: &str, params: &[(&str, &dyn Display)]) -> String {
    let mut s = String::with_capacity(modelo.len() + 16);
    let mut resto = modelo;
    while let Some(i) = resto.find(['{', '}']) {
        s.push_str(&resto[..i]);
        let r = &resto[i..];
        if r.starts_with("{{") || r.starts_with("}}") {
            s.push_str(&r[..1]);
            resto = &r[2..];
            continue;
        }
        if r.starts_with('{') {
            if let Some(fim) = r.find('}') {
                let nome = &r[1..fim];
                if let Some((_, v)) = params.iter().find(|(k, _)| *k == nome) {
                    s.push_str(&v.to_string());
                    resto = &r[fim + 1..];
                    continue;
                }
            }
        }
        s.push_str(&r[..1]);
        resto = &r[1..];
    }
    s.push_str(resto);
    s
}

/// Lê o índice de `dir_dados/idiomas/idiomas.json` (sem o arquivo: só inglês).
pub fn indice_de_pasta(dir_dados: &Path) -> Indice {
    let p = dir_dados.join(PASTA).join(ARQUIVO_INDICE);
    match std::fs::read_to_string(&p) {
        Ok(t) => serde_json::from_str::<serde_json::Value>(&t).map_err(|e| e.to_string()).and_then(Indice::de_valor).unwrap_or_else(|e| {
            eprintln!("{}: {e}", p.display());
            Indice::default()
        }),
        Err(_) => Indice::default(),
    }
}

/// O índice pelos dados do jogo (com os mods por cima).
pub fn indice_do_jogo(r: &Recursos) -> Indice {
    let nome = format!("{PASTA}/{ARQUIVO_INDICE}");
    if !r.existe_dado(&nome) {
        return Indice::default();
    }
    r.dados::<serde_json::Value>(&nome).and_then(Indice::de_valor).unwrap_or_else(|e| {
        eprintln!("{e}");
        Indice::default()
    })
}

/// Normaliza um código de idioma do sistema (`pt_BR.UTF-8`, `pt-br`, `en_US`) para a forma do port
/// (`pt-BR`, `en-US`). `C`/`POSIX` e vazios = `None`.
pub fn normalizar(codigo: &str) -> Option<String> {
    let base = codigo.split(['.', '@']).next()?.trim();
    if base.is_empty() || base.eq_ignore_ascii_case("C") || base.eq_ignore_ascii_case("POSIX") {
        return None;
    }
    let mut partes = base.split(['_', '-']);
    let lingua = partes.next()?.to_ascii_lowercase();
    match partes.next() {
        Some(r) if !r.is_empty() => Some(format!("{lingua}-{}", r.to_ascii_uppercase())),
        _ => Some(lingua),
    }
}

/// O idioma a usar: o `pedido` (se disponível), senão o primeiro dos `sistema` (em ordem de
/// preferência) que existe — exato ou pela mesma língua (`pt` ou `pt-PT` acham `pt-BR`) —, senão
/// inglês.
pub fn escolher(pedido: Option<&str>, sistema: &[String], disponiveis: &[String]) -> String {
    let achar = |c: &str| -> Option<String> {
        let c = normalizar(c)?;
        if let Some(d) = disponiveis.iter().find(|d| d.eq_ignore_ascii_case(&c)) {
            return Some(d.clone());
        }
        let lingua = c.split('-').next().unwrap_or_default();
        disponiveis.iter().find(|d| d.split('-').next().is_some_and(|l| l.eq_ignore_ascii_case(lingua))).cloned()
    };
    pedido.and_then(achar).or_else(|| sistema.iter().find_map(|s| achar(s))).unwrap_or_else(|| INGLES.to_string())
}

// ---------------------------------------------------------------------------------------------
// Os textos do jogo (globais)
// ---------------------------------------------------------------------------------------------

static ATUAL: RwLock<Option<Arc<Textos>>> = RwLock::new(None);

/// Os textos do jogo: os instalados ou, antes disso, o inglês embutido.
pub fn atual() -> Arc<Textos> {
    if let Some(t) = ATUAL.read().ok().and_then(|g| g.clone()) {
        return t;
    }
    let mut g = ATUAL.write().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(|| Arc::new(Textos::embutido())).clone()
}

/// Troca os textos do jogo (o executável, depois de abrir os dados e os mods).
pub fn instalar(t: Textos) {
    *ATUAL.write().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(t));
}

/// `tr!("chave")` ou `tr!("chave", nome = valor, ...)`: o texto de `chave` no idioma do jogo
/// ([`atual`]), com os parâmetros `{nome}` trocados.
#[macro_export]
macro_rules! tr {
    ($chave:expr) => {
        $crate::idiomas::atual().t($chave)
    };
    ($chave:expr, $($nome:ident = $valor:expr),+ $(,)?) => {
        $crate::idiomas::atual().f($chave, &[$((stringify!($nome), &$valor as &dyn ::std::fmt::Display)),+])
    };
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    fn raiz() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    #[test]
    fn parametros_e_chaves() {
        assert_eq!(formatar("Mod {nome}: {n} erros", &[("nome", &"x"), ("n", &3)]), "Mod x: 3 erros");
        assert_eq!(formatar("{{literal}} {falta} {", &[]), "{literal} {falta} {");
        assert_eq!(formatar("ação {a}", &[("a", &"ç")]), "ação ç");
    }

    #[test]
    fn reserva_em_ingles_e_chave_ausente() {
        let en = json!({"_comentario": "x", "a": "A {n}", "b": "B"});
        let pt = json!({"a": "Á {n}"});
        let t = Textos::de_valores("pt-BR", &pt, &en);
        assert_eq!(t.f("a", &[("n", &1)]), "Á 1");
        assert_eq!(t.t("b"), "B", "reserva");
        assert_eq!(t.t("c"), "c", "ausente em todos: a própria chave");
        assert!(!t.tem("_comentario"));
        assert!(t.tem_no_idioma("a") && !t.tem_no_idioma("b"));
        assert_eq!(t.codigo(), "pt-BR");
    }

    #[test]
    fn arquivos_do_port() {
        let dados = raiz().join("data");
        let indice = indice_de_pasta(&dados);
        assert_eq!(indice.codigos(CONTEXTO_LAUNCHER), vec!["en".to_string(), "pt-BR".into()]);
        assert_eq!(indice.codigos(CONTEXTO_JOGO), vec!["en".to_string()]);
        let en = Textos::de_pastas(&dados, &[], "en");
        let pt = Textos::de_pastas(&dados, &[], "pt-BR");
        // Toda chave do launcher em inglês existe em português, e vice-versa.
        for k in en.chaves().into_iter().filter(|k| k.starts_with("launcher.")) {
            assert!(pt.tem_no_idioma(k), "pt-BR sem {k}");
        }
        for k in pt.chaves() {
            assert!(en.tem_no_idioma(k), "en sem {k} (só em pt-BR)");
        }
        // O embutido é o mesmo arquivo.
        assert_eq!(Textos::embutido().chaves(), en.chaves());
    }

    #[test]
    fn mod_acrescenta_idioma_e_chaves() {
        let dir = std::env::temp_dir().join(format!("simant_idiomas_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let m = dir.join("meu-mod");
        std::fs::create_dir_all(m.join("data").join(PASTA)).unwrap();
        std::fs::write(m.join("mod.json"), r#"{"nome": "meu-mod", "versao": "1"}"#).unwrap();
        std::fs::write(
            m.join("data").join(PASTA).join(ARQUIVO_INDICE),
            r#"{"idiomas": {"es": {"nome": "Español", "contextos": ["jogo"]}}}"#,
        )
        .unwrap();
        std::fs::write(m.join("data").join(PASTA).join("es.json"), r#"{"mod.meu-mod.ola": "¡Hola, {nome}!"}"#).unwrap();
        std::fs::write(m.join("data").join(PASTA).join("en.json"), r#"{"mod.meu-mod.ola": "Hello, {nome}!", "mod.meu-mod.so_en": "only"}"#).unwrap();
        let r = Recursos::abrir(&dir.join("sem-jogo"), &[], &raiz().join("data"), Some(&dir)).unwrap();
        let indice = indice_do_jogo(&r);
        assert_eq!(indice.codigos(CONTEXTO_JOGO), vec!["en".to_string(), "es".into()], "o mod acrescenta, o port continua");
        assert_eq!(indice.codigos(CONTEXTO_LAUNCHER), vec!["en".to_string(), "pt-BR".into()]);
        let es = Textos::do_jogo(&r, "es");
        assert_eq!(es.f("mod.meu-mod.ola", &[("nome", &"Ana")]), "¡Hola, Ana!");
        assert_eq!(es.t("mod.meu-mod.so_en"), "only", "reserva em inglês do mod");
        assert!(es.tem("mods.script_desligado"), "chaves do port continuam");
        let en = Textos::do_jogo(&r, "en");
        assert_eq!(en.f("mod.meu-mod.ola", &[("nome", &"Ann")]), "Hello, Ann!");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escolha_do_idioma() {
        let disp = vec!["en".to_string(), "pt-BR".into()];
        assert_eq!(normalizar("pt_BR.UTF-8").as_deref(), Some("pt-BR"));
        assert_eq!(normalizar("C"), None);
        assert_eq!(escolher(None, &["pt_BR.UTF-8".into()], &disp), "pt-BR");
        assert_eq!(escolher(None, &["pt-PT".into()], &disp), "pt-BR", "mesma língua");
        assert_eq!(escolher(None, &["de-DE".into(), "pt".into()], &disp), "pt-BR", "ordem de preferência");
        assert_eq!(escolher(None, &["fr-FR".into()], &disp), "en");
        assert_eq!(escolher(Some("en"), &["pt-BR".into()], &disp), "en", "o pedido vence o sistema");
        assert_eq!(escolher(Some("xx"), &["pt-BR".into()], &disp), "pt-BR", "pedido indisponível: o sistema");
        assert_eq!(escolher(Some("pt-br"), &[], &disp), "pt-BR");
    }
}
