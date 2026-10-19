# Resolução da tela e layout (640×480, 800×600, 1024×768)

O SimAnt original não escala nada: os bitmaps, as fontes e as janelas têm sempre o mesmo tamanho em pixels. O que muda com a resolução do Windows é **onde** as janelas ficam e **quanto** de área cliente sobra em volta delas. Este documento descreve como o original decide isso e como o port imita com a "tela virtual" (`data/video.json`, `resolucao_virtual`).

Legenda: **[código]** = visto no pseudocódigo do Ghidra (`re_trabalho/decomp/`, nomes do `SIMANTW.SYM`); **[medido]** = medido pixel a pixel numa captura do original; **[hipótese]** = não confirmado.

## 1. Janela principal: maximizada, do tamanho da tela

- `_InitInstance` (`1000:3c8a`) cria a janela principal e chama `ShowWindow(..., nCmdShow | 3)`: com o 3 (`SW_SHOWMAXIMIZED`) ela abre **maximizada** em qualquer resolução [código]. A moldura da janela maximizada fica fora da tela; sobram a barra de título e a barra de menus.
- Logo depois, o `GetClientRect` da janela principal dá `_screenWidth`/`_screenHeight` e são criadas duas janelas filhas: a **faixa** (`_ribbonBarWnd`), com a largura toda da área cliente, e a **raiz** (`_rootWnd`), onde ficam as janelas do jogo [código].
- No `WM_SIZE` da janela principal (`MAINWNDPROC`, `1000:2930`), a faixa é redimensionada para (largura da área cliente × altura da faixa) e a raiz para (largura × altura da área cliente − altura da faixa); `_screenWidth`/`_screenHeight` passam a ser o tamanho da raiz [código].
- A área cliente começa na **linha 38** da tela: barra de título (linhas 0–18) e barra de menus (19–36, mais a linha preta 37). **Medido** em 640×480 (bancada QEMU, `VGA.DRV`; `data/barra_menus.json`), em 800×600 (DOSBox-X, `S3TRIO.DRV`; `formatos/janela.md`, "Posicionamento na tela") e em 640×480, 800×600 e 1024×768 na bancada SVGA (seção 5). As alturas vêm de `GetSystemMetrics` (`SM_CYCAPTION`, `SM_CYMENU`), que dependem da fonte do sistema do driver; com fontes grandes seriam outras [hipótese].

Logo, para uma tela de L × A:

| Tela | Área cliente da janela principal | Faixa (janela 34) | Raiz (janelas do jogo) |
|---|---|---|---|
| 640×480 | 640 × 442 | 640 × 73 | 640 × 369 |
| 800×600 | 800 × 562 | 800 × 73 | 800 × 489 |
| 1024×768 | 1024 × 730 | 1024 × 73 | 1024 × 657 |

A faixa tem fundo cinza claro na largura toda; os objetos da janela 34 (`mapRibbon`, 640 pixels de largura) ficam encostados à esquerda e o resto é só fundo [medido em 800×600: `titulo_800x600.png`, colunas 640–799 das linhas 38–110; e em 1024×768, `svga1024_quick.png`].

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
- **Caixas de mensagem** (`MessageBox`: "SimAnt Load", "SimAnt Quit", a "Message" de "Saved correctly"): centradas na **tela inteira** (não na raiz nem na janela-mãe), com a conta feita sem o contorno de 1 pixel: canto = ((L − (w − 2)) / 2, (A − (h − 2)) / 2), divisão inteira, e o x levado ao byte mais próximo pelo driver (§7) [medido]. A "SimAnt Quit" (348 × 129) fica em (144, 176) da tela em 640×480 VGA, (147, 176) em 640×480 SVGA, (227, 236) em 800×600 e (339, 320) em 1024×768; a "Message" de 272 × 137 em 800×600, em (265, 232) (`re_trabalho/svga/ag*_quit.png`, `ag*_salvo.png`; bancada VGA `l1`, `cen/salvo`).
- **Diálogos de arquivo** (`GetOpenFileName`/`GetSaveFileName`, "Load Ant Hill"/"Save Ant Hill As"): **mesmo lugar** nas três resoluções, canto em (58, 53) da tela em 256 cores e (56, 53) na VGA de 16 cores (x no byte, §7) [medido: `ag*_load_dialogo.png`, `ag*_salvar_dialogo.png`; bancada VGA `l2`].
- **Visor de ajuda** (WinHelp 3.1, sem `H_WindowPosition` no `WIN.INI`): da coluna L / 3 até L − 3 e da linha 2 até A − 3 da tela: (213, 2)–(637, 477) em 640×480, (266, 2)–(797, 597) em 800×600, (341, 2)–(1021, 765) em 1024×768 [medido; disco SVGA preparado com `--ajuda-padrao`, porque o `WIN.INI` da instalação do usuário guarda `[Windows Help] H_WindowPosition=[266,200,266,200,0]`, de uma sessão em 800×600, e com ela o visor abriria em outro lugar — hipótese, não testado]. O port põe o visor de L / 3 até L − 2 (exclusivo), do topo à base da área cliente (`data/ajuda.json` → `posicao_padrao`).
- **Quadro da amarela** (janela 26, `_YellowDialog`, `_SpiderDialog`, `_DrawSimPayoff`): **não** depende da resolução: o bitmap fica em (158, 163) da área cliente em 640, 800 e 1024 na SVGA (trapaça "fred", `ag*_fred_2.png`) e em (160, 163) na VGA (alinhamento ao byte na criação da janela, §7). O objeto do bitmap tem posição absoluta no layout (modo 0: (153, 85)) e a janela se ajusta a ele [código: `inspecionar layout 26`].
- **Fim de jogo, Info, About, quintal**: seguem a tabela (9/0 ou 9/8) como as outras janelas, com o alinhamento do driver. Medido na SVGA: fim de jogo com o contorno em (155, 241) da tela em 800×600 e 1024×768 (tabela 9/8: x = 155); Info em (313, 241) em 800×600 (311 + contorno 2); About em (178, 230) em 640×480 e (170, 230) em 800×600 e 1024×768; o quintal abre no lugar do mapa (contorno em x = 120 na SVGA, 119 na VGA), igual nas três resoluções. O quadro de texto do `_PictureDialog` (janela 30) fica centrado na raiz e **no mesmo x na VGA e na SVGA** (contorno em 186 em 640×480): a janela 30 já foi criada pelo registro da abertura, e o alinhamento só acontece na criação (§7).
- O resto (janelas de controle, mapa, History, Info, Examine, menus de segurar o botão) segue a tabela 9 ou a posição do clique, sem depender do tamanho da tela.

## 4. No port: tela virtual

O port não muda a resolução do monitor: desenha uma **tela virtual** do Windows 3.1 e a amplia com a escala inteira de sempre (`simant/src/video.rs`, decisões "Escala 1" e "Janela nítida").

- `data/video.json`, `resolucao_virtual`: `[640, 480]` (padrão, a da bancada) ou `[800, 600]`, `[1024, 768]`. Linha de comando: `--resolucao LxA` (vence o arquivo).
- `data/resolucao.json` guarda a regra do original como dados: linhas acima da área cliente (38), a escolha da tabela pela altura (até 480 → 0; senão 8) e as resoluções aceitas.
- Com a tela virtual L × A, o executável usa: área cliente L × (A − 38); tabela de posições pela regra; a abertura (`data/telas.json`) e a tela de jogo (`data/tela_jogo.json`) com esse tamanho e essa tabela. A janela do SDL tem a resolução lógica L × (A − 38) na abertura e L × (A − 38 + barra de menus) na partida (a barra de título é a do sistema atual), ampliada pela `escala`.
- Código: `engine/src/core/resolucao.rs` (regra e aplicação), `simant/src/video.rs` e `simant/src/main.rs`.
- Os testes e ferramentas que já existiam continuam com as configurações próprias: `tela_jogo.json` (640×442, tabela 0, a bancada) e as telas validadas do DOSBox-X (800×562, tabela 8).

## 5. Verificação

Referências do original: a captura antiga do DOSBox-X (800×600, `S3TRIO.DRV`, 256 cores), a bancada QEMU padrão (640×480, `VGA.DRV`, 16 cores) e a **bancada SVGA** (QEMU com o driver `vbesvga.drv`, 256 cores, em 640×480, 800×600 e 1024×768; [bancada-qemu.md](../../bancada-qemu.md), "Bancada SVGA"). Capturas da SVGA em `re_trabalho/svga/` (fora do git).

**Fatos verificados (2026-10-19):**

- A área cliente começa na linha 38 também em 1024×768 (barra de título 0–18, menus 19–37), e a faixa vai das linhas 38 a 110 com fundo cinza até a coluna 1023; o resto da área cliente, até a linha 767, tem o fundo creme (`svga1024_quick.png`). As 38 linhas deixam de ser hipótese.
- O título de 800×600 da bancada SVGA é idêntico ao do DOSBox-X (0 pixels diferentes em 6 bits por canal): os dois ambientes de 256 cores dão a mesma tela.
- As janelas de controle (Caste/Behavior) ficam na linha 522 (caixa "Soldiers", área cliente) em 800×600 e 1024×768 e na 395 em 640×480: **127 linhas** acima, a diferença entre as tabelas 9/8 e 9/0 (285 − 158). O port dá as mesmas linhas nas três resoluções. A janela de edição fica no mesmo lugar nas três (contorno externo na linha 97).
- O "Select a Game" e o título têm a mesma posição em 800×600 e 1024×768 (tabela 9/8); em 640×480 o título sobe (tabela 9/0).
- A abertura composta pela regra em 800×600 é idêntica (mesmo SHA-256) às telas já validadas contra o DOSBox-X (teste `engine/tests/resolucao_reais.rs`); o teste `simant/src/sessao.rs` (`testes_tela_virtual`) trava as posições das janelas de controle e o tamanho da tela em 640, 800 e 1024.

**Diferenças que vêm do driver, não da resolução** (iguais em 640×480, 800×600 e 1024×768 na bancada SVGA): a janela de edição tem o contorno externo em x = 16 em 256 cores e em x = 20 na bancada VGA de 16 cores; as janelas de controle, em x + 2 em 256 cores e x + 1 na VGA; o título da abertura, 3 pixels à direita na VGA (a "moldura 1 + 5 + 1" é a de 1 + 4 mais a primeira coluna do conteúdo, azul-escura, da mesma cor da moldura na VGA); o "Select a Game", 1 pixel à direita na VGA. Todas são o alinhamento ao byte (§7). As cores do sistema também mudam com o driver (§7).

## 6. Comparações registradas

Port (`--resolucao LxA --exportar-telas`, `--novo-jogo --semente 1 --exportar-tela-jogo`) contra a bancada SVGA, `tools/comparar_captura.py --mapear` (pixels iguais na área cliente):

| Tela | 640×480 | 800×600 | 1024×768 |
|---|---|---|---|
| Título | 99,89% | 99,98% | 99,99% |
| Select a Game | 96,19% | 97,60% | 98,56% |
| Quick Game, área cliente inteira | 87,97% | 91,23% | 94,94% |
| Quick Game, abaixo da linha 380 (janelas de controle) | 89,84% | 93,39% | 97,32% |

- Título: só o cursor difere. Select a Game: o conteúdo da janela 2 sai 1 pixel à esquerda na SVGA (nas três resoluções), e os botões configuráveis da faixa diferem. Quick Game: o jogo do original é outro (mundo aleatório; o port usa a semente 1), além do x da janela de edição (−4) e das de controle (+1) e da cor do título acima; a porcentagem cresce com a resolução porque sobra mais fundo igual. A mesma tela do port em 640×480 contra a bancada VGA: título 84,88%, Select a Game 99,25% (`qi_01_titulo.png`, `qi_05_selecao.png`).

Depois do perfil de driver (§7, 2026-10-19), o port com `"svga256"` contra a bancada SVGA (mesmos comandos, `--mapear`):

| Tela | 640×480 | 800×600 | 1024×768 |
|---|---|---|---|
| Título | 99,96% | 99,98% | 99,99% |
| Select a Game | 99,80% | 99,87% | 99,92% |
| Quick Game, área cliente inteira | 89,34% | 93,34% | 96,20% |
| Quick Game, abaixo da linha 380 | 96,21% | 98,60% | 99,39% |

E com o perfil padrão (`"vga"`) contra a bancada VGA em 640×480: título **99,96%** (antes 84,88%) e Select a Game **99,80%** (antes 99,25%); só o cursor e os botões configuráveis da faixa diferem. O que sobra no Quick Game é o mundo (aleatório no original) e o texto do título (preto em 256 cores; o port usa as cores do título da VGA na partida, §7).

Posições conferidas por teste (`simant/src/sessao.rs`, `posicoes_por_resolucao_e_driver`), nos perfis `"vga"` e `"svga256"` e nas três resoluções: caixa "SimAnt Quit", diálogo de arquivo, janela de edição, Info, fim de jogo, About, quintal, visor de ajuda e janela 26 — os valores de 256 cores são os medidos na SVGA; os da VGA, os medidos na bancada VGA em 640×480 e a regra nas outras.

**Não verificado:** a VGA de 16 cores em 800×600 e 1024×768 (não há driver de 16 cores para essas resoluções na bancada; o port aplica a regra). As capturas da SVGA feitas depois de um `loadvm` saem com a paleta de 256 cores trocada (cinzas pretos, título roxo): servem para posições, não para cores. O lugar do visor de ajuda com a posição guardada no `WIN.INI` do usuário.

## 7. Perfil de driver: o que muda com o driver de vídeo

O mesmo jogo, na mesma resolução, sai diferente no VGA de 16 cores (`VGA.DRV`) e em 256 cores (S3 Trio no DOSBox-X, `vbesvga.drv` na bancada SVGA). As causas encontradas:

1. **Alinhamento ao byte.** `_InitApplication` registra a classe `GenericWindow` (todas as janelas do jogo, `_win_Open`) e a `RibbonWindow` com o estilo **0x1008** = `CS_BYTEALIGNCLIENT | CS_DBLCLKS` [código: `REGISTERCLASS` em `Code1.c`, nomes em `DGROUP:0684` e `0692`]. Na criação da janela, o Windows põe a borda esquerda da **área cliente** no byte mais próximo da memória de vídeo: no VGA planar de 16 cores, 1 byte = 8 pixels; em 256 cores, 1 byte = 1 pixel (sem efeito). Arredondamento ao mais próximo, com o meio para cima (a janela de edição: área cliente em 16 + 4 = 20 → 24, janela em 20) [medido]. Conferido em todos os casos medidos: título (64 + 5 = 69 → 72: +3), Select a Game (138 + 5 = 143 → 144: +1), edição (+4), janelas de controle (86 + 2 + 1 = 89 → 88: −1, o "x + 1" contra o "x + 2" de 256 cores), fim de jogo (132 + 5 = 137 → 136: o ajuste de −1 medido antes), Info (277 + 2 + 1 = 280: já alinhada, o ajuste de +1 que o port tinha), About (o "contorno 1 pixel à direita do conteúdo"), janela 26 (158 → 160). Margens da área cliente pelo estilo de `_win_Open`: moldura de diálogo 5 (`WS_DLGFRAME`, janelas sem título), borda 1 (`WS_BORDER`, janelas com título), moldura de redimensionar 4 (`WS_THICKFRAME`, a edição). As **caixas de diálogo** do Windows (`MessageBox`, diálogos de arquivo) alinham o **canto da janela**, não a área cliente (`CS_BYTEALIGNWINDOW` da classe de diálogo [hipótese pelo comportamento]): "SimAnt Quit" 147 → 144, "Message" 173 → 176, diálogo de arquivo 58 → 56 [medido na VGA contra a SVGA]. Só a **criação** alinha: uma janela que já existe e é reposicionada fica onde foi pedida (o quadro do `_PictureDialog`, a janela 30 criada pelo registro; Examine e mini mapa da segunda abertura em diante: `gerenciador::alinhar_cliente_ao_criar`, `exame.json` e `mini_mapa.json` → `alinhamento_ao_criar`).
2. **Cores do sistema.** O `WIN.INI` do usuário não tem `[colors]`; o Windows usa então as cores padrão que o próprio driver traz no recurso `OEMBIN 1` (a partir do byte 0x12, 4 bytes por cor do sistema na ordem de `COLOR_*`) [visto nos .DRV com `tools/ne_info.py`]. `VGA.DRV`: área de trabalho cinza (C0C0C0), **título ativo azul-escuro (000080)** com texto branco, **fundo da aplicação branco (FFFFFF)**, realce azul-escuro. `S3TRIO.DRV` e `VBESVGA.DRV` (configuração 1): área de trabalho A0A0A4, **título ativo azul-claro (A4C8F0)** com texto preto, **fundo da aplicação creme (FFFBF0)**, realce A4C8F0. Por isso a abertura do port (medida no DOSBox-X) era azul-claro e creme, e a partida (medida na bancada VGA) azul-escuro. Os mesmos recursos têm, nos dois primeiros campos, a largura e a altura das barras de rolagem (17 × 17 no VGA; 22 × 21 na configuração 1 do S3/vbesvga, 17 × 17 na 2001) [visto; qual configuração o vbesvga usa em 96 dpi não foi medido].
3. **Um pixel do S3.** No DOSBox-X (S3 Trio), o conteúdo das janelas com contorno (objeto 0 tipo 15) sai 1 pixel à direita do que sai na VGA e no vbesvga, com a moldura no mesmo lugar (o bitmap 2500 do Select a Game mostra a coluna 0, que nos outros dois fica fora) [medido]. [hipótese: recorte do driver S3 com destino x negativo]. Não é regra do Windows.

**No port:** `data/resolucao.json` → `perfil_driver` (`"vga"`, o padrão: Windows 3.1 VGA padrão, a referência mais fiel; `docs/decisoes.md`) e `perfis_driver` → `alinhamento_x` (8 ou 1), `ajuste_x_contorno` (1 só no `"s3_dosbox"`) e `cores_sistema` (troca os índices 16 e 17 de `data/paleta.json`). Código: `engine/src/core/resolucao.rs` (`PerfilDriver`), `Interface::posicionar` e `Interface::centrar_dialogo` (`telas.rs`), `janelas::geometria_janela`, `ConfigTelaJogo::alinhar_edicao`, `ConfigDialogoArquivo::aplicar_driver`, `Avisos::ajuste_janela_imagem`, `engine::paleta::carregar_rgb`. Os testes das telas validadas contra o DOSBox-X (`telas_reais.rs`, `resolucao_reais.rs`) usam o perfil `"s3_dosbox"` e mantêm os hashes. Limites: o texto do título da partida, o realce dos menus e as outras cores do sistema continuam fixos nas cores da VGA (`tela_jogo.json`, `barra_menus.json`, `dialogos.json`); com `"svga256"` só mudam os índices 16 e 17. O perfil se escolhe pelo dado (um mod com `{"perfil_driver": "svga256"}` em `data/resolucao.json`); não há opção de linha de comando.

**Examine e mini mapa (2026-10-19, API 0.30):** o deslocamento da primeira abertura da sessão (a janela é criada com `CS_BYTEALIGNCLIENT`: 3 pixels no Examine, 1 no mini mapa, medidos na VGA) usa o mesmo `alinhamento_x` do perfil (`JanelaExame::aplicar_driver`, `MiniMapa::aplicar_driver`); em 256 cores não há deslocamento [código: alinhamento 1 = 1 byte por pixel; não capturado na SVGA]. As chaves `alinhamento_ao_criar` de `exame.json` e `mini_mapa.json` passaram a ser opcionais (um mod as fixa). Teste `posicao_e_area_da_bancada` (os dois perfis).
