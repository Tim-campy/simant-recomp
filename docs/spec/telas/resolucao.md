*Leia em português: [resolucao.md](../../pt-BR/spec/telas/resolucao.md)*

# Screen resolution and layout (640×480, 800×600, 1024×768)

The original SimAnt scales nothing: bitmaps, fonts and windows always have the same size in pixels. What changes with the Windows resolution is **where** the windows go and **how much** client area is left around them. This document describes how the original decides that and how the port imitates it with the "virtual screen" (`data/video.json`, `resolucao_virtual`).

Legend: **[code]** = seen in the Ghidra pseudocode (`re_trabalho/decomp/`, names from `SIMANTW.SYM`); **[measured]** = measured pixel by pixel in a capture of the original; **[hypothesis]** = not confirmed.

## 1. Main window: maximized, the size of the screen

- `_InitInstance` (`1000:3c8a`) creates the main window and calls `ShowWindow(..., nCmdShow | 3)`: with the 3 (`SW_SHOWMAXIMIZED`) it opens **maximized** at any resolution [code]. The maximized window's frame is off-screen; the title bar and the menu bar remain.
- Right after, the main window's `GetClientRect` gives `_screenWidth`/`_screenHeight` and two child windows are created: the **ribbon** (`_ribbonBarWnd`), with the full width of the client area, and the **root** (`_rootWnd`), where the game windows live [code].
- On the main window's `WM_SIZE` (`MAINWNDPROC`, `1000:2930`), the ribbon is resized to (client area width × ribbon height) and the root to (width × client area height − ribbon height); `_screenWidth`/`_screenHeight` become the root's size [code].
- The client area starts at screen **row 38**: title bar (rows 0–18) and menu bar (19–36, plus the black row 37). **Measured** at 640×480 (QEMU bench, `VGA.DRV`; `data/barra_menus.json`), at 800×600 (DOSBox-X, `S3TRIO.DRV`; `formatos/janela.md`, "Posicionamento na tela") and at 640×480, 800×600 and 1024×768 on the SVGA bench (section 5). The heights come from `GetSystemMetrics` (`SM_CYCAPTION`, `SM_CYMENU`), which depend on the driver's system font; with large fonts they would be different [hypothesis].

So, for a W × H screen:

| Screen | Main window client area | Ribbon (window 34) | Root (game windows) |
|---|---|---|---|
| 640×480 | 640 × 442 | 640 × 73 | 640 × 369 |
| 800×600 | 800 × 562 | 800 × 73 | 800 × 489 |
| 1024×768 | 1024 × 730 | 1024 × 73 | 1024 × 657 |

The ribbon has a light gray background across its full width; the objects of window 34 (`mapRibbon`, 640 pixels wide) sit against the left and the rest is just background [measured at 800×600: `titulo_800x600.png`, columns 640–799 of rows 38–110; and at 1024×768, `svga1024_quick.png`].

## 2. Window position table (object type 9)

`_win_LoadAllWindows` (`1030:c806`) takes the rectangle of the **desktop window** (`GetDesktopWindow` + `GetWindowRect`) and chooses the position table by the screen's **height** [code]:

| `_displayType` | desktop bottom < 481 (up to 480 rows) | 481 or more |
|---|---|---|
| 10 (color, `WINGANT`) | 9/0 | 9/8 |
| 9 (mono, `MWINNT`) | 9/5 | 9/7 |

Width does not enter the computation. **Consequence:** 800×600 and 1024×768 use the **same** table (9/8); there is no table of its own for 1024×768. Only these two exist per video mode.

The differences between 9/0 and 9/8 of `WINGANT` (entries `(x, y, width, height)` in the root area; the other 28 entries are the same) [data, `cargo run -p engine --example inspecionar -- <jogo> posicoes 0|8`]:

| Window | 9/0 (up to 480 rows) | 9/8 (more than 480) |
|---|---|---|
| 3 `titleWin` | (64, 3, 512, 342) | (64, 72, 512, 342) |
| 4 `gameOverWin` | (132, 59, 320, 200) | (155, 130, 320, 200) |
| 5 `infoWin` | (277, 128, 324, 220) | (311, 128, 324, 220) |
| 18 `modeControlWin` | (86, 158, 214, 192) | (86, 285, 214, 192) |
| 19 `casteControlWin` | (342, 158, 214, 191) | (342, 285, 214, 191) |
| 23 `antPopWin` | (0, 0, 184, 144) | (0, 0, 183, 144) |
| 25 (yard) | (62, 18, 516, 277) | (123, 18, 516, 277) |
| 31 `aboutWin` | (176, 117, 322, 202) | (168, 117, 322, 202) |
| 33 `quitWin` | (133, 70, 337, 166) | (133, 145, 337, 166) |

That is: on larger screens the title and the "Quit" move down, the control windows (Caste/Behavior) move down below the edit window, and Info, game over and yard shift. The edit window (0), the map (1), "Select a Game" (2) and the ribbon (34/35) stay in the same place.

`_win_LoadWindow` (`1030:c662`) copies the table entry into object 0 of each window (or keeps it, if the entry is `-32768`) [code]; `_win_Open` (`1030:ca2e`) adds `_ribbonBarHeight` (73) to y when the window is not positioned by parameter (mode 5) [code; formatos/janela.md].

## 3. What adapts to the root's size

- **Picture dialogs** (`_PictureDialog`, `1000:62bc`, window 30 — the opening's registration and others): centered in the **root's client area** (`GetClientRect(_rootWnd)`), half of the leftover on each side [code]. At 800×600, the registration sits at (270, 307)–(539, 411) of the screen [measured in `apos_titulo.png`].
- **Maximized edit window** (Tutorial, `_win_Zoom`, `ShowWindow(SW_SHOWMAXIMIZED)`): takes up the whole root, with the frame outside; `WM_GETMINMAXINFO` (`_AdjustWndMinMax`, `1000:1a6e`) uses the root's `GetClientRect` [code].
- **Message boxes** (`MessageBox`: "SimAnt Load", "SimAnt Quit", the "Message" of "Saved correctly"): centered on the **whole screen** (not on the root nor on the parent window), with the computation done without the 1-pixel outline: corner = ((W − (w − 2)) / 2, (H − (h − 2)) / 2), integer division, and x taken to the nearest byte by the driver (§7) [measured]. "SimAnt Quit" (348 × 129) sits at (144, 176) of the screen at 640×480 VGA, (147, 176) at 640×480 SVGA, (227, 236) at 800×600 and (339, 320) at 1024×768; the 272 × 137 "Message" at 800×600, at (265, 232) (`re_trabalho/svga/ag*_quit.png`, `ag*_salvo.png`; VGA bench `l1`, `cen/salvo`).
- **File dialogs** (`GetOpenFileName`/`GetSaveFileName`, "Load Ant Hill"/"Save Ant Hill As"): **same place** at the three resolutions, corner at (58, 53) of the screen in 256 colors and (56, 53) on 16-color VGA (x on the byte, §7) [measured: `ag*_load_dialogo.png`, `ag*_salvar_dialogo.png`; VGA bench `l2`].
- **Help viewer** (WinHelp 3.1; the `WIN.INI` key `H_WindowPosition` is the History window's and does not change this place, [ajuda.md](ajuda.md) §4): from column W / 3 to W − 3 and from row 2 to H − 3 of the screen: (213, 2)–(637, 477) at 640×480, (266, 2)–(797, 597) at 800×600, (341, 2)–(1021, 765) at 1024×768 [measured; SVGA disk prepared with `--ajuda-padrao`, because the `WIN.INI` of the user's installation stores `[Windows Help] H_WindowPosition=[266,200,266,200,0]`, from an 800×600 session, and with it the viewer would open elsewhere — hypothesis, not tested]. The port puts the viewer from W / 3 to W − 2 (exclusive), from the top to the bottom of the client area (`data/ajuda.json` → `posicao_padrao`).
- **Yellow ant message box** (window 26, `_YellowDialog`, `_SpiderDialog`, `_DrawSimPayoff`): does **not** depend on the resolution: the bitmap sits at (158, 163) of the client area at 640, 800 and 1024 on SVGA (cheat "fred", `ag*_fred_2.png`) and at (160, 163) on VGA (byte alignment on window creation, §7). The bitmap object has an absolute position in the layout (mode 0: (153, 85)) and the window adjusts to it [code: `inspecionar layout 26`].
- **Game over, Info, About, yard**: follow the table (9/0 or 9/8) like the other windows, with the driver's alignment. Measured on SVGA: game over with the outline at (155, 241) of the screen at 800×600 and 1024×768 (table 9/8: x = 155); Info at (313, 241) at 800×600 (311 + outline 2); About at (178, 230) at 640×480 and (170, 230) at 800×600 and 1024×768; the yard opens in the map's place (outline at x = 120 on SVGA, 119 on VGA), the same at the three resolutions. The `_PictureDialog` text box (window 30) is centered on the root and **at the same x on VGA and SVGA** (outline at 186 at 640×480): window 30 was already created by the opening's registration, and alignment only happens on creation (§7).
- The rest (control windows, map, History, Info, Examine, press-and-hold menus) follows table 9 or the click position, without depending on the screen size.

## 4. In the port: virtual screen

The port does not change the monitor's resolution: it draws a Windows 3.1 **virtual screen** and enlarges it with the usual integer scale (`simant/src/video.rs`, decisions "Escala 1" and "Janela nítida").

- `data/video.json`, `resolucao_virtual`: `[640, 480]` (default, the bench's) or `[800, 600]`, `[1024, 768]`. Command line: `--resolucao LxA` (overrides the file).
- `data/resolucao.json` stores the original's rule as data: rows above the client area (38), the table choice by height (up to 480 → 0; otherwise 8) and the accepted resolutions.
- With the W × H virtual screen, the executable uses: client area W × (H − 38); position table by the rule; the opening (`data/telas.json`) and the game screen (`data/tela_jogo.json`) with that size and that table. The SDL window has the logical resolution W × (H − 38) at the opening and W × (H − 38 + menu bar) during a game (the title bar is the current system's), enlarged by `escala`; with `moldura_windows31` (`data/video.json`), the game also gains the Windows 3.1 title bar and becomes W × H, the whole imitated screen (telas/menus.md §8).
- Code: `engine/src/core/resolucao.rs` (rule and application), `simant/src/video.rs` and `simant/src/main.rs`.
- The tests and tools that already existed keep their own configurations: `tela_jogo.json` (640×442, table 0, the bench) and the screens validated against DOSBox-X (800×562, table 8).

## 5. Verification

References from the original: the old DOSBox-X capture (800×600, `S3TRIO.DRV`, 256 colors), the default QEMU bench (640×480, `VGA.DRV`, 16 colors) and the **SVGA bench** (QEMU with the `vbesvga.drv` driver, 256 colors, at 640×480, 800×600 and 1024×768; [bancada-qemu.md](../../bancada-qemu.md), "Bancada SVGA"). SVGA captures in `re_trabalho/svga/` (outside git).

**Verified facts (2026-10-19):**

- The client area starts at row 38 at 1024×768 too (title bar 0–18, menus 19–37), and the ribbon goes from rows 38 to 110 with a gray background up to column 1023; the rest of the client area, down to row 767, has the cream background (`svga1024_quick.png`). The 38 rows are no longer a hypothesis.
- The 800×600 title on the SVGA bench is identical to DOSBox-X's (0 different pixels at 6 bits per channel): the two 256-color environments give the same screen.
- The control windows (Caste/Behavior) sit on row 522 ("Soldiers" box, client area) at 800×600 and 1024×768 and on 395 at 640×480: **127 rows** above, the difference between tables 9/8 and 9/0 (285 − 158). The port gives the same rows at the three resolutions. The edit window stays in the same place at all three (outer outline on row 97).
- "Select a Game" and the title have the same position at 800×600 and 1024×768 (table 9/8); at 640×480 the title moves up (table 9/0).
- The opening composed by the rule at 800×600 is identical (same SHA-256) to the screens already validated against DOSBox-X (test `engine/tests/resolucao_reais.rs`); the test `simant/src/sessao.rs` (`testes_tela_virtual`) locks the control window positions and the screen size at 640, 800 and 1024.

**Differences that come from the driver, not the resolution** (the same at 640×480, 800×600 and 1024×768 on the SVGA bench): the edit window has its outer outline at x = 16 in 256 colors and at x = 20 on the 16-color VGA bench; the control windows, at x + 2 in 256 colors and x + 1 on VGA; the opening's title, 3 pixels to the right on VGA (the "1 + 5 + 1 frame" is the 1 + 4 one plus the content's first column, dark blue, the same color as the frame on VGA); "Select a Game", 1 pixel to the right on VGA. All of them are byte alignment (§7). The system colors also change with the driver (§7).

## 6. Recorded comparisons

Port (`--resolucao LxA --exportar-telas`, `--novo-jogo --semente 1 --exportar-tela-jogo`) against the SVGA bench, `tools/comparar_captura.py --mapear` (equal pixels in the client area):

| Screen | 640×480 | 800×600 | 1024×768 |
|---|---|---|---|
| Title | 99.89% | 99.98% | 99.99% |
| Select a Game | 96.19% | 97.60% | 98.56% |
| Quick Game, whole client area | 87.97% | 91.23% | 94.94% |
| Quick Game, below row 380 (control windows) | 89.84% | 93.39% | 97.32% |

- Title: only the cursor differs. Select a Game: the content of window 2 comes out 1 pixel to the left on SVGA (at all three resolutions), and the ribbon's configurable buttons differ. Quick Game: the original's game is a different one (random world; the port uses seed 1), besides the x of the edit window (−4) and of the control windows (+1) and the color of the title above; the percentage grows with resolution because more identical background is left over. The same port screen at 640×480 against the VGA bench: title 84.88%, Select a Game 99.25% (`qi_01_titulo.png`, `qi_05_selecao.png`).

After the driver profile (§7, 2026-10-19), the port with `"svga256"` against the SVGA bench (same commands, `--mapear`):

| Screen | 640×480 | 800×600 | 1024×768 |
|---|---|---|---|
| Title | 99.96% | 99.98% | 99.99% |
| Select a Game | 99.80% | 99.87% | 99.92% |
| Quick Game, whole client area | 89.34% | 93.34% | 96.20% |
| Quick Game, below row 380 | 96.21% | 98.60% | 99.39% |

And with the default profile (`"vga"`) against the VGA bench at 640×480: title **99.96%** (before 84.88%) and Select a Game **99.80%** (before 99.25%); only the cursor and the ribbon's configurable buttons differ. What remains in Quick Game is the world (random in the original) and the title text (black in 256 colors; the port uses the VGA title colors during a game, §7).

Positions checked by test (`simant/src/sessao.rs`, `posicoes_por_resolucao_e_driver`), in the `"vga"` and `"svga256"` profiles and at the three resolutions: "SimAnt Quit" box, file dialog, edit window, Info, game over, About, yard, help viewer and window 26 — the 256-color values are those measured on SVGA; the VGA ones, those measured on the VGA bench at 640×480 and the rule at the others.

**Not verified:** 16-color VGA at 800×600 and 1024×768 (there is no 16-color driver for those resolutions on the bench; the port applies the rule). SVGA captures taken after a `loadvm` come out with the 256-color palette swapped (black grays, purple title): they are good for positions, not for colors. The help viewer's place with the position stored in the user's `WIN.INI`.

## 7. Driver profile: what changes with the video driver

The same game, at the same resolution, comes out different on 16-color VGA (`VGA.DRV`) and in 256 colors (S3 Trio on DOSBox-X, `vbesvga.drv` on the SVGA bench). The causes found:

1. **Byte alignment.** `_InitApplication` registers the class `GenericWindow` (all the game's windows, `_win_Open`) and `RibbonWindow` with the style **0x1008** = `CS_BYTEALIGNCLIENT | CS_DBLCLKS` [code: `REGISTERCLASS` in `Code1.c`, names at `DGROUP:0684` and `0692`]. On window creation, Windows puts the left edge of the **client area** on the nearest byte of video memory: on 16-color planar VGA, 1 byte = 8 pixels; in 256 colors, 1 byte = 1 pixel (no effect). Rounding to the nearest, with the half going up (the edit window: client area at 16 + 4 = 20 → 24, window at 20) [measured]. Checked in all measured cases: title (64 + 5 = 69 → 72: +3), Select a Game (138 + 5 = 143 → 144: +1), edit (+4), control windows (86 + 2 + 1 = 89 → 88: −1, the "x + 1" versus 256 colors' "x + 2"), game over (132 + 5 = 137 → 136: the −1 adjustment measured before), Info (277 + 2 + 1 = 280: already aligned, the +1 adjustment the port had), About (the "outline 1 pixel to the right of the content"), window 26 (158 → 160). Client area margins by `_win_Open`'s style: dialog frame 5 (`WS_DLGFRAME`, windows without a title), border 1 (`WS_BORDER`, windows with a title), resize frame 4 (`WS_THICKFRAME`, the edit window). Windows **dialog boxes** (`MessageBox`, file dialogs) align the **window's corner**, not the client area (`CS_BYTEALIGNWINDOW` of the dialog class [hypothesis from the behavior]): "SimAnt Quit" 147 → 144, "Message" 173 → 176, file dialog 58 → 56 [measured on VGA against SVGA]. Only **creation** aligns: a window that already exists and is repositioned stays where it was asked to go (the `_PictureDialog` box, window 30 created by the registration; Examine and mini map from the second opening on: `gerenciador::alinhar_cliente_ao_criar`, `exame.json` and `mini_mapa.json` → `alinhamento_ao_criar`).
2. **System colors.** The user's `WIN.INI` has no `[colors]`; Windows then uses the default colors the driver itself carries in the resource `OEMBIN 1` (from byte 0x12, 4 bytes per system color in `COLOR_*` order) [seen in the .DRV files with `tools/ne_info.py`]. `VGA.DRV`: gray desktop (C0C0C0), **dark blue active title (000080)** with white text, **white application background (FFFFFF)**, dark blue highlight. `S3TRIO.DRV` and `VBESVGA.DRV` (configuration 1): desktop A0A0A4, **light blue active title (A4C8F0)** with black text, **cream application background (FFFBF0)**, highlight A4C8F0. That is why the port's opening (measured on DOSBox-X) was light blue and cream, and the game (measured on the VGA bench) dark blue. The same resources have, in the first two fields, the width and height of the scroll bars (17 × 17 on VGA; 22 × 21 in configuration 1 of S3/vbesvga, 17 × 17 in 2001) [seen; which configuration vbesvga uses at 96 dpi was not measured].
3. **An S3 pixel.** On DOSBox-X (S3 Trio), the content of windows with an outline (object 0 type 15) comes out 1 pixel to the right of where it comes out on VGA and vbesvga, with the frame in the same place (bitmap 2500 of Select a Game shows column 0, which on the other two is outside) [measured]. [hypothesis: S3 driver clipping with a negative destination x]. It is not a Windows rule.

**In the port:** `data/resolucao.json` → `perfil_driver` (`"vga"`, the default: standard Windows 3.1 VGA, the most faithful reference; `docs/decisoes.md`) and `perfis_driver` → `alinhamento_x` (8 or 1), `ajuste_x_contorno` (1 only in `"s3_dosbox"`) and `cores_sistema` (replaces indices 16 and 17 of `data/paleta.json`). Code: `engine/src/core/resolucao.rs` (`PerfilDriver`), `Interface::posicionar` and `Interface::centrar_dialogo` (`telas.rs`), `janelas::geometria_janela`, `ConfigTelaJogo::alinhar_edicao`, `ConfigDialogoArquivo::aplicar_driver`, `Avisos::ajuste_janela_imagem`, `engine::paleta::carregar_rgb`. The tests of the screens validated against DOSBox-X (`telas_reais.rs`, `resolucao_reais.rs`) use the `"s3_dosbox"` profile and keep the hashes. **System colors beyond the title and the background (2026-10-19, API 0.32):** the port's palette gained indices 18 (`COLOR_CAPTIONTEXT`, active title text), 19 (`COLOR_HIGHLIGHT`, highlight of menus and lists) and 20 (`COLOR_HIGHLIGHTTEXT`), with the values from each driver's `OEMBIN 1` (VGA: white, 000080, white; S3/vbesvga: black, A4C8F0, black) [seen in the .DRV files]. The data that drew the title, the dialog frames and the highlights with the fixed indices 4 and 15 (the same on VGA) moved to the system indices: title of the edit window and control windows, message boxes and file dialogs, WinHelp, About/Status, Examine, Info popup, press-and-hold menus and window 26 (16 and 18); menu bar, system menu, dialog lists and WinHelp menus (19 and 20). On VGA nothing changes color (test `tela_de_jogo_do_quick2_igual_a_validada` with the new hash only because of the indices); with `"svga256"` the edit window's title matches 100% the SVGA bench boot capture outside the text (ag640_quick; the title text differs because the save is another one) and the frame of window 26 comes out in the title color, as in ag640_fred_1 (test `cores_do_sistema_do_perfil_no_titulo_da_edicao`). The menu highlight in 256 colors follows `OEMBIN`: **checked on 2026-10-19** (SVGA bench 640×480, captures from the same boot, `roteiros/qemu_svga640_256cores.txt`): File open with About highlighted, with New highlighted and Speed open, against `engine/examples/barra_menus` with a mod `{"perfil_driver": "svga256"}` (`--mods`): **100%** of the pixels in all three, at 6 bits per channel, exact colors (`tools/comparar_captura6.py`). **Scroll bars:** the S3/vbesvga `OEMBIN` has 22 × 21, but on the SVGA bench (vbesvga, 96 dpi) the edit window's bars measure 17 wide, as on VGA (ag640_quick: vertical arrow at x 421–436, horizontal at y 463–478 of the screen): nothing to change in the profile. **Opening:** the title's frame uses `interface.json` → `borda.cor` = 16, which follows the profile (dark blue on VGA, checked in the export of `titulo.png`); the "system colors at the opening" divergence predates the profile. Remaining: in 256 colors the system color may change with the game's palette [on the bench only after `snapshot_carregar`, a vbesvga artifact; bancada-qemu.md]. The profile is chosen through data (a mod with `{"perfil_driver": "svga256"}` in `data/resolucao.json`); there is no command-line option.

**Examine and mini map (2026-10-19, API 0.30):** the offset of the session's first opening (the window is created with `CS_BYTEALIGNCLIENT`: 3 pixels in Examine, 1 in the mini map, measured on VGA) uses the profile's same `alinhamento_x` (`JanelaExame::aplicar_driver`, `MiniMapa::aplicar_driver`); in 256 colors there is no offset [code: alignment 1 = 1 byte per pixel; not captured on SVGA]. The `alinhamento_ao_criar` keys of `exame.json` and `mini_mapa.json` became optional (a mod sets them). Test `posicao_e_area_da_bancada` (both profiles).

**Examine and mini map in 256 colors (SVGA bench, 2026-10-19, closing round):** captures in the same boot (vbesvga's 256-color palette does not come back after a `loadvm`): `tools/cenarios_fecha.py svga_examine` (Examine holding the button over the queen's head and tail, the first and second opening) and `roteiros/qemu_svga640_mini.txt` (mini map opened three times). Port with the `svga256` profile (`--mods` with the one-line mod), comparison at 6 bits with exact colors: Examine **98.06%** (first opening) and **98.08%** (second) — the light blue frame (#a4c8f0, the profile's title color) matches and the window sits in the computed place already on the first opening, without offset (byte alignment in 256 colors is 1 pixel, as predicted); the rest is the "Behavior/Egg Laying" text shifted by 1 pixel (the same residue as on VGA) and the cursor. Mini map **99.93%** (first) and **99.97%** (third), after two fixes: (1) **the first opening is also 1 pixel to the left in 256 colors** (155 and then 156, as on VGA), so that −1 is not `CS_BYTEALIGNCLIENT` [cause unknown]; the port moved to a fixed −1 adjustment on the first opening (`mini_mapa.json` → `moldura.ajuste_primeira_abertura`, API 0.35), which gives the same on VGA; (2) with the mini map open the edit window's title becomes **inactive** (white; also on VGA, `m02`): the port drew it active.
