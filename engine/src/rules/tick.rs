//! Esqueleto do tick da simulação (`_DoAntSim`, `1028:0000`) e a cadência do temporizador.
//! Especificação: docs/spec/regras/simulacao-visao-geral.md (§2–5 e "Implementação no port").
//!
//! O tick faz aqui mesmo o que a spec descreve por inteiro (contadores, periodicidade, alimentação
//! periódica, compactação das listas, `_CountAnts`, `_HistUpdate`, `_ClrModePop`/`_TallyModePop`,
//! decremento de `_FlyAway*`, fim de jogo) e chama as sub-rotinas de cada assunto pelas traits
//! abaixo, uma por documento de regras. Cada método tem implementação padrão vazia: um subsistema
//! ainda não implementado simplesmente não faz nada (e não consome sorteios).
//!
//! Para implementar um assunto: em `rules/<assunto>.rs`, escreva `impl Regras<Assunto> for
//! RegrasOriginais { ... }` e apague a linha vazia correspondente no fim deste arquivo.

use super::estado::{indice_cor, ConfigSimulacao, Estado, Evento};
use super::geracao::{Cor, TabelasGeracao};

/// Quintal (`_DoSimYard` com menino, pássaro, gato, cachorro, chuva e `_SimColonies`) e água
/// (`_DoWater`). Spec: simulacao-visao-geral.md §6, ninho.md §8.
pub trait RegrasQuintal {
    /// `_DoSimYard` (`1030:14a8`), todo tick. Inclui o incremento de `_YardCycle` e `_SimColonies`.
    fn simular_quintal(&mut self, _e: &mut Estado) {}
    /// `_DoWater` (`1020:07c6`), todo tick.
    fn simular_agua(&mut self, _e: &mut Estado) {}
}

/// Formiga-leão, aranha e lagarta (com bicho-de-conta). Spec: inimigos.md.
pub trait RegrasInimigos {
    /// `_DoAntLions` (`1030:43fa`), todo tick.
    fn simular_formigas_leao(&mut self, _e: &mut Estado) {}
    /// `_MoveSpider` (`1020:4704`), todo tick.
    fn mover_aranha(&mut self, _e: &mut Estado) {}
    /// `_DoPillar` (`1030:4cdc`, chama `_DoSow` no começo), só em ticks com `_Cycle` ímpar.
    fn simular_lagarta(&mut self, _e: &mut Estado) {}
}

/// Formigas da superfície, comida, estratégia e líder vermelha. Spec: superficie.md.
pub trait RegrasSuperficie {
    /// `_DoAntSimA` (`1028:04d8`): lista A, de trás para frente.
    fn simular_superficie(&mut self, _e: &mut Estado) {}
    /// `_AddFood(quantidade, som)` (`1030:6a58`), chamado pela alimentação periódica.
    fn adicionar_comida(&mut self, _e: &mut Estado, _quantidade: i32, _som: bool) {}
    /// `_GetStrategy` (`1030:0000`) com `_SetCasteProd` e `_SetModeProd`, todo tick.
    fn escolher_estrategia(&mut self, _e: &mut Estado) {}
    /// `_MakeRedInitiator` (`1028:967c`), chamado quando nenhuma vermelha está no modo 19.
    fn criar_lider_vermelha(&mut self, _e: &mut Estado) {}
}

/// Formigas dentro dos ninhos (com ovos e rainha). Spec: ninho.md e ciclo-de-vida.md.
pub trait RegrasNinho {
    /// `_DoAntSimB` (`1028:2d4e`) para `Cor::Preta`, `_DoAntSimR` (`1028:5344`) para `Cor::Vermelha`.
    fn simular_ninho(&mut self, _e: &mut Estado, _cor: Cor) {}
}

/// Formiga amarela. Spec: formiga-amarela.md.
pub trait RegrasAmarela {
    /// `_DoAntSimY` (`1028:7cf6`): vida, ovos da amarela rainha.
    fn simular_amarela(&mut self, _e: &mut Estado) {}
    /// `_DoAntMoveY` (`1028:75f4`): movimento por ordem (`_MeMoveMe`).
    fn mover_amarela(&mut self, _e: &mut Estado) {}
}

/// Tutorial e mensagens de conselho. Spec: [em aberto] (simulacao-visao-geral.md, lacunas).
pub trait RegrasConselhos {
    /// `_Feedback` (`1030:78ca`): todo tick e também dentro de `_FullCount`.
    fn dar_conselhos(&mut self, _e: &mut Estado) {}
}

/// Todas as sub-rotinas do tick. Implementada automaticamente para quem implementa as seis partes.
pub trait Regras:
    RegrasQuintal + RegrasInimigos + RegrasSuperficie + RegrasNinho + RegrasAmarela + RegrasConselhos
{
}

impl<T> Regras for T where
    T: RegrasQuintal + RegrasInimigos + RegrasSuperficie + RegrasNinho + RegrasAmarela + RegrasConselhos + ?Sized
{
}

/// As regras do jogo original. Guarda as tabelas de dados que as sub-rotinas usam.
#[derive(Clone, Debug, Default)]
pub struct RegrasOriginais {
    /// Tabelas da geração (seno de `_AddFood`, direções...), de `data/geracao.json` + banco.
    pub geracao: Option<TabelasGeracao>,
}

/// Um passo da simulação (`_DoAntSim`), na ordem exata do original (simulacao-visao-geral.md §5).
pub fn tick(e: &mut Estado, regras: &mut dyn Regras) {
    // 1–2. Ciclo (0..0x1000) e tempo de jogo (32 bits).
    e.partida.ciclo = e.partida.ciclo.wrapping_add(1);
    if e.partida.ciclo > 0x1000 {
        e.partida.ciclo = 0;
    }
    e.partida.tempo = e.partida.tempo.wrapping_add(1);
    let ciclo = e.partida.ciclo;

    // 3. Balões de fala.
    if e.partida.opcoes[5] == 1 {
        e.baloes.zerar();
    }

    // 4. Alimentação (corpo de `_FeedAnts`), a cada 64 ticks.
    if ciclo & 0x3F == 0 {
        alimentar(e, regras);
    }

    // 5. Manutenção em quatro fases (corpo de `_DoSmells`), a cada 32 ticks.
    if ciclo & 0x1F == 0 {
        match (ciclo & 0x60) >> 5 {
            0 => {
                e.superficie.lista.compactar();
                contagem_completa(e, regras);
            }
            1 => {
                e.ninho_b.lista.compactar();
                feromonios_fase_preta(e);
            }
            2 => contagem_completa(e, regras),
            _ => {
                e.ninho_r.lista.compactar();
                feromonios_fase_vermelha(e);
            }
        }
    }

    // 6–10. Quintal, água, formiga-leão, aranha e (ticks ímpares) lagarta.
    regras.simular_quintal(e);
    regras.simular_agua(e);
    regras.simular_formigas_leao(e);
    regras.mover_aranha(e);
    if ciclo & 1 != 0 {
        regras.simular_lagarta(e);
    }

    // 11. Estratégia.
    regras.escolher_estrategia(e);

    // 12. `_ClrModePop`.
    e.modos_do_tick = [[0; 20]; 2];

    // 13. Revoada: contagem regressiva (o original testa ≠ 0, não > 0).
    for cor in [Cor::Preta, Cor::Vermelha] {
        let c = e.colonia_mut(cor);
        if c.revoada != 0 {
            c.revoada -= 1;
        }
    }

    // 14–17. Formigas: superfície, ninho preto, ninho vermelho, amarela.
    regras.simular_superficie(e);
    regras.simular_ninho(e, Cor::Preta);
    regras.simular_ninho(e, Cor::Vermelha);
    regras.simular_amarela(e);

    // 18. `_TallyModePop`.
    e.resumir_modos();

    // 19. Líder vermelha, se nenhuma vermelha está no modo 19.
    if e.modos_do_tick[indice_cor(Cor::Vermelha)][19] < 1 {
        regras.criar_lider_vermelha(e);
    }

    // 20–21. Movimento da amarela e conselhos.
    regras.mover_amarela(e);
    regras.dar_conselhos(e);

    // 22–23. Fim de jogo e "precisa salvar".
    if e.partida.fim_de_jogo != 0 {
        e.eventos.push(Evento::DialogoFimDeJogo);
    }
    e.precisa_salvar = true;
}

/// Corpo de `_FeedAnts` (`1028:0474`): saúde −1 e, fora do Experimental, comida nova na superfície
/// quando `_FoodA < _NextFeed`.
fn alimentar(e: &mut Estado, regras: &mut dyn Regras) {
    if e.partida.sempre_saudavel == 0 {
        e.preta.saude = (e.preta.saude - 1).max(0);
    }
    e.vermelha.saude = (e.vermelha.saude - 1).max(0);
    if e.partida.tipo_jogo != 3 && e.ambiente.comida_superficie < e.ambiente.proxima_comida {
        regras.adicionar_comida(e, 150, true);
        e.ambiente.proxima_comida = e.s.rand1(50) as i16 + 1;
    }
}

/// Fases 0 e 2: `_FullCount` (= `_CountAnts` + `_Feedback`), `_HistUpdate`, `_SmoothAlarm`.
fn contagem_completa(e: &mut Estado, regras: &mut dyn Regras) {
    e.contar_formigas();
    regras.dar_conselhos(e);
    e.atualizar_historico();
    suavizar_alarme(e);
}

/// Fase 1 (tick ≡ 32 mod 128): `_FillHolesBN`, `_ColonySmellBN`, `_ColonySmellBT`.
fn feromonios_fase_preta(e: &mut Estado) {
    // Buracos pretos, decaimento do ninho preto e da trilha preta, nessa ordem (feromonios.md §2–3).
    super::superficie::rodar_fase_de_cheiro(e, super::feromonios::FaseCheiro::Pretos);
}

/// Fase 3 (tick ≡ 96 mod 128): `_FillHolesRN`, `_ColonySmellRN`, `_ColonySmellRT`.
fn feromonios_fase_vermelha(e: &mut Estado) {
    // Idem, para a cor vermelha.
    super::superficie::rodar_fase_de_cheiro(e, super::feromonios::FaseCheiro::Vermelhos);
}

/// `_SmoothAlarm` (`1028:9380`), ticks ≡ 0 e 64 (mod 128), depois de `_HistUpdate`.
fn suavizar_alarme(e: &mut Estado) {
    // feromonios.md §3, "Suavização do alarme".
    super::superficie::rodar_fase_de_cheiro(e, super::feromonios::FaseCheiro::Alarme);
}

// ---------------------------------------------------------------------------------------------
// Temporizador: quando um tick deve rodar (simulacao-visao-geral.md §2–4)
// ---------------------------------------------------------------------------------------------

/// Pausa (§4): o tick roda se o jogo não está pausado, ou se a amarela tem ordem em andamento
/// (`_MeMoveMe` ≠ 0) e a ferramenta atual é < 10.
pub fn simulacao_liberada(e: &Estado) -> bool {
    e.partida.pausado == 0 || (e.amarela.mover != 0 && e.partida.ferramenta < 10)
}

/// Contador de mensagens de temporizador desde o último tick (DGROUP `0x02EE` no original).
/// A camada de cima chama [`Relogio::contar_mensagem`] a cada "batida" (17 ms no original) e roda
/// um tick sempre que [`Relogio::liberar_tick`] devolver `true`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Relogio {
    pub atraso: i32,
}

impl Relogio {
    pub fn contar_mensagem(&mut self) {
        self.atraso += 1;
    }

    /// Velocidade (§3): `limite = atrasos_velocidade[_GameSpeed]`; roda se `limite ≤ 0` ou
    /// `limite ≤ atraso`, e zera o atraso. Índice fora da tabela conta como "todo laço".
    pub fn liberar_tick(&mut self, e: &Estado, cfg: &ConfigSimulacao) -> bool {
        if !simulacao_liberada(e) {
            return false;
        }
        let limite = usize::try_from(e.partida.velocidade)
            .ok()
            .and_then(|v| cfg.atrasos_velocidade.get(v).copied())
            .unwrap_or(0) as i32;
        if limite <= 0 || limite <= self.atraso {
            self.atraso = 0;
            true
        } else {
            false
        }
    }
}

// As implementações de cada parte para `RegrasOriginais` ficam em `rules/<assunto>.rs`
// (quintal, inimigos, superficie, ninho, amarela, conselhos).

#[cfg(test)]
mod testes {
    use super::*;
    use crate::formats::save::{Formiga, Lista};

    /// Regras que só anotam quem foi chamado, com o `_Cycle` do momento.
    #[derive(Default)]
    struct Gravador {
        chamadas: Vec<(i16, &'static str)>,
    }

    impl Gravador {
        fn anotar(&mut self, e: &Estado, nome: &'static str) {
            self.chamadas.push((e.partida.ciclo, nome));
        }
        fn ciclos(&self, nome: &str) -> Vec<i16> {
            self.chamadas.iter().filter(|c| c.1 == nome).map(|c| c.0).collect()
        }
        fn nomes_no_ciclo(&self, ciclo: i16) -> Vec<&'static str> {
            self.chamadas.iter().filter(|c| c.0 == ciclo).map(|c| c.1).collect()
        }
    }

    impl RegrasQuintal for Gravador {
        fn simular_quintal(&mut self, e: &mut Estado) {
            self.anotar(e, "quintal");
        }
        fn simular_agua(&mut self, e: &mut Estado) {
            self.anotar(e, "agua");
        }
    }
    impl RegrasInimigos for Gravador {
        fn simular_formigas_leao(&mut self, e: &mut Estado) {
            self.anotar(e, "formigas_leao");
        }
        fn mover_aranha(&mut self, e: &mut Estado) {
            self.anotar(e, "aranha");
        }
        fn simular_lagarta(&mut self, e: &mut Estado) {
            self.anotar(e, "lagarta");
        }
    }
    impl RegrasSuperficie for Gravador {
        fn simular_superficie(&mut self, e: &mut Estado) {
            self.anotar(e, "superficie");
        }
        fn adicionar_comida(&mut self, e: &mut Estado, quantidade: i32, som: bool) {
            assert_eq!((quantidade, som), (150, true));
            self.anotar(e, "comida");
        }
        fn escolher_estrategia(&mut self, e: &mut Estado) {
            self.anotar(e, "estrategia");
        }
        fn criar_lider_vermelha(&mut self, e: &mut Estado) {
            self.anotar(e, "lider");
        }
    }
    impl RegrasNinho for Gravador {
        fn simular_ninho(&mut self, e: &mut Estado, cor: Cor) {
            self.anotar(e, if cor == Cor::Preta { "ninho_b" } else { "ninho_r" });
        }
    }
    impl RegrasAmarela for Gravador {
        fn simular_amarela(&mut self, e: &mut Estado) {
            self.anotar(e, "amarela");
        }
        fn mover_amarela(&mut self, e: &mut Estado) {
            self.anotar(e, "mover_amarela");
        }
    }
    impl RegrasConselhos for Gravador {
        fn dar_conselhos(&mut self, e: &mut Estado) {
            self.anotar(e, "conselhos");
        }
    }

    fn rodar(e: &mut Estado, g: &mut Gravador, n: usize) {
        for _ in 0..n {
            tick(e, g);
        }
    }

    #[test]
    fn tick_vazio_incrementa_contadores() {
        let mut e = Estado::vazio();
        let mut r = RegrasOriginais::default();
        tick(&mut e, &mut r);
        assert_eq!((e.partida.ciclo, e.partida.tempo), (1, 1));
        assert!(e.precisa_salvar);
        e.partida.ciclo = 0x1000;
        e.partida.tempo = u32::MAX;
        tick(&mut e, &mut r);
        assert_eq!((e.partida.ciclo, e.partida.tempo), (0, 0), "ciclo volta a 0 depois de 0x1000");
    }

    #[test]
    fn periodicidades() {
        let mut e = Estado::vazio();
        e.partida.tipo_jogo = 1;
        let mut g = Gravador::default();
        rodar(&mut e, &mut g, 256);
        for todo_tick in [
            "quintal",
            "agua",
            "formigas_leao",
            "aranha",
            "estrategia",
            "superficie",
            "ninho_b",
            "ninho_r",
            "amarela",
            "lider",
            "mover_amarela",
        ] {
            assert_eq!(g.ciclos(todo_tick), (1..=256).collect::<Vec<i16>>(), "{todo_tick}");
        }
        assert_eq!(g.ciclos("lagarta"), (1..=256).filter(|c| c % 2 == 1).collect::<Vec<i16>>());
        // Conselhos: todo tick e mais um dentro de _FullCount nos ticks ≡ 0 e 64 (mod 128).
        let mut esperado: Vec<i16> = (1..=256).collect();
        esperado.extend([64, 128, 192, 256]);
        esperado.sort();
        assert_eq!(g.ciclos("conselhos"), esperado);
        // Sem comida abaixo de _NextFeed, nada de _AddFood.
        assert!(g.ciclos("comida").is_empty());
        // Saúde: −1 a cada 64 ticks.
        assert_eq!((e.preta.saude, e.vermelha.saude), (0, 0));
        // Histórico: 4 amostras (ticks 64, 128, 192, 256).
        assert_eq!(e.historico.contagem, 4);
    }

    #[test]
    fn ordem_dentro_do_tick() {
        let mut e = Estado::vazio();
        e.partida.tipo_jogo = 1;
        e.partida.ciclo = 127;
        e.ambiente.proxima_comida = 10;
        let mut g = Gravador::default();
        rodar(&mut e, &mut g, 2);
        assert_eq!(
            g.nomes_no_ciclo(128),
            [
                "comida",
                "conselhos",
                "quintal",
                "agua",
                "formigas_leao",
                "aranha",
                "estrategia",
                "superficie",
                "ninho_b",
                "ninho_r",
                "amarela",
                "lider",
                "mover_amarela",
                "conselhos"
            ]
        );
        assert_eq!(
            g.nomes_no_ciclo(129),
            [
                "quintal",
                "agua",
                "formigas_leao",
                "aranha",
                "lagarta",
                "estrategia",
                "superficie",
                "ninho_b",
                "ninho_r",
                "amarela",
                "lider",
                "mover_amarela",
                "conselhos"
            ]
        );
    }

    #[test]
    fn alimentacao_sorteia_a_proxima_comida() {
        let mut e = Estado::vazio();
        e.partida.ciclo = 63;
        e.preta.saude = 50;
        e.vermelha.saude = 0;
        e.ambiente.proxima_comida = 10;
        e.s.definir_semente(0x1234);
        let mut s = e.s;
        let mut g = Gravador::default();
        tick(&mut e, &mut g);
        assert_eq!(g.ciclos("comida"), vec![64]);
        assert_eq!(e.ambiente.proxima_comida, s.rand1(50) as i16 + 1);
        assert_eq!((e.preta.saude, e.vermelha.saude), (49, 0));

        // Experimental: sem comida; _AlwaysHealthy protege só a preta.
        let mut e = Estado::vazio();
        e.partida.tipo_jogo = 3;
        e.partida.sempre_saudavel = 1;
        e.partida.ciclo = 63;
        e.preta.saude = 50;
        e.vermelha.saude = 50;
        e.ambiente.proxima_comida = 10;
        let mut g = Gravador::default();
        tick(&mut e, &mut g);
        assert!(g.ciclos("comida").is_empty());
        assert_eq!((e.preta.saude, e.vermelha.saude), (50, 49));
    }

    #[test]
    fn compactacao_por_fase() {
        let mut e = Estado::vazio();
        for area in Lista::TODAS {
            for t in [0x12, 0, 0x14] {
                e.lista_mut(area).adicionar(Formiga { x: 1, y: 1, modo: 2, tipo: t, s: 0 });
            }
        }
        let mut g = Gravador::default();
        let indices = |e: &Estado| Lista::TODAS.map(|l| e.lista(l).indice);
        e.partida.ciclo = 0;
        rodar(&mut e, &mut g, 31);
        assert_eq!(indices(&e), [3, 3, 3]);
        rodar(&mut e, &mut g, 1); // 32: lista B
        assert_eq!(indices(&e), [3, 2, 3]);
        rodar(&mut e, &mut g, 64); // 96: lista R
        assert_eq!(indices(&e), [3, 2, 2]);
        rodar(&mut e, &mut g, 32); // 128: lista A
        assert_eq!(indices(&e), [2, 2, 2]);
    }

    #[test]
    fn revoada_e_resumo_de_modos() {
        struct Modos;
        impl RegrasQuintal for Modos {}
        impl RegrasInimigos for Modos {}
        impl RegrasSuperficie for Modos {
            fn simular_superficie(&mut self, e: &mut Estado) {
                e.modos_do_tick[0][2] += 3;
                e.modos_do_tick[0][3] += 1;
                e.modos_do_tick[1][19] += 1;
            }
            fn criar_lider_vermelha(&mut self, _e: &mut Estado) {
                panic!("há uma líder no modo 19");
            }
        }
        impl RegrasNinho for Modos {}
        impl RegrasAmarela for Modos {}
        impl RegrasConselhos for Modos {}
        let mut e = Estado::vazio();
        e.preta.revoada = 2;
        e.vermelha.revoada = -1;
        tick(&mut e, &mut Modos);
        assert_eq!((e.preta.revoada, e.vermelha.revoada), (1, -2));
        assert_eq!(e.preta.modos, [4, 0, 0, 0, 0, 0]);
        tick(&mut e, &mut Modos);
        assert_eq!(e.preta.modos, [4, 0, 0, 0, 0, 0], "contagem do tick é zerada antes das formigas");
    }

    #[test]
    fn fim_de_jogo_pede_dialogo() {
        let mut e = Estado::vazio();
        e.partida.fim_de_jogo = 1;
        tick(&mut e, &mut RegrasOriginais::default());
        assert_eq!(e.retirar_eventos(), vec![Evento::DialogoFimDeJogo]);
    }

    #[test]
    fn pausa_e_velocidade() {
        let cfg = ConfigSimulacao::de_json(include_str!("../../../data/simulacao.json")).unwrap();
        assert_eq!(cfg.atrasos_velocidade, vec![6, 2, 0, -1]);
        let mut e = Estado::vazio();
        e.partida.velocidade = 1;
        let mut r = Relogio::default();
        r.contar_mensagem();
        assert!(!r.liberar_tick(&e, &cfg));
        r.contar_mensagem();
        assert!(r.liberar_tick(&e, &cfg));
        assert_eq!(r.atraso, 0);
        e.partida.velocidade = 3;
        assert!(r.liberar_tick(&e, &cfg));
        e.partida.pausado = 1;
        assert!(!r.liberar_tick(&e, &cfg));
        e.amarela.mover = 1;
        e.partida.ferramenta = -1;
        assert!(r.liberar_tick(&e, &cfg), "pausado, mas a amarela anda por ordem");
        e.partida.ferramenta = 10;
        assert!(!r.liberar_tick(&e, &cfg));
    }
}
