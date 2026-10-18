//! Opções de vídeo do port: escala e tamanho da janela. Vêm de `data/video.json` (lido pelos
//! `Recursos`, então um mod pode trocá-lo) e da linha de comando (`--escala`, `--janela`), que vence.
//! Decisão em docs/decisoes.md (2026-10-18, "Escala 1 e opções de vídeo").

use serde::Deserialize;

pub const ARQUIVO: &str = "video.json";

/// `data/video.json`. Todas as chaves são opcionais.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConfigVideo {
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
}

fn escala_padrao() -> u32 {
    1
}

fn verdadeiro() -> bool {
    true
}

impl Default for ConfigVideo {
    fn default() -> Self {
        ConfigVideo { escala: 1, janela: None, redimensionavel: true, escala_inteira: true }
    }
}

impl ConfigVideo {
    /// Aplica as opções da linha de comando (`--escala`, `--janela`), que vencem o arquivo.
    pub fn com_linha_de_comando(mut self, escala: Option<u32>, janela: Option<(u32, u32)>) -> ConfigVideo {
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

/// `--janela LxA` (ex.: `1280x960`).
pub fn ler_tamanho(texto: &str) -> Result<(u32, u32), String> {
    let (l, a) = texto.split_once(['x', 'X']).ok_or("--janela LARGURAxALTURA (ex.: 1280x960)")?;
    let l: u32 = l.trim().parse().map_err(|_| "--janela: largura inválida")?;
    let a: u32 = a.trim().parse().map_err(|_| "--janela: altura inválida")?;
    if l == 0 || a == 0 {
        return Err("--janela: largura e altura positivas".into());
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
        assert!(arq.redimensionavel && arq.escala_inteira, "chaves ausentes ficam no padrão");
        let c = arq.clone().com_linha_de_comando(Some(2), None);
        assert_eq!(c.tamanho_inicial(640, 480, None), (2, (1280, 960)));
        let c = arq.com_linha_de_comando(None, Some((800, 600)));
        assert_eq!(c.tamanho_inicial(640, 480, None), (3, (800, 600)));
        assert_eq!(ler_tamanho("1280x960"), Ok((1280, 960)));
        assert!(ler_tamanho("1280").is_err());
        assert!(ler_tamanho("0x5").is_err());
    }
}
