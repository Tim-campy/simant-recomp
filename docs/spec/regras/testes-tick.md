*Leia em português: [testes-tick.md](../../pt-BR/spec/regras/testes-tick.md)*

# Tick-by-tick tests against the original (determinism and vectors)

This document describes how to compare the port's simulation with the original SimAnt **one tick at a time**: the determinism experiments done on the QEMU bench, the protocol for capturing "state before → state after" vectors, and what a vector needs to contain beyond the save layout. Bench: [../../bancada-qemu.md](../../bancada-qemu.md). Tick structure: [simulacao-visao-geral.md](simulacao-visao-geral.md). Random generators: [rng.md](rng.md).

Evidence convention as in the rest of `docs/spec/`: **verified** = measured on the test bench (date and command given); **seen in the code** = Ghidra pseudocode in `re_trabalho/decomp/`; **[hypothesis]** = not measured.

## Summary

- **It is possible to stop exactly at the start and at the end of a tick and advance one tick at a time** (verified, 2026-10-16). QEMU gdbstub breakpoints at the entry of `_DoAntSim` (`1028:0000`) and at its return address inside `MYTIMERFUNC` (`1000:2688`).
- **The original is deterministic tick by tick on the test bench** (verified): starting from the `quick` snapshot, two runs reach `GameTime` 300 with the 307 save blocks identical, **except `_BirdDelay`**, with or without forced seeds, and even when the VM runs free for 20 s (without breakpoints) midway.
- `_BirdDelay` diverges because `_SimBird` computes it from `_gameCycles`, a counter of the `MYTIMERFUNC` busy loop that depends on the host clock. It is the only non-deterministic input found inside the tick.
- Between two ticks (drawing, messages, timer loop) **nothing in the save changes and the S and R generators do not advance** in the `quick` snapshot configuration (verified over 139 intervals). Only `_gameCycles`, redraw counters and the stack change.
- To reproduce a tick, the port needs, besides the 307 save blocks: the state of the S generator, the state of the R generator, `_gameCycles`, `_LifeA/B/R` (or rebuild them from the lists) and `_GlobalKey`. Full list and justification in the "Question 3" section.

## Tools

| Tool | What it does |
|---|---|
| `tools/qemu_bancada.py`, commands `tick_entrada`, `tick_saida`, `ate_gametime N`, `passo_tick DIR`, `extras ARQ`, `segmentos PREFIXO` | tick mode (see [bancada-qemu.md](../../bancada-qemu.md#modo-tick-breakpoints-do-gdbstub)) |
| `tools/gerar_vetores_tick.py` | records N consecutive ticks in `tick_NNNNNN/` (see "Protocol") and maps reads outside the save (`--mapear-leituras`) |
| `tools/comparar_ticks.py A B [--detalhe]` | compares two vector directories, tick by tick |
| `tools/comparar_ticks.py --continuidade DIR` | what changes between the return of tick N and the entry of N+1 |
| `tools/comparar_ticks.py --fora-do-save DIR...` | bytes of segments 8, 9 and 10 that the tick writes outside the save layout (requires `--segmentos`) |
| `tests/referencias/roteiros/qemu_tick_passos.txt` | 3 ticks with `passo_tick` |
| `tests/referencias/roteiros/qemu_det_gametime.txt`, `qemu_det_livre.txt` | question 1 experiment, by script |

All of them require `SIMANT_RAIZ` pointing to the root with `referencia/qemu` and `re_trabalho/qemu` (with the `quick` snapshot saved in the overlay).

## Question 2: stopping at the start and at the end of a tick

**Technique (verified).**

1. **Code segment selector.** Code segments do not appear in the save table, so the data-selector method does not work. `Memoria.seletor_codigo(n)` reads the whole LDT (base and limit from `info registers`), filters the **present code** descriptors with a limit compatible with the segment size in the NE header (`SIMANTW.EXE`, segment 6 = `1028`, 0x9AF4 bytes) and compares the first 64 bytes of memory with those of the file (relocations change a few bytes). Observed (2026-10-16, `-m 16`): segment 6 = selector `0x1547`, linear base `0x80837500`, 58/64 bytes equal. All segments: 1 = `0x156f`, 2 = `0x1567`, 3 = `0x155f`, 4 = `0x1557`, 5 = `0x154f`, 6 = `0x1547`, 7 = `0x153f`, 8 = `0x1537`, 9 = `0x152f`, 10 = `0x1527` (they decrease by 8 in NE order, but the script always searches again).
2. **Breakpoint at a linear address.** The QEMU gdbstub compares the breakpoint with the **linear** PC (CS base + EIP). Entry of `_DoAntSim` = base of selector `0x1547` + 0. Confirmed: at each stop the script checks in `info registers` that CS base + EIP is the requested address (otherwise, it aborts), and the reads attributed to CS `0x1547` fall at function offsets of `1028` (e.g. `1028:04fa`, inside `_DoAntSimA`).
3. **End of the tick.** On entry, the top of the stack (SS:SP) holds the far return (IP, CS) of `_DoAntSim`. Observed: `156f:2688` = `1000:2688`, inside `MYTIMERFUNC` (`1000:2440`), consistent with the code. A breakpoint there stops the CPU right after the tick, **before** `_myServiceSong` and `_UpdateWindows`.
4. **No stepping over a breakpoint.** Each breakpoint is enabled only during the run up to it and disabled when it stops. Since the two addresses alternate, execution never continues from an active breakpoint.
5. **Persistent gdb session.** In tick mode the connection to the gdbstub stays open. With the VM running (after `continuar`), the gdbstub only accepts the interrupt byte 0x03; the script sends it before any command. Asynchronous stops (`T02`, generated by a QMP `stop`) are discarded.

**Definitions used in the vectors:**

- *state before* tick N: CPU at the 1st instruction of `_DoAntSim`, with `_GameTime = N` (not yet incremented);
- *state after*: CPU at the return of `_DoAntSim`, with `_GameTime = N + 1`.

**Measured cost:** `passo_tick` (state + life maps + extras, before and after) takes ~0.45 s per tick; `ate_gametime` advances ~3.1 ticks/s (221 ticks in 70 s), versus ~6 ticks/s for the free-running VM.

## Question 1: determinism

### Experiments (verified on 2026-10-16)

All of them start from the `quick` snapshot (`loadvm` with the VM running) and stop at the **first** entry of `_DoAntSim`.

| Id | Procedure | Result |
|---|---|---|
| E0 | first entry in all runs of the day (10 independent QEMU runs) | always `GameTime = 79`, S = `0x6b1d`, R = `0x53c15255`, `_gameCycles` = 10379; between E1a and E1b, the state and the whole segments 8, 9, 10 are identical at this point. The `loadvm` is reproducible up to the first tick |
| E1 | 2 × 3 ticks with `passo_tick`, without forcing seeds (`gerar_vetores_tick.py --ticks 3 --segmentos`) | tick 79: everything identical. Ticks 80 and 81: the 307 blocks equal **except `_BirdDelay`**; outside the save, only `_gameCycles` and the stack differ |
| E2 | 2 × (forced seeds S = `0x1234`, R = 1 at the 1st entry; tick by tick with breakpoints up to `GameTime = 300`; 2 ticks recorded) | equal except `_BirdDelay` (61550 × 59642) and `_gameCycles`. S and R identical (`0x1879`, `0x99dad475` at the entry of tick 300) |
| E3 | like E2, but the VM runs **free for 20 s** (without breakpoints, ~120 ticks) before the `ate_gametime 300` | equal to each other except `_BirdDelay`/`_gameCycles`; and also equal to E2 (except `_BirdDelay`) |
| E4 | 140 consecutive ticks (79 → 219) with `--segmentos`, `comparar_ticks.py --continuidade` | in all 139 intervals between ticks: save equal, S and R equal. Outside the save only `_gameCycles`, `_UDcntr`, `_UDMapFlip`, map buffers (`_mapMapBuf`, `_mapOutBuf`), `MYTIMERFUNC` counters (DGROUP `0x02F0`) and the stack change |

Working directories (outside git): `re_trabalho/det/{r1,r2,e2a,e2b,e3a,e3b,longo}`.

### Why `_BirdDelay` diverges

Seen in the code (`_SimBird`, `1030:28aa`), with the bird absent (`_BirdOn = 0`):

```
if _gameCycles > _BirdDelay (32 bits):
    _BirdDelay = _gameCycles + 20
    if _CurYardPnt > 4: draws SRand2(); if 0 and there is a mating flight (_SwarmCntB > 0 or 9:9c26 > 0): the bird appears
```

and, when the bird leaves the screen, `_BirdDelay = _gameCycles + 30`. `_gameCycles` (`9:9b2c`) **is not a tick counter**: `MYTIMERFUNC` adds 1 on each turn of the busy loop, which waits for `WM_TIMER` to arrive (see [simulacao-visao-geral.md §2](simulacao-visao-geral.md#2-o-que-mytimerfunc-faz-a-cada-chamada)). Measured: between two ticks at normal speed, `_gameCycles` advances by ~450 to ~1100, depending on the host clock.

Consequences:

- At normal (and slow) speed, `_gameCycles` always exceeds `_BirdDelay + 20` between two ticks, so the test is true on **every** tick: the `SRand2` random draw (if `_CurYardPnt > 4`) always happens and the S sequence does not depend on the clock. Only the **value** of `_BirdDelay` changes.
- [hypothesis] In "fast"/"ultra" (`_SpeedDelayVals` 0 and −1) there is one tick per loop turn, `_gameCycles` advances 1 per tick and the bird is then tested only every ~21 ticks. Then the S sequence also depends on the number of turns without a tick, which depends on the clock. Not measured.
- For the port: `_gameCycles` is an **input** of the tick (it comes in `extras.json`). A port that uses "ticks since start" in its place reproduces the behavior at normal speed, but not the value of `_BirdDelay`. In the comparison, `_BirdDelay` only matches if the port receives the vector's `_gameCycles`.

### Drawing, timer and mouse

- **Drawing consuming S:** not observed in the `quick` snapshot configuration (windows opened by the Quick Game, without touching anything): S does not advance between ticks over 139 intervals. The routines that call `_SRand*` while drawing (`_DrawCurBalloons`, `_DrawSpider`, `_DrawSwarm`, `_Draw_SimYard`, `_DrawRain`, see [rng.md](rng.md)) did not consume S in this period [hypothesis: they depend on balloons turned on, spider/swarm/rain visible or other open windows; not tested].
- **Inside the tick** there are reads of interface state (next section): `_win_IsWinOpen` (`_win_hwnd`), `_mapMessageRemoveTime`, `_editMessage`, fonts and clipping DC. [hypothesis] The tick decides whether or not to draw according to the open windows; if any of these routines draws from S, the sequence will depend on the windows. That is why the vectors are valid for the **window configuration of the snapshot** they came from.
- **Clock:** no clock read inside the tick was found other than `_gameCycles`. `_TickCount` is only called outside the tick (seeds, temporary messages).
- **Mouse and keyboard:** the test bench does not send events in the experiments. `_GlobalKey` (last key; 0xA8 turns on the rain in `_DoSimYard`) is read on every tick and was 0.
- **Pause and focus:** the game cannot be minimized (`MYTIMERFUNC` when minimized calls `SRand1(7)` outside the tick) and cannot be paused (the breakpoint is never reached; the script gives up after 120 s).

Conclusion: the breakpoint path is **sufficient for determinism** on the test bench. QEMU's `-icount` mode (virtual time by instruction counting) was not needed and was not tested.

## Question 3: what to capture besides the save

### Method (verified)

1. **Writes:** with `--segmentos`, the generator records the whole data segments 8, 9 and 10 before and after each tick. `comparar_ticks.py --fora-do-save` lists the bytes changed by the tick outside the 307 layout blocks, with the nearest symbol from the `.SYM`. Run over 140 ticks (79 → 219, covers phases 0–3 of 128 ticks and the 16-, 32- and 64-tick tasks).
2. **Reads:** `gerar_vetores_tick.py --mapear-leituras` places gdbstub **read watchpoints** on everything outside the save in segments 8, 9 and 10 (141 ranges, 80,062 bytes; minus the stack above `_end` and the already known maps), cut at symbol boundaries and into 64-byte pieces (QEMU reports the start of the watchpoint hit, not the byte read). Each read records the piece, the value and the CS:IP of the reader, translated to `10x0:offset`. Run over the same 140 ticks (~3.7 s per tick).

### Tick writes outside the save (140 ticks)

| Variable | Address | Nature |
|---|---|---|
| S state | `10:cbf2` (2 bytes) | generator; changes on every tick |
| R state | `10:ae34` (4 bytes) | generator; changes on every tick (`RRand(32)` of the rain in `_DoSimYard`) |
| `_LifeA` / `_LifeB` / `_LifeR` | `10:68e8` (8 KB), `10:88e8`, `10:98e8` (4 KB each) | occupancy maps; change when ants move |
| `_TemBModePop`, `_TemRModePop` | `9:786a`, `9:7be4` | temporaries: zeroed and rebuilt on every tick |
| `_TemCensus` | `9:7c4e` | count temporary (`_FullCount`) |
| area above `_end` (`10:cf56`) | stack (SS = DGROUP) | stack garbage, not state |

### Tick reads outside the save (140 ticks)

| Group | Variables | Value = file? | For the port |
|---|---|---|---|
| **Real state** | `_gameCycles` (`9:9b2c`) | no | tick input (see `_BirdDelay`) |
| | `_GlobalKey` (`9:7866`) | yes (0) | tick input (key) |
| | `_Tindex` (`9:9b6a`) | yes (0) | global index of `_DoAntSimA/B/R`; read at `1028:04fa`, at the start of `_DoAntSimA` [hypothesis: written before being used; captured for safety] |
| | S and R state | no | inputs |
| | `_LifeA/B/R` | no | not monitored (already captured separately) |
| **Constant tables** | `_Dx8`, `_Dy8`, `_Dx9`, `_Dy9`, `_TurnTab`, `_SpX`, `_SpY`, `_CasteTabB`, `_ModeTabB`, `_BxTab`, `_ByTab`, `_SowTab`; stretches of `10:235a`/`10:259a` (after `_commandStr`) | yes | executable data; the port loads them from `SIMANTW.EXE` or from `data/` |
| **Temporary** | `_TemDensity` (`8:4ad2`, 2 KB, read in 2 of the 140 ticks by `1028:93e2`) | yes (zeros) | [hypothesis] work area zeroed before use; the tick left no write in it |
| **Interface (outside the rules)** | `_win_hwnd`, `_win_handles`, `_ribbonBarHeight` (seg. 7, `_win_IsWinOpen`); `_mapMessage`, `_mapMessageRemoveTime`, `_editMessage`, `_editWidth`, `_editHeight` (seg. 1, `_DrawRibbonMessage`/`_EditMessage`); `_bigFontH`, `_paletteH`, `_displayType`, `_clipWind`, `_clipDC` (seg. 2, `GR_MODULE`); `_pack_buf` | — | the tick calls message and drawing routines; it only matters if it changes the state (see "Drawing") |
| **Segment selectors** | ~18 words in `10:be6c..10:c6ac` with values `0x152f`/`0x1537`/`0x1527` | no | selector constants the code uses to switch segments (fixed up by the loader); not state |

### Adopted capture list

`extras.json` (before and after), defined in `EXTRAS` of `tools/qemu_bancada.py`:

| Field | Address | Bytes |
|---|---|---|
| `SementeS` | `10:cbf2` | 2 |
| `SementeR` | `10:ae34` | 4 |
| `gameCycles` | `9:9b2c` | 4 |
| `Tindex` | `9:9b6a` | 2 |
| `GlobalKey` | `9:7866` | 2 |
| `GameTime`, `Cycle` | (redundant with the save, for checking) | — |
| `posicao` | `entrada` or `saida` | — |

Plus `vida_antes.bin` / `vida_depois.bin`: `_LifeA` (8,192) + `_LifeB` (4,096) + `_LifeR` (4,096) = 16,384 bytes. Per the [save specification](../formatos/save-ant.md), `_DoLoadInitializations` rebuilds these maps from the lists on load; the port can rebuild them the same way and use the file only for checking (verified on the 5 versioned vectors: the rebuild gives the same map in the middle of the simulation, before and after the tick; see "Comparing the port").

## Test protocol

### Generating vectors (original)

```
set SIMANT_RAIZ=C:\...\Simant recomp
python tools/gerar_vetores_tick.py --ticks 5                       # tests/referencias/ticks/
python tools/gerar_vetores_tick.py --ticks 3 --alvo 300 --semente-s 0x1234 --semente-r 1 --saida re_trabalho/v300
python tools/gerar_vetores_tick.py --ticks 1 --alvos 127,159,191,223 --saida tests/referencias/ticks/fases          # sparse vectors
python tools/gerar_vetores_tick.py --ticks 5 --preparar tests/referencias/roteiros/tick_amarela_ordem.txt --saida ...  # script (click) beforehand
python tools/gerar_vetores_tick.py --ticks 10 --livre 300 --saida tests/referencias/ticks/colonia_5min               # 300 s free-running beforehand
```

`--alvos` records `--ticks` ticks starting from each target, in the same run. `--preparar` runs a test bench script (`clicar_em`, `teclas`, `esperar`...) with the VM running, right after the `loadvm`. `--descricao` goes into `indice.json`. To avoid competing for the overlay with another bench, use `--overlay` with a copy of `re_trabalho/qemu/c_overlay.qcow2` (the `quick` snapshot goes along with it).

**Watch out for the phase:** the vector `tick_NNNNNN` is named after the `GameTime` **before** the tick, and the tick adds 1 to `_Cycle` before testing the periodic conditions. To exercise `_Cycle & 0x1F == 0` with `_Cycle` = 128, the vector is `tick_000127`.

```
```

Output per tick, in `tick_NNNNNN/` (NNNNNN = `GameTime` before the tick):

| File | Contents |
|---|---|
| `estado_antes.bin` | 307 save blocks, `.ANT` format (48,386 bytes), at the entry of `_DoAntSim` |
| `estado_depois.bin` | same, at the return of `_DoAntSim` |
| `vida_antes.bin`, `vida_depois.bin` | `_LifeA` + `_LifeB` + `_LifeR` (16,384 bytes) |
| `extras.json` | `{"antes": {...}, "depois": {...}, "MacTickCount_lidos": [...]}` with the fields of the table above; `antes.MacTickCount` = first value returned by `_MacTickCount` inside the tick (`null` if there was no call) and `MacTickCount_lidos` = all of them, in order (since 2026-10-17) |

`indice.json` stores the origin: snapshot, forced seeds, target, the first entry, the selectors and the return address of `_DoAntSim`.

### Vectors in the repository

`tests/referencias/ticks/`: ticks 79 to 83 of the `quick` snapshot (Quick Game, normal speed, start-of-game windows), without forcing seeds. On tick 80 (`_Cycle` = 80, a multiple of 16) the black queen lays an egg (`TotalEggsLaidB` 3 → 4, list B grows). Repeatability checked: ticks 79–81 match those of run E1 (except `_BirdDelay` from tick 80 on, see above).

Scenarios in subfolders (generated on 2026-10-17, listed in `"cenarios"` of the root `indice.json`; each one with its own `indice.json`, which stores the command, the `descricao` and the test exceptions):

| Subfolder | Ticks | How it was generated | What it exercises |
|---|---|---|---|
| `fases/` | 127, 159, 191, 223 (1 each) | `--alvos 127,159,191,223` | `_Cycle` = 128, 160, 192, 224 after the increment: phases 0, 1, 2 and 3 of `_DoSmells` (A/B/R compaction, `_FullCount`, `_HistUpdate`, `_SmoothAlarm`, `_FillHoles*`, `_ColonySmell*`); 128 and 192 are also multiples of 64 (`_FeedAnts`: `HealthB/R` −1, `MeHealth` −1). On 127 the original changed 43 blocks, among them `HealthB/R`, `MeHealth`, `H_*`, `HistCnt`, `HistStart`, `CastePop*` |
| `amarela_andando/` | 85–89 | `--preparar tests/referencias/roteiros/tick_amarela_ordem.txt`: single click on cell (32, 1) of the black nest (screen (160, 182)) | the yellow ant walks from (35, 8) to (33, 3), one step per tick, with `_MeMoveMe` = 1 and `_MeCmd` = 0, passing through the black queen's cells |
| `colonia_5min/` | 1885–1894 | `--livre 285 --alvo 1885` (normal speed, no intervention) | 140 slots in list A, 59 in B, 75 in R (many dead, T = 0), ants in various modes, cat off-scene (Mac clock read on every tick), antlion |

Event scenarios (2026-10-17, `tools/cenarios_tick.py` from the **rico** snapshot — `tests/referencias/roteiros/qemu_snapshot_rico.txt`: `quick` running for 285 s, ~1710 ticks, with rain falling, the boy mowing the lawn, a spider on screen and dozens of ants). The script records long sequences in `re_trabalho/cen/` (outside git), lists the events (`eventos`) and copies chosen windows (`selecionar`); the `indice.json` of each scenario stores the `passos` (memory writes, clicks) and the `selecao`:

| Subfolder | Ticks | Preparation | What it exercises |
|---|---|---|---|
| `chuva/` | 1711–1712, 1747–1751 | `_GlobalKey = 0xA8` at the entry of 1711 | flooding of the nests (`_DoWater`, `WaterTable` 46→45, `MapB/MapR`); end of the rain (1749) and restart with `_InitWater` (1750) |
| `chuva_secando/` | 1769–1771 | none | water receding (`WaterTable` 46→47, 0x4E back to tunnel with `S(8)`) |
| `revoada/` | 1711–1718 | 18 surface ants in M = 15 and `_FlyAwayB/R = 3` | modes 15 → 16 (`S1(F)`, then F = 1) and leaving the map in mode 16 (`S32`) |
| `menino_cortador/` | 1711–1715 | boy mowing (`_BoyHere = 3`, `_BoyIsMowing = 1`) placed on the current patch (11, 8) | `_FootFall` and `_MowerFall` (38 of 56 ants dead on 1712), under rain |
| `predadores/` | 1795–1797, 1812–1813 | none | spider eats a black ant (`_SMode` 2→3, `_BAntsEaten` +1, 1796); antlion eats (1813) |
| `amarela_superficie/` | 1732–1734, 1790–1792, 1850–1852 | Ctrl+8 and calculated clicks (`_MapPnt` written to put the cell in the center) | leaves the nest (`_ExitNest`), picks up food (double click, `_TryMyDropOrLift`) right after the compaction of list A, enters the red nest (with a view change) |
| `amarela_luta/` | 1860–1864 | double click on a red ant (order 3), right after a compaction | chase, `_YellowFight` with defeat, `_AnimYellowFight` (the clock is read ~1000 times in the tick), rebirth as an egg (`_MeEgg` 1→7) |
| `velocidade_rapida/`, `velocidade_ultra/` | 1899–1901, 1977–1979 | Shift+3 / Shift+4 | the tick is the same; `_gameCycles` advances 1 per tick and the Mac clock stays still (17427) while the test bench advances tick by tick |

Volume: 9.5 MB in total in `tests/referencias/ticks/` (~135 KB per tick).

### Comparing the port

For each `tick_NNNNNN`:

1. load `estado_antes.bin` as a save, **without** the load initializations that touch the generators (the original's `_LoadGame` calls `_RandYard`, which resets S; see [rng.md](rng.md#a-semente-está-no-save));
2. set S, R, `_gameCycles`, `_GlobalKey` (and `_Tindex`) to the values from `extras.json["antes"]`; build the life maps (rebuild or read `vida_antes.bin`);
3. run **one** `_DoAntSim`;
4. write the state in `.ANT` format and compare with `estado_depois.bin` (`tools/estado_diff.py`); compare S and R with `extras.json["depois"]` and the life with `vida_depois.bin`.

In the port, steps 1–4 are in `engine/src/rules/vetor_tick.rs` (`VetorTick::ler`, `montar_entrada`, `comparar_tick`):

```
cargo run -p engine --example comparar_tick -- tests/referencias/ticks/tick_000079   # one tick, full report
cargo run -p engine --example comparar_tick -- tests/referencias/ticks               # all of them, plus the scoreboard
```

The report includes: equal/different blocks (with the differing bytes and who changed the block: the original, the port or both); the first differences of each block, by ant and field in the lists (`formiga 5 campo T`), by cell `(x, y)` in the maps (`MapA/B/R`, `ExitMap*`, `PherMap*`, cells of the map itself) and by element in the others; S, R, `_gameCycles`, `_Tindex` and `_GlobalKey` after the tick; the life after the tick; and two checks of the life rebuild (below). The test `engine/tests/ticks_originais.rs` runs all vectors in `indice.json`.

**Life rebuild (verified, 2026-10-17, on the 5 vectors):** the `_LifeA/B/R` maps rebuilt from the lists like `_DoLoadInitializations` (`core/vida.rs`, the same one `Estado::de_save` uses) are **identical** to `vida_antes.bin` and, rebuilt from `estado_depois`, to `vida_depois.bin`. The hypothesis of the "Adopted capture list" section is confirmed for these ticks (without a yellow queen; her second cell still has no rebuild). The comparator uses the recorded life anyway, and the test requires the rebuild to keep matching.

Consecutive ticks can be chained: verified that the `estado_depois` of tick N equals the `estado_antes` of N+1 (only `_gameCycles` changes in the interval). Thus, an N-tick test can start from the first `estado_antes` and check each `estado_depois`, as long as it uses the `_gameCycles` of each `extras.json["antes"]`.

## Port scoreboard

One port tick (`RegrasOriginais` without generation tables) over each vector of `tests/referencias/ticks/`, compared with the original's `estado_depois`. "Changed" = blocks that the original altered in the tick; "extras" = S, R, `_gameCycles`, `_Tindex`, `_GlobalKey`.

**2026-10-17, after integrating yard, enemies, surface, nest, yellow ant and advice:** the 5 vectors match **entirely**.

| Tick | Equal blocks | Changed by the original (all equal) | Equal extras (S, R, `_gameCycles`, `_Tindex`, `_GlobalKey`) | Different life cells |
|---|---|---|---|---|
| 79 | 307/307 | 28 | 5/5 | 0 |
| 80 | 307/307 | 22 | 5/5 | 0 |
| 81 | 307/307 | 19 | 5/5 | 0 |
| 82 | 307/307 | 21 | 5/5 | 0 |
| 83 | 307/307 | 19 | 5/5 | 0 |

The last adjustment was the yard clock: `_BirdDelay` and `_CatDelay` only match by using the vector's `_gameCycles` (`Estado::ciclos_jogo`) also as the Mac clock [hypothesis; see quintal.md, "Implementação no port"]. History: the skeleton alone gave 280–289/307; with enemies, yellow ant and surface, 284–293; with the nest and the yard, 287–305; with the clock, 307.

**The test requires total equality** (`engine/tests/ticks_originais.rs`): any different block, extra or life cell fails. It runs the root vectors and those of each scenario in `"cenarios"`. Exceptions, always with the reason, in the scenario's `indice.json`: `"pendentes"` (the port still diverges; it is only required that the vector runs and that the save round trip works; whoever fixes the rule removes the tick from the list) and `"vida_nao_reconstruivel"` (the original's life does not come out of the lists; the rebuild is not checked, but the life after the tick is still required).

### Scenarios (2026-10-17, after recording the Mac clock)

All 24 vectors match **entirely** (307/307 blocks, extras 5/5, life 0): root 79–83, `fases/127, 159, 191, 223`, `amarela_andando/85–89`, `colonia_5min/1885–1894`. No vector in `"pendentes"`.

| Scenario | Ticks | Changed by the original (all equal) | `MacTickCount` read in the tick |
|---|---|---|---|
| root | 79–83 | 19–28 | no read (cat walking) |
| `fases/` | 127, 159, 191, 223 | 43, 30, 32, 20 | 127: none; 159: 3333; 191: 3621; 223: 3909 |
| `amarela_andando/` | 85–89 | 23–31 | no read |
| `colonia_5min/` | 1885–1894 | 22–40 | 18906, 18915, …, 18987 (one read per tick, +9 per tick) |

**Fix:** `tools/qemu_bancada.py` (`Depurador.ir_saida(relogio=...)`) places alternating breakpoints at the entry and at the RETF (`1000:96a5`) of `_MacTickCount` during the tick and records DX:AX of each call; `extras.json` gained `antes.MacTickCount` (first value read, `null` if the tick did not read it) and `MacTickCount_lidos` (all of them). The comparator puts this value in `Estado::relogio_mac`, and `quintal::Relogios::do_port` uses it as the Mac clock when present. All vectors were re-recorded (same ticks). Those of `fases/191`, `fases/223` and `colonia_5min` came out **different** from the first recording: since the cat and bark random draws depend on real time, the original is not deterministic tick by tick from the snapshot when the cat is off-scene (S at the entry of tick 191: 0x6038 in one run, 0x3353 in the other). The vectors remain valid because each one carries the clock it read.

**The two loose ends (1891 and 1893 of the first recording):** they were also the clock. Checked by injecting into the old vectors (outside the repository) the estimated clock (18903 + 9 per tick: 18903 on 1887, 18939 on 1891, 18957 on 1893): all three then match 307/307, with extras 5/5. Mechanism: with the wrong clock, the port made the cat's extra random draw (`_CatDelay` 19503 expired). This shifted the following random draws (the dog on 1891, two surface ants on 1893), and the total number of random draws in the tick coincided by chance, so that S and R matched at the end. There is no rule error.

**History — divergence of the first recording (resolved): the cat's Mac clock (yard, `_SimCat`, [quintal.md](quintal.md) section 6).** The port used `Estado::ciclos_jogo` (the vector's `_gameCycles`) as the Mac clock; the original uses `GetTickCount() / 55 × 3`. Evidence:
- `colonia_5min/1887`: the cat finishes its escape (state 3) and writes `_CatDelay = clock + 600`. Original: 19503, so clock ≈ 18903 (≈ 315 s in 1/60 s); port: 270585 = `_gameCycles` (269985) + 600. It is the only different block in this tick.
- `fases/159`, `191`, `223`: with `_CatOn = 0`, the original does not touch `_CatDelay` (3654, written on tick 127, and 4064), so its clock is still ≤ that value; the port compares with `_gameCycles` (29579, 36125, 42381), finds the delay expired, writes `_gameCycles + 200` and draws `S16`. The S generator diverges **before** `_DoAntSimA` (the yard is step 6 of the tick) and the rest follows in cascade: cat appearing only in the port (`CatOn`/`CatX`/`CatDir`/`CatCycle`, tick 159), dog (`DogDir`/`DogFrame`/`DogTurnCnt`/`DogX`), a surface ant (`AlistX/Y/T`) and, on 223, the black queen laying an egg only in the port (`Blist*[11]`, `ListIndexB`, `TotalEggsLaidB`, `HealthB`).
- `colonia_5min/1888–1894`: the same, starting from the original's `_CatDelay` = 19503: S diverges and from there surface and nest ants (`Alist*`, `Blist*`, `Rlist*`), pheromones (`PherMapBN/BT/RN/RT`), `MapA`, `SowDir`, `RedTask`, `LionListT` and the life. On 1889 the port also touches 64 bytes of `MapB` and of `MapR`, `WaterTable` and `ListIndexR` [hypothesis: also cascade]. On 1891 and 1893 S and R match at the end of the tick but the dog (1891) and two ants (1893) diverge: cause not isolated.
- On ticks 79–83 and 127 the clock is not read (cat walking or still on scene), which is why the hypothesis "Mac clock = `_gameCycles`" passed.

**Divergence 2 — life not rebuildable from the lists** (not a port error; the comparator uses the recorded life). Cases seen:
- `amarela_andando/86–88`: the yellow ant passed through the two cells of the black queen (Blist 0 and 1, T 96 and 104, at (34, 6) and (34, 7)); when leaving each one, the original zeroed the cell and the queen's mark disappeared from `_LifeB`.
- `fases/159`, `191`: several eggs and larvae at (34, 8); the egg laid in the tick (highest index, T 1) stays in `_LifeB` because it was written last, and the `_DoLoadInitializations` rebuild leaves the one with the lowest index (T 2).
- `colonia_5min` (all): dozens of dead slots (T = 0) parked on the hole cells (e.g. list A at (2, 32), list R at (32, 1)) erase, in the rebuild, the mark of the higher-index live ant that is on the same cell.
Consequence for the port: a save loaded mid-game may have a life different from the one the original had in memory; the original, when loading the same save, does the same rebuild, so this only matters for comparing with live memory.

### Event scenarios (2026-10-17, main at `317026e`)

The **48 new vectors match entirely** (307/307, extras 5/5, life 0); none stays in `"pendentes"`. Before the selection, the whole raw recordings were also compared: 552 ticks from `re_trabalho/cen/` (rain 80, free 250, yellow ant 122, fight 64, mating flight 12, boy 12, fast 6, ultra 6), all 307/307 after the two fixes below. All of them have `vida_nao_reconstruivel: {"*": ...}` (dead slots on the hole cells, as in `colonia_5min`).

Two divergences appeared and were fixed:

1. **Rule — `_DoAntMoveY` after entering the nest** (`engine/src/rules/amarela.rs`, `passo_da_ordem`, final block). With destination = the hole cell itself and the new plane ≠ displayed plane, the original ends the order (r = −1 → `_ResetYellowVars` at the new position: `MeGoal*` = (3, 32, 1), `MeLast*`, `MeMoveMe = 0`, `MeSteps = 0`); the port only centered and left the order running. In the pseudocode, after `_GotoMyAnt` the flow falls into label `1028:7b6b` (`local_8 = −1`). Fix: `return (-1, alvo)` in this case (4 lines). Vector: `amarela_superficie/1852`. Spec: formiga-amarela.md §4.
2. **Comparator — the view moves inside the tick.** `_GotoMyAnt` (`1000:a764`) calls `_CenterEdit`/`_SetMapPlaneLocation` in the middle of the tick and `_MapPnt`, `_MapPlane`, `_MapMode` are in the save; in the port this belongs to the interface (`Evento::CentralizarNaAmarela`, handled by `core::partida`). `vetor_tick.rs` (`aplicar_vista_da_bancada`) now reproduces the effect on the save when the tick requests the centering: it switches `_MapPlane`/`_MapMode` if the yellow ant is on another plane and moves `_MapPnt` like `_CenterEdit` (Bresenham with truncation, telas/jogo.md "Centralizar"), with the test bench window of 25 × 19 tiles. Checked on `amarela_superficie/1852` ((97, 23) → (20, 0)) and `amarela_luta/1864` ((47, 9) → (25, 1): the minor axis moves one step less). [hypothesis] `_editWidth` = 25 on the test bench (both calculations only work out with half-width 12, not 13).

**Gaps:** insecticide, Full Game and Experimental not recorded (the snapshot is from the Quick Game; reaching another game type requires navigating File > New dialogs, not attempted); mating flight only forced (not the one armed by `_GstrR`), without `_QueenStorage`/`_SwarmCnt` (they only rise in the Full Game); the recorded fight is a defeat — a victory is missing; only one vector per `_DoSmells` phase; the port uses one clock value per tick, and in `amarela_luta/1864` the read list has ~1000 different values, which still matched because the animation only uses the clock as a wait (no random draw depending on it) [hypothesis to check in `_AnimYellowFight`].

### Third round (2026-10-17, main at `c182a5d`): game modes, insecticide, victory, armed mating flight and long sequence

| Subfolder | Ticks | Preparation | What it exercises | Result |
|---|---|---|---|---|
| `jogo_completo/` | 31–35 | `full` snapshot (`roteiros/qemu_snapshot_full.txt`: boot, "Select a Game" → Full Game at (265, 300), `_CurGameType = 2`) | start of the Full Game (winged yellow queen on the surface), phase 1 of `_DoSmells` | 5 match |
| `experimental/` | 31–34 | `exp` snapshot (`roteiros/qemu_snapshot_exp.txt`, like `qemu_experimental_e01.txt` from the auxiliary agent) | Experimental without tools; egg laid on 31 | 4 match (31–32 with life not rebuildable: egg on top of larva) |
| `inseticida/` | 1711–1714 | rico; boy inside the house standing at node 32 ((45, 166) → patch (0, 12)) and `_CurYardPnt = (0, 12)` | stomping and insecticide on every tick (`(_RpopT + _BpopT) × 8` cell random draws; one ant dead per tick) | 4 match |
| `inseticida_morte/` | 19–21 | `full`, the same preparation | the yellow ant (winged queen) dies from the insecticide (`_YellowDeath(9)`, 21) | 19–20 match; **21 pending** (below) |
| `amarela_vitoria/` | 1799–1801 | rico; yellow ant placed as a soldier (life 100) next to a stationary red worker (M = 17), double click | `_YellowFight` with victory (`RAntsKilled` 0→1, red corpse) | 3 match |
| `revoada_armada/` | 1726–1727, 1825–1826, 1924–1927 | rico; 21 red ants become males (T 0xA0, M = 14) | the game arms `_FlyAwayR = 200` (`_GstrR`, 1727), pre-flight 14 → 15 with F > 100, take-offs (16) and exits until F = 0 | 8 match |

Long sequence (`tests/referencias/ticks_longos/rico_200/`, 392 KB): 200 consecutive ticks of the rico snapshot without intervention (GameTime 1711–1910), recorded by `cenarios_tick.py gravar longo`: `inicio/` (state and life at the entry of the 1st tick), `extras.json` (the extras before and after each tick, with the clock) and `depois_0050/`, `_0100/`, `_0150/`, `_0200/` (state and life). `vetor_tick::rodar_sequencia` runs the 200 chained ticks in the port, setting before each one only what changes outside the tick (`_gameCycles`, Mac clock, `_GlobalKey`), and checks S and R after each tick and the whole save and the life at the 4 points; `engine/tests/ticks_longos.rs` requires everything to be equal. **Result: 200/200, S and R equal on all ticks and the 4 points 307/307 with equal life.**

**Round scoreboard:** 27 new vectors, 26 match entirely and are required; 1 pending; the long sequence matches. Before the selection the raw recordings were also compared (Full Game 40, Experimental 40, insecticide 13, victory 30, mating flight 31): only `inseticida_morte/21` differs. No rule fix was needed in this round.

**Resolved (2026-10-18): `inseticida_morte/tick_000021` through the interface.** The port's tick ends with `_IsGameOver = 1` (what the rule dictates); the interface closes boxes 10042 and 10056 and the GAME OVER and, in "Select a Game", the Full Game starts with `carregar::jogo_novo_com_geradores`: `_RandYard` with the R and S **of the open game** (`_NewGame` does not reseed) and, as in the tutorial's Load, whatever `_RandYard` does not write stays as it was (list in telas/arquivos.md), plus `_LessonTemp = 1` (`_GotoMyAnt`) and the lesson only restarting in the Tutorial. Result: **307/307 with S and R equal** to the vector's after state (`engine/tests/fim_de_jogo_interface.rs`, `tick_21_volta_com_o_jogo_completo_novo_do_original`). In the scenario's `indice.json` the tick left `"pendentes"` and went to the new key `"pela_interface"` (the tick test only checks that it runs). Previous text:

**Pending `inseticida_morte/tick_000021` — game over through the interface inside the tick** (re-recorded on 2026-10-17 with the return breakpoint armed before each click: inside the same `_DoAntSim` box 10042, box 10056, the GAME OVER and the "Select a Game" screen open — captures `dialogo_21_0..3.png` —, the test bench chose Full Game and the tick returns with a new Full Game with `GameTime` **0**; the 8 of the 1st recording was a script artifact). Text of the 1st recording: With no body (no black egg or adult) and with `_ColonyTotalBlack = 1`, `_YellowDeath` sets `_IsGameOver = 1` and step 22 of the tick (`_EndGameDialog`) opens the "GAME OVER" box (captures `re_trabalho/cen/dialogo_21_*.png`: before, "You are covered with insecticide!"). The test bench clicked to continue and the original started a new Full Game **inside the same tick** (new world, `_CurYardPnt` (0, 12) → (11, 8), `GameTime` 8, `_IsGameOver` back to 0). The port ends with `_IsGameOver = 1` and the world intact — what the rule dictates; the restart belongs to the interface (`simant/`, `core/`), not to the tick. It is not a rule error.

**How to turn it into an interface test** (description; not implemented): start from `inseticida_morte/tick_000021/estado_antes.bin` in a `core::partida::Partida` (with `relogio_mac`, `_gameCycles` and seeds from `extras.json`), run one tick and check that (1) the port's state is the original's up to step 22, with `_IsGameOver = 1` (the tick vector already checks this, except the new world); (2) the queue has `Evento::DialogoFimDeJogo` once (the "GAME OVER" with score 27572 and the rating "Aardvark Fodder" from capture `dialogo_21_1.png`); (3) "continue" leads to the "Select a Game" screen and, with Full Game, to `Mundo::novo_jogo(TipoJogo::Completo, ...)`. The original does (3) inside the tick: the vector's `estado_depois.bin` is the start of a new Full Game (patch (11, 8), `GameTime` 8 — [hypothesis] ticks of the new game run before the test bench reaches the return of the old `_DoAntSim`), generated with the R and S seeds of the moment of the click, which depend on the clock; that is why the interface test must check the shape of the new game (type, patch, winged yellow queen on the surface, red colony), not the bytes.

Test bench (new features of `tools/cenarios_tick.py` and `qemu_bancada.py`): a tick that opens a modal box does not return by itself — after 15 s without reaching the return of `_DoAntSim`, the script captures the screen, clicks at (320, 330) and presses Enter (up to 5 times); the clock reads per tick stop at 200 (the list ends with `null`), because the animations (`_AnimYellowFight`, death) read the clock thousands of times and each stop costs ~0.1 s; a click can be sent with the VM going from the entry of one tick to the next (`clicar_celula(..., pausado=True)`), to record from the tick in which Windows handles it.

**Gaps:** mating flight in the Full Game with `_QueenStorage`/`_SwarmCnt` (requires a red colony with breeders in the Full Game; not attempted); `_FlyAwayB` (the player is the one who arms it, life cycle); Experimental with tools (those of the auxiliary agent apply with the game paused, outside the tick); `_SpecialXfer` (death in the Full Game with more than one black colony, choice on the yard map).

### Fourth round (2026-10-17, main at `613a5b1`): long sequence with interface, Tutorial, slow

**Long sequence with interface** (`tests/referencias/ticks_longos/full_600/`, 652 KB; `cenarios_tick.py gravar longo_full`): Full Game from the `full` snapshot, with rain requested (`_GlobalKey = 0xA8`) and swarms written at the entry (`_SwarmCntB/R` = 5/2, `_QueenStorageB/R` = 3/4) and the **Yard View in front** (Ctrl+Y) for 600 ticks (GameTime 31–631). Recorded: the initial state, the extras of each tick (S, R, `_gameCycles`, Mac clock, `_GlobalKey`) and the state and the life every 100 ticks. The S steps of the drawing between ticks are the difference between S at the end of one tick and at the start of the next. `vetor_tick::rodar_sequencia` (with `indice.json["desenho_quintal"] = 0`) runs the 600 ticks in the port, does `desenho::desenhar_quintal` between them and checks S and R after each tick and S after each drawing. **Result: 600/600, 17,650 S steps of the drawing equal to the original's, the 6 points 307/307 with equal life.** Found along the way: the drawing of the boy (`_DrawSimKid`, `1010:af14`) erases the balloon (`_BoyMessOn = 0`) when the clock passes `_BoyMsgCnt` — without this the 600-tick point differed only in that block (`desenho::desenhar_menino`).

**Tutorial** (`ticks/tutorial/`, `tut` snapshot: boot and "Select a Game" → Tutorial Game at (180, 225)): lesson 1 completed and lesson 2 requested (15), lesson 3 waiting for the digging milestone (`_TimeTemp` = 15, 46–47): 307/307, with `_TimeTemp` (`9:9b10`, outside the save) now recorded in the extras (`TimeTemp`) and put in the port's tutorial memory (`vetor_tick::regras_do_vetor`); the test checks `_TimeTemp` after the tick. Before recording it, the port considered lesson 3 completed (`_TilesDugB > _TimeTemp` with `_TimeTemp` = 0).
- **`tutorial/23` (re-recorded):** `_Feedback` → `_GiveLesson(3)` opens the lesson box with `_PictStrnDialog` inside the tick (`dialogo_23_0.png`). The 1st recording gave `GameTime` 23 → 40 and seemed to have nested ticks; it was a **script artifact** (it closed the box with the VM running without the return breakpoint: the tick ended unseen and the following ones ran loose). `capturar_tick` now arms the return breakpoint before clicking; re-recorded, the tick returns with `GameTime` 24 and matches 307/307 (the disassembly also shows that `_PictStrnDialog` stops the timer). The same applied to the `GameTime` 8 of `inseticida_morte/21`.
- Also in this scenario: `_CatDelay` = 4712 came from the tutorial file (another session) with the clock at ~3300, and the port "normalized" it (delay greater than now + maximum = expired). With a real clock the original leaves it as is: `quintal::normalizar_atrasos` now only acts with the synthetic clock (without `relogio_mac` and without `_gameCycles`).

**Slow speed** (`ticks/velocidade_lenta/`, Shift+1, `_GameSpeed` 0, 1719–1721): 307/307.

**Round scoreboard:** 8 new vectors, 7 match and are required, 1 pending; the long sequence with drawing matches entirely. New actions in `acoes/`: `exp_ninho`, `exp_cheiro` (telas/experimental.md §9), 307/307. Game over through the interface: `engine/tests/fim_de_jogo_interface.rs` (described above, now implemented): the vector's tick in a `core::partida::Partida` with the original's clock gives `_IsGameOver`, a single `DialogoFimDeJogo`, the three boxes in the original's order (10042, 10056 from `_PictStrnDialog` in `_YellowDeath`, and the GAME OVER) and the **score 27572** shown by the original; the new Full Game has the shape of the one the original started; the return to "Select a Game" is in `simant/` (`#[ignore]` test with the reason).

### Fifth round (2026-10-18): unverified rules from the review

Items from the "Not verified" list of [../../estado-do-projeto.md](../../estado-do-projeto.md) §3, recorded by `tools/cenarios_regras.py` (new scenarios on the same `Sessao` as `cenarios_tick.py`; `gravar`, `eventos`, `selecionar`, plus `registrar` and `selecionar_lote`). The preparation writes the original's memory at the entry of a tick (spider, list ants with the mark on the life map, the yellow ant, map cells); when the rule goes through the interface, the preparation uses the original's interface: X key and click on the spider to become the spider (`_DoLifeExchange`), key 1 and click for target mode, double click to run, patch change through the Yard View to reach the house. `rico` snapshot (Quick Game with ~1710 ticks), except the electric shock (`full`).

| Subfolder | Ticks | What it exercises |
|---|---|---|
| `aranha_cerco` | 1793 | 9 black ants in the 3 × 3: state 5, `_DeathCnt` 500, message 10046, `_SpidRevenge` 2 → 3 and `_MeSMode` = 7 |
| `aranha_foge` | 1793 | 6 black ants in the 3 × 3: state 4 and running |
| `aranha_vinganca` | 1793 | siege with `_SpidRevenge` 4 → 5: `_MeSMode` = 8 (laser) |
| `aranha_faro` | 1793, 1805, 1816 | `_SFoundAnt` by scent, chase, eating with `_EatCnt` = 11, `_DeadAntHere` |
| `aranha_laser` | 1794, 1796 | `_SpiderScan`: one black ant in the fan killed on each tick |
| `leao_cerco` | 1793 | antlion eating with 7 occupied neighbors: 10050, center 0x3F, shifted list |
| `aranha_jogador` | 1801, 1836, 1855, 1856 | player's spider: walks, runs, chases in target mode (`_MeSMode` 6) and eats (`_EatCnt` 50) |
| `aranha_jogador_cercada` | 1800 | the player's spider chasing and surrounded: `_YellowDeath(3)` |
| `aranha_jogador_fora` | 1800 | the player's spider leaves the map running: `_YellowDeath(4)` |
| `amarela_morte_leao` | 1711 | `_YellowDeath(2)` with `_AnimYellowFight` and two centerings in the tick |
| `amarela_morte_pisada` | 1712 | `_YellowDeath(5)` (the foot position came from a rehearsal run) |
| `amarela_morte_cortador` | 1712 | `_YellowDeath(6)` |
| `amarela_morte_afogada` | 1711 | `_YellowDeath(7)` with `_AnimYellowInsane` |
| `amarela_morte_fome` | 1711 | `_YellowDeath(8)` with `_AnimYellowInsane` |
| `amarela_morte_choque` | 27 | `_YellowDeath(10)`: the order ends on top of the house's power outlet 0x76 (patch (1, 5) of the Full Game) |
| `amarela_morte_aranha` | 1793 | `_YellowDeath(1)`: with the drawing inside the tick (below, finding 2), 307/307 since 2026-10-18 |
| `ninho_n01`, `ninho_n01_saude31` | 1714, 1720; 1711 | N-01: natural death with `_HealthB` 0; none with 31 |
| `ninho_n07` | 1712, 1713 | N-07: bites on the stored food |
| `ninho_n10` | 1711, 1713 | N-10: the water rises one row |
| `ninho_n12` | 1711, 1716 | N-12: the fight does not remove the checker; the cloud ends |
| `ninho_n14` | 1713 | N-14: passage 0x14 from the red nest to the black one |
| `ninho_n15` | 1711 | N-15: trophallaxis by the red intruder |
| `ovos_c04_c05` | 1791 | C-04: death at hatching with `_ModeAuto` 0; C-05: red caste with `_HealthR` 20 |
| `ovos_c06`, `ovos_c07` | 1711; 1711 | C-06: laying with health 100; C-07: none with health 0 |

**Scoreboard: 38 new vectors in 25 scenarios, 37 match entirely (307/307, S, R and life) and are required; 1 pending with the cause isolated** (resolved later, finding 2: today all 38 are required). Before the selection, the whole raw recordings were compared (≈ 230 ticks, including 83 of the player's spider and 24 of the scent): only the ticks below differed, and all for a known cause. Volume: 4.7 MB.

Findings:

1. **Two centerings in the same tick (port fixed).** In `amarela_morte_leao/1711` the original left `_MapPnt` at (21, 0) and the comparator at (22, 0). On death the original calls `_GotoMyAnt` with the yellow ant still at the center of the antlion (surface) and, on rebirth, again with it in the egg (black nest); each `_CenterEdit` starts from where the previous one left the view: (24, 0) → (18, 16) → (21, 0), with the minor axis one step short. The port centered only once, at the final position. Fix: `Evento::CentralizarNaAmarela` now carries `{plano, x, y}` from the moment of the request (`Estado::pedir_centralizar_na_amarela`); the interface (`core::partida::ir_para_amarela`) and the comparator (`vetor_tick::aplicar_vista_da_bancada`) apply each request in order. In mod scripts, the `centralizar_na_amarela` event gained `plano`, `x`, `y` (api-mods.md).
2. **Pending `amarela_morte_aranha/1793` (294/307): drawing inside the tick.** Isolated with `cenarios_regras.py gravar contar_aranha` (`contar_no_tick`: breakpoints at the entry of `_SRand1`, `_DrawPalps`, `_DrawSpider`, `_DoEditUpdateDraw`, `_GotoMyAnt`, `_SpiderDialog`, `_LionDialog`, `_AnimYellowFight`, `_mySongIsDone`, with S and, for `_SRand1`, the caller; writes `contagem.json`): 4 extra S steps in the original — `S1(4)` from `_DrawPalps` in the edit window drawings of `_GotoMyAnt`, from before the `_AnimYellowFight` loop and from the 1st frame (the spider in view, `_SMode` 3), and `S1(2)` from `_SpiderDialog` ([rng.md](rng.md)). The port did not do the drawings from inside the tick. **Resolved (2026-10-18):** the port now does the random draws of the redraws from inside the tick (`rules::desenho::EdicaoNoTick`, [rng.md](rng.md) "Desenho dentro do tick") and the `S1(2)` of `_SpiderDialog`; the vector matches 307/307 with S, R and life and left `"pendentes"`. The comparator assumes the edit window is open (as in all test bench recordings); a vector recorded with it closed carries `"EdicaoAberta": false` in `antes` of `extras.json`. The control count on death by the antlion (`contar_leao`) has no `_DrawPalps` (spider out of view) and matches. With the music on and no sound card on the test bench, `_mySongIsDone` ends the animation on the 1st frame (both counts).
3. **`_gameCycles` with a box that does not stop the timer.** In `ninho_n15/1711` `_DoTroph` opens "You feel much better now!" (`_YellowDialog`), and `_gameCycles` advances 647 inside the tick while the test bench closes the box (the `_PictStrnDialog` boxes stop the timer and do not change `_gameCycles`). New key in the scenario's `indice.json`, `"gamecycles_fora_do_tick"`: the test stops requiring only that extra.
4. **Test bench artifacts avoided:** (a) the click at (320, 310) that closes a box can reach the edit window after the box disappears and become an order for the yellow ant (seen in the 1st recording of `fome`: destination (40, 9) and one step); `cenarios_regras.py` closes the boxes with Enter; (b) the Mac clock read after a modal box is not recorded (the list ends at the box), and the `_CatDelay` written with it is not reproducible (seen in `cortador` and `pisada`); in these scenarios the cat is kept out of play (huge `_CatDelay` in the preparation).
5. **The player's spider with `_SMode` 0 or 1 is not surrounded**: the player's control replaces the rest of `_MoveSpider` (inimigos.md 2.2); the 1st attempt at `aranha_jogador_cercada` (spider standing still) did not die in 4 ticks. Surrounded while chasing, it dies.

### Sixth round (2026-10-19): the missing drawing inside the tick

`tools/cenarios_regras.py` (scenarios `aranha_autotrack`, `aranha_movey`, `morte_aranha_silly`), `rico` snapshot, with `capturar_tick_relogio`: besides the normal vector, the balloon variables (`Baloes`, in `antes` and `depois`) and the value of each `_TickCount` read by the drawing (`TickCount_lidos`, breakpoints at the returns of the `CALL _TickCount` of segment 3; the vector's Mac clock is the first read of `_MacTickCount`). The 14 vectors match 307/307 with S, R, life and the balloon extras (`ticks_originais`):

| Subfolder | Ticks | What it exercises |
|---|---|---|
| `aranha_jogador_autotrack` | 1810, 1812, 1844–1852 | player's spider with AutoTrack: walks (the view follows, without random draw) and, in target mode, chases (palps `S1(4)` in the `_GotoMyAnt` of each step) and eats; inimigos.md 2.7 |
| `desenho_aranha_movey` | 1800, 1811 | spider in view in the redraw before entering the hole and in the `_GotoMyAnt` after leaving the nest, with "Silly" (spider balloon); rng.md |
| `amarela_morte_aranha_silly` | 1793 | death by the spider with "Silly", without music and without Events: the `_TickCount` of each frame of `_AnimYellowFight` (+3 per frame); rng.md |

Findings: (1) the port did not call `_GotoMyAnt` on the player's spider with AutoTrack (fixed); (2) the balloon clock inside the tick is the one at the moment of drawing: equal to the one at the start outside the animations, +1 and then +3 per frame in the fight (the port models the waits; the drawing time and that of the modal boxes, not); (3) `_AnimYellowInsane` waits 2 steps per frame (110 ms, not 55: `amarela.json`).

### Eighth round (2026-10-19): strategy, recruitment and what remained

`tools/cenarios_raros.py` (new scenarios `estrategia_malvada`, `estrategia_vermelha_p`, `estrategia_vermelha_s`, `estrategia_preta_0`, `recrutar`, `revoada_limiar`, `queda_casa_celula`, `leao_pedrinha`, `volta_cycle`, `vista_xfer`), on **copies** of the overlays (`re_trabalho/ag8/`). Where the rule depends on a random draw, the entry S seed was chosen in the port with the new example `engine/examples/buscar_semente.rs` (`buscar_semente DIR_TICK SIMBOLO[ou NOME[i]] [VALOR|nz]`: runs the port's tick over the vector recorded without the seed, for each seed, until the block has the requested value) and written on the test bench before the before state. The 17 vectors and the 5 actions match 307/307 with S, R and life and are required by `ticks_originais.rs` and `acoes_originais.rs`. New action in the port: `{"comandos": [c...], "defesa_ultimo_tick": n}` (`vetor_tick::aplicar_acao`: `_YellowCommand(c)` with the game paused; `n` = `_TemBModePop[6]` read on the test bench, which `_UnRecruit` uses and is not in the save).

| Subfolder | Ticks | What it exercises |
|---|---|---|
| `estrategia_malvada` | 1711, 1712, 1767, 1768 | `_GstrR` by desperation: `_JustToBeMean` 0 → 56, mode 0, descending; 1 → 0 and the return to the normal strategy (mode 3) |
| `estrategia_vermelha_populosa` | 1711 | MALVADO "not normal", without random draw (`_RpopT` 150, `_BpopT` 20) |
| `estrategia_vermelha_sorteio` | 1711 | MALVADO by the random draws `S32()` = `S128()` = 0 (seed 0x009c) |
| `estrategia_preta_desespero` | 1711 | black ladder in mode 0 |
| `recrutadas` | 1726, 1727 | black ants recruited by keys 1 and 2 in Defense (`_DoRecruitAnt`) with the yellow ant on the surface |
| `revoada_limiar` | 1791, 1855 | count with 20 red breeders (does not arm) and with 21 (arms 200 on the count tick, 199 at the end) |
| `queda_casa_cozinha`, `queda_casa_ladrilhos`, `queda_casa_azulejo` | 34 | `_DROPdir` 2 and 0 (over a written 0x3F) and the kitchen tile 0x62, where nothing falls |
| `leao_pedrinha` | 1711 | the antlion spits the pebble 2 cells away (seed 0x02cd) |
| `volta_cycle` | 1711, 1712 | `_Cycle` 4095 → 0x1000 → 0: two phase 0s in a row |

Actions (`tests/referencias/acoes/`): `amarela_recrutar_5`, `amarela_recrutar_10`, `amarela_liberar_metade`, `amarela_recrutar_todas`, `amarela_liberar_todas` (keys 1, 2, 3, Ctrl+2, 4 with the game paused; superficie.md 9.4, T11).

Whole raw recordings compared before the selection (outside git): `estrategia_malvada` 7, the other strategies 2 each (and the random draw without seed, 2), recruitment 5 ticks + 5 actions (twice), threshold 5, fall 3 × 4, pebble 1 + 2, `_Cycle` wraparound 3: **all 307/307**.

Findings: no rule divergence. The `_GstrR` message ("The red ants are attacking!") goes to the ribbon, without a modal box (the tick did not stop). In the kitchen and on the tiles no tile of the patch makes the food fall (superficie.md 8.2). Outside the tick (test bench, no vector): Cancel of "Load Ant Hill" and of "Select a Game" at the opening (menus.md §7, abertura.md), Status in the Full Game and in the Experimental (menus.md §6) and the view after `_XferPatch` (divergencias.md; the port stopped centering on the yellow ant).

### Ninth round (2026-10-19): periodic feeding (closing round)

`tools/cenarios_fecha.py` (scenarios `alimentacao` and `alimentacao_casa`), on copies of the overlays `ov_r` (`rico`) and `ov_full` (`full`) in `re_trabalho/ag_fecha/`. At the entry of the tick: `_NextFeed` = 30000 (above any `_FoodA`) and `_Cycle` brought to the end of a 64 period; the tick calls `_AddFood(150, 1)` (superficie.md 8.3). The rule had never run in a vector (in all recorded ones `_FoodA` ≥ `_NextFeed` on ticks ≡ 0 mod 64, including in `volta_cycle`).

| Subfolder | Ticks | What it exercises |
|---|---|---|
| `alimentacao` | 1711 | `_AddFood(150, 1)` in the yard: center (S128, S64) = (85, 31), radius S8 + 5, 0x48 on floors < 0x18 and +1 on foods 0x48–0x4A, `_FoodA` 122 → 237, `_NextFeed` = S1(50) + 1 = 50 |
| `alimentacao_casa` | 31 | the same after `_XferPatch` to the left carpet (0, 4) of the Full Game: the food by the house floor rule (geracao-mundo.md 3.E) |

Finding: without the database tables (the tests run with `RegrasOriginais::default()`), the port's `adicionar_comida` **did nothing**: the tick gave 291/307 (without the food, with `_NextFeed` and S different). It now uses the computed sine, like object 9/1000 (like the laser), and both vectors give 307/307 with S, R and life (life rebuild waived for the reason of `volta_cycle`: dead slots parked in the holes). Scoreboard: 2 new vectors, 2 required.

### Seventh round (2026-10-19): rare branches

`tools/cenarios_raros.py` (same interface as `cenarios_regras.py`; snapshots `rico`, `full` and `exp`). The preparation writes the memory at the entry of a tick; where the rule is too rare to wait for, the entry S seed was **chosen in the port** (search over the vector of the same tick without the seed) and written on the test bench before the before state. The 31 new vectors (16 scenarios, 3.9 MB) match 307/307 with S, R and life and are required by `ticks_originais.rs`; the new action (`acoes/exp_leao_m1`) by `acoes_originais.rs`.

| Subfolder | Ticks | What it exercises |
|---|---|---|
| `morte_natural` | 1761, 1763, 1772, 1796, 1811, 1842 | `_HealthB` = `_HealthR` = 1 (`_AlwaysHealthy` = 1): natural death of a black soldier in the nest and on the surface, red worker on the surface (counts in `_BAntsExpired`, superficie.md 6.5), black male, new black queen in the nest and red male |
| `rainha_fome` | 1775, 1776 | `_HealthB` = 0: the head dies with `Sk(64)` = 0 (dialog 10015 inside the tick) and the tail disappears on the next tick (`_BlkQueens` = 0) |
| `rainha_vermelha_fome` | 1757, 1758 | the same for the red one (10016), with `_HealthR` = 0 reset at the entry of each tick (the red ants eat and the health rises) |
| `ninho_n02` | 1711 | N-02: `_HealthB` = 0 and S = 0x00A5 at the entry: the queen's head draws `Sk(256)` = 0 and does not die |
| `lutas_soldado`, `lutas_castas` | 1793 | boards of black and red ants in Bored: dozens of `_GetWinner` per tick |
| `ninho_n04`, `ninho_n05`, `ninho_n08` | 1719, 1721; 1711, 1714; 1713, 1718 | egg carried to the bottom and dropped with the same stage; red ants dropping 0x82; worker and soldier digging |
| `ninho_n11` | 1711, 1719, 1722, 1754 | rows 6..63 flooded without rain: drowned (red male, black worker), the water going down and the food of the dry row disappearing |
| `lista_a_cheia`, `lista_a_cheia_sem_vagas` | 1793 | `_ExitHole` with `_ListIndexA` = 1000 |
| `revoada_natural` | 3722, 3842, 3843 | males and new queens made by the red colony itself (rico in ultra until `_GstrR` arms `_FlyAwayR`): flight, take-off by `S1(F)` with F = 5 and leaving the map |
| `queda_casa_carpete_esq`, `queda_casa_carpete_dir` | 34 | spider in agony on a carpet strand in the house: the 3 foods fall to the right (`_DROPdir` = 1) or disappear (`_DROPdir` = 3) |
| `amarela_rainha` | 1733 | yellow queen walking in the nest: 0xFF and 0xFE rewritten at each step |

Whole raw recordings compared before the selection (outside git, `re_trabalho/raros/`): natural death 150 ticks, queens 66 + 48, N-02 3, fights 40 + 3 × 25, N-04/05/08 30 + 30 + 40, N-11 120, full list 2, natural mating flight 17 (plus 418 from an attempt in which `_FlyAwayR` did not arm), yellow queen 8, fall in the house 3 + 3 + 3 + 1, Load of a Full Game without red ants 40: **all 307/307**, except `ninho_n11/1797` (`_gameCycles`: the yellow ant's hunger box, `_YellowDialog`, does not stop the timer; like `ninho_n15`).

Findings:

1. **`_KillAntLion(-1)` writes 0x3F where the antlion that left was (port fixed).** The action `exp_leao_m1` (Experimental paused, two antlions in the list, insecticide spray on a 0x38 without an entry): the 1st spray writes 0x3F at (20, 0) (x = high byte of the `_EggStr` selector in this session, y = high byte of `_LastThemeSong`) and removes antlion 0; the 2nd writes 0x3F at its center (the X and Y of slot −1 are now its own) and removes the next one; the 3rd, at the center of this one; the rest repeat. The port did not do the `_SetMap`. Fix: `Estado::leao_x_vaga_menos_1` (outside the save; `None` while it is the selector) and `inimigos::matar_formiga_leao_sem_entrada` writes `(X[−1], high byte of _LastThemeSong)`; the test bench selector (20) goes in the action (`leao_x_vaga_menos_1`). Inimigos.md §1.4.
2. **`_DROPdir` in the extras.** Outside the save; the comparator used the port's 0. `qemu_bancada.EXTRAS` now records `DROPdir` and `vetor_tick` sets `Estado::passo_queda` with it. Read: 1 on patch (0, 4) (`_CarpetFloorL`) and 3 on (1, 5) (`_CarpetFloorR`), as in geracao-mundo.md §5.
3. **Victory ratio** (superficie.md 6.2): the 250 `_GetWinner` calls of the fight recordings (read in the port over the ticks that match 307/307, that is, the same as the original's): class 0 × 0, "a" won 45/106 (0.42; table 0.5); 0 × 1, 12/42 (0.29; 0.2); 1 × 0, 16/24 (0.67; 0.8); 0 × 2, 17/23 (0.74; 0.7); 2 × 0, 0/3; 1 × 2, 20/25 (0.80; 0.9); 2 × 1, 2/21 (0.10; 0.1); 1 × 1, 3/4; 2 × 2, 2/2. By the save counters (`_RedLost`/`_BlackLost`): workers × workers, the black ant won 40/70 (0.57); black soldiers × red workers, 43/66 (0.65); black males × red soldiers, 10/60 (0.17). All within what is expected for samples of this size with `RR(10)` < p (the soldier against the worker is ~2 deviations below 0.8); the rule itself is checked by the tick-by-tick equality (R and the counters match).
4. **Left click in the loop** (simulacao-visao-geral.md §2): `cenario_ritmo_clique`, in ultra, stops at the entry of `MYTIMERFUNC` (`1000:2440`) on each call (one instruction step before re-enabling the breakpoint). Without a click, 12–15 ticks per call (19 calls; one of 8 and one of 18). With a click on the title bar right after releasing the VM, the **next** call runs only 1–3 ticks (19 of 19), and the one with the click 9–22: the event reaches Windows at the end of the call in which it was sent, and the next one exits on the first turn in which it tests `GetAsyncKeyState`. Confirms the short-circuit in the code and what the port does (`botao_esquerdo_apertado`).
5. **`_LastColonyPopR` goes back to 1 on Load** (quintal.md, "Implementação"): `cenario_ultima_vermelha` (Full Game without red ants) showed 10052 and 10055 in the `_SimColonies` pass of tick 32; saved as `ULTVERM` and loaded via File → Load, the pass of tick 64 showed both again (captures `re_trabalho/cen/dialogo_32_*.png`, `dialogo_64_*.png`). The 40 ticks match 307/307 (the comparator does not check the boxes; the port has the test `ultima_populacao_vermelha_comeca_em_1`).

### Fifth round, tutorial (2026-10-18): full tutorial with the player's actions

`tests/referencias/ticks/tutorial_completo/` (16 ticks, 2.1 MB; `cenarios_tick.py gravar tutorial_completo DIR --snapshot tut`, details in [conselhos.md](conselhos.md) §3.1): the script does the action each lesson asks for (calculated clicks, keys, drag in Behavior, held click on the graph) and records in full only the ticks in which `_TutorCnt` reaches a multiple of 8; it saves the `tutcur` snapshot at each lesson to resume. Actions that depend on coordinates of other windows come from the environment (`TUT_EXTRA`, JSON per lesson). Two raw recordings (outside git): 124 ticks of lessons 1–26 (with the clock read after the box) and 185 of lessons 1–57.

**Result:** of the 309 ticks, all 307/307 with extras 5/5 and life 0, except three cases with the cause isolated:
- `tick_000575` of the 1st recording: `_CatDelay` different. On the `_FullCount` tick (`_Cycle` multiple of 64) the lesson box opens **before** `_DoSimYard`, and the cat read the clock after the box closed; the recording only had the reads from before the box (none), and the port used `_gameCycles`. Re-recorded with `MacTickCount_depois_da_caixa`: 307/307 (it is in the repository).
- `tick_001174`–`001229`: the test bench noted the map window (0x100) in front after the Patch button, but the original did not complete lesson 0x2A until Ctrl+M (the window was not in front). Annotation error, not a port error; outside the selection.
- `tick_001340`: the lesson 50 box with an image made Windows discard code segment 6 (`_DoAntSim`) and the tick did not return to the known return; the "after" was read a hundred ticks later. Invalid, outside the selection.

**Divergence found and fixed (rule, small):** `_TimeTemp` of lessons 7, 9 and 0x1A. `_GiveLesson` shows the box and only then reads `_MacTickCount` (tick 291: clock 10704 in the tick, 12213 after the box, `_TimeTemp` = 12813). The port used the tick's clock (`_TimeTemp` = 11304). Fix in `rules/conselhos.rs` (`MemoriaTutor::espera_depois_da_caixa` and `caixa_fechada`), called by `Partida::fechar_modal` and, in the vectors, by `vetor_tick::depois_do_tick` (the tick test checks `_TimeTemp`). Unit test `prazo_da_licao_7_conta_do_fechamento_da_caixa`.

**New extras** (only in `antes`, all optional): `JanelaNaFrente` (window the test bench put in front, in the `_win_IsWinInFront` format; goes to `RegrasOriginais::interface_tutor`), `VistaEdicao` (`[39, 21]`: the Tutorial's edit window is larger than the default 25 × 19; used by `aplicar_vista_da_bancada`) and, at the top level, `MacTickCount_depois_da_caixa`.

**Test bench (`cenarios_tick.py`):** `fechar_caixa_ate_o_retorno` also stops at the entry of the next tick (after state = that of the next entry) and does not require segment 6 to be present; it clicks lower in subsequent attempts (boxes with an image); with `relogio` it records the clock reads after the box closes (only on the `_FullCount` tick or when requested: `anotar_relogio_caixa`). Observed: the lesson box without an image does not call `_MacTickCount`.

## Limitations

- **Covered configurations.** The first vectors were only from the `quick` snapshot (Quick Game, normal speed, no balloons, rain or yellow ant moving). Today (2026-10-18) there are 107 vectors in 21 groups, from several snapshots (`quick`, `rico`, `full`, `tut`, Experimental), with rain, mating flight, predators, boy and lawnmower, insecticide, yellow ant walking and fighting, the four speeds and the tutorial (lessons 1–3 in `tutorial/` and the whole tutorial in `tutorial_completo/`), in addition to the long sequences `ticks_longos/rico_200` and `full_600` (the latter with the Yard View in front). Still without a vector: balloons turned on, the edit window behind during the yellow ant's animations, and the branches listed in [../../estado-do-projeto.md](../../estado-do-projeto.md) §3; in these cases the drawing may consume S or change the path of the tick.
- **"fast"/"ultra" speeds**: there are vectors (`velocidade_rapida`, `velocidade_ultra`; `_gameCycles` advances 1 per tick), but the pace in ticks per second depends on the CPU in the original and was only measured on the QEMU bench (see `_BirdDelay` and simulacao-visao-geral.md §3.1).
- **Reads:** the mapping covers data segments 8, 9 and 10 outside the save. It does not cover Windows memory (API called from inside the tick) nor the life maps (already captured). A variable read only in situations that did not happen in the 140 ticks does not appear.
- **`_Tindex` and `_TemDensity`**: classified by inference, not by test.
- **Determinism depends on the snapshot**: `loadvm` with the VM running always reached the same first tick in the runs made, but this is not guaranteed by QEMU without `icount` (the time until the first `WM_TIMER` comes from the host clock). If a run stops at a `GameTime` other than 79, compare by `GameTime`, not by order.
- **Selectors** are valid for `-m 16` and for the session saved in the snapshot; the script always searches again.
- Size: each tick takes ~130 KB (2 × 48,386 + 2 × 16,384 + JSON). Versioned (2026-10-18): 107 ticks in `ticks/` (15 MB) and the two sequences of `ticks_longos/` (1.1 MB, only the fields that change); other long sequences stay in `re_trabalho/`.
