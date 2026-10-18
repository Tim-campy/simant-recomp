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
| gato | `_CatOn ≠ 0` (modos 0 e 1) | 1300 + `_CatFrame` (1300–1305 andando, 1310–1312 sentado, 1320–1329 fugindo) | (`_CatX`, `_CatY`) + desl. por quadro (quadros ≥ 10); na bancada os quadros 0–12 aparecem 1 pixel abaixo da conta [medido, causa em aberto; `ajuste_medido`] |
| cachorro | `_ForSaleState = 0` | 8500 + `_DogFrame` | (`_DogX`, `_DogY`) + desl. por quadro |
| menino | `_BoyFrame` ≥ 0 e não (modo 0 e `_BoyHere = 0`) | 8000 + `_BoyFrame`, **qualquer** quadro ≥ 0: inclusive 200 (poltrona, nó 32), 201 (geladeira, nó 34) e 900–903 (porta, nós 20/21) | (`_BoyX`, `_BoyY`) + desl. por quadro; sem deslocamento a partir de 200 (`_DrawSimKid` `1010:af14`) |
| cortador (`_DrawMower`) | parado: sempre que o menino não está com ele | 8800 | (98, 181) |
| cortador | `_BoyHere` 3 ou 4 e `_BoyFrame` 103–105 / 109–111 | 8801 / 8802 | (`_BoyX` + 19, `_BoyY` + 15) / (`_BoyX` − 15, `_BoyY` + 15) |
| pássaro | `_BirdOn ≠ 0` | 1250 + `_BirdFrame` | (`_BirdX` − 10, `_BirdY` − 3) no quadro 0, (`_BirdX` − 9, `_BirdY` − 1) no 1 |
| placa "vende-se" | `_ForSaleState ≠ 0` | 1260 | (170, 186) |
| chuva | `_RainOn ≠ 0` | 7005, 14 gotas | (`S(400)` + 50, `S(150)`) |
| enxame (`_DrawSwarm` `1010:bb0e`) | `_MapMode` = 0; de cada cor, as formigas 0 … min(`_SwarmCnt`, 16, `_QueenStorage`) − 1 | 7006 (preta) / 7007 (vermelha): pontinhos | (`_SwarmCentX` + `_SwarmX[i]`, `_SwarmCentY` + `_SwarmY[i]`), centro = (28·px − 10·py + 178, 10·py + 46) do patch atual (o mesmo alvo do pássaro) |

Com o cortador nas mãos e o menino virado para cima ou para baixo (quadros 106–108 e 100–102), o cortador não é desenhado [código]. Os deslocamentos por quadro vêm de tabelas de bytes com sinal do DGROUP (`1048:18d4`…`1048:1953`, extraídas para `data/quintal_mapa.json`) [dados]. A ordem de desenho do `_hanim` (prioridades `0xffff`, 999, 1000, `0x8000`) não foi estudada [em aberto]; o port desenha gato, cachorro, menino, cortador, enxame, pássaro, placa, chuva.

**Enxame na bancada (2026-10-18, `cenarios_tick.py gravar tela_revoada`, snapshot `rico`, `_SwarmCntB/R` e `_QueenStorageB/R` = 8/6):** na Yard View, pontinhos pretos e vermelhos em volta de (528, 283) da tela (o centro do patch (11, 8)) e o pássaro chegando para comer. O port desenha o mesmo estado (`revoada_quintal.ANT`) com os pontinhos na mesma posição (`DesenhoQuintal::desenhar_enxame`, `sprites.enxame` em `data/quintal_mapa.json`); antes de 2026-10-18 o enxame não era desenhado.

**Rodapé de mensagem (`_UpdateYardMessage` `1010:ac0e`, também chamado por `_EditMessage`):** com a janela do quintal aberta, o mesmo texto da faixa (`_mapMessage`, com o mesmo prazo) é escrito no objeto `yardMsg` da janela 25 (fonte 2, cores do objeto, 12); vencido, o retângulo é pintado com a cor 12 [código]. Como a faixa 35 do quintal não tem área de mensagem, é ali que o jogador vê os conselhos e as respostas dos botões com o quintal na frente. **Bancada:** "Your nest needs expansion" (conselho), "Fertilize queens in Full Game only" e "Produce males and new queens first" (botão MATE) aparecem no rodapé [capturas `tela_chuva`, `tela_mate`]. Port: `DesenhoQuintal::desenhar_mensagem` (`mensagem` em `data/quintal_mapa.json`), desde 2026-10-18; antes o port não mostrava a mensagem com o quintal na frente. [hipótese: também nos modos 2 e 3, como no port.]

**Bancada dos sprites (2026-10-18, `cenarios_tick.py gravar sprites_quintal`, snapshot `rico`, cópia do overlay em `re_trabalho/sprites/`):** os bichos são escritos na entrada de um tick (ou no retorno de `_DoAntSim`, para quadros que a simulação não mantém) e a tela é capturada parada na entrada seguinte, com o estado desenhado em `NOME.ANT`; o port desenha o mesmo estado com `simant --save NOME.ANT --quintal M --exportar-tela-jogo`. 31 capturas (`cap/` e `cap2/`): Yard View livre, gato andando (quadros 1–5), sentado (10–12) e fugindo (21, 25), cachorro andando e sentado (100–103), pássaro (0, 1), menino cortando com o cortador nas mãos (104, 110), cortador parado, House View com o menino na poltrona (200), na geladeira (201), na porta (900–903) e andando. Janela inteira **98,7–99,5%** (`comparar_captura.py --mapear`, região (119, 99)–(632, 376)); as diferenças são as gotas de chuva, o título, o rodapé e o cursor. Posição de cada sprite (melhor deslocamento da máscara contra a captura): todos em (0, 0), depois do ajuste de 1 pixel dos quadros 0–12 do gato. Antes da correção o menino sumia nos quadros 200, 201 e 900–903 (o port só desenhava quadros < 200).

**Por que o gato "não aparecia":** o desenho e a regra já existiam e batem com o original. O relógio do Mac da partida (`_MacTickCount`) começava em 0 a cada sessão, e o `_CatDelay` do save (2669–5000 nos saves da bancada, valores do relógio da sessão que salvou) segurava o primeiro teste por quase um minuto, além do minuto médio de espera do próprio sorteio (`S16` a cada 200 unidades). O port agora começa o relógio no maior prazo do save (ver regras/quintal.md, "Relógios do port").

**Sorteios no desenho:** a chuva e o enxame usam `_SRand1`/`_SRand16` — o **gerador S da simulação** — cada vez que a janela é redesenhada [código]. Com a janela do quintal aberta e chovendo, o original consome sorteios de S fora do tick. O port reproduz os sorteios e o que o desenho grava no estado (`rules/desenho.rs`, chamado pela partida depois de cada tick; regras/rng.md "Sorteios do desenho"), mas a imagem das gotas usa posições próprias (ver [../../divergencias.md](../../divergencias.md)); o enxame é desenhado nas posições do estado.

**Som do cortador (`_DrawMower`):** a cada redesenho do quintal nos modos 0 e 1, com `_BoyHere` 3 ou 4 e o jogo não pausado (`_GamePaused`, `9c28`), toca o efeito 0x24 [código]. Só com a janela do quintal aberta: na janela de edição o cortador é mudo (só as pisadas, 0x23, do `_FootFall`). Port: `desenho::desenhar_cortador`, pedido junto com os sorteios do desenho do quintal (`quintal.json`, `sons.cortador`).

**Balão do menino** (`_DrawSimKid` `1010:af14`, `_MakeBalloon` `1008:59e4`) [código]: quando o menino é desenhado, sem pausa, se o relógio do Mac passou de `_BoyMsgCnt` o balão acaba (`_BoyMessOn = 0`, estado que vai no save; `desenho::desenhar_menino`). Com `_BoyMessOn` ≠ 0, o texto é a entrada `_BoyMsgOffset` da lista de strings carregada de 1200 (`_LoadStringAnt(1200)` em `_initStuff`), escrito na fonte 2, com as linhas separadas pela sequência de dois caracteres barra-n. `_MakeBalloon` monta uma imagem de máscara monocromática com peças 8×8 do DGROUP (`1048:1c6e`…`1048:1d0e`: cantos, bordas, centro e as cunhas): largura = (maior linha + 23) arredondada para baixo a múltiplo de 8; uma fileira de peças de cima, fileiras de bordas e centro a cada 8 linhas até a linha de baixo em `altura_fonte × n + 6`, as peças de baixo e a cunha da esquerda na coluna 1; cada linha do texto vai em `altura_fonte × i + 6`, invertida (tinta preta) sobre o fundo branco. A altura registrada é `altura_fonte × n + 15` (uma a menos com uma linha). O objeto animado do balão fica em x = (x do sprite do menino) + 4 e y = `DGROUP:0x8112` − altura + `_BoyY`, na frente dos outros (profundidade 999). **Bancada (2026-10-18, `cenarios_tick.py capturas`, balão nº 2 "I'm getting hungry from all this walking.", Yard View, chovendo):** janela do quintal **98,15%**, região do balão **80,3%**; o balão (forma, texto, posição) é o mesmo, a diferença é das gotas de chuva, sorteadas a cada desenho.

Port: `DesenhoQuintal::desenhar_balao_do_menino`, com as peças, a lista, a fonte e o deslocamento em `data/quintal_mapa.json` (`balao_menino`). [O texto de cada linha começa em x0 + 8 + ((−largura) & 6) / 2, conferido nos balões da janela de edição (telas/baloes.md), mesma `_MakeBalloon`. Hipótese: `0x8112` tomado como a origem y do quintal, a mesma dos sprites; sem captura do original para conferir.]

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
| `0x2306` | `ribMate` | só no Jogo Completo (`_CurGameType` = 2) e com pelo menos um reprodutor preto (`_CastePopB[3] + [4]`, machos e rainhas novas, menos 1 se `_MeType` é 0x40 ou 0x20): `_FlyAwayB` (`9:7c44`) = 200 (revoada) e 1010[6] "You have started the mating cycle"; senão som 1 e 1010[5] "Produce males and new queens first"; fora do Jogo Completo som 1 e 1010[15] "Fertilize queens in Full Game only". `_EditMessage(…, 0xb4, 0, 1)` (importante) [código; bancada `tela_mate`: as duas recusas, com `_FlyAwayB` inalterado]. Port: `rules::quintal::botao_revoada` (`botao_revoada` em `data/quintal.json`), desde 2026-10-18 |
| `0x2307` | `ribPlace` | `_PlaceQueenInYard` (seção 5) |
| `0x2309` / `0x230a` | `ribAuto` / `ribManual` | `_LayDownQueenMode` = 1 / 0 |
| `0x230b`–`0x2312` | `ribUserDef2A`–`H` | `_DoUserButton` |
| `0x2313`–`0x2319` | `ribbm1`–`7` | marcadores (`_DoBookMark`) |

Dados da faixa (`_DrawYardData` `1010:acd8`, `_font_SetFont(3)`; na lista de fontes do port é o índice 0, Gaia8x14, o que bate 98,5% com a captura [medido]): `ribQueens` = `_QueenStorageB`, `ribBlackCols` = `_ColonyTotalBlack`, `ribRedCols` = `_ColonyTotalRed`, e barras de saúde e população [código]. O port desenha a faixa 35 com o botão do modo selecionado (`_SetYardMode` seleciona `0x2302 + modo`… pela tabela `1048:1586`) [em aberto: conteúdo da tabela; o port seleciona `ribHouse` nos modos 0 e 1, `ribStrat` no 2 e `ribGraphic` no 3, como na captura] e escreve os três números; as barras de saúde e população ainda não.

## 4. Abrir a janela e escolher o modo

- Botão `Yard` da faixa 34 (`0x2202`) e Window → Map/Yard (`Ctrl+M`, comando `0xFD12`, `_OpenMapYard` `1000:9b2a`): abre a janela do quintal no `_YardMode` atual, ou a traz para a frente [código; o port não tem a janela do mapa, então o botão Yard sempre abre o quintal].
- Aceleradores (tabela do `SIMANTW.EXE`) [dados + código, `_ProcMenu` casos 0x21–0x24]: **Ctrl+Y** = Yard View (modo 0), **Ctrl+5** = House View (1), **Ctrl+6** = Strategic View (2), **Ctrl+7** = Graphic View (3). Cada um: `_SetYardMode(m)` se mudou, `_SetMapPlane(0)` se `_MapPlane ≠ 0`, abre a janela 25 e a traz ao topo.
- `_SetMapPlane(0)` (`1000:a5ca`): guarda o plano em `_OldMapPlane`, grava `_MapPlane = 0` e chama `_SetMapPlaneLocation(0, …)` (`1000:a386`), que guarda de novo `_OldMapPlane` (já 0, então ele termina em 0) e grava `_YMapPnt = _CurYardPnt`; no plano 0 não há `_CenterEdit`. Os planos 0 e 1 desenham o mesmo mapa (os testes `_MapPlane < 2` do desenho), então a **janela de edição passa a mostrar a superfície no mesmo canto da vista** que tinha [código; bancada p01: do ninho preto, a edição mostra a superfície]. Faixa: no plano 0 nenhum botão de plano fica selecionado. Quem chama: `_MapToYard` quando troca o mapa pelo quintal (botão Yard) e Ctrl+Y/5/6/7 com `_MapPlane` ≠ 0. **Port (desde 2026-10-18):** `Partida::ir_para_quintal` / `Navegacao::ir_para_quintal`, chamados por `abrir_quintal` (troca) e pelas teclas de modo; teste `ciclo_do_botao_yard_patch`. Antes o port deixava a edição no plano em que estava (divergência removida). `_SpecialXfer` (`1000:7eb6`) também faz `_SetMapPlane(0)` [código].

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

**No port** (`rules/troca_patch.rs`, `trocar_patch`): segue os passos acima e aplica o `Mundo` gerado ao `Estado` (`aplicar_patch`), trocando só os campos que `_RandWorld` escreve. O som de erro das recusas sai como `Evento::Som(1)`; músicas e textos saem como `Evento::Musica` e `Evento::Mensagem`. Os textos viram o quadro de mensagem modal de `_PictStrnDialog` (mensagens.md §3; as recusas 10000/10002/10004 têm o terceiro parâmetro 1 e aparecem mesmo com Options → Events desligado), desenhado **por cima** da janela do quintal (até 2026-10-18 o port o desenhava embaixo dela, e a recusa só aparecia no terminal). A janela de edição troca o conjunto de tiles quando `_TERRAINset` muda (quintal ↔ casa).

### 5.3 `_PlaceQueenInYard` (`1000:ad12`)

Botão `PLACE` (`0x2307`) [código]:

1. Se `_QueenStorageB < 1`: som de erro e mensagem na faixa (lista 1010: 7 "You have no new queens to drop" no Jogo Completo, 16 "Place queens in Full Game only" nos outros; duração 0xB4); fim.
2. `_LayDownQueenMode = 0` (se era 1); os patches válidos são os a até 3 de distância em x e em y do patch atual (limitados ao quintal).
3. Para a simulação (`_StopSimulation`); em cada volta do laço, a mensagem permanente 1010[8] "Click in yard to place new queen"; o jogador move o mouse (o patch sob o mouse é invertido se for válido e diferente do atual) e clica na área do quintal. Se o quintal deixa de ser a janela da frente, limpa a faixa e sai.
4. Fora da faixa válida: 1010[10] "That location is out of bounds" (0x78) e som de erro (com Shift, `_myDelay(30)`). Dentro: música `0x2AFB`, `_QueenStorageB −= 1`, `_YMapPopB[patch] += 1`, `_BColoniesStarted += 1`, `_ColonyUpdateFlag = 1`, redesenho do mapa e 1010[9] "You have started a new colony" (0x78); sem mais rainhas, termina. Depois de qualquer clique, o laço continua **enquanto Shift estiver apertado** (`GetAsyncKeyState(VK_SHIFT)`); senão termina.
5. Retoma a simulação.

Observação: o patch atual também é aceito (não há teste de diferente do atual na confirmação) [código].

**No port** (2026-10-18): o botão PLACE chama `troca_patch::iniciar_colocacao` (mensagens e `_LayDownQueenMode`) e liga o modo "colocar rainha", com a simulação parada (`Partida::parada_pela_interface`); cada clique na área do quintal chama `colocar_rainha` (mesmo teste, mesmas mudanças e mensagens) e `continuar_colocacao` decide pelo Shift do momento do clique; fechar o quintal encerra o modo e limpa a faixa. Teste `place_para_a_simulacao_e_shift_repete` (`engine/tests/partida_quick2.rs`). Não modelados: o patch invertido sob o mouse, o `_myDelay(30)` com Shift, `_ColonyUpdateFlag` (o vetor `acoes/place` bate sem ele: o redesenho do mapa o consome).

## 6. Jogo Completo: início, objetivo e fim

Resumo do que o código faz (detalhes nas specs de regras) [código]:

- **Início** (`_NewGame` tipo 2, geracao-mundo.md §1–2): patch (11, 8), sem colônia preta, a amarela é uma **rainha alada** (T = 0x40) na superfície, `_MeNestStarted = 0`, `_MapPlane = 1` (superfície). A faixa mostra "You need to dig a nest" [medido em `full_inicio.png`]. A colônia vermelha começa no mesmo patch.
- **Meio**: a rainha cava o ninho (`_MeNestStarted` = 1); a colônia cresce; rainhas aladas decolam (`ribMate`) e vão para `_QueenStorageB`; o jogador as põe em outros patches (`PLACE`, seção 5.3); a cada 32 ticks `_SimColonies` faz as colônias do quintal crescerem e brigarem (quintal.md §10). O jogador pode **mudar de patch** com duplo clique num patch com colônia preta (seção 5.2).
- **Objetivo / vitória**: quando há mais de 24 colônias (pretas ou vermelhas) na área da casa (`_HouseColonies > 24`), a casa é posta à venda (`_ForSaleState = 1`, mensagem 10054); se depois disso não houver nenhuma colônia vermelha, fim de jogo com vitória (`_IsGameOver = 1`, `_BlackWon = 1`, mensagem 10057).
- **Derrota**: menos de duas colônias pretas no quintal e nenhuma rainha, ovo, larva nem pupa preta (`_CastePopB[5,0,3,4] = 0`): `_IsGameOver = 1`, `_BlackWon = 0`, mensagem 10056.
- O diálogo de fim de jogo (`_EndGameDialog`, janela 4 `gameOverWin`) é pedido pela fila de eventos (`Evento::DialogoFimDeJogo`); o desenho dele é de outro trabalho (mensagens/diálogos).

## 7. O que o port faz

- Tela de jogo com a janela do quintal (`JanelaQuintal`): aberta pelo botão `Yard`, Ctrl+M, Ctrl+Y/5/6/7 (`_MapToYard`: toma o lugar da janela do mapa); o botão `Patch` da faixa 35 faz `_YardToMap` (o quintal vira o mapa da superfície no mesmo lugar, faixa 34; antes de 2026-10-18 o port só fechava a janela). Opção `--quintal N` abre no modo N (para exportar a tela). Ordem, foco, faixa 34/35 e cliques: gerenciador de janelas ([janelas.md](janelas.md) §8) — o clique numa janela de trás só a traz para a frente.
- Faixa 35 no lugar da 34 enquanto o quintal está aberto; botões House/Strat/Graphic/Patch/Place/Auto/Manual.
- Clique simples num patch: `_YMapPnt`; duplo: `trocar_patch`.
- Teclas e botões em `data/quintal_mapa.json` (sobrescrevível por mod).

## 8. Verificação

**Comparação com o original** (bancada QEMU, 2026-10-17; port exportado com `simant --tipo completo --semente 5 --quintal N --exportar-tela-jogo`, `python tools/comparar_captura.py PORT CAPTURA --mapear --regiao ...`; a partida do port é outra, então menino, cachorro, gato e colônias diferem):

| Tela | Região (área cliente) | Pixels iguais | O que difere |
|---|---|---|---|
| Yard View (`full_yard.png`) | janela 25, (119, 99)–(632, 376) | **99,27%** | menino, cachorro, gato (outra partida), texto do título e do rodapé |
| Strategic View (`yard_strat.png`) | idem | **99,59%** | título, rodapé |
| Graphic View (`yard_graphic.png`) | idem | **99,49%** | título, rodapé, barra do patch atual |
| faixa 35 (`full_yard.png`) | (0, 0)–(640, 73) | **93,09%** | cursor do mouse, barras de saúde/população, botões `ribUserDef2*` (do `WIN.INI`), contagem de colônias (outra partida) — a mesma faixa de diferença da faixa 34 em jogo.md |

A faixa 35 com a posição da tabela 9/0 saía 1 pixel à direita (75,8%); com o canto do objeto 0 no da faixa 34 bate (acima) [medido].

Testes: `engine/src/core/quintal_mapa.rs` (dados, clique → patch como `_YardArea` com truncamento do C, modos e botões, polígono), `engine/src/rules/troca_patch.rs` (rainhas), `engine/tests/troca_patch.rs` (recusas 10000/10002/10004, troca para o quintal com o mesmo `_MapA`/`_MapB` que `_RandWorld` da semente do patch, troca para a casa com `_TERRAINset` 1 e `_Barrier` da casa), `engine/tests/modos_reais.rs` (os quatro modos desenhados, cursor no pixel medido (517, 304), colônia preta no modo 2).

A geração dos patches da casa foi conferida com o original em 4 patches (geracao-mundo.md §5): o fork que fez a casa levou o original a trocar de patch por duplo clique na Yard View (Full Game, colônia preta escrita na memória) e os mapas gerados bateram 100%; isso confirma também o caminho `_XferPatch` → `_RandWorld` (textos "You are transferring to this place in the house" = 10007 e "Sorry, there is not a black colony at this location" = 10004).

**Conferido campo a campo com o original (bancada, 2026-10-17)** — `tests/referencias/acoes/*`, `engine/tests/acoes_originais.rs` (estado antes e depois da ação, com o jogo pausado; o port aplica a mesma ação ao estado de antes e tem de chegar ao de depois, 307 blocos, S e R; `tools/cenarios_tick.py xfer|place|special`):

| Vetor | Ação no original | Resultado |
|---|---|---|
| `xfer` | Jogo Completo do início (snapshot `full`), colônia preta escrita em `_YMapPopB` (10, 8) e `_ColonyTotalBlack = 2`; Ctrl+Y; duplo clique no patch (10, 8) → texto 10006 | 307/307, S e R iguais |
| `xfer_casa` | o mesmo no patch (1, 5) da casa → texto 10007 | 307/307 |
| `place` | `_QueenStorageB = 2`; Ctrl+Y; botão PLACE (360, 60); clique no patch (9, 7) | 307/307 |
| `place_fora` | o mesmo, clique em (5, 8) (fora do alcance) | 307/307 (nada muda além de `_YMapPnt`) |
| `special_xfer` | a amarela morre sem corpo com duas colônias pretas: o tick abre a caixa 10058 e a Strategic View; clique na colônia (10, 8) | 307/307 com S e R iguais, contando o **desenho** da janela do quintal que `_SpecialXfer` abre: `_DrawSwarm` grava `_SwarmCentX/Y` com o patch antigo e o desenho das colônias zera `_ColonyUpdateFlag` (`rules/desenho.rs`; regras/rng.md "Sorteios do desenho") |

O desenho da Yard View e da House View também sorteia S a cada redesenho (chuva: 28 passos; enxame: 2 por formiga, mais 2 por preta fora de ±15) e move `_SwarmX/Y` (regras/rng.md, "Sorteios do desenho"; `desenho::redesenho_do_timer` para a interface chamar depois de cada tick).

Diferenças achadas e corrigidas em `rules/troca_patch.rs::aplicar_patch` (o que `_RandWorld` não escreve fica como estava, como no original): as vagas das listas além do fim (lixo das formigas antigas); `_LastNewHole?`/`_LastNewExit?` quando a cor não tem colônia no patch novo; as vagas das formigas-leão além das geradas e as dos bichos-de-conta (a 0 sempre; todas na casa); `_InitSpider` grava `_StargLife = −1` e `_SuserX/Y = 0x40`; `_InitYelloAnt` (`1030:6492`) só escreve os seus campos e chama `_SetMyHealth(100)` e `_ResetYellowVars` (destino e "último" = posição, `_MeCrazyCnt = −2`), e o resto da amarela (`_MeFoodInType`, `_FuzLocX/Y`, ...) fica. A vista depois da troca: `_MapPnt` e `_AMapPnt` = (0x40, 0x20) (passo 18) e `_MapPlane` como estava (0, porque Ctrl+Y fez `_SetMapPlane(0)` antes) — iguais.

`_PlaceQueenInYard`: o clique que encerra o PLACE também chega a `_YardArea` e grava `_YMapPnt` = patch clicado, dentro ou fora do alcance. No port, `core/quintal_mapa.rs::clicar` grava `_YMapPnt` antes de `colocar_rainha` (corrigido em 2026-10-18; teste `engine/tests/partida_quick2.rs`), como `vetor_tick::aplicar_acao`. `_SpecialXfer` (`1000:7eb6`): zera `_YMapPopB` do patch atual, traz a janela do quintal na Strategic View (`_SetYardMode(2)`, `_ColonyUpdateFlag = 1`, `_SetMapPlane(0)`: `_MapPlane = _MapMode = 0`; `_OldMapPlane` terminou 0 [em aberto]), mostra a mensagem 10058 e espera um clique num patch **com** colônia preta (outro: som de erro e mensagem); com ele, `_YMapPnt` = patch, música 0x2AFB e `_XferPatch`. Tudo **dentro do tick** (passo 17): o resto do tick já roda no patch novo. Mensagens da escolha (lista 1010): 20 "Click on a black colony to continue" (permanente), 10 "That location is out of bounds" e 19 "There is no black colony there." (0x78, com som de erro); no fim, a faixa é limpa. No port: o tick pede a escolha (`Evento::Mensagem(10058)` e `Evento::EscolherColonia`); a partida para a simulação (`Partida::escolher_colonia`); fechado o quadro, a interface chama `quintal_mapa::iniciar_escolha_de_colonia` (`troca_patch::iniciar_transferencia_especial`: estado e mensagem) e traz o quintal para a frente; cada clique chama `troca_patch::escolher_colonia` (recusas ou música + `_XferPatch`). Ao vivo desde 2026-10-18 (teste `transferencia_especial_ao_vivo`). A troca acontece **depois** do tick, não no meio dele [divergência: ver `docs/divergencias.md`].

**Não verificado contra o original:** o modo House View (1) com o menino dentro de casa, os sprites (deslocamentos por quadro vêm das tabelas, sem captura com o menino andando no port e no original no mesmo instante), o cursor nos modos 0/1 desenhado como contorno (medido só na cor).
