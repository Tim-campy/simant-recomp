//! "Select a Game" (janela 2, `scenarioWin`): qual objeto foi clicado e que jogo começa.
//!
//! Especificação: docs/spec/telas/abertura.md, "Escolha do jogo" (`_DoScenario` `1000:5998`,
//! `_NewGame` `1000:d6dc`). Dados: data/modos.json (sobrescrevível por mod).

use std::collections::HashMap;

use serde::Deserialize;

use super::navegacao::Evento as EventoTecla;
use super::telas::{carregar_janela, Interface};
use crate::formats::janela::Janela;
use crate::recursos::Recursos;
use crate::rules::geracao::TipoJogo;

/// Nome do arquivo de dados.
pub const ARQUIVO_CONFIG: &str = "modos.json";

/// O que o "Select a Game" devolve.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Escolha {
    /// `scenTutorial` (0x206): `_CurGameType = 0`.
    Tutorial,
    /// `scenQuickGame` (0x202): 1.
    Rapido,
    /// `scenFullGame` (0x203, também Enter): 2.
    Completo,
    /// `scenExperimental` (0x204): 3.
    Experimental,
    /// `scenLoad` (0x207): `_LoadGame`.
    Carregar,
    /// `scenCancel` (0x205) ou `_DialogAbort`.
    Cancelar,
}

impl Escolha {
    /// Tipo de jogo novo, se a escolha cria um.
    pub fn tipo_jogo(self) -> Option<TipoJogo> {
        match self {
            Escolha::Tutorial => Some(TipoJogo::Tutorial),
            Escolha::Rapido => Some(TipoJogo::Rapido),
            Escolha::Completo => Some(TipoJogo::Completo),
            Escolha::Experimental => Some(TipoJogo::Experimental),
            Escolha::Carregar | Escolha::Cancelar => None,
        }
    }

    /// Nome para a linha de comando (`--tipo`) e para o título da janela.
    pub fn de_nome(nome: &str) -> Option<Escolha> {
        serde_json::from_value(serde_json::Value::String(nome.to_string())).ok()
    }
}

/// Tecla com modificadores (mesmas regras de `navegacao::Atalho`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Tecla {
    pub tecla: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
}

impl Tecla {
    pub fn bate(&self, e: &EventoTecla) -> bool {
        let EventoTecla::Tecla { nome, ctrl, shift, alt } = e;
        self.tecla.eq_ignore_ascii_case(nome) && self.ctrl == *ctrl && self.shift == *shift && self.alt == *alt
    }
}

/// Conteúdo de data/modos.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigModos {
    /// Nome da tela de data/telas.json onde a escolha acontece.
    pub tela: String,
    /// Janela do "Select a Game".
    pub janela: u16,
    /// Objeto clicado → escolha.
    pub botoes: HashMap<String, Escolha>,
    /// Teclas que escolhem sem clique (`GetAsyncKeyState(VK_RETURN)` → Full Game).
    pub teclas: Vec<(Tecla, Escolha)>,
    /// File → New Game (`Ctrl+N`, `_ProcMenu` caso 3): volta ao "Select a Game" durante a partida.
    pub novo_jogo: Tecla,
}

impl ConfigModos {
    pub fn de_json(texto: &str) -> Result<ConfigModos, String> {
        serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_CONFIG}: {e}"))
    }

    /// Escolha feita por uma tecla, se houver.
    pub fn escolha_da_tecla(&self, e: &EventoTecla) -> Option<Escolha> {
        self.teclas.iter().find(|(t, _)| t.bate(e)).map(|(_, esc)| *esc)
    }
}

/// O "Select a Game" pronto para receber cliques: layout e posição da janela.
pub struct SelecaoDeJogo {
    pub cfg: ConfigModos,
    janela: Janela,
    /// Deslocamento dos retângulos absolutos do layout para a tela.
    desloc: (i32, i32),
}

impl SelecaoDeJogo {
    pub fn carregar(cfg: ConfigModos, recursos: &Recursos, interface: &Interface) -> Result<SelecaoDeJogo, String> {
        let janela = carregar_janela(recursos, cfg.janela)?;
        let g = interface.posicionar(cfg.janela, &janela).ok_or(format!("janela {} sem posição", cfg.janela))?;
        let base = janela.retangulo();
        let desloc = (i32::from(g.origem.0 - base.esq), i32::from(g.origem.1 - base.topo));
        for nome in cfg.botoes.keys() {
            if !janela.objetos.iter().any(|o| &o.nome == nome) {
                return Err(format!("{ARQUIVO_CONFIG}: a janela {} não tem o objeto {nome}", cfg.janela));
            }
        }
        Ok(SelecaoDeJogo { cfg, janela, desloc })
    }

    /// Escolha do objeto sob o ponto (x, y) da área cliente, se for um dos botões. Objetos que não
    /// estão em `botoes` (a figura, o fundo) não fazem nada [hipótese: no original só objetos
    /// selecionáveis geram evento].
    pub fn escolha_em(&self, x: i32, y: i32) -> Option<Escolha> {
        let (dx, dy) = self.desloc;
        self.janela
            .objetos
            .iter()
            .skip(1)
            .filter(|o| {
                let r = o.retangulo;
                x - dx >= i32::from(r.esq) && x - dx < i32::from(r.dir) && y - dy >= i32::from(r.topo) && y - dy < i32::from(r.base)
            })
            .find_map(|o| self.cfg.botoes.get(&o.nome).copied())
    }

    /// Centro do botão da escolha na área cliente (para testes e automação).
    pub fn centro(&self, escolha: Escolha) -> Option<(i32, i32)> {
        let nome = self.cfg.botoes.iter().find(|(_, e)| **e == escolha)?.0;
        let r = self.janela.objetos.iter().find(|o| &o.nome == nome)?.retangulo;
        Some(((i32::from(r.esq) + i32::from(r.dir)) / 2 + self.desloc.0, (i32::from(r.topo) + i32::from(r.base)) / 2 + self.desloc.1))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn dados_carregam() {
        let c = ConfigModos::de_json(include_str!("../../../data/modos.json")).unwrap();
        assert_eq!(c.botoes.get("scenFullGame"), Some(&Escolha::Completo));
        assert_eq!(c.botoes.get("scenTutorial"), Some(&Escolha::Tutorial));
        let enter = EventoTecla::Tecla { nome: "Return".into(), ctrl: false, shift: false, alt: false };
        assert_eq!(c.escolha_da_tecla(&enter), Some(Escolha::Completo));
        let n = EventoTecla::Tecla { nome: "N".into(), ctrl: true, shift: false, alt: false };
        assert!(c.novo_jogo.bate(&n));
    }

    #[test]
    fn escolhas_e_tipos() {
        assert_eq!(Escolha::de_nome("experimental"), Some(Escolha::Experimental));
        assert_eq!(Escolha::de_nome("voar"), None);
        assert_eq!(Escolha::Rapido.tipo_jogo(), Some(TipoJogo::Rapido));
        assert_eq!(Escolha::Tutorial.tipo_jogo().map(|t| t as i16), Some(0));
        assert_eq!(Escolha::Carregar.tipo_jogo(), None);
    }
}
