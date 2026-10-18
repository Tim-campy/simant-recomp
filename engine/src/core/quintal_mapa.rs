//! Janela do quintal (janela 25, "Yard View") e faixa do quintal (janela 35), sem SDL: geometria dos
//! patches, desenho de cada modo, cliques e teclas, e a ponte com a `Partida` para escolher e trocar
//! de patch (`_YardArea`, `_XferPatch`) e colocar rainhas (`_PlaceQueenInYard`).
//!
//! Especificação: docs/spec/telas/quintal.md. Dados: data/quintal_mapa.json (sobrescrevível por mod).

use std::collections::HashMap;

use serde::Deserialize;

use super::navegacao::Evento as EventoTecla;
use super::partida::Partida;
use super::quadro::Quadro;
use super::telas::{carregar_janela, desenhar_objetos, Interface};
use crate::formats::banco::Chave;
use crate::formats::bitmap::{Bitmap, TIPO_BITMAP};
use crate::formats::fonte::Fonte;
use crate::formats::janela::{Janela, Retangulo};
use crate::recursos::Recursos;
use crate::rules::estado::{Estado, Evento};
use crate::rules::troca_patch::{self, ResultadoRainha, ResultadoTroca};

/// Nome do arquivo de dados.
pub const ARQUIVO_CONFIG: &str = "quintal_mapa.json";

/// Retângulo (x, y, largura, altura).
pub type Caixa = [i16; 4];

/// Bitmap numa posição relativa à área cliente da janela 25.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct BitmapPosto {
    pub id: u16,
    pub x: i16,
    pub y: i16,
}

/// O que aparece por cima do fundo num modo.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Conteudo {
    /// Menino, bichos, cortador, chuva (modos 0 e 1).
    Bichos,
    /// Cor de cada patch pela colônia (modo 2).
    Colonias,
    /// Barras de população (modo 3).
    Barras,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ModoQuintal {
    pub bitmaps: Vec<BitmapPosto>,
    pub conteudo: Conteudo,
    /// No modo 1 (House View) o menino aparece também dentro de casa (`_BoyHere = 0`).
    pub mostra_menino_dentro: bool,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct GeometriaPatch {
    pub colunas: i16,
    pub linhas: i16,
    /// Pixels por patch em x.
    pub passo_x: i16,
    /// Pixels por patch em y.
    pub passo_y: i16,
    /// Deslocamento em x por patch em y (−10).
    pub inclinacao: i16,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct CoresColonia {
    pub preta: u8,
    pub ambas: u8,
    pub vermelha: u8,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Barras {
    pub x_preta: i16,
    pub x_vermelha: i16,
    pub largura: i16,
    pub base_y: i16,
    pub cor_preta: u8,
    pub cor_vermelha: u8,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct EstiloCursor {
    pub contorno: Option<u8>,
    pub preenchimento: Option<u8>,
}

/// Sprite com um bitmap por quadro (`base + quadro`) e deslocamento por quadro.
#[derive(Clone, Debug, Deserialize)]
pub struct SpriteQuadros {
    pub base: u16,
    /// Maior quadro carregado (`base` … `base + ultimo_quadro`, os que existirem no banco). O
    /// menino vai até 903 (porta) [código: `_DrawSimKid` desenha qualquer quadro ≥ 0].
    #[serde(default = "ultimo_quadro_padrao")]
    pub ultimo_quadro: u16,
    /// Quadro (em texto) → (dx, dy); quadros ausentes não têm deslocamento.
    pub deslocamentos: HashMap<String, [i16; 2]>,
    /// Somado a `deslocamentos`: diferença medida na bancada que o código não explica.
    #[serde(default)]
    pub ajuste_medido: HashMap<String, [i16; 2]>,
}

fn ultimo_quadro_padrao() -> u16 {
    199
}

impl SpriteQuadros {
    fn deslocamento(&self, quadro: i16) -> (i16, i16) {
        let q = quadro.to_string();
        let [dx, dy] = self.deslocamentos.get(&q).copied().unwrap_or([0, 0]);
        let [ax, ay] = self.ajuste_medido.get(&q).copied().unwrap_or([0, 0]);
        (dx + ax, dy + ay)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Cortador {
    pub base: u16,
    pub parado: [i16; 2],
    pub direita: [i16; 2],
    pub esquerda: [i16; 2],
}

#[derive(Clone, Debug, Deserialize)]
pub struct Chuva {
    pub id: u16,
    pub gotas: u16,
    pub x_min: i16,
    pub x_faixa: i16,
    pub y_faixa: i16,
}

/// Rodapé de mensagem do quintal (`_UpdateYardMessage`, `1010:ac0e`): o texto da faixa
/// (`_mapMessage`, com o mesmo prazo) escrito no objeto `objeto` da janela `janela`, na fonte `fonte`,
/// com as cores do objeto; sem mensagem, o retângulo é pintado com a cor de fundo.
#[derive(Clone, Debug, Deserialize)]
pub struct MensagemQuintal {
    pub janela: u16,
    pub objeto: String,
    pub fonte: usize,
}

/// Balão de fala do menino (`_DrawSimKid` + `_MakeBalloon`; ver o comentário em data/quintal_mapa.json).
#[derive(Clone, Debug, Deserialize)]
pub struct BalaoMenino {
    pub lista: u16,
    pub fonte: usize,
    pub dx: i16,
    pub dy: i16,
    pub cor_tinta: u8,
    pub cor_fundo: u8,
    /// Peças 8×8 por nome: 8 linhas de 8 caracteres ('.', '#', 'o').
    pub pecas: HashMap<String, Vec<String>>,
}

/// Enxame da revoada (`_DrawSwarm`): bitmap de cada formiga preta e vermelha.
#[derive(Clone, Debug, Deserialize)]
pub struct Enxame {
    pub preta: u16,
    pub vermelha: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Sprites {
    pub menino: SpriteQuadros,
    pub cachorro: SpriteQuadros,
    pub gato: SpriteQuadros,
    pub passaro: SpriteQuadros,
    pub cortador: Cortador,
    pub placa: BitmapPosto,
    pub chuva: Chuva,
    /// Ausente num mod anterior à API 0.18: o enxame não é desenhado.
    #[serde(default)]
    pub enxame: Option<Enxame>,
}

/// Ações da janela do quintal (botões da faixa 35, teclas).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcaoQuintal {
    /// Abre a janela no modo atual e a traz para a frente (botão Yard, Ctrl+M).
    Abrir,
    /// Só fecha a janela (para mods; o original não tem botão assim).
    Fechar,
    /// Botão Patch (`_YardToMap`): o quintal vira a janela do mapa da superfície, no mesmo lugar,
    /// e a faixa volta à 34. Quem faz é a interface (o gerenciador de janelas).
    ParaMapa,
    /// Botão House: modo 0 ↔ 1; de 2 ou 3 vai para 0.
    AlternarCasa,
    /// Ctrl+Y / Ctrl+5 / Ctrl+6 / Ctrl+7 (e os botões Strat/Graphic): modo 0–3, abrindo a janela.
    ModoQuintal,
    ModoCasa,
    ModoEstrategico,
    ModoGrafico,
    /// Botão PLACE: o próximo clique num patch coloca uma rainha guardada.
    ColocarRainha,
    /// Botões Automatic / Manual (`_LayDownQueenMode`).
    RainhasAutomatico,
    RainhasManual,
    /// Botão MATE (`ribMate`): arma a revoada preta (`rules::quintal::botao_revoada`).
    Revoada,
}

/// Tecla → ação (mesmas regras de `navegacao::Atalho`).
#[derive(Clone, Debug, Deserialize)]
pub struct AtalhoQuintal {
    pub tecla: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    pub acao: AcaoQuintal,
}

/// Conteúdo de data/quintal_mapa.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigQuintalMapa {
    pub janela: u16,
    pub faixa: u16,
    /// Contorno externo da janela na área cliente da janela principal.
    pub externo: Caixa,
    pub altura_titulo: i16,
    pub cor_linha: u8,
    pub cor_titulo: u8,
    pub cor_botao: u8,
    pub cor_fundo: u8,
    pub titulos: Vec<String>,
    /// `_mapTileRect` relativo à área cliente da janela 25.
    pub origem_patches: [i16; 2],
    pub patch: GeometriaPatch,
    pub poligono_cursor: [[i16; 2]; 4],
    pub poligono_colonia: [[i16; 2]; 4],
    pub ajuste_cursor_modos_0_1: i16,
    pub modos: Vec<ModoQuintal>,
    pub rodape: BitmapPosto,
    pub cores_colonia: CoresColonia,
    pub barras: Barras,
    pub cursor: Vec<EstiloCursor>,
    pub transparente: u8,
    pub sprites: Sprites,
    pub balao_menino: BalaoMenino,
    /// Mensagem da faixa repetida no rodapé do quintal (`_UpdateYardMessage`, API 0.18; ausente: nada).
    #[serde(default)]
    pub mensagem: Option<MensagemQuintal>,
    /// Botão da faixa 34 que abre o quintal.
    pub botao_abrir: String,
    /// Botões da faixa 35 → ação.
    pub botoes_faixa: HashMap<String, AcaoQuintal>,
    /// Botão da faixa 35 selecionado em cada modo.
    pub botao_do_modo: Vec<String>,
    /// [automático, manual] (`_LayDownQueenMode` 1 e 0).
    pub botoes_rainhas: [String; 2],
    pub teclas: Vec<AtalhoQuintal>,
    /// Números da faixa 35 (`_DrawYardData`).
    pub dados_faixa: DadosFaixa,
}

/// Objetos da faixa 35 onde `_DrawYardData` escreve, e a fonte.
#[derive(Clone, Debug, Deserialize)]
pub struct DadosFaixa {
    pub fonte: u8,
    pub rainhas: String,
    pub colonias_pretas: String,
    pub colonias_vermelhas: String,
}

impl ConfigQuintalMapa {
    pub fn de_json(texto: &str) -> Result<ConfigQuintalMapa, String> {
        let c: ConfigQuintalMapa = serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_CONFIG}: {e}"))?;
        c.validar()?;
        Ok(c)
    }

    pub fn carregar(r: &Recursos) -> Result<ConfigQuintalMapa, String> {
        let c: ConfigQuintalMapa = r.dados(ARQUIVO_CONFIG)?;
        c.validar()?;
        Ok(c)
    }

    fn validar(&self) -> Result<(), String> {
        if self.modos.len() != 4 || self.cursor.len() != 4 || self.botao_do_modo.len() != 4 {
            return Err(format!("{ARQUIVO_CONFIG}: 'modos', 'cursor' e 'botao_do_modo' precisam de 4 itens (um por _YardMode)"));
        }
        let p = self.patch;
        if p.colunas <= 0 || p.linhas <= 0 || p.passo_x <= 0 || p.passo_y <= 0 {
            return Err(format!("{ARQUIVO_CONFIG}: 'patch' precisa de colunas, linhas e passos positivos"));
        }
        Ok(())
    }

    /// Ação da tecla, se houver.
    pub fn acao_da_tecla(&self, e: &EventoTecla) -> Option<AcaoQuintal> {
        let EventoTecla::Tecla { nome, ctrl, shift, alt } = e;
        self.teclas
            .iter()
            .find(|a| a.tecla.eq_ignore_ascii_case(nome) && a.ctrl == *ctrl && a.shift == *shift && a.alt == *alt)
            .map(|a| a.acao)
    }

    /// Área cliente da janela 25 (na área cliente da janela principal).
    pub fn cliente(&self) -> Retangulo {
        let [x, y, w, h] = self.externo;
        Retangulo { esq: x + 1, topo: y + 1 + self.altura_titulo + 1, dir: x + w - 1, base: y + h - 1 }
    }

    /// `_mapTileRect` (canto da origem dos patches) na área cliente da janela principal.
    pub fn origem(&self) -> (i32, i32) {
        let c = self.cliente();
        (i32::from(c.esq + self.origem_patches[0]), i32::from(c.topo + self.origem_patches[1]))
    }

    /// Canto (x, y) do patch, antes de somar o polígono (quintal.md §1).
    fn canto_patch(&self, px: i16, py: i16) -> (i32, i32) {
        let (ox, oy) = self.origem();
        let g = self.patch;
        (ox + i32::from(px * g.passo_x + py * g.inclinacao), oy + i32::from(py * g.passo_y))
    }

    /// Vértices do polígono do patch.
    fn poligono(&self, pol: &[[i16; 2]; 4], px: i16, py: i16, dx: i32) -> [(i32, i32); 4] {
        let (cx, cy) = self.canto_patch(px, py);
        pol.map(|[x, y]| (cx + i32::from(x) + dx, cy + i32::from(y)))
    }

    /// Patch sob o ponto (x, y) da área cliente principal (`_YardArea`, divisão do C), se no quintal.
    pub fn patch_em(&self, x: i32, y: i32) -> Option<(i16, i16)> {
        if !contem(self.cliente(), x, y) {
            return None;
        }
        let (ox, oy) = self.origem();
        let [x0, y0] = self.poligono_cursor[0];
        let g = self.patch;
        // Divisão inteira com truncamento para zero, como no C.
        let py = (y - oy - i32::from(y0)) / i32::from(g.passo_y);
        let px = (py * -i32::from(g.inclinacao) + x - ox - i32::from(x0)) / i32::from(g.passo_x);
        ((0..i32::from(g.colunas)).contains(&px) && (0..i32::from(g.linhas)).contains(&py)).then_some((px as i16, py as i16))
    }

    /// O ponto cai dentro do contorno externo da janela.
    pub fn contem(&self, x: i32, y: i32) -> bool {
        let [ex, ey, w, h] = self.externo;
        contem(Retangulo { esq: ex, topo: ey, dir: ex + w, base: ey + h }, x, y)
    }
}

fn contem(r: Retangulo, x: i32, y: i32) -> bool {
    x >= i32::from(r.esq) && x < i32::from(r.dir) && y >= i32::from(r.topo) && y < i32::from(r.base)
}

/// Estado da janela na interface (não vai para o save).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JanelaQuintal {
    /// Aberta (na interface, espelho do gerenciador de janelas).
    pub aberta: bool,
    /// A faixa 35 está no lugar da 34: o quintal foi a última ativada entre ele e a janela de edição
    /// (`WM_NCACTIVATE`, janelas.md §8). A ordem das janelas é do gerenciador.
    pub na_frente: bool,
    /// Esperando o clique do PLACE.
    pub colocando_rainha: bool,
}

/// O que um clique na janela do quintal causou (para o terminal/registro).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RespostaQuintal {
    Nada,
    /// `_YardArea`: patch escolhido (`_YMapPnt`).
    Escolheu(i16, i16),
    Troca(ResultadoTroca),
    Rainha(ResultadoRainha),
}

impl JanelaQuintal {
    /// Aplica uma ação: modos, abrir/fechar, PLACE e Automatic/Manual.
    pub fn aplicar(&mut self, a: AcaoQuintal, e: &mut Estado) {
        let modo = &mut e.vistas.modo_quintal;
        let novo = match a {
            AcaoQuintal::Abrir => None,
            AcaoQuintal::Fechar | AcaoQuintal::ParaMapa => {
                *self = JanelaQuintal::default();
                return;
            }
            AcaoQuintal::AlternarCasa => Some(if *modo < 2 { *modo ^ 1 } else { 0 }),
            AcaoQuintal::ModoQuintal => Some(0),
            AcaoQuintal::ModoCasa => Some(1),
            AcaoQuintal::ModoEstrategico => Some(2),
            AcaoQuintal::ModoGrafico => Some(3),
            AcaoQuintal::ColocarRainha => {
                // _PlaceQueenInYard: sem rainhas guardadas, só a mensagem (som de erro).
                self.colocando_rainha = e.preta.rainhas_guardadas >= 1;
                None
            }
            AcaoQuintal::RainhasAutomatico => {
                e.controles.modo_por_rainha = 1;
                return;
            }
            AcaoQuintal::RainhasManual => {
                e.controles.modo_por_rainha = 0;
                return;
            }
            // Precisa das tabelas do quintal: quem chama é `Partida::botao_revoada`.
            AcaoQuintal::Revoada => return,
        };
        if let Some(m) = novo {
            *modo = m;
        }
        self.aberta = true;
        self.na_frente = true;
    }
}

/// Um clique esquerdo (simples ou duplo) na área cliente da janela do quintal, com a partida:
/// PLACE pendente coloca a rainha; senão escolhe o patch e, no duplo clique, troca (`_XferPatch`).
pub fn clicar(j: &mut JanelaQuintal, cfg: &ConfigQuintalMapa, p: &mut Partida, x: i32, y: i32, duplo: bool) -> Result<RespostaQuintal, String> {
    j.na_frente = true;
    let Some((px, py)) = cfg.patch_em(x, y) else {
        return Ok(RespostaQuintal::Nada);
    };
    // _YardArea: _YMapPnt = patch (a navegação é a dona dos pontos lembrados). O clique que encerra
    // o PLACE também chega aqui, dentro ou fora do alcance (bancada: acoes/place e place_fora).
    p.nav.pontos[0] = (i32::from(px), i32::from(py));
    p.estado.vistas.ponto_quintal = (px, py);
    if j.colocando_rainha {
        j.colocando_rainha = false;
        let r = troca_patch::colocar_rainha(&mut p.estado, px, py);
        p.processar_eventos_do_estado();
        return Ok(RespostaQuintal::Rainha(r));
    }
    if !duplo {
        return Ok(RespostaQuintal::Escolheu(px, py));
    }
    let geracao = p.regras.geracao.as_ref().ok_or("tabelas de geração não carregadas (troca de patch)")?;
    let r = troca_patch::trocar_patch(&mut p.estado, geracao, &p.cfg_simulacao)?;
    if let ResultadoTroca::Trocou { .. } = r {
        // _RandWorld passo 18: pontos lembrados; o port leva a vista até a amarela [decisão do port].
        let e = &p.estado;
        let par = |(x, y): (i16, i16)| (i32::from(x), i32::from(y));
        p.nav.pontos = [par(e.vistas.ponto_quintal), par(e.vistas.ponto_superficie), par(e.preta.vista_ninho), par(e.vermelha.vista_ninho)];
        p.estado.eventos.push(Evento::CentralizarNaAmarela);
    }
    p.processar_eventos_do_estado();
    Ok(RespostaQuintal::Troca(r))
}

/// Bitmaps e layout da faixa 35, carregados uma vez.
pub struct DesenhoQuintal {
    pub cfg: ConfigQuintalMapa,
    faixa: Janela,
    origem_faixa: (i16, i16),
    bitmaps: HashMap<u16, Bitmap>,
    /// Falas do menino (lista 4/`balao_menino.lista`).
    falas_menino: Vec<Vec<u8>>,
    /// Retângulo do rodapé de mensagem (relativo ao canto externo da janela) e a cor do objeto.
    mensagem: Option<(Retangulo, u8)>,
}

/// Bit "selecionado" das flags de objeto (janela.md).
const FLAG_SELECIONADO: u16 = 4;

impl DesenhoQuintal {
    pub fn carregar(cfg: ConfigQuintalMapa, recursos: &Recursos, interface: &Interface) -> Result<DesenhoQuintal, String> {
        let faixa = carregar_janela(recursos, cfg.faixa)?;
        // A faixa 35 ocupa o lugar da 34, com o objeto 0 no mesmo canto [medido: com a posição da
        // tabela para a janela 35, tudo sairia 1 pixel à direita da captura `full_yard.png`].
        let id34 = interface.moldura.faixa.janela;
        let j34 = carregar_janela(recursos, id34)?;
        let origem_faixa = interface.posicionar(id34, &j34).ok_or(format!("janela {id34} sem posição"))?.origem;
        let mut ids: Vec<u16> = cfg.modos.iter().flat_map(|m| m.bitmaps.iter().map(|b| b.id)).collect();
        ids.push(cfg.rodape.id);
        let s = &cfg.sprites;
        for sp in [&s.menino, &s.cachorro, &s.gato, &s.passaro] {
            ids.extend((0..=sp.ultimo_quadro).map(|q| sp.base.wrapping_add(q)));
        }
        ids.extend([s.cortador.base, s.cortador.base + 1, s.cortador.base + 2, s.placa.id, s.chuva.id]);
        ids.extend(s.enxame.iter().flat_map(|x| [x.preta, x.vermelha]));
        let mut bitmaps = HashMap::new();
        for id in ids {
            if let Ok(b) = recursos.objeto(Chave { tipo: TIPO_BITMAP, id }).and_then(|d| Bitmap::decodificar(&d)) {
                bitmaps.insert(id, b);
            }
        }
        for b in cfg.modos.iter().flat_map(|m| m.bitmaps.iter()) {
            if !bitmaps.contains_key(&b.id) {
                return Err(format!("{ARQUIVO_CONFIG}: bitmap {} não existe no banco", b.id));
            }
        }
        let falas_menino = super::avisos::linhas_do_texto(recursos, cfg.balao_menino.lista);
        let mensagem = match &cfg.mensagem {
            Some(m) => {
                let j = carregar_janela(recursos, m.janela)?;
                let base = j.retangulo();
                let o = j.objetos.iter().find(|o| o.nome == m.objeto).ok_or(format!("janela {}: objeto {} não existe", m.janela, m.objeto))?;
                let r = o.retangulo;
                let rel = Retangulo { esq: r.esq - base.esq, topo: r.topo - base.topo, dir: r.dir - base.esq, base: r.base - base.topo };
                Some((rel, o.cor))
            }
            None => None,
        };
        Ok(DesenhoQuintal { cfg, faixa, origem_faixa, bitmaps, falas_menino, mensagem })
    }

    /// Desenha a janela 25 inteira (moldura e conteúdo do `_YardMode` do estado).
    pub fn desenhar_janela(&self, q: &mut Quadro, e: &Estado) {
        let c = &self.cfg;
        let [ex, ey, w, h] = c.externo;
        let externo = Retangulo { esq: ex, topo: ey, dir: ex + w, base: ey + h };
        q.preencher(externo, c.cor_fundo, c.cor_fundo);
        q.contorno(Retangulo { dir: externo.dir - 1, base: externo.base - 1, ..externo }, c.cor_linha);
        // Título: caixa do menu de sistema, faixa azul e linha embaixo (o texto não é desenhado).
        let t = Retangulo { esq: ex + 1, topo: ey + 1, dir: ex + w - 1, base: ey + 1 + c.altura_titulo };
        q.preencher(t, c.cor_titulo, c.cor_titulo);
        q.preencher(Retangulo { dir: t.esq + c.altura_titulo, ..t }, c.cor_botao, c.cor_botao);
        q.preencher(Retangulo { esq: t.esq + c.altura_titulo, dir: t.esq + c.altura_titulo + 1, ..t }, c.cor_linha, c.cor_linha);
        q.preencher(Retangulo { topo: t.base, base: t.base + 1, ..t }, c.cor_linha, c.cor_linha);

        let cliente = c.cliente();
        q.recorte = Some(cliente);
        let modo_n = e.vistas.modo_quintal.clamp(0, 3) as usize;
        let modo = &c.modos[modo_n];
        let (cx, cy) = (i32::from(cliente.esq), i32::from(cliente.topo));
        for b in &modo.bitmaps {
            if let Some(bm) = self.bitmaps.get(&b.id) {
                q.bitmap(bm, cx + i32::from(b.x), cy + i32::from(b.y));
            }
        }
        match modo.conteudo {
            Conteudo::Bichos => self.desenhar_bichos(q, e, modo.mostra_menino_dentro),
            Conteudo::Colonias => self.desenhar_colonias(q, e),
            Conteudo::Barras => self.desenhar_barras(q, e),
        }
        if let Some(bm) = self.bitmaps.get(&c.rodape.id) {
            q.bitmap(bm, cx + i32::from(c.rodape.x), cy + i32::from(c.rodape.y));
        }
        // Cursor do patch atual (_DrawYardCursor).
        let (px, py) = e.ambiente.patch;
        let dx = if modo_n < 2 { i32::from(c.ajuste_cursor_modos_0_1) } else { 0 };
        let pts = c.poligono(&c.poligono_cursor, px, py, dx);
        let estilo = c.cursor[modo_n];
        if let Some(cor) = estilo.preenchimento {
            poligono(q, &pts, cor);
        }
        if let Some(cor) = estilo.contorno {
            contorno_poligono(q, &pts, cor);
        }
        q.recorte = None;
    }

    fn sprite(&self, q: &mut Quadro, id: u16, x: i32, y: i32) {
        let Some(bm) = self.bitmaps.get(&id) else { return };
        let (ox, oy) = self.cfg.origem();
        for j in 0..bm.altura {
            for i in 0..bm.largura {
                let v = bm.pixels[j * bm.largura + i];
                if v != self.cfg.transparente {
                    q.ponto(ox + x + i as i32, oy + y + j as i32, v);
                }
            }
        }
    }

    /// `_UpdateYardMessage`: o texto da faixa (`texto`, já na codificação da fonte; `None` = sem
    /// mensagem) no rodapé do quintal.
    pub fn desenhar_mensagem(&self, q: &mut Quadro, interface: &Interface, texto: Option<&[u8]>) {
        let (Some((r, cor)), Some(m)) = (self.mensagem, &self.cfg.mensagem) else { return };
        let [ex, ey, _, _] = self.cfg.externo;
        let r = Retangulo { esq: r.esq + ex, topo: r.topo + ey, dir: r.dir + ex, base: r.base + ey };
        let c = interface.cores.get(cor as usize).copied().unwrap_or(crate::formats::janela::Cor { frente: 0, fundo: 15, fundo2: 15 });
        q.preencher(r, c.fundo, c.fundo);
        let (Some(t), Some(Some(f))) = (texto, interface.fontes.get(m.fonte)) else { return };
        let antes = q.recorte.replace(r);
        q.texto(r, f, t, 3, c.frente, c.fundo);
        q.recorte = antes;
    }

    /// Balão de fala do menino por cima do quintal (modos 0 e 1, menino desenhado e `_BoyMessOn`).
    /// A fonte é a `balao_menino.fonte` da interface (quem chama a escolhe).
    pub fn desenhar_balao_do_menino(&self, q: &mut Quadro, e: &Estado, fonte: Option<&Fonte>) {
        let modo = e.vistas.modo_quintal;
        let m = &e.menino;
        let menino_desenhado = modo <= 1 && !((modo == 0 && m.presente == 0) || m.quadro < 0);
        if !menino_desenhado || m.mensagem_ligada == 0 {
            return;
        }
        let fala = usize::try_from(m.mensagem_offset).ok().and_then(|i| self.falas_menino.get(i));
        let (Some(f), Some(texto)) = (fonte, fala) else { return };
        let b = &self.cfg.balao_menino;
        let forma = super::baloes::FormaBalao::medir(texto, f);
        let (dx, _) = self.cfg.sprites.menino.deslocamento(m.quadro);
        let (ox, oy) = self.cfg.origem();
        let x0 = ox + i32::from(m.x + dx) + i32::from(b.dx);
        let y0 = oy + i32::from(m.y) + i32::from(b.dy) - forma.altura;
        let antes = q.recorte.replace(self.cfg.cliente());
        let pecas = super::baloes::PecasBalao { pecas: &b.pecas, cor_tinta: b.cor_tinta, cor_fundo: b.cor_fundo };
        super::baloes::desenhar(q, &forma, f, pecas, x0, y0);
        q.recorte = antes;
    }

    /// Modos 0 e 1 (`_Draw_SimYard`): gato, cachorro, menino, cortador, pássaro, placa e chuva.
    fn desenhar_bichos(&self, q: &mut Quadro, e: &Estado, menino_dentro: bool) {
        let s = &self.cfg.sprites;
        let b = &e.bichos;
        let m = &e.menino;
        let quadro_de = |sp: &SpriteQuadros, base_x: i16, base_y: i16, quadro: i16| {
            let (dx, dy) = sp.deslocamento(quadro);
            (sp.base.wrapping_add(quadro as u16), i32::from(base_x + dx), i32::from(base_y + dy))
        };
        if b.gato_ligado != 0 {
            let (id, x, y) = quadro_de(&s.gato, b.gato_x, b.gato_y, b.gato_quadro);
            self.sprite(q, id, x, y);
        }
        if e.ambiente.casa_a_venda == 0 {
            let (id, x, y) = quadro_de(&s.cachorro, b.cachorro_x, b.cachorro_y, b.cachorro_quadro);
            self.sprite(q, id, x, y);
        }
        let menino_visivel = m.quadro >= 0 && (menino_dentro || m.presente != 0);
        // Qualquer quadro ≥ 0, inclusive 200 (poltrona), 201 (geladeira) e 900–903 (porta), sem
        // deslocamento a partir de 200 (_DrawSimKid).
        if menino_visivel {
            let (id, x, y) = quadro_de(&s.menino, m.x, m.y, m.quadro);
            self.sprite(q, id, x, y);
        }
        // _DrawMower: parado ao lado da casa, ou nas mãos do menino virado para os lados.
        let c = &s.cortador;
        let cortador = if m.presente == 3 || m.presente == 4 {
            match m.quadro {
                103..=105 => Some((c.base + 1, m.x + c.direita[0], m.y + c.direita[1])),
                109..=111 => Some((c.base + 2, m.x + c.esquerda[0], m.y + c.esquerda[1])),
                _ => None,
            }
        } else {
            Some((c.base, c.parado[0], c.parado[1]))
        };
        if let Some((id, x, y)) = cortador {
            self.sprite(q, id, i32::from(x), i32::from(y));
        }
        self.desenhar_enxame(q, e);
        if b.passaro_ligado != 0 {
            let (id, x, y) = quadro_de(&s.passaro, b.passaro_x, b.passaro_y, b.passaro_quadro);
            self.sprite(q, id, x, y);
        }
        if e.ambiente.casa_a_venda != 0 {
            self.sprite(q, s.placa.id, i32::from(s.placa.x), i32::from(s.placa.y));
        }
        if e.ambiente.chovendo != 0 {
            // O original sorteia as gotas com o gerador S a cada redesenho; o port usa posições
            // pseudoaleatórias próprias (divergência registrada), sem tocar no estado.
            let ch = &s.chuva;
            let mut v = e.partida.tempo.wrapping_mul(2_654_435_761) | 1;
            for _ in 0..ch.gotas {
                v ^= v << 13;
                v ^= v >> 17;
                v ^= v << 5;
                let x = i32::from(ch.x_min) + (v % ch.x_faixa.max(1) as u32) as i32;
                let y = ((v >> 16) % ch.y_faixa.max(1) as u32) as i32;
                self.sprite(q, ch.id, x, y);
            }
        }
    }

    /// `_DrawSwarm` (`1010:bb0e`), com `_MapMode` = 0: cada formiga do enxame (até
    /// `min(_SwarmCnt, 16, _QueenStorage)` de cada cor) em `_SwarmCent` + (`_SwarmX[i]`, `_SwarmY[i]`).
    /// As posições e os sorteios que as movem são do estado (`rules::desenho::desenhar_enxame`).
    fn desenhar_enxame(&self, q: &mut Quadro, e: &Estado) {
        let Some(ids) = &self.cfg.sprites.enxame else { return };
        if e.vistas.modo_mapa != 0 {
            return;
        }
        let (cx, cy) = (i32::from(e.ambiente.enxame_centro_x), i32::from(e.ambiente.enxame_centro_y));
        for (c, id) in [(&e.preta, ids.preta), (&e.vermelha, ids.vermelha)] {
            let n = c.enxame_contagem.min(crate::rules::desenho::ENXAME_MAXIMO).min(c.rainhas_guardadas).max(0) as usize;
            for i in 0..n {
                let (dx, dy) = (c.enxame_x.0[i] as i8, c.enxame_y.0[i] as i8);
                self.sprite(q, id, cx + i32::from(dx), cy + i32::from(dy));
            }
        }
    }

    /// Modo 2 (`_DrawSimColonies`).
    fn desenhar_colonias(&self, q: &mut Quadro, e: &Estado) {
        let c = &self.cfg;
        for px in 0..c.patch.colunas {
            for py in 0..c.patch.linhas {
                let i = (px * c.patch.linhas + py) as usize;
                let b = e.preta.populacao_quintal.0.get(i).copied().unwrap_or(0);
                let r = e.vermelha.populacao_quintal.0.get(i).copied().unwrap_or(0);
                let cor = match (b != 0, r != 0) {
                    (true, false) => c.cores_colonia.preta,
                    (true, true) => c.cores_colonia.ambas,
                    (false, true) => c.cores_colonia.vermelha,
                    (false, false) => continue,
                };
                let pts = c.poligono(&c.poligono_colonia, px, py, 0);
                poligono(q, &pts, cor);
                contorno_poligono(q, &pts, cor);
            }
        }
    }

    /// Modo 3 (`_DrawColonyBars`).
    fn desenhar_barras(&self, q: &mut Quadro, e: &Estado) {
        let c = &self.cfg;
        let b = c.barras;
        for px in 0..c.patch.colunas {
            for py in 0..c.patch.linhas {
                let i = (px * c.patch.linhas + py) as usize;
                let (cx, cy) = c.canto_patch(px, py);
                let base = cy + i32::from(b.base_y);
                for (pop, dx, cor) in [
                    (e.preta.populacao_quintal.0.get(i).copied().unwrap_or(0), b.x_preta, b.cor_preta),
                    (e.vermelha.populacao_quintal.0.get(i).copied().unwrap_or(0), b.x_vermelha, b.cor_vermelha),
                ] {
                    let h = (i32::from(pop) + 3) >> 2;
                    if h == 0 {
                        continue;
                    }
                    let x = cx + i32::from(dx);
                    let r = Retangulo { esq: x as i16, topo: (base - h) as i16, dir: (x + i32::from(b.largura)) as i16, base: base as i16 };
                    q.preencher(r, cor, cor);
                }
            }
        }
    }

    /// Desenha a faixa 35 no lugar da 34, com o botão do modo e Automatic/Manual selecionados.
    pub fn desenhar_faixa(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface, e: &Estado) -> Vec<String> {
        let c = &self.cfg;
        let f = &interface.moldura.faixa;
        q.preencher(Retangulo { esq: 0, topo: 0, dir: q.largura as i16, base: f.altura }, f.fundo, f.fundo);
        let mut faixa = self.faixa.clone();
        let modo = e.vistas.modo_quintal.clamp(0, 3) as usize;
        let rainha = if e.controles.modo_por_rainha == 1 { 0 } else { 1 };
        let grupos: [(&[String], &str); 2] = [(&c.botao_do_modo, &c.botao_do_modo[modo]), (&c.botoes_rainhas, &c.botoes_rainhas[rainha])];
        for ob in faixa.objetos.iter_mut() {
            for (grupo, escolhido) in grupos {
                if grupo.contains(&ob.nome) {
                    if ob.nome == escolhido {
                        ob.flags |= FLAG_SELECIONADO;
                    } else {
                        ob.flags &= !FLAG_SELECIONADO;
                    }
                }
            }
        }
        let pendentes = desenhar_objetos(q, recursos, interface, &faixa, self.origem_faixa, None);
        // _DrawYardData (fonte 3): rainhas guardadas e colônias pretas e vermelhas no quintal.
        let base = faixa.retangulo();
        let (dx, dy) = (self.origem_faixa.0 - base.esq, self.origem_faixa.1 - base.topo);
        let valores = [
            (c.dados_faixa.rainhas.as_str(), e.preta.rainhas_guardadas.max(0)),
            (c.dados_faixa.colonias_pretas.as_str(), e.preta.total_colonias),
            (c.dados_faixa.colonias_vermelhas.as_str(), e.vermelha.total_colonias),
        ];
        if let Some(Some(fonte)) = interface.fontes.get(usize::from(c.dados_faixa.fonte)) {
            for (nome, v) in valores {
                let Some(ob) = faixa.objetos.iter().find(|o| o.nome == nome) else { continue };
                let r = ob.retangulo;
                let r = Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy };
                let cor = interface.cores.get(usize::from(ob.cor)).copied();
                let (frente, fundo) = cor.map(|k| (k.frente, k.fundo)).unwrap_or((0, 15));
                q.preencher(r, fundo, fundo);
                q.texto(r, fonte, v.to_string().as_bytes(), 0, frente, fundo);
            }
        }
        pendentes
    }

    /// Índice do objeto visível da faixa sob o ponto (o último do layout que o contém; 0 se nenhum).
    /// Para a ajuda por contexto: id do objeto = janela × 256 + índice.
    pub fn indice_da_faixa_em(&self, x: i32, y: i32) -> usize {
        let base = self.faixa.retangulo();
        let (dx, dy) = (i32::from(self.origem_faixa.0 - base.esq), i32::from(self.origem_faixa.1 - base.topo));
        self.faixa.objetos.iter().enumerate().skip(1).rev().find(|(_, o)| o.visivel() && contem(o.retangulo, x - dx, y - dy)).map_or(0, |(i, _)| i)
    }

    /// Nomes dos objetos visíveis da faixa 35 sob o ponto (x, y).
    pub fn objetos_da_faixa_em(&self, x: i32, y: i32) -> Vec<&str> {
        let base = self.faixa.retangulo();
        let (dx, dy) = (i32::from(self.origem_faixa.0 - base.esq), i32::from(self.origem_faixa.1 - base.topo));
        self.faixa
            .objetos
            .iter()
            .skip(1)
            .filter(|o| o.visivel() && contem(o.retangulo, x - dx, y - dy))
            .map(|o| o.nome.as_str())
            .collect()
    }

    /// Retângulo (na área cliente) do objeto `nome` da faixa 35.
    pub fn retangulo_da_faixa(&self, nome: &str) -> Option<Retangulo> {
        let base = self.faixa.retangulo();
        let (dx, dy) = (self.origem_faixa.0 - base.esq, self.origem_faixa.1 - base.topo);
        let r = self.faixa.objetos.iter().find(|o| o.nome == nome)?.retangulo;
        Some(Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy })
    }

    /// Ação do botão da faixa 35 sob o ponto, se houver.
    pub fn acao_da_faixa_em(&self, x: i32, y: i32) -> Option<AcaoQuintal> {
        self.objetos_da_faixa_em(x, y).iter().find_map(|n| self.cfg.botoes_faixa.get(*n).copied())
    }
}

/// Pixels de um quadrilátero convexo pelo critério do centro do pixel (`Polygon` do GDI preenche o
/// interior; as bordas vêm de `contorno_poligono`).
pub fn pixels_do_poligono(pts: &[(i32, i32); 4]) -> Vec<(i32, i32)> {
    let y0 = pts.iter().map(|p| p.1).min().unwrap_or(0);
    let y1 = pts.iter().map(|p| p.1).max().unwrap_or(0);
    let mut out = Vec::new();
    for y in y0..y1 {
        let yc = f64::from(y) + 0.5;
        let mut xs = Vec::new();
        for k in 0..4 {
            let (a, b) = (pts[k], pts[(k + 1) % 4]);
            let (ya, yb) = (f64::from(a.1), f64::from(b.1));
            if (ya <= yc && yc < yb) || (yb <= yc && yc < ya) {
                xs.push(f64::from(a.0) + (yc - ya) * f64::from(b.0 - a.0) / (yb - ya));
            }
        }
        if xs.len() < 2 {
            continue;
        }
        let (xa, xb) = (xs.iter().cloned().fold(f64::MAX, f64::min), xs.iter().cloned().fold(f64::MIN, f64::max));
        let mut x = (xa - 0.5).ceil() as i32;
        while f64::from(x) + 0.5 < xb {
            out.push((x, y));
            x += 1;
        }
    }
    out
}

fn poligono(q: &mut Quadro, pts: &[(i32, i32); 4], cor: u8) {
    for (x, y) in pixels_do_poligono(pts) {
        q.ponto(x, y, cor);
    }
}

/// Contorno do polígono com linhas de Bresenham (caneta de 1 pixel, extremos incluídos).
fn contorno_poligono(q: &mut Quadro, pts: &[(i32, i32); 4], cor: u8) {
    for k in 0..4 {
        let (mut x0, mut y0) = pts[k];
        let (x1, y1) = pts[(k + 1) % 4];
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let mut err = dx + dy;
        loop {
            q.ponto(x0, y0, cor);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cfg() -> ConfigQuintalMapa {
        ConfigQuintalMapa::de_json(include_str!("../../../data/quintal_mapa.json")).unwrap()
    }

    #[test]
    fn dados_carregam_e_teclas() {
        let c = cfg();
        assert_eq!(c.janela, 25);
        assert_eq!(c.poligono_cursor, [[169, 67], [192, 67], [185, 74], [162, 74]]);
        let t = |nome: &str| EventoTecla::Tecla { nome: nome.into(), ctrl: true, shift: false, alt: false };
        assert_eq!(c.acao_da_tecla(&t("y")), Some(AcaoQuintal::ModoQuintal));
        assert_eq!(c.acao_da_tecla(&t("6")), Some(AcaoQuintal::ModoEstrategico));
        assert_eq!(c.acao_da_tecla(&t("M")), Some(AcaoQuintal::Abrir));
    }

    #[test]
    fn clique_vira_patch_como_yard_area() {
        // quintal.md §1: área cliente em (120, 157) da tela = (120, 119) da área cliente principal;
        // patch (11, 8): y em [147, 157) e x em [397, 425) relativos à origem.
        let c = cfg();
        let (ox, oy) = c.origem();
        assert_eq!((ox, oy), (120, 119));
        assert_eq!(c.patch_em(ox + 397, oy + 147), Some((11, 8)));
        assert_eq!(c.patch_em(ox + 424, oy + 156), Some((11, 8)));
        assert_eq!(c.patch_em(ox + 425, oy + 156), None, "x passa da coluna 11");
        assert_eq!(c.patch_em(ox + 169, oy + 67), Some((0, 0)));
        // Truncamento para zero: a linha acima do patch (0, 0) ainda dá py = 0.
        assert_eq!(c.patch_em(ox + 175, oy + 60), Some((0, 0)));
        assert_eq!(c.patch_em(ox + 175, oy + 58), Some((0, 0)));
        assert_eq!(c.patch_em(ox + 175, oy + 57), None, "−10 / 10 = −1");
        assert_eq!(c.patch_em(5, 5), None, "fora da janela");
    }

    #[test]
    fn modos_e_botoes() {
        let mut j = JanelaQuintal::default();
        let mut e = Estado::vazio();
        j.aplicar(AcaoQuintal::Abrir, &mut e);
        assert!(j.aberta && j.na_frente);
        assert_eq!(e.vistas.modo_quintal, 0);
        j.aplicar(AcaoQuintal::AlternarCasa, &mut e);
        assert_eq!(e.vistas.modo_quintal, 1);
        j.aplicar(AcaoQuintal::ModoGrafico, &mut e);
        assert_eq!(e.vistas.modo_quintal, 3);
        j.aplicar(AcaoQuintal::AlternarCasa, &mut e);
        assert_eq!(e.vistas.modo_quintal, 0, "de 2 ou 3, House vai para 0");
        j.aplicar(AcaoQuintal::ColocarRainha, &mut e);
        assert!(!j.colocando_rainha, "sem rainhas guardadas");
        e.preta.rainhas_guardadas = 1;
        j.aplicar(AcaoQuintal::ColocarRainha, &mut e);
        assert!(j.colocando_rainha);
        j.aplicar(AcaoQuintal::RainhasManual, &mut e);
        assert_eq!(e.controles.modo_por_rainha, 0);
        j.aplicar(AcaoQuintal::Fechar, &mut e);
        assert_eq!(j, JanelaQuintal::default());
    }

    #[test]
    fn poligono_do_patch() {
        let px = pixels_do_poligono(&[(169, 67), (192, 67), (185, 74), (162, 74)]);
        // 7 linhas, cada uma com 23 pixels.
        assert_eq!(px.len(), 7 * 23);
        assert!(px.contains(&(169, 67)) && !px.contains(&(192, 67)));
    }
}
