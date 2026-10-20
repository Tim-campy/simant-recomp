*Leia em português: [info.md](../../pt-BR/spec/telas/info.md)*

# Info ("Top Card", window 5)

A static encyclopedia about ants, in cards with pictures, texts and click areas. It does not read the game state nor do random draws. Sources: Ghidra pseudocode (analysis of 2026-10-18), the SHARED/WINGANT resources and the capture `re_trabalho/janelas/j03_info.png`.

## 1. Opening (`_OpenInfoWindow`, `1018:60da`) [code]

Through the menu Window → Info (`_ProcMenu` command 0x16), through function 3 of the configurable buttons (`_DoUserButton`) and through object 0x115 of the map window. If window 5 is not yet open: current card (DGROUP `a456`) = 128, history[0] (`a434`) = 128, history index (`a454`) = 0. Then, always `_win_Open(0x500)` (open, keeps the card).

## 2. Window 5 (`infoWin`) [data]

Object 0: outline, 324 × 220, entry 5 of the type 9 table at (277, 128) (+ the ribbon). `infoTitle` (0x501, type 18): the title bar; `infoArea` (0x502, type 2): the card area (320 × 200).

## 3. The card (`_win_DrawInfoWindow` `1018:534e` → `_DisplayCard` `1018:5a1c`) [code + data]

53 cards, ids 127 to 179 (128 "Top_Card", 129 "Index", 130 "Ant_Anatomy" …). For card `id`:

- **Title**: the type 19 resource `id` (text with '_' in place of spaces), written in `infoTitle`.
- **Body**: the type 17 resource `id`, with **big-endian** 16-bit integers (a Mac legacy): a 4-byte header (card height and width, 200 and 320) and a sequence of records with a 4-letter tag:
  - `PICT`: bitmap, top, left, bottom, right, `nHot`, and `nHot` × (top, left, bottom, right, value). Draws the bitmap (type 2, from any bank) at `infoArea` + (left, top) and registers the click areas.
  - `TEXT`: top, left, bottom, right, `n`, and `n` text ids. Each text (type 10 resource) is written in the rectangle with the styles of the type 21 resource with the same id (see "Text" below). In the 53 cards, every `TEXT` has a single text [data].
  - Any other tag: end (the original logs "Unknown Resource").
- **Text cleanup** (`_DisplayCard`) [code]: the type 10 resource may have a 0 byte in the middle, followed by control bytes (Mac leftovers: `socia\0\0\x01l insects` in text 310, `h\0\0\x01\0ave` in 420, two cuts in 212). While there is a 0 before the resource's last 4 bytes, the original removes that 0 and the following bytes smaller than 0x20 (signed `char` comparison: bytes ≥ 0x80 right after it also go), and moves back the starts of the style runs that came after. The text ends at the first remaining 0. **This was the port's bug** (reported on the cards Ants, Mouth/"Mandibles" and Soldier): the port stopped at the first 0 and cut off the text.
- **Styles** (type 21, the Mac's `styl`): number of runs (16 bits) and, per run, 20 bytes: start (32 bits), line height, ascent, font, faces (byte) + 1 byte, size, color (16-bit RGB). The run uses the second font when the faces word is 0x0100 (bold only) [code: comparison with 0x100 in `_win_PrintStyleTextInRect`]. Faces in the cards [data]: 0 (108 runs), 1 = bold (55), 4 = underline (8, drawn with the normal font, without underline).
- **Text** (`_win_PrintStyleTextInRect`, `1018:5368`) [code]: starts at (left + 1, top + 4); at most (bottom − top) / height lines, with height = the larger of the two fonts. Each line adds up the width of each character while the sum stays **smaller** than the rectangle's width, keeping the last break point of a character that fit: before a space, '(', '[', '{' and end of line (0, '\n', '\r'); after ')', ']', '}', '?', '!', ',', '-', '.'. End of line and end of text (the 0 is also measured) close the line. The line ends at the break point (with no break point: at the character that exceeded the width, inclusive); then spaces and one end of line are skipped. On entering a style run, the word starting there is measured with the previous font: if it fits, what came before is written and the font changes; if not, the line ends one character before. Consequence: the word only stays on the line if it **and the following space** fit (the "using" of j03).
- **Text links** [code + data]: with the last parameter on, each bold run written is uppercased and looked up in a table of 0x22-byte entries (32-byte name + id) at `DGROUP:a458`; if found, it registers the area in table `DGROUP:a1b2` (counter `a432`). Table `a458` is **zeroed** in `SIMANTW.EXE` (segment 10, file offset 0xa458 + 131072) and the decompiled code has no write to it: the original looks it up, does not find it and only logs "ERROR: Cannot find text rez". So **bold words do not open a popup** in the original [code + data; not tested by clicking on the bench]. The port does not register links either.
- Fonts: the game's font 2 for the text and font 5 for the highlighted runs (`_PopUpInfoWindow` calls `_win_GetStyleTextHeight(…, 2, 5)`) [code; the card body with the same fonts matches the bench pixel by pixel].

Example, card 128 [data]: PICT 21005 (bottom strip, (180, 0)–(200, 320)) with an area (184, 128)–(197, 192) of value 1 (PREVIOUS); PICT 151 (the ant) at (32, 109); PICT 150 at (29, 4) with 8 areas for cards 134, 130, 157, 139, 146, 152, 167 and 171; TEXT (1, 176)–(180, 320) with text 150.

## 4. Clicks (`_ProcInfoEvent`, `1018:65dc`) [code]

Only in `infoArea`. First the `PICT` areas: value 1 = back in the history (ring of 16 entries, only if the previous one ≠ 0); value > 1 = goes to the card with that id (stores it in the ring and clears the next entry); value ≤ 0 = popup of definition −value (27 areas in the cards, ids 911–967, all with text). Otherwise, the text links (empty, see above).

**Popup** (`_PopUpInfoWindow`, `1018:610c`) [code + bench p911/p912]: type 10/21 text of the id, while the button is held (`_StillDown`); on release, the window disappears. Computations, relative to `infoArea` (which, here, starts 1 pixel to the left of the card's drawing origin and measures 319 × 199 [measured]):

- text width `L` = half the area's width (159); centered on the middle of the clicked area; clamped 4 pixels from the edge (on the left: from 4 to 4 + `L` + 8; on the right: up to width − 4, starting 8 before);
- height `h` = lines (`_win_GetStyleTextHeight`: the same breaking, but the font changes on reaching the run, without measuring the word) × height + 5;
- below the clicked area (top = area's bottom + 4); if area height − 10 < `h` + bottom + 4, above (bottom = area's top − 4);
- child window (`WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_DLGFRAME`, class "GenericWindow") with x = left − (`SM_CXDLGFRAME` + 4), width = `L` + `SM_CXDLGFRAME` + 4 (the margin only enters once: the window ends at the end of the text), y = top − (`SM_CYDLGFRAME` + 4), height = `h` + 2 × (`SM_CYDLGFRAME` + 4), kept inside the area vertically; `SM_C?DLGFRAME` = 4 on the bench. The visible border is 5 pixels: 1 black + 4 blue; white background; text in the rectangle (4, 4) of the client area, width `L` (it goes past the client area and is clipped).

## 5. In the port

`engine/src/core/info.rs` and `data/info.json`: `Carta::ler` (types 17/19), `TextoCartao` (types 10/21 with the cleanup), `compor` (breaking of `_win_PrintStyleTextInRect`), `contar_linhas` (`_win_GetStyleTextHeight`), `JanelaInfo` (current card, history of 16, clicks on areas, popup open until `soltar`, called on button release in `simant/src/jogo.rs`). The window enters the control-window stack (`Controle::Info`); function 3 of the buttons and Ctrl+I (a port extension) open it. **Left out:** the bold-word links (the original does not have them either, see above); underline (face 4) is not drawn [the original also uses the face only to choose the font]; the port's popup does not suspend the rest of the interface like the `_StillDown` loop [hypothesis of small impact].

**Port measurements against the captures** [bench]: the window sits 1 pixel to the right of the common computation for control windows (`janelas.json` → `info.ajuste`); the top line of `infoArea` is the title's line (the card background starts one below); the cards' mono pictures have bit 1 as black ink (`inverter_mono`) and 2/150 has byte-aligned rows (fixed in `formats/bitmap.rs`); the popup uses the area 1 pixel to the left and 1 smaller (`popup.deslocamento_area`, `popup.ajuste_area`).

## 6. Verification (2026-10-18)

Script `tests/referencias/roteiros/qemu_info_cartas.txt` (copy of the `quick` overlay in `re_trabalho/info_cartas/`, save PORT50): captures `c128`, `c134` (Social Insects), `c135` (Ants), `c136` (Termites), `c130` (Ant Anatomy), `c131` (Head), `c160` (Mouth), `c139` (Castes), `c141` (Soldier) and the popups `p912` (Ocelli, opens below) and `p911` (Maxillae, opens above). Port: `cargo run -p engine --example exportar_info -- JOGO SAIDA --tela 280 222 128 134 … 131:912 131:911` and `comparar_captura.py --mapear --regiao 280 223 599 422` (card area): **99.65–99.75%** on all 9 cards; popups with the region (270, 223)–(599, 422): **99.29%** and **99.39%**. The remaining differences are the mouse cursor in the capture and the area's last line (bottom edge of the window). Before the fix: Top Card 96.36% (breaking of "using") and the texts of Ants, Mouth and Soldier cut at the first 0.

Automatic check of the 53 cards: `exportar_info JOGO SAIDA --conferir` (all with a readable picture, non-empty texts after cleanup and popups with text): no problems.

Tests: `core/info.rs` (records of card 128, text cleanup, faces, line breaking, runs, history and popup, geometry of popups p911/p912), `engine/tests/modos_reais.rs` (card 128 read from the database and the 53 cards without cut-off text).

**All reachable cards (2026-10-18):** `tools/gerar_roteiro_info.py` builds, from the click areas, the shortest path from the Top Card to each card and generates `tests/referencias/roteiros/qemu_info_todos.txt` (captures in `re_trabalho/info_todos/`). There are **51 cards** reachable by clicking; compared with `exportar_info … --tela 280 222` in the region (280, 223)–(599, 422): **99.64–99.96%** on all (what remains is the area's last line and the cursor). Observed: card 152 (Common Species) wrote its text only after ~2 s, and a click before that was lost (the first recording stopped there); with 6 s waits (`qemu_info_especies.txt`) cards 152–156 gave 99.65–99.75%.

**Not verified:** cards 127 (duplicate Top Card) and 129 (Index), which no click area reaches from 128; clicking a bold word in the original.
