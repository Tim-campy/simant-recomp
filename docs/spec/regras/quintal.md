*Leia em português: [quintal.md](../../pt-BR/spec/regras/quintal.md)*

# Yard: boy, lawnmower, cat, dog, bird, rain, water and colonies

This document describes `_DoSimYard` (`1030:14a8`), called every tick (step 6 of [simulacao-visao-geral.md](simulacao-visao-geral.md) §5), with everything it calls, and `_DoWater` (`1020:07c6`, step 7). It covers the boy (inside and outside the house), the lawnmower, the foot and the insecticide, the cat, the dog, the bird, the rain, the water and the strategic colony map (`_SimColonies`).

## Evidence convention

- **Fact seen in the code:** Ghidra pseudocode (`re_trabalho/decomp/Code7.c`, `Code5.c`), with address `selector:offset` and name from `SIMANTW.SYM`. Where the pseudocode was ambiguous, the disassembly of `SIMANTW.EXE` (capstone) and the NE relocations of DGROUP decided; the text says so.
- **Verified against the original:** case run on the QEMU test bench ([../../bancada-qemu.md](../../bancada-qemu.md)), with the case file indicated.
- **[hypothesis]** / **[open]:** as in the [README](../README.md).

Random draws: `S(n)` = `_SRand1(n)`, `S2()`, `S4()`, `S8()`, `S16()`, `S32()`, `S64()`, `S128()` = masks, `SG(n)` = `_SGSRand(n)` (three steps), `R(n)` = `_RRand(n)` (R generator). See [rng.md](rng.md). **The order of the random draws is the order in which they appear in the pseudocode below**; a draw inside a condition only happens if the conditions to its left are true (short-circuit evaluation, as in C).

Coordinates: the yard has 12 × 16 patches, `(px, py)` with `px` 0–11 and `py` 0–15; index into the `_YMapPop*` maps = `px * 16 + py`. The current patch is `_CurYardPnt` = (`9:7a60`, `9:7a62`). The yard creatures have a position in pixels of the isometric yard view, `(X, Y)`, and the patch is derived from it:

```
px = (X + Y − 200) / 28        py = (Y − 38) / 10           (integer division truncated toward zero)
```

## 1. Variables

All in the save, except those marked. Addresses from the `.SYM`; port fields in `rules/estado.rs` (`Menino`, `Bichos`, `Ambiente`, `Colonia`).

| Symbol | Address | Use |
|---|---|---|
| `_YardCycle` | `9:7a5c` | yard counter, 0..0x3FF |
| `_BoyX/_BoyY`, `_BoyPx/_BoyPy` | `8:…`/`9:78b0`/`9:78d2` | position in pixels and patch of the boy |
| `_BoyDir` | `8:8a72` | direction 0 = py − 1, 1 = px + 1, 2 = py + 1, 3 = px − 1 |
| `_BoyHere` | — | state of the boy outside (0 inside the house, 1–5, section 3) |
| `_BoyTurnCnt`, `_BoyStandCnt`, `_BoyWait` | `8:8a74`, `8:8a80`, `8:8a70` | turn and stop counts; `_BoyWait` is only initialized |
| `_BoyFrame` | — | drawing frame |
| `_NodeNum`, `_NodeCnt` | `8:8a86`, `8:8a84` | node of the indoor route and step within the node; `_NodeNum` ≥ 0x26 = outside |
| `_BoyIsMowing` | `9:7c92` | mowing the grass |
| `_LastMowX/Y`, `_MowX/Y` | `9:9bbe`, `9:9bd4`, `9:9af4`, `9:9afc` | last patch mowed; position of the lawnmower in the current patch |
| `_GrassMap` | `9:a0b6` | 12 words: bit `py` of word `px` = grass still to be mowed |
| `_FootHere`, `_FootX/Y`, `_FootTog` | `8:8a7e`, `9:7a4e`, `9:7a58`, `8:8a82` | foot in the current patch, position in cells and left/right alternation |
| `_BoyMessOn`, `_BoyMsgOffset`, `_BoyMsgCnt` | `9:99da`, `9:72f2`, `9:7d4a` | speech balloon of the boy (interface) |
| `_Cat*`, `_Dog*`, `_Bird*` | `9:…` | cat, dog and bird |
| `_RainOn`, `_RainCnt`, `_ClearRainFlag`, `_RxTab/_RyTab`, `_WaterTable` | `9:…`, `9:810c`, `8:8a66`, `9:79e6`, `9:7a72`, `9:99d8` | rain and water |
| `_YMapPopB/R` | `8:00a4`, `8:0164` | population of each yard colony (0 = none; cap 250) |
| `_SwarmCntB/R`, `_QueenStorageB/R`, `_SwarmDelayB/R` | `9:807a`, `9:9c26`, `10:ac8c`, `10:ac8e`, `9:99e6`, `9:9b30` | mating flight: size, stored queens, delay |
| `_ColonyTotalBlack/Red`, `_HouseColonies`, `_ColonyUpdateFlag` | `10:ac90`, `10:ac92`, `9:9b22`, — | counts from the last pass |
| `_LastColonyPopB/R` | `9:72c0`, `9:8118` | **not in the save**: 1 after `_InitSimYard`; afterwards, `_ColonyTotal*` of the previous pass **of the Full Game** (only that branch writes it) [code, `_SimColonies`] |
| `_gameCycles` | `9:9b2c` | **not in the save**: turns of the timer loop (bird clock) |
| `_MigrateX/Y` | — | **not in the save**: origin of a migration (section 10.4) |

**Clocks.** Three routines use a wall clock, not ticks (fact seen in the code): the cat and the dog's bark use `_MacTickCount` (1/60 s); the bird uses `_gameCycles`; the boy's balloon stores `_MacTickCount + 300`.

**Where `_MacTickCount` comes from** (fact seen in the code and in the bytes of `SIMANTW.EXE`, 2026-10-17): it is not a variable, it is a function. `_MacTickCount` (`1000:9694`, code segment 1) calls `_TickCount` (`1008:18ac`), which does `GETTICKCOUNT()` (Windows USER: milliseconds since Windows started, with the 55 ms timer resolution) and divides by 55 (`__aFuldiv`, unsigned) — that is, it counts 18.2 Hz ticks —, and returns the result × 3 in DX:AX (32 bits; `mov cx,ax / mov bx,dx / shl ax,1 / rcl dx,1 / add ax,cx / adc dx,bx / retf`, RETF at `1000:96a5`). So **Mac clock = ⌊Windows ms / 55⌋ × 3 ≈ 54.5 per second, in steps of 3**: real time, independent of the tick and of `_gameCycles`. All clock reads inside the tick go through it (`_SimCat`, `_SimDog`/`_MakeBark`, `_SimKidOutside`/`_SimKidInside`/`_SendBoyMsg`, `_FollowBoyDir`, `_GetMowDir`, `_SimRain`, `_DoSimYard`, `_Feedback`, `_AnimYellowFight`/`_AnimYellowInsane`); `_TickCount` directly only appears outside the tick (sounds, dialogs, `_SeedSRand`/`_SeedRRand`). Measured on the QEMU test bench (normal speed): the clock advances 9 per tick (3333, 3621, 3909 at ticks 159, 191, 223; 18885 → 18966 at ticks 1888–1897), that is ~165 ms of VM time per tick under TCG. Consequence: **the random-draw consumption of the cat and the bark depends on real time**, and the original stops being deterministic tick by tick when the cat is off scene (`_CatOn = 0`) or the dog can bark — two runs from the `quick` snapshot diverged in S as early as tick 191. The tick vectors record the value read (`extras.json`, `MacTickCount`). The delays stored in the save (`_CatDelay`, `_DogBarkDelay`, `_BirdDelay`, `_BoyMsgCnt`) are absolute values of these clocks. With the cat and the dog, this makes the **random-draw consumption** depend on the clock (the cat's `S16` and the bark's `S(30)` only happen when the delay has expired). [Verified in [testes-tick.md](testes-tick.md)]: at normal speed, `_gameCycles` advances hundreds per tick, so the bird's test is true on every tick; only the value of `_BirdDelay` differs between runs.

## 2. `_DoSimYard` (`1030:14a8`), every tick

**Fact seen in the code.**

```
_YardCycle += 1; if > 0x3FF: _YardCycle = 0
if _NodeNum < 0x26: _SimKidInside()  else: _SimKidOutside()         (sections 4 and 3)
_SimBird()                                                          (section 7)
_SimCat()                                                           (section 6)
if _ForSaleState == 0: _SimDog()                                    (section 5)
if _TERRAINset ≠ 1:                                                 (rain; nothing in the house)
    if _RainOn == 0:
        if _GlobalKey == 0xA8 or (R(32) == 0 and R(128) == 0):      (R(128) only if R(32) gave 0)
            _RainOn = 1; _RainCnt = S(150) + 150
            _InitWater()                                            (100 × _PlaceDrop: section 9.2)
            boy balloon no. 0 (_BoyMsgCnt = clock + 300, _BoyMessOn = 1, _BoyMsgOffset = 0)
    else:
        if _RainCnt > 0: _RainCnt −= 1
        if _RainCnt < 1: _RainOn = 0
_SimColonies()                                                      (section 10)
```

`_SimRain` (`1030:15ea`) has the same code as the rain, but **has no caller**: `_DoSimYard` carries the body embedded. Also without a caller: `_SendBoyMsg` (`1030:15ae`, embedded as "boy balloon"), `_GetMowDir` (`1030:1f5c`), `_NotMowed` (`1030:203e`), `_IsValidYard` (`1030:2072`), `_InitGrassMap` (`1030:2096`), `_FollowBoyDir` (`1030:316e`), `_FollowCatDir` (`1030:32a6`), `_MakeBark` (`1030:32ea`), `_MowerFall` (`1030:34da`), `_MaintainSwarm` (`1030:3580`), `_GetNearbyPatches` (`1030:3ce4`), `_Reproduce` (`1030:3d4c`), `_StartMigrate` (`1030:3df2`) and `_EndMigrate` (`1030:3e6c`). All of them, except the two migration ones, appear embedded in the routines below, with the same code (checked in the pseudocode).

"Boy balloon no. k" (body of `_SendBoyMsg`): if k < 23 (in `_SendBoyMsg`; the embedded points already guarantee it), `_BoyMsgCnt = clock + 300`, `_BoyMessOn = 1`, `_BoyMsgOffset = k`. The text is entry k of the string list loaded by `_LoadStringAnt(1200)` in `_initStuff` (`9:9e88`), drawn by `_UpdateYardMessage` ([hypothesis]: SHARED list 4/1200). It does not affect the simulation.

## 3. Boy outside: `_SimKidOutside` (`1030:169c`)

**Fact seen in the code.** Tables in `data/quintal.json` (`menino`, section 12).

```
if _FootHere ≠ 0 and (_YardCycle & 3) ≠ 0: END             ← in the current patch, walks 1 tick in every 4

(choice of direction)
if _BoyIsMowing == 0:
    _BoyTurnCnt −= 1
    if _BoyTurnCnt < 0: _BoyDir = (S(3) + _BoyDir − 1) & 3; _BoyTurnCnt = 4
else if (_LastMowX, _LastMowY) ≠ (_BoyPx, _BoyPy):          (entered a new patch: embedded _GetMowDir)
    for k = 3, 2, 1, 0:
        (vx, vy) = (_BoyPx + neighbor_dx[k], _BoyPy + neighbor_dy[k])
        if (vx, vy) is in the yard and bit vy of _GrassMap[vx] is set:
            clear the bit; _BoyDir = k; go to STATE
    if _BoyPx ≥ 11: _BoyDir = 0
    else if _BoyPy ≥ 15: _BoyDir = 1
    else if _BoyPy < 1: balloon no. 6; _BoyHere = 4; _BoyDir = 2
    else: _BoyDir = _YardCycle & 3

STATE (by _BoyHere):
1 (strolling):
    if S(900) == 0: balloon no. 1; _BoyHere = 2                       (goes to get the lawnmower)
    else if S(600) == 0 or _CurGameType == 3 or _CurGameType == 0:   (S(600) always drawn here)
        balloon no. 2; _BoyHere = 5                                 (goes back home)
    else:
        if S(200) == 0: k = S(8); balloon no. 7 + k
        if _RainOn ≠ 0: _BoyHere = 5
2 (going to the lawnmower):  py < 12 → dir 2; py == 12: if px < 4 → TAKES THE LAWNMOWER, else dir 3; py > 12 → dir 0
    TAKES THE LAWNMOWER: _BoyHere = 3; (_LastMowX, _LastMowY) = (3, 13); _BoyIsMowing = 1;
                         _GrassMap[0..2] = 0, [3..11] = 0xFFFF; _BoyDir = 2; (_BoyX, _BoyY) = (0x7D, 0xAA)
3 (mowing):
    if S(100) == 0: k = S(3); balloon no. 3 + k
    if S(200) == 0: balloon no. 6; _BoyHere = 4
    (_LastMowX, _LastMowY) = (_BoyPx, _BoyPy); _BoyIsMowing = 1
4 (putting away the lawnmower): py < 12 → dir 2; py == 12: if px < 4 → _BoyHere = 5, _BoyIsMowing = 0, else dir 3;
                          py > 12 → dir 0. Then (_LastMowX, _LastMowY) = (_BoyPx, _BoyPy)
5 (going back home): py < 5 → dir 2; py > 5 → dir 0; py == 5: px > 3 → dir 3, else _BoyHere = 1
(0 or other: nothing)

(frame and step)
_BoyFrame = frame_by_dir[_BoyDir] + frame_by_step[_YardCycle & 3]
if _BoyIsMowing == 0:
    if _BoyStandCnt == 0:
        if S32() == 0: _BoyStandCnt = 20
        WALK
    else: _BoyStandCnt −= 1; _BoyFrame = _BoyDir + 20                 (standing: does not walk)
else: _BoyFrame += 100; WALK
WALK: _BoyX += step_x[_BoyDir]; _BoyY += step_y[_BoyDir]

a = _BoyY + _BoyX − 200; b = _BoyY − 38
_BoyPx = a / 28; _BoyPy = b / 10; clamp _BoyPy to 0..15 and _BoyPx to 0..11

(edges, only without the lawnmower)
if _BoyIsMowing == 0:
    if _BoyPy < 1: _BoyDir = 2   else if _BoyPy > 14: _BoyDir = 0
    if _BoyPx ≥ 11: _BoyDir = 3
    else if _BoyPy < 5: if _BoyPx ≤ 3: _BoyDir = 1
    else if _BoyPx ≤ 2:
        _BoyDir = 0
        if _BoyPy == 5:
            if _BoyHere ≠ 3: _NodeNum = 0x15                          ← enters the house (section 4)
            else: _BoyDir = 1

(foot and lawnmower)
_FootHere = 0; _FootTog = 1 − _FootTog (0 ↔ 1)
if (_BoyPx, _BoyPy) == _CurYardPnt:
    _FootHere = 1
    _FootX = ((a mod 28) × 4 + 6) & 0x7F;  _FootY = ((b mod 10 + 1) × 6) & 0x3F     (remainder with the sign of a, b)
    _MowX = _FootX + mow_dx[_BoyDir & 3]; _MowY = _FootY + mow_dy[_BoyDir & 3]
    if _BoyDir even: _FootX += (_FootTog ≠ 0 ? 6 : −6)  else: _FootY += (_FootTog ≠ 0 ? 6 : −6)
    _FootFall(_FootX, _FootY)                                       (section 3.1)
    if _BoyIsMowing ≠ 0 and _CurGameType ≠ 0: LAWNMOWER IN THE PATCH (body of _MowerFall, 3.2)
else if _BoyIsMowing ≠ 0:                                           (mowing another patch)
    for the black colony and then the red one of patch (_BoyPx, _BoyPy):
        v = _YMapPop?[i]; if v ≠ 0: v −= v >> 2; if v == 0: _?ColoniesKilled += 1; write v
```

Notes (fact seen in the code):

- The lawnmower removes a quarter of the population of each colony in the patch the boy is in, on each tick he is there (walking or not). Since `v − (v >> 2)` only gives 0 with v = 0, the counter `_BColoniesKilled/_RColoniesKilled` never rises through here.
- In Tutorial and Experimental the strolling boy goes back home on every tick (`_CurGameType` 0 and 3 force the "go back" branch); he only goes back and forth near the door.
- `_BoyWait` is only initialized (20) and never read in the simulation routines.

**What the player sees with the boy mowing the grass (test bench, 2026-10-18, `cenarios_tick.py gravar tela_menino`):** no message on the ribbon nor any box. In the **edit window** nothing of the boy is drawn (neither foot nor lawnmower: the edit drawing code does not read `_FootHere`): only the ants disappear (3/4 per pass, no corpse) and the footsteps leave corpses; on the test bench the black surface ants dropped from 35 to 12 and the red ones from 28 to 6 in ~2 s. The sound is the footstep (0x23) on each step in the current patch. In the **map window** (surface, no trails) the foot (1200–1203) and the lawnmower (1210/1211) appear (`_DrawMapFoot`, telas/mapa.md). In the **Yard View**: the boy pushing the lawnmower, the lawnmower sound 0x24 on each redraw and balloons no. 1 ("This yard needs mowing."), 3–5 and 6 ("I've had enough of this!"). The port does the same (the 0x24 sound since 2026-10-18).

### 3.1 Footstep: `_FootFall(x, y)` (`1030:3386`)

```
sound 0x23
if _BoyDir even: x1 = x + 6, y1 = y + 16   else: x1 = x + 16, y1 = y + 6
for cx = x .. x1 − 1, for cy = y .. y1 − 1:
    if 0 ≤ cx ≤ 127 and 0 ≤ cy < 128 and v = _LifeA[cx × 64 + cy] ≠ 0:   ← cy up to 127: reads the next column (defect)
        if v is the yellow ant (0xFF or 0xFE): _YellowDeath(5)
        else: j = _FindInAList(cx, cy); if j ≥ 0: _DeadAntHere(cx, cy, T[j] & 0x80); T[j] = 0
if x ≤ _SuserX < x1 and y ≤ _SuserY < y1: _KillSpider(); if _MeMode == 1: _YellowDeath(5)
```

The spider test uses `_SuserX/Y` (destination of the player-controlled spider), not the spider's position (fact seen in the code). With `cy ≥ 64` the index falls into column `cx + 1`: an ant there is not found by `_FindInAList(cx, cy)` (none has Y ≥ 64), but the yellow ant is killed if it is there.

### 3.2 Lawnmower in the current patch (body of `_MowerFall`, `1030:34da`)

```
for j = _ListIndexA − 1 .. 0:   if T[j] ≠ 0 and S4() ≠ 0: _LifeA(X[j], Y[j]) = 0; T[j] = 0      (no corpse, not counted)
if _SpidOn ≠ 0 and S4() ≠ 0: _KillSpider()
if _MeMode < 2 and _MePlane == 1 and S4() ≠ 0: _YellowDeath(6)
```

Each pass kills 3/4 of the surface ants of the whole patch, every 4 ticks (the boy in the current patch only walks on ticks with `_YardCycle & 3 == 0`).

**Deaths of the yellow ant by the foot and by the lawnmower, verified against the original** (tick vectors from 2026-10-18, `tools/cenarios_regras.py`, snapshot `rico`, 307/307 with S, R and life): `amarela_morte_pisada/1712` (the foot lands on (68, 12), where the yellow ant was placed after a rehearsal that showed the foot's spot; `_YellowDeath(5)` and, in the same tick, `_KillSpider` because `_SuserX/Y` was inside the rectangle) and `amarela_morte_cortador/1712` (the pass kills the yellow ant with `S4()` ≠ 0, `_YellowDeath(6)`, and the spider goes into its death throes). In both, the cat was taken out of play (huge `_CatDelay`): the Mac clock read after a modal box is not recorded by the test bench, and the `_CatDelay` written with it would not be reproducible.

## 4. Boy inside the house: `_SimKidInside` (`1030:20b6`)

**Fact seen in the code.** The boy inside the house follows a route of nodes. Nodes 0–15, 20 and 21 are walking segments (tables of frame, x and y per step); nodes 30–38 are stops (fixed frame, x, y, chance of leaving and next node). Tables in `data/quintal.json` (`casa`).

```
if _ForSaleState == 1: END                                          (house sold: there is no boy)
if _NodeNum < 30:
    _NodeCnt += 1
    if _NodeCnt > duration[_NodeNum]:
        _NodeCnt = 0
        k = S(3); _NodeNum = next[k][_NodeNum]
        balloon: node 0x21 → no. 20; node 6 → no. 21; node 7 → no. 22; node 14 → no. 15 + S(3); node 16 → no. 18 + S(2)
if 30 ≤ _NodeNum ≤ 38 (stop node):
    (frame, x, y) = fixed_nodes[_NodeNum]
    if S(chance[_NodeNum]) == 0: _NodeNum = fixed_next[_NodeNum]; _NodeCnt = 0
else if _NodeNum in routes: (frame, x, y) = route[_NodeNum][_NodeCnt]
else: (frame, x, y) undefined (local variables without a value; nodes 16–19 and 22–29 are not reachable through the tables)
if _NodeNum == 0x26: _BoyDir = 1
_BoyHere = (_NodeNum == 0x26 ? 1 : 0)
if _NodeNum == 20 and _NodeCnt > 2: _BoyHere = 1
if _NodeNum == 21 and _NodeCnt < 3: _BoyHere = 1                   (near the door: visible outside)
(_BoyX, _BoyY, _BoyFrame) = (x, y, frame)
(sound 0x18 on four steps of nodes 20 and 21, only with the yard view open; no random draw)
a = _BoyY + _BoyX − 200; b = _BoyY − 38; _BoyPx = a / 28; _BoyPy = b / 10      (not clamped)
_FootHere = 0; _FootTog = 1 − _FootTog
if (_BoyPx, _BoyPy) ≠ _CurYardPnt: END
if the index of the current patch is 0, 1, 16 or 32: END            (patches (0,0), (0,1), (1,0), (2,0))
_BoyDir = (0 ≤ _BoyFrame ≤ 11 ? dir_of_frame[_BoyFrame] : 2)
_FootHere = 1; foot as in 3 (same formulas; no _MowX/Y); _FootFall(_FootX, _FootY); sound 9
INSECTICIDE: repeat (_RpopT + _BpopT) × 8 times:
    x = S128(); y = S64()
    if _LifeA(x, y) ≠ 0: _Tindex = _FindInAList(x, y); if _Tindex ≥ 0: _DeadAntHere(x, y, T & 0x80); T = 0; _LifeA(x, y) = 0
    if (_SpidX >> 4, _SpidY >> 4) == (x, y): _KillSpider()
if _CurGameType == 2 and _MePlane == 1 and S4() == 0: _YellowDeath(9)
```

The route starts at node 0 (`_InitSimYard` clears `_NodeNum`). Node 20 always exits to 0x26 (38, "outside"); in the same tick node 38 draws `S(10)` and stays at 38; on the next tick, since `_NodeNum` ≥ 0x26, `_SimKidOutside` runs starting from (0xB0, 0x68) with `_BoyHere = 1`. The boy goes back inside when, while strolling, he reaches `px ≤ 2, py = 5` (section 3: `_NodeNum = 0x15`, node 21, which leads to 14). Nodes 36 and 37 have chance 0 (`S(0)` would divide by zero), but they are not reachable.

**Verified** in `QUICK2.ANT`: `_NodeNum = 3`, `_NodeCnt = 10` and `(_BoyX, _BoyY) = (126, 92)` = `roteiro[3][10]`; `_BoyPx/_BoyPy = 0/5` by the formulas.

## 5. Dog: `_SimDog` (`1030:2dbc`)

**Fact seen in the code.** Directions 0–3 as for the boy; 4 and 5 = sitting. Tables `cachorro`.

```
_DogTurnCnt −= 1
if _DogTurnCnt < 0:
    if _CatOn ≠ 0:   (_FollowCatDir) _DogPx < 5 → 1; _DogPx > 8 → 3; _DogPy > 0 → 0; else _YardCycle & 3
    else if _NodeNum < 0x26:  d = (S(3) + _DogDir − 1) & 3                  (boy at home: strolls)
    else (_FollowBoyDir):
        dy = _DogPy − _BoyPy; dx = _DogPx − _BoyPx
        if |dx| < 1 and |dy| < 1 and _BoyIsMowing ≠ 0 and _MapMode == 0 and _YardMode < 2 and clock > _DogBarkDelay:
            sound 0x15; _DogBarkDelay = clock + 60 + S(30)                 (barks at the lawnmower)
        if |dx| > 1 or |dy| > 1: dy < 0 → 2; dy > 1 → 0; dx < 0 → 1; dx > 1 → 3
        else d = _YardCycle & 3
    _DogDir = d; _DogTurnCnt = 4
if _DogDir == 1 and S(10) == 0: _DogDir = S2() + 4; _DogTurnCnt = 6         (sits for 7 ticks)
_DogFrame = _DogDir < 4 ? frame_by_step[_YardCycle & 3] + frame_by_dir[_DogDir]
                        : frame_by_dir[_DogDir] + (_YardCycle & 1)
_DogY += step_y[_DogDir]; _DogPy = (_DogY − 38) / 10
_DogX += step_x[_DogDir]; _DogPx = (_DogY + _DogX − 200) / 28
clamp _DogPy to 0..15 and _DogPx to 0..11
_DogPy < 1 → _DogDir = 2; _DogPy > 14 → _DogDir = 0
_DogPx ≥ 11 → _DogDir = 3; else _DogPx < 4 → _DogDir = 1
if 100 ≤ _DogFrame ≤ 103 and S(6) == 0:                                     (sitting: barks sometimes)
    k = S4()
    if _MapMode == 0 and _YardMode < 2 and clock > _DogBarkDelay:
        sound (0x17, 0x15, 0x17, 0x16)[k]; _DogBarkDelay = clock + 60 + S(30)
```

The dog does not kill ants. `_DogBarkDelay` is on the Mac clock (1/60 s).

## 6. Cat: `_SimCat` (`1030:2b24`)

**Fact seen in the code.** `_CatOn`: 0 absent, 1 walking, 2 stopped (licking itself [hypothesis]), 3 fleeing from the dog.

```
if _CatOn == 0:
    if clock ≤ _CatDelay: END
    _CatDelay = clock + 200
    if S16() ≠ 0 and _GlobalKey ≠ 0x8D: END
    _CatOn = 1; (_CatX, _CatY) = (0xFC, 0x19); _CatDir = 2; _CatFrame = 0; _CatCycle = 0; END
_CatCycle = (_CatCycle + 1) & 0xFFF
state 3: _CatFrame += 1; if _CatFrame ≥ 30: _CatFrame = 0; _CatOn = 0; _CatDelay = clock + 600
state 2: _CatFrame += 1; if _CatFrame > 12: _CatFrame = 11; _CatCycle += 1
          if _CatCycle ≥ 31: _CatCycle = 0; _CatOn = 1; _CatFrame = (_CatDir == 2 ? 0 : 3)
state 1:
    (nx, ny) = (_CatX + 4 × Dx8[_CatDir], _CatY + 4 × Dy8[_CatDir])
    if S16() == 0 or nx < 0xFC or nx > 0x1EF:                    (turns around)
        _CatDir == 2 → _CatDir = 6, _CatFrame = 4;  else _CatDir = 2, _CatFrame = 1
    else: (_CatX, _CatY) = (nx, ny); _CatFrame += 1; back to 1 (dir 2) or 4 (dir 6) after 2 or 5
    if _MapMode == 0 and _YardMode < 2 and S64() == 0: sound 0xD; END
    if S16() == 0 and _CatCycle > 100 and _DogDir == 2 and _BoyHere == 0: _CatOn = 2; _CatCycle = 0; _CatFrame = 10; END
    if _CatCycle > 50 and _GetDis(cat, dog) < 2401: _CatOn = 3; _CatFrame = 20; (sound 0xE if the yard view is open)
```

The cat only walks horizontally (directions 2 and 6) and does not kill ants. `_GetDis` = dx² + dy² in 32 bits.

## 7. Bird: `_SimBird` (`1030:28aa`)

**Fact seen in the code.** `_BirdOn`: 0 absent, 1 flying to the swarm, 2 leaving.

```
if _BirdOn == 0:
    if _gameCycles > _BirdDelay:
        _BirdDelay = _gameCycles + 20
        if _CurYardPnt.x > 4 and S2() == 0 and (_SwarmCntB > 0 or _SwarmCntR > 0):
            _BirdGoalX = x × 28 − y × 10 + 178; _BirdGoalY = y × 10 + 46      ((x, y) = _CurYardPnt)
            _BirdOn = 1; _BirdX = 0; _BirdY = S64() + 4; _BirdFrame = 0
    END
_BirdX += (_BirdOn == 1 ? 16 : 8)
if _BirdOn == 1: d = _BirdGoalY − _BirdY; if d ≠ 0: _BirdY += sign(d) × (|d| < 4 ? 1 : 4)
else: _BirdY −= 8
_BirdFrame = (_BirdFrame + 1) & 1
if 0 ≤ _BirdX < 512 and 0 ≤ _BirdY < 256:
    if _BirdOn == 1 and _BirdX ≥ _BirdGoalX and |_BirdY − _BirdGoalY| ≤ 4:      (arrived: eats the swarm)
        for v in _SwarmCntB, _QueenStorageB, _SwarmCntR, _QueenStorageR: if v > 0: v −= (v + 7) >> 3
        if _MapMode == 0 and _YardMode < 2 and S2() == 0: sound 7
        _BirdOn = 2
else: _BirdOn = 0; _BirdDelay = _gameCycles + 30
```

The `S2()` of the appearance is only drawn with `_CurYardPnt.x > 4`; with the initial patch (11, 8) it is drawn on every tick in which the delay has expired.

## 8. `_InitSimYard` (`1030:1378`)

Called by `_RandYard` (new game). **Fact seen in the code.** `_BoyMsgCnt = 0`, `(_BoyX, _BoyY) = (0xB4, 0x49)`, `_BoyTurnCnt = 12`, `_BoyWait = 20`, `_BirdDelay = 0`, `_CatDelay = 0`, `(_DogX, _DogY) = (0xFA, 0x96)`, `_BoyDir = 2`, `_DogTurnCnt = 2`, `_BirdFrame = 1`, `_LastColonyPopB = _LastColonyPopR = 1`, `_BoyMsgOffset = −1`, `_YardCycle = −1`; clears `_BoyMessOn`, `_BoyPx/Py`, `_BoyFrame`, `_BoyHere`, `_BoyStandCnt`, `_NodeCnt`, `_NodeNum`, `_BirdOn`, `_CatCycle`, `_CatFrame`, `_CatOn`, `_DogFrame`, `_DogDir`, `_FootHere`, `_FootTog`, `_FootX/Y`, `_MowX/Y`, `_BoyIsMowing`, `_RainOn`, `_SwarmDelayB/R`. It does not touch `_GrassMap` (only the boy does, when taking the lawnmower).

## 9. Rain and water

### 9.1 `_DoWater` (`1020:07c6`), every tick

Specified in [ninho.md](ninho.md) §8.1–8.3 (checked again in the pseudocode for this document). Summary of the order of the random draws: with rain in the yard, `S(50)` (sound), `S(10)` (water rises), then the 100 drops (`S(14)` only for a drop on 0x77 and `_PlaceDrop` with `R(128)`, `R(64)` for each drop that is not in a growing puddle). Dry in the yard: `S(14)` for each drop in a puddle (only on the first tick after the rain) and `S(10)` (only if `_WaterTable` < 64), followed by `S(8)` for each flooded cell (0x4E) that turns back into tunnel, column by column, black before red.

### 9.2 `_PlaceDrop(i)` (`1020:0acc`) and `_InitWater` (`1020:0b76`)

```
x = R(128); y = R(64); _RxTab[i] = x; _RyTab[i] = y
if _MapA(x, y) < 0x0E:
    _MapA(x, y) = 0x74
    cell c = (x >> 1, y >> 1) of the scent maps:
        _PherMapA = 0; _PherMapF = 0; _PherMapBN −= 20 (minimum 0); _PherMapBT = 0; _PherMapRN −= 20 (minimum 0); _PherMapRT = 0
```

`_InitWater` calls `_PlaceDrop(i)` for i = 0..99, in order.

**What the player sees (test bench, 2026-10-18, `cenarios_tick.py gravar tela_chuva`):** the rain **has no message of its own** (neither on the ribbon nor a box): the ribbon keeps showing the usual advice. The signs are: the blue puddles on the surface (edit window and map); the rain drawn in the Yard View; boy balloon no. 0 ("Looks like a bit of rain!", list 4/1200) when he is in view in the Yard/House View; effect 0x29 (3.2 s) with a 1/50 chance per tick; and, in the nests, the water rising from below (ninho.md §8.4). The port does the same.

**Verified against the original (case Q-01, `tests/referencias/casos/Q-01.json`, 2026-10-17):** starting from `QUICK2.ANT` with `_RainOn = 1`, `_RainCnt = 600`, `_ClearRainFlag = 0`, the game stopped at tick 140 has `_RainCnt = 538` (exactly 1 per tick), `_RainOn = 1`, `_ClearRainFlag = 1` and `_WaterTable = 61` (three rows flooded in 62 ticks). Final state: `tests/referencias/saves/Q01_T140.ANT`.

## 10. Yard colonies: `_SimColonies` (`1030:35f8`)

**Fact seen in the code; disassembly checked at the marked points.** It only acts when `_YardCycle & 0x1F == 0` (once every 32 ticks). It runs in all game types; only the final part (messages, house, victory and defeat) belongs to the Full Game.

```
_ColonyUpdateFlag = 1
SWARM (body of _MaintainSwarm), for the black color and then the red:
    if _SwarmCnt > 0: _SwarmCnt −= (_SwarmCnt < 4 ? 1 : _SwarmCnt >> 2)
    _SwarmCnt = max(_SwarmCnt, _QueenStorage); _SwarmCnt = min(_SwarmCnt, 50)
_ColonyTotalBlack = (_BpopT > 0 ? 1 : 0); _ColonyTotalRed = (_RpopT > 0 ? 1 : 0); _HouseColonies = 0
_YMapPopB[current] = min(_BpopT & 0x3FF, 250); _YMapPopR[current] = min(_RpopT & 0x3FF, 250)
for px = 0..11, for py = 0..15, except the current patch:        (the map is changed in place, in this order)
    b = _YMapPopB[px, py]; r = _YMapPopR[px, py]; if b == 0 and r == 0: next
    if _CurGameType == 2 and (px < 2 or (px == 3 and py < 5)): _HouseColonies += 1
    p = SCORE(px, py)                                              (embedded _GetNearbyPatches, 10.1)
    if b ≠ 0:
        _ColonyTotalBlack += 1
        n = (p == 0 ? b + 1 : b + p)
        n < 1 → _BColoniesKilled += 1; _YMapPopB = 0
        n < 250 → _YMapPopB = n
        else → _YMapPopB = 250; if S(10) == 0: REPRODUCE black from (px, py)   (10.2)
    if r ≠ 0:
        _ColonyTotalRed += 1
        n = (p == 0 ? r + 1 : r − p)
        n < 1 → _RColoniesKilled += 1; _YMapPopR = 0
                if px == 11: _YMapPopR[2 + S(6), 0] = 20; and py becomes 8 (the loop continues at py = 9)   ← disassembly
        n < 250 → _YMapPopR = n
        else → _YMapPopR = 250; if S(10) == 0: REPRODUCE red
STORED MATING FLIGHT (black only with _LayDownQueenMode == 1; red always):
    _SwarmDelay += 1
    if S8() + 10 < _SwarmDelay:
        _SwarmDelay = 0
        while _QueenStorage > 0: REPRODUCE from the current patch; _QueenStorage −= 1
ONLY IN THE FULL GAME (_CurGameType == 2):
    if _ColonyTotalRed == 0 and _LastColonyPopR ≠ 0: music 0x4E22; message 10052; if _ForSaleState == 0: message 10055
    _LastColonyPopB = _ColonyTotalBlack; _LastColonyPopR = _ColonyTotalRed
    if _ForSaleState == 0 and _HouseColonies > 24:
        _ForSaleState = 1; _NodeNum = 0; _BoyHere = 0; _BoyFrame = −1; music 0x4E23; message 10054
        if _ColonyTotalRed ≠ 0: message 10053
    if _ForSaleState ≠ 0 and _ColonyTotalRed == 0:
        music 0x4E25; message 10057; _IsGameOver = 1; _BlackWon = 1; (_DrawSimPayoff); END
    if _ColonyTotalBlack < 2 and _CastePopB[5] == 0 and _CastePopB[0] == 0 and _CastePopB[3] == 0 and _CastePopB[4] == 0:
        _IsGameOver = 1; _BlackWon = 0; message 10056
```

Points checked in the disassembly (`1030:37c9`–`3816`, `392d`, `39fa`):

- The score reads the neighbor offsets **unsigned** (`mov al, [bx+0x25dc]; sub ah, ah` and `and si, 0xff`). The table has dx = (0, 1, 0, 0xFF, 0, 0) and dy = (0xFF, 0, 1, 0, 0, 0): the neighbors (x, y − 1) and (x − 1, y) become (x, y + 255) and (x + 255, y), outside the yard, and do not count; the last two (0, 0) pairs count **the patch itself twice**.
- With the red colony extinct at `px == 11`, the code writes `di = 8` (`di` is the loop's `py`) before drawing `S(6)`. The loop continues at `py = 9`: with `old_py < 8`, patches (11, old_py + 1 .. 8) are skipped in that pass; with `old_py > 9`, patches (11, 9 .. old_py) are processed again ((11, old_py) itself is already empty of reds). Fact seen in the disassembly; reading [hypothesis]: a defect, the intent would be to test patch (11, 8), the initial red nest.
- The stored black mating flight depends on `_LayDownQueenMode` (`8:836e`; the selector comes from `DGROUP:0xc56e`, which the NE relocation binds to segment 8). In `QUICK2.ANT` it is 1.
- `_BColoniesStarted` (`9:80d4`) and `_RColoniesStarted` (`9:9c80`) rise when REPRODUCE lands on an empty patch; `_BColoniesKilled` (`9:a0dc`) and `_RColoniesKilled` (`9:7896`) when a colony disappears.
- The `_HouseColonies` area (px < 2, or px = 3 with py < 5) differs from the "house" area of the scoring ([simulacao-visao-geral.md](simulacao-visao-geral.md) §8, [open] there).

### 10.1 SCORE (`_GetNearbyPatches`, `1030:3ce4`)

```
p = 0
for k = 0..5: (vx, vy) = (px + neighbor_dx[k], py + neighbor_dy[k])   (unsigned offsets)
    if 0 ≤ vx < 12 and 0 ≤ vy < 16: if _YMapPopB[vx, vy] ≠ 0: p += 3; if _YMapPopR[vx, vy] ≠ 0: p −= 3
```

With the executable's table: p = 6 × (black here) − 6 × (red here) + 3 × (black at (px + 1, py)) − 3 × (red there) + 3 × (black at (px, py + 1)) − 3 × (red there). Black colonies grow with p (`b + p`) and red ones with −p (`r − p`); with p = 0 both grow by 1.

### 10.2 REPRODUCE (`_Reproduce`, `1030:3d4c`)

```
x = SG(4) + px; y = SG(4) + py; clamp x to 0..11 and y to 0..15      (x drawn before y)
if (x, y) ≠ (px, py):
    if _YMapPop?[x, y] == 0: _?ColoniesStarted += 1
    _YMapPop?[x, y] += 1                                              (byte: 255 + 1 = 0)
```

### 10.3 Mating flight

**What the player sees (test bench, 2026-10-18, `tela_revoada` and `tela_mate`):** no message when the mating flight happens. The winged ants in pre-flight (M = 14/15) stay near the hole and, on taking off (M = 16), disappear from the surface. In the **Yard View** the swarm appears as little black and red dots around the current patch (`_DrawSwarm`, telas/quintal.md §2.3), and the bird comes to eat it (sound 7). Only the **MATE** button (Full Game) writes on the ribbon and in the yard footer: "You have started the mating cycle", or the refusals "Produce males and new queens first" / "Fertilize queens in Full Game only" (telas/quintal.md §3). In the port the swarm and the MATE button exist since 2026-10-18.

`_SwarmCnt*` rises when M = 16 ants take off ([superficie.md](superficie.md)) and `_QueenStorage*` with a 1/16 chance on each takeoff in the Full Game. The bird eats 1/8 (rounded up) of the swarm and of the stored queens (section 7).

### 10.4 Migration: `_StartMigrate(X, Y)` / `_EndMigrate(X, Y)` (`1030:3df2`, `1030:3e6c`)

No caller in the pseudocode [open: remnant of a tool for dragging colonies]. `(X, Y)` in pixels of the yard view, with a different origin: `mx = (X + Y − 238) / 28`, `my = (Y − 66) / 10`.

- `_StartMigrate`: stores `(_MigrateX, _MigrateY) = (mx, my)`; if outside the yard **or** without a black colony there, `_MigrateX = −1`.
- `_EndMigrate`: if `_MigrateX ≥ 0` and the destination is in the yard: `h = _YMapPopB[origin] >> 1`; the origin loses h and the destination gains h, capped at 250.

**Verified against the original (case Q-02, `tests/referencias/casos/Q-02.json`, 2026-10-17):** colonies placed by hand in `QUICK2.ANT` and one pass at tick 97. All 16 checks matched: (2,2) black 10 → 16; (5,5) black 10 with (6,5) black → 19 and (6,5) 20 → 26 (the neighbor (5,5) does not count: with signed offsets it would give 29); (8,10) red 10 → 16; (9,12) black 5 and red 5 → 6 and 6; (1,13) black 3 with red at (1,14) → 6, and the red 10 → 16; `_ColonyTotalBlack` = 6, `_ColonyTotalRed` = 4; `_SwarmCntB` 40 → 30, `_SwarmCntR` 3 → 2, `_SwarmDelayB/R` 3 → 4. State: `tests/referencias/saves/Q02_T100.ANT`.

## 11. Helper routines used here

- `_DeadAntHere(x, y, color)` ([superficie.md](superficie.md) 6.4), `_FindInAList` ([formigas.md](formigas.md)), `_KillSpider` ([inimigos.md](inimigos.md) 2.8), `_YellowDeath(5, 6, 9)` ([formiga-amarela.md](formiga-amarela.md) §9), `_DrownBList/_DrownRList` ([ninho.md](ninho.md) 8.2).

## 12. Executable tables

Extracted by `tools/extrair_quintal.py` to `data/quintal.json` (addresses at the top of the script). Summary:

| Table | Address | Values |
|---|---|---|
| `_BxTab`/`_ByTab` (boy's step) | `1038:8a76`/`8a7a` | x (4, 6, −4, −6), y (−4, 0, 4, 0) |
| boy's frame by direction / by phase | `1048:25c0`/`25bc` | (0, 3, 6, 9) / (0, 1, 2, 1) |
| lawnmower relative to the foot | `1048:25c4`/`25cc` | dx (−10, 25, −10, −33), dy (−33, −10, 25, −10) |
| neighbor by direction (mowing) | `1048:25d4`/`25d8` | dx (0, 1, 0, −1), dy (−1, 0, 1, 0) |
| foot direction by frame | `1048:25b0` | 0,0,0,1,1,1,2,2,2,3,3,3 |
| dog: step, frames | `1048:236a`/`2370`/`2376`/`237a` | dx (3, 4, −3, −4, 0, 0), dy (−3, 0, 3, 0, 0, 0), phase (0, 1, 2, 1), base (0, 3, 6, 9, 100, 102) |
| house route | `1048:2380`–`25ae` | duration of 22 nodes, 3 next-node tables, steps of nodes 0–15, 20, 21, stops 30–38 |
| colony neighbors | `1048:25dc`/`25e2` | dx (0, 1, 0, 255, 0, 0), dy (255, 0, 1, 0, 0, 0), read unsigned |

## 13. Test cases

Port unit tests (`engine/tests/regras_quintal.rs` and tests in `rules/quintal.rs`), based on this specification:

| Case | Input | Expected |
|---|---|---|
| Q-U1 score | lone black; black with a neighbor to the right; neighbor to the left | +6; +9; +6 (left does not count) |
| Q-U2 colony pass | the same patch as case Q-02 on `QUICK2.ANT` | equal to the original's state `Q02_T100.ANT` in both maps |
| Q-U3 reproduction at the cap | patch with 250 and S(10) = 0 | a neighbor within ±3 gains 1 and counts as founded if it was empty |
| Q-U4 red extinction at px = 11 | red 1 surrounded by black at (11, 3) | (11, 3) = 0, `_YMapPopR[2 + S(6), 0]` = 20 and the loop goes back to py = 9 |
| Q-U5 lawnmower elsewhere | boy mowing in patch (5, 5) with black 100 | 75 |
| Q-U6 house route | `_NodeNum = 3`, `_NodeCnt = 9` | (126, 92) at step 10, as in `QUICK2.ANT` |
| Q-U7 rain | `_GlobalKey = 0xA8` | `_RainOn = 1`, `_RainCnt` in 150..299, 100 drops drawn by R |
| Q-U8 water | `_RainOn = 1` for 62 ticks on `QUICK2.ANT` | flooded rows as in `Q01_T140.ANT` (0x4E or +0x2F) |
| Q-U9 Full Game victory and defeat | house for sale with no reds; one black colony with no queen and no brood | 10057 and end with `_BlackWon = 1`; 10056 with `_BlackWon = 0` |
| Q-U10 tick vectors from the original (`tests/referencias/ticks/`, ticks 79–83) | `estado_antes` with S, R and `_gameCycles` from `extras.json` | boy, dog, cat, bird and yard fields equal to `estado_depois` |

Cases on the original (QEMU test bench): Q-01 (rain and water) and Q-02 (colonies), both confirmed (see above and `tests/referencias/README.md`).

## Implementation in the port

- `engine/src/rules/quintal.rs`: `TabelasQuintal` (`data/quintal.json`), `simular_quintal_com(e, tabelas, vitimas, relogios)` (`_DoSimYard`; `Vitimas` = yellow ant tables and spider death throes, for `_YellowDeath` and `_KillSpider`), `simular_agua` (`_DoWater`), `iniciar_quintal` (`_InitSimYard`), `simular_colonias`, `placar_vizinhos`, `reproduzir`, `manter_enxame`, `iniciar_migracao`/`terminar_migracao`, `por_gota`/`iniciar_agua`. `impl RegrasQuintal for RegrasOriginais` uses `RegrasOriginais::quintal` (loaded from `data/quintal.json` with mods).
- **Port clocks** (decision recorded in [../../divergencias.md](../../divergencias.md)): `Estado::relogio_mac` (`Option<u32>`), when present, is the Mac clock — the tick vectors bring it from the original (`extras.json`, `MacTickCount`). Without it, when `Estado::ciclos_jogo` ≠ 0 (the game loop counts the turns of the `MYTIMERFUNC` loop, like the original; the tick vectors bring the original's `_gameCycles`), it is used as `_gameCycles` and as the Mac clock (a port rule, **refuted** as a model of the original: in the old `fases/159` and `colonia_5min/1887` vectors the original had a clock of ~3333 and ~18903 with `_gameCycles` 29579 and 269985). In the live game (`core::partida`) the clock is real time, `⌊ms since start / 55⌋ × 3`, added to a start equal to the largest deadline in the save on that clock (`_CatDelay`, `_DogBarkDelay`, `_BoyMsgCnt`; 0 in a new game): in the original `GetTickCount` is the time since Windows was started, generally larger than that of the session that saved, and the deadlines coming from the save have already expired on load [port decision, 2026-10-18; before, the clock started at 0 and the cat waited for the save's `_CatDelay`]. Otherwise: `_gameCycles` = `_GameTime` (1 per tick, as in the original in "fast"/"ultra" [hypothesis]); Mac clock = `_GameTime × mac_por_tick` (default 6, [hypothesis] ≈ normal speed). Only with the synthetic clock (no `relogio_mac` and no `_gameCycles`), a delay coming from an original save larger than "now + the largest delay the rule writes" is considered expired; with a real clock the delay stays as it is, as in the original (`tutorial` vectors, 2026-10-17: `_CatDelay` 4712 coming from the tutorial file with the clock at ~3300).
- **Verified** (`engine/tests/regras_quintal.rs`): Q-U2 gives both `_YMapPop*` maps of `Q02_T100.ANT` byte for byte; Q-U8 gives rows 61–63 of `_MapB/_MapR` of `Q01_T140.ANT`; Q-U10 reproduces, at ticks 79–83 of the original's vectors, all the fields of the boy, the cat, the dog and the bird (including `_BirdDelay`, with the vector's `_gameCycles`), except `_BoyMsgCnt`/`_CatDelay`/`_DogBarkDelay` (Mac clock, not captured). In those ticks the boy is outside standing still, the dog walks and sits, and the cat walks: the boy mowing, the insecticide and the colonies with random draws were not exercised against the original.
- The test `tick_vazio_sobre_o_quick2` (`engine/tests/estado_simulacao.rs`) assumes empty subroutines and starts failing with the yard turned on (the yard fields change); it needs to be adjusted during integration.
- Messages and music come out as `Evento::Mensagem(id)` and `Evento::Musica(id)`. Sounds (`_myBeginSound`) and the boy's balloon do not generate events (the balloon stays in `_BoyMessOn`/`_BoyMsgOffset`).

## Gaps

- `_YellowDeath` (causes 5 footstep, 6 lawnmower, 9 insecticide) calls `matar_amarela` from `rules/amarela.rs` (test `pisada_mata_a_amarela`). Cause 9 (insecticide) was checked against the original by the vector `ticks/inseticida_morte` (tick 21: death without a body and game over); causes 5 and 6 were not.
- `_DeadAntHere`, `_FindInAList` and `_KillSpider` are those in `engine/src/rules/comum.rs` (see [../../decisoes.md](../../decisoes.md), "Common rules module"); the spider's death throes come from `inimigos.json` (`regras.aranha_agonia`), no longer from a yard constant.
- Texts of the boy's balloon (list 4/1200): **all 23 checked on the test bench on 2026-10-19** (telas/quintal.md §2.3: text, shape and width identical; y 1 line lower and the rain on top, fixed). The yard sounds: not checked.
- `_LastColonyPopR` (not in the save) lives in `Estado::ultimas_colonias_vermelhas` since 2026-10-19: `Some(1)` in `iniciar_quintal` (`_InitSimYard`) and on Load (`Partida::ao_carregar`, because the `_RandYard` before reading calls `_InitSimYard`), written only in the Full Game branch. `None` (state from a vector, read mid-game) = the `_ColonyTotalRed` from before the pass, the previous approximation. Visible effect: on Load of a Full Game already without reds, the first pass shows 10052 (and 10055) again, like the original. **Checked on the test bench (2026-10-19, `cenarios_raros.py ultima_vermelha`):** Full Game without reds, 10052 ("Congratulations, the red ants have been eliminated from the yard.") and 10055 on the pass at tick 32; saved as ULTVERM.ANT and loaded via File → Load, the pass at tick 64 showed both again (captures `re_trabalho/cen/dialogo_32_*.png` and `dialogo_64_*.png`; the 40 recorded ticks match 307/307). Test `ultima_populacao_vermelha_comeca_em_1`. `_LastColonyPopB` is not read.
- Mac clock rate per tick (`mac_por_tick`): on the QEMU test bench (slow TCG) it is 9 per tick; on a real machine it depends on how many ms each tick takes (the clock is real time, see "Clocks") — not measured outside the test bench.
