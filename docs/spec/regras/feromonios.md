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

Detalhes de borda (**fato visto no código**, `1028:91de`–`1028:95b6`):

- Nenhuma dessas funções confere limites: a célula é `(x >> 1) × 32 + (y >> 1)` direto. Os chamadores só passam tiles da superfície.
- `_JamScent*` gravam quando `c < v` e `_AlarmHere2` quando `c ≤ v`; o efeito é `max(c, v)`. O valor vem do campo S (byte, 0–255) ou de uma constante (40).
- `_AlarmHere` grava `min(c + v, 200)` sem olhar `c`: uma célula que já passava de 200 (ferramenta Experimental) **cai** para 200 quando a amarela solta alarme ali.
- `_FillHoles*`: `_HoleMapB/R` é indexado pela linha `y` da superfície (que é a coluna do ninho, ver [geracao-mundo.md](geracao-mundo.md)) e guarda o `x` do buraco; 0 = sem buraco. A varredura é de `y` = 0 a 63; o tile testado é `_MapA[x][y]`.
- `_DecTSmell(x, y, cor)`: cor = bit 7 do tipo da formiga (0 = trilha preta, senão vermelha).

**Valor depositado: o contador S da formiga.** O deslocamento acontece antes do depósito: a cada passo, se S > 0, a formiga faz S −= 1 e então deposita S na célula de destino. Assim a trilha forma um gradiente que **decresce** a partir da origem:

- **Cheiro de ninho:** a formiga sai do buraco com S = 120, se o buraco fica do lado da sua cor e o modo não é 3, 6 nem 7 (`_ExitHole` `1020:2db6`). Ela vai deixando 119, 118… enquanto se afasta. O buraco recebe 255 periodicamente (`_FillHoles*`). Resultado: o valor máximo fica no buraco.
- **Trilha:** ao pegar comida, S = 200 (`_DoRandAntA`, `_DoForageAnt`), e a formiga volta ao ninho deixando 199, 198… O maior valor fica junto da comida. A amarela usa `_MeScent`, que vale 200 ao pegar comida (`_PickupMyFood` `1020:8f4c`). Ela só deposita na superfície, carregando comida (`_MeType` 0x18 ou 0x38), e deposita **antes** de decrementar. O contador cai 1 por passo, mas não desce de 10 (`_DoAntMoveY` `1028:75f4`). A cor da trilha vem do campo de cor da amarela (`_grndTile`+2).

## 3. Decaimento

**Fato visto no código.**

| Mapa | Função | Quando | Regra por célula |
|---|---|---|---|
| BN | `_ColonySmellBN` `1028:92aa` | tick ≡ 32 (mod 128), logo depois de `_FillHolesBN` | se c > 0: c −= 1 |
| RN | `_ColonySmellRN` `1028:92d8` | tick ≡ 96 (mod 128) | idem |
| BT | `_ColonySmellBT` `1028:9306` | tick ≡ 32 (mod 128) | se c < 8: c = 0; senão c = c − ⌊c/2⌋ (metade, arredondada para cima). `_ColonySmellRT` testa `c ≠ 0` antes; o resultado é o mesmo |
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

**Fato visto no código** (`1028:9380`): a cópia inteira é feita antes (primeiro laço), e o segundo laço lê só a cópia e grava só o mapa; a varredura é coluna a coluna, mas, como lê a cópia, a ordem não importa. Os 4 vizinhos são somados como inteiros sem sinal; não há volta nas bordas (diferente da leitura de direção, seção 4).

**"Tick" quer dizer `_Cycle`, não `_GameTime`** (fato visto no código, `_DoAntSim`): `_Cycle` vai de 0 a 0x1000 e então volta a 0. Os valores 0x1000 e 0 são ambos ≡ 0 (mod 128), então a suavização do alarme roda em dois ticks seguidos a cada 4097 ticks, e daí em diante as fases ficam defasadas de `_GameTime` em 1 por volta. **Conferido no original (2026-10-19, vetor `volta_cycle`, 307/307):** com `_Cycle` = 4095 escrito na entrada, os ticks 4095 → 0x1000 e 0x1000 → 0 rodam os dois a fase 0 (`_ListIndexA` 108 → 57 → 56 pela compactação, `_HistStart` 25 → 26 → 27, a alimentação de `_HealthB` nos dois); o seguinte (0 → 1) não.

[hipótese] Meia-vida aproximada: a trilha cai pela metade a cada 128 ticks, além do desgaste por uso. O cheiro de ninho só perde 1 a cada 128 ticks, ou seja, dura muito.

## 4. Leitura (escolha de direção)

**Fato visto no código.** As funções de direção ficam em `SIMTWO_MODULE`. A vizinhança usa `_Dx8/_Dy8` na grade 64 × 32 com coordenadas "dando a volta" (`& 0x3F` e `& 0x1F`); a borda real é tratada antes, forçando a formiga a apontar para dentro.

| Função | Mapa lido | Regra |
|---|---|---|
| `_GetForageDir` (`1030:0ab0`) | trilha da própria cor | maior vizinho; se todos forem 0, passeio aleatório (`_TurnTab[dir][SRand8]`); se o maior vizinho for **menor** que a célula atual, devolve −1 ("acabou a trilha"); senão gira no máximo 45° rumo ao maior (`_TurnTab`) |
| `_GetNestDir` (`1030:0c30`) | cheiro de ninho da cor pedida | célula atual = 0: em 3/4 das vezes aponta para o último buraco criado (`_LastNewHoleB/R`), senão passeia; senão, maior vizinho, com giro limitado em metade das vezes. `_DoAttackAnt` pede o mapa da **outra** cor e por isso marcha para o ninho inimigo |
| `_GetAlarmDir` (`1030:0e54`) | alarme | maior vizinho, com giro limitado; sem alarme ao redor, passeio aleatório |
| `_GetSmellT` (`1028:9612`) | trilha de uma cor numa direção | recebe coordenadas de **célula**; devolve 0 fora do mapa (sem dar a volta). Sem chamada encontrada |

Detalhes vistos no código (pseudocódigo próprio). `cx = x >> 1`, `cy = y >> 1`; "girar para a" = `_TurnTab[dir][a]` (tabela em [formigas.md](formigas.md#7-movimento-e-escolha-de-direção), extraída para `data/feromonios.json`). "Maior vizinho" percorre as direções 0 a 7 e só troca quando o valor é **estritamente** maior: em empate vence a direção de menor número; com todos os vizinhos em 0 o índice fica 0.

Bordas (antes de qualquer leitura de mapa; x de tile 0–127, y 0–63):

| Posição | `_GetForageDir` | `_GetNestDir`, `_GetAlarmDir` (e `_GetRandDir`) |
|---|---|---|
| canto (0, 0) | 3, sem sorteio | `SRand1(3)` + 2 → 2–4 |
| canto (0, 63) | 1, sem sorteio | `SRand1(3)` → 0–2 |
| canto (127, 0) | 5, sem sorteio | `SRand1(3)` + 4 → 4–6 |
| canto (127, 63) | 7, sem sorteio | (`SRand1(3)` + 6) & 7 → 6, 7, 0 |
| x = 0 | `SRand1(3)` + 1 → 1–3 | `SRand1(5)` → 0–4 |
| y = 0 | `SRand1(3)` + 3 → 3–5 | `SRand1(5)` + 2 → 2–6 |
| x = 127 | `SRand1(3)` + 5 → 5–7 | (`SRand1(5)` + 4) & 7 → 4–7, 0 |
| y = 63 | (`SRand1(3)` − 1) & 7 → 7, 0, 1 | (`SRand1(5)` + 6) & 7 → 6, 7, 0–2 |

Interior, com a ordem exata dos sorteios:

- `_GetForageDir`: lê a célula atual; **sorteia `SRand8` sempre** (o valor é descartado: só serviria se nenhum vizinho fosse maior que 0, e nesse caso é sobrescrito); acha o maior vizinho; se ele é 0, sorteia outro `SRand8` e gira para ele; se é menor que a célula atual, devolve −1; senão gira para ele.
- `_GetNestDir`: se a célula atual é 0, calcula a direção até `_LastNewHoleB/R` (`_GetDir` `1020:10cc`, só pelos sinais de dx e dy). Se a formiga não está no buraco, sorteia `SRand4`; se ≠ 0, gira para o buraco. Caso contrário (ou se está no buraco, sem sortear `SRand4`), gira para `SRand8`. Se a célula atual tem cheiro: maior vizinho, depois `SRand2`; se ≠ 0, `dir` = girar para o maior; e então devolve girar para o maior. Ou seja, **o giro é aplicado duas vezes em metade das vezes (até 90°) e uma vez na outra metade (até 45°)**; a versão anterior desta spec dizia "giro limitado em metade das vezes".
- `_GetAlarmDir`: maior vizinho do alarme; se for 0, gira para `SRand8`; senão gira para ele. Não olha a célula atual.
- A cor vem do bit 7 do tipo da formiga (`_GetForageDir`: `tipo & 0x80`; `_GetNestDir`: `tipo >> 7`).

Gatilhos de modo ligados a cheiro (`_DoAntSimA` e companhia):

- Foraging (2) numa célula com **alarme > 0** passa a Alarmed (11).
- Alarmed (11) numa célula **sem alarme**: com chance 1/4 sorteia novo modo; senão continua subindo o gradiente.
- Defense (6) sobe o gradiente do alarme se houver alarme na célula; senão segue a amarela.
- Defend Nest (12): cheiro de ninho da própria cor < 110 → volta ao ninho; senão passeia (fica perto do ninho).
- Take off (15): cheiro de ninho < 100 → volta ao ninho; senão passeia.

## 5. Estado inicial e verificação

- **Verificado** no `QUICK2.ANT` (tick 78): `_PherMapA`, `_PherMapBT`, `_PherMapRN` e `_PherMapRT` estão zerados; `_PherMapBN` tem uma única célula com 254, a do buraco preto (3, 32) → célula (1, 16). É o esperado: no tick 32, `_FillHolesBN` pôs 255 e `_ColonySmellBN` tirou 1. `_PherMapRN` deveria ter recebido o mesmo no tick 96, que ainda não tinha chegado.
- **Verificado** (caso SUP-04 da bancada, 2026-10-17): no tick 130 a célula do buraco vermelho (118, 32) → (59, 16) está com 254 em `_PherMapRN`; no mesmo caso, trilha vermelha 210 → 105 no tick 96, trilha preta intacta até o tick 160 e um alarme isolado de 200 vira 100 no centro e 25 nas 4 vizinhas no tick 128. O port dá o mesmo (`sup04_feromonios_iguais_ao_original`, `engine/tests/regras_superficie.rs`).

## 6. Como testar contra o original

Bancada: [docs/bancada-qemu.md](../../bancada-qemu.md).

1. **Decaimento da trilha.** Modo Experimental: use a ferramenta de cheiro na camada "Black Trail" numa célula longe das formigas até o valor saturar (3 cliques = 210, se partir de 0). Salve logo depois e de novo após 128, 256 e 384 ticks (controle por `_GameTime`). Esperado: 210 → 105 → 53 → 27 (para em 0 abaixo de 8), descontando o desgaste se alguma formiga passar ali.
2. **Suavização do alarme.** Ponha alarme numa célula isolada (camada Alarm, ferramenta Experimental) e salve antes e depois de um tick ≡ 0 ou 64 (mod 128). Compare a vizinhança com a fórmula da seção 3.
3. **Gradiente de ninho.** Em jogo Quick, salve depois de algumas centenas de ticks e trace os valores de `_PherMapBN` a partir do buraco: devem ficar perto de 255 no buraco e cair de 1 em 1 a partir de 120 ao longo dos caminhos de saída.
4. **Comparação com o port.** Os 5 mapas são blocos contíguos do `.ANT` (offsets 34576…42768, 2048 bytes cada). Um teste do port pode carregar o `.ANT`, aplicar só as funções de decaimento e suavização num tick ≡ 32, 64 ou 96 e comparar byte a byte com o save do original tirado no mesmo tick. Essas funções não usam o RNG, então o teste é determinístico.

## 7. Implementação no port

Código: `engine/src/rules/feromonios.rs`. Dados: `data/feromonios.json`, gerado por `tools/extrair_feromonios.py SIMANTW.EXE --json data/feromonios.json` (`_Dx8` `8:0000`, `_Dy8` `8:0008`, `_TurnTab` `8:0024`, mais as constantes 8, 9, 200, 255 e 0x51 vistas no pseudocódigo, no bloco `regras`). Um mod pode trocar a tabela de giro ou os limiares sobrescrevendo esse arquivo.

| Original | Port |
|---|---|
| `_PherMapA/BN/BT/RN/RT` | `Feromonios { alarme, ninho_preto, trilha_preta, ninho_vermelho, trilha_vermelha }`, cada um `MapaCheiro` (2048 bytes, coluna-major) |
| leitura/gravação no `.ANT` | `Feromonios::do_save`, `Feromonios::gravar_no_save` |
| `_JamScent*`, `_AlarmHere2` | `MapaCheiro::depositar`, `MapaCheiro::alarme_de_luta` |
| `_AlarmHere` | `MapaCheiro::somar_alarme` |
| `_DecTSmell` | `MapaCheiro::gastar` (na camada de trilha da cor) |
| `_ColonySmellBN/RN`, `_ColonySmellBT/RT` | `MapaCheiro::decair_ninho`, `MapaCheiro::decair_trilha` |
| `_SmoothAlarm` | `MapaCheiro::suavizar_alarme` |
| `_FillHolesBN/RN` | `MapaCheiro::preencher_buracos` |
| parte de cheiro de `_DoSmells` | `fase_do_ciclo`, `Feromonios::rodar_fase`, `Feromonios::etapa_do_tick` |
| `_GetSmellT` | `Feromonios::trilha_na_direcao` |
| `_GetForageDir`, `_GetNestDir`, `_GetAlarmDir` | `direcao_forrageio`, `direcao_ninho`, `direcao_alarme` (com `GeradorS`) |

Decisões do port:

- Coordenadas fora da superfície: o original não confere e escreveria fora do mapa; o port ignora a escrita e lê 0 (`celula_do_tile` devolve `None`). Um buraco com `x` ≥ 128 em `_HoleMapB/R` também é ignorado. Nenhum caso foi visto no jogo.
- `_ExpIncSmell` (ferramenta Experimental) não foi portada: depende da interface (`GetAsyncKeyState`, `_SmoothMany`, som).

Testes: `cargo test -p engine` roda os casos construídos a partir desta spec (módulo `feromonios`) e `engine/tests/feromonios_saves.rs`:

- leitura e gravação dos 5 mapas preservam o `QUICK2.ANT` e o `QUICK3.ANT` byte a byte;
- o estado da seção 5 (só a célula do buraco preto com 254) nos dois saves;
- `QUICK2.ANT` avançado só com as etapas de cheiro: no `_Cycle` 96 a célula do buraco vermelho (59, 16) fica com 254 (previsão do port para o item em aberto da seção 5, ainda não conferida no original);
- invariantes com mapas cheios de valores pseudoaleatórios (decaimento nunca aumenta; alarme suavizado ≤ máximo anterior e ≥ 9 ou 0);
- `vetores_de_tick_do_original`: para cada par `*antes*.ANT`/`*depois*.ANT` em `tests/referencias/ticks/` (qualquer subpasta), aplica ao "antes" só a etapa de cheiro do `_Cycle` do "depois" e compara os 5 mapas com o "depois". Diferenças em células a até 1 tile de uma formiga da lista A (antes ou depois) ou da amarela na superfície são contadas e ignoradas, porque as formigas depositam e gastam cheiro no mesmo tick. Sem arquivos, o teste avisa e passa. **Ainda não rodou com dados do original.**

## Lacunas

- Tile 0x51 (`'Q'`) em `_FillHoles*`: significado (ver `tiles-valores.md`).
- Leitura dos cheiros pela formiga amarela e pela aranha (`_SpiderScan`), se houver.
- Mapa de saídas dos ninhos (`_ExitMapB/R`, `_FixExitMapB/R`, `_GetExitDirB`, `_GetEnterDirB`): é o "cheiro" interno do ninho e merece documento próprio.
- Qual tecla diminui o cheiro na ferramenta Experimental (`GetAsyncKeyState` sem o argumento visível no pseudocódigo).
- Giro duplo de `_GetNestDir`: nos vetores de tick do repositório (todos 307/307), as carregadoras da superfície em modo 3 que seguem no modo 3 mudam a direção em 0 (196 casos), ±45° (359), **±90° (91)** e 135° (2) num tick (contagem de 2026-10-19 fora dos ticks de compactação). Os ±90° não caberiam num giro só de `_GetNestDir`; como o port reproduz esses ticks inteiros, o giro duplo está confirmado de forma indireta (sem um caso isolado só dele). A suavização dupla na volta de `_Cycle` foi conferida em 2026-10-19 (`volta_cycle`).
- `_GetRandDir`, `_GetDefendDir` e `_GetRedDefendDir` estão em `rules/superficie.rs` (privadas); a direção dentro dos ninhos ainda não foi portada.
- As etapas do tick (`_FillHoles*`, `_ColonySmell*`, `_SmoothAlarm`) são chamadas por `rules/tick.rs` (fases 1 e 3 e `_FullCount`) via `superficie::rodar_fase_de_cheiro`, que converte os mapas do `Estado` (`Grade`) para `feromonios::Feromonios` e de volta.
