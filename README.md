# SimAnt recomp

Reimplementação moderna do **SimAnt** (Maxis, 1991, versão Windows 3.x) em Rust + SDL2. O port lê os arquivos originais do usuário, que não são redistribuídos, e pode ser modificado por mods (dados JSON e scripts Lua). Diretrizes do projeto em `CLAUDE.md`; especificação em [`docs/spec/`](docs/spec/README.md); situação geral em [`docs/estado-do-projeto.md`](docs/estado-do-projeto.md).

## Instalar

- Rust stable (toolchain MSVC no Windows).
- SDL2 2.30.9 para Visual C++ extraído em `vendor/SDL2-2.30.9/`. O `build.rs` liga com essa versão e copia o `SDL2.dll` para `target/<perfil>/`. Para baixar:
  `curl -L -o vendor/SDL2-devel-2.30.9-VC.zip https://github.com/libsdl-org/SDL/releases/download/release-2.30.9/SDL2-devel-2.30.9-VC.zip` e descompacte em `vendor/`.
- Os arquivos do jogo instalado (a pasta com `SIMANTW.EXE`, `SHARED.DAT`, `WINGANT.DAT`, `SOUND.DAT`...) em `SimAnt-T/MAXIS/ANTWIN/`, ou em outra pasta indicada por `--jogo` ou pela variável `SIMANT_DIR`.

## Rodar

```
cargo run --release -p simant                              # abertura e "Select a Game"
cargo run --release -p simant -- --novo-jogo --tipo completo   # direto num jogo novo (tutorial, rapido, completo, experimental)
cargo run --release -p simant -- --save tests/referencias/saves/QUICK2.ANT   # abre um save
```

## Jogar

Você é a **formiga amarela** de uma colônia preta, contra as vermelhas, no quintal de uma casa. Os modos (escolhidos no "Select a Game"; Enter = Full Game, como no original) são Tutorial, Quick Game, Full Game e Experimental; ver [`telas/abertura.md`](docs/spec/telas/abertura.md), [`telas/quintal.md`](docs/spec/telas/quintal.md) e [`telas/experimental.md`](docs/spec/telas/experimental.md).

- **Mouse:** clique numa célula leva a amarela até lá; **duplo clique** age (ataca, pede comida, pega ou larga, cava). Segurar o botão sobre a amarela abre o menu dela. **Espaço/Insert** clicam onde está o cursor.
- **Teclas da amarela:** **1**/**2** recrutam 5/10 operárias (Ctrl+2: todas), **3**/**4** liberam metade/todas, **0** alarme, **X** ou F9 troca de corpo, **Backspace** cancela, **Home** centraliza.
- **Velocidade e partida:** **Shift+0** pausa, **Shift+1..4** lenta/normal/rápida/ultra, **Ctrl+G** AutoTrack, **Ctrl+S** salva, **Ctrl+A** salva como, **Ctrl+L** carrega um `.ANT`, **Ctrl+N** jogo novo. O `.ANT` gravado é aceito pelo SimAnt original.
- **Vista:** **Ctrl + setas** rolam a janela de edição (as setas sozinhas e PageUp/PageDown também, extensão do port); **Ctrl+8/9/0** ou os botões *Surface*/*Nest* da faixa trocam de plano.
- **Janelas:** **Ctrl+E** edição, **Ctrl+B** Behavior Control, **Ctrl+C** Caste Control, **Ctrl+H** History, **Ctrl+I** Info, **Ctrl+M** mapa (ou o quintal, se aberto), **Ctrl+Y/5/6/7** quintal nos modos quintal, casa, estratégico e gráfico. No quintal, duplo clique num patch troca de patch (Full Game). Gerenciador: **Ctrl+F4** fecha, **Ctrl+F6** próxima, **Ctrl+Espaço** menu de sistema, **Ctrl+Z** maximiza a edição.
- **Faixa:** os botões configuráveis da segunda linha (Ctrl+clique troca a função), os marcadores 1–7 (Ctrl+clique guarda a vista), as barras de saúde (clique abre o quadro das castas) e, no Experimental, o menu de ferramentas.
- **Ajuda:** **F1** abre o `SIMANT.HLP`; **Shift+F1**, **Delete**, o clique direito ou os botões *?* ligam o modo de ajuda (o próximo clique abre o tópico do que foi clicado).
- **Códigos de trapaça do original:** digite 4 letras na partida (`heal`, `just`, `qeen`, `eggs`...); lista em [`regras/trapacas.md`](docs/spec/regras/trapacas.md).

Todas as teclas vêm de `data/partida.json`, `data/navegacao.json`, `data/janelas.json`, `data/quintal_mapa.json` e `data/ajuda_contexto.json`. O que ainda falta para quem joga (barra de menus do Windows, balões de fala, sons de menus) está em [`docs/estado-do-projeto.md`](docs/estado-do-projeto.md) §4.

## Opções de linha de comando

| Opção | Efeito |
|---|---|
| `--jogo DIR`, `--dados DIR`, `--mods DIR` | pastas do jogo (padrão `SIMANT_DIR` ou `SimAnt-T/MAXIS/ANTWIN`), dos dados (`data`) e dos mods (`mods`) |
| `--novo-jogo`, `--tipo T`, `--semente N` | jogo novo do tipo `tutorial`, `rapido` (padrão), `completo` ou `experimental`, com semente fixa (reproduzível) |
| `--save ARQ.ANT`, `--salvar ARQ.ANT` | abre um save; `--salvar` grava o estado no fim de uma execução sem janela e, na janela, é o nome do "salvar como" (Ctrl+A; sem ele, `SALVO01.ANT`, `SALVO02.ANT`...) |
| `--resolucao LxA` | tela virtual do Windows 3.1: `640x480` (padrão), `800x600` ou `1024x768`; muda o layout como o original (janelas na tabela de posições da resolução, mais área livre em volta), sem escalar nada ([`telas/resolucao.md`](docs/spec/telas/resolucao.md)) |
| `--escala N`, `--janela LxA` | ampliação da tela virtual (inteira, nítida) e tamanho inicial da janela; `0` = a maior que cabe |
| `--sem-som`, `--soundfont ARQ.sf2` | sem áudio; SoundFont General MIDI para as músicas (ou `SIMANT_SOUNDFONT`; sem ela, sintetizador simples) |
| `--preferencias ARQ`, `--sem-preferencias` | arquivo das opções guardadas entre partidas (no lugar do `[SimAnt]` do `WIN.INI`; padrão em `%APPDATA%\SimAnt recomp\SimAnt\`) |
| `--ticks N`, `--avancar-ms MS`, `--acoes A,B`, `--quintal M`, `--janela-mapa M`, `--mini-mapa N` | testes sem interação: roda ticks, passa tempo real, aplica ações de navegação, abre o quintal, o mapa num modo ou o mini mapa (N aberturas) |
| `--exportar-tela-jogo ARQ.png`, `--exportar-telas DIR`, `--exportar-som DIR`, `--capturar-janela ARQ.png`, `--sair-apos-ms N` | grava a tela de jogo, as telas da abertura, os efeitos e músicas em WAV, ou o que a janela mostrava |

### Opções de vídeo (`data/video.json`)

```json
{"resolucao_virtual": [640, 480], "escala": 1, "janela": null, "redimensionavel": true, "escala_inteira": true}
```

- `resolucao_virtual`: a resolução do Windows 3.1 que o port imita. O SimAnt original não amplia nada: numa tela maior, sobra mais área em volta e algumas janelas (título, Caste/Behavior, Info, fim de jogo, quintal) mudam de lugar, pela tabela de posições que o próprio jogo escolhe pela altura da tela (até 480 linhas uma, acima outra). `[640, 480]` é o padrão (a bancada); `[800, 600]` é a do DOSBox-X de referência; `[1024, 768]` usa as mesmas posições do 800×600 com mais área livre. A regra fica em `data/resolucao.json`.
- `escala`, `janela`, `redimensionavel`, `escala_inteira`: como a tela virtual aparece no monitor — ampliada por múltiplos inteiros (pixels nítidos, tarjas em volta) ou, com `escala_inteira: false`, ocupando o máximo com a proporção.
- A linha de comando (`--resolucao`, `--escala`, `--janela`) vence o arquivo; um mod pode trazer o seu `video.json`.

A lista completa, com as opções de depuração, está no comentário do topo de `simant/src/main.rs` e em `simant/src/opcoes.rs`.

## Mods

Um mod é uma pasta em `mods/` com um `mod.json` (`nome`, `versao`, `dependencias`, `scripts`). Ele pode trocar objetos do banco original (`objetos/<tipo>/<id>.bin`), arquivos de dados do port (`data/<nome>.json`), arquivos soltos do jogo (`arquivos/<nome>`) e trazer scripts Lua em sandbox com ganchos (`ao_iniciar`, `depois_do_tick`, `ao_evento`, `ao_menu`...). A API, a lista de arquivos sobrescrevíveis e o histórico de compatibilidade estão em [`docs/api-mods.md`](docs/api-mods.md) (versão atual no topo).

Exemplos em `exemplos/mods/` (fora de `mods/`, para não mudar o jogo por padrão): `exemplo-lua/` (chuva de comida a cada 500 ticks) e `trapacas/` (menu **F12** para provocar chuva, revoada, menino, gato, fim de jogo, avançar ticks, painel de depuração). Para ligar: `--mods exemplos/mods`.

## Testes

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Os testes com arquivos reais procuram o jogo em `SIMANT_DIR` ou em `SimAnt-T/MAXIS/ANTWIN` e avisam e passam sem eles. Os vetores do original (`tests/referencias/`, ver o [`README`](tests/referencias/README.md) de lá e [`regras/testes-tick.md`](docs/spec/regras/testes-tick.md)) exigem igualdade bit a bit do estado depois de cada tick (`engine/tests/ticks_originais.rs`, `ticks_longos.rs`). A bancada que roda o original no QEMU está em [`docs/bancada-qemu.md`](docs/bancada-qemu.md); os extratores das tabelas do executável estão em `tools/` (`tools/conferir_extracoes.py SIMANTW.EXE` confere que reproduzem os `data/*.json`).

## Organização

| Pasta | Conteúdo |
|---|---|
| `engine/` | crate sem janela: `formats` (formatos originais), `rules` (simulação), `core` (telas, janelas, partida), `recursos` (original + mods), `scripts` (Lua) |
| `simant/` | executável SDL2 (vídeo, áudio, entrada) |
| `data/` | dados do port (tabelas das regras, telas, teclas, textos): é o que os mods sobrescrevem |
| `exemplos/mods/` | mods de exemplo |
| `tools/` | extratores e inspetores em Python, bancada QEMU, comparação de capturas |
| `tests/referencias/` | vetores, casos e roteiros gravados do original |
| `referencia/` | configuração do DOSBox-X para rodar o original |
| `docs/` | especificação, decisões, glossário, divergências, API de mods, estado do projeto |
