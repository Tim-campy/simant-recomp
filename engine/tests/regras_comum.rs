//! Pontos em que uma parte das regras chama a da formiga amarela (`_YellowFight`, `_DoTroph`) e o estado
//! do tutorial guardado em `RegrasOriginais`. Antes do módulo comum (docs/decisoes.md, "Módulo comum
//! das regras") esses pontos eram vazios ou criavam memória nova a cada chamada.

use engine::formats::save::{Formiga, Lista};
use engine::rules::amarela::TabelasAmarela;
use engine::rules::estado::{Estado, Evento};
use engine::rules::geracao::Cor;
use engine::rules::ninho::{simular_ninho, TabelasNinho};
use engine::rules::rng::GeradorS;
use engine::rules::tick::{RegrasConselhos, RegrasOriginais};

fn amarela_no_ninho_preto(x: i32, y: i32) -> Estado {
    let mut e = Estado::vazio();
    e.s = GeradorS::new(0x1234);
    e.preta.saude = 100;
    e.vermelha.saude = 100;
    let a = &mut e.amarela;
    a.plano = 2;
    a.x = x as i16;
    a.y = y as i16;
    a.tipo = 0x10;
    a.dir = 2;
    a.modo = 0;
    a.saude = 100;
    e.ninho_b.vida.definir(x, y, 0xFF);
    e
}

/// ninho.md §3.2 / formiga-amarela.md §10.1: a vermelha invasora na célula da amarela (preta) chama
/// `_YellowFight(2, i)`. Com `_Invincible` a amarela vence: a vermelha some e conta como morta.
#[test]
fn invasora_no_ninho_preto_luta_com_a_amarela() {
    let mut e = amarela_no_ninho_preto(10, 10);
    e.partida.invencivel = 1;
    e.adicionar_formiga(Lista::B, Formiga { x: 10, y: 10, modo: 0, tipo: 0x92, s: 0 });
    // `_AddAntToBList` gravou a vermelha na vida; a amarela está na mesma célula.
    e.ninho_b.vida.definir(10, 10, 0xFF);
    simular_ninho(&mut e, Cor::Preta, TabelasNinho::padrao(), TabelasAmarela::padrao());
    assert_eq!(e.ninho_b.lista.vagas[0].tipo, 0, "a invasora perdeu");
    assert_eq!(e.vermelha.mortas_em_luta, 1);
    assert_eq!(e.ninho_b.vida.em(10, 10), Some(0xFF), "a amarela continua marcada");
    assert_eq!((e.amarela.plano, e.amarela.x, e.amarela.y), (2, 10, 10));
}

/// ninho.md §4 (MOVER) / formiga-amarela.md §7.2: a preta que entra na célula da amarela com fome a
/// alimenta (`_DoTroph` → `_EatMyFood(1)`): vida cheia e sem fome.
#[test]
fn preta_no_ninho_alimenta_a_amarela() {
    let mut e = amarela_no_ninho_preto(11, 10);
    e.amarela.saude = 5;
    e.amarela.aviso_saude = 30;
    e.amarela.quer_comida = 1;
    // Operária virada para a direita (direção 2), em passeio (M = 0).
    e.adicionar_formiga(Lista::B, Formiga { x: 10, y: 10, modo: 0, tipo: 0x12, s: 0 });
    simular_ninho(&mut e, Cor::Preta, TabelasNinho::padrao(), TabelasAmarela::padrao());
    assert_eq!(e.amarela.saude, 100);
    assert_eq!(e.amarela.quer_comida, 0);
    // A amarela vira para quem a alimentou (direção 6, oeste).
    assert_eq!(e.amarela.dir, 6);
}

/// conselhos.md §3: `_TimeTemp` vale de um tick para o outro. A lição 3 marca "escavou 2 além do
/// atual"; com 1 célula escavada ela ainda não está concluída. (Com memória nova a cada chamada, o
/// marco voltava a 0 e a lição passava logo.)
#[test]
fn memoria_do_tutor_fica_entre_os_ticks() {
    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 0;
    e.tutorial.licao = 3;
    let mut r = RegrasOriginais::default();
    for _ in 0..8 {
        r.dar_conselhos(&mut e);
    }
    assert_eq!(e.tutorial.dada, 1);
    assert_eq!(r.memoria_tutor.tempo_limite, 2);
    assert!(e.retirar_eventos().contains(&Evento::Mensagem(11004)));
    e.preta.tiles_escavados = 1;
    for _ in 0..8 {
        r.dar_conselhos(&mut e);
    }
    assert_eq!(e.tutorial.licao, 3, "1 célula escavada não passa do marco 2");
    e.preta.tiles_escavados = 3;
    for _ in 0..8 {
        r.dar_conselhos(&mut e);
    }
    assert_eq!(e.tutorial.licao, 4);
}

/// conselhos.md §3: a lição 10 termina quando a janela 256 está na frente, informação que a interface
/// grava em `RegrasOriginais::interface_tutor`.
#[test]
fn tutor_consulta_as_janelas_da_interface() {
    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 0;
    e.tutorial.licao = 10;
    e.tutorial.dada = 1;
    let mut r = RegrasOriginais::default();
    for _ in 0..8 {
        r.dar_conselhos(&mut e);
    }
    assert_eq!(e.tutorial.licao, 10);
    r.interface_tutor.janelas.push(256);
    for _ in 0..8 {
        r.dar_conselhos(&mut e);
    }
    assert_eq!(e.tutorial.licao, 11);
}
