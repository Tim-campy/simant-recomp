//! Textos do original lidos da instalação do usuário (API de mods 0.35; docs/api-mods.md, "Textos do
//! original"; decisão de 2026-10-19 em docs/lancamento.md): os `data/*.json` do port trazem só a
//! **referência** (lista de strings do banco, segmento do `SIMANTW.EXE`, objeto de janela, título do
//! `.HLP`), e `Recursos::dados` a troca pelo texto.
//!
//! Com os arquivos do jogo (SIMANT_DIR ou ../SimAnt-T/MAXIS/ANTWIN), cada referência dos `data/*.json`
//! tem de dar exatamente o texto que os JSON traziam antes da troca (2026-10-19). O repositório não
//! guarda esses textos: guarda o SHA-256 de cada um (`ESPERADOS`). Sem os arquivos, avisa e passa.
//!
//! Para conferir contra uma cópia dos JSON antigos (fora do repositório), aponte
//! `SIMANT_TEXTOS_ANTIGOS` para a pasta dela: o teste `igual_aos_json_antigos` compara cada arquivo
//! inteiro (sem os comentários `_…`) depois de resolvido.

use std::path::PathBuf;

use engine::recursos::{Recursos, Referencia};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Recursos com os bancos de `data/jogo.json`, ou `None` (com aviso) sem os arquivos do jogo.
fn recursos() -> Option<Recursos> {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() || !jogo.join("SIMANTW.EXE").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; textos do original não conferidos", jogo.display());
        return None;
    }
    let cfg: Value = serde_json::from_str(&std::fs::read_to_string(raiz().join("data/jogo.json")).unwrap()).unwrap();
    let bancos: Vec<String> = cfg["bancos"].as_array().unwrap().iter().map(|b| b.as_str().unwrap().to_string()).collect();
    Some(Recursos::abrir(&jogo, &bancos, &raiz().join("data"), None).unwrap())
}

/// SHA-256 do texto de cada referência (arquivo, ponteiro JSON); para as peças de bitmap, das linhas
/// das peças em ordem (sem as puladas com `null`), separadas por `\n`. Gerados dos `data/*.json` de antes da troca.
const ESPERADOS: &[(&str, &str, &str)] = &[
    ("ajuda.json", "/titulo", "4761a3087d8aeb6558e4aba68fc0712104d0cd3d145ada0a0475cda9bb4bc919"),
    ("barra_menus.json", "/texto_pausa/0", "1dd707d18f52c7d58ea95b94a60aa57989fbfbf5309b80671bfedeabf8329270"),
    ("barra_menus.json", "/texto_pausa/1", "f988b4782a807a8ef0042f0abbfcb8ad0645f25af1d2f18712f73f09522d217a"),
    ("dialogos.json", "/caixa_mensagem/titulo_load", "0f93f198922d57a972e29ee5a95c1c6dff15d0617daa4712fb87dc2e028ec23e"),
    ("dialogos.json", "/caixa_mensagem/pergunta_load", "376057996b103e046c8b8ccc3b3399d5565a059f22117f4851b6d7df95e9342a"),
    ("dialogos.json", "/caixa_mensagem/titulo_quit", "d168a85f3a12fe4a48718622f3034a2da048ea8724faf8b6fecd751f69086e14"),
    ("dialogos.json", "/caixa_mensagem/titulo_mensagem", "2f77668a9dfbf8d5848b9eeb4a7145ca94c6ed9236e4a773f6dcafa5132b2f91"),
    ("dialogos.json", "/caixa_mensagem/salvo", "81ee3ef777c3cb4e1e8a03de34ddc40b1775cd7263db8a323ce7e23c6eec46e7"),
    ("dialogos.json", "/arquivo/titulo_abrir", "dd0a04202bf91bb09f50b014660b365e44ed9878b5f08091fe91470be9ecc621"),
    ("dialogos.json", "/arquivo/titulo_salvar", "12243dfd2934b9f230ec439c6bd9ad12a06dd8b27614cca4169acf4302f6d093"),
    ("dialogos.json", "/arquivo/texto_tipo", "f870a094396b19407e08a014dbc97b540573554d9b17c1cad79c13c5d9b30e5a"),
    ("dialogos.json", "/arquivo/campo_inicial", "a1ced915444c7d67277dc554a5ca106f22683d6c49b7a87d371572eb840e7be4"),
    ("janelas.json", "/castas/titulo", "a2740e05b7cb6da2021edcd76d447e86fc84f2b9313b6793b2042cf8260a96f5"),
    ("janelas.json", "/tarefas/titulo", "297b61f764977a0991f8a9300304cad3284d515fa4e42cf2e4990b3f6a7bcb59"),
    ("janelas.json", "/historico/titulo", "f54b614630eb132db48d6000a5b0716c3d906febc2085d857762bb7a237f0e74"),
    ("mensagens.json", "/faixa/velocidade/0", "569994721591543fa44a6207a160e46458aa0fe09f227a7e451de27066c2db01"),
    ("mensagens.json", "/faixa/velocidade/1", "c33577e73f2194e3b264b7752943f992df0f54e7d55b81d08ed009ecbb448ee9"),
    ("mensagens.json", "/faixa/velocidade/2", "7068ff002a13ea8c130fcc1f910c181902a36808304fb95c3e4a21e3a83d55e5"),
    ("mensagens.json", "/faixa/velocidade/3", "5edb86beaaeb9b85eaa936f56119fef0e2715072395989cf7493dd7cf1a0f19b"),
    ("mensagens.json", "/faixa/marcador_posto", "e58c60aebd3bd9e6f56fd0b68565180063986ed12284ad6ae093a20c178463f7"),
    ("mensagens.json", "/faixa/marcador_ausente", "8829667def397c7a2fc3591391a1d89b0a9a1b49a1618232d3470afb68a4e47d"),
    ("moldura_principal.json", "/texto", "268376a23d52117e56404cf6388ef49025233e90c516bb0f752c6adaf83c1be7"),
    ("quintal_mapa.json", "/titulos/0", "77427ea89a75445e488c060d2235c268aad216c803e292c937b6aa769ed38b85"),
    ("quintal_mapa.json", "/titulos/1", "7290a146676e96ad49e2787beb98bdfba46bd630bd00cf5b5046bc294d3bb266"),
    ("quintal_mapa.json", "/titulos/2", "ba6098eb5809bf446d04d3e50a18a2b88cbf1951ddd6f76cf46fea9a987a6965"),
    ("quintal_mapa.json", "/titulos/3", "750a37059e78db6b374f94aff7d1c012f1c035c88a2b482fc093d910b9a30f87"),
    ("quintal_mapa.json", "/balao_menino/pecas", "48480420f2fcd7330ce9c33bb34103acdb73d6112cc2c4c140071d3b4c0d5158"),
    ("tutorial_arquivo.json", "/nome", "bf4ff75c47e7297442184f6f23f139fef89ad815765247e3e8a730ed30172ba5"),
    ("tutorial_arquivo.json", "/titulo", "74a6bff3faa6e80f3ae25ed95cbd3b4cb4a39c0214e6284c6a960b37b39b79aa"),
];

/// Os `data/*.json` do port, em ordem.
fn arquivos_de_dados() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(raiz().join("data"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json"))
        .collect();
    v.sort();
    v
}

/// (ponteiro JSON, referência) de cada referência de texto em `v`.
fn referencias(v: &Value, caminho: String, saida: &mut Vec<(String, Referencia)>) {
    match v {
        Value::Object(o) => {
            if let Some(r) = Referencia::do_json(o) {
                saida.push((caminho, r.unwrap_or_else(|e| panic!("referência inválida: {e}"))));
                return;
            }
            for (k, x) in o {
                referencias(x, format!("{caminho}/{}", k.replace('~', "~0").replace('/', "~1")), saida);
            }
        }
        Value::Array(l) => l.iter().enumerate().for_each(|(i, x)| referencias(x, format!("{caminho}/{i}"), saida)),
        _ => {}
    }
}

fn sha256(t: &str) -> String {
    Sha256::digest(t.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// Todas as referências dos `data/*.json` estão em `ESPERADOS` (e vice-versa), sem os arquivos do jogo.
#[test]
fn toda_referencia_tem_o_texto_esperado_registrado() {
    let mut achadas = Vec::new();
    for nome in arquivos_de_dados() {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(raiz().join("data").join(&nome)).unwrap()).unwrap();
        let mut refs = Vec::new();
        referencias(&v, String::new(), &mut refs);
        achadas.extend(refs.into_iter().map(|(p, _)| (nome.clone(), p)));
    }
    let mut esperadas: Vec<(String, String)> = ESPERADOS.iter().map(|(a, p, _)| (a.to_string(), p.to_string())).collect();
    achadas.sort();
    esperadas.sort();
    assert_eq!(achadas, esperadas);
}

/// Com a instalação, cada referência resolve e dá o texto de antes da troca.
#[test]
fn referencias_dao_os_textos_de_antes() {
    let Some(r) = recursos() else { return };
    let mut falhas = Vec::new();
    for nome in arquivos_de_dados() {
        let bruto: Value = serde_json::from_str(&std::fs::read_to_string(raiz().join("data").join(&nome)).unwrap()).unwrap();
        let mut refs = Vec::new();
        referencias(&bruto, String::new(), &mut refs);
        // Todo arquivo resolve (inclusive os que não têm referências).
        let resolvido: Value = r.dados(&nome).unwrap_or_else(|e| panic!("{nome}: {e}"));
        for (ponteiro, referencia) in refs {
            let v = resolvido.pointer(&ponteiro).unwrap_or_else(|| panic!("{nome}{ponteiro}"));
            let texto = match (v, &referencia.alvo) {
                (Value::String(s), _) => s.clone(),
                (Value::Object(o), engine::recursos::Alvo::Executavel { pecas: Some(nomes), .. }) => nomes
                    .iter()
                    .flatten()
                    .flat_map(|n| o[n].as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_string()))
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => panic!("{nome}{ponteiro}: valor resolvido inesperado"),
            };
            let esperado = ESPERADOS.iter().find(|(a, p, _)| *a == nome && *p == ponteiro).map(|e| e.2);
            if esperado != Some(sha256(&texto).as_str()) {
                falhas.push(format!("{nome}{ponteiro} ({})", referencia.alvo.descrever()));
            }
        }
    }
    assert!(falhas.is_empty(), "textos diferentes dos de antes: {falhas:#?}");
}

/// Sem comentários (`_…`), para comparar os dados.
fn sem_comentarios(v: &mut Value) {
    match v {
        Value::Object(o) => {
            o.retain(|k, _| !k.starts_with('_'));
            o.values_mut().for_each(sem_comentarios);
        }
        Value::Array(l) => l.iter_mut().for_each(sem_comentarios),
        _ => {}
    }
}

/// Com `SIMANT_TEXTOS_ANTIGOS` (cópia dos `data/*.json` de antes da troca) e a instalação: cada arquivo
/// resolvido é igual ao antigo, fora os comentários.
#[test]
fn igual_aos_json_antigos() {
    let Some(antigos) = std::env::var_os("SIMANT_TEXTOS_ANTIGOS").map(PathBuf::from) else {
        eprintln!("AVISO: SIMANT_TEXTOS_ANTIGOS não definido; comparação com os JSON antigos não feita");
        return;
    };
    let Some(r) = recursos() else { return };
    let mut comparados = 0;
    for nome in arquivos_de_dados() {
        let Ok(t) = std::fs::read_to_string(antigos.join(&nome)) else { continue };
        let mut velho: Value = serde_json::from_str(&t).unwrap();
        let mut novo: Value = r.dados(&nome).unwrap();
        sem_comentarios(&mut velho);
        sem_comentarios(&mut novo);
        assert_eq!(novo, velho, "{nome}");
        comparados += 1;
    }
    assert!(comparados > 40, "só {comparados} arquivos comparados");
}
