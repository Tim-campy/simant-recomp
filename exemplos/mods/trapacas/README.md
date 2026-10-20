*Leia em português: [README.pt-BR.md](README.pt-BR.md)*

# Cheat mod (example, mod API 0.17, 0.26 and 0.43)

A menu to trigger the game's events on the spot and test the port: rain, the boy mowing the lawn or with bug spray, mating flight, cat, dog, spider, antlion, fight, food, the yellow ant's health, patch change (including the 38 house patches), castes and behavior, alarm, soldiers in the nest, queens kept for the PLACE button, tutorial lessons, game over, Mystery, fast tick advance and a debug panel.

It is an ordinary mod: it only uses the public script API (`docs/api-mods.md`). The base game has no code specific to it. Each action calls the original game's routine that does that thing (the start of the rain from `_DoSimYard`, `_XferPatch`, `_YellowDeath`, `_MysteryButton`...), so the game may refuse as the original would (for example, changing patch only in the Full Game).

## How to turn it on

- `--mods exemplos/mods` (also turns on the food rain example), or
- copy this folder into `mods/` (it stays on until you remove it).

## How to use it

- **F12** opens the menu (to change it, edit `TECLA` in `trapacas.lua`: a function key or a combination with Ctrl/Alt).
- Arrows + Enter or a click choose; Esc or a click outside close it. The simulation stops while the menu is open.
- Every cheat used shows up in the message strip ("Cheat: ...") and is counted in the panel. Refusals show up in the strip and in the console.
- Since API 0.26, the cheat count and the panel go to `NAME.ANT.mods.json`, next to the save (`ao_salvar`/`ao_carregar`); the `.ANT` stays the original's, which reads it normally (and does not know there was cheating). On Load, the strip says how many cheats the game has had.

## Texts (API 0.43)

The mod's texts (menu title and items, strip messages, panel) are not in the script: they come from `data/idiomas/<language>.json` inside the mod (keys `mod.trapacas.*`), read with `traduzir()` in the game's language (language frame, `docs/i18n.md`). The mod brings English (`en.json`, the fallback language and, for now, the game's only one) and a Portuguese translation (`pt-BR.json`) as a second example; it takes effect when the game has pt-BR in the `jogo` context. To translate the mod into another language, a `data/idiomas/<code>.json` with the same keys is enough.

## Difference from the original's codes

The original SimAnt has its own cheat codes (`heal`, `just`, `qeen`...), typed during the game; they are part of the base game and work with or without this mod (`docs/spec/regras/trapacas.md`).
