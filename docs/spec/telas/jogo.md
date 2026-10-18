# Tela de jogo (faixa + janela de edição)

Este documento descreve a tela que aparece durante uma partida: a faixa de ferramentas (janela 34, `mapRibbon`) e a **janela de edição** (janela 0, `editWin`), que mostra o mapa da superfície ou de um ninho com as formigas por cima. As outras janelas abertas numa partida (mapa, controle de castas e de modos) ficam para depois.

Implementação: `engine/src/core/vida.rs` (camada de vida), `engine/src/core/edicao.rs` (vista da janela de edição), `engine/src/core/tela_jogo.rs` (`compor_tela_de_jogo`, moldura do Windows), `engine/src/core/navegacao.rs` (rolagem e troca de plano), dados em `data/tela_jogo.json` e `data/navegacao.json`. Executável: `simant --save <save .ANT>` (janela) ou `--exportar-tela-jogo <png>`. Exemplo de comparação: `cargo run -p engine --example tela_jogo -- <pasta do jogo> <save .ANT> <pasta de saída> [captura.png]`.

**Fontes:** pseudocódigo do Ghidra em `re_trabalho/decomp/` (só consulta; nomes do `SIMANTW.SYM`), o `QUICK2.ANT` e a captura `re_trabalho/qi_14_salvo.png`, tirada pela bancada QEMU ([bancada-qemu.md](../../bancada-qemu.md)) na mesma sessão em que o save foi gravado. Legenda: **[código]** = visto no pseudocódigo, com o endereço; **[save]** = visto nos dados; **[medido]** = medido pixel a pixel na captura; **[hipótese]** = não confirmado.

## Janelas abertas num Quick Game

`_NewGame` (`1000:d6dc`), depois da escolha no "Select a Game" (o botão Quick Game, `scenQuickGame`, devolve 0x202, que vira o tipo de jogo 1; ver [abertura.md](abertura.md), "Escolha do jogo") [código]:

1. abre a janela de edição (`_win_Open(0)`), quando o tipo é 0;
2. `_OpenCasteWindow` e `_OpenModeWindow` (janelas de controle de casta e de modo);
3. `_SetMapPlane(_MapPlane)`; com `_MapPlane = 0` abre a janela 25 (`_MapToYard`), senão a janela 1, `mapWin` (`_YardToMap`);
4. `_SetMapTitle`, `_OpenEditWindow` (`_win_Open(0)`, `1010:0e2c`), `_SetMapPlane(_MePlane)`;
5. no tipo 0, se a janela de edição não estiver maximizada, `_win_Zoom(0)`, que é `ShowWindow(SW_SHOWMAXIMIZED)` (`1030:c2ba`);
6. `_CenterEdit(_MeLocX, _MeLocY)` e `_UpdateEdit` com `_editForce = 1`.

Ao carregar um jogo salvo, `_DoLoadInitializations` (`1010:9dbe`) chama `_SetDefaultWindows` (`1000:d66a`), que repete os passos 2 a 4 sem a maximização [código].

Na captura `qi_14_salvo.png` (640×480) aparecem a faixa, a janela de edição em primeiro plano e, atrás dela, à direita, partes do `mapWin` e de uma janela de controle ("Soldiers 40%") [medido]. O port, por enquanto, desenha só a faixa e a janela de edição.

## Planos e posição da vista

| `_MapPlane` | Vista | Mapa (`_DoCalcTile`) | Largura em tiles | Ponto lembrado |
|---|---|---|---|---|
| 0 | quintal (*yard*) | `_MapA` | 128 | `_YMapPnt` |
| 1 | superfície | `_MapA` | 128 | `_AMapPnt` |
| 2 | ninho preto | `_MapB` | 64 | `_BMapPnt` |
| 3 | ninho vermelho | `_MapR` | 64 | `_RMapPnt` |

**Fonte:** `_SetMapPlane` (`1000:a5ca`) lê o ponto lembrado do plano; `_BoundEditPosition` (`1010:15b4`) usa 0x80 nos planos 0 e 1 e 0x40 nos outros; `_SetMapPlaneLocation` (`1000:a386`) seleciona os botões da faixa (`Yard`, `Nest`, `Nest` vermelho, `Surface`) [código]. Todos os mapas têm 64 linhas.

- `_MapPnt` (`DS:7d2e`, par x/y; o y fica em `DS:7d30`) é a célula do mapa no canto superior esquerdo da vista. Está no save [save].
- Os pontos lembrados (`_YMapPnt`, `_AMapPnt`, `_BMapPnt`, `_RMapPnt`) são centros, não cantos: `_SetMapPlane` passa o ponto a `_CenterEdit` [código].
- No `QUICK2.ANT`: `_MapPlane = 2`, `_MePlane = 2`, `_MapPnt = (22, 0)`, `_BMapPnt = (34, 9)` = `(_MeLocX, _MeLocY)` [save].

### Limites (`_BoundEditPosition`)

Com `L` = largura do mapa do plano: se `x < 0`, `x = 0`; senão, se `x + _editWidth > L`, `x = L − _editWidth`. O mesmo para `y` com 64 e `_editHeight` [código; idêntico em `_win_EditChanged` (`1010:0c62`), `FUN_1010_16d4` e `_CenterEdit`].

### Centralizar (`_CenterEdit`, `1010:1620`)

1. deslocamento `dx = x − _MapPnt.x − _editWidth / 2`, `dy = y − _MapPnt.y − _editHeight / 2` (divisão inteira);
2. se os dois são 0, devolve 0 (nada a redesenhar);
3. senão, `FUN_1010_616c` anda o ponto como uma reta de Bresenham em ponto fixo: no eixo maior, dá |d| passos de ±1; no menor, soma a cada passo a fração `(|d_menor| << 16) / |d_maior|` (divisão inteira sem sinal) num acumulador de 32 bits e anda ±1 sempre que o bit 16 do acumulador fica ligado, apagando esse bit. Por causa do truncamento da fração, o eixo menor pode andar um passo a menos que o pedido (ex.: `dx = 3`, `dy = 1` → fração 21845, soma 65535, nenhum passo em y) [código];
4. aplica os limites e devolve 1.

### Tamanho da vista (`_editWidth`, `_editHeight`)

`_win_EditChanged` e `FUN_1010_16d4` pedem o retângulo do objeto 4 da janela 0 (`editTileArea`) a `_win_GetObjRect`, que soma 1 à direita e à base quando há faixa (`_ribbonBarHeight ≠ 0`, `1030:c2d2`). Depois: `_editWidth = ⌈largura / 16⌉`, `_editHeight = ⌈altura / 16⌉`, e o retângulo é esticado para múltiplos do tile [código].

`editTileArea` vai de (16, 42) a (416, 345) no `WINGANT`. Com a faixa, 401 × 304 pixels, o que daria 26 × 19; mas o original centraliza como se fossem **25 × 19** (mensagens.md §6.1), e o port usa 25. A janela recorta a última coluna: só 25 colunas (400 pixels) aparecem antes da barra de rolagem [medido].

### Posição ao carregar e na captura

- Ao carregar, `_DoLoadInitializations` grava `(_MeLocX, _MeLocY)` no ponto lembrado do plano `_MePlane`, e `_SetDefaultWindows` → `_SetMapPlane(_MapPlane)` centraliza a vista nele; se `_MePlane = _MapPlane`, centraliza de novo em `(_MeLocX, _MeLocY)` [código]. Com 26 colunas, o `QUICK2.ANT` carregado começaria em x = 34 − 13 = 21 [hipótese: não capturado]. **Bancada (2026-10-17, mensagens.md §6):** um save com a amarela e o `_BMapPnt` em (34, 9) no ninho preto (plano 2), lido da memória logo depois do Load, ficou com `_MapPnt.x` = **22**, não 21. O mesmo 22 aparece no `QUICK2.ANT` salvo logo depois de o Quick Game ser criado (centralizado em 34): `_editWidth` é **25**, não 26. O port passou a usar 25 colunas (`ajuste_area_com_faixa` = 0) e a vista depois do Load bate 99,94% com a captura (mensagens.md §6.1). O retângulo real do `editTileArea` depois de `_win_Recalc` com a janela maximizada continua sem conta [em aberto].
- A captura `qi_14_salvo.png` foi tirada no momento do save, sem carregar. Por isso a vista é a do `_MapPnt` salvo, (22, 0), que é o que o port usa ao reproduzir essa tela (`posicao: "salva"` em `data/tela_jogo.json`). O encaixe da captura confirma (22, 0) [medido].

## Camada de vida

### Mapas `_LifeA/B/R`

Um byte por célula, no mesmo índice do mapa de chão (`x × 64 + y`): 0 = vazio; 0xFF = formiga amarela; 0xFE = segunda metade da formiga amarela quando ela é rainha; outro valor = campo T da formiga ([save-ant.md](../formatos/save-ant.md), "Listas de formigas"). Não são salvos [código].

### Reconstrução ao carregar (`_DoLoadInitializations`, `1010:9dbe`)

1. troca o tileset de chão conforme `_TERRAINset` (`_OverlayTileSet(0, 1000 ou 1001)`) [código];
2. zera `_LifeA`, `_LifeB` e `_LifeR` [código];
3. para cada lista (A → `_LifeA`, B → `_LifeB`, R → `_LifeR`), percorre os índices **de `_ListIndex?` até 0, decrescendo, inclusive**: `Life[X[i] × 64 + Y[i]] = T[i]` [código]. Consequências:
   - a entrada de índice `_ListIndex?` (uma além da última usada) também é gravada. No `QUICK2.ANT` ela é toda zero e grava 0 na célula (0, 0) [save];
   - entradas com `T = 0` (buracos ainda não compactados) gravam 0 e apagam o que estava na célula;
   - quando duas formigas ocupam a mesma célula, **vence a de menor índice**, porque é gravada por último;
   - X e Y não são conferidos. Valores fora do mapa escreveriam em outra memória no original [código]; o port ignora a entrada e a lista como pendência;
4. se `_MeMode = 0`, chama `_SetMyLife(_MePlane, _MeLocX, _MeLocY, _MeType, _MeDir, 0xFF)` [código];
5. `_FullCount` (contagens), grava o ponto lembrado e chama `_SetDefaultWindows` (ver acima).

### Formiga amarela (`_SetMyLife`, `1020:6536`, e `_SetLife`, `1020:5d18`)

- Só age se (x, y) estiver dentro do mapa do plano (planos 0 e 1: 128 × 64; 2 e 3: 64 × 64).
- Grava o valor (0xFF) em `_LifeA` (planos 0 e 1), `_LifeB` (2) ou `_LifeR` (3), por cima do que as listas deixaram.
- Se `_MeType = 0x60` (rainha), grava também a segunda metade (0xFE) na célula vizinha, deslocada por uma tabela de direções indexada por `_MeDir ^ 4`. **A tabela não foi localizada** (o pseudocódigo mostra só o endereço relativo) [em aberto]; o port ainda não desenha essa metade e a registra como pendência.
- Nos ninhos, `_SetLife` com valor > 0 em célula de terra (`_IsItDirt`) ou de grama (0x1C a 0x1F) escava o tile (`_DigMyTile`) e toca um som [código]. Isso não muda a camada de vida e não ocorre no `QUICK2.ANT` (a formiga está num túnel); o port não reproduz a escavação ao carregar [hipótese: caso raro].

No `QUICK2.ANT`, `_MeMode = 0` e a formiga amarela está em (34, 9) do ninho preto, a mesma célula da rainha preta (T = 102) e de um ovo. Resultado em `_LifeB`: (34, 9) = 0xFF, (35, 9) = 110 (metade da rainha), (36, 9) = 1 (ovo) [save + código].

### Tile de vida de cada célula (`_DoCalcTile`, `1018:4a6b`, e `FUN_1010_63fe`)

Ver [tiles.md](../formatos/tiles.md), "Camada de vida". Já descontado o deslocamento de `FUN_1010_63fe`, o índice no buffer de vida do plano (superfície: objetos 9/15–17; ninhos: 9/18–20) é:

| Byte de vida *v* | Índice no buffer |
|---|---|
| 0 | nenhum (só o chão) |
| 0xFF | `0x100 + _MeBase + _MeType + (_MeType < 8 ? _MeEgg : _MeDir)` |
| 0xFE | `0x108 + _MeBase + _MeType + _MeDir` |
| outro | *v* |

A formiga amarela usa a terceira parte do buffer (índices 0x100 a 0x17F). No `QUICK2.ANT`: `_MeBase = 0`, `_MeType = 16`, `_MeDir = 2`, `_MeEgg = 0`, logo o tile 0x112 do buffer dos ninhos [save].

A célula é o tile de chão e, por cima, o tile de vida com o índice 13 transparente [código].

## Desenho da janela de edição

`FUN_1010_16d4` (`1010:16d4`) desenha as `_editWidth × _editHeight` células no buffer `_editBuf`, linha a linha; `_win_DrawEditWindow` (`1010:14f0`) copia o buffer para a tela no canto do `editTileArea` [código]. Cada célula (col, lin) vem de `FUN_1010_6250`: célula do mapa `(_MapPnt.x + col, _MapPnt.y + lin)`; se y passar de 63, soma 64 a x e faz `y &= 63` (com os limites, só aconteceria com `_editHeight > 64`) [código]. Por cima do mapa vêm a aranha e os balões (`_DrawSpider`, `_DrawBalloons`) e os gráficos de saúde (`_DrawEditGraphs`), ainda não especificados.

### Faixa

A faixa é a janela 34 desenhada como na abertura ([janela.md](../formatos/janela.md)), com uma diferença de estado: `_SetMapPlaneLocation` (`1000:a386`), quando há faixa, seleciona o botão do plano mostrado com `_win_MakeObjSelected`: `0x2205` (`ribToSurface`) no plano 1, `0x2203` (`ribToBNest`) no 2 e `0x2204` (`ribToRNest`) no 3; no plano 0 não seleciona nada [código; o pseudocódigo mostra esses números como ponteiros para texto, `s_Client_ERROR…_21ef + 0x14…0x16`]. O botão selecionado usa o bitmap alternativo (bit 2 das flags). Fica em `data/tela_jogo.json` (`botao_do_plano`). No `QUICK2.ANT` (plano 2), o `ribToBNest` selecionado bate pixel a pixel com a captura [medido].

Ainda não reproduzidos na faixa: as barras de população e saúde (`ribBlacks`, `ribReds`, `ribYellowHealth`, `ribBlackHealth`, `ribRedHealth`, `ribBlackP`, `ribRedP`, `mapRibCaste`, áreas desenhadas pelo código) e os botões configuráveis `ribUserDef1A`–`1H`, que o jogo troca conforme o `WIN.INI` (`_SetUserButton`).

### Objetos da janela 0

Os demais objetos da janela 0 não aparecem: só `editWinTitle` (tipo 18, título desenhado pelo Windows) e `editTileArea` (tipo 1) têm o bit "visível" [dados do `WINGANT`].

### Moldura e posição na bancada QEMU (640×480)

A janela de edição é uma janela do Windows com barra de título, moldura grossa e barras de rolagem: o byte de flags 0x1C do layout vale 0x0E, e `_win_Open` (`1030:ca2e`) monta o estilo `WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_HSCROLL | WS_VSCROLL` e soma ao tamanho `SM_CXVSCROLL + 2·SM_CXFRAME` e `SM_CYHSCROLL + 2·SM_CYFRAME`, além de `SM_CYCAPTION` [código]. A conta exata da posição, com a maximização do passo 5, **não foi fechada** [em aberto]. O port usa a geometria medida em `qi_10_quick.png` e `qi_14_salvo.png`, guardada em `data/tela_jogo.json` [medido; coordenadas de tela]:

| Elemento | Retângulo (x, y, largura, altura) |
|---|---|
| contorno externo (preto) | (20, 135, 424, 347) |
| moldura (cinza claro, índice 8) | 2 pixels dentro do contorno |
| linha interna (preta) | (23, 138, 418, 341) |
| barra de título | (24, 139, 416, 18), linha preta embaixo em y = 157 |
| caixa do menu de sistema | (24, 139, 18, 18), linha preta em x = 42 |
| botão de maximizar | (422, 139, 18, 18), linha preta em x = 421 |
| área do mapa visível | (24, 158, 400, 304) |
| barra de rolagem vertical | (424, 158, 16, 304), linha preta em x = 424 |
| barra de rolagem horizontal | (24, 462, 400, 16), linha preta em y = 462 |
| canto entre as barras | (424, 462, 16, 16) |

- A área cliente da janela principal começa em y = 38 (abaixo do menu), e a faixa ocupa as linhas 38 a 110 [medido], como no DOSBox-X ([janela.md](../formatos/janela.md)).
- O canto do `editTileArea` cai em (24, 158), o canto da área cliente da janela de edição logo abaixo do título. Isso bate com as regras de `_win_Recalc` (coordenadas relativas ao objeto 0 encolhido, menos a altura do título) [medido; regra de `_win_Recalc` vista no código, conta completa em aberto].
- O fundo da área cliente principal sai branco (255, 255, 255) na QEMU, e não (255, 251, 240) como no DOSBox-X [medido].
- Barras de rolagem: botões com relevo (branco em cima e à esquerda, cinza escuro índice 7 embaixo e à direita) e trilho cinza claro (índice 8) [medido]. As setas, o texto do título e o polegar das barras (cuja posição depende de `_ResetEditScrollRange`, não estudado) **não são desenhados** pelo port.

### Cores na bancada QEMU

O driver VGA do Windows na QEMU mostra algumas cores com valores diferentes da paleta padrão: índice 1 → (168, 0, 87), 2 → (0, 168, 87), 3 → (168, 168, 87), 4 → (0, 0, 168), 7 → (135, 136, 143), 8 → (192, 199, 200); 0, 9, 11, 12, 14 e 15 saem com os valores padrão; o fundo da aplicação (índice 17 no port) sai (255, 255, 255) [medido]. A comparação, por isso, mapeia cada índice do port para a cor RGB mais frequente da captura nos pixels com esse índice, e conta como igual o pixel cuja cor bate com o mapeamento. É uma diferença da bancada, não do jogo [hipótese].

## Navegação

Como o original muda a vista da janela de edição e o plano mostrado. Implementação: `engine/src/core/navegacao.rs` (estado `Navegacao`, ações, repetição), `TelaDeJogo` em `tela_jogo.rs` (desenho com qualquer vista, cliques na faixa e nas setas das barras), dados em `data/navegacao.json`; o executável (`simant --save`) só traduz teclas e cliques do SDL. Todas as fontes abaixo são o pseudocódigo do Ghidra (`re_trabalho/decomp/`, só consulta) e a tabela de aceleradores lida do `SIMANTW.EXE`; **nada desta seção foi testado no original** ainda.

### Teclado: aceleradores e `_DoKeyDown`

O laço de mensagens de `WINMAIN` (`1000:3fda`) carrega a tabela de aceleradores `ACCELERATORTABLE` (recurso do `SIMANTW.EXE`, deslocamento 500224 no arquivo) e passa cada mensagem por `TranslateAccelerator` [código]. As entradas que importam aqui (flags 0x09 = `FVIRTKEY | FCONTROL`) [dados do executável]:

| Tecla | Comando | Tratado em | Efeito |
|---|---|---|---|
| Ctrl+↑ | `0xFDA6` | `MAINWNDPROC`, `WM_COMMAND` com byte alto 0xFD e baixo 0xA0–0xAF | `_DoEditScroll(WM_VSCROLL, SB_LINEUP)` |
| Ctrl+↓ | `0xFDA7` | idem | `_DoEditScroll(WM_VSCROLL, SB_LINEDOWN)` |
| Ctrl+← | `0xFDA8` | idem | `_DoEditScroll(WM_HSCROLL, SB_LINEUP)` |
| Ctrl+→ | `0xFDA9` | idem | `_DoEditScroll(WM_HSCROLL, SB_LINEDOWN)` |
| Ctrl+8 | `0xFD26` | `_DoMenuEntry` → `_ProcMenu` (`1000:4fc2`), caso 0x26 | `_SetMapPlane(1)` e `_MakeEditOpen` |
| Ctrl+9 | `0xFD27` | idem, caso 0x27 | `_SetMapPlane(2)` e `_MakeEditOpen` |
| Ctrl+0 | `0xFD28` | idem, caso 0x28 | `_SetMapPlane(3)` e `_MakeEditOpen` |

- Depois do primeiro passo, os comandos 0xFDA6–0xFDA9 repetem `_DoEditScroll` num laço enquanto `GetAsyncKeyState` diz que Ctrl **e** a seta continuam apertados, sem esperar mensagens [código, `MAINWNDPROC` casos 0xA6–0xA9]. A velocidade, portanto, é a do redesenho.
- `_MakeEditOpen` (`1010:0e38`) abre a janela de edição se estiver fechada [código].
- As setas **sem** Ctrl não rolam: `_DoKeyDown` (`1000:0e8c`) move o cursor do mouse 8 pixels (`GetCursorPos`/`SetCursorPos`) [código].
- Pelo comportamento padrão do Windows, `TranslateAccelerator` só aceita a entrada se Shift e Alt estiverem soltos (as flags não pedem esses modificadores) [hipótese: regra da API, não testada no jogo].

### Barras de rolagem (`_DoEditScroll`, `1010:5c6e`)

`MAINWNDPROC` repassa `WM_HSCROLL` (0x114) e `WM_VSCROLL` (0x115) da janela de edição a `_DoEditScroll`; nos códigos `SB_LINEUP`/`SB_LINEDOWN` (setas da barra), repete enquanto o botão esquerdo do mouse continuar apertado (`GetAsyncKeyState(VK_LBUTTON)`) [código, `MAINWNDPROC` (`1000:2930`), casos 0x114/0x115]. A faixa da barra é a de `_ResetEditScrollRange` (`1010:6086`): de 0 a `64 − _editHeight` na vertical e de 0 a `L − _editWidth` na horizontal, com `L` = 0x40 nos planos 2 e 3 e 0x80 nos outros [código]. Com `p` = `_MapPnt.x` (horizontal) ou `_MapPnt.y` (vertical) [código]:

| Código | Efeito |
|---|---|
| 0 `SB_LINEUP` | se `p` > mínimo, `p −= 1`; senão nada |
| 1 `SB_LINEDOWN` | se `p` < máximo, `p += 1`; senão nada |
| 2 `SB_PAGEUP` | `p −= _editWidth` (ou `_editHeight`); se passar do mínimo, fica no mínimo |
| 3 `SB_PAGEDOWN` | `p += _editWidth` (ou `_editHeight`); se passar do máximo, fica no máximo |
| 4 `SB_THUMBPOSITION` | `p` = posição do polegar |
| 5 `SB_THUMBTRACK` | idem, só se `_OptionStates ≠ 0` (arrasto ao vivo) |
| 6 `SB_TOP` / 7 `SB_BOTTOM` | o pseudocódigo grava o mínimo (ou máximo) em **x e y** ao mesmo tempo [código; pode ser leitura errada do Ghidra, em aberto] |

- Nas linhas, o mapa já desenhado é deslocado (`_EditScroll*Asm` ou `_DoEditScrollLine` + `_EditScroll*Color`) e só a faixa nova é desenhada; nos outros códigos, `_UpdateEdit` redesenha com `ScrollWindow` [código]. O resultado na tela é o mesmo: a vista começa em `_MapPnt`.
- `_DoEditScroll` **não** atualiza o ponto lembrado do plano (`_AMapPnt` etc.) [código]. Quem atualiza são os `_goStep*` (`1030:6efe`ss.), que andam a vista um passo e gravam `_editWidth / 2 + _MapPnt` [código]. Porém nenhuma função do pseudocódigo chama os `_goStep*` nem os `_Finish*Step` [código: busca por referências em `Code*.c`; chamada indireta não procurada]: [hipótese] são restos da versão Mac, e a vista só segue a amarela por `_GotoMyAnt` (seção "Simulação ao vivo").
- Os símbolos `_ScrollUp`, `_ScrollDown`, `_ScrollLeft`, `_ScrollRight`, `_ScrollUpS`, `_ScrollDownS` e `_ScrollRightS` (`1000:96b8`–`96c6`) são funções vazias na versão Windows (só `return`); são chamadas pelos `_goStep*` quando o passo é 1 [código]. Provavelmente restos da versão Mac [hipótese].

### Faixa: botões de plano (`_ProcMapRibbonEvent`, `1010:c3d4`)

Os eventos de objeto chegam por `_DoEvent` (`1000:0bc2`), que despacha pelo byte alto do id (a janela): 0x22 (janela 34, faixa do mapa) vai a `_ProcMapRibbonEvent`, 0x23 (janela 35, faixa do quintal) a `_ProcYardRibbonEvent` [código]. Na faixa 34 [código]:

| Id | Objeto | Efeito |
|---|---|---|
| `0x2202` | 2 (botão Yard) | abre a janela do mapa (`_win_Open(0x100)`) se nem ela nem a do quintal estiverem abertas e põe a do quintal na frente (`_MapToYard`) ou a traz ao topo; **não muda a janela de edição** |
| `0x2203` | `ribToBNest` | `_ClearMapScentButtons`, `_SetMapPlane(2)` |
| `0x2204` | `ribToRNest` | `_ClearMapScentButtons`, `_SetMapPlane(3)` |
| `0x2205` | `ribToSurface` | `_ClearMapScentButtons`, `_SetMapPlane(1)` |

A janela do mapa (`_ProcMapEvent`, `1010:c222`) tem botões equivalentes (0x106–0x108 → planos 1 a 3) [código]; ela ainda não existe no port. Não foi estudado em que fase do clique (apertar ou soltar) o evento é gerado [em aberto]; o port age ao apertar.

### Troca de plano (`_SetMapPlane`, `1000:a5ca`; `_SetMapPlaneLocation`, `1000:a386`)

`_SetMapPlane(n)` [código]:

1. se `n = 0`, guarda o plano atual em `_OldMapPlane`; senão invalida o mapa pequeno (`_InvalEuMap`);
2. `_MapPlane = n`;
3. lê o ponto lembrado do plano `n` (`_YMapPnt`, `_AMapPnt`, `_BMapPnt` ou `_RMapPnt`);
4. `_SetMapPlaneLocation(n, ponto)`: repete os passos 1 e 2; no plano ≠ 0, `_CenterEdit(ponto)` (com `_UpdateEdit` se andou) e redesenha o mapa; no plano 0, grava em `_YMapPnt` (`DS:789e`) o par de `DS:7a60` (não identificado); depois `_SetMapModeAnt` e a seleção do botão da faixa (seção "Faixa", acima);
5. no plano ≠ 0, `_CenterEdit(ponto)` **de novo** e redesenha o mapa.

Consequências:

- A segunda `_CenterEdit` completa o passo que a primeira pode ter perdido no eixo menor (truncamento descrito em "Centralizar").
- `_CenterEdit` parte do `_MapPnt` do plano anterior, já com os limites do plano novo.
- Escolher o plano que já está na tela também centraliza: a vista volta ao ponto lembrado, desfazendo a rolagem feita pelas barras (que não atualiza o ponto) [código; não testado no original].

`_GotoMapPoint(n, x, y)` (`1000:a52a`): se `n` é o plano atual e > 0, só `_CenterEdit(x, y)`; senão `_SetMapPlaneLocation(n, x, y)` (troca de plano centralizando em (x, y), sem ler o ponto lembrado) [código]. É usada, por exemplo, por cliques no mapa pequeno; o port ainda não tem quem a chame.

### O que o port faz

- Estado: `Navegacao { plano_mapa, vista, pontos }`, iniciado do save. Com `posicao_inicial: "ao_carregar"` (padrão em `data/navegacao.json`) a vista é a de depois do "Load Game" (seção "Posição ao carregar") e o ponto do plano de `_MePlane` recebe `(_MeLocX, _MeLocY)`, como em `_DoLoadInitializations`; com `"salva"`, ficam os valores gravados.
- Ações: `linha_*` e `pagina_*` seguem a tabela de `_DoEditScroll` (sem mexer nos pontos lembrados); `superficie`, `ninho_preto` e `ninho_vermelho` seguem `_SetMapPlane(1/2/3)` com as duas centralizações. O resultado é limitado com `_BoundEditPosition` também quando `_CenterEdit` não anda, como faz o desenho (`FUN_1010_16d4`).
- Entradas ligadas por padrão: os aceleradores do original (Ctrl+setas, Ctrl+8/9/0), os botões `ribToSurface`, `ribToBNest` e `ribToRNest` da faixa e as setas das barras de rolagem (clique, com repetição enquanto o botão fica apertado). **Extensões do port**, marcadas em `data/navegacao.json` e em [divergencias.md](../../divergencias.md): setas sem Ctrl rolam (no original movem o cursor) e PageUp/PageDown rolam uma página na vertical.
- Ainda não implementado: polegar e trilho das barras (arrasto, página pelo clique no trilho, `SB_TOP/BOTTOM`), botão Yard e plano 0 (janela do mapa/quintal), `_ClearMapScentButtons` e `_GotoMapPoint` vindo do mapa pequeno. A vista seguindo a amarela está na seção seguinte.

## Simulação ao vivo

Como o original roda a partida enquanto a tela de jogo está aberta, e o que o port reproduz. Regras do temporizador, da velocidade e da pausa: [simulacao-visao-geral.md](../regras/simulacao-visao-geral.md) §1–4; controles da amarela: [formiga-amarela.md](../regras/formiga-amarela.md) §11. Implementação: `engine/src/core/partida.rs` (`Partida`, sem janela), dados em `data/partida.json`; o executável (`simant --save` / `--novo-jogo`) só traduz SDL em chamadas da `Partida` e redesenha.

### Relógio

- Fonte de tempo: o `SetTimer` de 17 ms (`MYTIMERFUNC`), 170 ms com o aplicativo sem foco (`WM_ACTIVATEAPP`) [código]. No Windows 3.1 o intervalo é arredondado para a resolução do relógio do sistema (55 ms): o de 17 ms dá **uma `WM_TIMER` a cada 55 ms** [verificado na bancada, simulacao-visao-geral.md §3.1]; o de 170 ms, 220 ms [hipótese]. No port, `Partida::avancar(ms)` transforma o tempo real em mensagens a cada `intervalo_ms()` = `intervalo_mensagem_ms` (17) ou `intervalo_sem_foco_ms` (170) arredondado para cima até `resolucao_timer_ms` (55), guardando o resto. Como as `WM_TIMER` não se acumulam, no máximo `max_mensagens_por_avanco` (3) mensagens saem de uma chamada; o tempo excedente é descartado [escolha do port].
- O laço de `MYTIMERFUNC` (simulacao-visao-geral.md §2): uma mensagem fora do laço abre uma chamada (atraso, DGROUP `0x02EE`, + 1); dentro dele, é drenada no fim da volta em curso (atraso + 1). Entre duas mensagens rodam `voltas_por_periodo[_GameSpeed]` voltas; cada uma soma 1 a `_gameCycles` (`Estado::ciclos_jogo`), passa por `Relogio::liberar_tick` (pausa, §4: roda se não está pausado, ou se `_MeMoveMe` ≠ 0 e `_CurGameTool` < 10; velocidade, §3: `_SpeedDelayVals` = 6, 2, 0, −1 de `data/simulacao.json`, roda se o limite ≤ 0 ou ≤ atraso e zera o atraso) e termina com o teste de saída do original (drenadas > 1 e (voltas > 2 ou drenadas > 2)). Cada chamada dura, portanto, 3 mensagens. Resultado: slow = 1 tick a cada 6 mensagens (3,03/s; bancada 3,0), normal = 1 a cada 3 (6,06/s; bancada 5,85–6,0), fast = 11 por chamada com o padrão 5 voltas (66,7/s; bancada 55–66), ultra = 15 por chamada com 7 voltas (90,9/s; bancada 91) [o número de voltas é dado: no original depende da CPU].
- `_gameCycles` conta as voltas do laço, rode ou não o tick, como no original: em normal passa de 100 entre dois ticks (o pássaro é testado em todo tick, testes-tick.md), em fast/ultra anda 1 por tick (bancada). O relógio do Mac vem de `Estado::relogio_mac`, que a `Partida` preenche antes de cada tick com ⌊ms reais acumulados / 55⌋ × 3 (`_MacTickCount`, quintal.md "Relógios"); em normal anda 9 por tick, como na bancada.
- Ignorado pelo port [divergencias.md]: a saída do laço quando o botão esquerdo do mouse foi apertado (`GetAsyncKeyState(VK_LBUTTON) & 1`).
- O atraso continua contando durante a pausa: ao despausar, o primeiro tick sai na mensagem seguinte [código: o +1 vem antes do teste].
- Redesenho (`_UpdateWindows`, `1000:2224`): todo tick em slow/normal, 1 a cada 2 em fast e 1 a cada 4 em ultra (`redesenho_mapa_a_cada`); sem foco, só quando o contador de quadros é múltiplo de 8 (`redesenho_sem_foco_a_cada`) [código]. [hipótese] O port incrementa o contador antes do teste (no original a ordem não foi conferida). Rolagem, troca de plano, cliques e teclas pedem redesenho na hora.

### Fila de eventos

As regras não chamam janela nem som: deixam pedidos em `Estado::eventos`. Depois de cada tick e de cada entrada do jogador, a `Partida` esvazia essa fila:

- `CentralizarNaAmarela` (`_GotoMyAnt`, `1000:a764`, pedido por `rules/amarela.rs` com AutoTrack ligado a cada passo de `_DoAntMoveY`, com Home, Backspace sem ordem, clique na própria amarela, ordem impossível, morte...) é tratado na hora: no Experimental nada (o original toca o som de erro); se a amarela está no plano mostrado (> 0), `_CenterEdit(_MeLoc)`; senão, `_SetMapPlaneLocation(_MePlane, _MeLoc)`, que troca o plano e centraliza **uma** vez, sem ler nem gravar o ponto lembrado [código].
- `Mensagem(id)`, `Musica(id)` e `DialogoFimDeJogo` vão para a fila da interface (`Partida::retirar_eventos`, escritos no terminal; música sem som). `Mensagem` e `DialogoFimDeJogo` também viram **avisos modais** (`Partida::avisos`): o quadro de mensagem e o diálogo de fim de jogo, que param a simulação até serem fechados, como `_StopSimulation` ([mensagens.md](mensagens.md)). `_EndGameDialog` é pedido a cada tick enquanto `_IsGameOver` ≠ 0; a `Partida` repassa só o primeiro.
- Depois de cada tick, o conselho de `_Feedback` (`rules::conselhos::conselho`) vai para a mensagem da faixa (`_EditMessage`, mensagens.md §2).

### Vista e pontos lembrados

A `Navegacao` é a dona do plano mostrado e da vista. Antes de cada tick e de cada entrada, `_MapPlane`, `_MapPnt` e os pontos lembrados vão para o `Estado` (as regras leem `_MapPlane`, por exemplo ao decidir se centralizam quando a amarela entra num ninho); depois do tick, os pontos lembrados voltam (as regras podem gravá-los).

A vista segue a amarela **só** pelo `_GotoMyAnt`: com AutoTrack (Options → AutoTrack, Ctrl+G, `_OptionStates[0]`) ligado, a cada passo de uma ordem; desligado (padrão de um jogo novo e do `QUICK2.ANT`), a vista fica parada e o jogador rola ou aperta Home. Os `_goStep*` não têm quem os chame no pseudocódigo (seção "Barras de rolagem").

### Entradas do jogador

| Entrada | Original | Port |
|---|---|---|
| Clique esquerdo na área do mapa | `_processEdit` com 0x200; duplo clique (`WM_LBUTTONDBLCLK`) com 0x2000 [código] | célula = `((x − área.x) / 16 + _MapPnt.x, (y − área.y) / 16 + _MapPnt.y)` (com a volta de `FUN_1010_6250` depois da linha 63), só dentro da área cliente visível; `clicks` = 1 → `clicar_no_mapa(..., duplo = falso)`, `clicks` = 2 → duplo |
| Clique direito | 0x800 (simples) ou 0x4000 (duplo), tratados como esquerdo em `_processEdit` [código] | igual |
| Clique nas setas das barras e nos botões da faixa | seção "Navegação" | como antes; o clique nunca vira ordem |
| Espaço / Insert / Delete | clique esquerdo / direito na posição do cursor; duplo se a mensagem seguinte é a mesma tecla [código] | posição do último movimento do mouse; duplo se a mesma tecla vier em até `duplo_clique_teclado_ms` (250) [escolha do port] |
| Teclas de comando (1–4, 0, X/F9, J e NumPad * com Tab, Backspace, Home, NumPad 0–4) | `_YellowCommandKey`, depois dos aceleradores [código] | `teclas_amarela` (nome da tecla do SDL → código de tecla virtual) → `tecla_de_comando`; Tab lido do estado do teclado |
| Shift+0 / Shift+1..4 (também Shift + NumPad) | Speed → Pause (0x41, alterna `_GamePaused`; ao despausar encerra as ferramentas 10/11; "Game Paused" na faixa) e Slow/Normal/Fast/Ultra (0x43–0x46, `_GameSpeed` = 0..3, **sem** mensagem na faixa: só o botão de velocidade escreve "Game speed now …") [código, `_ProcMenu`; bancada, simulacao-visao-geral.md §3.2] | `teclas` → `AcaoPartida`; a velocidade aparece só no título da janela [extensão do port] |
| Ctrl+G | Options → AutoTrack (0x31, alterna `_OptionStates[0]`) | igual |

Ordem no port, como no original: aceleradores de `partida.json`, aceleradores de `navegacao.json`, `_YellowCommandKey`, teclas que imitam o mouse. O título da janela do port mostra velocidade, pausa e AutoTrack.

Avisos e botões ([mensagens.md](mensagens.md)):

| Entrada | Original | Port |
|---|---|---|
| Ctrl+S / Ctrl+A | File → Save / Save As (`_SaveGame`, diálogo de arquivo) | grava o `.ANT` no arquivo da partida / num nome novo (`--salvar` ou `SALVOnn.ANT`), sem diálogo |
| Botões `ribUserDef1A`–`1H` da faixa | `_DoUserButton` (pausa, velocidade com "Game speed now …", opções, ir até a amarela, salvar; segurando: menu de ícones) | clique simples igual, exceto as funções que abrem janelas; sem o menu de ícones |
| Pausa (Shift+0 ou botão) | `_SetPause`: "Game Paused" na faixa, permanente; despausar limpa | igual (mensagens.md §2.3) |
| Clique simples no mapa | espera ~6 unidades de `_TickCount` (330 ms) sem simular; segurando, `_AntMenu`/Examine | espera igual; sem os menus |
| Quadro de mensagem / fim de jogo na tela | só clique no quadro (ou Espaço no quadro) fecha; a simulação fica parada; o quadro fecha sozinho em ~14,85 s | igual; ao fechar o fim de jogo, o port abre o "Select a Game" como o original |

### Jogo novo

`--novo-jogo` cria um Quick Game como `_initStuff` + `_NewGame`: geradores semeados pelo relógio (`_SeedSRand`/`_SeedRRand`; no port, a mesma semente nas duas leituras do relógio: a de `--semente N` ou os milissegundos do relógio do sistema), `Mundo::novo_jogo(Rapido)` e `Estado::de_mundo`. A tela abre como em `_NewGame` passos 4 e 6: plano da amarela e vista centralizada nela (o mesmo cálculo da posição "ao carregar"). A janela não é maximizada nem há as janelas de casta/modo.

### Fora do laço

`--ticks N` roda N ticks seguidos (`Partida::rodar_tick`, sem relógio, velocidade nem pausa) e sai; com `--exportar-tela-jogo`, grava a tela depois deles. Serve de teste de regressão sem janela (ex.: 50 ticks do `QUICK2.ANT`: `GameTime` 78 → 128).

## Verificação

Exemplo `tela_jogo` com o `QUICK2.ANT` e a captura `qi_14_salvo.png`, tabela de posições 9/0, comparando a área cliente (linhas 38 a 479 da captura) e ignorando o diálogo "Saved correctly" (176 ≤ x < 472, 171 ≤ y < 310 na tela) [medido, 2026-10-16]:

| Região (coordenadas de tela) | Pixels iguais | O que difere |
|---|---|---|
| mapa visível, (24, 158) a (423, 461) | **100%** (87.128 de 87.128) | nada: chão, túnel, grama e a vida (formiga amarela, metade da rainha, ovo) que aparece abaixo do diálogo |
| janela de edição inteira, (20, 135) a (443, 479) | 99,33% | texto do título, desenhos da caixa do menu de sistema e do botão de maximizar, setas e polegares das barras de rolagem |
| faixa, linhas 38 a 110 | 93,27% | barras de população/saúde e botões `ribUserDef*` |
| área cliente inteira | 78,24% | o acima, mais as janelas `mapWin` e de controle de casta atrás da janela de edição, que o port ainda não desenha |

Mapeamento de cores inferido (índice do port → cor da captura) igual ao da seção anterior. Imagens geradas em `re_trabalho/tela_jogo/` (fora do git): `tela_jogo.png` (port), `tela_jogo_640x480_mapeada.png`, `diferencas.png` e recortes ampliados da faixa e das formigas.

O teste `engine/tests/tela_jogo_real.rs` trava o resultado com um SHA-256 dos índices e confere a reconstrução de `_LifeA/B/R` do `QUICK2.ANT` (sem precisar dos bancos do jogo) e a vista salva (22, 0) e ao carregar (21, 0).

Navegação (2026-10-16): testes unitários em `navegacao.rs` e `tela_jogo.rs`; `tela_jogo_real.rs` confere, com o `QUICK2.ANT`, a navegação ao carregar (vista (21, 0), ponto do ninho preto (34, 9)), que os botões de `data/navegacao.json` existem na faixa e são achados pelo clique, e que os três planos desenham sem pendências. O executável exportou as telas do ninho preto (21, 0), da superfície (51, 23) e do ninho vermelho, conferidas a olho, e abriu a janela SDL por 2 s (`--sair-apos-ms`, `--capturar-janela`). **A navegação não foi comparada com o original** (falta capturar a vista depois de Ctrl+setas e da troca de plano na bancada).

Simulação ao vivo (2026-10-17; ritmo revisto em 2026-10-18): testes unitários em `core/partida.rs` (timer de 17 ms vira 55 ms, com resto, limite de mensagens, taxa por velocidade 2/6/66/90 ticks em 18 mensagens, `_gameCycles` por volta, pausa e exceção da amarela em movimento, despausar encerra a ferramenta 10, AutoTrack e velocidade pelos aceleradores, 170 ms sem foco, redesenho em fast/ultra, centralizar no mesmo plano e em outro plano, fim de jogo repassado uma vez) e em `tela_jogo.rs` (área cliente calculada = desenhada); `engine/tests/partida_quick2.rs` (50 ticks do `QUICK2.ANT`, `GameTime` + 50; 3,3 s em normal = 20 ticks; clique simples dá a ordem de andar). O executável rodou `--save QUICK2.ANT --ticks 50` (78 → 128), `--novo-jogo --semente 1234 --ticks 300` e `--semente 77 --acoes superficie --ticks 2000` (telas conferidas a olho, formigas espalhadas pela superfície) e abriu a janela por 3 s. **Comparado com o original (2026-10-18):** a taxa de ticks por velocidade (simulacao-visao-geral.md §3.1; antes o port rodava cerca de 29 ticks/s em normal, cinco vezes o original). AutoTrack, Espaço como clique e o comportamento do clique simples (espera de 6 ticks) ficam para a bancada.

Avisos (2026-10-17): os quadros de mensagem (1 e 2 linhas) e a mensagem da faixa ("Game Paused", "Game speed now fast.") batem **100%** com as capturas da bancada; o diálogo de fim de jogo, 97,95%; a pontuação do C-10 (19785) bate. Detalhes em [mensagens.md](mensagens.md) §8.

**Não verificado contra o original:** a vista ao carregar um save (falta uma captura depois de "Load Game"), a superfície e o ninho vermelho na janela de edição, a formiga amarela rainha (0xFE) e a moldura em outras resoluções (800×600 do DOSBox-X).
