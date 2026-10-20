*Leia em português: [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md)*

# Contributing to SimAnt recomp

Thank you for your interest. SimAnt recomp reimplements the logic of SimAnt (Maxis, 1991, Windows 3.x)
from a written specification and checks it against the original game. This document explains the
method the project follows; please read it before opening an issue or a pull request.

## Ground rules

- **No original game files in the repository, ever.** No graphics, sounds, music, texts, `.DAT`/`.NDX`,
  `SIMANTW.EXE`, `SIMANT.HLP`, captures of the original's screens or WAV recordings of it. Contributors
  use their own copy of the game (in `SimAnt-T/MAXIS/ANTWIN/`, ignored by git, or via `SIMANT_DIR`).
  Texts from the game are not copied into `data/*.json` either: the data only keeps the reference
  (database, executable, window layout, `.HLP`) and the port reads the text from the user's
  installation. Please do not ask for or share copies of the game in issues or discussions.
- **No copied machine code or decompiled code.** Reverse engineering (Ghidra, hex dumps, the
  `SIMANTW.SYM` symbol table, the QEMU test bench) is used to *understand* the behavior; the port is
  rewritten from the specification, never by pasting decompiled code.
- **Respect the license.** Contributions are dual-licensed under Apache-2.0 and MIT, like the rest of
  the project (see [`LICENSE`](LICENSE)).

## The method

### 1. Specification before code

Before implementing a subsystem, document the observed behavior in [`docs/spec/`](docs/spec/README.md):
file formats, screens, rules and state transitions. When you analyze the binary, write down in the spec
the addresses, original names (for example `_DoAntSim`) and structures you identified, with their
source. New internal names go into the glossary, [`docs/glossario.md`](docs/glossario.md).

### 2. The original is the oracle

When the documentation (the manual, the help, the spec) and the real behavior of the original differ,
the original wins. Record the divergence in [`docs/divergencias.md`](docs/divergencias.md), with what was
observed and how. The port reproduces the original's behavior, including its quirks; deliberate
departures (port extensions, user choices) are recorded in [`docs/decisoes.md`](docs/decisoes.md).

### 3. No claim without evidence

When you describe how the game works, say whether it is a verified fact (tested on the original, or
seen in a dump or in the code) or a hypothesis. Hypotheses are marked `[hypothesis]` until confirmed;
open questions about a cause are marked `[cause unknown]`. Do not invent function names, addresses or
formats: if you don't know, say so and propose how to find out.

The evidence the project uses:

- **Tick vectors** (`tests/referencias/`, see its [`README`](tests/referencias/README.md)): the state of
  the original recorded before and after each tick by the [QEMU test bench](docs/bancada-qemu.md); the
  port must reproduce it bit for bit (`engine/tests/ticks_originais.rs`, `ticks_longos.rs`).
- **Rule cases:** a save injected into the original, run for some ticks, and the resulting state.
- **Pixel-by-pixel comparison** of the port's screens with captures of the original (captures stay
  local, outside git).
- **Extractors** in `tools/` that read the original's tables from the executable and reproduce the
  `data/*.json` files (`tools/conferir_extracoes.py SIMANTW.EXE`).

### 4. Data outside the code

Rules, texts, tables, keys and layouts are loaded from data files (`data/*.json`) or scripts, not hard
coded. This is what makes mods possible. Text shown by the port (launcher, port messages) goes through
the language files, so it can be translated.

### 5. Small, incremental and simple

Prefer a small end-to-end flow before expanding. Keep the code readable and explicit; optimize only
with measurements.

## Mods

- A mod is a folder with a `mod.json` manifest (name, version, dependencies) plus the data or scripts it
  changes. Mods override data by key and never edit the original files.
- Mod scripts (Lua 5.4) run in a sandbox and reach the game only through the public API documented in
  [`docs/api-mods.md`](docs/api-mods.md).
- **Every change to the mod API must be recorded** in the compatibility history of `docs/api-mods.md`,
  with a compatibility note and the new API version. When you propose a change to a data format or to
  the architecture, explain its impact on mods.

## Documentation in two languages

The documentation shown on GitHub is in **English**; every document has a **Portuguese** copy:
`README.pt-BR.md`, `CHANGELOG.pt-BR.md`, `CONTRIBUTING.pt-BR.md`, `docs/pt-BR/<same path as in docs/>`
and `tests/referencias/README.pt-BR.md`. The first line of each file links to the other language. When
you change a document, update both versions (if you can only write one language, say so in the pull
request and someone will complete the other). Keep code identifiers, original function names, paths,
JSON keys and command-line options unchanged in both. The terminology is fixed in the glossary.

## Building and testing

See [Building from source](README.md#building-from-source) in the README. Before opening a pull request:

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The tests that need the game files warn and pass without them; CI (`.github/workflows/ci.yml`) runs
without the game, so if your change touches behavior covered by real-file tests, run them locally with
your copy and say so.

## Pull requests

- Commit messages in English.
- Describe what changed, **what was verified** (tests run, comparison with the original, which vectors
  or captures) and what was not.
- If the change alters behavior, update the spec and, if it departs from the original, `docs/divergencias.md`
  or `docs/decisoes.md` (date, decision, reason, discarded alternatives).
- Add an entry to the changelog (`CHANGELOG.md` and `CHANGELOG.pt-BR.md`) for changes players or modders
  will notice.
