*Leia em português: [bitmap.md](../../pt-BR/spec/formatos/bitmap.md)*

# Bitmap (database object type 2)

The content comes after decompression, if applicable (see [banco-dat-ndx.md](banco-dat-ndx.md)). Integers are little-endian.

| Offset | Type | Field |
|---|---|---|
| 0 | u16 | `kind`: 0 = opaque, 3 = masked (index 13 transparent), 0xFFFF = packed (see "kind field") [code] |
| 2 | u8 | format: bit 0x80 = packed 4 bpp (`0x84` in `WINGANT`); without it, `0x04` = planar and `0x01`/`0x00` = 1 bpp (`MWINNT`/`SHARED`) |
| 3 | u8 | ranges from 0x00 to 0x0F (and 0x65 in 96 sprites of `MWINNT`); not read by the drawing paths analyzed (see "Byte 3") |
| 4 | u32 | pixel size in bytes (in some `WINGANT` records with flag 5 the value is inconsistent; not used for reading) |
| 8 | u16 | width `w` |
| 10 | u16 | height `h` |
| 12 | … | pixels |

## Pixels

- **4 bpp (format 0x84):** two pixels per byte, high nibble first. Each row takes `((w·4 + 31) / 32) · 4` bytes (4-byte alignment, as in a Windows DIB). Rows are stored **bottom to top**.
- **1 bpp (format 0x01):** eight pixels per byte, most significant bit first, with the same 4-byte alignment. Rows are stored **top to bottom**. In `SHARED`, the size field declares the size already converted to 4 bpp (for example, 9632 bytes for a 2408-byte bitmap), which indicates that the game converts these mono bitmaps to color on load (`_ConvertMonoBitmap`).
- **Planar (format 0x04):** in `WINGANT`, only the icon menu (2/7200, 187 × 166); in `MWINNT`, 2/7005 and 2/8905. Each row, **top to bottom**, has the 4 1-bit planes in sequence, each with ⌈w/8⌉ bytes, **without alignment** (`_ConvertBitmap`, `1008:246e`: planes at +0, +⌈w/8⌉, +2⌈w/8⌉, +3⌈w/8⌉ of each row); plane k is bit k of the color index [measured: matches the menu capture (janelas.md §4.4); `MWINNT`'s 2/8905, 19 × 35, has 420 bytes = 35 × 4 × 3, which confirms the lack of alignment]. `MWINNT`'s 2/7005 (`kind` 3, 7 × 37) has 185 bytes = 37 × 5: a fifth plane per row [hypothesis: the mask of `_ConvertMaskBitmap2`]; the port reads the first 4.

**Source:**
- Verified fact: `tools`/`re_trabalho/sheet.py` correctly rendered 352 `WINGANT` bitmaps ("About" and "Select a Game" screens, caste controls, yard, sprites) and `MWINNT` bitmaps. Each of them matches the computed size.
- `_DoBitmap` (`1008:332a`) allocates `((w·4+31)/32)·4·h` bytes and copies the rows, which confirms the alignment.
- `_ConvertBitmap` (`1008:246e`) converts from 4 **planes** (rows of `ceil(w/8)` bytes per plane, 4 planes in sequence) to packed 4 bpp. It is called when the last parameter of `_DoBitmap` is not zero. The `WINGANT` bitmaps tested did not need this conversion. Which objects are planar remains open.
- `_GPutPic` (`1008:3c9a`) chooses `_DoBitmap` or `_DoMonoBitmap` by bit 0 of `_displayType`.

## Palette

**Verified fact (pixel-by-pixel comparison against DOSBox-X, 2026-10-16):** the indices of 4 bpp bitmaps follow the standard Windows 16-color palette, **in this order**: 0 black, 1–6 dark red/green/yellow/blue/magenta/cyan (0x80), **7 dark gray (0x808080)**, **8 light gray (0xC0C0C0)**, 9–14 the light versions (0xFF), 15 white. The first version of this document swapped 7 and 8; the capture showed the error (see `docs/divergencias.md`).

In the captures, colors appear expanded by the 6-bit VGA DAC (0x80 becomes 130 and 0xC0 becomes 195). This is an emulator effect, not the game's. `tools/comparar_captura.py` applies the same conversion. The game changes the palette in `_PatchColorArrays`, `_SetUpPalette` and `_InitPalette`, which have not been analyzed yet. Light magenta (index 13) appears as the sprite background, which points to a transparency color key [hypothesis].

## kind field

**Fact seen in the code** (`_DrawBitMapToBuffer`, `1008:418e`, which picks the copy routine from the header; `_GPutPic`/`_DoBitmap` receive only width, height and pixels):

- **0**: opaque. Color (even `_displayType`): format bit 0x80 set → `_CopyBitmap2` (packed 4 bpp), clear → `_ConvertBitmap2` (planar); monochrome → `_CopyMonoBitmap`.
- **3**: masked. `_CopyMaskBitmap2` (`1030:b110`) / `_ConvertMaskBitmap2` / `_CopyMonoMaskBitmap`. In the 4 bpp copy, each nibble equal to **0xD (13, light magenta)** is skipped: the transparent color is fixed, it does not come from the header (confirms the [hypothesis] in the "Palette" section). In `WINGANT` these are the sprites: ants (1000–1107), yard critters (1300…), drops and swarm (7005–7007), balloons (8020…, 8100…) etc. (105 objects).
- **0xFFFF (−1)**: packed. The u16 at +2 is the unpacked size (12-byte header + pixels) and an LZSS stream (the same as the database's) starts at +4; `_GUnpackPic` (`1008:40d4`) unpacks the 12-byte header and `size − 16` bytes of pixels (the last 4 bytes of the image are not unpacked and keep whatever was in memory) and proceeds by the inner `kind`. Only 3 objects, all in `MWINNT`: 2/2000 (title, 455 × 320), 2/2500 (315 × 258) and 2/7002 (514 × 226), stored in the database **without** the index compression bit. The inner ones have format 0x00 or 0x01 and byte-aligned 1 bpp rows. Checked by unpacking 2/2000 (the SimAnt title) and 2/7002 (the yard grid).

**Port** (2026-10-19): `Bitmap::decodificar` unpacks `kind` 0xFFFF (the 4 unpacked bytes stay 0), reads planar without alignment and treats format 0x00 as 1 bpp; all three databases decode in full (`todos_os_bitmaps_dos_bancos_decodificam`: 352 in `WINGANT`, 58 in `SHARED`, 343 in `MWINNT`). Sprite transparency is done by the port's drawing with index 13 (`kind` is not consulted). Monochrome mode (`MWINNT`) is not used by the port.

## Byte 3

Values per database (count of type 2 objects): `WINGANT` kind 0: 0 (32), 5 (13, the 17000 series), 9 (4, the 12000 series), 12 (2), 13 (1, the planar 7200), 15 (193); kind 3: 0 (62), 2 (7), 4 (16), 14 (20). `MWINNT`: 0, 1, 2, 3, 4, 13, 15 and 0x65 (96 sprites). None of the paths above reads the byte [code]; the value does not track the transparent color (fixed at 13) nor the format. [hypothesis: leftover from the conversion of the Mac resources, unused on Windows; other readers of the header not searched for].

## Open issues

- ~~A group of narrow, tall bitmaps (ids near 7100 in `WINGANT`) comes out with stripes~~ (2026-10-19): there are no bitmaps between 7011 and 7199 in `WINGANT`; the narrow, tall ones of the 7000 group are the raindrops (2/7005, 7 × 37, `kind` 3) and the swarm dots (7006/7007), which decode and are drawn by the port (rain checked on the test bench, telas/quintal.md). The ones that failed were the packed and planar ones of `MWINNT` (above).
- Byte 3 (above) and the fifth plane of `MWINNT`'s masked planar bitmaps.
