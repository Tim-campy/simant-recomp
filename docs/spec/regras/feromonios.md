# Feromônios (mapas de cheiro)

Este documento descreve os cinco mapas de feromônio da superfície: formato, como os valores são depositados, como decaem e como as formigas os leem para escolher direção. O ritmo de cada operação vem do tick descrito em [simulacao-visao-geral.md](simulacao-visao-geral.md); os modos que usam cada mapa estão em [formigas.md](formigas.md).

Convenção de evidência: igual à de [simulacao-visao-geral.md](simulacao-visao-geral.md#convenção-de-evidência).

## 1. Os mapas

**Fato visto no código** e tabela do save ([save-ant.md](../formatos/save-ant.md)). Nomes das camadas tirados da lista de texto SHARED 4/1000, descomprimida: "Black Nest Marker Pheromone", "Black Trail Marker Pheromone", "Red Nest Marker Pheromone", "Red Trail Marker Pheromone", "Alarm Signal Pheromone".

| Mapa | Endereço | Nome no jogo | Cor | Uso |
|---|---|---|---|---|
| `_PherMapA` | `8:52d2` | Alarm Signal | comum às duas | alarme: atrai Alarmed e Defense |
| `_PherMapBN` | `8:62d2` | Black Nest Marker | preta | gradiente que leva ao ninho |
| `_PherMapBT` | `8:6ad2` | Black Trail Marker | preta | trilha até a comida |
| `_PherMapRN` | `8:72d2` | Red Nest Marker | vermelha | idem, vermelha |
| `_PherMapRT` | `8:7ad2` | Red Trail Marker | vermelha | idem, vermelha |

- Cada mapa tem **2048 bytes = 64 × 32 células**, uma por bloco de 2 × 2 tiles da superfície (128 × 64). Valores de 0 a 255, sem sinal.
- Índice: `(x >> 1) × 32 + (y >> 1)`. No código aparece também como `(x & 0xFE) × 16 + (y >> 1)`, que dá o mesmo valor.
- Só existem na **superfície**. Os ninhos não têm mapa de cheiro; lá se usa o mapa de saídas `_ExitMapB/R` (`8:03a4`/`8:13a4`, 64 × 64), assunto de outro documento [em aberto].
- Mapa auxiliar: `_TemDensity` (`8:4ad2`, 2048 bytes), cópia usada na suavização do alarme. Não vai para o save.

## 2. Depósito

**Fato visto no código.** Pseudocódigo próprio; `c(x, y)` é a célula do mapa que contém o tile (x, y).

| Função | Endereço | Regra | Quem chama |
|---|---|---|---|
| `_JamScentBN` / `_JamScentRN` | `1028:94b6` / `1028:94f6` | `c = max(c, v)` | formiga da cor dada, a cada passo, nos modos Bored (0), Nesting/Dig In (1/4), Foraging (2) e Dig Out (5) (`_DoRandAntA`, `_DoToNestAnt`, `_DoForageAnt`, `_DoDigOutAntA`) |
| `_JamScentBT` / `_JamScentRT` | `1028:9536` / `1028:9576` | `c = max(c, v)` | `_DoReturnFoodAnt` (Got Food, 3); a formiga amarela (`_DoAntMoveY`, `_PickupMyFood`) |
| `_AlarmHere` | `1028:943c` | `c = min(c + v, 200)` | formiga amarela soltando alarme: v = 50 por passo (`_DoAntMoveY`, com `_MeDropAlarm` ligado; a tecla é o zero, segundo o tutorial 4/11052–11054) |
| `_AlarmHere2` | `1028:947e` | `c = max(c, v)` | toda luta que começa: v = 40 na célula da nuvem (`_DoRandAntA`, `_DoRandAntAA`, `_DoToNestAnt`, `_DoToAlarm`, `_DoForageAnt`, `_DoRecruitAnt`, `_StartFightA`, `_DoAttackAnt`) |
| `_FillHolesBN` / `_FillHolesRN` | `1028:91de` / `1028:9244` | para cada buraco da cor (`_HoleMapB/R`), `c = 255`, ou `c = 0` se o tile do buraco for 0x51 `'Q'` [hipótese: buraco tampado/inativo] | tick ≡ 32 (preto) e ≡ 96 (vermelho), mod 128 |
| `_ExpIncSmell` | `1010:7b9e` | ferramenta do modo Experimental: `c = c + 70`, ou `c − 40` com uma tecla apertada; limitado a 0..255. A camada vem da seleção do menu (`8:8605`: 0 = BN, 1 = BT, 2 = RN, 3 = RT, 4 = A) | interface |

**Valor depositado: o contador S da formiga.** O deslocamento acontece antes do depósito: a cada passo, se S > 0, a formiga faz S −= 1 e então deposita S na célula de destino. Assim a trilha forma um gradiente que **decresce** a partir da origem:

- **Cheiro de ninho:** a formiga sai do buraco com S = 120, se o buraco fica do lado da sua cor e o modo não é 3, 6 nem 7 (`_ExitHole` `1020:2db6`). Ela vai deixando 119, 118… enquanto se afasta. O buraco recebe 255 periodicamente (`_FillHoles*`). Resultado: o valor máximo fica no buraco.
- **Trilha:** ao pegar comida, S = 200 (`_DoRandAntA`, `_DoForageAnt`), e a formiga volta ao ninho deixando 199, 198… O maior valor fica junto da comida. A amarela usa `_MeScent`, que vale 200 ao pegar comida (`_PickupMyFood` `1020:8f4c`). Ela só deposita na superfície, carregando comida (`_MeType` 0x18 ou 0x38), e deposita **antes** de decrementar. O contador cai 1 por passo, mas não desce de 10 (`_DoAntMoveY` `1028:75f4`). A cor da trilha vem do campo de cor da amarela (`_grndTile`+2).

## 3. Decaimento

**Fato visto no código.**

| Mapa | Função | Quando | Regra por célula |
|---|---|---|---|
| BN | `_ColonySmellBN` `1028:92aa` | tick ≡ 32 (mod 128), logo depois de `_FillHolesBN` | se c > 0: c −= 1 |
| RN | `_ColonySmellRN` `1028:92d8` | tick ≡ 96 (mod 128) | idem |
| BT | `_ColonySmellBT` `1028:9306` | tick ≡ 32 (mod 128) | se c < 8: c = 0; senão c = c − ⌊c/2⌋ (metade, arredondada para cima) |
| RT | `_ColonySmellRT` `1028:9344` | tick ≡ 96 (mod 128) | idem |
| BT/RT | `_DecTSmell` `1028:95b6` | a cada passo de uma formiga em Bored (0) ou Foraging (2), na célula de destino | se c > 0: c −= 1 (a trilha "gasta" com o uso) |
| A | `_SmoothAlarm` `1028:9380` | ticks ≡ 0 e ≡ 64 (mod 128) | suavização (abaixo) |

**Suavização do alarme** (todas as células calculadas a partir da cópia anterior, `_TemDensity`):

```
para cada célula (i, j) do mapa 64×32:
    vizinhos = soma dos 4 vizinhos ortogonais da cópia (fora do mapa conta 0)
    novo = (cópia[i][j] + vizinhos/4) / 2          (divisões inteiras)
    se novo < 9: novo = 0
```

O alarme se espalha e perde intensidade. Um pico isolado de 200 vira 100 no centro e 25 nos vizinhos depois de uma passada.

[hipótese] Meia-vida aproximada: a trilha cai pela metade a cada 128 ticks, além do desgaste por uso. O cheiro de ninho só perde 1 a cada 128 ticks, ou seja, dura muito.

## 4. Leitura (escolha de direção)

**Fato visto no código.** As funções de direção ficam em `SIMTWO_MODULE`. A vizinhança usa `_Dx8/_Dy8` na grade 64 × 32 com coordenadas "dando a volta" (`& 0x3F` e `& 0x1F`); a borda real é tratada antes, forçando a formiga a apontar para dentro.

| Função | Mapa lido | Regra |
|---|---|---|
| `_GetForageDir` (`1030:0ab0`) | trilha da própria cor | maior vizinho; se todos forem 0, passeio aleatório (`_TurnTab[dir][SRand8]`); se o maior vizinho for **menor** que a célula atual, devolve −1 ("acabou a trilha"); senão gira no máximo 45° rumo ao maior (`_TurnTab`) |
| `_GetNestDir` (`1030:0c30`) | cheiro de ninho da cor pedida | célula atual = 0: em 3/4 das vezes aponta para o último buraco criado (`_LastNewHoleB/R`), senão passeia; senão, maior vizinho, com giro limitado em metade das vezes. `_DoAttackAnt` pede o mapa da **outra** cor e por isso marcha para o ninho inimigo |
| `_GetAlarmDir` (`1030:0e54`) | alarme | maior vizinho, com giro limitado; sem alarme ao redor, passeio aleatório |
| `_GetSmellT` (`1028:9612`) | trilha de uma cor numa direção | sem chamada encontrada |

Gatilhos de modo ligados a cheiro (`_DoAntSimA` e companhia):

- Foraging (2) numa célula com **alarme > 0** passa a Alarmed (11).
- Alarmed (11) numa célula **sem alarme**: com chance 1/4 sorteia novo modo; senão continua subindo o gradiente.
- Defense (6) sobe o gradiente do alarme se houver alarme na célula; senão segue a amarela.
- Defend Nest (12): cheiro de ninho da própria cor < 110 → volta ao ninho; senão passeia (fica perto do ninho).
- Take off (15): cheiro de ninho < 100 → volta ao ninho; senão passeia.

## 5. Estado inicial e verificação

- **Verificado** no `QUICK2.ANT` (tick 78): `_PherMapA`, `_PherMapBT`, `_PherMapRN` e `_PherMapRT` estão zerados; `_PherMapBN` tem uma única célula com 254, a do buraco preto (3, 32) → célula (1, 16). É o esperado: no tick 32, `_FillHolesBN` pôs 255 e `_ColonySmellBN` tirou 1. `_PherMapRN` deveria ter recebido o mesmo no tick 96, que ainda não tinha chegado.
- [em aberto] Conferir no próximo save (tick ≥ 96) que a célula do buraco vermelho (118, 32) → (59, 16) aparece com 254 em `_PherMapRN`.

## 6. Como testar contra o original

Bancada: [docs/bancada-qemu.md](../../bancada-qemu.md).

1. **Decaimento da trilha.** Modo Experimental: use a ferramenta de cheiro na camada "Black Trail" numa célula longe das formigas até o valor saturar (3 cliques = 210, se partir de 0). Salve logo depois e de novo após 128, 256 e 384 ticks (controle por `_GameTime`). Esperado: 210 → 105 → 53 → 27 (para em 0 abaixo de 8), descontando o desgaste se alguma formiga passar ali.
2. **Suavização do alarme.** Ponha alarme numa célula isolada (camada Alarm, ferramenta Experimental) e salve antes e depois de um tick ≡ 0 ou 64 (mod 128). Compare a vizinhança com a fórmula da seção 3.
3. **Gradiente de ninho.** Em jogo Quick, salve depois de algumas centenas de ticks e trace os valores de `_PherMapBN` a partir do buraco: devem ficar perto de 255 no buraco e cair de 1 em 1 a partir de 120 ao longo dos caminhos de saída.
4. **Comparação com o port.** Os 5 mapas são blocos contíguos do `.ANT` (offsets 34576…42768, 2048 bytes cada). Um teste do port pode carregar o `.ANT`, aplicar só as funções de decaimento e suavização num tick ≡ 32, 64 ou 96 e comparar byte a byte com o save do original tirado no mesmo tick. Essas funções não usam o RNG, então o teste é determinístico.

## Lacunas

- Tile 0x51 (`'Q'`) em `_FillHoles*`: significado (ver `tiles-valores.md`).
- Leitura dos cheiros pela formiga amarela e pela aranha (`_SpiderScan`), se houver.
- Mapa de saídas dos ninhos (`_ExitMapB/R`, `_FixExitMapB/R`, `_GetExitDirB`, `_GetEnterDirB`): é o "cheiro" interno do ninho e merece documento próprio.
- Qual tecla diminui o cheiro na ferramenta Experimental (`GetAsyncKeyState` sem o argumento visível no pseudocódigo).
