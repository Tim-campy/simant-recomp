//! `data/som.json`: tabela id → arquivo de música, taxas, volumes e canais. Ver
//! docs/spec/regras/som.md e docs/api-mods.md.

use std::collections::BTreeMap;

use serde::Deserialize;

pub const ARQUIVO: &str = "som.json";

/// Sintetizador das músicas.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TipoSintetizador {
    /// Ondas básicas por família GM (`sintetizador.rs`).
    Simples,
    /// `rustysynth` com a SoundFont de `soundfont` (sem ela, cai no simples).
    Soundfont,
    /// FM no estilo do OPL2/OPL3 com o banco de `fm` (`fm.rs`), o timbre da Sound Blaster.
    Fm,
}

impl TipoSintetizador {
    pub fn de_nome(nome: &str) -> Option<TipoSintetizador> {
        match nome {
            "simples" => Some(TipoSintetizador::Simples),
            "soundfont" => Some(TipoSintetizador::Soundfont),
            "fm" => Some(TipoSintetizador::Fm),
            _ => None,
        }
    }
}

/// Caminho FM: banco de timbres e ganho.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigFm {
    /// Banco JSON do port, lido de `data/` (sobrescrevível por mod).
    #[serde(default = "banco_padrao")]
    pub banco: String,
    /// Banco `.op2` (DMX/GENMIDI) do usuário, lido em tempo de execução; tem precedência sobre `banco`.
    #[serde(default)]
    pub banco_op2: Option<String>,
    /// Multiplica o `ganho` do banco.
    #[serde(default = "ganho_padrao")]
    pub ganho: f32,
}

fn banco_padrao() -> String {
    "opl_banco.json".into()
}
fn ganho_padrao() -> f32 {
    1.0
}

impl Default for ConfigFm {
    fn default() -> Self {
        ConfigFm { banco: banco_padrao(), banco_op2: None, ganho: 1.0 }
    }
}

/// De onde vêm as músicas.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FonteMusica {
    /// `<pasta_musicas>/<nome>.mid` pela tabela `musicas` (modo MCI do original).
    Arquivos,
    /// Objeto tipo 20 indicado pelo cabeçalho tipo 18, com a transposição dele (modo Sound Blaster).
    Banco,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigSom {
    /// Taxa dos efeitos tipo 5 (o original abre o `waveOut` a 4096 Hz).
    pub taxa_efeitos: u32,
    /// Taxa pedida ao dispositivo de áudio e usada ao exportar músicas.
    pub taxa_saida: u32,
    /// 0 a 1.
    pub volume_musica: f32,
    pub volume_efeitos: f32,
    pub fonte_musica: FonteMusica,
    /// Só toca a música se o cabeçalho tipo 18 existir, como `_myBeginSong`.
    pub exigir_cabecalho: bool,
    pub pasta_musicas: String,
    /// Canais MIDI tocados (1–16). Os `.MID` têm o arranjo duplo do Windows 3.1: 1–10 e 13–16.
    pub canais_midi: Vec<u8>,
    pub canal_percussao: u8,
    /// SoundFont (.sf2) para o `rustysynth`; sem ela, o sintetizador simples. Relativo à pasta atual.
    #[serde(default)]
    pub soundfont: Option<String>,
    /// Sintetizador (API 0.30). Ausente: SoundFont se houver uma, senão o simples (como até a 0.29).
    #[serde(default)]
    pub sintetizador: Option<TipoSintetizador>,
    #[serde(default)]
    pub fm: ConfigFm,
    pub musica_abertura: Option<u16>,
    pub musica_jogo_novo: Option<u16>,
    /// Tempo de soltura depois da última nota, ao exportar (segundos).
    pub cauda_exportacao_s: f32,
    /// `_TryAntTheme`: temas tocados ao entrar/sair de um ninho. Ausente = sem tema.
    #[serde(default)]
    pub tema_ninho: Option<TemaNinho>,
    /// Id do cabeçalho (decimal) → nome do arquivo sem extensão (tabela MCI do original).
    pub musicas: BTreeMap<u16, String>,
}

/// `_TryAntTheme` (`1020:1414`): um tema por vez, em ciclo, se já passou `intervalo_ms` desde o último
/// (7200 ticks de `_TickCount` ≈ 396 s). O relógio e o ciclo andam mesmo com a música desligada.
#[derive(Clone, Debug, Deserialize)]
pub struct TemaNinho {
    pub musicas: Vec<u16>,
    pub intervalo_ms: u64,
}
