//! Janelas de controle da partida: Caste Control (janela 19) e Behavior Control (janela 18), com o
//! triângulo que regula `_casteLevels`/`_modeLevels`, os botões Auto/Manual/Set e os níveis.
//!
//! Especificação: docs/spec/telas/janelas.md. Dados: data/janelas.json. Sem SDL: a interface só
//! repassa cliques, arrastos e o desenho.

use std::collections::HashMap;

use serde::Deserialize;

use super::gerenciador::{Clique, ConfigGerenciador, Estilo, Gerenciador, Metricas};
use super::historico::{ConfigHistorico, JanelaHistorico};
use super::quadro::Quadro;
use super::telas::{carregar_janela, desenhar_objetos, Interface};
use super::triangulo::{castas_ideais, texto_do_nivel, Triangulo};
use crate::formats::banco::Chave;
use crate::formats::bitmap::{Bitmap, TIPO_BITMAP};
use crate::formats::janela::{Cor, Janela, Retangulo};
use crate::recursos::Recursos;
use crate::rules::estado::Estado;

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_CONFIG: &str = "janelas.json";

/// Qual janela de controle.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Controle {
    /// Caste Control (`_OpenCasteWindow`, janela 19): `_casteLevels`.
    Castas,
    /// Behavior Control (`_OpenModeWindow`, janela 18): `_modeLevels`.
    Tarefas,
    /// History (`_OpenHistoryWindow`, janela 21): gráficos do histórico (core/historico.rs).
    Historico,
    /// Mapa (`_OpenMapWindow`, janela 1): desenhado por core/janela_mapa.rs; aqui só a pilha, a
    /// geometria e o clique na área (`clique_mapa`).
    Mapa,
    /// Info (`_OpenInfoWindow`, janela 5): desenhada por core/info.rs; aqui a pilha, a geometria e
    /// o clique na área (`clique_info`).
    Info,
    /// Quintal (`_MapToYard`, janela 25): desenhado e clicado por core/quintal_mapa.rs; aqui só a
    /// pilha e a geometria. Divide com o mapa a mesma janela do Windows (`_win_Swap`).
    Quintal,
}

/// Janela de edição (`_win_hwnd[0]`) no gerenciador.
pub const ID_EDICAO: u16 = 0;

/// Ação de um acelerador de janela (`MAINWNDPROC`, comandos 0xFDA0–0xFDA5).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcaoJanela {
    /// 0xFDA0 (Ctrl+F4, Ctrl+NumPad−): `WM_CLOSE` na janela de cima.
    FecharDeCima,
    /// 0xFDA1 (Ctrl+Espaço): `SC_KEYMENU`, o menu de sistema da janela de cima.
    MenuSistema,
    /// 0xFDA2 (Ctrl+F6, Ctrl+NumPad+): a janela de baixo vem para cima.
    Proxima,
    /// 0xFDA3 (Ctrl+P): `SC_MOVE` na janela de cima.
    Mover,
    /// 0xFDA5 (Ctrl+Z): maximiza ou restaura a janela de edição, se ela é a de cima.
    MaximizarEdicao,
}

/// Tecla → ação de janela (mesmo formato dos atalhos de data/partida.json).
#[derive(Clone, Debug, Deserialize)]
pub struct AtalhoJanela {
    pub tecla: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    pub acao: AcaoJanela,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigControle {
    pub janela: u16,
    pub titulo: String,
    pub objeto_triangulo: String,
    pub objeto_graficos: String,
    /// Objetos onde os três níveis são escritos (`_DrawControlLevels`), na ordem dos níveis.
    pub objetos_niveis: [String; 3],
    pub objeto_auto: String,
    pub objeto_manual: String,
    pub objetos_conjuntos: [String; 3],
    /// Áreas que alternam porcentagem/número (`_DrawControlLevels` com `!_ShowPercent`).
    pub objetos_alternar: [String; 3],
    /// Cor (índice da paleta) de cada barra; no original vêm de `_casteColors`/`_modeColors` por
    /// `_ConvColor`, cuja tabela não foi lida [medido nas capturas].
    pub cores_barras: [u8; 3],
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigJanelas {
    pub castas: ConfigControle,
    pub tarefas: ConfigControle,
    /// Ajuste da posição em relação à tabela de posições (tipo 9) + faixa [medido na bancada QEMU,
    /// 640×480: 2 linhas abaixo].
    pub ajuste_posicao: [i16; 2],
    pub altura_titulo: i16,
    pub fonte_titulo: Vec<usize>,
    /// Cores do título: [fundo, texto] da janela ativa e da inativa (índices da paleta).
    pub titulo_ativo: [u8; 2],
    pub titulo_inativo: [u8; 2],
    pub cor_linha: u8,
    pub cor_fundo: u8,
    pub cor_caixa_sistema: u8,
    /// Traço do menu de sistema: miolo e sombra.
    pub cor_traco: u8,
    pub cor_sombra: u8,
    /// Botão do triângulo (`_knobSize`, bitmap 0x578).
    pub bitmap_botao: u16,
    /// Índice transparente do botão (como o 13 dos tiles de vida).
    pub cor_transparente: u8,
    /// Deslocamento do botão em relação à conta de `_UpdateCasteWindow` [medido].
    pub ajuste_botao: [i16; 2],
    pub fonte_niveis: usize,
    /// Cor do contorno das barras (`_ConvColor(0xf)`) e do fundo do gráfico (`_ConvColor(0)`).
    pub cor_contorno_barra: u8,
    pub cor_fundo_grafico: u8,
    pub historico: ConfigHistorico,
    /// Janela do mapa (layout 1) e o objeto da área (`mapTileArea`).
    pub mapa: ConfigJanelaMapa,
    /// Janela Info (layout 5) e o objeto da área (`infoArea`).
    pub info: ConfigJanelaMapa,
    /// Janela do quintal (layout 25; a geometria vem de data/quintal_mapa.json).
    pub quintal: u16,
    /// Moldura, menu de sistema e limites (core/gerenciador.rs).
    pub gerenciador: ConfigGerenciador,
    /// Aceleradores de janela (0xFDA0–0xFDA5 da `ACCELERATORTABLE`).
    pub teclas: Vec<AtalhoJanela>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigJanelaMapa {
    pub janela: u16,
    pub objeto_area: String,
    /// Ajuste da janela (somado ao `ajuste_posicao` comum) e da área [medido].
    #[serde(default)]
    pub ajuste: [i16; 2],
    #[serde(default)]
    pub ajuste_area: [i16; 2],
}

impl ConfigJanelas {
    /// Configuração da janela de controle (a History não tem: devolve a de castas, nunca usada).
    pub fn de(&self, c: Controle) -> &ConfigControle {
        match c {
            Controle::Tarefas => &self.tarefas,
            _ => &self.castas,
        }
    }

    /// Id da janela do layout.
    pub fn id(&self, c: Controle) -> u16 {
        match c {
            Controle::Historico => self.historico.janela,
            Controle::Mapa => self.mapa.janela,
            Controle::Info => self.info.janela,
            Controle::Quintal => self.quintal,
            _ => self.de(c).janela,
        }
    }

    /// Ação do acelerador de janela, se houver.
    pub fn acao_da_tecla(&self, nome: &str, ctrl: bool, shift: bool, alt: bool) -> Option<AcaoJanela> {
        self.teclas
            .iter()
            .find(|a| a.tecla.eq_ignore_ascii_case(nome) && a.ctrl == ctrl && a.shift == shift && a.alt == alt)
            .map(|a| a.acao)
    }
}

/// Estado de interface das janelas de controle (fora do save, como no original) e o gerenciador
/// de todas as janelas filhas (ordem, moldura, cliques).
#[derive(Clone, Debug)]
pub struct JanelasControle {
    pub cfg: ConfigJanelas,
    /// Ordem, moldura e roteamento das janelas filhas (edição, controles, mapa, quintal).
    pub gerente: Gerenciador,
    /// Geometria de cada janela no lugar de origem (tabela de posições + faixa), por id.
    bases: HashMap<u16, GeometriaControle>,
    /// Estilo (de `_win_Open`) de cada janela, por id.
    estilos: HashMap<u16, Estilo>,
    /// `_casteSet` / `_modeSet` (DGROUP `1048:159e`/`15a0`): conjunto escolhido (0..2).
    pub conjunto: [usize; 2],
    /// `1048:15b4`/`15b2`: mostrar porcentagem (1) ou número de formigas.
    pub porcentagem: [bool; 2],
    /// Arrasto em andamento no triângulo (ou na área do mapa).
    pub arrastando: Option<Controle>,
    /// Tabela de posições das janelas (tipo 9) e altura da faixa, da `Interface`.
    posicoes: Vec<[i16; 4]>,
    faixa: i16,
    /// Estado da janela History.
    pub historico: JanelaHistorico,
    /// Último clique (ou arrasto) na área da janela do mapa, em coordenadas da área cliente; a
    /// interface o converte em célula e centraliza a vista (`_MapAreaEvent`).
    pub clique_mapa: Option<(i32, i32)>,
    /// Último clique na área da janela Info (`_ProcInfoEvent`).
    pub clique_info: Option<(i32, i32)>,
}

fn indice(c: Controle) -> usize {
    match c {
        Controle::Tarefas => 1,
        _ => 0,
    }
}

/// Geometria de uma janela de controle na área cliente.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeometriaControle {
    pub externo: Retangulo,
    pub titulo: Retangulo,
    pub cliente: Retangulo,
    /// Deslocamento layout → área cliente.
    pub dx: i16,
    pub dy: i16,
}

impl GeometriaControle {
    fn deslocada(self, dx: i16, dy: i16) -> GeometriaControle {
        GeometriaControle {
            externo: mover(self.externo, dx, dy),
            titulo: mover(self.titulo, dx, dy),
            cliente: mover(self.cliente, dx, dy),
            dx: self.dx + dx,
            dy: self.dy + dy,
        }
    }
}

fn mover(r: Retangulo, dx: i16, dy: i16) -> Retangulo {
    Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy }
}

fn contem(r: Retangulo, x: i32, y: i32) -> bool {
    x >= i32::from(r.esq) && x < i32::from(r.dir) && y >= i32::from(r.topo) && y < i32::from(r.base)
}

/// Os controles que têm layout próprio (o quintal entra pela geometria de quintal_mapa.json).
const COM_LAYOUT: [Controle; 5] = [Controle::Castas, Controle::Tarefas, Controle::Historico, Controle::Mapa, Controle::Info];

impl JanelasControle {
    /// Gerenciador com a janela de edição (`edicao`: contorno, estilo e medidas da moldura dela) e a
    /// geometria de origem de cada janela de controle. `area`: área cliente da janela-mãe (embaixo
    /// da faixa). Nenhuma janela de controle aberta: a interface chama [`Self::janelas_padrao`].
    pub fn new(
        cfg: ConfigJanelas,
        interface: &Interface,
        recursos: &Recursos,
        edicao: (Retangulo, Metricas),
        area: Retangulo,
    ) -> Result<JanelasControle, String> {
        let historico = JanelaHistorico::new(cfg.historico.clone());
        let mut j = JanelasControle {
            historico,
            gerente: Gerenciador::new(cfg.gerenciador.clone(), area),
            bases: HashMap::new(),
            estilos: HashMap::new(),
            cfg,
            conjunto: [0, 0],
            porcentagem: [true, true],
            arrastando: None,
            clique_mapa: None,
            clique_info: None,
            posicoes: interface.posicoes.clone(),
            faixa: interface.moldura.faixa.altura,
        };

        for c in COM_LAYOUT {
            let id = j.cfg.id(c);
            let janela = carregar_janela(recursos, id)?;
            j.bases.insert(id, j.geometria_de_origem(&janela, id));
            j.estilos.insert(id, Estilo::da_janela(&janela));
        }
        let janela_edicao = carregar_janela(recursos, ID_EDICAO)?;
        let (externo, metricas) = edicao;
        j.gerente.abrir(ID_EDICAO, externo, Estilo::da_janela(&janela_edicao), metricas);

        Ok(j)
    }

    /// Medidas da moldura das janelas de controle (1 pixel, título de `altura_titulo`).
    pub fn metricas(&self) -> Metricas {
        Metricas { espessura: 1, altura_titulo: self.cfg.altura_titulo, barra_rolagem: 16 }
    }

    /// Registra a geometria de origem do quintal (contorno externo de data/quintal_mapa.json).
    pub fn registrar_quintal(&mut self, externo: Retangulo) {
        let h = self.cfg.altura_titulo;
        let cliente = Retangulo { esq: externo.esq + 1, topo: externo.topo + 1 + h + 1, dir: externo.dir - 1, base: externo.base - 1 };
        let titulo = Retangulo { esq: cliente.esq, topo: externo.topo + 1, dir: cliente.dir, base: externo.topo + 1 + h };
        self.bases.insert(self.cfg.quintal, GeometriaControle { externo, titulo, cliente, dx: 0, dy: 0 });
        self.estilos.insert(self.cfg.quintal, Estilo::do_layout([0x06, 0x04], true));
    }

    /// Controle da janela `id`, se for um.
    pub fn controle(&self, id: u16) -> Option<Controle> {
        [Controle::Castas, Controle::Tarefas, Controle::Historico, Controle::Mapa, Controle::Info, Controle::Quintal]
            .into_iter()
            .find(|&c| self.cfg.id(c) == id)
    }

    /// A janela está aberta (visível).
    pub fn aberta(&self, c: Controle) -> bool {
        self.gerente.aberta(self.cfg.id(c))
    }

    /// `_win_Open`: abre a janela (onde estava, se já foi aberta antes) ou a traz para a frente.
    pub fn abrir(&mut self, c: Controle) {
        let id = self.cfg.id(c);
        let Some(base) = self.bases.get(&id).copied() else { return };
        let estilo = self.estilos.get(&id).copied().unwrap_or_default();
        let m = self.metricas();
        self.gerente.abrir(id, base.externo, estilo, m);
    }

    /// Traz para a frente (`BringWindowToTop`): `None` = janela de edição; uma de controle é aberta
    /// se estiver fechada (`_OpenCasteWindow`…).
    pub fn trazer_para_frente(&mut self, c: Option<Controle>) {
        match c {
            None => {
                if !self.gerente.trazer_para_frente(ID_EDICAO) {
                    // _OpenEditWindow com a janela fechada: reabre onde estava.
                    let zero = Retangulo { esq: 0, topo: 0, dir: 0, base: 0 };
                    self.gerente.abrir(ID_EDICAO, zero, Estilo::default(), self.metricas());
                }
            }
            Some(c) => self.abrir(c),
        }
    }

    /// `_win_Close`.
    pub fn fechar(&mut self, c: Controle) -> bool {
        self.gerente.fechar(self.cfg.id(c))
    }

    /// `_win_Swap(de, para)` (mapa ↔ quintal): a nova assume o lugar e a posição da velha. Se a
    /// velha está fechada, a nova é aberta no lugar dela [hipótese: `ShowWindow` da janela
    /// escondida], ou no lugar de origem.
    pub fn trocar(&mut self, de: Controle, para: Controle) {
        let (a, b) = (self.cfg.id(de), self.cfg.id(para));
        let estilo = self.estilos.get(&b).copied().unwrap_or_default();
        let m = self.metricas();
        if !self.gerente.trocar(a, b, estilo, m) {
            self.abrir(para);
        }
    }

    /// `_SetDefaultWindows` (`1000:d66a`, Quick Game e Load): Caste e Behavior para a frente (abrindo),
    /// o mapa (ou o quintal no plano 0) aberto se nenhum dos dois estiver, e a janela de edição por
    /// cima. Devolve se abriu o mapa (`_YardToMap`) ou o quintal (`_MapToYard`).
    pub fn janelas_padrao(&mut self, plano_mapa: i16) -> Option<Controle> {
        self.abrir(Controle::Castas);
        self.abrir(Controle::Tarefas);
        let alvo = if plano_mapa == 0 { Controle::Quintal } else { Controle::Mapa };
        let aberta = (!self.aberta(alvo)).then(|| {
            self.abrir(alvo);
            alvo
        });
        self.gerente.trazer_para_frente(ID_EDICAO);
        aberta
    }

    /// A janela de cima (a ativa), se for de controle; `None` para a de edição.
    pub fn ativa(&self) -> Option<Controle> {
        self.gerente.topo().and_then(|id| self.controle(id))
    }

    /// Geometria da janela `c` (layout + deslocamento medido + onde o jogador a pôs).
    pub fn geometria(&self, _recursos: &Recursos, c: Controle) -> Result<GeometriaControle, String> {
        let id = self.cfg.id(c);
        self.geometria_atual(id).ok_or(format!("janela {id} sem geometria"))
    }

    fn geometria_atual(&self, id: u16) -> Option<GeometriaControle> {
        let base = *self.bases.get(&id)?;
        let Some(j) = self.gerente.janela(id) else { return Some(base) };
        Some(base.deslocada(j.externo.esq - base.externo.esq, j.externo.topo - base.externo.topo))
    }

    fn geometria_de(&self, _j: &Janela, id: u16) -> GeometriaControle {
        self.geometria_atual(id).unwrap_or(GeometriaControle {
            externo: Retangulo { esq: 0, topo: 0, dir: 0, base: 0 },
            titulo: Retangulo { esq: 0, topo: 0, dir: 0, base: 0 },
            cliente: Retangulo { esq: 0, topo: 0, dir: 0, base: 0 },
            dx: 0,
            dy: 0,
        })
    }

    fn geometria_de_origem(&self, j: &Janela, id: u16) -> GeometriaControle {
        let pos = self.posicoes.get(id as usize).copied().unwrap_or([0; 4]);
        let [ax, ay] = self.cfg.ajuste_posicao;
        let [bx, by] = [&self.cfg.mapa, &self.cfg.info].iter().find(|c| c.janela == id).map_or([0, 0], |c| c.ajuste);
        geometria_janela(j, pos, self.faixa, self.cfg.altura_titulo, [ax + bx, ay + by])
    }

    /// Retângulo de um objeto da janela na área cliente; com `mais_um`, o pixel a mais à direita e
    /// embaixo de `_win_GetObjRect` com faixa.
    fn objeto(j: &Janela, g: &GeometriaControle, nome: &str, mais_um: bool) -> Option<(Retangulo, u8)> {
        let o = j.objetos.iter().find(|o| o.nome == nome)?;
        let r = mover(o.retangulo, g.dx, g.dy);
        let k = i16::from(mais_um);
        Some((Retangulo { dir: r.dir + k, base: r.base + k, ..r }, o.cor))
    }

    fn triangulo(j: &Janela, g: &GeometriaControle, cfg: &ConfigControle) -> Option<Triangulo> {
        let (r, _) = Self::objeto(j, g, &cfg.objeto_triangulo, true)?;
        Some(Triangulo { esq: r.esq.into(), topo: r.topo.into(), dir: r.dir.into(), base: r.base.into() })
    }

    /// Clique do botão esquerdo em (x, y): o gerenciador decide quem recebe (só a janela da frente
    /// sob o cursor; a de trás só vem para a frente) e o conteúdo das janelas de controle é tratado
    /// aqui. `Clique::Cliente` da edição e do quintal fica para a interface.
    pub fn clicar(&mut self, recursos: &Recursos, e: &mut Estado, x: i32, y: i32, duplo: bool) -> Result<Clique, String> {
        let c = self.gerente.pressionar(x, y, duplo);
        if let Clique::Cliente(id) = c {
            if let Some(ctl) = self.controle(id) {
                self.clique_no_conteudo(recursos, e, ctl, x, y)?;
            }
        }
        Ok(c)
    }

    fn clique_no_conteudo(&mut self, recursos: &Recursos, e: &mut Estado, c: Controle, x: i32, y: i32) -> Result<(), String> {
        if c == Controle::Quintal {
            return Ok(());
        }
        let id = self.cfg.id(c);
        let janela = carregar_janela(recursos, id)?;
        let g = self.geometria_de(&janela, id);
        match c {
            Controle::Info => {
                if Self::objeto(&janela, &g, &self.cfg.info.objeto_area, false).is_some_and(|(r, _)| contem(r, x, y)) {
                    self.clique_info = Some((x, y));
                }
            }
            Controle::Mapa => {
                if Self::objeto(&janela, &g, &self.cfg.mapa.objeto_area, true).is_some_and(|(r, _)| contem(r, x, y)) {
                    self.clique_mapa = Some((x, y));
                    self.arrastando = Some(c);
                }
            }
            Controle::Historico => self.historico.clicar(&janela, &g, x, y),
            _ => self.clique_na_janela(&janela, &g, c, e, x, y),
        }
        Ok(())
    }

    /// Ponto (x, y) dentro de alguma janela que esteja por cima da de edição.
    pub fn cobre(&self, x: i32, y: i32) -> bool {
        self.gerente.janela_em(x, y).is_some_and(|id| id != ID_EDICAO)
    }
    fn clique_na_janela(&mut self, j: &Janela, g: &GeometriaControle, c: Controle, e: &mut Estado, x: i32, y: i32) {
        let cfg = self.cfg.de(c).clone();
        let k = indice(c);
        let dentro = |nome: &str| Self::objeto(j, g, nome, false).is_some_and(|(r, _)| contem(r, x, y));
        if dentro(&cfg.objeto_auto) {
            // 0x1304: Auto.
            *self.auto_mut(e, c) = 1;
        } else if dentro(&cfg.objeto_manual) {
            *self.auto_mut(e, c) = 0;
        } else if let Some(n) = cfg.objetos_conjuntos.iter().position(|o| dentro(o)) {
            // 0x1306–0x1308: guarda os níveis no conjunto atual, carrega o escolhido e vai para Manual.
            let (niveis, sets) = niveis_e_conjuntos(e, c);
            let atual = self.conjunto[k];
            sets[atual * 3..atual * 3 + 3].copy_from_slice(niveis);
            self.conjunto[k] = n;
            let novo: [u16; 3] = sets[n * 3..n * 3 + 3].try_into().unwrap_or([0; 3]);
            *niveis = novo;
            *self.auto_mut(e, c) = 0;
        } else if cfg.objetos_alternar.iter().any(|o| dentro(o)) {
            // 0x130f–0x1311: alterna porcentagem / número.
            self.porcentagem[k] = !self.porcentagem[k];
        } else if let Some(t) = Self::triangulo(j, g, &cfg) {
            if t.contem(x, y) {
                *self.auto_mut(e, c) = 0;
                self.arrastando = Some(c);
                self.arrastar(j, g, c, e, x, y);
            }
        }
    }

    fn auto_mut<'e>(&self, e: &'e mut Estado, c: Controle) -> &'e mut i16 {
        match c {
            Controle::Castas | Controle::Historico | Controle::Mapa | Controle::Info | Controle::Quintal => &mut e.controles.casta_automatica,
            Controle::Tarefas => &mut e.controles.modo_automatico,
        }
    }

    fn arrastar(&mut self, j: &Janela, g: &GeometriaControle, c: Controle, e: &mut Estado, x: i32, y: i32) {
        let cfg = self.cfg.de(c).clone();
        let Some(t) = Self::triangulo(j, g, &cfg) else { return };
        let (px, py) = t.limitar(x, y);
        let n = t.niveis(px, py);
        let k = indice(c);
        let atual = self.conjunto[k];
        let (niveis, sets) = niveis_e_conjuntos(e, c);
        *niveis = n;
        sets[atual * 3..atual * 3 + 3].copy_from_slice(&n);
        if c == Controle::Castas {
            e.controles.castas_ideais = castas_ideais(n);
        }
    }

    /// Movimento do mouse com o botão apertado: continua o arrasto no triângulo.
    pub fn mover(&mut self, recursos: &Recursos, e: &mut Estado, x: i32, y: i32) -> Result<bool, String> {
        let Some(c) = self.arrastando else { return Ok(false) };
        if c == Controle::Mapa {
            // _MapAreaEvent: arrastar no mapa move a vista de edição.
            self.clique_mapa = Some((x, y));
            return Ok(true);
        }
        let j = carregar_janela(recursos, self.cfg.de(c).janela)?;
        let g = self.geometria_de(&j, self.cfg.de(c).janela);
        self.arrastar(&j, &g, c, e, x, y);
        Ok(true)
    }

    /// Botão solto: termina o arrasto.
    pub fn soltar(&mut self) -> bool {
        self.arrastando.take().is_some()
    }

    /// Desenha a janela `c` (moldura com título, objetos do layout, botão do triângulo, níveis e
    /// barras). `ativa` = título da janela ativa.
    /// Retângulo da área do mapa (objeto `mapTileArea`, com o pixel a mais) na área cliente.
    pub fn area_do_mapa(&self, recursos: &Recursos) -> Option<Retangulo> {
        let id = self.cfg.mapa.janela;
        let j = carregar_janela(recursos, id).ok()?;
        let g = self.geometria_de(&j, id);
        Self::objeto(&j, &g, &self.cfg.mapa.objeto_area, true).map(|(r, _)| r)
    }

    /// Retângulo da área do cartão (objeto `infoArea`) na área cliente.
    pub fn area_do_info(&self, recursos: &Recursos) -> Option<Retangulo> {
        let id = self.cfg.info.janela;
        let j = carregar_janela(recursos, id).ok()?;
        let g = self.geometria_de(&j, id);
        let [dx, dy] = self.cfg.info.ajuste_area;
        Self::objeto(&j, &g, &self.cfg.info.objeto_area, false).map(|(r, _)| Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy })
    }

    pub fn desenhar(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface, e: &Estado, c: Controle) -> Result<Vec<String>, String> {
        if c == Controle::Mapa || c == Controle::Info || c == Controle::Quintal {
            // Desenhada pela interface com core/janela_mapa.rs (precisa da navegação e do buffer).
            return Ok(Vec::new());
        }
        if c == Controle::Historico {
            return self.desenhar_historico(q, recursos, interface, e);
        }
        let cfg = self.cfg.de(c);
        let k = indice(c);
        let mut janela = carregar_janela(recursos, cfg.janela)?;
        let g = self.geometria_de(&janela, cfg.janela);
        let ativa = self.ativa() == Some(c);
        // Estado dos botões Auto/Manual e do conjunto (grupos 3 e 4 do layout).
        let auto = match c {
            Controle::Castas | Controle::Historico | Controle::Mapa | Controle::Info | Controle::Quintal => e.controles.casta_automatica != 0,
            Controle::Tarefas => e.controles.modo_automatico != 0,
        };
        for o in janela.objetos.iter_mut() {
            let sel = if o.nome == cfg.objeto_auto {
                Some(auto)
            } else if o.nome == cfg.objeto_manual {
                Some(!auto)
            } else {
                // Com Auto, nenhum conjunto aparece escolhido (grupo 4 "invisível") [medido].
                cfg.objetos_conjuntos.iter().position(|n| *n == o.nome).map(|i| !auto && i == self.conjunto[k])
            };
            if let Some(s) = sel {
                if s {
                    o.flags |= 4;
                } else {
                    o.flags &= !4;
                }
            }
        }
        let jc = &self.cfg;
        desenhar_moldura_com_titulo(q, interface, jc, &g, &cfg.titulo, ativa);
        let base = janela.retangulo();
        let origem = (base.esq + g.dx, base.topo + g.dy);
        let mut pendentes = desenhar_objetos(q, recursos, interface, &janela, origem, Some(g.cliente));
        pendentes.retain(|p| !p.ends_with("Title"));
        // Botão do triângulo.
        let niveis = match c {
            Controle::Castas | Controle::Historico | Controle::Mapa | Controle::Info | Controle::Quintal => e.controles.castas,
            Controle::Tarefas => e.controles.tarefas,
        };
        // O botão bate com as capturas (j01, j02) usando o retângulo do triângulo com 1 pixel a menos
        // à direita e embaixo (em vez do pixel a mais de `_win_GetObjRect`) [medido; causa em aberto].
        let triangulo_do_botao = Self::objeto(&janela, &g, &cfg.objeto_triangulo, false).map(|(r, _)| Triangulo {
            esq: r.esq.into(),
            topo: r.topo.into(),
            dir: (r.dir - 1).into(),
            base: (r.base - 1).into(),
        });
        if let Some(t) = triangulo_do_botao {
            if let Ok(bm) = recursos.objeto(Chave { tipo: TIPO_BITMAP, id: jc.bitmap_botao }).and_then(|d| Bitmap::decodificar(&d)) {
                let (x, y) = t.ponto(niveis);
                let (x, y) = (x + i32::from(jc.ajuste_botao[0]), y + i32::from(jc.ajuste_botao[1]));
                let antes = q.recorte.replace(g.cliente);
                q.bitmap_transparente(&bm, x - bm.largura as i32 / 2, y - bm.altura as i32 / 2, jc.cor_transparente);
                q.recorte = antes;
            }
        }
        // Níveis e barras (_DrawControlLevels).
        let total = match c {
            Controle::Castas | Controle::Historico | Controle::Mapa | Controle::Info | Controle::Quintal => i32::from(e.preta.castas[0]),
            Controle::Tarefas => e.preta.modos[..3].iter().map(|&v| i32::from(v)).sum(),
        };
        let cor = |i: u8| interface.cores.get(i as usize).copied().unwrap_or(Cor { frente: 0, fundo: 15, fundo2: 15 });
        let fonte = interface.fontes.get(jc.fonte_niveis).and_then(|f| f.as_ref());
        for (i, nome) in cfg.objetos_niveis.iter().enumerate() {
            if let (Some((r, ci)), Some(f)) = (Self::objeto(&janela, &g, nome, false), fonte) {
                let c = cor(ci);
                let antes = q.recorte.replace(r);
                let t = texto_do_nivel(niveis[i], self.porcentagem[k], total);
                q.texto(Retangulo { dir: r.dir - 1, ..r }, f, t.as_bytes(), 3, c.frente, c.fundo);
                q.recorte = antes;
            }
        }
        if let Some((gr, _)) = Self::objeto(&janela, &g, &cfg.objeto_graficos, true) {
            // Largura de cada barra = (dir − esq + 1) / 5, uma a cada duas larguras. Na vertical,
            // medido (j01): a barra vai da base do objeto − 2 até essa base menos nível × (altura − 2).
            let (l, t, r, b) = (i32::from(gr.esq), i32::from(gr.topo), i32::from(gr.dir), i32::from(gr.base) - 1);
            let bw = (r - l) / 5;
            let base = b - 2;
            let h = b - t - 2;
            for (i, &nivel) in niveis.iter().enumerate() {
                let x0 = l + i as i32 * 2 * bw;
                let x1 = x0 + bw - 1;
                let topo_barra = base - (nivel as i64 * h as i64 / 0xFFFF) as i32;
                let topo_barra = if topo_barra < base {
                    let c = cfg.cores_barras[i];
                    let barra = ret(x0, topo_barra, x1, base);
                    q.preencher(barra, c, c);
                    q.contorno(barra, jc.cor_contorno_barra);
                    topo_barra
                } else {
                    b
                };
                if topo_barra - 1 > t - 1 {
                    let vazio = ret(x0, t, x1 + 1, topo_barra);
                    q.preencher(vazio, jc.cor_fundo_grafico, jc.cor_fundo_grafico);
                }
            }
        }
        Ok(pendentes)
    }
}

impl JanelasControle {
    /// A janela History: moldura, layout com os botões ligados marcados e os gráficos.
    fn desenhar_historico(&self, q: &mut Quadro, recursos: &Recursos, interface: &Interface, e: &Estado) -> Result<Vec<String>, String> {
        let id = self.cfg.historico.janela;
        let mut janela = carregar_janela(recursos, id)?;
        let g = self.geometria_de(&janela, id);
        self.historico.marcar_botoes(&mut janela);
        let ativa = self.ativa() == Some(Controle::Historico);
        desenhar_moldura_com_titulo(q, interface, &self.cfg, &g, &self.cfg.historico.titulo, ativa);
        let base = janela.retangulo();
        let mut p = desenhar_objetos(q, recursos, interface, &janela, (base.esq + g.dx, base.topo + g.dy), Some(g.cliente));
        p.retain(|n| !n.ends_with("Title"));
        let antes = q.recorte.replace(g.cliente);
        self.historico.desenhar_graficos(q, recursos, interface, &janela, &g, e);
        q.recorte = antes;
        Ok(p)
    }
}

/// Geometria de uma janela de layout com título do Windows: o objeto 0 vai para a posição da
/// tabela tipo 9 (+ faixa + ajuste) e encolhe pelo contorno (`_win_Recalc`); o encolhido é o
/// contorno externo, com o título de `altura_titulo` + 1 linha e a área cliente embaixo.
pub fn geometria_janela(j: &Janela, pos: [i16; 4], faixa: i16, altura_titulo: i16, ajuste: [i16; 2]) -> GeometriaControle {
    let ob0 = &j.objetos[0];
    let t = if ob0.tipo == crate::formats::janela::TIPO_CONTORNO { ob0.byte_28 as i16 } else { 0 };
    let r = ob0.retangulo;
    let dx = pos[0] - r.esq + ajuste[0];
    let dy = pos[1] - r.topo + faixa + ajuste[1];
    // _win_Recalc: com contorno de espessura t a janela encolhe (L+t, T+t, R−(t+1), B−(t+1)).
    let enc = Retangulo { esq: r.esq + t, topo: r.topo + t, dir: r.dir - (t + 1), base: r.base - (t + 1) };
    let h = altura_titulo;
    let cliente = Retangulo { esq: enc.esq + dx, topo: enc.topo + h + dy, dir: enc.dir + dx, base: enc.base + dy + 1 };
    let titulo = Retangulo { esq: cliente.esq, topo: cliente.topo - h - 1, dir: cliente.dir, base: cliente.topo - 1 };
    let externo = Retangulo { esq: cliente.esq - 1, topo: titulo.topo - 1, dir: cliente.dir + 1, base: cliente.base + 1 };
    GeometriaControle { externo, titulo, cliente, dx, dy }
}

/// Moldura com título do Windows (linha externa, caixa do menu de sistema, faixa do título com o
/// nome, linha embaixo) e o fundo.
pub fn desenhar_moldura_com_titulo(q: &mut Quadro, interface: &Interface, jc: &ConfigJanelas, g: &GeometriaControle, titulo: &str, ativa: bool) {
    q.preencher(g.externo, jc.cor_fundo, jc.cor_fundo);
    q.contorno(Retangulo { dir: g.externo.dir - 1, base: g.externo.base - 1, ..g.externo }, jc.cor_linha);
    desenhar_titulo(q, interface, jc, g, titulo, ativa);
}

/// Só a faixa do título (caixa do menu de sistema, faixa ativa ou inativa com o nome, linhas).
pub fn desenhar_titulo(q: &mut Quadro, interface: &Interface, jc: &ConfigJanelas, g: &GeometriaControle, titulo: &str, ativa: bool) {
    let [fundo, texto] = if ativa { jc.titulo_ativo } else { jc.titulo_inativo };
    let h = jc.altura_titulo;
    let caixa = Retangulo { dir: g.titulo.esq + h, ..g.titulo };
    q.preencher(g.titulo, fundo, fundo);
    q.preencher(caixa, jc.cor_caixa_sistema, jc.cor_caixa_sistema);
    // Traço do menu de sistema do Windows 3.1 [medido em j01]: retângulo 7 × 3 de contorno preto e
    // miolo branco, com sombra cinza escura à direita e embaixo.
    let (cx, cy) = (caixa.esq + 6, caixa.topo + 7);
    q.preencher(Retangulo { esq: cx + 1, topo: cy + 3, dir: cx + 8, base: cy + 4 }, jc.cor_sombra, jc.cor_sombra);
    q.preencher(Retangulo { esq: cx + 7, topo: cy + 1, dir: cx + 8, base: cy + 4 }, jc.cor_sombra, jc.cor_sombra);
    q.contorno(Retangulo { esq: cx, topo: cy, dir: cx + 6, base: cy + 2 }, jc.cor_linha);
    q.preencher(Retangulo { esq: cx + 1, topo: cy + 1, dir: cx + 6, base: cy + 2 }, jc.cor_traco, jc.cor_traco);
    q.preencher(Retangulo { esq: caixa.dir, dir: caixa.dir + 1, ..g.titulo }, jc.cor_linha, jc.cor_linha);
    q.preencher(Retangulo { topo: g.titulo.base, base: g.titulo.base + 1, ..g.externo }, jc.cor_linha, jc.cor_linha);
    if let Some(f) = jc.fonte_titulo.iter().find_map(|&i| interface.fontes.get(i).and_then(|f| f.as_ref())) {
        let r = Retangulo { esq: caixa.dir + 1, ..g.titulo };
        q.texto(r, f, titulo.as_bytes(), 0, texto, fundo);
    }
}

fn ret(x0: i32, y0: i32, x1: i32, y1: i32) -> Retangulo {
    Retangulo { esq: x0 as i16, topo: y0 as i16, dir: x1 as i16, base: y1 as i16 }
}

/// Os três níveis e os quatro conjuntos de três do controle.
fn niveis_e_conjuntos(e: &mut Estado, c: Controle) -> (&mut [u16; 3], &mut [u16; 12]) {
    let k = &mut e.controles;
    match c {
        Controle::Castas | Controle::Historico | Controle::Mapa | Controle::Info | Controle::Quintal => (&mut k.castas, &mut k.castas_memorizadas),
        Controle::Tarefas => (&mut k.tarefas, &mut k.tarefas_memorizadas),
    }
}
