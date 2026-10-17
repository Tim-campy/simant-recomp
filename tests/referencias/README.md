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
| Tamanho do save | salvar qualquer jogo | arquivo `.ANT` de 48.386 bytes | `QUICK2.ANT` (Quick Game, salvo na bancada QEMU) tem 48.386 bytes | confirmado (`saves/QUICK2.ANT`) |
| Estado da memória = save | bancada QEMU, `roteiros/qemu_estado_save.txt` (salva `QUICK3.ANT` e grava `estado` com a VM pausada) | os 307 blocos lidos da memória formam um arquivo igual ao `.ANT` | idênticos nos 48.386 bytes (`tools/estado_diff.py`); 10 s depois, 70 blocos mudam (`GameTime` 92 → 154) | confirmado (`saves/QUICK3.ANT`, `docs/bancada-qemu.md`) |
| Paleta | captura de qualquer tela colorida | cores = paleta padrão do Windows (`data/paleta.json`) | igual, com 7 = cinza escuro e 8 = cinza claro (a especificação tinha trocado) | confirmado, divergência corrigida |
| Título pixel a pixel | `capturar_original.ps1 -Saida capturas\titulo_800x600.png`; `comparar_captura.py` contra `--exportar-telas` | port igual ao original | 99,98% iguais; diferença só sob o cursor do mouse | confirmado (`engine/tests/telas_reais.rs`) |

## Casos de teste de regras (bancada QEMU)

Cada caso é um JSON em `casos/` (save de partida, patch de `tools/ant_editar.py`, tick alvo, resultado esperado e conferências automáticas). Para rodar: `python tools/caso_ant.py rodar tests/referencias/casos/ID.json` (com `SIMANT_RAIZ` apontando para a raiz com `referencia/` e `re_trabalho/`). Injeção, passos do Load e o que o load muda: [docs/bancada-qemu.md](../../docs/bancada-qemu.md#casos-de-teste-injetar-um-save-e-rodar). O estado final de cada caso observado fica em `saves/` para comparação com o port.

| Caso | Regra | Entrada (patch) | Esperado (pela especificação) | Observado no original | Situação |
|---|---|---|---|---|---|
| C-01 (`casos/C-01.json`) | avanço dos ovos com P = 128 ([ciclo-de-vida.md](../../docs/spec/regras/ciclo-de-vida.md#31-ninho-preto-_simeggb-10283ca0)) | `QUICK2.ANT` sem mudanças (tick 78) | no tick 128, todos os ovos pretos de T = 1 a 2 e os vermelhos de 0x81 a 0x82; o posto no próprio tick 128 fica no estágio 1 | 2026-10-16, parado no tick 128: ovos pretos 2–6 com T = 2 e o 7 (posto no 128, em (36, 9)) com T = 1; vermelhos 2–8 com 0x82 e o 9 com 0x81. 9 de 9 conferências. Estado: `saves/C01_T128.ANT` | confirmado |
| C-02 (`casos/C-02.json`) | período curto P = 32 e eclosão (ciclo-de-vida 3.1) | `QUICK2.ANT`; remove a operária preta da lista A (para `_BpopT` = 2 depois do load); ovo B[2] com T = 7 | no tick 96, B[2] vira `_MakeMe` × 8 + 2 = 0x32 (soldado), M ∈ {1, 2}, S = 0; os outros ovos pretos passam a T = 2; os vermelhos (P = 128) ficam em 0x81 | 2026-10-16, parado no tick 96: `_BpopT` = 2; B[2] = (34, 9) T 0x32 M 2 S 0; ovos B[3..5] T = 2, B[6] (posto no 96) T = 1; vermelhos todos 0x81. 10 de 10 conferências. Numa execução que parou no tick 98, o soldado já estava em (32, 7) com T 0x37 (andou 2 células na diagonal para cima e à esquerda, com M = 2). Estado: `saves/C02_T96.ANT` | confirmado |
| SUP-01 (`casos/SUP-01.json`) | carregadora de comida sobre gradiente de ninho ([superficie.md](../../docs/spec/regras/superficie.md) 3.5, P2) | `QUICK2.ANT`; aranha desligada; chão livre em volta da linha y = 40; `_PherMapBN` crescente para oeste na linha cy = 20; formiga A (40, 40) M 3 T 0x1E S 200 | X = 40 − n, S = 200 − n; trilha preta com o S do 1º tile de cada célula | 2026-10-17, tick 94: (24, 40) T 0x1E M 3 S 184; trilha igual à do port. Estados: `saves/SUP01_CARREGADO.ANT`, `saves/SUP01_FIM.ANT` | confirmado (`sup01_carregadora_igual_ao_original`) |
| SUP-02 (`casos/SUP-02.json`) | `_GoInNest` (superficie.md 7) | operária preta no buraco preto (3, 32), M 0, S 55 | sai da lista A; entra na B em (32, 1) com T 0x14, M e S mantidos | 2026-10-17, tick 85: A[2].T = 0; B[5] grupo 2, S 55 (já andou no ninho); entrada escavada. `saves/SUP02_*.ANT` | confirmado (`sup02_...`) |
| SUP-03 (`casos/SUP-03.json`) | líder vermelha (superficie.md 9.2) | 30 pretas paradas (M 17) para `_BpopT` ≥ 30 e uma vermelha parada em (100, 30) no fim da lista A | líder T 0xB0 M 19; destino (59, 38); caminho sem sorteio | 2026-10-17, tick 90: líder (89, 38) T 0xB6 M 19, `_RedTask` 1, `_RedInitOn` 1; igual ao port. `saves/SUP03_*.ANT` | confirmado (`sup03_...`) |
| SUP-04 (`casos/SUP-04.json`) | etapas de feromônio do tick ([feromonios.md](../../docs/spec/regras/feromonios.md) §3) e P7 | trilha vermelha 210 em (40, 5), preta 210 em (45, 5), alarme 200 em (50, 25); formiga M 17 em (20, 10) | RT 105 (tick 96); BT 210; alarme 100/25 (tick 128); RN do buraco 254; formiga parada | 2026-10-17, tick 130: 11 de 11 conferências; igual ao port. `saves/SUP04_*.ANT` | confirmado (`sup04_...`) |

Vetores de tick do original (`ticks/`, [testes-tick.md](../../docs/spec/regras/testes-tick.md)): a superfície do port (lista A, vida e mapas de cheiro) é conferida contra os ticks 79–83 em `engine/tests/regras_superficie.rs` (`vetores_de_tick_do_original`).

## Como capturar e comparar

```
powershell -ExecutionPolicy Bypass -File tools\capturar_original.ps1 -Saida tests\referencias\capturas\NOME.png [-Espera 28] [-Cliques 'x,y;x,y']
cargo run -p simant -- --exportar-telas re_trabalho\telas_port
python tools\comparar_captura.py re_trabalho\telas_port\TELA.png tests\referencias\capturas\NOME.png --diff re_trabalho\diff.png
```

Atenção: o script traz a janela do DOSBox-X para a frente e, com `-Cliques`, move o mouse real. O cursor do Windows 3.1 dentro do emulador se move de forma relativa, então as coordenadas de clique não são confiáveis. Prefira esperar os tempos do jogo ou usar o teclado. A captura é feita em 1:1 (`windowresolution = original` na configuração). A área cliente do jogo começa na linha 38 da captura. As capturas contêm imagens do jogo e **não são versionadas** (ficam no `.gitignore`).

Quando um caso for observado: guarde a captura em `capturas/`, preencha a coluna "Observado" e, se houver diferença, registre em `docs/divergencias.md`.
