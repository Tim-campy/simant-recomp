//! Opções de vídeo do port: tela virtual, escala e tamanho da janela. Vêm de `data/video.json`
//! (lido pelos `Recursos`, então um mod pode trocá-lo) e da linha de comando (`--resolucao`,
//! `--escala`, `--janela`), que vence. Decisões em docs/decisoes.md (2026-10-18, "Escala 1 e opções
//! de vídeo"; "Tela virtual"); o layout de cada resolução, em docs/spec/telas/resolucao.md.

use serde::Deserialize;

pub const ARQUIVO: &str = "video.json";

/// `data/video.json`. Todas as chaves são opcionais.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConfigVideo {
    /// Resolução da tela do Windows 3.1 imitada, `[largura, altura]`: muda o tamanho da área
    /// cliente e a tabela de posições das janelas, como no original (data/resolucao.json). Padrão
    /// 640×480; 800×600 e 1024×768 opcionais.
    #[serde(default = "resolucao_padrao")]
    pub resolucao_virtual: [u32; 2],
    /// Escala inteira inicial (pixels da tela por pixel do jogo). 1 = 640×480 como no original;
    /// 0 = a maior que cabe na área útil do monitor.
    #[serde(default = "escala_padrao")]
    pub escala: u32,
    /// Tamanho inicial da janela em pixels da tela, `[largura, altura]`; `null` = resolução do
    /// jogo × escala. O jogo é centralizado com tarjas pretas.
    #[serde(default)]
    pub janela: Option<[u32; 2]>,
    /// A janela do sistema pode ser redimensionada (e maximizada).
    #[serde(default = "verdadeiro")]
    pub redimensionavel: bool,
    /// Ao redimensionar, ampliar só por múltiplos inteiros (pixels nítidos, tarjas em volta). Com
    /// `false`, ocupa o maior retângulo com a proporção do jogo (pixels de tamanhos desiguais).
    #[serde(default = "verdadeiro")]
    pub escala_inteira: bool,
    /// Desenha a barra de título e o menu de sistema da janela principal do Windows 3.1 em cima da
    /// barra de menus (a janela do port fica com a altura toda da tela imitada). Padrão `true` na 1.0
    /// (escolha do usuário, docs/decisoes.md, 2026-10-19); as exportações e os testes ficam sem ela
    /// sem `--moldura` explícito (`Opcoes::automacao`).
    #[serde(default = "verdadeiro")]
    pub moldura_windows31: bool,
    /// Com `moldura_windows31`, a barra desenhada substitui a moldura do sistema: a janela fica sem
    /// borda, arrasta-se pela faixa do título e, restaurada, ganha a moldura grossa do Windows 3.1
    /// (simant/src/moldura_janela.rs). Padrão `true` (escolha do usuário, docs/decisoes.md,
    /// 2026-10-19); `false` = a barra fica dentro da janela do sistema, como antes.
    #[serde(default = "verdadeiro")]
    pub substituir_moldura_do_sistema: bool,
}

fn resolucao_padrao() -> [u32; 2] {
    [640, 480]
}

fn escala_padrao() -> u32 {
    1
}

fn verdadeiro() -> bool {
    true
}

impl Default for ConfigVideo {
    fn default() -> Self {
        ConfigVideo {
            resolucao_virtual: resolucao_padrao(),
            escala: 1,
            janela: None,
            redimensionavel: true,
            escala_inteira: true,
            moldura_windows31: true,
            substituir_moldura_do_sistema: true,
        }
    }
}

impl ConfigVideo {
    /// Aplica as opções da linha de comando (`--resolucao`, `--escala`, `--janela`), que vencem o arquivo.
    pub fn com_linha_de_comando(mut self, resolucao: Option<(u32, u32)>, escala: Option<u32>, janela: Option<(u32, u32)>) -> ConfigVideo {
        if let Some((l, a)) = resolucao {
            self.resolucao_virtual = [l, a];
        }
        if let Some(e) = escala {
            self.escala = e;
            // Uma escala pedida na linha de comando define também o tamanho da janela.
            self.janela = None;
        }
        if let Some((l, a)) = janela {
            self.janela = Some([l, a]);
        }
        self
    }

    /// Escala e tamanho inicial da janela para a resolução lógica `largura` × `altura`, com a área
    /// útil do monitor (se conhecida). A janela nunca fica menor que a resolução lógica.
    pub fn tamanho_inicial(&self, largura: u32, altura: u32, area_util: Option<(u32, u32)>) -> (u32, (u32, u32)) {
        let cabe = area_util.map(|(w, h)| (w / largura.max(1)).min(h / altura.max(1))).unwrap_or(1).max(1);
        let escala = if self.escala == 0 { cabe } else { self.escala };
        let (w, h) = match self.janela {
            Some([w, h]) => (w, h),
            None => (largura * escala, altura * escala),
        };
        (escala, (w.max(largura), h.max(altura)))
    }
}

/// `--janela LxA` ou `--resolucao LxA` (ex.: `1280x960`); `opcao` é o nome para as mensagens, no
/// idioma de `t`.
pub fn ler_tamanho(opcao: &str, texto: &str, t: &engine::idiomas::Textos) -> Result<(u32, u32), String> {
    let erro = || t.f("cli.erro.tamanho", &[("opcao", &opcao)]);
    let (l, a) = texto.split_once(['x', 'X']).ok_or_else(erro)?;
    let l: u32 = l.trim().parse().map_err(|_| erro())?;
    let a: u32 = a.trim().parse().map_err(|_| erro())?;
    if l == 0 || a == 0 {
        return Err(erro());
    }
    Ok((l, a))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn padrao_e_640_por_480_em_escala_1() {
        let c = ConfigVideo::default();
        assert_eq!(c.tamanho_inicial(640, 480, Some((2560, 1400))), (1, (640, 480)));
        // O arquivo do port traz o mesmo padrão.
        let caminho = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/video.json");
        let arq: ConfigVideo = serde_json::from_str(&std::fs::read_to_string(caminho).unwrap()).unwrap();
        assert_eq!(arq, c);
    }

    #[test]
    fn escala_automatica_e_janela_fixa() {
        let auto = ConfigVideo { escala: 0, ..ConfigVideo::default() };
        assert_eq!(auto.tamanho_inicial(640, 480, Some((2560, 1400))), (2, (1280, 960)));
        assert_eq!(auto.tamanho_inicial(640, 480, Some((800, 600))), (1, (640, 480)), "no mínimo 1");
        assert_eq!(auto.tamanho_inicial(640, 480, None), (1, (640, 480)));
        let fixa = ConfigVideo { janela: Some([1000, 700]), ..ConfigVideo::default() };
        assert_eq!(fixa.tamanho_inicial(640, 480, None), (1, (1000, 700)));
        let pequena = ConfigVideo { janela: Some([320, 200]), ..ConfigVideo::default() };
        assert_eq!(pequena.tamanho_inicial(640, 480, None).1, (640, 480), "nunca menor que o jogo");
    }

    #[test]
    fn linha_de_comando_vence_o_arquivo() {
        let arq: ConfigVideo = serde_json::from_str(r#"{"escala": 3, "janela": [900, 700]}"#).unwrap();
        assert!(arq.redimensionavel && arq.escala_inteira && arq.moldura_windows31, "chaves ausentes ficam no padrão");
        assert!(arq.substituir_moldura_do_sistema, "a moldura do Windows 3.1, se ligada, substitui a do sistema");
        assert_eq!(arq.resolucao_virtual, [640, 480], "tela virtual padrão");
        let c = arq.clone().com_linha_de_comando(None, Some(2), None);
        assert_eq!(c.tamanho_inicial(640, 480, None), (2, (1280, 960)));
        let c = arq.clone().com_linha_de_comando(None, None, Some((800, 600)));
        assert_eq!(c.tamanho_inicial(640, 480, None), (3, (800, 600)));
        let c = arq.com_linha_de_comando(Some((1024, 768)), None, None);
        assert_eq!(c.resolucao_virtual, [1024, 768]);
        let t = engine::idiomas::Textos::embutido();
        assert_eq!(ler_tamanho("--janela", "1280x960", &t), Ok((1280, 960)));
        assert_eq!(ler_tamanho("--janela", "1280", &t), Err("--janela: use WIDTHxHEIGHT with positive numbers (e.g. 800x600)".into()));
        assert!(ler_tamanho("--resolucao", "0x5", &t).is_err());
    }
}
