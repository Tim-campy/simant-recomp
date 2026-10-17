# Janelas de controle, History, menus de segurar o botão e botões da faixa

Janelas que os botões configuráveis da faixa e o menu Window abrem durante a partida, os menus que aparecem segurando o botão do mouse e o estado desenhado dos botões. Complementa [jogo.md](jogo.md) e [mensagens.md](mensagens.md) (§5, botões da faixa).

Implementação: `engine/src/core/janelas.rs` (Caste/Behavior/History na pilha de janelas, moldura com título), `engine/src/core/triangulo.rs` (conta do triângulo), `engine/src/core/historico.rs` (gráficos), `engine/src/core/menus.rs` (menus), `engine/src/core/partida.rs` (funções dos botões, comandos do menu, espera do clique). Dados: `data/janelas.json`, `data/menus.json`, `data/partida.json` (`botoes_usuario`, aceleradores `janela_*`). Executável: `simant/src/main.rs`.

**Fontes:** pseudocódigo do Ghidra (`re_trabalho/decomp/`, só consulta), DGROUP e código do `SIMANTW.EXE`, layouts do `WINGANT` e as capturas da bancada QEMU do roteiro `tests/referencias/roteiros/qemu_janelas.txt` (`re_trabalho/janelas/j00`–`j12`, fora do git). Legenda: **[código]**, **[medido]**, **[bancada]**, **[hipótese]**.

## 1. Pilha de janelas e posição

- Num Quick Game e depois do Load, `_SetDefaultWindows`/`_NewGame` abrem Behavior (`_OpenModeWindow`, janela 18) e Caste (`_OpenCasteWindow`, 19) antes da janela de edição, que fica por cima [código]. O port guarda a pilha `[Behavior, Caste, Edição]`; o botão da faixa ou o acelerador traz a janela para a frente (`BRINGWINDOWTOTOP`), e o clique numa janela de trás também [hipótese: comportamento normal do Windows].
- Posição: a da tabela tipo 9 (9/0 na bancada, 640×480) mais a faixa (73), com o objeto 0 encolhido pelo contorno (`_win_Recalc`); o retângulo encolhido é o contorno externo da janela do Windows, com o título (18 linhas + 1 de separação) e a área cliente embaixo. Medido: mais 2 linhas (`ajuste_posicao`) [medido em j01, j02, j04].
- Título: caixa do menu de sistema, linha, faixa azul (ativa) ou branca (inativa) com o nome centrado; o port escreve com a fonte `Gaia8x14`, não a fonte do sistema do Windows (diferença de ~1% nas comparações).
- Os títulos ("Caste Control", "Behavior Control", "History Window") ficam em `data/janelas.json` [medido; a origem do texto no original não foi localizada].

## 2. Caste Control e Behavior Control

Variáveis (todas no save): `_casteLevels`/`_modeLevels` (3 níveis de 0 a 65535 que somam 65535), `_casteSetLevels`/`_modeSetLevels` (4 conjuntos de 3), `_CasteAuto`/`_ModeAuto`, `_IdealCaste` (%). Fora do save: `_casteSet`/`_modeSet` (`1048:159e`/`15a0`, conjunto escolhido) e `1048:15b4`/`15b2` (mostrar porcentagem; 1 no início) [código].

### 2.1 Triângulo (`_InitTriVars`, `_GetTriLatDist`, `_BoundPointToTri`, `_UpdateCasteWindow`)

Com o retângulo do objeto `casteTriangle`/`modeTriangle` (com o pixel a mais de `_win_GetObjRect`): largura `W`, metade `M = W >> 1`, altura `H`; vértice de cima em (esq + M, topo), de baixo em (esq, base) e (dir, base) [código]:

- **Clique** (`_ProcCasteEvent` 0x130d): só dentro do triângulo (`_IsPointInIsoTri`); passa para Manual, mostra os conjuntos; enquanto o botão segue apertado, o ponto é preso ao triângulo (`_BoundPointToTri`) e vira níveis (`_GetTriLatDist`):
  - `n0 = ((H − dy − 2) × 65535) / (H − 2)`, com `dy` = y − topo (0 se `dy > H − 2`);
  - `w0 = n0 × M / 65535`, linha `L = W − 2·w0`; com `L < 3`, `n1 = n2 = 0`;
  - `dx = x − esq − w0`; `n2 = 0` se `dx < 3`; `(65535 − n0) × dx / (L − 2)` se `dx < L − 2`; senão `65535 − n0`;
  - `n1 = 65535 − n2 − n0`.
- Ao soltar: os níveis vão para o conjunto atual; nas castas, `_IdealCaste` = (n1·100, n2·100, n0·50, n0·50) / 65535 arredondado (`_cvtLevels2IdealCaste`) [código].
- **Botão** (bitmap 1400, 14×14, índice 13 transparente): centro em `y = (H − 2)(65535 − n0)/65535 + topo`, `x = (L − 3)·n2/(65535 − n0) + esq + w0 + 2` (ou o vértice, com n0 = 65535) [código]. Para bater com as capturas, o port usa aqui o retângulo **sem** o pixel a mais e com 1 a menos à direita e embaixo [medido em j01 e j02; causa em aberto].
- Ordem dos níveis: castas = reprodutores (vértice de cima), operárias (esquerda), soldados (direita); comportamento = forragear, cavar, cuidar [código + j01/j02].

### 2.2 Botões

| Objeto | Efeito |
|---|---|
| Auto (0x1304/0x1204) | `_CasteAuto`/`_ModeAuto` = 1 e esconde os conjuntos (grupo 4) |
| Manual (0x1305) | = 0, mostra os conjuntos |
| Set 1–3 (0x1306–0x1308) | guarda os níveis no conjunto atual, carrega o escolhido, vai para Manual |
| áreas dos números (0x130f–0x1311) | alterna porcentagem / número de formigas |
| ? (0x1303) | WinHelp (não reproduzido) |

[código]. Medido (j01, com `_CasteAuto` = 1): Auto e os Set aparecem levantados e Manual afundado; o port desenha os Set sem escolha com Auto ligado [medido; a lógica dos grupos "invisíveis" não foi fechada].

### 2.3 Níveis e barras (`_DrawControlLevels`)

- Texto em cada objeto de nível (`casteBreeders`… / `modeForage`…), fonte 4: `"%-ld%%"` de `(n·100 + 0x3FFF)/0xFFFF`, ou `"%-ld"` de `(n·total + 0x3FFF)/0xFFFF` com total = `_CastePopB[0]` (castas) ou `_ModePopB[0..2]` (comportamento) [código].
- Barras no objeto `casteGraphs`/`modeGraphs`: largura `(dir − esq)/5`, a barra i em `esq + 2i·largura`; vão da base − 2 até base − 2 − n·(altura − 2)/65535, contorno preto, e o resto da coluna em branco [código; geometria vertical medida em j01]. Cores: as de `_casteColors` (45, 44, 41) e `_modeColors` (42, 43, 46) passam por `_ConvColor`, cuja tabela não foi lida; o port usa índices medidos (W azul, S vermelho, F verde, D amarelo, N ciano; B magenta [hipótese]).

## 3. History (janela 21)

- `_OpenHistoryWindow` (`1000:8a6a`); 10 botões (`histBPop` … `histKilled`, ids 0x1503–0x150c) ligam até 4 gráficos (`_ToggleHistButton`): o novo entra na frente da lista `8:8e54` (o quinto mais velho sai) e pega o primeiro dos 4 lugares livres (`8:8e5c`), que define a cor (`8:8e4c`: `_ConvColor(0x43, 0x46, 0x49, 0x45)` [cores do port: hipótese]). No começo nenhum gráfico está ligado [bancada j04; DGROUP].
- Série do botão b: `g = (b & 1)·5 + b/2` em `_HistGraphs` (`8:8672`: H_BPop, H_BFood, H_BHeal, H_FoodA, H_ADead, H_RPop, H_RFood, H_RHeal, H_PDead, H_FDead); a escala usa também a parceira de `_AltHistGraphs` (`8:869a`) [código + DGROUP].
- `_drawHistGraph`: área `histGraphArea` em branco; topo + 8 e base − 8; maior valor M das duas séries nas `_HistCnt` amostras; multiplica por k enquanto `k·M` cabe na altura, ou divide; 64 amostras do anel a partir de `_HistStart`, x avançando `(largura − 64)/64` por amostra mais o número do gráfico; nome (lista 4/1050) numa caixa no fim [código]. Segurar o clique no gráfico mostra os valores em vez dos nomes (não reproduzido).

## 4. Menus de segurar o botão

### 4.1 Espera do clique simples e decisão (`_processEdit`)

Ver mensagens.md §7. O port começa a espera ao apertar (`Partida::iniciar_espera_do_clique`) e, passados 6 × 55 ms, decide com o botão ainda apertado e Shift solto (`clicar_depois_da_espera(…, segurando)`): na própria amarela, `_AntMenu`; noutra célula com vida, Examine (`_MagnifyMenu`); senão a ordem de sempre (regras: `rules::amarela::clicar_no_mapa` com `segurando`).

### 4.2 `_AntMenu` (`1010:6d24`)

- Janela 7 (formiga), 8 (rainha alada sem colônia: `_MeType` = 0x40 e `8:8a6c` = 0) ou 32 (aranha, `9:9fe8` = 1), aberta por `_win_DoProxMenu` no ponto do clique e mantida dentro da janela principal; o item sob o cursor fica invertido (`_GRectInv`); ao soltar, o item vira `_YellowCommand` pela tabela do DGROUP (`1048:1baa` = 1, 2, 4, 5, 6, 0; `1ba6` = 8, 9, 0; `1bb0` = 10, 11, 6, 0); com Shift, o 2 (recrutar 10) vira 3 (todas). Código 0 = Help [código + DGROUP].
- Posição: o contorno externo (borda do Windows de 5 pixels) 19 pixels à direita e 8 abaixo do clique [medido em j09]. **100%** igual à captura j09.

### 4.3 Examine (`_MagnifyMenu`, `1010:6b4c`)

Chamado por `_processEdit` (depois da espera do clique simples, botão ainda apertado, Shift solto, célula que não é a amarela) com `_MapPlane`, e pela ferramenta 0 do Experimental [código]:

1. Lê a vida da célula (`_LifeA` nos planos 0 e 1, `_LifeB` no 2, `_LifeR` no 3); 0 → devolve −1 e não abre nada (o clique vira ordem; a captura j11, célula vazia, confirma).
2. Canto da janela: uma célula à direita e uma acima da clicada, `((x − _MapPnt.x) + 1)·_tileWidth + _editTileRect.left`, `((y − _MapPnt.y) − 1)·_tileHeight + _editTileRect.top`; se passa da base da janela principal, sobe a própria altura (sem passar do topo); se passa da direita, vai a própria largura para a esquerda (sem passar da esquerda).
3. Abre a janela 29 (`examineWin`, 92 × 87: bitmap 4510 de fundo — com os rótulos "Behavior" e "Carrying" — e os textos `examStr1..3`) e fica num laço **sem tratar mensagens** enquanto `_ButtonHeld` (botão esquerdo, Espaço, Insert, ou o direito/Delete; pelo menos ~2 unidades de `_TickCount`): a simulação não anda. Ao soltar, fecha e devolve o byte de vida; a ordem não é dada.
4. Textos (`_win_DrawExamineWindow` `1010:69c0`, fonte 2), com `g = (vida & 0x78) >> 3`: linha 1, `_EggStr[vida & 7]` (lista 1100) se g = 0, senão `_CasteStr[g]` (1101); linha 2, `_ModeStr[M]` (1102), M da formiga achada na lista do plano (0, "Bored", se não achou); linha 3, `_LoadStr[g]` (1103).
5. Sem sorteios [código]. Plano 0: a linha 2 usa uma variável não inicializada no original; o port usa a lista A. A amarela (0xFE/0xFF) dá g = 15, talvez além do fim de `_CasteStr` [hipótese].

**Port:** `core/exame.rs` (`JanelaExame`: canto, textos e desenho; `data/exame.json`), `rules::experimental::examinar` (vida, grupo, estágio, modo); a interface abre a janela no fim da espera do clique (ou com a ferramenta 0 do Experimental) e a fecha ao soltar o botão ou a tecla; a simulação fica parada enquanto ela está aberta. Sem janela: `simant --save … --examinar X,Y --exportar-tela-jogo …`. Alinhamento e cor dos textos [hipótese: centrados no objeto, tinta preta]; sem captura do original com a janela aberta.

### 4.4 Menu de ícones (Ctrl+clique num botão configurável)

- `_DoUserButton` com `GetAsyncKeyState(VK_CONTROL)` (não "segurar", como dizia mensagens.md §5 antes) abre a janela 36 (16 ícones, funções 0–15) com o item da função atual já realçado (`__win_SetProxItem`); ao soltar sobre um item, a função muda (`_SetUserButton`, bitmaps `0x3889 + f` e `0x3857 + f`); sem Shift, também o botão equivalente da faixa 35 [código; DGROUP].
- Posição: contorno externo 1 pixel à esquerda do botão, logo abaixo da faixa [medido em j07]. **99,93%** (só o cursor). O bitmap 7200 tem formato 0x04: 4 planos de 1 bpp por linha, de cima para baixo (bitmap.md).

## 5. Estado dos botões da faixa (`_SetUserButton`, `_UpdateUserButtons`)

Os botões de alternar (4 trilhas, 5 pausa, 7 Music, 9 Sound Effects, 10 Silly, 11 AutoTrack, 12 Events, 13 Messages) aparecem selecionados com a opção ligada; os outros são momentâneos [código]. Com isso a faixa passou de 93,27% para **96,27%** contra j00 (o resto são as barras de população e saúde).

## 6. O que não foi feito

- Info (função 3, Window → Info): feito em [info.md](info.md), sem os popups de definição e os links do texto.
- Trilhas (função 4): feitas com a janela do mapa ([mapa.md](mapa.md)).
- Mystery (função 8, `_MysteryButton`): sorteia (`_RRand(15)`) entre colocar formigas, matar algumas, comida, buracos, invencibilidade, aranha ao contrário, quadros 10022/10024… — implementado em `rules/misterio.rs` ([../regras/misterio.md](../regras/misterio.md)).
- O clique no gráfico do History (valores), WinHelp.

## 7. Verificação (2026-10-18)

`tools/comparar_captura.py --regiao … --mapear`, port com `simant --save PORT50.ANT --botoes F,… / --menu amarela@X,Y / --menu icones@OBJ / --marcar N --exportar-tela-jogo`:

| Janela | Captura | Iguais | O que difere |
|---|---|---|---|
| Caste Control (na frente) | j01 | **98,87%** | só o texto do título (fonte) |
| Behavior Control | j02 | **98,72%** | só o título |
| History Window | j04 | **98,92%** | o título e a última linha da moldura |
| `_AntMenu` | j09 | **100%** | — |
| menu de ícones | j07 | **99,93%** | o cursor |
| faixa com marcador posto | j05 | **99,73%** | o cursor; mensagem "Bookmark has been placed." e bitmaps do marcador iguais |

Testes: `core/triangulo.rs`, `core/menus.rs`, `core/historico.rs`, `formats/bitmap.rs` (planar), `engine/tests/tela_jogo_real.rs` (clique no triângulo, Set, Auto), `engine/tests/partida_quick2.rs` (botões, menu da amarela, marcadores).

**Não verificado:** arrastar no triângulo contra o original (os níveis lidos da memória depois do arrasto), gráficos do History com dados, o menu da rainha e o da aranha, outras resoluções.
