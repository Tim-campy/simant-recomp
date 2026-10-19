# Barra de menus da janela principal (File, Window, View, Options, Speed)

Fontes: pseudocódigo do Ghidra (`re_trabalho/decomp/Code1.c`, `Code2.c`, `Code7.c`), recursos do `SIMANTW.EXE` (`engine/src/formats/ne.rs`), objeto 6/0 do banco (`engine/src/formats/menu.rs`, [../formatos/menu.md](../formatos/menu.md)) e capturas da bancada QEMU (640×480, 16 cores) em `re_trabalho/agente_menus/` (fora do git), pelos roteiros `tests/referencias/roteiros/qemu_ui_menus.txt`, `qemu_ui_menus2.txt` e `qemu_ui_menus3.txt` (2026-10-18).

Port: `engine/src/core/barra_menus.rs` (barra, menus, teclado e mouse), `engine/src/core/status.rs` (Status), `engine/src/core/sobre.rs` (About), `simant/src/menus_ui.rs` (ligação com a partida), `data/barra_menus.json`.

## 1. De onde vem o menu

- O executável **não tem recurso MENU** (verificado: a tabela de recursos do NE só tem cursores, ícones, a ACCELERATORTABLE e outros; teste `aceleradores_do_simantw`). `_InitMenu(0)` (`1008:4e36`), chamado na inicialização (`Code2.c` l. 580), lê o **objeto tipo 6, id 0** do banco (`_db_LoadObject(0, 6, 0)`) e monta a barra com `CreateMenu`/`CreatePopupMenu`/`AppendMenu`; `SetMenu` a põe na janela principal [código].
- Cada título vira um popup; cada item, `AppendMenu` com o texto do objeto (`---` vira separador, `MF_SEPARATOR` = 0x800). O número do comando é **`0xFD01 + 16 × menu + item`** (os separadores também contam). Um item com um caractere especial e um número seria um submenu carregado de outro objeto tipo 6 (`_ms_LoadPopUpResource`, ids `0xF9xx`); o objeto 6/0 não tem nenhum [código; visto no dump do objeto].
- Os aceleradores (Ctrl+N etc.) são a ACCELERATORTABLE do executável, com os mesmos números (47 entradas, lidas por `ne::aceleradores`).

## 2. Itens e o que fazem (`_ProcMenu`, `1000:4fc2`)

`MAINWNDPROC` recebe `WM_COMMAND` com o byte alto 0xFD e manda para `_DoMenuEntry` (`1000:1876`), exceto os números `0xFDAx` (aceleradores de janela, janelas.md §8). `_DoMenuEntry` com o modo de ajuda ligado (`_bHelp`) desliga o modo e chama `_ProcMenuHelp`, que abre a ajuda do item (`WinHelp(HELP_CONTEXT, número do comando)`); senão, `_ProcMenu`.

| Menu | Item (texto do banco) | Comando | Atalho | O que faz |
|---|---|---|---|---|
| File | &About | 0xFD01 | — | `_AboutDialog` (§5) |
| | &New | 0xFD03 | Ctrl+N | `_myBeginSong(0x2711)` e `_NewGame(0)` ("Select a Game", abertura.md) |
| | &Load... | 0xFD04 | Ctrl+L | `_fileWaitFlag = 4` → `_LoadGame` (arquivos.md) |
| | &Save | 0xFD05 | Ctrl+S | `_fileWaitFlag = 5` → `_SaveGame(1)` |
| | Save &As... | 0xFD06 | Ctrl+A | `_fileWaitFlag = 6` → `_SaveGame(0)` |
| | &Quit | 0xFD08 | Ctrl+Q | `_fileWaitFlag = 8` → `_MenuQuit` (§4) |
| Window | &Edit | 0xFD11 | Ctrl+E | `_OpenEditWindow` |
| | &Map | 0xFD12 | Ctrl+M | `_OpenMapYard` (mapa, ou o quintal se ele estiver aberto) |
| | &Behavior | 0xFD13 | Ctrl+B | `_OpenModeWindow` |
| | &Caste | 0xFD14 | Ctrl+C | `_OpenCasteWindow` |
| | His&tory | 0xFD15 | Ctrl+H | `_OpenHistoryWindow` |
| | &Info | 0xFD16 | Ctrl+I | `_OpenInfoWindow` |
| | &Status | 0xFD17 | Ctrl+T | `_ScoreDialog` (§6) |
| | &Help | 0xFD18 | F1 | `WinHelp(HELP_INDEX)` |
| View | &Yard, &House, &Strategic, &Graphic | 0xFD21–0xFD24 | Ctrl+Y, 5, 6, 7 | `_SetYardMode(n)` se mudou, `_SetMapPlane(0)` se `_MapPlane` ≠ 0, `_MapToYard` se o quintal estava fechado, e o quintal para a frente (quintal.md) |
| | &Surface, &Black Nest, &Red Nest | 0xFD26–0xFD28 | Ctrl+8, 9, 0 | `_SetMapPlane(1..3)` e `_MakeEditOpen` |
| Options | &AutoTrack, &Music, &Sound Effects, &Events, Messa&ges, Si&lly | 0xFD31–0xFD36 | Ctrl+G, J, K (os três primeiros) | alterna `_OptionStates[n − 0x31]`; Music (0x32) chama `_StopSong` antes. Depois, `_SetMenuEntries`, `_UpdateUserButtons` e `_songsOnFlag`/`_effectsOnFlag` = `_OptionStates[1]`/`[2]` |
| Speed | &Pause | 0xFD41 | Shift+0 | alterna `_GamePaused` (com o fim das ferramentas 10/11 e a mensagem da faixa, como `_SetPause`) |
| | &Slow, &Normal, &Fast, &Ultra | 0xFD43–0xFD46 | Shift+1..4 | `_SimSpeed` = 0..3 |

**Marcas e textos (`_SetMenuEntries`, `1000:4c24`, também no fim de `_PauseGame`, `_SetPause` e `_ProcMenu`):** marca de seleção (`CheckMenuItem`, `MF_CHECKED`) no item da velocidade atual e em cada opção ligada de `_OptionStates`; o texto do Pause vira **"Un&pause\tShift+0"** com o jogo pausado e **"&Pause\tShift+0"** sem (`_SetMenuOptionText`, `ModifyMenu`; DGROUP 0x900 e 0x911) [código; bancada m05 e n05]. O objeto do banco traz "&Pause   \tShift+0" (com espaços), que só aparece antes do primeiro `_SetMenuEntries`.

**Acinzentados:** nenhum. O original não chama `EnableMenuItem` em lugar nenhum (`_CheckItem`, `1000:96b6`, é vazia) [código]; nas capturas nenhum item aparece cinza.

**Simulação:** com um menu aberto, o jogo não anda (bancada, `qemu_ui_menus3.txt`: `GameTime` 213 → 213 em 4 s com o File aberto) — o laço de menu do Windows não deixa o `WM_TIMER` chegar ao jogo [hipótese para a causa; o efeito foi medido]. O port para a simulação com a barra ativa (menu aberto ou título realçado pelo Alt [hipótese para o realce sem menu]).

## 3. Aparência e navegação (Windows 3.1)

Medido pixel a pixel nas capturas m00–m11 e n05 (bancada, 640×480):

- Barra: linhas 19–36 da tela (18 linhas, fundo branco) e a linha preta 37; a área cliente começa na 38 (a mesma origem de todas as outras specs de tela). Títulos com a fonte System do Windows (`VGASYS.FON`), um depois do outro desde x = 0, cada um ocupando a largura do texto + 8 pixels de cada lado: File 0–39, Window 40–107, View 108–153, Options 154–219, Speed 220–276. O título ativo fica em azul (cor do realce) com o texto branco, na altura toda da barra. A letra depois do `&` é sublinhada.
- Menu suspenso: contorno preto de 1 pixel, com a borda de cima na linha preta da barra e a esquerda no começo do realce do título; sem sombra. Itens de 18 linhas com a célula da fonte no topo; texto a 16 pixels da borda esquerda; atalho alinhado à esquerda numa coluna comum, 8 pixels depois do texto mais largo; a borda direita fica 15 pixels depois do atalho mais largo. Separadores de 7 linhas com a linha (de borda a borda) na 4.ª. Item realçado: faixa azul entre as bordas, texto branco. A largura é recalculada com o texto do momento (o Speed fica mais largo com "Unpause": borda direita em 351 contra 361).
- Marca de seleção: o desenho de 8 × 10 pixels de `data/barra_menus.json` (`marca`), a 5 pixels da borda e 3 abaixo do topo do item, na cor do texto (branca no item realçado).
- Mouse: apertar num título abre o menu dele; com o botão apertado, o realce segue o cursor pelos títulos e itens; soltar num item escolhe; soltar no título deixa o menu aberto com o **primeiro item realçado** (m09); soltar fora fecha. Clicar fora da barra e do menu fecha o menu (o clique não chega ao jogo).
- Teclado: Alt sozinho (apertado e solto) realça o título File sem abrir (m08); F10 também. Com um título realçado: ←/→ trocam de título, ↓/↑/Enter abrem com o primeiro item realçado, a letra sublinhada abre o menu dela, Esc ou Alt desligam. Alt+letra abre direto (m01: Alt+F abre o File com o About realçado). Com um menu aberto: ↑/↓ andam pelos itens (pulando separadores, com volta), ←/→ abrem o menu vizinho, Enter escolhe, a letra sublinhada escolhe o item, Esc fecha o menu e deixa o título realçado.

**Diferença:** no original, → no Speed e ← no File passam pelo menu de sistema da janela principal (m05b); no port, que não desenha o título da janela principal, a volta é direta entre File e Speed (divergencias.md).

**Conferência (2026-10-18):** a barra e os cinco menus desenhados pelo port (`engine/examples/barra_menus.rs`, e a janela do port com `--menu barra@0,0 --capturar-janela`) batem **100%** com m00 (fechada), m08 (Alt), m01 (File), m02 (Window), m03 (View), m04 (Options), m05 (Speed), n05 (Speed pausado, "Unpause") e m11 (Options com AutoTrack marcado, fora o cursor do mouse: 99,90%). A faixa da janela do port começa na linha certa (99,38% na região 0–639 × 38–111 de m00, com diferenças só do estado do jogo).

## 4. Quit (`_MenuQuit`, `1000:d628`)

- Chamado por File → Quit (Ctrl+Q), por `SC_CLOSE` da janela principal (fechar a janela, Alt+F4) e por `WM_QUERYENDSESSION` [código, `MAINWNDPROC`].
- Pergunta **sempre** (não olha `_gGameNeedsSaving`): `MessageBox("Do you want to save the current ant hill?", "SimAnt Quit", 0x1123)` — Yes/No/Cancel, ícone de pergunta, **No** como padrão (`MB_DEFBUTTON2`) [código; bancada m19]. Yes → `_SaveGame(0)` (Save As): salvo, o jogo termina; cancelado, continua. No → termina (bancada o03: o Windows fica sem o SimAnt). Cancel → volta ao jogo (m20). Uma segunda chamada depois de uma resposta não pergunta de novo (`DAT_1048_164e`).
- Port: a mesma caixa da pergunta do Load (dialogos.rs, `pergunta_sair`) com o título "SimAnt Quit": **99,51%** da caixa de m19 (as diferenças são o ícone, desenho próprio do port, e o cursor). Fechar a janela do port e Esc (extensão do port: antes saía na hora) também abrem a pergunta; com a pergunta aberta, fechar a janela de novo sai.

## 5. About (`_AboutDialog`, `1000:cf3a`)

- Guarda `_GamePaused`, chama `_SetPause(1)` ("Game Paused" na faixa), abre a janela 31 (`aboutWin`: fundo `aboutPic1` = bitmap 3000 com o título "ABOUT SIMANT" e o copyright, e a figura `aboutPic2` = bitmap 3001) e prende o mouse.
- Laço: espera com a figura; depois apaga a figura (redesenha `aboutPic1`) e rola os créditos da **lista 4/1700** com a fonte 4, cada linha centrada na largura de `aboutPic2` e recortada a ela, **um pixel para cima por passo**, começando com a linha 0 no topo; quando a última linha sai, a figura volta e o ciclo recomeça [código].
- Fecha com um clique ou Espaço/Insert (`_win_Events`): nos primeiros passos o evento é descartado (`local_e` > 1 com a figura, > 3 na rolagem); um clique com a figura antes disso só começa a rolagem [código]. Esc não fecha. Ao fechar, `_SetPause(o de antes)` (bancada: m16 continuou pausado porque o jogo estava pausado antes; n13 despausou).
- Tempos [medidos na bancada; o argumento de espera de `_WaitedEnough` não aparece no pseudocódigo]: figura por ~7,5–8,6 s (n11 ainda com a figura a ~6 s; n12 no deslocamento 0 a ~7,6 s; m14 no 25 a ~12,7 s) e ~172 ms por pixel (131 pixels em 22,5 s entre m14 e m15). O port usa 8 s e 165 ms (3 períodos de 55 ms) [hipótese].
- Moldura: a moldura de diálogo do Windows (contorno preto, 4 linhas azuis e o resto da borda) [medido em n09].
- **Conferência:** com a figura, **98,81%** de n09 (a moldura do original fica 1 pixel à direita da do port, com o conteúdo no lugar [causa em aberto]); a rolagem acha o deslocamento certo com 95,9–97,6% dos pixels da área dos créditos (n12, m14, m15; texto contra fundo).

## 6. Status (`_ScoreDialog`, `1000:5caa`; Window → Status, Ctrl+T)

- **É do jogo, não de depuração:** item "&Status\tCtrl+T" do menu Window, janela 24 com o título "STATUS" (bitmap 13000), colunas COLONY e UNIVERSE e a pontuação geral (bancada n02).
- `_CalcScore` dá as 8 notas (mensagens.md, "Pontuação"; `avisos::parciais`). As notas 0–3 vão para os objetos de texto `antEffEH`, `antEffFC`, `antEffAH`, `antEffFW` (Colony Health, Foraging, Eggs Hatched, Fights Won), com o formato deles ("%d%%"). `_win_DrawScoreWindow` (`1000:5aca`) escreve com a fonte 4: no Full Game (`_CurGameType` 2), as notas 4–7 ("%d%%") em `antEffCS`, `antEFFCH`, `antEffYF`, `antEffHF` (Colonies Started, Colonies Held, Yard Filled, House Filled); nos outros, "N.A." (4/1800[16]); embaixo, 4/1800[13] + nome do tipo (4/1001) + 4/1800[14] ("Overall Quick Game Score") e a pontuação ("%ld"); no Experimental, 4/1800[17] e [18] no lugar dessas duas linhas. O pé (`antEffContStr`, "%s") recebe 4/1010[14], "Click here to continue" [código; textos conferidos em n02].
- Fecha com um clique, Espaço ou Insert (`_win_Events`) ou Esc (`_DialogAbort`) (bancada: m18, n03, o02). **A simulação continua andando** com a janela aberta (`_win_Events` despacha o `WM_TIMER`; bancada: `GameTime` 119 → 139 em 5 s com o Status aberto). O port redesenha as notas a cada quadro.
- **Conferência:** com as notas da captura (97, 0, 100, 100; 19806), **100%** da janela de n02, com a janela 3 pixels à esquerda da posição calculada (`ajuste`) e os textos uma linha acima da regra do fim de jogo [medido; causa em aberto]. No Full Game e no Experimental não há captura.

## 7. Load Game do "Select a Game"

`_NewGame(0)` com Load Game faz o mesmo que File → Load (`_LoadGame`); sem jogo (na abertura, `_CurGameType` = −1), se o Load falha ou é cancelado, `_DoMenuEntry` chama `_NewGame(0)` de novo [código]. Port: na partida, o File → Load (com a pergunta, se a partida mudou); na abertura, o "Load Ant Hill" sobre o fundo da abertura (Cancel volta ao "Select a Game"; o arquivo escolhido abre como o `--save`). **Bancada (2026-10-19, `roteiros/qemu_load_abertura.txt`, capturas `re_trabalho/raros/load/a1`–`a5`):** o clique em Load Game (345, 375) **fecha** o "Select a Game" antes de abrir o "Load Ant Hill", que fica no mesmo lugar do File → Load da partida, sobre o fundo vazio; QUICK2 carrega pausado ("Game Paused", `_GamePaused` = 1), título QUICK2.ANT, vista do ninho. O port desenhava o diálogo por cima do "Select a Game" e passou a tirar a janela 2 (`simant/src/main.rs`). A tela carregada bate com `--save QUICK2.ANT` em 99,94% da janela de edição (o cursor); fora dela, a janela do mapa atrás difere (divergencias.md, 2026-10-19). Cancel não foi conferido.

## 8. O que não foi feito

- O menu de sistema da janela principal (minimizar, Switch To…) e o título dela: o port usa a janela do sistema operacional.
- Os submenus de `_ms_LoadPopUpResource` (sem uso no objeto 6/0).
- Status no Full Game e no Experimental, o Cancel do Load Game da abertura e a ajuda dos itens com o modo de ajuda (`_ProcMenuHelp`) não foram conferidos na bancada.
