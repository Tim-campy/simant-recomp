//! Seletores de pasta e de arquivo do sistema, para o launcher. Windows: o diálogo comum do Windows
//! (`IFileOpenDialog`, por COM, sem dependência nova). Linux e outros: `zenity` ou `kdialog`, se
//! instalados; sem eles, o usuário digita o caminho no campo (o launcher avisa).

use std::path::{Path, PathBuf};

/// Filtro de arquivo: nome mostrado e padrões (`*.ANT;*.ant`); `todos`: o nome do filtro "todos os
/// arquivos" (no idioma do launcher).
#[derive(Clone, Copy, Debug)]
pub struct Filtro<'a> {
    pub nome: &'a str,
    pub padroes: &'a [&'a str],
    pub todos: &'a str,
}

/// Erro de [`escolher_pasta`]/[`escolher_arquivo`] quando o sistema não tem seletor (Linux sem
/// zenity nem kdialog); o launcher mostra `launcher.erro.sem_seletor`. Outros erros são técnicos
/// (em inglês).
pub const SEM_SELETOR: &str = "no file chooser";

/// `Ok(None)` = o usuário cancelou.
pub fn escolher_pasta(titulo: &str, inicial: Option<&Path>) -> Result<Option<PathBuf>, String> {
    sistema::escolher(titulo, inicial, None)
}

pub fn escolher_arquivo(titulo: &str, inicial: Option<&Path>, filtro: Filtro) -> Result<Option<PathBuf>, String> {
    sistema::escolher(titulo, inicial, Some(filtro))
}

#[cfg(windows)]
mod sistema {
    use super::Filtro;
    use std::ffi::c_void;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};

    type Hresult = i32;

    #[repr(C)]
    struct Guid(u32, u16, u16, [u8; 8]);

    #[repr(C)]
    struct EspecFiltro {
        nome: *const u16,
        padrao: *const u16,
    }

    const CLSID_FILE_OPEN_DIALOG: Guid = Guid(0xDC1C5A9C, 0xE88A, 0x4DDE, [0xA5, 0xA1, 0x60, 0xF8, 0x2A, 0x20, 0xAE, 0xF7]);
    const IID_IFILE_OPEN_DIALOG: Guid = Guid(0xD57C7288, 0xD4AD, 0x4768, [0xBE, 0x02, 0x9D, 0x96, 0x95, 0x32, 0xD9, 0x60]);
    const IID_ISHELL_ITEM: Guid = Guid(0x43826D1E, 0xE718, 0x42EE, [0xBC, 0x55, 0xA1, 0xE2, 0x61, 0xC3, 0x7B, 0xFE]);
    const CLSCTX_INPROC_SERVER: u32 = 1;
    const COINIT_APARTMENTTHREADED: u32 = 0x2;
    const COINIT_DISABLE_OLE1DDE: u32 = 0x4;
    const RPC_E_CHANGED_MODE: Hresult = 0x8001_0106_u32 as i32;
    const CANCELADO: Hresult = 0x8007_04C7_u32 as i32;
    const FOS_PICKFOLDERS: u32 = 0x20;
    const FOS_FORCEFILESYSTEM: u32 = 0x40;
    const FOS_PATHMUSTEXIST: u32 = 0x800;
    const FOS_FILEMUSTEXIST: u32 = 0x1000;
    const SIGDN_FILESYSPATH: u32 = 0x8005_8000;

    // Posições na tabela de métodos (IUnknown → IModalWindow → IFileDialog; IShellItem).
    const RELEASE: usize = 2;
    const SHOW: usize = 3;
    const SET_FILE_TYPES: usize = 4;
    const SET_OPTIONS: usize = 9;
    const GET_OPTIONS: usize = 10;
    const SET_FOLDER: usize = 12;
    const SET_FILE_NAME: usize = 15;
    const SET_TITLE: usize = 17;
    const GET_RESULT: usize = 20;
    const GET_DISPLAY_NAME: usize = 5;

    #[link(name = "ole32")]
    extern "system" {
        fn CoInitializeEx(reservado: *mut c_void, modo: u32) -> Hresult;
        fn CoUninitialize();
        fn CoCreateInstance(clsid: *const Guid, externo: *mut c_void, contexto: u32, iid: *const Guid, saida: *mut *mut c_void) -> Hresult;
        fn CoTaskMemFree(p: *mut c_void);
    }

    #[link(name = "shell32")]
    extern "system" {
        fn SHCreateItemFromParsingName(caminho: *const u16, contexto: *mut c_void, iid: *const Guid, saida: *mut *mut c_void) -> Hresult;
    }

    fn largo(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Endereço do método `i` do objeto COM `obj`.
    ///
    /// SAFETY: `obj` precisa ser um objeto COM válido com pelo menos `i + 1` métodos.
    unsafe fn metodo(obj: *mut c_void, i: usize) -> *const c_void {
        let tabela = *(obj as *const *const *const c_void);
        *tabela.add(i)
    }

    /// Objeto COM liberado ao sair do escopo.
    struct Com(*mut c_void);

    impl Drop for Com {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: objeto válido obtido de CoCreateInstance/GetResult/SHCreateItem...
                unsafe {
                    let release: extern "system" fn(*mut c_void) -> u32 = std::mem::transmute(metodo(self.0, RELEASE));
                    release(self.0);
                }
            }
        }
    }

    fn verificar(hr: Hresult, o_que: &str) -> Result<(), String> {
        if hr < 0 {
            Err(format!("Windows dialog: {o_que} failed (0x{:08X})", hr as u32))
        } else {
            Ok(())
        }
    }

    pub fn escolher(titulo: &str, inicial: Option<&Path>, filtro: Option<Filtro>) -> Result<Option<PathBuf>, String> {
        // SAFETY: inicialização do COM nesta thread, desfeita no fim se foi feita aqui.
        let hr = unsafe { CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        if hr < 0 && hr != RPC_E_CHANGED_MODE {
            return Err(format!("Windows dialog: CoInitializeEx failed (0x{:08X})", hr as u32));
        }
        let r = mostrar(titulo, inicial, filtro, None);
        if hr >= 0 {
            // SAFETY: par do CoInitializeEx bem-sucedido acima.
            unsafe { CoUninitialize() };
        }
        r
    }

    /// O diálogo; `nome` preenche o campo do nome (usado pelo teste para escolher sem o mouse).
    pub(super) fn mostrar(titulo: &str, inicial: Option<&Path>, filtro: Option<Filtro>, nome: Option<&str>) -> Result<Option<PathBuf>, String> {
        // SAFETY: as chamadas seguem a interface IFileOpenDialog documentada; todo ponteiro passado
        // vive até o fim da chamada (as cadeias de texto ficam em variáveis deste escopo).
        unsafe {
            let mut p: *mut c_void = std::ptr::null_mut();
            verificar(CoCreateInstance(&CLSID_FILE_OPEN_DIALOG, std::ptr::null_mut(), CLSCTX_INPROC_SERVER, &IID_IFILE_OPEN_DIALOG, &mut p), "CoCreateInstance")?;
            let dialogo = Com(p);
            let d = dialogo.0;

            let get_options: extern "system" fn(*mut c_void, *mut u32) -> Hresult = std::mem::transmute(metodo(d, GET_OPTIONS));
            let set_options: extern "system" fn(*mut c_void, u32) -> Hresult = std::mem::transmute(metodo(d, SET_OPTIONS));
            let mut opcoes = 0u32;
            verificar(get_options(d, &mut opcoes), "GetOptions")?;
            opcoes |= FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST;
            opcoes |= if filtro.is_some() { FOS_FILEMUSTEXIST } else { FOS_PICKFOLDERS };
            verificar(set_options(d, opcoes), "SetOptions")?;

            let t = largo(titulo);
            let set_title: extern "system" fn(*mut c_void, *const u16) -> Hresult = std::mem::transmute(metodo(d, SET_TITLE));
            verificar(set_title(d, t.as_ptr()), "SetTitle")?;

            // Filtro de arquivos: o pedido e "todos os arquivos".
            let (fnome, fpadrao, tnome, tpadrao) = match filtro {
                Some(f) => (largo(f.nome), largo(&f.padroes.join(";")), largo(f.todos), largo("*.*")),
                None => (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            };
            if filtro.is_some() {
                let espec = [EspecFiltro { nome: fnome.as_ptr(), padrao: fpadrao.as_ptr() }, EspecFiltro { nome: tnome.as_ptr(), padrao: tpadrao.as_ptr() }];
                let set_file_types: extern "system" fn(*mut c_void, u32, *const EspecFiltro) -> Hresult = std::mem::transmute(metodo(d, SET_FILE_TYPES));
                verificar(set_file_types(d, espec.len() as u32, espec.as_ptr()), "SetFileTypes")?;
            }

            // Pasta inicial: a do valor atual (ou a pasta de um arquivo), se existir.
            let pasta = inicial.and_then(|p| if p.is_dir() { Some(p.to_path_buf()) } else { p.parent().filter(|q| q.is_dir()).map(Path::to_path_buf) });
            if let Some(pasta) = pasta.and_then(|p| std::path::absolute(p).ok()) {
                let w: Vec<u16> = pasta.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
                let mut item: *mut c_void = std::ptr::null_mut();
                if SHCreateItemFromParsingName(w.as_ptr(), std::ptr::null_mut(), &IID_ISHELL_ITEM, &mut item) >= 0 {
                    let item = Com(item);
                    let set_folder: extern "system" fn(*mut c_void, *mut c_void) -> Hresult = std::mem::transmute(metodo(d, SET_FOLDER));
                    let _ = set_folder(d, item.0);
                }
            }
            let n = nome.map(largo);
            if let Some(n) = &n {
                let set_file_name: extern "system" fn(*mut c_void, *const u16) -> Hresult = std::mem::transmute(metodo(d, SET_FILE_NAME));
                verificar(set_file_name(d, n.as_ptr()), "SetFileName")?;
            }

            let show: extern "system" fn(*mut c_void, *mut c_void) -> Hresult = std::mem::transmute(metodo(d, SHOW));
            let hr = show(d, std::ptr::null_mut());
            if hr == CANCELADO {
                return Ok(None);
            }
            verificar(hr, "Show")?;

            let get_result: extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult = std::mem::transmute(metodo(d, GET_RESULT));
            let mut item: *mut c_void = std::ptr::null_mut();
            verificar(get_result(d, &mut item), "GetResult")?;
            let item = Com(item);
            let get_display_name: extern "system" fn(*mut c_void, u32, *mut *mut u16) -> Hresult = std::mem::transmute(metodo(item.0, GET_DISPLAY_NAME));
            let mut texto: *mut u16 = std::ptr::null_mut();
            verificar(get_display_name(item.0, SIGDN_FILESYSPATH, &mut texto), "GetDisplayName")?;
            let mut n = 0;
            while *texto.add(n) != 0 {
                n += 1;
            }
            let caminho = std::ffi::OsString::from_wide(std::slice::from_raw_parts(texto, n));
            CoTaskMemFree(texto as *mut c_void);
            Ok(Some(PathBuf::from(caminho)))
        }
    }

    /// Confere o diálogo de verdade sem mexer no mouse nem no teclado do usuário: abre com o nome já
    /// preenchido e manda, pela fila de mensagens da própria janela do diálogo, o "OK" (ou o fechar).
    /// Abre uma janela por cerca de um segundo; por isso fica fora da execução normal dos testes
    /// (`cargo test -p simant -- --ignored dialogo_do_windows`).
    #[cfg(test)]
    mod testes {
        use super::*;

        #[link(name = "user32")]
        extern "system" {
            fn FindWindowW(classe: *const u16, titulo: *const u16) -> *mut c_void;
            fn PostMessageW(janela: *mut c_void, msg: u32, w: usize, l: isize) -> i32;
        }
        const WM_COMMAND: u32 = 0x0111;
        const WM_CLOSE: u32 = 0x0010;

        /// Espera o diálogo de título `titulo` aparecer e manda `msg` para ele.
        fn responder(titulo: &'static str, msg: u32, w: usize) -> std::thread::JoinHandle<bool> {
            std::thread::spawn(move || {
                let t = largo(titulo);
                for _ in 0..100 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    // SAFETY: busca por título; a janela é a do diálogo deste processo.
                    let j = unsafe { FindWindowW(std::ptr::null(), t.as_ptr()) };
                    if !j.is_null() {
                        std::thread::sleep(std::time::Duration::from_millis(400));
                        // SAFETY: mensagem postada para a janela achada acima.
                        return unsafe { PostMessageW(j, msg, w, 0) != 0 };
                    }
                }
                false
            })
        }

        #[test]
        #[ignore = "abre o diálogo do Windows por cerca de um segundo"]
        fn dialogo_do_windows_escolhe_pasta_e_arquivo_e_cancela() {
            // SAFETY: COM nesta thread de teste.
            unsafe { CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED) };
            let dir = std::env::temp_dir().join(format!("simant_dialogo_{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("TESTE.ANT"), b"x").unwrap();

            let r = responder("SimAnt teste pasta", WM_COMMAND, 1);
            let p = mostrar("SimAnt teste pasta", Some(&dir), None, Some(dir.to_str().unwrap())).unwrap();
            assert!(r.join().unwrap());
            assert_eq!(p.map(|p| p.canonicalize().unwrap()), Some(dir.canonicalize().unwrap()));

            let r = responder("SimAnt teste arquivo", WM_COMMAND, 1);
            let filtro = Filtro { nome: "SimAnt games (*.ANT)", padroes: &["*.ANT"], todos: "All files" };
            let p = mostrar("SimAnt teste arquivo", Some(&dir), Some(filtro), Some("TESTE.ANT")).unwrap();
            assert!(r.join().unwrap());
            assert_eq!(p.map(|p| p.canonicalize().unwrap()), Some(dir.join("TESTE.ANT").canonicalize().unwrap()));

            let r = responder("SimAnt teste cancelar", WM_CLOSE, 0);
            assert_eq!(mostrar("SimAnt teste cancelar", None, None, None).unwrap(), None);
            assert!(r.join().unwrap());
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

#[cfg(not(windows))]
mod sistema {
    use super::Filtro;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    pub fn escolher(titulo: &str, inicial: Option<&Path>, filtro: Option<Filtro>) -> Result<Option<PathBuf>, String> {
        let inicial = inicial.map(|p| p.display().to_string()).unwrap_or_default();
        // zenity (GNOME e a maioria das distribuições) e kdialog (KDE).
        let mut z = Command::new("zenity");
        z.arg("--file-selection").arg(format!("--title={titulo}"));
        if !inicial.is_empty() {
            z.arg(format!("--filename={inicial}/"));
        }
        match filtro {
            Some(f) => {
                let padroes: Vec<String> = f.padroes.iter().flat_map(|p| [p.to_string(), p.to_lowercase()]).collect();
                z.arg(format!("--file-filter={} | {}", f.nome, padroes.join(" ")));
            }
            None => {
                z.arg("--directory");
            }
        }
        let mut k = Command::new("kdialog");
        match filtro {
            Some(f) => {
                k.arg("--getopenfilename").arg(if inicial.is_empty() { ".".into() } else { inicial.clone() }).arg(format!("{} ({})", f.nome, f.padroes.join(" ")));
            }
            None => {
                k.arg("--getexistingdirectory").arg(if inicial.is_empty() { ".".into() } else { inicial.clone() });
            }
        }
        k.arg("--title").arg(titulo);
        for mut c in [z, k] {
            match c.output() {
                Ok(s) if s.status.success() => {
                    let t = String::from_utf8_lossy(&s.stdout).trim_end_matches(['\n', '\r']).to_string();
                    return Ok((!t.is_empty()).then(|| PathBuf::from(t)));
                }
                // Código 1 = cancelado (nos dois programas).
                Ok(s) if s.status.code() == Some(1) => return Ok(None),
                Ok(_) | Err(_) => continue,
            }
        }
        Err(super::SEM_SELETOR.into())
    }
}
