//! Tela virtual: o layout que o original escolhe para a resolução do Windows (640×480, 800×600,
//! 1024×768). Especificação: docs/spec/telas/resolucao.md. Regra em data/resolucao.json.
//!
//! O original não escala nada: muda só o tamanho da área cliente da janela principal (maximizada)
//! e a tabela de posições das janelas (objeto tipo 9), escolhida pela altura da tela.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::tela_jogo::ConfigTelaJogo;
use super::telas::Sequencia;

pub const ARQUIVO: &str = "resolucao.json";

/// Uma faixa da escolha da tabela de posições (`_win_LoadAllWindows`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct FaixaTabela {
    /// Vale para telas com até tantas linhas; `None` = qualquer altura (a última da lista).
    #[serde(default)]
    pub ate_altura: Option<u32>,
    /// Id do objeto tipo 9.
    pub tabela: u16,
}

/// `data/resolucao.json`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConfigResolucao {
    /// Linhas da tela acima da área cliente da janela principal maximizada (barra de título e de
    /// menus): 38, medidas em 640×480, 800×600 e 1024×768.
    pub linhas_acima_da_area_cliente: u32,
    /// Em ordem: a primeira faixa cuja `ate_altura` comporta a altura da tela dá a tabela.
    pub tabelas_posicoes: Vec<FaixaTabela>,
    /// Resoluções aceitas para a tela virtual.
    pub resolucoes: Vec<[u32; 2]>,
    /// Perfil do driver de vídeo imitado (chave de `perfis_driver`); sem ele, o padrão do port.
    #[serde(default)]
    pub perfil_driver: Option<String>,
    /// O que muda com o driver de vídeo do Windows, não com a resolução (telas/resolucao.md §7).
    #[serde(default)]
    pub perfis_driver: BTreeMap<String, PerfilDriver>,
}

/// Regras do Windows 3.1 que dependem do driver de vídeo (docs/spec/telas/resolucao.md §7).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PerfilDriver {
    /// Pixels por byte da memória de vídeo: o Windows alinha a área cliente das janelas com
    /// `CS_BYTEALIGNCLIENT` (a classe `GenericWindow` do jogo) e o retângulo das caixas de diálogo
    /// (`CS_BYTEALIGNWINDOW`) ao byte mais próximo em x. 8 no VGA de 16 cores (planar, 1 bit por
    /// pixel em cada plano); 1 em 256 cores (1 byte por pixel), ou seja, sem alinhamento.
    pub alinhamento_x: i16,
    /// Pixels a mais em x na origem do conteúdo das janelas com contorno (objeto 0 tipo 15): 1 no
    /// S3 Trio do DOSBox-X, 0 no VGA e no vbesvga [hipótese: recorte do driver com x negativo].
    #[serde(default)]
    pub ajuste_x_contorno: i16,
    /// Cores do sistema que o driver traz como padrão (recurso OEMBIN 1), sem `[colors]` no
    /// `WIN.INI`: índice da paleta do port → "#rrggbb" (16 = título ativo e moldura de diálogo,
    /// 17 = fundo da área de trabalho da aplicação).
    #[serde(default)]
    pub cores_sistema: BTreeMap<u8, String>,
}

impl Default for PerfilDriver {
    /// Sem alinhamento e sem trocar cores.
    fn default() -> Self {
        PerfilDriver { alinhamento_x: 1, ajuste_x_contorno: 0, cores_sistema: BTreeMap::new() }
    }
}

impl PerfilDriver {
    /// `x` arredondado ao múltiplo de `alinhamento_x` mais próximo (o meio vai para cima), como o
    /// Windows faz com a área cliente (`CS_BYTEALIGNCLIENT`) e com as caixas de diálogo.
    pub fn alinhar(&self, x: i16) -> i16 {
        let n = self.alinhamento_x.max(1);
        (x + n / 2).div_euclid(n) * n
    }

    /// Canto x da janela cuja área cliente começa `margem` pixels à direita dele, alinhada.
    pub fn alinhar_janela(&self, x: i16, margem: i16) -> i16 {
        self.alinhar(x + margem) - margem
    }

    /// A paleta do port (`rgb`) com as cores do sistema deste driver.
    pub fn aplicar_cores(&self, rgb: &mut [[u8; 3]]) -> Result<(), String> {
        for (&i, c) in &self.cores_sistema {
            let h = c.strip_prefix('#').filter(|h| h.len() == 6).ok_or(format!("{ARQUIVO}: cor inválida {c}"))?;
            let v = u32::from_str_radix(h, 16).map_err(|_| format!("{ARQUIVO}: cor inválida {c}"))?;
            let alvo = rgb.get_mut(i as usize).ok_or(format!("{ARQUIVO}: índice {i} fora da paleta"))?;
            *alvo = [(v >> 16) as u8, (v >> 8) as u8, v as u8];
        }
        Ok(())
    }
}

/// Layout calculado para uma resolução.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TelaVirtual {
    /// Resolução da tela do Windows imitada.
    pub resolucao: [u32; 2],
    /// Área cliente da janela principal (faixa + raiz), em pixels.
    pub largura: u32,
    pub altura: u32,
    /// Id do objeto tipo 9 com as posições das janelas.
    pub tabela_posicoes: u16,
}

impl ConfigResolucao {
    /// O perfil de driver `nome` (ou o de `perfil_driver`); sem nenhum, o padrão (sem alinhamento).
    pub fn perfil(&self, nome: Option<&str>) -> Result<PerfilDriver, String> {
        match nome.or(self.perfil_driver.as_deref()) {
            None => Ok(PerfilDriver::default()),
            Some(n) => self.perfis_driver.get(n).cloned().ok_or(format!("{ARQUIVO}: perfil de driver '{n}' não existe")),
        }
    }

    /// Layout para a tela `[largura, altura]`; erro se a resolução não é uma das aceitas.
    pub fn tela(&self, [l, a]: [u32; 2]) -> Result<TelaVirtual, String> {
        if !self.resolucoes.contains(&[l, a]) {
            let aceitas: Vec<String> = self.resolucoes.iter().map(|[l, a]| format!("{l}x{a}")).collect();
            return Err(format!("resolução virtual {l}x{a} não suportada (aceitas: {})", aceitas.join(", ")));
        }
        let altura = a.checked_sub(self.linhas_acima_da_area_cliente).filter(|&h| h > 0).ok_or("resolução virtual baixa demais")?;
        let tabela = self
            .tabelas_posicoes
            .iter()
            .find(|f| f.ate_altura.is_none_or(|m| a <= m))
            .map(|f| f.tabela)
            .ok_or(format!("{ARQUIVO}: nenhuma tabela de posições para {a} linhas"))?;
        Ok(TelaVirtual { resolucao: [l, a], largura: l, altura, tabela_posicoes: tabela })
    }
}

impl TelaVirtual {
    /// A tela de jogo (`data/tela_jogo.json`) nesta resolução.
    pub fn aplicar_tela_jogo(&self, cfg: &mut ConfigTelaJogo) {
        cfg.largura = self.largura;
        cfg.altura = self.altura;
        cfg.tabela_posicoes = self.tabela_posicoes;
    }

    /// As telas da abertura (`data/telas.json`) nesta resolução.
    pub fn aplicar_sequencia(&self, seq: &mut Sequencia) {
        for t in &mut seq.telas {
            t.largura = self.largura;
            t.altura = self.altura;
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn config() -> ConfigResolucao {
        let caminho = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data").join(ARQUIVO);
        serde_json::from_str(&std::fs::read_to_string(caminho).unwrap()).unwrap()
    }

    #[test]
    fn regra_do_original_para_as_tres_resolucoes() {
        let c = config();
        // _win_LoadAllWindows: até 480 linhas, 9/0; mais, 9/8 (a largura não entra na conta).
        assert_eq!(c.tela([640, 480]).unwrap(), TelaVirtual { resolucao: [640, 480], largura: 640, altura: 442, tabela_posicoes: 0 });
        assert_eq!(c.tela([800, 600]).unwrap(), TelaVirtual { resolucao: [800, 600], largura: 800, altura: 562, tabela_posicoes: 8 });
        assert_eq!(c.tela([1024, 768]).unwrap(), TelaVirtual { resolucao: [1024, 768], largura: 1024, altura: 730, tabela_posicoes: 8 });
        assert!(c.tela([1280, 1024]).is_err());
        assert!(c.tela([0, 0]).is_err());
    }

    #[test]
    fn faixas_em_ordem_e_ultima_sem_limite() {
        let c = ConfigResolucao {
            linhas_acima_da_area_cliente: 38,
            tabelas_posicoes: vec![FaixaTabela { ate_altura: Some(480), tabela: 5 }, FaixaTabela { ate_altura: None, tabela: 7 }],
            resolucoes: vec![[640, 480], [640, 481], [10, 20]],
            perfil_driver: None,
            perfis_driver: BTreeMap::new(),
        };
        assert_eq!(c.tela([640, 480]).unwrap().tabela_posicoes, 5);
        assert_eq!(c.tela([640, 481]).unwrap().tabela_posicoes, 7);
        assert!(c.tela([10, 20]).is_err(), "sem área cliente");
    }
}
