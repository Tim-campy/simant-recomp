# Formigas: listas, campos, castas, modos e máquina de estados

Este documento é a base para implementar o comportamento das formigas uma regra por vez. Descreve a representação (listas paralelas e mapas de "vida"), a codificação dos campos, castas e modos, a escolha de casta e modo (triângulos de controle) e a máquina de estados em alto nível. O laço que chama tudo isso está em [simulacao-visao-geral.md](simulacao-visao-geral.md); os cheiros estão em [feromonios.md](feromonios.md).

Convenção de evidência: igual à de [simulacao-visao-geral.md](simulacao-visao-geral.md#convenção-de-evidência). Endereços de variáveis no formato `segmento lógico:offset` do `symbols.tsv` (8 = seletor `1038`, 9 = `1040`, 10 = `1048`/DGROUP). Textos citados de SHARED tipo 4 foram descomprimidos com `tools/lzss_probe.py` (função `lzss`, janela preenchida com 0x20) a partir do dump de `tools/db_list.py`.

## 1. Planos e coordenadas

**Fato visto no código.**

| Plano | Código (`_MePlane`, `_SetLife`, `_GetLife`) | Mapa de terreno | Mapa de vida (quem ocupa a célula) | Tamanho | Lista |
|---|---|---|---|---|---|
| Superfície (quintal/casa) | 0 ou 1 | `_MapA` `10:28e8` | `_LifeA` `10:68e8` | 128 × 64 | A |
| Ninho preto | 2 | `_MapB` `10:48e8` | `_LifeB` `10:88e8` | 64 × 64 | B |
| Ninho vermelho | 3 | `_MapR` `10:58e8` | `_LifeR` `10:98e8` | 64 × 64 | R |

- Índice das matrizes: `x × 64 + y`. Na superfície, x vai de 0 a 127 e y de 0 a 63 (limites em `_SetLife` `1020:5d18` e `_GetLife` `1020:6040`). No ninho, os dois vão de 0 a 63.
- `_LifeA/B/R` **não** estão no save. São refeitos a partir das listas, ou o contrário: `_BuildAntListA` (`1020:3046`), chamada por `_RandWorld`, monta a lista A varrendo `_LifeA`.
- **Passagem superfície ↔ ninho.** Quando uma formiga de superfície entra num buraco em (x, y), ela vai para o ninho na posição (**y**, 1), virada para a direção 4 (baixo). O ninho escolhido é o preto se x < 64 e o vermelho se não (`_GoInNest` `1028:257a`). Para sair, a formiga precisa estar na linha 0 do ninho, numa célula de buraco (tile 0x18), e sai pelo buraco cuja coluna de superfície está em `_HoleMapB[x_ninho]` (`_GetOutB` `1028:520a`, `_LeaveNestB` `1028:515e`). Leitura [hipótese]: o ninho é um corte vertical em que a coordenada x corresponde à linha y da superfície, e o y do ninho é a profundidade.
- `_HoleMapB` (`8:82d2`, 64 bytes) e `_HoleMapR` (`8:8312`): índice = linha y da superfície (ou coluna x do ninho); valor = coluna x do buraco na superfície (0 = sem buraco). **Verificado** no `QUICK2.ANT`: `_HoleMapB[32] = 3` e `_HoleMapR[32] = 118`, ou seja, buracos em (3, 32) e (118, 32).
- **Direções** (8, sentido horário, começando no norte): tabelas `_Dx8` (`8:0000`) = `0 1 1 1 0 -1 -1 -1` e `_Dy8` (`8:0008`) = `-1 -1 0 1 1 1 0 -1` (lidas do `SIMANTW.EXE`). Direção 0 = y−1 (cima), 2 = x+1 (direita), 4 = y+1, 6 = x−1. `_Dx9`/`_Dy9` (`8:0010`/`8:001a`) têm 9 entradas: a primeira é "parado" e as demais são as 8 direções.

## 2. Listas paralelas

**Fato visto no código** e tabela do save ([save-ant.md](../formatos/save-ant.md)).

| Lista | Arrays (1 byte por formiga) | Capacidade | Contador | Quem está nela |
|---|---|---|---|---|
| A | `_AlistX` `8:23a4`, `_AlistY` `8:278e`, `_AlistM` `8:2b78`, `_AlistT` `8:2f62`, `_AlistS` `8:334c` | 1000 | `_ListIndexA` `9:80f0` | todas as formigas na superfície, das **duas** cores |
| B | `_BlistX..S` `8:3736…8:3f0e` | 500 | `_ListIndexB` `9:99d4` | dentro do ninho **preto**: pretas e invasoras vermelhas |
| R | `_RlistX..S` `8:4104…8:48dc` | 500 | `_ListIndexR` `9:72cc` | dentro do ninho **vermelho**: vermelhas e invasoras pretas |

A formiga amarela **não** está em lista nenhuma. Seu estado fica nas variáveis `_Me*` (`_MeLocX/Y`, `_MePlane`, `_MeType`, `_MeDir`, `_MeMode`, `_MeHealth`…).

### Campos

| Campo | Significado | Evidência |
|---|---|---|
| X, Y | posição no plano da lista | todas as funções de movimento |
| T | **tipo**: o mesmo byte gravado no mapa de vida. Codifica cor, casta/carga e direção (seção 3). T = 0 é uma vaga livre (formiga morta ou removida) | `_AddAntToAList` `1020:2ef0` grava T em `_LifeA`; `_CompactListA` descarta T = 0 |
| M | **modo** (tarefa atual), 0..19 (seção 5) | `_DoAntSimA` `1028:04d8` despacha por M |
| S | **contador ou memória**, conforme o modo: na superfície, quanto cheiro depositar no próximo passo (decresce 1 por passo; seção 7); na luta, o T do vencedor; no ninho, o estágio do ovo carregado e uma flag (bit 3) | `_DoRandAntA`, `_DoReturnFoodAnt`, `_StartFightA`, `_DoNestingB` |

### Inserção, remoção e compactação

- **Inserir:** `_AddAntToAList/BList/RList` (`1020:2ef0`/`2f4a`/`2fa4`) gravam na vaga `_ListIndex` e incrementam o contador. Também gravam T no mapa de vida. Se a lista está cheia, **descartam em silêncio**. `_AddAntToList` (`1020:5c78`) escolhe a lista pelo plano e chama `_SetLife`.
- **Remover numa iteração:** o código põe T = 0 na lista e zera a célula do mapa de vida. A vaga só é reaproveitada depois da compactação.
- **Compactar:** `_CompactListA/B/R` (`1020:2a16`/`2a7a`/`2ade`) deslizam as entradas com T ≠ 0 para a frente, mantendo a ordem, e diminuem o contador. Isso acontece a cada 128 ticks (uma lista por fase; ver a visão geral). Também acontece sob demanda: `_GoInNest` compacta a lista B/R se ela está cheia, e `_ExitHole` (`1020:2db6`) compacta a lista A (o mesmo algoritmo, embutido) se ela está cheia.
- **Remoção imediata** com deslocamento (`_RemoveFromAList` `1020:2b42`, via `_BlockMove`): usada pela formiga-leão (`_DoAntLions`) e por `_GetFromAlist`.
- **Busca por posição:** `_FindInAList(x, y)` (`1020:2c42`) varre a lista de trás para frente e devolve o primeiro índice com X, Y iguais e T ≠ 0, ou −1. As versões B e R recebem também o T esperado.
- **Ordem de processamento:** de trás para frente (índice `_ListIndex − 1` até 0).

## 3. Codificação do tipo T

**Fato visto no código**, com nomes do jogo tirados das listas de texto de SHARED tipo 4 (ids 1100, 1101, 1103), indexadas pelo grupo `(T & 0x78) >> 3`.

```
bit 7      : cor (0 = preta, 1 = vermelha)
bits 6..3  : grupo (casta + o que carrega), 0..15
bits 2..0  : direção 0..7 (nos ovos, estágio de desenvolvimento)
```

| Grupo | T (preta) | Nome (4/1101) | Carga (4/1103) | Censo (`_CastePop*[i]`) | Classe de luta |
|---|---|---|---|---|---|
| 0 | 0x01–0x07 | ovo (estágios em 4/1100) | — | [0] ovos | 0 |
| 1 | 0x08–0x0F | Worker | Egg (carregando ovo) | [1] operárias | 0 |
| 2 | 0x10–0x17 | Worker | — | [1] | 0 |
| 3 | 0x18–0x1F | Worker | Food | [1] | 0 |
| 4 | 0x20–0x27 | Male | — | [3] machos | 2 |
| 5 | 0x28–0x2F | Worker | Dirt | [1] | 0 |
| 6 | 0x30–0x37 | Soldier | — | [2] soldados | 1 |
| 7 | 0x38–0x3F | Soldier | Food | [2] | 1 |
| 8 | 0x40–0x47 | New Queen | — | [4] rainhas novas | 2 |
| 9 | 0x48–0x4F | Soldier | Dirt | [2] | 1 |
| 10 | 0x50–0x57 | Replete | — | não contado | 0 |
| 11 | 0x58–0x5F | "?" | — | não contado | 0 |
| 12 | 0x60–0x67 | Queen (cabeça) | — | [5] rainha | 3 |
| 13 | 0x68–0x6F | Queen (cauda) | — | não contado | 3 |
| 14 | 0x70–0x77 | Fight (nuvem de luta) | — | não contado | 0 |
| 15 | 0x78–0x7F | "?" | — | não contado | 0 |

- Pegar comida soma 8 ao T (grupo 2→3 ou 6→7). Pegar terra vai de 2→5 ou 6→9 (soma 0x18), e largar a terra volta (`_DoDigOutAntA` subtrai 0x18). Pegar ovo vai de 2→1 (subtrai 8). **Fato visto** em `_DoRandAntA`, `_DoForageAnt`, `_DoDigOutAntA`, `_DoNestingB`.
- **Ovos:** os bits 0..2 são o estágio. Estágios em 4/1100: Weird Egg, New Egg, Egg, 1st-Instar, 2nd-Instar, 3rd-Instar, Pupae, Hatching Pupae. `_IsThisEgg` (`1020:5ec8`) aceita `T & 0x7F` de 1 a 7.
- **Rainha:** ocupa 2 células, a cabeça (grupo 12) e a cauda (grupo 13). A cauda fica na direção da cabeça e vale cabeça + 8. Se a outra metade some, a que restou também é removida (`_SimQueenA` `1028:0a74`, `_SimQueenB` `1028:3dc2`).
- **Formiga amarela no mapa de vida:** 0xFF ou 0xFE (`_IsYellowAnt` `1020:5720`).
- **Censo:** `_CountAnts` (`1020:04de`) soma as três listas e a amarela por grupo (`_TemCensus` `9:7c4e`, 32 palavras: 16 grupos × 2 cores). Depois preenche `_CastePopB` (`10:ac94`) e `_CastePopR` (`10:aca0`) como na coluna "Censo". `_BpopT`/`_RpopT` = soma de [1]..[5] (sem os ovos). **Verificado** no `QUICK2.ANT`: `_CastePopB = (2, 2, 0, 0, 0, 1)` e `_BpopT = 3`. A lista B tem 3 ovos (grupo 0, M = 8), mais a cabeça e a cauda da rainha (grupos 12 e 13, M = 9). Os dois ovos do censo contra 3 na lista vêm de momentos diferentes (o censo roda a cada 64 ticks).

## 4. Castas e o triângulo de castas

**Fato visto no código.**

- `_casteLevels` (`9:9b64`, 3 palavras sem sinal, 0..65535) = posição do controle no triângulo "Workers/Soldiers/Breeders", na ordem **[reprodutores, operárias, soldados]**. Arrastar o controle (`_ProcCasteEvent` `1000:bd08`, via `_BoundPointToTri`/`_GetTriLatDist`) atualiza os níveis. Ao soltar o botão do mouse, o jogo grava o conjunto em `_casteSetLevels` e recalcula `_IdealCaste` (`8:8622`, 4 palavras, em %):

  | `_IdealCaste[i]` | casta | fórmula |
  |---|---|---|
  | 0 | operária | `(casteLevels[1] × 100 + 0x3FFF) / 0xFFFF` |
  | 1 | soldado | `(casteLevels[2] × 100 + 0x3FFF) / 0xFFFF` |
  | 2 | macho | `(casteLevels[0] × 50 + 0x3FFF) / 0xFFFF` |
  | 3 | rainha nova | igual ao macho (os reprodutores se dividem meio a meio) |

  A mesma conta existe em `_cvtLevels2IdealCaste` (`1000:c9de`), que não é chamada no código.
- Padrões lidos do executável: `_casteSetLevels` (`8:863a`, 4 conjuntos de 3) = (0, 39321, 26214), (32767, 16383, 16383), (0, 65535, 0), (0, 0, 65535). `_IdealCaste` = 60, 40, 0, 0. **Verificado** no `QUICK2.ANT`: `casteLevels = (0, 39321, 26214)` e `IdealCaste = (60, 40, 0, 0)`.
- [em aberto] Os botões de conjunto (objetos 0x1306–0x1308) trocam `_casteLevels`, mas não vi `_IdealCaste` ser recalculado nesse caminho. Conferir no original se trocar de conjunto muda a produção.
- `_CasteAuto` (`8:8630`) só aparece no código da interface (mostra ou esconde o grupo de botões). Não achei uso na simulação.

**Escolha da casta do próximo ovo**, `_SetCasteProd` (`1030:026e`), todo tick:

```
real[i]  = 100 × CastePopB[i+1] / Σ CastePopB[1..4]     (i = 0..3: operária, soldado, macho, rainha nova)
ideal[i] = 100 × IdealCaste[i] / Σ IdealCaste
escolhido = o i com a MENOR diferença (real − ideal), contando só diferenças < 0; se nenhuma, i = 0
_MakeMe (8:8a56) = _CasteTabB[escolhido]      (_CasteTabB 8:89ae = 2, 6, 4, 8: grupos de operária, soldado, macho, rainha nova)
```

**Eclosão** (`_SimEggB` `1028:3ca0`, só no ninho): a cada 32 ticks (se `_BpopT` < 3) ou 128 ticks, o estágio do ovo sobe 1. Ao chegar a 8, o ovo eclode:

- Com `_ModeAuto` = 0, o ovo **morre** (`_TotalEggsDiedB` += 1) se `SGRand(255)` > `modeLevels[2] >> 7` (nível de "cuidar das crias"). [hipótese] Com pouca gente cuidando, mais ovos se perdem. Com `_ModeAuto` = 1 o ovo sempre eclode.
- Se não morre: T = `_MakeMe × 8 + 2` (virada para a direita). M = 1 (Nesting) se for operária; senão, M = `_GetNewModeB(_MakeMe)`.

**Rainha põe ovos** (`_SimQueenB`, cauda, grupo 13): a cada 16 ticks, com probabilidade `SRand128() ≤ _HealthB`, põe um ovo (T = 1) atrás da cauda, se a posição estiver dentro do ninho. Cada ovo posto consome a reserva de comida (`_EatCountB` −1; quando fica negativa, `_HealthB` −1 e a reserva recomeça em `_BpopT/32`) e conta em `_TotalEggsLaidB`. A cabeça anda com chance 1/64 por tick (`_QueenMoveB`). Se `_HealthB` = 0 nesse momento, a rainha morre de fome (mensagem 10015).

## 5. Modos

**Fato visto no código.** Nomes de 4/1102, na ordem do índice M. "A" = superfície (`_DoAntSimA` `1028:04d8`), "B" = ninho preto (`_DoNestAntB` `1028:2dae`); o ninho vermelho (`_DoNestAntR` `1028:53a4`) espelha o B [hipótese: espelho fiel; há uma função extra `_StayInR`].

| M | Nome (4/1102) | Superfície (A) | Ninho (B) |
|---|---|---|---|
| 0 | Bored | `_DoRandAntA`: anda ao acaso; numa célula de comida, operária ou soldado pega a comida (M = 3, S = 200); 1/8 de chance de virar Foraging | 1/32 de chance de sortear novo modo; anda |
| 1 | Nesting | `_DoToNestAnt`: volta para o ninho seguindo o cheiro de ninho | `_DoNestingB`: cuida das crias (pega, carrega e larga ovos; come do estoque) |
| 2 | Foraging | `_DoForageAnt`: segue a trilha; pega comida (M = 3, S = 200); entra em Alarmed se houver alarme na célula; 1/32 de chance de virar Resting | `_DoDigOutB`: vai para a saída |
| 3 | Got Food | `_DoReturnFoodAnt`: volta ao ninho deixando trilha (S decrescente) | `_DoFoodInB`: leva a comida ao depósito |
| 4 | Dig In | `_DoToNestAnt` (igual a 1) | `_DoDigInB`: escava |
| 5 | Dig Out | `_DoDigOutAntA`: carrega terra para fora e larga (1/8 por passo) | `_DoDigOutB` |
| 6 | Defense | `_DoRecruitAnt`: segue a formiga amarela (ou a aranha, ou o alarme) | se a amarela não está no ninho preto: `_DoDigOutB`; senão anda e briga |
| 7 | Attack | `_DoAttackAnt`: vai para o ninho **inimigo** (segue o cheiro de ninho da outra cor) | `_DoDigOutB` (vermelho invasor: `_RaidInB`) |
| 8 | Hatching | ovo na superfície: some com chance 1/200 por tick | `_SimEggB` |
| 9 | Egg Laying | rainha na superfície: confere se a cauda/cabeça existe | `_SimQueenB` |
| 10 | Fighting | `_DoFightA`: nuvem de luta (seção 6) | `_DoNestFightB` |
| 11 | Alarmed | `_DoToAlarm`: sobe o gradiente de alarme; sem alarme, 1/4 de chance de sortear novo modo | `_DoDigOutB` |
| 12 | Defend Nest | cheiro de ninho < 110 na célula: volta ao ninho; senão anda perto dele | `_DoDigOutB` |
| 13 | Resting | parada; num buraco, entra; 1/4 de chance por tick de virar Foraging | parada; 1/20 de chance de sortear novo modo |
| 14 | Pre-Flight | metade das vezes vai para o ninho, metade anda; se `_FlyAway*` > 100, passa a 15 | anda; se `_FlyAwayB` > 100, passa a 15 |
| 15 | Take off | `_DoRepoExit`: cheiro de ninho < 100 → ninho, senão anda; quando `_FlyAway` = 1 (ou com chance 1/`_FlyAway`), passa a 16 | `_DoDigOutB` |
| 16 | Flying | com chance 1/32 (e se `_SwarmCnt*` < 50), sai do mapa; no Jogo Completo, `_SwarmCnt*` += 1 e, com chance 1/16, `_QueenStorage*` += 1 | `_DoDigOutB` |
| 17 | ??? | — | afogando [hipótese]: se o tile do ninho > 0x13, gira ao acaso e morre com chance 1/100; senão sorteia novo modo |
| 18 | ??? | — | como Bored |
| 19 | Avenger | `_DoRedInitiator`: a vermelha líder que conduz ataques | como Bored |

Regras comuns na superfície (vistas em `_DoRandAntA`, `_DoForageAnt`, `_DoToAlarm`…):

- **Buraco:** se a formiga está sobre um buraco (tile 0x50 `'P'` no quintal; 0x80–0x8F na casa, `_TERRAINset` = 1), entra no ninho (`_GoInNest`), em quase todos os modos.
- **Obstáculo:** se o tile de destino é maior que `_Barrier` (`9:7604`), a formiga não anda, só gira (tabela de giro, seção 7).
- **Célula ocupada:** se for a amarela de outra cor, há luta com a amarela (`_YellowFight`). Se for a amarela da mesma cor, a formiga pode alimentá-la (`_DoTroph`, quando `_MeWantFood` = 1). Se for formiga da mesma cor, gira e sorteia novo modo. Se for inimiga, há luta (seção 6).
- **Mudança de modo:** `_GetNewMode(grupo, T)` (`1030:0910`), explicado a seguir.

### Escolha de modo

**Fato visto no código** (`_GetNewMode` `1030:0910`, `_GetNewModeB` `1030:09d0`, `_GetNewModeR` `1030:0a50`, `_SetModeProd` `1030:0326`).

| Quem | `_ModeAuto` (`9:9fce`) = 1 | `_ModeAuto` = 0 | Vermelhas |
|---|---|---|---|
| Operária (grupo 2) | `_ModeTabWB[_StrategicModeB][SRand8]` | `_ModeMe` | `_ModeTabWB[_StrategicModeR][SRand8]` |
| Soldado (grupo 6) | `_ModeTabSB[_StrategicModeB][SRand8]` | `_ModeMe` | `_ModeTabSB[_StrategicModeR][SRand8]` |
| Demais grupos | `_CasteModeTabB[grupo]` | idem | idem |

Tabelas lidas do `SIMANTW.EXE` (segmento 8):

`_ModeTabWB` (`8:89e6`, linha = modo estratégico 0..5, 8 sorteios):

| Estratégico | Modos |
|---|---|
| 0 | 1 2 7 7 7 7 7 7 |
| 1 | 4 1 1 1 4 2 2 2 |
| 2 | 13 13 13 4 4 4 2 2 |
| 3 | 13 13 4 4 4 4 4 2 |
| 4 | 1 1 1 2 2 2 2 2 |
| 5 | 1 2 2 2 2 2 2 2 |

`_ModeTabSB` (`8:8a16`):

| Estratégico | Modos |
|---|---|
| 0 | 7 7 7 7 7 7 7 7 |
| 1 | 1 2 2 2 2 2 2 2 |
| 2 | 2 2 2 4 4 2 2 2 |
| 3 | 2 4 4 4 4 4 4 2 |
| 4 | 2 2 2 2 2 2 2 2 |
| 5 | 2 2 2 2 2 2 2 2 |

`_CasteModeTabB` (`8:8a46`, por grupo 0..15): `8 1 0 3 14 5 0 3 14 5 0 0 9 9 10 0`. Ou seja: ovo → 8, carregando ovo → 1, com comida → 3, com terra → 5, macho e rainha nova → 14 (Pre-Flight), rainha → 9, luta → 10.

**Triângulo de tarefas** (Forage/Dig/Nurse): `_modeLevels` (`9:9c74`, 3 palavras: **[forragear, cavar, cuidar]**), conjuntos em `_modeSetLevels` (`8:8658`): (60%, 20%, 20%), (100%, 0, 0), (0, 100%, 0), (0, 0, 100%). **Verificado** no `QUICK2.ANT`: `modeLevels = (39321, 13107, 13107)`. `_SetModeProd`, todo tick:

```
total     = ModePopB[0] + ModePopB[1] + ModePopB[2]
ideal[i]  = modeLevels[i] × total / 65535
falta[i]  = ideal[i] − ModePopB[i]
escolhido = o i com a MAIOR falta positiva (se nenhuma, 0)
_ModeMe (8:8a58) = _ModeTabB[escolhido]       (_ModeTabB 8:89b6 = 2, 4, 1: Foraging, Dig In, Nesting)
```

`_ModePopB` (`9:9e70`, 6 palavras) é montado no fim de cada tick a partir da contagem por modo daquele tick (`_TemBModePop`, A + B):

| `_ModePopB[i]` | Modos somados |
|---|---|
| 0 forragear | 2 + 3 |
| 1 cavar | 4 + 5 |
| 2 cuidar | 1 |
| 3 | 7 (Attack) |
| 4 | 12 (Defend Nest) |
| 5 | 6 (Defense) |

`_ModePopR` (`9:a084`) segue a mesma regra. `_TemRModePop[19]` conta as líderes vermelhas.

`_ModeAuto` e `_CasteAuto` começam em 1 (`_initControls` `1000:cc84`); **verificado** no `QUICK2.ANT`. [hipótese] Mexer no triângulo de tarefas desliga `_ModeAuto` (`_ProcModeEvent` `1000:c07a` zera a flag nos ramos de arrasto); falta conferir.

## 6. Luta, morte e saúde

**Fato visto no código.**

- **Encontro:** quando uma formiga tenta entrar numa célula com inimiga, a que se move **desaparece na hora** (T = 0). A da célula vira nuvem de luta: T = 0x70 + bit de cor do vencedor, M = 10, S = T do vencedor. Também se marca alarme ≥ 40 ali (`_AlarmHere2`). O vencedor sai de `_GetWinner` (`1028:26f4`):

  ```
  classe = tabela por grupo (DGROUP 0x22C6): 0,0,0,0,2,0,1,1,2,1,0,0,3,3,0,0
           (0 = operária/ovo, 1 = soldado, 2 = reprodutor, 3 = rainha)
  P[a][b] = chance (em 10) de a vencer b, matriz 4×4 em DGROUP 0x22D6:
            operária   5 2 7 3
            soldado    8 5 9 4
            reprodutor 3 1 5 2
            rainha     7 6 8 5
  vence a (a que estava na célula) se RRand(10) < P[classe(a)][classe(b)]
  ```

  A perdedora conta em `_BlackLost`/`_RedLost` e `_BAntsKilled`/`_RAntsKilled`. Com `_Invincible` = 1, a vermelha sempre perde. O segmento das duas tabelas foi deduzido do conteúdo do `SIMANTW.EXE` (as linhas da matriz somam 10 com a transposta); elas não têm nome no `.SYM`.
- **Nuvem de luta** (`_DoFightA` `1028:27e6`): a cada tick ela se redesenha. Com chance 1/16, a luta termina: T volta ao do vencedor (S), sorteia-se novo modo e um cadáver é posto no mapa (`_DeadAntHere`).
- **Cadáver** (`_DeadAntHere` `1028:28c0`): guarda a posição num anel de 100 posições (`_DeadX` `9:9c82`, `_DeadY` `9:9d76`, ponteiro `_DdPtr`). O cadáver mais antigo do anel some do mapa. O tile vira "formiga morta" (comestível), em valores próprios por cor (ver `tiles-valores.md`).
- **Morte natural (superfície):** a cada tick, para cada vaga da lista A, sorteia `SRand256()`. Se der 0 e a vaga estiver ocupada, compara a saúde da colônia (`_HealthB`/`_HealthR`) com `SRand32()`; se a saúde for menor, a formiga morre (cadáver, `_BAntsExpired` += 1, inclusive para vermelhas). Saúde ≥ 32 impede essa morte.
- **Morte natural (ninho preto):** igual, só para pretas e não-rainhas (M ≠ 9), sem cadáver. Aqui o `SRand256()` só é sorteado para formigas vivas: `_DoAntSimB` pula as vagas com T = 0 antes de chamar `_DoNestAntB`.
- **Saúde da colônia** (`_HealthB` `10:ac86`, `_HealthR` `10:ac88`, 0..100): perde 1 a cada 64 ticks. Ganha 1 quando as formigas comem o bastante: cada mordida no estoque soma 5 a `_EatCountB`, e quando ele passa de (`_BpopT` + soldados)/16 a saúde sobe 1 e o contador zera (`_EatFoodB` `1028:4844`, `_TryEatFoodB`, `_DoNestingB`). Os ovos consomem a reserva (`_DecEatB` `1028:48f8`).

## 7. Movimento e escolha de direção

**Fato visto no código.**

- **Tabela de giro** `_TurnTab` (`8:0024`, 8 × 8 bytes): `TurnTab[dir_atual][alvo]` dá a nova direção, girando no máximo 1 passo (45°) em direção ao alvo. As 8 linhas, lidas do executável:

| dir. atual | alvo 0 1 2 3 4 5 6 7 |
|---|---|
| 0 | 0 1 1 1 7 7 7 7 |
| 1 | 0 1 2 2 2 2 0 0 |
| 2 | 1 1 2 3 3 3 3 1 |
| 3 | 2 2 2 3 4 4 4 4 |
| 4 | 3 3 3 3 4 5 5 5 |
| 5 | 6 6 4 4 4 5 6 6 |
| 6 | 7 7 7 5 5 5 6 7 |
| 7 | 0 0 0 0 6 6 6 7 | O passeio aleatório usa `TurnTab[dir][SRand8()]`, o que favorece manter a direção ou girar 45°.
- **Bordas:** em todas as funções de direção, uma formiga na borda (x = 0 ou 127, y = 0 ou 63) sorteia uma direção que aponta para dentro. `_GetForageDir` usa 3 opções e cantos fixos; `_GetNestDir` e `_GetAlarmDir` usam 5 opções no meio da borda e 3 nos cantos (tabela em [feromonios.md](feromonios.md#4-leitura-escolha-de-direção)).
- `_GetRandDir` (`1030:0f72`): passeio aleatório.
- `_GetForageDir` (`1030:0ab0`): olha a trilha da própria cor nas 8 células vizinhas (resolução 2×2) e escolhe a maior. Se nenhuma tiver trilha, faz passeio aleatório. Se a maior for **menor** que a da célula atual, devolve −1 ("fim da trilha"): `_DoForageAnt` então põe a formiga em Bored (7/8) ou sorteia novo modo (1/8).
- `_GetNestDir` (`1030:0c30`): sobe o gradiente do cheiro de ninho da cor pedida (`_DoAttackAnt` pede a cor inimiga). Sem cheiro na célula: em 3/4 das vezes aponta para o último buraco feito (`_LastNewHoleB/R`, `8:835a`/`8:835e`), senão passeio aleatório. Com cheiro: escolhe o vizinho de maior valor e aplica o giro limitado uma vez ou, em metade das vezes (`SRand2` ≠ 0), duas vezes seguidas (até 90°). Detalhes e ordem dos sorteios em [feromonios.md](feromonios.md#4-leitura-escolha-de-direção).
- `_GetAlarmDir` (`1030:0e54`): sobe o gradiente do alarme (mapa comum às duas cores).
- `_GetDefendDir` (`1030:1026`): segue a amarela. Se a amarela está no ninho, segue o cheiro de ninho daquela cor; se está caçando a aranha, vai para a aranha; se está perto da posição difusa (`_FuzLocX/Y`), passeia.
- `_Recruit(n)` (`1030:06d2`): põe em Defense (M = 6, S = 0) até n operárias ou soldados pretos que ainda não estejam nesse modo. Varre a lista A e depois a B, de trás para frente.
- **Saída do ninho** (`_ExitHole` `1020:2db6`): a formiga aparece na primeira célula vizinha válida do buraco, testando as direções 0 a 7, que tenha tile < 0x50. Se o modo não for 3, 6 nem 7 e o buraco for do próprio lado do mapa (pretas com x < 64; vermelhas com x ≥ 65), S = 120 (vai depositar cheiro de ninho). No modo 6, S é preservado.

## 8. Formiga vermelha líder ("Avenger", modo 19)

**Fato visto no código.** Se nenhuma vermelha está no modo 19 e `_BpopT` ≥ 30, `_MakeRedInitiator` (`1028:967c`) transforma a última vermelha da lista A em líder: T = 0xB0 (soldado vermelho), M = 19, S = 0, `_RedTask` = 0, `_RedInitOn` = 1. `_DoRedInitiator` (`1028:96d4`) conduz recrutamentos e ataques vermelhos (`_RecruitRed`, `_UnRecruitRed`, `_GetNewRedTask`). Detalhamento [em aberto].

## 9. Formiga amarela (resumo)

**Fato visto no código.** `_DoAntSimY` (`1028:7cf6`) roda todo tick. Com `_MeMode` = 0, ele:

- grava a amarela no mapa de vida;
- tira 1 de vida a cada 64 ticks, e mais 1 por tick dentro do ninho se o tile da célula for > 0x4D;
- se a amarela é rainha (`_MeType` = 0x60) dentro de um ninho, a cada 16 ticks pode pôr ovo (custa 5 de vida).

`_DoAntMoveY` (`1028:75f4`) move a amarela quando há ordem (`_MeMoveMe`). Ela pode depositar trilha (`_MeScent`, que começa em 200 ao pegar comida e não desce de 10) e alarme (+50 por passo, máximo 200, com `_MeDropAlarm`). A morte e a reencarnação estão em `_YellowDeath` (`1000:7746`). A ordem de busca de um novo corpo é: ovo no ninho preto, adulto preto no ninho preto, adulto preto na superfície, adulto preto no ninho vermelho. Detalhes da amarela num documento próprio [em aberto].

## 10. Funções presentes mas não chamadas

Sem chamada encontrada no pseudocódigo: `_DoDefendNest` (`1028:0df6`), `_DoRepoLoit` (`1028:0c1e`), `_DoRestAnt` (`1028:0b76`), `_DoRepoFly` (`1028:0d4a`), `_SimEggA` (`1028:0a1c`), `_SimQueenA` (`1028:0a74`), `_GetSmellT` (`1028:9612`), `_cvtLevels2IdealCaste`, `_GetFromAlist`. A lógica equivalente está embutida em `_DoAntSimA`: os casos 8, 9, 13, 14 e 16 repetem `_SimEggA`, `_SimQueenA`, `_DoRestAnt`, `_DoRepoLoit` e `_DoRepoFly`. [hipótese] São restos do port do Mac ou chamadas por ponteiro que o Ghidra não resolveu.

## 11. Como testar contra o original

Bancada: [docs/bancada-qemu.md](../../bancada-qemu.md). Ferramenta de leitura sugerida: um `tools/ant_dump.py`, a escrever, que use os offsets de [save-ant.md](../formatos/save-ant.md).

1. **Codificação de T.** No original, abra a janela Examine sobre formigas conhecidas (operária, soldado, com comida) e compare com o T da lista no save feito logo depois (posição X, Y). Esperado: os nomes da tabela da seção 3.
2. **Eclosão e castas.** Jogo Quick, triângulo de castas no vértice "Soldiers", sem `_ModeAuto`. Salve a cada ~128 ticks e conte os grupos novos na lista B: só devem nascer soldados (grupo 6, T = 0x32).
3. **Modos.** Com `_ModeAuto` desligado e o triângulo de tarefas em "Dig", confira em saves sucessivos que as novas atribuições de operárias e soldados vão para M = 4 (Dig In).
4. **Lista e compactação.** Em saves sucessivos, `_ListIndexA` só pode cair em ticks ≡ 0 (mod 128). Fora desses ticks, as vagas com T = 0 permanecem.
5. **Luta.** No modo Experimental, ponha uma operária preta e uma vermelha lado a lado e repita várias vezes. A proporção de vitórias deve tender a 50%; soldado contra operária, a 80%.
6. **Comparação com o port.** Carregue o mesmo `.ANT` no port, rode N ticks com o mesmo RNG e compare as listas por vaga, na mesma ordem, além de `_TemCensus`, `_CastePop*`, `_ModePop*`, `_MakeMe` e `_ModeMe`.

## Lacunas

- Regras detalhadas de cada `_Do*Ant` (passo a passo) e das funções de ninho `_DoNestingB`, `_DoFoodInB`, `_DoDigInB`, `_DoDigOutB`, `_RaidInB/OutB`, `_DoNestFightB`, `_QueenMoveB`, e das diferenças do ninho vermelho.
- Significado dos grupos 10 (Replete), 11 e 15, e de onde surgem.
- Modo 17 (hipótese: afogamento) e modo 18.
- Efeito exato de trocar o conjunto memorizado no triângulo de castas.
- Formiga amarela, aranha, formiga-leão e lagarta: documentos próprios.
