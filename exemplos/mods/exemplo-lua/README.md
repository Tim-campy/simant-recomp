*Leia em português: [README.pt-BR.md](README.pt-BR.md)*

# Food rain (example mod with a Lua script, mod API 0.12 and 0.43)

Every 500 ticks a "food rain" falls on the surface: 30 tries on cells drawn by the script's own random generator (it does not touch the game's draws), with a message in the strip ("It rained food! (N pieces)").

- Turn it on with `--mods exemplos/mods` (also turns on the cheat mod) or by copying this folder into `mods/`.
- Script: `chuva_de_comida.lua`. API: `docs/api-mods.md`.
- Texts: `data/idiomas/en.json` (English, the fallback) and `data/idiomas/pt-BR.json` (a second example translation), keys `mod.exemplo-lua.*`, read with `traduzir()` in the game's language (`docs/i18n.md`).
