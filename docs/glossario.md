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
| `_goStep*` | andam a vista com a formiga amarela e atualizam o ponto lembrado | código |
| `_ScrollUp/Down/Left/Right`, `…S` | funções vazias na versão Windows | código |
| `_editWidth`, `_editHeight` | tamanho da vista em tiles: `editTileArea` (+1 com faixa) dividido por 16, arredondado para cima | `_win_EditChanged` |
| `_Me*` (`_MeLocX/Y`, `_MePlane`, `_MeType`, `_MeDir`, `_MeEgg`, `_MeBase`, `_MeMode`) | estado da formiga amarela; com `_MeMode` = 0 ela é gravada como 0xFF na camada de vida | `_SetMyLife`, `_DoCalcTile` |
| `_TERRAINset` | tileset de chão ativo: 0 quintal (9/10), 1 casa (9/9) | `_OverlayTileSet` |
| `_ELayerMode` | camada de feromônio sobreposta à superfície (−1 = nenhuma, 0–4 = `_PherMapA/BN/BT/RN/RT`) | `_DoCalcTile` |
| `_LifeA/B/R` | mapa de vida (formigas e outros) por célula, desenhado sobre o chão; não está no save | `_DoCalcTile` |
| `tilebufover`, `tilebufnest` | buffers de tiles de vida da superfície (9/15–17) e dos ninhos (9/18–20) | `_LoadTiles` |
