# Tela de jogo (faixa + janela de edição)

Este documento descreve a tela que aparece durante uma partida: a faixa de ferramentas (janela 34, `mapRibbon`) e a **janela de edição** (janela 0, `editWin`), que mostra o mapa da superfície ou de um ninho com as formigas por cima. As outras janelas abertas numa partida (mapa, controle de castas e de modos) ficam para depois.

Implementação: `engine/src/core/vida.rs` (camada de vida), `engine/src/core/edicao.rs` (vista da janela de edição), `engine/src/core/tela_jogo.rs` (`compor_tela_de_jogo`, moldura do Windows), dados em `data/tela_jogo.json`. Exemplo: `cargo run -p engine --example tela_jogo -- <pasta do jogo> <save .ANT> <pasta de saída> [captura.png]`.

**Fontes:** pseudocódigo do Ghidra em `re_trabalho/decomp/` (só consulta; nomes do `SIMANTW.SYM`), o `QUICK2.ANT` e a captura `re_trabalho/qi_14_salvo.png`, tirada pela bancada QEMU ([bancada-qemu.md](../../bancada-qemu.md)) na mesma sessão em que o save foi gravado. Legenda: **[código]** = visto no pseudocódigo, com o endereço; **[save]** = visto nos dados; **[medido]** = medido pixel a pixel na captura; **[hipótese]** = não confirmado.

## Janelas abertas num Quick Game

`_NewGame` (`1000:d6dc`), depois da escolha no "Select a Game" (o botão Quick Game devolve 0x206, que vira o tipo de jogo 0) [código]:

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

`editTileArea` vai de (16, 42) a (416, 345) no `WINGANT`. Com a faixa, 401 × 304 pixels, logo **26 × 19 tiles**. A janela recorta a última coluna: só 25 colunas (400 pixels) aparecem antes da barra de rolagem [medido].

### Posição ao carregar e na captura

- Ao carregar, `_DoLoadInitializations` grava `(_MeLocX, _MeLocY)` no ponto lembrado do plano `_MePlane`, e `_SetDefaultWindows` → `_SetMapPlane(_MapPlane)` centraliza a vista nele; se `_MePlane = _MapPlane`, centraliza de novo em `(_MeLocX, _MeLocY)` [código]. Com 26 colunas, o `QUICK2.ANT` carregado começaria em x = 34 − 13 = 21 [hipótese: não capturado].
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

**Não verificado contra o original:** a vista ao carregar um save (falta uma captura depois de "Load Game"), a superfície e o ninho vermelho na janela de edição, a formiga amarela rainha (0xFE) e a moldura em outras resoluções (800×600 do DOSBox-X).
