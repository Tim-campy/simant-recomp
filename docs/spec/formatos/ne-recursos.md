*Leia em português: [ne-recursos.md](../../pt-BR/spec/formatos/ne-recursos.md)*

# NE executable resources (`SIMANTW.EXE`): cursors and accelerators

Reader: `engine/src/formats/ne.rs` (`cursores(exe)`, `aceleradores(exe)`), tests `cursores_do_simantw` and `aceleradores_do_simantw`. Inspection: `python tools/ne_info.py SIMANTW.EXE` lists the resources.

## Resource table (Windows 3.x NE format)

- `MZ` at the start; at 0x3C, the offset of the `NE` header; at `NE + 0x24`, the offset (relative to `NE`) of the resource table.
- Table: `u16` alignment shift (offsets and sizes are in units of `1 << alignment`); then, per type: `u16` type (bit 0x8000 = number), `u16` count, `u32` reserved, and the 12-byte entries: `u16` offset, `u16` size, `u16` flags, `u16` id (bit 0x8000 = number; otherwise, offset of a Pascal name from the start of the table), `u32` reserved. Type 0 ends the table.

## Cursors

- `GROUP_CURSOR` (12): `u16` reserved, `u16` type (2), `u16` number of images; 14-byte entries, with the id of the `CURSOR` resource in the last 2.
- `CURSOR` (1): `u16` x and `u16` y of the hotspot, then a `BITMAPINFOHEADER` (doubled height), the 2-color palette, the XOR mask and the AND mask, 1 bit per pixel, 4-byte rows bottom to top. AND 1 + XOR 0 = transparent; AND 0 = black (XOR 0) or white (XOR 1); AND 1 + XOR 1 = invert.

In the user's `SIMANTW.EXE` (verified on 2026-10-18): 8 named groups — `HELPCURSOR`, `MAGCURSOR`, `ROCKCURSOR`, `DIGCURSOR`, `ANTCURSOR`, `FOODCURSOR`, `DROPCURSOR`, `SPRAYCURSOR` —, all 32×32. `HELPCURSOR` is the arrow with a "?" beside it, hotspot (0, 0), the same as the cursor in the test bench captures in help mode. Usage: [../telas/ajuda-contexto.md](../telas/ajuda-contexto.md) §5.

## Accelerators and menu

- `ACCELERATOR` (9): 5-byte entries — `u8` flags (1 = virtual key, 4 = Shift, 8 = Ctrl, 0x10 = Alt; 0x80 marks the last), `u16` key, `u16` command. The `ACCELERATORTABLE` of `SIMANTW.EXE` has 47 entries (verified on 2026-10-18): Ctrl+N/L/S/A/Q (0xFD03–0xFD08), Ctrl+E/M/B/C/H/I/T (0xFD11–0xFD17), Ctrl+Y/5/6/7 (0xFD21–0xFD24), Ctrl+8/9/0 (0xFD26–0xFD28), Ctrl+G/J/K (0xFD31–0xFD33), Shift+0..4 and Shift + NumPad 0..4 (0xFD41, 0xFD43–0xFD46), and the window ones 0xFDA0–0xFDAC (janelas.md §8, navegacao.json). The numbers are those of the menu items ([menu.md](menu.md)).
- **There is no `MENU` resource**: the bar is built by `_InitMenu` from database object 6/0 ([menu.md](menu.md)).

## Segments (DGROUP texts)

**Source:** NE format (Microsoft "Executable-File Header Format"); `tools/ne_info.py`; `engine/src/formats/ne.rs`, `segmento`.

The segment table starts at `ne + [ne + 0x22]`, with `[ne + 0x1C]` 8-byte entries: sector (u16), size in file (u16, 0 = 64 KB), flags (u16; bit 0 = data) and minimum allocation (u16). The segment starts in the file at sector << `[ne + 0x32]` (alignment). The numbering is that of `SIMANTW.SYM` and of the `n:offset` addresses in the documentation, starting at 1 (segment 10 = DGROUP, selector 1048; segment 8 = `SIMANT_DATA_GROUP`, selector 1038). The file bytes are the initialized data; relocation comes afterward and is not included.

The port reads from here the DGROUP texts it uses (mod API 0.35: references `{"exe": "SIMANTW.EXE", "segmento": 10, "offset": ...}` in the `data/*.json`; `docs/api-mods.md`, "Textos do original"), as ASCIIZ strings: for example `10:014b`–`10:01bc` (the four speed texts and the two bookmark texts, `_DoUserButton`/`_DoBookMark`), `10:0900`/`10:0911` (pause item, `_SetMenuOptionText`), `10:0626` (main window title, `CreateWindow` in `_InitInstance`), `10:1650`–`10:185a` (file boxes and dialogs of `_MenuQuit`, `_LoadGame`, `_FileSelect`, `_PopMsg`) and the balloon pieces at `10:1c6e` (11 × 16 bytes; `_MakeBalloon`). Checked: with the reference installation's `SIMANTW.EXE`, each text read is equal to what the `data/*.json` used to contain (`engine/tests/textos_originais.rs`).
