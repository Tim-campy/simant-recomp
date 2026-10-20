*Leia em português: [simulacao-visao-geral.md](../../pt-BR/spec/regras/simulacao-visao-geral.md)*

# Simulation: overview (tick, order, frequencies, speed and pause)

This document describes the **structure** of the simulation loop of SimAnt for Windows: who calls the simulation, how many times per second, in what order the subroutines run inside a step ("tick") and how often each one acts. The rules of each subroutine live in their own documents: [formigas.md](formigas.md) (lists, castes, modes), [feromonios.md](feromonios.md) (scent maps). The random number generator, world generation and tile values are being written in parallel (`rng.md`, `geracao-mundo.md`, `tiles-valores.md`).

## Evidence convention

- **Fact seen in the code:** read in the Ghidra pseudocode (`re_trabalho/decomp/Code*.c`), with address `selector:offset` and name from `SIMANTW.SYM`.
- **Verified fact:** checked against a real file (save `tests/referencias/saves/QUICK2.ANT`, `SIMANTW.EXE`) with a reproducible script.
- **[hypothesis]** / **[open]:** as in the [README](../README.md).

Variable addresses follow the format of `symbols.tsv`: `logical segment:offset`. Data segments: 8 = `SIMANT_DATA_GROUP` (selector `1038`), 9 = `PACK` (`1040`), 10 = `DGROUP` (`1048`). Ghidra shows many accesses by offset only, without the segment. When the name does not come out of the `.SYM`, the segment was deduced from the executable's contents, and the text says so.

## 1. What triggers the simulation: the Windows timer

**Fact seen in the code.** There is no dedicated simulation loop. The simulation runs inside the timer *callback* `MYTIMERFUNC` (`1000:2440`, `SIMANT_MODULE`), registered with `SetTimer`:

| Situation | Call | Source |
|---|---|---|
| Game start, resume and end of dialogs | `SetTimer(_rootWnd, id 0, 17 ms, MYTIMERFUNC)` | `WINMAIN` `1000:3fda`; `_RestartSimulation` `1000:0034` (bytes `ff 36 78 cd 6a 00 6a 11 ff 36 42 00 ff 36 40 00 9a…`: pushes `_rootWnd`, 0, 0x11 and the pointer `_lpTimerFunc` `10:0040`) |
| Application loses focus (`WM_ACTIVATEAPP` with wParam = 0) | the same timer, recreated with **170 ms** (0xAA) in the code; **on the test bench the rate does not change** (61 ticks in 10 s active, inactive with the Task List, and minimized; 2026-10-19, script `qemu_ui_moldura_principal.txt`) | `MAINWNDPROC` `1000:2930` |
| Application regains focus | back to 17 ms | `MAINWNDPROC` `1000:2930` |
| Dialogs, menus, saving (`_SaveGame`) | `KillTimer` before and `SetTimer` (17 ms) after | `_DoUserButton` `1000:04c4`, `_DoMenuEntry` `1000:1876` |

`_StopSimulation` (`1000:0000`) and `_RestartSimulation` (`1000:0034`) use a nesting counter (DGROUP `0x0044`, unnamed in the `.SYM`). The first `Stop` kills the timer and discards pending `WM_TIMER` (0x113) messages. The timer is only recreated when the counter returns to zero.

[hypothesis] 17 ms imitates the 1/60 s *tick* of the Macintosh (the game is a port from the Mac). On Windows 3.1, however, `SetTimer` has the resolution of the system clock (18.2 Hz, about 55 ms): the requested interval is rounded up, and the 17 ms timer yields **one `WM_TIMER` every 55 ms**. `WM_TIMER` messages do not accumulate. **Verified on the test bench** (2026-10-18, section 3.1): the rates measured in slow and normal are exactly those the loop of section 2 gives with messages every 55 ms, and the Mac clock (`⌊GetTickCount / 55⌋ × 3`) advances 9 per tick in normal (3 periods of 55 ms). The 170 ms one without focus would become 220 ms (4 periods), but the test bench shows the same rate without focus (table above): the port uses 17 ms without focus too (`data/partida.json`, `intervalo_sem_foco_ms`) [the mechanism that cancels the 170 ms remains open].

## 2. What `MYTIMERFUNC` does on each call

**Fact seen in the code** (`1000:2440`). Own pseudocode:

```
when called:
  if there is no main window: exit
  if the window is minimized (icon):
      choose the icon according to pause / game over / victory;
      if not paused nor game over: draws SRand1(7) to animate the icon   ← consumes the RNG!
  delay += 1                                    (DGROUP 0x02EE)
  repeat:
      _gameCycles += 1                          (9:9b2c, 32 bits)
      if not a network client (_SimAntClientFlag 8:8a9a == 0):
          can_run = (not paused  OR  (_MeMoveMe ≠ 0 AND _CurGameTool < 10))
          limit = _SpeedDelayVals[_GameSpeed]
          if can_run AND (limit == -1 OR limit == 0 OR limit <= delay) AND not reentering:
              mark reentry; delay = 0
              _DoAntSim()                       ← ONE simulation tick
              _myServiceSong()
              unmark reentry
              if network server: _NetworkSend()
              if not minimized:
                  if the app is active: _UpdateWindows()
                  else: _UpdateWindows() only when (frames & 7) == 0
              frames += 1                       (DGROUP 0x02F0, 32 bits)
      else (network client): processes the received packet (_ProcessPost) and redraws
      if active and not minimized: temporary map message and _UpdateYardMessage()
      while there is a pending WM_TIMER: remove it; delay += 1; drained += 1
      iterations += 1; _myServiceSong()
      exit if: no main window
            OR (drained > 1 AND (iterations > 2 OR drained > 2))
            OR the left mouse button was pressed (GetAsyncKeyState(VK_LBUTTON) & 1)
```

Consequences:

1. **At most one tick per iteration.** Each `_DoAntSim` is one step; there is no fractional step.
2. **Busy loop.** At high speed, the callback runs several ticks in a row and only returns control to Windows after seeing at least 2 new `WM_TIMER` messages arrive (or 3), or a click. [hypothesis] This makes the tick rate in "fast"/"ultra" depend on the CPU.
3. **The delay counts timer messages**, not milliseconds: +1 on entering the callback and +1 per `WM_TIMER` drained inside the loop.
4. **Each call consumes 3 messages.** The loop only exits after draining 2 `WM_TIMER` messages (with more than 2 turns), so a call lasts 3 timer periods: the message that opened it and the 2 drained ones. The delay is tested at the start of each turn: after the +1 on entry and after the +1 of the first drained message. The +1 of the second drained message is only tested on the next call, which adds 1 more on entry. Calculations, with the timer at 55 ms [fact seen in the code; checked on the test bench, §3.1]:
   - normal (limit 2): in steady state, the entry takes the delay from 2 to 3 and runs the tick (delay = 0); the two drained messages take it to 2, with no new test. **1 tick per call = 1 every 3 messages = 165 ms (6.06 ticks/s).**
   - slow (limit 6): the delay rises by 3 per call; the tick happens every 2 calls. **1 tick every 6 messages = 330 ms (3.03 ticks/s).** The slow:normal ratio is 1:2, not 1:3.
   - fast and ultra (limit ≤ 0): every turn runs a tick while the loop waits for the 2 drained messages: about 2/3 of the time is busy loop, the rest is left to Windows (drawing, input). The rate depends on the CPU.

Counters used here (DGROUP, unnamed in the `.SYM`): `0x02EE` delay since the last tick; `0x02F0/0x02F2` frames (32 bits); `0x02F4` reentry lock.

**Left button** [seen in the code, `1000:2440`]: the exit test is a short-circuit `or`, so `GetAsyncKeyState(VK_LBUTTON)` is only called when the drained messages are not yet enough, and the tested bit 0 means "pressed since the last query" (the click may have been anywhere, before or during the loop). In practice, in fast and ultra a click ends the call at the end of the next turn: fewer ticks in that period, and the click is handled right away. **Port** (2026-10-19): `Partida::botao_esquerdo_apertado` (called on each left click in the window) sets the bit, and `mensagem_de_timer` consumes it at the same point as the test; test `clique_esquerdo_encerra_o_laco_de_mytimerfunc`. Not reproduced: other queries of `GetAsyncKeyState(VK_LBUTTON)` in the original (which would also clear the bit) [not searched for]. **Test bench (2026-10-19, `cenarios_raros.py ritmo_clique`, ultra, stopping at the entry of `MYTIMERFUNC` on each call):** without a click, 12–15 ticks per call (19 calls; one of 8 and one of 18); with a click on the title bar sent right after releasing the VM, the next call ran only 1–3 ticks in 19 out of 19 times (the click's own call, 9–22): the event reaches Windows at the end of the call in which it is sent and the next one exits on the first turn that tests the button. Confirms the code.

## 3. Speed (Speed menu)

**Fact seen in the code.** `_GameSpeed` (`8:8614`) indexes `_SpeedDelayVals` (`8:8616`, 4 signed words). Values read from `SIMANTW.EXE` (segment 8, offset 0x8616): `6, 2, 0, -1`. The menu action in `_DoUserButton` (`1000:04c4`, case 6) advances the speed in a cycle 0→1→2→3→0 and shows the corresponding message (DGROUP strings `0x014B`, `0x0160`, `0x0177`, `0x018C`).

| `_GameSpeed` | Message | `_SpeedDelayVals` | Tick rule | Map redraw in `_UpdateWindows` (`1000:2224`) |
|---|---|---|---|---|
| 0 | "Game speed now slow." | 6 | runs when delay ≥ 6 | every tick |
| 1 | "Game speed now normal." | 2 | runs when delay ≥ 2 | every tick |
| 2 | "Game speed now fast." | 0 | runs on every loop iteration | 1 in every 2 (counter DGROUP `0x9B00`, odd skips) |
| 3 | "Game speed now ultra." | -1 | runs on every loop iteration | 1 in every 4 (`0x9B00 & 3`) |

- Initial value: `_InitSimVars` (`1030:5a70`) sets `_GameSpeed = 1` (normal). **Verified:** `QUICK2.ANT` has `_GameSpeed = 1`.
- With the real 55 ms timer, slow gives 3.03 ticks/s and normal 6.06 ticks/s (§2, consequence 4; **verified**, §3.1). Fast and ultra run at CPU speed; ultra draws less and therefore runs more ticks per second.
- The tick logic does not depend on the speed. The port reproduces the loop (messages every 55 ms, 3 per call, delay test per turn) and fixes in data the number of turns per period (`data/partida.json`, `voltas_por_periodo`), which in the original depends on the CPU.

### 3.1 Measurement on the test bench (2026-10-18)

Scripts `tests/referencias/roteiros/qemu_ritmo.txt` and `qemu_ritmo_rapido.txt` (QEMU TCG, WfW 3.11, from the `quick` snapshot): `GameTime` and `_gameCycles` read with the VM running, at intervals of 20 or 30 s of host clock.

| Speed | `GameTime` | Ticks/s | `_gameCycles` per tick | Model (§2) |
|---|---|---|---|---|
| normal | 97 → 277 in 30 s; 3309 → 3426 in 20 s | 6.0; 5.85 | 208; 111 | 6.06 |
| slow | 280 → 370 in 30 s | 3.0 | 453 | 3.03 |
| fast | 370 → 2353 in 30 s; 2187 → 3287 in 20 s | 66 (start); 55 | 1.0 | depends on the CPU |
| ultra | 293 → 2109 in 20 s | 91 | 1.0 | depends on the CPU |

- In fast and ultra, `_gameCycles` advances exactly 1 per tick: every loop turn runs a tick (confirms the hypothesis in [testes-tick.md](testes-tick.md)). In slow/normal, the turns without a tick add up to 100 to 450 per tick, depending on the load of the emulated CPU.
- In a first ultra attempt (`qemu_ritmo.txt`), the black queen died and box 10010 stopped the simulation; the value was discarded.
- After each Shift+1..4, the ribbon does **not** show "Game speed now …" (captures `re_trabalho/ritmo/slow.png`, `fast.png`, `ultra.png`): see §3.2.

### 3.2 Accelerators and speed button

**Fact seen in the code** (`_ProcMenu`, `1000:4fc2`, cases 0x43–0x46): the Speed → Slow/Normal/Fast/Ultra menu (and the Shift+1..4 accelerators) only write `_GameSpeed` and update the menu check marks and the text of the Pause item; they do **not** call `_EditMessage`. Only the configurable speed button (`_DoUserButton` case 6) writes "Game speed now …" on the ribbon. **Verified on the test bench:** after Shift+1, Shift+3 and Shift+4 the ribbon was empty or kept the advice that was already there. The menu Pause (0x41), by contrast, writes "Game Paused" (or the instruction for tool 10/11) and clears it on unpausing.

### 3.3 Fast and ultra with a fixed-speed CPU (`-icount`, 2026-10-18)

To bound the plausible range on a real machine, the speed of the emulated CPU was fixed with `-icount shift=N,sleep=on` (1 instruction = 2^N ns of VM clock). `tools/ritmo_icount.py N`: full boot on a new overlay (a snapshot without `-icount` does not run with it), Quick Game, and `GameTime` measured in normal, fast and ultra against the **BIOS tick counter** (`0040:006C`, 18.2065 Hz in VM time), read with the VM paused at the start and end of each window. Ruler check: normal gave 6.07 ticks/s (shift 5, 6, 7) and 5.94 (shift 4, window of only 5 s of VM time), against 6.06 from the model (§2). In fast and ultra, `_gameCycles` advances 1.00 per tick in all of them.

| shift | VM instructions/s | Normal | Fast | Ultra | Window (VM time) |
|---|---|---|---|---|---|
| 7 | 7.8 million | 6.07 | 25.9 | 34.7 | 18–21 s |
| 6 | 15.6 million | 6.07 | 51.4 | 52.4 | 10–12 s |
| 5 | 31.3 million | 6.07 | 104.6 | 114.8 | 6–7 s |
| 4 | 62.5 million | 5.94 | 185.8 | 189.8 | 4–5 s |
| — (free TCG, §3.1 and 2026-10-18 with the host loaded) | not fixed | 5.86 | 34–66 | 32–91 | 20 s |

- **Fast grows almost in proportion to the CPU:** ≈ 3.0–3.3 ticks/s per million instructions per second. Ultra stays between 1.0 and 1.35 × fast (it draws once every 4 ticks instead of 2; the difference shrinks on fast CPUs). [hypothesis] the variation between runs (fast 25.9 with shift 7 against 51.4/2 = 25.7 predicted; ultra 34.7 against 26.2) comes from what is in the edit window in each measurement.
- **Plausible range on a real machine** [hypothesis: `-icount` charges 1 unit per instruction, while on a 386/486 instructions cost from 1 to dozens of cycles and VGA access is much slower than in emulation; the numbers are a ceiling]: a 386DX-33 (~8–10 million instructions/s) would give ~25–35 ticks/s in fast; a 486DX2-66 (~25–40 million), ~80–130; a Pentium, hundreds. The port fixes 5 and 7 turns per period (66.7 and 90.9 ticks/s, `data/partida.json`), which corresponds to a machine of ~20–30 million instructions/s on the scale above: within the range of a 486 of the era.
- TCG without `-icount` (normal test bench) runs at a speed that depends on the host load: 55–66/91 in one measurement, 33/32 in another (with another VM and an analysis running in parallel).

## 4. Pause

**Fact seen in the code.**

- `_GamePaused` (`9:9c28`) is toggled by the menu (`_DoUserButton`, case 5) and applied by `_SetPause`/`_PauseGame` (`1000:4e34`/`1000:4cc2`). They adjust the message ("Game Paused", strings 4/1010), the menu items, and end tool modes 10 (body swap, `_EndLifeTransferMode`) and 11 (target, `_EndTargetMode`) on unpausing.
- **Exception:** with the game paused, the tick **keeps running** if `_MeMoveMe` (`9:7d24`) ≠ 0 and `_CurGameTool` (`9:7c22`) < 10. The whole tick runs (full `_DoAntSim`), not just the yellow ant. [hypothesis] This is the "paused walk" of the yellow ant to the clicked destination; it needs a test on the original.
- Game over (`_IsGameOver`, `9:9dda`) does not stop the timer. `_DoAntSim` calls `_EndGameDialog(0)` at the end of the tick while the flag is set.

## 5. One tick: `_DoAntSim` (`1028:0000`)

**Fact seen in the code.** Exact order of the actions inside a tick. `_DoAntSim` has the bodies of `_DoSmells` (`1028:02fa`), `_FeedAnts` (`1028:0474`), `_ClrModePop` (`1028:034a`) and `_TallyModePop` (`1028:038e`) embedded (inline); they exist as separate functions but are not called anywhere else.

| # | Action | Condition / frequency |
|---|---|---|
| 1 | `_Cycle` (`9:75fc`) += 1; if it goes past 0x1000, it wraps to 0 | every tick (the cycle has 4097 values) |
| 2 | `_GameTime` (`9:76a8`, 32 bits) += 1 | every tick |
| 3 | clears the text balloon counters (fight/rest/queen/egg) and the last points (−1) | only if `_OptionStates[5]` (`8:85fc`, Options → Silly) == 1; the same option turns on `_FightBalloons`/`_RestBalloons`/`_QueenBalloons`/`_EggBalloons` (telas/baloes.md) |
| 4 | **feeding** (body of `_FeedAnts`): `_HealthB` −1 (if not `_AlwaysHealthy`), `_HealthR` −1, minimum 0; if the game is not Experimental (`_CurGameType` ≠ 3) and `_FoodA` < `_NextFeed`: `_AddFood(150, 1)` and `_NextFeed = SRand1(50) + 1` | `_Cycle & 0x3F == 0` (every 64 ticks) |
| 5 | **maintenance in 4 phases** (body of `_DoSmells`), phase = `(_Cycle & 0x60) >> 5`: | `_Cycle & 0x1F == 0` (every 32 ticks; each phase every 128) |
| 5a | phase 0: `_CompactListA`, then `_FullCount` (= `_CountAnts` + `_Feedback`), `_HistUpdate`, `_SmoothAlarm` | `_Cycle mod 128 == 0` |
| 5b | phase 1: `_CompactListB`, `_FillHolesBN`, `_ColonySmellBN`, `_ColonySmellBT` | `_Cycle mod 128 == 32` |
| 5c | phase 2: `_FullCount`, `_HistUpdate`, `_SmoothAlarm` | `_Cycle mod 128 == 64` |
| 5d | phase 3: `_CompactListR`, `_FillHolesRN`, `_ColonySmellRN`, `_ColonySmellRT` | `_Cycle mod 128 == 96` |
| 6 | `_DoSimYard` (`1030:14a8`): yard (boy, bird, cat, dog, rain) and, inside it, `_SimColonies` | every tick (details in section 6) |
| 7 | `_DoWater` (`1020:07c6`) | every tick; only acts with `_RainOn` and yard terrain (`_TERRAINset` = 0) |
| 8 | `_DoAntLions` (`1030:43fa`) | every tick (internal random draws) |
| 9 | `_MoveSpider` (`1020:4704`) | every tick (counter `_Scycle2`, `9:77ac`, modulo 1024) |
| 10 | `_DoPillar` (`1030:4cdc`): caterpillar; calls `_DoSow` at the start | only on ticks with odd `_Cycle`; does nothing in the house (`_TERRAINset` = 1) |
| 11 | `_GetStrategy` (`1030:0000`): black and red strategic mode, `_SetCasteProd`, `_SetModeProd` | every tick |
| 12 | clears `_TemBModePop` (`9:786a`) and `_TemRModePop` (`9:7be4`), 20 words each (body of `_ClrModePop`) | every tick |
| 13 | `_FlyAwayB` (`9:7c44`) and `_FlyAwayR` (`9:8078`) −1 if ≠ 0 (the code tests ≠ 0, not > 0: a negative value would keep falling) | every tick |
| 14 | `_DoAntSimA` (`1028:04d8`): surface ants | every tick |
| 15 | `_DoAntSimB` (`1028:2d4e`): black nest | every tick |
| 16 | `_DoAntSimR` (`1028:5344`): red nest | every tick |
| 17 | `_DoAntSimY` (`1028:7cf6`): state of the yellow ant (life, eggs if it is a queen) | every tick |
| 18 | mode summary (body of `_TallyModePop`): builds `_ModePopB`/`_ModePopR` from `_TemBModePop`/`_TemRModePop` (see [formigas.md](formigas.md#5-modes)) | every tick |
| 19 | if no red ant is in mode 19 ("Avenger"; `_TemRModePop[19]`, `9:7c0a`, < 1): `_MakeRedInitiator` | every tick |
| 20 | `_DoAntMoveY` (`1028:75f4`): movement of the yellow ant | every tick; only acts with `_MeMoveMe` ≠ 0 |
| 21 | `_Feedback` (`1030:78ca`): tutorial and advice messages | every tick |
| 22 | if `_IsGameOver`: `_EndGameDialog(0)` | every tick |
| 23 | `_gGameNeedsSaving` (`8:8370`) = 1 | every tick |

Notes:

- **List iteration order:** `_DoAntSimA/B/R` walk the lists **back to front** (from index `_ListIndexX − 1` down to 0), using a global index `_Tindex` (`9:9b6a`). Ants with type 0 (dead) keep occupying the slot until the next compaction. The order affects who moves first and the sequence of random numbers.
- `_CountUpdate` (`1020:07c0`) only calls `_Feedback` and does not appear in the tick. The full count is `_FullCount` (phases 0 and 2).
- Phase 0 compacts list A and **also** runs `_FullCount`/`_HistUpdate`/`_SmoothAlarm`; phase 2 runs only the last three. Count, history and alarm therefore run every 64 ticks.
- `_DoSpider` does not exist in the `.SYM`. The spider is `_MoveSpider`.
- `_GameTime` is only incremented here. **Verified:** in `QUICK2.ANT`, `_GameTime = 78` and `_Cycle = 78` (the game starts with both at 0; `_RandWorld` `1030:6086` clears `_Cycle`).

### 5.1 Count, history, mode summary and compaction

**Fact seen in the code** (Ghidra pseudocode and, where indicated, disassembly of `SIMANTW.EXE`).

**`_CountAnts` (`1020:04de`)**, called by `_FullCount` (`1020:07b4`, which then calls `_Feedback`):

1. Clears `_CastePopB[0]`, `_CastePopR[0]` and the 32 counters of `_TemCensus` (`9:7c4e`).
2. For each list (A, B, R, in that order), walks the slots from `_ListIndex − 1` down to 0 and, if T ≠ 0, adds 1 to `_TemCensus[T >> 3]` (black ants at indices 0–15, red at 16–31, by bit 7).
3. If `_MeMode` = 0, adds the yellow ant to `_TemCensus[(_MeType | color) >> 3]`, with color = 0x80 if `_MeColor` ≠ 0 (Ghidra shows the color as `_grndTile+2`; the address is `10:ce98`, `_MeColor`).
4. Black: `_CastePopB[0]` = k0; `[1]` = k1 + k2 + k3 + k5; `[2]` = k6 + k7 + k9; `[3]` = k4; `[4]` = k8 (kN = `_TemCensus[N]`).
5. **Death of the black queen:** if the previous `_CastePopB[5]` ≠ 0, k12 = 0 and `_JustXfered` = 0: music 0x2B0C and message 10010; if `_CurGameType` < 2, also message 10011, `_IsGameOver` = 1 and `_BlackWon` = 0. Then `_CastePopB[5]` = k12.
6. Red: the same with k16…k28 in `_CastePopR`; on the death of the red queen, music 0x2B0D, messages 10012 and (Tutorial/Quick) 10013, with `_BlackWon` = 1. **In addition**, if `_CurGameType` = 2, `_ColonyTotalRed` = 1 and `_CurYardPnt` = (11, 8): `k = SRand1(6)` and the byte `8:0184 + 16·k` receives 0x14. That address is `_YMapPopR` (`8:0164`) + 0x20 + 16·k, that is, 20 red ants in patch (k + 2, 0). Fact seen in the disassembly: the selector comes from `DGROUP:0xc2e4`, which the NE relocation binds to segment 8. [hypothesis] Reading: a new red colony appears in the house.
7. `_BpopT` = sum of `_CastePopB[1..5]`; `_RpopT` likewise; `_JustXfered` = 0.

The count only goes up to `_ListIndex − 1`; the reconstruction of life on load reads up to `_ListIndex`, inclusive (see save-ant.md).

**`_HistUpdate` (`1000:9182`)**: with `i = _HistStart`, writes `_H_BPop[i] = _BpopT`, `_H_RPop[i] = _RpopT`, `_H_BFood[i] = _FoodB`, `_H_RFood[i] = _FoodR`, `_H_BHeal[i] = _HealthB`, `_H_RHeal[i] = _HealthR`, `_H_FoodA[i] = _FoodA`, `_H_ADead[i] = _BAntsExpired`, `_H_PDead[i] = _BAntsEaten` and `_H_FDead[i] = _BAntsKilled` (from the 32-bit counters, only the low word). Then it does `_HistCnt += 1` if < 63 and `_HistStart = (_HistStart + 1) & 63`. The rest of the function redraws the graph if window 0x1500 is open (interface). `_ClearHistory` (`1000:8cfc`) clears the ten series and sets `_HistStart = 63` and `_HistCnt = 0`.

**`_TallyModePop`** (embedded in step 18): `_ModePopB` = (m2 + m3, m4 + m5, m1, m7, m12, m6), with mN = `_TemBModePop[N]`; `_ModePopR` the same, with `_TemRModePop`. Since `_ClrModePop` clears the counts in step 12 and only the ants processed in steps 14–17 count, a tick with no simulated ants leaves `_ModePop*` = 0.

**`_CompactListA/B/R`** (`1020:2a16`/`2a7a`/`2ade`): walks slots 0…`_ListIndex − 1`; each slot with T ≠ 0 is copied (X, Y, M, T, S) to the first free position ahead; `_ListIndex` decreases by the number of slots with T = 0. The slots at the end that were left behind **are not erased** and remain in the save as garbage.

### Summary of frequencies

| Periodicity | What |
|---|---|
| every tick | `_DoSimYard`, `_DoWater`, `_DoAntLions`, `_MoveSpider`, `_GetStrategy` (+ `_SetCasteProd`, `_SetModeProd`), `_DoAntSimA/B/R/Y`, `_DoAntMoveY`, `_Feedback`, decrement of `_FlyAway*` |
| every 2 ticks (odd `_Cycle`) | `_DoPillar` (+ `_DoSow`) |
| every 32 ticks (`_YardCycle & 0x1F == 0`) | `_SimColonies` |
| every 64 ticks | embedded `_FeedAnts` (health −1, new food), `_FullCount`, `_HistUpdate`, `_SmoothAlarm` |
| every 128 ticks | `_CompactListA`, `_CompactListB` + black scents, `_CompactListR` + red scents |
| every 64 ticks, yellow ant only | `_DoAntSimY`: life −1 (`_Cycle & 0x3F == 0`) |
| every 16 ticks | queen (black/red, and yellow if it is a queen) may lay an egg (`_Cycle & 0xF == 0`) |
| every 32 or 128 ticks | egg advances one stage (`_SimEggB`: 32 if `_BpopT` < 3, otherwise 128) |

## 6. Yard and colonies (`_DoSimYard`, `_SimColonies`)

**Fact seen in the code.**

- `_DoSimYard` (`1030:14a8`): `_YardCycle` (`9:7a5c`) += 1, wrapping to 0 after 0x3FF. `_InitSimYard` (`1030:1378`) starts it at −1, and therefore `_YardCycle = _GameTime − 1` at the start (**verified** in `QUICK2.ANT`: 77 and 78). Then it calls the boy (`_SimKidInside` if `_NodeNum` < 0x26, otherwise `_SimKidOutside`), `_SimBird`, `_SimCat` and `_SimDog` (the latter only if the house is not "for sale", `_ForSaleState` = 0). It starts raining if `_GlobalKey` (`9:7866`) = 0xA8 or if `RRand(32) == 0` and `RRand(128) == 0` (1/4096 per tick). Duration: `_RainCnt = SRand1(150) + 150` ticks. At the end it calls `_SimColonies`.
- `_SimColonies` (`1030:35f8`) only acts when `_YardCycle & 0x1F == 0`. It updates the 16×12 strategic colony map (`_YMapPopB` `8:00a4`, `_YMapPopR` `8:0164`), the expansion and extinction of neighboring colonies, the mating flight (`_SwarmCnt*`, `_QueenStorage*`) and the victory and defeat conditions of the Full Game. The rules are left for a separate document [open]; the game-over conditions are in section 8.

## 7. Strategy (`_GetStrategy`, every tick)

**Fact seen in the code** (`1030:0000`, `_GstrB` `1030:01cc`, `_GstrR` `1030:03c2`).

- Computes the "fuzzy" position of the yellow ant for those who follow it: `_FuzLocY = MeLocY + SRand1(5) − 2`, clamped to 0..63. The same calculation is done for X, but at the end of the function `_FuzLocX` is overwritten with `_MeLocX` (the X random draw is discarded; both `SRand1(5)` are consumed anyway). Sets `_ChaseSpid` if the spider is less than 100 away. This part only runs with the yellow ant on the surface (`_MePlane` = 1).
- `_StrategicModeB` (`9:9b8a`), with S = `_HealthB`, P = `_BpopT`, D = `_TilesDugB`:

  | Result | Condition (evaluated in this order) |
  |---|---|
  | 0 (attack) | S ≤ 9 and P/2 > `_RpopT` > 0 and `_RedQueens` > 0 (desperation), **or** none of the rows below holds |
  | 5 | S < 30 |
  | 4 | S < 50 |
  | 3 | D < P |
  | 2 | D < 2P |
  | 1 | P < 101, or `_RpopT` < 1, or `_RedQueens` < 1, or P/3 ≤ `_RpopT` |

  Reading [hypothesis]: 5/4 = hunger, 3/2 = need to dig, 1 = normal, 0 = attack with a margin.
- `_StrategicModeR` (`9:7690`) = `_GstrR()`: same ladder for the reds, with two differences. First, a red **mating flight** trigger: if `_FlyAwayR` = 0 and red males + new queens > 20, then `_FlyAwayR = 200`. Second, a "mean" mode (`_JustToBeMean`, `9:78dc`): when red health is very low and the reds greatly outnumber, or by a rare random draw (`SRand32` and `SRand128`), it forces mode 0 for `SRand1(100) + 30` calls, plays music and shows message 4/1020 no. 6 ("The red ants are attacking!").
- Next, `_SetCasteProd` and `_SetModeProd` choose the caste of the next egg and the mode of the next workers (see [formigas.md](formigas.md#4-castes-and-the-caste-triangle)).

## 8. Game over and scoring

### End conditions (fact seen in the code)

`_IsGameOver` (`9:9dda`) = 1 ends the game; `_BlackWon` (`9:9cea`) gives the result. Messages in SHARED type 4 (decimal ids):

| Where | Condition | Game types | Result | Messages |
|---|---|---|---|---|
| `_CountAnts` `1020:04de` (every 64 ticks) | the black queen existed in the previous count and disappeared (and there was no body swap, `_JustXfered` = 0) | Tutorial and Quick (`_CurGameType` < 2) | defeat | 10010 "The black queen in this patch has died.", 10011 "Sorry, you have been defeated by the reds." |
| same | the red queen disappeared | Tutorial and Quick | victory | 10012 "The red queen…", 10013 "Congratulations! You have defeated the red menace…" |
| `_YellowDeath` `1000:7746` | the yellow ant dies and there is no other black ant (egg, then adult in nest B, on the surface or in nest R) to reincarnate into | Tutorial and Quick | defeat | — |
| same | same, in the Full Game | Full (2) | if there is more than one black colony, "transfers" (`_SpecialXfer`, 10058); otherwise, defeat (10056) | 10058, 10056 |
| `_SimColonies` `1030:35f8` (every 32 ticks) | house "for sale" (`_ForSaleState` ≠ 0) **and** no red colony | Full | victory | 10057 "…you have won the Full Game." |
| same | ≤ 1 black colony, no queen, no eggs, no males and no new queens | Full | defeat | 10056 "The black ants have no queens, breeders or eggs remaining…" |

Messages without game over, all in `_SimColonies` in the Full Game:

- The reds vanish from the map (there was a red colony on the previous pass and now there is none): 10052 ("…red ants have been eliminated from the yard"). If the house is not yet for sale, 10055 also comes ("Now if you can take over the house…").
- More than 24 house cells occupied (`_HouseColonies` > 0x18): `_ForSaleState = 1` and 10054 ("…the humans have been driven out of the house"). If there are still reds, 10053 also comes ("Now you just have to eliminate the red ants…").

Apart from that, `_SimQueenB` shows 10015 ("The black queen in this patch starves…") when the black queen dies from `_HealthB` = 0.

`_CurGameType` (`9:80b4`): 0 = Tutorial, 1 = Quick, 2 = Full, 3 = Experimental (order of list 4/1001 and of the uses in the code; **verified:** `QUICK2.ANT` has 1).

### Scoring: `_CalcScore` (`1000:5530`)

**Fact seen in the code**, with caveats about the 32-bit arithmetic. There are 8 grades from 0 to 100:

| i | Grade | Calculation |
|---|---|---|
| 0 | average health | average of `_H_BHeal` over the last `_HistCnt` samples |
| 1 | relative food | 100 × Σ`_H_BFood` / Σ`_H_RFood` (0 if there is no red food) |
| 2 | eggs that hatched | 100 × (`_TotalEggsLaidB` − `_TotalEggsDiedB`) / `_TotalEggsLaidB` (100 if none was laid) |
| 3 | casualties | 100 × `_BlackLost` / (`_BlackLost` + `_RedLost`) (100 if there was no fighting). **Checked against the original (2026-10-19, seven GAME OVERs, mensagens.md §4.1):** this is the calculation; with no black ants lost the grade is 0, and losing more black ants raises it [hypothesis: a defect of the original; the port follows it] |
| 4 | colonies founded | Full and Experimental only: 100 × `_BColoniesStarted` / (B + R founded) |
| 5 | surviving colonies | Full and Experimental only: 100 × founded / (founded + `_BColoniesKilled`) |
| 6 | yard occupied | Full and Experimental only: 100 × (non-zero cells of `_YMapPopB` in the yard area) / 155. Yard area: columns 0–4, rows 3–11, and columns 5–15, rows 2–11; 5×9 + 11×10 = 155 |
| 7 | house occupied | Full and Experimental only: 100 × (non-zero cells in the house rows) / 37. House area: rows 0–2 in columns 0–4 and rows 0–1 in the others; 15 + 22 = 37 |

[open] The `_HouseColonies` count in `_SimColonies` uses a different area: rows 0–1, plus row **3** in columns 0–4. The scoring uses row 2. It may be a defect of the original.

Total = `_MeHealth` + Σ weight[i] × grade[i] × 51, with weights `13, 17, 19, 23, 29, 31, 37, 41` (8 bytes in DGROUP, offset 0x0948; the segment was deduced from the contents). Then:

- Full: if `_GameTime` < 8100, total = total × max(1, `_GameTime`/100) / 81; then × 5.
- Other types: total = total × 29/10; if `_GameTime` < 4100, × max(1, `_GameTime`/100) / 41.

**Checked against the original (2026-10-19):** the return value is indeed `_GameTime` + total, in 32 bits (seven game endings with scores from 331894 to 1946265, identical in the port; mensagens.md §4.1). The two limits (8100 and 4100) appear in the pseudocode as "string pointer + offset" (`s_NetBIOS__Delete_Name__1048_1f96 + 0xe` = 0x1FA4 and `s_DeltaTime___lu_1048_0ff5 + 0xf` = 0x1004); this is Ghidra interpreting constants as addresses. The limits were checked in the same cases (Quick with 2001 and 4101; Full with 3001, 8100 and 9001).

## 9. Points of attention for the port's determinism

1. **RNG outside the tick.** With the window minimized, `MYTIMERFUNC` calls `SRand1(7)` on every callback (animated icon). On the test bench, never minimize the game.
2. **Variable rate.** The number of ticks per second depends on the timer and the CPU. Always compare **per tick** (`_GameTime`), never by wall-clock time.
3. **Reverse iteration order** of the lists and use of the global `_Tindex`.
4. **Random draw per slot, not per live ant:** in `_DoAntSimA`, `SRand256()` is called for **every** slot of the list, including those of type 0, before testing whether the ant exists.
5. **Pause with movement:** see section 4.

## 10. How to test against the original

Test bench: [docs/bancada-qemu.md](../../bancada-qemu.md).

1. **Tick rate per speed.** Done on 2026-10-18 (§3.1): scripts `qemu_ritmo.txt` and `qemu_ritmo_rapido.txt`, which read `GameTime` and `_gameCycles` with the VM running after each Shift+N. Slow 3.0 and normal 6.0 ticks/s (ratio 1:2); fast and ultra vary with the QEMU CPU (TCG).
2. **Phase sequence.** In successive saves, check the periodic effects:
   - `_HistCnt` rises by 1 every 64 ticks; `_ClearHistory` (`1000:8cfc`) initializes `_HistStart = 63` and `_HistCnt = 0`. **Verified:** in `QUICK2.ANT`, with `_GameTime` = 78, `_HistStart = 0` and `_HistCnt = 1` (one call, at tick 64).
   - `_HealthB` drops by 1 every 64 ticks without food.
   - The `_PherMapBN` map receives 255 in the holes at tick ≡ 32 (mod 128) and right afterwards loses 1. **Verified:** in `QUICK2.ANT` (tick 78) there is exactly one cell with 254 in `_PherMapBN`.
3. **Tick-by-tick comparison with the port.** Protocol and ready-made vectors in [testes-tick.md](testes-tick.md) (breakpoints at the entry and return of `_DoAntSim`). Coarse alternative: use the `.ANT` as a "snapshot" of the state: load the same `.ANT` in the original and in the port, advance the same number of ticks and compare the 307 blocks. The yard and scent fields are already stable between ticks; lists and RNG require the same seed (see `rng.md`).
4. **Memory dumps.** `pmemsave` writes **physical** memory. In 386 enhanced mode, the 50 KB segments are not contiguous in physical RAM. **Observed** in `re_trabalho/qemu_ram_quick2.bin`: the signature `_Dx8/_Dy8` + `_TurnTab` appears at 0x22F700, but the lists (`8:23a4…`) are not at the expected offset from there. For useful dumps, resolve the linear base of the selector (Windows LDT) and use QEMU's `memsave` (virtual address), or compare only within a 4 KB page. Signatures for finding the segments: `_Dx8+_Dy8` (`00 01 01 01 00 ff ff ff ff ff 00 01 01 01 00 ff`, start of segment 8) and the fight table (`00 00 00 00 02 00 01 01 02 01 00 00 03 03 00 00 05 02 07 03…`, DGROUP 0x22C6).
5. **Pause with movement.** Pause, click a destination for the yellow ant and save after a few seconds: if `_GameTime` advanced, the exception of section 4 is confirmed.

## Implementation in the port

Where it is (crate `engine`, module `rules`):

- **`rules/estado.rs`: `Estado`.** Holds all 307 blocks of the `.ANT` in typed groups by subject, with the original symbol in the comment of each field:
  - `partida`: `_GameTime`, `_Cycle`, game type, pause, speed, game over, corpses...;
  - `ambiente`: terrain, `_Barrier`, rain, `_FoodA`, `_NextFeed`, `_YardCycle`, patch, seeds;
  - `superficie`, `ninho_b` and `ninho_r`: map, exit map, life and list. Each list holds **all** slots of the capacity, plus the `_ListIndex`;
  - `feromonios`: the five maps, plus `_PherMapF`;
  - `preta` and `vermelha` (`Colonia`): health, food, castes, modes, mating flight, digging, holes, death counters...;
  - `amarela` (`_Me*`), `lider_vermelha`, `formigas_leao`, `aranha`, `lagarta_e_bichos`, `menino`, `bichos`, `controles`, `tutorial`, `vistas` and `historico`.

  Runtime-only fields: the generators `s` and `r`, `censo` (`_TemCensus`), `modos_do_tick` (`_TemBModePop`/`_TemRModePop`), `tindex`, `baloes`, `tecla_global` (`_GlobalKey`), `precisa_salvar` and the `eventos` queue (messages, music and the game-over dialog). The rules do not call audio or windowing: they leave the request in the queue.
  - `Estado::de_save(&JogoSalvo)` reads the blocks by symbol name and rebuilds `_LifeA/B/R` like `_DoLoadInitializations` (`core/vida.rs`). The generators are not in the save. By convention, S starts at the seed of the current patch and R at the initial state of the C runtime [hypothesis: after a load, the original is not reproducible; see rng.md].
  - `Estado::para_save(&LayoutSave)` writes it back. The round trip is **byte for byte** (verified with `QUICK2.ANT` and `QUICK3.ANT`).
  - `Estado::de_mundo(&Mundo, s, r, &ConfigSimulacao)` builds the state of a new game from the generation. It applies the steps of `_NewGame`, `_RandYard`, `_RandWorld` (step 18), `_InitSimVars`, `_initControls`, `_ClearHistory` and `_CountAnts`.
  - Routines: `contar_formigas` (`_CountAnts`), `resumir_modos` (`_TallyModePop`), `atualizar_historico` (`_HistUpdate`), `ListaFormigas::compactar` (`_CompactList?`) and `adicionar_formiga` (`_AddAntTo?List`, with the life map).
- **`rules/tick.rs`: `tick(&mut Estado, &mut dyn Regras)`.** Follows the table of §5 in the exact order. The tick itself does steps 1–5, 12, 13, 18, 22 and 23, the test of step 19 and the periodic feeding (the `_AddFood` is delegated). The subroutines of each subject are methods of six traits, all with an empty default implementation:

  | Trait | Methods | Spec |
  |---|---|---|
  | `RegrasQuintal` | `simular_quintal` (`_DoSimYard` + `_SimColonies`), `simular_agua` (`_DoWater`) | §6, ninho.md §8 |
  | `RegrasInimigos` | `simular_formigas_leao`, `mover_aranha`, `simular_lagarta` (`_DoPillar` + `_DoSow`) | inimigos.md |
  | `RegrasSuperficie` | `simular_superficie` (`_DoAntSimA`), `adicionar_comida` (`_AddFood`), `escolher_estrategia` (`_GetStrategy`), `criar_lider_vermelha` (`_MakeRedInitiator`) | superficie.md |
  | `RegrasNinho` | `simular_ninho(cor)` (`_DoAntSimB`/`_DoAntSimR`) | ninho.md, ciclo-de-vida.md |
  | `RegrasAmarela` | `simular_amarela` (`_DoAntSimY`), `mover_amarela` (`_DoAntMoveY`) | formiga-amarela.md |
  | `RegrasConselhos` | `dar_conselhos` (`_Feedback`) | [open] |

  `Regras` is the sum of the six traits. `RegrasOriginais` is the game's implementation. At the end of `tick.rs` there is an empty `impl Regras<Parte> for RegrasOriginais {}` line for each part.

  **To plug in a part:** in `rules/<subject>.rs` (for example, `ninho.rs`), write `impl RegrasNinho for RegrasOriginais { fn simular_ninho(...) {...} }`. Then delete the corresponding empty line in `tick.rs` and add `pub mod <subject>;` to `rules/mod.rs`. Data tables the rules need go in as fields of `RegrasOriginais` (`geracao`, `quintal`, `inimigos`, `superficie`, `feromonios`, `ninho`, `amarela`, plus the memory and the tutorial interface), loaded by `RegrasOriginais::carregar(&Recursos)` with mods; `RegrasOriginais::default()` uses the embedded copies of `data/*.json`.
  - The tick's pheromone points (`_FillHoles*`, `_ColonySmell*`, `_SmoothAlarm`) call `RegrasSuperficie::fase_de_cheiro`, whose default runs the phase from `rules/feromonios.rs` with the embedded tables; `RegrasOriginais` uses the ones it loaded.
  - `simulacao_liberada` (pause, §4) and `Relogio` (delay counted in timer messages and `_SpeedDelayVals`, §2–3) decide **when** to run a tick. The tick logic does not depend on the speed. They are used by the game loop `core/partida.rs` (`Partida::avancar`): the 17 ms timer (170 ms without focus) rounded up to the 55 ms resolution of Windows 3.1, the `MYTIMERFUNC` loop with 3 messages per call (§2), `_gameCycles` (`Estado::ciclos_jogo`) + 1 per turn, turns per period per speed and the `_UpdateWindows` redraw, with the values from `data/partida.json`; description in [jogo.md](../telas/jogo.md), "Live simulation".
- **Data:** `data/simulacao.json` (`ConfigSimulacao`). Contains `atrasos_velocidade` (`_SpeedDelayVals` = 6, 2, 0, −1) and the initial new-game values that do not come out of the generation:
  - memorized triangles and sets, `_ModeAuto`/`_CasteAuto`, buttons and percentages from `_initControls` (`1000:cc84`);
  - `_GameSpeed` and the health warnings from `_InitSimVars`;
  - `_NextFeed` = 40, a static value of the executable;
  - `_OptionStates` and `_HistButtStates` [hypothesis: default observed in QUICK2/QUICK3];
  - `_Barrier` 0x50/0x90, written by `_OverlayTileSet` (`1010:00f4`).

  A mod can override the file.

Verified (`engine/tests/estado_simulacao.rs` and unit tests of `estado.rs` and `tick.rs`):

- The symbols covered by `Estado` are exactly the 307 of the layout. The round trip is byte for byte with `QUICK2.ANT` and `QUICK3.ANT`.
- `contar_formigas` on `QUICK2.ANT` and `QUICK3.ANT` reproduces `_CastePop*[1..5]`, `_BpopT` and `_RpopT` from the save. `_CastePop*[0]` comes out greater or equal, because of the eggs laid after the last count (tick 64).
- One tick with the empty rules on `QUICK2.ANT` changes only `_GameTime`, `_Cycle` and `_ModePop*` (the latter because no ant was counted in the tick).
- Periodicity and order of the calls, with test rules that log each call and the `_Cycle`: every tick, odd ticks (caterpillar), every 64 (feeding, count, history), every 128 per phase (A/B/R compaction), and the wraparound of `_Cycle` after 0x1000.
- `de_mundo` with the seed of `QUICK2.ANT` (needs `SIMANT_DIR`): controls, options, speed, `_Barrier`, `_NextFeed`, views, holes, queens and digging equal to the save.

Not verified: no part of the tick has been compared step by step with the original, because the subroutines do not exist yet. The Full Game path in `_CountAnts` (new red colony) has no test. Since 2026-10-19 `de_mundo` applies `_ResetYellowVars`, the rest of `_InitSpider`, `_ClearHistory(1)`, the `_CurEditMode` of `_SetMapPlane` and the static values of the executable, and `_InitSimYard` is handled by `quintal::iniciar_quintal` (`carregar::estado_do_mundo_novo`): a new Full Game, Experimental and Tutorial game matches 307/307 with the original before the 1st tick (geracao-mundo.md §8).

## Gaps

- Fast/ultra rate on a real machine: only bounded by emulation (§3.1 free TCG; §3.3 with `-icount`, ~3 ticks/s per million instructions/s); no real machine measured. Cadence without focus: measured equal to with focus (2026-10-19); the mechanism remains open.
- Details of `_SimColonies`, `_DoSimYard` (boy, dog, cat, bird, lawnmower), `_DoAntLions`, `_MoveSpider`, `_DoPillar`/`_DoSow`, `_DoWater`: only the cadence is described here.
- `_Feedback`: rules for the tutorial and the advice messages.
- ~~Exact return value of `_CalcScore` and the meaning of grade 3~~ (checked on 2026-10-19, mensagens.md §4.1).
- Network (`_NetworkSend`/`_ProcessPost`): outside the scope of the initial port.
