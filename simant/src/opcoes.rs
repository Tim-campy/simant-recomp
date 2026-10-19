//! Linha de comando do executável.

use super::*;

#[derive(Clone)]
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
    /// Escala inteira da janela (`--escala`; 0 = a maior que cabe); vence `data/video.json`.
    pub(crate) escala: Option<u32>,
    /// Tela virtual (`--resolucao LxA`: 640x480, 800x600 ou 1024x768); vence `data/video.json`.
    pub(crate) resolucao: Option<(u32, u32)>,
    /// Layout da tela virtual (data/resolucao.json), resolvido em `principal`.
    pub(crate) tela_virtual: Option<engine::core::resolucao::TelaVirtual>,
    /// Tamanho inicial da janela (`--janela LxA`); vence `data/video.json`.
    pub(crate) janela: Option<(u32, u32)>,
    /// Opções de vídeo resolvidas (`data/video.json` + linha de comando), preenchidas em `principal`.
    pub(crate) video: video::ConfigVideo,
    /// Arquivo de preferências do jogador (`--preferencias ARQ`; padrão: a pasta de dados do usuário,
    /// resolvida em `principal` só para a partida com janela; `--sem-preferencias` desliga).
    pub(crate) preferencias: Option<PathBuf>,
    pub(crate) sem_preferencias: bool,
    /// Quick Game gerado pelo port.
    pub(crate) novo_jogo: bool,
    /// Semente do Quick Game (padrão: relógio).
    pub(crate) semente: Option<u32>,
    /// Roda N ticks sem janela (antes de exportar).
    pub(crate) ticks: Option<u64>,
    /// Sem janela, depois dos `--ticks`: passa tanto tempo real (ms) na partida, para exportar um
    /// quadro de uma animação da amarela em andamento.
    pub(crate) avancar_ms: Option<u64>,
    /// Tipo do jogo novo (`--tipo`; padrão: Quick Game).
    pub(crate) tipo: TipoJogo,
    /// Abre a janela do quintal nesse modo (0–3) antes de mostrar ou exportar.
    pub(crate) quintal: Option<i16>,
    /// Grava o estado num .ANT (sem janela: no fim; na janela: destino do "Save As").
    pub(crate) salvar: Option<PathBuf>,
    /// Sem janela: fecha os N primeiros avisos modais antes de exportar.
    pub(crate) fechar_avisos: usize,
    /// Sem janela: funções de botões da faixa (`_DoUserButton`) apertados antes de exportar.
    pub(crate) botoes: Vec<u16>,
    /// Sem janela: abre a History e alterna os botões de gráfico B (0–9, `histBPop`…) antes de exportar.
    pub(crate) historico: Vec<usize>,
    /// Sem janela: abre um menu antes de exportar ("amarela@X,Y" ou "icones@OBJETO").
    pub(crate) menu: Option<String>,
    /// Sem janela: Ctrl+clique no marcador N (1–7) antes de exportar.
    pub(crate) marcar: Option<usize>,
    /// Sem janela: abre um diálogo antes de exportar (load, pergunta, salvar_como, salvo).
    pub(crate) dialogo: Option<String>,
    /// Sem janela: abre a janela do mapa antes de exportar, no `_MapMode` dado (1 a 8).
    pub(crate) janela_mapa: Option<i16>,
    /// Sem janela: liga as trilhas (`_showTrails`) antes dos `--ticks`.
    pub(crate) trilhas: bool,
    /// Sem janela: abre o mini mapa (`ribToYellow`) N vezes depois dos `--ticks` (a última fica
    /// aberta; da segunda abertura em diante ele fica uma coluna à direita).
    pub(crate) mini_mapa: u32,
    /// Sem janela: Examine na célula X,Y do plano atual antes de exportar.
    pub(crate) examinar: Option<(i32, i32)>,
    /// Com --examinar: a janela já foi aberta antes nesta sessão (sem o ajuste da primeira abertura).
    pub(crate) exame_ja_aberto: bool,
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
        resolucao: None,
        tela_virtual: None,
        janela: None,
        video: video::ConfigVideo::default(),
        preferencias: None,
        sem_preferencias: false,
        novo_jogo: false,
        semente: None,
        ticks: None,
        avancar_ms: None,
        tipo: TipoJogo::Rapido,
        quintal: None,
        salvar: None,
        fechar_avisos: 0,
        botoes: Vec::new(),
        historico: Vec::new(),
        menu: None,
        marcar: None,
        dialogo: None,
        janela_mapa: None,
        trilhas: false,
        mini_mapa: 0,
        examinar: None,
        exame_ja_aberto: false,
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
            "--escala" => o.escala = Some(valor()?.parse().map_err(|_| format!("{a}: inteiro (0 = a maior que cabe)"))?),
            "--janela" => o.janela = Some(video::ler_tamanho("--janela", &valor()?)?),
            "--resolucao" => o.resolucao = Some(video::ler_tamanho("--resolucao", &valor()?)?),
            "--preferencias" => o.preferencias = Some(valor()?.into()),
            "--sem-preferencias" => o.sem_preferencias = true,
            "--capturar-janela" => o.capturar_janela = Some(valor()?.into()),
            "--novo-jogo" => o.novo_jogo = true,
            "--semente" => o.semente = Some(valor()?.parse().map_err(|_| format!("{a}: número inválido"))?),
            "--ticks" => o.ticks = Some(valor()?.parse().map_err(|_| format!("{a}: número inválido"))?),
            "--avancar-ms" => o.avancar_ms = Some(valor()?.parse().map_err(|_| format!("{a}: número inválido"))?),
            "--quintal" => o.quintal = Some(valor()?.parse().map_err(|_| format!("{a}: modo 0 a 3"))?),
            "--tipo" => {
                let v = valor()?;
                o.tipo = engine::core::selecao::Escolha::de_nome(&v)
                    .and_then(|e| e.tipo_jogo())
                    .ok_or(format!("{a}: use tutorial, rapido, completo ou experimental"))?;
                o.novo_jogo = true;
            }
            "--salvar" => o.salvar = Some(valor()?.into()),
            "--menu" => o.menu = Some(valor()?),
            "--dialogo" => o.dialogo = Some(valor()?),
            "--janela-mapa" => o.janela_mapa = Some(valor()?.parse().ok().filter(|m| (1..=8).contains(m)).ok_or(format!("{a}: modo 1 a 8"))?),
            "--trilhas" => o.trilhas = true,
            "--mini-mapa" => o.mini_mapa = valor()?.parse().map_err(|_| format!("{a}: número de aberturas"))?,
            "--exame-ja-aberto" => o.exame_ja_aberto = true,
            "--examinar" => {
                let v = valor()?;
                let (x, y) = v.split_once(',').ok_or("--examinar X,Y")?;
                o.examinar = Some((x.parse().map_err(|_| "--examinar: x")?, y.parse().map_err(|_| "--examinar: y")?));
            }
            "--marcar" => o.marcar = Some(valor()?.parse().map_err(|_| format!("{a}: número inválido"))?),
            "--historico" => {
                o.historico = valor()?.split(',').map(|b| b.trim().parse().map_err(|_| format!("{a}: número inválido"))).collect::<Result<_, _>>()?
            }
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
