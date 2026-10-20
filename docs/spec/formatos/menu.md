*Leia em português: [menu.md](../../pt-BR/spec/formatos/menu.md)*

# Main window menu (database object type 6)

Reader: `engine/src/formats/menu.rs` (`MenuBanco::decodificar`, `comando`), tests `objeto_sintetico` and `numeracao_dos_comandos`. Usage: [../telas/menus.md](../telas/menus.md).

`SIMANTW.EXE` has no MENU resource: `_InitMenu(0)` (`1008:4e36`) reads object **6/0** from the database (`SHARED`) and fixes up the pointers by adding the block address, before building the bar with `AppendMenu` [code].

## Format (verified on object 6/0, 726 bytes, 2026-10-18)

- At the start, a list of little-endian `u32` pointers (offsets from the start of the object), terminated by 0. The first points to the list of **titles**; the following ones, one per title, to the list of **items** of that menu.
- Each list is of `u32` pointers to ASCIIZ texts, terminated by 0.
- Texts: `&` before the underlined letter, `\t` (0x09) before the shortcut shown; `---` = separator.

In 6/0: titles `&File`, `&Window`, `&View`, `&Options`, `&Speed`; items:

| Menu | Items |
|---|---|
| File | `&About`, `---`, `&New\tCtrl+N`, `&Load...\tCtrl+L`, `&Save\tCtrl+S`, `Save &As...\tCtrl+A`, `---`, `&Quit\tCtrl+Q` |
| Window | `&Edit\tCtrl+E`, `&Map\tCtrl+M`, `&Behavior\tCtrl+B`, `&Caste\tCtrl+C`, `His&tory\tCtrl+H`, `&Info\tCtrl+I`, `&Status\tCtrl+T`, `&Help\tF1` |
| View | `&Yard\tCtrl+Y`, `&House\tCtrl+5`, `&Strategic\tCtrl+6`, `&Graphic\tCtrl+7`, `---`, `&Surface\tCtrl+8`, `&Black Nest\tCtrl+9`, `&Red Nest\tCtrl+0` |
| Options | `&AutoTrack\tCtrl+G`, `&Music\tCtrl+J`, `&Sound Effects\tCtrl+K`, `&Events`, `Messa&ges`, `Si&lly` |
| Speed | `&Pause   \tShift+0`, `---`, `&Slow\tShift+1`, `&Normal\tShift+2`, `&Fast\tShift+3`, `&Ultra\tShift+4` |

## Command numbers

`_InitMenu` gives each item the number `0xFD01 + 16 × menu + item` (separators included): About 0xFD01, Quit 0xFD08, Status 0xFD17, Pause 0xFD41 etc. They are the same numbers as the executable's ACCELERATORTABLE ([ne-recursos.md](ne-recursos.md)).

**Submenus** [code, unused in 6/0]: an item whose text has a marker character followed by a number would load another type 6 object as a simple list (`_ms_LoadPopUpResource`: count in byte 1, Pascal strings), with numbers `0xF9x0 + item`.

**Mods:** object 6/0 can be replaced by a mod like any database object (texts and shortcuts shown); what each number does is in `data/barra_menus.json` (`comandos`).
