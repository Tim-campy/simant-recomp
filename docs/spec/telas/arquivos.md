*Leia em português: [arquivos.md](../../pt-BR/spec/telas/arquivos.md)*

# Files: Load, Save and Save As

The Windows dialogs SimAnt opens to load and save the game, and what `_LoadGame` does outside the save blocks. The file format is in [../formatos/save-ant.md](../formatos/save-ant.md); writing, in [mensagens.md](mensagens.md) §6; the message boxes and the pause after Load, in [mensagens.md](mensagens.md) §6.1.

Sources: code (`_LoadGame` `1010:9fa0`, `_SaveGame` `1010:a23e`, `_FileSelect` `1010:9920`, `_PopMsg` `1008:1878`, in the Ghidra decomp) and QEMU bench captures in `re_trabalho/load/` (l1 the question, l2 the Load dialog, l4 Save As).

## 1. File → Load (`_LoadGame`)

1. If `_gGameNeedsSaving` (`8:8370`) ≠ 0 — the simulation sets it to 1 on every tick (simulacao-visao-geral.md, step 23) —, `MessageBox` with the title **"SimAnt Load"**, the text **"Do you want to save the current ant hill?"**, question icon and Yes / No / Cancel [code]. In capture l1 the highlighted (default) button is **No** [bench].
   - Cancel (`IDCANCEL` = 2): returns without loading.
   - Yes (6): `_SaveGame(0)`, that is, Save As with the file dialog. If it fails or is cancelled, the question comes back [code: the loop only exits with the save done or with No].
   - No (7): continues.
2. `_FileSelect` in Load mode: the common dialog "Load Ant Hill" (§3). Cancel → returns without loading.
3. Stores the name in `1038:9520` (the file for the next Save) and clears `_gGameNeedsSaving` [code].
4. If the name (without the folder) is the tutorial's (`Tutorial.Ant`, string 1800), it generates the tutorial (geracao-mundo.md §6) instead of reading, with the title "Tutorial Game", `_InitSimVars` and `_SeedSRand`, and goes on to `_DoLoadInitializations` and the pause as in a Load [code + bench; see below].
5. Otherwise: edit window title = the file name (as the dialog returns it, in uppercase); opens the file; sets `_JustXfered` (`9:9c70`) = 1, `_LastThemeTime` (`9:806e`, long) = 0, `_TimeTemp` (`9:9b10`, long) = 300 and `_EditMsgDelay` (`9:9e9e`, long) = 0; runs **`_RandYard`** and only then reads the blocks of table `8:9570` [code].
6. `_EditMessage(0, 0, −2, −1, 1)`, `_DoLoadInitializations` (count, windows, view), `_SetMenuEntries`, `_PauseGame(1)` and, with music off, `_StopSong` [code].

### 1.1 The `_RandYard` random draws on Load

`_RandYard` runs with the session's R generator as it was and the `_CurGameType` of the game that was open (0 at program start, before any game): 192 `_RRand` for the patch seeds and the generation of patch (11, 8), which seeds S (geracao-mundo.md). The blocks read afterwards overwrite the world, but S and R stay as `_RandYard` left them: **the state of the generators after Load depends on R before Load and on the previous game's type, not on the file** [code + bench: with R = `0x568a9979` and type Quick, R = `0x7e80df39` and S = `0x98b9` after Load, read with `ler_variavel` in the script `qemu_load_rng.txt`; test `engine/tests/load_geradores.rs`].

`_JustXfered` is in the save (block 297) and is overwritten; `_TimeTemp` = 300 survives (outside the save); `_LastThemeTime` and `_EditMsgDelay` stay zeroed [code].

**Port:** `carregar::geradores_depois_do_load` (S and R) and `TEMPO_TUTOR_DEPOIS_DO_LOAD`; the rest of the state outside the save starts zeroed in a new session. At program start, the R for `--save` is the one seeded from the clock (like `_initStuff`) and the type is 0; on a Load during the game (Ctrl+L or "Load Game" from Select a Game), the R and `_CurGameType` of the open game.

## 2. File → Save and Save As (`_SaveGame`)

- `_SaveGame(1)` (Save, Ctrl+S): with a stored name (`1038:9520`), writes to it without a dialog; without a name, like Save As [code].
- `_SaveGame(0)` (Save As, Ctrl+A, and the Yes of the Load question): `_FileSelect` in Save mode ("Save Ant Hill As") [code + bench l4].
- After writing: `_PopMsg("%s Saved correctly")` — `MessageBox` with the title **"Message"**, information icon (`MB_ICONINFORMATION`, 0x40) and OK [code]. `%s` is the chosen path [hypothesis: the exact text of the path was not captured].

## 3. The file dialog (`_FileSelect`)

With `COMMDLG.DLL` available, `GetOpenFileName`/`GetSaveFileName` with the filter "SimAnt Files(*.ANT)" and the initial name `*.ant`; without it, the game's own dialog (`DIALOGBOX` "SaveAs"/other) [code; only the common one was observed].

Geometry measured on the bench (640×480 screen, client area starting at y = 38; client-area coordinates) [bench l2/l4]:

| Element | Rectangle (x, y, width, height) | Note |
|---|---|---|
| Frame | (56, 15, 472, 247) | black outline, 4-pixel blue border (3 at the top), white interior |
| Title | (62, 20)–(522, 38) | 18×18 system-menu box and black line below; text in System |
| "File Name:" | field (71, 65, 160, 20) | `*.ant` selected (blue background) |
| File list | (71, 90, 160, 106) | 13-pixel items, x + 5; in Save As, the names in gray |
| "Directories:" | path at (254, 68); tree (253, 90, 163, 114) | 16-pixel items, indent of 4 per level, folder icons |
| "List Files of Type:" / "Save File as Type:" | combo box (72, 224, 158, 20) | "SimAnt Files(*.ANT)" |
| "Drives:" | combo box (254, 224, 161, 22) | "c:" with disk icon |
| OK (default) / Cancel | (425, 49, 88, 23) / (425, 78, 88, 23) | |
| "Read Only" | 13×13 box at (425, 153) | |

- Labels, buttons and lists use **MS Sans Serif 8 (height 13) in simulated bold** — each glyph drawn twice with a one-pixel offset, advance + 1; the title uses the **System** font [bench: glyphs with 2-pixel strokes and the list's line height of 13]. Underlined letters (mnemonics) at the baseline + 2.
- Scroll bars without a thumb (the list fits) with disabled arrows: dark gray with the white relief offset (+1, +1); 7-column arrows on rows 4 to 10 of the button [bench].
- **List that does not fit** (bench, 2026-10-19, 18 files, 8 visible; §6): light-gray track (the button color), black arrows and a square thumb (the same raised button as the arrows, 15 × 15) between two black lines. The thumb's top line goes from the bottom line of the top arrow (position 0) to 16 lines above the top line of the bottom arrow (maximum position = files − visible), proportional and **rounded** (positions 2, 8 and 9 of 10: 11, 46 and 51 lines of a 57-line travel). Arrow: one item; track above or below the thumb: one page of visible − 1 (from 2 to 9). A click on an item puts the name in the field, without selection (focus leaves the field), and highlights the item with the dotted focus rectangle.
- **Changing drive and folder:** "a:" and Enter in the field goes to drive A: (the field returns to `*.ant` selected, "Drives:" shows "a:" with the floppy icon, the tree only "a:\"); "c:" and Enter returns to the current folder of C: (`c:\maxis\antwin`, DOS's for the drive); double-click on a tree item opens the folder (the item is highlighted across the full width, with the focus rectangle; the field shows `*.ant` without selection). At the root of C: the tree shows `c:\`, `maxis` and `windows` and the list is empty, with the bar disabled [bench d00–d09].
- Buttons: black outline with clipped corners (two outlines on the default), 2-pixel white highlight at the top and left, 2-pixel dark gray shadow at the bottom and right, stepped at the corners [bench].

### 3.1 Dialog controls (bench 2026-10-19)

Script `tests/referencias/roteiros/qemu_dialogo_controles.txt` (captures `re_trabalho/ag_dlgaj/dlg/g00`–`g21`; floppy with the 18 `.ANT` of §6, 12 folders and a volume label, assembled by `tools/disquete_pastas.py`):

- **Drop-down lists** (`CBS_DROPDOWNLIST`): clicking the box (on the text or the arrow) opens the list below it, from the box's bottom line downward, at the box's width, with the chosen item highlighted; clicking the box again closes it (g01, g02). "List Files of Type" has a single item, the `_FileSelect` filter ("SimAnt Files(*.ANT)", pattern `*.ant`), in 13-pixel items with 7 visible (a 93-line list including the outline). "Drives:" lists the drives ("a:" with the floppy icon, "c:" with the hard-disk one; only the letter, without the label, even with the floppy labeled) in 16-pixel items with 5 visible (82 lines); ↑ and Enter in the open list choose "a:" and the dialog goes to drive A: (g03–g05). With focus, the closed box is highlighted (g05); open, it is not.
- **Drive icons** (16 × 9, g03): measured and stored in `dialogos.json` → `arquivo.controles.icones_unidade`.
- **Folder tree** with 13 items: the bottom arrow scrolls 1, the track one page (6), the top arrow goes back 1 (g06–g08), like the file list (§6).
- **List via keyboard** (g09–g11): clicking an item and three ↓ select the fourth ("delta.ant"), with the name in the field; End goes to the last one, scrolling the list. The selected item has the dotted focus rectangle (yellow dots on the blue).
- **Read Only** (g12): the click marks the box with an X (the two diagonals of the 11 × 11 interior) and puts the focus rectangle around the text. The game does not read the mark: `_FileSelect` passes `Flags` = `OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST` (0x1800) on Load and `OFN_PATHMUSTEXIST | OFN_OVERWRITEPROMPT` (0x802) on Save and does not look at `OFN_READONLY` on return [code]; with the box checked, Load loaded normally (g21).
- **Name field** (`EDIT`): Alt+N selects all; typing replaces the selection; the caret is a black column the height of the font plus 1 (g13); dragging with the button selects; double-click selects the word with the following spaces ("efgh " in "abcd efgh ijkl", g15); Shift+← extends (g16); **Ctrl+A does nothing** (g17); Home and Shift+End select all (g18). Text and selection in bold MS Sans Serif, blue highlight with white text.
- **Dragging by the title** (g19, g20): 1-pixel dotted XOR outline following the cursor (dots where screen x + y is even); on release, the dialog moves to the spot with x on the byte (dragged 60 pixels, the corner went from 56 to 120).

**Port** (`engine/src/core/dialogos.rs`, `engine/src/core/controles.rs` (`CampoTexto`), `simant/src/arquivos.rs`): everything above, plus keyboard focus in the common dialog's order (field, list, tree, type, drives, OK, Cancel, Read Only; Tab and Shift+Tab), mnemonics Alt+N/D/T/V/R, Enter (OK, opens the highlighted folder with focus on the tree, Cancel when focused on it), Esc (closes the open list or cancels), Alt+↓ and F4 open the list, arrows and PageUp/PageDown/Home/End in the lists, a letter typed with focus on the list goes to the next name starting with it, the bar thumbs are draggable and a held arrow or track repeats (400 ms and then every 50 ms [hypothesis]). Drives come from the system (`GetLogicalDrives`, `GetDriveTypeW`; outside Windows, only the path's). **Extensions** ([../../divergencias.md](../../divergencias.md)): Ctrl+A in the field and the mouse wheel in the lists (3 lines). Headless: `simant --dialogo load [--clique-dialogo X,Y[,duplo]] [--arrastar-dialogo X0,Y0,X1,Y1[,solto]] [--tecla-dialogo NOME[+shift][+ctrl][+alt]] [--digitar-dialogo TEXTO] --exportar-tela-jogo …` (in the given order).

**Checked** (`tools/comparar_captura.py --mapear`, port in a folder with the same 18 names; the rest is the mouse cursor or the computer's real path): open type list **99.63%** (g01), list via keyboard **99.89%** (g09, g11), Read Only **99.11%** (g12), typed field **100%** (g13), double-click **99.58%** (g15, the I-beam cursor), Shift+← **99.86%** (g16), moved dialog title **99.44%** (g20). The open "Drives:" list has the geometry of g03, with the computer's drives. Tests: `engine/src/core/controles.rs` (selection and editing) and `engine/src/core/dialogos.rs` (scrolling, thumb, repeat, wheel, keyboard, drop-down lists, Read Only, Tab, drag).

## 4. The message box (`MessageBox`)

Measured with "SimAnt Load" [bench l1]: 348×129 at (144, 138) of the client area, 1-pixel black outline (no thick border), 18-line title like the dialog's, 32×32 icon at (+19, +36), text in System starting at (+69, +44), 58×28 buttons at y + 90, Yes/No/Cancel at x + 71, + 145 and + 221. The width comes from the text (69 + width + 18); centered at (318, 202) [hypothesis for other texts: Windows computes the box from the text; only one was measured].

## 5. In the port

- `engine/src/core/dialogos.rs` draws and handles the message box and the file dialog; `data/dialogos.json` has the geometry, colors, texts and fonts (indices into `interface.json`: 6 = `VGASYS.FON#System`, 7 = `SSERIFE.FON#MS Sans Serif@13+negrito`, both read from the user's Windows installation; without them, the game's font 0).
- `simant/src/arquivos.rs`: the flow (question → Save As → "Saved correctly" → Load; Save; Save As) and the new session from Load. The dialogs are modal: the simulation does not advance while one of them is open [hypothesis: the Windows dialog's message loop could dispatch `WM_TIMER`; in the captures the game was paused]. Keys: Ctrl+L (Load, a port extension, since it has no menu bar), Ctrl+S, Ctrl+A; in the dialog, Enter = OK/default, Esc = Cancel, underlined letters on the box's buttons, typing in the name field, double-click in the list.
- The folder tree shows the real path (ancestors and subfolders) with its own icons; "Drives:" lists the system's drives; the controls are in §3.1.
- **Since 2026-10-19:** the file list scrolls like the original's (thumb, arrows, page; `PartesRolagem` in `core/dialogos.rs`); the tree also scrolls when it does not fit [hypothesis: the original was not scrolled in the tree]; a folder name in the field ("..", "sub", "d:", a path) and Enter changes folder; double-click on the tree opens the folder and leaves it highlighted. The port does not keep the current folder of each drive: "a:" goes to the root of A:. Headless: `--clique-dialogo X,Y[,duplo]` (repeatable) clicks on the dialog opened by `--dialogo` before exporting.
- Headless: `--dialogo load|pergunta|salvar_como|salvo` opens the dialog before `--exportar-tela-jogo`.

## 6. Verification (2026-10-18)

`tools/comparar_captura.py --mapear`, port exported with `--save … --dialogo …` in a folder with `quick1.ant` and `quick2.ant` (like the bench's):

| Dialog | Capture | Region | Equal |
|---|---|---|---|
| Load, left half | l2 | (56, 15)–(250, 262) | 99.97% |
| Load, buttons and Read Only | l2 | (416, 15)–(528, 262) | 99.39% |
| Whole Load (with the folder tree, which shows another path) | l2 | (56, 15)–(528, 262) | ≈97% |
| Save As, left half | l4 | (56, 15)–(250, 262) | 99.48% |
| Load question | l1 | (144, 138)–(492, 267) | 99.52% (the icon is our own drawing) |

**"Saved correctly" (bench, 2026-10-18, `cenarios_tick.py salvo`):** the Quick Game has no name, so Ctrl+S opens "Save Ant Hill As"; with a name that already exists the common dialog asks "This file already exists. Replace existing file?" (Yes/No); then, the "Message" box with the **dark blue** information icon and the text `C:\MAXIS\ANTWIN\QUICK1.ANT` / `Saved correctly` on **two lines** (the full path, and Windows breaks the line). Port (`simant --dialogo salvo`): **87.1%** in the box (178, 137)–(471, 268). Fixed: the color of the "i" icon (`icone_informacao` = 4, it was green). **Fixed (2026-10-18):** the format in DGROUP is `"%s
Saved correctly"` (the line break is in the text itself); the port passes the full path (the real one, `std::path::absolute`) and the box uses the dialog frame style measured here (`estilos[1]` of `data/dialogos.json`: outline, 4-pixel blue border — 3 at the top —, white margin in the title), with the icon 18 pixels to the right of the interior and 17 below the title line, the text at x + 68 centered on the icon's height (here two lines = its height), the OK 22 below and the interior ending 10 below the button; the same rules give the Load question (outline only). `simant --dialogo salvo` against `salvo.png`: **99.28%** in the box (176, 134)–(472, 271) (the rest is the icon, our own drawing); the Load question stays at 99.52%. The center x differs between the two boxes (318 and 324 in the client area) and is measured per style [hypothesis: Windows's criterion not found].

**Load of `Tutorial.Ant` (bench, 2026-10-18, `cenarios_tick.py load_tutorial`):** the file must exist for the open dialog (I saved the Quick Game as TUTORIAL.ANT); once loaded, the original **does not read the file**: it generates the tutorial (`_GenerateTutorial`, geracao-mundo.md §6) — title "Tutorial Game", `_CurGameType = 2`, `_GameTime = 0`, `_TutLesson = 1`, 64 new ants on the surface (`_ListIndexA` = 65), yard populations in the table's 30 patches (checked: 18 black and 12 red, those in the spec), the yellow ant in the black nest at (29, 10).

Since it does not go through `_NewGame`, whatever `_RandYard` does not write stays as it was in the previous game [bench]: the colonies' modes and strategic mode, colonies founded and totals, `_SowDir[0]`, cat (`_CatX/Y/Dir`), `_DogPx/Py`, `_ColonyUpdateFlag`, `_CurEditMode`, `_GamePaused`, `_FuzLocX`, `_MeCrazyCnt`, `_MeFoodInType`, `_MeGoalPlane`, `_MakeMe`, `_ModeMe`, `_StargLife`, `_SuserX/Y` and the X/Y of the slots past the end of lists B and R (T and M zeroed); `_MeGoalX/Y` and `_MeLastX/Y` go to the yellow ant's position; `_MapPnt` is the view centered on it (`_SetMapPlane(_MePlane)`).

**Port:** `carregar::estado_do_tutorial` (with `data/tutorial_arquivo.json`: name, title, the 30 patches and the values of `_InitSimVars`), used by the dialog's Load and by `--save` with that name (`Origem::Tutorial`). **Checked:** `engine/tests/load_tutorial.rs` starts from the before-state of the vector `tests/referencias/load_tutorial` (copied from `re_trabalho/cen/load_tut`) and reaches the after-state with **307/307 blocks and R equal** (S comes from the clock: the test uses the recorded one).

**Many files and changing folder (bench, 2026-10-19, `roteiros/qemu_dialogo_muitos.txt`, captures `re_trabalho/ag_fecha/dlg/d00`–`d09`):** a floppy with 18 `.ANT` (`caso_ant.montar_disquete`) in A:, Load, "a:" and Enter, arrows and the bar's track, one item, "c:" and Enter, double-click on the tree's "c:\". Port in a folder with the same 18 names (`--dialogo load` and the same clicks with `--clique-dialogo`; `comparar_captura.py --mapear`, left half (56, 15)–(250, 262)): list at the start **99.97%**, after two arrows **99.81%**, after the page **99.74%** (only the mouse cursor over the bar), with an item chosen **99.56%** (the dotted focus rectangle, which the port does not draw) and the root of C: **100%** (empty list, disabled bar, field without selection). The port's tree shows the computer's real path, not the bench's.

**Not verified:** the tree with more folders than fit (its scrolling) and the current folder per drive.

## 7. File → Quit (`_MenuQuit`)

The same box as the Load question with the title "SimAnt Quit", always (without looking at `_gGameNeedsSaving`); Yes = Save As and quits if saved, No = quits, Cancel = returns. See [menus.md](menus.md) §4.
