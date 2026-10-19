//! Gerenciador das janelas filhas da área de jogo (janela de edição, Caste, Behavior, History, Info,
//! mapa e quintal): ordem Z, janela ativa, roteamento do clique, arrasto pelo título, redimensionar,
//! maximizar/restaurar, menu de sistema e barras de rolagem — o que no original é feito pelo Windows
//! 3.1 (`CreateWindow` com os estilos de `_win_Open`, `DefWindowProc`) e por `MAINWNDPROC`.
//!
//! Especificação: docs/spec/telas/janelas.md §8. Dados: data/janelas.json ("gerenciador"). Sem SDL:
//! a interface repassa os eventos do mouse e do teclado e desenha com as funções daqui.

use serde::Deserialize;

use super::quadro::Quadro;
use crate::formats::fonte::Fonte;
use crate::formats::janela::Retangulo;

/// Alinhamento da área cliente na criação de uma janela (`CS_BYTEALIGNCLIENT`, estilo 0x1008 das
/// classes que `_InitApplication`, `1000:3dd0`, registra): na **primeira** abertura de uma janela (o `CreateWindow` de
/// `_win_Open`, `1030:ca2e`, só quando ainda não há `_win_hwnd`), o Windows põe a borda esquerda da
/// área cliente no múltiplo de `alinhamento` pixels mais próximo (um byte da memória de vídeo: 8
/// pixels em 16 cores, 1 em 256); nas aberturas seguintes a janela já existe e fica onde foi pedida.
/// Explica o Examine 3 pixels à direita e o mini mapa 1 coluna à esquerda só na primeira abertura
/// (telas/janelas.md §4.3, telas/mapa.md §7). [hipótese: o empate (resto 4) arredonda para cima]
pub fn alinhar_cliente_ao_criar(x_cliente: i32, alinhamento: i32) -> i32 {
    if alinhamento <= 1 {
        return x_cliente;
    }
    (x_cliente + alinhamento / 2).div_euclid(alinhamento) * alinhamento
}

/// Estilo da janela do Windows criada por `_win_Open` (`1030:ca2e`), a partir dos bytes 0x1C e 0x1D
/// do cabeçalho do layout e dos objetos de título (tipos 12 e 18).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Estilo {
    /// `WS_CAPTION`.
    pub titulo: bool,
    /// `WS_SYSMENU`: caixa do menu de sistema à esquerda do título.
    pub menu_sistema: bool,
    /// `WS_MINIMIZEBOX` (bit 0x10 do byte 0x1C; nenhuma janela do `WINGANT` o tem).
    pub minimizar: bool,
    /// `WS_MAXIMIZEBOX` (bit 0x01 do byte 0x1D; só a janela de edição).
    pub maximizar: bool,
    /// `WS_THICKFRAME`: moldura grossa, redimensionável (bit 0x08 do byte 0x1C).
    pub moldura_grossa: bool,
    /// `WS_HSCROLL | WS_VSCROLL` (bit 0x08 do byte 0x1C).
    pub rolagem: bool,
    /// Bit 0x40 do byte 0x1C: com uma janela assim no topo, o clique nas outras não as ativa
    /// (`WM_MOUSEACTIVATE` devolve 3).
    pub modal: bool,
}

impl Estilo {
    /// Os bits de `_win_Open`: 0x04 → `WS_CAPTION|WS_SYSMENU` (0xC8 no byte alto); 0x08 →
    /// `WS_THICKFRAME|WS_HSCROLL|WS_VSCROLL` (0x34); 0x10 → caption, sysmenu e minimizar (0xCA);
    /// 0x1D & 1 → caption, sysmenu e maximizar (0xC9); objeto tipo 12 ou 18 → caption (0xC0).
    pub fn do_layout(flags: [u8; 2], objeto_titulo: bool) -> Estilo {
        let [a, b] = flags;
        let sistema = a & 0x04 != 0 || a & 0x10 != 0 || b & 0x01 != 0;
        Estilo {
            titulo: sistema || objeto_titulo,
            menu_sistema: sistema,
            minimizar: a & 0x10 != 0,
            maximizar: b & 0x01 != 0,
            moldura_grossa: a & 0x08 != 0,
            rolagem: a & 0x08 != 0,
            modal: a & 0x40 != 0,
        }
    }

    /// Estilo do layout carregado (objeto de título = tipo 12 ou 18).
    pub fn da_janela(j: &crate::formats::janela::Janela) -> Estilo {
        Estilo::do_layout(j.flags, j.objetos.iter().any(|o| o.tipo == 12 || o.tipo == 18))
    }
}

/// Medidas da moldura de uma janela (as do Windows 3.1 a 640×480, medidas nas capturas).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct Metricas {
    /// Espessura da moldura (1 = `WS_BORDER`; 4 = `WS_THICKFRAME`, `SM_CXFRAME`).
    pub espessura: i16,
    /// Altura da faixa do título (sem a linha de baixo).
    pub altura_titulo: i16,
    /// Largura das barras de rolagem sem a linha que as separa da área cliente.
    pub barra_rolagem: i16,
}

/// Uma das barras de rolagem.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Barra {
    Horizontal,
    Vertical,
}

impl Barra {
    fn indice(self) -> usize {
        match self {
            Barra::Horizontal => 0,
            Barra::Vertical => 1,
        }
    }
}

/// Mensagem `WM_HSCROLL`/`WM_VSCROLL` que a barra manda à janela (`_DoEditScroll`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComandoRolagem {
    /// `SB_LINEUP`/`SB_LINEDOWN` (setas; repetem enquanto o botão fica apertado).
    LinhaMenos,
    LinhaMais,
    /// `SB_PAGEUP`/`SB_PAGEDOWN` (clique no trilho).
    PaginaMenos,
    PaginaMais,
    /// `SB_THUMBPOSITION`: polegar solto nessa posição.
    Posicao(i32),
    /// `SB_THUMBTRACK`: polegar sendo arrastado (o jogo só segue com AutoTrack, `_OptionStates[0]`).
    Acompanhar(i32),
}

/// Posição e máximo de uma barra (`SetScrollRange(0, max)`, `SetScrollPos`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rolagem {
    pub pos: i32,
    pub max: i32,
}

/// Parte da barra sob o ponto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParteBarra {
    SetaMenos,
    PaginaMenos,
    Polegar,
    PaginaMais,
    SetaMais,
}

/// Lados da moldura grossa pegos num redimensionamento.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lados {
    pub esq: bool,
    pub topo: bool,
    pub dir: bool,
    pub base: bool,
}

/// Parte da janela sob o ponto (o `WM_NCHITTEST` do Windows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parte {
    MenuSistema,
    Titulo,
    Maximizar,
    /// Moldura grossa (redimensionar pelos lados marcados).
    Borda(Lados),
    /// Moldura fina ou linha do título: não faz nada.
    Moldura,
    Rolagem(Barra, ParteBarra),
    /// Canto entre as duas barras.
    Canto,
    Cliente,
}

/// Retângulos de uma janela.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometria {
    pub externo: Retangulo,
    /// Dentro da moldura.
    pub interior: Retangulo,
    /// Faixa do título (a linha preta fica logo abaixo, em `titulo.base`).
    pub titulo: Option<Retangulo>,
    pub caixa_menu: Option<Retangulo>,
    pub botao_maximizar: Option<Retangulo>,
    /// Área cliente sem as barras (as linhas que separam as barras ficam em `cliente.dir` e
    /// `cliente.base`).
    pub cliente: Retangulo,
    pub barra_h: Option<Retangulo>,
    pub barra_v: Option<Retangulo>,
}

/// Uma janela aberta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JanelaFilha {
    /// Número da janela (`_win_hwnd[id]`; 0 = edição, 1 = mapa, 25 = quintal…).
    pub id: u16,
    pub externo: Retangulo,
    pub estilo: Estilo,
    pub metricas: Metricas,
    /// Maximizada: o retângulo para onde volta ao restaurar.
    pub restaurar: Option<Retangulo>,
    /// Barras de rolagem [horizontal, vertical], atualizadas pela interface.
    pub rolagem: [Rolagem; 2],
}

impl JanelaFilha {
    pub fn maximizada(&self) -> bool {
        self.restaurar.is_some()
    }

    pub fn geometria(&self) -> Geometria {
        geometria(self.externo, &self.estilo, &self.metricas)
    }
}

fn encolher(r: Retangulo, t: i16) -> Retangulo {
    Retangulo { esq: r.esq + t, topo: r.topo + t, dir: r.dir - t, base: r.base - t }
}

fn deslocar(r: Retangulo, dx: i16, dy: i16) -> Retangulo {
    Retangulo { esq: r.esq + dx, topo: r.topo + dy, dir: r.dir + dx, base: r.base + dy }
}

pub fn contem(r: Retangulo, x: i32, y: i32) -> bool {
    x >= i32::from(r.esq) && x < i32::from(r.dir) && y >= i32::from(r.topo) && y < i32::from(r.base)
}

/// Retângulos da janela com contorno `externo` (direita e base exclusivas).
pub fn geometria(externo: Retangulo, estilo: &Estilo, m: &Metricas) -> Geometria {
    let i = encolher(externo, m.espessura);
    let h = m.altura_titulo;
    let titulo = estilo.titulo.then_some(Retangulo { base: i.topo + h, ..i });
    let topo_area = titulo.map_or(i.topo, |t| t.base + 1);
    let caixa_menu = titulo.filter(|_| estilo.menu_sistema).map(|t| Retangulo { dir: t.esq + h, ..t });
    let botao_maximizar = titulo.filter(|_| estilo.maximizar).map(|t| Retangulo { esq: t.dir - h, ..t });
    let (cliente, barra_h, barra_v) = if estilo.rolagem {
        let b = m.barra_rolagem;
        let c = Retangulo { esq: i.esq, topo: topo_area, dir: i.dir - b, base: i.base - b };
        let v = Retangulo { esq: c.dir + 1, topo: c.topo, dir: i.dir, base: c.base };
        let hz = Retangulo { esq: c.esq, topo: c.base + 1, dir: c.dir, base: i.base };
        (c, Some(hz), Some(v))
    } else {
        (Retangulo { topo: topo_area, ..i }, None, None)
    };
    Geometria { externo, interior: i, titulo, caixa_menu, botao_maximizar, cliente, barra_h, barra_v }
}

/// Trilho de uma barra: posições das linhas pretas depois da seta de menos e antes da de mais.
pub fn trilho(barra: Retangulo, horizontal: bool, lado: i16) -> (i32, i32) {
    let (a, b) = if horizontal { (barra.esq, barra.dir) } else { (barra.topo, barra.base) };
    (i32::from(a + lado), i32::from(b - lado - 1))
}

/// Linha de início do polegar (a célula vai dela até ela + lado + 1), com a posição da barra.
/// `MulDiv` do Windows: arredonda [hipótese: o arredondamento; as capturas batem com os dois].
pub fn inicio_do_polegar(barra: Retangulo, horizontal: bool, lado: i16, r: Rolagem) -> Option<i32> {
    if r.max <= 0 {
        return None;
    }
    let (ini, fim) = trilho(barra, horizontal, lado);
    let curso = fim - ini - i32::from(lado) - 1;
    if curso < 0 {
        return None;
    }
    let pos = r.pos.clamp(0, r.max);
    Some(ini + (curso * pos * 2 + r.max) / (2 * r.max))
}

/// Posição da barra para o polegar começando em `inicio` (o inverso de [`inicio_do_polegar`]).
pub fn posicao_do_polegar(barra: Retangulo, horizontal: bool, lado: i16, max: i32, inicio: i32) -> i32 {
    let (ini, fim) = trilho(barra, horizontal, lado);
    let curso = fim - ini - i32::from(lado) - 1;
    if curso <= 0 || max <= 0 {
        return 0;
    }
    let d = (inicio - ini).clamp(0, curso);
    ((d * max * 2 + curso) / (2 * curso)).clamp(0, max)
}

/// Parte da barra sob a coordenada `c` (x na horizontal, y na vertical).
pub fn parte_da_barra(barra: Retangulo, horizontal: bool, lado: i16, r: Rolagem, c: i32) -> ParteBarra {
    let (ini, fim) = trilho(barra, horizontal, lado);
    if c <= ini {
        return ParteBarra::SetaMenos;
    }
    if c >= fim {
        return ParteBarra::SetaMais;
    }
    match inicio_do_polegar(barra, horizontal, lado, r) {
        Some(p) if c >= p && c <= p + i32::from(lado) + 1 => ParteBarra::Polegar,
        Some(p) if c < p => ParteBarra::PaginaMenos,
        Some(_) => ParteBarra::PaginaMais,
        // Sem polegar (faixa vazia): o trilho não faz nada [hipótese].
        None => ParteBarra::Polegar,
    }
}

/// Parte da janela `j` sob (x, y), se o ponto está nela.
pub fn parte_em(j: &JanelaFilha, x: i32, y: i32) -> Option<Parte> {
    let g = j.geometria();
    if !contem(g.externo, x, y) {
        return None;
    }
    let m = &j.metricas;
    if !contem(g.interior, x, y) {
        if !j.estilo.moldura_grossa || j.maximizada() {
            return Some(Parte::Moldura);
        }
        // Os cantos da moldura grossa vão até (espessura + título) de cada canto (as marcas desenhadas).
        let d = i32::from(m.espessura + m.altura_titulo);
        let e = g.externo;
        let (esq, dir) = (x < i32::from(g.interior.esq), x >= i32::from(g.interior.dir));
        let (topo, base) = (y < i32::from(g.interior.topo), y >= i32::from(g.interior.base));
        let lados = Lados {
            esq: esq || ((topo || base) && x < i32::from(e.esq) + d),
            dir: dir || ((topo || base) && x >= i32::from(e.dir) - d),
            topo: topo || ((esq || dir) && y < i32::from(e.topo) + d),
            base: base || ((esq || dir) && y >= i32::from(e.base) - d),
        };
        return Some(Parte::Borda(lados));
    }
    if let Some(t) = g.titulo {
        if y < i32::from(t.base) {
            if g.caixa_menu.is_some_and(|c| contem(c, x, y)) {
                return Some(Parte::MenuSistema);
            }
            if g.botao_maximizar.is_some_and(|b| contem(b, x, y)) {
                return Some(Parte::Maximizar);
            }
            return Some(Parte::Titulo);
        }
        if y == i32::from(t.base) {
            return Some(Parte::Moldura);
        }
    }
    let lado = m.barra_rolagem - 1;
    if let Some(v) = g.barra_v.filter(|_| x >= i32::from(g.cliente.dir) && y < i32::from(g.cliente.base)) {
        let _ = v;
        let p = parte_da_barra(v, false, lado, j.rolagem[Barra::Vertical.indice()], y);
        return Some(if x == i32::from(g.cliente.dir) { Parte::Moldura } else { Parte::Rolagem(Barra::Vertical, p) });
    }
    if let Some(hz) = g.barra_h.filter(|_| y >= i32::from(g.cliente.base)) {
        if x >= i32::from(g.cliente.dir) {
            return Some(Parte::Canto);
        }
        let p = parte_da_barra(hz, true, lado, j.rolagem[Barra::Horizontal.indice()], x);
        return Some(if y == i32::from(g.cliente.base) { Parte::Moldura } else { Parte::Rolagem(Barra::Horizontal, p) });
    }
    Some(Parte::Cliente)
}

/// Comando do menu de sistema (`SC_*`).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComandoSistema {
    Restaurar,
    Mover,
    Tamanho,
    Minimizar,
    Maximizar,
    Fechar,
    Proxima,
    Separador,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ItemMenuSistema {
    /// Texto com `&` antes da letra sublinhada (a tecla do item).
    #[serde(default)]
    pub texto: String,
    #[serde(default)]
    pub atalho: String,
    pub comando: ComandoSistema,
}

/// Aparência do menu de sistema [medido em w04 da bancada; a fonte é a System do Windows].
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigMenuSistema {
    pub itens: Vec<ItemMenuSistema>,
    pub largura: i16,
    pub altura_item: i16,
    pub altura_separador: i16,
    /// Linha do separador dentro da faixa dele.
    pub linha_separador: i16,
    /// Folga embaixo do último item.
    pub folga_base: i16,
    pub margem_texto: i16,
    /// Topo da célula da fonte em relação ao topo da faixa do item.
    #[serde(default)]
    pub topo_texto: i16,
    /// Distância do texto do atalho à borda esquerda.
    pub coluna_atalho: i16,
    pub fontes: Vec<usize>,
    pub cor_fundo: u8,
    pub cor_texto: u8,
    pub cor_desabilitado: u8,
    pub cor_realce: u8,
    pub cor_texto_realce: u8,
    pub cor_borda: u8,
}

/// Conteúdo de "gerenciador" em data/janelas.json.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigGerenciador {
    /// Menor contorno externo de uma janela com moldura grossa (`WM_GETMINMAXINFO`,
    /// `_AdjustWndMinMax`).
    pub tamanho_minimo: [i16; 2],
    /// Máscara XOR do contorno pontilhado do arrasto (mover, redimensionar, polegar): 15 = a cor
    /// complementar (desde 2026-10-19; antes, uma cor pintada por cima).
    pub cor_contorno_arrasto: u8,
    pub cor_seta: u8,
    pub menu_sistema: ConfigMenuSistema,
}

/// Arrasto em andamento (o botão está apertado).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrasto {
    /// Título (ou Move do menu): o contorno pontilhado segue o cursor, preso à área cliente.
    Mover { id: u16, inicio: (i32, i32), contorno: Retangulo, por_teclado: bool },
    /// Moldura grossa (ou Size do menu).
    Redimensionar { id: u16, lados: Lados, inicio: (i32, i32), original: Retangulo, contorno: Retangulo, por_teclado: bool },
    /// Polegar de uma barra: `pega` = distância do cursor ao início do polegar.
    Polegar { id: u16, barra: Barra, pega: i32, inicio: i32, pos: i32 },
    /// Botão de maximizar/restaurar apertado: age ao soltar dentro dele.
    BotaoMaximizar { id: u16 },
}

/// Menu de sistema aberto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuAberto {
    pub id: u16,
    /// Canto superior esquerdo (contorno).
    pub x: i32,
    pub y: i32,
    pub realcado: Option<usize>,
}

/// O que o clique (ou a tecla) causou; o que não é da moldura fica para a interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clique {
    /// Fora de todas as janelas (faixa ou fundo).
    Fora,
    /// Tratado pela moldura (arrasto começado, menu, nada a fazer).
    Nada,
    /// Janela de trás trazida para a frente; o clique é comido (`WM_MOUSEACTIVATE` → 2).
    Ativou(u16),
    /// Clique na área cliente da janela da frente: vai para o conteúdo dela.
    Cliente(u16),
    /// Fechar (duplo clique na caixa do menu, Close do menu, Ctrl+F4): a interface fecha e chama
    /// o que o original chama (`_YardToMap` no quintal).
    Fechar(u16),
    /// Maximizou ou restaurou (a interface recalcula a vista da janela de edição).
    Tamanho(u16),
    /// Moveu.
    Moveu(u16),
    /// Mensagem de rolagem para a janela.
    Rolar(u16, Barra, ComandoRolagem),
}

/// Ordem Z, janelas abertas, arrasto e menu de sistema.
#[derive(Clone, Debug)]
pub struct Gerenciador {
    pub cfg: ConfigGerenciador,
    /// Área cliente da janela-mãe (`_rootWnd`, embaixo da faixa): limite do cursor nos arrastos e
    /// tamanho da janela maximizada.
    pub area: Retangulo,
    /// Janelas abertas, de baixo para cima.
    pub pilha: Vec<JanelaFilha>,
    /// Janelas fechadas: reabrem onde estavam (`_win_Open` só mostra a janela escondida).
    fechadas: Vec<JanelaFilha>,
    pub arrasto: Option<Arrasto>,
    pub menu: Option<MenuAberto>,
}

impl Gerenciador {
    pub fn new(cfg: ConfigGerenciador, area: Retangulo) -> Gerenciador {
        Gerenciador { cfg, area, pilha: Vec::new(), fechadas: Vec::new(), arrasto: None, menu: None }
    }

    fn indice(&self, id: u16) -> Option<usize> {
        self.pilha.iter().position(|j| j.id == id)
    }

    pub fn aberta(&self, id: u16) -> bool {
        self.indice(id).is_some()
    }

    pub fn janela(&self, id: u16) -> Option<&JanelaFilha> {
        self.pilha.iter().find(|j| j.id == id)
    }

    pub fn janela_mut(&mut self, id: u16) -> Option<&mut JanelaFilha> {
        self.pilha.iter_mut().find(|j| j.id == id)
    }

    /// A janela ativa (a de cima).
    pub fn topo(&self) -> Option<u16> {
        self.pilha.last().map(|j| j.id)
    }

    /// Ids de baixo para cima.
    pub fn ordem(&self) -> Vec<u16> {
        self.pilha.iter().map(|j| j.id).collect()
    }

    /// `_win_Open`: abre no topo; aberta, só vem para a frente; fechada antes, reabre onde estava
    /// (`externo` só vale na primeira vez).
    pub fn abrir(&mut self, id: u16, externo: Retangulo, estilo: Estilo, metricas: Metricas) {
        if self.trazer_para_frente(id) {
            return;
        }
        let j = match self.fechadas.iter().position(|j| j.id == id) {
            Some(i) => self.fechadas.remove(i),
            None => JanelaFilha { id, externo, estilo, metricas, restaurar: None, rolagem: [Rolagem::default(); 2] },
        };
        self.pilha.push(j);
    }

    /// `BringWindowToTop`. Devolve false se a janela não está aberta.
    pub fn trazer_para_frente(&mut self, id: u16) -> bool {
        let Some(i) = self.indice(id) else { return false };
        let j = self.pilha.remove(i);
        self.pilha.push(j);
        true
    }

    /// `_win_Close`: esconde a janela (guardando posição e tamanho) e a de cima das que sobram
    /// fica ativa.
    pub fn fechar(&mut self, id: u16) -> bool {
        let Some(i) = self.indice(id) else { return false };
        let j = self.pilha.remove(i);
        self.fechadas.retain(|f| f.id != id);
        self.fechadas.push(j);
        if self.menu.is_some_and(|m| m.id == id) {
            self.menu = None;
        }
        if self.arrasto.is_some_and(|a| arrasto_id(&a) == id) {
            self.arrasto = None;
        }
        true
    }

    /// `_win_Swap(de, para)`: a janela `para` assume o lugar, o retângulo e a posição na ordem de
    /// `de` (mapa ↔ quintal). Sem `de` aberta, não faz nada.
    pub fn trocar(&mut self, de: u16, para: u16, estilo: Estilo, metricas: Metricas) -> bool {
        let Some(i) = self.indice(de) else { return false };
        self.fechadas.retain(|f| f.id != para);
        let j = &mut self.pilha[i];
        j.id = para;
        j.estilo = estilo;
        j.metricas = metricas;
        true
    }

    /// Ctrl+F6 / Next (`MAINWNDPROC`, comando 0xFDA2 e `SC_NEXTWINDOW`): a janela de baixo vem
    /// para a frente.
    pub fn proxima(&mut self) -> Option<u16> {
        if self.pilha.len() < 2 {
            return self.topo();
        }
        let j = self.pilha.remove(0);
        self.pilha.push(j);
        self.topo()
    }

    /// Janela da frente que contém o ponto.
    pub fn janela_em(&self, x: i32, y: i32) -> Option<u16> {
        self.pilha.iter().rev().find(|j| contem(j.externo, x, y)).map(|j| j.id)
    }

    /// O ponto cai na área cliente da janela `id` e nenhuma outra por cima o cobre.
    pub fn visivel_em(&self, id: u16, x: i32, y: i32) -> bool {
        self.janela_em(x, y) == Some(id) && self.janela(id).is_some_and(|j| parte_em(j, x, y) == Some(Parte::Cliente))
    }

    pub fn definir_rolagem(&mut self, id: u16, h: Rolagem, v: Rolagem) {
        if let Some(j) = self.janela_mut(id) {
            j.rolagem = [h, v];
        }
    }

    /// `SC_MAXIMIZE`: ocupa a área cliente da mãe com a moldura de fora (`ptMaxPosition` =
    /// −moldura).
    pub fn maximizar(&mut self, id: u16) -> bool {
        let area = self.area;
        let Some(j) = self.janela_mut(id) else { return false };
        if j.maximizada() || !j.estilo.maximizar {
            return false;
        }
        j.restaurar = Some(j.externo);
        j.externo = encolher(area, -j.metricas.espessura);
        true
    }

    /// `SC_RESTORE`.
    pub fn restaurar(&mut self, id: u16) -> bool {
        let Some(j) = self.janela_mut(id) else { return false };
        let Some(r) = j.restaurar.take() else { return false };
        j.externo = r;
        true
    }

    /// Botão de maximizar/restaurar (e Ctrl+Z na janela de edição, comando 0xFDA5).
    pub fn alternar_maximizar(&mut self, id: u16) -> bool {
        if self.janela(id).is_some_and(|j| j.maximizada()) {
            self.restaurar(id)
        } else {
            self.maximizar(id)
        }
    }

    /// Limita o retângulo de uma janela de moldura grossa: mínimo de `tamanho_minimo`, máximo da área
    /// cliente da mãe com a moldura (ou o tamanho atual, se maior), mexendo só nos lados pegos.
    fn limitar_tamanho(&self, j: &JanelaFilha, mut r: Retangulo, lados: Lados) -> Retangulo {
        let [wmin, hmin] = self.cfg.tamanho_minimo;
        let t = j.metricas.espessura;
        let wmax = (self.area.dir - self.area.esq + 2 * t).max(j.externo.dir - j.externo.esq);
        let hmax = (self.area.base - self.area.topo + 2 * t).max(j.externo.base - j.externo.topo);
        let w = (r.dir - r.esq).clamp(wmin, wmax.max(wmin));
        let h = (r.base - r.topo).clamp(hmin, hmax.max(hmin));
        if lados.esq {
            r.esq = r.dir - w;
        } else {
            r.dir = r.esq + w;
        }
        if lados.topo {
            r.topo = r.base - h;
        } else {
            r.base = r.topo + h;
        }
        r
    }

    fn cursor_na_area(&self, x: i32, y: i32) -> (i32, i32) {
        let a = self.area;
        (x.clamp(i32::from(a.esq), i32::from(a.dir) - 1), y.clamp(i32::from(a.topo), i32::from(a.base) - 1))
    }

    /// Botão esquerdo apertado em (x, y). `duplo`: segundo clique de um duplo clique.
    pub fn pressionar(&mut self, x: i32, y: i32, duplo: bool) -> Clique {
        if self.menu.is_some() {
            return self.clique_com_menu(x, y, duplo);
        }
        if let Some(a) = self.arrasto {
            // Mover/Size pelo teclado: o clique termina.
            if matches!(a, Arrasto::Mover { por_teclado: true, .. } | Arrasto::Redimensionar { por_teclado: true, .. }) {
                return self.soltar(x, y);
            }
        }
        let Some(id) = self.janela_em(x, y) else { return Clique::Fora };
        if self.topo() != Some(id) {
            // MAINWNDPROC, WM_MOUSEACTIVATE: BringWindowToTop e o clique é comido (MA_ACTIVATEANDEAT);
            // com uma janela de estilo "modal" no topo, a de trás não vem (MA_NOACTIVATE) [código].
            if self.pilha.last().is_some_and(|t| t.estilo.modal) {
                return Clique::Nada;
            }
            self.trazer_para_frente(id);
            return Clique::Ativou(id);
        }
        let Some(j) = self.janela(id).copied() else { return Clique::Fora };
        match parte_em(&j, x, y).unwrap_or(Parte::Moldura) {
            Parte::Cliente => Clique::Cliente(id),
            Parte::MenuSistema => {
                if duplo {
                    // Duplo clique na caixa do menu de sistema: SC_CLOSE.
                    return Clique::Fechar(id);
                }
                self.abrir_menu(id);
                Clique::Nada
            }
            Parte::Maximizar => {
                self.arrasto = Some(Arrasto::BotaoMaximizar { id });
                Clique::Nada
            }
            Parte::Titulo => {
                if duplo && j.estilo.maximizar {
                    // Duplo clique no título de janela com botão de maximizar [hipótese: o padrão do
                    // Windows 3.1].
                    self.alternar_maximizar(id);
                    return Clique::Tamanho(id);
                }
                if !j.maximizada() {
                    self.arrasto = Some(Arrasto::Mover { id, inicio: (x, y), contorno: j.externo, por_teclado: false });
                }
                Clique::Nada
            }
            Parte::Borda(lados) => {
                self.arrasto = Some(Arrasto::Redimensionar {
                    id,
                    lados,
                    inicio: (x, y),
                    original: j.externo,
                    contorno: j.externo,
                    por_teclado: false,
                });
                Clique::Nada
            }
            Parte::Rolagem(barra, p) => {
                let cmd = match p {
                    ParteBarra::SetaMenos => ComandoRolagem::LinhaMenos,
                    ParteBarra::SetaMais => ComandoRolagem::LinhaMais,
                    ParteBarra::PaginaMenos => ComandoRolagem::PaginaMenos,
                    ParteBarra::PaginaMais => ComandoRolagem::PaginaMais,
                    ParteBarra::Polegar => {
                        let g = j.geometria();
                        let horizontal = barra == Barra::Horizontal;
                        let r = j.rolagem[barra.indice()];
                        let lado = j.metricas.barra_rolagem - 1;
                        let caixa = if horizontal { g.barra_h } else { g.barra_v };
                        if let Some(inicio) = caixa.and_then(|c| inicio_do_polegar(c, horizontal, lado, r)) {
                            let c = if horizontal { x } else { y };
                            self.arrasto = Some(Arrasto::Polegar { id, barra, pega: c - inicio, inicio, pos: r.pos });
                        }
                        return Clique::Nada;
                    }
                };
                Clique::Rolar(id, barra, cmd)
            }
            Parte::Canto | Parte::Moldura => Clique::Nada,
        }
    }

    /// Mouse andando (com ou sem botão): arrastos e realce do menu. Devolve a mensagem de rolagem
    /// do polegar arrastado (`SB_THUMBTRACK`), se mudou, e se é preciso redesenhar.
    pub fn mover_mouse(&mut self, x: i32, y: i32) -> (bool, Option<Clique>) {
        if let Some(m) = self.menu {
            let novo = self.item_do_menu_em(&m, x, y).or(m.realcado.filter(|_| !self.contem_menu(&m, x, y)));
            let mudou = novo != m.realcado;
            if let Some(mm) = self.menu.as_mut() {
                mm.realcado = novo;
            }
            return (mudou, None);
        }
        let Some(a) = self.arrasto else { return (false, None) };
        match a {
            Arrasto::Mover { id, inicio, por_teclado, .. } => {
                let Some(j) = self.janela(id) else { return (false, None) };
                let (cx, cy) = self.cursor_na_area(x, y);
                let (ix, iy) = inicio;
                let contorno = deslocar(j.externo, (cx - ix) as i16, (cy - iy) as i16);
                self.arrasto = Some(Arrasto::Mover { id, inicio, contorno, por_teclado });
                (true, None)
            }
            Arrasto::Redimensionar { id, lados, inicio, original, por_teclado, .. } => {
                let Some(j) = self.janela(id).copied() else { return (false, None) };
                let (cx, cy) = self.cursor_na_area(x, y);
                let (dx, dy) = ((cx - inicio.0) as i16, (cy - inicio.1) as i16);
                let mut r = original;
                if lados.esq {
                    r.esq += dx;
                }
                if lados.dir {
                    r.dir += dx;
                }
                if lados.topo {
                    r.topo += dy;
                }
                if lados.base {
                    r.base += dy;
                }
                let contorno = self.limitar_tamanho(&j, r, lados);
                self.arrasto = Some(Arrasto::Redimensionar { id, lados, inicio, original, contorno, por_teclado });
                (true, None)
            }
            Arrasto::Polegar { id, barra, pega, pos, .. } => {
                let Some(j) = self.janela(id).copied() else { return (false, None) };
                let g = j.geometria();
                let horizontal = barra == Barra::Horizontal;
                let Some(caixa) = (if horizontal { g.barra_h } else { g.barra_v }) else { return (false, None) };
                let lado = j.metricas.barra_rolagem - 1;
                let (ini, fim) = trilho(caixa, horizontal, lado);
                let c = if horizontal { x } else { y };
                let inicio = (c - pega).clamp(ini, (fim - i32::from(lado) - 1).max(ini));
                let max = j.rolagem[barra.indice()].max;
                let novo = posicao_do_polegar(caixa, horizontal, lado, max, inicio);
                self.arrasto = Some(Arrasto::Polegar { id, barra, pega, inicio, pos: novo });
                let msg = (novo != pos).then_some(Clique::Rolar(id, barra, ComandoRolagem::Acompanhar(novo)));
                (true, msg)
            }
            Arrasto::BotaoMaximizar { .. } => (false, None),
        }
    }

    /// Botão solto: termina o arrasto (e a escolha no menu de sistema, se o botão foi solto num item).
    pub fn soltar(&mut self, x: i32, y: i32) -> Clique {
        if let Some(m) = self.menu {
            if let Some(i) = self.item_do_menu_em(&m, x, y) {
                return self.executar_item(m.id, i);
            }
            return Clique::Nada;
        }
        let Some(a) = self.arrasto.take() else { return Clique::Nada };
        match a {
            Arrasto::Mover { id, contorno, .. } => {
                let Some(j) = self.janela_mut(id) else { return Clique::Nada };
                if j.externo == contorno {
                    return Clique::Nada;
                }
                j.externo = contorno;
                Clique::Moveu(id)
            }
            Arrasto::Redimensionar { id, contorno, .. } => {
                let Some(j) = self.janela_mut(id) else { return Clique::Nada };
                if j.externo == contorno {
                    return Clique::Nada;
                }
                j.externo = contorno;
                Clique::Tamanho(id)
            }
            Arrasto::Polegar { id, barra, pos, .. } => Clique::Rolar(id, barra, ComandoRolagem::Posicao(pos)),
            Arrasto::BotaoMaximizar { id } => {
                let dentro = self
                    .janela(id)
                    .and_then(|j| j.geometria().botao_maximizar)
                    .is_some_and(|b| contem(b, x, y));
                if dentro && self.alternar_maximizar(id) {
                    Clique::Tamanho(id)
                } else {
                    Clique::Nada
                }
            }
        }
    }

    /// Ctrl+Espaço (0xFDA1, `SC_KEYMENU` na janela de cima) ou clique na caixa: abre o menu de
    /// sistema embaixo da caixa, com o primeiro item realçado [medido em w04].
    pub fn abrir_menu(&mut self, id: u16) -> bool {
        let Some(j) = self.janela(id) else { return false };
        let g = j.geometria();
        let Some(c) = g.caixa_menu else { return false };
        self.menu = Some(MenuAberto { id, x: i32::from(g.externo.esq), y: i32::from(c.base), realcado: Some(0) });
        true
    }

    /// Altura do menu e posição (relativa ao topo do contorno) de cada item.
    fn layout_menu(&self) -> (Vec<(i32, i32)>, i32) {
        let c = &self.cfg.menu_sistema;
        let mut y = 1;
        let mut itens = Vec::new();
        for it in &c.itens {
            let h = i32::from(if it.comando == ComandoSistema::Separador { c.altura_separador } else { c.altura_item });
            itens.push((y, h));
            y += h;
        }
        (itens, y + i32::from(c.folga_base) + 1)
    }

    /// Retângulo do menu aberto.
    pub fn retangulo_do_menu(&self, m: &MenuAberto) -> Retangulo {
        let (_, h) = self.layout_menu();
        Retangulo { esq: m.x as i16, topo: m.y as i16, dir: (m.x + i32::from(self.cfg.menu_sistema.largura)) as i16, base: (m.y + h) as i16 }
    }

    fn contem_menu(&self, m: &MenuAberto, x: i32, y: i32) -> bool {
        contem(self.retangulo_do_menu(m), x, y)
    }

    fn item_do_menu_em(&self, m: &MenuAberto, x: i32, y: i32) -> Option<usize> {
        if !self.contem_menu(m, x, y) {
            return None;
        }
        let (itens, _) = self.layout_menu();
        let dy = y - m.y;
        itens
            .iter()
            .position(|&(t, h)| dy >= t && dy < t + h)
            .filter(|&i| self.cfg.menu_sistema.itens[i].comando != ComandoSistema::Separador)
    }

    /// Item habilitado para a janela (o Windows acinzenta os que o estilo e o estado não permitem).
    pub fn habilitado(&self, id: u16, c: ComandoSistema) -> bool {
        let Some(j) = self.janela(id) else { return false };
        let max = j.maximizada();
        match c {
            ComandoSistema::Restaurar => max,
            ComandoSistema::Mover => !max,
            ComandoSistema::Tamanho => j.estilo.moldura_grossa && !max,
            ComandoSistema::Minimizar => j.estilo.minimizar,
            ComandoSistema::Maximizar => j.estilo.maximizar && !max,
            ComandoSistema::Fechar | ComandoSistema::Proxima => true,
            ComandoSistema::Separador => false,
        }
    }

    fn clique_com_menu(&mut self, x: i32, y: i32, duplo: bool) -> Clique {
        let Some(m) = self.menu else { return Clique::Nada };
        if let Some(i) = self.item_do_menu_em(&m, x, y) {
            if let Some(mm) = self.menu.as_mut() {
                mm.realcado = Some(i);
            }
            return Clique::Nada;
        }
        self.menu = None;
        let na_caixa = self.janela(m.id).and_then(|j| j.geometria().caixa_menu).is_some_and(|c| contem(c, x, y));
        if na_caixa && duplo {
            return Clique::Fechar(m.id);
        }
        Clique::Nada
    }

    /// Executa o item `i` do menu de sistema da janela `id`.
    fn executar_item(&mut self, id: u16, i: usize) -> Clique {
        let Some(item) = self.cfg.menu_sistema.itens.get(i) else { return Clique::Nada };
        let c = item.comando;
        if !self.habilitado(id, c) {
            // Item acinzentado: o menu fica aberto.
            return Clique::Nada;
        }
        self.menu = None;
        self.comando(id, c)
    }

    /// `WM_SYSCOMMAND` na janela `id`.
    pub fn comando(&mut self, id: u16, c: ComandoSistema) -> Clique {
        match c {
            ComandoSistema::Restaurar => {
                if self.restaurar(id) {
                    Clique::Tamanho(id)
                } else {
                    Clique::Nada
                }
            }
            ComandoSistema::Maximizar => {
                if self.maximizar(id) {
                    Clique::Tamanho(id)
                } else {
                    Clique::Nada
                }
            }
            ComandoSistema::Mover => {
                let Some(j) = self.janela(id).copied() else { return Clique::Nada };
                let t = j.geometria().titulo.unwrap_or(j.externo);
                let centro = (i32::from(t.esq + t.dir) / 2, i32::from(t.topo + t.base) / 2);
                self.arrasto = Some(Arrasto::Mover { id, inicio: centro, contorno: j.externo, por_teclado: true });
                Clique::Nada
            }
            ComandoSistema::Tamanho => {
                let Some(j) = self.janela(id).copied() else { return Clique::Nada };
                let canto = (i32::from(j.externo.dir) - 1, i32::from(j.externo.base) - 1);
                let lados = Lados { dir: true, base: true, ..Default::default() };
                self.arrasto = Some(Arrasto::Redimensionar {
                    id,
                    lados,
                    inicio: canto,
                    original: j.externo,
                    contorno: j.externo,
                    por_teclado: true,
                });
                Clique::Nada
            }
            ComandoSistema::Fechar => Clique::Fechar(id),
            ComandoSistema::Proxima => match self.proxima() {
                Some(t) => Clique::Ativou(t),
                None => Clique::Nada,
            },
            ComandoSistema::Minimizar | ComandoSistema::Separador => Clique::Nada,
        }
    }

    /// Tecla com o menu de sistema aberto (ou num arrasto pelo teclado). Devolve None se a tecla não
    /// é do gerenciador.
    pub fn tecla(&mut self, nome: &str) -> Option<Clique> {
        if let Some(m) = self.menu {
            let n = self.cfg.menu_sistema.itens.len();
            let validos: Vec<usize> = (0..n).filter(|&i| self.cfg.menu_sistema.itens[i].comando != ComandoSistema::Separador).collect();
            let atual = m.realcado.and_then(|r| validos.iter().position(|&v| v == r));
            let r = match nome.to_ascii_lowercase().as_str() {
                "escape" => {
                    self.menu = None;
                    Clique::Nada
                }
                "up" | "down" => {
                    let k = validos.len();
                    if k > 0 {
                        let novo = match (atual, nome.eq_ignore_ascii_case("up")) {
                            (Some(a), true) => (a + k - 1) % k,
                            (Some(a), false) => (a + 1) % k,
                            (None, _) => 0,
                        };
                        if let Some(mm) = self.menu.as_mut() {
                            mm.realcado = Some(validos[novo]);
                        }
                    }
                    Clique::Nada
                }
                "return" | "enter" => match m.realcado {
                    Some(i) => self.executar_item(m.id, i),
                    None => Clique::Nada,
                },
                letra => {
                    let achado = self.cfg.menu_sistema.itens.iter().position(|it| mnemonico(&it.texto).is_some_and(|c| letra.len() == 1 && letra.starts_with(c)));
                    match achado {
                        Some(i) => self.executar_item(m.id, i),
                        None => Clique::Nada,
                    }
                }
            };
            return Some(r);
        }
        if let Some(a) = self.arrasto {
            let teclado = matches!(a, Arrasto::Mover { por_teclado: true, .. } | Arrasto::Redimensionar { por_teclado: true, .. });
            if teclado {
                match nome.to_ascii_lowercase().as_str() {
                    "escape" => {
                        self.arrasto = None;
                        return Some(Clique::Nada);
                    }
                    "return" | "enter" => {
                        let (x, y) = match a {
                            Arrasto::Mover { contorno, .. } | Arrasto::Redimensionar { contorno, .. } => {
                                (i32::from(contorno.esq), i32::from(contorno.topo))
                            }
                            _ => (0, 0),
                        };
                        return Some(self.soltar(x, y));
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Desenha o contorno pontilhado do arrasto (mover, redimensionar; o polegar arrastado sem
    /// AutoTrack) e o menu de sistema aberto. `acompanhando`: o polegar segue a vista (AutoTrack),
    /// e então quem o desenha é a barra.
    pub fn desenhar_por_cima(&self, q: &mut Quadro, fonte: Option<&Fonte>, acompanhando: bool) {
        let cor = self.cfg.cor_contorno_arrasto;
        match self.arrasto {
            Some(Arrasto::Mover { contorno, .. }) | Some(Arrasto::Redimensionar { contorno, .. }) => {
                contorno_pontilhado(q, Retangulo { dir: contorno.dir - 1, base: contorno.base - 1, ..contorno }, cor);
            }
            Some(Arrasto::Polegar { id, barra, inicio, .. }) if !acompanhando => {
                if let Some(j) = self.janela(id) {
                    let g = j.geometria();
                    let lado = j.metricas.barra_rolagem - 1;
                    let horizontal = barra == Barra::Horizontal;
                    if let Some(c) = if horizontal { g.barra_h } else { g.barra_v } {
                        let p = inicio as i16;
                        let r = if horizontal {
                            Retangulo { esq: p, topo: c.topo - 1, dir: p + lado + 1, base: c.base }
                        } else {
                            Retangulo { esq: c.esq - 1, topo: p, dir: c.dir, base: p + lado + 1 }
                        };
                        contorno_pontilhado(q, r, cor);
                    }
                }
            }
            _ => {}
        }
        if let Some(m) = self.menu {
            // A caixa do menu de sistema fica invertida com o menu aberto (w04).
            if let Some(c) = self.janela(m.id).and_then(|j| j.geometria().caixa_menu) {
                inverter(q, c);
            }
            self.desenhar_menu(q, &m, fonte);
        }
    }

    fn desenhar_menu(&self, q: &mut Quadro, m: &MenuAberto, fonte: Option<&Fonte>) {
        let c = &self.cfg.menu_sistema;
        let r = self.retangulo_do_menu(m);
        q.preencher(r, c.cor_fundo, c.cor_fundo);
        q.contorno(Retangulo { dir: r.dir - 1, base: r.base - 1, ..r }, c.cor_borda);
        let (itens, _) = self.layout_menu();
        for (i, (it, &(t, h))) in c.itens.iter().zip(itens.iter()).enumerate() {
            let topo = r.topo + t as i16;
            if it.comando == ComandoSistema::Separador {
                let y = topo + c.linha_separador;
                q.preencher(Retangulo { esq: r.esq, topo: y, dir: r.dir, base: y + 1 }, c.cor_borda, c.cor_borda);
                continue;
            }
            let faixa = Retangulo { esq: r.esq + 1, topo, dir: r.dir - 1, base: topo + h as i16 };
            let realcado = m.realcado == Some(i);
            let fundo = if realcado { c.cor_realce } else { c.cor_fundo };
            if realcado {
                q.preencher(faixa, fundo, fundo);
            }
            let habilitado = self.habilitado(m.id, it.comando);
            let frente = if realcado {
                c.cor_texto_realce
            } else if !habilitado {
                c.cor_desabilitado
            } else {
                c.cor_texto
            };
            let Some(f) = fonte else { continue };
            let antes = q.recorte.replace(faixa);
            let y0 = topo + c.topo_texto;
            // Win 3.1: o texto do item alinhado à esquerda na margem e o do atalho (depois do tab)
            // alinhado à esquerda numa coluna comum [medido em w04]. Acinzentado e realçado: o
            // `GrayString` (pixels alternados na cor do texto realçado; w04 e s04).
            let escrever = if realcado && !habilitado { texto_acinzentado } else { texto_a_esquerda };
            escrever(q, f, &it.texto, r.esq + c.margem_texto, y0, frente, fundo);
            if !it.atalho.is_empty() {
                escrever(q, f, &it.atalho, r.esq + c.coluna_atalho, y0, frente, fundo);
            }
            q.recorte = antes;
        }
    }
}

/// Texto com a borda esquerda em `x` (o `Quadro::texto` com alinhamento 0 centraliza no retângulo;
/// um retângulo da largura exata do texto deixa-o em `x`). `&` sublinha a letra seguinte, na linha
/// de base + 2 e na largura da célula da letra [medido em w04: o "C" de Close e o "t" de Next].
pub(crate) fn texto_a_esquerda(q: &mut Quadro, f: &Fonte, texto: &str, x: i16, topo: i16, frente: u8, fundo: u8) {
    let bytes: Vec<u8> = texto.bytes().filter(|&b| b != b'&').collect();
    let w = f.largura_texto(&bytes) as i16;
    let alt = f.altura() as i16;
    q.texto(Retangulo { esq: x, topo, dir: x + w, base: topo + alt }, f, &bytes, 0, frente, fundo);
    if let Some(p) = texto.find('&') {
        let antes = f.largura_texto(&texto.as_bytes()[..p]) as i32;
        let letra = texto.as_bytes().get(p + 1).copied().unwrap_or(b' ');
        let lw = f.largura_texto(&[letra]) as i32;
        let yb = i32::from(topo) + f.ascendente() as i32 + 1;
        let x0 = i32::from(x) + antes;
        q.linha(x0, yb, x0 + lw - 1, yb, frente);
    }
}

/// Inverte as cores do retângulo (`DSTINVERT` no VGA de 16 cores: o índice complementar).
pub fn inverter(q: &mut Quadro, r: Retangulo) {
    for y in r.topo.max(0)..r.base {
        for x in r.esq.max(0)..r.dir {
            if (x as usize) < q.largura {
                let i = y as usize * q.largura + x as usize;
                if let Some(p) = q.pixels.get_mut(i) {
                    *p = super::moldura_principal::complementar(*p);
                }
            }
        }
    }
}

/// Texto acinzentado num item realçado: o `GrayString` do Windows 3.1 desenha só os pixels do glifo
/// (e do sublinhado) com x + y par, na cor `frente`; os outros ficam no fundo [medido em s04, no menu
/// de sistema da janela principal, e em w04, no Restore das janelas filhas].
pub(crate) fn texto_acinzentado(q: &mut Quadro, f: &Fonte, texto: &str, x: i16, topo: i16, frente: u8, fundo: u8) {
    // Marca os pixels do texto com uma cor fora da paleta e depois troca pelo padrão.
    const MARCA: u8 = 254;
    texto_a_esquerda(q, f, texto, x, topo, MARCA, fundo);
    let bytes: Vec<u8> = texto.bytes().filter(|&b| b != b'&').collect();
    let w = f.largura_texto(&bytes) as i32;
    let h = f.altura() as i32 + 2;
    for y in i32::from(topo)..i32::from(topo) + h {
        for xx in i32::from(x)..i32::from(x) + w {
            if xx < 0 || y < 0 || xx as usize >= q.largura {
                continue;
            }
            let i = y as usize * q.largura + xx as usize;
            if q.pixels.get(i) == Some(&MARCA) {
                q.pixels[i] = if (xx + y) % 2 == 0 { frente } else { fundo };
            }
        }
    }
}

fn arrasto_id(a: &Arrasto) -> u16 {
    match *a {
        Arrasto::Mover { id, .. } | Arrasto::Redimensionar { id, .. } | Arrasto::Polegar { id, .. } | Arrasto::BotaoMaximizar { id } => id,
    }
}

/// Letra depois do `&` (minúscula).
pub(crate) fn mnemonico(texto: &str) -> Option<char> {
    let i = texto.find('&')?;
    texto[i + 1..].chars().next().map(|c| c.to_ascii_lowercase())
}

/// Contorno de 1 pixel com um ponto sim, outro não: o retângulo de arrasto do Windows 3.1, desenhado
/// em XOR com o pincel cinza (meio-tom): nos pontos com x + y ímpar o índice da cor de baixo vira
/// `índice XOR mascara` (15 = a cor complementar na paleta de 16 cores: preto → branco, azul-escuro
/// → amarelo); os outros ficam como estão [medido em w02: 85 de 170 pontos da borda invertidos, todos
/// com x + y ímpar na tela, e a tela começa numa linha par da área cliente].
pub fn contorno_pontilhado(q: &mut Quadro, r: Retangulo, mascara: u8) {
    let (e, t, d, b) = (i32::from(r.esq), i32::from(r.topo), i32::from(r.dir), i32::from(r.base));
    let mut inverter = |x: i32, y: i32| {
        if (x + y).rem_euclid(2) == 1 && x >= 0 && y >= 0 && (x as usize) < q.largura && (y as usize) < q.altura {
            let c = q.pixels[y as usize * q.largura + x as usize];
            if c < 16 {
                q.ponto(x, y, (c ^ mascara) & 15);
            }
        }
    };
    for x in e..=d {
        inverter(x, t);
        if b != t {
            inverter(x, b);
        }
    }
    for y in t + 1..b {
        inverter(e, y);
        if d != e {
            inverter(d, y);
        }
    }
}

/// Direção da seta desenhada num botão.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seta {
    Cima,
    Baixo,
    Esquerda,
    Direita,
}

/// Seta do Windows 3.1 numa célula de botão: triângulo de 4 linhas (1, 3, 5, 7) e haste de 3 × 3,
/// centrada na célula; a ponta fica 5 pixels depois da linha preta da célula e a haste termina 11
/// pixels depois [medido em j00: setas das barras da janela de edição].
/// `celula`: interior do botão (sem as linhas pretas).
pub fn desenhar_seta(q: &mut Quadro, celula: Retangulo, s: Seta, cor: u8) {
    // Linha preta antes da célula = celula.esq − 1 (ou topo − 1); o centro fica 8 depois dela.
    let cx = i32::from(celula.esq) - 1 + 8;
    let cy = i32::from(celula.topo) - 1 + 8;
    let ax = i32::from(celula.esq) - 1;
    let ay = i32::from(celula.topo) - 1;
    for k in 0..4 {
        for d in -k..=k {
            match s {
                Seta::Cima => q.ponto(cx + d, ay + 5 + k, cor),
                Seta::Baixo => q.ponto(cx + d, ay + 11 - k, cor),
                Seta::Esquerda => q.ponto(ax + 5 + k, cy + d, cor),
                Seta::Direita => q.ponto(ax + 11 - k, cy + d, cor),
            }
        }
    }
    for a in 0..3 {
        for d in -1..=1 {
            match s {
                Seta::Cima => q.ponto(cx + d, ay + 9 + a, cor),
                Seta::Baixo => q.ponto(cx + d, ay + 5 + a, cor),
                Seta::Esquerda => q.ponto(ax + 9 + a, cy + d, cor),
                Seta::Direita => q.ponto(ax + 5 + a, cy + d, cor),
            }
        }
    }
}

/// Glifo do botão de restaurar (janela maximizada): triângulo para cima nas linhas 4–7 e para
/// baixo nas linhas 10–13 do botão, centrados [medido em w13].
pub fn desenhar_glifo_restaurar(q: &mut Quadro, botao: Retangulo, cor: u8) {
    let cx = i32::from(botao.esq) + 8;
    let y0 = i32::from(botao.topo);
    for k in 0..4 {
        for d in -k..=k {
            q.ponto(cx + d, y0 + 4 + k, cor);
            q.ponto(cx + d, y0 + 13 - k, cor);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn r(esq: i16, topo: i16, dir: i16, base: i16) -> Retangulo {
        Retangulo { esq, topo, dir, base }
    }

    const FINA: Metricas = Metricas { espessura: 1, altura_titulo: 18, barra_rolagem: 16 };
    const GROSSA: Metricas = Metricas { espessura: 4, altura_titulo: 18, barra_rolagem: 16 };

    fn controle() -> Estilo {
        Estilo::do_layout([0x06, 0x04], true)
    }

    fn edicao() -> Estilo {
        Estilo::do_layout([0x0e, 0x05], true)
    }

    fn cfg() -> ConfigGerenciador {
        ConfigGerenciador {
            tamanho_minimo: [317, 150],
            cor_contorno_arrasto: 15,
            cor_seta: 0,
            menu_sistema: ConfigMenuSistema {
                itens: [
                    ("&Restore", ComandoSistema::Restaurar),
                    ("&Move", ComandoSistema::Mover),
                    ("&Size", ComandoSistema::Tamanho),
                    ("Mi&nimize", ComandoSistema::Minimizar),
                    ("Ma&ximize", ComandoSistema::Maximizar),
                    ("", ComandoSistema::Separador),
                    ("&Close", ComandoSistema::Fechar),
                    ("", ComandoSistema::Separador),
                    ("Nex&t", ComandoSistema::Proxima),
                ]
                .iter()
                .map(|&(t, c)| ItemMenuSistema { texto: t.into(), atalho: String::new(), comando: c })
                .collect(),
                largura: 149,
                altura_item: 18,
                altura_separador: 7,
                linha_separador: 3,
                folga_base: 0,
                margem_texto: 16,
                topo_texto: 0,
                coluna_atalho: 88,
                fontes: vec![],
                cor_fundo: 15,
                cor_texto: 0,
                cor_desabilitado: 7,
                cor_realce: 4,
                cor_texto_realce: 15,
                cor_borda: 0,
            },
        }
    }

    /// Edição (0) restaurada como na bancada, Caste (19) por cima, Behavior (18) embaixo.
    fn tres() -> Gerenciador {
        let mut g = Gerenciador::new(cfg(), r(0, 73, 640, 442));
        g.abrir(18, r(89, 236, 277, 424), controle(), FINA);
        g.abrir(0, r(20, 97, 444, 444), edicao(), GROSSA);
        g.abrir(19, r(344, 234, 554, 424), controle(), FINA);
        g
    }

    #[test]
    fn contorno_de_arrasto_em_xor_como_w02() {
        // Sobre preto (0) e azul-escuro (4): nos pontos com x + y ímpar, branco (15) e amarelo (11).
        let mut q = Quadro::new(20, 20, 0);
        for x in 0..20 {
            q.ponto(x, 10, 4);
        }
        contorno_pontilhado(&mut q, r(2, 2, 12, 12), 15);
        let px = |x: usize, y: usize| q.pixels[y * 20 + x];
        assert_eq!((px(3, 2), px(4, 2)), (15, 0));
        assert_eq!((px(2, 3), px(2, 4)), (15, 0));
        assert_eq!((px(2, 9), px(12, 9), px(12, 10)), (15, 15, 4));
        assert_eq!(px(2, 10), 4, "x + y par: fica");
        assert_eq!((px(3, 10), px(5, 5)), (4, 0), "dentro do contorno nada muda");
        // Desenhar duas vezes desfaz (XOR).
        contorno_pontilhado(&mut q, r(2, 2, 12, 12), 15);
        assert!(q.pixels.iter().enumerate().all(|(i, &c)| c == if i / 20 == 10 { 4 } else { 0 }));
    }

    #[test]
    fn estilos_do_wingant() {
        // _win_Open: 0x06/0x04 (Caste, Behavior, mapa, quintal) = título e menu de sistema, moldura fina.
        let c = controle();
        assert!(c.titulo && c.menu_sistema && !c.maximizar && !c.minimizar && !c.moldura_grossa && !c.rolagem);
        // Janela de edição: 0x0e/0x05 = moldura grossa, barras e botão de maximizar.
        let e = edicao();
        assert!(e.titulo && e.menu_sistema && e.maximizar && !e.minimizar && e.moldura_grossa && e.rolagem);
        // Menus (0x41): modal, sem título.
        let m = Estilo::do_layout([0x41, 0x18], false);
        assert!(m.modal && !m.titulo);
    }

    #[test]
    fn geometria_igual_a_da_moldura_medida() {
        // Edição: contorno (20, 97)–(444, 444), área cliente (24, 120)–(424, 424) (tela_jogo.json).
        let g = geometria(r(20, 97, 444, 444), &edicao(), &GROSSA);
        assert_eq!(g.cliente, r(24, 120, 424, 424));
        assert_eq!(g.titulo, Some(r(24, 101, 440, 119)));
        assert_eq!(g.caixa_menu, Some(r(24, 101, 42, 119)));
        assert_eq!(g.botao_maximizar, Some(r(422, 101, 440, 119)));
        assert_eq!(g.barra_h, Some(r(24, 425, 424, 440)));
        assert_eq!(g.barra_v, Some(r(425, 120, 440, 424)));
        // Janela de controle: 1 pixel de moldura, título de 18 + 1 linha.
        let c = geometria(r(344, 234, 554, 424), &controle(), &FINA);
        assert_eq!(c.titulo, Some(r(345, 235, 553, 253)));
        assert_eq!(c.cliente, r(345, 254, 553, 423));
    }

    #[test]
    fn polegar_como_na_captura_j00() {
        // Barra horizontal da edição (y da bancada − 38): pos 22 de 39 → linha do polegar em x = 238.
        let hz = r(24, 425, 424, 440);
        assert_eq!(trilho(hz, true, 15), (39, 408));
        assert_eq!(inicio_do_polegar(hz, true, 15, Rolagem { pos: 22, max: 39 }), Some(238));
        assert_eq!(inicio_do_polegar(hz, true, 15, Rolagem { pos: 0, max: 39 }), Some(39));
        assert_eq!(inicio_do_polegar(hz, true, 15, Rolagem { pos: 39, max: 39 }), Some(392));
        assert_eq!(posicao_do_polegar(hz, true, 15, 39, 238), 22);
        assert_eq!(posicao_do_polegar(hz, true, 15, 39, 0), 0);
        assert_eq!(posicao_do_polegar(hz, true, 15, 39, 999), 39);
        let rol = Rolagem { pos: 22, max: 39 };
        assert_eq!(parte_da_barra(hz, true, 15, rol, 30), ParteBarra::SetaMenos);
        assert_eq!(parte_da_barra(hz, true, 15, rol, 100), ParteBarra::PaginaMenos);
        assert_eq!(parte_da_barra(hz, true, 15, rol, 246), ParteBarra::Polegar);
        assert_eq!(parte_da_barra(hz, true, 15, rol, 330), ParteBarra::PaginaMais);
        assert_eq!(parte_da_barra(hz, true, 15, rol, 415), ParteBarra::SetaMais);
        // Sem faixa (max 0): sem polegar.
        assert_eq!(inicio_do_polegar(hz, true, 15, Rolagem { pos: 0, max: 0 }), None);
    }

    #[test]
    fn clique_vai_so_para_a_janela_da_frente() {
        let mut g = tres();
        // (400, 300) está na edição e na Caste; a Caste é a de cima e recebe.
        assert_eq!(g.janela_em(400, 300), Some(19));
        assert_eq!(g.pressionar(400, 300, false), Clique::Cliente(19));
        // (200, 300): edição (por cima da Behavior), que não é a de cima: vem para a frente e o
        // clique é comido.
        assert_eq!(g.pressionar(200, 300, false), Clique::Ativou(0));
        assert_eq!(g.topo(), Some(0));
        assert_eq!(g.ordem(), vec![18, 19, 0]);
        // Segundo clique no mesmo ponto: agora vai para a área cliente da edição.
        assert_eq!(g.pressionar(200, 300, false), Clique::Cliente(0));
        // (400, 300) agora é da edição (que cobre a Caste).
        assert_eq!(g.janela_em(400, 300), Some(0));
        // Fora de tudo (faixa ou fundo).
        assert_eq!(g.pressionar(600, 100, false), Clique::Fora);
        assert!(g.visivel_em(0, 400, 300));
        assert!(!g.visivel_em(19, 400, 300));
    }

    #[test]
    fn ordem_z_abrir_fechar_reabrir_e_proxima() {
        let mut g = tres();
        assert_eq!(g.ordem(), vec![18, 0, 19]);
        g.abrir(18, r(0, 0, 10, 10), controle(), FINA);
        assert_eq!(g.ordem(), vec![0, 19, 18]);
        // Posição guardada ao fechar: reabre onde estava, não no retângulo pedido.
        assert!(g.fechar(18));
        assert!(!g.aberta(18));
        assert_eq!(g.topo(), Some(19));
        g.abrir(18, r(0, 0, 10, 10), controle(), FINA);
        assert_eq!(g.janela(18).unwrap().externo, r(89, 236, 277, 424));
        // Ctrl+F6: a de baixo vem para cima.
        assert_eq!(g.proxima(), Some(0));
        assert_eq!(g.ordem(), vec![19, 18, 0]);
        // _win_Swap: o quintal (25) assume o lugar e o retângulo do mapa (1).
        g.abrir(1, r(118, 98, 632, 376), controle(), FINA);
        g.trazer_para_frente(0);
        assert!(g.trocar(1, 25, controle(), FINA));
        assert_eq!(g.ordem(), vec![19, 18, 25, 0]);
        assert_eq!(g.janela(25).unwrap().externo, r(118, 98, 632, 376));
        assert!(!g.trocar(1, 25, controle(), FINA));
    }

    #[test]
    fn arrastar_pelo_titulo_com_o_cursor_preso_a_area() {
        let mut g = tres();
        // Título da Caste (cursor da bancada em (457, 281) = (457, 243) aqui), −200, −100.
        assert_eq!(g.pressionar(457, 243, false), Clique::Nada);
        let (redesenha, _) = g.mover_mouse(257, 143);
        assert!(redesenha);
        // Durante o arrasto a janela não sai do lugar; só o contorno.
        assert_eq!(g.janela(19).unwrap().externo, r(344, 234, 554, 424));
        assert!(matches!(g.arrasto, Some(Arrasto::Mover { contorno, .. }) if contorno == r(144, 134, 354, 324)));
        assert_eq!(g.soltar(257, 143), Clique::Moveu(19));
        assert_eq!(g.janela(19).unwrap().externo, r(144, 134, 354, 324));
        // Além do canto: o cursor fica em (0, 73) (bancada w05/w06: a janela sai pela esquerda e
        // por cima, cortada).
        g.pressionar(257, 143, false);
        g.mover_mouse(-143, -157);
        assert_eq!(g.soltar(-143, -157), Clique::Moveu(19));
        assert_eq!(g.janela(19).unwrap().externo, r(-113, 64, 97, 254));
    }

    #[test]
    fn menu_de_sistema_e_duplo_clique_fecham() {
        let mut g = tres();
        // Clique simples na caixa: abre o menu embaixo dela, com o primeiro item realçado.
        assert_eq!(g.pressionar(350, 243, false), Clique::Nada);
        let m = g.menu.unwrap();
        assert_eq!((m.id, m.x, m.y, m.realcado), (19, 344, 253, Some(0)));
        // Restore, Size, Minimize e Maximize acinzentados numa janela de controle.
        assert!(!g.habilitado(19, ComandoSistema::Restaurar));
        assert!(g.habilitado(19, ComandoSistema::Mover));
        assert!(!g.habilitado(19, ComandoSistema::Tamanho));
        assert!(!g.habilitado(19, ComandoSistema::Minimizar));
        assert!(!g.habilitado(19, ComandoSistema::Maximizar));
        // Escape fecha o menu.
        assert_eq!(g.tecla("Escape"), Some(Clique::Nada));
        assert!(g.menu.is_none());
        // Duplo clique na caixa: o primeiro abre o menu, o segundo fecha a janela.
        g.pressionar(350, 243, false);
        assert_eq!(g.pressionar(350, 243, true), Clique::Fechar(19));
        // Close pelo menu (clique no item e soltar nele).
        g.pressionar(350, 243, false);
        let item_close = 6;
        let (itens, _) = g.layout_menu();
        let y = 253 + itens[item_close].0 + 5;
        g.pressionar(360, y, false);
        assert_eq!(g.soltar(360, y), Clique::Fechar(19));
        // Pela tecla do item (C).
        g.abrir_menu(19);
        assert_eq!(g.tecla("C"), Some(Clique::Fechar(19)));
        // Item acinzentado não faz nada e o menu continua aberto.
        g.abrir_menu(19);
        assert_eq!(g.tecla("x"), Some(Clique::Nada));
        assert!(g.menu.is_some());
    }

    #[test]
    fn fechar_ativa_a_de_cima_das_que_sobram() {
        let mut g = tres();
        assert!(g.fechar(19));
        assert_eq!(g.topo(), Some(0));
        assert_eq!(g.ordem(), vec![18, 0]);
        assert!(!g.fechar(19));
    }

    #[test]
    fn maximizar_restaurar_e_botao() {
        let mut g = tres();
        g.trazer_para_frente(0);
        // Botão de maximizar: age ao soltar dentro dele.
        assert_eq!(g.pressionar(430, 110, false), Clique::Nada);
        assert_eq!(g.soltar(430, 110), Clique::Tamanho(0));
        let j = g.janela(0).unwrap();
        assert!(j.maximizada());
        // Maximizada: a moldura fica fora da área cliente da mãe (bancada w13).
        assert_eq!(j.externo, r(-4, 69, 644, 446));
        assert_eq!(j.geometria().cliente, r(0, 92, 624, 426));
        // Título não arrasta janela maximizada.
        assert_eq!(g.pressionar(300, 80, false), Clique::Nada);
        assert!(g.arrasto.is_none());
        // Soltar fora do botão não restaura.
        g.pressionar(630, 80, false);
        assert_eq!(g.soltar(300, 300), Clique::Nada);
        assert_eq!(g.comando(0, ComandoSistema::Restaurar), Clique::Tamanho(0));
        assert_eq!(g.janela(0).unwrap().externo, r(20, 97, 444, 444));
        // As de controle não maximizam.
        assert!(!g.maximizar(19));
    }

    #[test]
    fn redimensionar_com_minimo_e_maximo() {
        let mut g = tres();
        g.trazer_para_frente(0);
        // Borda direita (x = 442): −100.
        assert!(matches!(parte_em(g.janela(0).unwrap(), 442, 300), Some(Parte::Borda(Lados { dir: true, esq: false, topo: false, base: false }))));
        g.pressionar(442, 300, false);
        g.mover_mouse(342, 300);
        assert_eq!(g.soltar(342, 300), Clique::Tamanho(0));
        assert_eq!(g.janela(0).unwrap().externo, r(20, 97, 344, 444));
        // Borda esquerda +400: para no mínimo (317 de largura, bancada v17), com a direita parada.
        g.pressionar(21, 300, false);
        g.mover_mouse(421, 300);
        g.soltar(421, 300);
        assert_eq!(g.janela(0).unwrap().externo, r(27, 97, 344, 444));
        // Canto de baixo à direita: os dois lados; o máximo é a área cliente com a moldura.
        assert!(matches!(parte_em(g.janela(0).unwrap(), 343, 443), Some(Parte::Borda(Lados { dir: true, base: true, .. }))));
        g.pressionar(343, 443, false);
        g.mover_mouse(2000, 2000);
        g.soltar(2000, 2000);
        // O cursor fica em (639, 441): +296 e −2.
        assert_eq!(g.janela(0).unwrap().externo, r(27, 97, 640, 442));
        // A janela de controle não tem moldura grossa: a borda não redimensiona.
        g.trazer_para_frente(19);
        assert_eq!(parte_em(g.janela(19).unwrap(), 344, 300), Some(Parte::Moldura));
    }

    #[test]
    fn barras_de_rolagem_mandam_mensagens() {
        let mut g = tres();
        g.trazer_para_frente(0);
        g.definir_rolagem(0, Rolagem { pos: 22, max: 39 }, Rolagem { pos: 0, max: 45 });
        assert_eq!(g.pressionar(30, 432, false), Clique::Rolar(0, Barra::Horizontal, ComandoRolagem::LinhaMenos));
        assert_eq!(g.pressionar(415, 432, false), Clique::Rolar(0, Barra::Horizontal, ComandoRolagem::LinhaMais));
        assert_eq!(g.pressionar(330, 432, false), Clique::Rolar(0, Barra::Horizontal, ComandoRolagem::PaginaMais));
        assert_eq!(g.pressionar(100, 432, false), Clique::Rolar(0, Barra::Horizontal, ComandoRolagem::PaginaMenos));
        assert_eq!(g.pressionar(432, 127, false), Clique::Rolar(0, Barra::Vertical, ComandoRolagem::LinhaMenos));
        assert_eq!(g.pressionar(432, 300, false), Clique::Rolar(0, Barra::Vertical, ComandoRolagem::PaginaMais));
        // Arrasto do polegar horizontal (+100): SB_THUMBTRACK durante, SB_THUMBPOSITION ao soltar.
        assert_eq!(g.pressionar(246, 432, false), Clique::Nada);
        let (_, msg) = g.mover_mouse(346, 432);
        let Some(Clique::Rolar(0, Barra::Horizontal, ComandoRolagem::Acompanhar(p))) = msg else { panic!("{msg:?}") };
        assert_eq!(p, 33);
        assert_eq!(g.soltar(346, 432), Clique::Rolar(0, Barra::Horizontal, ComandoRolagem::Posicao(33)));
        // O canto entre as barras não faz nada.
        assert_eq!(g.pressionar(432, 432, false), Clique::Nada);
    }

    #[test]
    fn janela_modal_no_topo_nao_deixa_ativar_as_outras() {
        let mut g = tres();
        g.abrir(27, r(100, 100, 300, 200), Estilo::do_layout([0x42, 0x08], false), FINA);
        assert_eq!(g.pressionar(30, 300, false), Clique::Nada);
        assert_eq!(g.topo(), Some(27));
    }

    /// Geometria do menu de sistema medida pixel a pixel na captura w04 da bancada (menu da Caste
    /// Control): itens de 18 linhas a partir da linha 1 do contorno, separadores de 7, contorno de
    /// baixo logo depois do último item (Close em 98, Next em 123, altura 142).
    #[test]
    fn geometria_do_menu_de_sistema_como_na_bancada() {
        let g = Gerenciador::new(cfg(), r(0, 73, 640, 442));
        let (itens, altura) = g.layout_menu();
        let topos: Vec<i32> = itens.iter().map(|&(t, _)| t).collect();
        assert_eq!(topos, [1, 19, 37, 55, 73, 91, 98, 116, 123]);
        assert_eq!(altura, 142);
    }
}
