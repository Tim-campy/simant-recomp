//! Configuração do launcher (`launcher.json` na pasta de dados do usuário) e a linha de comando
//! equivalente. Sem SDL: a tela fica em `launcher/`. Decisão em docs/decisoes.md (2026-10-19,
//! "Launcher dentro do simant").
//!
//! A configuração é só uma forma guardada de opções `--`: [`ConfigLauncher::argumentos`] gera a
//! linha de comando e [`ConfigLauncher::de_argumentos`] faz o caminho inverso (o campo "linha de
//! comando" do launcher é editável). O `simant` lê o arquivo ao iniciar e põe essas opções **antes**
//! das da linha de comando, que vencem ([`combinar`]).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const ARQUIVO: &str = "launcher.json";

/// Como o jogo começa.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Inicio {
    /// Abertura e "Select a Game" (sem opção).
    #[default]
    Abertura,
    /// `--novo-jogo --tipo T`.
    NovoJogo,
    /// `--save ARQ.ANT`.
    Save,
}

/// Tipos de jogo novo (`--tipo`), na ordem da tela, com o nome mostrado.
pub const TIPOS: [(&str, &str); 4] = [("rapido", "Rápido"), ("completo", "Completo"), ("experimental", "Experimental"), ("tutorial", "Tutorial")];
/// Sintetizadores das músicas (`--sintetizador`); "Padrão" no launcher = o de `data/som.json` (o simples).
pub const SINTETIZADORES: [(&str, &str); 3] = [("simples", "Simples"), ("fm", "FM"), ("soundfont", "SoundFont")];
/// Resoluções da tela virtual (`--resolucao`).
pub const RESOLUCOES: [[u32; 2]; 3] = [[640, 480], [800, 600], [1024, 768]];

/// `launcher.json`. Toda chave é opcional; `null`/ausente = o padrão do port (o que vale sem a
/// opção na linha de comando).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigLauncher {
    /// Pasta da instalação do SimAnt (`--jogo`).
    pub jogo: Option<String>,
    pub inicio: Inicio,
    /// `--tipo` (com `inicio` = `novo_jogo`): `rapido`, `completo`, `experimental` ou `tutorial`.
    pub tipo: String,
    /// `--save` (com `inicio` = `save`).
    pub save: Option<String>,
    /// `--resolucao` `[largura, altura]`.
    pub resolucao: Option<[u32; 2]>,
    /// `--escala` (0 = a maior que cabe).
    pub escala: Option<u32>,
    /// `--janela` `[largura, altura]`.
    pub janela: Option<[u32; 2]>,
    /// `--moldura` (`true`) ou `--sem-moldura` (`false`).
    pub moldura: Option<bool>,
    /// `--perfil-driver` (chave de `data/resolucao.json` → `perfis_driver`).
    pub perfil_driver: Option<String>,
    /// `--velocidades` (`proporcional` ou `original`; ausente = `data/partida.json`, proporcional).
    pub velocidades: Option<String>,
    /// `--sem-foco` (`lenta` ou `original`; ausente = `data/partida.json`, lenta).
    pub sem_foco: Option<String>,
    /// `false` = `--sem-som`.
    pub som: bool,
    /// `--sintetizador`.
    pub sintetizador: Option<String>,
    /// `--soundfont`.
    pub soundfont: Option<String>,
    /// `--mods` (ausente = `mods/` da pasta do port).
    pub mods: Option<String>,
    /// `--desligar-mods`: nomes (do `mod.json`) dos mods que não carregam.
    pub mods_desligados: Vec<String>,
    /// `false` = `--sem-preferencias`.
    pub preferencias: bool,
    /// `--win-ini`.
    pub win_ini: Option<String>,
    /// Outras opções, como digitadas no campo da linha de comando (`--semente 5`...).
    pub extras: Vec<String>,
    /// O `simant` sem opções abre o launcher (senão, vai direto ao jogo com esta configuração).
    pub mostrar_launcher: bool,
}

impl Default for ConfigLauncher {
    fn default() -> Self {
        ConfigLauncher {
            jogo: None,
            inicio: Inicio::Abertura,
            tipo: "rapido".into(),
            save: None,
            resolucao: None,
            escala: None,
            janela: None,
            moldura: None,
            perfil_driver: None,
            velocidades: None,
            sem_foco: None,
            som: true,
            sintetizador: None,
            soundfont: None,
            mods: None,
            mods_desligados: Vec::new(),
            preferencias: true,
            win_ini: None,
            extras: Vec::new(),
            mostrar_launcher: true,
        }
    }
}

/// Opções que escolhem o que rodar (vindas da configuração só quando a linha de comando não traz
/// nenhuma opção).
const OPCOES_DE_INICIO: [&str; 3] = ["--novo-jogo", "--tipo", "--save"];

/// Opções de execução sem interação (testes, exportações): com uma delas, a configuração só dá a
/// pasta do jogo (as demais escolhas mudariam o resultado das comparações).
const OPCOES_DE_AUTOMACAO: [&str; 8] = [
    "--ticks",
    "--avancar-ms",
    "--exportar-telas",
    "--exportar-tela-jogo",
    "--exportar-som",
    "--capturar-janela",
    "--sair-apos-ms",
    "--salvar",
];

fn tamanho(t: [u32; 2]) -> String {
    format!("{}x{}", t[0], t[1])
}

impl ConfigLauncher {
    /// `launcher.json` na pasta de dados do usuário.
    pub fn caminho_padrao() -> Option<PathBuf> {
        crate::pastas::usuario().map(|p| p.join(ARQUIVO))
    }

    /// Lê o arquivo: ausente = `Ok(None)`; inválido = erro (com o caminho).
    pub fn ler(caminho: &Path) -> Result<Option<ConfigLauncher>, String> {
        match std::fs::read_to_string(caminho) {
            Ok(t) => serde_json::from_str(&t).map(Some).map_err(|e| format!("{}: {e}", caminho.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("{}: {e}", caminho.display())),
        }
    }

    pub fn gravar(&self, caminho: &Path) -> Result<(), String> {
        if let Some(d) = caminho.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        let texto = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(caminho, texto + "\n").map_err(|e| format!("{}: {e}", caminho.display()))
    }

    /// Opções de ambiente: pasta do jogo, vídeo, som, mods e preferências.
    pub fn argumentos_ambiente(&self) -> Vec<String> {
        let mut a: Vec<String> = Vec::new();
        let mut par = |o: &str, v: String| {
            a.push(o.into());
            a.push(v);
        };
        if let Some(j) = &self.jogo {
            par("--jogo", j.clone());
        }
        if let Some(r) = self.resolucao {
            par("--resolucao", tamanho(r));
        }
        if let Some(e) = self.escala {
            par("--escala", e.to_string());
        }
        if let Some(j) = self.janela {
            par("--janela", tamanho(j));
        }
        if let Some(p) = &self.perfil_driver {
            par("--perfil-driver", p.clone());
        }
        if let Some(v) = &self.velocidades {
            par("--velocidades", v.clone());
        }
        if let Some(s) = &self.sem_foco {
            par("--sem-foco", s.clone());
        }
        if let Some(s) = &self.sintetizador {
            par("--sintetizador", s.clone());
        }
        if let Some(s) = &self.soundfont {
            par("--soundfont", s.clone());
        }
        if let Some(m) = &self.mods {
            par("--mods", m.clone());
        }
        if !self.mods_desligados.is_empty() {
            par("--desligar-mods", self.mods_desligados.join(","));
        }
        if let Some(w) = &self.win_ini {
            par("--win-ini", w.clone());
        }
        match self.moldura {
            Some(true) => a.push("--moldura".into()),
            Some(false) => a.push("--sem-moldura".into()),
            None => {}
        }
        if !self.som {
            a.push("--sem-som".into());
        }
        if !self.preferencias {
            a.push("--sem-preferencias".into());
        }
        a
    }

    /// Opções de início (`--novo-jogo --tipo T` ou `--save ARQ`; nada para a abertura).
    pub fn argumentos_inicio(&self) -> Vec<String> {
        match self.inicio {
            Inicio::Abertura => Vec::new(),
            Inicio::NovoJogo => vec!["--novo-jogo".into(), "--tipo".into(), self.tipo.clone()],
            Inicio::Save => match &self.save {
                Some(s) => vec!["--save".into(), s.clone()],
                None => Vec::new(),
            },
        }
    }

    /// A linha de comando completa (sem o nome do programa): ambiente, início e extras.
    pub fn argumentos(&self) -> Vec<String> {
        let mut a = self.argumentos_ambiente();
        a.extend(self.argumentos_inicio());
        a.extend(self.extras.iter().cloned());
        a
    }

    /// O inverso de [`ConfigLauncher::argumentos`]: a configuração que gera `args`. As escolhas
    /// que não estão na linha (como `mostrar_launcher`) vêm de `self`; opções desconhecidas vão,
    /// na ordem, para `extras`.
    pub fn de_argumentos(&self, args: &[String]) -> Result<ConfigLauncher, String> {
        let mut c = ConfigLauncher { mostrar_launcher: self.mostrar_launcher, tipo: self.tipo.clone(), ..ConfigLauncher::default() };
        let mut novo_jogo = false;
        let mut it = args.iter();
        while let Some(a) = it.next() {
            let mut valor = || it.next().cloned().ok_or(format!("falta o valor de {a}"));
            match a.as_str() {
                "--jogo" => c.jogo = Some(valor()?),
                "--resolucao" => {
                    let (l, h) = crate::video::ler_tamanho(a, &valor()?)?;
                    c.resolucao = Some([l, h]);
                }
                "--escala" => c.escala = Some(valor()?.parse().map_err(|_| format!("{a}: inteiro (0 = a maior que cabe)"))?),
                "--janela" => {
                    let (l, h) = crate::video::ler_tamanho(a, &valor()?)?;
                    c.janela = Some([l, h]);
                }
                "--perfil-driver" => c.perfil_driver = Some(valor()?),
                "--velocidades" => c.velocidades = Some(valor()?),
                "--sem-foco" => c.sem_foco = Some(valor()?),
                "--sintetizador" => c.sintetizador = Some(valor()?),
                "--soundfont" => c.soundfont = Some(valor()?),
                "--mods" => c.mods = Some(valor()?),
                "--desligar-mods" => c.mods_desligados = lista(&valor()?),
                "--win-ini" => c.win_ini = Some(valor()?),
                "--moldura" => c.moldura = Some(true),
                "--sem-moldura" => c.moldura = Some(false),
                "--sem-som" => c.som = false,
                "--com-som" => c.som = true,
                "--sem-preferencias" => c.preferencias = false,
                "--novo-jogo" => novo_jogo = true,
                "--tipo" => {
                    let t = valor()?;
                    if !TIPOS.iter().any(|(n, _)| *n == t) {
                        return Err(format!("--tipo: use tutorial, rapido, completo ou experimental (não '{t}')"));
                    }
                    c.tipo = t;
                    novo_jogo = true;
                }
                "--save" => {
                    c.save = Some(valor()?);
                    c.inicio = Inicio::Save;
                }
                _ => c.extras.push(a.clone()),
            }
        }
        if novo_jogo {
            if c.inicio == Inicio::Save {
                return Err("use --save ou --novo-jogo, não os dois".into());
            }
            c.inicio = Inicio::NovoJogo;
        }
        Ok(c)
    }
}

/// `a,b, c` → `["a", "b", "c"]`.
pub fn lista(texto: &str) -> Vec<String> {
    texto.split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect()
}

/// Opções efetivas do `simant`: as da configuração antes das da linha de comando (as últimas
/// vencem). Sem nenhuma opção na linha, a configuração inteira; com opções de automação (`--ticks`,
/// exportações...), só a pasta do jogo; com outras, a configuração menos o início e os extras.
pub fn combinar(cfg: Option<&ConfigLauncher>, cli: &[String]) -> Vec<String> {
    let Some(cfg) = cfg else { return cli.to_vec() };
    let mut a = if cli.is_empty() {
        cfg.argumentos()
    } else if cli.iter().any(|o| OPCOES_DE_AUTOMACAO.contains(&o.as_str())) {
        cfg.jogo.iter().flat_map(|j| ["--jogo".to_string(), j.clone()]).collect()
    } else {
        cfg.argumentos_ambiente()
    };
    debug_assert!(cli.is_empty() || !a.iter().any(|o| OPCOES_DE_INICIO.contains(&o.as_str())));
    a.extend(cli.iter().cloned());
    a
}

/// Linha de comando para mostrar e editar: aspas em volta do que tem espaço (ou é vazio).
pub fn formatar_linha(args: &[String]) -> String {
    args.iter()
        .map(|a| if a.is_empty() || a.contains(char::is_whitespace) { format!("\"{a}\"") } else { a.clone() })
        .collect::<Vec<_>>()
        .join(" ")
}

/// O inverso de [`formatar_linha`]: separa por espaços, com `"..."` agrupando (sem escapes: a
/// barra invertida dos caminhos do Windows fica como está).
pub fn dividir_linha(linha: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut atual = String::new();
    let mut tem = false;
    let mut aspas = false;
    for c in linha.chars() {
        match c {
            '"' => {
                aspas = !aspas;
                tem = true;
            }
            c if c.is_whitespace() && !aspas => {
                if tem {
                    args.push(std::mem::take(&mut atual));
                    tem = false;
                }
            }
            c => {
                atual.push(c);
                tem = true;
            }
        }
    }
    if aspas {
        return Err("aspas sem fechar na linha de comando".into());
    }
    if tem {
        args.push(atual);
    }
    Ok(args)
}

/// Arquivos da instalação do SimAnt que o port lê: obrigatórios e opcionais (LEIAME §1).
pub const OBRIGATORIOS: [&str; 4] = ["SHARED.NDX", "SHARED.DAT", "WINGANT.NDX", "WINGANT.DAT"];
pub const OPCIONAIS: [(&str, &str); 5] =
    [("SOUND.NDX", "sons"), ("SOUND.DAT", "sons"), ("SIMANTW.EXE", "cursores"), ("SIMANT.HLP", "ajuda"), ("FONTRES.FON", "fontes")];

/// Resultado da conferência da pasta do jogo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Validacao {
    /// Tem todos os obrigatórios (dá para jogar).
    pub ok: bool,
    /// Uma linha para mostrar ao usuário.
    pub mensagem: String,
}

/// Confere se `pasta` tem os arquivos do SimAnt (sem diferença de maiúsculas, como o port os lê).
pub fn validar_pasta_jogo(pasta: &Path) -> Validacao {
    if !pasta.is_dir() {
        return Validacao { ok: false, mensagem: "Pasta não encontrada.".into() };
    }
    let nomes: Vec<String> = std::fs::read_dir(pasta)
        .map(|l| l.filter_map(Result::ok).map(|e| e.file_name().to_string_lossy().to_uppercase()).collect())
        .unwrap_or_default();
    let tem = |n: &str| nomes.iter().any(|x| x == n);
    let faltam: Vec<&str> = OBRIGATORIOS.iter().copied().filter(|n| !tem(n)).collect();
    if !faltam.is_empty() {
        let mensagem = if faltam.len() == OBRIGATORIOS.len() {
            "Não é a pasta do SimAnt: faltam SHARED.NDX/.DAT e WINGANT.NDX/.DAT.".to_string()
        } else {
            format!("Faltam arquivos obrigatórios: {}.", faltam.join(", "))
        };
        return Validacao { ok: false, mensagem };
    }
    let mut sem: Vec<&str> = OPCIONAIS.iter().filter(|(n, _)| !tem(n)).map(|(_, para)| *para).collect();
    sem.dedup();
    let mensagem = if sem.is_empty() {
        "OK: arquivos do SimAnt encontrados.".to_string()
    } else {
        format!("OK, mas sem: {} (o jogo roda sem eles).", sem.join(", "))
    };
    Validacao { ok: true, mensagem }
}

/// Um mod encontrado numa pasta de mods (para a lista do launcher).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfoMod {
    pub nome: String,
    pub versao: String,
}

/// Mods de `pasta` (subpastas com `mod.json` legível), em ordem de nome.
pub fn listar_mods(pasta: &Path) -> Vec<InfoMod> {
    let Ok(lista) = std::fs::read_dir(pasta) else { return Vec::new() };
    let mut mods: Vec<InfoMod> = lista
        .filter_map(Result::ok)
        .filter_map(|e| std::fs::read_to_string(e.path().join("mod.json")).ok())
        .filter_map(|t| serde_json::from_str::<engine::recursos::Manifesto>(&t).ok())
        .map(|m| InfoMod { nome: m.nome, versao: m.versao })
        .collect();
    mods.sort_by(|a, b| a.nome.cmp(&b.nome));
    mods
}

#[cfg(test)]
mod testes {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn completa() -> ConfigLauncher {
        ConfigLauncher {
            jogo: Some(r"C:\Jogos\MAXIS\ANTWIN".into()),
            inicio: Inicio::NovoJogo,
            tipo: "completo".into(),
            save: None,
            resolucao: Some([800, 600]),
            escala: Some(2),
            janela: Some([1300, 1000]),
            moldura: Some(true),
            perfil_driver: Some("svga256".into()),
            velocidades: Some("original".into()),
            sem_foco: Some("original".into()),
            som: false,
            sintetizador: Some("soundfont".into()),
            soundfont: Some("/home/eu/GM.sf2".into()),
            mods: Some("exemplos/mods".into()),
            mods_desligados: s(&["trapacas"]),
            preferencias: false,
            win_ini: Some(r"C:\WINDOWS\WIN.INI".into()),
            extras: s(&["--semente", "42"]),
            mostrar_launcher: false,
        }
    }

    #[test]
    fn padrao_nao_gera_opcao_nenhuma() {
        assert!(ConfigLauncher::default().argumentos().is_empty());
        // Um arquivo vazio ou parcial fica com os padrões no resto.
        let c: ConfigLauncher = serde_json::from_str(r#"{"jogo": "D:\\ANTWIN"}"#).unwrap();
        assert_eq!(c.argumentos(), s(&["--jogo", r"D:\ANTWIN"]));
        assert!(c.som && c.preferencias && c.mostrar_launcher);
    }

    #[test]
    fn linha_de_comando_gerada_da_configuracao() {
        let c = completa();
        assert_eq!(
            c.argumentos(),
            s(&[
                "--jogo", r"C:\Jogos\MAXIS\ANTWIN", "--resolucao", "800x600", "--escala", "2", "--janela", "1300x1000",
                "--perfil-driver", "svga256", "--velocidades", "original", "--sem-foco", "original", "--sintetizador", "soundfont", "--soundfont", "/home/eu/GM.sf2", "--mods",
                "exemplos/mods", "--desligar-mods", "trapacas", "--win-ini", r"C:\WINDOWS\WIN.INI", "--moldura", "--sem-som",
                "--sem-preferencias", "--novo-jogo", "--tipo", "completo", "--semente", "42",
            ])
        );
        let save = ConfigLauncher { inicio: Inicio::Save, save: Some("Meu Jogo.ANT".into()), ..ConfigLauncher::default() };
        assert_eq!(formatar_linha(&save.argumentos()), r#"--save "Meu Jogo.ANT""#);
        // Save escolhido sem arquivo: nada (o launcher avisa antes de jogar).
        assert!(ConfigLauncher { inicio: Inicio::Save, ..ConfigLauncher::default() }.argumentos().is_empty());
        let sem_moldura = ConfigLauncher { moldura: Some(false), ..ConfigLauncher::default() };
        assert_eq!(sem_moldura.argumentos(), s(&["--sem-moldura"]));
    }

    #[test]
    fn linha_editada_volta_para_a_configuracao() {
        let c = completa();
        let base = ConfigLauncher { mostrar_launcher: false, ..ConfigLauncher::default() };
        assert_eq!(base.de_argumentos(&c.argumentos()).unwrap(), c, "ida e volta");
        let linha = formatar_linha(&c.argumentos());
        assert_eq!(dividir_linha(&linha).unwrap(), c.argumentos());
        // --tipo sozinho já é jogo novo; opções desconhecidas ficam nos extras, na ordem.
        let d = ConfigLauncher::default().de_argumentos(&s(&["--tipo", "tutorial", "--trilhas", "--semente", "7"])).unwrap();
        assert_eq!((d.inicio, d.tipo.as_str()), (Inicio::NovoJogo, "tutorial"));
        assert_eq!(d.extras, s(&["--trilhas", "--semente", "7"]));
        assert!(ConfigLauncher::default().de_argumentos(&s(&["--save", "a.ANT", "--novo-jogo"])).is_err());
        assert!(ConfigLauncher::default().de_argumentos(&s(&["--tipo", "facil"])).is_err());
        assert!(ConfigLauncher::default().de_argumentos(&s(&["--escala", "x"])).is_err());
        assert!(ConfigLauncher::default().de_argumentos(&s(&["--jogo"])).is_err());
    }

    #[test]
    fn aspas_da_linha() {
        assert_eq!(dividir_linha(r#"  --jogo "C:\Meus Jogos\ANTWIN"   --sem-som "" "#).unwrap(), s(&["--jogo", r"C:\Meus Jogos\ANTWIN", "--sem-som", ""]));
        assert_eq!(dividir_linha("").unwrap(), Vec::<String>::new());
        assert!(dividir_linha(r#"--jogo "C:\x"#).is_err());
        assert_eq!(formatar_linha(&s(&["--jogo", r"C:\Meus Jogos", ""])), r#"--jogo "C:\Meus Jogos" """#);
    }

    #[test]
    fn linha_de_comando_vence_a_configuracao() {
        let c = completa();
        // Sem opções: a configuração inteira.
        assert_eq!(combinar(Some(&c), &[]), c.argumentos());
        // Com opções: o ambiente da configuração antes; o início e os extras, não.
        let a = combinar(Some(&c), &s(&["--save", "x.ANT", "--escala", "1"]));
        assert!(!a.contains(&"--novo-jogo".to_string()) && !a.contains(&"--semente".to_string()));
        assert_eq!(&a[a.len() - 4..], &s(&["--save", "x.ANT", "--escala", "1"])[..]);
        assert!(a.starts_with(&s(&["--jogo", r"C:\Jogos\MAXIS\ANTWIN"])));
        // Automação: só a pasta do jogo.
        assert_eq!(
            combinar(Some(&c), &s(&["--novo-jogo", "--ticks", "5"])),
            s(&["--jogo", r"C:\Jogos\MAXIS\ANTWIN", "--novo-jogo", "--ticks", "5"])
        );
        assert_eq!(combinar(None, &s(&["--ticks", "5"])), s(&["--ticks", "5"]));
    }

    #[test]
    fn gravar_e_ler_a_configuracao() {
        let dir = std::env::temp_dir().join(format!("simant_launcher_cfg_{}", std::process::id()));
        let arq = dir.join("sub").join(ARQUIVO);
        assert_eq!(ConfigLauncher::ler(&arq), Ok(None), "ausente");
        let c = completa();
        c.gravar(&arq).unwrap();
        assert_eq!(ConfigLauncher::ler(&arq), Ok(Some(c)));
        std::fs::write(&arq, "{ não é json").unwrap();
        assert!(ConfigLauncher::ler(&arq).unwrap_err().contains(ARQUIVO));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validacao_da_pasta_do_jogo() {
        let dir = std::env::temp_dir().join(format!("simant_launcher_jogo_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!validar_pasta_jogo(&dir).ok, "não existe");
        std::fs::create_dir_all(&dir).unwrap();
        let v = validar_pasta_jogo(&dir);
        assert!(!v.ok && v.mensagem.contains("Não é a pasta"), "{v:?}");
        // Nomes em minúsculas (cópias de CD no Linux) valem.
        for n in ["shared.ndx", "SHARED.DAT", "WINGANT.NDX"] {
            std::fs::write(dir.join(n), b"").unwrap();
        }
        let v = validar_pasta_jogo(&dir);
        assert!(!v.ok && v.mensagem.contains("WINGANT.DAT") && !v.mensagem.contains("SHARED"), "{v:?}");
        std::fs::write(dir.join("WinGant.Dat"), b"").unwrap();
        let v = validar_pasta_jogo(&dir);
        assert!(v.ok && v.mensagem.contains("sons") && v.mensagem.contains("ajuda"), "{v:?}");
        for n in ["SOUND.NDX", "SOUND.DAT", "SIMANTW.EXE", "SIMANT.HLP", "FONTRES.FON"] {
            std::fs::write(dir.join(n), b"").unwrap();
        }
        assert_eq!(validar_pasta_jogo(&dir), Validacao { ok: true, mensagem: "OK: arquivos do SimAnt encontrados.".into() });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mods_de_exemplo_sao_listados() {
        let pasta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../exemplos/mods");
        let nomes: Vec<String> = listar_mods(&pasta).into_iter().map(|m| m.nome).collect();
        assert!(nomes.contains(&"trapacas".to_string()) && nomes.contains(&"exemplo-lua".to_string()), "{nomes:?}");
        assert!(listar_mods(Path::new("nao/existe")).is_empty());
    }
}
