*Leia em português: [formigas.md](../../pt-BR/spec/regras/formigas.md)*

# Ants: lists, fields, castes, modes and state machine

This document is the basis for implementing ant behavior one rule at a time. It describes the representation (parallel lists and "life" maps), the encoding of the fields, castes and modes, the choice of caste and mode (control triangles) and the state machine at a high level. The loop that calls all of this is in [simulacao-visao-geral.md](simulacao-visao-geral.md); the scents are in [feromonios.md](feromonios.md).

Evidence convention: the same as in [simulacao-visao-geral.md](simulacao-visao-geral.md#evidence-convention). Variable addresses in the `logical segment:offset` format of `symbols.tsv` (8 = selector `1038`, 9 = `1040`, 10 = `1048`/DGROUP). Texts quoted from SHARED type 4 were decompressed with `tools/lzss_probe.py` (function `lzss`, window filled with 0x20) from the dump of `tools/db_list.py`.

## 1. Planes and coordinates

**Fact seen in the code.**

| Plane | Code (`_MePlane`, `_SetLife`, `_GetLife`) | Terrain map | Life map (who occupies the cell) | Size | List |
|---|---|---|---|---|---|
| Surface (yard/house) | 0 or 1 | `_MapA` `10:28e8` | `_LifeA` `10:68e8` | 128 × 64 | A |
| Black nest | 2 | `_MapB` `10:48e8` | `_LifeB` `10:88e8` | 64 × 64 | B |
| Red nest | 3 | `_MapR` `10:58e8` | `_LifeR` `10:98e8` | 64 × 64 | R |

- Array index: `x × 64 + y`. On the surface, x goes from 0 to 127 and y from 0 to 63 (limits in `_SetLife` `1020:5d18` and `_GetLife` `1020:6040`). In the nest, both go from 0 to 63.
- `_LifeA/B/R` are **not** in the save. They are rebuilt from the lists, or the other way around: `_BuildAntListA` (`1020:3046`), called by `_RandWorld`, builds list A by scanning `_LifeA`.
- **Surface ↔ nest passage.** When a surface ant enters a hole at (x, y), it goes to the nest at position (**y**, 1), facing direction 4 (down). The chosen nest is the black one if x < 64 and the red one otherwise (`_GoInNest` `1028:257a`). To leave, the ant must be on row 0 of the nest, on a hole cell (tile 0x18), and it exits through the hole whose surface column is in `_HoleMapB[x_nest]` (`_GetOutB` `1028:520a`, `_LeaveNestB` `1028:515e`). Reading [hypothesis]: the nest is a vertical cross-section in which the x coordinate corresponds to the surface y row, and the nest's y is the depth.
- `_HoleMapB` (`8:82d2`, 64 bytes) and `_HoleMapR` (`8:8312`): index = surface y row (or nest x column); value = x column of the hole on the surface (0 = no hole). **Verified** in `QUICK2.ANT`: `_HoleMapB[32] = 3` and `_HoleMapR[32] = 118`, that is, holes at (3, 32) and (118, 32).
- **Directions** (8, clockwise, starting at north): tables `_Dx8` (`8:0000`) = `0 1 1 1 0 -1 -1 -1` and `_Dy8` (`8:0008`) = `-1 -1 0 1 1 1 0 -1` (read from `SIMANTW.EXE`). Direction 0 = y−1 (up), 2 = x+1 (right), 4 = y+1, 6 = x−1. `_Dx9`/`_Dy9` (`8:0010`/`8:001a`) have 9 entries: the first is "stopped" and the rest are the 8 directions.

## 2. Parallel lists

**Fact seen in the code** and the save table ([save-ant.md](../formatos/save-ant.md)).

| List | Arrays (1 byte per ant) | Capacity | Counter | Who is in it |
|---|---|---|---|---|
| A | `_AlistX` `8:23a4`, `_AlistY` `8:278e`, `_AlistM` `8:2b78`, `_AlistT` `8:2f62`, `_AlistS` `8:334c` | 1000 | `_ListIndexA` `9:80f0` | all ants on the surface, of **both** colors |
| B | `_BlistX..S` `8:3736…8:3f0e` | 500 | `_ListIndexB` `9:99d4` | inside the **black** nest: blacks and red invaders |
| R | `_RlistX..S` `8:4104…8:48dc` | 500 | `_ListIndexR` `9:72cc` | inside the **red** nest: reds and black invaders |

The yellow ant is **not** in any list. Its state lives in the `_Me*` variables (`_MeLocX/Y`, `_MePlane`, `_MeType`, `_MeDir`, `_MeMode`, `_MeHealth`…).

### Fields

| Field | Meaning | Evidence |
|---|---|---|
| X, Y | position in the list's plane | all movement functions |
| T | **type**: the same byte written to the life map. Encodes color, caste/load and direction (section 3). T = 0 is a free slot (dead or removed ant) | `_AddAntToAList` `1020:2ef0` writes T into `_LifeA`; `_CompactListA` discards T = 0 |
| M | **mode** (current task), 0..19 (section 5) | `_DoAntSimA` `1028:04d8` dispatches on M |
| S | **counter or memory**, depending on the mode: on the surface, how much scent to deposit on the next step (decreases by 1 per step; section 7); in a fight, the winner's T; in the nest, the stage of the carried egg and a flag (bit 3) | `_DoRandAntA`, `_DoReturnFoodAnt`, `_StartFightA`, `_DoNestingB` |

### Insertion, removal and compaction

- **Insert:** `_AddAntToAList/BList/RList` (`1020:2ef0`/`2f4a`/`2fa4`) write into slot `_ListIndex` and increment the counter. They also write T into the life map. If the list is full, they **silently discard**. `_AddAntToList` (`1020:5c78`) chooses the list by plane and calls `_SetLife`.
- **Remove during an iteration:** the code sets T = 0 in the list and clears the life map cell. The slot is only reused after compaction.
- **Compact:** `_CompactListA/B/R` (`1020:2a16`/`2a7a`/`2ade`) slide the entries with T ≠ 0 forward, keeping the order, and decrease the counter. This happens every 128 ticks (one list per phase; see the overview). It also happens on demand: `_GoInNest` compacts list B/R if it is full, and `_ExitHole` (`1020:2db6`) compacts list A (the same algorithm, inlined) if it is full.
- **Immediate removal** with shifting (`_RemoveFromAList` `1020:2b42`, via `_BlockMove`): used by the antlion (`_DoAntLions`) and by `_GetFromAlist`.
- **Search by position:** `_FindInAList(x, y)` (`1020:2c42`) scans the list from back to front and returns the first index with equal X, Y and T ≠ 0, or −1. The B and R versions also take the expected T.
- **Processing order:** from back to front (index `_ListIndex − 1` down to 0).

## 3. Encoding of type T

**Fact seen in the code**, with in-game names taken from the text lists of SHARED type 4 (ids 1100, 1101, 1103), indexed by the group `(T & 0x78) >> 3`.

```
bit 7      : color (0 = black, 1 = red)
bits 6..3  : group (caste + what it carries), 0..15
bits 2..0  : direction 0..7 (in eggs, development stage)
```

| Group | T (black) | Name (4/1101) | Load (4/1103) | Census (`_CastePop*[i]`) | Fight class |
|---|---|---|---|---|---|
| 0 | 0x01–0x07 | egg (stages in 4/1100) | — | [0] eggs | 0 |
| 1 | 0x08–0x0F | Worker | Egg (carrying an egg) | [1] workers | 0 |
| 2 | 0x10–0x17 | Worker | — | [1] | 0 |
| 3 | 0x18–0x1F | Worker | Food | [1] | 0 |
| 4 | 0x20–0x27 | Male | — | [3] males | 2 |
| 5 | 0x28–0x2F | Worker | Dirt | [1] | 0 |
| 6 | 0x30–0x37 | Soldier | — | [2] soldiers | 1 |
| 7 | 0x38–0x3F | Soldier | Food | [2] | 1 |
| 8 | 0x40–0x47 | New Queen | — | [4] new queens | 2 |
| 9 | 0x48–0x4F | Soldier | Dirt | [2] | 1 |
| 10 | 0x50–0x57 | Replete | — | not counted | 0 |
| 11 | 0x58–0x5F | "?" | — | not counted | 0 |
| 12 | 0x60–0x67 | Queen (head) | — | [5] queen | 3 |
| 13 | 0x68–0x6F | Queen (tail) | — | not counted | 3 |
| 14 | 0x70–0x77 | Fight (fight cloud) | — | not counted | 0 |
| 15 | 0x78–0x7F | "?" | — | not counted | 0 |

- Picking up food adds 8 to T (group 2→3 or 6→7). Picking up dirt goes from 2→5 or 6→9 (adds 0x18), and dropping the dirt goes back (`_DoDigOutAntA` subtracts 0x18). Picking up an egg goes from 2→1 (subtracts 8). **Fact seen** in `_DoRandAntA`, `_DoForageAnt`, `_DoDigOutAntA`, `_DoNestingB`.
- **Eggs:** bits 0..2 are the stage. Stages in 4/1100: Weird Egg, New Egg, Egg, 1st-Instar, 2nd-Instar, 3rd-Instar, Pupae, Hatching Pupae. `_IsThisEgg` (`1020:5ec8`) accepts `T & 0x7F` from 1 to 7.
- **Queen:** occupies 2 cells, the head (group 12) and the tail (group 13). The tail lies in the head's direction and equals head + 8. If the other half disappears, the remaining one is also removed (`_SimQueenA` `1028:0a74`, `_SimQueenB` `1028:3dc2`).
- **Yellow ant on the life map:** 0xFF or 0xFE (`_IsYellowAnt` `1020:5720`).
- **Census:** `_CountAnts` (`1020:04de`) sums the three lists and the yellow ant by group (`_TemCensus` `9:7c4e`, 32 words: 16 groups × 2 colors). Then it fills `_CastePopB` (`10:ac94`) and `_CastePopR` (`10:aca0`) as in the "Census" column. `_BpopT`/`_RpopT` = sum of [1]..[5] (without the eggs). **Verified** in `QUICK2.ANT`: `_CastePopB = (2, 2, 0, 0, 0, 1)` and `_BpopT = 3`. List B has 3 eggs (group 0, M = 8), plus the queen's head and tail (groups 12 and 13, M = 9). The two eggs in the census against 3 in the list come from different moments (the census runs every 64 ticks).

## 4. Castes and the caste triangle

**Fact seen in the code.**

- `_casteLevels` (`9:9b64`, 3 unsigned words, 0..65535) = position of the control in the "Workers/Soldiers/Breeders" triangle, in the order **[breeders, workers, soldiers]**. Dragging the control (`_ProcCasteEvent` `1000:bd08`, via `_BoundPointToTri`/`_GetTriLatDist`) updates the levels. When the mouse button is released, the game writes the set into `_casteSetLevels` and recomputes `_IdealCaste` (`8:8622`, 4 words, in %):

  | `_IdealCaste[i]` | caste | formula |
  |---|---|---|
  | 0 | worker | `(casteLevels[1] × 100 + 0x3FFF) / 0xFFFF` |
  | 1 | soldier | `(casteLevels[2] × 100 + 0x3FFF) / 0xFFFF` |
  | 2 | male | `(casteLevels[0] × 50 + 0x3FFF) / 0xFFFF` |
  | 3 | new queen | same as the male (the breeders split half and half) |

  The same calculation exists in `_cvtLevels2IdealCaste` (`1000:c9de`), which is not called in the code.
- Defaults read from the executable: `_casteSetLevels` (`8:863a`, 4 sets of 3) = (0, 39321, 26214), (32767, 16383, 16383), (0, 65535, 0), (0, 0, 65535). `_IdealCaste` = 60, 40, 0, 0. **Verified** in `QUICK2.ANT`: `casteLevels = (0, 39321, 26214)` and `IdealCaste = (60, 40, 0, 0)`.
- [open] The set buttons (objects 0x1306–0x1308) change `_casteLevels`, but I did not see `_IdealCaste` being recomputed on that path. Check in the original whether switching sets changes production.
- `_CasteAuto` (`8:8630`) only appears in the interface code (shows or hides the button group). I found no use in the simulation.

**Choice of the next egg's caste**, `_SetCasteProd` (`1030:026e`), every tick:

```
real[i]  = 100 × CastePopB[i+1] / Σ CastePopB[1..4]     (i = 0..3: worker, soldier, male, new queen)
ideal[i] = 100 × IdealCaste[i] / Σ IdealCaste
chosen   = the i with the SMALLEST difference (real − ideal), counting only differences < 0; if none, i = 0
_MakeMe (8:8a56) = _CasteTabB[chosen]      (_CasteTabB 8:89ae = 2, 6, 4, 8: groups of worker, soldier, male, new queen)
```

**Hatching** (`_SimEggB` `1028:3ca0`, only in the nest): every 32 ticks (if `_BpopT` < 3) or 128 ticks, the egg's stage goes up by 1. When it reaches 8, the egg hatches:

- With `_ModeAuto` = 0, the egg **dies** (`_TotalEggsDiedB` += 1) if `SGRand(255)` > `modeLevels[2] >> 7` ("nurse the brood" level). [hypothesis] With few ants nursing, more eggs are lost. With `_ModeAuto` = 1 the egg always hatches.
- If it does not die: T = `_MakeMe × 8 + 2` (facing right). M = 1 (Nesting) if it is a worker; otherwise, M = `_GetNewModeB(_MakeMe)`.

**Queen lays eggs** (`_SimQueenB`, tail, group 13): every 16 ticks, with probability `SRand128() ≤ _HealthB`, she lays an egg (T = 1) behind the tail, if the position is inside the nest. Each egg laid consumes the food reserve (`_EatCountB` −1; when it becomes negative, `_HealthB` −1 and the reserve restarts at `_BpopT/32`) and counts in `_TotalEggsLaidB`. The head moves with chance 1/64 per tick (`_QueenMoveB`). If `_HealthB` = 0 at that moment, the queen dies of starvation (message 10015).

## 5. Modes

**Fact seen in the code.** Names from 4/1102, in the order of index M. "A" = surface (`_DoAntSimA` `1028:04d8`), "B" = black nest (`_DoNestAntB` `1028:2dae`); the red nest (`_DoNestAntR` `1028:53a4`) mirrors B [hypothesis: faithful mirror; there is an extra function `_StayInR`].

| M | Name (4/1102) | Surface (A) | Nest (B) |
|---|---|---|---|
| 0 | Bored | `_DoRandAntA`: walks at random; on a food cell, a worker or soldier picks up the food (M = 3, S = 200); 1/8 chance of becoming Foraging | 1/32 chance of drawing a new mode; walks |
| 1 | Nesting | `_DoToNestAnt`: returns to the nest following the nest scent | `_DoNestingB`: tends the brood (picks up, carries and drops eggs; eats from the store) |
| 2 | Foraging | `_DoForageAnt`: follows the trail; picks up food (M = 3, S = 200); enters Alarmed if there is alarm on the cell; 1/32 chance of becoming Resting | `_DoDigOutB`: goes to the exit |
| 3 | Got Food | `_DoReturnFoodAnt`: returns to the nest leaving a trail (decreasing S) | `_DoFoodInB`: takes the food to the store |
| 4 | Dig In | `_DoToNestAnt` (same as 1) | `_DoDigInB`: digs |
| 5 | Dig Out | `_DoDigOutAntA`: carries dirt outside and drops it (1/8 per step) | `_DoDigOutB` |
| 6 | Defense | `_DoRecruitAnt`: follows the yellow ant (or the spider, or the alarm) | if the yellow ant is not in the black nest: `_DoDigOutB`; otherwise walks and fights |
| 7 | Attack | `_DoAttackAnt`: goes to the **enemy** nest (follows the nest scent of the other color) | `_DoDigOutB` (red invader: `_RaidInB`) |
| 8 | Hatching | egg on the surface: disappears with chance 1/200 per tick | `_SimEggB` |
| 9 | Egg Laying | queen on the surface: checks whether the tail/head exists | `_SimQueenB` |
| 10 | Fighting | `_DoFightA`: fight cloud (section 6) | `_DoNestFightB` |
| 11 | Alarmed | `_DoToAlarm`: climbs the alarm gradient; without alarm, 1/4 chance of drawing a new mode | `_DoDigOutB` |
| 12 | Defend Nest | nest scent < 110 on the cell: returns to the nest; otherwise walks near it | `_DoDigOutB` |
| 13 | Resting | stopped; on a hole, enters; 1/4 chance per tick of becoming Foraging | stopped; 1/20 chance of drawing a new mode |
| 14 | Pre-Flight | half the time goes to the nest, half walks; if `_FlyAway*` > 100, moves to 15 | walks; if `_FlyAwayB` > 100, moves to 15 |
| 15 | Take off | `_DoRepoExit`: nest scent < 100 → nest, otherwise walks; when `_FlyAway` = 1 (or with chance 1/`_FlyAway`), moves to 16 | `_DoDigOutB` |
| 16 | Flying | with chance 1/32 (and if `_SwarmCnt*` < 50), leaves the map; in the Full Game, `_SwarmCnt*` += 1 and, with chance 1/16, `_QueenStorage*` += 1 | `_DoDigOutB` |
| 17 | ??? | — | drowning [hypothesis]: if the nest tile > 0x13, turns at random and dies with chance 1/100; otherwise draws a new mode |
| 18 | ??? | — | like Bored |
| 19 | Avenger | `_DoRedInitiator`: the red leader that conducts attacks | like Bored |

Common rules on the surface (seen in `_DoRandAntA`, `_DoForageAnt`, `_DoToAlarm`…):

- **Hole:** if the ant is on a hole (tile 0x50 `'P'` in the yard; 0x80–0x8F in the house, `_TERRAINset` = 1), it enters the nest (`_GoInNest`), in almost all modes.
- **Obstacle:** if the destination tile is greater than `_Barrier` (`9:7604`), the ant does not move, it only turns (turn table, section 7).
- **Occupied cell:** if it is the yellow ant of the other color, there is a fight with the yellow ant (`_YellowFight`). If it is the yellow ant of the same color, the ant may feed it (`_DoTroph`, when `_MeWantFood` = 1). If it is an ant of the same color, it turns and draws a new mode. If it is an enemy, there is a fight (section 6).
- **Mode change:** `_GetNewMode(group, T)` (`1030:0910`), explained below.

### Mode choice

**Fact seen in the code** (`_GetNewMode` `1030:0910`, `_GetNewModeB` `1030:09d0`, `_GetNewModeR` `1030:0a50`, `_SetModeProd` `1030:0326`).

| Who | `_ModeAuto` (`9:9fce`) = 1 | `_ModeAuto` = 0 | Reds |
|---|---|---|---|
| Worker (group 2) | `_ModeTabWB[_StrategicModeB][SRand8]` | `_ModeMe` | `_ModeTabWB[_StrategicModeR][SRand8]` |
| Soldier (group 6) | `_ModeTabSB[_StrategicModeB][SRand8]` | `_ModeMe` | `_ModeTabSB[_StrategicModeR][SRand8]` |
| Other groups | `_CasteModeTabB[group]` | same | same |

Tables read from `SIMANTW.EXE` (segment 8):

`_ModeTabWB` (`8:89e6`, row = strategic mode 0..5, 8 draws):

| Strategic | Modes |
|---|---|
| 0 | 1 2 7 7 7 7 7 7 |
| 1 | 4 1 1 1 4 2 2 2 |
| 2 | 13 13 13 4 4 4 2 2 |
| 3 | 13 13 4 4 4 4 4 2 |
| 4 | 1 1 1 2 2 2 2 2 |
| 5 | 1 2 2 2 2 2 2 2 |

`_ModeTabSB` (`8:8a16`):

| Strategic | Modes |
|---|---|
| 0 | 7 7 7 7 7 7 7 7 |
| 1 | 1 2 2 2 2 2 2 2 |
| 2 | 2 2 2 4 4 2 2 2 |
| 3 | 2 4 4 4 4 4 4 2 |
| 4 | 2 2 2 2 2 2 2 2 |
| 5 | 2 2 2 2 2 2 2 2 |

`_CasteModeTabB` (`8:8a46`, by group 0..15): `8 1 0 3 14 5 0 3 14 5 0 0 9 9 10 0`. That is: egg → 8, carrying an egg → 1, with food → 3, with dirt → 5, male and new queen → 14 (Pre-Flight), queen → 9, fight → 10.

**Task triangle** (Forage/Dig/Nurse): `_modeLevels` (`9:9c74`, 3 words: **[forage, dig, nurse]**), sets in `_modeSetLevels` (`8:8658`): (60%, 20%, 20%), (100%, 0, 0), (0, 100%, 0), (0, 0, 100%). **Verified** in `QUICK2.ANT`: `modeLevels = (39321, 13107, 13107)`. `_SetModeProd`, every tick:

```
total     = ModePopB[0] + ModePopB[1] + ModePopB[2]
ideal[i]  = modeLevels[i] × total / 65535
lack[i]   = ideal[i] − ModePopB[i]
chosen    = the i with the LARGEST positive lack (if none, 0)
_ModeMe (8:8a58) = _ModeTabB[chosen]       (_ModeTabB 8:89b6 = 2, 4, 1: Foraging, Dig In, Nesting)
```

`_ModePopB` (`9:9e70`, 6 words) is built at the end of each tick from that tick's count per mode (`_TemBModePop`, A + B):

| `_ModePopB[i]` | Modes summed |
|---|---|
| 0 forage | 2 + 3 |
| 1 dig | 4 + 5 |
| 2 nurse | 1 |
| 3 | 7 (Attack) |
| 4 | 12 (Defend Nest) |
| 5 | 6 (Defense) |

`_ModePopR` (`9:a084`) follows the same rule. `_TemRModePop[19]` counts the red leaders.

`_ModeAuto` and `_CasteAuto` start at 1 (`_initControls` `1000:cc84`); **verified** in `QUICK2.ANT`. [hypothesis] Moving the task triangle turns off `_ModeAuto` (`_ProcModeEvent` `1000:c07a` clears the flag in the drag branches); still to be checked.

## 6. Fighting, death and health

**Fact seen in the code.**

- **Encounter:** when an ant tries to enter a cell with an enemy, the one moving **disappears immediately** (T = 0). The one in the cell becomes a fight cloud: T = 0x70 + the winner's color bit, M = 10, S = the winner's T. Alarm ≥ 40 is also marked there (`_AlarmHere2`). The winner comes from `_GetWinner` (`1028:26f4`):

  ```
  class = table by group (DGROUP 0x22C6): 0,0,0,0,2,0,1,1,2,1,0,0,3,3,0,0
           (0 = worker/egg, 1 = soldier, 2 = breeder, 3 = queen)
  P[a][b] = chance (out of 10) of a beating b, 4×4 matrix at DGROUP 0x22D6:
            worker     5 2 7 3
            soldier    8 5 9 4
            breeder    3 1 5 2
            queen      7 6 8 5
  a wins (the one that was in the cell) if RRand(10) < P[class(a)][class(b)]
  ```

  The loser counts in `_BlackLost`/`_RedLost` and `_BAntsKilled`/`_RAntsKilled`. With `_Invincible` = 1, the red always loses. The segment of the two tables was deduced from the contents of `SIMANTW.EXE` (the matrix rows sum to 10 with the transpose); they have no name in the `.SYM`.
- **Fight cloud** (`_DoFightA` `1028:27e6`): every tick it redraws itself. With chance 1/16, the fight ends: T returns to the winner's (S), a new mode is drawn and a corpse is placed on the map (`_DeadAntHere`).
- **Corpse** (`_DeadAntHere` `1028:28c0`): stores the position in a ring of 100 positions (`_DeadX` `9:9c82`, `_DeadY` `9:9d76`, pointer `_DdPtr`). The oldest corpse in the ring disappears from the map. The tile becomes "dead ant" (edible), with values specific to each color (see `tiles-valores.md`).
- **Natural death (surface):** every tick, for each slot of list A, it draws `SRand256()`. If it comes out 0 and the slot is occupied, it compares the colony's health (`_HealthB`/`_HealthR`) with `SRand32()`; if the health is lower, the ant dies (corpse, `_BAntsExpired` += 1, including for reds). Health ≥ 32 prevents this death.
- **Natural death (black nest):** the same, only for blacks and non-queens (M ≠ 9), without a corpse. Here `SRand256()` is only drawn for living ants: `_DoAntSimB` skips the slots with T = 0 before calling `_DoNestAntB`.
- **Colony health** (`_HealthB` `10:ac86`, `_HealthR` `10:ac88`, 0..100): loses 1 every 64 ticks. Gains 1 when the ants eat enough: each bite from the store adds 5 to `_EatCountB`, and when it exceeds (`_BpopT` + soldiers)/16 the health goes up by 1 and the counter resets (`_EatFoodB` `1028:4844`, `_TryEatFoodB`, `_DoNestingB`). Eggs consume the reserve (`_DecEatB` `1028:48f8`).

## 7. Movement and direction choice

**Fact seen in the code.**

- **Turn table** `_TurnTab` (`8:0024`, 8 × 8 bytes): `TurnTab[current_dir][target]` gives the new direction, turning at most 1 step (45°) toward the target. The 8 rows, read from the executable:

| current dir. | target 0 1 2 3 4 5 6 7 |
|---|---|
| 0 | 0 1 1 1 7 7 7 7 |
| 1 | 0 1 2 2 2 2 0 0 |
| 2 | 1 1 2 3 3 3 3 1 |
| 3 | 2 2 2 3 4 4 4 4 |
| 4 | 3 3 3 3 4 5 5 5 |
| 5 | 6 6 4 4 4 5 6 6 |
| 6 | 7 7 7 5 5 5 6 7 |
| 7 | 0 0 0 0 6 6 6 7 | The random walk uses `TurnTab[dir][SRand8()]`, which favors keeping the direction or turning 45°.
- **Edges:** in all direction functions, an ant on the edge (x = 0 or 127, y = 0 or 63) draws a direction that points inward. `_GetForageDir` uses 3 options and fixed corners; `_GetNestDir` and `_GetAlarmDir` use 5 options in the middle of the edge and 3 in the corners (table in [feromonios.md](feromonios.md#4-reading-direction-choice)).
- `_GetRandDir` (`1030:0f72`): random walk.
- `_GetForageDir` (`1030:0ab0`): looks at the trail of its own color in the 8 neighboring cells (2×2 resolution) and picks the largest. If none has a trail, it does a random walk. If the largest is **smaller** than the current cell's, it returns −1 ("end of trail"): `_DoForageAnt` then puts the ant in Bored (7/8) or draws a new mode (1/8).
- `_GetNestDir` (`1030:0c30`): climbs the gradient of the nest scent of the requested color (`_DoAttackAnt` requests the enemy color). Without scent on the cell: 3/4 of the time it points to the last hole made (`_LastNewHoleB/R`, `8:835a`/`8:835e`), otherwise random walk. With scent: picks the neighbor with the highest value and applies the limited turn once or, half the time (`SRand2` ≠ 0), twice in a row (up to 90°). Details and the order of the random draws in [feromonios.md](feromonios.md#4-reading-direction-choice).
- `_GetAlarmDir` (`1030:0e54`): climbs the alarm gradient (map shared by both colors).
- `_GetDefendDir` (`1030:1026`): follows the yellow ant. If the yellow ant is in the nest, it follows that color's nest scent; if it is hunting the spider, it goes to the spider; if it is near the fuzzy position (`_FuzLocX/Y`), it wanders.
- `_Recruit(n)` (`1030:06d2`): puts into Defense (M = 6, S = 0) up to n black workers or soldiers that are not yet in that mode. Scans list A and then B, from back to front.
- **Nest exit** (`_ExitHole` `1020:2db6`): the ant appears on the first valid neighboring cell of the hole, testing directions 0 to 7, that has tile < 0x50. If the mode is not 3, 6 or 7 and the hole is on the ant's own side of the map (blacks with x < 64; reds with x ≥ 65), S = 120 (it will deposit nest scent). In mode 6, S is preserved.

## 8. Red leader ant ("Avenger", mode 19)

**Fact seen in the code.** If no red is in mode 19 and `_BpopT` ≥ 30, `_MakeRedInitiator` (`1028:967c`) turns the last red of list A into a leader: T = 0xB0 (red soldier), M = 19, S = 0, `_RedTask` = 0, `_RedInitOn` = 1. `_DoRedInitiator` (`1028:96d4`) conducts red recruitments and attacks (`_RecruitRed`, `_UnRecruitRed`, `_GetNewRedTask`). Details [open].

## 9. Yellow ant (summary)

**Fact seen in the code.** `_DoAntSimY` (`1028:7cf6`) runs every tick. With `_MeMode` = 0, it:

- writes the yellow ant into the life map;
- takes 1 health every 64 ticks, and 1 more per tick inside the nest if the cell's tile is > 0x4D;
- if the yellow ant is a queen (`_MeType` = 0x60) inside a nest, every 16 ticks it may lay an egg (costs 5 health).

`_DoAntMoveY` (`1028:75f4`) moves the yellow ant when there is an order (`_MeMoveMe`). It can deposit trail (`_MeScent`, which starts at 200 when picking up food and does not go below 10) and alarm (+50 per step, maximum 200, with `_MeDropAlarm`). Death and reincarnation are in `_YellowDeath` (`1000:7746`). The search order for a new body is: egg in the black nest, black adult in the black nest, black adult on the surface, black adult in the red nest. Details of the yellow ant in a document of its own [open].

## 10. Functions present but not called

No call found in the pseudocode: `_DoDefendNest` (`1028:0df6`), `_DoRepoLoit` (`1028:0c1e`), `_DoRestAnt` (`1028:0b76`), `_DoRepoFly` (`1028:0d4a`), `_SimEggA` (`1028:0a1c`), `_SimQueenA` (`1028:0a74`), `_GetSmellT` (`1028:9612`), `_cvtLevels2IdealCaste`, `_GetFromAlist`. The equivalent logic is inlined in `_DoAntSimA`: cases 8, 9, 13, 14 and 16 repeat `_SimEggA`, `_SimQueenA`, `_DoRestAnt`, `_DoRepoLoit` and `_DoRepoFly`. [hypothesis] They are leftovers from the Mac port or calls through pointers that Ghidra did not resolve.

## 11. How to test against the original

Test bench: [docs/bancada-qemu.md](../../bancada-qemu.md). Suggested reading tool: a `tools/ant_dump.py`, yet to be written, that uses the offsets from [save-ant.md](../formatos/save-ant.md).

1. **Encoding of T.** In the original, open the Examine window on known ants (worker, soldier, with food) and compare with the T in the list of the save made right after (position X, Y). Expected: the names in the table of section 3.
2. **Hatching and castes.** Quick game, caste triangle at the "Soldiers" vertex, without `_ModeAuto`. Save every ~128 ticks and count the new groups in list B: only soldiers should be born (group 6, T = 0x32).
3. **Modes.** With `_ModeAuto` off and the task triangle on "Dig", check in successive saves that the new assignments of workers and soldiers go to M = 4 (Dig In).
4. **List and compaction.** In successive saves, `_ListIndexA` can only drop on ticks ≡ 0 (mod 128). Outside those ticks, slots with T = 0 remain.
5. **Fighting.** In Experimental mode, place a black worker and a red one side by side and repeat several times. The proportion of wins should tend to 50%; soldier against worker, to 80%.
6. **Comparison with the port.** Load the same `.ANT` in the port, run N ticks with the same RNG and compare the lists slot by slot, in the same order, as well as `_TemCensus`, `_CastePop*`, `_ModePop*`, `_MakeMe` and `_ModeMe`.

## Gaps

- Detailed rules of each `_Do*Ant` (step by step) and of the nest functions `_DoNestingB`, `_DoFoodInB`, `_DoDigInB`, `_DoDigOutB`, `_RaidInB/OutB`, `_DoNestFightB`, `_QueenMoveB`, and of the differences of the red nest.
- Meaning of groups 10 (Replete), 11 and 15, and where they come from.
- Mode 17 (hypothesis: drowning) and mode 18.
- Exact effect of switching the memorized set in the caste triangle.
- Yellow ant, spider, antlion and caterpillar: documents of their own.
