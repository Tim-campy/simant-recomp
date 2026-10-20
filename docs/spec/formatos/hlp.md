*Leia em português: [hlp.md](../../pt-BR/spec/formatos/hlp.md)*

# Help file `SIMANT.HLP` (WinHelp 3.0)

The original does not draw its own help: it calls `WinHelp(hwnd, "SIMANT.HLP", command, n)` (`_DoWinHelp` `1000:5aac`, `_DoEvent`, `_HelpKeyDown`; the file name is in `_helpFile`, `9:76ac`) and Windows' WinHelp shows the topic. The port reads the user's file (`MAXIS/ANTWIN/SIMANT.HLP`, 197,256 bytes) and draws the window itself (viewer: [../telas/ajuda.md](../telas/ajuda.md)).

Implementation: `engine/src/formats/hlp.rs` (reading), `tools/hlp_inspecionar.py` (inspection in Python). Tests with the real file: `engine/tests/ajuda_reais.rs`.

Sources for the format: the public description "Windows Help File Format" (helpfile.txt, by M. Winterhoff, distributed with helpdeco) and byte-by-byte checking against `SIMANT.HLP`. Below, "[file]" = checked in `SIMANT.HLP`; the rest comes from the public description and is marked when unused.

## 1. Header and internal files

| Offset | Type | Content | In SIMANT.HLP |
|---|---|---|---|
| 0 | u32 | magic 0x00035F3F | [file] |
| 4 | i32 | directory start | 0x8C1 |
| 8 | i32 | first free block (−1) | −1 |
| 12 | u32 | file size | 197,256 |

Each internal file starts with 9 bytes: `i32` reserved, `i32` used (content size), `u8` flags; the content follows immediately.

The directory is an internal file with a B+ tree: after the 9 bytes, the tree header (38 bytes): `u16` magic 0x293B, `u16` flags, `u16` page size (1024), 16 bytes of structure ("z4"), `i16` 0, `i16` splits, `i16` root page, `i16` −1, `i16` total pages, `i16` levels, `i32` entries. The pages follow. Leaf page: `u16` free, `i16` number of entries, `i16` previous page, `i16` next page, and the entries (0-terminated name, `i32` position of the internal file). Index page: `u16` free, `i16` entries, `i16` first child page, … — to find the first leaf, just descend through the first child and follow the "next" links. In SIMANT.HLP the tree has one level and one page [file].

Internal files of SIMANT.HLP [file]: `bm0`…`bm65` (pictures), `|Phrases`, `|SYSTEM`, `|TOPIC`, `|TOMAP`, `|FONT`, `|CTXOMAP`, `|KWDATA`, `|KWMAP`, `|KWBTREE` (search keywords) and `|TTLBTREE` (titles). The port reads `|KWBTREE` and `|KWDATA` (since 2026-10-18, `ArquivoAjuda::palavras_chave`) and does not read `|KWMAP` nor `|TTLBTREE`.

`|KWBTREE` [file]: B+ tree like the directory (magic 0x293B, 2048-byte pages, 38-byte header; root at +26, levels at +32, 128 entries in SIMANT.HLP). In the leaves (`u16` free, `i16` entries, `i16` previous, `i16` next), each entry is the keyword (ASCIIZ), `u16` number of topics and `i32` offset into `|KWDATA`, which is a `u32` array with the topic positions (like those of `|CTXOMAP`). The order of the leaves is the alphabetical order of the Search list ("About", "Alarm", "Altruism"…; test bench s01).

## 2. `|SYSTEM`

`u16` magic 0x036C, `u16` minor version, `u16` major (1), `i32` date, `u16` flags. Minor 15 = HC30 compiler (Windows 3.0); 21 = HC31 (3.1); above 16, `|TOPIC` may be compressed (flags 4 or 8) and the title comes in records. In HC30 the title is a 0-terminated string right after the header.

SIMANT.HLP: minor 15, flags 8, title "SimAnt" [file]. The port only reads HC30 (the only one SimAnt ships); an HC31 gives an error.

## 3. `|Phrases` (HC30)

`u16` number of phrases N, `u16` 0x0100, N + 1 `u16` offsets **relative to the start of the table** (the first is 2 × (N + 1)), and the phrase text, without terminator (a phrase ends where the next begins). SIMANT.HLP: 1,147 phrases [file].

In the topic text, a byte 1–9 followed by a byte `d` forms the index `256 × (byte − 1) + d`; the phrase is `index / 2`, followed by a space when the index is odd. Text is only compressed this way when the uncompressed size (`DataLen2`) is larger than the space it takes in the record.

## 4. `|TOPIC` (HC30)

2048-byte blocks, each with a 12-byte header (`i32` last TOPICLINK, `i32` first TOPICLINK, `i32` last topic header); no LZ77 compression in HC30 [file]. Records (TOPICLINK) cross blocks: when reading a record, the 12 bytes of each block along the way are skipped.

TOPICLINK: `i32` record size, `i32` `DataLen2` (uncompressed text size), `i32` previous, `i32` **next = distance in bytes to the next record** (counting the skipped block headers; in HC31 it is an absolute position), `i32` `DataLen1` (header + LinkData1), `u8` type; LinkData1 runs up to `DataLen1`, the text (LinkData2) up to the end of the record. The first record is at position 12. SIMANT.HLP: 344 topics, 350 text records [file].

Types:

- **2 — topic header.** LinkData1: `u32` size, `i32` previous topic, `i32` next topic (topic numbers of the browse sequence, −1 = none; `<<` and `>>` buttons). LinkData2: the title. In SIMANT.HLP every title ends in "(id=0x…)" with the context number (e.g. "Patch Ribbon Bar (id=0x2200)") [file]; the port stores the raw title and a title without that suffix.
- **1 — text (HC30).** LinkData1: `compressed signed long` (size), `u8`, `u8`, `u16` id, `u16` paragraph bits, the optional values of the bits and the list of formatting commands up to 0xFF. In SIMANT.HLP the first four fields are always 0, 0x80, 0x08CA and a single record has bits ≠ 0 (0x0800, centered) [file].

Paragraph bits (from bit 0 upward): unknown value (`compressed signed long`), space above, below, between lines, left indent, right indent, first-line indent (`compressed signed short` each), unused, border (`u8` + `i16`), tabs (`compressed signed short` n, and n × `compressed unsigned short`, with bit 0x4000 = one more type value), right-aligned (0x0400), centered (0x0800). The units of the measurements were not checked (SIMANT.HLP uses none) [hypothesis].

Compressed values: *unsigned short* = 1 byte (even) or 2 bytes (odd), half of the value read; *signed short* = the same minus 64 (1 byte) or 16384 (2 bytes); *unsigned long* = 2 or 4 bytes, half; *signed long* = the same minus 16384 or 0x4000000.

Text: LinkData2 is a sequence of 0-separated strings. A string (possibly empty) comes before each command: string 0, command 0, string 1, command 1, …, up to 0xFF.

| Command | Arguments | Meaning |
|---|---|---|
| 0x80 | `u16` | changes the font (index into `|FONT`); the font carries over into the next record |
| 0x81 | — | line break |
| 0x82 | — | end of paragraph |
| 0x83 | — | tab |
| 0x86, 0x87, 0x88 | `u8` type, `compressed signed long` size, [type 0x22: `compressed unsigned short`], data (`i16` embedded, `i16` picture number `bmN` when 0) | picture in the text, left or right. SIMANT.HLP: 65 pictures in topics, type 3, size 4; 46 with 0x86 (in the middle of the text, button icons) and 19 with 0x87 [file] |
| 0x89 | — | end of hotspot |
| 0x8B, 0x8C | — | non-breaking space / hyphen |
| 0xC8, 0xCC | `u16` n, n bytes | macro |
| 0xE0, 0xE1 | `u32` topic number | popup / jump (HC30); the number indexes `|TOMAP` |
| 0xE2–0xE7 | `u32` | HC31 popup/jump (context hash) [not used] |
| 0xEA–0xEF | `u16` n, n bytes | jump to another file/window [not used] |
| 0xFF | — | end |

Checked example (index, context 0x100): font 0, "SimAnt", font 1, end of paragraph, "Welcome to SimAnt", end, end (blank line), and 14 times "jump 0xE1, text, 0x89, end of paragraph" (Introduction, Ribbon Bars, …, Credits), like capture a06 of the original's WinHelp.

## 5. `|TOMAP`, `|CTXOMAP`

- `|TOMAP` (HC30): `u32` array, position of each topic header by topic number (360 entries in SIMANT.HLP). The 0xE0/0xE1 jumps and the browse sequence use this number [file: the index's jump 0x124 leads to "Introduction"].
- `|CTXOMAP`: `u16` n and n pairs (`u32` context number, `u32` topic header position). This is what `WinHelp(…, HELP_CONTEXT, n)` uses. Checked with the original's WinHelp on the test bench: 0x2200 → "Patch Ribbon Bar" (ribbon "?" button, a04), 0x2209 → "Status Window Button" (a05), 4 → "Edit Window" (a02), and in the file 0x102 "Map Window", 0x1300 "Caste Control Window", 0x2308 "Help Button", 0xFD48 "Bookmarks", 0x100 "SimAnt" (the index; F1 opens the index, a06).

## 6. `|FONT` (HC30)

`u16` number of faces, `u16` number of descriptors, `u16` faces start, `u16` descriptors start. Faces have a fixed size ((descriptors start − faces start) / faces). Descriptor (11 bytes): `u8` attributes (bit 0 bold, 1 italic, 2 underline, …), `u8` half points, `u8` family, `u16` face index, 3 RGB bytes for foreground and 3 for background. SIMANT.HLP: 69 faces and 3 descriptors: 0 = Helv 12 pt bold, 1 = Helv 10 pt, 2 = Helv 10 pt bold [file].

## 7. `bmN` pictures (MRB/SHG)

`u16` magic 0x706C ("lp"), `u16` number of images, `u32` position of each image. Image: `u8` type (5 = DDB, 6 = DIB), `u8` packing (0 none, 1 RunLen, 2 LZ77, 3 LZ77 then RunLen), and, compressed: x and y dpi (`unsigned long`), planes and bits per pixel (`unsigned short`), width, height, colors used, important colors, data size, hotspot size (`unsigned long`); then `u32` data position and `u32` hotspot position (relative to the start of the image) and the RGBQUAD palette (DIB). The pixels are those of a DIB: rows bottom to top, aligned to 4 bytes.

RunLen: byte n; with bit 7, copy the next `n & 0x7F` bytes; otherwise, repeat the next byte n times. LZ77: flags byte (bit 0 first); bit 1 = `u16` pair with length `(v >> 12) + 3` and distance `(v & 0xFFF) + 1`; bit 0 = literal byte.

SIMANT.HLP: the 66 `bmN` are 4-bit, 16-color DIBs with RunLen (e.g. `bm0`, 75 × 75) [file: the 65 used ones decode; the image drawn by the port was not compared with the original's WinHelp].
