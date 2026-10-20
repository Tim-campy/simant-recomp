*Leia em português: [pt-BR/glossario.md](pt-BR/glossario.md)*

# Glossary of internal names

Names taken from `SIMANTW.SYM` and from the data. Meanings marked with [hypothesis] were inferred from the name and have not yet been confirmed in the code.

| Name | Meaning | Source |
|---|---|---|
| `A` (suffix/prefix: `_MapA`, `_AlistX`, `_DoAntSimA`) | surface / yard [hypothesis] | SYM; `_DoAntSimA` handles foraging, recruitment and fighting outside the nest |
| `B` (`_MapB`, `_BlistX`, `_DoAntSimB`) | **black** nest / colony (the player's) | SYM (`_PlaceBlackQueen`, `_BlkQueens`) |
| `R` (`_MapR`, `_RlistX`, `_DoAntSimR`) | **red** nest / colony (the opponent) | SYM (`_PlaceRedQueen`) |
| `Y` (`_DoAntSimY`, `_YellowBirth`) | the **yellow ant**, controlled by the player | SYM; "Yellow Health" bitmaps |
| `Alist`/`Blist`/`Rlist` + `X`,`Y`,`M`,`T`,`S` | parallel lists of ants **per area** (surface, black nest, red nest), of any colony: X/Y position, M = mode/task, T = type (bit 7 = red, bits 3–6 = class, 12 = queen), S = mode auxiliary [hypothesis] | `_AddAntToAList`, `_CountAnts`, `_DoNestAntB`; [formatos/save-ant.md](spec/formatos/save-ant.md#main-structures) |
| `ListIndexA/B/R` | number of entries used in each list | `_AddAntToAList`, `_CompactListA` |
| `LifeA/B/R` | occupancy map (T field per cell), rebuilt from the lists on load; not saved | `_DoLoadInitializations` |
| `_gameCycles` | iterations of the `MYTIMERFUNC` loop (32 bits), not ticks; depends on the host clock; not in the save | `MYTIMERFUNC`; measured on the test bench ([testes-tick.md](spec/regras/testes-tick.md)) |
| `_BirdDelay` | next value of `_gameCycles` at which `_SimBird` tests whether the bird comes (`_gameCycles` + 20 or + 30); saved, but not reproducible across runs | `_SimBird` `1030:28aa` |
| `YMap…` (`_YMapPopB`, `_YMapSeeds`, `_YMapPnt`) | map of the whole yard (*yard*), 12 × 16 patches; not the yellow ant | `_SimColonies`, `_XferPatch` |
| `CastePop`, `ModePop` | populations per caste group and per mode group (6 values each), recalculated by the simulation | `_CountAnts`, `_TallyModePop` |
| `PherMap` (`A`, `BN`, `BT`, `RN`, `RT`) | pheromone maps: alarm (A), nest (N) and trail (T) per colony [hypothesis, supported by string list 4/1000: "Black Nest Marker Pheromone", "Black Trail…"] | save table, SHARED 4/1000 |
| `HoleMap`, `ExitMap` | nest entrances/exits | save table |
| `Caste` (`_casteLevels`, `_IdealCaste`) | caste division: workers, soldiers, breeders (control triangle) | "Workers/Soldiers/Breeders" bitmaps |
| `Mode` (`_modeLevels`) | task division: forage, dig, nurse the brood (control triangle) | "Forage/Dig/Nurse" bitmaps |
| `Yard` | the whole yard seen from above, divided into "patches" | SYM (`_InitSimYard`, `_YardToMap`) |
| `Patch` | each square of the yard; a full game tries to dominate all of them | SYM (`_MakeYardPatch`, `_GetNearbyPatches`) |
| `Edit` (window) | the close-up/zoom view where the yellow ant walks | SYM (`_OpenEditWindow`), `editWin…` names in SHARED 1/0 |
| `Lion` | antlion | SYM (`_InitAntLions`) |
| `Pillar`/`Pill` | caterpillar [hypothesis: "caterpillar"] | SYM |
| `Sow` | [open] | SYM (`_InitSow`, `_DoSow`) |
| `Swarm` | nuptial mating flight (males and new queens) [hypothesis] | SYM (`_MaintainSwarm`, `_SwarmXB`) |
| `Boy`/`Kid`, `Cat`, `Dog`, `Bird`, `Mower` | human, cat, dog, bird and lawnmower in the yard | SYM; sprites in WINGANT |
| `db_*`, `DB*`, `ch_*` | `.NDX/.DAT` resource database and object cache | SYM; [formatos/banco-dat-ndx.md](spec/formatos/banco-dat-ndx.md) |
| `win_*` | the game's own window system (ported from the Mac) | SYM |
| `displayType` | video mode chosen by `SIMANT.CFG` | `_ReadConfig` |
| `_grndTile`, `_lifeTile` | index of the ground tile and of the life tile of the cell being drawn, computed by `_DoCalcTile` | code; [formatos/tiles.md](spec/formatos/tiles.md) |
| `_MapPlane` | map shown in the edit window: 0 yard (draws `_MapA`), 1 surface, 2 black nest, 3 red nest | `_DoCalcTile`, `_SetMapPlane`; [telas/jogo.md](spec/telas/jogo.md) |
| `_MapPnt` | map cell at the top-left corner of the edit window (x, y pair at `DS:7d2e`/`7d30`) | `FUN_1010_6250`, `_CenterEdit` |
| `_YMapPnt`, `_AMapPnt`, `_BMapPnt`, `_RMapPnt` | remembered center of the view in each plane (0 to 3) | `_SetMapPlane`, `_DoLoadInitializations` |
| `_DoEditScroll` | handles `WM_HSCROLL`/`WM_VSCROLL` of the edit window (line, page, thumb); does not update the remembered point | `MAINWNDPROC`; [telas/jogo.md](spec/telas/jogo.md), "Navigation" |
| `_SetMapPlane`, `_SetMapPlaneLocation`, `_GotoMapPoint` | switch the displayed plane and center the view on the remembered point (or on a given point) | `_ProcMapRibbonEvent`, `_ProcMenu` |
| `_goStep*` | move the view one step and update the remembered point; no calls in the pseudocode [hypothesis: leftover from the Mac version] | code |
| `_GotoMyAnt` | takes the view to the yellow ant (`_CenterEdit` or `_SetMapPlaneLocation`); called by AutoTrack, Home, Backspace... | code (`1000:a764`) |
| `_gameCycles` | iterations of the timer loop (`MYTIMERFUNC`), even without a tick; in the port, `Estado::ciclos_jogo` | code (`9:9b2c`) |
| `_ScrollUp/Down/Left/Right`, `…S` | empty functions in the Windows version | code |
| `_editWidth`, `_editHeight` | size of the view in tiles: `editTileArea` (+1 with the ribbon) divided by 16, rounded up | `_win_EditChanged` |
| `_Me*` (`_MeLocX/Y`, `_MePlane`, `_MeType`, `_MeDir`, `_MeEgg`, `_MeBase`, `_MeMode`) | state of the yellow ant; with `_MeMode` = 0 it is written as 0xFF in the life layer | `_SetMyLife`, `_DoCalcTile` |
| `_TERRAINset` | active ground tileset: 0 yard (9/10), 1 house (9/9) | `_OverlayTileSet` |
| `_OpenMiniMapWin`, `_Mini_DrawMapI`, `_miniMapRect`, `_mmapCursorState` | mini map (window 20), opened by the `ribToYellow` button of ribbon 34: modal, 1 pixel per cell; `_miniMapRect` = map area, `_mmapCursorState` = drawn view rectangle | `1010:e374`, `1010:d474`; telas/mapa.md §7 |
| `_ELayerMode` | pheromone layer overlaid on the surface (−1 = none, 0–4 = `_PherMapA/BN/BT/RN/RT`) | `_DoCalcTile` |
| `_LifeA/B/R` | life map (ants and others) per cell, drawn over the ground; not in the save | `_DoCalcTile` |
| `tilebufover`, `tilebufnest` | life tile buffers of the surface (9/15–17) and of the nests (9/18–20) | `_LoadTiles` |
| `_TickCount` | Windows clock in 55 ms units (`GetTickCount() / 55`): deadlines for messages and frames | `1008:18ac`; [telas/mensagens.md](spec/telas/mensagens.md) |
| `_EditMessage` / `_DrawRibbonMessage` | short ribbon message (`ribMessageArea`), timed or permanent, "important" or not | `1000:92c0`, `1000:0b26`; mensagens.md §2 |
| `_PictStrnDialog` / `_PictureDialog` | modal message frame (window 30, `picStrWin`): SHARED 4 text, optional picture, stops the simulation | `1000:615a`, `1000:62bc`; mensagens.md §3 |
| `_EndGameDialog` / `_CalcScore` | end-of-game dialog (window 4) and the score | `1000:6602`, `1000:5530`; mensagens.md §4 |
| `_DoUserButton` / `ribUserDef1A`–`1H` | configurable ribbon buttons (pause, speed, options, save…) | `1000:04c4`; mensagens.md §5 |
| `_OptionStates[0..5]` | options AutoTrack, Music, Sound Effects, Events (frames), Messages (ribbon), Silly (balloons) | `8:85f2`; formiga-amarela.md §11.7 |
| `_myBeginSong` / `_myBeginSound` | play music (id from the type 18 header) / effect (id of type 5) | `GR_MODULE`; [regras/som.md](spec/regras/som.md) |
| `_songsOnFlag`, `_effectsOnFlag` | music and effects enabled (copies of `_OptionStates[1]` and `[2]`, zeroed without focus) | `_myBeginSong`, `MAINWNDPROC` |
| DAC (`_UncompressDACInstrument`) | type 5 sound effect: 8 bits in 4-bit deltas | [formatos/som.md](spec/formatos/som.md) |
| `_CurGameType` | game type: 0 Tutorial, 1 Quick Game, 2 Full Game, 3 Experimental | `_NewGame` (`1000:d6dc`); [telas/abertura.md](spec/telas/abertura.md) |
| `_DoScenario` | the "Select a Game" (window 2): returns the id of the clicked object | [telas/abertura.md](spec/telas/abertura.md) |
| `_YardMode` | mode of the yard window: 0 Yard View, 1 House View, 2 Strategic View, 3 Graphic View | `_SetYardMode`; [telas/quintal.md](spec/telas/quintal.md) |
| `_CurYardPnt` / `_YMapPnt` | current patch / patch chosen in the yard window (12 × 16 patches) | `_XferPatch`, `_YardArea` |
| `_XferPatch` | patch change in the Full Game (generates the new patch with `_RandWorld`) | `1000:ab90`; [telas/quintal.md](spec/telas/quintal.md) §5 |
| `_PlaceQueenInYard` | PLACE button: puts a stored queen in a patch up to 3 away from the current one | `1000:ad12` |
| `_patchRgn`, `_patchRgn2` | polygon of a patch in the yard window (cursor/click and colony color) | DGROUP `1048:1894`, `1048:18a4` |
| `_MakeHousePatch` | ground of a house patch (`n = px × 16 + py` ≤ 37) | `1020:3120`; [regras/geracao-mundo.md](spec/regras/geracao-mundo.md) §5 |
| `_DROPdir` (`9:9c66`) | direction in which food falls in the house (0 tiles, 1 left carpet, 2 kitchen/plain, 3 right carpet) | geracao-mundo.md §5 |
| `_TileFrame1/2`, `_FillMapLegs` | tile frames; rectangle with fringes and 4 × 4 feet (furniture) | geracao-mundo.md §5 |
| `_CurExpTool`, `_ExpSubStates` | Experimental tool (0–7) and the sub-option of each one | [telas/experimental.md](spec/telas/experimental.md) |
| `_processExp`, `_DoTool` | click/drag on the map in Experimental and the tool applied | `1010:7022`, `1010:727e` |
| `_AntMenu` / `_win_DoProxMenu` | yellow ant menu when holding the button over it; items become `_YellowCommand` | `1010:6d24`, `1030:de62`; [telas/janelas.md](spec/telas/janelas.md) |
| `_casteLevels` / `_modeLevels` | the three levels (0–65535) of the Caste and Behavior Control triangles | `9:9b64`, `9:9c74`; janelas.md §2 |
| `_DoBookMark` | ribbon bookmarks 1–7 (Ctrl+click stores the view) | `1000:08a6`; mensagens.md §2.3 |
| `_LoadGame` / `_SaveGame` / `_FileSelect` | File → Load (question, dialog, `_RandYard`, blocks), Save/Save As and the common file dialog | `1010:9fa0`, `1010:a23e`, `1010:9920`; [telas/arquivos.md](spec/telas/arquivos.md) |
| `_gGameNeedsSaving` | game changed since the last Save/Load (1 at each tick) | `8:8370`; arquivos.md |
| `_StopSimulation` / `_RestartSimulation` | nested `KillTimer`/`SetTimer` around the modal boxes | `1000:0000`, `1000:0034`; mensagens.md §3.5 |
| `_MysteryButton`, `_AddRedAnts`, `_KillSomeAnts` | Mystery button (`_RRand(15)`), new red ants, kill up to 51 from list A | `1000:b0bc`, `1030:6cfe`, `1000:9d5e`; [regras/misterio.md](spec/regras/misterio.md) |
| `_MagnifyMenu` | Examine: window 29 while the button is held down | `1010:6b4c`; janelas.md §4.3 |
| `_SetMapModeAnt`, `_MapMode`, `_showTrails` | mode of the map window (planes and pheromones) and the trails | `1000:9e82`, `8:8610`, DGROUP `1998`; [telas/mapa.md](spec/telas/mapa.md) |
| `_OpenInfoWindow`, `_DisplayCard` | Info ("Top Card"): cards of types 17/19/10/21 | `1018:60da`, `1018:5a1c`; [telas/info.md](spec/telas/info.md) |
| `_MakeBalloon` | the boy's speech balloon (8×8 pieces from DGROUP) | `1008:59e4`; telas/quintal.md §2.3 |
| `_Cheats` (`8:83bc`) / `_CheatKeys` (`1018:67de`) | table of the 26 four-letter cheat codes (inverted bits) and the routine that applies each one | `SIMANTW.SYM`; test bench ([regras/trapacas.md](spec/regras/trapacas.md)) |
| `_GlobalKey` (`9:7866`) | "last key" read by `_DoSimYard` (0xA8 turns on the rain) and `_SimCat` (0x8D); nothing writes it on Windows | SYM; pseudocode (regras/trapacas.md §2.2) |
| `_bHelp` (`10:0010`) | help mode: right click, Shift+F1 and Delete toggle it; the next click on an object opens WinHelp in its context | `MAINWNDPROC`, `_HelpKeyDown`, `_DoEvent`; test bench ([telas/ajuda-contexto.md](spec/telas/ajuda-contexto.md)) |
| `_DoWinHelp` (`1000:5aac`) / `_helpFile` (`9:76ac`) | `WinHelp(HELP_CONTEXT, id & 0xFF00)` with `SIMANT.HLP` ("?" buttons) | pseudocode; telas/ajuda-contexto.md §4 |
| `_DrawCastePopUp` (`1000:5d9e`) | caste frame (window 23 `antPopWin`) open while the button is held down over the ribbon's bar area | pseudocode + test bench ([telas/mensagens.md](spec/telas/mensagens.md) §9.1) |
| `_MeWarnHealth` (`9:9bec`) / `_BlkWarnHealth` (`9:769c`) | health warning thresholds of the yellow ant and of the black colony (30 at the start); clicking the health bar changes them (`_DoHealthSetY`, `_DoWarnSetB`) | pseudocode + test bench (mensagens.md §9.1) |
| `_InitMenu` (`1008:4e36`) | builds the main window's menu bar from database object 6/0; commands `0xFD01 + 16 × menu + item` | pseudocode; [spec/telas/menus.md](spec/telas/menus.md), [spec/formatos/menu.md](spec/formatos/menu.md) |
| `_DoMenuEntry` (`1000:1876`) / `_ProcMenu` (`1000:4fc2`) / `_ProcMenuHelp` (`1000:4fa6`) | `WM_COMMAND` 0xFDxx: in help mode, the item's help; otherwise, what the item does (Load/Save/Quit deferred via `_fileWaitFlag`) | pseudocode; telas/menus.md §2 |
| `_SetMenuEntries` (`1000:4c24`) / `_SetMenuItemState` / `_SetMenuOptionText` | Speed and Options check marks and the Pause/Unpause text | pseudocode; telas/menus.md §2 |
| `_MenuQuit` (`1000:d628`) | "SimAnt Quit" question (Yes = Save As and quit, No = quit, Cancel) | pseudocode, test bench m19/o03; telas/menus.md §4 |
| `_AboutDialog` (`1000:cf3a`) | File → About: window 31, picture and scrolling credits (4/1700), game paused | pseudocode, test bench; telas/menus.md §5 |
| `_GotoSpider` (`1000:a844`) | `ribToSpider` button of ribbon 34 (0x2207) and `editGoSpider`: the view goes to the spider on the surface; without `_SpidOn`, the error sound | pseudocode; telas/jogo.md "Ribbon" |
| `_ScoreDialog` (`1000:5caa`) / `_win_DrawScoreWindow` (`1000:5aca`) | Window → Status (Ctrl+T) and `ribStats` button of ribbon 34 (0x2209): window 24 with the `_CalcScore` grades and the score; the game keeps running | pseudocode, test bench n02; telas/menus.md §6 |
| `_win_Events` (`1030:d040`) | dispatches pending messages and reports whether there was a click or Space/Insert (closes the game's dialogs) | pseudocode |
