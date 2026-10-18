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

    /// O que gravar ao sair: as opções e os botões do momento (`WriteProfileString` de tudo).
    pub fn do_momento(opcoes: &[i16; 6], funcoes: &[u16]) -> Preferencias {
        let b = |i: usize| Some(opcoes[i] != 0);
        Preferencias {
            auto_track: b(0),
            musica: b(1),
            efeitos: b(2),
            eventos: b(3),
            mensagens: b(4),
            silly: b(5),
            botoes_faixa: Some(funcoes.to_vec()),
        }
    }
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
    fn gravar_e_ler_devolve_o_mesmo() {
        let dir = std::env::temp_dir().join(format!("simant_pref_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let arq = dir.join(ARQUIVO);
        let p = Preferencias::do_momento(&[1, 0, 1, 0, 1, 0], &[15, 0, 1, 3, 7, 6, 5, 11]);
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
