//! Janela About (File → About): `_AboutDialog` (`1000:cf3a`) pausa o jogo, abre a janela 31 e alterna
//! a figura da formiga com os créditos (lista 4/1700) rolando para cima, um pixel por passo, dentro do
//! objeto `aboutPic2`. Fecha com um clique ou Espaço/Insert depois dos primeiros passos; no fim, devolve a
//! pausa como estava. Especificação: docs/spec/telas/menus.md, "About".

use serde::Deserialize;

use super::avisos::{linhas_do_texto, objeto_na_tela};
use super::quadro::Quadro;
use super::telas::{carregar_janela, desenhar_janela_deslocada, Interface};
use crate::formats::janela::{Cor, Retangulo};
use crate::recursos::Recursos;

/// Parte "sobre" de data/barra_menus.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigSobre {
    pub janela: u16,
    /// Objeto da figura (apagada pelo fundo no começo de cada volta e redesenhada no fim), que é também
    /// a área dos créditos.
    pub objeto_figura: String,
    pub lista: u16,
    pub fonte: usize,
    /// Tempo com a figura antes de cada volta de rolagem.
    pub espera_figura_ms: u64,
    /// Tempo de cada passo (um pixel) da rolagem.
    pub ms_por_passo: u64,
    /// Passos (`local_e` de `_AboutDialog`) a partir dos quais o clique fecha: com a figura e na rolagem.
    pub passos_para_fechar: [u32; 2],
    pub ajuste: [i16; 2],
    pub moldura: super::status::MolduraDialogo,
}

/// Fase da janela About.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fase {
    /// Figura da formiga, desde o instante dado.
    Figura { desde: u64 },
    /// Créditos rolando: deslocamento em pixels (o passo atual), desde o instante do passo.
    Rolando { deslocamento: u32, desde: u64 },
}

/// About aberto.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sobre {
    pub fase: Fase,
    /// Passos dados (`local_e`): cada passo de rolagem conta um.
    pub passos: u32,
    /// `_GamePaused` antes de abrir (restaurado ao fechar).
    pub pausado_antes: bool,
    /// Número de linhas dos créditos e altura da fonte (fixados ao abrir).
    linhas: u32,
    altura_linha: u32,
}

impl Sobre {
    pub fn abrir(recursos: &Recursos, interface: &Interface, cfg: &ConfigSobre, agora_ms: u64, pausado_antes: bool) -> Sobre {
        let linhas = linhas_do_texto(recursos, cfg.lista).len() as u32;
        let altura_linha = interface.fontes.get(cfg.fonte).and_then(|f| f.as_ref()).map_or(14, |f| f.altura() as u32);
        Sobre { fase: Fase::Figura { desde: agora_ms }, passos: 0, pausado_antes, linhas, altura_linha: altura_linha.max(1) }
    }

    fn fim_da_volta(&self) -> u32 {
        self.linhas * self.altura_linha
    }

    /// Passa o relógio: troca de fase e anda a rolagem. Devolve se mudou o desenho.
    pub fn passar_tempo(&mut self, cfg: &ConfigSobre, agora_ms: u64) -> bool {
        let mut mudou = false;
        loop {
            match self.fase {
                Fase::Figura { desde } if agora_ms >= desde + cfg.espera_figura_ms => {
                    self.passos += 1;
                    self.fase = Fase::Rolando { deslocamento: 0, desde: desde + cfg.espera_figura_ms };
                }
                Fase::Rolando { deslocamento, desde } if agora_ms >= desde + cfg.ms_por_passo.max(1) => {
                    self.passos += 1;
                    let t = desde + cfg.ms_por_passo.max(1);
                    self.fase = if deslocamento + 1 >= self.fim_da_volta() { Fase::Figura { desde: t } } else { Fase::Rolando { deslocamento: deslocamento + 1, desde: t } };
                }
                _ => return mudou,
            }
            mudou = true;
        }
    }

    /// Clique ou Espaço/Insert. Devolve true se fecha. Com a figura e poucos passos, começa a rolagem.
    pub fn clicar(&mut self, cfg: &ConfigSobre, agora_ms: u64) -> bool {
        match self.fase {
            Fase::Figura { .. } => {
                if self.passos > cfg.passos_para_fechar[0] {
                    return true;
                }
                self.passos += 1;
                self.fase = Fase::Rolando { deslocamento: 0, desde: agora_ms };
                false
            }
            Fase::Rolando { .. } => self.passos > cfg.passos_para_fechar[1],
        }
    }

    /// Desenha a janela na fase atual. Devolve o que não foi desenhado.
    pub fn desenhar(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface, cfg: &ConfigSobre) -> Result<Vec<String>, String> {
        let mut janela = carregar_janela(recursos, cfg.janela)?;
        let rolando = matches!(self.fase, Fase::Rolando { .. });
        if rolando {
            // _win_DrawObjectNum(aboutPic1) por cima de tudo: a figura some.
            for o in janela.objetos.iter_mut().filter(|o| o.nome == cfg.objeto_figura) {
                o.flags &= !1;
            }
        }
        let pendentes = desenhar_janela_deslocada(q, recursos, interface, cfg.janela, &janela, (cfg.ajuste[0], cfg.ajuste[1]))?;
        super::status::pintar_moldura(q, interface, cfg.janela, &janela, cfg.ajuste, cfg.moldura);
        let Fase::Rolando { deslocamento, .. } = self.fase else { return Ok(pendentes) };
        let Some(fonte) = interface.fontes.get(cfg.fonte).and_then(|f| f.as_ref()) else { return Ok(pendentes) };
        let (area, cor) = objeto_na_tela(interface, cfg.janela, &janela, &cfg.objeto_figura, cfg.ajuste)?;
        let c = interface.cores.get(cor as usize).copied().unwrap_or(Cor { frente: 0, fundo: 8, fundo2: 8 });
        let linhas = linhas_do_texto(recursos, cfg.lista);
        let h = self.altura_linha as i32;
        let antes = q.recorte.replace(area);
        q.preencher(area, c.fundo, c.fundo);
        let primeira = deslocamento as i32 / h;
        let mut y = i32::from(area.topo) - deslocamento as i32 % h;
        let mut i = primeira;
        while y < i32::from(area.base) {
            if let Some(l) = linhas.get(i as usize).filter(|l| !l.is_empty()) {
                // _gr_JustifyStrInRect: centrado na largura do objeto, na faixa da linha.
                let r = Retangulo { esq: area.esq, topo: y as i16, dir: area.dir, base: (y + h) as i16 };
                q.texto(r, fonte, l, 0, c.frente, c.fundo);
            }
            y += h;
            i += 1;
        }
        q.recorte = antes;
        Ok(pendentes)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cfg() -> ConfigSobre {
        ConfigSobre {
            janela: 31,
            objeto_figura: "aboutPic2".into(),
            lista: 1700,
            fonte: 4,
            espera_figura_ms: 1000,
            ms_por_passo: 50,
            passos_para_fechar: [1, 3],
            ajuste: [0, 0],
            moldura: super::super::status::MolduraDialogo { cor: 4, espessura: 4 },
        }
    }

    #[test]
    fn figura_rolagem_e_volta() {
        let c = cfg();
        let mut s = Sobre { fase: Fase::Figura { desde: 0 }, passos: 0, pausado_antes: false, linhas: 2, altura_linha: 10 };
        assert!(!s.passar_tempo(&c, 999));
        assert!(s.passar_tempo(&c, 1000));
        assert_eq!(s.fase, Fase::Rolando { deslocamento: 0, desde: 1000 });
        s.passar_tempo(&c, 1000 + 50 * 19);
        assert_eq!(s.fase, Fase::Rolando { deslocamento: 19, desde: 1950 });
        s.passar_tempo(&c, 2000);
        assert_eq!(s.fase, Fase::Figura { desde: 2000 }, "20 pixels = 2 linhas de 10: volta à figura");
    }

    #[test]
    fn clique_cedo_comeca_a_rolagem_e_depois_fecha() {
        let c = cfg();
        let mut s = Sobre { fase: Fase::Figura { desde: 0 }, passos: 0, pausado_antes: true, linhas: 5, altura_linha: 10 };
        assert!(!s.clicar(&c, 10), "o primeiro clique só começa a rolagem");
        assert!(matches!(s.fase, Fase::Rolando { deslocamento: 0, .. }));
        assert!(!s.clicar(&c, 20), "na rolagem, só depois de passos_para_fechar[1] passos");
        s.passar_tempo(&c, 20 + 50 * 3);
        assert!(s.clicar(&c, 200));
    }
}
