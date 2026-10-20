*Leia em português: [superficie.md](../../pt-BR/spec/regras/superficie.md)*

# Surface: rules for the yard and house ants (list A)

This document specifies, for a 1:1 reimplementation, everything that happens to the ants of **list A** (surface) within a tick: the `_DoAntSimA` loop, each mode, the direction functions, fighting, death, nest entry, food, the red and black strategy and the red leader ant. The yard creatures (antlion, spider, caterpillar, pill bug) are in [inimigos.md](inimigos.md); the yellow ant and the controls in [formiga-amarela.md](formiga-amarela.md). The order of the subroutines within the tick is in [simulacao-visao-geral.md](simulacao-visao-geral.md); lists, T encoding and mode tables in [formigas.md](formigas.md); scent maps in [feromonios.md](feromonios.md); generators in [rng.md](rng.md).

## Evidence convention

Same as in [simulacao-visao-geral.md](simulacao-visao-geral.md#convenção-de-evidência). Summary:

- **Fact seen in the code:** Ghidra pseudocode in `re_trabalho/decomp/Code*.c`, cited as `selector:offset` and the name from `SIMANTW.SYM`. Variable addresses in the `logical segment:offset` format of `symbols.tsv` (8 = selector `1038`, 9 = `1040`, 10 = `1048`/DGROUP).
- **Verified fact:** checked in `SIMANTW.EXE` (tables) or in the save `tests/referencias/saves/QUICK2.ANT`, with the method described.
- **Fact seen in the disassembly:** when the pseudocode was ambiguous, the bytes of `SIMANTW.EXE` were disassembled (capstone, 16 bits; seg. 6 = selector `1028`, in the file at 385024 = 0x5E000).
- **[hypothesis]**, **[open]**: as in the [README](../README.md).

No decompiled excerpt is reproduced: everything is in prose, tables and our own pseudocode.

### Notation used in the pseudocode

| Notation | Meaning |
|---|---|
| `X[i]`, `Y[i]`, `T[i]`, `M[i]`, `S[i]` | fields of slot `i` of list A (`_AlistX/Y/T/M/S`, see [formigas.md](formigas.md#2-listas-paralelas)) |
| `Vida(x, y)` | `_LifeA[x·64 + y]` |
| `Mapa(x, y)` | `_MapA[x·64 + y]` |
| `cor(T)` | `T & 0x80` (0 = black, 0x80 = red) |
| `grupo(T)` | `(T & 0x78) >> 3` |
| `dir(T)` | `T & 7` |
| `base(T)` | `T & 0xF8` (color + group; the direction is set again afterwards) |
| `D8[d]` | offset `(_Dx8[d], _Dy8[d])` (`8:0000`/`8:0008`) |
| `Giro[d][k]` | `_TurnTab[d·8 + k]` (`8:0024`, table in [formigas.md](formigas.md#7-movimento-e-escolha-de-direção)) |
| `Barreira` | `_Barrier` (`9:7604`): 0x50 in the yard, 0x90 in the house |
| `Casa` | `_TERRAINset` (`9:9b6e`) = 1 |
| `S1(n)`, `S2()`, `S4()`, `S8()`, `S16()`, `S32()`, `S128()`, `S256()` | `_SRand1(n)`, `_SRand2()` … of the S generator ([rng.md](rng.md)); each call is **one** step |
| `R(n)` | `_RRand(n)` of the R generator |
| `CelCheiro(x, y)` | cell `(x >> 1)·32 + (y >> 1)` of the 64 × 32 maps |
| `Alarme(x, y)`, `NinhoB/R(x, y)`, `TrilhaB/R(x, y)` | `_PherMapA`, `_PherMapBN/RN`, `_PherMapBT/RT` at `CelCheiro(x, y)` |

**Golden rule of determinism:** every `S*`/`R` call listed below happens exactly in the order and under the conditions written. Calls conditioned by short-circuit (`A and B`) only happen when the preceding condition is true; this is explicit in each case.

## 1. The `_DoAntSimA` loop (`1028:04d8`)

**Fact seen in the code.** Our own pseudocode:

```
_Tindex (9:9b6a) = _ListIndexA
while _Tindex > 0:
    _Tindex -= 1 ; i = _Tindex
    r = S256()                                   ← for EVERY slot, including T = 0
    if r == 0 and T[i] ≠ 0:
        health = (_HealthB if cor(T[i]) = 0, else _HealthR)
        if health < S32():                       ← S32 is only drawn here
            _DeadAntHere(X[i], Y[i], cor(T[i]))  (section 6.4)
            T[i] = 0
            _BAntsExpired (9:9b26, 32 bits) += 1  ← also for red ants
    if T[i] ≠ 0:
        count the mode: _TemBModePop[M[i]] += 1 (black) or _TemRModePop[M[i]] += 1 (red)
        dispatch by M[i] (table below)
```

| M | Action | Section |
|---|---|---|
| 0 | `_DoRandAntA` | 3.1 |
| 1, 4 | `_DoToNestAnt` | 3.3 |
| 2 | `_DoForageAnt` | 3.4 |
| 3 | `_DoReturnFoodAnt` | 3.5 |
| 5 | `_DoDigOutAntA` | 3.6 |
| 6 | `_DoRecruitAnt` | 3.7 |
| 7 | `_DoAttackAnt` | 3.8 |
| 8 | egg on the surface (inlined; same as `_SimEggA`) | 3.9 |
| 9 | queen on the surface (inlined; same as `_SimQueenA`) | 3.10 |
| 10 | `_DoFightA` | 6.3 |
| 11 | `_DoToAlarm` | 3.11 |
| 12 | defend the nest (inlined; same as `_DoDefendNest`) | 3.12 |
| 13 | rest (inlined; same as `_DoRestAnt`) | 3.13 |
| 14 | pre-flight (inlined; same as `_DoRepoLoit`) | 3.14 |
| 15 | `_DoRepoExit` | 3.15 |
| 16 | flight (inlined; same as `_DoRepoFly`) | 3.16 |
| 17, 18 | **nothing** (there is no case in the `switch`; the ant stays still and is only counted) | — |
| 19 | `_DoRedInitiator` | 9.2 |

The dispatch does not look at the color: black and red ants use the same functions; the color only changes which scent map is read/written and who is the enemy.

## 2. Common pieces

### 2.1 Hole test ("enters the nest")

**Fact seen in the code** (repeated at the start of almost every mode). The ant is on a hole if `(x, y)` is inside the map (`_IsValidA` `1020:9c02`: 0 ≤ x < 128, 0 ≤ y < 64) and:

- yard: `Mapa(x, y) == 0x50`;
- house: `0x80 ≤ Mapa(x, y) < 0x90`.

If it is, it calls `_GoInNest(x, y, i)` (section 7) and the mode ends without any draw. These do this test right at the start: modes 0, 0', 1/4, 2, 3, 6, 7, 11 and 13; modes 12, 14 and 15 do it inside the function they call (`_DoToNestAnt` or `_DoRandAntAA`). These do **not**: 5 (`_DoDigOutAntA`), 8, 9, 10, 16 and 19.

### 2.2 Turn without movement

When the ant cannot or does not want to walk, it turns: `T[i] = base(T) | Giro[dir(T)][S8()]` and `Vida(x, y) = T[i]`. It is always the ant's **original** direction (before the step's direction draw) that indexes the table. One exception (`_DoRecruitAnt`, section 3.7) uses the plain drawn direction, without the table.

### 2.3 The standard step

**Fact seen in the code.** Each moving mode chooses a direction `d`, computes the destination `(nx, ny) = (x, y) + D8[d]` and handles the destination in this order (each mode's variations are in table 2.4):

1. **Food** (only in the modes that pick up food; section 8.1): destination is food (`_IsItFood`: yard 0x48–0x4B; house 0x18–0x27) **and** `grupo(T)` is 2 (worker) or 6 (soldier) → the ant **does not walk**: `T[i] = d | base(T) | 8` (group 2→3 or 6→7), `Vida(x, y) = T[i]`, `M[i] = 3`, `_PickupFoodA(nx, ny)`, `S[i] = 200`. Done. If the destination is food but the group is not 2/6, the barrier test is **skipped** and it proceeds to step 3.
2. **Barrier:** `Mapa(nx, ny) > Barreira` (signed comparison, unsigned tile value) → turn (2.2). Done.
3. **Free cell** (`Vida(nx, ny) == 0`): walks. `Vida(nx, ny) = d | base(T)`; `T[i]` likewise; `Vida(x, y) = 0`; `X[i], Y[i] = nx, ny`. Then, if the mode deposits scent: if `S[i] > 0`, `S[i] -= 1` and deposits `S[i]` on the indicated map, with `c = max(c, S)` (`_JamScent*`, [feromonios.md](feromonios.md)).
4. **Yellow ant** at the destination (`Vida` = 0xFF or 0xFE, `_IsYellowAnt` `1020:5720`):
   - color different from the yellow ant's (`(T ^ _MeColor) & 0x80`, `_MeColor` = `10:ce98`) → `_YellowFight(1, i)` ([formiga-amarela.md](formiga-amarela.md)). Done.
   - same color: in the modes with trophallaxis, if `_MeWantFood` (`9:9af2`) = 1 (in mode 6: ≠ 0), the ant turns towards the yellow ant (`T[i] = d | base(T)`, `Vida(x, y) = T[i]`) and calls `_DoTroph(x, y, d)` (the yellow ant eats; see [formiga-amarela.md](formiga-amarela.md)). Afterwards, always, turn (2.2).
5. **Ant of the same color** → turn (2.2); in some modes, also draws a new mode (`M[i] = _GetNewMode(grupo(T), T)`, section 9.3), **after** the turn.
6. **Enemy** (different color) → **fight** (6.1). Done.

### 2.4 Variations by mode

| Mode | Function | Direction | Picks up food (1) | Scent when walking (3) | `_DecTSmell` at destination | Trophallaxis (4) | Same color (5) |
|---|---|---|---|---|---|---|---|
| 0 | `_DoRandAntA` | `_GetRandDir` | yes | own color's nest | yes | yes (`= 1`) | turn + new mode |
| 0' | `_DoRandAntAA` | `_GetRandDir` | no | no | no | no | turn |
| 1, 4 | `_DoToNestAnt` | `_GetNestDir` (own color) | yes | own color's nest | no | yes (`= 1`) | turn |
| 2 | `_DoForageAnt` | `_GetForageDir` | yes | own color's nest | yes | yes (`= 1`) | turn |
| 3 | `_DoReturnFoodAnt` | `_GetNestDir` (own color) | no | own color's **trail** | no | — | — (see 3.5) |
| 6 | `_DoRecruitAnt` | alarm / `_GetDefendDir` / `_GetRedDefendDir` | no | no | no | yes (`≠ 0`) | turn |
| 7 | `_DoAttackAnt` | `_GetNestDir` (**enemy** color) | no | no | no | no | turn |
| 11 | `_DoToAlarm` | `_GetAlarmDir` | no | no | no | no | turn |
| 19 | `_DoRedInitiator` | `_GetRedBestDirs` | no | no | no | no (no color test) | turn (`_RandTurn`) |

"0'" = `_DoRandAntAA` (`1028:1234`), the "silent" walk used by modes 12, 14 and 15.

### 2.5 Direction functions

**Fact seen in the code** (all in `SIMTWO_MODULE`, selector `1030`). All take `(x, y, current_dir)` and return a direction 0–7 (or a negative code, when indicated).

#### 2.5.1 Edge handling ("Bounce" pattern)

`_Bounce` (`1030:12ec`), `_GetRandDir`, `_GetNestDir`, `_GetAlarmDir`, `_GetDefendDir` and `_GetRedDefendDir` start with the same test. If the ant is on an edge of the surface, it draws **one** value and returns a direction pointing inwards; otherwise, it draws nothing and proceeds to the function's own rule:

| Position | Draw | Possible directions (result = base + draw, modulo 8) |
|---|---|---|
| x = 0, y = 0 | `S1(3)` | 2, 3, 4 |
| x = 0, y = 63 | `S1(3)` | 0, 1, 2 |
| x = 0, other y | `S1(5)` | 0, 1, 2, 3, 4 |
| x = 127, y = 0 | `S1(3)` | 4, 5, 6 |
| x = 127, y = 63 | `S1(3)` | 6, 7, 0 |
| x = 127, other y | `S1(5)` | 4, 5, 6, 7, 0 |
| y = 0, other x | `S1(5)` | 2, 3, 4, 5, 6 |
| y = 63, other x | `S1(5)` | 6, 7, 0, 1, 2 |

`_Bounce` returns `1 + direction` (or 0 off the edge); the others return the direction.

`_GetForageDir` has a **different** edge table (fact seen in the code):

| Position | Draw | Direction |
|---|---|---|
| x = 0, y = 0 | none | 3 |
| x = 0, y = 63 | none | 1 |
| x = 0, other y | `S1(3)` | 1, 2, 3 |
| x = 127, y = 0 | none | 5 |
| x = 127, y = 63 | none | 7 |
| x = 127, other y | `S1(3)` | 5, 6, 7 |
| y = 0, other x | `S1(3)` | 3, 4, 5 |
| y = 63, other x | `S1(3)` | 7, 0, 1 |

#### 2.5.2 Helpers `_GetDir` and `_GetDis`

- `_GetDir(x1, y1, x2, y2)` (`1020:10cc`): returns `1 + direction` from (x1, y1) to (x2, y2) looking **only at the sign** of dx and dy, or 0 if the points coincide. dx = 0: dy < 0 → 1 (north), dy > 0 → 5 (south). dx > 0: dy < 0 → 2, dy = 0 → 3, dy > 0 → 4. dx < 0: dy < 0 → 8, dy = 0 → 7, dy > 0 → 6.
- `_GetDis(x1, y1, x2, y2)` (`1020:1122`): `dx² + dy²` (32-bit multiplication; the callers compare the 16-bit result, enough for the map).
- On the scent maps, the 8 neighbors of a cell `(cx, cy)` are `((cx + Dx8[k]) & 63, (cy + Dy8[k]) & 31)`, that is, the map "wraps around" (the real edge has already been handled before).

#### 2.5.3 `_GetRandDir` (`1030:0f72`)

Edge (2.5.1); otherwise `Giro[dir][S8()]`.

#### 2.5.4 `_GetForageDir(x, y, dir, T)` (`1030:0ab0`)

```
edge: own table (2.5.1)
current = Trilha_cor(T)(x, y)
junk    = S8()                   ← ALWAYS drawn, result discarded if there is a trail
best = 0 ; k_best = 0
for k = 0..7: v = trail of neighbor k ; if v > best: best = v, k_best = k   (tie: lowest k)
if best == 0:  return Giro[dir][S8()]           ← second draw
if best < current: return −1                    ← "end of trail"
return Giro[dir][k_best]
```

Fact seen in the code. The first `S8()` is consumed even when it serves no purpose.

#### 2.5.5 `_GetNestDir(x, y, dir, T)` (`1030:0c30`)

Color = `T >> 7` (0 black, 1 red). Edge (2.5.1). Then:

```
current = Ninho_cor(x, y)
if current == 0:
    k = _GetDir(x, y, _LastNewHole_cor)          (_LastNewHoleB 8:835a / R 8:835e)
    if k ≠ 0 and S4() ≠ 0: return Giro[dir][k − 1]  ← S4 only if k ≠ 0
    return Giro[dir][S8()]
else:
    best = 0; k_best = 0; for k = 0..7: strictly greatest value (tie: lowest k)
    if S2() ≠ 0: dir = Giro[dir][k_best]        ← double turn (up to 90°)
    return Giro[dir][k_best]
```

Remarks (fact seen in the code): it does not compare the neighbor with the current cell (the ant always goes to the greatest neighbor, even at the peak); if all neighbors are 0 but the current cell is not, `k_best = 0` (north). [feromonios.md](feromonios.md) describes the "turn limited half of the time"; the exact behavior is: always limited, with a double turn when `S2() ≠ 0`.

#### 2.5.6 `_GetAlarmDir(x, y, dir)` (`1030:0e54`)

Edge; otherwise, greatest alarm neighbor (tie: lowest k). If all are 0: `Giro[dir][S8()]`; otherwise `Giro[dir][k_best]` (one turn step).

#### 2.5.7 `_GetDefendDir(x, y, dir)` (`1030:1026`) — follows the yellow ant

Edge; then, according to the yellow ant's plane (`_MePlane` `10:ce80`):

| `_MePlane` | Rule |
|---|---|
| 2 | returns `_GetNestDir(x, y, dir, 0)` (climbs the **black** nest scent) |
| 3 | returns `_GetNestDir(x, y, dir, 0x80)` (red nest) |
| 0 (yard view) | returns `dir` (goes straight) |
| 1 | see below |

On the surface: if `_ChaseSpid` (`9:72ec`) = 1, `k = _GetDir(x, y, _SpidX >> 4, _SpidY >> 4)`. Otherwise, if `_GetDis(x, y, _FuzLocX, _FuzLocY) ≤ _ModePopB[5] >> 1` (half the number of black ants in Defense, `9:9e7a`), `k = S1(8) + 1` (wander); otherwise `k = _GetDir(x, y, _FuzLocX, _FuzLocY)`. Result: `dir` if k = 0, otherwise `Giro[dir][k − 1]`. `_FuzLocX/Y` come from `_GetStrategy` (9.1).

#### 2.5.8 `_GetRedDefendDir(x, y, dir)` (`1030:1194`) — follows the red leader

Same as 2.5.7, replacing `_MePlane` with `_RedPlane` (`9:7606`), the target with (`_RedLocX` `9:80a6`, `_RedLocY` `9:80ac`) and the radius with `_ModePopR[5] >> 1` (`9:a08e`). There is no spider case. The comparison is `radius < distance` → `_GetDir`; otherwise `S1(8) + 1`.

## 3. The modes, one by one

In all of them: `x, y, T` are the slot's values at the start of the mode; "turn" is section 2.2; "fight" is section 6.1. Each numbered list gives the exact order of tests and draws.

### 3.1 Mode 0 — Bored: `_DoRandAntA` (`1028:0e66`)

1. Hole → `_GoInNest`. Done.
2. `d = _GetRandDir(x, y, dir)`.
3. Standard step (2.3) with: picks up food; barrier; free → walks and deposits own color's nest scent; then `_DecTSmell(nx, ny, cor)` (own color's trail −1 if > 0); then, if `S8() == 0` **and** group is 2 or 6 → `M[i] = 2` (Foraging). The `S8()` is drawn whenever the ant walks, even if the group is not 2/6.
4. Yellow ant: different color → `_YellowFight`; same color → trophallaxis if `_MeWantFood == 1`, then turn.
5. Same color → turn, then `M[i] = _GetNewMode(grupo, original T)`.
6. Enemy → fight.

### 3.2 Mode 0' — `_DoRandAntAA` (`1028:1234`)

Like 3.1, but: does not pick up food (a destination with food goes through the barrier test normally), does not deposit scent, does not call `_DecTSmell`, does not draw a mode change when walking; yellow ant of the same color and ant of the same color → turn only.

### 3.3 Modes 1 (Nesting) and 4 (Dig In): `_DoToNestAnt` (`1028:1676`)

1. Hole → `_GoInNest`.
2. `d = _GetNestDir(x, y, dir, T)` (own color's nest scent).
3. Standard step with: picks up food (yes, even when returning to the nest; becomes Got Food); barrier; free → walks and deposits nest scent; yellow ant (trophallaxis if `= 1`); same color → turn only; enemy → fight.

### 3.4 Mode 2 — Foraging: `_DoForageAnt` (`1028:1e42`)

1. Hole → `_GoInNest`.
2. `S32() == 0` → `M[i] = 13` (Resting). Done.
3. `Alarme(x, y) ≠ 0` → `M[i] = 11` (Alarmed). Done.
4. Group ≠ 2 and ≠ 6 → `M[i] = _GetNewMode(grupo, T)`, `S[i] = 0`. Done.
5. `d = _GetForageDir(x, y, dir, T)`.
6. If `d < 0` (end of trail): `S8() == 0` → `M[i] = _GetNewMode(grupo, T)`; otherwise `M[i] = 0`. `S[i] = 0`. Then `_DecTSmell(x >> 1, y >> 1, cor)` (see quirk P1). Done.
7. Food at the destination → picks it up (M = 3, S = 200).
8. Barrier → turn; then `S16() == 0` → `M[i] = _GetNewMode(grupo, T)`, `S[i] = 0`.
9. Occupied: yellow ant of another color → `_YellowFight`; yellow ant of the same color → trophallaxis if `= 1`, turn; same color → turn; enemy → fight.
10. Free → walks, deposits nest scent (S), `_DecTSmell(nx, ny, cor)`.

**Quirk P1 (fact seen in the disassembly, `1028:1fa6`–`1fb8`):** in the "end of trail" case, the game passes `(x >> 1, y >> 1)` to `_DecTSmell`, which in turn divides by 2 again. The decremented trail cell is `(x >> 2, y >> 2)` on the 64 × 32 map, not the ant's. Reproduce it this way.

### 3.5 Mode 3 — Got Food: `_DoReturnFoodAnt` (`1028:1cb4`)

1. Hole → `_GoInNest` (the food enters the nest with the ant).
2. `d = _GetNestDir(x, y, dir, T)`.
3. Barrier → turn. Done.
4. **Otherwise it always walks**, without looking at `Vida(nx, ny)`: writes `Vida(nx, ny) = d | base(T)` over whoever is there, clears `Vida(x, y)`, updates X/Y; if `S > 0`: `S −= 1` and deposits `S` on the own color's **trail** (`_JamScentBT/RT`).

**Quirk P2 (fact seen in the code):** ants carrying food pass through other ants (including enemies and the yellow ant), overwriting the life map; there is no fight. The run-over ant stays in the list and returns to `_LifeA` when it moves.

### 3.6 Mode 5 — Dig Out: `_DoDigOutAntA` (`1028:1480`)

Exact order (fact seen in the code):

1. `r = S8()`; `d = Giro[dir][r]`.
2. `b = _Bounce(x, y)` (draws only on the edge); if `b ≠ 0`, `d = b − 1`.
3. Group ≠ 5 and ≠ 9 (not carrying dirt) → `M[i] = _GetNewMode(grupo, T)`, `S[i] = 0`. Done.
4. `S8() == 0` → **drops the dirt**: `T[i] −= 0x18` (5→2, 9→6; the dirt vanishes, no tile is created), `M[i] = _GetNewMode(old group, old T)` (for groups 5/9 the table returns 5 again; on the next tick step 3 corrects it), `S[i] = 0`, `Vida(x, y) = T[i]`. Done.
5. Barrier at the destination → turn (`Giro[dir][S8()]`).
6. Destination occupied (by anyone) → turn. There is no fight nor trophallaxis in this mode.
7. Free → walks; if `S > 0`, `S −= 1` and deposits own color's nest scent.

There is no hole test in this mode.

### 3.7 Mode 6 — Defense (recruited): `_DoRecruitAnt` (`1028:22a8`)

1. Hole → `_GoInNest`.
2. Direction: if `Alarme(x, y) ≠ 0` → `_GetAlarmDir`; otherwise, black (`T < 0x80`) → `_GetDefendDir`; red → `_GetRedDefendDir`.
3. Barrier → **`T[i] = base(T) | S8()`** (plain drawn direction, without `_TurnTab`), `Vida(x, y) = T[i]`. Done.
4. Free → walks (no scent).
5. Occupied by a non-yellow ant: same color → turn (with table); enemy → fight.
6. Yellow ant: different color → `_YellowFight`; same color → trophallaxis if `_MeWantFood ≠ 0`; then turn with table.

`S[i]` is not used here (recruitment writes 0; `_ForceModeA` would write the encoded position of the yellow ant, see 9.4).

### 3.8 Mode 7 — Attack: `_DoAttackAnt` (`1028:2a40`)

1. Hole → `_GoInNest` (any hole it steps on, including its own nest's).
2. If `_IsCarryCaste[grupo]` (`8:0074`, words) = 1, the load is discarded: group ← `_UnCarryCaste[grupo]` (`8:0094`). The new T is only written at the end (steps 4–6).
3. `d = _GetNestDir(x, y, dir, T ^ 0x80)` (nest scent of the **enemy** color; without scent, it points 3/4 of the time to the last enemy hole).
4. Barrier → turn.
5. Free → walks (no scent).
6. Occupied: yellow ant of another color → `_YellowFight`; enemy → fight; same color (or yellow ant of the same color) → turn.

Tables read from `SIMANTW.EXE` (verified fact, segment 8 in the file at 0x9600):

| group | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10–15 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `_IsCarryCaste` | 0 | 1 | 0 | 1 | 0 | 1 | 0 | 1 | 0 | 1 | 0 |
| `_UnCarryCaste` | 0 | 2 | 2 | 2 | 4 | 2 | 6 | 6 | 8 | 6 | same as the group |

### 3.9 Mode 8 — egg on the surface (inlined in `_DoAntSimA`; same as `_SimEggA` `1028:0a1c`)

`Vida(x, y) = T`; `S1(200) == 0` → `T[i] = 0`, `Vida(x, y) = 0` (the egg vanishes). Eggs do not hatch on the surface.

### 3.10 Mode 9 — queen on the surface (inlined; same as `_SimQueenA` `1028:0a74`)

`Vida(x, y) = T`. Only the **tail** (`(T & 0x7F) > 0x67`, group 13) is checked: the head must be at `(x, y) + D8[dir(T)]` with `Vida = T − 8`, or, alternatively, `_FindInAList` must find any ant in that cell. If neither, the tail is removed (`T[i] = 0`, `Vida = 0`). The head is never removed here (correction to [formigas.md](formigas.md#3-codificação-do-tipo-t), which says "the remaining one is also removed"). No draw.

### 3.11 Mode 11 — Alarmed: `_DoToAlarm` (`1028:1a0a`)

1. Hole → `_GoInNest`.
2. If `Alarme(x, y) == 0` **and** `S4() == 0` (the `S4` is only drawn with zero alarm) → `Vida(x, y) = T`, `M[i] = _GetNewMode(grupo, T)`. Done.
3. `d = _GetAlarmDir(x, y, dir)`.
4. Barrier → turn. Free → walks (no scent). Yellow ant of another color → `_YellowFight`. Enemy → fight. Same color or yellow ant of the same color → turn.

### 3.12 Mode 12 — Defend Nest (inlined; same as `_DoDefendNest` `1028:0df6`)

If `Ninho_cor(x, y) < 0x6E` (110) → `_DoToNestAnt`; otherwise `_DoRandAntAA`. No draw of its own.

### 3.13 Mode 13 — Resting (inlined; same as `_DoRestAnt` `1028:0b76`)

1. Hole → `_GoInNest`. Done.
2. `S4() == 0` → `M[i] = 2` (Foraging).
3. Otherwise, if the "Silly" option (`_OptionStates[5]`, `8:85fc`) = 1 → `_RestBalloons(x, y, 1)` (balloon; does not draw).

The ant does not walk and does not write `Vida`.

### 3.14 Mode 14 — Pre-Flight (inlined; same as `_DoRepoLoit` `1028:0c1e`)

1. `S2() == 0` → `_DoToNestAnt(i)`; otherwise `_DoRandAntAA(i)`.
2. Reads the color from the **current T** (which may have become 0 if the ant entered the nest); if `_FlyAwayB` (`9:7c44`, color 0) or `_FlyAwayR` (`9:8078`, red color) > 100 → `M[_Tindex] = 15`.

### 3.15 Mode 15 — Take off: `_DoRepoExit` (`1028:0c7a`)

1. `Ninho_cor(x, y) < 100` → `_DoToNestAnt`; otherwise `_DoRandAntAA`.
2. Counter `F = _FlyAwayB` or `_FlyAwayR` according to the color of `T` (re-read): `F == 0` → nothing; `F == 1` → `M = 16`; otherwise `S1(F) == 0` → `M = 16` (the draw only happens with F ≥ 2).

### 3.16 Mode 16 — Flying (inlined; same as `_DoRepoFly` `1028:0d4a`)

`S32() == 0` **and** (black and `_SwarmCntB` (`9:807a`) < 50, or red and `_SwarmCntR` (`9:9c26`) < 50) → the ant leaves the map (`T[i] = 0`, `Vida = 0`). Only in the Full Game (`_CurGameType` `9:80b4` = 2): `_SwarmCnt*` += 1 and `S16() == 0` → `_QueenStorageB` (black; and redraws the gauge) or `_QueenStorageR` += 1. The `S16` is only drawn in the Full Game. The flying ant does not move on the map.

### 3.17 Modes with no effect: 17 and 18

There is no case in the `switch`: the ant is counted in `_TemB/RModePop` and does nothing (fact seen in the code). Since these modes only appear in the nest ([formigas.md](formigas.md#5-modos)), an ant that leaves the nest with M = 17 would stay still on the surface until it dies or is recruited [hypothesis: rare case].

## 4. Mating flight (summary of who decides)

- `_FlyAwayR` is armed by `_GstrR` (9.1): if `_FlyAwayR == 0` and red males + new queens (`_CastePopR[3] + [4]`, `10:aca6`/`10:aca8`) > 20 → `_FlyAwayR = 200`. `_FlyAwayB` is not armed in any routine of this part [open: probably by the player — life cycle, parallel document].
- Both counters drop by 1 per tick in `_DoAntSim` ([simulacao-visao-geral.md](simulacao-visao-geral.md), section 5, step 13).
- Males and new queens are born with M = 14 (`_CasteModeTabB`), go to 15 with the counter > 100, to 16 when it reaches 1 (or by the 1/F draw) and leave the map through mode 16.

## 5. Mode counting and `_MakeRedInitiator`

See 9.2. The increment of `_TemB/RModePop[M]` happens in 1 (before the dispatch), for every live ant of list A.

## 6. Fighting and death

### 6.1 Start of a fight (inlined in the modes; same as `_StartFightA` `1028:266a`)

**Fact seen in the code.** When ant `i` (attacker, "b") tries to enter the cell `(nx, ny)` of an enemy:

```
Tb = T[i] ; T[i] = 0 ; Vida(x, y) = 0          ← the attacker VANISHES from the surface
j = _FindInAList(nx, ny)                        (last slot with equal X, Y and T ≠ 0)
if j < 0: done                                   ← the attacker vanished without a fight
v = _GetWinner(T[j], Tb)                         (the one in the cell is "a")
T[j] = 0x70 + (v & 0x80) ; Vida(nx, ny) = T[j]  ← fight cloud with the winner's color
M[j] = 10 ; S[j] = v                             ← S keeps the winner's T
_AlarmHere2(nx, ny, 40)                          ← alarm = max(alarm, 40)
```

### 6.2 `_GetWinner(Ta, Tb)` (`1028:26f4`)

**Fact seen in the code; tables verified in `SIMANTW.EXE`** (DGROUP in the file at 0x20000):

```
if _Invincible (8:8a5c) == 1:
    _RAntsKilled (9:9e96, 32 bits) += 1
    return (Ta if cor(Ta) = 0, else Tb)          ← the black ant always wins; no draw
p = Matriz[Classe[grupo(Ta)]][Classe[grupo(Tb)]]
if R(10) < p: winner = Ta  else: winner = Tb             ← R generator
if cor(winner) = 0: _RedLost (9:a0e4) += 1 ; _RAntsKilled += 1
else:               _BlackLost (9:79e4) += 1 ; _BAntsKilled (9:99e0, 32 bits) += 1
return winner
```

`Classe` (`10:22c6`, 16 bytes) = `0 0 0 0 2 0 1 1 2 1 0 0 3 3 0 0`; `Matriz` (`10:22d6`, 4 × 4, chance in 10 of "a" winning) = rows `5 2 7 3`, `8 5 9 4`, `3 1 5 2`, `7 6 8 5` (classes 0 worker/egg, 1 soldier, 2 breeder, 3 queen). Same values as in [formigas.md](formigas.md#6-luta-morte-e-saúde).

The returned T is the winner's **complete** T (with load and direction); it is what goes back into the slot when the cloud dissolves.

### 6.3 Fight cloud: mode 10, `_DoFightA` (`1028:27e6`)

```
q = S1(7)                                       ← 0..6, animation frame
T[i] = base(T[i]) + q ; Vida(x, y) = T[i]
if S16() == 0:                                  ← the fight ends
    T[i] = S[i] (winner) ; Vida(x, y) = S[i]
    M[_Tindex] = _GetNewMode(grupo(S[i]), S[i]) ; S[i] = 0
    _DeadAntHere(x, y, winner's color)           ← corpse in the same cell
else, if "Silly" (8:85fc) == 1: _FightBalloons(x, y, 1)
```

**Quirk P3 (fact seen in the code):** the corpse is marked with the **winner's color**, and `_DeadAntHere` clears `Vida(x, y)` at the end. The winner stays in the list, in the same cell, with no mark on the life map until it moves again.

### 6.4 Corpse: `_DeadAntHere(x, y, cor)` (`1028:28c0`)

Ring of 100 positions (`_DeadX` `9:9c82`, `_DeadY` `9:9d76`, pointer `_DdPtr` `9:9ea8`). Fact seen in the code:

```
p = _DdPtr + 1 ; if p > 99: p = 0 ; _DdPtr = p
(ox, oy) = (_DeadX[p], _DeadY[p])               ← the oldest corpse
yard:
    if 0x10 ≤ Mapa(ox, oy) < 0x18: Mapa(ox, oy) = S16()     ← erases the old corpse (becomes ground)
    _DeadX[p], _DeadY[p] = x, y
    if Mapa(x, y) ≤ 0x17: Mapa(x, y) = (cor = 0 ? 0x10 : 0x14) + S4()
house:
    if 8 ≤ Mapa(ox, oy) < 0x18: Mapa(ox, oy) = (value − 8) >> 2   ← back to the floor
    _DeadX[p], _DeadY[p] = x, y
    v = Mapa(x, y) ; if v ≤ 3:
        black: Mapa = (v + 2)·4 + S1(2)           (0x08–0x15 depending on the floor)
        red: Mapa = v·4 + 10 + S1(2)
Vida(x, y) = 0
```

The `S16()` draw for the old corpse only happens if there is still a corpse there; the color ones (`S4`/`S1(2)`) only if the new tile is ground. `cor` is any value ≠ 0 for red (the spider passes 0 or 4, see [inimigos.md](inimigos.md)). Tile values: yard 0x10–0x13 black corpse, 0x14–0x17 red (completes [tiles-valores.md](tiles-valores.md), which left 0x10–0x17 open).

### 6.5 Natural death on the surface

Section 1. Colony health ≥ 32 prevents death (S32 ≤ 31). The dead ant becomes a corpse and counts in `_BAntsExpired`, even when it is red (fact seen in the code; probable oversight in the original).

## 7. Nest entry: `_GoInNest(x, y, i)` (`1028:257a`)

**Fact seen in the code.**

```
if x < 64:  list = B ; nest = black ; holes = _HoleMapB (8:82d2)
else:       list = R ; nest = red ; holes = _HoleMapR (8:8312)
if the list has ≥ 500: compact (_CompactListB/R)
if it still has ≥ 500: done                      ← the ant stays on the surface, nothing changes
add to the list: X = y, Y = 1, T = base(T) + 4 (facing down), M = M[i], S = S[i]
if holes[y] ≠ 0: _DigTileB/R(y, 1)              (guarantees the tunnel under the hole)
T[i] = 0 ; Vida(x, y) = 0
```

The nest is chosen by the hole's **column**, not by the ant's color: a black ant that steps on a red hole becomes an invader in the red nest, and vice versa. The yellow ant uses another limit (x < 65, [formiga-amarela.md](formiga-amarela.md)).

## 8. Food on the surface

### 8.1 `_PickupFoodA(x, y)` (`1020:0d18`)

Yard: tile 0x48 (last unit) → `Mapa = S16()` (ground); otherwise `Mapa −= 1`. House: if `value % 4 == 0` (last unit) → `Mapa = (value − 0x18) >> 2` (floor); otherwise `Mapa −= 1`. Then `_FoodA` (`9:9e84`) −1 if > 0. Fact seen in the code.

### 8.2 `_DropFoodA(x, y)` (`1020:0d86`)

Used by the yellow ant and by the dead spider. Yard: tile > 0x4A → fails (returns 0); tile < 0x48 → 0x48; otherwise +1. House: tile ≤ 3 → `(tile + 6)·4`; 8–0x17 (corpse) → `((tile − 8) >> 2 + 6)·4`; 0x18–0x26 → +1; other tile < 0x40 (4–7 and 0x27–0x3F) → the food **falls**; ≥ 0x40 → fails. Success: `_FoodA += 1`. Fact seen in the code.

**The fall** [code + disassembly of `1020:0dfe`–`0ea4`]: with d = `_DROPdir` (`9:9c66`), dx = byte `10:22be + d` and dy = byte `10:22c2 + d` (tables `[0, 1, 0, 0xFF]` and `[0xFF, 0, 1, 0]`), **added unsigned** (`mov al, …; sub ah, ah`). Starting from its own cell: if the tile is < 4, it gets `(tile + 6)·4`, `_FoodA += 1` and stops; otherwise it moves (dx, dy); stops if x leaves 0..127 or y leaves 0..63. Returns 1 even when the food vanishes. Since 0xFF adds 255, in directions 0 (tiles, "upwards") and 3 (right carpet, "to the left") the food leaves the map on the first step and **vanishes**; it only actually falls in directions 1 (left carpet, to the right) and 2 (kitchen, patches 9/36/37, downwards) [hypothesis: defect in the original (the code wants −1); behavior followed by the port]. `_DROPdir` is not in the save: after a Load it holds the value of the last house patch generated in the session (0 if none). **Port** (2026-10-18): `comum::largar_comida_superficie` + `comida_cai`, with `Estado::passo_queda` written when generating a house patch (table in `data/casa.json`, `direcoes_queda`); test `comida_cai_na_casa_pelo_dropdir`. **Checked on the test bench (2026-10-19, vectors `queda_casa_carpete_esq` and `queda_casa_carpete_dir`, testes-tick.md "Seventh round"):** the dying spider on a lint tile (0x3F/0x3E) drops 3 food units; on the left carpet (`_DROPdir` = 1, read from memory) each one falls to the right up to the first free carpet (106, 107 and 108, 44 = 0x24, `_FoodA` + 3); on the right one (`_DROPdir` = 3) all three vanish and `_FoodA` does not change — the unsigned 0xFF step is confirmed. **Kitchen (2) and tiles (0), checked on 2026-10-19 (eighth round, vectors `queda_casa_cozinha`, `queda_casa_ladrilhos` and `queda_casa_azulejo`):** no recipe of these patches generates a tile that falls (the kitchen only has 0 on the counter and ≥ 0x62 on the wall; the tiles, 0/1 and joints ≥ 0x5C), so the dead spider on the 0x62 tile **drops nothing** (`_DropFoodA` returns 0 for tile ≥ 0x40: the 3 food units are lost, `_FoodA` unchanged). With a 0x3F written into the cell (prepared in memory), in the kitchen (`_DROPdir` = 2, read) the 3 food units go down to the counter (rows 24, 25 and 26, `_FoodA` + 3); on the tiles (`_DROPdir` = 0) the 3 vanish on the first step (dy = unsigned 0xFF), as on the right carpet. In practice, then, 0 and 2 only act on a falling tile brought by another rule (none known) [hypothesis].

### 8.3 `_AddFood(n, sound)` (`1030:6a58`) and periodic feeding

The scattering algorithm (center, radius, sine/cosine from `_FracSineTab`) is in [geracao-mundo.md](geracao-mundo.md) (section 3.E), and applies the same during the game. During the game it is called by the periodic feeding (body of `_FeedAnts` `1028:0474`, inlined in `_DoAntSim` every 64 ticks): outside Experimental mode, if `_FoodA < _NextFeed` (`8:8a62`), `_AddFood(150, 1)` (plays a sound, center `(S128(), S64())`) and `_NextFeed = S1(50) + 1`. The order of the draws inside `_AddFood` is: center (`S128`, `S64`, or `S1(48)` in the initial case), radius `S8()`, and per unit `S256()` and `S1(radius)`. Writes `_LastFoodDrop` (`8:836a`), used by the red leader (9.2). Fact seen in the code. **Checked on the test bench (2026-10-19, vectors `alimentacao` and `alimentacao_casa`, `tools/cenarios_fecha.py`):** with `_NextFeed` = 30000 and `_Cycle` brought to the end of a 64-period on input, the tick calls `_AddFood(150, 1)`: in the yard (`rico`), center (85, 31), `_FoodA` 122 → 237, `_NextFeed` = 50; in the house (Full Game, patch (0, 4), left carpet), the food follows the house floor rule (geracao-mundo.md 3.E). Both 307/307 with S, R and life.

## 9. Strategy

### 9.1 `_GetStrategy` (`1030:0000`), `_GstrB` (`1030:01cc`), `_GstrR` (`1030:03c2`)

Exact order (fact seen in the code), every tick:

```
_ChaseSpid = 0
if _MePlane == 1:
    _FuzLocX = clamp(_MeLocX + S1(5) − 2, 0, 127)
    _FuzLocY = clamp(_MeLocY + S1(5) − 2, 0, 63)
    if _SpidOn ≠ 0 and _GetDis(_SpidX >> 4, _SpidY >> 4, _MeLocX, _MeLocY) < 100: _ChaseSpid = 1
_FuzLocX = _MeLocX                               ← always overwrites (the X draw is discarded)
_StrategicModeB = black ladder (simulacao-visao-geral.md §7; same computation as _GstrB)
_StrategicModeR = _GstrR()
_SetCasteProd() ; _SetModeProd()
```

`_GstrB` exists as a function, but `_GetStrategy` has the same logic inlined; there is no other caller. Verified in `QUICK2.ANT`: `_FuzLocX = _MeLocX = 34` with the yellow ant in the nest (`_MePlane = 2`) and `_FuzLocY = 0` (not updated off the surface).

`_GstrR` (S = `_HealthR`, P = `_RpopT`, D = `_TilesDugR`, Pb = `_BpopT`), in this order:

```
if _FlyAwayR == 0 and _CastePopR[3] + _CastePopR[4] > 20: _FlyAwayR = 200
if _JustToBeMean (9:78dc) ≠ 0: _JustToBeMean −= 1 ; return 0
if S < 10 and Pb < P/2 and Pb > 0 and _BlkQueens > 0: MEAN
if S < 30: return 5
if S < 50: return 4
if D < P: return 3
if D < 2P: return 2
normal = (P < 101 or Pb < 1 or _BlkQueens < 1 or P/3 ≤ Pb)
if not normal: MEAN                              ← no draw
if S32() ≠ 0: return 1
if P < 21: return 1
if P ≤ Pb: return 1
if S128() ≠ 0: return 1
MEAN:
    _JustToBeMean = S1(100) + 30
    music 0x2B0A ; message _AdviceStrs[5] (SHARED 4/1020, index 5 counting from 0: "The red ants are attacking!"),
    with parameter 120 ([hypothesis] duration in ticks)
    return 0
```

`_StartAttack` (`1030:050e`) does only the MEAN block (no caller found).

### 9.2 Red leader ant ("Avenger", mode 19)

**`_MakeRedInitiator` (`1028:967c`)**, called at the end of the tick if `_TemRModePop[19] < 1`:

```
_RedInitOn (8:8a64) = 0
if _BpopT ≥ 30:
    i = highest index of list A with T ≥ 0x80 (any red ant, including egg, queen or cloud)
    if it exists: T[i] = 0xB0 ; M[i] = 19 ; S[i] = 0 ; _RedTask (9:9d74) = 0 ; _RedInitOn = 1
```

`_LifeA` is not updated here; the new color/type appears when the leader moves (quirk P4).

**`_DoRedInitiator(i)` (`1028:96d4`)**:

```
_RedLocX, _RedLocY = X[i], Y[i] ; _RedPlane = 1
if _RedTask == 0:
    _UnRecruitRed()
    if _MePlane == 1 and S1(32) + 64 < _MeLocX and S1(10) < _ModePopB[5]:  ← short-circuit
        _RedTask = 2 ; _RecruitRed(_ModePopB[5])                          (hunt the yellow ant)
    else:
        (dx, dy) = _LastFoodDrop
        if dx < 31: if dy < 20: dy += 5 ; else if dy > 40: dy −= 5
        else: dx −= 5
        _RedDestX, _RedDestY (9:9bee / 9:9c22) = dx, dy
        n = _CastePopR[1] + _CastePopR[2] ; n = (n < 20 ? n >> 2 : n >> 3)
        _RecruitRed(n) ; _RedTask = 1
else if _RedTask == 2 and _MePlane == 1:
    _RedDestX, _RedDestY = _FuzLocX, _FuzLocY
d = _GetRedBestDirs(1, X[i], Y[i], _RedDestX, _RedDestY)
if d < 0: if S1(10) == 0: _RedTask = 0 ; Vida(x, y) = T[i]
else:
    empty destination → walks (no scent)
    yellow ant → _YellowFight(1, i)               ← no color test
    black ant (Vida < 0x80) → nothing (neither turn nor rewriting Vida)
    red → T[i] = base | _RandTurn(dir) ; Vida(x, y) = T[i]
```

`_RandTurn(d)` (`1028:2a22`) = `Giro[d][S8()]`. `_GetNewRedTask` (`1028:9940`) repeats the `_RedTask == 0` block and has no caller.

**`_GetRedBestDirs(plane, x, y, tx, ty)` (`1028:9a18`)**: fact seen in the code.

```
dist = _GetDis(x, y, tx, ty) ; if dist ≤ 0: return −1
free = −1 ; blocked = −2
for k = 0..7 (neighbor v = (x, y) + D8[k]):
    if _TileCanBeMovedOn(plane, v, 1, tx, ty, 0) and _GetDis(v, target) < dist:
        dist = _GetDis(v, target)                    ← the threshold drops with each improvement
        if _GetLife(plane, v) < 1 and _IsClearTile(plane, v): free = k
        else: blocked = k
return (free ≥ 0 ? free : blocked)
```

`_GetLife` returns −1 (0xFFFF) for an empty or off-map cell and the life map byte in the other cases (the yellow ant, 0xFF, counts as occupied). No draw. The same function, with a different passability test, is the yellow ant's `_GetMyBestDirs`.

### 9.3 New mode: `_GetNewMode(g, T)` (`1030:0910`), `_GetNewModeB` (`1030:09d0`), `_GetNewModeR` (`1030:0a50`)

As in [formigas.md](formigas.md#escolha-de-modo), with the explicit draw order: the `S8()` is only drawn for group 2 or 6 **and** (red, or black with `_ModeAuto` = 1). Black with `_ModeAuto` = 0 and group 2/6 → `_ModeMe` (`8:8a58`) without a draw. Other groups → `_CasteModeTabB[g]` without a draw. The table indices used are `_StrategicModeB` (black) and `_StrategicModeR` (red).

### 9.4 Recruitment: `_Recruit`, `_UnRecruit`, `_RecruitRed`, `_UnRecruitRed`, `_ForceModeA/B`

Fact seen in the code; none of them draws.

| Function | Address | Rule |
|---|---|---|
| `_Recruit(n)` | `1030:06d2` | list A from back to front, then list B: **black** ants of group 2 or 6 with M ≠ 6 → M = 6, S = 0, up to n |
| `_UnRecruit(p)` | `1030:078a` | quota = `_TemBModePop[6] / 2` if p = 0, or `_TemBModePop[6] + 100` if p ≠ 0 (Defense count of the last tick). List A: black with M = 6 → M = 0. List B: same. List R (black invaders): M = 6 → **M = 7** (Attack). Stops when the quota runs out |
| `_RecruitRed(n)` | `1030:0866` | list A: red ants of group 2/6 with M ≠ 19 and ≠ 6 → M = 6, S = 0, up to n |
| `_UnRecruitRed()` | `1030:08da` | list A: every red ant with M = 6 → M = 0 |
| `_ForceModeA(i, g, m)` / `_ForceModeB` | `1030:0550` / `0622` | no caller found. Drops the load according to the group (1: T += 8; 3/7: T −= 8 and, if the tile < 0x48, writes 0x48; 5/9: T −= 0x18), writes M = m and S = 0 for groups 1–3, 5–7, 9; and, if m = 6 and the yellow ant is on the surface, S = `(_MeLocY & 0xFC) << 2 | (_MeLocX >> 3)` |

`_UnRecruit` reads `_TemBModePop`, which at the moment of the player's click still holds the count from the previous tick (it is only cleared in step 12 of the next tick).

## 10. Functions with no caller found

Fact seen in the code (search for direct calls in the pseudocode): `_SimEggA`, `_SimQueenA`, `_LostHeadA` (`1028:0b1e`: returns 1 if the tail lost the head, same rule as 3.10), `_DoRestAnt`, `_DoRepoLoit`, `_DoRepoFly`, `_DoDefendNest`, `_StartFightA`, `_FeedAnts`, `_GstrB`, `_StartAttack`, `_ForceModeA/B`, `_GetNewRedTask`. The equivalent rules are inlined in the callers described above. [hypothesis] Leftovers from the Mac port or unresolved calls through pointers.

## 11. Quirks to reproduce

| Id | What | Where |
|---|---|---|
| P1 | `_DecTSmell` with coordinates already divided by 2 at the end of the trail | 3.4 |
| P2 | food carriers pass through other ants without fighting | 3.5 |
| P3 | fight corpse with the **winner's** color; `Vida` cleared under the winner | 6.3 |
| P4 | `_MakeRedInitiator` does not update `_LifeA` | 9.2 |
| P5 | the attacker vanishes if `_FindInAList` does not find the defender | 6.1 |
| P6 | natural death of red ants counts in `_BAntsExpired` | 6.5 |
| P7 | modes 17 and 18 do nothing on the surface | 3.17 |
| P8 | `S256()` drawn for empty slots; `S8()` of `_DoRandAntA` drawn even for those that cannot forage | 1, 3.1 |
| P9 | `_GetForageDir` consumes a useless `S8()` | 2.5.4 |
| P10 | `_GetWinner` draws with the R generator, not with S | 6.2 |

## 12. Suggested test cases (QEMU bench)

Test bench: [docs/bancada-qemu.md](../../bancada-qemu.md). Save reading tool: `tools/save_table.py` and the offsets in [save-ant.md](../formatos/save-ant.md). "Save" = `Ctrl+A` with a new name. All tests compare per tick (`_GameTime`, offset 45740).

| # | Rule | Input | Expected | How to observe |
|---|---|---|---|---|
| T1 | Loop and natural death (1, 6.5) | Quick Game; let `_HealthB` drop below 32 (do not feed; ~4,500 ticks) | black corpses 0x10–0x13 appear in `_MapA` and `_BAntsExpired` goes up; with health ≥ 32, `_BAntsExpired` does not change through the surface | successive saves; count tiles 0x10–0x17 and read `_BAntsExpired` |
| T2 | Picking up food (2.3 step 1) | save right after a worker touches the food | the ant stays in the same cell, T group 3, M = 3, S = 200; the food tile lost 1 | lists A of the save + `_MapA` |
| T3 | Return trail (3.5) | follow an ant with M = 3 in 2 saves a few ticks apart | `_PherMapBT` along the route with values 199, 198… decreasing from the food | BT map of the save |
| T4 | Run-over (P2) | Experimental: food and a line of ants on the way back | carrier passes over the others without a fight cloud | screenshot + lists (two ants in the same cell) |
| T5 | Fight (6.1–6.3) | Experimental: black and red worker face to face | in the next save, one slot with T = 0x70/0xF0, M = 10, S = winner's T; the other with T = 0; alarm ≥ 40 in the cell; after ~16 ticks, corpse of the winner's color | lists A + `_PherMapA` + `_MapA` |
| T6 | Win ratio (6.2) | repeat T5 ≥ 50 times, and soldier × worker | ~50% and ~80% for the one in the cell | count `_RedLost`/`_BlackLost` |
| T7 | Nest entry (7) | ant steps on a hole | vanishes from list A; appears in list B at (y_hole, 1) with T dir = 4 and M/S preserved | lists A and B |
| T8 | Defend Nest / Take off (3.12, 3.15) | Full Game with mating flight armed | ants with M = 15 stay near the hole (scent ≥ 100) and go to 16; `_SwarmCntB` count goes up | fields `_FlyAway*`, `_SwarmCnt*` |
| T9 | Red strategy (9.1) | long Quick Game until the reds dominate | `_JustToBeMean` > 0 and `_StrategicModeR = 0` for 30–129 ticks; message "The red ants are attacking!" | fields 48118 and 48298 |
| T10 | Red leader (9.2) | Quick Game until `_BpopT ≥ 30` | there is exactly one red slot with M = 19 and T = 0xB0 (after moving); `_RedInitOn = 1`; `_RedDestX/Y` near `_LastFoodDrop` (±5) or `_FuzLoc` | lists A + fields 48254–48268 |
| T11 | Recruitment (9.4) | key 1 (recruits 5) with the yellow ant on the surface | 5 black ants of group 2/6 with M = 6, S = 0 (preference for those with the highest index in list A) | lists A/B before and after |
| T12 | Port × original | load the same `.ANT` in both and run N ticks without interaction, same windows | lists A/B/R and maps equal slot by slot | binary comparison of the `.ANT` files (also requires reproducing the drawing draws, see [rng.md](rng.md)) |

## Gaps

- Who arms `_FlyAwayB` (black mating flight) and the rules of `_QueenStorage*`: life cycle document (in parallel).
- Food fall in the house (`_DropFoodA`): checked in all four directions (8.2).
- Practical meaning of modes 17/18 on the surface (whether any ant leaves the nest with them).
- `_ForceModeA/B`, `_StartAttack`, `_GetNewRedTask` and other functions with no caller: confirm with Ghidra whether there are indirect calls (pointer table).
- All cases T1–T11 of this section were verified against the original (T9 and T11 in the eighth round, 2026-10-19; section "Observed in the original").
- `_YellowFight` and `_DoTroph` are the complete ones from `amarela.rs` (with `_YellowDeath(0)` and `_ResetYellowVars`); the balloons (`_RestBalloons`, `_FightBalloons`) record through `rules::baloes` (telas/baloes.md).

## Observed in the original

**Tick vectors** ([testes-tick.md](testes-tick.md), `tests/referencias/ticks/`, ticks 79–83 of the Quick Game): for each tick, the port starts from `estado_antes`, finds how many S steps the earlier subroutines (yard, water, antlion, spider, strategy) used and runs only `_DoAntSimA`. In all 5 ticks there is a starting point (1, 4, 2, 15 and 2 steps after the input seed) with which list A, the surface life in the ants' cells and the 5 scent maps become **equal** to those of `estado_depois`, and the original's output seed is reachable from where the surface stopped. The surface ants in these ticks are a black male in Pre-Flight (mode 14) and a red worker in Foraging (mode 2). `_GetStrategy` (strategic modes, `_MakeMe`, `_ModeMe`) also matches. Test: `vetores_de_tick_do_original` in `engine/tests/regras_superficie.rs`. Verified fact (2026-10-17).

**Test bench cases** (2026-10-17, `tools/caso_ant.py`; all checks passed, and the port, starting from `carregado.ANT` and running up to the same `GameTime`, gives the same result; see `tests/referencias/README.md`):

| Case | Rule | Observed in the original |
|---|---|---|
| SUP-01 | 3.5 (carrier, trail, P2) and `_GetNestDir` with the greatest neighbor in its own direction | stopped at tick 94: the ant at (24, 40), T 0x1E, M 3, S 184 (16 steps west, S − 1 per step); `_PherMapBT` equal to the port cell by cell |
| SUP-02 | 7 (`_GoInNest`) | list A slot with T = 0; the ant is slot 5 of list B, group 2, S 55 preserved; `_MapB(32, 1)` dug |
| SUP-03 | 9.2 (`_MakeRedInitiator`, `_DoRedInitiator`, `_GetRedBestDirs`) | with `_BpopT` ≥ 30, the red ant at the end of the list became leader (M 19); destination (59, 38) = `_LastFoodDrop` − (5, 0); at tick 90 the leader is at (89, 38), T 0xB6, same as the port |
| SUP-04 | pheromones in the tick and P7 | red trail 210 → 105 at tick 96; black trail 210 intact until tick 130; alarm 200 → 100 in the center and 25 in the neighbors at tick 128; red hole (59, 16) = 254; ant with M = 17 still |

No divergence between this specification and the original appeared in these cases.

**Tick vectors of the seventh round** (2026-10-19, `tools/cenarios_raros.py`, [testes-tick.md](testes-tick.md#sétima-rodada-2026-10-19-ramos-raros); all 307/307 with S, R and life):

| Case | Vector | Observed in the original |
|---|---|---|
| T1 | `morte_natural` (1763, 1772, 1796, 1842) | with `_HealthB` = `_HealthR` = 1, a black soldier, a red worker, a black male and a red male die on the surface: corpse and `_BAntsExpired` + 1 **also for the red ones** (6.5) |
| T5, T6 | `lutas_soldado`, `lutas_castas` (1793) | boards of black and red ants in Bored: dozens of fights per tick; across the 250 fights in the recordings, the ratios per class stay close to the matrix (testes-tick.md, finding 3; the matrix is complementary, M[i][j] + M[j][i] = 10, so each class's chance does not depend on who attacks) |
| T9 | `estrategia_malvada` (1711, 1712, 1767, 1768), `estrategia_vermelha_populosa`, `estrategia_vermelha_sorteio` (1711) | the three paths to MEAN in `_GstrR` (9.1): desperation (`_HealthR` 5, `_BpopT` 13 < `_RpopT`/2 = 14) → `_JustToBeMean` = 56 (S1(100) + 30), `_StrategicModeR` = 0, the counter dropping 1 per tick with the mode at 0 and, on reaching 0, the normal strategy back (mode 3 with `_HealthR` 60, `_TilesDugR` < P); "not normal" without a draw (`_RpopT` 150, `_BpopT` 20) → 56; through the draws `S32()` = `S128()` = 0 (seed 0x009c chosen in the port with `buscar_semente`) → 83. No modal box: message 5 of 4/1020 goes to the ribbon |
| — | `estrategia_preta_desespero` (1711) | the black ladder at mode 0 through desperation (`_HealthB` 5, `_RpopT` 14 < `_BpopT`/2) |
| T11 | actions `amarela_recrutar_5`, `amarela_recrutar_10`, `amarela_liberar_metade`, `amarela_recrutar_todas`, `amarela_liberar_todas`; ticks `recrutadas` (1726, 1727) | with the game paused, keys 1, 2, 3, Ctrl+2 and 4 (formiga-amarela.md 11.4): black ants in Defense 0 → 5 → 15; `_UnRecruit(0)` with `_TemBModePop[6]` = 15 from the last tick releases 7 (15 → 8, from back to front in list A, without reaching R); `_Recruit(1000)` goes from list A (29) to B (the worker placed in the nest); `_UnRecruit(1)` clears A and B and sets M = 7 on the recruited black invader of list R. The following ticks (mode 6, `_DoRecruitAnt`) match |
| T8 | `revoada_natural` (3722, 3842, 3843) | without forced castes: the red colony of `rico`, running at ultra speed, produced its own males and new queens and `_GstrR` armed `_FlyAwayR`; flight (16) leaving the map, take-off 15 → 16 by `S1(F)` with F = 5 |
| T8 | `revoada_limiar` (1791, 1855) | the exact tick at which it arms, at normal speed: with 20 red males in the lists, the count at tick 1791 (`_Cycle` → 1792, multiple of 64) leaves `_CastePopR[3]` + `[4]` = 20 and `_FlyAwayR` stays 0 (the test is > 20); with 21, the count at tick 1855 arms 200 **in the same tick** (`_FullCount` in step 5, `_GstrR` in 11) and step 13 leaves it at 199 |

## Implementation in the port

Code: `engine/src/rules/superficie.rs`. Data: `data/superficie.json`, generated by `tools/extrair_superficie.py SIMANTW.EXE --json data/superficie.json` (addresses at the top of the script: `_IsCarryCaste`, `_UnCarryCaste`, `_CasteTabB`, `_ModeTabB`, `_ModeTabWB`, `_ModeTabSB`, `_CasteModeTabB`, fight class and matrix; and the constants of the `regras` block, with the origin of each one in `_origem`). Directions and `_TurnTab` come from `data/feromonios.json`.

| Original | Port |
|---|---|
| `_DoAntSimA` and the modes | `simular_superficie` (trait: `RegrasSuperficie::simular_superficie`) |
| `_AddFood` | `adicionar_comida` (the sine from `TabelasGeracao`; without them, the computed one, equal to the database's) |
| `_GetStrategy`, `_GstrR`, `_SetCasteProd`, `_SetModeProd` | `escolher_estrategia` |
| `_MakeRedInitiator` | `criar_lider_vermelha` |
| `_GetNewMode`, `_GetWinner`, `_DeadAntHere` | `modo_novo`, `vencedor_da_luta`, `cadaver_aqui` (public; implementation in `comum.rs`, the same as for the nest, the yellow ant, the yard and the enemies) |
| `_Recruit`, `_UnRecruit`, `_RecruitRed`, `_UnRecruitRed` | `recrutar`, `desrecrutar`, `recrutar_vermelhas`, `desrecrutar_vermelhas` |
| `_GetForageDir`, `_GetNestDir`, `_GetAlarmDir` | those of `feromonios.rs`, over a copy of the maps made at the start of the loop and returned at the end |
| `_GetRandDir`, `_GetDefendDir`, `_GetRedDefendDir`, `_GetRedBestDirs`, `_GoInNest` | private functions of `superficie.rs` |
| `_Bounce`, `_GetDir`, `_GetDis`, `_TileCanBeMovedOn`/`_IsClearTile` (surface), `_IsItHole`, `_DigTileB/R`, `_SmoothEdges`, `_FixExitMap`, `_PickupFoodA` | `engine/src/rules/comum.rs` (see [../../decisoes.md](../../decisoes.md), "Common rules module") |
| `_YellowFight`, `_DoTroph` | `amarela::lutar_com_amarela`, `amarela::trofalaxia` |

Port decisions:

- The trait implementation uses `RegrasOriginais::superficie` and `RegrasOriginais::feromonios` (loaded with mods); `tabelas_padrao()` returns the embedded copies, for tests. The public functions take the tables explicitly.
- Without `TabelasGeracao` (without the game files, as in the vector tests), `adicionar_comida` uses the computed sine `trunc(sin(i·π/128)·32767)`, equal to the 64 values of object 9/1000, like the laser (`inimigos::seno_calculado`). Until 2026-10-19 it did nothing without the tables; no vector exercised the feeding, and the first one (`alimentacao`) revealed the gap.
- Reads outside the map (which the original would do in neighboring memory) count as obstacles; table indices out of range: strategic mode > 5 uses the last row of the table (the same choice as the nest; before the unification the surface returned mode 0), mode ≥ 20 in the count is ignored.
- The sound of `_AddFood` and the `_EditMessage` of the red attack do not generate an event (missing variant in `Evento`); music 0x2B0A generates `Evento::Musica`.
