//! Controles do Windows 3.1 imitados nos diálogos do port: o campo de texto (`EDIT`) com cursor e
//! seleção. Sem desenho e sem SDL: o diálogo que o usa desenha e repassa a entrada.
//!
//! Especificação: docs/spec/telas/arquivos.md §3.1 (bancada 2026-10-19, capturas
//! `re_trabalho/ag_dlgaj/dlg/g13`–`g18`).

/// Campo de texto de uma linha (`EDIT` com `ES_AUTOHSCROLL`). O texto é ASCII (nomes de arquivo);
/// `cursor` e `ancora` são posições entre caracteres (0 = antes do primeiro): a seleção vai de uma
/// à outra.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CampoTexto {
    pub texto: String,
    pub cursor: usize,
    pub ancora: usize,
    /// Máximo de caracteres (o `nMaxFile` do diálogo comum).
    pub limite: usize,
}

/// O caractere conta como parte de uma palavra (duplo clique, Ctrl+setas).
fn de_palavra(c: u8) -> bool {
    c != b' '
}

impl CampoTexto {
    pub fn new(texto: &str, limite: usize) -> CampoTexto {
        let mut c = CampoTexto { texto: String::new(), cursor: 0, ancora: 0, limite };
        c.definir(texto, true);
        c
    }

    /// Troca o texto; `selecionado`: todo selecionado (o cursor no fim), como o `EM_SETSEL` que o
    /// diálogo comum faz ao abrir e ao trocar de pasta (g00: `*.ant` selecionado).
    pub fn definir(&mut self, texto: &str, selecionado: bool) {
        self.texto = texto.chars().filter(|c| c.is_ascii() && !c.is_ascii_control()).take(self.limite.max(1)).collect();
        self.cursor = self.texto.len();
        self.ancora = if selecionado { 0 } else { self.cursor };
    }

    /// A seleção (início, fim), com início ≤ fim.
    pub fn selecao(&self) -> (usize, usize) {
        (self.cursor.min(self.ancora), self.cursor.max(self.ancora))
    }

    pub fn tem_selecao(&self) -> bool {
        self.cursor != self.ancora
    }

    pub fn selecionar_tudo(&mut self) {
        self.ancora = 0;
        self.cursor = self.texto.len();
    }

    /// Leva o cursor a `pos`; `estender`: a âncora fica (Shift, arrasto).
    pub fn mover(&mut self, pos: usize, estender: bool) {
        self.cursor = pos.min(self.texto.len());
        if !estender {
            self.ancora = self.cursor;
        }
    }

    fn apagar_selecao(&mut self) {
        let (a, b) = self.selecao();
        self.texto.replace_range(a..b, "");
        self.cursor = a;
        self.ancora = a;
    }

    /// Texto digitado: troca a seleção (se houver) e entra no cursor, até o limite.
    pub fn inserir(&mut self, s: &str) {
        let novo: String = s.chars().filter(|c| c.is_ascii() && !c.is_ascii_control()).collect();
        if novo.is_empty() {
            return;
        }
        self.apagar_selecao();
        let cabe = self.limite.saturating_sub(self.texto.len());
        let novo = &novo[..novo.len().min(cabe)];
        self.texto.insert_str(self.cursor, novo);
        self.cursor += novo.len();
        self.ancora = self.cursor;
    }

    /// Início da palavra antes de `i` (Ctrl+←): pula os espaços e depois a palavra.
    fn palavra_antes(&self, i: usize) -> usize {
        let b = self.texto.as_bytes();
        let mut k = i;
        while k > 0 && !de_palavra(b[k - 1]) {
            k -= 1;
        }
        while k > 0 && de_palavra(b[k - 1]) {
            k -= 1;
        }
        k
    }

    /// Início da palavra seguinte depois de `i` (Ctrl+→): a palavra e os espaços dela.
    fn palavra_depois(&self, i: usize) -> usize {
        let b = self.texto.as_bytes();
        let mut k = i;
        while k < b.len() && de_palavra(b[k]) {
            k += 1;
        }
        while k < b.len() && !de_palavra(b[k]) {
            k += 1;
        }
        k
    }

    /// A palavra sob a posição `i` com os espaços que a seguem (o duplo clique do Windows 3.1
    /// selecionou "efgh " em "abcd efgh ijkl", bancada g15).
    pub fn palavra_em(&self, i: usize) -> (usize, usize) {
        let b = self.texto.as_bytes();
        let i = i.min(b.len());
        if b.is_empty() {
            return (0, 0);
        }
        // Clique num espaço: o bloco de espaços (com a palavra antes dele).
        let mut ini = i.min(b.len() - 1);
        if !de_palavra(b[ini]) && ini > 0 && de_palavra(b[ini - 1]) {
            ini -= 1;
        }
        if de_palavra(b[ini]) {
            while ini > 0 && de_palavra(b[ini - 1]) {
                ini -= 1;
            }
        }
        (ini, self.palavra_depois(ini))
    }

    /// Duplo clique na posição `i`.
    pub fn selecionar_palavra(&mut self, i: usize) {
        let (a, b) = self.palavra_em(i);
        self.ancora = a;
        self.cursor = b;
    }

    /// Tecla de edição (nome do SDL). Devolve se a tecla era do campo. Ctrl+A seleciona tudo: no
    /// Windows 3.1 o `EDIT` não tem esse atalho (g17) — extensão do port.
    pub fn tecla(&mut self, nome: &str, shift: bool, ctrl: bool) -> bool {
        let n = self.texto.len();
        match nome {
            "Left" => {
                let alvo = if ctrl {
                    self.palavra_antes(self.cursor)
                } else if self.tem_selecao() && !shift {
                    self.selecao().0
                } else {
                    self.cursor.saturating_sub(1)
                };
                self.mover(alvo, shift);
            }
            "Right" => {
                let alvo = if ctrl {
                    self.palavra_depois(self.cursor)
                } else if self.tem_selecao() && !shift {
                    self.selecao().1
                } else {
                    (self.cursor + 1).min(n)
                };
                self.mover(alvo, shift);
            }
            "Home" => self.mover(0, shift),
            "End" => self.mover(n, shift),
            "Backspace" => {
                if self.tem_selecao() {
                    self.apagar_selecao();
                } else if self.cursor > 0 {
                    self.texto.remove(self.cursor - 1);
                    self.mover(self.cursor - 1, false);
                }
            }
            "Delete" => {
                if self.tem_selecao() {
                    self.apagar_selecao();
                } else if self.cursor < n {
                    self.texto.remove(self.cursor);
                }
            }
            "A" | "a" if ctrl => self.selecionar_tudo(),
            _ => return false,
        }
        true
    }

    /// Posição entre caracteres mais perto de `dx` pixels do começo do texto, com `largura(prefixo)`
    /// dando a largura de um pedaço do texto.
    pub fn posicao_em(&self, dx: i32, largura: impl Fn(&str) -> i32) -> usize {
        let mut antes = 0;
        for i in 1..=self.texto.len() {
            let w = largura(&self.texto[..i]);
            if dx < (antes + w) / 2 + (antes + w) % 2 {
                return i - 1;
            }
            antes = w;
        }
        self.texto.len()
    }

    /// Primeiro caractere visível num campo de `visivel` pixels, para o cursor ficar à vista
    /// (`ES_AUTOHSCROLL`) [hipótese: o Windows rola de um terço do campo; o port só o necessário].
    pub fn primeiro_visivel(&self, visivel: i32, largura: impl Fn(&str) -> i32) -> usize {
        let mut ini = 0;
        while ini < self.cursor && largura(&self.texto[ini..self.cursor]) > visivel {
            ini += 1;
        }
        ini
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn larg(s: &str) -> i32 {
        7 * s.len() as i32
    }

    /// Bancada g13–g18: digitar troca a seleção; duplo clique seleciona a palavra com o espaço;
    /// Shift+← estende; Home e Shift+End selecionam tudo.
    #[test]
    fn edicao_e_selecao_como_o_edit_do_windows() {
        let mut c = CampoTexto::new("*.ant", 64);
        assert_eq!(c.selecao(), (0, 5), "o texto inicial vem selecionado");
        c.inserir("abcd efgh ijkl");
        assert_eq!((c.texto.as_str(), c.cursor, c.tem_selecao()), ("abcd efgh ijkl", 14, false));
        c.selecionar_palavra(7);
        assert_eq!(c.selecao(), (5, 10), "\"efgh \"");
        c.tecla("End", false, false);
        for _ in 0..3 {
            c.tecla("Left", true, false);
        }
        assert_eq!(c.selecao(), (11, 14), "\"jkl\"");
        c.tecla("Home", false, false);
        c.tecla("End", true, false);
        assert_eq!(c.selecao(), (0, 14));
        c.inserir("x");
        assert_eq!(c.texto, "x");
    }

    #[test]
    fn apagar_palavras_e_ctrl_a() {
        let mut c = CampoTexto::new("um dois tres", 64);
        c.mover(12, false);
        c.tecla("Left", false, true);
        assert_eq!(c.cursor, 8);
        c.tecla("Left", true, true);
        assert_eq!(c.selecao(), (3, 8));
        c.tecla("Delete", false, false);
        assert_eq!(c.texto, "um tres");
        c.tecla("Backspace", false, false);
        assert_eq!(c.texto, "umtres");
        c.tecla("Right", false, true);
        assert_eq!(c.cursor, 6);
        assert!(c.tecla("A", false, true));
        assert_eq!(c.selecao(), (0, 6));
        // Sem seleção, ← com seleção leva ao começo dela.
        c.tecla("Left", false, false);
        assert_eq!((c.cursor, c.tem_selecao()), (0, false));
    }

    #[test]
    fn clique_arrasto_e_rolagem_do_campo() {
        let mut c = CampoTexto::new("abcdef", 8);
        assert_eq!(c.posicao_em(0, larg), 0);
        assert_eq!(c.posicao_em(3, larg), 0);
        assert_eq!(c.posicao_em(4, larg), 1);
        assert_eq!(c.posicao_em(100, larg), 6);
        c.mover(c.posicao_em(8, larg), false);
        c.mover(c.posicao_em(30, larg), true);
        assert_eq!(c.selecao(), (1, 4));
        c.inserir("XYZWV");
        assert_eq!(c.texto.len(), 8, "o limite corta o que não cabe");
        c.mover(8, false);
        assert_eq!(c.primeiro_visivel(21, larg), 5);
        assert_eq!(c.primeiro_visivel(100, larg), 0);
    }
}
