//! Tela de jogo: faixa (janela 34) e janela de edição (janela 0) com o mapa e a vida de um jogo salvo.
//! Especificação: docs/spec/telas/jogo.md. Geometria e cores em data/tela_jogo.json.

use std::collections::HashMap;

use serde::Deserialize;

use super::edicao::{desenhar_vista, tiles_para, CenaEdicao, VistaEdicao};
use super::mapa::{MapaChao, Plano};
use super::quadro::Quadro;
use super::telas::{carregar_janela, desenhar_janela_carregada_na_tela, Interface};
use super::vida::{plano_de, reconstruir, FormigaAmarela};
use crate::formats::banco::Chave;
use crate::formats::janela::Retangulo;
use crate::formats::save::JogoSalvo;
use crate::formats::tiles::{self, Tileset};
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

/// Monta a tela de jogo: fundo, faixa e janela de edição com chão e vida.
/// Devolve o quadro e o que ainda não foi reproduzido.
pub fn compor_tela_de_jogo(
    cfg: &ConfigTelaJogo,
    recursos: &Recursos,
    interface: &Interface,
    jogo: &JogoSalvo,
) -> Result<(Quadro, Vec<String>), String> {
    let mut q = Quadro::new(cfg.largura as usize, cfg.altura as usize, cfg.fundo);
    let id_faixa = interface.moldura.faixa.janela;
    let mut faixa = carregar_janela(recursos, id_faixa)?;
    // _SetMapPlaneLocation: com faixa, seleciona o botão do plano mostrado.
    let plano_mapa = jogo.i16("MapPlane")?;
    if let Some(nome) = cfg.botao_do_plano.get(&plano_mapa.to_string()) {
        let ob = faixa
            .objetos
            .iter_mut()
            .find(|o| &o.nome == nome)
            .ok_or(format!("janela {id_faixa}: objeto {nome} não existe"))?;
        ob.flags |= FLAG_SELECIONADO;
    }
    let mut pendentes = desenhar_janela_carregada_na_tela(&mut q, recursos, interface, id_faixa, &faixa)?;

    let e = &cfg.edicao;
    let janela = carregar_janela(recursos, e.janela)?;
    let area = janela
        .objetos
        .iter()
        .find(|o| o.nome == e.area)
        .ok_or(format!("janela {}: objeto {} não existe", e.janela, e.area))?
        .retangulo;
    let ajuste = if interface.moldura.faixa.altura > 0 { e.ajuste_area_com_faixa } else { 0 };
    let largura = tiles_para((area.dir - area.esq + ajuste) as i32);
    let altura = tiles_para((area.base - area.topo + ajuste) as i32);
    let vista = vista_do_save(jogo, e.posicao, largura, altura)?;

    let partes = desenhar_moldura(&mut q, &e.moldura);
    let [ax, ay] = e.area_na_tela;
    let base = janela.retangulo();
    let origem = (ax - (area.esq - base.esq), ay - (area.topo - base.topo));
    let objs = super::telas::desenhar_objetos(&mut q, recursos, interface, &janela, origem, Some(partes.cliente));
    pendentes.extend(objs);

    let rec = reconstruir(jogo)?;
    pendentes.extend(rec.pendencias);
    let terreno = jogo.u16("TERRAINset")?;
    let chao = tileset(recursos, if terreno == 1 { tiles::ID_CHAO_CASA } else { tiles::ID_CHAO_QUINTAL })?;
    let ids_vida = if vista.plano == Plano::Superficie { tiles::IDS_VIDA_SUPERFICIE } else { tiles::IDS_VIDA_NINHO };
    let vida = tileset_vida(recursos, &ids_vida)?;
    let simbolo = bloco_do_mapa(vista.plano);
    let bytes = jogo.bloco(simbolo).ok_or(format!("bloco {simbolo} ausente"))?;
    let me = FormigaAmarela::do_save(jogo)?;
    if plano_de(me.plano).is_none() {
        pendentes.push(format!("_MePlane = {}", me.plano));
    }
    let cena = CenaEdicao {
        chao: MapaChao::new(vista.plano, bytes)?,
        vida: &rec.camadas,
        amarela: me,
        tiles_chao: &chao,
        tiles_vida: &vida,
    };
    q.recorte = Some(partes.cliente);
    let r = desenhar_vista(&mut q, &vista, &cena, (ax as i32, ay as i32));
    q.recorte = None;
    r?;
    Ok((q, pendentes))
}
