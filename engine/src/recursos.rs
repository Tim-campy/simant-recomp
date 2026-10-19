//! Acesso unificado a objetos do banco original e a arquivos de dados do port, com mods por cima.
//! Regras dos mods: docs/api-mods.md.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::formats::banco::{Banco, Chave};
use crate::formats::textos::ListaStrings;

#[derive(Debug, Deserialize)]
pub struct Manifesto {
    pub nome: String,
    pub versao: String,
    #[serde(default)]
    pub dependencias: Vec<String>,
    /// Arquivos `.lua` do mod, em ordem de carga (API 0.12; ver `scripts.rs` e docs/api-mods.md).
    #[serde(default)]
    pub scripts: Vec<String>,
}

#[derive(Debug)]
pub struct Mod {
    pub manifesto: Manifesto,
    pub dir: PathBuf,
}

pub struct Recursos {
    bancos: Vec<Banco>,
    dir_jogo: PathBuf,
    dir_dados: PathBuf,
    /// Em ordem de aplicação: um mod posterior sobrescreve um anterior.
    mods: Vec<Mod>,
    /// Arquivos e bancos lidos só pelas referências de texto (executável, ajuda, um banco que não
    /// está em `bancos`), para não reler a cada `dados()`.
    cache_textos: Mutex<CacheTextos>,
}

#[derive(Default)]
struct CacheTextos {
    arquivos: HashMap<String, std::sync::Arc<Vec<u8>>>,
    bancos: HashMap<String, Banco>,
}

impl Recursos {
    /// `dir_jogo`: pasta com os bancos originais (ex.: SimAnt-T/MAXIS/ANTWIN).
    /// `nomes_bancos`: em ordem de busca, como no original (`_db_LoadObject` percorre os bancos abertos).
    pub fn abrir(
        dir_jogo: &Path,
        nomes_bancos: &[String],
        dir_dados: &Path,
        dir_mods: Option<&Path>,
    ) -> Result<Recursos, String> {
        let bancos = nomes_bancos
            .iter()
            .map(|n| Banco::abrir(&dir_jogo.join(n)))
            .collect::<Result<Vec<_>, _>>()?;
        let mods = match dir_mods {
            Some(d) if d.is_dir() => carregar_mods(d)?,
            _ => Vec::new(),
        };
        Ok(Recursos {
            bancos,
            dir_jogo: dir_jogo.to_path_buf(),
            dir_dados: dir_dados.to_path_buf(),
            mods,
            cache_textos: Mutex::new(CacheTextos::default()),
        })
    }

    pub fn mods(&self) -> &[Mod] {
        &self.mods
    }

    /// Objeto (tipo, id): primeiro nos mods (do último para o primeiro), depois nos bancos originais.
    pub fn objeto(&self, chave: Chave) -> Result<Vec<u8>, String> {
        for m in self.mods.iter().rev() {
            let caminho = m.dir.join("objetos").join(chave.tipo.to_string()).join(format!("{}.bin", chave.id));
            if caminho.is_file() {
                return std::fs::read(&caminho).map_err(|e| format!("{}: {e}", caminho.display()));
            }
        }
        for b in &self.bancos {
            if let Some(r) = b.objeto(chave) {
                return r;
            }
        }
        Err(format!("objeto não encontrado: tipo {} id {}", chave.tipo, chave.id))
    }

    /// Arquivo solto da pasta do jogo (ex.: `font1`, `FONTRES.FON`, `sound/antthme1.mid`), sem
    /// diferenciar maiúsculas em nenhuma parte do caminho (separador `/`).
    /// Um mod pode substituí-lo em `arquivos/<nome>`.
    pub fn arquivo(&self, nome: &str) -> Result<Vec<u8>, String> {
        for m in self.mods.iter().rev() {
            if let Some(c) = achar_sem_caixa(&m.dir.join("arquivos"), nome) {
                return std::fs::read(&c).map_err(|e| format!("{}: {e}", c.display()));
            }
        }
        match achar_sem_caixa(&self.dir_jogo, nome) {
            Some(c) => std::fs::read(&c).map_err(|e| format!("{}: {e}", c.display())),
            None => Err(format!("arquivo do jogo não encontrado: {nome}")),
        }
    }

    /// Arquivo de dados JSON do port (`data/<nome>`) com os mods por cima, **mesclados por chave**
    /// (API de mods 0.26, [`mesclar`]): cada mod que traz `data/<nome>`, na ordem dos mods, é mesclado
    /// sobre o resultado anterior. Um mod que traz o arquivo inteiro dá o mesmo resultado de antes,
    /// salvo as chaves que ele omitia (agora vêm do port); `"_substituir": true` num objeto troca o
    /// objeto inteiro, como até a 0.25.
    pub fn dados<T: DeserializeOwned>(&self, nome: &str) -> Result<T, String> {
        let (valor, origem) = self.dados_mesclados(nome)?;
        serde_json::from_value(valor).map_err(|e| format!("{origem}: {e}"))
    }

    /// O JSON de `data/<nome>` já mesclado e a descrição de onde veio (para as mensagens de erro).
    fn dados_mesclados(&self, nome: &str) -> Result<(serde_json::Value, String), String> {
        let ler = |caminho: &Path| -> Result<serde_json::Value, String> {
            let texto = std::fs::read_to_string(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
            serde_json::from_str(&texto).map_err(|e| format!("{}: {e}", caminho.display()))
        };
        let base = self.dir_dados.join(nome);
        let mut valor = if base.is_file() { Some(ler(&base)?) } else { None };
        let mut origem = base.display().to_string();
        let mut dos_mods = Vec::new();
        for m in &self.mods {
            let c = m.dir.join("data").join(nome);
            if !c.is_file() {
                continue;
            }
            let v = ler(&c)?;
            valor = Some(match valor {
                Some(mut atual) => {
                    mesclar(&mut atual, v);
                    atual
                }
                None => {
                    let mut v = v;
                    tirar_marcas(&mut v);
                    v
                }
            });
            dos_mods.push(m.manifesto.nome.clone());
        }
        if !dos_mods.is_empty() {
            origem = format!("{origem} (mesclado com os mods {})", dos_mods.join(", "));
        }
        let mut valor = valor.ok_or_else(|| format!("{}: arquivo de dados não encontrado", base.display()))?;
        self.resolver_referencias(&mut valor).map_err(|e| format!("{origem}: {e}"))?;
        Ok((valor, origem))
    }

    /// Troca, em `v`, cada referência de texto do original ([`Referencia`], API de mods 0.35) pelo
    /// texto lido da instalação do usuário. Um texto literal (string) fica como está: é como um mod
    /// sobrescreve uma referência.
    pub fn resolver_referencias(&self, v: &mut serde_json::Value) -> Result<(), String> {
        percorrer_referencias(v, &mut String::new(), &mut |r, caminho| {
            self.resolver(r).map_err(|e| {
                format!("{caminho}: texto do original {} não resolvido: {e} (um mod pode trazer o texto no lugar da referência)", r.alvo.descrever())
            })
        })
    }

    /// O valor de uma referência: um texto ou, para `pecas_8x8`, o objeto com as peças.
    pub fn resolver(&self, r: &Referencia) -> Result<serde_json::Value, String> {
        let bytes = match &r.alvo {
            Alvo::Lista { banco, id, indice } => {
                let d = self.objeto_de(banco, Chave { tipo: crate::formats::textos::TIPO_STRINGS, id: *id })?;
                let l = ListaStrings::decodificar(&d)?;
                l.strings.get(*indice).cloned().ok_or_else(|| format!("a lista tem {} strings", l.strings.len()))?
            }
            Alvo::ObjetoJanela { banco, id, objeto } => {
                let d = self.objeto_de(banco, Chave { tipo: crate::formats::janela::TIPO_LAYOUT, id: *id })?;
                let j = crate::formats::janela::Janela::decodificar(&d, None)?;
                match j.objetos.get(*objeto).map(|o| &o.conteudo) {
                    Some(crate::formats::janela::Conteudo::Titulo { texto } | crate::formats::janela::Conteudo::Texto { texto, .. }) => {
                        return Ok(serde_json::Value::String(r.formatar(texto)));
                    }
                    Some(_) => return Err(format!("o objeto {objeto} não tem texto")),
                    None => return Err(format!("a janela tem {} objetos", j.objetos.len())),
                }
            }
            Alvo::Executavel { arquivo, segmento, offset, tamanho, pecas } => {
                let exe = self.arquivo_em_cache(arquivo)?;
                let seg = crate::formats::ne::segmento(&exe, *segmento)?;
                let resto = seg.get(*offset..).ok_or_else(|| format!("offset fora do segmento ({} bytes)", seg.len()))?;
                if let Some(nomes) = pecas {
                    return pecas_8x8(resto, nomes);
                }
                match tamanho {
                    Some(t) => resto.get(..*t).ok_or("tamanho passa do fim do segmento")?.to_vec(),
                    None => resto.iter().take_while(|&&b| b != 0).copied().collect(),
                }
            }
            Alvo::TituloAjuda { arquivo } => crate::formats::hlp::ArquivoAjuda::titulo_de(&self.arquivo_em_cache(arquivo)?)?.chars().map(|c| c as u8).collect(),
        };
        Ok(serde_json::Value::String(r.formatar(&latin1(&bytes))))
    }

    /// Objeto (tipo, id) de um banco pelo nome: primeiro os mods (`objetos/`), depois o banco aberto
    /// com esse nome ou, se ele não está entre os abertos (ex.: um teste que só abre o `SHARED`), o
    /// banco lido da pasta do jogo.
    fn objeto_de(&self, banco: &str, chave: Chave) -> Result<Vec<u8>, String> {
        for m in self.mods.iter().rev() {
            let caminho = m.dir.join("objetos").join(chave.tipo.to_string()).join(format!("{}.bin", chave.id));
            if caminho.is_file() {
                return std::fs::read(&caminho).map_err(|e| format!("{}: {e}", caminho.display()));
            }
        }
        if let Some(b) = self.bancos.iter().find(|b| b.nome.eq_ignore_ascii_case(banco)) {
            return b.objeto(chave).unwrap_or_else(|| Err(format!("{banco} não tem o objeto {}/{}", chave.tipo, chave.id)));
        }
        let mut cache = self.cache_textos.lock().map_err(|_| "cache de textos envenenado")?;
        if !cache.bancos.contains_key(banco) {
            let b = Banco::abrir(&self.dir_jogo.join(banco))?;
            cache.bancos.insert(banco.to_string(), b);
        }
        cache.bancos[banco].objeto(chave).unwrap_or_else(|| Err(format!("{banco} não tem o objeto {}/{}", chave.tipo, chave.id)))
    }

    /// [`Recursos::arquivo`] guardado em memória (o executável é lido uma vez por sessão).
    fn arquivo_em_cache(&self, nome: &str) -> Result<std::sync::Arc<Vec<u8>>, String> {
        let mut cache = self.cache_textos.lock().map_err(|_| "cache de textos envenenado")?;
        if let Some(d) = cache.arquivos.get(nome) {
            return Ok(d.clone());
        }
        let d = std::sync::Arc::new(self.arquivo(nome)?);
        cache.arquivos.insert(nome.to_string(), d.clone());
        Ok(d)
    }

    /// `nome` e os mods que trazem `data/<nome>` (para mensagens de erro de quem lê o JSON mesclado
    /// por [`Recursos::dados`] e o converte depois).
    pub fn descrever_dados(&self, nome: &str) -> String {
        let dos_mods: Vec<&str> =
            self.mods.iter().filter(|m| m.dir.join("data").join(nome).is_file()).map(|m| m.manifesto.nome.as_str()).collect();
        if dos_mods.is_empty() {
            nome.to_string()
        } else {
            format!("{nome} (mesclado com os mods {})", dos_mods.join(", "))
        }
    }

    /// Há um `data/<nome>` (no port ou num mod)? Para arquivos de dados opcionais.
    pub fn existe_dado(&self, nome: &str) -> bool {
        self.dir_dados.join(nome).is_file() || self.mods.iter().any(|m| m.dir.join("data").join(nome).is_file())
    }
}

/// Chave que, num objeto de um `data/*.json` de mod, pede a troca do objeto inteiro em vez da
/// mesclagem (API de mods 0.26).
pub const MARCA_SUBSTITUIR: &str = "_substituir";

/// Mesclagem por chave de um `data/*.json` de mod (`por_cima`) sobre o atual (API de mods 0.26):
///
/// - objeto sobre objeto: cada chave do mod é mesclada na chave de mesmo nome (recursivamente); as
///   chaves que o mod não traz ficam como estão;
/// - objeto do mod com `"_substituir": true`: troca o objeto inteiro (sem a marca);
/// - qualquer outro caso (listas, números, textos, `null`, tipos diferentes): o valor do mod
///   substitui o atual. Listas **não** são mescladas item a item.
pub fn mesclar(atual: &mut serde_json::Value, por_cima: serde_json::Value) {
    use serde_json::Value;
    match (atual, por_cima) {
        (Value::Object(a), Value::Object(mut b)) if !substituir(&b) => {
            b.remove(MARCA_SUBSTITUIR);
            for (k, v) in b {
                match a.get_mut(&k) {
                    Some(dest) => mesclar(dest, v),
                    None => {
                        let mut v = v;
                        tirar_marcas(&mut v);
                        a.insert(k, v);
                    }
                }
            }
        }
        (atual, mut v) => {
            tirar_marcas(&mut v);
            *atual = v;
        }
    }
}

fn substituir(o: &serde_json::Map<String, serde_json::Value>) -> bool {
    o.get(MARCA_SUBSTITUIR).and_then(serde_json::Value::as_bool).unwrap_or(false)
}

/// Tira as marcas `_substituir` de um valor que entra inteiro (não chegam às estruturas do port).
fn tirar_marcas(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(o) => {
            o.remove(MARCA_SUBSTITUIR);
            o.values_mut().for_each(tirar_marcas);
        }
        serde_json::Value::Array(l) => l.iter_mut().for_each(tirar_marcas),
        _ => {}
    }
}

/// Onde está um texto do original na instalação do usuário (API de mods 0.35; docs/api-mods.md,
/// "Textos do original"). Os textos do SimAnt não ficam nos `data/*.json` do port: no lugar de cada
/// um vai um objeto JSON com uma destas formas, que [`Recursos::dados`] troca pelo texto:
///
/// - `{"banco": "SHARED", "tipo": 4, "id": 1010, "indice": 7}`: string `indice` (a partir de 0) da
///   lista de strings tipo 4;
/// - `{"banco": "WINGANT", "tipo": 0, "id": 19, "objeto": 1}`: texto do objeto `objeto` do layout
///   de janela (título, tipo 18, ou texto, tipos 5, 9, 16, 17);
/// - `{"exe": "SIMANTW.EXE", "segmento": 10, "offset": "0x014b"}`: texto ASCIIZ no segmento
///   (numeração do `.SYM`: 10 = DGROUP) do executável; com `"tamanho": n`, exatamente `n` bytes;
///   com `"pecas_8x8": [nomes]`, peças de bitmap 8 × 8 de 16 bytes cada, na ordem do executável
///   (`null` pula uma peça que o port não usa; ver [`pecas_8x8`]);
/// - `{"hlp": "SIMANT.HLP", "campo": "titulo"}`: título do arquivo de ajuda.
///
/// Opcional em todas: `"formato"`, com `{}` no lugar do texto (ex.: `"{} - SIMANT.HLP"`). Chaves que
/// começam com `_` são comentários. Os bytes do original viram caracteres Latin-1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Referencia {
    pub alvo: Alvo,
    pub formato: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Alvo {
    Lista { banco: String, id: u16, indice: usize },
    ObjetoJanela { banco: String, id: u16, objeto: usize },
    Executavel { arquivo: String, segmento: u16, offset: usize, tamanho: Option<usize>, pecas: Option<Vec<Option<String>>> },
    TituloAjuda { arquivo: String },
}

impl Alvo {
    /// Descrição curta, para mensagens de erro e para o texto de [`marcar_referencias`].
    pub fn descrever(&self) -> String {
        match self {
            Alvo::Lista { banco, id, indice } => format!("{banco} 4/{id}[{indice}]"),
            Alvo::ObjetoJanela { banco, id, objeto } => format!("{banco} 0/{id} objeto {objeto}"),
            Alvo::Executavel { arquivo, segmento, offset, .. } => format!("{arquivo} {segmento}:{offset:04x}"),
            Alvo::TituloAjuda { arquivo } => format!("{arquivo} título"),
        }
    }
}

impl Referencia {
    /// `Some` se o objeto é uma referência (tem `exe` + `segmento`, `banco` + `tipo` + `id` ou `hlp`);
    /// `Some(Err)` se parece uma, mas está incompleta ou traz chaves desconhecidas.
    pub fn do_json(o: &serde_json::Map<String, serde_json::Value>) -> Option<Result<Referencia, String>> {
        let texto = |k: &str| o.get(k).and_then(serde_json::Value::as_str);
        let eh = (texto("exe").is_some() && o.contains_key("segmento"))
            || (texto("banco").is_some() && o.contains_key("tipo") && o.contains_key("id"))
            || texto("hlp").is_some();
        eh.then(|| Referencia::ler(o))
    }

    fn ler(o: &serde_json::Map<String, serde_json::Value>) -> Result<Referencia, String> {
        let numero = |k: &str| -> Result<u64, String> {
            match o.get(k) {
                Some(serde_json::Value::Number(n)) => n.as_u64().ok_or_else(|| format!("\"{k}\" não é um inteiro positivo")),
                Some(serde_json::Value::String(s)) => {
                    let s = s.trim();
                    let r = match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
                        Some(h) => u64::from_str_radix(h, 16),
                        None => s.parse(),
                    };
                    r.map_err(|_| format!("\"{k}\" inválido: {s}"))
                }
                Some(_) => Err(format!("\"{k}\" deve ser número ou texto em hexadecimal")),
                None => Err(format!("falta \"{k}\"")),
            }
        };
        let texto = |k: &str| -> Result<String, String> {
            o.get(k).and_then(serde_json::Value::as_str).map(str::to_string).ok_or_else(|| format!("falta \"{k}\" (texto)"))
        };
        let u16_de = |k: &str| -> Result<u16, String> { numero(k).and_then(|n| u16::try_from(n).map_err(|_| format!("\"{k}\" grande demais"))) };
        let (alvo, chaves): (Alvo, &[&str]) = if o.contains_key("exe") {
            let pecas = match o.get("pecas_8x8") {
                None => None,
                Some(v) => Some(
                    v.as_array()
                        .and_then(|l| {
                            l.iter()
                                .map(|n| match n {
                                    serde_json::Value::Null => Some(None),
                                    n => n.as_str().map(|s| Some(s.to_string())),
                                })
                                .collect::<Option<Vec<_>>>()
                        })
                        .ok_or("\"pecas_8x8\" deve ser uma lista de nomes (ou null)")?,
                ),
            };
            let tamanho = if o.contains_key("tamanho") { Some(numero("tamanho")? as usize) } else { None };
            (
                Alvo::Executavel { arquivo: texto("exe")?, segmento: u16_de("segmento")?, offset: numero("offset")? as usize, tamanho, pecas },
                &["exe", "segmento", "offset", "tamanho", "pecas_8x8", "formato"],
            )
        } else if o.contains_key("hlp") {
            if texto("campo")? != "titulo" {
                return Err("\"campo\" de uma referência \"hlp\" só pode ser \"titulo\"".into());
            }
            (Alvo::TituloAjuda { arquivo: texto("hlp")? }, &["hlp", "campo", "formato"])
        } else {
            let (banco, id) = (texto("banco")?, u16_de("id")?);
            match numero("tipo")? {
                4 => (Alvo::Lista { banco, id, indice: numero("indice")? as usize }, &["banco", "tipo", "id", "indice", "formato"]),
                0 => (Alvo::ObjetoJanela { banco, id, objeto: numero("objeto")? as usize }, &["banco", "tipo", "id", "objeto", "formato"]),
                t => return Err(format!("referência a objeto tipo {t}: só 4 (lista de strings) e 0 (layout de janela)")),
            }
        };
        if let Some(k) = o.keys().find(|k| !k.starts_with('_') && !chaves.contains(&k.as_str())) {
            return Err(format!("chave desconhecida \"{k}\" numa referência de texto"));
        }
        let formato = match o.get("formato") {
            None => None,
            Some(f) => Some(f.as_str().filter(|f| f.contains("{}")).ok_or("\"formato\" deve ser um texto com {}")?.to_string()),
        };
        Ok(Referencia { alvo, formato })
    }

    fn formatar(&self, texto: &str) -> String {
        match &self.formato {
            Some(f) => f.replacen("{}", texto, 1),
            None => texto.to_string(),
        }
    }
}

/// Percorre `v` e troca cada referência por `resolver(referência, caminho)`.
fn percorrer_referencias(
    v: &mut serde_json::Value,
    caminho: &mut String,
    resolver: &mut dyn FnMut(&Referencia, &str) -> Result<serde_json::Value, String>,
) -> Result<(), String> {
    use serde_json::Value;
    match v {
        Value::Object(o) => {
            if let Some(r) = Referencia::do_json(o) {
                let r = r.map_err(|e| format!("{caminho}: {e}"))?;
                *v = resolver(&r, caminho)?;
                return Ok(());
            }
            for (k, filho) in o.iter_mut() {
                let n = caminho.len();
                caminho.push('.');
                caminho.push_str(k);
                percorrer_referencias(filho, caminho, resolver)?;
                caminho.truncate(n);
            }
        }
        Value::Array(l) => {
            for (i, filho) in l.iter_mut().enumerate() {
                let n = caminho.len();
                caminho.push_str(&format!("[{i}]"));
                percorrer_referencias(filho, caminho, resolver)?;
                caminho.truncate(n);
            }
        }
        _ => {}
    }
    Ok(())
}

/// Para testes sem os arquivos do jogo: troca cada referência de texto por `‹descrição›` (as peças
/// de bitmap por peças transparentes), para que os `data/*.json` do port ainda virem as estruturas.
/// O texto não é o do original: testes que dependem dele precisam da instalação (`SIMANT_DIR`).
pub fn marcar_referencias(json: &str) -> String {
    let mut v: serde_json::Value = serde_json::from_str(json).expect("JSON inválido");
    percorrer_referencias(&mut v, &mut String::new(), &mut |r, _| {
        Ok(match &r.alvo {
            Alvo::Executavel { pecas: Some(nomes), .. } => {
                serde_json::Value::Object(nomes.iter().flatten().map(|n| (n.clone(), serde_json::json!(vec!["........"; 8]))).collect())
            }
            a => serde_json::Value::String(r.formatar(&format!("‹{}›", a.descrever()))),
        })
    })
    .expect("referência de texto inválida");
    v.to_string()
}

/// Bytes do original (Windows-1252 no jogo; aqui, como nas listas de strings, Latin-1) → texto.
fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

/// Peças de bitmap 8 × 8 do executável, 16 bytes cada, na ordem de `nomes`: por linha, 2 bytes, o
/// primeiro com os pixels de fundo e o segundo com os pixels pintados (fundo ou tinta), bit 7 = coluna
/// 0. Cada peça vira 8 linhas de texto: `.` transparente, `#` tinta, `o` fundo (o formato das
/// `pecas` de `data/quintal_mapa.json` → `balao_menino`; DGROUP `1048:1c6e`, `_MakeBalloon`).
fn pecas_8x8(d: &[u8], nomes: &[Option<String>]) -> Result<serde_json::Value, String> {
    let mut saida = serde_json::Map::new();
    for (i, nome) in nomes.iter().enumerate() {
        let Some(nome) = nome else { continue };
        let p = d.get(i * 16..i * 16 + 16).ok_or("peças passam do fim do segmento")?;
        let linhas: Vec<serde_json::Value> = p
            .chunks(2)
            .map(|l| {
                let s: String = (0..8)
                    .map(|x| {
                        let bit = 0x80 >> x;
                        match (l[1] & bit != 0, l[0] & bit != 0) {
                            (false, _) => '.',
                            (true, true) => 'o',
                            (true, false) => '#',
                        }
                    })
                    .collect();
                serde_json::Value::String(s)
            })
            .collect();
        saida.insert(nome.clone(), serde_json::Value::Array(linhas));
    }
    Ok(serde_json::Value::Object(saida))
}

/// Caminho `base/<nome>` com cada parte de `nome` (separada por `/`) achada sem diferenciar
/// maiúsculas; `None` se alguma parte não existe ou se o resultado não é um arquivo.
fn achar_sem_caixa(base: &Path, nome: &str) -> Option<PathBuf> {
    let mut atual = base.to_path_buf();
    for parte in nome.split('/').filter(|p| !p.is_empty()) {
        // `..` sobe uma pasta (ex.: a fonte do sistema do Windows, `../../WINDOWS/SYSTEM/VGASYS.FON`).
        if parte == ".." {
            atual.push("..");
            continue;
        }
        let alvo = parte.to_lowercase();
        let entrada = std::fs::read_dir(&atual)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().to_lowercase() == alvo)?;
        atual = entrada.path();
    }
    atual.is_file().then_some(atual)
}

/// Lê todos os subdiretórios com `mod.json` e ordena por dependências (e por nome, para ser determinístico).
pub fn carregar_mods(dir: &Path) -> Result<Vec<Mod>, String> {
    let mut encontrados: HashMap<String, Mod> = HashMap::new();
    let mut leitura = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("mod.json").is_file())
        .collect::<Vec<_>>();
    leitura.sort();
    for p in leitura {
        let texto = std::fs::read_to_string(p.join("mod.json")).map_err(|e| e.to_string())?;
        let manifesto: Manifesto =
            serde_json::from_str(&texto).map_err(|e| format!("{}: {e}", p.join("mod.json").display()))?;
        if encontrados.contains_key(&manifesto.nome) {
            return Err(format!("mod duplicado: {}", manifesto.nome));
        }
        encontrados.insert(manifesto.nome.clone(), Mod { manifesto, dir: p });
    }
    for m in encontrados.values() {
        for d in &m.manifesto.dependencias {
            if !encontrados.contains_key(d) {
                return Err(format!("mod {} depende de {d}, que não está instalado", m.manifesto.nome));
            }
        }
    }
    // Ordenação topológica simples: a cada passo, o primeiro (por nome) cujas dependências já entraram.
    let mut nomes: Vec<String> = encontrados.keys().cloned().collect();
    nomes.sort();
    let mut ordem: Vec<Mod> = Vec::new();
    let mut feitos: HashSet<String> = HashSet::new();
    while !nomes.is_empty() {
        let pos = nomes
            .iter()
            .position(|n| encontrados[n].manifesto.dependencias.iter().all(|d| feitos.contains(d)))
            .ok_or_else(|| format!("dependência circular entre mods: {nomes:?}"))?;
        let n = nomes.remove(pos);
        feitos.insert(n.clone());
        ordem.push(encontrados.remove(&n).unwrap());
    }
    Ok(ordem)
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Pasta temporária com: `jogo/SIMANTW.EXE` (NE mínimo), `data/textos.json` e um mod com a lista
    /// de strings 4/1000 e o layout de janela 0/19 em `objetos/` (sem bancos de verdade).
    fn montar(nome: &str, dados: &str, mod_data: Option<&str>) -> (PathBuf, Recursos) {
        let base = std::env::temp_dir().join(format!("simant_recursos_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (jogo, data, m) = (base.join("jogo"), base.join("data"), base.join("mods/m"));
        for d in [&jogo, &data, &m.join("objetos/4"), &m.join("objetos/0"), &m.join("data")] {
            std::fs::create_dir_all(d).unwrap();
        }
        // Segmento 1: "\0Game\0" e duas peças 8 × 8 (na segunda, a linha 0 é toda fundo e a 1 tem tinta nas pontas).
        let mut seg = b"\0Game\0".to_vec();
        seg.extend_from_slice(&[0u8; 16]);
        seg.extend_from_slice(&[0xFF, 0xFF, 0x00, 0x81, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        std::fs::write(jogo.join("SIMANTW.EXE"), crate::formats::ne::exe_de_teste(&seg)).unwrap();
        // Lista de strings: contagem 2, "Yard", "Caf\xe9" (Latin-1).
        std::fs::write(m.join("objetos/4/1000.bin"), [0, 2, 4, b'Y', b'a', b'r', b'd', 4, b'C', b'a', b'f', 0xE9]).unwrap();
        // Layout: 2 objetos; o 0 (janela, contorno) e o 1 (título, tipo 18, texto em 0x2E).
        let mut l = vec![0u8; 0x2C + 8];
        l[0x0C] = 2;
        let mut o0 = vec![0u8; 0x29];
        o0[0x21] = 15;
        o0[0x22] = 0x29;
        let mut o1 = vec![0u8; 0x2E];
        o1.extend_from_slice(b"Caste\0");
        o1[0x21] = 18;
        o1[0x22] = o1.len() as u8;
        l.extend(o0);
        l.extend(o1);
        std::fs::write(m.join("objetos/0/19.bin"), l).unwrap();
        std::fs::write(m.join("mod.json"), r#"{"nome": "m", "versao": "1"}"#).unwrap();
        std::fs::write(data.join("textos.json"), dados).unwrap();
        if let Some(t) = mod_data {
            std::fs::write(m.join("data/textos.json"), t).unwrap();
        }
        let r = Recursos::abrir(&jogo, &[], &data, Some(&base.join("mods"))).unwrap();
        (base, r)
    }

    const DADOS: &str = r#"{
        "lista": {"banco": "SHARED", "tipo": 4, "id": 1000, "indice": 1, "_simbolo": "comentário"},
        "exe": {"exe": "SIMANTW.EXE", "segmento": 1, "offset": "0x1", "formato": "[{}]"},
        "fixo": {"exe": "SIMANTW.EXE", "segmento": 1, "offset": 1, "tamanho": 2},
        "janela": {"banco": "WINGANT", "tipo": 0, "id": 19, "objeto": 1},
        "pecas": {"exe": "SIMANTW.EXE", "segmento": 1, "offset": "0x6", "pecas_8x8": [null, "b"]},
        "literal": "texto do port",
        "som": {"banco": "opl_banco.json", "ganho": 1}
    }"#;

    #[test]
    fn referencias_viram_os_textos_do_original() {
        let (base, r) = montar("ref", DADOS, None);
        let v: serde_json::Value = r.dados("textos.json").unwrap();
        assert_eq!(v["lista"], "Café");
        assert_eq!(v["exe"], "[Game]");
        assert_eq!(v["fixo"], "Ga");
        assert_eq!(v["janela"], "Caste");
        assert_eq!(v["pecas"], serde_json::json!({"b": ["oooooooo", "#......#", "........", "........", "........", "........", "........", "........"]}));
        assert_eq!(v["literal"], "texto do port");
        assert_eq!(v["som"], serde_json::json!({"banco": "opl_banco.json", "ganho": 1}), "objeto comum não é referência");
        let _ = std::fs::remove_dir_all(base);
    }

    /// Um mod troca a referência por um texto (como os mods até a 0.34) ou só muda o índice.
    #[test]
    fn mod_sobrescreve_com_texto_ou_com_outra_referencia() {
        let (base, r) = montar("mod", DADOS, Some(r#"{"lista": {"indice": 0}, "exe": "Jogo", "janela": "Castas"}"#));
        let v: serde_json::Value = r.dados("textos.json").unwrap();
        assert_eq!(v["lista"], "Yard");
        assert_eq!(v["exe"], "Jogo");
        assert_eq!(v["janela"], "Castas");
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn referencia_quebrada_explica_o_caminho() {
        let (base, r) = montar("erro", r#"{"a": [{"banco": "SHARED", "tipo": 4, "id": 1000, "indice": 9}]}"#, None);
        let e = r.dados::<serde_json::Value>("textos.json").unwrap_err();
        assert!(e.contains(".a[0]") && e.contains("SHARED 4/1000[9]") && e.contains("a lista tem 2 strings"), "{e}");
        let (base2, r) = montar("erro2", r#"{"a": {"exe": "SIMANTW.EXE", "segmento": 1, "offset": 0, "tamnho": 3}}"#, None);
        let e = r.dados::<serde_json::Value>("textos.json").unwrap_err();
        assert!(e.contains("chave desconhecida \"tamnho\""), "{e}");
        let _ = std::fs::remove_dir_all(base);
        let _ = std::fs::remove_dir_all(base2);
    }

    #[test]
    fn marcar_referencias_sem_o_jogo() {
        let v: serde_json::Value = serde_json::from_str(&marcar_referencias(DADOS)).unwrap();
        assert_eq!(v["lista"], "‹SHARED 4/1000[1]›");
        assert_eq!(v["exe"], "[‹SIMANTW.EXE 1:0001›]");
        assert_eq!(v["pecas"]["b"][0], "........");
        assert_eq!(v["literal"], "texto do port");
    }
}
