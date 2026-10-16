# Simulação: visão geral (tick, ordem, frequências, velocidade e pausa)

Este documento descreve a **estrutura** do laço da simulação do SimAnt para Windows: quem chama a simulação, quantas vezes por segundo, em que ordem as sub-rotinas rodam dentro de um passo ("tick") e com que frequência cada uma age. As regras de cada sub-rotina ficam em documentos próprios: [formigas.md](formigas.md) (listas, castas, modos), [feromonios.md](feromonios.md) (mapas de cheiro). Gerador de números aleatórios, geração do mundo e valores de tiles estão sendo escritos em paralelo (`rng.md`, `geracao-mundo.md`, `tiles-valores.md`).

## Convenção de evidência

- **Fato visto no código:** lido no pseudocódigo do Ghidra (`re_trabalho/decomp/Code*.c`), com endereço `seletor:offset` e nome do `SIMANTW.SYM`.
- **Fato verificado:** conferido em arquivo real (save `tests/referencias/saves/QUICK2.ANT`, `SIMANTW.EXE`) com script reproduzível.
- **[hipótese]** / **[em aberto]:** como no [README](../README.md).

Endereços de variáveis seguem o formato do `symbols.tsv`: `segmento lógico:offset`. Segmentos de dados: 8 = `SIMANT_DATA_GROUP` (seletor `1038`), 9 = `PACK` (`1040`), 10 = `DGROUP` (`1048`). O Ghidra mostra muitos acessos só pelo offset, sem o segmento. Quando o nome não sai do `.SYM`, o segmento foi deduzido do conteúdo do executável, e o texto avisa isso.

## 1. Quem dispara a simulação: o temporizador do Windows

**Fato visto no código.** Não há laço próprio de simulação. A simulação roda dentro do *callback* de temporizador `MYTIMERFUNC` (`1000:2440`, `SIMANT_MODULE`), registrado com `SetTimer`:

| Situação | Chamada | Fonte |
|---|---|---|
| Início do jogo, retomada e fim de diálogos | `SetTimer(_rootWnd, id 0, 17 ms, MYTIMERFUNC)` | `WINMAIN` `1000:3fda`; `_RestartSimulation` `1000:0034` (bytes `ff 36 78 cd 6a 00 6a 11 ff 36 42 00 ff 36 40 00 9a…`: empilha `_rootWnd`, 0, 0x11 e o ponteiro `_lpTimerFunc` `10:0040`) |
| Aplicativo perde o foco (`WM_ACTIVATEAPP` com wParam = 0) | o mesmo temporizador, refeito com **170 ms** (0xAA) | `MAINWNDPROC` `1000:2930` |
| Aplicativo volta ao foco | volta para 17 ms | `MAINWNDPROC` `1000:2930` |
| Diálogos, menus, salvar (`_SaveGame`) | `KillTimer` antes e `SetTimer` (17 ms) depois | `_DoUserButton` `1000:04c4`, `_DoMenuEntry` `1000:1876` |

`_StopSimulation` (`1000:0000`) e `_RestartSimulation` (`1000:0034`) usam um contador de aninhamento (DGROUP `0x0044`, sem nome no `.SYM`). O primeiro `Stop` mata o temporizador e descarta as mensagens `WM_TIMER` (0x113) pendentes. O temporizador só é refeito quando o contador volta a zero.

[hipótese] 17 ms imita o *tick* de 1/60 s do Macintosh (o jogo é port do Mac). No Windows 3.1, porém, o temporizador tem resolução de cerca de 55 ms, e as mensagens `WM_TIMER` não se acumulam. A cadência real precisa ser medida (ver "Como testar").

## 2. O que `MYTIMERFUNC` faz a cada chamada

**Fato visto no código** (`1000:2440`). Pseudocódigo próprio:

```
ao ser chamado:
  se não há janela principal: sair
  se a janela está minimizada (ícone):
      escolher o ícone conforme pausa / fim de jogo / vitória;
      se não está pausado nem em fim de jogo: sorteia SRand1(7) para animar o ícone   ← consome o RNG!
  atraso += 1                                   (DGROUP 0x02EE)
  repetir:
      _gameCycles += 1                          (9:9b2c, 32 bits)
      se não é cliente de rede (_SimAntClientFlag 8:8a9a == 0):
          pode_rodar = (não pausado  OU  (_MeMoveMe ≠ 0 E _CurGameTool < 10))
          limite = _SpeedDelayVals[_GameSpeed]
          se pode_rodar E (limite == -1 OU limite == 0 OU limite <= atraso) E não está reentrando:
              marca reentrada; atraso = 0
              _DoAntSim()                       ← UM tick da simulação
              _myServiceSong()
              desmarca reentrada
              se servidor de rede: _NetworkSend()
              se não minimizado:
                  se o app está ativo: _UpdateWindows()
                  senão: _UpdateWindows() só quando (quadros & 7) == 0
              quadros += 1                      (DGROUP 0x02F0, 32 bits)
      senão (cliente de rede): processa o pacote recebido (_ProcessPost) e redesenha
      se ativo e não minimizado: mensagem temporária do mapa e _UpdateYardMessage()
      enquanto houver WM_TIMER pendente: retira-a; atraso += 1; drenadas += 1
      iterações += 1; _myServiceSong()
      sair se: sem janela principal
            OU (drenadas > 1 E (iterações > 2 OU drenadas > 2))
            OU botão esquerdo do mouse foi apertado (GetAsyncKeyState(VK_LBUTTON) & 1)
```

Consequências:

1. **Um tick por iteração, no máximo.** Cada `_DoAntSim` é um passo; não há passo fracionário.
2. **Laço ocupado.** Em velocidade alta, o callback roda vários ticks seguidos e só devolve o controle ao Windows depois de ver chegar pelo menos 2 novas `WM_TIMER` (ou 3), ou um clique. [hipótese] Isso faz a taxa de ticks em "fast"/"ultra" depender da CPU.
3. **O atraso conta mensagens de timer**, não milissegundos: +1 na entrada do callback e +1 por `WM_TIMER` drenada dentro do laço.

Contadores usados aqui (DGROUP, sem nome no `.SYM`): `0x02EE` atraso desde o último tick; `0x02F0/0x02F2` quadros (32 bits); `0x02F4` trava de reentrada.

## 3. Velocidade (menu Speed)

**Fato visto no código.** `_GameSpeed` (`8:8614`) indexa `_SpeedDelayVals` (`8:8616`, 4 palavras com sinal). Valores lidos do `SIMANTW.EXE` (segmento 8, offset 0x8616): `6, 2, 0, -1`. A ação de menu em `_DoUserButton` (`1000:04c4`, caso 6) avança a velocidade em ciclo 0→1→2→3→0 e mostra a mensagem correspondente (strings do DGROUP `0x014B`, `0x0160`, `0x0177`, `0x018C`).

| `_GameSpeed` | Mensagem | `_SpeedDelayVals` | Regra do tick | Redesenho do mapa em `_UpdateWindows` (`1000:2224`) |
|---|---|---|---|---|
| 0 | "Game speed now slow." | 6 | roda quando atraso ≥ 6 | todo tick |
| 1 | "Game speed now normal." | 2 | roda quando atraso ≥ 2 | todo tick |
| 2 | "Game speed now fast." | 0 | roda em toda iteração do laço | 1 a cada 2 (contador DGROUP `0x9B00`, ímpar pula) |
| 3 | "Game speed now ultra." | -1 | roda em toda iteração do laço | 1 a cada 4 (`0x9B00 & 3`) |

- Valor inicial: `_InitSimVars` (`1030:5a70`) põe `_GameSpeed = 1` (normal). **Verificado:** `QUICK2.ANT` traz `_GameSpeed = 1`.
- [hipótese] Com o timer real de cerca de 55 ms, "slow" dá cerca de 3 ticks/s e "normal" cerca de 9 ticks/s. "fast" e "ultra" rodam à velocidade da CPU; "ultra" só desenha menos.
- Impacto no port: a velocidade deve virar um **intervalo entre ticks** configurável (dado, não código), com o padrão medido no original. A lógica do tick não depende da velocidade.

## 4. Pausa

**Fato visto no código.**

- `_GamePaused` (`9:9c28`) é alternado pelo menu (`_DoUserButton`, caso 5) e aplicado por `_SetPause`/`_PauseGame` (`1000:4e34`/`1000:4cc2`). Eles ajustam a mensagem ("Game Paused", strings 4/1010), os itens de menu e encerram os modos de ferramenta 10 (troca de corpo, `_EndLifeTransferMode`) e 11 (alvo, `_EndTargetMode`) ao despausar.
- **Exceção:** com o jogo pausado, o tick **continua rodando** se `_MeMoveMe` (`9:7d24`) ≠ 0 e `_CurGameTool` (`9:7c22`) < 10. O tick inteiro roda (`_DoAntSim` completo), não só a formiga amarela. [hipótese] É o "andar pausado" da formiga amarela até o destino clicado; precisa de teste no original.
- Fim de jogo (`_IsGameOver`, `9:9dda`) não para o temporizador. `_DoAntSim` chama `_EndGameDialog(0)` no fim do tick enquanto a flag está ligada.

## 5. Um tick: `_DoAntSim` (`1028:0000`)

**Fato visto no código.** Ordem exata das ações dentro de um tick. `_DoAntSim` tem embutidos (inline) os corpos de `_DoSmells` (`1028:02fa`), `_FeedAnts` (`1028:0474`), `_ClrModePop` (`1028:034a`) e `_TallyModePop` (`1028:038e`), que existem como funções separadas mas não são chamadas em outro lugar.

| # | Ação | Condição / frequência |
|---|---|---|
| 1 | `_Cycle` (`9:75fc`) += 1; se passar de 0x1000, volta a 0 | todo tick (o ciclo tem 4097 valores) |
| 2 | `_GameTime` (`9:76a8`, 32 bits) += 1 | todo tick |
| 3 | zera contadores de balões de texto (luta/descanso/rainha/ovo) | só se `_OptionStates[5]` (`8:85fc`) == 1 ([hipótese] opção "balões de fala"; a mesma flag liga `_FightBalloons`/`_RestBalloons`) |
| 4 | **alimentação** (corpo de `_FeedAnts`): `_HealthB` −1 (se não `_AlwaysHealthy`), `_HealthR` −1, mínimo 0; se o jogo não é Experimental (`_CurGameType` ≠ 3) e `_FoodA` < `_NextFeed`: `_AddFood(150, 1)` e `_NextFeed = SRand1(50) + 1` | `_Cycle & 0x3F == 0` (a cada 64 ticks) |
| 5 | **manutenção em 4 fases** (corpo de `_DoSmells`), fase = `(_Cycle & 0x60) >> 5`: | `_Cycle & 0x1F == 0` (a cada 32 ticks; cada fase a cada 128) |
| 5a | fase 0: `_CompactListA`, depois `_FullCount` (= `_CountAnts` + `_Feedback`), `_HistUpdate`, `_SmoothAlarm` | `_Cycle mod 128 == 0` |
| 5b | fase 1: `_CompactListB`, `_FillHolesBN`, `_ColonySmellBN`, `_ColonySmellBT` | `_Cycle mod 128 == 32` |
| 5c | fase 2: `_FullCount`, `_HistUpdate`, `_SmoothAlarm` | `_Cycle mod 128 == 64` |
| 5d | fase 3: `_CompactListR`, `_FillHolesRN`, `_ColonySmellRN`, `_ColonySmellRT` | `_Cycle mod 128 == 96` |
| 6 | `_DoSimYard` (`1030:14a8`): quintal (menino, pássaro, gato, cachorro, chuva) e, dentro dele, `_SimColonies` | todo tick (detalhes na seção 6) |
| 7 | `_DoWater` (`1020:07c6`) | todo tick; só age com `_RainOn` e terreno de quintal (`_TERRAINset` = 0) |
| 8 | `_DoAntLions` (`1030:43fa`) | todo tick (sorteios internos) |
| 9 | `_MoveSpider` (`1020:4704`) | todo tick (contador `_Scycle2`, `9:77ac`, módulo 1024) |
| 10 | `_DoPillar` (`1030:4cdc`): lagarta; chama `_DoSow` no começo | só em ticks com `_Cycle` ímpar; nada faz na casa (`_TERRAINset` = 1) |
| 11 | `_GetStrategy` (`1030:0000`): modo estratégico preto e vermelho, `_SetCasteProd`, `_SetModeProd` | todo tick |
| 12 | zera `_TemBModePop` (`9:786a`) e `_TemRModePop` (`9:7be4`), 20 palavras cada (corpo de `_ClrModePop`) | todo tick |
| 13 | `_FlyAwayB` (`9:7c44`) e `_FlyAwayR` (`9:8078`) −1 se ≠ 0 (o código testa ≠ 0, não > 0: um valor negativo continuaria caindo) | todo tick |
| 14 | `_DoAntSimA` (`1028:04d8`): formigas da superfície | todo tick |
| 15 | `_DoAntSimB` (`1028:2d4e`): ninho preto | todo tick |
| 16 | `_DoAntSimR` (`1028:5344`): ninho vermelho | todo tick |
| 17 | `_DoAntSimY` (`1028:7cf6`): estado da formiga amarela (vida, ovos se for rainha) | todo tick |
| 18 | resumo de modos (corpo de `_TallyModePop`): monta `_ModePopB`/`_ModePopR` a partir de `_TemBModePop`/`_TemRModePop` (ver [formigas.md](formigas.md#modos)) | todo tick |
| 19 | se nenhuma formiga vermelha está no modo 19 ("Avenger"; `_TemRModePop[19]`, `9:7c0a`, < 1): `_MakeRedInitiator` | todo tick |
| 20 | `_DoAntMoveY` (`1028:75f4`): movimento da formiga amarela | todo tick; só age com `_MeMoveMe` ≠ 0 |
| 21 | `_Feedback` (`1030:78ca`): tutorial e mensagens de conselho | todo tick |
| 22 | se `_IsGameOver`: `_EndGameDialog(0)` | todo tick |
| 23 | `_gGameNeedsSaving` (`8:8370`) = 1 | todo tick |

Observações:

- **Ordem de iteração das listas:** `_DoAntSimA/B/R` percorrem as listas **de trás para frente** (do índice `_ListIndexX − 1` até 0), num índice global `_Tindex` (`9:9b6a`). Formigas com tipo 0 (mortas) continuam ocupando a vaga até a próxima compactação. A ordem afeta quem se move primeiro e a sequência de números aleatórios.
- `_CountUpdate` (`1020:07c0`) só chama `_Feedback` e não aparece no tick. A contagem completa é `_FullCount` (fases 0 e 2).
- A fase 0 compacta a lista A e **também** roda `_FullCount`/`_HistUpdate`/`_SmoothAlarm`; a fase 2 roda só os três últimos. Contagem, histórico e alarme rodam, portanto, a cada 64 ticks.
- `_DoSpider` não existe no `.SYM`. A aranha é `_MoveSpider`.
- `_GameTime` só é incrementado aqui. **Verificado:** em `QUICK2.ANT`, `_GameTime = 78` e `_Cycle = 78` (o jogo começa com ambos em 0; `_RandWorld` `1030:6086` zera `_Cycle`).

### 5.1 Contagem, histórico, resumo de modos e compactação

**Fato visto no código** (pseudocódigo do Ghidra e, onde indicado, desmontagem do `SIMANTW.EXE`).

**`_CountAnts` (`1020:04de`)**, chamado por `_FullCount` (`1020:07b4`, que depois chama `_Feedback`):

1. Zera `_CastePopB[0]`, `_CastePopR[0]` e os 32 contadores de `_TemCensus` (`9:7c4e`).
2. Para cada lista (A, B, R, nessa ordem), percorre as vagas de `_ListIndex − 1` até 0 e, se T ≠ 0, soma 1 em `_TemCensus[T >> 3]` (pretas nos índices 0–15, vermelhas em 16–31, pelo bit 7).
3. Se `_MeMode` = 0, soma a amarela em `_TemCensus[(_MeType | cor) >> 3]`, com cor = 0x80 se `_MeColor` ≠ 0 (o Ghidra mostra a cor como `_grndTile+2`; o endereço é `10:ce98`, `_MeColor`).
4. Pretas: `_CastePopB[0]` = k0; `[1]` = k1 + k2 + k3 + k5; `[2]` = k6 + k7 + k9; `[3]` = k4; `[4]` = k8 (kN = `_TemCensus[N]`).
5. **Morte da rainha preta:** se o `_CastePopB[5]` anterior ≠ 0, k12 = 0 e `_JustXfered` = 0: música 0x2B0C e mensagem 10010; se `_CurGameType` < 2, também mensagem 10011, `_IsGameOver` = 1 e `_BlackWon` = 0. Depois `_CastePopB[5]` = k12.
6. Vermelhas: o mesmo com k16…k28 em `_CastePopR`; na morte da rainha vermelha, música 0x2B0D, mensagens 10012 e (Tutorial/Quick) 10013, com `_BlackWon` = 1. **Além disso**, se `_CurGameType` = 2, `_ColonyTotalRed` = 1 e `_CurYardPnt` = (11, 8): `k = SRand1(6)` e o byte `8:0184 + 16·k` recebe 0x14. Esse endereço é `_YMapPopR` (`8:0164`) + 0x20 + 16·k, ou seja, 20 formigas vermelhas no patch (k + 2, 0). Fato visto na desmontagem: o seletor vem de `DGROUP:0xc2e4`, que a relocação NE liga ao segmento 8. [hipótese] Leitura: surge uma nova colônia vermelha na casa.
7. `_BpopT` = soma de `_CastePopB[1..5]`; `_RpopT` idem; `_JustXfered` = 0.

A contagem vai só até `_ListIndex − 1`; a reconstrução da vida no load lê até `_ListIndex`, inclusive (ver save-ant.md).

**`_HistUpdate` (`1000:9182`)**: com `i = _HistStart`, grava `_H_BPop[i] = _BpopT`, `_H_RPop[i] = _RpopT`, `_H_BFood[i] = _FoodB`, `_H_RFood[i] = _FoodR`, `_H_BHeal[i] = _HealthB`, `_H_RHeal[i] = _HealthR`, `_H_FoodA[i] = _FoodA`, `_H_ADead[i] = _BAntsExpired`, `_H_PDead[i] = _BAntsEaten` e `_H_FDead[i] = _BAntsKilled` (dos contadores de 32 bits, só a palavra baixa). Depois faz `_HistCnt += 1` se < 63 e `_HistStart = (_HistStart + 1) & 63`. O resto da função redesenha o gráfico se a janela 0x1500 estiver aberta (interface). `_ClearHistory` (`1000:8cfc`) zera as dez séries e põe `_HistStart = 63` e `_HistCnt = 0`.

**`_TallyModePop`** (embutido no passo 18): `_ModePopB` = (m2 + m3, m4 + m5, m1, m7, m12, m6), com mN = `_TemBModePop[N]`; `_ModePopR` igual, com `_TemRModePop`. Como `_ClrModePop` zera as contagens no passo 12 e só as formigas processadas nos passos 14–17 contam, um tick sem formigas simuladas deixa `_ModePop*` = 0.

**`_CompactListA/B/R`** (`1020:2a16`/`2a7a`/`2ade`): percorre as vagas 0…`_ListIndex − 1`; cada vaga com T ≠ 0 é copiada (X, Y, M, T, S) para a primeira posição livre à frente; `_ListIndex` diminui do número de vagas com T = 0. As vagas do fim que ficaram para trás **não são apagadas** e continuam no save como lixo.

### Frequências resumidas

| Periodicidade | O quê |
|---|---|
| todo tick | `_DoSimYard`, `_DoWater`, `_DoAntLions`, `_MoveSpider`, `_GetStrategy` (+ `_SetCasteProd`, `_SetModeProd`), `_DoAntSimA/B/R/Y`, `_DoAntMoveY`, `_Feedback`, decremento de `_FlyAway*` |
| a cada 2 ticks (`_Cycle` ímpar) | `_DoPillar` (+ `_DoSow`) |
| a cada 32 ticks (`_YardCycle & 0x1F == 0`) | `_SimColonies` |
| a cada 64 ticks | `_FeedAnts` embutido (saúde −1, comida nova), `_FullCount`, `_HistUpdate`, `_SmoothAlarm` |
| a cada 128 ticks | `_CompactListA`, `_CompactListB` + cheiros pretos, `_CompactListR` + cheiros vermelhos |
| a cada 64 ticks, só formiga amarela | `_DoAntSimY`: vida −1 (`_Cycle & 0x3F == 0`) |
| a cada 16 ticks | rainha (preta/vermelha, e amarela se for rainha) pode pôr ovo (`_Cycle & 0xF == 0`) |
| a cada 32 ou 128 ticks | ovo avança um estágio (`_SimEggB`: 32 se `_BpopT` < 3, senão 128) |

## 6. Quintal e colônias (`_DoSimYard`, `_SimColonies`)

**Fato visto no código.**

- `_DoSimYard` (`1030:14a8`): `_YardCycle` (`9:7a5c`) += 1, voltando a 0 depois de 0x3FF. `_InitSimYard` (`1030:1378`) o inicia em −1, e por isso `_YardCycle = _GameTime − 1` no começo (**verificado** no `QUICK2.ANT`: 77 e 78). Depois chama o menino (`_SimKidInside` se `_NodeNum` < 0x26, senão `_SimKidOutside`), `_SimBird`, `_SimCat` e `_SimDog` (este só se a casa não está "à venda", `_ForSaleState` = 0). Começa a chover se `_GlobalKey` (`9:7866`) = 0xA8 ou se `RRand(32) == 0` e `RRand(128) == 0` (1/4096 por tick). Duração: `_RainCnt = SRand1(150) + 150` ticks. No fim chama `_SimColonies`.
- `_SimColonies` (`1030:35f8`) só age quando `_YardCycle & 0x1F == 0`. Ele atualiza o mapa estratégico 16×12 de colônias (`_YMapPopB` `8:00a4`, `_YMapPopR` `8:0164`), a expansão e extinção de colônias vizinhas, a revoada (`_SwarmCnt*`, `_QueenStorage*`) e as condições de vitória e derrota do Jogo Completo. As regras ficam para um documento próprio [em aberto]; as condições de fim de jogo estão na seção 8.

## 7. Estratégia (`_GetStrategy`, todo tick)

**Fato visto no código** (`1030:0000`, `_GstrB` `1030:01cc`, `_GstrR` `1030:03c2`).

- Calcula a posição "difusa" da formiga amarela para quem a segue: `_FuzLocY = MeLocY + SRand1(5) − 2`, limitado a 0..63. O mesmo cálculo é feito para X, mas no fim da função `_FuzLocX` é sobrescrito com `_MeLocX` (o sorteio de X é descartado; os dois `SRand1(5)` são consumidos mesmo assim). Liga `_ChaseSpid` se a aranha estiver a menos de 100 de distância. Esta parte só roda com a amarela na superfície (`_MePlane` = 1).
- `_StrategicModeB` (`9:9b8a`), com S = `_HealthB`, P = `_BpopT`, D = `_TilesDugB`:

  | Resultado | Condição (avaliada nesta ordem) |
  |---|---|
  | 0 (ataque) | S ≤ 9 e P/2 > `_RpopT` > 0 e `_RedQueens` > 0 (desespero), **ou** nenhuma das linhas abaixo vale |
  | 5 | S < 30 |
  | 4 | S < 50 |
  | 3 | D < P |
  | 2 | D < 2P |
  | 1 | P < 101, ou `_RpopT` < 1, ou `_RedQueens` < 1, ou P/3 ≤ `_RpopT` |

  Leitura [hipótese]: 5/4 = fome, 3/2 = falta escavar, 1 = normal, 0 = atacar com folga.
- `_StrategicModeR` (`9:7690`) = `_GstrR()`: mesma escada para os vermelhos, com duas diferenças. Primeiro, um gatilho de **revoada** vermelha: se `_FlyAwayR` = 0 e machos + rainhas novas vermelhos > 20, então `_FlyAwayR = 200`. Segundo, um modo "malvado" (`_JustToBeMean`, `9:78dc`): quando a saúde vermelha está muito baixa e os vermelhos estão bem mais numerosos, ou por sorteio raro (`SRand32` e `SRand128`), ele força o modo 0 por `SRand1(100) + 30` chamadas, toca música e mostra a mensagem 4/1020 nº 6 ("The red ants are attacking!").
- Em seguida, `_SetCasteProd` e `_SetModeProd` escolhem a casta do próximo ovo e o modo das próximas operárias (ver [formigas.md](formigas.md#controle-de-castas-e-modos)).

## 8. Fim de jogo e pontuação

### Condições de fim (fato visto no código)

`_IsGameOver` (`9:9dda`) = 1 encerra o jogo; `_BlackWon` (`9:9cea`) diz o resultado. Mensagens em SHARED tipo 4 (ids decimais):

| Onde | Condição | Tipos de jogo | Resultado | Mensagens |
|---|---|---|---|---|
| `_CountAnts` `1020:04de` (a cada 64 ticks) | a rainha preta existia na contagem anterior e sumiu (e não houve troca de corpo, `_JustXfered` = 0) | Tutorial e Quick (`_CurGameType` < 2) | derrota | 10010 "The black queen in this patch has died.", 10011 "Sorry, you have been defeated by the reds." |
| idem | a rainha vermelha sumiu | Tutorial e Quick | vitória | 10012 "The red queen…", 10013 "Congratulations! You have defeated the red menace…" |
| `_YellowDeath` `1000:7746` | a amarela morre e não há outra formiga preta (ovo, depois adulto no ninho B, na superfície ou no ninho R) para reencarnar | Tutorial e Quick | derrota | — |
| idem | idem, no Jogo Completo | Full (2) | se houver mais de uma colônia preta, "transfere" (`_SpecialXfer`, 10058); senão, derrota (10056) | 10058, 10056 |
| `_SimColonies` `1030:35f8` (a cada 32 ticks) | casa "à venda" (`_ForSaleState` ≠ 0) **e** nenhuma colônia vermelha | Full | vitória | 10057 "…you have won the Full Game." |
| idem | ≤ 1 colônia preta, sem rainha, sem ovos, sem machos e sem rainhas novas | Full | derrota | 10056 "The black ants have no queens, breeders or eggs remaining…" |

Mensagens sem fim de jogo, todas em `_SimColonies` no Jogo Completo:

- Os vermelhos somem do mapa (havia colônia vermelha na passada anterior e agora não há): 10052 ("…red ants have been eliminated from the yard"). Se a casa ainda não está à venda, vem também 10055 ("Now if you can take over the house…").
- Mais de 24 células de casa ocupadas (`_HouseColonies` > 0x18): `_ForSaleState = 1` e 10054 ("…the humans have been driven out of the house"). Se ainda há vermelhos, vem também 10053 ("Now you just have to eliminate the red ants…").

Fora daí, `_SimQueenB` mostra 10015 ("The black queen in this patch starves…") quando a rainha preta morre por `_HealthB` = 0.

`_CurGameType` (`9:80b4`): 0 = Tutorial, 1 = Quick, 2 = Full, 3 = Experimental (ordem da lista 4/1001 e dos usos no código; **verificado:** `QUICK2.ANT` traz 1).

### Pontuação: `_CalcScore` (`1000:5530`)

**Fato visto no código**, com ressalvas sobre a aritmética de 32 bits. São 8 notas de 0 a 100:

| i | Nota | Cálculo |
|---|---|---|
| 0 | saúde média | média de `_H_BHeal` nas últimas `_HistCnt` amostras |
| 1 | comida relativa | 100 × Σ`_H_BFood` / Σ`_H_RFood` (0 se não há comida vermelha) |
| 2 | ovos que vingaram | 100 × (`_TotalEggsLaidB` − `_TotalEggsDiedB`) / `_TotalEggsLaidB` (100 se nenhum foi posto) |
| 3 | baixas | 100 × `_BlackLost` / (`_BlackLost` + `_RedLost`) (100 se não houve luta) [em aberto: o sinal do peso; do jeito que está, perder mais formigas pretas aumentaria a nota] |
| 4 | colônias fundadas | só Full e Experimental: 100 × `_BColoniesStarted` / (B + R fundadas) |
| 5 | colônias sobreviventes | só Full e Experimental: 100 × fundadas / (fundadas + `_BColoniesKilled`) |
| 6 | quintal ocupado | só Full e Experimental: 100 × (células não nulas de `_YMapPopB` na área do quintal) / 155. Área do quintal: colunas 0–4, linhas 3–11, e colunas 5–15, linhas 2–11; 5×9 + 11×10 = 155 |
| 7 | casa ocupada | só Full e Experimental: 100 × (células não nulas nas linhas de casa) / 37. Área da casa: linhas 0–2 nas colunas 0–4 e linhas 0–1 nas demais; 15 + 22 = 37 |

[em aberto] A contagem de `_HouseColonies` em `_SimColonies` usa outra área: linhas 0–1, mais a linha **3** nas colunas 0–4. A pontuação usa a linha 2. Pode ser um defeito do original.

Total = `_MeHealth` + Σ peso[i] × nota[i] × 51, com pesos `13, 17, 19, 23, 29, 31, 37, 41` (8 bytes no DGROUP, offset 0x0948; o segmento foi deduzido do conteúdo). Depois:

- Full: se `_GameTime` < 8100, total = total × max(1, `_GameTime`/100) / 81; depois × 5.
- Outros tipos: total = total × 29/10; se `_GameTime` < 4100, × max(1, `_GameTime`/100) / 41.

[em aberto] O Ghidra mostra o retorno como `_GameTime + total`, o que provavelmente é erro de leitura do retorno de 32 bits (DX:AX). Os dois limites (8100 e 4100) aparecem no pseudocódigo como "ponteiro de string + deslocamento" (`s_NetBIOS__Delete_Name__1048_1f96 + 0xe` = 0x1FA4 e `s_DeltaTime___lu_1048_0ff5 + 0xf` = 0x1004); é o Ghidra interpretando constantes como endereços. O valor exibido deve ser conferido contra a tela de pontuação do original.

## 9. Pontos de atenção para o determinismo do port

1. **RNG fora do tick.** Com a janela minimizada, `MYTIMERFUNC` chama `SRand1(7)` a cada callback (ícone animado). Na bancada, nunca minimize o jogo.
2. **Taxa variável.** O número de ticks por segundo depende do timer e da CPU. Compare sempre **por tick** (`_GameTime`), nunca por tempo de relógio.
3. **Ordem de iteração reversa** das listas e uso de `_Tindex` global.
4. **Sorteio por vaga, não por formiga viva:** em `_DoAntSimA`, `SRand256()` é chamado para **toda** vaga da lista, inclusive as de tipo 0, antes de testar se a formiga existe.
5. **Pausa com movimento:** ver seção 4.

## 10. Como testar contra o original

Bancada: [docs/bancada-qemu.md](../../bancada-qemu.md).

1. **Taxa de ticks por velocidade.** Comece um Quick Game, salve (`Ctrl+A`), espere N segundos sem mexer, salve de novo com outro nome e compare `_GameTime` (offset 45740 do `.ANT`, 4 bytes). Repita nas 4 velocidades (o menu Speed alterna em ciclo). Resultado esperado: razão de cerca de 3:1 entre slow e normal [hipótese]; fast e ultra variam com a CPU do QEMU (TCG).
2. **Sequência de fases.** Em saves sucessivos, confira os efeitos periódicos:
   - `_HistCnt` sobe 1 a cada 64 ticks; `_ClearHistory` (`1000:8cfc`) inicia `_HistStart = 63` e `_HistCnt = 0`. **Verificado:** em `QUICK2.ANT`, com `_GameTime` = 78, `_HistStart = 0` e `_HistCnt = 1` (uma chamada, no tick 64).
   - `_HealthB` cai 1 a cada 64 ticks sem comida.
   - O mapa `_PherMapBN` recebe 255 nos buracos no tick ≡ 32 (mod 128) e logo depois perde 1. **Verificado:** em `QUICK2.ANT` (tick 78) há exatamente uma célula com 254 no `_PherMapBN`.
3. **Comparação tick a tick com o port.** Use o `.ANT` como "foto" do estado: carregue o mesmo `.ANT` no original e no port, avance o mesmo número de ticks e compare os 307 blocos. Os campos do quintal e do cheiro já ficam estáveis entre ticks; listas e RNG exigem a semente igual (ver `rng.md`).
4. **Dumps de memória.** `pmemsave` grava memória **física**. No modo 386 avançado, os segmentos de 50 KB não ficam contíguos na RAM física. **Observado** em `re_trabalho/qemu_ram_quick2.bin`: a assinatura `_Dx8/_Dy8` + `_TurnTab` aparece em 0x22F700, mas as listas (`8:23a4…`) não estão no deslocamento esperado a partir dali. Para dumps úteis, resolva a base linear do seletor (LDT do Windows) e use `memsave` (endereço virtual) do QEMU, ou compare só dentro de uma página de 4 KB. Assinaturas para achar os segmentos: `_Dx8+_Dy8` (`00 01 01 01 00 ff ff ff ff ff 00 01 01 01 00 ff`, início do segmento 8) e a tabela de luta (`00 00 00 00 02 00 01 01 02 01 00 00 03 03 00 00 05 02 07 03…`, DGROUP 0x22C6).
5. **Pausa com movimento.** Pause, clique num destino para a amarela e salve depois de alguns segundos: se `_GameTime` avançou, a exceção da seção 4 está confirmada.

## Implementação no port

Onde está (crate `engine`, módulo `rules`):

- **`rules/estado.rs`: `Estado`.** Guarda todos os 307 blocos do `.ANT` em grupos tipados por assunto, com o símbolo original no comentário de cada campo:
  - `partida`: `_GameTime`, `_Cycle`, tipo de jogo, pausa, velocidade, fim de jogo, cadáveres...;
  - `ambiente`: terreno, `_Barrier`, chuva, `_FoodA`, `_NextFeed`, `_YardCycle`, patch, sementes;
  - `superficie`, `ninho_b` e `ninho_r`: mapa, mapa de saída, vida e lista. Cada lista guarda **todas** as vagas da capacidade, mais o `_ListIndex`;
  - `feromonios`: os cinco mapas, mais `_PherMapF`;
  - `preta` e `vermelha` (`Colonia`): saúde, comida, castas, modos, revoada, escavação, buracos, contadores de mortes...;
  - `amarela` (`_Me*`), `lider_vermelha`, `formigas_leao`, `aranha`, `lagarta_e_bichos`, `menino`, `bichos`, `controles`, `tutorial`, `vistas` e `historico`.

  Campos só de execução: os geradores `s` e `r`, `censo` (`_TemCensus`), `modos_do_tick` (`_TemBModePop`/`_TemRModePop`), `tindex`, `baloes`, `tecla_global` (`_GlobalKey`), `precisa_salvar` e a fila `eventos` (mensagens, músicas e o diálogo de fim de jogo). As regras não chamam áudio nem janela: deixam o pedido na fila.
  - `Estado::de_save(&JogoSalvo)` lê os blocos pelo nome do símbolo e reconstrói `_LifeA/B/R` como `_DoLoadInitializations` (`core/vida.rs`). Os geradores não estão no save. Por convenção, S começa na semente do patch atual e R no estado inicial do runtime C [hipótese: depois de um load, o original não é reproduzível; ver rng.md].
  - `Estado::para_save(&LayoutSave)` grava de volta. A ida e volta é **byte a byte** (verificado com `QUICK2.ANT` e `QUICK3.ANT`).
  - `Estado::de_mundo(&Mundo, s, r, &ConfigSimulacao)` monta o estado de um jogo novo a partir da geração. Aplica os passos de `_NewGame`, `_RandYard`, `_RandWorld` (passo 18), `_InitSimVars`, `_initControls`, `_ClearHistory` e `_CountAnts`.
  - Rotinas: `contar_formigas` (`_CountAnts`), `resumir_modos` (`_TallyModePop`), `atualizar_historico` (`_HistUpdate`), `ListaFormigas::compactar` (`_CompactList?`) e `adicionar_formiga` (`_AddAntTo?List`, com o mapa de vida).
- **`rules/tick.rs`: `tick(&mut Estado, &mut dyn Regras)`.** Segue a tabela da §5 na ordem exata. O próprio tick faz os passos 1–5, 12, 13, 18, 22 e 23, o teste do passo 19 e a alimentação periódica (o `_AddFood` é delegado). As sub-rotinas de cada assunto são métodos de seis traits, todos com implementação padrão vazia:

  | Trait | Métodos | Spec |
  |---|---|---|
  | `RegrasQuintal` | `simular_quintal` (`_DoSimYard` + `_SimColonies`), `simular_agua` (`_DoWater`) | §6, ninho.md §8 |
  | `RegrasInimigos` | `simular_formigas_leao`, `mover_aranha`, `simular_lagarta` (`_DoPillar` + `_DoSow`) | inimigos.md |
  | `RegrasSuperficie` | `simular_superficie` (`_DoAntSimA`), `adicionar_comida` (`_AddFood`), `escolher_estrategia` (`_GetStrategy`), `criar_lider_vermelha` (`_MakeRedInitiator`) | superficie.md |
  | `RegrasNinho` | `simular_ninho(cor)` (`_DoAntSimB`/`_DoAntSimR`) | ninho.md, ciclo-de-vida.md |
  | `RegrasAmarela` | `simular_amarela` (`_DoAntSimY`), `mover_amarela` (`_DoAntMoveY`) | formiga-amarela.md |
  | `RegrasConselhos` | `dar_conselhos` (`_Feedback`) | [em aberto] |

  `Regras` é a soma das seis traits. `RegrasOriginais` é a implementação do jogo. No fim de `tick.rs` há uma linha `impl Regras<Parte> for RegrasOriginais {}` vazia para cada parte.

  **Para plugar uma parte:** em `rules/<assunto>.rs` (por exemplo, `ninho.rs`), escreva `impl RegrasNinho for RegrasOriginais { fn simular_ninho(...) {...} }`. Depois apague a linha vazia correspondente em `tick.rs` e acrescente `pub mod <assunto>;` em `rules/mod.rs`. Tabelas de dados de que as regras precisarem entram como campos de `RegrasOriginais` (hoje: `geracao`, com o seno de `_AddFood`).
  - Os pontos de feromônio do tick (`_FillHoles*`, `_ColonySmell*`, `_SmoothAlarm`) são funções privadas vazias marcadas com `TODO(feromonios.md)`, a ligar a `rules/feromonios.rs`.
  - `simulacao_liberada` (pausa, §4) e `Relogio` (atraso contado em mensagens de temporizador e `_SpeedDelayVals`, §2–3) decidem **quando** rodar um tick. A lógica do tick não depende da velocidade.
- **Dados:** `data/simulacao.json` (`ConfigSimulacao`). Contém `atrasos_velocidade` (`_SpeedDelayVals` = 6, 2, 0, −1) e os valores iniciais de jogo novo que não saem da geração:
  - triângulos e conjuntos memorizados, `_ModeAuto`/`_CasteAuto`, botões e porcentagens de `_initControls` (`1000:cc84`);
  - `_GameSpeed` e os avisos de saúde de `_InitSimVars`;
  - `_NextFeed` = 40, valor estático do executável;
  - `_OptionStates` e `_HistButtStates` [hipótese: padrão observado no QUICK2/QUICK3];
  - `_Barrier` 0x50/0x90, gravado por `_OverlayTileSet` (`1010:00f4`).

  Um mod pode sobrescrever o arquivo.

Verificado (`engine/tests/estado_simulacao.rs` e testes de unidade de `estado.rs` e `tick.rs`):

- Os símbolos cobertos pelo `Estado` são exatamente os 307 do layout. A ida e volta é byte a byte com `QUICK2.ANT` e `QUICK3.ANT`.
- `contar_formigas` sobre o `QUICK2.ANT` e o `QUICK3.ANT` reproduz `_CastePop*[1..5]`, `_BpopT` e `_RpopT` do save. `_CastePop*[0]` sai maior ou igual, por causa dos ovos postos depois da última contagem (tick 64).
- Um tick com as regras vazias sobre o `QUICK2.ANT` muda só `_GameTime`, `_Cycle` e `_ModePop*` (este porque nenhuma formiga foi contada no tick).
- Periodicidade e ordem das chamadas, com regras de teste que anotam cada chamada e o `_Cycle`: todo tick, ticks ímpares (lagarta), a cada 64 (alimentação, contagem, histórico), a cada 128 por fase (compactação A/B/R), e a volta do `_Cycle` depois de 0x1000.
- `de_mundo` com a semente do `QUICK2.ANT` (precisa de `SIMANT_DIR`): controles, opções, velocidade, `_Barrier`, `_NextFeed`, vistas, buracos, rainhas e escavação iguais ao save.

Não verificado: nenhuma parte do tick foi comparada passo a passo com o original, porque as sub-rotinas ainda não existem. O caminho do Full Game em `_CountAnts` (nova colônia vermelha) não tem teste. `de_mundo` não aplica `_InitSimYard` nem `_ResetYellowVars` (os campos ficam zerados).

## Lacunas

- Taxa real de ticks por segundo em cada velocidade (seção 3) [em aberto].
- Detalhes de `_SimColonies`, `_DoSimYard` (menino, cachorro, gato, pássaro, cortador), `_DoAntLions`, `_MoveSpider`, `_DoPillar`/`_DoSow`, `_DoWater`: só a cadência está descrita aqui.
- `_Feedback`: regras do tutorial e das mensagens de conselho.
- Retorno exato de `_CalcScore` e o sentido da nota 3.
- Rede (`_NetworkSend`/`_ProcessPost`): fora do escopo do port inicial.
