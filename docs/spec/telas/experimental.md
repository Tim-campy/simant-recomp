*Leia em português: [experimental.md](../../pt-BR/spec/telas/experimental.md)*

# Experimental mode: menu and editing tools

This document describes how the original chooses and applies the Experimental tools (adding and removing ants, food, rocks, walls, scent, insecticide, digging and filling), how a Quick Game becomes Experimental and how Experimental becomes a game ("Start Game"). World generation for Experimental is in [../regras/geracao-mundo.md](../regras/geracao-mundo.md) (§2, table by game type).

Implementation: `engine/src/rules/experimental.rs` (rules, no window), data in `data/experimental.json`, tests in `engine/tests/regras_experimental.rs`. The interface (menus, cursor, examine window) belongs to the executable.

**Sources and legend:** Ghidra pseudocode in `re_trabalho/decomp/Code1.c`, `Code3.c`, `Code5.c`, `Code7.c` (reference only; names from `SIMANTW.SYM`); disassembly with capstone where Ghidra marked blocks as unreachable; bytes of `SIMANTW.EXE`; object names from the `SHARED` bank (type 1). **[code]** = seen in the pseudocode/disassembly, with the address; **[original]** = verified on the QEMU bench; **[hypothesis]**; **[open]**.

## 1. Variables

| Symbol | Address | In the save | In the port |
|---|---|---|---|
| `_CurGameType` | `9:80b4` | yes | `Estado::partida.tipo_jogo` (0 Tutorial, 1 Quick, 2 Full, 3 Experimental) |
| `_CurExpTool` | `9:79e2` | yes | `Estado::partida.ferramenta_experimental` (0–7, table in section 3) |
| `_ExpSubStates[8]` | `8:8600` (bytes) | no | `ControleExperimental::subopcoes`; initial values from the executable `FF 00 00 00 00 00 00 FF` (only 1–6 are used) |
| `_ExpLastPnt` | `9:7d46` (x, y) | no | `ControleExperimental::ultimo_ponto` |
| `_MeMode` | `9:9fe8` | yes | `Estado::amarela.modo` (0 ant, 1 spider, 2 Experimental: the yellow ant does not exist on the map) |

[code] `_NewGame` clears `_CurExpTool` when starting an Experimental game (`1000:d6dc`) and so does `_InitSimVars` (`1030:5a70`).

## 2. How the tool is chosen

There is no Edit menu and no accelerators: the game's menu bar is File, Window, View, Options, Speed [original: bench captures], and no entry of the `ACCELERATORTABLE` (offset 500224 of `SIMANTW.EXE`) leads to `_SetExpTool` or to the tool menus [code + bytes]. The full table: Ctrl+N/L/S/A/Q (File: `0xFD03`–`0xFD08`), Ctrl+E/M/B/C/H/I/T (open windows, `0xFD11`–`0xFD17`: edit, map/yard, modes, castes, history, information, scoreboard), Ctrl+Y/5/6/7 (`0xFD21`–`0xFD24`, yard mode), Ctrl+8/9/0 (planes), Ctrl+G/J/K (`0xFD31`–`0xFD33`, Options), Shift+0..4 and Shift+NumPad (Speed), and `0xFDA0`–`0xFDAC` (Ctrl+S/M, Ctrl+Space, Ctrl+U/K, Ctrl+P, Ctrl+A, Ctrl+Z, Ctrl+arrows, Ctrl+U/V/F: window manipulation and debugging in `MAINWNDPROC`).

The tool menu is a "proximity" menu (`_win_DoProxMenu`), opened by three buttons [code]:

| Button | Object | Routine |
|---|---|---|
| edit window tools | `0x0007` `editTools` | `_EditToolsMenu` (`1010:67c6`), via `_ProcEditEvent` |
| map window tools | `0x0117` `mapTool` | `_MapToolsMenu` (`1010:dc1e`), via `_ProcMapEvent` |
| ribbon tools | `0x220A` `ribTools` | `_RibbonToolsMenu` (`1010:de76`), via `_ProcMapRibbonEvent` |

What the button does depends on `_CurGameType` [code]:

- **Quick Game (1):** opens window 10 (`editToExp`, a picture and the item `editToExpMode`). Once the item is chosen, the game becomes Experimental (section 5).
- **Full Game (2):** opens window 11 (`editNoExpMenu`, just a picture) and nothing changes.

**Press-and-hold menus of windows 10 and 11 (2026-10-19).** Both open via `_win_DoProxMenu(janela, −2, x, 0)` with x = (button right − window width + left) / 2, the same computation as window 9 [code, `1010:de76`]; with −2 there is no repositioning nor highlighted item when opening. `_win_DoProxMenu` (`1030:de62`) returns the object under the mouse minus 2 when the button is released on an object ≥ 2 of the window; the picture (object 1) and the background give −1 [code]. The Quick Game becomes Experimental with any value ≥ 0 (only the item `editToExpMode` exists) and plays sound 2; the Full Game ignores the value. **Bench (`qemu_ui_menu_quick_full.txt`, snapshots `quick` and `full`, q01 and f01):** the three windows (9, 10, 11) have the same style bits (0x41/0x18) and the same frame (black outline and 4 blue lines, outline on line 73 of the client area); the area inside the frame starts at x = 312 (window 9, i05), 296 (10) and 304 (11): it is the x from the computation placed **at the nearest byte** (8 pixels on 16-color VGA), like the client area of the game's other windows ([resolucao.md](resolucao.md) §7). The "+1" measured earlier in window 9 was this alignment. Releasing on the button (outside the item) closes without changing the game type (q03: `_CurGameType` = 1); releasing on the Full Game picture too (f02: 2). **Port:** `MenuFerramentas::abrir_por_tipo` (`QualMenu::Quick`/`Completo`, `data/menu_ferramentas.json` → `janela_quick`, `janela_completo`), position by the driver profile; releasing on the item calls `quick_para_experimental`; test `botao_de_ferramentas_no_quick_e_no_completo`; 100% of the frames of q01 and f01 and of window 9 of i05 (`comparar_captura.py --mapear`). The highlight of the window 10 item is the `_GRectInv` inversion [code], no capture.
- **Tutorial (0):** nothing.
- **Experimental (3):** `_DoExpMenu` (`1010:6ee2`) opens window 9 (`editToolsMenu`) with the item of the current tool highlighted (`__win_SetProxItem(0x902 + _CurExpTool)`), pauses the game while the menu is open (`_SetPause(1)`, and afterwards restores the previous value) and returns the chosen tool (−1 = nothing). If the item has a submenu and something was chosen in it, it writes `_ExpSubStates[ferramenta] = submenu item − 2`. Then: tool 7 → "Start Game" (section 6); in every case `_CurExpTool = ferramenta` (including 7) and the button drawing changes to the tool's bitmap.

| `_CurExpTool` | Item (window 9) | Submenu (window) | `_ExpSubStates` | Routine |
|---|---|---|---|---|
| 0 | `editExpInfo` | — | — | `_MagnifyMenu` |
| 1 | `editExpBarrier` | `wallRockMenu` (12) | 0 `toolWalls`, 1 `toolRocks` | `_DropWall` |
| 2 | `editExpDig` | `digFillMenu` (15) | 0 `toolDig`, 1 `toolFill` | `_ExpDig` |
| 3 | `editExpAddAnt` | `blackRedMenu` (16) | 0 `toolBlack`, 1 `toolRed` | `_ExpAddAnt` |
| 4 | `editExpAddFood` | `foodMenu` (17) | 0 `onePiece`, 1 `twentyPieces` | `_IncFoodHere` |
| 5 | `editExpTrails` | `placeScentMenu` (13) | 0 black nest, 1 black trail, 2 red nest, 3 red trail, 4 alarm | `_ExpIncSmell` |
| 6 | `editExpInsecticide` | `cideMenu` (14) | 0 `toolSpray`, 1 `toolStream` | `_ExpKillAnts` |
| 7 | `editExpStartGame` | — | — | section 6 |

The item → submenu window link goes through the object names; the code only sees the item index [hypothesis regarding the internal mechanism of `_win_DoProxMenu`, not studied]. The mouse cursor over the edit window (or over the map with Shift) changes according to the tool: `_magCursor`, `_rockCursor`, `_digCursor`, `_antCursor`, `_foodCursor`, `_dropCursor`, `_sprayCursor` (`MAINWNDPROC`, `WM_SETCURSOR`) [code].

## 3. Click and drag: `_processExp` (`1010:7022`)

In Experimental, every left click in the map area of the edit window (single or double) goes to `_processExp(x, y)` (the right click turns on help mode: [ajuda-contexto.md](ajuda-contexto.md) §1) instead of becoming an order for the yellow ant (`_processEdit`, `1000:6a38`) [code]. It also applies in the small map window, with a different coordinate conversion.

1. `_ExpLastPnt = (x, y)`; if `_IsValidLocation(_MapPlane, x, y)`, `_DoTool(x, y)`.
2. While the button stays pressed (`_myButton`), on each iteration: reads the cursor; if it changed cell, `_ExpLastPnt` = previous cell and `_DoTool(nova)` (if valid); and, **with `_CurExpTool` > 4** (scent and insecticide), `_DoTool` again on the cursor's cell on every iteration, even when stationary.
3. Each iteration redraws the edit window and/or the map; the pace of the iterations is that of the redraw (real time), not the tick.

`_IsValidLocation(plano, x, y)` (`1020:56da`): x in 0..127 on planes 0 and 1, in 0..63 in the nests; y in 0..63 [code].

`_DoTool(x, y)` (`1010:727e`) dispatches on `_CurExpTool` [code]. Holding Ctrl (`GetAsyncKeyState(VK_CONTROL)`, read inside each routine) inverts wall, dig, food and scent.

## 4. The tools

### 4.0 Info (`_MagnifyMenu`, `1010:6b4c`)

`_DoTool` always calls `_MagnifyMenu`, because `_GetLife` returns 0xFFFF for an empty cell; it is `_MagnifyMenu` that only opens the box if the life byte ≠ 0 [code]. The box (window 29 `examineWin`, `_win_DrawExamineWindow` `1010:69c0`) stays open while the button is pressed and shows three texts [code]:

- `examStr1`: group `g = (vida & 0x78) >> 3`; with g = 0 (egg), the stage `vida & 7` in SHARED list 4/1100 ("Weird Egg", "New Egg", "Egg", "1st-Instar"…); otherwise list 4/1101 ("Egg Tender", "Worker", …, "Queen").
- `examStr2`: the mode M of the ant found in the plane's list (`_FindInAList`, `_FindInBList/RList` with the T) in list 4/1102 ("Bored", "Nesting", "Foraging"…); 0 if not found.
- `examStr3`: list 4/1103 by group ("Nothing", "Egg", "Food", "Dirt"…).

Does not change the state. In the port: `examinar(e, plano, x, y) -> Option<Exame>`.

**Bench (2026-10-18, `qemu_ui_info_exp.txt` and `qemu_ui_info_exp2.txt`, snapshot `exp` paused):** black ant placed in cell (5, 5) with tool 3; with Info, holding the button on it opens Examine ("Worker / Foraging / Nothing") and releasing closes it (i01, i02); holding on an empty cell opens nothing (i04). The box has its outline at the corner computed by `_MagnifyMenu`, except on the **first** opening of the session, 3 pixels to the right (j01 versus j02 and j03, same cell; also seen in examine_0) [cause unknown; the port always uses the computed corner]. The port does the same (test `info_do_experimental_abre_o_exame`). **Only ants, eggs, larvae and pupae:** Info examines the life map; food, rock, wall and empty cells have no life byte and open nothing, and the spider and antlions are not on the life map (they have their own variables), so they don't either — in the original and in the port (answered to the user on 2026-10-19). The report "Info doesn't work" probably came from the tool menu: in the original it is press-and-hold (opens on press, chooses on release over an item, i05/i06), and the port only chose on a second click — whoever dragged and released on "ANT INFO", as in the original, kept the previous tool. Fixed (see "What the port does").

### 4.1 Barrier (`_DropWall`, `1010:7450`, and `_ConnectAll`, `1010:805a`)

Only on planes 0 and 1. Walks from `_ExpLastPnt` to (x, y): at each step, `k = _GetDir(ponto, destino)` and the point moves **only in x** if `_Dx9[k]` ≠ 0, otherwise in y ("L"-shaped path); stops after handling the destination. Stops immediately if the cell leaves the map (`_IsValidA`). In each cell [code]:

- **Ctrl:** if 0x50 < tile < 0x68 (pebble, rock, wall), tile = `SRand1(16)`; then `_ConnectAll`. No sound.
- **Walls (sub-option 0):** if tile < 0x50 and `_LifeA` = 0, tile = 0x60; then `_ConnectAll` (always); sound 0x28.
- **Pebbles (sub-option 1):** if tile < 0x50 and `_LifeA` = 0, tile = 0x51 + `SRand1(3)`; then **always** `SRand1(10000)` for the parameter of sound 10 (the draw counts in the S generator).

`_ConnectAll(x, y)`: for the cell and its top, right, bottom and left neighbors (neighbors only within the map), if the tile is a wall (0x60..0x67), writes `conexao_muro[m]`, with `m` = left × 8 + bottom × 4 + right × 2 + top (1 if the neighbor is a wall; outside the map it is not). Table (`DGROUP:0x1bce`): `60 64 65 66 62 61 62 61 63 63 60 60 67 67 67 67` [executable bytes].

### 4.2 Dig and fill (`_ExpDig`, `1010:756e`)

Fill = sub-option 1, inverted by Ctrl. Walks from `_ExpLastPnt` to (x, y) moving on both axes per step (`_Dx9/_Dy9`, diagonal) [code]:

- **Surface, dig:** `_DigMyNewHole(x, y)` (new hole with a free 3 × 3; see [formiga-amarela.md](../regras/formiga-amarela.md)). Succeeded: sound 0x13. Did not: error sound 1 and warning 22 of list 4/1010 ("Not enough room here for a hole.").
- **Surface, fill:** error sound and warning 21 ("Fill tool unavailable on the surface.").
- **Nest, dig:** if `_IsItDigable` (dirt or grass) and y ≠ 0: `_DigTileB/R(x, y)`; if y = 1, also `_MakeNewHoleB/R(x)` (hole on the surface for column x). Sound 0x13.
- **Nest, fill:** if **not** `_IsItDigable` and y ≠ 0: the ants of the nest list in that cell (all slots in use with T ≠ 0) get T = 0, without a corpse; tile = 0x2E (solid dirt) and life = 0; if `_TilesDug?` > 1, `_TileTotX?/Y?` −= x/y (32 bits; negative becomes 0) and `_TilesDug?` −= 1, **without** recomputing the center of mass; `_SmoothEdges?` on the top, right, bottom and left neighbors; `_ExitMap?` of the cell = 0. Sound 0x12. Any tile that is not dirt/grass gets filled (tunnel, food, passage).

### 4.3 Add ant (`_ExpAddAnt`, `1010:79fc`)

[code] `T = SRand1(8) + 0x10`, plus 0x80 if the sub-option is 1 (red); the draw always happens. Surface: only if `_IsClearTile(1, x, y)`; `_AddAntToAList(x, y, T, M = 2, S = 0)`. Nest: never on row 0; if the tile > 0x1B, `_DigTileB/R(x, y)`; M = 2 in the nest of its own color and 7 in the other's; `_AddAntToBList/RList`. The ant goes into slot `_ListIndex?` (full list: discarded) and writes the life. Sound 0x1C (except when it exits early: occupied cell or row 0).

### 4.4 Food (`_IncFoodHere`, `1010:7f3a`)

One piece (sub-option 0): `_IncFoodHere(x, y)` and, if it wrote, sound 0x1D. Twenty pieces (1): sound 0x20 and 20 times `(x + SRand1(9) − 4, y + SRand1(9) − 4)` (x drawn before y), `_IncFoodHere` on each valid cell [code].

`_IncFoodHere`, with threshold L = 0x48 on the surface (planes 0 and 1) and 0x10 in the nests [code]:

- Without Ctrl, if the tile ≤ L + 2: tile ≥ L → +1; tile < 0x18 → L; any other tile (object between 0x18 and L − 1) stays as is, **but counts**. Adds 1 to `_FoodA` (surface), `_FoodB` (plane 2) or `_FoodR` (plane 3). Writes and returns 1.
- With Ctrl, if L ≤ tile ≤ L + 3: tile = `SRand1(8)`; only on the surface it subtracts from `_FoodA` (with `_FoodA` < 1, it subtracts from `_FoodR` if > 0: the `_MapPlane == 2` test that would pick `_FoodB` never holds there); in the nests it subtracts nothing. Writes and returns 1.
- Otherwise returns 0 without changing anything.

### 4.5 Scent (`_ExpIncSmell`, `1010:7b9e`)

Only on planes 0 and 1. Cell (x ≫ 1, y ≫ 1) of the scent map of the sub-option (`_PherMapBN`, `BT`, `RN`, `RT`, `A`): +70, or −40 with Ctrl, clamped to 0..255; then `_SmoothMany(x ≫ 1, y ≫ 1)` on the same map and sound 0x1B [code].

`_SmoothMany` (`1010:889c`) calls `_SmoothACell` on the cell, above, to the right, below, to the left and again on the cell. `_SmoothACell` (`1010:88f4`): sums the four neighbors (outside the map counts as −1), divides by 4 (truncating toward zero), adds the cell's value and divides by 2; clamps to 255 and writes if the cell exists [code].

With the button held still, the scent is applied again on every loop iteration (section 3).

### 4.6 Insecticide (`_ExpKillAnts`, `1010:7d5c`)

Eight squirts [code; Ghidra marked a block as unreachable and the disassembly (`1010:7d62`–`7d8a`) settled it: `cmp byte [8606],1 / sbb / and 7 / inc` gives 8 with sub-option 0 and 1 with sub-option 1]:

- **Stream (`toolStream`, 1):** all 8 on the cell itself, no random draw.
- **Spray (`toolSpray`, 0):** each one at `(x + SRand1(9) − 4, y + SRand1(9) − 4)` plus one `SRand1(1000)` (+0x278F) for the sound.

Each squirt plays sound 9 and, if the cell is valid: on the surface, if `_LifeA` ≠ 0, `_Tindex = _FindInAList(x, y)` and, if found, `_DeadAntHere(x, y, cor)` (corpse) and T = 0; if `(_SpidX ≫ 4, _SpidY ≫ 4)` = (x, y), `_KillSpider`; if 0x37 < tile < 0x3F, `_KillAntLion(_FindInLionList(x, y))`. In the nest, if life ≠ 0, `_Tindex = _FindInBList/RList(x, y, vida)` and, if found, T = 0 and life = 0 (no corpse). The yellow ant (0xFF) is not in the lists and is not hit.

## 5. Quick Game → Experimental

Item `editToExpMode` of window 10 [code, `_EditToolsMenu` and `_MapToolsMenu`/`_RibbonToolsMenu`, which first do `_SetExpTool(_CurExpTool)`, with no effect]:

1. `_CurGameType = 3`;
2. `_ClearMyLife(_MePlane, _MeLoc, _MeType, _MeDir)`: erases the 0xFF mark (and the queen's 0xFE tail) from the life layer;
3. `_InitYelloAnt` (section 7): in Experimental, the yellow ant goes to plane 1, (0x40, 0x20), worker, `_MeMode = 2`, **without** a mark on the map;
4. `_MeMode = 2` again; `_InitSpider` (section 7); window title; sound 2.

The world (ants, nests, food) stays the same. In the port: `quick_para_experimental`.

## 6. "Start Game" (tool 7)

[code] `_CurGameType = 1` (Quick Game in the edited world), `_InitYelloAnt` (worker in the black nest, at `(_RebornX, _RebornY)`, with the 0xFF mark), and, if the map shows the surface (`_MapPlane` = 1), `_SetMyLife` again with the yellow ant's current data; title; music 0x2AFB; `_CurExpTool = 7`. The spider and everything else stay as they are. In the port: `escolher_ferramenta(…, ComecarJogo, …)` / `comecar_jogo`.

## 7. Support routines

- `_InitYelloAnt` (`1030:6492`): `_MeBase = 0`, `_MeEggCarried = 0xFD`; if `_MeDropAlarm` ≠ 0, clears it and sets `_ELayerMode = −1` (interface); `_SetMyHealth(100)`; Experimental: plane 1, (0x40, 0x20), `_MeType` 0x10, `_MeMode` 2 (direction does not change); otherwise `_MeMode` 0 and `_SetMyLife` with direction 2 and 0xFF: in the Full Game without a nest, winged queen (0x40) on the first cell < 0x10 out of up to 100 draws `(0x20 + Sk(16) − Sk(16), 0x20 + Sk(8) − Sk(8))` (no spot: (0x40, 0x20)); in the others, worker in the black nest at `_RebornX/Y`. Finally `_ResetYellowVars(_MePlane, _MeLoc)`. The same as [geracao-mundo.md](../regras/geracao-mundo.md) §3.D, here on the in-game state.
- `_InitSpider` (`1020:4668`): `_SpidBurpCnt = 10`; clears `_EatCnt`, `_SCorpseBase`, `_Scycle`, `_Scycle2`, `_SpidRevenge`, `_SpidDir`, `_RevSpider`, `_SMode`, `_MeSMode`; `_SpidOn = (_CurGameType ≠ 0)`; `(_SpidX, _SpidY) = (0x400, 0x200)`; `_Starg = −2`, `_StargLife = −1`; `(_SuserX, _SuserY) = (0x40, 0x40)`.

## 8. What changes in the `Estado`

| Tool | Fields |
|---|---|
| Barrier | `superficie.mapa`; `s` (pebble and erase) |
| Dig | surface: `superficie.mapa`, `preta`/`vermelha` (`buracos`, `ultimo_buraco`, `ultima_saida`, excavation totals), `ninho_?.mapa/saida`; nest: `ninho_?.mapa/saida/vida/lista`, excavation totals; `s` (dig) |
| Ant | `superficie.lista/vida` or `ninho_?.lista/vida` (and `mapa`, if it dug); `s` |
| Food | `superficie.mapa`/`ninho_?.mapa`, `ambiente.comida_superficie`, `preta.comida`, `vermelha.comida`; `s` (Ctrl and twenty pieces) |
| Scent | `feromonios.ninho_b/trilha_b/ninho_r/trilha_r/alarme` |
| Insecticide | `superficie.lista/vida/mapa`, `partida.cadaver_ponteiro/cadaveres_x/y`, `aranha`, `formigas_leao`, `ninho_?.lista/vida`, `tindex`; `s` |
| Mode changes | `partida.tipo_jogo`, `partida.ferramenta_experimental`, `amarela`, `aranha`, yellow ant's life |

## 8.1 In the port's interface

`simant/src/modos.rs` and `engine/src/core/menu_ferramentas.rs` (data in `data/menu_ferramentas.json`):

- In Experimental, the ribbon's `ribTools` button shows bitmap 14500 (`_NewGame` swaps the pair to 14550/14500) and, when **pressed**, opens the window 9 menu as in the original (i05, 2026-10-18): centered on the button by the `_RibbonToolsMenu` computation (x = (right − width + left) / 2, outline 1 pixel after), outline on line 74 of the client area, 1 black line and 4 blue ones, the item of the current tool **inverted** (`__win_SetProxItem`) — 100% of the pixels identical to the capture. With the button pressed, the highlight follows the mouse and an item with a submenu opens the submenu (windows 12–17) beside it; **releasing** on an item chooses it (in the submenu, writes `_ExpSubStates`); releasing outside, or on the button itself, closes without choosing (i06). The simulation stops while the menu is open (`_SetPause(1)`). Not done: the frame of the submenus (no capture) and the empty `_EditMessage` the original does on closing (the "Game Paused" message disappears from the ribbon in i06).
- In the Quick Game the same button switches the game to Experimental (`quick_para_experimental`) without the one-item menu (window 10); in the Full Game and in the Tutorial it does nothing (divergencias.md).
- In Experimental, every left click on the map of the edit window goes to `experimental::pressionar` (since 2026-10-18 the right click turns on help mode, as in the original); moving the mouse with the button pressed calls `arrastar` on every motion event (the original repeats at the pace of the redraw); releasing calls `soltar`. Ctrl is read from the keyboard state at that moment. The tools' sounds and music go to the event queue (played by the port's sound); the warnings go to the ribbon as in the original (`_EditMessage(texto, 0xB4, 0, 1)`, important; since 2026-10-18); the Info examine box (window 29) is drawn (section 4.0). The cursor changes with the tool over the client area of the edit window ([ajuda-contexto.md](ajuda-contexto.md) §5). The `MAGCURSOR` (Info) is the only one of the executable's cursors with inverting pixels (AND 1 and XOR 1: the magnifying glass lens); the SDL cursor does not invert and the port painted them black ("magnifying glass with a black lens", report from 2026-10-19). Since then the port hides the system cursor over the edit window with Info and draws the magnifying glass in the frame, inverting the index (i ^ 15, which in the default palette is the RGB complement, except for the grays 7 ↔ 8) (`ajuda_ui::desenhar_cursor`, test `cursor_com_lente_invertida`). The other cursors (help, rock, shovel, ant, food, drop, spray) only have transparent, black and white and remain system cursors.

## 9. Verification

Port tests (`engine/tests/regras_experimental.rs`), from the pseudocode:

| Case | Input | Expected |
|---|---|---|
| E-U1 | `QUICK2.ANT`, Quick → Experimental | 0xFF mark erased at (34, 9) of the black nest; yellow ant on plane 1 at (64, 32), `_MeMode` 2, health 100, `_MeEggCarried` 0xFD, no mark; spider reset |
| E-U2 | E-U1 + "Start Game" | `_CurGameType` 1, yellow worker ant at `_RebornX/Y` with 0xFF, `_CurExpTool` 7, music 0x2AFB; the menu outside Experimental does nothing |
| E-U3 | Full Game | the button changes nothing |
| E-U4/5 | food on the surface and in the nest | 0x48, 0x49, 0x4A, 0x4B and refusal; object 0x20 counts without changing; Ctrl; in the nest Ctrl does not subtract `_FoodB` |
| E-U6 | twenty pieces | same cells and same S state as the computation in section 4.4 |
| E-U7 | ant | T = 0x10 + S(8), M 2; occupied cell refused (S drawn); red ant in the black nest M 7; row 0 refused |
| E-U8 | insecticide stream on `QUICK2.ANT` | corpse of the color, T = 0, eight sounds |
| E-U9 | alarm scent | (5, 5) = 19 and the four neighbors = 4 |
| E-U10 | fill and dig in the black nest of `QUICK2.ANT` | 0x2E, exit 0, totals −1/−x/−y, center unchanged; Ctrl digs back; warning 21 on the surface |
| E-U11 | pebble and Ctrl | 0x51 + S(3) and sound with S(10000) + 2000; erase = S(16); wall in the nest: nothing |
| E-U12 | drag | food does not repeat when stationary; scent repeats |

**Verified against the original (QEMU bench, 2026-10-17):** scripts `tests/referencias/roteiros/qemu_experimental_e01.txt` and `qemu_experimental_e02.txt` (with a separate copy of the overlay: `--overlay re_trabalho/qemu/ov_exp.qcow2`; outputs in `re_trabalho/exp/`). New Experimental, game paused (Shift+0), `_CurExpTool` and `_ExpSubStates` written into memory via the gdbstub (`escrever_variavel CurExpTool N`, `escrever_variavel 8:860N V 1`), clicks with `clicar_em` in the edit window, `estado` and `SementeS` before and after each click. The states are in `tests/referencias/saves/E01_*.ANT` and `E02_*.ANT`, and the tests `e_01_…` and `e_02_…` start from them and reach the original's save **byte for byte**, including the state of the S generator:

- **E-01** (S 0x7CFE → 0xCBEF): food at (57, 28), which was rock 0x57 (nothing changes, no random draw); black ant at (59, 28) (T = 0x14, M = 2, one draw); wall at (61, 28), tile 0x28 → 0x60 (no draw); pebble at (62, 28), which was rock 0x59 (only the sound's `SRand1(10000)`); insecticide stream at (59, 28): corpse 0x13, T = 0, `DdPtr` 0 → 1 (one draw, the corpse's).
- **E-02** (a) twenty pieces at (64, 31) and spray at (55, 26): 15 cells, `_FoodA` 0 → 19, S 0xC14E → 0x0E20; (b) in the black nest (Ctrl+9, view at (20, 0)): dig (28, 4) (`_TileTotXB` 355 → 383, `_TilesDugB` 10 → 11, center 35 → 34), fill (28, 4) (back to 355/10, center **not** recomputed: stays 34) and fill (22, 6) (dirt: nothing), S 0x0E20 → 0x1C40.

Bench observation: the view of the new Experimental started at `_MapPnt` = (52, 23), not at (51, 23) as `_CenterEdit(0x40, 0x20)` with 26 columns would give [open: width of the view at the moment of `_NewGame`'s `_CenterEdit`].

**Verified against the original (bench, 2026-10-17, `tools/cenarios_tick.py quick_exp|exp_start|exp_ctrl`; vectors in `tests/referencias/acoes/`, `engine/tests/acoes_originais.rs`; game paused, state before and after, 307 blocks and S and R):**

- **Quick → Experimental** (`quick_exp`): snapshot `quick`, ribbon tools button at (354, 71) (`ribTools`, with the tool bitmap) and the "EXPERIMENTAL MODE" item of window 10 at (360, 127): `quick_para_experimental` gives 307/307.
- **"Start Game"** (`exp_start`): snapshot `exp`, same button (opens window 9 below it: ANT INFO … START GAME, the latter at (360, 246)): `escolher_ferramenta(ComecarJogo)` gives 307/307.
- **Ctrl and drag** (`exp_ctrl`, tool and sub-option written into memory, Ctrl pressed via QMP): food at (55, 35) and Ctrl+food on the same cell (the food goes away and the ground is drawn again), wall at (57, 37) and Ctrl+wall (erases, ground drawn), wall drag from (58, 39) to (63, 39) (connection tiles 0x65, 0x60 … 0x63) and food drag from (60, 38) to (64, 38) (five 0x48, `_FoodA` +5): `pressionar`/`arrastar`/`soltar` give 307/307 with the same S.

The view of the new Experimental at `_MapPnt` = (52, 23) (section above) checks out with `_editWidth` = 25 on the bench (0x40 − 12 = 52), the same as the centering of the yellow ant in the tick vectors (testes-tick.md).

- **Ctrl on dig/fill** (`exp_ninho`, black nest via the Ctrl+9 view): dig (28, 5), Ctrl+dig on the same cell (fills), Ctrl+fill at (29, 5) (digs) and dig drag from (24, 7) to (27, 7): 307/307 with the same S.
- **Scent** (`exp_cheiro`, surface): alarm with a short click at (60, 30), with the button held still ~2 s at (66, 30) and Ctrl+alarm at (60, 30): 307/307 with **6 stationary iterations** in the second one (the vector's `parado` step; ~3 iterations per second on the bench, the pace of the redraw).

**Not verified against the original:** ant in a nest, insecticide on the spider and on an antlion in the list. Insecticide on an antlion tile **without** an entry in the list (`_KillAntLion(-1)`) was checked on 2026-10-19 (action `acoes/exp_leao_m1`, regras/inimigos.md §1.4).

## 10. Gaps and divergences

- `_KillAntLion(_FindInLionList(x, y))` with an antlion tile without an entry in the list receives −1 and writes outside the table in the original: what happens and what the port reproduces is in [../regras/inimigos.md](../regras/inimigos.md) §1.4 (since 2026-10-19 the port removes antlion 0 from the list and writes the bytes of `_BoyMsgCnt` and `_LastThemeSong`, like the original; the `_SetMap` with x coming from a memory selector it does not).
- The number of drag iterations per second (scent and insecticide held still) depends on the original's redraw; the port leaves the pace to the interface.
- The texts of the examine box (lists 4/1100–1103) and of the warnings (4/1010) come from the bank; the port draws both (examine: §4.0; warnings in the ribbon). No capture of the warnings in the ribbon.
- `_ELayerMode`, cursors, button bitmaps and window title are interface.
