# Regras do ninho (preto e vermelho)

Este documento especifica, para reimplementação 1:1, o que as formigas fazem **dentro** dos ninhos: o laço `_DoAntSimB`/`_DoAntSimR`, o despacho por modo (M) e grupo (T), movimento, luta, comida guardada, escavação, mapa de saídas e água. Ovos, rainha e castas estão em [ciclo-de-vida.md](ciclo-de-vida.md). Listas, codificação de T, tabelas de modo e luta estão em [formigas.md](formigas.md); o tick, em [simulacao-visao-geral.md](simulacao-visao-geral.md); os geradores, em [rng.md](rng.md); os valores dos mapas, em [tiles-valores.md](tiles-valores.md).

## Convenções

- **Evidência:** igual à de [simulacao-visao-geral.md](simulacao-visao-geral.md#convenção-de-evidência). Salvo aviso, tudo aqui é **fato visto no código** (pseudocódigo do Ghidra em `re_trabalho/decomp/Code5.c`/`Code6.c`/`Code7.c`), com os trechos ambíguos conferidos na desmontagem do `SIMANTW.EXE` (capstone, modo 16 bits). O pseudocódigo abaixo é próprio, não é cópia do decompilado.
- **Endereços:** funções em `seletor:offset` com o nome do `.SYM`; variáveis em `segmento:offset` do `symbols.tsv` (8 = seletor `1038`, 9 = `1040`, 10 = DGROUP `1048`).
- **Sorteios** (gerador S, LFSR, salvo aviso): `S(n)` = `_SRand1(n)`; `Sk(K)` = `_SRandK()` (máscara, um passo); `SG(n)` = `_SGRand(n)` (dois passos). `RR(n)` = `_RRand(n)` (gerador R, `rand` do C). **A ordem das chamadas é parte da regra**: cada pseudocódigo lista os sorteios na ordem em que acontecem, e um sorteio só acontece se o ramo em que está for executado.
- **Notação:** `D8[d]` = (`_Dx8[d]`, `_Dy8[d]`) (`8:0000`/`8:0008`): 0 = cima (y−1), 2 = direita, 4 = baixo, 6 = esquerda. `inv(d)` = `d XOR 4` (direção oposta). Para a formiga em processamento, de índice `i = _Tindex` (`9:9b6a`): X, Y, M, T, S são os campos da lista do ninho (`_Blist*` `8:3736…8:3f0e`, `_Rlist*` `8:4104…8:48dc`), `pos` = (X, Y), `dir` = T & 7, `grupo` = (T & 0x78) >> 3. `Mapa` e `Vida` são `_MapB`/`_LifeB` no ninho preto e `_MapR`/`_LifeR` no vermelho; `Saída` é `_ExitMapB` (`8:03a4`) ou `_ExitMapR` (`8:13a4`). Todas as matrizes têm índice `x × 64 + y`, com y = 0 na linha da superfície.
- "Ninho B" e "rotina B" são do ninho preto; a versão R é igual salvo o que a seção 11 lista.

## 1. Estado usado pelo ninho

| Variável | Endereço | No save | Papel |
|---|---|---|---|
| `_MapB` / `_MapR` | `10:48e8` / `10:58e8` | sim | terreno do ninho (valores em [tiles-valores.md](tiles-valores.md#ninhos-_mapb-_mapr)) |
| `_LifeB` / `_LifeR` | `10:88e8` / `10:98e8` | não | quem ocupa cada célula (byte T); refeito no load (seção 13.1) |
| `_ExitMapB` / `_ExitMapR` | `8:03a4` / `8:13a4` | sim | "mapa de saída": gradiente que cresce em direção à superfície (seção 7.4) |
| `_HoleMapB` / `_HoleMapR` | `8:82d2` / `8:8312` | sim | para cada coluna x do ninho, a coluna do buraco na superfície (0 = nenhum) |
| `_LastNewHoleB/R`, `_LastNewExitB/R` | `8:835a`/`835e`, `8:8352`/`8356` | sim | último buraco criado: (x, y) na superfície e (coluna, 0) no ninho |
| `_TileTotXB/YB`, `_TilesDugB`, `_TileMassXB/YB` | `9:8104`, `9:811a`, `9:72c8`, `9:7c48`, `9:7c90` | sim | somas de x e y das escavações, contagem e centro de massa (alvo da rainha) |
| idem R | `9:9ddc`, `9:9de2`, `9:7a56`, `9:9fba`, `9:9fd2` | sim | idem |
| `_FoodB` / `_FoodR` | `9:9ea4` / `9:72de` | sim | unidades de comida guardadas (contador, não recalculado do mapa) |
| `_EatCountB` / `_EatCountR` | `9:7402` / `9:7c8e` | sim | "reserva" de alimentação: comer soma 5, cada ovo posto tira 1 |
| `_HealthB` / `_HealthR` | `10:ac86` / `10:ac88` | sim | saúde da colônia, 0..100 |
| `_WaterTable` | `9:99d8` | sim | linha y mais alta já inundada; 64 = nada inundado |
| `_TemBModePop` / `_TemRModePop` | `9:786a` / `9:7be4` | não | contagem de modos do tick (20 palavras cada) |
| `_MeColor` | `10:ce98` | sim (bloco 188) | cor da formiga amarela: 0 preta, ≠ 0 vermelha (o pseudocódigo do Ghidra mostra `_grndTile+2`) |
| `_MeWantFood` | `9:9af2` | sim | a amarela quer ser alimentada (trofalaxia) |
| `_OptionStates[5]` | `8:85fc` | sim | balões de fala; só desenho, sem sorteio |

## 2. O laço: `_DoAntSimB` (`1028:2d4e`) e `_DoAntSimR` (`1028:5344`)

```
n = _ListIndexB                       (lido uma vez, no início)
para i = n−1 até 0 (decrescente):     (_Tindex = i)
    se T[i] ≠ 0: _DoNestAntB(X[i], Y[i], T[i])
```

- Entradas acrescentadas durante o laço (ovos postos, formigas vindas do ninho vermelho pela passagem 0x14) ficam em índices ≥ n e só são processadas no tick seguinte.
- Vagas com T = 0 são puladas **sem sorteio** (diferente de `_DoAntSimA`, que sorteia para toda vaga).
- `_DoAntSimB` roda depois de `_DoAntSimA` e antes de `_DoAntSimR` (ordem do tick em [simulacao-visao-geral.md](simulacao-visao-geral.md#5-um-tick-_doantsim-10280000)).

## 3. Despacho: `_DoNestAntB` (`1028:2dae`) e `_DoNestAntR` (`1028:53a4`)

Há dois ramos, pela cor da formiga em relação ao ninho: **dona** (preta no B, vermelha no R) e **invasora** (vermelha no B, preta no R).

### 3.1 Formiga dona

```
_TemBModePop[M] += 1                                   (R: _TemRModePop)
a = Sk(256)                                             ← sempre
se a == 0 e M ≠ 9:
    b = Sk(32)
    se _HealthB < b:                                    (R: _HealthR)
        T = 0; Vida[pos] = 0; _BAntsExpired += 1       (R: _RAntsExpired); FIM
escolher por M (tabela 3.3)
```

Morte natural: só quando `Sk(256) = 0`, e então com chance (31 − saúde)/32 (b vai de 0 a 31 e precisa ser maior que a saúde). Saúde ≥ 31 impede a morte ([formigas.md](formigas.md#6-luta-morte-e-saúde) diz "≥ 32"; pela comparação estrita, 31 já basta). Ovos (M = 8) e nuvens de luta da cor do ninho (M = 10) também podem morrer assim; só a rainha (M = 9) escapa.

### 3.2 Formiga invasora

Ninho B, formiga vermelha (`_TemRModePop[M] += 1`, sem sorteio de morte):

```
se T ≥ 0xF0 (nuvem de luta vermelha, grupos 14–15): _DoNestFightB; FIM
v = Vida[pos]
se v é a amarela (0xFF/0xFE) e _MeColor == 0: _YellowFight(2, i); FIM
se 1 ≤ v ≤ 0x67 e j = _FindInBList(pos, v) ≥ 0:       (uma preta, ou ovo preto, na MESMA célula)
    se v ≥ 0x60 (cabeça da rainha): T[j] = 0; Vida[X[j], Y[j]] = 0
    se v > 7 (não é ovo):                               (luta; a invasora some)
        w = _GetWinner(T[j], T)                         ← T[j] já é 0 se era a rainha
        T = 0; S[j] = w; T[j] = 0x70 + (w & 0x80); Vida[pos] = T[j]; M[j] = 10; FIM
    senão (roubo de ovo):
        M = 3; T = T OR 8; Vida[pos] = T; T[j] = 0; _TotalEggsDiedB += 1; FIM
se M == 7: _RaidInB (seção 9.1); FIM
SAIR_DO_NINHO   (seção 9.2)
```

Ninho R, formiga preta (`_TemBModePop[M] += 1`, sem sorteio de morte):

```
se T < 8 (ovo preto no ninho vermelho): T = 0; Vida[pos] = 0; FIM       ← o ovo some sem contagem
se T ≥ 0x70 (nuvem de luta preta, grupos 14–15): _DoNestFightR; FIM
v = Vida[pos]
se 0x81 ≤ v ≤ 0xE7 e j = _FindInRList(pos, v) ≥ 0:
    se v ≥ 0xE0 (cabeça da rainha vermelha): T[j] = 0; Vida[X[j], Y[j]] = 0
    se v > 0x87: luta como acima (a invasora some); FIM
    senão: roubo do ovo vermelho: M = 3; T = T OR 8; Vida[pos] = T; T[j] = 0; FIM   (sem contador)
se M == 6:
    se _MePlane == 3: _StayInR (seção 9.3); FIM
    senão SAIR_DO_NINHO
se M == 7: _RaidInR; FIM
SAIR_DO_NINHO
```

Diferenças entre os dois ramos invasores: no R não há teste da formiga amarela na própria célula, há o "fique" do modo 6 (`_StayInR`) e os ovos pretos levados para o ninho vermelho são destruídos; no B não há contrapartida para ovos vermelhos (um ovo vermelho na lista B seria tratado como invasora comum).

**Atenção — rainha atacada na própria célula.** Quando uma invasora está na célula da cabeça da rainha, a rainha é zerada **antes** de `_GetWinner`, que recebe 0 como tipo da ocupante (classe 0, operária). A nuvem resultante guarda S = 0 ou o T da invasora; se "vencer" a ocupante, a nuvem vira T = 0 ao terminar (seção 4.4). Em qualquer caso a cabeça da rainha deixa de existir, e a cauda é removida pelo teste de `_SimQueenB` ([ciclo-de-vida.md](ciclo-de-vida.md#5-rainha)). [hipótese: defeito do original]

### 3.3 Tabela de despacho da formiga dona

| M | Nome (4/1102) | Ninho B | Ninho R |
|---|---|---|---|
| 0, 18, 19, ≥ 20 | Bored, ???, Avenger | PASSEIO (4.1) | PASSEIO (versão R) |
| 1 | Nesting | `_DoNestingB` (5) | `_DoNestingR` (5.4) |
| 2, 5, 7, 11, 12, 15, 16 | Foraging, Dig Out, Attack, Alarmed, Defend Nest, Take off, Flying | `_DoDigOutB` (7.2) | `_DoDigOutR` |
| 3 | Got Food | `_DoFoodInB` (6.2) | `_DoFoodInR` |
| 4 | Dig In | `_DoDigInB` (7.1) | `_DoDigInR` (7.1 + passagem) |
| 6 | Defense | se `_MePlane` ≠ 2: `_DoDigOutB`; senão PASSEIO SEM TROCA (4.2) | `_DoDigOutR` (sempre sai) |
| 8 | Hatching | `_SimEggB` ([ciclo-de-vida.md](ciclo-de-vida.md)) | `_SimEggR` |
| 9 | Egg Laying | `_SimQueenB` ([ciclo-de-vida.md](ciclo-de-vida.md)) | `_SimQueenR` |
| 10 | Fighting | `_DoNestFightB` (4.4) | `_DoNestFightR` |
| 13 | Resting | DESCANSO (10) | DESCANSO (versão R) |
| 14 | Pre-Flight | PRÉ-VOO (10) | PRÉ-VOO com `_FlyAwayR` |
| 17 | ??? | AFOGANDO (8.3) | AFOGANDO |

As funções `_DoRandB` (`1028:3876`), `_DoRecruitN` (`1028:396c`), `_DoRestB` (`1028:367e`), `_DoDrownB` (`1028:37a4`), `_RaidOutB` (`1028:3610`) e suas versões R existem, mas **não têm chamador**: o mesmo código está embutido no `switch` de `_DoNestAntB/R` (seção 12).

## 4. Primitivas de movimento e luta

### 4.1 Andar: `_TryMoveDirB` (`1028:439e`) / `_TryMoveDirR` (`1028:6850`) e PASSEIO

`MOVER(d)`, devolve sucesso:

```
se d < 0: falha
n = pos + D8[d]
se n.x ∉ 0..63 ou n.y > 63: falha
se n.y < 1: devolve _GetOutB(X)          (a formiga está em y = 1 e tenta subir; seção 4.6)
se Mapa[n] ≥ 0x1C: falha                 (passáveis: 0x00–0x1B: túnel, comida 0x10–0x13, 0x14, abertura 0x18)
[só B] se Vida[n] == 0xFF e _MeWantFood ≠ 0 e X < 0x80:
        Vida[pos] = (T & 0xF8) | d; _DoTroph(X, Y, d)      (alimenta a amarela)
Vida[n] = (T & 0xF8) | d; Vida[pos] = 0; pos = n; T = Vida[n]; sucesso
```

- **Não há teste de ocupação.** A formiga entra numa célula ocupada e sobrescreve `Vida[n]`; a outra formiga continua na lista, na mesma posição. O mapa de vida é "o último que escreveu". É assim que as lutas e os roubos começam: a formiga que ficou embaixo vê, no seu próximo processamento, outro T na própria célula (seções 3.2 e 4.3).
- O teste `X < 0x80` (desmontagem: `cmp byte [BlistX+i], 0x80`) é sempre verdadeiro; provavelmente o original queria testar a cor em T. Consequência: qualquer formiga no ninho B, inclusive invasora vermelha, alimenta a amarela. [hipótese: defeito]
- Só 0xFF (cabeça da amarela) dispara a trofalaxia, não 0xFE.

`TENTA(d)` = `se não MOVER(d): MOVER(Sk(8))` — uma tentativa aleatória de reserva; se falhar também, a formiga fica parada.

**PASSEIO** (M = 0, 18, 19 e outros; `_DoRandB`):

```
se Sk(32) == 0: M = _GetNewModeB(grupo)          (R: _GetNewModeR)
se LUTA_PRÓPRIA_CÉLULA(): FIM                    (4.3)
TENTA(dir)
```

### 4.2 PASSEIO SEM TROCA (M = 6 com a amarela no ninho preto; `_DoRecruitN`)

Igual ao passeio, sem o `Sk(32)`: `se LUTA_PRÓPRIA_CÉLULA(): FIM; TENTA(dir)`. Os soldados de defesa ficam andando no ninho enquanto a amarela está lá; não a seguem.

### 4.3 Checagem de luta: `_CheckNestFightB` (`1028:3ba2`) / `_CheckNestFightR` (`1028:61a2`)

`LUTA(c, Tref)`, devolve verdadeiro se houve luta:

```
v = Vida[c]
B: se v é amarela e _MeColor ≠ 0: _YellowFight(2, i); verdadeiro
   se 0x88 ≤ v ≤ 0xE7 e j = _FindInBList(c, v) ≥ 0: NUVEM(j, v, Tref); verdadeiro
R: se 0x08 ≤ v ≤ 0x67: se j = _FindInRList(c, v) ≥ 0: NUVEM(j, v, Tref); verdadeiro
   senão se v é amarela e _MeColor == 0: _YellowFight(3, i); verdadeiro
falso

NUVEM(j, v, Tref):  w = _GetWinner(v, Tref); S[j] = w; T[j] = 0x70 + (w & 0x80); Vida[c] = T[j]; M[j] = 10
```

- `LUTA_PRÓPRIA_CÉLULA()` = `LUTA(pos, T)`: alguém da outra cor entrou na minha célula.
- **A formiga que faz a checagem não some.** Quem vira nuvem é a inimiga encontrada (`j`); a formiga `i` continua na lista, na mesma posição. Se `_GetWinner` devolver `Tref` (o tipo de `i`), a nuvem termina como **uma segunda formiga** igual a `i` (seção 4.4). Isso difere da superfície, onde a formiga que se move desaparece ([formigas.md](formigas.md#6-luta-morte-e-saúde)). Fato visto no código e conferido na desmontagem (`1028:2eea`: nenhum acesso a `T[i]`). [hipótese: defeito do original]
- `_GetWinner(ocupante, atacante)` (`1028:26f4`) usa **`RR(10)`** (gerador R), salvo com `_Invincible = 1` (sem sorteio; a preta vence). A matriz está em [formigas.md](formigas.md#6-luta-morte-e-saúde). `_YellowFight` também chama `_GetWinner` (documentado com a formiga amarela).
- Os ovos da outra cor (0x81–0x87 no B; 0x01–0x07 no R) **não** disparam luta: a formiga passa por cima.

### 4.4 Nuvem de luta: `_DoNestFightB` (`1028:3a54`) / `_DoNestFightR` (`1028:6072`)

```
r = S(7); T = (T & 0xF8) + r; Vida[pos] = T          (animação: direção 0..6)
se Sk(16) ≠ 0: balão se _OptionStates[5] (B: == 1; R: ≠ 0); FIM
T = S; Vida[pos] = T                                  (a luta acabou: vira o vencedor)
se (T & 0x78) == 0x60 (cabeça de rainha, de qualquer cor):
    acrescenta à lista do ninho a cauda: posição pos + D8[inv(T & 7)], tipo T + 8, M = 9, S = 0
B: se T é vermelha: M = 7; senão M = _GetNewMode(grupo(T), T)
R: se T é preta:    M = 7; senão M = VitóriaR[grupo(T)]
```

`VitóriaR` = DGROUP `0x22E6`, 16 bytes lidos do `SIMANTW.EXE` (sem nome no `.SYM`):

| grupo | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| M | 1 | 1 | 1 | 3 | 0 | 5 | 2 | 3 | 0 | 5 | 0 | 0 | 9 | 9 | 10 | 0 |

- Duração: a cada tick a nuvem termina com chance 1/16.
- A nuvem da cor do ninho passa antes pelo sorteio de morte natural (3.1) e pode sumir sem vencedor.
- Uma vermelha que vence no ninho preto passa a M = 7 e, como invasora, entra em `_RaidInB` (procura comida). Uma preta que vence no vermelho, idem.
- A cauda recriada não mexe em `_BlkQueens`/`_RedQueens`.

### 4.5 Direções pelo mapa de saída

**`_GetExitDirB(x, y, d)`** (`1020:119c`; R `1020:1240`), devolve direção + 1, ou 0:

```
se y == 1:
    se Mapa[x][0] == 0x18: devolve 1                    (sobe: direção 0)
    devolve Sk(2) × 4 + 3                               (3 ou 7: direção 2 ou 6, anda de lado)
melhor = 0; res = 0
para k = 0..7, se k ≠ inv(d):                           (com d = 8 nenhuma é excluída)
    c = (x, y) + D8[k]; se c dentro do mapa (0..63, 0..63) e Saída[c] > melhor: melhor = Saída[c]; res = k + 1
devolve res                                             (empate: vence a menor direção)
```

**`_GetEnterDirB(x, y, d)`** (`1020:12e4`; R `1020:137c`), devolve direção ou −1:

```
atual = Saída[x][y]; res = −1
para k = 0..7, se k ≠ inv(d):
    c = (x, y) + D8[k]; se c dentro do mapa e v = Saída[c] ≠ 0 e v ≤ atual:
        se v == atual e Sk(2) == 0: ignora k              (sorteio SÓ em empate)
        res = k; atual = v
devolve res
```

Leitura: `Saída` cresce em direção à superfície (0xFF na linha 0), então "sair" é subir o gradiente e "entrar" é descer até o menor valor não nulo. Células de terra têm `Saída` = 0 e nunca são escolhidas.

**`_RandTurn(d)`** (`1028:2a22`): `_TurnTab[d][Sk(8)]` (tabela em [formigas.md](formigas.md#7-movimento-e-escolha-de-direção)).

### 4.6 Sair do ninho: `_GetOutB` (`1028:520a`) / `_GetOutR` (`1028:74ba`)

Chamada com a coluna x de uma formiga em y = 1 que tentou ir para y ≤ 0.

```
se Mapa[x][0] == 0x18:                                   (há abertura)
    t = T; T = 0
    se _HoleMapB[x] == 0: _MakeNewHoleB(x)               (sorteios: seção 7.5)
    r = Sk(8)
    ok = _ExitHole(_HoleMapB[x], x, (t & 0xF8) + r, M, S)
    se ok: Vida[x][1] = 0; devolve 1                     (a formiga foi para a lista A)
    senão: T = t; M = 0; devolve 0
senão:
    se Saída[x][0] ≠ 0: Saída[x][0] −= 1
    se Sk(2) == 0: se x ≤ 62 e terra(Mapa[x+1][1]): _DigTileThemB(x+1, 1)
    senão:         se x ≥ 1  e terra(Mapa[x−1][1]): _DigTileThemB(x−1, 1)
    MOVER(Sk(8)) a partir de (x, 1)                       (pode chamar _GetOutB de novo)
    devolve 0
```

- `_ExitHole` (`1020:2db6`) põe a formiga no fim da lista A, na primeira vizinha 0..7 do buraco (`_HoleMapB[x]`, x) dentro do quintal com `_MapA` < 0x50; a direção é a sorteada `r` e M é mantido. **S só é mantido com M = 6**; com os outros modos S = 0, ou 120 se M ∉ {3, 7} e o buraco é do lado da cor (pretas: x ≤ 63; vermelhas: x ≥ 65) ([formigas.md](formigas.md#7-movimento-e-escolha-de-direção)). `_ExitHole` **não grava `_LifeA`** (a célula da superfície só é marcada quando a formiga anda lá). Falha se nenhuma vizinha servir. (Revisto no pseudocódigo em 2026-10-17; a versão anterior desta spec dizia "M e S são mantidos".)
- Se `_MakeNewHoleB` não achar lugar, `_HoleMapB[x]` continua 0 e `_ExitHole` tenta em volta de (0, x). [comportamento resultante não testado]
- No pseudocódigo do Ghidra os argumentos M e S aparecem por engano na chamada de `_SRand8`; a desmontagem mostra que são de `_ExitHole`.
- `_LeaveNestB` (`1028:515e`) é a primeira metade desta rotina, sem chamador, e zera `Vida[x][y]` em vez de `Vida[x][1]`.

### 4.7 Entrar no ninho: `_GoInNest` (`1028:257a`)

Da superfície, uma formiga sobre o buraco (x, y) vai para o ninho preto se x < 64, senão para o vermelho: se a lista está cheia (≥ 500), compacta; se continua cheia, nada acontece. Entra na posição (y, 1) com T = (T & 0xF8) + 4 (virada para baixo), mesmos M e S. Se `_HoleMapB[y]` ≠ 0, chama `_DigTileB(y, 1)` (escava se for terra, alisa, refaz `Saída`). Sai da lista A (T = 0, `_LifeA` = 0).

## 5. Cuidar das crias (M = 1): `_DoNestingB` (`1028:44a8`)

O campo S guarda o ovo carregado: bits 0–2 = estágio do ovo, bit 3 = "já achou onde largar" (na operária carregando) ou "acabou de largar" (na operária livre).

### 5.1 Grupo 1 (operária carregando ovo, T 0x08–0x0F)

```
se S >> 3 == 0:                                  (procurando o fundo)
    d = _GetEnterDirB(pos, dir)                   (Sk(2) só em empate)
    se d < 0: S = (S & 7) | 8                     (chegou a um mínimo do mapa de saída)
    TENTA(d)                                      (d < 0 falha sem sorteio e cai no Sk(8))
senão:
    v = Vida[pos]
    se v == 0 ou v > 8:                           (larga o ovo)
        T = T + 8                                 (volta ao grupo 2)
        _PlaceEggB(X, Y, S & 7)                   (seção 7.6; ovo com o MESMO estágio)
        Vida[pos] = T; S = 8; M = _GetNewModeB(1) (= 1, sem sorteio)
    e = _GetExitDirB(pos, dir)
    TENTA(e − 1 se e ≠ 0, senão Sk(8))
```

Detalhe: uma carregadora com T = 0x08 (direção 0) tem `Vida[pos]` = 8 e **não consegue largar** enquanto estiver virada para cima. [hipótese: defeito; efeito pequeno]

### 5.2 Grupo 2 (operária livre)

```
se S >> 3 == 0:
    v = Vida[pos]
    se v == 0 ou v > 7:
        r = S(100)
        se _HealthB < r: COMER(pos)                         (seção 6.1)
        senão se Sk(16) == 0: M = _GetNewModeB(2)
    senão (há um ovo na minha célula):
        j = _FindInBList(pos, v)
        se j ≥ 0: T[j] = 0; T = T − 8; S = v; FIM           (pega o ovo; não anda neste tick)
    TENTA(dir)
senão:                                                       (acabou de largar)
    se Sk(8) == 0: S = 0
    e = _GetExitDirB(pos, dir); TENTA(e − 1 se e ≠ 0, senão Sk(8))
```

O ovo "aparece" em `Vida[pos]` porque a operária passou por cima dele e, no mesmo tick ou no seguinte, `_SimEggB` regravou o T do ovo na célula (seção 4.1).

### 5.3 Outros grupos com M = 1 (soldado, macho…)

```
r = S(100); se _HealthB < r: COMER(pos)
se Sk(8) == 0: M = _GetNewModeB(grupo)
TENTA(dir)
```

### 5.4 Versão vermelha: `_DoNestingR` (`1028:690a`) — regra diferente

```
grupo 1:
    se Sk(4) == 0 e Mapa[pos] < 0x10:
        T = T + 8; Vida[pos] = T; _PlaceEggR(X, Y, 0x82); S = 0; M = _GetNewModeR(1) (= 1); FIM   (não anda)
    se Sk(4) ≠ 0 e d = _GetEnterDirR(pos, dir) ≥ 0: TENTA(d)
    senão TENTA(Sk(8))
grupo 2:
    se Sk(4) == 0:
        v = Vida[pos]
        se v == 0 ou (v & 0x7F) > 7:
            r = S(100); se _HealthR < r: COMER_R(pos) senão M = _GetNewModeR(2)
        senão se j = _FindInRList(pos, v) ≥ 0: T[j] = 0; T = T − 8; FIM
    se Sk(4) ≠ 0: TENTA(dir) senão TENTA(Sk(8))
outros: M = _GetNewModeR(grupo); TENTA(dir)
```

Diferenças de efeito: a vermelha larga o ovo ao acaso (1/4 por tick em célula sem comida), não leva ao fundo; **o estágio do ovo se perde** (S não guarda nada e o ovo largado volta ao estágio 2, T = 0x82); a vermelha pega também ovos pretos (`v & 0x7F`); e, sem fome, troca de modo na hora (sem o `Sk(16)`).

## 6. Comida guardada

### 6.1 Comer: `_TryEatFoodB` (`1028:47c6`), `_EatFoodB` (`1028:4844`)

`COMER(c)`, embutido em várias rotinas:

```
v = Mapa[c]
se 0x10 ≤ v ≤ 0x13:
    Mapa[c] = (v == 0x10) ? Sk(8) : v − 1          (0x10 = 1 unidade; ao acabar vira túnel)
    se _FoodB > 0: _FoodB −= 1
    _EatCountB += 5
    se _EatCountB > (_BpopT + _CastePopB[2]) >> 4:  (soldados contam duas vezes)
        _EatCountB = 0; se _HealthB < 100: _HealthB += 1
```

`_EatFoodB` é a mesma coisa sem o teste de faixa. R: `_FoodR`, `_EatCountR`, `_RpopT + _CastePopR[2]`, `_HealthR`.

Quem come e quando (o sorteio acontece sempre que a linha é alcançada; `COMER` só age se houver comida na célula):

| Onde | Condição | Célula |
|---|---|---|
| Nesting, grupo 2 e outros | `S(100)` > saúde | a própria |
| Got Food, ao largar | `S(100)` > saúde | a própria (a recém-largada) |
| Dig In, Dig Out, depois de andar | `Sk(64)` > saúde | a nova (preto); a antiga (vermelho) |

Com saúde ≥ 63, Dig In/Dig Out nunca comem; com saúde 100, ninguém come.

`_DecEatB` (`1028:48f8`), chamado por ovo posto pela formiga amarela (a rainha usa o mesmo código embutido): `_EatCountB −= 1`; se ficar < 0: `_EatCountB = _BpopT >> 5` e, se `_HealthB` > 0 e `_AlwaysHealthy` = 0, `_HealthB −= 1`. `_DecEatR` não testa `_AlwaysHealthy`.

### 6.2 Levar comida: `_DoFoodInB` (`1028:492a`) (M = 3)

```
d = _GetEnterDirB(pos, dir)                        (Sk(2) só em empate)
se d < 0 ou Sk(16) == 0:                           (Sk(16) só se d ≥ 0)
    LARGAR_COMIDA(pos)                              (6.3)
    se T & 8: T = T − 8                             (grupo 3→2, 7→6)
    r = S(100); se _HealthB < r: COMER_SEM_TESTE(pos)   (_EatFoodB embutido: sem o teste 0x10–0x13)
    M = _GetNewModeB(grupo do T de ENTRADA)
senão:
    T = (T & 0xF8) | d; Vida[pos] = T
    n = pos + D8[d]; fora do mapa: FIM; n.y < 1: _GetOutB(X); FIM
    se Mapa[n] ≥ 0x30: FIM                          (fica parada)
    PASSO(n, d)                                      (6.4)
```

`COMER_SEM_TESTE` não confere a faixa: se a célula era 0x13 (cheia) ou 0x14 (passagem), LARGAR não a muda e a mordida a faz cair para 0x12 ou 0x13 (a passagem vira comida). Visto no pseudocódigo (`1028:492a`), igual em `_DoFoodInR`. [hipótese: defeito]

**Comida dobrada.** O novo modo é calculado com o grupo **antes** de tirar a comida (3 ou 7), e `_CasteModeTabB[3] = _CasteModeTabB[7] = 3`. A formiga fica em M = 3 sem carga; no próximo largar (fundo do túnel ou 1/16 por tick) ela credita mais uma unidade no mapa e em `_FoodB`, e só então troca de modo (agora com grupo 2 ou 6). Cada carga trazida rende **duas** unidades. Conferido na desmontagem (`1028:4bab`: o argumento é o T recebido, `[bp+0xa]`). [hipótese: defeito do original; confirmar na bancada, caso N-06]

### 6.3 Largar: `_DropFoodB` (`1028:3c3c`)

`LARGAR_COMIDA(c)`: se `Mapa[c]` < 0x10 (túnel), vira 0x10; senão, se < 0x13, soma 1; senão (0x13 cheia, ou 0x14 passagem) o mapa não muda. Em todos os casos `_FoodB += 1`, de modo que o contador pode ficar maior que a comida desenhada. A função separada (sem chamador) faz o mesmo, devolve 0 quando o mapa não mudou e tira o bit 8 de T.

### 6.4 Passo com checagem de luta (embutido em Dig In, Dig Out e Got Food)

`PASSO(n, d)`: `Vida[pos] = 0`; se `LUTA(n, (T & 0xF8) | d)`: FIM (a formiga fica, mas com `Vida[pos]` = 0); senão `T = (T & 0xF8) | d`, `Vida[n] = T`, `pos = n`. Não testa `Mapa[n]` < 0x1C: quem chama já exige `Mapa[n]` < 0x30 e (em Dig In/Out) "não é terra".

### 6.5 Roubo e coleta

- `_StealFoodB` (`1028:48b4`): tira uma unidade (0x10 → `Sk(8)`) e `_FoodB −= 1` se > 0, sem mexer na saúde. Embutido em `_RaidInB`/`_StayInR`.
- `_PickupFoodB` (`1020:0f40`) / `_PickupFoodR` (`1020:0fa2`): igual, só se 0x10–0x13. Sem chamador no pseudocódigo.

## 7. Escavação e mapa de saída

### 7.1 Cavar para dentro: `_DoDigInB` (`1028:4bd0`) (M = 4)

```
se grupo ∉ {2, 6}: M = _GetNewModeB(grupo); FIM
d = _GetEnterDirB(pos, dir); se d < 0: d = Sk(8)
T = (T & 0xF8) | d; Vida[pos] = T
se Y == 63: M = _GetNewModeB(grupo); FIM
n = pos + D8[d]; fora do mapa: FIM; n.y < 1: _GetOutB(X); FIM
se Mapa[n] ≥ 0x30: FIM
se terra(Mapa[n]) (0x20–0x2E):
    se não _DigTileThemB(n): M = _GetNewModeB(grupo); FIM       (R: M = 0)
    T = T + 0x18 (carrega terra: grupo 2→5, 6→9); M = 5; som 0x11 (R: 0x12)
PASSO(n, d); se houve luta: FIM                 (Tref da luta = (T de entrada & 0xF8) | d, SEM os 0x18 da terra)
r = Sk(64); se _HealthB < r: COMER(n)          (R: COMER(pos ANTIGA))
se Sk(4) == 0: _FixExitMapB(n)
[só R] se Mapa[n] == 0x14: PASSAGEM (7.7)
```

A operária anda para dentro da célula que acabou de cavar, já carregando a terra (M = 5, que no ninho é "sair": seção 7.2).

### 7.2 Sair carregando / sair: `_DoDigOutB` (`1028:4eb0`) (M = 2, 5, 7, 11, 12, 15, 16)

```
e = _GetExitDirB(pos, dir)
d = (e ≥ 1) ? e − 1 : _RandTurn(dir)                     (Sk(8) em _RandTurn)
T = (T & 0xF8) | d; Vida[pos] = T
n = pos + D8[d]; fora do mapa: FIM; n.y < 1: _GetOutB(X); FIM
se Mapa[n] ≥ 0x30 (terra inundada, pedrinha…):
    B: Saída[pos] −= 1 (byte; 0 vira 0xFF!)       R: só se Saída[pos] ≠ 0
    se grupo ∈ {5, 9}: T = T − 0x18; M = 4          (a terra some; volta a cavar)
    se grupo ∈ {2, 6}: M = 4
    FIM
se terra(Mapa[n]): FIM                               (parada, sem mudança)
PASSO(n, d); se houve luta: FIM
r = Sk(64); se _HealthB < r: COMER(n)          (R: COMER(pos ANTIGA))
```

- **No ninho vermelho, Dig In e Dig Out comem na célula de onde a formiga saiu**, não na nova (pseudocódigo de `_DoDigInR` `1028:6f0c` e `_DoDigOutR` `1028:722a`: o índice usado é o calculado antes de atualizar a posição). Visto no código em 2026-10-17.
- A terra carregada (grupos 5 e 9) só é descarregada na superfície (`_DoDigOutAntA`); no ninho, se o caminho estiver bloqueado, ela simplesmente some.
- No ninho preto, o decremento de `Saída` sem teste (conferido na desmontagem, `1028:4f83`) transforma uma célula com 0 em 0xFF, um falso "ponto de saída". [hipótese: defeito; a versão R testa]

### 7.3 Funções de escavação

**`_DigTileB(x, y)`** (`1020:1fe4`), usada para cavar sem condição (ovo, entrada, geração):

```
se terra(Mapa[x][y]):
    Mapa[x][y] = Sk(8)
    _TileTotXB += x; _TileTotYB += y; _TilesDugB += 1           (somas de 32 bits)
    _TileMassXB = _TileTotXB / _TilesDugB; _TileMassYB = _TileTotYB / _TilesDugB
    se y > 53 e S(64) == 0:                                      (passagem entre ninhos)
        Mapa[x][y] = 0x14
        se terra(_MapR[x][y]): _MapR[x][y] = Sk(8); acumula nos totais R
        ALISAR_R(x, y); _FixExitMapR(x, y); _MapR[x][y] = 0x14
ALISAR_B(x, y); _FixExitMapB(x, y)                               (sempre, mesmo sem escavar)
```

`ALISAR(x, y)` = `_SmoothEdges(x, y−1)`, `(x+1, y)`, `(x, y+1)`, `(x−1, y)`, nessa ordem.

**`_DigTileR(x, y)`** (`1020:21de`): igual, sem a passagem 0x14 (só o ninho preto cria passagens).

**`_DigTileThemB(x, y)`** (`1020:22d4`), usada pela operária: só cava se `0 < x < 63`, se a célula de baixo for terra (ou y > 62) e a de cima for terra (ou y < 3); não exige que a própria seja terra. Em y = 0 grava 0x18 e chama `_MakeNewHoleB(x)`; senão grava `Sk(8)`. Acumula sempre nos totais (mesmo recavando), alisa e refaz `Saída`. Devolve 1 se cavou. A regra "terra em cima e embaixo" faz túneis de uma célula de altura.

**`_SmoothEdgesB/R`** (`1020:255a`/`26e4`): regra em [tiles-valores.md](tiles-valores.md#ninhos-_mapb-_mapr). Detalhes que importam para o 1:1:

- y > 63 ou x fora de 0..63: nada. y = 0: se `Mapa[x][0]` ≤ 0x2F, vira 0x18 (abre a superfície acima de qualquer escavação na linha 1). y < 0 (chamada com y−1 = −1) não é filtrado [só acontece se alguém cavar y = 0 com `_DigTileB`].
- Em y = 1 o vizinho de cima conta sempre como sólido.
- Só sorteia (`Sk(8)`) quando a célula de terra fica sem nenhum vizinho sólido e não está inundada.

### 7.4 Mapa de saída: `_FixExitMapB` (`1020:284e`) / `_FixExitMapR` (`1020:2914`)

```
se y < 2: Saída[x][y] = (Mapa[x][y] == 0x18) ? 0xFF : 0xFE
senão: m = maior Saída entre as 8 vizinhas dentro do mapa; Saída[x][y] = (m == 0) ? 0 : m − 1
```

- Atualiza **uma** célula, sem propagar. Quem chama: toda escavação (`_DigTile*`, `_DigTileTheme*`), `_PlaceEggB` (via `_DigTileB`), e a operária em Dig In com chance 1/4 depois de cada passo. A linha 0 começa toda em 0xFF na geração.
- **Verificado no `QUICK2.ANT`**: a câmara da rainha preta tem `_ExitMapB` (34, 9) = 0xF6, (35, 9) = 0xF5, (36, 9) = 0xF4; a vermelha tem (39, 11) = 0xF4, (40, 11) = 0xF3, mas (41, 11) = 0. A diferença é explicada pela regra: (41, 11) foi cavada primeiro, sem vizinho com valor, e nunca recalculada; (36, 9) recebeu ovos (`_PlaceEggB` → `_DigTileB` → `_FixExitMapB`), que a recalcularam (máximo vizinho 0xF5 − 1).

### 7.5 Buracos na superfície

- `_MakeNewHoleB(c)` (`1020:1b06`) e `_MakeNewHoleR(c)` (`1020:1d02`): regra completa em [geracao-mundo.md](geracao-mundo.md#3a-buracos-de-entrada-_makenewholebc-10201b06-e-_makenewholerc-10201d02). Em jogo, são chamadas por `_GetOutB/R` (primeira saída por uma abertura nova), `_DigTileThemB/R` em y = 0 e pela escavação da amarela (`_DigMyTile`). Sorteios: `S(31)` e nada mais no quintal; depois `_DigTileB(c, 1)` (B) ou a escavação embutida (R).
- `_CreateNewHole(x, y)` (`1020:171a`), chamada por `_DigMyNewHole` (a amarela abre um buraco na superfície): exige 0 < x < 127 e 0 < y < 63. Quintal: `_MapA` = 0x50 com borda (tabela `DGROUP:0x230c`); casa: `_MapA` = 0x59. Se x < 64: `_HoleMapB[y] = x`, `_DigTileB(y, 1)`, `_LastNewHoleB = (x, y)`, `_LastNewExitB = (y, 0)`. Senão: `_HoleMapR[y] = x`; se `_MapR[y][1]` é terra, `Sk(8)` e totais R; alisa (y, 0), (y+1, 1), (y, 2), (y−1, 1); `_FixExitMapR(y, 1)`; `_LastNewHoleR`/`_LastNewExitR`.

### 7.6 Pôr um ovo: `_PlaceEggB` (`1020:1004`) / `_PlaceEggR` (`1020:1068`)

Se a lista tem < 500 entradas e 0 ≤ x ≤ 63, 1 ≤ y ≤ 63: `_DigTileB(x, y)` (cava se for terra; sorteios da seção 7.3), acrescenta (x, y, T = estágio, M = 8, S = 0) e grava `Vida[x][y]`. Não testa ocupação.

### 7.7 Passagem entre ninhos (só vermelhas indo para o ninho preto)

As células 0x14 existem nos dois mapas, na mesma (x, y) (7.3). Em `_DoDigInR`, se depois de andar a operária ou soldado vermelho estiver numa célula 0x14:

```
T_R = 0; Vida_R[n] = 0
novo T = ((T & 0x7F) < 0x30) ? 0x90 + Sk(8) : 0xB0 + Sk(8)    (operária ou soldado vermelho, direção sorteada; perde a carga)
_AddAntToBList(n, novo T, M = 3, S = 0); _LifeB[n] = novo T
```

(O M = 3 e o S = 0 vêm da desmontagem, `1028:71ef`.) No ninho preto, como invasora com M = 3, ela segue o mapa de saída até sair (3.2). Não há caminho inverso: pretas nunca usam a passagem.

### 7.8 `_FloodNestB` (`1020:29da`)

Para todo x e y = 3..63: terra 0x20–0x2D soma 0x31; valor < 0x14 vira 0x50. **Sem chamador**; os valores (0x50, +0x31) não batem com a inundação real (seção 8). [hipótese: resto de versão antiga]

## 8. Água

### 8.1 `_DoWater` (`1020:07c6`), todo tick

```
se _RainOn ≠ 0 e _TERRAINset == 0:                         (chovendo no quintal)
    se S(50) == 0: som 0x29
    _ClearRainFlag = 1
    se S(10) == 0 e _WaterTable > 4:
        _WaterTable −= 1; w = _WaterTable
        _DrownBList(w); _DrownRList(w)
        para x = 0..63: _MapB[x][w] = (_MapB < 0x20) ? 0x4E : _MapB + 0x2F
                        _MapR[x][w] = (_MapR < 0x20) ? 0x4E : _MapR + 0x2F
    para i = 0..99: gotas na superfície (abaixo)
senão se _TERRAINset == 0:                                  (seco, no quintal)
    se _ClearRainFlag == 1: _ClearRainFlag = 0; para i = 0..99: se _MapA da gota i ∈ 0x74..0x77: = S(14)
    se _WaterTable < 64 e S(10) == 0:                       (S(10) só se < 64)
        w = _WaterTable
        para x = 0..63: _MapB[x][w] = (_MapB == 0x4E) ? S(8) : _MapB − 0x2F
                        _MapR[x][w] = (_MapR == 0x4E) ? S(8) : _MapR − 0x2F    (B e R alternados por coluna)
        _WaterTable += 1
(na casa, nada)
```

Gotas (superfície; detalhe em `superficie.md`): para cada i, na célula (`_RxTab[i]`, `_RyTab[i]`) (`9:79e6`/`9:7a72`): se o valor é 0x74–0x76, soma 1 (a poça cresce); senão, se é 0x77 vira `S(14)`, e em seguida `_PlaceDrop(i)` sorteia nova posição com **`RR(128)` e depois `RR(64)`** (gerador R); se ali o valor for < 0x0E, vira 0x74 e o cheiro daquela célula 2×2 é lavado (`_PherMapA`, `_PherMapF`, `_PherMapBT`, `_PherMapRT` = 0; `_PherMapBN`, `_PherMapRN` −20, mínimo 0).

Consequências:

- A água sobe uma linha por vez, de baixo (y = 63) para cima, com chance 1/10 por tick de chuva, até y = 4. As linhas 0–3 nunca inundam. Ao secar, desce de cima para baixo, com chance 1/10 por tick seco.
- Comida guardada (0x10–0x13) numa linha inundada vira 0x4E e, ao secar, túnel `S(8)`: **a comida se perde**, sem baixar `_FoodB`.
- Túnel inundado (0x4E) é intransponível (`MOVER` exige < 0x1C; `PASSO` exige < 0x30). Ovos e rainha numa linha inundada não se afogam.

### 8.2 `_DrownBList(w)` (`1020:2d16`) / `_DrownRList` (`1020:2d66`)

Para cada formiga da lista (de trás para frente) com Y = w, T ≠ 0 e grupo 1..11: M = 17. Ficam de fora ovos (grupo 0), rainha (12, 13) e nuvens (14, 15). Vale para donas e invasoras.

### 8.3 AFOGANDO (M = 17; `_DoDrownB` embutido)

Só a formiga dona chega aqui (a invasora ignora M = 17 e segue saindo):

```
se Mapa[pos] ≤ 0x13: M = _GetNewModeB(grupo); FIM               (já não está na água)
r = S(3); T = (T & 0xF8) | ((T + r − 1) & 7); Vida[pos] = T      (gira −1, 0 ou +1)
se S(100) == 0: Vida[pos] = 0; T = 0; conta morte: _RAntsExpired se T era vermelha, senão _BAntsExpired
```

Ela não anda (túnel inundado é intransponível) e morre com chance 1/100 por tick até a água baixar.

`_AddWater(w)` (`1020:0b8a`) e `_DropWater(w)` (`1020:0c54`) são o mesmo código de inundar/secar uma linha, em função separada, **sem chamador**.

### 8.4 Como a chuva se comporta no ninho e nos túneis (resumo para o jogador)

Pergunta do usuário (2026-10-18): "Como a chuva deve se comportar no ninho? No momento ela acumula no fundo da terra e aparenta não ter efeito no ninho" e "Qual é o comportamento da chuva nos túneis?". Resposta, a partir de 8.1–8.3 (código) e das capturas abaixo:

- **A água vem de baixo, como um lençol freático.** Enquanto chove, em cada tick há 1 chance em 10 de a água subir **uma linha inteira** dos dois ninhos (`_WaterTable` −1), começando na linha 63 (o fundo) e no máximo até a linha 4 (as linhas 0–3 nunca inundam). Uma chuva dura `S(150) + 150` ticks (150–299, quintal.md §2): em média a água sobe 15 a 30 linhas, isto é, **fica na metade de baixo do ninho**. Quando a chuva acaba, a água desce uma linha a cada ~10 ticks secos, de cima para baixo. Por isso o comportamento observado ("acumula no fundo") é o do original: na maior parte das chuvas a água não chega aos túneis, que costumam ficar perto da superfície. Só chuvas seguidas (a água não teve tempo de baixar) ou longas a levam até lá.
- **Terra inundada quase não muda de cor.** A terra de uma linha inundada passa de 0x20–0x2D para 0x4F–0x5C (+0x2F), desenhada quase igual à terra seca (azul-escuro pontilhado); o que se vê de longe é **o túnel inundado, azul-claro** (0x4E). Sem túnel na linha, a subida da água praticamente não aparece.
- **Túnel inundado (0x4E) vira parede de água:** nenhuma formiga entra nele (`MOVER` exige < 0x1C e `PASSO` < 0x30). A colônia perde a parte de baixo do ninho até a água descer; ao secar, o túnel volta como túnel comum (`S(8)`: um dos tiles de túnel vazio).
- **Formigas:** no tick em que a linha delas inunda, as operárias, soldados e reprodutores (grupos 1–11) dessa linha entram em **AFOGANDO** (M = 17): não andam, giram no lugar (direção −1, 0 ou +1 por tick) e **morrem com chance 1/100 por tick**, contadas como mortas (`_BAntsExpired`/`_RAntsExpired`); se a água baixar antes, voltam ao trabalho. Formigas que entram em linha já inundada não se afogam (o tile as barra). Invasoras ignoram o afogamento e seguem saindo. Não há fuga: só a água descendo as salva.
- **Ovos, larvas, pupas e rainha não se afogam** (grupos 0, 12, 13 ficam fora de `_DrownBList`), mesmo debaixo d'água; mas ficam ilhados (as operárias não chegam a eles pelo túnel inundado).
- **Comida guardada** (0x10–0x13) numa linha que inunda vira água e, ao secar, túnel vazio: **a comida se perde**, sem baixar `_FoodB`.
- **A amarela** numa célula inundada do ninho perde 1 de vida por tick; com vida 0 por mais de 99 ticks, morre **afogada** (código 7, formiga-amarela.md §3, §9).
- **Na superfície**, as gotas viram poças (tiles 0x74–0x77) que crescem e somem e **lavam o cheiro** da célula 2×2 (trilhas, alarme, comida; o cheiro do ninho cai 20): as trilhas de comida se perdem na chuva (§8.1, quintal.md §9.2).

**Verificado no original (bancada QEMU, 2026-10-18, `cenarios_tick.py gravar tela_chuva`, snapshot `rico`, `_RainCnt` = 2000):** com a água na linha 25, o ninho preto do `rico` (túneis nas linhas 0–12) aparece igual ao seco; com a água na linha 4 (depois de ~300 ticks de chuva), os túneis abaixo da linha 4 estão azul-claros e a terra continua azul-escura. O port desenha os mesmos estados (`chuva_ninho_preto_meio.ANT`, `chuva_ninho_preto.ANT` abertos com `--save`) igual: túneis inundados azul-claros a partir da mesma linha. Os efeitos de regra (linhas, afogamento, comida) já eram conferidos pelo caso Q-01 e pelos vetores de tick (quintal.md §9, Q-U8). Capturas em `re_trabalho/a51/chuva2/` (fora do git).

## 9. Invasoras

### 9.1 Atacar o ninho: `_RaidInB` (`1028:3524`) / `_RaidInR` (`1028:5b2a`) (invasora com M = 7)

```
se Mapa[pos] ∈ 0x10..0x13:                                   (achou comida)
    tira uma unidade (0x10 → Sk(8)); _FoodB −= 1 se > 0      (_FoodB do ninho invadido)
    M = 3; T = T OR 8; Vida[pos] = T                          (passa a "com comida"; vai sair)
senão:
    r = S(3)
    se não MOVER((T + r − 2) & 7):                            (gira 0, −1 ou −2)
        d = _GetEnterDirB(pos, dir); se d < 0: d = S(8)
        se não MOVER(d): M = 1; Vida[pos] = T                 (desiste; com M ≠ 7 passa a sair)
```

Note `S(8)` (resto) e não `Sk(8)` aqui; os dois dão um passo do gerador.

### 9.2 SAIR_DO_NINHO (invasora; `_RaidOutB` embutido)

```
e = _GetExitDirB(pos, 8)                                     (nenhuma direção excluída)
se não MOVER(e ≠ 0 ? e − 1 : Sk(8)):
    se não MOVER(Sk(8)): Vida[pos] = T
```

### 9.3 Ficar no ninho vermelho: `_StayInR` (`1028:5c16`)

Preta em Defense (M = 6) no ninho vermelho, com a amarela também lá (`_MePlane` = 3):

```
se Mapa[pos] ∈ 0x10..0x13: rouba uma unidade (_FoodR); M = 3; T = T OR 8
senão:
    r = S(3); d = (T + r − 2) & 7; T = (T & 0xF8) | d
    se MOVER(d): FIM
    d2 = _GetEnterDirR(pos, dir); se d2 < 0: d2 = S(8)
    se MOVER(d2): FIM
Vida[pos] = T
```

## 10. Pré-voo e descanso

**PRÉ-VOO** (M = 14):

```
se Sk(32) == 0: M = _GetNewModeB(grupo)
se não LUTA_PRÓPRIA_CÉLULA(): TENTA(dir)
se _FlyAwayB > 100: M = 15                                    (R: _FlyAwayR)
```

**DESCANSO** (M = 13; `_DoRestB`):

```
se LUTA_PRÓPRIA_CÉLULA(): FIM
Vida[pos] = T
se S(20) ≠ 0: balão de descanso se _OptionStates[5] (B: == 1; R: ≠ 0); FIM
M = _GetNewMode(grupo, T)
```

A formiga descansando não anda e regrava a própria célula em `Vida` a cada tick.

## 11. Diferenças entre o ninho vermelho e o preto

| Ponto | Ninho preto (B) | Ninho vermelho (R) |
|---|---|---|
| Novo modo | `_GetNewModeB`: com `_ModeAuto` = 0, operária/soldado usam `_ModeMe` (triângulo do jogador) | `_GetNewModeR`: sempre `_ModeTabWB/SB[_StrategicModeR][Sk(8)]` |
| Defense (6) | fica no ninho se a amarela está nele (4.2) | sempre sai |
| Nesting (1) | leva o ovo ao fundo, guarda o estágio | larga ao acaso (1/4), estágio volta a 2 (5.4) |
| Dig In, terra não cavável | `M = _GetNewModeB(grupo)` | `M = 0` |
| Dig In, célula 0x14 | nada | vai para o ninho preto (7.7) |
| Dig In/Dig Out, mordida no estoque | célula nova | célula antiga |
| Dig Out bloqueado | `Saída[pos] −= 1` sem teste (0 → 0xFF) | só se ≠ 0 |
| Trofalaxia em `MOVER` | sim | não |
| Teste da amarela na checagem de luta | antes do teste de inimiga | depois |
| Invasora na própria célula | teste da amarela preta; roubo de ovo conta `_TotalEggsDiedB` | sem teste da amarela; roubo sem contador; ovo preto (T < 8) é destruído |
| Invasora em M = 6 | sai | `_StayInR` se a amarela está no ninho vermelho |
| Fim da luta, vencedora da cor do ninho | `_GetNewMode(grupo, T)` | tabela `VitóriaR` (4.4) |
| `_DecEat` | respeita `_AlwaysHealthy` | não |
| Balões (descanso, luta) | `_OptionStates[5] == 1` | `≠ 0` |
| Ovos, rainha | [ciclo-de-vida.md](ciclo-de-vida.md#9-diferenças-vermelho--preto) | |

## 12. Funções presentes mas não chamadas

Sem chamador no pseudocódigo (o mesmo código está embutido em outra rotina): `_DoRestB/R`, `_DoDrownB/R`, `_DoRandB/R`, `_DoRecruitN`, `_RaidOutB/R`, `_LeaveNestB`, `_TryEatFoodB/R`, `_EatFoodB/R`, `_StealFoodB/R`, `_PickupFoodB/R`, `_MakeNewTailB/R`, `_KillTailB/R`, `_FloodNestB`, `_AddWater`, `_DropWater`, `_FoodFall`. `_CheckNestFightB/R`, `_DropFoodB/R`, `_LostHeadB/R` e `_LostTailB/R` aparecem só na própria definição. O port deve implementar o comportamento embutido descrito aqui (há diferenças pequenas entre as funções soltas e o código embutido, por exemplo `_LeaveNestB`).

## 13. Casos de teste contra o original

Bancada: [../../bancada-qemu.md](../../bancada-qemu.md#casos-de-teste-injetar-um-save-e-rodar). Ela injeta e carrega saves editados, lê o estado inteiro da memória e escreve variáveis (`escrever_variavel`), mas o estado dos geradores **não** vai para o save (e é perdido no load; ver [rng.md](rng.md#a-semente-está-no-save)). Por isso os casos abaixo são de três tipos: **D** = determinístico (não depende do sorteio), **E** = estatístico (repetir e comparar proporções), **P** = comparação com o port rodando o mesmo `.ANT` com a mesma semente forçada.

### 13.1 Como montar a entrada

- Partir de `tests/referencias/saves/QUICK2.ANT` (tick 78) e descrever a edição num caso JSON em `tests/referencias/casos/` (operações de `tools/ant_editar.py`: escalares, células, formigas com `ListIndex`). `tools/caso_ant.py rodar CASO.json` injeta o save num disquete, carrega pelo File > Load, tira a pausa, roda até o `GameTime` alvo, pausa, lê o estado da memória (sem precisar salvar) e confere.
- O load deixa o jogo **pausado** (`_GamePaused` = 1) e **reconta** `_CastePopB/R`, `_BpopT/_RpopT` pelas listas [observado na bancada, 2026-10-16]: populações não podem ser forçadas pelo arquivo, só pelas listas.
- **No load o original refaz `_LifeB/R` a partir das listas** (`_DoLoadInitializations` `1010:9dbe`), do índice `_ListIndexB` (inclusive: a vaga logo depois da última também é escrita) até 0. Quando duas formigas estão na mesma célula, a de **menor índice** fica em `Vida`. Zere a vaga `_ListIndexB` para não criar ocupante fantasma.
- Compare por `_GameTime` (offset 45740), nunca por tempo de relógio.

### 13.2 Casos

| Id | Regra | Entrada | Esperado | Tipo |
|---|---|---|---|---|
| N-01 | morte natural (3.1) | QUICK2 com `_HealthB` = 0 e 40 operárias pretas (T 0x12, M 13, descansando) no túnel | perda média de 40 × (1/256) × (31/32) ≈ 0,15 formiga/tick (≈ 19 em 128 ticks); `_BAntsExpired` sobe igual. Com `_HealthB` = 31: nenhuma morte | E (e D para saúde 31) |
| N-02 | rainha imune (3.1) | `_HealthB` = 0 | a rainha não morre por 3.1, só por fome (ciclo-de-vida) | D |
| N-03 | sair do ninho (4.5, 4.6, 7.2) | uma operária preta T 0x12, M 2, em (32, 3) no túnel do QUICK2 | sobe a coluna 32 (`Saída` 0xFC → 0xFD → 0xFE) e aparece na lista A ao lado do buraco (3, 32) no 3º tick, com M = 2 e S = 120 | D (exceto a direção na superfície) |
| N-04 | levar ovo ao fundo (5.1) | operária T 0x0A (carregando), M 1, S 1 em (32, 2) | desce pelo gradiente (32,3) → (31,4) → (30,5) → (31,6) → (32,7) → (33,8) → (34,9) → (35,9) → (36,9), o mínimo (0xF4); ali liga o bit 3 de S, dá um passo `Sk(8)` e no tick seguinte larga um ovo T = 1 onde estiver; ela vira grupo 2 com S = 8 | D até a câmara, E depois |
| N-05 | ninho vermelho larga ao acaso (5.4) | vermelha T 0x8A, M 1 no túnel vermelho | o ovo largado tem T = 0x82, em qualquer célula do caminho | D (estágio), E (lugar) |
| N-06 | comida dobrada (6.2) | operária preta T 0x1A (com comida), M 3, em (32, 3); `_FoodB` = 0 | ao fim, `_FoodB` = 2 e o mapa tem 2 unidades (0x11 numa célula, ou 0x10 em duas) | D no total |
| N-07 | comer e saúde (6.1) | `_HealthB` = 50, `_EatCountB` = 0, `_BpopT` = 3, soldados 0; um soldado (T 0x32) M 1 preso numa célula 0x13 cercada de terra | cada mordida soma 5 a `_EatCountB`; como 5 > 3 >> 4 = 0, a saúde sobe 1 por mordida e o contador volta a 0; mordida com chance 49/100 por tick (`S(100)` > 50); a célula vai 0x13 → 0x12 → 0x11 → 0x10 → túnel; `_FoodB` desce até 0 | E |
| N-08 | cavar (7.1, 7.3) | operária T 0x12, M 4 em (36, 9) | túneis novos têm uma célula de altura; a operária vira T 0x2x, M 5, depois volta a M 4; `_TilesDugB` e `_TileMassXB/YB` mudam como na fórmula | D na forma, E no traçado |
| N-09 | decremento sem teste (7.2) | no ninho preto, `_ExitMapB` = 0 numa célula de túnel cercada de terra inundada (0x4F+) com uma operária M 2 nela | `_ExitMapB` da célula passa a 0xFF; no ninho vermelho, nas mesmas condições, fica 0 | D |
| N-10 | inundação (8.1, 8.2, 8.3) | `_RainOn` = 1, `_RainCnt` = 600, `_WaterTable` = 64 | `_WaterTable` cai cerca de 1 a cada 10 ticks até 4; cada linha y = `_WaterTable` mostra 0x4E nos túneis e +0x2F na terra; formigas nela ficam com M = 17 e morrem ≈ 1 %/tick; ovos e rainha não mudam de M | E (ritmo), D (valores) |
| N-11 | seca (8.1) | depois de N-10, chuva parada | linhas voltam de cima para baixo; comida que estava numa linha inundada some do mapa, `_FoodB` não muda | D |
| N-12 | luta não remove a checadora (4.3) | ninho preto, mesma célula: vermelha T 0x92 M 0 num índice **menor** (fica em `_LifeB` no load) e preta T 0x12 M 13 (descanso) num índice maior | no primeiro tick a vermelha vira nuvem (T 0x70 ou 0xF0, M 10, S = vencedor) e a preta continua; ao fim (≈ 16 ticks) há 2 pretas (a preta venceu) ou 1 preta e 1 vermelha | D na estrutura, E no vencedor (≈ 50 %) |
| N-13 | rainha atacada (3.2) | vermelha T 0x92 M 7 acrescentada no fim da lista B (índice 5), na célula da cabeça da rainha preta (34, 9) | no primeiro tick a cabeça vira nuvem de luta e a cauda (índice 1) some no mesmo tick; `_BlkQueens` = 0; a nuvem nunca volta a ser rainha; no próximo `_CountAnts` (tick ≡ 0 ou 64 mod 128) o Quick Game termina com derrota | D |
| N-14 | passagem (7.7) | célula 0x14 em `_MapB` e `_MapR` em (20, 60), operária vermelha M 4 vizinha a ela no ninho vermelho | ao entrar em (20, 60) ela some da lista R e aparece na lista B com T 0x90–0x97, M 3 | D (exceto direção) |
| N-15 | trofalaxia (4.1) | amarela no ninho preto com `_MeWantFood` = 1; vermelha invasora andando para a célula dela | a vermelha alimenta a amarela (o mesmo efeito de uma preta) | D |
| N-16 | port, tick a tick | QUICK2, semente S forçada (no port) | o port, rodando a partir do QUICK2 com qualquer semente, deve manter os invariantes: T ∈ tipos válidos, `Vida` coerente com as listas, `_FoodB` muda só nas rotinas da seção 6 | P |

### 13.3 Observado no original

Bancada QEMU, `python tools/caso_ant.py rodar tests/referencias/casos/ID.json` (2026-10-17). A bancada só para ~5 ticks depois do load (o roteiro espera 1 s depois de tirar a pausa), então os casos de poucos ticks conferem o que continua valendo alguns ticks depois. Estados em `tests/referencias/saves/` (`ID_carregado.ANT` = logo depois do load; `ID_Tnnn.ANT` = final).

| Id | Entrada efetiva | Observado | Resultado |
|---|---|---|---|
| N-03 | QUICK2 + operária T 0x12, M 2 em (32, 3) (B[5]) | parado no tick 84: B[5].T = 0; `_ListIndexA` = 3; A[2] com T & 0xF8 = 0x10, M = 2, S = 117, em (6, 33): saiu no tick 81 pelo buraco (3, 32) com S = 120 e andou 3 passos (S −1 por passo) | **confirmado** (§4.5, §4.6, §7.2; S = 120 com M = 2 e buraco do lado preto) |
| N-06 | QUICK2 sem A[0], `_HealthB` = 100, `_AlwaysHealthy` = 1, `_FoodB` = 0; operária T 0x1A, M 3 em (32, 3) | tick 100: `_FoodB` = 2; a operária com T 0x16 (grupo 2) | **confirmado**: uma carga rende duas unidades (§6.2, "comida dobrada") |
| N-09 | célula 0x02 com saída 0 cercada de 0x5D em (10, 40) nos dois ninhos; operária M 2 em cada uma | tick 86: `_ExitMapB`(10, 40) = 0xFF, `_ExitMapR`(10, 40) = 0; as duas com M = 4, paradas | **confirmado**: decremento sem teste no preto, com teste no vermelho (§7.2) |
| N-13 | QUICK2, amarela movida para (30, 5); vermelha T 0x92, M 7 em (34, 9) no fim da lista B | tick 128: B[1] (cauda) e B[5] (invasora) com T = 0, `_BlkQueens` = 0; B[0] (era a cabeça) com T = 0 e **M = 8**: a nuvem terminou com vencedor S = 0 (a "ocupante" 0 venceu) e o modo saiu de `_GetNewMode(0, 0)` = `_CasteModeTabB[0]` = 8; `_RAntsKilled` = 1. No tick 128 o `_CountAnts` recontou `_CastePopB[0..4]` e parou na mensagem 10010 (modal): `_CastePopB[5]` = 1 e `_IsGameOver` = 0 até alguém clicar | **confirmado** (§3.2, "rainha atacada": a rainha some e a cauda vai junto; o defeito de `_GetWinner(0, ...)` se confirma). O fim de jogo não é conferível sem clicar na mensagem |

Port: `engine/tests/regras_ninho.rs` parte de cada `ID_carregado.ANT`, roda o tick do port (só as regras do ninho ligadas) até o mesmo `GameTime` alvo, com 6 sementes S diferentes, e avalia as mesmas conferências do JSON: todas passam.

**Vetores de tick** (`tests/referencias/ticks/`, ticks 79–83 do snapshot `quick`, ver [testes-tick.md](testes-tick.md)): para cada tick, partindo do `estado_antes`, o port acha uma semente S de entrada do ninho tal que `_DoAntSimB` + `_DoAntSimR` reproduzem **exatamente** mapas, mapas de saída, `_LifeB/R`, listas e contadores das colônias do `estado_depois` **e** o gerador S termina no valor do `extras.json` (k sorteios: 13, 12, 12, 12, 12; a semente de entrada é a final andada k passos para trás). O tick 80 inclui uma postura da rainha preta. Isso confirma a ordem e a quantidade de sorteios do ninho nesses ticks (morte natural, rainha, cauda, ovos) e que nada depois de `_DoAntSimR` sorteia S nesses ticks. Também confirma que `_LifeB/R` reconstruídos das listas batem com a memória do original.

## Implementação no port

- `engine/src/rules/ninho.rs`: `impl RegrasNinho for RegrasOriginais` chama `simular_ninho(e, cor, tabelas, tabelas_amarela)` (pública, para testes com outras tabelas). A formiga em processamento é um `Formigueiro` (índice = `_Tindex`, com T, X, Y, M de entrada, como os parâmetros do original); cada método cita a rotina do original e a seção desta spec.
- Tabelas em `data/ninho.json` (`TabelasNinho`), geradas por `tools/extrair_ninho.py` (endereços no topo do script): `_Dx8/_Dy8`, `_TurnTab`, `_ModeTabWB/SB`, `_CasteModeTabB`, `_CasteTabC`, `VitóriaR`, classe e matriz de luta, borda do buraco. Guardadas em `RegrasOriginais::ninho` (com mods); `TabelasNinho::padrao` é a cópia embutida, para testes.
- `_YellowFight` e `_DoTroph` são os de `rules/amarela.rs` (`lutar_com_amarela`, `trofalaxia`). Balões não estão implementados; o som da escavação (0x11/0x12) vira `Evento::Som` ([som.md](som.md)).
- `_DigTile*`, `_SmoothEdges*`, `_FixExitMap*`, `_MakeNewHole*`, `_PlaceEgg*`, `_GetNewMode*`, `_GetWinner`, `_EatFood*`/`_StealFood*`, `_DecEat*`, `_GetBestDir` e `_ExitHole` estão em `engine/src/rules/comum.rs` (ver [../../decisoes.md](../../decisoes.md), "Módulo comum das regras"); `vencedor_da_luta` continua pública em `ninho.rs`, com as tabelas do ninho.

## Lacunas

- Gotas e poças da superfície (`_PlaceDrop`, `_InitWater`): só a ordem dos sorteios está aqui; regra completa em `superficie.md`.
- `_ExitHole` com a lista A cheia: compacta e, se continuar cheia, grava na vaga 1000 (fora da lista) [a conferir na desmontagem].
- Testados na bancada: N-03, N-06, N-09, N-13 (§13.3). Faltam os estatísticos (N-01, N-07, N-10, N-12 no vencedor) e os que precisam de saves com ninhos mais desenvolvidos que o `QUICK2.ANT` (N-14, N-15). O executor de casos só para ~5 ticks depois do load; casos de 1 tick exigem o modo tick da bancada (ver testes-tick.md).
- `_YellowFight`, `_DoTroph`: ligados (teste `engine/tests/regras_comum.rs`), mas os casos com a amarela por perto (N-15) não foram rodados no original.
- Os "defeitos" marcados ([hipótese]) devem ser confirmados no original antes de decidir se o port os reproduz por padrão (o princípio do projeto é reproduzir; um mod pode corrigir).
