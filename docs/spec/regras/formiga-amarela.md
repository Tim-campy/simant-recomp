*Leia em português: [formiga-amarela.md](../../pt-BR/spec/regras/formiga-amarela.md)*

# Yellow ant and player controls

This document specifies, for 1:1 reimplementation, the **yellow ant** (the player's body): state, presence on the map, what happens to it each tick, how it walks to the clicked destination, how it finds a path, enters and leaves the nests, picks up and drops objects, eats, fights, recruits, swaps bodies, founds a colony, dies and reincarnates. At the end, the **controls**: how clicks, keys, menus and accelerators become orders. The yellow ant can also control the spider; the spider part is in [inimigos.md](inimigos.md#27-aranha-controlada-pelo-jogador-_memode--1). The tick is in [simulacao-visao-geral.md](simulacao-visao-geral.md) (steps 17 and 20); the ordinary ants in [superficie.md](superficie.md); the creation of the yellow ant in a new game in [geracao-mundo.md](geracao-mundo.md) (3.D).

Evidence and notation conventions: the same as in [superficie.md](superficie.md#convenção-de-evidência). "Fact verified in `QUICK2.ANT`" uses the offsets of [save-ant.md](../formatos/save-ant.md). No decompiled code is reproduced.

## 1. State: the `_Me*` variables

**Fact seen in the code** (addresses from `symbols.tsv`; all of them are in the save). Values from `QUICK2.ANT` (tick 78, Quick Game) **verified**.

| Variable | Address | Meaning | `QUICK2.ANT` |
|---|---|---|---|
| `_MePlane` | `10:ce80` | plane: 0 yard view, 1 surface, 2 black nest, 3 red nest | 2 |
| `_MeLocX`, `_MeLocY` | `10:cd88`, `10:ce7e` | cell | 34, 9 |
| `_MeType` | `10:cc84` | group × 8 **without** color and without direction: 0x10 worker, 0x30 soldier, 0x40 winged queen, 0x60 queen; +8 = carrying (0x08 egg, 0x18/0x38 food, 0x28/0x48 dirt) | 0x10 |
| `_MeDir` | `10:cf50` | direction 0–7 | 2 |
| `_MeColor` | `10:ce98` | 0 = black, 0x80 = red (high byte of `_grndTile`+2 in the pseudocode) | 0 |
| `_MeHealth` | `10:ac8a` | health 0–100 | 99 |
| `_MeHealthCntDwn` | `9:9cf0` | ticks with health 0 (dies when it goes past 99) | 0 |
| `_MeWarnHealth` | `9:9bec` | hunger threshold chosen in the `editHealthY` control (default 30, `_InitSimVars`) | 30 |
| `_MeWantFood` | `9:9af2` | 1 = hungry (health ≤ `_MeWarnHealth` or ≤ 9) | — |
| `_MeEgg` | `10:ce92` | egg stage shown during rebirth | — |
| `_MeEggCarried` | `8:8a6a` | T of the carried egg (0xFD when none) | — |
| `_MeFoodInType` | `8:8a6e` | what was eaten in the last pickup: −1 nothing, 0 food, 3 egg | — |
| `_MeNestStarted` | `8:8a6c` | 1 = the player's colony has already been founded | 1 |
| `_MeBase` | `10:cf54` | zeroed on body swap [use unknown] | 0 |
| `_MeMode` | `9:9fe8` | 0 = ant, 1 = **spider**, 2 = Experimental mode (no body) | 0 |
| `_MeSMode` | `9:7d60` | spider hunting mode (see [inimigos.md](inimigos.md)) | 0 |
| `_MeMoveMe` | `9:7d24` | 1 = there is an order in progress | 0 |
| `_MeCmd` | `9:9bc4` | order: 0 go, 1 go and pick up/drop, 2 go digging, 3 attack the target, 4 be fed by the target | 0 |
| `_MeGoalPlane`, `_MeGoalX`, `_MeGoalY` | `9:9be0`, `9:80c6`, `9:80d2` | destination of the order | — |
| `_MeLastX`, `_MeLastY` | `9:a0d6`, `9:a0da` | previous cell (does not go back to it when going around) | — |
| `_MeSteps` | `9:9c7e` | steps taken (the parity charges health) | — |
| `_MeDis`, `_MePrevDis` | `9:9fe2`, `9:80aa` | distance to the hole when picking up food [use unknown] | — |
| `_MeCrazyCnt`, `_MeCrazyDir`, `_MeCrazyRot` | `9:72e4`, `9:a0d8`, `9:78a4` | state of the obstacle avoidance (5.3) | −2, —, — |
| `_MeTargIndex`, `_MeTargLife`, `_MeTargLifePlane`, `_MeTargLifeX/Y` | `9:99e4`, `9:72ee`, `9:9c6c`, `9:99e8/99ea` | target ant of an order 3 or 4 | — |
| `_MeStartedFight` | `9:9e7c` | 1 while the fight was started by the yellow ant (chooses the death message) | — |
| `_MeDropAlarm` | `9:75fa` | 1 = dropping alarm pheromone on each step | — |
| `_MeScent` | `10:ce88` | trail intensity when carrying food (200 → 10) | 0 |
| `_FuzLocX/Y` | `9:9fe4/9fea` | "fuzzy" position for the recruited ants ([superficie.md](superficie.md), 9.1) | 34, 0 |

## 2. Presence on the life map

**Fact seen in the code.** The yellow ant is in no list at all. It marks its cell with 0xFF; the queen (`_MeType = 0x60`) also marks the cell behind it, in the opposite direction (`_MeDir ^ 4`), with 0xFE.

- `_SetMyLife(plane, x, y, type, dir, v)` (`1020:6536`): writes `v` (and `v` or 0xFE in the tail) and, if `v ≠ 0`, copies plane/position/type/direction into the `_Me*` variables. With `v` = 0 it erases without touching the `_Me*` variables.
- `_ClearMyLife(plane, x, y, type, dir)` (`1020:631e`): erases the cell only if it contains 0xFF (and the tail only if it contains 0xFE).
- `_MoveMyLife(plane, x, y, type, dir)` (`1020:65fa`): `_ClearMyLife` at the current position, writes 0xFF (and 0xFE) at the new one and updates the `_Me*` variables. Plane 0 is treated as 1.
- `_SetLife` (`1020:5d18`), used by all of them: in a **nest**, when writing a value ≥ 1 into a dirt cell (0x20–0x2E) or a closed surface cell (0x1C–0x1F), it **digs** the cell (`_DigMyTile`) and plays a sound. This is how the yellow ant digs: it just has to enter the dirt (this also applies to any ant placed in the nest by this function).

## 3. Each tick: `_DoAntSimY` (`1028:7cf6`)

**Fact seen in the code.** It only acts with `_MeMode = 0`:

```
_SetMyLife(_MePlane, _MeLoc, _MeType, _MeDir, 0xFF)          ← rewrites the mark every tick
if (_Cycle & 63) == 0: _SetMyHealth(_MeHealth − 1)
if _MePlane ≥ 2 and (t = nest tile at the cell) > 0x4D: _SetMyHealth(_MeHealth − 1)   ← flooded cell
if _MeType == 0x60 and _MePlane ≥ 2 and (_Cycle & 15) == 0:                              ← queen lays egg
    p = _MeLoc + 2·D8[_MeDir ^ 4]
    if _InNestBounds(p) (0 ≤ x < 64, 0 < y < 64) and S128() ≤ _MeHealth:   ← S128 only inside the bounds
        _PlaceEggB(p, 1) and _DecEatB()   (or R, according to _MeColor)
        _SetMyHealth(_MeHealth − 5)
if _MeHealth < 1:
    _MeHealthCntDwn += 1
    if _MeHealthCntDwn > 99: _YellowDeath(7 if in a nest with t ≥ 0x4E, else 8)   ← drowned / starved
```

`_SetMyHealth(v)` (`1020:8c70`): if `_NeverHungry` (`8:8a5e`) ≠ 0, v = 100; if v > 0, `_MeHealthCntDwn = 0`; clamps v to 0..100; `_MeHealth = v`; `_MeWantFood = 0` if `v > _MeWarnHealth` and `v > 9`, else 1.

## 4. Walking by order: `_DoAntMoveY` (`1028:75f4`)

Called in step 20 of the tick. It also runs with the game paused if `_MeMoveMe ≠ 0` and no special tool is active ([simulacao-visao-geral.md](simulacao-visao-geral.md), section 4). **Fact seen in the code.**

```
if _MeMoveMe == 0: end
r = 0                                          ← 0 = continue ; −1 = completed ; −2 = failed
if _MeCmd ≥ 3 (target):
    (tx, ty, Tt) = current position and T of slot _MeTargIndex in plane _MeTargLifePlane
    if the slot vanished or (Tt ^ _MeTargLife) & 0xF0 ≠ 0: r = −2 ; END
    if the target is on another plane, or more than 1 cell away: destination = (tx, ty) ; continues as a walk order
    else (neighbor): turns toward it ; r = −1 ; END
WALK:
    if _MeCmd == 1 and _MeGoalPlane == _MePlane:
        k = _GetDir(_MeLoc, destination) ; p = _MeLoc + D9[k] (clamped to the map)
        if p == destination: q = _TryMyDropOrLift(_MePlane, destination)
            if q ≠ 0: r = (q == 1 ? −1 : −2) ; END
    d = _GetMyDir(_MePlane, _MeLoc, _MeGoalPlane, destination)     (section 5)
    if d < 0: r = d ; END
    _MeLast = _MeLoc ; _MoveMyLife(_MePlane, _MeLoc + D8[d], _MeType, d) ; _MeSteps += 1
    surface:
        if _MeScent > 0 and carrying food (_MeType 0x18 or 0x38):
            deposits _MeScent in the trail of color _MeColor (c = max) ; if _MeScent > 10: _MeScent −= 1
        if _MeDropAlarm: alarm = min(alarm + 50, 200)
    nest: updates the exit map (_FixExitMapB/R)
    if _MeSteps odd:
        if winged queen (0x40) without colony (_MeNestStarted = 0) and _HealthB > 0 and not _AlwaysHealthy: _HealthB −= 1
        _SetMyHealth(_MeHealth − 1)                            ← 1 health every 2 steps
    surface and cell is a hole and (destination in a nest, or destination is this cell): ENTER (6.1)
    nest, row y ≠ 0: if the destination tile is 0x14 and not a queen and destination == current cell:
        nest swap (6.3) ; r = −1
    nest, row 0: EXIT (6.2)
    after ENTER/EXIT: if the destination was exactly this cell/plane and the new plane ≠ displayed plane:
        centers (if AutoTrack off) and r = −1 (the order ends; the code falls into the
        "completed" label, `1028:7b6b`)  ← verified in the vector amarela_superficie/1852
        (if the new plane is the displayed one, r stays 0 and the order goes on, with the destination still on the surface)
END:
    if "AutoTrack": centers the view on the yellow ant
    if r == 0 and arrived (plane and cell of the destination) and _MeCmd == 0: r = −1
    if r ≠ 0:
        r == −2: error sound and centers
        r == −1 and _MeCmd == 3: _MeStartedFight = 1 ; _YellowFight(_MeTargLifePlane, _MeTargIndex) ; _MeStartedFight = 0
        r == −1 and _MeCmd == 4: turns the target toward the yellow ant (except queens) and _EatMyFood(1)
        _ResetYellowVars(_MePlane, _MeLoc)                       ← ends the order
        house and surface and tile under the yellow ant = 0x76 or 0x78: _YellowDeath(10)   ← electric shock
```

`_ResetYellowVars(plane, x, y)` (`1000:72ba`): `_MePlane` and `_MeLoc` = arguments (correction: the previous version of this spec omitted this), destination = position, `_MeLast` = position, `_MeCmd = 0`, `_MeMoveMe = 0`, `_MeDis = _MePrevDis = _MeSteps = 0`, `_MeCrazyCnt = −2`.

`D9` = `_Dx9/_Dy9` (index 0 = standing still). No random draw in this whole flow, except the `S8()` of `_ExitNest` (6.2) and the random draws of the actions called.

## 5. Path: `_GetMyDir` and company

### 5.1 Intermediate target — `_GetMyDir(plane, x, y, dest_plane, dx, dy)` (`1028:8eca`)

**Fact seen in the code.** First it chooses where to walk now:

| Where it is | Destination | Immediate target |
|---|---|---|
| surface (plane < 2) | surface | the destination |
| surface | black nest | `_LastNewHoleB` (`8:835a`, last black hole created) |
| surface | red nest | `_LastNewHoleR` (`8:835e`) |
| nest | the same nest | the destination |
| black nest | another plane | `_LastNewExitB` (`8:8352`: column of the last black hole, row 0) |
| red nest | another plane | `_LastNewExitR` (`8:8356`) |

Then it applies the avoidance machine (5.3) with that target.

### 5.2 Greedy step — `_GetMyBestDirs(plane, x, y, tx, ty)` (`1028:8828`)

Same rule as `_GetRedBestDirs` ([superficie.md](superficie.md), 9.2), with the passability test `_TileCanBeMovedOn(plane, neighbor, _MeGoalPlane, _MeGoalX, _MeGoalY, dig = (_MeCmd == 2))` (5.4): −1 if it is already at the target; otherwise the best free neighbor that reduces the distance (with the threshold dropping at each improvement), or the best one blocked by an ant/non-free tile, or −2 if no passable neighbor reduces the distance. No random draw.

`_CheckMyBestDirs(&n, plane, x, y, tx, ty)` (`1028:8b40`) simulates the greedy step without changing anything: applies `_GetMyBestDirs` from the position, up to 64 steps. Returns −2 if the simulation got stuck (local minimum), −1 if it arrived or completed 64 steps.

### 5.3 Obstacle avoidance — `_MeCrazyCnt`, `_MeCrazyDir`, `_MeCrazyRot`

**Fact seen in the code.** Each destination click sets `_MeCrazyCnt = −2`.

```
if _MeCrazyCnt < 0:                                          (direct mode)
    d = _GetMyBestDirs(target)
    if d == −2 and _MeCrazyCnt == −2:                            ← only on the first block after the click
        _MeCrazyDir = _GetDir(position, target) − 1 ; _MeCrazyCnt = 16 ; _MeCrazyRot = 0
        d = _GetMyRandDirs(...)
    return d
else:                                                       (going around)
    if _CheckMyBestDirs(target) == −2: d = _GetMyRandDirs(...)
    else: _MeCrazyCnt = −1 ; d = _GetMyBestDirs(target)
    _MeCrazyCnt −= 1
    return d
```

Consequence: the avoidance lasts at most 17 steps; if the greedy step has a free path again, the counter goes to −2 and a new avoidance can start. If the avoidance ends and the greedy step is still blocked, `_MeCrazyCnt = −1` and `_GetMyBestDirs` returns −2: the order fails (error sound).

`_GetMyRandDirs(&rot, &dirc, plane, x, y, tx, ty)` (`1028:8928`), with no random draw:

```
dist = _GetDis(position, target) ; if dist ≤ 0: return −1
ok[k] = neighbor k passable (5.4, with dig = (_MeCmd == 2)) and ≠ (_MeLastX, _MeLastY)
if none ok: return −2
a = b = dirc
if rot == 0:                                       ← first step of the avoidance: chooses the side
    for j = 0..7:
        if ok[a]: dirc = _GetDir(position, target) − 1 ; rot = +1 ; return a
        if ok[b]: dirc = _GetDir(position, target) − 1 ; rot = −1 ; return b
        a = (a + 1) & 7 ; b = (b − 1) & 7
else:
    for j = 0..7:
        c = (rot ≥ 1 ? a : b)
        if ok[c]:
            if _GetDis(position + D8[c], target) > dist: return c       ← still moving away: keeps the side
            dirc = _GetDir(position, target) − 1 ; rot = 0 ; return c      ← improved: starts over
        a = (a + 1) & 7 ; b = (b − 1) & 7
return −1
```

`_GetMyBestDir`, `_GetMyNextRandDirs` and `_GetMyInitialRandDir` (`1028:8d3a`, `8bea`, `8cde`) repeat parts of this logic and have no caller; neither does `_GetMyDis` (`1028:8682`).

### 5.4 Where the yellow ant can step — `_TileCanBeMovedOn` (`1020:9342`)

**Fact seen in the code.**

- Surface: inside the map and tile ≤ 0x53 in the yard (ground, objects, food, hole and pebbles; rocks 0x54+ and the caterpillar/pill bug are not) or ≤ 0x90 in the house.
- Nest: inside the map and: tile < 0x19 or pebble 0x30–0x31 (passable without digging); or, with `dig`, dirt 0x20–0x2E or closed surface 0x1C–0x1F (passable by digging). In rows y ≥ 2, or if the destination is another plane, this is enough. With the destination in this nest, rows 0 and 1 have restrictions:

| Row | Without `dig` | With `dig`, cell to dig | With `dig`, cell already open |
|---|---|---|---|
| y = 1 | only in the destination's column, or if the destination is not in row 0 | only in the destination's column | yes |
| y = 0 | only if it is exactly the destination | only in the destination's column and if the cell below (row 1) is not dirt | only if it is exactly the destination |

## 6. Nest: entering, exiting, switching nests

### 6.1 Entering (embedded in `_DoAntMoveY`; same as `_EnterNest` `1028:8408`)

**Fact seen in the code.** Nest music (`_TryAntTheme`); if it was dropping alarm, turns it off (`_SetAlarmDropState(0, 1)`); erases the mark; `_MePlane = 2` if `_MeLocX < 65`, else 3; `_MeLocX` ← the surface row (`_MeLocY`), `_MeLocY` ← 2 if queen (0x60), else 1; `_MeDir = 4`; `_DigMyTile` on the new cell; marks 0xFF.

**Quirk A1:** the limit is x < 65, while the ordinary ants (`_GoInNest`) and the hole creation (`_CreateNewHole`) use x < 64. A hole in column 64 belongs to the red nest, but the yellow ant that enters it goes to the black nest.

### 6.2 Exiting: `_ExitNest` (`1028:84a4`)

**Fact seen in the code.**

```
music ; erases the mark
row = _MeLocX & 63                         (nest column = surface row)
if _HoleMap(nest color)[row] == 0: _MakeNewHoleB/R(_MeLocX)     ← opens a new hole
x0 = _HoleMap[row] ; step = (queen ? 2 : 1)
base direction:
    destination on the surface: _GetDir((x0, row), destination) − 1 (0 if they coincide)
    destination in this same nest: 2 if x0 < 64, else 6
    other: _GetMyDir(1, (x0, row), destination) ; if < 0: S8()
for j = 0..7 (offsets _absSearchDirs `8:006c` = 0, +1, −1, +2, −2, +3, −3, +4):
    d = (base + offset) & 7 ; p = (x0, row) + step·D8[d]
    if _IsNotObstacle(1, p): position = p ; _MeDir = d ; stop
if none: _MeDir = base ; position = ((x0 + step) & 127, row)
_MePlane = 1 ; marks 0xFF
```

### 6.3 Passage between the nests (tile 0x14)

In a nest, outside row 0, if the yellow ant (non-queen) reaches the destination and the destination tile is 0x14, it passes to the **other** nest at the same cell (`_MePlane` 2 ↔ 3), with sound and centering; when entering the red one, it shows message 4 of SHARED 4/1010 ("You have entered the red nest", parameter 360). This confirms the [hypothesis] of [tiles-valores.md](tiles-valores.md) that 0x14 is the passage between the nests.

## 7. Picking up, dropping, digging, eating and health

### 7.1 `_TryMyDropOrLift(plane, x, y)` (`1020:98de`)

**Fact seen in the code.** Called when order 1 arrives next to the destination, or by command 8.

```
if carrying something (_MeType & 8):
    0x08 → _DropMyEgg ; 0x18/0x38 → _DropMyFood ; 0x28/0x48 → _DropMyRock ; other → fails
    returns 1 if dropped, −1 if not
else:
    ok = PICK_EGG (if _MeType == 0x10 or _MeHealth < 10)
    if not ok: ok = _PickupMyRock(plane, x, y)
    if not ok: ok = _PickupMyFood(plane, x, y)
    if ok:
        turns toward (x, y) ; if _MeFoodInType ≥ 0: _EatMyFood(_MeFoodInType) ; _MeFoodInType = −1
        returns 1
    if (_CommandKey (8:85e2) ≠ 0 or (winged queen 0x40 and _MeNestStarted = 0)) and surface and (x, y) == position:
        if _DigMyNewHole(x, y): _EnterNest() ; centers ; returns 1
        else returns −1
    returns 0
```

| Function | Address | Condition | Effect |
|---|---|---|---|
| PICK_EGG (embedded; same as `_PickupMyEgg` `1020:8e96`) | — | empty worker (0x10) or health < 10; egg (T & 0x7F from 1 to 7) at the cell (`_FindEggAt` `1020:88a2`: via the life map, or, if the cell has 0xFF/0xFE or nothing, via the list; returns the T of the slot, checked in the disassembly `1020:8a55`–`8a80`) | `_MeEggCarried = 0xFD`; removes the egg from the list; if health < 10: **eats the egg** (`_MeFoodInType = 3`); else `_MeEggCarried = T of the egg`, `_MeType = 8`, sound |
| `_PickupMyRock` | `1020:85ee` | empty 0x10 or 0x30; pebble (surface 0x51–0x53; nest 0x30–0x31) | surface: if the pebble was plugging a hole (the column/row is listed in `_HoleMap`), reopens it (`_MapB/R` row 0 = 0x18; surface = 0x50 in the yard, `S1(7) + 0x59` in the house); else the ground becomes `S16()` (yard) or 0 (house). Nest: 0x30 → `S8()` (tunnel); 0x31 (plug) → 0x18 and the surface hole goes back to 0x50. `_MeType` → 0x28 (worker) or 0x48 (soldier) |
| `_PickupMyFood` | `1020:8f4c` | food at the cell; if not hungry, only empty 0x10/0x30 | surface: `_PickupFoodA`; if it is going to carry: `_MeScent = 200`, `_MeDis = _GetDis(cell, _LastNewHoleB)`, `_MePrevDis = _MeDis + 1`, deposits 200 in the **black trail** (always `_JamScentBT`). Nest: 0x10 → `S8()`, else tile − 1; `_FoodB/R` −1. Hungry: `_MeFoodInType = 0` (eats on the spot); else sound and `_MeType += 8` |
| `_DropMyFood` | `1020:7576` | carrying food | looks for where to drop starting in the direction of the destination (`_GetDir − 1`, or `_MeDir`) with the `_absSearchDirs` offsets; a cell qualifies if it has no ant and is food with fewer than 3 units (+1), or free (`_IsClearTile`), or the center of an antlion (0x38); if no neighbor qualifies, tries its own cell. Surface: `_DropFoodA`; nest: writes the tile (0x10 on a free cell) and `_FoodB/R` +1. `_MeType &= ~8`, sound |
| `_DropMyRock` | `1020:7c52` | carrying dirt/pebble (0x28/0x48) | same search; a cell qualifies if it has no ant and is free, or is neither food nor pebble but is a hole or 0x38. `_DropPebble`: on the surface writes 0x51 and, if the cell is a hole, plugs the opening in the nest (row 0 = 0x31); in the nest writes 0x30, or 0x31 at an opening (and 0x51 in the surface hole). `_MeType −= 0x18`, sound |
| `_DropMyEgg` | `1020:8188` | carrying an egg (0x08) | same search (cell without ant and free, or 0x38); returns the egg to the plane's list with `T = _MeEggCarried`, `M = 8`, `S = 0`; `_MeType = 0x10`, `_MeEggCarried = 0xFD` |
| `_DigMyNewHole` | `1020:16ae` | winged queen without colony (or `_CommandKey`), at its own cell, 1 ≤ x < 128, 1 ≤ y < 64 | yard: requires a free 3 × 3; house: tile < 200. `_CreateNewHole` writes the hole (0x50 and border 0x18–0x1F; 0x59 in the house), updates `_HoleMap`, `_LastNewHole*` and `_LastNewExit*`, digs row 1 of the nest |

`_PickupMyObject` (`1020:9250`) and `_DropMyObject` (`1020:91ec`) group the same rules (only `_DropMyObject` is called, by `_YellowDeath`). `_IsLiftable` (`1020:97ca`) decides whether a double click in the nest becomes "pick up" (food, pebble or egg at the cell).

### 7.2 Eating: `_EatMyFood(type)` (`1020:8cda`)

**Fact seen in the code.** Messages from SHARED 4/1010 (`_WindPromptStrs`, `9:737a`).

| type | When | Effect |
|---|---|---|
| 0 | ate food it picked up (`_MeFoodInType = 0`) | sound; if `_MeHealth > 0` and `_BpopT > 1`: `_HealthB += _MeHealth / (_BpopT − 1)` (capped at 100); `_MeHealth = 100` |
| 1 | was fed by an ant (trophallaxis, `_DoTroph` and order 4) | dialog; message 11 "You feel much better now!"; `_MeHealth = 100` (no bonus to the colony) |
| 2 | [no caller seen] | message 12 "Thank you for your contribution"; if `_MeHealth ≥ 11`: `_MeHealth −= 10` |
| 3 | ate an egg (health < 10) | sounds; message 13 "Cannibalism has restored your health" (parameter 180); same effect as type 0 |

In all of them: `_MeHealthCntDwn = 0` and `_MeWantFood` recomputed as in `_SetMyHealth`.

**Trophallaxis** (`_DoTroph` `1000:846e`): when an ant of the same color tries to enter the yellow ant's cell with `_MeWantFood` on (modes 0, 1/4, 2 and 6, see [superficie.md](superficie.md), 2.3), the yellow ant turns toward it (`_MoveMyLife` on the same cell) and calls `_EatMyFood(1)`.

### 7.3 Alarm: `_SetAlarmDropState` (`1000:84cc`) and command 7

Turning it on is only possible on the surface and as an ant (`_MeMode = 0`); it turns on `_MeDropAlarm`, sets `_ELayerMode = 0` and selects the `editScent` button of the edit window. On each step on the surface, +50 alarm (maximum 200). Entering a nest, dying or restarting the yellow ant turns it off (`_MeDropAlarm = 0`, `_ELayerMode = −1`). Sound 0xF (ok) or 1 (error) [code: `_YellowCommandKey` case 7 `1000:8562`, `_SetAlarmDropState` `1000:84cc`, `_EnterNest`, `_YellowDeath`, `_InitYellow`].

**What you see** [code]: with `_ELayerMode = 0`, the edit window draws, on the surface, the alarm pheromone in place of the ground (`_DoCalcTile`: value > 0x10 → tiles 0xF1–0xFF, yellow to red dots; jogo.md, "Camada de cheiro na janela de edição"). Right after pressing 0 nothing changes, because there is no alarm yet: the dots appear behind the yellow ant when it walks. Foraging ants that step on a cell with alarm switch to Alarmed mode and climb the gradient; those in Defense also follow it ([feromonios.md](feromonios.md) §4). The alarm spreads and weakens every 64 ticks (`_SmoothAlarm`). **In the port** (2026-10-18): the 0 key already turned on the alarm and the deposit (+50 per step), but the edit window did not show the layer, and the user "did not notice anything different"; now `Estado::camada_edicao` (`_ELayerMode`) is turned on here and by the scent buttons of the ribbon, and the game screen draws it. **Checked on the test bench** (`qemu_alarme.txt`): `_ELayerMode` = 0 after the 0 key and the yellow trail in the edit window after walking (jogo.md, "Camada de cheiro na janela de edição").

## 8. Body swap and target: `_ExchangeLives`, `_DoLifeExchange`

### 8.1 Tool modes

**Fact seen in the code.** `_CurGameTool` (`9:7c22`): −1 none, 10 body swap, 11 target. `_StartLifeTransfer` (`1020:6744`) and `_TargetAnt` (`1020:66dc`) toggle the tool: when turning it on, they save `_GamePaused` in `_SaveGamePaused` (`9:9ea2`) and **pause the game**; when turning it off (or via `_EndLifeTransferMode`/`_EndTargetMode`), they restore the previous pause. Message 3 of 4/1010 ("Click on ant to exchange bodies") and message 17 ("Click on ant to chase or UnPause") accompany these modes [hypothesis: shown by the pause].

### 8.2 `_ExchangeLives(plane, x, y)` (`1020:67ac`)

Sets `_LessonTemp` (`9:9fb0`) = 1 and calls `_DoLifeExchange`. Success: turns off the tool (restores the pause); if the new body is the queen, special sound and music. Failure: error sound.

### 8.3 `_DoLifeExchange(plane, x, y)` (`1020:6842`)

**Fact seen in the code.** `v` = life map value at the cell (−1 if empty).

1. **Empty cell, on the surface, less than ~1.4 cells from the spider** (`_GetDis((x·16 + 8, y·16 + 8), (_SpidX, _SpidY)) < 512`): becomes the **spider**. If it is already the spider, nothing. Otherwise returns the current body to the list as an ordinary ant (2.1 below), erases the mark, `_Starg = −2`, spider destination = spider's cell, `_MeMode = 1`, `_SMode = _MeSMode = 0`, `_MeColor = 0`, `_ResetYellowVars(1, spider's cell)`, `_MeHealth = 100`, `_MeHealthCntDwn = 0`, `_MeWantFood` according to `_MeWarnHealth`.
2. **Clicked on itself** (0xFF/0xFE, or, being the spider, near the spider): returns 1 without doing anything.
3. **Queen:** if it clicked on the tail (group 13), the head is the cell `+ D8[dir]`; if on the head (group 12), the tail is `+ D8[dir ^ 4]`. Both are handled.
4. Looks for the slot in the plane's list (from back to front) with exact X, Y and T (and the tail's). Not found → 0.
5. Refuses (returns 0): red target; egg (group 0); groups 10, 11, 14, 15.
6. Target adjustments: carrying dirt (groups 5/9) → loses the dirt (`_UnCarryCaste`); carrying an egg (group 1) → `_MeEggCarried` = S of the slot (egg stage).
7. If it was an ant (`_MeMode = 0`): the old body becomes an ordinary ant at the current plane/position: `T = group' << 3 | target color | _MeDir`, with group' = group of `_MeType` (carrying dirt → without dirt; carrying egg → S = `_MeEggCarried`), `M = 9` if it was a queen (0x60) else 0, and the tail (`T + 8`) if queen. If the list is full (A ≥ 1000, B/R ≥ 500), returns 0 **after** having removed the target (quirk A2). If it was the spider, the spider just goes back to being controlled by the simulation.
8. The target's slots (and the tail's) are zeroed in the list.
9. New body: `_MeColor = target color`, `_MeBase = 0`, `_MeMode = _MeSMode = 0`, `_MeHealth = 100`, `_MeHealthCntDwn = 0`, `_MeWantFood` recomputed; marks 0xFF (and 0xFE on the tail); `_MeLoc` = head cell; `_MeDir = T & 7`; `_MeType = T & 0x78`; `_MePlane = plane`.

## 9. Death: `_YellowDeath` (`1000:7746`)

### 9.1 Causes

**Fact seen in the code; texts from SHARED 4 (decimal ids).** "Events" is the Options menu option that shows the dialog boxes (`_OptionStates[3]`, `8:85f8`). Cases 0, 1, 2, 5, 6, 7, 8 and 9 play a music `0x4E27 + S1(5)` (a random draw); 3 and 4 only an error sound; 10 no music.

| Code | Who calls | Message (if "Events") | Counter |
|---|---|---|---|
| 0 | `_YellowFight` (lost) | 10030 "Your brave attack…" if the yellow ant attacked (`_MeStartedFight`), else 10032 "The enemy ant has attacked you!…" | — (already counted in `_GetWinner`) |
| 1 | spider ate it | spider box (`_SpiderDialog`, with the `S1(2)` random draw of the text) and, after the fight, `_YellowDialog(9100)` | `_BAntsEaten` |
| 2 | antlion | antlion box (`_LionDialog`) and, after the fight, `_YellowDialog(9100)` | `_BAntsEaten` |
| 3 | spider-player surrounded | 10026 "You have been overcome by the massive onslaught of ants!…" | `_BAntsKilled` |
| 4 | spider-player left the map | 10028 "Due to a latent fear of being eaten by ants, you go insane…" | `_BAntsKilled` |
| 5 | stepped on (`_FootFall`) | 10034 "The inconsiderate human has stepped on you!…" | `_BAntsKilled` |
| 6 | lawnmower (`_MowerFall`, `_SimKidOutside`) | 10036 "The evil lawn mower…" | `_BAntsKilled` |
| 7 | drowned (health 0 in a flooded cell) | 10038 "You swim for your life…" | `_BAntsKilled` |
| 8 | starvation | 10040 "After going berserk with hunger…" | `_BAntsExpired` |
| 9 | insecticide (`_SimKidInside`) | 10042 "You are covered with insecticide!…" | `_BAntsKilled` |
| 10 | electric shock (tile 0x76/0x78 in the house) | 10045 "You have been electrocuted." | `_BAntsKilled` |

**Verified against the original** (tick vectors from 2026-10-18, [testes-tick.md](testes-tick.md#quinta-rodada-2026-10-18-regras-não-verificadas-da-revisão); all 307/307 with S, R and health, unless noted): 2 `amarela_morte_leao`, 3 `aranha_jogador_cercada`, 4 `aranha_jogador_fora`, 5 `amarela_morte_pisada`, 6 `amarela_morte_cortador`, 7 `amarela_morte_afogada`, 8 `amarela_morte_fome`, 9 `inseticida_morte` (2026-10-17), 10 `amarela_morte_choque` (house patch (1, 5) in the Full Game, outlet 0x76), 0 `amarela_luta` (2026-10-17). Code 1 (`amarela_morte_aranha`) is pending only because of the drawing random draws inside the tick (`S1(4)` of `_DrawPalps` three times and `S1(2)` of `_SpiderDialog`; see inimigos.md, "Verificado"). In all of them the rebirth as an egg (`_MeEgg` 1 → 7) or the transfer happens within the same tick. In death 2 the original centers the view twice in the tick: where the yellow ant died and on the new body (`_GotoMyAnt` twice; the port now carries the position in `Evento::CentralizarNaAmarela`).

Order: stops the simulation, ends tools 10/11, and, if it was carrying something, tries to drop it (`_DropMyObject` on its own cell); if it cannot, it loses the load. Animations that **consume random draws** and depend on the interface: 7, 8 and 9 → `_AnimYellowInsane`, **before** the music random draw (R generator, `R(7)` on reaching each target, and `S8()` when stuck; 32 frames with the edit window in front, 8 otherwise); 1, 2 and 10 → `_AnimYellowFight` (10.2). Afterwards (1 and 2) the "Your ant has been killed" box (4/1010 no. 23). In all cases: `_UnRecruit(1)` (releases all recruited ants) and turns off the alarm.

### 9.2 Reincarnation

**Fact seen in the code.** The search for a new body, in this order:

1. **Black egg in the black nest**: list B from index `_ListIndexB` (inclusive; quirk A3) down to 0, first T with 1 ≤ T ≤ 7. Removes the egg and is reborn as an egg (9.3, mode 0).
2. **Black adult in the black nest**: same scan, T from 8 to 0x67 (includes the queen's head, 0x60–0x67).
3. **Black adult on the surface**: list A, from `_ListIndexA` (inclusive) down to 0, T from 8 to 0x67.
4. **Black adult in the red nest**: list R, likewise.

In an adult (2–4): the slot is zeroed and the yellow ant takes the position with the **adult's type** (`T & 0xF8`) and the old direction; the mark written on the life map is the adult's own T, not 0xFF (quirk A4: corrected on the next tick by `_DoAntSimY`). `_MeMode = 0`, order zeroed (`_MeCrazyCnt = −2`), health 100, message 10059 "You have been transferred to an existing ant.".

No body: Tutorial and Quick → **defeat** (`_IsGameOver = 1`, `_BlackWon = 0`). Full Game → if there is more than one black colony (`_ColonyTotalBlack > 1`), message 10058 and `_SpecialXfer` (the player clicks on a black colony on the yard map); otherwise defeat with 10056.

### 9.3 Birth: `_YellowBirth(plane, x, y, type, queen)` (`1000:739e`)

**Fact seen in the code.** Stops the simulation during the animation (real time, not ticks).

- **Reborn as an egg** (`queen = 0`, called by reincarnation 9.2.1 with `type = _MeType`, and 0x10 if it was a queen): the yellow ant appears as an egg at the egg's cell (`_MeEgg` 1 → 7, one stage every ~10 ticks of the Mac clock), then becomes `type` facing 2 (if it was a queen) or 6. In practice always 6: `_YellowDeath` already replaced 0x60 with 0x10 before calling (fact seen in the code; see divergencias.md). Message 24 "You have been reborn!".
- **Founding the colony** (`queen = 1`, command 9): the yellow ant becomes a queen (0x60) at its own cell, an egg appears at `x, y` (2 cells behind) and goes through the stages; at the end, the queen becomes an ordinary simulation queen (`_MakeBlkQueen`/`_MakeRedQueen`, `_CastePop[5] += 1`) and the yellow ant is reborn as a **worker** (0x10) at the egg's cell, facing backward; `_CastePop[1] += 1`, `_CastePop[4] −= 1`, `_BpopT/_RpopT += 1`. Message 25 "You are now a worker!".
- In both: destination = position, order zeroed, `_MeNestStarted = 1`, health 100.
- **Stage timing** (fact seen in the code): each stage 2–7 is drawn (`_DoEditUpdateDraw`) and waits for `_MacTickCount` to reach "start + 10" (or an event, `_win_Events`); since the Mac clock advances 3 every 55 ms, that is 4 steps = 220 ms per stage (~1.3 s for all six). Before the stages, sound 0x1C and a `_myDelay(45)` (which returns right away, telas/mensagens.md §10); afterwards, sound 0x31, the final form drawn, the wait for the sound (`_mySoundIsDone`), the music and `_YellowDialog(9101)`. On the test bench (`morte_leao`, `morte_aranha`, telas/mensagens.md §11) the stretch between 9100 and 9101 lasted ~2.7 s.

### 9.4 Order within the tick and presentation in the port

**Fact seen in the code** (`_YellowFight` `1028:823e`, `_YellowDeath` `1000:7746`, `_YellowBirth` `1000:739e`, `_SpiderDialog`, `_LionDialog`, `_AnimYellowFight`, `_AnimYellowInsane`) **and checked in the sequential captures of the test bench** (`re_trabalho/cen/morte_leao`, `le_000`–`le_047`, and `cen/morte_aranha`, 2026-10-18; outside git): everything happens **within the tick**, one step after the other, with the program halted at each box and each animation. The view only leaves the place of death after the death boxes close.

| Step | What the original does | On screen |
|---|---|---|
| 1 (fight only, code 0) | `_YellowFight`: the enemy leaves the life map; if the yellow ant lost, `_GotoMyAnt`; `_AnimYellowFight(0)`: one drawing and 6 frames of 165 ms **at the place of the fight**; the enemy returns to the map and the yellow ant's corpse stays at its cell | the yellow and red cloud spinning at the place of the fight |
| 2 | `_YellowDeath(c)`: `_StopSimulation`, ends tools 10/11, drops the load; `_GotoMyAnt` (view at the place of death) and `_DoEditAndMapUpdateDraw` | the edit window at the place of death (in the fight, the corpse; in the others, the yellow ant still at its cell) |
| 3 (7, 8, 9) | `_AnimYellowInsane`: 32 frames of 110 ms (8 with the edit window behind) | the yellow ant running around and coming back |
| 4 | by cause: 0 music and 10030/10032; 1 `_SpiderDialog`, music, `_AnimYellowFight(1)`; 2 `_LionDialog`, music, `_AnimYellowFight(2)`; 3/4 error sound and 10026/10028; 5, 6, 7, 9 music and the box with an image; 8 music and 10040; 10 `_AnimYellowFight(10)` (32 frames) and 10045 (the boxes only with Events; §9.1) | `le_008`: the antlion box with the view on the surface |
| 5 (1, 2) | `_YellowDialog(9100)` | `le_010`: "Your ant has been killed" with the view **still on the surface** |
| 6 | `_UpdateEverything`; `_ClearMyLife` (the yellow ant leaves the life map, no redraw); `_UnRecruit(1)`; alarm off | — |
| 7 | search for the body (§9.2). Egg: `_YellowBirth` — marks the egg, `_GotoMyAnt` **on the new body**, drawing, sound, stages 2–7 (220 ms each), sound, final form, music 11005 and `_YellowDialog(9101)`. Adult: mark, `_GotoMyAnt`, drawing and 10059. None: defeat (Tutorial/Quick), or 10058 and `_SpecialXfer` / 10056 (Full Game) | `le_015`: the egg in the black nest; `le_018`: "You have been reborn!" with the view in the nest |

**Port (2026-10-19).** The rules run the whole tick, as before (the end-of-tick state and the vectors do not change), and leave the requests in the queue in the order above: `Evento::CentralizarNaAmarela { plano, x, y }` at each `_GotoMyAnt`, `Evento::Instantaneo { plano, quadro }` at each redraw of the edit window within the tick (`desenho::desenhar_no_tick`: copy of the ground map, the life map of the displayed plane and the `_Me*` fields, `desenho::foto_da_edicao`), `Evento::Animacao` at each animation (fight, death, shock, insanity and now the egg stages: code −2 on rebirth, −3 on founding the colony, `animacao.espera_ovo_ms` = 220), the music, the sounds and the boxes. The `Partida` (`core/partida.rs`) puts the requests on a **timeline** (`apresentacao`) and presents them one after the other (`seguir_apresentacao`): an animation or a modal notice (or the `_SpecialXfer` choice) holds back the rest until they finish; the sounds and waits that come right after an animation go along with it. While the presentation has not finished, no tick runs and the edit window shows the last snapshot (or the last frame of the animation; `Partida::quadro_animado`), not the end-of-tick state. Thus: the fight at its place, the death box with the view still there (the window frozen at the moment of death), and only afterwards does the view go to the new body, the egg grows and 9101 appears. This applies to all eleven causes (§9.1) and to trophallaxis (9110 with the yellow ant turned toward whoever feeds it). Mod scripts receive each request (`ao_evento`) when it is presented; snapshots do not go to scripts. Tests: `core::partida::testes::morte_na_luta_e_apresentada_na_ordem_do_original` (real rules, lost fight and egg), `regras_amarela.rs::linha_do_tempo_da_morte_e_do_renascimento`.

**Left out** [limitation]: the map window, the ribbon, the control windows and the creatures drawn over the map (spider) show the end-of-tick state during the presentation; the clock of the "Silly" balloons does not advance during the egg stages (the original's drawing reads `_TickCount` at each stage); the `_UpdateEverything` of step 6 has no snapshot of its own (the next frame is the new body's).

## 10. Fighting and colony commands

### 10.1 `_YellowFight(plane, i)` (`1028:823e`)

**Fact seen in the code.** Called when an enemy tries to enter the yellow ant's cell, or when order 3 completes.

```
erases the enemy from the life map
v = _GetWinner(_MeType, T[i])               ← the yellow ant is "a" (the one in the cell); R(10) ; _MeType has no color bit
if v ≠ _MeType: centers
_AnimYellowFight(_MePlane, _MeLoc, _MeDir, 0x70, 0)          (10.2)
if the yellow ant won:
    marks 0xFF ; surface: _DeadAntHere(enemy's cell, its color) ; T[i] = 0
    if it was the target of order 3: ends the order
else:
    returns the enemy to the life map ; surface: _DeadAntHere(yellow ant's cell, _MeColor)
    _YellowDeath(0)
```

Since `_MeType` has no color, with `_Invincible = 1` the yellow ant always wins (the `_GetWinner` rule makes the black one win). The casualty counters follow `_GetWinner` ([superficie.md](superficie.md), 6.2).

### 10.2 `_AnimYellowFight` (`1028:7e56`): an animation that consumes random draws

**Fact seen in the code.** Frames: 6 in the fight (code 0); 32 in the death by electricity (10); 8 in deaths 1/2 with the edit window behind; 32 (music off) or 64 (music on, `_OptionStates[1]`) with the window in front. Before the loop: `_SetMyLife(…, 0xFF)` and `_DoEditUpdateDraw` (the yellow ant still with its own type; in the fight the enemy has already left the life map). Each frame begins by waiting for the Mac clock to pass "start of previous frame + 6" (3 steps of 55 ms = 165 ms; the first frame only waits for the next step) and then: `S2()` and sound 0x25 if ≠ 0 (fight only); sound 0x31 and `S2()` that blinks the tile (shock only); with music on and code 1–9, **exits if `_mySongIsDone()`**; in the death by antlion, the center of the trap animates; **always** `S8()` (direction), `_MoveMyLife(plane, x, y, type, dir)` and `_DoEditUpdateDraw` (`_UpdateEdit` → `UPDATEWINDOW`: the drawing is synchronous, without going through the message loop). In the fight the type is 0x70: the 0xFF mark is drawn with tile 0x170 + direction from the life buffer, the "cloud" of tangled yellow and red ants. There is no message loop during the animation: **the whole program stays halted** (~0.8 s in the fight, ~5.3 s in the shock).

`_mySongIsDone` (`1008:a1a0`) always returns true on the Windows multimedia (MCI) path ([som.md](som.md)): with music on, deaths 1–9 exit on the first frame, after the sound and before the `S8()` — no frame drawn. Only on the Sound Blaster path would the 64 frames run.

**Verified against the original (QEMU bench, 2026-10-18, `cenarios_tick.py gravar luta_quadros`, snapshot `rico`):** soldier yellow ant placed next to a stationary red worker and a double click on it (order 3); at tick 1799 the fight happens and the screen was captured halted right after each `_DoEditUpdateDraw` (breakpoints at `1028:7e86`, `1028:8019` and `1028:7f1c`, see `tools/cenarios_tick.py`). There were **7 drawings**: the one before the loop (soldier yellow ant, `_MeType` 0x30, the red one already off the life map) and 6 frames with `_MeType` 0x70 and `_MeDir` 0, 2, 5, 6, 2, 0, at the same cell. The image is the yellow and red cloud, spinning on each frame. Captures in `re_trabalho/a51/luta_quadros/` (outside git, game images).

**Port.** `animar_luta` (`rules/amarela.rs`) saves, after each `_MoveMyLife`, the ground map, the plane's life map and the `_Me*` fields of the drawing (`QuadroAnimacao`) and requests `Evento::Animacao` (before the frame's waits and sounds). The interface (`core/partida.rs`) stops the simulation for `espera_ms × (quadros − 1)` ms (`bloqueio_ms`; no tick runs, the wall clock advances) and the edit window shows the current frame (`Partida::quadro_animado`, `TelaDeJogo::mostrar_quadro_animado`); a modal box requested in the same tick (the death after a lost fight) only appears afterwards. Between frames, `Evento::Espera(165)` keeps spreading out the sounds (regras/som.md). `_AnimYellowInsane` (§9.1, deaths 7–9) does the same with 110 ms per frame (`espera_loucura_ms`; until 2026-10-18 the port used 55, a wrong computation of "start + 3"). The drawing before the loop is not shown as an animation frame (it lasts up to 55 ms), but it remains as a snapshot (§9.4). Since 2026-10-19 the tick's requests are presented in order (§9.4): the death box waits for the fight to end and the view only goes to the new body after it. With music on, deaths 1–9 exit on the first frame (`animacao.morte_para_com_musica`, default true). Ticks requested by mods (`avancar_ticks`) do not stop. The animation only reads the state: the original's tick vectors (`ticks_originais`, `ticks_longos`) remain the same. Checked with the same input state (`estado_antes.bin` of tick 1799 loaded as a save; `simant --save … --ticks 1 --avancar-ms 0/200/400/600/800 --exportar-tela-jogo`): the port's frames show the same cloud at the same cell; the directions differ because the port's Load reseeds S.

**Quirk A5:** the number of random draws depends on the window in front and on the music option. For the port's determinism, reproduce the same rule (with the same interface state) or accept divergence from a death or fight of the yellow ant onward.

### 10.3 Commands: `_YellowCommand(c)` (`1000:87ba`)

**Fact seen in the code.** Ignored if `_MeMode > 1` (Experimental) or if the game is over.

| c | Effect | Music/sound |
|---|---|---|
| 0 | help (`WinHelp`; topic 1 ant, 2 spider, 3 winged queen) | — |
| 1 | `_Recruit(5)` | 0x2B05 |
| 2 | `_Recruit(10)` | 0x2B06 |
| 3 | `_Recruit(1000)` (all) | 0x2B07 |
| 4 | `_UnRecruit(0)` (releases half) | 0x2B08 |
| 5 | `_UnRecruit(1)` (releases all) | 0x2B09 |
| 6 | toggles body swap (`_StartLifeTransfer`) | — |
| 7 | toggles the alarm (7.3) | 0xF / 1 |
| 8 | on the surface: `_TryMyDropOrLift(1, position)` (picks up/drops at its own cell; winged queen without colony: digs the nest) | error 1 if it returns −1 or outside the surface |
| 9 | in the black nest, with y > 2: `_YellowBirth(2, position + 2·D8[_MeDir ^ 4], 0x10, 1)` (found) | error 1 otherwise |
| 10 | toggles target mode (`_TargetAnt`; used by the spider) | — |
| 11 | spider: toggles `_MeSMode` 7 (hunting by scent) / 0 | sound 2 |
| 12 | spider: toggles `_MeSMode` 8 (laser) / 0 | sound 0x29 |

`_Recruit`/`_UnRecruit` are in [superficie.md](superficie.md) (9.4).

## 11. Player controls

### 11.1 From Windows to the game event

**Fact seen in the code** (`MAINWNDPROC` → `_DoMouse` `1000:13f6` / `_DoKeyDown` `1000:0e8c` → `_DoEvent`). The event carries the position and a bit field:

| Source | Bits |
|---|---|
| left button pressed (`WM_LBUTTONDOWN`) | 0x200 |
| right button pressed (`WM_RBUTTONDOWN`) | 0x800 in the `_DoMouse` code, but `MAINWNDPROC` diverts 0x204 to help mode first (telas/ajuda-contexto.md §1) |
| left **double click** (`WM_LBUTTONDBLCLK`) | 0x2000 |
| right double click (`WM_RBUTTONDBLCLK`) | 0x4000 |
| Shift / Ctrl held | bit 0 / bit 1 |
| **Space** or **Insert** keys | like a left click at the cursor position (0x200); if the next message is already another Space/Insert, like a double click (0x2000) |
| **Delete** key | in the `_DoKeyDown` code, like a right click (0x800) or double (0x4000); but the branch is only reached with Space/Insert: Delete goes to `_HelpKeyDown` and toggles help mode (telas/ajuda-contexto.md §2) |
| arrows | move the cursor 8 pixels (`SetCursorPos`) |

Before reaching the game, `_HelpKeyDown` and `_YellowCommandKey` (11.4) take priority over the other keys. Letters A–Z feed a 4-character ring; if `_Cheats` (`8:83bc`) is enabled and the last 4 letters form a code from the table (stored with the bits inverted; read from `SIMANTW.EXE`: `oops rand erad FUND joke hole HOLE qeen QEEN eggs EGGS heal HEAL zhea ZHEA just will mick jeff joel fred jenn susi data serv clie`), it calls `_CheatKeys`. Enter clears the ring. Codes and effects: [trapacas.md](trapacas.md).

### 11.2 Edit window: `_ProcEditEvent` (`1010:0b7a`)

Objects of window 0 (names from SHARED 1/0, **fact verified** in the dump):

| Object | Name | Action |
|---|---|---|
| 4, 0x15 | `editTileArea`, `editTileArea2` | `_processEdit` (11.3) |
| 6 | `editHelpBut` | help |
| 7 | `editTools` | tools menu (`_EditToolsMenu`: Quick → switches to Experimental; Experimental → tools 11.6) |
| 8, 9, 0xA | `editTopBut`, `editBNestBut`, `editRNestBut` | shows the surface, the black nest or the red one (`_SetMapPlane(1/2/3)`) |
| 0xB | `editGoYellow` | centers on the yellow ant |
| 0xC, 0xD, 0xE | `editGoSpider`, `editGoBQueen`, `editGoRQueen` | centers on the spider / black queen / red queen |
| 0xF | `editPause` | pause/unpause |
| 0x10 | `editScent` | menu of the displayed scent layer (SHARED 1/6: Off, Alarm, BNest, BTrail, RNest, RTrail) |
| 0x11 | `editHealthY` | dragging sets `_MeWarnHealth` = height × 100 / total height |
| 0x12 | `editHealthB` | sets `_BlkWarnHealth` (`9:769c`) the same way |

Scrolling the view (`_goStepUp/Down/Left/Right/NE/SE/SW/NW`, `1030:6efe`–`754e`, and `_FinishDiagStep/HorzStep/VertStep`) only moves the window (`_MapPnt`, `9:7d2e`) and stores the center per plane; it does not touch the simulation.

### 11.3 Click on the map: `_processEdit` (`1000:6a38`)

**Fact seen in the code.** `double` = bits 0x6000 (double click, left or right). A single right click (0x800) is treated as a single click. Cell = `((px − left) / tile_width + _MapPnt.x, (py − top) / tile_height + _MapPnt.y)`. A single click with the edit window in front waits ~6 clock ticks before processing.

```
Experimental (_CurGameType = 3): _processExp (tool, 11.6) ; end
tool 10: _ExchangeLives(displayed plane, cell) ; end
tool 11: if surface and the player is the spider: _processSpider(cell, double) ; end
single click:
    on the yellow ant: if the button is still held (no Shift): yellow ant menu (_AntMenu, 11.5)
                else, if it clicked on the queen's tail (0xFE): nothing ; else centers
    on another cell with an ant: if the button is still held (no Shift): Examine window (_MagnifyMenu) ; end
    else: ORDER(cmd = 0)
double click:
    on a non-yellow, non-egg ant, while being an ant (_MeMode = 0):
        _MeTargIndex = slot ; _MeCmd = 4 (same color: be fed) or 3 (other color: attack)
        stores T, plane and position of the target ; destination = cell ; _MeMoveMe = 1
    else: ORDER(cmd = 1)
ORDER(cmd):
    surface view (displayed plane < 2):
        spider-player: _processSpider(cell, cmd)
        else: destination = (plane 1, cell) ; _MeCmd = cmd ; _MeMoveMe = 1 ; _MeCrazyCnt = −2
    nest view (ant only):
        cmd 1 → _MeCmd = 2 (dig) if the cell is diggable (dirt/closed surface) or is the closed
                row 0 with the yellow ant in a nest ; 1 (pick up/drop) if carrying something or the cell has
                food/pebble/egg ; 2 in the other cases
        cmd 0 → _MeCmd = 0
        destination = (displayed plane, cell), with x clamped to 1..62 in rows 0–1 ; _MeMoveMe = 1 ; _MeCrazyCnt = −2
```

`_processSpider(x, y, double)` (`1000:7198`): single click (outside target mode) → walks there (`_SuserX/Y`, `_SMode = 0`, no prey). Double click or target mode: if there is an ant at the cell, it becomes the prey (`_SMode = 2`); in target mode, `_MeSMode = 6` and the mode is ended. `_MeCmd = double` decides between walking and running.

### 11.4 Command keys: `_YellowCommandKey` (`1000:8562`)

**Fact seen in the disassembly** (jump table at `1000:85c8`). Windows virtual key codes:

| Key | Ordinary ant | Winged queen without colony | Spider |
|---|---|---|---|
| `1` or NumPad 1 | 1 recruit 5 | 8 dig nest | 10 target |
| `2` or NumPad 2 | 2 recruit 10 (with Ctrl: 3 all) | 9 found | 11 scent |
| `3` or NumPad 3 | 4 release half | same | same |
| `4` or NumPad 4 | 5 release all | same | same |
| `0` or NumPad 0 | 7 alarm | same | same |
| `X` or F9 | 6 body swap | same | same |
| `J` or NumPad `*`, with Tab held | — | — | 12 laser |
| Backspace | stops the order in progress (zeroes destination and `_MeMoveMe`); without an order, centers; with Shift, centers | same | spider: stops (destination = position, `_SMode = 0`) or centers |
| Home | centers (sound 0xF) | same | same |

Note: the Windows accelerator is applied first (11.7); that is why Shift+0..4 go to the Speed menu and Ctrl+0 to the View menu, and not to this table [hypothesis: precedence of `TranslateAccelerator` in the `WINMAIN` message loop, fact seen in the code].

### 11.5 Yellow ant menu windows

**Fact verified** in the object names of SHARED type 1 and in the tables `10:1ba6`/`10:1baa`/`10:1bb0` of `SIMANTW.EXE`. `_AntMenu` (`1010:6d24`) opens a proximity menu according to the body and translates the item into a command:

| Window | Items (object name) → command |
|---|---|
| 7 `yllwRecruitWin` (ant) | `yllwRecruit5` → 1, `yllwRecruit10` → 2 (with Shift: 3), `yllwReleaseHalf` → 4, `yllwReleaseAll` → 5, `yllwExchange` → 6, `yllwRecruitHelp` → 0 |
| 8 `yllwStartMenu` (winged queen without colony) | `yllwStartDig` → 8, `yllwStartBrood` → 9, `yllwStartHelp` → 0 |
| 32 `editSpiderMenu` (spider) | `spiderMenuChase` → 10, `spiderMenuHunt` → 11, `spiderMenuXchange` → 6, `spiderMenuHelp` → 0 |

### 11.6 Experimental mode: `_processExp` and `_DoTool` (`1010:7022`, `1010:727e`)

**Fact seen in the code.** The tool is `_CurExpTool` (`9:79e2`), chosen in the `editToolsMenu` menu (SHARED 1/9). With the button held, the tool is reapplied to each new cell under the cursor (and continuously for tools ≥ 5).

| `_CurExpTool` | Object | Effect |
|---|---|---|
| 0 | `editExpInfo` | Examine window of the ant at the cell |
| 1 | `editExpBarrier` | wall (surface only; `_DropWall`) |
| 2 | `editExpDig` | dig (`_ExpDig`) |
| 3 | `editExpAddAnt` | place ant (`_ExpAddAnt`) |
| 4 | `editExpAddFood` | food (with the submenu variant: 20 times at `S1(9) − 4` around, `_IncFoodHere`) |
| 5 | `editExpTrails` | scent (`_ExpIncSmell`, [feromonios.md](feromonios.md)) |
| 6 | `editExpInsecticide` | insecticide (`_ExpKillAnts`: 8 times; kills ants, spider and antlion) |
| 7 | `editExpStartGame` | leaves Experimental and starts the simulation with the yellow ant |

### 11.7 Menus and accelerators

**Fact verified:** menu texts from SHARED 6/0 and accelerator table from the `ACCELERATORTABLE` resource of `SIMANTW.EXE` (offset 500224, 5-byte entries). Commands (low byte of the id) handled by `_ProcMenu` (`1000:4fc2`).

| Menu | Item | Shortcut | Id |
|---|---|---|---|
| File | New / Load / Save / Save As / Quit | Ctrl+N / L / S / A / Q | 3 / 4 / 5 / 6 / 8 |
| Window | Edit, Map, Behavior, Caste, History, Info, Status | Ctrl+E, M, B, C, H, I, T | 0x11–0x17 |
| Window | Help | F1 | 0x18 |
| View | Yard, House, Strategic, Graphic | Ctrl+Y, 5, 6, 7 | 0x21–0x24 |
| View | Surface, Black Nest, Red Nest | Ctrl+8, 9, 0 | 0x26–0x28 |
| Options | AutoTrack, Music, Sound Effects, Events, Messages, Silly | Ctrl+G, J, K, —, —, — | 0x31–0x36 (toggle `_OptionStates[0..5]`, `8:85f2`) |
| Speed | Pause / Slow / Normal / Fast / Ultra | Shift+0 / 1 / 2 / 3 / 4 (also Shift+NumPad 0–4) | 0x41 / 0x43–0x46 |

The accelerator table has 13 more entries without a menu item (ids 0xA0–0xAC: Ctrl+F4, Ctrl+NumPad−, Ctrl+Space, Ctrl+F6, Ctrl+NumPad+, Ctrl+P, Ctrl+A, Ctrl+Z, Ctrl+arrows, Ctrl+U, Ctrl+V, Ctrl+F) [open: who handles them; Ctrl+A appears twice, the first one, Save As, wins]. The index of the "Silly" option = `_OptionStates[5]` confirms its use in [superficie.md](superficie.md) (balloons).

## 12. Quirks to reproduce

| Id | What | Where |
|---|---|---|
| A1 | the yellow ant enters the black nest up to column 64 (the other ants only up to 63) | 6.1 |
| A2 | body swap with the list full removes the target and fails | 8.3 |
| A3 | the reincarnation search starts at index `_ListIndex` (one slot past the end) | 9.2 |
| A4 | when reincarnating into an adult, the life map receives the adult's T instead of 0xFF until the next tick | 9.2 |
| A5 | random draws of the animations depend on the window in front and on the music | 10.2, 9.1 |
| A6 | `_PickupMyFood` always marks the **black** trail, even with a red yellow ant (the rest of the trail uses `_MeColor`) | 7.1 |
| A7 | `_GetWinner` with `_MeType` without color: the yellow ant is treated as black | 10.1 |
| A8 | the electric shock is only tested when an order ends | 4 |

## 13. Suggested test cases (QEMU bench)

Test bench: [docs/bancada-qemu.md](../../bancada-qemu.md) (commands `clicar_em`, `teclas`, `capturar`, save with Ctrl+A). Screen coordinates depend on the edit window being open at the default 640 × 480 position [to be measured once and recorded in the script].

| # | Rule | Script | Expected |
|---|---|---|---|
| Y1 | Health over time and per step (3, 4) | Quick Game; save while standing still at two moments; then click far away and save on arrival | standing still: −1 every 64 ticks (`_MeHealth`); walking: −1 every 2 steps (`_MeSteps`) |
| Y2 | Simple order (4) | single click on a free surface cell | `_MeMoveMe = 1`, `_MeCmd = 0`, `_MeCrazyCnt = −2` right after; on arrival, `_MeMoveMe = 0` and position = destination |
| Y3 | Avoidance (5.3) | click behind a large rock | the path goes around the rock (captures every second); `_MeCrazyCnt` between 16 and −2 during the avoidance |
| Y4 | Picking up and dropping food (7.1) | double click on food; then double click on ground near the hole | `_MeType` 0x10 → 0x18, `_MeScent = 200`; black trail 200, 199… along the path; on dropping, tile 0x48 and `_MeType = 0x10` |
| Y5 | Eating while hungry (7.2) | lower `editHealthY` to the current health; double click on food | `_MeHealth = 100`, `_HealthB` rises by `health / (_BpopT − 1)`; food tile −1 |
| Y6 | Trophallaxis (7.2) | while hungry, double click on a black worker | on contact, message "You feel much better now!", `_MeHealth = 100` |
| Y7 | Recruiting and releasing (10.3, 11.4) | keys 1, 2, Ctrl+2, 3, 4 | `_ModePopB[5]` (Defense) +5, +10, all; half; 0. **Checked (2026-10-19, `amarela_*` actions in `tests/referencias/acoes/`, 307/307):** each key with the game paused does exactly the `_Recruit`/`_UnRecruit` of superficie.md 9.4, including lists B and R |
| Y8 | Alarm (7.3) | key 0 on the surface and walk | `_MeDropAlarm = 1`; `_PherMapA` +50 per step (max. 200); entering the nest turns it off |
| Y9 | Entering/exiting the nest (6) | single click on the hole; then click on the surface while in the nest | `_MePlane` 1 → 2 with (x, y) = (y_hole, 1), dir 4; exit at a cell neighboring the hole in the direction of the destination |
| Y10 | Column 64 (A1) | Experimental: create a hole in column 64 and make the yellow ant enter it | `_MePlane = 2` (black), while ordinary ants at the same hole go to list R |
| Y11 | Fight (10.1) | double click on a red ant | `_MeCmd = 3`; on contact, fight; victory → red corpse; defeat → death code 0 with 10030 |
| Y12 | Body swap (8.3) | X key and click on a black soldier | `_MeType = 0x30`, the old body in the list as a worker (T group 2, M 0) at the old position |
| Y13 | Becoming the spider (8.3) | X key and click on the spider's cell | `_MeMode = 1`; `_MeLoc` follows `_SpidX >> 4` |
| Y14 | Death and reincarnation (9.2) | let the yellow ant starve to death | message 10040; the yellow ant is reborn as an egg if there is a black egg in the nest (`_MeEgg` 1→7) or transferred to an adult (10059) |
| Y15 | Founding a colony (9.3) | Full Game: winged queen, key 1 (dig) and then key 2 in the nest with y > 2 | new hole; `_MeType` 0x40 → yellow worker; ordinary queen in list B; `_MeNestStarted = 1` |
| Y16 | Keyboard as mouse (11.1) | position the cursor and press Space twice quickly | same effect as a double click |
| Y17 | Accelerators (11.7) | Shift+3, Ctrl+9, Ctrl+G | speed "fast", black nest view, AutoTrack toggled (`_GameSpeed`, `_MapPlane`, `_OptionStates[0]` in the save) |

## 14. Implementation in the port

**Where:** `engine/src/rules/amarela.rs`. Data: `data/amarela.json`, generated by `tools/extrair_amarela.py` from `SIMANTW.EXE` (`_Dx8/_Dy8` 8:0000/0008, `_Dx9/_Dy9` 8:0010/001a, `_absSearchDirs` 8:006c, `_UnCarryCaste` 8:0094, class and fight matrix 10:22c6/22d6, hole border 10:230c), plus the rule constants, the animation frames, the music and the messages (origin of each value in `_origem`).

- Tick: `impl RegrasAmarela for RegrasOriginais` calls `simular_amarela` (§3) and `mover_amarela` (§4) with `RegrasOriginais::amarela` (loaded from `data/amarela.json` with mods; `TabelasAmarela::padrao()` is the embedded copy, for tests).
- Used by other rules: `lutar_com_amarela(e, t, plano, i)` (`_YellowFight`; plane < 2 = list A, 2 = B, 3 = R; called by the surface and by both nests), `trofalaxia(e, t, x, y, d)` (`_DoTroph`; surface and black nest), `matar_amarela(e, t, CausaMorte)` (`_YellowDeath`, codes 0–10; yard: 5, 6, 9; antlion: 2; spider: 1, 3, 4) and `mover_marca` (`_MoveMyLife`, before the death by antlion and by spider).
- Player commands (no SDL; the interface converts pixel → cell, §11.3, and calls):

| Function | Original | Use |
|---|---|---|
| `clicar_no_mapa(e, t, ctl, plano_vista, x, y, duplo, segurando)` | `_processEdit` | full click: Experimental, tools 10/11, click on the yellow ant (center or menu), Examine, order. Returns `RespostaClique` |
| `ordem_ir(e, plano_vista, x, y)` | ORDER(0) | "go to (x, y)" (single click) |
| `ordem_agir(e, plano_vista, x, y)` | double click | attack or ask an ant for food (orders 3/4); otherwise go and pick up/drop/dig (1/2) |
| `comando(e, t, ctl, c)` | `_YellowCommand` | commands 0–12 (§10.3), also the menu items of §11.5; `false` = error sound |
| `tecla_de_comando(e, t, ctl, vk, Modificadores)` | `_YellowCommandKey` | Windows virtual keys (module `vk`): 1–4, 0, X/F9, J/NumPad* with Tab, Backspace, Home; returns `RespostaTecla` |
| `trocar_de_corpo(e, t, ctl, plano, x, y)` | `_ExchangeLives` | click with tool 10 |

`ControleJogador` holds what does not go into the save (`_SaveGamePaused`). Dialogs, wind messages (4/1010) and the view movement are not simulated: the music becomes `Evento::Musica`, the sound effects `Evento::Som` (ids in `sons` of `amarela.json`, see [som.md](som.md)) and `_TryAntTheme` `Evento::TemaDoNinho`, the boxes of §9.1 become `Evento::Mensagem`, and "centering" (`_GotoMyAnt`) writes `_LessonTemp = 1` and leaves `Evento::CentralizarNaAmarela` in the queue; what moves the view is the game loop (`core/partida.rs`, see [jogo.md](../telas/jogo.md), "Simulação ao vivo").

**Random draws and animations.** All random draws happen in the original's order, including those of the animations: `_AnimYellowFight` (S2 per frame in the fight or in the shock, S8 always; with the antlion the center tile oscillates 0x3B–0x3D and stays at the last value) and `_AnimYellowInsane` (R generator for the target, S8 when stuck; the yellow ant actually walks across the life map and comes back). The number of frames uses `animacao.janela_edicao_na_frente` (default: true) and `_OptionStates[1]` (music); with music on, deaths 1–9 exit on the first frame, as on the MCI path (§10.2). The drawn frames go to the interface in `Evento::Animacao` (§10.2). `_YellowBirth` is in real time and makes no random draws: the port applies the final state in the tick and shows the egg stages as a presentation animation (`Evento::Animacao` with code −2/−3, 220 ms per stage; §9.3 and §9.4).

**Port choices and gaps:** `_CommandKey` (8:85e2) is not in the state: treated as 0. The food drop in the house follows `_DROPdir` (`Estado::passo_queda`, superficie.md 8.2). `_SpecialXfer` (choosing another colony in the Full Game): since 2026-10-19 the swap happens within the tick, at the original's point, when the choice is known (`Estado::transferencia_no_tick`); without it, the tick emits message 10058 and `Evento::EscolherColonia`, and the interface runs the tick again with the choice (telas/quintal.md §8; `rules/troca_patch.rs`; action `special_xfer` in `tests/referencias/acoes/`, 307/307). When dying with tool 10/11 on, the tool is turned off but the saved pause is not restored (it is in the interface; by the pause rule the yellow ant does not die through the tick with these tools). The pieces shared with the nest and the surface (`_GetMap`, `_GetLife`, `_IsClearTile`, `_IsNotObstacle`, `_IsThisPebble`, `_IsItHole`, `_TileCanBeMovedOn` on the surface, `_GetBestDir`, `_FindAntIndex`, `_DigTileB/R`, `_SmoothEdges*`, `_FixExitMap*`, `_MakeNewHole*`, hole border, `_PlaceEgg*`, `_DecEat*`, `_PickupFoodA`, `_DropFoodA`, `_DeadAntHere`, `_GetWinner`, `_Recruit`, `_UnRecruit`, `_GetDir`, `_GetDis`) are in `engine/src/rules/comum.rs` (see [../../decisoes.md](../../decisoes.md), "Módulo comum das regras"); `amarela.rs` holds only what belongs to the yellow ant.

**Verified:**

- `engine/tests/regras_amarela.rs`: cases Y1–Y15 of §13 that do not need the interface (Y16/Y17 belong to the interface), the queen's egg laying, Backspace and the click on the yellow ant itself; unit tests in `amarela.rs` (`_TileCanBeMovedOn`, `_GetDir`).
- The original's tick vectors (`tests/referencias/ticks`, ticks 79–83, yellow ant standing still in the nest; [testes-tick.md](testes-tick.md)): `_Me*` fields and the 0xFF mark identical.
- Cases on the QEMU bench (2026-10-17), `tests/referencias/casos/Y-01..Y-04.json`, saves `tests/referencias/saves/Ynn_{ENTRADA,CARREGADO,FIM}.ANT`. In the original the yellow ant with an order walks during the load pause, so the port starts from the injected file (tick 78) and runs full ticks; the `_Me*` fields (position, type, direction, health, order, last step, steps, avoidance, scent trail) match the original at the `GameTime` of the state read right after the load and at the end:

| Case | Input | Original | Port |
|---|---|---|---|
| Y-01 walking in a straight line | (10, 20) → (50, 20) | tick 100: x = 32, health 88, 22 steps; tick 108: x = 40, health 84, 30 steps | same |
| Y-02 avoidance | wall 0x5C at x = 30, y 15–25; (25, 20) → (35, 20) | reached (35, 20) before tick 94, health 91 | same |
| Y-03 eating while hungry | (59, 36), order 1 on the food (62, 36) = 0x49; health 50, warning 60, `_HealthB` 40 | tick 81: (61, 36), health 100, `_HealthB` 88 (40 + 49, −1 from the queen's egg), tile 0x48 | same (the −1 from the egg comes from the nest rules, still empty in this branch) |
| Y-04 entering the nest | (8, 32) → black nest (31, 4) | tick 86: plane 2, (31, 4), health 95, order ended | same |

Also checked against the original via tick vectors ([testes-tick.md](testes-tick.md)): walking in the nest (`amarela_andando`), exiting the nest, picking up food and entering the red nest (`amarela_superficie`), fight with defeat, death and rebirth as an egg (`amarela_luta`), fight with victory (`amarela_vitoria`) and death by insecticide with no body, with game over (`inseticida_morte`).

Not verified against the original: body swap, recruiting, alarm, colony founding, the other causes of death and the key commands (only tests based on the spec).

## Gaps

- Effects of `_CommandKey` (the cheat codes, `_CheatKeys`, are in [trapacas.md](trapacas.md)).
- Who handles accelerators 0xA0–0xAC.
- The player-controlled mating flight (winged queen flying, `_FlyAwayB`): life cycle document. (`_SpecialXfer`/`_XferPatch` are in [geracao-mundo.md](geracao-mundo.md) and `rules/troca_patch.rs`.)
- Use of `_MeDis`, `_MePrevDis` and `_MeBase`.
- Drawing details of the Examine windows (`_MagnifyMenu`), recruiting windows and death dialogs (positions, bitmaps).
- Checked against the original: cases Y-01..Y-04 (walking, avoidance, eating, entering the nest; section 14), the tick vectors listed in "Verified" and, since 2026-10-18, the deaths of 9.1 in tick vectors (below) and the player's spider (Y13: `aranha_jogador`). The rest of section 13 (Y1, Y4–Y8, Y10–Y12, Y14–Y17 as interface scripts) remains to be run.
- Animations (§10.2): checked against the original are the fight frames (`luta_quadros`) and, via the state and the generators (tick vectors, 2026-10-18), the shock (`amarela_morte_choque`, 32 frames of `_AnimYellowFight`) and the insanity (`_AnimYellowInsane` in `amarela_morte_fome` and `amarela_morte_afogada`: R and S identical after the tick); the shock and insanity frames were not captured as images, and the deaths with the edit window behind were not recorded. The fight sound at the original's timing was not heard side by side with the original (the bench has no sound card). The drawing before the loop (up to 55 ms) is not shown. The captures of the original and of the port start from the same state, but the port's Load reseeds S, so the directions of the cloud differ.
