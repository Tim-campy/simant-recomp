*Leia em português: [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md)*

# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/). The port version and the **mod API** version
(`docs/api-mods.md`) are numbered separately.

> **Draft** for review (see `docs/lancamento.md`). The date and the number only become final when the
> `v1.0.0` tag is created; until then the `Cargo.toml` files stay at `0.1.0`.

## [1.0.0] — unreleased

First version for players. A reimplementation of **SimAnt: The Electronic Ant Colony** (Maxis, 1991,
Windows 3.x version) in Rust + SDL2 that reads the original game files from the user's own copy. No
game file is distributed; the game's texts are also read from the user's installation.

### The game

- **The original's four modes:** Tutorial (lessons 1 to 57), Quick Game, Full Game (the yard and the
  house, with patch switching) and Experimental (tools to dig, fill, food, rocks, ants, scent,
  insecticide and the hold-down tools menu).
- **Simulation identical to the original, tick by tick:** 221 tick vectors recorded from SimAnt running,
  2 long sequences (200 and 600 ticks), 21 interface actions, the tutorial Load and 20 rule cases
  checked bit for bit (307/307 state blocks); the generation of the Quick Game, of the 12 house patches
  and of a new Full, Experimental and Tutorial game also matches before the first tick.
- **Yellow ant:** walking, eating, digging, recruiting and releasing workers, alarm, trophallaxis, body
  switching, fights and the original's ten deaths; enemies (antlion, spider, including the player's
  spider with AutoTrack, caterpillar, pill bug), yard (boy, lawnmower, insecticide, cat, dog, bird,
  rain, mating flights) and end of game.
- **The original's screens and windows:** title and "Select a Game", edit window, map, mini map, yard
  (yard, house, strategic and graph modes), Caste Control, Behavior Control, History, Info (51 cards),
  Examine, speech balloons ("Silly" and the boy's 23), window 26 frames, Windows 3.1 menu bar with
  About, Status, Quit and Load Game, the windows' system menu, ribbon with configurable buttons,
  bookmarks and health bars. Compared pixel by pixel with captures of the original (93–100%).
- **Save and load** in the original's `.ANT` format (the file written by the port opens in SimAnt).
- **Help:** a viewer for `SIMANT.HLP` (WinHelp 3.0) and context help (Shift+F1, *?* buttons).
- The original's **cheat codes** (`heal`, `just`, `qeen`, `eggs`...).
- **Sound and music:** the original's effects and 28 songs. Music plays through a simple synthesizer by
  default; optional: our own FM synthesizer in the style of the Sound Blaster's OPL
  (`--sintetizador fm`), with the instrument bank tuned against recordings of the original, and a
  General MIDI SoundFont (`--soundfont`). Effects sound as on the
  original's Sound Blaster 16 (5000 Hz, in the driver's blocks), measured in a recorded session.
- **The original's pace:** Windows 3.1's 55 ms clock, slow/normal speeds measured on the original
  (3 and 6 ticks/s), fast and ultra.

### For today's systems

- Sharp window: DPI-aware, integer scaling, resizable.
- 640×480 (default), 800×600 or 1024×768 virtual screen, with the layout the original uses in each.
- Windows 3.1 title bar, on by default (`--sem-moldura` turns it off), replacing the system frame:
  borderless window, dragged by the title, maximize/restore/minimize and system menu acting on the real
  window (also with a dialog open and in "Select a Game"), 4-pixel thick frame flush with the edges when
  the window is restored, Alt+F4 always asks the Quit question (`--moldura-do-sistema` keeps the system
  border). Tests and exports run without it. User's choice. In the help viewer, closing is Ctrl+F4
  (Alt+F4 belongs to the game only).
- Fast and ultra speeds at 2.5× and 5× normal (user's choice; `--velocidades original` returns to the
  pace of the test bench's 486) and simulation at slow speed while the window is out of focus
  (`--sem-foco original` keeps the pace); the tick, the save and the Speed menu don't change. Checkboxes
  in the launcher.
- Simple synthesizer as the default for music (user's choice; FM is still available with
  `--sintetizador fm`).
- Preferences kept between games in the user folder (in place of `WIN.INI`'s `[SimAnt]` section).
- Keys and options in data files (`data/*.json`).
- Command line for running without a window (`--ticks`, `--salvar`, screen and sound export).
- **Launcher:** `simant` opened without options shows a startup options screen: SimAnt installation
  folder (system picker, checked and remembered), start (title, new game of each type or save), video,
  driver profile, sound and synthesizer, mods (folder and each mod on or off), preferences and the
  equivalent command line, editable. The choices are kept in `launcher.json` and also apply to `simant`
  run with options (which win).
- On Windows, `simant.exe` doesn't open a console: messages go to `simant.log` in the user folder, fatal
  errors to a message box, `simant-console.bat`/`--console` to see everything live.
- The package works from any folder: `data/` and `mods/` are looked up next to the executable.
- New options: `--launcher`, `--config`, `--sem-config`, `--console`, `--perfil-driver`,
  `--desligar-mods`, `--com-som`, `--capturar-launcher`.
- Documentation on GitHub in English, with every document also available in Portuguese
  (`README.pt-BR.md`, `docs/pt-BR/`).

### Mods

- **Mod API**, frozen at 1.0 at release (0.x versions during development; see `docs/api-mods.md`): replacement of objects in
  the original database, of the game's loose files and of 42 `data/*.json` files of the port, merged by
  key; sandboxed **Lua 5.4** scripts, with hooks outside and inside the tick, reading and validated
  writes of the state (food, scents, castes, ants in the nests), their own menu and panel and state
  saved next to the save (`JOGO.ANT.mods.json`).
- Example mods in `exemplos/mods/`: `exemplo-lua` (food rain) and `trapacas` (F12 menu with rain,
  mating flight, boy, cat, end of game and debug panel).

### Known limitations

- You need the original SimAnt for Windows 3.x (the `.DAT`/`.NDX` files of the `ANTWIN` folder).
- The help viewer's internal dialogs (Open, Print, Annotate...) don't exist.
- Some 1-pixel details in a few drawings (cat, baseline of the end-of-game texts, History graphs) and
  the menu highlight in 256 colors still differ from the original.
- The pace of the fast and ultra speeds was not measured on a real machine.
- The original's networking (NetBIOS/DDE) is out of scope.
- The full list is in `docs/estado-do-projeto.md` §3–§5 and `docs/divergencias.md`.

## Development (0.x, unreleased)

Summary of the repository history (338 commits, October 16 to 19, 2026); the details are in
`git log`, `docs/decisoes.md` and the compatibility history in `docs/api-mods.md`.

- **2026-10-16 — Milestone 1:** initial specification, parsers for the `.NDX/.DAT` database with LZSS,
  bitmaps, fonts and `.ANT` save; title screen in Rust + SDL2; choice of Rust + SDL2 and Lua; automated
  QEMU + SvarDOS test bench that reads and writes the original game's variables and records tick vectors
  tick by tick.
- **2026-10-17:** tick rules (pheromones, surface, nests, enemies, yard, water, advice, tutorial)
  checked against the tick vectors; sound and music; game modes, yard, patch switching, house and
  Experimental; Caste/Behavior/History, Info, Examine, map, Load/Save As windows; Lua mod scripts
  (API 0.12); CI on GitHub Actions.
- **2026-10-18:** end of game inside the tick, tutorial Load, complete tutorial on the original, sharp
  window and video options, player preferences, Windows menu bar, help (`SIMANT.HLP`), "Silly"
  balloons, tools menu, window 26 frames, spider drawing; API 0.17–0.25 with the cheats mod.
- **2026-10-19:** API 0.26 (merge by key, hooks inside the tick, state in the save); 640/800/1024
  virtual screen and driver profile; `WIN.INI` as the source of preferences; FM synthesizer (default
  for a while; the simple one is the default at release) and bank tuned to the original's 28 songs; optional Windows 3.1 title bar; seventh and eighth rounds
  of tick vectors (rare branches, red strategy, recruiting, mating flight) and generation checked;
  API 0.34. SimAnt's texts out of the repository: the `data/*.json` files only keep the reference
  (database, executable, window layout, `.HLP`) and the port reads the text from the user's
  installation; API 0.35.
