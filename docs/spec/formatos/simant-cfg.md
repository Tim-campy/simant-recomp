*Leia em português: [simant-cfg.md](../../pt-BR/spec/formatos/simant-cfg.md)*

# `SIMANT.CFG`

Text file in the game folder. The analyzed copy contains:

```
Display Mode: ?
Sound Mode: 6
```

## Reading

**Source:** fact seen in the code, `_ReadConfig` (`1008:08f4`).

- Reads two space-separated words ("Display", "Mode:") and then **a single character**, which is the video mode.
- Reads two more words ("Sound", "Mode:") and a character. If it is a digit, `_musicDevice = digit`.
- If the file does not exist, shows "SimAnt configuration file missing" and exits.
- An unknown video character shows "Bad 'Display Mode' in configuration file" and exits.

## Video mode → `_displayType` → graphics database

| Character | `_displayType` | Database prefix | Resulting database (with `nt` suffix) | In the analyzed copy |
|---|---|---|---|---|
| `E` | 0 | `hcega` | `HCEGANT` | absent |
| `T` | 2 | `tdyga` | `TDYGANT` | absent |
| `H` | 3 | `mono` | `MONONT` | absent |
| `e` | 4 | `lcega` | `LCEGANT` | absent |
| `M` | 5 | `mono` | `MONONT` | absent |
| `m` | 7 | `mono` | `MONONT` | absent |
| `V` | 8 | `hcega` | `HCEGANT` | absent |
| `w` | 9 | `mwin` | `MWINNT` | present |
| `W` | 10 | `winga` | `WINGANT` | present |
| `?` | automatic | — | 9 if the video has 1 plane and 1 bit per pixel (`GetDeviceCaps` PLANES = BITSPIXEL = 1); otherwise 10 | — |

The prefixes come from the far pointer table at `1048:b92b` used by `_InitGraphics` (`1008:010c`). The `nt` suffix is a [hypothesis] supported by the names of the existing files. The modes without a matching file look like a legacy of the DOS versions. The port only needs to support `W` (color) and, optionally, `w` (mono).

## `Sound Mode`

The value goes into `_musicDevice`, but no function reads this variable in the pseudocode: audio output is chosen by `_snd_Install`, which tests `MMSYSTEM.DLL`, `SNDBLST.DLL` and `DSOUND.DLL` (see [som.md](som.md)). On Windows, "Sound Mode" has no effect [seen in the code; legacy of the DOS versions]. The Music and Sound Effects options come from `WIN.INI` (section `[SimAnt]`), see [regras/som.md](../regras/som.md).

## Port

The port does not need this file. The choice of graphics database and audio output goes into the port's own configuration. The original `.CFG` may be read only as a default value.
