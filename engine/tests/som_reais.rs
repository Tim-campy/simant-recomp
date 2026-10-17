//! Som e música contra os arquivos do usuário (docs/spec/formatos/som.md).
//! Pasta do jogo: SIMANT_DIR ou ../SimAnt-T/MAXIS/ANTWIN; sem os arquivos, os testes passam sem verificar.

use std::path::PathBuf;

use engine::formats::banco::{Banco, Chave};
use engine::formats::midi::Midi;
use engine::formats::som::{decodificar_efeito, CabecalhoMusica, TIPO_CABECALHO_MUSICA, TIPO_EFEITO, TIPO_MIDI_SOUNDBLASTER};

fn dir_jogo() -> Option<PathBuf> {
    let d = std::env::var_os("SIMANT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../SimAnt-T/MAXIS/ANTWIN"));
    if d.join("SOUND.NDX").is_file() {
        Some(d)
    } else {
        eprintln!("AVISO: arquivos do jogo não encontrados em {}; teste ignorado", d.display());
        None
    }
}

fn objetos(b: &Banco, tipo: u8) -> Vec<(u16, Vec<u8>)> {
    b.entradas()
        .iter()
        .filter(|e| e.chave.tipo == tipo)
        .map(|e| (e.chave.id, b.objeto(e.chave).unwrap().unwrap()))
        .collect()
}

#[test]
fn efeitos_tipo_5_decodificam_em_duas_amostras_por_byte() {
    let Some(d) = dir_jogo() else { return };
    let b = Banco::abrir(&d.join("SOUND")).unwrap();
    let efeitos = objetos(&b, TIPO_EFEITO);
    assert_eq!(efeitos.len(), 57);
    assert_eq!(efeitos.iter().map(|e| e.0).collect::<Vec<_>>(), (0..57).collect::<Vec<u16>>());
    for (id, dados) in &efeitos {
        let pcm = decodificar_efeito(dados).unwrap();
        assert_eq!(pcm.len(), 2 * (dados.len() - 16), "efeito {id}");
        let min = *pcm.iter().min().unwrap();
        let max = *pcm.iter().max().unwrap();
        assert!(max - min > 16, "efeito {id} quase mudo: {min}..{max}");
    }
    // Id 41: 6464 bytes → 12 896 amostras (≈ 3,1 s a 4096 Hz).
    let (_, d41) = efeitos.iter().find(|e| e.0 == 41).unwrap();
    assert_eq!(decodificar_efeito(d41).unwrap().len(), 12_896);
}

#[test]
fn cabecalhos_tipo_18_apontam_para_midis_do_tipo_20() {
    let Some(d) = dir_jogo() else { return };
    let b = Banco::abrir(&d.join("SOUND")).unwrap();
    let cab = objetos(&b, TIPO_CABECALHO_MUSICA);
    assert_eq!(cab.len(), 33);
    for (id, dados) in &cab {
        assert_eq!(dados.len(), 18, "cabeçalho {id}");
        let c = CabecalhoMusica::de_bytes(dados).unwrap();
        assert!(b.contem(Chave { tipo: TIPO_MIDI_SOUNDBLASTER, id: c.midi }), "cabeçalho {id} → MIDI {}", c.midi);
        let esperado = match id {
            11014 => (11013, 6),
            11015 => (11013, 12),
            11017 => (11016, 12),
            _ => (*id, 0),
        };
        assert_eq!((c.midi, c.transposicao), esperado, "cabeçalho {id}");
    }
    assert!(!b.contem(Chave { tipo: TIPO_CABECALHO_MUSICA, id: 20006 }), "jolly5 não tem cabeçalho");
}

#[test]
fn midis_do_banco_tipos_20_e_50_sao_lidos() {
    let Some(d) = dir_jogo() else { return };
    let b = Banco::abrir(&d.join("SOUND")).unwrap();
    for tipo in [20u8, 50] {
        let lista = objetos(&b, tipo);
        assert_eq!(lista.len(), if tipo == 20 { 30 } else { 28 });
        for (id, dados) in lista {
            let m = Midi::de_bytes(&dados).unwrap_or_else(|e| panic!("tipo {tipo} id {id}: {e}"));
            let todos: Vec<u8> = (1..=16).collect();
            assert!(!m.sequencia(&todos, 0, 10).is_empty(), "tipo {tipo} id {id} sem mensagens");
        }
    }
}

#[test]
fn arquivos_mid_tem_arranjo_duplo_do_windows_31() {
    let Some(d) = dir_jogo() else { return };
    let pasta = std::fs::read_dir(&d).unwrap().flatten().find(|e| e.file_name().eq_ignore_ascii_case("sound")).unwrap().path();
    let mut n = 0;
    for e in std::fs::read_dir(pasta).unwrap().flatten() {
        let m = Midi::de_bytes(&std::fs::read(e.path()).unwrap()).unwrap();
        let nome = e.file_name().to_string_lossy().into_owned();
        assert_eq!((m.formato, m.divisao), (1, 240), "{nome}");
        // Notas ligadas por canal: o arranjo básico (13, 14, 15, 16) repete o estendido (3, 4, 5, 10).
        let notas = |canais: &[u8]| -> Vec<usize> {
            let s = m.sequencia(canais, 0, 10);
            canais.iter().map(|&c| s.iter().filter(|ev| ev.canal() == c && ev.comando() == 0x90).count()).collect()
        };
        assert_eq!(notas(&[3, 4, 5, 10]), notas(&[13, 14, 15, 16]), "{nome}");
        assert!(notas(&[1, 2, 6, 7, 8, 9, 11, 12]).iter().all(|&x| x == 0), "{nome}: canais fora dos dois arranjos");
        let s = m.sequencia(&(1..=10).collect::<Vec<u8>>(), 0, 10);
        assert!(s.last().unwrap().segundos > 1.0, "{nome}: curta demais");
        n += 1;
    }
    assert_eq!(n, 29);
}
