*Leia em português: [ciclo-de-vida.md](../../pt-BR/spec/regras/ciclo-de-vida.md)*

# Life cycle: eggs, queen and castes

This document specifies, for a 1:1 reimplementation, how the queen lays eggs, how the egg develops into an adult, how the adult's caste is chosen and how the queen (two cells, head and tail) lives, walks and dies. The behavior of the other ants inside the nest is in [ninho.md](ninho.md); lists, T encoding, mode tables and the caste triangle, in [formigas.md](formigas.md); the tick order, in [simulacao-visao-geral.md](simulacao-visao-geral.md); the generators, in [rng.md](rng.md).

## Conventions

The same as in [ninho.md](ninho.md#conventions): fact seen in the code unless noted otherwise; `S(n)`, `Sk(K)`, `SG(n)` = S generator; `RR(n)` = R generator; `D8[d]` and `inv(d) = d XOR 4`; X, Y, M, T, S of the ant being processed (index `_Tindex`); `Vida` = `_LifeB`/`_LifeR`. Our own pseudocode, not a copy of the decompiled code.

## 1. Overview

| Phase | Representation | Who processes it |
|---|---|---|
| egg and brood | nest list entry with group 0: T = stage 1..7 (black) or 0x80 + stage (red), M = 8 | `_SimEggB` (`1028:3ca0`) / `_SimEggR` (`1028:62a6`) |
| carried egg | does not exist in the list: the stage stays in the S of the worker carrying it (black nest only; section 4) | `_DoNestingB` |
| adult | T = group × 8 + direction (+ 0x80 if red) | [ninho.md](ninho.md) and `_DoAntSimA` |
| queen | two entries: head (group 12) and tail (group 13), M = 9 | `_SimQueenB` (`1028:3dc2`) / `_SimQueenR` (`1028:6386`) |

Stages (bits 0–2 of T). The names come from the SHARED 4/1100 text list, in order: Weird Egg, New Egg, Egg, 1st-Instar, 2nd-Instar, 3rd-Instar, Pupae, Hatching Pupae ([formigas.md](formigas.md#3-encoding-of-type-t)). [hypothesis: the index in the list is the stage; "Weird Egg" (0) never appears, since T & 0x7F = 0 is a free slot]

| T & 7 | Name | How it gets there |
|---|---|---|
| 1 | New Egg | laid by the queen (black and red) and by the yellow-queen |
| 2 | Egg | 1 advance; or red egg dropped by a nurse (always 0x82) |
| 3 | 1st-Instar | |
| 4 | 2nd-Instar | |
| 5 | 3rd-Instar | |
| 6 | Pupae | |
| 7 | Hatching Pupae | |
| (8) | hatches | the advance that would lead to 8 turns the entry into an adult |

## 2. Laying (queen's tail)

`_SimQueenB`, tail branch (group 13), executed every tick:

```
Vida[pos] = T
if _BlkQueens > 0:                                          (R: _RedQueens)
    h = pos + D8[T & 7]                                      (the head is in front of the tail)
    if Vida[h] ≠ T − 8 and _FindInBList(h, T − 8) < 0:        (head gone)
        _BlkQueens −= 1; T = 0; Vida[pos] = 0; END
e = pos + D8[inv(T & 7)]                                     (cell behind the tail)
if 0 ≤ e.x ≤ 63 and 1 ≤ e.y ≤ 63 (_InNestBounds):
    _LastBlackEgg = e                                        (R: _LastRedEgg, 8:8366)
    if (_Cycle & 0xF) == 0:
        r = Sk(128)
        if r ≤ _HealthB:
            _PlaceEggB(e, 1)                                  (R: _PlaceEggR(e, 0x81))
            _EatCountB −= 1
            if _EatCountB < 0:
                _EatCountB = _BpopT >> 5
                if _HealthB > 0 and _AlwaysHealthy == 0: _HealthB −= 1     (R: without the _AlwaysHealthy test)
            _TotalEggsLaidB += 1                              (32 bits; R: there is no counter)
```

- Chance of laying, per opportunity (every 16 ticks): (`_HealthB` + 1)/128, i.e., 101/128 ≈ 79 % with health 100.
- `_PlaceEggB` ([ninho.md](ninho.md#76-laying-an-egg-_placeeggb-10201004--_placeeggr-10201068)) digs the cell if it is soil (the queen widens the chamber by laying eggs), appends the egg at the **end** of the list and does not test occupancy: several eggs can end up in the same cell. With the list full (500), the egg is not created, but health and the counter are charged anyway.
- The health cost: while `_BpopT` < 32, `_EatCountB` goes back to 0 and **each egg costs 1 health**; the reserve only goes up by eating ([ninho.md](ninho.md#61-eating-_tryeatfoodb-102847c6-_eatfoodb-10284844)).
- `_LastBlackEgg` is updated on every tick in which the cell behind is inside the nest, whether an egg is laid or not.

**Fact verified in `QUICK2.ANT`** (tick 78, by manual calculation over the saved values):

- `_HealthB` = 96 = 100 − 1 (feeding at tick 64) − 3 (three eggs); `_HealthR` = 95 = 100 − 1 − 4 (four eggs); `_EatCountB` = `_EatCountR` = 0; `_TotalEggsLaidB` = 3. The opportunities were ticks 16, 32, 48 and 64.
- Black eggs (T = 1) at (34, 9) and twice at (36, 9); red eggs (T = 0x81) four times at (39, 11), behind the red tail (40, 11) facing right. `_LastBlackEgg` = (36, 9), behind the black tail (35, 9) facing left.

## 3. Development and hatching

### 3.1 Black nest: `_SimEggB` (`1028:3ca0`)

```
t = T; born = false
P = (_BpopT < 3) ? 32 : 128
if (_Cycle mod P) == 0:
    t = t + 1
    if (t & 0xF) == 8:                                         (hatches)
        if _ModeAuto == 0 and SG(255) > (_modeLevels[2] >> 7): (SG only with _ModeAuto = 0)
            t = 0; _TotalEggsDiedB += 1                         (died)
        else:
            c = _MakeMe                                         (2, 6, 4 or 8; section 6)
            t = c × 8 + 2                                       (facing right)
            M = (c == 2) ? 1 : _GetNewModeB(c)
            born = true
if _OptionStates[5] ≠ 0 and not born: egg balloon (no random draw)
Vida[pos] = t; T = t; S = 0
```

- **All eggs advance on the same tick**, since the test is on `_Cycle`, common to all of them. An egg laid on tick t advances on the next 7 ticks with `_Cycle` a multiple of P (the laying tick itself does not count: the new egg is at the end of the list and is only processed on the next tick). Duration of an egg laid by the queen (always on a tick that is a multiple of 16): between 6P + 16 and 7P ticks, that is, 784–896 ticks in a colony with 3 or more adults and 208–224 in a newborn colony (if P does not change midway). An egg dropped by a nurse, on any tick, takes between 6P + 1 and 7P from then on, counting the stage it already had.
- P is chosen every tick with the current `_BpopT`, which only changes when `_CountAnts` runs (ticks ≡ 0 and 64 mod 128).
- `_Cycle` goes from 0 to 0x1000 (4097 values) and back to 0: the ticks with `_Cycle` = 0x1000 and 0 are consecutive and **both** count, so once every 4097 ticks the eggs advance two stages in two consecutive ticks. The same applies to laying (section 2).
- Death at hatching: only with `_ModeAuto` = 0 (the player touched the task triangle; [formigas.md](formigas.md#mode-choice)). With L = `_modeLevels[2] >> 7` ("nurse" level, 0..511) and `SG(255)` = minimum of two random draws 0..254, the chance of dying is ((254 − L)/255)² if L < 254, and 0 if L ≥ 254. Examples: nurse 0 % → 99.2 %; 20 % (13107, L = 102) → 35.5 %; 50 % (L = 255) → 0.
- Mode at birth (`_GetNewModeB`, tables in [formigas.md](formigas.md#mode-choice)): worker → 1 (Nesting), no random draw; soldier → `_ModeTabSB[_StrategicModeB][Sk(8)]` with `_ModeAuto` = 1, or `_ModeMe` with 0; male and new queen → 14 (Pre-Flight), no random draw.
- An egg in the list also goes through the nest's natural death random draw ([ninho.md](ninho.md#31-owner-ant)), before `_SimEggB`.

### 3.2 Red nest: `_SimEggR` (`1028:62a6`)

```
t = T; born = false
P = (_RpopT == 1) ? 32 : 128                                    (exactly 1, not "< 3")
if (_Cycle mod P) == 0:
    t = t + 1
    if (t & 0xF) == 8:
        c = _CasteTabC[(_StrategicModeR mod 7) × 8 + Sk(8)]
        t = c × 8 + 0x82
        M = _GetNewModeR(c)                                     (Sk(8) inside, if c = 2 or 6)
        born = true
balloon as in black; Vida[pos] = t; T = t; S = 0
```

No death at hatching. Order of the random draws at hatching: the caste's `Sk(8)` and, afterwards, the `Sk(8)` of `_GetNewModeR` (for worker and soldier).

`_CasteTabC` (`8:897e`), row = `_StrategicModeR` (0..5), column = `Sk(8)`, values read from `SIMANTW.EXE` (2 = worker, 4 = male, 6 = soldier, 8 = new queen):

| `_StrategicModeR` | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | Summary |
|---|---|---|---|---|---|---|---|---|---|
| 0 (attack) | 2 | 2 | 2 | 2 | 2 | 6 | 6 | 6 | 5/8 worker, 3/8 soldier |
| 1 (normal) | 4 | 4 | 8 | 8 | 2 | 2 | 2 | 6 | 1/4 male, 1/4 new queen, 3/8 worker, 1/8 soldier |
| 2 | 4 | 8 | 2 | 2 | 2 | 2 | 6 | 6 | 1/8 male, 1/8 new queen, 1/2 worker, 1/4 soldier |
| 3 | 2 | 2 | 2 | 2 | 2 | 6 | 6 | 6 | same as 0 |
| 4 (hunger) | 2 | 2 | 2 | 2 | 2 | 2 | 6 | 6 | 3/4 worker |
| 5 (hunger) | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 6 | 7/8 worker |

Row 6 (reachable only if `_StrategicModeR` = 6, which does not happen) would be the start of `_CasteTabB`.

Mode at birth in red: worker and soldier draw from `_ModeTabWB/SB[_StrategicModeR]`; male and new queen → 14.

## 4. Eggs carried by nurses

| | Black nest (`_DoNestingB`) | Red nest (`_DoNestingR`) |
|---|---|---|
| Pick up | free worker (group 2, S < 8) whose cell shows an egg in `Vida`: the egg leaves the list (T = 0), the nurse becomes group 1 and stores the egg's T in S | free worker, with chance 1/4 per tick; the egg leaves the list; S stores nothing; accepts black or red egg |
| Carry | goes down the exit map to a local minimum; sets S bit 3; drops it on the next tick | walks at random or goes down (3/4 goes down, if there is somewhere to go) |
| Drop | `_PlaceEggB(pos, S & 7)`: **same stage** | with chance 1/4 in a cell without food: `_PlaceEggR(pos, 0x82)`: **back to stage 2** |
| During transport | the egg does not age | same |

Details and order of random draws in [ninho.md](ninho.md#5-tending-the-brood-m--1-_donestingb-102844a8). Consequence: in the red nest, an egg that is carried once restarts its development; the more nurses, the slower the birth [hypothesis about the in-game effect].

## 5. Queen

### 5.1 Shape

- Two entries in the nest list, both with M = 9 and S = 0: **head** T = 0x60 + d (black) / 0xE0 + d (red) and **tail** T = head + 8, in the cell behind the head: tail = head + D8[inv(d)]. The tail has the same direction d. Eggs come out behind the tail: head + 2 × D8[inv(d)].
- Creation: at world generation (`_PlaceBlackQueen`, red queen inline in `_RandWorld`; [geracao-mundo.md](geracao-mundo.md#3c-queens-_placeblackqueen-103065ce-and-the-red-queen)), with T = 0x62/0x6A (0xE2/0xEA). In game: `_MakeBlkQueen(x, y, d)` (`1030:671a`) / `_MakeRedQueen` (`1030:6906`): digs (x, y), (x, y) + D8[inv(d)] and (x, y) + 2·D8[inv(d)] with `_DigTile*`, appends head and tail and adds 1 to `_BlkQueens`/`_RedQueens`. Callers: `_YellowBirth` (`1000:739e`, the yellow-queen leaves a real queen behind when switching bodies; details with the yellow ant) and `_CheatKeys`.
- Counters `_BlkQueens` (`9:78e8`) and `_RedQueens` (`9:79dc`): go up on creation; go down when the head notices the tail is gone (always) or when the tail notices the head is gone (only if the counter is > 0).
- The queen does not suffer natural death ([ninho.md](ninho.md#31-owner-ant): M = 9 is excluded) nor drowning (groups 12–13 are left out of `_DrownBList`).

### 5.2 Head, every tick (`_SimQueenB`, group 12)

**Verified (2026-10-19, vectors `rainha_fome` and `rainha_vermelha_fome`, testes-tick.md "Sétima rodada"):** with `_HealthB` = 0 the black head dies at 1775 with dialog 10015 opened inside the tick and the tail disappears on the next tick, when `_BlkQueens` goes to 0; the same for the red one (10016) with `_HealthR` = 0. The immunity to natural death (M = 9) is in vector `ninho_n02`.

```
if Sk(64) == 0:
    if _HealthB == 0:                                   (R: _HealthR)
        T = 0; Vida[pos] = 0
        dialog 10015 "The black queen in this patch starves from lack of nourish…" (R: 10016, "The red queen…")
        END                                             (_BlkQueens does not change here; the tail takes care of it)
    if QUEEN_WALK(): END                                (5.3)
c = pos + D8[inv(T & 7)]
if Vida[c] ≠ T + 8 and _FindInBList(c, T + 8) < 0:       (tail gone)
    _BlkQueens −= 1; T = 0; Vida[pos] = 0
else Vida[pos] = T
queen balloon if _OptionStates[5] ≠ 0
```

Death by starvation: with health 0, chance 1/64 per tick (the same random draw that decides whether she tries to walk). The game ending comes later, when `_CountAnts` no longer finds the queen ([simulacao-visao-geral.md](simulacao-visao-geral.md#8-game-over-and-scoring)).

### 5.3 Walking: `_QueenMoveB` (`1028:4154`) and `_GetBestDir` (`1028:405e`)

The queen walks towards the **center of mass of the diggings** of her own nest (`_TileMassXB/YB`, `9:7c48`/`9:7c90`; R: `_TileMassXR/YR`), recalculated at every dug cell ([ninho.md](ninho.md#73-digging-functions)).

`_GetBestDir(plane, x, y, targetX, targetY)` returns a direction, −1 (already at target) or −2 (no neighbor gets closer):

```
lim = dist²((x, y), target)                      (_GetDis: dx² + dy²)
if lim ≤ 0: return −1
best = −1; fallback = −2
for k = 0..7:
    c = (x, y) + D8[k]; v = Map(c)
    if _IsNotObstacle(c) and not _IsThisPebble(v) and dist²(c, target) < lim:   (in the nest: v < 0x19)
        lim = dist²(c, target)                                              (the limit drops with each candidate)
        if Vida(c) == 0 and _IsClearTile(c) (v < 8): best = k
        else: fallback = k
return (best ≥ 0) ? best : fallback
```

Note that the limit is also updated by occupied candidates, and that the fallback is an occupied cell (or one with food, or 0x14/0x18) that gets closer: the queen walks over eggs and over her own tail.

`QUEEN_WALK()`:

```
d = _GetBestDir(2, X, Y, _TileMassXB, _TileMassYB)
if d == −1: return false
if d == −2: d = Sk(8)
if not (Y > 2 or 3 ≤ d ≤ 5): return false                (near the surface she only walks downward)
if not MOVE(d): return false                             ([ninho.md](ninho.md#41-walking-_trymovedirb-1028439e--_trymovedirr-10286850-and-wander))
c = old position + D8[inv(old dir)]                      (where the tail was)
Vida[c] = 0
j = _FindInBList(c, 0x68 + old dir)                      (R: 0xE8 + old dir)
if j ≥ 0 and T[j] ≠ 0: X[j], Y[j] = old head position; T[j] = 0x68 + d; Vida[old position] = T[j]
return true
```

The tail takes the old position of the head, with the new direction. If the queen walks backward (d = inv(dir)), head and tail swap places. Right after walking, the head's cell may have `Vida` = 0 until the next rewrite (when she enters the tail's old cell). In red, Ghidra shows the searched type as `(d & 7) − 0x18`; the disassembly (`1028:66a6`) confirms it is `0xE8 + d` as a byte.

**Fact verified in `QUICK2.ANT`** (by manual reconstruction): the black queen was created with the head at (36, 9) facing right (T = 0x62) and the tail at (35, 9); the saved center of mass is (32, 6). On the first attempt, the only neighbor that gets closer is the tail's (occupied), so `_GetBestDir` returns fallback 6 and the queen walks backward: head (35, 9) T = 0x66, tail (36, 9) T = 0x6E. On the second, the only one that gets closer is (34, 9), occupied by the first egg: head (34, 9), tail (35, 9). This is exactly what the save shows (list B: 0x66 at (34, 9), 0x6E at (35, 9)), and the eggs laid after that fall at (36, 9), behind the new tail.

### 5.4 Queen in a fight

- If an invader is on the head's cell, the head is erased and the entry becomes a fight cloud with `_GetWinner(0, invader)`; the queen does not come back ([ninho.md](ninho.md#32-invader-ant)). The tail disappears through the test in section 2.
- If a fight cloud ends with a queen-head T (group 12), `_DoNestFightB/R` recreates the tail behind it, without touching `_BlkQueens` (inlined `_MakeNewTailB`; [ninho.md](ninho.md#44-fight-cloud-_donestfightb-10283a54--_donestfightr-10286072)). This requires a queen head to be the "enemy found" in a fight check (in the black nest the checked range, 0x88–0xE7, includes the red head 0xE0–0xE7), which does not happen in normal play, since each queen only exists in her own nest's list. [hypothesis: safety code with no practical effect]
- `_LostHeadB` (`1028:42de`), `_LostTailB` (`1028:433c`) and `_KillTailB` (`1028:42b0`) are the tests and the removal described above, in separate functions with no caller.

## 6. Caste choice

### 6.1 Black nest: `_SetCasteProd` (`1030:026e`), every tick, inside `_GetStrategy`

```
p = sum of _CastePopB[1..4]; q = sum of _IdealCaste[0..3]
real[i]  = (p ≥ 1) ? 100 × _CastePopB[i+1] / p : 0          (i = 0..3; 32-bit integer division)
ideal[i] = (q ≥ 1) ? 100 × _IdealCaste[i] / q : 0
lowest = 0; e = 0
for i = 0..3: if real[i] − ideal[i] < lowest: lowest = real[i] − ideal[i]; e = i      (tie: the first)
_MakeMe = _CasteTabB[e]                                      (8:89ae = 2, 6, 4, 8)
```

`_CastePopB` = (eggs, workers, soldiers, males, new queens, queen), updated by `_CountAnts` every 64 ticks; `_IdealCaste` comes from the caste triangle ([formigas.md](formigas.md#4-castes-and-the-caste-triangle)). No random draw. The egg hatches with the `_MakeMe` of that moment.

### 6.2 Red nest

There is no red `_SetCasteProd`: the caste is drawn at hatching via `_CasteTabC` (section 3.2), according to the red strategic mode.

## 7. Egg losses

| Cause | Black | Red | Counter |
|---|---|---|---|
| nest natural death (`Sk(256)` = 0 and `Sk(32)` > health) | yes | yes | `_BAntsExpired` / `_RAntsExpired` |
| hatching with `_ModeAuto` = 0 | yes | no | `_TotalEggsDiedB` |
| invader steps on the egg's cell (steals it as food) | yes | yes | `_TotalEggsDiedB` (black only) |
| black-type egg (T < 8) inside the red nest | — | destroyed on the next tick | none |
| full list when laying | not created | same | `_TotalEggsLaidB` counts it anyway (black) |
| flooding | does not affect | does not affect | — |

## 8. The yellow ant as queen (summary)

Fact seen in `_DoAntSimY` (`1028:7cf6`); full rule with the yellow ant. With `_MeType` = 0x60 inside a nest (`_MePlane` > 1), when `_Cycle & 0xF` = 0: cell e = position + 2 × D8[inv(`_MeDir`)]; if `_InNestBounds(e)` and `Sk(128)` ≤ `_MeHealth`: if `_MeColor` = 0, `_PlaceEggB(e, 1)` and `_DecEatB`; otherwise `_PlaceEggR(e, 1)` and `_DecEatR`; and `_MeHealth −= 5`. The list is chosen by **color**, not by plane; and the red egg is laid with T = 1 (black type), which `_DoNestAntR` destroys on the next tick. [hypothesis: bug; only matters if the yellow ant is red]

## 9. Red × black differences

| Point | Black | Red |
|---|---|---|
| Egg period | 32 if `_BpopT` < 3, otherwise 128 | 32 if `_RpopT` = 1, otherwise 128 |
| Caste | `_MakeMe` (player's triangle) | `_CasteTabC[_StrategicModeR][Sk(8)]` |
| Death at hatching | with `_ModeAuto` = 0 | never |
| Mode at birth | worker → fixed 1; soldier with `_ModeAuto`/`_ModeMe` | `_GetNewModeR` (worker and soldier draw) |
| Carried egg | keeps the stage | back to stage 2 |
| Laying | `_TotalEggsLaidB`, respects `_AlwaysHealthy` | no counter, no `_AlwaysHealthy` |
| Queen starvation | message 10015 | 10016 |

## 10. Test cases against the original

Same test bench and same method as [ninho.md](ninho.md#13-test-cases-against-the-original) (edit `QUICK2.ANT`, load, run, save). D = deterministic; E = statistical.

| Id | Rule | Input | Expected | Type |
|---|---|---|---|---|
| C-01 | egg advance (3.1, 3.2) | `QUICK2.ANT` unchanged (tick 78; `_BpopT` = 3, `_RpopT` = 2) | saving between ticks 128 and 255: black eggs with T = 2 and red ones with T = 0x82, all at the same time; between 256 and 383: 3 and 0x83 (holds if `_BpopT` stays ≥ 3 and `_RpopT` ≠ 1, and if no egg died or was carried) | D |
| C-02 | short period (3.1) | QUICK2 with `_BpopT` = 2 and an egg T = 7 | the egg hatches on the first tick with `_Cycle` a multiple of 32 (tick 96) and becomes `_MakeMe` × 8 + 2 (soldier 0x32 with `_MakeMe` = 6, as in the save), M = `_ModeTabSB[1][Sk(8)]` = 1 or 2 | D (type), E (mode) |
| C-03 | red period (3.2) | QUICK2 with `_RpopT` = 1 | red eggs advance at tick 96; with `_RpopT` = 2 only at 128 | D |
| C-04 | death at hatching (3.1) | QUICK2 with `_ModeAuto` = 0, `_modeLevels[2]` = 0, 20 eggs T = 7 (`_BpopT` ≥ 3) | at tick 128 ≈ 99 % die (`_TotalEggsDiedB` +≈ 20); with `_modeLevels[2]` = 65535 none die | E / D |
| C-05 | red caste (3.2) | QUICK2 with `_HealthR` = 20 (which makes `_GetStrategy` give `_StrategicModeR` = 5 every tick; editing only `_StrategicModeR` does not help, it is recalculated) and 40 red eggs T = 0x87 | at tick 128 ≈ 35 workers (0x90–0x97) and ≈ 5 soldiers (0xB0–0xB7) are born, no male or new queen | E |
| C-06 | laying and cost (2) | QUICK2 with `_HealthB` = 100 and `_EatCountB` = 0 | every 16 ticks, ≈ 79 % chance of a new egg at (36, 9); `_HealthB` drops 1 per egg (plus 1 every 64 ticks); `_TotalEggsLaidB` goes up 1 per egg | E |
| C-07 | no laying with health 0 (2) | QUICK2 with `_HealthB` = 0 | no egg is laid (`Sk(128)` ≤ 0 only if it gives 0: chance 1/128 per opportunity); the head starves to death on average in 64 ticks with dialog 10015 | E |
| C-08 | queen walking (5.3) | QUICK2 with `_TilesDugB` = 1000, `_TileTotXB` = 34000, `_TileTotYB` = 20000, `_TileMassXB/YB` = (34, 20) (large totals so that the diggings made by the eggs barely change the target) and a clear tunnel (values 0–7) from (34, 10) to (34, 20) | the head goes down column 34, one cell per attempt (1/64 per tick), always followed by the tail; never goes up; `_LastBlackEgg` follows along | D (path), E (pace) |
| C-09 | queen walks backward (5.3) | reproduction of the real QUICK2 case from the new world | see the verification in 5.3: head and tail swap places when the only neighbor that gets closer is the tail | D |
| C-10 | orphan tail (2, 5.2) | QUICK2 with the black head removed (T = 0) | on the first tick the tail disappears and `_BlkQueens` = 0; at the next `_CountAnts`, defeat in the Quick Game | D |
| C-11 | black nurse keeps the stage (4) | worker T 0x12, M 1, S 0, on top of an egg T = 5 (egg at the lower index, so that it stays in `_LifeB`) | she becomes T 0x0A with S = 5; when dropping, the new egg has T = 5 | D |
| C-12 | red nurse restarts (4) | red worker T 0x92, M 1, on top of egg T = 0x86 | when dropping, the egg has T = 0x82 | D |

### Observed in the original

QEMU bench, `tools/caso_ant.py rodar tests/referencias/casos/ID.json` (versioned cases; injection, load and pacing in [../../bancada-qemu.md](../../bancada-qemu.md#casos-de-teste-injetar-um-save-e-rodar)). Final states in `tests/referencias/saves/`.

| Id | Date | Effective input | Observed | Result |
|---|---|---|---|---|
| C-01 | 2026-10-16 | `QUICK2.ANT` unchanged; stopped at tick 128 | black eggs that existed (B[2..6]) with T = 2; B[7], laid on tick 128 itself at (36, 9), with T = 1; red R[2..8] with 0x82 and R[9] (laid at 128) with 0x81 (`C01_T128.ANT`) | **confirmed**: simultaneous advance at tick 128 with P = 128; the egg laid on the tick does not advance on it |
| C-02 | 2026-10-16 | `QUICK2.ANT` without the black surface worker (A[0], T = 17), so that the `_BpopT` recounted on load is 2; egg B[2] with T = 7; stopped at tick 96 | `_BpopT` = 2; B[2] at (34, 9) with T = 0x32, M = 2, S = 0; B[3..5] with T = 2 (including the one laid at tick 80); B[6] (laid at tick 96) with T = 1; red eggs with 0x81 (`C02_T96.ANT`) | **confirmed**: P = 32 with `_BpopT` < 3, hatching at tick 96 as `_MakeMe` × 8 + 2 (soldier facing right), soldier mode in {1, 2}; red keeps P = 128 with `_RpopT` = 2 |
| C-11 | 2026-10-17 | `QUICK2.ANT`; egg B[3] moved to (32, 3) with T = 5 and worker T 0x12, M 1, S 0 at the end of the list in the same cell; stopped at tick 85 | B[3].T = 0; B[5] at (34, 9) with T = 0x0B, S = 5, M = 1: picked up the egg at tick 79 (S = stage) and went down (31, 4), (30, 5), (31, 6), (32, 7), (33, 8), (34, 9) along the exit map (`C11_T85.ANT`) | **confirmed** (§4: the black nurse keeps the stage in S; path from the nest spec §5.1) |
| C-10 | 2026-10-17 | `QUICK2.ANT` with the black queen's head (B[0]) zeroed | right after the load the message "The black queen in this patch has died." (10010) appears, modal, and the simulation does not advance (`GameTime` stays at 78); the state read still has `_CastePopB[5]` = 1 and `_IsGameOver` = 0 | **not checkable** with `caso_ant.py` (requires clicking the message). Shows that the load already detects the missing queen [open: which load routine shows the message] |

Notes from the runs:

- The load **recounts** `_CastePopB/R` and, with them, `_BpopT/_RpopT` (in `QUICK2.ANT`, `_CastePopB[0]` 2 → 3: the egg laid after the last count). That is why the C-02 input changes list A instead of writing `_BpopT`. The same applies to C-03 (`_RpopT` = 1 requires removing the red worker from list A).
- In a C-02 run that stopped at tick 98, the soldier was already at (32, 7) with T = 0x37 and M = 2: on ticks 97 and 98 it climbed two cells diagonally towards the exit (direction 7).

## Gaps

- Tested on the test bench: C-01, C-02 and C-11 (above); C-10 runs into the modal message on load. In the port, C-11 is checked in `engine/tests/regras_ninho.rs`, and the tick vectors 79–83 (with the laying at tick 80) match the port's egg, queen and tail (see [ninho.md](ninho.md#133-observed-in-the-original)). C-04 to C-07 became tick vectors (2026-10-18, `tools/cenarios_regras.py`, snapshot `rico`, 307/307 with S, R and life; [testes-tick.md](testes-tick.md#fifth-round-2026-10-18-unverified-rules-from-the-review)): `ovos_c04_c05/1791` (with `_ModeAuto` = 0 and `_modeLevels[2]` = 0 the 13 black eggs T = 7 die at hatching, `_TotalEggsDiedB` 0 → 13; with `_HealthR` = 20 the 21 red eggs 0x87 hatch into 17 workers and 4 soldiers, no male or queen), `ovos_c06/1711` (laying with `_HealthB` 100: new egg, `_TotalEggsLaidB` +1, `_HealthB` −1) and `ovos_c07/1711` (`_HealthB` 0: no egg) — **confirmed**. C-08 onward are still missing.
- The exact meaning of list 4/1100 per stage (the index used in the Examine window) has not been checked in the interface.
- `_YellowBirth` and the yellow-queen's laying: only summarized (they belong to `formiga-amarela.md`).
- The founding of new colonies in the Full Game (`_SimColonies`, `_QueenStorage*`, mating flight) is not here.
- The effects marked [hypothesis] (red yellow ant laying a black-type egg; tail recreation in fights) need testing.
