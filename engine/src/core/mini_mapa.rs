//! Mini mapa (janela 20, `miniMapWin`): aberto pelo botão `ribToYellow` da faixa 34
//! (`_OpenMiniMapWin`, `1010:e374`). Janela modal com o plano mostrado em 1 pixel por célula, sem
//! feromônio, aranha nem pegada, e o retângulo da vista de edição; o jogo fica pausado enquanto ela
//! está aberta. Especificação: docs/spec/telas/mapa.md §7; dados: data/mini_mapa.json.
//!
//! Nada aqui sorteia.

use serde::Deserialize;

use super::janela_mapa::{gerar_celulas, JanelaMapa, COLUNAS, LINHAS};
use super::navegacao::Navegacao;
use super::quadro::Quadro;
use super::telas::carregar_janela;
use crate::formats::janela::Retangulo;
use crate::recursos::Recursos;
use crate::rules::estado::Estado;

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_CONFIG: &str = "mini_mapa.json";

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigMiniMapa {
    /// Janela do layout (20) e o objeto que dá o tamanho dela (0, `miniMapWin`).
    pub janela: u16,
    /// Botão da faixa 34 que abre o mini mapa (`ribToYellow`).
    pub botao: String,
    pub moldura: MolduraMini,
    /// Pixels por célula (`_displayType` 10: 1).
    pub escala: i32,
    /// Espessura do retângulo da vista (`_GRectInvOutline(…, 1)`).
    pub espessura_cursor: i32,
    /// Largura e altura da área do mapa (`miniMapArea`, `_miniMapRect`), a partir do canto de dentro
    /// da moldura.
    pub area: [i32; 2],
}

/// Posição e moldura medidas na bancada (m01/m02 e mini3, 2026-10-18): x pela conta de
/// `_OpenMiniMapWin` (centro do botão − metade da largura do objeto 0) + `ajuste_x`; na primeira
/// abertura da execução a janela é criada e a área cliente fica alinhada
/// (`alinhamento_ao_criar`, `gerenciador::alinhar_cliente_ao_criar`); contorno na linha `topo` da área
/// cliente; 1 linha de contorno e `largura_borda − 1` de borda; o exterior mede o objeto 0 mais
/// `largura_borda` (a área de dentro fica 1 pixel menor que o mapa de 128 × 64 em cada sentido).
#[derive(Clone, Debug, Deserialize)]
pub struct MolduraMini {
    pub topo: i16,
    pub ajuste_x: i16,
    /// Alinhamento da área cliente na criação da janela (primeira abertura; 8 = um byte em 16
    /// cores): bancada mini3, 155 na primeira abertura e 156 depois. Ausente (padrão, API 0.30): o
    /// `alinhamento_x` do perfil de driver (`MiniMapa::aplicar_driver`); um mod pode fixá-lo.
    #[serde(default)]
    pub alinhamento_ao_criar: Option<i16>,
    /// Pixels somados ao x na primeira abertura, no lugar do alinhamento (API 0.35): a bancada SVGA
    /// de 256 cores (1 byte por pixel, sem alinhamento) também abre a primeira 1 pixel à esquerda
    /// (155 e depois 156, como na VGA), então o deslocamento não é o `CS_BYTEALIGNCLIENT` [causa em
    /// aberto]. Ausente: o alinhamento de antes.
    #[serde(default)]
    pub ajuste_primeira_abertura: Option<i16>,
    pub largura_borda: i16,
    pub cor_borda: u8,
    pub cor_contorno: u8,
}

/// O que um evento do mouse pede à sessão.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcaoMini {
    Nada,
    /// `_CenterEdit(x, y)` (a vista de edição vai até a célula).
    Centralizar(i32, i32),
    /// Fecha (clique fora da janela): o jogo volta ao estado de pausa de antes.
    Fechar,
    /// O botão que abriu o mini mapa foi solto dentro da área: fecha e `_OpenMapYard`.
    FecharEAbrirMapa,
}

pub struct MiniMapa {
    pub cfg: ConfigMiniMapa,
    tamanho: (i16, i16),
    /// Aberto: o contorno externo da janela.
    pub externo: Option<Retangulo>,
    celulas: Vec<u8>,
    /// Plano mostrado (o `_MapPlane` da abertura).
    plano: i16,
    /// O botão da faixa que abriu ainda está apertado (`_myButton` logo depois da abertura).
    segurando_desde_abertura: bool,
    arrastando: bool,
    /// `_GamePaused` de antes da abertura (`_SetPause(1)` ao abrir, o de antes ao fechar).
    pub pausa_antes: bool,
    /// Já foi aberto nesta execução (muda a coluna, ver `MolduraMini::ajuste_x_reaberto`).
    pub reaberto: bool,
}

fn contem(r: Retangulo, x: i32, y: i32) -> bool {
    x >= i32::from(r.esq) && x < i32::from(r.dir) && y >= i32::from(r.topo) && y < i32::from(r.base)
}

impl MiniMapa {
    pub fn carregar(recursos: &Recursos) -> Result<MiniMapa, String> {
        let cfg: ConfigMiniMapa = recursos.dados(ARQUIVO_CONFIG)?;
        let j = carregar_janela(recursos, cfg.janela)?;
        let r = j.objetos.first().ok_or("mini mapa: janela sem objetos")?.retangulo;
        Ok(MiniMapa::novo(cfg, (r.dir - r.esq, r.base - r.topo)))
    }

    /// O alinhamento da criação vem do perfil de driver, se os dados não o fixam
    /// (docs/spec/telas/resolucao.md §7).
    pub fn aplicar_driver(&mut self, driver: &super::resolucao::PerfilDriver) {
        self.cfg.moldura.alinhamento_ao_criar.get_or_insert(driver.alinhamento_x);
    }

    /// Sem banco (testes): `tamanho` = largura e altura do objeto 0.
    pub fn novo(cfg: ConfigMiniMapa, tamanho: (i16, i16)) -> MiniMapa {
        MiniMapa {
            cfg,
            tamanho,
            externo: None,
            celulas: vec![0; COLUNAS * LINHAS],
            plano: 1,
            segurando_desde_abertura: false,
            arrastando: false,
            pausa_antes: false,
            reaberto: false,
        }
    }

    pub fn aberto(&self) -> bool {
        self.externo.is_some()
    }

    /// `_OpenMiniMapWin`: abre junto ao botão (retângulo na área cliente) e gera o mapa do plano
    /// mostrado (`_Mini_DrawMapI`). Com trilhas e o mesmo `_MapMode` da última geração do mapa, parte
    /// do buffer dele e só escreve as células com vida (`_lastMapMapBuf`, `_showTrails`).
    pub fn abrir(&mut self, botao: Retangulo, e: &Estado, mapa: &JanelaMapa, trilhas: bool, segurando: bool, pausado: bool) {
        let m = &self.cfg.moldura;
        let (w, h) = self.tamanho;
        let b = m.largura_borda;
        let mut esq = botao.esq + (botao.dir - botao.esq) / 2 - w / 2 + m.ajuste_x;
        if !self.reaberto {
            esq = match m.ajuste_primeira_abertura {
                Some(d) => esq + d,
                None => {
                    super::gerenciador::alinhar_cliente_ao_criar(i32::from(esq + b), i32::from(m.alinhamento_ao_criar.unwrap_or(1))) as i16 - b
                }
            };
        }
        self.reaberto = true;
        self.externo = Some(Retangulo { esq, topo: m.topo, dir: esq + w + b, base: m.topo + h + b });
        self.plano = e.vistas.plano.max(1);
        let modo = if (2..=3).contains(&self.plano) { self.plano } else { 1 };
        let acumula = trilhas && mapa.ultimo_modo() == Some(e.vistas.modo_mapa);
        if acumula {
            self.celulas.copy_from_slice(&mapa.celulas);
        }
        gerar_celulas(&mapa.cfg, e, modo, acumula, &mut self.celulas);
        self.segurando_desde_abertura = segurando;
        self.arrastando = false;
        self.pausa_antes = pausado;
    }

    pub fn fechar(&mut self) {
        self.externo = None;
        self.arrastando = false;
        self.segurando_desde_abertura = false;
    }

    /// Área de dentro da moldura (a área cliente da janela).
    pub fn interior(&self) -> Option<Retangulo> {
        let r = self.externo?;
        let b = self.cfg.moldura.largura_borda;
        Some(Retangulo { esq: r.esq + b, topo: r.topo + b, dir: r.dir - b, base: r.base - b })
    }

    /// Área do mapa (`_miniMapRect`): do canto de dentro da moldura, com o tamanho do mapa.
    fn area(&self) -> Option<Retangulo> {
        let i = self.interior()?;
        let [w, h] = self.cfg.area;
        Some(Retangulo { dir: i.esq + w as i16, base: i.topo + h as i16, ..i })
    }

    fn ninho(&self) -> bool {
        (2..=3).contains(&self.plano)
    }

    /// Célula sob (x, y) da área: (x − esq) / escala, menos 32 nos ninhos; fora das colunas do plano,
    /// nada (o clique nas faixas laterais do ninho não centraliza).
    fn celula_em(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let a = self.area()?;
        let k = self.cfg.escala.max(1);
        let mut cx = (x - i32::from(a.esq)) / k;
        let cy = (y - i32::from(a.topo)) / k;
        let largura = if self.ninho() {
            cx -= 32;
            64
        } else {
            COLUNAS as i32
        };
        (0..largura).contains(&cx).then_some((cx, cy))
    }

    /// Botão apertado em (x, y): na área, centraliza e começa o arrasto; fora da janela, fecha. Dentro
    /// da janela e fora da área o original chama `_CenterEdit` com as coordenadas em pixels, mas a
    /// área cobre toda a área cliente, então isso não acontece [código, `1010:e710`].
    pub fn apertar(&mut self, x: i32, y: i32) -> AcaoMini {
        let (Some(a), Some(i)) = (self.area(), self.interior()) else { return AcaoMini::Nada };
        if contem(a, x, y) {
            self.arrastando = true;
            return self.celula_em(x, y).map_or(AcaoMini::Nada, |(cx, cy)| AcaoMini::Centralizar(cx, cy));
        }
        if contem(i, x, y) {
            let (px, py) = (x - i32::from(i.esq), y - i32::from(i.topo));
            return AcaoMini::Centralizar(px, py);
        }
        AcaoMini::Fechar
    }

    /// Mouse andando com o botão apertado: o arrasto segue centralizando.
    pub fn mover(&mut self, x: i32, y: i32) -> AcaoMini {
        if !self.arrastando {
            return AcaoMini::Nada;
        }
        self.celula_em(x, y).map_or(AcaoMini::Nada, |(cx, cy)| AcaoMini::Centralizar(cx, cy))
    }

    /// Botão solto: termina o arrasto; o botão da faixa que abriu, solto dentro da área, fecha e abre
    /// o mapa (ou traz o mapa ou o quintal para a frente).
    pub fn soltar(&mut self, x: i32, y: i32) -> AcaoMini {
        self.arrastando = false;
        if std::mem::take(&mut self.segurando_desde_abertura) && self.area().is_some_and(|a| contem(a, x, y)) {
            return AcaoMini::FecharEAbrirMapa;
        }
        AcaoMini::Nada
    }

    /// Desenha a moldura, as células (1 pixel cada; cortadas pela área de dentro) e o retângulo XOR
    /// da vista de edição. Cores 16+ (blocos da rampa do feromônio, só com trilhas sobre um
    /// feromônio) usam o primeiro pixel do bloco [hipótese].
    pub fn desenhar(&self, q: &mut Quadro, mapa: &JanelaMapa, nav: &Navegacao) {
        let (Some(ext), Some(int), Some(a)) = (self.externo, self.interior(), self.area()) else { return };
        let m = &self.cfg.moldura;
        q.preencher(ext, m.cor_borda, m.cor_borda);
        q.contorno(Retangulo { dir: ext.dir - 1, base: ext.base - 1, ..ext }, m.cor_contorno);
        let antes = q.recorte.replace(int);
        let k = self.cfg.escala.max(1);
        let (ox, oy) = (i32::from(a.esq), i32::from(a.topo));
        for y in 0..LINHAS {
            for x in 0..COLUNAS {
                let c = self.celulas[y * COLUNAS + x];
                let c = if c < 16 { c } else { mapa.cfg.blocos.get(usize::from(c - 16)).map_or(0, |b| b[0][0]) };
                let (px, py) = (ox + x as i32 * k, oy + y as i32 * k);
                q.preencher(Retangulo { esq: px as i16, topo: py as i16, dir: (px + k) as i16, base: (py + k) as i16 }, c, c);
            }
        }
        let v = &nav.vista;
        let desloc = if self.ninho() { 32 * k } else { 0 };
        let (cx, cy) = (ox + k * v.x + desloc, oy + k * v.y);
        let (w, h) = (k * v.largura, k * v.altura);
        let e = self.cfg.espessura_cursor;
        let r = |x0: i32, y0: i32, x1: i32, y1: i32| Retangulo { esq: x0 as i16, topo: y0 as i16, dir: x1 as i16, base: y1 as i16 };
        q.inverter(r(cx, cy, cx + w, cy + e));
        q.inverter(r(cx, cy + h - e, cx + w, cy + h));
        q.inverter(r(cx, cy + e, cx + e, cy + h - e));
        q.inverter(r(cx + w - e, cy + e, cx + w, cy + h - e));
        q.recorte = antes;
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn mini_com(alinhamento_x: i16) -> MiniMapa {
        let cfg: ConfigMiniMapa = serde_json::from_str(include_str!("../../../data/mini_mapa.json")).unwrap();
        let mut m = MiniMapa::novo(cfg, (132, 68));
        m.aplicar_driver(&crate::core::resolucao::PerfilDriver { alinhamento_x, ..Default::default() });
        m
    }

    /// Perfil VGA de 16 cores (a bancada): área cliente alinhada a 8 pixels na criação.
    fn mini() -> MiniMapa {
        mini_com(8)
    }

    fn mapa() -> JanelaMapa {
        JanelaMapa::sem_banco(serde_json::from_str(include_str!("../../../data/mapa.json")).unwrap())
    }

    /// ribToYellow (207, 41)–(232, 59): o mini mapa abre em x = 207 + 12 − 66 + 3 = 156, com a área
    /// cliente (161) alinhada a 160 na primeira abertura: 155, com o
    /// contorno na linha 75 e 137 × 73 por fora (bancada m02: (155, 113)–(291, 185) na tela, área
    /// cliente a partir da linha 38).
    #[test]
    fn posicao_e_area_da_bancada() {
        let mut m = mini();
        let mut e = Estado::vazio();
        e.vistas.plano = 2;
        m.abrir(Retangulo { esq: 207, topo: 41, dir: 232, base: 59 }, &e, &mapa(), false, true, false);
        assert_eq!(m.externo, Some(Retangulo { esq: 155, topo: 75, dir: 292, base: 148 }));
        assert_eq!(m.interior(), Some(Retangulo { esq: 160, topo: 80, dir: 287, base: 143 }));
        // Ninho: a coluna 32 da área é a célula 0; as faixas laterais não centralizam.
        assert_eq!(m.apertar(160 + 32 + 5, 80 + 7), AcaoMini::Centralizar(5, 7));
        assert_eq!(m.mover(160 + 10, 80 + 7), AcaoMini::Nada);
        assert_eq!(m.mover(160 + 40, 80 + 9), AcaoMini::Centralizar(8, 9));
        // O botão que abriu, solto dentro da área: fecha e abre o mapa (só uma vez).
        assert_eq!(m.soltar(200, 100), AcaoMini::FecharEAbrirMapa);
        assert_eq!(m.soltar(200, 100), AcaoMini::Nada);
        assert_eq!(m.apertar(400, 300), AcaoMini::Fechar);
        // Reaberto: uma coluna à direita (bancada mini3).
        m.fechar();
        m.abrir(Retangulo { esq: 207, topo: 41, dir: 232, base: 59 }, &e, &mapa(), false, true, false);
        assert_eq!(m.externo.map(|r| r.esq), Some(156));
        // Em 256 cores (perfil svga256, 1 byte por pixel) a primeira abertura também é 155 e as
        // seguintes 156 (bancada SVGA, qemu_svga640_mini.txt, 2026-10-19): o −1 não é alinhamento.
        let mut s = mini_com(1);
        s.abrir(Retangulo { esq: 207, topo: 41, dir: 232, base: 59 }, &e, &mapa(), false, true, false);
        assert_eq!(s.externo.map(|r| r.esq), Some(155));
        s.fechar();
        s.abrir(Retangulo { esq: 207, topo: 41, dir: 232, base: 59 }, &e, &mapa(), false, true, false);
        assert_eq!(s.externo.map(|r| r.esq), Some(156));
    }

    #[test]
    fn superficie_um_pixel_por_celula_e_cursor() {
        let mut m = mini();
        let mut e = Estado::vazio();
        e.vistas.plano = 1;
        e.vistas.modo_mapa = 8;
        e.superficie.vida.definir(3, 2, 0x20);
        let mp = mapa();
        m.abrir(Retangulo { esq: 207, topo: 41, dir: 232, base: 59 }, &e, &mp, false, false, true);
        assert!(m.pausa_antes);
        // Feromônio no mapa grande: o mini mapa mostra a superfície.
        assert_eq!(m.celulas[2 * COLUNAS + 3], mp.cfg.ltab[4]);
        assert_eq!(m.celulas[0], mp.cfg.ctab[0]);
        assert_eq!(m.apertar(160 + 70, 80 + 32), AcaoMini::Centralizar(70, 32));
        assert_eq!(m.soltar(160 + 70, 80 + 32), AcaoMini::Nada, "não estava segurando desde a abertura");
    }
}
