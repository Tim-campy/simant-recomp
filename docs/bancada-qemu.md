*Leia em português: [pt-BR/bancada-qemu.md](pt-BR/bancada-qemu.md)*

# Automated test bench (QEMU + SvarDOS)

Runs the original SimAnt with no window and without using the user's mouse/keyboard, controlled only through QEMU's QMP protocol (and through the gdbstub to write to memory). It is used to generate references (screenshots, saved games, memory states) and for rule tests against the original.

Decision and alternatives evaluated: `docs/decisoes.md` and `reports/Automação de testes Win16 emulado.md`.

## Components (all outside git, in `re_trabalho/` and `referencia/`)

| Piece | Source | Note |
|---|---|---|
| QEMU 2026-08-11 (`referencia/qemu/`) | qemu.weilnetz.de, official Windows installer (SHA-512 checked) | `qemu-system-i386`, TCG acceleration; `qemu-img` for the overlay |
| SvarDOS 20250427 (EDR-DOS kernel + SvarCOM + HIMEMX) | svardos.org, 1.44 MB floppy | Base of the minimal boot floppy assembled by `tools/qemu_bancada.py` |
| Disk C: (`re_trabalho/qemu/c.img`, FAT16, 100 MB) | copy of `SimAnt-T/WINDOWS` and `SimAnt-T/MAXIS` | created with DOSBox-X's `IMGMAKE`/`COPY` (QEMU's `vvfat` fails when Windows writes) |
| Overlay (`re_trabalho/qemu/c_overlay.qcow2`) | `qemu-img create -f qcow2 -b c.img -F raw` | created by the script when there are snapshots; holds the snapshots and whatever the game writes |

Tweaks to the **copy** of Windows for Workgroups 3.11 (the user's installation is not touched):

- `SYSTEM.INI`: `display.drv=vga.drv` and `display=*vddvga` (the S3 Trio does not exist in QEMU); `;device=vshare.386` (VSHARE hangs WfW on top of the EDR kernel); `InDOSPolling=TRUE`.
- `WIN.INI`: `MouseSpeed=0`, `MouseThreshold1=0`, `MouseThreshold2=0` (no acceleration: 1 movement unit = 1 pixel).

Result: WfW 3.11 runs in **386 enhanced mode**, with a **640×480, 16-color** screen. That is why the game uses the window position table for screens of up to 480 lines (object 9/0). The DOSBox-X (800×600) and QEMU (640×480) references are not interchangeable.

## Usage

```
python tools/qemu_bancada.py preparar                 # recreates the floppy and disk C: (≈1 min); deletes the default overlay
python tools/qemu_bancada.py roteiro tests/referencias/roteiros/qemu_salvar_quick.txt
python tools/fat16_extrair.py re_trabalho/qemu/c.img MAXIS/ANTWIN/QUICK2.ANT tests/referencias/saves/QUICK2.ANT

# with snapshots (disk in a qcow2 overlay):
python tools/qemu_bancada.py roteiro tests/referencias/roteiros/qemu_snapshot_quick.txt   # boot + savevm "quick" (≈1 min, once)
python tools/qemu_bancada.py roteiro tests/referencias/roteiros/qemu_estado_save.txt      # starts from "quick" without booting
python tools/qemu_bancada.py extrair MAXIS/ANTWIN/QUICK3.ANT re_trabalho/QUICK3.ANT --overlay re_trabalho/qemu/c_overlay.qcow2
python tools/estado_diff.py re_trabalho/QUICK3.ANT re_trabalho/estado_quick3_salvo.bin
```

`roteiro` options: `--overlay ARQ.qcow2` (opens disk C: through that overlay; path relative to the root) and `--mem MB` (VM RAM, default 16). The `SIMANT_RAIZ` environment variable points to the root containing `referencia/` and `re_trabalho/` (to run the script from a git worktree); the commands' output paths are relative to it. The QMP and gdbstub ports are picked free on each run, so two benches can run at the same time, as long as they do not write to the same disk.

`tools/medir_abertura.py OVERLAY [--espaco]` starts the VM, classifies the screen every ~0.2 s (title, registration, other) and reads the game's wait stack (`_DialogWaitInit`, `DGROUP:150a`/`cad4`/`caf4`) and the BIOS tick counter (`0040:006C`): it measures the waits in VM ticks, which do not depend on the host load (telas/abertura.md). Screenshots of screens with the game paused: scenarios `navegacao`, `examine_sessao`, `menu_amarela`, `historico` and `triangulo` of `tools/cenarios_tick.py` (tests/referencias/README.md).

`tools/estado_diff.py A B` compares two files in the `.ANT` format (save, `estado` or port dump) block by block, with the layout names and the values of blocks of up to 4 bytes.

### Script commands

| Command | What it does |
|---|---|
| `esperar SEG` | waits SEG seconds of the host clock |
| `teclas T1 T2 ...` | `send-key` with QMP qcodes (`spc`, `ret`, `esc`, `alt+f`, ...) |
| `digitar TEXTO` | types letters, digits and space |
| `mouse_canto`, `mover DX DY`, `clicar`, `clicar_em X Y`, `clicar_duplo_em X Y` | relative mouse (`input-send-event`); with acceleration off, 1 unit = 1 pixel; the double click releases both clicks within ~0.25 s (with `clicar_em` followed by `clicar`, Windows did not see a double click in the yard) |
| `capturar ARQ.png` | `screendump` |
| `memoria INICIO TAMANHO ARQ` | `pmemsave` (**physical** RAM) |
| `pausar` / `continuar` | QMP `stop` / `cont`: freezes the CPU, the VM clock and the simulation. In tick mode, `continuar` goes back to running free (the breakpoints are already off) |
| `snapshot_salvar NOME` / `snapshot_carregar NOME` | `savevm` / `loadvm` (HMP) on the qcow2 overlay |
| `ler_variavel NOME ARQ [TAM]` | writes the variable's bytes to ARQ and shows the value if it has up to 4 bytes |
| `escrever_variavel NOME VALOR [TAM]` | writes VALOR (little-endian integer in the variable's size, or `hex:AABB...`) through the gdbstub and checks by reading it back |
| `estado ARQ` | all 307 blocks of `data/save_layout.json`, in save order: the file has the exact format of an `.ANT` (48,386 bytes) |
| `disquete ARQ.img` | swaps the media in drive A: (QMP `blockdev-change-medium`, read-only); used to inject saves (see [Test cases](#test-cases-inject-a-save-and-run)) |
| `esperar_variavel NOME ALVO [TIMEOUT]` | polls NOME (unsigned) every 0.05 s until it reaches ALVO; fails after TIMEOUT s (default 120). Followed by `pausar`, it stops at tick ALVO or 1 later |
| `extras ARQ.json` | variables outside the save captured with the tick vectors (S and R seeds, `_gameCycles`, `_Tindex`, `_GlobalKey`), `GameTime`, `Cycle` and the position (`entrada`/`saida`/`livre`). The Mac clock (`_MacTickCount`, a function) is only captured by `passo_tick`/`gerar_vetores_tick.py`, with breakpoints inside the tick |
| `segmentos PREFIXO` | data segments 8, 9 and 10 in full (up to the descriptor limit) in `PREFIXO_seg8.bin` etc. |
| `tick_entrada` | tick mode: runs until the entry of `_DoAntSim` (state **before** a tick); does not advance if already there |
| `tick_chamada [TIMEOUT]` | runs until the `CALL far _DoAntSim` at `1000:2683` (segment 1, always in memory): state before the tick even when segment 6 has not been loaded yet, as right after a new game (spec/regras/geracao-mundo.md §8) |
| `tick_saida` | from the entry to the return of `_DoAntSim`: exactly one tick (state **after**) |
| `ate_gametime N` | stops at the entry of `_DoAntSim` with `GameTime` (not yet incremented) ≥ N, tick by tick |
| `passo_tick DIR` | one tick: `DIR/estado_antes.bin`, `estado_depois.bin`, `vida_antes.bin`, `vida_depois.bin`, `extras.json` |
| `fim` | clean `quit` |

`NOME` can be: a save layout symbol (`GameTime`, `MapA`, ...; size = count × size); any variable of segments 8, 9 and 10 in `re_trabalho/symbols.tsv` (with or without `_`; requires `TAM`); a `SEG:OFF` address in hexadecimal (`10:cbf2`; requires `TAM`); or a script alias: `SementeS` = `10:cbf2` (2 bytes, LFSR state) and `SementeR` = `10:ae34` (4 bytes, state of the C `rand`), see [spec/regras/rng.md](spec/regras/rng.md).

### Snapshots

`savevm` requires every writable disk to support snapshots, and `c.img` is raw. Therefore, when the script uses `snapshot_*` (or when `--overlay` is passed), the script creates, if it does not exist, `re_trabalho/qemu/c_overlay.qcow2` (`qemu-img create -f qcow2 -b c.img -F raw`) and opens the disk through it. The floppy is always opened read-only (`readonly=on`). The internal snapshot holds CPU, RAM, devices and the disk state **inside the overlay**.

- `c.img` does not change in this mode: the files the game writes stay in the overlay. To get them out: `qemu_bancada.py extrair CAMINHO DESTINO --overlay ARQ` (converts the overlay to raw in a temporary directory and uses `fat16_extrair`).
- The overlay is only valid on top of the `c.img` it was born from: `preparar` deletes the default overlay. If `c.img` is changed by a script without an overlay, delete the overlay too.
- `loadvm` requires the same machine: a snapshot saved with `--mem 16` does not work with `--mem 32`.
- Measured: `savevm` and `loadvm` of 16 MB take less than 1 s. The script that starts from the `quick` snapshot is in the Quick Game at instant 0, versus ~55 s of booting.

## Reading and writing the game's memory

### The problem

WfW 3.11 runs in 386 enhanced mode: there is paging, and each segment of the NE executable is an **LDT selector** whose base is a **linear** address. In physical RAM (`pmemsave`) the segments are not contiguous, and `_MapA` (8 KB) spans pages: in a physical dump made right after the save, the 8 KB of the saved `_MapA` do not appear whole anywhere.

### The technique adopted (`Memoria` in `tools/qemu_bancada.py`)

1. **Selectors from the save table.** `_SaveGame` uses the table at `DS(8):0x9570`, with entries `{u16 count, u16 size, u16 offset, u16 selector}` ([spec/formatos/save-ant.md](spec/formatos/save-ant.md)). The Windows NE loader fixes up the far pointers with the **real selectors** of segments 8, 9 and 10. The script does a `pmemsave` of the whole RAM and searches for the signature of the first 6 entries (count, size and offset known; selector as wildcard). That is 48 bytes, which almost always sit in a single page and appear contiguous in physical RAM. The selectors come from there.
2. **Linear base from the LDT.** `info registers` (HMP) gives the LDT's linear base and limit (`LDT=...`). The selector's descriptor (8 bytes at `base_LDT + (selector & ~7)`, read with `memsave`) gives the segment's linear base and limit.
3. **Validation.** Through the linear base of segment 8, it reads the whole table (307 entries) at `0x9570` and checks count, size, offset and the consistency of the selectors. That is where the selector of segment 9 shows up, since it is not in the first 6 entries. Only then are the selectors accepted.
4. **Reading:** QMP `memsave` with a linear address (`base + offset`), translated by the CPU's current paging. `estado` reads each segment at once (3 `memsave`), with the VM stopped during the read, and assembles the blocks in layout order.
5. **Writing:** QEMU's gdbstub (`-gdb tcp:127.0.0.1:PORTA`, always on) and a minimal RSP client in Python: packet `M address,length:hex` (linear address) and read-back with `m`. Opening the connection stops the VM. The script closes the socket **without** `D` (the detach would resume the VM right away) and restores through QMP the previous state (running or paused). This way, `pausar` + `escrever_variavel` + `estado` does not let the simulation advance between the write and the read.
6. The base of each segment is reread on every access (Win16 data segments can be movable). The selectors stay cached until a `snapshot_carregar`.

Selectors observed (2026-10-16, `-m 16`, at boot and after `loadvm`): segment 8 = `0x1537`, 9 = `0x152f`, 10 (DGROUP) = `0x1527`; table at physical address `0x246c70`. They are only valid for that configuration: the script always searches again.

Alternatives discarded: searching for the contents of `_MapA` in physical RAM (it spans pages and changes every tick); hardcoding selectors (they depend on load order and memory); reading the process's `DS` from the registers (at the moment of the pause the CPU is rarely in the game's code). HMP has no memory write command, hence the gdbstub.

## What was verified (2026-10-16)

| Item | Result |
|---|---|
| Boot to the game | full title logo ~32 s after starting (see below); Quick Game ready in ~55 s |
| Keyboard (`send-key`) | Space closes the title and the registration; Alt+F opens the File menu; Ctrl+A opens "Save As"; typing the name works |
| Mouse (relative `input-send-event`) | after `mouse_canto`, a movement of (100, 100) puts the arrow's tip exactly at (100, 100) |
| Click | `clicar_em 345 228` picks "Quick Game" in "Select a Game" |
| Screenshot (`screendump`) | 640×480 PNG, with no visible window |
| Memory (`pmemsave`) | 16 MB dump of physical RAM |
| Saved game | `QUICK2.ANT` with **48,386 bytes**, exactly as predicted in `spec/formatos/save-ant.md` |
| Snapshot | `qemu_snapshot_quick.txt` saves `quick`; `qemu_estado_save.txt` and `qemu_semente.txt` start from it |
| **`estado` = save, byte by byte** | `qemu_estado_save.txt`: Ctrl+A, `QUICK3`, Enter; with the "Saved correctly" box on screen, `pausar` and `estado`. The `QUICK3.ANT` extracted from the overlay and the `estado` are **identical** in all 48,386 bytes (`estado_diff.py`: "iguais"). The save is at `tests/referencias/saves/QUICK3.ANT` |
| Real-time reading | ~10.4 s (host) later, another `estado` differs in 70 of the 307 blocks: `GameTime` 92 → 154 (`Cycle` and `Scycle2` follow), `YardCycle` 91 → 153, `MapA` (13 bytes), lists `Alist*/Blist*/Rlist*`, `BoyX` 188 → 256, `DogX/Y`, `CatOn` 1 → 0, `SpidY` 205 → −1, `HealthB` 95 → 92, `TotalEggsLaidB` 4 → 6, `HistCnt` 1 → 2, among others |
| Writing | `qemu_semente.txt`: `SementeS` read 0x8a5e; with the VM paused, wrote 0x1234, read back 0x1234 and still 0x1234 two seconds later; ~1 s after `continuar`, it is 0x6acb, which is exactly the LFSR state of [spec/regras/rng.md](spec/regras/rng.md) 230 steps after 0x1234 (checked in Python). `HealthB` written as 7 with the VM running: read back 7, and 7 three seconds later |
| Injection and load | `qemu_carregar_save.txt`: floppy swapped after `loadvm`, `A:TESTE` loaded through File > Load; the state read right after the load equals the saved `QUICK2.ANT` except for three blocks (`_GamePaused` and the recounted `_CastePop*`) |
| Test cases | `caso_ant.py rodar` with C-01 and C-02 (`tests/referencias/casos/`): all checks passed, stopping at exactly ticks 128 and 96 (see `tests/referencias/README.md`) |

Pace (host clock, no determinism): `GameTime` advanced 62 ticks in ~10.4 s, about 6 ticks/s. **Explained (2026-10-18):** it is not TCG slowness; it is the `MYTIMERFUNC` loop with Windows 3.1's 55 ms timer, which runs 1 tick every 3 messages at normal speed. Measurement per speed (scripts `qemu_ritmo.txt` and `qemu_ritmo_rapido.txt`): [spec/regras/simulacao-visao-geral.md](spec/regras/simulacao-visao-geral.md) §3.1.

## Test cases: inject a save and run

Tools:

- `tools/ant_editar.py`: reads and writes `.ANT` through the layout (blocks, scalars, map cells, ant lists with `ListIndex`, patch in JSON). Tests: `python -m unittest discover -s tools -p "test_*.py"`.
- `tools/caso_ant.py rodar tests/referencias/casos/ID.json`: builds the case's `.ANT`, injects it, loads it in the original, runs until the target `GameTime`, pauses, saves the state and checks. Output in `re_trabalho/casos/ID/` (`entrada.ANT`, `carregado.ANT` = state right after the load, `estado.ANT` = final state, screens `t1`–`t4`, `roteiro.txt`, `resultado.json`). Exit code 0 if all checks pass.
- Format of the cases and of the checks: docstrings of both scripts; list of cases in `tests/referencias/README.md`.

### Injection: swapping the A: floppy

The save goes on a **FAT12 floppy** (a copy of the bench's `boot.img` with the `.ANT` added by `tools/fat12.py`; `caso_ant.py disquete SAIDA.img ARQ.ANT=NOME.ANT` builds one by hand). After `snapshot_carregar quick`, the `disquete` command swaps the media in drive A: and the game opens `A:NOME` through the Load dialog.

Why this one and not the other options:

| Option | Problem |
|---|---|
| Writing the file to C: (FAT16 in Python on the raw converted from the overlay, or DOSBox-X's `IMGMOUNT`+`COPY`) | `loadvm` brings the overlay disk back to the snapshot state, erasing the file; and the restored DOS/Windows has the FAT and directory cached (`BUFFERS`), inconsistent with a disk changed from outside. It would only work with a full boot (~55 s per case) and a qcow2 → raw → qcow2 conversion for every case |
| Second drive (`-drive if=floppy,index=1` = B:) | changes the machine: the `quick` snapshot was saved without B:, and the DOS in the snapshot's RAM detected a single drive at boot [hypothesis, not tested: with a single drive, DOS treats B: as a phantom of A: and asks for a floppy swap]; it would require a full boot |
| **Swapping the A: media** | changes neither the machine nor C:; the snapshot remains valid; DOS notices the swap through the controller's disk change line. It worked on the first try |

The new floppy keeps the boot floppy's files (kernel, `COMMAND.COM`), in case DOS needs to reload the command interpreter. Each run of `caso_ant.py` copies the default overlay (with the `quick` snapshot) to `re_trabalho/casos/ID/ov.qcow2`: the default overlay and the snapshot are not touched.

### Loading an injected save (steps observed on 2026-10-16)

Reference script: `tests/referencias/roteiros/qemu_carregar_save.txt`. Starting from the `quick` snapshot (Quick Game running):

1. `teclas ctrl+l` (File > Load). The box "SimAnt Load: Do you want to save the current ant hill?" (Yes / No / Cancel) appears.
2. `teclas n` (No). The common dialog "Load Ant Hill" opens: File Name field with the focus, directory `c:\maxis\antwin`, list `*.ANT`, drive `c:`.
3. Type `A:NOME` (`digitar A`, `teclas shift+semicolon`, `digitar NOME`; `digitar` only has letters, digits and space). Without an extension, the dialog completes `.ANT`. On the first attempt, without step 2, the letters of the name were taken by the box from step 1 (the "N" answered No) and the dialog received only part of the name ("t.ant": "Cannot find this file").
4. `teclas ret`. In ~5 s the game is loaded: window title = file name (`TESTE.ANT`), and the game is **paused** (message bar "Game Paused", `_GamePaused` = 1, even with 0 in the file).
5. `teclas shift+0` (Speed > Unpause; the Speed menu shows Unpause Shift+0, Slow Shift+1, Normal Shift+2, Fast Shift+3, Ultra Shift+4).
6. `esperar_variavel GameTime ALVO`, `pausar`, `estado ARQ`.

What the load changes (QUICK2.ANT loaded and read right after, still paused; `estado_diff.py`): only 3 blocks. `_GamePaused` 0 → 1, and `_CastePopB`/`_CastePopR` are **recounted** from the lists (`[2,2,0,0,0,1]` → `[3,2,0,0,0,1]` and `[3,1,…]` → `[4,1,…]`: the eggs laid after the last count). Consequence for the cases: `_BpopT`/`_RpopT` and `_CastePop*` cannot be forced through the file; change the lists. `_GameTime` and `_Cycle` stay as in the file.

Measured pace: from the load to the target tick, ~6 ticks/s (78 → 96 in ~3 s; 78 → 128 in ~8 s). With `esperar_variavel` (polling every 0.05 s) followed by `pausar`, the three runs stopped exactly at the target tick (96, 128 and 90). A full case run takes ~25 s plus the simulation time.
## Tick mode (gdbstub breakpoints)

Stops and advances the simulation exactly one tick at a time. Details, experiments and test protocol: [spec/regras/testes-tick.md](spec/regras/testes-tick.md).

- **Code selector.** `_DoAntSim` is at `1028:0000` (NE code segment 6). Like the data ones, the selector only exists at run time, but it is not in the save table: `Memoria.seletor_codigo(6)` searches the LDT for present code descriptors whose limit equals the segment's size in the NE and compares the first 64 bytes with `SIMANTW.EXE`. Observed: `0x1547` (linear base `0x80837500`); the 10 segments sit at `0x156f` (1) … `0x1527` (10), decreasing in steps of 8.
- **Breakpoints at linear addresses** (`Z0`; QEMU compares against CS base + EIP): at the entry of `_DoAntSim` and at the far return address read from the stack at the entry (`1000:2688`, inside `MYTIMERFUNC`). Each one is only enabled while running up to it.
- **Persistent session.** The connection to the gdbstub stays open; `escrever_variavel` uses it too. With the VM running, the script interrupts through the gdbstub (byte 0x03) before any command.
- **Read watchpoints** (`Z3`) in `Depurador.mapear_leituras`: QEMU reports the start of the watchpoint that was hit, not the byte read; that is why the ranges are cut into pieces.
- **Pace:** ~0.45 s per `passo_tick`; `ate_gametime` advances ~3 ticks/s (the free-running VM does ~6).
- The game must be running the simulation (not paused or minimized); with no stop within 120 s, the script interrupts and gives up.
- **Branch that depends on a random draw:** record the tick without forcing anything, search in the port for the entry S seed that leads to the branch (`cargo run -p engine --release --example buscar_semente -- DIR_TICK SIMBOLO[ou NOME[i]] [VALOR|nz]`) and write it (`SementeS`) at the entry of the same tick before recording again (testes-tick.md, seventh and eighth rounds).

## Logo with the top half white (investigated)

Symptom (old screenshot `re_trabalho/q30.png`, at 30 s): on the title, the top part of the logo rectangle appears white and only the bottom part is drawn.

Test: `qemu_titulo_logo.txt` captures the screen every ~1.4 s, from 24 s to 49 s, with `--mem 16` and with `--mem 32`. Sequence observed in both cases (with 32 MB, everything ~1 s later):

1. title window with the logo rectangle all white;
2. client area all black (the game clears the window);
3. white rectangle again;
4. the bitmap is painted **from bottom to top**: in one screenshot (31.2 s with 16 MB; 32.4 s with 32 MB) the top lines (up to y ≈ 270, or 340) are still white and the bottom ones already have the ant;
5. from ~32.5 s (16 MB) or ~33.7 s (32 MB) on, the logo is **complete** and stays that way until the end (49 s), with the same shape as captured in DOSBox-X.

**Cause:** it is not memory, nor the 9/0 position table, nor a VGA driver defect. It is a transient state: the bitmap (DIB, stored bottom to top) is drawn slowly under TCG with the 16-color VGA driver (which converts and dithers the colors), and the old screenshot landed in the middle of the drawing. To capture the title, wait at least ~35 s after boot (`qemu_entrada.txt`, with 35 s, already gets the complete logo). The colors differ from DOSBox-X's (16 dithered colors versus 256 colors), so the pixel-by-pixel comparison of the title is still done only in DOSBox-X.

## Extra QEMU arguments: `-icount` and sound card (2026-10-18)

`roteiro ... --qemu-args "ARGS"` (or the `SIMANT_QEMU_ARGS` variable, which also applies to `cenarios_tick.py` and `gerar_vetores_tick.py`, which create the `Bancada` without arguments) appends arguments to the QEMU command line.

- **`-icount shift=N,sleep=on`** ties the VM clock to the instruction count (2^N ns per instruction). A snapshot saved without `-icount` loads, but the simulation stays stopped (`GameTime` and `_gameCycles` do not advance in 60 s); a full boot is needed (`tools/ritmo_icount.py`). TCG with `icount` emulates only ~3–8 million instructions per host second, so the VM falls behind the host clock: VM time has to be measured inside it. The BIOS tick counter (`0040:006C`, read at linear address `0x46C` with `Memoria.ler_linear`) works: at normal speed it gives **6.07 ticks/s** in VM time with shift 5, 6 and 7 (the model gives 6.06), versus 1.5–3.7 ticks/s on the host clock. Results of the measurements: [spec/regras/simulacao-visao-geral.md](spec/regras/simulacao-visao-geral.md) §3.3.
- **Sound:** `-audiodev wav,... -device sb16,audiodev=...,irq=7 -parallel none [-device adlib,...]`; recording works, but it is not usable as a reference (OPL2 versus an OPL3 driver, silences discarded, effects that do not match): [spec/regras/som.md](spec/regras/som.md), "Original audio on the QEMU bench". It also requires a full boot.

## SVGA bench: 800×600 and 1024×768 in 256 colors (2026-10-19)

To check the port's virtual screen (spec/telas/resolucao.md) there is a separate disk C: with the free driver **vbesvga.drv** (github.com/PluMGMK/vbesvga.drv, release v1.0-beta4; zip downloaded to `re_trabalho/downloads/vbesvga/`, SHA-1 of the files checked against the package's `VBESVGA.SHA`), which uses the VBE of QEMU's `-vga std`. Nothing from the package runs on the host: `VBESVGA.DRV`, `VDDVBE.386` and `VBEVMDIB.3GR` are only copied to `WINDOWS\SYSTEM` of the copy.

- `python tools/qemu_bancada.py preparar_svga LARGURA ALTURA [BPP] [--disco ARQ.img] [--drivers DIR]` assembles `re_trabalho/qemu/svga_c_LxA/` from `SimAnt-T` and generates `svga_c_LxA.img` (DOSBox-X with `-silent`, no window). It refuses names that do not start with `svga`; it does not touch `c.img` nor the default bench's overlays.
- `SYSTEM.INI` of the copy: `[boot]` `display.drv=vbesvga.drv`, `386grabber=vbevmdib.3gr`; `[386Enh]` `display=vddvbe.386` (plus the default bench's tweaks); section `[VBESVGA.DRV]` with `Width`, `Height` and `Depth=8`. VGA fonts (96 dpi).
- `roteiro ... --disco ARQ.img --overlay ARQ.qcow2` uses that disk (with `--disco` and snapshots, `--overlay` is mandatory). Scripts: `qemu_svga_boot.txt`, `qemu_svga_selecao.txt`, `qemu_svga640_quick.txt`, `qemu_svga_quick.txt` (800×600) and `qemu_svga1024_quick.txt`. The boot is slower (full title around 60 s).
- `preparar_svga ... --ajuda-padrao` removes from the copy of `WIN.INI` the stored WinHelp position (`[Windows Help] H_WindowPosition`, which the user's installation has had since a session at 800×600), so that the viewer opens at the default place, as on the VGA bench.
- Resolution-dependent screens (2026-10-19): `qemu_svga{640,800,1024}_telas_boot.txt` (boot, title, Select a Game, paused Quick Game and `quick` snapshot), `qemu_svga{640,800,1024}_telas.txt` (Info, yard, About, "SimAnt Quit", "SimAnt Load", file dialogs, "Saved correctly", WinHelp, "fred" cheat in window 26) and `qemu_svga{640,800,1024}_fim.txt` (game over with the `C10.ANT` from the floppy of `qemu_mensagens.txt`), on `svga_ag_LxA.img` disks prepared with `--ajuda-padrao`. Screenshots `re_trabalho/svga/ag*`. **Caution:** after a `snapshot_carregar`, the vbesvga 256-color palette does not come back (black grays, purple titles): the screenshots are good for positions, not for colors; colors are only valid in the boot screenshots.
- Colors in 256 colors (2026-10-19): to check colors, capture everything in the **same boot** (without `snapshot_carregar`): `roteiros/qemu_svga640_256cores.txt` (menus), `qemu_svga640_mini.txt` (mini map) and `tools/cenarios_fecha.py svga_examine DISCO OVERLAY SAIDA` (Examine on ants found in the state read). `tools/comparar_captura6.py PORT.png CAPTURA.png X0 Y0 X1 Y1 [--topo 38] [--diff D.png --zoom N]` compares the port's export with the screenshot at 6 bits, exact colors (without `--mapear`).
- `tools/png_diff6.py` compares two PNGs at 6 bits per channel (the DOSBox-X DAC expands 6 bits to 0x82/0xC3, QEMU's gives 0x80/0xC0); `tools/png_cores.py` counts the colors of a PNG.

Result: 386 enhanced mode, 8 bpp; the active title bar comes out light blue (#a4c8f0, the same as DOSBox-X), like the default Windows scheme in 256 colors. The title at 800×600 is identical (0 differing pixels at 6 bits) to the old DOSBox-X screenshot. Screenshots in `re_trabalho/svga/` (`svga800_*`, `svga1024_*`); comparisons in `tests/referencias/README.md`, "Virtual screen".

## Cautions

- DOS writes with a delay. Before shutting down, wait about 15 s after saving; `fim` already uses `quit`. The first save (`QUICK1.ANT`) came out with 0 bytes because QEMU was killed right after saving.
- The `c.img` disk persists between runs. Use `preparar` to go back to the clean state, or new file names. With an overlay, `snapshot_carregar` also brings the disk back to the snapshot state.
- Times are from the host clock (QEMU is not in deterministic mode). `pausar` freezes the VM to read a consistent state, but the number of ticks between two commands varies from one run to another. For determinism, use tick mode: stopping at the entry of `_DoAntSim`, the original is reproducible tick by tick (except `_BirdDelay`), without needing `icount` (see [spec/regras/testes-tick.md](spec/regras/testes-tick.md)).
- `ler_variavel`, `escrever_variavel` and `estado` only work with the game loaded (the save table only exists with `SIMANTW.EXE` in memory). Before that, they fail with "tabela do save não encontrada".
