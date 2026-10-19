//! Saída de texto do executável sem console. No Windows, o `simant.exe` de lançamento é um programa
//! de janela (`windows_subsystem = "windows"`, só no perfil release; ver main.rs): aberto pelo duplo
//! clique, ele não tem console, e o que o port escreve (`[script]` dos mods, avisos, erros) vai para
//! `simant.log` na pasta de dados do usuário. Erros fatais aparecem também numa caixa de mensagem
//! (`SDL_ShowSimpleMessageBox`). Com a saída redirecionada por quem chamou (testes, scripts,
//! `simant.exe > saida.txt`) nada muda; `--console` liga um console (o do terminal que chamou ou um
//! novo). No Linux, o mesmo vale quando o programa é aberto sem opções e sem terminal (gerenciador de
//! arquivos). Decisão em docs/decisoes.md (2026-10-19, "Executável sem console").

use std::path::PathBuf;

pub const ARQUIVO_LOG: &str = "simant.log";
/// O log da execução anterior (o atual é renomeado para ele ao iniciar).
pub const ARQUIVO_LOG_ANTERIOR: &str = "simant-anterior.log";

/// Para onde a saída foi.
#[derive(Clone, Debug, Default)]
pub struct Saida {
    /// A saída e os erros vão para este arquivo (sem console nem redirecionamento).
    pub log: Option<PathBuf>,
}

impl Saida {
    /// Sem terminal para ler os erros: eles aparecem também numa caixa de mensagem.
    pub fn sem_terminal(&self) -> bool {
        self.log.is_some()
    }
}

/// Prepara a saída de texto. `console`: `--console`; `sem_opcoes`: o programa foi aberto sem
/// nenhuma opção (duplo clique).
pub fn preparar(console: bool, sem_opcoes: bool) -> Saida {
    if saida_utilizavel(sem_opcoes) {
        return Saida::default();
    }
    if console && ligar_console() {
        return Saida::default();
    }
    let Some(pasta) = crate::pastas::usuario() else { return Saida::default() };
    let log = pasta.join(ARQUIVO_LOG);
    let _ = std::fs::rename(&log, pasta.join(ARQUIVO_LOG_ANTERIOR));
    match std::fs::File::create(&log).map(redirecionar) {
        Ok(true) => Saida { log: Some(log) },
        _ => Saida::default(),
    }
}

/// Mostra um erro fatal: na saída de erros e, sem terminal, numa caixa de mensagem.
pub fn erro_fatal(saida: &Saida, mensagem: &str) {
    eprintln!("erro: {mensagem}");
    if saida.sem_terminal() {
        let mut texto = mensagem.to_string();
        if mensagem.contains("SHARED") {
            texto += "

A pasta do jogo original não foi encontrada. Abra o simant sem opções (ou com --launcher) para escolhê-la.";
        }
        if let Some(log) = &saida.log {
            texto += &format!("\n\nRegistro da execução: {}", log.display());
        }
        caixa_de_erro(&texto);
    }
}

pub fn caixa_de_erro(texto: &str) {
    use sdl2::messagebox::{show_simple_message_box, MessageBoxFlag};
    let _ = show_simple_message_box(MessageBoxFlag::ERROR, "SimAnt recomp", texto, None);
}

/// Pânico sem terminal: além da mensagem padrão (que vai para o log), a caixa de erro.
pub fn avisar_panico(saida: &Saida) {
    if !saida.sem_terminal() {
        return;
    }
    let log = saida.log.clone();
    let padrao = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        padrao(info);
        let mut texto = format!("Erro interno do SimAnt recomp:\n{info}");
        if let Some(l) = &log {
            texto += &format!("\n\nRegistro da execução: {}", l.display());
        }
        caixa_de_erro(&texto);
    }));
}

#[cfg(windows)]
mod sistema {
    use std::ffi::c_void;
    use std::os::windows::io::IntoRawHandle;

    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(n: u32) -> *mut c_void;
        fn SetStdHandle(n: u32, h: *mut c_void) -> i32;
        fn GetFileType(h: *mut c_void) -> u32;
        fn AttachConsole(processo: u32) -> i32;
        fn AllocConsole() -> i32;
    }

    fn valida(n: u32) -> bool {
        // SAFETY: chamadas sem ponteiros de entrada; o identificador devolvido só é consultado.
        unsafe {
            let h = GetStdHandle(n);
            !h.is_null() && h as isize != -1 && GetFileType(h) != 0
        }
    }

    /// Há para onde escrever: console (programa de console, `cargo run`) ou redirecionamento.
    pub fn saida_utilizavel(_sem_opcoes: bool) -> bool {
        valida(STD_OUTPUT_HANDLE) && valida(STD_ERROR_HANDLE)
    }

    fn trocar_saidas(arquivo: std::fs::File) -> bool {
        let h = arquivo.into_raw_handle();
        // SAFETY: `h` é um identificador de arquivo aberto que fica com o processo até o fim (a
        // saída padrão do Rust consulta GetStdHandle a cada escrita).
        unsafe { SetStdHandle(STD_OUTPUT_HANDLE, h) != 0 && SetStdHandle(STD_ERROR_HANDLE, h) != 0 }
    }

    pub fn ligar_console() -> bool {
        // SAFETY: funções sem argumentos de ponteiro.
        let ok = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) != 0 || AllocConsole() != 0 };
        ok && std::fs::OpenOptions::new().write(true).open("CONOUT$").is_ok_and(trocar_saidas)
    }

    pub fn redirecionar(arquivo: std::fs::File) -> bool {
        trocar_saidas(arquivo)
    }
}

#[cfg(unix)]
mod sistema {
    use std::io::IsTerminal;
    use std::os::unix::io::IntoRawFd;

    extern "C" {
        fn dup2(antigo: i32, novo: i32) -> i32;
    }

    /// Só o programa aberto sem opções e sem terminal (gerenciador de arquivos) troca a saída; com
    /// opções, a saída fica onde quem chamou a pôs (testes e scripts leem por pipes).
    pub fn saida_utilizavel(sem_opcoes: bool) -> bool {
        !sem_opcoes || std::io::stdout().is_terminal() || std::io::stderr().is_terminal()
    }

    pub fn ligar_console() -> bool {
        false
    }

    pub fn redirecionar(arquivo: std::fs::File) -> bool {
        let fd = arquivo.into_raw_fd();
        // SAFETY: `fd` é um arquivo aberto que fica com o processo; dup2 só troca os descritores 1 e 2.
        unsafe { dup2(fd, 1) >= 0 && dup2(fd, 2) >= 0 }
    }
}

#[cfg(not(any(windows, unix)))]
mod sistema {
    pub fn saida_utilizavel(_sem_opcoes: bool) -> bool {
        true
    }
    pub fn ligar_console() -> bool {
        false
    }
    pub fn redirecionar(_arquivo: std::fs::File) -> bool {
        false
    }
}

use sistema::{ligar_console, redirecionar, saida_utilizavel};
