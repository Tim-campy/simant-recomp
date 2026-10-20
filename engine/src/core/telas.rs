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
    /// Obsoleto desde a API 0.29: o ajuste depende do driver e fica em `data/resolucao.json`
    /// (`perfis_driver` → `ajuste_x_contorno`). Lido e ignorado, para os mods antigos.
    #[serde(default)]
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
    /// Regras do driver de vídeo imitado (alinhamento ao byte, ajuste do contorno; data/resolucao.json).
    pub driver: super::resolucao::PerfilDriver,
    /// Área cliente da janela principal (largura, altura): a tela virtual menos `linhas_acima`.
    pub area: [i16; 2],
    /// Linhas da tela acima da área cliente (título e menus).
    pub linhas_acima: i16,
}

fn carregar_fonte(recursos: &crate::recursos::Recursos, nome: &str) -> Result<crate::formats::fonte::Fonte, String> {
    use crate::formats::fonte::{fontes_de_fon, Fonte, FonteMac};
    match nome.split_once('#') {
        // "ARQUIVO.FON#Nome", com "@altura" para escolher o tamanho e "+negrito" para o negrito
        // simulado (docs/spec/formatos/fonte.md).
        Some((arquivo, face)) => {
            let (face, negrito) = match face.strip_suffix("+negrito") {
                Some(f) => (f, true),
                None => (face, false),
            };
            let (face, altura) = match face.split_once('@') {
                Some((f, a)) => (f, Some(a.parse::<usize>().map_err(|_| format!("font {nome}: invalid height"))?)),
                None => (face, None),
            };
            fontes_de_fon(&recursos.arquivo(arquivo)?)?
                .into_iter()
                .find(|f| f.nome == face && altura.is_none_or(|a| f.altura == a))
                .map(|mut f| {
                    f.negrito = negrito;
                    Fonte::Fnt(f)
                })
                .ok_or(format!("font {face} not found in {arquivo}"))
        }
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
        let (driver, linhas_acima) = if recursos.existe_dado(super::resolucao::ARQUIVO) {
            let r: super::resolucao::ConfigResolucao = recursos.dados(super::resolucao::ARQUIVO)?;
            (r.perfil(None)?, r.linhas_acima_da_area_cliente as i16)
        } else {
            (Default::default(), 38)
        };
        let area = [moldura.largura as i16, moldura.altura as i16];
        Ok(Interface { cores: decodificar_cores(&cab, &tab)?, posicoes: decodificar_posicoes(&pos)?, moldura, fontes, driver, area, linhas_acima })
    }

    /// Canto (x, y), na área cliente, de uma caixa de diálogo do Windows de `largura` × `altura`
    /// (contorno incluído) centrada na tela: `MessageBox` centra o diálogo sem o contorno de 1 pixel
    /// (metade da sobra, arredondada para baixo) e o x vai para o byte mais próximo
    /// (`CS_BYTEALIGNWINDOW`) [medido em 640×480 na bancada VGA e em 640, 800 e 1024 na SVGA;
    /// telas/resolucao.md §3].
    pub fn centrar_dialogo(&self, largura: i16, altura: i16) -> (i16, i16) {
        let [w, h] = self.area;
        let tela_a = h + self.linhas_acima;
        let x = (w - (largura - 2)).div_euclid(2);
        let y = (tela_a - (altura - 2)).div_euclid(2);
        (self.driver.alinhar(x), y - self.linhas_acima)
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
        // _win_Open cria a janela no canto da tabela; a área cliente (depois da moldura de diálogo de
        // `b` pixels) vai para o byte mais próximo (CS_BYTEALIGNCLIENT; telas/resolucao.md §7).
        let (ex, ey) = (self.driver.alinhar_janela(x + t, b), y + t + m.faixa.altura);
        let externo = Retangulo { esq: ex, topo: ey, dir: ex + lw + 2 * b, base: ey + lh + 2 * b };
        let interno = Retangulo { esq: ex + b, topo: ey + b, dir: ex + b + lw, base: ey + b + lh };
        let ajuste = if t > 0 { self.driver.ajuste_x_contorno } else { 0 };
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
    use super::quadro::Quadro;
    use crate::formats::banco::Chave;
    use crate::formats::bitmap::{Bitmap, TIPO_BITMAP};

    let mut q = Quadro::new(tela.largura as usize, tela.altura as usize, tela.fundo);
    for b in &tela.bitmaps {
        q.bitmap(&Bitmap::decodificar(&recursos.objeto(Chave { tipo: TIPO_BITMAP, id: b.id })?)?, b.x, b.y);
    }
    let mut pendentes = Vec::new();
    for j in &tela.janelas {
        pendentes.extend(desenhar_janela_na_tela(&mut q, recursos, interface, j.id)?);
    }
    Ok((q, pendentes))
}

/// Layout (tipo 0) e nomes (tipo 1) da janela `id`.
pub fn carregar_janela(recursos: &crate::recursos::Recursos, id: u16) -> Result<crate::formats::janela::Janela, String> {
    use crate::formats::banco::Chave;
    use crate::formats::janela::{Janela, TIPO_LAYOUT, TIPO_NOMES};
    let layout = recursos.objeto(Chave { tipo: TIPO_LAYOUT, id })?;
    let nomes = recursos.objeto(Chave { tipo: TIPO_NOMES, id }).ok();
    Janela::decodificar(&layout, nomes.as_deref())
}

/// Desenha os objetos visíveis da janela com o objeto 0 em `origem` e o recorte `interno`.
/// Devolve o que não foi desenhado (objetos sem suporte e bitmaps ausentes).
pub(crate) fn desenhar_objetos(
    q: &mut super::quadro::Quadro,
    recursos: &crate::recursos::Recursos,
    interface: &Interface,
    janela: &crate::formats::janela::Janela,
    origem: (i16, i16),
    interno: Option<crate::formats::janela::Retangulo>,
) -> Vec<String> {
    use super::quadro::desenhar_janela;
    use crate::formats::banco::Chave;
    use crate::formats::bitmap::{Bitmap, TIPO_BITMAP};
    let mut faltou = Vec::new();
    let mut bitmap = |id: u16| match recursos.objeto(Chave { tipo: TIPO_BITMAP, id }).and_then(|d| Bitmap::decodificar(&d)) {
        Ok(b) => Some(b),
        Err(e) => {
            faltou.push(format!("bitmap {id}: {e}"));
            None
        }
    };
    q.recorte = interno;
    let mut pendentes = desenhar_janela(q, janela, origem, &interface.cores, &interface.fontes, &mut bitmap);
    q.recorte = None;
    pendentes.extend(faltou);
    pendentes
}

/// Desenha a janela `id` na posição da tabela tipo 9, com a moldura medida no DOSBox-X (ou a faixa).
pub fn desenhar_janela_na_tela(
    q: &mut super::quadro::Quadro,
    recursos: &crate::recursos::Recursos,
    interface: &Interface,
    id: u16,
) -> Result<Vec<String>, String> {
    let janela = carregar_janela(recursos, id)?;
    desenhar_janela_carregada_na_tela(q, recursos, interface, id, &janela)
}

/// Como `desenhar_janela_na_tela`, com o layout já carregado (e talvez com objetos selecionados).
pub fn desenhar_janela_carregada_na_tela(
    q: &mut super::quadro::Quadro,
    recursos: &crate::recursos::Recursos,
    interface: &Interface,
    id: u16,
    janela: &crate::formats::janela::Janela,
) -> Result<Vec<String>, String> {
    desenhar_janela_deslocada(q, recursos, interface, id, janela, (0, 0))
}

/// Como `desenhar_janela_carregada_na_tela`, com a janela inteira deslocada de `(dx, dy)` em relação
/// à posição calculada (ajustes medidos numa resolução, como o do fim de jogo na bancada QEMU).
pub fn desenhar_janela_deslocada(
    q: &mut super::quadro::Quadro,
    recursos: &crate::recursos::Recursos,
    interface: &Interface,
    id: u16,
    janela: &crate::formats::janela::Janela,
    (dx, dy): (i16, i16),
) -> Result<Vec<String>, String> {
    use crate::formats::janela::Retangulo;
    let m = &interface.moldura;
    let mut g = interface.posicionar(id, janela).ok_or(format!("window {id} has no position"))?;
    let mover = |r: Retangulo| Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy };
    g.origem = (g.origem.0 + dx, g.origem.1 + dy);
    g.externo = g.externo.map(mover);
    g.interno = g.interno.map(mover);
    if id == m.faixa.janela {
        let faixa = Retangulo { esq: 0, topo: 0, dir: q.largura as i16, base: m.faixa.altura };
        q.preencher(faixa, m.faixa.fundo, m.faixa.fundo);
    }
    if let Some(r) = g.externo {
        q.preencher(r, m.borda.cor, m.borda.cor);
        q.contorno(Retangulo { dir: r.dir - 1, base: r.base - 1, ..r }, m.borda.cor_externa);
    }
    Ok(desenhar_objetos(q, recursos, interface, janela, g.origem, g.interno))
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
