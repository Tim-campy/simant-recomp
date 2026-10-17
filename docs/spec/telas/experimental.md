# Modo Experimental: menu e ferramentas de edição

Este documento descreve como o original escolhe e aplica as ferramentas do Experimental (pôr e tirar formigas, comida, pedras, muros, cheiro, inseticida, cavar e encher), como um Quick Game vira Experimental e como o Experimental vira jogo ("Start Game"). A geração do mundo do Experimental está em [../regras/geracao-mundo.md](../regras/geracao-mundo.md) (§2, tabela por tipo de jogo).

Implementação: `engine/src/rules/experimental.rs` (regras, sem janela), dados em `data/experimental.json`, testes em `engine/tests/regras_experimental.rs`. A interface (menus, cursor, janela de exame) é do executável.

**Fontes e legenda:** pseudocódigo do Ghidra em `re_trabalho/decomp/Code1.c`, `Code3.c`, `Code5.c`, `Code7.c` (só consulta; nomes do `SIMANTW.SYM`); desmontagem com capstone onde o Ghidra marcou blocos inalcançáveis; bytes do `SIMANTW.EXE`; nomes de objetos do banco `SHARED` (tipo 1). **[código]** = visto no pseudocódigo/desmontagem, com o endereço; **[original]** = verificado na bancada QEMU; **[hipótese]**; **[em aberto]**.

## 1. Variáveis

| Símbolo | Endereço | No save | No port |
|---|---|---|---|
| `_CurGameType` | `9:80b4` | sim | `Estado::partida.tipo_jogo` (0 Tutorial, 1 Quick, 2 Full, 3 Experimental) |
| `_CurExpTool` | `9:79e2` | sim | `Estado::partida.ferramenta_experimental` (0–7, tabela da seção 3) |
| `_ExpSubStates[8]` | `8:8600` (bytes) | não | `ControleExperimental::subopcoes`; valores iniciais do executável `FF 00 00 00 00 00 00 FF` (só 1–6 são usados) |
| `_ExpLastPnt` | `9:7d46` (x, y) | não | `ControleExperimental::ultimo_ponto` |
| `_MeMode` | `9:9fe8` | sim | `Estado::amarela.modo` (0 formiga, 1 aranha, 2 Experimental: a amarela não existe no mapa) |

[código] `_NewGame` zera `_CurExpTool` ao começar um Experimental (`1000:d6dc`) e `_InitSimVars` também (`1030:5a70`).

## 2. Como a ferramenta é escolhida

Não há menu Edit nem aceleradores: a barra de menus do jogo é File, Window, View, Options, Speed [original: capturas da bancada], e nenhuma entrada da `ACCELERATORTABLE` (deslocamento 500224 do `SIMANTW.EXE`) leva a `_SetExpTool` ou aos menus de ferramenta [código + bytes]. A tabela completa: Ctrl+N/L/S/A/Q (File: `0xFD03`–`0xFD08`), Ctrl+E/M/B/C/H/I/T (abrir janelas, `0xFD11`–`0xFD17`: edição, mapa/quintal, modos, castas, história, informação, placar), Ctrl+Y/5/6/7 (`0xFD21`–`0xFD24`, modo do quintal), Ctrl+8/9/0 (planos), Ctrl+G/J/K (`0xFD31`–`0xFD33`, Options), Shift+0..4 e Shift+NumPad (Speed), e `0xFDA0`–`0xFDAC` (Ctrl+S/M, Ctrl+Espaço, Ctrl+U/K, Ctrl+P, Ctrl+A, Ctrl+Z, Ctrl+setas, Ctrl+U/V/F: manipulação de janelas e depuração em `MAINWNDPROC`).

O menu de ferramentas é um menu "de proximidade" (`_win_DoProxMenu`), aberto por três botões [código]:

| Botão | Objeto | Rotina |
|---|---|---|
| ferramentas da janela de edição | `0x0007` `editTools` | `_EditToolsMenu` (`1010:67c6`), via `_ProcEditEvent` |
| ferramentas da janela do mapa | `0x0117` `mapTool` | `_MapToolsMenu` (`1010:dc1e`), via `_ProcMapEvent` |
| ferramentas da faixa | `0x220A` `ribTools` | `_RibbonToolsMenu` (`1010:de76`), via `_ProcMapRibbonEvent` |

O que o botão faz depende de `_CurGameType` [código]:

- **Quick Game (1):** abre a janela 10 (`editToExp`, uma figura e o item `editToExpMode`). Escolhido o item, o jogo vira Experimental (seção 5).
- **Full Game (2):** abre a janela 11 (`editNoExpMenu`, só uma figura) e nada muda.
- **Tutorial (0):** nada.
- **Experimental (3):** `_DoExpMenu` (`1010:6ee2`) abre a janela 9 (`editToolsMenu`) com o item da ferramenta atual realçado (`__win_SetProxItem(0x902 + _CurExpTool)`), pausa o jogo enquanto o menu está aberto (`_SetPause(1)`, e depois devolve o valor anterior) e devolve a ferramenta escolhida (−1 = nada). Se o item tem submenu e algo foi escolhido nele, grava `_ExpSubStates[ferramenta] = item do submenu − 2`. Depois: ferramenta 7 → "Start Game" (seção 6); em todos os casos `_CurExpTool = ferramenta` (inclusive 7) e o desenho do botão muda para o bitmap da ferramenta.

| `_CurExpTool` | Item (janela 9) | Submenu (janela) | `_ExpSubStates` | Rotina |
|---|---|---|---|---|
| 0 | `editExpInfo` | — | — | `_MagnifyMenu` |
| 1 | `editExpBarrier` | `wallRockMenu` (12) | 0 `toolWalls`, 1 `toolRocks` | `_DropWall` |
| 2 | `editExpDig` | `digFillMenu` (15) | 0 `toolDig`, 1 `toolFill` | `_ExpDig` |
| 3 | `editExpAddAnt` | `blackRedMenu` (16) | 0 `toolBlack`, 1 `toolRed` | `_ExpAddAnt` |
| 4 | `editExpAddFood` | `foodMenu` (17) | 0 `onePiece`, 1 `twentyPieces` | `_IncFoodHere` |
| 5 | `editExpTrails` | `placeScentMenu` (13) | 0 ninho preto, 1 trilha preta, 2 ninho vermelho, 3 trilha vermelha, 4 alarme | `_ExpIncSmell` |
| 6 | `editExpInsecticide` | `cideMenu` (14) | 0 `toolSpray`, 1 `toolStream` | `_ExpKillAnts` |
| 7 | `editExpStartGame` | — | — | seção 6 |

A ligação item → janela de submenu é pelos nomes dos objetos; o código só vê o índice do item [hipótese quanto ao mecanismo interno de `_win_DoProxMenu`, não estudado]. O cursor do mouse sobre a janela de edição (ou sobre o mapa com Shift) muda conforme a ferramenta: `_magCursor`, `_rockCursor`, `_digCursor`, `_antCursor`, `_foodCursor`, `_dropCursor`, `_sprayCursor` (`MAINWNDPROC`, `WM_SETCURSOR`) [código].

## 3. Clique e arrasto: `_processExp` (`1010:7022`)

No Experimental, todo clique na área do mapa da janela de edição (esquerdo ou direito, simples ou duplo) vai para `_processExp(x, y)` em vez de virar ordem da amarela (`_processEdit`, `1000:6a38`) [código]. Também vale na janela do mapa pequeno, com outra conversão de coordenadas.

1. `_ExpLastPnt = (x, y)`; se `_IsValidLocation(_MapPlane, x, y)`, `_DoTool(x, y)`.
2. Enquanto o botão continua apertado (`_myButton`), a cada volta: lê o cursor; se mudou de célula, `_ExpLastPnt` = célula anterior e `_DoTool(nova)` (se válida); e, **com `_CurExpTool` > 4** (cheiro e inseticida), `_DoTool` de novo na célula do cursor em toda volta, mesmo parado.
3. Cada volta redesenha a janela de edição e/ou o mapa; o ritmo das voltas é o do redesenho (tempo real), não o tick.

`_IsValidLocation(plano, x, y)` (`1020:56da`): x em 0..127 nos planos 0 e 1, em 0..63 nos ninhos; y em 0..63 [código].

`_DoTool(x, y)` (`1010:727e`) despacha por `_CurExpTool` [código]. Ctrl apertado (`GetAsyncKeyState(VK_CONTROL)`, lido dentro de cada rotina) inverte muro, cavar, comida e cheiro.

## 4. As ferramentas

### 4.0 Info (`_MagnifyMenu`, `1010:6b4c`)

`_DoTool` sempre chama `_MagnifyMenu`, porque `_GetLife` devolve 0xFFFF para célula vazia; é `_MagnifyMenu` que só abre o quadro se o byte de vida ≠ 0 [código]. O quadro (janela 29 `examineWin`, `_win_DrawExamineWindow` `1010:69c0`) fica aberto enquanto o botão está apertado e mostra três textos [código]:

- `examStr1`: grupo `g = (vida & 0x78) >> 3`; com g = 0 (ovo), o estágio `vida & 7` na lista SHARED 4/1100 ("Weird Egg", "New Egg", "Egg", "1st-Instar"…); senão a lista 4/1101 ("Egg Tender", "Worker", …, "Queen").
- `examStr2`: o modo M da formiga achada na lista do plano (`_FindInAList`, `_FindInBList/RList` com o T) na lista 4/1102 ("Bored", "Nesting", "Foraging"…); 0 se não achou.
- `examStr3`: a lista 4/1103 pelo grupo ("Nothing", "Egg", "Food", "Dirt"…).

Não muda o estado. No port: `examinar(e, plano, x, y) -> Option<Exame>`.

### 4.1 Barreira (`_DropWall`, `1010:7450`, e `_ConnectAll`, `1010:805a`)

Só nos planos 0 e 1. Percorre de `_ExpLastPnt` até (x, y): em cada passo, `k = _GetDir(ponto, destino)` e o ponto anda **só em x** se `_Dx9[k]` ≠ 0, senão em y (caminho em "L"); para depois de tratar o destino. Para logo se a célula sai do mapa (`_IsValidA`). Em cada célula [código]:

- **Ctrl:** se 0x50 < tile < 0x68 (pedrinha, pedra, muro), tile = `SRand1(16)`; depois `_ConnectAll`. Sem som.
- **Muros (subopção 0):** se tile < 0x50 e `_LifeA` = 0, tile = 0x60; depois `_ConnectAll` (sempre); som 0x28.
- **Pedrinhas (subopção 1):** se tile < 0x50 e `_LifeA` = 0, tile = 0x51 + `SRand1(3)`; depois **sempre** `SRand1(10000)` para o parâmetro do som 10 (o sorteio conta no gerador S).

`_ConnectAll(x, y)`: para a célula e as vizinhas de cima, direita, baixo e esquerda (as vizinhas só dentro do mapa), se o tile é muro (0x60..0x67), grava `conexao_muro[m]`, com `m` = esquerda × 8 + baixo × 4 + direita × 2 + cima (1 se a vizinha é muro; fora do mapa não é). Tabela (`DGROUP:0x1bce`): `60 64 65 66 62 61 62 61 63 63 60 60 67 67 67 67` [bytes do executável].

### 4.2 Cavar e encher (`_ExpDig`, `1010:756e`)

Encher = subopção 1, invertido por Ctrl. Percorre de `_ExpLastPnt` até (x, y) andando nos dois eixos por passo (`_Dx9/_Dy9`, diagonal) [código]:

- **Superfície, cavar:** `_DigMyNewHole(x, y)` (buraco novo com 3 × 3 livre; ver [formiga-amarela.md](../regras/formiga-amarela.md)). Conseguiu: som 0x13. Não: som de erro 1 e o aviso 22 da lista 4/1010 ("Not enough room here for a hole.").
- **Superfície, encher:** som de erro e o aviso 21 ("Fill tool unavailable on the surface.").
- **Ninho, cavar:** se `_IsItDigable` (terra ou capim) e y ≠ 0: `_DigTileB/R(x, y)`; se y = 1, também `_MakeNewHoleB/R(x)` (buraco na superfície para a coluna x). Som 0x13.
- **Ninho, encher:** se **não** `_IsItDigable` e y ≠ 0: as formigas da lista do ninho naquela célula (todas as vagas em uso com T ≠ 0) recebem T = 0, sem cadáver; tile = 0x2E (terra maciça) e vida = 0; se `_TilesDug?` > 1, `_TileTotX?/Y?` −= x/y (32 bits; negativo vira 0) e `_TilesDug?` −= 1, **sem** recalcular o centro de massa; `_SmoothEdges?` nas vizinhas de cima, direita, baixo e esquerda; `_ExitMap?` da célula = 0. Som 0x12. Qualquer tile que não é terra/capim é enchido (túnel, comida, passagem).

### 4.3 Pôr formiga (`_ExpAddAnt`, `1010:79fc`)

[código] `T = SRand1(8) + 0x10`, mais 0x80 se a subopção é 1 (vermelha); o sorteio acontece sempre. Superfície: só se `_IsClearTile(1, x, y)`; `_AddAntToAList(x, y, T, M = 2, S = 0)`. Ninho: nunca na linha 0; se o tile > 0x1B, `_DigTileB/R(x, y)`; M = 2 no ninho da própria cor e 7 no da outra; `_AddAntToBList/RList`. A formiga entra na vaga `_ListIndex?` (lista cheia: descartada) e grava a vida. Som 0x1C (menos quando sai antes: célula ocupada ou linha 0).

### 4.4 Comida (`_IncFoodHere`, `1010:7f3a`)

Um pedaço (subopção 0): `_IncFoodHere(x, y)` e, se gravou, som 0x1D. Vinte pedaços (1): som 0x20 e 20 vezes `(x + SRand1(9) − 4, y + SRand1(9) − 4)` (x sorteado antes de y), `_IncFoodHere` em cada célula válida [código].

`_IncFoodHere`, com o limiar L = 0x48 na superfície (planos 0 e 1) e 0x10 nos ninhos [código]:

- Sem Ctrl, se o tile ≤ L + 2: tile ≥ L → +1; tile < 0x18 → L; outro tile (objeto entre 0x18 e L − 1) fica como está, **mas conta**. Soma 1 em `_FoodA` (superfície), `_FoodB` (plano 2) ou `_FoodR` (plano 3). Grava e devolve 1.
- Com Ctrl, se L ≤ tile ≤ L + 3: tile = `SRand1(8)`; só na superfície desconta `_FoodA` (com `_FoodA` < 1, desconta `_FoodR` se > 0: o teste de `_MapPlane == 2` que escolheria `_FoodB` nunca vale ali); nos ninhos não desconta nada. Grava e devolve 1.
- Senão devolve 0 sem mudar nada.

### 4.5 Cheiro (`_ExpIncSmell`, `1010:7b9e`)

Só nos planos 0 e 1. Célula (x ≫ 1, y ≫ 1) do mapa de cheiro da subopção (`_PherMapBN`, `BT`, `RN`, `RT`, `A`): +70, ou −40 com Ctrl, limitado a 0..255; depois `_SmoothMany(x ≫ 1, y ≫ 1)` no mesmo mapa e som 0x1B [código].

`_SmoothMany` (`1010:889c`) chama `_SmoothACell` na célula, em cima, à direita, embaixo, à esquerda e de novo na célula. `_SmoothACell` (`1010:88f4`): soma as quatro vizinhas (fora do mapa vale −1), divide por 4 (truncando para zero), soma o valor da célula e divide por 2; limita a 255 e grava se a célula existe [código].

Com o botão parado, o cheiro é aplicado de novo a cada volta do laço (seção 3).

### 4.6 Inseticida (`_ExpKillAnts`, `1010:7d5c`)

Oito jatos [código; o Ghidra marcou um bloco como inalcançável e a desmontagem (`1010:7d62`–`7d8a`) decidiu: `cmp byte [8606],1 / sbb / and 7 / inc` dá 8 com subopção 0 e 1 com subopção 1]:

- **Jato (`toolStream`, 1):** os 8 na própria célula, sem sorteio.
- **Spray (`toolSpray`, 0):** cada um em `(x + SRand1(9) − 4, y + SRand1(9) − 4)` e mais um `SRand1(1000)` (+0x278F) para o som.

Cada jato toca o som 9 e, se a célula é válida: na superfície, se `_LifeA` ≠ 0, `_Tindex = _FindInAList(x, y)` e, achada, `_DeadAntHere(x, y, cor)` (cadáver) e T = 0; se `(_SpidX ≫ 4, _SpidY ≫ 4)` = (x, y), `_KillSpider`; se 0x37 < tile < 0x3F, `_KillAntLion(_FindInLionList(x, y))`. No ninho, se a vida ≠ 0, `_Tindex = _FindInBList/RList(x, y, vida)` e, achada, T = 0 e vida = 0 (sem cadáver). A amarela (0xFF) não está nas listas e não é atingida.

## 5. Quick Game → Experimental

Item `editToExpMode` da janela 10 [código, `_EditToolsMenu` e `_MapToolsMenu`/`_RibbonToolsMenu`, que antes fazem `_SetExpTool(_CurExpTool)`, sem efeito]:

1. `_CurGameType = 3`;
2. `_ClearMyLife(_MePlane, _MeLoc, _MeType, _MeDir)`: apaga a marca 0xFF (e a cauda 0xFE da rainha) da camada de vida;
3. `_InitYelloAnt` (seção 7): no Experimental, a amarela vai para o plano 1, (0x40, 0x20), operária, `_MeMode = 2`, **sem** marca no mapa;
4. `_MeMode = 2` de novo; `_InitSpider` (seção 7); título da janela; som 2.

O mundo (formigas, ninhos, comida) continua o mesmo. No port: `quick_para_experimental`.

## 6. "Start Game" (ferramenta 7)

[código] `_CurGameType = 1` (Quick Game no mundo editado), `_InitYelloAnt` (operária no ninho preto, em `(_RebornX, _RebornY)`, com a marca 0xFF), e, se o mapa mostra a superfície (`_MapPlane` = 1), `_SetMyLife` de novo com os dados atuais da amarela; título; música 0x2AFB; `_CurExpTool = 7`. A aranha e o resto ficam como estão. No port: `escolher_ferramenta(…, ComecarJogo, …)` / `comecar_jogo`.

## 7. Rotinas de apoio

- `_InitYelloAnt` (`1030:6492`): `_MeBase = 0`, `_MeEggCarried = 0xFD`; se `_MeDropAlarm` ≠ 0, zera e põe `_ELayerMode = −1` (interface); `_SetMyHealth(100)`; Experimental: plano 1, (0x40, 0x20), `_MeType` 0x10, `_MeMode` 2 (direção não muda); senão `_MeMode` 0 e `_SetMyLife` com direção 2 e 0xFF: no Full Game sem ninho, rainha alada (0x40) na primeira célula < 0x10 de até 100 sorteios `(0x20 + Sk(16) − Sk(16), 0x20 + Sk(8) − Sk(8))` (sem lugar: (0x40, 0x20)); nos outros, operária no ninho preto em `_RebornX/Y`. Por fim `_ResetYellowVars(_MePlane, _MeLoc)`. O mesmo de [geracao-mundo.md](../regras/geracao-mundo.md) §3.D, aqui sobre o estado em jogo.
- `_InitSpider` (`1020:4668`): `_SpidBurpCnt = 10`; zera `_EatCnt`, `_SCorpseBase`, `_Scycle`, `_Scycle2`, `_SpidRevenge`, `_SpidDir`, `_RevSpider`, `_SMode`, `_MeSMode`; `_SpidOn = (_CurGameType ≠ 0)`; `(_SpidX, _SpidY) = (0x400, 0x200)`; `_Starg = −2`, `_StargLife = −1`; `(_SuserX, _SuserY) = (0x40, 0x40)`.

## 8. O que muda no `Estado`

| Ferramenta | Campos |
|---|---|
| Barreira | `superficie.mapa`; `s` (pedrinha e apagar) |
| Cavar | superfície: `superficie.mapa`, `preta`/`vermelha` (`buracos`, `ultimo_buraco`, `ultima_saida`, totais de escavação), `ninho_?.mapa/saida`; ninho: `ninho_?.mapa/saida/vida/lista`, totais de escavação; `s` (cavar) |
| Formiga | `superficie.lista/vida` ou `ninho_?.lista/vida` (e `mapa`, se cavou); `s` |
| Comida | `superficie.mapa`/`ninho_?.mapa`, `ambiente.comida_superficie`, `preta.comida`, `vermelha.comida`; `s` (Ctrl e vinte pedaços) |
| Cheiro | `feromonios.ninho_b/trilha_b/ninho_r/trilha_r/alarme` |
| Inseticida | `superficie.lista/vida/mapa`, `partida.cadaver_ponteiro/cadaveres_x/y`, `aranha`, `formigas_leao`, `ninho_?.lista/vida`, `tindex`; `s` |
| Trocas de modo | `partida.tipo_jogo`, `partida.ferramenta_experimental`, `amarela`, `aranha`, vida da amarela |

## 8.1 Na interface do port

`simant/src/modos.rs` e `engine/src/core/menu_ferramentas.rs` (dados em `data/menu_ferramentas.json`):

- No Experimental, o botão `ribTools` da faixa mostra o bitmap 14500 (`_NewGame` troca o par para 14550/14500) e abre o menu da janela 9 abaixo do botão, com um contorno no item da ferramenta atual. Um item com submenu abre o submenu (janelas 12–17) ao lado; o clique num item do submenu grava `_ExpSubStates` e fecha o menu. Clique fora fecha sem escolher. A posição do menu é escolha do port [hipótese].
- No Quick Game o mesmo botão passa a partida para o Experimental (`quick_para_experimental`) sem o menu de um item (janela 10); no Full Game e no Tutorial não faz nada (divergencias.md).
- No Experimental, todo clique no mapa da janela de edição (esquerdo ou direito) vai para `experimental::pressionar`; mover o mouse com o botão apertado chama `arrastar` a cada evento de movimento (o original repete no ritmo do redesenho); soltar chama `soltar`. Ctrl é lido do estado do teclado no momento. Os sons e a música das ferramentas vão para a fila de eventos (tocados pelo som do port); avisos e o quadro de exame só aparecem no terminal.

## 9. Verificação

Testes do port (`engine/tests/regras_experimental.rs`), a partir do pseudocódigo:

| Caso | Entrada | Esperado |
|---|---|---|
| E-U1 | `QUICK2.ANT`, Quick → Experimental | marca 0xFF apagada em (34, 9) do ninho preto; amarela no plano 1 em (64, 32), `_MeMode` 2, saúde 100, `_MeEggCarried` 0xFD, sem marca; aranha reiniciada |
| E-U2 | E-U1 + "Start Game" | `_CurGameType` 1, amarela operária em `_RebornX/Y` com 0xFF, `_CurExpTool` 7, música 0x2AFB; o menu fora do Experimental não faz nada |
| E-U3 | Full Game | o botão não muda nada |
| E-U4/5 | comida na superfície e no ninho | 0x48, 0x49, 0x4A, 0x4B e recusa; objeto 0x20 conta sem mudar; Ctrl; no ninho o Ctrl não desconta `_FoodB` |
| E-U6 | vinte pedaços | mesmas células e mesmo estado de S que a conta da seção 4.4 |
| E-U7 | formiga | T = 0x10 + S(8), M 2; célula ocupada recusada (S sorteado); vermelha no ninho preto M 7; linha 0 recusada |
| E-U8 | inseticida em jato no `QUICK2.ANT` | cadáver da cor, T = 0, oito sons |
| E-U9 | cheiro de alarme | (5, 5) = 19 e as quatro vizinhas = 4 |
| E-U10 | encher e cavar no ninho preto do `QUICK2.ANT` | 0x2E, saída 0, totais −1/−x/−y, centro igual; Ctrl cava de volta; aviso 21 na superfície |
| E-U11 | pedrinha e Ctrl | 0x51 + S(3) e som com S(10000) + 2000; apagar = S(16); muro no ninho: nada |
| E-U12 | arrasto | comida não repete parada; cheiro repete |

**Verificado no original (bancada QEMU, 2026-10-17):** roteiros `tests/referencias/roteiros/qemu_experimental_e01.txt` e `qemu_experimental_e02.txt` (com uma cópia própria do overlay: `--overlay re_trabalho/qemu/ov_exp.qcow2`; saídas em `re_trabalho/exp/`). Experimental novo, jogo pausado (Shift+0), `_CurExpTool` e `_ExpSubStates` escritos na memória pelo gdbstub (`escrever_variavel CurExpTool N`, `escrever_variavel 8:860N V 1`), cliques com `clicar_em` na janela de edição, `estado` e `SementeS` antes e depois de cada clique. Os estados estão em `tests/referencias/saves/E01_*.ANT` e `E02_*.ANT`, e os testes `e_01_…` e `e_02_…` partem deles e chegam **byte a byte** ao save do original, inclusive no estado do gerador S:

- **E-01** (S 0x7CFE → 0xCBEF): comida em (57, 28), que era pedra 0x57 (nada muda, sem sorteio); formiga preta em (59, 28) (T = 0x14, M = 2, um sorteio); muro em (61, 28), tile 0x28 → 0x60 (sem sorteio); pedrinha em (62, 28), que era pedra 0x59 (só o `SRand1(10000)` do som); inseticida em jato em (59, 28): cadáver 0x13, T = 0, `DdPtr` 0 → 1 (um sorteio, o do cadáver).
- **E-02** (a) vinte pedaços em (64, 31) e spray em (55, 26): 15 células, `_FoodA` 0 → 19, S 0xC14E → 0x0E20; (b) no ninho preto (Ctrl+9, vista em (20, 0)): cavar (28, 4) (`_TileTotXB` 355 → 383, `_TilesDugB` 10 → 11, centro 35 → 34), encher (28, 4) (de volta a 355/10, centro **não** recalculado: fica 34) e encher (22, 6) (terra: nada), S 0x0E20 → 0x1C40.

Observação da bancada: a vista do Experimental novo começou em `_MapPnt` = (52, 23), não em (51, 23) como daria `_CenterEdit(0x40, 0x20)` com 26 colunas [em aberto: largura da vista no instante do `_CenterEdit` de `_NewGame`].

**Verificado no original (bancada, 2026-10-17, `tools/cenarios_tick.py quick_exp|exp_start|exp_ctrl`; vetores em `tests/referencias/acoes/`, `engine/tests/acoes_originais.rs`; jogo pausado, estado antes e depois, 307 blocos e S e R):**

- **Quick → Experimental** (`quick_exp`): snapshot `quick`, botão de ferramentas da faixa em (354, 71) (`ribTools`, com o bitmap da ferramenta) e o item "EXPERIMENTAL MODE" da janela 10 em (360, 127): `quick_para_experimental` dá 307/307.
- **"Start Game"** (`exp_start`): snapshot `exp`, mesmo botão (abre a janela 9 abaixo dele: ANT INFO … START GAME, este em (360, 246)): `escolher_ferramenta(ComecarJogo)` dá 307/307.
- **Ctrl e arrasto** (`exp_ctrl`, ferramenta e subopção escritas na memória, Ctrl apertado pelo QMP): comida em (55, 35) e Ctrl+comida na mesma célula (a comida sai e o chão é sorteado de novo), muro em (57, 37) e Ctrl+muro (apaga, chão sorteado), arrasto de muro de (58, 39) a (63, 39) (tiles de conexão 0x65, 0x60 … 0x63) e arrasto de comida de (60, 38) a (64, 38) (cinco 0x48, `_FoodA` +5): `pressionar`/`arrastar`/`soltar` dão 307/307 com o mesmo S.

A vista do Experimental novo em `_MapPnt` = (52, 23) (seção acima) fecha com `_editWidth` = 25 na bancada (0x40 − 12 = 52), o mesmo que a centralização da amarela nos vetores de tick (testes-tick.md).

**Não verificado no original:** Ctrl em cavar/encher e no cheiro, cheiro (a repetição parada depende do ritmo do redesenho), formiga em ninho, inseticida sobre a aranha e a formiga-leão.

## 10. Lacunas e divergências

- `_KillAntLion(_FindInLionList(x, y))` com um tile de formiga-leão sem entrada na lista recebe −1 e grava fora da tabela no original; o port não faz nada [divergência registrada para docs/divergencias.md].
- O número de voltas do arrasto por segundo (cheiro e inseticida parados) depende do redesenho do original; o port deixa o ritmo para a interface.
- Os textos do quadro de exame e dos avisos são do banco (listas 4/1010, 4/1100–1103); a interface do port ainda não os desenha.
- `_ELayerMode`, cursores, bitmaps do botão e título da janela são interface.
