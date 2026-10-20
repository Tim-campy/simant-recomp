//! Som e música do executável: lê os efeitos e as músicas pelos `Recursos`, mistura e toca com
//! SDL2. Especificação: docs/spec/regras/som.md e docs/spec/formatos/som.md.
//!
//! Opções de linha de comando (tratadas aqui): `--sem-som`, `--soundfont ARQ.sf2`,
//! `--sintetizador simples|soundfont|fm`, `--exportar-musica ID` (com `--exportar-som`, só essa),
//! `--exportar-som DIR` (grava os efeitos e as músicas em WAV e sai, sem abrir áudio).

pub mod banco_fm;
pub mod config;
pub mod controle;
pub mod fm;
pub mod mixer;
pub mod sintetizador;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use engine::formats::banco::Chave;
use engine::formats::midi::{EventoMidi, Midi};
use engine::formats::som::{self, CabecalhoMusica, TIPO_CABECALHO_MUSICA, TIPO_EFEITO, TIPO_MIDI_SOUNDBLASTER};
use engine::recursos::Recursos;
use engine::rules::estado::Evento;

use config::{ConfigSom, FonteMusica, TipoSintetizador};
use controle::{Comando, ControleSom};
use mixer::Mixer;
use banco_fm::Banco;
use fm::SintFm;
use sintetizador::{SintSimples, SintSoundFont, Sintetizador};

/// Opções de som da linha de comando.
#[derive(Clone, Debug, Default)]
pub struct OpcoesSom {
    pub desligado: bool,
    pub soundfont: Option<PathBuf>,
    pub exportar: Option<PathBuf>,
    pub sintetizador: Option<TipoSintetizador>,
    /// `--exportar-musica ID`: com `--exportar-som`, só essa música (e nenhum efeito).
    pub so_musica: Option<u16>,
}

impl OpcoesSom {
    /// Trata `arg` se for uma opção de som (pegando o valor com `valor`); `false` se não for.
    pub fn tratar(&mut self, arg: &str, valor: &mut dyn FnMut() -> Result<String, String>) -> Result<bool, String> {
        match arg {
            "--sem-som" => self.desligado = true,
            "--soundfont" => self.soundfont = Some(valor()?.into()),
            "--exportar-som" => self.exportar = Some(valor()?.into()),
            "--exportar-musica" => {
                let v = valor()?;
                self.so_musica = Some(v.parse().map_err(|_| format!("--exportar-musica: '{v}' is not an id"))?);
            }
            "--sintetizador" => {
                let v = valor()?;
                self.sintetizador = Some(
                    TipoSintetizador::de_nome(&v).ok_or_else(|| engine::tr!("cli.erro.escolha", opcao = "--sintetizador", valor = v, validos = "simples, soundfont, fm"))?,
                );
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

// ---------------------------------------------------------------------------------------------
// Leitura (sem SDL)
// ---------------------------------------------------------------------------------------------

/// Efeito tipo 5 decodificado (PCM de 8 bits sem sinal, na taxa `taxa_efeitos`).
pub fn efeito_pcm(recursos: &Recursos, id: u16) -> Result<Vec<u8>, String> {
    som::decodificar_efeito(&recursos.objeto(Chave { tipo: TIPO_EFEITO, id })?)
}

/// Eventos da música `id`. O `Err` de dentro traz o motivo quando o original não tocaria nada
/// (sem cabeçalho, fora da tabela); o de fora é erro de leitura.
pub fn musica(recursos: &Recursos, cfg: &ConfigSom, id: u16) -> Result<Result<Vec<EventoMidi>, String>, String> {
    let cabecalho = match recursos.objeto(Chave { tipo: TIPO_CABECALHO_MUSICA, id }) {
        Ok(b) => Some(CabecalhoMusica::de_bytes(&b)?),
        Err(_) => None,
    };
    if cfg.exigir_cabecalho && cabecalho.is_none() {
        return Ok(Err(format!("music {id}: no type 18 header")));
    }
    let (bytes, transposicao) = match cfg.fonte_musica {
        FonteMusica::Arquivos => {
            let Some(nome) = cfg.musicas.get(&id) else {
                return Ok(Err(format!("music {id}: not in the 'musicas' table")));
            };
            (recursos.arquivo(&format!("{}/{nome}.mid", cfg.pasta_musicas))?, 0)
        }
        FonteMusica::Banco => {
            let Some(c) = cabecalho else {
                return Ok(Err(format!("music {id}: no type 18 header")));
            };
            (recursos.objeto(Chave { tipo: TIPO_MIDI_SOUNDBLASTER, id: c.midi })?, c.transposicao)
        }
    };
    let midi = Midi::de_bytes(&bytes).map_err(|e| format!("music {id}: {e}"))?;
    Ok(Ok(midi.sequencia(&cfg.canais_midi, transposicao, cfg.canal_percussao)))
}

/// SoundFont da linha de comando, de `SIMANT_SOUNDFONT` ou de `data/som.json`, nessa ordem.
fn caminho_soundfont(cfg: &ConfigSom, opcoes: &OpcoesSom) -> Option<PathBuf> {
    opcoes
        .soundfont
        .clone()
        .or_else(|| std::env::var_os("SIMANT_SOUNDFONT").map(PathBuf::from))
        .or_else(|| cfg.soundfont.as_ref().map(PathBuf::from))
}

/// Banco FM: o `.op2` do usuário (`fm.banco_op2`), se houver, senão o JSON de `data/` (com mods).
pub fn banco_fm(recursos: &Recursos, cfg: &ConfigSom) -> Result<Banco, String> {
    let mut banco = match &cfg.fm.banco_op2 {
        Some(c) => Banco::de_op2(&std::fs::read(c).map_err(|e| format!("{c}: {e}"))?)?,
        None => {
            let b: Banco = recursos.dados(&cfg.fm.banco)?;
            b.validar()?;
            b
        }
    };
    banco.ganho *= cfg.fm.ganho;
    Ok(banco)
}

/// Sintetizador escolhido (linha de comando, senão `sintetizador` de `data/som.json`; sem nenhum dos
/// dois, SoundFont se houver uma, senão o simples). Uma SoundFont ou um banco FM que falhe cai no simples.
pub fn sintetizador(recursos: &Recursos, cfg: &ConfigSom, opcoes: &OpcoesSom, taxa: u32) -> Box<dyn Sintetizador> {
    let sf = caminho_soundfont(cfg, opcoes);
    let tipo = opcoes.sintetizador.or(cfg.sintetizador).unwrap_or(if sf.is_some() {
        TipoSintetizador::Soundfont
    } else {
        TipoSintetizador::Simples
    });
    match tipo {
        TipoSintetizador::Soundfont => match sf {
            Some(sf) => match SintSoundFont::abrir(&sf, taxa) {
                Ok(s) => {
                    println!("{}", engine::tr!("jogo.som.soundfont", arquivo = sf.display()));
                    return Box::new(s);
                }
                Err(e) => eprintln!("{}", engine::tr!("jogo.som.soundfont_erro", erro = e)),
            },
            None => eprintln!("{}", engine::tr!("jogo.som.sem_soundfont")),
        },
        TipoSintetizador::Fm => match banco_fm(recursos, cfg) {
            Ok(b) => return Box::new(SintFm::new(b, taxa, cfg.canal_percussao)),
            Err(e) => eprintln!("{}", engine::tr!("jogo.som.fm_erro", erro = e)),
        },
        TipoSintetizador::Simples => {}
    }
    Box::new(SintSimples::new(taxa, cfg.canal_percussao))
}

// ---------------------------------------------------------------------------------------------
// Reprodução (SDL2)
// ---------------------------------------------------------------------------------------------

struct Saida(Mixer);

impl sdl2::audio::AudioCallback for Saida {
    type Channel = f32;
    fn callback(&mut self, out: &mut [f32]) {
        self.0.render(out);
    }
}

/// Som de uma janela do jogo. Sem dispositivo (erro ou `--sem-som`), tudo vira no-op. As decisões
/// (opções, foco, linha do tempo dos efeitos) ficam em [`ControleSom`]; aqui só se executam.
pub struct Audio<'a> {
    recursos: &'a Recursos,
    cfg: Option<ConfigSom>,
    dispositivo: Option<sdl2::audio::AudioDevice<Saida>>,
    taxa: u32,
    efeitos: HashMap<u16, Vec<f32>>,
    controle: ControleSom,
    inicio: std::time::Instant,
    /// `_TryAntTheme`: último tema (relógio de parede) e posição no ciclo.
    ultimo_tema: Option<std::time::Instant>,
    proximo_tema: usize,
}

impl<'a> Audio<'a> {
    pub fn abrir(sdl: &sdl2::Sdl, recursos: &'a Recursos, opcoes: &OpcoesSom) -> Audio<'a> {
        let mut a = Audio {
            recursos,
            cfg: None,
            dispositivo: None,
            taxa: 0,
            efeitos: HashMap::new(),
            controle: ControleSom::default(),
            inicio: std::time::Instant::now(),
            ultimo_tema: None,
            proximo_tema: 0,
        };
        if opcoes.desligado {
            return a;
        }
        if let Err(e) = a.iniciar(sdl, opcoes) {
            eprintln!("{}", engine::tr!("jogo.som.desligado", erro = e));
        }
        a
    }

    fn iniciar(&mut self, sdl: &sdl2::Sdl, opcoes: &OpcoesSom) -> Result<(), String> {
        let cfg: ConfigSom = self.recursos.dados(config::ARQUIVO)?;
        let audio = sdl.audio()?;
        let pedido = sdl2::audio::AudioSpecDesired { freq: Some(cfg.taxa_saida as i32), channels: Some(2), samples: Some(1024) };
        let mut taxa = cfg.taxa_saida;
        let dispositivo = audio.open_playback(None, &pedido, |spec| {
            taxa = spec.freq as u32;
            Saida(Mixer::new(taxa, sintetizador(self.recursos, &cfg, opcoes, taxa), cfg.volume_musica, cfg.volume_efeitos))
        })?;
        dispositivo.resume();
        self.taxa = taxa;
        self.cfg = Some(cfg);
        self.dispositivo = Some(dispositivo);
        Ok(())
    }

    fn agora_ms(&self) -> u64 {
        self.inicio.elapsed().as_millis() as u64
    }

    /// Executa os comandos decididos pelo controle.
    fn executar(&mut self, comandos: Vec<Comando>) {
        for c in comandos {
            match c {
                Comando::TocarMusica(id) => self.tocar_musica(id),
                Comando::PararMusica => self.parar_musica(),
                Comando::TocarEfeito(id) => self.tocar_efeito(id),
                Comando::PararEfeito => {
                    if let Some(d) = &mut self.dispositivo {
                        d.lock().0.parar_efeito();
                    }
                }
            }
        }
    }

    /// Toca a música `id` (cabeçalho tipo 18) no dispositivo; a nova corta a anterior.
    fn tocar_musica(&mut self, id: u16) {
        let (Some(cfg), Some(_)) = (&self.cfg, &self.dispositivo) else { return };
        match musica(self.recursos, cfg, id) {
            Ok(Ok(eventos)) => {
                if let Some(d) = &mut self.dispositivo {
                    d.lock().0.tocar_musica(eventos);
                }
            }
            Ok(Err(motivo)) => println!("sound: {motivo} (the original does not play it either)"),
            Err(e) => eprintln!("sound: {e}"),
        }
    }

    /// Toca o efeito `id` (tipo 5) no dispositivo; o novo corta o anterior.
    fn tocar_efeito(&mut self, id: u16) {
        let Some(cfg) = &self.cfg else { return };
        if self.dispositivo.is_none() {
            return;
        }
        if !self.efeitos.contains_key(&id) {
            match efeito_pcm(self.recursos, id) {
                Ok(pcm) => {
                    let tocado = mixer::blocos_do_driver(&pcm, cfg.bloco_efeitos);
                    self.efeitos.insert(id, mixer::reamostrar(tocado, cfg.taxa_efeitos, self.taxa));
                }
                Err(e) => return eprintln!("sound: effect {id}: {e}"),
            }
        }
        if let (Some(d), Some(a)) = (&mut self.dispositivo, self.efeitos.get(&id)) {
            d.lock().0.tocar_efeito(a.clone());
        }
    }

    /// Música da abertura (`_ShowIntro`), pedida ao abrir a janela: toca quando ela estiver ativa.
    pub fn musica_abertura(&mut self) {
        if let Some(id) = self.cfg.as_ref().and_then(|c| c.musica_abertura) {
            let c = self.controle.musica_inicial(id);
            self.executar(c);
        }
    }

    /// Música do jogo novo. `inicial`: pedida ao abrir a janela da partida (espera o primeiro foco).
    pub fn musica_jogo_novo(&mut self, inicial: bool) {
        if let Some(id) = self.cfg.as_ref().and_then(|c| c.musica_jogo_novo) {
            let c = if inicial { self.controle.musica_inicial(id) } else { self.controle.musica(id) };
            self.executar(c);
        }
    }

    /// Pedidos das regras de um quadro e as opções Music/Sound Effects da partida
    /// (`_OptionStates[1]` e `[2]`; desligar a música para a que toca). Também solta os efeitos
    /// agendados que venceram: chamar a cada quadro, mesmo sem eventos.
    pub fn tratar(&mut self, eventos: &[Evento], opcoes: &[i16]) {
        let musica = opcoes.get(1).is_none_or(|&v| v != 0);
        let efeitos = opcoes.get(2).is_none_or(|&v| v != 0);
        let c = self.controle.opcoes(musica, efeitos);
        self.executar(c);
        let agora = self.agora_ms();
        let c = self.controle.receber(eventos, agora);
        self.executar(c);
        if eventos.iter().any(|e| matches!(e, Evento::TemaDoNinho)) {
            self.tema_do_ninho(std::time::Instant::now());
        }
    }

    /// `_TryAntTheme`: toca o próximo tema do ciclo se já passou o intervalo desde o último.
    fn tema_do_ninho(&mut self, agora: std::time::Instant) {
        let Some(tema) = self.cfg.as_ref().and_then(|c| c.tema_ninho.clone()) else { return };
        if let Some((id, quando)) = proximo_tema(&tema, self.ultimo_tema, self.proximo_tema, agora) {
            self.ultimo_tema = Some(quando);
            self.proximo_tema += 1;
            let c = self.controle.musica(id);
            self.executar(c);
        }
    }

    /// Janela ativa ou não (`WM_ACTIVATEAPP`): sem foco, para a música, corta o efeito e silencia
    /// até voltar. Chamar a cada quadro com o estado atual da janela (só as mudanças contam).
    pub fn foco(&mut self, ativo: bool) {
        let agora = self.agora_ms();
        let c = self.controle.foco(ativo, agora);
        self.executar(c);
    }

    pub fn parar_musica(&mut self) {
        if let Some(d) = &mut self.dispositivo {
            d.lock().0.parar_musica();
        }
    }
}

/// Decide o tema do ninho: `Some((id, agora))` se é hora (primeira vez ou intervalo vencido).
pub fn proximo_tema(
    tema: &config::TemaNinho,
    ultimo: Option<std::time::Instant>,
    indice: usize,
    agora: std::time::Instant,
) -> Option<(u16, std::time::Instant)> {
    if tema.musicas.is_empty() {
        return None;
    }
    let vencido = ultimo.is_none_or(|u| agora.duration_since(u).as_millis() as u64 >= tema.intervalo_ms);
    vencido.then(|| (tema.musicas[indice % tema.musicas.len()], agora))
}

// ---------------------------------------------------------------------------------------------
// Exportação para WAV (sem abrir áudio)
// ---------------------------------------------------------------------------------------------

/// Renderiza uma sequência inteira (mais a cauda) em estéreo intercalado, com o volume `volume`.
pub fn renderizar_musica(eventos: Vec<EventoMidi>, sint: Box<dyn Sintetizador>, taxa: u32, cauda_s: f32, volume: f32) -> Vec<f32> {
    let mut m = Mixer::new(taxa, sint, volume, 0.0);
    m.tocar_musica(eventos);
    let mut saida = Vec::new();
    let mut bloco = vec![0.0; 2 * 1024];
    while m.musica_tocando() {
        m.render(&mut bloco);
        saida.extend_from_slice(&bloco);
    }
    let mut cauda = vec![0.0; 2 * (cauda_s * taxa as f32) as usize];
    m.render(&mut cauda);
    saida.extend_from_slice(&cauda);
    saida
}

/// Grava `DIR/efeitos/efeito_NNN.wav` (PCM decodificado, exato, na taxa em que soa, `taxa_efeitos`),
/// `DIR/efeitos_tocados/efeito_NNN.wav` (o trecho que o driver toca, com `bloco_efeitos`; só se a
/// chave existir) e `DIR/musicas/<id>_<nome>.wav` (sintetizado, com `volume_musica`, o nível em que toca no jogo; os
/// efeitos saem sem o `volume_efeitos`). Não toca nada.
pub fn exportar(recursos: &Recursos, opcoes: &OpcoesSom, dir: &Path) -> Result<(), String> {
    let cfg: ConfigSom = recursos.dados(config::ARQUIVO)?;
    let (dir_e, dir_m, dir_t) = (dir.join("efeitos"), dir.join("musicas"), dir.join("efeitos_tocados"));
    let blocos = cfg.bloco_efeitos.filter(|&b| b > 0 && opcoes.so_musica.is_none());
    for d in [&dir_e, &dir_m].into_iter().chain(blocos.map(|_| &dir_t)) {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    let gravar = |c: PathBuf, b: Vec<u8>| std::fs::write(&c, b).map_err(|e| format!("{}: {e}", c.display()));
    let mut n = 0;
    for id in (0..=1023u16).filter(|_| opcoes.so_musica.is_none()) {
        let Ok(pcm) = efeito_pcm(recursos, id) else { continue };
        gravar(dir_e.join(format!("efeito_{id:03}.wav")), som::wav(&pcm, cfg.taxa_efeitos, 1, 8))?;
        if blocos.is_some() {
            let tocado = mixer::blocos_do_driver(&pcm, blocos);
            gravar(dir_t.join(format!("efeito_{id:03}.wav")), som::wav(tocado, cfg.taxa_efeitos, 1, 8))?;
        }
        n += 1;
    }
    println!("{n} effects in {}", dir_e.display());
    let mut ids: Vec<u16> = cfg.musicas.keys().copied().collect();
    if cfg.fonte_musica == FonteMusica::Banco {
        ids.extend((10000..=20999).filter(|&id| recursos.objeto(Chave { tipo: TIPO_CABECALHO_MUSICA, id }).is_ok()));
        ids.sort();
        ids.dedup();
    }
    if let Some(so) = opcoes.so_musica {
        ids.retain(|&id| id == so);
    }
    for id in ids {
        match musica(recursos, &cfg, id)? {
            Ok(eventos) => {
                let pcm = renderizar_musica(
                    eventos,
                    sintetizador(recursos, &cfg, opcoes, cfg.taxa_saida),
                    cfg.taxa_saida,
                    cfg.cauda_exportacao_s,
                    cfg.volume_musica,
                );
                let nome = cfg.musicas.get(&id).map(String::as_str).unwrap_or("banco");
                let c = dir_m.join(format!("{id}_{nome}.wav"));
                println!("{} ({:.1} s)", c.display(), pcm.len() as f64 / 2.0 / cfg.taxa_saida as f64);
                gravar(c, som::wav_estereo_f32(&pcm, cfg.taxa_saida))?;
            }
            Err(motivo) => println!("{motivo}: not exported (the original does not play it)"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Recursos com os arquivos do usuário (SIMANT_DIR) e o `data/` do repositório; `None` sem eles.
    fn recursos() -> Option<Recursos> {
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or_else(|| raiz.join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SOUND.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", jogo.display());
            return None;
        }
        Some(Recursos::abrir(&jogo, &["SOUND".to_string()], &raiz.join("data"), None).unwrap())
    }

    #[test]
    fn tema_do_ninho_cicla_e_espera_o_intervalo() {
        let tema = config::TemaNinho { musicas: vec![10004, 10005, 10003], intervalo_ms: 396_000 };
        let t0 = std::time::Instant::now();
        assert_eq!(proximo_tema(&tema, None, 0, t0), Some((10004, t0)));
        let t1 = t0 + std::time::Duration::from_secs(100);
        assert_eq!(proximo_tema(&tema, Some(t0), 1, t1), None);
        let t2 = t0 + std::time::Duration::from_secs(396);
        assert_eq!(proximo_tema(&tema, Some(t0), 1, t2), Some((10005, t2)));
        assert_eq!(proximo_tema(&tema, None, 5, t2).map(|x| x.0), Some(10003));
    }

    #[test]
    fn opcoes_de_som_da_linha_de_comando() {
        let mut o = OpcoesSom::default();
        let mut valor = || Ok("x.sf2".to_string());
        assert!(o.tratar("--sem-som", &mut valor).unwrap());
        assert!(o.tratar("--soundfont", &mut valor).unwrap());
        assert!(!o.tratar("--escala", &mut valor).unwrap());
        assert!(o.desligado);
        assert_eq!(o.soundfont, Some(PathBuf::from("x.sf2")));
    }

    #[test]
    fn tabela_de_musicas_aponta_para_arquivos_existentes() {
        let Some(r) = recursos() else { return };
        let cfg: ConfigSom = r.dados(config::ARQUIVO).unwrap();
        assert_eq!(cfg.musicas.len(), 31);
        for (id, nome) in &cfg.musicas {
            assert!(r.arquivo(&format!("{}/{nome}.mid", cfg.pasta_musicas)).is_ok(), "{id} {nome}");
            let tocaria = musica(&r, &cfg, *id).unwrap();
            assert_eq!(tocaria.is_ok(), *id != 20006, "{id}: só jolly5 fica sem cabeçalho");
        }
        // 11015 tem cabeçalho mas não está na tabela MCI: não toca; no modo banco toca, transposta.
        assert!(musica(&r, &cfg, 11015).unwrap().is_err());
        let banco = ConfigSom { fonte_musica: FonteMusica::Banco, ..cfg.clone() };
        let (a, b) = (musica(&r, &banco, 11013).unwrap().unwrap(), musica(&r, &banco, 11015).unwrap().unwrap());
        let nota = |s: &[EventoMidi]| s.iter().find(|e| e.comando() == 0x90 && e.canal() != 10).unwrap().d1;
        assert_eq!(nota(&b), nota(&a) + 12);
    }

    #[test]
    fn exporta_musica_curta_sem_tocar() {
        let Some(r) = recursos() else { return };
        let cfg: ConfigSom = r.dados(config::ARQUIVO).unwrap();
        let eventos = musica(&r, &cfg, 11007).unwrap().unwrap();
        let fim = eventos.last().unwrap().segundos;
        let pcm = renderizar_musica(eventos, Box::new(SintSimples::new(11025, 10)), 11025, 0.5, 1.0);
        let duracao = pcm.len() as f64 / 2.0 / 11025.0;
        assert!(duracao >= fim + 0.4 && duracao < fim + 0.7, "{duracao} s para {fim} s de eventos");
        assert!(pcm.iter().any(|x| x.abs() > 0.05));
        let wav = som::wav_estereo_f32(&pcm, 11025);
        assert_eq!(wav.len(), 44 + pcm.len() * 2);
    }

    #[test]
    fn exporta_musica_pelo_caminho_fm_sem_saturar() {
        let Some(r) = recursos() else { return };
        let cfg: ConfigSom = r.dados(config::ARQUIVO).unwrap();
        // O padrão de data/som.json é o simples (escolha do usuário, 2026-10-19); o FM segue selecionável.
        assert_eq!(cfg.sintetizador, Some(TipoSintetizador::Simples), "data/som.json usa o simples por padrão");
        let banco = banco_fm(&r, &cfg).unwrap();
        // badnews (11007) é a música mais forte do banco; call2arm (11013), outra curta.
        for id in [11007u16, 11013] {
            let eventos = musica(&r, &cfg, id).unwrap().unwrap();
            let fim = eventos.last().unwrap().segundos;
            let sint = Box::new(SintFm::new(banco.clone(), 22_050, cfg.canal_percussao));
            let pcm = renderizar_musica(eventos, sint, 22_050, 0.5, cfg.volume_musica);
            let duracao = pcm.len() as f64 / 2.0 / 22_050.0;
            assert!(duracao >= fim + 0.4 && duracao < fim + 0.7, "{id}: {duracao} s para {fim} s de eventos");
            let pico = pcm.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            let rms = (pcm.iter().map(|v| (v * v) as f64).sum::<f64>() / pcm.len() as f64).sqrt();
            assert!(pico < 0.95, "{id}: pico {pico} (saturação)");
            assert!(rms > 0.01, "{id}: RMS {rms} (silêncio)");
        }
    }

    #[test]
    fn efeito_decodificado_e_reamostrado() {
        let Some(r) = recursos() else { return };
        let pcm = efeito_pcm(&r, 15).unwrap();
        assert_eq!(pcm.len(), 102);
        assert_eq!(mixer::reamostrar(&pcm, 4096, 44100).len(), 102 * 44100 / 4096);
    }

    /// Os efeitos soam como na sessão gravada no DOSBox-X (SB16): a 5000 Hz, em blocos de 504
    /// amostras (docs/spec/regras/som.md, "Efeitos dentro do jogo, gravados numa sessão").
    #[test]
    fn efeitos_soam_como_na_sound_blaster_16() {
        let Some(r) = recursos() else { return };
        let cfg: ConfigSom = r.dados(config::ARQUIVO).unwrap();
        assert_eq!(cfg.taxa_efeitos, 5000);
        assert_eq!(cfg.bloco_efeitos, Some(504));
        let tocado = |id: u16| mixer::blocos_do_driver(&efeito_pcm(&r, id).unwrap(), cfg.bloco_efeitos).len();
        // Medido (amostras tocadas): 0x2C 1512 de 1824, luta 0x25 1512 de 1888, erro 1 504 de 752,
        // formiga-leão 0x26 2520 de 2880, comida 0x20 3024 de 3374, gato 0xD inteiro (2016).
        for (id, n) in [(0x2C, 1512), (0x25, 1512), (1, 504), (0x26, 2520), (0x20, 3024), (0xD, 2016)] {
            assert_eq!(tocado(id), n, "efeito {id}");
        }
        // Nenhum dos efeitos com menos de 504 amostras apareceu na sessão (Home, cavar, comida, ovo).
        for id in [15u16, 17, 18, 19, 28, 29, 40] {
            assert_eq!(tocado(id), 0, "efeito {id}");
        }
    }
}
