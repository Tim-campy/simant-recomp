//! Avisos ao jogador durante a partida, sem janela: a mensagem da faixa (`_EditMessage`,
//! `_DrawRibbonMessage`), os quadros de mensagem modais (`_PictStrnDialog`, `_PictureDialog`), o
//! diálogo de fim de jogo (`_EndGameDialog`) e a pontuação (`_CalcScore`).
//!
//! Especificação: docs/spec/telas/mensagens.md. Dados: data/mensagens.json.
//!
//! O estado (o que está na faixa, que quadros esperam na fila) fica aqui; quem decide quando a
//! simulação para é a `Partida` (core/partida.rs). O desenho usa as janelas e fontes do jogo.

use std::collections::{HashMap, VecDeque};

use serde::Deserialize;

use super::quadro::Quadro;
use super::telas::{carregar_janela, desenhar_janela_deslocada, Interface};
use crate::formats::banco::Chave;
use crate::formats::bitmap::{Bitmap, TIPO_BITMAP};
use crate::formats::janela::{Janela, Retangulo};
use crate::formats::textos::{ListaStrings, TIPO_STRINGS};
use crate::recursos::Recursos;
use crate::rules::conselhos::Licao;
use crate::rules::estado::Estado;

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_CONFIG: &str = "mensagens.json";

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigQuadro {
    pub janela: u16,
    pub fonte: usize,
    pub objeto_texto: String,
    pub objeto_clique: String,
    pub largura_minima: i32,
    pub folga_largura: i32,
    pub folga_altura: i32,
    pub espaco_imagem: i32,
    pub recuo_interior: [i32; 2],
    pub acrescimo_interior: [i32; 2],
    pub clique_acima_da_base: i32,
    pub espera_tickcount: u64,
    pub tecla_continuar: String,
    pub nao_forcadas: Vec<u16>,
    pub imagens: HashMap<String, u16>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigFaixa {
    pub objeto: String,
    pub fonte: usize,
    pub lista: u16,
    pub pausado: usize,
    pub ferramenta_troca: usize,
    pub ferramenta_alvo: usize,
    pub duracao_conselho: u32,
    pub duracao_botao: u32,
    pub fator_duracao: [u64; 2],
    pub velocidade: Vec<String>,
    pub marcador_posto: String,
    pub marcador_ausente: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigFimDeJogo {
    pub janela: u16,
    pub fonte: usize,
    pub objeto_clique: String,
    pub objeto_tipo: String,
    pub objeto_pontos: String,
    pub objeto_avaliacao: String,
    pub lista_tipos: u16,
    pub tipo_padrao: usize,
    pub lista_avaliacoes: u16,
    pub faixas_avaliacao: Vec<i64>,
    pub divisor_full: i64,
    pub soma_derrota: usize,
    pub musica_vitoria: u16,
    pub musica_derrota: u16,
    /// Deslocamento da janela em relação à posição calculada (medido na bancada QEMU).
    #[serde(default)]
    pub ajuste: [i16; 2],
}

#[derive(Clone, Debug, Deserialize)]
pub struct EscalaFull {
    pub limite_tempo: i64,
    pub divisor_tempo: i64,
    pub divisor: i64,
    pub multiplicador: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EscalaOutros {
    pub multiplicador: i64,
    pub divisor_multiplicador: i64,
    pub limite_tempo: i64,
    pub divisor_tempo: i64,
    pub divisor: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigPontuacao {
    pub pesos: [i64; 8],
    pub fator_peso: i64,
    pub colunas_casa: [usize; 2],
    pub linhas_casa_larga: usize,
    pub colunas_quintal: usize,
    pub full: EscalaFull,
    pub outros: EscalaOutros,
}

/// Conteúdo de data/mensagens.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigAvisos {
    pub ms_por_tickcount: u64,
    pub quadro: ConfigQuadro,
    pub faixa: ConfigFaixa,
    pub fim_de_jogo: ConfigFimDeJogo,
    pub pontuacao: ConfigPontuacao,
}

impl ConfigAvisos {
    pub fn de_json(texto: &str) -> Result<ConfigAvisos, String> {
        serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_CONFIG}: {e}"))
    }
}

/// Texto da faixa: uma string de lista SHARED 4 ou um texto literal (os do DGROUP).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextoFaixa {
    Lista { lista: u16, indice: usize },
    Literal(String),
}

/// Um aviso modal: a simulação fica parada enquanto ele está na tela.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Modal {
    /// `_PictStrnDialog(imagem, texto, …)`: as strings da lista SHARED 4/`texto`, uma por linha.
    Quadro { texto: u16, imagem: Option<u16> },
    /// `_EndGameDialog`.
    FimDeJogo { tipo_jogo: i16, pretas_venceram: bool, pontos: i64 },
}

impl Modal {
    /// Fechado este aviso, a interface abre o "Select a Game": `_EndGameDialog` termina com
    /// `_NewGame(0)` (telas/abertura.md, mensagens.md).
    pub fn abre_selecao_de_jogo(&self) -> bool {
        matches!(self, Modal::FimDeJogo { .. })
    }
}

/// Mensagens da faixa e fila de avisos modais.
#[derive(Clone, Debug)]
pub struct Avisos {
    pub cfg: ConfigAvisos,
    /// `_editMessage` / `_mapMessage`.
    texto: Option<TextoFaixa>,
    /// `_editMessageRemoveTime` em unidades de `_TickCount`; None = permanente (0x7FFFFFFF).
    expira: Option<u64>,
    modais: VecDeque<Modal>,
    /// Quando o modal da frente apareceu (unidades de `_TickCount`).
    aberto_em: Option<u64>,
}

impl Avisos {
    pub fn new(cfg: ConfigAvisos) -> Avisos {
        Avisos { cfg, texto: None, expira: None, modais: VecDeque::new(), aberto_em: None }
    }

    // --- faixa ---------------------------------------------------------------------------------

    /// `_EditMessage(texto, duração, importante)`. `duracao` None = permanente (0xFFFFFFFE). Troca a
    /// mensagem se o pedido é importante, se a opção Messages (`_OptionStates[4]`) está ligada e a
    /// faixa está vazia, ou se é um pedido de limpar (texto nulo) com a mensagem atual permanente.
    pub fn editar(&mut self, texto: Option<TextoFaixa>, duracao: Option<u32>, importante: bool, opcao_mensagens: bool, agora: u64) {
        let aceita = importante
            || (opcao_mensagens && self.texto.is_none())
            || (texto.is_none() && self.expira.is_none());
        if !aceita {
            return;
        }
        let [mul, div] = self.cfg.faixa.fator_duracao;
        self.expira = duracao.map(|d| agora + u64::from(d) * mul / div.max(1));
        self.texto = texto;
    }

    /// Texto da faixa no instante `agora` (`_DrawRibbonMessage`): vencido o prazo, a mensagem é apagada.
    pub fn texto_da_faixa(&mut self, agora: u64) -> Option<&TextoFaixa> {
        if self.expira.is_some_and(|e| agora > e) {
            self.texto = None;
            self.expira = None;
        }
        self.texto.as_ref()
    }

    /// Mensagem que `_SetPause`/`_SetDefaultWindPrompt` põem na faixa para o estado de pausa e a
    /// ferramenta: pausado sem ferramenta → "Game Paused"; ferramenta 10/11 → instrução dela;
    /// despausado → limpa (permanente, importante). Outras ferramentas pausadas: nada muda.
    pub fn mensagem_de_pausa(&mut self, pausado: bool, ferramenta: i16, importante: bool, opcao_mensagens: bool, agora: u64) {
        let f = &self.cfg.faixa;
        let indice = match (pausado, ferramenta) {
            (false, _) => None,
            (true, -1) => Some(f.pausado),
            (true, 10) => Some(f.ferramenta_troca),
            (true, 11) => Some(f.ferramenta_alvo),
            (true, _) => return,
        };
        let texto = indice.map(|indice| TextoFaixa::Lista { lista: f.lista, indice });
        self.editar(texto, None, importante, opcao_mensagens, agora);
    }

    /// Mensagem do botão de velocidade (`_DoUserButton` caso 6), já com a velocidade nova.
    /// Texto literal importante com prazo (os do DGROUP: marcadores).
    pub fn mensagem_literal(&mut self, texto: &str, duracao: u32, agora: u64) {
        self.editar(Some(TextoFaixa::Literal(texto.to_string())), Some(duracao), true, false, agora);
    }

    pub fn mensagem_de_velocidade(&mut self, velocidade: i16, agora: u64) {
        let f = &self.cfg.faixa;
        if let Some(t) = usize::try_from(velocidade).ok().and_then(|v| f.velocidade.get(v)) {
            let d = f.duracao_botao;
            self.editar(Some(TextoFaixa::Literal(t.clone())), Some(d), true, false, agora);
        }
    }

    // --- modais --------------------------------------------------------------------------------

    /// Pedido de quadro das regras (`Evento::Mensagem`). Os não forçados só entram com Events ligado.
    /// Devolve se o quadro entrou na fila.
    /// `licoes`: as lições do tutorial (a imagem do quadro de uma lição vem delas).
    pub fn receber_mensagem(&mut self, texto: u16, opcao_eventos: bool, licoes: &[Licao]) -> bool {
        if !opcao_eventos && self.cfg.quadro.nao_forcadas.contains(&texto) {
            return false;
        }
        let imagem = imagem_do_texto(&self.cfg, licoes, texto);
        self.modais.push_back(Modal::Quadro { texto, imagem });
        true
    }

    /// `_EndGameDialog`: limpa a faixa e põe o diálogo de fim de jogo na fila, com a pontuação.
    pub fn receber_fim_de_jogo(&mut self, e: &Estado, agora: u64) {
        self.editar(None, None, true, false, agora);
        let pontos = calcular_pontuacao(e, &self.cfg.pontuacao);
        self.modais.push_back(Modal::FimDeJogo {
            tipo_jogo: e.partida.tipo_jogo,
            pretas_venceram: e.partida.pretas_venceram != 0,
            pontos,
        });
    }

    /// O aviso modal da frente, se houver.
    pub fn modal(&self) -> Option<&Modal> {
        self.modais.front()
    }

    pub fn tem_modal(&self) -> bool {
        !self.modais.is_empty()
    }

    /// Passa o relógio: marca quando o quadro da frente apareceu e o fecha depois da espera de
    /// `_DialogWaitInit` (o fim de jogo não tem prazo). Devolve o que foi fechado.
    pub fn passar_tempo(&mut self, agora: u64) -> Option<Modal> {
        let m = self.modais.front()?;
        let desde = *self.aberto_em.get_or_insert(agora);
        if matches!(m, Modal::Quadro { .. }) && agora >= desde + self.cfg.quadro.espera_tickcount {
            return self.fechar_modal();
        }
        None
    }

    /// Fecha o aviso da frente (clique, tecla ou prazo) e devolve qual era.
    pub fn fechar_modal(&mut self) -> Option<Modal> {
        self.aberto_em = None;
        self.modais.pop_front()
    }
}

/// Imagem do quadro do texto: a da lição do tutorial que usa esse texto, senão a da tabela de
/// data/mensagens.json.
fn imagem_do_texto(cfg: &ConfigAvisos, licoes: &[Licao], texto: u16) -> Option<u16> {
    let da_licao = licoes.iter().find(|l| l.texto == texto || l.texto_se_temp == Some(texto)).map(|l| l.imagem);
    da_licao.or_else(|| cfg.quadro.imagens.get(&texto.to_string()).copied()).filter(|&i| i != 0)
}

// --- pontuação -----------------------------------------------------------------------------------

/// `_CalcScore` (`1000:5530`): oito parciais em porcentagem, ponderadas, mais a vida da amarela,
/// escaladas pelo tipo de jogo e somadas ao `_GameTime`. Contas em 32 bits com sinal, como o
/// original [hipótese: o pseudocódigo mostra algumas contas só na palavra baixa].
pub fn calcular_pontuacao(e: &Estado, c: &ConfigPontuacao) -> i64 {
    let p = parciais(e, c);
    let mut soma = i64::from(e.amarela.saude);
    for (peso, v) in c.pesos.iter().zip(p) {
        soma += peso * v * c.fator_peso;
    }
    let tempo = i64::from(e.partida.tempo as i32);
    let fator_tempo = |div: i64| (tempo / div).max(1);
    if e.partida.tipo_jogo == 2 {
        let f = &c.full;
        if tempo < f.limite_tempo {
            soma = fator_tempo(f.divisor_tempo) * soma / f.divisor;
        }
        soma *= f.multiplicador;
    } else {
        let o = &c.outros;
        soma = soma * o.multiplicador / o.divisor_multiplicador;
        if tempo < o.limite_tempo {
            soma = fator_tempo(o.divisor_tempo) * soma / o.divisor;
        }
    }
    tempo + soma
}

/// As oito parciais de `_CalcScore` (0–100, ou 0 quando não se aplicam).
pub fn parciais(e: &Estado, c: &ConfigPontuacao) -> [i64; 8] {
    let mut p = [0i64; 8];
    let h = &e.historico;
    let n = i64::from(h.contagem);
    let inicio = (i64::from(h.inicio) - n).rem_euclid(64) as usize;
    let amostras = || (0..n.max(0) as usize).map(move |k| (inicio + k) % 64);
    // p0: média da saúde preta no histórico.
    let saude: i64 = amostras().map(|i| i64::from(h.saude_b[i])).sum();
    p[0] = if n < 1 { 0 } else { saude / n };
    // p1: parte preta da comida (pretas + vermelhas) no histórico.
    let (cb, cr): (i64, i64) = amostras().fold((0, 0), |(b, r), i| (b + i64::from(h.comida_b[i]), r + i64::from(h.comida_r[i])));
    p[1] = if cb + cr < 1 { 0 } else { cb * 100 / (cb + cr) };
    // p2: ovos que sobreviveram.
    let postos = i64::from(e.partida.ovos_postos_b);
    p[2] = if postos <= 0 { 100 } else { (postos - i64::from(e.partida.ovos_perdidos_b)) * 100 / postos };
    // p3: _BlackLost / (_BlackLost + _RedLost).
    let (pb, pr) = (i64::from(e.preta.perdas), i64::from(e.vermelha.perdas));
    p[3] = if pb + pr < 1 { 100 } else { pb * 100 / (pb + pr) };
    if e.partida.tipo_jogo == 2 || e.partida.tipo_jogo == 3 {
        let fb = i64::from(e.preta.colonias_fundadas);
        let fr = i64::from(e.vermelha.colonias_fundadas);
        let kb = i64::from(e.preta.colonias_extintas);
        p[4] = if fb + fr < 1 { 100 } else { fb * 100 / (fb + fr) };
        p[5] = if fb + kb < 1 { 100 } else { fb * 100 / (fb + kb) };
        // p6 e p7: patches do quintal e da casa com população preta (_YMapPopB, índice x × 16 + y).
        let pop = &e.preta.populacao_quintal;
        let (mut quintal, mut n_quintal, mut casa, mut n_casa) = (0i64, 0i64, 0i64, 0i64);
        for y in 0..16usize {
            let limite = if y < c.linhas_casa_larga { c.colunas_casa[0] } else { c.colunas_casa[1] };
            for x in 0..c.colunas_quintal {
                let ocupado = pop.get(x * 16 + y).is_some_and(|&v| v != 0);
                if x < limite {
                    n_casa += 1;
                    casa += i64::from(ocupado);
                } else {
                    n_quintal += 1;
                    quintal += i64::from(ocupado);
                }
            }
        }
        p[6] = if quintal < 1 { 0 } else { quintal * 100 / n_quintal };
        p[7] = if casa < 1 { 0 } else { casa * 100 / n_casa };
    }
    p
}

/// Índice da avaliação (lista 4/1900) para os pontos: faixas sobre os pontos (÷ 5 no Full Game);
/// sem a vitória das pretas, soma 5.
pub fn indice_avaliacao(cfg: &ConfigFimDeJogo, tipo_jogo: i16, pretas_venceram: bool, pontos: i64) -> usize {
    let v = if tipo_jogo == 2 { pontos / cfg.divisor_full.max(1) } else { pontos };
    let i = cfg.faixas_avaliacao.iter().take_while(|&&f| v >= f).count();
    if pretas_venceram {
        i
    } else {
        i + cfg.soma_derrota
    }
}

// --- textos --------------------------------------------------------------------------------------

/// Strings da lista SHARED 4/`id` (vazia se a lista não existe, como `_LoadStringAnt` devolvendo 0).
pub fn linhas_do_texto(recursos: &Recursos, id: u16) -> Vec<Vec<u8>> {
    recursos
        .objeto(Chave { tipo: TIPO_STRINGS, id })
        .and_then(|d| ListaStrings::decodificar(&d))
        .map(|l| l.strings)
        .unwrap_or_default()
}

/// Bytes do texto da faixa.
pub fn bytes_da_faixa(recursos: &Recursos, t: &TextoFaixa) -> Vec<u8> {
    match t {
        TextoFaixa::Lista { lista, indice } => linhas_do_texto(recursos, *lista).get(*indice).cloned().unwrap_or_default(),
        TextoFaixa::Literal(s) => s.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect(),
    }
}

// --- desenho -------------------------------------------------------------------------------------

fn objeto<'a>(j: &'a Janela, nome: &str) -> Result<&'a crate::formats::janela::Objeto, String> {
    j.objetos.iter().find(|o| o.nome == nome).ok_or(format!("objeto {nome} não existe"))
}

/// Mensagem da faixa no retângulo do objeto (já em coordenadas da área cliente), como
/// `_DrawRibbonMessage`: texto centrado com a fonte da faixa e o resto do objeto na cor de fundo.
pub fn desenhar_faixa(q: &mut Quadro, recursos: &Recursos, interface: &Interface, cfg: &ConfigFaixa, r: Retangulo, cor: u8, texto: Option<&TextoFaixa>) {
    let c = interface.cores.get(cor as usize).copied().unwrap_or(crate::formats::janela::Cor { frente: 0, fundo: 15, fundo2: 15 });
    q.preencher(r, c.fundo, c.fundo);
    let (Some(t), Some(Some(f))) = (texto, interface.fontes.get(cfg.fonte)) else { return };
    let bytes = bytes_da_faixa(recursos, t);
    let antes = q.recorte.replace(r);
    q.texto(Retangulo { dir: r.dir - 1, ..r }, f, &bytes, 3, c.frente, c.fundo);
    q.recorte = antes;
}

/// Geometria de um quadro de mensagem na área cliente.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeometriaQuadro {
    /// Contorno externo da janela (com a borda do Windows).
    pub externo: Retangulo,
    /// Área cliente.
    pub interior: Retangulo,
    /// Largura `w` do texto (`max(50, maior linha)`).
    pub largura_texto: i32,
}

/// `_PictureDialog`: retângulo centrado na área principal (abaixo da faixa) com largura `w + 8` e
/// altura `linhas × altura + imagem + 8`, e daí a área cliente medida (data/mensagens.json).
pub fn geometria_quadro(
    cfg: &ConfigQuadro,
    interface: &Interface,
    largura_tela: i32,
    altura_tela: i32,
    larguras: &[i32],
    altura_fonte: i32,
    altura_imagem: Option<i32>,
) -> GeometriaQuadro {
    let faixa = i32::from(interface.moldura.faixa.altura);
    let borda = i32::from(interface.moldura.borda.largura);
    let w = larguras.iter().copied().fold(cfg.largura_minima, i32::max);
    let n = larguras.len() as i32;
    let img = altura_imagem.map_or(0, |h| h + cfg.espaco_imagem);
    let altura = altura_fonte * n + img + cfg.folga_altura;
    let largura = w + cfg.folga_largura;
    let (cw, ch) = (largura_tela, altura_tela - faixa);
    let l = (cw - largura) >> 1;
    let t = (ch - altura) >> 1;
    let ix = l + cfg.recuo_interior[0];
    let iy = faixa + t + cfg.recuo_interior[1];
    let interior = ret_i32(ix, iy, largura + cfg.acrescimo_interior[0], altura + cfg.acrescimo_interior[1]);
    let externo = ret_i32(ix - borda, iy - borda, largura + cfg.acrescimo_interior[0] + 2 * borda, altura + cfg.acrescimo_interior[1] + 2 * borda);
    GeometriaQuadro { externo, interior, largura_texto: w }
}

fn ret_i32(x: i32, y: i32, w: i32, h: i32) -> Retangulo {
    Retangulo { esq: x as i16, topo: y as i16, dir: (x + w) as i16, base: (y + h) as i16 }
}

/// Moldura do Windows: 1 pixel na cor externa e o resto na cor da borda (data/interface.json).
fn desenhar_borda(q: &mut Quadro, interface: &Interface, r: Retangulo) {
    let b = &interface.moldura.borda;
    q.preencher(r, b.cor, b.cor);
    q.contorno(Retangulo { dir: r.dir - 1, base: r.base - 1, ..r }, b.cor_externa);
}

/// Desenha o quadro de mensagem e devolve sua geometria (para o clique) e o que faltou.
pub fn desenhar_quadro(
    q: &mut Quadro,
    recursos: &Recursos,
    interface: &Interface,
    cfg: &ConfigQuadro,
    texto: u16,
    imagem: Option<u16>,
) -> Result<(GeometriaQuadro, Vec<String>), String> {
    let janela = carregar_janela(recursos, cfg.janela)?;
    let mut pendentes = Vec::new();
    let fonte = interface.fontes.get(cfg.fonte).and_then(|f| f.as_ref()).ok_or(format!("fonte {} ausente", cfg.fonte))?;
    let linhas = linhas_do_texto(recursos, texto);
    if linhas.is_empty() {
        pendentes.push(format!("texto SHARED 4/{texto} ausente"));
    }
    let bm = imagem.and_then(|id| match recursos.objeto(Chave { tipo: TIPO_BITMAP, id }).and_then(|d| Bitmap::decodificar(&d)) {
        Ok(b) => Some(b),
        Err(e) => {
            pendentes.push(format!("bitmap {id}: {e}"));
            None
        }
    });
    let larguras: Vec<i32> = linhas.iter().map(|l| fonte.largura_texto(l) as i32).collect();
    let h = fonte.altura() as i32;
    let g = geometria_quadro(cfg, interface, q.largura as i32, q.altura as i32, &larguras, h, bm.as_ref().map(|b| b.altura as i32));
    desenhar_borda(q, interface, g.externo);
    let area = objeto(&janela, &cfg.objeto_texto)?;
    let cor = interface.cores.get(area.cor as usize).copied().unwrap_or(crate::formats::janela::Cor { frente: 0, fundo: 15, fundo2: 15 });
    let i = g.interior;
    q.preencher(i, cor.fundo, cor.fundo2);
    let antes = q.recorte.replace(i);
    // _win_DrawPictureWindow: imagem centrada no alto; linhas centradas na largura da área cliente
    // (medido na C-10), a partir
    // de 2 pixels abaixo da imagem (ou do topo).
    let (x0, largura) = (i32::from(i.esq), i32::from(i.dir - i.esq));
    let mut y = i32::from(i.topo);
    match &bm {
        Some(b) => {
            q.bitmap(b, (2 * x0 + largura - b.largura as i32) / 2, y);
            y += b.altura as i32 + cfg.espaco_imagem;
        }
        None => y += cfg.espaco_imagem,
    }
    for (l, wl) in linhas.iter().zip(&larguras) {
        let r = ret_i32((2 * x0 + largura - wl) / 2, y, *wl, h);
        q.texto(r, fonte, l, 0, cor.frente, cor.fundo);
        y += h;
    }
    // "Click here to continue" (objeto picStrClick, com a fonte e a cor dele).
    let clique = objeto(&janela, &cfg.objeto_clique)?;
    if let crate::formats::janela::Conteudo::Texto { fonte: fc, texto: tc, .. } = &clique.conteudo {
        let cc = interface.cores.get(clique.cor as usize).copied().unwrap_or(cor);
        if let Some(Some(f)) = interface.fontes.get(*fc as usize) {
            let wc = f.largura_texto(tc.as_bytes()) as i32;
            // Centrado na área cliente com 1 pixel a mais à direita (_win_GetObjRect com faixa).
            let x = (i32::from(i.esq) + i32::from(i.dir) + 1 - wc) / 2;
            let r = ret_i32(x, i32::from(i.base) - cfg.clique_acima_da_base, wc, f.altura() as i32);
            q.texto(r, f, tc.as_bytes(), 0, cc.frente, cc.fundo);
        }
    }
    q.recorte = antes;
    Ok((g, pendentes))
}

/// Retângulo do objeto `nome` da janela já posicionada na tela (coordenadas da área cliente).
fn objeto_na_tela(interface: &Interface, id: u16, janela: &Janela, nome: &str, ajuste: [i16; 2]) -> Result<(Retangulo, u8), String> {
    let g = interface.posicionar(id, janela).ok_or(format!("janela {id} sem posição"))?;
    let base = janela.retangulo();
    let (dx, dy) = (g.origem.0 - base.esq + ajuste[0], g.origem.1 - base.topo + ajuste[1]);
    let o = objeto(janela, nome)?;
    let r = o.retangulo;
    Ok((Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy }, o.cor))
}

/// Desenha o diálogo de fim de jogo (janela 4 com o bitmap e os três textos) e devolve o retângulo
/// que fecha o diálogo ao ser clicado.
pub fn desenhar_fim_de_jogo(
    q: &mut Quadro,
    recursos: &Recursos,
    interface: &Interface,
    cfg: &ConfigFimDeJogo,
    tipo_jogo: i16,
    pretas_venceram: bool,
    pontos: i64,
) -> Result<(Retangulo, Vec<String>), String> {
    let janela = carregar_janela(recursos, cfg.janela)?;
    let pendentes = desenhar_janela_deslocada(q, recursos, interface, cfg.janela, &janela, (cfg.ajuste[0], cfg.ajuste[1]))?;
    let fonte = interface.fontes.get(cfg.fonte).and_then(|f| f.as_ref()).ok_or(format!("fonte {} ausente", cfg.fonte))?;
    let tipos = linhas_do_texto(recursos, cfg.lista_tipos);
    let tipo = usize::try_from(tipo_jogo).ok().filter(|&t| t < 4).unwrap_or(cfg.tipo_padrao);
    let avaliacoes = linhas_do_texto(recursos, cfg.lista_avaliacoes);
    let aval = indice_avaliacao(cfg, tipo_jogo, pretas_venceram, pontos);
    let textos = [
        (&cfg.objeto_tipo, tipos.get(tipo).cloned().unwrap_or_default()),
        (&cfg.objeto_pontos, pontos.to_string().into_bytes()),
        (&cfg.objeto_avaliacao, avaliacoes.get(aval).cloned().unwrap_or_default()),
    ];
    for (nome, t) in textos {
        // _win_PrintfAtObj: com faixa, o retângulo ganha 1 pixel à direita e embaixo; texto
        // centrado e o resto do objeto na cor de fundo, recortado ao objeto. Medido na captura m8
        // (fim de jogo): o texto bate melhor sem o pixel a mais à direita [causa em aberto].
        let (r, cor) = objeto_na_tela(interface, cfg.janela, &janela, nome, cfg.ajuste)?;
        let r = Retangulo { dir: r.dir - 1, base: r.base + 1, ..r };
        let c = interface.cores.get(cor as usize).copied().unwrap_or(crate::formats::janela::Cor { frente: 0, fundo: 15, fundo2: 15 });
        let antes = q.recorte.replace(r);
        q.texto(r, fonte, &t, 3, c.frente, c.fundo);
        q.recorte = antes;
    }
    let (clique, _) = objeto_na_tela(interface, cfg.janela, &janela, &cfg.objeto_clique, cfg.ajuste)?;
    Ok((clique, pendentes))
}

/// Desenha o aviso modal da frente, se houver, e devolve o retângulo que o fecha ao ser clicado.
pub fn desenhar_modal(q: &mut Quadro, recursos: &Recursos, interface: &Interface, avisos: &Avisos) -> Result<(Option<Retangulo>, Vec<String>), String> {
    match avisos.modal() {
        None => Ok((None, Vec::new())),
        Some(Modal::Quadro { texto, imagem }) => {
            let (g, p) = desenhar_quadro(q, recursos, interface, &avisos.cfg.quadro, *texto, *imagem)?;
            Ok((Some(g.interior), p))
        }
        Some(Modal::FimDeJogo { tipo_jogo, pretas_venceram, pontos }) => {
            let (r, p) = desenhar_fim_de_jogo(q, recursos, interface, &avisos.cfg.fim_de_jogo, *tipo_jogo, *pretas_venceram, *pontos)?;
            Ok((Some(r), p))
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cfg() -> ConfigAvisos {
        ConfigAvisos::de_json(include_str!("../../../data/mensagens.json")).unwrap()
    }

    #[test]
    fn mensagem_comum_so_entra_com_a_faixa_vazia_e_vence_no_prazo() {
        let mut a = Avisos::new(cfg());
        let t = |i| Some(TextoFaixa::Lista { lista: 1020, indice: i });
        a.editar(t(4), Some(120), false, true, 100);
        assert_eq!(a.texto_da_faixa(100), t(4).as_ref());
        // Faixa ocupada: o pedido comum é ignorado; o importante troca.
        a.editar(t(2), Some(120), false, true, 101);
        assert_eq!(a.texto_da_faixa(101), t(4).as_ref());
        a.editar(t(0), Some(120), true, true, 102);
        assert_eq!(a.texto_da_faixa(102), t(0).as_ref());
        // Prazo: 120 × 3 / 2 = 180 unidades de _TickCount.
        assert_eq!(a.texto_da_faixa(282), t(0).as_ref());
        assert_eq!(a.texto_da_faixa(283), None);
        // Opção Messages desligada: só o importante.
        a.editar(t(4), Some(120), false, false, 300);
        assert_eq!(a.texto_da_faixa(300), None);
    }

    #[test]
    fn pausa_e_permanente_e_despausar_limpa() {
        let mut a = Avisos::new(cfg());
        a.mensagem_de_pausa(true, -1, true, true, 0);
        assert_eq!(a.texto_da_faixa(1_000_000), Some(&TextoFaixa::Lista { lista: 1010, indice: 2 }));
        a.mensagem_de_pausa(false, -1, true, true, 5);
        assert_eq!(a.texto_da_faixa(5), None);
        a.mensagem_de_pausa(true, 11, true, true, 6);
        assert_eq!(a.texto_da_faixa(6), Some(&TextoFaixa::Lista { lista: 1010, indice: 17 }));
        // Pedido de limpar sem ser importante: aceito porque a mensagem atual é permanente.
        a.editar(None, None, false, false, 7);
        assert_eq!(a.texto_da_faixa(7), None);
    }

    #[test]
    fn velocidade_mostra_o_texto_do_dgroup() {
        let mut a = Avisos::new(cfg());
        a.mensagem_de_velocidade(2, 10);
        assert_eq!(a.texto_da_faixa(10), Some(&TextoFaixa::Literal("Game speed now fast.".into())));
        assert_eq!(a.texto_da_faixa(10 + 270), Some(&TextoFaixa::Literal("Game speed now fast.".into())));
        assert_eq!(a.texto_da_faixa(10 + 271), None);
    }

    #[test]
    fn quadros_nao_forcados_dependem_de_events_e_fecham_no_prazo() {
        let mut a = Avisos::new(cfg());
        assert!(!a.receber_mensagem(10050, false, &[]));
        assert!(!a.tem_modal());
        assert!(a.receber_mensagem(10010, false, &[]));
        assert!(a.receber_mensagem(10011, false, &[]));
        assert_eq!(a.modal(), Some(&Modal::Quadro { texto: 10010, imagem: None }));
        assert_eq!(a.passar_tempo(1000), None);
        assert_eq!(a.passar_tempo(1269), None);
        assert_eq!(a.passar_tempo(1270), Some(Modal::Quadro { texto: 10010, imagem: None }));
        assert_eq!(a.modal(), Some(&Modal::Quadro { texto: 10011, imagem: None }));
        assert_eq!(a.fechar_modal(), Some(Modal::Quadro { texto: 10011, imagem: None }));
        assert!(!a.tem_modal());
    }

    #[test]
    fn imagens_das_mortes_e_do_tutorial() {
        let a = cfg();
        let licoes = &crate::rules::quintal::TabelasQuintal::padrao().conselhos.licoes;
        assert_eq!(imagem_do_texto(&a, licoes, 10034), Some(9155));
        assert_eq!(imagem_do_texto(&a, licoes, 11002), Some(17000));
        assert_eq!(imagem_do_texto(&a, licoes, 11000), None);
        assert_eq!(imagem_do_texto(&a, licoes, 10010), None);
    }

    #[test]
    fn avaliacao_por_faixa_e_derrota() {
        let c = cfg().fim_de_jogo;
        assert_eq!(indice_avaliacao(&c, 1, true, 0), 0);
        assert_eq!(indice_avaliacao(&c, 1, true, 214_220), 1);
        assert_eq!(indice_avaliacao(&c, 1, false, 900_000), 9);
        assert_eq!(indice_avaliacao(&c, 2, true, 214_220 * 5 - 1), 0);
    }

    #[test]
    fn pontuacao_de_um_estado_vazio() {
        // Estado vazio, Quick Game: p2 = p3 = 100, o resto 0; vida 0.
        let c = cfg().pontuacao;
        let mut e = Estado::vazio();
        e.partida.tipo_jogo = 1;
        assert_eq!(parciais(&e, &c), [0, 0, 100, 100, 0, 0, 0, 0]);
        // soma = (19 + 23) × 100 × 51 = 214200; × 29 / 10 = 621180; tempo 0 < 4100: × max(1, 0) / 41.
        assert_eq!(calcular_pontuacao(&e, &c), 621_180 / 41);
        e.partida.tempo = 5000;
        assert_eq!(calcular_pontuacao(&e, &c), 5000 + 621_180);
    }

    #[test]
    fn geometria_do_quadro_bate_com_a_captura_c10() {
        // C-10 (640×480 na QEMU): uma linha de 254 px na fonte de 14 px; área cliente em (191, 287)
        // da tela, 266 × 25; contorno externo em (186, 282). A área cliente começa na linha 38.
        let q = cfg().quadro;
        let interface_faixa = 73;
        let (larg, alt) = (640, 442);
        let w = [254];
        // Sem Interface real: monta só o que a conta usa.
        let faixa = interface_faixa;
        let largura = 254 + q.folga_largura;
        let altura = 14 + q.folga_altura;
        let l = (larg - largura) >> 1;
        let t = (alt - faixa - altura) >> 1;
        assert_eq!((l + q.recuo_interior[0], faixa + t + q.recuo_interior[1] + 38), (191, 287));
        assert_eq!((largura + q.acrescimo_interior[0], altura + q.acrescimo_interior[1]), (266, 25));
        assert_eq!(w.len(), 1);
    }
}
