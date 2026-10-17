# Glossário de nomes internos

Nomes tirados do `SIMANTW.SYM` e dos dados. Significados marcados com [hipótese] foram inferidos do nome e ainda não confirmados no código.

| Nome | Significado | Fonte |
|---|---|---|
| `A` (sufixo/prefixo: `_MapA`, `_AlistX`, `_DoAntSimA`) | superfície / quintal [hipótese] | SYM; `_DoAntSimA` trata forrageio, recrutamento e luta fora do ninho |
| `B` (`_MapB`, `_BlistX`, `_DoAntSimB`) | ninho / colônia **preta** (a do jogador) | SYM (`_PlaceBlackQueen`, `_BlkQueens`) |
| `R` (`_MapR`, `_RlistX`, `_DoAntSimR`) | ninho / colônia **vermelha** (adversária) | SYM (`_PlaceRedQueen`) |
| `Y` (`_DoAntSimY`, `_YellowBirth`) | a **formiga amarela**, controlada pelo jogador | SYM; bitmaps "Yellow Health" |
| `Alist`/`Blist`/`Rlist` + `X`,`Y`,`M`,`T`,`S` | listas paralelas de formigas **por área** (superfície, ninho preto, ninho vermelho), de qualquer colônia: posição X/Y, M = modo/tarefa, T = tipo (bit 7 = vermelha, bits 3–6 = classe, 12 = rainha), S = auxiliar do modo [hipótese] | `_AddAntToAList`, `_CountAnts`, `_DoNestAntB`; [formatos/save-ant.md](spec/formatos/save-ant.md#estruturas-principais) |
| `ListIndexA/B/R` | número de entradas usadas em cada lista | `_AddAntToAList`, `_CompactListA` |
| `LifeA/B/R` | mapa de ocupação (campo T por célula), reconstruído das listas ao carregar; não é salvo | `_DoLoadInitializations` |
| `_gameCycles` | voltas do laço do `MYTIMERFUNC` (32 bits), não ticks; depende do relógio do host; fora do save | `MYTIMERFUNC`; medido na bancada ([testes-tick.md](spec/regras/testes-tick.md)) |
| `_BirdDelay` | próximo valor de `_gameCycles` em que `_SimBird` testa a vinda do pássaro (`_gameCycles` + 20 ou + 30); salvo, mas não reproduzível entre execuções | `_SimBird` `1030:28aa` |
| `YMap…` (`_YMapPopB`, `_YMapSeeds`, `_YMapPnt`) | mapa do quintal inteiro (*yard*), 12 × 16 patches; não é a formiga amarela | `_SimColonies`, `_XferPatch` |
| `CastePop`, `ModePop` | populações por grupo de casta e de modo (6 valores cada), recalculadas pela simulação | `_CountAnts`, `_TallyModePop` |
| `PherMap` (`A`, `BN`, `BT`, `RN`, `RT`) | mapas de feromônio: alarme (A), ninho (N) e trilha (T) por colônia [hipótese, apoiada pela lista de strings 4/1000: "Black Nest Marker Pheromone", "Black Trail…"] | tabela do save, SHARED 4/1000 |
| `HoleMap`, `ExitMap` | entradas/saídas dos ninhos | tabela do save |
| `Caste` (`_casteLevels`, `_IdealCaste`) | divisão de castas: operárias, soldados, reprodutores (triângulo de controle) | bitmaps "Workers/Soldiers/Breeders" |
| `Mode` (`_modeLevels`) | divisão de tarefas: forragear, cavar, cuidar das crias (triângulo de controle) | bitmaps "Forage/Dig/Nurse" |
| `Yard` | o quintal inteiro visto de cima, dividido em "patches" | SYM (`_InitSimYard`, `_YardToMap`) |
| `Patch` | cada quadrado do quintal; um jogo completo tenta dominar todos | SYM (`_MakeYardPatch`, `_GetNearbyPatches`) |
| `Edit` (janela) | a vista de perto/zoom onde a formiga amarela anda | SYM (`_OpenEditWindow`), nomes `editWin…` no SHARED 1/0 |
| `Lion` | formiga-leão | SYM (`_InitAntLions`) |
| `Pillar`/`Pill` | lagarta [hipótese: "caterpillar"] | SYM |
| `Sow` | [em aberto] | SYM (`_InitSow`, `_DoSow`) |
| `Swarm` | revoada nupcial (machos e rainhas novas) [hipótese] | SYM (`_MaintainSwarm`, `_SwarmXB`) |
| `Boy`/`Kid`, `Cat`, `Dog`, `Bird`, `Mower` | humano, gato, cachorro, pássaro e cortador de grama no quintal | SYM; sprites no WINGANT |
| `db_*`, `DB*`, `ch_*` | banco de recursos `.NDX/.DAT` e cache de objetos | SYM; [formatos/banco-dat-ndx.md](spec/formatos/banco-dat-ndx.md) |
| `win_*` | sistema de janelas próprio do jogo (portado do Mac) | SYM |
| `displayType` | modo de vídeo escolhido pelo `SIMANT.CFG` | `_ReadConfig` |
| `_grndTile`, `_lifeTile` | índice do tile de chão e de vida da célula sendo desenhada, calculados por `_DoCalcTile` | código; [formatos/tiles.md](spec/formatos/tiles.md) |
| `_MapPlane` | mapa mostrado na janela de edição: 0 quintal (desenha `_MapA`), 1 superfície, 2 ninho preto, 3 ninho vermelho | `_DoCalcTile`, `_SetMapPlane`; [telas/jogo.md](spec/telas/jogo.md) |
| `_MapPnt` | célula do mapa no canto superior esquerdo da janela de edição (par x, y em `DS:7d2e`/`7d30`) | `FUN_1010_6250`, `_CenterEdit` |
| `_YMapPnt`, `_AMapPnt`, `_BMapPnt`, `_RMapPnt` | centro lembrado da vista em cada plano (0 a 3) | `_SetMapPlane`, `_DoLoadInitializations` |
| `_DoEditScroll` | trata `WM_HSCROLL`/`WM_VSCROLL` da janela de edição (linha, página, polegar); não atualiza o ponto lembrado | `MAINWNDPROC`; [telas/jogo.md](spec/telas/jogo.md), "Navegação" |
| `_SetMapPlane`, `_SetMapPlaneLocation`, `_GotoMapPoint` | troca o plano mostrado e centraliza a vista no ponto lembrado (ou num ponto dado) | `_ProcMapRibbonEvent`, `_ProcMenu` |
| `_goStep*` | andam a vista um passo e atualizam o ponto lembrado; sem chamadas no pseudocódigo [hipótese: resto da versão Mac] | código |
| `_GotoMyAnt` | leva a vista até a formiga amarela (`_CenterEdit` ou `_SetMapPlaneLocation`); chamado pelo AutoTrack, Home, Backspace... | código (`1000:a764`) |
| `_gameCycles` | voltas do laço do temporizador (`MYTIMERFUNC`), mesmo sem tick; no port, `Estado::ciclos_jogo` | código (`9:9b2c`) |
| `_ScrollUp/Down/Left/Right`, `…S` | funções vazias na versão Windows | código |
| `_editWidth`, `_editHeight` | tamanho da vista em tiles: `editTileArea` (+1 com faixa) dividido por 16, arredondado para cima | `_win_EditChanged` |
| `_Me*` (`_MeLocX/Y`, `_MePlane`, `_MeType`, `_MeDir`, `_MeEgg`, `_MeBase`, `_MeMode`) | estado da formiga amarela; com `_MeMode` = 0 ela é gravada como 0xFF na camada de vida | `_SetMyLife`, `_DoCalcTile` |
| `_TERRAINset` | tileset de chão ativo: 0 quintal (9/10), 1 casa (9/9) | `_OverlayTileSet` |
| `_ELayerMode` | camada de feromônio sobreposta à superfície (−1 = nenhuma, 0–4 = `_PherMapA/BN/BT/RN/RT`) | `_DoCalcTile` |
| `_LifeA/B/R` | mapa de vida (formigas e outros) por célula, desenhado sobre o chão; não está no save | `_DoCalcTile` |
| `tilebufover`, `tilebufnest` | buffers de tiles de vida da superfície (9/15–17) e dos ninhos (9/18–20) | `_LoadTiles` |
| `_TickCount` | relógio do Windows em unidades de 55 ms (`GetTickCount() / 55`): prazos de mensagens e quadros | `1008:18ac`; [telas/mensagens.md](spec/telas/mensagens.md) |
| `_EditMessage` / `_DrawRibbonMessage` | mensagem curta da faixa (`ribMessageArea`), com prazo ou permanente, "importante" ou não | `1000:92c0`, `1000:0b26`; mensagens.md §2 |
| `_PictStrnDialog` / `_PictureDialog` | quadro de mensagem modal (janela 30, `picStrWin`): texto SHARED 4, imagem opcional, para a simulação | `1000:615a`, `1000:62bc`; mensagens.md §3 |
| `_EndGameDialog` / `_CalcScore` | diálogo de fim de jogo (janela 4) e a pontuação | `1000:6602`, `1000:5530`; mensagens.md §4 |
| `_DoUserButton` / `ribUserDef1A`–`1H` | botões configuráveis da faixa (pausa, velocidade, opções, salvar…) | `1000:04c4`; mensagens.md §5 |
| `_OptionStates[0..5]` | opções AutoTrack, Music, Sound Effects, Events (quadros), Messages (faixa), Silly (balões) | `8:85f2`; formiga-amarela.md §11.7 |
| `_myBeginSong` / `_myBeginSound` | tocar música (id do cabeçalho tipo 18) / efeito (id do tipo 5) | `GR_MODULE`; [regras/som.md](spec/regras/som.md) |
| `_songsOnFlag`, `_effectsOnFlag` | música e efeitos ligados (cópias de `_OptionStates[1]` e `[2]`, zeradas sem foco) | `_myBeginSong`, `MAINWNDPROC` |
| DAC (`_UncompressDACInstrument`) | efeito sonoro tipo 5: 8 bits em deltas de 4 bits | [formatos/som.md](spec/formatos/som.md) |
| `_CurGameType` | tipo de jogo: 0 Tutorial, 1 Quick Game, 2 Full Game, 3 Experimental | `_NewGame` (`1000:d6dc`); [telas/abertura.md](spec/telas/abertura.md) |
| `_DoScenario` | o "Select a Game" (janela 2): devolve o id do objeto clicado | [telas/abertura.md](spec/telas/abertura.md) |
| `_YardMode` | modo da janela do quintal: 0 Yard View, 1 House View, 2 Strategic View, 3 Graphic View | `_SetYardMode`; [telas/quintal.md](spec/telas/quintal.md) |
| `_CurYardPnt` / `_YMapPnt` | patch atual / patch escolhido na janela do quintal (12 × 16 patches) | `_XferPatch`, `_YardArea` |
| `_XferPatch` | troca de patch no Full Game (gera o patch novo com `_RandWorld`) | `1000:ab90`; [telas/quintal.md](spec/telas/quintal.md) §5 |
| `_PlaceQueenInYard` | botão PLACE: põe uma rainha guardada num patch a até 3 do atual | `1000:ad12` |
| `_patchRgn`, `_patchRgn2` | polígono de um patch na janela do quintal (cursor/clique e cor das colônias) | DGROUP `1048:1894`, `1048:18a4` |
| `_MakeHousePatch` | chão de um patch da casa (`n = px × 16 + py` ≤ 37) | `1020:3120`; [regras/geracao-mundo.md](spec/regras/geracao-mundo.md) §5 |
| `_DROPdir` (`9:9c66`) | direção da queda da comida na casa (0 ladrilhos, 1 carpete esquerdo, 2 cozinha/liso, 3 carpete direito) | geracao-mundo.md §5 |
| `_TileFrame1/2`, `_FillMapLegs` | molduras de azulejo; retângulo com fiapos e pés 4 × 4 (móvel) | geracao-mundo.md §5 |
| `_CurExpTool`, `_ExpSubStates` | ferramenta do Experimental (0–7) e a subopção de cada uma | [telas/experimental.md](spec/telas/experimental.md) |
| `_processExp`, `_DoTool` | clique/arrasto no mapa no Experimental e a ferramenta aplicada | `1010:7022`, `1010:727e` |
| `_AntMenu` / `_win_DoProxMenu` | menu da formiga amarela ao segurar o botão sobre ela; itens viram `_YellowCommand` | `1010:6d24`, `1030:de62`; [telas/janelas.md](spec/telas/janelas.md) |
| `_casteLevels` / `_modeLevels` | os três níveis (0–65535) dos triângulos Caste e Behavior Control | `9:9b64`, `9:9c74`; janelas.md §2 |
| `_DoBookMark` | marcadores 1–7 da faixa (Ctrl+clique guarda a vista) | `1000:08a6`; mensagens.md §2.3 |
| `_LoadGame` / `_SaveGame` / `_FileSelect` | File → Load (pergunta, diálogo, `_RandYard`, blocos), Save/Save As e o diálogo comum de arquivo | `1010:9fa0`, `1010:a23e`, `1010:9920`; [telas/arquivos.md](spec/telas/arquivos.md) |
| `_gGameNeedsSaving` | partida mudada desde o último Save/Load (1 a cada tick) | `8:8370`; arquivos.md |
| `_StopSimulation` / `_RestartSimulation` | `KillTimer`/`SetTimer` aninhados em volta das caixas modais | `1000:0000`, `1000:0034`; mensagens.md §3.5 |
| `_MysteryButton`, `_AddRedAnts`, `_KillSomeAnts` | botão Mystery (`_RRand(15)`), vermelhas novas, matar até 51 da lista A | `1000:b0bc`, `1030:6cfe`, `1000:9d5e`; [regras/misterio.md](spec/regras/misterio.md) |
| `_MagnifyMenu` | Examine: janela 29 enquanto o botão fica apertado | `1010:6b4c`; janelas.md §4.3 |
| `_SetMapModeAnt`, `_MapMode`, `_showTrails` | modo da janela do mapa (planos e feromônios) e as trilhas | `1000:9e82`, `8:8610`, DGROUP `1998`; [telas/mapa.md](spec/telas/mapa.md) |
| `_OpenInfoWindow`, `_DisplayCard` | Info ("Top Card"): cartões dos tipos 17/19/10/21 | `1018:60da`, `1018:5a1c`; [telas/info.md](spec/telas/info.md) |
| `_MakeBalloon` | balão de fala (peças 8×8 do DGROUP) do menino | `1008:59e4`; telas/quintal.md §2.3 |
