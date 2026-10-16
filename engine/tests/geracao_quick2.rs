//! Geração do mundo comparada com o `QUICK2.ANT` (Quick Game salvo pelo original, 78 ciclos depois
//! de gerado). Especificação: docs/spec/regras/geracao-mundo.md, seção "Implementação no port".
//! Pasta do jogo: SIMANT_DIR ou ../SimAnt-T/MAXIS/ANTWIN (banco SHARED, para a tabela de seno).
//! Sem os arquivos, os testes avisam e passam sem verificar nada.

use std::path::PathBuf;

use engine::formats::save::{JogoSalvo, LayoutSave, Lista};
use engine::recursos::Recursos;
use engine::rules::geracao::{Mundo, TabelasGeracao, TipoJogo, PATCH_INICIAL};
use engine::rules::rng::GeradorS;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn tabelas() -> Option<TabelasGeracao> {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
    if !jogo.join("SHARED.NDX").is_file() {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
        return None;
    }
    let r = Recursos::abrir(&jogo, &["SHARED".to_string()], &raiz().join("data"), None).unwrap();
    Some(TabelasGeracao::carregar(&r).unwrap())
}

fn quick2() -> Option<JogoSalvo> {
    let p = raiz().join("tests/referencias/saves/QUICK2.ANT");
    if !p.is_file() {
        eprintln!("AVISO: {} não encontrado; teste ignorado", p.display());
        return None;
    }
    let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
    Some(JogoSalvo::abrir(&layout, &p).unwrap())
}

/// Regera o patch atual do save como o `_RandYard` de um Quick Game: semente `_YMapSeeds[8 * 12 + 11]`,
/// uma colônia de cada cor, patch (11, 8).
fn gerar(save: &JogoSalvo, tab: &TabelasGeracao) -> Mundo {
    let mut m = Mundo::vazio(TipoJogo::Rapido);
    let sementes = save.vetor_i16("YMapSeeds").unwrap();
    for (d, o) in m.sementes.iter_mut().zip(&sementes) {
        *d = *o as u16;
    }
    m.ninho_iniciado = true;
    let (px, py) = PATCH_INICIAL;
    let semente = m.sementes[(py * 12 + px) as usize];
    assert_eq!(semente, 0x3332, "semente do patch (11, 8) no QUICK2.ANT");
    let mut s = GeradorS::new(0);
    m.gerar_patch(&mut s, tab, semente, 1, 1, px, py).unwrap();
    m
}

/// Células do `_MapA` que diferem porque bichos andaram nos 78 ciclos entre a geração e o save:
/// (x, y, valor no save, valor gerado). O valor "do outro lado" é sempre o chão sorteado na geração.
/// - (55, 27) e (37, 10): onde a geração pôs os dois bichos-de-conta (0x71, 0x73); no save, o chão
///   original voltou (`_SowSave`).
/// - (32, 46) e (49, 8): onde os bichos-de-conta estão no save (`_SowX/_SowY` índices 1 e 2).
/// - (0, 0): 0x72 no save. [hipótese] a rotina que move os bichos também processa o índice 0 das
///   tabelas `_Sow*`, que fica zerado, e desenha um bicho em (0, 0).
/// - (3…7, 58): a lagarta (0x69–0x6B), criada depois da geração.
const DIFERENCAS_MAPA_A: [(usize, usize, u8, u8); 10] = [
    (0, 0, 0x72, 0x04),
    (3, 58, 0x69, 0x04),
    (4, 58, 0x69, 0x01),
    (5, 58, 0x69, 0x05),
    (6, 58, 0x6a, 0x06),
    (7, 58, 0x6b, 0x05),
    (32, 46, 0x71, 0x0c),
    (37, 10, 0x0e, 0x73),
    (49, 8, 0x71, 0x09),
    (55, 27, 0x08, 0x71),
];

fn diferencas(salvo: &[u8], gerado: &[u8]) -> Vec<(usize, usize, u8, u8)> {
    assert_eq!(salvo.len(), gerado.len());
    (0..salvo.len()).filter(|&i| salvo[i] != gerado[i]).map(|i| (i / 64, i % 64, salvo[i], gerado[i])).collect()
}

#[test]
fn quick2_mapas_byte_a_byte() {
    let (Some(tab), Some(save)) = (tabelas(), quick2()) else { return };
    let m = gerar(&save, &tab);
    assert_eq!(diferencas(save.bloco("MapB").unwrap(), &m.mapa_b), vec![], "_MapB");
    assert_eq!(diferencas(save.bloco("MapR").unwrap(), &m.mapa_r), vec![], "_MapR");
    assert_eq!(diferencas(save.bloco("MapA").unwrap(), &m.mapa_a), DIFERENCAS_MAPA_A.to_vec(), "_MapA");
    // Todas as outras 8.182 células do _MapA são iguais (chão, pedras, objetos, buracos, formigas-leão, comida).
}

#[test]
fn quick2_mapas_de_saida_e_buracos() {
    let (Some(tab), Some(save)) = (tabelas(), quick2()) else { return };
    let m = gerar(&save, &tab);
    assert_eq!(diferencas(save.bloco("ExitMapR").unwrap(), &m.saida_r), vec![], "_ExitMapR");
    // (36, 9) é onde a rainha preta foi criada; no save ela já se virou para (34, 9) e a célula tem
    // 244. [hipótese] a simulação recalcula a saída onde a rainha anda; a geração deixa 0 ali porque,
    // quando a célula é escavada, nenhuma vizinha ainda foi.
    assert_eq!(diferencas(save.bloco("ExitMapB").unwrap(), &m.saida_b), vec![(36, 9, 244, 0)], "_ExitMapB");
    assert_eq!(save.bloco("HoleMapB").unwrap(), &m.buracos_b[..]);
    assert_eq!(save.bloco("HoleMapR").unwrap(), &m.buracos_r[..]);
    assert_eq!(m.buracos_b[32], 3);
    assert_eq!(m.buracos_r[32], 118);
    // _LastNewHole? guarda (x, coluna); _LastNewExit? guarda (coluna, 0).
    assert_eq!(save.par_i16("LastNewHoleB").unwrap(), m.ultimo_buraco_b);
    assert_eq!(save.par_i16("LastNewHoleR").unwrap(), m.ultimo_buraco_r);
    assert_eq!(save.par_i16("LastNewExitB").unwrap(), (32, 0));
    assert_eq!(save.par_i16("LastNewExitR").unwrap(), (32, 0));
}

#[test]
fn quick2_escalares_e_listas() {
    let (Some(tab), Some(save)) = (tabelas(), quick2()) else { return };
    let m = gerar(&save, &tab);
    for (cor, e) in [("B", m.escavacao_b), ("R", m.escavacao_r)] {
        assert_eq!(save.i32(&format!("TileTotX{cor}")).unwrap(), e.tot_x, "TileTotX{cor}");
        assert_eq!(save.i32(&format!("TileTotY{cor}")).unwrap(), e.tot_y, "TileTotY{cor}");
        assert_eq!(save.i16(&format!("TilesDug{cor}")).unwrap(), e.cavados, "TilesDug{cor}");
        assert_eq!(save.i16(&format!("TileMassX{cor}")).unwrap(), e.massa_x, "TileMassX{cor}");
        assert_eq!(save.i16(&format!("TileMassY{cor}")).unwrap(), e.massa_y, "TileMassY{cor}");
    }
    assert_eq!(save.i16("WaterTable").unwrap(), m.lencol);
    assert_eq!(save.i16("FoodA").unwrap(), m.comida_a);
    assert_eq!(m.comida_a, 117);
    assert_eq!(save.par_i16("LastFoodDrop").unwrap(), m.ultima_comida);
    assert_eq!(save.par_i16("LastRedEgg").unwrap(), m.ultimo_ovo_vermelho);
    assert_eq!((save.i16("RebornX").unwrap(), save.i16("RebornY").unwrap()), m.renascer);
    assert_eq!(save.i16("InitialLions").unwrap(), m.formigas_leao_iniciais);
    assert_eq!(save.i16("LionIndex").unwrap() as usize, m.formigas_leao.len());
    let lx = save.bloco("LionListX").unwrap();
    let ly = save.bloco("LionListY").unwrap();
    for (k, &(x, y)) in m.formigas_leao.iter().enumerate() {
        assert_eq!((lx[k], ly[k]), (x, y), "formiga-leão {k}");
    }
    assert_eq!(save.i16("BlkQueens").unwrap(), m.rainhas_pretas);
    assert_eq!(save.i16("RedQueens").unwrap(), m.rainhas_vermelhas);
    // Formiga amarela: operária no ninho preto, na câmara da rainha.
    assert_eq!(save.i16("MePlane").unwrap(), m.amarela.plano);
    assert_eq!((save.i16("MeLocX").unwrap(), save.i16("MeLocY").unwrap()), (m.amarela.x, m.amarela.y));
    assert_eq!(save.i16("MeType").unwrap(), m.amarela.tipo as i16);
    assert_eq!(save.i16("MeDir").unwrap(), m.amarela.dir as i16);
    // Lista A: as formigas da superfície (nenhuma nasceu nem morreu nesses 78 ciclos).
    assert_eq!(save.i16("ListIndexA").unwrap() as usize, m.lista_a.len());
    // Lista R: a rainha vermelha e sua segunda metade são as duas primeiras entradas do save.
    let r = save.formigas(Lista::R).unwrap();
    assert_eq!(&r[..2], &m.lista_r[..]);
    // Lista B: a rainha preta já se virou (T 0x66/0x6E, cabeça em (34, 9)); a segunda metade continua
    // em (35, 9). A geração a cria virada para a direita (0x62/0x6A, cabeça em (36, 9)).
    let b = save.formigas(Lista::B).unwrap();
    assert_eq!((b[0].x, b[0].y, b[0].tipo, b[1].x, b[1].y, b[1].tipo), (34, 9, 0x66, 35, 9, 0x6E));
    let g = &m.lista_b;
    assert_eq!((g[0].x, g[0].y, g[0].tipo, g[1].x, g[1].y, g[1].tipo), (36, 9, 0x62, 35, 9, 0x6A));
    assert_eq!(m.ultimo_ovo_preto, (34, 9));
}
