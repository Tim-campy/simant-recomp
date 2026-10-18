# Estado do projeto (revisão geral, 2026-10-18)

Revisão feita sobre o `main` em `1ff4b82`, sem mudar código. Fontes: todas as specs de `docs/spec/`, `docs/divergencias.md`, `docs/api-mods.md`, `docs/spec/regras/testes-tick.md`, `tests/referencias/README.md`, `README.md`, comentários de `engine/src` e `simant/src`, e a tabela de símbolos `re_trabalho/symbols.tsv` (2.171 nomes do `SIMANTW.SYM`).

**Atualização de 2026-10-18 (tabelas comuns e documentação):** depois da revisão, as tabelas repetidas foram juntadas em `data/comum.json` (API de mods 0.20), `jogo.json` ficou sobrescrevível, a documentação desatualizada listada em §5 foi corrigida e os comentários do código conferidos. Mudaram as linhas 7, 10 e 12 da tabela de §1 e as seções 2.2, 2.5, 2.6, 2.7, 2.10, 2.11, 2.12, 4 e 5; o resto continua sendo a revisão sobre `1ff4b82`.

**Medições desta revisão:**

- `cargo test --workspace` com `SIMANT_DIR` apontando para `SimAnt-T/MAXIS/ANTWIN`: **518 testes passaram, 0 falharam, 0 ignorados** em 35 binários de teste (unitários de `engine` e `simant` e os testes de integração de `engine/tests/`).
- Vetores do original no repositório (`tests/referencias/`):
  - **107 vetores de tick** em 21 grupos (`ticks/`), todos exigidos 307/307 blocos (com extras e vida) por `engine/tests/ticks_originais.rs`; nenhum em `"pendentes"` (o último, `inseticida_morte/21`, foi resolvido pela interface). **Atualização 2026-10-18 (quinta rodada):** +38 vetores em 25 cenários (cerco, laser, faro, aranha do jogador, mortes da amarela, N-01/07/10/12/14/15, C-04..C-07), 37 exigidos 307/307 e 1 pendente com a causa isolada (`amarela_morte_aranha`, desenho dentro do tick);
  - **2 sequências longas** (`ticks_longos/rico_200`, 200 ticks; `ticks_longos/full_600`, 600 ticks do Jogo Completo com a Yard View na frente), iguais ao original;
  - **15 ações da interface** (`acoes/`: troca de patch, PLACE, `_SpecialXfer`, Experimental, desenho da chuva e do enxame, Mystery), todas 307/307;
  - **1 Load do tutorial** (`load_tutorial/`);
  - **21 casos de regra** da bancada (`casos/*.json`): 20 confirmados no original e iguais no port; C-10 não conferível (mensagem modal no load).
- Cobertura por nome das funções do original (heurística, ver §2.13): das 1.319 funções dos segmentos de código 1–7, **452 (34%) são citadas pelo nome no código ou nos dados do port** e 663 (50%) no código ou nas specs. Nos três módulos de regras (`SIMONE`, `SIMANT1`, `SIMTWO`), 252 de 433 (58%) estão no código e só 71 não aparecem em lugar nenhum.

## 1. Resumo

### Critério

- **Implementado:** o port faz o que o original faz naquela área, ligado ao jogo (não só na engine), a partir da spec. Funções do original substituídas por equivalentes modernos (gerência de memória, `_win_*`, primitivas `G*`, runtime C) contam como feitas quando o efeito visível está reproduzido.
- **Verificado contra o original:** há evidência de comparação com o SimAnt rodando (vetor de tick 307/307, caso da bancada, captura comparada pixel a pixel, medição de ritmo) para aquela parte. "Visto no código" (pseudocódigo do Ghidra) **não** conta como verificado. Por isso o "verificado" é sempre ≤ "implementado".
- As porcentagens são estimativas do revisor, feitas a partir das lacunas listadas nas specs e no código. Não são medidas automáticas. Onde a área é extensão do port (mods, ferramentas, documentação), não existe "contra o original" e a coluna fica "n/a".

### Tabela por área

| # | Área | Implementado | Verificado no original | Comentário curto |
|---|---|---|---|---|
| 1 | Formatos de arquivo | 90% | 85% | banco NDX/DAT + LZSS, bitmaps, fontes, janelas, `SIMANT.CFG`, `.ANT` (byte a byte), sons, tiles, `.HLP`; restam bitmaps ~7100, `kind`/byte 3, bits 4/8 de flags |
| 2 | Regras da simulação (tick) | 95% | 75% | 107 vetores + 2 sequências + 20 casos batem; desde 2026-10-18 também cerco, laser, faro, aranha do jogador, mortes da amarela e os casos estatísticos (+38 vetores, 1 pendente) |
| 3 | Geração de mundo | 95% | 75% | Quick Game byte a byte; casa em 4 patches; Tutorial via Load; ramos de Full/Experimental e vários patches da casa sem caso |
| 4 | Jogo ao vivo (laço, relógio, ritmo) | 90% | 65% | slow/normal medidos (3,0/6,0 ticks/s); fast/ultra e relógio do Mac só na bancada QEMU e com decisões do port |
| 5 | Telas e janelas | 85% | 70% | quase todas as janelas existem e foram comparadas (93–100%); faltam balões, gráficos de saúde da edição, partes da ajuda |
| 6 | Diálogos e menus | 60% | 50% | **sem barra de menus do Windows** (File/Window/View/Options/Speed); "Load Game" do "Select a Game" não existe |
| 7 | Som e música | 75% | 35% | efeitos e músicas das regras e do Experimental tocam; sons de menus e diálogos não; timbre diferente; nada comparado de ouvido |
| 8 | Modos (Quick, Completo, Experimental, Tutorial) | 90% | 75% | os quatro existem e têm vetores; tutorial é o menos conferido |
| 9 | Salvar/carregar | 95% | 90% | `.ANT` aceito pelo original; Load com `_RandYard` conferido; `Tutorial.Ant` gerado |
| 10 | Mods (dados + Lua) | 90% | n/a | API 0.20, 37 JSON sobrescrevíveis (todos, inclusive `jogo.json`), tabelas comuns uma vez só em `comum.json`, Lua em sandbox, seção "Planejado"; testes em `mods.rs`, `mods_lua.rs` e `mod_trapacas.rs` |
| 11 | Ferramentas e testes | 90% | n/a | 30 ferramentas em `tools/`, bancada QEMU com gdbstub, 518 testes |
| 12 | Documentação | 90% | n/a | 42 specs (~8,8 mil linhas); trechos desatualizados de §5 corrigidos em 2026-10-18 |

**Geral (conta em §6): cerca de 88% implementado e 70% verificado contra o original** (este último só nas áreas 1–9).

## 2. Por área

### 2.1 Formatos de arquivo

- **Completo e verificado:** banco `.NDX/.DAT` com LZSS (todas as telas e bitmaps saem dele); bitmaps e paleta (paleta do Windows, com a troca 7/8 corrigida; título 99,98%, "Select a Game" 99,87%); fontes do Mac e Gaia (blocos idênticos na captura do registro, `fontes_mac_batem_com_o_original`); layouts de janela (tipos 0/1); save `.ANT` (48.386 bytes, 307 blocos, memória = arquivo, gravado pelo port e aceito pelo original); tiles (ninho preto do `QUICK2.ANT` 100% na área do mapa); sons tipo 5/51/18/20/50 (decodificados; o formato confere com o que o original toca); `SIMANT.HLP` (WinHelp 3.0, 99,94–99,97% nas capturas da ajuda).
- **Implementado, não verificado:** Gaia8x14 e fontes com `kernMax < 0` (`formatos/fonte.md`); significado dos bits 4 e 8 do índice (`banco-dat-ndx.md`, [hipótese]); origem do sufixo dos nomes de banco (`banco-dat-ndx.md`, [hipótese]); classes do campo T acima de 2 (`save-ant.md`, [hipótese]).
- **Parcial:** objeto cuja estrutura "ainda não decodificada" (`banco-dat-ndx.md` l. 80); `_PatchColorArrays`/`_SetUpPalette` não analisadas (`bitmap.md`).
- **Falta:** bitmaps estreitos perto de 7100 no `WINGANT` (saem com faixas; `bitmap.md`, "Em aberto"); `kind` e byte 3 do cabeçalho de bitmap.

### 2.2 Regras da simulação (tick)

- **Completo e verificado:** ordem do tick (`_DoAntSim`), geradores S e R, superfície (lista A, feromônios, luta, comida, entrada no ninho, líder vermelha), ninhos preto e vermelho (escavação, comida, saída, invasoras, água), ciclo de vida (ovos, rainha, castas), inimigos (formiga-leão, aranha, lagarta, bicho-de-conta), formiga amarela (andar, contorno, comer, ninho, luta com derrota e vitória), quintal (menino, cortador, inseticida, gato, cachorro, pássaro, chuva, enxames, `_SimColonies`), conselhos e tutorial no tick, trapaças (`heal`, `just`, `qeen` etc. conferidas). Evidência: 107 vetores, 2 sequências longas, 20 casos da bancada.
- **Implementado, não verificado:** ver a lista consolidada em §3 (morte natural, proporção de vitórias, revoada natural, estratégia vermelha, recrutamento, cerco, laser, faro, aranha do jogador, morte da amarela por inimigos, casos estatísticos dos ovos e do ninho, defeitos do original marcados [hipótese]).
- **Parcial:** `_SpecialXfer` termina fora do tick (o original troca de patch dentro do passo 17; S e R batem assim mesmo, `divergencias.md`), e **só nos vetores**: na partida ao vivo a interface não reage à mensagem 10058 (a morte da amarela no Jogo Completo com outra colônia preta só mostra a mensagem; achado de 2026-10-18, comentário em `rules/amarela.rs`); `_DropFoodA` (comida que cai na casa some porque `_DROPdir` não está no `Estado`, `geracao-mundo.md` l. 294); `_ExitHole` com a lista A cheia (vaga 1000 no original; `ninho.md`); `_KillAntLion(-1)` no inseticida (o original grava fora da tabela; o port ignora).
- **Falta:** modos 17/18 e grupos 10/11/15 sem significado conhecido (`formigas.md`); `_RevSpider` sem chamador visto (`inimigos.md`); chamadas indiretas de `_ForceModeA/B`, `_StartAttack`, `_GetNewRedTask` não procuradas (`superficie.md`).

### 2.3 Geração de mundo

- **Completo e verificado:** `_RandYard`/`_RandWorld` do Quick Game (`_MapB`/`_MapR` idênticos, `_MapA` com 10 diferenças, todas de bichos que andaram; `engine/tests/geracao_quick2.rs`); casa em quatro patches pelo `_XferPatch`; `_InitPillar` corrigido; `_GenerateTutorial` no Load de `Tutorial.Ant` (`load_tutorial/`); jogo novo depois do fim de jogo (`inseticida_morte/21`).
- **Implementado, não verificado** (`geracao-mundo.md` l. 106, 300, 339): `_CanBeHouseHole` na casa; patches 5–7, 9, 15, 36/37 e cozinha sem pia; `_DigOutBNest/RNest`; ramos de Tutorial, Full e Experimental de `_InitYelloAnt`/`_RandWorld`; `_AddFood` com parâmetro ≥ 0; `_DROPdir`.
- **Falta:** guardar `_DROPdir` no estado (liga com a queda de comida na casa).

### 2.4 Jogo ao vivo (laço, relógio, ritmo)

- **Completo e verificado:** temporizador de 55 ms com 3 mensagens por chamada; 6,06 ticks/s em normal e 3,03 em slow contra 6,0/3,0 na bancada; pausa (inclusive a amarela andando durante a pausa); simulação parada com quadro modal e nas animações da luta; `_gameCycles` por volta do laço.
- **Implementado com decisão do port, não verificado em máquina real:** fast/ultra com 5 e 7 voltas fixas (66,7 e 90,9 ticks/s; bancada QEMU TCG deu 33–66 e 32–91; com `-icount` o original faz ~3 ticks/s em fast por milhão de instruções/s — 26 a 7,8 M, 105 a 31 M, 186 a 62,5 M —, então os valores do port equivalem a uma CPU de ~20–30 M instruções/s, um 486 da época; `simulacao-visao-geral.md` §3.3); relógio do Mac começando no maior prazo do save; `mac_por_tick` (9 na bancada); cadência sem foco 170 → 220 ms [hipótese] (`simulacao-visao-geral.md`).
- **Parcial:** o laço não sai com o botão esquerdo apertado (`GetAsyncKeyState(VK_LBUTTON)`, `divergencias.md`); a repetição da rolagem usa a do sistema; descarte de efeito no mesmo milissegundo não reproduzido.

### 2.5 Telas e janelas (por janela)

| Janela | Implementado | Verificado | Evidência / o que falta |
|---|---|---|---|
| Abertura e título | 100% | 90% | 99,98%; duração (~39,6 s) e Espaço pendentes (`tests/referencias/README.md`) |
| "Select a Game" | 90% | 90% | 99,87%; falta o Load Game |
| Tela de jogo: faixa 34 | 90% | 85% | 93,27% numa captura antiga (antes das barras e botões); botões, barras e popup das castas conferidos depois |
| Tela de jogo: janela de edição | 85% | 75% | mapa 100%, janela 99,33–99,59%; faltam balões ("Silly", luta, descanso, ovos, rainha: `_DrawBalloons`), gráficos de saúde (`_DrawEditGraphs`), a 2ª metade de alguns bichos (tabela não localizada, `jogo.md` l. 83); superfície, ninho vermelho e rainha amarela (0xFE) não comparados |
| Gerenciador de janelas | 85% | 70% | ordem, foco, arrasto, maximizar, menu de sistema; contorno XOR e menu de sistema aproximados; altura mínima [hipótese] |
| Caste / Behavior / History | 95% | 80% | 98,72–98,92% (só o título); arrasto no triângulo e gráficos com dados não conferidos |
| Menus de segurar (amarela, ícones) | 100% | 80% | `_AntMenu` 100%, ícones 99,93%; menus da rainha e da aranha não conferidos |
| Mapa (`mapWin`) | 90% | 75% | 93,78–99,89% por modo; `_ELayerMode` da edição e `_win_Swap` ligados; faltam Shift+clique dar ordem e o redesenho alternado atrás (o port gera a cada tick); cores de `_ConvColor` [hipótese] |
| Quintal (Yard View, faixa 35) | 90% | 85% | 99,27–99,59% nos três modos, faixa 93,09%; botões configuráveis da faixa 35 não existem |
| Info (cartões) | 90% | 70% | 11 de 53 cartões contra capturas (96,36–99,75%); os outros 42 só por conferência automática |
| Examine (`_MagnifyMenu`) | 95% | 90% | conferido na bancada (2026-10-18): 97,6–99,4% na região da janela, inclusive o deslocamento de 3 pixels da primeira abertura da sessão |
| Ajuda (visor do WinHelp) | 75% | 70% | 99,94–99,97%; Search, History, menus e popups pequenos não existem; desenhada na área cliente, 2 linhas acima |
| Diálogos de arquivo | 85% | 85% | 99,28–99,97% nas partes comparáveis; é uma imitação (unidade fixa "c:", "Read Only" sem efeito) |
| Menu de ferramentas do Experimental | 100% | 100% | 100% na bancada |
| Quadros de mensagem e faixa | 100% | 95% | 100% nas capturas; texto 1 pixel à esquerda por ajuste medido |
| Fim de jogo e pontuação | 95% | 90% | 97,95%; pontuação 19785 do C-10 e 27572 do `inseticida_morte` iguais; `_DrawSimPayoff` (animação) não existe |

### 2.6 Diálogos e menus

- **Completo e verificado:** quadros de mensagem (`_PictStrnDialog`), fim de jogo, "Saved correctly", pergunta do Load, Save As/Load (imitação), menus de segurar, menu de ícones, menu de ferramentas do Experimental, aceleradores do original (Speed, AutoTrack, Save, Save As, janelas, planos, quintal).
- **Falta (alto impacto):**
  - **A barra de menus do Windows** (File, Window, View, Options, Speed): o port não tem (`data/partida.json`: "o port não tem barra de menus"); tudo depende de aceleradores e dos botões configuráveis da faixa. Sem ela, quem joga não descobre Options (Music, Sound Effects, Events, Messages, Silly), Quit, About, a janela de placar (Ctrl+T, `0xFD17`) nem Ctrl+J/K.
  - "Load Game" no "Select a Game", na abertura e durante a partida (`simant/src/modos.rs`: imprime "Load Game ainda não implementado no port"; o Ctrl+L da partida existe). A linha de `divergencias.md` que dizia o contrário foi corrigida em 2026-10-18.
  - `_YellowDialog` da trofalaxia (`rules/amarela.rs` l. 1212, "[lacuna]"); `_AboutDialog`, `_ScoreDialog` sem equivalente.
- **Implementado, não verificado:** diálogos de arquivo modais (a simulação não anda) [hipótese, `telas/arquivos.md`]; primeiro clique numa janela de trás comido também no título [hipótese].

### 2.7 Som e música

- **Completo:** 57 efeitos decodificados (8 bits, 4096 Hz); músicas `.MID` com a tabela MCI; opções Music/Sound Effects; silêncio sem foco em todas as telas; linha do tempo dos sons da luta (165 ms).
- **Verificado:** só o formato e o catálogo (quem pede cada som, pelo código). **Nada foi comparado de ouvido com o original.** Tentado em 2026-10-18 na bancada QEMU com SB16 + OPL2 e saída em WAV (`tools/som_original.py`): grava, mas não serve de referência — o driver FM é de OPL3 (notas presas no OPL2 do QEMU), o backend descarta os silêncios e os efeitos gravados não batem com os decodificados (`regras/som.md`, "Áudio do original na bancada QEMU").
- **Parcial / decisão do port:** timbre por `rustysynth` + SoundFont ou sintetizador simples, não o OPL/MIDI Mapper; volume relativo efeito/música não medido; efeito novo sempre corta o anterior.
- **Falta:** sons de menus e diálogos (`_LionDialog`, `_YellowDialog`, `_SpiderDialog`, `_DrawSimPayoff`, itens da barra de menus; `regras/som.md`, "Interface: o que está ligado e o que falta"). Os das ferramentas do Experimental, dos marcadores, das trapaças, do Mystery, do fim de jogo e da troca de patch tocam.

### 2.8 Modos

| Modo | Implementado | Verificado | Notas |
|---|---|---|---|
| Quick Game | 95% | 90% | quase todos os vetores saem do snapshot `quick`/`rico` |
| Jogo Completo | 90% | 80% | `jogo_completo/`, `full_600`, troca de patch, PLACE, `_SpecialXfer`, 38 patches da casa; PLACE não para a simulação nem aceita Shift |
| Experimental | 90% | 85% | ações `exp_*` 307/307; textos do quadro de exame e avisos das listas 4/1010, 4/1100–1103 não desenhados (`telas/experimental.md` §10) |
| Tutorial | 80% | 50% | vetores `tutorial/` (lições 1–3) e Load de `Tutorial.Ant`; as demais lições e as condições de janela só pelo código |

### 2.9 Salvar/carregar

- **Completo e verificado:** gravar `.ANT` (aceito pelo original); Load com `_RandYard` antes de ler (S e R batem com `qemu_load_rng.txt`), `_TimeTemp` = 300; `Tutorial.Ant` gera o tutorial; vista depois do Load (`_editWidth` = 25, 99,94%); preferências entre partidas (`preferencias.json` no lugar do `WIN.INI`).
- **Falta:** Load pelo "Select a Game" (§2.6); o port não lê o `WIN.INI` do usuário.

### 2.10 Mods (dados + Lua)

- **Completo:** API 0.20; objetos do banco por (tipo, id), arquivos soltos, 37 `data/*.json` (todos sobrescrevíveis; `jogo.json` desde a 0.20, lido com os mods antes de abrir os bancos); as tabelas das regras chegam ao tick (inclusive `amarela.json`), e as 14 que o original tem uma vez só ficam só em `comum.json` (um mod antigo com a cópia numa parte continua valendo para aquela parte, com aviso); scripts Lua com ganchos, menu, painel, ações e avanço de ticks, em sandbox; mod de trapaças e de chuva de comida em `exemplos/`. Testes: `mods.rs` (5, inclusive `comum.json` e a cópia antiga) e `mods_lua.rs` (22), mais `mod_trapacas`.
- **"Planejado":** seção nova em `docs/api-mods.md` (2026-10-18): sobrescrita parcial por chave, botões configuráveis da faixa 35, itens da barra de menus vindos de dados, leitura das tabelas pelos scripts e o fim do aceite das cópias antigas das tabelas comuns.
- **Risco:** mudanças de formato quebram mods (ex.: 0.7 e 0.13 exigiram chaves novas), porque cada arquivo é trocado inteiro (a sobrescrita por chave está em "Planejado"). O histórico está em ordem desde 2026-10-18.

### 2.11 Ferramentas e testes

- 32 ferramentas em `tools/` (inspetores de formato, extratores de tabelas — `extrair_comum.py` e `conferir_extracoes.py`, que confere que os extratores reproduzem os `data/*.json`, desde 2026-10-18 —, `cenarios_tick.py`, `caso_ant.py`, `comparar_captura(s).py`, bancada QEMU com gdbstub), 51 roteiros em `tests/referencias/roteiros/`.
- 518 testes; o de vetores exige igualdade total; os de arquivos reais avisam e passam sem `SIMANT_DIR`.
- **Fraqueza:** capturas e `re_trabalho/` ficam fora do git (direitos autorais), então as porcentagens de tela só são reprodutíveis na máquina do usuário; o determinismo da bancada depende do snapshot (QEMU sem `icount`).

### 2.12 Documentação

- 42 specs, `decisoes.md`, `glossario.md`, `divergencias.md` (67 linhas desde 2026-10-18, em três seções: em aberto, extensões do port, resolvidas), `bancada-qemu.md`, `api-mods.md`. A convenção de evidência (verificado / visto no código / [hipótese]) é seguida; há 154 marcas `[hipótese` (com ou sem complemento) nas specs.
- **Desatualizado:** os documentos listados em §5 item 9 foram corrigidos em 2026-10-18 (README reescrito como guia enxuto; specs conferidas contra o código).

### 2.13 Funções do original sem equivalente (estimativa pela tabela de símbolos)

Método: cada nome de função dos segmentos 1–7 de `re_trabalho/symbols.tsv` foi procurado como identificador em `engine/src`, `simant/src` e `data/` (código) e em `docs/` (specs). É uma estimativa grosseira: o port implementa várias funções R/B com uma só rotina (por exemplo `_RaidOutR`, `_DoDrownR`, `_QueenMoveR` não aparecem pelo nome mas o ninho vermelho bate nos vetores), e muitas funções são infraestrutura trocada por Rust/SDL2.

| Grupo | Funções | Citadas no código | Só nas specs | Em lugar nenhum | Situação |
|---|---|---|---|---|---|
| `_win_*` (sistema de janelas próprio) | 123 | 21 | 12 | 90 | substituído pelo gerenciador do port; não precisa de equivalente 1:1 |
| `_db_`/`_DB*`/`_ch_` (banco, cache) | 28 | 1 | 3 | 24 | substituído pelo leitor do banco |
| `_mem_*`/`_Ralloc*`, `_hanim_*`, `G*` | 77 | 5 | 8 | 64 | infraestrutura (memória, animação, primitivas gráficas) |
| `_Draw*` | 34 | 22 | 7 | 5 | sem equivalente: `_DrawBalloons`, `_DrawEditGraphs`, `_DrawSimPayoff`, `_DrawLegs`, `_DrawForSale`, `_DrawMapSpider`, `_DrawMapFoot`, `_DrawMiniMapCursor` |
| Rede (NetBIOS, DDE, `_ProcessPost`) | 5 | 0 | 3 | 2 | fora do escopo (trapaças `data`, `serv`, `clie` só escrevem no console) |
| Som/MIDI | 21 | 6 | 6 | 9 | resto são caminhos Sound Blaster/alto-falante/Disney, não reproduzidos |
| Impressão | 0 | — | — | — | o `.SYM` não tem funções de impressão |
| Runtime C (`_TEXT`) | 256 | 10 | 8 | 238 | não se aplica |
| `SIMANT_MODULE` (resto) | 146 | 82 | 23 | 41 | faltam `_AboutDialog`, `_ScoreDialog`, `_Quit`/`_MenuQuit`, `_ProcMenuHelp`, `_CheckItem` (barra de menus), `_GotoQueen`/`_GotoSpider`, `_DoDebugWin` |
| `ANTEDIT_MODULE` (resto) | 112 | 43 | 13 | 56 | balões (`_EggBalloons`, `_QueenBalloons`, `_AddMsgBalloon`), minimapa (`_Mini_*`, `_OpenMiniMapWin` [em aberto: talvez sem uso no Windows]), `_clip_*`/`_ed_*` (infraestrutura) |
| `SIMONE_MODULE` (resto) | 169 | 101 | 53 | 15 | quase tudo coberto; faltantes são variantes R ou utilitários |
| `SIMANT1_MODULE` (resto) | 123 | 83 | 27 | 13 | os 13 são variantes R implementadas por rotina comum |
| `SIMTWO_MODULE` (resto) | 141 | 68 | 30 | 43 | `_goStep*`/`_Finish*Step` (sem chamador, provável resto do Mac), índice/pack do banco (infraestrutura) |

Conclusão: **na lógica do jogo, não há grupo grande de funções sem equivalente**. O que falta de fato é interface: barra de menus e seus diálogos, balões, gráficos de saúde da edição, `_DrawSimPayoff`, placar, About, minimapa (se for usado) e rede.

## 3. Não verificado contra o original (lista consolidada)

Formatos
- Gaia8x14 e fontes com `kernMax < 0` (`formatos/fonte.md`).
- Bits 4/8 das flags do índice e sufixo dos nomes de banco (`formatos/banco-dat-ndx.md`, [hipótese]).
- Classes do campo T e bits 0–2 nas crias (`formatos/save-ant.md`, [hipótese]).

Regras
- A lista 4/1100 por estágio (`regras/ciclo-de-vida.md`, "Lacunas"). (C-04 a C-07 verificados em 2026-10-18, vetores `ovos_*`.)
- Amarela vermelha pondo ovo preto e recriação da cauda em lutas (`regras/ciclo-de-vida.md`, [hipótese]).
- Morte natural (T1), luta e proporção de vitórias (T5, T6), revoada natural (T8), estratégia vermelha (T9), recrutamento pelo jogador (T11) (`regras/superficie.md`).
- Pedrinha cuspida pela formiga-leão (`regras/inimigos.md`). (Cerco da aranha e da formiga-leão, fuga, vingança, faro, laser e aranha do jogador verificados em 2026-10-18, testes-tick.md "Quinta rodada".)
- Desenho dentro do tick (`_DrawPalps` e `_SpiderDialog` sorteiam S no meio do tick; medido, não implementado): deixa pendente `amarela_morte_aranha/1793`, a morte da amarela pela aranha (`regras/rng.md`, `regras/inimigos.md`).
- Afogamento de adultas no ninho (1/100 por tick, `regras/ninho.md` 8.3) e N-02, N-04, N-05, N-08, N-11. (N-01, N-07, N-10, N-12, N-14, N-15 verificados em 2026-10-18, vetores `ninho_*`.)
- Defeitos do original marcados [hipótese] no ninho (`regras/ninho.md`) e coluna 106 de `_CarpetFloorR` (`divergencias.md`).
- Suavização dupla na volta de `_Cycle` e giro duplo de `_GetNestDir` (`regras/feromonios.md`).
- Textos do balão do menino (lista 4/1200) (`regras/quintal.md`).
- Seção 13 de `regras/formiga-amarela.md` além de Y-01..Y-04 e Y13; quadros (imagens) do choque e da loucura e as mortes com a janela de edição atrás. (Mortes 2–10 da amarela, inclusive choque e loucura pelo estado e pelos geradores, verificadas em 2026-10-18, vetores `amarela_morte_*`.)
- Tutorial além das lições 1–3 (`regras/conselhos.md`, `testes-tick.md`).
- Retorno exato de `_CalcScore` e o sentido da nota 3 (`regras/simulacao-visao-geral.md`).

Geração de mundo
- `_CanBeHouseHole`; patches da casa 5–7, 9, 15, 36/37 e cozinha sem pia; `_DigOutBNest/RNest`; ramos de Tutorial/Full/Experimental de `_InitYelloAnt`/`_RandWorld`; `_AddFood` com parâmetro ≥ 0; `_DROPdir` (`regras/geracao-mundo.md` l. 106, 300, 339).

Jogo ao vivo
- Taxa de fast/ultra em máquina real: só limitada por emulação (`-icount`, ~3 ticks/s em fast por milhão de instruções/s; `regras/simulacao-visao-geral.md` §3.3); cadência sem foco 170 → 220 ms.
- `mac_por_tick` fora da bancada (`regras/quintal.md`).
- `_editWidth` = 25 também em outras resoluções (`testes-tick.md`, [hipótese]).

Telas
- Rainha amarela 0xFE e outras resoluções (`telas/jogo.md`, "Não verificado"). Navegação, superfície e ninho vermelho na edição: verificados em 2026-10-18 (`telas/jogo.md`, "Verificação").
- Desenho da aranha na janela de edição (`_DrawSpider`: o port não desenha a aranha; visto na captura do menu da aranha, `telas/janelas.md` §7).
- Altura mínima da edição e clique comido na janela de trás (`telas/janelas.md` §8, [hipótese]).
- Cores do mapa (`_ConvColor`) (`telas/mapa.md`, [hipótese]).
- Cartões 127 e 129 do Info (não alcançáveis por clique a partir do Top Card) e clique em palavra em negrito (`telas/info.md` §6).
- Clique direito no Experimental e nas janelas de controle, F1 com janela de controle, Shift+F1, Delete, `yardHelpBut`, cursor das ferramentas (`telas/ajuda-contexto.md` §6).
- Posição do menu de ferramentas no Quick Game (`divergencias.md`).
- O que o original mostra depois de `_XferPatch` (`divergencias.md`).

Som
- Tudo o que é audível: timbre, volume relativo, sons da luta no tempo, preempção no mesmo milissegundo (`regras/som.md`, `divergencias.md`). A gravação pela bancada QEMU (SB16/OPL2 em WAV) foi tentada e não serve de referência (`regras/som.md`, "Áudio do original na bancada QEMU"); falta DOSBox-X com SB16/OPL3 e captura de WAV.

Diálogos
- Modalidade dos diálogos de arquivo em relação ao `WM_TIMER` (`telas/arquivos.md`, [hipótese]).

## 4. Falta para concluir (priorizado)

Esforço relativo: **P** = até um dia de trabalho focado; **M** = alguns dias; **G** = uma semana ou mais.

### Alto impacto para quem joga

| Item | Esforço | Referência |
|---|---|---|
| Barra de menus do Windows (File, Window, View, Options, Speed), com Quit, About e as opções que hoje só existem nos botões configuráveis | G | `telas/experimental.md` l. 23 (tabela de aceleradores); `data/partida.json` |
| "Load Game" no "Select a Game" (abertura e partida) | P | `simant/src/modos.rs`; `telas/abertura.md` |
| Balões de fala da edição ("Silly", luta, descanso, ovos, rainha, aranha) | M | `_DrawBalloons`, `_EggBalloons`, `_QueenBalloons`, `_RestBalloons`, `_FightBalloons`; `rules/desenho.rs` |
| Gráficos de saúde na janela de edição (`_DrawEditGraphs`) | P–M | `telas/jogo.md` l. 105 |
| Sons de menus e diálogos (os do Experimental já tocam) | P | `regras/som.md`, "Interface: o que está ligado e o que falta" |
| Janela de placar (Ctrl+T, `0xFD17`) e `_ScoreDialog`, se forem do jogo e não de depuração [em aberto] | M | aceleradores em `telas/experimental.md` |

### Médio impacto

| Item | Esforço | Referência |
|---|---|---|
| Diálogo da trofalaxia (`_YellowDialog`) | P | `rules/amarela.rs` l. 1212 |
| Ajuda: Search, History, menus e popups pequenos | M | `telas/ajuda.md`, `divergencias.md` |
| Mapa: Shift+clique dá ordem e redesenho alternado atrás | P | `telas/mapa.md`, `divergencias.md` |
| Textos do quadro de exame e avisos do Experimental (listas 4/1010, 4/1100–1103) | P | `telas/experimental.md` §10 |
| PLACE parando a simulação e Shift para outra rainha | P | `divergencias.md` (`_PlaceQueenInYard`) |
| `_SpecialXfer` na partida ao vivo (hoje só nos vetores) e dentro do tick (ordem do original) | M | `divergencias.md`, `rules/amarela.rs` |
| Animação `_DrawSimPayoff` (trapaça `fred` e fim de jogo) | M | `regras/trapacas.md` |
| Botões configuráveis da faixa 35 | P | `regras/som.md` l. 32 |
| Queda de comida na casa (`_DROPdir` no `Estado`, `_DropFoodA`) | P | `regras/geracao-mundo.md` l. 294 |
| Timbre mais próximo do original (FM/OPL ou SoundFont escolhida) | M | `formatos/som.md`, `decisoes.md` |
| Ler o `[SimAnt]` do `WIN.INI` como origem inicial das preferências | P | `divergencias.md` |

### Baixo impacto

| Item | Esforço | Referência |
|---|---|---|
| Minimapa (`_Mini_*`, `_OpenMiniMapWin`): descobrir se o Windows usa | P (investigar) | símbolos |
| `_ExitHole` com a lista A cheia (vaga 1000) e `_KillAntLion(-1)` | P | `regras/ninho.md`, `divergencias.md` |
| Descarte de efeito pedido no mesmo milissegundo | P | `regras/som.md` |
| Bitmaps ~7100 e `kind`/byte 3 | P–M | `formatos/bitmap.md` |
| Rede (NetBIOS, DDE) | G | fora do escopo declarado |
| Janela de depuração (`_DoDebugWin`) | P | símbolos |

### Trabalho de verificação (não muda o jogo, mas fecha a conta)

| Item | Esforço |
|---|---|
| Desenho dentro do tick (`_DrawPalps`/`_SpiderDialog`) para fechar `amarela_morte_aranha` | P–M |
| Capturas: cartões 127 e 129 do Info, rainha amarela 0xFE, outras resoluções (navegação, Examine, menus, History, triângulo, 51 cartões e abertura feitos em 2026-10-18) | P |
| Tutorial completo (todas as lições) em vetores | M |
| Comparação de áudio com o original: DOSBox-X com SB16/OPL3 e captura de WAV (a bancada QEMU com SB16 foi tentada e não serve, `regras/som.md`) | M |
| Ritmo fast/ultra numa máquina real (com `-icount` feito: `simulacao-visao-geral.md` §3.3) | P |

## 5. Riscos e dívidas técnicas

1. ~~**Tabelas duplicadas nos JSON.**~~ **Resolvido em 2026-10-18 (API de mods 0.20).** A revisão achou 30 tabelas numéricas repetidas, 28 em arquivos diferentes; as que são a mesma tabela do original (14: `dx8`/`dy8`, `dx9`/`dy9`, `turn_tab`, `grupo_sem_carga`, `modos_operaria`/`modos_soldado`/`modo_por_grupo`, `sow_tab`, `classe_luta`, `matriz_luta`, `borda_buraco`, `borda_formiga_leao`) ficam agora só em `data/comum.json`, com o endereço de cada uma. As outras coincidências são tabelas diferentes do original com os mesmos valores (`boca_aranha_x/y` = `_SeatX/_SeatY`, roteiros de `quintal.json` e `casa.json`, bordas de `dialogos.json`) e ficaram. Um mod antigo com a cópia numa parte vale só para aquela parte, com aviso (decisoes.md).
2. **Ajustes de pixel medidos sem causa conhecida** ([causa em aberto] em `divergencias.md`): texto da faixa e janela do fim de jogo 1 pixel à esquerda (`mensagens.json`, `ajuste`); gato 1 pixel abaixo nos quadros 0–12 (`quintal_mapa.json`, `ajuste_medido`); título, triângulo e barras das janelas de controle; Info 1 pixel à direita e popup deslocado (`info.json`). Funcionam em 640×480 e podem errar em outras resoluções ou se a causa for outra.
3. **Hipóteses no relógio.** O relógio do Mac ao vivo começa no maior prazo do save (o port não sabe há quanto tempo o Windows está ligado); `mac_por_tick` = 9 só vale na bancada; sem relógio, `_GameTime × 6` [hipótese] em `quintal.json`. O gato, o cachorro e o balão do menino dependem disso, e com o gato e o cachorro **o consumo de sorteios** também: uma partida ao vivo não reproduz exatamente uma do original.
4. **Ritmo de fast/ultra dependente da CPU no original.** O port fixa 5 e 7 voltas por período (decisão registrada). `_BirdDelay` depende de `_gameCycles` e é a única entrada não determinística do tick.
5. **Desenho que consome o gerador S no original e não no port** (chuva e enxame do quintal, `rules/desenho.rs`; sorteios de `_DrawSpider` não medidos). Com essas janelas abertas, a sequência de sorteios pode se afastar do original. Os vetores com a Yard View na frente (`full_600`) passam porque o port faz os sorteios à parte, entre os ticks.
6. **Load ressemeia S**, então capturas do original e do port do mesmo estado têm direções diferentes nas animações (`formiga-amarela.md`).
7. **Bancada QEMU TCG** é lenta e sem placa de som utilizável (a SB16 emulada grava, mas não reproduz o original, `regras/som.md`): o ritmo, o relógio e o som medidos ali podem não valer numa máquina real. O determinismo depende do snapshot (QEMU sem `icount`).
8. **Evidência fora do git.** Capturas, `re_trabalho/` e pseudocódigo não são versionados (direitos autorais); as porcentagens de tela não podem ser refeitas por terceiros sem o jogo e a bancada.
9. ~~**Documentação desatualizada**~~ **Corrigida em 2026-10-18:** `README.md` (reescrito como guia atual), `spec/README.md`, `regras/som.md`, `regras/conselhos.md`, `regras/formiga-amarela.md`, `regras/quintal.md`, `telas/mensagens.md`, `telas/jogo.md` (a tabela de 2026-10-16 ficou como histórico), `testes-tick.md`, `executavel.md`, `divergencias.md` (duplicadas removidas, cada linha conferida, seção "Resolvidas") e `api-mods.md` (histórico em ordem, seção "Planejado").
10. **Comentários com `[hipótese]`, "ainda não" ou "[lacuna]" no código** (`engine/src`, `simant/src`): a busca de 2026-10-18 achou 56 linhas (a contagem de 70 da revisão incluía outras grafias); 8 estavam desatualizados e foram corrigidos (entre eles `core/partida.rs`, "Menus de segurar o botão ainda não existem", e `rules/tick.rs`, "subsistema ainda não implementado"); os 48 restantes são hipóteses ainda sem evidência ou lacunas reais (balões, `_YellowDialog`, `_CommandKey`, `_DropFoodA` na casa, Load Game do "Select a Game"). Nenhum `todo!` ou `unimplemented!`.
11. ~~**`jogo.json` não é sobrescrevível por mod**~~ **Resolvido em 2026-10-18 (API 0.20):** é lido com os mods antes de abrir os bancos.
12. **Ordem das janelas fixa nas animações da amarela.** `animacao.janela_edicao_na_frente` de `amarela.json` é sempre verdadeiro: com a janela de edição atrás, o original faz menos quadros (e sorteios) nas mortes e na loucura, e o port não acompanha a ordem real das janelas. Uma partida ao vivo nessa situação pode se afastar do original na sequência de S (achado de 2026-10-18, comentário em `rules/amarela.rs`).
13. **Cauda da amarela rainha na reconstrução do mapa de vida** (`core/vida.rs`): não é gravada, embora as regras já usem `_Dx8/_Dy8[_MeDir ^ 4]` para ela; provavelmente basta a mesma tabela, a conferir numa captura (`telas/jogo.md`).

## 6. Porcentagem geral estimada

Pesos por área, pensados no que importa para quem joga um port completo (as regras pesam mais; documentação e ferramentas pesam pouco):

| Área | Peso | Implementado | Peso × impl. | Verificado | Peso × verif. |
|---|---|---|---|---|---|
| Formatos de arquivo | 8 | 90 | 720 | 85 | 680 |
| Regras da simulação | 25 | 95 | 2.375 | 75 | 1.875 |
| Geração de mundo | 7 | 95 | 665 | 75 | 525 |
| Jogo ao vivo | 8 | 90 | 720 | 65 | 520 |
| Telas e janelas | 15 | 85 | 1.275 | 70 | 1.050 |
| Diálogos e menus | 8 | 60 | 480 | 50 | 400 |
| Som e música | 6 | 75 | 450 | 35 | 210 |
| Modos | 8 | 90 | 720 | 75 | 600 |
| Salvar/carregar | 4 | 95 | 380 | 90 | 360 |
| Mods | 5 | 90 | 450 | n/a | — |
| Ferramentas e testes | 3 | 90 | 270 | n/a | — |
| Documentação | 3 | 90 | 270 | n/a | — |
| **Total** | **100** | | **8.775** | | **6.220** (peso 89) |

- **Implementado:** 8.775 / 100 = **≈ 88%** (era 8.735 na revisão; subiram mods e documentação em 2026-10-18).
- **Verificado contra o original** (só as áreas 1–9, peso 89): 6.220 / 89 = **≈ 70%**.

Leitura: a simulação está praticamente completa e muito bem amarrada ao original (vetores de tick inteiros). O que separa o port de um jogo "completo" para quem joga é a camada de interface do Windows: barra de menus, Load pelo "Select a Game", balões, gráficos de saúde e sons de interface. O que separa de "verificado" são os ramos raros das regras, o áudio e as janelas sem captura.
