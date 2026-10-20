*Leia em português: [som.md](../../pt-BR/spec/regras/som.md)*

# Sound and music: when they play

Formats and playback paths: [formatos/som.md](../formatos/som.md). Here: the functions the rest of the game calls, the player's options, preemption, and the catalog of who plays what.

## Sound functions (`GR_MODULE`)

| Function | Address | What it does (multimedia mode, the Windows 3.1 path with a sound card) |
|---|---|---|
| `_myBeginSong(id, prioridade)` | `1008:858e` | Returns if sound was not installed or if `_songsOnFlag` = 0. Loads the type 18 header `id` (returns if it does not exist), looks up `id` in the name table (returns if it is not there), closes the open MCI song, if any, and opens/plays `sound\<nome>.mid`. The priority (0x7e, 0x3f) is not used. |
| `_StopSong()` | `1008:8294` | Closes the MCI device (`MCI_CLOSE`). |
| `_mySongIsDone()` | `1008:a1a0` | In multimedia mode it **always** returns true; only the Sound Blaster mode queries the real end. |
| `_myServiceSong()` | `1008:a216` | With an MCI song open, a `PeekMessage` so the MCI notification moves along. |
| `_myBeginSound(id, taxa, prioridade)` | `1008:98b0` | Returns if sound was not installed or if `_effectsOnFlag` = 0. Loads type 5 `id`, decodes it (formatos/som.md) and asks `waveOutOpen`/`waveOutWrite` for 4096 Hz (on the Sound Blaster 16 it comes out at 5000 Hz, in blocks of 504 samples: section "In-game effects, recorded in a session played by the user"). The second argument is only used with `DSOUND.DLL` (it is the high word of the Mac rate: 0x56ee = 22 254 Hz, 0x2b77 = 11 127 Hz). The priority is not used. |
| `_myBeginSoundReverse(id, …)` | `1008:a18c` | Same as `_myBeginSound` (the "reverse" is not done on Windows). |
| `_mySoundIsDone()` | `1008:a188` | Always 1. |
| `_myBeginSoundList`, `_myBeginSoundSection` | `1008:8292`, `1008:a264` | Empty. |
| `_SysBeep`, `_Beep` | `1008:828a`, `1008:a266` | `MessageBeep`. |

Fact seen in the code for the whole table.

### One effect at a time; the new one interrupts the previous one

There is a single `waveOut` device open. If, when an effect is requested, the previous one is still playing, `_myBeginSound` compares `GetTickCount()` with a deadline recorded at the start of the previous effect (`GetTickCount()` + the position returned by `waveOutGetPosition` in milliseconds, which is ~0 right after the `waveOutWrite`). If the deadline has not yet passed, the request is **discarded**; otherwise, it does `waveOutReset` and plays the new one (fact seen in the code of `_myBeginSound`; the deadline is at `1048:1238`). In practice the deadline is the start instant itself, so only a request in the same millisecond is discarded: the new effect cuts off the previous one [code; **the cut was measured** in the recorded session (2026-10-19): the previous one stops ~35 ms before the new one sounds; the same-millisecond discard does not show up in the recording]. The comparison is `≤` (a request in the very millisecond of the start is discarded) and only applies with the device still in use (`DGROUP:0af0/0af2`, the header of the previous effect not yet released by `MM_WOM_DONE`) [code]. As a consequence, of two effects requested in a row in the same tick with no wait between them, **the first one stays**. **Port (since 2026-10-19):** `ControleSom` keeps the start instant of the last accepted effect (the immediate one or the scheduled one) and discards a request that would start in the same millisecond; stopping effects (option or focus) clears this memory (`simant/src/som/controle.rs`, test `efeito_no_mesmo_milissegundo_e_descartado`). [hypothesis: the resolution of `GetTickCount` on Windows 3.1 may be coarser than 1 ms; the port uses the millisecond of its own clock]

Songs too: opening a new one closes the previous one. Music and an effect play together (different devices).

## Player options

- **Options menu → Music** (command 0x32, `_ProcMenu` `1000:4fc2`): toggles `_OptionStates[1]` (`[0x85f4]`); if turned off, `_StopSong()`.
- **Options → Sound Effects** (command 0x33): toggles `_OptionStates[2]` (`[0x85f6]`).
- `_songsOnFlag` and `_effectsOnFlag` copy these options; they are the ones `_myBeginSong`/`_myBeginSound` test.
- **Persistence:** at the start of `WINMAIN` (`1000:3fda`), before `_NewGame(1)`, the six options come from `WIN.INI`, section `[SimAnt]`, keys `autotrack`, `music`, `effects`, `events`, `messages`, `silly` (`GetProfileInt`, default = the variable's current value), and `_songsOnFlag`/`_effectsOnFlag` copy them; next, the functions of the configurable buttons `mapuserbutton0`–`7` (ribbon 34) and `yarduserbutton0`–`7` (ribbon 35, `_SetUserButton`). On leaving the message loop, `WINMAIN` writes everything back with `WriteProfileString` ("1"/"0" for the options). `_NewGame` does not write to `_OptionStates`: in a new game the current options apply; a loaded `.ANT` brings its own (the `_OptionStates` block of the save), which are the ones written on exit. They do not come from `SIMANT.CFG` [code; keys checked in the `WIN.INI` of the user's installation]. **Port:** `simant/src/preferencias.rs`, file `preferencias.json` in the user's data folder (`SDL_GetPrefPath`, on Windows `%APPDATA%\SimAnt recomp\SimAnt\`), read when opening the game with a window and written on exit; options and buttons of ribbons 34 (`botoes_faixa`) and 35 (`botoes_quintal`, since 2026-10-18). New game (Ctrl+N, end of game) and Load of `Tutorial.Ant` inherit the options of the open game; Load of a `.ANT` uses the ones from the file (`Sessao::herdar_opcoes`). **Initial source** (2026-10-19): while `preferencias.json` does not exist, the port reads the `[SimAnt]` section of the `WIN.INI` of the user's installation (`--win-ini ARQ`, or `WINDOWS/WIN.INI` in the game folder or in one of the two above, case-insensitive; in the user's installation, `SimAnt-T/WINDOWS/WIN.INI` with the 6 options and the 16 buttons), with the `GetProfileInt` conversion (leading digits; missing key = current value; section and keys case-insensitive). It only reads: on exit it writes `preferencias.json`, which applies from then on. An option other than 0 becomes 1 (the original keeps the number read). `--sem-preferencias` turns both off. Tests `win_ini_como_o_getprofileint` and `win_ini_achado_subindo_da_pasta_do_jogo`.
- **Inactive window** (`MAINWNDPROC`, `WM_ACTIVATEAPP`, `1000:2930`): on losing focus, `_StopSong()`, saves the two options and zeroes options and flags (total silence); on returning, restores them. The interrupted song does not restart by itself.

Fact seen in the code.

## Songs: who requests them

Header ids (decimal). "File" from the MCI table in [formatos/som.md](../formatos/som.md); "—" = does not play in MCI mode.

| Id | File | Who requests it | Situation |
|---|---|---|---|
| 10001 | gamethme | `_ShowIntro` (`1000:d3e2`), `_ProcMenu` command 3 (Game → New) before `_NewGame` | intro and new game |
| 10003–10005 | antthme1–3 | `_TryAntTheme` (`1020:1414`), called by `_EnterNest`, `_ExitNest` and `_DoAntMoveY` | when entering/leaving the nest with the yellow ant, if **7200 ticks of `_TickCount`** (~396 s) have passed since the last theme; cycles 10004, 10005, 10003, … (counter at `[0x80ba]`) |
| 10003/10004 | antthme1/2 | `_EndGameDialog` (`1000:6602`) | after the end music, `10003 + _SRand2(…)` [hypothesis: random draw between 0 and 1], once, when `_mySongIsDone` (immediate in MCI) |
| 10008 / 10010 | victory / defeat | `_EndGameDialog` | opens the end dialog with victory (`[0x9cea]` ≠ 0) or defeat, if `_OptionStates[1]` |
| 11002 | — | `_XferPatch` (`1000:ab90`) | patch change in the yard |
| 11003 | — | `_SpecialXfer`, `_PlaceQueenInYard`, `_EditToolsMenu`, `_MapToolsMenu`, `_RibbonToolsMenu` | — |
| 11005 / 20001 | newbody / happy2 | `_YellowBirth` (`1000:739e`) | reborn as a new ant / found a colony |
| 20000 | happy1 | `_YellowBirth` | start of the founding |
| 11006 | qn4day | `_ExchangeLives` (`1020:67ac`) | the yellow ant becomes queen |
| 11013–11017 | call2arm / victqndd | `_YellowCommand` (`1000:87ba`) | recruit 5, 10, all; release half, all (11015 does not play in MCI) |
| 11018 | attack | `_GstrR` (`1030:03c2`), `_StartAttack` (`1030:050e`) | red attack |
| 11020 / 11021 | blackq / redq | `_CountAnts` (`1020:04de`) | black / red queen died (with dialog 10010/10012) |
| 20002, 20003, 20005 | jolly1, jolly2, jolly4 | `_SimColonies` (`1030:35f8`) | yard: reds eliminated, house taken, victory (quintal.md) |
| 20003, 20004 | jolly2, jolly3 | `_DrawSimPayoff` (`1000:cda8`) | in sequence while it shows the score (in MCI, since `_mySongIsDone` is immediate, the second cuts off the first) |
| 20007–20011 | omnious1, sad1–4 | `_YellowDeath` (`1000:7746`) | death of the yellow ant: `0x4E27 + S1(5)` |

Fact seen in the code (calls in `Code1/3/5/6/7`), with the ids checked in the type 18 headers.

## Effects: who requests them

Type 5 ids. The names are the context of the call, not a label from the original. Those that depend on a random draw or on the interface already have the random draws reproduced in the rules (`_MakeBark`, `_SimCat`, chewing etc.).

| Id | Who requests it (function, address) |
|---|---|
| 1 | error: `_GotoMyAnt` in Experimental, `_DoBookMark`, `_XferPatch`, `_PlaceQueenInYard`, `_YellowDeath`, `_SpecialXfer`, `_processSpider`, `_DoAntMoveY`, `_ExpDig`, `_ProcYardEvent`, `_CheatKeys`, yellow ant commands that fail (`_YellowCommand`, `_SetAlarmDropState`) |
| 2 | `_EditToolsMenu`, `_MapToolsMenu`, `_RibbonToolsMenu`, `_CheatKeys`, spider: command 11 |
| 7 | `_SimBird` (`1030:28aa`): bird |
| 9 | `_SimKidInside` (`1030:20b6`), `_ExpKillAnts` |
| 10 | `_EatMyFood`, `_MoveSpider`, `_CheatKeys` |
| 13 | `_SimCat` (`1030:2b24`): cat walking, when `_MapMode` = 0, `_YardMode` < 2 and `_SRand64()` = 0 (0.49 s) |
| 14 | `_SimCat`: the cat starts fleeing from the dog (`_CatOn` = 3), with `_MapMode` = 0 and `_YardMode` < 2, no random draw (1.27 s) |
| 15 | ok/center: `_YellowCommandKey` (Home), `_processSpider`, `_ExchangeLives`, `_SetAlarmDropState` (turned on) |
| 17 / 18 | digging in the black / red nest: `_DoDigInB` (`1028:4bd0`) / `_DoDigInR` (`1028:6f0c`) (ninho.md: "sound 0x11 (R: 0x12)") |
| 19 | `_SetLife` (`1020:5d18`): the yellow ant digs when entering the soil |
| 21 / 22 / 23 | dog bark: `_MakeBark` (`1030:32ea`) type 1 / 3 / 0 and 2; `_SimDog`, `_FollowBoyDir` (21) |
| 24 | `_SimKidInside` |
| 27 | `_ExpIncSmell` |
| 28 | egg/object: `_DropMyEgg`, `_PickupMyEgg`, `_PickupMyObject`, `_TryMyDropOrLift`, `_ExpAddAnt`, `_YellowBirth` |
| 29 | food: `_DropMyFood`, `_PickupMyFood`, `_ExpAddFood`, `_DoTool` |
| 30 | soil/pebble: `_DropMyRock`, `_PickupMyRock`, `_DoAntLions` |
| 32 | `_AddFood`, `_DoTool`, `_ExpAddFood`, `_MysteryButton` |
| 35 | human footstep: `_FootFall` (`1030:3386`) |
| 36 | lawnmower: `_DrawMower` (`1010:b2c4`), at each redraw of the yard (modes 0 and 1) with `_BoyHere` 3 or 4 and the game not paused |
| 37 | fight: `_AnimYellowFight` (`1028:7e56`), `_LionDialog` |
| 38 | antlion: `_LionDialog`, `_DoAntLions` |
| 41 | water: `_DoWater` (`1020:07c6`); spider: command 12 (laser) |
| 42, 45 | `_YellowDialog`, `_SpiderDialog` |
| 44 | eating: `_EatMyFood`, `_MoveSpider` |
| 47 | `_MoveSpider` |
| 49 | `_YellowBirth`, `_AnimYellowFight` |
| 55 | laser: `_DoLaserFire` (`1000:6ec8`) |

Ids 0, 3–6, 8, 11, 12, 16, 20, 25, 26, 31, 33, 34, 39, 40, 43, 46, 48, 50–54 and 56 do not appear as a constant in the calls seen [open: they may come from variables, as in `_MysteryButton` and `_CheatKeys`].

## Port

**Rules → interface.** The rules do not play sound: they request `Evento::Musica(id)` (type 18 header id), `Evento::Som(id)` (type 5 id) and `Evento::TemaDoNinho` (`_TryAntTheme`) on the `Estado` queue (`engine/src/rules/estado.rs`, `Estado::pedir_som`). The effect ids are in the `sons` section of each `data/<parte>.json` (optional: missing id = no sound). Requesting a sound **does not draw random numbers or change the state**: the random draws the original makes because of a sound (bark `S(4)`, cat `S(64)`, bird `S2`, rain `S(50)`, chewing `S2`/`S256`, burp `S2`, fight `S2`) were already being made and stay in the same place. Checked by `engine/tests/sons_regras.rs` (each case runs with and without the ids and compares the states, including over 400 ticks of six saves) and by the original's 24 tick vectors (307/307 blocks).

### What is wired up

| Part (`data/…json`) | Original | Request in the port |
|---|---|---|
| yard | `_SimDog`/`_MakeBark`: bark by the random draw `S(4)` (0x17, 0x15, 0x17, 0x16); bark 0x15 following the lawnmower | `cachorro` → `latir` |
| yard | `_SimCat`: 13 with the view on the yard and `S(64)` = 0; 14 when starting to flee from the dog (since 2026-10-18) | `gato`, `gato_fugindo` |
| yard | `_DrawMower`: 0x24 at each redraw of the yard with the boy mowing or putting away the lawnmower, outside pause (since 2026-10-18; requested by `rules::desenho::desenhar_cortador`, called by the game after the tick) | `cortador` |
| yard | `_SimBird`: 7 when the bird catches the swarm, view on the yard and `S2` = 0 | `passaro` |
| yard | `_FootFall`: 0x23 at each step (outside and inside the house); `_SimKidInside`: 9 (insecticide) and 0x18 in the combinations (node count, node, frame) = (3, 20, 900), (1, 21, 903), (6, 20, 6), (3, 21, 1) | `menino_fora`, `menino_dentro` |
| yard | `_DoWater`: 0x29 with rain and `S(50)` = 0 | `simular_agua` |
| nest | `_DoDigInB/R`: 0x11 / 0x12 when picking up the dug soil | `cavar_para_dentro` |
| surface | `_AddFood(n, 1)`: 0x20 (periodic food) | `RegrasSuperficie::adicionar_comida` |
| enemies | `_DoAntLions`: 0x1E when spitting a pebble/food | `formiga_leao_esperando` |
| enemies | `_MoveSpider`: 0x2F running after the prey; 0x2C chewing; 10 burp (Silly option) | `aranha_perseguir`, `aranha_comer` |
| enemies | `_DoLaserFire`: 0x37 for each ant hit | `laser` |
| yellow | `_SetLife` digging the soil: 0x13 | `por_vida` |
| yellow | `_PickupMyRock`/`_DropMyRock`: 0x1E; `_PickupMyFood` (to carry)/`_DropMyFood`: 0x1D; picking up an egg/`_DropMyEgg`: 0x1C | `pegar_*`, `largar_*`, `tentar_largar_ou_pegar` |
| yellow | `_EatMyFood`: 0x2C (food and egg) and 10 (cannibalism burp) | `comer` |
| yellow | `_AnimYellowFight`: 0x25 per fight frame with `S2` ≠ 0; 0x31 per shock frame | `animar_luta` |
| yellow | `_DoAntMoveY`: 1 when crossing the passage between the nests and when an order fails; `_TryAntTheme` when entering/leaving the nest (also in `_EnterNest`/`_ExitNest`) | `passo_da_ordem`, `mover_amarela`, `entrar_no_ninho`, `sair_do_ninho` |
| yellow | `_YellowDeath` 3 and 4 (surrounded, fled): 1 | `matar_amarela` |
| yellow | `_YellowBirth`: 0x1C when laying the egg and 0x31 when born; before the egg the effects are off (`_OptionStates[2]` = 0), so sounds requested there (digging) are discarded | `nascer_de_ovo`, `fundar_colonia` |
| yellow | `_YellowCommand`: alarm 0xF (on/off) or 1; impossible commands 8 and 9: 1; spider 11: 2; spider 12: 0x29 when turning the laser on, 2 when turning it off | `comando` |
| yellow | `_YellowCommandKey` Home: 0xF; `_ExchangeLives`: 0xF (or the queen music), 1 if it fails; `_processSpider` in target mode: 0xF or 1 | `tecla_de_comando`, `trocar_de_corpo`, `clicar_no_mapa` |
| game (`core/partida.rs`) | `_GotoMyAnt` in Experimental: 1 | `ir_para_amarela` |
| simant | `_TryAntTheme`: 10004, 10005, 10003 in a cycle, if 396 s (7200 × 55 ms) have passed since the last one; the clock runs even with the music off | `som::Audio`, `data/som.json` → `tema_ninho` |

### Interface: what is wired up and what is missing

Checked against the code on 2026-10-18 (before that, this section said Experimental did not exist in the port).

- **Wired up outside the tick** (the requests reach `simant/src/som/` like those of the rules): Experimental tools, `_DoTool` and the "Start Game" of the tools menu (`rules/experimental.rs`, `data/experimental.json` → `sons`, `musicas.comecar_jogo` = 11003; the tools button in Quick Game requests 2, `para_experimental`); `_GotoMyAnt` in Experimental (`core/partida.rs`, sound 1); empty bookmark of `_DoBookMark` (`partida.json` → `marcadores.som_vazio`); `_CheatKeys` (`data/codigos_trapaca.json` → `sons`); `_MysteryButton` (`rules/misterio.rs`); `_EndGameDialog` (10008/10010, `mensagens.json` → `fim_de_jogo`); `_XferPatch`/`_SpecialXfer`/`_PlaceQueenInYard` (1, 11002, 11003; `rules/troca_patch.rs` and `core/quintal_mapa.rs`); MATE button of `_ProcYardEvent` (`quintal.json` → `botao_revoada`).
- **Wired up in the frames of window 26** (2026-10-18, `telas/mensagens.md` §10–11): `_YellowDialog` (42 and 45 of trophallaxis), `_SpiderDialog` (45 on opening and at each loop of the frames), `_LionDialog` (38 on opening, with the wait for the sound to finish, and 37 on odd frames) and `_DrawSimPayoff` (songs 20003 and 20004); `core/quadros_animados.rs` and `core/avisos.rs` request the sounds through the game queue, at the instant of the frame (`mensagens.json` → `quadro_amarela`, `quadros_animados`).
- **Not wired up**, because the screen or the path does not exist in the port: `_StartAttack` through the menu (11018); the tools menus of the edit window and of the map window (`_EditToolsMenu`, `_MapToolsMenu`: 2 and 11003, the same as the ribbon button), because those buttons do not exist in the port; the other buttons of ribbon 35 (`_ProcYardEvent`).
- **Menu bar** (2026-10-18, `telas/menus.md`): only File → New plays something in the original (`_ProcMenu` 3: `_myBeginSong(0x2711)` before `_NewGame(0)`); wired up in `simant/src/menus_ui.rs` (`barra_menus.json` → `musica_novo`), also via Ctrl+N. Opening the menus, About, Status and Quit do not request a sound from the game (the Windows boxes may play the system sound, which the port does not reproduce) [code: `_ProcMenu`, `_AboutDialog`, `_ScoreDialog`, `_MenuQuit` do not call `_myBeginSound`].
- `_myBeginSoundSection` (0x26 of the antlion) is empty on Windows: nothing to request (the `S8()` remains).

**Executable.** `simant/src/som/` receives the events and plays with SDL2:

- Music: looks up the id in `data/som.json` → `musicas` (the MCI table above) and, if `exigir_cabecalho`, requires type 18 in the database, like the original. Reads `sound/<nome>.mid` from the game folder (`Recursos::arquivo`, case-insensitive; mod: `arquivos/sound/<nome>.mid`). With `fonte_musica` = `"banco"`, uses type 20 of word 0 of the header and the transposition of word 3 (the Sound Blaster path). Plays only `canais_midi` (1–10). A new song cuts off the previous one.
- Effect: type 5 decoded, cut into the driver's blocks (`bloco_efeitos` = 504: only whole blocks; less than one block = mute, but it still cuts off the previous one) and resampled from `taxa_efeitos` (5000 Hz, the rate measured on the SB16; the code requests 4096) to the output, with linear interpolation (DOSBox-X's, measured). A new effect cuts off the previous one, except in the same millisecond (discarded; section above).
- Options: follows `_OptionStates[1]` (music) and `[2]` (effects) of the game state; turning off the music stops the one playing.
- Focus (`WM_ACTIVATEAPP`), **on all screens** (intro, "Select a Game" and game): each frame the executable reads whether the window has keyboard focus and is not minimized (`janela_ativa` in `simant/src/jogo.rs`; previously only the game's focus events, which a modal loop could swallow, and the intro did not handle focus). On becoming inactive: stops the music, cuts off the effect, forgets the scheduled ones and plays nothing until it returns; on returning, nothing restarts. The intro music and the new game music are requested when creating the window, before SDL gives focus: they are held until the first focus [port choice; in the original, with the application inactive at the intro, `_songsOnFlag` = 0 and the music would not play]. The user observed in the original the intro music stopping without focus. Logic in `simant/src/som/controle.rs` (`ControleSom`), with tests.
- Intro and new game play `musica_abertura` and `musica_jogo_novo` (10001).

**Timeline of the effects.** Within a tick the original can stay stopped in animations: `_AnimYellowFight` (`1028:7e56`) waits, at the start of each frame, for `_MacTickCount() > início + 6` [code]; since the Mac clock advances in steps of 3 every 55 ms (quintal.md, "Clocks"), that is 3 steps = **165 ms** between two frames (the first waits only for the next step, up to 55 ms; after the last there is no wait). The frame's sound (0x25 in the fight with `S2` ≠ 0, 0x31 in the shock) comes out right after the wait. In the fight (6 frames) the 0x25s are spread over ~825 ms (**measured on 2026-10-19** in the recorded session: intervals of 161–174 ms between frames, average 164.8 ms over 11 intervals; section "In-game effects"), and the effect requested after the animation (in the same tick or the next) only comes out at its end — and, since after the last frame there is no wait, an effect requested right after the last 0x25 falls in the same millisecond and is discarded (since 2026-10-19). The port used to receive the requests of the whole tick in the same video frame: each effect cut off the previous one immediately, and the fight became, at most, one 0x25 — often cut off right after by the digging (0x11/0x12) or by another effect of the same tick; that is why the fight sound could hardly be heard. Now the rules request `Evento::Espera(ms)` between the frames (`amarela.json`, `animacao.espera_quadro_ms` = 165) and `ControleSom` schedules each effect at the right instant: the ones following the wait go into a queue, and the queue of the next frame starts after the waits already scheduled. Since 2026-10-18 the port's simulation also stays stopped during the animation and the edit window shows the frames (`Evento::Animacao`, [formiga-amarela.md](formiga-amarela.md) §10.2), as in the original: no sound from another tick can cut off those of the fight. Volume: 0x25 is 0.46 s long and is among the quietest effects (RMS 0.19 against 0.25–0.40 for most, measured in the WAVs from `--exportar-som`); the port plays effects and music with `volume_efeitos` = `volume_musica` = 0.5 from `data/som.json`. The ratio between `waveOut` and MIDI was measured in DOSBox-X on 2026-10-19 (music 20 dB below an effect at gain 1, section "The original's audio in DOSBox-X"); the FM bank is calibrated for it.
- `--sem-som` turns everything off; `--exportar-som DIR` writes the decoded effects and songs to WAV, without opening audio.

**Synthesizer.** See `docs/decisoes.md` (2026-10-17 and 2026-10-19). `data/som.json` → `sintetizador` chooses (command line: `--sintetizador`): **`"fm"`, the default since 2026-10-19** — the port's own FM synthesizer in the style of the OPL2/OPL3 (`simant/src/som/fm.rs`) with its own free General MIDI bank (`data/opl_banco.json`, generated by `tools/gerar_banco_opl.py`; or a user `.op2` in `fm.banco_op2`), the path closest to the original's Sound Blaster (comparison below); `"soundfont"` — `rustysynth` with the SoundFont from `--soundfont`, `SIMANT_SOUNDFONT` or `soundfont`; `"simples"` — basic waves per GM family and noise for percussion. Without the key, the behavior up to API 0.30 applies (SoundFont if there is one, otherwise the simple one). The FM timbre is not that of the Creative driver (own bank, chip not emulated cycle by cycle): `docs/divergencias.md`. `--exportar-som` writes the songs with `volume_musica` (the game level) and `--exportar-musica ID` limits the export to one song.

**Not reproduced:** the Sound Blaster, PC speaker and Disney Sound Source modes as automatic paths; `_mySongIsDone` false in Sound Blaster mode (in the port, consecutive songs play as in MCI).

## The original's audio in DOSBox-X (2026-10-19)

Tool: `tools/som_dosbox.py` (`capturar`, `niveis`, `comparar`, `harmonicos`, `midi`, `quadros`). **Nothing plays on the PC:** DOSBox-X runs with `[mixer] nosound = true` (sound is still emulated and WAV capture works; checked: the WAVs have the audio) and `SDL_AUDIODRIVER=dummy`, with the window minimized and without activation; no key or click from the user is used. The game files are copied to `re_trabalho/som/C` (a folder ignored by version control); `SimAnt-T` and `referencia/dosbox-x.conf` are not touched. Capture is through the DOSBox-X command `DX-CAPTURE /A` around `win C:\MAXIS\ANTWIN\SIMANTW.EXE` (with `/V`, also ZMBV video, from which `quadros` extracts the keyframes as PNG to know what was on screen).

**Reference installation (fact verified in the files of `SimAnt-T/WINDOWS`):** `SYSTEM.INI` → `[drivers]` `Midi=sb16fm.drv` ("Creative Stereo Music Synthesizer Driver" 2.00, OPL3), `Wave=sb16snd.drv`, `midimapper=midimap.drv`; `[sndblst.drv]` with `VoiceVolume` = `FMVolume` = 16,16. `MIDIMAP.CFG` has the Creative setups ("SB16 All/Basic/Extended MIDI", "SB16 Basic/Extended/All Music Synthesis"); the word at offset 6 of the header is 0x0A, the number of the setup **"SB16 Ext Synth"** (Extended Music Synthesis) [inferred: the `MIDIMAP.CFG` format is not documented; the number matches the record of that setup]. In it, channels 1–10 go to the "Creative Music Synthesizer" on the same channels, **with no patch map or key map**, and 11–16 are muted: the MIDI Mapper hands the FM driver only the extended arrangement of the `.MID` files, with the GM programs unchanged — which is what the port does (`canais_midi` 1–10). `SB16FM.DRV` is only 4 KB; the Creative FM timbre bank is not in it [hypothesis: it is in `VSBPD.386`; not searched] and the port does not use it.

**Captured (without input):**

1. **Intro (10001, `gamethme`)**, 130 s: the sound starts 6.26 s after Windows starts and lasts **102.90 s** (the rest is silence, background noise −74 dBFS). **Mono** output (L and R channels identical, correlation 1.000). The video frames show the title until ~40 s (the 720-tick time limit) and then the "Select a Game", idle; the music plays to the end after the title closes.
2. **Level of an effect on the same path:** effect 41 (water, 3.15 s) decoded by the port, resampled to 11 025 Hz and set as the "Windows Start" sound in the copy's `WIN.INI` (`capturar --som-inicio`), comes out through the same `sb16snd.drv` (`waveOut`) as the game's effects, 1.7 s before the music. In the recording it appears with **correlation 0.99989** with the PCM and **gain 0.2504 (−12.0 dB)** relative to 8-bit full scale. The music, on the same scale, has **RMS −32.0 dBFS** (peak 0.15). Therefore, in the original, **the music is 20.0 dB below an effect played at gain 1** (RMS of the music against the gain of the effect, with the SB16 mixer volumes of the installation). With the effect at 4096 Hz (the game's rate), Windows 3.1 crashed when playing the startup sound ("Corrupt UMB chain" and an invalid interrupt loop in the log) [observed, not investigated: the game's `waveOutOpen` may take another path]; the 11 025 Hz rate does not change the level.

**Not captured (in this section):** the other songs (captured later, next section) and the in-game effects. The DOSBox-X `AUTOTYPE` did not deliver keys with the window minimized and without focus (tested in DOS, typing a `mkdir`, and in Windows, with space on the title and Enter on "Select a Game": nothing changed on screen or in the sound), and operating the window would require focus on the user's PC. In DOS the cause was the keyboard layout (next section); in Windows they still do not arrive. `-time-limit` also does not terminate DOSBox-X when the guest crashes (the process had to be killed).

**Comparison with the port** (`som_dosbox.py comparar`, whole song, port exported at volume 1; 10001 is also the new game music):

| Measure | Original (DOSBox-X) | Port, simple synthesizer | Port, FM (port's bank) |
|---|---|---|---|
| Sound duration | 102.90 s | 103.12 s | 103.18 s |
| Tempo (best port scale, step 0.25 %) | — | 0.9975 (chroma 0.825; with 1.000: 0.819) | 0.9975 (0.857; with 1.000: 0.849) |
| Notes and order: frame-by-frame chroma, aligned (control: port shifted 0.9–4 s) | — | 0.819 (0.391) | 0.849 (0.431) |
| Average spectral centroid | 1038 Hz | 1713 Hz | 854 Hz |
| Energy 0–200 / 200–500 / 500–1k / 1–2k / 2–4k / 4k+ Hz | 30 / 29 / 12 / 12 / 12 / 6 % | 34 / 17 / 10 / 13 / 7 / 18 % | 36 / 27 / 16 / 12 / 5 / 5 % |
| Average spectrum (60 Hz–8 kHz): correlation; average difference | — | 0.590; 9.6 dB | 0.677; 8.2 dB |
| Music relative to an effect at gain 1 (`volume_musica` = `volume_efeitos`) | −20.0 dB | −23.7 dB | −19.8 dB (calibrated) |

- **Duration and tempo:** they match. The original's sound lasts 102.90 s for 102.99 s of notes in the MIDI (ratio 0.999); the scale that best aligns the port (0.9975) improves the chroma by less than 0.01, within the resolution of the test [verified].
- **Notes and order:** the port's chroma aligned to the original gives 0.82–0.85, against 0.39–0.43 with the port shifted: the notes are in the same order and at the same time, with the original 6.20 s after the start of the recording [verified on the intro]. The channel 10 percussion uses notes outside the GM map (85, 86, 89, 91, 95 in the `.MID` files): in the windows where only 89 sounds, the original shows no sound of its own for it [observed in two windows; the port's bank leaves them silent, a hypothesis for the others].
- **Timbre:** in the original the bass (program 34) has the 2nd harmonic as the strongest, the 1st 10 dB and the 3rd 6 dB below and the 4th–12th flat between −17 and −24 dB (27 isolated notes, `som_dosbox.py harmonicos`); the piano (program 3) starts with the first 5 harmonics nearly equal and the high ones fall faster than the fundamental; tone 43 sounds like a low tone of ~69 Hz with falling pitch; there is no noise in the percussion (spectrogram). The simple synthesizer is far too bright (sawtooth/square waves and white-noise percussion: 18 % of the energy above 4 kHz, against 6 %); the FM gets closer in the average spectrum and in the distribution per band, but is still dark between 2 and 4 kHz (5 % against 12 %). The FM bank was tuned on the `gamethme` programs and, since the capture of the other songs, on 59 (next section) (bass 34: profile −2/0/−6/−11/−12/−13… in the port, against −10/0/−6/−22/−17/−17… in the original). The other programs used by the songs were compared on 2026-10-19 (next section).
- **Relative volume:** with the simple one, the music was 3.7 dB quieter relative to the effects than in the original; the FM bank had `ganho` 2.25, calibrated for the intro to give −19.8 dB (against −20.0 dB measured); since the 2026-10-19 adjustment (section "FM bank adjustment"), 1.65 and −20.0 dB. The other songs are at whatever relative level the bank gives [not measured].

## The original's other songs, without input (2026-10-19)

**Route:** `tools/som_dosbox.py musicas PASTA [NOME ...]`. In the **copy** of the game (`re_trabalho/som/C`), `GAMETHME.MID` is replaced by the requested song and SimAnt itself plays it on opening, through the same path as all songs on Windows (MCI → MIDI Mapper "SB16 Ext Synth" → `sb16fm.drv`, DOSBox-X's OPL3); the original is restored at the end. No key, no click, nothing played on the PC (same conditions as the intro capture, above). The 28 songs with `.MID` were recorded (the 27 besides the intro, plus the intro again as a level reference); `jolly5` too, but the original does not play it (20006 has no header, formatos/som.md) and neither does the port. Comparison: `som_dosbox.py comparar_todas ORIGINAIS PORT/musicas saida.json` (port exported with `--exportar-som`, FM).

| Id, song | Sound (s): original / port | Aligned chroma (shifted control) | Envelope in dB (control) | Centroid (Hz): original / port | Level relative to the intro (dB): original / port (difference) |
|---|---|---|---|---|---|
| 10001 gamethme | 102.90 / 103.15 | 0.86 (0.43) | 0.64 (0.29) | 1038 / 854 | +0.0 / +0.0 (+0.0) |
| 10003 antthme1 | 55.37 / 55.70 | 0.51 (0.48) | 0.37 (-0.05) | 1053 / 2085 | -9.1 / -7.7 (+1.4) |
| 10004 antthme2 | 47.48 / 47.93 | 0.54 (0.48) | 0.75 (0.02) | 1094 / 4081 | -8.1 / -5.5 (+2.5) |
| 10005 antthme3 | 41.12 / 41.38 | 0.64 (0.56) | 0.58 (0.16) | 345 / 496 | -8.1 / -8.1 (+0.1) |
| 10006 hmanawar | 19.99 / 20.12 | 0.82 (0.32) | 0.81 (0.09) | 953 / 1100 | -0.3 / +3.4 (+3.7) |
| 10007 hmanthme | 20.07 / 20.19 | 0.82 (0.32) | 0.93 (0.18) | 913 / 1130 | -0.4 / +3.0 (+3.4) |
| 10008 victory | 30.04 / 30.13 | 0.87 (0.43) | 0.96 (0.01) | 458 / 769 | -3.0 / +0.2 (+3.3) |
| 10010 defeat | 34.40 / 34.58 | 0.90 (0.42) | 0.97 (0.10) | 622 / 564 | -0.8 / +2.7 (+3.5) |
| 11004 newcoln2 | 4.47 / 4.11 | 0.69 (0.21) | 0.96 (-0.14) | 767 / 1526 | -5.1 / -1.9 (+3.2) |
| 11005 newbody | 6.98 / 6.99 | 0.75 (0.22) | 0.90 (-0.08) | 731 / 911 | -2.8 / +3.1 (+5.9) |
| 11006 qn4day | 19.96 / 17.65 | 0.90 (0.56) | 0.42 (-0.00) | 742 / 1104 | +1.5 / +5.8 (+4.4) |
| 11007 badnews | 8.31 / 6.12 | 0.83 (0.53) | -0.02 (0.04) | 110 / 253 | +3.2 / +8.3 (+5.1) |
| 11014 call2arm | 6.80 / 6.11 | 0.85 (0.46) | 0.81 (0.01) | 364 / 585 | +3.1 / +7.7 (+4.6) |
| 11017 victqndd | 8.07 / 5.63 | 0.82 (0.60) | 0.07 (0.04) | 467 / 761 | +3.1 / +8.3 (+5.1) |
| 11018 attack | 7.46 / 5.86 | 0.76 (0.25) | 0.85 (0.03) | 622 / 339 | -7.7 / -6.3 (+1.4) |
| 11020 blackq | 4.84 / 4.91 | 0.67 (0.24) | 0.96 (-0.13) | 458 / 701 | -12.4 / -13.8 (-1.3) |
| 11021 redq | 7.47 / 8.10 | 0.94 (0.25) | 0.84 (-0.13) | 601 / 641 | +1.5 / +3.8 (+2.3) |
| 20000 happy1 | 5.47 / 4.63 | 0.69 (0.23) | 0.75 (-0.17) | 1591 / 1418 | -9.4 / +2.1 (+11.6) |
| 20001 happy2 | 5.28 / 4.49 | 0.65 (0.18) | 0.88 (-0.14) | 1093 / 784 | -6.5 / -6.4 (+0.1) |
| 20002 jolly1 | 7.79 / 7.83 | 0.71 (0.34) | 0.92 (-0.12) | 934 / 282 | -9.9 / -4.9 (+5.0) |
| 20003 jolly2 | 6.45 / 6.62 | 0.56 (0.21) | 0.92 (-0.12) | 1000 / 316 | -3.7 / +4.9 (+8.6) |
| 20004 jolly3 | 4.06 / 4.17 | 0.50 (0.31) | 0.90 (-0.19) | 1039 / 340 | -8.8 / -3.0 (+5.8) |
| 20005 jolly4 | 7.98 / 8.09 | 0.70 (0.35) | 0.81 (-0.08) | 946 / 346 | -11.8 / -5.3 (+6.4) |
| 20007 omnious1 | 7.87 / 7.21 | 0.74 (0.37) | 0.62 (-0.04) | 341 / 351 | -5.5 / -0.2 (+5.3) |
| 20008 sad1 | 14.85 / 13.58 | 0.84 (0.43) | 0.63 (0.13) | 861 / 1176 | -7.6 / -1.3 (+6.2) |
| 20009 sad2 | 1.21 / 7.49 | 0.96 (0.32) | 0.79 (-0.12) | 342 / 452 | -9.4 / -2.9 (+6.5) |
| 20010 sad3 | 8.64 / 8.70 | 0.96 (0.18) | 0.91 (-0.07) | 670 / 359 | -5.0 / -4.8 (+0.3) |
| 20011 sad4 | 10.18 / 9.04 | 0.89 (0.90) | -0.14 (0.23) | 795 / 552 | -6.2 / +1.4 (+7.6) |

Reading (fact measured in the recordings; "control" = the port shifted 0.9–4 s):

- **Notes, order and tempo match in all of them.** The aligned chroma is above the control in all melodic songs (0.50–0.96 against 0.18–0.60), and the frame-by-frame energy envelope gives 0.6–0.97 in most; the sound duration matches (differences of up to ~1 s come from the release tails of the notes). The exceptions are of timbre, not of notes: `antthme1–3` are percussion only (channel 10; chroma does not apply, the envelope gives 0.37–0.75 against control ≤ 0.16); `badnews` and `victqndd` (program 95, Pad 8) and `sad4`/`qn4day` have a low envelope because in the original the pad and the strings sound with a different envelope. **Revised on 2026-10-19 (next section):** the low envelope came almost entirely from the DC component that DOSBox-X records (a subsonic tail after the notes); removed with a 20 Hz high-pass on both recordings, `badnews`/`victqndd` give 0.95/0.93 with the old bank.
- **Program 49 (String Ensemble 2):** in the original a strong attack and, half a second later, the sustain ~17 dB below, with the following notes (short, 0.23 s, same velocity, no controllers in the `.MID`) quiet (`sad2`: "useful" sound of 1.2 s against 7.5 s in the port). A decay on the carrier of the port's bank did not change the measurement (the notes are too short) and was undone [open: probably the timbre and voice allocation of the Creative driver]. **Explained on 2026-10-19 (next section):** the strong note is not from 49: in the `.MID` the first note comes before the program change and plays with program 0; the original's 49 has a slow attack (~0.5 s), and the 0.23 s notes do not reach full level.
- **Program 59 (Muted Trumpet):** ~10–12 dB too loud in the port relative to the intro (`happy2`, with only it: +10.2 dB). **Fixed in the bank** (`tools/gerar_banco_opl.py`, carrier 16 TL steps lower): `happy2` +0.1 dB.
- **Level:** with 59 fixed, the other songs are on average ~4 dB above the original relative to the intro (median 4.1; from −1.3 to +8.6, except `happy1`, +11.6, and `sad4`, +7.6, which mix in 49). The intro is calibrated (−19.8 dB against an effect at gain 1); the overall gain was not touched, because the difference is per program [open: a gain per program, measured song by song]. **Done on 2026-10-19** (next section: `NIVEL_TL`).
- **Timbre:** the original's centroid is between 340 and 1600 Hz; in the port, close in the bass and string songs, darker in the `jolly*` (programs 19 and 69: 282–346 Hz against ~1000 Hz) and brighter in the `antthme*` (percussion). Programs 19, 21, 25, 31, 33, 35, 49, 57, 69 and 95 still have the timbre of the port's own bank, without fine tuning. **Revised on 2026-10-19:** the "dark" of the `jolly*` was the DC component of the port's 19 (additive with half sines, +5 dB of DC in `jolly2`); without it the port was far too bright (2–4 kHz 28–37 % against 8 %).

## FM bank adjustment against the 28 songs (2026-10-19)

Route: the same recordings (`re_trabalho/som/musicas/`, previous section), the port exported with `--exportar-som` and `som_dosbox.py comparar_todas`, plus two new commands: `perfil ORIGINAIS PORT PROGRAMA músicas` (average harmonic profile of a program in the windows where no other note starts, skipping the partials that coincide with those of another note) and `percussao ORIGINAIS PORT músicas` (level, dominant frequency and centroid of each channel 10 note at the isolated onsets). The bank comes out of `tools/gerar_banco_opl.py` (`medidos()`, `NIVEL_TL`, `percussao()`, `CURVA_VOLUME_DB`, `GANHO`, with the original's value commented in each timbre).

**Change in the measurement (measured fact):** DOSBox-X records the DC component of the OPL waves (the half sines and waves 2–7 have a nonzero mean; a real card has a capacitor-coupled output) and, after the notes, a subsonic tail that decays ~8 dB/s (`badnews`: with a 40 Hz high-pass the note ends in less than 0.2 s; without it, it "lasts" 3 s more with the dominant frequency at 0 Hz). The port also generates a DC component (in the old 19, +5 dB in the RMS of `jolly2`). Since this date `comparar` passes both recordings through a 20 Hz high-pass and also records `energia_alinhada_db` (the energy in the same aligned frames, which does not depend on where each recording finds the end of the sound). With this, part of what the previous section saw changed: the "different envelope" of 95 and of `sad4`/`qn4day` was the subsonic tail (envelope 0.85–0.95 with the old bank) and the `jolly*` were too bright, not dark.

**Findings (facts measured in the recordings):**

1. **Velocity curve.** `sad2` plays 32 notes of 49, one at a time, with the velocity falling from 126 to 20: in the original the level falls ~23–29 dB per decade of velocity (110 → 68: −4.8 dB; 110 → 52: −7.4 dB), against 40 in the previous bank. `curva_volume_db` = 25.
2. **Program before the change.** In 13 of the 29 `.MID` files the first note of a channel comes before the Program Change at the same instant (order of the events in the track); that note plays with the program the channel had. In the recording (freshly started Windows) it sounds like a piano (program 0): it is the "strong attack" of 49 in `sad2` and `omnious1`. The port does the same (the channel starts on program 0 at each song). [hypothesis] in a game the Creative driver would keep the program of the previous song; there is no way to record two songs in a row without input.
3. **Harmonic profiles** (harmonics 1–12, dB; "." = covered by another note): 0 `0 -10 -21 -8 -9 -11 -16 -17 -25 -28 -43 -42` (the previous one died out in ~1.5 s; the original decays ~12 dB/s and then ~5 dB/s); 19 `0 -10 -27 -3 -5 -40 -5 -17 -41 -18 -17 -49` (the multiples of 3 are missing: FM with the modulator at 3×); 21 `0 -71 -9 -64 -9 -69 … -18` (odd ones only); 33 `0 -11 -7 -21 -25 -16 -14 -13 -14 -15 -17 -18`; 35 `-3 0 -5 -13 -22 -31 -37 -47 -51 -56 -57 -60`; 49 `0 -2 -5 -9 -15 -20 -22 . -34 -34 -40 -37`; 57 `. . 0 0 -10 -7 -10 . -17 -38 -27 -42`; 59 `0 -6 -1 -1 -2 -3 -5 0 -5 -7 -10 -15` (all strong; the previous one had only odd ones); 95 `0 -85 -64 -88 -69 -96 …` (almost a pure sine, against −27…−101 in the previous one). With the new bank, the port's 19 gives `0 -7 -59 -3 -5 -57 -6 -14 -58 -16 -18 -55`. Programs 3, 25, 26, 31 and 69 do not have enough isolated windows: adjusted by the per-octave spectrum of the songs (`gamethme`: piano 3 left the 2–4 kHz band 5–6 dB below in the passages where it plays; without KSR on both operators, the high notes, 70–101, sustain as in the original).
4. **Envelopes:** 49 with an attack of ~0.5 s up to full level (final chord of `omnious1`; the 0.23 s notes of `sad2` stay ~13 dB below full); 95 with immediate attack, flat sustain and short release (more than 30 dB in 0.2 s).
5. **Level per program** (TL steps of 0.75 dB, `NIVEL_TL`): 0 +6, 19 +8, 21 +4, 25 +4, 31 +6, 33 +4, 35 +3, 57 +6, 59 −14 (on top of the 16 steps from before), 69 +6, 95 −2. The overall `ganho` was recalibrated from 2.25 to 1.65: the exported intro (volume 0.5) gives −26.0 dBFS, that is, **−20.0 dB** relative to an effect at gain 1, as in the original.
6. **Percussion** (channel 10, isolated onsets; level in dB relative to the RMS of the intro, dominant frequency of the first 55 ms; full table with `som_dosbox.py percussao`):

| Note | Original | Port before | Port after | What changed |
|---|---|---|---|---|
| 36 | −6.2 dB; 91 Hz; centroid 576 Hz | −6.1; 64 Hz; 65 Hz | −9.4; fundamental 92 Hz; 359 Hz | carrier at note 42 (was ½ × note 26, subsonic) |
| 39 | −3.9; 1950 Hz (tonal) | −2.8; noise (centroid 9.2 kHz) | −5.8; 1977 Hz | tone at note 95 |
| 40 | −6.8; noise (9.7 kHz) | −1.8 | −9.0 | 8 steps lower |
| 43 | −2.1; 68 Hz; 281 Hz | −6.8; 68 Hz; 137 Hz | −4.2; 68 Hz; 156 Hz | without the modulator at ½; noise click on the attack |
| 47 | −2.6 (3 onsets); ~100 Hz | −8.7 | −7.2; 100 Hz | note 43 |
| 48 | −2.5; 91 Hz; 452 Hz | −8.1; 245 Hz | −4.8; 95 Hz; 515 Hz | note 42 |
| 49 | −12.4; noise (7.9 kHz) | −4.9; 9.4 kHz | −15.1; 8.6 kHz | 13 steps, carrier at 1× |
| 50 | −4.4; 123 Hz; 624 Hz | −14.1; 145 Hz (bands at 0.5× and 1.5×) | −4.6; 123 Hz | note 47, modulator at 1× |
| 58 | −17.7; centroid 2.5 kHz | −7.3; 9.2 kHz | −19.2 | 19 steps |
| 59, 77, 80 | **no sound** (67, 18 and 9 onsets) | sounded (−6.5 to −9.2) | no timbre | muted |
| 60 | −4.5; noise (8.7 kHz) | −5.2; 586 Hz tone | −6.4; noise | noise |
| 67 | no increase in any band at the 12 isolated onsets of `gamethme` | sounded | no timbre | muted |
| 68 | −1.4; 2081 Hz tone lasting the note | −3.3; short 786 Hz | −5.6; sustained 2095 Hz | note 96, sustained |
| 70 | inconclusive (`gamethme`, dense song) | already inaudible (note 98) | no timbre | muted |
| 76 | −7.4; 823 Hz | −3.3; 700 Hz | −8.1 | note 80 |
| 85, 86, 89, 91, 95 | **no sound** (86, 89, 91: 5–26 onsets; 86 and 95 together in `hmanawar`, 4.38 s: silence; 85 only appears with others) | no timbre | no timbre | — |

The "after" levels are 1–3 dB below the original in this table because the port's intro (the reference) became brighter and fuller; the percussion-only songs ended up between +0.4 and +1.4 dB (next table). Notes not measured (35, 37, 42, 46, 51, 65, 72, 75: up to 2 isolated onsets) kept the template's timbre, with the level set by that of their neighbors.

**Before and after** (`comparar_todas` with the high-pass; level = the song's `energia_alinhada_db` minus the intro's, port against original; envelope = correlation of the energy per frame in dB; spectrum = correlation of the average spectrum 60 Hz–8 kHz; "before" = the 2026-10-19 bank with 59 already fixed, the one from the previous section):

| Song | Original's level relative to the intro (dB) | Port − original (dB): before / after | Envelope: before / after | Spectrum: before / after | Centroid (Hz): original / before / after | 2–4 kHz (%): original / before / after |
|---|---|---|---|---|---|---|
| antthme1 | -9.2 | +1.4 / +0.7 | 0.40 / 0.65 | 0.59 / 0.73 | 1281 / 2095 / 1289 | 35 / 11 / 17 |
| antthme2 | -8.2 | +2.9 / +1.4 | 0.52 / 0.87 | 0.47 / 0.41 | 1766 / 4023 / 2434 | 31 / 14 / 30 |
| antthme3 | -8.1 | -0.2 / +0.4 | 0.62 / 0.90 | 0.72 / 0.70 | 348 / 494 / 502 | 1 / 2 / 2 |
| attack | -7.6 | +0.3 / -0.5 | 0.72 / 0.95 | 0.90 / 0.96 | 623 / 309 / 588 | 1 / 0 / 1 |
| badnews | +3.9 | +4.9 / +0.5 | 0.95 / 0.94 | 0.82 / 0.89 | 162 / 259 / 120 | 0 / 0 / 0 |
| blackq | -12.4 | -1.4 / +0.3 | 0.96 / 0.96 | 0.64 / 0.88 | 464 / 703 / 416 | 0 / 6 / 0 |
| call2arm | +4.1 | +4.1 / -0.7 | 0.90 / 0.97 | 0.40 / 0.91 | 379 / 604 / 367 | 0 / 2 / 0 |
| defeat | -0.8 | +1.9 / -1.2 | 0.96 / 0.97 | 0.61 / 0.77 | 623 / 895 / 794 | 5 / 6 / 6 |
| gamethme | +0.0 | +0.0 / +0.0 | 0.83 / 0.85 | 0.68 / 0.63 | 1056 / 852 / 1296 | 12 / 5 / 7 |
| happy1 | -9.0 | +11.4 / +1.8 | 0.92 / 0.98 | 0.64 / 0.70 | 1996 / 1433 / 1694 | 21 / 25 / 24 |
| happy2 | -5.9 | -0.8 / -1.0 | 0.97 / 0.98 | 0.69 / 0.75 | 1153 / 778 / 1171 | 13 / 8 / 12 |
| hmanawar | -0.2 | +3.6 / -0.5 | 0.96 / 0.95 | 0.66 / 0.69 | 956 / 1112 / 1126 | 6 / 4 / 11 |
| hmanthme | -0.3 | +3.4 / -0.1 | 0.97 / 0.95 | 0.56 / 0.69 | 916 / 1143 / 1005 | 8 / 4 / 9 |
| jolly1 | -9.8 | -1.1 / -0.2 | 0.95 / 0.96 | 0.58 / 0.74 | 959 / 1543 / 1121 | 8 / 37 / 13 |
| jolly2 | -3.6 | +3.5 / +0.4 | 0.97 / 0.96 | 0.60 / 0.73 | 1012 / 1501 / 1194 | 8 / 28 / 12 |
| jolly3 | -8.8 | -0.2 / +0.1 | 0.23 / 0.98 | 0.59 / 0.70 | 1073 / 1586 / 1273 | 9 / 31 / 13 |
| jolly4 | -11.7 | +3.9 / +2.2 | 0.93 / 0.94 | 0.63 / 0.70 | 960 / 1389 / 1028 | 6 / 6 / 7 |
| newbody | -2.7 | +5.8 / -1.2 | 0.92 / 0.93 | 0.55 / 0.88 | 732 / 915 / 915 | 3 / 6 / 6 |
| newcoln2 | -4.7 | +2.9 / -1.0 | 0.98 / 0.99 | 0.75 / 0.76 | 787 / 1527 / 1189 | 4 / 13 / 3 |
| omnious1 | -5.1 | +5.0 / -1.5 | 0.88 / 0.92 | 0.88 / 0.90 | 359 / 351 / 385 | 0 / 0 / 0 |
| qn4day | +1.7 | +4.1 / -0.4 | 0.85 / 0.97 | 0.57 / 0.76 | 922 / 1173 / 770 | 4 / 4 / 0 |
| redq | +1.6 | +2.6 / -1.4 | 0.88 / 0.98 | 0.71 / 0.96 | 601 / 641 / 612 | 0 / 0 / 0 |
| sad1 | -7.4 | +6.1 / -1.8 | 0.93 / 0.97 | 0.62 / 0.72 | 950 / 1189 / 1146 | 5 / 15 / 13 |
| sad2 | -9.3 | +15.0 / +1.5 | 0.95 / 0.97 | 0.89 / 0.92 | 425 / 456 / 339 | 0 / 0 / 0 |
| sad3 | -5.0 | +0.2 / -1.0 | 0.87 / 0.94 | 0.89 / 0.94 | 670 / 359 / 737 | 2 / 0 / 2 |
| sad4 | -5.8 | +7.5 / +0.7 | 0.88 / 0.95 | 0.68 / 0.73 | 832 / 603 / 892 | 8 / 1 / 5 |
| victory | -3.0 | +3.2 / -0.6 | 0.93 / 0.90 | 0.86 / 0.87 | 471 / 779 / 556 | 1 / 1 / 0 |
| victqndd | +4.3 | +4.3 / -0.1 | 0.93 / 0.98 | 0.51 / 0.82 | 606 / 807 / 456 | 1 / 2 / 0 |
| **Average** | | **+3.4 / −0.1** (average of the absolute value: 3.6 / 0.8) | 0.85 / 0.94 | 0.67 / 0.78 | absolute value of log₂(port/original): 0.53 / 0.24 | 6.8 / 8.2 / 6.8 |

- **Level:** from +3.4 dB on average (the "~4 dB above" of the previous section, which without the high-pass gave +4.0) to −0.1 dB; no song exceeds ±2.2 dB (`jolly4` +2.2, `sad1` −1.8). The intro is still −20.0 dB below an effect at gain 1.
- **Timbre:** the average spectrum rises from 0.67 to 0.78 correlation and the average difference falls from 14.3 to 10.2 dB; the centroid is within 18 % of the original on average (it was 44 %). The 2–4 kHz band is the same on average (6.8 %), but per song there are still deviations: the intro has 7 % against 12 % and its centroid went past the original's (1296 against 1056 Hz, because of the 8 kHz of the percussion); `antthme1` 17 % against 35 %; `sad1`, `hmanawar` and the `jolly*` 4–8 points above.
- **Envelope:** from 0.85 to 0.94 on average (`jolly3` 0.23 → 0.98, `attack` 0.72 → 0.95, `antthme2` 0.52 → 0.87).
- **Aligned chroma** (notes): 0.78 → 0.83 on average.

**Still different [open]:** the Creative bank was not read (the imitation is by measurement, with the port's own 2-operator timbres); `antthme1` (35 % of the energy in 2–4 kHz in the original against 17 %) and `antthme2` (spectrum 0.41): the original's percussion has components that the 2-operator-per-note model does not reproduce well (47 and 60 are tone + noise); 26 (the guitar of `gamethme`) has only 2 isolated windows and was left as it was; the notes of the percussion table with fewer than 3 isolated onsets.

**In-game effects: captured later in a session played by the user** (next section). Before that, attempted (2026-10-19): the DOSBox-X `AUTOTYPE` **works with the window minimized and without focus** when the DOS keyboard layout is US (`[dos] keyboardlayout = us`, now in `som_dosbox.py`): with this machine's automatic layout (BR) it delivered nothing, which was the failure recorded before. In DOS (`mkdir` typed) it gets through; **in Windows 3.1 in 386 enhanced mode no key arrives**: Space on the title at 25–27 s does not close it (still there at 38 s, video frames), Enter and Alt on "Select a Game" do nothing [hypothesis: `AUTOTYPE` puts the keys in the BIOS buffer, which Windows does not read]. The QEMU test bench (alternative a) still does not work for this: QEMU's `sb16` plays the effects with the wrong rate and buffer and there is no OPL3 (section below); the user's installation has no driver for AC97/ES1370. Remaining routes: input through the serial port (serial mouse emulated via `nullmodem`) in DOSBox-X, or a keyboard driver in the copy's Windows [not attempted].

## In-game effects, recorded in a session played by the user (2026-10-19)

**Recording:** the user played the original in DOSBox-X (`tools/som_jogar.py`: the same copy `re_trabalho/som/C`, SB16 + OPL3, Windows 3.1, effects through `waveOut` on `sb16snd.drv`) for 9 min 35 s, with the whole audio recorded (`re_trabalho/som/sessao/efeitos_original.wav`, 44 100 Hz, 16 bits, stereo with both channels identical; outside git). Script without marked times: walking and digging in the nest, picking up/dropping food and a pebble, eating, recruiting/releasing, alarm, switching bodies, Home, fights, impossible order and empty bookmark, Yard View, spider and antlion, Mystery, Experimental.

**Analysis:** `tools/som_efeitos.py analisar SESSAO.wav EFEITOS_DIR` (the 57 type 5 effects as the port decodes them, `--exportar-som`). Matched filter of the start of each effect (120 ms) against the recording decimated to 11 025 Hz, with the session's fixed gain (which removes the false positives from the music); then, on the 44.1 kHz recording, fine alignment (sample and phase), and the played duration in 2 ms blocks (the recording follows the effect, goes silent, or shows something else). `som_efeitos.py taxa` measures the rate (3500–12 000 Hz) and `som_efeitos.py musicas` finds the songs recorded without input (`som_dosbox.py musicas`) in the session. Nothing plays on the PC. Controlled test: `som_dosbox.py capturar --som-inicio` with an effect from the port as the Windows startup sound (the same driver), muted, without input.

### Separating the track

- **Songs** (MCI → OPL3), found by correlation with the recordings without input: `gamethme` at 6.2 s (the intro, sample by sample equal to the reference recording up to ~24 s, gain 1.000; after that the sequencer's tempo slips a few ms with the game running), `call2arm` at 100.8 and 108.2 s (recruit), `victqndd` at 114.2 and 120.0 s (release), `sad3` at 212.4 s (death of the yellow ant), `newbody` at 224.1 and 407.8 s (rebirth), `antthme2` at 393.4 s and `antthme1` at 537.8 s (nest themes), `gamethme` again at 430.1 and 448.0 s [hypothesis: new game]. No song other than those the port requests in these situations.
- **Effects** (`waveOut`): **156 occurrences of 14 effects**, all with the same gain (−12.2 dB, see below). Apart from them, no sound with an abrupt onset that is not a note of the music (the music subtracted from the reference recording between 7 and 24 s leaves nothing besides the effects found).

### Rate, interpolation and level (measured facts)

- **Rate: 5000 Hz** (5000 ± 0.5 Hz: the whole 0x20 effect, 0.6 s, gives NCC 0.975 at 5000 Hz and drops to 0.95 at ±2 Hz; 0x26, swept from 3500 to 12 000 Hz in steps of 100, gives 0.998 at 5000 and 0.19 at the second best). The game requests 4096 Hz (formatos/som.md), the SB16 does not go below 5000 Hz and `sb16snd.drv` plays the bytes at 5000 Hz without resampling: **the effects sound 22 % faster and higher-pitched than at 4096 Hz** (and 10 % slower than on the Mac, ~5563 Hz). Confirmed in the controlled test: a 4096 Hz WAV through the same driver also comes out at 5000 Hz.
- **Interpolation:** the recording follows the PCM decoded by the port **sample by sample** (residual −45 dB in passages without music) with **linear** interpolation from 5000 to 44 100 Hz (NCC 0.988 against 0.968 for the step), which is DOSBox-X's resampler, not the filter of a real card [hypothesis: on a real SB16 the output filter rounds off more]. The port already resamples with linear interpolation.
- **Gain:** −12.2 dB (0.245) relative to 8-bit full scale, the same for all effects (−12.1 to −12.9 dB on the whole ones), the same as the previous measurement with effect 41 (−12.0 dB). **Level relative to the music:** the effects have RMS of −21 to −30 dBFS and the music, in the same passages, −25 to −40 dBFS (median per effect: 4 to 16 dB above the music). Since the gain of the effects and the level of the music (the intro with gain 1.000 relative to the reference) are the ones already measured, "music 20 dB below an effect at gain 1" still holds (section "The original's audio in DOSBox-X"); nothing to change in the volumes.

### The driver plays the effects in blocks of 504 samples (measured fact)

1. **Effect requested with `waveOut` free:** only **whole blocks of 504 samples** sound; the rest of the last block is lost. Measured on all isolated effects: 0x2C 1512 of 1824 samples (10 times), 0x25 1512 of 1888 (10), 1 504 of 752 (4), 0x18 1008 of 1480 (7), 0x26 2520 of 2880, 0x20 3024 of 3374 (3), 9 504 of 800, 10 2520 of 2706; the cat 0xD (2016 = 4 × 504) plays whole (4 times).
2. **An effect with fewer than 504 samples does not sound.** No occurrence of 15 (102 samples, Home/alarm), 0x11 and 0x12 (358 and 406, digging by the blacks and reds), 0x13 (148, the yellow ant digging), 0x1C (436, egg), 0x1D (172, food) and 0x28 (184), although the script walked, dug, picked up food and pressed Home; all effects with 752 samples or more that the script requested appeared. **Controlled test** (Windows startup sound at 4096 Hz through the same driver, without input): 15, 0x1C and 0x23 (484) do not sound; 0x2C comes out with 1512 of 1824 samples, with the same glitch in the middle; at 5000 Hz 0x2C comes out whole. Therefore, the cut and the silence come from the driver (or from its emulation in DOSBox-X) with 4096 Hz buffers, not from the game [hypothesis: on a real SB16 with the same driver it would be the same; there is no way to check here]. In the code, `_myBeginSound` has no minimum size; the header of an effect that did not play is not released (no `MM_WOM_DONE`), so the next request does `waveOutReset` and plays normally.
3. **Short glitch:** in several effects there are ~10 ms in the middle where the recording does not follow the PCM (0x2C near 186 ms, 0x25 near 192 and 212 ms), the same in the controlled test [hypothesis: the driver switching DMA blocks]. The port does not reproduce it.
4. **Preemption:** a new effect cuts off the one playing (measured: 0x25 of the fight, 0x24 of the lawnmower, 0x2C, 10); the previous one stops **~35 ms before** the new one sounds (in the fight, each 0x25 plays 126 ms of the 162 ms until the next). The duration of the effect that cuts off another does not always follow the block rule: the last 0x25 of each fight plays 1512 samples (the rule), but some 0x24s of the lawnmower stop at the end of the previous effect's block (363.18 s: 539 samples, ending at 3 × 504 counting from the 0x24 195 ms earlier), and the only 0x23 heard (484 samples, smaller than a block) played whole while cutting off a 0x24 [open: the driver's exact rule with DMA already running]. Effect 0x29 (rain, 2.6 s) stopped 4 times between 126 and 475 ms, twice with no audible effect afterwards [hypothesis: cut off by a request for a short effect, which does not sound but does the `waveOutReset`].
5. **Same-millisecond discard:** does not show up (there is no way to see in the recording a request that did not sound).

### Table per effect

| Effect (who requests it) | Times | Samples played / of the effect | How it ended | RMS level (dBFS) | Effect − music (dB, median) |
|---|---|---|---|---|---|
| 1 (error: impossible order, empty bookmark) | 4 | 504 / 752 | block of 504 | −22.0 | +10.0 |
| 9 (Experimental insecticide) | 7 | 504 / 800 | block of 504 (4); others under the music | −25.9 | +5.5 |
| 10 (burp: spider or cheat; 13 in 2.5 s, 471–474 s) | 13 | 2520 / 2706 (the last one) | cut off by one another (137–327 ms) | −21.1 | +6.7 |
| 0xD (cat) | 5 | 2016 / 2016 | whole (4) | −26.9 | +7.4 |
| 0x18 (boy inside the house) | 9 | 1008 / 1480 | block of 504 (7) | −24.8 | +7.7 |
| 0x1E (pebble) | 2 | 490 and 350 / 962 (504 expected; the weak ending stays under the music) | block of 504 [likely] | −22.9 | +7.9 |
| 0x20 (food: Mystery/Experimental) | 4 | 3024 / 3374 | block of 504 (3) | −24.3 | +15.8 |
| 0x23 (boy's footstep) | 1 | 484 / 484 | whole (cutting off a 0x24) | −23.2 | +10.2 |
| 0x24 (lawnmower) | 39 | 1512 / 1864 when free | cut off by the next (64–390 ms) or block | −29.8 | +4.4 |
| 0x25 (fight; antlion frames) | 20 | 1512 / 1888 | cut off by the next (9); block (10) | −27.1 | no music in the fights |
| 0x26 (antlion) | 1 | 2520 / 2880 | block of 504 | −22.5 | no music |
| 0x29 (rain) | 4 | 630–2380 / 12 896 | stopped (2), cut off by 0x2C (2) | −27.2 | — |
| 0x2C (eating; spider chewing) | 25 | 1512 / 1824 | block (10), cut off (8), stopped (7) | −26.9 | +5.8 |
| 0x31 (fight shock / birth) | 2 | under the music | — | −23.5 | +13.0 |

**Not heard:** 15, 0x11, 0x12, 0x13, 0x1C, 0x1D, 0x28 and 0x2F (smaller than a block: the driver does not play them); 2, 7, 0x15–0x17 (barks), 0x1B, 0x2A, 0x2D, 0x2E, 0x37 (the script did not request them or the random draw did not bring them: no bark or bird in the recorded Yard View; no spider dialog).

### Timeline (measured facts)

- **Fights:** 199.95 s: 0x25 at 0, 332, 495, 656 and 818 ms (the 2nd frame without sound: `S2` = 0); 206.64 s: 0, 174, 336, 660, 822 ms; 217.66 s: 0, 174, 813 ms. Interval between frames: 161–174 ms, **average 164.8 ms** over 11 intervals (the port uses 165 ms; the first interval sometimes 174 ms, the clock's 55 ms step). Each 0x25 is cut off by the next; the last one plays 1512 samples. What comes after the fight (0x2C, 0.4–1.4 s later) does not fall in the same millisecond.
- **Antlion** (403.48 s): 0x26 and, **562 ms later**, the first 0x25; then 0x25 every 1.08–1.18 s (two Mac frames of 30) until 5.03 s, 5 times. The first 0x25 comes at the normal interval of the loop (550 ms), not after the sound (0x26 lasts 576 ms at 5000 Hz and plays 504): `_mySoundIsDone` returns true immediately in multimedia mode. The port waited 703 ms (the sound at 4096 Hz) before the loop, and the first 0x25 came out ~1.27 s after the 0x26.
- **Lawnmower** (354.9–363.2 s): 39 × 0x24, one at each redraw of the yard (intervals of 64 to 390 ms, most 160–250), each one cutting off the previous, mixed with 0x2C and one 0x23.
- **Yard View** (335–364 s): 0x29 (rain), 0x2C, 0x18 (boy at home) and 0xD (cat) interleaved, every 0.2–0.8 s.

### Comparison with the port and fixes

| Script situation | Port requests | Original in the session |
|---|---|---|
| Walking and digging in the nest | 0x13, 0x11/0x12, 0x1E | only 0x1E (2 times); 0x11/0x12/0x13 never (smaller than a block) |
| Picking up/dropping food, egg | 0x1D, 0x1C | never (smaller than a block) |
| Eating | 0x2C | 25 times |
| Recruiting/releasing | songs 11013/11014, 11016/11017 | `call2arm` and `victqndd`, 2 times each |
| Alarm, Home, switching bodies | 0xF (1 if it fails) | 0xF never (102 samples); 1 on failures |
| Fights, death, rebirth | 0x25 per frame with `S2` ≠ 0, 0x31; songs `sad*`, `newbody` | the same |
| Yard View | 0xD/0xE, 7, 0x15–0x17, 0x18, 9, 0x23, 0x24, 0x29 | 0xD, 0x18, 0x23 (1), 0x24, 0x29 |
| Spider, antlion | 0x2F, 0x2C, 10; 0x26 + 0x25 | 0x2C, 10; 0x26 + 0x25 (0x2F never: 466 samples) |
| Mystery, Experimental | 0x20, 9 and others | 0x20, 9 |

No effect the original played was left without the corresponding request in the port. **Fixed in the port (2026-10-19, API 0.42):** `data/som.json` → `taxa_efeitos` 4096 → **5000** and the new key `bloco_efeitos` = **504** (`simant/src/som/mixer.rs::blocos_do_driver`: only whole blocks; an effect smaller than a block is muted, but still cuts off the one that was playing, like the original's `waveOutReset`); `--exportar-som` writes the effects at 5000 Hz and, with the key, also `efeitos_tocados/` (the stretch the driver plays); `mensagens.json` → `quadros_animados.formiga_leao.espera_som_ms` 703 → **0**. Tests: `driver_toca_so_blocos_inteiros`, `efeito_mudo_ainda_corta_o_anterior`, `efeitos_soam_como_na_sound_blaster_16` (with the user's files: 1512, 504, 2520, 3024, 2016 samples and the short ones muted) and `formiga_leao_toca_o_som_e_troca_de_quadro_a_cada_30_do_mac`. To play the effects as the code requests, without the driver: `taxa_efeitos` 4096 and `bloco_efeitos` 0.

**Did not change:** volumes (they already matched), the fight timeline (165 ms), same-millisecond discard (not observable). **Not reproduced:** the ~10 ms glitch in the middle of some effects; the exact duration of an effect that cuts off another (the port applies the block rule in that case too) and the short 0x23 that sounded when cutting off a 0x24 [open].

## The original's audio on the QEMU bench (2026-10-18)

Attempt to record the original's sound to compare with `--exportar-som`. Tool: `tools/som_original.py` (`boot`, `latido`, `chuva`, `luta`, `analisar`, `procurar`). **Result: the test bench records audio, but what comes out is not usable as a reference; the comparison by ear remains open.**

**Setup (works).** The test bench's copy of WfW 3.11 already has the Sound Blaster 16 drivers (`SYSTEM.INI`: `Wave=sb16snd.drv`, `Midi=sb16fm.drv`, `Midi1=sb16snd.drv`, `device=vsbpd.386`; `[sndblst.drv]` Port=220, Int=7, DmaChannel=1, HDmaChannel=5). The QEMU in `referencia/qemu` has `sb16`, `adlib` (OPL2) and the `wav` backend: `-audiodev wav,id=som,path=X.wav -device sb16,audiodev=som,irq=7 -parallel none` (the parallel port occupies IRQ 7), plus `-device adlib,audiodev=som` for FM (`SIMANT_SOM_OPL=1`). The machine changes, so a full boot is needed (the test bench's snapshots do not load) and a snapshot of its own (`som`). In Quick Game set up this way, `_songsOnFlag` = `_effectsOnFlag` = 1 and the dog placed sitting (`_DogDir` = 4) barked ~6 times in 40 s (`_DogBarkDelay` renewed: 6270, 6416, 6726, 6861, 7308, 7675).

**Why it is not usable (observed):**

1. **Music (FM).** `sb16fm.drv` is an OPL3 driver; QEMU's `adlib` is OPL2 and treats 0x38A/0x38B as 0x388/0x389, so the writes to the 2nd bank fall into the 1st. With `adlib`, the WAV has continuous sound (RMS ≈ 0.19) from the loading of Windows, before the game, and throughout the Quick Game, unrelated to `gamethme`: stuck notes. Without `adlib`, no music plays (the boot WAV is empty). There is no OPL3 in QEMU.
2. **Timing.** The `wav` backend only records while some voice is active: without `adlib`, the silences between effects disappear (3 s of audio for ~45 s of play with 3 Home presses and ~6 barks). Order and duration over time are not recoverable; with `adlib` active the time is continuous, but under host load the WAV comes out shorter than the VM time (52 s recorded in a boot of ~150 s).
3. **Content of the effects.** The port's effects (type 5 decoded, `--exportar-som`) **do not appear** in the recording: the normalized correlation (`som_original.py procurar`, resampling to 4096, 5000, 5512 and 6000 Hz and, for effect 15, from 3500 to 12000 Hz) stays below 0.5 for barks 21–23 (effects of a few tens of ms give peaks of up to 0.7 by chance). Each Home produces a stretch of 0.8–1.1 s, when effect 15 has 102 samples (25 ms at 4096 Hz); the breaks in the wave (QEMU's linear interpolation) come every 7–8 samples of 44 100 Hz, that is, a source of ~5.5–6.3 kHz and not 4096 Hz. [hypothesis] `sb16snd.drv` raises the rate to the card's minimum (5000 Hz) and QEMU's `sb16` plays the whole DMA buffer (old data) instead of the requested block; not isolated. The rain (effect 41, 3.1 s; `_GlobalKey` = 0xA8, `RainOn` = 1, `WaterTable` 62 → 48 in 25 s) generated no WAV at all.

**Was still unverified:** timbre, effect/music relative volume, the fight sounds over time and same-millisecond preemption. The capture through DOSBox-X (section above, 2026-10-19) resolved, for the intro, tempo, notes, relative volume and a timbre reference; the in-game effects (real rate on the SB16, fight, preemption) were measured in the session played by the user (section "In-game effects, recorded in a session played by the user"). The "source of ~5.5–6.3 kHz" seen here was the SB16 playing at 5000 Hz.
