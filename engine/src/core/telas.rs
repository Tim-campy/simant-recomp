//! Sequência de telas estáticas (abertura). Especificação: docs/spec/telas/abertura.md.
//! Sem dependência de janela: quem desenha é a camada de render.

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct BitmapNaTela {
    pub id: u16,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Gatilho {
    Espaco,
    Clique,
    QualquerTecla,
}

/// Janela do jogo (layout tipo 0 do banco) desenhada na tela, na posição da tabela tipo 9.
#[derive(Clone, Debug, Deserialize)]
pub struct JanelaNaTela {
    pub id: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Tela {
    pub nome: String,
    pub largura: u32,
    pub altura: u32,
    /// Índice de paleta do fundo.
    pub fundo: u8,
    #[serde(default)]
    pub bitmaps: Vec<BitmapNaTela>,
    #[serde(default)]
    pub janelas: Vec<JanelaNaTela>,
    pub duracao_ticks: u32,
    pub fechar_com: Vec<Gatilho>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Sequencia {
    pub ms_por_tick: u32,
    pub telas: Vec<Tela>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evento {
    Espaco,
    Clique,
    OutraTecla,
}

/// Estado de execução de uma `Sequencia`.
pub struct Reprodutor {
    seq: Sequencia,
    indice: usize,
    ms_na_tela: u64,
}

impl Reprodutor {
    pub fn new(seq: Sequencia) -> Reprodutor {
        Reprodutor { seq, indice: 0, ms_na_tela: 0 }
    }

    pub fn atual(&self) -> Option<&Tela> {
        self.seq.telas.get(self.indice)
    }

    pub fn terminou(&self) -> bool {
        self.indice >= self.seq.telas.len()
    }

    fn proxima(&mut self) {
        self.indice += 1;
        self.ms_na_tela = 0;
    }

    pub fn passar_tempo(&mut self, ms: u64) {
        let Some(t) = self.atual() else { return };
        let limite = t.duracao_ticks as u64 * self.seq.ms_por_tick as u64;
        self.ms_na_tela += ms;
        if self.ms_na_tela >= limite {
            self.proxima();
        }
    }

    pub fn evento(&mut self, e: Evento) {
        let Some(t) = self.atual() else { return };
        let fecha = t.fechar_com.iter().any(|g| {
            matches!(
                (g, e),
                (Gatilho::QualquerTecla, Evento::Espaco | Evento::OutraTecla)
                    | (Gatilho::Espaco, Evento::Espaco)
                    | (Gatilho::Clique, Evento::Clique)
            )
        });
        if fecha {
            self.proxima();
        }
    }
}

/// Faixa de ferramentas no topo da área cliente (`_ribbonBarHeight`).
#[derive(Clone, Debug, Deserialize)]
pub struct Faixa {
    /// Janela desenhada na faixa (34 = `mapRibbon`).
    pub janela: u16,
    pub altura: i16,
    /// A faixa é desenhada em y − `y_tabela` em relação à tabela de posições.
    pub y_tabela: i16,
    pub fundo: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Borda {
    pub largura: i16,
    pub cor_externa: u8,
    pub cor: u8,
    /// Pixels a mais em x na origem do conteúdo quando o objeto 0 é contorno (tipo 15) [hipótese].
    pub ajuste_x_contorno: i16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigFontes {
    /// Índice = byte "fonte" dos objetos (`_font_SetFont`): "ARQUIVO.FON#Nome" ou arquivo FONT do Mac.
    pub lista: Vec<String>,
}

/// Moldura da interface (data/interface.json), medida no original.
#[derive(Clone, Debug, Deserialize)]
pub struct Moldura {
    pub fontes: ConfigFontes,
    pub largura: u32,
    pub altura: u32,
    pub fundo: u8,
    pub faixa: Faixa,
    pub borda: Borda,
}

/// Cores, posições das janelas e moldura, carregadas uma vez (docs/spec/formatos/janela.md).
pub struct Interface {
    pub cores: Vec<crate::formats::janela::Cor>,
    pub posicoes: Vec<[i16; 4]>,
    pub moldura: Moldura,
    /// Fontes por índice; None quando o arquivo não existe (como `font5`/`font6` no original).
    pub fontes: Vec<Option<crate::formats::fonte::Fonte>>,
}

fn carregar_fonte(recursos: &crate::recursos::Recursos, nome: &str) -> Result<crate::formats::fonte::Fonte, String> {
    use crate::formats::fonte::{fontes_de_fon, Fonte, FonteMac};
    match nome.split_once('#') {
        Some((arquivo, face)) => fontes_de_fon(&recursos.arquivo(arquivo)?)?
            .into_iter()
            .find(|f| f.nome == face)
            .map(Fonte::Fnt)
            .ok_or(format!("fonte {face} não encontrada em {arquivo}")),
        None => Ok(Fonte::Mac(FonteMac::decodificar(&recursos.arquivo(nome)?)?)),
    }
}

impl Interface {
    /// `tabela_posicoes`: id do objeto tipo 9 (0 ou 8 no modo colorido).
    pub fn carregar(recursos: &crate::recursos::Recursos, tabela_posicoes: u16) -> Result<Interface, String> {
        use crate::formats::banco::Chave;
        use crate::formats::janela::*;
        let cab = recursos.objeto(Chave { tipo: TIPO_LAYOUT, id: ID_CABECALHO_CORES })?;
        let tab = recursos.objeto(Chave { tipo: TIPO_LAYOUT, id: ID_TABELA_CORES })?;
        let pos = recursos.objeto(Chave { tipo: TIPO_POSICOES, id: tabela_posicoes })?;
        let moldura: Moldura = recursos.dados("interface.json")?;
        let fontes = moldura.fontes.lista.iter().map(|n| carregar_fonte(recursos, n).ok()).collect();
        Ok(Interface {
            cores: decodificar_cores(&cab, &tab)?,
            posicoes: decodificar_posicoes(&pos)?,
            moldura,
            fontes,
        })
    }

    /// Geometria da janela `id` na área cliente (ver docs/spec/formatos/janela.md, "Posicionamento"):
    /// origem do objeto 0, retângulo externo da borda (None na faixa) e retângulo interno (recorte).
    pub fn posicionar(&self, id: u16, janela: &crate::formats::janela::Janela) -> Option<Geometria> {
        use crate::formats::janela::{Retangulo, TIPO_CONTORNO};
        let [x, y, w, h] = *self.posicoes.get(id as usize)?;
        let m = &self.moldura;
        if id == m.faixa.janela {
            let origem = (x, y - m.faixa.y_tabela);
            return Some(Geometria { origem, externo: None, interno: None });
        }
        // _win_Recalc: com objeto 0 de contorno de espessura t, a janela encolhe (L+t, T+t, R−(t+1), B−(t+1)).
        let ob0 = &janela.objetos[0];
        let t = if ob0.tipo == TIPO_CONTORNO { ob0.byte_28 as i16 } else { 0 };
        let (lw, lh) = if t > 0 { (w - 2 * t - 1, h - 2 * t - 1) } else { (w, h) };
        let b = m.borda.largura;
        let (ex, ey) = (x + t, y + t + m.faixa.altura);
        let externo = Retangulo { esq: ex, topo: ey, dir: ex + lw + 2 * b, base: ey + lh + 2 * b };
        let interno = Retangulo { esq: ex + b, topo: ey + b, dir: ex + b + lw, base: ey + b + lh };
        let ajuste = if t > 0 { m.borda.ajuste_x_contorno } else { 0 };
        let origem = (interno.esq - t + ajuste, interno.topo - t);
        Some(Geometria { origem, externo: Some(externo), interno: Some(interno) })
    }
}

/// Onde e com que recorte uma janela é desenhada.
pub struct Geometria {
    pub origem: (i16, i16),
    pub externo: Option<crate::formats::janela::Retangulo>,
    pub interno: Option<crate::formats::janela::Retangulo>,
}

/// Monta a tela como índices de paleta: fundo, bitmaps soltos e depois janelas, na ordem da lista.
/// Devolve o quadro e os nomes de objetos que ainda não sabemos desenhar.
pub fn compor(
    tela: &Tela,
    recursos: &crate::recursos::Recursos,
    interface: &Interface,
) -> Result<(super::quadro::Quadro, Vec<String>), String> {
    use super::quadro::{desenhar_janela, Quadro};
    use crate::formats::banco::Chave;
    use crate::formats::bitmap::{Bitmap, TIPO_BITMAP};
    use crate::formats::janela::{Janela, Retangulo, TIPO_LAYOUT, TIPO_NOMES};

    let carregar = |id: u16| -> Result<Bitmap, String> {
        Bitmap::decodificar(&recursos.objeto(Chave { tipo: TIPO_BITMAP, id })?)
    };
    let m = &interface.moldura;
    let mut q = Quadro::new(tela.largura as usize, tela.altura as usize, tela.fundo);
    for b in &tela.bitmaps {
        q.bitmap(&carregar(b.id)?, b.x, b.y);
    }
    let mut pendentes = Vec::new();
    for j in &tela.janelas {
        let layout = recursos.objeto(Chave { tipo: TIPO_LAYOUT, id: j.id })?;
        let nomes = recursos.objeto(Chave { tipo: TIPO_NOMES, id: j.id }).ok();
        let janela = Janela::decodificar(&layout, nomes.as_deref())?;
        let g = interface.posicionar(j.id, &janela).ok_or(format!("janela {} sem posição", j.id))?;
        if j.id == m.faixa.janela {
            let faixa = Retangulo { esq: 0, topo: 0, dir: tela.largura as i16, base: m.faixa.altura };
            q.preencher(faixa, m.faixa.fundo, m.faixa.fundo);
        }
        if let Some(r) = g.externo {
            q.preencher(r, m.borda.cor, m.borda.cor);
            q.contorno(Retangulo { dir: r.dir - 1, base: r.base - 1, ..r }, m.borda.cor_externa);
        }
        let mut faltou = Vec::new();
        let mut bitmap = |id: u16| match carregar(id) {
            Ok(b) => Some(b),
            Err(e) => {
                faltou.push(format!("bitmap {id}: {e}"));
                None
            }
        };
        q.recorte = g.interno;
        pendentes.extend(desenhar_janela(&mut q, &janela, g.origem, &interface.cores, &interface.fontes, &mut bitmap));
        q.recorte = None;
        pendentes.extend(faltou);
    }
    Ok((q, pendentes))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn seq() -> Sequencia {
        serde_json::from_str(
            r#"{"ms_por_tick": 55, "telas": [
                {"nome":"a","largura":10,"altura":10,"fundo":0,"bitmaps":[],"duracao_ticks":720,"fechar_com":["espaco","clique"]},
                {"nome":"b","largura":10,"altura":10,"fundo":0,"bitmaps":[],"duracao_ticks":10,"fechar_com":[]}
            ]}"#,
        )
        .unwrap()
    }

    #[test]
    fn abertura_dura_720_ticks_de_55ms() {
        // docs/spec/telas/abertura.md: 40 * 18 ticks de 55 ms ≈ 39,6 s
        let mut r = Reprodutor::new(seq());
        r.passar_tempo(720 * 55 - 1);
        assert_eq!(r.atual().unwrap().nome, "a");
        r.passar_tempo(1);
        assert_eq!(r.atual().unwrap().nome, "b");
    }

    #[test]
    fn espaco_fecha_mas_outra_tecla_nao() {
        let mut r = Reprodutor::new(seq());
        r.evento(Evento::OutraTecla);
        assert_eq!(r.atual().unwrap().nome, "a");
        r.evento(Evento::Espaco);
        assert_eq!(r.atual().unwrap().nome, "b");
    }

    #[test]
    fn termina_depois_da_ultima_tela() {
        let mut r = Reprodutor::new(seq());
        r.evento(Evento::Clique);
        r.passar_tempo(10 * 55);
        assert!(r.terminou());
    }
}
