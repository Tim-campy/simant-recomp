//! Execução das ações pedidas pelos scripts dos mods (API 0.17, docs/api-mods.md, "Ações").
//!
//! Os métodos de ação do objeto `estado` (`engine/src/scripts.rs`) só validam os argumentos e
//! guardam um [`PedidoMod`]; a `Partida` chama [`aplicar`] logo depois do gancho, com as tabelas das
//! regras. Cada ação usa a rotina do original que faz a mesma coisa no jogo (o começo da chuva de
//! `_DoSimYard`, `_XferPatch`, `_YellowDeath`, `_MysteryButton`...), ou, quando o original não tem
//! rotina (menino cortando no patch atual, reprodutores), escreve os mesmos campos que a rotina do
//! original escreveria naquela situação (as preparações de `tools/cenarios_tick.py`, conferidas na
//! bancada). Nenhuma ação sorteia S ou R além do que a rotina sorteia. Recusas voltam como `Err`
//! com o motivo (vão para o registro do mod e para a faixa) e não mudam nada; os motivos vêm de
//! `mods.recusa.*` em `data/idiomas/` (docs/i18n.md).

use super::partida::Partida;
use crate::formats::save::Lista;
use crate::rules::amarela::{self, CausaMorte};
use crate::rules::estado::{Estado, Evento};
use crate::rules::geracao::Cor;
use crate::rules::quintal::{self, Relogios};
use crate::rules::troca_patch::{self, ResultadoTroca};
use crate::rules::{inimigos, misterio};
use crate::scripts::PedidoMod;
use crate::tr;

/// Nó do menino dentro de casa onde ele fica parado com o inseticida (quintal.md §4; cenário
/// `inseticida` da bancada: patch (0, 12)).
const NO_INSETICIDA: i16 = 32;
/// Primeiro nó "lá fora" (`_NodeNum` ≥ 0x26: `_SimKidOutside`).
const NO_LA_FORA: i16 = 0x26;
/// `_BoyHere` = 3: cortando a grama (quintal.md §3).
const MENINO_CORTANDO: i16 = 3;
/// Modo 14 (Pre-Flight) e grupo 4 (macho alado), formigas.md.
const MODO_PRE_VOO: u8 = 14;
const GRUPO_MACHO: u8 = 4;
const GRUPO_RAINHA_NOVA: u8 = 8;
/// Grupos adultos que viram reprodutores (operárias e soldados; cenário `revoada_natural`).
const GRUPOS_ADULTOS: [u8; 7] = [1, 2, 3, 5, 6, 7, 9];

fn lista_do_ninho(cor: Cor) -> Lista {
    match cor {
        Cor::Preta => Lista::B,
        Cor::Vermelha => Lista::R,
    }
}

fn e_da_cor(tipo: u8, cor: Cor) -> bool {
    (tipo & 0x80 != 0) == (cor == Cor::Vermelha)
}

fn grupo(tipo: u8) -> u8 {
    (tipo & 0x78) >> 3
}

/// Executa um pedido. `AvancarTicks` e `PararAvanco` são da própria `Partida`.
pub fn aplicar(p: &mut Partida, pedido: &PedidoMod) -> Result<(), String> {
    p.vistas_para_estado();
    let rel = Relogios::do_port(&p.estado, &p.regras.quintal);
    let e = &mut p.estado;
    match *pedido {
        PedidoMod::ComecarChuva => {
            if e.ambiente.chovendo != 0 {
                return Err(tr!("mods.recusa.ja_chove"));
            }
            if e.ambiente.terreno == 1 {
                return Err(tr!("mods.recusa.chuva_em_casa"));
            }
            quintal::comecar_chuva(e, &p.regras.quintal, rel);
        }
        PedidoMod::PararChuva => {
            if e.ambiente.chovendo == 0 {
                return Err(tr!("mods.recusa.nao_chove"));
            }
            // `_DoSimYard` desliga a chuva no tick em que a contagem chega a 0.
            e.ambiente.chuva_contagem = 0;
        }
        PedidoMod::MeninoCortarGrama => {
            let (px, py) = e.ambiente.patch;
            if troca_patch::e_casa(px, py) {
                return Err(tr!("mods.recusa.patch_da_casa"));
            }
            menino_cortando_aqui(e, px, py);
        }
        PedidoMod::MeninoInseticida => {
            if e.ambiente.casa_a_venda == 1 {
                return Err(tr!("mods.recusa.casa_a_venda"));
            }
            e.menino.no_numero = NO_INSETICIDA;
            e.menino.no_contagem = 0;
        }
        PedidoMod::CriarReprodutores { cor, n } => {
            if criar_reprodutores(e, cor, n) == 0 {
                return Err(tr!("mods.recusa.sem_adultos"));
            }
        }
        PedidoMod::ArmarRevoada(cor) => {
            if e.colonia(cor).revoada != 0 {
                return Err(tr!("mods.recusa.revoada_armada"));
            }
            if reprodutores(e, cor) == 0 {
                return Err(tr!("mods.recusa.sem_reprodutores"));
            }
            e.colonia_mut(cor).revoada = p.regras.superficie.regras.revoada_vermelha_contagem;
        }
        PedidoMod::GuardarRainhas { cor, n } => {
            let q = &mut e.colonia_mut(cor).rainhas_guardadas;
            *q = q.saturating_add(n);
        }
        PedidoMod::PovoarPatch { cor, x, y, n } => {
            let i = troca_patch::indice_patch(x, y).ok_or_else(|| tr!("mods.recusa.patch_fora"))?;
            e.colonia_mut(cor).populacao_quintal.0[i] = n;
        }
        PedidoMod::TrocarPatch { x, y } => return trocar_patch(p, x, y),
        PedidoMod::TrazerGato => {
            if e.bichos.gato_ligado != 0 {
                return Err(tr!("mods.recusa.gato"));
            }
            quintal::gato_aparece(e);
        }
        PedidoMod::Latir => quintal::latido(e, &p.regras.quintal, rel),
        PedidoMod::TrazerAranha => {
            if e.aranha.ligada != 0 {
                return Err(tr!("mods.recusa.aranha"));
            }
            inimigos::aranha_entra(e);
        }
        PedidoMod::PorFormigaLeao { x, y } => {
            // `_MakeHousePatch` → `_InitAntLions(0)`: a casa não tem formigas-leão, e os tiles delas
            // são outros desenhos no conjunto da casa.
            if e.ambiente.terreno == 1 {
                return Err(tr!("mods.recusa.leao_em_casa"));
            }
            if !inimigos::nova_formiga_leao(e, &p.regras.inimigos, x, y) {
                return Err(tr!("mods.recusa.leao_sem_lugar", x = x, y = y));
            }
        }
        PedidoMod::SaudeAmarela(v) => amarela::definir_saude(e, &p.tabelas_amarela, v),
        PedidoMod::MatarAmarela => amarela::matar_amarela(e, &p.tabelas_amarela, CausaMorte::Fome),
        PedidoMod::SaudeColonia { cor, v } => e.colonia_mut(cor).saude = v,
        PedidoMod::AvancarLicao => {
            if e.partida.tipo_jogo != 0 {
                return Err(tr!("mods.recusa.so_tutorial"));
            }
            // O que `_RunTutor` faz quando `_LessonDone` é verdadeiro.
            e.tutorial.licao += 1;
            e.tutorial.dada = 0;
        }
        PedidoMod::TerminarJogo { vitoria } => {
            if e.partida.fim_de_jogo != 0 {
                return Err(tr!("mods.recusa.jogo_acabou"));
            }
            // Como `_SimColonies` no fim do Jogo Completo (vitória) e `_CutGrass` (derrota).
            let m = &p.regras.quintal.mensagens;
            if vitoria {
                e.eventos.push(Evento::Musica(m.musica_vitoria));
                e.eventos.push(Evento::Mensagem(m.vitoria));
            } else {
                e.eventos.push(Evento::Mensagem(m.derrota));
            }
            e.partida.fim_de_jogo = 1;
            e.partida.pretas_venceram = i16::from(vitoria);
        }
        PedidoMod::Misterio => {
            let efeito = misterio::botao_misterio(&mut p.estado, &mut p.regras, &p.tabelas_amarela);
            p.ultimo_misterio = Some(efeito);
        }
        PedidoMod::AvancarTicks(_) | PedidoMod::PararAvanco => {}
    }
    Ok(())
}

/// O menino lá fora cortando a grama no meio do patch (px, py): os campos do estado 3 de
/// `_SimKidOutside` (cenário `menino` da bancada). `_BoyPx = (Y + X − 200) / 28`, `_BoyPy = (Y − 38) / 10`.
fn menino_cortando_aqui(e: &mut Estado, px: i16, py: i16) {
    let y = 38 + py * 10 + 5;
    let x = px * 28 + 14 + 200 - y;
    let m = &mut e.menino;
    m.no_numero = NO_LA_FORA;
    m.presente = MENINO_CORTANDO;
    m.cortando_grama = 1;
    m.x = x;
    m.y = y;
    m.px = px;
    m.py = py;
    m.ultimo_corte_x = px;
    m.ultimo_corte_y = py;
    m.pe_presente = 1;
}

/// Até `n` operárias e soldados da cor (primeiro os do ninho, depois os da superfície) viram machos
/// alados (grupo 4) em pré-voo (modo 14), com a marca no mapa de vida (cenário `revoada_natural`,
/// que usa as listas R e A). Devolve quantas.
fn criar_reprodutores(e: &mut Estado, cor: Cor, n: i32) -> i32 {
    let mut feitas = 0;
    for area in [lista_do_ninho(cor), Lista::A] {
        for i in 0..e.lista(area).usadas() {
            if feitas >= n {
                return feitas;
            }
            let f = e.lista(area).vagas[i];
            if f.tipo == 0 || !e_da_cor(f.tipo, cor) || !GRUPOS_ADULTOS.contains(&grupo(f.tipo)) {
                continue;
            }
            let t = (f.tipo & 0x80) | (GRUPO_MACHO << 3) | (f.tipo & 7);
            let v = &mut e.lista_mut(area).vagas[i];
            v.tipo = t;
            v.modo = MODO_PRE_VOO;
            let (x, y) = (i32::from(f.x), i32::from(f.y));
            if area == Lista::A {
                e.superficie.vida.definir(x, y, t);
            } else {
                e.ninho_mut(cor).vida.definir(x, y, t);
            }
            feitas += 1;
        }
    }
    feitas
}

/// Machos e rainhas aladas da cor em todas as listas.
fn reprodutores(e: &Estado, cor: Cor) -> usize {
    Lista::TODAS
        .into_iter()
        .flat_map(|a| e.lista(a).ativas().map(|(_, f)| f.tipo).collect::<Vec<_>>())
        .filter(|&t| t != 0 && e_da_cor(t, cor) && matches!(grupo(t), GRUPO_MACHO | GRUPO_RAINHA_NOVA))
        .count()
}

/// `_XferPatch` para (x, y), como o duplo clique na janela do quintal (`core::quintal_mapa::clicar`).
fn trocar_patch(p: &mut Partida, x: i16, y: i16) -> Result<(), String> {
    let geracao = p.regras.geracao.as_ref().ok_or("generation tables not loaded")?;
    p.nav.pontos[0] = (i32::from(x), i32::from(y));
    p.estado.vistas.ponto_quintal = (x, y);
    match troca_patch::trocar_patch(&mut p.estado, geracao, &p.cfg_simulacao)? {
        ResultadoTroca::Trocou { .. } => {
            let e = &p.estado;
            let par = |(x, y): (i16, i16)| (i32::from(x), i32::from(y));
            // Como o duplo clique: a vista fica nos pontos de _RandWorld (core::quintal_mapa::clicar).
            p.nav.pontos = [par(e.vistas.ponto_quintal), par(e.vistas.ponto_superficie), par(e.preta.vista_ninho), par(e.vermelha.vista_ninho)];
            Ok(())
        }
        ResultadoTroca::MesmoPatch => Err(tr!("mods.recusa.mesmo_patch")),
        ResultadoTroca::Recusada(id) => Err(tr!("mods.recusa.troca_recusada", id = id)),
    }
}
