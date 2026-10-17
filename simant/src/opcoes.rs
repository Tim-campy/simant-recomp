//! Linha de comando do executável.

use super::*;

pub(crate) struct Opcoes {
    pub(crate) jogo: PathBuf,
    pub(crate) dados: PathBuf,
    pub(crate) mods: PathBuf,
    pub(crate) exportar: Option<PathBuf>,
    pub(crate) save: Option<PathBuf>,
    pub(crate) exportar_tela_jogo: Option<PathBuf>,
    /// Ações aplicadas à navegação antes de mostrar ou exportar (nomes de data/navegacao.json).
    pub(crate) acoes: Vec<Acao>,
    /// Fecha a janela sozinha depois desse tempo (para testes sem interação).
    pub(crate) sair_apos_ms: Option<u64>,
    /// Com --sair-apos-ms, grava o que a janela mostra (resolução da janela) antes de fechar.
    pub(crate) capturar_janela: Option<PathBuf>,
    /// Escala inteira da janela; sem ela, a maior que cabe na área útil do monitor.
    pub(crate) escala: Option<u32>,
    /// Quick Game gerado pelo port.
    pub(crate) novo_jogo: bool,
    /// Semente do Quick Game (padrão: relógio).
    pub(crate) semente: Option<u32>,
    /// Roda N ticks sem janela (antes de exportar).
    pub(crate) ticks: Option<u64>,
    /// Grava o estado num .ANT (sem janela: no fim; na janela: destino do "Save As").
    pub(crate) salvar: Option<PathBuf>,
    /// Sem janela: fecha os N primeiros avisos modais antes de exportar.
    pub(crate) fechar_avisos: usize,
    /// Sem janela: funções de botões da faixa (`_DoUserButton`) apertados antes de exportar.
    pub(crate) botoes: Vec<u16>,
    pub(crate) som: som::OpcoesSom,
}

pub(crate) fn ler_acoes(texto: &str) -> Result<Vec<Acao>, String> {
    texto
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| serde_json::from_value(serde_json::Value::String(s.trim().into())).map_err(|_| format!("ação desconhecida: {s}")))
        .collect()
}

pub(crate) fn ler_opcoes() -> Result<Opcoes, String> {
    let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(PathBuf::from("SimAnt-T/MAXIS/ANTWIN"));
    let mut o = Opcoes {
        jogo,
        dados: PathBuf::from("data"),
        mods: PathBuf::from("mods"),
        exportar: None,
        save: None,
        exportar_tela_jogo: None,
        acoes: Vec::new(),
        sair_apos_ms: None,
        capturar_janela: None,
        escala: None,
        novo_jogo: false,
        semente: None,
        ticks: None,
        salvar: None,
        fechar_avisos: 0,
        botoes: Vec::new(),
        som: som::OpcoesSom::default(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut valor = || args.next().ok_or(format!("falta o valor de {a}"));
        match a.as_str() {
            "--jogo" => o.jogo = valor()?.into(),
            "--dados" => o.dados = valor()?.into(),
            "--mods" => o.mods = valor()?.into(),
            "--exportar-telas" => o.exportar = Some(valor()?.into()),
            "--save" => o.save = Some(valor()?.into()),
            "--exportar-tela-jogo" => o.exportar_tela_jogo = Some(valor()?.into()),
            "--acoes" => o.acoes = ler_acoes(&valor()?)?,
            "--sair-apos-ms" => o.sair_apos_ms = Some(valor()?.parse().map_err(|_| format!("{a}: número inválido"))?),
            "--escala" => o.escala = Some(valor()?.parse().ok().filter(|&e| e > 0).ok_or(format!("{a}: inteiro positivo"))?),
            "--capturar-janela" => o.capturar_janela = Some(valor()?.into()),
            "--novo-jogo" => o.novo_jogo = true,
            "--semente" => o.semente = Some(valor()?.parse().map_err(|_| format!("{a}: número inválido"))?),
            "--ticks" => o.ticks = Some(valor()?.parse().map_err(|_| format!("{a}: número inválido"))?),
            "--salvar" => o.salvar = Some(valor()?.into()),
            "--botoes" => {
                o.botoes = valor()?.split(',').map(|b| b.trim().parse().map_err(|_| format!("{a}: número inválido"))).collect::<Result<_, _>>()?
            }
            "--fechar-avisos" => o.fechar_avisos = valor()?.parse().map_err(|_| format!("{a}: número inválido"))?,
            outro if o.som.tratar(outro, &mut valor)? => {}
            _ => return Err(format!("opção desconhecida: {a}")),
        }
    }
    let partida = o.save.is_some() || o.novo_jogo;
    if !partida && (o.exportar_tela_jogo.is_some() || !o.acoes.is_empty() || o.ticks.is_some() || o.salvar.is_some()) {
        return Err("--exportar-tela-jogo, --acoes, --ticks e --salvar precisam de --save ou --novo-jogo".into());
    }
    if o.save.is_some() && o.novo_jogo {
        return Err("use --save ou --novo-jogo, não os dois".into());
    }
    if o.semente.is_some() && !o.novo_jogo {
        return Err("--semente precisa de --novo-jogo".into());
    }
    Ok(o)
}

/// Semente do Quick Game: a da linha de comando ou o relógio (milissegundos desde 1970, como o
/// `_TickCount` do original só importa a palavra baixa).
pub(crate) fn semente(o: &Opcoes) -> u32 {
    o.semente.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u32)
            .unwrap_or(0)
    })
}
