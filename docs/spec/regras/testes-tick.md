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

Mais `vida_antes.bin` / `vida_depois.bin`: `_LifeA` (8.192) + `_LifeB` (4.096) + `_LifeR` (4.096) = 16.384 bytes. Pela [especificação do save](../formatos/save-ant.md), `_DoLoadInitializations` reconstrói esses mapas das listas ao carregar; o port pode reconstruí-los do mesmo jeito e usar o arquivo só para conferir [hipótese: a reconstrução dá o mesmo mapa no meio da simulação; conferir com os vetores].

## Protocolo de teste

### Gerar vetores (original)

```
set SIMANT_RAIZ=C:\...\Simant recomp
python tools/gerar_vetores_tick.py --ticks 5                       # tests/referencias/ticks/
python tools/gerar_vetores_tick.py --ticks 3 --alvo 300 --semente-s 0x1234 --semente-r 1 --saida re_trabalho/v300
```

Saída por tick, em `tick_NNNNNN/` (NNNNNN = `GameTime` antes do tick):

| Arquivo | Conteúdo |
|---|---|
| `estado_antes.bin` | 307 blocos do save, formato `.ANT` (48.386 bytes), na entrada de `_DoAntSim` |
| `estado_depois.bin` | idem, no retorno de `_DoAntSim` |
| `vida_antes.bin`, `vida_depois.bin` | `_LifeA` + `_LifeB` + `_LifeR` (16.384 bytes) |
| `extras.json` | `{"antes": {...}, "depois": {...}}` com os campos da tabela acima |

`indice.json` guarda a origem: snapshot, sementes forçadas, alvo, a primeira entrada, os seletores e o endereço de retorno de `_DoAntSim`.

### Vetores no repositório

`tests/referencias/ticks/`: ticks 79 a 83 do snapshot `quick` (Quick Game, velocidade normal, janelas do início do jogo), sem forçar sementes. No tick 80 (`_Cycle` = 80, múltiplo de 16) a rainha preta põe um ovo (`TotalEggsLaidB` 3 → 4, lista B cresce). Repetibilidade conferida: os ticks 79–81 batem com os da execução E1 (exceto `_BirdDelay` a partir do tick 80, ver acima).

### Comparar o port

Para cada `tick_NNNNNN`:

1. carregar `estado_antes.bin` como um save, **sem** as inicializações de carga que mexem nos geradores (o `_LoadGame` do original chama `_RandYard`, que reinicia S; ver [rng.md](rng.md#a-semente-está-no-save));
2. pôr S, R, `_gameCycles`, `_GlobalKey` (e `_Tindex`) com os valores de `extras.json["antes"]`; montar os mapas de vida (reconstruir ou ler `vida_antes.bin`);
3. rodar **um** `_DoAntSim`;
4. gravar o estado no formato `.ANT` e comparar com `estado_depois.bin` (`tools/estado_diff.py`); comparar S e R com `extras.json["depois"]` e a vida com `vida_depois.bin`.

Os ticks consecutivos são encadeáveis: verificado que o `estado_depois` do tick N é igual ao `estado_antes` do N+1 (só `_gameCycles` muda no intervalo). Assim, um teste de N ticks pode partir do primeiro `estado_antes` e conferir cada `estado_depois`, desde que use o `_gameCycles` de cada `extras.json["antes"]`.

## Limitações

- **Configuração única.** Snapshot `quick`: Quick Game, velocidade normal, as janelas que o jogo abre sozinho, sem balões, sem chuva, sem a formiga amarela em movimento. Outras janelas, velocidades e eventos (aranha na tela, revoada, chuva) podem fazer o desenho consumir S ou mudar o caminho do tick.
- **Velocidades "fast"/"ultra"** não medidas (ver `_BirdDelay`).
- **Leituras:** o mapeamento cobre os segmentos de dados 8, 9 e 10 fora do save. Não cobre a memória do Windows (API chamada de dentro do tick) nem os mapas de vida (já capturados). Uma variável lida só em situações que não aconteceram nos 140 ticks não aparece.
- **`_Tindex` e `_TemDensity`**: classificados por inferência, não por teste.
- **Determinismo depende do snapshot**: o `loadvm` com a VM rodando chegou sempre ao mesmo primeiro tick nas execuções feitas, mas isso não é garantido pelo QEMU sem `icount` (o tempo até o primeiro `WM_TIMER` vem do relógio do host). Se uma execução parar num `GameTime` diferente de 79, compare por `GameTime`, não pela ordem.
- **Seletores** valem para `-m 16` e para a sessão salva no snapshot; o script sempre procura de novo.
- Tamanho: cada tick ocupa ~130 KB (2 × 48.386 + 2 × 16.384 + JSON). Por isso só 5 ticks estão versionados; sequências longas ficam em `re_trabalho/`.
