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

### 3.1 Conferido no original: o tutorial inteiro (2026-10-18)

**Verificado** na bancada QEMU (`python tools/cenarios_tick.py gravar tutorial_completo DIR --snapshot tut`, cópia do overlay `ov_tut.qcow2`): o roteiro faz a ação que cada lição pede e grava inteiros os ticks em que `_TutorCnt` chega a múltiplo de 8. As 56 lições foram dadas e concluídas, da lição 1 (tick 8) ao fim (`_TutLesson` = 57 no tick 1632, sem nada a mostrar). Duas gravações: 124 ticks (lições 1–26) e 185 ticks (lições 1–57); todos 307/307 no port, menos os três casos explicados em [testes-tick.md](testes-tick.md) ("Quinta rodada"). 16 deles estão em `tests/referencias/ticks/tutorial_completo/`.

Ações usadas (o que o texto da lição pede; coordenadas da bancada, janela de edição do Tutorial com 39 × 21 tiles a partir de (0, 128)): duplo clique na terra perto da amarela (3, 4); duplo clique na linha 0 acima dela (5; a 20 com clique simples na abertura 0x18); clique simples no chão (7, 26); clique na própria amarela (8, 50: `_LessonTemp` = 1 pelo centralizar); nenhuma (9: venceu o prazo de 600); Ctrl+M (10, 32, 42), Ctrl+E (12, 49), Ctrl+Y (33), Ctrl+5 (36), Ctrl+8 (38), Ctrl+B (39), Ctrl+C (47); clique simples numa comida (13) e duplo clique em outra (14 comer, 16 pegar); clique no buraco preto (18) e duplo clique no túnel (19, larga a comida); duplo clique numa pedrinha (21) e no chão (22); tecla 2 (24) e cinco vezes (52); tecla 0 e andar (27); X e clique numa preta (30); arrastar o círculo do triângulo do Behavior (41); clique segurado 1,5 s no gráfico de barras (46: `_LessonTemp` = 1); clique no buraco vermelho (54: entrou no ninho vermelho, `_LessonTemp` = 0 e a lição 55 mostrou o texto 11108).

Observado, além do que a tabela acima já diz:

- **O prazo das lições 7, 9 e 0x1A conta do fechamento da caixa.** `_GiveLesson` chama `_PictStrnDialog` (modal) **antes** de ler `_MacTickCount` para `_TimeTemp` (visto no código, `1030:7c4d`/`7c5e`; o mesmo vale para todas as ações: a caixa vem primeiro, menos na 0x17, em que `_AddBlackAnts` vem antes). Na bancada: lição 7 dada no tick 291 com o relógio em 10704 no começo do tick; a caixa ficou aberta até 12213, e `_TimeTemp` = 12813 = 12213 + 600. Lição 9 (tick 448): 15090 + 600 = 15690. O port fazia `_TimeTemp` = relógio do tick + 600; agora a interface refaz o prazo ao fechar a caixa (`MemoriaTutor::caixa_fechada`, abaixo).
- **Lição 0x2B pula para a 0x2C no mesmo tick** (tick 1245: `_TutLesson` 43 → 44 com `_TutGiven` = 1, caixa 11086), como na tabela.
- **Lição 0x29** guarda `_modeLevels[1] + _modeLevels[0]` em 16 bits com sinal (13107 + 39321 = −13108 no tick 1151).
- **Condições de janela:** as lições 0xA, 0xC, 0x20, 0x27, 0x2A e 0x2F concluíram com a janela pedida na frente (Ctrl+M, Ctrl+E, Ctrl+B, Ctrl+C). A 0x2A (voltar ao mapa) **não** concluiu com o botão Patch da faixa, com o Behavior aberto e a House View por baixo: o mapa troca de lugar com o quintal (`_YardToMap`), mas não fica na frente; só Ctrl+M concluiu. Depois de Ctrl+Y/Ctrl+5 com o quintal aberto, Ctrl+M traz o **quintal** (0x1900) para a frente, não o mapa (`_OpenMapYard`, mapa.md). A janela na frente não está na memória do jogo: o roteiro anota a que pôs na frente (`JanelaNaFrente` nos extras) e a anotação foi conferida pelo resultado (a lição conclui ou não no original). A 0x31 (janela 0 depois de `_AddBlackAnts(80)`) concluiu com Ctrl+E.
- `_TutorCnt` passa de 0x400 a 0 (a fronteira de 8 também acontece em 0x400 → 0) e anda **2** no tick em que `_FullCount` chama `_Feedback` de novo (`_Cycle` múltiplo de 64): tick 575, `_TutorCnt` 583 → 585.

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

- `engine/src/rules/conselhos.rs`: `conselho(&Estado, &TabelasConselhos) -> Conselho` (a mensagem da faixa como **estado derivado**: a interface a consulta depois de cada tick; não vira evento, porque o original a refaz em todo tick), `executar_tutor(e, tabelas, tabelas_amarela, memoria, interface, relogio)` e `impl RegrasConselhos for RegrasOriginais`.
- Os quadros das lições saem como `Evento::Mensagem(texto)`; a interface acha a imagem pela lição que usa aquele texto (`imagem_do_texto` em `engine/src/core/avisos.rs`, com `data/quintal.json` → `conselhos.licoes[].imagem`; os outros textos usam `mensagens.json` → `quadro.imagens`).
- Prazo das lições 7, 9 e 0x1A (§3.1): `dar_licao` põe `tempo_limite` = relógio do tick + espera e guarda a espera em `MemoriaTutor::espera_depois_da_caixa`; `Partida::fechar_modal` chama `MemoriaTutor::caixa_fechada(relógio agora)`, que refaz `tempo_limite` = relógio + espera (como o original, que lê `_MacTickCount` depois do `_PictStrnDialog`). Sem interface (vetor sem a leitura, avanço sem caixas), fica o prazo do tick. Nos vetores, `vetor_tick::depois_do_tick` faz o mesmo com `MacTickCount_depois_da_caixa`.
- `_TimeTemp` não está no save nem no `Estado`: fica em `MemoriaTutor`. Relógio: o mesmo do quintal (`_GameTime × mac_por_tick`, [quintal.md](quintal.md), "Implementação no port"). Janelas na frente: trait `InterfaceTutor`; `RegrasOriginais` guarda `memoria_tutor` (`MemoriaTutor`, vale de um tick para o outro, como a global do original) e `interface_tutor` (`JanelasNaFrente`: a interface grava ali as janelas que estão na frente; vazio = nenhuma). A lição que muda a vida da amarela usa o `_SetMyHealth` de `amarela.rs`, com `data/amarela.json`.
- Dados (`data/quintal.json`, bloco `conselhos`): lista 4/1020, limiar 10 da vida crítica, os 16 ciclos de espera e a tabela de lições acima (texto, imagem, ação, condição), editável por mods.

## Lacunas

- A interface grava `RegrasOriginais::interface_tutor` a cada quadro com a janela de cima, no formato de `_win_IsWinInFront` (número da janela << 8; `simant/src/sessao.rs`, desde que existem as janelas de controle, do mapa e do quintal). As condições de janela das lições 0xA, 0xC, 0x20, 0x27, 0x2A, 0x2F e 0x31 foram conferidas no original (§3.1) com a janela anotada pela bancada; não se conferiu se a `JanelasNaFrente` gravada pela interface do port coincide com a do original em cada situação (ex.: Ctrl+M com o quintal aberto traz o quintal). `_TimeTemp` não vai para o save (como no original): carregar um jogo zera a memória.
- `_SetDefaultWindPrompt` e o texto padrão da faixa por ferramenta: interface, não especificados aqui.
- Casos do original: os vetores `tests/referencias/ticks/tutorial/` (snapshot `tut`, lições 1–3: lição 1 concluída e lição 2 pedida no tick 15, lição 3 dada no 23 e esperando o marco de escavação nos 46–47, com `_TimeTemp` em `extras.json`) e o Load de `Tutorial.Ant` (`tests/referencias/load_tutorial/`) batem com o port; o tutorial inteiro foi conferido em 2026-10-18 (§3.1, `tests/referencias/ticks/tutorial_completo/`). Os casos de conselho C-U1..C-U6 (faixa de mensagem) continuam conferidos só pelo código; C-U8..C-U10 aconteceram no original durante a gravação (a C-U7, lição 1, já tinha sido dada no snapshot). Caixa da lição comparada com o port numa captura só (lição 14, tick 622): o texto e a imagem saem iguais (máscara do texto azul 99,6%, diferença sob o cursor), e, desde 2026-10-18, também a moldura (azul-escura com 1 pixel preto, `mensagens.json` → `quadro.cor_borda`; antes o port a fazia azul-clara) e a posição (centrada na área abaixo da faixa nos dois) — telas/mensagens.md §3.3.
