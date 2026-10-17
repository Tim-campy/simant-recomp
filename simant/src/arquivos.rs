//! File → Load, Save e Save As com os diálogos do Windows (engine/src/core/dialogos.rs):
//! `_LoadGame` pergunta se salva a partida atual (`_gGameNeedsSaving`), abre o diálogo de arquivo e
//! carrega; `_SaveGame` grava e avisa "%s Saved correctly" (`_PopMsg`). Os diálogos são modais: a
//! simulação fica parada enquanto um deles está aberto. Especificação: docs/spec/telas/arquivos.md.

use super::*;
use engine::core::dialogos::{CaixaMensagem, DialogoArquivo, RespostaArquivo};

/// Diálogo aberto agora.
#[derive(Clone, Debug)]
pub(crate) enum Dialogo {
    /// "SimAnt Load: Do you want to save the current ant hill?" (Yes / No / Cancel).
    PerguntaLoad(CaixaMensagem),
    /// Save As pedido pelo Yes da pergunta (`_SaveGame(0)`); depois, o Load.
    SalvarAntesDoLoad(DialogoArquivo),
    /// "Load Ant Hill".
    Load(DialogoArquivo),
    /// "Save Ant Hill As" (Ctrl+A, ou Save sem arquivo).
    SalvarComo(DialogoArquivo),
    /// "%s Saved correctly"; com `true`, o Load vem depois do OK.
    Salvo(CaixaMensagem, bool),
}

/// O que a interface faz depois de uma entrada no diálogo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RespostaDialogo {
    Nada,
    /// Carregar este arquivo (nova sessão).
    Carregar(PathBuf),
}

impl Sessao<'_> {
    /// Pasta dos diálogos: a do arquivo da partida ou a atual.
    fn pasta_dos_dialogos(&self) -> PathBuf {
        self.arquivo
            .as_ref()
            .and_then(|a| a.parent())
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub(crate) fn dialogo_arquivo(&self, salvar: bool) -> DialogoArquivo {
        DialogoArquivo::new(salvar, &self.pasta_dos_dialogos(), &self.dialogos_cfg)
    }

    /// File → Load (`_LoadGame`): com a partida mudada desde o último Save/Load, pergunta antes.
    pub(crate) fn abrir_load(&mut self) {
        self.dialogo = Some(if self.partida.estado.precisa_salvar {
            Dialogo::PerguntaLoad(CaixaMensagem::pergunta_load(&self.dialogos_cfg))
        } else {
            Dialogo::Load(self.dialogo_arquivo(false))
        });
    }

    /// File → Save (`_SaveGame(1)`) e Save As (`_SaveGame(0)`): sem arquivo, ou em Save As, o
    /// diálogo de arquivo; `destino` (da linha de comando) pula o diálogo.
    pub(crate) fn salvar_com_dialogo(&mut self, como: bool, destino: Option<&Path>) {
        if let Some(d) = destino {
            self.gravar_e_avisar(d, false);
        } else if let (Some(a), false) = (self.arquivo.clone(), como) {
            self.gravar_e_avisar(&a, false);
        } else {
            self.dialogo = Some(Dialogo::SalvarComo(self.dialogo_arquivo(true)));
        }
    }

    /// Grava em `caminho` e abre o "Saved correctly"; erro vai para o terminal e devolve false.
    fn gravar_e_avisar(&mut self, caminho: &Path, depois_load: bool) -> bool {
        match self.salvar(true, Some(caminho)) {
            Ok(p) => {
                println!("{} Saved correctly", p.display());
                self.partida.estado.precisa_salvar = false;
                let nome = p.display().to_string().to_uppercase();
                self.dialogo = Some(Dialogo::Salvo(CaixaMensagem::salvo(&self.dialogos_cfg, &nome), depois_load));
                true
            }
            Err(e) => {
                eprintln!("erro ao salvar: {e}");
                false
            }
        }
    }

    /// Botão `i` de uma caixa de mensagem escolhido.
    fn escolher_botao(&mut self, i: usize) -> RespostaDialogo {
        match self.dialogo.take() {
            Some(Dialogo::PerguntaLoad(_)) => match i {
                0 => self.dialogo = Some(Dialogo::SalvarAntesDoLoad(self.dialogo_arquivo(true))),
                1 => self.dialogo = Some(Dialogo::Load(self.dialogo_arquivo(false))),
                _ => {}
            },
            Some(Dialogo::Salvo(_, true)) => self.dialogo = Some(Dialogo::Load(self.dialogo_arquivo(false))),
            outro => self.dialogo = outro.filter(|d| !matches!(d, Dialogo::Salvo(..))),
        }
        RespostaDialogo::Nada
    }

    /// Resposta de um diálogo de arquivo.
    fn responder_arquivo(&mut self, r: RespostaArquivo) -> RespostaDialogo {
        let Some(d) = self.dialogo.take() else { return RespostaDialogo::Nada };
        match (d, r) {
            (d, RespostaArquivo::Nada) => self.dialogo = Some(d),
            // Cancel no Save As do Yes: _LoadGame pergunta de novo.
            (Dialogo::SalvarAntesDoLoad(_), RespostaArquivo::Cancelar) => {
                self.dialogo = Some(Dialogo::PerguntaLoad(CaixaMensagem::pergunta_load(&self.dialogos_cfg)));
            }
            (Dialogo::SalvarAntesDoLoad(_), RespostaArquivo::Escolher(p)) => {
                if !self.gravar_e_avisar(&p, true) {
                    self.dialogo = Some(Dialogo::PerguntaLoad(CaixaMensagem::pergunta_load(&self.dialogos_cfg)));
                }
            }
            (Dialogo::SalvarComo(_), RespostaArquivo::Escolher(p)) => {
                self.gravar_e_avisar(&p, false);
            }
            (Dialogo::Load(_), RespostaArquivo::Escolher(p)) => return RespostaDialogo::Carregar(p),
            _ => {}
        }
        RespostaDialogo::Nada
    }

    /// Clique esquerdo com um diálogo aberto.
    pub(crate) fn clicar_no_dialogo(&mut self, x: i32, y: i32, duplo: bool) -> RespostaDialogo {
        let cfg = self.dialogos_cfg.clone();
        let r = match &mut self.dialogo {
            Some(Dialogo::PerguntaLoad(m) | Dialogo::Salvo(m, _)) => match m.botao_em(&self.interface, &cfg, x, y) {
                Some(i) => return self.escolher_botao(i),
                None => return RespostaDialogo::Nada,
            },
            Some(Dialogo::SalvarAntesDoLoad(d) | Dialogo::Load(d) | Dialogo::SalvarComo(d)) => d.clicar(&cfg, x, y, duplo),
            None => return RespostaDialogo::Nada,
        };
        self.responder_arquivo(r)
    }

    /// Tecla com um diálogo aberto (nome do SDL).
    pub(crate) fn tecla_no_dialogo(&mut self, nome: &str) -> RespostaDialogo {
        let cfg = self.dialogos_cfg.clone();
        let r = match &mut self.dialogo {
            Some(Dialogo::PerguntaLoad(m) | Dialogo::Salvo(m, _)) => match m.tecla(nome) {
                Some(i) => return self.escolher_botao(i),
                None => return RespostaDialogo::Nada,
            },
            Some(Dialogo::SalvarAntesDoLoad(d) | Dialogo::Load(d) | Dialogo::SalvarComo(d)) => d.tecla(nome, &cfg),
            None => return RespostaDialogo::Nada,
        };
        self.responder_arquivo(r)
    }

    /// Texto digitado com um diálogo de arquivo aberto.
    pub(crate) fn digitar_no_dialogo(&mut self, texto: &str) {
        let cfg = &self.dialogos_cfg;
        if let Some(Dialogo::SalvarAntesDoLoad(d) | Dialogo::Load(d) | Dialogo::SalvarComo(d)) = &mut self.dialogo {
            d.digitar(texto, cfg);
        }
    }

    /// Desenha o diálogo aberto por cima de tudo.
    pub(crate) fn desenhar_dialogo(&self, q: &mut Quadro) {
        match &self.dialogo {
            Some(Dialogo::PerguntaLoad(m) | Dialogo::Salvo(m, _)) => m.desenhar(q, &self.interface, &self.dialogos_cfg),
            Some(Dialogo::SalvarAntesDoLoad(d) | Dialogo::Load(d) | Dialogo::SalvarComo(d)) => {
                d.desenhar(q, &self.interface, &self.dialogos_cfg)
            }
            None => {}
        }
    }
}

/// Nova sessão com o arquivo escolhido no Load: `_RandYard` roda com o gerador R e o
/// `_CurGameType` da partida que estava aberta (carregar::geradores_depois_do_load).
pub(crate) fn carregar<'a>(s: &Sessao<'a>, arquivo: &Path, o: &Opcoes) -> Result<Sessao<'a>, String> {
    let r = s.partida.estado.r;
    let tipo = TipoJogo::de_i16(s.partida.estado.partida.tipo_jogo).unwrap_or(TipoJogo::Tutorial);
    println!("Load: {} (R da sessão, tipo {tipo:?})", arquivo.display());
    Sessao::abrir(s.recursos, &o.dados, Origem::Save(arquivo, r, tipo), s.cores.clone())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn pergunta_do_load_leva_ao_save_as_ou_ao_load() {
        // Só a máquina de estados (sem sessão): Yes → Save As, No → Load, Cancel → nada.
        let cfg: engine::core::dialogos::ConfigDialogos =
            serde_json::from_str(&std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/dialogos.json")).unwrap())
                .unwrap();
        let m = CaixaMensagem::pergunta_load(&cfg);
        assert_eq!(m.botoes.len(), 3);
        assert_eq!(m.padrao, 1);
    }
}
