//! Janela Status (Window → Status, Ctrl+T): `_ScoreDialog` (`1000:5caa`) abre a janela 24 com as notas de
//! `_CalcScore` e `_win_DrawScoreWindow` (`1000:5aca`) escreve as do universo e a pontuação. Fica aberta
//! até um clique ou Espaço/Insert (`_win_Events`) ou Esc (`_DialogAbort`). Especificação:
//! docs/spec/telas/menus.md, "Status".

use serde::Deserialize;

use super::avisos::{calcular_pontuacao, linhas_do_texto, objeto_na_tela, parciais, ConfigPontuacao};
use super::quadro::Quadro;
use super::telas::{carregar_janela, desenhar_janela_deslocada, Interface};
use crate::formats::janela::{Conteudo, Cor, Retangulo};
use crate::recursos::Recursos;
use crate::rules::estado::Estado;

/// Parte "status" de data/barra_menus.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigStatus {
    pub janela: u16,
    /// Fonte dos textos (`_font_SetFont(4)`).
    pub fonte: usize,
    /// Objetos das notas 0–3 (`_win_SetObjFormatStr`, com o formato do próprio objeto).
    pub objetos_colonia: Vec<String>,
    /// Objetos das notas 4–7 (`_win_PrintfAtObj`).
    pub objetos_universo: Vec<String>,
    /// Formato das notas do universo (DGROUP 0x956, "%d%%"), só no tipo `tipo_com_universo`.
    pub formato_universo: String,
    pub tipo_com_universo: i16,
    /// Lista de textos (4/1800): "N.A." no índice `indice_na`, título em `indices_titulo` (antes e depois
    /// do nome do tipo de jogo), e as duas linhas do Experimental em `indices_experimental`.
    pub lista: u16,
    pub indice_na: usize,
    pub indices_titulo: [usize; 2],
    pub tipo_experimental: i16,
    pub indices_experimental: [usize; 2],
    /// Nomes dos tipos de jogo (4/1001).
    pub lista_tipos: u16,
    pub objeto_titulo: String,
    pub objeto_pontos: String,
    /// Formato da pontuação (DGROUP 0x95b, "%ld").
    pub formato_pontos: String,
    /// Texto do pé (`_win_SetObjFormatStr(0x180c, …)`, lista 4/1010 índice 14).
    pub objeto_continuar: String,
    pub lista_continuar: u16,
    pub indice_continuar: usize,
    /// Deslocamento da janela medido na bancada. [0, 0] desde 2026-10-19: os −3 medidos eram o
    /// alinhamento ao byte da área cliente, que `Interface::posicionar` já faz.
    pub ajuste: [i16; 2],
    pub moldura: MolduraDialogo,
    /// Fundo da área cliente entre a moldura e o bitmap (o fundo da janela 24, branco em n02: 3
    /// pixels em volta do `antEffPic`). Opcional (API 0.32); sem ele, fica a cor da borda.
    #[serde(default)]
    pub cor_fundo_janela: Option<u8>,
}

/// `%d`, `%ld`, `%s` e `%%` de um formato do original, com um argumento.
fn formatar(formato: &str, arg: &str) -> String {
    let mut s = String::new();
    let mut it = formato.chars().peekable();
    while let Some(c) = it.next() {
        if c != '%' {
            s.push(c);
            continue;
        }
        match it.peek() {
            Some('%') => {
                it.next();
                s.push('%');
            }
            _ => {
                // Pula as letras do especificador (d, ld, s).
                while let Some(&l) = it.peek() {
                    it.next();
                    if l != 'l' {
                        break;
                    }
                }
                s.push_str(arg);
            }
        }
    }
    s
}

fn bytes(s: &str) -> Vec<u8> {
    s.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect()
}

/// Os textos de cada objeto da janela Status para o estado `e` (objeto, texto).
pub fn textos(recursos: &Recursos, cfg: &ConfigStatus, pontuacao: &ConfigPontuacao, e: &Estado) -> Vec<(String, Vec<u8>)> {
    let notas = parciais(e, pontuacao);
    let pontos = calcular_pontuacao(e, pontuacao);
    let lista = linhas_do_texto(recursos, cfg.lista);
    let linha = |i: usize| lista.get(i).cloned().unwrap_or_default();
    let tipo = e.partida.tipo_jogo;
    let janela = carregar_janela(recursos, cfg.janela).ok();
    let formato_do_objeto = |nome: &str| {
        janela
            .as_ref()
            .and_then(|j| j.objetos.iter().find(|o| o.nome == nome))
            .and_then(|o| match &o.conteudo {
                Conteudo::Texto { texto, .. } => Some(texto.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "%d".into())
    };
    let mut v = Vec::new();
    for (i, nome) in cfg.objetos_colonia.iter().enumerate() {
        v.push((nome.clone(), bytes(&formatar(&formato_do_objeto(nome), &notas[i].to_string()))));
    }
    for (i, nome) in cfg.objetos_universo.iter().enumerate() {
        let t = if tipo == cfg.tipo_com_universo { bytes(&formatar(&cfg.formato_universo, &notas[4 + i].to_string())) } else { linha(cfg.indice_na) };
        v.push((nome.clone(), t));
    }
    if tipo == cfg.tipo_experimental {
        v.push((cfg.objeto_titulo.clone(), linha(cfg.indices_experimental[0])));
        v.push((cfg.objeto_pontos.clone(), linha(cfg.indices_experimental[1])));
    } else {
        let tipos = linhas_do_texto(recursos, cfg.lista_tipos);
        let mut t = linha(cfg.indices_titulo[0]);
        t.extend(tipos.get(usize::try_from(tipo).unwrap_or(0)).cloned().unwrap_or_default());
        t.extend(linha(cfg.indices_titulo[1]));
        v.push((cfg.objeto_titulo.clone(), t));
        v.push((cfg.objeto_pontos.clone(), bytes(&formatar(&cfg.formato_pontos, &pontos.to_string()))));
    }
    let continuar = linhas_do_texto(recursos, cfg.lista_continuar).get(cfg.indice_continuar).cloned().unwrap_or_default();
    v.push((cfg.objeto_continuar.clone(), continuar));
    v
}

/// Moldura de diálogo do Windows (`WS_DLGFRAME`) das janelas modais abertas pelos menus: dentro do
/// contorno preto da borda, `espessura` linhas na cor `cor` (o azul do título ativo) [medido em n02 e
/// n09: 4 linhas azuis; as 3 seguintes são a cor 16 da borda do port].
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct MolduraDialogo {
    pub cor: u8,
    pub espessura: i16,
}

/// Pinta a moldura de diálogo da janela `id` (já desenhada com `ajuste`).
pub fn pintar_moldura(q: &mut Quadro, interface: &Interface, id: u16, janela: &crate::formats::janela::Janela, ajuste: [i16; 2], m: MolduraDialogo) {
    let Some(r) = interface.posicionar(id, janela).and_then(|g| g.externo) else { return };
    let r = Retangulo { esq: r.esq + ajuste[0], topo: r.topo + ajuste[1], dir: r.dir + ajuste[0], base: r.base + ajuste[1] };
    for k in 1..=m.espessura {
        q.contorno(Retangulo { esq: r.esq + k, topo: r.topo + k, dir: r.dir - 1 - k, base: r.base - 1 - k }, m.cor);
    }
}

/// Desenha a janela Status com os textos (`textos`). Devolve o que não foi desenhado.
pub fn desenhar(q: &mut Quadro, recursos: &Recursos, interface: &Interface, cfg: &ConfigStatus, textos: &[(String, Vec<u8>)]) -> Result<Vec<String>, String> {
    let mut janela = carregar_janela(recursos, cfg.janela)?;
    // Os objetos de texto com formato são escritos aqui (o desenho genérico não sabe o argumento).
    for o in janela.objetos.iter_mut() {
        if textos.iter().any(|(n, _)| *n == o.nome) {
            o.flags &= !1;
        }
    }
    let mut pendentes = desenhar_janela_deslocada(q, recursos, interface, cfg.janela, &janela, (cfg.ajuste[0], cfg.ajuste[1]))?;
    pintar_moldura(q, interface, cfg.janela, &janela, cfg.ajuste, cfg.moldura);
    if let (Some(cor), Some(g), Some(fig)) = (cfg.cor_fundo_janela, interface.posicionar(cfg.janela, &janela), janela.objetos.get(1)) {
        // A faixa entre a área cliente e o primeiro objeto (o bitmap), no fundo da janela.
        if let Some(i) = g.interno {
            let o = janela.objetos[0].retangulo;
            let margem = (fig.retangulo.esq - o.esq).min(fig.retangulo.topo - o.topo);
            let i = Retangulo { esq: i.esq + cfg.ajuste[0], topo: i.topo + cfg.ajuste[1], dir: i.dir + cfg.ajuste[0], base: i.base + cfg.ajuste[1] };
            for k in 0..margem.max(0) {
                q.contorno(Retangulo { esq: i.esq + k, topo: i.topo + k, dir: i.dir - 1 - k, base: i.base - 1 - k }, cor);
            }
        }
    }
    let janela = carregar_janela(recursos, cfg.janela)?;
    let Some(fonte) = interface.fontes.get(cfg.fonte).and_then(|f| f.as_ref()) else {
        pendentes.push(format!("font {}", cfg.fonte));
        return Ok(pendentes);
    };
    for (nome, t) in textos {
        let (r, cor) = objeto_na_tela(interface, cfg.janela, &janela, nome, cfg.ajuste)?;
        let fonte_obj = janela
            .objetos
            .iter()
            .find(|o| o.nome == *nome)
            .and_then(|o| match o.conteudo {
                Conteudo::Texto { fonte, .. } => interface.fontes.get(usize::from(fonte)).and_then(|f| f.as_ref()),
                _ => None,
            })
            .unwrap_or(fonte);
        // _win_PrintfAtObj: texto centrado e o resto do objeto na cor de fundo, recortado ao objeto
        // no retângulo do `_win_Recalc` (direita e base − 1, `Retangulo::do_recalc`; medido em n02).
        // O fim de jogo fica uma linha abaixo dessa regra [em aberto].
        let r = r.do_recalc();
        let c = interface.cores.get(cor as usize).copied().unwrap_or(Cor { frente: 0, fundo: 15, fundo2: 15 });
        let antes = q.recorte.replace(r);
        q.texto(r, fonte_obj, t, 3, c.frente, c.fundo);
        q.recorte = antes;
    }
    Ok(pendentes)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn formatos_do_original() {
        assert_eq!(formatar("%d%%", "97"), "97%");
        assert_eq!(formatar("%ld", "19806"), "19806");
        assert_eq!(formatar("%s", "Click"), "Click");
    }
}
