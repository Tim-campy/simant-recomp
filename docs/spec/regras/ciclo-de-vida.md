# Ciclo de vida: ovos, rainha e castas

Este documento especifica, para reimplementação 1:1, como a rainha põe ovos, como o ovo se desenvolve até virar adulto, como a casta do adulto é escolhida e como a rainha (duas células, cabeça e cauda) vive, anda e morre. O comportamento das demais formigas dentro do ninho está em [ninho.md](ninho.md); listas, codificação de T, tabelas de modo e o triângulo de castas, em [formigas.md](formigas.md); a ordem do tick, em [simulacao-visao-geral.md](simulacao-visao-geral.md); os geradores, em [rng.md](rng.md).

## Convenções

As mesmas de [ninho.md](ninho.md#convenções): fato visto no código salvo aviso; `S(n)`, `Sk(K)`, `SG(n)` = gerador S; `RR(n)` = gerador R; `D8[d]` e `inv(d) = d XOR 4`; X, Y, M, T, S da formiga em processamento (índice `_Tindex`); `Vida` = `_LifeB`/`_LifeR`. Pseudocódigo próprio, não cópia do decompilado.

## 1. Visão geral

| Fase | Representação | Quem processa |
|---|---|---|
| ovo e cria | entrada da lista do ninho com grupo 0: T = estágio 1..7 (preto) ou 0x80 + estágio (vermelho), M = 8 | `_SimEggB` (`1028:3ca0`) / `_SimEggR` (`1028:62a6`) |
| ovo carregado | não existe na lista: o estágio fica no S da operária que o carrega (só no ninho preto; seção 4) | `_DoNestingB` |
| adulto | T = grupo × 8 + direção (+ 0x80 se vermelho) | [ninho.md](ninho.md) e `_DoAntSimA` |
| rainha | duas entradas: cabeça (grupo 12) e cauda (grupo 13), M = 9 | `_SimQueenB` (`1028:3dc2`) / `_SimQueenR` (`1028:6386`) |

Estágios (bits 0–2 de T). Os nomes vêm da lista de texto SHARED 4/1100, na ordem: Weird Egg, New Egg, Egg, 1st-Instar, 2nd-Instar, 3rd-Instar, Pupae, Hatching Pupae ([formigas.md](formigas.md#3-codificação-do-tipo-t)). [hipótese: o índice na lista é o estágio; "Weird Egg" (0) nunca aparece, pois T & 0x7F = 0 é vaga livre]

| T & 7 | Nome | Como chega aí |
|---|---|---|
| 1 | New Egg | posto pela rainha (preta e vermelha) e pela amarela-rainha |
| 2 | Egg | 1 avanço; ou ovo vermelho largado por uma ama (sempre 0x82) |
| 3 | 1st-Instar | |
| 4 | 2nd-Instar | |
| 5 | 3rd-Instar | |
| 6 | Pupae | |
| 7 | Hatching Pupae | |
| (8) | eclode | o avanço que levaria a 8 transforma a entrada em adulto |

## 2. Postura (cauda da rainha)

`_SimQueenB`, ramo da cauda (grupo 13), executado a cada tick:

```
Vida[pos] = T
se _BlkQueens > 0:                                          (R: _RedQueens)
    h = pos + D8[T & 7]                                      (a cabeça fica à frente da cauda)
    se Vida[h] ≠ T − 8 e _FindInBList(h, T − 8) < 0:          (cabeça sumiu)
        _BlkQueens −= 1; T = 0; Vida[pos] = 0; FIM
e = pos + D8[inv(T & 7)]                                     (célula atrás da cauda)
se 0 ≤ e.x ≤ 63 e 1 ≤ e.y ≤ 63 (_InNestBounds):
    _LastBlackEgg = e                                        (R: _LastRedEgg, 8:8366)
    se (_Cycle & 0xF) == 0:
        r = Sk(128)
        se r ≤ _HealthB:
            _PlaceEggB(e, 1)                                  (R: _PlaceEggR(e, 0x81))
            _EatCountB −= 1
            se _EatCountB < 0:
                _EatCountB = _BpopT >> 5
                se _HealthB > 0 e _AlwaysHealthy == 0: _HealthB −= 1     (R: sem o teste de _AlwaysHealthy)
            _TotalEggsLaidB += 1                              (32 bits; R: não há contador)
```

- Chance de pôr, por oportunidade (a cada 16 ticks): (`_HealthB` + 1)/128, ou seja, 101/128 ≈ 79 % com saúde 100.
- `_PlaceEggB` ([ninho.md](ninho.md#76-pôr-um-ovo-_placeeggb-10201004--_placeeggr-10201068)) cava a célula se for terra (a rainha alarga a câmara pondo ovos), acrescenta o ovo no **fim** da lista e não testa ocupação: vários ovos podem ficar na mesma célula. Com a lista cheia (500), o ovo não é criado, mas a saúde e o contador são cobrados mesmo assim.
- O custo em saúde: enquanto `_BpopT` < 32, `_EatCountB` volta a 0 e **cada ovo custa 1 de saúde**; a reserva só sobe comendo ([ninho.md](ninho.md#61-comer-_tryeatfoodb-102847c6-_eatfoodb-10284844)).
- `_LastBlackEgg` é atualizado em todo tick em que a célula de trás está no ninho, ponha ovo ou não.

**Fato verificado no `QUICK2.ANT`** (tick 78, por conta manual sobre os valores salvos):

- `_HealthB` = 96 = 100 − 1 (alimentação do tick 64) − 3 (três ovos); `_HealthR` = 95 = 100 − 1 − 4 (quatro ovos); `_EatCountB` = `_EatCountR` = 0; `_TotalEggsLaidB` = 3. As oportunidades foram os ticks 16, 32, 48 e 64.
- Ovos pretos (T = 1) em (34, 9) e duas vezes em (36, 9); ovos vermelhos (T = 0x81) quatro vezes em (39, 11), atrás da cauda vermelha (40, 11) virada para a direita. `_LastBlackEgg` = (36, 9), atrás da cauda preta (35, 9) virada para a esquerda.

## 3. Desenvolvimento e eclosão

### 3.1 Ninho preto: `_SimEggB` (`1028:3ca0`)

```
t = T; nasceu = falso
P = (_BpopT < 3) ? 32 : 128
se (_Cycle mod P) == 0:
    t = t + 1
    se (t & 0xF) == 8:                                         (eclode)
        se _ModeAuto == 0 e SG(255) > (_modeLevels[2] >> 7):   (SG só com _ModeAuto = 0)
            t = 0; _TotalEggsDiedB += 1                         (morreu)
        senão:
            c = _MakeMe                                         (2, 6, 4 ou 8; seção 6)
            t = c × 8 + 2                                       (virado para a direita)
            M = (c == 2) ? 1 : _GetNewModeB(c)
            nasceu = verdadeiro
se _OptionStates[5] ≠ 0 e não nasceu: balão de ovo (sem sorteio)
Vida[pos] = t; T = t; S = 0
```

- **Todos os ovos avançam no mesmo tick**, pois o teste é sobre `_Cycle`, comum a todos. Um ovo posto no tick t avança nos 7 ticks seguintes com `_Cycle` múltiplo de P (o próprio tick de postura não conta: o ovo novo fica no fim da lista e só é processado no tick seguinte). Duração de um ovo posto pela rainha (sempre num tick múltiplo de 16): entre 6P + 16 e 7P ticks, isto é, 784–896 ticks numa colônia com 3 ou mais adultos e 208–224 numa colônia nascente (se P não mudar no meio). Um ovo largado por uma ama, em qualquer tick, leva entre 6P + 1 e 7P a partir daí, contando o estágio que já tinha.
- P é escolhido a cada tick com o `_BpopT` atual, que só muda quando `_CountAnts` roda (ticks ≡ 0 e 64 mod 128).
- `_Cycle` vai de 0 a 0x1000 (4097 valores) e volta a 0: os ticks com `_Cycle` = 0x1000 e 0 são consecutivos e **ambos** contam, então uma vez a cada 4097 ticks os ovos avançam dois estágios em dois ticks seguidos. O mesmo vale para a postura (seção 2).
- Morte na eclosão: só com `_ModeAuto` = 0 (o jogador mexeu no triângulo de tarefas; [formigas.md](formigas.md#escolha-de-modo)). Com L = `_modeLevels[2] >> 7` (nível de "cuidar", 0..511) e `SG(255)` = mínimo de dois sorteios 0..254, a chance de morrer é ((254 − L)/255)² se L < 254, e 0 se L ≥ 254. Exemplos: cuidar 0 % → 99,2 %; 20 % (13107, L = 102) → 35,5 %; 50 % (L = 255) → 0.
- Modo ao nascer (`_GetNewModeB`, tabelas em [formigas.md](formigas.md#escolha-de-modo)): operária → 1 (Nesting), sem sorteio; soldado → `_ModeTabSB[_StrategicModeB][Sk(8)]` com `_ModeAuto` = 1, ou `_ModeMe` com 0; macho e rainha nova → 14 (Pre-Flight), sem sorteio.
- Um ovo na lista também passa pelo sorteio de morte natural do ninho ([ninho.md](ninho.md#31-formiga-dona)), antes de `_SimEggB`.

### 3.2 Ninho vermelho: `_SimEggR` (`1028:62a6`)

```
t = T; nasceu = falso
P = (_RpopT == 1) ? 32 : 128                                    (exatamente 1, não "< 3")
se (_Cycle mod P) == 0:
    t = t + 1
    se (t & 0xF) == 8:
        c = _CasteTabC[(_StrategicModeR mod 7) × 8 + Sk(8)]
        t = c × 8 + 0x82
        M = _GetNewModeR(c)                                     (Sk(8) dentro, se c = 2 ou 6)
        nasceu = verdadeiro
balão como no preto; Vida[pos] = t; T = t; S = 0
```

Sem morte na eclosão. Ordem dos sorteios na eclosão: o `Sk(8)` da casta e, depois, o `Sk(8)` de `_GetNewModeR` (para operária e soldado).

`_CasteTabC` (`8:897e`), linha = `_StrategicModeR` (0..5), coluna = `Sk(8)`, valores lidos do `SIMANTW.EXE` (2 = operária, 4 = macho, 6 = soldado, 8 = rainha nova):

| `_StrategicModeR` | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | Resumo |
|---|---|---|---|---|---|---|---|---|---|
| 0 (ataque) | 2 | 2 | 2 | 2 | 2 | 6 | 6 | 6 | 5/8 operária, 3/8 soldado |
| 1 (normal) | 4 | 4 | 8 | 8 | 2 | 2 | 2 | 6 | 1/4 macho, 1/4 rainha nova, 3/8 operária, 1/8 soldado |
| 2 | 4 | 8 | 2 | 2 | 2 | 2 | 6 | 6 | 1/8 macho, 1/8 rainha nova, 1/2 operária, 1/4 soldado |
| 3 | 2 | 2 | 2 | 2 | 2 | 6 | 6 | 6 | como 0 |
| 4 (fome) | 2 | 2 | 2 | 2 | 2 | 2 | 6 | 6 | 3/4 operária |
| 5 (fome) | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 6 | 7/8 operária |

A linha 6 (alcançável só se `_StrategicModeR` = 6, que não ocorre) seria o começo de `_CasteTabB`.

Modo ao nascer no vermelho: operária e soldado sorteiam em `_ModeTabWB/SB[_StrategicModeR]`; macho e rainha nova → 14.

## 4. Ovos carregados pelas amas

| | Ninho preto (`_DoNestingB`) | Ninho vermelho (`_DoNestingR`) |
|---|---|---|
| Pegar | operária livre (grupo 2, S < 8) cuja célula mostra um ovo em `Vida`: o ovo sai da lista (T = 0), a ama vira grupo 1 e guarda o T do ovo em S | operária livre, com chance 1/4 por tick; o ovo sai da lista; S não guarda nada; aceita ovo preto ou vermelho |
| Levar | desce pelo mapa de saída até um mínimo local; marca S bit 3; no tick seguinte larga | anda ao acaso ou desce (3/4 desce, se houver para onde) |
| Largar | `_PlaceEggB(pos, S & 7)`: **mesmo estágio** | com chance 1/4 em célula sem comida: `_PlaceEggR(pos, 0x82)`: **volta ao estágio 2** |
| Durante o transporte | o ovo não envelhece | idem |

Detalhes e ordem de sorteios em [ninho.md](ninho.md#5-cuidar-das-crias-m--1-_donestingb-102844a8). Consequência: no ninho vermelho, um ovo que é carregado uma vez reinicia o desenvolvimento; quanto mais amas, mais lento o nascimento [hipótese sobre o efeito no jogo].

## 5. Rainha

### 5.1 Forma

- Duas entradas na lista do ninho, ambas com M = 9 e S = 0: **cabeça** T = 0x60 + d (preta) / 0xE0 + d (vermelha) e **cauda** T = cabeça + 8, na célula atrás da cabeça: cauda = cabeça + D8[inv(d)]. A cauda tem a mesma direção d. Os ovos saem atrás da cauda: cabeça + 2 × D8[inv(d)].
- Criação: na geração do mundo (`_PlaceBlackQueen`, rainha vermelha em linha no `_RandWorld`; [geracao-mundo.md](geracao-mundo.md#3c-rainhas-_placeblackqueen-103065ce-e-a-rainha-vermelha)), com T = 0x62/0x6A (0xE2/0xEA). Em jogo: `_MakeBlkQueen(x, y, d)` (`1030:671a`) / `_MakeRedQueen` (`1030:6906`): cava (x, y), (x, y) + D8[inv(d)] e (x, y) + 2·D8[inv(d)] com `_DigTile*`, acrescenta cabeça e cauda e soma 1 em `_BlkQueens`/`_RedQueens`. Chamadores: `_YellowBirth` (`1000:739e`, a amarela-rainha deixa uma rainha de verdade ao trocar de corpo; detalhes com a formiga amarela) e `_CheatKeys`.
- Contadores `_BlkQueens` (`9:78e8`) e `_RedQueens` (`9:79dc`): sobem na criação; descem quando a cabeça percebe que a cauda sumiu (sempre) ou quando a cauda percebe que a cabeça sumiu (só se o contador for > 0).
- A rainha não sofre a morte natural ([ninho.md](ninho.md#31-formiga-dona): M = 9 é excluído) nem o afogamento (grupos 12–13 ficam fora de `_DrownBList`).

### 5.2 Cabeça, a cada tick (`_SimQueenB`, grupo 12)

**Verificado (2026-10-19, vetores `rainha_fome` e `rainha_vermelha_fome`, testes-tick.md "Sétima rodada"):** com `_HealthB` = 0 a cabeça preta morre no 1775 com o diálogo 10015 aberto dentro do tick e a cauda some no tick seguinte, quando `_BlkQueens` vai a 0; o mesmo na vermelha (10016) com `_HealthR` = 0. A imunidade à morte natural (M = 9) está no vetor `ninho_n02`.

```
se Sk(64) == 0:
    se _HealthB == 0:                                   (R: _HealthR)
        T = 0; Vida[pos] = 0
        diálogo 10015 "The black queen in this patch starves from lack of nourish…" (R: 10016, "The red queen…")
        FIM                                             (_BlkQueens não muda aqui; a cauda cuida disso)
    se ANDAR_RAINHA(): FIM                              (5.3)
c = pos + D8[inv(T & 7)]
se Vida[c] ≠ T + 8 e _FindInBList(c, T + 8) < 0:         (cauda sumiu)
    _BlkQueens −= 1; T = 0; Vida[pos] = 0
senão Vida[pos] = T
balão de rainha se _OptionStates[5] ≠ 0
```

Morte por fome: com saúde 0, chance 1/64 por tick (o mesmo sorteio que decide se ela tenta andar). O fim de jogo vem depois, quando `_CountAnts` não acha mais a rainha ([simulacao-visao-geral.md](simulacao-visao-geral.md#8-fim-de-jogo-e-pontuação)).

### 5.3 Andar: `_QueenMoveB` (`1028:4154`) e `_GetBestDir` (`1028:405e`)

A rainha anda em direção ao **centro de massa das escavações** do próprio ninho (`_TileMassXB/YB`, `9:7c48`/`9:7c90`; R: `_TileMassXR/YR`), recalculado a cada célula cavada ([ninho.md](ninho.md#73-funções-de-escavação)).

`_GetBestDir(plano, x, y, alvoX, alvoY)`, devolve direção, −1 (já no alvo) ou −2 (nenhuma vizinha aproxima):

```
lim = dist²((x, y), alvo)                        (_GetDis: dx² + dy²)
se lim ≤ 0: devolve −1
melhor = −1; reserva = −2
para k = 0..7:
    c = (x, y) + D8[k]; v = Mapa(c)
    se _IsNotObstacle(c) e não _IsThisPebble(v) e dist²(c, alvo) < lim:      (no ninho: v < 0x19)
        lim = dist²(c, alvo)                                                (o limite cai a cada candidata)
        se Vida(c) == 0 e _IsClearTile(c) (v < 8): melhor = k
        senão: reserva = k
devolve (melhor ≥ 0) ? melhor : reserva
```

Note que o limite é atualizado também pelas candidatas ocupadas, e que a reserva é uma célula ocupada (ou com comida, ou 0x14/0x18) que aproxima: a rainha anda para cima de ovos e da própria cauda.

`ANDAR_RAINHA()`:

```
d = _GetBestDir(2, X, Y, _TileMassXB, _TileMassYB)
se d == −1: devolve falso
se d == −2: d = Sk(8)
se não (Y > 2 ou 3 ≤ d ≤ 5): devolve falso              (perto da superfície só anda para baixo)
se não MOVER(d): devolve falso                           ([ninho.md](ninho.md#41-andar-_trymovedirb-1028439e--_trymovedirr-10286850-e-passeio))
c = posição antiga + D8[inv(dir antiga)]                 (onde estava a cauda)
Vida[c] = 0
j = _FindInBList(c, 0x68 + dir antiga)                   (R: 0xE8 + dir antiga)
se j ≥ 0 e T[j] ≠ 0: X[j], Y[j] = posição antiga da cabeça; T[j] = 0x68 + d; Vida[posição antiga] = T[j]
devolve verdadeiro
```

A cauda ocupa a posição antiga da cabeça, com a nova direção. Se a rainha anda para trás (d = inv(dir)), cabeça e cauda trocam de lugar. Logo depois de andar a célula da cabeça pode ficar com `Vida` = 0 até a próxima regravação (quando ela entra na célula antiga da cauda). No vermelho, o Ghidra mostra o tipo procurado como `(d & 7) − 0x18`; a desmontagem (`1028:66a6`) confirma que é `0xE8 + d` em byte.

**Fato verificado no `QUICK2.ANT`** (por reconstrução manual): a rainha preta foi criada com a cabeça em (36, 9) virada para a direita (T = 0x62) e a cauda em (35, 9); o centro de massa salvo é (32, 6). Na primeira tentativa, a única vizinha que aproxima é a da cauda (ocupada), então `_GetBestDir` devolve a reserva 6 e a rainha anda para trás: cabeça (35, 9) T = 0x66, cauda (36, 9) T = 0x6E. Na segunda, a única que aproxima é (34, 9), ocupada pelo primeiro ovo: cabeça (34, 9), cauda (35, 9). É exatamente o que o save mostra (lista B: 0x66 em (34, 9), 0x6E em (35, 9)), e os ovos postos depois disso caem em (36, 9), atrás da nova cauda.

### 5.4 Rainha em luta

- Se uma invasora está na célula da cabeça, a cabeça é apagada e a entrada vira nuvem de luta com `_GetWinner(0, invasora)`; a rainha não volta ([ninho.md](ninho.md#32-formiga-invasora)). A cauda some pelo teste da seção 2.
- Se uma nuvem de luta termina com T de cabeça de rainha (grupo 12), `_DoNestFightB/R` recria a cauda atrás dela, sem mexer em `_BlkQueens` (`_MakeNewTailB` embutido; [ninho.md](ninho.md#44-nuvem-de-luta-_donestfightb-10283a54--_donestfightr-10286072)). Isso exige que uma cabeça de rainha seja a "inimiga encontrada" numa checagem de luta (no ninho preto a faixa checada, 0x88–0xE7, inclui a cabeça vermelha 0xE0–0xE7), o que não acontece em jogo normal, pois cada rainha só existe na lista do próprio ninho. [hipótese: código de segurança sem efeito prático]
- `_LostHeadB` (`1028:42de`), `_LostTailB` (`1028:433c`) e `_KillTailB` (`1028:42b0`) são os testes e a remoção descritos acima, em funções separadas sem chamador.

## 6. Escolha da casta

### 6.1 Ninho preto: `_SetCasteProd` (`1030:026e`), todo tick, dentro de `_GetStrategy`

```
p = soma de _CastePopB[1..4]; q = soma de _IdealCaste[0..3]
real[i]  = (p ≥ 1) ? 100 × _CastePopB[i+1] / p : 0          (i = 0..3; divisão inteira de 32 bits)
ideal[i] = (q ≥ 1) ? 100 × _IdealCaste[i] / q : 0
menor = 0; e = 0
para i = 0..3: se real[i] − ideal[i] < menor: menor = real[i] − ideal[i]; e = i      (empate: o primeiro)
_MakeMe = _CasteTabB[e]                                      (8:89ae = 2, 6, 4, 8)
```

`_CastePopB` = (ovos, operárias, soldados, machos, rainhas novas, rainha), atualizado por `_CountAnts` a cada 64 ticks; `_IdealCaste` vem do triângulo de castas ([formigas.md](formigas.md#4-castas-e-o-triângulo-de-castas)). Sem sorteio. O ovo eclode com o `_MakeMe` do momento.

### 6.2 Ninho vermelho

Não há `_SetCasteProd` vermelho: a casta é sorteada na eclosão por `_CasteTabC` (seção 3.2), conforme o modo estratégico vermelho.

## 7. Perdas de ovos

| Causa | Preto | Vermelho | Contador |
|---|---|---|---|
| morte natural do ninho (`Sk(256)` = 0 e `Sk(32)` > saúde) | sim | sim | `_BAntsExpired` / `_RAntsExpired` |
| eclosão com `_ModeAuto` = 0 | sim | não | `_TotalEggsDiedB` |
| invasora pisa a célula do ovo (rouba como comida) | sim | sim | `_TotalEggsDiedB` (só no preto) |
| ovo de tipo preto (T < 8) dentro do ninho vermelho | — | destruído no tick seguinte | nenhum |
| lista cheia na postura | não é criado | idem | `_TotalEggsLaidB` conta mesmo assim (preto) |
| inundação | não afeta | não afeta | — |

## 8. A formiga amarela como rainha (resumo)

Fato visto em `_DoAntSimY` (`1028:7cf6`); regra completa com a formiga amarela. Com `_MeType` = 0x60 dentro de um ninho (`_MePlane` > 1), quando `_Cycle & 0xF` = 0: célula e = posição + 2 × D8[inv(`_MeDir`)]; se `_InNestBounds(e)` e `Sk(128)` ≤ `_MeHealth`: se `_MeColor` = 0, `_PlaceEggB(e, 1)` e `_DecEatB`; senão `_PlaceEggR(e, 1)` e `_DecEatR`; e `_MeHealth −= 5`. A lista é escolhida pela **cor**, não pelo plano; e o ovo vermelho é posto com T = 1 (tipo preto), que `_DoNestAntR` destrói no tick seguinte. [hipótese: defeito; só importa se a amarela for vermelha]

## 9. Diferenças vermelho × preto

| Ponto | Preto | Vermelho |
|---|---|---|
| Período do ovo | 32 se `_BpopT` < 3, senão 128 | 32 se `_RpopT` = 1, senão 128 |
| Casta | `_MakeMe` (triângulo do jogador) | `_CasteTabC[_StrategicModeR][Sk(8)]` |
| Morte na eclosão | com `_ModeAuto` = 0 | nunca |
| Modo ao nascer | operária → 1 fixo; soldado com `_ModeAuto`/`_ModeMe` | `_GetNewModeR` (operária e soldado sorteiam) |
| Ovo carregado | guarda o estágio | volta ao estágio 2 |
| Postura | `_TotalEggsLaidB`, respeita `_AlwaysHealthy` | sem contador, sem `_AlwaysHealthy` |
| Fome da rainha | mensagem 10015 | 10016 |

## 10. Casos de teste contra o original

Mesma bancada e mesmo método de [ninho.md](ninho.md#13-casos-de-teste-contra-o-original) (editar o `QUICK2.ANT`, carregar, rodar, salvar). D = determinístico; E = estatístico.

| Id | Regra | Entrada | Esperado | Tipo |
|---|---|---|---|---|
| C-01 | avanço dos ovos (3.1, 3.2) | `QUICK2.ANT` sem mudanças (tick 78; `_BpopT` = 3, `_RpopT` = 2) | salvando entre os ticks 128 e 255: ovos pretos com T = 2 e vermelhos com T = 0x82, todos ao mesmo tempo; entre 256 e 383: 3 e 0x83 (vale se `_BpopT` continuar ≥ 3 e `_RpopT` ≠ 1, e se nenhum ovo morreu ou foi carregado) | D |
| C-02 | período curto (3.1) | QUICK2 com `_BpopT` = 2 e um ovo T = 7 | o ovo eclode no primeiro tick com `_Cycle` múltiplo de 32 (tick 96) e vira `_MakeMe` × 8 + 2 (soldado 0x32 com `_MakeMe` = 6, como no save), M = `_ModeTabSB[1][Sk(8)]` = 1 ou 2 | D (tipo), E (modo) |
| C-03 | período do vermelho (3.2) | QUICK2 com `_RpopT` = 1 | ovos vermelhos avançam no tick 96; com `_RpopT` = 2 só no 128 | D |
| C-04 | morte na eclosão (3.1) | QUICK2 com `_ModeAuto` = 0, `_modeLevels[2]` = 0, 20 ovos T = 7 (`_BpopT` ≥ 3) | no tick 128 ≈ 99 % morrem (`_TotalEggsDiedB` +≈ 20); com `_modeLevels[2]` = 65535 nenhum morre | E / D |
| C-05 | casta vermelha (3.2) | QUICK2 com `_HealthR` = 20 (o que faz `_GetStrategy` dar `_StrategicModeR` = 5 a cada tick; editar só `_StrategicModeR` não adianta, ele é recalculado) e 40 ovos vermelhos T = 0x87 | no tick 128 nascem ≈ 35 operárias (0x90–0x97) e ≈ 5 soldados (0xB0–0xB7), nenhum macho ou rainha nova | E |
| C-06 | postura e custo (2) | QUICK2 com `_HealthB` = 100 e `_EatCountB` = 0 | a cada 16 ticks, ≈ 79 % de chance de um ovo novo em (36, 9); `_HealthB` cai 1 por ovo (mais 1 a cada 64 ticks); `_TotalEggsLaidB` sobe 1 por ovo | E |
| C-07 | sem postura com saúde 0 (2) | QUICK2 com `_HealthB` = 0 | nenhum ovo é posto (`Sk(128)` ≤ 0 só se der 0: chance 1/128 por oportunidade); a cabeça morre de fome em média em 64 ticks com o diálogo 10015 | E |
| C-08 | andar da rainha (5.3) | QUICK2 com `_TilesDugB` = 1000, `_TileTotXB` = 34000, `_TileTotYB` = 20000, `_TileMassXB/YB` = (34, 20) (totais grandes para que as escavações feitas pelos ovos quase não mudem o alvo) e um túnel livre (valores 0–7) de (34, 10) a (34, 20) | a cabeça desce a coluna 34, uma célula por tentativa (1/64 por tick), sempre seguida da cauda; nunca sobe; `_LastBlackEgg` acompanha | D (trajeto), E (ritmo) |
| C-09 | rainha anda para trás (5.3) | reprodução do caso real do QUICK2 a partir do mundo novo | ver a verificação em 5.3: cabeça e cauda trocam de lugar quando a única vizinha que aproxima é a cauda | D |
| C-10 | cauda órfã (2, 5.2) | QUICK2 com a cabeça preta removida (T = 0) | no primeiro tick a cauda some e `_BlkQueens` = 0; no próximo `_CountAnts`, derrota no Quick Game | D |
| C-11 | ama preta guarda o estágio (4) | operária T 0x12, M 1, S 0, sobre um ovo T = 5 (ovo no índice menor, para ficar em `_LifeB`) | ela vira T 0x0A com S = 5; ao largar, o ovo novo tem T = 5 | D |
| C-12 | ama vermelha reinicia (4) | operária vermelha T 0x92, M 1, sobre ovo T = 0x86 | ao largar, o ovo tem T = 0x82 | D |

### Observado no original

Bancada QEMU, `tools/caso_ant.py rodar tests/referencias/casos/ID.json` (casos versionados; injeção, load e ritmo em [../../bancada-qemu.md](../../bancada-qemu.md#casos-de-teste-injetar-um-save-e-rodar)). Estados finais em `tests/referencias/saves/`.

| Id | Data | Entrada efetiva | Observado | Resultado |
|---|---|---|---|---|
| C-01 | 2026-10-16 | `QUICK2.ANT` sem mudanças; parado no tick 128 | ovos pretos que existiam (B[2..6]) com T = 2; B[7], posto no próprio tick 128 em (36, 9), com T = 1; vermelhos R[2..8] com 0x82 e R[9] (posto no 128) com 0x81 (`C01_T128.ANT`) | **confirmado**: avanço simultâneo no tick 128 com P = 128; o ovo posto no tick não avança nele |
| C-02 | 2026-10-16 | `QUICK2.ANT` sem a operária preta da superfície (A[0], T = 17), para que o `_BpopT` recontado no load seja 2; ovo B[2] com T = 7; parado no tick 96 | `_BpopT` = 2; B[2] em (34, 9) com T = 0x32, M = 2, S = 0; B[3..5] com T = 2 (incluindo o posto no tick 80); B[6] (posto no tick 96) com T = 1; ovos vermelhos com 0x81 (`C02_T96.ANT`) | **confirmado**: P = 32 com `_BpopT` < 3, eclosão no tick 96 como `_MakeMe` × 8 + 2 (soldado virado para a direita), modo do soldado em {1, 2}; o vermelho mantém P = 128 com `_RpopT` = 2 |
| C-11 | 2026-10-17 | `QUICK2.ANT`; ovo B[3] movido para (32, 3) com T = 5 e operária T 0x12, M 1, S 0 no fim da lista na mesma célula; parado no tick 85 | B[3].T = 0; B[5] em (34, 9) com T = 0x0B, S = 5, M = 1: pegou o ovo no tick 79 (S = estágio) e desceu (31, 4), (30, 5), (31, 6), (32, 7), (33, 8), (34, 9) pelo mapa de saída (`C11_T85.ANT`) | **confirmado** (§4: a ama preta guarda o estágio em S; trajeto da spec de ninho §5.1) |
| C-10 | 2026-10-17 | `QUICK2.ANT` com a cabeça da rainha preta (B[0]) zerada | logo depois do load aparece a mensagem "The black queen in this patch has died." (10010), modal, e a simulação não anda (`GameTime` fica em 78); o estado lido ainda tem `_CastePopB[5]` = 1 e `_IsGameOver` = 0 | **não conferível** com `caso_ant.py` (exige clicar na mensagem). Mostra que o load já detecta a rainha que sumiu [em aberto: qual rotina do load mostra a mensagem] |

Observações das execuções:

- O load **reconta** `_CastePopB/R` e, com eles, `_BpopT/_RpopT` (no `QUICK2.ANT`, `_CastePopB[0]` 2 → 3: o ovo posto depois da última contagem). Por isso a entrada do C-02 muda a lista A em vez de escrever `_BpopT`. O mesmo vale para C-03 (`_RpopT` = 1 exige tirar a operária vermelha da lista A).
- Numa execução do C-02 que parou no tick 98, o soldado já estava em (32, 7) com T = 0x37 e M = 2: nos ticks 97 e 98 subiu duas células na diagonal para a saída (direção 7).

## Lacunas

- Testados na bancada: C-01, C-02 e C-11 (acima); C-10 esbarra na mensagem modal do load. No port, C-11 é conferido em `engine/tests/regras_ninho.rs`, e os vetores de tick 79–83 (com a postura do tick 80) batem com o ovo, a rainha e a cauda do port (ver [ninho.md](ninho.md#133-observado-no-original)). C-04 a C-07 viraram vetores de tick (2026-10-18, `tools/cenarios_regras.py`, snapshot `rico`, 307/307 com S, R e vida; [testes-tick.md](testes-tick.md#quinta-rodada-2026-10-18-regras-não-verificadas-da-revisão)): `ovos_c04_c05/1791` (com `_ModeAuto` = 0 e `_modeLevels[2]` = 0 os 13 ovos pretos T = 7 morrem na eclosão, `_TotalEggsDiedB` 0 → 13; com `_HealthR` = 20 os 21 ovos vermelhos 0x87 nascem 17 operárias e 4 soldados, nenhum macho ou rainha), `ovos_c06/1711` (postura com `_HealthB` 100: ovo novo, `_TotalEggsLaidB` +1, `_HealthB` −1) e `ovos_c07/1711` (`_HealthB` 0: nenhum ovo) — **confirmados**. Faltam C-08 em diante.
- O significado exato da lista 4/1100 por estágio (o índice usado na janela Examine) não foi conferido na interface.
- `_YellowBirth` e a postura da amarela-rainha: só resumidos (ficam com `formiga-amarela.md`).
- A fundação de colônias novas no Jogo Completo (`_SimColonies`, `_QueenStorage*`, revoada) não está aqui.
- Os efeitos marcados [hipótese] (amarela vermelha pondo ovo de tipo preto; recriação da cauda em lutas) precisam de teste.
