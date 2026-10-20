//! Os idiomas preferidos do sistema, em ordem, para o padrão do launcher e do jogo
//! (`engine::idiomas::escolher`). Windows: `GetUserPreferredUILanguages` (os idiomas da interface do
//! usuário) e, sem eles, `GetUserDefaultLocaleName`. Outros sistemas: `LANGUAGE`, `LC_ALL`,
//! `LC_MESSAGES` e `LANG`. Os códigos vêm como o sistema os dá (`pt-BR`, `pt_BR.UTF-8`);
//! `engine::idiomas::normalizar` os acerta.

pub fn preferidos() -> Vec<String> {
    let mut v = sistema::preferidos();
    v.retain(|s| !s.trim().is_empty());
    v
}

#[cfg(windows)]
mod sistema {
    const MUI_LANGUAGE_NAME: u32 = 0x8;
    const LOCALE_NAME_MAX_LENGTH: usize = 85;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserPreferredUILanguages(flags: u32, quantos: *mut u32, buffer: *mut u16, tamanho: *mut u32) -> i32;
        fn GetUserDefaultLocaleName(nome: *mut u16, tamanho: i32) -> i32;
    }

    pub fn preferidos() -> Vec<String> {
        let mut quantos = 0u32;
        let mut tamanho = 0u32;
        // SAFETY: primeira chamada só pede o tamanho (buffer nulo); a segunda escreve no máximo
        // `tamanho` u16 no vetor desse tamanho.
        let mut v = unsafe {
            if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut quantos, std::ptr::null_mut(), &mut tamanho) != 0 && tamanho > 0 {
                let mut buf = vec![0u16; tamanho as usize];
                if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut quantos, buf.as_mut_ptr(), &mut tamanho) != 0 {
                    // Lista de textos terminados em 0, terminada por um 0 a mais.
                    buf.split(|&c| c == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy).collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        };
        let mut nome = [0u16; LOCALE_NAME_MAX_LENGTH];
        // SAFETY: `nome` tem LOCALE_NAME_MAX_LENGTH posições, o tamanho passado.
        let n = unsafe { GetUserDefaultLocaleName(nome.as_mut_ptr(), nome.len() as i32) };
        if n > 1 {
            v.push(String::from_utf16_lossy(&nome[..n as usize - 1]));
        }
        v
    }
}

#[cfg(not(windows))]
mod sistema {
    pub fn preferidos() -> Vec<String> {
        let mut v = Vec::new();
        // LANGUAGE (GNU) é uma lista separada por ':'; as outras, um código cada.
        if let Ok(l) = std::env::var("LANGUAGE") {
            v.extend(l.split(':').map(str::to_string));
        }
        for n in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(l) = std::env::var(n) {
                v.push(l);
            }
        }
        v
    }
}

#[cfg(test)]
mod testes {
    #[test]
    fn idiomas_do_sistema_nao_quebram() {
        // O resultado depende da máquina; só confere que a consulta funciona e dá códigos legíveis.
        for c in super::preferidos() {
            assert!(c.chars().all(|ch| ch.is_ascii_graphic()), "{c:?}");
        }
    }
}
