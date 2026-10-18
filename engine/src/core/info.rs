//! Info ("Top Card", janela 5): cartões de uma enciclopédia sobre formigas lidos do banco (tipos 17,
//! 19, 10 e 21). Especificação: docs/spec/telas/info.md; dados: data/info.json.

use serde::Deserialize;

use super::quadro::Quadro;
use super::telas::Interface;
use crate::formats::banco::Chave;
use crate::formats::bitmap::{Bitmap, TIPO_BITMAP};
use crate::formats::fonte::Fonte;
use crate::formats::janela::Retangulo;
use crate::recursos::Recursos;

pub const ARQUIVO_CONFIG: &str = "info.json";

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigInfo {
    pub janela: u16,
    pub objeto_area: String,
    pub carta_inicial: u16,
    pub tipo_corpo: u8,
    pub tipo_titulo: u8,
    pub tipo_texto: u8,
    pub tipo_estilo: u8,
    /// Fonte do texto e dos trechos destacados (índices de interface.json).
    pub fonte_texto: usize,
    pub fonte_destaque: usize,
    pub cor_texto: u8,
    pub cor_fundo: u8,
    pub tamanho_historico: usize,
    pub inverter_mono: bool,
    /// Canto do texto em relação ao retângulo do registro `TEXT` (`_win_PrintStyleTextInRect`).
    pub deslocamento_texto: [i32; 2],
    pub popup: ConfigPopup,
}

/// Popup de definição (`_PopUpInfoWindow`).
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigPopup {
    /// Espessura da moldura da janela (`GetSystemMetrics` 7 e 8, SM_CXDLGFRAME/SM_CYDLGFRAME).
    pub moldura: [i32; 2],
    /// Margem branca entre a moldura e o retângulo do texto.
    pub margem: i32,
    /// Altura somada à do texto.
    pub altura_extra: i32,
    /// Somado ao canto e ao tamanho da área do cartão nas contas do popup [medido].
    pub deslocamento_area: [i32; 2],
    pub ajuste_area: [i32; 2],
    /// Cores da moldura, de fora para dentro (uma por pixel de espessura).
    pub cores_moldura: Vec<u8>,
    pub cor_fundo: u8,
}

/// Área de clique de um `PICT`: retângulo (relativo à área) e valor (1 = voltar, > 1 = cartão,
/// ≤ 0 = popup da definição −valor).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AreaClique {
    pub ret: Retangulo,
    pub valor: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Registro {
    Figura { bitmap: u16, x: i16, y: i16, areas: Vec<AreaClique> },
    Texto { ret: Retangulo, textos: Vec<u16> },
}

/// Um cartão: título e registros do corpo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Carta {
    pub titulo: String,
    pub registros: Vec<Registro>,
}

fn be16(d: &[u8], o: usize) -> Option<i16> {
    d.get(o..o + 2).map(|s| i16::from_be_bytes([s[0], s[1]]))
}

/// Corpo do cartão (tipo 17): 4 bytes de cabeçalho e registros `PICT`/`TEXT`.
pub fn ler_corpo(d: &[u8]) -> Vec<Registro> {
    let mut v = Vec::new();
    let mut o = 4;
    let w = |o: usize| be16(d, o);
    while let Some(tag) = d.get(o..o + 4) {
        o += 4;
        match tag {
            b"PICT" => {
                let (Some(bm), Some(t), Some(l), Some(n)) = (w(o), w(o + 2), w(o + 4), w(o + 10)) else { break };
                o += 12;
                let mut areas = Vec::new();
                for _ in 0..n.max(0) {
                    let (Some(t), Some(l), Some(b), Some(r), Some(valor)) = (w(o), w(o + 2), w(o + 4), w(o + 6), w(o + 8)) else { break };
                    areas.push(AreaClique { ret: Retangulo { esq: l, topo: t, dir: r, base: b }, valor });
                    o += 10;
                }
                v.push(Registro::Figura { bitmap: bm as u16, x: l, y: t, areas });
            }
            b"TEXT" => {
                let (Some(t), Some(l), Some(b), Some(r), Some(n)) = (w(o), w(o + 2), w(o + 4), w(o + 6), w(o + 8)) else { break };
                o += 10;
                let textos = (0..n.max(0) as usize).filter_map(|i| w(o + 2 * i).map(|x| x as u16)).collect();
                o += 2 * n.max(0) as usize;
                v.push(Registro::Texto { ret: Retangulo { esq: l, topo: t, dir: r, base: b }, textos });
            }
            _ => break,
        }
    }
    v
}

/// Trechos do recurso tipo 21 (o `styl` do Mac, big-endian): número de trechos e, por trecho, 20
/// bytes. Devolve (início em bytes, destaque); o trecho é destacado (segunda fonte) quando a
/// palavra das faces (bytes 10–11) vale 0x0100, isto é, só negrito.
pub fn ler_trechos(d: &[u8]) -> Vec<(usize, bool)> {
    let n = be16(d, 0).unwrap_or(0).max(0) as usize;
    (0..n)
        .map_while(|i| {
            let r = d.get(2 + 20 * i..22 + 20 * i)?;
            Some((u32::from_be_bytes([r[0], r[1], r[2], r[3]]) as usize, r[10] == 1 && r[11] == 0))
        })
        .collect()
}

/// Texto de um cartão como `_DisplayCard` o entrega ao desenho: os bytes até o primeiro 0, depois
/// de tirar as sequências de controle que começam num 0 no meio do texto, e os trechos de estilo.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextoCartao {
    pub bytes: Vec<u8>,
    pub trechos: Vec<(usize, bool)>,
}

impl TextoCartao {
    /// Limpeza de `_DisplayCard`: enquanto houver um 0 antes dos 4 últimos bytes, tira ele e os
    /// bytes seguintes menores que 0x20 (comparação com sinal: os ≥ 0x80 também saem), e recua os
    /// trechos que começam depois do que saiu. O texto termina no primeiro 0 que sobrar.
    pub fn novo(bruto: &[u8], estilos: &[u8]) -> TextoCartao {
        let mut t = bruto.to_vec();
        let mut trechos = ler_trechos(estilos);
        loop {
            let limite = t.len().saturating_sub(4);
            let Some(z) = t[..limite].iter().position(|&b| b == 0) else { break };
            let mut f = z;
            while f < limite && (t[f] as i8) < 0x20 {
                f += 1;
            }
            t.drain(z..f);
            for (ini, _) in trechos.iter_mut().filter(|(ini, _)| *ini >= f) {
                *ini -= f - z;
            }
        }
        let fim = t.iter().position(|&b| b == 0).unwrap_or(t.len());
        t.truncate(fim);
        TextoCartao { bytes: t, trechos }
    }

    /// Texto `id` (tipo 10) com os estilos do mesmo id (tipo 21, opcional).
    pub fn ler(recursos: &Recursos, cfg: &ConfigInfo, id: u16) -> Option<TextoCartao> {
        let bruto = recursos.objeto(Chave { tipo: cfg.tipo_texto, id }).ok()?;
        let estilos = recursos.objeto(Chave { tipo: cfg.tipo_estilo, id }).unwrap_or_default();
        Some(TextoCartao::novo(&bruto, &estilos))
    }

    fn destaque_do_trecho(&self, i: usize) -> bool {
        self.trechos.get(i).is_some_and(|t| t.1)
    }

    fn byte(&self, i: usize) -> u8 {
        self.bytes.get(i).copied().unwrap_or(0)
    }
}

/// Ponto de quebra de `_win_PrintStyleTextInRect`: antes do espaço, de '(', '[', '{' e do fim de
/// linha (0, '\n', '\r'); depois de ')', ']', '}', '?', '!', ',', '-' e '.'.
fn quebra_em(c: u8, i: usize) -> Option<usize> {
    match c {
        0 | b'\n' | b'\r' | b' ' | b'(' | b'[' | b'{' => Some(i),
        b')' | b']' | b'}' | b'?' | b'!' | b',' | b'-' | b'.' => Some(i + 1),
        _ => None,
    }
}

/// Pula os espaços e um fim de linha depois do fim de uma linha.
fn proxima_linha(texto: &TextoCartao, mut pos: usize) -> usize {
    while texto.byte(pos) == b' ' {
        pos += 1;
    }
    if matches!(texto.byte(pos), b'\n' | b'\r') {
        pos += 1;
    }
    pos
}

/// Um pedaço de texto numa fonte só: linha, x (somado ao canto do texto) e bytes `ini..fim`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segmento {
    pub linha: usize,
    pub x: i32,
    pub ini: usize,
    pub fim: usize,
    pub destaque: bool,
}

/// Quebra de linha de `_win_PrintStyleTextInRect` [código]. Cada linha soma a largura dos caracteres
/// enquanto a soma for menor que `largura`, guardando o último ponto de quebra de um caractere que
/// coube; fim de linha ou fim do texto encerram a linha. Na entrada de um trecho de estilo, a
/// palavra que começa ali é medida com a fonte de antes: se cabe, o que veio antes vira um segmento
/// e a fonte troca; se não, a linha acaba ali. No máximo `max_linhas` linhas.
/// `medir(c, destaque)` dá a largura de um caractere.
pub fn compor(texto: &TextoCartao, largura: i32, max_linhas: usize, medir: &dyn Fn(u8, bool) -> i32) -> Vec<Segmento> {
    let n = texto.bytes.len();
    let mut saida = Vec::new();
    let mut trecho = 0;
    let mut destaque = texto.destaque_do_trecho(0);
    let (mut pos, mut linha) = (0, 0);
    while pos < n && linha < max_linhas {
        let (mut soma, mut x, mut ini) = (0, 0, pos);
        let mut quebra: Option<usize> = None;
        let mut fim_da_linha = false;
        while !fim_da_linha && soma < largura {
            if let Some(&(inicio, d)) = texto.trechos.get(trecho + 1) {
                if inicio <= pos {
                    let palavra: i32 = texto.bytes[pos.min(n)..]
                        .iter()
                        .take_while(|&&c| !matches!(c, b' ' | b'\n' | b'\r'))
                        .map(|&c| medir(c, destaque))
                        .sum();
                    if soma + palavra >= largura {
                        // Não cabe: a linha acaba um caractere antes da entrada do trecho.
                        pos = pos.saturating_sub(1);
                        break;
                    }
                    if pos > ini {
                        saida.push(Segmento { linha, x, ini, fim: pos, destaque });
                    }
                    trecho += 1;
                    destaque = d;
                    x = soma;
                    ini = pos;
                }
            }
            let c = texto.byte(pos);
            soma += medir(c, destaque);
            if soma < largura {
                if let Some(q) = quebra_em(c, pos) {
                    quebra = Some(q);
                }
                fim_da_linha = matches!(c, 0 | b'\n' | b'\r');
            }
            pos += 1;
        }
        // O original trata a posição 0 como "sem ponto de quebra".
        if let Some(q) = quebra.filter(|&q| q != 0) {
            pos = q;
            ini = ini.min(q);
        }
        if pos > ini {
            saida.push(Segmento { linha, x, ini, fim: pos.min(n), destaque });
        }
        pos = proxima_linha(texto, pos);
        linha += 1;
    }
    saida
}

/// Número de linhas de `_win_GetStyleTextHeight` [código]: a mesma soma por linha, mas a fonte
/// troca ao alcançar cada trecho, sem medir a palavra antes.
pub fn contar_linhas(texto: &TextoCartao, largura: i32, medir: &dyn Fn(u8, bool) -> i32) -> usize {
    let n = texto.bytes.len();
    let (mut trecho, mut destaque) = (0, texto.destaque_do_trecho(0));
    let (mut pos, mut linhas) = (0, 0);
    // Guarda contra o laço sem fim do original (linha que só quebra na própria entrada).
    while pos < n && linhas <= n {
        let mut soma = 0;
        let mut quebra = 0;
        let mut fim_da_linha = false;
        while !fim_da_linha && soma < largura {
            if let Some(&(inicio, d)) = texto.trechos.get(trecho + 1) {
                if inicio <= pos {
                    trecho += 1;
                    destaque = d;
                }
            }
            let c = texto.byte(pos);
            soma += medir(c, destaque);
            if soma < largura {
                if let Some(q) = quebra_em(c, pos) {
                    quebra = q;
                }
                fim_da_linha = matches!(c, 0 | b'\n' | b'\r');
            }
            pos += 1;
        }
        if quebra != 0 {
            pos = quebra;
        }
        pos = proxima_linha(texto, pos);
        linhas += 1;
    }
    linhas
}

impl Carta {
    pub fn ler(recursos: &Recursos, cfg: &ConfigInfo, id: u16) -> Result<Carta, String> {
        let corpo = recursos.objeto(Chave { tipo: cfg.tipo_corpo, id })?;
        let titulo = recursos
            .objeto(Chave { tipo: cfg.tipo_titulo, id })
            .map(|d| d.split(|&b| b == 0).next().unwrap_or(&[]).iter().map(|&b| if b == b'_' { ' ' } else { b as char }).collect())
            .unwrap_or_default();
        Ok(Carta { titulo, registros: ler_corpo(&corpo) })
    }
}

/// Popup de definição aberto enquanto o botão fica apertado: texto e a área clicada (relativa à
/// área do cartão): centro em x, topo e base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Popup {
    pub texto: u16,
    pub centro_x: i32,
    pub topo: i32,
    pub base: i32,
}

/// Estado da janela Info (fora do save): cartão atual, o anel do histórico e o popup aberto.
#[derive(Clone, Debug)]
pub struct JanelaInfo {
    pub cfg: ConfigInfo,
    pub atual: u16,
    historico: Vec<u16>,
    indice: usize,
    pub carta: Option<Carta>,
    pub popup: Option<Popup>,
}

impl JanelaInfo {
    pub fn new(cfg: ConfigInfo) -> JanelaInfo {
        let n = cfg.tamanho_historico.max(1);
        let mut historico = vec![0; n];
        historico[0] = cfg.carta_inicial;
        JanelaInfo { atual: cfg.carta_inicial, historico, indice: 0, carta: None, popup: None, cfg }
    }

    /// `_OpenInfoWindow` com a janela fechada: volta ao cartão inicial e zera o histórico.
    pub fn reiniciar(&mut self) {
        let c = self.cfg.carta_inicial;
        self.historico.iter_mut().for_each(|h| *h = 0);
        self.historico[0] = c;
        self.indice = 0;
        self.atual = c;
        self.carta = None;
        self.popup = None;
    }

    /// Lê o cartão atual se ainda não foi lido.
    pub fn carregar(&mut self, recursos: &Recursos) {
        if self.carta.is_none() {
            match Carta::ler(recursos, &self.cfg, self.atual) {
                Ok(c) => self.carta = Some(c),
                Err(e) => eprintln!("info: cartão {}: {e}", self.atual),
            }
        }
    }

    fn ir_para(&mut self, id: u16) {
        let n = self.historico.len();
        self.indice = (self.indice + 1) % n;
        self.historico[self.indice] = id;
        self.historico[(self.indice + 1) % n] = 0;
        self.atual = id;
        self.carta = None;
    }

    fn voltar(&mut self) {
        let n = self.historico.len();
        let anterior = (self.indice + n - 1) % n;
        if self.historico[anterior] != 0 {
            self.indice = anterior;
            self.atual = self.historico[anterior];
            self.carta = None;
        }
    }

    /// Clique em (x, y) relativo à área do cartão (`_ProcInfoEvent`): as áreas das figuras, a
    /// primeira que contém o ponto. Devolve se algo mudou (cartão ou popup).
    pub fn clicar(&mut self, x: i32, y: i32) -> bool {
        let Some(c) = &self.carta else { return false };
        let area = c.registros.iter().find_map(|r| match r {
            Registro::Figura { areas, .. } => areas.iter().find(|a| {
                x >= i32::from(a.ret.esq) && x < i32::from(a.ret.dir) && y >= i32::from(a.ret.topo) && y < i32::from(a.ret.base)
            }),
            _ => None,
        });
        let Some(a) = area.copied() else { return false };
        match a.valor {
            1 => {
                let antes = self.atual;
                self.voltar();
                self.atual != antes
            }
            v if v > 1 => {
                self.ir_para(v as u16);
                true
            }
            v => {
                let (l, r) = (i32::from(a.ret.esq), i32::from(a.ret.dir));
                self.popup = Some(Popup { texto: (-i32::from(v)) as u16, centro_x: (r - l) / 2 + l, topo: i32::from(a.ret.topo), base: i32::from(a.ret.base) });
                true
            }
        }
    }

    /// Botão solto: fecha o popup (fim do laço de `_StillDown`). Devolve se havia um.
    pub fn soltar(&mut self) -> bool {
        self.popup.take().is_some()
    }

    /// Desenha o cartão em `area` (canto da área da janela) e, por cima, o popup aberto.
    pub fn desenhar(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface, area: Retangulo) {
        // A linha de cima da área é a linha do título; o fundo começa uma abaixo.
        q.preencher(Retangulo { topo: area.topo + 1, ..area }, self.cfg.cor_fundo, self.cfg.cor_fundo);
        let Some(c) = &self.carta else { return };
        let fonte = |i| interface.fontes.get(i).and_then(|f: &Option<Fonte>| f.as_ref());
        let fontes = fonte(self.cfg.fonte_texto).map(|n| (n, fonte(self.cfg.fonte_destaque).unwrap_or(n)));
        let antes = q.recorte.replace(area);
        let (ox, oy) = (i32::from(area.esq), i32::from(area.topo));
        for r in &c.registros {
            match r {
                Registro::Figura { bitmap, x, y, .. } => {
                    let Ok(d) = recursos.objeto(Chave { tipo: TIPO_BITMAP, id: *bitmap }) else { continue };
                    let Ok(mut b) = Bitmap::decodificar(&d) else { continue };
                    // Figuras monocromáticas dos cartões (herança do Mac): bit 1 = tinta preta, o
                    // inverso do que `Bitmap` supõe para o mono [medido em j03].
                    if self.cfg.inverter_mono && d.get(2) == Some(&0x01) {
                        b.pixels.iter_mut().for_each(|p| *p = if *p == 0 { 15 } else { 0 });
                    }
                    q.bitmap(&b, ox + i32::from(*x), oy + i32::from(*y));
                }
                Registro::Texto { ret, textos } => {
                    let Some((normal, destaque)) = fontes else { continue };
                    // Os 53 cartões só têm registros `TEXT` com um texto; com mais de um, o port
                    // escreve todos no mesmo retângulo [hipótese; o original não foi visto assim].
                    let ret = Retangulo { esq: ret.esq + area.esq, topo: ret.topo + area.topo, dir: ret.dir + area.esq, base: ret.base + area.topo };
                    for &id in textos {
                        if let Some(t) = TextoCartao::ler(recursos, &self.cfg, id) {
                            self.escrever(q, normal, destaque, &t, ret);
                        }
                    }
                }
            }
        }
        q.recorte = antes;
        if let (Some(p), Some((normal, destaque))) = (&self.popup, fontes) {
            self.desenhar_popup(q, recursos, normal, destaque, area, p);
        }
    }

    /// `_win_PrintStyleTextInRect` no retângulo `ret` (absoluto): o texto começa em
    /// (esq, topo) + `deslocamento_texto` e tem no máximo (base − topo) / altura linhas.
    fn escrever(&self, q: &mut Quadro, normal: &Fonte, destaque: &Fonte, t: &TextoCartao, ret: Retangulo) {
        let alto = normal.altura().max(destaque.altura()) as i32;
        let largura = i32::from(ret.dir) - i32::from(ret.esq);
        let max_linhas = ((i32::from(ret.base) - i32::from(ret.topo)) / alto.max(1)).max(0) as usize;
        let fonte = |d: bool| if d { destaque } else { normal };
        let medir = |c: u8, d: bool| fonte(d).largura_texto(&[c]) as i32;
        let [dx, dy] = self.cfg.deslocamento_texto;
        for s in compor(t, largura, max_linhas, &medir) {
            let img = fonte(s.destaque).imagem(&t.bytes[s.ini..s.fim]);
            let (x0, y0) = (i32::from(ret.esq) + dx + s.x, i32::from(ret.topo) + dy + s.linha as i32 * alto);
            for j in 0..img.altura {
                for i in 0..img.largura {
                    if img.bits[j * img.largura + i] {
                        q.ponto(x0 + i as i32, y0 + j as i32, self.cfg.cor_texto);
                    }
                }
            }
        }
    }

    /// Geometria do popup (`_PopUpInfoWindow`) [código + medido em p911/p912], relativa à área do
    /// cartão (com `ajuste_area`): largura do texto = metade da área, centrada na área clicada e
    /// mantida 4 pixels dentro da área; abaixo da área clicada (4 pixels de folga) ou, se não couber
    /// até 10 pixels da base, acima. A janela começa `moldura + margem` à esquerda e acima do texto,
    /// mas a largura passada ao Windows só soma isso uma vez (a janela acaba no fim do texto); na
    /// vertical soma dos dois lados e fica dentro da área. Devolve a janela ([esq, dir) × [topo,
    /// base)) e o retângulo do texto (dentro da borda e da margem; pode passar da janela).
    pub fn geometria_popup(&self, area: Retangulo, p: &Popup, altura_texto: i32) -> (Retangulo, Retangulo) {
        let [aw, ah] = self.cfg.popup.ajuste_area;
        let lw = i32::from(area.dir) - i32::from(area.esq) + aw;
        let lh = i32::from(area.base) - i32::from(area.topo) + ah;
        let meia = lw / 2;
        let (mut esq, mut dir) = (p.centro_x - meia / 2, p.centro_x + meia / 2);
        if esq < 4 {
            esq = 4;
            dir = 4 + meia + 8;
        }
        if dir > lw - 4 {
            esq = lw - 4 - meia - 8;
            dir = lw - 4;
        }
        let h = altura_texto + self.cfg.popup.altura_extra;
        let (cima, baixo) = (p.topo - 4, p.base + 4);
        let topo = if lh - 10 < h + baixo { cima - h } else { baixo };
        let [mx, my] = self.cfg.popup.moldura;
        let m = self.cfg.popup.margem;
        let (fx, fy) = (mx + m, my + m);
        let mut jt = topo - fy;
        if jt < 0 {
            jt = 0;
        } else if jt + h + 2 * fy > lh {
            jt = lh - 2 * fy - h;
        }
        let [dx, dy] = self.cfg.popup.deslocamento_area;
        let (ax, ay) = (i32::from(area.esq) + dx, i32::from(area.topo) + dy);
        let r = |l: i32, t: i32, rr: i32, b: i32| Retangulo { esq: (ax + l) as i16, topo: (ay + t) as i16, dir: (ax + rr) as i16, base: (ay + b) as i16 };
        let x0 = esq - fx;
        let janela = r(x0, jt, dir, jt + h + 2 * fy);
        let borda = self.cfg.popup.cores_moldura.len() as i32;
        let (tx, ty) = (x0 + borda + m, jt + borda + m);
        (janela, r(tx, ty, tx + (dir - esq), ty + h))
    }

    fn desenhar_popup(&self, q: &mut Quadro, recursos: &Recursos, normal: &Fonte, destaque: &Fonte, area: Retangulo, p: &Popup) {
        let Some(t) = TextoCartao::ler(recursos, &self.cfg, p.texto) else { return };
        let alto = normal.altura().max(destaque.altura()) as i32;
        let medir = |c: u8, d: bool| if d { destaque } else { normal }.largura_texto(&[c]) as i32;
        let largura = (i32::from(area.dir) - i32::from(area.esq) + self.cfg.popup.ajuste_area[0]) / 2;
        let altura_texto = contar_linhas(&t, largura, &medir) as i32 * alto;
        let (janela, texto) = self.geometria_popup(area, p, altura_texto);
        let mut r = janela;
        for &cor in &self.cfg.popup.cores_moldura {
            q.contorno(Retangulo { dir: r.dir - 1, base: r.base - 1, ..r }, cor);
            r = Retangulo { esq: r.esq + 1, topo: r.topo + 1, dir: r.dir - 1, base: r.base - 1 };
        }
        q.preencher(r, self.cfg.popup.cor_fundo, self.cfg.popup.cor_fundo);
        // O texto fica recortado na área cliente da janela.
        let antes = q.recorte.replace(r);
        self.escrever(q, normal, destaque, &t, texto);
        q.recorte = antes;
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn corpo_do_cartao_128() {
        // Os primeiros registros do cartão 128 (SHARED tipo 17), big-endian.
        let mut d = vec![0x00, 0xc8, 0x01, 0x40];
        d.extend(b"PICT");
        for w in [21005u16, 180, 0, 200, 320, 1, 184, 128, 197, 192, 1] {
            d.extend(w.to_be_bytes());
        }
        d.extend(b"TEXT");
        for w in [1u16, 176, 180, 320, 1, 150] {
            d.extend(w.to_be_bytes());
        }
        let r = ler_corpo(&d);
        assert_eq!(r.len(), 2);
        assert_eq!(
            r[0],
            Registro::Figura { bitmap: 21005, x: 0, y: 180, areas: vec![AreaClique { ret: Retangulo { esq: 128, topo: 184, dir: 192, base: 197 }, valor: 1 }] }
        );
        assert_eq!(r[1], Registro::Texto { ret: Retangulo { esq: 176, topo: 1, dir: 320, base: 180 }, textos: vec![150] });
    }

    fn estilos(trechos: &[(u32, u8)]) -> Vec<u8> {
        let mut d = (trechos.len() as u16).to_be_bytes().to_vec();
        for &(ini, face) in trechos {
            d.extend(ini.to_be_bytes());
            d.extend([0, 12, 0, 10, 0, 3, face, 0, 0, 9, 0, 0, 0, 0, 0, 0]);
        }
        d
    }

    /// Texto 310 (cartão 135, "Ants"): um 0 no meio, seguido de controles, não corta o texto.
    #[test]
    fn zero_no_meio_do_texto_sai_com_os_controles() {
        let bruto = b"the premier socia\0\0\x01l insects. Sold\0\0\x01\0iers\0";
        let t = TextoCartao::novo(bruto, &estilos(&[(0, 0), (24, 1), (30, 0)]));
        assert_eq!(t.bytes, b"the premier social insects. Soldiers");
        // Trechos depois do primeiro corte recuam 3 bytes.
        assert_eq!(t.trechos, [(0, false), (21, true), (27, false)]);
        // Um 0 nos 4 últimos bytes é o fim.
        assert_eq!(TextoCartao::novo(b"abc\0\0\0", &[]).bytes, b"abc");
    }

    #[test]
    fn faces_diferentes_de_negrito_nao_destacam() {
        assert_eq!(ler_trechos(&estilos(&[(0, 0), (5, 1), (9, 4)])), [(0, false), (5, true), (9, false)]);
    }

    /// Largura fixa de 1 por caractere e 2 no destaque.
    fn medir(_: u8, d: bool) -> i32 {
        if d {
            2
        } else {
            1
        }
    }

    fn linhas(t: &TextoCartao, largura: i32) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for s in compor(t, largura, 100, &medir) {
            if v.len() <= s.linha {
                v.resize(s.linha + 1, String::new());
            }
            v[s.linha] += &String::from_utf8_lossy(&t.bytes[s.ini..s.fim]);
        }
        v
    }

    /// A palavra só fica na linha se ela e o caractere de quebra seguinte somarem menos que a
    /// largura (a causa do "using" de j03).
    #[test]
    fn quebra_exige_o_espaco_seguinte_dentro_da_largura() {
        let t = TextoCartao::novo(b"aa bb cc", &[]);
        // O 0 do fim também é medido: "cc" só fica com "bb" se o 0 couber.
        assert_eq!(linhas(&t, 6), ["aa", "bb", "cc"]);
        assert_eq!(linhas(&t, 7), ["aa bb", "cc"]);
        let t = TextoCartao::novo(b"x-y z\rw", &[]);
        assert_eq!(linhas(&t, 3), ["x-", "y", "z", "w"]);
    }

    #[test]
    fn trecho_destacado_e_segmento_proprio() {
        let t = TextoCartao::novo(b"ab cd ef", &estilos(&[(0, 0), (3, 1), (5, 0)]));
        let s = compor(&t, 100, 10, &medir);
        assert_eq!(
            s,
            [
                Segmento { linha: 0, x: 0, ini: 0, fim: 3, destaque: false },
                Segmento { linha: 0, x: 3, ini: 3, fim: 5, destaque: true },
                Segmento { linha: 0, x: 7, ini: 5, fim: 8, destaque: false },
            ]
        );
        // Limite de linhas.
        assert_eq!(compor(&TextoCartao::novo(b"a b c d", &[]), 3, 2, &medir).len(), 2);
        assert_eq!(contar_linhas(&TextoCartao::novo(b"a b c d", &[]), 3, &medir), 4);
    }

    #[test]
    fn historico_anda_e_volta() {
        let cfg: ConfigInfo = serde_json::from_str(include_str!("../../../data/info.json")).unwrap();
        let mut j = JanelaInfo::new(cfg);
        let area = |valor| Registro::Figura { bitmap: 0, x: 0, y: 0, areas: vec![AreaClique { ret: Retangulo { esq: 0, topo: 0, dir: 10, base: 10 }, valor }] };
        j.carta = Some(Carta { titulo: String::new(), registros: vec![area(134)] });
        assert!(j.clicar(5, 5));
        assert_eq!(j.atual, 134);
        j.carta = Some(Carta { titulo: String::new(), registros: vec![area(1)] });
        assert!(j.clicar(5, 5));
        assert_eq!(j.atual, 128);
        j.carta = Some(Carta { titulo: String::new(), registros: vec![area(1)] });
        assert!(!j.clicar(5, 5), "antes do primeiro não há para onde voltar");
        j.carta = Some(Carta { titulo: String::new(), registros: vec![area(-912)] });
        assert!(j.clicar(5, 5));
        assert_eq!(j.popup, Some(Popup { texto: 912, centro_x: 5, topo: 0, base: 10 }));
        assert!(j.soltar());
        assert_eq!(j.popup, None);
    }

    /// Popups das áreas do cartão 131 (Head) medidos na bancada (p912 e p911, área do cartão em
    /// (280, 260) na tela para o desenho, 1 pixel à esquerda para o popup): Ocelli (3 linhas) abre abaixo na janela (315, 288)–(480, 338);
    /// Maxillae (5 linhas) não cabe abaixo e abre acima, presa a 4 pixels da esquerda, na janela
    /// (275, 309)–(449, 379) (coordenadas inclusivas).
    #[test]
    fn popup_abaixo_ou_acima_da_area() {
        let cfg: ConfigInfo = serde_json::from_str(include_str!("../../../data/info.json")).unwrap();
        let j = JanelaInfo::new(cfg);
        let area = Retangulo { esq: 280, topo: 260, dir: 280 + 320, base: 260 + 200 };
        let (jan, t) = j.geometria_popup(area, &Popup { texto: 0, centro_x: 123, topo: 19, base: 32 }, 30);
        assert_eq!((jan.esq, jan.topo, jan.dir - 1, jan.base - 1), (315, 288, 480, 338));
        assert_eq!((t.esq, t.topo, t.dir - t.esq), (324, 297, 158));
        let (jan, _) = j.geometria_popup(area, &Popup { texto: 0, centro_x: 41, topo: 116, base: 131 }, 50);
        assert_eq!((jan.esq, jan.topo, jan.dir - 1, jan.base - 1), (275, 309, 449, 379));
        let (jan, _) = j.geometria_popup(area, &Popup { texto: 0, centro_x: 41, topo: 116, base: 131 }, 40);
        assert_eq!(jan.topo, 260 + 135 - 8, "4 linhas cabem abaixo");
    }
}
