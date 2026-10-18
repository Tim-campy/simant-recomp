# Formiga amarela e controles do jogador

Este documento especifica, para reimplementação 1:1, a **formiga amarela** (o corpo do jogador): estado, presença no mapa, o que acontece com ela a cada tick, como anda até o destino clicado, como acha caminho, entra e sai dos ninhos, pega e larga objetos, come, luta, recruta, troca de corpo, funda uma colônia, morre e reencarna. No fim, os **controles**: como cliques, teclas, menus e aceleradores viram ordens. A formiga amarela também pode controlar a aranha; a parte da aranha está em [inimigos.md](inimigos.md#27-aranha-controlada-pelo-jogador-_memode--1). O tick está em [simulacao-visao-geral.md](simulacao-visao-geral.md) (passos 17 e 20); as formigas comuns em [superficie.md](superficie.md); a criação da amarela no novo jogo em [geracao-mundo.md](geracao-mundo.md) (3.D).

Convenção de evidência e notação: iguais às de [superficie.md](superficie.md#convenção-de-evidência). "Fato verificado no `QUICK2.ANT`" usa os offsets de [save-ant.md](../formatos/save-ant.md). Nenhum código decompilado é reproduzido.

## 1. Estado: as variáveis `_Me*`

**Fato visto no código** (endereços do `symbols.tsv`; todas estão no save). Valores do `QUICK2.ANT` (tick 78, Quick Game) **verificados**.

| Variável | Endereço | Significado | `QUICK2.ANT` |
|---|---|---|---|
| `_MePlane` | `10:ce80` | plano: 0 vista do quintal, 1 superfície, 2 ninho preto, 3 ninho vermelho | 2 |
| `_MeLocX`, `_MeLocY` | `10:cd88`, `10:ce7e` | célula | 34, 9 |
| `_MeType` | `10:cc84` | grupo × 8 **sem** cor e sem direção: 0x10 operária, 0x30 soldado, 0x40 rainha alada, 0x60 rainha; +8 = carregando (0x08 ovo, 0x18/0x38 comida, 0x28/0x48 terra) | 0x10 |
| `_MeDir` | `10:cf50` | direção 0–7 | 2 |
| `_MeColor` | `10:ce98` | 0 = preta, 0x80 = vermelha (byte alto de `_grndTile`+2 no pseudocódigo) | 0 |
| `_MeHealth` | `10:ac8a` | vida 0–100 | 99 |
| `_MeHealthCntDwn` | `9:9cf0` | ticks com vida 0 (morre ao passar de 99) | 0 |
| `_MeWarnHealth` | `9:9bec` | limiar de fome escolhido no controle `editHealthY` (padrão 30, `_InitSimVars`) | 30 |
| `_MeWantFood` | `9:9af2` | 1 = com fome (vida ≤ `_MeWarnHealth` ou ≤ 9) | — |
| `_MeEgg` | `10:ce92` | estágio de ovo exibido durante o renascimento | — |
| `_MeEggCarried` | `8:8a6a` | T do ovo carregado (0xFD quando nenhum) | — |
| `_MeFoodInType` | `8:8a6e` | o que foi comido no último pegar: −1 nada, 0 comida, 3 ovo | — |
| `_MeNestStarted` | `8:8a6c` | 1 = a colônia do jogador já foi fundada | 1 |
| `_MeBase` | `10:cf54` | zerado na troca de corpo [uso em aberto] | 0 |
| `_MeMode` | `9:9fe8` | 0 = formiga, 1 = **aranha**, 2 = modo Experimental (sem corpo) | 0 |
| `_MeSMode` | `9:7d60` | modo de caça da aranha (ver [inimigos.md](inimigos.md)) | 0 |
| `_MeMoveMe` | `9:7d24` | 1 = há uma ordem em andamento | 0 |
| `_MeCmd` | `9:9bc4` | ordem: 0 ir, 1 ir e pegar/largar, 2 ir cavando, 3 atacar o alvo, 4 ser alimentada pelo alvo | 0 |
| `_MeGoalPlane`, `_MeGoalX`, `_MeGoalY` | `9:9be0`, `9:80c6`, `9:80d2` | destino da ordem | — |
| `_MeLastX`, `_MeLastY` | `9:a0d6`, `9:a0da` | célula anterior (não volta para ela ao contornar) | — |
| `_MeSteps` | `9:9c7e` | passos dados (a paridade cobra a vida) | — |
| `_MeDis`, `_MePrevDis` | `9:9fe2`, `9:80aa` | distância ao buraco ao pegar comida [uso em aberto] | — |
| `_MeCrazyCnt`, `_MeCrazyDir`, `_MeCrazyRot` | `9:72e4`, `9:a0d8`, `9:78a4` | estado do contorno de obstáculos (5.3) | −2, —, — |
| `_MeTargIndex`, `_MeTargLife`, `_MeTargLifePlane`, `_MeTargLifeX/Y` | `9:99e4`, `9:72ee`, `9:9c6c`, `9:99e8/99ea` | formiga-alvo de uma ordem 3 ou 4 | — |
| `_MeStartedFight` | `9:9e7c` | 1 enquanto a luta foi iniciada pela amarela (escolhe a mensagem de morte) | — |
| `_MeDropAlarm` | `9:75fa` | 1 = soltando feromônio de alarme a cada passo | — |
| `_MeScent` | `10:ce88` | intensidade da trilha ao carregar comida (200 → 10) | 0 |
| `_FuzLocX/Y` | `9:9fe4/9fea` | posição "difusa" para as recrutadas ([superficie.md](superficie.md), 9.1) | 34, 0 |

## 2. Presença no mapa de vida

**Fato visto no código.** A amarela não está em lista nenhuma. Ela marca a célula com 0xFF; a rainha (`_MeType = 0x60`) marca também a célula atrás, na direção oposta (`_MeDir ^ 4`), com 0xFE.

- `_SetMyLife(plano, x, y, tipo, dir, v)` (`1020:6536`): grava `v` (e `v` ou 0xFE na cauda) e, se `v ≠ 0`, copia plano/posição/tipo/direção para as `_Me*`. Com `v` = 0 apaga sem mexer nas `_Me*`.
- `_ClearMyLife(plano, x, y, tipo, dir)` (`1020:631e`): apaga a célula só se ela contém 0xFF (e a cauda só se contém 0xFE).
- `_MoveMyLife(plano, x, y, tipo, dir)` (`1020:65fa`): `_ClearMyLife` na posição atual, grava 0xFF (e 0xFE) na nova e atualiza as `_Me*`. Plano 0 é tratado como 1.
- `_SetLife` (`1020:5d18`), usado por todas: num **ninho**, ao gravar um valor ≥ 1 numa célula de terra (0x20–0x2E) ou de superfície fechada (0x1C–0x1F), **escava** a célula (`_DigMyTile`) e toca um som. É assim que a amarela cava: basta entrar na terra (vale também para qualquer formiga posta no ninho por essa função).

## 3. A cada tick: `_DoAntSimY` (`1028:7cf6`)

**Fato visto no código.** Só age com `_MeMode = 0`:

```
_SetMyLife(_MePlane, _MeLoc, _MeType, _MeDir, 0xFF)          ← regrava a marca todo tick
se (_Cycle & 63) == 0: _SetMyHealth(_MeHealth − 1)
se _MePlane ≥ 2 e (t = tile do ninho na célula) > 0x4D: _SetMyHealth(_MeHealth − 1)   ← célula inundada
se _MeType == 0x60 e _MePlane ≥ 2 e (_Cycle & 15) == 0:                              ← rainha põe ovo
    p = _MeLoc + 2·D8[_MeDir ^ 4]
    se _InNestBounds(p) (0 ≤ x < 64, 0 < y < 64) e S128() ≤ _MeHealth:   ← S128 só dentro dos limites
        _PlaceEggB(p, 1) e _DecEatB()   (ou R, conforme _MeColor)
        _SetMyHealth(_MeHealth − 5)
se _MeHealth < 1:
    _MeHealthCntDwn += 1
    se _MeHealthCntDwn > 99: _YellowDeath(7 se está num ninho com t ≥ 0x4E, senão 8)   ← afogada / de fome
```

`_SetMyHealth(v)` (`1020:8c70`): se `_NeverHungry` (`8:8a5e`) ≠ 0, v = 100; se v > 0, `_MeHealthCntDwn = 0`; limita v a 0..100; `_MeHealth = v`; `_MeWantFood = 0` se `v > _MeWarnHealth` e `v > 9`, senão 1.

## 4. Andar por ordem: `_DoAntMoveY` (`1028:75f4`)

Chamado no passo 20 do tick. Também roda com o jogo pausado se `_MeMoveMe ≠ 0` e nenhuma ferramenta especial estiver ativa ([simulacao-visao-geral.md](simulacao-visao-geral.md), seção 4). **Fato visto no código.**

```
se _MeMoveMe == 0: fim
r = 0                                          ← 0 = continua ; −1 = concluída ; −2 = fracassou
se _MeCmd ≥ 3 (alvo):
    (tx, ty, Tt) = posição e T atuais da vaga _MeTargIndex no plano _MeTargLifePlane
    se a vaga sumiu ou (Tt ^ _MeTargLife) & 0xF0 ≠ 0: r = −2 ; FIM
    se o alvo está em outro plano, ou a mais de 1 célula: destino = (tx, ty) ; segue como ordem de andar
    senão (vizinho): vira-se para ele ; r = −1 ; FIM
ANDAR:
    se _MeCmd == 1 e _MeGoalPlane == _MePlane:
        k = _GetDir(_MeLoc, destino) ; p = _MeLoc + D9[k] (limitado ao mapa)
        se p == destino: q = _TryMyDropOrLift(_MePlane, destino)
            se q ≠ 0: r = (q == 1 ? −1 : −2) ; FIM
    d = _GetMyDir(_MePlane, _MeLoc, _MeGoalPlane, destino)     (seção 5)
    se d < 0: r = d ; FIM
    _MeLast = _MeLoc ; _MoveMyLife(_MePlane, _MeLoc + D8[d], _MeType, d) ; _MeSteps += 1
    superfície:
        se _MeScent > 0 e carrega comida (_MeType 0x18 ou 0x38):
            deposita _MeScent na trilha da cor _MeColor (c = max) ; se _MeScent > 10: _MeScent −= 1
        se _MeDropAlarm: alarme = min(alarme + 50, 200)
    ninho: atualiza o mapa de saídas (_FixExitMapB/R)
    se _MeSteps ímpar:
        se rainha alada (0x40) sem colônia (_MeNestStarted = 0) e _HealthB > 0 e não _AlwaysHealthy: _HealthB −= 1
        _SetMyHealth(_MeHealth − 1)                            ← 1 de vida a cada 2 passos
    superfície e célula é buraco e (destino num ninho, ou destino é esta célula): ENTRA (6.1)
    ninho, linha y ≠ 0: se o tile do destino é 0x14 e não é rainha e destino == célula atual:
        troca de ninho (6.3) ; r = −1
    ninho, linha 0: SAI (6.2)
    depois de ENTRA/SAI: se o destino era exatamente esta célula/plano e o plano novo ≠ plano exibido:
        centraliza (se AutoTrack desligado) e r = −1 (a ordem termina; o código cai no rótulo de
        "concluída", `1028:7b6b`)  ← verificado no vetor amarela_superficie/1852
        (se o plano novo é o exibido, r continua 0 e a ordem segue, com destino ainda na superfície)
FIM:
    se "AutoTrack": centraliza a vista na amarela
    se r == 0 e chegou (plano e célula do destino) e _MeCmd == 0: r = −1
    se r ≠ 0:
        r == −2: som de erro e centraliza
        r == −1 e _MeCmd == 3: _MeStartedFight = 1 ; _YellowFight(_MeTargLifePlane, _MeTargIndex) ; _MeStartedFight = 0
        r == −1 e _MeCmd == 4: vira o alvo para a amarela (exceto rainhas) e _EatMyFood(1)
        _ResetYellowVars(_MePlane, _MeLoc)                       ← encerra a ordem
        casa e superfície e tile sob a amarela = 0x76 ou 0x78: _YellowDeath(10)   ← choque elétrico
```

`_ResetYellowVars(plano, x, y)` (`1000:72ba`): `_MePlane` e `_MeLoc` = argumentos (correção: a versão anterior desta spec omitia), destino = posição, `_MeLast` = posição, `_MeCmd = 0`, `_MeMoveMe = 0`, `_MeDis = _MePrevDis = _MeSteps = 0`, `_MeCrazyCnt = −2`.

`D9` = `_Dx9/_Dy9` (índice 0 = parado). Sem sorteio em todo este fluxo, exceto o `S8()` de `_ExitNest` (6.2) e os sorteios das ações chamadas.

## 5. Caminho: `_GetMyDir` e companhia

### 5.1 Alvo intermediário — `_GetMyDir(plano, x, y, plano_dest, dx, dy)` (`1028:8eca`)

**Fato visto no código.** Primeiro escolhe para onde andar agora:

| Onde está | Destino | Alvo imediato |
|---|---|---|
| superfície (plano < 2) | superfície | o destino |
| superfície | ninho preto | `_LastNewHoleB` (`8:835a`, último buraco preto criado) |
| superfície | ninho vermelho | `_LastNewHoleR` (`8:835e`) |
| ninho | o mesmo ninho | o destino |
| ninho preto | outro plano | `_LastNewExitB` (`8:8352`: coluna do último buraco preto, linha 0) |
| ninho vermelho | outro plano | `_LastNewExitR` (`8:8356`) |

Depois aplica a máquina de contorno (5.3) com esse alvo.

### 5.2 Passo guloso — `_GetMyBestDirs(plano, x, y, tx, ty)` (`1028:8828`)

Mesma regra de `_GetRedBestDirs` ([superficie.md](superficie.md), 9.2), com o teste de passagem `_TileCanBeMovedOn(plano, vizinha, _MeGoalPlane, _MeGoalX, _MeGoalY, cavar = (_MeCmd == 2))` (5.4): −1 se já está no alvo; senão a melhor vizinha livre que reduz a distância (com o limiar caindo a cada melhora), ou a melhor bloqueada por formiga/tile não-livre, ou −2 se nenhuma vizinha passável reduz a distância. Sem sorteio.

`_CheckMyBestDirs(&n, plano, x, y, tx, ty)` (`1028:8b40`) simula o guloso sem mexer em nada: aplica `_GetMyBestDirs` a partir da posição, até 64 passos. Devolve −2 se a simulação empacou (mínimo local), −1 se chegou ou se completou 64 passos.

### 5.3 Contorno de obstáculos — `_MeCrazyCnt`, `_MeCrazyDir`, `_MeCrazyRot`

**Fato visto no código.** Cada clique de destino põe `_MeCrazyCnt = −2`.

```
se _MeCrazyCnt < 0:                                          (modo direto)
    d = _GetMyBestDirs(alvo)
    se d == −2 e _MeCrazyCnt == −2:                            ← só na primeira trava depois do clique
        _MeCrazyDir = _GetDir(posição, alvo) − 1 ; _MeCrazyCnt = 16 ; _MeCrazyRot = 0
        d = _GetMyRandDirs(...)
    devolve d
senão:                                                       (contornando)
    se _CheckMyBestDirs(alvo) == −2: d = _GetMyRandDirs(...)
    senão: _MeCrazyCnt = −1 ; d = _GetMyBestDirs(alvo)
    _MeCrazyCnt −= 1
    devolve d
```

Consequência: o contorno dura no máximo 17 passos; se o guloso voltar a ter caminho livre, o contador vai a −2 e um novo contorno pode começar. Se o contorno terminar e o guloso ainda travar, `_MeCrazyCnt = −1` e `_GetMyBestDirs` devolve −2: a ordem fracassa (som de erro).

`_GetMyRandDirs(&rot, &dirc, plano, x, y, tx, ty)` (`1028:8928`), sem sorteio:

```
dist = _GetDis(posição, alvo) ; se dist ≤ 0: devolve −1
ok[k] = vizinha k passável (5.4, com cavar = (_MeCmd == 2)) e ≠ (_MeLastX, _MeLastY)
se nenhuma ok: devolve −2
a = b = dirc
se rot == 0:                                       ← primeiro passo do contorno: escolhe o lado
    para j = 0..7:
        se ok[a]: dirc = _GetDir(posição, alvo) − 1 ; rot = +1 ; devolve a
        se ok[b]: dirc = _GetDir(posição, alvo) − 1 ; rot = −1 ; devolve b
        a = (a + 1) & 7 ; b = (b − 1) & 7
senão:
    para j = 0..7:
        c = (rot ≥ 1 ? a : b)
        se ok[c]:
            se _GetDis(posição + D8[c], alvo) > dist: devolve c       ← ainda se afastando: mantém o lado
            dirc = _GetDir(posição, alvo) − 1 ; rot = 0 ; devolve c      ← melhorou: recomeça
        a = (a + 1) & 7 ; b = (b − 1) & 7
devolve −1
```

`_GetMyBestDir`, `_GetMyNextRandDirs` e `_GetMyInitialRandDir` (`1028:8d3a`, `8bea`, `8cde`) repetem partes dessa lógica e não têm chamador; `_GetMyDis` (`1028:8682`) também não.

### 5.4 Onde a amarela pode pisar — `_TileCanBeMovedOn` (`1020:9342`)

**Fato visto no código.**

- Superfície: dentro do mapa e tile ≤ 0x53 no quintal (chão, objetos, comida, buraco e pedrinhas; pedras 0x54+ e a lagarta/bicho-de-conta não) ou ≤ 0x90 na casa.
- Ninho: dentro do mapa e: tile < 0x19 ou pedrinha 0x30–0x31 (passável sem cavar); ou, com `cavar`, terra 0x20–0x2E ou superfície fechada 0x1C–0x1F (passável cavando). Nas linhas y ≥ 2, ou se o destino é outro plano, isso basta. Com o destino neste ninho, as linhas 0 e 1 têm restrições:

| Linha | Sem `cavar` | Com `cavar`, célula a cavar | Com `cavar`, célula já aberta |
|---|---|---|---|
| y = 1 | só na coluna do destino, ou se o destino não está na linha 0 | só na coluna do destino | sim |
| y = 0 | só se for exatamente o destino | só na coluna do destino e se a célula de baixo (linha 1) não for terra | só se for exatamente o destino |

## 6. Ninho: entrar, sair, trocar de ninho

### 6.1 Entrar (embutido em `_DoAntMoveY`; igual a `_EnterNest` `1028:8408`)

**Fato visto no código.** Música do ninho (`_TryAntTheme`); se soltava alarme, desliga (`_SetAlarmDropState(0, 1)`); apaga a marca; `_MePlane = 2` se `_MeLocX < 65`, senão 3; `_MeLocX` ← a linha da superfície (`_MeLocY`), `_MeLocY` ← 2 se rainha (0x60), senão 1; `_MeDir = 4`; `_DigMyTile` na nova célula; marca 0xFF.

**Peculiaridade A1:** o limite é x < 65, enquanto as formigas comuns (`_GoInNest`) e a criação de buracos (`_CreateNewHole`) usam x < 64. Um buraco na coluna 64 pertence ao ninho vermelho, mas a amarela que entra nele vai para o ninho preto.

### 6.2 Sair: `_ExitNest` (`1028:84a4`)

**Fato visto no código.**

```
música ; apaga a marca
linha = _MeLocX & 63                         (coluna do ninho = linha da superfície)
se _HoleMap(cor do ninho)[linha] == 0: _MakeNewHoleB/R(_MeLocX)     ← abre um buraco novo
x0 = _HoleMap[linha] ; passo = (rainha ? 2 : 1)
direção base:
    destino na superfície: _GetDir((x0, linha), destino) − 1 (0 se coincidir)
    destino neste mesmo ninho: 2 se x0 < 64, senão 6
    outro: _GetMyDir(1, (x0, linha), destino) ; se < 0: S8()
para j = 0..7 (desvios _absSearchDirs `8:006c` = 0, +1, −1, +2, −2, +3, −3, +4):
    d = (base + desvio) & 7 ; p = (x0, linha) + passo·D8[d]
    se _IsNotObstacle(1, p): posição = p ; _MeDir = d ; para
se nenhuma: _MeDir = base ; posição = ((x0 + passo) & 127, linha)
_MePlane = 1 ; marca 0xFF
```

### 6.3 Passagem entre os ninhos (tile 0x14)

Num ninho, fora da linha 0, se a amarela (não-rainha) chega ao destino e o tile do destino é 0x14, ela passa para o **outro** ninho na mesma célula (`_MePlane` 2 ↔ 3), com som e centralização; entrando no vermelho, mostra a mensagem 4 de SHARED 4/1010 ("You have entered the red nest", parâmetro 360). Confirma a [hipótese] de [tiles-valores.md](tiles-valores.md) de que 0x14 é a passagem entre os ninhos.

## 7. Pegar, largar, cavar, comer e saúde

### 7.1 `_TryMyDropOrLift(plano, x, y)` (`1020:98de`)

**Fato visto no código.** Chamada quando a ordem 1 chega ao lado do destino, ou pelo comando 8.

```
se carrega algo (_MeType & 8):
    0x08 → _DropMyEgg ; 0x18/0x38 → _DropMyFood ; 0x28/0x48 → _DropMyRock ; outro → falha
    devolve 1 se largou, −1 se não
senão:
    ok = PEGA_OVO (se _MeType == 0x10 ou _MeHealth < 10)
    se não ok: ok = _PickupMyRock(plano, x, y)
    se não ok: ok = _PickupMyFood(plano, x, y)
    se ok:
        vira-se para (x, y) ; se _MeFoodInType ≥ 0: _EatMyFood(_MeFoodInType) ; _MeFoodInType = −1
        devolve 1
    se (_CommandKey (8:85e2) ≠ 0 ou (rainha alada 0x40 e _MeNestStarted = 0)) e superfície e (x, y) == posição:
        se _DigMyNewHole(x, y): _EnterNest() ; centraliza ; devolve 1
        senão devolve −1
    devolve 0
```

| Função | Endereço | Condição | Efeito |
|---|---|---|---|
| PEGA_OVO (embutido; igual a `_PickupMyEgg` `1020:8e96`) | — | operária vazia (0x10) ou vida < 10; ovo (T & 0x7F de 1 a 7) na célula (`_FindEggAt` `1020:88a2`: pelo mapa de vida, ou, se a célula tem 0xFF/0xFE ou nada, pela lista; devolve o T da vaga, conferido na desmontagem `1020:8a55`–`8a80`) | `_MeEggCarried = 0xFD`; remove o ovo da lista; se vida < 10: **come o ovo** (`_MeFoodInType = 3`); senão `_MeEggCarried = T do ovo`, `_MeType = 8`, som |
| `_PickupMyRock` | `1020:85ee` | 0x10 ou 0x30 vazia; pedrinha (superfície 0x51–0x53; ninho 0x30–0x31) | superfície: se a pedrinha tampava um buraco (a coluna/linha consta em `_HoleMap`), reabre (`_MapB/R` linha 0 = 0x18; superfície = 0x50 no quintal, `S1(7) + 0x59` na casa); senão o chão vira `S16()` (quintal) ou 0 (casa). Ninho: 0x30 → `S8()` (túnel); 0x31 (tampa) → 0x18 e o buraco da superfície volta a 0x50. `_MeType` → 0x28 (operária) ou 0x48 (soldado) |
| `_PickupMyFood` | `1020:8f4c` | comida na célula; se sem fome, só 0x10/0x30 vazia | superfície: `_PickupFoodA`; se vai carregar: `_MeScent = 200`, `_MeDis = _GetDis(célula, _LastNewHoleB)`, `_MePrevDis = _MeDis + 1`, deposita 200 na **trilha preta** (sempre `_JamScentBT`). Ninho: 0x10 → `S8()`, senão tile − 1; `_FoodB/R` −1. Com fome: `_MeFoodInType = 0` (come na hora); senão som e `_MeType += 8` |
| `_DropMyFood` | `1020:7576` | carrega comida | procura onde largar começando na direção do destino (`_GetDir − 1`, ou `_MeDir`) com os desvios `_absSearchDirs`; serve uma célula sem formiga que seja comida com menos de 3 unidades (+1), ou livre (`_IsClearTile`), ou o centro de uma formiga-leão (0x38); se nenhuma vizinha serve, tenta a própria célula. Superfície: `_DropFoodA`; ninho: grava o tile (0x10 numa célula livre) e `_FoodB/R` +1. `_MeType &= ~8`, som |
| `_DropMyRock` | `1020:7c52` | carrega terra/pedrinha (0x28/0x48) | mesma busca; serve célula sem formiga e livre, ou que não seja comida nem pedrinha mas seja buraco ou 0x38. `_DropPebble`: na superfície grava 0x51 e, se a célula é um buraco, tampa a abertura no ninho (linha 0 = 0x31); no ninho grava 0x30, ou 0x31 numa abertura (e 0x51 no buraco da superfície). `_MeType −= 0x18`, som |
| `_DropMyEgg` | `1020:8188` | carrega ovo (0x08) | mesma busca (célula sem formiga e livre, ou 0x38); devolve o ovo à lista do plano com `T = _MeEggCarried`, `M = 8`, `S = 0`; `_MeType = 0x10`, `_MeEggCarried = 0xFD` |
| `_DigMyNewHole` | `1020:16ae` | rainha alada sem colônia (ou `_CommandKey`), na própria célula, 1 ≤ x < 128, 1 ≤ y < 64 | quintal: exige 3 × 3 livre; casa: tile < 200. `_CreateNewHole` grava o buraco (0x50 e borda 0x18–0x1F; 0x59 na casa), atualiza `_HoleMap`, `_LastNewHole*` e `_LastNewExit*`, escava a linha 1 do ninho |

`_PickupMyObject` (`1020:9250`) e `_DropMyObject` (`1020:91ec`) agrupam as mesmas regras (só `_DropMyObject` é chamada, por `_YellowDeath`). `_IsLiftable` (`1020:97ca`) decide se um duplo clique no ninho vira "pegar" (comida, pedrinha ou ovo na célula).

### 7.2 Comer: `_EatMyFood(tipo)` (`1020:8cda`)

**Fato visto no código.** Mensagens de SHARED 4/1010 (`_WindPromptStrs`, `9:737a`).

| tipo | Quando | Efeito |
|---|---|---|
| 0 | comeu comida que pegou (`_MeFoodInType = 0`) | som; se `_MeHealth > 0` e `_BpopT > 1`: `_HealthB += _MeHealth / (_BpopT − 1)` (limitado a 100); `_MeHealth = 100` |
| 1 | foi alimentada por uma formiga (trofalaxia, `_DoTroph` e ordem 4) | diálogo; mensagem 11 "You feel much better now!"; `_MeHealth = 100` (sem bônus à colônia) |
| 2 | [sem chamador visto] | mensagem 12 "Thank you for your contribution"; se `_MeHealth ≥ 11`: `_MeHealth −= 10` |
| 3 | comeu um ovo (vida < 10) | sons; mensagem 13 "Cannibalism has restored your health" (parâmetro 180); mesmo efeito do tipo 0 |

Em todos: `_MeHealthCntDwn = 0` e `_MeWantFood` recalculado como em `_SetMyHealth`.

**Trofalaxia** (`_DoTroph` `1000:846e`): quando uma formiga da mesma cor tenta entrar na célula da amarela com `_MeWantFood` ligado (modos 0, 1/4, 2 e 6, ver [superficie.md](superficie.md), 2.3), a amarela se vira para ela (`_MoveMyLife` na mesma célula) e chama `_EatMyFood(1)`.

### 7.3 Alarme: `_SetAlarmDropState` (`1000:84cc`) e comando 7

Ligar só é possível na superfície e como formiga (`_MeMode = 0`); liga `_MeDropAlarm`, põe `_ELayerMode = 0` e seleciona o botão `editScent` da janela de edição. A cada passo na superfície, +50 de alarme (máximo 200). Entrar num ninho, morrer ou reiniciar a amarela desliga (`_MeDropAlarm = 0`, `_ELayerMode = −1`). Som 0xF (ok) ou 1 (erro) [código: `_YellowCommandKey` caso 7 `1000:8562`, `_SetAlarmDropState` `1000:84cc`, `_EnterNest`, `_YellowDeath`, `_InitYellow`].

**O que se vê** [código]: com `_ELayerMode = 0`, a janela de edição desenha, na superfície, o feromônio de alarme no lugar do chão (`_DoCalcTile`: valor > 0x10 → tiles 0xF1–0xFF, pontos amarelos a vermelhos; jogo.md, "Camada de cheiro na janela de edição"). Logo depois de apertar 0 nada muda, porque ainda não há alarme: os pontos aparecem atrás da amarela quando ela anda. As formigas que forrageiam e pisam numa célula com alarme passam ao modo Alarmed e sobem o gradiente; as de Defense também o seguem ([feromonios.md](feromonios.md) §4). O alarme se espalha e enfraquece a cada 64 ticks (`_SmoothAlarm`). **No port** (2026-10-18): a tecla 0 já ligava o alarme e o depósito (+50 por passo), mas a janela de edição não mostrava a camada, e o usuário "não percebia nada de diferente"; agora `Estado::camada_edicao` (`_ELayerMode`) é ligado aqui e pelos botões de cheiro da faixa, e a tela de jogo o desenha. **Conferido na bancada** (`qemu_alarme.txt`): `_ELayerMode` = 0 depois da tecla 0 e o rastro amarelo na janela de edição depois de andar (jogo.md, "Camada de cheiro na janela de edição").

## 8. Troca de corpo e alvo: `_ExchangeLives`, `_DoLifeExchange`

### 8.1 Modos de ferramenta

**Fato visto no código.** `_CurGameTool` (`9:7c22`): −1 nenhuma, 10 troca de corpo, 11 alvo. `_StartLifeTransfer` (`1020:6744`) e `_TargetAnt` (`1020:66dc`) alternam a ferramenta: ao ligar, guardam `_GamePaused` em `_SaveGamePaused` (`9:9ea2`) e **pausam o jogo**; ao desligar (ou por `_EndLifeTransferMode`/`_EndTargetMode`), restauram a pausa anterior. A mensagem 3 de 4/1010 ("Click on ant to exchange bodies") e a 17 ("Click on ant to chase or UnPause") acompanham esses modos [hipótese: mostradas pela pausa].

### 8.2 `_ExchangeLives(plano, x, y)` (`1020:67ac`)

Põe `_LessonTemp` (`9:9fb0`) = 1 e chama `_DoLifeExchange`. Sucesso: desliga a ferramenta (restaura a pausa); se o novo corpo é a rainha, som e música especiais. Fracasso: som de erro.

### 8.3 `_DoLifeExchange(plano, x, y)` (`1020:6842`)

**Fato visto no código.** `v` = valor do mapa de vida na célula (−1 se vazia).

1. **Célula vazia, na superfície, a menos de ~1,4 célula da aranha** (`_GetDis((x·16 + 8, y·16 + 8), (_SpidX, _SpidY)) < 512`): vira **aranha**. Se já é aranha, nada. Senão devolve o corpo atual à lista como formiga comum (2.1 abaixo), apaga a marca, `_Starg = −2`, destino da aranha = célula da aranha, `_MeMode = 1`, `_SMode = _MeSMode = 0`, `_MeColor = 0`, `_ResetYellowVars(1, célula da aranha)`, `_MeHealth = 100`, `_MeHealthCntDwn = 0`, `_MeWantFood` conforme `_MeWarnHealth`.
2. **Clicou em si mesma** (0xFF/0xFE, ou, sendo aranha, perto da aranha): devolve 1 sem fazer nada.
3. **Rainha:** se clicou na cauda (grupo 13), a cabeça é a célula `+ D8[dir]`; se na cabeça (grupo 12), a cauda é `+ D8[dir ^ 4]`. Ambas são tratadas.
4. Procura a vaga na lista do plano (de trás para frente) com X, Y e T exatos (e a da cauda). Não achou → 0.
5. Recusa (devolve 0): alvo vermelho; ovo (grupo 0); grupos 10, 11, 14, 15.
6. Ajustes do alvo: carregando terra (grupos 5/9) → perde a terra (`_UnCarryCaste`); carregando ovo (grupo 1) → `_MeEggCarried` = S da vaga (estágio do ovo).
7. Se era formiga (`_MeMode = 0`): o corpo antigo vira formiga comum no plano/posição atuais: `T = grupo' << 3 | cor do alvo | _MeDir`, com grupo' = grupo de `_MeType` (carregando terra → sem terra; carregando ovo → S = `_MeEggCarried`), `M = 9` se era rainha (0x60) senão 0, e a cauda (`T + 8`) se rainha. Se a lista está cheia (A ≥ 1000, B/R ≥ 500), devolve 0 **depois** de ter removido o alvo (peculiaridade A2). Se era aranha, a aranha só volta a ser controlada pela simulação.
8. As vagas do alvo (e da cauda) são zeradas na lista.
9. Novo corpo: `_MeColor = cor do alvo`, `_MeBase = 0`, `_MeMode = _MeSMode = 0`, `_MeHealth = 100`, `_MeHealthCntDwn = 0`, `_MeWantFood` recalculado; marca 0xFF (e 0xFE na cauda); `_MeLoc` = célula da cabeça; `_MeDir = T & 7`; `_MeType = T & 0x78`; `_MePlane = plano`.

## 9. Morte: `_YellowDeath` (`1000:7746`)

### 9.1 Causas

**Fato visto no código; textos de SHARED 4 (ids decimais).** "Events" é a opção do menu Options que mostra os quadros (`_OptionStates[3]`, `8:85f8`). Os casos 0, 1, 2, 5, 6, 7, 8 e 9 tocam uma música `0x4E27 + S1(5)` (um sorteio); 3 e 4 só um som de erro; 10 nenhuma música.

| Código | Quem chama | Mensagem (se "Events") | Contador |
|---|---|---|---|
| 0 | `_YellowFight` (perdeu) | 10030 "Your brave attack…" se a amarela atacou (`_MeStartedFight`), senão 10032 "The enemy ant has attacked you!…" | — (já contado em `_GetWinner`) |
| 1 | aranha a comeu | quadro da aranha (`_SpiderDialog`) | `_BAntsEaten` |
| 2 | formiga-leão | quadro da formiga-leão (`_LionDialog`) | `_BAntsEaten` |
| 3 | jogador-aranha cercado | 10026 "You have been overcome by the massive onslaught of ants!…" | `_BAntsKilled` |
| 4 | jogador-aranha saiu do mapa | 10028 "Due to a latent fear of being eaten by ants, you go insane…" | `_BAntsKilled` |
| 5 | pisada (`_FootFall`) | 10034 "The inconsiderate human has stepped on you!…" | `_BAntsKilled` |
| 6 | cortador (`_MowerFall`, `_SimKidOutside`) | 10036 "The evil lawn mower…" | `_BAntsKilled` |
| 7 | afogada (vida 0 em célula inundada) | 10038 "You swim for your life…" | `_BAntsKilled` |
| 8 | fome | 10040 "After going berserk with hunger…" | `_BAntsExpired` |
| 9 | inseticida (`_SimKidInside`) | 10042 "You are covered with insecticide!…" | `_BAntsKilled` |
| 10 | choque elétrico (tile 0x76/0x78 na casa) | 10045 "You have been electrocuted." | `_BAntsKilled` |

Ordem: para a simulação, encerra as ferramentas 10/11, e, se carregava algo, tenta largar (`_DropMyObject` na própria célula); se não conseguir, perde a carga. Animações que **consomem sorteios** e dependem da interface: 7, 8 e 9 → `_AnimYellowInsane`, **antes** do sorteio da música (gerador R, `R(7)` ao chegar a cada alvo, e `S8()` quando trava; 32 quadros com a janela de edição à frente, 8 senão); 1, 2 e 10 → `_AnimYellowFight` (10.2). Depois (1 e 2) o quadro "Your ant has been killed" (4/1010 nº 23). Em todos os casos: `_UnRecruit(1)` (libera todas as recrutadas) e desliga o alarme.

### 9.2 Reencarnação

**Fato visto no código.** A busca por um novo corpo, nesta ordem:

1. **Ovo preto no ninho preto**: lista B do índice `_ListIndexB` (inclusive; peculiaridade A3) até 0, primeiro T com 1 ≤ T ≤ 7. Remove o ovo e renasce como ovo (9.3, modo 0).
2. **Adulto preto no ninho preto**: mesma varredura, T de 8 a 0x67 (inclui a cabeça da rainha, 0x60–0x67).
3. **Adulto preto na superfície**: lista A, de `_ListIndexA` (inclusive) até 0, T de 8 a 0x67.
4. **Adulto preto no ninho vermelho**: lista R, idem.

Num adulto (2–4): a vaga é zerada e a amarela assume a posição com o **tipo do adulto** (`T & 0xF8`) e a direção antiga; a marca gravada no mapa de vida é o próprio T do adulto, não 0xFF (peculiaridade A4: corrigida no tick seguinte por `_DoAntSimY`). `_MeMode = 0`, ordem zerada (`_MeCrazyCnt = −2`), vida 100, mensagem 10059 "You have been transferred to an existing ant.".

Sem corpo: Tutorial e Quick → **derrota** (`_IsGameOver = 1`, `_BlackWon = 0`). Jogo Completo → se há mais de uma colônia preta (`_ColonyTotalBlack > 1`), mensagem 10058 e `_SpecialXfer` (o jogador clica numa colônia preta do mapa do quintal); senão derrota com 10056.

### 9.3 Nascimento: `_YellowBirth(plano, x, y, tipo, rainha)` (`1000:739e`)

**Fato visto no código.** Para a simulação durante a animação (tempo real, não ticks).

- **Renascer como ovo** (`rainha = 0`, chamado pela reencarnação 9.2.1 com `tipo = _MeType`, e 0x10 se era rainha): a amarela aparece como ovo na célula do ovo (`_MeEgg` 1 → 7, um estágio a cada ~10 ticks do relógio do Mac), depois vira `tipo` virada para 2 (se era rainha) ou 6. Na prática sempre 6: `_YellowDeath` já trocou 0x60 por 0x10 antes de chamar (fato visto no código; ver divergencias.md). Mensagem 24 "You have been reborn!".
- **Fundar a colônia** (`rainha = 1`, comando 9): a amarela vira rainha (0x60) na própria célula, um ovo aparece em `x, y` (2 células atrás) e passa pelos estágios; no fim, a rainha vira uma rainha comum da simulação (`_MakeBlkQueen`/`_MakeRedQueen`, `_CastePop[5] += 1`) e a amarela renasce como **operária** (0x10) na célula do ovo, virada para trás; `_CastePop[1] += 1`, `_CastePop[4] −= 1`, `_BpopT/_RpopT += 1`. Mensagem 25 "You are now a worker!".
- Em ambos: destino = posição, ordem zerada, `_MeNestStarted = 1`, vida 100.

## 10. Luta e comandos de colônia

### 10.1 `_YellowFight(plano, i)` (`1028:823e`)

**Fato visto no código.** Chamado quando uma inimiga tenta entrar na célula da amarela, ou ao concluir a ordem 3.

```
apaga a inimiga do mapa de vida
v = _GetWinner(_MeType, T[i])               ← a amarela é "a" (a da célula); R(10) ; _MeType não tem bit de cor
se v ≠ _MeType: centraliza
_AnimYellowFight(_MePlane, _MeLoc, _MeDir, 0x70, 0)          (10.2)
se a amarela venceu:
    marca 0xFF ; superfície: _DeadAntHere(célula da inimiga, cor dela) ; T[i] = 0
    se era o alvo da ordem 3: encerra a ordem
senão:
    devolve a inimiga ao mapa de vida ; superfície: _DeadAntHere(célula da amarela, _MeColor)
    _YellowDeath(0)
```

Como `_MeType` não tem cor, com `_Invincible = 1` a amarela sempre vence (a regra de `_GetWinner` faz a preta vencer). Os contadores de baixa seguem `_GetWinner` ([superficie.md](superficie.md), 6.2).

### 10.2 `_AnimYellowFight` (`1028:7e56`): animação que consome sorteios

**Fato visto no código.** Quadros: 6 na luta (código 0); 32 na morte por eletricidade (10); 8 nas mortes 1/2 com a janela de edição atrás; 32 (música desligada) ou 64 (música ligada, `_OptionStates[1]`) com a janela à frente. Antes do laço: `_SetMyLife(…, 0xFF)` e `_DoEditUpdateDraw` (a amarela ainda com o seu tipo; na luta a inimiga já saiu do mapa de vida). Cada quadro começa esperando o relógio do Mac passar de "início do quadro anterior + 6" (3 degraus de 55 ms = 165 ms; o primeiro quadro espera só o próximo degrau) e então: `S2()` e som 0x25 se ≠ 0 (só na luta); som 0x31 e `S2()` que pisca o tile (só no choque); com música ligada e código 1–9, **sai se `_mySongIsDone()`**; na morte por formiga-leão, o centro da armadilha anima; **sempre** `S8()` (direção), `_MoveMyLife(plano, x, y, tipo, dir)` e `_DoEditUpdateDraw` (`_UpdateEdit` → `UPDATEWINDOW`: o desenho é síncrono, sem passar pelo laço de mensagens). Na luta o tipo é 0x70: a marca 0xFF é desenhada com o tile 0x170 + direção do buffer de vida, a "nuvem" de formiga amarela e vermelha embolada. Não há laço de mensagens durante a animação: **o programa inteiro fica parado** (~0,8 s na luta, ~5,3 s no choque).

`_mySongIsDone` (`1008:a1a0`) devolve sempre verdadeiro no caminho multimídia (MCI) do Windows ([som.md](som.md)): com música ligada as mortes 1–9 saem no primeiro quadro, depois do som e antes do `S8()` — nenhum quadro desenhado. Só no caminho Sound Blaster os 64 quadros correriam.

**Verificado no original (bancada QEMU, 2026-10-18, `cenarios_tick.py gravar luta_quadros`, snapshot `rico`):** amarela soldado posta ao lado de uma operária vermelha parada e duplo clique nela (ordem 3); no tick 1799 a luta acontece e a tela foi capturada parada logo depois de cada `_DoEditUpdateDraw` (breakpoints em `1028:7e86`, `1028:8019` e `1028:7f1c`, ver `tools/cenarios_tick.py`). Foram **7 desenhos**: o de antes do laço (amarela soldado, `_MeType` 0x30, a vermelha já fora do mapa de vida) e 6 quadros com `_MeType` 0x70 e `_MeDir` 0, 2, 5, 6, 2, 0, na mesma célula. A imagem é a nuvem amarela e vermelha, girando a cada quadro. Capturas em `re_trabalho/a51/luta_quadros/` (fora do git, imagens do jogo).

**Port.** `animar_luta` (`rules/amarela.rs`) guarda, depois de cada `_MoveMyLife`, o mapa de chão, o mapa de vida do plano e os campos `_Me*` do desenho (`QuadroAnimacao`) e pede `Evento::Animacao` (antes das esperas e dos sons do quadro). A interface (`core/partida.rs`) para a simulação por `espera_ms × (quadros − 1)` ms (`bloqueio_ms`; nenhum tick roda, o relógio de parede anda) e a janela de edição mostra o quadro da vez (`Partida::quadro_animado`, `TelaDeJogo::mostrar_quadro_animado`); um quadro modal pedido no mesmo tick (a morte depois de uma luta perdida) só aparece depois. Entre os quadros, `Evento::Espera(165)` continua espalhando os sons (regras/som.md). `_AnimYellowInsane` (§9.1, mortes 7–9) faz o mesmo com 55 ms por quadro (`espera_loucura_ms`). O desenho de antes do laço não é mostrado (dura até 55 ms). Com música ligada as mortes 1–9 saem no primeiro quadro (`animacao.morte_para_com_musica`, padrão verdadeiro). Os ticks pedidos pelos mods (`avancar_ticks`) não param. A animação só lê o estado: os vetores de tick do original (`ticks_originais`, `ticks_longos`) continuam iguais. Conferido com o mesmo estado de entrada (`estado_antes.bin` do tick 1799 carregado como save; `simant --save … --ticks 1 --avancar-ms 0/200/400/600/800 --exportar-tela-jogo`): os quadros do port mostram a mesma nuvem na mesma célula; as direções diferem porque o Load do port ressemeia S.

**Peculiaridade A5:** o número de sorteios depende da janela à frente e da opção de música. Para o determinismo do port, reproduzir a mesma regra (com o mesmo estado de interface) ou aceitar divergência a partir de uma morte ou luta da amarela.

### 10.3 Comandos: `_YellowCommand(c)` (`1000:87ba`)

**Fato visto no código.** Ignorado se `_MeMode > 1` (Experimental) ou se o jogo acabou.

| c | Efeito | Música/som |
|---|---|---|
| 0 | ajuda (`WinHelp`; tópico 1 formiga, 2 aranha, 3 rainha alada) | — |
| 1 | `_Recruit(5)` | 0x2B05 |
| 2 | `_Recruit(10)` | 0x2B06 |
| 3 | `_Recruit(1000)` (todas) | 0x2B07 |
| 4 | `_UnRecruit(0)` (libera metade) | 0x2B08 |
| 5 | `_UnRecruit(1)` (libera todas) | 0x2B09 |
| 6 | liga/desliga a troca de corpo (`_StartLifeTransfer`) | — |
| 7 | liga/desliga o alarme (7.3) | 0xF / 1 |
| 8 | na superfície: `_TryMyDropOrLift(1, posição)` (pega/larga na própria célula; rainha alada sem colônia: cava o ninho) | erro 1 se devolver −1 ou fora da superfície |
| 9 | no ninho preto, com y > 2: `_YellowBirth(2, posição + 2·D8[_MeDir ^ 4], 0x10, 1)` (fundar) | erro 1 senão |
| 10 | liga/desliga o modo alvo (`_TargetAnt`; usado pela aranha) | — |
| 11 | aranha: alterna `_MeSMode` 7 (caça por faro) / 0 | som 2 |
| 12 | aranha: alterna `_MeSMode` 8 (laser) / 0 | som 0x29 |

`_Recruit`/`_UnRecruit` estão em [superficie.md](superficie.md) (9.4).

## 11. Controles do jogador

### 11.1 Do Windows ao evento do jogo

**Fato visto no código** (`MAINWNDPROC` → `_DoMouse` `1000:13f6` / `_DoKeyDown` `1000:0e8c` → `_DoEvent`). O evento carrega a posição e um campo de bits:

| Origem | Bits |
|---|---|
| botão esquerdo apertado (`WM_LBUTTONDOWN`) | 0x200 |
| botão direito apertado (`WM_RBUTTONDOWN`) | 0x800 |
| **duplo clique** esquerdo (`WM_LBUTTONDBLCLK`) | 0x2000 |
| duplo clique direito (`WM_RBUTTONDBLCLK`) | 0x4000 |
| Shift / Ctrl seguros | bit 0 / bit 1 |
| teclas **Espaço** ou **Insert** | como clique esquerdo na posição do cursor (0x200); se a mensagem seguinte já é outra Espaço/Insert, como duplo clique (0x2000) |
| tecla **Delete** | como clique direito (0x800), ou duplo (0x4000) |
| setas | movem o cursor 8 pixels (`SetCursorPos`) |

Antes de chegar ao jogo, `_HelpKeyDown` e `_YellowCommandKey` (11.4) têm prioridade sobre as demais teclas. Letras A–Z alimentam um anel de 4 caracteres; se `_Cheats` (`8:83bc`) estiver habilitado e as 4 últimas letras formarem um código da tabela (guardada com os bits invertidos; lida do `SIMANTW.EXE`: `oops rand erad FUND joke hole HOLE qeen QEEN eggs EGGS heal HEAL zhea ZHEA just will mick jeff joel fred jenn susi data serv clie`), chama `_CheatKeys`. Enter limpa o anel. Códigos e efeitos: [trapacas.md](trapacas.md).

### 11.2 Janela de edição: `_ProcEditEvent` (`1010:0b7a`)

Objetos da janela 0 (nomes de SHARED 1/0, **fato verificado** no dump):

| Objeto | Nome | Ação |
|---|---|---|
| 4, 0x15 | `editTileArea`, `editTileArea2` | `_processEdit` (11.3) |
| 6 | `editHelpBut` | ajuda |
| 7 | `editTools` | menu de ferramentas (`_EditToolsMenu`: Quick → passa para Experimental; Experimental → ferramentas 11.6) |
| 8, 9, 0xA | `editTopBut`, `editBNestBut`, `editRNestBut` | mostra a superfície, o ninho preto ou o vermelho (`_SetMapPlane(1/2/3)`) |
| 0xB | `editGoYellow` | centraliza na amarela |
| 0xC, 0xD, 0xE | `editGoSpider`, `editGoBQueen`, `editGoRQueen` | centraliza na aranha / rainha preta / rainha vermelha |
| 0xF | `editPause` | pausa/despausa |
| 0x10 | `editScent` | menu de camada de cheiro exibida (SHARED 1/6: Off, Alarm, BNest, BTrail, RNest, RTrail) |
| 0x11 | `editHealthY` | arrastar define `_MeWarnHealth` = altura × 100 / altura total |
| 0x12 | `editHealthB` | define `_BlkWarnHealth` (`9:769c`) do mesmo modo |

A rolagem da vista (`_goStepUp/Down/Left/Right/NE/SE/SW/NW`, `1030:6efe`–`754e`, e `_FinishDiagStep/HorzStep/VertStep`) só move a janela (`_MapPnt`, `9:7d2e`) e guarda o centro por plano; não mexe na simulação.

### 11.3 Clique no mapa: `_processEdit` (`1000:6a38`)

**Fato visto no código.** `dupla` = bits 0x6000 (duplo clique, esquerdo ou direito). Um clique direito simples (0x800) é tratado como clique simples. Célula = `((px − esquerda) / largura_tile + _MapPnt.x, (py − topo) / altura_tile + _MapPnt.y)`. Clique simples com a janela de edição à frente espera ~6 ticks do relógio antes de processar.

```
Experimental (_CurGameType = 3): _processExp (ferramenta, 11.6) ; fim
ferramenta 10: _ExchangeLives(plano exibido, célula) ; fim
ferramenta 11: se superfície e o jogador é a aranha: _processSpider(célula, dupla) ; fim
clique simples:
    na amarela: se o botão continua apertado (sem Shift): menu da amarela (_AntMenu, 11.5)
                senão, se clicou na cauda da rainha (0xFE): nada ; senão centraliza
    em outra célula com formiga: se o botão continua apertado (sem Shift): janela Examine (_MagnifyMenu) ; fim
    senão: ORDEM(cmd = 0)
duplo clique:
    em formiga não-amarela, não-ovo, sendo formiga (_MeMode = 0):
        _MeTargIndex = vaga ; _MeCmd = 4 (mesma cor: ser alimentada) ou 3 (outra cor: atacar)
        guarda T, plano e posição do alvo ; destino = célula ; _MeMoveMe = 1
    senão: ORDEM(cmd = 1)
ORDEM(cmd):
    vista da superfície (plano exibido < 2):
        jogador-aranha: _processSpider(célula, cmd)
        senão: destino = (plano 1, célula) ; _MeCmd = cmd ; _MeMoveMe = 1 ; _MeCrazyCnt = −2
    vista de ninho (só formiga):
        cmd 1 → _MeCmd = 2 (cavar) se a célula é escavável (terra/superfície fechada) ou é a linha 0
                fechada com a amarela num ninho ; 1 (pegar/largar) se carrega algo ou a célula tem
                comida/pedrinha/ovo ; 2 nos demais casos
        cmd 0 → _MeCmd = 0
        destino = (plano exibido, célula), com x limitado a 1..62 nas linhas 0–1 ; _MeMoveMe = 1 ; _MeCrazyCnt = −2
```

`_processSpider(x, y, dupla)` (`1000:7198`): clique simples (fora do modo alvo) → anda até lá (`_SuserX/Y`, `_SMode = 0`, sem presa). Duplo clique ou modo alvo: se há formiga na célula, ela vira a presa (`_SMode = 2`); no modo alvo, `_MeSMode = 6` e o modo é encerrado. `_MeCmd = dupla` decide entre andar e correr.

### 11.4 Teclas de comando: `_YellowCommandKey` (`1000:8562`)

**Fato visto na desmontagem** (tabela de saltos em `1000:85c8`). Códigos de tecla virtual do Windows:

| Tecla | Formiga comum | Rainha alada sem colônia | Aranha |
|---|---|---|---|
| `1` ou NumPad 1 | 1 recrutar 5 | 8 cavar ninho | 10 alvo |
| `2` ou NumPad 2 | 2 recrutar 10 (com Ctrl: 3 todas) | 9 fundar | 11 faro |
| `3` ou NumPad 3 | 4 liberar metade | idem | idem |
| `4` ou NumPad 4 | 5 liberar todas | idem | idem |
| `0` ou NumPad 0 | 7 alarme | idem | idem |
| `X` ou F9 | 6 troca de corpo | idem | idem |
| `J` ou NumPad `*`, com Tab apertado | — | — | 12 laser |
| Backspace | parar a ordem em andamento (zera destino e `_MeMoveMe`); sem ordem, centraliza; com Shift, centraliza | idem | aranha: para (destino = posição, `_SMode = 0`) ou centraliza |
| Home | centraliza (som 0xF) | idem | idem |

Observação: o acelerador do Windows é aplicado antes (11.7); por isso Shift+0..4 vão para o menu Speed e Ctrl+0 para o menu View, e não para esta tabela [hipótese: precedência do `TranslateAccelerator` no laço de mensagens de `WINMAIN`, fato visto no código].

### 11.5 Janelas de menu da amarela

**Fato verificado** nos nomes de objeto de SHARED tipo 1 e nas tabelas `10:1ba6`/`10:1baa`/`10:1bb0` do `SIMANTW.EXE`. `_AntMenu` (`1010:6d24`) abre um menu de proximidade conforme o corpo e traduz o item em comando:

| Janela | Itens (nome do objeto) → comando |
|---|---|
| 7 `yllwRecruitWin` (formiga) | `yllwRecruit5` → 1, `yllwRecruit10` → 2 (com Shift: 3), `yllwReleaseHalf` → 4, `yllwReleaseAll` → 5, `yllwExchange` → 6, `yllwRecruitHelp` → 0 |
| 8 `yllwStartMenu` (rainha alada sem colônia) | `yllwStartDig` → 8, `yllwStartBrood` → 9, `yllwStartHelp` → 0 |
| 32 `editSpiderMenu` (aranha) | `spiderMenuChase` → 10, `spiderMenuHunt` → 11, `spiderMenuXchange` → 6, `spiderMenuHelp` → 0 |

### 11.6 Modo Experimental: `_processExp` e `_DoTool` (`1010:7022`, `1010:727e`)

**Fato visto no código.** A ferramenta é `_CurExpTool` (`9:79e2`), escolhida no menu `editToolsMenu` (SHARED 1/9). Com o botão apertado, a ferramenta é reaplicada a cada célula nova sob o cursor (e continuamente para as ferramentas ≥ 5).

| `_CurExpTool` | Objeto | Efeito |
|---|---|---|
| 0 | `editExpInfo` | janela Examine da formiga na célula |
| 1 | `editExpBarrier` | parede (só superfície; `_DropWall`) |
| 2 | `editExpDig` | cavar (`_ExpDig`) |
| 3 | `editExpAddAnt` | pôr formiga (`_ExpAddAnt`) |
| 4 | `editExpAddFood` | comida (com a variante do submenu: 20 vezes em `S1(9) − 4` em volta, `_IncFoodHere`) |
| 5 | `editExpTrails` | cheiro (`_ExpIncSmell`, [feromonios.md](feromonios.md)) |
| 6 | `editExpInsecticide` | inseticida (`_ExpKillAnts`: 8 vezes; mata formigas, aranha e formiga-leão) |
| 7 | `editExpStartGame` | sai do Experimental e começa a simulação com a amarela |

### 11.7 Menus e aceleradores

**Fato verificado:** textos do menu de SHARED 6/0 e tabela de aceleradores do recurso `ACCELERATORTABLE` do `SIMANTW.EXE` (offset 500224, entradas de 5 bytes). Comandos (byte baixo do id) tratados por `_ProcMenu` (`1000:4fc2`).

| Menu | Item | Atalho | Id |
|---|---|---|---|
| File | New / Load / Save / Save As / Quit | Ctrl+N / L / S / A / Q | 3 / 4 / 5 / 6 / 8 |
| Window | Edit, Map, Behavior, Caste, History, Info, Status | Ctrl+E, M, B, C, H, I, T | 0x11–0x17 |
| Window | Help | F1 | 0x18 |
| View | Yard, House, Strategic, Graphic | Ctrl+Y, 5, 6, 7 | 0x21–0x24 |
| View | Surface, Black Nest, Red Nest | Ctrl+8, 9, 0 | 0x26–0x28 |
| Options | AutoTrack, Music, Sound Effects, Events, Messages, Silly | Ctrl+G, J, K, —, —, — | 0x31–0x36 (alternam `_OptionStates[0..5]`, `8:85f2`) |
| Speed | Pause / Slow / Normal / Fast / Ultra | Shift+0 / 1 / 2 / 3 / 4 (também Shift+NumPad 0–4) | 0x41 / 0x43–0x46 |

A tabela de aceleradores tem mais 13 entradas sem item de menu (ids 0xA0–0xAC: Ctrl+F4, Ctrl+NumPad−, Ctrl+Espaço, Ctrl+F6, Ctrl+NumPad+, Ctrl+P, Ctrl+A, Ctrl+Z, Ctrl+setas, Ctrl+U, Ctrl+V, Ctrl+F) [em aberto: quem as trata; Ctrl+A aparece duas vezes, a primeira, Save As, vence]. O índice da opção "Silly" = `_OptionStates[5]` confirma o uso em [superficie.md](superficie.md) (balões).

## 12. Peculiaridades a reproduzir

| Id | O quê | Onde |
|---|---|---|
| A1 | a amarela entra no ninho preto até a coluna 64 (as demais formigas só até 63) | 6.1 |
| A2 | troca de corpo com a lista cheia remove o alvo e falha | 8.3 |
| A3 | a busca de reencarnação começa no índice `_ListIndex` (uma vaga além do fim) | 9.2 |
| A4 | ao reencarnar num adulto, o mapa de vida recebe o T do adulto em vez de 0xFF até o próximo tick | 9.2 |
| A5 | sorteios das animações dependem da janela à frente e da música | 10.2, 9.1 |
| A6 | `_PickupMyFood` sempre marca a trilha **preta**, mesmo com a amarela vermelha (o resto da trilha usa `_MeColor`) | 7.1 |
| A7 | `_GetWinner` com `_MeType` sem cor: a amarela é tratada como preta | 10.1 |
| A8 | o choque elétrico só é testado quando uma ordem termina | 4 |

## 13. Casos de teste sugeridos (bancada QEMU)

Bancada: [docs/bancada-qemu.md](../../bancada-qemu.md) (comandos `clicar_em`, `teclas`, `capturar`, save com Ctrl+A). Coordenadas de tela dependem da janela de edição aberta na posição padrão de 640 × 480 [a medir uma vez e registrar no roteiro].

| # | Regra | Roteiro | Esperado |
|---|---|---|---|
| Y1 | Vida por tempo e por passo (3, 4) | Quick Game; salvar parado em dois momentos; depois clicar longe e salvar ao chegar | parado: −1 a cada 64 ticks (`_MeHealth`); andando: −1 a cada 2 passos (`_MeSteps`) |
| Y2 | Ordem simples (4) | clique simples numa célula livre da superfície | `_MeMoveMe = 1`, `_MeCmd = 0`, `_MeCrazyCnt = −2` logo depois; ao chegar, `_MeMoveMe = 0` e posição = destino |
| Y3 | Contorno (5.3) | clique atrás de uma pedra grande | o caminho contorna a pedra (capturas a cada segundo); `_MeCrazyCnt` entre 16 e −2 durante o contorno |
| Y4 | Pegar e largar comida (7.1) | duplo clique em comida; depois duplo clique em chão perto do buraco | `_MeType` 0x10 → 0x18, `_MeScent = 200`; trilha preta 200, 199… no caminho; ao largar, tile 0x48 e `_MeType = 0x10` |
| Y5 | Comer com fome (7.2) | baixar `editHealthY` até a vida atual; duplo clique em comida | `_MeHealth = 100`, `_HealthB` sobe `vida / (_BpopT − 1)`; tile de comida −1 |
| Y6 | Trofalaxia (7.2) | com fome, duplo clique numa operária preta | ao encostar, mensagem "You feel much better now!", `_MeHealth = 100` |
| Y7 | Recrutar e liberar (10.3, 11.4) | teclas 1, 2, Ctrl+2, 3, 4 | `_ModePopB[5]` (Defense) +5, +10, todas; metade; 0 |
| Y8 | Alarme (7.3) | tecla 0 na superfície e andar | `_MeDropAlarm = 1`; `_PherMapA` +50 por passo (máx. 200); entrar no ninho desliga |
| Y9 | Entrar/sair do ninho (6) | clique simples no buraco; depois clique na superfície estando no ninho | `_MePlane` 1 → 2 com (x, y) = (y_buraco, 1), dir 4; saída numa célula vizinha do buraco na direção do destino |
| Y10 | Coluna 64 (A1) | Experimental: criar buraco na coluna 64 e fazer a amarela entrar | `_MePlane = 2` (preto), enquanto formigas comuns no mesmo buraco vão para a lista R |
| Y11 | Luta (10.1) | duplo clique numa vermelha | `_MeCmd = 3`; ao encostar, luta; vitória → cadáver vermelho; derrota → morte código 0 com 10030 |
| Y12 | Troca de corpo (8.3) | tecla X e clique numa soldado preta | `_MeType = 0x30`, o corpo antigo na lista como operária (T grupo 2, M 0) na posição antiga |
| Y13 | Virar aranha (8.3) | tecla X e clique na célula da aranha | `_MeMode = 1`; `_MeLoc` acompanha `_SpidX >> 4` |
| Y14 | Morte e reencarnação (9.2) | deixar a amarela morrer de fome | mensagem 10040; amarela renasce como ovo se houver ovo preto no ninho (`_MeEgg` 1→7) ou transferida para adulto (10059) |
| Y15 | Fundar colônia (9.3) | Jogo Completo: rainha alada, tecla 1 (cavar) e depois tecla 2 no ninho com y > 2 | buraco novo; `_MeType` 0x40 → amarela operária; rainha comum na lista B; `_MeNestStarted = 1` |
| Y16 | Teclado como mouse (11.1) | posicionar o cursor e apertar Espaço duas vezes rapidamente | mesmo efeito de um duplo clique |
| Y17 | Aceleradores (11.7) | Shift+3, Ctrl+9, Ctrl+G | velocidade "fast", vista do ninho preto, AutoTrack alternado (`_GameSpeed`, `_MapPlane`, `_OptionStates[0]` no save) |

## 14. Implementação no port

**Onde:** `engine/src/rules/amarela.rs`. Dados: `data/amarela.json`, gerado por `tools/extrair_amarela.py` a partir do `SIMANTW.EXE` (`_Dx8/_Dy8` 8:0000/0008, `_Dx9/_Dy9` 8:0010/001a, `_absSearchDirs` 8:006c, `_UnCarryCaste` 8:0094, classe e matriz de luta 10:22c6/22d6, borda do buraco 10:230c), mais as constantes das regras, os quadros das animações, as músicas e as mensagens (origem de cada valor em `_origem`).

- Tick: `impl RegrasAmarela for RegrasOriginais` chama `simular_amarela` (§3) e `mover_amarela` (§4) com `RegrasOriginais::amarela` (carregado de `data/amarela.json` com mods; `TabelasAmarela::padrao()` é a cópia embutida, para testes).
- Usadas por outras regras: `lutar_com_amarela(e, t, plano, i)` (`_YellowFight`; plano < 2 = lista A, 2 = B, 3 = R; chamada pela superfície e pelos dois ninhos), `trofalaxia(e, t, x, y, d)` (`_DoTroph`; superfície e ninho preto), `matar_amarela(e, t, CausaMorte)` (`_YellowDeath`, códigos 0–10; quintal: 5, 6, 9; formiga-leão: 2; aranha: 1, 3, 4) e `mover_marca` (`_MoveMyLife`, antes da morte pela formiga-leão e pela aranha).
- Comandos do jogador (sem SDL; a interface converte pixel → célula, §11.3, e chama):

| Função | Original | Uso |
|---|---|---|
| `clicar_no_mapa(e, t, ctl, plano_vista, x, y, duplo, segurando)` | `_processEdit` | clique completo: Experimental, ferramentas 10/11, clique na amarela (centralizar ou menu), Examine, ordem. Devolve `RespostaClique` |
| `ordem_ir(e, plano_vista, x, y)` | ORDEM(0) | "ir para (x, y)" (clique simples) |
| `ordem_agir(e, plano_vista, x, y)` | duplo clique | atacar ou pedir comida a uma formiga (ordens 3/4); senão ir e pegar/largar/cavar (1/2) |
| `comando(e, t, ctl, c)` | `_YellowCommand` | comandos 0–12 (§10.3), também os itens dos menus da §11.5; `false` = som de erro |
| `tecla_de_comando(e, t, ctl, vk, Modificadores)` | `_YellowCommandKey` | teclas virtuais do Windows (módulo `vk`): 1–4, 0, X/F9, J/NumPad* com Tab, Backspace, Home; devolve `RespostaTecla` |
| `trocar_de_corpo(e, t, ctl, plano, x, y)` | `_ExchangeLives` | clique com a ferramenta 10 |

`ControleJogador` guarda o que não vai para o save (`_SaveGamePaused`). Diálogos, mensagens de vento (4/1010) e o movimento da vista não são simulados: as músicas viram `Evento::Musica`, os efeitos sonoros `Evento::Som` (ids em `sons` de `amarela.json`, ver [som.md](som.md)) e `_TryAntTheme` `Evento::TemaDoNinho`, os quadros da §9.1 viram `Evento::Mensagem`, e "centralizar" (`_GotoMyAnt`) grava `_LessonTemp = 1` e deixa `Evento::CentralizarNaAmarela` na fila; quem move a vista é o laço de jogo (`core/partida.rs`, ver [jogo.md](../telas/jogo.md), "Simulação ao vivo").

**Sorteios e animações.** Todos os sorteios acontecem na ordem do original, inclusive os das animações: `_AnimYellowFight` (S2 por quadro na luta ou no choque, S8 sempre; na formiga-leão o tile do centro oscila 0x3B–0x3D e fica no último valor) e `_AnimYellowInsane` (gerador R para o alvo, S8 quando trava; a amarela anda de verdade pelo mapa de vida e volta). O número de quadros usa `animacao.janela_edicao_na_frente` (padrão: verdadeiro) e `_OptionStates[1]` (música); com música ligada as mortes 1–9 saem no primeiro quadro, como no caminho MCI (§10.2). Os quadros desenhados vão para a interface em `Evento::Animacao` (§10.2). `_YellowBirth` é em tempo real e não sorteia: o port aplica só o estado final.

**Escolhas e lacunas do port:** `_CommandKey` (8:85e2) não está no estado: tratado como 0. A queda da comida na casa (`_DROPdir`, 9:9c66) não está no estado: a comida some. `_SpecialXfer` (escolher outra colônia no Jogo Completo) só emite a mensagem 10058. Ao morrer com a ferramenta 10/11 ligada, a ferramenta é desligada mas a pausa salva não é restaurada (está na interface; pela regra da pausa a amarela não morre pelo tick com essas ferramentas). As peças comuns com o ninho e a superfície (`_GetMap`, `_GetLife`, `_IsClearTile`, `_IsNotObstacle`, `_IsThisPebble`, `_IsItHole`, `_TileCanBeMovedOn` na superfície, `_GetBestDir`, `_FindAntIndex`, `_DigTileB/R`, `_SmoothEdges*`, `_FixExitMap*`, `_MakeNewHole*`, borda do buraco, `_PlaceEgg*`, `_DecEat*`, `_PickupFoodA`, `_DropFoodA`, `_DeadAntHere`, `_GetWinner`, `_Recruit`, `_UnRecruit`, `_GetDir`, `_GetDis`) estão em `engine/src/rules/comum.rs` (ver [../../decisoes.md](../../decisoes.md), "Módulo comum das regras"); `amarela.rs` guarda só o que é da amarela.

**Verificado:**

- `engine/tests/regras_amarela.rs`: casos Y1–Y15 da §13 que não precisam de interface (Y16/Y17 são da interface), postura da rainha, Backspace e clique na própria amarela; testes de unidade em `amarela.rs` (`_TileCanBeMovedOn`, `_GetDir`).
- Vetores de tick do original (`tests/referencias/ticks`, ticks 79–83, amarela parada no ninho; [testes-tick.md](testes-tick.md)): campos `_Me*` e a marca 0xFF iguais.
- Casos na bancada QEMU (2026-10-17), `tests/referencias/casos/Y-01..Y-04.json`, saves `tests/referencias/saves/Ynn_{ENTRADA,CARREGADO,FIM}.ANT`. No original a amarela com ordem anda durante a pausa do load, então o port parte do arquivo injetado (tick 78) e roda ticks completos; os campos `_Me*` (posição, tipo, direção, vida, ordem, último passo, passos, contorno, rastro) batem com o original no `GameTime` do estado lido logo após o load e no final:

| Caso | Entrada | Original | Port |
|---|---|---|---|
| Y-01 andar em linha reta | (10, 20) → (50, 20) | tick 100: x = 32, vida 88, 22 passos; tick 108: x = 40, vida 84, 30 passos | igual |
| Y-02 contorno | parede 0x5C em x = 30, y 15–25; (25, 20) → (35, 20) | chegou a (35, 20) antes do tick 94, vida 91 | igual |
| Y-03 comer com fome | (59, 36), ordem 1 na comida (62, 36) = 0x49; vida 50, aviso 60, `_HealthB` 40 | tick 81: (61, 36), vida 100, `_HealthB` 88 (40 + 49, −1 do ovo da rainha), tile 0x48 | igual (o −1 do ovo vem das regras do ninho, ainda vazias neste ramo) |
| Y-04 entrar no ninho | (8, 32) → ninho preto (31, 4) | tick 86: plano 2, (31, 4), vida 95, ordem encerrada | igual |

Não verificado no original: luta, troca de corpo, morte e reencarnação, recrutamento, alarme, fundação da colônia e os comandos por tecla (só testes a partir da spec).

## Lacunas

- Efeitos de `_CommandKey` (os códigos de trapaça, `_CheatKeys`, estão em [trapacas.md](trapacas.md)).
- Quem trata os aceleradores 0xA0–0xAC.
- `_SpecialXfer`/`_XferPatch` (mudar de colônia no Jogo Completo) e a revoada controlada pelo jogador (rainha alada voando, `_FlyAwayB`): documento de ciclo de vida.
- Uso de `_MeDis`, `_MePrevDis` e `_MeBase`.
- Detalhes de desenho das janelas Examine (`_MagnifyMenu`), de recrutamento e dos diálogos de morte (posições, bitmaps).
- Conferidos no original só os casos Y-01..Y-04 (andar, contorno, comer, entrar no ninho; seção 14) e os vetores de tick 79–83; o resto da seção 13 continua por rodar.
- `RegrasOriginais` sem campo para `TabelasAmarela` (mods de `amarela.json` não chegam ao tick).
- Animações (§10.2): conferidos no original só os quadros da luta (`luta_quadros`); o choque elétrico, as mortes com a janela de edição atrás e a loucura (`_AnimYellowInsane`) não foram capturados. O som da luta no tempo do original não foi ouvido lado a lado com o original (a bancada não tem placa de som). O desenho de antes do laço (até 55 ms) não é mostrado. As capturas do original e do port partem do mesmo estado, mas o Load do port ressemeia S, então as direções da nuvem diferem.
