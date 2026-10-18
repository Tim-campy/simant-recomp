# Quintal: menino, cortador, gato, cachorro, pássaro, chuva, água e colônias

Este documento descreve `_DoSimYard` (`1030:14a8`), chamado em todo tick (passo 6 de [simulacao-visao-geral.md](simulacao-visao-geral.md) §5), com tudo o que ele chama, e `_DoWater` (`1020:07c6`, passo 7). Cobre o menino (dentro e fora de casa), o cortador de grama, o pé e o inseticida, o gato, o cachorro, o pássaro, a chuva, a água e o mapa estratégico de colônias (`_SimColonies`).

## Convenção de evidência

- **Fato visto no código:** pseudocódigo do Ghidra (`re_trabalho/decomp/Code7.c`, `Code5.c`), com endereço `seletor:offset` e nome do `SIMANTW.SYM`. Onde o pseudocódigo era ambíguo, a desmontagem do `SIMANTW.EXE` (capstone) e as relocações NE do DGROUP decidiram; o texto avisa.
- **Verificado no original:** caso rodado na bancada QEMU ([../../bancada-qemu.md](../../bancada-qemu.md)), com o arquivo do caso indicado.
- **[hipótese]** / **[em aberto]:** como no [README](../README.md).

Sorteios: `S(n)` = `_SRand1(n)`, `S2()`, `S4()`, `S8()`, `S16()`, `S32()`, `S64()`, `S128()` = máscaras, `SG(n)` = `_SGSRand(n)` (três passos), `R(n)` = `_RRand(n)` (gerador R). Ver [rng.md](rng.md). **A ordem dos sorteios é a ordem em que aparecem nos pseudocódigos abaixo**; um sorteio dentro de uma condição só acontece se as condições à esquerda forem verdadeiras (avaliação em curto-circuito, como no C).

Coordenadas: o quintal tem 12 × 16 patches, `(px, py)` com `px` 0–11 e `py` 0–15; índice nos mapas `_YMapPop*` = `px * 16 + py`. O patch atual é `_CurYardPnt` = (`9:7a60`, `9:7a62`). Os bichos do quintal têm uma posição em pixels da vista isométrica do quintal, `(X, Y)`, e o patch sai dela:

```
px = (X + Y − 200) / 28        py = (Y − 38) / 10           (divisão inteira truncada para zero)
```

## 1. Variáveis

Todas no save, exceto as marcadas. Endereços do `.SYM`; campos do port em `rules/estado.rs` (`Menino`, `Bichos`, `Ambiente`, `Colonia`).

| Símbolo | Endereço | Uso |
|---|---|---|
| `_YardCycle` | `9:7a5c` | contador do quintal, 0..0x3FF |
| `_BoyX/_BoyY`, `_BoyPx/_BoyPy` | `8:…`/`9:78b0`/`9:78d2` | posição em pixels e patch do menino |
| `_BoyDir` | `8:8a72` | direção 0 = py − 1, 1 = px + 1, 2 = py + 1, 3 = px − 1 |
| `_BoyHere` | — | estado do menino lá fora (0 dentro de casa, 1–5, seção 3) |
| `_BoyTurnCnt`, `_BoyStandCnt`, `_BoyWait` | `8:8a74`, `8:8a80`, `8:8a70` | contagens de giro e de parada; `_BoyWait` só é iniciado |
| `_BoyFrame` | — | quadro de desenho |
| `_NodeNum`, `_NodeCnt` | `8:8a86`, `8:8a84` | nó do roteiro de dentro de casa e passo dentro do nó; `_NodeNum` ≥ 0x26 = lá fora |
| `_BoyIsMowing` | `9:7c92` | cortando grama |
| `_LastMowX/Y`, `_MowX/Y` | `9:9bbe`, `9:9bd4`, `9:9af4`, `9:9afc` | último patch cortado; posição do cortador no patch atual |
| `_GrassMap` | `9:a0b6` | 12 palavras: bit `py` da palavra `px` = grama ainda por cortar |
| `_FootHere`, `_FootX/Y`, `_FootTog` | `8:8a7e`, `9:7a4e`, `9:7a58`, `8:8a82` | pé no patch atual, posição em células e alternância esquerdo/direito |
| `_BoyMessOn`, `_BoyMsgOffset`, `_BoyMsgCnt` | `9:99da`, `9:72f2`, `9:7d4a` | balão de fala do menino (interface) |
| `_Cat*`, `_Dog*`, `_Bird*` | `9:…` | gato, cachorro e pássaro |
| `_RainOn`, `_RainCnt`, `_ClearRainFlag`, `_RxTab/_RyTab`, `_WaterTable` | `9:…`, `9:810c`, `8:8a66`, `9:79e6`, `9:7a72`, `9:99d8` | chuva e água |
| `_YMapPopB/R` | `8:00a4`, `8:0164` | população de cada colônia do quintal (0 = nenhuma; teto 250) |
| `_SwarmCntB/R`, `_QueenStorageB/R`, `_SwarmDelayB/R` | `9:807a`, `9:9c26`, `10:ac8c`, `10:ac8e`, `9:99e6`, `9:9b30` | revoada: tamanho, rainhas guardadas, atraso |
| `_ColonyTotalBlack/Red`, `_HouseColonies`, `_ColonyUpdateFlag` | `10:ac90`, `10:ac92`, `9:9b22`, — | contagens da última passada |
| `_LastColonyPopB/R` | `9:72c0`, `9:8118` | **fora do save**: `_ColonyTotal*` da passada anterior |
| `_gameCycles` | `9:9b2c` | **fora do save**: voltas do laço do temporizador (relógio do pássaro) |
| `_MigrateX/Y` | — | **fora do save**: origem de uma migração (seção 10.4) |

Relógios. Três rotinas usam relógio de parede, não ticks (fato visto no código): o gato e o latido do cachorro usam `_MacTickCount` (1/60 s); o pássaro usa `_gameCycles`; o balão do menino guarda `_MacTickCount + 300`.

**De onde vem `_MacTickCount`** (fato visto no código e nos bytes do `SIMANTW.EXE`, 2026-10-17): não é variável, é uma função. `_MacTickCount` (`1000:9694`, segmento de código 1) chama `_TickCount` (`1008:18ac`), que faz `GETTICKCOUNT()` (USER do Windows: milissegundos desde o início do Windows, com resolução do timer de 55 ms) e divide por 55 (`__aFuldiv`, sem sinal) — isto é, conta tiques de 18,2 Hz —, e devolve o resultado × 3 em DX:AX (32 bits; `mov cx,ax / mov bx,dx / shl ax,1 / rcl dx,1 / add ax,cx / adc dx,bx / retf`, RETF em `1000:96a5`). Logo **relógio do Mac = ⌊ms do Windows / 55⌋ × 3 ≈ 54,5 por segundo, em degraus de 3**: tempo real, independente do tick e de `_gameCycles`. Todas as leituras de relógio dentro do tick passam por ela (`_SimCat`, `_SimDog`/`_MakeBark`, `_SimKidOutside`/`_SimKidInside`/`_SendBoyMsg`, `_FollowBoyDir`, `_GetMowDir`, `_SimRain`, `_DoSimYard`, `_Feedback`, `_AnimYellowFight`/`_AnimYellowInsane`); `_TickCount` direto só aparece fora do tick (sons, diálogos, `_SeedSRand`/`_SeedRRand`). Medido na bancada QEMU (velocidade normal): o relógio anda 9 por tick (3333, 3621, 3909 nos ticks 159, 191, 223; 18885 → 18966 nos ticks 1888–1897), ou seja ~165 ms de VM por tick no TCG. Consequência: **o consumo de sorteios do gato e do latido depende do tempo real**, e o original deixa de ser determinístico tick a tick quando o gato está fora de cena (`_CatOn = 0`) ou o cachorro pode latir — duas execuções a partir do snapshot `quick` divergiram em S já no tick 191. Os vetores de tick gravam o valor lido (`extras.json`, `MacTickCount`). Os atrasos guardados no save (`_CatDelay`, `_DogBarkDelay`, `_BirdDelay`, `_BoyMsgCnt`) são valores absolutos desses relógios. Com o gato e o cachorro, isso faz o **consumo de sorteios** depender do relógio (o `S16` do gato e o `S(30)` do latido só acontecem quando o atraso venceu). [Verificado em [testes-tick.md](testes-tick.md)]: na velocidade normal, `_gameCycles` anda centenas por tick, de modo que o teste do pássaro dá verdadeiro em todo tick; só o valor de `_BirdDelay` difere entre execuções.

## 2. `_DoSimYard` (`1030:14a8`), todo tick

**Fato visto no código.**

```
_YardCycle += 1; se > 0x3FF: _YardCycle = 0
se _NodeNum < 0x26: _SimKidInside()  senão: _SimKidOutside()        (seções 4 e 3)
_SimBird()                                                          (seção 7)
_SimCat()                                                           (seção 6)
se _ForSaleState == 0: _SimDog()                                    (seção 5)
se _TERRAINset ≠ 1:                                                 (chuva; nada na casa)
    se _RainOn == 0:
        se _GlobalKey == 0xA8 ou (R(32) == 0 e R(128) == 0):        (R(128) só se R(32) deu 0)
            _RainOn = 1; _RainCnt = S(150) + 150
            _InitWater()                                            (100 × _PlaceDrop: seção 9.2)
            balão do menino nº 0 (_BoyMsgCnt = relógio + 300, _BoyMessOn = 1, _BoyMsgOffset = 0)
    senão:
        se _RainCnt > 0: _RainCnt −= 1
        se _RainCnt < 1: _RainOn = 0
_SimColonies()                                                      (seção 10)
```

`_SimRain` (`1030:15ea`) tem o mesmo código da chuva, mas **não tem chamador**: o `_DoSimYard` traz o corpo embutido. Também sem chamador: `_SendBoyMsg` (`1030:15ae`, embutido como "balão do menino"), `_GetMowDir` (`1030:1f5c`), `_NotMowed` (`1030:203e`), `_IsValidYard` (`1030:2072`), `_InitGrassMap` (`1030:2096`), `_FollowBoyDir` (`1030:316e`), `_FollowCatDir` (`1030:32a6`), `_MakeBark` (`1030:32ea`), `_MowerFall` (`1030:34da`), `_MaintainSwarm` (`1030:3580`), `_GetNearbyPatches` (`1030:3ce4`), `_Reproduce` (`1030:3d4c`), `_StartMigrate` (`1030:3df2`) e `_EndMigrate` (`1030:3e6c`). Todas, exceto as duas de migração, aparecem embutidas nas rotinas abaixo, com o mesmo código (conferido no pseudocódigo).

"Balão do menino nº k" (corpo de `_SendBoyMsg`): se k < 23 (no `_SendBoyMsg`; os pontos embutidos já garantem isso), `_BoyMsgCnt = relógio + 300`, `_BoyMessOn = 1`, `_BoyMsgOffset = k`. O texto é a entrada k da lista de strings carregada por `_LoadStringAnt(1200)` em `_initStuff` (`9:9e88`), desenhada por `_UpdateYardMessage` ([hipótese]: lista SHARED 4/1200). Não afeta a simulação.

## 3. Menino lá fora: `_SimKidOutside` (`1030:169c`)

**Fato visto no código.** Tabelas em `data/quintal.json` (`menino`, seção 12).

```
se _FootHere ≠ 0 e (_YardCycle & 3) ≠ 0: FIM               ← no patch atual, anda 1 tick em cada 4

(escolha da direção)
se _BoyIsMowing == 0:
    _BoyTurnCnt −= 1
    se _BoyTurnCnt < 0: _BoyDir = (S(3) + _BoyDir − 1) & 3; _BoyTurnCnt = 4
senão se (_LastMowX, _LastMowY) ≠ (_BoyPx, _BoyPy):         (entrou num patch novo: _GetMowDir embutido)
    para k = 3, 2, 1, 0:
        (vx, vy) = (_BoyPx + vizinho_dx[k], _BoyPy + vizinho_dy[k])
        se (vx, vy) está no quintal e o bit vy de _GrassMap[vx] está ligado:
            desliga o bit; _BoyDir = k; vai para ESTADO
    se _BoyPx ≥ 11: _BoyDir = 0
    senão se _BoyPy ≥ 15: _BoyDir = 1
    senão se _BoyPy < 1: balão nº 6; _BoyHere = 4; _BoyDir = 2
    senão: _BoyDir = _YardCycle & 3

ESTADO (por _BoyHere):
1 (passeando):
    se S(900) == 0: balão nº 1; _BoyHere = 2                         (vai buscar o cortador)
    senão se S(600) == 0 ou _CurGameType == 3 ou _CurGameType == 0:  (S(600) sempre sorteado aqui)
        balão nº 2; _BoyHere = 5                                    (volta para casa)
    senão:
        se S(200) == 0: k = S(8); balão nº 7 + k
        se _RainOn ≠ 0: _BoyHere = 5
2 (indo ao cortador):  py < 12 → dir 2; py == 12: se px < 4 → PEGA O CORTADOR, senão dir 3; py > 12 → dir 0
    PEGA O CORTADOR: _BoyHere = 3; (_LastMowX, _LastMowY) = (3, 13); _BoyIsMowing = 1;
                     _GrassMap[0..2] = 0, [3..11] = 0xFFFF; _BoyDir = 2; (_BoyX, _BoyY) = (0x7D, 0xAA)
3 (cortando):
    se S(100) == 0: k = S(3); balão nº 3 + k
    se S(200) == 0: balão nº 6; _BoyHere = 4
    (_LastMowX, _LastMowY) = (_BoyPx, _BoyPy); _BoyIsMowing = 1
4 (guardando o cortador): py < 12 → dir 2; py == 12: se px < 4 → _BoyHere = 5, _BoyIsMowing = 0, senão dir 3;
                          py > 12 → dir 0. Depois (_LastMowX, _LastMowY) = (_BoyPx, _BoyPy)
5 (voltando para casa): py < 5 → dir 2; py > 5 → dir 0; py == 5: px > 3 → dir 3, senão _BoyHere = 1
(0 ou outro: nada)

(quadro e passo)
_BoyFrame = quadro_por_dir[_BoyDir] + quadro_por_passo[_YardCycle & 3]
se _BoyIsMowing == 0:
    se _BoyStandCnt == 0:
        se S32() == 0: _BoyStandCnt = 20
        ANDA
    senão: _BoyStandCnt −= 1; _BoyFrame = _BoyDir + 20                (parado: não anda)
senão: _BoyFrame += 100; ANDA
ANDA: _BoyX += passo_x[_BoyDir]; _BoyY += passo_y[_BoyDir]

a = _BoyY + _BoyX − 200; b = _BoyY − 38
_BoyPx = a / 28; _BoyPy = b / 10; limita _BoyPy a 0..15 e _BoyPx a 0..11

(bordas, só sem cortador)
se _BoyIsMowing == 0:
    se _BoyPy < 1: _BoyDir = 2   senão se _BoyPy > 14: _BoyDir = 0
    se _BoyPx ≥ 11: _BoyDir = 3
    senão se _BoyPy < 5: se _BoyPx ≤ 3: _BoyDir = 1
    senão se _BoyPx ≤ 2:
        _BoyDir = 0
        se _BoyPy == 5:
            se _BoyHere ≠ 3: _NodeNum = 0x15                          ← entra em casa (seção 4)
            senão: _BoyDir = 1

(pé e cortador)
_FootHere = 0; _FootTog = 1 − _FootTog (0 ↔ 1)
se (_BoyPx, _BoyPy) == _CurYardPnt:
    _FootHere = 1
    _FootX = ((a mod 28) × 4 + 6) & 0x7F;  _FootY = ((b mod 10 + 1) × 6) & 0x3F     (resto com o sinal de a, b)
    _MowX = _FootX + corte_dx[_BoyDir & 3]; _MowY = _FootY + corte_dy[_BoyDir & 3]
    se _BoyDir par: _FootX += (_FootTog ≠ 0 ? 6 : −6)  senão: _FootY += (_FootTog ≠ 0 ? 6 : −6)
    _FootFall(_FootX, _FootY)                                       (seção 3.1)
    se _BoyIsMowing ≠ 0 e _CurGameType ≠ 0: CORTADOR NO PATCH (corpo de _MowerFall, 3.2)
senão se _BoyIsMowing ≠ 0:                                          (cortando outro patch)
    para a colônia preta e depois a vermelha do patch (_BoyPx, _BoyPy):
        v = _YMapPop?[i]; se v ≠ 0: v −= v >> 2; se v == 0: _?ColoniesKilled += 1; grava v
```

Observações (fato visto no código):

- O cortador tira um quarto da população de cada colônia do patch em que o menino está, em cada tick em que ele está ali (andando ou não). Como `v − (v >> 2)` só dá 0 com v = 0, o contador `_BColoniesKilled/_RColoniesKilled` nunca sobe por aqui.
- Em Tutorial e Experimental o menino passeando volta para casa em todo tick (`_CurGameType` 0 e 3 forçam o ramo "volta"); ele só vai e vem perto da porta.
- `_BoyWait` só é iniciado (20) e nunca lido nas rotinas da simulação.

**O que o jogador vê com o menino cortando a grama (bancada, 2026-10-18, `cenarios_tick.py gravar tela_menino`):** nenhuma mensagem na faixa nem quadro. Na **janela de edição** nada do menino é desenhado (nem pé, nem cortador: o código de desenho da edição não lê `_FootHere`): só as formigas somem (3/4 por passada, sem cadáver) e as pisadas deixam cadáveres; na bancada as pretas da superfície caíram de 35 para 12 e as vermelhas de 28 para 6 em ~2 s. O som é o da pisada (0x23) a cada passo no patch atual. Na **janela do mapa** (superfície, sem trilhas) aparecem o pé (1200–1203) e o cortador (1210/1211) (`_DrawMapFoot`, telas/mapa.md). Na **Yard View**: o menino empurrando o cortador, o som 0x24 do cortador a cada redesenho e os balões nº 1 ("This yard needs mowing."), 3–5 e 6 ("I've had enough of this!"). O port faz o mesmo (o som 0x24 desde 2026-10-18).

### 3.1 Pisada: `_FootFall(x, y)` (`1030:3386`)

```
som 0x23
se _BoyDir par: x1 = x + 6, y1 = y + 16   senão: x1 = x + 16, y1 = y + 6
para cx = x .. x1 − 1, para cy = y .. y1 − 1:
    se 0 ≤ cx ≤ 127 e 0 ≤ cy < 128 e v = _LifeA[cx × 64 + cy] ≠ 0:   ← cy até 127: lê a coluna seguinte (defeito)
        se v é a amarela (0xFF ou 0xFE): _YellowDeath(5)
        senão: j = _FindInAList(cx, cy); se j ≥ 0: _DeadAntHere(cx, cy, T[j] & 0x80); T[j] = 0
se x ≤ _SuserX < x1 e y ≤ _SuserY < y1: _KillSpider(); se _MeMode == 1: _YellowDeath(5)
```

O teste da aranha usa `_SuserX/Y` (destino da aranha controlada pelo jogador), não a posição da aranha (fato visto no código). Com `cy ≥ 64` o índice cai na coluna `cx + 1`: uma formiga ali não é achada pelo `_FindInAList(cx, cy)` (nenhuma tem Y ≥ 64), mas a amarela é morta se estiver lá.

### 3.2 Cortador no patch atual (corpo de `_MowerFall`, `1030:34da`)

```
para j = _ListIndexA − 1 .. 0:   se T[j] ≠ 0 e S4() ≠ 0: _LifeA(X[j], Y[j]) = 0; T[j] = 0      (sem cadáver, sem contar)
se _SpidOn ≠ 0 e S4() ≠ 0: _KillSpider()
se _MeMode < 2 e _MePlane == 1 e S4() ≠ 0: _YellowDeath(6)
```

Cada passada mata 3/4 das formigas da superfície do patch inteiro, a cada 4 ticks (o menino no patch atual só anda nos ticks com `_YardCycle & 3 == 0`).

**Mortes da amarela pelo pé e pelo cortador, verificadas no original** (vetores de tick de 2026-10-18, `tools/cenarios_regras.py`, snapshot `rico`, 307/307 com S, R e vida): `amarela_morte_pisada/1712` (o pé cai em (68, 12), onde a amarela foi posta depois de um ensaio que mostrou o lugar do pé; `_YellowDeath(5)` e, no mesmo tick, `_KillSpider` porque `_SuserX/Y` estava no retângulo) e `amarela_morte_cortador/1712` (a passada mata a amarela com `S4()` ≠ 0, `_YellowDeath(6)`, e a aranha entra em agonia). Nos dois o gato foi tirado de jogo (`_CatDelay` enorme): o relógio do Mac lido depois de uma caixa modal não é gravado pela bancada, e o `_CatDelay` gravado com ele não seria reproduzível.

## 4. Menino dentro de casa: `_SimKidInside` (`1030:20b6`)

**Fato visto no código.** O menino dentro de casa segue um roteiro de nós. Nós 0–15, 20 e 21 são trechos de caminhada (tabelas de quadro, x e y por passo); nós 30–38 são paradas (quadro, x, y fixos, chance de sair e próximo nó). Tabelas em `data/quintal.json` (`casa`).

```
se _ForSaleState == 1: FIM                                          (casa vendida: não há menino)
se _NodeNum < 30:
    _NodeCnt += 1
    se _NodeCnt > duracao[_NodeNum]:
        _NodeCnt = 0
        k = S(3); _NodeNum = proximo[k][_NodeNum]
        balão: nó 0x21 → nº 20; nó 6 → nº 21; nó 7 → nº 22; nó 14 → nº 15 + S(3); nó 16 → nº 18 + S(2)
se 30 ≤ _NodeNum ≤ 38 (nó de parada):
    (quadro, x, y) = nos_fixos[_NodeNum]
    se S(chance[_NodeNum]) == 0: _NodeNum = proximo_fixo[_NodeNum]; _NodeCnt = 0
senão se _NodeNum em roteiros: (quadro, x, y) = roteiro[_NodeNum][_NodeCnt]
senão: (quadro, x, y) não definidos (variáveis locais sem valor; nós 16–19 e 22–29 não são alcançáveis pelas tabelas)
se _NodeNum == 0x26: _BoyDir = 1
_BoyHere = (_NodeNum == 0x26 ? 1 : 0)
se _NodeNum == 20 e _NodeCnt > 2: _BoyHere = 1
se _NodeNum == 21 e _NodeCnt < 3: _BoyHere = 1                     (perto da porta: visível lá fora)
(_BoyX, _BoyY, _BoyFrame) = (x, y, quadro)
(som 0x18 em quatro passos dos nós 20 e 21, só com a vista do quintal aberta; sem sorteio)
a = _BoyY + _BoyX − 200; b = _BoyY − 38; _BoyPx = a / 28; _BoyPy = b / 10      (sem limitar)
_FootHere = 0; _FootTog = 1 − _FootTog
se (_BoyPx, _BoyPy) ≠ _CurYardPnt: FIM
se o índice do patch atual é 0, 1, 16 ou 32: FIM                    (patches (0,0), (0,1), (1,0), (2,0))
_BoyDir = (0 ≤ _BoyFrame ≤ 11 ? dir_do_quadro[_BoyFrame] : 2)
_FootHere = 1; pé como em 3 (mesmas fórmulas; sem _MowX/Y); _FootFall(_FootX, _FootY); som 9
INSETICIDA: repetir (_RpopT + _BpopT) × 8 vezes:
    x = S128(); y = S64()
    se _LifeA(x, y) ≠ 0: _Tindex = _FindInAList(x, y); se _Tindex ≥ 0: _DeadAntHere(x, y, T & 0x80); T = 0; _LifeA(x, y) = 0
    se (_SpidX >> 4, _SpidY >> 4) == (x, y): _KillSpider()
se _CurGameType == 2 e _MePlane == 1 e S4() == 0: _YellowDeath(9)
```

O roteiro começa no nó 0 (`_InitSimYard` zera `_NodeNum`). O nó 20 sai sempre para 0x26 (38, "lá fora"); no mesmo tick o nó 38 sorteia `S(10)` e fica em 38; no tick seguinte, como `_NodeNum` ≥ 0x26, roda `_SimKidOutside` a partir de (0xB0, 0x68) com `_BoyHere = 1`. O menino volta para dentro quando, passeando, chega a `px ≤ 2, py = 5` (seção 3: `_NodeNum = 0x15`, o nó 21, que leva ao 14). Os nós 36 e 37 têm chance 0 (`S(0)` dividiria por zero), mas não são alcançáveis.

**Verificado** no `QUICK2.ANT`: `_NodeNum = 3`, `_NodeCnt = 10` e `(_BoyX, _BoyY) = (126, 92)` = `roteiro[3][10]`; `_BoyPx/_BoyPy = 0/5` pelas fórmulas.

## 5. Cachorro: `_SimDog` (`1030:2dbc`)

**Fato visto no código.** Direções 0–3 como o menino; 4 e 5 = sentado. Tabelas `cachorro`.

```
_DogTurnCnt −= 1
se _DogTurnCnt < 0:
    se _CatOn ≠ 0:   (_FollowCatDir) _DogPx < 5 → 1; _DogPx > 8 → 3; _DogPy > 0 → 0; senão _YardCycle & 3
    senão se _NodeNum < 0x26:  d = (S(3) + _DogDir − 1) & 3                 (menino em casa: passeia)
    senão (_FollowBoyDir):
        dy = _DogPy − _BoyPy; dx = _DogPx − _BoyPx
        se |dx| < 1 e |dy| < 1 e _BoyIsMowing ≠ 0 e _MapMode == 0 e _YardMode < 2 e relógio > _DogBarkDelay:
            som 0x15; _DogBarkDelay = relógio + 60 + S(30)                 (late para o cortador)
        se |dx| > 1 ou |dy| > 1: dy < 0 → 2; dy > 1 → 0; dx < 0 → 1; dx > 1 → 3
        senão d = _YardCycle & 3
    _DogDir = d; _DogTurnCnt = 4
se _DogDir == 1 e S(10) == 0: _DogDir = S2() + 4; _DogTurnCnt = 6          (senta por 7 ticks)
_DogFrame = _DogDir < 4 ? quadro_por_passo[_YardCycle & 3] + quadro_por_dir[_DogDir]
                        : quadro_por_dir[_DogDir] + (_YardCycle & 1)
_DogY += passo_y[_DogDir]; _DogPy = (_DogY − 38) / 10
_DogX += passo_x[_DogDir]; _DogPx = (_DogY + _DogX − 200) / 28
limita _DogPy a 0..15 e _DogPx a 0..11
_DogPy < 1 → _DogDir = 2; _DogPy > 14 → _DogDir = 0
_DogPx ≥ 11 → _DogDir = 3; senão _DogPx < 4 → _DogDir = 1
se 100 ≤ _DogFrame ≤ 103 e S(6) == 0:                                      (sentado: late às vezes)
    k = S4()
    se _MapMode == 0 e _YardMode < 2 e relógio > _DogBarkDelay:
        som (0x17, 0x15, 0x17, 0x16)[k]; _DogBarkDelay = relógio + 60 + S(30)
```

O cachorro não mata formigas. `_DogBarkDelay` está no relógio do Mac (1/60 s).

## 6. Gato: `_SimCat` (`1030:2b24`)

**Fato visto no código.** `_CatOn`: 0 ausente, 1 andando, 2 parado (se lambendo [hipótese]), 3 fugindo do cachorro.

```
se _CatOn == 0:
    se relógio ≤ _CatDelay: FIM
    _CatDelay = relógio + 200
    se S16() ≠ 0 e _GlobalKey ≠ 0x8D: FIM
    _CatOn = 1; (_CatX, _CatY) = (0xFC, 0x19); _CatDir = 2; _CatFrame = 0; _CatCycle = 0; FIM
_CatCycle = (_CatCycle + 1) & 0xFFF
estado 3: _CatFrame += 1; se _CatFrame ≥ 30: _CatFrame = 0; _CatOn = 0; _CatDelay = relógio + 600
estado 2: _CatFrame += 1; se _CatFrame > 12: _CatFrame = 11; _CatCycle += 1
          se _CatCycle ≥ 31: _CatCycle = 0; _CatOn = 1; _CatFrame = (_CatDir == 2 ? 0 : 3)
estado 1:
    (nx, ny) = (_CatX + 4 × Dx8[_CatDir], _CatY + 4 × Dy8[_CatDir])
    se S16() == 0 ou nx < 0xFC ou nx > 0x1EF:                    (vira)
        _CatDir == 2 → _CatDir = 6, _CatFrame = 4;  senão _CatDir = 2, _CatFrame = 1
    senão: (_CatX, _CatY) = (nx, ny); _CatFrame += 1; volta a 1 (dir 2) ou 4 (dir 6) depois de 2 ou 5
    se _MapMode == 0 e _YardMode < 2 e S64() == 0: som 0xD; FIM
    se S16() == 0 e _CatCycle > 100 e _DogDir == 2 e _BoyHere == 0: _CatOn = 2; _CatCycle = 0; _CatFrame = 10; FIM
    se _CatCycle > 50 e _GetDis(gato, cachorro) < 2401: _CatOn = 3; _CatFrame = 20; (som 0xE se a vista do quintal está aberta)
```

O gato anda só na horizontal (direções 2 e 6) e não mata formigas. `_GetDis` = dx² + dy² em 32 bits.

## 7. Pássaro: `_SimBird` (`1030:28aa`)

**Fato visto no código.** `_BirdOn`: 0 ausente, 1 voando até o enxame, 2 indo embora.

```
se _BirdOn == 0:
    se _gameCycles > _BirdDelay:
        _BirdDelay = _gameCycles + 20
        se _CurYardPnt.x > 4 e S2() == 0 e (_SwarmCntB > 0 ou _SwarmCntR > 0):
            _BirdGoalX = x × 28 − y × 10 + 178; _BirdGoalY = y × 10 + 46      ((x, y) = _CurYardPnt)
            _BirdOn = 1; _BirdX = 0; _BirdY = S64() + 4; _BirdFrame = 0
    FIM
_BirdX += (_BirdOn == 1 ? 16 : 8)
se _BirdOn == 1: d = _BirdGoalY − _BirdY; se d ≠ 0: _BirdY += sinal(d) × (|d| < 4 ? 1 : 4)
senão: _BirdY −= 8
_BirdFrame = (_BirdFrame + 1) & 1
se 0 ≤ _BirdX < 512 e 0 ≤ _BirdY < 256:
    se _BirdOn == 1 e _BirdX ≥ _BirdGoalX e |_BirdY − _BirdGoalY| ≤ 4:          (chegou: come o enxame)
        para v em _SwarmCntB, _QueenStorageB, _SwarmCntR, _QueenStorageR: se v > 0: v −= (v + 7) >> 3
        se _MapMode == 0 e _YardMode < 2 e S2() == 0: som 7
        _BirdOn = 2
senão: _BirdOn = 0; _BirdDelay = _gameCycles + 30
```

O `S2()` do aparecimento só é sorteado com `_CurYardPnt.x > 4`; com o patch inicial (11, 8) ele é sorteado em todo tick em que o atraso venceu.

## 8. `_InitSimYard` (`1030:1378`)

Chamado por `_RandYard` (novo jogo). **Fato visto no código.** `_BoyMsgCnt = 0`, `(_BoyX, _BoyY) = (0xB4, 0x49)`, `_BoyTurnCnt = 12`, `_BoyWait = 20`, `_BirdDelay = 0`, `_CatDelay = 0`, `(_DogX, _DogY) = (0xFA, 0x96)`, `_BoyDir = 2`, `_DogTurnCnt = 2`, `_BirdFrame = 1`, `_LastColonyPopB = _LastColonyPopR = 1`, `_BoyMsgOffset = −1`, `_YardCycle = −1`; zera `_BoyMessOn`, `_BoyPx/Py`, `_BoyFrame`, `_BoyHere`, `_BoyStandCnt`, `_NodeCnt`, `_NodeNum`, `_BirdOn`, `_CatCycle`, `_CatFrame`, `_CatOn`, `_DogFrame`, `_DogDir`, `_FootHere`, `_FootTog`, `_FootX/Y`, `_MowX/Y`, `_BoyIsMowing`, `_RainOn`, `_SwarmDelayB/R`. Não mexe em `_GrassMap` (só o menino ao pegar o cortador).

## 9. Chuva e água

### 9.1 `_DoWater` (`1020:07c6`), todo tick

Especificado em [ninho.md](ninho.md) §8.1–8.3 (conferido de novo no pseudocódigo para este documento). Resumo da ordem dos sorteios: com chuva no quintal, `S(50)` (som), `S(10)` (sobe a água), depois as 100 gotas (`S(14)` só para gota em 0x77 e `_PlaceDrop` com `R(128)`, `R(64)` para cada gota que não está numa poça em crescimento). Seco no quintal: `S(14)` para cada gota em poça (só no primeiro tick depois da chuva) e `S(10)` (só se `_WaterTable` < 64), seguido de `S(8)` para cada célula inundada (0x4E) que volta a túnel, coluna a coluna, preta antes da vermelha.

### 9.2 `_PlaceDrop(i)` (`1020:0acc`) e `_InitWater` (`1020:0b76`)

```
x = R(128); y = R(64); _RxTab[i] = x; _RyTab[i] = y
se _MapA(x, y) < 0x0E:
    _MapA(x, y) = 0x74
    célula c = (x >> 1, y >> 1) dos mapas de cheiro:
        _PherMapA = 0; _PherMapF = 0; _PherMapBN −= 20 (mínimo 0); _PherMapBT = 0; _PherMapRN −= 20 (mínimo 0); _PherMapRT = 0
```

`_InitWater` chama `_PlaceDrop(i)` para i = 0..99, na ordem.

**O que o jogador vê (bancada, 2026-10-18, `cenarios_tick.py gravar tela_chuva`):** a chuva **não tem mensagem própria** (nem na faixa, nem quadro): a faixa segue com os conselhos de sempre. Os sinais são: as poças azuis na superfície (janela de edição e mapa); a chuva desenhada na Yard View; o balão do menino nº 0 ("Looks like a bit of rain!", lista 4/1200) quando ele está à vista na Yard/House View; o efeito 0x29 (3,2 s) com chance 1/50 por tick; e, nos ninhos, a água subindo de baixo (ninho.md §8.4). O port faz o mesmo.

**Verificado no original (caso Q-01, `tests/referencias/casos/Q-01.json`, 2026-10-17):** a partir do `QUICK2.ANT` com `_RainOn = 1`, `_RainCnt = 600`, `_ClearRainFlag = 0`, o jogo parado no tick 140 tem `_RainCnt = 538` (exatamente 1 por tick), `_RainOn = 1`, `_ClearRainFlag = 1` e `_WaterTable = 61` (três linhas inundadas em 62 ticks). Estado final: `tests/referencias/saves/Q01_T140.ANT`.

## 10. Colônias do quintal: `_SimColonies` (`1030:35f8`)

**Fato visto no código; desmontagem conferida nos pontos marcados.** Só age quando `_YardCycle & 0x1F == 0` (uma vez a cada 32 ticks). Roda em todos os tipos de jogo; só a parte final (mensagens, casa, vitória e derrota) é do Jogo Completo.

```
_ColonyUpdateFlag = 1
ENXAME (corpo de _MaintainSwarm), para a cor preta e depois a vermelha:
    se _SwarmCnt > 0: _SwarmCnt −= (_SwarmCnt < 4 ? 1 : _SwarmCnt >> 2)
    _SwarmCnt = max(_SwarmCnt, _QueenStorage); _SwarmCnt = min(_SwarmCnt, 50)
_ColonyTotalBlack = (_BpopT > 0 ? 1 : 0); _ColonyTotalRed = (_RpopT > 0 ? 1 : 0); _HouseColonies = 0
_YMapPopB[atual] = min(_BpopT & 0x3FF, 250); _YMapPopR[atual] = min(_RpopT & 0x3FF, 250)
para px = 0..11, para py = 0..15, exceto o patch atual:          (o mapa é alterado no lugar, nessa ordem)
    b = _YMapPopB[px, py]; r = _YMapPopR[px, py]; se b == 0 e r == 0: próximo
    se _CurGameType == 2 e (px < 2 ou (px == 3 e py < 5)): _HouseColonies += 1
    p = PLACAR(px, py)                                             (_GetNearbyPatches embutido, 10.1)
    se b ≠ 0:
        _ColonyTotalBlack += 1
        n = (p == 0 ? b + 1 : b + p)
        n < 1 → _BColoniesKilled += 1; _YMapPopB = 0
        n < 250 → _YMapPopB = n
        senão → _YMapPopB = 250; se S(10) == 0: REPRODUZ preta a partir de (px, py)   (10.2)
    se r ≠ 0:
        _ColonyTotalRed += 1
        n = (p == 0 ? r + 1 : r − p)
        n < 1 → _RColoniesKilled += 1; _YMapPopR = 0
                se px == 11: _YMapPopR[2 + S(6), 0] = 20; e py passa a 8 (o laço continua em py = 9)   ← desmontagem
        n < 250 → _YMapPopR = n
        senão → _YMapPopR = 250; se S(10) == 0: REPRODUZ vermelha
REVOADA GUARDADA (preta só com _LayDownQueenMode == 1; vermelha sempre):
    _SwarmDelay += 1
    se S8() + 10 < _SwarmDelay:
        _SwarmDelay = 0
        enquanto _QueenStorage > 0: REPRODUZ a partir do patch atual; _QueenStorage −= 1
SÓ NO JOGO COMPLETO (_CurGameType == 2):
    se _ColonyTotalRed == 0 e _LastColonyPopR ≠ 0: música 0x4E22; mensagem 10052; se _ForSaleState == 0: mensagem 10055
    _LastColonyPopB = _ColonyTotalBlack; _LastColonyPopR = _ColonyTotalRed
    se _ForSaleState == 0 e _HouseColonies > 24:
        _ForSaleState = 1; _NodeNum = 0; _BoyHere = 0; _BoyFrame = −1; música 0x4E23; mensagem 10054
        se _ColonyTotalRed ≠ 0: mensagem 10053
    se _ForSaleState ≠ 0 e _ColonyTotalRed == 0:
        música 0x4E25; mensagem 10057; _IsGameOver = 1; _BlackWon = 1; (_DrawSimPayoff); FIM
    se _ColonyTotalBlack < 2 e _CastePopB[5] == 0 e _CastePopB[0] == 0 e _CastePopB[3] == 0 e _CastePopB[4] == 0:
        _IsGameOver = 1; _BlackWon = 0; mensagem 10056
```

Pontos conferidos na desmontagem (`1030:37c9`–`3816`, `392d`, `39fa`):

- O placar lê os deslocamentos dos vizinhos **sem sinal** (`mov al, [bx+0x25dc]; sub ah, ah` e `and si, 0xff`). A tabela tem dx = (0, 1, 0, 0xFF, 0, 0) e dy = (0xFF, 0, 1, 0, 0, 0): os vizinhos (x, y − 1) e (x − 1, y) viram (x, y + 255) e (x + 255, y), fora do quintal, e não contam; os dois últimos pares (0, 0) contam **o próprio patch duas vezes**.
- Com a colônia vermelha extinta em `px == 11`, o código grava `di = 8` (`di` é o `py` do laço) antes de sortear `S(6)`. O laço segue em `py = 9`: com `py_antigo < 8`, os patches (11, py_antigo + 1 .. 8) são pulados nessa passada; com `py_antigo > 9`, os patches (11, 9 .. py_antigo) são processados de novo (o próprio (11, py_antigo) já está vazio de vermelhas). Fato visto na desmontagem; leitura [hipótese]: defeito, a intenção seria testar o patch (11, 8), o ninho vermelho inicial.
- A revoada preta guardada depende de `_LayDownQueenMode` (`8:836e`; o seletor vem de `DGROUP:0xc56e`, que a relocação NE liga ao segmento 8). No `QUICK2.ANT` vale 1.
- `_BColoniesStarted` (`9:80d4`) e `_RColoniesStarted` (`9:9c80`) sobem quando REPRODUZ cai num patch vazio; `_BColoniesKilled` (`9:a0dc`) e `_RColoniesKilled` (`9:7896`) quando uma colônia some.
- A área de `_HouseColonies` (px < 2, ou px = 3 com py < 5) difere da área "casa" da pontuação ([simulacao-visao-geral.md](simulacao-visao-geral.md) §8, [em aberto] lá).

### 10.1 PLACAR (`_GetNearbyPatches`, `1030:3ce4`)

```
p = 0
para k = 0..5: (vx, vy) = (px + vizinho_dx[k], py + vizinho_dy[k])   (deslocamentos sem sinal)
    se 0 ≤ vx < 12 e 0 ≤ vy < 16: se _YMapPopB[vx, vy] ≠ 0: p += 3; se _YMapPopR[vx, vy] ≠ 0: p −= 3
```

Com a tabela do executável: p = 6 × (preta aqui) − 6 × (vermelha aqui) + 3 × (preta em (px + 1, py)) − 3 × (vermelha ali) + 3 × (preta em (px, py + 1)) − 3 × (vermelha ali). As pretas crescem com p (`b + p`) e as vermelhas com −p (`r − p`); com p = 0 as duas crescem 1.

### 10.2 REPRODUZ (`_Reproduce`, `1030:3d4c`)

```
x = SG(4) + px; y = SG(4) + py; limita x a 0..11 e y a 0..15          (x sorteado antes de y)
se (x, y) ≠ (px, py):
    se _YMapPop?[x, y] == 0: _?ColoniesStarted += 1
    _YMapPop?[x, y] += 1                                              (byte: 255 + 1 = 0)
```

### 10.3 Revoada

**O que o jogador vê (bancada, 2026-10-18, `tela_revoada` e `tela_mate`):** nenhuma mensagem quando a revoada acontece. As formigas aladas em pré-voo (M = 14/15) ficam perto do buraco e, ao decolar (M = 16), somem da superfície. Na **Yard View** o enxame aparece como pontinhos pretos e vermelhos em volta do patch atual (`_DrawSwarm`, telas/quintal.md §2.3), e o pássaro vem comê-lo (som 7). Só o botão **MATE** (Jogo Completo) escreve na faixa e no rodapé do quintal: "You have started the mating cycle", ou as recusas "Produce males and new queens first" / "Fertilize queens in Full Game only" (telas/quintal.md §3). No port o enxame e o botão MATE existem desde 2026-10-18.

`_SwarmCnt*` sobe quando formigas M = 16 decolam ([superficie.md](superficie.md)) e `_QueenStorage*` com chance 1/16 em cada decolagem no Jogo Completo. O pássaro come 1/8 (arredondado para cima) do enxame e das rainhas guardadas (seção 7).

### 10.4 Migração: `_StartMigrate(X, Y)` / `_EndMigrate(X, Y)` (`1030:3df2`, `1030:3e6c`)

Sem chamador no pseudocódigo [em aberto: resto de uma ferramenta de arrastar colônias]. `(X, Y)` em pixels da vista do quintal, com outra origem: `mx = (X + Y − 238) / 28`, `my = (Y − 66) / 10`.

- `_StartMigrate`: guarda `(_MigrateX, _MigrateY) = (mx, my)`; se fora do quintal **ou** sem colônia preta ali, `_MigrateX = −1`.
- `_EndMigrate`: se `_MigrateX ≥ 0` e o destino está no quintal: `h = _YMapPopB[origem] >> 1`; a origem perde h e o destino ganha h, limitado a 250.

**Verificado no original (caso Q-02, `tests/referencias/casos/Q-02.json`, 2026-10-17):** colônias postas à mão no `QUICK2.ANT` e uma passada no tick 97. As 16 conferências bateram: (2,2) preta 10 → 16; (5,5) preta 10 com (6,5) preta → 19 e (6,5) 20 → 26 (o vizinho (5,5) não conta: com deslocamentos com sinal daria 29); (8,10) vermelha 10 → 16; (9,12) preta 5 e vermelha 5 → 6 e 6; (1,13) preta 3 com vermelha em (1,14) → 6, e a vermelha 10 → 16; `_ColonyTotalBlack` = 6, `_ColonyTotalRed` = 4; `_SwarmCntB` 40 → 30, `_SwarmCntR` 3 → 2, `_SwarmDelayB/R` 3 → 4. Estado: `tests/referencias/saves/Q02_T100.ANT`.

## 11. Rotinas auxiliares usadas aqui

- `_DeadAntHere(x, y, cor)` ([superficie.md](superficie.md) 6.4), `_FindInAList` ([formigas.md](formigas.md)), `_KillSpider` ([inimigos.md](inimigos.md) 2.8), `_YellowDeath(5, 6, 9)` ([formiga-amarela.md](formiga-amarela.md) §9), `_DrownBList/_DrownRList` ([ninho.md](ninho.md) 8.2).

## 12. Tabelas do executável

Extraídas por `tools/extrair_quintal.py` para `data/quintal.json` (endereços no topo do script). Resumo:

| Tabela | Endereço | Valores |
|---|---|---|
| `_BxTab`/`_ByTab` (passo do menino) | `1038:8a76`/`8a7a` | x (4, 6, −4, −6), y (−4, 0, 4, 0) |
| quadro do menino por direção / por fase | `1048:25c0`/`25bc` | (0, 3, 6, 9) / (0, 1, 2, 1) |
| cortador relativo ao pé | `1048:25c4`/`25cc` | dx (−10, 25, −10, −33), dy (−33, −10, 25, −10) |
| vizinho por direção (cortar) | `1048:25d4`/`25d8` | dx (0, 1, 0, −1), dy (−1, 0, 1, 0) |
| direção do pé pelo quadro | `1048:25b0` | 0,0,0,1,1,1,2,2,2,3,3,3 |
| cachorro: passo, quadros | `1048:236a`/`2370`/`2376`/`237a` | dx (3, 4, −3, −4, 0, 0), dy (−3, 0, 3, 0, 0, 0), fase (0, 1, 2, 1), base (0, 3, 6, 9, 100, 102) |
| roteiro da casa | `1048:2380`–`25ae` | duração de 22 nós, 3 tabelas de próximo nó, passos dos nós 0–15, 20, 21, paradas 30–38 |
| vizinhos das colônias | `1048:25dc`/`25e2` | dx (0, 1, 0, 255, 0, 0), dy (255, 0, 1, 0, 0, 0), lidos sem sinal |

## 13. Casos de teste

Unitários do port (`engine/tests/regras_quintal.rs` e testes de `rules/quintal.rs`), a partir desta especificação:

| Caso | Entrada | Esperado |
|---|---|---|
| Q-U1 placar | preta sozinha; preta com vizinha à direita; vizinha à esquerda | +6; +9; +6 (esquerda não conta) |
| Q-U2 passada de colônias | o mesmo patch do caso Q-02 sobre o `QUICK2.ANT` | igual ao estado do original `Q02_T100.ANT` nos dois mapas |
| Q-U3 reprodução no teto | patch com 250 e S(10) = 0 | um vizinho a ±3 ganha 1 e conta como fundado se estava vazio |
| Q-U4 extinção vermelha em px = 11 | vermelha 1 cercada de pretas em (11, 3) | (11, 3) = 0, `_YMapPopR[2 + S(6), 0]` = 20 e o laço volta a py = 9 |
| Q-U5 cortador fora | menino cortando no patch (5, 5) com preta 100 | 75 |
| Q-U6 roteiro da casa | `_NodeNum = 3`, `_NodeCnt = 9` | (126, 92) no passo 10, como no `QUICK2.ANT` |
| Q-U7 chuva | `_GlobalKey = 0xA8` | `_RainOn = 1`, `_RainCnt` em 150..299, 100 gotas sorteadas por R |
| Q-U8 água | `_RainOn = 1` por 62 ticks sobre o `QUICK2.ANT` | linhas inundadas como no `Q01_T140.ANT` (0x4E ou +0x2F) |
| Q-U9 vitória e derrota do Jogo Completo | casa à venda sem vermelhas; uma colônia preta sem rainha nem crias | 10057 e fim com `_BlackWon = 1`; 10056 com `_BlackWon = 0` |
| Q-U10 vetores de tick do original (`tests/referencias/ticks/`, ticks 79–83) | `estado_antes` com S, R e `_gameCycles` do `extras.json` | campos do menino, cachorro, gato, pássaro e quintal iguais ao `estado_depois` |

Casos no original (bancada QEMU): Q-01 (chuva e água) e Q-02 (colônias), ambos confirmados (ver acima e `tests/referencias/README.md`).

## Implementação no port

- `engine/src/rules/quintal.rs`: `TabelasQuintal` (`data/quintal.json`), `simular_quintal_com(e, tabelas, vitimas, relogios)` (`_DoSimYard`; `Vitimas` = tabelas da amarela e agonia da aranha, para `_YellowDeath` e `_KillSpider`), `simular_agua` (`_DoWater`), `iniciar_quintal` (`_InitSimYard`), `simular_colonias`, `placar_vizinhos`, `reproduzir`, `manter_enxame`, `iniciar_migracao`/`terminar_migracao`, `por_gota`/`iniciar_agua`. `impl RegrasQuintal for RegrasOriginais` usa `RegrasOriginais::quintal` (carregado de `data/quintal.json` com mods).
- **Relógios do port** (decisão registrada em [../../divergencias.md](../../divergencias.md)): `Estado::relogio_mac` (`Option<u32>`), quando presente, é o relógio do Mac — os vetores de tick o trazem do original (`extras.json`, `MacTickCount`). Sem ele, quando `Estado::ciclos_jogo` ≠ 0 (o laço de jogo conta as voltas do laço de `MYTIMERFUNC`, como o original; os vetores de tick trazem o `_gameCycles` do original), ele é usado como `_gameCycles` e como relógio do Mac (regra do port, **refutada** como modelo do original: nos vetores `fases/159` e `colonia_5min/1887` antigos o original tinha relógio ~3333 e ~18903 com `_gameCycles` 29579 e 269985). No jogo ao vivo (`core::partida`) o relógio é o tempo real, `⌊ms desde o início / 55⌋ × 3`, somado a um início igual ao maior prazo do save nesse relógio (`_CatDelay`, `_DogBarkDelay`, `_BoyMsgCnt`; 0 num jogo novo): no original o `GetTickCount` é o tempo desde que o Windows ligou, em geral maior que o da sessão que salvou, e os prazos vindos do save já venceram ao carregar [decisão do port, 2026-10-18; antes o relógio começava em 0 e o gato esperava o `_CatDelay` do save]. Senão: `_gameCycles` = `_GameTime` (1 por tick, como no original em "fast"/"ultra" [hipótese]); relógio do Mac = `_GameTime × mac_por_tick` (padrão 6, [hipótese] ≈ velocidade normal). Só com o relógio sintético (sem `relogio_mac` e sem `_gameCycles`), um atraso vindo de um save do original maior que "agora + o maior atraso que a regra grava" é considerado vencido; com relógio real o atraso fica como está, como no original (vetores `tutorial`, 2026-10-17: `_CatDelay` 4712 vindo do arquivo do tutorial com o relógio em ~3300).
- **Verificado** (`engine/tests/regras_quintal.rs`): Q-U2 dá os dois mapas `_YMapPop*` do `Q02_T100.ANT` byte a byte; Q-U8 dá as linhas 61–63 de `_MapB/_MapR` do `Q01_T140.ANT`; Q-U10 reproduz, nos ticks 79–83 dos vetores do original, todos os campos do menino, do gato, do cachorro e do pássaro (inclusive `_BirdDelay`, com o `_gameCycles` do vetor), exceto `_BoyMsgCnt`/`_CatDelay`/`_DogBarkDelay` (relógio do Mac, não capturado). Nesses ticks o menino está lá fora parado, o cachorro anda e senta e o gato anda: o menino cortando, o inseticida e as colônias com sorteio não foram exercitados contra o original.
- O teste `tick_vazio_sobre_o_quick2` (`engine/tests/estado_simulacao.rs`) supõe sub-rotinas vazias e passa a falhar com o quintal ligado (mudam os campos do quintal); precisa ser ajustado na integração.
- Mensagens e músicas saem como `Evento::Mensagem(id)` e `Evento::Musica(id)`. Sons (`_myBeginSound`) e o balão do menino não geram evento (o balão fica em `_BoyMessOn`/`_BoyMsgOffset`).

## Lacunas

- `_YellowDeath` (causas 5 pisada, 6 cortador, 9 inseticida) chama `matar_amarela` de `rules/amarela.rs` (teste `pisada_mata_a_amarela`); não foi conferido contra o original.
- `_DeadAntHere`, `_FindInAList` e `_KillSpider` são os de `engine/src/rules/comum.rs` (ver [../../decisoes.md](../../decisoes.md), "Módulo comum das regras"); a agonia da aranha vem de `inimigos.json` (`regras.aranha_agonia`), não mais de uma constante do quintal.
- Textos do balão do menino (lista 4/1200) e dos sons: não conferidos.
- `_LastColonyPop*` não está no save nem no `Estado`: o port usa o `_ColonyTotal*` da passada anterior, que é o mesmo valor exceto logo depois de `_InitSimYard` (1 no original).
- Taxa do relógio do Mac por tick (`mac_por_tick`): na bancada QEMU (TCG lento) são 9 por tick; em máquina real depende de quantos ms cada tick leva (o relógio é tempo real, ver "Relógios") — não medida fora da bancada.
