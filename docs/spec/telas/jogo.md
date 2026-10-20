*Leia em português: [jogo.md](../../pt-BR/spec/telas/jogo.md)*

# Game screen (ribbon + edit window)

This document describes the screen shown during a game: the tool ribbon (window 34, `mapRibbon`) and the **edit window** (window 0, `editWin`), which shows the map of the surface or of a nest with the ants on top. The other windows open during a game (map, caste control and mode control) are left for later.

Implementation: `engine/src/core/vida.rs` (life layer), `engine/src/core/edicao.rs` (edit window view), `engine/src/core/tela_jogo.rs` (`compor_tela_de_jogo`, Windows frame), `engine/src/core/navegacao.rs` (scrolling and plane switching), data in `data/tela_jogo.json` and `data/navegacao.json`. Executable: `simant --save <save .ANT>` (window) or `--exportar-tela-jogo <png>`. Comparison example: `cargo run -p engine --example tela_jogo -- <game folder> <save .ANT> <output folder> [captura.png]`.

**Sources:** Ghidra pseudocode in `re_trabalho/decomp/` (reference only; names from `SIMANTW.SYM`), `QUICK2.ANT` and the capture `re_trabalho/qi_14_salvo.png`, taken by the QEMU bench ([bancada-qemu.md](../../bancada-qemu.md)) in the same session in which the save was written. Legend: **[code]** = seen in the pseudocode, with the address; **[save]** = seen in the data; **[measured]** = measured pixel by pixel in the capture; **[hypothesis]** = not confirmed.

## Windows open in a Quick Game

`_NewGame` (`1000:d6dc`), after the choice in "Select a Game" (the Quick Game button, `scenQuickGame`, returns 0x202, which becomes game type 1; see [abertura.md](abertura.md), "Game choice") [code]:

1. opens the edit window (`_win_Open(0)`), when the type is 0;
2. `_OpenCasteWindow` and `_OpenModeWindow` (caste and mode control windows);
3. `_SetMapPlane(_MapPlane)`; with `_MapPlane = 0` it opens window 25 (`_MapToYard`), otherwise window 1, `mapWin` (`_YardToMap`);
4. `_SetMapTitle`, `_OpenEditWindow` (`_win_Open(0)`, `1010:0e2c`), `_SetMapPlane(_MePlane)`;
5. on type 0, if the edit window is not maximized, `_win_Zoom(0)`, which is `ShowWindow(SW_SHOWMAXIMIZED)` (`1030:c2ba`);
6. `_CenterEdit(_MeLocX, _MeLocY)` and `_UpdateEdit` with `_editForce = 1`.

When loading a saved game, `_DoLoadInitializations` (`1010:9dbe`) calls `_SetDefaultWindows` (`1000:d66a`), which repeats steps 2 to 4 without the maximization [code].

The capture `qi_14_salvo.png` (640×480) shows the ribbon, the edit window in the foreground and, behind it on the right, parts of `mapWin` and of a control window ("Soldiers 40%") [measured]. For now the port draws only the ribbon and the edit window.

## Planes and view position

| `_MapPlane` | View | Map (`_DoCalcTile`) | Width in tiles | Remembered point |
|---|---|---|---|---|
| 0 | yard | `_MapA` | 128 | `_YMapPnt` |
| 1 | surface | `_MapA` | 128 | `_AMapPnt` |
| 2 | black nest | `_MapB` | 64 | `_BMapPnt` |
| 3 | red nest | `_MapR` | 64 | `_RMapPnt` |

**Source:** `_SetMapPlane` (`1000:a5ca`) reads the plane's remembered point; `_BoundEditPosition` (`1010:15b4`) uses 0x80 on planes 0 and 1 and 0x40 on the others; `_SetMapPlaneLocation` (`1000:a386`) selects the ribbon buttons (`Yard`, `Nest`, red `Nest`, `Surface`) [code]. All maps have 64 rows.

- `_MapPnt` (`DS:7d2e`, x/y pair; y is at `DS:7d30`) is the map cell at the top-left corner of the view. It is in the save [save].
- The remembered points (`_YMapPnt`, `_AMapPnt`, `_BMapPnt`, `_RMapPnt`) are centers, not corners: `_SetMapPlane` passes the point to `_CenterEdit` [code].
- In `QUICK2.ANT`: `_MapPlane = 2`, `_MePlane = 2`, `_MapPnt = (22, 0)`, `_BMapPnt = (34, 9)` = `(_MeLocX, _MeLocY)` [save].

### Bounds (`_BoundEditPosition`)

With `L` = width of the plane's map: if `x < 0`, `x = 0`; otherwise, if `x + _editWidth > L`, `x = L − _editWidth`. The same for `y` with 64 and `_editHeight` [code; identical in `_win_EditChanged` (`1010:0c62`), `FUN_1010_16d4` and `_CenterEdit`].

### Centering (`_CenterEdit`, `1010:1620`)

1. offset `dx = x − _MapPnt.x − _editWidth / 2`, `dy = y − _MapPnt.y − _editHeight / 2` (integer division);
2. if both are 0, returns 0 (nothing to redraw);
3. otherwise, `FUN_1010_616c` walks the point like a fixed-point Bresenham line: on the major axis it takes |d| steps of ±1; on the minor axis, at each step it adds the fraction `(|d_minor| << 16) / |d_major|` (unsigned integer division) to a 32-bit accumulator and steps ±1 whenever bit 16 of the accumulator becomes set, clearing that bit. Because the fraction is truncated, the minor axis may move one step less than requested (e.g.: `dx = 3`, `dy = 1` → fraction 21845, sum 65535, no step in y) [code];
4. applies the bounds and returns 1.

### View size (`_editWidth`, `_editHeight`)

`_win_EditChanged` and `FUN_1010_16d4` ask `_win_GetObjRect` for the rectangle of object 4 of window 0 (`editTileArea`), which adds 1 to the right and bottom when there is a ribbon (`_ribbonBarHeight ≠ 0`, `1030:c2d2`). Then: `_editWidth = ⌈width / 16⌉`, `_editHeight = ⌈height / 16⌉`, and the rectangle is stretched to multiples of the tile [code].

`editTileArea` goes from (16, 42) to (416, 345) in `WINGANT`. With the ribbon, 401 × 304 pixels, which would give 26 × 19; but the original centers as if it were **25 × 19** (mensagens.md §6.1), and the port uses 25. The window clips the last column: only 25 columns (400 pixels) appear before the scroll bar [measured].

### Position on load and in the capture

- On load, `_DoLoadInitializations` writes `(_MeLocX, _MeLocY)` to the remembered point of plane `_MePlane`, and `_SetDefaultWindows` → `_SetMapPlane(_MapPlane)` centers the view on it; if `_MePlane = _MapPlane`, it centers again on `(_MeLocX, _MeLocY)` [code]. With 26 columns, the loaded `QUICK2.ANT` would start at x = 34 − 13 = 21 [hypothesis: not captured]. **Bench (2026-10-17, mensagens.md §6):** a save with the yellow ant and `_BMapPnt` at (34, 9) in the black nest (plane 2), read from memory right after Load, ended up with `_MapPnt.x` = **22**, not 21. The same 22 appears in `QUICK2.ANT` saved right after the Quick Game was created (centered on 34): `_editWidth` is **25**, not 26. The port switched to 25 columns (`ajuste_area_com_faixa` = 0) and the view after Load matches the capture at 99.94% (mensagens.md §6.1). The real rectangle of `editTileArea` after `_win_Recalc` with the maximized window still has no calculation [open].
- The capture `qi_14_salvo.png` was taken at the moment of the save, without loading. That is why the view is that of the saved `_MapPnt`, (22, 0), which is what the port uses when reproducing this screen (`posicao: "salva"` in `data/tela_jogo.json`). The fit of the capture confirms (22, 0) [measured].

## Life layer

### Maps `_LifeA/B/R`

One byte per cell, at the same index as the ground map (`x × 64 + y`): 0 = empty; 0xFF = yellow ant; 0xFE = second half of the yellow ant when it is a queen; any other value = the ant's T field ([save-ant.md](../formatos/save-ant.md), "Ant lists"). They are not saved [code].

### Reconstruction on load (`_DoLoadInitializations`, `1010:9dbe`)

1. switches the ground tileset according to `_TERRAINset` (`_OverlayTileSet(0, 1000 or 1001)`) [code];
2. clears `_LifeA`, `_LifeB` and `_LifeR` [code];
3. for each list (A → `_LifeA`, B → `_LifeB`, R → `_LifeR`), walks the indices **from `_ListIndex?` down to 0, decreasing, inclusive**: `Life[X[i] × 64 + Y[i]] = T[i]` [code]. Consequences:
   - the entry at index `_ListIndex?` (one past the last used) is also written. In `QUICK2.ANT` it is all zero and writes 0 to cell (0, 0) [save];
   - entries with `T = 0` (holes not yet compacted) write 0 and erase whatever was in the cell;
   - when two ants occupy the same cell, **the one with the lower index wins**, because it is written last;
   - X and Y are not checked. Values outside the map would write to other memory in the original [code]; the port ignores the entry and lists it as pending;
4. if `_MeMode = 0`, calls `_SetMyLife(_MePlane, _MeLocX, _MeLocY, _MeType, _MeDir, 0xFF)` [code];
5. `_FullCount` (counts), writes the remembered point and calls `_SetDefaultWindows` (see above).

### Yellow ant (`_SetMyLife`, `1020:6536`, and `_SetLife`, `1020:5d18`)

- Acts only if (x, y) is inside the plane's map (planes 0 and 1: 128 × 64; 2 and 3: 64 × 64).
- Writes the value (0xFF) to `_LifeA` (planes 0 and 1), `_LifeB` (2) or `_LifeR` (3), over whatever the lists left.
- If `_MeType = 0x60` (queen), also writes the second half (0xFE) at `(x + _Dx8[_MeDir ^ 4], y + _Dy8[_MeDir ^ 4])`: `_SetMyLife` (`1020:6536`) reads `*(char *)(dir ^ 4)` and `((char *)(dir ^ 4))[8]`, which are `_Dx8` (DS:0) and `_Dy8` (DS:8) — the same tail table as in the rules (`rules/amarela.rs`, `cauda`) [code: pseudocode, with the addresses DS:0 and DS:8 checked in the symbol table; **bench 2026-10-19**: tick vector `amarela_rainha` (0xFF and 0xFE rewritten at each step of the queen in the black nest, life 307/307) and capture `re_trabalho/raros/b_amarela_rainha/rainha_amarela.png`, equal to the port (`--save` of the state after tick 1739 with `--exportar-tela-jogo`) on 99.90% of the edit window, with all the difference in the mouse cursor]. The port's reconstruction (`core/vida.rs`) writes this half since 2026-10-18.
- In the nests, `_SetLife` with value > 0 on a dirt cell (`_IsItDirt`) or a grass cell (0x1C to 0x1F) digs the tile (`_DigMyTile`) and plays a sound [code]. This does not change the life layer and does not occur in `QUICK2.ANT` (the ant is in a tunnel); the port does not reproduce the digging on load [hypothesis: rare case].

In `QUICK2.ANT`, `_MeMode = 0` and the yellow ant is at (34, 9) of the black nest, the same cell as the black queen (T = 102) and an egg. Result in `_LifeB`: (34, 9) = 0xFF, (35, 9) = 110 (half of the queen), (36, 9) = 1 (egg) [save + code].

### Life tile of each cell (`_DoCalcTile`, `1018:4a6b`, and `FUN_1010_63fe`)

See [tiles.md](../formatos/tiles.md), "Life layer". With the offset of `FUN_1010_63fe` already accounted for, the index into the plane's life buffer (surface: objects 9/15–17; nests: 9/18–20) is:

| Life byte *v* | Index in the buffer |
|---|---|
| 0 | none (ground only) |
| 0xFF | `0x100 + _MeBase + _MeType + (_MeType < 8 ? _MeEgg : _MeDir)` |
| 0xFE | `0x108 + _MeBase + _MeType + _MeDir` |
| other | *v* |

The yellow ant uses the third part of the buffer (indices 0x100 to 0x17F). In `QUICK2.ANT`: `_MeBase = 0`, `_MeType = 16`, `_MeDir = 2`, `_MeEgg = 0`, hence tile 0x112 of the nests buffer [save].

The cell is the ground tile with, on top, the life tile with index 13 transparent [code].

## Drawing the edit window

`FUN_1010_16d4` (`1010:16d4`) draws the `_editWidth × _editHeight` cells into the buffer `_editBuf`, row by row; `_win_DrawEditWindow` (`1010:14f0`) copies the buffer to the screen at the corner of `editTileArea` [code]. Each cell (col, row) comes from `FUN_1010_6250`: map cell `(_MapPnt.x + col, _MapPnt.y + row)`; if y goes past 63, it adds 64 to x and does `y &= 63` (with the bounds, this would only happen with `_editHeight > 64`) [code]. On top of the map come the spider and the balloons (`_DrawSpider`, `_DrawBalloons`; balloons in [baloes.md](baloes.md)). The health graphs (`_DrawEditGraphs`) are not drawn in the Windows modes (`_displayType` 9 and 10; baloes.md §3).

### Spider in the edit window (`_PreDrawSpider` `1010:1c18`, `_DrawSpider` `1010:1d6a`)

The spider is the only creature drawn on top of the cells; antlion, caterpillar, pill bug and pebbles are tiles of `_MapA` (below). Source: pseudocode of the routines cited and of `_DrawLegs` (`1010:2be0`), `_DrawPalps` (`1010:2594`), `_ed_LineTo` (`1010:22fa`), `_PlotLine` (`1018:6fbe`) and `_CopyMaskBitmap2` (`1030:b110`); segment 8 tables with the names from the `.SYM` (`tools/extrair_desenho_aranha.py` → `data/desenho_aranha.json`).

- **When** [code]: `FUN_1010_16d4` calls `_PreDrawSpider` before the cells and `_DrawSpider` after them (before `_DrawBalloons`). The spider appears with `_SpidOn` ≠ 0, `_MapPlane` 0 or 1 and the center (`_SpidX`, `_SpidY`, in map pixels: 16 per cell) inside the view, edges included; the drawing is clipped to the view buffer (`_editWidth × _editHeight` cells).
- **Body** [code]: type 2 sprite `1000 + _SpidDir` (8 sprites, 14 × 36 to 36 × 14) with its corner at center + (`_BodX[dir]`, `_BodY[dir]`). In agony (`_SMode` 5): sprite `1050 + _Scycle` (four frames, from 67 × 67 to 41 × 36, legs already included) at center + (`_DBodX[q]`, `_DBodY[q]`), with no legs or palps. Index 13 (0xD) of the sprite is transparent.
- **Legs** (`_DrawLegs`) [code]: 4 on each side, each in two lines (body → knee → foot), in color `_ConvColor(0xF)`. The tables are for two base directions: `_Lx/_Ly` (where each leg starts) and `_L{k}xA/yA` (knee), `_L{k}xB/yB` (foot) per frame 0–7 for direction 0; `_Dx/_Dy`, `_D{k}xA/yA`, `_D{k}xB/yB` for direction 1. One side uses frame `_Scycle & 7`, the other frame + 4 (alternating step), mirrored about the body axis (x → −x on the straight base; (x, y) → (−y, −x) on the diagonal). The other directions are rotations/mirrors of the base: 2 → (−y, x), 4 → (−x, −y), 6 → (y, −x); 3 → (x, −y), 5 → (−x, −y), 7 → (−x, y).
- **Palps** (`_DrawPalps`) [code]: two, each in two lines (start → knee → tip), with the start at (±4, −10) on the straight base and (10, −3)/(3, −10) on the diagonal, and knee and tip from the tables `_PxT/_PyT/_PxT2/_PyT2` (straight) and `_PxD/_PyD/_PxD2/_PyD2` (diagonal), index r = `S1(4)` drawn at draw time, or 0 with `_SMode` < 2 and `_MeSMode` < 6 (no random draw; regras/rng.md). Same rotations. Color `_ConvColor(1)` if `_MeMode` = 1 (the player's spider), otherwise `_ConvColor(0xF)`.
- **Lines** [code]: `_ed_LineTo` draws each line twice, the second 1 pixel lower (if |dx| > |dy|) or to the right; `_PlotLine` uses a 16-bit accumulator that starts at 0xFFFF (the minor axis moves already on the first step), walks in the increasing direction of the major axis, draws |dy| + 1 points with y as the major axis and |dx| points (without the last) with x as the major axis; with |dx| = |dy| it becomes x-axis with |dx| + 1.
- **Colors on the bench** [measured]: `_ConvColor(0xF)` = black (index 0) and `_ConvColor(1)` = yellow (11), the Mac → Windows conversion from [mapa.md](mapa.md).
- The rest of `_DrawSpider` does not draw: it marks the cells under the spider for the next redraw and handles the spider's balloon ([baloes.md](baloes.md)).

**In the port:** `core::aranha_edicao` (`DesenhoAranha`, `AranhaNaEdicao`), called by `TelaDeJogo::compor_com` after the cells and before the balloon; the palps index comes from the rules (`rules::desenho::desenhar_palpos` stores the `S1(4)` in `Estado::palpos_aranha`). Tables and colors in `data/desenho_aranha.json` (mod API 0.24).

**Verified on the bench (2026-10-18, `tools/cenarios_regras.py gravar aranha_desenho`, snapshot `rico`, game paused, the spider placed in the middle of the view and the view redrawn with Ctrl+↑/Ctrl+↓; captures and states in `re_trabalho/ag_aranha/desenho/`, outside git; port with `cargo run -p engine --example tela_jogo -- JOGO NOME.ANT SAÍDA NOME.png`, `PALPOS=k` for the palps index):** 18 cases — walking in the 8 directions with frames 0–7, stopped, chasing, eating, fleeing, the 4 agony frames, the player's spider walking with `_MeSMode` 6 and stopped. Map area **99.99–100%** and the region of ±48 pixels around the spider **99.91–100%** (13 cases at 100%). In the 4 cases with a random draw, the index that gives 100% is always S mod 4 after the redraw (the last `S1(4)` of the palps), and the player's palps are yellow. The few differences: 8 pixels of a leg that goes past the bottom edge of the view (`andando_d7`, solved on 2026-10-19: see "Line clipping") and 1–2 pixels in the agony.

**Line clipping (`_os_ClipLine`, `1018:6e24`, called by `_PlotLine` before walking the line)** [code]: region codes of the two endpoints relative to the clipping rectangle (edges included: 8 left of 0, 4 right of the maximum, 1 above, 2 below); both on the same outer side → nothing; both inside → draws. Otherwise, an outside endpoint is replaced by the edge point found **by bisection**: the midpoint of the segment to the other endpoint (average with arithmetic shift, i.e. rounded down) replaces the half that is outside, until the midpoint lands exactly on the edge; on the side of the smaller coordinates the new endpoint moves 1 beyond the midpoint (otherwise the bisection does not end). Edges tested in this order: top, bottom, left, right; repeats until both endpoints are inside. The line is then walked by `_PlotLine` from the clipped endpoints — which is not, in general, the original line cut at the edge. **Port:** `core::aranha_edicao::recortar_reta` on each line of `_ed_LineTo` (with the frame's clipping, which is the view's). **Verified (2026-10-19):** the 18 spider captures again with `tela_jogo`: `andando_d7` went to **100%** in the spider region (it was 8 different pixels); the other 17 stay the same (13 at 100%, the agony with 1–2 pixels). The capture of the player's spider menu (`menu_amarela` with `ARANHA=1`, janelas.md §7) also matches: map area and spider region 100%.

**Creatures that are tiles** [measured, `cenarios_regras.py gravar bichos_desenho`, same snapshot]: antlion (9 cells of the trap), caterpillar (5), pill bug (1) and pebbles (7, including a 0x51 placed next to the antlion like the spat one) come out **100%** equal in their cells; the rest of the differences in these captures (0.1–0.8% of the map area) is in ant cells, from the life reconstruction of snapshot `rico` ([testes-tick.md](../regras/testes-tick.md), `vida_nao_reconstruivel`). Nothing is missing.

### Ribbon

The ribbon is window 34 drawn as in the opening screen ([janela.md](../formatos/janela.md)), with one difference in state: `_SetMapPlaneLocation` (`1000:a386`), when there is a ribbon, selects the button of the plane shown with `_win_MakeObjSelected`: `0x2205` (`ribToSurface`) on plane 1, `0x2203` (`ribToBNest`) on 2 and `0x2204` (`ribToRNest`) on 3; on plane 0 it selects nothing [code; the pseudocode shows these numbers as pointers to text, `s_Client_ERROR…_21ef + 0x14…0x16`]. The selected button uses the alternate bitmap (bit 2 of the flags). It is in `data/tela_jogo.json` (`botao_do_plano`). In `QUICK2.ANT` (plane 2), the selected `ribToBNest` matches the capture pixel by pixel [measured].

Ribbon bars and numbers (`ribBlacks`, `ribReds`, `ribYellowHealth`, `ribBlackHealth`, `ribRedHealth`, `ribBlackP`, `ribRedP`; `_DrawMapData`): reproduced since 2026-10-18, checked against the bench ([mensagens.md](mensagens.md) §9). The click on the `mapRibCaste` area (caste panel and health thresholds) is in [mensagens.md](mensagens.md) §9.1; the functions of the configurable buttons `ribUserDef1A`–`1H` are kept between games in `preferencias.json`, instead of `WIN.INI` (`_SetUserButton`; [../../decisoes.md](../../decisoes.md)).

### Objects of window 0

The other objects of window 0 do not appear: only `editWinTitle` (type 18, title drawn by Windows) and `editTileArea` (type 1) have the "visible" bit [`WINGANT` data].

### Frame and position on the QEMU bench (640×480)

The edit window is a Windows window with a title bar, thick frame and scroll bars: the flags byte 0x1C of the layout is 0x0E, and `_win_Open` (`1030:ca2e`) builds the style `WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_HSCROLL | WS_VSCROLL` and adds to the size `SM_CXVSCROLL + 2·SM_CXFRAME` and `SM_CYHSCROLL + 2·SM_CYFRAME`, plus `SM_CYCAPTION` [code]. The exact calculation of the position, with the maximization of step 5, **has not been worked out** [open]. The port uses the geometry measured in `qi_10_quick.png` and `qi_14_salvo.png`, stored in `data/tela_jogo.json` [measured; screen coordinates]:

| Element | Rectangle (x, y, width, height) |
|---|---|
| outer outline (black) | (20, 135, 424, 347) |
| frame (light gray, index 8) | 2 pixels inside the outline |
| inner line (black) | (23, 138, 418, 341) |
| title bar | (24, 139, 416, 18), black line below at y = 157 |
| system menu box | (24, 139, 18, 18), black line at x = 42 |
| maximize button | (422, 139, 18, 18), black line at x = 421 |
| visible map area | (24, 158, 400, 304) |
| vertical scroll bar | (424, 158, 16, 304), black line at x = 424 |
| horizontal scroll bar | (24, 462, 400, 16), black line at y = 462 |
| corner between the bars | (424, 462, 16, 16) |

- The client area of the main window starts at y = 38 (below the menu), and the ribbon occupies rows 38 to 110 [measured], as in DOSBox-X ([janela.md](../formatos/janela.md)).
- The corner of `editTileArea` falls at (24, 158), the corner of the edit window's client area right below the title. This matches the rules of `_win_Recalc` (coordinates relative to the shrunk object 0, minus the title height) [measured; `_win_Recalc` rule seen in the code, full calculation open].
- The background of the main client area comes out white (255, 255, 255) in QEMU, and not (255, 251, 240) as in DOSBox-X [measured].
- Scroll bars: buttons with relief (white on top and left, dark gray index 7 on bottom and right) and light gray track (index 8) [measured]. The arrows, the title text and the scroll bar thumbs (whose position depends on `_ResetEditScrollRange`, not studied) **are not drawn** by the port.

### Colors on the QEMU bench

The Windows VGA driver in QEMU shows some colors with values different from the standard palette: index 1 → (168, 0, 87), 2 → (0, 168, 87), 3 → (168, 168, 87), 4 → (0, 0, 168), 7 → (135, 136, 143), 8 → (192, 199, 200); 0, 9, 11, 12, 14 and 15 come out with the standard values; the application background (index 17 in the port) comes out (255, 255, 255) [measured]. The comparison therefore maps each port index to the most frequent RGB color of the capture in the pixels with that index, and counts as equal a pixel whose color matches the mapping. It is a difference of the bench, not of the game [hypothesis].

## Navigation

How the original changes the edit window's view and the plane shown. Implementation: `engine/src/core/navegacao.rs` (state `Navegacao`, actions, repetition), `TelaDeJogo` in `tela_jogo.rs` (drawing with any view, clicks on the ribbon and on the scroll bar arrows), data in `data/navegacao.json`; the executable (`simant --save`) only translates SDL keys and clicks. Sources: the Ghidra pseudocode (`re_trabalho/decomp/`, reference only) and the accelerator table read from `SIMANTW.EXE`. **Checked on the bench on 2026-10-18** (see "Verification"): Ctrl+arrows, plane switching and the centering when choosing the plane already on screen.

### Keyboard: accelerators and `_DoKeyDown`

The message loop of `WINMAIN` (`1000:3fda`) loads the accelerator table `ACCELERATORTABLE` (a resource of `SIMANTW.EXE`, offset 500224 in the file) and passes each message through `TranslateAccelerator` [code]. The entries that matter here (flags 0x09 = `FVIRTKEY | FCONTROL`) [executable data]:

| Key | Command | Handled in | Effect |
|---|---|---|---|
| Ctrl+↑ | `0xFDA6` | `MAINWNDPROC`, `WM_COMMAND` with high byte 0xFD and low byte 0xA0–0xAF | `_DoEditScroll(WM_VSCROLL, SB_LINEUP)` |
| Ctrl+↓ | `0xFDA7` | same | `_DoEditScroll(WM_VSCROLL, SB_LINEDOWN)` |
| Ctrl+← | `0xFDA8` | same | `_DoEditScroll(WM_HSCROLL, SB_LINEUP)` |
| Ctrl+→ | `0xFDA9` | same | `_DoEditScroll(WM_HSCROLL, SB_LINEDOWN)` |
| Ctrl+8 | `0xFD26` | `_DoMenuEntry` → `_ProcMenu` (`1000:4fc2`), case 0x26 | `_SetMapPlane(1)` and `_MakeEditOpen` |
| Ctrl+9 | `0xFD27` | same, case 0x27 | `_SetMapPlane(2)` and `_MakeEditOpen` |
| Ctrl+0 | `0xFD28` | same, case 0x28 | `_SetMapPlane(3)` and `_MakeEditOpen` |

- After the first step, commands 0xFDA6–0xFDA9 repeat `_DoEditScroll` in a loop while `GetAsyncKeyState` says Ctrl **and** the arrow are still pressed, without waiting for messages [code, `MAINWNDPROC` cases 0xA6–0xA9]. The speed is therefore that of the redraw.
- **Rate and stopped simulation (bench, 2026-10-19, `qemu_ritmo_rolagem.txt`, snapshot `quick`):** the down arrow of the vertical bar held for 1, 1 and 1.5 s moved 22, 25 and 35 rows (~23 steps/s on QEMU TCG; the original's rate depends on the CPU); the left arrow of the horizontal bar, 0.5 s → 3 to 7 and 1 s → all 24 up to the limit. With the game running, `GameTime` stayed still (102 → 102) for 2 s with the arrow held and advanced 13 in the following 2 s: the loop does not handle `WM_TIMER`, **the simulation stops** while the scrolling repeats. **Port (since 2026-10-19):** the same loop for clicking the arrows and for the line-scroll keys: first step on press, then one every `repeticao.intervalo_ms` (43 ms ≈ 23/s, starting after `atraso_ms` = 43) while the button, or the key and Ctrl, stay pressed (`GetAsyncKeyState`; the system's key repeat is ignored), and the game does not advance during that time (`simant/src/jogo.rs`, `Repeticao::ativa`).
- `_MakeEditOpen` (`1010:0e38`) opens the edit window if it is closed [code].
- The arrows **without** Ctrl do not scroll: `_DoKeyDown` (`1000:0e8c`) moves the mouse cursor 8 pixels (`GetCursorPos`/`SetCursorPos`) [code].
- By standard Windows behavior, `TranslateAccelerator` only accepts the input if Shift and Alt are released (the flags do not ask for these modifiers) [hypothesis: API rule, not tested in the game].

### Scroll bars (`_DoEditScroll`, `1010:5c6e`)

`MAINWNDPROC` forwards `WM_HSCROLL` (0x114) and `WM_VSCROLL` (0x115) of the edit window to `_DoEditScroll`; on the codes `SB_LINEUP`/`SB_LINEDOWN` (bar arrows), it repeats while the left mouse button is still pressed (`GetAsyncKeyState(VK_LBUTTON)`) [code, `MAINWNDPROC` (`1000:2930`), cases 0x114/0x115]. The bar's range is that of `_ResetEditScrollRange` (`1010:6086`): from 0 to `64 − _editHeight` vertically and from 0 to `L − _editWidth` horizontally, with `L` = 0x40 on planes 2 and 3 and 0x80 on the others [code]. With `p` = `_MapPnt.x` (horizontal) or `_MapPnt.y` (vertical) [code]:

| Code | Effect |
|---|---|
| 0 `SB_LINEUP` | if `p` > minimum, `p −= 1`; otherwise nothing |
| 1 `SB_LINEDOWN` | if `p` < maximum, `p += 1`; otherwise nothing |
| 2 `SB_PAGEUP` | `p −= _editWidth` (or `_editHeight`); if it goes past the minimum, stays at the minimum |
| 3 `SB_PAGEDOWN` | `p += _editWidth` (or `_editHeight`); if it goes past the maximum, stays at the maximum |
| 4 `SB_THUMBPOSITION` | `p` = thumb position |
| 5 `SB_THUMBTRACK` | same, only if `_OptionStates ≠ 0` (live dragging) |
| 6 `SB_TOP` / 7 `SB_BOTTOM` | the pseudocode writes the minimum (or maximum) to **x and y** at the same time [code; may be a misreading by Ghidra, open] |

- For lines, the map already drawn is shifted (`_EditScroll*Asm` or `_DoEditScrollLine` + `_EditScroll*Color`) and only the new strip is drawn; for the other codes, `_UpdateEdit` redraws with `ScrollWindow` [code]. The result on screen is the same: the view starts at `_MapPnt`.
- `_DoEditScroll` does **not** update the plane's remembered point (`_AMapPnt` etc.) [code]. What updates them are the `_goStep*` (`1030:6efe` ff.), which move the view one step and write `_editWidth / 2 + _MapPnt` [code]. However, no function in the pseudocode calls the `_goStep*` or the `_Finish*Step` [code: search for references in `Code*.c`; indirect calls not searched]: [hypothesis] they are leftovers from the Mac version, and the view only follows the yellow ant through `_GotoMyAnt` (section "Live simulation").
- The symbols `_ScrollUp`, `_ScrollDown`, `_ScrollLeft`, `_ScrollRight`, `_ScrollUpS`, `_ScrollDownS` and `_ScrollRightS` (`1000:96b8`–`96c6`) are empty functions in the Windows version (just `return`); they are called by the `_goStep*` when the step is 1 [code]. Probably leftovers from the Mac version [hypothesis].

### Ribbon: plane buttons (`_ProcMapRibbonEvent`, `1010:c3d4`)

Object events arrive through `_DoEvent` (`1000:0bc2`), which dispatches by the high byte of the id (the window): 0x22 (window 34, map ribbon) goes to `_ProcMapRibbonEvent`, 0x23 (window 35, yard ribbon) to `_ProcYardRibbonEvent` [code]. On ribbon 34 [code]:

| Id | Object | Effect |
|---|---|---|
| `0x2202` | 2 (Yard button) | opens the map window (`_win_Open(0x100)`) if neither it nor the yard window is open and brings the yard one to the front (`_MapToYard`) or raises it to the top; **does not change the edit window** |
| `0x2203` | `ribToBNest` | `_ClearMapScentButtons`, `_SetMapPlane(2)` |
| `0x2204` | `ribToRNest` | `_ClearMapScentButtons`, `_SetMapPlane(3)` |
| `0x2205` | `ribToSurface` | `_ClearMapScentButtons`, `_SetMapPlane(1)` |
| `0x2206` | `ribToYellow` | opens the mini map (`_OpenMiniMapWin`; [mapa.md](mapa.md) §7) |
| `0x2207` | `ribToSpider` (spider button) | `_GotoSpider` (`1000:a844`): without `_SpidOn`, only the error sound (`_myBeginSound(1, 0, 0x7e)`); otherwise, with the surface on screen, `_CenterEdit(_SpidX >> 4, _SpidY >> 4)` and the map toggle cleared; on another plane, `_SetMapPlaneLocation(1, x, y)` (switches to the surface centering once, without touching the remembered point) and `_DrawMap` |
| `0x2208` | `ribHelp` | ribbon help ([ajuda-contexto.md](ajuda-contexto.md)) |
| `0x2209` | `ribStats` (Status Window Button) | `_ScoreDialog`: the Status window, the same as Window → Status ([menus.md](menus.md) §6) |
| `0x220A` | `ribTools` | `_RibbonToolsMenu` |

**Port:** `ribToSpider` and `ribStats` in `data/navegacao.json` → `botoes_faixa_outros` (`Partida::ir_para_aranha`, `simant/src/modos.rs`; test `botao_da_aranha` in `engine/tests/partida_quick2.rs`). Ribbon 35 does not have these buttons [code: `_ProcYardRibbonEvent`]; `editGoSpider` (object 12 of the edit window) calls the same `_GotoSpider`, but is not visible at 640×480.

The map window (`_ProcMapEvent`, `1010:c222`) has equivalent buttons (0x106–0x108 → planes 1 to 3) [code]; the window exists in the port since 2026-10-18 ([mapa.md](mapa.md)). It was not studied in which phase of the click (press or release) the event is generated [open]; the port acts on press.

### Plane switching (`_SetMapPlane`, `1000:a5ca`; `_SetMapPlaneLocation`, `1000:a386`)

`_SetMapPlane(n)` [code]:

1. if `n = 0`, saves the current plane in `_OldMapPlane`; otherwise invalidates the small map (`_InvalEuMap`);
2. `_MapPlane = n`;
3. reads the remembered point of plane `n` (`_YMapPnt`, `_AMapPnt`, `_BMapPnt` or `_RMapPnt`);
4. `_SetMapPlaneLocation(n, point)`: repeats steps 1 and 2; on plane ≠ 0, `_CenterEdit(point)` (with `_UpdateEdit` if it moved) and redraws the map; on plane 0, writes to `_YMapPnt` (`DS:789e`) the pair from `DS:7a60` (not identified); then `_SetMapModeAnt` and the selection of the ribbon button (section "Ribbon", above);
5. on plane ≠ 0, `_CenterEdit(point)` **again** and redraws the map.

Consequences:

- The second `_CenterEdit` completes the step that the first may have lost on the minor axis (truncation described in "Centering").
- `_CenterEdit` starts from the `_MapPnt` of the previous plane, already with the bounds of the new plane.
- Choosing the plane that is already on screen also centers: the view returns to the remembered point, undoing the scrolling done with the bars (which does not update the point) [code; not tested in the original].

`_GotoMapPoint(n, x, y)` (`1000:a52a`): if `n` is the current plane and > 0, only `_CenterEdit(x, y)`; otherwise `_SetMapPlaneLocation(n, x, y)` (switches plane centering on (x, y), without reading the remembered point) [code]. It is used, for example, by clicks on the small map; in the port, only markers 1–7 call it (`Navegacao`, `core/partida.rs`).

### What the port does

- State: `Navegacao { plano_mapa, vista, pontos }`, initialized from the save. With `posicao_inicial: "ao_carregar"` (default in `data/navegacao.json`) the view is the one after "Load Game" (section "Position on load") and the point of the `_MePlane` plane receives `(_MeLocX, _MeLocY)`, as in `_DoLoadInitializations`; with `"salva"`, the saved values are kept.
- Actions: `linha_*` and `pagina_*` follow the `_DoEditScroll` table (without touching the remembered points); `superficie`, `ninho_preto` and `ninho_vermelho` follow `_SetMapPlane(1/2/3)` with the two centerings. The result is bounded with `_BoundEditPosition` also when `_CenterEdit` does not move, as the drawing does (`FUN_1010_16d4`).
- Inputs enabled by default: the original's accelerators (Ctrl+arrows, Ctrl+8/9/0), the ribbon buttons `ribToSurface`, `ribToBNest` and `ribToRNest` and the scroll bar arrows (click, with repetition while the button is held). **Port extensions**, marked in `data/navegacao.json` and in [divergencias.md](../../divergencias.md): arrows without Ctrl scroll (in the original they move the cursor) and PageUp/PageDown scroll one page vertically.
- Also implemented later (2026-10-18): thumb dragging (with AutoTrack the view follows, without it it jumps on release, as in the original; `divergencias.md`), page by clicking the track (`SB_PAGEUP/PAGEDOWN`), the Yard button and the map and yard windows ([mapa.md](mapa.md), [quintal.md](quintal.md)), markers with `_GotoMapPoint`. Not yet: `SB_TOP/BOTTOM`. The mini map (`ribToYellow` button) centers the view with `_CenterEdit`, like the map window ([mapa.md](mapa.md) §7, in the port since 2026-10-18). The view following the yellow ant is in the next section.

## Live simulation

How the original runs the game while the game screen is open, and what the port reproduces. Rules for the timer, speed and pause: [simulacao-visao-geral.md](../regras/simulacao-visao-geral.md) §1–4; yellow ant controls: [formiga-amarela.md](../regras/formiga-amarela.md) §11. Implementation: `engine/src/core/partida.rs` (`Partida`, windowless), data in `data/partida.json`; the executable (`simant --save` / `--novo-jogo`) only translates SDL into `Partida` calls and redraws.

### Clock

- Time source: the 17 ms `SetTimer` (`MYTIMERFUNC`), 170 ms with the application unfocused (`WM_ACTIVATEAPP`) [code]. On Windows 3.1 the interval is rounded to the system clock resolution (55 ms): the 17 ms one gives **one `WM_TIMER` every 55 ms** [verified on the bench, simulacao-visao-geral.md §3.1]; the 170 ms one, 220 ms [hypothesis]. In the port, `Partida::avancar(ms)` turns real time into messages every `intervalo_ms()` = `intervalo_mensagem_ms` (17) or `intervalo_sem_foco_ms` (170) rounded up to `resolucao_timer_ms` (55), keeping the remainder. Since `WM_TIMER` messages do not accumulate, at most `max_mensagens_por_avanco` (3) messages come out of one call; the excess time is discarded [port choice].
- The `MYTIMERFUNC` loop (simulacao-visao-geral.md §2): a message outside the loop opens a call (delay, DGROUP `0x02EE`, + 1); inside it, it is drained at the end of the current pass (delay + 1). Between two messages, `voltas_por_periodo[_GameSpeed]` passes run; each adds 1 to `_gameCycles` (`Estado::ciclos_jogo`), goes through `Relogio::liberar_tick` (pause, §4: runs if not paused, or if `_MeMoveMe` ≠ 0 and `_CurGameTool` < 10; speed, §3: `_SpeedDelayVals` = 6, 2, 0, −1 from `data/simulacao.json`, runs if the limit ≤ 0 or ≤ delay and resets the delay) and ends with the original's exit test (drained > 1 and (passes > 2 or drained > 2)). Each call therefore lasts 3 messages. Result: slow = 1 tick every 6 messages (3.03/s; bench 3.0), normal = 1 every 3 (6.06/s; bench 5.85–6.0), fast = 11 per call with the default 5 passes (66.7/s; bench 55–66), ultra = 15 per call with 7 passes (90.9/s; bench 91) [the number of passes is data: in the original it depends on the CPU].
- `_gameCycles` counts the loop passes, whether the tick runs or not, as in the original: in normal it goes past 100 between two ticks (the bird is tested every tick, testes-tick.md), in fast/ultra it advances 1 per tick (bench). The Mac clock comes from `Estado::relogio_mac`, which `Partida` fills before each tick with ⌊accumulated real ms / 55⌋ × 3 (`_MacTickCount`, regras/quintal.md "Clocks"); in normal it advances 9 per tick, as on the bench.
- Ignored by the port [divergencias.md]: the loop exit when the left mouse button has been pressed (`GetAsyncKeyState(VK_LBUTTON) & 1`).
- The delay keeps counting during the pause: on unpausing, the first tick comes on the following message [code: the +1 comes before the test].
- Yellow ant animations within the tick (`_AnimYellowFight`, `_AnimYellowInsane`, [formiga-amarela.md](../regras/formiga-amarela.md) §10.2): the original redraws the edit window every frame and does not return to the message loop until the end (~0.8 s in the fight). The port receives the frames in `Evento::Animacao`, ends the `MYTIMERFUNC` call, adds the duration to the block (the same one as the single-click wait) and, while it lasts, runs no tick, shows the current frame (`Partida::quadro_animado`) and only afterwards shows a modal box requested in the same tick. Real time (and the Mac clock) keeps running. [port choice: the Windows `WM_TIMER` messages during the wait become a single one, as in the rest of the port.]
- Redraw (`_UpdateWindows`, `1000:2224`): every tick in slow/normal, 1 every 2 in fast and 1 every 4 in ultra (`redesenho_mapa_a_cada`); unfocused, only when the frame counter is a multiple of 8 (`redesenho_sem_foco_a_cada`) [code]. [hypothesis] The port increments the counter before the test (in the original the order was not checked). Scrolling, plane switching, clicks and keys request an immediate redraw.

### Scent layer in the edit window (`_ELayerMode`)

[code] `_DoCalcTile` (`1018:4a6b`), on planes 0 and 1: with `_ELayerMode` = 0 (alarm), 1 (black nest), 2 (black trail), 3 (red nest) or 4 (red trail), it reads the pheromone map of the 2×2 cell (`(x >> 1) × 32 + (y >> 1)`) and, if the value exceeds 0x10, draws tile `(value >> 4) − 0x10` (0xF1–0xFF) in place of the ground; life goes on top. What changes `_ELayerMode` (not in the save; −1 = none):

| Origin | Value |
|---|---|
| yellow ant key 0 / `_SetAlarmDropState(1)` (only on the surface and as an ant) | 0 |
| turning the alarm off (key 0 again, entering a nest, death, `_InitYellow`, tutorial) | −1 |
| ribbon scent buttons (`_ProcMapRibbonEvent` `0x220b`–`0x220f`): `_SetMapModeAnt(4..8)` and, if the mode ended up on, 1, 2, 0, 3, 4 (black nest, black trail, alarm, red nest, red trail); if the same button turned it off, −1 | 0–4 / −1 |
| menu of the edit window's `editScent` button (`_EditScentMenu`, `1010:675e`: Off, Alarm, BNest, BTrail, RNest, RTrail) | −1–4 |

In the port: `Estado::camada_edicao`, set by `rules/amarela.rs` (key 0 and turn-offs) and `Partida::botao_de_cheiro`; `TelaDeJogo::atualizar` copies the layer and `compor` overlays it on the surface (`MapaChao::com_feromonio`). Test `camada_de_cheiro_na_janela_de_edicao` (`engine/tests/tela_jogo_real.rs`): in `QUICK2.ANT`, with layer 1, exactly the 4 tiles of the black hole change. **Not reproduced, on purpose:** the `_EditScentMenu` menu and the selected state of the `editScent` button: in Windows the object is neither visible nor clickable and nothing makes it so, so the menu is unreachable ([mapa.md](mapa.md) §8). **Bench** (script `qemu_alarme.txt`, 2026-10-18): the yellow ant went up to the surface by an order, key 0 gave `_ELayerMode` = 0 (read from memory) and, without walking, nothing changed on screen (`alarme_3_ligado.png`); after walking 7 cells, the edit window showed the trail in blocks of 2×2 yellow tiles, those of the recent trail stronger and those spread by `_SmoothAlarm` weaker (`alarme_4_andou.png`). The pixel-by-pixel comparison with the port was not done (the yellow ant's path in the port is not the same).

### Event queue

The rules do not call windows or sound: they leave requests in `Estado::eventos`. After each tick and each player input, `Partida` empties this queue into a **timeline** and presents them in order (since 2026-10-19; [../regras/formiga-amarela.md](../regras/formiga-amarela.md) §9.4): a yellow ant animation or a modal notice holds the following requests until they finish, no tick runs while requests remain, and the edit window shows the last snapshot drawn within the tick (`Evento::Instantaneo`):

- `CentralizarNaAmarela` (`_GotoMyAnt`, `1000:a764`, requested by `rules/amarela.rs` with AutoTrack on at each step of `_DoAntMoveY`, with Home, Backspace without an order, click on the yellow ant itself, impossible order, death...) is handled immediately: in Experimental nothing (the original plays the error sound); if the yellow ant is on the plane shown (> 0), `_CenterEdit(_MeLoc)`; otherwise, `_SetMapPlaneLocation(_MePlane, _MeLoc)`, which switches the plane and centers **once**, without reading or writing the remembered point [code].
- `Mensagem(id)`, `Musica(id)` and `DialogoFimDeJogo` go to the interface queue (`Partida::retirar_eventos`, written to the terminal; music without sound). `Mensagem` and `DialogoFimDeJogo` also become **modal notices** (`Partida::avisos`): the message box and the game-over dialog, which stop the simulation until they are closed, like `_StopSimulation` ([mensagens.md](mensagens.md)). `_EndGameDialog` is requested every tick while `_IsGameOver` ≠ 0; `Partida` passes on only the first.
- After each tick, the advice from `_Feedback` (`rules::conselhos::conselho`) goes to the ribbon message (`_EditMessage`, mensagens.md §2).

### View and remembered points

`Navegacao` owns the plane shown and the view. Before each tick and each input, `_MapPlane`, `_MapPnt` and the remembered points go to `Estado` (the rules read `_MapPlane`, for example when deciding whether to center when the yellow ant enters a nest); after the tick, the remembered points come back (the rules may write them).

The view follows the yellow ant **only** through `_GotoMyAnt`: with AutoTrack (Options → AutoTrack, Ctrl+G, `_OptionStates[0]`) on, at each step of an order; off (default for a new game and for `QUICK2.ANT`), the view stays still and the player scrolls or presses Home. The `_goStep*` have no caller in the pseudocode (section "Scroll bars").

### Player inputs

| Input | Original | Port |
|---|---|---|
| Left click in the map area | `_processEdit` with 0x200; double click (`WM_LBUTTONDBLCLK`) with 0x2000 [code] | cell = `((x − area.x) / 16 + _MapPnt.x, (y − area.y) / 16 + _MapPnt.y)` (with the wrap of `FUN_1010_6250` past row 63), only inside the visible client area; `clicks` = 1 → `clicar_no_mapa(..., duplo = falso)`, `clicks` = 2 → double |
| Right click | **toggles help mode** (`_bHelp`): `MAINWNDPROC` diverts `WM_RBUTTONDOWN` before `_DoMouse`; the 0x800/0x4000 branch of `_processEdit` is not reached by the mouse [code + bench, 2026-10-18; ajuda-contexto.md §1] | same (until 2026-10-18 the port treated the right button like the left) |
| Click on the bar arrows and the ribbon buttons | section "Navigation" | as before; the click never becomes an order |
| Space / Insert | left click at the cursor position; double if the next message is the same key [code]. Delete toggles help mode (`_HelpKeyDown`, ajuda-contexto.md §2) | position of the last mouse movement; double if the same key comes within `duplo_clique_teclado_ms` (250) [port choice] |
| Command keys (1–4, 0, X/F9, J and NumPad * with Tab, Backspace, Home, NumPad 0–4) | `_YellowCommandKey`, after the accelerators [code] | `teclas_amarela` (SDL key name → virtual key code) → `tecla_de_comando`; Tab read from the keyboard state |
| Shift+0 / Shift+1..4 (also Shift + NumPad) | Speed → Pause (0x41, toggles `_GamePaused`; on unpausing ends tools 10/11; "Game Paused" on the ribbon) and Slow/Normal/Fast/Ultra (0x43–0x46, `_GameSpeed` = 0..3, **without** a ribbon message: only the speed button writes "Game speed now …") [code, `_ProcMenu`; bench, simulacao-visao-geral.md §3.2] | `teclas` → `AcaoPartida`; the speed appears only in the window title [port extension] |
| Ctrl+G | Options → AutoTrack (0x31, toggles `_OptionStates[0]`) | same |

Order in the port, as in the original: accelerators from `partida.json`, accelerators from `navegacao.json`, `_YellowCommandKey`, keys that imitate the mouse. The port's window title shows speed, pause and AutoTrack.

Notices and buttons ([mensagens.md](mensagens.md)):

| Input | Original | Port |
|---|---|---|
| Ctrl+S / Ctrl+A | File → Save / Save As (`_SaveGame`, file dialog) | writes the `.ANT` to the game's file / to a new name (`--salvar` or `SALVOnn.ANT`), without a dialog |
| Ribbon buttons `ribUserDef1A`–`1H` | `_DoUserButton` (pause, speed with "Game speed now …", options, go to the yellow ant, save; when held: icon menu) | single click the same, except the functions that open windows; no icon menu |
| Pause (Shift+0 or button) | `_SetPause`: "Game Paused" on the ribbon, permanent; unpausing clears it | same (mensagens.md §2.3) |
| Single click on the map | waits ~6 units of `_TickCount` (330 ms) without simulating; when held, `_AntMenu`/Examine | same wait; without the menus |
| Message box / game over on screen | only a click on the box (or Space on the box) closes it; the simulation stays stopped; the box closes by itself in ~14.85 s | same; on closing the game over, the port opens "Select a Game" like the original |

### New game

`--novo-jogo` creates a Quick Game like `_initStuff` + `_NewGame`: generators seeded by the clock (`_SeedSRand`/`_SeedRRand`; in the port, the same seed in both clock reads: that of `--semente N` or the milliseconds of the system clock), `Mundo::novo_jogo(Rapido)` and `Estado::de_mundo`. The screen opens as in `_NewGame` steps 4 and 6: the yellow ant's plane and the view centered on it (the same calculation as the "on load" position). The window is not maximized and there are no caste/mode windows.

### Outside the loop

`--ticks N` runs N ticks in a row (`Partida::rodar_tick`, without clock, speed or pause) and exits; with `--exportar-tela-jogo`, it writes the screen after them. It serves as a windowless regression test (e.g.: 50 ticks of `QUICK2.ANT`: `GameTime` 78 → 128).

## Verification

Example `tela_jogo` with `QUICK2.ANT` and the capture `qi_14_salvo.png`, position table 9/0, comparing the client area (rows 38 to 479 of the capture) and ignoring the "Saved correctly" dialog (176 ≤ x < 472, 171 ≤ y < 310 on screen) [measured, 2026-10-16]:

| Region (screen coordinates) | Equal pixels | What differs |
|---|---|---|
| visible map, (24, 158) to (423, 461) | **100%** (87,128 of 87,128) | nothing: ground, tunnel, grass and the life (yellow ant, half of the queen, egg) that appears below the dialog |
| whole edit window, (20, 135) to (443, 479) | 99.33% | title text, drawings of the system menu box and the maximize button, scroll bar arrows and thumbs |
| ribbon, rows 38 to 110 | 93.27% | population/health bars and `ribUserDef*` buttons |
| whole client area | 78.24% | the above, plus the `mapWin` and caste control windows behind the edit window, which the port did not yet draw on 2026-10-16 |

This table is from 2026-10-16 and is kept as history: after it came the ribbon bars and buttons, the map window and the control windows, each compared in its own document (mensagens.md §9, [mapa.md](mapa.md), [janelas.md](janelas.md)); summary in [../../estado-do-projeto.md](../../estado-do-projeto.md) §2.5.

Inferred color mapping (port index → capture color) the same as in the previous section. Images generated in `re_trabalho/tela_jogo/` (outside git): `tela_jogo.png` (port), `tela_jogo_640x480_mapeada.png`, `diferencas.png` and enlarged crops of the ribbon and the ants.

The test `engine/tests/tela_jogo_real.rs` locks the result with a SHA-256 of the indices and checks the reconstruction of `_LifeA/B/R` of `QUICK2.ANT` (without needing the game's databases) and the view as saved (22, 0) and on load (21, 0).

Navigation (2026-10-16): unit tests in `navegacao.rs` and `tela_jogo.rs`; `tela_jogo_real.rs` checks, with `QUICK2.ANT`, the navigation on load (view (21, 0), black nest point (34, 9)), that the buttons from `data/navegacao.json` exist on the ribbon and are found by the click, and that the three planes draw with nothing pending. The executable exported the screens of the black nest (21, 0), the surface (51, 23) and the red nest, checked by eye, and opened the SDL window for 2 s (`--sair-apos-ms`, `--capturar-janela`). **Bench (2026-10-18, `cenarios_tick.py gravar navegacao`, snapshot rico, game paused):** 12 views, each with the capture and the state read. One Ctrl+arrow moved 1 cell; held ~150 ms by the bench, 2 to 5 (the `MAINWNDPROC` repeat loop, confirmed). Ctrl+9 took the view to (24, 0) = `_CenterEdit` of the remembered point (36, 9); Ctrl+0 to (20, 0) from (32, 1); Ctrl+8 to (52, 23) from (64, 32), and Ctrl+8 with the surface already on screen and the view scrolled to (52, 28) went back to (52, 23); scrolling and plane switching did not change the remembered points. Test `navegacao_igual_a_bancada` (`engine/tests/tela_jogo_real.rs`, save `tests/referencias/saves/NAV_RICO.ANT`). The drawing of each view (`examples/tela_jogo` with the state read): map area **100%** in all 12, including surface and red nest; edit window 99.57% (title).

Live simulation (2026-10-17; rate revised on 2026-10-18): unit tests in `core/partida.rs` (17 ms timer becomes 55 ms, with remainder, message limit, rate per speed 2/6/66/90 ticks in 18 messages, `_gameCycles` per pass, pause and the exception for the moving yellow ant, unpausing ends tool 10, AutoTrack and speed through the accelerators, 170 ms unfocused, redraw in fast/ultra, centering on the same plane and on another plane, game over passed on once) and in `tela_jogo.rs` (computed client area = drawn); `engine/tests/partida_quick2.rs` (50 ticks of `QUICK2.ANT`, `GameTime` + 50; 3.3 s in normal = 20 ticks; single click gives the walk order). The executable ran `--save QUICK2.ANT --ticks 50` (78 → 128), `--novo-jogo --semente 1234 --ticks 300` and `--semente 77 --acoes superficie --ticks 2000` (screens checked by eye, ants spread over the surface) and opened the window for 3 s. **Compared with the original (2026-10-18):** the tick rate per speed (simulacao-visao-geral.md §3.1; before, the port ran about 29 ticks/s in normal, five times the original). AutoTrack, Space as a click and the behavior of the single click (6-tick wait) are left for the bench.

Notices (2026-10-17): the message boxes (1 and 2 lines) and the ribbon message ("Game Paused", "Game speed now fast.") match the bench captures **100%**; the game-over dialog, 97.95%; the C-10 score (19785) matches. Details in [mensagens.md](mensagens.md) §8.

**Not verified against the original:** the view when loading a save (a capture after "Load Game" is missing), and the frame at other resolutions (800×600 in DOSBox-X). The spider has been drawn and checked since 2026-10-18 ("Spider in the edit window").
