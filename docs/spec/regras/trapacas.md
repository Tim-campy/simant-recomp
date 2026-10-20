*Leia em português: [trapacas.md](../../pt-BR/spec/regras/trapacas.md)*

# Cheat codes of the original (`_CheatKeys`)

SimAnt for Windows has built-in cheats: **4-letter codes typed during the game**. There is no hidden menu nor special key combination; there is no on-screen notice other than the sound (or the text frame) of each code. This document describes the input (`_DoKeyDown`, `1000:0e8c`), the table (`_Cheats`, `8:83bc`) and the effects (`_CheatKeys`, `1018:67de`).

Sources: Ghidra pseudocode (`re_trabalho/decomp/Code1.c`, `_DoKeyDown`; `Code4.c`, `_CheatKeys`), disassembly of `1018:691c`–`69a0` (cases 9 and 10) and the QEMU test bench (script `tests/referencias/roteiros/qemu_trapacas.txt`, 2026-10-18).

## 1. Input: the 4-letter ring

`_DoKeyDown(hwnd, vk, ...)` handles `WM_KEYDOWN` of the main window. In order [code]:

1. Space, Insert and Delete become clicks (formiga-amarela.md §11.1).
2. `_HelpKeyDown` and `_YellowCommandKey` (formiga-amarela.md §11.4): if one of them handled the key, done. **A letter that is a yellow ant command (X; J with Tab) does not enter the ring.**
3. Arrows: move the cursor 8 pixels; done.
4. Enter (`vk = 0x0D`): resets the ring position and count (`DAT_1048_01da`, `DAT_1048_01dc`); done.
5. Only letters A–Z (`vk` 0x41–0x5A) continue. The character is **lowercase without Shift and uppercase with Shift** (`GetAsyncKeyState(VK_SHIFT)`; the C character-type table decides whether to add or subtract 0x20). Caps Lock does not count [code; **verified** on the test bench: `heal` and `HEAL` typed with and without Shift produce different effects].
6. The character goes into the 4-position ring (`_edata[pos]`, `pos = (pos + 1) mod 4`); the count goes up to 4.
7. With 4 letters in the ring, it searches the `_Cheats` table for a code equal to the last 4 letters, from oldest to newest. The `_Cheats != 0` test before the search reads the first byte of the table itself (it is not an on/off switch): the codes **are always active**.
8. Code *k* found: `_CheatKeys(k)`. The ring is not cleared: typing `heal` and then `l` does not repeat (the ring becomes `eall`), but `healheal` applies twice [code].

The table has 26 4-byte codes stored with inverted bits (`~c`), terminated by a 0. Read from the original's memory on the test bench (`ler_variavel 8:83bc`) and from `SIMANTW.EXE` [**verified**]:

`oops rand erad FUND joke hole HOLE qeen QEEN eggs EGGS heal HEAL zhea ZHEA just will mick jeff joel fred jenn susi data serv clie`

The index *k* is the position in the table (0 = `oops`).

## 2. Effects (`_CheatKeys(k)`)

First of all, `_WinPrintf("CHEAT %d", k)` (debug output, no effect on the game). Sounds via `_myBeginSound(id, 0, 0x7E)`. `(x, y)` of the yellow ant = `_MeLocX`, `_MeLocY`.

| k | Code | Effect [code] | Test bench |
|---|---|---|---|
| 0 | `oops` | sound 10; `_QueenStorageR += 10` | **verified** (0 → 10) |
| 1 | `rand` | sound 1; `_SetMyHealth(100)` | |
| 2 | `erad` | sound 1; `_SetMyHealth(1)` | |
| 3 | `FUND` | text frame 10020 ("You are now $10,000 richer!! Unfortunately, money is worthless in this game.") | |
| 4 | `joke` | frame 10022, `_UpdateEverything`, wait cursor, `_myBeginSoundList(0)` (empty list on Windows, as in the Mystery), frame 10024 | |
| 5 | `hole` | `_MakeNewHoleB(c)` for c = 0..63 (64 new black holes) | |
| 6 | `HOLE` | `_MakeNewHoleR(c)` for c = 0..63 | |
| 7 | `qeen` | `_MakeBlkQueen(x + 2, y, 2)` (new black queen in the black nest) | **verified** (`_BlkQueens` 1 → 2) |
| 8 | `QEEN` | `_MakeRedQueen(x + 2, y, 2)` | |
| 9 | `eggs` | 16 times: `_PlaceEggB(x + dx, y + dy, RRand(6) + 1)` with `dy = D8y[SRand8()]` drawn **before** `dx = D8x[SRand8()]` (see §2.1) | |
| 10 | `EGGS` | same, `_PlaceEggR` and type `RRand(6) + 0x81` | |
| 11 | `heal` | `_HealthB = 100` | **verified** (40 → 100; read 99 one tick later) |
| 12 | `HEAL` | `_HealthR = 100` | **verified** (40 → 100) |
| 13 | `zhea` | `_HealthR = 0` | **verified** (40 → 0) |
| 14 | `ZHEA` | `_AlwaysHealthy = 0`; `_HealthB = 0` | |
| 15 | `just` | `_QueenStorageB += 10`; `_InvalQueenStorageDisp`; sound 0x29 | **verified** (0 → 10) |
| 16 | `will` | toggles `_Invincible`; sound 2 if turned on, 1 if turned off | |
| 17 | `mick` | toggles `_RevSpider`; sound 2 / 1 | |
| 18 | `jeff` | sound 2; for each of the 192 patches, `_YMapPopB[i] += 4`; `_BColoniesStarted += 16` per column (+192 in total) | |
| 19 | `joel` | sound 1; for each patch, `SRand2()`: 0 → `_YMapPopB[i] += 1` and `_BColoniesStarted += 1`; otherwise `_YMapPopR[i] += 1` and `_RColoniesStarted += 1` | |
| 20 | `fred` | `_DrawSimPayoff` (victory animation over the yard, with songs 20003 and 20004; drawing only) | |
| 21 | `jenn` | toggles `_NeverHungry`; turned on: `_MeHealth = 100` and sound 2; turned off: sound 1 | |
| 22 | `susi` | toggles `_AlwaysHealthy`; turned on: `_HealthB = 100` and sound 2; turned off: sound 1 | **verified** (0 → 1) |
| 23 | `data` | DDE with a "database" (`_GtInitiateDDE`); failure: box "Unable to access database." | |
| 24 | `serv` | NetBIOS, server ("SERVANT"); failure: box "Unable to access network." | |
| 25 | `clie` | NetBIOS, client ("CLIEANT"); failure: same | |

Also **verified** on the test bench: `Heal` (Shift only on the H) does nothing, and `he` Enter `al` does nothing (Enter clears the ring).

The added values are not capped (`_QueenStorage*` goes from 10 to 20 etc.); the `_YMapPop*` are bytes and wrap around after 255 [code].

### 2.1 The eggs (`eggs`/`EGGS`)

Disassembly of `1018:691c`: the type `RRand(6) + 1` (or `+ 0x81`) is pushed first (third argument); then `_SRand8()` indexes the 16-byte table at `[0xc28a]:8` (the y) and another `_SRand8()` indexes `[0xc28c]:0` (the x). Both are offsets from −1 to 1 [hypothesis: the table is the 16-byte `D8` with dx in the first 8 and dy in the last 8, like `dx8`/`dy8` in `amarela.json`]. Since the two directions are drawn separately, the egg lands in any of the 9 surrounding cells (including the yellow ant's own). `_PlaceEggB/R` ignores coordinates outside the nest (ninho.md §7.6): in the yard the code has no useful effect. Draws: R 16 times, S 32 times.

### 2.2 `_GlobalKey` is not a Windows cheat

`_DoSimYard` starts the rain if `_GlobalKey` (`9:7866`) = 0xA8, and `_SimCat` tests 0x8D (quintal.md). No routine in the pseudocode writes `_GlobalKey`, and the test bench always read it as 0 [code; likely a leftover from the Mac version, hypothesis]. There is no way to turn on the rain from the keyboard in the original.

## 3. In the port

- Rules and ring: `engine/src/rules/trapacas.rs`. Table and name of each effect: `data/codigos_trapaca.json` (moddable: a mod can change the codes or remove a code from the list).
- The key arrives via `Partida::tecla_de_trapaca` (`engine/src/core/partida.rs`) after `_YellowCommandKey`, as in the original; Enter clears the ring.
- `fred` requests `Evento::AnimacaoVitoria` (2026-10-18): the interface shows `_DrawSimPayoff` (telas/mensagens.md §11.3; checked on the test bench, scenario `payoff`).
- Effects with no equivalent in the port: `data`, `serv` and `clie` (DDE and NetBIOS). The port logs the code to the console and changes nothing [recorded divergence]. `joke` shows the two frames like the Mystery (not forced: only with Options → Events).
- Frame 10020 of `FUND` is not forced (third parameter 0 of `_PictStrnDialog`): it was added to `nao_forcadas` in `data/mensagens.json`.
