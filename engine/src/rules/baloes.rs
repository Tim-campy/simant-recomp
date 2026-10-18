//! Balões de fala da janela de edição (opção "Silly", `_OptionStates[5]`).
//!
//! Especificação: docs/spec/telas/baloes.md. Três partes, na ordem do original:
//!
//! 1. **No tick** ([`anotar`]): `_FightBalloons`, `_RestBalloons`, `_QueenBalloons`, `_EggBalloons`
//!    anotam a célula de uma luta, de um descanso, da rainha ou de um ovo à vista. Sem sorteio.
//! 2. **No redesenho da janela de edição** ([`sorteios_dos_baloes`] e [`balao_da_aranha`]):
//!    `_DrawCurBalloons` e a parte final de `_DrawSpider` decidem, com prazos em tempo real
//!    (`_TickCount`), se cada fala aparece. **Consomem o gerador S** (`SRand32`/`SRand64` e
//!    `SRand2`/`SRand4`), por isso ficam em `rules` e são chamados de `desenho::redesenho_do_timer`.
//! 3. **No fim do redesenho** ([`escolher_balao`]): `_DrawBalloons` desenha só o primeiro balão
//!    pedido que está na vista; a interface desenha [`Baloes::desenhado`] (`core::baloes`).
//!
//! As listas de falas, as esperas e as chances vêm de `data/baloes.json` ([`TabelasBaloes`]).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use super::estado::Estado;
use crate::recursos::Recursos;

/// Nome do arquivo de dados (em `data/` ou num mod).
pub const ARQUIVO_TABELAS: &str = "baloes.json";

/// Lado do tile da janela de edição em pixels (`_tileWidth` = `_tileHeight` = 16 no modo colorido).
const TILE: i32 = 16;

/// O que o tick anota (índice em `Baloes::contagem` e afins).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoBalao {
    Luta = 0,
    Descanso = 1,
    Rainha = 2,
    Ovo = 3,
}

/// As seis falas do desenho (índice em `Baloes::prazo`, `ligado` e `indice`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fala {
    LutaPreta = 0,
    LutaVermelha = 1,
    Ovo = 2,
    Rainha = 3,
    Descanso = 4,
    Aranha = 5,
}

impl Fala {
    pub const TODAS: [Fala; 6] = [Fala::LutaPreta, Fala::LutaVermelha, Fala::Ovo, Fala::Rainha, Fala::Descanso, Fala::Aranha];
}

/// Um balão pedido no redesenho (`_AddMsgBalloon`): ponto em pixels do mapa, plano e a fala.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Balao {
    pub x: i16,
    pub y: i16,
    pub plano: i16,
    pub fala: Fala,
    /// Índice na lista da fala (a interface o reduz ao tamanho da lista, como a volta a 0 do
    /// original quando o ponteiro seguinte é nulo).
    pub indice: i16,
}

/// Prazo e sorteio de uma fala.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub struct ConfigFala {
    /// Lista SHARED tipo 4 com as falas.
    pub lista: u16,
    /// Constante somada ao prazo (em `_TickCount`).
    pub espera: u32,
    /// `SRandK` somado ao prazo.
    pub sorteio_espera: u16,
    /// `SRandK` que liga o balão quando dá 0.
    pub chance: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub struct ConfigAranha {
    pub espera_troca: u32,
    pub sorteio_troca: u16,
    pub falas_por_modo: i16,
    pub modo_maximo: i16,
    pub plano: i16,
    pub passo: i16,
}

/// `data/baloes.json`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct TabelasBaloes {
    pub falas: HashMap<Fala, ConfigFala>,
    pub aranha: ConfigAranha,
    pub maximo: usize,
    pub linhas_acima_minimas: i16,
    pub meio_da_celula: i16,
    /// Canto do balão: (x + dx, y − (altura + dy)).
    pub deslocamento: [i16; 2],
    pub fonte: usize,
    pub cor_tinta: u8,
    pub cor_fundo: u8,
}

impl TabelasBaloes {
    pub fn carregar(r: &Recursos) -> Result<TabelasBaloes, String> {
        let t: TabelasBaloes = r.dados(ARQUIVO_TABELAS)?;
        t.validar()?;
        Ok(t)
    }

    pub fn de_json(texto: &str) -> Result<TabelasBaloes, String> {
        let t: TabelasBaloes = serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_TABELAS}: {e}"))?;
        t.validar()?;
        Ok(t)
    }

    /// Cópia embutida de `data/baloes.json` (sem mods).
    pub fn padrao() -> &'static TabelasBaloes {
        static PADRAO: OnceLock<TabelasBaloes> = OnceLock::new();
        PADRAO.get_or_init(|| TabelasBaloes::de_json(include_str!("../../../data/baloes.json")).expect("data/baloes.json inválido"))
    }

    fn validar(&self) -> Result<(), String> {
        let k_ok = |k: u16| k.is_power_of_two() && (2..=256).contains(&k);
        for f in Fala::TODAS {
            let c = self.falas.get(&f).ok_or(format!("{ARQUIVO_TABELAS}: falta a fala {f:?}"))?;
            if !k_ok(c.sorteio_espera) || !k_ok(c.chance) {
                return Err(format!("{ARQUIVO_TABELAS}: {f:?}: sorteio_espera e chance devem ser potências de 2 entre 2 e 256"));
            }
        }
        if !k_ok(self.aranha.sorteio_troca) || self.aranha.falas_por_modo < 1 {
            return Err(format!("{ARQUIVO_TABELAS}: aranha inválida"));
        }
        Ok(())
    }

    pub fn fala(&self, f: Fala) -> &ConfigFala {
        &self.falas[&f]
    }
}

/// `_BalloonIsVisible` (`1010:48a8`): a célula (x, y) do plano está na vista da janela de edição, com
/// ao menos `linhas` linhas acima dela.
fn na_vista(e: &Estado, linhas: i16, plano: i16, x: i32, y: i32) -> bool {
    let (px, py) = (i32::from(e.vistas.ponto.0), i32::from(e.vistas.ponto.1));
    let (l, a) = (i32::from(e.baloes.vista.0), i32::from(e.baloes.vista.1));
    plano == e.vistas.plano && px <= x && x < px + l && py <= y - i32::from(linhas) && y < py + a
}

/// `_FightBalloons`, `_RestBalloons`, `_QueenBalloons`, `_EggBalloons` (`1010:48f0`…`4aee`), chamados
/// no tick (só com "Silly"; quem chama confere a opção como o original): se ainda não há balão
/// desse tipo neste tick e a célula está na vista, ou confirma o balão atual (mesma célula e plano)
/// ou anota a célula como a última vista.
pub fn anotar(e: &mut Estado, tipo: TipoBalao, x: i32, y: i32, plano: i32) {
    let plano = plano as i16;
    let i = tipo as usize;
    if e.baloes.contagem[i] != 0 || !na_vista(e, 3, plano, x, y) {
        return;
    }
    let b = &mut e.baloes;
    if b.atual[i] == (x as i16, y as i16) && b.atual_plano[i] == plano {
        b.contagem[i] = 1;
        return;
    }
    b.ultimo[i] = (x as i16, y as i16);
    b.ultimo_plano[i] = plano;
}

/// Prazo vencido? (`_TickCount` > prazo, em 32 bits.)
fn venceu(prazo: u32, agora: u32) -> bool {
    prazo < agora
}

/// Sorteia o novo prazo e se a fala aparece; devolve os passos de S (sempre 2).
fn sortear_fala(e: &mut Estado, t: &TabelasBaloes, f: Fala, agora: u32, indice_maximo: Option<i16>) -> u32 {
    let c = t.fala(f);
    let r = e.s.mascara(c.sorteio_espera) as u32;
    let i = f as usize;
    e.baloes.prazo[i] = r.wrapping_add(agora).wrapping_add(c.espera);
    if e.s.mascara(c.chance) == 0 {
        e.baloes.ligado[i] = true;
        let mut k = e.baloes.indice[i].wrapping_add(1);
        if indice_maximo.is_some_and(|m| k > m) {
            k = 0;
        }
        e.baloes.indice[i] = k;
    } else {
        e.baloes.ligado[i] = false;
    }
    2
}

/// `_AddMsgBalloon` (ou o mesmo código embutido): até `maximo` pedidos, só os que estão na vista.
/// `x`, `y` em pixels do mapa.
fn pedir(e: &mut Estado, t: &TabelasBaloes, b: Balao) {
    let n = e.baloes.quantidade.max(0) as usize;
    if n >= t.maximo.min(e.baloes.pedidos.len()) {
        return;
    }
    let (cx, cy) = (i32::from(b.x) / TILE, i32::from(b.y) / TILE);
    if !na_vista(e, t.linhas_acima_minimas, b.plano, cx, cy) {
        return;
    }
    e.baloes.pedidos[n] = Some(b);
    e.baloes.quantidade += 1;
}

/// `_DrawCurBalloons` (`1010:4cac`), no começo de cada redesenho da janela de edição, com "Silly" e
/// sem camada de cheiro (`_ELayerMode` = −1). Para luta, ovo, rainha e descanso, nessa ordem: um
/// balão novo (contagem 0 e um último ponto anotado) passa a ser o atual, com os prazos zerados (e,
/// menos na luta, a fala desligada);
/// havendo balão atual e o jogo não pausado, cada fala com o prazo vencido sorteia o prazo novo e se
/// aparece; as falas ligadas pedem o balão no ponto atual. `agora` = `_TickCount`. Devolve os passos
/// de S.
pub fn sorteios_dos_baloes(e: &mut Estado, t: &TabelasBaloes, agora: u32) -> u32 {
    if e.partida.opcoes[5] == 0 || e.camada_edicao != -1 {
        return 0;
    }
    let grupos: [(TipoBalao, &[Fala]); 4] = [
        (TipoBalao::Luta, &[Fala::LutaPreta, Fala::LutaVermelha]),
        (TipoBalao::Ovo, &[Fala::Ovo]),
        (TipoBalao::Rainha, &[Fala::Rainha]),
        (TipoBalao::Descanso, &[Fala::Descanso]),
    ];
    let mut passos = 0;
    for (tipo, falas) in grupos {
        let i = tipo as usize;
        let b = &mut e.baloes;
        if b.contagem[i] == 0 && b.ultimo[i].0 >= 0 && b.ultimo[i].1 >= 0 {
            b.atual[i] = b.ultimo[i];
            b.atual_plano[i] = b.ultimo_plano[i];
            b.contagem[i] += 1;
            for f in falas {
                b.prazo[*f as usize] = 0;
                // Ovo, rainha e descanso também desligam a fala; a luta não (1010:4cac).
                if tipo != TipoBalao::Luta {
                    b.ligado[*f as usize] = false;
                }
            }
        }
        if e.baloes.contagem[i] <= 0 {
            continue;
        }
        if e.partida.pausado == 0 {
            for &f in falas {
                if venceu(e.baloes.prazo[f as usize], agora) {
                    passos += sortear_fala(e, t, f, agora, None);
                }
            }
        }
        for &f in falas {
            if e.baloes.ligado[f as usize] {
                let (x, y) = e.baloes.atual[i];
                let meio = t.meio_da_celula;
                let b = Balao {
                    x: x.wrapping_mul(TILE as i16).wrapping_add(meio),
                    y: y.wrapping_mul(TILE as i16).wrapping_add(meio),
                    plano: e.baloes.atual_plano[i],
                    fala: f,
                    indice: e.baloes.indice[f as usize],
                };
                pedir(e, t, b);
            }
        }
    }
    passos
}

/// Fim de `_DrawSpider` (`1010:1d6a`), com a aranha visível e "Silly": sem pausa, prazo vencido →
/// `SRandK` + `agora` + espera e `SRand2` (0 liga, e a fala anda; volta a 0 depois de
/// `falas_por_modo` − 1). Se `_SMode` mudou desde o último ou passa de `modo_maximo`, prazo novo
/// (`SRand32` + 30, mesmo pausado), balão desligado, e acaba. Ligado, pede o balão na frente da
/// aranha (`(_SpidX, _SpidY) + passo × (dx8, dy8)[_SpidDir]`, plano 1). `d8` = `_Dx8`/`_Dy8`.
/// Devolve os passos de S.
pub fn balao_da_aranha(e: &mut Estado, t: &TabelasBaloes, d8: (&[i8; 8], &[i8; 8]), agora: u32) -> u32 {
    if e.partida.opcoes[5] == 0 {
        return 0;
    }
    let a = t.aranha;
    let i = Fala::Aranha as usize;
    let mut passos = 0;
    if e.partida.pausado == 0 && venceu(e.baloes.prazo[i], agora) {
        passos += sortear_fala(e, t, Fala::Aranha, agora, Some(a.falas_por_modo - 1));
    }
    let modo = e.aranha.estado;
    if modo != e.baloes.modo_aranha_visto || modo > a.modo_maximo {
        let r = e.s.mascara(a.sorteio_troca) as u32;
        e.baloes.prazo[i] = r.wrapping_add(agora).wrapping_add(a.espera_troca);
        e.baloes.ligado[i] = false;
        e.baloes.modo_aranha_visto = modo;
        return passos + 1;
    }
    if !e.baloes.ligado[i] {
        return passos;
    }
    let dir = (e.aranha.dir & 7) as usize;
    let x = e.aranha.x.wrapping_add(i16::from(d8.0[dir]) * a.passo);
    let y = e.aranha.y.wrapping_add(i16::from(d8.1[dir]) * a.passo);
    let indice = modo.wrapping_mul(a.falas_por_modo).wrapping_add(e.baloes.indice[i]);
    pedir(e, t, Balao { x, y, plano: a.plano, fala: Fala::Aranha, indice });
    passos
}

/// `_DrawBalloons` (`1010:57aa`), no fim do redesenho: desenha só o primeiro balão pedido que está
/// na vista (o laço para no primeiro desenhado) e esvazia a lista. O desenhado fica até o próximo
/// redesenho.
pub fn escolher_balao(e: &mut Estado, t: &TabelasBaloes) {
    let n = e.baloes.quantidade.max(0) as usize;
    let pedidos = e.baloes.pedidos;
    e.baloes.desenhado = pedidos[..n.min(pedidos.len())].iter().flatten().copied().find(|b| {
        na_vista(e, t.linhas_acima_minimas, b.plano, i32::from(b.x) / TILE, i32::from(b.y) / TILE)
    });
    e.baloes.pedidos = [None; 6];
    e.baloes.quantidade = 0;
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::rules::rng::GeradorS;

    fn estado() -> Estado {
        let mut e = Estado::vazio();
        e.partida.opcoes[5] = 1;
        e.camada_edicao = -1;
        e.vistas.plano = 2;
        e.vistas.ponto = (10, 10);
        e.baloes.vista = (25, 19);
        e.baloes.zerar();
        e.s = GeradorS::new(0x1234);
        e
    }

    #[test]
    fn anotar_confirma_o_atual_ou_guarda_o_ultimo() {
        let mut e = estado();
        anotar(&mut e, TipoBalao::Rainha, 20, 20, 2);
        assert_eq!(e.baloes.ultimo[2], (20, 20));
        assert_eq!(e.baloes.contagem[2], 0);
        // Fora da vista (menos de 3 linhas acima) ou em outro plano: nada.
        anotar(&mut e, TipoBalao::Ovo, 20, 12, 2);
        anotar(&mut e, TipoBalao::Ovo, 20, 20, 3);
        assert_eq!(e.baloes.ultimo[3], (-1, -1));
        // Mesma célula do balão atual: confirma (contagem 1) e ignora os seguintes.
        e.baloes.atual[3] = (15, 15);
        e.baloes.atual_plano[3] = 2;
        anotar(&mut e, TipoBalao::Ovo, 15, 15, 2);
        anotar(&mut e, TipoBalao::Ovo, 16, 16, 2);
        assert_eq!((e.baloes.contagem[3], e.baloes.ultimo[3]), (1, (-1, -1)));
    }

    #[test]
    fn balao_novo_sorteia_na_hora_e_espera_o_prazo() {
        let t = TabelasBaloes::padrao();
        let mut e = estado();
        anotar(&mut e, TipoBalao::Luta, 20, 20, 2);
        let s0 = e.s;
        // Novo: prazos zerados, as duas falas da luta sorteiam (S32 + S2 cada).
        assert_eq!(sorteios_dos_baloes(&mut e, t, 1000), 4);
        let mut s = s0;
        let r = s.mascara(32) as u32;
        assert_eq!(e.baloes.prazo[0], 1000 + r + 60);
        assert_eq!(e.baloes.atual[0], (20, 20));
        // Antes do prazo, nada sorteia; o pedido do balão ligado se repete.
        e.baloes.ligado[0] = true;
        e.baloes.quantidade = 0;
        assert_eq!(sorteios_dos_baloes(&mut e, t, 1001), 0);
        assert_eq!(e.baloes.quantidade, 1);
        assert_eq!(e.baloes.pedidos[0].map(|b| (b.x, b.y)), Some((20 * 16 + 8, 20 * 16 + 8)));
        escolher_balao(&mut e, t);
        assert_eq!(e.baloes.desenhado.map(|b| b.fala), Some(Fala::LutaPreta));
        assert_eq!(e.baloes.quantidade, 0);
        // Pausado: não sorteia nem com o prazo vencido.
        e.partida.pausado = 1;
        assert_eq!(sorteios_dos_baloes(&mut e, t, 5000), 0);
        // Sem Silly ou com camada de cheiro: nada.
        e.partida.pausado = 0;
        e.camada_edicao = 0;
        assert_eq!(sorteios_dos_baloes(&mut e, t, 5000), 0);
    }

    #[test]
    fn aranha_troca_de_modo_sorteia_mesmo_pausada() {
        let t = TabelasBaloes::padrao();
        let d8 = ([0, 1, 1, 1, 0, -1, -1, -1], [-1, -1, 0, 1, 1, 1, 0, -1]);
        let mut e = estado();
        e.vistas.plano = 1;
        e.partida.pausado = 1;
        e.aranha.estado = 2;
        assert_eq!(balao_da_aranha(&mut e, t, (&d8.0, &d8.1), 100), 1);
        assert_eq!(e.baloes.modo_aranha_visto, 2);
        assert!(!e.baloes.ligado[5]);
        // Mesmo modo, sem pausa e prazo vencido: S64 + S2.
        e.partida.pausado = 0;
        assert_eq!(balao_da_aranha(&mut e, t, (&d8.0, &d8.1), 10_000), 2);
        // Modo 5 (morrendo): sorteia a troca a cada redesenho.
        e.aranha.estado = 5;
        e.baloes.modo_aranha_visto = 5;
        assert_eq!(balao_da_aranha(&mut e, t, (&d8.0, &d8.1), 10_001), 1);
    }
}
