*Leia em português: [abertura.md](../../pt-BR/spec/telas/abertura.md)*

# Opening screen

## Behavior

**Source:** fact seen in the code, `_ShowIntro` (`1000:d3e2`).

1. Opens the internal window `0x300` (768) of the game's own window system (`_win_Open`) and captures the mouse.
2. Plays the music with id 10001 (`_myBeginSong(0x2711, 0x7e)`; the meaning of the second parameter is [open]). In the `SOUND` database, id 10001 exists as a header (type 18) and as MIDI (types 20 and 50).
3. Builds an 8-digit number from a DGROUP table [hypothesis: serial number shown on screen; not used in this milestone].
4. Waits up to **40 × 18 = 720 ticks** (`_DialogWaitInit(0x28)`). The tick of `_TickCount` (`1008:18ac`) is `GetTickCount() / 55`, i.e. about 18.2 per second, which gives about **39.6 s**. **Verified (2026-10-18, `tools/medir_abertura.py`):** the wait stack (`DGROUP:150a`, start at `cad4`, duration at `caf4`) shows the title armed at `_TickCount` 1013 with 720 and the registration at 1733 = 1013 + 720 with 270; in the VM, 719 and 270 BIOS ticks. The logo is drawn before the wait starts. Space closes the title and the registration in ~0.25 s; if pressed before the wait starts, it is lost. Host wall-clock time is no good (QEMU without `icount` loses Windows ticks when the machine is loaded).
5. Closes early if `_win_Events()` returns an event (click or key handled by the window) or if the **space bar** is pressed (`GetAsyncKeyState(VK_SPACE)` in `_DialogAbortOrCont`, `1000:9c98`).
6. Closes window `0x300` and discards pending events.

## Visual content

- Window `0x300` is window 3 (`titleWin`), with the objects `titlePic1` (bitmap 2000) and `titlePic2` (bitmap 2001). Fact seen in the data; see [../formatos/janela.md](../formatos/janela.md).
- **Observed in the original (DOSBox-X, 2026-10-16):** on starting `SIMANTW.EXE`, the client area shows the tool ribbon (window 34) and the bordered title window, with no "Maxis Presents" screen before. The port reproduces this with 99.98% of pixels equal (the difference is the cursor). Capture: `tests/referencias/capturas/titulo_800x600.png`. Above the client area the original already shows the title bar and the menu bar of the main (maximized) window, frozen during the opening; the port draws the menu bar and, with `moldura_windows31`, the title (menus.md §8, 2026-10-19).
- **Open:** where the "Maxis Presents" bitmaps (2/28000, 2/28003) come in.

## Startup sequence

**Source:** `WINMAIN` (`1000:3fda`) calls `_ShowIntro` → `_CustomerIDDialog` → `_NewGame(1)`. Observed in DOSBox-X (2026-10-16), letting the timers run out without touching anything (`tools/capturar_original.ps1 -Espera 80`):

1. **Title** (window 3): up to 720 ticks (≈ 39.6 s), or space, or click.
2. **Registration** (window 30, `picStrWin`, via `_PictureDialog`): the registration text, the registered name, the customer number and "Click here to continue" (name and number of this copy omitted). Closes after 15 × 18 = 270 ticks (≈ 14.9 s), with space or with a click. The fixed texts come from 10/131 ("This copy of SimAnt is registered to:") and 10/132 ("Customer ID:"); the name comes from a string decoded by `_DecodeString` (DGROUP `0x8b3e`); the 8-digit number is the one `_ShowIntro` builds. Fonts: `font3` (blue) and `font2` ("Click here to continue"), validated pixel by pixel (see [../formatos/fonte.md](../formatos/fonte.md)). Capture: `apos_titulo.png`. **Out of the port for now:** the window size and the text positions come from `_PictureDialog`/`_win_DrawPictureWindow`, not yet specified.
3. **Select a Game** (window 2, `scenarioWin`, via `_DoScenario`): stays open until a choice is made. Capture: `novo_jogo.png`. The port reproduces it with 99.87% of pixels equal.

The times are approximate because the tick is 55 ms. In the tests, the sum matches: "Select a Game" was already open 80 s after starting DOSBox-X.

## Game choice (`_DoScenario` `1000:5998`, `_NewGame` `1000:d6dc`)

**Source:** pseudocode [code], names of object 1/2 of `SHARED` [data] and clicks on the QEMU bench [verified].

- `_DoScenario` opens window 2, waits for an object event and returns the object id; `GetAsyncKeyState(VK_RETURN)` returns 0x203 (Full Game); `_DialogAbort` returns 0x205 [code].
- `_NewGame` converts: `scenTutorial` 0x206 → `_CurGameType` 0, `scenQuickGame` 0x202 → 1, `scenFullGame` 0x203 → 2, `scenExperimental` 0x204 → 3; `scenLoad` 0x207 → `_LoadGame` (if it fails, shows the box again); `scenCancel` 0x205 → returns −1 without changing anything [code]. Then, `_RandYard` and the game windows ([../regras/geracao-mundo.md](../regras/geracao-mundo.md) §1–2).
- The same dialog comes back during the game via File → New Game (**Ctrl+N**, `_ProcMenu` case 3) and when closing the game-over dialog (`_EndGameDialog` → `_NewGame(0)`), with the simulation paused (`_SetPause(1)`) [code].
- On the test bench (640×480): Quick Game at (345, 228), Tutorial at (185, 228), Full Game at (280, 290) on screen; Full Game starts on the surface with the winged queen and the ribbon says "You need to dig a nest" (`re_trabalho/modos/full_inicio.png`) [verified].

**In the port:** `engine/src/core/selecao.rs` (`SelecaoDeJogo`: object under the click → `Escolha`), data in `data/modos.json` (object → choice, Enter → Full Game, New Game key). At the opening, the `selecao` screen of `telas.json` no longer closes on any click: only a button (or Enter) chooses, and the game of the chosen type starts (`simant/src/modos.rs`). During a game, Ctrl+N draws window 2 over the game screen (position from table 9/0) and pauses the simulation until the choice. `--tipo tutorial|rapido|completo|experimental` starts a game of that type directly. Load Game opens File → Load during a game and "Load Ant Hill" over "Select a Game" at the opening (Cancel returns to the choice; menus.md §7); Cancel in "Select a Game" at the opening closes the port, like the original (checked on the test bench on 2026-10-19: SimAnt exits and the Program Manager remains; `roteiros/qemu_selecao_cancelar.txt`).

**Verified:** the test `engine/tests/modos_reais.rs` checks that, in position table 0, the points clicked on the test bench fall on the right buttons (Quick, Tutorial, Full, Experimental, Load) and that the center of each button returns its own choice.

## In the port (milestone 1)

The sequence is in `data/telas.json` (data, not code): for each screen, the game windows (and, optionally, loose bitmaps), the duration in ticks and the triggers that end it. Today it only contains the validated opening. A mod can replace bitmaps or layouts by (type, id), or change the sequence.
