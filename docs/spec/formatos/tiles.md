*Leia em português: [tiles.md](../../pt-BR/spec/formatos/tiles.md)*

# Tiles (type 9 objects) and assembling the map image

This document covers the tilesets of the color graphics database (`WINGANT`, mode `W`, `_displayType = 10`) and how the edit window assembles the image of the surface and nests from the maps.

Implementation: `engine/src/formats/tiles.rs` (tilesets) and `engine/src/core/mapa.rs` (composition). Example: `cargo run -p engine --example exportar_mapa -- <game folder> <save .ANT> <output folder>`.

**Sources:** Ghidra pseudocode (reference only) of the functions `_OverlayTileSet` (`1010:00f4`), `_LoadTiles` (`1010:01d8`), `FUN_1010_6250` (`1010:6250`, draws one cell of the edit window), `FUN_1010_63fe` (`1010:63fe`, cell with life), `_XferTileColor` (`1018:47dd`), `_XferLifeTileColor` (`1018:48fa`) and `_DoCalcTile` (`1018:4a6b`). The names come from `SIMANTW.SYM`. The data and the image were checked against `QUICK2.ANT` and the capture `qi_14_salvo.png` from the QEMU bench (see "Verification").

## Objects

All are type 9 in `WINGANT`. The call is `_db_LoadObject(id, 9, 1)`.

| Object | Content | Size | Loaded by |
|---|---|---|---|
| 9/10 | yard ground (outside the house) | 32,768 bytes = 256 tiles | `_OverlayTileSet(0, 1000)` and `_LoadTiles` |
| 9/9 | ground inside the house | 32,768 bytes = 256 tiles | `_OverlayTileSet(0, 1001)` |
| 9/15, 9/16, 9/17 | life on the surface (ants, eggs etc.), buffer `tilebufover` | 16,384 bytes each = 128 tiles | `_LoadTiles` |
| 9/18, 9/19, 9/20 | life in the nests, buffer `tilebufnest` | 16,384 bytes each = 128 tiles | `_LoadTiles` |

- The active ground is in a single handle (`DS:9de0`). `_OverlayTileSet` swaps 9/10 ↔ 9/9 and writes to `_TERRAINset` (`9:9b6e`, also at `DS:8df8`) 0 for the yard and 1 for the house. `_MakeYardPatch` and `_MakeHousePatch` call this swap. `_TERRAINset` is in the save (offset 48316, see [save-ant.md](save-ant.md)). In `QUICK2.ANT` it is 0 (yard). **Fact seen in the code.**
- `_OverlayTileSet` also writes to `DS:7604` 0x50 (yard) or 0x90 (house). The simulation compares this value with the bytes of `_MapA` in several `Code6` functions. It is the boundary between passable tiles and obstacles [hypothesis]. `_OverlayOutsideTiles` and `_OverlayInsideTiles` (`1030:78b2`/`78be`) only write this value.
- The life buffers are assembled by copying each object to offset 0, 0x4000 and 0x8000. Each object thus takes 128 tiles, which gives 384 tiles per buffer. In the files analyzed each object has exactly 16,384 bytes, so no gaps are left. **Verified fact** (test `tilesets_de_vida_tem_384_tiles_com_transparencia`).
- In the other color modes (`_displayType` even and different from 10), the life objects go through `_ConvertMaskBitmap` and each tile takes 0xA0 bytes, with mask. These modes do not exist in the analyzed copy and the port does not implement them.

## Tile format

- 16×16 pixels at 4 bpp, 8 bytes per row, 128 bytes per tile. Tiles are stored one after the other, so tile *i* starts at *i* × 128 (`_XferTileColor` adds `index * 0x80`). **Fact seen in the code.**
- Within a byte, the left pixel is in the high nibble, as in 4 bpp bitmaps ([bitmap.md](bitmap.md)).
- **Rows are stored bottom to top.** `_XferTileColor` copies the 16 rows, in order, into a bottom-up DIB (the destination starts at row `(editHeight − row − 1) × tileHeight` and goes up). Therefore the first stored row is the bottom one. **Fact seen in the code.** This was confirmed by comparing with the capture (the grass at the top of the nest only looks right in this orientation).
- The indices are those of the 16-color palette (`data/paleta.json`).

### Transparency of life tiles

`_XferLifeTileColor` walks the tile byte by byte:
- byte 0xDD (both pixels with index 13): writes nothing;
- otherwise, a nibble equal to 0xD keeps the destination pixel and the other nibble is written.

In practice, **index 13 (magenta) is transparent in life tiles**. In ground tiles, 13 is drawn normally. **Fact seen in the code.** In the port this is `TRANSPARENTE = 13`, and the pixel is skipped when drawing (`desenhar_tile_de_vida`).

Tile 0 of each life buffer is not used as life, because the value 0 in `_LifeA/_LifeB/_LifeR` means "cell without life" (see below). In `WINGANT` it has no transparent pixels.

## Map indexing

Maps are stored **by columns**: index = x × 64 + y, where y goes from 0 (top) to 63. **Fact seen in the code** (`_DoCalcTile`).

| `_MapPlane` | Map | Size (tiles) | Index into the ground tileset |
|---|---|---|---|
| 0 or 1 | `_MapA` (surface), 8192 bytes | 128 × 64 (`x & 0x7f`) | the byte itself |
| 2 | `_MapB` (black nest), 4096 bytes | 64 × 64 (`x & 0x3f`) | `(byte − 0x70) & 0xff` |
| 3 | `_MapR` (red nest), 4096 bytes | 64 × 64 | `(byte − 0x70) & 0xff` |

- In the nests, the subtraction wraps around within a byte. In `QUICK2.ANT` the nests only have values from 0x00 to 0x2E, which fall on tiles 0x90 to 0xBE of the ground tileset: grass and sky in row 0 (0x1C to 0x1F → 0xAC to 0xAF), entrance (0x18 → 0xA8), tunnel and dirt (0x2E → 0xBE). **Verified fact.**
- The nests use the same ground handle as the surface (`DS:9de0`), that is, 9/10 or 9/9 depending on `_TERRAINset`. Tiles 0x90 to 0xBF of both tilesets appear to be the same [hypothesis: not checked byte by byte].
- The value 0x70 (yard) vs. 0x90 (house) at `DS:7604` resembles the nest offset, but the relationship has not been studied.

### Pheromone layers (surface)

On the surface, `_DoCalcTile` first consults a pheromone layer, depending on `_ELayerMode`:

| `_ELayerMode` | Layer |
|---|---|
| 0 | `_PherMapA` |
| 1 | `_PherMapBN` |
| 2 | `_PherMapBT` |
| 3 | `_PherMapRN` |
| 4 | `_PherMapRT` |
| other (−1 = off, the default) | none |

Each layer has 2048 bytes (64 × 32), one cell for each 2×2 tiles, also by columns: index = (x / 2) × 32 + y / 2. If the byte is greater than 0x10, the ground tile becomes `((byte >> 4) − 0x10) & 0xff`, that is, 0xF1 to 0xFF (the range of yellow to red dots at the end of the tileset). Otherwise, `_MapA` applies. **Fact seen in the code.** The meaning of each mode (N = nest, T = trail) is a [hypothesis] based on the names. `_ELayerMode` is not in the save. The port optionally accepts the layer (`MapaChao::com_feromonio`). In `QUICK2.ANT` `_PherMapA` is all zero, so this part was not checked against the original.

## Life layer

`_DoCalcTile` also computes `_lifeTile` from `_LifeA` (surface), `_LifeB` or `_LifeR` (nests), with the same cell index:

| Life byte | `_lifeTile` |
|---|---|
| 0 | 0 (no life: only the ground is drawn) |
| 0xFF (yellow ant) | nest: `0x300 + _MeBase + _MeType + (_MeType < 8 ? _MeEgg : _MeDir)`; surface: the same with `0x380` |
| 0xFE | nest: `0x308 + _MeBase + _MeType + _MeDir`; surface: `0x388 + …` |
| other *v* | surface: `v + 0x100`; nest: `v + 0x200` |

`FUN_1010_63fe` subtracts the offset before indexing: −0x100 if `_lifeTile` < 0x200 (buffer `tilebufover`), −0x200 if it is between 0x200 and 0x37F (buffer `tilebufnest`) and −0x280 if it is ≥ 0x380 (buffer `tilebufover`). The yellow ant thus falls in the third part (indices 0x100 to 0x17F) of each buffer, which on the contact sheet is precisely that of the yellow ants. The cell is drawn with the ground (`_XferTileColor`) and, on top, the life tile with transparency (`_XferLifeTileColor`). **Fact seen in the code.**

`_LifeA`, `_LifeB` and `_LifeR` are not in the save. `_DoLoadInitializations` rebuilds them from the ant lists and the yellow ant: see [telas/jogo.md](../telas/jogo.md), "Life layer" [code]. The port draws life on the game screen (`core/edicao.rs`), checked against the capture.

## Assembling the image (edit window)

`FUN_1010_6250(col, row)` draws the visible cell (col, row) of the edit window:
1. the map cell is `x = _MapPnt + col` and `y = DAT_1040_7d30 + row`. If y goes past 63, it adds 0x40 to x and does `y &= 0x3f`. The effect on scrolling has not been studied yet [hypothesis: the nest view does not scroll beyond the bottom];
2. `_DoCalcTile(x, y)` computes `_grndTile` and `_lifeTile`;
3. if nothing changed since the last drawing (per-cell cache at `DAT_1048_143e`/`1442`) and `_editForce` = 0, it does not redraw;
4. without life, it copies the ground tile; with life, it calls `FUN_1010_63fe` (ground and life on top);
5. it invalidates the rectangle `(col × tileWidth + DS:9b08, row × tileHeight + DS:9b0a)` with `tileWidth × tileHeight` (16 × 16).

The port assembles any rectangle of cells (`MapaChao::desenhar`) into a `Quadro`. Each cell becomes a 16×16 tile, with cell (x0, y0) at the destination point.

## Verification

- **`QUICK2.ANT` black nest against `qi_14_salvo.png`** (captured in the same session in which the save was written, nest view of a Quick Game). The client area of the edit window starts at (24, 158) in the capture. The best fit is with the view starting at cell (22, 0). Outside the "Saved correctly" dialog, 99.9% of the pixels match with a 1:1 color mapping between the two images. The grass and sky row matches pixel by pixel. The few remaining differences are at the edges of the dialog and on the ants (life layer, which is not drawn). The tunnel appears at the same position and with the same shape where it is not covered. Tool: ad hoc alignment script in Python with `tools/png_simples.py`.
- QEMU shows some colors with RGB values different from the standard palette: index 3 comes out as (168,168,87), index 7 as (135,136,143) and green 2 as (0,168,87). The mapping is still 1:1 and so it does not change the conclusion. It is a difference of the test bench's video driver, not of the game [hypothesis].
- The test `ninho_preto_do_quick2_bate_com_a_referencia` stores the SHA-256 of the indices of the whole black nest map (1024 × 1024), generated after this check.
- The surface and the red nest of `QUICK2.ANT` were rendered and look consistent: rocks, leaves, nest entrances, mounds at the edges and the red nest's tunnel. There is, however, no capture of the surface in that session to compare with. **Not verified against the original.**
