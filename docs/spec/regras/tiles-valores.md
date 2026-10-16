# Valores das células dos mapas (`_MapA`, `_MapB`, `_MapR`)

Cada célula de mapa é um byte. O mesmo byte serve de **índice do tile** desenhado e de **tipo de terreno** para as regras. Este documento lista o que cada faixa de valores significa para as regras, a partir das funções de teste do código e da geração do mundo ([geracao-mundo.md](geracao-mundo.md)).

Mapas (ver também [../formatos/save-ant.md](../formatos/save-ant.md)):

| Mapa | Endereço | Tamanho | Índice | Plano (`_MePlane`/`_MapPlane`) |
|---|---|---|---|---|
| `_MapA` | `1048:28e8` | 128 × 64 | `x * 64 + y` | 0 (quintal) e 1 (superfície do patch) |
| `_MapB` | `1048:48e8` | 64 × 64 | `x * 64 + y`, `y = 0` = linha da superfície | 2 (ninho preto) |
| `_MapR` | `1048:58e8` | 64 × 64 | idem | 3 (ninho vermelho) |

Nas funções de teste, o plano `< 2` usa `_MapA`, 2 usa `_MapB` e 3 usa `_MapR` (fato visto no código, por exemplo `_IsClearTile` @ `1020:5b2c`, `_IsNotObstacle` @ `1020:94c6`). A superfície tem dois conjuntos de tiles, escolhidos por `_TERRAINset` (`1040:9b6e`): **0 = quintal** (objeto tipo 9 nº 10, `_Barrier = 0x50`) e **1 = casa** (objeto tipo 9 nº 9, `_Barrier = 0x90`), trocados por `_OverlayTileSet` (`1010:00f4`). Várias faixas mudam de sentido com `_TERRAINset`.

Sobre o desenho: `_DoCalcTile` desenha o valor da superfície como índice de tile e, nos ninhos, `valor − 0x70` (byte, com estouro) (registrado em save-ant.md). Por isso não existem valores ≥ 0x70 guardados nos ninhos: os valores do ninho vão de 0 a 0x5D, e é o tile desenhado que cai em 0x90 e acima.

## Superfície, quintal (`_TERRAINset = 0`)

| Valor | Significado | Fonte |
|---|---|---|
| 0x00–0x0F | chão (16 variações de terra/grama, sorteadas com `Sk(16)` na geração) | fato visto no código (`_MakeYardPatch`); `_IsClearTile` considera "livre" só `< 0x10` |
| 0x18–0x1F | borda do buraco do ninho, por direção: 0x18 canto sup. esq., 0x19 cima, 0x1A canto sup. dir., 0x1B esquerda, 0x1C direita, 0x1D canto inf. esq., 0x1E baixo, 0x1F canto inf. dir. | fato visto no código: tabela `DGROUP:0x230c` indexada pela direção em `_MakeNewHoleB/R`; orientação deduzida de `_Dx8/_Dy8` |
| 0x20–0x23 | objeto 2 × 2: 0x20 sup. esq., 0x21 sup. dir., 0x22 inf. esq., 0x23 inf. dir. | fato visto no código (`_MakeYardPatch`); o que é [hipótese: tufo de grama/planta, a confirmar no tile] |
| 0x24/0x25, 0x26/0x27 | objetos 2 × 1 (esquerda/direita), duas variantes | fato visto no código; o que é [hipótese: a confirmar no tile] |
| 0x28 | objeto de uma célula, 100 tentativas por patch | fato visto no código; o que é [hipótese] |
| 0x30–0x37 | borda da armadilha da formiga-leão, por direção (0x30 sup. esq., 0x31 cima, 0x32 sup. dir., 0x33 esq., 0x34 dir., 0x35 inf. esq., 0x36 baixo, 0x37 inf. dir.) | fato visto no código (`_InitAntLions`, tabela `DGROUP:0x25e8`) |
| 0x38 | centro da armadilha da formiga-leão | fato visto no código |
| 0x48–0x4B | comida, 1 a 4 unidades (`_AddFood` cria 0x48 e soma 1 até 0x4B) | fato visto no código (`_IsItFood` @ `1028:2d1a`, `_AddFood`) |
| 0x50 | buraco (entrada de ninho) | fato visto no código (`_IsItHole` @ `1028:2cc0`: no quintal, buraco é exatamente 0x50) |
| 0x51–0x53 | pedrinha que se pode carregar (a geração só cria 0x51) | fato visto no código (`_IsThisPebble` @ `1020:5f32`, só no plano 1; `_IsLiftable` @ `1020:97ca`) |
| 0x54–0x5F | pedra grande (formas 5 × 5 e 3 × 3): 0x5B canto sup. esq., 0x54 borda de cima, 0x55 canto sup. dir., 0x56 borda dir., 0x57 canto inf. dir., 0x58 borda de baixo, 0x59 canto inf. esq., 0x5A borda esq., 0x5C miolo, 0x5D–0x5F cantos internos | fato visto no código (tabelas de `_AddRock5/3`); nomes das peças deduzidos da posição nas formas |
| 0x60–0x67 | parede (ferramenta do modo Experimental) | fato visto no código (`_IsItWall` @ `1010:86fe`); uso [hipótese] |
| 0x69–0x6B | lagarta (segmentos) | [hipótese]: aparecem no `QUICK2.ANT` em linha (x 3–7, y 58), não são criados pela geração |
| 0x70–0x73 | bicho-de-conta, um valor por direção (`_SowTab` = `72 73 71 70 72 73 71 70`) | fato visto no código (`_InitPillar`) |
| 0x74–0x77 | poça de chuva (`_InitWater` cria 0x74; `_DoWater` faz crescer) | fato visto no código |

Faixas sem uso visto na geração do quintal: 0x10–0x17, 0x29–0x2F, 0x39–0x47, 0x4C–0x4F, 0x68, 0x6C–0x6F, ≥ 0x78 [em aberto].

## Superfície, casa (`_TERRAINset = 1`)

Só o que as funções de teste dizem (o conteúdo de cada patch está em aberto, ver [geracao-mundo.md](geracao-mundo.md) §5):

| Valor | Significado | Fonte |
|---|---|---|
| 0x00–0x0F | chão livre (`_IsClearTile`) | fato visto no código |
| 0x18–0x27 | comida (`_IsItFood` na casa); `_AddFood` converte chão em comida e soma 1 enquanto `valor & 3 < 3` | fato visto no código |
| 0x3E/0x3F | fiapos no carpete (`_CarpetFloorL/R`, `_MakeLint2`) | fato visto no código; nome [hipótese] |
| 0x5C–0x5F, 0x60/0x61 | ladrilhos (`_FloorTiles` grava 0x5C–0x5F nas juntas e 0x60/0x61 em faixas) | fato visto no código |
| 0x64, 0x7A, 0x7B, 0x03 | preenchimentos do carpete esquerdo (`_CarpetFloorL`) | fato visto no código; nomes [hipótese] |
| 0x80–0x8F | buraco na casa (`_IsItHole`); `_CanBeHouseHole` cria 0x84, 0x85, 0x86, 0x8A e 0x80–0x83 | fato visto no código |

## Ninhos (`_MapB`, `_MapR`)

| Valor | Significado | Fonte |
|---|---|---|
| 0x00–0x07 | túnel (chão escavado, 8 variações `Sk(8)`) | fato visto no código (`_DigTileB/R`, `_SmoothEdges*`) |
| 0x08–0x0F | [em aberto] (passáveis: `< 0x19`) | |
| 0x10–0x13 | comida guardada no ninho | fato visto no código (`_IsItNFood` @ `1020:5f64`, `_IsThisFood`, `_IsItFoodAt`) |
| 0x14 | criado por `_DigTileB` nos dois ninhos ao mesmo tempo, abaixo de y = 53 | fato visto no código; [hipótese: passagem entre os ninhos] |
| 0x18 | abertura para a superfície na linha y = 0 | fato visto no código (`_IsItAHole` @ `1020:9b4a`, `_SmoothEdges*`, `_GetExitDirB`) |
| 0x1C–0x1F | linha da superfície ainda fechada (grama vista de lado) | fato visto no código (`_IsThisGrass` @ `1020:5ee4`; geração em `_RandWorld`) |
| 0x20–0x2E | terra; os 4 bits baixos de `valor − 0x1F` dizem quais vizinhas também são terra (bit 1 = cima, 2 = direita, 4 = baixo, 8 = esquerda; borda do mapa conta como terra). 0x2E = cercada (terra maciça, o valor inicial) | fato visto no código (`_SmoothEdgesB` @ `1020:255a`, `_IsItDirt` @ `1020:1182`) |
| 0x30–0x31 | pedrinha dentro do ninho (carregável, passável) | fato visto no código (`_IsThisPebble`, `_TileCanBeMovedOn`) |
| 0x4E | túnel inundado | fato visto no código (`_AddWater` @ `1020:0b8a`: túnel `< 0x20` vira 0x4E) |
| 0x4F–0x5D | terra inundada (`terra + 0x2F`) | fato visto no código (`_AddWater`, `_RIsItDirt` @ `1020:26c4` aceita 0x20–0x2F e ≥ 0x4F) |

Regra de alisamento (`_SmoothEdgesB/R`, fato visto no código): ao escavar (x, y), o jogo reavalia as quatro vizinhas. Para cada uma que seja terra (≥ 0x20, fora de 0x30–0x4E): calcula a máscara de vizinhas sólidas (sólida = ≥ 0x20 e fora de 0x30–0x4E); se a máscara não é zero, grava `0x1F + máscara` (+ 0x2F se a célula estava inundada); se é zero (terra isolada), a célula vira túnel `Sk(8)` (ou 0x4E se inundada). Na linha y = 0, uma célula ≤ 0x2F vira 0x18 (abertura). Fato verificado indiretamente: a reimplementação reproduz `_MapB`/`_MapR` do `QUICK2.ANT` byte a byte.

## Funções de teste (resumo)

Fato visto no código. "Superfície" = plano < 2.

| Função | Endereço | Regra |
|---|---|---|
| `_IsItDirt(v)` | `1020:1182` | 0x20 ≤ v ≤ 0x2E |
| `_RIsItDirt(v)` | `1020:26c4` | v ≥ 0x20 e (v < 0x30 ou v > 0x4E) |
| `_IsThisFood(p, v)` | `1020:5f04` | superfície: `_IsItFood(v)`; ninho: 0x10 ≤ v ≤ 0x13 |
| `_IsItFood(v)` | `1028:2d1a` | quintal: 0x48–0x4B; casa: 0x18–0x27 |
| `_IsThisPebble(p, v)` | `1020:5f32` | plano 1: 0x51–0x53; plano 0: nunca; ninho: 0x30–0x31 |
| `_IsThisGrass(p, v)` | `1020:5ee4` | ninho e 0x1C ≤ v ≤ 0x1F |
| `_IsThisEgg(T)` | `1020:5ec8` | (T & 0x7F) de 1 a 7 (testa o tipo da formiga, não o mapa) |
| `_IsItDigable(p, x, y)` | `1020:95c6` | só ninho, dentro do mapa, e v terra (0x20–0x2E) ou superfície fechada (0x1C–0x1F) |
| `_IsNotBarrier(v)` | `1020:94a0` | quintal: v ≤ 0x50; casa: v ≤ 0x5F |
| `_IsNotObstacle(p, x, y)` | `1020:94c6` | superfície: dentro do mapa e v ≤ 0x53 (quintal) ou ≤ 0x5F (casa); ninho: v < 0x19 ou 0x30–0x31 |
| `_IsLessThanHole(v)` | `1020:9784` | quintal: v ≤ 0x4F; casa: v ≤ 0x58 |
| `_IsItHole(x, y)` | `1028:2cc0` | quintal: v = 0x50; casa: 0x80 ≤ v ≤ 0x8F |
| `_IsItAHole(p, x, y)` | `1020:9b4a` | superfície: `_IsItHole`; ninho: só em y = 0 e v = 0x18 |
| `_IsItWall(v)` | `1010:86fe` | 0x60 ≤ v ≤ 0x67 |
| `_IsClearTile(p, x, y)` | `1020:5b2c` | dentro do mapa; sem formiga na célula de `_Life?` (0 ou 0xFF/0xFE, a formiga amarela, contam como livre); e v < 0x10 (superfície) ou v < 8 (ninho) |
| `_IsClear3x3(p, x, y)` | `1020:5ad2` | `_IsClearTile` na célula e nas 8 vizinhas |
| `_TileCanBeMovedOn(p, x, y, …, cavar)` | `1020:9342` | superfície: dentro do mapa e v ≤ 0x53 (quintal) ou ≤ 0x90 (casa). Ninho: v < 0x19 ou 0x30–0x31; se o último parâmetro for verdadeiro ([hipótese]: "pode cavar"), também 0x1C–0x1F e 0x20–0x2E. Nas linhas y ≤ 1 há regras extras sobre trocar de plano e sobre a célula de baixo [detalhe não especificado aqui] |

Observação: em `_IsNotObstacle` o segundo teste de `_TERRAINset = 0` (limite 0x50) nunca é alcançado; o limite efetivo no quintal é 0x53 (fato visto no código, provável erro do original).

## Valores do `_LifeA/B/R` gravados na geração

Não são valores de mapa, mas a geração escreve neles: o byte T da formiga (bit 7 = vermelha; bits 3–6 = classe; bits 0–2 = direção nos adultos), 0xFF/0xFE para a formiga amarela. A geração usa T = 0x10, 0x20, 0x30, 0x40 (+ direção) na superfície, 0x62/0x6A (rainha preta e sua segunda metade) e 0xE2/0xEA (vermelha). Nomes das classes: ver [../formatos/save-ant.md](../formatos/save-ant.md) [hipótese lá registrada].

## Como testar contra o original

1. **Contagem por valor nos saves:** o `QUICK2.ANT` tem, no `_MapA`, 0x00–0x0F (7.415 células), 0x18–0x1F (2 de cada: dois buracos), 0x20–0x23 (18 de cada), 0x24/0x25 (10), 0x26/0x27 (18), 0x28 (96), 0x30–0x38 (2 de cada: duas formigas-leão), 0x48–0x4B (49 células de comida), 0x50 (2), 0x51 (117), 0x54–0x5F (pedras), 0x69–0x6B, 0x71/0x72; nos ninhos, só 0x00–0x07, 0x18, 0x1C–0x1F e 0x20–0x2E. Cada save novo deve continuar dentro destas faixas; um valor fora delas é sinal de faixa não documentada.
2. **Identificação visual:** desenhar o `_MapA` do `QUICK2.ANT` com o conjunto de tiles do quintal (objeto tipo 9 nº 10; folha em `re_trabalho/png/tiles_9_10_fora.png`) e comparar com uma captura da janela de edição na bancada QEMU ([../../bancada-qemu.md](../../bancada-qemu.md)) centrada nos mesmos pontos. Isso confirma as [hipóteses] de nome (0x20–0x28, 0x69–0x6B) e a orientação x/y na tela.
3. **Regras de passagem:** na bancada, com a formiga amarela, tentar andar sobre pedrinha (0x51), pedra (0x54–0x5F) e buraco, e cavar em 0x1C–0x1F; comparar com `_TileCanBeMovedOn`/`_IsItDigable`.
