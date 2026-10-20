*Leia em português: [menus-mods.md](../../pt-BR/spec/telas/menus-mods.md)*

# Mod script menu and panel (port extension)

**Does not exist in the original.** It is the screen part of mod API 0.17 (`docs/api-mods.md`, "Lua scripts"): a script can have a menu opened by a key and a text panel. The example cheat mod (`exemplos/mods/trapacas/`) is the first use. The base game has no code specific to any mod.

## Behavior

- **Key.** `definir_menu` registers the menu key (F1–F24, or a combination with Ctrl or Alt). On the game screen, key order is the original's (accelerators from `partida.json` and `navegacao.json`, window keys from `janelas.json`) and only then the mod menus' keys; next `_YellowCommandKey` and the ring of the original's cheat codes (`regras/trapacas.md`). A mod key equal to a game shortcut never opens the menu; `simant` warns on the console when opening the game.
- **Menu.** Opens centered in x, just below the ribbon, on top of all windows and notices. Windows 3.1 drop-down menu look: white background, black outline, blue title bar with the title and the key, highlighted item in dark blue with white text, gray separators. If the items do not fit in the client area's height, the menu wraps into columns (with a gray line between them), never starting a column with a separator. Geometry, fonts and colors: `data/menus_mods.json`.
- **Input.** With the menu open, it receives all input: the mouse highlights the item under the cursor, a click chooses (outside the menu closes), arrows move up and down skipping separators (wrapping around), Enter chooses the highlighted one, Esc closes. The simulation stays stopped while the menu is open (like the message queue of a Windows menu).
- **Choice.** Closes the menu and calls the owning mod's `ao_menu(indice, texto, estado)` hook. The actions requested in the hook are executed right after (`core/acoes_mods.rs`) and the events they generate (sounds, music, message boxes) are handled as after a tick.
- **Panel.** `definir_painel` shows up to 16 lines in the corner chosen in `menus_mods.json` (default: top right, below the ribbon), with the mod's name; one panel per mod, one below the other. It sits below the menu and on top of the windows.
- **Text.** Script texts (UTF-8) are converted to the font's encoding: Latin-1 in Windows fonts (FNT) and Mac Roman in Macintosh fonts (FONT); a letter without a glyph becomes the unaccented letter.

## Implementation

`engine/src/core/menus_mods.rs` (geometry, input and drawing, headless), `simant/src/mods_ui.rs` (keys, SDL events and drawing on the game screen). For headless tests: `simant --save ARQ --mods DIR --menu mod@F12 --exportar-tela-jogo tela.png` (`mod@F12#N` chooses item N before exporting).
