*Leia em português: [conselhos.md](../../pt-BR/spec/regras/conselhos.md)*

# Advice and tutorial: `_Feedback` (`1030:78ca`)

`_Feedback` runs every tick (step 21 of [simulacao-visao-geral.md](simulacao-visao-geral.md) §5) and also inside `_FullCount` (phases 0 and 2, every 64 ticks). It has two parts: the **tutorial** (only in the Tutorial, `_CurGameType = 0`) and the **advice message** of the message ribbon (all game types, if the option is turned on).

Evidence convention: **fact seen in the code** = Ghidra pseudocode (`re_trabalho/decomp/Code7.c`, `Code1.c`, `Code5.c`) with address and name from `SIMANTW.SYM`; texts read from the decompressed `SHARED` bank (`re_trabalho/dump2/SHARED/t04_*.bin`); **[hypothesis]** as in the [README](../README.md).

## 1. Variables

| Symbol | Address | In the save | Use |
|---|---|---|---|
| `_TutorCnt` | `8:85ec` | yes | tutor counter, 0..0x400 |
| `_TutLesson` | `8:85ee` | yes | current lesson (1 = first) |
| `_TutGiven` | `8:85f0` | yes | the current lesson has already been shown |
| `_TutHowLong` | `9:7d62` | yes | how many times (every 8 calls) the lesson has been awaited |
| `_LessonTemp` | `9:9fb0` | yes | lesson memory (position, view, tasks, result) |
| `_TimeTemp` | `9:9b10` | **no** | 32 bits: deadline on the Mac clock (1/60 s) or digging milestone |
| `_OptionStates[4]` | `8:85fa` | yes | turns on the advice message ([hypothesis]: advice/messages menu item) |
| `_AdviceStrs` | `9:7c94` | no | SHARED string list 4/1020, loaded by `_initStuff` (`_LoadStringAnt(0x3fc)`) |

## 2. Flow of `_Feedback`

**Fact seen in the code.**

```
if _CurGameType == 0:                                     (tutorial; the same body as _RunTutor 1030:7a90)
    _TutorCnt += 1; if _TutorCnt > 0x400: _TutorCnt = 0
    if _TutorCnt & 7 == 0:
        _TutHowLong += 1
        if _TutGiven == 0:
            _GiveLesson(_TutLesson); _TutGiven = 1; _TutHowLong = 0
        else if _LessonDone(_TutLesson):
            _TutLesson += 1; _TutGiven = 0                  (the next lesson comes out 8 calls from now)
        else if _TutHowLong ≥ 16 and clock > _TimeTemp:
            _TutGiven = 0                                   (repeats the same lesson)
if _OptionStates[4] == 0: END
if _MeMode == 0 and (_MeHealth < _MeWarnHealth or _MeHealth < 10):
    advice = (_MeHealth < 10 ? no. 0 (important) : no. 1)
else if _HealthB < _BlkWarnHealth: advice no. 2
else if _BpopT ≤ _TilesDugB: _SetDefaultWindPrompt(0) (default window text, according to the tool); END
else if _MeMode == 0 and _MeNestStarted == 0 and _TilesDugB == 0: advice no. 3
else: advice no. 4
_EditMessage(advice text, duration 0x78, important)
```

Texts of list 4/1020 (fact verified in the bank):

| no. | Text | When |
|---|---|---|
| 0 | "Your health is critical - You must eat" | yellow (ant) with health < 10; shown even with another message on the ribbon |
| 1 | "Your health is low - Find some food" | health < `_MeWarnHealth` (30 at the start) |
| 2 | "Your colony needs food" | `_HealthB` < `_BlkWarnHealth` (30) |
| 3 | "You need to dig a nest" | colony larger than the digging, yellow ant without a nest and nothing dug |
| 4 | "Your nest needs expansion" | `_BpopT` > `_TilesDugB` in the other cases |
| 5 | "The red ants are attacking!" | not used here (`_GstrR`, [simulacao-visao-geral.md](simulacao-visao-geral.md) §7) |

`_EditMessage` (`1000:…`, Code1) only replaces the ribbon message when the request is "important" or when the option is on and the ribbon is empty; the removal deadline is reset on every call (with the same text, the message stays as long as the condition lasts). It is interface: it does not change the simulation state nor make random draws.

## 3. Lessons: `_GiveLesson(n)` (`1030:7b4e`) and `_LessonDone(n)` (`1030:7f02`)

**Fact seen in the code.** Each lesson shows a box (`_PictStrnDialog(image, text, 1)`) with the SHARED text 4/`11000 + 2(n − 1)` (exceptions in the table) and sometimes performs an action. The lesson ends when the condition holds. Texts: 50 objects 4/11000…11112 in the bank (e.g.: 11000 "Welcome to the SimAnt Tutorial…", 11004 "Try double-clicking somewhere in the black dirt near the yellow ant.", 11112 "This is the end of the SimAnt tutorial…").

Actions when giving the lesson:

| Lesson | Action |
|---|---|
| 3, 4 | `_TimeTemp = _TilesDugB + 2` (3) or `+ 10` (4): digging milestone |
| 7, 0x1A | `_LessonTemp = _MeLocY + _MeLocX`; `_TimeTemp = clock + 600` |
| 9 | `_LessonTemp = _MapPnt.y + _MapPnt.x`; `_TimeTemp = clock + 600` |
| 0x1B | `_LessonTemp = _MeLocY + _MeLocX` |
| 0x29 | `_LessonTemp = _modeLevels[1] + _modeLevels[0]` |
| 8, 0x1E, 0x2E, 0x32 | `_LessonTemp = 0` |
| 0xE | `_SetMyHealth(8)` (the yellow ant becomes hungry, [formiga-amarela.md](formiga-amarela.md)) |
| 0x17 | `_AddBlackAnts(10)` **before** the box ([geracao-mundo.md](geracao-mundo.md): draws `S(10)` and `S8()` per ant) |
| 0x31 | box; `_AddBlackAnts(80)`; clears the whole `_PherMapA` |
| 0x2B | `_TutLesson += 1` and shows the text of lesson 0x2C (11086, image 0x4271): text 11084 never appears |
| 0x37 | text 11108 if `_LessonTemp == 0`, otherwise 11110 ("Too bad. Your attack on the red ants has failed…") |
| 0x38 | text 11112 |
| 0 and > 0x38 | nothing (end of the tutorial) |

Images (first argument; 0 = none): 2 → 17000, 5 → 0x4269, 0xB → 0x4276, 0xD and 0xE → 0x426A, 0x11 → 0x4275, 0x12 → 0x426B, 0x15 → 0x426C, 0x21 → 0x426D, 0x24 → 0x426E, 0x26 → 0x426F, 0x27–0x29 → 0x4270, 0x2B and 0x2C → 0x4271, 0x2D and 0x2E → 0x4272, 0x2F and 0x30 → 0x4273, 0x32 → 0x4274. [open: which resource these ids refer to; they are not among the SHARED type 10 objects.]

Completion conditions (`_LessonDone`):

| Lessons | Ends when |
|---|---|
| 1, 2, 6, 0xB, 0xF, 0x11, 0x17, 0x19, 0x1C, 0x1D, 0x1F, 0x22, 0x23, 0x25, 0x28, 0x2C, 0x2D, 0x30, 0x33, 0x35, 0x37, 0x38 | always (text only) |
| 3, 4 | `_TilesDugB > _TimeTemp` and `_MeMoveMe == 0` |
| 5, 0x14 | `_MePlane == 1` (surface) |
| 0x12 | `_MePlane == 2` (black nest) |
| 7, 0x1A | the position changed (`_MeLocY + _MeLocX ≠ _LessonTemp`), `_MeDis == 0` and clock > `_TimeTemp` |
| 9 | the view changed (`_MapPnt.y + _MapPnt.x ≠ _LessonTemp`), **or** clock > `_TimeTemp` |
| 0xA, 0x20, 0x2A | window 0x100 in front (`_win_IsWinInFront`) |
| 0xC, 0x31 | window 0 in front |
| 0x27 | window 0x1200 in front |
| 0x2F | window 0x1300 in front |
| 0xD | yellow ant on the surface over a tile > 0x47 |
| 0xE | `_MeHealth > 90` |
| 0x10 / 0x13, 0x16 / 0x15 | `_MeType == 0x18` / `0x10` / `0x28` |
| 0x18 / 0x34 | `_ModePopB[5]` (`9:9e7a`) > 1 / > 40 |
| 0x1B | the position changed, `_MeDropAlarm ≠ 0` and `_MeDis == 0` |
| 0x1E | (if `_MeDropAlarm ≠ 0`, turns off the alarm: `_SetAlarmDropState(0, 1)`) and `_LessonTemp ≠ 0` |
| 8, 0x2B, 0x2E, 0x32 | `_LessonTemp ≠ 0` (set by player actions, e.g.: `_ExchangeLives` sets 1) |
| 0x21 / 0x26 | `_MapPlane == 0` / `== 1` |
| 0x24 | `_YardMode == 1` |
| 0x29 | `_modeLevels[1] + _modeLevels[0] ≠ _LessonTemp` |
| 0x36 | `_MePlane == 3` (sets `_LessonTemp = 0`) or `_MePlane == 2` (sets 1) |
| others | never |

The comparisons with `_TimeTemp` are signed 32-bit. Lessons 3 and 4 use `_TimeTemp` as a digging milestone: in `_Feedback`, "clock > `_TimeTemp`" is then always true and the lesson is repeated every 16 × 8 calls.

### 3.1 Checked against the original: the whole tutorial (2026-10-18)

**Verified** on the QEMU bench (`python tools/cenarios_tick.py gravar tutorial_completo DIR --snapshot tut`, copy of the overlay `ov_tut.qcow2`): the script performs the action each lesson asks for and records in full the ticks in which `_TutorCnt` reaches a multiple of 8. All 56 lessons were given and completed, from lesson 1 (tick 8) to the end (`_TutLesson` = 57 at tick 1632, with nothing to show). Two recordings: 124 ticks (lessons 1–26) and 185 ticks (lessons 1–57); all 307/307 in the port, except the three cases explained in [testes-tick.md](testes-tick.md) ("Fifth round"). 16 of them are in `tests/referencias/ticks/tutorial_completo/`.

Actions used (what the lesson text asks for; bench coordinates, Tutorial edit window with 39 × 21 tiles starting at (0, 128)): double click on the dirt near the yellow ant (3, 4); double click on row 0 above it (5; for 20 with a single click on the 0x18 opening); single click on the ground (7, 26); click on the yellow ant itself (8, 50: `_LessonTemp` = 1 from the centering); none (9: the 600 deadline expired); Ctrl+M (10, 32, 42), Ctrl+E (12, 49), Ctrl+Y (33), Ctrl+5 (36), Ctrl+8 (38), Ctrl+B (39), Ctrl+C (47); single click on a food item (13) and double click on another (14 eat, 16 pick up); click on the black hole (18) and double click on the tunnel (19, drops the food); double click on a pebble (21) and on the ground (22); key 2 (24) and five times (52); key 0 and walk (27); X and click on a black ant (30); drag the circle of the Behavior triangle (41); click held for 1.5 s on the bar chart (46: `_LessonTemp` = 1); click on the red hole (54: entered the red nest, `_LessonTemp` = 0 and lesson 55 showed text 11108).

Observed, beyond what the table above already says:

- **The deadline of lessons 7, 9 and 0x1A counts from the closing of the box.** `_GiveLesson` calls `_PictStrnDialog` (modal) **before** reading `_MacTickCount` for `_TimeTemp` (seen in the code, `1030:7c4d`/`7c5e`; the same holds for all actions: the box comes first, except in 0x17, where `_AddBlackAnts` comes before). On the bench: lesson 7 given at tick 291 with the clock at 10704 at the start of the tick; the box stayed open until 12213, and `_TimeTemp` = 12813 = 12213 + 600. Lesson 9 (tick 448): 15090 + 600 = 15690. The port set `_TimeTemp` = tick clock + 600; now the interface resets the deadline when the box is closed (`MemoriaTutor::caixa_fechada`, below).
- **Lesson 0x2B skips to 0x2C in the same tick** (tick 1245: `_TutLesson` 43 → 44 with `_TutGiven` = 1, box 11086), as in the table.
- **Lesson 0x29** stores `_modeLevels[1] + _modeLevels[0]` as signed 16-bit (13107 + 39321 = −13108 at tick 1151).
- **Window conditions:** lessons 0xA, 0xC, 0x20, 0x27, 0x2A and 0x2F completed with the requested window in front (Ctrl+M, Ctrl+E, Ctrl+B, Ctrl+C). 0x2A (back to the map) did **not** complete with the ribbon's Patch button, with Behavior open and the House View underneath: the map swaps places with the yard (`_YardToMap`), but does not come to the front; only Ctrl+M completed it. After Ctrl+Y/Ctrl+5 with the yard open, Ctrl+M brings the **yard** (0x1900) to the front, not the map (`_OpenMapYard`, mapa.md). The window in front is not in the game's memory: the script notes which one it put in front (`JanelaNaFrente` in the extras) and the note was checked by the result (the lesson completes or not in the original). 0x31 (window 0 after `_AddBlackAnts(80)`) completed with Ctrl+E.
- `_TutorCnt` goes from 0x400 to 0 (the multiple-of-8 boundary also happens at 0x400 → 0) and advances by **2** in the tick in which `_FullCount` calls `_Feedback` again (`_Cycle` a multiple of 64): tick 575, `_TutorCnt` 583 → 585.

## 4. Test cases

| Case | Input | Expected |
|---|---|---|
| C-U1 advice | `_OptionStates[4] = 1`, `_MeMode = 0`, `_MeHealth = 5` | advice no. 0, important |
| C-U2 | health 20 (< 30) | no. 1 |
| C-U3 | health 80, `_HealthB = 10` | no. 2 |
| C-U4 | `_BpopT = 5`, `_TilesDugB = 10` | default window text |
| C-U5 | `_BpopT = 5`, `_TilesDugB = 0`, `_MeNestStarted = 0` | no. 3; with `_MeNestStarted = 1`, no. 4 |
| C-U6 option off | `_OptionStates[4] = 0` | none |
| C-U7 tutor | Tutorial, `_TutLesson = 1`, `_TutGiven = 0`, `_TutorCnt = 7` | message 11000; `_TutGiven = 1`; `_TutHowLong = 0` |
| C-U8 tutor | lesson 1 given, 8 calls later | `_TutLesson = 2`, `_TutGiven = 0`; 8 more → message 11002 |
| C-U9 lesson 5 | `_MePlane = 2` | does not end; `_MePlane = 1` ends |
| C-U10 lesson 0x2B | give lesson 0x2B | message 11086 and `_TutLesson = 0x2C` |

## Implementation in the port

- `engine/src/rules/conselhos.rs`: `conselho(&Estado, &TabelasConselhos) -> Conselho` (the ribbon message as **derived state**: the interface queries it after each tick; it does not become an event, because the original redoes it every tick), `executar_tutor(e, tabelas, tabelas_amarela, memoria, interface, relogio)` and `impl RegrasConselhos for RegrasOriginais`.
- The lesson boxes come out as `Evento::Mensagem(texto)`; the interface finds the image through the lesson that uses that text (`imagem_do_texto` in `engine/src/core/avisos.rs`, with `data/quintal.json` → `conselhos.licoes[].imagem`; the other texts use `mensagens.json` → `quadro.imagens`).
- Deadline of lessons 7, 9 and 0x1A (§3.1): `dar_licao` sets `tempo_limite` = tick clock + wait and stores the wait in `MemoriaTutor::espera_depois_da_caixa`; `Partida::fechar_modal` calls `MemoriaTutor::caixa_fechada(clock now)`, which resets `tempo_limite` = clock + wait (like the original, which reads `_MacTickCount` after `_PictStrnDialog`). Without an interface (vector without the reading, advancing without boxes), the tick deadline stays. In the vectors, `vetor_tick::depois_do_tick` does the same with `MacTickCount_depois_da_caixa`.
- `_TimeTemp` is not in the save nor in `Estado`: it lives in `MemoriaTutor`. Clock: the same as the yard's (`_GameTime × mac_por_tick`, [quintal.md](quintal.md), "Implementation in the port"). Windows in front: trait `InterfaceTutor`; `RegrasOriginais` keeps `memoria_tutor` (`MemoriaTutor`, persists from one tick to the next, like the original's global) and `interface_tutor` (`JanelasNaFrente`: the interface writes there the windows that are in front; empty = none). The lesson that changes the yellow ant's health uses the `_SetMyHealth` of `amarela.rs`, with `data/amarela.json`.
- Data (`data/quintal.json`, `conselhos` block): list 4/1020, the critical-health threshold of 10, the 16 waiting cycles and the lesson table above (text, image, action, condition), editable by mods.

## Gaps

- The interface writes `RegrasOriginais::interface_tutor` every frame with the top window, in the `_win_IsWinInFront` format (window number << 8; `simant/src/sessao.rs`, ever since the control, map and yard windows exist). The window conditions of lessons 0xA, 0xC, 0x20, 0x27, 0x2A, 0x2F and 0x31 were checked against the original (§3.1) with the window noted by the bench; it was not checked whether the `JanelasNaFrente` written by the port's interface matches the original's in every situation (e.g.: Ctrl+M with the yard open brings the yard). `_TimeTemp` does not go into the save (as in the original): loading a game clears the memory.
- `_SetDefaultWindPrompt` and the default ribbon text per tool: interface, not specified here.
- Cases from the original: the vectors `tests/referencias/ticks/tutorial/` (snapshot `tut`, lessons 1–3: lesson 1 completed and lesson 2 requested at tick 15, lesson 3 given at 23 and waiting for the digging milestone at 46–47, with `_TimeTemp` in `extras.json`) and the Load of `Tutorial.Ant` (`tests/referencias/load_tutorial/`) match the port; the whole tutorial was checked on 2026-10-18 (§3.1, `tests/referencias/ticks/tutorial_completo/`). The advice cases C-U1..C-U6 (message ribbon) are still checked only against the code; C-U8..C-U10 happened in the original during the recording (C-U7, lesson 1, had already been given in the snapshot). The lesson box was compared with the port in a single capture (lesson 14, tick 622): the text and the image come out the same (blue text mask 99.6%, difference under the cursor), and, since 2026-10-18, also the frame (dark blue with 1 black pixel, `mensagens.json` → `quadro.cor_borda`; before, the port made it light blue) and the position (centered in the area below the ribbon in both) — telas/mensagens.md §3.3.
