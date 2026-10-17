//! Executável do port: janela, entrada e desenho sobre SDL2. A lógica fica no crate `engine`; aqui só
//! se traduzem eventos do SDL em eventos do engine e se mostra o quadro composto.
//!
//! Uso:
//!   simant [--jogo DIR] [--dados DIR] [--mods DIR] [--exportar-telas DIR]
//!   simant (--save ARQ.ANT | --novo-jogo [--semente N]) [--acoes a,b,...]
//!          [--ticks N] [--botoes F,F,...] [--fechar-avisos N] [--exportar-tela-jogo ARQ.png]
//!          [--salvar ARQ.ANT]
//!   opções da janela: [--escala N] [--sair-apos-ms N [--capturar-janela ARQ.png]]
//!   som (simant/src/som): [--sem-som] [--soundfont ARQ.sf2] [--exportar-som DIR]
//!
//! Padrões: --jogo $SIMANT_DIR ou SimAnt-T/MAXIS/ANTWIN, --dados data, --mods mods (relativos à
//! pasta atual). Sem --save nem --novo-jogo, mostra a abertura. Com um deles, a simulação roda ao
//! vivo (engine::core::partida); com --ticks, --exportar-tela-jogo ou --salvar, roda sem janela e
//! sai (--salvar grava o .ANT do estado no fim). Na janela, Ctrl+S salva no arquivo da partida e
//! Ctrl+A ("Save As") num nome novo (o de --salvar, ou SALVO01.ANT, SALVO02.ANT… na pasta atual).

mod som;

mod modos;

use std::path::{Path, PathBuf};
use std::time::Instant;

use engine::core::avisos::{self, ConfigAvisos, Modal};
use engine::core::navegacao::{Acao, ConfigNavegacao, Evento as EventoNav, Repeticao};
use engine::core::partida::{self, AcaoPartida, BotaoTeclado, ConfigPartida, Partida, RespostaBotao};
use engine::formats::janela::Retangulo;
use engine::core::quintal_mapa::AcaoQuintal;
use engine::core::tela_jogo::{ConfigTelaJogo, PosicaoVista, TelaDeJogo};
use engine::core::telas::{compor, Evento, Interface, Reprodutor, Sequencia};
use engine::formats::save::{JogoSalvo, LayoutSave};
use engine::paleta::Paleta;
use engine::recursos::Recursos;
use engine::rules::amarela::{Modificadores, RespostaTecla, TabelasAmarela};
use engine::rules::estado::{ConfigSimulacao, Estado, Evento as EventoRegras};
use engine::rules::geracao::{Mundo, TabelasGeracao, TipoJogo};
use engine::rules::rng::{GeradorR, GeradorS};
use engine::rules::tick::RegrasOriginais;
use serde::Deserialize;

#[derive(Deserialize)]
struct ConfigJogo {
    bancos: Vec<String>,
    /// Id do objeto tipo 9 com as posições das janelas (docs/spec/formatos/janela.md).
    tabela_posicoes: u16,
}

#[derive(Clone)]
struct Opcoes {
    jogo: PathBuf,
    dados: PathBuf,
    mods: PathBuf,
    exportar: Option<PathBuf>,
    save: Option<PathBuf>,
    exportar_tela_jogo: Option<PathBuf>,
    /// Ações aplicadas à navegação antes de mostrar ou exportar (nomes de data/navegacao.json).
    acoes: Vec<Acao>,
    /// Fecha a janela sozinha depois desse tempo (para testes sem interação).
    sair_apos_ms: Option<u64>,
    /// Com --sair-apos-ms, grava o que a janela mostra (resolução da janela) antes de fechar.
    capturar_janela: Option<PathBuf>,
    /// Escala inteira da janela; sem ela, a maior que cabe na área útil do monitor.
    escala: Option<u32>,
    /// Quick Game gerado pelo port.
    novo_jogo: bool,
    /// Semente do Quick Game (padrão: relógio).
    semente: Option<u32>,
    /// Roda N ticks sem janela (antes de exportar).
    ticks: Option<u64>,
    /// Tipo do jogo novo (`--tipo`; padrão: Quick Game).
    tipo: TipoJogo,
    /// Abre a janela do quintal nesse modo (0–3) antes de mostrar ou exportar.
    quintal: Option<i16>,
    /// Grava o estado num .ANT (sem janela: no fim; na janela: destino do "Save As").
    salvar: Option<PathBuf>,
    /// Sem janela: fecha os N primeiros avisos modais antes de exportar.
    fechar_avisos: usize,
    /// Sem janela: funções de botões da faixa (`_DoUserButton`) apertados antes de exportar.
    botoes: Vec<u16>,
    som: som::OpcoesSom,
}

fn ler_acoes(texto: &str) -> Result<Vec<Acao>, String> {
    texto
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| serde_json::from_value(serde_json::Value::String(s.trim().into())).map_err(|_| format!("ação desconhecida: {s}")))
        .collect()
}

fn ler_opcoes() -> Result<Opcoes, String> {
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
        tipo: TipoJogo::Rapido,
        quintal: None,
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
            "--quintal" => o.quintal = Some(valor()?.parse().map_err(|_| format!("{a}: modo 0 a 3"))?),
            "--tipo" => {
                let v = valor()?;
                o.tipo = engine::core::selecao::Escolha::de_nome(&v)
                    .and_then(|e| e.tipo_jogo())
                    .ok_or(format!("{a}: use tutorial, rapido, completo ou experimental"))?;
                o.novo_jogo = true;
            }
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

fn para_rgb(indices: &[u8], cores: &[[u8; 3]]) -> Vec<u8> {
    indices.iter().flat_map(|&i| cores.get(i as usize).copied().unwrap_or([255, 0, 255])).collect()
}

fn salvar_png(caminho: &Path, largura: u32, altura: u32, rgb: &[u8]) -> Result<(), String> {
    if let Some(dir) = caminho.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let arq = std::fs::File::create(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(arq), largura, altura);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(rgb).map_err(|e| e.to_string())
}

/// Compõe a tela e avisa (uma vez por tela) o que ainda não sabemos desenhar.
fn compor_rgb(t: &engine::core::telas::Tela, recursos: &Recursos, interface: &Interface, cores: &[[u8; 3]]) -> Result<Vec<u8>, String> {
    let (quadro, pendentes) = compor(t, recursos, interface)?;
    if !pendentes.is_empty() {
        eprintln!("tela {}: não desenhado ainda: {}", t.nome, pendentes.join(", "));
    }
    Ok(para_rgb(&quadro.pixels, cores))
}

fn exportar(seq: &Sequencia, recursos: &Recursos, interface: &Interface, cores: &[[u8; 3]], dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for t in &seq.telas {
        let rgb = compor_rgb(t, recursos, interface, cores)?;
        let caminho = dir.join(format!("{}.png", t.nome));
        salvar_png(&caminho, t.largura, t.altura, &rgb)?;
        println!("{}", caminho.display());
    }
    Ok(())
}

/// Janela SDL com a resolução lógica (largura × altura) ampliada pela maior escala inteira que cabe
/// na tela (no mínimo 1), e mantida em escala inteira ao redimensionar.
struct Janela {
    sdl: sdl2::Sdl,
    canvas: sdl2::render::WindowCanvas,
}

fn abrir_janela(titulo: &str, largura: u32, altura: u32, escala: Option<u32>) -> Result<Janela, String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let escala = escala.unwrap_or_else(|| {
        video
            .display_usable_bounds(0)
            .map(|r| (r.width() / largura).min(r.height() / altura))
            .unwrap_or(2)
            .clamp(1, 4)
    });
    let janela = video
        .window(titulo, largura * escala, altura * escala)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = janela.into_canvas().present_vsync().build().map_err(|e| e.to_string())?;
    canvas.set_logical_size(largura, altura).map_err(|e| e.to_string())?;
    canvas.set_integer_scale(true)?;
    Ok(Janela { sdl, canvas })
}

/// Abertura até o "Select a Game"; devolve a escolha feita nele (None = saiu).
fn executar(
    seq: Sequencia,
    recursos: &Recursos,
    interface: &Interface,
    cores: &[[u8; 3]],
    o: &Opcoes,
) -> Result<Option<engine::core::selecao::Escolha>, String> {
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::pixels::PixelFormatEnum;

    let (lw, lh) = seq.telas.first().map(|t| (t.largura, t.altura)).unwrap_or((640, 480));
    let Janela { sdl, mut canvas } = abrir_janela("SimAnt", lw, lh, o.escala)?;
    let criador = canvas.texture_creator();
    let mut eventos = sdl.event_pump()?;
    let selecao = modos::selecao_da_abertura(recursos, interface)?;
    let mut audio = som::Audio::abrir(&sdl, recursos, &o.som);
    audio.musica_abertura();

    let mut rep = Reprodutor::new(seq);
    let mut desenhada: Option<String> = None;
    let mut textura = None;
    let inicio = Instant::now();
    let mut antes = inicio;

    while !rep.terminou() {
        let escolhendo = rep.atual().is_some_and(|t| t.nome == selecao.cfg.tela);
        for ev in eventos.poll_iter() {
            match ev {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return Ok(None),
                Event::KeyDown { keycode: Some(k), keymod, .. } if escolhendo => {
                    if let Some(e) = selecao.cfg.escolha_da_tecla(&evento_de_tecla(k, keymod)) {
                        return Ok(Some(e));
                    }
                }
                Event::MouseButtonDown { x, y, .. } if escolhendo => {
                    if let Some(e) = selecao.escolha_em(x, y) {
                        return Ok(Some(e));
                    }
                }
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => rep.evento(Evento::Espaco),
                Event::KeyDown { .. } => rep.evento(Evento::OutraTecla),
                Event::MouseButtonDown { .. } => rep.evento(Evento::Clique),
                _ => {}
            }
        }
        let agora = Instant::now();
        rep.passar_tempo((agora - antes).as_millis() as u64);
        antes = agora;
        if o.sair_apos_ms.is_some_and(|ms| (agora - inicio).as_millis() as u64 >= ms) {
            return Ok(None);
        }

        let Some(tela) = rep.atual() else { break };
        if desenhada.as_deref() != Some(tela.nome.as_str()) {
            let rgb = compor_rgb(tela, recursos, interface, cores)?;
            let mut t = criador
                .create_texture_static(PixelFormatEnum::RGB24, tela.largura, tela.altura)
                .map_err(|e| e.to_string())?;
            t.update(None, &rgb, tela.largura as usize * 3).map_err(|e| e.to_string())?;
            canvas.set_logical_size(tela.largura, tela.altura).map_err(|e| e.to_string())?;
            textura = Some(t);
            desenhada = Some(tela.nome.clone());
        }
        canvas.clear();
        if let Some(t) = &textura {
            canvas.copy(t, None, None)?;
        }
        canvas.present();
    }
    Ok(None)
}

/// De onde vem a partida.
enum Origem<'o> {
    Save(&'o Path),
    /// Jogo novo gerado pelo port com a semente dada.
    Novo(u32, TipoJogo),
}

/// Tudo o que a tela de jogo ao vivo precisa: a tela carregada (layouts e tilesets), a configuração
/// de navegação e a partida (estado, regras, relógio e vista).
struct Sessao<'a> {
    recursos: &'a Recursos,
    interface: Interface,
    tela: TelaDeJogo,
    nav_cfg: ConfigNavegacao,
    partida: Partida,
    cores: Vec<[u8; 3]>,
    /// Janela do quintal e "Select a Game" durante a partida (simant/src/modos.rs).
    modos: modos::Modos,
    layout: LayoutSave,
    /// Arquivo da partida (o do --save ou o último salvo), alvo do File → Save.
    arquivo: Option<PathBuf>,
    /// Retângulo que fecha o aviso modal da frente, como desenhado no último quadro.
    clique_modal: Option<Retangulo>,
}

impl Sessao<'_> {
    fn abrir<'a>(recursos: &'a Recursos, dados: &Path, origem: Origem, cores: Vec<[u8; 3]>) -> Result<Sessao<'a>, String> {
        let cfg: ConfigTelaJogo = recursos.dados("tela_jogo.json")?;
        let nav_cfg: ConfigNavegacao = recursos.dados("navegacao.json")?;
        let cfg_sim = ConfigSimulacao::carregar(recursos)?;
        let cfg_partida: ConfigPartida = recursos.dados(partida::ARQUIVO_CONFIG)?;
        let interface = Interface::carregar(recursos, cfg.tabela_posicoes)?;
        let layout = LayoutSave::carregar(&dados.join("save_layout.json"))?;
        let geracao = TabelasGeracao::carregar(recursos)?;
        let (jogo, estado) = match origem {
            Origem::Save(save) => {
                let jogo = JogoSalvo::abrir(&layout, save)?;
                let estado = Estado::de_save(&jogo)?;
                (jogo, estado)
            }
            Origem::Novo(semente, tipo) => {
                // _initStuff: _SeedSRand e _SeedRRand com o relógio; o port usa a semente nas duas
                // leituras do relógio. Depois _NewGame → _RandYard (geracao-mundo.md).
                let mut s = GeradorS::new(0);
                let mut r = GeradorR::default();
                r.semear_com_ticks(&mut s, semente, semente);
                let m = Mundo::novo_jogo(tipo, &mut r, &mut s, &geracao)?;
                let mut estado = Estado::de_mundo(&m, s, r, &cfg_sim)?;
                // _RandYard passo 3: _InitSimYard (menino, cachorro...), que Estado::de_mundo não faz.
                let quintal = RegrasOriginais::carregar_sem_banco(recursos)?.quintal;
                engine::rules::quintal::iniciar_quintal(&mut estado, &quintal);
                (estado.para_save(&layout)?, estado)
            }
        };
        let mut tela = TelaDeJogo::carregar(&cfg, recursos, &interface, &jogo)?;
        tela.atualizar(&estado);
        // Jogo novo: _NewGame faz _SetMapPlane(_MePlane) e _CenterEdit na amarela, o mesmo que a
        // posição "ao carregar"; um save segue data/navegacao.json.
        let posicao = if matches!(origem, Origem::Novo(..)) { PosicaoVista::AoCarregar } else { nav_cfg.posicao_inicial };
        let nav = tela.navegacao(&jogo, posicao)?;
        // Tabelas de todas as partes com mods (data/*.json); o seno da geração já foi lido do banco.
        let regras = RegrasOriginais { geracao: Some(geracao), ..RegrasOriginais::carregar_sem_banco(recursos)? };
        let cfg_avisos: ConfigAvisos = recursos.dados(avisos::ARQUIVO_CONFIG)?;
        let partida =
            Partida::nova(estado, nav, regras, TabelasAmarela::carregar(recursos)?, cfg_sim, cfg_partida, cfg_avisos);
        let arquivo = match origem {
            Origem::Save(p) => Some(p.to_path_buf()),
            Origem::Novo(..) => None,
        };
        let modos = modos::Modos::carregar(recursos, &interface)?;
        Ok(Sessao { recursos, interface, tela, nav_cfg, partida, cores, layout, arquivo, clique_modal: None, modos })
    }

    /// File → Save (`_SaveGame(1)`): grava no arquivo da partida; sem arquivo, ou em "Save As"
    /// (`_SaveGame(0)`), num nome novo. O original pergunta o nome num diálogo comum do Windows.
    fn salvar(&mut self, como: bool, destino_como: Option<&Path>) -> Result<PathBuf, String> {
        let caminho = match (&self.arquivo, como, destino_como) {
            (Some(a), false, _) => a.clone(),
            (_, _, Some(d)) => d.to_path_buf(),
            _ => (1..1000)
                .map(|n| PathBuf::from(format!("SALVO{n:02}.ANT")))
                .find(|p| !p.exists())
                .ok_or("nenhum nome livre para salvar")?,
        };
        self.partida.estado.para_save(&self.layout)?.gravar(&caminho)?;
        self.arquivo = Some(caminho.clone());
        Ok(caminho)
    }

    fn quadro_rgb(&mut self) -> Result<Vec<u8>, String> {
        self.tela.atualizar(&self.partida.estado);
        let (mut q, mut pendentes) = self.tela.compor(self.recursos, &self.interface, &self.partida.nav)?;
        let agora = self.partida.agora();
        let (clique, p) = self.tela.desenhar_avisos(&mut q, self.recursos, &self.interface, &mut self.partida.avisos, agora)?;
        self.clique_modal = clique;
        pendentes.extend(p);
        pendentes.extend(modos::desenhar(self, &mut q)?);
        if !pendentes.is_empty() {
            eprintln!("tela de jogo: não desenhado ainda: {}", pendentes.join(", "));
        }
        Ok(para_rgb(&q.pixels, &self.cores))
    }

    /// Botão configurável da faixa sob (x, y): a função dele (`_DoUserButton`).
    fn botao_usuario_em(&self, x: i32, y: i32) -> Option<u16> {
        self.tela.objetos_da_faixa_em(x, y).iter().find_map(|n| self.partida.cfg.funcao_do_botao(n))
    }

    /// Ação do clique esquerdo em (x, y) da área cliente: seta da barra de rolagem ou botão da faixa.
    fn acao_do_clique(&self, x: i32, y: i32) -> Option<Acao> {
        self.tela
            .seta_em(x, y)
            .or_else(|| self.tela.objetos_da_faixa_em(x, y).iter().find_map(|n| self.nav_cfg.botoes_faixa.get(*n).copied()))
    }

    /// Clique (ou tecla que imita o mouse) em (x, y): se cai numa célula do mapa, vira ordem para a
    /// formiga amarela.
    fn clicar_no_mapa(&mut self, x: i32, y: i32, duplo: bool) {
        if let Some((cx, cy)) = self.tela.celula_em(x, y, &self.partida.nav) {
            let r = self.partida.clicar(cx, cy, duplo);
            println!("clique {} em ({cx}, {cy}), plano {}: {r:?}", if duplo { "duplo" } else { "simples" }, self.partida.nav.plano_mapa);
        }
    }

    /// Escreve os pedidos das regras à interface (mensagens, músicas, fim de jogo) e os devolve
    /// (o som é tocado por `som::Audio`); por enquanto sem texto nem diálogo.
    fn registrar_eventos(&mut self) -> Vec<EventoRegras> {
        let eventos = self.partida.retirar_eventos();
        for ev in &eventos {
            let t = self.partida.estado.partida.tempo;
            match *ev {
                EventoRegras::Mensagem(id) => {
                    let texto: Vec<String> =
                        avisos::linhas_do_texto(self.recursos, id).iter().map(|l| l.iter().map(|&b| b as char).collect()).collect();
                    println!("[tick {t}] mensagem {id}: {}", texto.join(" / "));
                }
                EventoRegras::Musica(id) => println!("[tick {t}] música {id}"),
                EventoRegras::Som(_) | EventoRegras::TemaDoNinho => {} // tocados por som::Audio
                EventoRegras::DialogoFimDeJogo => println!("[tick {t}] fim de jogo (_EndGameDialog)"),
                EventoRegras::CentralizarNaAmarela => {}
            }
        }
        eventos
    }

    /// Velocidade e pausa, para o título da janela.
    fn situacao(&self) -> String {
        let p = &self.partida.estado.partida;
        let vel = ["lenta", "normal", "rápida", "ultra"].get(p.velocidade as usize).copied().unwrap_or("?");
        let pausa = if p.pausado != 0 { ", pausado" } else { "" };
        let track = if p.opcoes[0] != 0 { ", AutoTrack" } else { "" };
        format!("{vel}{pausa}{track}")
    }
}

/// Tecla do SDL → evento do engine (nome da tecla no SDL e modificadores, esquerdo ou direito).
fn evento_de_tecla(k: sdl2::keyboard::Keycode, m: sdl2::keyboard::Mod) -> EventoNav {
    use sdl2::keyboard::Mod;
    EventoNav::Tecla {
        nome: k.name(),
        ctrl: m.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
        shift: m.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
        alt: m.intersects(Mod::LALTMOD | Mod::RALTMOD),
    }
}

/// Uma tecla na tela de jogo, na ordem do original: aceleradores (Speed/Options e navegação),
/// depois `_YellowCommandKey`, depois as teclas que imitam o mouse. Devolve o botão imitado, se for
/// o caso.
fn tratar_tecla(s: &mut Sessao, ev: &EventoNav, tab: bool, o: &Opcoes) -> Option<BotaoTeclado> {
    if let Some(a) = s.partida.cfg.acao_da_tecla(ev) {
        if let AcaoPartida::Salvar | AcaoPartida::SalvarComo = a {
            salvar_e_avisar(s, a == AcaoPartida::SalvarComo, o);
            return None;
        }
        s.partida.aplicar(a);
        println!("{a:?}: {}", s.situacao());
        return None;
    }
    if let Some(a) = s.nav_cfg.acao_da_tecla(ev) {
        s.partida.navegar(a);
        return None;
    }
    let EventoNav::Tecla { nome, ctrl, shift, alt } = ev;
    if *alt {
        return None;
    }
    if let Some(vk) = s.partida.cfg.tecla_virtual(nome) {
        let r = s.partida.tecla_amarela(vk, Modificadores { shift: *shift, ctrl: *ctrl, tab });
        if r != RespostaTecla::Ignorada {
            println!("tecla {nome} (VK 0x{vk:02X}): {r:?}");
            return None;
        }
    }
    s.partida.cfg.botao_da_tecla(nome)
}

/// Salva e escreve o resultado ("%s Saved correctly", `_PopMsg`, no original uma caixa de mensagem).
fn salvar_e_avisar(s: &mut Sessao, como: bool, o: &Opcoes) {
    match s.salvar(como, o.salvar.as_deref()) {
        Ok(p) => println!("{} Saved correctly", p.display()),
        Err(e) => eprintln!("erro ao salvar: {e}"),
    }
}

/// Entrada com um aviso modal na tela (`_PictureDialog`, `_EndGameDialog`): só o clique no
/// retângulo do aviso e (no quadro) Espaço o fecham. Devolve o aviso fechado.
fn tratar_modal(s: &mut Sessao, ev: &sdl2::event::Event) -> Option<Modal> {
    use sdl2::event::Event;
    use sdl2::mouse::MouseButton;
    let r = s.clique_modal?;
    let tecla = s.partida.avisos.cfg.quadro.tecla_continuar.clone();
    let quadro = matches!(s.partida.avisos.modal(), Some(Modal::Quadro { .. }));
    let fecha = match ev {
        Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => {
            let dentro = *x >= r.esq as i32 && *x < r.dir as i32 && *y >= r.topo as i32 && *y < r.base as i32;
            if !dentro {
                // O original toca MessageBeep.
                println!("(bip)");
            }
            dentro
        }
        Event::KeyDown { keycode: Some(k), .. } => quadro && k.name().eq_ignore_ascii_case(&tecla),
        _ => false,
    };
    if fecha {
        s.partida.fechar_modal()
    } else {
        None
    }
}

fn executar_partida(mut s: Sessao, titulo: &str, o: &Opcoes, jogo_novo: bool) -> Result<(), String> {
    use sdl2::event::{Event, WindowEvent};
    use sdl2::keyboard::{Keycode, Scancode};
    use sdl2::mouse::MouseButton;
    use sdl2::pixels::PixelFormatEnum;

    let (lw, lh) = (s.tela.cfg.largura, s.tela.cfg.altura);
    let Janela { sdl, mut canvas } = abrir_janela(titulo, lw, lh, o.escala)?;
    let criador = canvas.texture_creator();
    let mut textura = criador.create_texture_static(PixelFormatEnum::RGB24, lw, lh).map_err(|e| e.to_string())?;
    let mut eventos = sdl.event_pump()?;
    let mut audio = som::Audio::abrir(&sdl, s.recursos, &o.som);
    if jogo_novo {
        audio.musica_jogo_novo();
    }
    let mut repeticao = Repeticao::default();
    let mut redesenhar = true;
    // Posição do cursor em coordenadas lógicas (para Espaço/Insert/Delete) e a última tecla-clique.
    let mut cursor = (0, 0);
    let mut ultima_tecla_clique: Option<(BotaoTeclado, Instant)> = None;
    let mut situacao = String::new();
    let mut faixa = None;
    let inicio = Instant::now();
    let mut antes = inicio;
    println!("partida: {} (Shift+0 pausa, Shift+1..4 velocidade, Ctrl+G AutoTrack)", s.situacao());

    loop {
        let lote: Vec<Event> = eventos.poll_iter().collect();
        for ev in lote {
            if s.partida.avisos.tem_modal() && !matches!(ev, Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } | Event::Window { .. }) {
                if let Event::MouseMotion { x, y, .. } = ev {
                    cursor = (x, y);
                }
                if let Some(Modal::FimDeJogo { .. }) = tratar_modal(&mut s, &ev) {
                    // _EndGameDialog → _NewGame(0): abre o "Select a Game" (simant/src/modos.rs).
                    println!("fim de jogo: Select a Game");
                    s.modos.escolhendo = true;
                }
                redesenhar = true;
                continue;
            }
            match ev {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return Ok(()),
                Event::Window { win_event: WindowEvent::FocusGained, .. } => {
                    s.partida.definir_foco(true);
                    audio.foco(true);
                }
                Event::Window { win_event: WindowEvent::FocusLost, .. } => {
                    s.partida.definir_foco(false);
                    audio.foco(false);
                }
                Event::Window { win_event: WindowEvent::Exposed, .. } => redesenhar = true,
                Event::MouseMotion { x, y, mousestate, .. } => {
                    cursor = (x, y);
                    if mousestate.left() && modos::mover(&mut s, x, y, ctrl_apertado(&eventos)) {
                        redesenhar = true;
                    }
                }
                Event::KeyDown { keycode: Some(k), keymod, .. } => {
                    let tab = eventos.keyboard_state().is_scancode_pressed(Scancode::Tab);
                    let ev = evento_de_tecla(k, keymod);
                    match modos::tecla(&mut s, &ev) {
                        modos::Resposta::Seguir => {}
                        modos::Resposta::Tratada => {
                            redesenhar = true;
                            continue;
                        }
                        modos::Resposta::NovoJogo(e) => {
                            s = novo_jogo(&s, e, o)?;
                            audio.musica_jogo_novo();
                            redesenhar = true;
                            continue;
                        }
                    }
                    if let Some(b) = tratar_tecla(&mut s, &ev, tab, o) {
                        let agora = Instant::now();
                        let duplo = ultima_tecla_clique.is_some_and(|(ub, t)| {
                            ub == b && (agora - t).as_millis() as u64 <= s.partida.cfg.duplo_clique_teclado_ms
                        });
                        ultima_tecla_clique = if duplo { None } else { Some((b, agora)) };
                        s.clicar_no_mapa(cursor.0, cursor.1, duplo);
                    }
                }
                Event::MouseButtonDown { mouse_btn: MouseButton::Left, clicks, x, y, .. } => {
                    cursor = (x, y);
                    match modos::clique(&mut s, x, y, clicks >= 2, ctrl_apertado(&eventos)) {
                        modos::Resposta::Seguir => {}
                        modos::Resposta::Tratada => {
                            redesenhar = true;
                            continue;
                        }
                        modos::Resposta::NovoJogo(e) => {
                            s = novo_jogo(&s, e, o)?;
                            audio.musica_jogo_novo();
                            redesenhar = true;
                            continue;
                        }
                    }
                    if let Some(f) = s.botao_usuario_em(x, y) {
                        match s.partida.botao_usuario(f) {
                            RespostaBotao::Feito => println!("botão {f}: {}", s.situacao()),
                            RespostaBotao::Salvar => salvar_e_avisar(&mut s, false, o),
                            RespostaBotao::NaoImplementado(n) => println!("botão {n}: janela ainda não implementada"),
                        }
                    } else if let Some(a) = s.acao_do_clique(x, y) {
                        s.partida.navegar(a);
                        repeticao.pressionar(a);
                    } else {
                        s.clicar_no_mapa(x, y, clicks >= 2);
                    }
                }
                Event::MouseButtonDown { mouse_btn: MouseButton::Right, clicks, x, y, .. } => {
                    // Experimental: _processExp recebe também o clique direito.
                    if let modos::Resposta::Seguir = modos::clique(&mut s, x, y, clicks >= 2, ctrl_apertado(&eventos)) {
                        // _processEdit: clique direito simples = simples; duplo direito = duplo.
                        s.clicar_no_mapa(x, y, clicks >= 2);
                    }
                    redesenhar = true;
                }
                Event::MouseButtonUp { mouse_btn: MouseButton::Left, .. } => {
                    repeticao.soltar();
                    modos::soltar(&mut s);
                }
                Event::MouseButtonUp { mouse_btn: MouseButton::Right, .. } => modos::soltar(&mut s),
                _ => {}
            }
        }
        let agora = Instant::now();
        let ms = (agora - antes).as_millis() as u64;
        antes = agora;
        if let Some((a, n)) = repeticao.passar_tempo(ms, &s.nav_cfg.repeticao) {
            for _ in 0..n {
                s.partida.navegar(a);
            }
        }
        // Com o "Select a Game" aberto (Ctrl+N), a simulação fica parada (_SetPause(1) em _NewGame).
        if !s.modos.escolhendo {
            s.partida.avancar(ms);
        }
        let pedidos = s.registrar_eventos();
        audio.tratar(&pedidos, &s.partida.estado.partida.opcoes);
        redesenhar |= s.partida.tomar_redesenho();
        // _DrawRibbonMessage: a mensagem vencida some da faixa.
        let agora_faixa = s.partida.agora();
        let nova_faixa = s.partida.avisos.texto_da_faixa(agora_faixa).cloned();
        if nova_faixa != faixa {
            faixa = nova_faixa;
            redesenhar = true;
        }
        if redesenhar {
            let rgb = s.quadro_rgb()?;
            textura.update(None, &rgb, lw as usize * 3).map_err(|e| e.to_string())?;
            redesenhar = false;
        }
        let nova = s.situacao();
        if nova != situacao {
            canvas.window_mut().set_title(&format!("{titulo} [{nova}]")).map_err(|e| e.to_string())?;
            situacao = nova;
        }
        canvas.clear();
        canvas.copy(&textura, None, None)?;
        if o.sair_apos_ms.is_some_and(|ms| (agora - inicio).as_millis() as u64 >= ms) {
            // Lê o quadro antes do present (depois dele o conteúdo do buffer é indefinido). Sem
            // redimensionar, a janela tem exatamente a resolução lógica × escala.
            if let Some(c) = &o.capturar_janela {
                let (w, h) = canvas.output_size()?;
                let px = canvas.read_pixels(None, PixelFormatEnum::RGB24)?;
                salvar_png(c, w, h, &px)?;
                println!("{} ({w} × {h})", c.display());
            }
            println!("saindo: GameTime = {}, {}", s.partida.estado.partida.tempo, s.situacao());
            return Ok(());
        }
        canvas.present();
    }
}

/// Ctrl apertado agora (`GetAsyncKeyState(VK_CONTROL)` das ferramentas do Experimental).
fn ctrl_apertado(eventos: &sdl2::EventPump) -> bool {
    use sdl2::keyboard::Scancode;
    let k = eventos.keyboard_state();
    k.is_scancode_pressed(Scancode::LCtrl) || k.is_scancode_pressed(Scancode::RCtrl)
}

/// Jogo novo escolhido no "Select a Game" do Ctrl+N: nova sessão com a semente do relógio.
fn novo_jogo<'a>(s: &Sessao<'a>, e: engine::core::selecao::Escolha, o: &Opcoes) -> Result<Sessao<'a>, String> {
    let tipo = e.tipo_jogo().ok_or("escolha sem jogo novo")?;
    let n = semente(&Opcoes { semente: None, ..o.clone() });
    println!("novo jogo {e:?} com semente {n}");
    Sessao::abrir(s.recursos, &o.dados, Origem::Novo(n, tipo), s.cores.clone())
}

/// Semente do Quick Game: a da linha de comando ou o relógio (milissegundos desde 1970, como o
/// `_TickCount` do original só importa a palavra baixa).
fn semente(o: &Opcoes) -> u32 {
    o.semente.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u32)
            .unwrap_or(0)
    })
}

fn principal() -> Result<(), String> {
    let o = ler_opcoes()?;
    let config: ConfigJogo = serde_json::from_str(
        &std::fs::read_to_string(o.dados.join("jogo.json")).map_err(|e| format!("data/jogo.json: {e}"))?,
    )
    .map_err(|e| format!("data/jogo.json: {e}"))?;
    let recursos = Recursos::abrir(&o.jogo, &config.bancos, &o.dados, Some(&o.mods))?;
    for m in recursos.mods() {
        println!("mod carregado: {} {}", m.manifesto.nome, m.manifesto.versao);
    }
    let cores = recursos.dados::<Paleta>("paleta.json")?.rgb()?;
    if let Some(dir) = &o.som.exportar {
        return som::exportar(&recursos, &o.som, dir);
    }

    let mut o = o;
    if !o.novo_jogo && o.save.is_none() && o.exportar.is_none() {
        // Abertura até o "Select a Game"; a escolha vira o jogo novo.
        let seq: Sequencia = recursos.dados("telas.json")?;
        let interface = Interface::carregar(&recursos, config.tabela_posicoes)?;
        match executar(seq, &recursos, &interface, &cores, &o)? {
            Some(e) => match e.tipo_jogo() {
                Some(tipo) => {
                    println!("Select a Game: {e:?}");
                    o.novo_jogo = true;
                    o.tipo = tipo;
                }
                None => {
                    println!("Select a Game: {e:?} (sem jogo novo)");
                    return Ok(());
                }
            },
            None => return Ok(()),
        }
    }
    let origem = match (&o.save, o.novo_jogo) {
        (Some(save), _) => Some(Origem::Save(save)),
        (None, true) => {
            let n = semente(&o);
            println!("{:?} com semente {n}", o.tipo);
            Some(Origem::Novo(n, o.tipo))
        }
        _ => None,
    };
    if let Some(origem) = origem {
        let nome = match &origem {
            Origem::Save(save) => save.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            Origem::Novo(_, tipo) => format!("{tipo:?}"),
        };
        let mut s = Sessao::abrir(&recursos, &o.dados, origem, cores)?;
        for &a in &o.acoes {
            s.partida.navegar(a);
        }
        if let Some(m) = o.quintal {
            let a = [AcaoQuintal::ModoQuintal, AcaoQuintal::ModoCasa, AcaoQuintal::ModoEstrategico, AcaoQuintal::ModoGrafico];
            let a = *a.get(m as usize).ok_or("--quintal: modo 0 a 3")?;
            s.modos.janela.aplicar(a, &mut s.partida.estado);
        }
        if let Some(n) = o.ticks {
            let antes = s.partida.estado.partida.tempo;
            for _ in 0..n {
                s.partida.rodar_tick();
            }
            s.registrar_eventos();
            println!("{n} ticks: GameTime {antes} -> {}", s.partida.estado.partida.tempo);
        }
        for &b in &o.botoes {
            println!("botão {b}: {:?}", s.partida.botao_usuario(b));
        }
        for _ in 0..o.fechar_avisos {
            s.partida.fechar_modal();
        }
        if let Some(Modal::FimDeJogo { pontos, .. }) = s.partida.avisos.modal() {
            println!("fim de jogo: {pontos} pontos");
        }
        if let Some(png) = &o.exportar_tela_jogo {
            let rgb = s.quadro_rgb()?;
            salvar_png(png, s.tela.cfg.largura, s.tela.cfg.altura, &rgb)?;
            let (p, v) = (&s.partida.nav, &s.partida.nav.vista);
            println!("{} (plano {}, vista em ({}, {}), {} × {} tiles)", png.display(), p.plano_mapa, v.x, v.y, v.largura, v.altura);
        }
        if let Some(arq) = &o.salvar {
            s.partida.estado.para_save(&s.layout)?.gravar(arq)?;
            println!("{} Saved correctly", arq.display());
        }
        if o.ticks.is_some() || o.exportar_tela_jogo.is_some() || o.salvar.is_some() {
            return Ok(());
        }
        let jogo_novo = o.novo_jogo;
        return executar_partida(s, &format!("SimAnt - {nome}"), &o, jogo_novo);
    }

    let seq: Sequencia = recursos.dados("telas.json")?;
    let interface = Interface::carregar(&recursos, config.tabela_posicoes)?;
    match &o.exportar {
        Some(dir) => exportar(&seq, &recursos, &interface, &cores, dir),
        None => Ok(()),
    }
}

fn main() {
    if let Err(e) = principal() {
        eprintln!("erro: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use sdl2::keyboard::{Keycode, Mod};

    #[test]
    fn teclas_do_sdl_viram_acoes_de_data_navegacao() {
        let caminho = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/navegacao.json");
        let cfg: ConfigNavegacao = serde_json::from_str(&std::fs::read_to_string(caminho).unwrap()).unwrap();
        let acao = |k, m| cfg.acao_da_tecla(&evento_de_tecla(k, m));
        // Aceleradores do original (Ctrl + seta, Ctrl + 8/9/0), com Ctrl esquerdo ou direito.
        assert_eq!(acao(Keycode::Right, Mod::LCTRLMOD), Some(Acao::LinhaDireita));
        assert_eq!(acao(Keycode::Up, Mod::RCTRLMOD | Mod::NUMMOD), Some(Acao::LinhaAcima), "NumLock não conta");
        assert_eq!(acao(Keycode::Num8, Mod::LCTRLMOD), Some(Acao::Superficie));
        assert_eq!(acao(Keycode::Num9, Mod::RCTRLMOD), Some(Acao::NinhoPreto));
        assert_eq!(acao(Keycode::Num0, Mod::LCTRLMOD), Some(Acao::NinhoVermelho));
        // Extensões do port: setas e PageUp/PageDown sem modificador.
        assert_eq!(acao(Keycode::Left, Mod::NOMOD), Some(Acao::LinhaEsquerda));
        assert_eq!(acao(Keycode::PageDown, Mod::NOMOD), Some(Acao::PaginaAbaixo));
        assert_eq!(acao(Keycode::Num8, Mod::NOMOD), None);
        assert_eq!(acao(Keycode::Down, Mod::LSHIFTMOD), None);
    }

    #[test]
    fn teclas_do_sdl_viram_aceleradores_e_comandos_de_data_partida() {
        use engine::core::partida::AcaoPartida;
        let caminho = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/partida.json");
        let cfg = ConfigPartida::de_json(&std::fs::read_to_string(caminho).unwrap()).unwrap();
        let acao = |k, m| cfg.acao_da_tecla(&evento_de_tecla(k, m));
        assert_eq!(acao(Keycode::Num0, Mod::LSHIFTMOD), Some(AcaoPartida::Pausa));
        assert_eq!(acao(Keycode::Num4, Mod::RSHIFTMOD), Some(AcaoPartida::VelocidadeUltra));
        assert_eq!(acao(Keycode::Kp2, Mod::LSHIFTMOD), Some(AcaoPartida::VelocidadeNormal));
        assert_eq!(acao(Keycode::G, Mod::LCTRLMOD), Some(AcaoPartida::AutoTrack));
        assert_eq!(acao(Keycode::Num0, Mod::NOMOD), None);
        assert_eq!(cfg.tecla_virtual(&Keycode::Num1.name()), Some(0x31));
        assert_eq!(cfg.tecla_virtual(&Keycode::KpMultiply.name()), Some(0x6A));
        assert_eq!(cfg.tecla_virtual(&Keycode::Backspace.name()), Some(0x08));
        assert_eq!(cfg.tecla_virtual(&Keycode::F9.name()), Some(0x78));
        assert_eq!(cfg.botao_da_tecla(&Keycode::Space.name()), Some(BotaoTeclado::Esquerdo));
        assert_eq!(cfg.botao_da_tecla(&Keycode::Insert.name()), Some(BotaoTeclado::Esquerdo));
        assert_eq!(cfg.botao_da_tecla(&Keycode::Delete.name()), Some(BotaoTeclado::Direito));
    }

    #[test]
    fn lista_de_acoes_da_linha_de_comando() {
        assert_eq!(ler_acoes("superficie, linha_direita").unwrap(), vec![Acao::Superficie, Acao::LinhaDireita]);
        assert!(ler_acoes("voar").is_err());
    }
}
