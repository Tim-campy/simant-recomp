# Resolução da tela e layout (640×480, 800×600, 1024×768)

O SimAnt original não escala nada: os bitmaps, as fontes e as janelas têm sempre o mesmo tamanho em pixels. O que muda com a resolução do Windows é **onde** as janelas ficam e **quanto** de área cliente sobra em volta delas. Este documento descreve como o original decide isso e como o port imita com a "tela virtual" (`data/video.json`, `resolucao_virtual`).

Legenda: **[código]** = visto no pseudocódigo do Ghidra (`re_trabalho/decomp/`, nomes do `SIMANTW.SYM`); **[medido]** = medido pixel a pixel numa captura do original; **[hipótese]** = não confirmado.

## 1. Janela principal: maximizada, do tamanho da tela

- `_InitInstance` (`1000:3c8a`) cria a janela principal e chama `ShowWindow(..., nCmdShow | 3)`: com o 3 (`SW_SHOWMAXIMIZED`) ela abre **maximizada** em qualquer resolução [código]. A moldura da janela maximizada fica fora da tela; sobram a barra de título e a barra de menus.
- Logo depois, o `GetClientRect` da janela principal dá `_screenWidth`/`_screenHeight` e são criadas duas janelas filhas: a **faixa** (`_ribbonBarWnd`), com a largura toda da área cliente, e a **raiz** (`_rootWnd`), onde ficam as janelas do jogo [código].
- No `WM_SIZE` da janela principal (`MAINWNDPROC`, `1000:2930`), a faixa é redimensionada para (largura da área cliente × altura da faixa) e a raiz para (largura × altura da área cliente − altura da faixa); `_screenWidth`/`_screenHeight` passam a ser o tamanho da raiz [código].
- A área cliente começa na **linha 38** da tela: barra de título (linhas 0–18) e barra de menus (19–36, mais a linha preta 37). **Medido** em 640×480 (bancada QEMU, `VGA.DRV`; `data/barra_menus.json`) e em 800×600 (DOSBox-X, `S3TRIO.DRV`; `formatos/janela.md`, "Posicionamento na tela"). Em 1024×768 é [hipótese]: as alturas vêm de `GetSystemMetrics` (`SM_CYCAPTION`, `SM_CYMENU`), que dependem da fonte do sistema do driver; com fontes pequenas (as do VGA e do S3 a 800×600) dão as mesmas 38 linhas.

Logo, para uma tela de L × A:

| Tela | Área cliente da janela principal | Faixa (janela 34) | Raiz (janelas do jogo) |
|---|---|---|---|
| 640×480 | 640 × 442 | 640 × 73 | 640 × 369 |
| 800×600 | 800 × 562 | 800 × 73 | 800 × 489 |
| 1024×768 | 1024 × 730 [hipótese: 38 linhas] | 1024 × 73 | 1024 × 657 |

A faixa tem fundo cinza claro na largura toda; os objetos da janela 34 (`mapRibbon`, 640 pixels de largura) ficam encostados à esquerda e o resto é só fundo [medido em 800×600: `titulo_800x600.png`, colunas 640–799 das linhas 38–110].

## 2. Tabela de posições das janelas (objeto tipo 9)

`_win_LoadAllWindows` (`1030:c806`) pega o retângulo da **janela da área de trabalho** (`GetDesktopWindow` + `GetWindowRect`) e escolhe a tabela de posições pela **altura** da tela [código]:

| `_displayType` | base da área de trabalho < 481 (até 480 linhas) | 481 ou mais |
|---|---|---|
| 10 (cor, `WINGANT`) | 9/0 | 9/8 |
| 9 (mono, `MWINNT`) | 9/5 | 9/7 |

A largura não entra na conta. **Consequência:** 800×600 e 1024×768 usam a **mesma** tabela (9/8); não há tabela própria para 1024×768. Só existem essas duas por modo de vídeo.

As diferenças entre 9/0 e 9/8 do `WINGANT` (entradas `(x, y, largura, altura)` na área da raiz; as demais 28 entradas são iguais) [dados, `cargo run -p engine --example inspecionar -- <jogo> posicoes 0|8`]:

| Janela | 9/0 (até 480 linhas) | 9/8 (mais de 480) |
|---|---|---|
| 3 `titleWin` | (64, 3, 512, 342) | (64, 72, 512, 342) |
| 4 `gameOverWin` | (132, 59, 320, 200) | (155, 130, 320, 200) |
| 5 `infoWin` | (277, 128, 324, 220) | (311, 128, 324, 220) |
| 18 `modeControlWin` | (86, 158, 214, 192) | (86, 285, 214, 192) |
| 19 `casteControlWin` | (342, 158, 214, 191) | (342, 285, 214, 191) |
| 23 `antPopWin` | (0, 0, 184, 144) | (0, 0, 183, 144) |
| 25 (quintal) | (62, 18, 516, 277) | (123, 18, 516, 277) |
| 31 `aboutWin` | (176, 117, 322, 202) | (168, 117, 322, 202) |
| 33 `quitWin` | (133, 70, 337, 166) | (133, 145, 337, 166) |

Ou seja: em telas maiores o título e o "Quit" descem, as janelas de controle (Caste/Behavior) descem para baixo da janela de edição, Info, fim de jogo e quintal se deslocam. A janela de edição (0), o mapa (1), o "Select a Game" (2) e a faixa (34/35) ficam no mesmo lugar.

`_win_LoadWindow` (`1030:c662`) copia a entrada da tabela para o objeto 0 de cada janela (ou a guarda, se a entrada for `-32768`) [código]; `_win_Open` (`1030:ca2e`) soma `_ribbonBarHeight` (73) ao y quando a janela não é posicionada por parâmetro (modo 5) [código; formatos/janela.md].

## 3. O que se adapta ao tamanho da raiz

- **Diálogos de imagem** (`_PictureDialog`, `1000:62bc`, janela 30 — o registro da abertura e outros): centralizados na **área cliente da raiz** (`GetClientRect(_rootWnd)`), metade da sobra de cada lado [código]. Em 800×600, o registro fica em (270, 307)–(539, 411) da tela [medido em `apos_titulo.png`].
- **Janela de edição maximizada** (Tutorial, `_win_Zoom`, `ShowWindow(SW_SHOWMAXIMIZED)`): ocupa a raiz toda, com a moldura para fora; o `WM_GETMINMAXINFO` (`_AdjustWndMinMax`, `1000:1a6e`) usa o `GetClientRect` da raiz [código].
- **Caixas de mensagem e diálogos comuns do Windows** (`MessageBox`, abrir/salvar): o próprio Windows os centraliza na tela ou na janela-mãe [hipótese para o lugar exato em 800×600 e 1024×768: não capturado].
- O resto (janelas de controle, mapa, History, Info, Examine, menus de segurar o botão) segue a tabela 9 ou a posição do clique, sem depender do tamanho da tela.

## 4. No port: tela virtual

O port não muda a resolução do monitor: desenha uma **tela virtual** do Windows 3.1 e a amplia com a escala inteira de sempre (`simant/src/video.rs`, decisões "Escala 1" e "Janela nítida").

- `data/video.json`, `resolucao_virtual`: `[640, 480]` (padrão, a da bancada) ou `[800, 600]`, `[1024, 768]`. Linha de comando: `--resolucao LxA` (vence o arquivo).
- `data/resolucao.json` guarda a regra do original como dados: linhas acima da área cliente (38), a escolha da tabela pela altura (até 480 → 0; senão 8) e as resoluções aceitas.
- Com a tela virtual L × A, o executável usa: área cliente L × (A − 38); tabela de posições pela regra; a abertura (`data/telas.json`) e a tela de jogo (`data/tela_jogo.json`) com esse tamanho e essa tabela. A janela do SDL tem a resolução lógica L × (A − 38) na abertura e L × (A − 38 + barra de menus) na partida (a barra de título é a do sistema atual), ampliada pela `escala`.
- Código: `engine/src/core/resolucao.rs` (regra e aplicação), `simant/src/video.rs` e `simant/src/main.rs`.
- Os testes e ferramentas que já existiam continuam com as configurações próprias: `tela_jogo.json` (640×442, tabela 0, a bancada) e as telas validadas do DOSBox-X (800×562, tabela 8).

## 5. Verificação

- 800×600: a abertura composta pela regra (tela virtual 800×600 → 800×562, tabela 8) é **idêntica** (mesmo SHA-256) às telas validadas contra o DOSBox-X: título 99,98% e "Select a Game" 99,87% dos pixels iguais (formatos/janela.md, "Validação"; teste `engine/tests/resolucao_reais.rs`).
- 640×480: a abertura pela regra (640×442, tabela 0) foi comparada com as capturas da bancada `qi_01_titulo.png` e `qi_05_selecao.png` (seção 6).
- Tela de jogo em 800×600 e 1024×768: ver seção 6.

## 6. Comparações registradas

Ver a tabela em `tests/referencias/README.md` ("Tela virtual") e `docs/divergencias.md`.

**Não verificado:** a altura da barra de título e de menus em 1024×768 (38 linhas é [hipótese]); a posição das caixas de mensagem do Windows, do visor de ajuda (WinHelp) e do quadro da amarela (janela 26) fora de 640×480 — no port essas três vêm de medidas da bancada em 640×480 (`data/dialogos.json`, `data/ajuda.json`, `data/mensagens.json`) e não se adaptam.
