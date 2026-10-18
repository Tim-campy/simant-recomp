# Janelas de controle, History, menus de segurar o botão e botões da faixa

Janelas que os botões configuráveis da faixa e o menu Window abrem durante a partida, os menus que aparecem segurando o botão do mouse e o estado desenhado dos botões. Complementa [jogo.md](jogo.md) e [mensagens.md](mensagens.md) (§5, botões da faixa).

Implementação: `engine/src/core/gerenciador.rs` (ordem, moldura, cliques, rolagem; §8) e `simant/src/janelas_ui.rs` (ligação com a sessão), `engine/src/core/janelas.rs` (Caste/Behavior/History, moldura com título), `engine/src/core/triangulo.rs` (conta do triângulo), `engine/src/core/historico.rs` (gráficos), `engine/src/core/menus.rs` (menus), `engine/src/core/partida.rs` (funções dos botões, comandos do menu, espera do clique). Dados: `data/janelas.json`, `data/menus.json`, `data/partida.json` (`botoes_usuario`, aceleradores `janela_*`). Executável: `simant/src/main.rs`.

**Fontes:** pseudocódigo do Ghidra (`re_trabalho/decomp/`, só consulta), DGROUP e código do `SIMANTW.EXE`, layouts do `WINGANT` e as capturas da bancada QEMU do roteiro `tests/referencias/roteiros/qemu_janelas.txt` (`re_trabalho/janelas/j00`–`j12`, fora do git). Legenda: **[código]**, **[medido]**, **[bancada]**, **[hipótese]**.

## 1. Pilha de janelas e posição

- Num Quick Game e no Load, `_SetDefaultWindows` (`1000:d66a`) abre (ou traz para a frente) Caste (`_OpenCasteWindow`, 19), depois Behavior (`_OpenModeWindow`, 18), abre o mapa (ou o quintal, no plano 0) **só se** nenhum dos dois estiver aberto, e por fim traz a janela de edição para cima [código]. Num jogo que começa, a ordem fica `[Caste, Behavior, mapa, edição]`; num Load com o mapa já aberto, `[mapa, Caste, Behavior, edição]` — o que a bancada mostra (Ctrl+F6 em x03/x04 traz primeiro o mapa "Black Nest View", depois a Caste) [código + bancada]. O botão da faixa ou o acelerador traz a janela para a frente (`BRINGWINDOWTOTOP`). Ordem, foco, moldura e cliques: §8.
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

**Port:** `core/exame.rs` (`JanelaExame`: canto, textos e desenho; `data/exame.json`), `rules::experimental::examinar` (vida, grupo, estágio, modo); a interface abre a janela no fim da espera do clique (ou com a ferramenta 0 do Experimental) e a fecha ao soltar o botão ou a tecla; a simulação fica parada enquanto ela está aberta. Sem janela: `simant --save … --examinar X,Y --exportar-tela-jogo …`. **Bancada (2026-10-18, `cenarios_tick.py capturas`, três formigas do ninho preto do snapshot rico: cabeça e cauda da rainha e um ovo):** no original a moldura é azul-escura (índice 4, 1 linha preta e 4 de azul) e a 1ª linha do texto é preta, a 2ª e a 3ª azuis (12) — corrigido em `data/exame.json` (`cor_borda`, `cores_texto`). Comparação na região da janela (`tools/comparar_capturas.py`): **82,5%**, **93,2%** e **92,9%**. **Corrigido (2026-10-18):** a janela é contorno preto + 4 pixels de azul (`largura_borda` 5), a área do objeto 0 (92 × 87) branca — a linha branca em volta do bitmap 4510, que fica em (1, 1) — e o contorno no canto calculado por `_MagnifyMenu` (o `ajuste_canto` era somado duas vezes no port, o que deslocava a janela); as três linhas descem 1, 6 e 3 pixels em relação ao topo dos objetos `examStr1..3` (`deslocamento_linhas`, medido: os retângulos do layout têm alturas negativas). Resultado: examine_1 **97,6%** e examine_2 **99,4%** (resta 1 pixel em x numa linha e o cursor); examine_0 **82,6%**: nessa captura o original pôs a janela 3 pixels à direita das outras duas, com a mesma coluna de célula e a mesma vista [causa em aberto].

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


## 8. Gerenciador de janelas: moldura, ordem, foco, cliques e rolagem

As janelas do jogo dentro da área de jogo (edição 0, mapa 1, Info 5, Behavior 18, Caste 19, History 21, quintal 25) são **janelas filhas do Windows** (`WS_CHILD|WS_CLIPSIBLINGS`) da `_rootWnd`, a área cliente embaixo da faixa; a faixa é outra janela. Quem desenha a moldura, move, redimensiona e decide quem recebe o clique é o Windows 3.1 (`DefWindowProc`), com as exceções tratadas em `MAINWNDPROC` (`1000:2930`). O port reproduz isso em `engine/src/core/gerenciador.rs` (sem SDL; dados em `data/janelas.json`, "gerenciador" e "teclas").

**Bancada:** roteiros `tests/referencias/roteiros/qemu_gerenciador_janelas.txt`, `…2.txt` e `…3.txt` (capturas `re_trabalho/janelas_wm/w00`–`w24`, `v00`–`v17`, `x00`–`x06`, fora do git), com o PORT50.ANT carregado como em `qemu_janelas.txt`.

### 8.1 Estilo de cada janela (`_win_Open`, `1030:ca2e`)

O estilo vem dos bytes 0x1C e 0x1D do cabeçalho do layout (`formats::janela::Janela::flags`) [código]:

| Bit | Estilo (palavra alta) | Efeito |
|---|---|---|
| 0x1C & 0x04 | 0xC8 = `WS_CAPTION|WS_SYSMENU` | título com caixa do menu de sistema; sem 0x08, moldura fina (`WS_BORDER`) |
| 0x1C & 0x08 | 0x34 = `WS_THICKFRAME|WS_HSCROLL|WS_VSCROLL` | moldura grossa (redimensionável) e as duas barras de rolagem |
| 0x1C & 0x10 | 0xCA (título, menu, `WS_MINIMIZEBOX`) | nenhuma janela do `WINGANT` |
| 0x1D & 0x01 | 0xC9 (título, menu, `WS_MAXIMIZEBOX`) | só a janela de edição |
| objeto tipo 12 ou 18 | 0xC0 (`WS_CAPTION`) | título |
| 0x1C & 0x40 | — | "modal": com ela no topo, o clique nas outras não as ativa (`WM_MOUSEACTIVATE` devolve 3) |

Valores do `WINGANT`: edição `0e/05` (título, menu, maximizar, moldura grossa, barras); mapa, Caste, Behavior, quintal `06/04` e Info `06/1c` (título e menu, moldura fina); History `06/0c`; menus e quadros `4x/xx` (modais, sem título). Consequências [código + bancada w01–w13]:

- **Nenhuma janela filha tem botão de minimizar** nem ícone minimizado: o "Minimize" do menu de sistema fica acinzentado. Só a janela principal do SimAnt minimiza (no port, a janela do SDL; `JanelasDesenho::minimizada`).
- Só a janela de edição tem botão de maximizar (▲; maximizada, o de restaurar ↕), moldura grossa e barras de rolagem. As outras só mudam de lugar.
- Fechar é pelo menu de sistema: clique simples na caixa abre o menu; **duplo clique na caixa fecha** (w09); Close do menu; Ctrl+F4.

### 8.2 Menu de sistema

`_win_Open` mexe no menu de sistema de cada janela com `WS_SYSMENU`: tira o "Close Alt+F4", põe "&Close\tCtrl+F4" (`SC_CLOSE`, texto em `1048:bda7`), tira o "Switch To…" e põe um separador e "Nex&t\tCtrl+F6" (`SC_NEXTWINDOW`, `1048:bdb6`) [código]. Fica (w04): Restore, Move, Size, Minimize, Maximize, —, Close Ctrl+F4, —, Next Ctrl+F6, aberto embaixo da caixa, com o canto esquerdo no contorno da janela e o primeiro item realçado. O Windows acinzenta Restore (janela não maximizada), Size (sem moldura grossa), Minimize (sem botão) e Maximize (sem botão) [medido em w04, numa janela de controle]. Escape fecha o menu (w04 → w05). O port desenha o menu com a fonte System do Windows (`VGASYS.FON`) e a geometria medida aproximada (`data/janelas.json`, `menu_sistema`) [aproximado: posição do texto e altura dos separadores].

`MAINWNDPROC` trata, para as filhas (`WM_SYSCOMMAND`): `SC_NEXTWINDOW` (a de baixo vem para cima) e, **no quintal, `SC_CLOSE` chama `_YardToMap` antes** (o quintal vira o mapa, que é então fechado) [código]. `WM_CLOSE` de uma filha chama `_win_Close` (esconde e traz a de cima das que sobram, `1030:cf98`) [código].

### 8.3 Ordem Z, foco e roteamento do clique

- Ordem: a de cima é a "ativa" (`_MyGetTopWindow`, `_win_IsWinInFront`); título azul nela e branco nas outras (j01). `_win_Open` de janela já aberta só faz `BringWindowToTop`; de janela fechada (escondida), mostra-a **onde estava** (a posição não volta à da tabela) [código].
- **Clique numa janela que não é a de cima:** `WM_MOUSEACTIVATE` com `WM_LBUTTONDOWN` faz `BringWindowToTop` e devolve `MA_ACTIVATEANDEAT`: a janela vem para a frente e **o clique é comido** [código]. Bancada: em w12 o clique no botão de maximizar da edição inativa só a trouxe para a frente; em v12 o clique num patch do quintal atrás da edição trouxe o quintal (e a faixa 35) sem escolher o patch. O port come o primeiro clique em qualquer parte da janela de trás [hipótese para o título e as barras].
- Só a janela da frente sob o cursor recebe o clique (o Windows manda a mensagem a uma janela só): no port, o gerenciador procura de cima para baixo e o conteúdo das outras não vê o clique.
- O clique direito não é comido (`WM_MOUSEACTIVATE` só come `0x201`): a janela vem para a frente e recebe [código]; no port, só a área do mapa da janela de edição usa o direito.
- **Faixa:** `WM_NCACTIVATE` da janela de edição troca a faixa 35 pela 34; o do quintal troca a 34 pela 35 [código]. A faixa segue a última ativada **entre esses dois**; ativar a Caste não muda a faixa. Bancada v11/v12.
- Aceleradores (`ACCELERATORTABLE`, comandos tratados em `MAINWNDPROC`): Ctrl+F4 e Ctrl+NumPad− = `WM_CLOSE` na de cima (w07); Ctrl+Espaço = menu de sistema da de cima; Ctrl+F6 e Ctrl+NumPad+ = a de baixo para cima (x03, x04); Ctrl+P = Move; Ctrl+Z = maximiza/restaura a de edição se ela é a de cima (w13); Ctrl+A (Size) perde para o Save As [dados + código].

### 8.4 Mover e redimensionar

- Arrastar pelo título: um contorno pontilhado de 1 pixel acompanha o cursor; a janela só vai para o lugar ao soltar (w02, w03). O cursor fica preso à área cliente da janela-mãe (embaixo da faixa): a janela pode sair por cima e pela esquerda, cortada, e a faixa fica por cima dela (w05, w06: cursor em (0, 73), janela com o canto em (−113, 64)) [bancada].
- Redimensionar só a janela de edição (moldura grossa de 4 pixels; os cantos pegam dois lados até 4 + 18 pixels de cada canto). Tamanho mínimo do contorno: largura **317** [medido em v17]; altura 296 [hipótese: `_AdjustWndMinMax` soma métricas do sistema e o retângulo do objeto 0x13; não medida]. Máximo: a área da mãe com a moldura (ou o tamanho atual, se maior) [código]. `WM_SIZE` da janela de edição refaz o layout e a vista: colunas e linhas que cobrem a área cliente (`_UpdateEdit`), com o mesmo canto, limitado ao mapa [código].
- Maximizar (botão ▲ ou Ctrl+Z): ocupa a área da mãe com a moldura de fora (`ptMaxPosition` = −4); a área cliente fica 624 × 334 (39 × 21 células); o botão vira o de restaurar (w13, v01). Duplo clique no título maximiza/restaura [hipótese: padrão do Windows].

### 8.5 Barras de rolagem da janela de edição

Só a janela de edição tem barras (`WS_HSCROLL|WS_VSCROLL`). Faixas (`_ResetEditScrollRange`, `1010:6086`): vertical 0..64 − `_editHeight`; horizontal 0..128 − `_editWidth` (0..64 nos ninhos) [código]. Posição = `_MapPnt` [código].

- Desenho (j00): setas de 1, 3, 5, 7 pixels + haste 3 × 3, centradas nas células de 17 pixels das pontas; o polegar é uma célula de 17 pixels (linhas pretas incluídas) em `trilho + (curso − 17) × pos / max`, com o trilho das linhas pretas das setas. Medido: pos 22 de 39 → polegar em x = 238 da tela.
- Mensagens (`_DoEditScroll`, `1010:5c6e`): seta = `SB_LINEUP/DOWN` (uma célula; `MAINWNDPROC` repete enquanto o botão segue apertado); trilho = `SB_PAGEUP/DOWN` (o tamanho da vista, até o limite; v03/v04); polegar solto = `SB_THUMBPOSITION`; polegar arrastado = `SB_THUMBTRACK`, que **só move a vista com AutoTrack** (`_OptionStates[0]`) [código; bancada: v05 sem AutoTrack, a vista fica e um contorno pontilhado do polegar acompanha o cursor; x01 com AutoTrack, a vista acompanha]. **Reconferido (2026-10-18, roteiro `qemu_patch_rolagem.txt`, capturas `re_trabalho/patch_rol/r*`):** `_OptionStates` lido = `[0, 1, 1, 1, 1, 0]` (AutoTrack desligado); polegar horizontal arrastado +40 e +80 com o botão apertado (r01, r02): a vista não anda, só o contorno pontilhado; solto (r03): a vista pula para a posição. O vertical (r04, r05) igual. Com Ctrl+G (`_OptionStates[0]` = 1) e o mesmo arrasto (r06): a vista acompanha o polegar no meio do arrasto. O AutoTrack vem do `WIN.INI` (`[SimAnt] autotrack=`) num jogo novo e é gravado de volta ao sair (`WRITEPROFILESTRING` no fim do `WINMAIN`, `1000:3fda`, de `_OptionStates[0]` = `8:85f2`), e um `.ANT` carregado traz o seu: quem jogou com AutoTrack ligado uma vez vê o arrasto "ao vivo" nas partidas seguintes [código; o port guarda o equivalente em `preferencias.json`, regras/som.md "Persistência"].
- Ctrl+setas: as mesmas mensagens de linha (jogo.md).

### 8.6 Mapa ↔ quintal e o botão Patch

O mapa (1) e o quintal (25) dividem a mesma janela do Windows: `_win_Swap(de, para)` passa o `HWND`, o retângulo e o lugar na ordem de uma para a outra [código].

- `_MapToYard` (botão Yard, Ctrl+Y/5/6/7): com o mapa aberto e o quintal não, `_SetMapPlane(0)` (a edição passa à superfície, sem centralizar) e troca mapa → quintal; senão abre o quintal [código]. Vem para a frente com a faixa 35 [bancada w20].
- `_YardToMap` (**botão Patch** da faixa 35, `SC_CLOSE` do quintal): com o quintal aberto e o mapa não, `_SetMapPlane(1)` (superfície), troca quintal → mapa no mesmo lugar, faixa 35 → 34 e `_DrawMap`; senão abre o mapa [código]. Bancada w21: o "Yard View" virou "Surface View" no mesmo retângulo, faixa 34, janela de edição na superfície. O port fazia só "fechar o quintal" (registrado em divergencias.md).
- `_OpenMapYard` (Ctrl+M): mapa aberto → para a frente; quintal aberto → para a frente; senão `_YardToMap` [código].
- **Ciclo do botão da primeira posição da faixa** (bancada 2026-10-18, `qemu_patch_rolagem.txt`, capturas `re_trabalho/patch_rol/p01`–`p06`, com `_MapPlane`/`_YardMode` lidos a cada clique): o Yard da faixa 34 e o Patch da faixa 35 ficam no mesmo lugar, então clicar seguidamente ali **alterna dois estados**: ninho preto na edição com o mapa atrás (`_MapPlane` 2) → Yard View na frente, faixa 35 (`_MapPlane` 0) → Surface View no mesmo retângulo, faixa 34 (1) → Yard View (0) → Surface View (1)…; `_YardMode` fica 0 (o modo do quintal não muda) e nenhum clique fecha as duas janelas. Para fechar o quintal ou o mapa: o menu de sistema (Close, Ctrl+F4). O port faz o mesmo ciclo (teste `ciclo_do_botao_yard_patch` em `simant/src/modos.rs`). Ao abrir o quintal, `_SetMapPlane(0)` leva a janela de edição para a superfície no mesmo canto (p01: a edição sai do ninho e mostra a superfície); o port faz o mesmo desde 2026-10-18 (quintal.md §4).

### 8.7 Desenho e o "congelamento" com a Yard

O port desenhava o quintal por cima de tudo, depois dos avisos modais. Um quadro modal (mensagens das regras do quintal, recusa da troca de patch…) ficava escondido atrás da Yard: a simulação parava (`Partida::avancar` não roda com modal aberto), nada parecia acontecer, e o jogo "voltava" quando um clique caía, por acaso, no retângulo do quadro escondido. Agora todas as janelas seguem a ordem do gerenciador (as de baixo da edição entre a faixa e ela; as outras depois), a faixa (outra janela) é refeita por cima delas, e o aviso modal, os menus, o contorno do arrasto e o menu de sistema vêm por último. O clique que "vazava" para a janela de trás também vinha daí: a Caste/Behavior recebiam o clique antes do quintal desenhado por cima delas.

### 8.8 Verificação (2026-10-18)

| O quê | Como | Resultado |
|---|---|---|
| Moldura da edição com setas e polegares, mapa atrás, Caste por cima | `simant --save j00_estado.ANT --exportar-tela-jogo` × j00, `--mapear` | **99,59%** na janela de edição; **99,75%** na área (0, 73)–(640, 442) (resta o texto do título e o cursor) |
| Título do quintal trocado com o mapa, na frente | `--quintal 0` × v12, faixa do título (118, 98)–(634, 140) | **99,99%** |
| Ordem, roteamento, arrasto, limites, menu, fechar, maximizar, redimensionar, polegar | testes de `core/gerenciador.rs` (12) e `core/navegacao.rs` (`polegar_e_tamanho_da_vista`), números das capturas | passam |
| Clique comido na janela de trás (Caste → edição) | `engine/tests/tela_jogo_real.rs` | passa |

**Não verificado:** o desenho do menu de sistema (fonte e separadores aproximados), o contorno pontilhado (o original faz XOR; o port pinta pontos cinza), a altura mínima, Move/Size pelo teclado, o clique comido no título e nas barras de uma janela de trás, Ctrl+Espaço (na bancada não abriu o menu da History, w10), a faixa 35 com os botões configuráveis e os marcadores (`ribUserDef2*`, `ribbm*`, não ligados no port).
