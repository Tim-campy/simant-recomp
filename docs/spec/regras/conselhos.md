# Conselhos e tutorial: `_Feedback` (`1030:78ca`)

`_Feedback` roda em todo tick (passo 21 de [simulacao-visao-geral.md](simulacao-visao-geral.md) §5) e também dentro de `_FullCount` (fases 0 e 2, a cada 64 ticks). Tem duas partes: o **tutorial** (só no Tutorial, `_CurGameType = 0`) e a **mensagem de conselho** da faixa de mensagens (todos os tipos de jogo, se a opção estiver ligada).

Convenção de evidência: **fato visto no código** = pseudocódigo do Ghidra (`re_trabalho/decomp/Code7.c`, `Code1.c`, `Code5.c`) com endereço e nome do `SIMANTW.SYM`; textos lidos do banco `SHARED` descomprimido (`re_trabalho/dump2/SHARED/t04_*.bin`); **[hipótese]** como no [README](../README.md).

## 1. Variáveis

| Símbolo | Endereço | No save | Uso |
|---|---|---|---|
| `_TutorCnt` | `8:85ec` | sim | contador do tutor, 0..0x400 |
| `_TutLesson` | `8:85ee` | sim | lição atual (1 = primeira) |
| `_TutGiven` | `8:85f0` | sim | a lição atual já foi mostrada |
| `_TutHowLong` | `9:7d62` | sim | quantas vezes (de 8 em 8 chamadas) a lição está sendo esperada |
| `_LessonTemp` | `9:9fb0` | sim | memória da lição (posição, vista, tarefas, resultado) |
| `_TimeTemp` | `9:9b10` | **não** | 32 bits: prazo no relógio do Mac (1/60 s) ou marco de escavação |
| `_OptionStates[4]` | `8:85fa` | sim | liga a mensagem de conselho ([hipótese]: item de menu de conselhos/mensagens) |
| `_AdviceStrs` | `9:7c94` | não | lista de strings SHARED 4/1020, carregada por `_initStuff` (`_LoadStringAnt(0x3fc)`) |

## 2. Fluxo de `_Feedback`

**Fato visto no código.**

```
se _CurGameType == 0:                                     (tutorial; o mesmo corpo de _RunTutor 1030:7a90)
    _TutorCnt += 1; se _TutorCnt > 0x400: _TutorCnt = 0
    se _TutorCnt & 7 == 0:
        _TutHowLong += 1
        se _TutGiven == 0:
            _GiveLesson(_TutLesson); _TutGiven = 1; _TutHowLong = 0
        senão se _LessonDone(_TutLesson):
            _TutLesson += 1; _TutGiven = 0                  (a próxima lição sai daqui a 8 chamadas)
        senão se _TutHowLong ≥ 16 e relógio > _TimeTemp:
            _TutGiven = 0                                   (repete a mesma lição)
se _OptionStates[4] == 0: FIM
se _MeMode == 0 e (_MeHealth < _MeWarnHealth ou _MeHealth < 10):
    conselho = (_MeHealth < 10 ? nº 0 (importante) : nº 1)
senão se _HealthB < _BlkWarnHealth: conselho nº 2
senão se _BpopT ≤ _TilesDugB: _SetDefaultWindPrompt(0) (texto padrão da janela, conforme a ferramenta); FIM
senão se _MeMode == 0 e _MeNestStarted == 0 e _TilesDugB == 0: conselho nº 3
senão: conselho nº 4
_EditMessage(texto do conselho, duração 0x78, importante)
```

Textos da lista 4/1020 (fato verificado no banco):

| nº | Texto | Quando |
|---|---|---|
| 0 | "Your health is critical - You must eat" | amarela (formiga) com vida < 10; mostrado mesmo com outra mensagem na faixa |
| 1 | "Your health is low - Find some food" | vida < `_MeWarnHealth` (30 no início) |
| 2 | "Your colony needs food" | `_HealthB` < `_BlkWarnHealth` (30) |
| 3 | "You need to dig a nest" | colônia maior que a escavação, amarela sem ninho e nada escavado |
| 4 | "Your nest needs expansion" | `_BpopT` > `_TilesDugB` nos demais casos |
| 5 | "The red ants are attacking!" | não é usado aqui (`_GstrR`, [simulacao-visao-geral.md](simulacao-visao-geral.md) §7) |

`_EditMessage` (`1000:…`, Code1) só troca a mensagem da faixa quando o pedido é "importante" ou quando a opção está ligada e a faixa está vazia; o prazo de remoção é refeito a cada chamada (com o mesmo texto, a mensagem fica enquanto a condição durar). É interface: não muda o estado da simulação nem sorteia.

## 3. Lições: `_GiveLesson(n)` (`1030:7b4e`) e `_LessonDone(n)` (`1030:7f02`)

**Fato visto no código.** Cada lição mostra um quadro (`_PictStrnDialog(imagem, texto, 1)`) com o texto SHARED 4/`11000 + 2(n − 1)` (exceções na tabela) e às vezes faz uma ação. A lição termina quando a condição vale. Textos: 50 objetos 4/11000…11112 no banco (ex.: 11000 "Welcome to the SimAnt Tutorial…", 11004 "Try double-clicking somewhere in the black dirt near the yellow ant.", 11112 "This is the end of the SimAnt tutorial…").

Ações ao dar a lição:

| Lição | Ação |
|---|---|
| 3, 4 | `_TimeTemp = _TilesDugB + 2` (3) ou `+ 10` (4): marco de escavação |
| 7, 0x1A | `_LessonTemp = _MeLocY + _MeLocX`; `_TimeTemp = relógio + 600` |
| 9 | `_LessonTemp = _MapPnt.y + _MapPnt.x`; `_TimeTemp = relógio + 600` |
| 0x1B | `_LessonTemp = _MeLocY + _MeLocX` |
| 0x29 | `_LessonTemp = _modeLevels[1] + _modeLevels[0]` |
| 8, 0x1E, 0x2E, 0x32 | `_LessonTemp = 0` |
| 0xE | `_SetMyHealth(8)` (a amarela fica com fome, [formiga-amarela.md](formiga-amarela.md)) |
| 0x17 | `_AddBlackAnts(10)` **antes** do quadro ([geracao-mundo.md](geracao-mundo.md): sorteia `S(10)` e `S8()` por formiga) |
| 0x31 | quadro; `_AddBlackAnts(80)`; zera `_PherMapA` inteiro |
| 0x2B | `_TutLesson += 1` e mostra o texto da lição 0x2C (11086, imagem 0x4271): o texto 11084 nunca aparece |
| 0x37 | texto 11108 se `_LessonTemp == 0`, senão 11110 ("Too bad. Your attack on the red ants has failed…") |
| 0x38 | texto 11112 |
| 0 e > 0x38 | nada (fim do tutorial) |

Imagens (primeiro argumento; 0 = nenhuma): 2 → 17000, 5 → 0x4269, 0xB → 0x4276, 0xD e 0xE → 0x426A, 0x11 → 0x4275, 0x12 → 0x426B, 0x15 → 0x426C, 0x21 → 0x426D, 0x24 → 0x426E, 0x26 → 0x426F, 0x27–0x29 → 0x4270, 0x2B e 0x2C → 0x4271, 0x2D e 0x2E → 0x4272, 0x2F e 0x30 → 0x4273, 0x32 → 0x4274. [em aberto: a que recurso esses ids se referem; não estão entre os tipos 10 do SHARED.]

Condições de conclusão (`_LessonDone`):

| Lições | Termina quando |
|---|---|
| 1, 2, 6, 0xB, 0xF, 0x11, 0x17, 0x19, 0x1C, 0x1D, 0x1F, 0x22, 0x23, 0x25, 0x28, 0x2C, 0x2D, 0x30, 0x33, 0x35, 0x37, 0x38 | sempre (só texto) |
| 3, 4 | `_TilesDugB > _TimeTemp` e `_MeMoveMe == 0` |
| 5, 0x14 | `_MePlane == 1` (superfície) |
| 0x12 | `_MePlane == 2` (ninho preto) |
| 7, 0x1A | a posição mudou (`_MeLocY + _MeLocX ≠ _LessonTemp`), `_MeDis == 0` e relógio > `_TimeTemp` |
| 9 | a vista mudou (`_MapPnt.y + _MapPnt.x ≠ _LessonTemp`), **ou** relógio > `_TimeTemp` |
| 0xA, 0x20, 0x2A | janela 0x100 na frente (`_win_IsWinInFront`) |
| 0xC, 0x31 | janela 0 na frente |
| 0x27 | janela 0x1200 na frente |
| 0x2F | janela 0x1300 na frente |
| 0xD | amarela na superfície sobre tile > 0x47 |
| 0xE | `_MeHealth > 90` |
| 0x10 / 0x13, 0x16 / 0x15 | `_MeType == 0x18` / `0x10` / `0x28` |
| 0x18 / 0x34 | `_ModePopB[5]` (`9:9e7a`) > 1 / > 40 |
| 0x1B | a posição mudou, `_MeDropAlarm ≠ 0` e `_MeDis == 0` |
| 0x1E | (se `_MeDropAlarm ≠ 0`, desliga o alarme: `_SetAlarmDropState(0, 1)`) e `_LessonTemp ≠ 0` |
| 8, 0x2B, 0x2E, 0x32 | `_LessonTemp ≠ 0` (posto por ações do jogador, ex.: `_ExchangeLives` põe 1) |
| 0x21 / 0x26 | `_MapPlane == 0` / `== 1` |
| 0x24 | `_YardMode == 1` |
| 0x29 | `_modeLevels[1] + _modeLevels[0] ≠ _LessonTemp` |
| 0x36 | `_MePlane == 3` (põe `_LessonTemp = 0`) ou `_MePlane == 2` (põe 1) |
| outras | nunca |

As comparações com `_TimeTemp` são de 32 bits com sinal. As lições 3 e 4 usam `_TimeTemp` como marco de escavação: no `_Feedback`, "relógio > `_TimeTemp`" fica então sempre verdadeiro e a lição é repetida a cada 16 × 8 chamadas.

## 4. Casos de teste

| Caso | Entrada | Esperado |
|---|---|---|
| C-U1 conselho | `_OptionStates[4] = 1`, `_MeMode = 0`, `_MeHealth = 5` | conselho nº 0, importante |
| C-U2 | vida 20 (< 30) | nº 1 |
| C-U3 | vida 80, `_HealthB = 10` | nº 2 |
| C-U4 | `_BpopT = 5`, `_TilesDugB = 10` | texto padrão da janela |
| C-U5 | `_BpopT = 5`, `_TilesDugB = 0`, `_MeNestStarted = 0` | nº 3; com `_MeNestStarted = 1`, nº 4 |
| C-U6 opção desligada | `_OptionStates[4] = 0` | nenhum |
| C-U7 tutor | Tutorial, `_TutLesson = 1`, `_TutGiven = 0`, `_TutorCnt = 7` | mensagem 11000; `_TutGiven = 1`; `_TutHowLong = 0` |
| C-U8 tutor | lição 1 dada, 8 chamadas depois | `_TutLesson = 2`, `_TutGiven = 0`; mais 8 → mensagem 11002 |
| C-U9 lição 5 | `_MePlane = 2` | não termina; `_MePlane = 1` termina |
| C-U10 lição 0x2B | dar a lição 0x2B | mensagem 11086 e `_TutLesson = 0x2C` |

## Implementação no port

- `engine/src/rules/conselhos.rs`: `conselho(&Estado, &TabelasConselhos) -> Conselho` (a mensagem da faixa como **estado derivado**: a interface a consulta depois de cada tick; não vira evento, porque o original a refaz em todo tick), `executar_tutor(e, tabelas, memoria, interface)` e `impl RegrasConselhos for RegrasOriginais`.
- Os quadros das lições saem como `Evento::Mensagem(texto)` (o id da imagem não cabe no evento atual [lacuna]).
- `_TimeTemp` não está no save nem no `Estado`: fica em `MemoriaTutor`. Relógio: o mesmo do quintal (`_GameTime × mac_por_tick`, [quintal.md](quintal.md), "Implementação no port"). Janelas na frente: trait `InterfaceTutor` (padrão: nenhuma janela).
- Dados (`data/quintal.json`, bloco `conselhos`): lista 4/1020, limiar 10 da vida crítica, os 16 ciclos de espera e a tabela de lições acima (texto, imagem, ação, condição), editável por mods.

## Lacunas

- `RegrasOriginais` não guarda `MemoriaTutor` nem a interface: o `impl` do tick usa uma memória nova em cada chamada (`_TimeTemp = 0`), o que faz as lições 3, 4, 7, 9 e 0x1A terminarem mais cedo que no original, e as condições de janela nunca valem (lições 0xA, 0xC, 0x20, 0x27, 0x2A, 0x2F e 0x31 só avançam pela repetição). A integração precisa de um campo em `RegrasOriginais` (ou no `Estado`) e de um canal da interface.
- `_SetDefaultWindPrompt` e o texto padrão da faixa por ferramenta: interface, não especificados aqui.
- Nenhum caso do tutorial foi rodado no original.
