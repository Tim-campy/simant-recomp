# Estado do projeto (revisão geral, 2026-10-19)

Revisão refeita por inteiro sobre o branch que integra o `main` em `27543a9` (perfil de driver, API 0.29) mais os resíduos desta rodada (lista A cheia, `_KillAntLion(-1)`, botão esquerdo no laço, `WIN.INI`, bitmaps empacotados, `_LastColonyPopR`, alinhamento do Examine e do mini mapa pelo perfil de driver; API 0.30). Fontes: o código (`engine/src`, `simant/src`, `data/`), as 46 specs de `docs/spec/`, `docs/divergencias.md`, `docs/api-mods.md`, `docs/spec/regras/testes-tick.md`, `tests/referencias/` e a tabela de símbolos `re_trabalho/symbols.tsv` (2.171 nomes do `SIMANTW.SYM`). A revisão anterior (2026-10-18, sobre `1ff4b82`) e os seus adendos "Feito depois desta revisão" foram absorvidos aqui; o que eles listavam como feito está em §2 e saiu de §4.

**Medições desta revisão** (comandos rodados nesta data):

- `cargo test --workspace` com `SIMANT_DIR` = `SimAnt-T/MAXIS/ANTWIN`: **591 testes passaram, 0 falharam, 0 ignorados**, em 36 binários de teste (603 depois da sétima rodada de vetores e do merge do áudio FM; 614 depois da oitava rodada e do merge da API 0.33; **618** depois da rodada do timbre FM e das verificações da geração, do balão e da abertura, API 0.34, mesma data) (unitários de `engine` e `simant` e 33 arquivos de integração em `engine/tests/`). `cargo clippy --workspace --all-targets -- -D warnings`: sem avisos.
- Vetores do original no repositório (`tests/referencias/`, contados nas pastas e na saída dos testes):
  - **221 vetores de tick** (5 na raiz e 216 em 77 subpastas de `ticks/`; eram 173 antes da sétima rodada e 204 antes da oitava, `regras/testes-tick.md`): **220 exigidos 307/307** blocos, com extras e vida, por `engine/tests/ticks_originais.rs`; o restante (`inseticida_morte/21`, fim de jogo) é conferido pela interface (`fim_de_jogo_interface.rs`), 307/307 com S e R. Nenhum vetor pendente.
  - **2 sequências longas** (`ticks_longos/rico_200`, 200 ticks; `full_600`, 600 ticks do Jogo Completo com a Yard View na frente, inclusive os 17.650 passos de S do desenho entre os ticks): iguais ao original.
  - **21 ações da interface** (`acoes/`: as cinco teclas de recrutar e liberar da amarela, desde a oitava rodada; inseticida num tile de formiga-leão sem entrada, desde a sétima; troca de patch e da casa, PLACE dentro e fora, `_SpecialXfer`, Quick → Experimental, Start Game, Ctrl e arrasto, ninho e cheiro do Experimental, chuva e enxame desenhados, dois Mystery): todas 307/307.
  - **1 Load do tutorial** (`load_tutorial/`): 307/307.
  - **21 casos de regra** da bancada (`casos/*.json`): 20 confirmados no original e iguais no port; C-10 não conferível (mensagem modal no Load). (C-04 a C-07 viraram vetores de tick.)
  - 76 roteiros de bancada em `tests/referencias/roteiros/`.
- Cobertura por nome das funções do original (heurística, §2.13): das 1.319 funções dos segmentos de código 1–7, **497 (38%) aparecem pelo nome no código ou nos dados do port** e 715 (54%) no código ou nas specs (eram 452 e 663 na revisão anterior). Nos módulos de regras `SIMONE` e `SIMANT1`, 185 de 292 (63%) estão no código e só 24 não aparecem em lugar nenhum.

## 1. Resumo

### Critério

- **Implementado:** o port faz o que o original faz naquela área, ligado ao jogo (não só na engine), a partir da spec. Funções do original substituídas por equivalentes modernos (gerência de memória, `_win_*`, primitivas `G*`, runtime C) contam como feitas quando o efeito visível está reproduzido.
- **Verificado contra o original:** há evidência de comparação com o SimAnt rodando (vetor de tick 307/307, caso da bancada, captura comparada pixel a pixel, medição de ritmo) para aquela parte. "Visto no código" (pseudocódigo do Ghidra) **não** conta como verificado. Por isso o "verificado" é sempre ≤ "implementado".
- As porcentagens são estimativas do revisor, feitas a partir das lacunas listadas nas specs e no código; não são medidas automáticas. Onde a área é extensão do port (mods, ferramentas, documentação), não existe "contra o original" e a coluna fica "n/a".

### Tabela por área

| # | Área | Implementado | Verificado no original | Comentário curto |
|---|---|---|---|---|
| 1 | Formatos de arquivo | 95% (era 90%) | 85% | banco NDX/DAT + LZSS, bitmaps (os três bancos decodificam inteiros desde 2026-10-19, com o `kind` 0xFFFF empacotado e o planar sem alinhamento), fontes, janelas, `SIMANT.CFG`, `.ANT` (byte a byte), sons, tiles, `.HLP`; restam o byte 3 dos bitmaps, os bits 4/8 das flags e o layout binário tipo 0 |
| 2 | Regras da simulação (tick) | 98% (97%) | 84% (80%) | 221 vetores (todos iguais), 2 sequências, 21 ações e 20 casos; estratégia vermelha, recrutamento, limiar da revoada, pedrinha e volta de `_Cycle` na oitava rodada; desenho dentro do tick, aranha do jogador com AutoTrack; `_SpecialXfer` dentro do tick (2026-10-19, ação `special_xfer` 307/307 assim); restam hipóteses de defeitos do original (§3) |
| 3 | Geração de mundo | 97% (96%) | 92% (77%) | Quick Game byte a byte; 12 patches da casa (todas as receitas menos a 8) e o jogo novo de Full, Experimental e Tutorial 307/307 antes do 1º tick (2026-10-19); Tutorial via Load; `_DROPdir` nas quatro direções; faltam o patch 8, `_DigOut*` no quintal e `_AddFood(≥ 0)` |
| 4 | Jogo ao vivo (laço, relógio, ritmo) | 96% (95%) | 68% (67%) | laço de `MYTIMERFUNC` com o clique esquerdo (2026-10-19); slow/normal medidos (3,0/6,0 ticks/s); fast/ultra e relógio do Mac com decisões do port; rolagem repetida a ~23 passos/s com a simulação parada (medido, 2026-10-19); com o jogo pausado a edição guarda o patch antigo depois da troca, como o original |
| 5 | Telas e janelas | 96% (95%) | 86% (85%) | todas as janelas alcançáveis existem e foram comparadas (93–100%), inclusive balões, aranha, mapa, mini mapa e Info (51 cartões); tela virtual 640/800/1024 e **perfil de driver** (VGA de 16 cores × 256 cores: alinhamento ao byte e cores do sistema), com as caixas, o WinHelp e a janela 26 conferidos na SVGA; desde 2026-10-19, cores do sistema 18–20 por driver, History do WinHelp no `WIN.INI`, contorno do arrasto em XOR, 9100/9101 e Status sem ajuste (100%/99,90%), faixa 35 e mapa depois do Load como no original; os 23 balões do menino (99,95%) e as barras da janela na abertura e no "Select a Game" (100%) conferidos depois |
| 6 | Diálogos e menus | 95% (93%) | 88% (83%) | barra de menus do Windows (100% nas capturas), About, Status, Quit, Load Game, quadros da janela 26, menus de um item do botão de ferramentas no Quick/Completo (100%, 2026-10-19); barra de título e menu de sistema da janela principal opcionais (`moldura_windows31`, 100% contra a bancada, 2026-10-19); Status no Full/Experimental e os dois Cancel da abertura conferidos (oitava rodada); faltam os diálogos internos do WinHelp |
| 7 | Som e música | 88% (86%) | 72% (65%) | efeitos e músicas de regras, Experimental, diálogos e File → New ligados; sintetizador FM (OPL) próprio como padrão, com o banco ajustado contra as 28 músicas gravadas do original (nível −0,1 dB em média em relação à abertura, era +3,4; curva de velocidade, perfis harmônicos, percussão com as notas mudas do original); efeitos dentro do jogo sem captura |
| 8 | Modos (Quick, Completo, Experimental, Tutorial) | 94% (93%) | 86% (84%) | os quatro têm vetores e o jogo novo de cada um conferido antes do 1º tick; tutorial conferido da lição 1 ao fim; PLACE, `_SpecialXfer` ao vivo e avisos do Experimental feitos |
| 9 | Salvar/carregar | 98% (95%) | 90% | `.ANT` aceito pelo original; Load com `_RandYard`; Load Game no "Select a Game"; preferências com o `WIN.INI` do usuário como origem (2026-10-19) |
| 10 | Mods (dados + Lua) | 92% (90%) | n/a | API 0.35 (textos do original lidos da instalação, `lancamento.md` §6); dados mesclados por chave; ganchos dentro do tick; 42 `data/*.json` sobrescrevíveis (mais o `save_layout.json`, que não é) |
| 11 | Ferramentas e testes | 92% (90%) | n/a | 44 ferramentas em `tools/`, bancada QEMU com gdbstub (VGA e SVGA, `tick_chamada` para o jogo novo), 618 testes |
| 12 | Documentação | 92% (90%) | n/a | 46 specs (~8,8 mil linhas), divergências revistas (33 em aberto, 56 resolvidas) |

**Geral (conta em §6): cerca de 95% implementado e 84% verificado contra o original** (este último só nas áreas 1–9; era 82% antes da rodada de 2026-10-19 do timbre FM e das verificações). Na revisão anterior: 88% e 70%.

## 2. Por área

### 2.1 Formatos de arquivo

- **Completo e verificado:** banco `.NDX/.DAT` com LZSS (todas as telas e bitmaps saem dele); bitmaps e paleta (paleta do Windows com a troca 7/8 corrigida; título 99,98%, "Select a Game" 99,87%); fontes do Mac e Gaia (`fontes_mac_batem_com_o_original`); layouts de janela (tipos 0/1); save `.ANT` (48.386 bytes, 307 blocos, memória = arquivo, gravado pelo port e aceito pelo original); tiles (ninho preto do `QUICK2.ANT` 100%); sons tipo 5/51/18/20/50; `SIMANT.HLP` (WinHelp 3.0, 99,94–99,97%).
- **Feito nesta rodada (2026-10-19, `formatos/bitmap.md`):** o campo `kind` lido no código (`_DrawBitMapToBuffer`): 0 opaco, 3 com máscara (o índice 13 é a cor transparente fixa), 0xFFFF **empacotado** (LZSS dentro do objeto, `_GUnpackPic`); o planar tem ⌈w/8⌉ bytes por plano, sem alinhar. Com isso os **três bancos decodificam inteiros** (352 bitmaps no `WINGANT`, 58 no `SHARED`, 343 no `MWINNT`; teste `todos_os_bitmaps_dos_bancos_decodificam`). O item "bitmaps ~7100 com faixas" era antigo: não há bitmaps entre 7011 e 7199; os estreitos e altos são as gotas e o enxame (7005–7007), desenhados e conferidos.
- **Feito e conferido em 2026-10-19:** o caractere ausente das fontes do Mac sai como o glifo 0 (vazio), porque `_font_MakeImage` (`1030:ade0`) guarda o índice 256 num byte; a largura continua a da caixa (balão nº 4 do menino, `formatos/fonte.md`).
- **Implementado, não verificado:** Gaia8x14 e fontes com `kernMax < 0` (`formatos/fonte.md`); bits 4 e 8 do índice e sufixo dos nomes de banco (`banco-dat-ndx.md`, [hipótese]); classes do campo T acima de 2 (`save-ant.md`, [hipótese]); o modo monocromático (`MWINNT`), que o port decodifica mas não usa.
- **Falta:** o byte 3 do cabeçalho de bitmap (não é lido pelos caminhos de desenho analisados; [hipótese] resto da conversão do Mac); o quinto plano dos planares com máscara do `MWINNT`; o layout binário tipo 0 ("estrutura ainda não decodificada", `banco-dat-ndx.md`; o port usa o tipo 1); `_PatchColorArrays`/`_SetUpPalette` não analisadas.

### 2.2 Regras da simulação (tick)

- **Completo e verificado:** ordem do tick (`_DoAntSim`), geradores S e R, superfície (lista A, feromônios, luta, comida, entrada no ninho, líder vermelha), ninhos preto e vermelho (escavação, comida, saída, invasoras, água), ciclo de vida (ovos, rainha, castas, casos C-04 a C-07), inimigos (formiga-leão com cerco, aranha com cerco, fuga, vingança, faro, laser e aranha do jogador com e sem AutoTrack, lagarta, bicho-de-conta), formiga amarela (andar, contorno, comer, ninho, luta, as mortes 1–10, choque, loucura), quintal (menino, cortador, inseticida, gato, cachorro, pássaro, chuva, enxames, `_SimColonies`), conselhos e tutorial no tick, trapaças, e o **desenho dentro do tick** (sorteios do redesenho da janela de edição em `_GotoMyAnt`, nas animações, na trofalaxia e em `_DoAntMoveY`, inclusive com "Silly" e com o `_TickCount` do momento). Evidência: 173 vetores, 2 sequências, 15 ações, 20 casos.
- **Feito nesta rodada (2026-10-19):** `_ExitHole` com a lista A cheia igual ao original (a formiga vai para a folga da vaga 1000 e se perde; a compactação deixa uma vaga com resto; `regras/ninho.md` §4.6); `_KillAntLion(-1)` do Inseticida reproduzido no que cai no estado (a formiga-leão 0 sai da lista; bytes altos de `_BoyMsgCnt` e `_LastThemeSong`), sem as gravações em ponteiros do Windows (`regras/inimigos.md` §1.4); `_LastColonyPopR` fora do save (1 depois de `_InitSimYard` e do Load; `regras/quintal.md`). Nenhum dos três tem caso na bancada.
- **Implementado, não verificado:** a lista consolidada está em §3 (defeitos do original marcados [hipótese] e poucos outros). **Conferidos na oitava rodada (2026-10-19, `regras/testes-tick.md`):** estratégia vermelha (T9) pelos três caminhos do MALVADO e a escada preta no modo 0, recrutamento pelo jogador (T11, as cinco teclas, listas A, B e R), o tick exato em que `_FlyAwayR` arma (o da contagem com mais de 20 reprodutores), a queda de comida na cozinha e nos ladrilhos, a pedrinha cuspida pela formiga-leão e a volta de `_Cycle` (duas fases 0 seguidas); o giro duplo de `_GetNestDir` ficou confirmado de forma indireta (91 giros de 90° de carregadoras nos vetores que batem). **Conferidos em 2026-10-19 (sétima rodada de vetores, `regras/testes-tick.md`):** morte natural por casta e das rainhas, proporção de vitórias, revoada natural, N-02/04/05/08/11, afogamento de adultas, lista A cheia em `_ExitHole`, `_KillAntLion(-1)` (com uma correção: o `_SetMap` da vaga −1), `_LastColonyPopR` depois do Load, o clique esquerdo no laço do `MYTIMERFUNC` e a queda de comida na casa nos dois carpetes.
- **Feito em 2026-10-19 (antes parcial):** `_SpecialXfer` troca de patch dentro do tick, com a escolha conhecida (`Estado::transferencia_no_tick`; ao vivo o tick é refeito a partir da foto do começo; telas/quintal.md §8). O registro antigo: trocava **depois** do tick (o original troca dentro do passo 17; S e R batem assim mesmo nos vetores e na ação `special_xfer`); desde 2026-10-18 a partida ao vivo reage à mensagem 10058.
- **Falta:** modos 17/18 e grupos 10/11/15 sem significado conhecido (`formigas.md`); `_RevSpider` sem chamador visto (`inimigos.md`); chamadas indiretas de `_ForceModeA/B`, `_StartAttack`, `_GetNewRedTask` não procuradas (`superficie.md`); a direção dentro dos ninhos de `_GetRandDir`/`_GetDefendDir` (`feromonios.md`, "ainda não foi portada"; os vetores do ninho batem, então ou o caminho não é exercitado ou a spec está desatualizada — a conferir).

### 2.3 Geração de mundo

- **Completo e verificado:** `_RandYard`/`_RandWorld` do Quick Game (`engine/tests/geracao_quick2.rs`); casa em **12 patches** pelo `_XferPatch` (`xfer_casa` e, desde 2026-10-19, `casa_igual_ao_original_depois_de_xfer_patch`: os patches 5, 6, 7, 9, 15, 36, 37 e a cozinha sem pia, com `_MakeNewHoleR`, `_DigOutBNest` e `_DigOutRNest` na casa; todas as receitas menos a 8); o **jogo novo de Full Game, Experimental e Tutorial** gravado antes do 1º tick (`tick_chamada` na bancada) e regerado pelo port, 307/307 blocos (`engine/tests/geracao_modos.rs`; corrigidos `_ResetYellowVars`, o resto de `_InitSpider`, `_ClearHistory(1)`, `_CurEditMode` por plano e cinco valores estáticos do executável, `geracao-mundo.md` §8); `_InitPillar`; `_GenerateTutorial` no Load de `Tutorial.Ant` (`load_tutorial/`); jogo novo depois do fim de jogo (`inseticida_morte/21`).
- **Implementado, não verificado** (`geracao-mundo.md`): o patch 8 (clipe e mesa; as rotinas dele foram conferidas em outros patches); `_DigOutBNest/RNest` num patch do quintal; `_AddFood` com parâmetro ≥ 0; o jogo novo escolhido durante a partida (só visto no código). A queda de comida na casa foi conferida nas quatro direções de `_DROPdir` (carpetes na sétima rodada, cozinha e ladrilhos na oitava; `superficie.md` 8.2).
- **Falta:** só a verificação dos itens acima.

### 2.4 Jogo ao vivo (laço, relógio, ritmo)

- **Completo e verificado:** temporizador de 55 ms com 3 mensagens por chamada; 6,06 ticks/s em normal e 3,03 em slow contra 6,0/3,0 na bancada; pausa (inclusive a amarela andando na pausa); simulação parada com quadro modal e nas animações; `_gameCycles` por volta.
- **Feito nesta rodada:** o clique esquerdo encerra o laço de voltas como o `GetAsyncKeyState(VK_LBUTTON) & 1` do original (`simulacao-visao-geral.md` §2).
- **Implementado com decisão do port, não verificado em máquina real:** fast/ultra com 5 e 7 voltas fixas (66,7 e 90,9 ticks/s; com `-icount` o original faz ~3 ticks/s em fast por milhão de instruções/s, então o port equivale a ~20–30 M instruções/s, um 486 da época; `simulacao-visao-geral.md` §3.3); relógio do Mac começando no maior prazo do save; `mac_por_tick` (9 na bancada); cadência sem foco: medida igual à com foco (61 ticks em 10 s ativa, com a Task List e minimizada; 2026-10-19), e o port passou a 17 ms sem foco.
- **Feito em 2026-10-19 (depois):** com o jogo pausado, depois da troca de patch a janela de edição continua mostrando o patch antigo até o próximo redesenho (`_GameTime` andar ou a vista mudar), como o original (`Sessao::edicao_congelada`; teste `edicao_mostra_o_patch_antigo_ate_o_proximo_redesenho`; que uma rolagem com o jogo pausado redesenhe com o patch novo é [hipótese]).
- **Feito em 2026-10-19:** a repetição da rolagem no ritmo do redesenho medido (~23 passos/s, sem a repetição do sistema) e com a simulação parada durante o laço (`GameTime` parado na bancada, telas/jogo.md); descarte de efeito pedido no mesmo milissegundo (§2.7).

### 2.5 Telas e janelas (por janela)

| Janela | Implementado | Verificado | Evidência / o que falta |
|---|---|---|---|
| Abertura e título | 100% | 96% | barra de menus (e título com `moldura_windows31`) desenhados e parados como no original, 100% nas linhas 0–37 (2026-10-19); 99,96% contra a VGA de 16 cores com o perfil `vga` (o deslocamento de 3 pixels era o `CS_BYTEALIGNCLIENT`) e 99,89–99,99% nas três resoluções da SVGA; duração (~39,6 s) medida |
| "Select a Game" | 100% | 92% | 99,87%; desde 2026-10-19 com a barra de menus (e o título) da janela principal parada: Alt+F, cliques no File, na caixa e no minimizar não fazem nada no original (`qemu_selecao_moldura.txt`), 99,82% com a moldura; Load Game não conferido na bancada |
| Tela de jogo: faixa 34 | 95% | 90% | botões, barras, popup das castas, marcadores, mini mapa conferidos; botões configuráveis com bitmaps trocados |
| Tela de jogo: janela de edição | 95% | 90% | mapa 100%, janela 99,33–99,59%; balões "Silly" (100% nas regiões dos balões), aranha (18 capturas 99,91–100%), formiga-leão, lagarta, bicho-de-conta, navegação, superfície e ninho vermelho conferidos; `_DrawEditGraphs` não desenha nos modos do Windows (`telas/baloes.md` §3); faltam a rainha amarela 0xFE e a 2.ª metade de alguns bichos (tabela não localizada, `jogo.md`) |
| Gerenciador de janelas | 90% | 75% | ordem, foco, arrasto, maximizar, menu de sistema, alinhamento da área cliente (`CS_BYTEALIGNCLIENT`); contorno XOR; menu de sistema com o acinzentado em cinza-claro, o realce acinzentado do `GrayString` e a caixa invertida (w04, 2026-10-19); altura mínima [hipótese] |
| Caste / Behavior / History | 100% | 90% | 98,72–98,92% (título); arrasto no triângulo e History com dados conferidos na bancada |
| Menus de segurar (amarela, rainha, aranha, ícones) | 100% | 95% | `_AntMenu` 100%, ícones 99,93%, rainha alada e aranha conferidos |
| Mapa (`mapWin`) | 100% | 90% | 93,78–99,89% por modo; cliques, Shift, duplo clique, arrasto, troca com o quintal e alternância `_DrawMap`/`_DrawMapData` atrás; cores de `_ConvColor` [hipótese] |
| Mini mapa | 100% | 95% | 99,99–100%; teclas descartadas, como no original |
| Quintal (Yard View, faixa 35) | 95% | 87% | balão do menino: os 23 textos conferidos (99,95%; y +1 e chuva por cima do balão corrigidos, 2026-10-19); 99,27–99,59% nos três modos, faixa 93,09%; botões configuráveis da faixa 35 sem captura com bitmaps trocados; gato 1 pixel abaixo por ajuste medido [causa em aberto] |
| Info (cartões) | 95% | 90% | **51 dos 53 cartões** contra capturas (99,64–99,96%); 127 e 129 não são alcançáveis por clique; clique em palavra em negrito não conferido |
| Examine | 100% | 95% | 97,6–99,4% na janela; o deslocamento da primeira abertura explicado pelo alinhamento da área cliente |
| Ajuda (visor do WinHelp) | 95% | 90% | 99,94–99,97%; menus, Search, History e popups 99,29–99,89%; itens que abririam diálogos do WinHelp só fecham o menu; desenhada na área cliente, 2 linhas acima do original |
| Diálogos de arquivo | 90% | 85% | 99,28–99,97% nas partes comparáveis; imitação (unidade fixa "c:", "Read Only" sem efeito); rolagem com muitos arquivos não conferida |
| Barra de menus, About, Status, Quit | 95% | 95% | barra e os cinco menus 100%; About 98,81%; Status 100% (com ajuste medido); Quit 99,51% (`telas/menus.md`) |
| Menu de ferramentas do Experimental | 100% | 100% | 100% na bancada |
| Quadros de mensagem, faixa, janela 26 | 100% | 95% | 100% nas capturas; `_YellowDialog`, `_SpiderDialog`, `_LionDialog` 100%; 9100/9101 2 pixels à esquerda por ajuste medido |
| Fim de jogo e pontuação | 100% | 90% | 97,95%; pontuações 19785 e 27572 iguais; `_DrawSimPayoff` 99,78% |
| Telas maiores e perfil de driver | 95% | 80% | tela virtual 640/800/1024 com a tabela de posições 0/8; perfil de driver (`data/resolucao.json`: `vga` padrão, `svga256`, `s3_dosbox`) com o alinhamento ao byte das janelas (`CS_BYTEALIGNCLIENT`) e dos diálogos, as cores do sistema do driver e o pixel do S3; caixas de mensagem, diálogo de arquivo, janela 26 e WinHelp posicionados por resolução e conferidos na SVGA (`posicoes_por_resolucao_e_driver`); Select a Game 99,80–99,92% e Quick Game 96–99% contra a SVGA; Examine e mini mapa pelo perfil desde 2026-10-19 (sem captura em 256 cores); a VGA de 16 cores em 800×600/1024×768 não existe na bancada |

### 2.6 Diálogos e menus

- **Completo e verificado:** barra de menus da janela principal (File, Window, View, Options, Speed; itens, marcas, aceleradores, navegação por teclado; 100% nas capturas m00–m08), About (98,81%), Status (`_ScoreDialog`, 100%), Quit (99,51%), quadros de mensagem, fim de jogo, "Saved correctly", pergunta do Load, Save As/Load (imitação), menus de segurar e de ícones, menu de ferramentas do Experimental, quadros animados da janela 26 (trofalaxia, 9100/9101, aranha, formiga-leão, vitória).
- **Conferidos na bancada em 2026-10-19 (oitava rodada):** Cancel do "Load Ant Hill" aberto pela abertura (o "Select a Game" volta idêntico), CANCEL do "Select a Game" da abertura (o SimAnt encerra, como o port), Status no Full Game e no Experimental (99,56% e 99,62%; `telas/menus.md` §6).
- **Implementado, não verificado:** a ajuda dos itens de menu com o modo de ajuda (`_ProcMenuHelp`); diálogos de arquivo modais em relação ao `WM_TIMER` [hipótese].
- **Feito em 2026-10-19 (depois):** as barras da janela principal também na abertura e no "Select a Game" (barra de menus sempre, título com `moldura_windows31`), paradas como no original (`telas/menus.md` §8).
- **Feito em 2026-10-19:** o título e o menu de sistema da janela principal (opção de vídeo `moldura_windows31`, padrão desligado, decisoes.md; com ela as setas passam pela caixa; telas/menus.md §8), conferidos 100% contra m00, m05b e s00–s16; e a coluna e a linha cinza-claro por fora dos menus (m01, m05b).
- **Falta:** o menu de um item (`editToExp`, janela 10; **feito em 2026-10-19**) do botão de ferramentas no Quick Game (o port vira Experimental direto); os diálogos internos do WinHelp (Open, Print, Annotate, Define…).

### 2.7 Som e música

- **Completo:** 57 efeitos decodificados (8 bits, 4096 Hz); músicas `.MID` com a tabela MCI; opções Music/Sound Effects; silêncio sem foco em todas as telas; linha do tempo dos sons da luta (165 ms); sons das regras, das ferramentas do Experimental, dos marcadores, das trapaças, do Mystery, da troca de patch, do fim de jogo, dos quadros da janela 26 e de File → New (o único item da barra que toca algo no original).
- **Verificado:** formato e catálogo (pelo código). **Desde 2026-10-19, contra a captura do original no DOSBox-X** (SB16 + OPL3, MIDI Mapper "SB16 Ext Synth" da instalação, sem som no PC; `tools/som_dosbox.py`, `regras/som.md` "Áudio do original no DOSBox-X"): na abertura (10001), duração (102,90 s contra 102,99 s de MIDI), andamento, notas e ordem (croma alinhada 0,85 contra 0,43 deslocada), saída mono e o volume relativo música/efeito (a música 20 dB abaixo de um efeito de ganho 1, medido com o efeito 41 pelo mesmo `waveOut`). Os canais 1–10 sem mapas conferem com o `MIDIMAP.CFG`.
- **Parcial / decisão do port:** timbre por um **sintetizador FM próprio no estilo do OPL** com banco GM próprio (`data/opl_banco.json`), padrão desde 2026-10-19 (`decisoes.md`), mais perto do original que o simples, mas não igual; SoundFont e o simples continuam selecionáveis. **Ajustado em 2026-10-19 contra as 28 músicas** (`regras/som.md`, "Ajuste do banco FM contra as 28 músicas"): curva de velocidade medida (25 dB/década, era 40), os 15 programas das músicas pelos perfis harmônicos e envelopes medidos (19 em FM 3:1, 21 só ímpares, 59 com todos os harmônicos, 95 quase senoidal, 49 com ataque de ~0,5 s), nível por programa, percussão com alturas e níveis medidos e as notas 59, 67, 70, 77, 80 e 85–95 mudas como no original; ganho recalibrado (abertura −20,0 dB contra um efeito de ganho 1). A comparação tira a componente contínua que o DOSBox-X grava (era ela que fazia o 95 e o 49 parecerem ter "outro envelope" e as `jolly*` parecerem escuras). Antes → depois: nível em relação à abertura +3,4 → −0,1 dB em média (módulo 3,6 → 0,8), espectro médio 0,67 → 0,78, envelope 0,85 → 0,94, centróide a 44 % → 18 % do original. Efeito novo sempre corta o anterior (o original descarta um pedido no mesmo milissegundo).
- **Feito em 2026-10-19:** o descarte de um efeito pedido no mesmo milissegundo em que outro começou (`_myBeginSound`; `simant/src/som/controle.rs`): de dois efeitos seguidos no mesmo tick, fica o primeiro [não medido no original].
- **Capturado sem input (2026-10-19, `som_dosbox.py musicas`):** as outras 27 músicas, tocadas pelo próprio jogo na cópia com o `GAMETHME.MID` trocado (mesmo caminho MCI → SB16 FM): notas, ordem e andamento batem em todas; o nível e o timbre foram ajustados depois (item acima).
- **Não capturado:** os efeitos dentro do jogo (taxa real dos efeitos de 4096 Hz na SB16, luta, preempção): o `AUTOTYPE` do DOSBox-X entrega teclas no DOS com a janela minimizada (o problema anterior era o layout BR), mas não no Windows 3.1 em modo 386 avançado; a bancada QEMU não tem OPL3 e toca os efeitos errado.
- **Não ligados porque o caminho não existe no port:** `_StartAttack` pelo menu e os menus de ferramentas da janela de edição e da janela do mapa (os botões não existem no Windows).

### 2.8 Modos

| Modo | Implementado | Verificado | Notas |
|---|---|---|---|
| Quick Game | 97% | 92% | quase todos os vetores saem dos snapshots `quick`/`rico` |
| Jogo Completo | 94% | 88% | `jogo_completo/`, `full_600`, troca de patch, PLACE (para a simulação e repete com Shift), `_SpecialXfer` ao vivo e dentro do tick, 38 patches da casa (12 receitas conferidas), jogo novo conferido antes do 1º tick, vitória por `_DrawSimPayoff`; falta o patch 8 |
| Experimental | 95% | 86% | ações `exp_*` 307/307; jogo novo conferido antes do 1º tick (2026-10-19); textos do exame e avisos (listas 4/1010, 4/1100–1103) desenhados, sem captura dos avisos na faixa; o menu de um item do Quick Game falta (§2.6) |
| Tutorial | 95% | 90% | `tutorial/` e `tutorial_completo/` (as 57 lições jogadas, 309 ticks 307/307), Load de `Tutorial.Ant`, janela de edição maximizada, moldura da lição 100% |

### 2.9 Salvar/carregar

- **Completo e verificado:** gravar `.ANT` (aceito pelo original); Load com `_RandYard` antes de ler (S e R batem); `_TimeTemp` = 300; `Tutorial.Ant` gera o tutorial; vista depois do Load (`_editWidth` = 25, 99,94%).
- **Implementado, não verificado:** Load Game pelo "Select a Game" (abertura e partida); preferências entre partidas (`preferencias.json`) e, **desde 2026-10-19**, o `[SimAnt]` do `WIN.INI` do usuário como origem inicial enquanto o arquivo do port não existe (`--win-ini`, ou `WINDOWS/WIN.INI` acima da pasta do jogo; na instalação do usuário, `SimAnt-T/WINDOWS/WIN.INI`; só leitura; `regras/som.md`, "Persistência"; `decisoes.md`).
- **Falta:** nada de jogo; o estado dos scripts de mod fica em `NOME.ANT.mods.json` (extensão do port).

### 2.10 Mods (dados + Lua)

- **Completo:** API 0.28; objetos do banco por (tipo, id), arquivos soltos, `data/*.json` **mesclados por chave** (`"_substituir": true` troca o objeto inteiro); tabelas comuns uma vez só em `comum.json`; scripts Lua em sandbox com ganchos antes/depois do tick e **dentro do tick** (em volta das 13 sub-rotinas de `_DoAntSim`, só quando algum mod os define), menu, painel, ações, escritas validadas (cheiro, castas, comportamento, formigas, comida) e estado dos scripts ao lado do save. Mods de exemplo em `exemplos/mods/` (`trapacas`, `exemplo-lua`). Testes em `mods.rs`, `mods_lua.rs` e `mod_trapacas.rs`.
- **"Planejado"** (`api-mods.md`, revisto nesta data): scripts acrescentarem itens à barra de menus; leitura das tabelas pelos scripts; fim do aceite das cópias antigas das tabelas comuns.
- **Risco:** os dados mudam de forma a cada rodada (0.20 a 0.28 em dois dias); a mesclagem por chave reduziu o efeito em mods antigos, e cada versão tem nota de compatibilidade.

### 2.11 Ferramentas e testes

- 44 ferramentas em `tools/` (43 Python e 1 PowerShell; desde 2026-10-19 também `comparar_baloes_menino.py` e os comandos `perfil`/`percussao` do `som_dosbox.py`: inspetores de formato, extratores de tabelas e `conferir_extracoes.py`, `cenarios_tick.py`/`cenarios_regras.py`/`bancada_mapa.py` da bancada QEMU com gdbstub em VGA e SVGA, comparadores de captura, medição de ritmo e de abertura, `som_original.py`, `som_dosbox.py` (captura do áudio do original no DOSBox-X e comparação), `gerar_banco_opl.py` e `ajustar_timbre_fm.py` (banco FM)), 74 roteiros, exemplos de `engine/examples/` (exportadores e comparadores).
- 618 testes; os de vetores exigem igualdade total; os de arquivos reais avisam e passam sem `SIMANT_DIR`.
- **Fraqueza:** capturas e `re_trabalho/` ficam fora do git (direitos autorais), então as porcentagens de tela só são reprodutíveis na máquina do usuário; o determinismo da bancada depende do snapshot (QEMU sem `icount`).

### 2.12 Documentação

- 46 specs (~8,8 mil linhas, parágrafos longos), `decisoes.md`, `glossario.md`, `divergencias.md` (33 em aberto, 3 extensões do port, 56 resolvidas, depois da varredura de 2026-10-19), `bancada-qemu.md`, `api-mods.md`. A convenção de evidência (verificado / visto no código / [hipótese]) é seguida; há 174 marcas `[hipótese` nas specs.
- **Desatualizado (achado nesta revisão, corrigido):** o "O que falta" de `spec/README.md`, a "Pendente" da aranha em `regras/inimigos.md`, o botão Yard em `telas/quintal.md`, a contagem de vetores em `tests/referencias/README.md` e o "Planejado" de `api-mods.md`. **Ainda desatualizados, sem correção nesta rodada:** trechos históricos de `testes-tick.md` (rodadas antigas, que contam os vetores daquela data), a lacuna "os de dentro do tick não sorteiam" de `telas/baloes.md` §4 (o desenho dentro do tick foi feito depois, `regras/rng.md`) e a direção dentro dos ninhos em `regras/feromonios.md` (a conferir).

### 2.13 Funções do original sem equivalente (estimativa pela tabela de símbolos)

Método (refeito em 2026-10-19): cada nome de função dos segmentos 1–7 de `re_trabalho/symbols.tsv` foi procurado como identificador inteiro em `engine/src`, `simant/src` e `data/` (código) e em `docs/` (specs). Estimativa grosseira: o port implementa várias funções R/B com uma só rotina, e muitas funções são infraestrutura trocada por Rust/SDL2.

| Módulo | Funções | No código | Só nas specs | Em lugar nenhum | Situação |
|---|---|---|---|---|---|
| `SIMANT_MODULE` (interface principal) | 165 | 101 | 28 | 36 | faltam de fato: nada alcançável pelo jogador; os 36 são infraestrutura (paleta, cursores, `_win_*Closed`, `_Dialog*Wait`), `_GotoRQueen` (só pela coluna de ícones da edição, invisível no Windows), `_NetBIOSPost` (rede) e `_DoDebugWin` (sem chamador: inalcançável, ver abaixo) |
| `ANTEDIT_MODULE` (edição, mapa, arquivos) | 145 | 75 | 21 | 49 | `_clip_*`, `_ed_*`, buffers e cursores do mapa (infraestrutura); `OPENDLG`/`SAVEASDLG` (o port imita os diálogos) |
| `SIMONE_MODULE` (regras) | 169 | 102 | 53 | 14 | variantes R implementadas por rotina comum e utilitários |
| `SIMANT1_MODULE` (regras) | 123 | 83 | 30 | 10 | os 10 são variantes R (`_DoRestR`, `_EatFoodR`…) implementadas por rotina comum |
| `SIMTWO_MODULE` (regras, banco, janelas) | 282 | 93 | 43 | 146 | `_db_*`/`_ch_*`/índice/pack do banco e `_win_*` de desenho (infraestrutura); `_goStep*`/`_Finish*Step` sem chamador |
| `GR_MODULE` (gráficos) | 181 | 28 | 35 | 118 | primitivas `G*`, conversões de bitmap (substituídas) |
| `_TEXT` (runtime C) | 254 | 15 | 8 | 231 | não se aplica |
| **Total** | **1.319** | **497** | **218** | **604** | |

`_DoDebugWin` (`1000:1cbe`): escreve a média de `GetTickCount` por chamada e as chamadas por minuto no objeto 0x2a8 da janela 28 (`debugWin`); **não tem chamador** no pseudocódigo (o Ghidra resolve chamadas e relocações) e nenhum menu abre a janela 28. Inalcançável pelo jogador: nada a implementar.

Conclusão: **na lógica do jogo e na interface alcançável não há grupo de funções sem equivalente.** O que fica sem equivalente é infraestrutura substituída, rede (NetBIOS, DDE) e código morto.

## 3. Não verificado contra o original (lista consolidada)

Formatos
- Gaia8x14 e fontes com `kernMax < 0` (`formatos/fonte.md`).
- Bits 4/8 das flags do índice e sufixo dos nomes de banco (`formatos/banco-dat-ndx.md`, [hipótese]).
- Classes do campo T e bits 0–2 nas crias (`formatos/save-ant.md`, [hipótese]).

Regras
- A lista 4/1100 por estágio (`regras/ciclo-de-vida.md`); amarela vermelha pondo ovo preto e recriação da cauda em lutas ([hipótese]).
- Defeitos do original marcados [hipótese] no ninho e coluna 106 de `_CarpetFloorR`.
- Giro duplo de `_GetNestDir` sem caso isolado (confirmado só de forma indireta, `regras/feromonios.md`).
- Seção 13 de `regras/formiga-amarela.md` além de Y-01..Y-04 e Y13; quadros do choque e da loucura com a janela de edição atrás.
- Retorno exato de `_CalcScore` e o sentido da nota 3 (`regras/simulacao-visao-geral.md`).
- `_KillAntLion(-1)`: o x do primeiro `_SetMap` é o byte alto do seletor de `_EggStr` (memória do Windows; o port só o tem quando o vetor o traz).

Geração de mundo
- Patch 8 da casa; `_DigOutBNest/RNest` num patch do quintal; `_AddFood` com parâmetro ≥ 0; o jogo novo escolhido durante a partida. (Os outros patches, `_CanBeHouseHole` e os ramos de Tutorial/Full/Experimental de `_InitYelloAnt`/`_RandWorld` foram conferidos em 2026-10-19.)

Jogo ao vivo
- Taxa de fast/ultra em máquina real; `mac_por_tick` fora da bancada. (A cadência sem foco foi medida em 2026-10-19: igual à com foco.)
- O que o original mostra depois de `_XferPatch` foi conferido em 2026-10-19 (a vista fica em `_MapPnt` = (64, 32); o port deixou de centralizar na amarela). Com o jogo pausado o original mantém o patch antigo na janela até o próximo redesenho; o port faz o mesmo desde 2026-10-19 (resta [hipótese]: a rolagem com o jogo pausado redesenhar com o patch novo).

Telas
- Cartões 127 e 129 do Info e clique em palavra em negrito.
- Telas maiores e perfil de driver: a VGA de 16 cores em 800×600 e 1024×768 (sem driver na bancada); Examine e mini mapa em 256 cores; as cores do perfil `svga256` além do título e do fundo (`divergencias.md`).
- Altura mínima da edição e clique comido na janela de trás (`telas/janelas.md` §8, [hipótese]); a fase do xadrez do item acinzentado realçado numa janela em x ímpar; a sombra dos menus das janelas filhas.
- Cores do mapa (`_ConvColor`, [hipótese]).
- Clique direito no Experimental e nas janelas de controle, F1 com janela de controle, Shift+F1, Delete, `yardHelpBut`, cursor das ferramentas (`telas/ajuda-contexto.md` §6).
- Posição do menu de ferramentas no Quick Game; avisos do Experimental na faixa. Faixa 35 aberta com o jogo pausado: o original deixa vazios os números do quintal e as barras de saúde até o próximo redesenho, o port os desenha (`divergencias.md`, 2026-10-19).
- A janela do mapa atrás logo depois de um Load (o original a desenha; o port a deixa preta na exportação, `divergencias.md`); ajuda dos itens de menu.

Som
- Músicas comparadas todas e o banco FM ajustado a elas (nível −0,1 dB em média; restam a percussão de `antthme1/2` na faixa de 2–4 kHz, a abertura um pouco clara e os programas 3, 25, 26, 31 e 69, sem notas isoladas para medir; o programa que a primeira nota herda numa partida [hipótese], `regras/som.md`). Sem captura: os efeitos dentro do jogo (taxa real na SB16), os sons da luta no tempo, a preempção no mesmo milissegundo e os sons da janela 26.

Diálogos
- Modalidade dos diálogos de arquivo em relação ao `WM_TIMER` ([hipótese]); rolagem com muitos arquivos e mudança de pasta.

## 4. Falta para concluir (priorizado)

Itens da revisão anterior que **foram feitos** e saíram desta lista (detalhes nas specs citadas em §2): barra de menus com About, Status, Quit e os sons de menu; Load Game no "Select a Game"; balões "Silly"; gráficos de saúde (não desenham no Windows: nada a fazer); quadros da janela 26 (`_YellowDialog`, `_SpiderDialog`, `_LionDialog`, `_DrawSimPayoff`); PLACE parando a simulação e com Shift; `_SpecialXfer` ao vivo; botões configuráveis da faixa 35; queda de comida na casa (`_DROPdir`); textos do exame e avisos do Experimental; redesenho alternado do mapa atrás; mini mapa; desenho da aranha e desenho dentro do tick; AutoTrack da aranha do jogador; recorte das retas (`_os_ClipLine`); tutorial maximizado; tela virtual 640/800/1024; ordem real das janelas nas animações da amarela; e, nesta rodada, `_ExitHole` com a lista cheia, `_KillAntLion(-1)`, o clique esquerdo no laço, o `WIN.INI` como origem das preferências, os bitmaps empacotados/planares e `_LastColonyPopR`. **Feito depois desta revisão (2026-10-19, branch dos itens de alto e médio impacto, API 0.32):** `_SpecialXfer` trocando de patch dentro do tick (ao vivo, o tick é refeito com a escolha; ação `special_xfer` 307/307); menus de um item do botão de ferramentas no Quick Game (janela 10) e no Jogo Completo (janela 11), 100% contra a bancada; repetição da rolagem no ritmo medido (~23 passos/s) e com a simulação parada, como o laço de `MAINWNDPROC`; descarte de efeito pedido no mesmo milissegundo; cores do sistema do perfil de driver além do título e do fundo (índices 18–20; título da edição em `svga256` 100% fora do texto); a cor da abertura (já resolvida pelo perfil); a posição do WinHelp no `WIN.INI` (é a da janela History, medida); contorno do arrasto em XOR; e, dos ajustes de pixel, 9100/9101 (a janela 26 só alinha na criação) e Status (o ajuste somava ao alinhamento) com causa achada; a faixa 35 sem números até o primeiro tick e o mapa atrás desenhado logo no Load (as duas divergências da verificação dos ramos raros). A janela de placar (Ctrl+T) é o Status e está feita. **Feito depois (2026-10-19, branch do título da janela principal, API 0.33):** barra de título e menu de sistema da janela principal (opcionais); realce e desenho do menu de sistema das janelas filhas (acinzentado, `GrayString`, caixa invertida); sombra dos menus; ritmo sem foco igual ao com foco (medido); e os ajustes de pixel do texto da faixa, do triângulo e dos níveis trocados pela regra do retângulo de `_win_Recalc` (direita e base − 1). `_DoDebugWin` é inalcançável (§2.13). **Feito depois (2026-10-19, API 0.34):** o banco FM ajustado contra as 28 músicas do original (curva de velocidade, perfis, envelopes, nível por programa e percussão; `regras/som.md`); 12 patches da casa e o jogo novo de Full, Experimental e Tutorial conferidos (307/307, com cinco correções no estado do jogo novo); os 23 balões do menino conferidos (posição, chuva por cima, caractere ausente); as barras da janela principal na abertura e no "Select a Game"; e o patch antigo na edição com o jogo pausado.

Esforço relativo: **P** = até um dia de trabalho focado; **M** = alguns dias; **G** = uma semana ou mais.

### Alto impacto para quem joga

| Item | Esforço | Referência |
|---|---|---|
| ~~Timbre FM: programas que ainda diferiam e nível por programa~~ (**feito em 2026-10-19**: −0,1 dB em média, espectro 0,78, envelope 0,94). Resto, de impacto menor: percussão de `antthme1/2` (2–4 kHz), a abertura um pouco clara, os programas 3, 25, 26, 31 e 69 sem notas isoladas | P | `regras/som.md` "Ajuste do banco FM contra as 28 músicas" |

Não há outro item de alto impacto: tudo o que o jogador alcança no original existe no port (a tela virtual e o perfil de driver entraram no `main` em 2026-10-19).

### Médio impacto

| Item | Esforço | Referência |
|---|---|---|
| Ajustes de pixel que restam: gato 1 pixel abaixo nos quadros 0–12 (descartados cabeçalho, tabelas, largura e altura); o +1 com faixa de `_win_GetObjRect`/`_win_PrintfAtObj` que o pseudocódigo soma e nenhuma medição mostra (o texto da faixa, o triângulo e os níveis seguem a regra do `_win_Recalc` desde 2026-10-19); a base dos textos do fim de jogo e o deslocamento dos gráficos do History | M | `divergencias.md`, `formatos/janela.md` |
| Menu aberto em 256 cores (realce pelo `OEMBIN`, sem captura) e a cor do sistema que muda com a paleta do jogo em 256 cores | P | `telas/resolucao.md` §7 |

### Baixo impacto

| Item | Esforço | Referência |
|---|---|---|
| Diálogos internos do WinHelp (Open, Print, Annotate, Define, About…) | M | `telas/ajuda.md` |
| Byte 3 dos bitmaps, bits 4/8 das flags, layout binário tipo 0 (só se algum leitor aparecer) | P–M | `formatos/bitmap.md`, `formatos/banco-dat-ndx.md` |
| `_SetMap` de `_KillAntLion(-1)` e as gravações em ponteiros do Windows (decidido não reproduzir) | — | `regras/inimigos.md` §1.4 |
| Rede (NetBIOS, DDE) | G | fora do escopo declarado |

### Trabalho de verificação (não muda o jogo, mas fecha a conta)

| Item | Esforço |
|---|---|
| Captura dos efeitos dentro do jogo (as músicas já estão: `som_dosbox.py musicas`); falta uma entrada que chegue ao Windows sem o foco (mouse serial pelo `nullmodem` do DOSBox-X, não tentado) | M |
| Capturas: telas maiores, avisos do Experimental | P |
| Patch 8 da casa, `_DigOut*` no quintal, `_AddFood(≥ 0)` e o jogo novo pela partida (a geração do jogo novo dos três modos foi conferida em 2026-10-19) | P |
| Ritmo fast/ultra numa máquina real | P |

## 5. Riscos e dívidas técnicas

1. ~~Tabelas duplicadas nos JSON~~ (resolvido em 2026-10-18, `comum.json`, API 0.20).
2. **Ajustes de pixel medidos sem causa conhecida** ([causa em aberto] em `divergencias.md`): gato 1 pixel abaixo nos quadros 0–12; a ausência do +1 com faixa de `_win_GetObjRect` (desde 2026-10-19 o texto da faixa, o triângulo, os níveis, os números da faixa e o Status usam uma regra só, o retângulo de `_win_Recalc`, `Retangulo::do_recalc`). (9100/9101 e Status saíram em 2026-10-19: a janela 26 só alinha ao byte na criação, e o ajuste do Status somava ao alinhamento já aplicado.) Vários dos antigos foram explicados em 2026-10-19 pelo **perfil de driver** (o `CS_BYTEALIGNCLIENT` arredonda a área cliente a 8 pixels só no VGA planar: título +3, edição, janelas de controle, fim de jogo, Info, About, Examine, mini mapa) e viraram regra do driver em `data/resolucao.json`; o que sobra pode ter a mesma natureza e deve ser revisto com o perfil.
3. **Hipóteses no relógio.** O relógio do Mac ao vivo começa no maior prazo do save; `mac_por_tick` = 9 só vale na bancada; sem relógio, `_GameTime × 6` [hipótese]. O gato, o cachorro e o balão do menino dependem disso, e com o gato e o cachorro **o consumo de sorteios** também: uma partida ao vivo não reproduz exatamente uma do original.
4. **Ritmo de fast/ultra dependente da CPU no original.** O port fixa 5 e 7 voltas por período. `_BirdDelay` depende de `_gameCycles` e é a única entrada não determinística do tick.
5. **Desenho que consome o gerador S fora do tick** (chuva e enxame do quintal): o port faz os sorteios à parte, entre os ticks (`full_600` bate com os 17.650 passos), mas as posições desenhadas da chuva são próprias. Os sorteios dentro do tick foram resolvidos em 2026-10-18/19.
6. **Load ressemeia S**, então capturas do original e do port do mesmo estado têm direções diferentes nas animações.
7. **Bancada QEMU TCG** é lenta e sem placa de som utilizável; ritmo, relógio e som medidos ali podem não valer numa máquina real. O determinismo depende do snapshot.
8. **Evidência fora do git.** Capturas, `re_trabalho/` e pseudocódigo não são versionados (direitos autorais); as porcentagens de tela não podem ser refeitas por terceiros sem o jogo e a bancada.
9. **Ritmo de mudança da API de mods** (0.20 a 0.28 em dois dias). A mesclagem por chave (0.26) protege mods de dados; scripts dependem de nomes de campos que ainda mudam.
10. **Comentários com `[hipótese]`, "ainda não" ou "[lacuna]" no código** (`engine/src`, `simant/src`): 55 linhas nesta revisão; nenhum `todo!` ou `unimplemented!`. Não foram reconferidos um a um nesta rodada.
11. **Perfil de driver.** (O áudio — captura no DOSBox-X e timbre FM — entrou em 2026-10-19.) O perfil padrão é `vga` (a bancada de referência); quem compara com o original no DOSBox-X de 256 cores deve usar `svga256`/`s3_dosbox`, e as capturas antigas feitas lá têm de ser lidas com esse perfil.
12. ~~Ordem das janelas fixa nas animações da amarela~~ (resolvido em 2026-10-18: a interface grava a ordem real). ~~Cauda da amarela rainha na reconstrução da vida~~ (feito em 2026-10-18, branch dos balões).

## 6. Porcentagem geral estimada

Pesos por área, pensados no que importa para quem joga um port completo (as regras pesam mais; documentação e ferramentas pesam pouco). Mesmos pesos da revisão anterior.

| Área | Peso | Implementado | Peso × impl. | Verificado | Peso × verif. |
|---|---|---|---|---|---|
| Formatos de arquivo | 8 | 95 | 760 | 85 | 680 |
| Regras da simulação | 25 | 98 | 2.450 | 84 | 2.100 |
| Geração de mundo | 7 | 97 | 679 | 92 | 644 |
| Jogo ao vivo | 8 | 96 | 768 | 68 | 544 |
| Telas e janelas | 15 | 96 | 1.440 | 86 | 1.290 |
| Diálogos e menus | 8 | 95 | 760 | 88 | 704 |
| Som e música | 6 | 88 | 528 | 72 | 432 |
| Modos | 8 | 94 | 752 | 86 | 688 |
| Salvar/carregar | 4 | 98 | 392 | 90 | 360 |
| Mods | 5 | 92 | 460 | n/a | — |
| Ferramentas e testes | 3 | 92 | 276 | n/a | — |
| Documentação | 3 | 92 | 276 | n/a | — |
| **Total** | **100** | | **9.541** | | **7.442** (peso 89) |

- **Implementado:** 9.541 / 100 = **≈ 95%** (9.499 antes da rodada do timbre FM e das verificações de 2026-10-19; 9.483 antes da barra de título, 2026-10-19; era 8.775, ≈ 88%; 9.359 antes do caminho FM; 9.389 antes dos itens de alto e médio impacto de 2026-10-19).
- **Verificado contra o original** (só as áreas 1–9, peso 89): 7.442 / 89 = **≈ 84%** (7.264 antes da rodada do timbre FM e das verificações de 2026-10-19; 7.083 antes da oitava rodada de verificação; 7.067 antes da barra de título; era 6.220, ≈ 70%; 6.844 antes da captura de áudio; 6.964 antes dos itens de 2026-10-19).

Leitura: a simulação está praticamente completa e amarrada ao original por vetores de tick inteiros; a interface alcançável pelo jogador existe inteira e foi comparada com capturas na maior parte. O que separa o port de "completo" é o som (timbre FM aproximado, agora ajustado às 28 músicas; os efeitos dentro do jogo sem captura); o resto são resíduos pequenos (§4). O que separa de "verificado" são as hipóteses que restam nas regras (§3), os efeitos sonoros dentro do jogo e as poucas janelas sem captura nas resoluções maiores e em 256 cores.
