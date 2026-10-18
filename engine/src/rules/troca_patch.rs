//! Troca de patch no quintal (`_XferPatch`, `1000:ab90`) e colocação de rainhas guardadas
//! (`_PlaceQueenInYard`, `1000:ad12`), sobre o `Estado`, sem janela.
//!
//! Especificação: docs/spec/telas/quintal.md §5 (o que cada rotina faz) e docs/spec/regras/geracao-mundo.md
//! §3 (`_RandWorld`, que gera o patch novo). Os pedidos à interface (música, texto) saem pela fila de
//! eventos do estado.

use super::estado::{Amarela, Aranha, ConfigSimulacao, Estado, Evento, Feromonios, FormigasLeao, Grade, LagartaEBichos};
use super::estado::{FEROMONIO, NINHO, SUPERFICIE};
use super::geracao::{Cor, Mundo, TabelasGeracao, TipoJogo};
use crate::formats::save::Lista;

/// Patches do quintal (x 0..12, y 0..16).
pub const COLUNAS: i16 = 12;
pub const LINHAS: i16 = 16;
/// Índice máximo de um patch da casa (`_MakeMap`: `n ≤ 37`).
pub const ULTIMO_PATCH_DA_CASA: i16 = 37;

/// Textos (SHARED tipo 4) e músicas de `_XferPatch` e `_PlaceQueenInYard`.
pub mod ids {
    /// "Só no Jogo Completo".
    pub const SO_NO_JOGO_COMPLETO: u16 = 10000;
    /// Poucas formigas pretas para mudar.
    pub const POUCAS_FORMIGAS: u16 = 10002;
    /// "Sorry, there is not a black colony at this location" (visto na bancada).
    pub const SEM_COLONIA_PRETA: u16 = 10004;
    /// Mudou para um patch do quintal.
    pub const MUDOU_QUINTAL: u16 = 10006;
    /// "You are transferring to this place in the house" (visto na bancada).
    pub const MUDOU_CASA: u16 = 10007;
    /// Música da troca de patch.
    pub const MUSICA_TROCA: u16 = 0x2AFA;
    /// Música de rainha colocada.
    pub const MUSICA_RAINHA: u16 = 0x2AFB;
    /// Som de erro das recusas (`_myBeginSound(1, …)`).
    pub const SOM_ERRO: u16 = 1;
}

/// Índice do patch nos mapas `_YMapPop*` (`x * 16 + y`), se está no quintal.
pub fn indice_patch(x: i16, y: i16) -> Option<usize> {
    ((0..COLUNAS).contains(&x) && (0..LINHAS).contains(&y)).then(|| (x * LINHAS + y) as usize)
}

/// O patch é da casa (`_MakeMap`).
pub fn e_casa(x: i16, y: i16) -> bool {
    x * LINHAS + y <= ULTIMO_PATCH_DA_CASA
}

/// O que `_XferPatch` fez.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultadoTroca {
    /// O patch escolhido é o atual: nada.
    MesmoPatch,
    /// Recusada, com o texto mostrado (o original também toca o som de erro).
    Recusada(u16),
    /// Trocou; o texto mostrado depende de o patch ser da casa.
    Trocou { mensagem: u16, casa: bool },
}

/// `_XferPatch`: troca para o patch `_YMapPnt` (`e.vistas.ponto_quintal`) se as condições deixarem
/// (quintal.md §5.2). `cfg` dá a barreira (`_Barrier`) do quintal e da casa.
pub fn trocar_patch(e: &mut Estado, tab: &TabelasGeracao, cfg: &ConfigSimulacao) -> Result<ResultadoTroca, String> {
    let (tx, ty) = e.vistas.ponto_quintal;
    if (tx, ty) == e.ambiente.patch {
        return Ok(ResultadoTroca::MesmoPatch);
    }
    let i = indice_patch(tx, ty).ok_or(format!("patch ({tx}, {ty}) fora do quintal"))?;
    let recusa = |e: &mut Estado, id: u16| {
        e.eventos.push(Evento::Som(ids::SOM_ERRO));
        e.eventos.push(Evento::Mensagem(id));
        Ok(ResultadoTroca::Recusada(id))
    };
    if e.tipo_jogo() != Some(TipoJogo::Completo) {
        return recusa(e, ids::SO_NO_JOGO_COMPLETO);
    }
    if e.preta.total_colonias < 2 && e.preta.populacao < 2 {
        return recusa(e, ids::POUCAS_FORMIGAS);
    }
    let pretas = e.preta.populacao_quintal.0[i];
    let vermelhas = e.vermelha.populacao_quintal.0[i];
    if pretas == 0 && (e.amarela.tipo != 0x40 || e.amarela.ninho_iniciado != 0) {
        return recusa(e, ids::SEM_COLONIA_PRETA);
    }

    // Gera antes de mudar o estado: se a geração falhar (caso não especificado), nada muda.
    let semente = e.ambiente.sementes.0[(ty * COLUNAS + tx) as usize];
    let mut m = mundo_para_troca(e);
    m.patch = (i32::from(tx), i32::from(ty));
    let mut s = e.s;
    m.gerar_patch(&mut s, tab, semente, i32::from(pretas), i32::from(vermelhas), i32::from(tx), i32::from(ty))?;

    e.eventos.push(Evento::Musica(ids::MUSICA_TROCA));
    e.ambiente.patch = (tx, ty);
    e.partida.acabou_de_trocar = 1;
    for c in [Cor::Preta, Cor::Vermelha] {
        let col = e.colonia_mut(c);
        col.rainhas_guardadas = 0;
        col.enxame_contagem = 0;
    }
    e.amarela.ninho_iniciado = 1;
    e.s = s;
    aplicar_patch(e, &m, cfg)?;
    if vermelhas == 0 {
        e.vermelha.saude = 0;
    }
    let casa = (i as i16) < 0x25;
    let mensagem = if casa { ids::MUDOU_CASA } else { ids::MUDOU_QUINTAL };
    e.eventos.push(Evento::Mensagem(mensagem));
    Ok(ResultadoTroca::Trocou { mensagem, casa })
}

/// O mundo como `_RandWorld` o encontra no meio do jogo: tipo de jogo, sementes, `_MeNestStarted`
/// (já 1 em `_XferPatch`) e os mapas atuais (a geração de um patch da casa não reescreve tudo, e as
/// formigas-leão ainda veem o `_LifeA` do patch anterior; geracao-mundo.md §3, passo 5).
fn mundo_para_troca(e: &Estado) -> Mundo {
    let tipo = e.tipo_jogo().unwrap_or(TipoJogo::Completo);
    let mut m = Mundo::vazio(tipo);
    m.sementes = e.ambiente.sementes.0;
    m.terreno = e.ambiente.terreno as u8;
    m.mapa_a = e.superficie.mapa.dados.clone();
    m.vida_a = e.superficie.vida.dados.clone();
    m.ninho_iniciado = true;
    m
}

/// Aplica ao estado o que `_RandWorld` (geracao-mundo.md §3) escreve, a partir do mundo gerado;
/// o resto (quintal, menino, bichos, controles, tempo) fica como está.
pub fn aplicar_patch(e: &mut Estado, m: &Mundo, cfg: &ConfigSimulacao) -> Result<(), String> {
    let ini = &cfg.inicial;
    e.ambiente.patch = (m.patch.0 as i16, m.patch.1 as i16);
    // _MakeMap / _OverlayTileSet.
    e.ambiente.terreno = i16::from(m.terreno);
    e.ambiente.barreira = if m.terreno == 0 { ini.barreira_quintal } else { ini.barreira_casa };
    e.superficie.mapa = Grade::de_bytes(SUPERFICIE, m.mapa_a.clone())?;
    e.superficie.vida = Grade::de_bytes(SUPERFICIE, m.vida_a.clone())?;
    for (n, mapa, saida, vida) in [
        (&mut e.ninho_b, &m.mapa_b, &m.saida_b, &m.vida_b),
        (&mut e.ninho_r, &m.mapa_r, &m.saida_r, &m.vida_r),
    ] {
        n.mapa = Grade::de_bytes(NINHO, mapa.clone())?;
        n.saida = Grade::de_bytes(NINHO, saida.clone())?;
        n.vida = Grade::de_bytes(NINHO, vida.clone())?;
    }
    // Passo 7: os seis mapas de cheiro zerados.
    let vazio = || Grade::nova(FEROMONIO);
    e.feromonios = Feromonios { alarme: vazio(), ninho_b: vazio(), trilha_b: vazio(), ninho_r: vazio(), trilha_r: vazio(), f: vazio() };
    // Passo 13: listas refeitas.
    for (lista, formigas) in [(Lista::A, &m.lista_a), (Lista::B, &m.lista_b), (Lista::R, &m.lista_r)] {
        // `_ListIndex? = 0` e as formigas novas por cima: as vagas além do fim guardam o lixo da lista
        // anterior, como no original (conferido na bancada, telas/quintal.md §8).
        let l = e.lista_mut(lista);
        l.indice = 0;
        for f in formigas {
            l.adicionar(*f);
        }
    }
    // Colônias: buracos, escavação, rainhas, ovos, comida, saúde (passos 10–18).
    for (cor, buracos, ultimo_buraco, escav, rainhas, ultimo_ovo, comida, saude) in [
        (Cor::Preta, &m.buracos_b, m.ultimo_buraco_b, m.escavacao_b, m.rainhas_pretas, m.ultimo_ovo_preto, m.comida_b, m.saude_b),
        (Cor::Vermelha, &m.buracos_r, m.ultimo_buraco_r, m.escavacao_r, m.rainhas_vermelhas, m.ultimo_ovo_vermelho, m.comida_r, m.saude_r),
    ] {
        let c = e.colonia_mut(cor);
        c.buracos = super::estado::Tabela(*buracos);
        // `_LastNewHole?`/`_LastNewExit?` só são escritos por `_MakeNewHole?` (colônia gerada).
        if ultimo_buraco != (0, 0) {
            c.ultimo_buraco = ultimo_buraco;
            c.ultima_saida = (ultimo_buraco.1, 0);
        }
        c.soma_x_escavada = escav.tot_x;
        c.soma_y_escavada = escav.tot_y;
        c.tiles_escavados = escav.cavados;
        c.centro_x = escav.massa_x;
        c.centro_y = escav.massa_y;
        c.rainhas = rainhas;
        c.ultimo_ovo = ultimo_ovo;
        c.comida = comida;
        c.saude = saude;
        c.perdas = 0;
        c.vista_ninho = (0x20, 1);
    }
    e.ambiente.lencol = m.lencol;
    e.ambiente.comida_superficie = m.comida_a;
    e.ambiente.ultima_comida = m.ultima_comida;
    e.partida.ciclo = 0;
    // _ClearHistory(0): séries e contadores de mortes zerados; começo do anel em 63.
    let h = &mut e.historico;
    let botoes = h.botoes;
    *h = super::estado::Historico { inicio: 63, contagem: 0, botoes, ..Default::default() };
    for c in [Cor::Preta, Cor::Vermelha] {
        let col = e.colonia_mut(c);
        col.comidas_por_predadores = 0;
        col.mortes_naturais = 0;
        col.mortas_em_luta = 0;
    }
    e.partida.ovos_postos_b = 0;
    e.partida.ovos_perdidos_b = 0;
    // _InitAntLions e _InitPillar (dentro de _MakeMap).
    let mut fl = FormigasLeao::default();
    // `_InitAntLions` grava só as n primeiras vagas; as outras (todas, na casa) ficam como estavam.
    (fl.x, fl.y) = (e.formigas_leao.x, e.formigas_leao.y);
    for (k, &(x, y)) in m.formigas_leao.iter().enumerate().take(10) {
        fl.x[k] = x;
        fl.y[k] = y;
    }
    fl.quantidade = m.formigas_leao.len() as i16;
    fl.iniciais = m.formigas_leao_iniciais;
    e.formigas_leao = fl;
    let mut lb = LagartaEBichos::default();
    // `_InitPillar` só usa as vagas 1 e 2 dos bichos-de-conta; a 0 fica como estava.
    let ant = &e.lagarta_e_bichos;
    (lb.bicho_x[0], lb.bicho_y[0], lb.bicho_dir[0], lb.bicho_chao[0]) =
        (ant.bicho_x[0], ant.bicho_y[0], ant.bicho_dir[0], ant.bicho_chao[0]);
    // Na casa `_InitPillar` não põe bichos-de-conta: as vagas 1 e 2 também ficam como estavam.
    let usadas = if m.terreno == 0 { 1 } else { 3 };
    for k in 1..usadas.min(3) {
        (lb.bicho_x[k], lb.bicho_y[k], lb.bicho_dir[k], lb.bicho_chao[k]) =
            (ant.bicho_x[k], ant.bicho_y[k], ant.bicho_dir[k], ant.bicho_chao[k]);
    }
    for (k, b) in m.bichos_de_conta.iter().enumerate().skip(usadas) {
        lb.bicho_x[k] = b.x;
        lb.bicho_y[k] = b.y;
        lb.bicho_dir[k] = b.dir;
        lb.bicho_chao[k] = b.chao as i16;
    }
    e.lagarta_e_bichos = lb;
    // _InitSpider.
    e.aranha = Aranha {
        x: m.aranha.x,
        y: m.aranha.y,
        dir: m.aranha.dir,
        arrotos: m.aranha.arrotos,
        ligada: m.aranha.ligada as i16,
        alvo: -2,
        // `_InitSpider` (`1020:4668`): `_StargLife = −1`, `_SuserX/Y = 0x40`.
        alvo_tipo: -1,
        destino_x: 0x40,
        destino_y: 0x40,
        ..Aranha::default()
    };
    // _InitYelloAnt.
    // `_InitYelloAnt` (`1030:6492`) escreve só estes campos e chama `_SetMyHealth(100)` e
    // `_ResetYellowVars`; o resto da amarela fica como estava (conferido na bancada).
    let a = &m.amarela;
    let anterior = e.amarela.clone();
    let mut am = Amarela {
        plano: a.plano,
        x: a.x,
        y: a.y,
        tipo: a.tipo as i16,
        dir: a.dir as i16,
        modo: a.modo,
        saude: a.saude,
        ovo_carregado: a.ovo_carregado,
        renascer_x: m.renascer.0,
        renascer_y: m.renascer.1,
        base: 0,
        cor: 0,
        soltando_alarme: 0,
        saude_contagem: 0,
        ..anterior
    };
    am.quer_comida = if am.aviso_saude < am.saude && am.saude > 9 { 0 } else { 1 };
    (am.alvo_plano, am.alvo_x, am.alvo_y, am.ultimo_x, am.ultimo_y) = (am.plano, am.x, am.y, am.x, am.y);
    (am.comando, am.mover, am.dis, am.dis_anterior, am.passos, am.contorno_contagem) = (0, 0, 0, 0, 0, -2);
    e.amarela = am;
    // Passo 18: vistas.
    e.vistas.ponto = (0x40, 0x20);
    e.vistas.ponto_superficie = (0x40, 0x20);
    e.contar_formigas();
    Ok(())
}

/// O que `_PlaceQueenInYard` fez num clique.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultadoRainha {
    /// Não há rainhas guardadas (`_QueenStorageB < 1`).
    SemRainhas,
    /// O patch está fora do quintal ou a mais de 3 patches do atual em x ou em y.
    ForaDoAlcance,
    /// Rainha posta no patch; `restam` = rainhas ainda guardadas.
    Colocada { restam: i16 },
}

/// Distância máxima (em x e em y) entre o patch atual e o de uma rainha colocada.
pub const ALCANCE_RAINHA: i16 = 3;

/// Patches válidos para `_PlaceQueenInYard`: a até 3 do atual em x e em y, limitados ao quintal.
pub fn alcance_rainha(e: &Estado) -> (i16, i16, i16, i16) {
    let (px, py) = e.ambiente.patch;
    let c = |v: i16, max: i16| v.clamp(0, max - 1);
    (c(px - ALCANCE_RAINHA, COLUNAS), c(py - ALCANCE_RAINHA, LINHAS), c(px + ALCANCE_RAINHA, COLUNAS), c(py + ALCANCE_RAINHA, LINHAS))
}

/// Um clique de `_PlaceQueenInYard` no patch (x, y) (quintal.md §5.3).
pub fn colocar_rainha(e: &mut Estado, x: i16, y: i16) -> ResultadoRainha {
    if e.preta.rainhas_guardadas < 1 {
        return ResultadoRainha::SemRainhas;
    }
    if e.controles.modo_por_rainha == 1 {
        e.controles.modo_por_rainha = 0;
    }
    let (x0, y0, x1, y1) = alcance_rainha(e);
    let Some(i) = indice_patch(x, y).filter(|_| (x0..=x1).contains(&x) && (y0..=y1).contains(&y)) else {
        return ResultadoRainha::ForaDoAlcance;
    };
    e.eventos.push(Evento::Musica(ids::MUSICA_RAINHA));
    let p = &mut e.preta;
    p.rainhas_guardadas -= 1;
    p.populacao_quintal.0[i] = p.populacao_quintal.0[i].wrapping_add(1);
    p.colonias_fundadas += 1;
    ResultadoRainha::Colocada { restam: p.rainhas_guardadas }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn indices_e_casa() {
        assert_eq!(indice_patch(11, 8), Some(184));
        assert_eq!(indice_patch(12, 0), None);
        assert_eq!(indice_patch(0, -1), None);
        assert!(e_casa(0, 0) && e_casa(2, 5) && e_casa(1, 15));
        assert!(!e_casa(2, 6) && !e_casa(11, 8));
    }

    #[test]
    fn rainha_fora_do_alcance_e_sem_rainhas() {
        let mut e = Estado::vazio();
        e.ambiente.patch = (11, 8);
        assert_eq!(colocar_rainha(&mut e, 10, 8), ResultadoRainha::SemRainhas);
        e.preta.rainhas_guardadas = 2;
        e.controles.modo_por_rainha = 1;
        assert_eq!(colocar_rainha(&mut e, 7, 8), ResultadoRainha::ForaDoAlcance);
        assert_eq!(e.controles.modo_por_rainha, 0, "o modo automático é desligado antes do teste");
        assert_eq!(colocar_rainha(&mut e, 8, 11), ResultadoRainha::Colocada { restam: 1 });
        assert_eq!(e.preta.populacao_quintal.0[8 * 16 + 11], 1);
        assert_eq!(e.preta.colonias_fundadas, 1);
        // O próprio patch atual também é aceito.
        assert_eq!(colocar_rainha(&mut e, 11, 8), ResultadoRainha::Colocada { restam: 0 });
        assert_eq!(e.retirar_eventos(), vec![Evento::Musica(ids::MUSICA_RAINHA), Evento::Musica(ids::MUSICA_RAINHA)]);
    }

    #[test]
    fn alcance_limitado_ao_quintal() {
        let mut e = Estado::vazio();
        e.ambiente.patch = (11, 1);
        assert_eq!(alcance_rainha(&e), (8, 0, 11, 4));
    }
}
