# Geração do mundo (novo jogo e patches do quintal)

Este documento descreve como o original monta o mundo de um Tutorial, Quick Game, Full Game ou Experimental, e como gera cada patch do quintal. Os geradores aleatórios estão em [rng.md](rng.md): **S** é o LFSR (`_SRand*`) e **R** é o `rand()` do C (`_RRand`). Os valores dos mapas estão em [tiles-valores.md](tiles-valores.md).

Convenções:

- Endereços `seletor:offset` com o nome do `.SYM`. Variáveis: `_Nome` (segmento `1038` = seg. 8, `1040` = seg. 9, `1048` = DGROUP). Muitas variáveis aparecem no pseudocódigo do Ghidra só como offset (ex.: `*(int*)0x80b4`) porque o código troca `ES` com seletores guardados no DGROUP; o nome foi resolvido pelo offset nos segmentos 8/9/10 e pelo contexto, e conferido na desmontagem quando havia dúvida.
- Coordenadas: `_MapA` tem 128 × 64 células, índice `x * 64 + y`; `_MapB`/`_MapR` têm 64 × 64, índice `x * 64 + y`, com `y = 0` na linha da superfície. `x` é horizontal e `y` vertical (fato: as tabelas de forma das pedras, da borda do buraco e da formiga-leão só formam desenhos coerentes, com cantos e bordas no lugar, nessa orientação; ver [tiles-valores.md](tiles-valores.md)).
- Direções 0–7 pelas tabelas `_Dx8` (`1038:0000`) = `0, 1, 1, 1, 0, −1, −1, −1` e `_Dy8` (`1038:0008`) = `−1, −1, 0, 1, 1, 1, 0, −1`: 0 = cima, 2 = direita, 4 = baixo, 6 = esquerda (fato verificado: bytes do segmento 8 no executável).
- `_SRand1(n)` é escrito `S(n)`; `_SRandK()` é escrito `Sk(K)`; `_SGRand(n)` é `SG(n)`.

## Verificação feita

**Fato verificado (2026-10-16)** contra o `tests/referencias/saves/QUICK2.ANT` (Quick Game salvo pelo original na bancada QEMU): reimplementando em Python a sequência descrita aqui a partir da semente salva em `_YMapSeeds`, o resultado reproduz:

- `_MapB` e `_MapR`: **idênticos** (4.096 de 4.096 bytes cada, incluindo os valores aleatórios dos túneis e o alisamento das bordas);
- `_MapA`: idêntico exceto 10 células, todas de bichos que andaram durante o jogo (bichos-de-conta, 0x70–0x73, e uma lagarta, 0x69–0x6B, que aparece depois);
- `_HoleMapB[32] = 3` e `_HoleMapR[32] = 118`; rainha vermelha em (41, 11) com T = 0xE2 e a metade em (40, 11) com T = 0xEA, iguais à lista R salva;
- `_FoodA = 117` (o save tem 117).

Script: `re_trabalho/verifica_geracao_quick2.py` (material de trabalho, fora do git). A geração da **casa** (`_MakeHousePatch`) e o caminho do Full Game ao trocar de patch **não** foram verificados contra o original.

## 1. Escolha do jogo: `_NewGame` e `_DoScenario`

`_DoScenario` (`1000:5998`) abre a janela 2 (`scenarioWin`, id 0x200) e devolve o id do objeto clicado. Os nomes vêm do objeto tipo 1 nº 2 do banco `SHARED` (fato verificado: `re_trabalho/dump2/SHARED/t01_00002_b1.bin`). `_NewGame` (`1000:d6dc`) converte o id no tipo de jogo `_CurGameType` (`1040:80b4`). Fato visto no código.

| Id | Objeto | `_DoScenario` devolve quando | `_NewGame` faz |
|---|---|---|---|
| 0x202 | `scenQuickGame` | clique | `_CurGameType = 1` |
| 0x203 | `scenFullGame` | clique **ou tecla Enter** (`GetAsyncKeyState(0x0D)`) | `_CurGameType = 2` |
| 0x204 | `scenExperimental` | clique | `_CurGameType = 3` |
| 0x205 | `scenCancel` | clique ou `_DialogAbort` | devolve −1 sem mudar nada |
| 0x206 | `scenTutorial` | clique | `_CurGameType = 0` |
| 0x207 | `scenLoad` | clique | `_LoadGame(0, 0)`; se falhar, mostra o diálogo de novo |

Para os tipos 0–3, `_NewGame`, nesta ordem: encerra modos de ferramenta pendentes (`_CurGameTool` 10 ou 11), se um par de variáveis do DGROUP (`0xcf22/0xcf24`, sem nome) não for zero, troca os bitmaps de um botão da faixa (um par de bitmaps para Experimental, que também zera `_CurExpTool`, e outro para os demais tipos), `_SetDefaultWindPrompt(1)`, `_SetEditWinTitle`, `_QueenStorageB = 0`, `_JustXfered = 1`, `_LayDownQueenMode = 1`, `_UpdateLayQueenModeDisplay`, **`_RandYard()`**, e, só no Tutorial (tipo 0), `_TutLesson = 1`, `_TutGiven = 0` e abre a janela 0 (`editWin`) se o parâmetro for 0. Depois reabre as janelas de casta/modo, mapa e edição e centra a vista na formiga amarela. Fato visto no código.

Observações (fatos vistos no código):

- O **Tutorial do menu** é um jogo tipo 0 gerado por `_RandYard` normal. A rotina `_GenerateTutorial` (abaixo) só roda quando se carrega um arquivo chamado `Tutorial.Ant` (string 1800 do banco, objeto tipo 4 nº 1800 do `SHARED`), via `_LoadGame` ou `_SpecialTutorialInit` (`1010:9cea`).
- `_InitSimVars` (`1030:5a70`) **não** é chamada no novo jogo, só na inicialização do programa (`_initStuff`) e no tutorial por arquivo. Ela põe `_GameSpeed = 1`, `_MeWarnHealth = 30`, `_BlkWarnHealth = 30`, `_CurExpTool = 0`, `_SwarmCntR = _SwarmCntB = 0`, `_QueenStorageR = _QueenStorageB = 0`.
- O novo jogo **não** ressemeia S nem R: o mundo depende do estado de R no momento do clique (ver passo 2.4).

## 2. `_RandYard` (`1030:6086`)

Ordem das ações (fato visto no código):

1. `_ClrArrays` (`1030:6dec`): zera `_MapA`, `_LifeA`, `_MapB`, `_MapR`, `_ExitMapB`, `_ExitMapR`, `_LifeB`, `_LifeR`, os seis mapas de feromônio (`_PherMapA`, `_PherMapF`, `_PherMapBN/BT/RN/RT`) e as listas de formigas.
2. `_ClearHistory(1)`, `_initControls()`.
3. `_CurYardPnt = _YMapPnt = (11, 8)`; `_InitSimYard` (`1030:1378`: menino em (0xB4, 0x49), cachorro em (0xFA, 0x96), zera gato, pássaro, chuva etc.).
4. Zera estados de partida: `_CurGameTool = −1`, `_LastThemeTime`, `_TimeTemp`, `_LastThemeSong`, `_HouseColonies`, `_JustToBeMean`, `_BlackWon`, `_IsGameOver`, `_ForSaleState`, `_Invincible`, `_NeverHungry`, `_AlwaysHealthy`, `_GameTime` (32 bits).
5. `_MapPlane = 2` (ninho preto) se `_CurGameType < 2`, senão `1` (superfície); `_MapMode = _MapPlane`; `_YardMode = 0`.
6. **Sementes dos patches:** para cada uma das 192 palavras de `_YMapSeeds` (`1038:0224`), em ordem crescente de endereço: `semente = (RRand(0x7FFF) + 0x4000) & 0x7FFF`. Valores de 0 a 0x7FFE.
7. `_MeNestStarted = 1` se `_CurGameType ≠ 2`, senão `0`.
8. `_RandWorld(semente, pretas, vermelhas = 1, x = 11, y = 8)`, com `semente = _YMapSeeds[8 * 12 + 11]` e `pretas = 1` se `_CurGameType ≠ 2`, senão `0`.
9. Copia `(_MeLocX, _MeLocY)` para o ponteiro de vista do plano da formiga amarela: `_AMapPnt` (plano 0/1), `_BMapPnt` (2) ou `_RMapPnt` (3).

Resumo por tipo de jogo (fato visto no código, consequência direta dos passos acima e de `_RandWorld`):

| Tipo | `_CurGameType` | Vista inicial | Colônia preta | Formiga amarela | Comida inicial | Aranha |
|---|---|---|---|---|---|---|
| Tutorial (menu) | 0 | ninho preto | sim | operária (T = 0x10) no ninho preto | sim | **desligada** (`_SpidOn = 0`) |
| Quick Game | 1 | ninho preto | sim | operária no ninho preto | sim | ligada |
| Full Game | 2 | superfície | **não** (nem buraco, nem rainha) | rainha alada (T = 0x40) na superfície | sim | ligada |
| Experimental | 3 | superfície | sim | operária na superfície em (0x40, 0x20), `_MeMode = 2` | **não** | ligada |

## 3. `_RandWorld(semente, pretas, vermelhas, px, py)` (`1030:5ab0`)

Chamadores: `_RandYard` (novo jogo e antes de carregar um save) e `_XferPatch` (`1000:ab90`, troca de patch no Full Game, com `pretas = _YMapPopB[px][py]` e `vermelhas = _YMapPopR[px][py]`). Ordem exata (fato visto no código; passos 3.1 a 3.12 verificados no `QUICK2.ANT`):

1. `_SetSRandSeed(semente)`.
2. `_InitSpider` (`1020:4668`): sem sorteio. Aranha em `_SpidX = 0x400`, `_SpidY = 0x200` (coordenadas ×16), direção 0, `_SpidBurpCnt = 10`, `_SpidOn = (_CurGameType ≠ 0)`, alvo nenhum.
3. `_MakeMap(px, py)` (`1020:3100`): `n = px * 16 + py`; se `n > 0x25` (37), `_MakeYardPatch()`; senão `_MakeHousePatch(n)`. O patch inicial (11, 8) dá 184, quintal.
4. `pretas += pretas >> 2` e `vermelhas += vermelhas >> 2` (×1,25 com truncamento; 1 continua 1).
5. Para todas as 4.096 células dos ninhos: `_MapB = _MapR = 0x2E` (terra maciça), `_ExitMapB = _ExitMapR = 0`, `_LifeB = _LifeR = 0`; e as 8.192 de `_LifeA = 0`. **Atenção:** isso acontece depois de `_MakeMap`, então, em `_XferPatch`, a geração do patch (formigas-leão, passo 4.3) ainda vê o `_LifeA` do patch anterior (só `_RandYard` limpa antes).
6. Linha da superfície dos ninhos, para cada coluna `c = 0…63`, em ordem: se `_TERRAINset = 0` (quintal), `_MapB[c][0] = 0x1C + Sk(4)` e depois `_MapR[c][0] = 0x1C + Sk(4)`; na casa, se `py > 3`, os dois recebem `0x1C + S(2)` (nessa ordem), senão os dois recebem `0x1E` sem sorteio. `_ExitMapB[c][0] = _ExitMapR[c][0] = 0xFF`.
7. Zera os seis mapas de feromônio.
8. **Formigas pretas na superfície** (`_LifeA`): `k = S(6)`, `j = Sk(2)` (sorteados sempre, mesmo com `pretas = 0`). Repete `pretas` vezes: `x = SG(128)` (puxado para a esquerda), `y = Sk(64)`; se `_MapA[x][y] < 0x50`: se a célula de `_LifeA` está livre, sorteia `v = Sk(16)` e o tipo T:
   - `v < k + 7` → `0x10 + Sk(8)` (operária [hipótese de nome, ver save-ant.md]);
   - senão, `v < 15 − j` → `0x30 + Sk(8)`;
   - senão, `Sk(2) = 0` → `0x40 + Sk(8)`, senão `0x20 + Sk(8)`.
   Os 3 bits baixos são a direção. Se a célula já tem formiga, a tentativa não conta (até 49 repetições no total por colônia). Se `_MapA ≥ 0x50`, a tentativa é perdida sem repetição.
9. **Formigas vermelhas na superfície:** igual, com `vermelhas`, `x = 127 − SG(128)` (puxado para a direita) e os tipos com o bit 7 ligado (0x90, 0xB0, 0xC0, 0xA0 + direção).
10. Zera `_HoleMapB` e `_HoleMapR`. Se `_CurGameType ≠ 2` ou `pretas > 0`: `_MakeNewHoleB(32)`. Se `vermelhas > 0`: `_MakeNewHoleR(32)` (ver 3.A).
11. Zera os acumuladores de escavação (`_TileTotXB/YB/XR/YR`, `_TilesDugB/R`, `_TileMassXB/YB/XR/YR`) e põe `_WaterTable = 64` (nenhuma linha inundada).
12. Se `pretas > 1`: `_DigOutBNest(pretas * 16)`; se `vermelhas > 1`: `_DigOutRNest(vermelhas * 16)` (ver 3.B). No novo jogo isso nunca acontece (valores 0 ou 1).
13. `_BuildAntListA` (reconstrói a lista A varrendo `_LifeA`, x de 0 a 127 e y de 0 a 63, M = 2, S = 0), `_ClearListB`, `_ClearListR`, `_BlkQueens = _RedQueens = 0`, `_LastBlackEgg = _LastRedEgg = (−1, −1)`.
14. Se `vermelhas > 0`: rainha vermelha (mesma lógica de `_PlaceRedQueen`, escrita em linha; ver 3.C).
15. Se `pretas > 0`: `_PlaceBlackQueen()` (3.C).
16. `_InitYelloAnt()` (3.D).
17. `_FoodB = _FoodR = _FoodA = 0`; se `_CurGameType ≠ 3`: `_AddFood(−1, 0)` (3.E).
18. `_HealthB = _HealthR = 100`, `_EditMsgDelay = 0`, `_BlackLost = _RedLost = 0`, `_Cycle = 0`, `_ClearHistory(0)`, `_CountAnts()`, invalida o mapa da tela, `_MapPnt = _AMapPnt = (0x40, 0x20)`, `_BMapPnt = _RMapPnt = (0x20, 1)`.

### 3.A Buracos de entrada: `_MakeNewHoleB(c)` (`1020:1b06`) e `_MakeNewHoleR(c)` (`1020:1d02`)

O parâmetro `c` é a coluna do ninho; na superfície o buraco fica na linha `y = c` (fato visto no código: `_MapA[x * 64 + c]`).

No quintal (`_TERRAINset = 0`):

1. `r = S(31)`.
2. Para `t = 0…33`: preto: `x = (r + t) mod 32 + 2` (x de 2 a 33, lado esquerdo); vermelho: `x = 126 − (r + t) mod 32` (95 a 126, lado direito). Para no primeiro `x` em que `_IsClear3x3(1, x, c)` é verdadeiro (ver [tiles-valores.md](tiles-valores.md): célula e 8 vizinhas dentro do mapa, sem formiga e com chão `< 0x10`). Se nenhum servir, sai sem fazer nada.
3. `_MapA[x][c] = 0x50` (buraco); cada vizinha `d` (0–7) que esteja dentro do mapa e tenha valor `< 0x50` recebe a borda `DGROUP:0x230c[d]` = `19 1a 1c 1f 1e 1d 1b 18`.
4. `_LastNewHoleB/R = x`, `_LastNewExitB/R = c`, `_HoleMapB/R[c] = x`.
5. Preto: `_DigTileB(c, 1)`. Vermelho: escava `_MapR[c][1]` (se for terra, `Sk(8)` e acumula a escavação) e alisa as quatro vizinhas e a linha 0, como `_DigTileR`.

Na casa (`_TERRAINset ≠ 0`), o laço não testa `_IsClear3x3`: aceita a primeira célula cujo valor tenha versão "com buraco" (`_CanBeHouseHole`, `1020:1cba`: 0 → 0x86; 2 ou 3 → 0x8A; 0x5E–0x61 → valor + 0x22; 0x66 → 0x85; 0x68 → 0x84) e não desenha borda. [não verificado contra o original]

### 3.B Escavar um ninho: `_DigOutBNest(n)` (`1030:62de`) e `_DigOutRNest(n)` (`1030:63b8`)

Fato visto no código; não exercitado no novo jogo.

```
pos = (32, 1); dir = 4 (para baixo); escava (32, 1)
repetir n vezes:
    dir = (dir + S(5) − 3) & 7          // giro de −3 a +1: a deriva é assimétrica
    nx = x + Dx8[dir]; ny = y + Dy8[dir]
    se nx < 1:  nx = 1,  dir = 2   senão se nx > 62: nx = 62, dir = 6
    se ny < 2:  ny = 1,  dir = 4   senão se ny > 62: ny = 62, dir = 0
    se _DigTileThemB(nx, ny) escavou:
        pos = (nx, ny)
        se ny == 1 e _HoleMapB[nx] == 0: _MakeNewHoleB(nx)
```

`_DigTileThemB(x, y)` (`1020:22d4`) só escava se `0 < x < 63`, se a célula de baixo for terra (ou `y > 62`) e se a de cima for terra (ou `y < 3`); não exige que a própria célula seja terra (uma célula já escavada é sorteada de novo). Em `y = 0` grava 0x18 e abre buraco; senão grava `Sk(8)`. Depois acumula a escavação, alisa as quatro vizinhas e atualiza `_ExitMapB`.

### 3.C Rainhas: `_PlaceBlackQueen` (`1030:65ce`) e a rainha vermelha

Rainha vermelha (em linha no `_RandWorld`; `_PlaceRedQueen`, `1030:67da`, tem a mesma lógica mas não tem chamadores no pseudocódigo):

```
x = 32; prof = Sk(4) + 7
para y = 1 até prof − 1:
    escava (x, y)
    t = x + S(3) − 1                     // −1, 0 ou +1
    se 7 < t < 57: x = t
escava (x, y); x += 1; y += 1            // dois passos em diagonal, para baixo e à direita
escava (x, y); x += 1; y += 1
escava (x, y)                            // câmara da rainha
_LastRedEgg = (x, y)
rx = x + 2
escava (rx, y), (rx − 1, y), (rx − 2, y)
lista R: (rx, y) com T = 0xE2 (rainha virada para a direita), M = 9, S = 0
lista R: (rx − 1, y) com T = 0xEA (segunda metade), M = 9, S = 0
_RedQueens += 1
```

A rainha preta é igual, com `_DigTileB`, lista B, T = 0x62 e 0x6A, `_BlkQueens += 1`, e com duas diferenças: a deriva lateral só é sorteada de novo quando `Sk(2) = 0` (senão repete a anterior, começando em 0); e grava a posição da câmara também em `_RebornX/_RebornY` (`1040:9fec/9fee`), onde a formiga amarela nasce. Ordem dos sorteios na rainha preta por linha: escava, `Sk(2)`, e só se der 0, `S(3)`.

"Escavar" é `_DigTileB/R(x, y)` (`1020:1fe4`/`21de`): se a célula é terra (0x20–0x2E), vira túnel com valor `Sk(8)` e soma (x, y) nos acumuladores de centro de massa; depois alisa as quatro vizinhas (`_SmoothEdges`, ver [tiles-valores.md](tiles-valores.md)) e atualiza o mapa de saída. Em `_DigTileB`, se `y > 53` e `S(64) = 0`, a célula vira 0x14 nos **dois** ninhos [hipótese: passagem entre os ninhos].

### 3.D Formiga amarela: `_InitYelloAnt` (`1030:6492`)

`_MeBase = 0`, `_MeEggCarried = 0xFD`, saúde 100. Fato visto no código:

- Experimental: plano 1 (superfície), `_MeMode = 2`, posição (0x40, 0x20), `_MeType = 0x10`. Sem sorteio.
- Full Game sem ninho (`_MeNestStarted = 0`): até 100 tentativas de `x = 0x20 + Sk(16) − Sk(16)`, `y = 0x20 + Sk(8) − Sk(8)` (nessa ordem de sorteio), até achar `_MapA < 0x10`; se nenhuma servir, usa (0x40, 0x20). Põe uma rainha alada (`_MeType = 0x40`) no plano 1, direção 2.
- Demais casos: operária (`_MeType = 0x10`) no plano 2 (ninho preto) em `(_RebornX, _RebornY)`, direção 2.

A formiga amarela marca `_LifeA/B` com 0xFF (cabeça) e 0xFE (segunda célula) em vez de entrar nas listas (`_SetMyLife`, `1020:6536`).

### 3.E Comida inicial: `_AddFood(−1, 0)` (`1030:6a58`)

```
cx = 0x40; cy = S(48) + 8; quantidade = 200       // com parâmetro ≥ 0: cx = Sk(128), cy = Sk(64)
_LastFoodDrop = (cx, cy)
raio = Sk(8) + 5
repetir quantidade vezes:
    a = Sk(256); r = S(raio)
    x = cx + trunc(cos(a) * r / 0x7FFF); y = cy + trunc(sin(a) * r / 0x7FFF)
    se (x, y) está no mapa e sem formiga em _LifeA:
        quintal: chão < 0x18 → 0x48; 0x48–0x4A → +1; conta _FoodA
        casa:    chão < 4 → (chão + 6) * 4; chão 4–0x17 → ((chão − 8) & ~3) + 0x18; 0x18–0x27 com (valor & 3) < 3 → +1; conta _FoodA
```

`sin`/`cos` usam a tabela de 64 palavras do objeto tipo 9 nº 1000 do banco (`_FracSineTab`, `1040:9fca`, palavras em big-endian, `sin(i·π/128)·32767`): ângulo de 0 a 255 para uma volta, valor 0x7FFF no ângulo 64, sinal negativo de 128 a 255; `cos(a) = sin(a + 64)`. A divisão é com sinal e trunca para zero. Fato visto no código; verificado no `QUICK2.ANT` (centro (64, 38), 117 unidades).

## 4. Patch do quintal: `_MakeYardPatch` (`1020:41a2`)

Fato visto no código e verificado no `QUICK2.ANT` (todas as células, exceto bichos que se moveram):

1. Se `_TERRAINset ≠ 0`: `_CurGndTileID = 1000` e `_OverlayTileSet(0, 1000)` (carrega os tiles do quintal, objeto tipo 9 nº 10, `_TERRAINset = 0`, `_Barrier = 0x50`).
2. **Chão:** para as 8.192 células em ordem de índice (x de 0 a 127, e dentro de cada x, y de 0 a 63): `_MapA = Sk(16)`.
3. **Pedras:** `_AddRocks` (4.1).
4. **Formigas-leão:** `_InitAntLions(Sk(4) + 1)` (4.2).
5. **Bichos-de-conta:** `_InitPillar` (4.3).
6. **Objetos** (posição `x = S(125) + 1`, `y = S(61) + 1`, sorteados nessa ordem; só grava se todas as células envolvidas forem `< 0x18`; sem nova tentativa):
   - 20 vezes, bloco 2 × 2: (x, y) = 0x20, (x+1, y) = 0x21, (x, y+1) = 0x22, (x+1, y+1) = 0x23;
   - 30 vezes, par horizontal: `v = (S(2) + 0x12) * 2` (0x24 ou 0x26; o `S(2)` só é sorteado se as duas células servirem), (x, y) = v, (x+1, y) = v + 1;
   - 100 vezes, célula 0x28;
   - 128 vezes, célula 0x51 (pedrinha que se carrega).

### 4.1 `_AddRocks` (`1020:4334`), `_AddRock5` (`1020:44ec`), `_AddRock3` (`1020:45ac`)

```
n = S(3) + 2                                    // 2 a 4
repetir n vezes:   para forma k = 0…5:  y = S(58); x = S(122); _AddRock5(x, y, k)
repetir 2n vezes:  para forma k = 0…5:  y = S(60); x = S(124); _AddRock3(x, y, k)
```

A ordem de sorteio é **y antes de x** (fato verificado na desmontagem: o segundo argumento é calculado e empilhado primeiro). Cada pedra é uma forma 5 × 5 (`1038:870a + 25·k`) ou 3 × 3 (`1038:87a0 + 9·k`), com a célula `(x + i, y + j)` recebendo `forma[j * N + i]` quando não é zero. Antes de gravar, se alguma célula não nula da forma cair sobre `_MapA > 0x10`, a pedra inteira é descartada. As formas (fato verificado: bytes do segmento 8; linha = j, coluna = i, `..` = 0):

| k | `_AddRock5` | `_AddRock3` |
|---|---|---|
| 0 | `.. 5b 54 55 ..` / `5b 5c 5c 56 ..` / `5a 5c 5c 5e 55` / `59 5d 5c 5c 56` / `.. 59 58 58 57` | `5b 54 55` / `5a 5c 56` / `59 58 57` |
| 1 | `.. .. 5b 54 55` / `.. 5b 5c 5c 56` / `5b 5c 5c 5f 57` / `5a 5f 58 57 ..` / `59 57 .. .. ..` | `5b 55 ..` / `59 5e 55` / `.. 59 57` |
| 2 | `.. .. 5b 54 55` / `5b 54 5c 5f 57` / `5a 5f 58 57 ..` / `59 57 .. .. ..` / `.. .. .. .. ..` | `5b 54 55` / `59 58 57` / `.. .. ..` |
| 3 | `.. .. .. .. ..` / `5b 54 54 54 55` / `59 5d 5c 5c 56` / `.. 59 5d 5c 56` / `.. .. 59 58 57` | `5b 55 ..` / `59 57 ..` / `.. .. ..` |
| 4 | `5b 54 54 54 55` / `59 5d 5c 5f 57` / `.. 5a 5c 56 ..` / `5b 5c 5f 57 ..` / `59 58 57 .. ..` | `5b 55 ..` / `5a 56 ..` / `59 57 ..` |
| 5 | `.. .. 5b 54 55` / `.. 5b 5c 5f 57` / `5b 5c 5c 56 ..` / `59 5d 5c 5e 55` / `.. 59 58 58 57` | `5b 55 ..` / `5a 5e 55` / `59 58 57` |

Os 32 bytes anteriores, em `_RockTab` (`1038:86ea`), não são usados por essas rotinas [em aberto: quem os usa].

### 4.2 `_InitAntLions(n)` (`1030:40c6`)

Fato visto no código: `_LionIndex = 0`, `n = min(n, 10)`, `_InitialLions = n`. Para cada formiga-leão, até 200 tentativas: `x = S(64) + S(65)`, `y = S(32) + S(33)` (distribuição triangular em torno do centro); aceita se `_IsClear3x3(1, x, y)`, ou, depois de 100 tentativas, se só a célula central estiver livre. Grava 0x38 no centro e, em cada vizinha livre (`_IsClearTile`), `0x30 + DGROUP:0x25e8[d]` com a tabela `1 2 4 7 6 5 3 0` (isto é, 0x30 canto superior esquerdo, 0x31 cima, 0x32 canto superior direito, 0x33 esquerda, 0x34 direita, 0x35 canto inferior esquerdo, 0x36 baixo, 0x37 canto inferior direito). Guarda a posição em `_LionListX/Y[_LionIndex]`, zera `_LionListM/S/T` e incrementa `_LionIndex` (até 9). `_AddRandAntLion` (`1030:4222`) faz o mesmo para uma formiga-leão nova durante o jogo.

### 4.3 `_InitPillar` (`1030:4bf8`)

Fato visto no código: zera a lagarta (`_PillarState/X/Y/Seg`, `__PillDir`) e `_PillarMap`. Só no quintal: duas tentativas (índices 2 e 1 das tabelas `_Sow*`): `x = S(128)`, `y = S(64)`; se `_MapA < 0x10`, sorteia `dir = S(8)`, guarda `_SowX/_SowY/_SowDir` e o chão original em `_SowSave`, e grava `_SowTab[dir]` (`1038:8a90` = `72 73 71 70 72 73 71 70`). Sem nova tentativa se a célula não servir. A lagarta não é criada aqui (no `QUICK2.ANT` ela aparece depois, com 0x69–0x6B).

## 5. Patch da casa: `_MakeHousePatch(n)` (`1020:3120`)

Fato visto no código, **não verificado** contra o original e só resumido aqui (lacuna):

- Se `_TERRAINset ≠ 1`: `_CurGndTileID = 1001` e `_OverlayTileSet(0, 1001)` (tiles da casa, objeto tipo 9 nº 9; `_TERRAINset = 1`; `_Barrier = 0x90`).
- `_InitAntLions(0)` e `_InitPillar()` (sem formigas-leão nem bichos-de-conta).
- Por número do patch `n = px * 16 + py` (0 a 37):

| n | Rotinas |
|---|---|
| 0 | `_MakeKitchenWall`, `_MakeSink` |
| 2, 3, 17, 18, 19, 33, 34, 35 | `_FloorTiles` (ladrilhos em xadrez de 16 × 16; `_DROPdir = 0`) |
| 4, 10, 11, 13, 14 | `_CarpetFloorL` (carpete com fiapos 0x3E/0x3F a 1/200 por célula, tomada `_MakeOutletH(2, 0x19)`; `_DROPdir = 1`) |
| 5, 6, 7 | `_CarpetFloorL` mais áreas preenchidas com 2 e objetos copiados de tabelas do DGROUP |
| 8, 12, 15 | `_CarpetFloorL` mais `_FillMapLegs(...)` (pés de móvel) e, em 8 e 15, objetos |
| 9 | toda a `_MapA` = 3; `_DROPdir = 2` |
| 20 a 31 | `_CarpetFloorR` |
| 36, 37 | toda a `_MapA` = 0; `_DROPdir = 2` |
| demais (1, 16, 32) | `_MakeKitchenWall` |

[em aberto: conteúdo de `_MakeKitchenWall`, `_MakeSink`, `_TileFrame1/2`, `_MakeOutlet*`, `_MakePlug*`, `_MakeKnob`, `_MakePenny`, `_MakeClip`, `_FillMap`, `_FillMapLegs` e das tabelas que eles copiam; quais patches (x, y) são a casa na tela]

## 6. Tutorial por arquivo: `_GenerateTutorial` (`1030:6262`)

Fato visto no código:

1. `_CurGameType = 1`; `_RandYard()` (gera como Quick Game).
2. `_AddBlackAnts(32)` e `_AddRedAnts(32)` (abaixo); `_FullCount()`.
3. `_CurGameType = 2` (o tutorial roda como Full Game); `_SetMapPlane(_MePlane)`.
4. Preenche 30 patches de `_YMapPopB/_YMapPopR` (índice `x * 16 + y`) com valores de quatro tabelas de 30 bytes no DGROUP (`0x25f0` = x, `0x260e` = y, `0x262c` = pretas, `0x264a` = vermelhas), cada população multiplicada por 32. Fato verificado (bytes do DGROUP):

(0,0) P64; (1,0) P128; (2,0) P192; (5,0) P160; (7,0) P224; (2,1) P128; (2,2) P64; (7,2) P64; (8,2) P96; (6,3) P192; (3,4) P32; (7,4) P224; (7,5) P128; (10,5) P160; (11,5) P96; (6,6) P32; (8,6) P32; (10,6) V128; (11,6) V192; (9,7) V96; (10,7) V128; (11,7) V192; (9,8) V160; (10,8) V192; (11,8) V96; (9,9) P160; (10,9) V128; (10,10) V96; (10,11) V64; (11,11) V160 (P = pretas, V = vermelhas).

Depois `_SpecialTutorialInit`/`_LoadGame` põem o título "Tutorial Game", chamam `_InitSimVars` e `_SeedSRand`.

`_AddBlackAnts(n)` (`1030:6c5a`): percorre x de 0 a 63 e, em cada x, y de 16 a 47; em cada célula com `_MapA < 0x50` e sem formiga, `S(10)`: se 0–3, T = 0x30 + `Sk(8)` com M = 2; senão T = 0x10 + `Sk(8)` com M = 4; adiciona à lista A. Para depois de `n` formigas ou com a lista A cheia (1.000). `_AddRedAnts(n)` (`1030:6cfe`) faz o mesmo de x = 127 descendo até 64, com T + 0x80. Também usadas pelas lições (`_GiveLesson`), `_AddSomeAnts` e `_MysteryButton`.

## 7. Rotinas pedidas que não fazem parte da geração

- `_InitWater` (`1020:0b76`): chamada quando começa a chover (`_SimRain`, `_DoSimYard`), não na geração. Coloca 100 gotas: `x = RRand(128)`, `y = RRand(64)` (gerador R), guardadas em `_RxTab/_RyTab`; se o chão for `< 0x0E`, vira poça 0x74 e zera/enfraquece os feromônios do bloco 2 × 2. Fato visto no código.
- `_InitGrassMap` (`1030:2096`): põe as 3 primeiras palavras de `_GrassMap` em 0 e as 9 seguintes em 0xFFFF. Não tem chamadores no pseudocódigo [em aberto: pode ser chamada por ponteiro].
- `_InitSpider`: ver 3.2.

## Como testar contra o original

1. **Regressão sem emulador (pronto):** `python re_trabalho/verifica_geracao_quick2.py` regera o patch a partir de `_YMapSeeds` do `QUICK2.ANT` e lista as diferenças. O port deve ter um teste equivalente em Rust (entrada: semente 0x3332, `_CurGameType = 1`, patch (11, 8); saída esperada: `_MapB`, `_MapR` e `_MapA` do save, ignorando as células de bichos).
2. **Novos saves na bancada QEMU** ([../../bancada-qemu.md](../../bancada-qemu.md)): gravar saves logo depois de gerar cada tipo de jogo (Tutorial, Full, Experimental), antes de qualquer passo de simulação se possível (pausar logo após o clique), e rodar a mesma comparação. Falta para isso: coordenadas dos botões Full/Experimental/Tutorial no roteiro e um roteiro por tipo.
3. **Patches da casa e Full Game:** num Full Game, levar a rainha a outro patch (ou carregar um save em patch da casa) e salvar; comparar `_MapA` com a reimplementação de `_MakeHousePatch` usando `_YMapSeeds` do save.
4. **Dump de memória:** `memoria` da bancada logo após o clique em "Quick Game" permite ler `_YMapSeeds` e os mapas sem passar pelo save, e conferir a sequência R (192 chamadas a `_RRand`).
