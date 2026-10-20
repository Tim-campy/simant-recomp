*Leia em português: [janela.md](../../pt-BR/spec/formatos/janela.md)*

# Game windows (objects of type 0, 1 and 9)

SimAnt has its own window system, inherited from the Mac version (`_win_*` functions in `SIMTWO_MODULE`). Each game window (edit, map, title, caste controls…) is a list of rectangular **objects** drawn by type. Integers are little-endian.

## Identification

The code refers to windows by a number whose **high byte** is the id of the type 0 object in the database. Example: `_win_Open(0x300)` opens window 3 (`titleWin`).

**Source:** fact seen in the code. `_win_LoadWindow` (`1030:c662`) calls `_db_LoadObject(param >> 8, 0, 1)`. The error message is "CANNOT LOAD WINDOW %03x".

| Object | Database | Content |
|---|---|---|
| type 0, id 0–36 | `WINGANT`/`MWINNT` | window layout (this document) |
| type 1, id 0–36 | `SHARED` | names of the window's objects, 32 bytes ASCIIZ each, in the same order |
| type 0, id 128 (0x80) | `WINGANT`/`MWINNT` | three u16; the second is the number of colors in the table (17 in `WINGANT`) |
| type 0, id 129 (0x81) | `WINGANT`/`MWINNT` | window color table, 6 bytes per entry |
| type 9, ids 0, 5, 7, 8 | `WINGANT`/`MWINNT` | position of each window, depending on the resolution |

The 37 windows of `WINGANT` (by the name of object 0): 0 `editWin`, 1 `mapWin`, 2 `scenarioWin`, 3 `titleWin`, 4 `gameOverWin`, 5 `infoWin`, 6 `editScentMenu`, 7 `yllwRecruitWin`, 8 `yllwStartMenu`, 9 `editToolsMenu`, 10 `editToExp`, 11 `editNoExpMenu`, 12 `wallRockMenu`, 13 `placeScentMenu`, 14 `cideMenu`, 15 `digFillMenu`, 16 `blackRedMenu`, 17 `foodMenu`, 18 `modeControlWin`, 19 `casteControlWin`, 20 `miniMapWin`, 21 `historyWin`, 22 `fileSelWin`, 23 `antPopWin`, 24 `antEffWin`, 25 (no name), 26 `dialogPicWin`, 27 `msgWindow`, 28 `debugWin`, 29 `examineWin`, 30 `picStrWin`, 31 `aboutWin`, 32 `editSpiderMenu`, 33 `quitWin`, 34 `mapRibbon`, 35 `ribbonYard`, 36 `iconMenu`.

## Layout (type 0)

| Offset | Type | Field |
|---|---|---|
| 0x00 | 4 × i16 | window rectangle (left, top, right, bottom), filled in at run time from object 0 |
| 0x0C | i16 | number of objects `N`, including object 0, which is the window itself |
| 0x1C | u8 | window flags. Bit 0x20 turns drawing off; bits 0x04 and 0x08 change the style of the Windows window created (`_win_Open`) |
| 0x2C | `N` × 4 bytes | space for far pointers (zeros in the file, filled in on load) |
| 0x2C + 4N | — | the `N` objects, one after the other |

**Source:** verified fact. With this structure, the 37 `WINGANT` layouts are walked to the last byte. `_RepointObjects` (`1030:c606`) advances from object to object using the size at +0x22.

### Object

| Offset | Type | Field |
|---|---|---|
| 0x00 | 4 × i16 | absolute rectangle (left, top, right, bottom), already computed for the reference resolution |
| 0x08 | 4 × i16 | offset of each coordinate (left, top, right, bottom) |
| 0x10 | 4 × u16 | reference of each coordinate: object id (`window << 8 | index`) or parameter index |
| 0x18 | 4 × u16 | mode of each coordinate (see "Layout computation") |
| 0x20 | u8 | [open] |
| 0x21 | u8 | **object type** (see below) |
| 0x22 | u16 | object size in bytes |
| 0x24 | u16 | flags: bit 0 = visible; bit 2 = selected (uses the alternate color and bitmap); bits 7–8 = text alignment |
| 0x25 | u8 | (high byte of the flags) bit 1 = clip the drawing to the rectangle |
| 0x26 | u8 | normal color index (table 0/129) |
| 0x27 | u8 | color index when selected |
| 0x28 | … | type-dependent data |

In object 0, the offsets at 0x08 hold the window's position and size `(x, y, width, height)`: `_win_LoadWindow` copies this value into the position table, or overwrites it with the table.

### Layout computation

**Source:** fact seen in the code, `_win_Recalc` (`1030:e6e2`). Each coordinate *k* (0 = left, 1 = top, 2 = right, 3 = bottom) of each object is:

| Mode (0x18 + 2k) | Value |
|---|---|
| 0 | offset (absolute value) |
| 1–4 | coordinate (mode − 1) of the object given by the reference, plus the offset |
| 5 | parameter of `_win_Open` with index = reference (header, 0x10 + 2·ref), plus the offset |
| other | 1 |

The computation repeats until nothing changes, because references can point to later objects. Example, window 3: object 0 has modes (0, 0, 1, 2) and offsets (64, 72, 512, 342), which gives (64, 72, 576, 414). `titlePic2` has modes (1, 2, 1, 2), with a reference to object 0 in the first two coordinates and offsets (0, 168), which gives (64, 240).

After the computation:

- The window rectangle is that of object 0. If object 0 is an **outline (type 15)** with thickness *t* (byte 0x28), the window shrinks to (L+t, T+t, R−(t+1), B−(t+1)). Example: window 2 (`scenarioWin`, t = 2) goes from (136, 41) with 322×258 in the table to (138, 43) with 317×253.
- Object coordinates become relative to the window: x − (X + 2) with ribbon (x − X without it); y − (Y + 0x12) if the window has a title (bit 0x04 of the flags), otherwise y − (Y + 2) with ribbon (y − Y without it), where (X, Y) is the corner of the window rectangle. Before that, each rectangle is normalized (left ≤ right, top ≤ bottom) [code, `1030:e6e2`]. The port does not reproduce this step: it uses the measured geometry (section "Positioning on screen").
- Finally, **the right and bottom of every object decrease by 1** (the coordinates become inclusive) [code]. `_win_GetObjRect` (`1030:c2d2`), `_win_PrintfAtObj` (`1030:d6da`) and `_win_DrawHBar` (`1030:dadc`) add 1 back when `_ribbonBarHeight` ≠ 0 [code]; but **all measurements** that depend on this match the `_win_Recalc` rectangle **without** that +1: the center of the ribbon text (mensagens.md §2.1), the triangle and levels of the control windows (telas/janelas.md §2), the width 63 of the ribbon bars (mensagens.md §9), Status (telas/menus.md §6) and the 25 columns of the edit window (telas/jogo.md). The port uses this rule (`Retangulo::do_recalc` in `engine/src/formats/janela.rs`) instead of the previously measured adjustments [measured rule; why the +1 does not appear remains open: perhaps the variable read in these functions is not the ribbon height]. Exception still unaccounted for: the bottom of the game-over texts (+1 measured).

### Object types

**Source:** `switch` in `_win_DrawObjectI` (`1030:b710`). `_win_DrawWindow` (`1030:bb96`) draws objects 1…N−1 that have the "visible" bit, in file order.

| Type | Occurrences | Drawing | Data from 0x28 |
|---|---|---|---|
| 0 | 25 | the window (not drawn as an object) | — |
| 1 | 181 | area whose content is drawn by the code (maps, graphs…) | — |
| 2 | 6 | rectangle filled with the background color | — |
| 4 | 1 | nothing (debug log only) | — |
| 5 | 12 | fills and writes text (like type 9) | same as 9 |
| 6 | 41 | **bitmap** at the top-left corner of the rectangle | u16 bitmap id |
| 7, 8 | 1 | nothing (log only) | — |
| 9 | 4 | text | u8 font; from 0x2A, ASCIIZ text |
| 12 | 1 | nothing (log only) | — |
| 13 | 76 | **bitmap button** | u16 bitmap when selected; u16 normal bitmap (0x2A) |
| 15 | 15 | 1-pixel outline in the foreground color | — |
| 16 | 9 | text | u8 font; far pointer (0x2A, zero in the file); from 0x2E, ASCIIZ text (may be a `printf` format, e.g. `%s`) |
| 17 | 1 | fills and writes text (like 16) | same as 16 |
| 18 | 8 | window title, drawn by the frame (not by `_win_DrawObjectI`) | far pointer (0x2A, zero in the file); from 0x2E, ASCIIZ text, like 16 [verified fact in windows 18, 19 and 21: "Behavior Control", "Caste Control", "History Window"; the port reads these titles from here, `data/janelas.json`] |
| 19, 20 | 1 | raised (3D) outline, light and dark sides swapped between 19 and 20 | u8 thickness |
| 21, 22 | 0 | fills and draws the raised outline | u8 thickness |

### Missing bitmap

If the bitmap fails to load, `_win_DrawBitMapAtObj` (`1030:be30`) fills the rectangle with the ground color (`_grndTile`).

## Color table (type 0, id 129)

Each entry has 6 bytes: `[0]` foreground color, `[1]` [open, equal to `[0]` in all entries], `[2]` background color, `[3]` second background color (in mono mode, the background color), `[4..5]` always `E1 04` [open].

When `[2] ≠ [3]` in color mode, `_win_RectFill` (`1030:b54a`) fills with the `_GPatBox` pattern mixing the two colors. Hypothesis: 50% checkerboard [to be confirmed by comparing with DOSBox-X]. Example: entry 1 has `[2]=7` (light gray) and `[3]=8` (dark gray).

## Window positions (type 9)

`_win_LoadAllWindows` (`1030:c806`) loads a table of 40 entries `(x, y, width, height)` in i16 (320 bytes, inside a 480-byte object). The id depends on the video and the desktop height:

| `_displayType` | desktop with up to 480 lines | desktop with more than 480 lines |
|---|---|---|
| 10 (color) | 2/9/0 | 2/9/8 |
| 9 (mono) | 9/5 | 9/7 |

Example: in `WINGANT`, window 3 is at (64, 3) with up to 480 lines and at (64, 72) with more. The absolute rectangles stored in the objects (offset 0x00) coincide with the **more than 480 lines** table: `titleWin` goes from (64,72) to (576,414).

## Positioning on screen

**Verified fact** by pixel-by-pixel measurement of the capture `tests/referencias/capturas/titulo_800x600.png` (DOSBox-X, Windows 3.1 at 800×600, game maximized). The coordinates below are in the client area of the main window, which starts at screen line 38, right below the menu.

- **Ribbon** (`_ribbonBarHeight`): takes lines 0–72 of the client area (height 73) across the full width, with a light gray background (index 8). Window 34 (`mapRibbon`) is drawn in it; its table entry is (0, 17, 639, 73). That is, the ribbon is drawn at y − 17.
- **Other windows:** the outer rectangle starts at (x, y + 73), where (x, y) comes from the type 9 table. This matches `_win_Open` (`1030:ca2e`), which adds `_ribbonBarHeight` when creating the window. For window 3: outer at (64, 145), content at (69, 150).
- **Border:** 5 pixels on each side, 1 black pixel on the outside and 4 in color (166, 202, 240). The 512×342 content goes from (69,150) to (580,491).
- **Window with outline (t > 0):** the border wraps the already shrunk window. Window 2: outer at (138, 116), interior at (143, 121) with 317×253. Object drawing is **clipped** by the interior (bitmap 2500, 318×259, appears cut off). Measured in `novo_jogo.png` (bitmap 2500 located pixel by pixel): object 0 lands at (interior.x − t + 1, interior.y − t). The "+1" in x only is from DOSBox-X's S3 driver: on the VGA and SVGA (vbesvga) test bench object 0 lands at (interior.x − t, interior.y − t) [measured, 2026-10-19; hypothesis: S3 driver clipping]. It is kept in the driver profile (`data/resolucao.json` → `perfis_driver` → `ajuste_x_contorno`; `telas/resolucao.md` §7). On 16-color VGA the window also goes 1 pixel to the right (client area byte-aligned, `CS_BYTEALIGNCLIENT`). With t = 0 (window 3), object 0 lands at the corner of the interior.
- **Client area background:** (255, 251, 240).
- The border and background are **Windows system colors** (no `[colors]` section in `WIN.INI`, hence the defaults), not SimAnt's. In the port, they are at indices 16 and 17 of `data/paleta.json`.

The numbers are in `data/interface.json`.

## Validation

`tools/comparar_captura.py` against DOSBox-X captures:

| Screen | Windows | Matching pixels | Differences |
|---|---|---|---|
| title (`titulo_800x600.png`) | 34, 3 | 99.98% | only the mouse cursor |
| Select a Game (`novo_jogo.png`) | 34, 2 | 99.87% | the cursor and the ribbon buttons `ribUserDef1E`/`1G`, which the game swaps at run time (`_SetUserButton`, with values from `WIN.INI` read by `GetProfileInt` in `WINMAIN`) |

The test `engine/tests/telas_reais.rs` locks in both results with hashes.

The two-color fill pattern (checkerboard) was **not** exercised in this comparison, because no visible object of windows 34 and 3 uses it. It remains a [hypothesis].

## Port

The port uses the objects' absolute rectangles, shifted to the position computed above. In the executable, the table (0 or 8) and the client area size come from the virtual screen (`data/video.json` → `resolucao_virtual`, rule in `data/resolucao.json`; see [../telas/resolucao.md](../telas/resolucao.md)); `data/jogo.json` → `tabela_posicoes` was kept for tests and examples. Text is not drawn yet: the fonts still need to be specified. Windows 34 and 3 have no visible text.
