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
    /// "SimAnt Quit: Do you want to save the current ant hill?" (`_MenuQuit`).
    PerguntaSair(CaixaMensagem),
    /// Save As pedido pelo Yes da pergunta do Quit (`_SaveGame(0)`); salvo, o jogo termina.
    SalvarAntesDeSair(DialogoArquivo),
}

/// O que a interface faz depois de uma entrada no diálogo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RespostaDialogo {
    Nada,
    /// Carregar este arquivo (nova sessão).
    Carregar(PathBuf),
    /// Sair do jogo (Quit respondido com No, ou Yes e salvo).
    Sair,
}

/// Nome do diálogo para o terminal.
fn nome_do_dialogo(d: &Dialogo) -> &'static str {
    match d {
        Dialogo::PerguntaLoad(_) => "Load question",
        Dialogo::SalvarAntesDoLoad(_) | Dialogo::SalvarComo(_) | Dialogo::SalvarAntesDeSair(_) => "Save As",
        Dialogo::Load(_) => "Load",
        Dialogo::Salvo(..) => "Saved correctly",
        Dialogo::PerguntaSair(_) => "Quit question",
    }
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

    /// File → Quit (`_MenuQuit`), Ctrl+Q ou fechar a janela: pergunta sempre (não olha
    /// `_gGameNeedsSaving`).
    pub(crate) fn abrir_pergunta_sair(&mut self) {
        self.dialogo = Some(Dialogo::PerguntaSair(CaixaMensagem::pergunta_sair(&self.dialogos_cfg)));
    }

    /// Fechar a janela principal (duplo clique na caixa, Close do menu de sistema, Alt+F4 ou o
    /// fechamento pelo sistema): a pergunta do Quit, sempre. Com um diálogo do Windows aberto, o
    /// original desabilita a janela principal e o SC_CLOSE não chega ao `MAINWNDPROC` (o Alt+F4 vai
    /// ao diálogo, que o trata como Cancel) [hipótese: comportamento do Windows 3.1, não conferido na
    /// bancada]; no port (extensão, docs/decisoes.md, 2026-10-19), o diálogo aberto é cancelado e a
    /// pergunta abre no lugar dele. Com a própria pergunta (ou o Save As do Yes dela) aberta, nada.
    /// Menus de segurar, o Examine, o mini mapa, o menu de um mod, o About e o Status fecham antes.
    /// Devolve se a pergunta abriu.
    pub(crate) fn pedir_para_fechar(&mut self) -> bool {
        if matches!(self.dialogo, Some(Dialogo::PerguntaSair(_) | Dialogo::SalvarAntesDeSair(_))) {
            println!("(bip)");
            return false;
        }
        if let Some(d) = &self.dialogo {
            println!("close: the open dialog ({}) is canceled", nome_do_dialogo(d));
        }
        self.menus.barra.fechar();
        self.menu = None;
        self.exame = None;
        modos::soltar_segurado(self);
        self.menu_mod = None;
        if self.mini.aberto() {
            mini_ui::fechar(self);
        }
        if let Some(sobre) = self.menus.sobre.take() {
            self.partida.definir_pausa(sobre.pausado_antes);
        }
        self.menus.status = false;
        self.abrir_pergunta_sair();
        true
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
                // O caminho completo, como o diálogo comum o devolve no original.
                let nome = std::path::absolute(&p).unwrap_or_else(|_| p.clone()).display().to_string();
                self.dialogo = Some(Dialogo::Salvo(CaixaMensagem::salvo(&self.dialogos_cfg, &nome), depois_load));
                true
            }
            Err(e) => {
                eprintln!("{}", engine::tr!("jogo.erro.salvar", erro = e));
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
            // _MenuQuit: Yes = Save As e sai se salvou; No = sai; Cancel = volta ao jogo.
            Some(Dialogo::PerguntaSair(_)) => match i {
                0 => self.dialogo = Some(Dialogo::SalvarAntesDeSair(self.dialogo_arquivo(true))),
                1 => return RespostaDialogo::Sair,
                _ => {}
            },
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
            // Cancel no Save As do Yes do Quit: _MenuQuit devolve 0 e o jogo continua.
            (Dialogo::SalvarAntesDeSair(_), RespostaArquivo::Escolher(p)) => {
                return match self.salvar(true, Some(&p)) {
                    Ok(p) => {
                        println!("{} Saved correctly", p.display());
                        RespostaDialogo::Sair
                    }
                    Err(e) => {
                        eprintln!("{}", engine::tr!("jogo.erro.salvar", erro = e));
                        RespostaDialogo::Nada
                    }
                };
            }
            (Dialogo::SalvarComo(_), RespostaArquivo::Escolher(p)) => {
                self.gravar_e_avisar(&p, false);
            }
            (Dialogo::Load(_), RespostaArquivo::Escolher(p)) => return RespostaDialogo::Carregar(p),
            _ => {}
        }
        RespostaDialogo::Nada
    }

    /// O diálogo de arquivo aberto, se houver.
    fn dialogo_de_arquivo(&mut self) -> Option<&mut DialogoArquivo> {
        match &mut self.dialogo {
            Some(Dialogo::SalvarAntesDoLoad(d) | Dialogo::Load(d) | Dialogo::SalvarComo(d) | Dialogo::SalvarAntesDeSair(d)) => Some(d),
            _ => None,
        }
    }

    /// Clique esquerdo com um diálogo aberto.
    pub(crate) fn clicar_no_dialogo(&mut self, x: i32, y: i32, duplo: bool) -> RespostaDialogo {
        self.clicar_no_dialogo_com(x, y, duplo, false)
    }

    /// Como [`Sessao::clicar_no_dialogo`], com Shift (estende a seleção do campo do nome).
    pub(crate) fn clicar_no_dialogo_com(&mut self, x: i32, y: i32, duplo: bool, shift: bool) -> RespostaDialogo {
        let cfg = self.dialogos_cfg.clone();
        let r = match &mut self.dialogo {
            Some(Dialogo::PerguntaLoad(m) | Dialogo::Salvo(m, _) | Dialogo::PerguntaSair(m)) => match m.botao_em(&self.interface, &cfg, x, y) {
                Some(i) => return self.escolher_botao(i),
                None => return RespostaDialogo::Nada,
            },
            Some(Dialogo::SalvarAntesDoLoad(d) | Dialogo::Load(d) | Dialogo::SalvarComo(d) | Dialogo::SalvarAntesDeSair(d)) => {
                d.clicar(&cfg, Some(&self.interface), x, y, duplo, shift)
            }
            None => return RespostaDialogo::Nada,
        };
        self.responder_arquivo(r)
    }

    /// Mouse andando com o botão apertado (arrasto do título, do polegar, seleção do campo).
    pub(crate) fn mover_no_dialogo(&mut self, x: i32, y: i32) -> bool {
        let cfg = self.dialogos_cfg.clone();
        let interface = &self.interface;
        match &mut self.dialogo {
            Some(Dialogo::SalvarAntesDoLoad(d) | Dialogo::Load(d) | Dialogo::SalvarComo(d) | Dialogo::SalvarAntesDeSair(d)) => d.mover_mouse(&cfg, Some(interface), x, y),
            _ => false,
        }
    }

    /// Botão solto com um diálogo aberto.
    pub(crate) fn soltar_no_dialogo(&mut self) -> bool {
        let cfg = self.dialogos_cfg.clone();
        let interface = &self.interface;
        match &mut self.dialogo {
            Some(Dialogo::SalvarAntesDoLoad(d) | Dialogo::Load(d) | Dialogo::SalvarComo(d) | Dialogo::SalvarAntesDeSair(d)) => d.soltar(&cfg, Some(interface)),
            _ => false,
        }
    }

    /// Roda do mouse com um diálogo aberto.
    pub(crate) fn rolar_no_dialogo(&mut self, x: i32, y: i32, linhas: i32) -> bool {
        let cfg = self.dialogos_cfg.clone();
        self.dialogo_de_arquivo().is_some_and(|d| d.rolar(&cfg, x, y, linhas))
    }

    /// Tempo passando com um diálogo aberto (repetição da seta segurada).
    pub(crate) fn avancar_dialogo(&mut self, ms: u32) -> bool {
        let cfg = self.dialogos_cfg.clone();
        self.dialogo_de_arquivo().is_some_and(|d| d.avancar(&cfg, ms))
    }

    /// Tecla com um diálogo aberto (nome do SDL).
    #[cfg(test)]
    pub(crate) fn tecla_no_dialogo(&mut self, nome: &str) -> RespostaDialogo {
        self.tecla_no_dialogo_com(nome, false, false, false)
    }

    /// Como [`Sessao::tecla_no_dialogo`], com Shift, Ctrl e Alt.
    pub(crate) fn tecla_no_dialogo_com(&mut self, nome: &str, shift: bool, ctrl: bool, alt: bool) -> RespostaDialogo {
        let cfg = self.dialogos_cfg.clone();
        let r = match &mut self.dialogo {
            Some(Dialogo::PerguntaLoad(m) | Dialogo::Salvo(m, _) | Dialogo::PerguntaSair(m)) => match m.tecla(nome) {
                Some(i) => return self.escolher_botao(i),
                None => return RespostaDialogo::Nada,
            },
            Some(Dialogo::SalvarAntesDoLoad(d) | Dialogo::Load(d) | Dialogo::SalvarComo(d) | Dialogo::SalvarAntesDeSair(d)) => {
                d.tecla_com(nome, shift, ctrl, alt, &cfg)
            }
            None => return RespostaDialogo::Nada,
        };
        self.responder_arquivo(r)
    }

    /// Texto digitado com um diálogo de arquivo aberto.
    pub(crate) fn digitar_no_dialogo(&mut self, texto: &str) {
        let cfg = &self.dialogos_cfg;
        if let Some(Dialogo::SalvarAntesDoLoad(d) | Dialogo::Load(d) | Dialogo::SalvarComo(d) | Dialogo::SalvarAntesDeSair(d)) = &mut self.dialogo {
            d.digitar(texto, cfg);
        }
    }

    /// Desenha o diálogo aberto por cima de tudo.
    pub(crate) fn desenhar_dialogo(&self, q: &mut Quadro) {
        match &self.dialogo {
            Some(Dialogo::PerguntaLoad(m) | Dialogo::Salvo(m, _) | Dialogo::PerguntaSair(m)) => m.desenhar(q, &self.interface, &self.dialogos_cfg),
            Some(Dialogo::SalvarAntesDoLoad(d) | Dialogo::Load(d) | Dialogo::SalvarComo(d) | Dialogo::SalvarAntesDeSair(d)) => {
                d.desenhar(q, &self.interface, &self.dialogos_cfg)
            }
            None => {}
        }
    }
}

/// "Load Game" no "Select a Game" da abertura: o "Load Ant Hill" sobre a tela da abertura (que é da
/// tela inteira; o diálogo fica `topo_na_tela` linhas abaixo, na área cliente). Não há partida para
/// salvar, então não há pergunta. Cancel volta ao "Select a Game" (`_DoMenuEntry`: `_LoadGame` falhou
/// sem jogo → `_NewGame(0)` de novo).
pub(crate) struct LoadNaAbertura {
    pub(crate) dialogo: DialogoArquivo,
    cfg: engine::core::dialogos::ConfigDialogos,
}

impl LoadNaAbertura {
    pub(crate) fn abrir(recursos: &Recursos) -> Result<LoadNaAbertura, String> {
        let cfg = engine::core::dialogos::ConfigDialogos::carregar(recursos)?;
        Ok(LoadNaAbertura { dialogo: DialogoArquivo::new(false, Path::new("."), &cfg), cfg })
    }

    /// Evento do SDL (coordenadas da tela). `Some` = respondido (arquivo ou Cancel).
    pub(crate) fn evento(&mut self, ev: &sdl2::event::Event, interface: &Interface) -> Option<RespostaArquivo> {
        use sdl2::event::Event;
        use sdl2::keyboard::Mod;
        use sdl2::mouse::MouseButton;
        let topo = i32::from(self.cfg.topo_na_tela);
        let r = match ev {
            Event::MouseButtonDown { mouse_btn: MouseButton::Left, clicks, x, y, .. } => {
                self.dialogo.clicar(&self.cfg, Some(interface), *x, *y - topo, *clicks >= 2, false)
            }
            Event::MouseMotion { x, y, mousestate, .. } if mousestate.left() => {
                self.dialogo.mover_mouse(&self.cfg, Some(interface), *x, *y - topo);
                RespostaArquivo::Nada
            }
            Event::MouseButtonUp { mouse_btn: MouseButton::Left, .. } => {
                self.dialogo.soltar(&self.cfg, Some(interface));
                RespostaArquivo::Nada
            }
            Event::MouseWheel { y, mouse_x, mouse_y, .. } => {
                self.dialogo.rolar(&self.cfg, *mouse_x, *mouse_y - topo, *y);
                RespostaArquivo::Nada
            }
            Event::KeyDown { keycode: Some(k), keymod, .. } => self.dialogo.tecla_com(
                &k.name(),
                keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
                keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
                keymod.intersects(Mod::LALTMOD | Mod::RALTMOD),
                &self.cfg,
            ),

            Event::TextInput { text, .. } => {
                self.dialogo.digitar(text, &self.cfg);
                RespostaArquivo::Nada
            }
            _ => RespostaArquivo::Nada,
        };
        (r != RespostaArquivo::Nada).then_some(r)
    }

    /// Desenha o diálogo por cima da tela da abertura.
    pub(crate) fn desenhar(&self, q: &mut Quadro, interface: &Interface) {
        let topo = self.cfg.topo_na_tela.max(0) as usize;
        let (w, h) = (q.largura, q.altura.saturating_sub(topo));
        let mut c = Quadro::new(w, h, 0);
        c.pixels.copy_from_slice(&q.pixels[topo * w..]);
        self.dialogo.desenhar(&mut c, interface, &self.cfg);
        q.pixels[topo * w..].copy_from_slice(&c.pixels);
    }
}

/// Nova sessão com o arquivo escolhido no Load: `_RandYard` roda com o gerador R e o
/// `_CurGameType` da partida que estava aberta (carregar::geradores_depois_do_load).
pub(crate) fn carregar<'a>(s: &Sessao<'a>, arquivo: &Path, o: &Opcoes) -> Result<Sessao<'a>, String> {
    let r = s.partida.estado.r;
    let tipo = TipoJogo::de_i16(s.partida.estado.partida.tipo_jogo).unwrap_or(TipoJogo::Tutorial);
    println!("Load: {} (session R, type {tipo:?})", arquivo.display());
    let tut: engine::core::carregar::ConfigTutorialArquivo = s.recursos.dados("tutorial_arquivo.json")?;
    if engine::core::carregar::e_o_tutorial(arquivo, &tut) {
        // _LoadGame com o nome do tutorial: não lê o arquivo, gera o tutorial (_SeedSRand pelo relógio).
        let semente = semente(&Opcoes { semente: None, ..o.clone() }) as u16;
        println!("Load: {} is the tutorial: _GenerateTutorial", arquivo.display());
        let mut nova = Sessao::abrir_na_tela(s.recursos, &o.dados, Origem::Tutorial(&s.partida.estado, semente), s.cores.clone(), o.tela_virtual)?;
        nova.herdar_janelas(s);
        nova.herdar_opcoes(s, false);
        return Ok(nova);
    }
    let mut nova = Sessao::abrir_na_tela(s.recursos, &o.dados, Origem::Save(arquivo, r, tipo), s.cores.clone(), o.tela_virtual)?;
    // As janelas da partida anterior ficam (o Load não as fecha); as opções vêm do arquivo.
    nova.herdar_janelas(s);
    nova.herdar_opcoes(s, true);
    Ok(nova)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn pergunta_do_load_leva_ao_save_as_ou_ao_load() {
        // Só a máquina de estados (sem sessão): Yes → Save As, No → Load, Cancel → nada.
        let cfg: engine::core::dialogos::ConfigDialogos =
            serde_json::from_str(&engine::recursos::marcar_referencias(
                &std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/dialogos.json")).unwrap(),
            ))
            .unwrap();
        let m = CaixaMensagem::pergunta_load(&cfg);
        assert_eq!(m.botoes.len(), 3);
        assert_eq!(m.padrao, 1);
    }
}
