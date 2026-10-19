//! Navegação na tela de jogo, sem janela: rolagem da janela de edição e troca de plano.
//! O executável traduz teclas e cliques em `Evento`; aqui eles viram `Acao` (pelas tabelas de
//! data/navegacao.json) e mudam o estado `Navegacao`.
//! Especificação: docs/spec/telas/jogo.md, seção "Navegação".

use serde::Deserialize;

use super::edicao::VistaEdicao;
use super::mapa::{Plano, ALTURA};
use super::tela_jogo::{vista_do_save, PosicaoVista};
use super::vida::{plano_de, FormigaAmarela};
use crate::formats::save::JogoSalvo;

/// O que o jogador pode pedir. Os nomes (em snake_case) são os usados em data/navegacao.json.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Acao {
    /// `_DoEditScroll` com `SB_LINEUP`/`SB_LINEDOWN` (seta da barra de rolagem, Ctrl+seta).
    LinhaAcima,
    LinhaAbaixo,
    LinhaEsquerda,
    LinhaDireita,
    /// `_DoEditScroll` com `SB_PAGEUP`/`SB_PAGEDOWN` (clique no trilho da barra).
    PaginaAcima,
    PaginaAbaixo,
    PaginaEsquerda,
    PaginaDireita,
    /// `_SetMapPlane(1)`, `(2)` e `(3)`.
    Superficie,
    NinhoPreto,
    NinhoVermelho,
}

impl Acao {
    /// Ações que se repetem enquanto o botão do mouse fica apertado (só as de linha, como no
    /// laço de `MAINWNDPROC` para `WM_HSCROLL`/`WM_VSCROLL`).
    pub fn repete(self) -> bool {
        matches!(self, Acao::LinhaAcima | Acao::LinhaAbaixo | Acao::LinhaEsquerda | Acao::LinhaDireita)
    }
}

/// Uma tecla ligada a uma ação. `tecla` é o nome da tecla (o do SDL: "Up", "Left", "8", "PageUp"...),
/// comparado sem diferenciar maiúsculas. Os modificadores têm de bater exatamente, como nos
/// aceleradores do Windows.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Atalho {
    pub tecla: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    pub acao: Acao,
}

/// Intervalos da repetição enquanto o mouse fica apertado numa seta da barra de rolagem.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub struct ConfigRepeticao {
    /// Espera antes da primeira repetição (ms).
    pub atraso_ms: u32,
    /// Intervalo entre repetições (ms).
    pub intervalo_ms: u32,
}

/// Conteúdo de data/navegacao.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigNavegacao {
    /// Vista inicial ao abrir um save no executável.
    pub posicao_inicial: PosicaoVista,
    pub teclas: Vec<Atalho>,
    /// Objeto da faixa (janela 34) → ação do clique.
    pub botoes_faixa: std::collections::HashMap<String, Acao>,
    pub repeticao: ConfigRepeticao,
    /// Outros botões fixos da faixa 34 (`_ProcMapRibbonEvent`): `ribToSpider` (0x2207,
    /// `_GotoSpider`) e `ribStats` (0x2209, `_ScoreDialog`). Opcional (API 0.40): sem ele, os dois
    /// não fazem nada, como antes.
    #[serde(default)]
    pub botoes_faixa_outros: std::collections::HashMap<String, BotaoFaixa>,
}

/// Botão fixo da faixa que não é de navegação por plano.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BotaoFaixa {
    /// `_GotoSpider`: a vista vai à aranha.
    Aranha,
    /// `_ScoreDialog`: a janela Status (a mesma de Window → Status, Ctrl+T).
    Status,
}

/// Evento de entrada já sem SDL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evento {
    Tecla { nome: String, ctrl: bool, shift: bool, alt: bool },
}

impl ConfigNavegacao {
    /// Ação ligada à tecla, se houver (a primeira da lista que bater).
    pub fn acao_da_tecla(&self, e: &Evento) -> Option<Acao> {
        let Evento::Tecla { nome, ctrl, shift, alt } = e;
        self.teclas
            .iter()
            .find(|a| a.tecla.eq_ignore_ascii_case(nome) && a.ctrl == *ctrl && a.shift == *shift && a.alt == *alt)
            .map(|a| a.acao)
    }
}

/// Estado da navegação: `_MapPlane`, a vista (`_MapPnt`, `_editWidth/_editHeight`) e os pontos
/// lembrados de cada plano (`_YMapPnt`, `_AMapPnt`, `_BMapPnt`, `_RMapPnt`, centros da vista).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Navegacao {
    pub plano_mapa: i16,
    pub vista: VistaEdicao,
    pub pontos: [(i32, i32); 4],
}

const PONTOS: [&str; 4] = ["YMapPnt", "AMapPnt", "BMapPnt", "RMapPnt"];

impl Navegacao {
    /// Estado ao abrir o save, com a vista conforme `posicao` (jogo.md, "Posição ao carregar").
    pub fn do_save(jogo: &JogoSalvo, posicao: PosicaoVista, largura: i32, altura: i32) -> Result<Navegacao, String> {
        let vista = vista_do_save(jogo, posicao, largura, altura)?;
        let mut pontos = [(0, 0); 4];
        for (p, s) in pontos.iter_mut().zip(PONTOS) {
            let (x, y) = jogo.par_i16(s)?;
            *p = (x as i32, y as i32);
        }
        if posicao == PosicaoVista::AoCarregar {
            // _DoLoadInitializations grava a posição da formiga amarela no ponto do plano dela.
            let me = FormigaAmarela::do_save(jogo)?;
            if let Some(p) = pontos.get_mut(me.plano as usize) {
                *p = (me.x as i32, me.y as i32);
            }
        }
        Ok(Navegacao { plano_mapa: jogo.i16("MapPlane")?, vista, pontos })
    }

    /// Maior posição da barra de rolagem em x e y (`_ResetEditScrollRange`): largura do mapa
    /// (0x40 nos planos 2 e 3, 0x80 nos outros) ou 64 linhas, menos o tamanho da vista.
    pub fn maximo(&self) -> (i32, i32) {
        let largura_mapa = if self.plano_mapa == 2 || self.plano_mapa == 3 { 0x40 } else { 0x80 };
        (largura_mapa - self.vista.largura, ALTURA as i32 - self.vista.altura)
    }

    /// Aplica a ação. Devolve true se algo mudou (vista ou plano) e a tela precisa ser redesenhada.
    pub fn aplicar(&mut self, a: Acao) -> bool {
        match a {
            Acao::LinhaAcima => self.linha(false, -1),
            Acao::LinhaAbaixo => self.linha(false, 1),
            Acao::LinhaEsquerda => self.linha(true, -1),
            Acao::LinhaDireita => self.linha(true, 1),
            Acao::PaginaAcima => self.pagina(false, -1),
            Acao::PaginaAbaixo => self.pagina(false, 1),
            Acao::PaginaEsquerda => self.pagina(true, -1),
            Acao::PaginaDireita => self.pagina(true, 1),
            Acao::Superficie => self.ir_para_plano(1),
            Acao::NinhoPreto => self.ir_para_plano(2),
            Acao::NinhoVermelho => self.ir_para_plano(3),
        }
    }

    /// `SB_LINEUP`/`SB_LINEDOWN`: anda uma célula se ainda não está no limite. Não mexe no ponto
    /// lembrado do plano.
    fn linha(&mut self, horizontal: bool, sentido: i32) -> bool {
        let (mx, my) = self.maximo();
        let (pos, max) = if horizontal { (&mut self.vista.x, mx) } else { (&mut self.vista.y, my) };
        let pode = if sentido < 0 { *pos > 0 } else { *pos < max };
        if pode {
            *pos += sentido;
        }
        pode
    }

    /// `SB_THUMBPOSITION`/`SB_THUMBTRACK` (`_DoEditScroll` casos 4 e 5): a vista vai para a posição
    /// do polegar, dentro da faixa da barra. Devolve se mudou.
    pub fn ir_para_posicao(&mut self, horizontal: bool, pos: i32) -> bool {
        let (mx, my) = self.maximo();
        let (p, max) = if horizontal { (&mut self.vista.x, mx) } else { (&mut self.vista.y, my) };
        let novo = pos.clamp(0, max.max(0));
        let mudou = *p != novo;
        *p = novo;
        mudou
    }

    /// Nova largura e altura da vista (janela de edição redimensionada, maximizada ou restaurada):
    /// a vista continua com o mesmo canto, limitada ao mapa (`_BoundEditPosition`).
    pub fn redimensionar_vista(&mut self, largura: i32, altura: i32) -> bool {
        let antes = self.vista;
        self.vista.largura = largura;
        self.vista.altura = altura;
        self.vista.limitar();
        self.vista != antes
    }

    /// `SB_PAGEUP`/`SB_PAGEDOWN`: anda o tamanho da vista e para no limite.
    fn pagina(&mut self, horizontal: bool, sentido: i32) -> bool {
        let (mx, my) = self.maximo();
        let (pos, max, passo) = if horizontal {
            (&mut self.vista.x, mx, self.vista.largura)
        } else {
            (&mut self.vista.y, my, self.vista.altura)
        };
        let antes = *pos;
        *pos = (*pos + sentido * passo).clamp(0, max.max(0));
        *pos != antes
    }

    /// `_SetMapPlane(n)` para n = 1, 2 ou 3: troca o plano e centraliza a vista no ponto lembrado
    /// dele, duas vezes (em `_SetMapPlaneLocation` e de novo em `_SetMapPlane`). Vale também quando
    /// o plano já é o atual: a vista volta ao ponto lembrado.
    pub fn ir_para_plano(&mut self, n: i16) -> bool {
        let Some(plano) = plano_de(n).filter(|_| n > 0) else { return false };
        let antes = (self.plano_mapa, self.vista);
        self.plano_mapa = n;
        self.vista.plano = plano;
        let (cx, cy) = self.pontos[n as usize];
        self.vista.centralizar(cx, cy);
        self.vista.centralizar(cx, cy);
        // _CenterEdit só limita quando anda; a troca de plano pode deixar a vista fora do mapa novo.
        self.vista.limitar();
        antes != (self.plano_mapa, self.vista)
    }
}

impl Navegacao {
    /// `_SetMapPlaneLocation(n, x, y)` para n = 1, 2 ou 3: troca o plano e centraliza a vista
    /// **uma** vez em (x, y), sem ler nem gravar o ponto lembrado (jogo.md, `_GotoMapPoint`).
    pub fn ir_para_ponto(&mut self, n: i16, x: i32, y: i32) -> bool {
        let Some(plano) = plano_de(n).filter(|_| n > 0) else { return false };
        let antes = (self.plano_mapa, self.vista);
        self.plano_mapa = n;
        self.vista.plano = plano;
        self.vista.centralizar(x, y);
        self.vista.limitar();
        antes != (self.plano_mapa, self.vista)
    }

    /// `_SetMapPlane(0)` (`1000:a5ca`, com `_SetMapPlaneLocation(0, …)`): o plano mostrado passa a 0,
    /// o do quintal. A janela de edição mostra então o mapa da superfície (os planos 0 e 1 desenham
    /// o mesmo mapa), **sem** centralizar: a vista fica com o canto que tinha (no ninho, as mesmas
    /// coordenadas, agora na superfície). `_YMapPnt` recebe o patch atual (`_CurYardPnt`).
    /// [código; bancada p01: do ninho preto, a edição passa a mostrar a superfície no mesmo canto.]
    pub fn ir_para_quintal(&mut self, patch: (i32, i32)) -> bool {
        let antes = (self.plano_mapa, self.vista);
        self.plano_mapa = 0;
        self.vista.plano = Plano::Superficie;
        self.pontos[0] = patch;
        self.vista.limitar();
        antes != (self.plano_mapa, self.vista)
    }

    /// Parte de vista de `_GotoMyAnt` (`1000:a764`): se a amarela está no plano mostrado (> 0),
    /// `_CenterEdit` na posição dela; senão, `_SetMapPlaneLocation` para o plano dela.
    pub fn ir_para_amarela(&mut self, plano: i16, x: i32, y: i32) -> bool {
        if plano == self.plano_mapa && plano > 0 {
            let antes = self.vista;
            self.vista.centralizar(x, y);
            self.vista.limitar();
            return antes != self.vista;
        }
        self.ir_para_ponto(plano, x, y)
    }
}

/// Repetição de uma ação enquanto o botão do mouse fica apertado.
#[derive(Clone, Debug, Default)]
pub struct Repeticao {
    ativa: Option<(Acao, u64)>,
}

impl Repeticao {
    /// Começa a repetir `a` (se for uma ação que repete). A primeira aplicação é do chamador.
    pub fn pressionar(&mut self, a: Acao) {
        self.ativa = a.repete().then_some((a, 0));
    }

    pub fn soltar(&mut self) {
        self.ativa = None;
    }

    /// Há uma repetição em andamento (botão ou tecla ainda apertados).
    pub fn ativa(&self) -> bool {
        self.ativa.is_some()
    }

    /// Avança o tempo e devolve quantas vezes a ação deve ser aplicada de novo.
    pub fn passar_tempo(&mut self, ms: u64, cfg: &ConfigRepeticao) -> Option<(Acao, u32)> {
        let (a, decorrido) = self.ativa.as_mut()?;
        let antes = *decorrido;
        *decorrido += ms;
        let contar = |t: u64| -> u64 {
            if t < cfg.atraso_ms as u64 {
                0
            } else {
                1 + (t - cfg.atraso_ms as u64) / cfg.intervalo_ms.max(1) as u64
            }
        };
        let n = contar(*decorrido) - contar(antes);
        (n > 0).then_some((*a, n as u32))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn nav(plano_mapa: i16, x: i32, y: i32) -> Navegacao {
        let plano = plano_de(plano_mapa).unwrap();
        Navegacao {
            plano_mapa,
            vista: VistaEdicao { plano, x, y, largura: 26, altura: 19 },
            pontos: [(0, 0), (64, 32), (34, 9), (41, 11)],
        }
    }

    #[test]
    fn linha_anda_uma_celula_e_para_no_limite() {
        let mut n = nav(2, 0, 0);
        assert!(!n.aplicar(Acao::LinhaEsquerda));
        assert!(!n.aplicar(Acao::LinhaAcima));
        assert!(n.aplicar(Acao::LinhaDireita));
        assert!(n.aplicar(Acao::LinhaAbaixo));
        assert_eq!((n.vista.x, n.vista.y), (1, 1));
        // Ninho: máximo x = 64 − 26 = 38, y = 64 − 19 = 45.
        let mut n = nav(2, 38, 45);
        assert!(!n.aplicar(Acao::LinhaDireita));
        assert!(!n.aplicar(Acao::LinhaAbaixo));
        // Superfície: máximo x = 128 − 26 = 102.
        let mut n = nav(1, 101, 0);
        assert!(n.aplicar(Acao::LinhaDireita));
        assert!(!n.aplicar(Acao::LinhaDireita));
        assert_eq!(n.vista.x, 102);
    }

    #[test]
    fn rolagem_nao_muda_o_ponto_lembrado() {
        let mut n = nav(2, 21, 0);
        n.aplicar(Acao::LinhaDireita);
        assert_eq!(n.pontos[2], (34, 9));
    }

    #[test]
    fn pagina_anda_o_tamanho_da_vista() {
        let mut n = nav(1, 10, 0);
        assert!(n.aplicar(Acao::PaginaDireita));
        assert_eq!(n.vista.x, 36);
        assert!(n.aplicar(Acao::PaginaEsquerda));
        assert!(n.aplicar(Acao::PaginaEsquerda));
        assert_eq!(n.vista.x, 0);
        assert!(n.aplicar(Acao::PaginaAbaixo));
        assert_eq!(n.vista.y, 19);
        assert!(n.aplicar(Acao::PaginaAbaixo));
        assert_eq!(n.vista.y, 38);
        // Máximo 64 − 19 = 45.
        assert!(n.aplicar(Acao::PaginaAbaixo));
        assert_eq!(n.vista.y, 45);
        assert!(!n.aplicar(Acao::PaginaAbaixo));
    }

    #[test]
    fn polegar_e_tamanho_da_vista() {
        // Ninho, vista de 26 × 19: faixa 0..38 em x e 0..45 em y.
        let mut n = nav(2, 0, 0);
        assert!(n.ir_para_posicao(true, 22));
        assert_eq!(n.vista.x, 22);
        assert!(!n.ir_para_posicao(true, 22));
        assert!(n.ir_para_posicao(false, 99));
        assert_eq!(n.vista.y, 45);
        assert!(n.ir_para_posicao(true, -5));
        assert_eq!(n.vista.x, 0);
        // Janela maximizada (39 × 21): o canto fica, limitado ao mapa.
        let mut n = nav(2, 30, 45);
        assert!(n.redimensionar_vista(39, 21));
        assert_eq!((n.vista.x, n.vista.y, n.vista.largura, n.vista.altura), (25, 43, 39, 21));
        assert_eq!(n.maximo(), (25, 43));
    }

    #[test]
    fn troca_de_plano_centraliza_no_ponto_lembrado() {
        let mut n = nav(2, 21, 0);
        assert!(n.aplicar(Acao::Superficie));
        assert_eq!(n.plano_mapa, 1);
        assert_eq!(n.vista.plano, Plano::Superficie);
        // Centro (64, 32): canto (64 − 13, 32 − 9).
        assert_eq!((n.vista.x, n.vista.y), (51, 23));
        assert!(n.aplicar(Acao::NinhoVermelho));
        assert_eq!((n.vista.plano, n.vista.x, n.vista.y), (Plano::NinhoVermelho, 28, 2));
        assert!(n.aplicar(Acao::NinhoPreto));
        assert_eq!((n.vista.x, n.vista.y), (21, 0));
        // De novo no mesmo plano, já centralizado: nada muda.
        assert!(!n.aplicar(Acao::NinhoPreto));
    }

    #[test]
    fn mesmo_plano_volta_ao_ponto_lembrado_depois_de_rolar() {
        let mut n = nav(2, 21, 0);
        n.aplicar(Acao::PaginaDireita);
        assert_eq!(n.vista.x, 38);
        assert!(n.aplicar(Acao::NinhoPreto));
        assert_eq!(n.vista.x, 21);
    }

    #[test]
    fn troca_de_plano_respeita_o_mapa_novo() {
        // Vindo da superfície em x = 100, o ninho (64 colunas) limita a vista em 38.
        let mut n = nav(1, 100, 0);
        n.pontos[2] = (63, 63);
        n.aplicar(Acao::NinhoPreto);
        assert_eq!((n.vista.x, n.vista.y), (38, 45));
    }

    #[test]
    fn segunda_centralizacao_corrige_o_passo_truncado() {
        // dx = 3, dy = 1: a primeira _CenterEdit não anda em y (fração truncada); a segunda anda.
        let mut n = nav(1, 10, 10);
        n.pontos[1] = (10 + 13 + 3, 10 + 9 + 1);
        n.aplicar(Acao::Superficie);
        assert_eq!((n.vista.x, n.vista.y), (13, 11));
    }

    fn config() -> ConfigNavegacao {
        serde_json::from_str(
            r#"{"posicao_inicial": "ao_carregar",
                "teclas": [{"tecla": "Up", "ctrl": true, "acao": "linha_acima"},
                           {"tecla": "9", "ctrl": true, "acao": "ninho_preto"}],
                "botoes_faixa": {"ribToBNest": "ninho_preto"},
                "repeticao": {"atraso_ms": 43, "intervalo_ms": 43}}"#,
        )
        .unwrap()
    }

    #[test]
    fn teclas_com_modificadores_exatos() {
        let c = config();
        let t = |nome: &str, ctrl, shift| Evento::Tecla { nome: nome.into(), ctrl, shift, alt: false };
        assert_eq!(c.acao_da_tecla(&t("up", true, false)), Some(Acao::LinhaAcima));
        assert_eq!(c.acao_da_tecla(&t("Up", false, false)), None);
        assert_eq!(c.acao_da_tecla(&t("Up", true, true)), None);
        assert_eq!(c.acao_da_tecla(&t("9", true, false)), Some(Acao::NinhoPreto));
        assert_eq!(c.botoes_faixa["ribToBNest"], Acao::NinhoPreto);
    }

    #[test]
    fn repeticao_so_nas_linhas_com_atraso_e_intervalo() {
        let cfg = config().repeticao;
        let mut r = Repeticao::default();
        r.pressionar(Acao::PaginaAbaixo);
        assert_eq!(r.passar_tempo(1000, &cfg), None);
        r.pressionar(Acao::LinhaAbaixo);
        assert!(r.ativa());
        assert_eq!(r.passar_tempo(42, &cfg), None);
        assert_eq!(r.passar_tempo(1, &cfg), Some((Acao::LinhaAbaixo, 1)));
        assert_eq!(r.passar_tempo(42, &cfg), None);
        // ~23 passos por segundo, como a bancada (qemu_ritmo_rolagem.txt).
        assert_eq!(r.passar_tempo(87, &cfg), Some((Acao::LinhaAbaixo, 3)));
        r.soltar();
        assert!(!r.ativa());
        assert_eq!(r.passar_tempo(1000, &cfg), None);
    }
}
