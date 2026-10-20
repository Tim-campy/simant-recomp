*Leia em português: [LEIAME-usuario.md](LEIAME-usuario.md) (in the release package: `LEIAME.txt`)*

# SimAnt recomp — README

Version 1.0.0 (2026-10-20). A modern reimplementation of **SimAnt: The Electronic Ant Colony** (Maxis, 1991,
Windows 3.x version) for current Windows and Linux. The game logic is rewritten from scratch; graphics,
sounds, music and help come from **your own copy** of the original SimAnt, which this package does not
contain.

> **Legal notice.** This is a non-profit fan project, **not affiliated with or endorsed by Maxis or
> Electronic Arts**. "SimAnt" and "Maxis" are trademarks of their respective owners. This package
> **does not distribute any file from the original game** (graphics, sounds, music, text, executable):
> to play, you need your own copy of SimAnt for Windows 3.x. Please do not ask for or share copies of
> the game in the project's channels.

## 1. Requirements

- **Windows 10 or 11, 64-bit:** nothing besides the package (`SDL2.dll` is included).
- **Linux x86-64** (glibc 2.35 or newer, such as Ubuntu 22.04, Debian 12, Fedora 36 onwards) with the
  system **SDL2**:
  - Debian/Ubuntu: `sudo apt install libsdl2-2.0-0`
  - Fedora: `sudo dnf install SDL2` (or `sdl2-compat`)
  - Arch: `sudo pacman -S sdl2-compat` (or `sdl2`)
- **The SimAnt for Windows 3.x files**, from your installation or the original floppy/CD. The port
  needs the folder that contains:

  | File | What for |
  |---|---|
  | `SHARED.NDX`/`.DAT`, `WINGANT.NDX`/`.DAT` | graphics, screens, texts (required) |
  | `SOUND.NDX`/`.DAT` and the `SOUND` folder (`*.MID`) | effects and music (without them the game is silent) |
  | `SIMANTW.EXE` | cursors of the help mode and of the tools (only read, never run) |
  | `SIMANT.HLP` | the help (F1) |
  | `FONTRES.FON`, `FONT1`…`FONT4` | the game's fonts |

  The other files in the folder (`MWINNT.*`, `SIMANT.CFG`...) are not used; it does no harm to leave
  them there.

  Optional: if the game folder sits inside a Windows 3.1 installation (`...\MAXIS\ANTWIN` with
  `...\WINDOWS\SYSTEM` nearby), the port uses the system fonts (`VGASYS.FON`, `SSERIFE.FON`) in titles
  and dialogs and reads the options from the `[SimAnt]` section of `WIN.INI`. Without them, it uses the
  game's font.

  Copy the files from an installation or from the disk (a folder such as `C:\MAXIS\ANTWIN`). The port
  only **reads** these files; it never changes them.

## 2. Installing

1. Unpack the archive into any folder (for example `C:\Games\simant-recomp` or `~/games/simant-recomp`).
2. Open `simant.exe` (Windows, double-click) or `simant` (Linux). The first time, the **startup
   options** screen appears (the launcher, section 3): choose the original game's folder there and
   click **Play**. The choice is remembered.

Other ways to point to the game folder (the first one that exists wins): the `--jogo "C:\MAXIS\ANTWIN"`
option; the folder chosen in the launcher; the `SIMANT_DIR` environment variable; or a
`SimAnt-T\MAXIS\ANTWIN` folder inside the package folder.

## 3. Running: the launcher

`simant` opened **without options** (double-click, shortcut, menu) shows the launcher. The launcher is
available in English and Portuguese; the Portuguese labels are given in parentheses below.

- **SimAnt installation folder** (*Pasta da instalação do SimAnt*): *Browse...* (*Procurar...*) opens
  the system folder picker (on Linux, the one from `zenity` or `kdialog`, if installed; without them,
  type the path). You can also drag the folder onto the window. The line below says whether the folder
  has the game files (`SHARED.NDX`...) and what is missing.
- **Start** (*Início*): the title screen with "Select a Game" (like the original), straight into a
  **new game** (Quick, Full, Experimental or Tutorial) or into a **saved game** (`.ANT`, with
  *Browse...*).
- **Video** (*Vídeo*): emulated resolution (640×480, 800×600, 1024×768), window scale (1x to 4x or the
  largest that fits), initial window size (`WxH`), the Windows 3.1 title bar and the driver profile
  (the Windows 3.1 colors in 16-color VGA, 256-color SVGA or DOSBox-X's S3). The "Win 3.1 frame"
  (*Moldura Win 3.1*) comes checked: the window loses the system border and behaves like a Windows 3.1
  window: drag it by the title, double-click the title (or use the right button) to maximize or
  restore, the gray frame resizes, minimize and the system menu work even with a dialog open, and
  Alt+F4 asks whether you want to save. Uncheck it to use the normal system window.
- **Sound** (*Som*): on or off, and the music synthesizer (simple — the default —, FM or SoundFont,
  with the `.sf2` file).
- **Mods**: the folder (`mods`, `exemplos/mods` or another one) and a checkbox to turn each of its mods
  on or off.
- **Player preferences** (*Preferências do jogador*): keep the options between games and the `WIN.INI`
  from which to read the original's options the first time (the automatic choice is the
  `WINDOWS\WIN.INI` near the game folder). "486 Fast/Ultra" (*Fast/Ultra do 486*) returns the fast and
  ultra speeds to the original's pace (much faster than the port's default, 2.5× and 5× normal);
  "Slow without focus" (*Lenta sem foco*) runs the simulation at slow speed while the window is not
  focused.
- **Equivalent command line** (*Linha de comando equivalente*): the `--` options matching the choices
  (the same as in section 5). You can edit it and press Enter: the choices above change accordingly,
  and options the screen doesn't have (such as `--semente 42`) are kept.
- **Play** (*Jogar*) saves the choices and starts the game; **Save settings** (*Salvar configuração*)
  only saves; **Quit** (*Sair*) closes. Unchecking **Open this screen at startup** (*Abrir esta tela ao
  iniciar*) makes the double-click go straight to the game with the saved choices; to see the launcher
  again, run `simant --launcher` (or delete the configuration file).

Keys in the launcher: Enter = Play (outside the fields), Esc = Quit, Tab moves between fields, Ctrl+V
pastes, Ctrl+C copies the field (or the command line).

The choices are stored in `launcher.json` in the user data folder: `%APPDATA%\SimAnt recomp\SimAnt\`
(Windows) or `~/.local/share/SimAnt recomp/SimAnt/` (Linux). The port finds `data/` and `mods/` next to
the executable, so it works from any folder, from a shortcut or from the terminal.

**Messages and errors.** On Windows `simant.exe` doesn't open a console: the messages (from mods,
warnings) go to `simant.log` in that same data folder (the previous run's log is kept as
`simant-anterior.log`), and an error that prevents the game from opening shows up in a message box. To
see the messages live, use `simant-console.bat` (or `simant.exe --console`). On Linux, when run from a
terminal, the messages appear there; when opened from the file manager, they go to the same
`simant.log`.

### From the terminal

With options, `simant` goes straight to the game; the choices saved in the launcher still apply
underneath (game folder, video, sound, mods), and the typed options win:

```
simant                                  launcher (or the game directly, if so configured)
simant --launcher                       always the launcher
simant --novo-jogo --tipo completo      straight into a new game: tutorial, rapido, completo, experimental
simant --save MYGAME.ANT                opens a save (the original's .ANT files too)
simant --escala 2                       window scaled 2x (0 = the largest that fits)
simant --resolucao 800x600              the original's 800x600 layout
simant --sem-som                        no audio
simant --sem-config --novo-jogo         ignores the choices saved in the launcher
```

(The game types are `rapido` = Quick Game, `completo` = Full Game, `experimental` and `tutorial`.)

If `erro: não encontrei ...\SHARED.ndx` ("not found") appears, the game folder is wrong: open the
launcher and choose another one.

## 4. How to play

You are the **yellow ant** of a black colony, against the red ants, in the yard of a house. The game
itself is in English, like the original (its texts come from your copy).

- **Mouse:** a click takes the yellow ant there; a **double-click** acts (attacks, asks for food, picks
  up or drops, digs); holding the button on the yellow ant opens its menu.
- **Yellow ant:** **1**/**2** recruit 5/10 workers, **3**/**4** release half/all, **0** alarm, **X**
  switches body, **Home** centers the view.
- **Speed and game:** **Shift+0** pauses, **Shift+1..4** speeds (without the window's focus, the
  simulation runs at slow speed and resumes by itself), **Ctrl+S** saves, **Ctrl+A** saves as,
  **Ctrl+L** loads, **Ctrl+N** new game.
- **Windows:** **Ctrl+E** edit window, **Ctrl+B** Behavior, **Ctrl+C** Caste, **Ctrl+H** History,
  **Ctrl+I** Info, **Ctrl+M** map; **Ctrl + arrows** scroll the view.
- **Help:** **F1** opens the game's help; **Shift+F1** or the *?* buttons explain whatever you click
  next.
- **The original's cheats:** type the four letters during a game (`heal`, `just`, `qeen`, `eggs`...).

All keys can be changed in `data/partida.json`, `data/navegacao.json` and `data/janelas.json`.

## 5. Options

| Option | Effect |
|---|---|
| `--jogo DIR` | folder of the game files (default: the launcher's, `SIMANT_DIR` or `SimAnt-T/MAXIS/ANTWIN`) |
| `--dados DIR`, `--mods DIR` | folders of the port's data (`data`) and of the mods (`mods`), next to the executable |
| `--desligar-mods A,B` | doesn't load these mods (names from `mod.json`) |
| `--novo-jogo`, `--tipo T`, `--semente N` | new game of type `tutorial`, `rapido`, `completo` or `experimental`, with a fixed seed |
| `--save FILE.ANT` | opens a save |
| `--resolucao WxH` | emulated Windows 3.1 screen: `640x480` (default), `800x600` or `1024x768` |
| `--escala N`, `--janela WxH` | integer scaling and initial window size |
| `--moldura`, `--sem-moldura` | Windows 3.1 title bar in place of the system frame (on by default; `--sem-moldura` turns it off; `--moldura-do-sistema`: inside the system frame) |
| `--velocidades proporcional\|original` | fast and ultra at 2.5× and 5× normal (default) or at the original's pace |
| `--sem-foco lenta\|original` | without focus, slow speed (default) or the usual pace, like the original |
| `--perfil-driver vga\|svga256\|s3_dosbox` | colors and alignments of the emulated Windows 3.1 video driver (default: `vga`) |
| `--sem-som`, `--com-som` | no audio (or with audio, overriding the launcher) |
| `--sintetizador simples\|fm\|soundfont`, `--soundfont FILE.sf2` | how the music is played (default: the simple one; `fm` in the Sound Blaster style); with `soundfont`, your own General MIDI SoundFont |
| `--preferencias FILE`, `--sem-preferencias`, `--win-ini FILE` | file of the options kept between games; `WIN.INI` read the first time |
| `--launcher` | opens the launcher even with other options or with it turned off |
| `--config FILE`, `--sem-config` | another launcher configuration file, or none |
| `--console` | (Windows) shows the messages in a console instead of `simant.log` |

The default video options are in `data/video.json`. The player preferences, the launcher configuration
and the log are in `%APPDATA%\SimAnt recomp\SimAnt\` (Windows) or `~/.local/share/SimAnt recomp/SimAnt/`
(Linux).

## 6. Mods

A mod is a folder inside `mods/` with a `mod.json`. It can replace port data (`data/*.json`, only the
keys that change), graphics and sounds from the original database, and bring Lua scripts (which run
isolated: no `io`, `os` or `require`, with time and memory limits). To try the examples:

```
simant --mods exemplos/mods
```

(or, in the launcher, *Mods → exemplos/mods*).

- `exemplo-lua`: food rain every 500 ticks;
- `trapacas`: **F12** menu with rain, mating flight, boy, cat, end of game and a debug panel.

To keep a mod always on, copy its folder into `mods/` (or choose `exemplos/mods` in the launcher). The
launcher's checkboxes turn each mod on and off without removing it from the folder. The full reference
(mod API 1.0) is in `docs/api-mods.md` in the project repository.

## 7. License and credits

- The SimAnt recomp code: see `LICENSE.txt`.
- Third-party libraries included in the executable (SDL2, Lua, Rust crates, the launcher font) and
  their licenses: `AVISOS-TERCEIROS.txt`. `SDL2.dll` (Windows) is the official SDL 2.30.9 binary,
  under the zlib license.
- SimAnt © 1991 Maxis. This project neither uses nor distributes the game's code or files.
- Source code, documentation and issue reports: https://github.com/Tim-campy/simant-recomp
- Version history: `CHANGELOG.txt` (English) and `CHANGELOG.pt-BR.txt` (Portuguese).
