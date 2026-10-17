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
cargo run -p simant -- --save tests/referencias/saves/QUICK2.ANT                        # joga a partida do save
cargo run -p simant -- --novo-jogo                                                       # Quick Game novo (semente do relógio)
cargo run -p simant -- --novo-jogo --semente 1234                                        # Quick Game reproduzível
cargo run -p simant -- --save QUICK2.ANT --exportar-tela-jogo saida/jogo.png            # tela em PNG, sem janela
cargo run -p simant -- --save QUICK2.ANT --acoes superficie,linha_direita --exportar-tela-jogo s.png
cargo run -p simant -- --save QUICK2.ANT --ticks 50 --exportar-tela-jogo s.png          # 50 ticks sem janela
```

Opções: `--jogo DIR` (padrão: `SIMANT_DIR` ou `SimAnt-T/MAXIS/ANTWIN`), `--dados DIR` (padrão `data`), `--mods DIR` (padrão `mods`). Os testes procuram o jogo em `SIMANT_DIR` ou em `../SimAnt-T/MAXIS/ANTWIN`.

### Como jogar

`--save ARQ.ANT` ou `--novo-jogo` abre a tela de jogo (faixa e janela de edição) com a simulação rodando: você é a **formiga amarela**. Ainda não há menus, diálogos, textos de mensagem nem som (mensagens e músicas aparecem no terminal), nem as janelas de mapa, castas e história. Partes das regras ainda faltam (ver `docs/spec/`).

- **Clique** numa célula do mapa: a amarela vai até lá. **Duplo clique**: age (numa formiga de outra cor, ataca; da mesma cor, pede comida; em comida/pedrinha, pega ou larga; no ninho, cava). Clique direito funciona igual. **Espaço/Insert** (e **Delete**) clicam onde está o cursor; duas vezes seguidas, duplo clique.
- Clicar no buraco do ninho leva a amarela para dentro; no ninho, clicar na linha de cima a leva para fora.
- **1** / **2** recrutam 5 / 10 operárias (**Ctrl+2**: todas), **3** / **4** liberam metade / todas, **0** liga o alarme, **X** (ou F9) troca de corpo (depois clique numa formiga), **Backspace** cancela a ordem, **Home** centraliza a vista na amarela.
- Velocidade (menu Speed do original): **Shift+0** pausa/despausa, **Shift+1..4** lenta / normal / rápida / ultra. **Ctrl+G** liga o AutoTrack (a vista segue a amarela; desligado por padrão, como no original). O título da janela mostra a velocidade e a pausa. Com o jogo pausado, a amarela ainda anda até o destino (como no original).
- As teclas da partida ficam em `data/partida.json` (sobrescrevível por mod).

Navegação na tela de jogo:

- **Ctrl + setas** rolam a janela de edição uma célula (como no original); as setas sozinhas e PageUp/PageDown também rolam (extensão do port). Clicar nas setas das barras de rolagem rola, repetindo enquanto o botão fica apertado.
- **Ctrl+8 / Ctrl+9 / Ctrl+0** ou os botões *Surface*, *Nest* (preto) e *Nest* (vermelho) da faixa trocam para a superfície, o ninho preto e o ninho vermelho. Esc fecha.
- As teclas e os botões ficam em `data/navegacao.json` (sobrescrevível por mod). Ações aceitas por `--acoes`: `linha_acima`, `linha_abaixo`, `linha_esquerda`, `linha_direita`, `pagina_acima`, `pagina_abaixo`, `pagina_esquerda`, `pagina_direita`, `superficie`, `ninho_preto`, `ninho_vermelho`.
- Janela: `--escala N` (padrão: a maior escala inteira que cabe no monitor). Para testes sem interação: `--sair-apos-ms N` fecha sozinha e `--capturar-janela ARQ.png` grava o que a janela mostrava. `--ticks N` roda N ticks sem janela (com `--exportar-tela-jogo`, grava a tela depois) e escreve o `GameTime` antes e depois.

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
