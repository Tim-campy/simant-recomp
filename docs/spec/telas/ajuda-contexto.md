*Leia em português: [ajuda-contexto.md](../../pt-BR/spec/telas/ajuda-contexto.md)*

# Help: right button, help mode, "?" buttons and cursors

How SimAnt gets to the help (`SIMANT.HLP`, opened by WinHelp in the original). The port's viewer (what shows up afterward) is in [ajuda.md](ajuda.md); the file format in [../formatos/hlp.md](../formatos/hlp.md).

Implementation: `engine/src/core/ajuda_contexto.rs` (context and key table), `simant/src/ajuda_ui.rs` (input and cursors), `engine/src/formats/ne.rs` (cursors from the executable). Data: `data/ajuda_contexto.json`.

## 1. The right button turns help mode on

All game windows (edit, map, yard, controls, ribbon) use `MAINWNDPROC`. In it, `WM_RBUTTONDOWN` (0x204) does **not** go to `_DoMouse`: it toggles `_bHelp` (`DGROUP:0010`) and changes the cursor (on: `HELPCURSOR`; off: the class cursor) [code: `1000:330e`]. `WM_RBUTTONUP` goes to `_DoMouse`, which only updates `_mouse_state`; `WM_RBUTTONDBLCLK` (0x206) is not handled (the second click of a double click does not toggle again) [code]. The `0x204`/`0x206` branches of `_DoMouse` (state 0x800/0x4000, "treated as left" in `_processEdit`) are not reached by the mouse.

**Test bench** (2026-10-18, `qemu_ui_barras_direito.txt` and `qemu_ui_ajuda.txt`, paused `quick` snapshot):

- right click on an empty spot of the ribbon: `_bHelp` 0 → 1 and the cursor becomes the arrow with "?" (`r01_direito_faixa.png`); another right click: 1 → 0;
- right click on the edit window map: `_bHelp` 0 → 1, the yellow ant's destination (`_MeGoalX`) does **not** change (`a01_direito_edicao.png`);
- with the mode on, a left click on the map opens WinHelp at the "Edit Window" topic and the mode turns off (`_bHelp` 0); the yellow ant gets no order (`a02_ajuda_edicao.png`).

This corrects the earlier specification (jogo.md, experimental.md), which said the right click counted as the left one in the edit window and in Experimental: it did according to the pseudocode of `_DoMouse`, but `MAINWNDPROC` diverts 0x204 before that.

## 2. Keys (`_HelpKeyDown`, `1000:0de6`)

Called by `_DoKeyDown` before `_YellowCommandKey` [code]:

| Key | Effect |
|---|---|
| F1 | `WinHelp(HELP_INDEX)`: the index (topic "SimAnt", context 0x100) (`a06_f1.png`) |
| Shift+F1 | toggles `_bHelp` |
| Delete | toggles `_bHelp` (and continues to `_YellowCommandKey`) |
| Esc | with `_bHelp` on, turns it off |

Space and Insert still imitate the left click at the cursor position; the `_DoKeyDown` branch that would treat Delete as a right click is only reached with Space/Insert [code]. The port removed Delete from `teclas_clique` (`data/partida.json`).

## 3. Clicking with the mode on (`_DoEvent`, `1000:0bc2`)

With `_bHelp` on, the event of the clicked object does not go to the window's routine: `_bHelp = 0` and `WinHelp(HELP_CONTEXT, context)` [code]. The context, by object id (high byte = window):

| Object | Context |
|---|---|
| window 0 (edit), any object | 4 "Edit Window" |
| window 1 (map) and 0x19 (yard) | 0x102 "Map Window" |
| windows 5, 0x12, 0x13 | window × 0x100 (Information, Behavior Control, Caste Control) |
| History 0x1503–0x150D | the id itself (Black Population … Help); the rest of 0x15: 0x1500 |
| ribbon 0x2202–0x2209, 0x220B–0x220D, 0x2302–0x2305, 0x2308 | the id itself (Yard View Button … Status Window Button, Patch … Graphic, Help Button) |
| 0x220E, 0x220F (red nest and trail) | id − 3 (the black ones' topics) |
| bookmarks 0x2218–0x221E and 0x2313–0x2319 | 0xFD48 "Bookmarks" |
| configurable buttons 0x2210–0x2217 and 0x230B–0x2312 | context of the button's **function**: table `DS:8AE6` of segment 8 (4 bytes per function; read on the test bench: 0x23F Behavior, 0x23E Caste, 0x23D History, 0x23C Info, 0x237 Trail, 0x23A Pause, 0x239 Speed, 0xFD32, 0x238 Mystery, 0xFD33, 0xFD36, 0xFD31, 0xFD35, 0xFD34, 0x23B Save, 0xFD47 Goto Yellow Ant) |
| other ribbon objects (background, `ribTools`, message, bars…) | 0x2200 / 0x2300 |
| other windows | none: only turns the mode off |

**Test bench:** edit window map → "Edit Window" (a02); `ribStats` → "Status Window Button" (a05); `ribUserDef1A` (function 15) → "Goto Yellow Ant Button" (a10). The topic names come from the `|CTXOMAP` of `SIMANT.HLP` ([../formatos/hlp.md](../formatos/hlp.md)).

The port finds the object like `_win_FindObject`: the front window under the cursor (the last layout object that contains the point) or the ribbon; a click outside any object does nothing and the mode stays on [hypothesis: in the original a click on the main window's background generates no event].

## 4. "?" buttons

| Button | Routine | Context (`_DoWinHelp` uses `id & 0xFF00`) |
|---|---|---|
| `ribHelp` (0x2208, ribbon 34, below the message area) | `_ProcMapRibbonEvent`: `_LessonTemp = 1`, `_DoWinHelp(0x2208)` | 0x2200 "Patch Ribbon Bar" (`a04_ribhelp.png`) |
| `ribHelp2` (0x2308, ribbon 35) | `_ProcYardRibbonEvent`: `_DoWinHelp(0x2308)` | 0x2300 "Yard Ribbon Bar" |
| Caste 0x1303 / Behavior 0x1203 / History 0x150D | `_DoWinHelp` | 0x1300 / 0x1200 / 0x1500 |
| `yardHelpBut` (0x1911, yard) | `_DoWinHelp(0x1906)` | 0x1900 (does not exist in `|CTXOMAP`) |
| `editHelpBut` (0x0006) and `mapHelpBut` (0x0116) | `_DoWinHelp(2)` / `_LessonTemp = 1`, `_DoWinHelp(0x103)` | invisible in the 640×480 layout (flags 0) |

The "?" right below the ribbon's message area, which the user reported, is `ribHelp`. "Mystery" is another button (function 8 of the configurable ones).

## 5. Cursors (`WM_SETCURSOR` in `MAINWNDPROC`)

With `_bHelp` on, always `HELPCURSOR`. Otherwise, in Experimental, over the client area of the edit window when it is the front one (or of the front map window with Shift), the tool's cursor: 0 `MAGCURSOR`, 1 `ROCKCURSOR`, 2 `DIGCURSOR`, 3 `ANTCURSOR`, 4 `FOODCURSOR`, 5 `DROPCURSOR`, 6 `SPRAYCURSOR`; tool 7 and the rest keep the class cursor (arrow) [code]. The cursors are `GROUP_CURSOR`/`CURSOR` resources of `SIMANTW.EXE` (32×32, monochrome; `HELPCURSOR` has the hotspot at the tip of the arrow, as in capture r01) — see [../formatos/ne-recursos.md](../formatos/ne-recursos.md).

The port creates SDL cursors from them (AND 1 + XOR 1, "invert", becomes black) and chooses one every frame (`ajuda_ui::cursor_em`). It does not handle the map window with Shift case.

## 6. What the port does and what is missing

- Right click anywhere (except with "Select a Game" or the tools menu open): the window under the cursor comes to the front and help mode toggles; the second click of a double click is ignored. In Experimental, the right button no longer applies the tool.
- Keys F1, Shift+F1, Delete, Esc; click in help mode; "?" buttons of the ribbons and of the control windows.
- Tests: `ajuda_por_contexto_como_no_original` (simant), `contextos_do_original` (engine), `cursores_do_simantw` (engine).
- **Not verified on the test bench:** right click in Experimental and in the control windows (only in Quick Game, in the edit window and in the ribbon), F1 with a control window in front, Shift+F1, Delete, `yardHelpBut`, the tool cursors.
