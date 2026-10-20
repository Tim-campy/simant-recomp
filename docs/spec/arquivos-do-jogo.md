*Leia em português: [arquivos-do-jogo.md](../pt-BR/spec/arquivos-do-jogo.md)*

# File inventory (`SimAnt-T/`)

The folder provided by the user is a complete Windows 3.1 installation with the game installed, in the style of DOSBox packages. The files belong to the user and are not redistributed.

| Path | What it is |
|---|---|
| `MAXIS/ANTWIN/` | **installed game**: this is where the port reads from |
| `MAXIS/ANTWIN/SIMANTW.EXE` | 16-bit NE executable (see [executavel.md](executavel.md)) |
| `MAXIS/ANTWIN/SIMANTW.SYM` | MAPSYM symbol table: 2,171 function and variable names |
| `MAXIS/ANTWIN/SHARED.*`, `WINGANT.*`, `MWINNT.*`, `SOUND.*` | resource banks (see [formatos/banco-dat-ndx.md](formatos/banco-dat-ndx.md)) |
| `MAXIS/ANTWIN/SIMANT.CFG` | video and sound mode (see [formatos/simant-cfg.md](formatos/simant-cfg.md)) |
| `MAXIS/ANTWIN/SOUND/*.MID` | 29 tunes in standard MIDI (Windows 3.1 dual arrangement). The EXE builds the path with `"%ssound\%s.mid"`; see [formatos/som.md](formatos/som.md) |
| `MAXIS/ANTWIN/FONT1`–`FONT4`, `FONTRES.FON` | fonts. `FONTRES.FON` contains `Gaia8x8`; `FONT1`–`4` have their own format, read by `_font_ReadFont` [open] |
| `MAXIS/ANTWIN/SIMANT.HLP` | WinHelp help file |
| `MAXIS/ANTWIN/README.WRI` | readme in Windows Write |
| `CD/` | installation media. The files with `_` in their name (`_IMANTW.EXE` etc.) are compressed with **SZDD** (signature `SZDD`, the format of Microsoft's `COMPRESS.EXE`). They are copies of the installed content [hypothesis: identical once expanded, not verified] |
| `WINDOWS/`, `SB16/`, `drivers/`, `AUTOEXEC.BAT`, `CONFIG.SYS` | Windows 3.1, Sound Blaster 16 and S3 Trio drivers (800×600, 256 colors, according to `SYSTEM.INI`) |
| `SimAnt - The Electronic Ant Colony (1991).ba1` | eXoWin3x package launcher. It points to `games\SimTower\` and `games\!win3x\SimTower\dosbox.conf`, i.e. it does not work for this game as it stands. There is no SimAnt `dosbox.conf` in the folder |

No `.ANT` saved game came in the folder.
