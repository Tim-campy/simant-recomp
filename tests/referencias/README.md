# Referências do original

Capturas e logs feitos no SimAnt original (DOSBox-X, `referencia/dosbox-x.conf`), usados para comparar com o port.

## Casos pendentes de observação

| Caso | Entrada | Esperado (pela especificação) | Observado no original | Situação |
|---|---|---|---|---|
| Duração da abertura | iniciar o jogo e não tocar em nada | a janela de abertura fecha depois de ~39,6 s (720 ticks de 55 ms) | — | pendente |
| Abertura fecha com espaço | apertar Espaço durante a abertura | fecha imediatamente | — | pendente |
| Composição da abertura | captura da tela de abertura | logotipo = bitmaps 2/2000 + 2/2001 empilhados | logotipo aparece **dentro da janela principal** do SimAnt (menu File/Window/View/Options/Speed e barra de ferramentas visíveis acima), com moldura azul; não há tela Maxis antes (o jogo foi lançado direto por `win SIMANTW.EXE`). Captura: `capturas/abertura_titulo_dosbox.png` (2026-10-16, ~30 s após iniciar) | divergência registrada |
| Ordem das telas | iniciar o jogo | Maxis Presents → título → Select a Game [hipótese] | — | pendente |
| Tamanho do save | salvar qualquer jogo | arquivo `.ANT` de 48.386 bytes | — | pendente |
| Paleta | captura de qualquer tela colorida | cores = paleta padrão do Windows (`data/paleta.json`) | cores do logotipo e da barra de ferramentas coincidem visualmente | parcialmente confirmado (só comparação visual) |

Quando um caso for observado: guarde a captura em `capturas/`, preencha a coluna "Observado" e, se houver diferença, registre em `docs/divergencias.md`.
