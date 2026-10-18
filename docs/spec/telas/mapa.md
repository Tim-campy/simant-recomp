# Janela do mapa (`mapWin`, janela 1)

Visão geral do plano atual: superfície, ninho preto, ninho vermelho ou um dos cinco mapas de feromônio, com as formigas, a aranha, a pegada do menino e o retângulo da vista de edição. Fonte: pseudocódigo do Ghidra (análise de 2026-10-18; endereços abaixo) e os layouts do WINGANT/SHARED. **Capturas do original** (bancada, 2026-10-18; seção 6): ninho e os cinco feromônios; trilhas ainda sem captura.

## 1. Janela

- Layout 1 (`mapWin`, objeto 0 = contorno t = 2), posição da tabela tipo 9 (entrada 1: (118, 24), 516 × 278) + a faixa, como as janelas de controle (janelas.md). Objetos visíveis: `mapTitle` (0x101, título de 18 linhas) e `mapTileArea` (0x102, 513 × 256: 512 × 256 mais o pixel a mais de `_win_GetObjRect`). Os outros objetos (botões de cheiro, de plano, de janelas, Mystery, painel de populações) estão ocultos: com a faixa, os controles ficam nela [layout; hipótese de que só aparecem sem faixa].
- `_InitMapFunctions` (`1010:c19c`): 4 × 4 pixels por célula (`_mapXsize` = `_mapYsize` = 4) [código].
- Abrir: Window → Map/Yard (Ctrl+M, `_ProcMenu` 0x12 → `_OpenMapYard` `1000:9b2a`): se o mapa ou o quintal já está aberto, vai para a frente; senão `_YardToMap` (`1000:972a`): `_SetMapPlane(1)` e o mapa na superfície. O mapa e o quintal ocupam o mesmo lugar (`_win_Swap(0x100, 0x1900)`); `_MapToYard` (`1000:9918`) volta ao quintal [código].
- **Título** (`_SetMapTitle`, `1000:953c`): a entrada `_MapMode` da lista SHARED 4/1000: 0 "BackYard View", 1 "Surface View", 2 "Black Nest View", 3 "Red Nest View", 4 "Black Nest Marker Pheromone", 5 "Black Trail Marker Pheromone", 6 "Red Nest Marker Pheromone", 7 "Red Trail Marker Pheromone", 8 "Alarm Signal Pheromone" [código + dados].

## 2. Modo (`_SetMapModeAnt`, `1000:9e82`; `_MapMode` = `8:8610`)

- n de 4 a 8 com `_MapMode` = n: volta a 1 (o mesmo botão de cheiro de novo desliga o feromônio).
- n de 4 a 8: se `_MapPlane` ≠ 1, antes `_SetMapPlane(1)` — os feromônios só existem na superfície.
- Grava `_MapMode`, troca o título e marca o botão do modo na faixa (grupo 1 da janela 34; n ≤ 3 desmarca o grupo).
- `_SetMapPlane` termina com `_SetMapModeAnt(_MapPlane)`: **trocar de plano volta ao mapa simples dele** [código].
- Botões da faixa 34 (`_ProcMapRibbonEvent`, `1010:c3d4`): `ribBNestScent` → 4, `ribBTrailScent` → 5, `ribAlarmScent` → 8, `ribRNestScent` → 6, `ribRTrailScent` → 7; ligados também gravam `_ELayerMode` (1, 2, 0, 3, 4; desligado, −1) e redesenham a janela de edição: o botão de cheiro muda ao mesmo tempo o mapa e a camada sobreposta da edição [código].

## 3. Desenho

`_DrawMap` (`1010:cd3e`) gera o buffer do mapa (rotina sem nome `1010:d7a0`) e `_DrawMapData` (`1010:cb60`) só atualiza o painel da faixa (populações e saúde). A pintura (`_win_DrawMapWindow`, `1010:ce46`) só copia o buffer [código].

Geração: um byte de cor por célula num buffer de 128 × 64, comparado linha a linha com o da geração anterior (`_lastMapMapBuf`) para invalidar só o que mudou; cada byte vira um bloco de 4 × 4 pixels [código]:

- **Superfície** (`_GenOverMap`, `1018:46e9`): vida ≠ 0 → `_LTab[vida >> 3]` (a formiga por cima); senão `_CTab[terreno]` (quintal) ou `_HTab[terreno]` (casa).
- **Ninhos** (`_GenNestMap`, `1018:4754`), 64 × 64 no meio (deslocamento de 128 pixels, faixas laterais de `_ConvColor(0xF)`): vida 0 → `_CTabR[terreno >> 2]` (também no ninho preto: `_CTabB` não é usada); 0xFE/0xFF (amarela) → `_ConvColor(1)`; < 0x80 (preta) → `_ConvColor(0xF)`; outras → `_ConvColor(3)`.
- **Feromônio** (modos 4 a 8: `_PherMapBN`, `BT`, `RN`, `RT`, `A`): cada célula da grade de 64 × 32 cobre 2 × 2 células; valor 0 → `_ConvColor(0xB)`, v > 0 → cor 16 + (v >> 4), uma rampa de blocos 4 × 4 pontilhados (tabelas `1048:1a56`, `1a96`, `1ad6`, `1b16`). Não desenha formigas.
- As tabelas do DGROUP estão na numeração do Mac e são convertidas uma vez por `_PatchColorArrays` (`1000:3f0c`) com M = [F, B, 2, 9, E, 5, 4, C, 2, A, 7, 3, 6, 8, 7, 0]; os valores convertidos estão em `data/mapa.json`. A tabela do `_ConvColor` (`1008:4c0e`) é preenchida em tempo de execução [hipótese: a mesma M].
- **Por cima** (plano 1, sem trilhas): a aranha (`_SpidOn`), bitmap 1100 + `_SpidDir` em (4·(`_SpidX` >> 4), 4·(`_SpidY` >> 4)) (`_DrawMapSpider`, `1010:d10e`); a pegada (`_FootHere`), bitmap 1200 + (`_BoyDir` & 3) em (4·`_FootX`, 4·`_FootY`) e, com `_BoyHere` 3 ou 4, o cortador 1210 + (~`_BoyDir` & 1) em (4·`_MowX`, 4·`_MowY`) (`_DrawMapFoot`, `1010:d226`).
- **Cursor** (`_DrawMapCursor`, `1010:c652`): a vista de edição, retângulo XOR de 2 pixels de (4·`_MapPnt.x` + deslocamento, 4·`_MapPnt.y`) com 4·`_editWidth` × 4·`_editHeight`.
- **Trilhas** (`_showTrails`, DGROUP `1998`; botão configurável 4 e objeto 0x10E): **não é feromônio**. Ligadas e com o mesmo modo, a geração parte das células da geração anterior e só escreve as que têm vida: as posições antigas das formigas ficam pintadas e acumulam rastros (sem aranha nem pegada). Ao ligar com a aranha presente, o original redesenha antes sem ela, para não deixar o rastro do sprite [código].
- **Quando:** em `_UpdateWindows` depois de cada tick (com a velocidade como na janela de edição; com a janela atrás e fora do ultra, um redesenho sim e outro não). **Nenhuma rotina de desenho do mapa sorteia** [código].

## 4. Interação (`_MapAreaEvent`, `1010:e100`)

- Clique e arrasto na área: célula = ((x − área.esq) / 4 − 32 nos ninhos, (y − área.topo) / 4); dentro do plano, `_CenterEdit` nela (a vista de edição vai até lá) [código].
- Com Shift, o clique vira o clique da janela de edição naquela célula (`_processEdit`: ordem à amarela ou ferramenta do Experimental) [código].
- O mini mapa (janela 20, `ribToYellow` na faixa) é outra janela, modal, sem feromônio [código].

## 5. No port

- `engine/src/core/janela_mapa.rs` (`JanelaMapa`: geração com trilhas, desenho, célula sob o clique; `modo_do_botao_de_cheiro`) e `data/mapa.json` (tabelas convertidas, blocos, bitmaps, botões de cheiro). A janela entra na pilha das janelas de controle (`Controle::Mapa`, `data/janelas.json` → `mapa`), com a moldura e o título delas.
- `Partida::botao_de_cheiro` (botões de cheiro da faixa) e a sincronização `_MapMode` = plano ao trocar de plano (`Partida::vistas_para_estado`).
- Ctrl+M com o quintal fechado abre o mapa (`Sessao::abrir_mapa`); com o quintal aberto, o quintal vem para a frente. Clique/arrasto na área centraliza a vista. Sem janela: `simant --janela-mapa MODO [--trilhas] [--ticks N] --exportar-tela-jogo …`.
- A geração acontece quando o `_GameTime` ou o modo muda (o original gera em todo `_UpdateWindows`; com trilhas, a acumulação é por tick nos dois).
- **Fora (lacunas):** `_ELayerMode` na janela de edição; o Shift+clique no mapa; o mini mapa; a alternância "um redesenho sim, outro não" com a janela atrás; a troca mapa ↔ quintal no mesmo lugar (`_win_Swap`) e o painel `_DrawMapData`; a cor real de `_ConvColor` [hipótese].

Testes: `core/janela_mapa.rs` (tabelas convertidas; formiga por cima na superfície e trilhas que acumulam; feromônio em blocos 2 × 2, ninho centrado com faixas; célula do clique; alternância do botão de cheiro).

## 6. Verificação contra o original (2026-10-18)

Bancada: `tools/cenarios_tick.py capturas` e `capturas_mapa` (snapshot rico pausado; Ctrl+M abre o mapa no plano mostrado; os botões de cheiro da faixa em (418, 46), (482, 46), (450, 57), (418, 70), (482, 70) dão os modos 4, 6, 8, 5, 7). Port: `tools/comparar_capturas.py` (`simant --save <estado do original> --janela-mapa N --exportar-tela-jogo`, `comparar_captura.py --mapear` na janela, (118, 60)–(634, 376) da área cliente):

| Modo | Iguais |
|---|---|
| 2, ninho preto | **99,89%** |
| 4, Black Nest Marker | 96,28% |
| 5, Black Trail Marker | 98,48% |
| 6, Red Nest Marker | 93,78% |
| 7, Red Trail Marker | 99,52% |
| 8, Alarm | 99,78% |

**Achado:** com o jogo pausado o botão de cheiro troca o título e o modo, mas o conteúdo do mapa **não** é regerado (fica o da superfície, de `_SetMapPlane(1)`): a geração só acontece em `_UpdateWindows`, depois de um tick. Por isso as capturas de feromônio foram feitas depois de um tick (Shift+0, um tick, Shift+0). O port gera ao trocar o modo, também pausado [divergência pequena, de interface]. As diferenças restantes nos feromônios (até ~6% no 6) ficam nos blocos da rampa pontilhada (cores 16+ convertidas pelo `--mapear`); não foram isoladas. Outro achado do port: `--janela-mapa N` com `_MapMode` já = N no save alterna e desliga o feromônio (como o botão do original); `comparar_capturas.py` parte de `_MapMode` = 1.

