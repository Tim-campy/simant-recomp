# Tela de abertura

## Comportamento

**Fonte:** fato visto no código, `_ShowIntro` (`1000:d3e2`).

1. Abre a janela interna `0x300` (768) do sistema de janelas próprio (`_win_Open`) e captura o mouse.
2. Toca a música de id 10001 (`_myBeginSong(0x2711, 0x7e)`; o segundo parâmetro tem significado [em aberto]). No banco `SOUND`, o id 10001 existe como cabeçalho (tipo 18) e como MIDI (tipos 20 e 50).
3. Monta um número de 8 dígitos a partir de uma tabela do DGROUP [hipótese: número de série exibido na tela; não é usado neste marco].
4. Espera até **40 × 18 = 720 ticks** (`_DialogWaitInit(0x28)`). O tick de `_TickCount` (`1008:18ac`) é `GetTickCount() / 55`, ou seja, cerca de 18,2 por segundo, o que dá cerca de **39,6 s**. **Verificado (2026-10-18, `tools/medir_abertura.py`):** a pilha de esperas (`DGROUP:150a`, início em `cad4`, duração em `caf4`) mostra o título armado em `_TickCount` 1013 com 720 e o registro em 1733 = 1013 + 720 com 270; na VM, 719 e 270 ticks do BIOS. O logotipo é desenhado antes de a espera começar. Espaço fecha o título e o registro em ~0,25 s; apertado antes de a espera começar, se perde. O tempo no relógio do host não serve (o QEMU sem `icount` perde ticks do Windows com a máquina carregada).
5. Fecha antes do tempo se `_win_Events()` devolver um evento (clique ou tecla tratada pela janela) ou se a **barra de espaço** estiver pressionada (`GetAsyncKeyState(VK_SPACE)` em `_DialogAbortOrCont`, `1000:9c98`).
6. Fecha a janela `0x300` e descarta os eventos pendentes.

## Conteúdo visual

- A janela `0x300` é a janela 3 (`titleWin`), com os objetos `titlePic1` (bitmap 2000) e `titlePic2` (bitmap 2001). Fato visto nos dados; ver [../formatos/janela.md](../formatos/janela.md).
- **Observado no original (DOSBox-X, 2026-10-16):** ao iniciar `SIMANTW.EXE`, a área cliente mostra a faixa de ferramentas (janela 34) e a janela de título com borda, sem nenhuma tela "Maxis Presents" antes. O port reproduz isso com 99,98% dos pixels iguais (a diferença é o cursor). Captura: `tests/referencias/capturas/titulo_800x600.png`.
- **Em aberto:** onde entram os bitmaps "Maxis Presents" (2/28000, 2/28003).

## Sequência de inicialização

**Fonte:** `WINMAIN` (`1000:3fda`) chama `_ShowIntro` → `_CustomerIDDialog` → `_NewGame(1)`. Observado no DOSBox-X (2026-10-16), deixando os tempos esgotarem sem tocar em nada (`tools/capturar_original.ps1 -Espera 80`):

1. **Título** (janela 3): até 720 ticks (≈ 39,6 s), ou espaço, ou clique.
2. **Registro** (janela 30, `picStrWin`, via `_PictureDialog`): "This copy of SimAnt is registered to: [nome omitido] / Customer ID: [número de cliente omitido] / Click here to continue". Fecha depois de 15 × 18 = 270 ticks (≈ 14,9 s), com espaço ou com clique. Os textos fixos vêm de 10/131 ("This copy of SimAnt is registered to:") e 10/132 ("Customer ID:"); o nome vem de uma string decodificada por `_DecodeString` (DGROUP `0x8b3e`); o número de 8 dígitos é o que `_ShowIntro` monta. Fontes: `font3` (azul) e `font2` ("Click here to continue"), validadas pixel a pixel (ver [../formatos/fonte.md](../formatos/fonte.md)). Captura: `apos_titulo.png`. **Fora do port por enquanto:** o tamanho da janela e a posição dos textos vêm de `_PictureDialog`/`_win_DrawPictureWindow`, ainda não especificados.
3. **Select a Game** (janela 2, `scenarioWin`, via `_DoScenario`): fica aberta até haver escolha. Captura: `novo_jogo.png`. O port reproduz com 99,87% de pixels iguais.

Os tempos são aproximados porque o tick é de 55 ms. Nos testes, a soma bate: o "Select a Game" já estava aberto 80 s depois de iniciar o DOSBox-X.

## Escolha do jogo (`_DoScenario` `1000:5998`, `_NewGame` `1000:d6dc`)

**Fonte:** pseudocódigo [código], nomes do objeto 1/2 do `SHARED` [dados] e cliques na bancada QEMU [verificado].

- `_DoScenario` abre a janela 2, espera um evento de objeto e devolve o id do objeto; `GetAsyncKeyState(VK_RETURN)` devolve 0x203 (Full Game); `_DialogAbort` devolve 0x205 [código].
- `_NewGame` converte: `scenTutorial` 0x206 → `_CurGameType` 0, `scenQuickGame` 0x202 → 1, `scenFullGame` 0x203 → 2, `scenExperimental` 0x204 → 3; `scenLoad` 0x207 → `_LoadGame` (se falhar, mostra a caixa de novo); `scenCancel` 0x205 → devolve −1 sem mudar nada [código]. Depois, `_RandYard` e as janelas do jogo ([../regras/geracao-mundo.md](../regras/geracao-mundo.md) §1–2).
- O mesmo diálogo volta durante a partida por File → New Game (**Ctrl+N**, `_ProcMenu` caso 3) e ao fechar o diálogo de fim de jogo (`_EndGameDialog` → `_NewGame(0)`), com a simulação pausada (`_SetPause(1)`) [código].
- Na bancada (640×480): Quick Game em (345, 228), Tutorial em (185, 228), Full Game em (280, 290) na tela; o Full Game começa na superfície com a rainha alada e a faixa diz "You need to dig a nest" (`re_trabalho/modos/full_inicio.png`) [verificado].

**No port:** `engine/src/core/selecao.rs` (`SelecaoDeJogo`: objeto sob o clique → `Escolha`), dados em `data/modos.json` (objeto → escolha, Enter → Full Game, tecla do New Game). Na abertura, a tela `selecao` de `telas.json` não fecha mais com qualquer clique: só um botão (ou Enter) escolhe, e o jogo do tipo escolhido começa (`simant/src/modos.rs`). Na partida, Ctrl+N desenha a janela 2 por cima da tela de jogo (posição da tabela 9/0) e pausa a simulação até a escolha. `--tipo tutorial|rapido|completo|experimental` começa direto um jogo do tipo. Load Game ainda não existe (aviso no terminal) e Cancel na abertura fecha o port (divergencias.md).

**Verificado:** o teste `engine/tests/modos_reais.rs` confere que, na tabela de posições 0, os pontos clicados na bancada caem nos botões certos (Quick, Tutorial, Full, Experimental, Load) e que o centro de cada botão devolve a própria escolha.

## No port (marco 1)

A sequência fica em `data/telas.json` (dados, não código): para cada tela, as janelas do jogo (e, opcionalmente, bitmaps soltos), a duração em ticks e os gatilhos que a encerram. Hoje só contém a abertura validada. Um mod pode trocar bitmaps ou layouts por (tipo, id), ou mudar a sequência.
