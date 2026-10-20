*Leia em português: [rng.md](../../pt-BR/spec/regras/rng.md)*

# Random number generators

SimAnt uses **two independent generators**:

| Name used here | Routines | State | Algorithm |
|---|---|---|---|
| **S** ("world seed") | `_SRand1`, `_SRand2` … `_SRand256`, `_SGRand`, `_SGIRand`, `_SGSRand`, `_SetSRandSeed`, `_GetSRandSeed`, `_SeedSRand` | 1 16-bit word at `DGROUP:0xcbf2` (no name in the `.SYM`) | 16-bit Galois LFSR |
| **R** ("C rand") | `_RRand`, `_SeedRRand` (and the runtime functions `_srand`/`_rand`) | 32 bits at `DGROUP:0xae34` (low word) and `0xae36` (high word), no name in the `.SYM` | Microsoft C runtime linear congruential |

Sources: Ghidra pseudocode (`re_trabalho/decomp/Code5.c`, `Code4.c`, `Code2.c`) and direct disassembly of the `SIMANTW.EXE` bytes (capstone, 16-bit mode) to check instructions that the pseudocode leaves ambiguous (signed or unsigned division, evaluation order).

## S generator (LFSR)

### Step

Fact seen in the code (`_SRand1` @ `1020:158a`, and the variants below, checked in the disassembly):

```
step(s):             // s = unsigned 16-bit word
    carry = bit 15 of s
    s = (s << 1) & 0xFFFF
    if carry: s = s XOR 0x1BF5
    return s         // the new state is also the drawn value
```

Properties (fact verified by enumerating the 65,536 states with a Python script):

- The step is a bijection. The cycles are: state 0 (fixed: with seed 0 the generator returns 0 forever), a cycle of 57,337 states, one of 8,191 and one of 7 (`0x3859 → 0x70b2 → 0xe164 → 0xd93d → 0xa98f → 0x48eb → 0x91d6 → 0x3859`). It is not a maximum-period LFSR.
- Bit 0 of the new state equals bit 15 of the previous state (because 0x1BF5 is odd).
- Test vectors: seed 1 → 2, 4, 8, 0x10, …; seed 0x1234 → 0x2468, 0x48d0, 0x91a0, 0x38b5, 0x716a, 0xe2d4, 0xde5d, 0xa74f, 0x556b, 0xaad6.

### Output functions

All of them take exactly **one** step, except the three "G" ones, which take two or three.

| Function | Address | Result | Note |
|---|---|---|---|
| `_SRand1(n)` | `1020:158a` | `step() mod n` | **unsigned** 16-bit division (`div`), result 0…n−1 |
| `_SRand2()` | `1020:15ae` | `step() & 1` | |
| `_SRand4()` | `1020:15ce` | `step() & 3` | |
| `_SRand8()` | `1020:15ee` | `step() & 7` | |
| `_SRand16()` | `1020:160e` | `step() & 0xF` | |
| `_SRand32()` | `1020:162e` | `step() & 0x1F` | |
| `_SRand64()` | `1020:164e` | `step() & 0x3F` | |
| `_SRand128()` | `1020:166e` | `step() & 0x7F` | |
| `_SRand256()` | `1020:168e` | `step() & 0xFF` | |
| `_SGRand(n)` | `1020:14a4` | `min(a, b)` with `a = SRand1(n)` and then `b = SRand1(n)` | distribution skewed towards 0 |
| `_SGIRand(n)` | `1020:147c` | `max(a, b)`, same order | no caller in the pseudocode |
| `_SGSRand(n)` | `1020:14cc` | `m = min(SRand1(n), SRand1(n))`; then `SRand2()`; if it gives 1, returns `−m` | signed value, concentrated near 0 |

Caution: `_SRandK()` (mask) and `_SRand1(K)` (remainder) give the same number for K a power of 2, but the code uses both forms; to reproduce the original it is enough that each call takes one step.

### Seed

- Initial value in the executable: `DGROUP:0xcbf2 = 0` (fact verified: bytes of segment 10 in the file, file offset `0x20000 + 0xcbf2`).
- `_SetSRandSeed(v)` (`1020:1506`) writes v; `_GetSRandSeed()` (`1020:1512`) reads it (returns DX = 0). Fact seen in the code.
- `_SeedSRand()` (`1020:152a`): `s = (_TickCount() & 0xFFFF) XOR 0x3751`. Fact seen in the code.
- `_TickCount()` (`1008:18ac`) = `GetTickCount() / 55` (unsigned long division by 0x37), that is, the Windows clock in "ticks" of ~55 ms, as on the Mac. Callers use only the low word. Fact seen in the code.

### Who seeds S

| Moment | Call | Source |
|---|---|---|
| Program startup | `_initStuff` (`1020:0000`) calls `_BootUpGame` (`1000:96b2`, which always returns 0) and then `_InitSimVars`, `_SeedSRand`, `_SeedRRand` | fact seen in the code |
| Generation of each yard patch | `_RandWorld` (`1030:5ab0`) starts with `_SetSRandSeed(patch seed)` | fact seen in the code |
| Tutorial loaded from `Tutorial.Ant` | `_SpecialTutorialInit` (`1010:9cea`) and `_LoadGame` (`1010:9fa0`) call `_GenerateTutorial` and then `_SeedSRand` | fact seen in the code |

Consequence: a patch's map is **a function of the patch seed only** (plus the yard state, see [geracao-mundo.md](geracao-mundo.md)); after generation, S continues from where the generation stopped and goes on to serve the simulation.

## R generator (Microsoft C `rand`)

Fact seen in the code (`_srand` @ `1018:06f6`, `_rand` @ `1018:070a`, checked in the disassembly):

```
32-bit state, initial = 1 (bytes 01 00 00 00 at DGROUP:0xae34)
srand(v):  state = v (high word zeroed)
rand():    state = (state * 0x343FD + 0x269EC3) mod 2^32
           return (state >> 16) & 0x7FFF
```

It is the classic Microsoft C runtime generator (multiplier 214013, increment 2531011).

- `_RRand(n)` (`1020:156e`): `v = rand()`; if `v < 0`, `v = −v` (never happens, `rand` is 0…0x7FFF); returns `v mod n` with **signed** division (`idiv`). Fact seen in the code.
- `_SeedRRand()` (`1020:1536`): does `s = TickCount XOR 0x3751` (same as `_SeedSRand`), draws `k = SRand128()` (advances S), calls `srand(TickCount)` (a new clock reading) and discards `k` values of `rand()`. Fact seen in the code.
- `_SetRRandSeed` (`1020:1518`) is empty. `_GetRRandSeed` (`1020:151a`) reads a double word at `0x046C:0000` (the linear address of the BIOS tick counter, 0040:006C, used as if it were a selector); it has no callers in the pseudocode. Both look like leftovers from the DOS port [hypothesis].

## Who uses each generator

Count of calls in the pseudocode (fact seen in the code; full list obtained with `grep` over `re_trabalho/decomp/*.c`):

- **R (`_RRand`)**: only 9 routines. `_RandYard` (seeds of the 192 patches), `_PlaceDrop` (position of the raindrops), `_SimRain`/`_DoSimYard` (random draw for the start of rain), `_GetWinner` (fights), `_MysteryButton`, `_CheatKeys`, `_AnimYellowInsane`.
- **S**: everything else. Map generation (`_MakeYardPatch`, `_AddRocks`, `_InitAntLions`, `_InitPillar`, `_RandWorld`, `_PlaceBlackQueen`, `_AddFood`, `_DigTileB/R` and `_SmoothEdgesB/R` via `_SRand8`), the ant simulation (`SIMANT1_MODULE`), yard (boy, cat, dog, bird, spider, antlion), and **also drawing routines**: `_DrawCurBalloons`, `_DrawSpider`, `_DrawSwarm`, `_Draw_SimYard`, `_DrawRain` call `_SRand*`.

Important consequence for the port: since drawing consumes S, the simulation sequence in the original depends on what was drawn (open windows, balloons). A bit-exact reproduction of the simulation requires either reproducing these drawing calls or accepting divergence after world generation [hypothesis: not measured how much this matters in practice]. World generation itself does not call drawing between `_SetSRandSeed` and the end of `_RandWorld` (fact seen in the code, and confirmed by the exact reproduction described below).

## Drawing random draws

Outside the tick, the original consumes S (never R) when **redrawing**. Sources: pseudocode (analysis from 2026-10-17, addresses below) and QEMU bench (recordings in `re_trabalho/cen/desenho_*`, `tools/cenarios_tick.py teclas` + `entre`, which counts the S and R steps between the end of one tick and the start of the next). Implementation: `engine/src/rules/desenho.rs`; checking: vectors `tests/referencias/acoes/desenho_*` (the interval between two ticks: state after one, state before the next), `engine/tests/acoes_originais.rs`.

**When there is a redraw** (`MYTIMERFUNC` `1000:2440` → `_DoAntSim` → `_UpdateWindows` `1000:2224`) [code]: after **each tick**, if the main window is not minimized; with the application inactive, only 1 tick in 8 (`DGROUP:0x2f0`). Without a tick (game paused) there is no `_UpdateWindows`. Speed: `_GameSpeed` 2 only draws with `_UDcntr` (`9:9b00`, +1 per call, modulo 64) even; 3, only with `_UDcntr % 4 == 0`. Order: map (0x100, no random draw), **yard** (0x1900: in front, every call; behind, every other call), **edit window** (in front, every call; behind, every other call). The windows' WM_PAINT only copies the buffer and does not draw random numbers.

| Routine | Condition | Random draws per redraw | Written to the save |
|---|---|---|---|
| `_Draw_SimYard(_YardMode)` (`1010:b462`), via `_UpdateYard` | yard open, `_YardMode` 0 (Yard) or 1 (House) | rain (`_RainOn` ≠ 0): **14 × (`SRand1(400)`, `SRand1(150)`) = 28** | — |
| `_DrawSwarm` (`1010:bb0e`), at the end of `_Draw_SimYard` (modes 0 and 1) | `_MapMode` = 0 | black: `nB = min(_SwarmCntB, 16, _QueenStorageB)`; each one outside ±15 goes back to (`S16 − 10`, `S16 − 10`); all move `S1(3) − 1` in x and y → **2·nB + 2·(outside)**; red: outside ±15 go to (0, 0) without random draw, all move `S1(3) − 1` → **2·nR** | `_SwarmCentX = 28·px − 10·py + 178`, `_SwarmCentY = 10·py + 46` (current patch), `_SwarmXB/YB/XR/YR` |
| `_DrawPalps` (`1010:2594`) in `_DrawSpider`, via `_UpdateEdit` → `1010:16d4` | edit window open, spider visible (`_PreDrawSpider`: `_SpidOn`, plane 0 or 1, inside the view), `_SMode` ≠ 5 | **1 × `SRand1(4)`**, except with `_SMode` < 2 and `_MeSMode` < 6 | — |
| `_DrawCurBalloons` (`1010:4cac`), via `1010:16d4`, **before** the cells and the palps | "Silly", `_ELayerMode = −1`, game not paused, a balloon noted in the tick | per speech whose real-time deadline has expired (or new balloon: deadline 0): black and red fight `S32`+`S2` each, egg and queen `S32`+`S4`, rest `S64`+`S2` (see [../telas/baloes.md](../telas/baloes.md)) | — |
| spider balloon (end of `_DrawSpider`), **after** the palps | "Silly", spider visible | `S64` + `S2` when the deadline expires (not paused); then, if `_SMode` changed or > 4, `S32` (even when paused; on every redraw with `_SMode` = 5) | — |
| `MYTIMERFUNC` | window minimized, game neither paused nor over | `SRand1(7)` per timer call (icon "Ant0"–"Ant6") | — |
| `_EndGameDialog`, `_SpiderDialog` (inside the tick) | game over; death by the spider with "Events" | one `S2`; one `S1(2)` (`1:685a`, after the animation: picks text 0x273F or 0x2740) | — |
| `_DoEditUpdateDraw`/`_GotoMyAnt` inside the tick (`_DoAntMoveY`, `_YellowFight`, `_YellowDeath`...) | edit window **open** (in front or not) | the same as the edit window (palps, balloons), in the middle of the tick: see "Drawing inside the tick" below | — |

### Drawing inside the tick

Sources: pseudocode (`_UpdateEdit` `1010:11e0`, `_GotoMyAnt` `1000:a764`, `_SetMapPlaneLocation` `1000:a386`, `_CenterEdit` `1010:1620`, `_YellowDeath` `1000:7746`, `_YellowBirth` `1000:739e`, `_AnimYellowFight` `1028:7e56`, `_AnimYellowInsane` `1028:8078`, `_DoAntMoveY` `1028:75f4`, `_DoTroph` `1000:846e`, `_TryMyDropOrLift` `1020:98de`) and test bench (count below).

Some tick routines redraw the edit window on the spot, via `_UpdateEdit` → `FUN_1010_16d4` (the same redraw as the timer's: `_PreDrawSpider`, `_DrawCurBalloons`, cells, `_DrawSpider` with the palps and the spider balloon, `_DrawBalloons`). `_UpdateEdit` only requires `_win_IsWinOpen(0)`: the edit window **open**, in front or behind [code]. The random draws are those of the timer redraw, with the view at that instant (`_MapPlane`, `_MapPnt`, `_editWidth × _editHeight`). When it happens [code]:

| Point in the tick | Drawing |
|---|---|
| `_GotoMyAnt` (outside Experimental, `_GameType` ≠ 3) | one redraw **if `_CenterEdit` moved the view** (in the shown plane, or after `_SetMapPlaneLocation` switches the plane); without movement, none. Called on death and on rebirth, in the AutoTrack of `_DoAntMoveY`, in the lost fight (`_YellowFight`), when passing between nests, when digging a new hole (`_TryMyDropOrLift`) and by the player's spider with AutoTrack (`_MoveSpider`) |
| `_YellowDeath(c)` | `_GotoMyAnt` and right after a `_DoEditAndMapUpdateDraw`; on reincarnation into an adult, `_GotoMyAnt` + another redraw |
| `_AnimYellowFight` (fight, deaths 1, 2, shock) | one before the loop and one per frame (after the frame's `S8`); with music and death 1–9 the loop exits on the 1st frame **before** the drawing |
| `_AnimYellowInsane` (deaths 7–9) | one before the loop and one per frame |
| `_YellowBirth` (reborn from an egg; founding a colony) | egg: `_GotoMyAnt`, one redraw, one per stage 2–7 and one at the end (8); colony: one after becoming queen, one with the egg, one per stage and one at the end |
| `_DoAntMoveY` | one before entering a surface hole and before leaving the nest; one on arriving next to the target ant (orders 3 and 4); one when order 4 turns the ant towards the yellow ant |
| `_DoTroph`, `_TryMyDropOrLift` (pick up) | one, after turning the yellow ant |

In practice only (1) the **palps** (`S1(4)`) draw random numbers: spider in sight in the current view and `_SMode` ≥ 2 (or `_MeSMode` ≥ 6), `_SMode` ≠ 5; and (2) the "Silly" **balloons**. Other than that, drawing inside the tick does not change S.

**Measured** (2026-10-18, `cenarios_regras.py contar_aranha`: breakpoints on `_SRand1`, `_DrawPalps`, `_DoEditUpdateDraw`, `_GotoMyAnt`, `_SpiderDialog`; `re_trabalho/cen_ag88/contar_aranha/contagem.json`): on the yellow ant's death by the spider, in order, the `S1(4)` of `_DrawPalps` (called from `3:25bd`) in the `_GotoMyAnt` drawing, another in the `_DoEditAndMapUpdateDraw` of `_YellowDeath`, the `S1(2)` of `_SpiderDialog` (from `1:685a`), the music's `S1(5)` (`1:78ce`) and the third `S1(4)` in the drawing before the `_AnimYellowFight` loop; the 1st frame exits via `_mySongIsDone` before the drawing. The 8 `_DoEditUpdateDraw` of `_YellowBirth` after the 2nd `_GotoMyAnt` (the view already in the black nest) do not draw random numbers. Control: death by the antlion (`contar_leao`, spider out of view) has no `_DrawPalps`. `_LionDialog` does not draw random numbers.

**In the port** (2026-10-18): `Estado::edicao_no_tick` (`rules::desenho::EdicaoNoTick`, outside the save) says whether the edit window is open and the view size; whoever runs the tick fills it in (`core::partida::Partida::preparar_edicao_no_tick`, from the open window of `janelas_desenho`; the vector comparator, `vetor_tick::montar_entrada`, with the bench window, open unless `"EdicaoAberta": false` in the extras). At the start of the tick (`desenho::comecar_tick`) the tick view starts from `_MapPlane`/`_MapPnt`; each `_GotoMyAnt` of the rules (`amarela::centralizar`) moves that view with the same `_CenterEdit` (`desenho::centralizar_vista`) and redraws if it moved (`desenho::centralizar_no_tick`); the table's `_DoEditUpdateDraw` calls `desenho::desenhar_no_tick`. The real view is still moved after the tick by the `Evento::CentralizarNaAmarela` requests, with the same calculation, so the two coincide; since 2026-10-19 these requests are presented in order with the frames and animations, and each `desenhar_no_tick` also leaves an `Evento::Instantaneo` with the drawn window (formiga-amarela.md §9.4; it draws no random numbers). The `S1(2)` of `_SpiderDialog` is drawn in `amarela::matar_amarela` (only with Events), between the death redraw and the music, and becomes `Evento::DialogoAranha { texto }` (`amarela.json` → `mensagens.aranha_comeu`). Checking: vector `ticks/amarela_morte_aranha/1793` 307/307 with S, R and life (was pending, 294/307); the other 144 vectors, the 2 long sequences and the actions remain equal. [hypothesis: with the program minimized the port also draws, like `_UpdateEdit`.]

**Balloon clock inside the tick (verified on 2026-10-19).** Each redraw reads `_TickCount` on the spot (`_DrawSpider` and `_DrawCurBalloons` call `_TickCount` before comparing and writing each deadline). The test bench recorded the value of each reading with breakpoints right after the `CALL _TickCount` of segment 3 (`cenarios_regras.py`, `capturar_tick_relogio`): outside the animations, all readings in the tick give the same value (`desenho_aranha_movey`: 6159 in the three readings of the redraw before entering the hole; 6245 in the two of the `_GotoMyAnt` when leaving the nest); in the yellow ant's animations the clock moves: on death by the spider with "Silly", no music and no Events (`amarela_morte_aranha_silly/1793`), the `_GotoMyAnt` redraw read 6023, the `_YellowDeath` one 6028, the one before the `_AnimYellowFight` loop 6029 and the 32 frames 6030, 6033, …, 6123 — **+1 on the first frame and +3 on each of the following ones**, the wait of "start + 6" on the Mac clock (which moves 3 per step). The 5 steps between the 1st and the 2nd redraw come from the routine's own time on the TCG bench (`_DrawMap` and the window drawing), not from a wait in the code. With Events, the `_SpiderDialog` box stops the clock in the middle of the tick for as long as the player takes to close it (1st recording: 6030 before and 6039 after the box).

**In the port:** the tick starts from `_TickCount` = `ms_total / 55` of the game (`EdicaoNoTick::tick_count`); `_AnimYellowFight` and `_AnimYellowInsane` advance it before each frame (`desenho::avancar_relogio_no_tick`: 1 step on the first, `espera_quadro_ms / 55` = 3 on the following ones of the fight and `espera_loucura_ms / 55` = 2 in the insanity). The time the original spends drawing and the spider box are not included (the port shows the box after the tick): in a live game with "Silly", a deadline that expires in these intervals may draw in a different redraw than in the original [limitation]. Balloon visibility (`_BalloonIsVisible`) uses the current view: inside the tick, the one the `_GotoMyAnt` calls have already moved (`baloes::na_vista`). Vectors: balloons are not in the save, so the new vectors record the balloon variables at input and output (`Baloes` in `extras.json`) and the `_TickCount` readings (`TickCount_lidos`); the comparator puts the balloons into the input state, uses the first drawing reading as the tick clock and compares deadlines, speeches, indices and `_LastSMode` afterwards (`vetor_tick::aplicar_baloes`, `comparar_baloes`).

**Spider in view during `_DoAntMoveY` (verified on 2026-10-19):** `desenho_aranha_movey` (the spider eating, stopped 3 cells from a hole, "Silly" and `_LastSMode` ≠ 3): on the tick in which the yellow ant enters the hole, the redraw before the entry draws the spider balloon (`S32`, deadline with the clock of the moment) and the palps (`S1(4)`); when leaving the nest, the subsequent `_GotoMyAnt` (back at the surface) does the same. Both ticks match 307/307 with S, R, life and the 19 balloon values.

`_DrawRain` (`1010:ba9e`, the same rain block) and `_SubtractFood` have no caller in the pseudocode [open: pointer]. `_MysteryButton` and `_CheatKeys` draw R and S, but they are player input, not drawing.

**Verified on the test bench (2026-10-17, rich snapshot, raining):** Yard View (Ctrl+Y) and House View (Ctrl+5) in front: **28 S steps between ticks**, in all 7 intervals; with the rain over (tick 1749), 0. Yard View with `_SwarmCntB = 5`, `_QueenStorageB = 3`, `_SwarmCntR = 2`, `_QueenStorageR = 4`: **38** (28 + 2·3 + 2·2) and, when `_SwarmCntB` dropped to 2, **36**; `_SwarmXB/YB/XR/YR` change between ticks exactly as `desenho::desenhar_enxame` (vectors `desenho_chuva_enxame`, `_enxame2`, `desenho_chuva_casa`: 307/307 and equal S). Strategic View (Ctrl+6, mode 2): 0. Edit window on the surface, without "Silly" and without the spider in view, and in the nest: 0 in all vector recordings (hundreds of intervals). `_SpecialXfer` opens the yard window inside the tick: `_DrawSwarm` writes `_SwarmCentX/Y` with the old patch (11, 8) and the colony drawing zeroes `_ColonyUpdateFlag` (`acoes/special_xfer`, 307/307 with that).

**In the port:** `desenho::redesenho_do_timer(e, janelas, contadores)` performs, after each tick, the yard and palp random draws with the same rules for when to redraw (the interface reports the open/in-front windows, active/minimized application and whether the spider is visible, and keeps the counters that are not in the save); `desenhar_quintal`, `desenhar_enxame`, `desenhar_palpos`, `desenhar_colonias` separate. Wired up (2026-10-18): `core::partida::Partida::rodar_tick` calls `redesenho_do_timer` after each tick, with `Partida::janelas_desenho` (the interface says on each frame whether the yard and the edit window are open and in front and whether the window is minimized — `simant/src/sessao.rs::atualizar_janelas_desenho`; focus comes from `definir_foco`; the visible spider is calculated in the tick with the view, `desenho::aranha_visivel`, like `_PreDrawSpider`) and the counters in `Partida::contadores_redesenho` (outside the save); minimized and with no pause or game over, `mensagem_de_timer` draws the icon's `SRand1(7)`. Tests: `core/partida.rs` (rain with the yard in front = 28 random draws after the tick, inactive 1 in 8, minimized icon). "Silly" and spider balloons (2026-10-18): `desenho::desenhar_edicao` calls `rules::baloes` in the order of the edit window redraw (balloons, palps, spider balloon, choice), with `_TickCount` = `ms_total / 55` of the game and the state in `Estado::baloes` (outside the save; telas/baloes.md). The drawings inside the tick: see "Drawing inside the tick" (2026-10-18). **Not covered**: the scroll redraw and the redraws the interface does for player actions outside the tick (Home, body switch, click on the yellow ant itself) [gaps]; the order between the yard and the control windows is not modeled (the edit window counts as "in front" if no control window is above it and the yard is not in front) [hypothesis].

## Is the seed in the save?

- The state of S (`DGROUP:0xcbf2`) and of R (`0xae34`) are **not** in the `.ANT` block table (fact seen in the code: the table `1038:9570` read by `_SaveGame`/`_LoadGame`, listed in [../formatos/save-ant.md](../formatos/save-ant.md), has no block at those addresses).
- What is in the save is **`_YMapSeeds`** (`1038:0224`, 192 words, block 29, file offset 45328): the seed of each yard patch, index `y * 12 + x`. With it the current patch can be regenerated. Fact seen in the code and verified in `QUICK2.ANT`.
- On load, `_LoadGame` calls `_RandYard` before reading the blocks (fact seen in the code), which reseeds S with the seed of a new patch; the exact state of S at the moment of saving is lost. Therefore, after a load, the original's simulation does not follow the same random sequence it would have followed without saving [hypothesis, to be measured].

## Verification done

Fact verified (2026-10-16): with the current patch seed read from `QUICK2.ANT` (`_YMapSeeds[8 * 12 + 11] = 13106 = 0x3332`) and the step above, the 8,192 `_SRand16` surface random draws reproduce **all** 7,415 ground cells of the saved `_MapA`, and the rest of the generation (see [geracao-mundo.md](geracao-mundo.md)) reproduces `_MapB` and `_MapR` entirely and `_MapA` except for the creatures that moved. Script: `re_trabalho/verifica_geracao_quick2.py` (working material, outside git).

## How to test against the original

1. **Without emulator (already done):** run `python re_trabalho/verifica_geracao_quick2.py`; it reads `tests/referencias/saves/QUICK2.ANT`, regenerates the patch and prints the differences. In the port, the same case is the test `engine/tests/geracao_quick2.rs`, and the vectors on this page are in the unit tests of `engine/src/rules/rng.rs` (implementation: `GeradorS` and `GeradorR`).
2. **Forced seed on the QEMU bench** ([../../bancada-qemu.md](../../bancada-qemu.md)): the `memoria` command gives physical RAM dumps. To force S, one needs to find the physical address of DGROUP (search the dump for a known DGROUP pattern, such as the table `DGROUP:0x230c = 19 1a 1c 1f 1e 1d 1b 18`) and write to the word `0xcbf2` with the QEMU monitor (HMP command `pmemsave`/`xp` to read; writing requires `gdbstub` or a new command in the bench). **Done:** the bench has `escrever_variavel SementeS ...` (gdbstub) and the tick mode; with the seeds forced, two runs give the same S and R sequence tick by tick (verified, see [testes-tick.md](testes-tick.md)). Caution: `_SimBird` only draws `SRand2` when `_gameCycles` (counter of the `MYTIMERFUNC` loop, not of ticks) has passed `_BirdDelay`; at normal speed this holds on every tick.
3. **R:** to test `_RRand`, generate two Quick Games in a row in the same session and compare `_YMapSeeds` of the two saves: the second table should be the continuation of the `rand()` sequence of the first (192 calls later), as long as nothing else called `_RRand` between the two (rain, fights) [hypothesis to be confirmed].
