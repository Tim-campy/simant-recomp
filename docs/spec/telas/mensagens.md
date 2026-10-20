*Leia em português: [mensagens.md](../../pt-BR/spec/telas/mensagens.md)*

# Notices to the player: ribbon, message boxes and game over

How the original shows the player what the rules request (`Evento::Mensagem`, `Evento::DialogoFimDeJogo`), the short ribbon message, the speed button and saving. Complements [jogo.md](jogo.md) ("Live simulation").

Implementation: `engine/src/core/avisos.rs` (state, drawing and score), `engine/src/core/partida.rs` (when the simulation stops; pause, buttons, advice, click wait), `engine/src/formats/textos.rs` (string lists), executable in `simant/src/main.rs`. Data: `data/mensagens.json` and `data/partida.json` (`botoes_usuario`, `espera_clique_simples_tickcount`, keys `salvar`/`salvar_como`).

**Sources:** Ghidra pseudocode (`re_trabalho/decomp/Code*.c`, reference only; names from `SIMANTW.SYM`), DGROUP strings read from `SIMANTW.EXE` (segment 10) and captures from the QEMU bench (script `tests/referencias/roteiros/qemu_mensagens.txt` and case C-10, [bancada-qemu.md](../../bancada-qemu.md)). Legend as in jogo.md: **[code]**, **[measured]**, **[bench]** (observed in the original), **[hypothesis]**.

## 1. Notice clock

`_TickCount` (`1008:18ac`) = `GetTickCount() / 55` [code]. All deadlines below are in this unit (≈ 55 ms) and run in real time, including with the game paused or with a box on screen. In the port: `Partida::agora()` = real ms passed to `Partida::avancar` / `ms_por_tickcount`.

## 2. Ribbon message (`_EditMessage`, `_DrawRibbonMessage`)

### 2.1 Where and how

- Object `ribMessageArea` (object 31 of window 34, id `0x221f`), rectangle (190, 21)–(384, 35) of the layout, i.e. (190, 4)–(384, 18) of the client area [`WINGANT` data].
- `_DrawRibbonMessage` (`1000:0b26`): if `_TickCount` ≤ deadline and there is text, `_font_SetFont(2)` and `_win_PrintfAtObj(0x221f, text)` — centered text, rest of the object in the background color, with the object's color (entry 12 of the color table: foreground 12, light blue; background 8, gray); otherwise it erases the text and fills the object with the background [code].
- There is no footer in the edit window: `_editMessage` and `_mapMessage` hold the same text; the second is also drawn in the animated yard window (`_DrawAnimYardMessage`, window 25), which the port does not have [code].
- On the bench, "Game Paused" and "Game speed now fast." match the port 100% (see §7) [measured]. For this the port centers the text at `x = (left + right − 1 − width) / 2`, with the layout rectangle: it is the rectangle that `_win_Recalc` writes (right − 1, inclusive coordinates; formatos/janela.md, "Layout calculation"), without the +1 that `_win_PrintfAtObj` adds with a ribbon in the pseudocode. The measured widths ("Game Paused" 68, "Game speed now fast." 110, in `font1`) are even, the case in which the two calculations differ; with an odd width they would give the same [measured rule; the missing +1 remains open].

### 2.2 `_EditMessage(text, duration, important)` (`1000:92c0`)

The request is accepted if [code]:

1. `important` ≠ 0; or
2. the **Messages** option (`_OptionStates[4]`, `8:85fa`) is on **and** the ribbon is empty; or
3. the text is null and the current message is permanent (request to clear).

Accepted: deadline = permanent if the duration is negative (the calls use `0xfffe, 0xffff` = −2), otherwise `_TickCount + duration × 3 / 2`; the text becomes the requested one (null = clears); redraws the ribbon (and the animated yard, if open) [code].

### 2.3 Who writes to the ribbon

| Origin | Text | Duration | Important |
|---|---|---|---|
| `_SetPause(1)` / Speed → Pause, with `_CurGameTool` = −1 | SHARED 4/1010[2] "Game Paused" | permanent | yes |
| same, tool 10 (body exchange) / 11 (target) | 1010[3] "Click on ant to exchange bodies" / 1010[17] "Click on ant to chase or UnPause" | permanent | yes |
| same, other tool | nothing changes | — | — |
| `_SetPause(0)` / unpausing | clears | permanent | yes |
| `_Feedback` (every tick), advice | 1020[0..4] ([conselhos.md](../regras/conselhos.md) §2) | 0x78 → 180 (≈ 9.9 s) | only no. 0 |
| `_Feedback` → `_SetDefaultWindPrompt(0)` | the same pause/tool text, or clears | permanent | no |
| `_DoUserButton` case 6 (speed button) | "Game speed now slow./normal./fast./ultra." (DGROUP `1048:014b`, `0160`, `0177`, `018c`), for the new speed | 0xb4 → 270 (≈ 14.9 s) | yes |
| Speed → Slow/Normal/Fast/Ultra and Shift+1..4 (`_ProcMenu` 0x43–0x46) | **nothing**: they only change `_GameSpeed` and the menu check marks [code; bench `qemu_ritmo.txt`, simulacao-visao-geral.md §3.2] | — | — |
| `_DoBookMark` | "Bookmark has been placed." / "…has not been placed." (`1048:01a2`, `01bc`) | 0xb4 | yes |
| `_EndGameDialog` | clears | permanent | yes |

[code; texts from the database and the DGROUP verified].

Messages requested by the rules (`Evento::MensagemFaixa { lista, indice, duracao, importante }`, no random draw; the values are in the rules' data files) [code]:

| Origin | Text | Duration | Important |
|---|---|---|---|
| `_GstrR` / `_StartAttack` (red attack) | 1020[5] "The red ants are attacking!" | 0x78 | no |
| `_EatMyFood` type 1 (trophallaxis) / 2 (donation) / 3 (egg eaten) | 1010[11] / [12] / [13] | 0x78 / 0x78 / 0xb4 | no |
| `_DoAntMoveY`, passage to the red nest | 1010[4] "You have entered the red nest" | 0x168 | no |

"You have entered the red nest" only comes out when the yellow ant crosses the **passage between the nests** (tile 0x14, from the black nest to the red one). Entering the red nest through the surface hole (`_EnterNest`, `1028:8408`) writes nothing to the ribbon [code]. Since it is not important, it also does not appear if the ribbon already has a message with a deadline (an advice, for example) or if Options → Messages is off (§2.2). The port follows all three rules (`rules/amarela.rs`, test `mensagem_do_ninho_vermelho_so_pela_passagem` in `engine/tests/partida_quick2.rs`).

Bookmarks (`_DoBookMark`, `1000:08a6`, buttons `ribBM1`–`7`): **Ctrl**+click (`GetAsyncKeyState(VK_CONTROL)`: the code pushes 0x11) stores plane, `_MapPnt`, `_MapMode` and `_YardMode` and swaps the button's bitmaps (`0x38dd + i`/`0x38ab + i`), with "Bookmark has been placed."; a click without Ctrl on an empty bookmark plays sound 1 and shows "Bookmark has not been placed."; on a placed bookmark, `_GotoMapPoint(plane, _editWidth/2 + x, _editHeight/2 + y)` [code]. Bookmarks are not in the save; `_NewGame` clears them. **Bench:** Ctrl+click on `ribBM1` gave the message and the bitmaps equal to the port (99.73% on the ribbon; j05) and the click on the empty `ribBM2` gave "Bookmark has not been placed." (j06). The port does not store the yard mode (the yard window belongs to another module).

**Bench** (`m1`–`m4`): after Load the game is paused with "Game Paused"; the speed button replaces it with "Game speed now fast." (it was normal); unpausing (Shift+0) **erases** this message (important request to clear), and ~18 s later the ribbon showed the advice "Your nest needs expansion" [bench]. The 270-unit deadline was not measured directly.

## 3. Message box (`_PictStrnDialog`, `_PictureDialog`)

### 3.1 When it appears

`_PictStrnDialog(image, text, force)` (`1000:615a`): only acts if `force` ≠ 0 or if the **Events** option (`_OptionStates[3]`, `8:85f8`) is on. Then `_StopSimulation` (kills the timer and discards pending `WM_TIMER` messages), reads the list SHARED 4/`text` (`_LoadStringAnt`: one Pascal string per line), shows the box and, on closing, `_RestartSimulation` [code].

The calls with `force` = 0 (depending on Events): 10006/10007 (`_XferPatch`), 10022/10024 (`_MysteryButton`), 10046 (spider dead), 10050 (antlion dead). The others have `force` = 1; when the original tests Events before calling (yellow ant deaths), the port's rule already tests it (`rules/amarela.rs`) [code]. List in `data/mensagens.json` (`nao_forcadas`).

**The simulation stops in the middle of the tick** that requests the box (the call is synchronous). The port finishes the tick and only then shows the box: the state is the same, because the box changes nothing [code]. Several boxes in the same tick (10010 and 10011 of the defeat) appear one after the other. Since 2026-10-19 the tick's requests are presented **in order** (the timeline of `core::partida`, regras/formiga-amarela.md §9.4): whatever the tick requested after a box (another centering, animation, another box) is only presented when it closes, and the edit window shows the snapshot drawn up to that point of the tick (`Evento::Instantaneo`), with the view where the original left it.

### 3.2 Image

The first parameter is a bitmap (type 2) drawn above the text, or 0. Yellow ant deaths (`_YellowDeath`, `1000:7746`): 10034 → 9155, 10036 → 9152, 10038 → 9150, 10040 → 9154, 10042 → 9151, 10045 → 9156, and the lost fight, 10030/10032 → 9156 (the same `push 0x23c4` at `1000:7ab9`, reached from case 0 at `1000:78a1`; checked in the disassembly on 2026-10-19, before that the port showed these two without an image); tutorial lessons: the `imagem` of each lesson in `data/quintal.json` (17000–17009) [code]. No box with an image has been captured yet.

### 3.3 Geometry (`_PictureDialog`, `1000:62bc`, and `_win_DrawPictureWindow`, `1000:61f6`)

With font 4 (`_font_SetFont(4)`, file `font3`, height H = 14), `n` lines, `w = max(50, widest line width)` and `P` = image height + 2 (or 0) [code]:

- requested rectangle: width `w + 8`, height `e = n·H + P + 8`, centered in the main area **below the ribbon** (`GetClientRect` gives 640 × 369 on the bench): `l = (640 − (w + 8)) >> 1`, `t = (369 − e) >> 1` [code + measured];
- window 30 (`picStrWin`) has an outline object 0 (thickness 2) with offsets (−5, −4, +4, +4) over this rectangle, and shrinks by the outline; measured result: **client area at (l + 2, ribbon + t + 3), with (w + 12) × (e + 3)**, and the Windows frame (1 black pixel + 4 in the border color) around it [measured in C-10, 10010 and 10011]; the border color is that of the active Windows title bar: dark blue (index 4) on the QEMU bench, measured on the lesson 14 box (`tutorial_completo`, tick 622; frame strip 100% equal), in `mensagens.json` → `quadro.cor_borda` [measured 2026-10-18; in the DOSBox-X environment the window border is light blue, `interface.json`];
- background: background color of the `picStrArea` object (entry 12: gray); text in the foreground color (light blue);
- image centered at the top of the client area; lines starting 2 pixels below the top (or right below the image + 2), one every H, **centered on the width of the client area** [code + measured];
- "Click here to continue" (object `picStrClick`, font 3 = `font2`, color 9: black on gray) centered in the client area with 1 extra pixel to the right and with the top of the text image 10 pixels above the bottom of the client area [measured].

### 3.4 How it closes

`_DialogWaitInit(0xf)` and the `_PictureDialog` loop [code]:

- click in the area (`picStrArea`, selectable object): closes;
- click outside (event of object 0): `MessageBeep`, continues;
- **Space** (`GetAsyncKeyState(VK_SPACE)` in `_DialogAbortOrCont`): closes;
- **deadline**: 15 × 18 = 270 units of `_TickCount` (≈ 14.85 s) after opening, it closes by itself. **Confirmed on the bench**: the 10010 opened on Load had already been replaced by 10011 18 s later (`m6`) [bench].

The port closes with a left click inside the client area, with Space (`tecla_continuar`) or by the deadline; a click outside only writes "(bip)" (no sound).

### 3.5 The simulation with the box open

Which boxes stop the simulation (disassembly, 2026-10-18) [code]:

- `_PictStrnDialog` (`1000:615a`) calls `_StopSimulation` before opening and `_RestartSimulation` after closing — **always**, whether it comes from inside the tick (tutorial lesson `_GiveLesson`, yellow ant death, advice from `_Feedback`) or from outside (the `_FullCount` boxes on Load). `_StopSimulation` (`1000:0000`) does `KillTimer` and discards pending `WM_TIMER` messages on the first call; the calls nest through a counter (`DGROUP:0x44`), and only the last `_RestartSimulation` redoes the `SetTimer` (17 ms).
- `_EndGameDialog` (GAME OVER) and the boxes of the file commands also kill the timer (`KillTimer` in `_ProcMenu`, commands 0xE/5/6) and redo it at the end.
- `MYTIMERFUNC` only calls `_DoAntSim` with the lock `DGROUP:0x2f4` at 0; it stays at 1 during the whole `_DoAntSim` (the only caller). A box opened **inside the tick**, therefore, never lets another tick run, even if the timer comes back.
- Exception: `WM_ACTIVATEAPP` (message 0x1C in `MAINWNDPROC`, `1000:2930`) redoes the timer without looking at the `_StopSimulation` counter (170 ms when losing focus, 17 ms when gaining it). With a box open **outside** the tick (Load), switching the application's focus would make the simulation run with the box on screen [code; not observed].
- On the bench: on the C-10 Load, `GameTime` stayed at 78 with boxes 10010/10011 open (without a focus change), which matches the code. The tick vector `tutorial/23` (`GameTime` 23 → 40 "with the box open") contradicts the lock: since the box closes by itself in 270 units (≈ 14.85 s) and the bench script only clicks after 15 s without seeing `_DoAntSim` return, the most likely explanation is that the entry tick returned (`GameTime` 24) after the deadline and the read came ticks later [hypothesis to check: record the vector again closing the box before the deadline].

**Port:** with a modal notice on screen (`Avisos::tem_modal`), `Partida::avancar` generates no timer messages (`_StopSimulation`) and, if the box opened in the middle of a batch, discards the pending messages; the tick that requested the box finishes entirely before it (the rules do not wait for the answer), but the rest of its requests wait for the box to close, and no tick runs while the tick's presentation is not over (`Partida::apresentando`; regras/formiga-amarela.md §9.4). A focus change does not restart the simulation with a box open (the exception above is not modeled).

## 4. Game over (`_EndGameDialog`, `1000:6602`)

1. With **Music** on (`_OptionStates[1]`), plays 0x2718 (blacks won, `_BlackWon` ≠ 0) or 0x271a; when the music ends, plays one of 0x2713 + `_SRand2(...)` [code; the port only records the first music, without sound].
2. `_EditMessage` clears the ribbon; opens window 4 (`gameOverWin`, bitmap 16100, position from table 9/0 like the other windows).
3. `_win_DrawEndGameWindow` (`1000:64d4`), with font 4: `gameOverType` = SHARED 4/1001[`_CurGameType`] ("Tutorial Game", "Quick Game", "Full Game", "Experimental"; outside 0..3 uses 2); `gameOverScore` = points (`%ld`); `gameOverRating` = 4/1900[i], with `i` by the points range (÷ 5 in the Full Game): < 214220 → 0, < 428440 → 1, < 642660 → 2, < 856880 → 3, otherwise 4; without `_BlackWon`, `i + 5` [code].
4. Waits for the click on `gameOverPic` (the whole window); it has no deadline [code; bench: 20 s without closing, `m9`].
5. Closes, `_DialogAbortAll`, `_NewGame(0)`: the "Select a Game" box [bench, `m10`].

`_EndGameDialog` is called every tick while `_IsGameOver` ≠ 0 (simulacao-visao-geral.md); since it stays in a modal loop, in practice it appears once. **Bench:** after the C-10 Load (game paused), boxes 10010 and 10011 appeared, but the game over only after a click on the map, which gave an order to the yellow ant and made a tick run even while paused (§4 of simulacao-visao-geral.md) [bench, `m7`/`m8`].

**Port:** the dialog is drawn with window 4; the measured position is 1 pixel to the left of the calculated one (`ajuste` in `data/mensagens.json`) and the texts center without the extra pixel to the right, as on the ribbon [measured]. On closing, the port opens "Select a Game" over the game, like `_NewGame(0)` in the original (`simant/src/modos.rs`).

**After GAME OVER** (`_EndGameDialog` → `_NewGame(0)`, inside the tick in the original): "Select a Game"; the choice generates the new game **without reseeding** S or R — the world comes out of the generators at that instant — and the tick returns with the new game at `GameTime` 0 [bench, vector `inseticida_morte/21` re-recorded]. Port: the game requests the dialog once; the interface shows the boxes in order and, once GAME OVER is closed, opens "Select a Game" (`Modal::abre_selecao_de_jogo`); the new game comes from `carregar::jogo_novo_com_geradores` (also on Ctrl+N), checked 307/307 with S and R against the vector.

### 4.1 Score (`_CalcScore`, `1000:5530`)

Eight partials `p[0..7]` (0 to 100) [code]:

| p | Calculation | No data |
|---|---|---|
| 0 | mean of `_H_BHeal` over the `_HistCnt` samples of the ring (from `_HistStart − _HistCnt`) | 0 |
| 1 | Σ`_H_BFood` × 100 / (Σ`_H_BFood` + Σ`_H_RFood`) over the same samples | 0 |
| 2 | (`_TotalEggsLaidB` − `_TotalEggsDiedB`) × 100 / `_TotalEggsLaidB` (32 bits) | 100 |
| 3 | `_BlackLost` × 100 / (`_BlackLost` + `_RedLost`) | 100 |
| 4 | Full/Experimental only: `_BColoniesStarted` × 100 / (`_BColoniesStarted` + `_RColoniesStarted`) | 100 |
| 5 | same: `_BColoniesStarted` × 100 / (`_BColoniesStarted` + `_BColoniesKilled`) | 100 |
| 6 | same: yard patches with `_YMapPopB` ≠ 0, × 100 / 155 (x ≥ 3 for y < 5, x ≥ 2 for y ≥ 5, x < 12; index x × 16 + y) | 0 |
| 7 | same: house patches (the rest: 37) with `_YMapPopB` ≠ 0, × 100 / 37 | 0 |

`sum = _MeHealth + Σ weight[i] × p[i] × 51`, with the DGROUP weights `1048:0948` = 13, 17, 19, 23, 29, 31, 37, 41. Full Game: if `_GameTime` < 8100, `sum = max(1, _GameTime / 100) × sum / 81`; then `sum × 5`. Other types: `sum = sum × 29 / 10`; if `_GameTime` < 4100, `sum = max(1, _GameTime / 100) × sum / 41`. **Points = `_GameTime` + sum** [code; signed 32-bit arithmetic: the pseudocode shows some of it only in the low word, port hypothesis].

**Return value and partial 3 checked (bench, 2026-10-19, `tools/cenarios_fecha.py gravar pontuacao`, snapshots `rico` and `full`):** seven game overs provoked by writing `_GameTime`, `_BlackLost`, `_RedLost`, `_MeHealth` and `_IsGameOver` = 1 at the entry of a tick (the end of `_DoAntSim` opens GAME OVER); the state read with the box open gives in the port exactly the points and the rating on screen (`engine/tests/pontuacao_original.rs`, states `FIM_*.ANT`):

| Case | Type | `_GameTime` | Blacks/reds lost (partial 3) | `_MeHealth` | Points | Rating |
|---|---|---|---|---|---|---|
| `q_tempo_longo` | Quick | 5001 | 30/10 (75) | 50 | 902307 | Llama Repair Specialist |
| `q_tempo_curto` | Quick | 2001 (< 4100) | 5/45 (10) | 80 | 331894 | Weak Link in the Food Chain |
| `q_sem_baixas_pretas` | Quick | 4101 | 0/25 (0) | 100 | 646424 | Aphid Herder |
| `q_32bits` | Quick | 30001 | 3/1 (75) | 1 | 927165 | Llama Repair Specialist |
| `f_tempo_longo` | Full | 9001 | 12/36 (25) | 70 | 1800726 | Weak Link in the Food Chain |
| `f_tempo_curto` | Full | 3001 (< 8100) | 40/0 (100) | 100 | 829571 | Aardvark Fodder |
| `f_8100` | Full | 8100 | 7/7 (50) | 33 | 1946265 | Weak Link in the Food Chain |

Confirmed: the **return value is `_GameTime` + sum in 32 bits** (points well above 65535; Ghidra's "`_GameTime` + total" was right); the two time branches (Quick × 29 / 10 and, below 4100, × max(1, t/100) / 41; Full × max(1, t/100) / 81 below 8100, then × 5); and the **direction of partial 3**: `_BlackLost` × 100 / (`_BlackLost` + `_RedLost`), so that with no black lost the partial is 0 and losing more blacks increases it (the "Fights Won" column of Status shows this calculation; [hypothesis] a defect of the original, which the port follows). The rating on GAME OVER is the points range (÷ 5 in Full) plus 5 (the blacks did not win).

**Bench:** C-10 gave "Quick Game", **19785**, "Aardvark Fodder" (`m8`). The port, with the C-10 state and `_GameTime` = 79 (one tick after Load, the one of the click order), gives exactly 19785 (`cargo run -p engine --example pontuacao -- C10.ANT 79`) [bench].

## 5. Configurable ribbon buttons (`_DoUserButton`, `1000:04c4`)

The objects `ribUserDef1A`–`1H` (ids `0x2210`–`0x2217`) call `_DoUserButton(id, index)`; each button's function comes from the table `1048:0012` (default: 15, 0, 1, 3, 7, 6, 5, 10; `WIN.INI` can change it via `_SetUserButton`) [code + DGROUP]. Single click:

| Function | Effect |
|---|---|
| 0, 1, 2, 3 | opens the Behavior, Caste, History or Info window (port: Behavior, Caste and History in janelas.md; Info in [info.md](info.md)) |
| 4 | toggles the map trails (port: `Partida::mostrar_trilhas`, used by the map window, [mapa.md](mapa.md)) |
| 5 | `_SetPause(!_GamePaused)` (§2.3) |
| 6 | `_GameSpeed` += 1 (3 wraps to 0) and the message "Game speed now …" (§2.3) |
| 7, 9, 10, 11, 12, 13 | toggle `_OptionStates[1]` (Music), `[2]` (Sound Effects), `[5]` (Silly), `[0]` (AutoTrack), `[3]` (Events), `[4]` (Messages) |
| 8 | `_MysteryButton` (port: `rules/misterio.rs`; actions `misterio_*` in `tests/referencias/acoes/`) |
| 14 | `_SaveGame(1)` (§6) |
| 15 | opens the edit window and `_GotoMyAnt` |

With **Ctrl** pressed (`GetAsyncKeyState(VK_CONTROL)`; the code pushes 0x11 — an earlier version of this document said "held"), opens window 36 (`iconMenu`, bitmap 7200, 16 icons) to choose the button's function; with Shift, it only changes this button, without Shift the two equivalent buttons of ribbons 34 and 35 [code].

Ribbon 35 (yard) has its own: `ribUserDef2A`–`2H` (ids `0x230b`–`0x2312`, objects 11–18 of window 35), handled by `_ProcYardRibbonEvent` with `_DoUserButton(id, id − 0x230b)`; the table is `_yardUserButton` (`1048:0022`, default the same as ribbon 34: 15, 0, 1, 3, 7, 6, 5, 10; `WIN.INI` `yarduserbuttonN`). `_DoUserButton` chooses the table by the high byte of the id (0x22 → `1048:0012`, otherwise `1048:0022`) and, without Shift, writes the function to both tables and calls `_SetUserButton` for both ids (id ± 0xFB) [code]. **Port** (2026-10-18): `partida.json` → `botoes_usuario_quintal`, `Partida::funcoes_botoes_quintal`; click, Ctrl+click (icon menu) and the drawn state (bitmaps and on) like those of ribbon 34; preferences `botoes_quintal`. Test `botoes_da_faixa_e_menu_da_amarela`. **Bench (2026-10-19, `roteiros/qemu_faixa35_botoes.txt`, captures `re_trabalho/raros/faixa35/`):** with the Yard View in front, Ctrl+click on the 1st configurable button of ribbon 35 opened the icon menu (window 36) and releasing over MUSIC changed the function; the button started showing the musical note **on both ribbons** (35 and, after Ctrl+E, 34), like `_DoUserButton` without Shift. The row of configurable buttons of ribbon 35 is equal to the port (`--save QUICK2.ANT --quintal 0` with `--win-ini` giving `yarduserbutton0=7`) in 100% of the pixels. With the game paused right after Ctrl+Y, the original left Queens, the colonies and the health bars empty (divergencias.md, 2026-10-19). The menu and the drawn state of the buttons are in [janelas.md](janelas.md) §4.4 and §5.

**Bench:** the 6th button (`ribUserDef1F`, at (159, 96) of the screen) showed "Game speed now fast." starting from normal (`m2`) [bench].

## 6. Saving (`_SaveGame`, `1010:a23e`)

- File → Save (command 5, **Ctrl+S**) calls `_SaveGame(1)`: if there is already a file name (`1038:9520`), writes to it; otherwise (and in Save As, command 6, **Ctrl+A**) asks for the name in the Windows common dialog (`_FileSelect`) [code]. The file commands go through `_fileWaitFlag` and are handled outside the timer.
- Checks the free space, opens/creates the file and writes the blocks of the table `DS(8):0x9570` in order ([save-ant.md](../formatos/save-ant.md)); then `_PopMsg("%s Saved correctly")` (Windows message box, as in `qi_14_salvo.png`) [code + capture].
- **Port:** `Estado::para_save` writes the 307 blocks (the round trip is byte for byte, `estado_simulacao.rs`). Ctrl+S writes to the `--save` file (or to the last saved one); without a file, and on Ctrl+A, to the name from `--salvar`, or to the first free `SALVOnn.ANT` in the current folder. Since 2026-10-18, without a file and on Ctrl+A it opens the "Save Ant Hill As" dialog and, after writing, the "Message" box with "… Saved correctly" ([arquivos.md](arquivos.md)); `--salvar` skips the dialog. Without a window, `--salvar ARQ` writes the state after the `--ticks`.
- **Bench:** `PORT50.ANT` (QUICK2 + 50 ticks in the port, `--salvar`) was loaded by the original without error; the state read right after Load differs from the file only in `_GamePaused` (Load pauses), `_CastePopR` (recounted on Load, as with `QUICK2.ANT`) and `_MapPnt` (21 → 22: the view that Load computes; jogo.md, "Position on load") [bench, `port_carregado.ANT`].

### 6.1 What Load does besides reading the blocks (`_LoadGame` `1010:9fa0`, `_DoLoadInitializations` `1010:9dbe`)

- `_FullCount` (`_CountAnts` + `_Feedback`): takes a census of the castes — that is why `_CastePopB/R` change on Load (bancada-qemu.md) — and already detects the queen that disappeared: in case C-10, boxes 10010 and 10011 appear **right on Load**, with `_GameTime` = 78 [code + bench]. The boxes are modal and appear before the windows are redrawn (the capture shows the previous game behind the box).
- `_SetDefaultWindows`, `_CenterEdit` on the yellow ant, `_SetDefaultWindPrompt(1)` and, after everything, `_PauseGame(1)`: the loaded game stays paused with "Game Paused" — only after the `_FullCount` boxes close (the ribbon is empty with the box on screen) [code + bench m5/m7].
- Before reading: `_RandYard` (consumes random draws; rng.md), `9:9c70` = 1, `_TimeTemp` = 300 and three more variables cleared [code; the port does not reproduce those outside the save].
- Port: `Partida::ao_carregar()` (called by the executable on `--save`): `contar_formigas`, events, advice and the pause, postponed until the boxes close. The tick vectors (72/72) do not go through here.
- View: the original centers the yellow ant at (34, 9) with `_MapPnt.x` = 22, both when loading and when creating the QUICK2 Quick Game: `_editWidth` = 25, not 26. The port switched to 25 columns (`ajuste_area_com_faixa` = 0 in `data/tela_jogo.json`); with that, the edit window after Load matches capture m7 at 99.94% (only the cursor) [bench].

## 7. Single-click wait (`_processEdit`, `1000:6a38`)

On a single (not double) click with the edit window in front, the original stays in a loop until `_TickCount` = start + 6 (≈ 330 ms), without handling messages: the simulation does not advance. Then, if the button is still pressed (`_myButton`) and Shift is released, it opens `_AntMenu` (click on the yellow ant itself) or `_MagnifyMenu` (Examine, on another cell); if the menu returns a choice, the order is not given [code]. The port stops the simulation for the same 330 ms (`espera_clique_simples_tickcount` × 55 ms) and decides at the end of them, with the button and Shift of that instant: yellow ant menu (janelas.md §4.2), Examine (without window 29, a gap) or the order.

## 8. Verification (2026-10-17)

Pixel-by-pixel comparison with `tools/comparar_captura.py --regiao … --mapear` (colors mapped by index, see jogo.md), port exported with `simant --save … --ticks/--botoes/--fechar-avisos … --exportar-tela-jogo`:

| Notice | Capture | Region (client area) | Equal |
|---|---|---|---|
| box 10010 (1 line) | `C-10/t3_carregado.png` and `mensagens/m5` | (186, 244)–(462, 279) | **100%** |
| box 10011 (2 lines) | `mensagens/m6_apos_18s.png` | (201, 237)–(446, 286) | **100%** |
| ribbon "Game Paused" | `mensagens/m1_port_carregado.png` | (190, 4)–(384, 18) | **100%** |
| ribbon "Game speed now fast." | `mensagens/m2_velocidade.png` | (190, 4)–(384, 18) | **100%** |
| game over (window 4) | `mensagens/m8_apos_clique2.png` | (128, 128)–(464, 346) | 97.95% (points from another `_GameTime`: 19787 versus 19785; the rating line 1 pixel lower; the mouse cursor) |

Tests: `core/avisos.rs` (deadline and rules of `_EditMessage`, pause, speed, forced boxes, 270 deadline, images, rating, score), `engine/tests/partida_quick2.rs` (save and reload, boxes stop the simulation until closed, speed button, click wait).

## 9. Ribbon bars and numbers (`_DrawMapData`, `1010:cb60`)

On every `_UpdateWindows` (and with the map: `_DrawMap`), with the ribbon open [code]:

- **Numbers** (`_win_PrintfAtObj` with "%d", font 3, object colors): `ribBlacks` = `_BpopT`, `ribReds` = `_RpopT`. Centered in the object, like the ribbon message.
- **Bars** (`_win_DrawHBar(object, fraction in 1/65536)`): `ribYellowHealth` = `_MeHealth`/100; `ribBlackHealth` = `_HealthB`/100; `ribBlackP` = `_BpopT`/m; `ribRedHealth` = `_HealthR`/100, or 0 if `_RpopT` = 0; `ribRedP` = `_RpopT`/m; with m = max(1, `_BpopT`, `_RpopT`). The colors come from `_win_SetColorFromObjNum`: the object's `cor` entry in the color table (WINGANT 0/129), foreground up to the fraction and background in the rest (yellow: color 4; blacks: 9; reds: 10).
- **Marks** (`_GInvBox`, inverts a column): on `ribBlackHealth` at `_BlkWarnHealth` and on `ribYellowHealth` at `_MeWarnHealth` (both are 30 in a new game), at column `left + ⌊width × threshold / 100⌋`.
- `_ConvColor` only changes the color with `_displayType` = 10 (DGROUP table `0x7238`); it is not called here (the ribbon message background uses `_ConvColor(0xc)`).

**Measured** geometry (bench, script `qemu_barras_faixa.txt`: three captures with the VM paused and the seven values read right after): the bars paint columns `left..right − 1` of the object (64 pixels; column `right` is the frame of the ribbon bitmap) and rows `top..bottom − 1` (5 or 8); the division uses width 63: `s = left + ⌊63 × fraction / 65536⌋`, foreground in `left..s − 1` and background in `s..right − 1` (full: all foreground). The 15 bars of the three captures and the two marks match this rule; the pseudocode (`right + 1` with `_ribbonBarHeight` ≠ 0, and the layout rectangle) would give 65; the measurement is that of the `_win_Recalc` rectangle (right − 1) without this +1, the same rule as the ribbon text and the triangle (formatos/janela.md, "Layout calculation"). The port puts the values in `data/tela_jogo.json` (`barras_faixa`, `encolher` = 1) and draws in `core/barras_faixa.rs`. **Checked:** with `QUICK3.ANT`, the region of the bars and numbers (x ≥ 512, screen rows 38–110) is equal to the capture `qs_salvo.png` of the same instant, except the exact color of the mark on the yellow bar (dark blue of the port's palette, (0, 0, 128), versus (0, 0, 168) of the QEMU VGA).

**Ribbon 35 right after the switch (bench, 2026-10-19, `qemu_faixa35_botoes.txt`):** with the game paused before Ctrl+Y, the fields of `_DrawYardData` (Queens, black and red colonies) stay empty: the switch from ribbon 34 to 35 comes after the data is drawn, and with the simulation stopped nothing redraws them; with the game running, they appear on the next tick (`a51/mate_quick`). **Port (2026-10-19):** `JanelaQuintal::faixa_sem_dados` stores the `GameTime` of the switch and `dados_da_faixa` only releases the numbers and bars after one tick; test `numeros_da_faixa_35_so_depois_de_um_tick`; right half of the ribbon 100% against `f4_depois.png`. **Only through Ctrl+Y/5/6/7** (bench `re_trabalho/agyard/b_faixa35`): the Yard button, the click on the yard and Ctrl+M draw the data immediately, even when paused (telas/quintal.md §3).

### 9.1 Click on the bars area (`mapRibCaste`, `yardRibCaste`)

The bars area is a ribbon object (`mapRibCaste`, object 39 of window 34, id `0x2227`; `yardRibCaste`, object 36 of 35, id `0x2324`) that generates an event when clicked. `_ProcMapRibbonEvent` and `_ProcYardRibbonEvent` [code]:

- Point inside the yellow ant's health bar (`0x2222` `ribYellowHealth` / `0x231F` `ribYellowH2`): `_DoHealthSetY` writes `_MeWarnHealth = (x − left) × 100 / (right − left)`.
- Inside the black colony's health bar (`0x2223` `ribBlackHealth` / `0x2320` `ribBlackH2`): `_DoWarnSetB` writes `_BlkWarnHealth` with the same calculation.
- At any other point of the area (population numbers, population bars, red health bar, labels): `_DrawCastePopUp` (`1000:5d9e`).

The two thresholds (30 in a new game, `_InitSimVars` `1030:5a70`) are the inverted marks of the bars (§9) and decide the health warnings (`_MeWarnHealth`: the yellow ant wants to eat; `_BlkWarnHealth`: colony warning). The calculation uses the rectangles `ribYellowH2`/`ribBlackH2` of **ribbon 35** on both ribbons (in the pseudocode, `_win_GetObjRect(0x231F/0x2320)`), with left = 567 and right = 631 [code + bench]. **Bench** (`qemu_ui_barras_direito.txt`, 2026-10-18, snapshot `quick` paused): click at x = 580 of the black bar → `_BlkWarnHealth` 20; x = 620 of the yellow one → `_MeWarnHealth` 82; pressing at x = 590 → 35 and dragging to 610 with the button pressed does **not** change it (only the click counts).

`_DrawCastePopUp` [code + bench]: saves `_GamePaused`, `_SetPause(1)`, `_LessonTemp = 1`, opens window 23 (`antPopWin`: bitmap 14002 with the labels Larvae, Workers, Soldiers, Males, New Queens, Queens and the figures) against the right edge, below the ribbon, and writes, with font 2, for each caste i = 0..5 the `_CastePopB[i]` (color of object `antPopBlack`) and, one line below, the `_CastePopR[i]` (`antPopRed`), in the left column of `antPopGraphArea`; for i ≥ 1, bars of length `value × L / m` (L = right of the area − (left + 4 × width of the widest digit + 2), m = largest number of castes 1..5 of both colors, at least 1). The lines are spread over the height of the area: leftover = height − 12 × font height + 3, one fifth between the castes and the rest distributed 1 pixel at a time; the last one goes up 1 pixel. The box stays open while the button is pressed (`_ButtonHeld`) and closes on release, restoring the previous pause.

Measured geometry (capture `b01_ribBlacks.png`): black outline at (451, 76) of the client area, 189 × 149, 1 black line and 4 dark blue ones; the content starts 2 pixels before the inner corner of the frame and is clipped by it on the right and bottom; the right and bottom of the graph area count 1 less than the layout rectangle. **Port** (`core/popup_castas.rs`, `data/popup_castas.json`; `simant/src/modos.rs`): same rules; the simulation does not advance with the box open. Compared with `b01_ribBlacks.png` (same numbers): **99.98%** of the box's pixels equal (the difference is the cursor). Test: `barras_da_faixa_como_no_original` (simant).

**Not verified:** boxes with an image (yellow ant deaths, tutorial), the 270 deadline of the ribbon, game over with victory and in the Full Game, the score with partials 4–7, other resolutions (800×600).

## 10. Yellow ant box (`_YellowDialog`, `1000:8262`)

Called with a bitmap and a text index [code]: by `_EatMyFood(1)` (trophallaxis, `0x2396` = 9110, index −1), by `_YellowDeath` for causes 1 and 2 (spider and antlion, `0x238c` = 9100, index 0) and by `_YellowBirth` (`0x238d` = 9101, index `param + 1`). Only with Options → Events (`_OptionStates[3]`) ≠ 0. Opens window 26 (`dialogPicWin`) with the bitmap in object 1 (`dialogPic`), captures the mouse and:

- **9110 (trophallaxis):** `_myBeginSound(0x2a)`, `_myDelay(0x2d)` (45 units of `_TickCount`; an event or Space ends the wait earlier), with the window still open draws bitmap 9111 at `_YellowEatPnt` (`8:896c` = (72, 62)) from the corner of object 1, `_myBeginSound(0x2d)`, `_myDelay(0x2d)`.
- **9101:** writes, in font 4, string `23 + index` of list 1010 ("You have been reborn!", "You are now a worker!") in object 2 (`dialogText`), and `_myDelay(300)`.
- **9100:** the text only in the monochrome modes; `_myDelay(300)`.

Then, with no pending event, `_DialogWaitInit(3)` (54 units) until the deadline, Space or an event; closes the window and redraws everything. All of this happens **inside the tick** (the simulation stays stopped).

**Bench (2026-10-18, scenario `trofalaxia` of `tools/cenarios_tick.py`, snapshot `rico`):** yellow ant with `_MeHealth` = 10 and a double click on a black worker on the surface; on touching, the box appears: dark blue 4-pixel outline from (156, 159) to (361, 326) of the client area (window of 206 × 168), bitmap at (160, 163) clipped to 198 × 160 (the bitmap is 199 × 161), and 9111 overlaid; `_MeHealth` goes to 100 after the box. The port (`Modal::Amarela`, `core/avisos.rs`, data in `mensagens.json` → `quadro_amarela`; test `quadro_da_trofalaxia_como_no_original`) draws the box region with the overlay **100%** equal to the captures `trof_029`–`031`; capture `trof_028` caught 9111 halfway (the top lines already equal). In the port, a click or key closes the box at once (in the original, the event ends the `_myDelay` and skips the final wait) [approximation].

**`_myDelay` returns right away (bench, 2026-10-18).** `_myDelay(n)` (`1000:9620`) waits `n` units of `_TickCount`, but exits earlier on an event, on Space or when the deadline of the current level of the `_DialogWaitInit` stack expires (`1048:150a`, deadlines at `1048:cad4`/`caf4`) [code]. Outside a `_DialogWaitInit` the level is −1, and the test reads the entry before the table: on the bench, none of these `_myDelay` shows. The trophallaxis stayed ~2 s on screen (versus 45 + 45 + 54 units ≈ 7.9 s from the code) and boxes 9100 and 9101 ~3 s each, only the `_DialogWaitInit(3)` = 54 units (≈ 3 s), versus 300 + 54 units [hypothesis supported by the bench and the code: the deadline of level −1 has already expired]. The port uses `espera_tickcount` = 0 in the three sequences: the overlaid trophallaxis image appears together with the window, and sounds 42 and 45 come out together (45 cuts 42).

**Boxes 9100 and 9101 (bench, scenarios `morte_aranha` and `morte_leao` of `tools/cenarios_tick.py`, 2026-10-18).** Death by the spider: after the fight, 9100 ("Your ant has been killed", with a shadow: it is part of the bitmap, because the text is only written in the monochrome modes) and, after the egg in the nest, 9101 with "You have been reborn!" in yellow (color 13 of the color table, that of the `dialogText` object: yellow on black) centered at the top of the bitmap, glyphs from (183, 164) to (329, 172) of the client area. The two windows are **2 pixels to the left** of the others of the same size (outline from (154, 159) to (360, 326); 9200, also 200 × 161, is at 156) **Cause (2026-10-19):** the byte alignment of the client area only happens on the window's **creation** ([resolucao.md](resolucao.md) §7): `_win_Open` (`1030:ca2e`) calls `CreateWindow` only when `_win_hwnd` is still 0; `_win_Close` (`1030:cf98`) only hides (`ShowWindow(0)`), and reopening a hidden window uses `SetWindowPos` at the requested x (158), without aligning [code]. In both scenarios the spider or antlion animation created window 26 first (outline at 156: x = 160, aligned), and 9100/9101 reopen it at 158 (measured: `ar_001`–`ar_013` with the inner outline at 156, `ar_034` onwards at 154; `le_001` 156, `le_009` 154). The trophallaxis and the victory measured were the first opening of the session (160). **Port:** `Avisos` stores whether window 26 has already been created in the session and decides, when each window-26 notice comes to the front, whether it creates it (aligns) or reopens it (without aligning) (`Avisos::alinhar_frente`); the window the spider leaves on screen keeps its place. The `ajuste` (−2, 0) of `mensagens.json` was removed; 9100 against `ar_036` and `le_010` **100%** in the window, without adjustment. The port requests 9100 for causes 1 and 2 (after the fight), 9101 at the end of `_YellowBirth` (index 1 when reborn from an egg, `rules/amarela.rs::nascer_de_ovo`; 2 when founding a colony, `fundar_colonia`, not checked on the bench) with `Evento::QuadroAmarela { imagem, indice }`; ids in `amarela.json` → `quadros` (optional, API 0.25). Comparison (`simant` test `janela_26_como_no_original` with `SIMANT_SAIDA_TESTE`, `tools/comparar_captura.py --mapear --regiao` on the window with the outer line): 9100 against `le_010` **100%**, 9101 against `le_018` **100%**, trophallaxis against `trof_030` **100%** (now with the black outer line, which the first measurement did not see because the background was black).

**Geometry with other bitmaps (bench).** Window 26 in the database has the `dialogPic` object of 1 × 1 at (153, 85); `_win_SetObjBitmap` swaps the bitmap and the window opens at its size [code]. Measured with bitmaps 9110 (199 × 161), 9200 (200 × 161), 12000 (189 × 171) and 16200 (254 × 199): the bitmap's corner is always at (160, 163) of the client area, the area shown is the bitmap minus 1 pixel on each axis, with the 4-pixel dark blue outline (color 4) and **1 black pixel outside** (`avisos::geometria_janela_imagem`; `cor_externa`). With another window on top (the spider's text box), the outline is **white** (`cor_borda_inativa`, color 15), like an inactive window frame.

## 11. Animated boxes of window 26 (`_SpiderDialog`, `_LionDialog`, `_DrawSimPayoff`)

All three use the same window 26 (`dialogPicWin`) as `_YellowDialog`, with another bitmap in the `dialogPic` object, and a loop that swaps frames drawn on top, in real time, without running the simulation [code]. Clocks: `_TickCount` (55 ms) and `_MacTickCount` = `_TickCount` × 3 (steps of 3; regras/quintal.md, "Clocks"). A deadline of *n* Mac units expires on the first `_TickCount` at which the Mac clock has reached it, i.e. ⌈*n*/3⌉ units of `_TickCount` later.

### 11.1 `_SpiderDialog` (`1000:673a`): the spider eats the yellow ant

Called by `_YellowDeath(1)` only with Options → Events (`_OptionStates[3]`) [code]:

1. Opens window 26 with bitmap 12000 (189 × 171) and captures the mouse; `_myBeginSound(0x2d)` (45); `_DialogWaitInit(8)` = 144 units of `_TickCount`.
2. Loop until the deadline, a click or Space (`_DialogAbortOrCont`, `_win_Events`): every 8 Mac units, the frame counter advances (0–3, wrapping to 0 with a new sound 45) and frame `12001 + table[counter]` (98 × 80) is drawn at (+46, +77) from the bitmap's corner. The table is at `8:87fc` = (0, 1, 2, 1) [read from `SIMANTW.EXE` and from memory on the bench]: the frames come out 12002, 12003, 12002, 12001 (sound), 12002… The first comes out 8 Mac units (3 of `_TickCount`) after opening.
3. End of the loop: the last frame again; **a random draw `_SRand1(2)`** chooses the text: 0 → list 10047 ("Your guts are dissolved and…"), otherwise 10048 ("You writhe in burning agony…"); `_StopSimulation`, `_PictureDialog` with the lines of the text, without image and forced (the message box of §3), `_RestartSimulation`.
4. **Window 26 stays on screen**, with the last frame, under the text and during the fight (`_AnimYellowFight`), until `_YellowDialog(9100)` reopens it with bitmap 9100 and closes it [bench]. Correction (2026-10-19): `_SpiderDialog` does end with `_win_Close` (`1000:693f`), which only hides the window; the image that remains is the one nothing redrew over [hypothesis], and 9100 reopens the hidden window at 158 (above).

The `_SRand1(2)` belongs to the S generator and happens **inside the tick**, before the `_SRand1(5)` of the death music (rng.md). In the port it is done by the rules (`rules/amarela.rs::matar_amarela`), which request `Evento::DialogoAranha { texto }` with the text already drawn (ids in `amarela.json` → `mensagens.aranha_comeu`). Without Events, neither box nor random draw, as in the original.

**Bench (scenario `morte_aranha`, snapshot `rico`, 2026-10-18):** yellow ant on the surface and the spider one cell below, in the eating state with target = yellow ant. Sequence on the host clock: spider animation from 1.0 to 8.7 s (the `_DialogWaitInit(8)` = 7.9 s), text box 10048 from 9.4 to 23.2 s (closed by the 14.85 s deadline, with the last spider frame behind and its outline white), the fight (too fast for the captures), 9100 from 23.95 to 26.9 s, the egg in the nest and 9101 from 29.6 to 32.4 s. The difference between two captures of the loop is entirely within (206, 242)–(301, 317): the frame at +46/+77. Port against the captures: spider window with frame 12002 against `ar_001` **100%**, inactive window under the text against `ar_020` **99.98%** (a line at the edge of the text box).

### 11.2 `_LionDialog` (`1000:80a0`): the antlion eats the yellow ant

Called by `_YellowDeath(2)` only with Events [code]:

1. Opens window 26 with bitmap 9200 (200 × 161); `_myBeginSound(0x26)` (38) and **waits for the sound to end** in a loop without events (`_mySoundIsDone`), which in multimedia mode returns true immediately (regras/som.md): **no wait**. Measured in the session recorded in DOSBox-X (2026-10-19): the first 0x25 comes out 562 ms after 0x26, the normal interval of the loop (30 Mac units); until 2026-10-19 the port waited 703 ms (the 2880 samples of 0x26 at 4096 Hz) before the loop (`espera_som_ms`, now 0).
2. Loop for 300 Mac units (≈ 5.5 s), or until an event: every 30 Mac units (10 of `_TickCount`), draws frame `9200 + n` with n = 1, 2, 3, 2, 3, 2, 3… at the bitmap's corner, with `_myBeginSound(0x25)` (37) when n is odd. Outside color mode (`_displayType` ≠ 10) the frames get the offsets of the table `8:895c` = ((0, 0), (38, 39), (38, 52), (38, 52)); in color mode, none [code; table read on the bench].
3. Closes the window and redraws everything. That is 9 frames in 100 units of `_TickCount` after the sound.

There is no random draw. In the port: `Evento::QuadroFormigaLeao`.

**Bench (scenario `morte_leao`):** antlion 0 placed on the yellow ant's cell (on the surface). Antlion window from 1.06 to 5.95 s (300 Mac units = 5.5 s), 9100 from 6.7 to 9.5 s, egg, 9101 from 12.3 to 15.0 s. Port against `le_001` **100%**.

### 11.3 `_DrawSimPayoff` (`1000:cda8`): victory

Called by the Full Game victory (end of `_SimColonies`, after music 0x4e25 and box 10057, with `_IsGameOver` = 1 and `_BlackWon` = 1; regras/quintal.md) and by the `fred` cheat (`_CheatKeys` case 0x14; regras/trapacas.md) [code]. It does not look at Events.

1. `_MapToYard` (the yard takes the place of the map, or comes to the front), `_SetYardMode(0)` and `_SetMapPlane(0)` if needed, `_DrawYard` and `_myDelay(0x96)` (returns right away: §10).
2. Opens window 26 with bitmap 16200 (254 × 199).
3. Loop until 600 Mac units or `_DialogWaitInit(10)` = 180 units of `_TickCount` (whichever expires first: 180 ≈ 9.9 s), or a click/Space: requests musics 20003 and 20004 in sequence (`_mySongIsDone` is immediate in the Windows MCI, so the second cuts the first on the next pass; regras/som.md); every 8 Mac units draws frame `16210 + k` (13 × 13: the spiral eyes), with k = 1, 2, 0, 1, 2…, in six positions from the bitmap's corner, from the table `1048:15d8` ((y, x) pairs) [read from `SIMANTW.EXE` and from memory]: (72, 74), (87, 66), (175, 64), (191, 66), (159, 122), (179, 124) in (x, y).
4. `_myDelay(300)` (returns right away) and closes the window.

**Bench (scenario `payoff`, `fred` cheat typed in snapshot `rico`):** the window appears in less than 1 s, stays ~10 s (the `_DialogWaitInit(10)`) and disappears; `GameTime` does not advance. During the window the edit goes from the nest to the surface, the map becomes the yard (in the same place) and the ribbon becomes the yard one (35); on closing, the ribbon goes back to the edit one (34) and the edit stays on the surface. The difference between two captures of the loop falls exactly on the six positions. Port against `pay_001`: **99.78%** (the rest is the mouse cursor). The music was not heard (the bench has no sound card).

In the port: `Evento::AnimacaoVitoria`; the interface applies Ctrl+Y (yard mode 0, plane 0, yard in front, `modos::aplicar_acao_quintal`) and shows the box; on closing, the ribbon goes back to 34.

### 11.4 Port

`engine/src/core/quadros_animados.rs` (script: phases before, sound, loop and final, on the game's `_TickCount`), `core/avisos.rs` (`Modal::Animado`, drawing, the window the spider leaves open), data in `mensagens.json` → `quadros_animados` (bitmaps, frames, positions, intervals, sounds and music; optional, API 0.25). The simulation stays stopped while the box is on screen (like the other notices). A click anywhere or a key ends the current wait (`Animado::pular`); the antlion's sound wait is not interrupted. **Order with the fight:** in the same tick, the spider (or antlion) box and the spider text come before the fight animation, and 9100 after it: `Partida` presents the tick's requests in order and each notice or animation holds the following ones (timeline, regras/formiga-amarela.md §9.4; until 2026-10-19 only the animation waited for the notices before it, and 9100 appeared under the fight). Tests: `core::quadros_animados::testes`, `core::partida::testes::quadro_da_aranha_vem_antes_da_luta_e_a_morte_depois`, `regras_amarela.rs` (`quadros_da_morte_pela_aranha_e_pela_formiga_leao`, `renascer_de_ovo_pede_o_quadro_9101`), `trapacas_originais.rs` (`fred_pede_a_animacao_da_vitoria_e_nao_muda_o_estado`), `simant` (`janela_26_como_no_original`, `quadro_da_trofalaxia_como_no_original`).

**Not checked:** the sounds and the music (no sound card on the bench); 9101 of the colony founding (index 2); the fight frames between the spider and 9100 (too fast for the captures); the cause of the 2 pixels of 9100/9101; the exact timing of the frames (the TCG VM clock is approximate; the deadlines of 7.9 s, 9.9 s and 14.85 s matched).
