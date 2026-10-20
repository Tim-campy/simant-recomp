*Leia em português: [banco-dat-ndx.md](../../pt-BR/spec/formatos/banco-dat-ndx.md)*

# `.NDX` / `.DAT` resource database

SimAnt for Windows keeps its graphics, texts, windows, menus, sounds and music in `NAME.NDX` + `NAME.DAT` pairs in the game folder (`MAXIS\ANTWIN`).

| Database | Contents | When it is opened |
|---|---|---|
| `SHARED` | windows, menus, string lists, information system texts, common mono bitmaps | always |
| `WINGANT` | window layouts and 4 bpp (16-color) bitmaps | color video mode (`_displayType` = 10) |
| `MWINNT` | window layouts and 1 bpp (monochrome) bitmaps | monochrome mode (`_displayType` = 9) |
| `SOUND` | digital instruments, music headers, MIDI files | always |
| `LANGUAGE` | (does not exist in the analyzed copy) | attempt to open `language.dat` [hypothesis: optional translation database] |

**Source:**
- Database names: `_InitGraphics` (`1008:010c`) builds the name with `"%s%s"` from a prefix table in DGROUP (`1048:b92b`…: `hcega`, `mono`, `tdyga`, `lcega`, `l256`, `mwin`, `winga`) indexed by `_displayType`, and calls `_db_SetDataBase`. The `nt` suffix comes from the caller. With `mwin`+`nt` and `winga`+`nt` you get the existing files, but the origin of the suffix has not yet been confirmed in the code [hypothesis].
- Strings `language.dat`, `language`, `shared` and `lrshare` in `SIMANTW.EXE`, file offset `0x2b9a0`.
- Verified fact with `tools/db_list.py`: the four existing pairs are read end to end with nothing left over.

## `.NDX`: index

Little-endian integers.

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | number of entries `N` |
| 0x04 | 16 bytes | zeros in every analyzed file (meaning unknown) |
| 0x14 | `N` × 8 bytes | entries |

Entry:

| Offset | Type | Field |
|---|---|---|
| 0 | u32 | position of the record in the `.DAT`, relative to the end of the 14-byte header |
| 4 | u16 | object id |
| 6 | u8 | object type (see table below) |
| 7 | u8 | storage flags (bit 0 = compressed) |

The **(type, id)** pair identifies the object. The same id repeats across different types (example: id 1000 exists both as a bitmap and as a string list).

**Source:** verified fact. File sizes match `0x14 + 8·N` in all four indexes, and every position lands exactly on a `.DAT` record.

## `.DAT`: records

File header (14 bytes):

| Offset | Type | Field |
|---|---|---|
| 0 | u32 | `0x12345678` (signature) |
| 4 | u16 | always slightly larger than the number of `.NDX` entries (example: 0x19c versus 406) [meaning unknown] |
| 6 | u16 | unknown |
| 8 | u32 | unknown (values from 1 to 5) |
| 12 | u16 | 0 |

Then come the records, one after another:

| Offset | Type | Field |
|---|---|---|
| 0 | u32 | `0x12345678` |
| 4 | u16 | 0 in every record |
| 6 | u32 | content size `S` |
| 10 | `S` bytes | content |

## Compression (flag bit 0)

When bit 0 of the flags byte is set, the content is:

- u16: uncompressed size;
- an Okumura-style **LZSS** stream: 4096-byte circular window and initial write position 4096−18. Each control byte is read from the least significant bit to the most significant: bit 1 = 1-byte literal, bit 0 = 2-byte reference `a b`, with position = `a | (b & 0xF0) << 4` and length = `(b & 0x0F) + 3`.

**Source:**
- Verified fact: with this algorithm, all 828 compressed records of the four databases decompress to exactly the declared size and consume the whole content (`tools/lzss_probe.py`).
- The names `_InitTree`, `_InsertNode`, `_DeleteNode`, `_Encode` and `_Unpack` in `SIMANTW.SYM` (segment `SIMTWO_MODULE`) correspond to Okumura's binary-tree encoder.
- Initial window fill: fact seen in the code. `_UnpackInit` (`1030:a62a`) fills positions 0 to 0xFED with 0x20 (space) and starts writing at 0xFEE. `_Unpack` (`1030:a668`) reads the control bits from least to most significant and computes the length as `(b & 0x0F) + _pack_threshold + 1`, with `_pack_threshold` = 2 [the value 2 was deduced from the validation above]. This matters: with zero instead of space, 67 bitmaps would come out different.

Other values observed in the flags byte: 0, 1, 5 and 8. Bit 2 (value 4) and bit 3 (value 8) appear in bitmaps and sounds, but have no known meaning yet [hypothesis: they indicate discard/cache in `_mem_MakeDiscardable`].

## Object types

| Type | Use | Evidence |
|---|---|---|
| 0 | binary window layout, per video mode (`WINGANT`/`MWINNT`, ids 0–129) | the ids match those of type 1; structure not yet decoded |
| 1 | name table of a window's objects: 32-byte records with an ASCIIZ name (`editWin`, `editWinTitle`, …) | dump of `SHARED` 1/0 |
| 2 | bitmap, see [bitmap.md](bitmap.md) | `_db_LoadObject(id, 2, 1)` in `GR_MODULE`; images rendered correctly |
| 4 | string list (big-endian u16 with the count, followed by Pascal strings; a literal `\n` marks a line break) | dump of `SHARED` 4/1000 ("BackYard View", …); `_LoadStringAnt` (`1020:03f4`) reads only the low byte of the count and makes each string one line (parser: `engine/src/formats/textos.rs`) |
| 6 | main window menu (titles and items, see [menu.md](menu.md)) | `_db_LoadObject(param_1, 6, …)` followed by the message "Couldn't load menu %d"; object 6/0 decoded |
| 9 | 16×16 4 bpp tilesets: ground (ids 9 and 10) and life (ids 15–20), see [tiles.md](tiles.md) | `_OverlayTileSet`, `_LoadTiles` |
| 10 | running text of the information system (ASCII) | dump of `SHARED` 10/150 |
| 17 | information system card layout (big-endian, `PICT`/`TEXT` items) | dump of `SHARED` 17/127 |
| 18 | music header (18 bytes, big-endian), see [som.md](som.md) | `_db_LoadObject(x, 0x12, 1)` before playing |
| 19 | information system card name (ASCIIZ, e.g. `Top_Card`) | dump |
| 20, 50 | standard MIDI file (`MThd`…): 20 = Sound Blaster, 50 = PC speaker, see [som.md](som.md) | signature; `_db_LoadObject(x, 0x14 / 0x32, 1)` |
| 5 | sound effect: 8-bit samples as 4-bit deltas, 4096 Hz, see [som.md](som.md) | `_UncompressDACInstrument`, `_myBeginSound` |
| 51 | sound effect in a Macintosh `snd ` resource (only with `DSOUND.DLL`), see [som.md](som.md) | dump; `_myBeginSound` |
| 7, 8, 11, 15, 21, 22 | unknown | — |

The big-endian values (counts in string lists, cards) indicate that this data came from the Macintosh version (the SYM also has `_MacTickCount`). The bitmap headers and the database itself, on the other hand, are little-endian.

## Impact on mods

Since each object is addressed by (type, id), a mod can override, for example, just string list 4/1220 or just bitmap 2/7000, without touching the original files. The port's loader must check the mods first and only then the original database.
