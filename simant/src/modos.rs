//! Modos de jogo na interface SDL: escolha no "Select a Game" (na abertura e com File → New Game,
//! Ctrl+N, durante a partida), janela do quintal (Yard View, faixa 35) com a escolha e a troca de
//! patch. A lógica fica no engine (`core::selecao`, `core::quintal_mapa`, `rules::troca_patch`);
//! aqui só se liga a entrada do SDL e o desenho por cima da tela de jogo.
//!
//! Especificação: docs/spec/telas/abertura.md ("Escolha do jogo") e docs/spec/telas/quintal.md.

use engine::core::menu_ferramentas::{ItemMenu, MenuFerramentas};
use engine::core::navegacao::Evento as EventoNav;
use engine::core::popup_castas::{CliqueBarras, PopupCastas};
use engine::core::quadro::Quadro;
use engine::core::janelas::Controle;
use engine::core::quintal_mapa::{self, AcaoQuintal, ConfigQuintalMapa, DesenhoQuintal, JanelaQuintal};
use engine::core::selecao::{ConfigModos, Escolha, SelecaoDeJogo};
use engine::core::telas::{desenhar_janela_na_tela, Interface};
use engine::recursos::Recursos;
use engine::rules::estado::Evento;
use engine::rules::experimental::{self, Contexto, ControleExperimental, Efeito, Ferramenta, TabelasExperimental};

use super::Sessao;

/// Estado dos modos na tela de jogo: janela do quintal e "Select a Game" aberto pelo Ctrl+N.
pub struct Modos {
    pub quintal: DesenhoQuintal,
    pub janela: JanelaQuintal,
    pub selecao: SelecaoDeJogo,
    /// O "Select a Game" está aberto sobre a partida (a simulação fica pausada).
    pub escolhendo: bool,
    /// Menu de ferramentas do Experimental (botão ribTools).
    pub menu: MenuFerramentas,
    pub menu_aberto: bool,
    pub tabelas_exp: TabelasExperimental,
    /// `_ExpSubStates` e o arrasto de `_processExp`.
    pub controle_exp: ControleExperimental,
    /// Área das barras da faixa: limiares de aviso e o quadro das castas (`_DrawCastePopUp`), aberto
    /// enquanto o botão fica apertado.
    pub popup_castas: PopupCastas,
    pub castas_aberto: bool,
}

impl Modos {
    pub fn carregar(recursos: &Recursos, interface: &Interface) -> Result<Modos, String> {
        let cfg = ConfigQuintalMapa::carregar(recursos)?;
        let quintal = DesenhoQuintal::carregar(cfg, recursos, interface)?;
        let selecao = SelecaoDeJogo::carregar(recursos.dados::<ConfigModos>(engine::core::selecao::ARQUIVO_CONFIG)?, recursos, interface)?;
        let tabelas_exp = TabelasExperimental::carregar(recursos)?;
        let controle_exp = ControleExperimental::novo(&tabelas_exp);
        let menu = MenuFerramentas::carregar(recursos)?;
        let popup_castas = PopupCastas::carregar(recursos)?;
        Ok(Modos {
            quintal,
            janela: JanelaQuintal::default(),
            selecao,
            escolhendo: false,
            menu,
            menu_aberto: false,
            tabelas_exp,
            controle_exp,
            popup_castas,
            castas_aberto: false,
        })
    }
}

/// "Select a Game" da abertura (com a interface da abertura).
pub fn selecao_da_abertura(recursos: &Recursos, interface: &Interface) -> Result<SelecaoDeJogo, String> {
    SelecaoDeJogo::carregar(recursos.dados::<ConfigModos>(engine::core::selecao::ARQUIVO_CONFIG)?, recursos, interface)
}

/// A faixa do momento: com o quintal ativado por último, a 35 no lugar da 34; senão, na 34, o botão
/// de ferramentas do Experimental. Devolve o que não foi desenhado.
pub fn desenhar_faixa(s: &Sessao, q: &mut Quadro) -> Vec<String> {
    let m = &s.modos;
    if m.janela.na_frente {
        return m.quintal.desenhar_faixa(q, s.recursos, &s.interface, &s.partida.estado, &s.partida.estado_dos_botoes());
    }
    // _NewGame: no Experimental o botão de ferramentas da faixa usa outros bitmaps.
    if s.partida.estado.partida.tipo_jogo == 3 {
        if let Some(r) = s.tela.retangulo_da_faixa(&m.menu.cfg.botao) {
            let id = m.menu.cfg.bitmaps_botao_experimental[1];
            if let Ok(b) = s.recursos.objeto(engine::formats::banco::Chave { tipo: engine::formats::bitmap::TIPO_BITMAP, id }).and_then(|d| engine::formats::bitmap::Bitmap::decodificar(&d)) {
                q.bitmap(&b, i32::from(r.esq), i32::from(r.topo));
            }
        }
    }
    Vec::new()
}

/// Desenha, por cima de tudo, o menu de ferramentas do Experimental e o "Select a Game" do Ctrl+N.
/// (O quintal é uma janela do gerenciador: `Sessao::desenhar_janelas`.) Devolve o que não foi
/// desenhado.
pub fn desenhar(s: &Sessao, q: &mut Quadro) -> Result<Vec<String>, String> {
    let m = &s.modos;
    let mut pendentes = Vec::new();
    if m.menu_aberto {
        pendentes.extend(m.menu.desenhar(q, s.recursos, &s.interface));
    }
    if m.escolhendo {
        pendentes.extend(desenhar_janela_na_tela(q, s.recursos, &s.interface, m.selecao.cfg.janela)?);
    }
    if m.castas_aberto {
        pendentes.extend(m.popup_castas.desenhar(q, s.recursos, &s.interface, &s.partida.estado, s.tela.cfg.largura as i32));
    }
    Ok(pendentes)
}

/// O que fazer depois de uma entrada tratada aqui.
pub enum Resposta {
    /// Não era para os modos: segue o tratamento normal da tela de jogo.
    Seguir,
    /// Tratada.
    Tratada,
    /// O jogador escolheu um jogo novo no "Select a Game" do Ctrl+N.
    NovoJogo(Escolha),
}

/// Tecla na tela de jogo: Ctrl+N (File → New Game), teclas do "Select a Game" aberto e atalhos do
/// quintal (Ctrl+Y/5/6/7, Ctrl+M).
pub fn tecla(s: &mut Sessao, ev: &EventoNav) -> Resposta {
    let m = &mut s.modos;
    if m.escolhendo {
        return match m.selecao.cfg.escolha_da_tecla(ev) {
            Some(e) => escolher(s, e),
            None => Resposta::Tratada,
        };
    }
    if m.selecao.cfg.novo_jogo.bate(ev) {
        // Ctrl+N = File → New (_ProcMenu 3): a música e o "Select a Game".
        super::menus_ui::executar(s, engine::core::barra_menus::ComandoMenu::Novo, None);
        return Resposta::Tratada;
    }
    if let Some(a) = m.quintal.cfg.acao_da_tecla(ev) {
        if a == AcaoQuintal::Abrir && !s.janelas.aberta(Controle::Quintal) {
            // _OpenMapYard: sem o quintal aberto, Ctrl+M abre a janela do mapa (_YardToMap).
            s.abrir_mapa();
            println!("mapa aberto (modo {})", s.partida.estado.vistas.modo_mapa);
            return Resposta::Tratada;
        }
        aplicar_acao_quintal(s, a, true);
        return Resposta::Tratada;
    }
    Resposta::Seguir
}

pub(crate) fn escolher(s: &mut Sessao, e: Escolha) -> Resposta {
    s.modos.escolhendo = false;
    match e {
        Escolha::Cancelar => Resposta::Tratada,
        // Load Game e os jogos novos: o laço da janela decide (Load Game = File → Load, jogo.rs).
        outra => Resposta::NovoJogo(outra),
    }
}

/// Os efeitos das ferramentas viram pedidos da fila (som, música) ou linhas no terminal.
fn tratar_efeitos(s: &mut Sessao, efeitos: Vec<Efeito>) {
    for e in efeitos {
        match e {
            Efeito::Som { id, .. } => s.partida.estado.eventos.push(Evento::Som(id)),
            Efeito::Musica(id) => s.partida.estado.eventos.push(Evento::Musica(id)),
            // _EditMessage(texto, 0xB4, 0, 1) das ferramentas (1010:763b): faixa, importante.
            Efeito::Aviso { lista, indice } => s.partida.estado.eventos.push(Evento::MensagemFaixa(engine::rules::estado::MensagemFaixa {
                lista,
                indice: indice as u8,
                duracao: 0xB4,
                importante: true,
            })),
            Efeito::Examinar(x) => s.abrir_exame(x),
        }
    }
    s.partida.processar_eventos_do_estado();
}

/// Botão de ferramentas (`_RibbonToolsMenu`): Quick Game → Experimental; no Experimental abre o
/// menu; no Full Game e no Tutorial, nada (o original mostra só uma figura no Full Game).
fn botao_ferramentas(s: &mut Sessao) {
    match s.partida.estado.partida.tipo_jogo {
        1 => {
            let c = Contexto { tabelas: &s.modos.tabelas_exp, amarela: &s.partida.tabelas_amarela, inimigos: &s.partida.regras.inimigos };
            let e = &mut s.partida.estado;
            let efeitos = experimental::quick_para_experimental(e, c);
            println!("Quick Game -> Experimental");
            tratar_efeitos(s, efeitos);
        }
        3 => {
            if let Some(r) = s.tela.retangulo_da_faixa(&s.modos.menu.cfg.botao) {
                let atual = Ferramenta::de_i16(s.partida.estado.partida.ferramenta_experimental);
                s.modos.menu.abrir(r, atual);
                s.modos.menu_aberto = true;
            }
        }
        _ => println!("ferramentas: nada neste tipo de jogo"),
    }
}

fn clique_no_menu(s: &mut Sessao, x: i32, y: i32) {
    if !s.modos.menu.contem(x, y) {
        s.modos.menu_aberto = false;
        return;
    }
    let Some(item) = s.modos.menu.item_em(x, y) else { return };
    let plano = i32::from(s.partida.nav.plano_mapa);
    let (f, sub, fecha) = match item {
        ItemMenu::Ferramenta(f) => (f, None, s.modos.menu.submenu.is_none()),
        ItemMenu::Subopcao(f, i) => (f, Some(i), true),
    };
    let mut ctl = s.modos.controle_exp.clone();
    let c = Contexto { tabelas: &s.modos.tabelas_exp, amarela: &s.partida.tabelas_amarela, inimigos: &s.partida.regras.inimigos };
    let e = &mut s.partida.estado;
    let efeitos = experimental::escolher_ferramenta(e, c, &mut ctl, f, sub, plano);
    s.modos.controle_exp = ctl;
    println!("ferramenta {f:?} (subopção {:?})", sub.unwrap_or(s.modos.controle_exp.subopcao(f)));
    if fecha {
        s.modos.menu_aberto = false;
    }
    tratar_efeitos(s, efeitos);
}

/// Experimental: botão apertado (`pressionar`) ou arrasto (`arrastar`) numa célula do mapa.
fn ferramenta_no_mapa(s: &mut Sessao, x: i32, y: i32, ctrl: bool, arrasto: bool) -> bool {
    let Some((cx, cy)) = s.tela.celula_em(x, y, &s.partida.nav) else { return false };
    ferramenta_na_celula(s, cx, cy, ctrl, arrasto);
    true
}

/// Experimental: a ferramenta na célula (cx, cy) do plano mostrado (`_processExp`).
pub(crate) fn ferramenta_na_celula(s: &mut Sessao, cx: i32, cy: i32, ctrl: bool, arrasto: bool) {
    let plano = i32::from(s.partida.nav.plano_mapa);
    let mut ctl = s.modos.controle_exp.clone();
    let c = Contexto { tabelas: &s.modos.tabelas_exp, amarela: &s.partida.tabelas_amarela, inimigos: &s.partida.regras.inimigos };
    let e = &mut s.partida.estado;
    let efeitos = if arrasto {
        experimental::arrastar(e, c, &mut ctl, plano, cx, cy, ctrl)
    } else {
        experimental::pressionar(e, c, &mut ctl, plano, cx, cy, ctrl)
    };
    s.modos.controle_exp = ctl;
    tratar_efeitos(s, efeitos);
}

/// Menu de ferramentas aberto (`_DoExpMenu`, menu "de proximidade"): o mouse andando com o botão
/// apertado realça o item sob ele e abre o submenu dele. Devolve se o menu está aberto.
pub fn mover_no_menu(s: &mut Sessao, x: i32, y: i32) -> bool {
    if !s.modos.menu_aberto {
        return false;
    }
    let _ = s.modos.menu.item_em(x, y);
    true
}

/// Botão solto com o menu de ferramentas aberto: escolhe o item sob o mouse (do menu ou do submenu)
/// e fecha; solto fora (inclusive no próprio botão da faixa), fecha sem escolher [bancada: i05/i06].
/// Devolve se o menu estava aberto.
pub fn soltar_no_menu(s: &mut Sessao, x: i32, y: i32) -> bool {
    if !s.modos.menu_aberto {
        return false;
    }
    if s.modos.menu.contem(x, y) {
        clique_no_menu(s, x, y);
    }
    s.modos.menu_aberto = false;
    true
}

/// Mouse andando com o botão esquerdo apertado: arrasto da ferramenta do Experimental.
pub fn mover(s: &mut Sessao, x: i32, y: i32, ctrl: bool) -> bool {
    if s.modos.controle_exp.atual.is_none() || s.partida.estado.partida.tipo_jogo != 3 {
        return false;
    }
    ferramenta_no_mapa(s, x, y, ctrl, true)
}

/// Botão solto: fim do arrasto.
pub fn soltar(s: &mut Sessao) {
    experimental::soltar(&mut s.modos.controle_exp);
}

/// Clique com um "popup" dos modos aberto ("Select a Game" do Ctrl+N, menu de ferramentas do
/// Experimental): eles ficam por cima das janelas e recebem o clique antes delas.
pub fn clique_popup(s: &mut Sessao, x: i32, y: i32) -> Resposta {
    if s.modos.escolhendo {
        return match s.modos.selecao.escolha_em(x, y) {
            Some(e) => escolher(s, e),
            None => Resposta::Tratada,
        };
    }
    if s.modos.menu_aberto {
        clique_no_menu(s, x, y);
        return Resposta::Tratada;
    }
    Resposta::Seguir
}

/// Uma ação do quintal (tecla ou botão da faixa) com o gerenciador de janelas: o Patch é
/// `_YardToMap`; abrir ou trocar de modo por tecla traz o quintal para a frente (`_MapToYard`).
pub fn aplicar_acao_quintal(s: &mut Sessao, a: AcaoQuintal, trazer: bool) {
    match a {
        AcaoQuintal::ParaMapa => s.quintal_para_mapa(),
        AcaoQuintal::Fechar => {
            let id = s.janelas.cfg.quintal;
            s.fechar_janela(id, false);
        }
        AcaoQuintal::Revoada => {
            let armou = s.partida.botao_revoada();
            println!("quintal: MATE {}", if armou { "armou a revoada" } else { "recusado" });
        }
        _ => {
            let aberta = s.janelas.aberta(Controle::Quintal);
            s.modos.janela.aplicar(a, &mut s.partida.estado);
            // _PlaceQueenInYard: _StopSimulation até o clique.
            s.partida.parada_pela_interface = s.modos.janela.colocando_rainha;
            s.partida.processar_eventos_do_estado();
            let abre = matches!(
                a,
                AcaoQuintal::Abrir | AcaoQuintal::ModoQuintal | AcaoQuintal::ModoCasa | AcaoQuintal::ModoEstrategico | AcaoQuintal::ModoGrafico | AcaoQuintal::AlternarCasa
            );
            // _ProcMenu 0x21–0x24 (Ctrl+Y/5/6/7): _SetMapPlane(0) se _MapPlane ≠ 0, antes de abrir.
            let tecla_de_modo = matches!(a, AcaoQuintal::ModoQuintal | AcaoQuintal::ModoCasa | AcaoQuintal::ModoEstrategico | AcaoQuintal::ModoGrafico);
            if trazer && tecla_de_modo && s.partida.nav.plano_mapa != 0 {
                s.partida.ir_para_quintal();
            }
            if abre && (trazer || !aberta) {
                s.abrir_quintal();
            }
        }
    }
    println!("quintal: {a:?} (modo {})", s.partida.estado.vistas.modo_quintal);
}

/// Clique fora das janelas (na faixa): botão de ferramentas e botão Yard na faixa 34; os botões da
/// faixa 35 quando ela está na tela.
pub fn clique_na_faixa(s: &mut Sessao, x: i32, y: i32) -> Resposta {
    if barras_da_faixa(s, x, y) {
        return Resposta::Tratada;
    }
    if super::ajuda_ui::botao_da_faixa(s, x, y) {
        return Resposta::Tratada;
    }
    if s.modos.janela.na_frente {
        // Faixa 35 no lugar da 34.
        if y < i32::from(s.interface.moldura.faixa.altura) {
            // Botões configuráveis da faixa 35 (`_ProcYardRibbonEvent` → `_DoUserButton`): quem
            // trata é o laço da janela, como os da faixa 34.
            if s.botao_usuario_em(x, y).is_some() {
                return Resposta::Seguir;
            }
            if let Some(a) = s.modos.quintal.acao_da_faixa_em(x, y) {
                aplicar_acao_quintal(s, a, false);
            }
            return Resposta::Tratada;
        }
        return Resposta::Seguir;
    }
    let objetos = s.tela.objetos_da_faixa_em(x, y);
    if objetos.contains(&s.modos.menu.cfg.botao.as_str()) {
        botao_ferramentas(s);
        return Resposta::Tratada;
    }
    if objetos.contains(&s.modos.quintal.cfg.botao_abrir.as_str()) {
        aplicar_acao_quintal(s, AcaoQuintal::Abrir, true);
        return Resposta::Tratada;
    }
    Resposta::Seguir
}

/// Área das barras da faixa (`mapRibCaste` / `yardRibCaste`): o clique na barra de saúde da amarela
/// ou da colônia preta muda o limiar de aviso; no resto da área, abre o quadro das castas até soltar
/// o botão (`_DrawCastePopUp`: `_LessonTemp` = 1, jogo pausado enquanto aberto).
fn barras_da_faixa(s: &mut Sessao, x: i32, y: i32) -> bool {
    let quintal = s.modos.janela.na_frente;
    let clique = {
        let (tela, desenho) = (&s.tela, &s.modos.quintal);
        let objeto = |n: &str| if quintal { desenho.retangulo_da_faixa(n) } else { tela.retangulo_da_faixa(n) };
        s.modos.popup_castas.clique(x, y, quintal, &objeto)
    };
    let Some(c) = clique else { return false };
    match c {
        CliqueBarras::AvisoAmarela(v) => {
            s.partida.estado.amarela.aviso_saude = v;
            println!("_MeWarnHealth = {v}");
        }
        CliqueBarras::AvisoColonia(v) => {
            s.partida.estado.partida.aviso_saude_colonia = v;
            println!("_BlkWarnHealth = {v}");
        }
        CliqueBarras::Castas => {
            s.partida.estado.tutorial.temporario = 1;
            s.modos.castas_aberto = true;
        }
    }
    true
}

/// Há um quadro aberto só enquanto o botão fica apertado (quadro das castas)?
pub fn segurando(s: &Sessao) -> bool {
    s.modos.castas_aberto
}

/// Botão solto: fecha o quadro das castas.
pub fn soltar_segurado(s: &mut Sessao) {
    s.modos.castas_aberto = false;
}

/// Clique na área cliente da janela de edição: no Experimental, vai para a ferramenta
/// (`_processExp`). Devolve se tratou.
pub fn clique_na_edicao(s: &mut Sessao, x: i32, y: i32, ctrl: bool) -> bool {
    s.partida.estado.partida.tipo_jogo == 3 && ferramenta_no_mapa(s, x, y, ctrl, false)
}

/// Clique na área cliente do quintal (que é a janela da frente): patch, troca de patch, PLACE.
pub fn clique_no_quintal(s: &mut Sessao, x: i32, y: i32, duplo: bool, shift: bool) {
    let m = &mut s.modos;
    match quintal_mapa::clicar(&mut m.janela, &m.quintal.cfg, &mut s.partida, x, y, duplo, shift) {
        Ok(r) => println!("quintal: clique {} em ({x}, {y}): {r:?}", if duplo { "duplo" } else { "simples" }),
        Err(e) => eprintln!("quintal: {e}"),
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::*;

    fn raiz() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    /// Clique esquerdo como no laço da janela (jogo.rs): janelas primeiro, depois a faixa.
    fn clicar(s: &mut Sessao, x: i32, y: i32) {
        match s.clicar_nas_janelas(x, y, false) {
            janelas_ui::Roteamento::Fora => {
                clique_na_faixa(s, x, y);
            }
            _ => println!("roteado para uma janela"),
        }
    }

    #[test]
    fn opcoes_e_botoes_passam_ao_jogo_novo_mas_o_save_traz_as_suas() {
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SHARED.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
            return;
        }
        let dados = raiz().join("data");
        let cfg: ConfigJogo = serde_json::from_str(&std::fs::read_to_string(dados.join("jogo.json")).unwrap()).unwrap();
        let recursos = Recursos::abrir(&jogo, &cfg.bancos, &dados, None).unwrap();
        let cores = recursos.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
        let save = raiz().join("tests/referencias/saves/QUICK2.ANT");
        let mut velha = Sessao::abrir(&recursos, &dados, Origem::Save(&save, GeradorR::default(), TipoJogo::Rapido), cores.clone()).unwrap();
        let do_arquivo = velha.partida.estado.partida.opcoes;
        // O jogador liga AutoTrack e Silly e troca a função do último botão configurável.
        velha.partida.estado.partida.opcoes[0] = 1;
        velha.partida.estado.partida.opcoes[5] = 1;
        velha.partida.funcoes_botoes[7] = 11;
        // Jogo novo (_NewGame não toca em _OptionStates): ficam as opções e os botões do momento.
        let mut nova = Sessao::abrir(&recursos, &dados, Origem::NovoNaPartida(&velha.partida.estado, TipoJogo::Rapido), cores.clone()).unwrap();
        nova.herdar_opcoes(&velha, false);
        assert_eq!(nova.partida.estado.partida.opcoes, velha.partida.estado.partida.opcoes);
        assert_eq!(nova.partida.funcoes_botoes[7], 11);
        // Load de um .ANT: as opções vêm do arquivo; os botões (fora do save) ficam.
        let mut lida = Sessao::abrir(&recursos, &dados, Origem::Save(&save, GeradorR::default(), TipoJogo::Rapido), cores).unwrap();
        lida.herdar_opcoes(&velha, true);
        assert_eq!(lida.partida.estado.partida.opcoes, do_arquivo);
        assert_eq!(lida.partida.funcoes_botoes[7], 11);
    }

    #[test]
    fn ciclo_do_botao_yard_patch() {
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SHARED.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
            return;
        }
        let dados = raiz().join("data");
        let cfg: ConfigJogo = serde_json::from_str(&std::fs::read_to_string(dados.join("jogo.json")).unwrap()).unwrap();
        let recursos = Recursos::abrir(&jogo, &cfg.bancos, &dados, None).unwrap();
        let cores = recursos.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
        let save = raiz().join("tests/referencias/saves/QUICK2.ANT");
        let mut s = Sessao::abrir(&recursos, &dados, Origem::Save(&save, GeradorR::default(), TipoJogo::Rapido), cores).unwrap();
        let (r, _) = s.tela.objeto_da_faixa(&s.modos.quintal.cfg.botao_abrir).unwrap();
        let (x, y) = (i32::from(r.esq + r.dir) / 2, i32::from(r.topo + r.base) / 2);
        let quintal = s.janelas.cfg.quintal;
        let mapa = s.janelas.cfg.id(Controle::Mapa);
        // Bancada (qemu_patch_rolagem.txt, p01–p06): o mesmo lugar da faixa é o Yard da 34 e o Patch
        // da 35; seis cliques alternam quintal (faixa 35) e mapa da superfície (faixa 34), sempre
        // com o modo do quintal mantido. Nunca fecha as duas.
        assert_eq!(s.partida.nav.plano_mapa, 2, "QUICK2.ANT abre no ninho preto");
        for i in 0..6 {
            let canto = (s.partida.nav.vista.x, s.partida.nav.vista.y);
            clicar(&mut s, x, y);
            if i % 2 == 0 {
                // _MapToYard → _SetMapPlane(0): a edição mostra a superfície no mesmo canto (p01, p03).
                assert_eq!(s.partida.nav.plano_mapa, 0, "clique {i}: _MapPlane 0");
                assert_eq!(s.partida.nav.vista.plano, engine::core::mapa::Plano::Superficie);
                assert_eq!((s.partida.nav.vista.x, s.partida.nav.vista.y), canto, "clique {i}: sem centralizar");
                assert_eq!(s.partida.estado.vistas.plano, 0);
                assert_eq!(s.partida.estado.vistas.plano_anterior, 0);
                let p = s.partida.estado.ambiente.patch;
                assert_eq!(s.partida.nav.pontos[0], (i32::from(p.0), i32::from(p.1)), "_YMapPnt = _CurYardPnt");
                s.quadro_rgb().unwrap();
            }
            let estado = (s.janelas.gerente.topo(), s.janelas.aberta(Controle::Quintal), s.janelas.aberta(Controle::Mapa), s.modos.janela.na_frente);
            if i % 2 == 0 {
                assert_eq!(estado, (Some(quintal), true, false, true), "clique {i}: quintal na frente, faixa 35");
            } else {
                assert_eq!(estado, (Some(mapa), false, true, false), "clique {i}: mapa na frente, faixa 34");
                assert_eq!(s.partida.nav.plano_mapa, 1, "clique {i}: _YardToMap vai à superfície");
            }
            assert_eq!(s.partida.estado.vistas.modo_quintal, 0);
        }
    }

    #[test]
    fn info_do_experimental_abre_o_exame() {
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SHARED.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
            return;
        }
        let dados = raiz().join("data");
        let cfg: ConfigJogo = serde_json::from_str(&std::fs::read_to_string(dados.join("jogo.json")).unwrap()).unwrap();
        let recursos = Recursos::abrir(&jogo, &cfg.bancos, &dados, None).unwrap();
        let cores = recursos.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
        let mut s = Sessao::abrir(&recursos, &dados, Origem::Novo(1234, TipoJogo::Experimental), cores).unwrap();
        assert_eq!(s.partida.estado.partida.tipo_jogo, 3);
        // Uma célula da área do mapa da edição.
        let [ax, ay] = s.tela.cfg.edicao.area_na_tela;
        let (x, y) = (i32::from(ax) + 16 * 5 + 8, i32::from(ay) + 16 * 5 + 8);
        let (cx, cy) = s.tela.celula_em(x, y, &s.partida.nav).unwrap();
        // Põe uma formiga (ferramenta 3) e examina com a Info (ferramenta 0).
        s.partida.estado.partida.ferramenta_experimental = Ferramenta::Formiga as i16;
        assert!(matches!(s.clicar_nas_janelas(x, y, false), janelas_ui::Roteamento::Edicao));
        assert!(clique_na_edicao(&mut s, x, y, false));
        soltar(&mut s);
        let plano = i32::from(s.partida.nav.plano_mapa);
        assert!(engine::rules::comum::vida_bruta(&s.partida.estado, plano, cx, cy).unwrap_or(0) != 0, "formiga posta em ({cx}, {cy})");
        s.partida.estado.partida.ferramenta_experimental = Ferramenta::Info as i16;
        assert!(matches!(s.clicar_nas_janelas(x, y, false), janelas_ui::Roteamento::Edicao));
        assert!(clique_na_edicao(&mut s, x, y, false));
        assert!(s.exame.is_some(), "Info numa formiga abre o Examine");
        let rgb = s.quadro_rgb().unwrap();
        if let Some(saida) = std::env::var_os("SIMANT_SAIDA_TESTE") {
            crate::salvar_png(&PathBuf::from(saida).join("info_exame.png"), s.tela.cfg.largura, s.tela.cfg.altura, &rgb).unwrap();
        }
        s.exame = None;
        // Menu de ferramentas como no original (i05/i06): apertar o botão abre com a Info realçada;
        // soltar no próprio botão fecha sem escolher; apertar, arrastar até BARRIERS e soltar escolhe.
        s.partida.estado.partida.ferramenta_experimental = Ferramenta::Info as i16;
        let (bx, by) = (354, 70 - 37);
        clique_na_faixa(&mut s, bx, by);
        assert!(s.modos.menu_aberto);
        assert_eq!(s.modos.menu.realce, Some(Ferramenta::Info));
        let rgb = s.quadro_rgb().unwrap();
        if let Some(saida) = std::env::var_os("SIMANT_SAIDA_TESTE") {
            crate::salvar_png(&PathBuf::from(saida).join("menu_ferramentas.png"), s.tela.cfg.largura, s.tela.cfg.altura, &rgb).unwrap();
        }
        assert!(soltar_no_menu(&mut s, bx, by));
        assert!(!s.modos.menu_aberto);
        assert_eq!(s.partida.estado.partida.ferramenta_experimental, Ferramenta::Info as i16);
        clique_na_faixa(&mut s, bx, by);
        let r = s.tela.retangulo_da_faixa(&s.modos.menu.cfg.botao).unwrap();
        let o = s.modos.menu.origem;
        // Item 3 (formiga) do menu: os itens começam 3 pixels abaixo do canto do objeto 0 e têm 17 de
        // altura; soltar nele escolhe a ferramenta 3 com a subopção atual (o submenu abre ao lado).
        let (ix, iy) = (i32::from(o.0) + 40, i32::from(o.1) + 3 + 17 * 3 + 8);
        assert!(r.base <= o.1);
        assert!(mover_no_menu(&mut s, ix, iy));
        assert_eq!(s.modos.menu.realce, Some(Ferramenta::Formiga));
        assert!(soltar_no_menu(&mut s, ix, iy));
        assert_eq!(s.partida.estado.partida.ferramenta_experimental, Ferramenta::Formiga as i16);
        assert!(!s.modos.menu_aberto);
    }

    /// Bancada (qemu_ui_barras_direito.txt, 2026-10-18): segurar o botão no número da população preta
    /// abre o quadro das castas (b01); o clique na barra de saúde da colônia em x = 580 dá
    /// `_BlkWarnHealth` 20 e na da amarela em x = 620 dá `_MeWarnHealth` 82 (tela − 37 linhas = área
    /// cliente do port).
    #[test]
    fn barras_da_faixa_como_no_original() {
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SHARED.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
            return;
        }
        let dados = raiz().join("data");
        let cfg: ConfigJogo = serde_json::from_str(&std::fs::read_to_string(dados.join("jogo.json")).unwrap()).unwrap();
        let recursos = Recursos::abrir(&jogo, &cfg.bancos, &dados, None).unwrap();
        let cores = recursos.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
        let save = raiz().join("tests/referencias/saves/QUICK2.ANT");
        let mut s = Sessao::abrir(&recursos, &dados, Origem::Save(&save, GeradorR::default(), TipoJogo::Rapido), cores).unwrap();
        clicar(&mut s, 580, 77 - 37);
        assert_eq!(s.partida.estado.partida.aviso_saude_colonia, 20);
        clicar(&mut s, 620, 48 - 37);
        assert_eq!(s.partida.estado.amarela.aviso_saude, 82);
        assert!(!segurando(&s));
        // Números do snapshot quick no momento da captura b01.
        s.partida.estado.preta.castas = [2, 1, 0, 0, 1, 1];
        s.partida.estado.vermelha.castas = [2, 1, 0, 0, 0, 1];
        clicar(&mut s, 536, 69 - 37);
        assert!(segurando(&s), "o quadro das castas abre");
        assert_eq!(s.partida.estado.tutorial.temporario, 1);
        let rgb = s.quadro_rgb().unwrap();
        if let Some(saida) = std::env::var_os("SIMANT_SAIDA_TESTE") {
            crate::salvar_png(&PathBuf::from(saida).join("popup_castas.png"), s.tela.cfg.largura, s.tela.cfg.altura, &rgb).unwrap();
        }
        soltar_segurado(&mut s);
        assert!(!segurando(&s));
    }

    /// Abre o save do cenário `baloes` da bancada, põe a vista do ninho preto com o canto em (22, 0),
    /// desenha a tela sem e com o balão e devolve (sem, com, largura). Com `SIMANT_SAIDA_TESTE`, grava
    /// `<nome>.png` para comparar com tests/referencias/capturas/<nome>.png (fora do git; topo 38).
    fn tela_com_balao(save: &str, balao: engine::rules::baloes::Balao, nome: &str) -> Option<(Vec<u8>, Vec<u8>, usize)> {
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SHARED.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
            return None;
        }
        let dados = raiz().join("data");
        let cfg: ConfigJogo = serde_json::from_str(&std::fs::read_to_string(dados.join("jogo.json")).unwrap()).unwrap();
        let recursos = Recursos::abrir(&jogo, &cfg.bancos, &dados, None).unwrap();
        let cores = recursos.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
        let save = raiz().join("tests/referencias/saves").join(save);
        let mut s = Sessao::abrir(&recursos, &dados, Origem::Save(&save, GeradorR::default(), TipoJogo::Rapido), cores).unwrap();
        s.partida.nav.vista.x = 22;
        s.partida.nav.vista.y = 0;
        let sem = s.quadro_rgb().unwrap();
        s.partida.estado.baloes.desenhado = Some(balao);
        let com = s.quadro_rgb().unwrap();
        if let Some(saida) = std::env::var_os("SIMANT_SAIDA_TESTE") {
            crate::salvar_png(&PathBuf::from(saida).join(format!("{nome}.png")), s.tela.cfg.largura, s.tela.cfg.altura, &com).unwrap();
        }
        Some((sem, com, s.tela.cfg.largura as usize))
    }

    fn pixel(rgb: &[u8], w: usize, x: usize, y: usize) -> [u8; 3] {
        let i = (y * w + x) * 3;
        [rgb[i], rgb[i + 1], rgb[i + 2]]
    }

    /// Bancada (cenário `baloes`, intervalo 18, 2026-10-18): Silly ligado; falas do ovo e da rainha
    /// ligadas, só o balão do ovo (o primeiro pedido) aparece, com a fala 1 da lista 1240 ("Sure is
    /// dark\nin here!") sobre o ovo em (34, 8). Comparado com a captura: a região do balão
    /// (x 225–285, linhas 218–250 da área cliente) é igual pixel a pixel.
    #[test]
    fn balao_do_ovo_como_no_original() {
        use engine::rules::baloes::{Balao, Fala};
        let b = Balao { x: 34 * 16 + 8, y: 8 * 16 + 8, plano: 2, fala: Fala::Ovo, indice: 1 };
        let Some((sem, com, w)) = tela_com_balao("BALAO_OVO.ANT", b, "balao_ovo") else { return };
        // Captura: borda esquerda do balão na coluna 229, corpo branco nas linhas 224–245.
        assert_eq!(pixel(&com, w, 260, 233), [255, 255, 255]);
        assert_ne!(pixel(&sem, w, 260, 233), [255, 255, 255]);
        assert_eq!(pixel(&com, w, 229, 233), [0, 0, 0]);
    }

    /// Bancada (cenário `baloes`, intervalo 6): só a fala da rainha ligada, fala 1 da lista 1250
    /// ("Sometimes I think all I\ndo is lay eggs!") sobre a rainha em (34, 6).
    #[test]
    fn balao_da_rainha_como_no_original() {
        use engine::rules::baloes::{Balao, Fala};
        let b = Balao { x: 34 * 16 + 8, y: 6 * 16 + 8, plano: 2, fala: Fala::Rainha, indice: 1 };
        let Some((sem, com, w)) = tela_com_balao("BALAO_RAINHA.ANT", b, "balao_rainha") else { return };
        assert_eq!(pixel(&com, w, 260, 200), [255, 255, 255]);
        assert_ne!(pixel(&sem, w, 260, 200), [255, 255, 255]);
    }

    /// Bancada (cenário `trofalaxia`, 2026-10-18): o quadro de `_YellowDialog(9110)` com a contorno
    /// azul-escuro de 4 pixels em (156, 159)–(361, 326) da área cliente; depois de 45 unidades de
    /// `_TickCount`, o bitmap 9111 sobreposto em (72, 62), com o som 45; fecha sozinho depois de
    /// 45 + 45 + 54. Capturas trofalaxia_028/030.png (fora do git, topo 38).
    #[test]
    fn quadro_da_trofalaxia_como_no_original() {
        use engine::core::avisos::Modal;
        let jogo = std::env::var_os("SIMANT_DIR").map(PathBuf::from).unwrap_or(raiz().join("SimAnt-T/MAXIS/ANTWIN"));
        if !jogo.join("SHARED.NDX").is_file() {
            eprintln!("AVISO: arquivos do jogo não encontrados; teste ignorado");
            return;
        }
        let dados = raiz().join("data");
        let cfg: ConfigJogo = serde_json::from_str(&std::fs::read_to_string(dados.join("jogo.json")).unwrap()).unwrap();
        let recursos = Recursos::abrir(&jogo, &cfg.bancos, &dados, None).unwrap();
        let cores = recursos.dados::<Paleta>("paleta.json").unwrap().rgb().unwrap();
        let save = raiz().join("tests/referencias/saves/BALAO_OVO.ANT");
        let mut s = Sessao::abrir(&recursos, &dados, Origem::Save(&save, GeradorR::default(), TipoJogo::Rapido), cores).unwrap();
        let a = &mut s.partida.avisos;
        assert!(a.receber_quadro_amarela(9110));
        assert_eq!(a.passar_tempo(1000), None);
        assert_eq!(a.retirar_sons(), vec![42]);
        let primeiro = s.quadro_rgb().unwrap();
        let a = &mut s.partida.avisos;
        assert_eq!(a.passar_tempo(1044), None);
        assert!(!a.sobreposta_visivel());
        assert_eq!(a.passar_tempo(1045), None);
        assert!(a.sobreposta_visivel());
        assert_eq!(a.retirar_sons(), vec![45]);
        let segundo = s.quadro_rgb().unwrap();
        if let Some(saida) = std::env::var_os("SIMANT_SAIDA_TESTE") {
            let (l, h) = (s.tela.cfg.largura, s.tela.cfg.altura);
            let saida = PathBuf::from(saida);
            crate::salvar_png(&saida.join("trofalaxia_a.png"), l, h, &primeiro).unwrap();
            crate::salvar_png(&saida.join("trofalaxia_b.png"), l, h, &segundo).unwrap();
        }
        let w = s.tela.cfg.largura as usize;
        assert_eq!(pixel(&primeiro, w, 157, 200), [0, 0, 128], "contorno azul-escuro");
        assert_ne!(primeiro, segundo, "a imagem sobreposta muda o quadro");
        let a = &mut s.partida.avisos;
        assert_eq!(a.passar_tempo(1000 + 45 + 45 + 53), None);
        assert_eq!(a.passar_tempo(1000 + 45 + 45 + 54), Some(Modal::Amarela { imagem: 9110 }));
    }
}
