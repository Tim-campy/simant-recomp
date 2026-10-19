# Testes tick a tick contra o original (determinismo e vetores)

Este documento descreve como comparar a simulação do port com o SimAnt original **um tick por vez**: os experimentos de determinismo feitos na bancada QEMU, o protocolo de captura de vetores "estado antes → estado depois" e o que um vetor precisa conter além do layout do save. Bancada: [../../bancada-qemu.md](../../bancada-qemu.md). Estrutura do tick: [simulacao-visao-geral.md](simulacao-visao-geral.md). Geradores aleatórios: [rng.md](rng.md).

Convenção de evidência como no resto de `docs/spec/`: **verificado** = medido na bancada (data e comando indicados); **visto no código** = pseudocódigo do Ghidra em `re_trabalho/decomp/`; **[hipótese]** = não medido.

## Resumo

- **É possível parar exatamente no início e no fim de um tick e avançar um tick por vez** (verificado, 2026-10-16). Breakpoints do gdbstub do QEMU na entrada de `_DoAntSim` (`1028:0000`) e no endereço de retorno dele dentro do `MYTIMERFUNC` (`1000:2688`).
- **O original é determinístico tick a tick na bancada** (verificado): partindo do snapshot `quick`, duas execuções chegam ao `GameTime` 300 com os 307 blocos do save idênticos, **exceto `_BirdDelay`**, com ou sem sementes forçadas, e mesmo quando a VM roda 20 s livre (sem breakpoints) no meio do caminho.
- `_BirdDelay` diverge porque `_SimBird` o calcula a partir de `_gameCycles`, um contador do laço ocupado do `MYTIMERFUNC` que depende do relógio do host. É a única entrada não determinística encontrada dentro do tick.
- Entre dois ticks (desenho, mensagens, laço do temporizador) **nada do save muda e os geradores S e R não andam** na configuração do snapshot `quick` (verificado em 139 intervalos). Só mudam `_gameCycles`, contadores de redesenho e a pilha.
- Para reproduzir um tick, o port precisa, além dos 307 blocos do save: estado do gerador S, estado do gerador R, `_gameCycles`, `_LifeA/B/R` (ou reconstruí-los das listas) e `_GlobalKey`. Lista completa e justificativa na seção "Pergunta 3".

## Ferramentas

| Ferramenta | O que faz |
|---|---|
| `tools/qemu_bancada.py`, comandos `tick_entrada`, `tick_saida`, `ate_gametime N`, `passo_tick DIR`, `extras ARQ`, `segmentos PREFIXO` | modo tick (ver [bancada-qemu.md](../../bancada-qemu.md#modo-tick-breakpoints-do-gdbstub)) |
| `tools/gerar_vetores_tick.py` | grava N ticks consecutivos em `tick_NNNNNN/` (ver "Protocolo") e mapeia leituras fora do save (`--mapear-leituras`) |
| `tools/comparar_ticks.py A B [--detalhe]` | compara dois diretórios de vetores, tick a tick |
| `tools/comparar_ticks.py --continuidade DIR` | o que muda entre o retorno do tick N e a entrada do N+1 |
| `tools/comparar_ticks.py --fora-do-save DIR...` | bytes dos segmentos 8, 9 e 10 que o tick escreve fora do layout do save (exige `--segmentos`) |
| `tests/referencias/roteiros/qemu_tick_passos.txt` | 3 ticks com `passo_tick` |
| `tests/referencias/roteiros/qemu_det_gametime.txt`, `qemu_det_livre.txt` | experimento da pergunta 1, por roteiro |

Todos exigem `SIMANT_RAIZ` apontando para a raiz com `referencia/qemu` e `re_trabalho/qemu` (com o snapshot `quick` gravado no overlay).

## Pergunta 2: parar no início e no fim de um tick

**Técnica (verificada).**

1. **Seletor do segmento de código.** Os segmentos de código não aparecem na tabela do save, então o método dos seletores de dados não serve. `Memoria.seletor_codigo(n)` lê a LDT inteira (base e limite de `info registers`), filtra os descritores de **código presentes** com limite compatível com o tamanho do segmento no cabeçalho NE (`SIMANTW.EXE`, segmento 6 = `1028`, 0x9AF4 bytes) e compara os primeiros 64 bytes da memória com os do arquivo (as relocações mudam poucos bytes). Observado (2026-10-16, `-m 16`): segmento 6 = seletor `0x1547`, base linear `0x80837500`, 58/64 bytes iguais. Todos os segmentos: 1 = `0x156f`, 2 = `0x1567`, 3 = `0x155f`, 4 = `0x1557`, 5 = `0x154f`, 6 = `0x1547`, 7 = `0x153f`, 8 = `0x1537`, 9 = `0x152f`, 10 = `0x1527` (decrescem de 8 em 8 na ordem do NE, mas o script sempre procura de novo).
2. **Breakpoint em endereço linear.** O gdbstub do QEMU compara o breakpoint com o PC **linear** (base do CS + EIP). Entrada de `_DoAntSim` = base do seletor `0x1547` + 0. Confirmado: a cada parada o script confere em `info registers` que base do CS + EIP é o endereço pedido (senão, aborta), e as leituras atribuídas ao CS `0x1547` caem em offsets de funções do `1028` (ex.: `1028:04fa`, dentro de `_DoAntSimA`).
3. **Fim do tick.** Na entrada, o topo da pilha (SS:SP) tem o retorno far (IP, CS) de `_DoAntSim`. Observado: `156f:2688` = `1000:2688`, dentro do `MYTIMERFUNC` (`1000:2440`), coerente com o código. Um breakpoint ali para a CPU logo depois do tick, **antes** de `_myServiceSong` e `_UpdateWindows`.
4. **Sem passo por cima de breakpoint.** Cada breakpoint é ligado só durante a corrida até ele e desligado ao parar. Como os dois endereços se alternam, nunca se continua de um breakpoint ativo.
5. **Sessão gdb persistente.** No modo tick a conexão com o gdbstub fica aberta. Com a VM rodando (depois de `continuar`), o gdbstub só aceita o byte de interrupção 0x03; o script o envia antes de qualquer comando. Paradas assíncronas (`T02`, geradas por um `stop` do QMP) são descartadas.

**Definições usadas nos vetores:**

- *estado antes* do tick N: CPU na 1ª instrução de `_DoAntSim`, com `_GameTime = N` (ainda não somado);
- *estado depois*: CPU no retorno de `_DoAntSim`, com `_GameTime = N + 1`.

**Custo medido:** `passo_tick` (estado + mapas de vida + extras, antes e depois) leva ~0,45 s por tick; `ate_gametime` avança ~3,1 ticks/s (221 ticks em 70 s), contra ~6 ticks/s da VM livre.

## Pergunta 1: determinismo

### Experimentos (verificados em 2026-10-16)

Todos partem do snapshot `quick` (`loadvm` com a VM rodando) e param na **primeira** entrada de `_DoAntSim`.

| Id | Procedimento | Resultado |
|---|---|---|
| E0 | primeira entrada em todas as execuções do dia (10 execuções de QEMU independentes) | sempre `GameTime = 79`, S = `0x6b1d`, R = `0x53c15255`, `_gameCycles` = 10379; entre E1a e E1b, o estado e os segmentos 8, 9, 10 inteiros são idênticos nesse ponto. O `loadvm` é reprodutível até o primeiro tick |
| E1 | 2 × 3 ticks com `passo_tick`, sem forçar sementes (`gerar_vetores_tick.py --ticks 3 --segmentos`) | tick 79: tudo idêntico. Ticks 80 e 81: os 307 blocos iguais **exceto `_BirdDelay`**; fora do save, diferem só `_gameCycles` e a pilha |
| E2 | 2 × (sementes forçadas S = `0x1234`, R = 1 na 1ª entrada; tick a tick com breakpoints até `GameTime = 300`; 2 ticks gravados) | iguais exceto `_BirdDelay` (61550 × 59642) e `_gameCycles`. S e R idênticos (`0x1879`, `0x99dad475` na entrada do tick 300) |
| E3 | como E2, mas a VM roda **20 s livre** (sem breakpoints, ~120 ticks) antes do `ate_gametime 300` | iguais entre si exceto `_BirdDelay`/`_gameCycles`; e iguais a E2 também (exceto `_BirdDelay`) |
| E4 | 140 ticks consecutivos (79 → 219) com `--segmentos`, `comparar_ticks.py --continuidade` | em todos os 139 intervalos entre ticks: save igual, S e R iguais. Fora do save mudam só `_gameCycles`, `_UDcntr`, `_UDMapFlip`, buffers de mapa (`_mapMapBuf`, `_mapOutBuf`), contadores do `MYTIMERFUNC` (DGROUP `0x02F0`) e a pilha |

Diretórios de trabalho (fora do git): `re_trabalho/det/{r1,r2,e2a,e2b,e3a,e3b,longo}`.

### Por que `_BirdDelay` diverge

Visto no código (`_SimBird`, `1030:28aa`), com o pássaro ausente (`_BirdOn = 0`):

```
se _gameCycles > _BirdDelay (32 bits):
    _BirdDelay = _gameCycles + 20
    se _CurYardPnt > 4: sorteia SRand2(); se 0 e houver revoada (_SwarmCntB > 0 ou 9:9c26 > 0): o pássaro aparece
```

e, quando o pássaro sai da tela, `_BirdDelay = _gameCycles + 30`. `_gameCycles` (`9:9b2c`) **não é contador de ticks**: o `MYTIMERFUNC` soma 1 a cada volta do laço ocupado, que espera chegar `WM_TIMER` (ver [simulacao-visao-geral.md §2](simulacao-visao-geral.md#2-o-que-mytimerfunc-faz-a-cada-chamada)). Medido: entre dois ticks na velocidade normal, `_gameCycles` anda de ~450 a ~1100, conforme o relógio do host.

Consequências:

- Em velocidade normal (e lenta), `_gameCycles` sempre passa de `_BirdDelay + 20` entre dois ticks, então o teste dá verdadeiro em **todo** tick: o sorteio `SRand2` (se `_CurYardPnt > 4`) acontece sempre e a sequência de S não depende do relógio. Só o **valor** de `_BirdDelay` muda.
- [hipótese] Em "fast"/"ultra" (`_SpeedDelayVals` 0 e −1) há um tick por volta do laço, `_gameCycles` anda 1 por tick e o pássaro passa a ser testado só a cada ~21 ticks. Aí a sequência de S também depende do número de voltas sem tick, que depende do relógio. Não medido.
- Para o port: `_gameCycles` é uma **entrada** do tick (vem em `extras.json`). Um port que use "ticks desde o início" no lugar dele reproduz o comportamento em velocidade normal, mas não o valor de `_BirdDelay`. Na comparação, `_BirdDelay` só bate se o port receber o `_gameCycles` do vetor.

### Desenho, temporizador e mouse

- **Desenho consumindo S:** não observado na configuração do snapshot `quick` (janelas abertas pelo Quick Game, sem mexer em nada): S não anda entre ticks em 139 intervalos. As rotinas que chamam `_SRand*` ao desenhar (`_DrawCurBalloons`, `_DrawSpider`, `_DrawSwarm`, `_Draw_SimYard`, `_DrawRain`, ver [rng.md](rng.md)) não consumiram S nesse período [hipótese: dependem de balões ligados, aranha/enxame/chuva visíveis ou de outras janelas abertas; não testado].
- **Dentro do tick** há leituras de estado de interface (seção seguinte): `_win_IsWinOpen` (`_win_hwnd`), `_mapMessageRemoveTime`, `_editMessage`, fontes e DC de recorte. [hipótese] O tick decide desenhar ou não conforme as janelas abertas; se alguma dessas rotinas sortear S, a sequência dependerá das janelas. Por isso os vetores valem para a **configuração de janelas do snapshot** de onde saíram.
- **Relógio:** nenhuma leitura de relógio dentro do tick foi encontrada além de `_gameCycles`. `_TickCount` só é chamado fora do tick (sementes, mensagens temporárias).
- **Mouse e teclado:** a bancada não envia eventos nos experimentos. `_GlobalKey` (última tecla; 0xA8 liga a chuva em `_DoSimYard`) é lida em todo tick e valeu 0.
- **Pausa e foco:** o jogo não pode estar minimizado (o `MYTIMERFUNC` minimizado chama `SRand1(7)` fora do tick) e não pode estar pausado (o breakpoint nunca chega; o script desiste depois de 120 s).

Conclusão: o caminho por breakpoints é **suficiente para determinismo** na bancada. O modo `-icount` do QEMU (tempo virtual por contagem de instruções) não foi necessário e não foi testado.

## Pergunta 3: o que capturar além do save

### Método (verificado)

1. **Escritas:** com `--segmentos`, o gerador grava os segmentos de dados 8, 9 e 10 inteiros antes e depois de cada tick. `comparar_ticks.py --fora-do-save` lista os bytes alterados pelo tick fora dos 307 blocos do layout, com o símbolo mais próximo do `.SYM`. Rodado sobre 140 ticks (79 → 219, cobre as fases 0–3 de 128 ticks e as tarefas de 16, 32 e 64 ticks).
2. **Leituras:** `gerar_vetores_tick.py --mapear-leituras` põe **watchpoints de leitura** do gdbstub em tudo o que está fora do save nos segmentos 8, 9 e 10 (141 faixas, 80.062 bytes; menos a pilha acima de `_end` e os mapas já conhecidos), cortados nas fronteiras dos símbolos e em pedaços de 64 bytes (o QEMU informa o início do watchpoint atingido, não o byte lido). Cada leitura registra o pedaço, o valor e o CS:IP de quem leu, traduzido para `10x0:offset`. Rodado sobre os mesmos 140 ticks (~3,7 s por tick).

### Escritas do tick fora do save (140 ticks)

| Variável | Endereço | Natureza |
|---|---|---|
| estado de S | `10:cbf2` (2 bytes) | gerador; muda em todo tick |
| estado de R | `10:ae34` (4 bytes) | gerador; muda em todo tick (`RRand(32)` da chuva em `_DoSimYard`) |
| `_LifeA` / `_LifeB` / `_LifeR` | `10:68e8` (8 KB), `10:88e8`, `10:98e8` (4 KB cada) | mapas de ocupação; mudam quando formigas se movem |
| `_TemBModePop`, `_TemRModePop` | `9:786a`, `9:7be4` | temporários: zerados e remontados em todo tick |
| `_TemCensus` | `9:7c4e` | temporário da contagem (`_FullCount`) |
| área acima de `_end` (`10:cf56`) | pilha (SS = DGROUP) | lixo de pilha, não é estado |

### Leituras do tick fora do save (140 ticks)

| Grupo | Variáveis | Valor = arquivo? | Para o port |
|---|---|---|---|
| **Estado de verdade** | `_gameCycles` (`9:9b2c`) | não | entrada do tick (ver `_BirdDelay`) |
| | `_GlobalKey` (`9:7866`) | sim (0) | entrada do tick (tecla) |
| | `_Tindex` (`9:9b6a`) | sim (0) | índice global de `_DoAntSimA/B/R`; lido em `1028:04fa`, no começo de `_DoAntSimA` [hipótese: escrito antes de usado; capturado por segurança] |
| | estado de S e de R | não | entradas |
| | `_LifeA/B/R` | não | não monitorados (já capturados à parte) |
| **Tabelas constantes** | `_Dx8`, `_Dy8`, `_Dx9`, `_Dy9`, `_TurnTab`, `_SpX`, `_SpY`, `_CasteTabB`, `_ModeTabB`, `_BxTab`, `_ByTab`, `_SowTab`; trechos de `10:235a`/`10:259a` (depois de `_commandStr`) | sim | dados do executável; o port os carrega do `SIMANTW.EXE` ou de `data/` |
| **Temporário** | `_TemDensity` (`8:4ad2`, 2 KB, lido em 2 dos 140 ticks por `1028:93e2`) | sim (zeros) | [hipótese] área de trabalho zerada antes do uso; o tick não deixou escrita nela |
| **Interface (fora da regra)** | `_win_hwnd`, `_win_handles`, `_ribbonBarHeight` (seg. 7, `_win_IsWinOpen`); `_mapMessage`, `_mapMessageRemoveTime`, `_editMessage`, `_editWidth`, `_editHeight` (seg. 1, `_DrawRibbonMessage`/`_EditMessage`); `_bigFontH`, `_paletteH`, `_displayType`, `_clipWind`, `_clipDC` (seg. 2, `GR_MODULE`); `_pack_buf` | — | o tick chama rotinas de mensagem e desenho; só importa se mudar o estado (ver "Desenho") |
| **Seletores de segmento** | ~18 palavras em `10:be6c..10:c6ac` com valores `0x152f`/`0x1537`/`0x1527` | não | constantes de seletor que o código usa para trocar de segmento (corrigidas pelo carregador); não são estado |

### Lista de captura adotada

`extras.json` (antes e depois), definido em `EXTRAS` de `tools/qemu_bancada.py`:

| Campo | Endereço | Bytes |
|---|---|---|
| `SementeS` | `10:cbf2` | 2 |
| `SementeR` | `10:ae34` | 4 |
| `gameCycles` | `9:9b2c` | 4 |
| `Tindex` | `9:9b6a` | 2 |
| `GlobalKey` | `9:7866` | 2 |
| `GameTime`, `Cycle` | (redundantes com o save, para conferência) | — |
| `posicao` | `entrada` ou `saida` | — |

Mais `vida_antes.bin` / `vida_depois.bin`: `_LifeA` (8.192) + `_LifeB` (4.096) + `_LifeR` (4.096) = 16.384 bytes. Pela [especificação do save](../formatos/save-ant.md), `_DoLoadInitializations` reconstrói esses mapas das listas ao carregar; o port pode reconstruí-los do mesmo jeito e usar o arquivo só para conferir (verificado nos 5 vetores versionados: a reconstrução dá o mesmo mapa no meio da simulação, antes e depois do tick; ver "Comparar o port").

## Protocolo de teste

### Gerar vetores (original)

```
set SIMANT_RAIZ=C:\...\Simant recomp
python tools/gerar_vetores_tick.py --ticks 5                       # tests/referencias/ticks/
python tools/gerar_vetores_tick.py --ticks 3 --alvo 300 --semente-s 0x1234 --semente-r 1 --saida re_trabalho/v300
python tools/gerar_vetores_tick.py --ticks 1 --alvos 127,159,191,223 --saida tests/referencias/ticks/fases          # vetores esparsos
python tools/gerar_vetores_tick.py --ticks 5 --preparar tests/referencias/roteiros/tick_amarela_ordem.txt --saida ...  # roteiro (clique) antes
python tools/gerar_vetores_tick.py --ticks 10 --livre 300 --saida tests/referencias/ticks/colonia_5min               # 300 s livre antes
```

`--alvos` grava `--ticks` ticks a partir de cada alvo, na mesma execução. `--preparar` executa um roteiro da bancada (`clicar_em`, `teclas`, `esperar`...) com a VM rodando, logo depois do `loadvm`. `--descricao` vai para o `indice.json`. Para não disputar o overlay com outra bancada, use `--overlay` com uma cópia de `re_trabalho/qemu/c_overlay.qcow2` (o snapshot `quick` vai junto).

**Cuidado com a fase:** o vetor `tick_NNNNNN` é nomeado pelo `GameTime` **antes** do tick, e o tick soma 1 a `_Cycle` antes de testar as condições periódicas. Para exercitar `_Cycle & 0x1F == 0` com `_Cycle` = 128, o vetor é o `tick_000127`.

```
```

Saída por tick, em `tick_NNNNNN/` (NNNNNN = `GameTime` antes do tick):

| Arquivo | Conteúdo |
|---|---|
| `estado_antes.bin` | 307 blocos do save, formato `.ANT` (48.386 bytes), na entrada de `_DoAntSim` |
| `estado_depois.bin` | idem, no retorno de `_DoAntSim` |
| `vida_antes.bin`, `vida_depois.bin` | `_LifeA` + `_LifeB` + `_LifeR` (16.384 bytes) |
| `extras.json` | `{"antes": {...}, "depois": {...}, "MacTickCount_lidos": [...]}` com os campos da tabela acima; `antes.MacTickCount` = primeiro valor devolvido por `_MacTickCount` dentro do tick (`null` se não houve chamada) e `MacTickCount_lidos` = todos, na ordem (desde 2026-10-17) |

`indice.json` guarda a origem: snapshot, sementes forçadas, alvo, a primeira entrada, os seletores e o endereço de retorno de `_DoAntSim`.

### Vetores no repositório

`tests/referencias/ticks/`: ticks 79 a 83 do snapshot `quick` (Quick Game, velocidade normal, janelas do início do jogo), sem forçar sementes. No tick 80 (`_Cycle` = 80, múltiplo de 16) a rainha preta põe um ovo (`TotalEggsLaidB` 3 → 4, lista B cresce). Repetibilidade conferida: os ticks 79–81 batem com os da execução E1 (exceto `_BirdDelay` a partir do tick 80, ver acima).

Cenários em subpastas (gerados em 2026-10-17, listados em `"cenarios"` do `indice.json` da raiz; cada um com o seu `indice.json`, que guarda o comando, a `descricao` e as exceções do teste):

| Subpasta | Ticks | Como foi gerado | O que exercita |
|---|---|---|---|
| `fases/` | 127, 159, 191, 223 (1 cada) | `--alvos 127,159,191,223` | `_Cycle` = 128, 160, 192, 224 depois da soma: fases 0, 1, 2 e 3 de `_DoSmells` (compactação A/B/R, `_FullCount`, `_HistUpdate`, `_SmoothAlarm`, `_FillHoles*`, `_ColonySmell*`); 128 e 192 também são múltiplos de 64 (`_FeedAnts`: `HealthB/R` −1, `MeHealth` −1). No 127 o original mudou 43 blocos, entre eles `HealthB/R`, `MeHealth`, `H_*`, `HistCnt`, `HistStart`, `CastePop*` |
| `amarela_andando/` | 85–89 | `--preparar tests/referencias/roteiros/tick_amarela_ordem.txt`: clique simples na célula (32, 1) do ninho preto (tela (160, 182)) | a amarela anda de (35, 8) a (33, 3), um passo por tick, com `_MeMoveMe` = 1 e `_MeCmd` = 0, passando pelas células da rainha preta |
| `colonia_5min/` | 1885–1894 | `--livre 285 --alvo 1885` (velocidade normal, sem intervenção) | 140 vagas na lista A, 59 na B, 75 na R (muitas mortas, T = 0), formigas em vários modos, gato fora de cena (relógio do Mac lido em todo tick), formiga-leão |

Cenários de eventos (2026-10-17, `tools/cenarios_tick.py` a partir do snapshot **rico** — `tests/referencias/roteiros/qemu_snapshot_rico.txt`: o `quick` rodando 285 s, ~1710 ticks, com chuva caindo, menino cortando grama, aranha na tela e dezenas de formigas). O script grava sequências longas em `re_trabalho/cen/` (fora do git), lista os eventos (`eventos`) e copia janelas escolhidas (`selecionar`); o `indice.json` de cada cenário guarda os `passos` (escritas na memória, cliques) e a `selecao`:

| Subpasta | Ticks | Preparação | O que exercita |
|---|---|---|---|
| `chuva/` | 1711–1712, 1747–1751 | `_GlobalKey = 0xA8` na entrada do 1711 | inundação dos ninhos (`_DoWater`, `WaterTable` 46→45, `MapB/MapR`); fim da chuva (1749) e recomeço com `_InitWater` (1750) |
| `chuva_secando/` | 1769–1771 | nenhuma | água baixando (`WaterTable` 46→47, 0x4E volta a túnel com `S(8)`) |
| `revoada/` | 1711–1718 | 18 formigas da superfície em M = 15 e `_FlyAwayB/R = 3` | modos 15 → 16 (`S1(F)`, depois F = 1) e saída do mapa no modo 16 (`S32`) |
| `menino_cortador/` | 1711–1715 | menino cortando (`_BoyHere = 3`, `_BoyIsMowing = 1`) posto no patch atual (11, 8) | `_FootFall` e `_MowerFall` (38 de 56 formigas mortas no 1712), sob chuva |
| `predadores/` | 1795–1797, 1812–1813 | nenhuma | aranha come uma preta (`_SMode` 2→3, `_BAntsEaten` +1, 1796); formiga-leão come (1813) |
| `amarela_superficie/` | 1732–1734, 1790–1792, 1850–1852 | Ctrl+8 e cliques calculados (`_MapPnt` escrito para pôr a célula no centro) | sai do ninho (`_ExitNest`), pega comida (duplo clique, `_TryMyDropOrLift`) logo depois da compactação da lista A, entra no ninho vermelho (com troca da vista) |
| `amarela_luta/` | 1860–1864 | duplo clique numa vermelha (ordem 3), logo depois de uma compactação | perseguição, `_YellowFight` com derrota, `_AnimYellowFight` (o relógio é lido ~1000 vezes no tick), renascimento como ovo (`_MeEgg` 1→7) |
| `velocidade_rapida/`, `velocidade_ultra/` | 1899–1901, 1977–1979 | Shift+3 / Shift+4 | o tick é o mesmo; `_gameCycles` anda 1 por tick e o relógio do Mac fica parado (17427) enquanto a bancada avança tick a tick |

Volume: 9,5 MB ao todo em `tests/referencias/ticks/` (~135 KB por tick).

### Comparar o port

Para cada `tick_NNNNNN`:

1. carregar `estado_antes.bin` como um save, **sem** as inicializações de carga que mexem nos geradores (o `_LoadGame` do original chama `_RandYard`, que reinicia S; ver [rng.md](rng.md#a-semente-está-no-save));
2. pôr S, R, `_gameCycles`, `_GlobalKey` (e `_Tindex`) com os valores de `extras.json["antes"]`; montar os mapas de vida (reconstruir ou ler `vida_antes.bin`);
3. rodar **um** `_DoAntSim`;
4. gravar o estado no formato `.ANT` e comparar com `estado_depois.bin` (`tools/estado_diff.py`); comparar S e R com `extras.json["depois"]` e a vida com `vida_depois.bin`.

No port, os passos 1–4 estão em `engine/src/rules/vetor_tick.rs` (`VetorTick::ler`, `montar_entrada`, `comparar_tick`):

```
cargo run -p engine --example comparar_tick -- tests/referencias/ticks/tick_000079   # um tick, relatório completo
cargo run -p engine --example comparar_tick -- tests/referencias/ticks               # todos, mais o placar
```

O relatório traz: blocos iguais/diferentes (com bytes diferentes e quem mudou o bloco: o original, o port ou os dois); as primeiras diferenças de cada bloco, por formiga e campo nas listas (`formiga 5 campo T`), por célula `(x, y)` nos mapas (`MapA/B/R`, `ExitMap*`, `PherMap*`, células do próprio mapa) e por elemento nos demais; S, R, `_gameCycles`, `_Tindex` e `_GlobalKey` depois do tick; a vida depois do tick; e duas conferências da reconstrução da vida (abaixo). O teste `engine/tests/ticks_originais.rs` roda todos os vetores de `indice.json`.

**Reconstrução da vida (verificado, 2026-10-17, nos 5 vetores):** os mapas `_LifeA/B/R` reconstruídos das listas como `_DoLoadInitializations` (`core/vida.rs`, o mesmo que `Estado::de_save` usa) são **idênticos** a `vida_antes.bin` e, reconstruídos do `estado_depois`, a `vida_depois.bin`. A hipótese da seção "Lista de captura adotada" fica confirmada para esses ticks (sem amarela rainha; a segunda célula dela continua sem reconstrução). O comparador usa a vida gravada mesmo assim, e o teste exige que a reconstrução continue batendo.

Os ticks consecutivos são encadeáveis: verificado que o `estado_depois` do tick N é igual ao `estado_antes` do N+1 (só `_gameCycles` muda no intervalo). Assim, um teste de N ticks pode partir do primeiro `estado_antes` e conferir cada `estado_depois`, desde que use o `_gameCycles` de cada `extras.json["antes"]`.

## Placar do port

Um tick do port (`RegrasOriginais` sem tabelas de geração) sobre cada vetor de `tests/referencias/ticks/`, comparado com o `estado_depois` do original. "Mudados" = blocos que o original alterou no tick; "extras" = S, R, `_gameCycles`, `_Tindex`, `_GlobalKey`.

**2026-10-17, depois de integrar quintal, inimigos, superfície, ninho, amarela e conselhos:** os 5 vetores batem **inteiros**.

| Tick | Blocos iguais | Mudados pelo original (todos iguais) | Extras iguais (S, R, `_gameCycles`, `_Tindex`, `_GlobalKey`) | Células de vida diferentes |
|---|---|---|---|---|
| 79 | 307/307 | 28 | 5/5 | 0 |
| 80 | 307/307 | 22 | 5/5 | 0 |
| 81 | 307/307 | 19 | 5/5 | 0 |
| 82 | 307/307 | 21 | 5/5 | 0 |
| 83 | 307/307 | 19 | 5/5 | 0 |

O último ajuste foi o relógio do quintal: `_BirdDelay` e `_CatDelay` só batem usando o `_gameCycles` do vetor (`Estado::ciclos_jogo`) também como relógio do Mac [hipótese; ver quintal.md, "Implementação no port"]. Histórico: só o esqueleto dava 280–289/307; com inimigos, amarela e superfície, 284–293; com o ninho e o quintal, 287–305; com o relógio, 307.

**O teste exige igualdade total** (`engine/tests/ticks_originais.rs`): qualquer bloco, extra ou célula de vida diferente falha. Ele roda os vetores da raiz e os de cada cenário em `"cenarios"`. Exceções, sempre com o motivo, no `indice.json` do cenário: `"pendentes"` (o port ainda diverge; só se exige que o vetor rode e que a ida e volta do save funcione; quem corrigir a regra tira o tick da lista) e `"vida_nao_reconstruivel"` (a vida do original não sai das listas; não se confere a reconstrução, mas a vida depois do tick continua exigida).

### Cenários (2026-10-17, depois de gravar o relógio do Mac)

Todos os 24 vetores batem **inteiros** (307/307 blocos, extras 5/5, vida 0): raiz 79–83, `fases/127, 159, 191, 223`, `amarela_andando/85–89`, `colonia_5min/1885–1894`. Nenhum vetor em `"pendentes"`.

| Cenário | Ticks | Mudados pelo original (todos iguais) | `MacTickCount` lido no tick |
|---|---|---|---|
| raiz | 79–83 | 19–28 | nenhuma leitura (gato andando) |
| `fases/` | 127, 159, 191, 223 | 43, 30, 32, 20 | 127: nenhuma; 159: 3333; 191: 3621; 223: 3909 |
| `amarela_andando/` | 85–89 | 23–31 | nenhuma leitura |
| `colonia_5min/` | 1885–1894 | 22–40 | 18906, 18915, …, 18987 (uma leitura por tick, +9 por tick) |

**Correção:** `tools/qemu_bancada.py` (`Depurador.ir_saida(relogio=...)`) põe breakpoints alternados na entrada e no RETF (`1000:96a5`) de `_MacTickCount` durante o tick e anota DX:AX de cada chamada; `extras.json` ganhou `antes.MacTickCount` (primeiro valor lido, `null` se o tick não leu) e `MacTickCount_lidos` (todos). O comparador põe esse valor em `Estado::relogio_mac`, e `quintal::Relogios::do_port` o usa como relógio do Mac quando presente. Todos os vetores foram regravados (mesmos ticks). Os de `fases/191`, `fases/223` e `colonia_5min` saíram **diferentes** da primeira gravação: como os sorteios do gato e do latido dependem do tempo real, o original não é determinístico tick a tick a partir do snapshot quando o gato está fora de cena (S na entrada do tick 191: 0x6038 numa execução, 0x3353 na outra). Os vetores continuam válidos porque cada um traz o relógio que leu.

**As duas pontas soltas (1891 e 1893 da primeira gravação):** também eram o relógio. Conferido injetando nos vetores antigos (fora do repositório) o relógio estimado (18903 + 9 por tick: 18903 no 1887, 18939 no 1891, 18957 no 1893): os três passam a bater 307/307, com extras 5/5. Mecanismo: com o relógio errado, o port fazia o sorteio extra do gato (`_CatDelay` 19503 vencido). Isso deslocava os sorteios seguintes (o cachorro no 1891, duas formigas da superfície no 1893), e o total de sorteios do tick coincidia por acaso, de modo que S e R batiam no fim. Não há erro de regra.

**Histórico — divergência da primeira gravação (resolvida): relógio do Mac do gato (quintal, `_SimCat`, [quintal.md](quintal.md) seção 6).** O port usava `Estado::ciclos_jogo` (o `_gameCycles` do vetor) como relógio do Mac; o original usa `GetTickCount() / 55 × 3`. Evidência:
- `colonia_5min/1887`: o gato termina a fuga (estado 3) e grava `_CatDelay = relógio + 600`. Original: 19503, logo relógio ≈ 18903 (≈ 315 s em 1/60 s); port: 270585 = `_gameCycles` (269985) + 600. É o único bloco diferente nesse tick.
- `fases/159`, `191`, `223`: com `_CatOn = 0`, o original não mexe em `_CatDelay` (3654, gravado no tick 127, e 4064), portanto o relógio dele ainda é ≤ esse valor; o port compara com `_gameCycles` (29579, 36125, 42381), acha o atraso vencido, grava `_gameCycles + 200` e sorteia `S16`. O gerador S diverge **antes** de `_DoAntSimA` (o quintal é o passo 6 do tick) e o resto vem em cascata: gato aparecendo só no port (`CatOn`/`CatX`/`CatDir`/`CatCycle`, tick 159), cachorro (`DogDir`/`DogFrame`/`DogTurnCnt`/`DogX`), uma formiga da superfície (`AlistX/Y/T`) e, no 223, a rainha preta que põe um ovo só no port (`Blist*[11]`, `ListIndexB`, `TotalEggsLaidB`, `HealthB`).
- `colonia_5min/1888–1894`: o mesmo, partindo do `_CatDelay` = 19503 do original: S diverge e daí formigas da superfície e dos ninhos (`Alist*`, `Blist*`, `Rlist*`), feromônios (`PherMapBN/BT/RN/RT`), `MapA`, `SowDir`, `RedTask`, `LionListT` e a vida. No 1889 o port ainda mexe em 64 bytes de `MapB` e de `MapR`, em `WaterTable` e em `ListIndexR` [hipótese: também cascata]. No 1891 e no 1893 S e R batem no fim do tick mas o cachorro (1891) e duas formigas (1893) divergem: causa não isolada.
- Nos ticks 79–83 e 127 o relógio não é lido (gato andando ou ainda em cena), por isso a hipótese "relógio do Mac = `_gameCycles`" passava.

**Divergência 2 — vida não reconstruível das listas** (não é erro do port; o comparador usa a vida gravada). Casos vistos:
- `amarela_andando/86–88`: a amarela passou pelas duas células da rainha preta (Blist 0 e 1, T 96 e 104, em (34, 6) e (34, 7)); ao sair de cada uma, o original zerou a célula e a marca da rainha sumiu de `_LifeB`.
- `fases/159`, `191`: vários ovos e larvas em (34, 8); o ovo posto no tick (índice mais alto, T 1) fica em `_LifeB` porque foi escrito por último, e a reconstrução de `_DoLoadInitializations` deixa o de menor índice (T 2).
- `colonia_5min` (todos): dezenas de vagas mortas (T = 0) paradas nas células dos buracos (ex.: lista A em (2, 32), lista R em (32, 1)) apagam, na reconstrução, a marca da formiga viva de índice maior que está na mesma célula.
Consequência para o port: um save carregado no meio do jogo pode ter vida diferente da que o original tinha em memória; o original, ao carregar o mesmo save, faz a mesma reconstrução, então isso só importa para comparar com memória viva.

### Cenários de eventos (2026-10-17, main em `317026e`)

Os **48 vetores novos batem inteiros** (307/307, extras 5/5, vida 0); nenhum fica em `"pendentes"`. Antes da seleção, as gravações brutas inteiras também foram comparadas: 552 ticks de `re_trabalho/cen/` (chuva 80, livre 250, amarela 122, luta 64, revoada 12, menino 12, fast 6, ultra 6), todos 307/307 depois das duas correções abaixo. Todos têm `vida_nao_reconstruivel: {"*": ...}` (vagas mortas nas células dos buracos, como em `colonia_5min`).

Duas divergências apareceram e foram corrigidas:

1. **Regra — `_DoAntMoveY` depois de entrar no ninho** (`engine/src/rules/amarela.rs`, `passo_da_ordem`, bloco final). Com destino = a própria célula do buraco e o plano novo ≠ plano exibido, o original termina a ordem (r = −1 → `_ResetYellowVars` na posição nova: `MeGoal*` = (3, 32, 1), `MeLast*`, `MeMoveMe = 0`, `MeSteps = 0`); o port só centralizava e deixava a ordem andando. No pseudocódigo, depois do `_GotoMyAnt` o fluxo cai no rótulo `1028:7b6b` (`local_8 = −1`). Correção: `return (-1, alvo)` nesse caso (4 linhas). Vetor: `amarela_superficie/1852`. Spec: formiga-amarela.md §4.
2. **Comparador — a vista anda dentro do tick.** `_GotoMyAnt` (`1000:a764`) chama `_CenterEdit`/`_SetMapPlaneLocation` no meio do tick e `_MapPnt`, `_MapPlane`, `_MapMode` estão no save; no port isso é da interface (`Evento::CentralizarNaAmarela`, tratado por `core::partida`). `vetor_tick.rs` (`aplicar_vista_da_bancada`) agora reproduz o efeito no save quando o tick pede a centralização: troca `_MapPlane`/`_MapMode` se a amarela está noutro plano e anda `_MapPnt` como `_CenterEdit` (Bresenham com truncamento, telas/jogo.md "Centralizar"), com a janela da bancada de 25 × 19 tiles. Conferido em `amarela_superficie/1852` ((97, 23) → (20, 0)) e `amarela_luta/1864` ((47, 9) → (25, 1): o eixo menor anda um passo a menos). [hipótese] `_editWidth` = 25 na bancada (as duas contas só fecham com meia largura 12, não 13).

**Lacunas:** inseticida, Jogo Completo e Experimental não gravados (o snapshot é do Quick Game; chegar a outro tipo de jogo exige navegar diálogos de File > New, não tentado); revoada só forçada (não a armada por `_GstrR`), sem `_QueenStorage`/`_SwarmCnt` (só sobem no Jogo Completo); a luta gravada é uma derrota — falta uma vitória; só um vetor por fase de `_DoSmells`; o port usa um valor de relógio por tick, e em `amarela_luta/1864` a lista lida tem ~1000 valores diferentes, que ainda assim bateu porque a animação só usa o relógio como espera (sem sorteio dependente dele) [hipótese a conferir em `_AnimYellowFight`].

### Terceira rodada (2026-10-17, main em `c182a5d`): modos de jogo, inseticida, vitória, revoada armada e sequência longa

| Subpasta | Ticks | Preparação | O que exercita | Resultado |
|---|---|---|---|---|
| `jogo_completo/` | 31–35 | snapshot `full` (`roteiros/qemu_snapshot_full.txt`: boot, "Select a Game" → Full Game em (265, 300), `_CurGameType = 2`) | início do Jogo Completo (rainha alada amarela na superfície), fase 1 de `_DoSmells` | 5 batem |
| `experimental/` | 31–34 | snapshot `exp` (`roteiros/qemu_snapshot_exp.txt`, como `qemu_experimental_e01.txt` do agente auxiliar) | Experimental sem ferramentas; ovo posto no 31 | 4 batem (31–32 com vida não reconstruível: ovo sobre larva) |
| `inseticida/` | 1711–1714 | rico; menino dentro de casa parado no nó 32 ((45, 166) → patch (0, 12)) e `_CurYardPnt = (0, 12)` | pisada e inseticida em todo tick (`(_RpopT + _BpopT) × 8` sorteios de célula; uma formiga morta por tick) | 4 batem |
| `inseticida_morte/` | 19–21 | `full`, a mesma preparação | a amarela (rainha alada) morre pelo inseticida (`_YellowDeath(9)`, 21) | 19–20 batem; **21 pendente** (abaixo) |
| `amarela_vitoria/` | 1799–1801 | rico; amarela posta como soldado (vida 100) ao lado de uma operária vermelha parada (M = 17), duplo clique | `_YellowFight` com vitória (`RAntsKilled` 0→1, cadáver vermelho) | 3 batem |
| `revoada_armada/` | 1726–1727, 1825–1826, 1924–1927 | rico; 21 vermelhas viram machos (T 0xA0, M = 14) | o jogo arma `_FlyAwayR = 200` (`_GstrR`, 1727), pré-voo 14 → 15 com F > 100, decolagens (16) e saídas até F = 0 | 8 batem |

Sequência longa (`tests/referencias/ticks_longos/rico_200/`, 392 KB): 200 ticks consecutivos do snapshot rico sem intervenção (GameTime 1711–1910), gravados por `cenarios_tick.py gravar longo`: `inicio/` (estado e vida na entrada do 1º tick), `extras.json` (os extras antes e depois de cada tick, com o relógio) e `depois_0050/`, `_0100/`, `_0150/`, `_0200/` (estado e vida). `vetor_tick::rodar_sequencia` roda os 200 ticks encadeados no port, pondo antes de cada um só o que muda fora do tick (`_gameCycles`, relógio do Mac, `_GlobalKey`), e confere S e R depois de cada tick e o save inteiro e a vida nos 4 pontos; `engine/tests/ticks_longos.rs` exige tudo igual. **Resultado: 200/200, S e R iguais em todos os ticks e os 4 pontos 307/307 com vida igual.**

**Placar da rodada:** 27 vetores novos, 26 batem inteiros e exigidos; 1 pendente; a sequência longa bate. Antes da seleção também foram comparadas as gravações brutas (Jogo Completo 40, Experimental 40, inseticida 13, vitória 30, revoada 31): só o `inseticida_morte/21` difere. Nenhuma correção de regra foi necessária nesta rodada.

**Resolvido (2026-10-18): `inseticida_morte/tick_000021` pela interface.** O tick do port termina com `_IsGameOver = 1` (o que a regra manda); a interface fecha as caixas 10042 e 10056 e o GAME OVER e, no "Select a Game", o Full Game começa com `carregar::jogo_novo_com_geradores`: `_RandYard` com o R e o S **da partida aberta** (o `_NewGame` não ressemeia) e, como no Load do tutorial, o que `_RandYard` não escreve fica como estava (lista em telas/arquivos.md), mais `_LessonTemp = 1` (`_GotoMyAnt`) e a lição só recomeçando no Tutorial. Resultado: **307/307 com S e R iguais** ao estado de depois do vetor (`engine/tests/fim_de_jogo_interface.rs`, `tick_21_volta_com_o_jogo_completo_novo_do_original`). No `indice.json` do cenário o tick saiu de `"pendentes"` e foi para a chave nova `"pela_interface"` (o teste de ticks só confere que ele roda). Texto anterior:

**Pendente `inseticida_morte/tick_000021` — fim de jogo pela interface dentro do tick** (regravado em 2026-10-17 com o breakpoint do retorno armado antes de cada clique: dentro do mesmo `_DoAntSim` abrem-se a caixa 10042, a 10056, o GAME OVER e a tela "Select a Game" — capturas `dialogo_21_0..3.png` —, a bancada escolheu Full Game e o tick volta com um Jogo Completo novo com `GameTime` **0**; o 8 da 1ª gravação era artefato do script). Texto da 1ª gravação: Sem corpo (nenhum ovo ou adulto preto) e com `_ColonyTotalBlack = 1`, `_YellowDeath` põe `_IsGameOver = 1` e o passo 22 do tick (`_EndGameDialog`) abre a caixa "GAME OVER" (capturas `re_trabalho/cen/dialogo_21_*.png`: antes, "You are covered with insecticide!"). A bancada clicou para continuar e o original começou um Jogo Completo novo **dentro do mesmo tick** (mundo novo, `_CurYardPnt` (0, 12) → (11, 8), `GameTime` 8, `_IsGameOver` de volta a 0). O port termina com `_IsGameOver = 1` e o mundo intacto — o que a regra manda; o reinício é da interface (`simant/`, `core/`), não do tick. Não é erro de regra.

**Como virar teste de interface** (descrição; não implementado): partir de `inseticida_morte/tick_000021/estado_antes.bin` numa `core::partida::Partida` (com `relogio_mac`, `_gameCycles` e sementes de `extras.json`), rodar um tick e conferir que (1) o estado do port é o do original até o passo 22, com `_IsGameOver = 1` (o vetor de tick já confere isso, menos o mundo novo); (2) a fila tem `Evento::DialogoFimDeJogo` uma vez (o "GAME OVER" com pontuação 27572 e a classificação "Aardvark Fodder" da captura `dialogo_21_1.png`); (3) o "continuar" leva à tela "Select a Game" e, com Full Game, a `Mundo::novo_jogo(TipoJogo::Completo, ...)`. O original faz (3) dentro do tick: o `estado_depois.bin` do vetor é o começo de um Jogo Completo novo (patch (11, 8), `GameTime` 8 — [hipótese] ticks do jogo novo rodados antes de a bancada chegar ao retorno do `_DoAntSim` antigo), gerado com as sementes de R e S do instante do clique, que dependem do relógio; por isso o teste de interface deve conferir a forma do jogo novo (tipo, patch, rainha alada amarela na superfície, colônia vermelha), não os bytes.

Bancada (novidades de `tools/cenarios_tick.py` e `qemu_bancada.py`): um tick que abre uma caixa modal não volta sozinho — depois de 15 s sem chegar ao retorno de `_DoAntSim`, o script captura a tela, clica em (320, 330) e aperta Enter (até 5 vezes); as leituras do relógio por tick param em 200 (a lista termina com `null`), porque as animações (`_AnimYellowFight`, morte) leem o relógio milhares de vezes e cada parada custa ~0,1 s; um clique pode ser enviado com a VM indo da entrada de um tick à próxima (`clicar_celula(..., pausado=True)`), para gravar desde o tick em que o Windows o trata.

**Lacunas:** revoada no Jogo Completo com `_QueenStorage`/`_SwarmCnt` (exige colônia vermelha com reprodutores no Jogo Completo; não tentado); `_FlyAwayB` (quem arma é o jogador, ciclo de vida); Experimental com ferramentas (as do agente auxiliar aplicam com o jogo pausado, fora do tick); `_SpecialXfer` (morte no Jogo Completo com mais de uma colônia preta, escolha no mapa do quintal).

### Quarta rodada (2026-10-17, main em `613a5b1`): sequência longa com interface, Tutorial, slow

**Sequência longa com interface** (`tests/referencias/ticks_longos/full_600/`, 652 KB; `cenarios_tick.py gravar longo_full`): Jogo Completo do snapshot `full`, com chuva pedida (`_GlobalKey = 0xA8`) e enxames escritos na entrada (`_SwarmCntB/R` = 5/2, `_QueenStorageB/R` = 3/4) e a **Yard View na frente** (Ctrl+Y) durante 600 ticks (GameTime 31–631). Gravados: o estado inicial, os extras de cada tick (S, R, `_gameCycles`, relógio do Mac, `_GlobalKey`) e o estado e a vida a cada 100 ticks. Os passos de S do desenho entre os ticks são a diferença entre o S no fim de um tick e no começo do seguinte. `vetor_tick::rodar_sequencia` (com `indice.json["desenho_quintal"] = 0`) roda os 600 ticks no port, faz `desenho::desenhar_quintal` entre eles e confere S e R depois de cada tick e o S depois de cada desenho. **Resultado: 600/600, 17.650 passos de S do desenho iguais aos do original, os 6 pontos 307/307 com a vida igual.** Achado no caminho: o desenho do menino (`_DrawSimKid`, `1010:af14`) apaga o balão (`_BoyMessOn = 0`) quando o relógio passa de `_BoyMsgCnt` — sem isso o ponto de 600 ticks diferia só nesse bloco (`desenho::desenhar_menino`).

**Tutorial** (`ticks/tutorial/`, snapshot `tut`: boot e "Select a Game" → Tutorial Game em (180, 225)): lição 1 concluída e lição 2 pedida (15), lição 3 esperando o marco de escavação (`_TimeTemp` = 15, 46–47): 307/307, com `_TimeTemp` (`9:9b10`, fora do save) agora gravado nos extras (`TimeTemp`) e posto na memória do tutorial do port (`vetor_tick::regras_do_vetor`); o teste confere `_TimeTemp` depois do tick. Antes de gravá-lo, o port dava a lição 3 por concluída (`_TilesDugB > _TimeTemp` com `_TimeTemp` = 0).
- **`tutorial/23` (regravado):** `_Feedback` → `_GiveLesson(3)` abre a caixa da lição com `_PictStrnDialog` dentro do tick (`dialogo_23_0.png`). A 1ª gravação dava `GameTime` 23 → 40 e parecia ter ticks aninhados; era **artefato do script** (ele fechava a caixa com a VM correndo sem o breakpoint do retorno: o tick terminava sem ser visto e os seguintes rodavam soltos). `capturar_tick` agora arma o breakpoint do retorno antes de clicar; regravado, o tick volta com `GameTime` 24 e bate 307/307 (a desmontagem também mostra que `_PictStrnDialog` para o temporizador). O mesmo valia para o `GameTime` 8 de `inseticida_morte/21`.
- Também neste cenário: `_CatDelay` = 4712 vinha do arquivo do tutorial (outra sessão) com o relógio em ~3300, e o port o "normalizava" (atraso maior que agora + máximo = vencido). Com relógio real o original deixa como está: `quintal::normalizar_atrasos` agora só age com o relógio sintético (sem `relogio_mac` e sem `_gameCycles`).

**Velocidade slow** (`ticks/velocidade_lenta/`, Shift+1, `_GameSpeed` 0, 1719–1721): 307/307.

**Placar da rodada:** 8 vetores novos, 7 batem e são exigidos, 1 pendente; a sequência longa com desenho bate inteira. Ações novas em `acoes/`: `exp_ninho`, `exp_cheiro` (telas/experimental.md §9), 307/307. Fim de jogo pela interface: `engine/tests/fim_de_jogo_interface.rs` (descrito acima, agora implementado): o tick do vetor numa `core::partida::Partida` com o relógio do original dá `_IsGameOver`, um só `DialogoFimDeJogo`, as três caixas na ordem do original (10042, 10056 de `_PictStrnDialog` em `_YellowDeath`, e o GAME OVER) e a **pontuação 27572** mostrada pelo original; o Jogo Completo novo tem a forma do que o original começou; a volta para "Select a Game" está em `simant/` (teste `#[ignore]` com o motivo).

### Quinta rodada (2026-10-18): regras não verificadas da revisão

Itens da lista "Não verificado" de [../../estado-do-projeto.md](../../estado-do-projeto.md) §3, gravados por `tools/cenarios_regras.py` (cenários novos sobre a mesma `Sessao` de `cenarios_tick.py`; `gravar`, `eventos`, `selecionar`, mais `registrar` e `selecionar_lote`). A preparação escreve a memória do original na entrada de um tick (aranha, formigas da lista com a marca no mapa de vida, a amarela, células dos mapas); quando a regra passa pela interface, a preparação usa a interface do original: tecla X e clique na aranha para virar a aranha (`_DoLifeExchange`), tecla 1 e clique para o modo alvo, duplo clique para correr, troca de patch pela Yard View para chegar à casa. Snapshot `rico` (Quick Game com ~1710 ticks), salvo o choque (`full`).

| Subpasta | Ticks | O que exercita |
|---|---|---|
| `aranha_cerco` | 1793 | 9 pretas no 3 × 3: estado 5, `_DeathCnt` 500, mensagem 10046, `_SpidRevenge` 2 → 3 e `_MeSMode` = 7 |
| `aranha_foge` | 1793 | 6 pretas no 3 × 3: estado 4 e corrida |
| `aranha_vinganca` | 1793 | cerco com `_SpidRevenge` 4 → 5: `_MeSMode` = 8 (laser) |
| `aranha_faro` | 1793, 1805, 1816 | `_SFoundAnt` por faro, perseguição, comer com `_EatCnt` = 11, `_DeadAntHere` |
| `aranha_laser` | 1794, 1796 | `_SpiderScan`: uma preta do leque morta em cada tick |
| `leao_cerco` | 1793 | formiga-leão comendo com 7 vizinhas ocupadas: 10050, centro 0x3F, lista deslocada |
| `aranha_jogador` | 1801, 1836, 1855, 1856 | aranha do jogador: anda, corre, persegue no modo alvo (`_MeSMode` 6) e come (`_EatCnt` 50) |
| `aranha_jogador_cercada` | 1800 | a aranha do jogador perseguindo e cercada: `_YellowDeath(3)` |
| `aranha_jogador_fora` | 1800 | a aranha do jogador sai do mapa correndo: `_YellowDeath(4)` |
| `amarela_morte_leao` | 1711 | `_YellowDeath(2)` com `_AnimYellowFight` e duas centralizações no tick |
| `amarela_morte_pisada` | 1712 | `_YellowDeath(5)` (o lugar do pé veio de uma execução de ensaio) |
| `amarela_morte_cortador` | 1712 | `_YellowDeath(6)` |
| `amarela_morte_afogada` | 1711 | `_YellowDeath(7)` com `_AnimYellowInsane` |
| `amarela_morte_fome` | 1711 | `_YellowDeath(8)` com `_AnimYellowInsane` |
| `amarela_morte_choque` | 27 | `_YellowDeath(10)`: a ordem termina em cima da tomada 0x76 da casa (patch (1, 5) do Jogo Completo) |
| `amarela_morte_aranha` | 1793 | `_YellowDeath(1)`: com o desenho dentro do tick (abaixo, achado 2), 307/307 desde 2026-10-18 |
| `ninho_n01`, `ninho_n01_saude31` | 1714, 1720; 1711 | N-01: morte natural com `_HealthB` 0; nenhuma com 31 |
| `ninho_n07` | 1712, 1713 | N-07: mordidas na comida guardada |
| `ninho_n10` | 1711, 1713 | N-10: a água sobe uma linha |
| `ninho_n12` | 1711, 1716 | N-12: a luta não remove a checadora; a nuvem termina |
| `ninho_n14` | 1713 | N-14: passagem 0x14 do ninho vermelho para o preto |
| `ninho_n15` | 1711 | N-15: trofalaxia pela invasora vermelha |
| `ovos_c04_c05` | 1791 | C-04: morte na eclosão com `_ModeAuto` 0; C-05: casta vermelha com `_HealthR` 20 |
| `ovos_c06`, `ovos_c07` | 1711; 1711 | C-06: postura com saúde 100; C-07: nenhuma com saúde 0 |

**Placar: 38 vetores novos em 25 cenários, 37 batem inteiros (307/307, S, R e vida) e são exigidos; 1 pendente com a causa isolada** (resolvido depois, achado 2: hoje os 38 são exigidos). Antes da seleção, as gravações brutas inteiras foram comparadas (≈ 230 ticks, inclusive 83 da aranha do jogador e 24 do faro): só os ticks abaixo diferiram, e todos por causa conhecida. Volume: 4,7 MB.

Achados:

1. **Duas centralizações no mesmo tick (port corrigido).** Em `amarela_morte_leao/1711` o original deixou `_MapPnt` em (21, 0) e o comparador em (22, 0). Na morte o original chama `_GotoMyAnt` com a amarela ainda no centro da formiga-leão (superfície) e, no renascimento, de novo com ela no ovo (ninho preto); cada `_CenterEdit` parte de onde o anterior deixou a vista: (24, 0) → (18, 16) → (21, 0), com o eixo menor um passo aquém. O port centralizava uma vez só, na posição final. Correção: `Evento::CentralizarNaAmarela` passou a levar `{plano, x, y}` do momento do pedido (`Estado::pedir_centralizar_na_amarela`); a interface (`core::partida::ir_para_amarela`) e o comparador (`vetor_tick::aplicar_vista_da_bancada`) aplicam cada pedido na ordem. Nos scripts de mods, o evento `centralizar_na_amarela` ganhou `plano`, `x`, `y` (api-mods.md).
2. **Pendente `amarela_morte_aranha/1793` (294/307): desenho dentro do tick.** Isolado com `cenarios_regras.py gravar contar_aranha` (`contar_no_tick`: breakpoints na entrada de `_SRand1`, `_DrawPalps`, `_DrawSpider`, `_DoEditUpdateDraw`, `_GotoMyAnt`, `_SpiderDialog`, `_LionDialog`, `_AnimYellowFight`, `_mySongIsDone`, com o S e, para `_SRand1`, quem chamou; grava `contagem.json`): 4 passos de S a mais no original — `S1(4)` de `_DrawPalps` nos desenhos da janela de edição de `_GotoMyAnt`, de antes do laço de `_AnimYellowFight` e do 1º quadro (a aranha à vista, `_SMode` 3), e `S1(2)` de `_SpiderDialog` ([rng.md](rng.md)). O port não fazia os desenhos de dentro do tick. **Resolvido (2026-10-18):** o port passou a fazer os sorteios dos redesenhos de dentro do tick (`rules::desenho::EdicaoNoTick`, [rng.md](rng.md) "Desenho dentro do tick") e o `S1(2)` de `_SpiderDialog`; o vetor bate 307/307 com S, R e vida e saiu de `"pendentes"`. O comparador supõe a janela de edição aberta (como em todas as gravações da bancada); um vetor gravado com ela fechada leva `"EdicaoAberta": false` em `antes` do `extras.json`. A contagem de controle na morte pela formiga-leão (`contar_leao`) não tem `_DrawPalps` (aranha fora da vista) e bate. Com a música ligada e sem placa de som na bancada, `_mySongIsDone` encerra a animação no 1º quadro (as duas contagens).
3. **`_gameCycles` com caixa que não para o temporizador.** Em `ninho_n15/1711` o `_DoTroph` abre "You feel much better now!" (`_YellowDialog`), e `_gameCycles` anda 647 dentro do tick enquanto a bancada fecha a caixa (as caixas de `_PictStrnDialog` param o temporizador e não mudam `_gameCycles`). Chave nova no `indice.json` do cenário, `"gamecycles_fora_do_tick"`: o teste deixa de exigir só esse extra.
4. **Artefatos da bancada evitados:** (a) o clique em (320, 310) que fecha uma caixa pode chegar à janela de edição depois de a caixa sumir e virar uma ordem para a amarela (visto na 1ª gravação de `fome`: destino (40, 9) e um passo); `cenarios_regras.py` fecha as caixas com Enter; (b) o relógio do Mac lido depois de uma caixa modal não é gravado (a lista termina na caixa), e o `_CatDelay` gravado com ele não é reproduzível (vistos em `cortador` e `pisada`); nesses cenários o gato fica fora de jogo (`_CatDelay` enorme na preparação).
5. **A aranha do jogador com `_SMode` 0 ou 1 não é cercada**: o controle do jogador substitui o resto de `_MoveSpider` (inimigos.md 2.2); a 1ª tentativa de `aranha_jogador_cercada` (aranha parada) não morreu em 4 ticks. Cercada perseguindo, morre.

### Sexta rodada (2026-10-19): desenho dentro do tick que faltava

`tools/cenarios_regras.py` (cenários `aranha_autotrack`, `aranha_movey`, `morte_aranha_silly`), snapshot `rico`, com `capturar_tick_relogio`: além do vetor normal, as variáveis dos balões (`Baloes`, em `antes` e `depois`) e o valor de cada `_TickCount` lido pelo desenho (`TickCount_lidos`, breakpoints nos retornos dos `CALL _TickCount` do segmento 3; o relógio do Mac do vetor é a primeira leitura de `_MacTickCount`). Os 14 vetores batem 307/307 com S, R, vida e os extras dos balões (`ticks_originais`):

| Subpasta | Ticks | O que exercita |
|---|---|---|
| `aranha_jogador_autotrack` | 1810, 1812, 1844–1852 | aranha do jogador com AutoTrack: anda (a vista segue, sem sorteio) e, no modo alvo, persegue (palpos `S1(4)` no `_GotoMyAnt` de cada passo) e come; inimigos.md 2.7 |
| `desenho_aranha_movey` | 1800, 1811 | aranha à vista no redesenho de antes de entrar no buraco e no `_GotoMyAnt` depois de sair do ninho, com "Silly" (balão da aranha); rng.md |
| `amarela_morte_aranha_silly` | 1793 | morte pela aranha com "Silly", sem música e sem Events: o `_TickCount` de cada quadro de `_AnimYellowFight` (+3 por quadro); rng.md |

Achados: (1) o port não chamava `_GotoMyAnt` na aranha do jogador com AutoTrack (corrigido); (2) o relógio dos balões dentro do tick é o do momento do desenho: igual ao do começo fora das animações, +1 e depois +3 por quadro na luta (o port modela as esperas; o tempo de desenho e o das caixas modais, não); (3) `_AnimYellowInsane` espera 2 degraus por quadro (110 ms, não 55: `amarela.json`).

### Sétima rodada (2026-10-19): ramos raros

`tools/cenarios_raros.py` (mesma interface de `cenarios_regras.py`; snapshots `rico`, `full` e `exp`). A preparação escreve a memória na entrada de um tick; onde a regra é rara demais para esperar, a semente S da entrada foi **escolhida no port** (busca sobre o vetor do mesmo tick sem a semente) e escrita na bancada antes do estado de antes. Os 31 vetores novos (16 cenários, 3,9 MB) batem 307/307 com S, R e vida e são exigidos por `ticks_originais.rs`; a ação nova (`acoes/exp_leao_m1`) por `acoes_originais.rs`.

| Subpasta | Ticks | O que exercita |
|---|---|---|
| `morte_natural` | 1761, 1763, 1772, 1796, 1811, 1842 | `_HealthB` = `_HealthR` = 1 (`_AlwaysHealthy` = 1): morte natural de soldado preto no ninho e na superfície, operária vermelha na superfície (conta em `_BAntsExpired`, superficie.md 6.5), macho preto, rainha nova preta no ninho e macho vermelho |
| `rainha_fome` | 1775, 1776 | `_HealthB` = 0: a cabeça morre com `Sk(64)` = 0 (diálogo 10015 dentro do tick) e a cauda some no tick seguinte (`_BlkQueens` = 0) |
| `rainha_vermelha_fome` | 1757, 1758 | o mesmo para a vermelha (10016), com `_HealthR` = 0 reposto na entrada de cada tick (as vermelhas comem e a saúde sobe) |
| `ninho_n02` | 1711 | N-02: `_HealthB` = 0 e S = 0x00A5 na entrada: a cabeça da rainha sorteia `Sk(256)` = 0 e não morre |
| `lutas_soldado`, `lutas_castas` | 1793 | tabuleiros de pretas e vermelhas em Bored: dezenas de `_GetWinner` por tick |
| `ninho_n04`, `ninho_n05`, `ninho_n08` | 1719, 1721; 1711, 1714; 1713, 1718 | ovo levado ao fundo e largado com o mesmo estágio; vermelhas largando 0x82; operária e soldado cavando |
| `ninho_n11` | 1711, 1719, 1722, 1754 | linhas 6..63 inundadas sem chuva: afogadas (macho vermelho, operária preta), a água descendo e a comida da linha seca sumindo |
| `lista_a_cheia`, `lista_a_cheia_sem_vagas` | 1793 | `_ExitHole` com `_ListIndexA` = 1000 |
| `revoada_natural` | 3722, 3842, 3843 | machos e rainhas novas feitos pela própria colônia vermelha (rico em ultra até `_GstrR` armar `_FlyAwayR`): voo, decolagem por `S1(F)` com F = 5 e saída do mapa |
| `queda_casa_carpete_esq`, `queda_casa_carpete_dir` | 34 | aranha em agonia sobre um fiapo do carpete na casa: as 3 comidas caem para a direita (`_DROPdir` = 1) ou somem (`_DROPdir` = 3) |
| `amarela_rainha` | 1733 | rainha amarela andando no ninho: 0xFF e 0xFE regravadas a cada passo |

Gravações brutas inteiras comparadas antes da seleção (fora do git, `re_trabalho/raros/`): morte natural 150 ticks, rainhas 66 + 48, N-02 3, lutas 40 + 3 × 25, N-04/05/08 30 + 30 + 40, N-11 120, lista cheia 2, revoada natural 17 (mais 418 de uma tentativa em que `_FlyAwayR` não armou), rainha amarela 8, queda na casa 3 + 3 + 3 + 1, Load de um Jogo Completo sem vermelhas 40: **todas 307/307**, menos `ninho_n11/1797` (`_gameCycles`: a caixa de fome da amarela, `_YellowDialog`, não para o temporizador; como `ninho_n15`).

Achados:

1. **`_KillAntLion(-1)` grava 0x3F onde estava a formiga-leão que saiu (port corrigido).** A ação `exp_leao_m1` (Experimental pausado, duas formigas-leão na lista, inseticida em jato num 0x38 sem entrada): o 1º jato grava 0x3F em (20, 0) (x = byte alto do seletor de `_EggStr` nesta sessão, y = byte alto de `_LastThemeSong`) e tira a formiga-leão 0; o 2º grava 0x3F no centro dela (o X e o Y da vaga −1 agora são os dela) e tira a seguinte; o 3º, no centro desta; os demais repetem. O port não fazia o `_SetMap`. Correção: `Estado::leao_x_vaga_menos_1` (fora do save; `None` enquanto é o seletor) e `inimigos::matar_formiga_leao_sem_entrada` grava `(X[−1], byte alto de _LastThemeSong)`; o seletor da bancada (20) vai na ação (`leao_x_vaga_menos_1`). Inimigos.md §1.4.
2. **`_DROPdir` nos extras.** Fora do save; o comparador usava o 0 do port. `qemu_bancada.EXTRAS` passou a gravar `DROPdir` e `vetor_tick` põe `Estado::passo_queda` com ele. Lidos: 1 no patch (0, 4) (`_CarpetFloorL`) e 3 no (1, 5) (`_CarpetFloorR`), como em geracao-mundo.md §5.
3. **Proporção de vitórias** (superficie.md 6.2): as 250 chamadas de `_GetWinner` das gravações de luta (lidas no port sobre os ticks que batem 307/307, isto é, as mesmas do original): classe 0 × 0, "a" venceu 45/106 (0,42; tabela 0,5); 0 × 1, 12/42 (0,29; 0,2); 1 × 0, 16/24 (0,67; 0,8); 0 × 2, 17/23 (0,74; 0,7); 2 × 0, 0/3; 1 × 2, 20/25 (0,80; 0,9); 2 × 1, 2/21 (0,10; 0,1); 1 × 1, 3/4; 2 × 2, 2/2. Pelos contadores do save (`_RedLost`/`_BlackLost`): operárias × operárias, a preta venceu 40/70 (0,57); soldados pretos × operárias vermelhas, 43/66 (0,65); machos pretos × soldados vermelhos, 10/60 (0,17). Tudo dentro do esperado para amostras desse tamanho com `RR(10)` < p (o soldado contra a operária fica ~2 desvios abaixo de 0,8); a regra em si está conferida pela igualdade tick a tick (R e os contadores batem).
4. **Clique esquerdo no laço** (simulacao-visao-geral.md §2): `cenario_ritmo_clique`, em ultra, para na entrada do `MYTIMERFUNC` (`1000:2440`) a cada chamada (um passo de instrução antes de religar o breakpoint). Sem clique, 12–15 ticks por chamada (19 chamadas; uma de 8 e uma de 18). Com um clique na barra de título logo depois de soltar a VM, a chamada **seguinte** roda só 1–3 ticks (19 de 19), e a do clique 9–22: o evento chega ao Windows no fim da chamada em que foi enviado, e a seguinte sai na primeira volta em que testa `GetAsyncKeyState`. Confirma o curto-circuito do código e o que o port faz (`botao_esquerdo_apertado`).
5. **`_LastColonyPopR` volta a 1 no Load** (quintal.md, "Implementação"): `cenario_ultima_vermelha` (Jogo Completo sem vermelhas) mostrou 10052 e 10055 na passada de `_SimColonies` do tick 32; salvo como `ULTVERM` e carregado pelo File → Load, a passada do tick 64 mostrou as duas de novo (capturas `re_trabalho/cen/dialogo_32_*.png`, `dialogo_64_*.png`). Os 40 ticks batem 307/307 (o comparador não confere as caixas; o port tem o teste `ultima_populacao_vermelha_comeca_em_1`).

### Quinta rodada, tutorial (2026-10-18): tutorial completo com as ações do jogador

`tests/referencias/ticks/tutorial_completo/` (16 ticks, 2,1 MB; `cenarios_tick.py gravar tutorial_completo DIR --snapshot tut`, detalhes em [conselhos.md](conselhos.md) §3.1): o roteiro faz a ação que cada lição pede (cliques calculados, teclas, arrasto no Behavior, clique segurado no gráfico) e grava inteiros só os ticks em que `_TutorCnt` chega a múltiplo de 8; grava o snapshot `tutcur` a cada lição para retomar. Ações que dependem de coordenadas de outras janelas vêm do ambiente (`TUT_EXTRA`, JSON por lição). Duas gravações brutas (fora do git): 124 ticks das lições 1–26 (com o relógio lido depois da caixa) e 185 das lições 1–57.

**Resultado:** dos 309 ticks, todos 307/307 com extras 5/5 e vida 0, menos três casos com a causa isolada:
- `tick_000575` da 1ª gravação: `_CatDelay` diferente. No tick de `_FullCount` (`_Cycle` múltiplo de 64) a caixa da lição abre **antes** de `_DoSimYard`, e o gato leu o relógio depois de a caixa fechar; a gravação só tinha as leituras de antes da caixa (nenhuma), e o port usou `_gameCycles`. Regravado com `MacTickCount_depois_da_caixa`: 307/307 (está no repositório).
- `tick_001174`–`001229`: a bancada anotou a janela do mapa (0x100) na frente depois do botão Patch, mas o original não concluiu a lição 0x2A até o Ctrl+M (a janela não estava na frente). Erro de anotação, não do port; fora da seleção.
- `tick_001340`: a caixa com imagem da lição 50 fez o Windows descartar o segmento de código 6 (`_DoAntSim`) e o tick não voltou ao retorno conhecido; o "depois" foi lido cem ticks adiante. Inválido, fora da seleção.

**Divergência achada e corrigida (regra, pequena):** `_TimeTemp` das lições 7, 9 e 0x1A. `_GiveLesson` mostra a caixa e só depois lê `_MacTickCount` (tick 291: relógio 10704 no tick, 12213 depois da caixa, `_TimeTemp` = 12813). O port usava o relógio do tick (`_TimeTemp` = 11304). Correção em `rules/conselhos.rs` (`MemoriaTutor::espera_depois_da_caixa` e `caixa_fechada`), chamada por `Partida::fechar_modal` e, nos vetores, por `vetor_tick::depois_do_tick` (o teste de ticks confere `_TimeTemp`). Teste de unidade `prazo_da_licao_7_conta_do_fechamento_da_caixa`.

**Extras novos** (só em `antes`, todos opcionais): `JanelaNaFrente` (janela que a bancada pôs na frente, no formato de `_win_IsWinInFront`; vai para `RegrasOriginais::interface_tutor`), `VistaEdicao` (`[39, 21]`: a janela de edição do Tutorial é maior que a padrão de 25 × 19; usada por `aplicar_vista_da_bancada`) e, no nível de cima, `MacTickCount_depois_da_caixa`.

**Bancada (`cenarios_tick.py`):** `fechar_caixa_ate_o_retorno` também para na entrada do tick seguinte (estado de depois = o da entrada seguinte) e não exige o segmento 6 presente; clica mais embaixo nas tentativas seguintes (caixas com imagem); com `relogio` anota as leituras do relógio depois de a caixa fechar (só no tick de `_FullCount` ou quando pedido: `anotar_relogio_caixa`). Observado: a caixa da lição sem imagem não chama `_MacTickCount`.

## Limitações

- **Configurações cobertas.** Os primeiros vetores eram só do snapshot `quick` (Quick Game, velocidade normal, sem balões, chuva nem amarela em movimento). Hoje (2026-10-18) há 107 vetores em 21 grupos, de vários snapshots (`quick`, `rico`, `full`, `tut`, Experimental), com chuva, revoada, predadores, menino e cortador, inseticida, amarela andando e lutando, as quatro velocidades e o tutorial (lições 1–3 em `tutorial/` e o tutorial inteiro em `tutorial_completo/`), além das sequências longas `ticks_longos/rico_200` e `full_600` (esta com a Yard View na frente). Continuam sem vetor: balões ligados, a janela de edição atrás nas animações da amarela, e os ramos listados em [../../estado-do-projeto.md](../../estado-do-projeto.md) §3; nesses casos o desenho pode consumir S ou mudar o caminho do tick.
- **Velocidades "fast"/"ultra"**: há vetores (`velocidade_rapida`, `velocidade_ultra`; `_gameCycles` anda 1 por tick), mas o ritmo em ticks por segundo depende da CPU no original e só foi medido na bancada QEMU (ver `_BirdDelay` e simulacao-visao-geral.md §3.1).
- **Leituras:** o mapeamento cobre os segmentos de dados 8, 9 e 10 fora do save. Não cobre a memória do Windows (API chamada de dentro do tick) nem os mapas de vida (já capturados). Uma variável lida só em situações que não aconteceram nos 140 ticks não aparece.
- **`_Tindex` e `_TemDensity`**: classificados por inferência, não por teste.
- **Determinismo depende do snapshot**: o `loadvm` com a VM rodando chegou sempre ao mesmo primeiro tick nas execuções feitas, mas isso não é garantido pelo QEMU sem `icount` (o tempo até o primeiro `WM_TIMER` vem do relógio do host). Se uma execução parar num `GameTime` diferente de 79, compare por `GameTime`, não pela ordem.
- **Seletores** valem para `-m 16` e para a sessão salva no snapshot; o script sempre procura de novo.
- Tamanho: cada tick ocupa ~130 KB (2 × 48.386 + 2 × 16.384 + JSON). Versionados (2026-10-18): 107 ticks em `ticks/` (15 MB) e as duas sequências de `ticks_longos/` (1,1 MB, só os campos que mudam); outras sequências longas ficam em `re_trabalho/`.
