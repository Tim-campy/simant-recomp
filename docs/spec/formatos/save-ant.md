# Jogo salvo (`.ANT`)

O `.ANT` não tem cabeçalho. É a concatenação, em ordem fixa, de 307 blocos de memória do jogo, num total de **48.386 bytes**, gravados crus em little-endian (memória do x86).

**Fonte:**
- Fato visto no código: `_SaveGame` (`1010:a23e`) percorre a tabela no segmento de dados 8 (`SIMANT_DATA_GROUP`, seletor `1038`), offset `0x9570`, e para cada entrada `{u16 count, u16 tamanho do elemento, ponteiro far}` grava `count × tamanho` bytes com `_lwrite`. A tabela termina na primeira entrada com `count = 0`. `_LoadGame` (`1010:9fa0`) lê de volta com a mesma tabela.
- Antes de gravar, `_SaveGame` checa o espaço livre em disco e exige 125% da soma dos blocos.
- A tabela abaixo foi gerada por `tools/save_table.py`, que resolve os segmentos seguindo as cadeias de relocação NE. Os nomes vêm do `SIMANTW.SYM`.
- **Verificado (2026-10-16):** o `QUICK2.ANT`, salvo pelo original na bancada QEMU (`docs/bancada-qemu.md`), tem exatamente 48.386 bytes (`tests/referencias/saves/QUICK2.ANT`).

Esta tabela também é o **modelo de estado da simulação**: tudo o que precisa sobreviver a salvar e carregar está aqui. Legenda dos prefixos (inferida dos nomes): `A` = superfície (o patch atual), `B` = ninho preto, `R` = ninho vermelho, `Me` = formiga amarela (a controlada pelo jogador), `YMap` = mapa do quintal inteiro (*yard*, ver abaixo), `H_` = séries históricas do gráfico.

As dimensões e a indexação das matrizes principais estão na seção [Estruturas principais](#estruturas-principais), abaixo da tabela.

**Dados do port:** a mesma tabela está em `data/save_layout.json` (símbolo sem o `_` inicial, `count`, `tamanho`, `endereco`), gerada com `python tools/save_table.py SIMANTW.EXE SIMANTW.SYM --json data/save_layout.json`. O carregador (`engine/src/formats/save.rs`) lê o layout desse arquivo, não do código. O teste `layout_bate_com_a_tabela_do_executavel` (`engine/tests/save_ant.rs`) confere `count` e `tamanho` contra o executável quando `SIMANT_DIR` aponta para ele.

| # | Offset no arquivo | Bytes | count × tam | Endereço | Símbolo |
|---|---|---|---|---|---|
| 0 | 0 | 8192 | 1×8192 | `10:28e8` | `_MapA` |
| 1 | 8192 | 4096 | 1×4096 | `10:48e8` | `_MapB` |
| 2 | 12288 | 4096 | 1×4096 | `10:58e8` | `_MapR` |
| 3 | 16384 | 4096 | 1×4096 | `8:03a4` | `_ExitMapB` |
| 4 | 20480 | 4096 | 1×4096 | `8:13a4` | `_ExitMapR` |
| 5 | 24576 | 1000 | 1×1000 | `8:23a4` | `_AlistX` |
| 6 | 25576 | 1000 | 1×1000 | `8:278e` | `_AlistY` |
| 7 | 26576 | 1000 | 1×1000 | `8:2b78` | `_AlistM` |
| 8 | 27576 | 1000 | 1×1000 | `8:2f62` | `_AlistT` |
| 9 | 28576 | 1000 | 1×1000 | `8:334c` | `_AlistS` |
| 10 | 29576 | 500 | 1×500 | `8:3736` | `_BlistX` |
| 11 | 30076 | 500 | 1×500 | `8:392c` | `_BlistY` |
| 12 | 30576 | 500 | 1×500 | `8:3b22` | `_BlistM` |
| 13 | 31076 | 500 | 1×500 | `8:3d18` | `_BlistT` |
| 14 | 31576 | 500 | 1×500 | `8:3f0e` | `_BlistS` |
| 15 | 32076 | 500 | 1×500 | `8:4104` | `_RlistX` |
| 16 | 32576 | 500 | 1×500 | `8:42fa` | `_RlistY` |
| 17 | 33076 | 500 | 1×500 | `8:44f0` | `_RlistM` |
| 18 | 33576 | 500 | 1×500 | `8:46e6` | `_RlistT` |
| 19 | 34076 | 500 | 1×500 | `8:48dc` | `_RlistS` |
| 20 | 34576 | 2048 | 1×2048 | `8:52d2` | `_PherMapA` |
| 21 | 36624 | 2048 | 1×2048 | `8:62d2` | `_PherMapBN` |
| 22 | 38672 | 2048 | 1×2048 | `8:6ad2` | `_PherMapBT` |
| 23 | 40720 | 2048 | 1×2048 | `8:72d2` | `_PherMapRN` |
| 24 | 42768 | 2048 | 1×2048 | `8:7ad2` | `_PherMapRT` |
| 25 | 44816 | 64 | 1×64 | `8:82d2` | `_HoleMapB` |
| 26 | 44880 | 64 | 1×64 | `8:8312` | `_HoleMapR` |
| 27 | 44944 | 192 | 1×192 | `8:00a4` | `_YMapPopB` |
| 28 | 45136 | 192 | 1×192 | `8:0164` | `_YMapPopR` |
| 29 | 45328 | 384 | 2×192 | `8:0224` | `_YMapSeeds` |
| 30 | 45712 | 4 | 4×1 | `9:76a0` | `_BAntsEaten` |
| 31 | 45716 | 4 | 4×1 | `9:9b26` | `_BAntsExpired` |
| 32 | 45720 | 4 | 4×1 | `9:99e0` | `_BAntsKilled` |
| 33 | 45724 | 4 | 4×1 | `9:9fbc` | `_BirdDelay` |
| 34 | 45728 | 4 | 4×1 | `9:7d4a` | `_BoyMsgCnt` |
| 35 | 45732 | 4 | 4×1 | `9:737e` | `_CatDelay` |
| 36 | 45736 | 4 | 4×1 | `9:9de6` | `_DogBarkDelay` |
| 37 | 45740 | 4 | 4×1 | `9:76a8` | `_GameTime` |
| 38 | 45744 | 4 | 4×1 | `9:7898` | `_OverallScore` |
| 39 | 45748 | 4 | 4×1 | `9:7d5c` | `_RAntsEaten` |
| 40 | 45752 | 4 | 4×1 | `9:9fc6` | `_RAntsExpired` |
| 41 | 45756 | 4 | 4×1 | `9:9e96` | `_RAntsKilled` |
| 42 | 45760 | 4 | 4×1 | `9:8104` | `_TileTotXB` |
| 43 | 45764 | 4 | 4×1 | `9:9ddc` | `_TileTotXR` |
| 44 | 45768 | 4 | 4×1 | `9:811a` | `_TileTotYB` |
| 45 | 45772 | 4 | 4×1 | `9:9de2` | `_TileTotYR` |
| 46 | 45776 | 4 | 4×1 | `9:7c1e` | `_TotalEggsDiedB` |
| 47 | 45780 | 4 | 4×1 | `9:9af8` | `_TotalEggsLaidB` |
| 48 | 45784 | 4 | 4×1 | `9:9f2c` | `_AMapPnt` |
| 49 | 45788 | 4 | 4×1 | `9:9fb2` | `_BMapPnt` |
| 50 | 45792 | 4 | 4×1 | `9:7a60` | `_CurYardPnt` |
| 51 | 45796 | 4 | 4×1 | `8:8362` | `_LastBlackEgg` |
| 52 | 45800 | 4 | 4×1 | `8:836a` | `_LastFoodDrop` |
| 53 | 45804 | 4 | 4×1 | `8:8352` | `_LastNewExitB` |
| 54 | 45808 | 4 | 4×1 | `8:8356` | `_LastNewExitR` |
| 55 | 45812 | 4 | 4×1 | `8:835a` | `_LastNewHoleB` |
| 56 | 45816 | 4 | 4×1 | `8:835e` | `_LastNewHoleR` |
| 57 | 45820 | 4 | 4×1 | `8:8366` | `_LastRedEgg` |
| 58 | 45824 | 4 | 4×1 | `9:7d2e` | `_MapPnt` |
| 59 | 45828 | 4 | 4×1 | `9:72f6` | `_RMapPnt` |
| 60 | 45832 | 4 | 4×1 | `9:789e` | `_YMapPnt` |
| 61 | 45836 | 50 | 1×50 | `9:9b32` | `_SwarmXB` |
| 62 | 45886 | 50 | 1×50 | `9:9bf0` | `_SwarmXR` |
| 63 | 45936 | 50 | 1×50 | `9:9b8c` | `_SwarmYB` |
| 64 | 45986 | 50 | 1×50 | `9:9c34` | `_SwarmYR` |
| 65 | 46036 | 100 | 1×100 | `9:9c82` | `_DeadX` |
| 66 | 46136 | 100 | 1×100 | `9:9d76` | `_DeadY` |
| 67 | 46236 | 10 | 1×10 | `9:7a68` | `_LionListM` |
| 68 | 46246 | 10 | 1×10 | `9:7d34` | `_LionListS` |
| 69 | 46256 | 10 | 1×10 | `9:7d4e` | `_LionListT` |
| 70 | 46266 | 10 | 1×10 | `9:809c` | `_LionListX` |
| 71 | 46276 | 10 | 1×10 | `9:80bc` | `_LionListY` |
| 72 | 46286 | 100 | 1×100 | `9:79e6` | `_RxTab` |
| 73 | 46386 | 100 | 1×100 | `9:7a72` | `_RyTab` |
| 74 | 46486 | 6 | 2×3 | `9:9b64` | `_casteLevels` |
| 75 | 46492 | 24 | 2×12 | `8:863a` | `_casteSetLevels` |
| 76 | 46516 | 6 | 2×3 | `9:9c74` | `_modeLevels` |
| 77 | 46522 | 24 | 2×12 | `8:8658` | `_modeSetLevels` |
| 78 | 46546 | 12 | 2×6 | `10:ac94` | `_CastePopB` |
| 79 | 46558 | 12 | 2×6 | `10:aca0` | `_CastePopR` |
| 80 | 46570 | 24 | 2×12 | `9:a0b6` | `_GrassMap` |
| 81 | 46594 | 128 | 2×64 | `9:9cf2` | `_H_ADead` |
| 82 | 46722 | 128 | 2×64 | `9:7c98` | `_H_BFood` |
| 83 | 46850 | 128 | 2×64 | `9:9f30` | `_H_BHeal` |
| 84 | 46978 | 128 | 2×64 | `9:72fa` | `_H_BPop` |
| 85 | 47106 | 128 | 2×64 | `9:9dea` | `_H_FDead` |
| 86 | 47234 | 128 | 2×64 | `9:7382` | `_H_FoodA` |
| 87 | 47362 | 128 | 2×64 | `9:9eac` | `_H_PDead` |
| 88 | 47490 | 128 | 2×64 | `9:7d76` | `_H_RFood` |
| 89 | 47618 | 128 | 2×64 | `9:9ff0` | `_H_RHeal` |
| 90 | 47746 | 128 | 2×64 | `9:7608` | `_H_RPop` |
| 91 | 47874 | 8 | 2×4 | `8:8622` | `_IdealCaste` |
| 92 | 47882 | 12 | 2×6 | `9:9e70` | `_ModePopB` |
| 93 | 47894 | 12 | 2×6 | `9:a084` | `_ModePopR` |
| 94 | 47906 | 12 | 2×6 | `9:7c0e` | `_PillarMap` |
| 95 | 47918 | 6 | 2×3 | `9:9c2a` | `_SowDir` |
| 96 | 47924 | 6 | 2×3 | `9:78cc` | `_SowSave` |
| 97 | 47930 | 6 | 2×3 | `9:9bc8` | `_SowX` |
| 98 | 47936 | 6 | 2×3 | `9:9bda` | `_SowY` |
| 99 | 47942 | 20 | 2×10 | `8:86d6` | `_HistButtStates` |
| 100 | 47962 | 12 | 2×6 | `8:85f2` | `_OptionStates` |
| 101 | 47974 | 2 | 2×1 | `9:9c6e` | `_AntsEatenByLions` |
| 102 | 47976 | 2 | 2×1 | `9:a0dc` | `_BColoniesKilled` |
| 103 | 47978 | 2 | 2×1 | `9:80d4` | `_BColoniesStarted` |
| 104 | 47980 | 2 | 2×1 | `9:7604` | `_Barrier` |
| 105 | 47982 | 2 | 2×1 | `10:ac62` | `_BirdFrame` |
| 106 | 47984 | 2 | 2×1 | `9:78bc` | `_BirdGoalX` |
| 107 | 47986 | 2 | 2×1 | `9:78d8` | `_BirdGoalY` |
| 108 | 47988 | 2 | 2×1 | `10:ac64` | `_BirdOn` |
| 109 | 47990 | 2 | 2×1 | `10:ac66` | `_BirdX` |
| 110 | 47992 | 2 | 2×1 | `10:ac68` | `_BirdY` |
| 111 | 47994 | 2 | 2×1 | `9:79e4` | `_BlackLost` |
| 112 | 47996 | 2 | 2×1 | `9:78e8` | `_BlkQueens` |
| 113 | 47998 | 2 | 2×1 | `9:769c` | `_BlkWarnHealth` |
| 114 | 48000 | 2 | 2×1 | `8:8a72` | `_BoyDir` |
| 115 | 48002 | 2 | 2×1 | `10:ac7a` | `_BoyFrame` |
| 116 | 48004 | 2 | 2×1 | `10:ac74` | `_BoyHere` |
| 117 | 48006 | 2 | 2×1 | `9:99da` | `_BoyMessOn` |
| 118 | 48008 | 2 | 2×1 | `9:72f2` | `_BoyMsgOffset` |
| 119 | 48010 | 2 | 2×1 | `9:78b0` | `_BoyPx` |
| 120 | 48012 | 2 | 2×1 | `9:78d2` | `_BoyPy` |
| 121 | 48014 | 2 | 2×1 | `8:8a80` | `_BoyStandCnt` |
| 122 | 48016 | 2 | 2×1 | `8:8a74` | `_BoyTurnCnt` |
| 123 | 48018 | 2 | 2×1 | `8:8a70` | `_BoyWait` |
| 124 | 48020 | 2 | 2×1 | `10:ac76` | `_BoyX` |
| 125 | 48022 | 2 | 2×1 | `10:ac78` | `_BoyY` |
| 126 | 48024 | 2 | 2×1 | `10:ac82` | `_BpopT` |
| 127 | 48026 | 2 | 2×1 | `9:72b6` | `_CatCycle` |
| 128 | 48028 | 2 | 2×1 | `9:72bc` | `_CatDir` |
| 129 | 48030 | 2 | 2×1 | `10:ac6a` | `_CatFrame` |
| 130 | 48032 | 2 | 2×1 | `10:ac6c` | `_CatOn` |
| 131 | 48034 | 2 | 2×1 | `10:ac6e` | `_CatX` |
| 132 | 48036 | 2 | 2×1 | `10:ac70` | `_CatY` |
| 133 | 48038 | 2 | 2×1 | `9:72ec` | `_ChaseSpid` |
| 134 | 48040 | 2 | 2×1 | `8:8a66` | `_ClearRainFlag` |
| 135 | 48042 | 2 | 2×1 | `10:ac90` | `_ColonyTotalBlack` |
| 136 | 48044 | 2 | 2×1 | `10:ac92` | `_ColonyTotalRed` |
| 137 | 48046 | 2 | 2×1 | `8:8a68` | `_ColonyUpdateFlag` |
| 138 | 48048 | 2 | 2×1 | `9:9fe0` | `_CurEditMode` |
| 139 | 48050 | 2 | 2×1 | `9:79e2` | `_CurExpTool` |
| 140 | 48052 | 2 | 2×1 | `9:7c22` | `_CurGameTool` |
| 141 | 48054 | 2 | 2×1 | `9:80b4` | `_CurGameType` |
| 142 | 48056 | 2 | 2×1 | `9:75fc` | `_Cycle` |
| 143 | 48058 | 2 | 2×1 | `9:9ea8` | `_DdPtr` |
| 144 | 48060 | 2 | 2×1 | `9:72e0` | `_DeathCnt` |
| 145 | 48062 | 2 | 2×1 | `9:7d5a` | `_DogDir` |
| 146 | 48064 | 2 | 2×1 | `10:ac60` | `_DogFrame` |
| 147 | 48066 | 2 | 2×1 | `9:77b0` | `_DogPx` |
| 148 | 48068 | 2 | 2×1 | `9:789c` | `_DogPy` |
| 149 | 48070 | 2 | 2×1 | `9:7a5a` | `_DogTurnCnt` |
| 150 | 48072 | 2 | 2×1 | `10:ac5c` | `_DogX` |
| 151 | 48074 | 2 | 2×1 | `10:ac5e` | `_DogY` |
| 152 | 48076 | 2 | 2×1 | `9:77ae` | `_EatCnt` |
| 153 | 48078 | 2 | 2×1 | `9:7402` | `_EatCountB` |
| 154 | 48080 | 2 | 2×1 | `9:7c8e` | `_EatCountR` |
| 155 | 48082 | 2 | 2×1 | `9:7c44` | `_FlyAwayB` |
| 156 | 48084 | 2 | 2×1 | `9:8078` | `_FlyAwayR` |
| 157 | 48086 | 2 | 2×1 | `9:9e84` | `_FoodA` |
| 158 | 48088 | 2 | 2×1 | `9:9ea4` | `_FoodB` |
| 159 | 48090 | 2 | 2×1 | `9:72de` | `_FoodR` |
| 160 | 48092 | 2 | 2×1 | `8:8a7e` | `_FootHere` |
| 161 | 48094 | 2 | 2×1 | `9:7a4e` | `_FootX` |
| 162 | 48096 | 2 | 2×1 | `9:7a58` | `_FootY` |
| 163 | 48098 | 2 | 2×1 | `9:9fe4` | `_FuzLocX` |
| 164 | 48100 | 2 | 2×1 | `9:9fea` | `_FuzLocY` |
| 165 | 48102 | 2 | 2×1 | `9:9c28` | `_GamePaused` |
| 166 | 48104 | 2 | 2×1 | `8:8614` | `_GameSpeed` |
| 167 | 48106 | 2 | 2×1 | `10:ac86` | `_HealthB` |
| 168 | 48108 | 2 | 2×1 | `10:ac88` | `_HealthR` |
| 169 | 48110 | 2 | 2×1 | `8:8670` | `_HistCnt` |
| 170 | 48112 | 2 | 2×1 | `9:9e6e` | `_HistStart` |
| 171 | 48114 | 2 | 2×1 | `9:9b22` | `_HouseColonies` |
| 172 | 48116 | 2 | 2×1 | `9:9e8c` | `_InitialLions` |
| 173 | 48118 | 2 | 2×1 | `9:78dc` | `_JustToBeMean` |
| 174 | 48120 | 2 | 2×1 | `9:9bbe` | `_LastMowX` |
| 175 | 48122 | 2 | 2×1 | `9:9bd4` | `_LastMowY` |
| 176 | 48124 | 2 | 2×1 | `9:80ba` | `_LastThemeSong` |
| 177 | 48126 | 2 | 2×1 | `8:836e` | `_LayDownQueenMode` |
| 178 | 48128 | 2 | 2×1 | `9:9fb0` | `_LessonTemp` |
| 179 | 48130 | 2 | 2×1 | `8:8a88` | `_LionIndex` |
| 180 | 48132 | 2 | 2×1 | `9:80f0` | `_ListIndexA` |
| 181 | 48134 | 2 | 2×1 | `9:99d4` | `_ListIndexB` |
| 182 | 48136 | 2 | 2×1 | `9:72cc` | `_ListIndexR` |
| 183 | 48138 | 2 | 2×1 | `8:8a56` | `_MakeMe` |
| 184 | 48140 | 2 | 2×1 | `8:8610` | `_MapMode` |
| 185 | 48142 | 2 | 2×1 | `10:cc76` | `_MapPlane` |
| 186 | 48144 | 2 | 2×1 | `10:cf54` | `_MeBase` |
| 187 | 48146 | 2 | 2×1 | `9:9bc4` | `_MeCmd` |
| 188 | 48148 | 2 | 2×1 | `10:ce98` | `_MeColor` |
| 189 | 48150 | 2 | 2×1 | `9:72e4` | `_MeCrazyCnt` |
| 190 | 48152 | 2 | 2×1 | `9:a0d8` | `_MeCrazyDir` |
| 191 | 48154 | 2 | 2×1 | `9:78a4` | `_MeCrazyRot` |
| 192 | 48156 | 2 | 2×1 | `10:cf50` | `_MeDir` |
| 193 | 48158 | 2 | 2×1 | `9:9fe2` | `_MeDis` |
| 194 | 48160 | 2 | 2×1 | `10:ce92` | `_MeEgg` |
| 195 | 48162 | 2 | 2×1 | `8:8a6a` | `_MeEggCarried` |
| 196 | 48164 | 2 | 2×1 | `8:8a6e` | `_MeFoodInType` |
| 197 | 48166 | 2 | 2×1 | `9:9be0` | `_MeGoalPlane` |
| 198 | 48168 | 2 | 2×1 | `9:80c6` | `_MeGoalX` |
| 199 | 48170 | 2 | 2×1 | `9:80d2` | `_MeGoalY` |
| 200 | 48172 | 2 | 2×1 | `10:ac8a` | `_MeHealth` |
| 201 | 48174 | 2 | 2×1 | `9:9cf0` | `_MeHealthCntDwn` |
| 202 | 48176 | 2 | 2×1 | `9:a0d6` | `_MeLastX` |
| 203 | 48178 | 2 | 2×1 | `9:a0da` | `_MeLastY` |
| 204 | 48180 | 2 | 2×1 | `10:cd88` | `_MeLocX` |
| 205 | 48182 | 2 | 2×1 | `10:ce7e` | `_MeLocY` |
| 206 | 48184 | 2 | 2×1 | `9:9fe8` | `_MeMode` |
| 207 | 48186 | 2 | 2×1 | `9:7d24` | `_MeMoveMe` |
| 208 | 48188 | 2 | 2×1 | `8:8a6c` | `_MeNestStarted` |
| 209 | 48190 | 2 | 2×1 | `10:ce80` | `_MePlane` |
| 210 | 48192 | 2 | 2×1 | `9:80aa` | `_MePrevDis` |
| 211 | 48194 | 2 | 2×1 | `9:7d60` | `_MeSMode` |
| 212 | 48196 | 2 | 2×1 | `10:ce88` | `_MeScent` |
| 213 | 48198 | 2 | 2×1 | `9:9c7e` | `_MeSteps` |
| 214 | 48200 | 2 | 2×1 | `9:99e4` | `_MeTargIndex` |
| 215 | 48202 | 2 | 2×1 | `9:72ee` | `_MeTargLife` |
| 216 | 48204 | 2 | 2×1 | `9:9c6c` | `_MeTargLifePlane` |
| 217 | 48206 | 2 | 2×1 | `9:99e8` | `_MeTargLifeX` |
| 218 | 48208 | 2 | 2×1 | `9:99ea` | `_MeTargLifeY` |
| 219 | 48210 | 2 | 2×1 | `10:cc84` | `_MeType` |
| 220 | 48212 | 2 | 2×1 | `9:9bec` | `_MeWarnHealth` |
| 221 | 48214 | 2 | 2×1 | `8:8a58` | `_ModeMe` |
| 222 | 48216 | 2 | 2×1 | `9:9af4` | `_MowX` |
| 223 | 48218 | 2 | 2×1 | `9:9afc` | `_MowY` |
| 224 | 48220 | 2 | 2×1 | `8:8a62` | `_NextFeed` |
| 225 | 48222 | 2 | 2×1 | `8:8a84` | `_NodeCnt` |
| 226 | 48224 | 2 | 2×1 | `8:8a86` | `_NodeNum` |
| 227 | 48226 | 2 | 2×1 | `10:cc78` | `_OldMapPlane` |
| 228 | 48228 | 2 | 2×1 | `9:9b1e` | `_PillDir` |
| 229 | 48230 | 2 | 2×1 | `9:78d4` | `_PillarSeg` |
| 230 | 48232 | 2 | 2×1 | `8:8a8a` | `_PillarState` |
| 231 | 48234 | 2 | 2×1 | `8:8a8c` | `_PillarX` |
| 232 | 48236 | 2 | 2×1 | `8:8a8e` | `_PillarY` |
| 233 | 48238 | 2 | 2×1 | `10:ac8c` | `_QueenStorageB` |
| 234 | 48240 | 2 | 2×1 | `10:ac8e` | `_QueenStorageR` |
| 235 | 48242 | 2 | 2×1 | `9:7896` | `_RColoniesKilled` |
| 236 | 48244 | 2 | 2×1 | `9:9c80` | `_RColoniesStarted` |
| 237 | 48246 | 2 | 2×1 | `9:810c` | `_RainCnt` |
| 238 | 48248 | 2 | 2×1 | `10:ac72` | `_RainOn` |
| 239 | 48250 | 2 | 2×1 | `9:9fec` | `_RebornX` |
| 240 | 48252 | 2 | 2×1 | `9:9fee` | `_RebornY` |
| 241 | 48254 | 2 | 2×1 | `9:9bee` | `_RedDestX` |
| 242 | 48256 | 2 | 2×1 | `9:9c22` | `_RedDestY` |
| 243 | 48258 | 2 | 2×1 | `9:80a6` | `_RedLocX` |
| 244 | 48260 | 2 | 2×1 | `9:80ac` | `_RedLocY` |
| 245 | 48262 | 2 | 2×1 | `9:a0e4` | `_RedLost` |
| 246 | 48264 | 2 | 2×1 | `9:7606` | `_RedPlane` |
| 247 | 48266 | 2 | 2×1 | `9:79dc` | `_RedQueens` |
| 248 | 48268 | 2 | 2×1 | `9:9d74` | `_RedTask` |
| 249 | 48270 | 2 | 2×1 | `10:ac84` | `_RpopT` |
| 250 | 48272 | 2 | 2×1 | `9:78da` | `_SCorpseBase` |
| 251 | 48274 | 2 | 2×1 | `9:729e` | `_SMode` |
| 252 | 48276 | 2 | 2×1 | `9:77ac` | `_Scycle2` |
| 253 | 48278 | 2 | 2×1 | `9:7290` | `_Scycle` |
| 254 | 48280 | 2 | 2×1 | `9:8072` | `_SpidBurpCnt` |
| 255 | 48282 | 2 | 2×1 | `10:ac80` | `_SpidDir` |
| 256 | 48284 | 2 | 2×1 | `9:9bd2` | `_SpidOn` |
| 257 | 48286 | 2 | 2×1 | `9:72c2` | `_SpidRevenge` |
| 258 | 48288 | 2 | 2×1 | `10:ac7c` | `_SpidX` |
| 259 | 48290 | 2 | 2×1 | `10:ac7e` | `_SpidY` |
| 260 | 48292 | 2 | 2×1 | `9:7a5e` | `_Starg` |
| 261 | 48294 | 2 | 2×1 | `9:80a8` | `_StargLife` |
| 262 | 48296 | 2 | 2×1 | `9:9b8a` | `_StrategicModeB` |
| 263 | 48298 | 2 | 2×1 | `9:7690` | `_StrategicModeR` |
| 264 | 48300 | 2 | 2×1 | `9:78b8` | `_SuserX` |
| 265 | 48302 | 2 | 2×1 | `9:78ea` | `_SuserY` |
| 266 | 48304 | 2 | 2×1 | `9:7d58` | `_SwarmCentX` |
| 267 | 48306 | 2 | 2×1 | `9:7efe` | `_SwarmCentY` |
| 268 | 48308 | 2 | 2×1 | `9:807a` | `_SwarmCntB` |
| 269 | 48310 | 2 | 2×1 | `9:9c26` | `_SwarmCntR` |
| 270 | 48312 | 2 | 2×1 | `9:99e6` | `_SwarmDelayB` |
| 271 | 48314 | 2 | 2×1 | `9:9b30` | `_SwarmDelayR` |
| 272 | 48316 | 2 | 2×1 | `9:9b6e` | `_TERRAINset` |
| 273 | 48318 | 2 | 2×1 | `9:7c48` | `_TileMassXB` |
| 274 | 48320 | 2 | 2×1 | `9:9fba` | `_TileMassXR` |
| 275 | 48322 | 2 | 2×1 | `9:7c90` | `_TileMassYB` |
| 276 | 48324 | 2 | 2×1 | `9:9fd2` | `_TileMassYR` |
| 277 | 48326 | 2 | 2×1 | `9:72c8` | `_TilesDugB` |
| 278 | 48328 | 2 | 2×1 | `9:7a56` | `_TilesDugR` |
| 279 | 48330 | 2 | 2×1 | `8:85f0` | `_TutGiven` |
| 280 | 48332 | 2 | 2×1 | `9:7d62` | `_TutHowLong` |
| 281 | 48334 | 2 | 2×1 | `8:85ee` | `_TutLesson` |
| 282 | 48336 | 2 | 2×1 | `8:85ec` | `_TutorCnt` |
| 283 | 48338 | 2 | 2×1 | `9:99d8` | `_WaterTable` |
| 284 | 48340 | 2 | 2×1 | `9:7a5c` | `_YardCycle` |
| 285 | 48342 | 2 | 2×1 | `10:ce82` | `_YardMode` |
| 286 | 48344 | 2 | 2×1 | `9:9c24` | `_YellowDied` |
| 287 | 48346 | 2 | 2×1 | `9:9be4` | `_casteButtonState` |
| 288 | 48348 | 2 | 2×1 | `9:72a0` | `_modeButtonState` |
| 289 | 48350 | 2 | 2×1 | `8:8a60` | `_AlwaysHealthy` |
| 290 | 48352 | 2 | 2×1 | `9:7c92` | `_BoyIsMowing` |
| 291 | 48354 | 2 | 2×1 | `9:9cea` | `_BlackWon` |
| 292 | 48356 | 2 | 2×1 | `8:8630` | `_CasteAuto` |
| 293 | 48358 | 2 | 2×1 | `8:8a82` | `_FootTog` |
| 294 | 48360 | 2 | 2×1 | `10:ac5a` | `_ForSaleState` |
| 295 | 48362 | 2 | 2×1 | `9:9dda` | `_IsGameOver` |
| 296 | 48364 | 2 | 2×1 | `8:8a5c` | `_Invincible` |
| 297 | 48366 | 2 | 2×1 | `9:9c70` | `_JustXfered` |
| 298 | 48368 | 2 | 2×1 | `9:75fa` | `_MeDropAlarm` |
| 299 | 48370 | 2 | 2×1 | `9:9e7c` | `_MeStartedFight` |
| 300 | 48372 | 2 | 2×1 | `9:9af2` | `_MeWantFood` |
| 301 | 48374 | 2 | 2×1 | `9:9fce` | `_ModeAuto` |
| 302 | 48376 | 2 | 2×1 | `8:8a5e` | `_NeverHungry` |
| 303 | 48378 | 2 | 2×1 | `8:8a64` | `_RedInitOn` |
| 304 | 48380 | 2 | 2×1 | `8:8a5a` | `_RevSpider` |
| 305 | 48382 | 2 | 2×1 | `8:8632` | `_ShowPercentCaste` |
| 306 | 48384 | 2 | 2×1 | `9:7602` | `_ShowPercentMode` |

## Estruturas principais

Fontes: pseudocódigo do Ghidra em `re_trabalho/decomp/` (fora do git), funções nomeadas pelo `SIMANTW.SYM`, e o `QUICK2.ANT`. Legenda: **[código]** = visto no pseudocódigo, com o endereço da função; **[save]** = observado nos dados do `QUICK2.ANT`; **[hipótese]** = inferência ainda não confirmada.

Convenção de coordenadas: `x` é a coluna (horizontal) e `y` a linha (vertical; nos ninhos, a profundidade). Todos os mapas grandes são guardados **coluna a coluna**: o byte de `(x, y)` fica em `x * altura + y`.

### Mapas de terreno

| Bloco | Dimensão | Índice | Evidência |
|---|---|---|---|
| `_MapA` (superfície) | 128 × 64 | `x * 0x40 + y`, `x & 0x7f` | [código] `_DoCalcTile` (`1018:4a6b`), `_DoWater` (`1020:07c6`), `_ExitHole` (`1020:2db6`) |
| `_MapB` (ninho preto) | 64 × 64 | `x * 0x40 + y`, `x & 0x3f` | [código] `_DoCalcTile`; `_DoWater` percorre `x = 0..63` na linha `y = _WaterTable` com passo `0x40` |
| `_MapR` (ninho vermelho) | 64 × 64 | igual a `_MapB` | [código] idem |
| `_ExitMapB`, `_ExitMapR` | 64 × 64 | igual a `_MapB` [hipótese] | [save] 255 em toda a linha `y = 0` e valores 254, 253, … descendo pelo túnel: parece um campo de distância até a saída [hipótese] |
| `_HoleMapB`, `_HoleMapR` | 64 | por coluna `x` do ninho [hipótese] | [save] único valor não nulo em `x = 32`, a coluna da entrada do túnel |

- `_DoCalcTile` escolhe o mapa pelo plano `_MapPlane`: 0 e 1 usam `_MapA`, 2 usa `_MapB`, 3 usa `_MapR` [código]. Nos ninhos, o tile de chão desenhado é `valor − 0x70` (byte, com estouro) [código].
- Na superfície, o valor é o próprio tile de chão [código]. `_DoWater` sorteia grama com `_SRand1(0xe)` (0 a 13) e trata 0x74–0x77 como poças que crescem com a chuva [código]; `_ExitHole` só aceita sair em células `< 0x50` [código]. No `QUICK2.ANT` os valores vão de 0 a 114 [save].
- Inundação dos ninhos (`_DoWater`): quando `_WaterTable` sobe, a linha `y = _WaterTable` vira `'N'` (0x4E) se o valor era `< 0x20`, ou `valor + 0x2F` se não; ao secar, faz o inverso (`'N'` vira `_SRand1(8)`) [código]. Com `_WaterTable = 64` (nenhuma linha inundada) os ninhos do `QUICK2.ANT` têm valores de 0 a 0x2E, e 0x2E (terra maciça [hipótese]) ocupa ~97,5% das células [save].
- Confirmação da ordem coluna-major com dados: as formigas das listas B e R ficam em células diferentes de 0x2E (túnel escavado), o túnel desce em diagonal de `(32, 0)` até `(36, 9)` e a linha `y = 0` é toda diferente de 0x2E [save; teste `formigas_dos_ninhos_estao_em_tuneis_coluna_major`].

### Mapas de feromônio

`_PherMapA`, `_PherMapBN`, `_PherMapBT`, `_PherMapRN`, `_PherMapRT`: 2048 bytes cada, **64 × 32**, uma célula para cada bloco de 2 × 2 tiles da superfície, índice `(x >> 1) * 0x20 + (y >> 1)` [código: `_DoCalcTile`, que escolhe o mapa por `_ELayerMode` 0 a 4 nessa ordem e desenha `(valor >> 4) − 0x10` quando `valor > 0x10`]. Os cinco usam coordenadas da superfície nessa função; se `BN`/`BT`/`RN`/`RT` também valem dentro dos ninhos não foi visto [hipótese: só superfície].

### Mapas de vida (não salvos)

`_LifeA` (128 × 64), `_LifeB` e `_LifeR` (64 × 64) guardam, por célula, o campo T da formiga que está ali (0 = vazia; 0xFF e 0xFE = formiga amarela) [código: `_DoCalcTile`, `_AddAntToAList`]. **Não estão no save**: `_DoLoadInitializations` (`1010:9dbe`) zera os três e os reconstrói a partir das listas, percorrendo cada uma de `_ListIndex?` até 0 inclusive (a entrada de menor índice vence quando duas ocupam a mesma célula), e depois chama `_SetMyLife` para a formiga amarela se `_MeMode == 0` e `_FullCount` [código]. Regras completas em [telas/jogo.md](../telas/jogo.md#camada-de-vida).

### Listas de formigas

Cada área tem cinco vetores paralelos de bytes, um elemento por formiga: `X`, `Y`, `M`, `T`, `S`. Capacidade: 1000 em `A`, 500 em `B` e `R` [código: limites em `_AddAntToAList` (`1020:2ef0`), `_AddAntToBList` (`1020:2f4a`), `_AddAntToRList` (`1020:2fa4`)].

- **A lista é da área, não da colônia.** `A` = formigas na superfície; `B` = dentro do ninho preto; `R` = dentro do ninho vermelho. A colônia de cada formiga está no bit 7 de `T`. `_DoNestAntB` (`1028:2dae`) trata formigas vermelhas (`T & 0x80`) dentro da lista B, que invadem o ninho preto [código]; no `QUICK2.ANT` a lista A tem uma operária preta (T = 17) e uma vermelha (T = 150) [save].
- **Contadores:** `_ListIndexA`, `_ListIndexB`, `_ListIndexR` (salvos) = número de entradas usadas, `0..ListIndex` [código]. `_AddAntTo?List` grava na posição `ListIndex` e incrementa. Uma entrada com `T = 0` é um buraco (formiga morta/removida): `_CompactListA/B/R` (`1020:2a16`/`2a7a`/`2ade`) desloca as entradas seguintes e diminui o contador; `_DoAntSim` (`1028:0000`) chama uma compactação a cada 32 ciclos, alternando A, B, R [código]. `_RemoveFromAList` (`1020:2b42`) remove por deslocamento (`_BlockMove`) e zera a célula em `_LifeA` [código]. `_BuildAntListA` (`1020:3046`) reconstrói a lista A varrendo `_LifeA` (x de 0 a 127, y de 0 a 63) [código].
- **X, Y:** coordenadas da célula no mapa da área (A: x 0–127, y 0–63; B/R: 0–63) [código: `_AddAntTo?List` também grava `T` em `_Life?[x * 0x40 + y]`].
- **T (tipo)** [código: `_CountAnts` (`1020:04de`), `_GetFromAlist` (`1020:2ffe`), `_DrownBList` (`1020:2d16`), `_DoCalcTile`]:
  - bit 7: colônia vermelha (1) ou preta (0);
  - bits 3–6: classe `k` (0–15). `_CountAnts` conta `T >> 3` (32 contadores, pretos 0–15 e vermelhos 16–31);
  - bits 0–2: para tipos `< 8` (classe 0), o desenho usa `_MeEgg` em vez de `_MeDir`, logo classe 0 = crias e os bits baixos indicam o estágio [hipótese]; nos adultos, direção 0–7 [hipótese, apoiada por `_lifeTile = T + 0x100` (superfície) ou `T + 0x200` (ninho), em que T seria o índice do sprite].
  - `_CountAnts` agrupa as classes em `_CastePopB/R[6]`: `[0] = k0`, `[1] = k1+k2+k3+k5`, `[2] = k6+k7+k9`, `[3] = k4`, `[4] = k8`, `[5] = k12`, e faz `_BpopT/_RpopT = soma de [1..5]` (sem as crias) [código]. Se `[5]` passa de não-zero a zero, toca música e mostra o diálogo de fim de colônia, e no jogo rápido (`_CurGameType < 2`) liga `_IsGameOver` e define `_BlackWon` [código], então `k12` = rainha.
  - Nomes das outras classes [hipótese, a confirmar pelos bitmaps/sprites]: grupo `[1]` operárias, `[2]` soldados, `[3]`/`[4]` reprodutores (machos/fêmeas aladas). `k13` aparece sempre ao lado da rainha (T = 110 em x = 35 com a rainha T = 102 em x = 34; na colônia vermelha 234 ao lado de 226) e não é contado em nenhum grupo: provavelmente a segunda metade do sprite da rainha [hipótese]. `k14` (`T = 0x70 | cor`) é gravado como resultado de luta em `_DoNestAntB` (cadáver? [hipótese]).
  - `_DrownBList`/`_DrownRList` afogam (M = 0x11) as formigas da linha `y = _WaterTable` com classe 1–11, isto é, nem crias nem rainha [código].
- **M (modo/tarefa):** índice de 0 a 19 dos contadores por modo (`20` palavras por colônia, zerados a cada ciclo em `_DoAntSim` e somados em `_ModePopB/R[6]`: `[0] = m2+m3`, `[1] = m4+m5`, `[2] = m1`, `[3] = m7`, `[4] = m12`, `[5] = m6`) [código: `_DoNestAntB`, `_DoAntSim`/`_TallyModePop` (`1028:038e`)]. Valores vistos: 2 ao reconstruir a lista A [código: `_BuildAntListA`]; 7 = invadir ninho (`_RaidInB`) [código]; 10 depois de vencer uma luta [código]; 0x11 = afogada [código]; 8 nas crias e 9 na rainha e na sua metade [save]. Nomes dos demais modos [em aberto].
- **S:** valor auxiliar do modo. `_ExitHole` grava o 5.º argumento só quando M = 6, 0x78 em certos casos e 0 nos demais; `_DoNestAntB` grava ali o resultado de `_GetWinner` [código]. Significado geral [hipótese: alvo/contador do modo].
- **Defasagem dos contadores** [save + código]: `_CountAnts` só roda a cada 64 ciclos (`_DoAntSim`, fases `(_Cycle & 0x60) >> 5` = 0 e 2; ver [simulacao-visao-geral.md](../regras/simulacao-visao-geral.md), §5.1). No `QUICK2.ANT`, `_CastePopB = [2, 2, 0, 0, 0, 1]` mas há 3 ovos pretos na lista B; os índices 1–5 e os totais (`_BpopT = 3`, `_RpopT = 2`) batem com a recontagem das listas mais a formiga amarela (`_MeType = 16`, classe 2, contada quando `_MeMode == 0`) [teste `contadores_de_populacao_batem_com_as_listas`]. A colônia da formiga amarela na contagem depende de `_grndTile + 2`, que não é salvo [código]; no `QUICK2.ANT` ela é preta. **No load os contadores são refeitos** [observado na bancada QEMU, 2026-10-16]: o `QUICK2.ANT` carregado e lido da memória tem `_CastePopB` = `[3, 2, 0, 0, 0, 1]` e `_CastePopR` = `[4, 1, 0, 0, 0, 1]` (os ovos da lista), e um save com a operária preta da lista A removida dá `_BpopT` = 2. O load também liga `_GamePaused` (o jogo carregado fica pausado). Ver [../../bancada-qemu.md](../../bancada-qemu.md#carregar-um-save-injetado-passos-observados-em-2026-10-16).

### Quintal (patches) e pontos

- `_YMapPopB`, `_YMapPopR` (192 bytes): população por patch do quintal, **12 × 16**, índice `x * 0x10 + y`, com `(x, y) = _CurYardPnt`; `_SimColonies` (`1030:35f8`) grava ali `min(_BpopT & 0x3ff, 250)` e `min(_RpopT & 0x3ff, 250)` no patch atual [código]. Como `x * 16` vai até 176, `x` vai de 0 a 11 e `y` de 0 a 15; qual eixo é horizontal na tela não foi verificado [hipótese]. No `QUICK2.ANT`, o patch `(11, 8)` (índice 184) tem 3 e 2, iguais a `_BpopT`/`_RpopT` [save].
- `_YMapSeeds` (192 palavras): semente por patch, índice `y * 12 + x` (ordem trocada em relação a `_YMapPop`), passada a `_RandWorld` (`1030:5ab0`) ao entrar num patch, junto com as populações `_YMapPopB/R` do patch [código: `_XferPatch` (`1000:ab90`)].
- Blocos `*Pnt` de 4 bytes são **pares de palavras (x, y)**, não inteiros de 32 bits [código: `_DoLoadInitializations` grava `_MeLocX`/`_MeLocY` em `_YMapPnt` (plano 0), `_AMapPnt` (1), `_BMapPnt` (2) ou `_RMapPnt` (3) conforme `_MePlane`]. `_CurYardPnt` = patch atual (no `QUICK2.ANT`, `(11, 8)`).
- `_GameTime` (4 bytes) é um contador de 32 bits: `_DoAntSim` incrementa a palavra baixa e propaga o vai-um para a alta [código].
- O prefixo `Y` em `_YMap*` significa quintal (*yard*), não formiga amarela [código: índice pelo patch atual]. O plano 0 de `_MePlane` corresponde à vista do quintal [hipótese].

Impacto no port: o formato de save do port deve ser próprio (versionado e legível, por exemplo JSON com as mesmas chaves). O `.ANT` original entra só como importação, para testes de comparação com o original.
