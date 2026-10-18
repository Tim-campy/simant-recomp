//! Ajuda por contexto: o modo de ajuda (`_bHelp`, cursor `HELPCURSOR`) e o contexto do WinHelp de
//! cada objeto clicado nele (`_DoEvent` `1000:0bc2`), mais as teclas da ajuda (`_HelpKeyDown`
//! `1000:0de6`). Sem SDL e sem o visualizador (core/ajuda.rs).
//!
//! Especificação: docs/spec/telas/ajuda-contexto.md; dados: data/ajuda_contexto.json.

use std::collections::HashMap;

use serde::Deserialize;

use super::navegacao::Evento as EventoTecla;

pub const ARQUIVO_CONFIG: &str = "ajuda_contexto.json";

/// Faixa de ids [primeiro, último] (inclusive).
pub type Intervalo = [u16; 2];

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigAjudaContexto {
    /// Números das janelas das faixas (34 e 35).
    pub faixa_mapa: u16,
    pub faixa_quintal: u16,
    /// Cursores do executável: o do modo de ajuda e os das ferramentas 0–6 do Experimental.
    pub cursores: ConfigCursores,
    /// Botões "?" das faixas (`_DoWinHelp`): objeto, contexto e se grava `_LessonTemp` = 1.
    pub botoes_ajuda: Vec<BotaoAjuda>,
    /// Botões "?" das janelas: [id do objeto, contexto].
    pub botoes_janela: Vec<[u32; 2]>,
    /// Contexto de cada janela que tem ajuda (número da janela → contexto); as outras não têm (o
    /// clique só desliga o modo de ajuda).
    pub janelas: HashMap<u16, u32>,
    /// Ids de objeto que são o próprio contexto.
    pub objetos_proprios: Vec<Intervalo>,
    /// Ids de objeto cujo contexto é o id somado a um deslocamento: ([primeiro, último], deslocamento).
    pub deslocados: Vec<(Intervalo, i32)>,
    /// Marcadores das duas faixas e o contexto deles.
    pub marcadores: Vec<Intervalo>,
    pub contexto_marcadores: u32,
    /// Botões configuráveis de cada faixa (o 1.º = posição 0): o contexto vem da função do botão.
    pub botoes_usuario: Vec<Intervalo>,
    /// Contexto de cada função de `_DoUserButton` (tabela `DS:8AE6` do segmento 8, lida na bancada).
    pub contexto_funcao: Vec<u32>,
    /// Contexto do índice (F1: `WinHelp(HELP_INDEX)`, o primeiro tópico).
    pub contexto_indice: u32,
    /// Teclas: F1 abre o índice; as outras ligam/desligam ou desligam o modo de ajuda.
    pub tecla_indice: Tecla,
    pub teclas_alternar: Vec<Tecla>,
    pub teclas_sair: Vec<Tecla>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BotaoAjuda {
    pub objeto: String,
    pub contexto: u32,
    #[serde(default)]
    pub licao: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigCursores {
    pub executavel: String,
    pub ajuda: String,
    pub ferramentas: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Tecla {
    pub tecla: String,
    #[serde(default)]
    pub shift: bool,
}

impl Tecla {
    fn bate(&self, ev: &EventoTecla) -> bool {
        matches!(ev, EventoTecla::Tecla { nome, ctrl: false, shift, alt: false } if *nome == self.tecla && *shift == self.shift)
    }
}

/// O que fazer com uma tecla.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeclaAjuda {
    Indice,
    Alternar,
    Sair,
}

fn dentro(i: &Intervalo, id: u16) -> bool {
    (i[0]..=i[1]).contains(&id)
}

impl ConfigAjudaContexto {
    /// Contexto do WinHelp do objeto `id` (byte alto = janela) clicado no modo de ajuda;
    /// `funcoes_botoes`: função de cada botão configurável, na ordem das posições da faixa.
    /// `None`: o objeto não tem ajuda (o original só desliga o modo).
    pub fn contexto(&self, id: u16, funcoes_botoes: &[u16]) -> Option<u32> {
        if self.objetos_proprios.iter().any(|i| dentro(i, id)) {
            return Some(u32::from(id));
        }
        if let Some((_, d)) = self.deslocados.iter().find(|(i, _)| dentro(i, id)) {
            return Some((i32::from(id) + d) as u32);
        }
        if self.marcadores.iter().any(|i| dentro(i, id)) {
            return Some(self.contexto_marcadores);
        }
        if let Some(i) = self.botoes_usuario.iter().find(|i| dentro(i, id)) {
            let f = funcoes_botoes.get(usize::from(id - i[0])).copied()?;
            return self.contexto_funcao.get(usize::from(f)).copied();
        }
        self.janelas.get(&(id >> 8)).copied()
    }

    pub fn tecla(&self, ev: &EventoTecla) -> Option<TeclaAjuda> {
        if self.tecla_indice.bate(ev) {
            Some(TeclaAjuda::Indice)
        } else if self.teclas_alternar.iter().any(|t| t.bate(ev)) {
            Some(TeclaAjuda::Alternar)
        } else if self.teclas_sair.iter().any(|t| t.bate(ev)) {
            Some(TeclaAjuda::Sair)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cfg() -> ConfigAjudaContexto {
        let d = std::fs::read_to_string(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data").join(ARQUIVO_CONFIG)).unwrap();
        serde_json::from_str(&d).unwrap()
    }

    /// Contextos conferidos na bancada (qemu_ui_ajuda.txt, qemu_ui_ajuda_tabelas.txt): o mapa da
    /// edição abre "Edit Window" (4), o botão de estatísticas "Status Window Button" (0x2209), o
    /// primeiro botão configurável (função 15) "Goto Yellow Ant Button" (0xFD47).
    #[test]
    fn contextos_do_original() {
        let c = cfg();
        let funcoes = [15, 0, 1, 3, 7, 6, 5, 10];
        assert_eq!(c.contexto(0x0004, &funcoes), Some(4));
        assert_eq!(c.contexto(0x0007, &funcoes), Some(4));
        assert_eq!(c.contexto(0x2209, &funcoes), Some(0x2209));
        assert_eq!(c.contexto(0x2210, &funcoes), Some(0xFD47));
        assert_eq!(c.contexto(0x2211, &funcoes), Some(0x23F));
        assert_eq!(c.contexto(0x2218, &funcoes), Some(0xFD48));
        assert_eq!(c.contexto(0x220E, &funcoes), Some(0x220B));
        assert_eq!(c.contexto(0x220A, &funcoes), Some(0x2200));
        assert_eq!(c.contexto(0x2227, &funcoes), Some(0x2200));
        assert_eq!(c.contexto(0x2308, &funcoes), Some(0x2308));
        assert_eq!(c.contexto(0x2306, &funcoes), Some(0x2300));
        assert_eq!(c.contexto(0x230B, &funcoes), Some(0xFD47));
        assert_eq!(c.contexto(0x1503, &funcoes), Some(0x1503));
        assert_eq!(c.contexto(0x1502, &funcoes), Some(0x1500));
        assert_eq!(c.contexto(0x1302, &funcoes), Some(0x1300));
        assert_eq!(c.contexto(0x0102, &funcoes), Some(0x102));
        assert_eq!(c.contexto(0x1903, &funcoes), Some(0x102));
        assert_eq!(c.contexto(0x1D00, &funcoes), None);
    }
}
