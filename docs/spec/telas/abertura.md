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

- **Observado no original (DOSBox-X, 2026-10-16):** a janela `0x300` aparece dentro da janela principal do SimAnt (menu e barra de ferramentas visíveis acima), com moldura azul em volta do logotipo. Ver `tests/referencias/capturas/abertura_titulo_dosbox.png` e `docs/divergencias.md`.
- **Em aberto:** a composição exata da janela `0x300` vem dos objetos de janela (tipos 0 e 1 do banco), cujo formato ainda não foi decodificado.
- Bitmaps que provavelmente compõem a abertura [hipótese, pela aparência]: 2/2000 (metade de cima do logotipo SIMANT, 512×168) e 2/2001 (metade de baixo, 512×174, com "© 1991 MAXIS"). Juntos dão 512×342.
- "Maxis Presents" (2/28000 e 2/28003, 236×197) e "Select a Game" (2/2500, 318×259) existem no banco. A ordem em que aparecem em relação à abertura ainda não foi observada [em aberto: confirmar no DOSBox-X].

## No port (marco 1)

A sequência de telas fica em `data/telas.json` (dados, não código): para cada tela, os bitmaps com a posição de cada um, a duração em ticks e as teclas que encerram. A sequência inicial é uma [hipótese] para validar contra o original. Um mod pode trocar bitmaps ou mudar a sequência.
