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
cargo run -p simant -- --save tests/referencias/saves/QUICK2.ANT                        # tela de jogo do save
cargo run -p simant -- --save QUICK2.ANT --exportar-tela-jogo saida/jogo.png            # mesma tela em PNG, sem janela
cargo run -p simant -- --save QUICK2.ANT --acoes superficie,linha_direita --exportar-tela-jogo s.png
```

Opções: `--jogo DIR` (padrão: `SIMANT_DIR` ou `SimAnt-T/MAXIS/ANTWIN`), `--dados DIR` (padrão `data`), `--mods DIR` (padrão `mods`). Os testes procuram o jogo em `SIMANT_DIR` ou em `../SimAnt-T/MAXIS/ANTWIN`.

Tela de jogo (`--save`; ainda sem simulação, só a faixa e a janela de edição com o mapa e as formigas do save):

- **Ctrl + setas** rolam a janela de edição uma célula (como no original); as setas sozinhas e PageUp/PageDown também rolam (extensão do port). Clicar nas setas das barras de rolagem rola, repetindo enquanto o botão fica apertado.
- **Ctrl+8 / Ctrl+9 / Ctrl+0** ou os botões *Surface*, *Nest* (preto) e *Nest* (vermelho) da faixa trocam para a superfície, o ninho preto e o ninho vermelho. Esc fecha.
- As teclas e os botões ficam em `data/navegacao.json` (sobrescrevível por mod). Ações aceitas por `--acoes`: `linha_acima`, `linha_abaixo`, `linha_esquerda`, `linha_direita`, `pagina_acima`, `pagina_abaixo`, `pagina_esquerda`, `pagina_direita`, `superficie`, `ninho_preto`, `ninho_vermelho`.
- Janela: `--escala N` (padrão: a maior escala inteira que cabe no monitor). Para testes sem interação: `--sair-apos-ms N` fecha sozinha e `--capturar-janela ARQ.png` grava o que a janela mostrava.

## Organização

| Pasta | Conteúdo |
|---|---|
| `engine/` | crate sem janela: `formats` (banco NDX/DAT, LZSS, bitmap), `recursos` (original + mods), `core` (sequência de telas, tela de jogo, navegação), `paleta` |
| `simant/` | executável SDL2 (render e entrada) |
| `data/` | dados do port (paleta, telas, bancos usados): é o que os mods sobrescrevem |
| `mods/` | mods instalados (ver `docs/api-mods.md`) |
| `tools/` | inspetores em Python dos formatos originais; `capturar_original.ps1` e `comparar_captura.py` para comparar o port com o original |
| `referencia/` | configuração do DOSBox-X para rodar o original |
| `docs/` | especificação, decisões, glossário, divergências, API de mods |
| `re_trabalho/` | material de engenharia reversa, fora do controle de versão e fora do port |
