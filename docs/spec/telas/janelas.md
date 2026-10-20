*Leia em português: [janelas.md](../../pt-BR/spec/telas/janelas.md)*

# Control windows, History, press-and-hold menus and ribbon buttons

Windows that the configurable ribbon buttons and the Window menu open during a game, the menus that appear when holding down the mouse button, and the drawn state of the buttons. Complements [jogo.md](jogo.md) and [mensagens.md](mensagens.md) (§5, ribbon buttons).

Implementation: `engine/src/core/gerenciador.rs` (order, frame, clicks, scrolling; §8) and `simant/src/janelas_ui.rs` (link with the session), `engine/src/core/janelas.rs` (Caste/Behavior/History, frame with title), `engine/src/core/triangulo.rs` (triangle computation), `engine/src/core/historico.rs` (graphs), `engine/src/core/menus.rs` (menus), `engine/src/core/partida.rs` (button functions, menu commands, click wait). Data: `data/janelas.json`, `data/menus.json`, `data/partida.json` (`botoes_usuario`, accelerators `janela_*`). Executable: `simant/src/main.rs`.

**Sources:** Ghidra pseudocode (`re_trabalho/decomp/`, reference only), DGROUP and code of `SIMANTW.EXE`, `WINGANT` layouts and the QEMU bench captures from the script `tests/referencias/roteiros/qemu_janelas.txt` (`re_trabalho/janelas/j00`–`j12`, outside git). Legend: **[code]**, **[measured]**, **[bench]**, **[hypothesis]**.

## 1. Window stack and position

- In a Quick Game and on Load, `_SetDefaultWindows` (`1000:d66a`) opens (or brings to the front) Caste (`_OpenCasteWindow`, 19), then Behavior (`_OpenModeWindow`, 18), opens the map (or the yard, on plane 0) if it is not open — with the other one open in its place, `_YardToMap`/`_MapToYard` swap one for the other (`_win_Swap`) — and finally brings the edit window to the top (`_OpenEditWindow`; its `WM_NCACTIVATE` swaps ribbon 35 for 34) [code]. **New game with the yard in front (user report in the Tutorial, 2026-10-19):** the port inherited ribbon 35 and left the yard open beside the new map; now the yard turns back into the map and the ribbon goes back to 34 (`JanelasControle::janelas_padrao`, `Sessao::herdar_janelas`; test `jogo_novo_com_o_quintal_na_frente_volta_a_faixa_34`). In a game that starts, the order is `[Caste, Behavior, mapa, edição]`; in a Load with the map already open, `[mapa, Caste, Behavior, edição]` — which is what the bench shows (Ctrl+F6 in x03/x04 first brings up the map "Black Nest View", then Caste) [code + bench]. The ribbon button or the accelerator brings the window to the front (`BRINGWINDOWTOTOP`). Order, focus, frame and clicks: §8.
- Position: the one from the type 9 table (9/0 on the bench, 640×480) plus the ribbon (73), with object 0 shrunk by the outline (`_win_Recalc`); the shrunk rectangle is the outer outline of the Windows window, with the title (18 lines + 1 separator) and the client area below. Measured: plus 2 lines (`ajuste_posicao`) [measured in j01, j02, j04].
- Title: system menu box, line, blue bar (active) or white (inactive) with the name centered; the port writes it with the `Gaia8x14` font, not the Windows system font (~1% difference in the comparisons).
- The titles ("Caste Control", "Behavior Control", "History Window") live in `data/janelas.json` [measured; the source of the text in the original has not been located].

## 2. Caste Control and Behavior Control

Variables (all in the save): `_casteLevels`/`_modeLevels` (3 levels from 0 to 65535 that add up to 65535), `_casteSetLevels`/`_modeSetLevels` (4 sets of 3), `_CasteAuto`/`_ModeAuto`, `_IdealCaste` (%). Outside the save: `_casteSet`/`_modeSet` (`1048:159e`/`15a0`, chosen set) and `1048:15b4`/`15b2` (show percentage; 1 at the start) [code].

### 2.1 Triangle (`_InitTriVars`, `_GetTriLatDist`, `_BoundPointToTri`, `_UpdateCasteWindow`)

With the rectangle of the `casteTriangle`/`modeTriangle` object (with the extra pixel from `_win_GetObjRect`): width `W`, half `M = W >> 1`, height `H`; top vertex at (left + M, top), bottom ones at (left, bottom) and (right, bottom) [code]:

- **Click** (`_ProcCasteEvent` 0x130d): only inside the triangle (`_IsPointInIsoTri`); switches to Manual, shows the sets; while the button stays pressed, the point is clamped to the triangle (`_BoundPointToTri`) and turned into levels (`_GetTriLatDist`):
  - `n0 = ((H − dy − 2) × 65535) / (H − 2)`, with `dy` = y − top (0 if `dy > H − 2`);
  - `w0 = n0 × M / 65535`, row `L = W − 2·w0`; with `L < 3`, `n1 = n2 = 0`;
  - `dx = x − left − w0`; `n2 = 0` if `dx < 3`; `(65535 − n0) × dx / (L − 2)` if `dx < L − 2`; otherwise `65535 − n0`;
  - `n1 = 65535 − n2 − n0`.
- On release: the levels go into the current set; for castes, `_IdealCaste` = (n1·100, n2·100, n0·50, n0·50) / 65535 rounded (`_cvtLevels2IdealCaste`) [code].
- **Click triangle (verified, 2026-10-18):** a drag on the bench (`cenarios_tick.py gravar triangulo`: pressed at (435, 421) on screen, released at (420, 395)) left `_casteLevels` = (18321, 39138, 8076), `_CasteAuto` = 0 and `_IdealCaste` = (59, 12, 14, 14). This only checks out with `H − 2` = 93, that is, with the same triangle as the button (below: without the extra pixel and 1 less on the right and bottom); the port now uses this triangle for the click too (test `arrasto_no_triangulo_igual_a_bancada`). The window after the drag: 99.80% (the cursor).
- **Button** (bitmap 1400, 14×14, index 13 transparent): center at `y = (H − 2)(65535 − n0)/65535 + top`, `x = (L − 3)·n2/(65535 − n0) + left + w0 + 2` (or the vertex, with n0 = 65535) [code]. The port uses the rectangle as `_win_Recalc` writes it (right and bottom − 1, inclusive coordinates; `Retangulo::do_recalc`, formatos/janela.md), without the +1 of `_win_GetObjRect`: that way it matches j01 and j02 and the drag above. It is the same rule as the ribbon text and the levels; why the +1 with ribbon in the pseudocode does not show up remains open.
- Order of the levels: castes = breeders (top vertex), workers (left), soldiers (right); behavior = forage, dig, nurse [code + j01/j02].

### 2.2 Buttons

| Object | Effect |
|---|---|
| Auto (0x1304/0x1204) | `_CasteAuto`/`_ModeAuto` = 1 and hides the sets (group 4) |
| Manual (0x1305) | = 0, shows the sets |
| Set 1–3 (0x1306–0x1308) | stores the levels in the current set, loads the chosen one, switches to Manual |
| number areas (0x130f–0x1311) | toggles percentage / number of ants |
| ? (0x1303) | `_DoWinHelp`: help in context 0x1300 "Caste Control Window" (ajuda-contexto.md §4) |

[code]. Measured (j01, with `_CasteAuto` = 1): Auto and the Sets appear raised and Manual sunken; the port draws the Sets without a selection with Auto on [measured; the logic of the "invisible" groups has not been settled].

### 2.3 Levels and bars (`_DrawControlLevels`)

- Text in each level object (`casteBreeders`… / `modeForage`…), font 4: `"%-ld%%"` of `(n·100 + 0x3FFF)/0xFFFF`, or `"%-ld"` of `(n·total + 0x3FFF)/0xFFFF` with total = `_CastePopB[0]` (castes) or `_ModePopB[0..2]` (behavior) [code].
- Bars in the `casteGraphs`/`modeGraphs` object: width `(right − left)/5`, bar i at `left + 2i·width`; they go from bottom − 2 up to bottom − 2 − n·(height − 2)/65535, black outline, and the rest of the column in white [code; vertical geometry measured in j01]. Colors: those of `_casteColors` (45, 44, 41) and `_modeColors` (42, 43, 46) go through `_ConvColor`, whose table has not been read; the port uses measured indices (W blue, S red, F green, D yellow, N cyan; B magenta [hypothesis]).

## 3. History (window 21)

- `_OpenHistoryWindow` (`1000:8a6a`); 10 buttons (`histBPop` … `histKilled`, ids 0x1503–0x150c) turn on up to 4 graphs (`_ToggleHistButton`): the new one goes to the front of list `8:8e54` (the fifth, oldest, drops out) and takes the first of the 4 free slots (`8:8e5c`), which defines the color (`8:8e4c`: `_ConvColor(0x43, 0x46, 0x49, 0x45)` [port colors: hypothesis]). At the start no graph is on [bench j04; DGROUP].
- Series of button b: `g = (b & 1)·5 + b/2` in `_HistGraphs` (`8:8672`: H_BPop, H_BFood, H_BHeal, H_FoodA, H_ADead, H_RPop, H_RFood, H_RHeal, H_PDead, H_FDead); the scale also uses the partner from `_AltHistGraphs` (`8:869a`) [code + DGROUP].
- `_drawHistGraph`: area `histGraphArea` in white; top + 8 and bottom − 8; largest value M of the two series over the `_HistCnt` samples; multiplies by k while `k·M` fits in the height, or divides; 64 samples of the ring starting at `_HistStart`, x advancing `(width − 64)/64` per sample plus the graph number; name (list 4/1050) in a box at the end [code]. Holding the click on the graph shows the values instead of the names (not reproduced).
- **With data (bench, 2026-10-18, `cenarios_tick.py gravar historico`, rich snapshot):** BPop, RHealth, SFood and Killed turned on in sequence and then Expired (which pushes BPop out). Colors of slots 0–3, measured: red (9), dark blue (4), olive (3) and dark magenta (5) (the port had 12, 9, 10, 13). The lines and the name box are 1 pixel above and to the left of the object rectangle and use the width without the extra pixel (`ajuste_grafico` in `data/janelas.json`; the fill still uses the extra pixel). Port with `simant --save hist.ANT --historico 0,5,…`: **99.0–99.6%** in the window (previously 97.4–99.0%); 1 pixel remains on some steps of the lines [cause unknown: rasterization of `_GLine`].

## 4. Press-and-hold menus

### 4.1 Single-click wait and decision (`_processEdit`)

See mensagens.md §7. The port starts the wait on press (`Partida::iniciar_espera_do_clique`) and, after 6 × 55 ms, decides with the button still pressed and Shift released (`clicar_depois_da_espera(…, segurando)`): on the yellow ant itself, `_AntMenu`; on another cell with life, Examine (`_MagnifyMenu`); otherwise the usual order (rules: `rules::amarela::clicar_no_mapa` with `segurando`).

### 4.2 `_AntMenu` (`1010:6d24`)

- Window 7 (ant), 8 (winged queen without a colony: `_MeType` = 0x40 and `8:8a6c` = 0) or 32 (spider, `9:9fe8` = 1), opened by `_win_DoProxMenu` at the click point and kept inside the main window; the item under the cursor is inverted (`_GRectInv`); on release, the item becomes `_YellowCommand` through the DGROUP table (`1048:1baa` = 1, 2, 4, 5, 6, 0; `1ba6` = 8, 9, 0; `1bb0` = 10, 11, 6, 0); with Shift, 2 (recruit 10) becomes 3 (all). Code 0 = Help [code + DGROUP].
- Position: the outer outline (5-pixel Windows border) 19 pixels to the right and 8 below the click in the ant menu [measured in j09]. **100%** identical to capture j09. **Winged queen and spider (bench, 2026-10-18, `cenarios_tick.py gravar menu_amarela`, snapshots full and rico; for the spider, X and a click on its cell beforehand):** 3 to the right and 0 below (queen) and 1 to the right and 8 below (spider), one capture each [cause of the difference not located; `_win_Open(id, x, y)` receives the same point]. The port uses each menu's offset (`data/menus.json`): queen **99.02%** (the cursor); the spider menu matches; the region was 87% because the port did not draw the spider, and with its drawing (2026-10-18, [jogo.md](jogo.md) "Aranha na janela de edição") the map area of the same capture comes out 100%. The frame comes out dark blue on the bench and light blue in the port: it is the active border color of Windows in each environment (QEMU × DOSBox-X), not the game's [hypothesis]. The original opens the spider menu by holding the button on the spider's cell, which has no mark on the life map.

### 4.3 Examine (`_MagnifyMenu`, `1010:6b4c`)

Called by `_processEdit` (after the single-click wait, button still pressed, Shift released, cell that is not the yellow ant) with `_MapPlane`, and by tool 0 of Experimental [code]:

1. Reads the cell's life (`_LifeA` on planes 0 and 1, `_LifeB` on 2, `_LifeR` on 3); 0 → returns −1 and opens nothing (the click becomes an order; capture j11, empty cell, confirms it).
2. Window corner: one cell to the right and one above the clicked one, `((x − _MapPnt.x) + 1)·_tileWidth + _editTileRect.left`, `((y − _MapPnt.y) − 1)·_tileHeight + _editTileRect.top`; if it goes past the bottom of the main window, it moves up by its own height (without going past the top); if it goes past the right, it moves its own width to the left (without going past the left).
3. Opens window 29 (`examineWin`, 92 × 87: bitmap 4510 as background — with the labels "Behavior" and "Carrying" — and the texts `examStr1..3`) and stays in a loop **without processing messages** while `_ButtonHeld` (left button, Space, Insert, or the right one/Delete; at least ~2 units of `_TickCount`): the simulation does not advance. On release, it closes and returns the life byte; the order is not given.
4. Texts (`_win_DrawExamineWindow` `1010:69c0`, font 2), with `g = (vida & 0x78) >> 3`: line 1, `_EggStr[vida & 7]` (list 1100) if g = 0, otherwise `_CasteStr[g]` (1101); line 2, `_ModeStr[M]` (1102), M of the ant found in the plane's list (0, "Bored", if not found); line 3, `_LoadStr[g]` (1103).
5. No random draws [code]. Plane 0: line 2 uses an uninitialized variable in the original; the port uses list A. The yellow ant (0xFE/0xFF) gives g = 15, perhaps past the end of `_CasteStr` [hypothesis].

**Port:** `core/exame.rs` (`JanelaExame`: corner, texts and drawing; `data/exame.json`), `rules::experimental::examinar` (life, group, stage, mode); the interface opens the window at the end of the click wait (or with Experimental tool 0) and closes it on releasing the button or key; the simulation stays stopped while it is open. Without a window: `simant --save … --examinar X,Y --exportar-tela-jogo …`. **Bench (2026-10-18, `cenarios_tick.py capturas`, three ants of the black nest from the rich snapshot: the queen's head and tail and an egg):** in the original the frame is dark blue (index 4, 1 black line and 4 blue) and the 1st line of text is black, the 2nd and 3rd blue (12) — fixed in `data/exame.json` (`cor_borda`, `cores_texto`). Comparison in the window region (`tools/comparar_capturas.py`): **82.5%**, **93.2%** and **92.9%**. **Fixed (2026-10-18):** the window is a black outline + 4 pixels of blue (`largura_borda` 5), the area of object 0 (92 × 87) white — the white line around bitmap 4510, which sits at (1, 1) — and the outline at the corner computed by `_MagnifyMenu` (the `ajuste_canto` was added twice in the port, which shifted the window); the three lines go down 1, 6 and 3 pixels relative to the top of objects `examStr1..3` (`deslocamento_linhas`, measured: the layout rectangles have negative heights). Result: examine_1 **97.6%** and examine_2 **99.4%** (1 pixel in x on one line and the cursor remain); examine_0 **82.6%**: in that capture the original placed the window 3 pixels to the right of the other two, with the same cell column and the same view. **Explained (2026-10-18, `cenarios_tick.py gravar examine_sessao`, order 1, 0, 2, 1, 0 in a new session):** it is the **first opening of the session** — the first window (now on cell (34, 7)) came out 3 pixels to the right (alignment 8081 of 8100 pixels with the fifth, on the same cell); the fourth and fifth, on the cells of the first and second, at the computed place. **Cause (2026-10-19):** the window classes that `_InitApplication` (`1000:3dd0`) registers have style 0x1008 = `CS_BYTEALIGNCLIENT | CS_DBLCLKS` [code]; on the first opening `_win_Open` (`1030:ca2e`) creates the window (`CreateWindow`, only when `_win_hwnd` is still 0) and Windows puts the left edge of the client area on a multiple of 8 pixels (1 byte of video memory in 16 colors); on the following ones the window already exists and stays where it was requested. On the bench: white area (the client area) at x = 205 at the computed place and 208 on the first opening — the nearest multiple of 8. The same rule gives the missing column of the mini map on the first opening (mapa.md §7). The port implements the rule (`gerenciador::alinhar_cliente_ao_criar`, `exame.json` → `alinhamento_ao_criar` = 8, in place of the old measured `ajuste_primeira_abertura`) [hypothesis: the tie, remainder 4, rounds up; in 256 colors the alignment would be 1 pixel]. With it, the five openings: **97.6%, 97.6%, 99.4%, 97.6%, 97.6%** (`JanelaExame::canto_ao_abrir`; `simant --examinar … --exame-ja-aberto` for the following ones).

### 4.4 Icon menu (Ctrl+click on a configurable button)

- `_DoUserButton` with `GetAsyncKeyState(VK_CONTROL)` (not "holding", as mensagens.md §5 used to say) opens window 36 (16 icons, functions 0–15) with the item of the current function already highlighted (`__win_SetProxItem`); on release over an item, the function changes (`_SetUserButton`, bitmaps `0x3889 + f` and `0x3857 + f`); without Shift, also the equivalent button of ribbon 35 [code; DGROUP].
- Position: outer outline 1 pixel to the left of the button, just below the ribbon [measured in j07]. **99.93%** (only the cursor). Bitmap 7200 has format 0x04: 4 planes of 1 bpp per row, top to bottom (bitmap.md).

## 5. State of the ribbon buttons (`_SetUserButton`, `_UpdateUserButtons`)

The toggle buttons (4 trails, 5 pause, 7 Music, 9 Sound Effects, 10 Silly, 11 AutoTrack, 12 Events, 13 Messages) appear selected when the option is on; the others are momentary [code]. With this the ribbon went from 93.27% to **96.27%** against j00 (the rest are the population and health bars).

## 6. What has not been done

- Info (function 3, Window → Info): done in [info.md](info.md), without the definition popups and the text links.
- Trails (function 4): done with the map window ([mapa.md](mapa.md)).
- Mystery (function 8, `_MysteryButton`): draws at random (`_RRand(15)`) among placing ants, killing some, food, holes, invincibility, reversed spider, boxes 10022/10024… — implemented in `rules/misterio.rs` ([../regras/misterio.md](../regras/misterio.md)).
- The click on the History graph (values), WinHelp.

## 7. Verification (2026-10-18)

`tools/comparar_captura.py --regiao … --mapear`, port with `simant --save PORT50.ANT --botoes F,… / --menu amarela@X,Y / --menu icones@OBJ / --marcar N --exportar-tela-jogo`:

| Window | Capture | Identical | What differs |
|---|---|---|---|
| Caste Control (in front) | j01 | **98.87%** | only the title text (font) |
| Behavior Control | j02 | **98.72%** | only the title |
| History Window | j04 | **98.92%** | the title and the last line of the frame |
| `_AntMenu` | j09 | **100%** | — |
| icon menu | j07 | **99.93%** | the cursor |
| ribbon with a bookmark placed | j05 | **99.73%** | the cursor; message "Bookmark has been placed." and bookmark bitmaps identical |

Tests: `core/triangulo.rs`, `core/menus.rs`, `core/historico.rs`, `formats/bitmap.rs` (planar), `engine/tests/tela_jogo_real.rs` (click on the triangle, Set, Auto), `engine/tests/partida_quick2.rs` (buttons, yellow ant menu, bookmarks).

**Verified on 2026-10-18:** drag on the triangle, History with data, queen and spider menus, Examine within a session (sections 2.1, 3, 4.2, 4.3; `tests/referencias/README.md`). **Not verified:** other resolutions. (The spider drawing was checked on 2026-10-18, jogo.md.)


## 8. Window manager: frame, order, focus, clicks and scrolling

The game windows inside the play area (edit 0, map 1, Info 5, Behavior 18, Caste 19, History 21, yard 25) are **Windows child windows** (`WS_CHILD|WS_CLIPSIBLINGS`) of `_rootWnd`, the client area below the ribbon; the ribbon is another window. Whoever draws the frame, moves, resizes and decides who gets the click is Windows 3.1 (`DefWindowProc`), with the exceptions handled in `MAINWNDPROC` (`1000:2930`). The port reproduces this in `engine/src/core/gerenciador.rs` (no SDL; data in `data/janelas.json`, "gerenciador" and "teclas").

**Bench:** scripts `tests/referencias/roteiros/qemu_gerenciador_janelas.txt`, `…2.txt` and `…3.txt` (captures `re_trabalho/janelas_wm/w00`–`w24`, `v00`–`v17`, `x00`–`x06`, outside git), with PORT50.ANT loaded as in `qemu_janelas.txt`.

### 8.1 Style of each window (`_win_Open`, `1030:ca2e`)

The style comes from bytes 0x1C and 0x1D of the layout header (`formats::janela::Janela::flags`) [code]:

| Bit | Style (high word) | Effect |
|---|---|---|
| 0x1C & 0x04 | 0xC8 = `WS_CAPTION|WS_SYSMENU` | title with system menu box; without 0x08, thin frame (`WS_BORDER`) |
| 0x1C & 0x08 | 0x34 = `WS_THICKFRAME|WS_HSCROLL|WS_VSCROLL` | thick frame (resizable) and both scroll bars |
| 0x1C & 0x10 | 0xCA (title, menu, `WS_MINIMIZEBOX`) | no `WINGANT` window |
| 0x1D & 0x01 | 0xC9 (title, menu, `WS_MAXIMIZEBOX`) | only the edit window |
| object type 12 or 18 | 0xC0 (`WS_CAPTION`) | title |
| 0x1C & 0x40 | — | "modal": with it on top, clicking the others does not activate them (`WM_MOUSEACTIVATE` returns 3) |

`WINGANT` values: edit `0e/05` (title, menu, maximize, thick frame, bars); map, Caste, Behavior, yard `06/04` and Info `06/1c` (title and menu, thin frame); History `06/0c`; menus and boxes `4x/xx` (modal, no title). Consequences [code + bench w01–w13]:

- **No child window has a minimize button** nor a minimized icon: "Minimize" in the system menu is grayed out. Only the main SimAnt window minimizes (in the port, the SDL window; `JanelasDesenho::minimizada`).
- Only the edit window has a maximize button (▲; when maximized, the restore one ↕), a thick frame and scroll bars. The others can only be moved.
- **In the Tutorial the edit window opens maximized:** `_NewGame` (`1000:d6dc`), after opening the windows, calls `_win_Zoom(0)` (`ShowWindow(…, SW_MAXIMIZE)`) if the type chosen in "Select a Game" is the Tutorial (0) and it is not maximized yet, before the `_CenterEdit` on the yellow ant; Load (including of `Tutorial.Ant`) does not maximize [code]. Matches the bench: in the tutorial vectors the view is 39 × 21 (`VistaEdicao`) and the `_MapPnt.x` of lesson 1 (10, with the yellow ant at x = 29) is that of a 39-column view. **Port:** `Sessao::abrir_na_tela` maximizes through the window manager and centers again (`Partida::centralizar_edicao_na_amarela`), at the size of the virtual screen (test `tutorial_abre_a_janela_de_edicao_maximizada_na_tela_virtual`: 39 × 21 at 640×480).
- Closing is through the system menu: a single click on the box opens the menu; **double click on the box closes** (w09); Close in the menu; Ctrl+F4.

**WinHelp window:** the same window manager handles the help window as a top-level window (minimize, maximize, move, resize, system menu with Close Alt+F4 and Switch To...), with the port's client area as the desktop ([ajuda.md](ajuda.md) §2.1). For it the window manager has the minimize button (`Parte::Minimizar`, to the left of the maximize one), the minimized state (`JanelaFilha::minimizada`: the window disappears and receives no clicks; `SC_RESTORE` brings it back), the `Alternar` command (Switch To...) and the menu with its corner at the box and a shadow (`menu_sistema.alinhar_na_caixa`, `sombra`). The drag outline of thick-frame windows has the thickness of the frame minus 1 (3 pixels, measured in WinHelp, h03) [hypothesis for the edit window].

### 8.2 System menu

`_win_Open` modifies the system menu of each window with `WS_SYSMENU`: removes "Close Alt+F4", adds "&Close\tCtrl+F4" (`SC_CLOSE`, text at `1048:bda7`), removes "Switch To…" and adds a separator and "Nex&t\tCtrl+F6" (`SC_NEXTWINDOW`, `1048:bdb6`) [code]. Result (w04): Restore, Move, Size, Minimize, Maximize, —, Close Ctrl+F4, —, Next Ctrl+F6, opened below the box, with its left corner on the window outline and the first item highlighted. Windows grays out Restore (window not maximized), Size (no thick frame), Minimize (no button) and Maximize (no button) [measured in w04, on a control window]. Escape closes the menu (w04 → w05). Geometry [measured pixel by pixel in w04, 2026-10-18]: black outline of 1 pixel and 149 wide; items of 18 lines starting at line 1 of the outline, with the font cell at the top of the band; separators of 7 lines with the line on the 4th; the bottom outline right after the last item. The item text is left-aligned 16 pixels from the edge; the shortcut (the text after `\t`) is left-aligned in a common column, 88 pixels from the edge, as Windows 3.1 does with menus that have a tab; the letter after `&` is underlined 2 lines below the glyph baseline, across the width of the letter's cell. The port draws it that way, with the Windows System font (`VGASYS.FON`) and the numbers in `data/janelas.json` (`menu_sistema`); the menu lines match the capture. **Since 2026-10-19** (measured in w04 and in the system menu of the main window, menus.md §8): grayed text is light gray (index 8; it was 7, dark gray); the grayed and highlighted item (the Restore of w04) has the `GrayString` text, only the pixels with x + y even in the highlighted text color (white), over the blue [the phase of the checkerboard, tied to the screen or to the text, cannot be distinguished in the captures: both give the same result in the measured menus]; and the system menu box stays inverted (complementary color) while the menu is open (`gerenciador::inverter`). With this the port's menu (`--menu sistema@19`) matches w04 100% outside the mouse cursor and the checkerboard phase (the port's window is in another position). Until 2026-10-18 the port centered the text and the shortcut in their rectangles, which left "Close" stuck to "Ctrl+F4" (user report).

`MAINWNDPROC` handles, for the children (`WM_SYSCOMMAND`): `SC_NEXTWINDOW` (the one below comes to the top) and, **in the yard, `SC_CLOSE` calls `_YardToMap` first** (the yard becomes the map, which is then closed) [code]. `WM_CLOSE` of a child calls `_win_Close` (hides it and brings up the top one of those remaining, `1030:cf98`) [code].

### 8.3 Z order, focus and click routing

- Order: the top one is the "active" one (`_MyGetTopWindow`, `_win_IsWinInFront`); blue title on it and white on the others (j01). `_win_Open` of an already open window only does `BringWindowToTop`; of a closed (hidden) window, shows it **where it was** (the position does not go back to the table's) [code].
- **Click on a window that is not the top one:** `WM_MOUSEACTIVATE` with `WM_LBUTTONDOWN` does `BringWindowToTop` and returns `MA_ACTIVATEANDEAT`: the window comes to the front and **the click is eaten** [code]. Bench: in w12 the click on the maximize button of the inactive edit window only brought it to the front; in v12 the click on a yard patch behind the edit window brought up the yard (and ribbon 35) without choosing the patch. The port eats the first click anywhere on the back window [hypothesis for the title and the bars].
- Only the front window under the cursor receives the click (Windows sends the message to a single window): in the port, the window manager searches top to bottom and the content of the others does not see the click.
- The right click is not eaten (`WM_MOUSEACTIVATE` only eats `0x201`): the window comes to the front [code]; `WM_RBUTTONDOWN` toggles help mode in any window ([ajuda-contexto.md](ajuda-contexto.md) §1).
- **Ribbon:** `WM_NCACTIVATE` of the edit window swaps ribbon 35 for 34; that of the yard swaps 34 for 35 [code]. The ribbon follows the last one activated **between these two**; activating Caste does not change the ribbon. Bench v11/v12.
- Accelerators (`ACCELERATORTABLE`, commands handled in `MAINWNDPROC`): Ctrl+F4 and Ctrl+NumPad− = `WM_CLOSE` on the top one (w07); Ctrl+Space = system menu of the top one; Ctrl+F6 and Ctrl+NumPad+ = the bottom one to the top (x03, x04); Ctrl+P = Move; Ctrl+Z = maximizes/restores the edit window if it is the top one (w13); Ctrl+A (Size) loses to Save As [data + code].

### 8.4 Moving and resizing

- Dragging by the title: a 1-pixel dotted outline follows the cursor (XOR with the gray brush: at points with x + y odd the color underneath becomes the complementary one, index XOR 15 — black → white, dark blue → yellow; the other points stay; measured in w02, 85 of 170 points of the left edge. **Port since 2026-10-19:** `gerenciador::contorno_pontilhado` in XOR, `janelas.json` → `cor_contorno_arrasto` = 15 as a mask; previously it painted gray); the window only moves to the new place on release (w02, w03). The cursor is confined to the client area of the parent window (below the ribbon): the window can go off the top and the left, clipped, and the ribbon stays on top of it (w05, w06: cursor at (0, 73), window with its corner at (−113, 64)) [bench].
- Only the edit window can be resized (thick frame of 4 pixels; the corners grab two sides up to 4 + 18 pixels from each corner). Minimum outline size: width **317** [measured in v17]; height 296 [hypothesis: `_AdjustWndMinMax` adds system metrics and the rectangle of object 0x13; not measured]. Maximum: the parent's area with the frame (or the current size, if larger) [code]. `WM_SIZE` of the edit window redoes the layout and the view: columns and rows that cover the client area (`_UpdateEdit`), with the same corner, limited to the map [code].
- Maximize (▲ button or Ctrl+Z): fills the parent's area with the frame outside (`ptMaxPosition` = −4); the client area becomes 624 × 334 (39 × 21 cells); the button becomes the restore one (w13, v01). Double click on the title maximizes/restores [hypothesis: Windows default].

### 8.5 Scroll bars of the edit window

Only the edit window has bars (`WS_HSCROLL|WS_VSCROLL`). Ranges (`_ResetEditScrollRange`, `1010:6086`): vertical 0..64 − `_editHeight`; horizontal 0..128 − `_editWidth` (0..64 in the nests) [code]. Position = `_MapPnt` [code].

- Drawing (j00): arrows of 1, 3, 5, 7 pixels + 3 × 3 shaft, centered in the 17-pixel cells at the ends; the thumb is a 17-pixel cell (black lines included) at `track + (travel − 17) × pos / max`, with the track starting from the black lines of the arrows. Measured: pos 22 of 39 → thumb at x = 238 on screen.
- Messages (`_DoEditScroll`, `1010:5c6e`): arrow = `SB_LINEUP/DOWN` (one cell; `MAINWNDPROC` repeats while the button stays pressed); track = `SB_PAGEUP/DOWN` (the size of the view, up to the limit; v03/v04); thumb released = `SB_THUMBPOSITION`; thumb dragged = `SB_THUMBTRACK`, which **only moves the view with AutoTrack** (`_OptionStates[0]`) [code; bench: v05 without AutoTrack, the view stays and a dotted outline of the thumb follows the cursor; x01 with AutoTrack, the view follows]. **Rechecked (2026-10-18, script `qemu_patch_rolagem.txt`, captures `re_trabalho/patch_rol/r*`):** `_OptionStates` read = `[0, 1, 1, 1, 1, 0]` (AutoTrack off); horizontal thumb dragged +40 and +80 with the button pressed (r01, r02): the view does not move, only the dotted outline; released (r03): the view jumps to the position. Vertical (r04, r05) the same. With Ctrl+G (`_OptionStates[0]` = 1) and the same drag (r06): the view follows the thumb in the middle of the drag. AutoTrack comes from `WIN.INI` (`[SimAnt] autotrack=`) in a new game and is written back on exit (`WRITEPROFILESTRING` at the end of `WINMAIN`, `1000:3fda`, from `_OptionStates[0]` = `8:85f2`), and a loaded `.ANT` brings its own: whoever played with AutoTrack on once sees the "live" drag in the following games [code; the port stores the equivalent in `preferencias.json`, regras/som.md "Persistência"].
- Ctrl+arrows: the same line messages (jogo.md).

### 8.6 Map ↔ yard and the Patch button

The map (1) and the yard (25) share the same Windows window: `_win_Swap(de, para)` passes the `HWND`, the rectangle and the place in the order from one to the other [code].

- `_MapToYard` (Yard button, Ctrl+Y/5/6/7): with the map open and the yard not, `_SetMapPlane(0)` (the edit window goes to the surface, without centering) and swaps map → yard; otherwise opens the yard [code]. Comes to the front with ribbon 35 [bench w20].
- `_YardToMap` (**Patch button** of ribbon 35, `SC_CLOSE` of the yard): with the yard open and the map not, `_SetMapPlane(1)` (surface), swaps yard → map in the same place, ribbon 35 → 34 and `_DrawMap`; otherwise opens the map [code]. Bench w21: "Yard View" became "Surface View" in the same rectangle, ribbon 34, edit window on the surface. The port only did "close the yard" (recorded in divergencias.md).
- `_OpenMapYard` (Ctrl+M): map open → to the front; yard open → to the front; otherwise `_YardToMap` [code].
- **Cycle of the button in the first ribbon position** (bench 2026-10-18, `qemu_patch_rolagem.txt`, captures `re_trabalho/patch_rol/p01`–`p06`, with `_MapPlane`/`_YardMode` read on each click): the Yard of ribbon 34 and the Patch of ribbon 35 are in the same place, so clicking there repeatedly **alternates between two states**: black nest in the edit window with the map behind (`_MapPlane` 2) → Yard View in front, ribbon 35 (`_MapPlane` 0) → Surface View in the same rectangle, ribbon 34 (1) → Yard View (0) → Surface View (1)…; `_YardMode` stays 0 (the yard mode does not change) and no click closes both windows. To close the yard or the map: the system menu (Close, Ctrl+F4). The port does the same cycle (test `ciclo_do_botao_yard_patch` in `simant/src/modos.rs`). When opening the yard, `_SetMapPlane(0)` takes the edit window to the surface at the same corner (p01: the edit window leaves the nest and shows the surface); the port does the same since 2026-10-18 (quintal.md §4).

### 8.7 Drawing and the "freeze" with the Yard

The port drew the yard on top of everything, after the modal warnings. A modal box (messages from the yard rules, refusal of the patch transfer…) was hidden behind the Yard: the simulation stopped (`Partida::avancar` does not run with a modal open), nothing seemed to happen, and the game "came back" when a click happened, by chance, to land in the rectangle of the hidden box. Now all windows follow the window manager's order (those below the edit window between the ribbon and it; the others after), the ribbon (another window) is redrawn on top of them, and the modal warning, the menus, the drag outline and the system menu come last. The click that "leaked" to the back window also came from this: Caste/Behavior received the click before the yard drawn on top of them.

### 8.8 Verification (2026-10-18)

| What | How | Result |
|---|---|---|
| Edit window frame with arrows and thumbs, map behind, Caste on top | `simant --save j00_estado.ANT --exportar-tela-jogo` × j00, `--mapear` | **99.59%** in the edit window; **99.75%** in the area (0, 73)–(640, 442) (the title text and the cursor remain) |
| Yard title swapped with the map, in front | `--quintal 0` × v12, title band (118, 98)–(634, 140) | **99.99%** |
| Order, routing, drag, limits, menu, close, maximize, resize, thumb | tests of `core/gerenciador.rs` (12) and `core/navegacao.rs` (`polegar_e_tamanho_da_vista`), numbers from the captures | pass |
| Click eaten on the back window (Caste → edit) | `engine/tests/tela_jogo_real.rs` | passes |

**Not verified:** the light gray column and row outside the menus (measured on the menu bar menus and on the main window's, menus.md §3; on the control windows, the background is the same color and cannot be seen; the port does not draw them on the children), the dotted outline (the original does XOR; the port paints gray dots), the minimum height, Move/Size by keyboard, the click eaten on the title and bars of a back window, Ctrl+Space (on the bench it did not open the History menu, w10), the bookmarks of ribbon 35 (`ribbm*`, not wired in the port). The configurable buttons of ribbon 35 were checked on 2026-10-19 (mensagens.md §9).
