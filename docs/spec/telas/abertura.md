# Tela de abertura

## Comportamento

**Fonte:** fato visto no código, `_ShowIntro` (`1000:d3e2`).

1. Abre a janela interna `0x300` (768) do sistema de janelas próprio (`_win_Open`) e captura o mouse.
2. Toca a música de id 10001 (`_myBeginSong(0x2711, 0x7e)`; o segundo parâmetro tem significado [em aberto]). No banco `SOUND`, o id 10001 existe como cabeçalho (tipo 18) e como MIDI (tipos 20 e 50).
3. Monta um número de 8 dígitos a partir de uma tabela do DGROUP [hipótese: número de série exibido na tela; não é usado neste marco].
4. Espera até **40 × 18 = 720 ticks** (`_DialogWaitInit(0x28)`). O tick de `_TickCount` (`1008:18ac`) é `GetTickCount() / 55`, ou seja, cerca de 18,2 por segundo, o que dá cerca de **39,6 s**.
5. Fecha antes do tempo se `_win_Events()` devolver um evento (clique ou tecla tratada pela janela) ou se a **barra de espaço** estiver pressionada (`GetAsyncKeyState(VK_SPACE)` em `_DialogAbortOrCont`, `1000:9c98`).
6. Fecha a janela `0x300` e descarta os eventos pendentes.

## Conteúdo visual

- A janela `0x300` é a janela 3 (`titleWin`), com os objetos `titlePic1` (bitmap 2000) e `titlePic2` (bitmap 2001). Fato visto nos dados; ver [../formatos/janela.md](../formatos/janela.md).
- **Observado no original (DOSBox-X, 2026-10-16):** ao iniciar `SIMANTW.EXE`, a área cliente mostra a faixa de ferramentas (janela 34) e a janela de título com borda, sem nenhuma tela "Maxis Presents" antes. O port reproduz isso com 99,98% dos pixels iguais (a diferença é o cursor). Captura: `tests/referencias/capturas/titulo_800x600.png`.
- **Em aberto:** onde entram os bitmaps "Maxis Presents" (2/28000, 2/28003).

## Sequência de inicialização

**Fonte:** `WINMAIN` (`1000:3fda`) chama `_ShowIntro` → `_CustomerIDDialog` → `_NewGame(1)`. Observado no DOSBox-X (2026-10-16), deixando os tempos esgotarem sem tocar em nada (`tools/capturar_original.ps1 -Espera 80`):

1. **Título** (janela 3): até 720 ticks (≈ 39,6 s), ou espaço, ou clique.
2. **Registro** (janela 30, `picStrWin`, via `_PictureDialog`): "This copy of SimAnt is registered to: [nome omitido] / Customer ID: [número de cliente omitido] / Click here to continue". Fecha depois de 15 × 18 = 270 ticks (≈ 14,9 s), com espaço ou com clique. O nome vem do texto 10/0x83 e de uma string decodificada por `_DecodeString`; o número de 8 dígitos é o que `_ShowIntro` monta (é o *Customer ID*). Captura: `apos_titulo.png`. **No port, os textos ainda não são desenhados** (falta especificar as fontes).
3. **Select a Game** (janela 2, `scenarioWin`, via `_DoScenario`): fica aberta até haver escolha. Captura: `novo_jogo.png`. O port reproduz com 99,87% de pixels iguais.

Os tempos são aproximados porque o tick é de 55 ms. Nos testes, a soma bate: o "Select a Game" já estava aberto 80 s depois de iniciar o DOSBox-X.

## No port (marco 1)

A sequência fica em `data/telas.json` (dados, não código): para cada tela, as janelas do jogo (e, opcionalmente, bitmaps soltos), a duração em ticks e os gatilhos que a encerram. Hoje só contém a abertura validada. Um mod pode trocar bitmaps ou layouts por (tipo, id), ou mudar a sequência.
