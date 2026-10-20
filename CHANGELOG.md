*Leia em português: [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md)*

# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/). The port version and the **mod API** version
(`docs/api-mods.md`) are numbered separately.

## [1.0.0] — 2026-10-20

First version for players. A reimplementation of **SimAnt: The Electronic Ant Colony** (Maxis, 1991,
Windows 3.x version) in Rust + SDL2 that reads the original game files from the user's own copy. No
game file is distributed; even the game's texts are read from the user's installation. Licensed
**MIT OR Apache-2.0** (the license covers only the port's code and data, never the SimAnt files).

### The game

- **The original's four modes:** Tutorial (lessons 1 to 57), Quick Game, Full Game (the yard and the
  house, with patch switching) and Experimental (tools to dig, fill, food, rocks, ants, scent,
  insecticide and the hold-down tools menu).
- **Simulation identical to the original, tick by tick:** 223 tick vectors recorded from SimAnt running,
  2 long sequences (200 and 600 ticks), 21 interface actions and the tutorial Load, all checked bit for
  bit (307/307 state blocks), plus 21 rule cases from the test bench (20 confirmed in the original and
  equal in the port; one cannot be checked). The generation of the Quick Game, of the house and yard
  patches and of a new game of each type also matches the original before the first tick.
- **Yellow ant:** walking, eating, digging, recruiting and releasing workers, alarm, trophallaxis, body
  switching, fights and the original's ten deaths; **death, lost fight and rebirth are presented in the
  original's order** (the same timeline of frames and messages as the original). Enemies
  (antlion, spider, including the player's spider with AutoTrack, caterpillar, pill bug), yard (boy,
  lawnmower, insecticide, cat, dog, bird, rain, mating flights) and end of game with the original's
  score and rating.
- **The original's screens and windows:** title and "Select a Game", edit window, map, mini map, yard
  (yard, house, strategic and graph modes, with ribbon 35's bars and populations), Caste Control,
  Behavior Control, History, Info (51 cards), Examine, speech balloons ("Silly" and the boy's 23),
  window 26 frames, Windows 3.1 menu bar with About, Status, Quit and Load Game, the windows' system
  menu, ribbon with configurable buttons (including the spider and Status buttons), bookmarks and
  health bars. Compared pixel by pixel with captures of the original (93–100%).
- **Complete Load/Save As dialogs**, like Windows 3.1's common dialog: dragging by the title, drop-down
  lists for file type and drive, full scrolling of the lists, folder change, Read Only and editing of
  the file name.
- **Save and load** in the original's `.ANT` format (the file written by the port opens in SimAnt).
- **Help:** a viewer for `SIMANT.HLP` (WinHelp 3.0) that is a **Windows window** of its own (system
  menu, move, resize, minimize, maximize; Ctrl+F4 closes it), with Search and History, and context help
  (Shift+F1, *?* buttons).
- **Cheats:** the original's 26 cheat codes (`heal`, `just`, `qeen`, `eggs`...) and, as an example
  mod, a cheat menu (section "Mods").
- **Sound and music:** the original's effects and 28 songs. Music plays through a **simple
  synthesizer by default** (user's choice); optional: our own FM synthesizer in the style of the Sound
  Blaster's OPL (`--sintetizador fm`), with the instrument bank tuned against recordings of the
  original, and a General MIDI SoundFont (`--soundfont`). **Effects play at 5000 Hz in blocks of 504
  samples**, as the Sound Blaster 16 driver plays them (measured in a session of the original played in
  DOSBox-X: an effect shorter than a block is silent, and the rest of the last block is lost).
- **The original's pace:** Windows 3.1's 55 ms clock and the slow/normal speeds measured on the
  original (3 and 6 ticks/s).

### For today's systems

- Sharp window: DPI-aware, integer scaling, resizable.
- **640×480** (default), **800×600** or **1024×768** virtual screen, with the window layout the
  original uses at each one, and **driver profiles** (16-color VGA, the default; 256-color SVGA; the
  DOSBox-X S3), which change the byte alignment of windows and the system colors as each driver does.
- **The Windows 3.1 title bar is the window frame by default**, replacing the system's: borderless
  window, dragged by the title, maximize/restore/minimize and system menu acting on the real window
  (also with a dialog open and in "Select a Game"), 4-pixel thick frame when the window is restored,
  Alt+F4 always asks the Quit question. `--moldura-do-sistema` keeps the system border around it and
  `--sem-moldura` turns it off. Tests and exports run without it.
- **Proportional pace** (user's choice): Fast at 2.5× and Ultra at 5× Normal, and the simulation at
  slow speed while the window is out of focus. For the original's behavior: `--velocidades original`
  (the pace of the test bench's 486) and `--sem-foco original`, also as checkboxes in the launcher. The
  tick, the save and the Speed menu do not change.
- **Launcher in English and Portuguese:** `simant` opened without options shows a startup options
  screen: SimAnt installation folder (system picker, checked and remembered), start (title, new game of
  each type or a save), video, driver profile, sound and synthesizer, mods (folder and each mod on or
  off), preferences and the equivalent command line, editable. The choices are kept in
  `launcher.json` and also apply to `simant` run with options (which win).
- **Languages:** the port's own texts (launcher, messages about mods, console) go through a language
  frame in `data/idiomas/` that can be extended, also by mods (`docs/i18n.md`). The game is in English,
  like the original; its texts come from the user's installation.
- **No console on Windows:** `simant.exe` writes its messages to `simant.log` in the user folder and
  fatal errors to a message box; `simant-console.bat`/`--console` show everything live.
- Preferences kept between games in the user folder (the first time, read from the `[SimAnt]` section
  of the `WIN.INI` near the game folder, when there is one).
- Keys and options in data files (`data/*.json`).
- Command line for running without a window (`--ticks`, `--salvar`, screen and sound export).
- The package works from any folder: `data/` and `mods/` are looked up next to the executable.
- **Documentation in English**, with every document also available in Portuguese
  (`README.pt-BR.md`, `CHANGELOG.pt-BR.md`, `docs/pt-BR/`); a link checker runs in CI.

### Mods

- **Mod API 1.0** (frozen at this release; it is 0.43 under its final number; from now on 1.x versions
  only add compatible things, and an incompatible change makes 2.0; see `docs/api-mods.md`):
  replacement of objects in the original databases, of the game's loose files and of the port's
  `data/*.json` files, merged by key; language files in `data/idiomas/` (keys or a whole language);
  sandboxed **Lua 5.4** scripts, with hooks outside and inside the tick, reading and validated writes
  of the state (food, scents, castes, ants in the nests), their own menu and panel, state saved next to
  the save (`JOGO.ANT.mods.json`) and texts in the game's language (`idioma()`, `traduzir()`).
- Example mods in `exemplos/mods/`: `exemplo-lua` (food rain) and `trapacas` (cheat mod: F12 menu with
  rain, mating flight, boy, cat, food, end of game and a debug panel), both with English texts and a
  Portuguese example.

### Verified for this release

- `cargo test --workspace` with the game files: **693 tests passed, 0 failed** (1 ignored by default:
  it opens the Windows folder dialog), in 41 test binaries; also without the game files (the tests that
  need them warn and pass). `cargo clippy --workspace --all-targets -- -D warnings`: no warnings.

### Known limitations

- You need the original SimAnt for Windows 3.x (the `.DAT`/`.NDX` files of the `ANTWIN` folder, plus
  `SIMANTW.EXE` and `SIMANT.HLP`, from which the texts are read).
- **The game is only in English for now** (like the original); only the launcher has Portuguese.
- **An effect that cuts off another one does not always follow the rule of the driver's blocks:** the
  504-sample blocks were measured for effects that play to the end; when one effect interrupts another,
  the port's cut does not always land where the Sound Blaster 16 driver would cut.
- **The Fast and Ultra pace is the port's choice** (2.5× and 5× Normal), not a measurement: on the
  original they depend on the machine's speed (`--velocidades original` gives the pace of the test
  bench's 486).
- **The dog's bark, the bird, the spider's dialogs and trophallaxis have no audio check:** their
  sounds were not compared with a recording of the original, as the other effects were.
- **Linux was not tested by hand:** the package is built and smoke-tested (without the game files) by
  the release workflow, but the game, the launcher and the file pickers were not run on a Linux machine
  with the game files.
- The help viewer's internal dialogs (Open, Print, Annotate...) don't exist.
- Some 1-pixel details in a few drawings (cat, baseline of the end-of-game texts, History graphs) and
  the menu highlight in 256 colors still differ from the original.
- The original's networking and DDE (NetBIOS, links to other programs) are out of scope.
- The full list is in `docs/estado-do-projeto.md` §3–§5 and `docs/divergencias.md`.

## Development (0.x, unreleased)

Summary of the repository history (422 commits, October 16 to 20, 2026); the details are in
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
  virtual screen and driver profile; `WIN.INI` as the source of preferences; FM synthesizer and bank
  tuned to the original's 28 songs; Windows 3.1 title bar; seventh and eighth rounds of tick vectors
  (rare branches, red strategy, recruiting, mating flight) and generation checked; license; launcher
  and console-less executable; SimAnt's texts out of the repository, read from the user's installation
  (API 0.35); death timeline in the original's order (0.37); proportional pace, slow without focus,
  simple synthesizer as the default and the title bar replacing the system frame (0.38); ribbon 35 bars
  (0.39); help as a Windows window, complete file dialogs, spider and Status buttons (0.40); the title
  bar on by default; mod food and stones inside the house (0.41); effects at 5000 Hz in 504-sample
  blocks, measured in a played session (0.42).
- **2026-10-20:** documentation translated into English, with the Portuguese copies in
  `docs/pt-BR/`, and a link checker in CI; language frame (`data/idiomas/`), launcher in English and
  Portuguese and the port's texts in English (API 0.43); release 1.0.0 with the mod API frozen at 1.0.
