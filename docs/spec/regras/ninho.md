*Leia em português: [ninho.md](../../pt-BR/spec/regras/ninho.md)*

# Nest rules (black and red)

This document specifies, for a 1:1 reimplementation, what the ants do **inside** the nests: the `_DoAntSimB`/`_DoAntSimR` loop, dispatch by mode (M) and group (T), movement, fighting, stored food, digging, the exit map and water. Eggs, queen and castes are in [ciclo-de-vida.md](ciclo-de-vida.md). Lists, T encoding, mode and fight tables are in [formigas.md](formigas.md); the tick, in [simulacao-visao-geral.md](simulacao-visao-geral.md); the generators, in [rng.md](rng.md); map values, in [tiles-valores.md](tiles-valores.md).

## Conventions

- **Evidence:** same as in [simulacao-visao-geral.md](simulacao-visao-geral.md#evidence-convention). Unless noted, everything here is **fact seen in the code** (Ghidra pseudocode in `re_trabalho/decomp/Code5.c`/`Code6.c`/`Code7.c`), with ambiguous passages checked against the disassembly of `SIMANTW.EXE` (capstone, 16-bit mode). The pseudocode below is our own, not a copy of the decompiled output.
- **Addresses:** functions as `selector:offset` with the `.SYM` name; variables as `segment:offset` from `symbols.tsv` (8 = selector `1038`, 9 = `1040`, 10 = DGROUP `1048`).
- **Random draws** (S generator, LFSR, unless noted): `S(n)` = `_SRand1(n)`; `Sk(K)` = `_SRandK()` (mask, one step); `SG(n)` = `_SGRand(n)` (two steps). `RR(n)` = `_RRand(n)` (R generator, C `rand`). **The order of the calls is part of the rule**: each pseudocode lists the random draws in the order they happen, and a draw only happens if the branch it is in is executed.
- **Notation:** `D8[d]` = (`_Dx8[d]`, `_Dy8[d]`) (`8:0000`/`8:0008`): 0 = up (y−1), 2 = right, 4 = down, 6 = left. `inv(d)` = `d XOR 4` (opposite direction). For the ant being processed, with index `i = _Tindex` (`9:9b6a`): X, Y, M, T, S are the fields of the nest list (`_Blist*` `8:3736…8:3f0e`, `_Rlist*` `8:4104…8:48dc`), `pos` = (X, Y), `dir` = T & 7, `group` = (T & 0x78) >> 3. `Map` and `Life` are `_MapB`/`_LifeB` in the black nest and `_MapR`/`_LifeR` in the red one; `Exit` is `_ExitMapB` (`8:03a4`) or `_ExitMapR` (`8:13a4`). All matrices are indexed `x × 64 + y`, with y = 0 on the surface row.
- "Nest B" and "routine B" refer to the black nest; the R version is the same except for what section 11 lists.

## 1. State used by the nest

| Variable | Address | In save | Role |
|---|---|---|---|
| `_MapB` / `_MapR` | `10:48e8` / `10:58e8` | yes | nest terrain (values in [tiles-valores.md](tiles-valores.md#nests-_mapb-_mapr)) |
| `_LifeB` / `_LifeR` | `10:88e8` / `10:98e8` | no | who occupies each cell (byte T); rebuilt on load (section 13.1) |
| `_ExitMapB` / `_ExitMapR` | `8:03a4` / `8:13a4` | yes | "exit map": gradient that grows toward the surface (section 7.4) |
| `_HoleMapB` / `_HoleMapR` | `8:82d2` / `8:8312` | yes | for each nest column x, the column of the hole on the surface (0 = none) |
| `_LastNewHoleB/R`, `_LastNewExitB/R` | `8:835a`/`835e`, `8:8352`/`8356` | yes | last hole created: (x, y) on the surface and (column, 0) in the nest |
| `_TileTotXB/YB`, `_TilesDugB`, `_TileMassXB/YB` | `9:8104`, `9:811a`, `9:72c8`, `9:7c48`, `9:7c90` | yes | sums of x and y of the digs, count and center of mass (the queen's target) |
| same for R | `9:9ddc`, `9:9de2`, `9:7a56`, `9:9fba`, `9:9fd2` | yes | same |
| `_FoodB` / `_FoodR` | `9:9ea4` / `9:72de` | yes | stored food units (a counter, not recomputed from the map) |
| `_EatCountB` / `_EatCountR` | `9:7402` / `9:7c8e` | yes | feeding "reserve": eating adds 5, each egg laid subtracts 1 |
| `_HealthB` / `_HealthR` | `10:ac86` / `10:ac88` | yes | colony health, 0..100 |
| `_WaterTable` | `9:99d8` | yes | highest y row already flooded; 64 = nothing flooded |
| `_TemBModePop` / `_TemRModePop` | `9:786a` / `9:7be4` | no | mode count for the tick (20 words each) |
| `_MeColor` | `10:ce98` | yes (block 188) | color of the yellow ant: 0 black, ≠ 0 red (Ghidra pseudocode shows `_grndTile+2`) |
| `_MeWantFood` | `9:9af2` | yes | the yellow ant wants to be fed (trophallaxis) |
| `_OptionStates[5]` | `8:85fc` | yes | speech balloons; drawing only, no random draw |

## 2. The loop: `_DoAntSimB` (`1028:2d4e`) and `_DoAntSimR` (`1028:5344`)

```
n = _ListIndexB                       (read once, at the start)
for i = n−1 down to 0 (decreasing):   (_Tindex = i)
    if T[i] ≠ 0: _DoNestAntB(X[i], Y[i], T[i])
```

- Entries added during the loop (eggs laid, ants coming from the red nest through passage 0x14) are at indices ≥ n and are only processed in the next tick.
- Slots with T = 0 are skipped **without a random draw** (unlike `_DoAntSimA`, which draws for every slot).
- `_DoAntSimB` runs after `_DoAntSimA` and before `_DoAntSimR` (tick order in [simulacao-visao-geral.md](simulacao-visao-geral.md#5-one-tick-_doantsim-10280000)).

## 3. Dispatch: `_DoNestAntB` (`1028:2dae`) and `_DoNestAntR` (`1028:53a4`)

There are two branches, by the ant's color relative to the nest: **owner** (black in B, red in R) and **invader** (red in B, black in R).

### 3.1 Owner ant

```
_TemBModePop[M] += 1                                   (R: _TemRModePop)
a = Sk(256)                                             ← always
if a == 0 and M ≠ 9:
    b = Sk(32)
    if _HealthB < b:                                    (R: _HealthR)
        T = 0; Life[pos] = 0; _BAntsExpired += 1       (R: _RAntsExpired); END
choose by M (table 3.3)
```

Natural death: only when `Sk(256) = 0`, and then with chance (31 − health)/32 (b goes from 0 to 31 and must be greater than health). Health ≥ 31 prevents death ([formigas.md](formigas.md#6-fighting-death-and-health) says "≥ 32"; with the strict comparison, 31 is already enough). Eggs (M = 8) and fight clouds of the nest's color (M = 10) can also die this way; only the queen (M = 9) escapes.

### 3.2 Invader ant

Nest B, red ant (`_TemRModePop[M] += 1`, no death draw):

```
if T ≥ 0xF0 (red fight cloud, groups 14–15): _DoNestFightB; END
v = Life[pos]
if v is the yellow ant (0xFF/0xFE) and _MeColor == 0: _YellowFight(2, i); END
if 1 ≤ v ≤ 0x67 and j = _FindInBList(pos, v) ≥ 0:     (a black ant, or black egg, in the SAME cell)
    if v ≥ 0x60 (queen's head): T[j] = 0; Life[X[j], Y[j]] = 0
    if v > 7 (not an egg):                              (fight; the invader disappears)
        w = _GetWinner(T[j], T)                         ← T[j] is already 0 if it was the queen
        T = 0; S[j] = w; T[j] = 0x70 + (w & 0x80); Life[pos] = T[j]; M[j] = 10; END
    else (egg theft):
        M = 3; T = T OR 8; Life[pos] = T; T[j] = 0; _TotalEggsDiedB += 1; END
if M == 7: _RaidInB (section 9.1); END
LEAVE_NEST   (section 9.2)
```

Nest R, black ant (`_TemBModePop[M] += 1`, no death draw):

```
if T < 8 (black egg in the red nest): T = 0; Life[pos] = 0; END        ← the egg disappears without being counted
if T ≥ 0x70 (black fight cloud, groups 14–15): _DoNestFightR; END
v = Life[pos]
if 0x81 ≤ v ≤ 0xE7 and j = _FindInRList(pos, v) ≥ 0:
    if v ≥ 0xE0 (red queen's head): T[j] = 0; Life[X[j], Y[j]] = 0
    if v > 0x87: fight as above (the invader disappears); END
    else: theft of the red egg: M = 3; T = T OR 8; Life[pos] = T; T[j] = 0; END   (no counter)
if M == 6:
    if _MePlane == 3: _StayInR (section 9.3); END
    else LEAVE_NEST
if M == 7: _RaidInR; END
LEAVE_NEST
```

Differences between the two invader branches: in R there is no test for the yellow ant in the cell itself, there is the mode 6 "stay" (`_StayInR`), and black eggs carried into the red nest are destroyed; in B there is no counterpart for red eggs (a red egg in list B would be treated as an ordinary invader).

**Caution — queen attacked in her own cell.** When an invader is in the cell of the queen's head, the queen is zeroed **before** `_GetWinner`, which receives 0 as the occupant's type (class 0, worker). The resulting cloud stores S = 0 or the invader's T; if it "beats" the occupant, the cloud becomes T = 0 when it ends (section 4.4). In any case the queen's head ceases to exist, and the tail is removed by the `_SimQueenB` test ([ciclo-de-vida.md](ciclo-de-vida.md#5-queen)). [hypothesis: defect of the original]

### 3.3 Owner ant dispatch table

| M | Name (4/1102) | Nest B | Nest R |
|---|---|---|---|
| 0, 18, 19, ≥ 20 | Bored, ???, Avenger | WANDER (4.1) | WANDER (R version) |
| 1 | Nesting | `_DoNestingB` (5) | `_DoNestingR` (5.4) |
| 2, 5, 7, 11, 12, 15, 16 | Foraging, Dig Out, Attack, Alarmed, Defend Nest, Take off, Flying | `_DoDigOutB` (7.2) | `_DoDigOutR` |
| 3 | Got Food | `_DoFoodInB` (6.2) | `_DoFoodInR` |
| 4 | Dig In | `_DoDigInB` (7.1) | `_DoDigInR` (7.1 + passage) |
| 6 | Defense | if `_MePlane` ≠ 2: `_DoDigOutB`; else WANDER WITHOUT SWITCH (4.2) | `_DoDigOutR` (always leaves) |
| 8 | Hatching | `_SimEggB` ([ciclo-de-vida.md](ciclo-de-vida.md)) | `_SimEggR` |
| 9 | Egg Laying | `_SimQueenB` ([ciclo-de-vida.md](ciclo-de-vida.md)) | `_SimQueenR` |
| 10 | Fighting | `_DoNestFightB` (4.4) | `_DoNestFightR` |
| 13 | Resting | REST (10) | REST (R version) |
| 14 | Pre-Flight | PRE-FLIGHT (10) | PRE-FLIGHT with `_FlyAwayR` |
| 17 | ??? | DROWNING (8.3) | DROWNING |

The functions `_DoRandB` (`1028:3876`), `_DoRecruitN` (`1028:396c`), `_DoRestB` (`1028:367e`), `_DoDrownB` (`1028:37a4`), `_RaidOutB` (`1028:3610`) and their R versions exist, but **have no caller**: the same code is inlined in the `switch` of `_DoNestAntB/R` (section 12).

## 4. Movement and fight primitives

### 4.1 Walking: `_TryMoveDirB` (`1028:439e`) / `_TryMoveDirR` (`1028:6850`) and WANDER

`MOVE(d)`, returns success:

```
if d < 0: fail
n = pos + D8[d]
if n.x ∉ 0..63 or n.y > 63: fail
if n.y < 1: return _GetOutB(X)           (the ant is at y = 1 and tries to go up; section 4.6)
if Map[n] ≥ 0x1C: fail                   (passable: 0x00–0x1B: tunnel, food 0x10–0x13, 0x14, opening 0x18)
[B only] if Life[n] == 0xFF and _MeWantFood ≠ 0 and X < 0x80:
        Life[pos] = (T & 0xF8) | d; _DoTroph(X, Y, d)      (feeds the yellow ant)
Life[n] = (T & 0xF8) | d; Life[pos] = 0; pos = n; T = Life[n]; success
```

- **There is no occupancy test.** The ant enters an occupied cell and overwrites `Life[n]`; the other ant stays in the list, at the same position. The life map is "last writer wins". This is how fights and thefts start: the ant left underneath sees, on its next processing, another T in its own cell (sections 3.2 and 4.3).
- The `X < 0x80` test (disassembly: `cmp byte [BlistX+i], 0x80`) is always true; the original probably meant to test the color in T. Consequence: any ant in nest B, including a red invader, feeds the yellow ant. **Confirmed in the original** (vector `ninho_n15/1711`, 2026-10-18: a red invader entering the cell of the hungry yellow ant gives `_MeHealth` = 100). [hypothesis: programming defect, but it is the game's behavior]
- Only 0xFF (the yellow ant's head) triggers trophallaxis, not 0xFE.

`TRY(d)` = `if not MOVE(d): MOVE(Sk(8))` — one random fallback attempt; if that also fails, the ant stays put.

**WANDER** (M = 0, 18, 19 and others; `_DoRandB`):

```
if Sk(32) == 0: M = _GetNewModeB(group)          (R: _GetNewModeR)
if FIGHT_OWN_CELL(): END                         (4.3)
TRY(dir)
```

### 4.2 WANDER WITHOUT SWITCH (M = 6 with the yellow ant in the black nest; `_DoRecruitN`)

Same as wander, without the `Sk(32)`: `if FIGHT_OWN_CELL(): END; TRY(dir)`. Defense soldiers keep walking around in the nest while the yellow ant is there; they do not follow it.

### 4.3 Fight check: `_CheckNestFightB` (`1028:3ba2`) / `_CheckNestFightR` (`1028:61a2`)

`FIGHT(c, Tref)`, returns true if there was a fight:

```
v = Life[c]
B: if v is yellow and _MeColor ≠ 0: _YellowFight(2, i); true
   if 0x88 ≤ v ≤ 0xE7 and j = _FindInBList(c, v) ≥ 0: CLOUD(j, v, Tref); true
R: if 0x08 ≤ v ≤ 0x67: if j = _FindInRList(c, v) ≥ 0: CLOUD(j, v, Tref); true
   else if v is yellow and _MeColor == 0: _YellowFight(3, i); true
false

CLOUD(j, v, Tref):  w = _GetWinner(v, Tref); S[j] = w; T[j] = 0x70 + (w & 0x80); Life[c] = T[j]; M[j] = 10
```

- `FIGHT_OWN_CELL()` = `FIGHT(pos, T)`: someone of the other color entered my cell.
- **The ant doing the check does not disappear.** The one that becomes a cloud is the enemy found (`j`); ant `i` stays in the list, at the same position. If `_GetWinner` returns `Tref` (the type of `i`), the cloud ends as **a second ant** identical to `i` (section 4.4). This differs from the surface, where the moving ant disappears ([formigas.md](formigas.md#6-fighting-death-and-health)). Fact seen in the code and checked in the disassembly (`1028:2eea`: no access to `T[i]`). [hypothesis: defect of the original]
- `_GetWinner(occupant, attacker)` (`1028:26f4`) uses **`RR(10)`** (R generator), except with `_Invincible = 1` (no draw; the black ant wins). The matrix is in [formigas.md](formigas.md#6-fighting-death-and-health). `_YellowFight` also calls `_GetWinner` (documented with the yellow ant).
- Eggs of the other color (0x81–0x87 in B; 0x01–0x07 in R) do **not** trigger a fight: the ant walks over them.

### 4.4 Fight cloud: `_DoNestFightB` (`1028:3a54`) / `_DoNestFightR` (`1028:6072`)

```
r = S(7); T = (T & 0xF8) + r; Life[pos] = T          (animation: direction 0..6)
if Sk(16) ≠ 0: balloon if _OptionStates[5] (B: == 1; R: ≠ 0); END
T = S; Life[pos] = T                                  (the fight is over: becomes the winner)
if (T & 0x78) == 0x60 (queen's head, of either color):
    adds the tail to the nest list: position pos + D8[inv(T & 7)], type T + 8, M = 9, S = 0
B: if T is red:   M = 7; else M = _GetNewMode(group(T), T)
R: if T is black: M = 7; else M = VitóriaR[group(T)]
```

`VitóriaR` = DGROUP `0x22E6`, 16 bytes read from `SIMANTW.EXE` (no name in the `.SYM`):

| group | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| M | 1 | 1 | 1 | 3 | 0 | 5 | 2 | 3 | 0 | 5 | 0 | 0 | 9 | 9 | 10 | 0 |

- Duration: each tick the cloud ends with chance 1/16.
- A cloud of the nest's color first goes through the natural death draw (3.1) and can disappear with no winner.
- A red ant that wins in the black nest switches to M = 7 and, as an invader, enters `_RaidInB` (looks for food). A black ant that wins in the red nest, likewise.
- The recreated tail does not touch `_BlkQueens`/`_RedQueens`.

### 4.5 Directions from the exit map

**`_GetExitDirB(x, y, d)`** (`1020:119c`; R `1020:1240`), returns direction + 1, or 0:

```
if y == 1:
    if Map[x][0] == 0x18: return 1                      (goes up: direction 0)
    return Sk(2) × 4 + 3                                (3 or 7: direction 2 or 6, walks sideways)
best = 0; res = 0
for k = 0..7, if k ≠ inv(d):                            (with d = 8 none is excluded)
    c = (x, y) + D8[k]; if c inside the map (0..63, 0..63) and Exit[c] > best: best = Exit[c]; res = k + 1
return res                                              (tie: the lowest direction wins)
```

**`_GetEnterDirB(x, y, d)`** (`1020:12e4`; R `1020:137c`), returns direction or −1:

```
current = Exit[x][y]; res = −1
for k = 0..7, if k ≠ inv(d):
    c = (x, y) + D8[k]; if c inside the map and v = Exit[c] ≠ 0 and v ≤ current:
        if v == current and Sk(2) == 0: skip k            (random draw ONLY on a tie)
        res = k; current = v
return res
```

Reading: `Exit` grows toward the surface (0xFF on row 0), so "leaving" is climbing the gradient and "entering" is descending to the smallest non-zero value. Dirt cells have `Exit` = 0 and are never chosen.

**`_RandTurn(d)`** (`1028:2a22`): `_TurnTab[d][Sk(8)]` (table in [formigas.md](formigas.md#7-movement-and-direction-choice)).

### 4.6 Leaving the nest: `_GetOutB` (`1028:520a`) / `_GetOutR` (`1028:74ba`)

Called with the column x of an ant at y = 1 that tried to go to y ≤ 0.

```
if Map[x][0] == 0x18:                                    (there is an opening)
    t = T; T = 0
    if _HoleMapB[x] == 0: _MakeNewHoleB(x)               (random draws: section 7.5)
    r = Sk(8)
    ok = _ExitHole(_HoleMapB[x], x, (t & 0xF8) + r, M, S)
    if ok: Life[x][1] = 0; return 1                      (the ant went to list A)
    else: T = t; M = 0; return 0
else:
    if Exit[x][0] ≠ 0: Exit[x][0] −= 1
    if Sk(2) == 0: if x ≤ 62 and dirt(Map[x+1][1]): _DigTileThemB(x+1, 1)
    else:          if x ≥ 1  and dirt(Map[x−1][1]): _DigTileThemB(x−1, 1)
    MOVE(Sk(8)) starting from (x, 1)                      (may call _GetOutB again)
    return 0
```

- `_ExitHole` (`1020:2db6`) puts the ant at the end of list A, on the first neighbor 0..7 of the hole (`_HoleMapB[x]`, x) inside the yard with `_MapA` < 0x50; the direction is the drawn `r` and M is kept. **S is only kept with M = 6**; with the other modes S = 0, or 120 if M ∉ {3, 7} and the hole is on the color's side (black: x ≤ 63; red: x ≥ 65) ([formigas.md](formigas.md#7-movement-and-direction-choice)). `_ExitHole` **does not write `_LifeA`** (the surface cell is only marked when the ant walks there). Fails if no neighbor fits. (Revised against the pseudocode on 2026-10-17; the previous version of this spec said "M and S are kept".)
- **List A full** (`_ListIndexA` = 1000) [seen in the code, `1020:2db6`; vectors `lista_a_cheia` and `lista_a_cheia_sem_vagas` 307/307, 2026-10-19]: the ant is written to slot `_ListIndexA` **before** the capacity test, that is, to slot 1000. Each list A array is 1002 bytes in memory (`_AlistX` at `8:23a4`, `_AlistY` at `8:278e`: 0x3EA = 1002 apart), so slot 1000 is slack inside the array, outside the save (which stores 1000) and outside every scan (`0.._ListIndexA`). Then the list is compacted as in `_CompactListA` (only slots 0..999; the end is not cleared) and, if compaction opened slots, the counter still adds 1. Result: **the ant that left is lost** and, if there were gaps, the list now includes the slot after the compacted ones with whatever was in it — empty (T = 0) if it was a gap, or **a copy of an ant** that was slid back (the same ant twice in the list). `_ExitHole` returns 1 (the caller removes the ant from the nest). **Port** (2026-10-19): `comum::sair_pelo_buraco` does the same without writing slot 1000 (the slack does not exist in the port and nothing reads it); test `saida_com_a_lista_a_cheia_segue_o_original`. Previously the port compacted and wrote the ant to the first free slot.
- If `_MakeNewHoleB` finds no place, `_HoleMapB[x]` stays 0 and `_ExitHole` tries around (0, x). [resulting behavior not tested]
- In the Ghidra pseudocode the M and S arguments mistakenly appear in the `_SRand8` call; the disassembly shows they belong to `_ExitHole`.
- `_LeaveNestB` (`1028:515e`) is the first half of this routine, with no caller, and zeroes `Life[x][y]` instead of `Life[x][1]`.

### 4.7 Entering the nest: `_GoInNest` (`1028:257a`)

From the surface, an ant on the hole (x, y) goes to the black nest if x < 64, otherwise to the red one: if the list is full (≥ 500), it compacts; if it is still full, nothing happens. It enters at position (y, 1) with T = (T & 0xF8) + 4 (facing down), same M and S. If `_HoleMapB[y]` ≠ 0, it calls `_DigTileB(y, 1)` (digs if it is dirt, smooths, rebuilds `Exit`). It leaves list A (T = 0, `_LifeA` = 0).

## 5. Tending the brood (M = 1): `_DoNestingB` (`1028:44a8`)

Field S stores the carried egg: bits 0–2 = egg stage, bit 3 = "already found where to drop it" (in the carrying worker) or "just dropped it" (in the free worker).

### 5.1 Group 1 (worker carrying an egg, T 0x08–0x0F)

```
if S >> 3 == 0:                                  (looking for the bottom)
    d = _GetEnterDirB(pos, dir)                   (Sk(2) only on a tie)
    if d < 0: S = (S & 7) | 8                     (reached a minimum of the exit map)
    TRY(d)                                        (d < 0 fails without a draw and falls to Sk(8))
else:
    v = Life[pos]
    if v == 0 or v > 8:                           (drops the egg)
        T = T + 8                                 (back to group 2)
        _PlaceEggB(X, Y, S & 7)                   (section 7.6; egg with the SAME stage)
        Life[pos] = T; S = 8; M = _GetNewModeB(1) (= 1, no random draw)
    e = _GetExitDirB(pos, dir)
    TRY(e − 1 if e ≠ 0, else Sk(8))
```

Detail: a carrier with T = 0x08 (direction 0) has `Life[pos]` = 8 and **cannot drop** while it is facing up. [hypothesis: defect; small effect]

### 5.2 Group 2 (free worker)

```
if S >> 3 == 0:
    v = Life[pos]
    if v == 0 or v > 7:
        r = S(100)
        if _HealthB < r: EAT(pos)                           (section 6.1)
        else if Sk(16) == 0: M = _GetNewModeB(2)
    else (there is an egg in my cell):
        j = _FindInBList(pos, v)
        if j ≥ 0: T[j] = 0; T = T − 8; S = v; END           (picks up the egg; does not walk this tick)
    TRY(dir)
else:                                                        (just dropped)
    if Sk(8) == 0: S = 0
    e = _GetExitDirB(pos, dir); TRY(e − 1 if e ≠ 0, else Sk(8))
```

The egg "appears" in `Life[pos]` because the worker walked over it and, in the same tick or the next, `_SimEggB` rewrote the egg's T into the cell (section 4.1).

### 5.3 Other groups with M = 1 (soldier, male…)

```
r = S(100); if _HealthB < r: EAT(pos)
if Sk(8) == 0: M = _GetNewModeB(group)
TRY(dir)
```

### 5.4 Red version: `_DoNestingR` (`1028:690a`) — a different rule

```
group 1:
    if Sk(4) == 0 and Map[pos] < 0x10:
        T = T + 8; Life[pos] = T; _PlaceEggR(X, Y, 0x82); S = 0; M = _GetNewModeR(1) (= 1); END   (does not walk)
    if Sk(4) ≠ 0 and d = _GetEnterDirR(pos, dir) ≥ 0: TRY(d)
    else TRY(Sk(8))
group 2:
    if Sk(4) == 0:
        v = Life[pos]
        if v == 0 or (v & 0x7F) > 7:
            r = S(100); if _HealthR < r: EAT_R(pos) else M = _GetNewModeR(2)
        else if j = _FindInRList(pos, v) ≥ 0: T[j] = 0; T = T − 8; END
    if Sk(4) ≠ 0: TRY(dir) else TRY(Sk(8))
others: M = _GetNewModeR(group); TRY(dir)
```

Differences in effect: the red ant drops the egg at random (1/4 per tick on a cell without food), it does not carry it to the bottom; **the egg's stage is lost** (S stores nothing and the dropped egg goes back to stage 2, T = 0x82); the red ant also picks up black eggs (`v & 0x7F`); and, when not hungry, it switches mode right away (without the `Sk(16)`).

## 6. Stored food

### 6.1 Eating: `_TryEatFoodB` (`1028:47c6`), `_EatFoodB` (`1028:4844`)

`EAT(c)`, inlined in several routines:

```
v = Map[c]
if 0x10 ≤ v ≤ 0x13:
    Map[c] = (v == 0x10) ? Sk(8) : v − 1           (0x10 = 1 unit; when it runs out it becomes tunnel)
    if _FoodB > 0: _FoodB −= 1
    _EatCountB += 5
    if _EatCountB > (_BpopT + _CastePopB[2]) >> 4:  (soldiers count twice)
        _EatCountB = 0; if _HealthB < 100: _HealthB += 1
```

`_EatFoodB` is the same thing without the range test. R: `_FoodR`, `_EatCountR`, `_RpopT + _CastePopR[2]`, `_HealthR`.

Who eats and when (the random draw always happens when the line is reached; `EAT` only acts if there is food in the cell):

| Where | Condition | Cell |
|---|---|---|
| Nesting, group 2 and others | `S(100)` > health | its own |
| Got Food, when dropping | `S(100)` > health | its own (the one just dropped) |
| Dig In, Dig Out, after walking | `Sk(64)` > health | the new one (black); the old one (red) |

With health ≥ 63, Dig In/Dig Out never eat; with health 100, nobody eats.

`_DecEatB` (`1028:48f8`), called for an egg laid by the yellow ant (the queen uses the same code inlined): `_EatCountB −= 1`; if it goes < 0: `_EatCountB = _BpopT >> 5` and, if `_HealthB` > 0 and `_AlwaysHealthy` = 0, `_HealthB −= 1`. `_DecEatR` does not test `_AlwaysHealthy`.

### 6.2 Carrying food: `_DoFoodInB` (`1028:492a`) (M = 3)

```
d = _GetEnterDirB(pos, dir)                        (Sk(2) only on a tie)
if d < 0 or Sk(16) == 0:                           (Sk(16) only if d ≥ 0)
    DROP_FOOD(pos)                                  (6.3)
    if T & 8: T = T − 8                             (group 3→2, 7→6)
    r = S(100); if _HealthB < r: EAT_NO_TEST(pos)   (_EatFoodB inlined: without the 0x10–0x13 test)
    M = _GetNewModeB(group of the INCOMING T)
else:
    T = (T & 0xF8) | d; Life[pos] = T
    n = pos + D8[d]; outside the map: END; n.y < 1: _GetOutB(X); END
    if Map[n] ≥ 0x30: END                           (stays put)
    STEP(n, d)                                       (6.4)
```

`EAT_NO_TEST` does not check the range: if the cell was 0x13 (full) or 0x14 (passage), DROP does not change it and the bite makes it drop to 0x12 or 0x13 (the passage becomes food). Seen in the pseudocode (`1028:492a`), same in `_DoFoodInR`. [hypothesis: defect]

**Doubled food.** The new mode is computed with the group **before** removing the food (3 or 7), and `_CasteModeTabB[3] = _CasteModeTabB[7] = 3`. The ant stays in M = 3 with no load; on the next drop (bottom of the tunnel or 1/16 per tick) it credits one more unit to the map and to `_FoodB`, and only then switches mode (now with group 2 or 6). Each load brought in yields **two** units. Checked in the disassembly (`1028:4bab`: the argument is the received T, `[bp+0xa]`). [hypothesis: defect of the original; confirm on the test bench, case N-06]

### 6.3 Dropping: `_DropFoodB` (`1028:3c3c`)

`DROP_FOOD(c)`: if `Map[c]` < 0x10 (tunnel), it becomes 0x10; else, if < 0x13, adds 1; else (0x13 full, or 0x14 passage) the map does not change. In all cases `_FoodB += 1`, so the counter can become larger than the food drawn. The separate function (with no caller) does the same, returns 0 when the map did not change and clears bit 8 of T.

### 6.4 Step with fight check (inlined in Dig In, Dig Out and Got Food)

`STEP(n, d)`: `Life[pos] = 0`; if `FIGHT(n, (T & 0xF8) | d)`: END (the ant stays, but with `Life[pos]` = 0); else `T = (T & 0xF8) | d`, `Life[n] = T`, `pos = n`. It does not test `Map[n]` < 0x1C: the caller already requires `Map[n]` < 0x30 and (in Dig In/Out) "is not dirt".

### 6.5 Theft and pickup

- `_StealFoodB` (`1028:48b4`): removes one unit (0x10 → `Sk(8)`) and `_FoodB −= 1` if > 0, without touching health. Inlined in `_RaidInB`/`_StayInR`.
- `_PickupFoodB` (`1020:0f40`) / `_PickupFoodR` (`1020:0fa2`): same, only if 0x10–0x13. No caller in the pseudocode.

## 7. Digging and exit map

### 7.1 Digging in: `_DoDigInB` (`1028:4bd0`) (M = 4)

```
if group ∉ {2, 6}: M = _GetNewModeB(group); END
d = _GetEnterDirB(pos, dir); if d < 0: d = Sk(8)
T = (T & 0xF8) | d; Life[pos] = T
if Y == 63: M = _GetNewModeB(group); END
n = pos + D8[d]; outside the map: END; n.y < 1: _GetOutB(X); END
if Map[n] ≥ 0x30: END
if dirt(Map[n]) (0x20–0x2E):
    if not _DigTileThemB(n): M = _GetNewModeB(group); END       (R: M = 0)
    T = T + 0x18 (carries dirt: group 2→5, 6→9); M = 5; sound 0x11 (R: 0x12)
STEP(n, d); if there was a fight: END           (fight Tref = (incoming T & 0xF8) | d, WITHOUT the dirt's 0x18)
r = Sk(64); if _HealthB < r: EAT(n)            (R: EAT(OLD pos))
if Sk(4) == 0: _FixExitMapB(n)
[R only] if Map[n] == 0x14: PASSAGE (7.7)
```

The worker walks into the cell it has just dug, already carrying the dirt (M = 5, which in the nest means "go out": section 7.2).

### 7.2 Going out carrying / going out: `_DoDigOutB` (`1028:4eb0`) (M = 2, 5, 7, 11, 12, 15, 16)

```
e = _GetExitDirB(pos, dir)
d = (e ≥ 1) ? e − 1 : _RandTurn(dir)                     (Sk(8) in _RandTurn)
T = (T & 0xF8) | d; Life[pos] = T
n = pos + D8[d]; outside the map: END; n.y < 1: _GetOutB(X); END
if Map[n] ≥ 0x30 (flooded dirt, pebble…):
    B: Exit[pos] −= 1 (byte; 0 becomes 0xFF!)      R: only if Exit[pos] ≠ 0
    if group ∈ {5, 9}: T = T − 0x18; M = 4          (the dirt disappears; goes back to digging)
    if group ∈ {2, 6}: M = 4
    END
if dirt(Map[n]): END                                 (stopped, no change)
STEP(n, d); if there was a fight: END
r = Sk(64); if _HealthB < r: EAT(n)            (R: EAT(OLD pos))
```

- **In the red nest, Dig In and Dig Out eat in the cell the ant left**, not in the new one (pseudocode of `_DoDigInR` `1028:6f0c` and `_DoDigOutR` `1028:722a`: the index used is the one computed before updating the position). Seen in the code on 2026-10-17.
- Carried dirt (groups 5 and 9) is only unloaded on the surface (`_DoDigOutAntA`); in the nest, if the path is blocked, it simply disappears.
- In the black nest, the untested decrement of `Exit` (checked in the disassembly, `1028:4f83`) turns a cell with 0 into 0xFF, a false "exit point". [hypothesis: defect; the R version tests]

### 7.3 Digging functions

**`_DigTileB(x, y)`** (`1020:1fe4`), used to dig unconditionally (egg, entrance, generation):

```
if dirt(Map[x][y]):
    Map[x][y] = Sk(8)
    _TileTotXB += x; _TileTotYB += y; _TilesDugB += 1           (32-bit sums)
    _TileMassXB = _TileTotXB / _TilesDugB; _TileMassYB = _TileTotYB / _TilesDugB
    if y > 53 and S(64) == 0:                                    (passage between nests)
        Map[x][y] = 0x14
        if dirt(_MapR[x][y]): _MapR[x][y] = Sk(8); accumulates into the R totals
        SMOOTH_R(x, y); _FixExitMapR(x, y); _MapR[x][y] = 0x14
SMOOTH_B(x, y); _FixExitMapB(x, y)                               (always, even without digging)
```

`SMOOTH(x, y)` = `_SmoothEdges(x, y−1)`, `(x+1, y)`, `(x, y+1)`, `(x−1, y)`, in that order.

**`_DigTileR(x, y)`** (`1020:21de`): same, without the 0x14 passage (only the black nest creates passages).

**`_DigTileThemB(x, y)`** (`1020:22d4`), used by the worker: only digs if `0 < x < 63`, if the cell below is dirt (or y > 62) and the cell above is dirt (or y < 3); it does not require the cell itself to be dirt. At y = 0 it writes 0x18 and calls `_MakeNewHoleB(x)`; otherwise it writes `Sk(8)`. It always accumulates into the totals (even when re-digging), smooths and rebuilds `Exit`. Returns 1 if it dug. The "dirt above and below" rule makes tunnels one cell high.

**`_SmoothEdgesB/R`** (`1020:255a`/`26e4`): rule in [tiles-valores.md](tiles-valores.md#nests-_mapb-_mapr). Details that matter for 1:1:

- y > 63 or x outside 0..63: nothing. y = 0: if `Map[x][0]` ≤ 0x2F, it becomes 0x18 (opens the surface above any dig on row 1). y < 0 (called with y−1 = −1) is not filtered [only happens if someone digs y = 0 with `_DigTileB`].
- At y = 1 the upper neighbor always counts as solid.
- It only draws (`Sk(8)`) when the dirt cell is left with no solid neighbor and is not flooded.

### 7.4 Exit map: `_FixExitMapB` (`1020:284e`) / `_FixExitMapR` (`1020:2914`)

```
if y < 2: Exit[x][y] = (Map[x][y] == 0x18) ? 0xFF : 0xFE
else: m = largest Exit among the 8 neighbors inside the map; Exit[x][y] = (m == 0) ? 0 : m − 1
```

- Updates **one** cell, without propagating. Callers: every dig (`_DigTile*`, `_DigTileTheme*`), `_PlaceEggB` (via `_DigTileB`), and the worker in Dig In with chance 1/4 after each step. Row 0 starts entirely at 0xFF at generation.
- **Verified in `QUICK2.ANT`**: the black queen's chamber has `_ExitMapB` (34, 9) = 0xF6, (35, 9) = 0xF5, (36, 9) = 0xF4; the red one has (39, 11) = 0xF4, (40, 11) = 0xF3, but (41, 11) = 0. The difference is explained by the rule: (41, 11) was dug first, with no neighbor having a value, and never recomputed; (36, 9) received eggs (`_PlaceEggB` → `_DigTileB` → `_FixExitMapB`), which recomputed it (largest neighbor 0xF5 − 1).

### 7.5 Holes on the surface

- `_MakeNewHoleB(c)` (`1020:1b06`) and `_MakeNewHoleR(c)` (`1020:1d02`): full rule in [geracao-mundo.md](geracao-mundo.md#3a-entrance-holes-_makenewholebc-10201b06-and-_makenewholerc-10201d02). In play, they are called by `_GetOutB/R` (first exit through a new opening), `_DigTileThemB/R` at y = 0 and by the yellow ant's digging (`_DigMyTile`). Random draws: `S(31)` and nothing else in the yard; then `_DigTileB(c, 1)` (B) or the inlined digging (R).
- `_CreateNewHole(x, y)` (`1020:171a`), called by `_DigMyNewHole` (the yellow ant opens a hole on the surface): requires 0 < x < 127 and 0 < y < 63. Yard: `_MapA` = 0x50 with border (table `DGROUP:0x230c`); house: `_MapA` = 0x59. If x < 64: `_HoleMapB[y] = x`, `_DigTileB(y, 1)`, `_LastNewHoleB = (x, y)`, `_LastNewExitB = (y, 0)`. Else: `_HoleMapR[y] = x`; if `_MapR[y][1]` is dirt, `Sk(8)` and R totals; smooths (y, 0), (y+1, 1), (y, 2), (y−1, 1); `_FixExitMapR(y, 1)`; `_LastNewHoleR`/`_LastNewExitR`.

### 7.6 Laying an egg: `_PlaceEggB` (`1020:1004`) / `_PlaceEggR` (`1020:1068`)

If the list has < 500 entries and 0 ≤ x ≤ 63, 1 ≤ y ≤ 63: `_DigTileB(x, y)` (digs if it is dirt; random draws from section 7.3), appends (x, y, T = stage, M = 8, S = 0) and writes `Life[x][y]`. It does not test occupancy.

### 7.7 Passage between nests (only red ants going to the black nest)

The 0x14 cells exist in both maps, at the same (x, y) (7.3). In `_DoDigInR`, if after walking the red worker or soldier is on a 0x14 cell:

```
T_R = 0; Life_R[n] = 0
new T = ((T & 0x7F) < 0x30) ? 0x90 + Sk(8) : 0xB0 + Sk(8)    (red worker or soldier, random direction; loses its load)
_AddAntToBList(n, new T, M = 3, S = 0); _LifeB[n] = new T
```

(The M = 3 and S = 0 come from the disassembly, `1028:71ef`.) In the black nest, as an invader with M = 3, it follows the exit map until it leaves (3.2). There is no reverse path: black ants never use the passage.

### 7.8 `_FloodNestB` (`1020:29da`)

For every x and y = 3..63: dirt 0x20–0x2D adds 0x31; a value < 0x14 becomes 0x50. **No caller**; the values (0x50, +0x31) do not match the real flooding (section 8). [hypothesis: leftover from an old version]

## 8. Water

### 8.1 `_DoWater` (`1020:07c6`), every tick

```
if _RainOn ≠ 0 and _TERRAINset == 0:                       (raining in the yard)
    if S(50) == 0: sound 0x29
    _ClearRainFlag = 1
    if S(10) == 0 and _WaterTable > 4:
        _WaterTable −= 1; w = _WaterTable
        _DrownBList(w); _DrownRList(w)
        for x = 0..63: _MapB[x][w] = (_MapB < 0x20) ? 0x4E : _MapB + 0x2F
                       _MapR[x][w] = (_MapR < 0x20) ? 0x4E : _MapR + 0x2F
    for i = 0..99: drops on the surface (below)
else if _TERRAINset == 0:                                   (dry, in the yard)
    if _ClearRainFlag == 1: _ClearRainFlag = 0; for i = 0..99: if _MapA of drop i ∈ 0x74..0x77: = S(14)
    if _WaterTable < 64 and S(10) == 0:                     (S(10) only if < 64)
        w = _WaterTable
        for x = 0..63: _MapB[x][w] = (_MapB == 0x4E) ? S(8) : _MapB − 0x2F
                       _MapR[x][w] = (_MapR == 0x4E) ? S(8) : _MapR − 0x2F    (B and R alternating per column)
        _WaterTable += 1
(in the house, nothing)
```

Drops (surface; details in `superficie.md`): for each i, at cell (`_RxTab[i]`, `_RyTab[i]`) (`9:79e6`/`9:7a72`): if the value is 0x74–0x76, add 1 (the puddle grows); otherwise, if it is 0x77 it becomes `S(14)`, and then `_PlaceDrop(i)` draws a new position with **`RR(128)` and then `RR(64)`** (R generator); if the value there is < 0x0E, it becomes 0x74 and the scent of that 2×2 cell is washed away (`_PherMapA`, `_PherMapF`, `_PherMapBT`, `_PherMapRT` = 0; `_PherMapBN`, `_PherMapRN` −20, minimum 0).

Consequences:

- The water rises one row at a time, from the bottom (y = 63) upward, with chance 1/10 per rain tick, up to y = 4. Rows 0–3 never flood. When drying, it goes down from top to bottom, with chance 1/10 per dry tick.
- Stored food (0x10–0x13) on a flooded row becomes 0x4E and, when drying, tunnel `S(8)`: **the food is lost**, without lowering `_FoodB`.
- A flooded tunnel (0x4E) is impassable (`MOVE` requires < 0x1C; `STEP` requires < 0x30). Eggs and queen on a flooded row do not drown.

### 8.2 `_DrownBList(w)` (`1020:2d16`) / `_DrownRList` (`1020:2d66`)

For each ant in the list (back to front) with Y = w, T ≠ 0 and group 1..11: M = 17. Excluded are eggs (group 0), queen (12, 13) and clouds (14, 15). Applies to owners and invaders.

### 8.3 DROWNING (M = 17; `_DoDrownB` inlined)

Only the owner ant gets here (the invader ignores M = 17 and keeps leaving):

```
if Map[pos] ≤ 0x13: M = _GetNewModeB(group); END                 (no longer in the water)
r = S(3); T = (T & 0xF8) | ((T + r − 1) & 7); Life[pos] = T      (turns −1, 0 or +1)
if S(100) == 0: Life[pos] = 0; T = 0; counts death: _RAntsExpired if T was red, else _BAntsExpired
```

It does not walk (a flooded tunnel is impassable) and dies with chance 1/100 per tick until the water goes down.

`_AddWater(w)` (`1020:0b8a`) and `_DropWater(w)` (`1020:0c54`) are the same row flood/dry code, in a separate function, **with no caller**.

### 8.4 How rain behaves in the nest and in the tunnels (summary for the player)

User's question (2026-10-18): "How should rain behave in the nest? Right now it accumulates at the bottom of the dirt and seems to have no effect on the nest" and "What is the behavior of rain in the tunnels?". Answer, based on 8.1–8.3 (code) and the captures below:

- **Water comes from below, like a water table.** While it rains, each tick there is a 1 in 10 chance of the water rising **a whole row** of both nests (`_WaterTable` −1), starting at row 63 (the bottom) and at most up to row 4 (rows 0–3 never flood). A rain lasts `S(150) + 150` ticks (150–299, quintal.md §2): on average the water rises 15 to 30 rows, that is, **it stays in the lower half of the nest**. When the rain ends, the water goes down one row every ~10 dry ticks, from top to bottom. That is why the observed behavior ("accumulates at the bottom") is that of the original: in most rains the water does not reach the tunnels, which tend to be near the surface. Only back-to-back rains (the water had no time to go down) or long ones bring it there.
- **Flooded dirt barely changes color.** The dirt of a flooded row goes from 0x20–0x2D to 0x4F–0x5C (+0x2F), drawn almost the same as dry dirt (dotted dark blue); what you see from afar is **the flooded tunnel, light blue** (0x4E). With no tunnel on the row, the rising water is practically invisible.
- **A flooded tunnel (0x4E) becomes a wall of water:** no ant enters it (`MOVE` requires < 0x1C and `STEP` < 0x30). The colony loses the lower part of the nest until the water goes down; when it dries, the tunnel comes back as an ordinary tunnel (`S(8)`: one of the empty tunnel tiles).
- **Ants:** on the tick their row floods, the workers, soldiers and breeders (groups 1–11) on that row enter **DROWNING** (M = 17): they do not walk, they turn in place (direction −1, 0 or +1 per tick) and **die with chance 1/100 per tick**, counted as dead (`_BAntsExpired`/`_RAntsExpired`); if the water goes down first, they go back to work. Ants that enter an already flooded row do not drown (the tile blocks them). Invaders ignore drowning and keep leaving. There is no escape: only the water going down saves them.
- **Eggs, larvae, pupae and queen do not drown** (groups 0, 12, 13 are left out of `_DrownBList`), even under water; but they are stranded (the workers cannot reach them through the flooded tunnel).
- **Stored food** (0x10–0x13) on a row that floods becomes water and, when drying, empty tunnel: **the food is lost**, without lowering `_FoodB`.
- **The yellow ant** in a flooded nest cell loses 1 health per tick; with health 0 for more than 99 ticks, it dies **drowned** (code 7, formiga-amarela.md §3, §9).
- **On the surface**, the drops become puddles (tiles 0x74–0x77) that grow and vanish and **wash away the scent** of the 2×2 cell (trails, alarm, food; the nest scent drops by 20): food trails are lost in the rain (§8.1, quintal.md §9.2).

**Verified against the original (QEMU bench, 2026-10-18, `cenarios_tick.py gravar tela_chuva`, snapshot `rico`, `_RainCnt` = 2000):** with the water at row 25, the black nest of `rico` (tunnels on rows 0–12) looks the same as when dry; with the water at row 4 (after ~300 rain ticks), the tunnels below row 4 are light blue and the dirt stays dark blue. The port draws the same states (`chuva_ninho_preto_meio.ANT`, `chuva_ninho_preto.ANT` opened with `--save`) identically: flooded tunnels light blue from the same row. The rule effects (rows, drowning, food) were already checked by case Q-01 and by the tick vectors (quintal.md §9, Q-U8). Captures in `re_trabalho/a51/chuva2/` (outside git).

## 9. Invaders

### 9.1 Raiding the nest: `_RaidInB` (`1028:3524`) / `_RaidInR` (`1028:5b2a`) (invader with M = 7)

```
if Map[pos] ∈ 0x10..0x13:                                    (found food)
    removes one unit (0x10 → Sk(8)); _FoodB −= 1 if > 0      (_FoodB of the invaded nest)
    M = 3; T = T OR 8; Life[pos] = T                          (becomes "with food"; will leave)
else:
    r = S(3)
    if not MOVE((T + r − 2) & 7):                             (turns 0, −1 or −2)
        d = _GetEnterDirB(pos, dir); if d < 0: d = S(8)
        if not MOVE(d): M = 1; Life[pos] = T                  (gives up; with M ≠ 7 it starts leaving)
```

Note `S(8)` (remainder) and not `Sk(8)` here; both take one step of the generator.

### 9.2 LEAVE_NEST (invader; `_RaidOutB` inlined)

```
e = _GetExitDirB(pos, 8)                                     (no direction excluded)
if not MOVE(e ≠ 0 ? e − 1 : Sk(8)):
    if not MOVE(Sk(8)): Life[pos] = T
```

### 9.3 Staying in the red nest: `_StayInR` (`1028:5c16`)

Black ant in Defense (M = 6) in the red nest, with the yellow ant also there (`_MePlane` = 3):

```
if Map[pos] ∈ 0x10..0x13: steals one unit (_FoodR); M = 3; T = T OR 8
else:
    r = S(3); d = (T + r − 2) & 7; T = (T & 0xF8) | d
    if MOVE(d): END
    d2 = _GetEnterDirR(pos, dir); if d2 < 0: d2 = S(8)
    if MOVE(d2): END
Life[pos] = T
```

## 10. Pre-flight and rest

**PRE-FLIGHT** (M = 14):

```
if Sk(32) == 0: M = _GetNewModeB(group)
if not FIGHT_OWN_CELL(): TRY(dir)
if _FlyAwayB > 100: M = 15                                    (R: _FlyAwayR)
```

**REST** (M = 13; `_DoRestB`):

```
if FIGHT_OWN_CELL(): END
Life[pos] = T
if S(20) ≠ 0: rest balloon if _OptionStates[5] (B: == 1; R: ≠ 0); END
M = _GetNewMode(group, T)
```

The resting ant does not walk and rewrites its own cell in `Life` every tick.

## 11. Differences between the red nest and the black nest

| Point | Black nest (B) | Red nest (R) |
|---|---|---|
| New mode | `_GetNewModeB`: with `_ModeAuto` = 0, worker/soldier use `_ModeMe` (the player's triangle) | `_GetNewModeR`: always `_ModeTabWB/SB[_StrategicModeR][Sk(8)]` |
| Defense (6) | stays in the nest if the yellow ant is in it (4.2) | always leaves |
| Nesting (1) | carries the egg to the bottom, keeps the stage | drops at random (1/4), stage goes back to 2 (5.4) |
| Dig In, undiggable dirt | `M = _GetNewModeB(group)` | `M = 0` |
| Dig In, 0x14 cell | nothing | goes to the black nest (7.7) |
| Dig In/Dig Out, bite from the store | new cell | old cell |
| Dig Out blocked | `Exit[pos] −= 1` without test (0 → 0xFF) | only if ≠ 0 |
| Trophallaxis in `MOVE` | yes | no |
| Yellow ant test in the fight check | before the enemy test | after |
| Invader in its own cell | black yellow-ant test; egg theft counts `_TotalEggsDiedB` | no yellow-ant test; theft without counter; black egg (T < 8) is destroyed |
| Invader in M = 6 | leaves | `_StayInR` if the yellow ant is in the red nest |
| End of fight, winner of the nest's color | `_GetNewMode(group, T)` | `VitóriaR` table (4.4) |
| `_DecEat` | respects `_AlwaysHealthy` | no |
| Balloons (rest, fight) | `_OptionStates[5] == 1` | `≠ 0` |
| Eggs, queen | [ciclo-de-vida.md](ciclo-de-vida.md#9-red--black-differences) | |

## 12. Functions present but not called

No caller in the pseudocode (the same code is inlined in another routine): `_DoRestB/R`, `_DoDrownB/R`, `_DoRandB/R`, `_DoRecruitN`, `_RaidOutB/R`, `_LeaveNestB`, `_TryEatFoodB/R`, `_EatFoodB/R`, `_StealFoodB/R`, `_PickupFoodB/R`, `_MakeNewTailB/R`, `_KillTailB/R`, `_FloodNestB`, `_AddWater`, `_DropWater`, `_FoodFall`. `_CheckNestFightB/R`, `_DropFoodB/R`, `_LostHeadB/R` and `_LostTailB/R` appear only in their own definition. The port must implement the inlined behavior described here (there are small differences between the standalone functions and the inlined code, for example `_LeaveNestB`).

## 13. Test cases against the original

Test bench: [../../bancada-qemu.md](../../bancada-qemu.md#casos-de-teste-injetar-um-save-e-rodar). It injects and loads edited saves, reads the whole state from memory and writes variables (`escrever_variavel`), but the generators' state does **not** go into the save (and is lost on load; see [rng.md](rng.md#is-the-seed-in-the-save)). That is why the cases below are of three types: **D** = deterministic (does not depend on the random draw), **E** = statistical (repeat and compare proportions), **P** = comparison with the port running the same `.ANT` with the same forced seed.

### 13.1 How to build the input

- Start from `tests/referencias/saves/QUICK2.ANT` (tick 78) and describe the edit in a JSON case in `tests/referencias/casos/` (operations of `tools/ant_editar.py`: scalars, cells, ants with `ListIndex`). `tools/caso_ant.py rodar CASO.json` injects the save into a floppy disk, loads it via File > Load, unpauses, runs until the target `GameTime`, pauses, reads the state from memory (without needing to save) and checks.
- Loading leaves the game **paused** (`_GamePaused` = 1) and **recounts** `_CastePopB/R`, `_BpopT/_RpopT` from the lists [observed on the test bench, 2026-10-16]: populations cannot be forced through the file, only through the lists.
- **On load the original rebuilds `_LifeB/R` from the lists** (`_DoLoadInitializations` `1010:9dbe`), from index `_ListIndexB` (inclusive: the slot right after the last one is also written) down to 0. When two ants are in the same cell, the one with the **lowest index** stays in `Life`. Zero slot `_ListIndexB` to avoid creating a phantom occupant.
- Compare by `_GameTime` (offset 45740), never by wall-clock time.

### 13.2 Cases

| Id | Rule | Input | Expected | Type |
|---|---|---|---|---|
| N-01 | natural death (3.1) | QUICK2 with `_HealthB` = 0 and 40 black workers (T 0x12, M 13, resting) in the tunnel | average loss of 40 × (1/256) × (31/32) ≈ 0.15 ant/tick (≈ 19 in 128 ticks); `_BAntsExpired` rises the same. With `_HealthB` = 31: no deaths | E (and D for health 31) |
| N-02 | immune queen (3.1) | `_HealthB` = 0 | the queen does not die by 3.1, only by starvation (ciclo-de-vida) | D |
| N-03 | leaving the nest (4.5, 4.6, 7.2) | a black worker T 0x12, M 2, at (32, 3) in the QUICK2 tunnel | climbs column 32 (`Exit` 0xFC → 0xFD → 0xFE) and appears in list A next to the hole (3, 32) on the 3rd tick, with M = 2 and S = 120 | D (except the direction on the surface) |
| N-04 | carrying an egg to the bottom (5.1) | worker T 0x0A (carrying), M 1, S 1 at (32, 2) | descends the gradient (32,3) → (31,4) → (30,5) → (31,6) → (32,7) → (33,8) → (34,9) → (35,9) → (36,9), the minimum (0xF4); there it sets bit 3 of S, takes an `Sk(8)` step and on the next tick drops an egg T = 1 wherever it is; it becomes group 2 with S = 8 | D up to the chamber, E afterwards |
| N-05 | red nest drops at random (5.4) | red T 0x8A, M 1 in the red tunnel | the dropped egg has T = 0x82, in any cell along the path | D (stage), E (place) |
| N-06 | doubled food (6.2) | black worker T 0x1A (with food), M 3, at (32, 3); `_FoodB` = 0 | at the end, `_FoodB` = 2 and the map has 2 units (0x11 in one cell, or 0x10 in two) | D in total |
| N-07 | eating and health (6.1) | `_HealthB` = 50, `_EatCountB` = 0, `_BpopT` = 3, soldiers 0; a soldier (T 0x32) M 1 stuck in a 0x13 cell surrounded by dirt | each bite adds 5 to `_EatCountB`; since 5 > 3 >> 4 = 0, health rises 1 per bite and the counter goes back to 0; bite with chance 49/100 per tick (`S(100)` > 50); the cell goes 0x13 → 0x12 → 0x11 → 0x10 → tunnel; `_FoodB` goes down to 0 | E |
| N-08 | digging (7.1, 7.3) | worker T 0x12, M 4 at (36, 9) | new tunnels are one cell high; the worker becomes T 0x2x, M 5, then goes back to M 4; `_TilesDugB` and `_TileMassXB/YB` change as in the formula | D in shape, E in layout |
| N-09 | untested decrement (7.2) | in the black nest, `_ExitMapB` = 0 on a tunnel cell surrounded by flooded dirt (0x4F+) with a worker M 2 on it | the cell's `_ExitMapB` becomes 0xFF; in the red nest, under the same conditions, it stays 0 | D |
| N-10 | flooding (8.1, 8.2, 8.3) | `_RainOn` = 1, `_RainCnt` = 600, `_WaterTable` = 64 | `_WaterTable` drops about 1 every 10 ticks down to 4; each row y = `_WaterTable` shows 0x4E in the tunnels and +0x2F in the dirt; ants on it get M = 17 and die ≈ 1 %/tick; eggs and queen do not change M | E (pace), D (values) |
| N-11 | drying (8.1) | after N-10, rain stopped | rows return from top to bottom; food that was on a flooded row disappears from the map, `_FoodB` does not change | D |
| N-12 | fight does not remove the checker (4.3) | black nest, same cell: red T 0x92 M 0 at a **lower** index (stays in `_LifeB` on load) and black T 0x12 M 13 (rest) at a higher index | on the first tick the red becomes a cloud (T 0x70 or 0xF0, M 10, S = winner) and the black continues; at the end (≈ 16 ticks) there are 2 black ants (the black won) or 1 black and 1 red | D in structure, E in winner (≈ 50 %) |
| N-13 | queen attacked (3.2) | red T 0x92 M 7 appended at the end of list B (index 5), in the cell of the black queen's head (34, 9) | on the first tick the head becomes a fight cloud and the tail (index 1) disappears in the same tick; `_BlkQueens` = 0; the cloud never becomes a queen again; at the next `_CountAnts` (tick ≡ 0 or 64 mod 128) the Quick Game ends in defeat | D |
| N-14 | passage (7.7) | 0x14 cell in `_MapB` and `_MapR` at (20, 60), red worker M 4 next to it in the red nest | on entering (20, 60) it disappears from list R and appears in list B with T 0x90–0x97, M 3 | D (except direction) |
| N-15 | trophallaxis (4.1) | yellow ant in the black nest with `_MeWantFood` = 1; red invader walking into its cell | the red ant feeds the yellow ant (the same effect as a black ant) | D |
| N-16 | port, tick by tick | QUICK2, forced S seed (in the port) | the port, running from QUICK2 with any seed, must keep the invariants: T ∈ valid types, `Life` consistent with the lists, `_FoodB` changes only in the routines of section 6 | P |

### 13.3 Observed in the original

QEMU bench, `python tools/caso_ant.py rodar tests/referencias/casos/ID.json` (2026-10-17). The test bench only stops ~5 ticks after the load (the script waits 1 s after unpausing), so the few-tick cases check what still holds a few ticks later. States in `tests/referencias/saves/` (`ID_carregado.ANT` = right after the load; `ID_Tnnn.ANT` = final).

| Id | Effective input | Observed | Result |
|---|---|---|---|
| N-03 | QUICK2 + worker T 0x12, M 2 at (32, 3) (B[5]) | stopped at tick 84: B[5].T = 0; `_ListIndexA` = 3; A[2] with T & 0xF8 = 0x10, M = 2, S = 117, at (6, 33): left at tick 81 through the hole (3, 32) with S = 120 and walked 3 steps (S −1 per step) | **confirmed** (§4.5, §4.6, §7.2; S = 120 with M = 2 and hole on the black side) |
| N-06 | QUICK2 without A[0], `_HealthB` = 100, `_AlwaysHealthy` = 1, `_FoodB` = 0; worker T 0x1A, M 3 at (32, 3) | tick 100: `_FoodB` = 2; the worker with T 0x16 (group 2) | **confirmed**: one load yields two units (§6.2, "doubled food") |
| N-09 | cell 0x02 with exit 0 surrounded by 0x5D at (10, 40) in both nests; worker M 2 on each | tick 86: `_ExitMapB`(10, 40) = 0xFF, `_ExitMapR`(10, 40) = 0; both with M = 4, stopped | **confirmed**: untested decrement in the black nest, tested in the red one (§7.2) |
| N-13 | QUICK2, yellow ant moved to (30, 5); red T 0x92, M 7 at (34, 9) at the end of list B | tick 128: B[1] (tail) and B[5] (invader) with T = 0, `_BlkQueens` = 0; B[0] (was the head) with T = 0 and **M = 8**: the cloud ended with winner S = 0 (the "occupant" 0 won) and the mode came from `_GetNewMode(0, 0)` = `_CasteModeTabB[0]` = 8; `_RAntsKilled` = 1. At tick 128 `_CountAnts` recounted `_CastePopB[0..4]` and stopped at message 10010 (modal): `_CastePopB[5]` = 1 and `_IsGameOver` = 0 until someone clicks | **confirmed** (§3.2, "queen attacked": the queen disappears and the tail goes with it; the `_GetWinner(0, ...)` defect is confirmed). The game over cannot be checked without clicking the message |

**Tick vectors (2026-10-18, `tools/cenarios_regras.py`, snapshot `rico`).** The statistical cases became tick vectors: the setup is written to memory at the entry of a tick and the original runs tick by tick; each vector checks the rule with the seed the original had (the "statistical" becomes deterministic). All 307/307 with S, R and health ([testes-tick.md](testes-tick.md#fifth-round-2026-10-18-unverified-rules-from-the-review)):

| Id | Vector | Setup | Observed in the original |
|---|---|---|---|
| N-01 | `ninho_n01` (1714, 1720), `ninho_n01_saude31` (1711) | `_HealthB` = 0 and `_AlwaysHealthy` = 0 (`rico` has only eggs and the queen in the black nest) | one egg dies at 1714 and another at 1720 (`_BAntsExpired` +1: eggs also die by 3.1); with `_HealthB` = 31 no deaths in 2 ticks — **confirmed** (3.1, including "31 is already enough") |
| N-07 | `ninho_n07` (1712, 1713) | 0x13 cell surrounded by dirt, black soldier T 0x32 M 1 appended on it; `_HealthB` 50, `_EatCountB` 0 | bites at 1712 and 1713: `_FoodB` −1, cell −1, `_HealthB` +1 — **confirmed** (6.1) |
| N-10 | `ninho_n10` (1711, 1713) | rain (`_RainOn`, `_RainCnt` 600) with rows 50..63 flooded by hand and `_WaterTable` = 50 | rises one row at 1711 and at 1713 (`_WaterTable` 50 → 49 → 48, 0x4E in the tunnels, +0x2F in the dirt) — **confirmed** (8.1, 8.2); no adult ant on the rows (`rico` only has eggs there), so the 1/100 per tick death (8.3) was not exercised |
| N-12 | `ninho_n12` (1711, 1716) | two eggs become red T 0x92 M 0 (index 2) and black T 0x12 M 13 (index 14) in the same cell (33, 4); `_LifeB` = 0x92 | at 1711 the red becomes a cloud (T 0x73, M 10, S 0x12 = black winner) and the black continues (the checker is not removed); the cloud lasts until 1716 and becomes black (0x12, M 1) — **confirmed** (4.3, 4.4) |
| N-14 | `ninho_n14` (1713) | 0x14 at (20, 60) in both maps; the only adult red worker of `rico` facing the passage, M 4, on a neighbor | at 1713 it leaves list R and enters B with T 0x91 (0x90 + Sk(8)), M 3 — **confirmed** (7.7) |
| N-15 | `ninho_n15` (1711) | yellow ant in a black nest tunnel with `_MeHealth` 20 and `_MeWantFood` 1; red invader T 0x90 M 0 right below, on the exit path | at 1711 the invader enters cell 0xFF and `_DoTroph` gives `_MeHealth` = 100 — **confirmed**: MOVE's `X < 0x80` test does not look at the color (4.1). Only `_gameCycles` is not required: the "You feel much better now!" box (`_YellowDialog`) does not stop the timer |

**Seventh round (2026-10-19, `tools/cenarios_raros.py`, [testes-tick.md](testes-tick.md#seventh-round-2026-10-19-rare-branches)):** the remaining cases, also as tick vectors 307/307:

| Id | Vector | Setup | Observed in the original |
|---|---|---|---|
| N-02 | `ninho_n02` (1711) | `_HealthB` = 0 and seed S = 0x00A5 at entry (chosen in the port so that the queen's head draws `Sk(256)` = 0) | the queen stays alive (T 0x60): **confirmed** (3.1, M = 9 excluded). Without the seed, with `_HealthB` = 0, it dies only from starvation (`rainha_fome`, ciclo-de-vida.md 5.2) |
| N-04 | `ninho_n04` (1719, 1721) | worker T 0x0A, M 1, S 1 appended at (33, 3) | descends the exit map to (38, 9), sets bit 3 of S (1719: S = 9), takes the step and on the next tick drops the egg T = 1 at (37, 10) and becomes T 0x17, S = 8 — **confirmed** (5.1, same stage) |
| N-05 | `ninho_n05` (1711, 1714) | red T 0x8A and 0x8C, M 1, at the top of the red tunnel | always drop eggs T = 0x82, in different cells along the path, and pick up eggs again — **confirmed** (5.4) |
| N-08 | `ninho_n08` (1713, 1718) | worker T 0x12 and soldier T 0x34, M 4, at the bottom of the black tunnel | they dig: T + 0x18 (0x2B; 0x4A), M 5, `_TilesDugB` + 1, and climb with the dirt until they leave — **confirmed** (7.1) |
| N-11 and 8.3 | `ninho_n11` (1711, 1719, 1722, 1754) | rows 6..63 flooded as `_DoWater` would, without rain; black and red adults on the rows, M = 17; food 0x11 in two cells | drowning ants die (red male at 1711, black worker at 1722; in the 120 recordings also a black soldier, a red worker and a red soldier: `_BAntsExpired`/`_RAntsExpired`), the water goes down one row (1719) and the food row dries into tunnel `S(8)` with `_FoodB` unchanged (1754) — **confirmed** (8.1, 8.3) |
| 4.6 | `lista_a_cheia`, `lista_a_cheia_sem_vagas` (1793) | `_ListIndexA` = 1000 (with empty slots; or with them filled by idle workers) and a worker leaving through the opening | the original and the port give the same state (the ant is lost; compaction opens the slots of those that died or entered the nest in the same tick) |

Port: `engine/tests/regras_ninho.rs` starts from each `ID_carregado.ANT`, runs the port's tick (only the nest rules enabled) until the same target `GameTime`, with 6 different S seeds, and evaluates the same checks as the JSON: all pass.

**Tick vectors** (`tests/referencias/ticks/`, ticks 79–83 of the `quick` snapshot, see [testes-tick.md](testes-tick.md)): for each tick, starting from `estado_antes`, the port finds a nest entry S seed such that `_DoAntSimB` + `_DoAntSimR` reproduce **exactly** the maps, exit maps, `_LifeB/R`, lists and colony counters of `estado_depois` **and** the S generator ends at the value in `extras.json` (k random draws: 13, 12, 12, 12, 12; the entry seed is the final one walked k steps back). Tick 80 includes an egg laying by the black queen. This confirms the order and number of the nest's random draws in those ticks (natural death, queen, tail, eggs) and that nothing after `_DoAntSimR` draws from S in those ticks. It also confirms that `_LifeB/R` rebuilt from the lists match the original's memory.

## Implementation in the port

- `engine/src/rules/ninho.rs`: `impl RegrasNinho for RegrasOriginais` calls `simular_ninho(e, cor, tabelas, tabelas_amarela)` (public, for tests with other tables). The ant being processed is a `Formigueiro` (index = `_Tindex`, with incoming T, X, Y, M, like the original's parameters); each method cites the original routine and the section of this spec.
- Tables in `data/ninho.json` (`TabelasNinho`), generated by `tools/extrair_ninho.py` (addresses at the top of the script): `_Dx8/_Dy8`, `_TurnTab`, `_ModeTabWB/SB`, `_CasteModeTabB`, `_CasteTabC`, `VitóriaR`, fight class and matrix, hole border. Stored in `RegrasOriginais::ninho` (with mods); `TabelasNinho::padrao` is the embedded copy, for tests.
- `_YellowFight` and `_DoTroph` are those of `rules/amarela.rs` (`lutar_com_amarela`, `trofalaxia`). Balloons are not implemented; the digging sound (0x11/0x12) becomes `Evento::Som` ([som.md](som.md)).
- `_DigTile*`, `_SmoothEdges*`, `_FixExitMap*`, `_MakeNewHole*`, `_PlaceEgg*`, `_GetNewMode*`, `_GetWinner`, `_EatFood*`/`_StealFood*`, `_DecEat*`, `_GetBestDir` and `_ExitHole` are in `engine/src/rules/comum.rs` (see [../../decisoes.md](../../decisoes.md), "Common rules module"); `vencedor_da_luta` remains public in `ninho.rs`, with the nest tables.

## Gaps

- Surface drops and puddles (`_PlaceDrop`, `_InitWater`): only the order of the random draws is here; full rule in `superficie.md`.
- ~~`_ExitHole` with list A full~~: resolved on 2026-10-19 (§4.6, "List A full"); the port follows the original and the `lista_a_cheia*` vectors (§13.3) match.
- Tested on the test bench: N-03, N-06, N-09, N-13 (cases, §13.3) and, as tick vectors, N-01, N-07, N-10, N-12, N-14 and N-15 (2026-10-18) and N-02, N-04, N-05, N-08, N-11 and the drowning of adults (2026-10-19). All cases of §13.2 are checked.
- The "defects" marked ([hypothesis]) must be confirmed in the original before deciding whether the port reproduces them by default (the project's principle is to reproduce; a mod can fix them).
