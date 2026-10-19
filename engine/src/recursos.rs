//! Acesso unificado a objetos do banco original e a arquivos de dados do port, com mods por cima.
//! Regras dos mods: docs/api-mods.md.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::formats::banco::{Banco, Chave};

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
    /// Valores mesclados por cima de `data/<nome>` depois dos mods (opções da linha de comando do
    /// executável, como `--perfil-driver`; [`Recursos::sobrepor_dados`]).
    sobreposicoes: Vec<(String, serde_json::Value)>,
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
        Recursos::abrir_sem(dir_jogo, nomes_bancos, dir_dados, dir_mods, &[])
    }

    /// Como [`Recursos::abrir`], sem os mods cujo `nome` (do `mod.json`) está em `desligados` (o
    /// launcher e a opção `--desligar-mods` do executável). Nomes que não existem são ignorados.
    pub fn abrir_sem(
        dir_jogo: &Path,
        nomes_bancos: &[String],
        dir_dados: &Path,
        dir_mods: Option<&Path>,
        desligados: &[String],
    ) -> Result<Recursos, String> {
        let bancos = nomes_bancos
            .iter()
            .map(|n| Banco::abrir(&dir_jogo.join(n)))
            .collect::<Result<Vec<_>, _>>()?;
        let mods = match dir_mods {
            Some(d) if d.is_dir() => carregar_mods_sem(d, desligados)?,
            _ => Vec::new(),
        };
        Ok(Recursos { bancos, dir_jogo: dir_jogo.to_path_buf(), dir_dados: dir_dados.to_path_buf(), mods, sobreposicoes: Vec::new() })
    }

    /// Mescla `valor` por cima de `data/<nome>` (depois dos mods, com as regras de [`mesclar`]) em
    /// toda leitura seguinte. Para opções do executável que trocam uma chave de dados, como
    /// `--perfil-driver` (`resolucao.json` → `perfil_driver`).
    pub fn sobrepor_dados(&mut self, nome: &str, valor: serde_json::Value) {
        self.sobreposicoes.push((nome.to_string(), valor));
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
        for (n, v) in &self.sobreposicoes {
            if n == nome {
                if let Some(atual) = valor.as_mut() {
                    mesclar(atual, v.clone());
                    origem = format!("{origem} (com opções da linha de comando)");
                }
            }
        }
        let valor = valor.ok_or_else(|| format!("{}: arquivo de dados não encontrado", base.display()))?;
        Ok((valor, origem))
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
    carregar_mods_sem(dir, &[])
}

/// Como [`carregar_mods`], deixando de fora os mods cujo nome está em `desligados`. Um mod ligado
/// que depende de um desligado é erro (como uma dependência não instalada).
pub fn carregar_mods_sem(dir: &Path, desligados: &[String]) -> Result<Vec<Mod>, String> {
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
        if desligados.contains(&manifesto.nome) {
            continue;
        }
        encontrados.insert(manifesto.nome.clone(), Mod { manifesto, dir: p });
    }
    for m in encontrados.values() {
        for d in &m.manifesto.dependencias {
            if desligados.contains(d) {
                return Err(format!("mod {} depende de {d}, que está desligado", m.manifesto.nome));
            }
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
