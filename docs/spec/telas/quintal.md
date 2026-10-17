# Janela do quintal (Yard View), troca de patch e Jogo Completo

Este documento descreve a janela do quintal (janela 25, aberta pelo código como `0x1900`), a faixa do quintal (janela 35, `ribbonYard`), a escolha e a troca de patch (`_XferPatch`), a colocação de rainhas (`_PlaceQueenInYard`) e o fluxo do Jogo Completo (início, objetivo e fim). As regras do quintal que rodam no tick (menino, bichos, colônias, vitória e derrota) estão em [../regras/quintal.md](../regras/quintal.md); a geração do patch novo, em [../regras/geracao-mundo.md](../regras/geracao-mundo.md).

Implementação: `engine/src/core/quintal_mapa.rs` (geometria, desenho e cliques da janela, sem SDL), `engine/src/rules/troca_patch.rs` (`_XferPatch` e `_PlaceQueenInYard` sobre o `Estado`), dados em `data/quintal_mapa.json` (gerado em parte por `tools/extrair_quintal_mapa.py`), executável em `simant/src/modos.rs`.

**Fontes e legenda:** **[código]** = pseudocódigo do Ghidra em `re_trabalho/decomp/` (só consulta), com o endereço `seletor:offset` e o nome do `SIMANTW.SYM`; **[dados]** = bytes do `SIMANTW.EXE` ou dos bancos; **[medido]** = medido pixel a pixel em captura da bancada QEMU (640×480, [../../bancada-qemu.md](../../bancada-qemu.md)); **[hipótese]**; **[em aberto]**.

Capturas de referência (bancada QEMU, 2026-10-17, roteiros em `re_trabalho/modos/`, fora do git): `full_inicio.png` (Full Game logo depois de escolhido), `full_yard.png` (botão Yard), `yard_strat.png`, `yard_graphic.png`, `yard_house.png`, e `yard_strat2.png`/`yard_graphic2.png` (com colônias escritas na memória: `_YMapPopB`/`_YMapPopR` por `escrever_variavel`).

## 1. Coordenadas do quintal

O quintal tem **12 × 16 patches**, `(px, py)` com `px` 0–11 e `py` 0–15; índice nos mapas `_YMapPopB/R` = `px × 16 + py` [código]. Os patches com índice ≤ 37 (`px` = 0 ou 1, ou `px` = 2 com `py` ≤ 5) são a **casa** (`_MakeMap`, geracao-mundo.md §3); os outros, o gramado. Cada patch é um paralelogramo: um passo em `px` anda 28 pixels para a direita; um passo em `py` anda 10 pixels para baixo e 10 para a esquerda [código: `_InvertPatch` `1010:be04`, `_DrawSimColonies` `1010:bea4`, `_YardArea` `1010:c09a`].

Com `(ox, oy)` = canto de `_mapTileRect` (`9:8110`), o vértice `k` do patch é

```
x = ox + 28·px − 10·py + polígono[k].x        y = oy + 10·py + polígono[k].y
```

com dois polígonos do DGROUP [dados]:

| Tabela | Endereço | Vértices (x, y) | Uso |
|---|---|---|---|
| `_patchRgn` | `1048:1894` | (169, 67), (192, 67), (185, 74), (162, 74) | cursor (`_InvertPatch`) e clique |
| `_patchRgn2` | `1048:18a4` | (169, 67), (193, 67), (186, 74), (162, 74) | cor das colônias (`_DrawSimColonies`) |

Nos modos 0 e 1 o cursor fica 1 pixel à esquerda (`−1` no x de `_InvertPatch`) [código].

**Clique → patch** (`_YardArea`, mesma conta em `_PlaceQueenInYard`): `py = (y − oy − 67) / 10` e `px = (10·py + x − ox − 169) / 28`, divisão inteira do C (trunca para zero); vale se 0 ≤ `px` < 12 e 0 ≤ `py` < 16 [código]. Por causa do truncamento, a linha logo acima do patch (0, 0) também cai em `py` = 0 [código; não testado no original].

`(ox, oy)` na bancada = canto da área cliente da janela 25: o cursor do patch (11, 8) aparece com o canto superior esquerdo em (517, 304) na tela, que dá `(ox, oy)` = (120, 157) = canto da área cliente [medido em `yard_strat.png`].

## 2. A janela 25

### 2.1 Layout e moldura

Janela 25 do `WINGANT` (sem nomes no `SHARED`; os nomes abaixo são do port): objeto 0 contorno de espessura 2; 22 objetos [dados]. Na bancada [medido em `full_yard.png`]:

| Elemento | Na tela (x, y, largura, altura) |
|---|---|
| contorno externo (1 pixel preto) | (119, 137, 513, 277) |
| barra de título (azul, índice 4), com a caixa do menu de sistema em (120, 138, 18, 18) e linha preta em x = 138 | (120, 138, 511, 18), linha preta em y = 156 |
| área cliente | (120, 157, 511, 256) |

É uma janela do Windows com título, sem moldura grossa nem barras de rolagem. O título é o item `9 + _YardMode` da lista de textos 4/1000 do `SHARED`: "Yard View", "House View", "Strategic View", "Graphic View" (`_SetMapTitle` `1000:953c`) [código + dados]. O port, como na janela de edição, não desenha o texto do título.

### 2.2 Conteúdo por modo (`_YardMode`, `_SetYardMode` `1000:9f94`, `_win_DrawYardWindow` `1010:a9e0`)

`_SetYardMode(m)` monta um buffer de 512 × 225 pixels e a janela o copia na área cliente [código]:

| Modo | Título | Buffer | Onde o buffer cai na área cliente |
|---|---|---|---|
| 0 | Yard View | bitmap 7000 (279 × 225, a casa por fora) em (0, 0) e 7002 (233 × 225, gramado e cerca) em (278, 0) | (0, 2) |
| 1 | House View | bitmap 7001 (a casa em corte) em (0, 0) e 7002 em (278, 0) | (0, 2) |
| 2 | Strategic View | bitmap 7020 (514 × 161, a grade dos patches) | (0, 66), abaixo de um preenchimento do objeto 20 (0, 0, 512, 66) |
| 3 | Graphic View | igual ao modo 2 | (0, 66) |

Posições do buffer [medido: 7000 em (120, 159), 7002 em (398, 159), 7020 em (120, 223) na tela]. Embaixo, em todos os modos, o bitmap 7010 (512 × 29) em (−1, 227) da área cliente [medido: (119, 384)], com a área de mensagem (`_DrawAnimYardMessage`, texto do rodapé, fonte 2) [código; o texto não é desenhado pelo port]. O resto da área cliente é cinza escuro (índice 7) [medido].

Os bitmaps 7000, 7001, 7002, 7010 e 7020 estão no banco `WINGANT` (e versões mono no `MWINNT`) [dados].

### 2.3 Modos 0 e 1: bichos (`_Draw_SimYard` `1010:b462`)

Sprites com fundo transparente (índice 13, magenta, em todos os bitmaps abaixo) [dados; hipótese de que 13 é a cor transparente do `_hanim`], posicionados em pixels relativos a `(ox, oy)`:

| Bicho | Quando | Bitmap | Posição |
|---|---|---|---|
| gato | `_CatOn ≠ 0` | 1300 + `_CatFrame` | (`_CatX`, `_CatY`) + desl. por quadro (quadros ≥ 10) |
| cachorro | `_ForSaleState = 0` | 8500 + `_DogFrame` | (`_DogX`, `_DogY`) + desl. por quadro |
| menino | `_BoyFrame` ≥ 0 e não (modo 0 e `_BoyHere = 0`) | 8000 + `_BoyFrame` | (`_BoyX`, `_BoyY`) + desl. por quadro (quadros < 200) |
| cortador (`_DrawMower`) | parado: sempre que o menino não está com ele | 8800 | (98, 181) |
| cortador | `_BoyHere` 3 ou 4 e `_BoyFrame` 103–105 / 109–111 | 8801 / 8802 | (`_BoyX` + 19, `_BoyY` + 15) / (`_BoyX` − 15, `_BoyY` + 15) |
| pássaro | `_BirdOn ≠ 0` | 1250 + `_BirdFrame` | (`_BirdX` − 10, `_BirdY` − 3) no quadro 0, (`_BirdX` − 9, `_BirdY` − 1) no 1 |
| placa "vende-se" | `_ForSaleState ≠ 0` | 1260 | (170, 186) |
| chuva | `_RainOn ≠ 0` | 7005, 14 gotas | (`S(400)` + 50, `S(150)`) |
| enxame | `_SwarmCntB`/`R` > 0, quando `_YardMode = 0`… | 7006 / 7007 | perto do patch atual (`_DrawSwarm`) |

Com o cortador nas mãos e o menino virado para cima ou para baixo (quadros 106–108 e 100–102), o cortador não é desenhado [código]. Os deslocamentos por quadro vêm de tabelas de bytes com sinal do DGROUP (`1048:18d4`…`1048:1953`, extraídas para `data/quintal_mapa.json`) [dados]. A ordem de desenho do `_hanim` (prioridades `0xffff`, 999, 1000, `0x8000`) não foi estudada [em aberto]; o port desenha gato, cachorro, menino, cortador, pássaro, placa, chuva.

**Sorteios no desenho:** a chuva e o enxame usam `_SRand1`/`_SRand16` — o **gerador S da simulação** — cada vez que a janela é redesenhada [código]. Com a janela do quintal aberta e chovendo, o original consome sorteios de S fora do tick. O port não reproduz isso: desenha a chuva com posições fixas por gota e não desenha o enxame (ver [../../divergencias.md](../../divergencias.md)).

### 2.4 Modo 2: colônias (`_DrawSimColonies` `1010:bea4`)

Para cada patch, a cor depende de haver pretas (`_YMapPopB` ≠ 0) e vermelhas (`_YMapPopR` ≠ 0) [código]:

| Pretas | Vermelhas | Categoria | Cores (`1048:1982` / `1048:198a`, antes de `_ConvColor`) | Na tela [medido em `yard_strat2.png`] |
|---|---|---|---|---|
| sim | não | 0 | 0x0F / 0x0F | preto (índice 0) |
| sim | sim | 1 | 0x1F / 0x13 | cinza claro (índice 8): `_TrapFill` com duas cores diferentes usa a cor 8 |
| não | sim | 2 | 0x23 / 0x23 | vermelho (índice 9) |
| não | não | 3 | 0x48 / 0x48 | verde da grade (índice 2) |

O polígono é `_patchRgn2` (`_TrapFill`, `1008:528c`, `Polygon` do GDI com caneta e pincel da mesma cor) [código]. A tabela de `_ConvColor` (`1048:7238`) é preenchida em tempo de execução; as cores da última coluna são as observadas [medido].

### 2.5 Modo 3: barras (`_DrawColonyBars` `1010:bf9c`)

Para cada patch com população *p* ≠ 0, uma barra de altura `(p + 3) >> 2` pixels, com a base em `oy + 10·py + 71` e a esquerda em `ox + 28·px − 10·py + 168` (pretas, 9 de largura, cor `_ConvColor(15)` = preto) ou `+ 180` (vermelhas, 9 de largura, `_ConvColor(3)` = vermelho) [código; cores medidas em `yard_graphic2.png`]. O preenchimento é `_GRectFill` [hipótese: retângulo semiaberto, direita e base exclusivas].

### 2.6 Cursor do patch atual (`_DrawYardCursor`/`_InvertPatch`)

O patch atual (`_CurYardPnt`, `9:7a60`) é marcado com `_patchRgn` [código]. Na tela [medido]: nos modos 0 e 1, só o contorno do polígono em magenta (índice 13); nos modos 2 e 3, o polígono preenchido de cinza claro (índice 8) com contorno cinza escuro (índice 7). O original obtém isso invertendo os pixels (`_TrapFill` sem cores, modo de inversão) [hipótese]; o port desenha as cores medidas (`data/quintal_mapa.json`, `cursor`).

## 3. Faixa do quintal (janela 35, `ribbonYard`)

Quando a janela do quintal está aberta no lugar do mapa, a faixa mostrada é a 35 em vez da 34 (`_MapToYard` `1000:9918` troca `0x2200` ↔ `0x2300`, `_YardToMap` faz o contrário) [código]. Botões (`_ProcYardRibbonEvent` `1010:a6ce`) [código]:

| Id | Objeto | Efeito |
|---|---|---|
| `0x2302` | `ribPatch` | `_YardToMap`: volta ao mapa (janela 1) e à faixa 34 |
| `0x2303` | `ribHouse` | modo 0 ↔ 1 (`_YardMode ^ 1`); de 2 ou 3 vai para 0 |
| `0x2304` | `ribStrat` | modo 2 |
| `0x2305` | `ribGraphic` | modo 3 |
| `0x2306` | `ribMate` | só no Jogo Completo e com pelo menos uma rainha alada fora a amarela: `0x7c44 = 200` (revoada) e mensagem; senão som de erro e mensagem |
| `0x2307` | `ribPlace` | `_PlaceQueenInYard` (seção 5) |
| `0x2309` / `0x230a` | `ribAuto` / `ribManual` | `_LayDownQueenMode` = 1 / 0 |
| `0x230b`–`0x2312` | `ribUserDef2A`–`H` | `_DoUserButton` |
| `0x2313`–`0x2319` | `ribbm1`–`7` | marcadores (`_DoBookMark`) |

Dados da faixa (`_DrawYardData` `1010:acd8`, fonte 3): `ribQueens` = `_QueenStorageB`, `ribBlackCols` = `_ColonyTotalBlack`, `ribRedCols` = `_ColonyTotalRed`, e barras de saúde e população [código]. O port desenha a faixa 35 com o botão do modo selecionado (`_SetYardMode` seleciona `0x2302 + modo`… pela tabela `1048:1586`) [em aberto: conteúdo da tabela; o port seleciona `ribHouse` nos modos 0 e 1, `ribStrat` no 2 e `ribGraphic` no 3, como na captura] e ainda não desenha os números nem as barras.

## 4. Abrir a janela e escolher o modo

- Botão `Yard` da faixa 34 (`0x2202`) e Window → Map/Yard (`Ctrl+M`, comando `0xFD12`, `_OpenMapYard` `1000:9b2a`): abre a janela do quintal no `_YardMode` atual, ou a traz para a frente [código; o port não tem a janela do mapa, então o botão Yard sempre abre o quintal].
- Aceleradores (tabela do `SIMANTW.EXE`) [dados + código, `_ProcMenu` casos 0x21–0x24]: **Ctrl+Y** = Yard View (modo 0), **Ctrl+5** = House View (1), **Ctrl+6** = Strategic View (2), **Ctrl+7** = Graphic View (3). Cada um: `_SetYardMode(m)` se mudou, `_SetMapPlane(0)` se `_MapPlane ≠ 0`, abre a janela 25 e a traz ao topo.
- `_SetMapPlane(0)` guarda o plano em `_OldMapPlane` e grava `_YMapPnt = _CurYardPnt` [código, jogo.md]. **O port não muda `_MapPlane`** ao abrir o quintal (a janela de edição continua no plano em que estava) [divergência registrada].

## 5. Escolher patch, trocar de patch e colocar rainhas

### 5.1 Clique (`_YardArea`)

Clique esquerdo na área do quintal (objeto `0x1902`): calcula o patch (seção 1) e, se está no quintal, grava `_YMapPnt` = (px, py) (o patch escolhido). Se o clique é **duplo** (bits 0x60 do evento) chama `_XferPatch` [código].

### 5.2 `_XferPatch` (`1000:ab90`)

Com `t = (tx, ty) = _YMapPnt` e `i = tx × 16 + ty` [código]:

1. Se `t = _CurYardPnt`: nada.
2. Se `_CurGameType ≠ 2` (não é Jogo Completo): som de erro e diálogo com o texto **10000**.
3. Senão, se `_ColonyTotalBlack < 2` **e** `_BpopT < 2`: som de erro e texto **10002**.
4. Senão, se `_YMapPopB[i] = 0` e (`_MeType ≠ 0x40` ou `_MeNestStarted ≠ 0`): som de erro e texto **10004** (não há colônia preta ali e a amarela não é uma rainha alada sem ninho).
5. Senão, a troca:
   - música `0x2AFA`;
   - `_CurYardPnt = t`; `_JustXfered = 1`; `_QueenStorageB = _QueenStorageR = 0`; `_SwarmCntB = _SwarmCntR = 0`; `_MeNestStarted = 1`;
   - `_RandWorld(_YMapSeeds[ty × 12 + tx], _YMapPopB[i], _YMapPopR[i], tx, ty)` (geracao-mundo.md §3: o patch novo é gerado da semente do patch, com tantas formigas pretas e vermelhas quantas a população do quintal ali — ×1,25 — e ninhos escavados quando > 1);
   - se `_YMapPopR[i] = 0`: `_HealthR = 0`;
   - `_ClearBookmarks`;
   - diálogo (sem erro) com o texto **10007** se `i < 0x25` (casa) ou **10006** (quintal).

Consequências [código]: o que é do quintal inteiro fica (`_YMapPop*`, `_YMapSeeds`, menino, bichos, chuva do quintal, contadores de colônias, `_GameTime`, controles, histórico); o que é do patch é refeito por `_RandWorld` (mapas, listas, vida, feromônios, buracos, comida, saúde das colônias = 100, aranha, formigas-leão, bichos-de-conta, amarela). O estado de S antes da troca não importa (a semente é redefinida); R não é usado.

**No port** (`rules/troca_patch.rs`, `trocar_patch`): segue os passos acima e aplica o `Mundo` gerado ao `Estado` (`aplicar_patch`), trocando só os campos que `_RandWorld` escreve. Os pedidos de som de erro não geram evento (o port ainda não tem som de efeito pela fila); músicas e textos saem como `Evento::Musica` e `Evento::Mensagem`. A janela de edição troca o conjunto de tiles quando `_TERRAINset` muda (quintal ↔ casa).

### 5.3 `_PlaceQueenInYard` (`1000:ad12`)

Botão `PLACE` (`0x2307`) [código]:

1. Se `_QueenStorageB < 1`: som de erro e mensagem no rodapé; fim.
2. `_LayDownQueenMode = 0` (se era 1); os patches válidos são os a até 3 de distância em x e em y do patch atual (limitados ao quintal).
3. Para a simulação; o jogador move o mouse (o patch sob o mouse é invertido se for válido e diferente do atual) e clica na área do quintal.
4. Fora da faixa válida: mensagem e som de erro. Dentro: música `0x2AFB`, `_QueenStorageB −= 1`, `_YMapPopB[patch] += 1`, `_BColoniesStarted += 1`, mensagem no rodapé. Com Shift apertado, repete para outra rainha enquanto houver; senão termina.
5. Retoma a simulação.

Observação: o patch atual também é aceito (não há teste de diferente do atual na confirmação) [código].

**No port**: o botão PLACE liga o modo "colocar rainha"; o próximo clique simples na área do quintal chama `colocar_rainha` (`rules/troca_patch.rs`), com o mesmo teste e as mesmas mudanças. A simulação não para durante a escolha e Shift não repete [divergência registrada].

## 6. Jogo Completo: início, objetivo e fim

Resumo do que o código faz (detalhes nas specs de regras) [código]:

- **Início** (`_NewGame` tipo 2, geracao-mundo.md §1–2): patch (11, 8), sem colônia preta, a amarela é uma **rainha alada** (T = 0x40) na superfície, `_MeNestStarted = 0`, `_MapPlane = 1` (superfície). A faixa mostra "You need to dig a nest" [medido em `full_inicio.png`]. A colônia vermelha começa no mesmo patch.
- **Meio**: a rainha cava o ninho (`_MeNestStarted` = 1); a colônia cresce; rainhas aladas decolam (`ribMate`) e vão para `_QueenStorageB`; o jogador as põe em outros patches (`PLACE`, seção 5.3); a cada 32 ticks `_SimColonies` faz as colônias do quintal crescerem e brigarem (quintal.md §10). O jogador pode **mudar de patch** com duplo clique num patch com colônia preta (seção 5.2).
- **Objetivo / vitória**: quando há mais de 24 colônias (pretas ou vermelhas) na área da casa (`_HouseColonies > 24`), a casa é posta à venda (`_ForSaleState = 1`, mensagem 10054); se depois disso não houver nenhuma colônia vermelha, fim de jogo com vitória (`_IsGameOver = 1`, `_BlackWon = 1`, mensagem 10057).
- **Derrota**: menos de duas colônias pretas no quintal e nenhuma rainha, ovo, larva nem pupa preta (`_CastePopB[5,0,3,4] = 0`): `_IsGameOver = 1`, `_BlackWon = 0`, mensagem 10056.
- O diálogo de fim de jogo (`_EndGameDialog`, janela 4 `gameOverWin`) é pedido pela fila de eventos (`Evento::DialogoFimDeJogo`); o desenho dele é de outro trabalho (mensagens/diálogos).

## 7. O que o port faz

- Tela de jogo com a janela do quintal (`JanelaQuintal`): aberta pelo botão `Yard`, Ctrl+M, Ctrl+Y/5/6/7; fechada pelo botão `Patch` da faixa 35 ou Esc com ela na frente [Esc: extensão do port]. Desenhada por cima da janela de edição quando está na frente; atrás dela, só a parte fora da janela de edição aparece. Clique na janela de edição a traz para a frente; clique no quintal traz o quintal.
- Faixa 35 no lugar da 34 enquanto o quintal está aberto; botões House/Strat/Graphic/Patch/Place/Auto/Manual.
- Clique simples num patch: `_YMapPnt`; duplo: `trocar_patch`.
- Teclas e botões em `data/quintal_mapa.json` (sobrescrevível por mod).

## 8. Verificação

Ver a seção "Comparação com o original" no fim deste documento (preenchida com os percentuais de `tools/comparar_captura.py`).
