# SimAnt recomp

Reimplementação moderna do **SimAnt** (Maxis, 1991, versão Windows 3.x) em Rust + SDL2. O port lê os arquivos originais do usuário, que não são redistribuídos. Diretrizes em `CLAUDE.md`; especificação em `docs/spec/`.

## Requisitos

- Rust stable (toolchain MSVC no Windows).
- SDL2 2.30.9 para Visual C++ extraído em `vendor/SDL2-2.30.9/`. O `build.rs` liga com essa versão e copia o `SDL2.dll` para `target/<perfil>/`. Para baixar:
  `curl -L -o vendor/SDL2-devel-2.30.9-VC.zip https://github.com/libsdl-org/SDL/releases/download/release-2.30.9/SDL2-devel-2.30.9-VC.zip` e descompacte em `vendor/`.
- Arquivos do jogo instalado em `SimAnt-T/MAXIS/ANTWIN/`, ou indique a pasta com `--jogo`.

## Uso

```
cargo test                                         # parsers, regras e mods (testes com arquivos reais se presentes)
cargo run -p simant                                # abertura: Espaço/clique avança, Esc sai
cargo run -p simant -- --exportar-telas saida/     # grava cada tela da sequência em PNG
```

Opções: `--jogo DIR` (padrão `SimAnt-T/MAXIS/ANTWIN`), `--dados DIR` (padrão `data`), `--mods DIR` (padrão `mods`). Os testes procuram o jogo em `SIMANT_DIR` ou em `../SimAnt-T/MAXIS/ANTWIN`.

## Organização

| Pasta | Conteúdo |
|---|---|
| `engine/` | crate sem janela: `formats` (banco NDX/DAT, LZSS, bitmap), `recursos` (original + mods), `core` (sequência de telas), `paleta` |
| `simant/` | executável SDL2 (render e entrada) |
| `data/` | dados do port (paleta, telas, bancos usados): é o que os mods sobrescrevem |
| `mods/` | mods instalados (ver `docs/api-mods.md`) |
| `tools/` | inspetores em Python dos formatos originais; `capturar_original.ps1` e `comparar_captura.py` para comparar o port com o original |
| `referencia/` | configuração do DOSBox-X para rodar o original |
| `docs/` | especificação, decisões, glossário, divergências, API de mods |
| `re_trabalho/` | material de engenharia reversa, fora do controle de versão e fora do port |
