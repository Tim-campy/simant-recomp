*Leia em português: [feromonios.md](../../pt-BR/spec/regras/feromonios.md)*

# Pheromones (scent maps)

This document describes the five surface pheromone maps: format, how values are deposited, how they decay and how ants read them to choose a direction. The pace of each operation comes from the tick described in [simulacao-visao-geral.md](simulacao-visao-geral.md); the modes that use each map are in [formigas.md](formigas.md).

Evidence convention: the same as in [simulacao-visao-geral.md](simulacao-visao-geral.md#evidence-convention).

## 1. The maps

**Fact seen in the code** and save table ([save-ant.md](../formatos/save-ant.md)). Layer names taken from the decompressed SHARED 4/1000 text list: "Black Nest Marker Pheromone", "Black Trail Marker Pheromone", "Red Nest Marker Pheromone", "Red Trail Marker Pheromone", "Alarm Signal Pheromone".

| Map | Address | In-game name | Color | Use |
|---|---|---|---|---|
| `_PherMapA` | `8:52d2` | Alarm Signal | shared by both | alarm: attracts Alarmed and Defense |
| `_PherMapBN` | `8:62d2` | Black Nest Marker | black | gradient leading to the nest |
| `_PherMapBT` | `8:6ad2` | Black Trail Marker | black | trail to the food |
| `_PherMapRN` | `8:72d2` | Red Nest Marker | red | same, red |
| `_PherMapRT` | `8:7ad2` | Red Trail Marker | red | same, red |

- Each map has **2048 bytes = 64 × 32 cells**, one per 2 × 2 tile block of the surface (128 × 64). Values from 0 to 255, unsigned.
- Index: `(x >> 1) × 32 + (y >> 1)`. In the code it also appears as `(x & 0xFE) × 16 + (y >> 1)`, which gives the same value.
- They only exist on the **surface**. The nests have no scent map; there the exit map `_ExitMapB/R` (`8:03a4`/`8:13a4`, 64 × 64) is used, the subject of another document [open].
- Auxiliary map: `_TemDensity` (`8:4ad2`, 2048 bytes), a copy used in alarm smoothing. It does not go into the save.

## 2. Deposit

**Fact seen in the code.** Our own pseudocode; `c(x, y)` is the map cell containing tile (x, y).

| Function | Address | Rule | Who calls it |
|---|---|---|---|
| `_JamScentBN` / `_JamScentRN` | `1028:94b6` / `1028:94f6` | `c = max(c, v)` | ant of the given color, at every step, in modes Bored (0), Nesting/Dig In (1/4), Foraging (2) and Dig Out (5) (`_DoRandAntA`, `_DoToNestAnt`, `_DoForageAnt`, `_DoDigOutAntA`) |
| `_JamScentBT` / `_JamScentRT` | `1028:9536` / `1028:9576` | `c = max(c, v)` | `_DoReturnFoodAnt` (Got Food, 3); the yellow ant (`_DoAntMoveY`, `_PickupMyFood`) |
| `_AlarmHere` | `1028:943c` | `c = min(c + v, 200)` | yellow ant releasing alarm: v = 50 per step (`_DoAntMoveY`, with `_MeDropAlarm` on; the key is zero, according to tutorial 4/11052–11054) |
| `_AlarmHere2` | `1028:947e` | `c = max(c, v)` | every fight that starts: v = 40 at the cloud's cell (`_DoRandAntA`, `_DoRandAntAA`, `_DoToNestAnt`, `_DoToAlarm`, `_DoForageAnt`, `_DoRecruitAnt`, `_StartFightA`, `_DoAttackAnt`) |
| `_FillHolesBN` / `_FillHolesRN` | `1028:91de` / `1028:9244` | for each hole of the color (`_HoleMapB/R`), `c = 255`, or `c = 0` if the hole's tile is 0x51 `'Q'` [hypothesis: plugged/inactive hole] | tick ≡ 32 (black) and ≡ 96 (red), mod 128 |
| `_ExpIncSmell` | `1010:7b9e` | Experimental mode tool: `c = c + 70`, or `c − 40` with a key held down; clamped to 0..255. The layer comes from the menu selection (`8:8605`: 0 = BN, 1 = BT, 2 = RN, 3 = RT, 4 = A) | interface |

Edge details (**fact seen in the code**, `1028:91de`–`1028:95b6`):

- None of these functions checks bounds: the cell is `(x >> 1) × 32 + (y >> 1)` directly. Callers only pass surface tiles.
- `_JamScent*` write when `c < v` and `_AlarmHere2` when `c ≤ v`; the effect is `max(c, v)`. The value comes from the S field (byte, 0–255) or from a constant (40).
- `_AlarmHere` writes `min(c + v, 200)` without looking at `c`: a cell that was already above 200 (Experimental tool) **drops** to 200 when the yellow ant releases alarm there.
- `_FillHoles*`: `_HoleMapB/R` is indexed by the surface row `y` (which is the nest column, see [geracao-mundo.md](geracao-mundo.md)) and stores the hole's `x`; 0 = no hole. The scan goes from `y` = 0 to 63; the tested tile is `_MapA[x][y]`.
- `_DecTSmell(x, y, color)`: color = bit 7 of the ant's type (0 = black trail, otherwise red).

**Deposited value: the ant's S counter.** The move happens before the deposit: at each step, if S > 0, the ant does S −= 1 and then deposits S on the destination cell. Thus the trail forms a gradient that **decreases** from the origin:

- **Nest scent:** the ant leaves the hole with S = 120, if the hole is on its color's side and the mode is not 3, 6 or 7 (`_ExitHole` `1020:2db6`). It leaves 119, 118… as it moves away. The hole receives 255 periodically (`_FillHoles*`). Result: the maximum value is at the hole.
- **Trail:** on picking up food, S = 200 (`_DoRandAntA`, `_DoForageAnt`), and the ant returns to the nest leaving 199, 198… The highest value is next to the food. The yellow ant uses `_MeScent`, which is 200 on picking up food (`_PickupMyFood` `1020:8f4c`). It only deposits on the surface, while carrying food (`_MeType` 0x18 or 0x38), and deposits **before** decrementing. The counter drops 1 per step, but does not go below 10 (`_DoAntMoveY` `1028:75f4`). The trail color comes from the yellow ant's color field (`_grndTile`+2).

## 3. Decay

**Fact seen in the code.**

| Map | Function | When | Rule per cell |
|---|---|---|---|
| BN | `_ColonySmellBN` `1028:92aa` | tick ≡ 32 (mod 128), right after `_FillHolesBN` | if c > 0: c −= 1 |
| RN | `_ColonySmellRN` `1028:92d8` | tick ≡ 96 (mod 128) | same |
| BT | `_ColonySmellBT` `1028:9306` | tick ≡ 32 (mod 128) | if c < 8: c = 0; otherwise c = c − ⌊c/2⌋ (half, rounded up). `_ColonySmellRT` tests `c ≠ 0` first; the result is the same |
| RT | `_ColonySmellRT` `1028:9344` | tick ≡ 96 (mod 128) | same |
| BT/RT | `_DecTSmell` `1028:95b6` | at every step of an ant in Bored (0) or Foraging (2), on the destination cell | if c > 0: c −= 1 (the trail "wears out" with use) |
| A | `_SmoothAlarm` `1028:9380` | ticks ≡ 0 and ≡ 64 (mod 128) | smoothing (below) |

**Alarm smoothing** (all cells computed from the previous copy, `_TemDensity`):

```
for each cell (i, j) of the 64×32 map:
    neighbors = sum of the 4 orthogonal neighbors in the copy (outside the map counts 0)
    new = (copy[i][j] + neighbors/4) / 2          (integer divisions)
    if new < 9: new = 0
```

The alarm spreads and loses intensity. An isolated peak of 200 becomes 100 at the center and 25 at the neighbors after one pass.

**Fact seen in the code** (`1028:9380`): the whole copy is made first (first loop), and the second loop only reads the copy and only writes the map; the scan is column by column, but since it reads the copy, the order does not matter. The 4 neighbors are summed as unsigned integers; there is no wraparound at the edges (unlike direction reading, section 4).

**"Tick" means `_Cycle`, not `_GameTime`** (fact seen in the code, `_DoAntSim`): `_Cycle` goes from 0 to 0x1000 and then back to 0. The values 0x1000 and 0 are both ≡ 0 (mod 128), so the alarm smoothing runs on two consecutive ticks every 4097 ticks, and from then on the phases are offset from `_GameTime` by 1 per wraparound. **Checked against the original (2026-10-19, vector `volta_cycle`, 307/307):** with `_Cycle` = 4095 written in the input, ticks 4095 → 0x1000 and 0x1000 → 0 both run phase 0 (`_ListIndexA` 108 → 57 → 56 from compaction, `_HistStart` 25 → 26 → 27, the `_HealthB` feeding on both); the next one (0 → 1) does not.

[hypothesis] Approximate half-life: the trail drops by half every 128 ticks, in addition to wear from use. The nest scent only loses 1 every 128 ticks, i.e., it lasts a long time.

## 4. Reading (direction choice)

**Fact seen in the code.** The direction functions are in `SIMTWO_MODULE`. The neighborhood uses `_Dx8/_Dy8` on the 64 × 32 grid with "wrapping around" coordinates (`& 0x3F` and `& 0x1F`); the real edge is handled beforehand, forcing the ant to point inward.

| Function | Map read | Rule |
|---|---|---|
| `_GetForageDir` (`1030:0ab0`) | trail of its own color | highest neighbor; if all are 0, random walk (`_TurnTab[dir][SRand8]`); if the highest neighbor is **lower** than the current cell, returns −1 ("trail ended"); otherwise turns at most 45° towards the highest (`_TurnTab`) |
| `_GetNestDir` (`1030:0c30`) | nest scent of the requested color | current cell = 0: 3/4 of the time points to the last created hole (`_LastNewHoleB/R`), otherwise walks around; otherwise, highest neighbor, with limited turn half of the time. `_DoAttackAnt` requests the map of the **other** color and therefore marches to the enemy nest |
| `_GetAlarmDir` (`1030:0e54`) | alarm | highest neighbor, with limited turn; with no alarm around, random walk |
| `_GetSmellT` (`1028:9612`) | trail of a color in a direction | takes **cell** coordinates; returns 0 outside the map (no wraparound). No call found |

Details seen in the code (our own pseudocode). `cx = x >> 1`, `cy = y >> 1`; "turn towards a" = `_TurnTab[dir][a]` (table in [formigas.md](formigas.md#7-movement-and-direction-choice), extracted to `data/feromonios.json`). "Highest neighbor" goes through directions 0 to 7 and only switches when the value is **strictly** greater: on a tie the lowest-numbered direction wins; with all neighbors at 0 the index stays 0.

Edges (before any map reading; tile x 0–127, y 0–63):

| Position | `_GetForageDir` | `_GetNestDir`, `_GetAlarmDir` (and `_GetRandDir`) |
|---|---|---|
| corner (0, 0) | 3, no random draw | `SRand1(3)` + 2 → 2–4 |
| corner (0, 63) | 1, no random draw | `SRand1(3)` → 0–2 |
| corner (127, 0) | 5, no random draw | `SRand1(3)` + 4 → 4–6 |
| corner (127, 63) | 7, no random draw | (`SRand1(3)` + 6) & 7 → 6, 7, 0 |
| x = 0 | `SRand1(3)` + 1 → 1–3 | `SRand1(5)` → 0–4 |
| y = 0 | `SRand1(3)` + 3 → 3–5 | `SRand1(5)` + 2 → 2–6 |
| x = 127 | `SRand1(3)` + 5 → 5–7 | (`SRand1(5)` + 4) & 7 → 4–7, 0 |
| y = 63 | (`SRand1(3)` − 1) & 7 → 7, 0, 1 | (`SRand1(5)` + 6) & 7 → 6, 7, 0–2 |

Interior, with the exact order of the random draws:

- `_GetForageDir`: reads the current cell; **always draws `SRand8`** (the value is discarded: it would only matter if no neighbor were greater than 0, and in that case it is overwritten); finds the highest neighbor; if it is 0, draws another `SRand8` and turns towards it; if it is lower than the current cell, returns −1; otherwise turns towards it.
- `_GetNestDir`: if the current cell is 0, computes the direction to `_LastNewHoleB/R` (`_GetDir` `1020:10cc`, only from the signs of dx and dy). If the ant is not on the hole, draws `SRand4`; if ≠ 0, turns towards the hole. Otherwise (or if it is on the hole, without drawing `SRand4`), turns towards `SRand8`. If the current cell has scent: highest neighbor, then `SRand2`; if ≠ 0, `dir` = turn towards the highest; and then returns turn towards the highest. That is, **the turn is applied twice half of the time (up to 90°) and once the other half (up to 45°)**; the previous version of this spec said "limited turn half of the time".
- `_GetAlarmDir`: highest alarm neighbor; if it is 0, turns towards `SRand8`; otherwise turns towards it. It does not look at the current cell.
- The color comes from bit 7 of the ant's type (`_GetForageDir`: `type & 0x80`; `_GetNestDir`: `type >> 7`).

Mode triggers tied to scent (`_DoAntSimA` and company):

- Foraging (2) on a cell with **alarm > 0** switches to Alarmed (11).
- Alarmed (11) on a cell **without alarm**: with chance 1/4 draws a new mode; otherwise keeps climbing the gradient.
- Defense (6) climbs the alarm gradient if there is alarm on the cell; otherwise follows the yellow ant.
- Defend Nest (12): nest scent of its own color < 110 → returns to the nest; otherwise walks around (stays near the nest).
- Take off (15): nest scent < 100 → returns to the nest; otherwise walks around.

## 5. Initial state and verification

- **Verified** in `QUICK2.ANT` (tick 78): `_PherMapA`, `_PherMapBT`, `_PherMapRN` and `_PherMapRT` are zeroed; `_PherMapBN` has a single cell with 254, that of the black hole (3, 32) → cell (1, 16). This is as expected: at tick 32, `_FillHolesBN` set 255 and `_ColonySmellBN` removed 1. `_PherMapRN` should have received the same at tick 96, which had not yet arrived.
- **Verified** (test bench case SUP-04, 2026-10-17): at tick 130 the red hole's cell (118, 32) → (59, 16) has 254 in `_PherMapRN`; in the same case, red trail 210 → 105 at tick 96, black trail intact until tick 160 and an isolated alarm of 200 becomes 100 at the center and 25 at the 4 neighbors at tick 128. The port gives the same (`sup04_feromonios_iguais_ao_original`, `engine/tests/regras_superficie.rs`).

## 6. How to test against the original

Test bench: [docs/bancada-qemu.md](../../bancada-qemu.md).

1. **Trail decay.** Experimental mode: use the scent tool on the "Black Trail" layer on a cell far from the ants until the value saturates (3 clicks = 210, starting from 0). Save right after and again after 128, 256 and 384 ticks (controlled by `_GameTime`). Expected: 210 → 105 → 53 → 27 (stops at 0 below 8), discounting the wear if some ant passes there.
2. **Alarm smoothing.** Put alarm on an isolated cell (Alarm layer, Experimental tool) and save before and after a tick ≡ 0 or 64 (mod 128). Compare the neighborhood with the formula in section 3.
3. **Nest gradient.** In a Quick game, save after a few hundred ticks and plot the `_PherMapBN` values from the hole: they should be near 255 at the hole and drop by 1 starting from 120 along the exit paths.
4. **Comparison with the port.** The 5 maps are contiguous blocks of the `.ANT` (offsets 34576…42768, 2048 bytes each). A port test can load the `.ANT`, apply only the decay and smoothing functions on a tick ≡ 32, 64 or 96 and compare byte by byte with the original's save taken on the same tick. These functions do not use the RNG, so the test is deterministic.

## 7. Implementation in the port

Code: `engine/src/rules/feromonios.rs`. Data: `data/feromonios.json`, generated by `tools/extrair_feromonios.py SIMANTW.EXE --json data/feromonios.json` (`_Dx8` `8:0000`, `_Dy8` `8:0008`, `_TurnTab` `8:0024`, plus the constants 8, 9, 200, 255 and 0x51 seen in the pseudocode, in the `regras` block). A mod can change the turn table or the thresholds by overriding that file.

| Original | Port |
|---|---|
| `_PherMapA/BN/BT/RN/RT` | `Feromonios { alarme, ninho_preto, trilha_preta, ninho_vermelho, trilha_vermelha }`, each one a `MapaCheiro` (2048 bytes, column-major) |
| reading/writing in the `.ANT` | `Feromonios::do_save`, `Feromonios::gravar_no_save` |
| `_JamScent*`, `_AlarmHere2` | `MapaCheiro::depositar`, `MapaCheiro::alarme_de_luta` |
| `_AlarmHere` | `MapaCheiro::somar_alarme` |
| `_DecTSmell` | `MapaCheiro::gastar` (on the trail layer of the color) |
| `_ColonySmellBN/RN`, `_ColonySmellBT/RT` | `MapaCheiro::decair_ninho`, `MapaCheiro::decair_trilha` |
| `_SmoothAlarm` | `MapaCheiro::suavizar_alarme` |
| `_FillHolesBN/RN` | `MapaCheiro::preencher_buracos` |
| scent part of `_DoSmells` | `fase_do_ciclo`, `Feromonios::rodar_fase`, `Feromonios::etapa_do_tick` |
| `_GetSmellT` | `Feromonios::trilha_na_direcao` |
| `_GetForageDir`, `_GetNestDir`, `_GetAlarmDir` | `direcao_forrageio`, `direcao_ninho`, `direcao_alarme` (with `GeradorS`) |

Port decisions:

- Coordinates outside the surface: the original does not check and would write outside the map; the port ignores the write and reads 0 (`celula_do_tile` returns `None`). A hole with `x` ≥ 128 in `_HoleMapB/R` is also ignored. No such case was seen in the game.
- `_ExpIncSmell` (Experimental tool) was not ported: it depends on the interface (`GetAsyncKeyState`, `_SmoothMany`, sound).

Tests: `cargo test -p engine` runs the cases built from this spec (module `feromonios`) and `engine/tests/feromonios_saves.rs`:

- reading and writing the 5 maps preserve `QUICK2.ANT` and `QUICK3.ANT` byte by byte;
- the state from section 5 (only the black hole's cell with 254) in both saves;
- `QUICK2.ANT` advanced with only the scent steps: at `_Cycle` 96 the red hole's cell (59, 16) has 254 (the port's prediction for the open item in section 5, not yet checked against the original);
- invariants with maps full of pseudorandom values (decay never increases; smoothed alarm ≤ previous maximum and ≥ 9 or 0);
- `vetores_de_tick_do_original`: for each `*antes*.ANT`/`*depois*.ANT` pair in `tests/referencias/ticks/` (any subfolder), applies to the "before" only the scent step of the "after"'s `_Cycle` and compares the 5 maps with the "after". Differences in cells within 1 tile of an ant in list A (before or after) or of the yellow ant on the surface are counted and ignored, because ants deposit and wear scent on the same tick. Without files, the test warns and passes. **Not yet run with data from the original.**

## Gaps

- Tile 0x51 (`'Q'`) in `_FillHoles*`: meaning (see `tiles-valores.md`).
- Scent reading by the yellow ant and by the spider (`_SpiderScan`), if any.
- Nest exit map (`_ExitMapB/R`, `_FixExitMapB/R`, `_GetExitDirB`, `_GetEnterDirB`): it is the nest's internal "scent" and deserves its own document.
- Which key decreases the scent in the Experimental tool (`GetAsyncKeyState` without the argument visible in the pseudocode).
- Double turn of `_GetNestDir`: in the repository's tick vectors (all 307/307), the surface carriers in mode 3 that remain in mode 3 change direction by 0 (196 cases), ±45° (359), **±90° (91)** and 135° (2) in one tick (count from 2026-10-19 excluding the compaction ticks). The ±90° would not fit in a single `_GetNestDir` turn; since the port reproduces these ticks entirely, the double turn is indirectly confirmed (without an isolated case just for it). The double smoothing at the `_Cycle` wraparound was checked on 2026-10-19 (`volta_cycle`).
- `_GetRandDir`, `_GetDefendDir` and `_GetRedDefendDir` are in `rules/superficie.rs` (private); direction inside the nests has not been ported yet.
- The tick steps (`_FillHoles*`, `_ColonySmell*`, `_SmoothAlarm`) are called by `rules/tick.rs` (phases 1 and 3 and `_FullCount`) via `superficie::rodar_fase_de_cheiro`, which converts the `Estado` maps (`Grade`) to `feromonios::Feromonios` and back.
