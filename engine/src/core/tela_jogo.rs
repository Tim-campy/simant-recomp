//! Tela de jogo: faixa (janela 34) e janela de edição (janela 0) com o mapa e a vida de um jogo salvo.
//! Especificação: docs/spec/telas/jogo.md. Geometria e cores em data/tela_jogo.json.

use std::collections::HashMap;

use serde::Deserialize;

use super::avisos::{desenhar_faixa, desenhar_modal, Avisos};
use super::edicao::{desenhar_vista, tiles_para, CenaEdicao, VistaEdicao};
use super::mapa::{MapaChao, Plano};
use super::navegacao::{Acao, Navegacao};
use super::quadro::Quadro;
use super::telas::{carregar_janela, desenhar_janela_carregada_na_tela, Interface};
use super::vida::{plano_de, reconstruir, CamadasVida, FormigaAmarela};
use crate::formats::banco::Chave;
use crate::formats::janela::{Janela, Retangulo};
use crate::formats::save::JogoSalvo;
use crate::formats::tiles::{self, Tileset, LADO};
use crate::rules::estado::Estado;
use crate::recursos::Recursos;

/// Retângulo (x, y, largura, altura) em coordenadas da área cliente da janela principal.
pub type Caixa = [i16; 4];

fn ret(c: Caixa) -> Retangulo {
    Retangulo { esq: c[0], topo: c[1], dir: c[0] + c[2], base: c[1] + c[3] }
}

/// De onde vem a posição da vista.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PosicaoVista {
    /// `_MapPnt` do save, como no instante em que foi gravado.
    Salva,
    /// Como depois de carregar: centraliza no ponto lembrado do plano e na formiga amarela.
    AoCarregar,
}

/// Moldura de janela do Windows com título, moldura grossa e barras de rolagem (medida na captura).
#[derive(Clone, Debug, Deserialize)]
pub struct MolduraWindows {
    /// Contorno externo.
    pub externo: Caixa,
    /// Espessura total da moldura: 1 linha externa, preenchimento e 1 linha interna.
    pub espessura: i16,
    pub altura_titulo: i16,
    /// Largura das barras de rolagem, incluindo a linha preta que as separa da área cliente.
    pub barra_rolagem: i16,
    pub cor_linha: u8,
    pub cor_moldura: u8,
    pub cor_titulo: u8,
    pub cor_botao: u8,
    pub cor_relevo_claro: u8,
    pub cor_relevo_escuro: u8,
    pub cor_trilho: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigEdicao {
    pub janela: u16,
    /// Nome do objeto tipo 1 onde o mapa é desenhado.
    pub area: String,
    /// Pixels somados à direita e à base do retângulo da área quando há faixa (`_win_GetObjRect`).
    pub ajuste_area_com_faixa: i16,
    /// Onde o canto da área cai na tela.
    pub area_na_tela: [i16; 2],
    pub posicao: PosicaoVista,
    pub moldura: MolduraWindows,
}

/// Conteúdo de data/tela_jogo.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigTelaJogo {
    pub largura: u32,
    pub altura: u32,
    pub fundo: u8,
    /// Objeto tipo 9 com as posições das janelas (0 = até 480 linhas).
    pub tabela_posicoes: u16,
    /// Objeto da faixa selecionado para cada `_MapPlane` (chave em texto: "1", "2", "3").
    pub botao_do_plano: HashMap<String, String>,
    pub edicao: ConfigEdicao,
}

/// Bit "selecionado" das flags de objeto (janela.md).
const FLAG_SELECIONADO: u16 = 4;

/// Retângulos da moldura calculados a partir do contorno externo.
pub struct PartesMoldura {
    pub titulo: Retangulo,
    /// Área cliente sem as barras de rolagem: onde o mapa aparece.
    pub cliente: Retangulo,
}

/// Botão em relevo do Windows 3.1: 1 pixel claro em cima e à esquerda, 2 escuros embaixo e à direita.
fn botao(q: &mut Quadro, r: Retangulo, m: &MolduraWindows) {
    q.preencher(r, m.cor_botao, m.cor_botao);
    let (x0, y0, x1, y1) = (r.esq as i32, r.topo as i32, r.dir as i32 - 1, r.base as i32 - 1);
    for x in x0..x1 {
        q.ponto(x, y0, m.cor_relevo_claro);
    }
    for y in y0..y1 {
        q.ponto(x0, y, m.cor_relevo_claro);
    }
    for k in 0..2 {
        for x in x0 + k..=x1 {
            q.ponto(x, y1 - k, m.cor_relevo_escuro);
        }
        for y in y0 + k..=y1 {
            q.ponto(x1 - k, y, m.cor_relevo_escuro);
        }
    }
}

fn linha_h(q: &mut Quadro, x0: i16, x1: i16, y: i16, cor: u8) {
    q.preencher(Retangulo { esq: x0, topo: y, dir: x1, base: y + 1 }, cor, cor);
}

fn linha_v(q: &mut Quadro, x: i16, y0: i16, y1: i16, cor: u8) {
    q.preencher(Retangulo { esq: x, topo: y0, dir: x + 1, base: y1 }, cor, cor);
}

/// Desenha a moldura (sem texto do título, setas e polegares das barras) e devolve as partes.
pub fn desenhar_moldura(q: &mut Quadro, m: &MolduraWindows) -> PartesMoldura {
    let e = ret(m.externo);
    let (t, h, b) = (m.espessura, m.altura_titulo, m.barra_rolagem);
    let k = m.cor_linha;
    q.preencher(e, m.cor_moldura, m.cor_moldura);
    q.contorno(Retangulo { dir: e.dir - 1, base: e.base - 1, ..e }, k);
    q.contorno(Retangulo { esq: e.esq + t - 1, topo: e.topo + t - 1, dir: e.dir - t, base: e.base - t }, k);
    // Marcas dos cantos da moldura grossa, a (espessura + título) de cada canto.
    let d = t + h;
    for x in [e.esq + d, e.dir - 1 - d] {
        linha_v(q, x, e.topo, e.topo + t, k);
        linha_v(q, x, e.base - t, e.base, k);
    }
    for y in [e.topo + d, e.base - 1 - d] {
        linha_h(q, e.esq, e.esq + t, y, k);
        linha_h(q, e.dir - t, e.dir, y, k);
    }
    let i = Retangulo { esq: e.esq + t, topo: e.topo + t, dir: e.dir - t, base: e.base - t };
    // Título: caixa do menu de sistema, faixa colorida, botão de maximizar e linha embaixo.
    let titulo = Retangulo { base: i.topo + h, ..i };
    q.preencher(titulo, m.cor_titulo, m.cor_titulo);
    q.preencher(Retangulo { dir: i.esq + h, ..titulo }, m.cor_botao, m.cor_botao);
    linha_v(q, i.esq + h, titulo.topo, titulo.base, k);
    botao(q, Retangulo { esq: i.dir - h, ..titulo }, m);
    linha_v(q, i.dir - h - 1, titulo.topo, titulo.base, k);
    linha_h(q, i.esq, i.dir, titulo.base, k);
    // Barras de rolagem: linha preta, botões nas pontas e trilho; canto entre as duas.
    let c = Retangulo { esq: i.esq, topo: titulo.base + 1, dir: i.dir - b, base: i.base - b };
    let lado = b - 1;
    let v = Retangulo { esq: c.dir + 1, topo: c.topo, dir: i.dir, base: c.base };
    q.preencher(v, m.cor_trilho, m.cor_trilho);
    linha_v(q, c.dir, c.topo, c.base, k);
    botao(q, Retangulo { base: v.topo + lado, ..v }, m);
    linha_h(q, v.esq, v.dir, v.topo + lado, k);
    botao(q, Retangulo { topo: v.base - lado, ..v }, m);
    linha_h(q, v.esq, v.dir, v.base - lado - 1, k);
    let hz = Retangulo { esq: c.esq, topo: c.base + 1, dir: c.dir, base: i.base };
    q.preencher(hz, m.cor_trilho, m.cor_trilho);
    linha_h(q, c.esq, c.dir, c.base, k);
    botao(q, Retangulo { dir: hz.esq + lado, ..hz }, m);
    linha_v(q, hz.esq + lado, hz.topo, hz.base, k);
    botao(q, Retangulo { esq: hz.dir - lado, ..hz }, m);
    linha_v(q, hz.dir - lado - 1, hz.topo, hz.base, k);
    let canto = Retangulo { esq: c.dir, topo: c.base, dir: i.dir, base: i.base };
    q.preencher(canto, m.cor_botao, m.cor_botao);
    linha_v(q, c.dir, c.base, i.base, k);
    linha_h(q, c.dir, i.dir, c.base, k);
    PartesMoldura { titulo, cliente: c }
}

fn tileset(recursos: &Recursos, id: u16) -> Result<Tileset, String> {
    Tileset::decodificar(&recursos.objeto(Chave { tipo: tiles::TIPO_TILES, id })?)
}

fn tileset_vida(recursos: &Recursos, ids: &[u16]) -> Result<Tileset, String> {
    let partes = ids
        .iter()
        .map(|&id| recursos.objeto(Chave { tipo: tiles::TIPO_TILES, id }))
        .collect::<Result<Vec<_>, _>>()?;
    Tileset::concatenar_vida(&partes)
}

/// Bloco do save com o mapa de chão do plano.
fn bloco_do_mapa(plano: Plano) -> &'static str {
    match plano {
        Plano::Superficie => "MapA",
        Plano::NinhoPreto => "MapB",
        Plano::NinhoVermelho => "MapR",
    }
}

fn ponto_lembrado(n: i16) -> Option<&'static str> {
    match n {
        0 => Some("YMapPnt"),
        1 => Some("AMapPnt"),
        2 => Some("BMapPnt"),
        3 => Some("RMapPnt"),
        _ => None,
    }
}

/// Vista da janela de edição para o save, conforme `posicao` (jogo.md, "Posição ao carregar").
pub fn vista_do_save(jogo: &JogoSalvo, posicao: PosicaoVista, largura: i32, altura: i32) -> Result<VistaEdicao, String> {
    let mut v = VistaEdicao::do_save(jogo, largura, altura)?;
    if posicao == PosicaoVista::AoCarregar {
        let me = FormigaAmarela::do_save(jogo)?;
        let plano_mapa = jogo.i16("MapPlane")?;
        // _DoLoadInitializations grava a posição da formiga no ponto lembrado do plano dela.
        let centro = if me.plano == plano_mapa {
            (me.x as i32, me.y as i32)
        } else {
            let s = ponto_lembrado(plano_mapa).ok_or("plano sem ponto lembrado")?;
            let (x, y) = jogo.par_i16(s)?;
            (x as i32, y as i32)
        };
        // _SetMapPlane: plano 0 não centraliza; nos outros, _SetMapPlaneLocation e _SetMapPlane
        // centralizam (a segunda chamada não muda nada).
        if plano_mapa != 0 {
            v.centralizar(centro.0, centro.1);
        }
        if me.plano == plano_mapa {
            v.centralizar(me.x as i32, me.y as i32);
        }
    }
    Ok(v)
}

/// Tela de jogo pronta para ser desenhada várias vezes: layouts, tilesets e vida já carregados do
/// save. A navegação (plano e vista) fica de fora, em `Navegacao`.
pub struct TelaDeJogo {
    pub cfg: ConfigTelaJogo,
    /// Bitmaps trocados em botões da faixa (`_win_SetButtonBitmaps`): nome, selecionado, normal.
    pub bitmaps_da_faixa: Vec<(String, u16, u16)>,
    /// Botões da faixa com o estado "selecionado" ligado ou desligado pelo jogo (`_UpdateUserButtons`).
    pub selecao_da_faixa: Vec<(String, bool)>,
    faixa: Janela,
    /// Canto do objeto 0 da faixa na área cliente.
    origem_faixa: (i16, i16),
    edicao: Janela,
    /// Tamanho da vista em tiles (`_editWidth`, `_editHeight`).
    pub largura_vista: i32,
    pub altura_vista: i32,
    /// Tiles de chão do quintal (9/10) e da casa (9/9); o usado depende de `_TERRAINset`.
    tiles_chao: [Tileset; 2],
    /// `_TERRAINset`: 0 quintal, 1 casa.
    terreno: u16,
    tiles_vida_superficie: Tileset,
    tiles_vida_ninho: Tileset,
    /// Mapas de chão `_MapA`, `_MapB` e `_MapR`.
    mapas: [Vec<u8>; 3],
    vida: CamadasVida,
    amarela: FormigaAmarela,
    /// O que não foi reproduzido ao carregar (vida fora do mapa, `_MePlane` desconhecido...).
    pendencias: Vec<String>,
}

fn indice_do_plano(p: Plano) -> usize {
    match p {
        Plano::Superficie => 0,
        Plano::NinhoPreto => 1,
        Plano::NinhoVermelho => 2,
    }
}

impl TelaDeJogo {
    pub fn carregar(cfg: &ConfigTelaJogo, recursos: &Recursos, interface: &Interface, jogo: &JogoSalvo) -> Result<TelaDeJogo, String> {
        let id_faixa = interface.moldura.faixa.janela;
        let faixa = carregar_janela(recursos, id_faixa)?;
        let origem_faixa = interface.posicionar(id_faixa, &faixa).ok_or(format!("janela {id_faixa} sem posição"))?.origem;
        let e = &cfg.edicao;
        let edicao = carregar_janela(recursos, e.janela)?;
        let area = area_de_edicao(&edicao, e)?;
        let ajuste = if interface.moldura.faixa.altura > 0 { e.ajuste_area_com_faixa } else { 0 };
        let largura_vista = tiles_para((area.dir - area.esq + ajuste) as i32);
        let altura_vista = tiles_para((area.base - area.topo + ajuste) as i32);

        let rec = reconstruir(jogo)?;
        let mut pendencias = rec.pendencias;
        let terreno = jogo.u16("TERRAINset")?;
        let tiles_chao = [tileset(recursos, tiles::ID_CHAO_QUINTAL)?, tileset(recursos, tiles::ID_CHAO_CASA)?];
        let amarela = FormigaAmarela::do_save(jogo)?;
        if plano_de(amarela.plano).is_none() {
            pendencias.push(format!("_MePlane = {}", amarela.plano));
        }
        let mapa = |p: Plano| -> Result<Vec<u8>, String> {
            let s = bloco_do_mapa(p);
            let bytes = jogo.bloco(s).ok_or(format!("bloco {s} ausente"))?;
            MapaChao::new(p, bytes)?;
            Ok(bytes.to_vec())
        };
        Ok(TelaDeJogo {
            cfg: cfg.clone(),
            faixa,
            origem_faixa,
            edicao,
            largura_vista,
            altura_vista,
            tiles_chao,
            terreno,
            tiles_vida_superficie: tileset_vida(recursos, &tiles::IDS_VIDA_SUPERFICIE)?,
            tiles_vida_ninho: tileset_vida(recursos, &tiles::IDS_VIDA_NINHO)?,
            mapas: [mapa(Plano::Superficie)?, mapa(Plano::NinhoPreto)?, mapa(Plano::NinhoVermelho)?],
            vida: rec.camadas,
            amarela,
            pendencias,
            bitmaps_da_faixa: Vec::new(),
            selecao_da_faixa: Vec::new(),
        })
    }

    /// Troca o conteúdo dinâmico (mapas de chão, camadas de vida e formiga amarela) pelo do estado
    /// da simulação. Layouts ficam como estão; o terreno (`_TERRAINset`) escolhe os tiles de chão,
    /// porque a troca de patch (`_XferPatch`) pode ir do quintal para a casa.
    pub fn atualizar(&mut self, e: &Estado) {
        self.terreno = e.ambiente.terreno as u16;
        self.mapas = [e.superficie.mapa.dados.clone(), e.ninho_b.mapa.dados.clone(), e.ninho_r.mapa.dados.clone()];
        self.vida = CamadasVida {
            a: e.superficie.vida.dados.clone(),
            b: e.ninho_b.vida.dados.clone(),
            r: e.ninho_r.vida.dados.clone(),
        };
        let a = &e.amarela;
        self.amarela =
            FormigaAmarela { modo: a.modo, plano: a.plano, x: a.x, y: a.y, tipo: a.tipo, dir: a.dir, ovo: a.ovo, base: a.base };
    }

    /// Célula do mapa sob o ponto (x, y) da área cliente, se ele cai na parte visível da janela de
    /// edição (jogo.md, "Clique no mapa": `(px − esquerda) / 16 + _MapPnt.x`, idem para y).
    pub fn celula_em(&self, x: i32, y: i32, nav: &Navegacao) -> Option<(i32, i32)> {
        let cliente = cliente_da_moldura(&self.cfg.edicao.moldura);
        if !contem(cliente, x, y) {
            return None;
        }
        let [ax, ay] = self.cfg.edicao.area_na_tela;
        let (dx, dy) = (x - ax as i32, y - ay as i32);
        if dx < 0 || dy < 0 {
            return None;
        }
        let (col, lin) = (dx / LADO as i32, dy / LADO as i32);
        if col >= nav.vista.largura || lin >= nav.vista.altura {
            return None;
        }
        let (cx, cy) = nav.vista.celula(col, lin);
        Some((cx as i32, cy as i32))
    }

    /// Estado de navegação inicial para o save, com a vista conforme `posicao`.
    pub fn navegacao(&self, jogo: &JogoSalvo, posicao: PosicaoVista) -> Result<Navegacao, String> {
        Navegacao::do_save(jogo, posicao, self.largura_vista, self.altura_vista)
    }

    /// Desenha fundo, faixa (com o botão do plano selecionado) e janela de edição com a vista de `nav`.
    /// Devolve o quadro e o que ainda não foi reproduzido.
    pub fn compor(&self, recursos: &Recursos, interface: &Interface, nav: &Navegacao) -> Result<(Quadro, Vec<String>), String> {
        self.compor_com(recursos, interface, nav, &mut |_| Ok(Vec::new()))
    }

    /// Como `compor`, desenhando com `atras` o que fica entre a faixa e a janela de edição (as
    /// janelas de controle que estão atrás dela).
    pub fn compor_com(
        &self,
        recursos: &Recursos,
        interface: &Interface,
        nav: &Navegacao,
        atras: &mut dyn FnMut(&mut Quadro) -> Result<Vec<String>, String>,
    ) -> Result<(Quadro, Vec<String>), String> {
        let cfg = &self.cfg;
        let mut q = Quadro::new(cfg.largura as usize, cfg.altura as usize, cfg.fundo);
        let id_faixa = interface.moldura.faixa.janela;
        // _SetMapPlaneLocation: com faixa, seleciona o botão do plano mostrado (e só ele).
        let mut faixa = self.faixa.clone();
        for ob in faixa.objetos.iter_mut() {
            if let Some((_, s, n)) = self.bitmaps_da_faixa.iter().find(|(nome, _, _)| *nome == ob.nome) {
                if let crate::formats::janela::Conteudo::BotaoBitmap { selecionado, normal } = &mut ob.conteudo {
                    (*selecionado, *normal) = (*s, *n);
                }
            }
            if let Some((_, sel)) = self.selecao_da_faixa.iter().find(|(nome, _)| *nome == ob.nome) {
                if *sel {
                    ob.flags |= FLAG_SELECIONADO;
                } else {
                    ob.flags &= !FLAG_SELECIONADO;
                }
            }
        }
        for ob in faixa.objetos.iter_mut().filter(|o| cfg.botao_do_plano.values().any(|n| *n == o.nome)) {
            ob.flags &= !FLAG_SELECIONADO;
        }
        if let Some(nome) = cfg.botao_do_plano.get(&nav.plano_mapa.to_string()) {
            let ob = faixa
                .objetos
                .iter_mut()
                .find(|o| &o.nome == nome)
                .ok_or(format!("janela {id_faixa}: objeto {nome} não existe"))?;
            ob.flags |= FLAG_SELECIONADO;
        }
        let mut pendentes = desenhar_janela_carregada_na_tela(&mut q, recursos, interface, id_faixa, &faixa)?;
        pendentes.extend(atras(&mut q)?);

        let e = &cfg.edicao;
        let area = area_de_edicao(&self.edicao, e)?;
        let partes = desenhar_moldura(&mut q, &e.moldura);
        let [ax, ay] = e.area_na_tela;
        let base = self.edicao.retangulo();
        let origem = (ax - (area.esq - base.esq), ay - (area.topo - base.topo));
        pendentes.extend(super::telas::desenhar_objetos(&mut q, recursos, interface, &self.edicao, origem, Some(partes.cliente)));
        pendentes.extend(self.pendencias.iter().cloned());

        let vista = &nav.vista;
        let tiles_vida = if vista.plano == Plano::Superficie { &self.tiles_vida_superficie } else { &self.tiles_vida_ninho };
        let cena = CenaEdicao {
            chao: MapaChao::new(vista.plano, &self.mapas[indice_do_plano(vista.plano)])?,
            vida: &self.vida,
            amarela: self.amarela,
            tiles_chao: &self.tiles_chao[usize::from(self.terreno == 1)],
            tiles_vida,
        };
        q.recorte = Some(partes.cliente);
        let r = desenhar_vista(&mut q, vista, &cena, (ax as i32, ay as i32));
        q.recorte = None;
        r?;
        Ok((q, pendentes))
    }

    /// Nomes dos objetos visíveis da faixa sob o ponto (x, y) da área cliente.
    pub fn objetos_da_faixa_em(&self, x: i32, y: i32) -> Vec<&str> {
        let base = self.faixa.retangulo();
        let (dx, dy) = ((self.origem_faixa.0 - base.esq) as i32, (self.origem_faixa.1 - base.topo) as i32);
        self.faixa
            .objetos
            .iter()
            .skip(1)
            .filter(|o| o.visivel() && contem(o.retangulo, x - dx, y - dy))
            .map(|o| o.nome.as_str())
            .collect()
    }

    /// Retângulo (na área cliente) e cor do objeto `nome` da faixa.
    /// Contorno externo da janela de edição (área cliente).
    pub fn retangulo_da_edicao(&self) -> Retangulo {
        ret(self.cfg.edicao.moldura.externo)
    }

    pub fn objeto_da_faixa(&self, nome: &str) -> Option<(Retangulo, u8)> {
        let base = self.faixa.retangulo();
        let (dx, dy) = (self.origem_faixa.0 - base.esq, self.origem_faixa.1 - base.topo);
        let o = self.faixa.objetos.iter().find(|o| o.nome == nome)?;
        let r = o.retangulo;
        Some((Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy }, o.cor))
    }

    /// Desenha os avisos por cima da tela composta: a mensagem da faixa (`_DrawRibbonMessage`) e o
    /// aviso modal da frente. Devolve o retângulo que fecha o modal ao ser clicado e o que faltou.
    pub fn desenhar_avisos(
        &self,
        q: &mut Quadro,
        recursos: &Recursos,
        interface: &Interface,
        avisos: &mut Avisos,
        agora: u64,
    ) -> Result<(Option<Retangulo>, Vec<String>), String> {
        let cfg = avisos.cfg.faixa.clone();
        let (r, cor) = self.objeto_da_faixa(&cfg.objeto).ok_or(format!("faixa sem o objeto {}", cfg.objeto))?;
        let texto = avisos.texto_da_faixa(agora).cloned();
        desenhar_faixa(q, recursos, interface, &cfg, r, cor, texto.as_ref());
        desenhar_modal(q, recursos, interface, avisos)
    }

    /// Retângulo (na área cliente) do objeto da faixa com esse nome.
    pub fn retangulo_da_faixa(&self, nome: &str) -> Option<Retangulo> {
        let base = self.faixa.retangulo();
        let (dx, dy) = (self.origem_faixa.0 - base.esq, self.origem_faixa.1 - base.topo);
        let r = self.faixa.objetos.iter().find(|o| o.nome == nome)?.retangulo;
        Some(Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy })
    }

    /// Ação da seta da barra de rolagem da janela de edição sob o ponto (x, y), se houver.
    pub fn seta_em(&self, x: i32, y: i32) -> Option<Acao> {
        setas_de_rolagem(&self.cfg.edicao.moldura).into_iter().find(|(r, _)| contem(*r, x, y)).map(|(_, a)| a)
    }
}

fn area_de_edicao(janela: &Janela, e: &ConfigEdicao) -> Result<Retangulo, String> {
    janela
        .objetos
        .iter()
        .find(|o| o.nome == e.area)
        .map(|o| o.retangulo)
        .ok_or(format!("janela {}: objeto {} não existe", e.janela, e.area))
}

/// Área cliente da janela de edição sem as barras de rolagem (a mesma geometria de `desenhar_moldura`).
pub fn cliente_da_moldura(m: &MolduraWindows) -> Retangulo {
    let e = ret(m.externo);
    let (t, h, b) = (m.espessura, m.altura_titulo, m.barra_rolagem);
    Retangulo { esq: e.esq + t, topo: e.topo + t + h + 1, dir: e.dir - t - b, base: e.base - t - b }
}

fn contem(r: Retangulo, x: i32, y: i32) -> bool {
    x >= r.esq as i32 && x < r.dir as i32 && y >= r.topo as i32 && y < r.base as i32
}

/// Botões de seta das barras de rolagem (mesma geometria de `desenhar_moldura`) e a ação de cada um:
/// `SB_LINEUP`/`SB_LINEDOWN` de `WM_VSCROLL` e `WM_HSCROLL`.
pub fn setas_de_rolagem(m: &MolduraWindows) -> [(Retangulo, Acao); 4] {
    let e = ret(m.externo);
    let (t, h, b) = (m.espessura, m.altura_titulo, m.barra_rolagem);
    let i = Retangulo { esq: e.esq + t, topo: e.topo + t, dir: e.dir - t, base: e.base - t };
    let c = Retangulo { esq: i.esq, topo: i.topo + h + 1, dir: i.dir - b, base: i.base - b };
    let lado = b - 1;
    let v = Retangulo { esq: c.dir + 1, topo: c.topo, dir: i.dir, base: c.base };
    let hz = Retangulo { esq: c.esq, topo: c.base + 1, dir: c.dir, base: i.base };
    [
        (Retangulo { base: v.topo + lado, ..v }, Acao::LinhaAcima),
        (Retangulo { topo: v.base - lado, ..v }, Acao::LinhaAbaixo),
        (Retangulo { dir: hz.esq + lado, ..hz }, Acao::LinhaEsquerda),
        (Retangulo { esq: hz.dir - lado, ..hz }, Acao::LinhaDireita),
    ]
}

/// Monta a tela de jogo do save com a vista de `cfg.edicao.posicao`: fundo, faixa e janela de edição
/// com chão e vida. Devolve o quadro e o que ainda não foi reproduzido.
pub fn compor_tela_de_jogo(
    cfg: &ConfigTelaJogo,
    recursos: &Recursos,
    interface: &Interface,
    jogo: &JogoSalvo,
) -> Result<(Quadro, Vec<String>), String> {
    let tela = TelaDeJogo::carregar(cfg, recursos, interface, jogo)?;
    let nav = tela.navegacao(jogo, cfg.edicao.posicao)?;
    tela.compor(recursos, interface, &nav)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn setas_batem_com_a_moldura_medida() {
        // jogo.md, "Moldura e posição": barra vertical em (424, 158, 16, 304) e horizontal em
        // (24, 462, 400, 16) na tela, que começa 38 linhas acima da área cliente.
        let m: MolduraWindows = serde_json::from_str(
            r#"{"externo": [20, 97, 424, 347], "espessura": 4, "altura_titulo": 18, "barra_rolagem": 16,
                "cor_linha": 0, "cor_moldura": 8, "cor_titulo": 4, "cor_botao": 8,
                "cor_relevo_claro": 15, "cor_relevo_escuro": 7, "cor_trilho": 8}"#,
        )
        .unwrap();
        let s = setas_de_rolagem(&m);
        let r = |a: Acao| s.iter().find(|(_, b)| *b == a).unwrap().0;
        assert_eq!(r(Acao::LinhaAcima), Retangulo { esq: 425, topo: 120, dir: 440, base: 135 });
        assert_eq!(r(Acao::LinhaAbaixo), Retangulo { esq: 425, topo: 409, dir: 440, base: 424 });
        assert_eq!(r(Acao::LinhaEsquerda), Retangulo { esq: 24, topo: 425, dir: 39, base: 440 });
        assert_eq!(r(Acao::LinhaDireita), Retangulo { esq: 409, topo: 425, dir: 424, base: 440 });
    }

    #[test]
    fn cliente_calculado_e_o_mesmo_do_desenho() {
        let m: MolduraWindows = serde_json::from_str(
            r#"{"externo": [20, 97, 424, 347], "espessura": 4, "altura_titulo": 18, "barra_rolagem": 16,
                "cor_linha": 0, "cor_moldura": 8, "cor_titulo": 4, "cor_botao": 8,
                "cor_relevo_claro": 15, "cor_relevo_escuro": 7, "cor_trilho": 8}"#,
        )
        .unwrap();
        let mut q = Quadro::new(640, 480, 0);
        assert_eq!(desenhar_moldura(&mut q, &m).cliente, cliente_da_moldura(&m));
    }
}
