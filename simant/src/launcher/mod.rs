//! Launcher: a tela de opções de inicialização do `simant`, aberta quando o programa roda sem opções
//! (duplo clique) ou com `--launcher`. Escolhe a pasta do SimAnt original, o tipo de início, vídeo,
//! som, mods e preferências, mostra a linha de comando equivalente (editável) e guarda tudo em
//! `launcher.json` na pasta de dados do usuário (`config_launcher.rs`). Desenhada em SDL2 por
//! software, sem nada do jogo original. Decisão em docs/decisoes.md (2026-10-19, "Launcher dentro
//! do simant").

pub mod desenho;
pub mod dialogo_nativo;
mod fonte8x8;

use std::path::{Path, PathBuf};

use crate::config_launcher::{self as cfgl, ConfigLauncher, InfoMod, Inicio, Validacao};
use desenho::*;

pub const LARGURA: i32 = 640;
pub const ALTURA: i32 = 480;

/// Os campos de texto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdCampo {
    Jogo,
    Save,
    Janela,
    SoundFont,
    Mods,
    WinIni,
    Linha,
}

const CAMPOS: [IdCampo; 7] = [IdCampo::Jogo, IdCampo::Save, IdCampo::Janela, IdCampo::SoundFont, IdCampo::Mods, IdCampo::WinIni, IdCampo::Linha];

#[derive(Clone, Debug, Default)]
struct Campo {
    texto: String,
    /// Posição do cursor, em caracteres.
    cursor: usize,
    /// Primeiro caractere (campo de uma linha) ou primeira linha (campo de várias) visível.
    rolagem: usize,
}

/// O que um clique faz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acao {
    Procurar(IdCampo),
    Focar(IdCampo),
    Inicio(Inicio),
    Tipo(usize),
    Resolucao(Option<usize>),
    Escala(Option<u32>),
    Moldura,
    Perfil(Option<usize>),
    Som,
    Sintetizador(Option<usize>),
    /// 0 = `mods/` do port, 1 = `exemplos/mods`, 2 = outra.
    PastaMods(u8),
    Mod(usize),
    Preferencias,
    /// Fast/Ultra no ritmo do 486 da bancada (`--velocidades original`) ou o proporcional (padrão).
    VelocidadesOriginais,
    /// Lenta sem foco (padrão) ou o ritmo do original (`--sem-foco original`).
    LentaSemFoco,
    MostrarLauncher,
    Jogar,
    Salvar,
    Sair,
}

/// Resultado do launcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Saida {
    /// Rodar o jogo com estas opções.
    Jogar(Vec<String>),
    Sair,
}

#[derive(Clone, Debug)]
enum Tipo {
    Texto(String, Cor, bool),
    Secao(String),
    Botao(String, Acao, bool),
    Opcao(String, Acao, bool, bool),
    Caixa(String, Acao, bool, bool),
    Campo(IdCampo, usize),
}

#[derive(Clone, Debug)]
struct Widget {
    r: Ret,
    tipo: Tipo,
}

pub struct Estado {
    pub cfg: ConfigLauncher,
    /// Onde a configuração é gravada (`None`: não grava).
    caminho: Option<PathBuf>,
    base: PathBuf,
    perfis: Vec<String>,
    validacao: Validacao,
    mods: Vec<InfoMod>,
    win_ini_auto: Option<PathBuf>,
    campos: Vec<Campo>,
    foco: Option<IdCampo>,
    /// Mensagem da última linha e a cor.
    status: (String, Cor),
    apertado: Option<Acao>,
}

const EXEMPLOS: &str = "exemplos/mods";

impl Estado {
    pub fn novo(cfg: Option<ConfigLauncher>, caminho: Option<PathBuf>, base: PathBuf) -> Estado {
        let mut cfg = cfg.unwrap_or_default();
        // Sem pasta escolhida: a que o jogo usaria (SIMANT_DIR ou SimAnt-T/MAXIS/ANTWIN), se for válida.
        if cfg.jogo.is_none() {
            let padrao = crate::opcoes::pasta_jogo_padrao(&base);
            if cfgl::validar_pasta_jogo(&padrao).ok {
                cfg.jogo = Some(padrao.display().to_string());
            }
        }
        let perfis = perfis_driver(&base.join("data"));
        let mut e = Estado {
            cfg,
            caminho,
            base,
            perfis,
            validacao: Validacao { ok: false, mensagem: String::new() },
            mods: Vec::new(),
            win_ini_auto: None,
            campos: vec![Campo::default(); CAMPOS.len()],
            foco: None,
            status: (String::new(), PRETO),
            apertado: None,
        };
        e.campos_da_config();
        e.atualizar();
        e.status = match &e.caminho {
            Some(c) if c.is_file() => (format!("Configuração: {}", c.display()), CINZA),
            _ => ("Escolha a pasta do SimAnt e clique em Jogar.".into(), CINZA),
        };
        e
    }

    fn campo(&self, id: IdCampo) -> &Campo {
        &self.campos[id as usize]
    }

    fn campo_mut(&mut self, id: IdCampo) -> &mut Campo {
        &mut self.campos[id as usize]
    }

    fn pasta_jogo(&self) -> PathBuf {
        match &self.cfg.jogo {
            Some(j) => crate::pastas::resolver(Path::new(j), &self.base),
            None => crate::opcoes::pasta_jogo_padrao(&self.base),
        }
    }

    fn pasta_mods(&self) -> PathBuf {
        crate::pastas::resolver(Path::new(self.cfg.mods.as_deref().unwrap_or("mods")), &self.base)
    }

    /// Escolha de pasta de mods: 0 = padrão, 1 = exemplos, 2 = outra.
    fn tipo_pasta_mods(&self) -> u8 {
        match self.cfg.mods.as_deref() {
            None => 0,
            Some(m) if m.replace('\\', "/") == EXEMPLOS => 1,
            Some(_) => 2,
        }
    }

    /// Textos dos campos a partir da configuração (depois de uma mudança que não veio deles).
    fn campos_da_config(&mut self) {
        let texto = |o: &Option<String>| o.clone().unwrap_or_default();
        let valores = [
            texto(&self.cfg.jogo),
            texto(&self.cfg.save),
            self.cfg.janela.map(|[l, a]| format!("{l}x{a}")).unwrap_or_default(),
            texto(&self.cfg.soundfont),
            if self.tipo_pasta_mods() == 2 { texto(&self.cfg.mods) } else { String::new() },
            texto(&self.cfg.win_ini),
        ];
        for (id, v) in CAMPOS.iter().zip(valores) {
            if self.foco != Some(*id) {
                let c = self.campo_mut(*id);
                c.cursor = v.chars().count();
                c.texto = v;
                c.rolagem = 0;
            }
        }
    }

    /// Recalcula o que depende da configuração: conferência da pasta, mods, WIN.INI e a linha.
    fn atualizar(&mut self) {
        self.validacao = match &self.cfg.jogo {
            None => Validacao { ok: false, mensagem: "Escolha a pasta onde está o SimAnt (com SHARED.NDX e WINGANT.NDX).".into() },
            Some(_) => cfgl::validar_pasta_jogo(&self.pasta_jogo()),
        };
        self.mods = cfgl::listar_mods(&self.pasta_mods());
        self.win_ini_auto = crate::preferencias::procurar_win_ini(&self.pasta_jogo());
        if self.foco != Some(IdCampo::Linha) {
            let linha = cfgl::formatar_linha(&self.cfg.argumentos());
            let c = self.campo_mut(IdCampo::Linha);
            c.cursor = linha.chars().count();
            c.texto = linha;
        }
    }

    /// O texto do campo `id` mudou (digitação, colagem): passa para a configuração. A linha de
    /// comando só é aplicada ao confirmar ([`Estado::confirmar_linha`]).
    fn campo_mudou(&mut self, id: IdCampo) {
        let t = self.campo(id).texto.trim().to_string();
        let opcional = |t: &str| (!t.is_empty()).then(|| t.to_string());
        match id {
            IdCampo::Jogo => self.cfg.jogo = opcional(&t),
            IdCampo::Save => {
                self.cfg.save = opcional(&t);
                if self.cfg.save.is_some() {
                    self.cfg.inicio = Inicio::Save;
                }
            }
            IdCampo::Janela => {
                if t.is_empty() {
                    self.cfg.janela = None;
                } else {
                    match crate::video::ler_tamanho("Janela", &t) {
                        Ok((l, a)) => self.cfg.janela = Some([l, a]),
                        Err(_) => {
                            self.status = ("Janela: use LARGURAxALTURA (ex.: 1280x960) ou deixe vazio.".into(), VERMELHO);
                            return;
                        }
                    }
                }
            }
            IdCampo::SoundFont => self.cfg.soundfont = opcional(&t),
            IdCampo::Mods => self.cfg.mods = opcional(&t),
            IdCampo::WinIni => self.cfg.win_ini = opcional(&t),
            IdCampo::Linha => return,
        }
        self.atualizar();
    }

    /// Aplica a linha de comando editada à configuração (e aos outros campos).
    fn confirmar_linha(&mut self) -> bool {
        let texto = self.campo(IdCampo::Linha).texto.clone();
        match cfgl::dividir_linha(&texto).and_then(|a| self.cfg.de_argumentos(&a)) {
            Ok(c) => {
                self.cfg = c;
                let foco = self.foco.take();
                self.campos_da_config();
                self.atualizar();
                self.foco = foco.filter(|f| *f != IdCampo::Linha);
                true
            }
            Err(e) => {
                self.status = (format!("Linha de comando: {e}"), VERMELHO);
                false
            }
        }
    }

    /// Tira o foco do campo atual (a linha de comando é aplicada).
    fn desfocar(&mut self) -> bool {
        match self.foco {
            Some(IdCampo::Linha) => {
                if self.confirmar_linha() {
                    self.foco = None;
                    true
                } else {
                    false
                }
            }
            _ => {
                self.foco = None;
                self.atualizar();
                true
            }
        }
    }

    pub fn acao(&mut self, a: Acao) -> Option<Saida> {
        if !matches!(a, Acao::Focar(_)) && !self.desfocar() {
            return None;
        }
        match a {
            Acao::Focar(id) => {
                if self.foco != Some(id) && !self.desfocar() {
                    return None;
                }
                self.foco = Some(id);
            }
            Acao::Procurar(id) => self.procurar(id),
            Acao::Inicio(i) => self.cfg.inicio = i,
            Acao::Tipo(t) => {
                self.cfg.tipo = cfgl::TIPOS[t].0.into();
                self.cfg.inicio = Inicio::NovoJogo;
            }
            Acao::Resolucao(r) => self.cfg.resolucao = r.map(|i| cfgl::RESOLUCOES[i]),
            Acao::Escala(e) => self.cfg.escala = e,
            // Ligada por padrão (data/video.json): desmarcar grava --sem-moldura; marcar volta ao padrão.
            Acao::Moldura => self.cfg.moldura = if self.cfg.moldura != Some(false) { Some(false) } else { None },
            Acao::Perfil(p) => self.cfg.perfil_driver = p.map(|i| self.perfis[i].clone()),
            Acao::Som => self.cfg.som = !self.cfg.som,
            Acao::Sintetizador(s) => self.cfg.sintetizador = s.map(|i| cfgl::SINTETIZADORES[i].0.into()),
            Acao::PastaMods(p) => {
                self.cfg.mods = match p {
                    0 => None,
                    1 => Some(EXEMPLOS.into()),
                    _ => Some(self.campo(IdCampo::Mods).texto.trim().to_string()).filter(|t| !t.is_empty()).or_else(|| Some("mods".into())),
                };
                if p != 2 {
                    self.campo_mut(IdCampo::Mods).texto.clear();
                }
            }
            Acao::Mod(i) => {
                if let Some(m) = self.mods.get(i) {
                    let nome = m.nome.clone();
                    match self.cfg.mods_desligados.iter().position(|n| *n == nome) {
                        Some(p) => {
                            self.cfg.mods_desligados.remove(p);
                        }
                        None => self.cfg.mods_desligados.push(nome),
                    }
                }
            }
            Acao::Preferencias => self.cfg.preferencias = !self.cfg.preferencias,
            Acao::VelocidadesOriginais => {
                self.cfg.velocidades = if self.cfg.velocidades.as_deref() == Some("original") { None } else { Some("original".into()) }
            }
            Acao::LentaSemFoco => self.cfg.sem_foco = if self.cfg.sem_foco.as_deref() == Some("original") { None } else { Some("original".into()) },
            Acao::MostrarLauncher => self.cfg.mostrar_launcher = !self.cfg.mostrar_launcher,
            Acao::Jogar => return self.jogar(),
            Acao::Salvar => {
                self.salvar();
            }
            Acao::Sair => return Some(Saida::Sair),
        }
        self.campos_da_config();
        self.atualizar();
        None
    }

    fn salvar(&mut self) -> bool {
        let Some(c) = &self.caminho else {
            self.status = ("Sem pasta de dados do usuário: a configuração não pode ser gravada.".into(), VERMELHO);
            return false;
        };
        match self.cfg.gravar(c) {
            Ok(()) => {
                self.status = (format!("Configuração salva em {}", c.display()), VERDE);
                true
            }
            Err(e) => {
                self.status = (format!("Não foi possível salvar: {e}"), VERMELHO);
                false
            }
        }
    }

    fn jogar(&mut self) -> Option<Saida> {
        if !self.validacao.ok {
            self.status = (format!("Pasta do jogo: {}", self.validacao.mensagem), VERMELHO);
            return None;
        }
        if self.cfg.inicio == Inicio::Save && self.cfg.save.is_none() {
            self.status = ("Escolha o arquivo do jogo salvo (.ANT).".into(), VERMELHO);
            return None;
        }
        if self.caminho.is_some() {
            self.salvar();
        }
        Some(Saida::Jogar(self.cfg.argumentos()))
    }

    fn procurar(&mut self, id: IdCampo) {
        use dialogo_nativo::{escolher_arquivo, escolher_pasta, Filtro};
        let atual = self.campo(id).texto.trim().to_string();
        let inicial = (!atual.is_empty()).then(|| crate::pastas::resolver(Path::new(&atual), &self.base));
        let inicial = inicial.as_deref();
        let r = match id {
            IdCampo::Jogo => escolher_pasta("Pasta da instalação do SimAnt (com SHARED.NDX)", inicial.or(Some(&self.base))),
            IdCampo::Mods => escolher_pasta("Pasta de mods", inicial.or(Some(&self.base))),
            IdCampo::Save => {
                let jogo = self.pasta_jogo();
                escolher_arquivo("Jogo salvo do SimAnt", inicial.or(Some(&jogo)), Filtro { nome: "Jogos do SimAnt (*.ANT)", padroes: &["*.ANT"] })
            }
            IdCampo::SoundFont => escolher_arquivo("SoundFont General MIDI", inicial, Filtro { nome: "SoundFont (*.sf2)", padroes: &["*.sf2"] }),
            IdCampo::WinIni => escolher_arquivo("WIN.INI do Windows 3.1", inicial, Filtro { nome: "WIN.INI", padroes: &["*.INI"] }),
            IdCampo::Janela | IdCampo::Linha => return,
        };
        match r {
            Ok(Some(p)) => self.definir_caminho(id, &p),
            Ok(None) => {}
            Err(e) => self.status = (e, VERMELHO),
        }
    }

    /// Um caminho escolhido (diálogo ou arrastado para a janela) vai para o campo `id`.
    fn definir_caminho(&mut self, id: IdCampo, p: &Path) {
        let t = p.display().to_string();
        let c = self.campo_mut(id);
        c.cursor = t.chars().count();
        c.texto = t;
        if id == IdCampo::Mods {
            self.cfg.mods = Some(p.display().to_string());
        }
        self.campo_mudou(id);
        if id == IdCampo::Jogo {
            self.status = (self.validacao.mensagem.clone(), if self.validacao.ok { VERDE } else { VERMELHO });
        }
        self.campos_da_config();
        self.atualizar();
    }

    /// Arquivo ou pasta arrastado para a janela: pasta com SHARED.NDX = jogo; `.ANT` = save;
    /// `.sf2` = SoundFont; `.INI` = WIN.INI; outra pasta com mods = pasta de mods.
    pub fn soltar(&mut self, p: &Path) {
        if !self.desfocar() {
            return;
        }
        let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let id = if p.is_dir() {
            if cfgl::validar_pasta_jogo(p).ok {
                IdCampo::Jogo
            } else if !cfgl::listar_mods(p).is_empty() {
                IdCampo::Mods
            } else {
                IdCampo::Jogo
            }
        } else {
            match ext.as_str() {
                "ant" => IdCampo::Save,
                "sf2" => IdCampo::SoundFont,
                "ini" => IdCampo::WinIni,
                _ => {
                    self.status = (format!("Não sei o que fazer com {}", p.display()), VERMELHO);
                    return;
                }
            }
        };
        self.definir_caminho(id, p);
    }

    // -----------------------------------------------------------------------------------------
    // Teclado
    // -----------------------------------------------------------------------------------------

    /// Texto digitado (ou colado) no campo com foco.
    pub fn digitar(&mut self, s: &str) {
        let Some(id) = self.foco else { return };
        let c = self.campo_mut(id);
        let limpo: String = s.chars().filter(|ch| !ch.is_control()).collect();
        let pos = byte_do_caractere(&c.texto, c.cursor);
        c.texto.insert_str(pos, &limpo);
        c.cursor += limpo.chars().count();
        self.campo_mudou(id);
    }

    /// Teclas de edição e de navegação. Devolve a saída se a tecla terminou o launcher.
    pub fn tecla(&mut self, t: Tecla) -> Option<Saida> {
        let Some(id) = self.foco else {
            return match t {
                Tecla::Enter => self.acao(Acao::Jogar),
                Tecla::Esc => self.acao(Acao::Sair),
                Tecla::Tab => {
                    self.foco = Some(IdCampo::Jogo);
                    None
                }
                _ => None,
            };
        };
        let colunas = self.colunas(id);
        let c = self.campo_mut(id);
        let n = c.texto.chars().count();
        let mut mudou = false;
        match t {
            Tecla::Esquerda => c.cursor = c.cursor.saturating_sub(1),
            Tecla::Direita => c.cursor = (c.cursor + 1).min(n),
            Tecla::Inicio => c.cursor = 0,
            Tecla::Fim => c.cursor = n,
            Tecla::Cima => c.cursor = c.cursor.saturating_sub(colunas),
            Tecla::Baixo => c.cursor = (c.cursor + colunas).min(n),
            Tecla::Apagar => {
                if c.cursor > 0 {
                    let p = byte_do_caractere(&c.texto, c.cursor - 1);
                    c.texto.remove(p);
                    c.cursor -= 1;
                    mudou = true;
                }
            }
            Tecla::Delete => {
                if c.cursor < n {
                    let p = byte_do_caractere(&c.texto, c.cursor);
                    c.texto.remove(p);
                    mudou = true;
                }
            }
            Tecla::Enter | Tecla::Esc => {
                self.desfocar();
                return None;
            }
            Tecla::Tab => {
                if self.desfocar() {
                    let i = CAMPOS.iter().position(|x| *x == id).unwrap_or(0);
                    self.foco = Some(CAMPOS[(i + 1) % CAMPOS.len()]);
                }
                return None;
            }
        }
        if mudou {
            self.campo_mudou(id);
        }
        None
    }

    /// O texto do campo com foco (para Ctrl+C), ou a linha de comando.
    pub fn texto_para_copiar(&self) -> String {
        self.campo(self.foco.unwrap_or(IdCampo::Linha)).texto.clone()
    }

    fn colunas(&self, id: IdCampo) -> usize {
        let (_, r, _) = posicao_campo(id);
        ((r.w - 6) / CAR).max(1) as usize
    }

    // -----------------------------------------------------------------------------------------
    // Mouse
    // -----------------------------------------------------------------------------------------

    pub fn apertar(&mut self, x: i32, y: i32) -> Option<Saida> {
        let w = self.widgets();
        let alvo = w.iter().rev().find(|w| w.r.contem(x, y)).map(|w| (w.tipo.clone(), w.r));
        match alvo {
            Some((Tipo::Botao(_, a, _), _)) => self.apertado = Some(a),
            Some((Tipo::Opcao(_, a, _, _) | Tipo::Caixa(_, a, _, _), _)) => return self.acao(a),
            Some((Tipo::Campo(id, linhas), r)) => {
                self.acao(Acao::Focar(id));
                if self.foco == Some(id) {
                    let colunas = self.colunas(id);
                    let c = self.campo_mut(id);
                    let n = c.texto.chars().count();
                    let col = ((x - r.x - 3).max(0) / CAR) as usize;
                    c.cursor = if linhas > 1 {
                        let lin = ((y - r.y - 3).max(0) / ALTURA_LINHA_CAMPO) as usize;
                        ((c.rolagem + lin) * colunas + col.min(colunas - 1)).min(n)
                    } else {
                        (c.rolagem + col).min(n)
                    };
                }
            }
            _ => {
                self.desfocar();
            }
        }
        None
    }

    pub fn soltar_botao(&mut self, x: i32, y: i32) -> Option<Saida> {
        let a = self.apertado.take()?;
        let em_cima = self.widgets().iter().any(|w| matches!(&w.tipo, Tipo::Botao(_, b, _) if *b == a) && w.r.contem(x, y));
        if em_cima {
            self.acao(a)
        } else {
            None
        }
    }

    // -----------------------------------------------------------------------------------------
    // Layout e desenho
    // -----------------------------------------------------------------------------------------

    fn widgets(&self) -> Vec<Widget> {
        let mut w = Vec::new();
        let x0 = 12;
        let rotulo_w = 13 * CAR;
        let texto = |w: &mut Vec<Widget>, x: i32, y: i32, t: &str, cor: Cor| {
            let largura = t.chars().count() as i32 * CAR;
            w.push(Widget { r: Ret { x, y, w: largura, h: CAR }, tipo: Tipo::Texto(t.into(), cor, false) });
            x + largura
        };
        let secao = |w: &mut Vec<Widget>, y: i32, t: &str| w.push(Widget { r: Ret { x: x0, y, w: LARGURA - 2 * x0, h: CAR }, tipo: Tipo::Secao(t.into()) });
        // Uma fileira de botões de opção a partir de x; devolve o x do fim.
        let opcoes = |w: &mut Vec<Widget>, x: i32, y: i32, itens: &[(String, Acao, bool)], ativa: bool| {
            let mut x = x;
            for (t, a, m) in itens {
                let largura = 14 + t.chars().count() as i32 * CAR;
                w.push(Widget { r: Ret { x, y: y - 1, w: largura, h: 10 }, tipo: Tipo::Opcao(t.clone(), *a, *m, ativa) });
                x += largura + 10;
            }
            x
        };
        let caixa = |w: &mut Vec<Widget>, x: i32, y: i32, t: &str, a: Acao, m: bool| {
            let largura = 14 + t.chars().count() as i32 * CAR;
            w.push(Widget { r: Ret { x, y: y - 1, w: largura, h: 10 }, tipo: Tipo::Caixa(t.into(), a, m, true) });
            x + largura
        };
        let campo = |w: &mut Vec<Widget>, id: IdCampo| {
            let (_, r, linhas) = posicao_campo(id);
            w.push(Widget { r, tipo: Tipo::Campo(id, linhas) });
            if let Some(b) = botao_procurar(id) {
                w.push(Widget { r: b, tipo: Tipo::Botao("Procurar...".into(), Acao::Procurar(id), false) });
            }
        };
        let c = &self.cfg;

        // Pasta do jogo.
        secao(&mut w, 36, "Pasta da instalação do SimAnt (Windows 3.x)");
        campo(&mut w, IdCampo::Jogo);
        let cor = if self.validacao.ok { VERDE } else { VERMELHO };
        texto(&mut w, x0, 70, &encurtar(&self.validacao.mensagem, 77), cor);

        // Início.
        secao(&mut w, 86, "Início");
        let inicios = [("Abertura e \"Select a Game\"", Inicio::Abertura), ("Jogo novo", Inicio::NovoJogo), ("Jogo salvo", Inicio::Save)];
        let itens: Vec<_> = inicios.iter().map(|(t, i)| (t.to_string(), Acao::Inicio(*i), c.inicio == *i)).collect();
        opcoes(&mut w, x0, 101, &itens, true);
        texto(&mut w, x0 + 16, 117, "Tipo:", PRETO);
        let itens: Vec<_> = cfgl::TIPOS.iter().enumerate().map(|(i, (n, t))| (t.to_string(), Acao::Tipo(i), c.tipo == *n)).collect();
        opcoes(&mut w, x0 + rotulo_w, 117, &itens, c.inicio == Inicio::NovoJogo);
        texto(&mut w, x0 + 16, 133, "Arquivo:", if c.inicio == Inicio::Save { PRETO } else { CINZA });
        campo(&mut w, IdCampo::Save);

        // Vídeo.
        secao(&mut w, 152, "Vídeo");
        texto(&mut w, x0, 167, "Resolução:", PRETO);
        let mut itens = vec![("Padrão".to_string(), Acao::Resolucao(None), c.resolucao.is_none())];
        itens.extend(cfgl::RESOLUCOES.iter().enumerate().map(|(i, [l, a])| (format!("{l}x{a}"), Acao::Resolucao(Some(i)), c.resolucao == Some([*l, *a]))));
        opcoes(&mut w, x0 + rotulo_w, 167, &itens, true);
        caixa(&mut w, 452, 167, "Moldura Win 3.1", Acao::Moldura, c.moldura != Some(false));
        texto(&mut w, x0, 183, "Escala:", PRETO);
        let mut itens = vec![("Padrão".to_string(), Acao::Escala(None), c.escala.is_none())];
        itens.extend((1..=4).map(|e| (format!("{e}x"), Acao::Escala(Some(e)), c.escala == Some(e))));
        itens.push(("Máxima".to_string(), Acao::Escala(Some(0)), c.escala == Some(0)));
        opcoes(&mut w, x0 + rotulo_w, 183, &itens, true);
        texto(&mut w, 452, 183, "Janela:", PRETO);
        campo(&mut w, IdCampo::Janela);
        texto(&mut w, x0, 199, "Driver:", PRETO);
        let mut itens = vec![("Padrão".to_string(), Acao::Perfil(None), c.perfil_driver.is_none())];
        itens.extend(self.perfis.iter().enumerate().map(|(i, p)| (nome_perfil(p), Acao::Perfil(Some(i)), c.perfil_driver.as_deref() == Some(p))));
        opcoes(&mut w, x0 + rotulo_w, 199, &itens, true);

        // Som.
        secao(&mut w, 218, "Som");
        let fim = caixa(&mut w, x0, 233, "Som ligado", Acao::Som, c.som);
        let x = texto(&mut w, fim + 24, 233, "Sintetizador:", if c.som { PRETO } else { CINZA });
        let mut itens = vec![("Padrão".to_string(), Acao::Sintetizador(None), c.sintetizador.is_none())];
        itens.extend(cfgl::SINTETIZADORES.iter().enumerate().map(|(i, (n, t))| (t.to_string(), Acao::Sintetizador(Some(i)), c.sintetizador.as_deref() == Some(n))));
        opcoes(&mut w, x + CAR, 233, &itens, c.som);
        texto(&mut w, x0 + 16, 249, "SoundFont:", if c.som { PRETO } else { CINZA });
        campo(&mut w, IdCampo::SoundFont);

        // Mods.
        secao(&mut w, 268, "Mods");
        texto(&mut w, x0, 283, "Pasta:", PRETO);
        let tp = self.tipo_pasta_mods();
        let itens = [("mods".to_string(), Acao::PastaMods(0), tp == 0), ("exemplos/mods".to_string(), Acao::PastaMods(1), tp == 1), ("Outra:".to_string(), Acao::PastaMods(2), tp == 2)];
        opcoes(&mut w, x0 + 7 * CAR, 283, &itens, true);
        campo(&mut w, IdCampo::Mods);
        if self.mods.is_empty() {
            texto(&mut w, x0 + 16, 299, &format!("(nenhum mod em {})", encurtar(&self.pasta_mods().display().to_string(), 60)), CINZA);
        } else {
            for (i, m) in self.mods.iter().take(8).enumerate() {
                let (x, y) = (x0 + 16 + (i as i32 % 4) * 150, 299 + (i as i32 / 4) * 14);
                let t = encurtar(&format!("{} {}", m.nome, m.versao), 15);
                caixa(&mut w, x, y, &t, Acao::Mod(i), !c.mods_desligados.contains(&m.nome));
            }
            if self.mods.len() > 8 {
                texto(&mut w, x0 + 16, 327, &format!("... e mais {} (ligados)", self.mods.len() - 8), CINZA);
            }
        }

        // Preferências.
        secao(&mut w, 340, "Preferências do jogador");
        caixa(&mut w, x0, 355, "Guardar opções entre partidas", Acao::Preferencias, c.preferencias);
        texto(&mut w, 300, 355, "WIN.INI:", if c.preferencias { PRETO } else { CINZA });
        campo(&mut w, IdCampo::WinIni);
        let auto = match (&c.win_ini, &self.win_ini_auto) {
            (Some(_), _) => "Lido só na 1a partida; nunca gravado.".to_string(),
            (None, Some(p)) => format!("Auto: {}", encurtar(&p.display().to_string(), 33)),
            (None, None) => "Auto: nenhum WIN.INI perto do jogo.".to_string(),
        };
        // Ritmo (extensões do port, data/partida.json): marcado = o do original.
        let fim = caixa(&mut w, x0, 371, "Fast/Ultra do 486", Acao::VelocidadesOriginais, c.velocidades.as_deref() == Some("original"));
        caixa(&mut w, fim + 10, 371, "Lenta sem foco", Acao::LentaSemFoco, c.sem_foco.as_deref() != Some("original"));
        texto(&mut w, 316, 371, &encurtar(&auto, 39), CINZA);

        // Linha de comando.
        secao(&mut w, 388, "Linha de comando equivalente (editável; Enter aplica)");
        campo(&mut w, IdCampo::Linha);

        caixa(&mut w, x0, 441, "Abrir esta tela ao iniciar", Acao::MostrarLauncher, c.mostrar_launcher);
        let botoes = [("Jogar", Acao::Jogar, true, 72), ("Salvar configuração", Acao::Salvar, false, 168), ("Sair", Acao::Sair, false, 64)];
        let mut x = LARGURA - x0;
        for (t, a, padrao, largura) in botoes.iter().rev() {
            x -= largura;
            w.push(Widget { r: Ret { x, y: 434, w: *largura, h: 22 }, tipo: Tipo::Botao(t.to_string(), *a, *padrao) });
            x -= 8;
        }
        let (s, cor) = &self.status;
        texto(&mut w, x0, 464, &encurtar(s, 77), *cor);
        w
    }

    /// Desenha a tela inteira. `cursor_visivel`: o cursor do campo com foco (pisca).
    pub fn desenhar(&mut self, cursor_visivel: bool) -> Tela {
        let mut t = Tela::nova(LARGURA, ALTURA);
        // Faixa do título.
        t.cheio(Ret { x: 0, y: 0, w: LARGURA, h: 28 }, AZUL);
        t.texto(12, 6, "SimAnt recomp", BRANCO, 2);
        let sub = format!("Opções de inicialização - versão {}", env!("CARGO_PKG_VERSION"));
        t.texto(LARGURA - 12 - sub.chars().count() as i32 * CAR, 14, &sub, BRANCO, 1);
        t.horizontal(0, 28, LARGURA, PRETO);
        // Rolagem dos campos para o cursor ficar visível.
        for id in CAMPOS {
            let (_, r, linhas) = posicao_campo(id);
            let colunas = ((r.w - 6) / CAR).max(1) as usize;
            let c = self.campo_mut(id);
            if linhas > 1 {
                let lin = c.cursor / colunas;
                if lin < c.rolagem {
                    c.rolagem = lin;
                } else if lin >= c.rolagem + linhas {
                    c.rolagem = lin + 1 - linhas;
                }
            } else if c.cursor < c.rolagem {
                c.rolagem = c.cursor;
            } else if c.cursor >= c.rolagem + colunas {
                c.rolagem = c.cursor + 1 - colunas;
            }
        }
        for w in self.widgets() {
            match &w.tipo {
                Tipo::Texto(s, cor, negrito) => {
                    if *negrito {
                        t.negrito(w.r.x, w.r.y, s, *cor);
                    } else {
                        t.texto(w.r.x, w.r.y, s, *cor, 1);
                    }
                }
                Tipo::Secao(s) => {
                    let fim = w.r.x + t.negrito(w.r.x, w.r.y, s, PRETO) + 6;
                    t.horizontal(fim, w.r.y + 4, w.r.x + w.r.w - fim, CINZA);
                    t.horizontal(fim, w.r.y + 5, w.r.x + w.r.w - fim, BRANCO);
                }
                Tipo::Botao(s, a, padrao) => t.botao(w.r, s, *padrao, self.apertado == Some(*a)),
                Tipo::Opcao(s, _, m, ativa) => {
                    t.opcao(w.r.x, w.r.y, *m, *ativa);
                    t.texto(w.r.x + 14, w.r.y + 1, s, if *ativa { PRETO } else { CINZA }, 1);
                }
                Tipo::Caixa(s, _, m, ativa) => {
                    t.caixa(w.r.x, w.r.y, *m, *ativa);
                    t.texto(w.r.x + 14, w.r.y + 1, s, if *ativa { PRETO } else { CINZA }, 1);
                }
                Tipo::Campo(id, linhas) => {
                    let focado = self.foco == Some(*id);
                    t.cheio(w.r, BRANCO);
                    t.contorno(w.r, if focado { AZUL } else { PRETO });
                    if focado {
                        t.contorno(Ret { x: w.r.x - 1, y: w.r.y - 1, w: w.r.w + 2, h: w.r.h + 2 }, AZUL);
                    }
                    let c = self.campo(*id);
                    let colunas = ((w.r.w - 6) / CAR).max(1) as usize;
                    let chars: Vec<char> = c.texto.chars().collect();
                    let vazio = chars.is_empty() && !focado;
                    if vazio {
                        if let Some(dica) = dica_campo(*id) {
                            t.texto(w.r.x + 3, w.r.y + 3, &encurtar(dica, colunas), CINZA, 1);
                        }
                    }
                    for lin in 0..*linhas {
                        let (ini, fim) = if *linhas > 1 {
                            let ini = (c.rolagem + lin) * colunas;
                            (ini, ini + colunas)
                        } else {
                            (c.rolagem, c.rolagem + colunas)
                        };
                        let pedaco: String = chars.iter().skip(ini).take(fim.saturating_sub(ini)).collect();
                        t.texto(w.r.x + 3, w.r.y + 3 + lin as i32 * ALTURA_LINHA_CAMPO, &pedaco, PRETO, 1);
                        if focado && cursor_visivel && c.cursor >= ini && (c.cursor < fim || (c.cursor == fim && (lin + 1 == *linhas || *linhas == 1))) {
                            let cx = w.r.x + 2 + (c.cursor - ini) as i32 * CAR;
                            t.vertical(cx, w.r.y + 2 + lin as i32 * ALTURA_LINHA_CAMPO, 10, PRETO);
                        }
                    }
                }
            }
        }
        t
    }
}

const ALTURA_LINHA_CAMPO: i32 = 11;

/// (rótulo, retângulo, linhas) de cada campo.
fn posicao_campo(id: IdCampo) -> (&'static str, Ret, usize) {
    let procurar = 104;
    let x0 = 12;
    let direita = LARGURA - x0;
    match id {
        IdCampo::Jogo => ("jogo", Ret { x: x0, y: 50, w: direita - x0 - procurar - 8, h: 14 }, 1),
        IdCampo::Save => ("save", Ret { x: x0 + 13 * CAR, y: 130, w: direita - x0 - 13 * CAR - procurar - 8, h: 14 }, 1),
        IdCampo::Janela => ("janela", Ret { x: 452 + 8 * CAR, y: 180, w: direita - 452 - 8 * CAR, h: 14 }, 1),
        IdCampo::SoundFont => ("soundfont", Ret { x: x0 + 13 * CAR, y: 246, w: direita - x0 - 13 * CAR - procurar - 8, h: 14 }, 1),
        IdCampo::Mods => ("mods", Ret { x: 340, y: 280, w: direita - 340 - procurar - 8, h: 14 }, 1),
        IdCampo::WinIni => ("win_ini", Ret { x: 300 + 9 * CAR, y: 352, w: direita - 300 - 9 * CAR - procurar - 8, h: 14 }, 1),
        IdCampo::Linha => ("linha", Ret { x: x0, y: 402, w: direita - x0, h: 6 + 2 * ALTURA_LINHA_CAMPO }, 2),
    }
}

fn botao_procurar(id: IdCampo) -> Option<Ret> {
    let (_, r, _) = posicao_campo(id);
    matches!(id, IdCampo::Jogo | IdCampo::Save | IdCampo::SoundFont | IdCampo::Mods | IdCampo::WinIni)
        .then_some(Ret { x: LARGURA - 12 - 104, y: r.y - 2, w: 104, h: 18 })
}

fn dica_campo(id: IdCampo) -> Option<&'static str> {
    match id {
        IdCampo::Jogo => Some("Ex.: C:\\MAXIS\\ANTWIN (ou arraste a pasta para esta janela)"),
        IdCampo::Save => Some("Arquivo .ANT"),
        IdCampo::Janela => Some("LxA"),
        IdCampo::SoundFont => Some("Arquivo .sf2 (com o sintetizador SoundFont)"),
        IdCampo::Mods => Some("Pasta"),
        IdCampo::WinIni => Some("Automático"),
        IdCampo::Linha => None,
    }
}

fn nome_perfil(p: &str) -> String {
    match p {
        "vga" => "VGA 16 cores".into(),
        "svga256" => "SVGA 256".into(),
        "s3_dosbox" => "S3 (DOSBox-X)".into(),
        outro => outro.into(),
    }
}

/// Perfis de driver de `data/resolucao.json` (sem o arquivo, os três do port).
fn perfis_driver(dados: &Path) -> Vec<String> {
    let lido = std::fs::read_to_string(dados.join("resolucao.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("perfis_driver").and_then(|p| p.as_object()).map(|o| o.keys().cloned().collect::<Vec<_>>()));
    let mut p = lido.unwrap_or_else(|| vec!["vga".into(), "svga256".into(), "s3_dosbox".into()]);
    // A ordem do arquivo não se mantém no mapa do serde_json: vga primeiro, depois por nome.
    p.sort_by_key(|n| (n != "vga", n.clone()));
    p
}

fn encurtar(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        s.to_string()
    } else {
        let resto: String = s.chars().skip(n - (max - 3)).collect();
        format!("...{resto}")
    }
}

fn byte_do_caractere(s: &str, i: usize) -> usize {
    s.char_indices().nth(i).map_or(s.len(), |(b, _)| b)
}

/// Teclas de edição tratadas pelo launcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tecla {
    Esquerda,
    Direita,
    Cima,
    Baixo,
    Inicio,
    Fim,
    Apagar,
    Delete,
    Enter,
    Esc,
    Tab,
}

/// Grava a tela do launcher num PNG, sem abrir janela (documentação e testes).
pub fn capturar(estado: &mut Estado, png: &Path) -> Result<(), String> {
    let t = estado.desenhar(false);
    crate::salvar_png(png, t.largura as u32, t.altura as u32, &t.rgb)
}

/// Abre a janela do launcher e espera a escolha. `aviso`: mensagem para a linha de status (por
/// exemplo, a pasta do jogo da configuração não vale mais).
/// `jogar_apos_ms`: aperta Jogar sozinho depois desse tempo (testes do caminho launcher → jogo).
pub fn executar(estado: &mut Estado, aviso: Option<String>, jogar_apos_ms: Option<u64>) -> Result<Saida, String> {
    use sdl2::event::{Event, WindowEvent};
    use sdl2::keyboard::{Keycode, Mod};
    use sdl2::pixels::PixelFormatEnum;

    if let Some(a) = aviso {
        estado.status = (a, VERMELHO);
    }
    sdl2::hint::set("SDL_WINDOWS_DPI_AWARENESS", "permonitorv2");
    sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "0");
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let area = video.display_usable_bounds(0).ok().map(|r| (r.width(), r.height()));
    let cfg_video = crate::video::ConfigVideo { escala: 0, ..Default::default() };
    let (_, (w, h)) = cfg_video.tamanho_inicial(LARGURA as u32, ALTURA as u32, area);
    // A escala máxima pode ocupar a tela toda; o launcher fica em no máximo 2x.
    let (w, h) = (w.min(LARGURA as u32 * 2), h.min(ALTURA as u32 * 2));
    let janela = video
        .window("SimAnt recomp - opções de inicialização", w, h)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = janela.into_canvas().present_vsync().build().map_err(|e| e.to_string())?;
    canvas.set_logical_size(LARGURA as u32, ALTURA as u32).map_err(|e| e.to_string())?;
    canvas.set_integer_scale(true)?;
    let criador = canvas.texture_creator();
    let mut textura = criador.create_texture_streaming(PixelFormatEnum::RGB24, LARGURA as u32, ALTURA as u32).map_err(|e| e.to_string())?;
    let mut eventos = sdl.event_pump()?;
    video.text_input().start();
    let inicio = std::time::Instant::now();

    loop {
        for ev in eventos.poll_iter() {
            let saida = match ev {
                Event::Quit { .. } => Some(Saida::Sair),
                Event::Window { win_event: WindowEvent::Close, .. } => Some(Saida::Sair),
                Event::MouseButtonDown { x, y, .. } => estado.apertar(x, y),
                Event::MouseButtonUp { x, y, .. } => estado.soltar_botao(x, y),
                Event::TextInput { text, .. } => {
                    estado.digitar(&text);
                    None
                }
                Event::DropFile { filename, .. } => {
                    estado.soltar(Path::new(&filename));
                    None
                }
                Event::KeyDown { keycode: Some(k), keymod, .. } => {
                    let ctrl = keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
                    let t = match k {
                        Keycode::V if ctrl => {
                            if let Ok(s) = video.clipboard().clipboard_text() {
                                estado.digitar(&s.replace(['\r', '\n'], " "));
                            }
                            None
                        }
                        Keycode::C if ctrl => {
                            let _ = video.clipboard().set_clipboard_text(&estado.texto_para_copiar());
                            None
                        }
                        Keycode::Left => Some(Tecla::Esquerda),
                        Keycode::Right => Some(Tecla::Direita),
                        Keycode::Up => Some(Tecla::Cima),
                        Keycode::Down => Some(Tecla::Baixo),
                        Keycode::Home => Some(Tecla::Inicio),
                        Keycode::End => Some(Tecla::Fim),
                        Keycode::Backspace => Some(Tecla::Apagar),
                        Keycode::Delete => Some(Tecla::Delete),
                        Keycode::Return | Keycode::KpEnter => Some(Tecla::Enter),
                        Keycode::Escape => Some(Tecla::Esc),
                        Keycode::Tab => Some(Tecla::Tab),
                        _ => None,
                    };
                    t.and_then(|t| estado.tecla(t))
                }
                _ => None,
            };
            if let Some(s) = saida {
                return Ok(s);
            }
        }
        if jogar_apos_ms.is_some_and(|ms| inicio.elapsed().as_millis() as u64 >= ms) {
            if let Some(s) = estado.acao(Acao::Jogar) {
                return Ok(s);
            }
        }
        let piscar = (inicio.elapsed().as_millis() / 500).is_multiple_of(2);
        let tela = estado.desenhar(piscar);
        textura.update(None, &tela.rgb, LARGURA as usize * 3).map_err(|e| e.to_string())?;
        canvas.clear();
        canvas.copy(&textura, None, None)?;
        canvas.present();
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn estado() -> (Estado, PathBuf) {
        let dir = std::env::temp_dir().join(format!("simant_launcher_ui_{}_{:?}", std::process::id(), std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&dir);
        let jogo = dir.join("ANTWIN");
        std::fs::create_dir_all(&jogo).unwrap();
        for n in cfgl::OBRIGATORIOS {
            std::fs::write(jogo.join(n), b"").unwrap();
        }
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let cfg = ConfigLauncher { jogo: Some(jogo.display().to_string()), ..ConfigLauncher::default() };
        (Estado::novo(Some(cfg), Some(dir.join(cfgl::ARQUIVO)), base), dir)
    }

    fn clicar_em(e: &mut Estado, a: Acao) -> Option<Saida> {
        let w = e.widgets().into_iter().find(|w| matches!(&w.tipo, Tipo::Opcao(_, b, ..) | Tipo::Caixa(_, b, ..) | Tipo::Botao(_, b, _) if *b == a)).expect("widget");
        let (x, y) = (w.r.x + 2, w.r.y + 2);
        e.apertar(x, y).or_else(|| e.soltar_botao(x, y))
    }

    #[test]
    fn cliques_mudam_a_configuracao_e_a_linha() {
        let (mut e, dir) = estado();
        assert!(e.validacao.ok, "{:?}", e.validacao);
        clicar_em(&mut e, Acao::Tipo(1));
        clicar_em(&mut e, Acao::Resolucao(Some(1)));
        clicar_em(&mut e, Acao::Escala(Some(2)));
        clicar_em(&mut e, Acao::Som);
        clicar_em(&mut e, Acao::PastaMods(1));
        let i = e.mods.iter().position(|m| m.nome == "trapacas").expect("mods de exemplo listados");
        clicar_em(&mut e, Acao::Mod(i));
        assert_eq!(e.cfg.inicio, Inicio::NovoJogo);
        let linha = e.campo(IdCampo::Linha).texto.clone();
        for parte in ["--resolucao 800x600", "--escala 2", "--sem-som", "--mods exemplos/mods", "--desligar-mods trapacas", "--novo-jogo --tipo completo"] {
            assert!(linha.contains(parte), "{parte} em {linha}");
        }
        // Jogar grava a configuração e devolve a linha.
        let saida = clicar_em(&mut e, Acao::Jogar);
        assert_eq!(saida, Some(Saida::Jogar(e.cfg.argumentos())));
        assert_eq!(ConfigLauncher::ler(&dir.join(cfgl::ARQUIVO)).unwrap(), Some(e.cfg.clone()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn linha_editada_e_campos_digitados() {
        let (mut e, dir) = estado();
        // Campo da janela: digitado.
        e.acao(Acao::Focar(IdCampo::Janela));
        e.digitar("1300x1000");
        assert_eq!(e.cfg.janela, Some([1300, 1000]));
        // Linha de comando: editada e aplicada com Enter.
        e.acao(Acao::Focar(IdCampo::Linha));
        e.tecla(Tecla::Fim);
        e.digitar(" --moldura --semente 9");
        e.tecla(Tecla::Enter);
        assert_eq!(e.cfg.moldura, Some(true));
        assert_eq!(e.cfg.extras, vec!["--semente".to_string(), "9".into()]);
        assert_eq!(e.foco, None);
        // Linha inválida: fica o texto, com o erro no status.
        e.acao(Acao::Focar(IdCampo::Linha));
        e.digitar(" \"aberta");
        e.tecla(Tecla::Enter);
        assert_eq!(e.foco, Some(IdCampo::Linha));
        assert_eq!(e.status.1, VERMELHO);
        for _ in 0..8 {
            e.tecla(Tecla::Apagar);
        }
        e.tecla(Tecla::Enter);
        assert_eq!(e.foco, None);
        // Pasta do jogo errada: não joga.
        e.acao(Acao::Focar(IdCampo::Jogo));
        e.digitar("x");
        assert!(!e.validacao.ok);
        assert_eq!(e.acao(Acao::Jogar), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn captura_da_tela_sem_janela() {
        let (mut e, dir) = estado();
        let png = dir.join("launcher.png");
        capturar(&mut e, &png).unwrap();
        let dec = png::Decoder::new(std::fs::File::open(&png).unwrap());
        let info = dec.read_info().unwrap();
        assert_eq!((info.info().width, info.info().height), (LARGURA as u32, ALTURA as u32));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
