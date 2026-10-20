//! Peças comuns às regras do tick e à geração do mundo: rotinas que o original tem **uma vez só** e que
//! várias partes chamam (`_DigTileB/R`, `_SmoothEdges*`, `_FixExitMap*`, `_MakeNewHole*`, `_FindInAList`,
//! `_DeadAntHere`, `_GetWinner`, `_GetNewMode*`, `_KillSpider`...), mais os testes de tipo e de tile.
//!
//! Antes desta unificação cada arquivo de `rules/` tinha a sua cópia (ver docs/decisoes.md,
//! "Módulo comum das regras"). A especificação de cada rotina continua no documento do assunto que a
//! descreve primeiro; o comentário de cada função cita a seção.
//!
//! Organização:
//! - **Tipo T** (`grupo`, cor, direção) e **valores de tile** (terra, comida, sólido, amarela…).
//! - **Tabelas comuns** (`data/comum.json`, API de mods 0.20): `_Dx8`, `_TurnTab`, as tabelas de luta, de
//!   modos, da borda do buraco... que o original tem uma vez só. Cada parte lê as que usa por
//!   [`completar_com_comuns`] ao carregar o seu `data/<parte>.json`.
//! - **Direções**: as traits [`Direcoes`] (`_Dx8/_Dy8`) e [`TabelaDeGiro`] (`_TurnTab`), implementadas
//!   pelas tabelas de cada parte (preenchidas a partir de `comum.json`).
//! - **Planos** (`_MePlane`: 0/1 superfície, 2 ninho preto, 3 vermelho): `_GetMap`, `_GetLife`,
//!   `_IsClearTile`, `_IsNotObstacle`, `_IsThisPebble`, `_GetBestDir`.
//! - **Listas**: `_FindInAList`, `_FindAntIndex`/`_FindInBList`/`_FindInRList`.
//! - **Superfície**: `_DeadAntHere`, `_PickupFoodA`, `_DropFoodA`, `_Recruit`, `_UnRecruit`.
//! - **Luta e modos**: `_GetWinner`, `_GetNewMode`/`B`/`R`, `_KillSpider`.
//! - **Ninhos**: comida (`_EatFoodB/R`, `_StealFoodB/R`), `_DecEatB/R`, `_PlaceEggB/R`, `_ExitHole`.
//! - **Escavação** sobre a trait [`Terreno`], implementada pelo [`Estado`] (tick) e pelo `Mundo` da
//!   geração (`geracao.rs`): `_DigTileB/R`, `_DigTileThemB/R`, `_SmoothEdgesB/R`, `_FixExitMapB/R`,
//!   `_IsClear3x3`, `_MakeNewHoleB/R`.

use super::estado::{Colonia, Estado, CAPACIDADE_A, CAPACIDADE_NINHO};
use super::geracao::Cor;
use super::rng::GeradorS;
use crate::formats::save::{Formiga, Lista};

// ---------------------------------------------------------------------------------------------
// Valores de tile e de mapa de vida (tiles-valores.md)
// ---------------------------------------------------------------------------------------------

/// Marca da formiga amarela no mapa de vida, e da segunda célula da amarela rainha.
pub const VIDA_AMARELA: u8 = 0xFF;
pub const VIDA_AMARELA_2: u8 = 0xFE;
/// Comida guardada no ninho: uma unidade (0x10) a cheia (0x13).
pub const COMIDA_NINHO_UMA: u8 = 0x10;
pub const COMIDA_NINHO_CHEIA: u8 = 0x13;
/// Passagem entre os dois ninhos e abertura do ninho (linha 0).
pub const PASSAGEM_ENTRE_NINHOS: u8 = 0x14;
pub const ABERTURA_NINHO: u8 = 0x18;
pub const TUNEL_INUNDADO: u8 = 0x4E;
/// Buraco do ninho na superfície do quintal; também o limite das células onde a formiga sai.
pub const BURACO_QUINTAL: u8 = 0x50;
/// Mapa de saída nas linhas 0 e 1 do ninho.
pub const SAIDA_ABERTA: u8 = 0xFF;
pub const SAIDA_FECHADA: u8 = 0xFE;
/// Lado do ninho (64 × 64) e largura da superfície (128 × 64).
pub const LADO_NINHO: i32 = 64;
pub const LARGURA_SUPERFICIE: i32 = 128;

// ---------------------------------------------------------------------------------------------
// Tipo T da formiga
// ---------------------------------------------------------------------------------------------

/// Grupo (casta + carga) de um T: bits 3..6.
pub fn grupo(t: u8) -> u8 {
    (t & 0x78) >> 3
}

/// T vermelho (bit 7).
pub fn e_vermelha(t: u8) -> bool {
    t & 0x80 != 0
}

/// Cor de um T.
pub fn cor_do_tipo(t: u8) -> Cor {
    if e_vermelha(t) {
        Cor::Vermelha
    } else {
        Cor::Preta
    }
}

/// Operária ou soldado (grupos 2 e 6), os únicos que sorteiam modo.
pub fn operaria_ou_soldado(g: u8) -> bool {
    g == 2 || g == 6
}

/// Lista do ninho de uma cor.
pub fn lista_do_ninho(cor: Cor) -> Lista {
    match cor {
        Cor::Preta => Lista::B,
        Cor::Vermelha => Lista::R,
    }
}

/// `_MePlane` do ninho: 2 preto, 3 vermelho.
pub fn plano_do_ninho(cor: Cor) -> i32 {
    match cor {
        Cor::Preta => 2,
        Cor::Vermelha => 3,
    }
}

/// Cor do ninho de um plano ≥ 2 (2 preto, os demais vermelho, como o original).
pub fn cor_do_plano(plano: i32) -> Cor {
    if plano == 2 {
        Cor::Preta
    } else {
        Cor::Vermelha
    }
}

// ---------------------------------------------------------------------------------------------
// Testes de valor de tile
// ---------------------------------------------------------------------------------------------

/// `_IsYellowAnt`: a marca da amarela (0xFF) ou da cauda da amarela rainha (0xFE).
pub fn e_amarela(v: u8) -> bool {
    v == VIDA_AMARELA || v == VIDA_AMARELA_2
}

/// `_IsItDirt`: terra 0x20–0x2E.
pub fn e_terra(v: u8) -> bool {
    (0x20..=0x2E).contains(&v)
}

/// Capim da linha 0 do ninho (0x1C–0x1F): escavável pela amarela.
pub fn e_capim(v: u8) -> bool {
    (0x1C..=0x1F).contains(&v)
}

/// Sólido para o alisamento: ≥ 0x20 e fora de 0x30–0x4E.
pub fn e_solido(v: u8) -> bool {
    v >= 0x20 && !(0x30..=0x4E).contains(&v)
}

/// Comida guardada no ninho: 0x10–0x13.
pub fn e_comida_ninho(v: u8) -> bool {
    (COMIDA_NINHO_UMA..=COMIDA_NINHO_CHEIA).contains(&v)
}

/// `_IsItFood` (`1028:2d1a`): comida na superfície; quintal 0x48–0x4B, casa 0x18–0x27.
pub fn e_comida_superficie(casa: bool, v: u8) -> bool {
    if casa {
        (0x18..=0x27).contains(&v)
    } else {
        (0x48..=0x4B).contains(&v)
    }
}

/// Uma unidade de comida a mais numa célula da superfície da **casa**, com a codificação de `_AddFood`
/// e `_DropFoodA` na casa (superficie.md 8): o chão de carpete/ladrilho 0–3 vira `(v + 6) × 4` (uma
/// unidade, o chão guardado nos bits altos) e a comida 0x18–0x27 com menos de quatro unidades ganha
/// uma. `None` se a célula não recebe comida. (Os valores do quintal, 0x48–0x4B, são peças do botão do
/// fogão no conjunto de tiles da casa.)
pub fn comida_na_casa_mais_uma(v: u8) -> Option<u8> {
    match v {
        0..=3 => Some((v + 6) * 4),
        0x18..=0x27 if v & 3 < 3 => Some(v + 1),
        _ => None,
    }
}

/// O chão que fica sob a comida 0x18–0x27 da superfície da casa (a última unidade de `_PickupFoodA`
/// volta a ele); `None` se a célula não tem comida.
pub fn chao_sob_comida_na_casa(v: u8) -> Option<u8> {
    (0x18..=0x27).contains(&v).then(|| ((v & 0xFC) - 0x18) >> 2)
}

/// `_IsItHole(x, y)` (superficie.md 2.1): dentro da superfície e buraco do conjunto de tiles atual
/// (quintal 0x50; casa 0x80–0x8F).
pub fn e_buraco_superficie(e: &Estado, x: i32, y: i32) -> bool {
    let Some(v) = e.superficie.mapa.em(x, y) else { return false };
    if em_casa(e) {
        (0x80..0x90).contains(&v)
    } else {
        v == BURACO_QUINTAL
    }
}

/// Superfície da casa (`_TERRAINset` ≠ 0)?
pub fn em_casa(e: &Estado) -> bool {
    e.ambiente.terreno != 0
}

// ---------------------------------------------------------------------------------------------
// Tabelas comuns (data/comum.json)
// ---------------------------------------------------------------------------------------------

/// Arquivo das tabelas que o original tem uma vez só e várias partes usam (gerado por
/// `tools/extrair_comum.py`, com o endereço de cada uma em `_enderecos`).
pub const ARQUIVO_COMUNS: &str = "comum.json";

/// Uma tabela comum usada por uma parte: (chave no `data/<parte>.json` — e no struct da parte —,
/// chave em `comum.json`). As duas só diferem onde a parte já tinha nome próprio (`descarregar_grupo`
/// da amarela, `chance_vitoria` do ninho).
pub type UsoComum = (&'static str, &'static str);

/// `comum.json` embutido na compilação (sem mods), para os `padrao()` e os `de_json` das partes.
pub fn comuns_embutidas() -> &'static serde_json::Value {
    static C: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    C.get_or_init(|| serde_json::from_str(include_str!("../../../data/comum.json")).expect("data/comum.json inválido"))
}

/// Põe no JSON de uma parte as tabelas comuns que ela usa, tiradas de `comuns`.
///
/// Compatibilidade (API de mods 0.20): até a 0.19 cada `data/<parte>.json` trazia a sua cópia. Se o
/// arquivo da parte (de um mod antigo) ainda trouxer a chave, ela **vale só para aquela parte**, como
/// antes, e o console avisa; a tabela de `comum.json` continua valendo para as outras partes.
pub fn completar_com_comuns(
    arquivo: &str,
    parte: &mut serde_json::Value,
    comuns: &serde_json::Value,
    usos: &[UsoComum],
) -> Result<(), String> {
    let obj = parte.as_object_mut().ok_or_else(|| format!("{arquivo}: esperado um objeto JSON"))?;
    for &(chave, chave_comum) in usos {
        if obj.contains_key(chave) {
            eprintln!(
                "aviso: {arquivo} traz \"{chave}\", que desde a API de mods 0.20 fica em {ARQUIVO_COMUNS} \
                 (\"{chave_comum}\"); a cópia vale só para esta parte"
            );
            continue;
        }
        let v = comuns
            .get(chave_comum)
            .ok_or_else(|| format!("{ARQUIVO_COMUNS}: falta a tabela \"{chave_comum}\" (usada por {arquivo})"))?;
        obj.insert(chave.to_string(), v.clone());
    }
    Ok(())
}

/// Lê o JSON de uma parte a partir do texto, com as tabelas comuns embutidas.
pub fn parte_do_texto<T: serde::de::DeserializeOwned>(arquivo: &str, texto: &str, usos: &[UsoComum]) -> Result<T, String> {
    let mut v: serde_json::Value = serde_json::from_str(texto).map_err(|e| format!("{arquivo}: {e}"))?;
    completar_com_comuns(arquivo, &mut v, comuns_embutidas(), usos)?;
    serde_json::from_value(v).map_err(|e| format!("{arquivo}: {e}"))
}

/// Lê `data/<arquivo>` e `data/comum.json` pelos [`Recursos`](crate::recursos::Recursos) (com mods).
pub fn parte_dos_recursos<T: serde::de::DeserializeOwned>(
    r: &crate::recursos::Recursos,
    arquivo: &str,
    usos: &[UsoComum],
) -> Result<T, String> {
    let mut v: serde_json::Value = r.dados(arquivo)?;
    let comuns: serde_json::Value = r.dados(ARQUIVO_COMUNS)?;
    completar_com_comuns(arquivo, &mut v, &comuns, usos)?;
    serde_json::from_value(v).map_err(|e| format!("{}: {e}", r.descrever_dados(arquivo)))
}

// ---------------------------------------------------------------------------------------------
// Direções e geometria
// ---------------------------------------------------------------------------------------------

/// `_Dx8/_Dy8`: deslocamento por direção 0–7 (0 = cima, 2 = direita, 4 = baixo, 6 = esquerda). Cada
/// tabela de parte que usa `dx8`/`dy8` (preenchidos a partir de `data/comum.json`) implementa esta trait.
pub trait Direcoes {
    fn deslocamentos(&self) -> (&[i8; 8], &[i8; 8]);

    /// Deslocamento da direção `d & 7`.
    fn d8(&self, d: i32) -> (i32, i32) {
        let (dx, dy) = self.deslocamentos();
        let k = (d & 7) as usize;
        (dx[k] as i32, dy[k] as i32)
    }

    /// Célula vizinha de (x, y) na direção `d`.
    fn vizinha(&self, x: i32, y: i32, d: i32) -> (i32, i32) {
        let (dx, dy) = self.d8(d);
        (x + dx, y + dy)
    }
}

/// Validação comum de `dx8`/`dy8` dos arquivos de dados: cada deslocamento em −1..1.
pub fn validar_direcoes(arquivo: &str, dx: &[i8], dy: &[i8]) -> Result<(), String> {
    if dx.iter().chain(dy).any(|d| !(-1..=1).contains(d)) {
        return Err(format!("{arquivo}: dx8/dy8 (e dx9/dy9) devem estar em −1..1"));
    }
    Ok(())
}

/// `_TurnTab[atual][alvo]`: a direção que gira de `atual` rumo a `alvo` no máximo 45°.
pub trait TabelaDeGiro {
    fn turn_tab(&self) -> &[[u8; 8]; 8];

    fn virar(&self, atual: i32, alvo: i32) -> u8 {
        self.turn_tab()[(atual & 7) as usize][(alvo & 7) as usize]
    }
}

/// `_GetDir`: 1 + direção de (x1, y1) para (x2, y2) pelos sinais de dx e dy; 0 se coincidem.
pub fn get_dir(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    match ((x2 - x1).signum(), (y2 - y1).signum()) {
        (0, 0) => 0,
        (0, -1) => 1,
        (1, -1) => 2,
        (1, 0) => 3,
        (1, 1) => 4,
        (0, _) => 5,
        (-1, 1) => 6,
        (-1, 0) => 7,
        _ => 8,
    }
}

/// `_GetDis`: dx² + dy² em 32 bits (quintal.md, gato). Quem guarda o resultado num `int` de 16 bits
/// trunca (superficie.rs); dentro do mapa o valor cabe em 16 bits.
pub fn get_dis(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    let dx = x2.wrapping_sub(x1);
    let dy = y2.wrapping_sub(y1);
    dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy))
}

/// `_Bounce` (`1030:12ec`) e o mesmo teste no começo de `_GetRandDir`, `_GetNestDir`, `_GetAlarmDir`,
/// `_GetDefendDir` e `_GetRedDefendDir`: numa célula da borda da superfície (128 × 64), um `S1(n)` e uma
/// direção que aponta para dentro (5 opções no meio da borda, 3 nos cantos); fora da borda, `None` sem
/// sorteio (superficie.md 2.5, feromonios.md).
pub fn quicar(s: &mut GeradorS, x: i32, y: i32) -> Option<u8> {
    const ULTIMO_X: i32 = LARGURA_SUPERFICIE - 1;
    const ULTIMO_Y: i32 = LADO_NINHO - 1;
    let (n, primeira) = if x == 0 {
        if y == 0 {
            (3, 2)
        } else if y == ULTIMO_Y {
            (3, 0)
        } else {
            (5, 0)
        }
    } else if y == 0 {
        if x == ULTIMO_X {
            (3, 4)
        } else {
            (5, 2)
        }
    } else if x == ULTIMO_X {
        if y == ULTIMO_Y {
            (3, 6)
        } else {
            (5, 4)
        }
    } else if y == ULTIMO_Y {
        (5, 6)
    } else {
        return None;
    };
    Some(((s.rand1(n) + primeira) & 7) as u8)
}

// ---------------------------------------------------------------------------------------------
// Planos (`_MePlane`): acesso a mapas e testes de célula
// ---------------------------------------------------------------------------------------------

/// Largura do plano: 128 na superfície (planos 0 e 1), 64 nos ninhos.
pub fn largura_do_plano(plano: i32) -> i32 {
    if plano < 2 {
        LARGURA_SUPERFICIE
    } else {
        LADO_NINHO
    }
}

/// Teste de limites usado por quase todas as rotinas de plano do original.
pub fn dentro(plano: i32, x: i32, y: i32) -> bool {
    (0..largura_do_plano(plano)).contains(&x) && (0..LADO_NINHO).contains(&y)
}

/// Lista de um plano: < 2 → A, 2 → B, senão R (como `_GetAntIndex`).
pub fn lista_do_plano(plano: i32) -> Lista {
    if plano < 2 {
        Lista::A
    } else if plano == 2 {
        Lista::B
    } else {
        Lista::R
    }
}

/// `_GetMap(plano, x, y)`: valor do terreno, ou −1 fora do mapa (ou plano desconhecido).
pub fn tile(e: &Estado, plano: i32, x: i32, y: i32) -> i32 {
    if !dentro(plano, x, y) {
        return -1;
    }
    let g = match plano {
        0 | 1 => &e.superficie.mapa,
        2 => &e.ninho_b.mapa,
        3 => &e.ninho_r.mapa,
        _ => return -1,
    };
    g.em(x, y).map_or(-1, i32::from)
}

/// Byte do mapa de vida do plano (0 = vazio), ou `None` fora do mapa.
pub fn vida_bruta(e: &Estado, plano: i32, x: i32, y: i32) -> Option<u8> {
    if !dentro(plano, x, y) {
        return None;
    }
    let g = match plano {
        0 | 1 => &e.superficie.vida,
        2 => &e.ninho_b.vida,
        3 => &e.ninho_r.vida,
        _ => return None,
    };
    g.em(x, y)
}

/// `_GetLife(plano, x, y)`: −1 para célula vazia ou fora do mapa; senão o byte.
pub fn vida(e: &Estado, plano: i32, x: i32, y: i32) -> i32 {
    match vida_bruta(e, plano, x, y) {
        Some(v) if v != 0 => v as i32,
        _ => -1,
    }
}

/// `_IsClearTile(plano, x, y)`: dentro do mapa, sem formiga (a amarela conta como livre) e chão
/// (< 0x10 na superfície, < 8 no ninho).
pub fn celula_livre(e: &Estado, plano: i32, x: i32, y: i32) -> bool {
    let v = tile(e, plano, x, y);
    if v < 0 {
        return false;
    }
    let ocupante = vida(e, plano, x, y);
    if ocupante >= 0 && !e_amarela(ocupante as u8) {
        return false;
    }
    if plano < 2 {
        v < 0x10
    } else {
        v < 8
    }
}

/// `_IsThisPebble(plano, v)`: pedrinha (0x51–0x53 só no plano 1; 0x30–0x31 nos ninhos).
pub fn e_pedrinha(plano: i32, v: i32) -> bool {
    if plano < 2 {
        plano == 1 && (0x51..=0x53).contains(&v)
    } else {
        (0x30..=0x31).contains(&v)
    }
}

/// `_IsNotObstacle(plano, x, y)`: superfície ≤ 0x53 (quintal) ou ≤ 0x5F (casa); ninho < 0x19 ou
/// pedrinha.
pub fn nao_e_obstaculo(e: &Estado, plano: i32, x: i32, y: i32) -> bool {
    let v = tile(e, plano, x, y);
    if v < 0 {
        return false;
    }
    if plano < 2 {
        // O teste de 0x50 do quintal nunca é alcançado no original (tiles-valores.md).
        return if em_casa(e) { v <= 0x5F } else { v <= 0x53 };
    }
    v < 0x19 || (0x30..=0x31).contains(&v)
}

/// `_TileCanBeMovedOn` no plano da superfície (`1020:9342`, formiga-amarela.md §5.4): dentro do mapa e
/// tile ≤ 0x53 (quintal) ou ≤ 0x90 (casa). Nos ninhos a rotina tem as regras das linhas 0–1 (só a
/// amarela anda por lá; ver `amarela.rs`).
pub fn pode_pisar_superficie(e: &Estado, x: i32, y: i32) -> bool {
    match e.superficie.mapa.em(x, y) {
        None => false,
        Some(v) => v <= if em_casa(e) { 0x90 } else { 0x53 },
    }
}

/// `_GetBestDir(plano, x, y, ax, ay)` (`1028:405e`; inimigos.md 2.9, ciclo-de-vida.md §5): a vizinha
/// que mais aproxima do alvo, sem obstáculo nem pedrinha, preferindo célula livre (`_GetLife` vazio e
/// `_IsClearTile`); −1 se já está no alvo; −2 se nenhuma vizinha aproxima. Sem sorteio.
pub fn melhor_direcao<D: Direcoes + ?Sized>(e: &Estado, dirs: &D, plano: i32, x: i32, y: i32, ax: i32, ay: i32) -> i32 {
    let mut dist = get_dis(x, y, ax, ay);
    if dist <= 0 {
        return -1;
    }
    let mut livre = -1;
    let mut bloqueada = -2;
    for k in 0..8 {
        let (nx, ny) = dirs.vizinha(x, y, k);
        if !nao_e_obstaculo(e, plano, nx, ny) || e_pedrinha(plano, tile(e, plano, nx, ny)) {
            continue;
        }
        let d2 = get_dis(nx, ny, ax, ay);
        if d2 < dist {
            dist = d2;
            if vida(e, plano, nx, ny) < 1 && celula_livre(e, plano, nx, ny) {
                livre = k;
            } else {
                bloqueada = k;
            }
        }
    }
    if livre >= 0 {
        livre
    } else {
        bloqueada
    }
}

// ---------------------------------------------------------------------------------------------
// Listas
// ---------------------------------------------------------------------------------------------

/// `_FindInAList(x, y)` (`1020:2c42`): de trás para frente, a primeira vaga em uso com essa posição e
/// T ≠ 0.
pub fn achar_na_lista_a(e: &Estado, x: i32, y: i32) -> Option<usize> {
    let l = &e.superficie.lista;
    (0..l.usadas()).rev().find(|&j| {
        let f = &l.vagas[j];
        f.tipo != 0 && f.x as i32 == x && f.y as i32 == y
    })
}

/// `_FindAntIndex(plano, x, y, T)` e `_FindInBList/RList(x, y, T)` (`1020:2c86`/`2cce`): de trás para
/// frente, a primeira vaga em uso com essa posição e esse T exato.
pub fn achar_formiga(e: &Estado, lista: Lista, x: i32, y: i32, t: u8) -> Option<usize> {
    let l = e.lista(lista);
    (0..l.usadas()).rev().find(|&j| {
        let f = &l.vagas[j];
        f.x as i32 == x && f.y as i32 == y && f.tipo == t
    })
}

// ---------------------------------------------------------------------------------------------
// Superfície: cadáver, comida, recrutamento
// ---------------------------------------------------------------------------------------------

/// `_DeadAntHere(x, y, cor)` (`1028:28c0`, superficie.md 6.4): avança o anel de 100 cadáveres (o mais
/// antigo some do `_MapA`), põe o cadáver da cor em (x, y) e apaga a vida da célula.
pub fn cadaver_aqui(e: &mut Estado, x: i32, y: i32, vermelha: bool) {
    let mut p = e.partida.cadaver_ponteiro + 1;
    if p > 99 {
        p = 0;
    }
    e.partida.cadaver_ponteiro = p;
    let p = p.clamp(0, 99) as usize;
    let (ox, oy) = (e.partida.cadaveres_x[p] as i32, e.partida.cadaveres_y[p] as i32);
    let casa = em_casa(e);
    // Fora do mapa o tile conta como 0xFF (nenhum dos casos abaixo).
    let velho = e.superficie.mapa.em(ox, oy).unwrap_or(0xFF);
    if !casa {
        if (0x10..0x18).contains(&velho) {
            let v = e.s.mascara(16) as u8;
            e.superficie.mapa.definir(ox, oy, v);
        }
    } else if (8..0x18).contains(&velho) {
        e.superficie.mapa.definir(ox, oy, (velho - 8) >> 2);
    }
    e.partida.cadaveres_x[p] = x as u8;
    e.partida.cadaveres_y[p] = y as u8;
    let v = e.superficie.mapa.em(x, y).unwrap_or(0xFF);
    if !casa {
        if v <= 0x17 {
            let base = if vermelha { 0x14 } else { 0x10 };
            let novo = base + e.s.mascara(4) as u8;
            e.superficie.mapa.definir(x, y, novo);
        }
    } else if v <= 3 {
        let r = e.s.rand1(2) as u8;
        let novo = if vermelha { v * 4 + 10 + r } else { (v + 2) * 4 + r };
        e.superficie.mapa.definir(x, y, novo);
    }
    e.superficie.vida.definir(x, y, 0);
}

/// `_PickupFoodA(x, y)` (`1020:0d18`, superficie.md 8.1): tira uma unidade de comida da superfície.
pub fn pegar_comida_superficie(e: &mut Estado, x: i32, y: i32) {
    let Some(v) = e.superficie.mapa.em(x, y) else { return };
    let novo = if !em_casa(e) {
        if v == 0x48 {
            e.s.mascara(16) as u8
        } else {
            v.wrapping_sub(1)
        }
    } else if v.is_multiple_of(4) {
        v.wrapping_sub(0x18) >> 2
    } else {
        v.wrapping_sub(1)
    };
    e.superficie.mapa.definir(x, y, novo);
    if e.ambiente.comida_superficie > 0 {
        e.ambiente.comida_superficie -= 1;
    }
}

/// `_DropFoodA(x, y)` (`1020:0d86`, superficie.md 8.2): uma unidade de comida na célula; `false` se
/// não coube.
///
/// Na casa, um tile 4–7 ou 0x27–0x3F faz a comida **cair** (superficie.md 8.2): a partir da célula,
/// anda o passo `e.passo_queda` (somado sem sinal) até achar chão livre (tile < 4), onde a comida
/// fica, ou sair do mapa (a comida some); devolve sucesso nos dois casos. A aranha morta ignora o
/// resultado; a amarela o usa para saber se largou a carga.
pub fn largar_comida_superficie(e: &mut Estado, x: i32, y: i32) -> bool {
    let Some(v) = e.superficie.mapa.em(x, y) else { return false };
    let novo = if em_casa(e) {
        match v {
            0..=3 => (v + 6) * 4,
            8..=0x17 => (((v - 8) >> 2) + 6) * 4,
            0x18..=0x26 => v + 1,
            4..=7 | 0x27..=0x3F => {
                comida_cai(e, x, y);
                return true;
            }
            _ => return false,
        }
    } else {
        if v > 0x4A {
            return false;
        }
        if v < 0x48 {
            0x48
        } else {
            v + 1
        }
    };
    e.superficie.mapa.definir(x, y, novo);
    e.ambiente.comida_superficie += 1;
    true
}

/// A queda de `_DropFoodA`: célula a célula no passo de `_DROPdir`, a primeira com tile < 4 recebe a
/// comida (`(v + 6) × 4`, `_FoodA += 1`); para no mapa ou fora dele.
fn comida_cai(e: &mut Estado, mut x: i32, mut y: i32) {
    let (dx, dy) = (i32::from(e.passo_queda.0), i32::from(e.passo_queda.1));
    loop {
        if let Some(v) = e.superficie.mapa.em(x, y).filter(|&v| v < 4) {
            e.superficie.mapa.definir(x, y, (v + 6) * 4);
            e.ambiente.comida_superficie += 1;
            return;
        }
        x += dx;
        y += dy;
        if !(0..=0x7F).contains(&x) || !(0..=0x3F).contains(&y) {
            return;
        }
    }
}

/// Modo Defense (formigas.md §5).
const M_DEFESA: u8 = 6;

/// `_Recruit(n)` (`1030:06d2`, superficie.md 9.4): pretas de grupo 2/6 com M ≠ 6 → M = 6, S = 0; lista
/// A de trás para frente, depois lista B, até `n`.
pub fn recrutar(e: &mut Estado, mut n: i32) {
    for area in [Lista::A, Lista::B] {
        let l = e.lista_mut(area);
        for j in (0..l.usadas()).rev() {
            if n < 1 {
                return;
            }
            let f = &mut l.vagas[j];
            if f.tipo != 0 && !e_vermelha(f.tipo) && operaria_ou_soldado(grupo(f.tipo)) && f.modo != M_DEFESA {
                f.modo = M_DEFESA;
                f.s = 0;
                n -= 1;
            }
        }
    }
}

/// `_UnRecruit(p)` (`1030:078a`, superficie.md 9.4): quota = metade das pretas em Defense do último
/// tick (p = 0) ou todas + 100 (p ≠ 0). Listas A e B: M 6 → 0; lista R (invasoras pretas): M 6 → 7.
pub fn desrecrutar(e: &mut Estado, p: i32) {
    let em_defesa = e.modos_do_tick[0][M_DEFESA as usize] as i32;
    let mut quota = if p == 0 { em_defesa / 2 } else { em_defesa + 100 };
    for (area, novo) in [(Lista::A, 0), (Lista::B, 0), (Lista::R, 7)] {
        let l = e.lista_mut(area);
        for j in (0..l.usadas()).rev() {
            if quota < 1 {
                return;
            }
            let f = &mut l.vagas[j];
            if f.tipo != 0 && !e_vermelha(f.tipo) && f.modo == M_DEFESA {
                f.modo = novo;
                quota -= 1;
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Luta, modos e aranha
// ---------------------------------------------------------------------------------------------

/// Tabelas de `_GetWinner`: classe de luta por grupo (`10:22c6`) e chance em 10 de a formiga da célula
/// vencer, `[classe(a)][classe(b)]` (`10:22d6`).
#[derive(Clone, Copy, Debug)]
pub struct TabelaLuta<'a> {
    pub classe: &'a [u8; 16],
    pub chance: &'a [[u8; 4]; 4],
}

/// `_GetWinner(Ta, Tb)` (`1028:26f4`, superficie.md 6.2, formigas.md §6): `a` é a formiga da célula,
/// `b` a que chega. Sorteia `RR(10)` (gerador **R**) contra a matriz de classes e conta a perdedora em
/// `_BlackLost/_RedLost` e `_BAntsKilled/_RAntsKilled` pela cor da vencedora. Com `_Invincible` = 1 não
/// sorteia: conta uma vermelha morta e a preta vence. Devolve o T completo da vencedora.
pub fn vencedor_da_luta(e: &mut Estado, t: TabelaLuta, ta: u8, tb: u8) -> u8 {
    if e.partida.invencivel == 1 {
        e.vermelha.mortas_em_luta = e.vermelha.mortas_em_luta.wrapping_add(1);
        return if e_vermelha(ta) { tb } else { ta };
    }
    let classe = |v: u8| t.classe[grupo(v) as usize] as usize;
    let p = t.chance[classe(ta)][classe(tb)] as i32;
    let v = if e.r.rrand(10) < p { ta } else { tb };
    let perdedora = if e_vermelha(v) { &mut e.preta } else { &mut e.vermelha };
    perdedora.perdas = perdedora.perdas.wrapping_add(1);
    perdedora.mortas_em_luta = perdedora.mortas_em_luta.wrapping_add(1);
    v
}

/// Tabelas de `_GetNewMode*` (formigas.md, "Escolha de modo"): `_ModeTabWB`/`_ModeTabSB`
/// `[modo estratégico][Sk(8)]` e `_CasteModeTabB[grupo]`.
#[derive(Clone, Copy, Debug)]
pub struct TabelaModos<'a> {
    pub operaria: &'a [[u8; 8]],
    pub soldado: &'a [[u8; 8]],
    pub por_grupo: &'a [u8; 16],
}

/// Linha `m` de uma tabela por modo estratégico (`_ModeTab*`, `_CasteTabC`). O original não confere o
/// índice (leria a memória seguinte); fora da tabela o port usa a última linha [divergência só em
/// estado inválido: o modo estratégico é sempre 0..5]. A tabela não pode ser vazia.
pub fn linha_por_modo<T: Copy>(tabela: &[T], m: i16) -> T {
    tabela[(m.max(0) as usize).min(tabela.len().saturating_sub(1))]
}

impl TabelaModos<'_> {
    /// Sorteio da operária (g = 2) ou do soldado (g = 6) com o modo estratégico `m`.
    fn sortear(&self, s: &mut GeradorS, g: u8, m: i16) -> u8 {
        let tabela = if g == 2 { self.operaria } else { self.soldado };
        let k = s.mascara(8) as usize;
        linha_por_modo(tabela, m)[k]
    }

    fn do_grupo(&self, g: u8) -> u8 {
        self.por_grupo[(g & 0xF) as usize]
    }
}

/// `_GetNewModeB(grupo)` (`1030:09d0`): com `_ModeAuto` = 1, operária e soldado sorteiam `Sk(8)` na
/// tabela do `_StrategicModeB`; com 0, recebem `_ModeMe`. Os demais grupos: `_CasteModeTabB`.
pub fn novo_modo_preto(e: &mut Estado, t: TabelaModos, g: u8) -> u8 {
    if operaria_ou_soldado(g) {
        if e.controles.modo_automatico == 1 {
            let m = e.preta.modo_estrategico;
            return t.sortear(&mut e.s, g, m);
        }
        return e.controles.proximo_modo as u8;
    }
    t.do_grupo(g)
}

/// `_GetNewModeR(grupo)` (`1030:0a50`): sempre pelas tabelas, com `_StrategicModeR`.
pub fn novo_modo_vermelho(e: &mut Estado, t: TabelaModos, g: u8) -> u8 {
    if operaria_ou_soldado(g) {
        let m = e.vermelha.modo_estrategico;
        return t.sortear(&mut e.s, g, m);
    }
    t.do_grupo(g)
}

/// `_GetNewMode(grupo, T)` (`1030:0910`): a versão do preto ou do vermelho, pela cor de T.
pub fn novo_modo(e: &mut Estado, t: TabelaModos, g: u8, tipo: u8) -> u8 {
    if e_vermelha(tipo) {
        novo_modo_vermelho(e, t, g)
    } else {
        novo_modo_preto(e, t, g)
    }
}

/// `_KillSpider` (inimigos.md 2.8): agonia de `agonia` ticks, sem mensagem.
pub fn matar_aranha(e: &mut Estado, agonia: i16) {
    let a = &mut e.aranha;
    a.estado = 5;
    a.agonia = agonia;
    a.ciclo_pernas = 0;
}

// ---------------------------------------------------------------------------------------------
// Ninhos: comida, reserva, ovos, saída para a superfície
// ---------------------------------------------------------------------------------------------

/// `_StealFoodB/R` (embutido nas invasoras) e a parte de ninho de `_PickupMyFood`: tira uma unidade de
/// comida da célula (0x10 vira túnel `Sk(8)`) e baixa a comida guardada, sem mexer na saúde.
pub fn tirar_comida_do_ninho(e: &mut Estado, cor: Cor, x: i32, y: i32) {
    let v = e.ninho(cor).mapa.em(x, y).unwrap_or(0);
    let novo = if v == COMIDA_NINHO_UMA { e.s.mascara(8) as u8 } else { v.wrapping_sub(1) };
    e.ninho_mut(cor).mapa.definir(x, y, novo);
    let c = e.colonia_mut(cor);
    if c.comida > 0 {
        c.comida -= 1;
    }
}

/// `_EatFoodB/R` (`1028:4844`/`6bb6`, ninho.md §6.1) sem o teste de faixa: [`tirar_comida_do_ninho`] e
/// soma 5 à reserva; quando a reserva passa de (adultos + soldados) / 16, zera e a saúde sobe 1 (até
/// 100).
pub fn comer_no_ninho(e: &mut Estado, cor: Cor, x: i32, y: i32) {
    tirar_comida_do_ninho(e, cor, x, y);
    let c = e.colonia_mut(cor);
    let limite = c.populacao.wrapping_add(c.castas[2]) >> 4;
    c.reserva_comer = c.reserva_comer.wrapping_add(5);
    if limite < c.reserva_comer {
        c.reserva_comer = 0;
        if c.saude < 100 {
            c.saude += 1;
        }
    }
}

/// `_DecEatB/R` (ciclo-de-vida.md §3, por ovo posto): reserva − 1; abaixo de 0, volta a população / 32
/// e a saúde cai 1 (a preta não cai com `_AlwaysHealthy`).
pub fn gastar_reserva(e: &mut Estado, cor: Cor) {
    let sempre_saudavel = e.partida.sempre_saudavel != 0;
    let c = e.colonia_mut(cor);
    c.reserva_comer = c.reserva_comer.wrapping_sub(1);
    if c.reserva_comer < 0 {
        c.reserva_comer = c.populacao >> 5;
        if c.saude > 0 && !(cor == Cor::Preta && sempre_saudavel) {
            c.saude -= 1;
        }
    }
}

/// `_PlaceEggB/R(x, y, t)` (`1020:1004`/`1068`, ninho.md §7.6): cava a célula (se for terra) e
/// acrescenta o ovo no fim da lista, sem testar ocupação. Nada com a lista cheia ou fora de
/// 0..63 × 1..63.
pub fn por_ovo<D: Direcoes + ?Sized>(e: &mut Estado, dirs: &D, cor: Cor, x: i32, y: i32, t: u8) {
    if e.ninho(cor).lista.indice as i32 >= CAPACIDADE_NINHO as i32 {
        return;
    }
    if !(0..LADO_NINHO).contains(&x) || !(1..LADO_NINHO).contains(&y) {
        return;
    }
    cavar(e, dirs, cor, x, y);
    let f = Formiga { x: x as u8, y: y as u8, modo: 8, tipo: t, s: 0 };
    e.adicionar_formiga(lista_do_ninho(cor), f);
    e.ninho_mut(cor).vida.definir(x, y, t);
}

/// `_ExitHole(buraco_x, coluna, t, m, s)` (`1020:2db6`, ninho.md §4.6): põe a formiga na superfície, na
/// primeira vizinha 0..7 de (buraco_x, coluna) dentro do mapa e com `_MapA` < 0x50. S = s só se M = 6;
/// senão 0, ou 120 se M ∉ {3, 7} e o buraco é do lado da cor (pretas: x ≤ 63; vermelhas: x ≥ 65). Não
/// grava `_LifeA` (fato visto no código). Devolve se achou lugar.
pub fn sair_pelo_buraco<D: Direcoes + ?Sized>(e: &mut Estado, dirs: &D, hx: i32, c: i32, t: u8, m: u8, s: u8) -> bool {
    let lugar = (0..8).map(|k| dirs.vizinha(hx, c, k)).find(|&(ax, ay)| {
        e.superficie.mapa.em(ax, ay).is_some_and(|v| v < BURACO_QUINTAL)
    });
    let Some((ax, ay)) = lugar else { return false };
    let lado = if e_vermelha(t) { hx >= 65 } else { hx <= 63 };
    let s_novo = if m == 6 {
        s
    } else if m != 3 && m != 7 && lado {
        0x78
    } else {
        0
    };
    let l = &mut e.superficie.lista;
    let f = Formiga { x: ax as u8, y: ay as u8, modo: m, tipo: t, s: s_novo };
    if l.usadas() < CAPACIDADE_A {
        l.adicionar(f);
    } else {
        // Lista cheia (ninho.md §4.6): o original grava na vaga 1000, que é a folga de 2 bytes depois
        // de cada vetor de 1000 (fora do save e de toda varredura), e só então compacta as vagas
        // 0..999. A formiga se perde; se a compactação abriu vagas, o contador ainda soma 1 e passa a
        // incluir a vaga seguinte às compactadas com o que estava nela (a compactação não apaga o
        // fim): uma vaga vazia (T = 0) ou a cópia de uma formiga que foi deslizada. Devolve 1 mesmo
        // assim (quem chamou tira a formiga do ninho).
        l.compactar();
        if l.usadas() < CAPACIDADE_A {
            l.indice += 1;
        }
    }
    true
}

// ---------------------------------------------------------------------------------------------
// Escavação (ninho.md §7.3–7.6, tiles-valores.md "Regra de alisamento", geracao-mundo.md §3.A)
// ---------------------------------------------------------------------------------------------

/// Leitura da superfície (`_MapA`, `_LifeA`) para `_IsClearTile`/`_IsClear3x3`. Implementada pelo
/// [`Estado`] e pelo `Mundo` da geração.
pub trait Superficie {
    /// `_MapA` (`None` fora da superfície).
    fn tile_superficie(&self, x: i32, y: i32) -> Option<u8>;
    /// `_LifeA`.
    fn vida_superficie(&self, x: i32, y: i32) -> Option<u8>;
}

/// O que as rotinas de escavação leem e gravam. Implementada pelo [`Estado`] (tick, com o gerador
/// `Estado::s`) e pelo `Mundo` da geração junto com o gerador S dela (`geracao.rs`).
pub trait Terreno: Superficie {
    /// `_MapB/_MapR` (`None` fora do ninho).
    fn tile_ninho(&self, cor: Cor, x: i32, y: i32) -> Option<u8>;
    fn gravar_tile_ninho(&mut self, cor: Cor, x: i32, y: i32, v: u8);
    /// `_ExitMapB/_ExitMapR`.
    fn saida(&self, cor: Cor, x: i32, y: i32) -> Option<u8>;
    fn gravar_saida(&mut self, cor: Cor, x: i32, y: i32, v: u8);
    /// Uma célula escavada nos totais da cor (ver [`somar_escavacao`]).
    fn somar_escavacao(&mut self, cor: Cor, x: i32, y: i32);
    /// Grava no `_MapA` (fora da superfície, nada).
    fn gravar_tile_superficie(&mut self, x: i32, y: i32, v: u8);
    /// `_TERRAINset` ≠ 0.
    fn casa(&self) -> bool;
    /// Buraco novo da cor em (x, c): `_HoleMap?[c] = x`, `_LastNewHole? = (x, c)`, `_LastNewExit? =
    /// (c, 0)`.
    fn registrar_buraco(&mut self, cor: Cor, x: i32, c: i32);
    /// Sorteios `Sk(k)` e `S(n)` do gerador S.
    fn sk(&mut self, k: u16) -> i32;
    fn s1(&mut self, n: u16) -> i32;
}

/// Totais de escavação de um ninho (`_TileTotX?/Y?`, `_TilesDug?`, `_TileMassX?/Y?`): soma uma célula e
/// recalcula o centro de massa (alvo da rainha). Somas de 32 bits, contagem e centro de 16 bits,
/// divisão com sinal (ninho.md §7.3).
pub fn somar_escavacao(soma: (&mut i32, &mut i32), cavados: &mut i16, centro: (&mut i16, &mut i16), x: i32, y: i32) {
    *soma.0 = soma.0.wrapping_add(x);
    *soma.1 = soma.1.wrapping_add(y);
    *cavados = cavados.wrapping_add(1);
    let n = *cavados as i32;
    if n > 0 {
        *centro.0 = (*soma.0 / n) as i16;
        *centro.1 = (*soma.1 / n) as i16;
    }
}

fn somar_na_colonia(c: &mut Colonia, x: i32, y: i32) {
    somar_escavacao(
        (&mut c.soma_x_escavada, &mut c.soma_y_escavada),
        &mut c.tiles_escavados,
        (&mut c.centro_x, &mut c.centro_y),
        x,
        y,
    );
}

impl Superficie for Estado {
    fn tile_superficie(&self, x: i32, y: i32) -> Option<u8> {
        self.superficie.mapa.em(x, y)
    }
    fn vida_superficie(&self, x: i32, y: i32) -> Option<u8> {
        self.superficie.vida.em(x, y)
    }
}

impl Terreno for Estado {
    fn tile_ninho(&self, cor: Cor, x: i32, y: i32) -> Option<u8> {
        self.ninho(cor).mapa.em(x, y)
    }
    fn gravar_tile_ninho(&mut self, cor: Cor, x: i32, y: i32, v: u8) {
        self.ninho_mut(cor).mapa.definir(x, y, v);
    }
    fn saida(&self, cor: Cor, x: i32, y: i32) -> Option<u8> {
        self.ninho(cor).saida.em(x, y)
    }
    fn gravar_saida(&mut self, cor: Cor, x: i32, y: i32, v: u8) {
        self.ninho_mut(cor).saida.definir(x, y, v);
    }
    fn somar_escavacao(&mut self, cor: Cor, x: i32, y: i32) {
        somar_na_colonia(self.colonia_mut(cor), x, y);
    }
    fn gravar_tile_superficie(&mut self, x: i32, y: i32, v: u8) {
        self.superficie.mapa.definir(x, y, v);
    }
    fn casa(&self) -> bool {
        em_casa(self)
    }
    fn registrar_buraco(&mut self, cor: Cor, x: i32, c: i32) {
        let col = self.colonia_mut(cor);
        col.ultimo_buraco = (x as i16, c as i16);
        col.ultima_saida = (c as i16, 0);
        if let Some(b) = col.buracos.get_mut(c as usize) {
            *b = x as u8;
        }
    }
    fn sk(&mut self, k: u16) -> i32 {
        self.s.mascara(k)
    }
    fn s1(&mut self, n: u16) -> i32 {
        self.s.rand1(n)
    }
}

fn no_ninho(x: i32, y: i32) -> bool {
    (0..LADO_NINHO).contains(&x) && (0..LADO_NINHO).contains(&y)
}

/// `_SmoothEdgesB/R(x, y)` (`1020:255a`/`26e4`). Na linha 0, terra e capim viram abertura (0x18); abaixo,
/// uma célula sólida recebe a borda pela máscara das quatro vizinhas sólidas (0x20 + máscara − 1, ou
/// + 0x2F se inundada); sem vizinha sólida vira túnel (`Sk(8)`) ou túnel inundado.
pub fn alisar<T: Terreno + ?Sized>(t: &mut T, cor: Cor, x: i32, y: i32) {
    // O original não filtra y < 0 (leria a coluna anterior); só aconteceria cavando a linha 0 com
    // `_DigTileB`, o que nenhuma regra faz.
    if !no_ninho(x, y) {
        return;
    }
    let m = |t: &T, x: i32, y: i32| t.tile_ninho(cor, x, y).unwrap_or(0);
    if y == 0 {
        if m(t, x, 0) <= 0x2F {
            t.gravar_tile_ninho(cor, x, 0, ABERTURA_NINHO);
        }
        return;
    }
    let v = m(t, x, y);
    if !e_solido(v) {
        return;
    }
    let inundada = v >= TUNEL_INUNDADO;
    let mut mascara = 0u8;
    if y < 2 || e_solido(m(t, x, y - 1)) {
        mascara |= 1;
    }
    if x >= LADO_NINHO - 1 || e_solido(m(t, x + 1, y)) {
        mascara |= 2;
    }
    if y >= LADO_NINHO - 1 || e_solido(m(t, x, y + 1)) {
        mascara |= 4;
    }
    if x <= 0 || e_solido(m(t, x - 1, y)) {
        mascara |= 8;
    }
    let novo = if mascara != 0 {
        0x1F + mascara + if inundada { 0x2F } else { 0 }
    } else if inundada {
        TUNEL_INUNDADO
    } else {
        t.sk(8) as u8
    };
    t.gravar_tile_ninho(cor, x, y, novo);
}

/// `_FixExitMapB/R(x, y)` (`1020:284e`/`2914`, ninho.md §7.4): nas linhas 0 e 1, 0xFF se a célula é
/// abertura e 0xFE se não; abaixo, o maior valor das 8 vizinhas menos 1 (ou 0). Só a célula dada muda.
pub fn corrigir_saida<T: Terreno + ?Sized, D: Direcoes + ?Sized>(t: &mut T, dirs: &D, cor: Cor, x: i32, y: i32) {
    if !no_ninho(x, y) {
        return;
    }
    let novo = if y < 2 {
        if t.tile_ninho(cor, x, y) == Some(ABERTURA_NINHO) {
            SAIDA_ABERTA
        } else {
            SAIDA_FECHADA
        }
    } else {
        let maior = (0..8).filter_map(|k| {
            let (vx, vy) = dirs.vizinha(x, y, k);
            t.saida(cor, vx, vy)
        });
        maior.max().unwrap_or(0).saturating_sub(1)
    };
    t.gravar_saida(cor, x, y, novo);
}

/// As quatro chamadas a `_SmoothEdges?` (cima, direita, baixo, esquerda) e `_FixExitMap?` da célula.
pub fn alisar_vizinhas<T: Terreno + ?Sized, D: Direcoes + ?Sized>(t: &mut T, dirs: &D, cor: Cor, x: i32, y: i32) {
    alisar(t, cor, x, y - 1);
    alisar(t, cor, x + 1, y);
    alisar(t, cor, x, y + 1);
    alisar(t, cor, x - 1, y);
    corrigir_saida(t, dirs, cor, x, y);
}

/// `_DigTileB(x, y)` (`1020:1fe4`) e `_DigTileR` (`1020:21de`): cava se for terra (`Sk(8)`); no ninho
/// preto, abaixo de y = 53, 1/64 de chance de abrir a passagem 0x14 nos dois ninhos. Sempre alisa as
/// vizinhas e refaz a saída da célula. (`_MakeNewHoleR`, `_CreateNewHole` e `_DigMyTile` têm a versão
/// vermelha embutida; é a mesma conta.)
pub fn cavar<T: Terreno + ?Sized, D: Direcoes + ?Sized>(t: &mut T, dirs: &D, cor: Cor, x: i32, y: i32) {
    if !no_ninho(x, y) {
        return;
    }
    if t.tile_ninho(cor, x, y).is_some_and(e_terra) {
        let v = t.sk(8) as u8;
        t.gravar_tile_ninho(cor, x, y, v);
        t.somar_escavacao(cor, x, y);
        if cor == Cor::Preta && y > 53 && t.s1(64) == 0 {
            t.gravar_tile_ninho(Cor::Preta, x, y, PASSAGEM_ENTRE_NINHOS);
            if t.tile_ninho(Cor::Vermelha, x, y).is_some_and(e_terra) {
                let v = t.sk(8) as u8;
                t.gravar_tile_ninho(Cor::Vermelha, x, y, v);
                t.somar_escavacao(Cor::Vermelha, x, y);
            }
            alisar_vizinhas(t, dirs, Cor::Vermelha, x, y);
            t.gravar_tile_ninho(Cor::Vermelha, x, y, PASSAGEM_ENTRE_NINHOS);
        }
    }
    alisar_vizinhas(t, dirs, cor, x, y);
}

/// `_DigTileThemB/R(x, y)` (`1020:22d4`/`241c`): a escavação da operária. Só cava com 0 < x < 63, terra
/// embaixo (ou y > 62) e terra em cima (ou y < 3); não exige que a própria célula seja terra. Na linha
/// 0 abre o buraco (abertura e [`novo_buraco`]). Devolve se cavou.
pub fn cavar_tunel<T: Terreno + ?Sized, D: Direcoes + ?Sized>(
    t: &mut T,
    dirs: &D,
    borda_buraco: &[u8; 8],
    cor: Cor,
    x: i32,
    y: i32,
) -> bool {
    let terra_em = |t: &T, x: i32, y: i32| t.tile_ninho(cor, x, y).is_some_and(e_terra);
    let baixo = y > 62 || terra_em(t, x, y + 1);
    let cima = y < 3 || terra_em(t, x, y - 1);
    if !(baixo && cima && x != 0 && x < 63) {
        return false;
    }
    if y == 0 {
        t.gravar_tile_ninho(cor, x, 0, ABERTURA_NINHO);
        novo_buraco(t, dirs, borda_buraco, cor, x);
    } else {
        let v = t.sk(8) as u8;
        t.gravar_tile_ninho(cor, x, y, v);
    }
    t.somar_escavacao(cor, x, y);
    alisar_vizinhas(t, dirs, cor, x, y);
    true
}

/// `_IsClearTile(1, x, y)` na superfície: dentro do mapa, sem formiga (a amarela conta como livre) e
/// chão (< 0x10).
pub fn livre_na_superficie<T: Superficie + ?Sized>(t: &T, x: i32, y: i32) -> bool {
    match (t.tile_superficie(x, y), t.vida_superficie(x, y)) {
        (Some(m), Some(v)) => (v == 0 || e_amarela(v)) && m < 0x10,
        _ => false,
    }
}

/// `_IsClear3x3(1, x, y)`: a célula e as 8 vizinhas livres.
pub fn livre_3x3<T: Superficie + ?Sized, D: Direcoes + ?Sized>(t: &T, dirs: &D, x: i32, y: i32) -> bool {
    livre_na_superficie(t, x, y)
        && (0..8).all(|k| {
            let (vx, vy) = dirs.vizinha(x, y, k);
            livre_na_superficie(t, vx, vy)
        })
}

/// Borda de um buraco recém-aberto no quintal em (x, y): as vizinhas com tile < 0x50 recebem a borda da
/// direção (`DGROUP:0x230c`). Parte de `_MakeNewHole*` e de `_CreateNewHole`.
pub fn por_borda_do_buraco<T: Terreno + ?Sized, D: Direcoes + ?Sized>(t: &mut T, dirs: &D, borda_buraco: &[u8; 8], x: i32, y: i32) {
    for k in 0..8 {
        let (vx, vy) = dirs.vizinha(x, y, k);
        if t.tile_superficie(vx, vy).is_some_and(|v| v < BURACO_QUINTAL) {
            t.gravar_tile_superficie(vx, vy, borda_buraco[k as usize]);
        }
    }
}

/// `_CanBeHouseHole`: versão "com buraco" de um tile da casa, ou `None`.
fn buraco_da_casa(v: u8) -> Option<u8> {
    Some(match v {
        0 => 0x86,
        2 | 3 => 0x8A,
        0x5E..=0x61 => v + 0x22,
        0x66 => 0x85,
        0x68 => 0x84,
        _ => return None,
    })
}

/// `_MakeNewHoleB(c)` (`1020:1b06`) e `_MakeNewHoleR(c)` (`1020:1d02`) (geracao-mundo.md §3.A): buraco
/// novo na linha `c` da superfície, ligado à coluna `c` do ninho. Um `S(31)`; até 34 colunas candidatas
/// (pretas a partir de x = 2, vermelhas a partir de x = 126 para a esquerda). Quintal: a primeira com
/// 3 × 3 livre recebe o buraco (0x50) e a borda nas vizinhas < 0x50; casa: a primeira com versão "com
/// buraco". Depois registra o buraco e cava (c, 1) no ninho. Sem lugar, nada muda.
pub fn novo_buraco<T: Terreno + ?Sized, D: Direcoes + ?Sized>(t: &mut T, dirs: &D, borda_buraco: &[u8; 8], cor: Cor, c: i32) {
    let r = t.s1(31);
    let x_de = |k: i32| {
        let m = (r + k) % 32;
        match cor {
            Cor::Preta => m + 2,
            Cor::Vermelha => 126 - m,
        }
    };
    let achado = if !t.casa() {
        let x = (0..34).map(x_de).find(|&x| livre_3x3(t, dirs, x, c));
        if let Some(x) = x {
            t.gravar_tile_superficie(x, c, BURACO_QUINTAL);
            por_borda_do_buraco(t, dirs, borda_buraco, x, c);
        }
        x
    } else {
        let x = (0..34).map(x_de).find_map(|x| {
            let novo = buraco_da_casa(t.tile_superficie(x, c).unwrap_or(0xFF))?;
            Some((x, novo))
        });
        if let Some((x, novo)) = x {
            t.gravar_tile_superficie(x, c, novo);
        }
        x.map(|(x, _)| x)
    };
    let Some(x) = achado else { return };
    t.registrar_buraco(cor, x, c);
    cavar(t, dirs, cor, c, 1);
}

#[cfg(test)]
mod testes {
    use super::*;

    struct Dirs8;
    impl Direcoes for Dirs8 {
        fn deslocamentos(&self) -> (&[i8; 8], &[i8; 8]) {
            (&[0, 1, 1, 1, 0, -1, -1, -1], &[-1, -1, 0, 1, 1, 1, 0, -1])
        }
    }

    fn lista_a_cheia(buracos: &[usize]) -> Estado {
        let mut e = Estado::vazio();
        let l = &mut e.superficie.lista;
        for (i, f) in l.vagas.iter_mut().enumerate() {
            *f = Formiga { x: (i % 100) as u8, y: (i / 100) as u8, modo: 2, tipo: if buracos.contains(&i) { 0 } else { 0x10 }, s: i as u8 };
        }
        l.indice = CAPACIDADE_A as i16;
        e
    }

    /// ninho.md §4.6: com a lista A cheia a formiga vai para a vaga 1000 (perdida) e a compactação
    /// deixa o contador com uma vaga a mais, com o resto que estava nela.
    #[test]
    fn saida_com_a_lista_a_cheia_segue_o_original() {
        // Sem buracos: a formiga se perde e o contador fica em 1000; a saída conta como feita.
        let mut e = lista_a_cheia(&[]);
        let antes = e.superficie.lista.clone();
        assert!(sair_pelo_buraco(&mut e, &Dirs8, 30, 30, 0x10, 2, 0));
        assert_eq!(e.superficie.lista, antes);

        // Dois buracos antes da vaga 998: as 998 vivas deslizam e a vaga 998 (que tinha a formiga
        // 998, agora copiada na 996) entra na lista: a formiga nova não aparece, a 998 fica duplicada.
        let mut e = lista_a_cheia(&[5, 7]);
        let velha_998 = e.superficie.lista.vagas[998];
        assert!(sair_pelo_buraco(&mut e, &Dirs8, 30, 30, 0x10, 2, 0));
        let l = &e.superficie.lista;
        assert_eq!(l.indice, 999);
        assert_eq!(l.vagas[996], velha_998);
        assert_eq!(l.vagas[998], velha_998);
        assert!(l.vagas[..999].iter().all(|f| f.x != 30 || f.y != 29), "a formiga que saiu não está na lista");

        // Buraco no fim (vaga 999): a vaga incluída é a própria 999, vazia.
        let mut e = lista_a_cheia(&[999]);
        assert!(sair_pelo_buraco(&mut e, &Dirs8, 30, 30, 0x10, 2, 0));
        let l = &e.superficie.lista;
        assert_eq!(l.indice, 1000);
        assert_eq!(l.vagas[999].tipo, 0);
    }

    #[test]
    fn comida_cai_na_casa_pelo_dropdir() {
        let mut e = Estado::vazio();
        e.ambiente.terreno = 1;
        for y in 0..64 {
            e.superficie.mapa.definir(10, y, 0x30);
        }
        e.superficie.mapa.definir(10, 14, 2);
        // _DROPdir 2 (para baixo): a comida cai até a primeira célula livre.
        e.passo_queda = (0, 1);
        assert!(largar_comida_superficie(&mut e, 10, 10));
        assert_eq!(e.superficie.mapa.em(10, 14), Some((2 + 6) * 4));
        assert_eq!(e.ambiente.comida_superficie, 1);
        // _DROPdir 0 (dy = 255, somado sem sinal): sai do mapa no primeiro passo; a comida some.
        e.passo_queda = (0, 255);
        e.superficie.mapa.definir(10, 9, 1);
        assert!(largar_comida_superficie(&mut e, 10, 10));
        assert_eq!(e.superficie.mapa.em(10, 9), Some(1));
        assert_eq!(e.ambiente.comida_superficie, 1);
        // Tiles 4–7 também fazem cair.
        e.passo_queda = (1, 0);
        e.superficie.mapa.definir(20, 5, 5);
        e.superficie.mapa.definir(22, 5, 0);
        e.superficie.mapa.definir(21, 5, 0x30);
        assert!(largar_comida_superficie(&mut e, 20, 5));
        assert_eq!(e.superficie.mapa.em(22, 5), Some(24));
    }

    struct D8;
    impl Direcoes for D8 {
        fn deslocamentos(&self) -> (&[i8; 8], &[i8; 8]) {
            (&[0, 1, 1, 1, 0, -1, -1, -1], &[-1, -1, 0, 1, 1, 1, 0, -1])
        }
    }

    #[test]
    fn get_dir_pelos_sinais() {
        let esperado = [((5, 5), 0), ((5, 0), 1), ((9, 0), 2), ((9, 5), 3), ((9, 9), 4), ((5, 9), 5), ((0, 9), 6), ((0, 5), 7), ((0, 0), 8)];
        for ((x, y), d) in esperado {
            assert_eq!(get_dir(5, 5, x, y), d, "({x}, {y})");
            if d > 0 {
                // A direção devolvida aponta para o alvo.
                let (dx, dy) = D8.d8(d - 1);
                assert_eq!((dx, dy), ((x - 5).signum(), (y - 5).signum()));
            }
        }
        assert_eq!(get_dis(0, 0, 3, 4), 25);
    }

    #[test]
    fn soma_da_escavacao_e_centro_de_massa() {
        let mut c = Colonia::default();
        somar_na_colonia(&mut c, 30, 4);
        somar_na_colonia(&mut c, 33, 7);
        assert_eq!((c.tiles_escavados, c.centro_x, c.centro_y), (2, 31, 5));
    }

    #[test]
    fn quicar_aponta_para_dentro() {
        let mut s = GeradorS::new(0x1234);
        for _ in 0..200 {
            for (x, y) in [(0, 0), (0, 30), (0, 63), (127, 0), (127, 30), (127, 63), (60, 0), (60, 63)] {
                let d = quicar(&mut s, x, y).unwrap() as i32;
                let (vx, vy) = D8.vizinha(x, y, d);
                assert!((0..128).contains(&vx) && (0..64).contains(&vy), "({x},{y}) dir {d}");
            }
        }
        let antes = s;
        assert_eq!(quicar(&mut s, 5, 5), None);
        assert_eq!(s, antes, "fora da borda não sorteia");
    }

    #[test]
    fn modo_fora_da_tabela_usa_a_ultima_linha() {
        let linhas = [[1u8; 8], [2; 8]];
        assert_eq!(linha_por_modo(&linhas, 7), [2; 8]);
        assert_eq!(linha_por_modo(&linhas, -3), [1; 8]);
    }
}
