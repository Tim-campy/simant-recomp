*Leia em português: [executavel.md](../pt-BR/spec/executavel.md)*

# `SIMANTW.EXE`: executable map

File analyzed: `SimAnt-T/MAXIS/ANTWIN/SIMANTW.EXE` (516,096 bytes), accompanied by `SIMANTW.SYM` (37,060 bytes, MAPSYM symbol table).

## Tools and how to reproduce

- **REA (MCP):** the tool rejects the executable with "unsupported NE executable in MZ image". REA's Ghidra mode on Windows only accepts PE. For this reason REA was not used on the code, only the check that it is working.
- **Ghidra 12.1.4 headless** (`C:\Ghidra`), NE loader, language `x86:LE:16:Protected Mode`. The scripts are in `re_trabalho/ghidra_scripts/`:
  - `ApplySym.java`: applies the 2,169 names from the `.SYM` and creates 1,307 functions;
  - `ExportDecomp.java`: exports the pseudocode of 1,538 functions (no failures) to `re_trabalho/decomp/CodeN.c`.
- `tools/ne_info.py`: NE header, segments, resources, imports and `.SYM`. Output in `re_trabalho/simantw_ne.json`.
- `tools/save_table.py`: block table of the saved game (see [formatos/save-ant.md](formatos/save-ant.md)).

`re_trabalho/` is working material and **is not part of the port** (it is in `.gitignore`). By the project rules, no decompiled excerpt may be copied into the code.

Address convention: NE segment N = Ghidra selector `0x1000 + (N−1)·8`. Example: segment 7 = `1030`.

## NE header

Linker 5.30, Windows 3.0 or later, flags `0x030a`, entry point `4:0061` (`__astart`), DGROUP = segment 10. Imports from `KERNEL`, `GDI`, `USER`, `SOUND`, `KEYBOARD` and `WIN87EM`, a total of 195 functions by ordinal.

Resources inside the EXE: only cursors (`HELPCURSOR`, `MAGCURSOR`, `ROCKCURSOR`, `DIGCURSOR`, `ANTCURSOR`, `FOODCURSOR`, `DROPCURSOR`, `SPRAYCURSOR`), icons (`SIMANT`, `ANT0`–`ANT7`, `REDWIN`, `BLACKWIN`), the `OPEN`/`SAVEAS` dialogs and an accelerator table. Everything else comes from the `.DAT` bank.

## Segments and modules (names from the `.SYM`)

| Seg | Selector | Name in SYM | Type | Contents, judging by function names |
|---|---|---|---|---|
| 1 | 1000 | `SIMANT_MODULE` | code | `WINMAIN`, `MAINWNDPROC`, `MYTIMERFUNC`, menus, pause, score (`_CalcScore`), scenarios, dialogs, history, caste/mode control (the triangle), yellow ant, new game |
| 2 | 1008 | `GR_MODULE` | code | video (`_DetectDisplay`, `_InitGraphics`, `_ReadConfig`), `G*` primitives, bitmaps, fonts, memory management (`_mem_*`, `_Ralloc*`), animation (`_hanim_*`), sound and MIDI |
| 3 | 1010 | `ANTEDIT_MODULE` | code | edit/zoom window, drawing of the spider and balloons, tools, yard (`_DrawYard`, cat, dog, boy, lawnmower, rain), map, minimap, load/save |
| 4 | 1018 | `_TEXT` | code | Microsoft C runtime, tiles/scaling in assembly, information window, NetBIOS/DDE (network mode) |
| 5 | 1020 | `SIMONE_MODULE` | code | initialization, random generators (`_SRand*`, `_RRand`), water, food, eggs, tunnel digging, ant lists, map generation (house, yard, rocks), spider |
| 6 | 1028 | `SIMANT1_MODULE` | code | **core of the ant simulation**: `_DoAntSim`, behavior per colony (`A` surface, `B` black, `R` red, `Y` yellow), pheromones, alarm, fights, queen |
| 7 | 1030 | `SIMTWO_MODULE` | code | colony strategy, yard (boy, bird, cat, dog), yard colonization, antlion, caterpillar, world/tutorial generation, database (`_db_*`, `_DB*`, `_ch_*` cache, index), LZSS, fonts, custom window system (`_win_*`) |
| 8 | 1038 | `SIMANT_DATA_GROUP` | data | tunnel maps, ant lists, pheromones, save table (`0x9570`) |
| 9 | 1040 | `PACK` | data | counters, histories, yard state |
| 10 | 1048 | `DGROUP` | data | global variables, strings, stack |

## Main flow (seen in the code, at a high level)

- `_ReadConfig` (`1008:08f4`) → `_InitGraphics` (`1008:010c`) → `_db_SetDataBase` (opens `SHARED` and the graphics bank).
- The simulation advances inside `MYTIMERFUNC` (`1000:2440`), a `SetTimer` callback, which calls `_DoAntSim` and then `_UpdateWindows` and `_UpdateYardMessage`. `SetTimer` requests an interval of 0x11 = 17 ms. On Windows 3.1 the timer has a 55 ms resolution, so in practice it fires every 55 ms (measured on the test bench: 6.0 ticks/s at normal and 3.0 at slow, simulacao-visao-geral.md §3.1). Correction of an earlier version of this document: `1048:0042` is **not** the interval, it is the segment of the callback function pointer (`MakeProcInstance` in `WINMAIN`).
- Initialization sequence: see [telas/abertura.md](telas/abertura.md).
- The game windows use a custom system (`_win_LoadWindow`, `_win_Events`, …) on top of a single Windows window. The definitions come from the type 0 and 1 objects in the bank.

## Data interface

- Object bank: see [formatos/banco-dat-ndx.md](formatos/banco-dat-ndx.md). `_db_LoadObject(id, type, flag)` at `1030:8292` looks first in the cache (`_ch_LookUpId`) and then in each open bank, in order (`_DBRecall`).
- Configuration: [formatos/simant-cfg.md](formatos/simant-cfg.md).
- Saved game: [formatos/save-ant.md](formatos/save-ant.md).
