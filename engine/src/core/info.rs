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
    /// Deslocamento do texto em relação ao retângulo do registro `TEXT` [medido em j03].
    pub deslocamento_texto: [i32; 2],
    /// Margem à direita do retângulo na quebra de linha [medido em j03].
    pub margem_direita: i32,
}

/// Área de clique de um `PICT`: retângulo (relativo à área) e valor (1 = voltar, > 1 = cartão).
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

/// Trechos em destaque (estilo ≠ 0) do recurso tipo 21: (início, fim) em bytes do texto.
pub fn ler_destaques(d: &[u8], tamanho_texto: usize) -> Vec<(usize, usize)> {
    let Some(n) = be16(d, 0) else { return Vec::new() };
    let inicio = |i: usize| d.get(2 + 20 * i..6 + 20 * i).map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]) as usize);
    let mut v = Vec::new();
    for i in 0..n.max(0) as usize {
        let (Some(a), Some(&estilo)) = (inicio(i), d.get(2 + 20 * i + 10)) else { break };
        if estilo != 0 {
            let b = inicio(i + 1).unwrap_or(tamanho_texto);
            v.push((a, b.min(tamanho_texto)));
        }
    }
    v
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

/// Estado da janela Info (fora do save): cartão atual e o anel do histórico.
#[derive(Clone, Debug)]
pub struct JanelaInfo {
    pub cfg: ConfigInfo,
    pub atual: u16,
    historico: Vec<u16>,
    indice: usize,
    pub carta: Option<Carta>,
}

impl JanelaInfo {
    pub fn new(cfg: ConfigInfo) -> JanelaInfo {
        let n = cfg.tamanho_historico.max(1);
        let mut historico = vec![0; n];
        historico[0] = cfg.carta_inicial;
        JanelaInfo { atual: cfg.carta_inicial, historico, indice: 0, carta: None, cfg }
    }

    /// `_OpenInfoWindow` com a janela fechada: volta ao cartão inicial e zera o histórico.
    pub fn reiniciar(&mut self) {
        let c = self.cfg.carta_inicial;
        self.historico.iter_mut().for_each(|h| *h = 0);
        self.historico[0] = c;
        self.indice = 0;
        self.atual = c;
        self.carta = None;
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

    /// Clique em (x, y) relativo à área do cartão (`_ProcInfoEvent`): as áreas das figuras.
    /// Devolve se mudou de cartão.
    pub fn clicar(&mut self, x: i32, y: i32) -> bool {
        let Some(c) = &self.carta else { return false };
        let area = c.registros.iter().find_map(|r| match r {
            Registro::Figura { areas, .. } => areas.iter().find(|a| {
                x >= i32::from(a.ret.esq) && x < i32::from(a.ret.dir) && y >= i32::from(a.ret.topo) && y < i32::from(a.ret.base)
            }),
            _ => None,
        });
        match area.map(|a| a.valor) {
            Some(1) => {
                let antes = self.atual;
                self.voltar();
                self.atual != antes
            }
            Some(v) if v > 1 => {
                self.ir_para(v as u16);
                true
            }
            // Valor ≤ 0: popup da definição (não implementado).
            _ => false,
        }
    }

    /// Desenha o cartão em `area` (canto da área da janela).
    pub fn desenhar(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface, area: Retangulo) {
        // A linha de cima da área é a linha do título; o fundo começa uma abaixo.
        q.preencher(Retangulo { topo: area.topo + 1, ..area }, self.cfg.cor_fundo, self.cfg.cor_fundo);
        let Some(c) = &self.carta else { return };
        let antes = q.recorte.replace(area);
        let (ox, oy) = (i32::from(area.esq), i32::from(area.topo));
        let fonte = |i| interface.fontes.get(i).and_then(|f: &Option<Fonte>| f.as_ref());
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
                    let (Some(normal), destaque) = (fonte(self.cfg.fonte_texto), fonte(self.cfg.fonte_destaque)) else { continue };
                    let destaque = destaque.unwrap_or(normal);
                    let [tx, ty] = self.cfg.deslocamento_texto;
                    let mut y = oy + i32::from(ret.topo) + ty;
                    for &id in textos {
                        let Ok(bruto) = recursos.objeto(Chave { tipo: self.cfg.tipo_texto, id }) else { continue };
                        let texto: Vec<u8> = bruto.split(|&b| b == 0).next().unwrap_or(&[]).to_vec();
                        let estilos = recursos.objeto(Chave { tipo: self.cfg.tipo_estilo, id }).unwrap_or_default();
                        let destaques = ler_destaques(&estilos, texto.len());
                        y = self.escrever_texto(q, normal, destaque, &texto, &destaques, ox + i32::from(ret.esq) + tx, ox + i32::from(ret.dir) - self.cfg.margem_direita, y);
                    }
                }
            }
        }
        q.recorte = antes;
    }

    /// Texto com quebra por palavra entre `x0` e `x1` a partir de `y`; devolve o y seguinte.
    #[allow(clippy::too_many_arguments)]
    fn escrever_texto(&self, q: &mut Quadro, normal: &Fonte, destaque: &Fonte, texto: &[u8], destaques: &[(usize, usize)], x0: i32, x1: i32, mut y: i32) -> i32 {
        let alto = normal.altura().max(destaque.altura()) as i32;
        let em_destaque = |i: usize| destaques.iter().any(|&(a, b)| i >= a && i < b);
        // Palavras com o espaço que as segue, e as quebras de linha forçadas.
        let mut palavras: Vec<(usize, usize)> = Vec::new();
        let mut i = 0;
        while i < texto.len() {
            let ini = i;
            if texto[i] == b'\r' || texto[i] == b'\n' {
                palavras.push((ini, i + 1));
                i += 1;
                continue;
            }
            while i < texto.len() && texto[i] != b' ' && texto[i] != b'\r' && texto[i] != b'\n' {
                i += 1;
            }
            while i < texto.len() && texto[i] == b' ' {
                i += 1;
            }
            palavras.push((ini, i));
        }
        let largura = |a: usize, b: usize| -> i32 {
            (a..b).map(|k| if em_destaque(k) { destaque } else { normal }.largura_texto(&texto[k..k + 1]) as i32).sum()
        };
        let mut x = x0;
        for (a, b) in palavras {
            if texto[a] == b'\r' || texto[a] == b'\n' {
                x = x0;
                y += alto;
                continue;
            }
            // A palavra sem os espaços de depois tem de caber até `x1`.
            let sem_espaco = (a..b).rev().find(|&k| texto[k] != b' ').map_or(a, |k| k + 1);
            if x > x0 && x + largura(a, sem_espaco) > x1 {
                x = x0;
                y += alto;
            }
            for k in a..b {
                let f = if em_destaque(k) { destaque } else { normal };
                let img = f.imagem(&texto[k..k + 1]);
                for j in 0..img.altura {
                    for i in 0..img.largura {
                        if img.bits[j * img.largura + i] {
                            q.ponto(x + i as i32, y + j as i32, self.cfg.cor_texto);
                        }
                    }
                }
                x += img.largura as i32;
            }
        }
        y + alto
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
    }
}
