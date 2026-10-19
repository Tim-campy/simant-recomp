//! Estado completo da simulação: tudo o que o `.ANT` guarda, tipado por assunto, mais o que só existe
//! durante a execução (geradores, mapas de vida, contagens do tick, eventos para a interface).
//!
//! Especificação: docs/spec/regras/simulacao-visao-geral.md (seção "Implementação no port"),
//! docs/spec/formatos/save-ant.md (tabela de blocos) e formigas.md (listas, censo).
//!
//! Cada campo que vem do save traz, na declaração, o símbolo original (sem o `_`); a ordem e o tamanho
//! dos blocos continuam vindo de `data/save_layout.json`. A conversão é feita por nome de símbolo:
//! [`Estado::de_save`] e [`Estado::para_save`] fazem ida e volta byte a byte.
//!
//! Os valores iniciais que não saem da geração do mundo (controles, velocidade, opções...) ficam em
//! `data/simulacao.json` ([`ConfigSimulacao`]), não aqui.

use std::ops::{Deref, DerefMut};

use serde::Deserialize;

use super::geracao::{Cor, Mundo, TipoJogo};
use super::rng::{GeradorR, GeradorS};
use crate::core::vida;
use crate::formats::save::{Formiga, JogoSalvo, LayoutSave, Lista};
use crate::recursos::Recursos;

/// Arquivo de dados com os valores iniciais e a tabela de velocidades.
pub const ARQUIVO_CONFIG: &str = "simulacao.json";

// ---------------------------------------------------------------------------------------------
// Conversão campo <-> bloco do save
// ---------------------------------------------------------------------------------------------

/// Um tipo que sabe ler e gravar a si mesmo num bloco do save (little-endian, tamanho conferido).
pub trait CampoSave: Sized {
    fn ler(j: &JogoSalvo, simbolo: &str) -> Result<Self, String>;
    fn gravar(&self, j: &mut JogoSalvo, simbolo: &str) -> Result<(), String>;
}

fn bloco<'a>(j: &'a JogoSalvo, simbolo: &str, n: usize) -> Result<&'a [u8], String> {
    let b = j.bloco(simbolo).ok_or_else(|| format!("bloco {simbolo} não existe no layout"))?;
    if b.len() != n {
        return Err(format!("bloco {simbolo} tem {} bytes; o estado espera {n}", b.len()));
    }
    Ok(b)
}

fn bloco_mut<'a>(j: &'a mut JogoSalvo, simbolo: &str, n: usize) -> Result<&'a mut [u8], String> {
    let b = j.bloco_mut(simbolo).ok_or_else(|| format!("bloco {simbolo} não existe no layout"))?;
    if b.len() != n {
        return Err(format!("bloco {simbolo} tem {} bytes; o estado espera {n}", b.len()));
    }
    Ok(b)
}

/// Elemento de vetor do save: byte ou palavra.
pub trait Elemento: Copy + Default {
    const BYTES: usize;
    fn de_le(b: &[u8]) -> Self;
    fn para_le(self, b: &mut [u8]);
}

impl Elemento for u8 {
    const BYTES: usize = 1;
    fn de_le(b: &[u8]) -> u8 {
        b[0]
    }
    fn para_le(self, b: &mut [u8]) {
        b[0] = self;
    }
}

impl Elemento for i16 {
    const BYTES: usize = 2;
    fn de_le(b: &[u8]) -> i16 {
        i16::from_le_bytes([b[0], b[1]])
    }
    fn para_le(self, b: &mut [u8]) {
        b[..2].copy_from_slice(&self.to_le_bytes());
    }
}

impl Elemento for u16 {
    const BYTES: usize = 2;
    fn de_le(b: &[u8]) -> u16 {
        u16::from_le_bytes([b[0], b[1]])
    }
    fn para_le(self, b: &mut [u8]) {
        b[..2].copy_from_slice(&self.to_le_bytes());
    }
}

fn ler_elementos<T: Elemento>(j: &JogoSalvo, simbolo: &str, saida: &mut [T]) -> Result<(), String> {
    let b = bloco(j, simbolo, saida.len() * T::BYTES)?;
    for (v, c) in saida.iter_mut().zip(b.chunks(T::BYTES)) {
        *v = T::de_le(c);
    }
    Ok(())
}

fn gravar_elementos<T: Elemento>(j: &mut JogoSalvo, simbolo: &str, entrada: &[T]) -> Result<(), String> {
    let b = bloco_mut(j, simbolo, entrada.len() * T::BYTES)?;
    for (v, c) in entrada.iter().zip(b.chunks_mut(T::BYTES)) {
        v.para_le(c);
    }
    Ok(())
}

macro_rules! campo_escalar {
    ($t:ty) => {
        impl CampoSave for $t {
            fn ler(j: &JogoSalvo, simbolo: &str) -> Result<Self, String> {
                let mut v = [<$t>::default()];
                ler_elementos(j, simbolo, &mut v)?;
                Ok(v[0])
            }
            fn gravar(&self, j: &mut JogoSalvo, simbolo: &str) -> Result<(), String> {
                gravar_elementos(j, simbolo, &[*self])
            }
        }
    };
}
campo_escalar!(i16);
campo_escalar!(u16);

/// Blocos de 4 bytes "4×1" que são inteiros de 32 bits (`_GameTime`, contadores de mortes...).
impl CampoSave for i32 {
    fn ler(j: &JogoSalvo, simbolo: &str) -> Result<i32, String> {
        let b = bloco(j, simbolo, 4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn gravar(&self, j: &mut JogoSalvo, simbolo: &str) -> Result<(), String> {
        bloco_mut(j, simbolo, 4)?.copy_from_slice(&self.to_le_bytes());
        Ok(())
    }
}

impl CampoSave for u32 {
    fn ler(j: &JogoSalvo, simbolo: &str) -> Result<u32, String> {
        Ok(i32::ler(j, simbolo)? as u32)
    }
    fn gravar(&self, j: &mut JogoSalvo, simbolo: &str) -> Result<(), String> {
        (*self as i32).gravar(j, simbolo)
    }
}

/// Blocos de 4 bytes que são um par de palavras (x, y): os `*Pnt`, `_Last*Egg`, `_LastNew*`...
impl CampoSave for (i16, i16) {
    fn ler(j: &JogoSalvo, simbolo: &str) -> Result<(i16, i16), String> {
        let mut v = [0i16; 2];
        ler_elementos(j, simbolo, &mut v)?;
        Ok((v[0], v[1]))
    }
    fn gravar(&self, j: &mut JogoSalvo, simbolo: &str) -> Result<(), String> {
        gravar_elementos(j, simbolo, &[self.0, self.1])
    }
}

impl<T: Elemento, const N: usize> CampoSave for [T; N] {
    fn ler(j: &JogoSalvo, simbolo: &str) -> Result<Self, String> {
        let mut v = [T::default(); N];
        ler_elementos(j, simbolo, &mut v)?;
        Ok(v)
    }
    fn gravar(&self, j: &mut JogoSalvo, simbolo: &str) -> Result<(), String> {
        gravar_elementos(j, simbolo, self)
    }
}

/// Vetor de tamanho fixo com `Default` para qualquer N (a biblioteca padrão só cobre até 32).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tabela<T, const N: usize>(pub [T; N]);

impl<T: Copy + Default, const N: usize> Default for Tabela<T, N> {
    fn default() -> Self {
        Tabela([T::default(); N])
    }
}

impl<T, const N: usize> Deref for Tabela<T, N> {
    type Target = [T; N];
    fn deref(&self) -> &[T; N] {
        &self.0
    }
}

impl<T, const N: usize> DerefMut for Tabela<T, N> {
    fn deref_mut(&mut self) -> &mut [T; N] {
        &mut self.0
    }
}

impl<T: Elemento, const N: usize> CampoSave for Tabela<T, N> {
    fn ler(j: &JogoSalvo, simbolo: &str) -> Result<Self, String> {
        Ok(Tabela(<[T; N]>::ler(j, simbolo)?))
    }
    fn gravar(&self, j: &mut JogoSalvo, simbolo: &str) -> Result<(), String> {
        self.0.gravar(j, simbolo)
    }
}

/// Declara um grupo de campos do save: struct pública, lista de símbolos e leitura/gravação.
macro_rules! grupo_do_save {
    ($(#[$m:meta])* $nome:ident { $( $(#[$fm:meta])* $campo:ident : $tipo:ty = $simbolo:literal ),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        pub struct $nome {
            $( $(#[$fm])* pub $campo: $tipo, )*
        }

        impl $nome {
            /// Símbolos do save (sem o `_`) cobertos por este grupo, na ordem dos campos.
            pub const SIMBOLOS: &'static [&'static str] = &[$($simbolo),*];

            fn ler(j: &JogoSalvo) -> Result<Self, String> {
                Ok(Self { $( $campo: CampoSave::ler(j, $simbolo)?, )* })
            }

            fn gravar(&self, j: &mut JogoSalvo) -> Result<(), String> {
                $( self.$campo.gravar(j, $simbolo)?; )*
                Ok(())
            }
        }
    };
}

/// Como [`grupo_do_save`], para os grupos espelhados por cor: cada campo tem o símbolo da colônia
/// preta e o da vermelha (`"HealthB" | "HealthR"`).
macro_rules! grupo_por_cor {
    ($(#[$m:meta])* $nome:ident { $( $(#[$fm:meta])* $campo:ident : $tipo:ty = $preta:literal | $vermelha:literal ),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        pub struct $nome {
            $( $(#[$fm])* pub $campo: $tipo, )*
        }

        impl $nome {
            /// Símbolos da colônia preta, na ordem dos campos.
            pub const SIMBOLOS_PRETA: &'static [&'static str] = &[$($preta),*];
            /// Símbolos da colônia vermelha, na ordem dos campos.
            pub const SIMBOLOS_VERMELHA: &'static [&'static str] = &[$($vermelha),*];

            fn ler(j: &JogoSalvo, cor: Cor) -> Result<Self, String> {
                Ok(Self { $( $campo: CampoSave::ler(j, match cor { Cor::Preta => $preta, Cor::Vermelha => $vermelha })?, )* })
            }

            fn gravar(&self, j: &mut JogoSalvo, cor: Cor) -> Result<(), String> {
                $( self.$campo.gravar(j, match cor { Cor::Preta => $preta, Cor::Vermelha => $vermelha })?; )*
                Ok(())
            }
        }
    };
}

// ---------------------------------------------------------------------------------------------
// Mapas e listas
// ---------------------------------------------------------------------------------------------

/// Superfície: 128 × 64 células.
pub const SUPERFICIE: (usize, usize) = (128, 64);
/// Ninhos: 64 × 64 células (y = profundidade, 0 = linha da superfície).
pub const NINHO: (usize, usize) = (64, 64);
/// Mapas de feromônio: 64 × 32, uma célula por bloco de 2 × 2 tiles da superfície.
pub const FEROMONIO: (usize, usize) = (64, 32);
/// Capacidade das listas: 1000 na superfície, 500 em cada ninho.
pub const CAPACIDADE_A: usize = 1000;
pub const CAPACIDADE_NINHO: usize = 500;

/// Mapa de bytes guardado coluna a coluna, como no original: índice `x * altura + y`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grade {
    pub largura: usize,
    pub altura: usize,
    pub dados: Vec<u8>,
}

impl Grade {
    pub fn nova((largura, altura): (usize, usize)) -> Grade {
        Grade { largura, altura, dados: vec![0; largura * altura] }
    }

    /// Grade a partir de bytes já em ordem coluna-major (tamanho conferido).
    pub fn de_bytes(dim: (usize, usize), dados: Vec<u8>) -> Result<Grade, String> {
        if dados.len() != dim.0 * dim.1 {
            return Err(format!("grade {}×{} com {} bytes", dim.0, dim.1, dados.len()));
        }
        Ok(Grade { largura: dim.0, altura: dim.1, dados })
    }

    pub fn contem(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.largura && (y as usize) < self.altura
    }

    pub fn indice(&self, x: usize, y: usize) -> usize {
        x * self.altura + y
    }

    /// Valor em (x, y), ou `None` fora do mapa.
    pub fn em(&self, x: i32, y: i32) -> Option<u8> {
        self.contem(x, y).then(|| self.dados[self.indice(x as usize, y as usize)])
    }

    /// Grava `v` em (x, y) se a célula existe; devolve se gravou.
    pub fn definir(&mut self, x: i32, y: i32, v: u8) -> bool {
        if !self.contem(x, y) {
            return false;
        }
        let i = self.indice(x as usize, y as usize);
        self.dados[i] = v;
        true
    }

    fn ler(j: &JogoSalvo, simbolo: &str, dim: (usize, usize)) -> Result<Grade, String> {
        Grade::de_bytes(dim, bloco(j, simbolo, dim.0 * dim.1)?.to_vec())
    }

    fn gravar(&self, j: &mut JogoSalvo, simbolo: &str) -> Result<(), String> {
        bloco_mut(j, simbolo, self.dados.len())?.copy_from_slice(&self.dados);
        Ok(())
    }
}

/// Uma lista de formigas (`_?listX/Y/M/T/S` + `_ListIndex?`), com **todas** as vagas da capacidade.
///
/// As vagas a partir de `indice` guardam lixo de formigas antigas; o original as grava no save, e
/// o port as mantém para que a ida e volta seja byte a byte. Uma vaga com T = 0 dentro de
/// `0..indice` é um buraco que só some na compactação.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListaFormigas {
    pub vagas: Vec<Formiga>,
    /// `_ListIndex?`: número de vagas em uso (o próximo `_AddAntTo?List` grava aqui).
    pub indice: i16,
}

impl ListaFormigas {
    pub fn vazia(capacidade: usize) -> ListaFormigas {
        ListaFormigas { vagas: vec![Formiga { x: 0, y: 0, modo: 0, tipo: 0, s: 0 }; capacidade], indice: 0 }
    }

    pub fn capacidade(&self) -> usize {
        self.vagas.len()
    }

    /// Vagas em uso (`_ListIndex?`), limitado a 0..capacidade.
    pub fn usadas(&self) -> usize {
        (self.indice.max(0) as usize).min(self.capacidade())
    }

    pub fn cheia(&self) -> bool {
        self.usadas() >= self.capacidade()
    }

    /// Vagas em uso com T ≠ 0, com o índice.
    pub fn ativas(&self) -> impl Iterator<Item = (usize, &Formiga)> {
        self.vagas[..self.usadas()].iter().enumerate().filter(|(_, f)| f.tipo != 0)
    }

    /// `_AddAntTo?List` sem o mapa de vida: grava na vaga `indice` e incrementa. Lista cheia:
    /// descarta em silêncio e devolve `false` (formigas.md §2).
    pub fn adicionar(&mut self, f: Formiga) -> bool {
        if self.cheia() {
            return false;
        }
        let i = self.usadas();
        self.vagas[i] = f;
        self.indice = i as i16 + 1;
        true
    }

    /// `_CompactListA/B/R`: desliza as vagas com T ≠ 0 para a frente, mantendo a ordem, e diminui
    /// `indice`. As vagas que sobram no fim não são apagadas (como no original).
    pub fn compactar(&mut self) {
        let n = self.usadas();
        let mut destino = 0;
        for i in 0..n {
            if self.vagas[i].tipo != 0 {
                if destino != i {
                    self.vagas[destino] = self.vagas[i];
                }
                destino += 1;
            }
        }
        self.indice -= (n - destino) as i16;
    }

    fn ler(j: &JogoSalvo, lista: Lista) -> Result<ListaFormigas, String> {
        let cap = j.capacidade(lista)?;
        let campo = |c: char| -> Result<&[u8], String> { bloco(j, &lista.simbolo(c), cap) };
        let (xs, ys, ms, ts, ss) = (campo('X')?, campo('Y')?, campo('M')?, campo('T')?, campo('S')?);
        let vagas = (0..cap).map(|i| Formiga { x: xs[i], y: ys[i], modo: ms[i], tipo: ts[i], s: ss[i] }).collect();
        Ok(ListaFormigas { vagas, indice: i16::ler(j, &lista.simbolo_contador())? })
    }

    fn gravar(&self, j: &mut JogoSalvo, lista: Lista) -> Result<(), String> {
        let cap = self.capacidade();
        type Leitor = fn(&Formiga) -> u8;
        let campos: [(char, Leitor); 5] =
            [('X', |f| f.x), ('Y', |f| f.y), ('M', |f| f.modo), ('T', |f| f.tipo), ('S', |f| f.s)];
        for (c, valor) in campos {
            let b = bloco_mut(j, &lista.simbolo(c), cap)?;
            for (d, f) in b.iter_mut().zip(&self.vagas) {
                *d = valor(f);
            }
        }
        self.indice.gravar(j, &lista.simbolo_contador())
    }
}

/// Superfície do patch atual: terreno, vida, lista A.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Superficie {
    /// `MapA`: terreno (valores em tiles-valores.md).
    pub mapa: Grade,
    /// `_LifeA` (não salvo): T da formiga em cada célula, 0xFF/0xFE = amarela.
    pub vida: Grade,
    /// Lista A (`AlistX..S`, `ListIndexA`): formigas da superfície, das duas cores.
    pub lista: ListaFormigas,
}

/// Um ninho: terreno, mapa de saída, vida e a lista das formigas que estão dentro (de qualquer cor).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ninho {
    /// `MapB` / `MapR`.
    pub mapa: Grade,
    /// `ExitMapB` / `ExitMapR`: gradiente até a saída (ninho.md §7.4).
    pub saida: Grade,
    /// `_LifeB` / `_LifeR` (não salvo).
    pub vida: Grade,
    /// Lista B / R.
    pub lista: ListaFormigas,
}

impl Ninho {
    fn vazio() -> Ninho {
        Ninho {
            mapa: Grade::nova(NINHO),
            saida: Grade::nova(NINHO),
            vida: Grade::nova(NINHO),
            lista: ListaFormigas::vazia(CAPACIDADE_NINHO),
        }
    }
}

/// Os mapas de cheiro (feromonios.md §1). Todos 64 × 32, índice `(x >> 1) * 32 + (y >> 1)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Feromonios {
    /// `PherMapA`: alarme, comum às duas cores.
    pub alarme: Grade,
    /// `PherMapBN`: cheiro de ninho preto.
    pub ninho_b: Grade,
    /// `PherMapBT`: trilha preta.
    pub trilha_b: Grade,
    /// `PherMapRN`.
    pub ninho_r: Grade,
    /// `PherMapRT`.
    pub trilha_r: Grade,
    /// `_PherMapF` (não salvo): sexto mapa, zerado por `_ClrArrays` e lavado pela chuva
    /// (geracao-mundo.md §2, ninho.md §8). [em aberto] quem escreve nele e para quê.
    pub f: Grade,
}

impl Feromonios {
    fn vazios() -> Feromonios {
        let g = || Grade::nova(FEROMONIO);
        Feromonios { alarme: g(), ninho_b: g(), trilha_b: g(), ninho_r: g(), trilha_r: g(), f: g() }
    }

    /// Cheiro de ninho da cor.
    pub fn ninho(&self, cor: Cor) -> &Grade {
        match cor {
            Cor::Preta => &self.ninho_b,
            Cor::Vermelha => &self.ninho_r,
        }
    }

    pub fn ninho_mut(&mut self, cor: Cor) -> &mut Grade {
        match cor {
            Cor::Preta => &mut self.ninho_b,
            Cor::Vermelha => &mut self.ninho_r,
        }
    }

    /// Trilha da cor.
    pub fn trilha(&self, cor: Cor) -> &Grade {
        match cor {
            Cor::Preta => &self.trilha_b,
            Cor::Vermelha => &self.trilha_r,
        }
    }

    pub fn trilha_mut(&mut self, cor: Cor) -> &mut Grade {
        match cor {
            Cor::Preta => &mut self.trilha_b,
            Cor::Vermelha => &mut self.trilha_r,
        }
    }

    /// Célula do mapa de cheiro que contém o tile (x, y) da superfície.
    pub fn celula(x: i32, y: i32) -> (i32, i32) {
        (x >> 1, y >> 1)
    }
}

// ---------------------------------------------------------------------------------------------
// Grupos de escalares do save
// ---------------------------------------------------------------------------------------------

grupo_por_cor! {
    /// Estado de uma colônia (preta = sufixo B, vermelha = sufixo R).
    Colonia {
        /// `_HealthB/R`: saúde da colônia, 0..100.
        saude: i16 = "HealthB" | "HealthR",
        /// `_FoodB/R`: unidades de comida guardadas no ninho.
        comida: i16 = "FoodB" | "FoodR",
        /// `_EatCountB/R`: reserva de alimentação (comer soma 5; cada ovo tira 1).
        reserva_comer: i16 = "EatCountB" | "EatCountR",
        /// `_BpopT/_RpopT`: adultos (soma de `castas[1..=5]`), recalculado por `_CountAnts`.
        populacao: i16 = "BpopT" | "RpopT",
        /// `_CastePopB/R`: [ovos, operárias, soldados, machos, rainhas novas, rainha].
        castas: [i16; 6] = "CastePopB" | "CastePopR",
        /// `_ModePopB/R`: [forragear, cavar, cuidar, ataque, defender ninho, defesa].
        modos: [i16; 6] = "ModePopB" | "ModePopR",
        /// `_StrategicModeB/R` (0..5, simulacao-visao-geral.md §7).
        modo_estrategico: i16 = "StrategicModeB" | "StrategicModeR",
        /// `_FlyAwayB/R`: contagem regressiva da revoada.
        revoada: i16 = "FlyAwayB" | "FlyAwayR",
        /// `_SwarmCntB/R`.
        enxame_contagem: i16 = "SwarmCntB" | "SwarmCntR",
        /// `_SwarmDelayB/R`.
        enxame_atraso: i16 = "SwarmDelayB" | "SwarmDelayR",
        /// `_SwarmXB/R` (50 bytes).
        enxame_x: Tabela<u8, 50> = "SwarmXB" | "SwarmXR",
        /// `_SwarmYB/R` (50 bytes).
        enxame_y: Tabela<u8, 50> = "SwarmYB" | "SwarmYR",
        /// `_QueenStorageB/R`.
        rainhas_guardadas: i16 = "QueenStorageB" | "QueenStorageR",
        /// `_BlkQueens/_RedQueens`: rainhas vivas no patch.
        rainhas: i16 = "BlkQueens" | "RedQueens",
        /// `_BAntsEaten/_RAntsEaten` (32 bits): comidas por predadores.
        comidas_por_predadores: i32 = "BAntsEaten" | "RAntsEaten",
        /// `_BAntsExpired/_RAntsExpired` (32 bits): mortes naturais.
        mortes_naturais: i32 = "BAntsExpired" | "RAntsExpired",
        /// `_BAntsKilled/_RAntsKilled` (32 bits): mortas em luta.
        mortas_em_luta: i32 = "BAntsKilled" | "RAntsKilled",
        /// `_BlackLost/_RedLost`: lutas perdidas (pontuação).
        perdas: i16 = "BlackLost" | "RedLost",
        /// `_BColoniesStarted/_RColoniesStarted`.
        colonias_fundadas: i16 = "BColoniesStarted" | "RColoniesStarted",
        /// `_BColoniesKilled/_RColoniesKilled`.
        colonias_extintas: i16 = "BColoniesKilled" | "RColoniesKilled",
        /// `_ColonyTotalBlack/_ColonyTotalRed`: colônias no quintal.
        total_colonias: i16 = "ColonyTotalBlack" | "ColonyTotalRed",
        /// `_TileTotXB/R` (32 bits): soma dos x escavados.
        soma_x_escavada: i32 = "TileTotXB" | "TileTotXR",
        /// `_TileTotYB/R` (32 bits).
        soma_y_escavada: i32 = "TileTotYB" | "TileTotYR",
        /// `_TilesDugB/R`.
        tiles_escavados: i16 = "TilesDugB" | "TilesDugR",
        /// `_TileMassXB/R`: centro de massa da escavação (alvo da rainha).
        centro_x: i16 = "TileMassXB" | "TileMassXR",
        /// `_TileMassYB/R`.
        centro_y: i16 = "TileMassYB" | "TileMassYR",
        /// `_LastNewHoleB/R`: (x, y) do último buraco na superfície.
        ultimo_buraco: (i16, i16) = "LastNewHoleB" | "LastNewHoleR",
        /// `_LastNewExitB/R`: (coluna, 0) da última saída no ninho.
        ultima_saida: (i16, i16) = "LastNewExitB" | "LastNewExitR",
        /// `_LastBlackEgg/_LastRedEgg`: onde o último ovo foi posto.
        ultimo_ovo: (i16, i16) = "LastBlackEgg" | "LastRedEgg",
        /// `_HoleMapB/R`: por linha y da superfície (= coluna do ninho), o x do buraco (0 = nenhum).
        buracos: Tabela<u8, 64> = "HoleMapB" | "HoleMapR",
        /// `_YMapPopB/R`: população por patch do quintal, índice `x * 16 + y`.
        populacao_quintal: Tabela<u8, 192> = "YMapPopB" | "YMapPopR",
        /// `_BMapPnt/_RMapPnt`: ponto lembrado da vista do ninho.
        vista_ninho: (i16, i16) = "BMapPnt" | "RMapPnt",
    }
}

grupo_do_save! {
    /// Dados gerais da partida.
    Partida {
        /// `_GameTime` (32 bits): ticks desde o começo.
        tempo: u32 = "GameTime",
        /// `_Cycle`: 0..0x1000, base das periodicidades.
        ciclo: i16 = "Cycle",
        /// `_CurGameType`: 0 Tutorial, 1 Quick, 2 Full, 3 Experimental.
        tipo_jogo: i16 = "CurGameType",
        /// `_GamePaused`.
        pausado: i16 = "GamePaused",
        /// `_GameSpeed`: índice em `atrasos_velocidade` (0 slow … 3 ultra).
        velocidade: i16 = "GameSpeed",
        /// `_IsGameOver`.
        fim_de_jogo: i16 = "IsGameOver",
        /// `_BlackWon`.
        pretas_venceram: i16 = "BlackWon",
        /// `_OverallScore` (32 bits).
        pontuacao: i32 = "OverallScore",
        /// `_Invincible`: a vermelha sempre perde a luta.
        invencivel: i16 = "Invincible",
        /// `_NeverHungry`.
        nunca_com_fome: i16 = "NeverHungry",
        /// `_AlwaysHealthy`: `_HealthB` não cai.
        sempre_saudavel: i16 = "AlwaysHealthy",
        /// `_JustXfered`: houve troca de corpo/patch desde a última contagem.
        acabou_de_trocar: i16 = "JustXfered",
        /// `_CurGameTool` (−1 = nenhuma; 10 troca de corpo, 11 alvo).
        ferramenta: i16 = "CurGameTool",
        /// `_CurExpTool`.
        ferramenta_experimental: i16 = "CurExpTool",
        /// `_CurEditMode`.
        modo_edicao: i16 = "CurEditMode",
        /// `_BlkWarnHealth`: limiar de aviso de fome da colônia.
        aviso_saude_colonia: i16 = "BlkWarnHealth",
        /// `_LastThemeSong`.
        ultima_musica: i16 = "LastThemeSong",
        /// `_OptionStates[6]`; `[5]` = balões de fala ("Silly").
        opcoes: [i16; 6] = "OptionStates",
        /// `_TotalEggsLaidB` (32 bits).
        ovos_postos_b: i32 = "TotalEggsLaidB",
        /// `_TotalEggsDiedB` (32 bits).
        ovos_perdidos_b: i32 = "TotalEggsDiedB",
        /// `_AntsEatenByLions`.
        comidas_por_formiga_leao: i16 = "AntsEatenByLions",
        /// `_DdPtr`: próximo índice do anel de cadáveres.
        cadaver_ponteiro: i16 = "DdPtr",
        /// `_DeadX` (anel de 100 cadáveres).
        cadaveres_x: Tabela<u8, 100> = "DeadX",
        /// `_DeadY`.
        cadaveres_y: Tabela<u8, 100> = "DeadY",
    }
}

grupo_do_save! {
    /// Terreno e clima da superfície, comida e o quintal como um todo.
    Ambiente {
        /// `_TERRAINset`: 0 quintal, 1 casa.
        terreno: i16 = "TERRAINset",
        /// `_Barrier`: tile acima do qual não se anda (0x50 quintal, 0x90 casa; `_OverlayTileSet`).
        barreira: i16 = "Barrier",
        /// `_WaterTable`: linha mais alta inundada nos ninhos (64 = nenhuma).
        lencol: i16 = "WaterTable",
        /// `_FoodA`: comida na superfície.
        comida_superficie: i16 = "FoodA",
        /// `_NextFeed`: abaixo dessa comida na superfície, a cada 64 ticks cai comida nova.
        proxima_comida: i16 = "NextFeed",
        /// `_LastFoodDrop`: centro da última comida.
        ultima_comida: (i16, i16) = "LastFoodDrop",
        /// `_RainCnt`.
        chuva_contagem: i16 = "RainCnt",
        /// `_RainOn`.
        chovendo: i16 = "RainOn",
        /// `_ClearRainFlag`.
        limpar_chuva: i16 = "ClearRainFlag",
        /// `_RxTab`: x das 100 gotas.
        gotas_x: Tabela<u8, 100> = "RxTab",
        /// `_RyTab`.
        gotas_y: Tabela<u8, 100> = "RyTab",
        /// `_GrassMap` [uso em aberto].
        grama: [i16; 12] = "GrassMap",
        /// `_YardCycle`: 0..0x3FF, contador de `_DoSimYard`.
        ciclo_quintal: i16 = "YardCycle",
        /// `_CurYardPnt`: patch atual (x, y).
        patch: (i16, i16) = "CurYardPnt",
        /// `_YMapSeeds`: semente de cada patch, índice `y * 12 + x`.
        sementes: Tabela<u16, 192> = "YMapSeeds",
        /// `_ForSaleState`: casa à venda.
        casa_a_venda: i16 = "ForSaleState",
        /// `_HouseColonies`.
        colonias_na_casa: i16 = "HouseColonies",
        /// `_ColonyUpdateFlag`.
        atualizar_colonias: i16 = "ColonyUpdateFlag",
        /// `_SwarmCentX`.
        enxame_centro_x: i16 = "SwarmCentX",
        /// `_SwarmCentY`.
        enxame_centro_y: i16 = "SwarmCentY",
    }
}

grupo_do_save! {
    /// O menino (e o cortador de grama).
    Menino {
        boy_msg_cnt: i32 = "BoyMsgCnt",
        dir: i16 = "BoyDir",
        quadro: i16 = "BoyFrame",
        presente: i16 = "BoyHere",
        mensagem_ligada: i16 = "BoyMessOn",
        mensagem_offset: i16 = "BoyMsgOffset",
        px: i16 = "BoyPx",
        py: i16 = "BoyPy",
        parado_contagem: i16 = "BoyStandCnt",
        giro_contagem: i16 = "BoyTurnCnt",
        espera: i16 = "BoyWait",
        x: i16 = "BoyX",
        y: i16 = "BoyY",
        cortando_grama: i16 = "BoyIsMowing",
        cortador_x: i16 = "MowX",
        cortador_y: i16 = "MowY",
        ultimo_corte_x: i16 = "LastMowX",
        ultimo_corte_y: i16 = "LastMowY",
        /// `_FootHere`: pé (pisada) no quintal.
        pe_presente: i16 = "FootHere",
        pe_x: i16 = "FootX",
        pe_y: i16 = "FootY",
        pe_alterna: i16 = "FootTog",
        /// `_NodeCnt`/`_NodeNum`: caminho do menino (`_NodeNum` < 0x26 = dentro de casa).
        no_contagem: i16 = "NodeCnt",
        no_numero: i16 = "NodeNum",
    }
}

grupo_do_save! {
    /// Pássaro, gato e cachorro.
    Bichos {
        passaro_atraso: i32 = "BirdDelay",
        passaro_quadro: i16 = "BirdFrame",
        passaro_alvo_x: i16 = "BirdGoalX",
        passaro_alvo_y: i16 = "BirdGoalY",
        passaro_ligado: i16 = "BirdOn",
        passaro_x: i16 = "BirdX",
        passaro_y: i16 = "BirdY",
        gato_atraso: i32 = "CatDelay",
        gato_ciclo: i16 = "CatCycle",
        gato_dir: i16 = "CatDir",
        gato_quadro: i16 = "CatFrame",
        gato_ligado: i16 = "CatOn",
        gato_x: i16 = "CatX",
        gato_y: i16 = "CatY",
        cachorro_atraso_latido: i32 = "DogBarkDelay",
        cachorro_dir: i16 = "DogDir",
        cachorro_quadro: i16 = "DogFrame",
        cachorro_px: i16 = "DogPx",
        cachorro_py: i16 = "DogPy",
        cachorro_giro_contagem: i16 = "DogTurnCnt",
        cachorro_x: i16 = "DogX",
        cachorro_y: i16 = "DogY",
    }
}

grupo_do_save! {
    /// Formigas-leão (inimigos.md §1).
    FormigasLeao {
        /// `_LionListM`: 0 esperando, 1 comendo, 2 voltando.
        m: [u8; 10] = "LionListM",
        /// `_LionListS`: contador do estado 1.
        s: [u8; 10] = "LionListS",
        /// `_LionListT`: quadro da animação.
        t: [u8; 10] = "LionListT",
        x: [u8; 10] = "LionListX",
        y: [u8; 10] = "LionListY",
        /// `_LionIndex`: quantas existem.
        quantidade: i16 = "LionIndex",
        /// `_InitialLions`.
        iniciais: i16 = "InitialLions",
    }
}

grupo_do_save! {
    /// Aranha (inimigos.md §2). Coordenadas em dezesseis avos de célula.
    Aranha {
        x: i16 = "SpidX",
        y: i16 = "SpidY",
        dir: i16 = "SpidDir",
        /// `_SpidOn`.
        ligada: i16 = "SpidOn",
        /// `_SMode`: 0 parada, 1 andando, 2 perseguindo, 3 comendo, 4 fugindo, 5 morrendo.
        estado: i16 = "SMode",
        /// `_Starg`: índice na lista A, −1 = amarela, −2 = nenhum.
        alvo: i16 = "Starg",
        /// `_StargLife`.
        alvo_tipo: i16 = "StargLife",
        /// `_Scycle`: quadro das pernas.
        ciclo_pernas: i16 = "Scycle",
        /// `_Scycle2`: contador de ticks 0..1023.
        ciclo: i16 = "Scycle2",
        /// `_EatCnt`.
        mastigando: i16 = "EatCnt",
        /// `_SCorpseBase`.
        base_cadaver: i16 = "SCorpseBase",
        /// `_SpidBurpCnt`.
        arrotos: i16 = "SpidBurpCnt",
        /// `_DeathCnt`.
        agonia: i16 = "DeathCnt",
        /// `_SpidRevenge`.
        vinganca: i16 = "SpidRevenge",
        /// `_RevSpider`.
        de_re: i16 = "RevSpider",
        /// `_SuserX/Y`: destino clicado quando o jogador é a aranha.
        destino_x: i16 = "SuserX",
        destino_y: i16 = "SuserY",
        /// `_ChaseSpid`: formigas perseguem a aranha.
        perseguir: i16 = "ChaseSpid",
    }
}

grupo_do_save! {
    /// Lagarta e bichos-de-conta (inimigos.md §3 e §4).
    LagartaEBichos {
        /// `_PillarMap`: chão sob o corpo.
        lagarta_chao: [i16; 6] = "PillarMap",
        lagarta_dir: i16 = "PillDir",
        lagarta_fase: i16 = "PillarSeg",
        /// `_PillarState`: 0 inexistente, 1 andando.
        lagarta_estado: i16 = "PillarState",
        lagarta_x: i16 = "PillarX",
        lagarta_y: i16 = "PillarY",
        /// `_SowDir/Save/X/Y[3]` (vagas 1 e 2 usadas).
        bicho_dir: [i16; 3] = "SowDir",
        bicho_chao: [i16; 3] = "SowSave",
        bicho_x: [i16; 3] = "SowX",
        bicho_y: [i16; 3] = "SowY",
    }
}

grupo_do_save! {
    /// Formiga amarela, controlada pelo jogador (formiga-amarela.md §1).
    Amarela {
        plano: i16 = "MePlane",
        x: i16 = "MeLocX",
        y: i16 = "MeLocY",
        /// `_MeType`: grupo × 8, sem cor nem direção.
        tipo: i16 = "MeType",
        dir: i16 = "MeDir",
        /// `_MeColor`: 0 preta, ≠ 0 vermelha.
        cor: i16 = "MeColor",
        saude: i16 = "MeHealth",
        saude_contagem: i16 = "MeHealthCntDwn",
        aviso_saude: i16 = "MeWarnHealth",
        quer_comida: i16 = "MeWantFood",
        ovo: i16 = "MeEgg",
        ovo_carregado: i16 = "MeEggCarried",
        comeu_tipo: i16 = "MeFoodInType",
        ninho_iniciado: i16 = "MeNestStarted",
        base: i16 = "MeBase",
        /// `_MeMode`: 0 formiga, 1 aranha, 2 Experimental.
        modo: i16 = "MeMode",
        modo_aranha: i16 = "MeSMode",
        /// `_MeMoveMe`: há uma ordem em andamento.
        mover: i16 = "MeMoveMe",
        comando: i16 = "MeCmd",
        alvo_plano: i16 = "MeGoalPlane",
        alvo_x: i16 = "MeGoalX",
        alvo_y: i16 = "MeGoalY",
        ultimo_x: i16 = "MeLastX",
        ultimo_y: i16 = "MeLastY",
        passos: i16 = "MeSteps",
        dis: i16 = "MeDis",
        dis_anterior: i16 = "MePrevDis",
        contorno_contagem: i16 = "MeCrazyCnt",
        contorno_dir: i16 = "MeCrazyDir",
        contorno_rot: i16 = "MeCrazyRot",
        alvo_indice: i16 = "MeTargIndex",
        alvo_vida: i16 = "MeTargLife",
        alvo_vida_plano: i16 = "MeTargLifePlane",
        alvo_vida_x: i16 = "MeTargLifeX",
        alvo_vida_y: i16 = "MeTargLifeY",
        comecou_luta: i16 = "MeStartedFight",
        soltando_alarme: i16 = "MeDropAlarm",
        rastro: i16 = "MeScent",
        /// `_FuzLocX/Y`: posição difusa seguida pelas recrutadas.
        difusa_x: i16 = "FuzLocX",
        difusa_y: i16 = "FuzLocY",
        /// `_RebornX/Y`: onde a amarela renasce no ninho preto.
        renascer_x: i16 = "RebornX",
        renascer_y: i16 = "RebornY",
        morreu: i16 = "YellowDied",
    }
}

grupo_do_save! {
    /// Formiga vermelha líder ("Avenger", modo 19) e a malvadeza vermelha (superficie.md §9).
    LiderVermelha {
        destino_x: i16 = "RedDestX",
        destino_y: i16 = "RedDestY",
        x: i16 = "RedLocX",
        y: i16 = "RedLocY",
        plano: i16 = "RedPlane",
        tarefa: i16 = "RedTask",
        /// `_RedInitOn`.
        ligada: i16 = "RedInitOn",
        /// `_JustToBeMean`: chamadas restantes do modo "malvado".
        malvadeza: i16 = "JustToBeMean",
    }
}

grupo_do_save! {
    /// Triângulos de casta e de tarefa e o que a estratégia escolheu (formigas.md §4–5).
    Controles {
        /// `_casteLevels`: [reprodutores, operárias, soldados], 0..65535.
        castas: [u16; 3] = "casteLevels",
        /// `_casteSetLevels`: 4 conjuntos de 3.
        castas_memorizadas: [u16; 12] = "casteSetLevels",
        /// `_modeLevels`: [forragear, cavar, cuidar].
        tarefas: [u16; 3] = "modeLevels",
        /// `_modeSetLevels`.
        tarefas_memorizadas: [u16; 12] = "modeSetLevels",
        /// `_IdealCaste` (%): operária, soldado, macho, rainha nova.
        castas_ideais: [i16; 4] = "IdealCaste",
        /// `_MakeMe`: grupo do próximo ovo.
        proxima_casta: i16 = "MakeMe",
        /// `_ModeMe`: modo das próximas operárias.
        proximo_modo: i16 = "ModeMe",
        /// `_ModeAuto`.
        modo_automatico: i16 = "ModeAuto",
        /// `_CasteAuto`.
        casta_automatica: i16 = "CasteAuto",
        botao_castas: i16 = "casteButtonState",
        botao_tarefas: i16 = "modeButtonState",
        mostrar_porcentagem_castas: i16 = "ShowPercentCaste",
        mostrar_porcentagem_tarefas: i16 = "ShowPercentMode",
        /// `_LayDownQueenMode`.
        modo_por_rainha: i16 = "LayDownQueenMode",
    }
}

grupo_do_save! {
    /// Tutorial e lições.
    Tutorial {
        dada: i16 = "TutGiven",
        duracao: i16 = "TutHowLong",
        licao: i16 = "TutLesson",
        contagem: i16 = "TutorCnt",
        temporario: i16 = "LessonTemp",
    }
}

grupo_do_save! {
    /// Vistas da interface que vão no save (mapa mostrado, pontos lembrados).
    Vistas {
        ponto: (i16, i16) = "MapPnt",
        ponto_superficie: (i16, i16) = "AMapPnt",
        ponto_quintal: (i16, i16) = "YMapPnt",
        modo_mapa: i16 = "MapMode",
        plano: i16 = "MapPlane",
        plano_anterior: i16 = "OldMapPlane",
        modo_quintal: i16 = "YardMode",
    }
}

grupo_do_save! {
    /// Séries do gráfico de história (anel de 64 amostras, uma a cada 64 ticks).
    Historico {
        /// `_HistStart`: próxima posição do anel.
        inicio: i16 = "HistStart",
        /// `_HistCnt`: amostras válidas (até 63).
        contagem: i16 = "HistCnt",
        populacao_b: Tabela<i16, 64> = "H_BPop",
        populacao_r: Tabela<i16, 64> = "H_RPop",
        comida_b: Tabela<i16, 64> = "H_BFood",
        comida_r: Tabela<i16, 64> = "H_RFood",
        saude_b: Tabela<i16, 64> = "H_BHeal",
        saude_r: Tabela<i16, 64> = "H_RHeal",
        comida_superficie: Tabela<i16, 64> = "H_FoodA",
        /// `_H_ADead` ← `_BAntsExpired`.
        mortes_naturais: Tabela<i16, 64> = "H_ADead",
        /// `_H_PDead` ← `_BAntsEaten`.
        mortes_por_predadores: Tabela<i16, 64> = "H_PDead",
        /// `_H_FDead` ← `_BAntsKilled`.
        mortes_em_luta: Tabela<i16, 64> = "H_FDead",
        /// `_HistButtStates`: séries ligadas no gráfico.
        botoes: [i16; 10] = "HistButtStates",
    }
}

// ---------------------------------------------------------------------------------------------
// Estado de execução (não salvo)
// ---------------------------------------------------------------------------------------------

/// Algo que a simulação pede à interface (som, mensagem, diálogo). As regras não chamam áudio nem
/// janela; deixam o pedido aqui, e a camada de cima esvazia a fila depois de cada tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evento {
    /// Texto de SHARED tipo 4 (`_PictStrnDialog`), pelo id decimal (ex.: 10010).
    Mensagem(u16),
    /// `_YellowDialog(bitmap)` (`1000:8262`): quadro com imagem da amarela (trofalaxia: 9110). A
    /// interface só o mostra com Options → Events.
    QuadroAmarela(u16),
    /// `_SpecialXfer` (`1000:7eb6`): depois do quadro 10058, o jogador escolhe outra colônia preta no
    /// mapa do quintal. A interface chama `troca_patch::iniciar_transferencia_especial` e, no clique,
    /// `troca_patch::escolher_colonia` (no original, dentro do tick; no port, logo depois).
    EscolherColonia,
    /// Música (`_myBeginSong`), pelo id do cabeçalho tipo 18 do banco SOUND.
    Musica(u16),
    /// Efeito sonoro (`_myBeginSound`), pelo id do objeto tipo 5 do banco SOUND
    /// (catálogo em docs/spec/regras/som.md).
    Som(u16),
    /// `_TryAntTheme` (`1020:1414`): a amarela entrou ou saiu de um ninho. Se toca um tema e qual
    /// depende do relógio de parede (7200 ticks de `_TickCount` desde o último), que é da interface.
    TemaDoNinho,
    /// `_EndGameDialog(0)`: pedido a cada tick enquanto `_IsGameOver` ≠ 0.
    DialogoFimDeJogo,
    /// `_GotoMyAnt` (`1000:a764`): levar a vista da janela de edição até a formiga amarela
    /// (AutoTrack, Home, Backspace, clique na própria amarela, ordem impossível...). Leva a posição
    /// da amarela **no momento do pedido**: num mesmo tick o original pode centralizar mais de uma vez
    /// (na morte, no lugar onde ela morreu; depois, no corpo novo), e cada `_CenterEdit` parte de onde
    /// o anterior deixou a vista (vetor `amarela_leao/1711`, testes-tick.md).
    CentralizarNaAmarela { plano: i16, x: i16, y: i16 },
    /// `_EditMessage(texto, duração, importante)`: mensagem da faixa (telas/mensagens.md §2).
    MensagemFaixa(MensagemFaixa),
    /// Uma animação dentro do tick (`_AnimYellowFight`) espera tantos milissegundos de tempo real
    /// entre dois quadros. No original o programa inteiro fica parado nesse tempo, então os sons
    /// pedidos depois saem mais tarde; a interface usa a espera para espalhar no tempo os efeitos
    /// da mesma fila (regras/som.md, "Linha do tempo dos efeitos").
    Espera(u16),
    /// Os quadros de uma animação da amarela dentro do tick (`_AnimYellowFight`): o original redesenha
    /// a janela de edição a cada quadro e fica parado `espera_ms` entre dois quadros (o programa
    /// inteiro, inclusive a simulação). A interface mostra os quadros e não roda ticks nesse tempo.
    /// Não muda o estado (regras/formiga-amarela.md §10.2).
    Animacao(Box<AnimacaoDaAmarela>),
}

/// Animação da amarela (`_AnimYellowFight`, `1028:7e56`): o plano animado e, para cada quadro, a
/// janela de edição como `_DoEditUpdateDraw` a desenharia naquele instante.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimacaoDaAmarela {
    /// Código de `_AnimYellowFight`: 0 luta, 1–9 mortes, 10 choque elétrico.
    pub codigo: i16,
    /// `_MePlane` animado (1 superfície, 2 ninho preto, 3 ninho vermelho).
    pub plano: i16,
    /// Tempo real entre o começo de dois quadros (ms); depois do último não há espera.
    pub espera_ms: u16,
    pub quadros: Vec<QuadroAnimacao>,
}

/// Um quadro: mapa de chão e de vida do plano animado, e os campos `_Me*` que o desenho da marca
/// 0xFF lê (a luta desenha a amarela com `_MeType` = 0x70, a nuvem de luta, numa direção sorteada).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuadroAnimacao {
    pub mapa: Vec<u8>,
    pub vida: Vec<u8>,
    pub x: i16,
    pub y: i16,
    pub tipo: i16,
    pub dir: i16,
    pub ovo: i16,
    pub base: i16,
}

impl AnimacaoDaAmarela {
    /// Tempo em que o programa fica parado (ms): uma espera antes de cada quadro, menos o primeiro
    /// (que espera só o próximo degrau do relógio, até 55 ms, não contado).
    pub fn duracao_ms(&self) -> u64 {
        u64::from(self.espera_ms) * self.quadros.len().saturating_sub(1) as u64
    }
}

/// Pedido de mensagem na faixa: string `indice` da lista SHARED 4/`lista`, com a duração do
/// original (prazo = duração × 3 / 2 unidades de `_TickCount`) e o "importante" de `_EditMessage`.
/// Vem dos arquivos de dados das regras.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub struct MensagemFaixa {
    pub lista: u16,
    pub indice: u8,
    pub duracao: u16,
    pub importante: bool,
}

/// Balões de fala da janela de edição (opção "Silly"), fora do save. O tick só anota onde há
/// balão (`_FightBalloons` & cia.); os sorteios e a escolha do balão são do desenho
/// (`rules::baloes`). Especificação: docs/spec/telas/baloes.md.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Baloes {
    /// `_FightBalloonCnt`, `_RestBalloonCnt`, `_QueenBalloonCnt`, `_EggBalloonCnt` (índice:
    /// `rules::baloes::TipoBalao`); zerados no começo do tick com "Silly".
    pub contagem: [i16; 4],
    /// `_LastFightPnt` & cia. (célula; −1 = nenhum) e `_Last*Plane`.
    pub ultimo: [(i16, i16); 4],
    pub ultimo_plano: [i16; 4],
    /// `_CurFightPnt` & cia. (célula do balão mostrado) e `_Cur*Plane`.
    pub atual: [(i16, i16); 4],
    pub atual_plano: [i16; 4],
    /// `_editWidth` e `_editHeight` (a interface informa antes do tick).
    pub vista: (i16, i16),
    /// Por fala (`rules::baloes::Fala`): prazo em `_TickCount`, balão ligado e índice da fala.
    pub prazo: [u32; 6],
    pub ligado: [bool; 6],
    pub indice: [i16; 6],
    /// `DGROUP:0x72f4`: `_SMode` visto pelo último balão da aranha.
    pub modo_aranha_visto: i16,
    /// `_CurBalloonCnt` e a lista de balões do redesenho (`_AddMsgBalloon`, até 6).
    pub pedidos: [Option<crate::rules::baloes::Balao>; 6],
    pub quantidade: i16,
    /// O balão desenhado no último redesenho da janela de edição (`_DrawBalloons` desenha um só).
    pub desenhado: Option<crate::rules::baloes::Balao>,
}

impl Baloes {
    /// Passo 3 do tick (`_DoAntSim`): zera as contagens e os últimos pontos (os planos ficam).
    pub fn zerar(&mut self) {
        self.contagem = [0; 4];
        self.ultimo = [(-1, -1); 4];
    }
}

// ---------------------------------------------------------------------------------------------
// Configuração (dados)
// ---------------------------------------------------------------------------------------------

/// Valores iniciais de um jogo novo que não saem da geração do mundo (`data/simulacao.json`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ValoresIniciais {
    pub castas: [u16; 3],
    pub castas_memorizadas: [u16; 12],
    pub tarefas: [u16; 3],
    pub tarefas_memorizadas: [u16; 12],
    pub castas_ideais: [i16; 4],
    pub modo_automatico: i16,
    pub casta_automatica: i16,
    /// `_casteButtonState` e `_modeButtonState`.
    pub botao_controles: i16,
    /// `_ShowPercentCaste` e `_ShowPercentMode`.
    pub mostrar_porcentagem: i16,
    pub velocidade: i16,
    pub aviso_saude: i16,
    pub proxima_comida: i16,
    pub opcoes: [i16; 6],
    pub botoes_historico: [i16; 10],
    pub barreira_quintal: i16,
    pub barreira_casa: i16,
}

/// Conteúdo de `data/simulacao.json`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConfigSimulacao {
    /// `_SpeedDelayVals`, por `_GameSpeed`: mensagens de temporizador entre ticks (≤ 0 = todo laço).
    pub atrasos_velocidade: Vec<i16>,
    pub inicial: ValoresIniciais,
}

impl ConfigSimulacao {
    pub fn de_json(texto: &str) -> Result<ConfigSimulacao, String> {
        serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_CONFIG}: {e}"))
    }

    /// Lê `simulacao.json` (com mods).
    pub fn carregar(r: &Recursos) -> Result<ConfigSimulacao, String> {
        r.dados(ARQUIVO_CONFIG)
    }
}

// ---------------------------------------------------------------------------------------------
// Estado
// ---------------------------------------------------------------------------------------------

/// Todo o estado da simulação.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Estado {
    pub partida: Partida,
    pub ambiente: Ambiente,
    pub superficie: Superficie,
    pub ninho_b: Ninho,
    pub ninho_r: Ninho,
    pub feromonios: Feromonios,
    pub preta: Colonia,
    pub vermelha: Colonia,
    pub amarela: Amarela,
    pub lider_vermelha: LiderVermelha,
    pub formigas_leao: FormigasLeao,
    pub aranha: Aranha,
    pub lagarta_e_bichos: LagartaEBichos,
    pub menino: Menino,
    pub bichos: Bichos,
    pub controles: Controles,
    pub tutorial: Tutorial,
    pub vistas: Vistas,
    pub historico: Historico,

    // --- execução: não vai para o save ---
    /// Gerador S (LFSR, `_SRand*`): quase toda a simulação.
    pub s: GeradorS,
    /// Gerador R (`_RRand`): sementes, chuva, lutas.
    pub r: GeradorR,
    /// `_TemCensus`: 32 contadores por `T >> 3` (pretos 0..15, vermelhos 16..31), da última contagem.
    pub censo: [i16; 32],
    /// `_TemBModePop` e `_TemRModePop`: formigas por modo (0..19) contadas neste tick.
    pub modos_do_tick: [[i16; 20]; 2],
    /// `_Tindex`: índice da formiga em processamento no laço da lista atual.
    pub tindex: i32,
    pub baloes: Baloes,
    /// `_GlobalKey`: última tecla especial (0xA8 faz chover).
    pub tecla_global: i16,
    /// `_gameCycles` (`9:9b2c`): voltas do laço do temporizador (não é contador de ticks; depende do
    /// relógio). Entrada do tick: `_SimBird` o usa para `_BirdDelay` (testes-tick.md, "Pergunta 1").
    pub ciclos_jogo: u32,
    /// Relógio do Mac (`_MacTickCount` @ `1000:9694` = `GetTickCount() / 55 × 3`, quintal.md "Relógios")
    /// a usar neste tick, quando conhecido (vetores de tick do original). `None`: o port o deriva
    /// (`quintal::Relogios::do_port`).
    pub relogio_mac: Option<u32>,
    /// `_gGameNeedsSaving`.
    pub precisa_salvar: bool,
    /// `_ELayerMode` (fora do save): camada de feromônio sobreposta à superfície na janela de edição
    /// (`_DoCalcTile`): −1 nenhuma, 0 alarme, 1 ninho preto, 2 trilha preta, 3 ninho vermelho,
    /// 4 trilha vermelha. Ligada pelo alarme da amarela (`_SetAlarmDropState`) e pelos botões de
    /// cheiro da faixa (jogo.md, "Camada de cheiro na janela de edição").
    pub camada_edicao: i16,
    /// `_DROPdir` (`9:9c66`, fora do save) já resolvido no passo (dx, dy) da comida que cai na casa
    /// (`TabelasCasa::passo_queda`): gravado quando um patch da casa é gerado; antes disso, o de
    /// `_DROPdir` = 0 (variável zerada na carga do programa).
    pub passo_queda: (u8, u8),
    /// Pedidos à interface acumulados desde a última vez que foram retirados.
    pub eventos: Vec<Evento>,
}

/// Índice de cor nas tabelas por cor (`modos_do_tick`).
pub fn indice_cor(cor: Cor) -> usize {
    match cor {
        Cor::Preta => 0,
        Cor::Vermelha => 1,
    }
}

/// Cor de um T (bit 7).
pub fn cor_do_tipo(t: u8) -> Cor {
    if t & 0x80 != 0 {
        Cor::Vermelha
    } else {
        Cor::Preta
    }
}

impl Estado {
    /// `_GotoMyAnt`: pede à interface que leve a vista até a amarela, na posição de agora.
    pub fn pedir_centralizar_na_amarela(&mut self) {
        let a = &self.amarela;
        let ev = Evento::CentralizarNaAmarela { plano: a.plano, x: a.x, y: a.y };
        self.eventos.push(ev);
    }

    /// Estado com todos os blocos zerados e geradores na semente 0 / estado 1.
    pub fn vazio() -> Estado {
        Estado {
            partida: Partida::default(),
            ambiente: Ambiente::default(),
            superficie: Superficie {
                mapa: Grade::nova(SUPERFICIE),
                vida: Grade::nova(SUPERFICIE),
                lista: ListaFormigas::vazia(CAPACIDADE_A),
            },
            ninho_b: Ninho::vazio(),
            ninho_r: Ninho::vazio(),
            feromonios: Feromonios::vazios(),
            preta: Colonia::default(),
            vermelha: Colonia::default(),
            amarela: Amarela::default(),
            lider_vermelha: LiderVermelha::default(),
            formigas_leao: FormigasLeao::default(),
            aranha: Aranha::default(),
            lagarta_e_bichos: LagartaEBichos::default(),
            menino: Menino::default(),
            bichos: Bichos::default(),
            controles: Controles::default(),
            tutorial: Tutorial::default(),
            vistas: Vistas::default(),
            historico: Historico::default(),
            s: GeradorS::new(0),
            r: GeradorR::default(),
            censo: [0; 32],
            modos_do_tick: [[0; 20]; 2],
            tindex: 0,
            baloes: Baloes::default(),
            tecla_global: 0,
            ciclos_jogo: 0,
            relogio_mac: None,
            precisa_salvar: false,
            camada_edicao: -1,
            passo_queda: (0, 255),
            eventos: Vec::new(),
        }
    }

    // --- acesso por cor / área ---

    pub fn colonia(&self, cor: Cor) -> &Colonia {
        match cor {
            Cor::Preta => &self.preta,
            Cor::Vermelha => &self.vermelha,
        }
    }

    pub fn colonia_mut(&mut self, cor: Cor) -> &mut Colonia {
        match cor {
            Cor::Preta => &mut self.preta,
            Cor::Vermelha => &mut self.vermelha,
        }
    }

    /// Ninho da cor (preto = B, vermelho = R).
    pub fn ninho(&self, cor: Cor) -> &Ninho {
        match cor {
            Cor::Preta => &self.ninho_b,
            Cor::Vermelha => &self.ninho_r,
        }
    }

    pub fn ninho_mut(&mut self, cor: Cor) -> &mut Ninho {
        match cor {
            Cor::Preta => &mut self.ninho_b,
            Cor::Vermelha => &mut self.ninho_r,
        }
    }

    pub fn lista(&self, area: Lista) -> &ListaFormigas {
        match area {
            Lista::A => &self.superficie.lista,
            Lista::B => &self.ninho_b.lista,
            Lista::R => &self.ninho_r.lista,
        }
    }

    pub fn lista_mut(&mut self, area: Lista) -> &mut ListaFormigas {
        match area {
            Lista::A => &mut self.superficie.lista,
            Lista::B => &mut self.ninho_b.lista,
            Lista::R => &mut self.ninho_r.lista,
        }
    }

    /// Mapa de vida da área.
    pub fn vida_mut(&mut self, area: Lista) -> &mut Grade {
        match area {
            Lista::A => &mut self.superficie.vida,
            Lista::B => &mut self.ninho_b.vida,
            Lista::R => &mut self.ninho_r.vida,
        }
    }

    /// `_AddAntToAList/BList/RList`: acrescenta na lista e grava T no mapa de vida. Lista cheia:
    /// nada muda e devolve `false`.
    pub fn adicionar_formiga(&mut self, area: Lista, f: Formiga) -> bool {
        if !self.lista_mut(area).adicionar(f) {
            return false;
        }
        self.vida_mut(area).definir(f.x as i32, f.y as i32, f.tipo);
        true
    }

    /// Tipo de jogo (`_CurGameType`), se for um dos quatro conhecidos.
    pub fn tipo_jogo(&self) -> Option<TipoJogo> {
        TipoJogo::de_i16(self.partida.tipo_jogo)
    }

    /// Retira os eventos acumulados.
    pub fn retirar_eventos(&mut self) -> Vec<Evento> {
        std::mem::take(&mut self.eventos)
    }

    /// Pede o efeito sonoro `id` (tipo 5) à interface; `None` (som não definido nos dados) não faz nada.
    /// Não sorteia nem muda o estado da simulação.
    pub fn pedir_som(&mut self, id: Option<u16>) {
        if let Some(id) = id {
            self.eventos.push(Evento::Som(id));
        }
    }

    // --- save ---

    /// Estado a partir de um `.ANT`. Os mapas de vida são reconstruídos como `_DoLoadInitializations`
    /// (core/vida.rs). Os geradores não estão no save: S começa na semente do patch atual e R no
    /// estado inicial do runtime C (ver [`Estado::de_save_com_pendencias`]).
    pub fn de_save(j: &JogoSalvo) -> Result<Estado, String> {
        Ok(Estado::de_save_com_pendencias(j)?.0)
    }

    /// Como [`Estado::de_save`], devolvendo também o que a reconstrução da vida não reproduziu
    /// (por exemplo, a segunda célula da amarela rainha).
    ///
    /// O original, ao carregar, chama `_RandYard` antes de ler os blocos (rng.md, "A semente está no
    /// save?"; conferido na bancada, `qemu_load_rng.txt`), então S depois de um load depende do R da
    /// sessão. Aqui, sem a sessão, o port usa a semente do patch atual como convenção determinística
    /// (vetores de tick); o Load da interface refaz o `_RandYard` (`core::carregar::geradores_depois_do_load`).
    pub fn de_save_com_pendencias(j: &JogoSalvo) -> Result<(Estado, Vec<String>), String> {
        let rec = vida::reconstruir(j)?;
        let ambiente = Ambiente::ler(j)?;
        let (px, py) = ambiente.patch;
        let semente =
            if (0..12).contains(&px) && (0..16).contains(&py) { ambiente.sementes[(py * 12 + px) as usize] } else { 0 };
        let ninho = |mapa: &str, saida: &str, vida: Vec<u8>, lista: Lista| -> Result<Ninho, String> {
            Ok(Ninho {
                mapa: Grade::ler(j, mapa, NINHO)?,
                saida: Grade::ler(j, saida, NINHO)?,
                vida: Grade::de_bytes(NINHO, vida)?,
                lista: ListaFormigas::ler(j, lista)?,
            })
        };
        let e = Estado {
            partida: Partida::ler(j)?,
            ambiente,
            superficie: Superficie {
                mapa: Grade::ler(j, "MapA", SUPERFICIE)?,
                vida: Grade::de_bytes(SUPERFICIE, rec.camadas.a)?,
                lista: ListaFormigas::ler(j, Lista::A)?,
            },
            ninho_b: ninho("MapB", "ExitMapB", rec.camadas.b, Lista::B)?,
            ninho_r: ninho("MapR", "ExitMapR", rec.camadas.r, Lista::R)?,
            feromonios: Feromonios {
                alarme: Grade::ler(j, "PherMapA", FEROMONIO)?,
                ninho_b: Grade::ler(j, "PherMapBN", FEROMONIO)?,
                trilha_b: Grade::ler(j, "PherMapBT", FEROMONIO)?,
                ninho_r: Grade::ler(j, "PherMapRN", FEROMONIO)?,
                trilha_r: Grade::ler(j, "PherMapRT", FEROMONIO)?,
                f: Grade::nova(FEROMONIO),
            },
            preta: Colonia::ler(j, Cor::Preta)?,
            vermelha: Colonia::ler(j, Cor::Vermelha)?,
            amarela: Amarela::ler(j)?,
            lider_vermelha: LiderVermelha::ler(j)?,
            formigas_leao: FormigasLeao::ler(j)?,
            aranha: Aranha::ler(j)?,
            lagarta_e_bichos: LagartaEBichos::ler(j)?,
            menino: Menino::ler(j)?,
            bichos: Bichos::ler(j)?,
            controles: Controles::ler(j)?,
            tutorial: Tutorial::ler(j)?,
            vistas: Vistas::ler(j)?,
            historico: Historico::ler(j)?,
            s: GeradorS::new(semente),
            ..Estado::vazio()
        };
        Ok((e, rec.pendencias))
    }

    /// Grava o estado num `.ANT` com o layout dado (blocos de `data/save_layout.json`).
    pub fn para_save(&self, layout: &LayoutSave) -> Result<JogoSalvo, String> {
        let mut j = JogoSalvo::ler(layout, vec![0; layout.total_bytes])?;
        self.partida.gravar(&mut j)?;
        self.ambiente.gravar(&mut j)?;
        self.superficie.mapa.gravar(&mut j, "MapA")?;
        self.superficie.lista.gravar(&mut j, Lista::A)?;
        for (n, mapa, saida, lista) in
            [(&self.ninho_b, "MapB", "ExitMapB", Lista::B), (&self.ninho_r, "MapR", "ExitMapR", Lista::R)]
        {
            n.mapa.gravar(&mut j, mapa)?;
            n.saida.gravar(&mut j, saida)?;
            n.lista.gravar(&mut j, lista)?;
        }
        let f = &self.feromonios;
        f.alarme.gravar(&mut j, "PherMapA")?;
        f.ninho_b.gravar(&mut j, "PherMapBN")?;
        f.trilha_b.gravar(&mut j, "PherMapBT")?;
        f.ninho_r.gravar(&mut j, "PherMapRN")?;
        f.trilha_r.gravar(&mut j, "PherMapRT")?;
        self.preta.gravar(&mut j, Cor::Preta)?;
        self.vermelha.gravar(&mut j, Cor::Vermelha)?;
        self.amarela.gravar(&mut j)?;
        self.lider_vermelha.gravar(&mut j)?;
        self.formigas_leao.gravar(&mut j)?;
        self.aranha.gravar(&mut j)?;
        self.lagarta_e_bichos.gravar(&mut j)?;
        self.menino.gravar(&mut j)?;
        self.bichos.gravar(&mut j)?;
        self.controles.gravar(&mut j)?;
        self.tutorial.gravar(&mut j)?;
        self.vistas.gravar(&mut j)?;
        self.historico.gravar(&mut j)?;
        Ok(j)
    }

    /// Todos os símbolos do save que o estado cobre (para conferir contra o layout).
    pub fn simbolos_cobertos() -> Vec<String> {
        let mut v: Vec<String> = [
            "MapA",
            "MapB",
            "MapR",
            "ExitMapB",
            "ExitMapR",
            "PherMapA",
            "PherMapBN",
            "PherMapBT",
            "PherMapRN",
            "PherMapRT",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        for l in Lista::TODAS {
            v.extend("XYMTS".chars().map(|c| l.simbolo(c)));
            v.push(l.simbolo_contador());
        }
        let grupos: [&[&str]; 15] = [
            Colonia::SIMBOLOS_PRETA,
            Colonia::SIMBOLOS_VERMELHA,
            Partida::SIMBOLOS,
            Ambiente::SIMBOLOS,
            Menino::SIMBOLOS,
            Bichos::SIMBOLOS,
            FormigasLeao::SIMBOLOS,
            Aranha::SIMBOLOS,
            LagartaEBichos::SIMBOLOS,
            Amarela::SIMBOLOS,
            LiderVermelha::SIMBOLOS,
            Controles::SIMBOLOS,
            Tutorial::SIMBOLOS,
            Vistas::SIMBOLOS,
            Historico::SIMBOLOS,
        ];
        v.extend(grupos.iter().flat_map(|g| g.iter().map(|s| s.to_string())));
        v
    }

    // --- geração ---

    /// Estado de um jogo novo a partir do mundo gerado (`_NewGame` → `_RandYard` → `_RandWorld`,
    /// geracao-mundo.md §1–3). `s` e `r` são os geradores como a geração os deixou.
    ///
    /// Além do que a geração produz, aplica os passos de `_NewGame`, `_RandYard`, `_RandWorld` (18),
    /// `_InitSimVars`, `_initControls`, `_ClearHistory`, `_OverlayTileSet` (`_Barrier`) e `_CountAnts`
    /// descritos na spec, com os valores de `data/simulacao.json`. Não reproduz `_InitSimYard`
    /// (posições do menino e do cachorro) nem `_ResetYellowVars`: ficam zerados (lacuna).
    pub fn de_mundo(m: &Mundo, s: GeradorS, r: GeradorR, cfg: &ConfigSimulacao) -> Result<Estado, String> {
        let ini = &cfg.inicial;
        let mut e = Estado::vazio();
        e.s = s;
        e.r = r;

        // _NewGame (geracao-mundo.md §1) e _InitSimVars.
        let p = &mut e.partida;
        p.tipo_jogo = m.tipo_jogo as i16;
        p.acabou_de_trocar = 1;
        p.ferramenta = -1;
        p.velocidade = ini.velocidade;
        p.aviso_saude_colonia = ini.aviso_saude;
        p.opcoes = ini.opcoes;
        e.controles = Controles {
            castas: ini.castas,
            castas_memorizadas: ini.castas_memorizadas,
            tarefas: ini.tarefas,
            tarefas_memorizadas: ini.tarefas_memorizadas,
            castas_ideais: ini.castas_ideais,
            modo_automatico: ini.modo_automatico,
            casta_automatica: ini.casta_automatica,
            botao_castas: ini.botao_controles,
            botao_tarefas: ini.botao_controles,
            mostrar_porcentagem_castas: ini.mostrar_porcentagem,
            mostrar_porcentagem_tarefas: ini.mostrar_porcentagem,
            modo_por_rainha: 1,
            ..Controles::default()
        };
        if m.tipo_jogo == TipoJogo::Tutorial {
            e.tutorial.licao = 1;
            e.tutorial.dada = 0;
        }

        // _RandYard (§2) e _ClearHistory.
        e.historico.inicio = 63;
        e.historico.contagem = 0;
        e.historico.botoes = ini.botoes_historico;
        e.ambiente.patch = (m.patch.0 as i16, m.patch.1 as i16);
        e.vistas.ponto_quintal = e.ambiente.patch;
        e.ambiente.ciclo_quintal = -1; // _InitSimYard
        e.vistas.plano = if (m.tipo_jogo as i16) < 2 { 2 } else { 1 };
        e.vistas.modo_mapa = e.vistas.plano;
        e.ambiente.sementes = Tabela(m.sementes);

        // Mapas, listas e vida.
        e.ambiente.terreno = m.terreno as i16;
        e.ambiente.barreira = if m.terreno == 0 { ini.barreira_quintal } else { ini.barreira_casa };
        if m.terreno == 1 {
            e.passo_queda = m.passo_queda;
        }
        e.superficie.mapa = Grade::de_bytes(SUPERFICIE, m.mapa_a.clone())?;
        e.superficie.vida = Grade::de_bytes(SUPERFICIE, m.vida_a.clone())?;
        e.ninho_b.mapa = Grade::de_bytes(NINHO, m.mapa_b.clone())?;
        e.ninho_b.saida = Grade::de_bytes(NINHO, m.saida_b.clone())?;
        e.ninho_b.vida = Grade::de_bytes(NINHO, m.vida_b.clone())?;
        e.ninho_r.mapa = Grade::de_bytes(NINHO, m.mapa_r.clone())?;
        e.ninho_r.saida = Grade::de_bytes(NINHO, m.saida_r.clone())?;
        e.ninho_r.vida = Grade::de_bytes(NINHO, m.vida_r.clone())?;
        for (lista, formigas) in [(Lista::A, &m.lista_a), (Lista::B, &m.lista_b), (Lista::R, &m.lista_r)] {
            for f in formigas {
                e.lista_mut(lista).adicionar(*f);
            }
        }

        // Colônias.
        for (cor, buracos, ultimo_buraco, escav, rainhas, ultimo_ovo, comida) in [
            (
                Cor::Preta,
                &m.buracos_b,
                m.ultimo_buraco_b,
                m.escavacao_b,
                m.rainhas_pretas,
                m.ultimo_ovo_preto,
                m.comida_b,
            ),
            (
                Cor::Vermelha,
                &m.buracos_r,
                m.ultimo_buraco_r,
                m.escavacao_r,
                m.rainhas_vermelhas,
                m.ultimo_ovo_vermelho,
                m.comida_r,
            ),
        ] {
            let c = e.colonia_mut(cor);
            c.buracos = Tabela(*buracos);
            c.ultimo_buraco = ultimo_buraco;
            // _LastNewExit? = (coluna, 0); a coluna é o y do buraco na superfície (ninho.md §1).
            c.ultima_saida = (ultimo_buraco.1, 0);
            c.soma_x_escavada = escav.tot_x;
            c.soma_y_escavada = escav.tot_y;
            c.tiles_escavados = escav.cavados;
            c.centro_x = escav.massa_x;
            c.centro_y = escav.massa_y;
            c.rainhas = rainhas;
            c.ultimo_ovo = ultimo_ovo;
            c.comida = comida;
            c.vista_ninho = (0x20, 1);
        }
        e.preta.saude = m.saude_b;
        e.vermelha.saude = m.saude_r;

        // Superfície.
        e.ambiente.lencol = m.lencol;
        e.ambiente.comida_superficie = m.comida_a;
        e.ambiente.ultima_comida = m.ultima_comida;
        e.ambiente.proxima_comida = ini.proxima_comida;
        e.vistas.ponto = (0x40, 0x20);
        e.vistas.ponto_superficie = (0x40, 0x20);

        // Bichos.
        let fl = &mut e.formigas_leao;
        for (k, &(x, y)) in m.formigas_leao.iter().enumerate().take(10) {
            fl.x[k] = x;
            fl.y[k] = y;
        }
        fl.quantidade = m.formigas_leao.len() as i16;
        fl.iniciais = m.formigas_leao_iniciais;
        let lb = &mut e.lagarta_e_bichos;
        for (k, b) in m.bichos_de_conta.iter().enumerate() {
            lb.bicho_x[k] = b.x;
            lb.bicho_y[k] = b.y;
            lb.bicho_dir[k] = b.dir;
            lb.bicho_chao[k] = b.chao as i16;
        }
        e.aranha = Aranha {
            x: m.aranha.x,
            y: m.aranha.y,
            dir: m.aranha.dir,
            arrotos: m.aranha.arrotos,
            ligada: m.aranha.ligada as i16,
            alvo: -2,
            ..Aranha::default()
        };

        // Formiga amarela (_InitYelloAnt) e a vista do plano dela (_RandYard, passo 9).
        let a = &m.amarela;
        e.amarela = Amarela {
            plano: a.plano,
            x: a.x,
            y: a.y,
            tipo: a.tipo as i16,
            dir: a.dir as i16,
            modo: a.modo,
            saude: a.saude,
            ovo_carregado: a.ovo_carregado,
            ninho_iniciado: m.ninho_iniciado as i16,
            aviso_saude: ini.aviso_saude,
            renascer_x: m.renascer.0,
            renascer_y: m.renascer.1,
            ..Amarela::default()
        };
        let ponto = (a.x, a.y);
        match a.plano {
            2 => e.preta.vista_ninho = ponto,
            3 => e.vermelha.vista_ninho = ponto,
            _ => e.vistas.ponto_superficie = ponto,
        }

        // _RandWorld passo 18: _CountAnts (com _JustXfered = 1, sem fim de jogo).
        e.contar_formigas();
        Ok(e)
    }

    // --- contagens ---

    /// `_CountAnts` (`1020:04de`, simulacao-visao-geral.md §5.1): recenseia as três listas e a
    /// amarela, preenche `castas` e `populacao` das duas colônias e detecta a morte das rainhas.
    pub fn contar_formigas(&mut self) {
        let mut censo = [0i16; 32];
        for area in Lista::TODAS {
            let l = self.lista(area);
            for i in (0..l.usadas()).rev() {
                let t = l.vagas[i].tipo;
                if t != 0 {
                    censo[(t >> 3) as usize] += 1;
                }
            }
        }
        if self.amarela.modo == 0 {
            let cor = if self.amarela.cor != 0 { 0x80 } else { 0 };
            let t = (self.amarela.tipo as u16 & 0xFFFB) | cor;
            if let Some(c) = censo.get_mut((t >> 3) as usize) {
                *c += 1;
            }
        }
        self.censo = censo;

        let trocou = self.partida.acabou_de_trocar != 0;
        let rapido = self.partida.tipo_jogo < 2;
        for (cor, base, musica, msg_morte, msg_fim) in
            [(Cor::Preta, 0, 0x2B0C, 10010, 10011), (Cor::Vermelha, 16, 0x2B0D, 10012, 10013)]
        {
            let k = |g: usize| censo[base + g];
            let rainha_antes = self.colonia(cor).castas[5];
            let c = self.colonia_mut(cor);
            c.castas[0] = k(0);
            c.castas[1] = k(1) + k(2) + k(3) + k(5);
            c.castas[2] = k(6) + k(7) + k(9);
            c.castas[3] = k(4);
            c.castas[4] = k(8);
            if rainha_antes != 0 && k(12) == 0 && !trocou {
                self.eventos.push(Evento::Musica(musica));
                self.eventos.push(Evento::Mensagem(msg_morte));
                if rapido {
                    self.eventos.push(Evento::Mensagem(msg_fim));
                    self.partida.fim_de_jogo = 1;
                    self.partida.pretas_venceram = (cor == Cor::Vermelha) as i16;
                }
                if cor == Cor::Vermelha
                    && self.partida.tipo_jogo == 2
                    && self.vermelha.total_colonias == 1
                    && self.ambiente.patch == (11, 8)
                {
                    // Uma nova colônia vermelha (população 20) no patch (k + 2, 0) do quintal.
                    let k = self.s.rand1(6) as usize;
                    self.vermelha.populacao_quintal[k * 16 + 0x20] = 0x14;
                }
            }
            let c = self.colonia_mut(cor);
            c.castas[5] = k(12);
            c.populacao = c.castas[1..].iter().sum();
        }
        self.partida.acabou_de_trocar = 0;
    }

    /// Resumo de modos (corpo de `_TallyModePop`): `modos` das colônias a partir de `modos_do_tick`.
    pub fn resumir_modos(&mut self) {
        for cor in [Cor::Preta, Cor::Vermelha] {
            let t = self.modos_do_tick[indice_cor(cor)];
            self.colonia_mut(cor).modos = [t[2] + t[3], t[4] + t[5], t[1], t[7], t[12], t[6]];
        }
    }

    /// `_HistUpdate` (`1000:9182`, simulacao-visao-geral.md §5.1): grava uma amostra na
    /// posição `inicio` do anel e avança.
    pub fn atualizar_historico(&mut self) {
        let i = (self.historico.inicio as usize) & 0x3F;
        let h = &mut self.historico;
        h.populacao_b[i] = self.preta.populacao;
        h.populacao_r[i] = self.vermelha.populacao;
        h.comida_b[i] = self.preta.comida;
        h.comida_r[i] = self.vermelha.comida;
        h.saude_b[i] = self.preta.saude;
        h.saude_r[i] = self.vermelha.saude;
        h.comida_superficie[i] = self.ambiente.comida_superficie;
        // As três séries de mortes guardam só a palavra baixa dos contadores de 32 bits.
        h.mortes_naturais[i] = self.preta.mortes_naturais as i16;
        h.mortes_por_predadores[i] = self.preta.comidas_por_predadores as i16;
        h.mortes_em_luta[i] = self.preta.mortas_em_luta as i16;
        if h.contagem < 0x3F {
            h.contagem += 1;
        }
        h.inicio = (h.inicio + 1) & 0x3F;
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn f(tipo: u8) -> Formiga {
        Formiga { x: tipo, y: 1, modo: 2, tipo, s: 0 }
    }

    #[test]
    fn compactar_mantem_ordem_e_deixa_o_resto() {
        let mut l = ListaFormigas::vazia(6);
        for t in [5, 0, 7, 0, 9] {
            l.adicionar(f(t));
        }
        l.compactar();
        assert_eq!(l.indice, 3);
        let tipos: Vec<u8> = l.vagas.iter().map(|f| f.tipo).collect();
        // As duas vagas depois do índice ficam com o que já estava lá (como no original).
        assert_eq!(tipos, vec![5, 7, 9, 0, 9, 0]);
    }

    #[test]
    fn lista_cheia_descarta() {
        let mut l = ListaFormigas::vazia(2);
        assert!(l.adicionar(f(1)));
        assert!(l.adicionar(f(2)));
        assert!(!l.adicionar(f(3)));
        assert_eq!(l.indice, 2);
    }

    #[test]
    fn censo_e_morte_da_rainha_no_quick() {
        let mut e = Estado::vazio();
        e.partida.tipo_jogo = 1;
        e.amarela.modo = 1; // aranha: não entra no censo
        e.adicionar_formiga(Lista::B, Formiga { x: 3, y: 3, modo: 9, tipo: 0x62, s: 0 });
        e.adicionar_formiga(Lista::B, Formiga { x: 4, y: 3, modo: 9, tipo: 0x6A, s: 0 });
        e.adicionar_formiga(Lista::A, Formiga { x: 4, y: 3, modo: 2, tipo: 0x12, s: 0 });
        e.adicionar_formiga(Lista::A, Formiga { x: 5, y: 3, modo: 2, tipo: 0xB4, s: 0 });
        e.contar_formigas();
        assert_eq!(e.preta.castas, [0, 1, 0, 0, 0, 1]);
        assert_eq!(e.preta.populacao, 2);
        assert_eq!(e.vermelha.castas, [0, 0, 1, 0, 0, 0]);
        assert_eq!(e.partida.fim_de_jogo, 0);

        e.ninho_b.lista.vagas[0].tipo = 0;
        e.contar_formigas();
        assert_eq!(e.partida.fim_de_jogo, 1);
        assert_eq!(e.partida.pretas_venceram, 0);
        assert_eq!(e.eventos, vec![Evento::Musica(0x2B0C), Evento::Mensagem(10010), Evento::Mensagem(10011)]);
    }

    #[test]
    fn historico_avanca_no_anel() {
        let mut e = Estado::vazio();
        e.historico.inicio = 63;
        e.preta.populacao = 7;
        e.atualizar_historico();
        assert_eq!((e.historico.inicio, e.historico.contagem), (0, 1));
        assert_eq!(e.historico.populacao_b[63], 7);
    }
}
