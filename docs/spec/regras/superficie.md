# Superfície: regras das formigas do quintal e da casa (lista A)

Este documento especifica, para reimplementação 1:1, tudo o que acontece com as formigas da **lista A** (superfície) dentro de um tick: o laço `_DoAntSimA`, cada modo, as funções de direção, a luta, a morte, a entrada no ninho, a comida, a estratégia vermelha e preta e a formiga vermelha líder. Os bichos do quintal (formiga-leão, aranha, lagarta, bicho-de-conta) estão em [inimigos.md](inimigos.md); a formiga amarela e os controles em [formiga-amarela.md](formiga-amarela.md). A ordem das sub-rotinas dentro do tick está em [simulacao-visao-geral.md](simulacao-visao-geral.md); listas, codificação de T e tabelas de modo em [formigas.md](formigas.md); mapas de cheiro em [feromonios.md](feromonios.md); geradores em [rng.md](rng.md).

## Convenção de evidência

Igual à de [simulacao-visao-geral.md](simulacao-visao-geral.md#convenção-de-evidência). Resumo:

- **Fato visto no código:** pseudocódigo do Ghidra em `re_trabalho/decomp/Code*.c`, citado como `seletor:offset` e nome do `SIMANTW.SYM`. Endereços de variáveis no formato `segmento lógico:offset` do `symbols.tsv` (8 = seletor `1038`, 9 = `1040`, 10 = `1048`/DGROUP).
- **Fato verificado:** conferido no `SIMANTW.EXE` (tabelas) ou no save `tests/referencias/saves/QUICK2.ANT`, com o método descrito.
- **Fato visto na desmontagem:** quando o pseudocódigo era ambíguo, os bytes do `SIMANTW.EXE` foram desmontados (capstone, 16 bits; seg. 6 = seletor `1028`, no arquivo em 385024 = 0x5E000).
- **[hipótese]**, **[em aberto]**: como no [README](../README.md).

Nenhum trecho decompilado é reproduzido: tudo está em prosa, tabelas e pseudocódigo próprio.

### Notação usada no pseudocódigo

| Notação | Significado |
|---|---|
| `X[i]`, `Y[i]`, `T[i]`, `M[i]`, `S[i]` | campos da vaga `i` da lista A (`_AlistX/Y/T/M/S`, ver [formigas.md](formigas.md#2-listas-paralelas)) |
| `Vida(x, y)` | `_LifeA[x·64 + y]` |
| `Mapa(x, y)` | `_MapA[x·64 + y]` |
| `cor(T)` | `T & 0x80` (0 = preta, 0x80 = vermelha) |
| `grupo(T)` | `(T & 0x78) >> 3` |
| `dir(T)` | `T & 7` |
| `base(T)` | `T & 0xF8` (cor + grupo; a direção é reposta depois) |
| `D8[d]` | deslocamento `(_Dx8[d], _Dy8[d])` (`8:0000`/`8:0008`) |
| `Giro[d][k]` | `_TurnTab[d·8 + k]` (`8:0024`, tabela em [formigas.md](formigas.md#7-movimento-e-escolha-de-direção)) |
| `Barreira` | `_Barrier` (`9:7604`): 0x50 no quintal, 0x90 na casa |
| `Casa` | `_TERRAINset` (`9:9b6e`) = 1 |
| `S1(n)`, `S2()`, `S4()`, `S8()`, `S16()`, `S32()`, `S128()`, `S256()` | `_SRand1(n)`, `_SRand2()` … do gerador S ([rng.md](rng.md)); cada chamada é **um** passo |
| `R(n)` | `_RRand(n)` do gerador R |
| `CelCheiro(x, y)` | célula `(x >> 1)·32 + (y >> 1)` dos mapas 64 × 32 |
| `Alarme(x, y)`, `NinhoB/R(x, y)`, `TrilhaB/R(x, y)` | `_PherMapA`, `_PherMapBN/RN`, `_PherMapBT/RT` em `CelCheiro(x, y)` |

**Regra de ouro do determinismo:** toda chamada `S*`/`R` listada abaixo acontece exatamente na ordem e nas condições escritas. Chamadas condicionadas por curto-circuito (`A e B`) só acontecem quando a condição anterior é verdadeira; isso está explícito em cada caso.

## 1. O laço `_DoAntSimA` (`1028:04d8`)

**Fato visto no código.** Pseudocódigo próprio:

```
_Tindex (9:9b6a) = _ListIndexA
enquanto _Tindex > 0:
    _Tindex -= 1 ; i = _Tindex
    r = S256()                                   ← para TODA vaga, inclusive T = 0
    se r == 0 e T[i] ≠ 0:
        saúde = (_HealthB se cor(T[i]) = 0, senão _HealthR)
        se saúde < S32():                        ← S32 só é sorteado aqui
            _DeadAntHere(X[i], Y[i], cor(T[i]))  (seção 6.4)
            T[i] = 0
            _BAntsExpired (9:9b26, 32 bits) += 1  ← também para vermelhas
    se T[i] ≠ 0:
        conta o modo: _TemBModePop[M[i]] += 1 (preta) ou _TemRModePop[M[i]] += 1 (vermelha)
        despacha por M[i] (tabela abaixo)
```

| M | Ação | Seção |
|---|---|---|
| 0 | `_DoRandAntA` | 3.1 |
| 1, 4 | `_DoToNestAnt` | 3.3 |
| 2 | `_DoForageAnt` | 3.4 |
| 3 | `_DoReturnFoodAnt` | 3.5 |
| 5 | `_DoDigOutAntA` | 3.6 |
| 6 | `_DoRecruitAnt` | 3.7 |
| 7 | `_DoAttackAnt` | 3.8 |
| 8 | ovo na superfície (embutido; igual a `_SimEggA`) | 3.9 |
| 9 | rainha na superfície (embutido; igual a `_SimQueenA`) | 3.10 |
| 10 | `_DoFightA` | 6.3 |
| 11 | `_DoToAlarm` | 3.11 |
| 12 | defender o ninho (embutido; igual a `_DoDefendNest`) | 3.12 |
| 13 | descanso (embutido; igual a `_DoRestAnt`) | 3.13 |
| 14 | pré-voo (embutido; igual a `_DoRepoLoit`) | 3.14 |
| 15 | `_DoRepoExit` | 3.15 |
| 16 | voo (embutido; igual a `_DoRepoFly`) | 3.16 |
| 17, 18 | **nada** (não há caso no `switch`; a formiga fica parada e só é contada) | — |
| 19 | `_DoRedInitiator` | 9.2 |

O despacho não olha a cor: pretas e vermelhas usam as mesmas funções; a cor só muda qual mapa de cheiro é lido/escrito e quem é inimigo.

## 2. Peças comuns

### 2.1 Teste de buraco ("entra no ninho")

**Fato visto no código** (repetido no início de quase todos os modos). A formiga está num buraco se `(x, y)` está dentro do mapa (`_IsValidA` `1020:9c02`: 0 ≤ x < 128, 0 ≤ y < 64) e:

- quintal: `Mapa(x, y) == 0x50`;
- casa: `0x80 ≤ Mapa(x, y) < 0x90`.

Se estiver, chama `_GoInNest(x, y, i)` (seção 7) e o modo termina sem nenhum sorteio. Fazem esse teste, logo no início: os modos 0, 0', 1/4, 2, 3, 6, 7, 11 e 13; os modos 12, 14 e 15 o fazem dentro da função que chamam (`_DoToNestAnt` ou `_DoRandAntAA`). **Não** fazem: 5 (`_DoDigOutAntA`), 8, 9, 10, 16 e 19.

### 2.2 Giro sem movimento

Quando a formiga não pode ou não quer andar, ela gira: `T[i] = base(T) | Giro[dir(T)][S8()]` e `Vida(x, y) = T[i]`. É sempre a direção **original** da formiga (antes do sorteio de direção do passo) que indexa a tabela. Uma exceção (`_DoRecruitAnt`, seção 3.7) usa a direção sorteada pura, sem tabela.

### 2.3 O passo padrão

**Fato visto no código.** Cada modo móvel escolhe uma direção `d`, calcula o destino `(nx, ny) = (x, y) + D8[d]` e trata o destino nesta ordem (as variações de cada modo estão na tabela 2.4):

1. **Comida** (só nos modos que pegam comida; seção 8.1): destino é comida (`_IsItFood`: quintal 0x48–0x4B; casa 0x18–0x27) **e** `grupo(T)` é 2 (operária) ou 6 (soldado) → a formiga **não anda**: `T[i] = d | base(T) | 8` (grupo 2→3 ou 6→7), `Vida(x, y) = T[i]`, `M[i] = 3`, `_PickupFoodA(nx, ny)`, `S[i] = 200`. Fim. Se o destino é comida mas o grupo não é 2/6, o teste de barreira é **pulado** e segue para o passo 3.
2. **Barreira:** `Mapa(nx, ny) > Barreira` (comparação com sinal, valor do tile sem sinal) → giro (2.2). Fim.
3. **Célula livre** (`Vida(nx, ny) == 0`): anda. `Vida(nx, ny) = d | base(T)`; `T[i]` idem; `Vida(x, y) = 0`; `X[i], Y[i] = nx, ny`. Depois, se o modo deposita cheiro: se `S[i] > 0`, `S[i] -= 1` e deposita `S[i]` no mapa indicado, com `c = max(c, S)` (`_JamScent*`, [feromonios.md](feromonios.md)).
4. **Formiga amarela** no destino (`Vida` = 0xFF ou 0xFE, `_IsYellowAnt` `1020:5720`):
   - cor diferente da amarela (`(T ^ _MeColor) & 0x80`, `_MeColor` = `10:ce98`) → `_YellowFight(1, i)` ([formiga-amarela.md](formiga-amarela.md)). Fim.
   - mesma cor: nos modos com trofalaxia, se `_MeWantFood` (`9:9af2`) = 1 (no modo 6: ≠ 0), a formiga se vira para a amarela (`T[i] = d | base(T)`, `Vida(x, y) = T[i]`) e chama `_DoTroph(x, y, d)` (a amarela come; ver [formiga-amarela.md](formiga-amarela.md)). Depois, sempre, giro (2.2).
5. **Formiga da mesma cor** → giro (2.2); em alguns modos, também sorteia modo novo (`M[i] = _GetNewMode(grupo(T), T)`, seção 9.3), **depois** do giro.
6. **Inimiga** (cor diferente) → **luta** (6.1). Fim.

### 2.4 Variações por modo

| Modo | Função | Direção | Pega comida (1) | Cheiro ao andar (3) | `_DecTSmell` no destino | Trofalaxia (4) | Mesma cor (5) |
|---|---|---|---|---|---|---|---|
| 0 | `_DoRandAntA` | `_GetRandDir` | sim | ninho da própria cor | sim | sim (`= 1`) | giro + modo novo |
| 0' | `_DoRandAntAA` | `_GetRandDir` | não | não | não | não | giro |
| 1, 4 | `_DoToNestAnt` | `_GetNestDir` (própria cor) | sim | ninho da própria cor | não | sim (`= 1`) | giro |
| 2 | `_DoForageAnt` | `_GetForageDir` | sim | ninho da própria cor | sim | sim (`= 1`) | giro |
| 3 | `_DoReturnFoodAnt` | `_GetNestDir` (própria cor) | não | **trilha** da própria cor | não | — | — (ver 3.5) |
| 6 | `_DoRecruitAnt` | alarme / `_GetDefendDir` / `_GetRedDefendDir` | não | não | não | sim (`≠ 0`) | giro |
| 7 | `_DoAttackAnt` | `_GetNestDir` (cor **inimiga**) | não | não | não | não | giro |
| 11 | `_DoToAlarm` | `_GetAlarmDir` | não | não | não | não | giro |
| 19 | `_DoRedInitiator` | `_GetRedBestDirs` | não | não | não | não (sem teste de cor) | giro (`_RandTurn`) |

"0'" = `_DoRandAntAA` (`1028:1234`), o passeio "mudo" usado pelos modos 12, 14 e 15.

### 2.5 Funções de direção

**Fato visto no código** (todas em `SIMTWO_MODULE`, seletor `1030`). Todas recebem `(x, y, dir_atual)` e devolvem uma direção 0–7 (ou um código negativo, quando indicado).

#### 2.5.1 Tratamento de borda (padrão "Bounce")

`_Bounce` (`1030:12ec`), `_GetRandDir`, `_GetNestDir`, `_GetAlarmDir`, `_GetDefendDir` e `_GetRedDefendDir` começam pelo mesmo teste. Se a formiga está numa borda da superfície, sorteia **um** valor e devolve uma direção que aponta para dentro; senão, não sorteia nada e segue para a regra própria da função:

| Posição | Sorteio | Direções possíveis (resultado = base + sorteio, módulo 8) |
|---|---|---|
| x = 0, y = 0 | `S1(3)` | 2, 3, 4 |
| x = 0, y = 63 | `S1(3)` | 0, 1, 2 |
| x = 0, outro y | `S1(5)` | 0, 1, 2, 3, 4 |
| x = 127, y = 0 | `S1(3)` | 4, 5, 6 |
| x = 127, y = 63 | `S1(3)` | 6, 7, 0 |
| x = 127, outro y | `S1(5)` | 4, 5, 6, 7, 0 |
| y = 0, outro x | `S1(5)` | 2, 3, 4, 5, 6 |
| y = 63, outro x | `S1(5)` | 6, 7, 0, 1, 2 |

`_Bounce` devolve `1 + direção` (ou 0 fora da borda); as demais devolvem a direção.

`_GetForageDir` tem uma tabela de borda **diferente** (fato visto no código):

| Posição | Sorteio | Direção |
|---|---|---|
| x = 0, y = 0 | nenhum | 3 |
| x = 0, y = 63 | nenhum | 1 |
| x = 0, outro y | `S1(3)` | 1, 2, 3 |
| x = 127, y = 0 | nenhum | 5 |
| x = 127, y = 63 | nenhum | 7 |
| x = 127, outro y | `S1(3)` | 5, 6, 7 |
| y = 0, outro x | `S1(3)` | 3, 4, 5 |
| y = 63, outro x | `S1(3)` | 7, 0, 1 |

#### 2.5.2 Auxiliares `_GetDir` e `_GetDis`

- `_GetDir(x1, y1, x2, y2)` (`1020:10cc`): devolve `1 + direção` de (x1, y1) para (x2, y2) olhando **só o sinal** de dx e dy, ou 0 se os pontos coincidem. dx = 0: dy < 0 → 1 (norte), dy > 0 → 5 (sul). dx > 0: dy < 0 → 2, dy = 0 → 3, dy > 0 → 4. dx < 0: dy < 0 → 8, dy = 0 → 7, dy > 0 → 6.
- `_GetDis(x1, y1, x2, y2)` (`1020:1122`): `dx² + dy²` (multiplicação de 32 bits; os chamadores comparam o resultado de 16 bits, suficiente para o mapa).
- Nos mapas de cheiro, as 8 vizinhas de uma célula `(cx, cy)` são `((cx + Dx8[k]) & 63, (cy + Dy8[k]) & 31)`, isto é, o mapa "dá a volta" (a borda real já foi tratada antes).

#### 2.5.3 `_GetRandDir` (`1030:0f72`)

Borda (2.5.1); senão `Giro[dir][S8()]`.

#### 2.5.4 `_GetForageDir(x, y, dir, T)` (`1030:0ab0`)

```
borda: tabela própria (2.5.1)
atual = Trilha_cor(T)(x, y)
lixo  = S8()                     ← sorteado SEMPRE, resultado descartado se houver trilha
melhor = 0 ; k_melhor = 0
para k = 0..7: v = trilha da vizinha k ; se v > melhor: melhor = v, k_melhor = k   (empate: menor k)
se melhor == 0:  devolve Giro[dir][S8()]        ← segundo sorteio
se melhor < atual: devolve −1                   ← "fim da trilha"
devolve Giro[dir][k_melhor]
```

Fato visto no código. O primeiro `S8()` é consumido mesmo quando não serve para nada.

#### 2.5.5 `_GetNestDir(x, y, dir, T)` (`1030:0c30`)

Cor = `T >> 7` (0 preta, 1 vermelha). Borda (2.5.1). Depois:

```
atual = Ninho_cor(x, y)
se atual == 0:
    k = _GetDir(x, y, _LastNewHole_cor)          (_LastNewHoleB 8:835a / R 8:835e)
    se k ≠ 0 e S4() ≠ 0: devolve Giro[dir][k − 1]  ← S4 só se k ≠ 0
    devolve Giro[dir][S8()]
senão:
    melhor = 0; k_melhor = 0; para k = 0..7: maior valor estrito (empate: menor k)
    se S2() ≠ 0: dir = Giro[dir][k_melhor]      ← giro duplo (até 90°)
    devolve Giro[dir][k_melhor]
```

Observações (fato visto no código): não compara a vizinha com a célula atual (a formiga sempre vai para a maior vizinha, mesmo no pico); se todas as vizinhas valem 0 mas a atual não, `k_melhor = 0` (norte). [feromonios.md](feromonios.md) descreve o "giro limitado em metade das vezes"; o comportamento exato é: sempre limitado, com giro duplo quando `S2() ≠ 0`.

#### 2.5.6 `_GetAlarmDir(x, y, dir)` (`1030:0e54`)

Borda; senão, maior vizinha do alarme (empate: menor k). Se todas são 0: `Giro[dir][S8()]`; senão `Giro[dir][k_melhor]` (um passo de giro).

#### 2.5.7 `_GetDefendDir(x, y, dir)` (`1030:1026`) — segue a amarela

Borda; depois, conforme o plano da amarela (`_MePlane` `10:ce80`):

| `_MePlane` | Regra |
|---|---|
| 2 | devolve `_GetNestDir(x, y, dir, 0)` (sobe o cheiro de ninho **preto**) |
| 3 | devolve `_GetNestDir(x, y, dir, 0x80)` (ninho vermelho) |
| 0 (vista do quintal) | devolve `dir` (segue reto) |
| 1 | ver abaixo |

Na superfície: se `_ChaseSpid` (`9:72ec`) = 1, `k = _GetDir(x, y, _SpidX >> 4, _SpidY >> 4)`. Senão, se `_GetDis(x, y, _FuzLocX, _FuzLocY) ≤ _ModePopB[5] >> 1` (metade do número de formigas pretas em Defense, `9:9e7a`), `k = S1(8) + 1` (passeio); senão `k = _GetDir(x, y, _FuzLocX, _FuzLocY)`. Resultado: `dir` se k = 0, senão `Giro[dir][k − 1]`. `_FuzLocX/Y` vêm de `_GetStrategy` (9.1).

#### 2.5.8 `_GetRedDefendDir(x, y, dir)` (`1030:1194`) — segue a líder vermelha

Igual a 2.5.7, trocando `_MePlane` por `_RedPlane` (`9:7606`), o alvo por (`_RedLocX` `9:80a6`, `_RedLocY` `9:80ac`) e o raio por `_ModePopR[5] >> 1` (`9:a08e`). Não há caso de aranha. A comparação é `raio < distância` → `_GetDir`; senão `S1(8) + 1`.

## 3. Os modos, um a um

Em todos: `x, y, T` são os valores da vaga no início do modo; "giro" é a seção 2.2; "luta" é a seção 6.1. Cada lista numerada dá a ordem exata dos testes e sorteios.

### 3.1 Modo 0 — Bored: `_DoRandAntA` (`1028:0e66`)

1. Buraco → `_GoInNest`. Fim.
2. `d = _GetRandDir(x, y, dir)`.
3. Passo padrão (2.3) com: pega comida; barreira; livre → anda e deposita cheiro de ninho da própria cor; depois `_DecTSmell(nx, ny, cor)` (trilha da própria cor −1 se > 0); depois, se `S8() == 0` **e** grupo é 2 ou 6 → `M[i] = 2` (Foraging). O `S8()` é sorteado sempre que a formiga anda, mesmo que o grupo não seja 2/6.
4. Amarela: cor diferente → `_YellowFight`; mesma cor → trofalaxia se `_MeWantFood == 1`, depois giro.
5. Mesma cor → giro, depois `M[i] = _GetNewMode(grupo, T original)`.
6. Inimiga → luta.

### 3.2 Modo 0' — `_DoRandAntAA` (`1028:1234`)

Como 3.1, mas: não pega comida (destino com comida passa pelo teste de barreira normalmente), não deposita cheiro, não chama `_DecTSmell`, não sorteia mudança de modo ao andar, amarela da mesma cor e formiga da mesma cor → só giro.

### 3.3 Modos 1 (Nesting) e 4 (Dig In): `_DoToNestAnt` (`1028:1676`)

1. Buraco → `_GoInNest`.
2. `d = _GetNestDir(x, y, dir, T)` (cheiro de ninho da própria cor).
3. Passo padrão com: pega comida (sim, mesmo voltando ao ninho; vira Got Food); barreira; livre → anda e deposita cheiro de ninho; amarela (trofalaxia se `= 1`); mesma cor → só giro; inimiga → luta.

### 3.4 Modo 2 — Foraging: `_DoForageAnt` (`1028:1e42`)

1. Buraco → `_GoInNest`.
2. `S32() == 0` → `M[i] = 13` (Resting). Fim.
3. `Alarme(x, y) ≠ 0` → `M[i] = 11` (Alarmed). Fim.
4. Grupo ≠ 2 e ≠ 6 → `M[i] = _GetNewMode(grupo, T)`, `S[i] = 0`. Fim.
5. `d = _GetForageDir(x, y, dir, T)`.
6. Se `d < 0` (fim da trilha): `S8() == 0` → `M[i] = _GetNewMode(grupo, T)`; senão `M[i] = 0`. `S[i] = 0`. Depois `_DecTSmell(x >> 1, y >> 1, cor)` (ver peculiaridade P1). Fim.
7. Comida no destino → pega (M = 3, S = 200).
8. Barreira → giro; depois `S16() == 0` → `M[i] = _GetNewMode(grupo, T)`, `S[i] = 0`.
9. Ocupado: amarela de outra cor → `_YellowFight`; amarela da mesma cor → trofalaxia se `= 1`, giro; mesma cor → giro; inimiga → luta.
10. Livre → anda, deposita cheiro de ninho (S), `_DecTSmell(nx, ny, cor)`.

**Peculiaridade P1 (fato visto na desmontagem, `1028:1fa6`–`1fb8`):** no caso "fim da trilha", o jogo passa `(x >> 1, y >> 1)` a `_DecTSmell`, que por sua vez divide de novo por 2. A célula de trilha decrementada é `(x >> 2, y >> 2)` no mapa 64 × 32, não a da formiga. Reproduzir assim.

### 3.5 Modo 3 — Got Food: `_DoReturnFoodAnt` (`1028:1cb4`)

1. Buraco → `_GoInNest` (a comida entra no ninho com a formiga).
2. `d = _GetNestDir(x, y, dir, T)`.
3. Barreira → giro. Fim.
4. **Senão anda sempre**, sem olhar `Vida(nx, ny)`: grava `Vida(nx, ny) = d | base(T)` por cima de quem estiver lá, zera `Vida(x, y)`, atualiza X/Y; se `S > 0`: `S −= 1` e deposita `S` na **trilha** da própria cor (`_JamScentBT/RT`).

**Peculiaridade P2 (fato visto no código):** formigas carregando comida atravessam outras formigas (inclusive inimigas e a amarela), sobrescrevendo o mapa de vida; não há luta. A formiga atropelada continua na lista e volta ao `_LifeA` quando se mover.

### 3.6 Modo 5 — Dig Out: `_DoDigOutAntA` (`1028:1480`)

Ordem exata (fato visto no código):

1. `r = S8()`; `d = Giro[dir][r]`.
2. `b = _Bounce(x, y)` (sorteia só na borda); se `b ≠ 0`, `d = b − 1`.
3. Grupo ≠ 5 e ≠ 9 (não carrega terra) → `M[i] = _GetNewMode(grupo, T)`, `S[i] = 0`. Fim.
4. `S8() == 0` → **larga a terra**: `T[i] −= 0x18` (5→2, 9→6; a terra some, nenhum tile é criado), `M[i] = _GetNewMode(grupo antigo, T antigo)` (para os grupos 5/9 a tabela devolve 5 de novo; no tick seguinte o passo 3 corrige), `S[i] = 0`, `Vida(x, y) = T[i]`. Fim.
5. Barreira no destino → giro (`Giro[dir][S8()]`).
6. Destino ocupado (qualquer um) → giro. Não há luta nem trofalaxia neste modo.
7. Livre → anda; se `S > 0`, `S −= 1` e deposita cheiro de ninho da própria cor.

Não há teste de buraco neste modo.

### 3.7 Modo 6 — Defense (recrutada): `_DoRecruitAnt` (`1028:22a8`)

1. Buraco → `_GoInNest`.
2. Direção: se `Alarme(x, y) ≠ 0` → `_GetAlarmDir`; senão, preta (`T < 0x80`) → `_GetDefendDir`; vermelha → `_GetRedDefendDir`.
3. Barreira → **`T[i] = base(T) | S8()`** (direção sorteada pura, sem `_TurnTab`), `Vida(x, y) = T[i]`. Fim.
4. Livre → anda (sem cheiro).
5. Ocupado por não-amarela: mesma cor → giro (com tabela); inimiga → luta.
6. Amarela: cor diferente → `_YellowFight`; mesma cor → trofalaxia se `_MeWantFood ≠ 0`; depois giro com tabela.

`S[i]` não é usado aqui (o recrutamento grava 0; `_ForceModeA` gravaria a posição codificada da amarela, ver 9.4).

### 3.8 Modo 7 — Attack: `_DoAttackAnt` (`1028:2a40`)

1. Buraco → `_GoInNest` (qualquer buraco em que pise, inclusive o do próprio ninho).
2. Se `_IsCarryCaste[grupo]` (`8:0074`, palavras) = 1, a carga é descartada: grupo ← `_UnCarryCaste[grupo]` (`8:0094`). O novo T só é gravado no fim (passos 4–6).
3. `d = _GetNestDir(x, y, dir, T ^ 0x80)` (cheiro de ninho da cor **inimiga**; sem cheiro, aponta em 3/4 das vezes para o último buraco inimigo).
4. Barreira → giro.
5. Livre → anda (sem cheiro).
6. Ocupado: amarela de outra cor → `_YellowFight`; inimiga → luta; mesma cor (ou amarela da mesma cor) → giro.

Tabelas lidas do `SIMANTW.EXE` (fato verificado, segmento 8 no arquivo em 0x9600):

| grupo | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10–15 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `_IsCarryCaste` | 0 | 1 | 0 | 1 | 0 | 1 | 0 | 1 | 0 | 1 | 0 |
| `_UnCarryCaste` | 0 | 2 | 2 | 2 | 4 | 2 | 6 | 6 | 8 | 6 | igual ao grupo |

### 3.9 Modo 8 — ovo na superfície (embutido em `_DoAntSimA`; igual a `_SimEggA` `1028:0a1c`)

`Vida(x, y) = T`; `S1(200) == 0` → `T[i] = 0`, `Vida(x, y) = 0` (o ovo some). Ovos não eclodem na superfície.

### 3.10 Modo 9 — rainha na superfície (embutido; igual a `_SimQueenA` `1028:0a74`)

`Vida(x, y) = T`. Só a **cauda** (`(T & 0x7F) > 0x67`, grupo 13) é verificada: a cabeça deve estar em `(x, y) + D8[dir(T)]` com `Vida = T − 8`, ou, alternativamente, `_FindInAList` deve achar qualquer formiga nessa célula. Se nenhuma das duas, a cauda é removida (`T[i] = 0`, `Vida = 0`). A cabeça nunca é removida aqui (correção a [formigas.md](formigas.md#3-codificação-do-tipo-t), que diz "a que restou também é removida"). Sem sorteio.

### 3.11 Modo 11 — Alarmed: `_DoToAlarm` (`1028:1a0a`)

1. Buraco → `_GoInNest`.
2. Se `Alarme(x, y) == 0` **e** `S4() == 0` (o `S4` só é sorteado com alarme zero) → `Vida(x, y) = T`, `M[i] = _GetNewMode(grupo, T)`. Fim.
3. `d = _GetAlarmDir(x, y, dir)`.
4. Barreira → giro. Livre → anda (sem cheiro). Amarela de outra cor → `_YellowFight`. Inimiga → luta. Mesma cor ou amarela da mesma cor → giro.

### 3.12 Modo 12 — Defend Nest (embutido; igual a `_DoDefendNest` `1028:0df6`)

Se `Ninho_cor(x, y) < 0x6E` (110) → `_DoToNestAnt`; senão `_DoRandAntAA`. Sem sorteio próprio.

### 3.13 Modo 13 — Resting (embutido; igual a `_DoRestAnt` `1028:0b76`)

1. Buraco → `_GoInNest`. Fim.
2. `S4() == 0` → `M[i] = 2` (Foraging).
3. Senão, se a opção "Silly" (`_OptionStates[5]`, `8:85fc`) = 1 → `_RestBalloons(x, y, 1)` (balão; não sorteia).

A formiga não anda e não grava `Vida`.

### 3.14 Modo 14 — Pre-Flight (embutido; igual a `_DoRepoLoit` `1028:0c1e`)

1. `S2() == 0` → `_DoToNestAnt(i)`; senão `_DoRandAntAA(i)`.
2. Lê a cor do **T atual** (que pode ter virado 0 se a formiga entrou no ninho); se `_FlyAwayB` (`9:7c44`, cor 0) ou `_FlyAwayR` (`9:8078`, cor vermelha) > 100 → `M[_Tindex] = 15`.

### 3.15 Modo 15 — Take off: `_DoRepoExit` (`1028:0c7a`)

1. `Ninho_cor(x, y) < 100` → `_DoToNestAnt`; senão `_DoRandAntAA`.
2. Contador `F = _FlyAwayB` ou `_FlyAwayR` conforme a cor de `T` (relida): `F == 0` → nada; `F == 1` → `M = 16`; senão `S1(F) == 0` → `M = 16` (o sorteio só acontece com F ≥ 2).

### 3.16 Modo 16 — Flying (embutido; igual a `_DoRepoFly` `1028:0d4a`)

`S32() == 0` **e** (preta e `_SwarmCntB` (`9:807a`) < 50, ou vermelha e `_SwarmCntR` (`9:9c26`) < 50) → a formiga sai do mapa (`T[i] = 0`, `Vida = 0`). Só no Jogo Completo (`_CurGameType` `9:80b4` = 2): `_SwarmCnt*` += 1 e `S16() == 0` → `_QueenStorageB` (preta; e redesenha o mostrador) ou `_QueenStorageR` += 1. O `S16` só é sorteado no Jogo Completo. A formiga em voo não se move no mapa.

### 3.17 Modos sem efeito: 17 e 18

Não há caso no `switch`: a formiga é contada em `_TemB/RModePop` e não faz nada (fato visto no código). Como esses modos só aparecem no ninho ([formigas.md](formigas.md#5-modos)), uma formiga que saia do ninho com M = 17 ficaria parada na superfície até morrer ou ser recrutada [hipótese: caso raro].

## 4. Revoada (resumo de quem decide)

- `_FlyAwayR` é armado por `_GstrR` (9.1): se `_FlyAwayR == 0` e machos + rainhas novas vermelhos (`_CastePopR[3] + [4]`, `10:aca6`/`10:aca8`) > 20 → `_FlyAwayR = 200`. `_FlyAwayB` não é armado em nenhuma rotina desta parte [em aberto: provavelmente pelo jogador — ciclo de vida, documento paralelo].
- Os dois contadores caem 1 por tick em `_DoAntSim` ([simulacao-visao-geral.md](simulacao-visao-geral.md), seção 5, passo 13).
- Machos e rainhas novas nascem com M = 14 (`_CasteModeTabB`), passam a 15 com o contador > 100, a 16 quando ele chega a 1 (ou por sorteio 1/F) e saem do mapa pelo modo 16.

## 5. Contagem de modos e `_MakeRedInitiator`

Ver 9.2. O incremento de `_TemB/RModePop[M]` acontece em 1 (antes do despacho), para toda formiga viva da lista A.

## 6. Luta e morte

### 6.1 Início de luta (embutido nos modos; igual a `_StartFightA` `1028:266a`)

**Fato visto no código.** Quando a formiga `i` (atacante, "b") tenta entrar na célula `(nx, ny)` de uma inimiga:

```
Tb = T[i] ; T[i] = 0 ; Vida(x, y) = 0          ← o atacante SOME da superfície
j = _FindInAList(nx, ny)                        (última vaga com X, Y iguais e T ≠ 0)
se j < 0: fim                                    ← o atacante sumiu sem luta
v = _GetWinner(T[j], Tb)                         (a da célula é "a")
T[j] = 0x70 + (v & 0x80) ; Vida(nx, ny) = T[j]  ← nuvem de luta com a cor do vencedor
M[j] = 10 ; S[j] = v                             ← S guarda o T do vencedor
_AlarmHere2(nx, ny, 40)                          ← alarme = max(alarme, 40)
```

### 6.2 `_GetWinner(Ta, Tb)` (`1028:26f4`)

**Fato visto no código; tabelas verificadas no `SIMANTW.EXE`** (DGROUP no arquivo em 0x20000):

```
se _Invincible (8:8a5c) == 1:
    _RAntsKilled (9:9e96, 32 bits) += 1
    devolve (Ta se cor(Ta) = 0, senão Tb)       ← a preta sempre vence; sem sorteio
p = Matriz[Classe[grupo(Ta)]][Classe[grupo(Tb)]]
se R(10) < p: vencedor = Ta  senão: vencedor = Tb         ← gerador R
se cor(vencedor) = 0: _RedLost (9:a0e4) += 1 ; _RAntsKilled += 1
senão:                _BlackLost (9:79e4) += 1 ; _BAntsKilled (9:99e0, 32 bits) += 1
devolve vencedor
```

`Classe` (`10:22c6`, 16 bytes) = `0 0 0 0 2 0 1 1 2 1 0 0 3 3 0 0`; `Matriz` (`10:22d6`, 4 × 4, chance em 10 de "a" vencer) = linhas `5 2 7 3`, `8 5 9 4`, `3 1 5 2`, `7 6 8 5` (classes 0 operária/ovo, 1 soldado, 2 reprodutor, 3 rainha). Mesmos valores de [formigas.md](formigas.md#6-luta-morte-e-saúde).

O T devolvido é o T **completo** do vencedor (com carga e direção); é ele que volta à vaga quando a nuvem se desfaz.

### 6.3 Nuvem de luta: modo 10, `_DoFightA` (`1028:27e6`)

```
q = S1(7)                                       ← 0..6, quadro da animação
T[i] = base(T[i]) + q ; Vida(x, y) = T[i]
se S16() == 0:                                  ← a luta termina
    T[i] = S[i] (vencedor) ; Vida(x, y) = S[i]
    M[_Tindex] = _GetNewMode(grupo(S[i]), S[i]) ; S[i] = 0
    _DeadAntHere(x, y, cor do vencedor)          ← cadáver na própria célula
senão, se "Silly" (8:85fc) == 1: _FightBalloons(x, y, 1)
```

**Peculiaridade P3 (fato visto no código):** o cadáver é marcado com a **cor do vencedor**, e `_DeadAntHere` zera `Vida(x, y)` no fim. O vencedor fica na lista, na mesma célula, sem marca no mapa de vida até se mover de novo.

### 6.4 Cadáver: `_DeadAntHere(x, y, cor)` (`1028:28c0`)

Anel de 100 posições (`_DeadX` `9:9c82`, `_DeadY` `9:9d76`, ponteiro `_DdPtr` `9:9ea8`). Fato visto no código:

```
p = _DdPtr + 1 ; se p > 99: p = 0 ; _DdPtr = p
(ox, oy) = (_DeadX[p], _DeadY[p])               ← o cadáver mais antigo
quintal:
    se 0x10 ≤ Mapa(ox, oy) < 0x18: Mapa(ox, oy) = S16()     ← apaga o cadáver velho (vira chão)
    _DeadX[p], _DeadY[p] = x, y
    se Mapa(x, y) ≤ 0x17: Mapa(x, y) = (cor = 0 ? 0x10 : 0x14) + S4()
casa:
    se 8 ≤ Mapa(ox, oy) < 0x18: Mapa(ox, oy) = (valor − 8) >> 2   ← volta ao piso
    _DeadX[p], _DeadY[p] = x, y
    v = Mapa(x, y) ; se v ≤ 3:
        preta: Mapa = (v + 2)·4 + S1(2)           (0x08–0x15 conforme o piso)
        vermelha: Mapa = v·4 + 10 + S1(2)
Vida(x, y) = 0
```

O sorteio `S16()` do cadáver velho só acontece se lá ainda houver um cadáver; os de cor (`S4`/`S1(2)`) só se o tile novo for chão. `cor` é qualquer valor ≠ 0 para vermelha (a aranha passa 0 ou 4, ver [inimigos.md](inimigos.md)). Valores de tile: quintal 0x10–0x13 cadáver preto, 0x14–0x17 vermelho (completa [tiles-valores.md](tiles-valores.md), que deixava 0x10–0x17 em aberto).

### 6.5 Morte natural na superfície

Seção 1. Saúde da colônia ≥ 32 impede a morte (S32 ≤ 31). A formiga morta vira cadáver e conta em `_BAntsExpired`, inclusive quando é vermelha (fato visto no código; provável descuido do original).

## 7. Entrada no ninho: `_GoInNest(x, y, i)` (`1028:257a`)

**Fato visto no código.**

```
se x < 64:  lista = B ; ninho = preto ; buracos = _HoleMapB (8:82d2)
senão:      lista = R ; ninho = vermelho ; buracos = _HoleMapR (8:8312)
se a lista tem ≥ 500: compacta (_CompactListB/R)
se ainda tem ≥ 500: fim                          ← a formiga fica na superfície, nada muda
adiciona na lista: X = y, Y = 1, T = base(T) + 4 (virada para baixo), M = M[i], S = S[i]
se buracos[y] ≠ 0: _DigTileB/R(y, 1)            (garante o túnel sob o buraco)
T[i] = 0 ; Vida(x, y) = 0
```

O ninho é escolhido pela **coluna** do buraco, não pela cor da formiga: uma preta que pise num buraco vermelho vira invasora no ninho vermelho, e vice-versa. A amarela usa outro limite (x < 65, [formiga-amarela.md](formiga-amarela.md)).

## 8. Comida na superfície

### 8.1 `_PickupFoodA(x, y)` (`1020:0d18`)

Quintal: tile 0x48 (última unidade) → `Mapa = S16()` (chão); senão `Mapa −= 1`. Casa: se `valor % 4 == 0` (última unidade) → `Mapa = (valor − 0x18) >> 2` (piso); senão `Mapa −= 1`. Em seguida `_FoodA` (`9:9e84`) −1 se > 0. Fato visto no código.

### 8.2 `_DropFoodA(x, y)` (`1020:0d86`)

Usada pela amarela e pela aranha morta. Quintal: tile > 0x4A → falha (devolve 0); tile < 0x48 → 0x48; senão +1. Casa: tile ≤ 3 → `(tile + 6)·4`; 8–0x17 (cadáver) → `((tile − 8) >> 2 + 6)·4`; 0x18–0x26 → +1; outro tile < 0x40 (4–7 e 0x27–0x3F) → a comida **cai**; ≥ 0x40 → falha. Sucesso: `_FoodA += 1`. Fato visto no código.

**A queda** [código + desmontagem de `1020:0dfe`–`0ea4`]: com d = `_DROPdir` (`9:9c66`), dx = byte `10:22be + d` e dy = byte `10:22c2 + d` (tabelas `[0, 1, 0, 0xFF]` e `[0xFF, 0, 1, 0]`), **somados sem sinal** (`mov al, …; sub ah, ah`). A partir da própria célula: se o tile é < 4, recebe `(tile + 6)·4`, `_FoodA += 1` e para; senão anda (dx, dy); para se x sai de 0..127 ou y de 0..63. Devolve 1 mesmo quando a comida some. Como 0xFF soma 255, nas direções 0 (ladrilhos, "para cima") e 3 (carpete direito, "para a esquerda") a comida sai do mapa no primeiro passo e **some**; só cai de fato nas direções 1 (carpete esquerdo, para a direita) e 2 (cozinha, patches 9/36/37, para baixo) [hipótese: defeito do original (o código quer −1); comportamento seguido pelo port]. `_DROPdir` não está no save: depois de um Load vale o do último patch da casa gerado na sessão (0 se nenhum). **Port** (2026-10-18): `comum::largar_comida_superficie` + `comida_cai`, com `Estado::passo_queda` gravado ao gerar um patch da casa (tabela em `data/casa.json`, `direcoes_queda`); teste `comida_cai_na_casa_pelo_dropdir`. **Conferido na bancada (2026-10-19, vetores `queda_casa_carpete_esq` e `queda_casa_carpete_dir`, testes-tick.md "Sétima rodada"):** a aranha em agonia sobre um fiapo (0x3F/0x3E) larga 3 comidas; no carpete esquerdo (`_DROPdir` = 1, lido na memória) cada uma cai para a direita até o primeiro carpete livre (106, 107 e 108, 44 = 0x24, `_FoodA` + 3); no direito (`_DROPdir` = 3) as três somem e `_FoodA` não muda — o passo 0xFF sem sinal se confirma. **Cozinha (2) e ladrilhos (0), conferidos em 2026-10-19 (oitava rodada, vetores `queda_casa_cozinha`, `queda_casa_ladrilhos` e `queda_casa_azulejo`):** nenhuma receita desses patches gera tile que cai (a cozinha só tem 0 na bancada e ≥ 0x62 na parede; os ladrilhos, 0/1 e juntas ≥ 0x5C), então a aranha morta sobre o azulejo 0x62 **não larga nada** (`_DropFoodA` devolve 0 para tile ≥ 0x40: as 3 comidas se perdem, `_FoodA` igual). Com um 0x3F escrito na célula (preparação na memória), na cozinha (`_DROPdir` = 2, lido) as 3 comidas descem até a bancada (linhas 24, 25 e 26, `_FoodA` + 3); nos ladrilhos (`_DROPdir` = 0) as 3 somem no primeiro passo (dy = 0xFF sem sinal), como no carpete direito. Na prática, então, 0 e 2 só agem sobre um tile que cai trazido por outra regra (nenhuma conhecida) [hipótese].

### 8.3 `_AddFood(n, som)` (`1030:6a58`) e alimentação periódica

O algoritmo de espalhamento (centro, raio, seno/cosseno de `_FracSineTab`) está em [geracao-mundo.md](geracao-mundo.md) (seção 3.E), e vale igual durante o jogo. Durante o jogo ele é chamado pela alimentação periódica (corpo de `_FeedAnts` `1028:0474`, embutido em `_DoAntSim` a cada 64 ticks): fora do modo Experimental, se `_FoodA < _NextFeed` (`8:8a62`), `_AddFood(150, 1)` (toca som, centro `(S128(), S64())`) e `_NextFeed = S1(50) + 1`. A ordem dos sorteios dentro de `_AddFood` é: centro (`S128`, `S64`, ou `S1(48)` no caso inicial), raio `S8()`, e por unidade `S256()` e `S1(raio)`. Grava `_LastFoodDrop` (`8:836a`), usado pela líder vermelha (9.2). Fato visto no código. **Conferido na bancada (2026-10-19, vetores `alimentacao` e `alimentacao_casa`, `tools/cenarios_fecha.py`):** com `_NextFeed` = 30000 e `_Cycle` levado ao fim de um período de 64 na entrada, o tick chama `_AddFood(150, 1)`: no quintal (`rico`), centro (85, 31), `_FoodA` 122 → 237, `_NextFeed` = 50; na casa (Jogo Completo, patch (0, 4), carpete esquerdo), a comida pela regra do chão da casa (geracao-mundo.md 3.E). Os dois 307/307 com S, R e vida.

## 9. Estratégia

### 9.1 `_GetStrategy` (`1030:0000`), `_GstrB` (`1030:01cc`), `_GstrR` (`1030:03c2`)

Ordem exata (fato visto no código), todo tick:

```
_ChaseSpid = 0
se _MePlane == 1:
    _FuzLocX = clamp(_MeLocX + S1(5) − 2, 0, 127)
    _FuzLocY = clamp(_MeLocY + S1(5) − 2, 0, 63)
    se _SpidOn ≠ 0 e _GetDis(_SpidX >> 4, _SpidY >> 4, _MeLocX, _MeLocY) < 100: _ChaseSpid = 1
_FuzLocX = _MeLocX                               ← sobrescreve sempre (o sorteio de X é descartado)
_StrategicModeB = escada preta (simulacao-visao-geral.md §7; mesma conta de _GstrB)
_StrategicModeR = _GstrR()
_SetCasteProd() ; _SetModeProd()
```

`_GstrB` existe como função, mas `_GetStrategy` tem a mesma lógica embutida; não há outro chamador. Verificado no `QUICK2.ANT`: `_FuzLocX = _MeLocX = 34` com a amarela no ninho (`_MePlane = 2`) e `_FuzLocY = 0` (não atualizado fora da superfície).

`_GstrR` (S = `_HealthR`, P = `_RpopT`, D = `_TilesDugR`, Pb = `_BpopT`), nesta ordem:

```
se _FlyAwayR == 0 e _CastePopR[3] + _CastePopR[4] > 20: _FlyAwayR = 200
se _JustToBeMean (9:78dc) ≠ 0: _JustToBeMean −= 1 ; devolve 0
se S < 10 e Pb < P/2 e Pb > 0 e _BlkQueens > 0: MALVADO
se S < 30: devolve 5
se S < 50: devolve 4
se D < P: devolve 3
se D < 2P: devolve 2
normal = (P < 101 ou Pb < 1 ou _BlkQueens < 1 ou P/3 ≤ Pb)
se não normal: MALVADO                           ← sem sorteio
se S32() ≠ 0: devolve 1
se P < 21: devolve 1
se P ≤ Pb: devolve 1
se S128() ≠ 0: devolve 1
MALVADO:
    _JustToBeMean = S1(100) + 30
    música 0x2B0A ; mensagem _AdviceStrs[5] (SHARED 4/1020, índice 5 a partir de 0: "The red ants are attacking!"),
    com parâmetro 120 ([hipótese] duração em ticks)
    devolve 0
```

`_StartAttack` (`1030:050e`) faz só o bloco MALVADO (sem chamador encontrado).

### 9.2 Formiga vermelha líder ("Avenger", modo 19)

**`_MakeRedInitiator` (`1028:967c`)**, chamado no fim do tick se `_TemRModePop[19] < 1`:

```
_RedInitOn (8:8a64) = 0
se _BpopT ≥ 30:
    i = maior índice da lista A com T ≥ 0x80 (qualquer formiga vermelha, inclusive ovo, rainha ou nuvem)
    se existir: T[i] = 0xB0 ; M[i] = 19 ; S[i] = 0 ; _RedTask (9:9d74) = 0 ; _RedInitOn = 1
```

O `_LifeA` não é atualizado aqui; a nova cor/tipo aparece quando a líder se mover (peculiaridade P4).

**`_DoRedInitiator(i)` (`1028:96d4`)**:

```
_RedLocX, _RedLocY = X[i], Y[i] ; _RedPlane = 1
se _RedTask == 0:
    _UnRecruitRed()
    se _MePlane == 1 e S1(32) + 64 < _MeLocX e S1(10) < _ModePopB[5]:     ← curto-circuito
        _RedTask = 2 ; _RecruitRed(_ModePopB[5])                          (caçar a amarela)
    senão:
        (dx, dy) = _LastFoodDrop
        se dx < 31: se dy < 20: dy += 5 ; senão se dy > 40: dy −= 5
        senão: dx −= 5
        _RedDestX, _RedDestY (9:9bee / 9:9c22) = dx, dy
        n = _CastePopR[1] + _CastePopR[2] ; n = (n < 20 ? n >> 2 : n >> 3)
        _RecruitRed(n) ; _RedTask = 1
senão se _RedTask == 2 e _MePlane == 1:
    _RedDestX, _RedDestY = _FuzLocX, _FuzLocY
d = _GetRedBestDirs(1, X[i], Y[i], _RedDestX, _RedDestY)
se d < 0: se S1(10) == 0: _RedTask = 0 ; Vida(x, y) = T[i]
senão:
    destino vazio → anda (sem cheiro)
    amarela → _YellowFight(1, i)                  ← sem teste de cor
    formiga preta (Vida < 0x80) → nada (nem giro nem regravação de Vida)
    vermelha → T[i] = base | _RandTurn(dir) ; Vida(x, y) = T[i]
```

`_RandTurn(d)` (`1028:2a22`) = `Giro[d][S8()]`. `_GetNewRedTask` (`1028:9940`) repete o bloco `_RedTask == 0` e não tem chamador.

**`_GetRedBestDirs(plano, x, y, tx, ty)` (`1028:9a18`)**: fato visto no código.

```
dist = _GetDis(x, y, tx, ty) ; se dist ≤ 0: devolve −1
livre = −1 ; bloqueada = −2
para k = 0..7 (vizinha v = (x, y) + D8[k]):
    se _TileCanBeMovedOn(plano, v, 1, tx, ty, 0) e _GetDis(v, alvo) < dist:
        dist = _GetDis(v, alvo)                      ← o limiar cai a cada melhora
        se _GetLife(plano, v) < 1 e _IsClearTile(plano, v): livre = k
        senão: bloqueada = k
devolve (livre ≥ 0 ? livre : bloqueada)
```

`_GetLife` devolve −1 (0xFFFF) para célula vazia ou fora do mapa e o byte do mapa de vida nos demais casos (a amarela, 0xFF, conta como ocupada). Sem sorteio. A mesma função, com outro teste de passagem, é `_GetMyBestDirs` da amarela.

### 9.3 Modo novo: `_GetNewMode(g, T)` (`1030:0910`), `_GetNewModeB` (`1030:09d0`), `_GetNewModeR` (`1030:0a50`)

Como em [formigas.md](formigas.md#escolha-de-modo), com a ordem de sorteio explícita: o `S8()` só é sorteado para grupo 2 ou 6 **e** (vermelha, ou preta com `_ModeAuto` = 1). Preta com `_ModeAuto` = 0 e grupo 2/6 → `_ModeMe` (`8:8a58`) sem sorteio. Outros grupos → `_CasteModeTabB[g]` sem sorteio. Os índices de tabela usados são `_StrategicModeB` (pretas) e `_StrategicModeR` (vermelhas).

### 9.4 Recrutamento: `_Recruit`, `_UnRecruit`, `_RecruitRed`, `_UnRecruitRed`, `_ForceModeA/B`

Fato visto no código; nenhum sorteia.

| Função | Endereço | Regra |
|---|---|---|
| `_Recruit(n)` | `1030:06d2` | lista A de trás para frente, depois lista B: formigas **pretas** de grupo 2 ou 6 com M ≠ 6 → M = 6, S = 0, até n |
| `_UnRecruit(p)` | `1030:078a` | quota = `_TemBModePop[6] / 2` se p = 0, ou `_TemBModePop[6] + 100` se p ≠ 0 (contagem de Defense do último tick). Lista A: pretas com M = 6 → M = 0. Lista B: idem. Lista R (invasoras pretas): M = 6 → **M = 7** (Attack). Para ao esgotar a quota |
| `_RecruitRed(n)` | `1030:0866` | lista A: vermelhas de grupo 2/6 com M ≠ 19 e ≠ 6 → M = 6, S = 0, até n |
| `_UnRecruitRed()` | `1030:08da` | lista A: toda vermelha com M = 6 → M = 0 |
| `_ForceModeA(i, g, m)` / `_ForceModeB` | `1030:0550` / `0622` | sem chamador encontrado. Larga a carga conforme o grupo (1: T += 8; 3/7: T −= 8 e, se o tile < 0x48, grava 0x48; 5/9: T −= 0x18), grava M = m e S = 0 para grupos 1–3, 5–7, 9; e, se m = 6 e a amarela está na superfície, S = `(_MeLocY & 0xFC) << 2 | (_MeLocX >> 3)` |

`_UnRecruit` lê `_TemBModePop`, que no momento do clique do jogador ainda guarda a contagem do tick anterior (só é zerado no passo 12 do tick seguinte).

## 10. Funções sem chamador encontrado

Fato visto no código (busca por chamadas diretas no pseudocódigo): `_SimEggA`, `_SimQueenA`, `_LostHeadA` (`1028:0b1e`: devolve 1 se a cauda perdeu a cabeça, mesma regra de 3.10), `_DoRestAnt`, `_DoRepoLoit`, `_DoRepoFly`, `_DoDefendNest`, `_StartFightA`, `_FeedAnts`, `_GstrB`, `_StartAttack`, `_ForceModeA/B`, `_GetNewRedTask`. As regras equivalentes estão embutidas nos chamadores descritos acima. [hipótese] Restos do port do Mac ou chamadas por ponteiro não resolvidas.

## 11. Peculiaridades a reproduzir

| Id | O quê | Onde |
|---|---|---|
| P1 | `_DecTSmell` com coordenadas já divididas por 2 no fim da trilha | 3.4 |
| P2 | carregadoras de comida atravessam outras formigas sem luta | 3.5 |
| P3 | cadáver da luta com a cor do **vencedor**; `Vida` zerada sob o vencedor | 6.3 |
| P4 | `_MakeRedInitiator` não atualiza `_LifeA` | 9.2 |
| P5 | o atacante some se `_FindInAList` não achar a defensora | 6.1 |
| P6 | morte natural de vermelhas conta em `_BAntsExpired` | 6.5 |
| P7 | modos 17 e 18 não fazem nada na superfície | 3.17 |
| P8 | `S256()` sorteado para vagas vazias; `S8()` de `_DoRandAntA` sorteado mesmo para quem não pode forragear | 1, 3.1 |
| P9 | `_GetForageDir` consome um `S8()` inútil | 2.5.4 |
| P10 | `_GetWinner` sorteia com o gerador R, não com o S | 6.2 |

## 12. Casos de teste sugeridos (bancada QEMU)

Bancada: [docs/bancada-qemu.md](../../bancada-qemu.md). Ferramenta de leitura do save: `tools/save_table.py` e os offsets de [save-ant.md](../formatos/save-ant.md). "Salvar" = `Ctrl+A` com nome novo. Todos os testes comparam por tick (`_GameTime`, offset 45740).

| # | Regra | Entrada | Esperado | Como observar |
|---|---|---|---|---|
| T1 | Laço e morte natural (1, 6.5) | Quick Game; deixar `_HealthB` cair abaixo de 32 (não alimentar; ~4.500 ticks) | aparecem cadáveres pretos 0x10–0x13 no `_MapA` e `_BAntsExpired` sobe; com saúde ≥ 32, `_BAntsExpired` não muda pela superfície | saves sucessivos; contar tiles 0x10–0x17 e ler `_BAntsExpired` |
| T2 | Pegar comida (2.3 passo 1) | save logo após uma operária encostar na comida | a formiga continua na mesma célula, T grupo 3, M = 3, S = 200; o tile de comida perdeu 1 | listas A do save + `_MapA` |
| T3 | Trilha de volta (3.5) | seguir uma formiga com M = 3 em 2 saves com poucos ticks de diferença | `_PherMapBT` na rota com valores 199, 198… decrescentes a partir da comida | mapa BT do save |
| T4 | Atropelamento (P2) | Experimental: comida e uma fila de formigas no caminho de volta | carregadora passa sobre as outras sem nuvem de luta | captura de tela + listas (duas formigas na mesma célula) |
| T5 | Luta (6.1–6.3) | Experimental: operária preta e vermelha frente a frente | no save seguinte, uma vaga com T = 0x70/0xF0, M = 10, S = T do vencedor; a outra com T = 0; alarme ≥ 40 na célula; depois de ~16 ticks, cadáver da cor do vencedor | listas A + `_PherMapA` + `_MapA` |
| T6 | Proporção de vitórias (6.2) | repetir T5 ≥ 50 vezes, e soldado × operária | ~50% e ~80% para a da célula | contagem `_RedLost`/`_BlackLost` |
| T7 | Entrada no ninho (7) | formiga pisa num buraco | some da lista A; aparece na lista B em (y_buraco, 1) com T dir = 4 e M/S preservados | listas A e B |
| T8 | Defend Nest / Take off (3.12, 3.15) | Jogo Completo com revoada armada | formigas M = 15 ficam perto do buraco (cheiro ≥ 100) e passam a 16; contagem `_SwarmCntB` sobe | campos `_FlyAway*`, `_SwarmCnt*` |
| T9 | Estratégia vermelha (9.1) | Quick Game longo até vermelhos dominarem | `_JustToBeMean` > 0 e `_StrategicModeR = 0` durante 30–129 ticks; mensagem "The red ants are attacking!" | campos 48118 e 48298 |
| T10 | Líder vermelha (9.2) | Quick Game até `_BpopT ≥ 30` | existe exatamente uma vaga vermelha com M = 19 e T = 0xB0 (após mover); `_RedInitOn = 1`; `_RedDestX/Y` perto de `_LastFoodDrop` (±5) ou de `_FuzLoc` | listas A + campos 48254–48268 |
| T11 | Recrutamento (9.4) | tecla 1 (recruta 5) com a amarela na superfície | 5 pretas de grupo 2/6 com M = 6, S = 0 (preferência pelas de maior índice da lista A) | listas A/B antes e depois |
| T12 | Port × original | carregar o mesmo `.ANT` nos dois e rodar N ticks sem interação, janelas iguais | listas A/B/R e mapas iguais vaga a vaga | comparação binária dos `.ANT` (exige reproduzir também os sorteios de desenho, ver [rng.md](rng.md)) |

## Lacunas

- Quem arma `_FlyAwayB` (revoada preta) e as regras de `_QueenStorage*`: documento de ciclo de vida (em paralelo).
- Queda de comida na casa (`_DropFoodA`): conferida nas quatro direções (8.2).
- Significado prático dos modos 17/18 na superfície (se alguma formiga sai do ninho com eles).
- `_ForceModeA/B`, `_StartAttack`, `_GetNewRedTask` e demais funções sem chamador: confirmar se há chamadas indiretas (tabela de ponteiros) com o Ghidra.
- Todos os casos T1–T11 desta seção foram conferidos no original (T9 e T11 na oitava rodada, 2026-10-19; seção "Observado no original").
- `_YellowFight` e `_DoTroph` são os completos de `amarela.rs` (com `_YellowDeath(0)` e `_ResetYellowVars`); os balões (`_RestBalloons`, `_FightBalloons`) anotam pelo `rules::baloes` (telas/baloes.md).

## Observado no original

**Vetores de tick** ([testes-tick.md](testes-tick.md), `tests/referencias/ticks/`, ticks 79–83 do Quick Game): para cada tick, o port parte do `estado_antes`, procura quantos passos de S as sub-rotinas anteriores (quintal, água, formiga-leão, aranha, estratégia) gastaram e roda só `_DoAntSimA`. Nos 5 ticks existe um ponto de partida (1, 4, 2, 15 e 2 passos depois da semente de entrada) com o qual a lista A, a vida da superfície nas células das formigas e os 5 mapas de cheiro ficam **iguais** aos do `estado_depois`, e a semente de saída do original é alcançável de onde a superfície parou. As formigas da superfície nesses ticks são um macho preto em Pre-Flight (modo 14) e uma operária vermelha em Foraging (modo 2). `_GetStrategy` (modos estratégicos, `_MakeMe`, `_ModeMe`) também bate. Teste: `vetores_de_tick_do_original` em `engine/tests/regras_superficie.rs`. Fato verificado (2026-10-17).

**Casos da bancada** (2026-10-17, `tools/caso_ant.py`; todas as conferências passaram, e o port, partindo do `carregado.ANT` e rodando até o mesmo `GameTime`, dá o mesmo resultado; ver `tests/referencias/README.md`):

| Caso | Regra | Observado no original |
|---|---|---|
| SUP-01 | 3.5 (carregadora, trilha, P2) e `_GetNestDir` com a maior vizinha na própria direção | parado no tick 94: a formiga em (24, 40), T 0x1E, M 3, S 184 (16 passos para oeste, S − 1 por passo); `_PherMapBT` igual ao port célula a célula |
| SUP-02 | 7 (`_GoInNest`) | vaga da lista A com T = 0; a formiga é a vaga 5 da lista B, grupo 2, S 55 preservado; `_MapB(32, 1)` escavado |
| SUP-03 | 9.2 (`_MakeRedInitiator`, `_DoRedInitiator`, `_GetRedBestDirs`) | com `_BpopT` ≥ 30, a vermelha do fim da lista virou líder (M 19); destino (59, 38) = `_LastFoodDrop` − (5, 0); no tick 90 a líder está em (89, 38), T 0xB6, igual ao port |
| SUP-04 | feromônios no tick e P7 | trilha vermelha 210 → 105 no tick 96; trilha preta 210 intacta até o tick 130; alarme 200 → 100 no centro e 25 nas vizinhas no tick 128; buraco vermelho (59, 16) = 254; formiga com M = 17 parada |

Nenhuma divergência entre esta especificação e o original apareceu nesses casos.

**Vetores de tick da sétima rodada** (2026-10-19, `tools/cenarios_raros.py`, [testes-tick.md](testes-tick.md#sétima-rodada-2026-10-19-ramos-raros); todos 307/307 com S, R e vida):

| Caso | Vetor | Observado no original |
|---|---|---|
| T1 | `morte_natural` (1763, 1772, 1796, 1842) | com `_HealthB` = `_HealthR` = 1, morrem na superfície um soldado preto, uma operária vermelha, um macho preto e um macho vermelho: cadáver e `_BAntsExpired` + 1 **também para as vermelhas** (6.5) |
| T5, T6 | `lutas_soldado`, `lutas_castas` (1793) | tabuleiros de pretas e vermelhas em Bored: dezenas de lutas por tick; nas 250 lutas das gravações, as proporções por classe ficam perto da matriz (testes-tick.md, achado 3; a matriz é complementar, M[i][j] + M[j][i] = 10, então a chance de cada classe independe de quem ataca) |
| T9 | `estrategia_malvada` (1711, 1712, 1767, 1768), `estrategia_vermelha_populosa`, `estrategia_vermelha_sorteio` (1711) | os três caminhos para MALVADO de `_GstrR` (9.1): desespero (`_HealthR` 5, `_BpopT` 13 < `_RpopT`/2 = 14) → `_JustToBeMean` = 56 (S1(100) + 30), `_StrategicModeR` = 0, o contador descendo 1 por tick com o modo em 0 e, ao chegar a 0, a estratégia normal de volta (modo 3 com `_HealthR` 60, `_TilesDugR` < P); "não normal" sem sorteio (`_RpopT` 150, `_BpopT` 20) → 56; pelos sorteios `S32()` = `S128()` = 0 (semente 0x009c escolhida no port com `buscar_semente`) → 83. Nenhuma caixa modal: a mensagem 5 de 4/1020 vai para a faixa |
| — | `estrategia_preta_desespero` (1711) | a escada preta no modo 0 pelo desespero (`_HealthB` 5, `_RpopT` 14 < `_BpopT`/2) |
| T11 | ações `amarela_recrutar_5`, `amarela_recrutar_10`, `amarela_liberar_metade`, `amarela_recrutar_todas`, `amarela_liberar_todas`; ticks `recrutadas` (1726, 1727) | com o jogo pausado, as teclas 1, 2, 3, Ctrl+2 e 4 (formiga-amarela.md 11.4): pretas em Defense 0 → 5 → 15; `_UnRecruit(0)` com `_TemBModePop[6]` = 15 do último tick libera 7 (15 → 8, de trás para frente na lista A, sem chegar à R); `_Recruit(1000)` passa da lista A (29) à B (a operária posta no ninho); `_UnRecruit(1)` zera A e B e põe M = 7 na invasora preta recrutada da lista R. Os ticks seguintes (modo 6, `_DoRecruitAnt`) batem |
| T8 | `revoada_natural` (3722, 3842, 3843) | sem castas forçadas: a colônia vermelha do `rico`, rodando em ultra, fez os próprios machos e rainhas novas e `_GstrR` armou `_FlyAwayR`; voo (16) saindo do mapa, decolagem 15 → 16 por `S1(F)` com F = 5 |
| T8 | `revoada_limiar` (1791, 1855) | o tick exato em que arma, em velocidade normal: com 20 machos vermelhos nas listas, a contagem do tick 1791 (`_Cycle` → 1792, múltiplo de 64) deixa `_CastePopR[3]` + `[4]` = 20 e `_FlyAwayR` fica 0 (o teste é > 20); com 21, a contagem do tick 1855 arma 200 **no próprio tick** (`_FullCount` no passo 5, `_GstrR` no 11) e o passo 13 o deixa em 199 |

## Implementação no port

Código: `engine/src/rules/superficie.rs`. Dados: `data/superficie.json`, gerado por `tools/extrair_superficie.py SIMANTW.EXE --json data/superficie.json` (endereços no topo do script: `_IsCarryCaste`, `_UnCarryCaste`, `_CasteTabB`, `_ModeTabB`, `_ModeTabWB`, `_ModeTabSB`, `_CasteModeTabB`, classe e matriz de luta; e as constantes do bloco `regras`, com a origem de cada uma em `_origem`). Direções e `_TurnTab` vêm de `data/feromonios.json`.

| Original | Port |
|---|---|
| `_DoAntSimA` e os modos | `simular_superficie` (trait: `RegrasSuperficie::simular_superficie`) |
| `_AddFood` | `adicionar_comida` (o seno das `TabelasGeracao`; sem elas, o calculado, igual ao do banco) |
| `_GetStrategy`, `_GstrR`, `_SetCasteProd`, `_SetModeProd` | `escolher_estrategia` |
| `_MakeRedInitiator` | `criar_lider_vermelha` |
| `_GetNewMode`, `_GetWinner`, `_DeadAntHere` | `modo_novo`, `vencedor_da_luta`, `cadaver_aqui` (públicas; implementação em `comum.rs`, a mesma do ninho, da amarela, do quintal e dos inimigos) |
| `_Recruit`, `_UnRecruit`, `_RecruitRed`, `_UnRecruitRed` | `recrutar`, `desrecrutar`, `recrutar_vermelhas`, `desrecrutar_vermelhas` |
| `_GetForageDir`, `_GetNestDir`, `_GetAlarmDir` | as de `feromonios.rs`, sobre uma cópia dos mapas feita no começo do laço e devolvida no fim |
| `_GetRandDir`, `_GetDefendDir`, `_GetRedDefendDir`, `_GetRedBestDirs`, `_GoInNest` | funções privadas de `superficie.rs` |
| `_Bounce`, `_GetDir`, `_GetDis`, `_TileCanBeMovedOn`/`_IsClearTile` (superfície), `_IsItHole`, `_DigTileB/R`, `_SmoothEdges`, `_FixExitMap`, `_PickupFoodA` | `engine/src/rules/comum.rs` (ver [../../decisoes.md](../../decisoes.md), "Módulo comum das regras") |
| `_YellowFight`, `_DoTroph` | `amarela::lutar_com_amarela`, `amarela::trofalaxia` |

Decisões do port:

- A implementação da trait usa `RegrasOriginais::superficie` e `RegrasOriginais::feromonios` (carregados com mods); `tabelas_padrao()` devolve as cópias embutidas, para testes. As funções públicas recebem as tabelas explícitas.
- Sem `TabelasGeracao` (sem os arquivos do jogo, como nos testes de vetores), `adicionar_comida` usa o seno calculado `trunc(sin(i·π/128)·32767)`, igual aos 64 valores do objeto 9/1000, como o laser (`inimigos::seno_calculado`). Até 2026-10-19 ela não fazia nada sem as tabelas; nenhum vetor exercitava a alimentação, e o primeiro (`alimentacao`) mostrou a falta.
- Leituras fora do mapa (que o original faria em memória vizinha) contam como obstáculo; índices de tabela fora da faixa: modo estratégico > 5 usa a última linha da tabela (a mesma escolha do ninho; antes da unificação a superfície devolvia o modo 0), modo ≥ 20 na contagem é ignorado.
- O som de `_AddFood` e o `_EditMessage` do ataque vermelho não geram evento (falta variante em `Evento`); a música 0x2B0A gera `Evento::Musica`.
