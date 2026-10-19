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

Script: `re_trabalho/verifica_geracao_quick2.py` (material de trabalho, fora do git). O port reproduz o mesmo resultado em Rust, com mais variáveis conferidas (ver "Implementação no port"). A geração da **casa** (`_MakeHousePatch`) foi verificada depois, em quatro patches gerados pelo `_XferPatch` do original (§5.3).

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

"Escavar" é `_DigTileB/R(x, y)` (`1020:1fe4`/`21de`): se a célula é terra (0x20–0x2E), vira túnel com valor `Sk(8)` e soma (x, y) nos acumuladores de centro de massa; depois alisa as quatro vizinhas (`_SmoothEdges`, ver [tiles-valores.md](tiles-valores.md)) e atualiza o mapa de saída da célula escavada (`_FixExitMapB/R`, `1020:284e`/`2914`: nas linhas 0 e 1, 0xFF se a célula é abertura 0x18 e 0xFE se não; abaixo, o maior `_ExitMap` das 8 vizinhas menos 1, ou 0 se todas são 0; as vizinhas não são recalculadas). O `_ExitMap` cresce, portanto, como uma distância até a superfície, mas só na ordem em que as células são escavadas. Fato visto no código; verificado no `QUICK2.ANT` (`_ExitMapR` idêntico, `_ExitMapB` igual exceto uma célula mudada pela simulação, ver "Implementação no port"). Em `_DigTileB`, se `y > 53` e `S(64) = 0`, a célula vira 0x14 nos **dois** ninhos [hipótese: passagem entre os ninhos].

### 3.D Formiga amarela: `_InitYelloAnt` (`1030:6492`)

`_MeBase = 0`, `_MeEggCarried = 0xFD`, saúde 100. Fato visto no código:

- Experimental: plano 1 (superfície), `_MeMode = 2`, posição (0x40, 0x20), `_MeType = 0x10`. Sem sorteio.
- Full Game sem ninho (`_MeNestStarted = 0`): até 100 tentativas de `x = 0x20 + Sk(16) − Sk(16)`, `y = 0x20 + Sk(8) − Sk(8)` (nessa ordem de sorteio), até achar `_MapA < 0x10`; se nenhuma servir, usa (0x40, 0x20). Põe uma rainha alada (`_MeType = 0x40`) no plano 1, direção 2.
- Demais casos: operária (`_MeType = 0x10`) no plano 2 (ninho preto) em `(_RebornX, _RebornY)`, direção 2.

A formiga amarela marca `_LifeA/B` com 0xFF em vez de entrar nas listas (`_SetMyLife`, `1020:6536`); só quando o tipo é 0x60 (rainha) ela ocupa também a célula atrás (direção oposta) com 0xFE. A operária e a rainha alada do novo jogo ocupam uma célula. No Experimental, `_InitYelloAnt` não chama `_SetMyLife`: nenhuma célula de `_LifeA` é marcada. Se a célula do ninho for terra ou superfície fechada, `_SetLife` a escava (`_DigMyTile`, não especificado); na geração isso não acontece, porque a formiga nasce na câmara da rainha. Fato visto no código.

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

Fato visto no código: zera a lagarta (`_PillarState/X/Y/Seg`, `__PillDir`) e `_PillarMap`. Só no quintal: coloca dois bichos-de-conta (índices 2 e 1 das tabelas `_Sow*`, nessa ordem): `x = S(128)`, `y = S(64)`; se `_MapA < 0x10`, sorteia `dir = S(8)`, guarda `_SowX/_SowY/_SowDir` e o chão original em `_SowSave`, grava `_SowTab[dir]` (`1038:8a90` = `72 73 71 70 72 73 71 70`) e passa ao índice seguinte. **Se a célula não servir, sorteia de novo para o mesmo índice**, até colocar os dois (fato visto na desmontagem de `1030:4c4e`–`4cd5`: o contador `si` só diminui, `sub si, 2`, depois de gravar; correção de uma versão anterior desta spec, que dizia "sem nova tentativa"; no `QUICK2.ANT` as duas primeiras tentativas serviram, então o save não distingue as duas leituras). A lagarta não é criada aqui (no `QUICK2.ANT` ela aparece depois, com 0x69–0x6B).

## 5. Patch da casa: `_MakeHousePatch(n)` (`1020:3120`)

Fato visto no código (pseudocódigo de `Code5.c`; laços `rep stosw` de `_CarpetFloorR` e `_MakeSink` conferidos na desmontagem) e **verificado no original** em quatro patches (§5.3). Os 38 patches `n = px * 16 + py ≤ 37` são as colunas `px` = 0 e 1 inteiras e `px` = 2 com `py` ≤ 5.

Coordenadas: `(x, y)` de `_MapA` (128 × 64, índice `x * 64 + y`; o `_MapA` fica em `DGROUP:0x28e8`, e o endereço de cada escrita do código foi convertido para (x, y) por `(endereço − 0x28e8) = x * 64 + y`). Intervalos `a..b` são inclusivos.

1. Se `_TERRAINset ≠ 1`: `_CurGndTileID = 1001` e `_OverlayTileSet(0, 1001)` (tiles da casa, objeto tipo 9 nº 9; `_TERRAINset = 1`; `_Barrier = 0x90`).
2. `_InitAntLions(0)` (nenhuma formiga-leão; `_InitialLions = 0`; sem sorteio) e `_InitPillar()` (zera lagarta e bichos-de-conta; como `_TERRAINset = 1`, não põe bichos-de-conta nem sorteia).
3. Monta o chão conforme `n` (todas as receitas escrevem as 8.192 células, então nada do patch anterior sobra):

| n | Receita (rotinas em §5.1) |
|---|---|
| 0 | `_MakeKitchenWall`, `_MakeSink` |
| 1, 16, 32 (caso `default`) | `_MakeKitchenWall` |
| 2, 3, 17, 18, 19, 33, 34, 35 | `_FloorTiles` |
| 4, 10, 11, 13, 14 | `_CarpetFloorL` |
| 5 | `_CarpetFloorL`; x 24..70, y 20..63 = 2; moeda em (40, 45) |
| 6 | `_CarpetFloorL`; x 24..70, y 0..63 = 2; moeda em (32, 20); clipe em (38, 55) |
| 7 | `_CarpetFloorL`; x 24..70, y 0..20 = 2; clipe em (42, 10) |
| 8 | `_CarpetFloorL`; `_FillMapLegs(0x18, 0x36, 0x0F, 0x2D, 2)`; clipe em (28, 34) |
| 9 | toda a `_MapA` = 3; `_DROPdir = 2` |
| 12 | `_CarpetFloorL`; `_FillMapLegs(0x2D, 0x5F, 8, 0x2E, 2)` |
| 15 | `_CarpetFloorL`; `_FillMapLegs(0x32, 100, 10, 0x2D, 2)`; moeda em (75, 34) |
| 20 a 31 | `_CarpetFloorR` |
| 36, 37 | toda a `_MapA` = 0; `_DROPdir = 2` |

"Moeda" é `_MakePenny` e "clipe" é `_MakeClip` (o código copia as formas em linha, com os mesmos endereços de tabela). `_DROPdir` (`9:9c66`, direção para onde a comida largada cai na casa; não está no save) é gravado pelas rotinas: 0 nos ladrilhos, 1 no carpete esquerdo, 2 na cozinha e nos patches 9/36/37, 3 no carpete direito.

### 5.1 Rotinas

**Formas** (`DGROUP`, bytes do executável; célula `(x + i, y + j)` recebe `forma[j * largura + i]`):

| Forma | Endereço | Largura × altura | Bytes (linha j por linha) |
|---|---|---|---|
| tomada vertical (`_MakePlugV`) | `1048:2314` | 5 × 4 | `6b 6c 6c 6c 6d / 71 78 64 78 72 / 71 79 74 79 72 / 6e 6f 6f 6f 70` |
| tomada horizontal (`_MakePlugH`) | `1048:2328` | 4 × 5 | `6b 6c 6c 6d / 71 76 77 72 / 71 64 64 72 / 71 76 77 72 / 6e 6f 6f 70` |
| botão (`_MakeKnob`) | `1048:233c` | 5 × 5 | `40 41 41 41 42 / 43 4e 4e 4e 44 / 43 4e 4c 4e 44 / 43 4e 4e 4e 44 / 46 47 47 47 49` |
| moeda (`_MakePenny`) | `1048:2356` | 3 × 3 | `28 29 2a / 2b 2c 2d / 2e 2f 30` |
| clipe (`_MakeClip`) | `1048:2360` | 3 × 3 | `00 31 32 / 33 34 35 / 36 37 00` — **zeros não são copiados** |

Entre as formas há um byte 0 (`1048:2355`, `1048:235f`).

**`_FillMap(x1, x2, y1, y2, v)`** (`1020:3f54`): retângulo `x1..x2 × y1..y2` = `v`. Não é chamada pela casa (os casos 5–7 fazem o mesmo em linha).

**`_TileFrame1(x1, x2, y1, y2)`** (`1020:3944`), nesta ordem: linha `y1` (x de x1 a x2) = 0x54; linha `y2` = 0x51; coluna `x1` (y de y1 a y2) = 0x5A; coluna `x2` = 0x5B; cantos `(x1, y1)` = 0x53, `(x2, y1)` = 0x55, `(x1, y2)` = 0x50, `(x2, y2)` = 0x52.

**`_TileFrame2(x1, x2, y1, y2)`** (`1020:3aa2`): igual, com linha `y1` = 0x51, linha `y2` = 0x54, coluna `x1` = 0x5B, coluna `x2` = 0x5A e cantos 0x56, 0x58, 0x57, 0x59.

**`_MakeOutletV(x, y)`** (`1020:3c00`): `x..x+8 × y..y+12` = 0x63; `_TileFrame1(x, x + 8, y, y + 12)`; tomada vertical em `(x + 2, y + 2)` e em `(x + 2, y + 7)`; `(x + 4, y + 6)` = 0x65 (parafuso).

**`_MakeOutletH(x, y)`** (`1020:3d44`): `x..x+12 × y..y+8` = 0x63; `_TileFrame1(x, x + 12, y, y + 8)`; tomada horizontal em `(x + 2, y + 2)` e `(x + 7, y + 2)`; `(x + 6, y + 4)` = 0x65.

**`_FloorTiles`** (`1020:3406`), sem sorteio, `_DROPdir = 0`:
1. toda célula: 1 se `(x >> 4) + (y >> 4)` é par, senão 0;
2. linhas `y` = 0, 16, 32, 48 (todas as colunas): 0x61 se `(x + y) & 0x10 = 0`, senão 0x60;
3. colunas `x` = 0, 16, …, 112 (todas as linhas): se a célula vale menos que 2 (não é junta de linha), 0x5F se `(x + y) & 0x10 = 0`, senão 0x5E; se é junta de linha (cruzamento), 0x5C ou 0x5D com o mesmo teste.

**`_CarpetFloorL`** (`1020:34c2`), `_DROPdir = 1`: x 0..20 = 0x64 (parede); x 21 = 0x7A; x 22 = 0x7B (rodapé); x 23..127 = 0x03 (carpete); **fiapos**: para x de 23 a 127 e, dentro, y de 0 a 63, `S(200)`; se deu 0, a célula recebe `0x3E + S(2)`; depois `_MakeOutletH(2, 0x19)`.

**`_CarpetFloorR`** (`1020:35e2`), `_DROPdir = 3`: x 0..105 = 0x03; fiapos em x 0..105 (mesma regra); x 106 = 0x7C; x 104..105 = 0x7B; x 106..127 = 0x64 — **a coluna 106 recebe 0x7C e logo depois 0x64** (conferido na desmontagem `1020:3639`–`367b`; não é leitura errada do Ghidra) —; depois `_MakeOutletH(0x71, 0x23)`. [hipótese] Defeito: a intenção seria o espelho do carpete esquerdo (0x7C na coluna 106).

**`_MakeLint2(x1, x2, y1, y2)`** (`1020:3578`): os mesmos fiapos num retângulo; não é chamada pela casa.

**`_MakeKitchenWall`** (`1020:3698`), `_DROPdir = 2`, sem sorteio: y 0..23 = 0x62 (azulejo) e y 24..63 = 0 (bancada), em todas as colunas; linhas y = 0, 8 e 16 = 0x68 (juntas); nas colunas x = 0, 8, …, 120, para y 0..23: 0x62 vira 0x66 e qualquer outro valor (a junta) vira 0x67; na linha 23, em todas as colunas: 0x62 vira 0x68 e outro valor vira 0x69; depois `_MakeOutletV(0x24, 2)` e `_MakeOutletV(0x54, 2)`.

**`_MakeSink`** (`1020:375c`), sem sorteio, nesta ordem: x 37..91 × y 26..56 = 0x01; x 41..68 × y 33..53 = 0xC2; x 42..68 × y 35..53 = 0xC1; `_TileFrame2(0x28, 0x44, 0x20, 0x35)`; x 73..88 × y 33..53 = 0xC2; x 74..88 × y 35..53 = 0xC1; `_TileFrame2(0x48, 0x58, 0x20, 0x35)`; `_TileFrame1(0x25, 0x5B, 0x1A, 0x38)`; botões em (44, 27) e (60, 27); `(62, 29)` = 0x4D (o centro do segundo botão: "H" e "C"); botões em (52, 27) e (52, 36); x 52..56 × y 29..38 = 0x4E; coluna 52 (y 29..38) = 0x43; coluna 56 (y 29..38) = 0x44; `(52, 40)` = 0x45 e `(56, 40)` = 0x48 (torneira); x 54..56 × y 41..42 = 0xC2; x 57..58 × y 35..42 = 0xC2.

**`_FillMapLegs(x1, x2, y1, y2, v)`** (`1020:3faa`): retângulo = `v`; para x de x1 a x2 e, dentro, y de y1 a y2, `S(20)`; se deu 0, a célula recebe `0x38 + S(5)`; depois os quatro cantos 4 × 4 = 0xC0 (pés do móvel): `(x1..x1+3, y1..y1+3)`, `(x2−3..x2, y1..y1+3)`, `(x1..x1+3, y2−3..y2)`, `(x2−3..x2, y2−3..y2)`.

**Sorteios:** só os fiapos dos carpetes e o miolo de `_FillMapLegs` sorteiam, sempre com x por fora e y por dentro, e o segundo sorteio só acontece quando o primeiro dá 0. A cozinha, a pia, os ladrilhos e os patches 9, 36 e 37 não sorteiam. Depois do chão, o resto de `_RandWorld` (§3, passos 4 em diante) segue igual ao do quintal, com os ramos "casa" já descritos (linha da superfície dos ninhos no passo 6, `_CanBeHouseHole` em 3.A, comida em 3.E).

### 5.2 No port

`engine/src/rules/casa.rs` executa as receitas de `data/casa.json`, gerado por `tools/extrair_casa.py` (formas lidas do `SIMANTW.EXE`; receitas escritas no script a partir desta seção, com o endereço de cada rotina). Cada patch é uma lista de operações (`preencher`, `celula`, `sortear`, `moldura`, `trocar`, `forma`, `ladrilhos`, `queda`, `rotina`); as rotinas acima são entradas de `rotinas`. Um mod pode redesenhar qualquer cômodo trocando a receita ou as formas; o carregamento recusa operações fora do mapa, `S(0)` e rotinas em laço. `Mundo::gerar_patch` chama a casa quando `n ≤ 37` e guarda `_DROPdir` em `Mundo::queda` e o passo da queda em `Mundo::passo_queda`, que vai para `Estado::passo_queda` quando o patch é aplicado (a comida que cai na casa: superficie.md 8.2).

### 5.3 Verificação no original (bancada QEMU, 2026-10-17)

Partindo do snapshot `quick` (Quick Game), com a VM rodando: `_CurGameType` escrito como 2 (só o Jogo Completo transfere), `_YMapPopB[n]` = 1 no patch de destino (sem colônia preta lá, `_XferPatch` recusa com a mensagem "Sorry, there is not a black colony at this location"), clique no botão Yard da faixa (37, 55), duplo clique no patch na Yard View (`(x, y)` de tela ≈ `(298 + 28·px − 10·py, 227 + 10·py)`), espera, `pausar` e `estado` com o diálogo "You are transferring to this place in the house." aberto. Patches (0, 0) (cozinha e pia), (0, 4) (carpete esquerdo, fiapos), (0, 12) (carpete com mesa, `_FillMapLegs`) e (1, 4) (carpete direito): regerando com a semente `_YMapSeeds[py * 12 + px]` do próprio estado, pretas = `_YMapPopB[n]` (7 nas quatro execuções: a colônia escrita cresceu até o clique) e vermelhas = `_YMapPopR[n]` (0), `_MapA`, `_MapB`, `_MapR`, `_ExitMapB` e `_ExitMapR` saem **idênticos** (8.192 e 4 × 4.096 bytes), e `_TERRAINset` = 1. Isso confere também, na casa, a linha da superfície dos ninhos (com `py ≤ 3` e `> 3`), `_MakeNewHoleB` com `_CanBeHouseHole`, `_DigOutBNest` (pretas 7 → 8 > 1), as formigas pretas da superfície e `_AddFood`. Estados em `tests/referencias/saves/CASA_0_0.ANT`, `CASA_0_4.ANT`, `CASA_0_12.ANT`, `CASA_1_4.ANT`; teste `engine/tests/geracao_casa.rs` (`casa_igual_ao_original_depois_de_xfer_patch`). Roteiros: `tests/referencias/roteiros/qemu_casa_0_0.txt`, `qemu_casa_0_4.txt`, `qemu_casa_0_12.txt`, `qemu_casa_1_4.txt` (rodar com `--overlay` numa cópia do overlay com o snapshot `quick`).

Não verificados no original: os patches 5–7, 9, 15, 36/37 e a cozinha sem pia (seguem as mesmas rotinas e tabelas já conferidas). `_DROPdir` (fora do save, gravado nos extras dos vetores): 1 no carpete esquerdo, 3 no direito, 2 na cozinha (0, 1) e 0 nos ladrilhos (0, 2), lidos na bancada (2026-10-19); a cozinha e os ladrilhos foram gerados pelo `_XferPatch` do original nos vetores `queda_casa_*` (superficie.md 8.2).

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

## Implementação no port

Onde está:

- `engine/src/rules/rng.rs`: `GeradorS` (LFSR, `_SRand1`, máscaras, `_SGRand`/`_SGIRand`/`_SGSRand`, semente pelo relógio) e `GeradorR` (`rand()` do Microsoft C, `_RRand`, `_SeedRRand`). Estado explícito em cada valor, nada global. Ver [rng.md](rng.md).
- `engine/src/rules/geracao.rs`: `Mundo::novo_jogo` (`_RandYard`: sementes dos 192 patches com R e geração do patch (11, 8)) e `Mundo::gerar_patch` (`_RandWorld` completo para o quintal: `_InitSpider`, `_MakeYardPatch` com `_AddRocks`, `_InitAntLions`, `_InitPillar` e objetos, linha da superfície, formigas da superfície, `_MakeNewHoleB/R`, `_DigOutBNest/RNest`, `_BuildAntListA`, rainhas, `_InitYelloAnt`, `_AddFood`). Produz `_MapA/B/R`, `_ExitMapB/R`, `_LifeA/B/R`, as listas A/B/R, `_HoleMapB/R`, `_LastNewHole?`, acumuladores de escavação, `_FoodA`, `_LastFoodDrop`, `_LastBlackEgg/_LastRedEgg`, `_RebornX/Y`, formigas-leão, bichos-de-conta, formiga amarela e aranha.
- Tabelas: `data/geracao.json` (`_Dx8`/`_Dy8`, bordas do buraco e da formiga-leão, `_SowTab`, formas de pedra 5 × 5 e 3 × 3), gerado por `python tools/extrair_geracao.py SIMANTW.EXE --json data/geracao.json` (endereços no topo do script). A tabela de seno é lida do banco em tempo de execução (objeto tipo 9 nº 1000 do `SHARED`). Um mod pode trocar `geracao.json` inteiro (por exemplo, outras formas de pedra) ou o objeto 9/1000.

Verificado (teste `engine/tests/geracao_quick2.rs`, roda com `SIMANT_DIR` apontando para a pasta do jogo; sem ela, avisa e passa sem verificar): com a semente `_YMapSeeds[8 * 12 + 11] = 0x3332` do `QUICK2.ANT`, Quick Game, uma colônia de cada cor, patch (11, 8):

- `_MapB` e `_MapR`: idênticos byte a byte (4.096 de 4.096 cada).
- `_MapA`: 8.182 de 8.192 células idênticas. As 10 diferenças são exatamente as de bichos que andaram nos 78 ciclos entre a geração e o save, listadas no teste: as duas posições onde a geração pôs os bichos-de-conta, (55, 27) e (37, 10), que no save voltaram ao chão de `_SowSave`; as posições atuais deles no save, (32, 46) e (49, 8) (iguais a `_SowX/_SowY`); a lagarta em (3…7, 58); e um 0x72 em (0, 0) [hipótese: a rotina que move os bichos-de-conta também processa o índice 0 das tabelas `_Sow*`, que é zerado, e desenha um bicho em (0, 0)].
- `_ExitMapR` idêntico; `_ExitMapB` igual exceto (36, 9), onde a rainha preta foi criada (save 244, geração 0) [hipótese: a simulação recalcula a saída por onde a rainha passa; no save ela já se virou e está com a cabeça em (34, 9)].
- `_HoleMapB/R`, `_LastNewHoleB/R` (par x, coluna), `_TileTotX/Y?`, `_TilesDug?`, `_TileMassX/Y?`, `_WaterTable`, `_FoodA = 117`, `_LastFoodDrop = (64, 38)`, `_LastRedEgg`, `_RebornX/Y`, `_InitialLions`, `_LionIndex`, `_LionListX/Y`, `_BlkQueens`, `_RedQueens`, `_MePlane/_MeLocX/_MeLocY/_MeType/_MeDir`, `_ListIndexA` e as duas primeiras entradas da lista R: iguais ao save.
- Lista B: a rainha preta é criada em (36, 9) com T = 0x62 e a metade em (35, 9) com 0x6A; no save ela está virada (0x66 em (34, 9), 0x6E em (35, 9)), e `_LastBlackEgg` já mudou para (36, 9) por causa dos ovos postos. Isso é simulação, não geração.

Não verificado contra o original (implementado conforme o código, sem caso de teste real): `_DigOutBNest/RNest` (só acontece com mais de uma formiga por colônia, em `_XferPatch`), os ramos de Tutorial, Full Game e Experimental de `_InitYelloAnt`/`_RandWorld`, `_AddFood` com parâmetro ≥ 0 e `_MakeNewHoleR` na casa. (`_DigOutBNest`, `_MakeNewHoleB`, `_AddFood(−1)` e a linha da superfície dos ninhos na casa foram conferidos em §5.3.) A sequência R de `novo_jogo` (192 chamadas a `_RRand`) segue a spec, mas o estado de R no momento do clique não está no save.

Fora do escopo desta implementação (lacunas): `_XferPatch` (o estado do jogo; a geração do patch é `Mundo::gerar_patch`), feromônios, `_ClearHistory`, `_CountAnts`, `_ResetYellowVars`, vistas (`_MapPnt`, `_AMapPnt`...), `_DigMyTile` (a formiga amarela nascendo em terra devolve erro) e as demais variáveis zeradas por `_RandYard` que não são estado do mapa.

## Como testar contra o original

1. **Regressão sem emulador (pronto):** `python re_trabalho/verifica_geracao_quick2.py` regera o patch a partir de `_YMapSeeds` do `QUICK2.ANT` e lista as diferenças. O equivalente no port é `engine/tests/geracao_quick2.rs` (entrada: semente 0x3332, `_CurGameType = 1`, patch (11, 8); saída esperada: `_MapB`, `_MapR` e `_MapA` do save, com as 10 células de bichos listadas): `SIMANT_DIR=<pasta do jogo> cargo test -p engine --test geracao_quick2`.
2. **Novos saves na bancada QEMU** ([../../bancada-qemu.md](../../bancada-qemu.md)): gravar saves logo depois de gerar cada tipo de jogo (Tutorial, Full, Experimental), antes de qualquer passo de simulação se possível (pausar logo após o clique), e rodar a mesma comparação. Falta para isso: coordenadas dos botões Full/Experimental/Tutorial no roteiro e um roteiro por tipo.
3. **Patches da casa e Full Game:** feito para a casa (§5.3: `_CurGameType` = 2 e uma colônia escrita na memória, duplo clique na Yard View). Falta repetir para patches do quintal com várias colônias e para os demais cômodos.
4. **Dump de memória:** `memoria` da bancada logo após o clique em "Quick Game" permite ler `_YMapSeeds` e os mapas sem passar pelo save, e conferir a sequência R (192 chamadas a `_RRand`).
