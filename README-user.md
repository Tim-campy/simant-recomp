# SimAnt recomp — README

Version 1.0.0 (draft). A modern reimplementation of **SimAnt: The Electronic Ant Colony** (Maxis, 1991,
Windows 3.x version) for current Windows and Linux. The game logic is rewritten from scratch; graphics,
sounds, music and help come from **your own copy** of the original SimAnt, which this package does not
contain. (The full guide, in Portuguese, is `LEIAME.txt`.)

> **Legal notice.** This is a non-profit fan project, **not affiliated with or endorsed by Maxis or
> Electronic Arts**. "SimAnt" and "Maxis" are trademarks of their respective owners. This package
> **does not distribute any file from the original game** (graphics, sounds, music, text, executable):
> you need your own copy of SimAnt for Windows 3.x. Please do not ask for or share copies of the game
> in the project's channels.

## 1. Requirements

- **Windows 10/11, 64-bit:** nothing else (`SDL2.dll` is included).
- **Linux x86-64** (glibc 2.35 or newer) with the system **SDL2**: `sudo apt install libsdl2-2.0-0`
  (Debian/Ubuntu), `sudo dnf install SDL2` (Fedora), `sudo pacman -S sdl2-compat` (Arch).
- **The SimAnt for Windows 3.x files**: the folder (usually `C:\MAXIS\ANTWIN`) containing
  `SHARED.NDX/.DAT` and `WINGANT.NDX/.DAT` (required), `SOUND.NDX/.DAT` and the `SOUND` folder (sound
  and music), `SIMANTW.EXE` (cursors; read only, never run), `SIMANT.HLP` (help), `FONTRES.FON` and
  `FONT1`–`FONT4` (fonts). If that folder sits inside a Windows 3.1 install (`...\WINDOWS\SYSTEM` two
  levels up), the system fonts and the `[SimAnt]` section of `WIN.INI` are used too. The port only
  reads these files.

## 2. Setup and running

1. Unpack the archive anywhere.
2. Point the port at the game files: `--jogo "C:\MAXIS\ANTWIN"`, or the `SIMANT_DIR` environment
   variable, or copy the game folder to `SimAnt-T/MAXIS/ANTWIN` inside the package folder.
3. Run from the package folder (it looks for `data/` and `mods/` in the current directory):
   Windows: double-click `simant.exe`; Linux: `./simant.sh`.

```
simant                                  title screen and "Select a Game" (Enter = Full Game)
simant --novo-jogo --tipo completo      new game: tutorial, rapido (quick), completo (full), experimental
simant --save MYGAME.ANT                load a save (original .ANT files work too)
simant --escala 2                       2x window (0 = largest that fits)
simant --resolucao 800x600              the original's 800x600 layout
simant --sem-som                        no audio
```

`erro: não encontrei ...\SHARED.ndx` ("not found") means the game folder path is wrong.

## 3. Playing

You are the **yellow ant** of a black colony, fighting the red ants in a suburban backyard. Click to
move, double-click to act (attack, ask for food, pick up/drop, dig); hold the button on the yellow ant
for its menu. **1/2** recruit 5/10 workers, **3/4** release half/all, **0** alarm, **X** switch body,
**Shift+0** pause, **Shift+1..4** speed, **Ctrl+S** save, **Ctrl+L** load, **F1** help. The original
cheat codes work (type `heal`, `just`, `qeen`, `eggs`... during a game). All keys are configurable in
`data/*.json`.

## 4. Mods

A mod is a folder in `mods/` with a `mod.json`; it can override port data (`data/*.json`, by key),
graphics and sounds from the original banks, and bring sandboxed Lua 5.4 scripts. Try the examples
with `simant --mods exemplos/mods` (`trapacas` adds an **F12** debug menu). Reference: `docs/api-mods.md`
(mod API 1.0) in the project repository.

## 5. License and credits

Project code: `LICENSE.txt`. Third-party libraries in the executable (SDL2 under the zlib license, Lua,
Rust crates): `AVISOS-TERCEIROS.txt`. SimAnt © 1991 Maxis. Source and issues:
https://github.com/Tim-campy/simant-recomp
