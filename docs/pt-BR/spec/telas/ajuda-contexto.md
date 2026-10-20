*Read in English: [ajuda-contexto.md](../../../spec/telas/ajuda-contexto.md)*

# Ajuda: botão direito, modo de ajuda, botões "?" e cursores

Como o SimAnt chega à ajuda (`SIMANT.HLP`, aberto pelo WinHelp no original). O visualizador do port (o que aparece depois) está em [ajuda.md](ajuda.md); o formato do arquivo em [../formatos/hlp.md](../formatos/hlp.md).

Implementação: `engine/src/core/ajuda_contexto.rs` (tabela de contextos e teclas), `simant/src/ajuda_ui.rs` (entrada e cursores), `engine/src/formats/ne.rs` (cursores do executável). Dados: `data/ajuda_contexto.json`.

## 1. O botão direito liga o modo de ajuda

Todas as janelas do jogo (edição, mapa, quintal, controles, faixa) usam o `MAINWNDPROC`. Nele, `WM_RBUTTONDOWN` (0x204) **não** vai para `_DoMouse`: inverte `_bHelp` (`DGROUP:0010`) e troca o cursor (ligado: `HELPCURSOR`; desligado: o cursor da classe) [código: `1000:330e`]. `WM_RBUTTONUP` vai a `_DoMouse`, que só atualiza `_mouse_state`; `WM_RBUTTONDBLCLK` (0x206) não é tratado (o segundo clique de um duplo não inverte de novo) [código]. Os ramos `0x204`/`0x206` de `_DoMouse` (estado 0x800/0x4000, "tratados como esquerdo" no `_processEdit`) não são alcançados pelo mouse.

**Bancada** (2026-10-18, `qemu_ui_barras_direito.txt` e `qemu_ui_ajuda.txt`, snapshot `quick` pausado):

- clique direito num ponto vazio da faixa: `_bHelp` 0 → 1 e o cursor vira a seta com "?" (`r01_direito_faixa.png`); outro clique direito: 1 → 0;
- clique direito no mapa da janela de edição: `_bHelp` 0 → 1, o destino da amarela (`_MeGoalX`) **não** muda (`a01_direito_edicao.png`);
- com o modo ligado, o clique esquerdo no mapa abre o WinHelp no tópico "Edit Window" e o modo desliga (`_bHelp` 0); a amarela não recebe ordem (`a02_ajuda_edicao.png`).

Isso corrige a especificação anterior (jogo.md, experimental.md), que dizia que o clique direito valia como o esquerdo na edição e no Experimental: valia pelo pseudocódigo de `_DoMouse`, mas o `MAINWNDPROC` desvia o 0x204 antes.

## 2. Teclas (`_HelpKeyDown`, `1000:0de6`)

Chamado por `_DoKeyDown` antes de `_YellowCommandKey` [código]:

| Tecla | Efeito |
|---|---|
| F1 | `WinHelp(HELP_INDEX)`: o índice (tópico "SimAnt", contexto 0x100) (`a06_f1.png`) |
| Shift+F1 | inverte `_bHelp` |
| Delete | inverte `_bHelp` (e segue para `_YellowCommandKey`) |
| Esc | com `_bHelp` ligado, desliga |

Espaço e Insert continuam imitando o clique esquerdo na posição do cursor; o ramo de `_DoKeyDown` que trataria Delete como clique direito só é alcançado com Espaço/Insert [código]. O port tirou Delete de `teclas_clique` (`data/partida.json`).

## 3. O clique com o modo ligado (`_DoEvent`, `1000:0bc2`)

Com `_bHelp` ligado, o evento do objeto clicado não vai para a rotina da janela: `_bHelp = 0` e `WinHelp(HELP_CONTEXT, contexto)` [código]. O contexto, pelo id do objeto (byte alto = janela):

| Objeto | Contexto |
|---|---|
| janela 0 (edição), qualquer objeto | 4 "Edit Window" |
| janela 1 (mapa) e 0x19 (quintal) | 0x102 "Map Window" |
| janelas 5, 0x12, 0x13 | janela × 0x100 (Information, Behavior Control, Caste Control) |
| History 0x1503–0x150D | o próprio id (Black Population … Help); o resto da 0x15: 0x1500 |
| faixa 0x2202–0x2209, 0x220B–0x220D, 0x2302–0x2305, 0x2308 | o próprio id (Yard View Button … Status Window Button, Patch … Graphic, Help Button) |
| 0x220E, 0x220F (ninho e trilha vermelhos) | id − 3 (os tópicos dos pretos) |
| marcadores 0x2218–0x221E e 0x2313–0x2319 | 0xFD48 "Bookmarks" |
| botões configuráveis 0x2210–0x2217 e 0x230B–0x2312 | contexto da **função** do botão: tabela `DS:8AE6` do segmento 8 (4 bytes por função; lida na bancada: 0x23F Behavior, 0x23E Caste, 0x23D History, 0x23C Info, 0x237 Trail, 0x23A Pause, 0x239 Speed, 0xFD32, 0x238 Mystery, 0xFD33, 0xFD36, 0xFD31, 0xFD35, 0xFD34, 0x23B Save, 0xFD47 Goto Yellow Ant) |
| outros objetos das faixas (fundo, `ribTools`, mensagem, barras…) | 0x2200 / 0x2300 |
| outras janelas | nenhum: só desliga o modo |

**Bancada:** mapa da edição → "Edit Window" (a02); `ribStats` → "Status Window Button" (a05); `ribUserDef1A` (função 15) → "Goto Yellow Ant Button" (a10). Os nomes dos tópicos vêm do `|CTXOMAP` do `SIMANT.HLP` ([../formatos/hlp.md](../formatos/hlp.md)).

O port acha o objeto como `_win_FindObject`: a janela da frente sob o cursor (o último objeto do layout que contém o ponto) ou a faixa; um clique fora de qualquer objeto não faz nada e o modo continua [hipótese: no original o clique no fundo da janela principal não gera evento].

## 4. Botões "?"

| Botão | Rotina | Contexto (`_DoWinHelp` usa `id & 0xFF00`) |
|---|---|---|
| `ribHelp` (0x2208, faixa 34, embaixo da área de mensagens) | `_ProcMapRibbonEvent`: `_LessonTemp = 1`, `_DoWinHelp(0x2208)` | 0x2200 "Patch Ribbon Bar" (`a04_ribhelp.png`) |
| `ribHelp2` (0x2308, faixa 35) | `_ProcYardRibbonEvent`: `_DoWinHelp(0x2308)` | 0x2300 "Yard Ribbon Bar" |
| Caste 0x1303 / Behavior 0x1203 / History 0x150D | `_DoWinHelp` | 0x1300 / 0x1200 / 0x1500 |
| `yardHelpBut` (0x1911, quintal) | `_DoWinHelp(0x1906)` | 0x1900 (não existe no `|CTXOMAP`) |
| `editHelpBut` (0x0006) e `mapHelpBut` (0x0116) | `_DoWinHelp(2)` / `_LessonTemp = 1`, `_DoWinHelp(0x103)` | invisíveis no layout de 640×480 (flags 0) |

O "?" logo abaixo da área de mensagens da faixa, que o usuário relatou, é o `ribHelp`. O "Mystery" é outro botão (função 8 dos configuráveis).

## 5. Cursores (`WM_SETCURSOR` em `MAINWNDPROC`)

Com `_bHelp` ligado, sempre `HELPCURSOR`. Senão, no Experimental, sobre a área cliente da janela de edição quando ela é a da frente (ou da janela do mapa da frente com Shift), o cursor da ferramenta: 0 `MAGCURSOR`, 1 `ROCKCURSOR`, 2 `DIGCURSOR`, 3 `ANTCURSOR`, 4 `FOODCURSOR`, 5 `DROPCURSOR`, 6 `SPRAYCURSOR`; a ferramenta 7 e o resto ficam com o cursor da classe (seta) [código]. Os cursores são recursos `GROUP_CURSOR`/`CURSOR` do `SIMANTW.EXE` (32×32, monocromáticos; o `HELPCURSOR` tem o ponto quente na ponta da seta, como na captura r01) — ver [../formatos/ne-recursos.md](../formatos/ne-recursos.md).

O port cria cursores do SDL com eles (AND 1 + XOR 1, "inverter", vira preto) e escolhe a cada quadro (`ajuda_ui::cursor_em`). Não faz o caso da janela do mapa com Shift.

## 6. O que o port faz e o que falta

- Clique direito em qualquer lugar (exceto com o "Select a Game" ou o menu de ferramentas abertos): a janela sob o cursor vem para a frente e o modo de ajuda alterna; o segundo clique de um duplo é ignorado. No Experimental, o direito não aplica mais a ferramenta.
- Teclas F1, Shift+F1, Delete, Esc; clique no modo; botões "?" das faixas e das janelas de controle.
- Testes: `ajuda_por_contexto_como_no_original` (simant), `contextos_do_original` (engine), `cursores_do_simantw` (engine).
- **Não verificado na bancada:** o clique direito no Experimental e nas janelas de controle (só no Quick Game, na edição e na faixa), F1 com uma janela de controle na frente, Shift+F1, Delete, o `yardHelpBut`, o cursor das ferramentas.
