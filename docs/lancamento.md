*Leia em português: [pt-BR/lancamento.md](pt-BR/lancamento.md)*

# Release of version 1.0

Working document for the **1.0.0** release, prepared on 2026-10-19 and **approved by the user on
2026-10-20**. The commit "Release 1.0.0" on `main` sets the version to `1.0.0` in `Cargo.toml`, freezes the
mod API at **1.0** (it was 0.43) and dates the changelog (`[1.0.0] — 2026-10-20`). **Not published yet:** no tag
was created and nothing was pushed; the remaining steps are in section 4, "Release". Decisions taken along
the way are marked **decided**; the remaining optional ones, **[DECIDE, optional]**.

Files prepared in this branch:

| File | What it is |
|---|---|
| `.github/workflows/release.yml` | build and package for Windows x64 and Linux x64, triggered **only** by a `v*` tag; creates the release as a **draft** |
| `tools/gerar_avisos_terceiros.py` | generates the package's `AVISOS-TERCEIROS.txt` (licenses of what goes into the executable) |
| `CHANGELOG.md` | the history, in English, with the section `[1.0.0] — 2026-10-20`; the source of the release notes (goes in the package as `CHANGELOG.txt`) |
| `CHANGELOG.pt-BR.md` | Portuguese version of the history (goes in the package as `CHANGELOG.pt-BR.txt`) |
| `LEIAME-usuario.md` | player's guide in Portuguese (goes in the package as `LEIAME.txt`) |
| `README-user.md` | player's guide in English (goes in the package as `README.txt`) |
| `docs/lancamento.md` | this document |
| `simant/src/launcher/`, `config_launcher.rs`, `registro.rs`, `pastas.rs` | launcher, configuration, console-less output and folders next to the executable (section 4, "Improvements made") |

## 1. Project license — **decided (2026-10-19): "MIT OR Apache-2.0"**

The user chose the recommended option. Files: `LICENSE` (notice of the dual license and of the exclusion of the original SimAnt files), `LICENSE-MIT`, `LICENSE-APACHE` (official text from apache.org); `license = "MIT OR Apache-2.0"` in the workspace `Cargo.toml`; the package carries all three. The analysis that led to the choice follows below.

*Analysis made before the decision (kept for the record):* the repository **had no `LICENSE`** (checked: no `LICENSE*`, no `COPYING*`, no `license` field in the
`Cargo.toml` files). Without a license, public code on GitHub is "all rights reserved": it can be read,
but no one can legally redistribute, modify or package it. The choice is the author's; the release
workflow **fails on purpose** as long as there is no `LICENSE` at the root.

All dependencies (section 2) are MIT, Apache-2.0, zlib or dual-licensed: **the three options below are
compatible with them.** What changes is what others can do with the port and with the mods.

| Option | Pros | Cons |
|---|---|---|
| **MIT** | The simplest and best known; the same as most dependencies (Lua, mlua, sdl2, rustysynth); anyone can package it (Linux distributions, stores, Flatpak) and reuse parts (parsers for the Maxis formats, FM synthesizer) without friction. | Allows closed forks and commercial use without contributing improvements back. No patent clause. |
| **Apache-2.0** | Permissive like MIT, plus an explicit patent grant and clear rules for contributions (section 5); the standard of many Rust projects (usually as "MIT OR Apache-2.0"). | Long text; requires marking modified files and keeping a `NOTICE` if there is one; incompatible with GPL-2.0-only (compatible with GPL-3.0). Patents are of little relevance here. |
| **GPL-3.0** (or **-or-later**) | Copyleft: whoever distributes a modified version must publish the code; keeps the port and its derivatives open (it is the choice of several game ports and reimplementations, such as OpenTTD and OpenRCT2). | Puts off those who want to reuse parts in permissive or closed projects; questions about mods: Lua scripts that only use the API and data files in general are not a "derivative work", but it is worth stating this (an exception or a note in `docs/api-mods.md`). Distributing a binary requires offering the corresponding source code (the GitHub link with the tag is enough). |
| The combination **"MIT OR Apache-2.0"** | Convention of the Rust ecosystem; each user chooses. | Two license files; little gain for a game. |

Points that apply to any choice:

- The license covers **only the port's code and data**, never the Maxis files. It is worth repeating this in the
  `LICENSE` (separate paragraph) and in the README.
- **Content of `data/` coming from the original — decided (2026-10-19): option (b), see section 6.** The `data/*.json` files contained numeric tables
  extracted from `SIMANTW.EXE` (`tools/extrair_*.py`) and a few dozen short English interface phrases from the
  game (for example in `mensagens.json`, `dialogos.json`, `amarela.json`,
  `superficie.json`, `ajuda.json`). Rule numbers are usually treated as facts; the game's phrases
  are Maxis text. The repository is already public with them; before packaging them in a release,
  it is worth deciding: (a) keep them as they are, (b) read those phrases from the user's database/executable at
  run time (as already happens with most of the texts, which come from `SHARED`) or (c) state in the
  license that those excerpts are not covered by it.
- After the choice: create `LICENSE`, put `license = "<SPDX>"` and `repository =
  "https://github.com/Tim-campy/simant-recomp"` in `[workspace.package]` (and `license.workspace = true`
  in both crates), and mention the license in `README.md`.

## 2. Dependency licenses

Gathered with `cargo metadata` and `cargo tree -p simant -e normal,no-proc-macro --target <alvo>`
(`cargo-deny` is not installed; see the optional item in the checklist). Only what is **linked into the
executable** goes into the package; build dependencies (`cc`, `lua-src` as a tool, `pkg-config`...),
macros (`serde_derive`, `syn`, `quote`, `proc-macro2`, `unicode-ident` — the latter Unicode-3.0) and test
dependencies (`sha2` and co.) do not go into the binary.

Linked into `simant` (33 crates on Windows; on Linux `windows-link` drops out):

| Declared license | Crates | Notice in the package? |
|---|---|---|
| MIT | `mlua`, `mlua-sys`, `rustysynth`, `sdl2`, `simd-adler32`, `zmij` | **Yes**: MIT requires the copyright and permission notice in "all copies", including binary ones |
| MIT AND Zlib | `sdl2-sys` (the zlib part is the SDL headers) | Yes (the MIT part) |
| MIT OR Apache-2.0 (and variants: `Apache-2.0 OR MIT`, `MIT/Apache-2.0`) | `bitflags`, `bstr`, `cfg-if`, `crc32fast`, `either`, `fdeflate`, `flate2`, `itoa`, `lazy_static`, `libc`, `lock_api`, `num-traits`, `parking_lot`, `parking_lot_core`, `png`, `rustc-hash`, `scopeguard`, `serde`, `serde_core`, `serde_json`, `smallvec`, `windows-link` | Yes, under MIT (chosen in the package) |
| 0BSD OR MIT OR Apache-2.0 | `adler2` | Optional (0BSD does not require a notice); included under MIT |
| Unlicense OR MIT | `memchr` | Optional; included under MIT |
| MIT OR Zlib OR Apache-2.0 | `miniz_oxide` 0.8 and 0.9 | Included under MIT |

Besides the crates, also inside the package:

| Component | License | Notice |
|---|---|---|
| **Lua 5.4.8**, compiled into the executable (`mlua`, feature `vendored`, via `lua-src`) | MIT (© Lua.org, PUC-Rio) | **Yes** |
| Rust standard library (statically linked) | MIT OR Apache-2.0 | Yes (MIT) |
| **font8x8** (the launcher's 8 × 8 font, `simant/src/launcher/fonte8x8.rs`; Daniel Hepper, derived from IBM's VGA fonts) | **public domain** | Not required; the package cites the origin |
| **SDL2 2.30.9** (`SDL2.dll`, only in the Windows package; official binary from libsdl.org, unmodified) | **zlib** (`vendor/SDL2-2.30.9/COPYING.txt`, checked) | zlib **does not require** a notice in binaries (it only asks not to misrepresent the origin and to keep the notice in *source* distributions), but the package includes the text and `README-SDL.txt`, as SDL itself recommends |

No dependency is copyleft (GPL/LGPL/MPL). `tools/gerar_avisos_terceiros.py` assembles
`AVISOS-TERCEIROS.txt` from what is in `Cargo.lock` and from each crate's license files
(46 KB on Windows), so the list follows dependency updates without manual editing. On
Linux SDL2 is the system's and does not go into the package.

## 3. The package

### Contents

```
simant-recomp-1.0.0-windows-x64/          simant-recomp-1.0.0-linux-x64/
  simant.exe   (launcher built in)          simant      (launcher built in)
  simant-console.bat (console output)       simant.sh   (shortcut; no longer needed)
  SDL2.dll, README-SDL.txt
  data/          (44 JSON + idiomas/)       data/
  exemplos/mods/ (exemplo-lua, trapacas)    exemplos/mods/
  mods/LEIAME.txt (empty folder for mods)   mods/LEIAME.txt
  LICENSE.txt, LICENSE-MIT.txt,             LICENSE.txt, LICENSE-MIT.txt,
  LICENSE-APACHE.txt,                       LICENSE-APACHE.txt,
  LEIAME.txt, README.txt,                   LEIAME.txt, README.txt,
  CHANGELOG.txt, CHANGELOG.pt-BR.txt,       CHANGELOG.txt, CHANGELOG.pt-BR.txt,
  AVISOS-TERCEIROS.txt                      AVISOS-TERCEIROS.txt
```

`README.txt` is the English user guide (`README-user.md`), `LEIAME.txt` the Portuguese one
(`LEIAME-usuario.md`); `CHANGELOG.txt` is the English `CHANGELOG.md` and `CHANGELOG.pt-BR.txt` the
Portuguese `CHANGELOG.pt-BR.md`.

**Never included:** game files (`SimAnt-T/`, `*.DAT`, `*.NDX`, `*.HLP`, `*.FON`, `*.CFG`, `*.SYM`,
`SIMANTW.EXE`, `WIN.INI`), `*.ANT` saves (not even those in `tests/referencias/saves/`), `re_trabalho/`, `.re/`,
captures (`tests/referencias/capturas/`, `*.png`), `referencia/` (DOSBox-X, `qemu/`, images),
`research_notes/`, WAVs, MIDIs and SoundFonts. The package is assembled by copying only the list above (never the repository
root), and the workflow's "Check the contents" step **fails the build** if any
file with those extensions or paths shows up.

### Size (measured locally, Windows x64)

Package assembled by hand with the same steps as the workflow, from the `cargo build --release -p simant`
of 2026-10-19 (without the `LICENSE`, which does not exist yet):

| Item | Size |
|---|---|
| `simant.exe` | 5.8 MB (5,784,576 bytes) |
| `SDL2.dll` | 1.6 MB |
| `data/` + `exemplos/` + texts | ~0.45 MB |
| **Total uncompressed** | **7.7 MB in 57 files** |
| **`.zip`** | **2.9 MB** (2,864,192 bytes) |

The Linux package was not assembled locally (only on the runner): without `SDL2.dll`, it should be around
6 MB uncompressed and 2–2.5 MB as `.tar.gz` [estimate, check in the release draft].

### Verified locally (2026-10-19, Windows 11, `SIMANT_DIR` = the `SimAnt-T` installation)

- `cargo build --release -p simant`: compiles without warnings (2 min 44 s from scratch).
- Without a window: `simant --novo-jogo --ticks 5 --sem-preferencias` → `5 ticks: GameTime 0 -> 5`, exit code 0
  (0.5 s); the same with `--sem-som` (0.05 s); `--save tests/referencias/saves/QUICK2.ANT --ticks 5
  --sem-som` → `GameTime 78 -> 83`.
- From inside the package folder: `--novo-jogo --ticks 600 --sem-som --mods exemplos/mods` loads the
  two example mods (`[exemplo-lua] tick 500: 27 pedaços de comida`), exit code 0.
- With a window: `--sair-apos-ms 2000` with and without `--sem-som` opens the 640×461 window and exits with code 0.
- Without the game files: `--novo-jogo --ticks 5 --jogo <pasta inexistente>` → `erro: não encontrei
  .../SHARED.ndx`, exit code 1, no panic (this is what the workflow's smoke test on Linux expects).
- `.github/workflows/release.yml` is valid YAML (read with PyYAML; trigger only `push: tags: ["v*"]`).

Verified on 2026-10-19 after the launcher (Windows 11, release build, windowed executable):

- `cargo build --release -p simant`: `simant.exe` at 6,053,376 bytes (was 5,784,576).
- With output redirected (pipes, as in CI): `--config <cfg> --novo-jogo --ticks 600 --mods
  exemplos/mods` prints the mods' `[script]` lines and `600 ticks: GameTime 0 -> 600`; `--sem-config
  --novo-jogo --ticks 5 --jogo /nao/existe` → `erro: não encontrei .../SHARED.ndx`, exit code 1.
- Without a console or redirection (`Start-Process`): the same output went to `%APPDATA%\SimAnt
  recomp\SimAnt\simant.log`; with the wrong game folder the error box appeared (closed through its message
  queue) and the exit code was 1; with `--console`, exit code 0 and the log was not touched.
- Demonstration package in another folder (exe, `SDL2.dll`, `data/`, `exemplos/mods`), run with the current
  folder at `%TEMP%`: `--mods exemplos/mods --ticks 600` found `data/` and the mods next to the executable.
- Launcher: `--launcher-jogar-apos-ms 800 --config <cfg>` opened the launcher window, pressed Play,
  wrote `launcher.json` and opened the game in the same process (`--sair-apos-ms 1500 --capturar-janela`
  closed and saved the screen); the Windows folder and file dialog was checked by the test
  `dialogo_do_windows_escolhe_pasta_e_arquivo_e_cancela` (ignored by default: it opens the dialog for ~1 s).
- `cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` passes (with
  a fake C compiler for Lua: only the Rust code for the Linux target was checked, nothing was linked or
  run on Linux).

**Not verified:** the workflow itself (it has not been triggered); the launcher on Linux (window, `zenity`/`kdialog`,
`simant.log` opened by the file manager); the build and execution on Linux; the game on Linux
with the game files in lowercase names (the database accepts `.NDX`/`.ndx`, the loose files are
looked up case-insensitively, but there is no test on a case-sensitive file system);
`simant-console.bat` on double-click (`--console` was checked only through `AllocConsole`).

## 4. 1.0 checklist

### Before the tag

- [x] License (section 1): **decided (2026-10-19), "MIT OR Apache-2.0"**; `LICENSE`, `LICENSE-MIT`,
      `LICENSE-APACHE` and `license` in the workspace `Cargo.toml`.
- [x] Original phrases in `data/`: decided (2026-10-19) and done — read from the user's installation (section 6).
- [x] **Version 1.0.0 in the `Cargo.toml` files** (2026-10-20): `version = "1.0.0"` in `[workspace.package]`
      (both crates inherit it) and `Cargo.lock` updated by `cargo build`. The workflow checks that the
      `vX.Y.Z` tag equals this version and fails if it does not.
- [x] **Mod API frozen at 1.0** (2026-10-20): `VERSAO_API = "1.0"` in `engine/src/scripts.rs` (it was
      0.43); `docs/api-mods.md` with "API version: **1.0**", the summary of the current API at the top,
      the `1.0` line in the history (0.43 frozen; 1.x only compatible additions, incompatible = 2.0) and
      "Planned" reviewed (the removal of the old copies of the common tables only in 2.0). The example
      mods only print `API_VERSAO` and the tests use the constant. Decision in `docs/decisoes.md`.
- [x] `CHANGELOG.md` reviewed and dated (`[1.0.0] — 2026-10-20`, draft notice removed; numbers measured on
      2026-10-20, also in the header of `docs/estado-do-projeto.md`).
- [x] `LEIAME-usuario.md` and `README-user.md` reviewed ("draft" removed; version 1.0.0, simple
      synthesizer by default, frame on by default, launcher in English and Portuguese).
- [x] Repository `README.md`: download section pointing to the releases, the legal notice (the same
      as in the LEIAME: no affiliation with Maxis/EA, does not distribute game files), the license and,
      since 2026-10-20, the 1.0.0 status and the languages.
- [x] `cargo test --workspace` with and without `SIMANT_DIR` (2026-10-20: 693 passed, 1 ignored, with the
      game files; without them, the tests that need the files warn and pass); `cargo clippy --workspace
      --all-targets -- -D warnings` with no warnings; `python tools/conferir_links_docs.py` with no
      problems; package assembled locally as the workflow does, with no game file (section 3).
- [ ] Test the Linux package on a real machine with the game files (including with lowercase
      names, as they come off some CDs and copies). **Not done:** a known limitation of 1.0.0
      (`CHANGELOG.md`).
- [ ] Optional: `cargo install cargo-deny` and a `deny.toml` with the list of accepted licenses
      (MIT, Apache-2.0, Zlib, 0BSD, Unlicense, Unicode-3.0), running in CI.

### Improvements made for 1.0 (2026-10-19)

- **Folders relative to the executable:** `data/`, `mods/` and `SimAnt-T/MAXIS/ANTWIN` are looked up next to
  the executable and only then in the current folder (`simant/src/pastas.rs`); the Linux `simant.sh` remains only as a
  shortcut.
- **No console on Windows, with a log and an error box:** `windows_subsystem = "windows"` in the release profile,
  `simant.log` in the user's data folder, `SDL_ShowSimpleMessageBox` on fatal errors, `--console` and
  `simant-console.bat` (`simant/src/registro.rs`).
- **Launcher:** `simant` without options opens the startup options screen, with the game folder
  (native picker and file check), start, video, sound, mods, preferences and the equivalent
  editable command line; the choice is kept in `launcher.json` and also applies to `simant` run with
  options (`simant/src/launcher/`, `simant/src/config_launcher.rs`). Capture in `docs/imagens/launcher.png`.
  Decisions in `docs/decisoes.md` (2026-10-19: launcher, console-less executable, folders).

### Suggested improvements (not made)

- Icon and metadata for the `.exe` (version resource).
- Launcher: text fields without mouse selection (cursor only, Ctrl+V pastes and Ctrl+C copies the whole
  field); no screen reader support; on Linux without `zenity`/`kdialog` there is no picker (the path is typed).

### Release (done by the user)

`main` already has the commit "Release 1.0.0" (version, API 1.0, dated changelog). No tag was created and
nothing was pushed. The final steps:

1. **Push `main`** (`git push origin main`) and wait for the green CI (`ci.yml`: tests, clippy, docs
   links).
2. **Tag:** `git tag -a v1.0.0 -m "SimAnt recomp 1.0.0"` on that commit and `git push origin v1.0.0`.
3. **Workflow:** the "Lançamento" workflow runs on the tag: version check (tag = `Cargo.toml`) and
   `LICENSE`, tests, build, package, content check and smoke test on Windows and Linux, and creates the
   **v1.0.0 draft** release with both packages and the `.sha256` files, and with the notes taken from
   the `[1.0.0]` section of `CHANGELOG.md` plus the legal notice.
4. **Draft:** download both packages from the draft, check the contents (no game file) and run them with
   the game files; read the notes.
5. **Publish** the draft on the GitHub releases page (the workflow never publishes on its own).

If something fails after the tag: delete the draft and the tag (`git push --delete origin v1.0.0`,
`git tag -d v1.0.0`), fix it and recreate the tag.

## 5. Workflow details

- **Trigger:** only `push` of a `v*` tag. It does not run on branch pushes or on PRs (`ci.yml` still takes care
  of that).
- **Windows** (`windows-2022`, `x86_64-pc-windows-msvc`): downloads `SDL2-devel-2.30.9-VC.zip` from SDL's official
  release and checks the SHA-256 (`8c91d91e…52ae`, the same as the local `vendor/`) before extracting into
  `vendor/`, where `simant/build.rs` already looks; `SDL2.dll` goes into the package.
- **Linux** (`ubuntu-22.04`, glibc 2.35, to run on not-so-new distributions): the system's `libsdl2-dev`;
  the executable links dynamically against `libSDL2-2.0.so.0`, which the user installs (LEIAME §1).
- **Steps:** tag version = `Cargo.toml` and `LICENSE` present → `cargo test --workspace --release` →
  `cargo build --release --locked` → package → content check → smoke test on both systems,
  from outside the package folder (error without the game files, exit code 1; capture of the launcher without a window,
  uploaded as the `launcher-<so>` artifact, outside the package) → `.zip`/`.tar.gz` with `.sha256` → artifacts → the `rascunho` job creates the release with
  `gh release create --draft --verify-tag`.
- **Permissions:** `contents: read` by default; only the `rascunho` job has `contents: write`.

## 6. Original texts and tables in `data/` — **decided (2026-10-19): read the texts from the user's installation**

The user chose option (b) of section 1: the SimAnt phrases that were copied into the `data/*.json` files are now
read **from the user's installation at run time** and leave the repository. In place of each text,
the JSON keeps the **reference** to the source, which `Recursos::dados` (`engine/src/recursos.rs`, `Referencia`)
resolves after the mods are merged. Reference format and rules for mods: `docs/api-mods.md`,
"Original texts" (mod API **0.35**; a mod that ships the literal text is still valid). Record of the
decision: `docs/decisoes.md` (2026-10-19).

### 6.1 Inventory of the original's texts in the repository (sweep of 2026-10-19)

Swept: all `data/*.json` files (values and `_…` comments), the Rust code (`engine/`, `simant/`), the tests,
`exemplos/mods/`, `tools/` and `docs/`. The source of each text was located in the files of the reference
installation (`SimAnt-T/MAXIS/ANTWIN`): type-4 string lists in the databases, segments of `SIMANTW.EXE`
(numbering from the `.SYM`; address = offset within the segment), window layout objects and the `.HLP`'s `|SYSTEM`.

**Replaced by a reference** (29 references; text checked byte by byte, see 6.4):

| File → key | Source |
|---|---|
| `mensagens.json` → `faixa.velocidade[0..3]` (four "Game speed now …") | `SIMANTW.EXE` 10:014b, 10:0160, 10:0177, 10:018c (`_DoUserButton`) |
| `mensagens.json` → `faixa.marcador_posto`, `marcador_ausente` | 10:01a2, 10:01bc (`_DoBookMark`) |
| `dialogos.json` → `caixa_mensagem.titulo_load`, `pergunta_load` | 10:17eb, 10:17f7 (`_LoadGame`) |
| `dialogos.json` → `caixa_mensagem.titulo_quit` | 10:1650 (`_MenuQuit`; the question repeats at 10:165c) |
| `dialogos.json` → `caixa_mensagem.titulo_mensagem` ("Message") | 10:185a (tail of "SimAnt Message" at 10:1853, `_PopMsg`) |
| `dialogos.json` → `caixa_mensagem.salvo` (format with `%s` and a line break) | 10:1840 (`_PopMsg`) |
| `dialogos.json` → `arquivo.titulo_abrir`, `titulo_salvar`, `texto_tipo`, `campo_inicial` | 10:1745, 10:1763, 10:1715, 10:1729 (`_FileSelect`) |
| `janelas.json` → `tarefas.titulo`, `castas.titulo`, `historico.titulo` | object 1 (type 18, title) of the `WINGANT` layouts 0/18, 0/19, 0/21 (identical in `MWINNT`) |
| `quintal_mapa.json` → `titulos[0..3]` (Yard/House/Strategic/Graphic View) | `SHARED` 4/1000, indices 9–12 (`_SetMapTitle`) |
| `quintal_mapa.json` → `balao_menino.pecas` (10 pieces 8 × 8 in "ASCII art") | 10:1c6e, 11 pieces of 16 bytes (`_MakeBalloon`; the 10th, the right-pointing wedge, is not used by the port) |
| `tutorial_arquivo.json` → `nome` (`Tutorial.Ant`) | `SHARED` 4/1800[0] |
| `tutorial_arquivo.json` → `titulo` ("Tutorial Game") | 10:17dd (`_LoadGame`; the same text is in `SHARED` 4/1001[0]) |
| `barra_menus.json` → `texto_pausa[0..1]` (Pause/Unpause item with the shortcut) | 10:0911, 10:0900 (`_SetMenuOptionText`) |
| `moldura_principal.json` → `texto` ("SimAnt") | 10:0626 (`CreateWindow` in `_InitInstance`) |
| `ajuda.json` → `titulo` | title of `SIMANT.HLP` (`|SYSTEM`: "SimAnt") with the format `"{} Help - SIMANT.HLP"` (the measured bar is that text; that WinHelp builds it this way is [hypothesis]) |

**Removed from comments** (`_…` in the JSON files and in the extractors): the phrases quoted in `amarela.json` → `mensagens._faixa`
(`SHARED` 4/1010[4, 11, 12, 13]), `superficie.json` → `_origem.mensagem_ataque_vermelho` (4/1020[5]) and
`barra_menus.json` → `_texto_pausa`, and the same ones in `tools/extrair_amarela.py` and `tools/extrair_superficie.py`.

**Rust code:** the only original text used at run time was the default title `"SimAnt Quit"` in
`dialogos.rs` (when a mod removed the key): it is gone (without the key, the Load title applies). What remains: documentation
comments that quote a short phrase to say which message the rule shows (`rules/troca_patch.rs`,
`rules/amarela.rs`, `rules/superficie.rs`, `core/partida.rs`, `core/dialogos.rs`), the `"SimAnt"` title of the SDL
window and the `"Tutorial Game"` label of the console output in `simant/src/main.rs` (names, not phrases), and the
`"SimAnt"`/`"Windows Help"` sections of `WIN.INI` read by `simant/src/preferencias.rs` (format keys).

**Tests:** those that compared the original's phrases now compare the reference (`partida_quick2.rs`,
`core/avisos.rs`), a SHA-256 of the whole text (`modos_reais.rs`, three Info card texts, previously
checked by excerpts) or a description in Portuguese (`assert` messages in `regras_quintal.rs` and
`partida_quick2.rs`). What remains, because they are short names that identify what the test checks: the `.HLP` topic
titles in `ajuda_reais.rs` and `simant/src/ajuda_ui.rs` ("Patch Ribbon Bar", "Welcome to SimAnt"…),
the card and item titles in `modos_reais.rs` ("Top Card", "Queen"…) and the label "Click here to continue"
measured in `arquivos_reais.rs` (text of the `gameOverClickHere` object of window 4).

**Documentation:** `docs/spec/` quotes about 60 short phrases from the game (one phrase per message, ~3 KB in
total; the largest in `regras/inimigos.md`, `regras/trapacas.md`, `regras/simulacao-visao-geral.md`,
`telas/quintal.md`) to document **which** message appears in each rule, as principle 3 of
`CLAUDE.md` requires. They are short, specific quotations: they stay. No doc reproduces `.HLP` topics or whole card
texts. In `telas/abertura.md`, the quotation of the registration screen included the name and customer number
**of this copy**: they were removed. Test bench tools (`tools/caso_ant.py`, `cenarios_tick.py`,
`cenarios_raros.py`) and scripts (`tests/referencias/roteiros/*.txt`) quote box titles to recognize them
in the VM: they stay (they do not go into the package).

**Stay as text in the port, because they are not from SimAnt:**

| What | Where | From whom / why it stays |
|---|---|---|
| System menu items (Restore, Move, Size, Minimize, Maximize, Close, Next, Switch To…) | `janelas.json` → `gerenciador.menu_sistema`, `moldura_principal.json` → `itens` | Windows 3.1 (`USER.EXE`). Standard 1–2 word labels |
| Yes/No/Cancel/OK buttons of the message box | `dialogos.json` → `caixa_mensagem.sim`, `nao`, `cancelar`, `ok` | Windows 3.1 (`USER.EXE`) |
| Labels of the common file dialog (File Name, Directories, List Files of Type, Drives, Read Only, OK, Cancel, `c:`) | `dialogos.json` → `arquivo` | Windows 3.1 (`COMMDLG.DLL`) |
| WinHelp menus, buttons and dialogs (File, Edit, Bookmark, Help, Contents, Search, Back, History, Search and History) | `ajuda.json` → `menu`, `botoes`, `janelas` | Windows 3.1 (`WINHELP.EXE`). The two instructions of the Search dialog are the longest phrases (8 and 7 words) |
| Menu check mark (8 × 10 bitmap in "ASCII art") | `barra_menus.json` → `marca` | Windows 3.1; measured on screen |
| Names of the 128 General MIDI instruments and 47 percussion sounds | `opl_banco.json` | GM standard (MMA); the FM bank is the port's |

These texts are not in the SimAnt files; reading them from the user's `WINDOWS` would require a Windows 3.1
installation next to the game, which most users do not have. They are short, functional labels of the
system's standard interface. **[DECIDE, optional]** If you want to remove them too: read from
`../../WINDOWS/SYSTEM/{USER.EXE,COMMDLG.DLL}` and `../../WINDOWS/WINHELP.EXE` when they exist (NE
`STRING`/`MENU`/`DIALOG` resources), with a port text as a fallback.

**Stay because they are identifiers, not text:** window object names (`ribHelp`, `antEffEH`…, the
`SHARED` type 1 names, used to find the objects), symbol names from `SIMANTW.SYM` in
`save_layout.json` (the save format), cursor resource names (`HELPCURSOR`…), file names
(`SIMANTW.EXE`, `SIMANT.HLP`, `font1`, `FONTRES.FON#Gaia8x14`, the 28 songs in `som.json`), `printf`
formats (`%d%%`, `%ld`) and the 26 four-letter cheat codes in `codigos_trapaca.json` (the `_Cheats` table
at 8:83bc, stored inverted; they are the keys the game accepts, published in guides since 1991, and the cheat
mod needs to be able to change them).

### 6.2 What the user sees

Nothing changes on screen: the resolved texts are the same, and the screen tests (hashes) remain the same.
Without the installation, the port already would not open (it needs the databases); now it also needs `SIMANTW.EXE` (which was already
read for the cursors and accelerators) and `SIMANT.HLP` (already read by the help) for these texts. A
reference that does not resolve prevents the file from loading, with the key and the source in the message.

### 6.3 Numeric tables extracted from the executable — analysis

The `data/*.json` files carry tables extracted from `SIMANTW.EXE` by `tools/extrair_*.py` (the address of each one at the
top of the script and in the JSON's `_origem`/`_enderecos`). Count of numeric values (excluding comments):
`quintal.json` 904, `mapa.json` 692, `save_layout.json` 615, `casa.json` 549, `desenho_aranha.json` 338,
`comum.json` 282, `quintal_mapa.json` 269, `geracao.json` 204, `tutorial_arquivo.json` 126, `inimigos.json`
90, `amarela.json` 82, `ninho.json` 74, `simulacao.json` 72, `superficie.json` 46 (the largest, `opl_banco.json`,
is measured by the port and does not come from SimAnt). Analysis:

- **What they are.** Directions and turns (`dx8`, `dy8`, `turn_tab`), mode and caste tables per strategy
  (`modos_operaria`, `castas_vermelhas`…), fight chances (`matriz_luta`), tile edges per direction,
  tiles of the rock shapes (6 of 5 × 5 and 6 of 3 × 3), terrain colors per type (`ctab`, `htab`…), 4 × 4
  dithered blocks of the pheromone ramp, sprite offsets per frame, polygon vertices,
  initial values, tutorial positions, rule constants and the save's address map. They are **facts and
  parameters** of how the game works: values the rule uses, with no expressive content of their own.
- **In their own format.** None is a binary copy: each value is in decimal, with the port's name and structure
  (lists, matrices, objects with Portuguese keys), many reorganized (the recipes in `casa.json` are
  operations written from the code, not copied bytes), and this is what lets mods change the rules
  (principle 4 of `CLAUDE.md`). They serve **interoperability** (reading the saves and reproducing the rules to
  run the user's data).
- **Size.** Small compared with the executable (≈ 4,300 numbers in total, in ~100 KB of JSON with the
  comments, against the 504 KB of `SIMANTW.EXE`); no table is text.
- **Borderline cases**, evaluated carefully: (1) the **balloon pieces** were an "ASCII art" bitmap
  copied from DGROUP: treated as text and replaced by a reference (6.1). (2) `desenho_aranha.json`
  (338 numbers: knee and foot coordinates of the 8 legs in 8 steps, for 2 base directions, and the palps) is the
  table closest to a drawing; they are coordinates of line segments that the code joins with `LineTo`, a
  2-color skeleton, not an image. It stayed as it is; if the user prefers, it can become a reference to the
  executable like the balloon pieces (the `{"exe": …}` format allows a numeric variant). (3)
  `casa.json` → `formas` (coin, paper clip, outlets, button) are 3 × 3 to 5 × 5 tile indices, which point to
  the tiles in the user's database; without the database they draw nothing. (4) `mapa.json` → `blocos` are 16 dithered
  4 × 4 patterns, functional. (5) `save_layout.json` reproduces symbol names and addresses from the `.SYM` to read and
  write the save: it is the description of the file format, necessary for interoperability.
- **Conclusion:** the numeric tables stay in the repository and in the package. None was removed. If it is ever
  necessary, the resolver already knows how to read segments of the executable, and any table can become a reference without
  changing the format seen by the code.

### 6.4 Verified

- `engine/tests/textos_originais.rs`: with the installation, the 29 references give exactly the texts the JSON files
  contained (SHA-256 of each one in the test; the repository does not keep the text). Also run with
  `SIMANT_TEXTOS_ANTIGOS` pointing to a copy of the 44 `data/*.json` files from before the change, made outside the
  repository: each resolved file is **identical to the old one** (except for the comments). Without the installation, the test
  warns and passes; the one that checks that every reference has the expected text registered always runs.
- Unit tests of the resolver (`recursos.rs`): minimal NE, list and layout coming from a mod's `objetos/`,
  format, pieces, mod with literal text and with another reference, errors with the key path.
- `cargo test --workspace` with and without `SIMANT_DIR`, and `cargo clippy --workspace --all-targets -- -D warnings`:
  no failures or warnings; the screen tests (hashes) did not change.
- `simant --novo-jogo --ticks 5`, `--save QUICK2.ANT --ticks 5` and `--save QUICK2.ANT --moldura --sair-apos-ms
  2500` with the installation: run as before.

**Not verified:** the "Message" box screen and the help opened and inspected by eye (covered by the screen tests and by the
comparison of the JSON files); an installation with lowercase file names (the executable and the `.HLP` are found
case-insensitively, like the other loose files, and the databases accept `.ndx`/`.dat`).
