//! Geração do mundo: novo jogo (`_RandYard`) e patch do quintal (`_RandWorld`, `_MakeYardPatch`...).
//! Especificação: docs/spec/regras/geracao-mundo.md (ordem dos sorteios) e tiles-valores.md (valores).
//!
//! As tabelas (formas das pedras, bordas, direções, bicho-de-conta) vêm de `data/geracao.json`
//! (extraído do executável por `tools/extrair_geracao.py`); a tabela de seno vem do objeto tipo 9
//! nº 1000 do banco original, lido em tempo de execução. Os valores de tile que aparecem aqui como
//! constantes são os descritos em tiles-valores.md.
//!
//! A escavação (`_DigTile*`, `_SmoothEdges*`, `_FixExitMap*`, `_MakeNewHole*`, `_IsClear3x3`) é a mesma
//! do tick: `rules/comum.rs`, sobre a trait `Terreno`, que o [`Mundo`] implementa junto com o gerador S
//! da geração.
//!
//! Fora do escopo (ver a seção "Implementação no port" da spec): patches da casa (`_MakeHousePatch`),
//! feromônios, histórico, contagens de casta e estado de interface (vistas, janelas).

use serde::Deserialize;

use super::comum::{self, e_terra, Direcoes, Superficie, Terreno};
use super::rng::{GeradorR, GeradorS};
use crate::formats::banco::Chave;
use crate::formats::save::Formiga;
use crate::recursos::Recursos;

/// Superfície (`_MapA`, `_LifeA`): 128 × 64, índice `x * 64 + y`.
pub const LARGURA_SUPERFICIE: i32 = 128;
/// Altura de todos os mapas e largura dos ninhos (`_MapB`, `_MapR`: 64 × 64).
pub const LADO: i32 = 64;
pub const CELULAS_SUPERFICIE: usize = (LARGURA_SUPERFICIE * LADO) as usize;
pub const CELULAS_NINHO: usize = (LADO * LADO) as usize;
/// Número de patches do quintal (12 × 16), tamanho de `_YMapSeeds`.
pub const PATCHES: usize = 192;
/// Patch inicial de todo novo jogo: `(11, 8)`.
pub const PATCH_INICIAL: (i32, i32) = (11, 8);

/// Arquivo de dados com as tabelas da geração.
pub const ARQUIVO_TABELAS: &str = "geracao.json";
/// Objeto do banco com a tabela de seno (`_FracSineTab`): 64 palavras big-endian.
pub const CHAVE_SENO: Chave = Chave { tipo: 9, id: 1000 };

// Valores de tile (docs/spec/regras/tiles-valores.md).
const TERRA_MACICA: u8 = 0x2E;
const BURACO: u8 = 0x50;
const PEDRINHA: u8 = 0x51;
const COMIDA: u8 = 0x48;
const FORMIGA_LEAO_CENTRO: u8 = 0x38;
const FORMIGA_LEAO_BORDA: u8 = 0x30;
const SAIDA_ABERTA: u8 = 0xFF;
/// `_LifeA/B` da formiga amarela: cabeça e segunda célula (só a rainha, tipo 0x60, ocupa duas).
const VIDA_AMARELA: u8 = 0xFF;
const VIDA_AMARELA_2: u8 = 0xFE;

/// Tipo de jogo (`_CurGameType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoJogo {
    Tutorial = 0,
    Rapido = 1,
    Completo = 2,
    Experimental = 3,
}

impl TipoJogo {
    pub fn de_i16(v: i16) -> Option<TipoJogo> {
        Some(match v {
            0 => TipoJogo::Tutorial,
            1 => TipoJogo::Rapido,
            2 => TipoJogo::Completo,
            3 => TipoJogo::Experimental,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cor {
    Preta,
    Vermelha,
}

/// Tabelas da geração (conteúdo de `data/geracao.json` mais o seno do banco).
#[derive(Clone, Debug, Deserialize)]
pub struct TabelasGeracao {
    /// `_Dx8`/`_Dy8`: deslocamento por direção 0–7 (0 = cima, 2 = direita, 4 = baixo, 6 = esquerda).
    pub dx8: [i8; 8],
    pub dy8: [i8; 8],
    /// Borda do buraco do ninho por direção (`DGROUP:0x230c`).
    pub borda_buraco: [u8; 8],
    /// Borda da armadilha da formiga-leão por direção, somada a 0x30 (`DGROUP:0x25e8`).
    pub borda_formiga_leao: [u8; 8],
    /// `_SowTab`: tile do bicho-de-conta por direção.
    pub sow_tab: [u8; 8],
    /// Formas de pedra 5 × 5 e 3 × 3: célula `(x + i, y + j)` recebe `forma[j * N + i]` se não for 0.
    pub pedras5: Vec<Vec<u8>>,
    pub pedras3: Vec<Vec<u8>>,
    /// `_FracSineTab`: `sin(i·π/128)·32767` para i = 0…63.
    #[serde(skip)]
    pub seno: Vec<i16>,
}

impl TabelasGeracao {
    /// Lê `geracao.json` (com mods) e o seno do banco.
    pub fn carregar(r: &Recursos) -> Result<TabelasGeracao, String> {
        let mut t: TabelasGeracao = r.dados(ARQUIVO_TABELAS)?;
        t.seno = seno_do_objeto(&r.objeto(CHAVE_SENO)?)?;
        t.validar()?;
        Ok(t)
    }

    /// Monta a partir do texto do JSON e dos bytes do objeto 9/1000.
    pub fn de_partes(json: &str, objeto_seno: &[u8]) -> Result<TabelasGeracao, String> {
        let mut t: TabelasGeracao = serde_json::from_str(json).map_err(|e| format!("{ARQUIVO_TABELAS}: {e}"))?;
        t.seno = seno_do_objeto(objeto_seno)?;
        t.validar()?;
        Ok(t)
    }

    fn validar(&self) -> Result<(), String> {
        comum::validar_direcoes(ARQUIVO_TABELAS, &self.dx8, &self.dy8)?;
        if self.pedras5.len() != 6 || self.pedras5.iter().any(|f| f.len() != 25) {
            return Err("pedras5 deve ter 6 formas de 25 bytes".into());
        }
        if self.pedras3.len() != 6 || self.pedras3.iter().any(|f| f.len() != 9) {
            return Err("pedras3 deve ter 6 formas de 9 bytes".into());
        }
        if self.seno.len() != 64 {
            return Err("tabela de seno deve ter 64 valores".into());
        }
        Ok(())
    }

    /// Seno com ângulo de 0 a 255 por volta, em unidades de 1/32767 (rotina em linha de `_AddFood`).
    pub fn seno(&self, angulo: i32) -> i32 {
        let mut u = angulo & 0x7F;
        if u > 0x3F {
            u = 0x80 - u;
        }
        let v = if u == 0x40 { 0x7FFF } else { self.seno[(u & 0x3F) as usize] as i32 };
        if angulo & 0xFF > 0x7F {
            -v
        } else {
            v
        }
    }

    pub fn cosseno(&self, angulo: i32) -> i32 {
        self.seno((angulo + 0x40) & 0xFF)
    }

    /// [`Direcoes::vizinha`] com índice `usize`.
    fn vizinha(&self, x: i32, y: i32, d: usize) -> (i32, i32) {
        Direcoes::vizinha(self, x, y, d as i32)
    }
}

impl Direcoes for TabelasGeracao {
    fn deslocamentos(&self) -> (&[i8; 8], &[i8; 8]) {
        (&self.dx8, &self.dy8)
    }
}

fn seno_do_objeto(b: &[u8]) -> Result<Vec<i16>, String> {
    if b.len() < 128 {
        return Err(format!("objeto 9/1000 (seno) com {} bytes, esperado 128", b.len()));
    }
    Ok(b[..128].chunks(2).map(|p| i16::from_be_bytes([p[0], p[1]])).collect())
}

/// Acumuladores de escavação de um ninho (`_TileTotX?/Y?`, `_TilesDug?`, `_TileMassX?/Y?`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Escavacao {
    pub tot_x: i32,
    pub tot_y: i32,
    pub cavados: i16,
    pub massa_x: i16,
    pub massa_y: i16,
}

impl Escavacao {
    fn somar(&mut self, x: i32, y: i32) {
        comum::somar_escavacao(
            (&mut self.tot_x, &mut self.tot_y),
            &mut self.cavados,
            (&mut self.massa_x, &mut self.massa_y),
            x,
            y,
        );
    }
}

/// Bicho-de-conta parado (`_SowX/_SowY/_SowDir/_SowSave`, índices 1 e 2 usados).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BichoDeConta {
    pub x: i16,
    pub y: i16,
    pub dir: i16,
    /// Chão que estava na célula.
    pub chao: u16,
}

/// Formiga amarela (variáveis `_Me*`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormigaAmarela {
    /// `_MePlane`: 0/1 superfície, 2 ninho preto, 3 ninho vermelho.
    pub plano: i16,
    pub x: i16,
    pub y: i16,
    pub tipo: u8,
    pub dir: u8,
    /// `_MeMode`.
    pub modo: i16,
    pub saude: i16,
    /// `_MeEggCarried`.
    pub ovo_carregado: i16,
}

/// Aranha (`_InitSpider`); coordenadas × 16.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Aranha {
    pub x: i16,
    pub y: i16,
    pub dir: i16,
    pub arrotos: i16,
    pub ligada: bool,
}

/// Estado do mundo produzido pela geração. Nomes do original entre crases nos comentários.
#[derive(Clone, Debug)]
pub struct Mundo {
    pub tipo_jogo: TipoJogo,
    /// `_TERRAINset`: 0 = quintal, 1 = casa.
    pub terreno: u8,
    pub mapa_a: Vec<u8>,
    pub mapa_b: Vec<u8>,
    pub mapa_r: Vec<u8>,
    /// `_ExitMapB/R`.
    pub saida_b: Vec<u8>,
    pub saida_r: Vec<u8>,
    pub vida_a: Vec<u8>,
    pub vida_b: Vec<u8>,
    pub vida_r: Vec<u8>,
    pub lista_a: Vec<Formiga>,
    pub lista_b: Vec<Formiga>,
    pub lista_r: Vec<Formiga>,
    /// `_HoleMapB/R`: x do buraco na superfície por coluna do ninho (0 = nenhum).
    pub buracos_b: [u8; 64],
    pub buracos_r: [u8; 64],
    /// `(_LastNewHoleB, _LastNewExitB)` e o equivalente vermelho.
    pub ultimo_buraco_b: (i16, i16),
    pub ultimo_buraco_r: (i16, i16),
    pub escavacao_b: Escavacao,
    pub escavacao_r: Escavacao,
    /// `_WaterTable` (64 = nenhuma linha inundada).
    pub lencol: i16,
    pub rainhas_pretas: i16,
    pub rainhas_vermelhas: i16,
    pub ultimo_ovo_preto: (i16, i16),
    pub ultimo_ovo_vermelho: (i16, i16),
    /// `(_RebornX, _RebornY)`: onde a formiga amarela nasce no ninho preto.
    pub renascer: (i16, i16),
    /// `_LionListX/Y` (M, S e T ficam 0 na geração).
    pub formigas_leao: Vec<(u8, u8)>,
    /// `_InitialLions`.
    pub formigas_leao_iniciais: i16,
    /// `_SowX/Y/Dir/Save[0..3]`; a geração usa os índices 2 e 1, nessa ordem.
    pub bichos_de_conta: [BichoDeConta; 3],
    pub comida_a: i16,
    pub comida_b: i16,
    pub comida_r: i16,
    /// `_LastFoodDrop`.
    pub ultima_comida: (i16, i16),
    pub amarela: FormigaAmarela,
    pub aranha: Aranha,
    pub saude_b: i16,
    pub saude_r: i16,
    /// `_MeNestStarted`.
    pub ninho_iniciado: bool,
    /// `_YMapSeeds`, índice `y * 12 + x`.
    pub sementes: [u16; PATCHES],
    /// `_CurYardPnt`.
    pub patch: (i32, i32),
}

fn ix(x: i32, y: i32) -> usize {
    debug_assert!(x >= 0 && (0..LADO).contains(&y));
    (x * LADO + y) as usize
}

fn na_superficie(x: i32, y: i32) -> bool {
    (0..LARGURA_SUPERFICIE).contains(&x) && (0..LADO).contains(&y)
}

/// O mundo em geração com o gerador S dela: o que as rotinas de escavação de `comum.rs` precisam.
struct Escavando<'a> {
    m: &'a mut Mundo,
    s: &'a mut GeradorS,
}

fn no_ninho(x: i32, y: i32) -> bool {
    (0..LADO).contains(&x) && (0..LADO).contains(&y)
}

impl Superficie for Mundo {
    fn tile_superficie(&self, x: i32, y: i32) -> Option<u8> {
        na_superficie(x, y).then(|| self.mapa_a[ix(x, y)])
    }
    fn vida_superficie(&self, x: i32, y: i32) -> Option<u8> {
        na_superficie(x, y).then(|| self.vida_a[ix(x, y)])
    }
}

impl Superficie for Escavando<'_> {
    fn tile_superficie(&self, x: i32, y: i32) -> Option<u8> {
        self.m.tile_superficie(x, y)
    }
    fn vida_superficie(&self, x: i32, y: i32) -> Option<u8> {
        self.m.vida_superficie(x, y)
    }
}

impl Terreno for Escavando<'_> {
    fn tile_ninho(&self, cor: Cor, x: i32, y: i32) -> Option<u8> {
        let mapa = if cor == Cor::Preta { &self.m.mapa_b } else { &self.m.mapa_r };
        no_ninho(x, y).then(|| mapa[ix(x, y)])
    }
    fn gravar_tile_ninho(&mut self, cor: Cor, x: i32, y: i32, v: u8) {
        if no_ninho(x, y) {
            self.m.ninho_mut(cor)[ix(x, y)] = v;
        }
    }
    fn saida(&self, cor: Cor, x: i32, y: i32) -> Option<u8> {
        let saida = if cor == Cor::Preta { &self.m.saida_b } else { &self.m.saida_r };
        no_ninho(x, y).then(|| saida[ix(x, y)])
    }
    fn gravar_saida(&mut self, cor: Cor, x: i32, y: i32, v: u8) {
        if no_ninho(x, y) {
            let saida = if cor == Cor::Preta { &mut self.m.saida_b } else { &mut self.m.saida_r };
            saida[ix(x, y)] = v;
        }
    }
    fn somar_escavacao(&mut self, cor: Cor, x: i32, y: i32) {
        self.m.escavacao_mut(cor).somar(x, y);
    }
    fn gravar_tile_superficie(&mut self, x: i32, y: i32, v: u8) {
        if na_superficie(x, y) {
            self.m.mapa_a[ix(x, y)] = v;
        }
    }
    fn casa(&self) -> bool {
        self.m.terreno != 0
    }
    /// A geração não tem `_LastNewExit?` separado: `Estado::de_mundo` o deriva do buraco.
    fn registrar_buraco(&mut self, cor: Cor, x: i32, c: i32) {
        let (ultimo, buracos) = match cor {
            Cor::Preta => (&mut self.m.ultimo_buraco_b, &mut self.m.buracos_b),
            Cor::Vermelha => (&mut self.m.ultimo_buraco_r, &mut self.m.buracos_r),
        };
        *ultimo = (x as i16, c as i16);
        if let Some(b) = buracos.get_mut(c as usize) {
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

impl Mundo {
    /// Mundo com todos os mapas e listas zerados (`_ClrArrays`).
    pub fn vazio(tipo_jogo: TipoJogo) -> Mundo {
        Mundo {
            tipo_jogo,
            terreno: 0,
            mapa_a: vec![0; CELULAS_SUPERFICIE],
            mapa_b: vec![0; CELULAS_NINHO],
            mapa_r: vec![0; CELULAS_NINHO],
            saida_b: vec![0; CELULAS_NINHO],
            saida_r: vec![0; CELULAS_NINHO],
            vida_a: vec![0; CELULAS_SUPERFICIE],
            vida_b: vec![0; CELULAS_NINHO],
            vida_r: vec![0; CELULAS_NINHO],
            lista_a: Vec::new(),
            lista_b: Vec::new(),
            lista_r: Vec::new(),
            buracos_b: [0; 64],
            buracos_r: [0; 64],
            ultimo_buraco_b: (0, 0),
            ultimo_buraco_r: (0, 0),
            escavacao_b: Escavacao::default(),
            escavacao_r: Escavacao::default(),
            lencol: 64,
            rainhas_pretas: 0,
            rainhas_vermelhas: 0,
            ultimo_ovo_preto: (-1, -1),
            ultimo_ovo_vermelho: (-1, -1),
            renascer: (0, 0),
            formigas_leao: Vec::new(),
            formigas_leao_iniciais: 0,
            bichos_de_conta: [BichoDeConta::default(); 3],
            comida_a: 0,
            comida_b: 0,
            comida_r: 0,
            ultima_comida: (0, 0),
            amarela: FormigaAmarela::default(),
            aranha: Aranha::default(),
            saude_b: 100,
            saude_r: 100,
            ninho_iniciado: false,
            sementes: [0; PATCHES],
            patch: PATCH_INICIAL,
        }
    }

    /// Novo jogo (`_RandYard`, geracao-mundo.md §2): sementes dos 192 patches com o gerador R e
    /// geração do patch inicial (11, 8) com o gerador S.
    pub fn novo_jogo(
        tipo_jogo: TipoJogo,
        r: &mut GeradorR,
        s: &mut GeradorS,
        tab: &TabelasGeracao,
    ) -> Result<Mundo, String> {
        let mut m = Mundo::vazio(tipo_jogo);
        m.patch = PATCH_INICIAL;
        for semente in m.sementes.iter_mut() {
            *semente = ((r.rrand(0x7FFF) + 0x4000) & 0x7FFF) as u16;
        }
        m.ninho_iniciado = tipo_jogo != TipoJogo::Completo;
        let (px, py) = PATCH_INICIAL;
        let semente = m.sementes[(py * 12 + px) as usize];
        let pretas = if tipo_jogo != TipoJogo::Completo { 1 } else { 0 };
        m.gerar_patch(s, tab, semente, pretas, 1, px, py)?;
        Ok(m)
    }

    /// `_RandWorld(semente, pretas, vermelhas, px, py)` (geracao-mundo.md §3). Usa os mapas atuais
    /// como estão (em `_XferPatch`, o `_LifeA` do patch anterior ainda é visto pelas formigas-leão).
    #[allow(clippy::too_many_arguments)]
    pub fn gerar_patch(
        &mut self,
        s: &mut GeradorS,
        tab: &TabelasGeracao,
        semente: u16,
        pretas: i32,
        vermelhas: i32,
        px: i32,
        py: i32,
    ) -> Result<(), String> {
        s.definir_semente(semente);
        self.aranha = Aranha {
            x: 0x400,
            y: 0x200,
            dir: 0,
            arrotos: 10,
            ligada: self.tipo_jogo != TipoJogo::Tutorial,
        };
        self.gerar_mapa(s, tab, px, py)?;
        let pretas = pretas + (pretas >> 2);
        let vermelhas = vermelhas + (vermelhas >> 2);

        self.mapa_b.fill(TERRA_MACICA);
        self.mapa_r.fill(TERRA_MACICA);
        self.saida_b.fill(0);
        self.saida_r.fill(0);
        self.vida_b.fill(0);
        self.vida_r.fill(0);
        self.vida_a.fill(0);

        for c in 0..LADO {
            let i = ix(c, 0);
            if self.terreno == 0 {
                self.mapa_b[i] = 0x1C + s.mascara(4) as u8;
                self.mapa_r[i] = 0x1C + s.mascara(4) as u8;
            } else if py > 3 {
                self.mapa_b[i] = 0x1C + s.rand1(2) as u8;
                self.mapa_r[i] = 0x1C + s.rand1(2) as u8;
            } else {
                self.mapa_b[i] = 0x1E;
                self.mapa_r[i] = 0x1E;
            }
            self.saida_b[i] = SAIDA_ABERTA;
            self.saida_r[i] = SAIDA_ABERTA;
        }

        self.formigas_superficie(s, Cor::Preta, pretas);
        self.formigas_superficie(s, Cor::Vermelha, vermelhas);

        self.buracos_b = [0; 64];
        self.buracos_r = [0; 64];
        if self.tipo_jogo != TipoJogo::Completo || pretas > 0 {
            self.novo_buraco(s, tab, Cor::Preta, 32);
        }
        if vermelhas > 0 {
            self.novo_buraco(s, tab, Cor::Vermelha, 32);
        }
        self.escavacao_b = Escavacao::default();
        self.escavacao_r = Escavacao::default();
        self.lencol = 64;
        if pretas > 1 {
            self.escavar_ninho(s, tab, Cor::Preta, pretas * 16);
        }
        if vermelhas > 1 {
            self.escavar_ninho(s, tab, Cor::Vermelha, vermelhas * 16);
        }

        self.montar_lista_a();
        self.lista_b.clear();
        self.lista_r.clear();
        self.rainhas_pretas = 0;
        self.rainhas_vermelhas = 0;
        self.ultimo_ovo_preto = (-1, -1);
        self.ultimo_ovo_vermelho = (-1, -1);
        if vermelhas > 0 {
            self.rainha(s, tab, Cor::Vermelha);
        }
        if pretas > 0 {
            self.rainha(s, tab, Cor::Preta);
        }
        self.iniciar_amarela(s)?;

        self.comida_a = 0;
        self.comida_b = 0;
        self.comida_r = 0;
        if self.tipo_jogo != TipoJogo::Experimental {
            self.adicionar_comida(s, tab, None);
        }
        self.saude_b = 100;
        self.saude_r = 100;
        Ok(())
    }

    /// `_MakeMap`: quintal se `px * 16 + py > 37`, senão casa (não implementada).
    fn gerar_mapa(&mut self, s: &mut GeradorS, tab: &TabelasGeracao, px: i32, py: i32) -> Result<(), String> {
        let n = px * 16 + py;
        if n > 37 {
            self.gerar_quintal(s, tab);
            Ok(())
        } else {
            Err(format!("patch da casa ({px}, {py}) = {n}: _MakeHousePatch ainda não implementado"))
        }
    }

    /// `_MakeYardPatch` (geracao-mundo.md §4).
    fn gerar_quintal(&mut self, s: &mut GeradorS, tab: &TabelasGeracao) {
        self.terreno = 0;
        for v in self.mapa_a.iter_mut() {
            *v = s.mascara(16) as u8;
        }
        self.adicionar_pedras(s, tab);
        let n = s.mascara(4) + 1;
        self.iniciar_formigas_leao(s, tab, n);
        self.iniciar_bichos_de_conta(s, tab);

        let sorteio = |s: &mut GeradorS| {
            let x = s.rand1(125) + 1;
            let y = s.rand1(61) + 1;
            (x, y)
        };
        let livre = |v: u8| v < 0x18;
        for _ in 0..20 {
            let (x, y) = sorteio(s);
            let c = [ix(x, y), ix(x + 1, y), ix(x, y + 1), ix(x + 1, y + 1)];
            if c.iter().all(|&i| livre(self.mapa_a[i])) {
                for (k, &i) in c.iter().enumerate() {
                    self.mapa_a[i] = 0x20 + k as u8;
                }
            }
        }
        for _ in 0..30 {
            let (x, y) = sorteio(s);
            let (a, b) = (ix(x, y), ix(x + 1, y));
            if livre(self.mapa_a[a]) && livre(self.mapa_a[b]) {
                let v = (s.rand1(2) as u8 + 0x12) * 2;
                self.mapa_a[a] = v;
                self.mapa_a[b] = v + 1;
            }
        }
        for (vezes, valor) in [(100, 0x28), (128, PEDRINHA)] {
            for _ in 0..vezes {
                let (x, y) = sorteio(s);
                let i = ix(x, y);
                if livre(self.mapa_a[i]) {
                    self.mapa_a[i] = valor;
                }
            }
        }
    }

    /// `_AddRocks`: sorteia y antes de x.
    fn adicionar_pedras(&mut self, s: &mut GeradorS, tab: &TabelasGeracao) {
        let n = s.rand1(3) + 2;
        for _ in 0..n {
            for forma in &tab.pedras5 {
                let y = s.rand1(58);
                let x = s.rand1(122);
                self.pedra(x, y, forma, 5);
            }
        }
        for _ in 0..2 * n {
            for forma in &tab.pedras3 {
                let y = s.rand1(60);
                let x = s.rand1(124);
                self.pedra(x, y, forma, 3);
            }
        }
    }

    /// `_AddRock5/3`: descarta a pedra inteira se alguma célula da forma cair sobre `_MapA > 0x10`.
    fn pedra(&mut self, x: i32, y: i32, forma: &[u8], lado: i32) {
        let celulas = (0..lado).flat_map(|j| (0..lado).map(move |i| (i, j)));
        let ocupada = celulas.clone().any(|(i, j)| forma[(j * lado + i) as usize] != 0 && self.mapa_a[ix(x + i, y + j)] > 0x10);
        if ocupada {
            return;
        }
        for (i, j) in celulas {
            let v = forma[(j * lado + i) as usize];
            if v != 0 {
                self.mapa_a[ix(x + i, y + j)] = v;
            }
        }
    }

    /// `_IsClearTile(1, x, y)` na superfície ([`comum::livre_na_superficie`]).
    fn livre_superficie(&self, x: i32, y: i32) -> bool {
        comum::livre_na_superficie(self, x, y)
    }

    /// `_IsClear3x3(1, x, y)` ([`comum::livre_3x3`]).
    fn livre_3x3(&self, tab: &TabelasGeracao, x: i32, y: i32) -> bool {
        comum::livre_3x3(self, tab, x, y)
    }

    /// `_InitAntLions(n)`.
    fn iniciar_formigas_leao(&mut self, s: &mut GeradorS, tab: &TabelasGeracao, n: i32) {
        self.formigas_leao.clear();
        let n = n.min(10);
        for _ in 0..n.max(0) {
            let mut achou = None;
            for tentativa in 0..200 {
                let x = s.rand1(64) + s.rand1(65);
                let y = s.rand1(32) + s.rand1(33);
                if self.livre_3x3(tab, x, y) || (self.livre_superficie(x, y) && tentativa > 99) {
                    achou = Some((x, y));
                    break;
                }
            }
            let Some((x, y)) = achou else { continue };
            self.mapa_a[ix(x, y)] = FORMIGA_LEAO_CENTRO;
            for d in 0..8 {
                let (vx, vy) = tab.vizinha(x, y, d);
                if self.livre_superficie(vx, vy) {
                    self.mapa_a[ix(vx, vy)] = FORMIGA_LEAO_BORDA + tab.borda_formiga_leao[d];
                }
            }
            // A lista tem 10 posições; a partir da décima, a última é sobrescrita.
            if self.formigas_leao.len() < 10 {
                self.formigas_leao.push((x as u8, y as u8));
            } else {
                self.formigas_leao[9] = (x as u8, y as u8);
            }
        }
        self.formigas_leao_iniciais = n as i16;
    }

    /// `_InitPillar`: dois bichos-de-conta (índices 2 e 1). Sorteia até conseguir os dois.
    fn iniciar_bichos_de_conta(&mut self, s: &mut GeradorS, tab: &TabelasGeracao) {
        self.bichos_de_conta = [BichoDeConta::default(); 3];
        if self.terreno != 0 {
            return;
        }
        let mut k = 2;
        while k > 0 {
            let x = s.rand1(128);
            let y = s.rand1(64);
            let i = ix(x, y);
            if self.mapa_a[i] < 0x10 {
                let dir = s.rand1(8);
                self.bichos_de_conta[k] = BichoDeConta { x: x as i16, y: y as i16, dir: dir as i16, chao: self.mapa_a[i] as u16 };
                self.mapa_a[i] = tab.sow_tab[dir as usize];
                k -= 1;
            }
        }
    }

    /// Formigas de uma colônia na superfície (passos 3.8 e 3.9).
    fn formigas_superficie(&mut self, s: &mut GeradorS, cor: Cor, quantidade: i32) {
        let k = s.rand1(6);
        let j = s.mascara(2);
        let mut restantes = quantidade;
        let mut repetidas = 0;
        while restantes > 0 {
            restantes -= 1;
            let g = s.sg_rand(128);
            let y = s.mascara(64);
            let x = if cor == Cor::Preta { g } else { 127 - g };
            let i = ix(x, y);
            if self.mapa_a[i] >= BURACO {
                continue;
            }
            if self.vida_a[i] != 0 {
                repetidas += 1;
                if repetidas < 50 {
                    restantes += 1;
                }
                continue;
            }
            let v = s.mascara(16);
            let base: u8 = if v < k + 7 {
                0x10
            } else if v < 15 - j {
                0x30
            } else if s.mascara(2) == 0 {
                0x40
            } else {
                0x20
            };
            let t = base + s.mascara(8) as u8;
            self.vida_a[i] = if cor == Cor::Vermelha { t | 0x80 } else { t };
        }
    }

    /// `_MakeNewHoleB/R(c)` ([`comum::novo_buraco`]): buraco na superfície (linha `c`) ligado à coluna
    /// `c` do ninho.
    fn novo_buraco(&mut self, s: &mut GeradorS, tab: &TabelasGeracao, cor: Cor, c: i32) {
        comum::novo_buraco(&mut Escavando { m: self, s }, tab, &tab.borda_buraco, cor, c);
    }

    fn ninho_mut(&mut self, cor: Cor) -> &mut Vec<u8> {
        match cor {
            Cor::Preta => &mut self.mapa_b,
            Cor::Vermelha => &mut self.mapa_r,
        }
    }

    fn escavacao_mut(&mut self, cor: Cor) -> &mut Escavacao {
        match cor {
            Cor::Preta => &mut self.escavacao_b,
            Cor::Vermelha => &mut self.escavacao_r,
        }
    }

    /// `_DigTileB/R(x, y)` ([`comum::cavar`]): escava se for terra, alisa as quatro vizinhas e atualiza a
    /// saída.
    fn cavar(&mut self, s: &mut GeradorS, tab: &TabelasGeracao, cor: Cor, x: i32, y: i32) {
        comum::cavar(&mut Escavando { m: self, s }, tab, cor, x, y);
    }

    /// `_DigTileThemB/R(x, y)` ([`comum::cavar_tunel`]).
    fn cavar_tunel(&mut self, s: &mut GeradorS, tab: &TabelasGeracao, cor: Cor, x: i32, y: i32) -> bool {
        comum::cavar_tunel(&mut Escavando { m: self, s }, tab, &tab.borda_buraco, cor, x, y)
    }

    /// `_DigOutBNest/RNest(n)`: passeio aleatório a partir de (32, 1). Não usado no novo jogo.
    fn escavar_ninho(&mut self, s: &mut GeradorS, tab: &TabelasGeracao, cor: Cor, n: i32) {
        let (mut x, mut y, mut dir) = (32, 1, 4usize);
        self.cavar(s, tab, cor, 32, 1);
        for _ in 0..n {
            dir = ((dir as i32 + s.rand1(5) - 3) & 7) as usize;
            let (mut nx, mut ny) = tab.vizinha(x, y, dir);
            if nx < 1 {
                nx = 1;
                dir = 2;
            } else if nx > 62 {
                nx = 62;
                dir = 6;
            }
            if ny < 2 {
                ny = 1;
                dir = 4;
            } else if ny > 62 {
                ny = 62;
                dir = 0;
            }
            if self.cavar_tunel(s, tab, cor, nx, ny) {
                x = nx;
                y = ny;
                let buracos = if cor == Cor::Preta { &self.buracos_b } else { &self.buracos_r };
                if y == 1 && buracos[x as usize] == 0 {
                    self.novo_buraco(s, tab, cor, x);
                }
            }
        }
    }

    /// Rainha (§3.C): túnel de `Sk(4) + 7` linhas a partir de (32, 1), dois passos em diagonal e a câmara.
    fn rainha(&mut self, s: &mut GeradorS, tab: &TabelasGeracao, cor: Cor) {
        let mut x = 32;
        let mut y = 1;
        let profundidade = s.mascara(4) + 7;
        let mut deriva = 0;
        while y < profundidade {
            self.cavar(s, tab, cor, x, y);
            match cor {
                // Vermelha: deriva nova a cada linha.
                Cor::Vermelha => deriva = s.rand1(3) - 1,
                // Preta: só muda a deriva quando Sk(2) = 0.
                Cor::Preta => {
                    if s.mascara(2) == 0 {
                        deriva = s.rand1(3) - 1;
                    }
                }
            }
            let t = x + deriva;
            if 7 < t && t < 57 {
                x = t;
            }
            y += 1;
        }
        for _ in 0..2 {
            self.cavar(s, tab, cor, x, y);
            x += 1;
            y += 1;
        }
        self.cavar(s, tab, cor, x, y);
        // A rainha fica 2 à direita da câmara, virada para a direita; a segunda metade atrás dela.
        let rx = x + 2;
        let (bx, by) = tab.vizinha(rx, y, 6);
        let (cx, cy) = (rx + 2 * tab.dx8[6] as i32, y + 2 * tab.dy8[6] as i32);
        self.cavar(s, tab, cor, rx, y);
        self.cavar(s, tab, cor, bx, by);
        self.cavar(s, tab, cor, cx, cy);
        // T = 0x62 (rainha, direção 2) e 0x6A (segunda metade); bit 7 nas vermelhas.
        let cor_bit = if cor == Cor::Vermelha { 0x80 } else { 0 };
        self.adicionar_na_lista(cor, rx, y, 0x62 | cor_bit);
        self.adicionar_na_lista(cor, bx, by, 0x6A | cor_bit);
        match cor {
            Cor::Preta => {
                self.ultimo_ovo_preto = (x as i16, y as i16);
                self.renascer = (x as i16, y as i16);
                self.rainhas_pretas += 1;
            }
            Cor::Vermelha => {
                self.ultimo_ovo_vermelho = (x as i16, y as i16);
                self.rainhas_vermelhas += 1;
            }
        }
    }

    /// `_AddAntToBList/RList`: M = 9 (rainha), S = 0; marca `_LifeB/R`. Até 500 por lista.
    fn adicionar_na_lista(&mut self, cor: Cor, x: i32, y: i32, tipo: u8) {
        let (lista, vida) = match cor {
            Cor::Preta => (&mut self.lista_b, &mut self.vida_b),
            Cor::Vermelha => (&mut self.lista_r, &mut self.vida_r),
        };
        if lista.len() < 500 {
            lista.push(Formiga { x: x as u8, y: y as u8, modo: 9, tipo, s: 0 });
            vida[ix(x, y)] = tipo;
        }
    }

    /// `_BuildAntListA`: varre `_LifeA` (x, depois y) e põe cada formiga (menos a amarela) na lista A.
    fn montar_lista_a(&mut self) {
        self.lista_a.clear();
        for x in 0..LARGURA_SUPERFICIE {
            for y in 0..LADO {
                let t = self.vida_a[ix(x, y)];
                if t == 0 || t == VIDA_AMARELA || t == VIDA_AMARELA_2 {
                    continue;
                }
                let f = Formiga { x: x as u8, y: y as u8, modo: 2, tipo: t, s: 0 };
                // O original grava na posição atual e só avança enquanto o índice for < 997.
                if self.lista_a.len() <= 997 {
                    self.lista_a.push(f);
                } else {
                    *self.lista_a.last_mut().unwrap() = f;
                }
            }
        }
    }

    /// `_InitYelloAnt` (§3.D).
    fn iniciar_amarela(&mut self, s: &mut GeradorS) -> Result<(), String> {
        self.amarela = FormigaAmarela { saude: 100, ovo_carregado: 0xFD, ..FormigaAmarela::default() };
        if self.tipo_jogo == TipoJogo::Experimental {
            self.amarela.plano = 1;
            self.amarela.modo = 2;
            self.amarela.x = 0x40;
            self.amarela.y = 0x20;
            self.amarela.tipo = 0x10;
            return Ok(());
        }
        let (plano, x, y, tipo) = if self.tipo_jogo == TipoJogo::Completo && !self.ninho_iniciado {
            let mut pos = (0x40, 0x20);
            for _ in 0..100 {
                let x = 0x20 + s.mascara(16) - s.mascara(16);
                let y = 0x20 + s.mascara(8) - s.mascara(8);
                if self.mapa_a[ix(x, y)] < 0x10 {
                    pos = (x, y);
                    break;
                }
            }
            (1, pos.0, pos.1, 0x40)
        } else {
            (2, self.renascer.0 as i32, self.renascer.1 as i32, 0x10)
        };
        // `_SetMyLife`: no ninho, uma célula de terra seria escavada (`_DigMyTile`, não especificado);
        // na geração a formiga nasce na câmara da rainha, já escavada.
        if plano >= 2 {
            let v = self.mapa_b[ix(x, y)];
            if e_terra(v) || (0x1C..=0x1F).contains(&v) {
                return Err(format!("formiga amarela em ({x}, {y}) cairia em terra: _DigMyTile não implementado"));
            }
        }
        let vida = if plano < 2 { &mut self.vida_a } else { &mut self.vida_b };
        vida[ix(x, y)] = VIDA_AMARELA;
        self.amarela.plano = plano;
        self.amarela.x = x as i16;
        self.amarela.y = y as i16;
        self.amarela.tipo = tipo;
        self.amarela.dir = 2;
        Ok(())
    }

    /// `_AddFood(param, _)` (§3.E). `centro = None` é o `param < 0` do novo jogo (200 unidades em
    /// torno de (64, 8…55)); `Some(n)` sorteia o centro e espalha `n` unidades.
    pub fn adicionar_comida(&mut self, s: &mut GeradorS, tab: &TabelasGeracao, quantidade: Option<i32>) {
        let (cx, cy, n) = match quantidade {
            None => (0x40, s.rand1(0x30) + 8, 200),
            Some(n) => {
                let cx = s.mascara(128);
                let cy = s.mascara(64);
                (cx, cy, n)
            }
        };
        self.ultima_comida = (cx as i16, cy as i16);
        let raio = s.mascara(8) as u16 + 5;
        for _ in 0..n {
            let a = s.mascara(256);
            let r = s.rand1(raio);
            let x = cx + tab.cosseno(a) * r / 0x7FFF;
            let y = cy + tab.seno(a) * r / 0x7FFF;
            if !na_superficie(x, y) || self.vida_a[ix(x, y)] != 0 {
                continue;
            }
            let i = ix(x, y);
            let v = self.mapa_a[i];
            let novo = if self.terreno == 0 {
                match v {
                    0..=0x17 => Some(COMIDA),
                    0x48..=0x4A => Some(v + 1),
                    _ => None,
                }
            } else {
                match v {
                    0..=3 => Some((v + 6) * 4),
                    4..=0x17 => Some((v.wrapping_sub(8) & 0xFC).wrapping_add(0x18)),
                    0x18..=0x27 if v & 3 < 3 => Some(v + 1),
                    _ => None,
                }
            };
            if let Some(nv) = novo {
                self.mapa_a[i] = nv;
                self.comida_a += 1;
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Tabelas mínimas para testes sem os arquivos do jogo: direções reais e seno aproximado.
    fn tabelas_de_teste() -> TabelasGeracao {
        let seno: Vec<u8> = (0..64)
            .flat_map(|i| ((((i as f64) * std::f64::consts::PI / 128.0).sin() * 32767.0).round() as i16).to_be_bytes())
            .collect();
        let json = r#"{"dx8":[0,1,1,1,0,-1,-1,-1],"dy8":[-1,-1,0,1,1,1,0,-1],
            "borda_buraco":[25,26,28,31,30,29,27,24],"borda_formiga_leao":[1,2,4,7,6,5,3,0],
            "sow_tab":[114,115,113,112,114,115,113,112],
            "pedras5":[[0,0,0,0,0,0,0,0,0,0,0,0,92,0,0,0,0,0,0,0,0,0,0,0,0],
                       [92,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                       [92,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                       [92,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                       [92,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                       [92,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]],
            "pedras3":[[92,0,0,0,0,0,0,0,0],[92,0,0,0,0,0,0,0,0],[92,0,0,0,0,0,0,0,0],
                       [92,0,0,0,0,0,0,0,0],[92,0,0,0,0,0,0,0,0],[92,0,0,0,0,0,0,0,0]]}"#;
        TabelasGeracao::de_partes(json, &seno).unwrap()
    }

    #[test]
    fn seno_e_cosseno_nos_quadrantes() {
        let t = tabelas_de_teste();
        assert_eq!(t.seno(0), 0);
        assert_eq!(t.seno(64), 0x7FFF);
        assert_eq!(t.seno(128), 0);
        assert_eq!(t.seno(192), -0x7FFF);
        assert_eq!(t.cosseno(0), 0x7FFF);
        assert_eq!(t.cosseno(128), -0x7FFF);
        assert_eq!(t.seno(32), t.seno(96));
        assert_eq!(t.seno(160), -t.seno(32));
    }

    #[test]
    fn alisar_marca_vizinhas_de_terra() {
        let t = tabelas_de_teste();
        let mut m = Mundo::vazio(TipoJogo::Rapido);
        m.mapa_b.fill(TERRA_MACICA);
        let mut s = GeradorS::new(0x1234);
        m.cavar(&mut s, &t, Cor::Preta, 10, 10);
        assert!(m.mapa_b[ix(10, 10)] < 8, "célula escavada vira túnel 0–7");
        // Vizinha de cima: tem terra à direita, embaixo não (túnel), à esquerda e em cima: 1|2|8.
        assert_eq!(m.mapa_b[ix(10, 9)], 0x1F + (1 | 2 | 8));
        assert_eq!(m.mapa_b[ix(11, 10)], 0x1F + (1 | 2 | 4));
        assert_eq!(m.mapa_b[ix(10, 11)], 0x1F + (2 | 4 | 8));
        assert_eq!(m.mapa_b[ix(9, 10)], 0x1F + (1 | 4 | 8));
        assert_eq!(m.escavacao_b, Escavacao { tot_x: 10, tot_y: 10, cavados: 1, massa_x: 10, massa_y: 10 });
    }

    #[test]
    fn mesma_semente_mesmo_mundo() {
        let t = tabelas_de_teste();
        let gerar = || {
            let mut r = GeradorR::default();
            let mut s = GeradorS::new(0x4321);
            Mundo::novo_jogo(TipoJogo::Rapido, &mut r, &mut s, &t).unwrap()
        };
        let (a, b) = (gerar(), gerar());
        assert_eq!(a.mapa_a, b.mapa_a);
        assert_eq!(a.mapa_b, b.mapa_b);
        assert_eq!(a.lista_b.len(), 2);
        assert_eq!(a.lista_r.len(), 2);
        assert_eq!(a.amarela.plano, 2);
        assert_eq!(a.vida_b[ix(a.renascer.0 as i32, a.renascer.1 as i32)], VIDA_AMARELA);
        // Sementes de patch ficam em 0…0x7FFE.
        assert!(a.sementes.iter().all(|&v| v < 0x7FFF));
    }

    #[test]
    fn casa_ainda_nao_implementada() {
        let t = tabelas_de_teste();
        let mut m = Mundo::vazio(TipoJogo::Completo);
        let mut s = GeradorS::new(1);
        assert!(m.gerar_patch(&mut s, &t, 1, 1, 1, 0, 0).is_err());
    }
}
