//! Sorteios do DESENHO: o que as rotinas de desenho do original consomem do gerador S (e gravam no
//! estado) a cada redesenho, sem desenhar nada. A interface do port chama estas funções quando o
//! original redesenharia (`_UpdateWindows` depois de cada tick, ver [`redesenho_do_timer`]), para que o
//! gerador S siga o do original. Especificação: docs/spec/regras/rng.md, "Sorteios do desenho".
//!
//! Os balões da opção "Silly" (`_DrawCurBalloons` e o balão da aranha em `_DrawSpider`, prazos em
//! tempo real) estão em `rules::baloes` e são chamados daqui, na ordem do redesenho da janela de
//! edição. Fora daqui: o ícone animado com a janela minimizada (`_SRand1(7)` no `MYTIMERFUNC`, em
//! `core::partida`), as caixas de fim de jogo e da aranha.

use super::baloes::{self, TabelasBaloes};
use super::estado::{Estado, Evento, QuadroAnimacao};
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
    desenhar_menino(e, modo_quintal);
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

/// `_DrawSimKid` (`1010:af14`), chamado por `_Draw_SimYard` quando o menino aparece (modo 1, ou
/// `_BoyHere` ≠ 0, e `_BoyFrame` ≥ 0): sem pausa, se o relógio do Mac passou de `_BoyMsgCnt`, o balão
/// do menino acaba (`_BoyMessOn = 0`). Sem sorteio. O relógio é o do tick que acabou de rodar
/// (`Estado::relogio_mac`); sem ele, nada.
pub fn desenhar_menino(e: &mut Estado, modo_quintal: i16) {
    let m = &e.menino;
    if (modo_quintal == 0 && m.presente == 0) || m.quadro < 0 || e.partida.pausado != 0 {
        return;
    }
    if let Some(agora) = e.relogio_mac {
        if agora > e.menino.boy_msg_cnt as u32 {
            e.menino.mensagem_ligada = 0;
        }
    }
}

/// `_DrawMower` (`1010:b2c4`), chamado por `_Draw_SimYard` nos modos 0 e 1: com o menino cortando ou
/// guardando o cortador (`_BoyHere` 3 ou 4) e o jogo não pausado (`_GamePaused`), o efeito do
/// cortador a cada redesenho do quintal. Sem sorteio.
pub fn desenhar_cortador(e: &mut Estado, som: Option<u16>) {
    let m = &e.menino;
    if e.vistas.modo_quintal <= 1 && (m.presente == 3 || m.presente == 4) && e.partida.pausado == 0 {
        e.pedir_som(som);
    }
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
/// O índice sorteado fica em `Estado::palpos_aranha` (0 sem sorteio), para o desenho.
pub fn desenhar_palpos(e: &mut Estado) -> u32 {
    if e.aranha.estado == 5 {
        return 0;
    }
    if e.aranha.estado < 2 && e.amarela.modo_aranha < 6 {
        e.palpos_aranha = 0;
        return 0;
    }
    e.palpos_aranha = e.s.rand1(4) as u8;
    1
}

/// `_PreDrawSpider` (`1010:1c18`): a aranha aparece na janela de edição se `_SpidOn` ≠ 0, o plano do
/// mapa é 0 ou 1 e a posição dela (em 1/16 de célula) está dentro da vista de `largura` × `altura`
/// células com o canto em (`x`, `y`), bordas incluídas. [O original compara em pixels da tela; com
/// tiles de 12 pixels arredonda `SpidX·3/4`, o que pode mudar a borda em 1/16 de célula.]
pub fn aranha_visivel(e: &Estado, plano: i16, x: i32, y: i32, largura: i32, altura: i32) -> bool {
    let (sx, sy) = (i32::from(e.aranha.x), i32::from(e.aranha.y));
    e.aranha.ligada != 0
        && (plano == 0 || plano == 1)
        && (16 * x..=16 * (x + largura)).contains(&sx)
        && (16 * y..=16 * (y + altura)).contains(&sy)
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
    /// Efeito do cortador de grama (`_DrawMower`, 0x24; `quintal.json`, `sons.cortador`).
    pub som_cortador: Option<u16>,
    /// `_TickCount` (`GetTickCount() / 55`) do redesenho: prazos dos balões "Silly".
    pub tick_count: u32,
}

/// Tabelas que o redesenho da janela de edição usa: os balões e as direções da aranha
/// (`_Dx8`/`_Dy8`, para o balão dela).
#[derive(Clone, Copy, Debug)]
pub struct TabelasDesenho<'a> {
    pub baloes: &'a TabelasBaloes,
    pub d8: (&'a [i8; 8], &'a [i8; 8]),
}

impl TabelasDesenho<'static> {
    /// Cópias embutidas (`data/baloes.json` e `data/inimigos.json`), para testes e vetores.
    pub fn padrao() -> TabelasDesenho<'static> {
        let i = super::inimigos::TabelasInimigos::padrao();
        TabelasDesenho { baloes: TabelasBaloes::padrao(), d8: (&i.dx8, &i.dy8) }
    }
}

/// O redesenho da janela de edição (`FUN_1010_16d4`), só o que sorteia, na ordem do original:
/// `_DrawCurBalloons`, as células, `_DrawSpider` (palpos e balão da aranha) com a aranha visível, e
/// `_DrawBalloons` (escolhe o balão desenhado). Devolve os passos de S.
pub fn desenhar_edicao(e: &mut Estado, j: &JanelasDesenho, t: &TabelasDesenho) -> u32 {
    let mut passos = baloes::sorteios_dos_baloes(e, t.baloes, j.tick_count);
    if j.aranha_visivel {
        passos += desenhar_palpos(e);
        passos += baloes::balao_da_aranha(e, t.baloes, t.d8, j.tick_count);
    }
    baloes::escolher_balao(e, t.baloes);
    passos
}

// ---------------------------------------------------------------------------------------------
// Desenho DENTRO do tick (rng.md, "Desenho dentro do tick")
// ---------------------------------------------------------------------------------------------

/// A janela de edição para os redesenhos que o original faz no meio do tick: `_GotoMyAnt` (quando
/// `_CenterEdit` move a vista), `_DoEditUpdateDraw`/`_DoEditAndMapUpdateDraw` (mortes, nascimento,
/// animações, trofalaxia, entrada e saída do ninho). Cada um passa por `_UpdateEdit`, que só exige a
/// janela **aberta** (não precisa estar na frente), e sorteia como o redesenho do timer
/// ([`desenhar_edicao`]). Quem roda o tick põe isto em `Estado::edicao_no_tick` (`None` = janela
/// fechada: nada é desenhado).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdicaoNoTick {
    /// `_editWidth`, `_editHeight` (células da vista).
    pub largura: i32,
    pub altura: i32,
    /// `_TickCount` usado pelos prazos dos balões "Silly" dentro do tick: o do começo do tick, mais
    /// as esperas das animações da amarela ([`avancar_relogio_no_tick`]). O original lê o relógio a
    /// cada desenho; fora das animações o tick não espera, e as leituras dão o mesmo valor (bancada,
    /// rng.md "Desenho dentro do tick").
    pub tick_count: u32,
    /// `data/baloes.json` e `_Dx8`/`_Dy8` (balão da aranha).
    pub baloes: std::sync::Arc<TabelasBaloes>,
    pub d8: ([i8; 8], [i8; 8]),
    /// `_MapPlane` e `_MapPnt` como o original os deixa no meio do tick: copiados de `Estado::vistas`
    /// no começo ([`comecar_tick`]) e movidos por [`centralizar_no_tick`]. A vista de verdade continua
    /// sendo movida depois do tick pelos pedidos `Evento::CentralizarNaAmarela`, com o mesmo cálculo.
    pub plano: i16,
    pub ponto: (i16, i16),
    /// Dentro de um tick (entre [`comecar_tick`] e [`terminar_tick`]); fora dele não há desenho daqui.
    pub no_tick: bool,
    /// Redesenhos e passos de S feitos dentro do último tick (conferência).
    pub desenhos: u32,
    pub passos: u32,
}

impl EdicaoNoTick {
    pub fn new(largura: i32, altura: i32, baloes: std::sync::Arc<TabelasBaloes>, d8: ([i8; 8], [i8; 8])) -> EdicaoNoTick {
        EdicaoNoTick { largura, altura, tick_count: 0, baloes, d8, plano: 0, ponto: (0, 0), no_tick: false, desenhos: 0, passos: 0 }
    }

    /// Com as tabelas embutidas (testes e vetores).
    pub fn padrao(largura: i32, altura: i32) -> EdicaoNoTick {
        let i = super::inimigos::TabelasInimigos::padrao();
        EdicaoNoTick::new(largura, altura, std::sync::Arc::new(TabelasBaloes::padrao().clone()), (i.dx8, i.dy8))
    }
}

/// Começo do tick: a vista do tick parte de `_MapPlane`/`_MapPnt`.
pub fn comecar_tick(e: &mut Estado) {
    let (plano, ponto) = (e.vistas.plano, e.vistas.ponto);
    if let Some(j) = e.edicao_no_tick.as_deref_mut() {
        j.plano = plano;
        j.ponto = ponto;
        j.no_tick = true;
        j.desenhos = 0;
        j.passos = 0;
    }
}

/// As animações da amarela (`_AnimYellowFight`, `_AnimYellowInsane`) esperam o relógio antes de cada
/// quadro: o desenho do quadro lê `_TickCount` `degraus` adiante. Sem efeito fora do tick.
pub fn avancar_relogio_no_tick(e: &mut Estado, degraus: u32) {
    if let Some(j) = e.edicao_no_tick.as_deref_mut().filter(|j| j.no_tick) {
        j.tick_count = j.tick_count.wrapping_add(degraus);
    }
}

/// Fim do tick: os desenhos daqui param (o resto é do redesenho do timer, [`redesenho_do_timer`]).
pub fn terminar_tick(e: &mut Estado) {
    if let Some(j) = e.edicao_no_tick.as_deref_mut() {
        j.no_tick = false;
    }
}

/// `_UpdateEdit` (`1010:11e0`) dentro do tick: o redesenho da janela de edição (`FUN_1010_16d4`) com a
/// vista do tick, se a janela está aberta. Devolve os passos de S.
pub fn desenhar_no_tick(e: &mut Estado) -> u32 {
    let Some(j) = e.edicao_no_tick.as_deref().filter(|j| j.no_tick) else { return 0 };
    let (baloes, d8, tick_count) = (j.baloes.clone(), j.d8, j.tick_count);
    let (plano, (x, y), w, h) = (j.plano, j.ponto, j.largura, j.altura);
    let visivel = aranha_visivel(e, plano, i32::from(x), i32::from(y), w, h);
    let janelas = JanelasDesenho { aranha_visivel: visivel, tick_count, ..Default::default() };
    let tabelas = TabelasDesenho { baloes: &baloes, d8: (&d8.0, &d8.1) };
    let passos = desenhar_edicao(e, &janelas, &tabelas);
    if let Some(j) = e.edicao_no_tick.as_deref_mut() {
        j.desenhos += 1;
        j.passos += passos;
    }
    // O instantâneo do que foi desenhado, para a linha do tempo da apresentação (só lê o estado).
    if let Some(quadro) = foto_da_edicao(e, plano) {
        let plano = plano.max(1);
        e.eventos.push(Evento::Instantaneo { plano, quadro: Box::new(quadro) });
    }
    passos
}

/// A janela de edição do plano `plano` (1 superfície, 2 ninho preto, 3 ninho vermelho; 0 conta como
/// a superfície) como `_DoEditUpdateDraw` a desenharia agora: mapa de chão, mapa de vida e os campos
/// `_Me*` que o desenho da marca 0xFF lê. Só lê o estado.
pub fn foto_da_edicao(e: &Estado, plano: i16) -> Option<QuadroAnimacao> {
    let (mapa, vida) = match plano {
        0 | 1 => (&e.superficie.mapa, &e.superficie.vida),
        2 => (&e.ninho_b.mapa, &e.ninho_b.vida),
        3 => (&e.ninho_r.mapa, &e.ninho_r.vida),
        _ => return None,
    };
    let a = &e.amarela;
    Some(QuadroAnimacao {
        mapa: mapa.dados.clone(),
        vida: vida.dados.clone(),
        x: a.x,
        y: a.y,
        tipo: a.tipo,
        dir: a.dir,
        ovo: a.ovo,
        base: a.base,
    })
}

/// A parte de vista de `_GotoMyAnt` (`1000:a764`) dentro do tick, com a amarela em (`plano`, x, y):
/// fora do Experimental, `_CenterEdit` no plano mostrado ou, noutro plano, `_SetMapPlaneLocation`
/// (troca o plano e centraliza); se a vista andou, `_UpdateEdit` redesenha. Devolve os passos de S.
pub fn centralizar_no_tick(e: &mut Estado, plano: i16, x: i16, y: i16) -> u32 {
    if e.partida.tipo_jogo == 3 || plano <= 0 {
        return 0;
    }
    let Some(j) = e.edicao_no_tick.as_deref_mut().filter(|j| j.no_tick) else { return 0 };
    j.plano = plano;
    let antes = j.ponto;
    let (px, py) = centralizar_vista(antes, i32::from(x), i32::from(y), plano, (j.largura, j.altura));
    j.ponto = (px as i16, py as i16);
    if j.ponto == antes {
        return 0;
    }
    desenhar_no_tick(e)
}

/// `_CenterEdit` (`1010:1620`, telas/jogo.md "Centralizar"): anda `_MapPnt` até (x − w/2, y − h/2)
/// como uma reta de Bresenham em ponto fixo (`FUN_1010_616c`: o eixo menor pode ficar um passo aquém)
/// e aplica os limites do plano (128 × 64 na superfície, 64 × 64 nos ninhos).
pub fn centralizar_vista(ponto: (i16, i16), x: i32, y: i32, plano: i16, vista: (i32, i32)) -> (i32, i32) {
    let (w, h) = vista;
    let (mut px, mut py) = (ponto.0 as i32, ponto.1 as i32);
    let (dx, dy) = (x - px - w / 2, y - py - h / 2);
    let (ax, ay) = (dx.unsigned_abs(), dy.unsigned_abs());
    let (sx, sy) = (dx.signum(), dy.signum());
    let (maior, menor) = (ax.max(ay), ax.min(ay));
    if let Some(fracao) = (menor << 16).checked_div(maior) {
        let mut acum: u32 = 0;
        for _ in 0..maior {
            acum += fracao;
            let anda_menor = acum & 0x1_0000 != 0;
            acum &= !0x1_0000;
            if ax >= ay {
                px += sx;
                py += if anda_menor { sy } else { 0 };
            } else {
                py += sy;
                px += if anda_menor { sx } else { 0 };
            }
        }
    }
    let largura = if plano == 1 || plano == 0 { 128 } else { 64 };
    (px.clamp(0, largura - w), py.clamp(0, 64 - h))
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
/// não é modelado.]
pub fn redesenho_do_timer(e: &mut Estado, j: &JanelasDesenho, k: &mut ContadoresRedesenho, t: &TabelasDesenho) -> u32 {
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
            desenhar_cortador(e, j.som_cortador);
        }
    }
    if j.edicao_aberta {
        let desenha = j.edicao_na_frente || {
            k.alterna_edicao = !k.alterna_edicao;
            !k.alterna_edicao
        };
        if desenha {
            passos += desenhar_edicao(e, j, t);
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

    /// A amarela em (65, 32) da superfície com a aranha comendo ao lado (vetor amarela_morte_aranha).
    fn aranha_a_vista() -> Estado {
        let mut e = Estado::vazio();
        e.s = super::super::rng::GeradorS::new(0x1234);
        e.vistas.plano = 1;
        e.vistas.ponto = (0, 0);
        e.aranha.ligada = 1;
        e.aranha.estado = 3;
        e.aranha.x = 64 * 16 + 8;
        e.aranha.y = 32 * 16 + 8;
        e.edicao_no_tick = Some(Box::new(EdicaoNoTick::padrao(25, 19)));
        e
    }

    #[test]
    fn desenho_no_tick_so_dentro_do_tick_e_com_a_janela_aberta() {
        let mut e = aranha_a_vista();
        assert_eq!(desenhar_no_tick(&mut e), 0, "fora do tick");
        comecar_tick(&mut e);
        assert_eq!(desenhar_no_tick(&mut e), 0, "aranha fora da vista (0, 0)");
        assert_eq!(centralizar_no_tick(&mut e, 1, 65, 32), 1, "a vista andou: redesenho com os palpos");
        assert_eq!(e.edicao_no_tick.as_ref().unwrap().ponto, (53, 22), "o eixo menor fica um passo aquém");
        assert_eq!(centralizar_no_tick(&mut e, 1, 65, 32), 1, "anda o passo que faltou");
        assert_eq!(centralizar_no_tick(&mut e, 1, 65, 32), 0, "a vista não andou: sem redesenho");
        assert_eq!(desenhar_no_tick(&mut e), 1);
        e.aranha.estado = 5;
        assert_eq!(desenhar_no_tick(&mut e), 0, "morrendo: sem palpos");
        terminar_tick(&mut e);
        e.aranha.estado = 3;
        assert_eq!(desenhar_no_tick(&mut e), 0, "fora do tick");
        e.edicao_no_tick = None;
        comecar_tick(&mut e);
        assert_eq!(centralizar_no_tick(&mut e, 1, 65, 32), 0, "janela fechada");
    }

    #[test]
    fn centralizar_no_tick_no_experimental_nao_desenha() {
        let mut e = aranha_a_vista();
        e.partida.tipo_jogo = 3;
        comecar_tick(&mut e);
        assert_eq!(centralizar_no_tick(&mut e, 1, 65, 32), 0);
        assert_eq!(e.edicao_no_tick.as_ref().unwrap().ponto, (0, 0));
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
        assert_eq!(redesenho_do_timer(&mut e, &j, &mut k, &TabelasDesenho::padrao()), 28);
        k.chamadas = 0;
        e.partida.velocidade = 3;
        let n: u32 = (0..8).map(|_| redesenho_do_timer(&mut e, &j, &mut k, &TabelasDesenho::padrao())).sum();
        assert_eq!(n, 2 * 28, "ultra: 1 em 4");
        e.partida.velocidade = 1;
        let atras = JanelasDesenho { quintal_na_frente: false, ..j };
        let n: u32 = (0..4).map(|_| redesenho_do_timer(&mut e, &atras, &mut k, &TabelasDesenho::padrao())).sum();
        assert_eq!(n, 2 * 28, "atrás: um sim, outro não");
        let inativa = JanelasDesenho { ativa: false, ..j };
        k.ticks = 0;
        let n: u32 = (0..16).map(|_| redesenho_do_timer(&mut e, &inativa, &mut k, &TabelasDesenho::padrao())).sum();
        assert_eq!(n, 2 * 28, "inativa: 1 tick em 8");
        let min = JanelasDesenho { minimizada: true, ..j };
        assert_eq!(redesenho_do_timer(&mut e, &min, &mut k, &TabelasDesenho::padrao()), 0);
    }
}
