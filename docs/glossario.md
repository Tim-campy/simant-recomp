# Glossário de nomes internos

Nomes tirados do `SIMANTW.SYM` e dos dados. Significados marcados com [hipótese] foram inferidos do nome e ainda não confirmados no código.

| Nome | Significado | Fonte |
|---|---|---|
| `A` (sufixo/prefixo: `_MapA`, `_AlistX`, `_DoAntSimA`) | superfície / quintal [hipótese] | SYM; `_DoAntSimA` trata forrageio, recrutamento e luta fora do ninho |
| `B` (`_MapB`, `_BlistX`, `_DoAntSimB`) | ninho / colônia **preta** (a do jogador) | SYM (`_PlaceBlackQueen`, `_BlkQueens`) |
| `R` (`_MapR`, `_RlistX`, `_DoAntSimR`) | ninho / colônia **vermelha** (adversária) | SYM (`_PlaceRedQueen`) |
| `Y` (`_DoAntSimY`, `_YellowBirth`) | a **formiga amarela**, controlada pelo jogador | SYM; bitmaps "Yellow Health" |
| `Alist`/`Blist`/`Rlist` + `X`,`Y`,`M`,`T`,`S` | listas paralelas de formigas por área: posição X/Y e três campos [hipótese: M = modo, T = tipo/casta, S = estado] | tabela do save |
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
