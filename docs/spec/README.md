*Leia em português: [README.md](../pt-BR/spec/README.md)*

# SimAnt Specification (Windows 3.x)

Game: **SimAnt: The Electronic Ant Colony** (Maxis, 1991), Windows version by "Lightning" Rod Lai. Genre: simulation/strategy (ant colony). The player controls a yellow ant of the black colony, which competes with the red colony for the yard and the house. The credits and the description come from `WINGANT` bitmaps with ids between 1000 and 4999 (contact sheet at `re_trabalho/png/sheet_wingant_a.png`): the credits screen and the "Select a Game" menu, which offers Tutorial, Quick Game, Full Game, Experimental and Load Game.

## Evidence convention

- **Verified fact:** tested in the original game or validated against the real files with a reproducible tool (the text says which).
- **Fact seen in the code:** read in the Ghidra pseudocode, with the `selector:offset` address and the name from the `.SYM`.
- **[hypothesis]:** inference not yet confirmed.
- **[open]:** known question, not yet answered.

Reference environment: DOSBox-X (`referencia/`). Comparisons with the original are in `tests/referencias/README.md`.

## Index

- [executavel.md](executavel.md): map of `SIMANTW.EXE`, modules and main flow.
- [arquivos-do-jogo.md](arquivos-do-jogo.md): inventory of the installation files.
- Screens:
  - [telas/abertura.md](telas/abertura.md): intro (`_ShowIntro`), implemented in milestone 1.
  - [telas/resolucao.md](telas/resolucao.md): layout per resolution (640×480, 800×600, 1024×768): maximized main window, position table by screen height, and the port's virtual screen.
  - [telas/jogo.md](telas/jogo.md): game screen (ribbon + edit window with ground and life), planes, scrolling and reconstruction of `_LifeA/B/R`.
  - [telas/mensagens.md](telas/mensagens.md): notices to the player (ribbon message, message boxes, game over and score), configurable ribbon buttons and saving.
  - [telas/menus-mods.md](telas/menus-mods.md): menu and panel for mod scripts (API 0.17; a port extension, does not exist in the original).
  - [telas/janelas.md](telas/janelas.md): Caste and Behavior Control (triangle), History, hold-the-button menus (`_AntMenu`, icons), ribbon button state and window manager.
  - [telas/mapa.md](telas/mapa.md): map window (`mapWin`): planes, pheromone maps, trails, spider, footprint and view rectangle.
  - [telas/quintal.md](telas/quintal.md): yard window (Yard View) and ribbon 35, patch switching (`_XferPatch`), PLACE and Full Game.
  - [telas/experimental.md](telas/experimental.md): Experimental mode (menu and editing tools, Quick Game → Experimental, "Start Game") and accelerator table.
  - [telas/info.md](telas/info.md): Info window (encyclopedia cards).
  - [telas/arquivos.md](telas/arquivos.md): Load, Save and Save As dialogs.
  - [telas/ajuda.md](telas/ajuda.md) and [telas/ajuda-contexto.md](telas/ajuda-contexto.md): `SIMANT.HLP` viewer and how help is reached (F1, help mode, "?" buttons, cursors).
- Formats:
  - [formatos/banco-dat-ndx.md](formatos/banco-dat-ndx.md): resource bank and LZSS compression.
  - [formatos/bitmap.md](formatos/bitmap.md): bitmaps and palette.
  - [formatos/fonte.md](formatos/fonte.md): fonts (Mac FONT and Gaia from `FONTRES.FON`) and text positioning.
  - [formatos/janela.md](formatos/janela.md): window system (layouts, colors, positions, on-screen computation and placement).
  - [formatos/simant-cfg.md](formatos/simant-cfg.md): `SIMANT.CFG`.
  - [formatos/save-ant.md](formatos/save-ant.md): saved game and simulation state model.
  - [formatos/som.md](formatos/som.md): sound effects (type 5, type 51), music headers (type 18), MIDI (`SOUND\*.MID`, types 20 and 50) and the original's audio outputs.
  - [formatos/tiles.md](formatos/tiles.md): tiles (type 9 objects) and assembly of the map image.
  - [formatos/hlp.md](formatos/hlp.md): `SIMANT.HLP` (WinHelp 3.0).
  - [formatos/ne-recursos.md](formatos/ne-recursos.md): NE executable resources (cursors).
- Rules:
  - [regras/rng.md](regras/rng.md): random generators (LFSR `_SRand*` and C `rand` in `_RRand`), seeds and who uses each one.
  - [regras/geracao-mundo.md](regras/geracao-mundo.md): new game (`_NewGame`, `_RandYard`, `_RandWorld`), yard and house patches, queens, food and tutorial; verified against `QUICK2.ANT`.
  - [regras/tiles-valores.md](regras/tiles-valores.md): meaning of the values of `_MapA`, `_MapB` and `_MapR` and of the tile test functions.
  - [regras/simulacao-visao-geral.md](regras/simulacao-visao-geral.md): tick (`MYTIMERFUNC`/`_DoAntSim`), order and frequency of the subroutines, speed, pause, game over and score.
  - [regras/formigas.md](regras/formigas.md): A/B/R lists, X/Y/M/T/S fields, castes, modes, control triangles and state machine.
  - [regras/feromonios.md](regras/feromonios.md): scent maps (`_PherMap*`), deposit, decay and reading.
  - [regras/superficie.md](regras/superficie.md): surface ants (`_DoAntSimA` and each mode), direction functions, fighting, corpses, nest entry, food, black/red strategy, recruitment and the red leader ant.
  - [regras/inimigos.md](regras/inimigos.md): antlion, spider (including player-controlled), caterpillar and pill bug.
  - [regras/formiga-amarela.md](regras/formiga-amarela.md): yellow ant (`_Me*` state, movement and path, nest, pick up/drop, eating, fighting, body swap, recruitment, death and reincarnation) and player controls (mouse, keys, menus, accelerators, Experimental mode).
  - [regras/ninho.md](regras/ninho.md): ants inside the black and red nests (`_DoAntSimB/R`, `_DoNestAntB/R`): dispatch by mode, movement, fighting, stored food, digging, exit map, invaders, water and flooding; R × B differences and test cases.
  - [regras/ciclo-de-vida.md](regras/ciclo-de-vida.md): laying, development and hatching of eggs (`_SimEggB/R`), queen (head and tail, walking, hunger), caste choice (`_SetCasteProd`, `_CasteTabC`) and test cases.
  - [regras/testes-tick.md](regras/testes-tick.md): determinism of the original on the QEMU test bench, "state before → after" tick vectors and what to capture besides the save.
  - [regras/quintal.md](regras/quintal.md): yard (`_DoSimYard`): boy inside and outside the house, lawnmower, foot and insecticide, cat, dog, bird, rain and water, 12 × 16 yard colonies (`_SimColonies`, mating flight, migration) and the end of the Full Game; cases Q-01 and Q-02 confirmed in the original.
  - [regras/som.md](regras/som.md): sound functions, Music/Sound Effects options, preemption and a catalog of who plays each music track and sound effect.
  - [regras/trapacas.md](regras/trapacas.md): the original's 4-letter cheat codes (`_Cheats`, `_CheatKeys`), checked on the test bench.
  - [regras/conselhos.md](regras/conselhos.md): `_Feedback`: advice messages (list 4/1020) and tutorial (`_GiveLesson`/`_LessonDone`, lessons 4/11000…11112).
  - [regras/misterio.md](regras/misterio.md): Mystery button (`_MysteryButton`).

## What remains to be specified

The tiles and maps, the simulation rules and the sound, which were on this list, are now specified (index above). What still remains to be specified or implemented is in [../estado-do-projeto.md](../estado-do-projeto.md) §4 (revised on 2026-10-19). The menu bar, the balloons and the window 26 boxes, which were listed here, were done on 2026-10-18 ([telas/menus.md](telas/menus.md), [telas/baloes.md](telas/baloes.md), [telas/mensagens.md](telas/mensagens.md)); the health graphs (`_DrawEditGraphs`) do not draw in the Windows modes ([telas/baloes.md](telas/baloes.md) §3).
