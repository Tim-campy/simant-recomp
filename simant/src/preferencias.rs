//! Preferências do jogador, o equivalente do port à seção `[SimAnt]` do `WIN.INI` do original.
//!
//! O original (`WINMAIN`, `1000:3fda`) lê ao iniciar, com `GetProfileInt` e o valor atual como
//! padrão, as seis opções de `_OptionStates` (`autotrack`, `music`, `effects`, `events`,
//! `messages`, `silly`) e as funções dos botões configuráveis das faixas (`mapuserbuttonN` e
//! `yarduserbuttonN`, N = 0–7), e grava tudo de volta ao sair (`WriteProfileString`). Como
//! `_NewGame` não mexe em `_OptionStates`, num jogo novo valem as opções do momento (as do
//! `WIN.INI` no primeiro); um `.ANT` carregado traz as suas (estão no save), e são essas que vão
//! para o `WIN.INI` ao sair. Especificação: docs/spec/regras/som.md ("Persistência") e
//! docs/decisoes.md ("Preferências do jogador").
//!
//! O port grava `preferencias.json` na pasta de dados do usuário do sistema (`SDL_GetPrefPath`;
//! no Windows, `%APPDATA%\SimAnt recomp\SimAnt\`), fora do repositório e da pasta do jogo. Só a
//! partida com janela lê e grava; as execuções sem janela (exportações, `--ticks`) não.
//!
//! Enquanto esse arquivo não existe (a primeira partida), a origem das preferências é a seção
//! `[SimAnt]` do `WIN.INI` da instalação do usuário, se houver uma (`--win-ini ARQ`, ou
//! `WINDOWS/WIN.INI` subindo da pasta do jogo: [`procurar_win_ini`]), lida como o `GetProfileInt` do
//! original. O port só lê o `WIN.INI`; nunca grava nele (docs/decisoes.md).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const ARQUIVO: &str = "preferencias.json";

/// Conteúdo do arquivo. Chave ausente = fica o valor do momento (o padrão do `GetProfileInt`).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Preferencias {
    /// `_OptionStates[0]`, Options → AutoTrack (`autotrack`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_track: Option<bool>,
    /// `_OptionStates[1]`, Music (`music`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub musica: Option<bool>,
    /// `_OptionStates[2]`, Sound Effects (`effects`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub efeitos: Option<bool>,
    /// `_OptionStates[3]`, Events (`events`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eventos: Option<bool>,
    /// `_OptionStates[4]`, Messages (`messages`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mensagens: Option<bool>,
    /// `_OptionStates[5]`, Silly (`silly`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub silly: Option<bool>,
    /// Função de `_DoUserButton` de cada botão configurável da faixa 34 (`mapuserbutton0`–`7`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub botoes_faixa: Option<Vec<u16>>,
    /// Os da faixa 35 do quintal (`yarduserbutton0`–`7`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub botoes_quintal: Option<Vec<u16>>,
}

impl Preferencias {
    /// Pasta de dados do usuário (`SDL_GetPrefPath`), criada se preciso.
    pub fn caminho_padrao() -> Option<PathBuf> {
        match sdl2::filesystem::pref_path("SimAnt recomp", "SimAnt") {
            Ok(p) => Some(PathBuf::from(p).join(ARQUIVO)),
            Err(e) => {
                eprintln!("preferências: sem pasta de dados do usuário ({e})");
                None
            }
        }
    }

    /// Lê o arquivo; ausente ou inválido = sem preferências (com aviso no segundo caso).
    pub fn ler(caminho: &Path) -> Preferencias {
        match std::fs::read_to_string(caminho) {
            Ok(texto) => serde_json::from_str(&texto).unwrap_or_else(|e| {
                eprintln!("preferências: {} inválido ({e}); usando os padrões", caminho.display());
                Preferencias::default()
            }),
            Err(_) => Preferencias::default(),
        }
    }

    pub fn gravar(&self, caminho: &Path) -> Result<(), String> {
        let texto = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(caminho, texto + "\n").map_err(|e| format!("{}: {e}", caminho.display()))
    }

    fn opcoes(&self) -> [Option<bool>; 6] {
        [self.auto_track, self.musica, self.efeitos, self.eventos, self.mensagens, self.silly]
    }

    /// `GetProfileInt` das seis opções sobre `_OptionStates` (as ausentes ficam como estão).
    pub fn aplicar_opcoes(&self, opcoes: &mut [i16; 6]) {
        for (o, p) in opcoes.iter_mut().zip(self.opcoes()) {
            if let Some(v) = p {
                *o = i16::from(v);
            }
        }
    }

    /// Funções dos botões configuráveis (só as posições que existem nos dois lados).
    pub fn aplicar_botoes(&self, funcoes: &mut [u16]) {
        if let Some(b) = &self.botoes_faixa {
            for (f, &v) in funcoes.iter_mut().zip(b) {
                *f = v;
            }
        }
    }

    /// Como [`Preferencias::aplicar_botoes`], para os botões da faixa 35.
    pub fn aplicar_botoes_quintal(&self, funcoes: &mut [u16]) {
        if let Some(b) = &self.botoes_quintal {
            for (f, &v) in funcoes.iter_mut().zip(b) {
                *f = v;
            }
        }
    }

    /// A seção `[SimAnt]` de um `WIN.INI` como o `WINMAIN` a lê: `GetProfileInt` de cada chave com o
    /// valor do momento como padrão (`opcoes`, `funcoes`, `funcoes_quintal`). Seção e chaves sem
    /// diferença de maiúsculas, como no Windows. Uma opção diferente de 0 vira 1 (o original guarda o
    /// número lido; só muda algo com um valor que não seja 0 nem 1 no `WIN.INI`).
    pub fn do_win_ini(texto: &str, opcoes: &[i16; 6], funcoes: &[u16], funcoes_quintal: &[u16]) -> Preferencias {
        let secao = secao_ini(texto, "SimAnt");
        let inteiro = |chave: &str, padrao: i32| -> i32 {
            secao.iter().find(|(k, _)| k.eq_ignore_ascii_case(chave)).map_or(padrao, |(_, v)| profile_int(v))
        };
        let nomes = ["autotrack", "music", "effects", "events", "messages", "silly"];
        let o: Vec<bool> = nomes.iter().zip(opcoes).map(|(n, &v)| inteiro(n, i32::from(v)) != 0).collect();
        let botoes = |prefixo: &str, atuais: &[u16]| -> Vec<u16> {
            atuais.iter().enumerate().map(|(i, &v)| inteiro(&format!("{prefixo}{i}"), i32::from(v)) as u16).collect()
        };
        Preferencias {
            auto_track: Some(o[0]),
            musica: Some(o[1]),
            efeitos: Some(o[2]),
            eventos: Some(o[3]),
            mensagens: Some(o[4]),
            silly: Some(o[5]),
            botoes_faixa: Some(botoes("mapuserbutton", funcoes)),
            botoes_quintal: (!funcoes_quintal.is_empty()).then(|| botoes("yarduserbutton", funcoes_quintal)),
        }
    }

    /// O que gravar ao sair: as opções e os botões do momento (`WriteProfileString` de tudo).
    pub fn do_momento(opcoes: &[i16; 6], funcoes: &[u16], funcoes_quintal: &[u16]) -> Preferencias {
        let b = |i: usize| Some(opcoes[i] != 0);
        Preferencias {
            auto_track: b(0),
            musica: b(1),
            efeitos: b(2),
            eventos: b(3),
            mensagens: b(4),
            silly: b(5),
            botoes_faixa: Some(funcoes.to_vec()),
            botoes_quintal: (!funcoes_quintal.is_empty()).then(|| funcoes_quintal.to_vec()),
        }
    }
}

/// `[Windows Help] H_WindowPosition` de um `WIN.INI`: a posição em que o WinHelp abre a janela
/// **History** (não a principal), `[x, y, largura, altura, maximizada]` em pixels da tela (bancada
/// SVGA, 2026-10-19: com `[100,50,300,250,0]` a History abriu em (100, 50) com 300 × 250 e a
/// janela principal no lugar padrão). Devolve `[x, y, largura, altura]`; formato inválido = `None`.
pub fn posicao_historico_ajuda(texto: &str) -> Option<[i16; 4]> {
    let secao = secao_ini(texto, "Windows Help");
    let (_, v) = secao.iter().find(|(k, _)| k.eq_ignore_ascii_case("H_WindowPosition"))?;
    let dentro = v.trim().strip_prefix('[')?.strip_suffix(']')?;
    let n: Vec<i16> = dentro.split(',').map(|p| p.trim().parse::<i16>()).collect::<Result<_, _>>().ok()?;
    (n.len() >= 4 && n[2] > 0 && n[3] > 0).then(|| [n[0], n[1], n[2], n[3]])
}

/// Pares (chave, valor) de uma seção de um arquivo `.INI` (a primeira com esse nome, sem diferença de
/// maiúsculas). Linhas sem `=` e comentários (`;`) são ignorados.
fn secao_ini<'a>(texto: &'a str, nome: &str) -> Vec<(&'a str, &'a str)> {
    let mut dentro = false;
    let mut pares = Vec::new();
    for linha in texto.lines().map(str::trim) {
        if let Some(cabecalho) = linha.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if dentro {
                break;
            }
            dentro = cabecalho.trim().eq_ignore_ascii_case(nome);
        } else if dentro && !linha.starts_with(';') {
            if let Some((k, v)) = linha.split_once('=') {
                pares.push((k.trim(), v.trim()));
            }
        }
    }
    pares
}

/// Conversão do `GetProfileInt`: os dígitos decimais do começo; sem dígito, 0.
fn profile_int(valor: &str) -> i32 {
    let digitos: String = valor.trim_start().chars().take_while(char::is_ascii_digit).collect();
    digitos.parse::<i64>().map_or(0, |v| v.min(i64::from(u16::MAX)) as i32)
}

/// Um `WIN.INI` da instalação do usuário: `WINDOWS/WIN.INI` na pasta do jogo ou em uma das duas de
/// cima (`C:\MAXIS\ANTWIN` → `C:\WINDOWS`), sem diferença de maiúsculas.
pub fn procurar_win_ini(pasta_jogo: &Path) -> Option<PathBuf> {
    let mut base = Some(pasta_jogo);
    for _ in 0..3 {
        let b = base?;
        if let Some(arq) = filho_sem_caixa(b, "WINDOWS").and_then(|w| filho_sem_caixa(&w, "WIN.INI")) {
            if arq.is_file() {
                return Some(arq);
            }
        }
        base = b.parent();
    }
    None
}

fn filho_sem_caixa(pasta: &Path, nome: &str) -> Option<PathBuf> {
    std::fs::read_dir(pasta)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(nome)))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn chaves_ausentes_mantem_o_valor_do_momento() {
        let p: Preferencias = serde_json::from_str(r#"{"auto_track": true, "silly": true}"#).unwrap();
        let mut o = [0, 1, 1, 1, 1, 0];
        p.aplicar_opcoes(&mut o);
        assert_eq!(o, [1, 1, 1, 1, 1, 1]);
        let mut f = vec![15, 0, 1, 3, 7, 6, 5, 10];
        p.aplicar_botoes(&mut f);
        assert_eq!(f, vec![15, 0, 1, 3, 7, 6, 5, 10], "sem botões no arquivo, ficam os do port");
    }

    #[test]
    fn posicao_da_history_do_winhelp() {
        let ini = "[windows]\r\nload=\r\n[Windows Help]\r\nH_WindowPosition=[266,200,266,200,0]\r\n[SimAnt]\r\nmusic=1\r\n";
        assert_eq!(posicao_historico_ajuda(ini), Some([266, 200, 266, 200]));
        assert_eq!(posicao_historico_ajuda("[windows help]\nh_windowposition= [ 1, 2, 3, 4, 1 ]"), Some([1, 2, 3, 4]));
        assert_eq!(posicao_historico_ajuda("[Windows Help]\nH_WindowPosition=266,200"), None);
        assert_eq!(posicao_historico_ajuda("[SimAnt]\nmusic=0"), None);
    }

    #[test]
    fn win_ini_como_o_getprofileint() {
        let texto = concat!(
            "[Windows]\r\nload=\r\n\r\n[simant]\r\nAutoTrack=1\r\nmusic=0\r\nsilly=x\r\n; comentário\r\n",
            "mapuserbutton1=12\r\nyarduserbutton7=  4 \r\n[Outra]\r\nevents=0\r\n"
        );
        let p = Preferencias::do_win_ini(texto, &[0, 1, 1, 1, 1, 1], &[15, 0, 1, 3, 7, 6, 5, 10], &[15, 0, 1, 3, 7, 6, 5, 10]);
        let mut o = [0; 6];
        p.aplicar_opcoes(&mut o);
        // Chaves ausentes ficam com o valor do momento; "x" vale 0; a seção [Outra] não conta.
        assert_eq!(o, [1, 0, 1, 1, 1, 0]);
        let mut f = vec![0; 8];
        p.aplicar_botoes(&mut f);
        assert_eq!(f, vec![15, 12, 1, 3, 7, 6, 5, 10]);
        p.aplicar_botoes_quintal(&mut f);
        assert_eq!(f[7], 4);
        // Sem a seção: tudo como estava.
        let nada = Preferencias::do_win_ini("[Windows]\nload=\n", &[0, 1, 1, 1, 1, 0], &[15, 0], &[]);
        let mut o = [9; 6];
        nada.aplicar_opcoes(&mut o);
        assert_eq!(o, [0, 1, 1, 1, 1, 0]);
        assert_eq!(nada.botoes_quintal, None);
    }

    #[test]
    fn win_ini_achado_subindo_da_pasta_do_jogo() {
        let raiz = std::env::temp_dir().join(format!("simant_winini_{}", std::process::id()));
        let jogo = raiz.join("MAXIS").join("ANTWIN");
        std::fs::create_dir_all(&jogo).unwrap();
        assert_eq!(procurar_win_ini(&jogo), None);
        std::fs::create_dir_all(raiz.join("Windows")).unwrap();
        std::fs::write(raiz.join("Windows").join("win.ini"), "[SimAnt]\n").unwrap();
        assert_eq!(procurar_win_ini(&jogo), Some(raiz.join("Windows").join("win.ini")));
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn gravar_e_ler_devolve_o_mesmo() {
        let dir = std::env::temp_dir().join(format!("simant_pref_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let arq = dir.join(ARQUIVO);
        let p = Preferencias::do_momento(&[1, 0, 1, 0, 1, 0], &[15, 0, 1, 3, 7, 6, 5, 11], &[15, 0, 1, 3, 7, 6, 5, 8]);
        p.gravar(&arq).unwrap();
        let lida = Preferencias::ler(&arq);
        assert_eq!(lida, p);
        let mut o = [0; 6];
        lida.aplicar_opcoes(&mut o);
        assert_eq!(o, [1, 0, 1, 0, 1, 0]);
        let mut f = vec![0; 8];
        lida.aplicar_botoes(&mut f);
        assert_eq!(f[7], 11);
        // Ausente ou inválido: sem preferências.
        assert_eq!(Preferencias::ler(&dir.join("nao_existe.json")), Preferencias::default());
        std::fs::write(&arq, "{ não é json").unwrap();
        assert_eq!(Preferencias::ler(&arq), Preferencias::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
