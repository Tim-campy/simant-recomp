# Referências do original

Capturas e logs feitos no SimAnt original (DOSBox-X, `referencia/dosbox-x.conf`), usados para comparar com o port.

## Casos pendentes de observação

| Caso | Entrada | Esperado (pela especificação) | Observado no original | Situação |
|---|---|---|---|---|
| Duração da abertura | iniciar o jogo e não tocar em nada | a janela de abertura fecha depois de ~39,6 s (720 ticks de 55 ms) | — | pendente |
| Abertura fecha com espaço | apertar Espaço durante a abertura | fecha imediatamente | — | pendente |
| Composição da abertura (1ª observação) | captura da tela de abertura | logotipo = bitmaps 2/2000 + 2/2001 empilhados | logotipo aparece **dentro da janela principal** do SimAnt (menu File/Window/View/Options/Speed e barra de ferramentas visíveis acima), com moldura azul; não há tela Maxis antes (o jogo foi lançado direto por `win SIMANTW.EXE`). Captura: `capturas/abertura_titulo_dosbox.png` (2026-10-16, ~30 s após iniciar) | divergência registrada |
| Ordem das telas | iniciar o jogo e esperar 80 s | Maxis Presents → título → Select a Game [hipótese] | título → registro (Customer ID) → Select a Game; nenhuma tela Maxis. Capturas `titulo_800x600.png`, `apos_titulo.png`, `novo_jogo.png` | confirmado, hipótese corrigida |
| Fontes Mac pixel a pixel | renderizar "Click here to continue" (font2) e "Customer ID:" (font3) e procurar na captura do registro | blocos idênticos | encontrados exatos em (362,397) e (361,356) de `apos_titulo.png` | confirmado (`fontes_mac_batem_com_o_original`) |
| Select a Game pixel a pixel | `capturar_original.ps1 -Espera 80` | port igual ao original | 99,87% iguais; diferem o cursor e os botões configuráveis da faixa | confirmado (`engine/tests/telas_reais.rs`) |
| Tamanho do save | salvar qualquer jogo | arquivo `.ANT` de 48.386 bytes | — | pendente |
| Paleta | captura de qualquer tela colorida | cores = paleta padrão do Windows (`data/paleta.json`) | igual, com 7 = cinza escuro e 8 = cinza claro (a especificação tinha trocado) | confirmado, divergência corrigida |
| Título pixel a pixel | `capturar_original.ps1 -Saida capturas\titulo_800x600.png`; `comparar_captura.py` contra `--exportar-telas` | port igual ao original | 99,98% iguais; diferença só sob o cursor do mouse | confirmado (`engine/tests/telas_reais.rs`) |

## Como capturar e comparar

```
powershell -ExecutionPolicy Bypass -File tools\capturar_original.ps1 -Saida tests\referencias\capturas\NOME.png [-Espera 28] [-Cliques 'x,y;x,y']
cargo run -p simant -- --exportar-telas re_trabalho\telas_port
python tools\comparar_captura.py re_trabalho\telas_port\TELA.png tests\referencias\capturas\NOME.png --diff re_trabalho\diff.png
```

Atenção: o script traz a janela do DOSBox-X para a frente e, com `-Cliques`, move o mouse real. O cursor do Windows 3.1 dentro do emulador se move de forma relativa, então as coordenadas de clique não são confiáveis. Prefira esperar os tempos do jogo ou usar o teclado. A captura é feita em 1:1 (`windowresolution = original` na configuração). A área cliente do jogo começa na linha 38 da captura. As capturas contêm imagens do jogo e **não são versionadas** (ficam no `.gitignore`).

Quando um caso for observado: guarde a captura em `capturas/`, preencha a coluna "Observado" e, se houver diferença, registre em `docs/divergencias.md`.
