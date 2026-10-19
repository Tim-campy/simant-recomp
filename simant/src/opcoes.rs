//! Linha de comando do executável.

use super::*;

/// Entrada no diálogo aberto por `--dialogo`, sem janela (na ordem da linha de comando).
#[derive(Clone, Debug)]
pub(crate) enum EntradaDialogo {
    /// X, Y (área cliente), duplo.
    Clique(i32, i32, bool),
    /// Aperta em (X0, Y0), move até (X1, Y1) e, com `true`, solta.
    Arrasto(i32, i32, i32, i32, bool),
    /// Tecla do SDL com Shift, Ctrl, Alt.
    Tecla(String, bool, bool, bool),
    /// Texto digitado.
    Texto(String),
}

#[derive(Clone)]
pub(crate) struct Opcoes {

    pub(crate) jogo: PathBuf,
    pub(crate) dados: PathBuf,
    pub(crate) mods: PathBuf,
    /// Mods que não carregam (`--desligar-mods a,b`; nomes do `mod.json`).
    pub(crate) mods_desligados: Vec<String>,
    /// Perfil de driver imitado (`--perfil-driver`; vence `data/resolucao.json` → `perfil_driver`).
    pub(crate) perfil_driver: Option<String>,
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
    /// Barra de título do Windows 3.1 (`--moldura` liga, `--sem-moldura` desliga); vence `data/video.json`.
    pub(crate) moldura: Option<bool>,
    /// Com a moldura do Windows 3.1, manter a do sistema (`--moldura-do-sistema`) ou substituí-la
    /// (`--sem-moldura-do-sistema`); vence `data/video.json` → `substituir_moldura_do_sistema`.
    pub(crate) substituir_moldura: Option<bool>,
    /// Ritmo de Fast/Ultra (`--velocidades original|proporcional`); vence `data/partida.json`.
    pub(crate) velocidades: Option<String>,
    /// Cadência sem foco (`--sem-foco lenta|original`); vence `data/partida.json`.
    pub(crate) sem_foco: Option<String>,
    /// Opções de vídeo resolvidas (`data/video.json` + linha de comando), preenchidas em `principal`.
    pub(crate) video: video::ConfigVideo,
    /// Arquivo de preferências do jogador (`--preferencias ARQ`; padrão: a pasta de dados do usuário,
    /// resolvida em `principal` só para a partida com janela; `--sem-preferencias` desliga).
    pub(crate) preferencias: Option<PathBuf>,
    pub(crate) sem_preferencias: bool,
    /// `WIN.INI` lido como origem das preferências enquanto não há o arquivo do port (`--win-ini ARQ`;
    /// padrão: procurado a partir da pasta do jogo, simant/src/preferencias.rs).
    pub(crate) win_ini: Option<PathBuf>,
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
    /// Sem janela, com --dialogo: cliques no diálogo (x, y na área cliente; duplo) antes de exportar.
    pub(crate) cliques_dialogo: Vec<EntradaDialogo>,
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

/// Valores de `--velocidades` (data/partida.json → `relogio.ritmo_velocidades.modo`).
pub(crate) const VELOCIDADES: [&str; 2] = ["proporcional", "original"];
/// Valores de `--sem-foco` (`lenta` = `relogio.sem_foco_velocidade` 0; `original` = `null`).
pub(crate) const SEM_FOCO: [&str; 2] = ["lenta", "original"];

fn ler_escolha(opcao: &str, v: String, validos: &[&str]) -> Result<String, String> {
    if validos.contains(&v.as_str()) {
        Ok(v)
    } else {
        Err(format!("{opcao}: '{v}' ({})", validos.join(" ou ")))
    }
}

/// O que `--velocidades` e `--sem-foco` mudam em `data/partida.json` (por cima dos mods), ou `None`.
pub(crate) fn sobreposicao_partida(velocidades: Option<&str>, sem_foco: Option<&str>) -> Option<serde_json::Value> {
    let mut relogio = serde_json::Map::new();
    if let Some(v) = velocidades {
        relogio.insert("ritmo_velocidades".into(), serde_json::json!({ "modo": v }));
    }
    if let Some(s) = sem_foco {
        relogio.insert("sem_foco_velocidade".into(), if s == "lenta" { serde_json::json!(0) } else { serde_json::Value::Null });
    }
    (!relogio.is_empty()).then(|| serde_json::json!({ "relogio": relogio }))
}

pub(crate) fn ler_acoes(texto: &str) -> Result<Vec<Acao>, String> {
    texto
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| serde_json::from_value(serde_json::Value::String(s.trim().into())).map_err(|_| format!("ação desconhecida: {s}")))
        .collect()
}

/// Pasta do jogo sem `--jogo` nem configuração: `SIMANT_DIR` ou `SimAnt-T/MAXIS/ANTWIN` na pasta
/// base do port (a do executável, se tiver `data/`; `pastas.rs`).
pub(crate) fn pasta_jogo_padrao(base: &Path) -> PathBuf {
    std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or_else(|| base.join("SimAnt-T").join("MAXIS").join("ANTWIN"))
}

/// Lê as opções `args` (sem o nome do programa; a configuração do launcher já combinada na frente,
/// `config_launcher::combinar`). Padrões das pastas: `data/`, `mods/` e o jogo relativos à pasta base
/// do port, não à pasta atual; caminhos relativos dados nas opções valem a partir da pasta atual ou,
/// se não existirem lá, da base (`pastas::resolver`).
pub(crate) fn ler_opcoes(args: Vec<String>) -> Result<Opcoes, String> {
    let base = crate::pastas::base();
    let mut o = Opcoes {
        jogo: pasta_jogo_padrao(&base),
        dados: base.join("data"),
        mods: base.join("mods"),
        mods_desligados: Vec::new(),
        perfil_driver: None,
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
        moldura: None,
        substituir_moldura: None,
        velocidades: None,
        sem_foco: None,
        video: video::ConfigVideo::default(),
        preferencias: None,
        sem_preferencias: false,
        win_ini: None,
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
        cliques_dialogo: Vec::new(),
        janela_mapa: None,
        trilhas: false,
        mini_mapa: 0,
        examinar: None,
        exame_ja_aberto: false,
        som: som::OpcoesSom::default(),
    };
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        let mut valor = || args.next().ok_or(format!("falta o valor de {a}"));
        match a.as_str() {
            "--jogo" => o.jogo = valor()?.into(),
            "--dados" => o.dados = valor()?.into(),
            "--mods" => o.mods = valor()?.into(),
            "--desligar-mods" => o.mods_desligados = crate::config_launcher::lista(&valor()?),
            "--perfil-driver" => o.perfil_driver = Some(valor()?),
            "--com-som" => o.som.desligado = false,
            "--exportar-telas" => o.exportar = Some(valor()?.into()),
            "--save" => o.save = Some(valor()?.into()),
            "--exportar-tela-jogo" => o.exportar_tela_jogo = Some(valor()?.into()),
            "--acoes" => o.acoes = ler_acoes(&valor()?)?,
            "--sair-apos-ms" => o.sair_apos_ms = Some(valor()?.parse().map_err(|_| format!("{a}: número inválido"))?),
            "--escala" => o.escala = Some(valor()?.parse().map_err(|_| format!("{a}: inteiro (0 = a maior que cabe)"))?),
            "--janela" => o.janela = Some(video::ler_tamanho("--janela", &valor()?)?),
            "--resolucao" => o.resolucao = Some(video::ler_tamanho("--resolucao", &valor()?)?),
            "--moldura" => o.moldura = Some(true),
            "--sem-moldura" => o.moldura = Some(false),
            "--moldura-do-sistema" => o.substituir_moldura = Some(false),
            "--sem-moldura-do-sistema" => o.substituir_moldura = Some(true),
            "--velocidades" => o.velocidades = Some(ler_escolha(&a, valor()?, &VELOCIDADES)?),
            "--sem-foco" => o.sem_foco = Some(ler_escolha(&a, valor()?, &SEM_FOCO)?),
            "--preferencias" => o.preferencias = Some(valor()?.into()),
            "--sem-preferencias" => o.sem_preferencias = true,
            "--win-ini" => o.win_ini = Some(valor()?.into()),
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
            "--clique-dialogo" => {
                let v = valor()?;
                let p: Vec<&str> = v.split(',').collect();
                let n = |s: &str| s.parse::<i32>().map_err(|_| format!("{a}: X,Y[,duplo]"));
                if p.len() < 2 {
                    return Err(format!("{a}: X,Y[,duplo]"));
                }
                o.cliques_dialogo.push(EntradaDialogo::Clique(n(p[0])?, n(p[1])?, p.get(2) == Some(&"duplo")));
            }
            "--arrastar-dialogo" => {
                // X0,Y0,X1,Y1[,solto]: aperta, move e (com "solto") solta.
                let v = valor()?;
                let p: Vec<&str> = v.split(',').collect();
                let n = |s: &str| s.parse::<i32>().map_err(|_| format!("{a}: X0,Y0,X1,Y1[,solto]"));
                if p.len() < 4 {
                    return Err(format!("{a}: X0,Y0,X1,Y1[,solto]"));
                }
                o.cliques_dialogo.push(EntradaDialogo::Arrasto(n(p[0])?, n(p[1])?, n(p[2])?, n(p[3])?, p.get(4) == Some(&"solto")));
            }
            "--tecla-dialogo" => {
                // NOME[+shift][+ctrl][+alt] (nome do SDL).
                let v = valor()?;
                let mut p = v.split('+');
                let nome = p.next().unwrap_or_default().to_string();
                let m: Vec<&str> = p.collect();
                o.cliques_dialogo.push(EntradaDialogo::Tecla(nome, m.contains(&"shift"), m.contains(&"ctrl"), m.contains(&"alt")));
            }
            "--digitar-dialogo" => o.cliques_dialogo.push(EntradaDialogo::Texto(valor()?)),
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
    o.jogo = crate::pastas::resolver(&o.jogo, &base);
    o.dados = crate::pastas::resolver(&o.dados, &base);
    o.mods = crate::pastas::resolver(&o.mods, &base);
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

#[cfg(test)]
mod testes {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// `--velocidades` e `--sem-foco` viram uma sobreposição de `data/partida.json` que, mesclada
    /// por cima do arquivo do port, volta às regras do original.
    #[test]
    fn ritmo_pela_linha_de_comando() {
        let o = ler_opcoes(args(&["--velocidades", "original", "--sem-foco", "original"])).unwrap();
        let v = sobreposicao_partida(o.velocidades.as_deref(), o.sem_foco.as_deref()).unwrap();
        let caminho = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/partida.json");
        let mut json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(caminho).unwrap()).unwrap();
        let padrao = ConfigPartida::de_json(&json.to_string()).unwrap();
        assert_eq!(padrao.relogio.ritmo_velocidades.modo, partida::ModoRitmo::Proporcional);
        assert_eq!(padrao.relogio.sem_foco_velocidade, Some(0));
        engine::recursos::mesclar(&mut json, v);
        let c = ConfigPartida::de_json(&json.to_string()).unwrap();
        assert_eq!(c.relogio.ritmo_velocidades.modo, partida::ModoRitmo::Original);
        assert_eq!(c.relogio.ritmo_velocidades.multiplicadores, padrao.relogio.ritmo_velocidades.multiplicadores);
        assert_eq!(c.relogio.sem_foco_velocidade, None);
        assert!(sobreposicao_partida(None, None).is_none());
        assert_eq!(sobreposicao_partida(None, Some("lenta")), Some(serde_json::json!({"relogio": {"sem_foco_velocidade": 0}})));
        assert!(ler_opcoes(args(&["--velocidades", "rapida"])).is_err());
        assert!(ler_opcoes(args(&["--sem-foco"])).is_err());
    }
}
