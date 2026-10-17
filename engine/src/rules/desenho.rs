//! Sorteios do DESENHO: o que as rotinas de desenho do original consomem do gerador S (e gravam no
//! estado) a cada redesenho, sem desenhar nada. A interface do port chama estas funções quando o
//! original redesenharia (`_UpdateWindows` depois de cada tick, ver [`redesenho_do_timer`]), para que o
//! gerador S siga o do original. Especificação: docs/spec/regras/rng.md, "Sorteios do desenho".
//!
//! Fora daqui (dependem de estado da interface que não está no `Estado`): os balões da opção "Silly"
//! (`_DrawCurBalloons`, prazos em tempo real) e o balão da aranha (`_DrawSpider`), o ícone animado com
//! a janela minimizada (`_SRand1(7)` no `MYTIMERFUNC`), as caixas de fim de jogo e da aranha.

use super::estado::Estado;
use super::geracao::Cor;

/// Gotas da chuva redesenhadas por quadro (`_Draw_SimYard`, `1010:b462`): 14 posições sorteadas.
pub const GOTAS_POR_QUADRO: usize = 14;
/// Formigas de cada enxame desenhadas no máximo (`_DrawSwarm`).
pub const ENXAME_MAXIMO: i16 = 16;

/// `_Draw_SimYard(modo)` (`1010:b462`), só a parte que sorteia: nos modos 0 (Yard) e 1 (House), se
/// chove, 14 × (`SRand1(400)`, `SRand1(150)`) para as gotas; depois `_DrawSwarm`. Nos modos 2 e 3
/// nada. Devolve os passos de S consumidos.
pub fn desenhar_quintal(e: &mut Estado, modo_quintal: i16) -> u32 {
    if modo_quintal > 1 {
        return 0;
    }
    let mut passos = 0;
    if e.ambiente.chovendo != 0 {
        for _ in 0..GOTAS_POR_QUADRO {
            e.s.rand1(400);
            e.s.rand1(150);
            passos += 2;
        }
    }
    passos + desenhar_enxame(e)
}

/// `_DrawSwarm` (`1010:bb0e`): com `_MapMode` = 0, grava o centro do enxame na tela do quintal
/// (`_SwarmCentX = 28·px − 10·py + 178`, `_SwarmCentY = 10·py + 46`, do patch atual) e mexe cada formiga
/// dos enxames (até `min(_SwarmCnt, 16, _QueenStorage)`): a preta fora de ±15 volta para
/// (`S16 − 10`, `S16 − 10`); a vermelha fora de ±15 volta para (0, 0) sem sorteio; depois cada uma anda
/// `S1(3) − 1` em x e em y. Devolve os passos de S.
pub fn desenhar_enxame(e: &mut Estado) -> u32 {
    if e.vistas.modo_mapa != 0 {
        return 0;
    }
    let (px, py) = e.ambiente.patch;
    e.ambiente.enxame_centro_y = py * 10 + 46;
    e.ambiente.enxame_centro_x = px * 28 - py * 10 + 178;
    let mut passos = 0;
    for cor in [Cor::Preta, Cor::Vermelha] {
        let c = e.colonia(cor);
        let n = c.enxame_contagem.min(ENXAME_MAXIMO).min(c.rainhas_guardadas.max(0)).max(0) as usize;
        for i in 0..n {
            let c = e.colonia(cor);
            let (mut x, mut y) = (c.enxame_x.0[i] as i8, c.enxame_y.0[i] as i8);
            if !(-16 < x && x < 16 && -16 < y && y < 16) {
                if cor == Cor::Preta {
                    x = (e.s.mascara(16) - 10) as i8;
                    y = (e.s.mascara(16) - 10) as i8;
                    passos += 2;
                } else {
                    (x, y) = (0, 0);
                }
            }
            x = x.wrapping_add((e.s.rand1(3) - 1) as i8);
            y = y.wrapping_add((e.s.rand1(3) - 1) as i8);
            passos += 2;
            let c = e.colonia_mut(cor);
            c.enxame_x.0[i] = x as u8;
            c.enxame_y.0[i] = y as u8;
        }
    }
    passos
}

/// Desenho do mapa de colônias (Strategic View, `_DrawSimColonies` `1010:bea4`): consome o pedido de
/// redesenho `_ColonyUpdateFlag` (1 → 0). Sem sorteio. [hipótese quanto à rotina que zera; observado
/// na bancada no vetor `acoes/special_xfer`]
pub fn desenhar_colonias(e: &mut Estado) {
    e.ambiente.atualizar_colonias = 0;
}

/// `_DrawPalps` (`1010:2594`) dentro de `_DrawSpider`, a cada redesenho da janela de edição com a
/// aranha visível (`_PreDrawSpider`: `_SpidOn`, plano 0 ou 1 e a aranha dentro da vista — quem sabe é
/// a interface) e `_SMode` ≠ 5: um `SRand1(4)`, exceto com `_SMode` < 2 e `_MeSMode` < 6.
pub fn desenhar_palpos(e: &mut Estado) -> u32 {
    if e.aranha.estado == 5 || (e.aranha.estado < 2 && e.amarela.modo_aranha < 6) {
        return 0;
    }
    e.s.rand1(4);
    1
}

/// O que a interface diz sobre as janelas no momento do redesenho.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JanelasDesenho {
    /// Aplicação ativa (`_activeAppFlag`).
    pub ativa: bool,
    /// Janela minimizada (nada é redesenhado).
    pub minimizada: bool,
    /// Janela do quintal (0x1900) aberta, e se está na frente.
    pub quintal_aberto: bool,
    pub quintal_na_frente: bool,
    /// Janela de edição aberta, e se está na frente.
    pub edicao_aberta: bool,
    pub edicao_na_frente: bool,
    /// A aranha aparece na janela de edição (`_PreDrawSpider`).
    pub aranha_visivel: bool,
}

/// Contadores do `MYTIMERFUNC`/`_UpdateWindows` que não vão no save (a interface os guarda).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContadoresRedesenho {
    /// `DGROUP:0x2f0`: ticks (com a aplicação inativa, redesenha 1 em 8).
    pub ticks: u32,
    /// `_UDcntr` (`9:9b00`): chamadas de `_UpdateWindows`, módulo 64.
    pub chamadas: u8,
    /// Alternância do quintal atrás (`DAT_02ea`) e da edição atrás (`DAT_02ec`).
    pub alterna_quintal: bool,
    pub alterna_edicao: bool,
}

/// `_UpdateWindows` (`1000:2224`) depois de um tick, só os sorteios: decide se redesenha (aplicação
/// ativa, ou 1 tick em 8 inativa; nunca minimizada; velocidade 2 pula metade e 3 três quartos) e, em
/// ordem, o quintal (sempre na frente; atrás, um redesenho sim e outro não) e a janela de edição (idem).
/// Devolve os passos de S. [hipótese: `_WaitedEnough` do quintal com `_YardMode` > 1, que só não sorteia,
/// não é modelado; os balões e o balão da aranha ficam de fora (módulo).]
pub fn redesenho_do_timer(e: &mut Estado, j: &JanelasDesenho, k: &mut ContadoresRedesenho) -> u32 {
    k.ticks = k.ticks.wrapping_add(1);
    if j.minimizada || (!j.ativa && k.ticks & 7 != 0) {
        return 0;
    }
    let chamada = k.chamadas;
    k.chamadas = (k.chamadas + 1) & 0x3F;
    let pula = match e.partida.velocidade {
        2 => chamada & 1 != 0,
        3 => chamada & 3 != 0,
        _ => false,
    };
    if pula {
        return 0;
    }
    let mut passos = 0;
    if j.quintal_aberto {
        let desenha = j.quintal_na_frente || {
            k.alterna_quintal = !k.alterna_quintal;
            !k.alterna_quintal
        };
        if desenha {
            passos += desenhar_quintal(e, e.vistas.modo_quintal);
        }
    }
    if j.edicao_aberta {
        let desenha = j.edicao_na_frente || {
            k.alterna_edicao = !k.alterna_edicao;
            !k.alterna_edicao
        };
        if desenha && j.aranha_visivel {
            passos += desenhar_palpos(e);
        }
    }
    passos
}

#[cfg(test)]
mod testes {
    use super::*;

    fn chuva() -> Estado {
        let mut e = Estado::vazio();
        e.ambiente.chovendo = 1;
        e.vistas.modo_mapa = 1;
        e.s = super::super::rng::GeradorS::new(0x1234);
        e
    }

    #[test]
    fn chuva_sorteia_28_e_so_nos_modos_0_e_1() {
        let mut e = chuva();
        assert_eq!(desenhar_quintal(&mut e, 0), 28);
        assert_eq!(desenhar_quintal(&mut e, 1), 28);
        assert_eq!(desenhar_quintal(&mut e, 2), 0);
        e.ambiente.chovendo = 0;
        assert_eq!(desenhar_quintal(&mut e, 0), 0);
    }

    #[test]
    fn redesenho_respeita_velocidade_janela_atras_e_inativa() {
        let j = JanelasDesenho { ativa: true, quintal_aberto: true, quintal_na_frente: true, ..Default::default() };
        let mut e = chuva();
        let mut k = ContadoresRedesenho::default();
        assert_eq!(redesenho_do_timer(&mut e, &j, &mut k), 28);
        k.chamadas = 0;
        e.partida.velocidade = 3;
        let n: u32 = (0..8).map(|_| redesenho_do_timer(&mut e, &j, &mut k)).sum();
        assert_eq!(n, 2 * 28, "ultra: 1 em 4");
        e.partida.velocidade = 1;
        let atras = JanelasDesenho { quintal_na_frente: false, ..j };
        let n: u32 = (0..4).map(|_| redesenho_do_timer(&mut e, &atras, &mut k)).sum();
        assert_eq!(n, 2 * 28, "atrás: um sim, outro não");
        let inativa = JanelasDesenho { ativa: false, ..j };
        k.ticks = 0;
        let n: u32 = (0..16).map(|_| redesenho_do_timer(&mut e, &inativa, &mut k)).sum();
        assert_eq!(n, 2 * 28, "inativa: 1 tick em 8");
        let min = JanelasDesenho { minimizada: true, ..j };
        assert_eq!(redesenho_do_timer(&mut e, &min, &mut k), 0);
    }
}
