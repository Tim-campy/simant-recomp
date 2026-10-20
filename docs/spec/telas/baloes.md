*Leia em português: [baloes.md](../../pt-BR/spec/telas/baloes.md)*

# Speech balloons in the edit window ("Silly" option)

With Options → Silly on (`_OptionStates[5]`, `8:85fc`), the edit window shows speech balloons over fights, resting ants, the queen, the eggs and the spider. Source: Ghidra pseudocode (addresses below) [code]; text lists read from the database (`inspecionar strings`) [data]. Bench check: see "Verification".

Implementation: `engine/src/rules/baloes.rs` (annotation in the tick, redraw random draws, balloon choice), `engine/src/core/baloes.rs` (shape, `_MakeBalloon`), `engine/src/core/tela_jogo.rs` (`desenhar_balao`). Data: `data/baloes.json` (lists, waits, chances, geometry) and the 8×8 pieces of `data/quintal_mapa.json` (`balao_menino.pecas`, the same as the boy's balloon).

## 1. In the tick: where there is a balloon

Step 3 of `_DoAntSim` (`1028:0000`), with Silly = 1: zeroes `_FightBalloonCnt`, `_RestBalloonCnt`, `_QueenBalloonCnt`, `_EggBalloonCnt` and sets the last points to −1 (`_LastFightPnt`, `_LastRestPnt`, `_LastQueenPnt`, `_LastEggPnt`, x and y) [code]. The `_Last*Plane` planes remain.

Who annotates (always checking Silly first, like the original; none of them does a random draw) [code]:

| Routine | Called by | Condition | Plane |
|---|---|---|---|
| `_FightBalloons(x, y, p)` (`1010:499a`) | `_DoFightA` (`1028:27e6`); `_DoNestFightB/R` (`1028:3a54`/`6072`) | the fight cloud continues (the `SRand16` did not give 0) | 1, 2, 3 |
| `_RestBalloons(x, y, p)` (`1010:4aee`) | rest on the surface (`_DoRestAnt` `1028:0b76` and the same snippet in `_DoAntSimA`); rest in the nests (`_DoRestB/R` and the snippet in `_DoNestAntB/R`) | the ant keeps resting (`SRand4` ≠ 0 on the surface, `SRand1(20)` ≠ 0 in the nests) | 1, 2, 3 |
| `_QueenBalloons(x, y, p)` (`1010:4a44`) | `_SimQueenB/R`, queen's head | at the end, after the tail check (not when the queen moved nor when she starved) | 2, 3 |
| `_EggBalloons(x, y, p)` (`1010:48f0`) | `_SimEggB/R` | the egg did not hatch this tick (a black egg that died counts) | 2, 3 |

Each one does [code]: if the type's count is still 0 and the cell is in view (`_BalloonIsVisible`, below), then, if (x, y, p) is the current balloon (`_CurFightPnt`/`_CurFightPlane` etc.), the count becomes 1 (the balloon continues); otherwise (x, y, p) becomes the last point. After count 1 the following calls in the same tick change nothing.

`_BalloonIsVisible(p, x, y)` (`1010:48a8`): `p = _MapPlane`, `_MapPnt.x ≤ x < _MapPnt.x + _editWidth` and `_MapPnt.y ≤ y − 3 < …`, `y < _MapPnt.y + _editHeight`: the cell needs at least 3 rows of the view above it (room for the balloon) [code].

## 2. On the edit window redraw: random draws

`FUN_1010_16d4` (the redraw of the edit window buffer: after each tick in `_UpdateEdit`, on scrolling and in `_UpdateEditIfBufInvalid`) does, in this order: `_PreDrawSpider`, **`_DrawCurBalloons`**, the cells, `_DrawSpider` with the spider visible (palps and **spider balloon**), **`_DrawBalloons`** [code]. The two marked routines consume the S generator; the deadlines are in `_TickCount` (`1008:18ac` = `GetTickCount() / 55`, real time) [code].

### 2.1 `_DrawCurBalloons` (`1010:4cac`)

Only with Silly ≠ 0 and `_ELayerMode` = −1 (no scent layer). For each type, in the order **fight, egg, queen, rest** [code]:

1. **New balloon:** count 0 and last point ≥ 0 (x and y) → the last point and plane become the current ones, the count becomes 1 and the deadlines of the type's lines go to 0. For egg, queen and rest the line is also turned off; for fight, it is not.
2. With count > 0 and the game **not paused** (`_GamePaused` = 0), for each line of the type whose deadline has expired (deadline < `_TickCount`, 32 bits): deadline = `SRandK` + `_TickCount` + wait; then `SRandC` = 0 turns the line on and advances the line index (back to 0 when the list's next pointer is null), ≠ 0 turns it off.
3. Each line that is on requests the balloon (`_AddMsgBalloon`, inlined): point = current cell × 16 + 8 (middle of the tile), current plane, text = the index's entry in the list; up to 6 requests and only if the point's cell is in view (the same computation, with the point divided by 16).

| Line | Type | List | Deadline `K` | wait | Chance `C` |
|---|---|---|---|---|---|
| fight, black | fight | 1220 (8 lines: "Take that\nyou fiend!"…) | 32 | 60 | 2 |
| fight, red | fight | 1230 (8: "Your queen mates\nwith termites!"…) | 32 | 60 | 2 |
| egg | egg | 1240 (15: "Where am I?"…) | 32 | 170 | 4 |
| queen | queen | 1250 (8: "Being a queen is\nterribly overrated!"…) | 32 | 180 | 4 |
| rest | rest | 1260 (8: "Whew! I'm\nexhausted!"…) | 64 | 120 | 2 |

Fight has two lines (one balloon of each color, each with its own deadline), both drawn before the requests. With a new balloon the deadlines are 0, so the random draws happen in the same redraw. S consumption per redraw: 2 per line with an expired deadline (`S32`/`S64` and `S2`/`S4`), nothing while the deadline has not expired. The lists are loaded in `_initStuff` (`_LoadStringAnt(0x4c4, 0x4ce, 0x4d8, 0x4e2, 0x4ec)`) [code]. The indices start at 0 and are never reset; since the index advances before being used, the first line shown from each list is the one at index 1 [code].

Finally, if there are requests and the buffer is valid, `_editForce = 1` and `InvalidateRect` (full window redraw) [code; in the port the whole frame is already redrawn].

### 2.2 Spider balloon (end of `_DrawSpider`, `1010:1d6a`)

Only with the spider visible (`_PreDrawSpider`) and Silly ≠ 0, after the palps [code]:

1. Not paused and with the deadline (`DGROUP:0x87d6`) expired: deadline = `SRand64` + `_TickCount` + 180; `SRand2` = 0 turns on and advances the index (`0x87da`, from 0 to 4: after 4 back to 0), otherwise turns off.
2. If `_SMode` differs from the last one seen (`0x72f4`) or is > 4: deadline = `SRand32` + `_TickCount` + 30, turns off, stores `_SMode` and stops. **This random draw happens even when paused**, and on every redraw while `_SMode` = 5 (spider dying).
3. On: text = entry `_SMode × 5 + index` of list 1210 (25 lines, 5 per mode: stopped, walking, chasing, eating, fleeing); point = (`_SpidX` + 8·`_Dx8[_SpidDir]`, `_SpidY` + 8·`_Dy8[_SpidDir]`) in map pixels, plane 1; request with the same view check (the surface must be the plane shown).

The spider balloon enters the list after those of `_DrawCurBalloons`.

### 2.3 `_DrawBalloons` (`1010:57aa`)

Text of each line (`_MakeBalloon`): starts at column x0 + 8 (the second column of pieces), shifted `((−width) & 6) / 2` pixels (`_font_MakeImage(texto, (−w & 6) >> 1)`, the text image goes to the mask's next byte); for the widest line this coincides with centering, the others end up almost left-aligned [code + bench, below]. Line i is at y0 + `font_height × i` + 6.

With font 2, it walks the requests and **draws only the first** that is in view (the loop stops after the first one drawn) [code]; clears the list. The balloon is made by `_MakeBalloon(texto, 0)` (left wedge; the same routine as the boy's balloon, telas/quintal.md §2.3) and copied with a mask into the edit window buffer with the corner at (x + 4, y − (height + 4)) in map pixels (x, y = request point; height = the one recorded by `_MakeBalloon`, `font_height × n + 15`, one less with a single line). The covered cells are marked for the next redraw, which erases the balloon if it is not requested again [code]. The balloon stays on screen until the next buffer redraw (when paused, it stays still).

## 3. Edit window health graphs (`_DrawEditGraphs`, `1010:198a`)

`_DrawEditGraphs` (called after each buffer copy, `_win_DrawEditWindow`, and on scrolling) draws three vertical bars in objects 17–19 of window 0 (`editHealthY`, `editHealthB`, `editHealthR`, in the icon column `editIcons`), with `_MeHealth`, `_HealthB` and `_HealthR` (pointers in table `8:8e00`, objects at `8:8dfa`), the mark of `_MeWarnHealth` (white line) and of `_BlkWarnHealth` (inverted line) [code]. **But the routine does nothing with `_displayType` 9 or 10** — the two modes whose graphics bank exists (`MWINNT` and `WINGANT`; the others are DOS leftovers, formatos/simant-cfg.md). Therefore, in SimAnt for Windows, the edit window **has no health graphs**: health appears only in the ribbon's bars (telas/mensagens.md §9). The port does not draw them, on purpose. [hypothesis: a leftover from the DOS and Mac versions. With the ribbon, window 0's icon column does not even appear: the map starts at the inner edge of the frame, telas/jogo.md.]

## 4. Port

- State: `Estado::baloes` (`rules::estado::Baloes`, outside the save): counts, last and current points, `_editWidth/_editHeight` (`vista`, set by the interface before the tick with `_MapPnt`), deadlines, lines on, indices, `_SMode` seen, the request list and the balloon drawn.
- Tick: `baloes::anotar` in the places in the §1 table (`rules/superficie.rs`, `rules/ninho.rs`).
- Redraw: `desenho::redesenho_do_timer` → `desenho::desenhar_edicao` (balloons, palps, spider balloon, choice), under the same conditions of the edit window redraw already modeled (rng.md, "Sorteios do desenho"); `_TickCount` = `ms_total / 55` of the game (`JanelasDesenho::tick_count`).
- Drawing: `TelaDeJogo::desenhar_balao` draws `Baloes::desenhado` with the pieces and the font, if the cell is still in view.

Gaps and hypotheses:

- The scroll redraw (`_DoEditScroll`, `FUN_1010_16d4` outside the timer) and those inside the tick (`_DoEditUpdateDraw`) do not do random draws in the port (like the palps) [gap].
- The line index is stored without wraparound in the port; the interface reduces it to the list size (equivalent to the original's wrap to 0 while the list does not change).
- On a Load, the deadlines and indices remain the session's (the original does not save them either) [code].

## Verification

The repository's tick vectors all have Silly = 0 (`_OptionStates` = 0,1,1,1,1,0 in the 150 states checked), so they do not exercise this part. Own check on the bench (2026-10-18, `tools/cenarios_tick.py gravar baloes`, snapshot `rico`): Silly written at the entry of a tick, view of the black nest with `_MapPnt` = (22, 0) (queen at (34, 6), eggs at (34, 8)), 60 intervals between the end of one tick and the entry of the next, with 1.5 s of free VM time between them; in each interval, the balloon variables (table below) and the S generator.

- **Random draws:** in the three intervals in which S advanced, the number of steps and the values match the §2.1 model: new balloon (count 0 → 1 for egg and queen, current points = last) with 4 steps, **egg before queen**, each `S32` + `S4`, and deadlines `5993 = 2 + now + 170` and `6003 = 2 + now + 180` with the same `now` (5821); expired egg deadline with `S32` = 7, `S4` = 2 (turns off: `_WantEggBalloon` 1 → 0) and deadline `6735 = 7 + 6558 + 170` (6558 > previous deadline 6557); another with `S4` = 0 (turns on, index 2 → 3). In the other 57 intervals, with no expired deadline, S did not advance.
- **Drawing:** with the egg and queen lines on, only the egg balloon appears (the first request); the first line shown is the one at index 1. The port (`balao_do_ovo_como_no_original` and `balao_da_rainha_como_no_original`, `simant/src/modos.rs`, with the states `tests/referencias/saves/BALAO_OVO.ANT` and `BALAO_RAINHA.ANT`) draws the balloon region **100%** equal to the captures (`comparar_captura.py --topo 38 --mapear`; egg: "Sure is dark\nin here!", queen: "Sometimes I think all I\ndo is lay eggs!"). The two captures settled the text alignment (previously centered [hypothesis]).

Not checked on the bench: fight, rest and spider balloons (same code, other constants), the pause and the spider's `_SMode` change.

Variable names (`SIMANTW.SYM`): `_FightBalloonCnt` `9:7404`, `_RestBalloonCnt` `9:a07e`, `_QueenBalloonCnt` `9:8122`, `_EggBalloonCnt` `9:7d44`; `_Cur*Pnt`/`_Cur*Plane` (`_CurFightPnt` `9:7a52`, `_CurEggPnt` `9:7d2a`, `_CurQueenPnt` `9:7d64`, `_CurRestPnt` `9:7d40`); `_Last*Pnt`/`_Last*Plane`; deadlines `_BFightBalloonTicks` `9:811e`, `_RFightBalloonTicks` `9:9e6a`, `_EggBalloonTicks` `9:a0e6`, `_QueenBalloonTicks` `9:78a8`, `_RestBalloonTicks` `9:9ce6`, `_SpidBalloonTicks` `8:87d6`; lines on `_WantBFightBalloon` `9:9bea`, `_WantRFightBalloon` `9:a0d4`, `_WantEggBalloon` `9:9afe`, `_WantQueenBalloon` `9:7c3c`, `_WantRestBalloon` `9:72be`, `_WantSpiderBalloon` `9:75f8`; indices `_BFightBalloonIndex` `9:7a50`, `_RFightBalloonIndex` `9:810e`, `_EggBalloonIndex` `9:9eaa`, `_QueenBalloonIndex` `9:78d6`, `_RestBalloonIndex` `9:9e86` (the spider's, `8:87da`, unnamed); `_LastSMode` `9:72f4`; request list `_CurBalloonCnt` `9:9fd0`, `_CurBalloonPnts` `9:80d6`, `_CurBalloonPlane` `9:7d68`, `_CurBalloonFlags` `9:78be`, `_CurBalloonMsgs` `9:7c24`.
