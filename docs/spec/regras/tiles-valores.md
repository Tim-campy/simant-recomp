*Leia em português: [tiles-valores.md](../../pt-BR/spec/regras/tiles-valores.md)*

# Map cell values (`_MapA`, `_MapB`, `_MapR`)

Each map cell is one byte. The same byte serves as the **index of the tile** drawn and as the **terrain type** for the rules. This document lists what each range of values means for the rules, based on the test functions in the code and on world generation ([geracao-mundo.md](geracao-mundo.md)).

Maps (see also [../formatos/save-ant.md](../formatos/save-ant.md)):

| Map | Address | Size | Index | Plane (`_MePlane`/`_MapPlane`) |
|---|---|---|---|---|
| `_MapA` | `1048:28e8` | 128 × 64 | `x * 64 + y` | 0 (yard) and 1 (patch surface) |
| `_MapB` | `1048:48e8` | 64 × 64 | `x * 64 + y`, `y = 0` = surface row | 2 (black nest) |
| `_MapR` | `1048:58e8` | 64 × 64 | same | 3 (red nest) |

In the test functions, plane `< 2` uses `_MapA`, 2 uses `_MapB` and 3 uses `_MapR` (fact seen in the code, for example `_IsClearTile` @ `1020:5b2c`, `_IsNotObstacle` @ `1020:94c6`). The surface has two tile sets, chosen by `_TERRAINset` (`1040:9b6e`): **0 = yard** (object type 9 no. 10, `_Barrier = 0x50`) and **1 = house** (object type 9 no. 9, `_Barrier = 0x90`), switched by `_OverlayTileSet` (`1010:00f4`). Several ranges change meaning with `_TERRAINset`.

About drawing: `_DoCalcTile` draws the surface value as the tile index and, in the nests, `value − 0x70` (byte, with overflow) (recorded in save-ant.md). That is why no values ≥ 0x70 are stored in the nests: nest values go from 0 to 0x5D, and it is the drawn tile that falls at 0x90 and above.

## Surface, yard (`_TERRAINset = 0`)

| Value | Meaning | Source |
|---|---|---|
| 0x00–0x0F | ground (16 dirt/grass variations, drawn with `Sk(16)` during generation) | fact seen in the code (`_MakeYardPatch`); `_IsClearTile` considers only `< 0x10` "clear" |
| 0x18–0x1F | nest hole border, by direction: 0x18 top-left corner, 0x19 top, 0x1A top-right corner, 0x1B left, 0x1C right, 0x1D bottom-left corner, 0x1E bottom, 0x1F bottom-right corner | fact seen in the code: table `DGROUP:0x230c` indexed by direction in `_MakeNewHoleB/R`; orientation deduced from `_Dx8/_Dy8` |
| 0x20–0x23 | 2 × 2 object: 0x20 top-left, 0x21 top-right, 0x22 bottom-left, 0x23 bottom-right | fact seen in the code (`_MakeYardPatch`); what it is [hypothesis: tuft of grass/plant, to be confirmed on the tile] |
| 0x24/0x25, 0x26/0x27 | 2 × 1 objects (left/right), two variants | fact seen in the code; what it is [hypothesis: to be confirmed on the tile] |
| 0x28 | single-cell object, 100 attempts per patch | fact seen in the code; what it is [hypothesis] |
| 0x30–0x37 | antlion trap border, by direction (0x30 top-left, 0x31 top, 0x32 top-right, 0x33 left, 0x34 right, 0x35 bottom-left, 0x36 bottom, 0x37 bottom-right) | fact seen in the code (`_InitAntLions`, table `DGROUP:0x25e8`) |
| 0x38 | center of the antlion trap | fact seen in the code |
| 0x48–0x4B | food, 1 to 4 units (`_AddFood` creates 0x48 and adds 1 up to 0x4B) | fact seen in the code (`_IsItFood` @ `1028:2d1a`, `_AddFood`) |
| 0x50 | hole (nest entrance) | fact seen in the code (`_IsItHole` @ `1028:2cc0`: in the yard, a hole is exactly 0x50) |
| 0x51–0x53 | small pebble that can be carried (generation only creates 0x51) | fact seen in the code (`_IsThisPebble` @ `1020:5f32`, only on plane 1; `_IsLiftable` @ `1020:97ca`) |
| 0x54–0x5F | large rock (5 × 5 and 3 × 3 shapes): 0x5B top-left corner, 0x54 top edge, 0x55 top-right corner, 0x56 right edge, 0x57 bottom-right corner, 0x58 bottom edge, 0x59 bottom-left corner, 0x5A left edge, 0x5C core, 0x5D–0x5F inner corners | fact seen in the code (tables of `_AddRock5/3`); piece names deduced from their position in the shapes |
| 0x60–0x67 | wall (Experimental mode tool) | fact seen in the code (`_IsItWall` @ `1010:86fe`); use [hypothesis] |
| 0x69–0x6B | caterpillar (segments) | [hypothesis]: they appear in `QUICK2.ANT` in a row (x 3–7, y 58), they are not created by generation |
| 0x70–0x73 | pill bug, one value per direction (`_SowTab` = `72 73 71 70 72 73 71 70`) | fact seen in the code (`_InitPillar`) |
| 0x74–0x77 | rain puddle (`_InitWater` creates 0x74; `_DoWater` makes it grow) | fact seen in the code |

Ranges with no use seen in yard generation: 0x10–0x17, 0x29–0x2F, 0x39–0x47, 0x4C–0x4F, 0x68, 0x6C–0x6F, ≥ 0x78 [open].

## Surface, house (`_TERRAINset = 1`)

Only what the test functions say (the content of each patch is open, see [geracao-mundo.md](geracao-mundo.md) §5):

| Value | Meaning | Source |
|---|---|---|
| 0x00–0x0F | clear ground (`_IsClearTile`) | fact seen in the code |
| 0x18–0x27 | food (`_IsItFood` in the house); `_AddFood` converts ground into food and adds 1 while `value & 3 < 3` | fact seen in the code |
| 0x3E/0x3F | lint on the carpet (`_CarpetFloorL/R`, `_MakeLint2`) | fact seen in the code; name [hypothesis] |
| 0x5C–0x5F, 0x60/0x61 | floor tiles (`_FloorTiles` writes 0x5C–0x5F at the joints and 0x60/0x61 in strips) | fact seen in the code |
| 0x64, 0x7A, 0x7B, 0x03 | fills of the left carpet (`_CarpetFloorL`) | fact seen in the code; names [hypothesis] |
| 0x80–0x8F | hole in the house (`_IsItHole`); `_CanBeHouseHole` creates 0x84, 0x85, 0x86, 0x8A and 0x80–0x83 | fact seen in the code |

## Nests (`_MapB`, `_MapR`)

| Value | Meaning | Source |
|---|---|---|
| 0x00–0x07 | tunnel (dug ground, 8 variations `Sk(8)`) | fact seen in the code (`_DigTileB/R`, `_SmoothEdges*`) |
| 0x08–0x0F | [open] (passable: `< 0x19`) | |
| 0x10–0x13 | food stored in the nest | fact seen in the code (`_IsItNFood` @ `1020:5f64`, `_IsThisFood`, `_IsItFoodAt`) |
| 0x14 | created by `_DigTileB` in both nests at the same time, below y = 53 | fact seen in the code; [hypothesis: passage between the nests] |
| 0x18 | opening to the surface on row y = 0 | fact seen in the code (`_IsItAHole` @ `1020:9b4a`, `_SmoothEdges*`, `_GetExitDirB`) |
| 0x1C–0x1F | surface row still closed (grass seen from the side) | fact seen in the code (`_IsThisGrass` @ `1020:5ee4`; generation in `_RandWorld`) |
| 0x20–0x2E | dirt; the 4 low bits of `value − 0x1F` say which neighbors are also dirt (bit 1 = up, 2 = right, 4 = down, 8 = left; the map border counts as dirt). 0x2E = surrounded (solid dirt, the initial value) | fact seen in the code (`_SmoothEdgesB` @ `1020:255a`, `_IsItDirt` @ `1020:1182`) |
| 0x30–0x31 | pebble inside the nest (carriable, passable) | fact seen in the code (`_IsThisPebble`, `_TileCanBeMovedOn`) |
| 0x4E | flooded tunnel | fact seen in the code (`_AddWater` @ `1020:0b8a`: tunnel `< 0x20` becomes 0x4E) |
| 0x4F–0x5D | flooded dirt (`dirt + 0x2F`) | fact seen in the code (`_AddWater`, `_RIsItDirt` @ `1020:26c4` accepts 0x20–0x2F and ≥ 0x4F) |

Smoothing rule (`_SmoothEdgesB/R`, fact seen in the code): when digging (x, y), the game re-evaluates the four neighbors. For each one that is dirt (≥ 0x20, outside 0x30–0x4E): it computes the mask of solid neighbors (solid = ≥ 0x20 and outside 0x30–0x4E); if the mask is not zero, it writes `0x1F + mask` (+ 0x2F if the cell was flooded); if it is zero (isolated dirt), the cell becomes tunnel `Sk(8)` (or 0x4E if flooded). On row y = 0, a cell ≤ 0x2F becomes 0x18 (opening). Fact verified indirectly: the reimplementation reproduces `_MapB`/`_MapR` of `QUICK2.ANT` byte for byte.

## Test functions (summary)

Fact seen in the code. "Surface" = plane < 2.

| Function | Address | Rule |
|---|---|---|
| `_IsItDirt(v)` | `1020:1182` | 0x20 ≤ v ≤ 0x2E |
| `_RIsItDirt(v)` | `1020:26c4` | v ≥ 0x20 and (v < 0x30 or v > 0x4E) |
| `_IsThisFood(p, v)` | `1020:5f04` | surface: `_IsItFood(v)`; nest: 0x10 ≤ v ≤ 0x13 |
| `_IsItFood(v)` | `1028:2d1a` | yard: 0x48–0x4B; house: 0x18–0x27 |
| `_IsThisPebble(p, v)` | `1020:5f32` | plane 1: 0x51–0x53; plane 0: never; nest: 0x30–0x31 |
| `_IsThisGrass(p, v)` | `1020:5ee4` | nest and 0x1C ≤ v ≤ 0x1F |
| `_IsThisEgg(T)` | `1020:5ec8` | (T & 0x7F) from 1 to 7 (tests the ant type, not the map) |
| `_IsItDigable(p, x, y)` | `1020:95c6` | nest only, inside the map, and v dirt (0x20–0x2E) or closed surface (0x1C–0x1F) |
| `_IsNotBarrier(v)` | `1020:94a0` | yard: v ≤ 0x50; house: v ≤ 0x5F |
| `_IsNotObstacle(p, x, y)` | `1020:94c6` | surface: inside the map and v ≤ 0x53 (yard) or ≤ 0x5F (house); nest: v < 0x19 or 0x30–0x31 |
| `_IsLessThanHole(v)` | `1020:9784` | yard: v ≤ 0x4F; house: v ≤ 0x58 |
| `_IsItHole(x, y)` | `1028:2cc0` | yard: v = 0x50; house: 0x80 ≤ v ≤ 0x8F |
| `_IsItAHole(p, x, y)` | `1020:9b4a` | surface: `_IsItHole`; nest: only at y = 0 and v = 0x18 |
| `_IsItWall(v)` | `1010:86fe` | 0x60 ≤ v ≤ 0x67 |
| `_IsClearTile(p, x, y)` | `1020:5b2c` | inside the map; no ant in the `_Life?` cell (0 or 0xFF/0xFE, the yellow ant, count as clear); and v < 0x10 (surface) or v < 8 (nest) |
| `_IsClear3x3(p, x, y)` | `1020:5ad2` | `_IsClearTile` on the cell and its 8 neighbors |
| `_TileCanBeMovedOn(p, x, y, …, dig)` | `1020:9342` | surface: inside the map and v ≤ 0x53 (yard) or ≤ 0x90 (house). Nest: v < 0x19 or 0x30–0x31; if the last parameter is true ([hypothesis]: "can dig"), also 0x1C–0x1F and 0x20–0x2E. On rows y ≤ 1 there are extra rules about changing plane and about the cell below [detail not specified here] |

Note: in `_IsNotObstacle` the second test for `_TERRAINset = 0` (limit 0x50) is never reached; the effective limit in the yard is 0x53 (fact seen in the code, probably a bug in the original).

## `_LifeA/B/R` values written during generation

They are not map values, but generation writes to them: the ant's T byte (bit 7 = red; bits 3–6 = class; bits 0–2 = direction in adults), 0xFF/0xFE for the yellow ant. Generation uses T = 0x10, 0x20, 0x30, 0x40 (+ direction) on the surface, 0x62/0x6A (black queen and her second half) and 0xE2/0xEA (red). Class names: see [../formatos/save-ant.md](../formatos/save-ant.md) [hypothesis recorded there].

## How to test against the original

1. **Count per value in the saves:** `QUICK2.ANT` has, in `_MapA`, 0x00–0x0F (7,415 cells), 0x18–0x1F (2 of each: two holes), 0x20–0x23 (18 of each), 0x24/0x25 (10), 0x26/0x27 (18), 0x28 (96), 0x30–0x38 (2 of each: two antlions), 0x48–0x4B (49 food cells), 0x50 (2), 0x51 (117), 0x54–0x5F (rocks), 0x69–0x6B, 0x71/0x72; in the nests, only 0x00–0x07, 0x18, 0x1C–0x1F and 0x20–0x2E. Each new save must stay within these ranges; a value outside them is a sign of an undocumented range.
2. **Visual identification:** draw the `_MapA` of `QUICK2.ANT` with the yard tile set (object type 9 no. 10; sheet in `re_trabalho/png/tiles_9_10_fora.png`) and compare with a capture of the edit window on the QEMU bench ([../../bancada-qemu.md](../../bancada-qemu.md)) centered on the same points. This confirms the name [hypotheses] (0x20–0x28, 0x69–0x6B) and the x/y orientation on screen.
3. **Passage rules:** on the test bench, with the yellow ant, try to walk over a pebble (0x51), a rock (0x54–0x5F) and a hole, and to dig into 0x1C–0x1F; compare with `_TileCanBeMovedOn`/`_IsItDigable`.
