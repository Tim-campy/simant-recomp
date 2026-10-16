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
        Ok(Recursos { bancos, dir_jogo: dir_jogo.to_path_buf(), dir_dados: dir_dados.to_path_buf(), mods })
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

    /// Arquivo solto da pasta do jogo (ex.: `font1`, `FONTRES.FON`), sem diferenciar maiúsculas.
    /// Um mod pode substituí-lo em `arquivos/<nome>`.
    pub fn arquivo(&self, nome: &str) -> Result<Vec<u8>, String> {
        for m in self.mods.iter().rev() {
            let c = m.dir.join("arquivos").join(nome);
            if c.is_file() {
                return std::fs::read(&c).map_err(|e| format!("{}: {e}", c.display()));
            }
        }
        let alvo = nome.to_lowercase();
        let entradas = std::fs::read_dir(&self.dir_jogo).map_err(|e| format!("{}: {e}", self.dir_jogo.display()))?;
        for e in entradas.flatten() {
            if e.file_name().to_string_lossy().to_lowercase() == alvo {
                return std::fs::read(e.path()).map_err(|err| format!("{}: {err}", e.path().display()));
            }
        }
        Err(format!("arquivo do jogo não encontrado: {nome}"))
    }

    /// Arquivo de dados JSON do port (`data/<nome>`), sobrescrito por inteiro pelo último mod que o tiver.
    pub fn dados<T: DeserializeOwned>(&self, nome: &str) -> Result<T, String> {
        let mut caminho = self.dir_dados.join(nome);
        for m in self.mods.iter().rev() {
            let c = m.dir.join("data").join(nome);
            if c.is_file() {
                caminho = c;
                break;
            }
        }
        let texto = std::fs::read_to_string(&caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
        serde_json::from_str(&texto).map_err(|e| format!("{}: {e}", caminho.display()))
    }
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
