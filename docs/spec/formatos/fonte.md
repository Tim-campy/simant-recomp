*Leia em português: [fonte.md](../../pt-BR/spec/formatos/fonte.md)*

# Fonts and window text

The game uses two kinds of font, chosen by `_font_SetFont(n)` (`1008:611c`) from the "font" byte (0x28) of text objects:

| n | Font | Drawing |
|---|---|---|
| 0 | **Gaia8x14**, from `FONTRES.FON` | GDI: `AddFontResource("fontres.fon")` and `CreateFont(height 14, "Gaia8x14")` in `_GSetBigFont` (`1008:1664`), then `TextOut` |
| 1 | **Gaia8x8**, from `FONTRES.FON` | GDI: same with height 8 (`_GSetSmallFont`, `1008:1560`) |
| 2–5 | files `font1`–`font4` in the game folder (Macintosh `FONT` format) | its own renderer (`_font_MakeImage` + `_DrawChar`) |
| < 0 or > 5 | treated as 2 | |

`_font_InitFonts` (`1008:616c`) also tries `font5` and `font6`, which do not exist in the installation. Use in the `WINGANT` layouts: font 0 appears in 15 text objects, font 3 in 6, font 4 in 4 and font 2 in 1.

## `font1`–`font4`: Macintosh FONT format

**Source:** `_font_ReadFont` (`1030:aa4c`) and checking of the four files: the sizes match and `owTLoc` points exactly to the width/offset table. Values are **big-endian**.

| Offset | Field (Mac name) | font1 | font2 | font3 | font4 |
|---|---|---|---|---|---|
| 0x00 | `fontType` | 0x9801 | 0x9001 | 0x9000 | 0x9001 |
| 0x02 | `firstChar` | 0 | 0 | 0 | 0 |
| 0x04 | `lastChar` | 255 | 255 | 216 | 255 |
| 0x06 | `widMax` | | | | |
| 0x08 | `kernMax` | −2 | 0 | 0 | −1 |
| 0x0A | `nDescent` | | | | |
| 0x0C | `fRectWidth` | | | | |
| 0x0E | `fRectHeight` | 10 | 10 | 15 | 11 |
| 0x10 | `owTLoc` (in words, from 0x10) | | | | |
| 0x12–0x16 | `ascent`, `descent`, `leading` | 8/2/1 | 8/2/1 | 12/3/1 | 9/2/0 |
| 0x18 | `rowWords` | 56 | 55 | 68 | 68 |
| 0x1A | glyph strip: `rowWords·2` bytes per row × `fRectHeight` rows, 1 bit per pixel, most significant bit on the left | | | | |
| … | `locTable`: (`lastChar − firstChar + 3`) u16, x position of each glyph in the strip | | | | |
| … | `owTable`: same count; high byte = offset, low byte = advance; 0xFFFF = missing character | | | | |
| … | 516 extra bytes in three files (Mac width table), **not read** by the game | | | | |

`fontType & 0x2000 = 0` marks a proportional font (all four).

### Drawing a string (`_font_MakeImage`, `1030:ade0`; `_DrawChar`, `1030:b033`)

- A missing character (`owTable[c] = 0xFFFF`) **measures** as the glyph with index `lastChar − firstChar + 1` (`__font_StringWidth`, `__font_CharWidth`), but `_font_MakeImage` reads that index as a **byte** (`*(byte *)(font + 0x28)`) and draws glyph `(lastChar − firstChar + 1) & 0xFF`, moving by its advance: in the fonts with `lastChar` = 255 (`font1`, `font2`, `font4`) it is glyph 0, empty, and the missing character does not appear (the image keeps the measured width). In `font3` (`lastChar` 216) it is glyph 217 itself. [code; checked in boy no. 4's balloon, which has a 0x7F: telas/quintal.md §2.3, 2026-10-19.] The table is indexed by the character code itself (`firstChar` is 0 in all of them).
- **Advance** = low byte of `owTable[c]`, **plus 1 if `kernMax < 0`**.
- Glyph **x offset** = high byte + `kernMax` if `kernMax ≥ 0`; if `kernMax < 0`, the offset is 0. This is a quirk of the code, not the Mac rule.
- The glyph is the column range `locTable[c] .. locTable[c+1]` of the strip. The game copies starting from **row 1** of the strip (row 0 is ignored), with `fRectHeight − 1` rows. The image height and `__font_FontHeight` are `fRectHeight − 1`.
- Glyphs are combined by OR into a 1-bit image whose width is the sum of the advances. The image is drawn **opaque** (`_DoMonoBitmap` → `SetDIBitsToDevice`): bit 1 in the foreground color, bit 0 in the background color.

## `FONTRES.FON`: Gaia8x8 and Gaia8x14

NE executable with two `FONT` resources in Windows `.FNT` version 2.0 format (118-byte header; character table with `(width u16, offset u16)`; glyphs stored column by column, 8 pixels per column, one byte per row). Author: "Rodney S.K. Lai", 1991.

| Resource | Name | Cell | Ascent | Characters | Default |
|---|---|---|---|---|---|
| FONT 1 | Gaia8x8 | 8×8 | 6 | 32–255 | 128 |
| FONT 2 | Gaia8x14 | 8×14 | 12 | 32–255 | 128 |

`TextOut` draws the whole cell with an **opaque** background (`SetBkMode(OPAQUE)` in `_GPutStr`, `1008:1792`), top-left corner at (x, y). For the GDI fonts, `_fontHeight` = `tmHeight` + `tmExternalLeading` (14 for Gaia8x14) and `_fontWidth` = average of `tmAveCharWidth` and `tmMaxCharWidth` (8).

## Text position in an object (`_gr_JustifyStrInRect`, `1030:b300`)

With the rectangle (L, T, R, B), the string width W and the font height H:

| Alignment (bits 7–8 of the object flags) | x |
|---|---|
| 0 or 3 | (L + R − W) / 2 |
| 1 | L + 4 |
| 2 | R − W − 4 |

x is never less than L. y = (T + B − H) / 2 (integer division). In alignment 3, the rest of the line to the left and right of the text is filled with the background color.

**Colors** (`_win_DrawObjectI`): foreground = entry `[0]` of the object's color table, background = `[2]` in color mode.

## Dynamic text

Texts with `%s` or `%d` (e.g. `fileSelDO`, `antEffEH`) are `printf` formats filled in at run time. The port does not draw them yet.

## Validation

**Verified fact** against `tests/referencias/capturas/apos_titulo.png` (registration window in DOSBox-X): rendering with the port and searching for the whole 1-bit block in the capture, the pattern appears exactly once.

| Text | Font | Size | Position in capture | Ink |
|---|---|---|---|---|
| "Click here to continue" | slot 3 (`font2`) | 84×9 | (362, 397) | black on light gray (color 9 of the table: `[0]`=0, `[2]`=8) |
| "Customer ID:" | slot 4 (`font3`) | 85×14 | (361, 356) | blue (index 12) |

This confirms the cropping of row 0, the height `fRectHeight − 1`, the advances and the `kernMax ≥ 0` case. **Not yet validated:** the `kernMax < 0` case (`font1`, `font4`) and the GDI Gaia fonts, because no screen captured so far uses these fonts. The test `fontes_mac_batem_com_o_original` (`engine/tests/arquivos_reais.rs`) locks in both results.

## Open issues

- Validate Gaia8x14 (used in `fileSelWin`, `quitWin`) and the fonts with `kernMax < 0`.
- The registration dialog text is drawn by `_win_DrawPictureWindow`, not by text objects: title in `font3`, blue.
