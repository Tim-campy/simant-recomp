//! `data/som.json`: tabela id → arquivo de música, taxas, volumes e canais. Ver
//! docs/spec/regras/som.md e docs/api-mods.md.

use std::collections::BTreeMap;

use serde::Deserialize;

pub const ARQUIVO: &str = "som.json";

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
    pub musica_abertura: Option<u16>,
    pub musica_jogo_novo: Option<u16>,
    /// Tempo de soltura depois da última nota, ao exportar (segundos).
    pub cauda_exportacao_s: f32,
    /// Id do cabeçalho (decimal) → nome do arquivo sem extensão (tabela MCI do original).
    pub musicas: BTreeMap<u16, String>,
}
