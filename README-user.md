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
2. Run `simant.exe` (Windows, double-click) or `simant` (Linux). With no options it opens the
   **startup options screen** (the launcher, in Portuguese):
   - *Pasta da instalação do SimAnt* (game folder): *Procurar...* opens the system folder picker (on
     Linux, `zenity` or `kdialog` if installed; otherwise type the path), or drag the folder onto the
     window. The line below says whether the folder has the game files and what is missing.
   - *Início* (start): the title screen and "Select a Game", a new game (*Rápido* = Quick,
     *Completo* = Full, *Experimental*, *Tutorial*) or a saved game (`.ANT`).
   - *Vídeo*: emulated resolution, window scale and size, the Windows 3.1 title bar, video driver
     profile (with *Moldura Win 3.1*, the window loses the system frame and behaves like a Windows 3.1
     window: drag by the title, double-click it or the right button to maximize/restore, resize by the
     gray frame, Alt+F4 asks to save). *Som*: sound on/off and the music synthesizer (simple — the
     default —, FM, SoundFont `.sf2`). *Mods*: the
     mods folder and a checkbox per mod. *Preferências*: keep options between games, `WIN.INI` to read
     the original's options from.
   - *Linha de comando equivalente*: the matching `--` options (section below), editable (press Enter to
     apply).
   - *Jogar* (play) saves the choices and starts the game; *Salvar configuração* only saves; *Sair*
     quits. Unchecking *Abrir esta tela ao iniciar* makes the double-click go straight to the game;
     `simant --launcher` brings the screen back.

The choices are stored in `launcher.json` in the user data folder (`%APPDATA%\SimAnt recomp\SimAnt\` on
Windows, `~/.local/share/SimAnt recomp/SimAnt/` on Linux). The port finds `data/` and `mods/` next to
the executable, so it works from any folder. Other ways to set the game folder (first one found wins):
`--jogo "C:\MAXIS\ANTWIN"`, the launcher choice, the `SIMANT_DIR` environment variable, or a
`SimAnt-T/MAXIS/ANTWIN` folder inside the package.

On Windows `simant.exe` opens no console: messages (mods, warnings) go to `simant.log` in the user data
folder, and an error that stops the game shows a message box. `simant-console.bat` (or
`simant.exe --console`) shows them in a console instead.

With options, `simant` goes straight to the game; the launcher choices still apply underneath and the
typed options win (`--sem-config` ignores them):

```
simant --launcher                       always the launcher
simant --novo-jogo --tipo completo      new game: tutorial, rapido (quick), completo (full), experimental
simant --save MYGAME.ANT                load a save (original .ANT files work too)
simant --escala 2                       2x window (0 = largest that fits)
simant --resolucao 800x600              the original's 800x600 layout
simant --sem-som                        no audio
simant --desligar-mods trapacas         do not load that mod
```

`erro: não encontrei ...\SHARED.ndx` ("not found") means the game folder is wrong: open the launcher
and pick another one.

## 3. Playing

You are the **yellow ant** of a black colony, fighting the red ants in a suburban backyard. Click to
move, double-click to act (attack, ask for food, pick up/drop, dig); hold the button on the yellow ant
for its menu. **1/2** recruit 5/10 workers, **3/4** release half/all, **0** alarm, **X** switch body,
**Shift+0** pause, **Shift+1..4** speed (Fast and Ultra run at 2.5× and 5× Normal; while the window is
not focused the game runs at Slow and resumes on focus — `--velocidades original` and
`--sem-foco original`, or the *Fast/Ultra do 486* and *Lenta sem foco* checkboxes, restore the
original pace), **Ctrl+S** save, **Ctrl+L** load, **F1** help. The original
cheat codes work (type `heal`, `just`, `qeen`, `eggs`... during a game). All keys are configurable in
`data/*.json`.

## 4. Mods

A mod is a folder in `mods/` with a `mod.json`; it can override port data (`data/*.json`, by key),
graphics and sounds from the original banks, and bring sandboxed Lua 5.4 scripts. Try the examples
with `simant --mods exemplos/mods` or by choosing `exemplos/mods` in the launcher (`trapacas` adds an
**F12** debug menu); the launcher checkboxes turn each mod on or off. Reference: `docs/api-mods.md`
(mod API 1.0) in the project repository.

## 5. License and credits

Project code: `LICENSE.txt`. Third-party libraries in the executable (SDL2 under the zlib license, Lua,
Rust crates, the public-domain launcher font): `AVISOS-TERCEIROS.txt`. SimAnt © 1991 Maxis. Source and issues:
https://github.com/Tim-campy/simant-recomp
