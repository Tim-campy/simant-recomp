*Leia em português: [inimigos.md](../../pt-BR/spec/regras/inimigos.md)*

# Enemies and yard creatures: antlion, spider, caterpillar and pill bug

This document specifies, for 1:1 reimplementation, the creatures that the simulation moves every tick outside the ant lists: the **antlion** (`_DoAntLions`), the **spider** (`_MoveSpider`), the **caterpillar** (`_DoPillar`) and the **pill bug** (`_DoSow`). The position of each one in the tick is in [simulacao-visao-geral.md](simulacao-visao-geral.md) (steps 8, 9 and 10); their creation at the start of the game in [geracao-mundo.md](geracao-mundo.md) (sections 3.2, 4.2 and 4.3); the ants in [superficie.md](superficie.md); the yellow ant (which can die to them or even become the spider) in [formiga-amarela.md](formiga-amarela.md).

Evidence convention and notation: the same as in [superficie.md](superficie.md#evidence-convention). `S1(n)`, `S2()`, `S4()`, `S8()`, `S16()`… are the S generator; `Map`/`Life` are `_MapA`/`_LifeA` (surface, index `x·64 + y`). No decompiled code is reproduced.

The boy, the dog, the cat, the bird, the lawnmower and the insecticide also kill ants and the yellow ant (`_SimKidOutside`, `_SimKidInside`, `_FootFall`, `_MowerFall`), but they are part of the yard (`_DoSimYard`) and are left for a document of their own [open]. Here we only record their effect on the spider (section 2.8) and the yellow ant deaths they cause (table in [formiga-amarela.md](formiga-amarela.md#9-death-_yellowdeath-10007746)).

## 1. Antlion

### 1.1 State

**Fact seen in the code.** Up to 10 antlions in parallel byte lists (all in the save, [save-ant.md](../formatos/save-ant.md), blocks 67–71):

| Field | Address | Meaning |
|---|---|---|
| `_LionListX/Y` | `9:809c` / `9:80bc` | center of the trap |
| `_LionListM` | `9:7a68` | state: 0 waiting, 1 eating, 2 going back to waiting |
| `_LionListS` | `9:7d34` | time counter of state 1 |
| `_LionListT` | `9:7d4e` | animation frame (0–6: 0–4 when entering and leaving state 1, 3–6 drawn at random during the meal); the center tile is `0x38 + T` (0x38–0x3E) |
| `_LionIndex` | `8:8a88` | how many exist (0–9; the increment stops at 9) |
| `_InitialLions` | `9:9e8c` | how many the patch had when it was created (0 in the house) |

Tiles (fact seen in the code): center 0x38 (frame 0) to 0x3C; rim 0x30–0x37 (see [tiles-valores.md](tiles-valores.md)); 0x3F = dead antlion. The "jump" used when eating is 0x39 (frame 1).

### 1.2 Creation: `_InitAntLions`, `_AddRandAntLion`, `_AddAntLion`

`_InitAntLions(n)` is in [geracao-mundo.md](geracao-mundo.md) (4.2). `_AddRandAntLion` (`1030:4222`) repeats the position draw of a single antlion; the same block is inlined twice in `_DoAntLions` (1.3). `_AddAntLion(x, y)` (`1030:4340`) creates one at a given position, without a random draw [no caller found].

Position draw (fact seen in the code), up to 200 attempts, in this order per attempt: `x = S1(64) + S1(65)`, `y = S1(32) + S1(33)`; accepted if `_IsClear3x3(1, x, y)`, or if the center cell is free (`_IsClearTile`) and there have already been more than 100 attempts. Writes 0x38 at the center and the rim (0x30 + `DGROUP:25e8[k]`) on the free neighbors; clears M, S and T; `_LionIndex += 1` if < 9 (with 9 antlions, the tenth overwrites the last slot).

### 1.3 `_DoAntLions` (`1030:43fa`), every tick

**Fact seen in the code.** Our own pseudocode:

```
if _LionIndex == 0 and _InitialLions > 0 and S1(1024) == 0:     ← short-circuit: S1 only if the first two hold
    create a new antlion (draw 1.2) ; end of the tick for the antlions
else, for k = 0 .. _LionIndex − 1:
    (x, y) = center of k
    case M[k]:
    0 (waiting):
        v = Map(x, y)
        if v is a pebble (0x51–0x53) or food:              ← something fell into the trap
            if S1(512) == 0:
                d = S8()
                for j = 0..7: target = (x, y) + 2·D8[d]
                    if _IsClearTile(1, target): Map(target) = v ; sound ; leave the loop
                    d = (d + 1) & 7
                sound ; S[k] = 25 ; ENTER_1
            (no ant test on this tick)
        else:
            c = (0, 0)                                     ← first cell tested: the center itself
            repeat 2 times:
                if Life(x + c) ≠ 0: GRAB(x + c) ; leave
                c = D9[S1(8) + 1]                         ← also drawn after the 2nd empty cell
    1 (eating):
        n = number of neighbors (8 directions) with Life ≠ 0
        if n ≥ 7:                                         ← the ants surrounded and killed the antlion
            message 10050 ; Map(x, y) = 0x3F ; remove k from the list (shifts the following ones) ; _LionIndex −= 1
            (the loop continues to k + 1: the antlion shifted into slot k does NOT run on this tick;
             fact seen in the disassembly, 1030:48ce → 4ac8 `inc di`)
        else:
            if T[k] < 4: T[k] += 1
            else if S[k] == 0: M[k] = 2 ; T[k] = 4
            else: S[k] −= 1 ; if S[k] is odd: T[k] = S1(4) + 3
            Map(x, y) = 0x38 + T[k]
    2 (going back):
        if T[k] == 0:
            M[k] = 0 ; _AntsEatenByLions += 1
            if _LionIndex < 9 and (_AntsEatenByLions & 15) == 15: create a new antlion (draw 1.2)
        else: T[k] −= 1
        Map(x, y) = 0x38 + T[k]

GRAB(cell):
    if it is the yellow ant (0xFF/0xFE): _MoveMyLife(1, center, _MeType, _MeDir) ; _YellowDeath(2)
    else:
        if S8() == 0: sound
        j = _FindInAList(cell) ; if j ≥ 0: _RemoveFromAList(j)    ← immediate removal, with shifting
        black: _BAntsEaten (9:76a0, 32 bits) += 1 ; red: _RAntsEaten (9:7d5c) += 1
    S[k] = 50
ENTER_1:
    M[k] = 1 ; T[k] = 1 ; Map(x, y) = 0x39
```

Notes (fact seen in the code):

- `D9` = `_Dx9/_Dy9` (`8:0010`/`8:001a`): index 0 is "stopped", 1–8 are directions 0–7. The antlion looks at its own center and **one** randomly drawn neighbor per tick.
- `_RemoveFromAList` (`1020:2b42`) clears the ant's `Life` and shifts the following slots of list A one position back. Since `_DoAntLions` runs before `_DoAntSimA`, this only changes the indices before the ant loop.
- When the antlion dies, the center becomes 0x3F, but the 0x30–0x37 rim remains on the map.
- When the trap "spits out" a pebble or food and finds no free place, the object is lost (the center is overwritten by 0x39).
- The antlion never moves.
- Message 10050 (SHARED 4/10050): "The ants have surrounded and killed an ant lion! They tear its pulpy body apart and eat it.", shown by `_PictStrnDialog`.

### 1.4 `_KillAntLion(k)` (`1030:4b58`)

Same removal as in state 1 (center = 0x3F, list shifting), without a message. Called by the Insecticide tool of Experimental mode (`_ExpKillAnts` `1010:7d5c`) when the tile hit is in 0x38–0x3E; the antlion is found by `_FindInLionList` (`1030:4b12`).

**`k` = −1** (antlion tile with no entry in the list: `_FindInLionList` returns −1 and `_ExpKillAnts` calls `_KillAntLion(-1)` without testing) [seen in the code, `1030:4b58`; **checked on the test bench on 2026-10-19**, action `acoes/exp_leao_m1`]. The original reads and writes "slot −1" of each list vector, which is the last byte of the preceding variable in DGROUP (names from the `.SYM`):

| Vector | Slot −1 | What it is |
|---|---|---|
| `_LionListX` (`9:809c`) | `9:809b` | high byte of the selector of `_EggStr` (far pointer, `9:8098`) |
| `_LionListY` (`9:80bc`) | `9:80bb` | high byte of `_LastThemeSong` (`9:80ba`; theme counter 0–2, usually 0) |
| `_LionListT` (`9:7d4e`) | `9:7d4d` | high byte of `_BoyMsgCnt` (`9:7d4a`, 32 bits) |
| `_LionListM` (`9:7a68`) | `9:7a67` | high byte of the y of `_CenterPnt` (`9:7a64`) |
| `_LionListS` (`9:7d34`) | `9:7d33` | high byte of `_mapOutBuf` (`9:7d32`, memory handle) |

Effect: (1) `_SetMap(1, X[−1], Y[−1], 0x3F)`: a 0x3F in `_MapA` at (high byte of the selector of `_EggStr`, high byte of `_LastThemeSong`), if it fits in the map — with `_LastThemeSong` < 256, on row 0; (2) with `_LionIndex` > 0, the counter drops by 1 and the shifting starts at slot −1: slots −1 … n−1 receive 0 … n, that is, **antlion 0 leaves the list** (its tile stays on the map, frozen) and the five bytes in the table above receive its fields (X → selector of `_EggStr`, Y → `_LastThemeSong`, T → `_BoyMsgCnt`, M → `_CenterPnt`, S → `_mapOutBuf`). With `_LionIndex` = 0 only the `_SetMap`. The tile hit does not change. The writes to the selector of `_EggStr` and to the `_mapOutBuf` handle may corrupt the original's interface later [hypothesis: not observed].

**Port** (2026-10-19, `inimigos::matar_formiga_leao_sem_entrada`): reproduces what stays in the game state (and in the save): the removal of entry 0 and the counter, the high byte of `_BoyMsgCnt` (the boy's balloon starts waiting 2²⁴ × T of the Mac clock; in practice it never appears again if T > 0) and that of `_LastThemeSong` (the next theme goes back to the first). It does not reproduce the `_SetMap` (x is a Windows memory selector, with no equivalent) nor the writes to `_EggStr`, `_CenterPnt` and `_mapOutBuf` (Windows interface; reproducing them is not safe). Test `inseticida_sem_entrada_na_lista_tira_a_formiga_leao_0`. Before, the port did nothing.

**Test bench (2026-10-19, `cenarios_raros.py exp_leao_m1`, Experimental paused, two antlions in the list, insecticide in spray mode — 8 sprays — on a 0x38 with no entry):** the 1st spray wrote 0x3F at (20, 0) — x = high byte of the selector of `_EggStr` in this session (0x14xx), y = high byte of `_LastThemeSong` (0) — and removed antlion 0; the 2nd wrote 0x3F at its center, (51, 10), because after the shifting slot −1 holds its X (in the selector) and its Y (in `_LastThemeSong`), and removed the next one; the 3rd wrote 0x3F at that one's center, (28, 37); the other 5 repeat that `_SetMap`. `_LastThemeSong` ended at 0x2500 (Y = 37 in the high byte), `_BoyMsgCnt` with T (0) in the high byte, the tile hit intact. **Port fixed:** `Estado::leao_x_vaga_menos_1` stores the X that went to slot −1 (outside the save; `None` while it is the Windows selector) and `matar_formiga_leao_sem_entrada` does the `_SetMap(X[−1], high byte of _LastThemeSong, 0x3F)` when it knows it; the vector carries the test bench's selector (`leao_x_vaga_menos_1` = 20 in the action). 307/307. Only the first `_SetMap` of a game without that value is left out (x would be the Windows selector).

## 2. Spider

### 2.1 State and coordinates

**Fact seen in the code.** The spider is not in any list and does not appear in `_LifeA`. Coordinates in sixteenths of a cell: cell = `_SpidX >> 4`, `_SpidY >> 4`.

| Variable | Address | Meaning |
|---|---|---|
| `_SpidX`, `_SpidY` | `10:ac7c` / `10:ac7e` | position (×16) |
| `_SpidDir` | `10:ac80` | direction 0–7 |
| `_SpidOn` | `9:9bd2` | 1 = spider on the map |
| `_SMode` | `9:729e` | state: 0 stopped, 1 walking, 2 chasing, 3 eating, 4 fleeing, 5 dying |
| `_Starg` | `9:7a5e` | target: index in list A, −1 = the yellow ant, −2 = none |
| `_StargLife` | `9:80a8` | target's T at the moment it was chosen (0xFF for the yellow ant) |
| `_Scycle` | `9:7290` | leg animation frame (0–1023) |
| `_Scycle2` | `9:77ac` | tick counter (0–1023) |
| `_EatCnt` | `9:77ae` | remaining chewing ticks |
| `_SCorpseBase` | `9:78da` | 0 if it ate a black (or the yellow ant), 4 if it ate a red |
| `_SpidBurpCnt` | `9:8072` | counter for the burp (starts at 10) |
| `_DeathCnt` | `9:72e0` | remaining death-throes ticks (state 5) |
| `_SpidRevenge` | `9:72c2` | how many spiders have already died (maximum 5) |
| `_MeSMode` | `9:7d60` | "hunting mode": 0 normal, 6 target chosen by the player, 7 scent, 8 laser |
| `_RevSpider` | `8:8a5a` | 1 = walks backwards (steps are inverted) [open: nothing sets the flag] |
| `_SuserX/Y` | `9:78b8` / `9:78ea` | clicked destination when the player is the spider |
| `_MeMode` | `9:9fe8` | 1 = the player controls the spider (section 2.7) |

Tables (fact verified in `SIMANTW.EXE`, segment 8): step `_SpX` (`8:87dc`) = `0 3 4 3 0 −3 −4 −3`, `_SpY` (`8:87e4`) = `−4 −3 0 3 4 3 0 −3` (in sixteenths: walking, about 1/4 of a cell per tick; running, ×5); mouth cell `_SeatX/_SeatY` (`8:87ec`/`8:87f4`) = `_Dx8/_Dy8`.

Start of the game (`_InitSpider` `1020:4668`, [geracao-mundo.md](geracao-mundo.md)): (0x400, 0x200) = cell (64, 32), `_SMode = 0`, `_SpidOn = 1` except in the Tutorial.

### 2.2 `_MoveSpider` (`1020:4704`), every tick — overview

```
_Scycle2 = (_Scycle2 + 1) & 1023
(cx, cy) = (_SpidX >> 4, _SpidY >> 4)
if _MeMode == 1 and _SMode ∉ {2, 3}: PLAYER CONTROL (2.7) ; end
if _SpidOn == 0: MAYBE APPEAR (2.3) ; end
if (_Scycle2 & 3) == 0 and _SMode < 5: SURROUND (2.4)       ← may end the tick
if _SMode < 4 and _MeSMode == 8: _SpiderScan()               (2.6)
run state _SMode (2.5)
if the cell (_SpidX >> 4, _SpidY >> 4) left the map:
    _SpidOn = 0 ; if _MeMode == 1: _MeMode = 0 ; _YellowDeath(4)
if house and Map(cell) > 0x90: _SpidOn = 0
```

### 2.3 Appearing

If `_CurGameType` = 0 (Tutorial): never. Otherwise, with `S1(300) == 0`: `_SpidX = S1(1024) + 512` (cell 32–95), `_SpidOn = 1`, `_SMode = 1`, `_RevSpider = 0`; `S1(2) ≠ 0` → enters from the top (`_SpidY = 1`, direction 4); otherwise from the bottom (`_SpidY = 0x3FF`, direction 0). Fact seen in the code.

### 2.4 Surrounding: the ants kill the spider

Every 4 ticks (`_Scycle2 & 3 == 0`), outside the death throes: counts the occupied cells in `_LifeA` in the 3 × 3 square around the spider's cell, including its own (the spider does not occupy `_LifeA`).

- 9 occupied (all): the spider dies. `_SMode = 5`, `_DeathCnt = 500`, `_Scycle = 0`. If the player was the spider: `_MeMode = 0` and `_YellowDeath(3)`; end. Otherwise: message 10046 ("Congratulations! The spider has been killed by the ants."), `_SpidRevenge += 1` (up to 5); with `_SpidRevenge` 3 or 4, the **next** spider hunts by scent (`_MeSMode = 7`); with 5, by laser (`_MeSMode = 8`). (An `S2()` draw that would reset the revenge is in unreachable code, because `_SpidRevenge` never goes above 5.)
- 5 to 8 occupied: `_SMode = 4` (flees).

### 2.5 States

**Fact seen in the code.** "Walk 1" = add `(_SpX[dir], _SpY[dir])` (or subtract, with `_RevSpider`) and `_Scycle ± 1`; "run" = the same ×5 and `_Scycle ± 2`; `_Scycle` always `& 1023`.

| `_SMode` | Exact order of the actions |
|---|---|
| 0 stopped | `_Scycle = 2`; `S1(150) == 0` → `_SMode = 1`; `_Starg = _SFoundAnt()`; if ≠ −2 → FOUND (below) and end; otherwise `S1(30) == 0` → `_SpidDir = Turn[_SpidDir][S1(8)]`; if `_MeSMode == 8` → `_SpiderScan()` (again) |
| 1 walking | walk 1; `S1(20) == 0` → `_SpidDir = Turn[_SpidDir][S1(8)]`; `_Starg = _SFoundAnt()`; ≠ −2 → FOUND; otherwise `S1(50) == 0` → `_SMode = 0`, `_Scycle = 2` |
| 2 chasing | see below |
| 3 eating | see below |
| 4 fleeing | `_SpidDir = Turn[_SpidDir][S1(8)]`; run; `S1(50) == 0` → `_SMode = 0`, `_Scycle = 2` |
| 5 dying | `_DeathCnt −= 1`; if 0: `_SpidOn = 0`, `_SMode = 0` and `_DropFoodA` three times on the cell (food for the ants) and end; otherwise `S1(1000) < _DeathCnt` → `_Scycle = S1(2) + 2` (if `_DeathCnt ≤ 400`) or `S1(3) + 1` (animation) |

FOUND: `_SMode = 2`; `_StargLife = T[_Starg]` (or 0xFF if `_Starg = −1`).

**State 2, chasing:**

```
if _Starg < 0 (the yellow ant):
    if _MePlane > 1 (the yellow ant is in a nest): _SMode = 0 ; _Starg = −2 ; end
else if (T[_Starg] ^ _StargLife) & 0xF0 ≠ 0:        ← the target changed (died, picked up a load, etc.)
    _SMode = 0 ; _Starg = −2
    if _MeMode == 1: player's destination and _MeLoc = spider's cell ; _MeSMode 6 → 0 ; center (AutoTrack option)
    end
dist = |dx| + |dy| to the target (the yellow ant: _MeLocX/Y ; ant: X/Y of the slot)
if dist > 64 and _MeMode ≠ 1: _SMode = 0 ; _Starg = −2 ; end       ← gives up
if dist > 1:
    _SpidDir = Turn[_SpidDir][_GetDir(cell, target) − 1] ; sound ; run
    if _MeMode == 1: _MeLoc = spider's cell (and center if AutoTrack)
    end
_SMode = 3 ; center the spider on the cell ((& 0xFFF0) + 8) and go straight on to state 3
```

**State 3, eating.** Mouth cell `b = ((cx + _SeatX[dir]) & 127, (cy + _SeatY[dir]) & 63)`.

```
(if _MeMode == 1: _MeLoc = cell ; _MeSMode 6 → 0 ; center if AutoTrack)
if _Starg == −2:                                    ← already chewing
    CHEW
else if _Starg < 0 (the yellow ant):
    _MoveMyLife(_MePlane, b, _MeType, _MeDir) ; _YellowDeath(1) ; _SCorpseBase = 0 ; START
else if (T[_Starg] ^ _StargLife) & 0xF0 == 0:      ← the prey is still the same
    black: _BAntsEaten += 1 ; _SCorpseBase = 0      red: _RAntsEaten += 1 ; _SCorpseBase = 4
    Life(prey) = 0 ; T[_Starg] = 0 ; START
else: _Starg = −2 ; FINISH
START: _Starg = −2 ; _EatCnt = (_MeSMode < 7 ? 50 : 11) ; CHEW
CHEW:
    if yard and Map(b) < 0x18: Map(b) = S4() + _SCorpseBase + 0x10      ← remains at the mouth
    if _EatCnt > 0:
        _EatCnt −= 1
        if _EatCnt % 10 == 0 and S2() ≠ 0: sound with S256()                ← S256 only if S2 ≠ 0
        end of the tick (stays in state 3)
    _SpidBurpCnt −= 1 ; if it reached 0: _SpidBurpCnt = 10 ; if "Silly" ≠ 0 and S2() == 0: burp
    _DeadAntHere(b, _SCorpseBase)                    ← permanent corpse ([superficie.md](superficie.md) 6.4)
    FINISH
FINISH: _SMode = 0 ; if _MeMode == 1: _MeSMode 6 → 0 ; player's destination = spider's cell
```

`_DeadAntHere` receives `_SCorpseBase` (0 or 4) as the "color": 4 counts as red.

### 2.6 Looking for prey: `_SFoundAnt` (`1020:53f6`) and `_SpiderScan` (`1020:5536`)

**`_SFoundAnt()`**, without random draws:

- `_MeSMode == 7` (scent): scans list A **from back to front** and returns the first living slot with `_GetDis(cell, ant) < 801` (radius ≈ 28). If none, and the yellow ant is on the surface and is not the spider (`_MeMode == 0`, `_MePlane == 1`), with distance² < 801 → −1. Otherwise −2.
- otherwise (sight): walks along the straight line in direction `_SpidDir`, up to 20 cells: left the map → −2; distance² > 400 → −2; cell with `Life ≠ 0`: the yellow ant → −1; ant found in the list by `_FindAntIndex(1, x, y, T)` → the index. Nothing found → −2.

**`_SpiderScan()`** (laser; only with `_MeSMode == 8`):

```
if _SMode == 5: return −1
base = ((_SpidDir − 2) & 7) · 32                 ← angle from 0 to 255 (one turn), 0 = east, 64 = south
repeat 2 times:
    for a = base − 32 .. base + 31:
        r = S1(12) + 1
        p = (cx + cos(a)·r / 0x7FFF, cy + sin(a)·r / 0x7FFF)     (_fracCOS/_fracSIN, same table as _AddFood)
        if p on the map and Life(p) ≠ 0 and _FindAntIndex(1, p, Life(p)) = j ≥ 0:
            draw the laser (_DoLaserFire) ; if S4() ≠ 0: Life(p) = 0 ; T[j] = 0 ; _DeadAntHere(p, color)
            return j
return −1
```

That is up to 128 `S1(12)` draws per call without a target. Note: the laser ignores the yellow ant (it is not in list A).

### 2.7 Player-controlled spider (`_MeMode = 1`)

**Fact seen in the code.** The player becomes the spider through the body exchange (see [formiga-amarela.md](formiga-amarela.md#8-body-swap-and-target-_exchangelives-_dolifeexchange)). While `_SMode` is neither 2 nor 3:

```
if _MeSMode == 7: _Starg = _SFoundAnt() ; if ≠ −2: FOUND (starts chasing) and end
else if _MeSMode == 8: _SpiderScan()
dist = _GetDis(_MeLoc, _Suser)
if dist < 1: _Scycle = 2 ; end                         ← arrived
d = _GetBestDir(1, _MeLoc, _Suser)                       (section 2.9)
if d == −1: end ; if d == −2: d = _GetDir(_MeLoc, _Suser) − 1 ; if d < 0: end
_SpidDir = Turn[_SpidDir][d]
if _MeCmd == 0 or dist < 3: walk 1   else: run (×5)
_MeLoc = (_SpidX >> 4, _SpidY >> 4) ; center if AutoTrack
```

With AutoTrack (`_OptionStates[0]`, `8:85f2`), `_MoveSpider` calls `_GotoMyAnt` at four points, always after setting `_MeLoc` to the spider's cell [code, `1020:4704`]: after the player-control step (above; not when it arrived or found no direction), in the chase when the target changed (state 2, "the target changed"), at each chase step (state 2, dist > 1) and at the start of state 3. Within the tick, the `_CenterEdit` that moves the view redraws the edit window, and with `_SMode` ≥ 2 or `_MeSMode` ≥ 6 the palps draw `S1(4)` ([rng.md](rng.md), "Drawing within the tick"). **Port:** `rules::inimigos::autotrack` → `amarela::centralizar` (the same `_GotoMyAnt` from the rules). **Verified (2026-10-19):** vectors `ticks/aranha_jogador_autotrack` (11 ticks: walks with the view following the spider; chases in target mode with the palps in the middle of the tick; eats), 307/307 with S, R and health; without the fix, 10 ticks get `_MapPnt` wrong and the chase ones get S, R, lists and health wrong.

`_MeLoc` follows the spider's cell. The destination and `_MeCmd` come from the click (`_processSpider`, see [formiga-amarela.md](formiga-amarela.md#11-player-controls)). When the player's spider leaves the map (`_YellowDeath(4)`, message 10028) or is surrounded (`_YellowDeath(3)`, message 10026), the player reincarnates in an ant.

### 2.8 Other spider deaths: `_KillSpider` (`1020:53d4`)

Sets `_SMode = 5`, `_DeathCnt = 500`, `_Scycle = 0` (death throes; no message). Callers (fact seen in the code): the boy outside (`_SimKidOutside`) and inside the house (`_SimKidInside`), the foot (`_FootFall`), the lawnmower (`_MowerFall`) and the Insecticide of Experimental mode when it hits the spider's cell. `_ScanForAnts` (`1020:5362`) counts the ants in the 3 × 3 like the surround, but has no caller.

### 2.9 `_GetBestDir(plane, x, y, tx, ty)` (`1028:405e`)

Same rule as `_GetRedBestDirs` ([superficie.md](superficie.md), 9.2), replacing the passability test with: `_IsNotObstacle(plane, v)` and the tile **not** being a pebble (`_IsThisPebble`). Returns the best free neighbor, or the best blocked one, or −2; −1 if it is already at the target. No random draw.

### 2.10 `_GetStrategy` and the spider

With the yellow ant on the surface, `_ChaseSpid = 1` when the spider is less than 10 cells away (distance² < 100): the black ants in Defense start going toward the spider ([superficie.md](superficie.md), 2.5.7). `_ChaseSpid` is reset every tick.

## 3. Caterpillar (`_DoPillar`)

### 3.1 State

**Fact seen in the code; names from the `.SYM`** ("Pillar" from *caterpillar*).

| Variable | Address | Meaning |
|---|---|---|
| `_PillarState` | `8:8a8a` | 0 = does not exist (will be created on the next step), 1 = walking |
| `_PillarX/Y` | `8:8a8c` / `8:8a8e` | head |
| `__PillDir` | `9:9b1e` | 0 up (y−1), 1 right (x+1), 2 down (y+1), 3 left (x−1) |
| `_PillarSeg` | `9:78d4` | step phase (5 → 0) |
| `_PillarMap` | `9:7c0e` | 6 words: the ground that was under the body, indexed by `coordinate % 6` (y if the direction is vertical, x if horizontal) |

Tiles (fact seen in the code): head 0x6C (going up), 0x6B (right), 0x6F (going down), 0x68 (left); body 0x6D/0x6E (vertical) and 0x69/0x6A (horizontal). This confirms the [hypothesis] in [tiles-valores.md](tiles-valores.md) that 0x69–0x6B are the caterpillar and completes the 0x68–0x6F range.

### 3.2 `_DoPillar` (`1030:4cdc`), only on odd ticks and only in the yard

**Fact seen in the code.** First it calls `_DoSow` (section 4). Then:

```
if _PillarState == 0: _MakeAPill() ; _PillarState = 1 ; _PillarSeg = 4 ; end
front = head + step of the direction (dir 0: (X, Y−1) ; 1: (X+1, Y) ; 2: (X, Y+1) ; 3: (X−1, Y))
if front on the map and Life(front) ≠ 0:                  ← an ant blocks
    n = occupied cells in the 3 × 3 around the head
    if n ≥ 6: _PillarState = 0 ; _MakePillFood()            ← the ants kill the caterpillar
    end (the caterpillar does not move on this tick)
s = _PillarSeg − 1 ; _PillarSeg = s
"behind(k)" = head − k·step
if s == 4: restore the ground at behind(5) (from _PillarMap)
else:      write body 0x6D (vertical) or 0x69 (horizontal) at behind(s + 1)
write body 0x6E (vertical) or 0x6A (horizontal) at behind(s)
if s ≠ 0: end
advance the head one step
if X < −6 or X > 134 or Y < −6 or Y > 69: _PillarState = 0 ; end     ← left the yard
store the ground under the new head in _PillarMap ; write the head and, at behind(1), the body 0x6D/0x69
_PillarSeg = 5
```

All writes to the map go through `_IsValidA` (outside the map, nothing is written). There is no random draw outside `_MakeAPill`. The caterpillar does not test the terrain: it passes over any tile (storing and restoring the ground) and only stops in front of ants. Every 5 calls (10 ticks) the head advances one cell; the body occupies the head and the 4–5 cells behind.

**`_MakeAPill` (`1030:53da`):** `dir = S1(4)`; dir 0: head at `(S1(128), 63)`; 1: `(0, S1(64))`; 2: `(S1(128), 0)`; 3: `(127, S1(64))` (one draw besides the direction one). Stores the ground and writes the head. The caterpillar enters from the edge opposite to the direction in which it walks.

**`_MakePillFood` (`1030:57d2`):** for k = 0..5 along the body (head and 5 cells behind), restores the stored ground and, if the resulting tile is < 0x18, writes 0x4B (food with 4 units). The functions `_IsPillDead`, `_StorePillarMap`, `_ReplacePillarMap`, `_PlacePillTile` and `_PillGetLife` do parts of this same logic and are not called by `_DoPillar` (which has everything inlined).

**Verified in `QUICK2.ANT`** (tick 78): `_PillarState = 1`, head at (7, 58), `_PillarSeg = 1`; tiles 0x6B at (7, 58), 0x6A at (6, 58) and 0x69 at (3–5, 58), exactly the drawing predicted for direction 1 with s = 1.

**Quirk (fact seen in the code):** the index `coordinate % 6` uses C's signed remainder; with the head outside the map (negative coordinate) the index would be negative, but writes outside the map are discarded by `_IsValidA`. Reproduce by storing only when the position is valid.

## 4. Pill bug (`_DoSow`)

### 4.1 State

**Fact seen in the code** ("Sow" from *sowbug*). Three word slots: `_SowX` (`9:9bc8`), `_SowY` (`9:9bda`), `_SowDir` (`9:9c2a`), `_SowSave` (`9:78cc`, the ground under the bug). Tiles by direction: `_SowTab` (`8:8a90`) = `72 73 71 70 72 73 71 70` (fact verified in `SIMANTW.EXE`).

### 4.2 Creation

Creation is inlined in `_InitPillar` (`1030:4bf8`) and repeated in `_InitSow` (`1030:3ef8`): only in the yard, it fills slots **2 and 1** (in that order). For each one, it repeats until it succeeds: `x = S1(128)`, `y = S1(64)`; if `Map(x, y) < 0x10`: `dir = S1(8)`, stores position, direction and ground, and writes `_SowTab[dir]`. Correction to [geracao-mundo.md](geracao-mundo.md) (4.3): it is not "two attempts without repetition"; the loop only ends after two successes (fact seen in the code).

Slot **0 is never initialized** (it keeps whatever is in memory: zeros in a new game).

### 4.3 `_DoSow` (`1030:3f8a`), called at the start of `_DoPillar` (odd ticks)

```
for v = 0, 1, 2:
    if S4() ≠ 0:                                       ← 3/4 chance of acting
        if S4() == 0:                                  ← 1/4 chance of turning
            _SowDir[v] = (_SowDir[v] + S1(3) − 1) & 7
            Map(position) = _SowTab[_SowDir[v]]          (redraws in place)
        target = position + D8[_SowDir[v]]
        if target on the map and Life(target) == 0 and Map(target) < 0x10:
            Map(position) = _SowSave[v] ; _SowSave[v] = Map(target)
            Map(target) = _SowTab[_SowDir[v]] ; position = target
```

The pill bug neither kills nor is killed; it only walks on ground (< 0x10). Its tiles 0x70–0x73 are greater than `_Barrier` (0x50), and therefore an obstacle for the ants.

**Quirk P-Sow, verified in `QUICK2.ANT`:** slot 0 (never initialized, at (0, 0) with direction 0) is processed as a third "ghost" pill bug. The save has `_SowX/Y/Dir = (0,0,0), (32,46,6), (49,8,2)` and tiles 0x71 at (32, 46) and (49, 8) (the two real slots, `_SowTab[6]` and `_SowTab[2]`) and **0x72 at (0, 0)**, which is `_SowTab[0]` written by the ghost when turning. The port must reproduce slot 0.

## 5. Order of the random draws per tick (summary)

| Tick step | Creature | Possible draws, in order |
|---|---|---|
| 8 | antlions | `S1(1024)` (with none); per antlion: state 0 `S1(512)` and `S8()` (object) or `S1(8)`, `S1(8)` and `S8()` (ant); state 1 `S1(4)`; state 2 the position draw of a new one |
| 9 | spider | appearing: `S1(300)`, `S1(1024)`, `S1(2)`; laser `S1(12)`×n and `S4()`; state 0: `S1(150)`, `S1(30)`, `S1(8)`; state 1: `S1(20)`, `S1(8)`, `S1(50)`; state 3: `S4()`, `S2()`, `S256()`, `S2()`, and those of `_DeadAntHere`; state 4: `S1(8)`, `S1(50)`; state 5: `S1(1000)`, `S1(2)`/`S1(3)` |
| 10 | pill bug (3 slots) and caterpillar | per slot: `S4()`, `S4()`, `S1(3)`; new caterpillar: `S1(4)`, `S1(128)` or `S1(64)` |

The conditional details are those of the sections above.

## 6. Suggested test cases (QEMU bench)

Test bench: [docs/bancada-qemu.md](../../bancada-qemu.md). Reading the save via the offsets of [save-ant.md](../formatos/save-ant.md).

| # | Rule | Input | Expected | How to observe |
|---|---|---|---|---|
| I1 | Ghost pill bug (4.3) | any new Quick Game | tile 0x70–0x73 at (0, 0) after a few ticks; `_SowX[0] = _SowY[0] = 0` | **already verified** in `QUICK2.ANT` |
| I2 | Pill bug walking | saves every ~20 ticks | `_SowX/Y` change by at most 1 cell per odd tick; the previous ground reappears (`_SowSave`) | fields 47918–47936 and `_MapA` |
| I3 | Caterpillar (3.2) | successive saves until the caterpillar crosses the yard | head advances 1 cell every 10 ticks; body drawing as in section 3.2; when leaving, `_PillarState = 0` and a new one appears at an edge | fields 47906, 48230–48236 and `_MapA` |
| I4 | Dead caterpillar becomes food | Experimental: surround the head with ≥ 6 ants | `_PillarState = 0` and up to 6 tiles 0x4B in place of the body | `_MapA` |
| I5 | Antlion eats (1.3) | Experimental: release ants near a trap | slot removed from list A (with shifting), `_BAntsEaten` +1, `_LionListM = 1`, S = 50, center 0x39–0x3C | lists A, blocks 67–71, `_MapA` |
| I6 | Antlion surrounded | Experimental: ≥ 7 ants around it during state 1 | center 0x3F, `_LionIndex − 1`, message 10050 | capture + fields |
| I7 | Pebble in the trap | Experimental or yellow ant: drop a pebble at the center | after 512 ticks on average the pebble appears 2 cells away; no ant is caught before that | `_MapA` |
| I8 | Spider disappears and reappears (2.3) | Quick Game; wait for the spider to leave through the edge | `_SpidOn = 0` and, later, `_SpidOn = 1`, `_SMode = 1`, `_SpidY` = 1 or 0x3FF | fields 48274–48290 |
| I9 | Spider eats (2.5) | bring ants close to the stopped spider | `_SMode` 2 → 3, prey with T = 0, `_EatCnt` 50 → 0, corpse 0x10–0x17 on the mouth cell | fields and lists |
| I10 | Surrounding the spider (2.4) | Experimental: 9 ants in the spider's 3 × 3 | `_SMode = 5`, `_DeathCnt = 500` decreasing; at the end, 3 foods on the cell; `_SpidRevenge` +1 | fields 48060, 48286 |
| I11 | Revenge (2.4) | repeat I10 three and five times | `_MeSMode = 7` and then 8 (laser visible on screen) | field 48194 + capture |
| I12 | Port × original | `.ANT` with creatures in various states, N ticks | identical positions, states and tiles | comparison of the blocks cited |

## Implementation in the port

Code: `engine/src/rules/inimigos.rs`. Public functions: `formigas_leao` (`_DoAntLions`), `aranha` (`_MoveSpider`, with `_SFoundAnt`, `_SpiderScan`, surround and player's spider), `lagarta` (`_DoPillar`, which calls `bichos_de_conta` = `_DoSow`), `matar_aranha` (`_KillSpider`), `matar_formiga_leao` (`_KillAntLion`) and `achar_formiga_leao` (`_FindInLionList`). `impl RegrasInimigos for RegrasOriginais` wires the first three into the tick.

Data: `data/inimigos.json` (`TabelasInimigos`), generated by `python tools/extrair_inimigos.py SIMANTW.EXE --json data/inimigos.json` (addresses at the top of the script): `_Dx8/_Dy8`, `_Dx9/_Dy9`, `_TurnTab`, `_SpX/_SpY`, `_SeatX/_SeatY`, `_SowTab`, the trap rim (`DGROUP:25e8`) and, in the `regras` block, the constants (chances, times, tiles, limits), with the source function in `_origem`. Since `RegrasOriginais` (in `tick.rs`) does not yet have a field for these tables, the tick uses the copy embedded at compile time (`TabelasInimigos::padrao()`); `TabelasInimigos::carregar(&Recursos)` already reads with mods, it only remains to store the result in `RegrasOriginais`. The laser's sine comes from `RegrasOriginais::geracao` (object 9/1000); without it, it uses `trunc(sin(i·π/128)·32767)`, which matches the 64 values of the database (verified: test `seno_calculado_igual_ao_do_banco`).

Decisions and details reproduced (fact seen in the code, unless stated otherwise):

- `_DoAntLions` reads the neighbors' `_LifeA` through the linear index `x·64 + y` without testing validity (at the top/bottom edge it falls into the neighboring column). The port does the same; outside the 8 KB block it returns 0 [hypothesis: in the original it would be another variable's memory].
- Antlion death by surrounding: the next one (shifted into slot k) does not run on the same tick (correction in this spec, see 1.3).
- An antlion created in state 2 (every 16 meals) runs on the same tick, because the loop re-reads `_LionIndex`.
- `_RemoveFromAList` leaves the last used slot duplicated (like the original's `_BlockMove`).
- The drawing of the laser and the centering of the view (AutoTrack) are not reproduced. Sounds become `Evento::Som` (see [som.md](som.md)); the draws tied to sounds (the antlion's `S8()`, chewing's `S2()`/`S256()`, the burp's `S2()`) are made in the same place, with or without the request.
- Messages 10050 and 10046 go to `Estado::eventos` (`Evento::Mensagem`).
- `_DropFoodA` in the house, in the "food falls" case: superficie.md 8.2 (port: `Estado::passo_queda`). The dead spider does not use the result.
- `_MoveMyLife` and `_YellowDeath` (causes 1 spider, 2 antlion, 3 surrounded, 4 fled the map) are those of `rules/amarela.rs` (`mover_marca`, `matar_amarela`), with animations that draw random numbers, messages and reincarnation (formiga-amarela.md §9). `aranha` and `formigas_leao` receive the yellow ant's tables.
- `_IsClearTile`/`_IsClear3x3`, `_IsNotObstacle`, `_IsThisPebble`, `_IsItFood`, `_GetDis`, `_GetDir`, `_GetBestDir`, `_FindInAList`, `_FindAntIndex`, `_DeadAntHere`, `_DropFoodA` and `_KillSpider` are in `engine/src/rules/comum.rs` (see [../../decisoes.md](../../decisoes.md), "Common rules module"); `_RemoveFromAList` remains local (only the enemies use it).

Verified:

- Unit tests in `engine/tests/regras_inimigos.rs` for I1–I11 (ghost bug and pill bug walking; caterpillar advancing, leaving and being born, surrounded and becoming food; antlion eating, looking at the randomly drawn neighbor, catching the yellow ant, surrounded, spitting out the pebble, being born; spider appearing, leaving through the edge, by sight and by scent, chasing and eating, giving up, surround and revenge, fleeing, death throes and food, laser, player's spider), including the order and number of random draws where the spec determines it.
- **Against the original** (QEMU bench, cases in `tests/referencias/casos/`, saves in `tests/referencias/saves/I0n_INICIO.ANT` = state right after loading and `I0n_FIM.ANT` = final state): I-01 (caterpillar: head from (7, 58) to (14, 58) at tick 140), I-02 (antlion eats the worker placed at the center; at tick 200 it went back to waiting, `_AntsEatenByLions = 1`), I-03 (spider with `_DeathCnt = 40` at (100, 10): at tick 130, `_DeathCnt = 0` and 0x4A on the cell). All checks passed (2026-10-17). The `caso_i0n_*` tests run the port from the initial state up to the same `_GameTime` and compare what does not depend on the S generator: caterpillar (fields, `_PillarMap` and the map row), antlion (state, frame, `_AntsEatenByLions`, center) and death throes + the spider's cell. All equal.
- **Tick vectors** (`tests/referencias/ticks/`, [testes-tick.md](testes-tick.md)), ticks 79–83: starting from the "state before", antlions, spider (walking, without turning) and caterpillar come out equal to the "state after". Since the yard and water draw before the enemies in the tick, the S seed at the moment of the enemies is not known; comparing the draws requires the other parts of the tick.

- **Tick vectors of the fifth round** (2026-10-18, `tools/cenarios_regras.py` from the `rico` snapshot, [testes-tick.md](testes-tick.md#fifth-round-2026-10-18-unverified-rules-from-the-review)), all 307/307 with S, R and health equal: spider surrounded by 9 blacks (`aranha_cerco`: state 5, `_DeathCnt = 500`, message 10046, revenge 2 → 3 and `_MeSMode = 7`), by 6 (`aranha_foge`: state 4) and with `_SpidRevenge = 4` (`aranha_vinganca`: `_MeSMode = 8`); scent (`aranha_faro`: `_SFoundAnt` from back to front, chase, `_EatCnt = 11`, `_DeadAntHere`); laser (`aranha_laser`: two kills by `_SpiderScan` in a fan of blacks); antlion surrounded (`leao_cerco`: 7 neighbors, 10050, center 0x3F, list shifted); player's spider through the original's interface (`aranha_jogador`: X key and click on the spider, walk, run, target mode with `_MeSMode = 6` and eating with `_EatCnt = 50`); the player's spider surrounded (`aranha_jogador_cercada`, `_YellowDeath(3)`) and leaving the map (`aranha_jogador_fora`, `_YellowDeath(4)`); the yellow ant eaten by the antlion (`amarela_morte_leao`, `_YellowDeath(2)`). Finding: **the surround does not run for the player's spider with `_SMode` 0 or 1** — player control (2.7) replaces the rest of `_MoveSpider`, as section 2.2 already said; it can only be surrounded while chasing or eating.
- **Resolved on 2026-10-18 (drawing within the tick, [rng.md](rng.md)); history:** the yellow ant eaten by the spider (`amarela_morte_aranha/1793`, `_YellowDeath(1)`) gave 294/307, now 307/307. Cause isolated with breakpoints on `_SRand1` and on the drawing functions (`cenarios_regras.py contar_aranha`): the original draws 4 extra S steps, `S1(4)` from `_DrawPalps` in three drawings of the edit window within the tick (the one from `_GotoMyAnt` at death, the one before the `_AnimYellowFight` loop and the one of the 1st frame; the spider is in view and with `_SMode` ≥ 2) and the `S1(2)` from `_SpiderDialog` ("Events" option). The port does not draw within the tick ([rng.md](rng.md), "Outside"). It is not a rule error; the death by the antlion, without the spider in view, matches (control count `contar_leao`).

Not verified against the original: the drawing of the laser (`_DoLaserFire`). **The spat-out pebble was checked on 2026-10-19** (vector `leao_pedrinha`, 307/307): pebble 0x51 written at the center (30, 25) of the waiting antlion and S seed 0x02cd chosen in the port (`buscar_semente ... LionListM[0] 1`) for `S1(512)` = 0; at tick 1711 the original put the pebble at (28, 25) (2·D8[d] from the center), the center became 0x39, M 0 → 1, S = 25, as in 1.3.

## Gaps

- Walking backwards (`_RevSpider`) is not turned on by any routine seen [open].
- `_DoLaserFire` and the leg animation (`_Scycle`) are drawing only; the random draws inside `_DrawSpider` consume the S generator ([rng.md](rng.md)) and need to be measured.
- Boy, dog, cat, bird, lawnmower, rain and insecticide: a document of their own.
- Food falling in the house when the spider dies inside the house (`_DropFoodA` with the fall table).
- Checked by running the original: the caterpillar walking (I-01), the antlion eating an ant (I-02) and the spider in death throes becoming food (I-03), tick vectors 79–83 and, since 2026-10-18, surrounding of the spider and of the antlion, fleeing, revenge, scent, laser, player's spider (walking, running, target mode, surrounded, off the map) and yellow ant deaths 2, 3 and 4 (section "Verified"). The spat-out pebble matches since 2026-10-19 (`leao_pedrinha`). Death 1 (by the spider), which diverged only because of the drawing draws within the tick, matches since 2026-10-18 (vector `amarela_morte_aranha`, [rng.md](rng.md) "Drawing within the tick").
