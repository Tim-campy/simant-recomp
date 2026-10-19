# Inimigos e bichos do quintal: formiga-leão, aranha, lagarta e bicho-de-conta

Este documento especifica, para reimplementação 1:1, os bichos que a simulação move a cada tick fora das listas de formigas: a **formiga-leão** (`_DoAntLions`), a **aranha** (`_MoveSpider`), a **lagarta** (`_DoPillar`) e o **bicho-de-conta** (`_DoSow`). A posição de cada um no tick está em [simulacao-visao-geral.md](simulacao-visao-geral.md) (passos 8, 9 e 10); a criação no começo do jogo em [geracao-mundo.md](geracao-mundo.md) (seções 3.2, 4.2 e 4.3); as formigas em [superficie.md](superficie.md); a amarela (que pode morrer para eles ou até virar a aranha) em [formiga-amarela.md](formiga-amarela.md).

Convenção de evidência e notação: iguais às de [superficie.md](superficie.md#convenção-de-evidência). `S1(n)`, `S2()`, `S4()`, `S8()`, `S16()`… são o gerador S; `Mapa`/`Vida` são `_MapA`/`_LifeA` (superfície, índice `x·64 + y`). Nenhum código decompilado é reproduzido.

O menino, o cachorro, o gato, o pássaro, o cortador de grama e o inseticida também matam formigas e a amarela (`_SimKidOutside`, `_SimKidInside`, `_FootFall`, `_MowerFall`), mas fazem parte do quintal (`_DoSimYard`) e ficam para um documento próprio [em aberto]. Aqui só se registra o efeito deles sobre a aranha (seção 2.8) e as mortes da amarela que provocam (tabela em [formiga-amarela.md](formiga-amarela.md#9-morte-_yellowdeath-10007746)).

## 1. Formiga-leão

### 1.1 Estado

**Fato visto no código.** Até 10 formigas-leão em listas paralelas de bytes (todas no save, [save-ant.md](../formatos/save-ant.md), blocos 67–71):

| Campo | Endereço | Significado |
|---|---|---|
| `_LionListX/Y` | `9:809c` / `9:80bc` | centro da armadilha |
| `_LionListM` | `9:7a68` | estado: 0 esperando, 1 comendo, 2 voltando a esperar |
| `_LionListS` | `9:7d34` | contador de tempo do estado 1 |
| `_LionListT` | `9:7d4e` | quadro da animação (0–6: 0–4 ao entrar e sair do estado 1, 3–6 sorteado durante a refeição); o tile do centro é `0x38 + T` (0x38–0x3E) |
| `_LionIndex` | `8:8a88` | quantas existem (0–9; o incremento para em 9) |
| `_InitialLions` | `9:9e8c` | quantas o patch tinha ao ser criado (0 na casa) |

Tiles (fato visto no código): centro 0x38 (quadro 0) a 0x3C; borda 0x30–0x37 (ver [tiles-valores.md](tiles-valores.md)); 0x3F = formiga-leão morta. O "salto" usado ao comer é 0x39 (quadro 1).

### 1.2 Criação: `_InitAntLions`, `_AddRandAntLion`, `_AddAntLion`

`_InitAntLions(n)` está em [geracao-mundo.md](geracao-mundo.md) (4.2). `_AddRandAntLion` (`1030:4222`) repete o sorteio de posição de uma única formiga-leão; o mesmo bloco está embutido duas vezes em `_DoAntLions` (1.3). `_AddAntLion(x, y)` (`1030:4340`) cria uma numa posição dada, sem sorteio [sem chamador encontrado].

Sorteio de posição (fato visto no código), até 200 tentativas, nesta ordem por tentativa: `x = S1(64) + S1(65)`, `y = S1(32) + S1(33)`; aceita se `_IsClear3x3(1, x, y)`, ou se a célula central está livre (`_IsClearTile`) e já houve mais de 100 tentativas. Grava 0x38 no centro e a borda (0x30 + `DGROUP:25e8[k]`) nas vizinhas livres; zera M, S e T; `_LionIndex += 1` se < 9 (com 9 formigas-leão, a décima sobrescreve a última vaga).

### 1.3 `_DoAntLions` (`1030:43fa`), todo tick

**Fato visto no código.** Pseudocódigo próprio:

```
se _LionIndex == 0 e _InitialLions > 0 e S1(1024) == 0:     ← curto-circuito: S1 só se as duas primeiras valem
    cria uma formiga-leão nova (sorteio 1.2) ; fim do tick para as formigas-leão
senão, para k = 0 .. _LionIndex − 1:
    (x, y) = centro da k
    caso M[k]:
    0 (esperando):
        v = Mapa(x, y)
        se v é pedrinha (0x51–0x53) ou comida:            ← algo caiu na armadilha
            se S1(512) == 0:
                d = S8()
                para j = 0..7: alvo = (x, y) + 2·D8[d]
                    se _IsClearTile(1, alvo): Mapa(alvo) = v ; som ; sai do laço
                    d = (d + 1) & 7
                som ; S[k] = 25 ; ENTRA_EM_1
            (sem teste de formiga neste tick)
        senão:
            c = (0, 0)                                     ← primeira célula testada: o próprio centro
            repetir 2 vezes:
                se Vida(x + c) ≠ 0: PEGA(x + c) ; sai
                c = D9[S1(8) + 1]                         ← sorteado também depois da 2ª célula vazia
    1 (comendo):
        n = número de vizinhas (8 direções) com Vida ≠ 0
        se n ≥ 7:                                         ← as formigas cercaram e mataram a formiga-leão
            mensagem 10050 ; Mapa(x, y) = 0x3F ; remove a k da lista (desloca as seguintes) ; _LionIndex −= 1
            (o laço segue para k + 1: a formiga-leão deslocada para a vaga k NÃO roda neste tick;
             fato visto na desmontagem, 1030:48ce → 4ac8 `inc di`)
        senão:
            se T[k] < 4: T[k] += 1
            senão se S[k] == 0: M[k] = 2 ; T[k] = 4
            senão: S[k] −= 1 ; se S[k] for ímpar: T[k] = S1(4) + 3
            Mapa(x, y) = 0x38 + T[k]
    2 (voltando):
        se T[k] == 0:
            M[k] = 0 ; _AntsEatenByLions += 1
            se _LionIndex < 9 e (_AntsEatenByLions & 15) == 15: cria uma formiga-leão nova (sorteio 1.2)
        senão: T[k] −= 1
        Mapa(x, y) = 0x38 + T[k]

PEGA(célula):
    se é a amarela (0xFF/0xFE): _MoveMyLife(1, centro, _MeType, _MeDir) ; _YellowDeath(2)
    senão:
        se S8() == 0: som
        j = _FindInAList(célula) ; se j ≥ 0: _RemoveFromAList(j)    ← remoção imediata, com deslocamento
        preta: _BAntsEaten (9:76a0, 32 bits) += 1 ; vermelha: _RAntsEaten (9:7d5c) += 1
    S[k] = 50
ENTRA_EM_1:
    M[k] = 1 ; T[k] = 1 ; Mapa(x, y) = 0x39
```

Observações (fato visto no código):

- `D9` = `_Dx9/_Dy9` (`8:0010`/`8:001a`): índice 0 é "parado", 1–8 são as direções 0–7. A formiga-leão olha o próprio centro e **uma** vizinha sorteada por tick.
- `_RemoveFromAList` (`1020:2b42`) zera `Vida` da formiga e desloca as vagas seguintes da lista A uma posição para trás. Como `_DoAntLions` roda antes de `_DoAntSimA`, isso só muda os índices antes do laço das formigas.
- Na morte da formiga-leão, o centro vira 0x3F, mas a borda 0x30–0x37 continua no mapa.
- Quando a armadilha "cospe" uma pedrinha ou comida e não acha lugar livre, o objeto se perde (o centro é sobrescrito por 0x39).
- A formiga-leão nunca se move.
- Mensagem 10050 (SHARED 4/10050): "The ants have surrounded and killed an ant lion! They tear its pulpy body apart and eat it.", mostrada por `_PictStrnDialog`.

### 1.4 `_KillAntLion(k)` (`1030:4b58`)

Mesma remoção do estado 1 (centro = 0x3F, deslocamento da lista), sem mensagem. Chamada pela ferramenta Inseticida do modo Experimental (`_ExpKillAnts` `1010:7d5c`) quando o tile atingido está em 0x38–0x3E; a formiga-leão é achada por `_FindInLionList` (`1030:4b12`).

**`k` = −1** (tile de formiga-leão sem entrada na lista: `_FindInLionList` devolve −1 e `_ExpKillAnts` chama `_KillAntLion(-1)` sem testar) [visto no código, `1030:4b58`; não testado no original]. O original lê e grava a "vaga −1" de cada vetor da lista, que é o último byte da variável anterior no DGROUP (nomes do `.SYM`):

| Vetor | Vaga −1 | O que é |
|---|---|---|
| `_LionListX` (`9:809c`) | `9:809b` | byte alto do seletor de `_EggStr` (ponteiro far, `9:8098`) |
| `_LionListY` (`9:80bc`) | `9:80bb` | byte alto de `_LastThemeSong` (`9:80ba`; contador 0–2 do tema, em geral 0) |
| `_LionListT` (`9:7d4e`) | `9:7d4d` | byte alto de `_BoyMsgCnt` (`9:7d4a`, 32 bits) |
| `_LionListM` (`9:7a68`) | `9:7a67` | byte alto do y de `_CenterPnt` (`9:7a64`) |
| `_LionListS` (`9:7d34`) | `9:7d33` | byte alto de `_mapOutBuf` (`9:7d32`, alça de memória) |

Efeito: (1) `_SetMap(1, X[−1], Y[−1], 0x3F)`: um 0x3F no `_MapA` em (byte alto do seletor de `_EggStr`, byte alto de `_LastThemeSong`), se couber no mapa — com `_LastThemeSong` < 256, na linha 0; (2) com `_LionIndex` > 0, o contador cai 1 e o deslocamento começa na vaga −1: as vagas −1 … n−1 recebem as 0 … n, ou seja, **a formiga-leão 0 sai da lista** (o tile dela fica no mapa, parado) e os cinco bytes da tabela acima recebem os campos dela (X → seletor de `_EggStr`, Y → `_LastThemeSong`, T → `_BoyMsgCnt`, M → `_CenterPnt`, S → `_mapOutBuf`). Com `_LionIndex` = 0 só o `_SetMap`. O tile atingido não muda. As gravações no seletor de `_EggStr` e na alça `_mapOutBuf` podem corromper a interface do original depois [hipótese: não observado].

**Port** (2026-10-19, `inimigos::matar_formiga_leao_sem_entrada`): reproduz o que fica no estado da partida (e no save): a saída da entrada 0 e o contador, o byte alto de `_BoyMsgCnt` (o balão do menino passa a esperar 2²⁴ × T do relógio do Mac, na prática nunca mais aparece se T > 0) e o de `_LastThemeSong` (o próximo tema volta ao primeiro). Não reproduz o `_SetMap` (o x é um seletor de memória do Windows, sem equivalente) nem as gravações em `_EggStr`, `_CenterPnt` e `_mapOutBuf` (interface do Windows; reproduzi-las não é seguro). Teste `inseticida_sem_entrada_na_lista_tira_a_formiga_leao_0`. Antes o port não fazia nada.

## 2. Aranha

### 2.1 Estado e coordenadas

**Fato visto no código.** A aranha não está em lista nenhuma e não aparece no `_LifeA`. Coordenadas em dezesseis avos de célula: célula = `_SpidX >> 4`, `_SpidY >> 4`.

| Variável | Endereço | Significado |
|---|---|---|
| `_SpidX`, `_SpidY` | `10:ac7c` / `10:ac7e` | posição (×16) |
| `_SpidDir` | `10:ac80` | direção 0–7 |
| `_SpidOn` | `9:9bd2` | 1 = aranha no mapa |
| `_SMode` | `9:729e` | estado: 0 parada, 1 andando, 2 perseguindo, 3 comendo, 4 fugindo, 5 morrendo |
| `_Starg` | `9:7a5e` | alvo: índice na lista A, −1 = a amarela, −2 = nenhum |
| `_StargLife` | `9:80a8` | T do alvo no momento em que foi escolhido (0xFF para a amarela) |
| `_Scycle` | `9:7290` | quadro de animação das pernas (0–1023) |
| `_Scycle2` | `9:77ac` | contador de ticks (0–1023) |
| `_EatCnt` | `9:77ae` | ticks de mastigação restantes |
| `_SCorpseBase` | `9:78da` | 0 se comeu preta (ou a amarela), 4 se comeu vermelha |
| `_SpidBurpCnt` | `9:8072` | contador para o arroto (começa em 10) |
| `_DeathCnt` | `9:72e0` | ticks de agonia restantes (estado 5) |
| `_SpidRevenge` | `9:72c2` | quantas aranhas já morreram (máximo 5) |
| `_MeSMode` | `9:7d60` | "modo de caça": 0 normal, 6 alvo escolhido pelo jogador, 7 faro, 8 laser |
| `_RevSpider` | `8:8a5a` | 1 = anda de ré (os passos são invertidos) [em aberto: nada liga a flag] |
| `_SuserX/Y` | `9:78b8` / `9:78ea` | destino clicado quando o jogador é a aranha |
| `_MeMode` | `9:9fe8` | 1 = o jogador controla a aranha (seção 2.7) |

Tabelas (fato verificado no `SIMANTW.EXE`, segmento 8): passo `_SpX` (`8:87dc`) = `0 3 4 3 0 −3 −4 −3`, `_SpY` (`8:87e4`) = `−4 −3 0 3 4 3 0 −3` (em dezesseis avos: andando, cerca de 1/4 de célula por tick; correndo, ×5); célula da boca `_SeatX/_SeatY` (`8:87ec`/`8:87f4`) = `_Dx8/_Dy8`.

Começo do jogo (`_InitSpider` `1020:4668`, [geracao-mundo.md](geracao-mundo.md)): (0x400, 0x200) = célula (64, 32), `_SMode = 0`, `_SpidOn = 1` exceto no Tutorial.

### 2.2 `_MoveSpider` (`1020:4704`), todo tick — visão geral

```
_Scycle2 = (_Scycle2 + 1) & 1023
(cx, cy) = (_SpidX >> 4, _SpidY >> 4)
se _MeMode == 1 e _SMode ∉ {2, 3}: CONTROLE DO JOGADOR (2.7) ; fim
se _SpidOn == 0: TALVEZ APAREÇA (2.3) ; fim
se (_Scycle2 & 3) == 0 e _SMode < 5: CERCO (2.4)            ← pode encerrar o tick
se _SMode < 4 e _MeSMode == 8: _SpiderScan()                 (2.6)
executa o estado _SMode (2.5)
se a célula (_SpidX >> 4, _SpidY >> 4) saiu do mapa:
    _SpidOn = 0 ; se _MeMode == 1: _MeMode = 0 ; _YellowDeath(4)
se casa e Mapa(célula) > 0x90: _SpidOn = 0
```

### 2.3 Aparecer

Se `_CurGameType` = 0 (Tutorial): nunca. Senão, com `S1(300) == 0`: `_SpidX = S1(1024) + 512` (célula 32–95), `_SpidOn = 1`, `_SMode = 1`, `_RevSpider = 0`; `S1(2) ≠ 0` → entra por cima (`_SpidY = 1`, direção 4); senão por baixo (`_SpidY = 0x3FF`, direção 0). Fato visto no código.

### 2.4 Cerco: as formigas matam a aranha

A cada 4 ticks (`_Scycle2 & 3 == 0`), fora da agonia: conta as células ocupadas em `_LifeA` no quadrado 3 × 3 em volta da célula da aranha, incluindo a própria (a aranha não ocupa `_LifeA`).

- 9 ocupadas (todas): a aranha morre. `_SMode = 5`, `_DeathCnt = 500`, `_Scycle = 0`. Se o jogador era a aranha: `_MeMode = 0` e `_YellowDeath(3)`; fim. Senão: mensagem 10046 ("Congratulations! The spider has been killed by the ants."), `_SpidRevenge += 1` (até 5); com `_SpidRevenge` 3 ou 4, a **próxima** aranha caça por faro (`_MeSMode = 7`); com 5, por laser (`_MeSMode = 8`). (Um sorteio `S2()` que zeraria a vingança está em código inalcançável, porque `_SpidRevenge` nunca passa de 5.)
- 5 a 8 ocupadas: `_SMode = 4` (foge).

### 2.5 Estados

**Fato visto no código.** "Andar 1" = soma `(_SpX[dir], _SpY[dir])` (ou subtrai, com `_RevSpider`) e `_Scycle ± 1`; "correr" = o mesmo ×5 e `_Scycle ± 2`; `_Scycle` sempre `& 1023`.

| `_SMode` | Ordem exata das ações |
|---|---|
| 0 parada | `_Scycle = 2`; `S1(150) == 0` → `_SMode = 1`; `_Starg = _SFoundAnt()`; se ≠ −2 → ACHOU (abaixo) e fim; senão `S1(30) == 0` → `_SpidDir = Giro[_SpidDir][S1(8)]`; se `_MeSMode == 8` → `_SpiderScan()` (de novo) |
| 1 andando | anda 1; `S1(20) == 0` → `_SpidDir = Giro[_SpidDir][S1(8)]`; `_Starg = _SFoundAnt()`; ≠ −2 → ACHOU; senão `S1(50) == 0` → `_SMode = 0`, `_Scycle = 2` |
| 2 perseguindo | ver abaixo |
| 3 comendo | ver abaixo |
| 4 fugindo | `_SpidDir = Giro[_SpidDir][S1(8)]`; corre; `S1(50) == 0` → `_SMode = 0`, `_Scycle = 2` |
| 5 morrendo | `_DeathCnt −= 1`; se 0: `_SpidOn = 0`, `_SMode = 0` e `_DropFoodA` três vezes na célula (comida para as formigas) e fim; senão `S1(1000) < _DeathCnt` → `_Scycle = S1(2) + 2` (se `_DeathCnt ≤ 400`) ou `S1(3) + 1` (animação) |

ACHOU: `_SMode = 2`; `_StargLife = T[_Starg]` (ou 0xFF se `_Starg = −1`).

**Estado 2, perseguindo:**

```
se _Starg < 0 (a amarela):
    se _MePlane > 1 (a amarela está num ninho): _SMode = 0 ; _Starg = −2 ; fim
senão se (T[_Starg] ^ _StargLife) & 0xF0 ≠ 0:        ← o alvo mudou (morreu, pegou carga, etc.)
    _SMode = 0 ; _Starg = −2
    se _MeMode == 1: destino do jogador e _MeLoc = célula da aranha ; _MeSMode 6 → 0 ; centraliza (opção AutoTrack)
    fim
dist = |dx| + |dy| até o alvo (a amarela: _MeLocX/Y ; formiga: X/Y da vaga)
se dist > 64 e _MeMode ≠ 1: _SMode = 0 ; _Starg = −2 ; fim       ← desiste
se dist > 1:
    _SpidDir = Giro[_SpidDir][_GetDir(célula, alvo) − 1] ; som ; corre
    se _MeMode == 1: _MeLoc = célula da aranha (e centraliza se AutoTrack)
    fim
_SMode = 3 ; centraliza a aranha na célula ((& 0xFFF0) + 8) e segue direto para o estado 3
```

**Estado 3, comendo.** Célula da boca `b = ((cx + _SeatX[dir]) & 127, (cy + _SeatY[dir]) & 63)`.

```
(se _MeMode == 1: _MeLoc = célula ; _MeSMode 6 → 0 ; centraliza se AutoTrack)
se _Starg == −2:                                    ← já mastigando
    MASTIGA
senão se _Starg < 0 (a amarela):
    _MoveMyLife(_MePlane, b, _MeType, _MeDir) ; _YellowDeath(1) ; _SCorpseBase = 0 ; COMEÇA
senão se (T[_Starg] ^ _StargLife) & 0xF0 == 0:     ← a presa ainda é a mesma
    preta: _BAntsEaten += 1 ; _SCorpseBase = 0      vermelha: _RAntsEaten += 1 ; _SCorpseBase = 4
    Vida(presa) = 0 ; T[_Starg] = 0 ; COMEÇA
senão: _Starg = −2 ; TERMINA
COMEÇA: _Starg = −2 ; _EatCnt = (_MeSMode < 7 ? 50 : 11) ; MASTIGA
MASTIGA:
    se quintal e Mapa(b) < 0x18: Mapa(b) = S4() + _SCorpseBase + 0x10      ← restos na boca
    se _EatCnt > 0:
        _EatCnt −= 1
        se _EatCnt % 10 == 0 e S2() ≠ 0: som com S256()                    ← S256 só se S2 ≠ 0
        fim do tick (continua no estado 3)
    _SpidBurpCnt −= 1 ; se chegou a 0: _SpidBurpCnt = 10 ; se "Silly" ≠ 0 e S2() == 0: arroto
    _DeadAntHere(b, _SCorpseBase)                    ← cadáver definitivo ([superficie.md](superficie.md) 6.4)
    TERMINA
TERMINA: _SMode = 0 ; se _MeMode == 1: _MeSMode 6 → 0 ; destino do jogador = célula da aranha
```

`_DeadAntHere` recebe `_SCorpseBase` (0 ou 4) como "cor": 4 conta como vermelha.

### 2.6 Procurar presa: `_SFoundAnt` (`1020:53f6`) e `_SpiderScan` (`1020:5536`)

**`_SFoundAnt()`**, sem sorteio:

- `_MeSMode == 7` (faro): varre a lista A **de trás para frente** e devolve a primeira vaga viva com `_GetDis(célula, formiga) < 801` (raio ≈ 28). Se nenhuma, e a amarela está na superfície e não é a aranha (`_MeMode == 0`, `_MePlane == 1`), com distância² < 801 → −1. Senão −2.
- senão (visão): anda pela linha reta na direção `_SpidDir`, até 20 células: saiu do mapa → −2; distância² > 400 → −2; célula com `Vida ≠ 0`: a amarela → −1; formiga achada na lista por `_FindAntIndex(1, x, y, T)` → o índice. Nada achado → −2.

**`_SpiderScan()`** (laser; só com `_MeSMode == 8`):

```
se _SMode == 5: devolve −1
base = ((_SpidDir − 2) & 7) · 32                 ← ângulo de 0 a 255 (uma volta), 0 = leste, 64 = sul
repetir 2 vezes:
    para a = base − 32 .. base + 31:
        r = S1(12) + 1
        p = (cx + cos(a)·r / 0x7FFF, cy + sin(a)·r / 0x7FFF)     (_fracCOS/_fracSIN, mesma tabela de _AddFood)
        se p no mapa e Vida(p) ≠ 0 e _FindAntIndex(1, p, Vida(p)) = j ≥ 0:
            desenha o laser (_DoLaserFire) ; se S4() ≠ 0: Vida(p) = 0 ; T[j] = 0 ; _DeadAntHere(p, cor)
            devolve j
devolve −1
```

São até 128 sorteios `S1(12)` por chamada sem alvo. Observação: o laser ignora a amarela (ela não está na lista A).

### 2.7 Aranha controlada pelo jogador (`_MeMode = 1`)

**Fato visto no código.** O jogador vira a aranha pela troca de corpo (ver [formiga-amarela.md](formiga-amarela.md#8-troca-de-corpo-e-alvo-_exchangelives-_dolifeexchange)). Enquanto `_SMode` não é 2 nem 3:

```
se _MeSMode == 7: _Starg = _SFoundAnt() ; se ≠ −2: ACHOU (passa a perseguir) e fim
senão se _MeSMode == 8: _SpiderScan()
dist = _GetDis(_MeLoc, _Suser)
se dist < 1: _Scycle = 2 ; fim                         ← chegou
d = _GetBestDir(1, _MeLoc, _Suser)                       (seção 2.9)
se d == −1: fim ; se d == −2: d = _GetDir(_MeLoc, _Suser) − 1 ; se d < 0: fim
_SpidDir = Giro[_SpidDir][d]
se _MeCmd == 0 ou dist < 3: anda 1   senão: corre (×5)
_MeLoc = (_SpidX >> 4, _SpidY >> 4) ; centraliza se AutoTrack
```

Com AutoTrack (`_OptionStates[0]`, `8:85f2`), `_MoveSpider` chama `_GotoMyAnt` em quatro pontos, sempre depois de pôr `_MeLoc` na célula da aranha [código, `1020:4704`]: depois do passo do controle do jogador (acima; não quando chegou ou não achou direção), na perseguição quando o alvo mudou (estado 2, "o alvo mudou"), a cada passo da perseguição (estado 2, dist > 1) e no começo do estado 3. Dentro do tick, o `_CenterEdit` que move a vista redesenha a janela de edição, e com `_SMode` ≥ 2 ou `_MeSMode` ≥ 6 os palpos sorteiam `S1(4)` ([rng.md](rng.md), "Desenho dentro do tick"). **Port:** `rules::inimigos::autotrack` → `amarela::centralizar` (o mesmo `_GotoMyAnt` das regras). **Verificado (2026-10-19):** vetores `ticks/aranha_jogador_autotrack` (11 ticks: anda com a vista atrás da aranha; persegue no modo alvo com os palpos no meio do tick; come), 307/307 com S, R e vida; sem a correção, 10 ticks erram `_MapPnt` e os da perseguição erram S, R, listas e vida.

`_MeLoc` acompanha a célula da aranha. O destino e `_MeCmd` vêm do clique (`_processSpider`, ver [formiga-amarela.md](formiga-amarela.md#11-controles-do-jogador)). Quando a aranha do jogador sai do mapa (`_YellowDeath(4)`, mensagem 10028) ou é cercada (`_YellowDeath(3)`, mensagem 10026), o jogador reencarna numa formiga.

### 2.8 Outras mortes da aranha: `_KillSpider` (`1020:53d4`)

Põe `_SMode = 5`, `_DeathCnt = 500`, `_Scycle = 0` (agonia; sem mensagem). Chamadores (fato visto no código): o menino lá fora (`_SimKidOutside`) e dentro de casa (`_SimKidInside`), o pé (`_FootFall`), o cortador (`_MowerFall`) e o Inseticida do modo Experimental quando atinge a célula da aranha. `_ScanForAnts` (`1020:5362`) conta as formigas no 3 × 3 como o cerco, mas não tem chamador.

### 2.9 `_GetBestDir(plano, x, y, tx, ty)` (`1028:405e`)

Mesma regra de `_GetRedBestDirs` ([superficie.md](superficie.md), 9.2), trocando o teste de passagem por: `_IsNotObstacle(plano, v)` e o tile **não** ser pedrinha (`_IsThisPebble`). Devolve a melhor vizinha livre, ou a melhor bloqueada, ou −2; −1 se já está no alvo. Sem sorteio.

### 2.10 `_GetStrategy` e a aranha

Com a amarela na superfície, `_ChaseSpid = 1` quando a aranha está a menos de 10 células (distância² < 100): as formigas pretas em Defense passam a ir para a aranha ([superficie.md](superficie.md), 2.5.7). `_ChaseSpid` é zerado a cada tick.

## 3. Lagarta (`_DoPillar`)

### 3.1 Estado

**Fato visto no código; nomes do `.SYM`** ("Pillar" de *caterpillar*).

| Variável | Endereço | Significado |
|---|---|---|
| `_PillarState` | `8:8a8a` | 0 = não existe (será criada no próximo passo), 1 = andando |
| `_PillarX/Y` | `8:8a8c` / `8:8a8e` | cabeça |
| `__PillDir` | `9:9b1e` | 0 sobe (y−1), 1 direita (x+1), 2 desce (y+1), 3 esquerda (x−1) |
| `_PillarSeg` | `9:78d4` | fase do passo (5 → 0) |
| `_PillarMap` | `9:7c0e` | 6 palavras: o chão que estava sob o corpo, indexado por `coordenada % 6` (y se a direção é vertical, x se horizontal) |

Tiles (fato visto no código): cabeça 0x6C (subindo), 0x6B (direita), 0x6F (descendo), 0x68 (esquerda); corpo 0x6D/0x6E (vertical) e 0x69/0x6A (horizontal). Isto confirma a [hipótese] de [tiles-valores.md](tiles-valores.md) de que 0x69–0x6B são a lagarta e completa a faixa 0x68–0x6F.

### 3.2 `_DoPillar` (`1030:4cdc`), só em ticks ímpares e só no quintal

**Fato visto no código.** Primeiro chama `_DoSow` (seção 4). Depois:

```
se _PillarState == 0: _MakeAPill() ; _PillarState = 1 ; _PillarSeg = 4 ; fim
frente = cabeça + passo da direção (dir 0: (X, Y−1) ; 1: (X+1, Y) ; 2: (X, Y+1) ; 3: (X−1, Y))
se frente no mapa e Vida(frente) ≠ 0:                     ← uma formiga bloqueia
    n = células ocupadas no 3 × 3 em volta da cabeça
    se n ≥ 6: _PillarState = 0 ; _MakePillFood()            ← as formigas matam a lagarta
    fim (a lagarta não anda neste tick)
s = _PillarSeg − 1 ; _PillarSeg = s
"atrás(k)" = cabeça − k·passo
se s == 4: restaura o chão em atrás(5) (de _PillarMap)
senão:     grava corpo 0x6D (vertical) ou 0x69 (horizontal) em atrás(s + 1)
grava corpo 0x6E (vertical) ou 0x6A (horizontal) em atrás(s)
se s ≠ 0: fim
avança a cabeça um passo
se X < −6 ou X > 134 ou Y < −6 ou Y > 69: _PillarState = 0 ; fim     ← saiu do quintal
guarda o chão sob a nova cabeça em _PillarMap ; grava a cabeça e, em atrás(1), o corpo 0x6D/0x69
_PillarSeg = 5
```

Todas as escritas no mapa passam por `_IsValidA` (fora do mapa, nada é gravado). Não há sorteio fora de `_MakeAPill`. A lagarta não testa o terreno: passa por cima de qualquer tile (guardando e restaurando o chão) e só para diante de formigas. A cada 5 chamadas (10 ticks) a cabeça avança uma célula; o corpo ocupa a cabeça e as 4–5 células atrás.

**`_MakeAPill` (`1030:53da`):** `dir = S1(4)`; dir 0: cabeça em `(S1(128), 63)`; 1: `(0, S1(64))`; 2: `(S1(128), 0)`; 3: `(127, S1(64))` (um sorteio além do de direção). Guarda o chão e grava a cabeça. A lagarta entra pela borda oposta ao sentido em que anda.

**`_MakePillFood` (`1030:57d2`):** para k = 0..5 ao longo do corpo (cabeça e 5 células atrás), restaura o chão guardado e, se o tile resultante for < 0x18, grava 0x4B (comida com 4 unidades). As funções `_IsPillDead`, `_StorePillarMap`, `_ReplacePillarMap`, `_PlacePillTile` e `_PillGetLife` fazem partes dessa mesma lógica e não são chamadas por `_DoPillar` (que tem tudo embutido).

**Verificado no `QUICK2.ANT`** (tick 78): `_PillarState = 1`, cabeça em (7, 58), `_PillarSeg = 1`; tiles 0x6B em (7, 58), 0x6A em (6, 58) e 0x69 em (3–5, 58), exatamente o desenho previsto para a direção 1 com s = 1.

**Peculiaridade (fato visto no código):** o índice `coordenada % 6` usa o resto com sinal do C; com a cabeça fora do mapa (coordenada negativa) o índice seria negativo, mas as escritas fora do mapa são descartadas por `_IsValidA`. Reproduzir guardando só quando a posição é válida.

## 4. Bicho-de-conta (`_DoSow`)

### 4.1 Estado

**Fato visto no código** ("Sow" de *sowbug*). Três vagas de palavras: `_SowX` (`9:9bc8`), `_SowY` (`9:9bda`), `_SowDir` (`9:9c2a`), `_SowSave` (`9:78cc`, o chão sob o bicho). Tiles por direção: `_SowTab` (`8:8a90`) = `72 73 71 70 72 73 71 70` (fato verificado no `SIMANTW.EXE`).

### 4.2 Criação

A criação está embutida em `_InitPillar` (`1030:4bf8`) e repetida em `_InitSow` (`1030:3ef8`): só no quintal, preenche as vagas **2 e 1** (nessa ordem). Para cada uma, repete até conseguir: `x = S1(128)`, `y = S1(64)`; se `Mapa(x, y) < 0x10`: `dir = S1(8)`, guarda posição, direção e chão, e grava `_SowTab[dir]`. Correção a [geracao-mundo.md](geracao-mundo.md) (4.3): não são "duas tentativas sem repetição"; o laço só termina depois de dois sucessos (fato visto no código).

A vaga **0 nunca é inicializada** (fica com o que estiver na memória: zeros num jogo novo).

### 4.3 `_DoSow` (`1030:3f8a`), chamado no começo de `_DoPillar` (ticks ímpares)

```
para v = 0, 1, 2:
    se S4() ≠ 0:                                       ← 3/4 de chance de agir
        se S4() == 0:                                  ← 1/4 de chance de virar
            _SowDir[v] = (_SowDir[v] + S1(3) − 1) & 7
            Mapa(posição) = _SowTab[_SowDir[v]]          (redesenha no lugar)
        alvo = posição + D8[_SowDir[v]]
        se alvo no mapa e Vida(alvo) == 0 e Mapa(alvo) < 0x10:
            Mapa(posição) = _SowSave[v] ; _SowSave[v] = Mapa(alvo)
            Mapa(alvo) = _SowTab[_SowDir[v]] ; posição = alvo
```

O bicho-de-conta não mata nem é morto; só anda sobre chão (< 0x10). Seus tiles 0x70–0x73 são maiores que `_Barrier` (0x50), portanto obstáculo para as formigas.

**Peculiaridade P-Sow, verificada no `QUICK2.ANT`:** a vaga 0 (nunca inicializada, em (0, 0) com direção 0) é processada como um terceiro bicho-de-conta "fantasma". O save tem `_SowX/Y/Dir = (0,0,0), (32,46,6), (49,8,2)` e os tiles 0x71 em (32, 46) e (49, 8) (as duas vagas reais, `_SowTab[6]` e `_SowTab[2]`) e **0x72 em (0, 0)**, que é `_SowTab[0]` gravado pelo fantasma ao virar. O port deve reproduzir a vaga 0.

## 5. Ordem dos sorteios por tick (resumo)

| Passo do tick | Bicho | Sorteios possíveis, na ordem |
|---|---|---|
| 8 | formigas-leão | `S1(1024)` (sem nenhuma); por formiga-leão: estado 0 `S1(512)` e `S8()` (objeto) ou `S1(8)`, `S1(8)` e `S8()` (formiga); estado 1 `S1(4)`; estado 2 o sorteio de posição de uma nova |
| 9 | aranha | aparecer: `S1(300)`, `S1(1024)`, `S1(2)`; laser `S1(12)`×n e `S4()`; estado 0: `S1(150)`, `S1(30)`, `S1(8)`; estado 1: `S1(20)`, `S1(8)`, `S1(50)`; estado 3: `S4()`, `S2()`, `S256()`, `S2()`, e os de `_DeadAntHere`; estado 4: `S1(8)`, `S1(50)`; estado 5: `S1(1000)`, `S1(2)`/`S1(3)` |
| 10 | bicho-de-conta (3 vagas) e lagarta | por vaga: `S4()`, `S4()`, `S1(3)`; lagarta nova: `S1(4)`, `S1(128)` ou `S1(64)` |

Os detalhes condicionais são os das seções acima.

## 6. Casos de teste sugeridos (bancada QEMU)

Bancada: [docs/bancada-qemu.md](../../bancada-qemu.md). Leitura do save pelos offsets de [save-ant.md](../formatos/save-ant.md).

| # | Regra | Entrada | Esperado | Como observar |
|---|---|---|---|---|
| I1 | Bicho-de-conta fantasma (4.3) | qualquer Quick Game novo | tile 0x70–0x73 em (0, 0) depois de alguns ticks; `_SowX[0] = _SowY[0] = 0` | **já verificado** no `QUICK2.ANT` |
| I2 | Andar do bicho-de-conta | saves a cada ~20 ticks | `_SowX/Y` mudam no máximo 1 célula por tick ímpar; o chão anterior reaparece (`_SowSave`) | campos 47918–47936 e `_MapA` |
| I3 | Lagarta (3.2) | saves sucessivos até a lagarta cruzar o quintal | cabeça avança 1 célula a cada 10 ticks; desenho de corpo como na seção 3.2; ao sair, `_PillarState = 0` e uma nova surge numa borda | campos 47906, 48230–48236 e `_MapA` |
| I4 | Lagarta morta vira comida | Experimental: cercar a cabeça com ≥ 6 formigas | `_PillarState = 0` e até 6 tiles 0x4B no lugar do corpo | `_MapA` |
| I5 | Formiga-leão come (1.3) | Experimental: soltar formigas perto de uma armadilha | vaga removida da lista A (com deslocamento), `_BAntsEaten` +1, `_LionListM = 1`, S = 50, centro 0x39–0x3C | listas A, blocos 67–71, `_MapA` |
| I6 | Formiga-leão cercada | Experimental: ≥ 7 formigas em volta durante o estado 1 | centro 0x3F, `_LionIndex − 1`, mensagem 10050 | captura + campos |
| I7 | Pedrinha na armadilha | Experimental ou amarela: largar uma pedrinha no centro | após em média 512 ticks a pedrinha aparece a 2 células; nenhuma formiga é pega antes disso | `_MapA` |
| I8 | Aranha some e reaparece (2.3) | Quick Game; esperar a aranha sair pela borda | `_SpidOn = 0` e, mais tarde, `_SpidOn = 1`, `_SMode = 1`, `_SpidY` = 1 ou 0x3FF | campos 48274–48290 |
| I9 | Aranha come (2.5) | aproximar formigas da aranha parada | `_SMode` 2 → 3, presa com T = 0, `_EatCnt` 50 → 0, cadáver 0x10–0x17 na célula da boca | campos e listas |
| I10 | Cerco à aranha (2.4) | Experimental: 9 formigas no 3 × 3 da aranha | `_SMode = 5`, `_DeathCnt = 500` decrescendo; ao fim, 3 comidas na célula; `_SpidRevenge` +1 | campos 48060, 48286 |
| I11 | Vingança (2.4) | repetir I10 três e cinco vezes | `_MeSMode = 7` e depois 8 (laser visível na tela) | campo 48194 + captura |
| I12 | Port × original | `.ANT` com bichos em vários estados, N ticks | posições, estados e tiles idênticos | comparação dos blocos citados |

## Implementação no port

Código: `engine/src/rules/inimigos.rs`. Funções públicas: `formigas_leao` (`_DoAntLions`), `aranha` (`_MoveSpider`, com `_SFoundAnt`, `_SpiderScan`, cerco e aranha do jogador), `lagarta` (`_DoPillar`, que chama `bichos_de_conta` = `_DoSow`), `matar_aranha` (`_KillSpider`), `matar_formiga_leao` (`_KillAntLion`) e `achar_formiga_leao` (`_FindInLionList`). `impl RegrasInimigos for RegrasOriginais` liga as três primeiras ao tick.

Dados: `data/inimigos.json` (`TabelasInimigos`), gerado por `python tools/extrair_inimigos.py SIMANTW.EXE --json data/inimigos.json` (endereços no topo do script): `_Dx8/_Dy8`, `_Dx9/_Dy9`, `_TurnTab`, `_SpX/_SpY`, `_SeatX/_SeatY`, `_SowTab`, a borda da armadilha (`DGROUP:25e8`) e, no bloco `regras`, as constantes (chances, tempos, tiles, limites), com a função de origem em `_origem`. Como `RegrasOriginais` (em `tick.rs`) ainda não tem campo para estas tabelas, o tick usa a cópia embutida na compilação (`TabelasInimigos::padrao()`); `TabelasInimigos::carregar(&Recursos)` já lê com mods, falta só guardar o resultado em `RegrasOriginais`. O seno do laser vem de `RegrasOriginais::geracao` (objeto 9/1000); sem ele, usa `trunc(sin(i·π/128)·32767)`, que coincide com os 64 valores do banco (verificado: teste `seno_calculado_igual_ao_do_banco`).

Decisões e detalhes reproduzidos (fato visto no código, salvo indicação):

- `_DoAntLions` lê o `_LifeA` das vizinhas pelo índice linear `x·64 + y` sem testar a validade (na borda de cima/baixo cai na coluna vizinha). O port faz o mesmo; fora do bloco de 8 KB devolve 0 [hipótese: no original seria memória de outra variável].
- Morte da formiga-leão por cerco: a seguinte (deslocada para a vaga k) não roda no mesmo tick (correção desta spec, ver 1.3).
- Uma formiga-leão criada no estado 2 (a cada 16 refeições) roda no mesmo tick, porque o laço relê `_LionIndex`.
- `_RemoveFromAList` deixa a última vaga usada repetida (como o `_BlockMove` do original).
- O desenho do laser e o centralizar da vista (AutoTrack) não são reproduzidos. Os sons viram `Evento::Som` (ver [som.md](som.md)); os sorteios ligados a sons (`S8()` da formiga-leão, `S2()`/`S256()` da mastigação, `S2()` do arroto) são feitos no mesmo lugar, com ou sem o pedido.
- Mensagens 10050 e 10046 vão para `Estado::eventos` (`Evento::Mensagem`).
- `_DropFoodA` na casa, no caso "a comida cai": superficie.md 8.2 (port: `Estado::passo_queda`). A aranha morta não usa o resultado.
- `_MoveMyLife` e `_YellowDeath` (causas 1 aranha, 2 formiga-leão, 3 cercada, 4 fugiu do mapa) são os de `rules/amarela.rs` (`mover_marca`, `matar_amarela`), com animações que sorteiam, mensagens e reencarnação (formiga-amarela.md §9). `aranha` e `formigas_leao` recebem as tabelas da amarela.
- `_IsClearTile`/`_IsClear3x3`, `_IsNotObstacle`, `_IsThisPebble`, `_IsItFood`, `_GetDis`, `_GetDir`, `_GetBestDir`, `_FindInAList`, `_FindAntIndex`, `_DeadAntHere`, `_DropFoodA` e `_KillSpider` estão em `engine/src/rules/comum.rs` (ver [../../decisoes.md](../../decisoes.md), "Módulo comum das regras"); `_RemoveFromAList` continua local (só os inimigos o usam).

Verificado:

- Testes de unidade em `engine/tests/regras_inimigos.rs` para I1–I11 (bicho fantasma e andar do bicho-de-conta; lagarta avançando, saindo e nascendo, cercada e virando comida; formiga-leão comendo, olhando a vizinha sorteada, pegando a amarela, cercada, cuspindo a pedrinha, nascendo; aranha aparecendo, saindo pela borda, vendo e por faro, perseguindo e comendo, desistindo, cerco e vingança, fuga, agonia e comida, laser, aranha do jogador), incluindo a ordem e o número de sorteios onde ela é determinada pela spec.
- **Contra o original** (bancada QEMU, casos em `tests/referencias/casos/`, saves em `tests/referencias/saves/I0n_INICIO.ANT` = estado logo depois do load e `I0n_FIM.ANT` = estado final): I-01 (lagarta: cabeça de (7, 58) a (14, 58) no tick 140), I-02 (formiga-leão come a operária posta no centro; no tick 200 voltou a esperar, `_AntsEatenByLions = 1`), I-03 (aranha com `_DeathCnt = 40` em (100, 10): no tick 130, `_DeathCnt = 0` e 0x4A na célula). Todas as conferências passaram (2026-10-17). Os testes `caso_i0n_*` rodam o port do estado inicial até o mesmo `_GameTime` e comparam o que não depende do gerador S: lagarta (campos, `_PillarMap` e a linha do mapa), formiga-leão (estado, quadro, `_AntsEatenByLions`, centro) e agonia + célula da aranha. Todos iguais.
- **Vetores de tick** (`tests/referencias/ticks/`, [testes-tick.md](testes-tick.md)), ticks 79–83: partindo do "estado antes", formigas-leão, aranha (andando, sem virar) e lagarta saem iguais ao "estado depois". Como quintal e água sorteiam antes dos inimigos no tick, a semente S no momento dos inimigos não é conhecida; a comparação de sorteios exige as outras partes do tick.

- **Vetores de tick da quinta rodada** (2026-10-18, `tools/cenarios_regras.py` a partir do snapshot `rico`, [testes-tick.md](testes-tick.md#quinta-rodada-2026-10-18-regras-não-verificadas-da-revisão)), todos 307/307 com S, R e vida iguais: cerco da aranha com 9 pretas (`aranha_cerco`: estado 5, `_DeathCnt = 500`, mensagem 10046, vingança 2 → 3 e `_MeSMode = 7`), com 6 (`aranha_foge`: estado 4) e com `_SpidRevenge = 4` (`aranha_vinganca`: `_MeSMode = 8`); faro (`aranha_faro`: `_SFoundAnt` de trás para frente, perseguição, `_EatCnt = 11`, `_DeadAntHere`); laser (`aranha_laser`: duas mortes pelo `_SpiderScan` num leque de pretas); cerco da formiga-leão (`leao_cerco`: 7 vizinhas, 10050, centro 0x3F, lista deslocada); aranha do jogador pela interface do original (`aranha_jogador`: tecla X e clique na aranha, andar, correr, modo alvo com `_MeSMode = 6` e comer com `_EatCnt = 50`); a aranha do jogador cercada (`aranha_jogador_cercada`, `_YellowDeath(3)`) e saindo do mapa (`aranha_jogador_fora`, `_YellowDeath(4)`); a amarela comida pela formiga-leão (`amarela_morte_leao`, `_YellowDeath(2)`). Achado: **o cerco não roda para a aranha do jogador com `_SMode` 0 ou 1** — o controle do jogador (2.7) substitui o resto de `_MoveSpider`, como a seção 2.2 já dizia; ela só é cercada perseguindo ou comendo.
- **Pendente:** a amarela comida pela aranha (`amarela_morte_aranha/1793`, `_YellowDeath(1)`): 294/307. Causa isolada com breakpoints em `_SRand1` e nas funções de desenho (`cenarios_regras.py contar_aranha`): o original sorteia 4 passos de S a mais, `S1(4)` de `_DrawPalps` em três desenhos da janela de edição dentro do tick (o de `_GotoMyAnt` na morte, o de antes do laço de `_AnimYellowFight` e o do 1º quadro; a aranha está à vista e com `_SMode` ≥ 2) e o `S1(2)` de `_SpiderDialog` (opção "Events"). O port não desenha dentro do tick ([rng.md](rng.md), "Fora"). Não é erro de regra; a morte pela formiga-leão, sem a aranha à vista, bate (contagem de controle `contar_leao`).

Não verificado contra o original: a pedrinha cuspida pela formiga-leão e o desenho do laser (`_DoLaserFire`).

## Lacunas

- O sentido de andar de ré (`_RevSpider`) não é ligado por nenhuma rotina vista [em aberto].
- `_DoLaserFire` e a animação das pernas (`_Scycle`) são só desenho; os sorteios dentro de `_DrawSpider` consomem o gerador S ([rng.md](rng.md)) e precisam ser medidos.
- Menino, cachorro, gato, pássaro, cortador, chuva e inseticida: documento próprio.
- Queda de comida na casa ao morrer a aranha dentro de casa (`_DropFoodA` com a tabela de queda).
- Conferido rodando o original: a lagarta andando (I-01), a formiga-leão comendo uma formiga (I-02) e a aranha em agonia virando comida (I-03), os vetores de tick 79–83 e, desde 2026-10-18, cerco da aranha e da formiga-leão, fuga, vingança, faro, laser, aranha do jogador (andar, correr, modo alvo, cercada, fora do mapa) e as mortes da amarela 2, 3 e 4 (seção "Verificado"). Falta: pedrinha cuspida. A morte 1 (pela aranha), que divergia só pelos sorteios do desenho dentro do tick, bate desde 2026-10-18 (vetor `amarela_morte_aranha`, [rng.md](rng.md) "Desenho dentro do tick").
