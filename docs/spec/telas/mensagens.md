# Avisos ao jogador: faixa, quadros de mensagem e fim de jogo

Como o original mostra ao jogador o que as regras pedem (`Evento::Mensagem`, `Evento::DialogoFimDeJogo`), a mensagem curta da faixa, o botão de velocidade e o salvamento. Complementa [jogo.md](jogo.md) ("Simulação ao vivo").

Implementação: `engine/src/core/avisos.rs` (estado, desenho e pontuação), `engine/src/core/partida.rs` (quando a simulação para; pausa, botões, conselhos, espera do clique), `engine/src/formats/textos.rs` (listas de strings), executável em `simant/src/main.rs`. Dados: `data/mensagens.json` e `data/partida.json` (`botoes_usuario`, `espera_clique_simples_tickcount`, teclas `salvar`/`salvar_como`).

**Fontes:** pseudocódigo do Ghidra (`re_trabalho/decomp/Code*.c`, só consulta; nomes do `SIMANTW.SYM`), strings do DGROUP lidas do `SIMANTW.EXE` (segmento 10) e capturas da bancada QEMU (roteiro `tests/referencias/roteiros/qemu_mensagens.txt` e o caso C-10, [bancada-qemu.md](../../bancada-qemu.md)). Legenda como em jogo.md: **[código]**, **[medido]**, **[bancada]** (observado no original), **[hipótese]**.

## 1. Relógio dos avisos

`_TickCount` (`1008:18ac`) = `GetTickCount() / 55` [código]. Todos os prazos abaixo são nessa unidade (≈ 55 ms) e correm em tempo real, inclusive com o jogo pausado ou com um quadro na tela. No port: `Partida::agora()` = ms reais passados a `Partida::avancar` / `ms_por_tickcount`.

## 2. Mensagem da faixa (`_EditMessage`, `_DrawRibbonMessage`)

### 2.1 Onde e como

- Objeto `ribMessageArea` (objeto 31 da janela 34, id `0x221f`), retângulo (190, 21)–(384, 35) do layout, isto é (190, 4)–(384, 18) da área cliente [dados do `WINGANT`].
- `_DrawRibbonMessage` (`1000:0b26`): se `_TickCount` ≤ prazo e há texto, `_font_SetFont(2)` e `_win_PrintfAtObj(0x221f, texto)` — texto centrado, resto do objeto na cor de fundo, com a cor do objeto (entrada 12 da tabela de cores: frente 12, azul claro; fundo 8, cinza); senão apaga o texto e preenche o objeto com o fundo [código].
- Não existe rodapé na janela de edição: o `_editMessage` e o `_mapMessage` guardam o mesmo texto; o segundo é desenhado também na janela do quintal animado (`_DrawAnimYardMessage`, janela 25), que o port não tem [código].
- Na bancada, "Game Paused" e "Game speed now fast." batem 100% com o port (ver §7) [medido]. Para isso o port centra o texto em `x = (esq + dir − 1 − largura) / 2`, com o retângulo do layout, quando `_win_PrintfAtObj` sugeriria `dir + 1` (o pixel a mais com faixa): 2 pixels de diferença na conta, 1 na tela [medido; causa em aberto, talvez a largura de string do original].

### 2.2 `_EditMessage(texto, duração, importante)` (`1000:92c0`)

O pedido é aceito se [código]:

1. `importante` ≠ 0; ou
2. a opção **Messages** (`_OptionStates[4]`, `8:85fa`) está ligada **e** a faixa está vazia; ou
3. o texto é nulo e a mensagem atual é permanente (pedido de limpar).

Aceito: prazo = permanente se a duração é negativa (as chamadas usam `0xfffe, 0xffff` = −2), senão `_TickCount + duração × 3 / 2`; o texto passa a ser o pedido (nulo = limpa); redesenha a faixa (e o quintal animado, se aberto) [código].

### 2.3 Quem escreve na faixa

| Origem | Texto | Duração | Importante |
|---|---|---|---|
| `_SetPause(1)` / Speed → Pause, com `_CurGameTool` = −1 | SHARED 4/1010[2] "Game Paused" | permanente | sim |
| idem, ferramenta 10 (troca de corpo) / 11 (alvo) | 1010[3] "Click on ant to exchange bodies" / 1010[17] "Click on ant to chase or UnPause" | permanente | sim |
| idem, outra ferramenta | nada muda | — | — |
| `_SetPause(0)` / despausar | limpa | permanente | sim |
| `_Feedback` (todo tick), conselhos | 1020[0..4] ([conselhos.md](../regras/conselhos.md) §2) | 0x78 → 180 (≈ 9,9 s) | só o nº 0 |
| `_Feedback` → `_SetDefaultWindPrompt(0)` | o mesmo texto de pausa/ferramenta, ou limpa | permanente | não |
| `_DoUserButton` caso 6 (botão de velocidade) | "Game speed now slow./normal./fast./ultra." (DGROUP `1048:014b`, `0160`, `0177`, `018c`), pela velocidade nova | 0xb4 → 270 (≈ 14,9 s) | sim |
| `_DoBookMark` | "Bookmark has been placed." / "…has not been placed." (`1048:01a2`, `01bc`) | 0xb4 | sim |
| `_EndGameDialog` | limpa | permanente | sim |

[código; textos do banco e do DGROUP verificados].

Mensagens pedidas pelas regras (`Evento::MensagemFaixa { lista, indice, duracao, importante }`, sem sorteio; os valores ficam nos arquivos de dados das regras) [código]:

| Origem | Texto | Duração | Importante |
|---|---|---|---|
| `_GstrR` / `_StartAttack` (ataque vermelho) | 1020[5] "The red ants are attacking!" | 0x78 | não |
| `_EatMyFood` tipo 1 (trofalaxia) / 2 (doação) / 3 (ovo comido) | 1010[11] / [12] / [13] | 0x78 / 0x78 / 0xb4 | não |
| `_DoAntMoveY`, passagem para o ninho vermelho | 1010[4] "You have entered the red nest" | 0x168 | não |

Marcadores (`_DoBookMark`, `1000:08a6`, botões `ribBM1`–`7`): **Ctrl**+clique (`GetAsyncKeyState(VK_CONTROL)`: o código empurra 0x11) guarda plano, `_MapPnt`, `_MapMode` e `_YardMode` e troca os bitmaps do botão (`0x38dd + i`/`0x38ab + i`), com "Bookmark has been placed."; clique sem Ctrl num marcador vazio toca o som 1 e mostra "Bookmark has not been placed."; num marcador posto, `_GotoMapPoint(plano, _editWidth/2 + x, _editHeight/2 + y)` [código]. Os marcadores não estão no save; `_NewGame` os limpa. **Bancada:** o Ctrl+clique no `ribBM1` deu a mensagem e os bitmaps iguais ao port (99,73% na faixa; j05) e o clique no `ribBM2` vazio deu "Bookmark has not been placed." (j06). O port não guarda o modo do quintal (a janela do quintal é de outro módulo).

**Bancada** (`m1`–`m4`): depois do Load o jogo fica pausado com "Game Paused"; o botão de velocidade troca por "Game speed now fast." (era normal); despausar (Shift+0) **apaga** essa mensagem (pedido importante de limpar), e ~18 s depois a faixa mostrava o conselho "Your nest needs expansion" [bancada]. O prazo de 270 unidades não foi medido diretamente.

## 3. Quadro de mensagem (`_PictStrnDialog`, `_PictureDialog`)

### 3.1 Quando aparece

`_PictStrnDialog(imagem, texto, forçar)` (`1000:615a`): só age se `forçar` ≠ 0 ou se a opção **Events** (`_OptionStates[3]`, `8:85f8`) está ligada. Então `_StopSimulation` (mata o temporizador e descarta as `WM_TIMER` pendentes), lê a lista SHARED 4/`texto` (`_LoadStringAnt`: uma string Pascal por linha), mostra o quadro e, ao fechar, `_RestartSimulation` [código].

As chamadas com `forçar` = 0 (dependem de Events): 10006/10007 (`_XferPatch`), 10022/10024 (`_MysteryButton`), 10046 (aranha morta), 10050 (formiga-leão morta). As demais têm `forçar` = 1; quando o original testa Events antes de chamar (mortes da amarela), a regra do port já testa (`rules/amarela.rs`) [código]. Lista em `data/mensagens.json` (`nao_forcadas`).

**A simulação para no meio do tick** que pede o quadro (a chamada é síncrona). O port termina o tick e só então mostra o quadro: o estado é o mesmo, porque o quadro não muda nada [código]. Vários quadros no mesmo tick (10010 e 10011 da derrota) aparecem um depois do outro.

### 3.2 Imagem

O primeiro parâmetro é um bitmap (tipo 2) desenhado acima do texto, ou 0. Mortes da amarela (`_YellowDeath`, `1000:7746`): 10034 → 9155, 10036 → 9152, 10038 → 9150, 10040 → 9154, 10042 → 9151, 10045 → 9156; lições do tutorial: a `imagem` de cada lição em `data/quintal.json` (17000–17009) [código]. Nenhum quadro com imagem foi capturado ainda.

### 3.3 Geometria (`_PictureDialog`, `1000:62bc`, e `_win_DrawPictureWindow`, `1000:61f6`)

Com a fonte 4 (`_font_SetFont(4)`, arquivo `font3`, altura H = 14), `n` linhas, `w = max(50, maior largura de linha)` e `P` = altura da imagem + 2 (ou 0) [código]:

- retângulo pedido: largura `w + 8`, altura `e = n·H + P + 8`, centrado na área principal **abaixo da faixa** (`GetClientRect` dá 640 × 369 na bancada): `l = (640 − (w + 8)) >> 1`, `t = (369 − e) >> 1` [código + medido];
- a janela 30 (`picStrWin`) tem objeto 0 de contorno (espessura 2) com deslocamentos (−5, −4, +4, +4) sobre esse retângulo, e encolhe pelo contorno; resultado medido: **área cliente em (l + 2, faixa + t + 3), com (w + 12) × (e + 3)**, e a moldura do Windows (1 pixel preto + 4 na cor da borda) em volta [medido em C-10, 10010 e 10011];
- fundo: cor de fundo do objeto `picStrArea` (entrada 12: cinza); texto na cor de frente (azul claro);
- imagem centrada no alto da área cliente; linhas a partir de 2 pixels abaixo do topo (ou logo abaixo da imagem + 2), uma a cada H, **centradas na largura da área cliente** [código + medido];
- "Click here to continue" (objeto `picStrClick`, fonte 3 = `font2`, cor 9: preto sobre cinza) centrado na área cliente com 1 pixel a mais à direita e com o topo da imagem do texto 10 pixels acima da base da área cliente [medido].

### 3.4 Como fecha

`_DialogWaitInit(0xf)` e o laço de `_PictureDialog` [código]:

- clique na área (`picStrArea`, objeto selecionável): fecha;
- clique fora (evento do objeto 0): `MessageBeep`, continua;
- **Espaço** (`GetAsyncKeyState(VK_SPACE)` em `_DialogAbortOrCont`): fecha;
- **prazo**: 15 × 18 = 270 unidades de `_TickCount` (≈ 14,85 s) depois de abrir, fecha sozinho. **Confirmado na bancada**: o 10010 aberto no Load já tinha sido trocado pelo 10011 18 s depois (`m6`) [bancada].

O port fecha com o clique esquerdo dentro da área cliente, com Espaço (`tecla_continuar`) ou pelo prazo; o clique fora só escreve "(bip)" (sem som).

### 3.5 A simulação com o quadro aberto

Quais caixas param a simulação (desmontagem, 2026-10-18) [código]:

- `_PictStrnDialog` (`1000:615a`) chama `_StopSimulation` antes de abrir e `_RestartSimulation` depois de fechar — **sempre**, venha de dentro do tick (lição do tutorial `_GiveLesson`, morte da amarela, conselhos de `_Feedback`) ou de fora (os quadros do `_FullCount` no Load). `_StopSimulation` (`1000:0000`) faz `KillTimer` e descarta os `WM_TIMER` pendentes na primeira chamada; as chamadas se aninham por um contador (`DGROUP:0x44`), e só a última `_RestartSimulation` refaz o `SetTimer` (17 ms).
- `_EndGameDialog` (GAME OVER) e as caixas dos comandos de arquivo também matam o temporizador (`KillTimer` no `_ProcMenu`, comandos 0xE/5/6) e o refazem no fim.
- `MYTIMERFUNC` só chama `_DoAntSim` com a trava `DGROUP:0x2f4` em 0; ela fica em 1 durante todo o `_DoAntSim` (o único chamador). Uma caixa aberta **dentro do tick**, portanto, nunca deixa outro tick rodar, mesmo que o temporizador volte.
- Exceção: o `WM_ACTIVATEAPP` (mensagem 0x1C em `MAINWNDPROC`, `1000:2930`) refaz o temporizador sem olhar o contador do `_StopSimulation` (170 ms ao perder o foco, 17 ms ao ganhar). Com uma caixa aberta **fora** do tick (Load), trocar o foco da aplicação faria a simulação andar com a caixa na tela [código; não observado].
- Na bancada: no Load do C-10 o `GameTime` ficou em 78 com os quadros 10010/10011 abertos (sem troca de foco), o que bate com o código. O vetor `tutorial/23` (`GameTime` 23 → 40 "com a caixa aberta") contradiz a trava: como a caixa fecha sozinha em 270 unidades (≈ 14,85 s) e o script da bancada só clica depois de 15 s sem ver o retorno de `_DoAntSim`, a explicação mais provável é que o tick de entrada voltou (`GameTime` 24) depois do prazo e a leitura veio ticks mais tarde [hipótese a conferir: gravar o vetor de novo fechando a caixa antes do prazo].

**Port:** com um aviso modal na tela (`Avisos::tem_modal`), `Partida::avancar` não gera mensagens de temporizador (`_StopSimulation`) e, se a caixa abriu no meio de um lote, descarta as mensagens pendentes; o tick que pediu a caixa termina inteiro antes dela (as regras não esperam a resposta). A troca de foco não reinicia a simulação com uma caixa aberta (a exceção acima não é modelada).

## 4. Fim de jogo (`_EndGameDialog`, `1000:6602`)

1. Com **Music** ligado (`_OptionStates[1]`), toca 0x2718 (pretas venceram, `_BlackWon` ≠ 0) ou 0x271a; quando a música acaba, toca uma de 0x2713 + `_SRand2(...)` [código; o port só registra a primeira música, sem som].
2. `_EditMessage` limpa a faixa; abre a janela 4 (`gameOverWin`, bitmap 16100, posição da tabela 9/0 como as outras janelas).
3. `_win_DrawEndGameWindow` (`1000:64d4`), com a fonte 4: `gameOverType` = SHARED 4/1001[`_CurGameType`] ("Tutorial Game", "Quick Game", "Full Game", "Experimental"; fora de 0..3 usa o 2); `gameOverScore` = pontos (`%ld`); `gameOverRating` = 4/1900[i], com `i` pela faixa dos pontos (÷ 5 no Full Game): < 214220 → 0, < 428440 → 1, < 642660 → 2, < 856880 → 3, senão 4; sem `_BlackWon`, `i + 5` [código].
4. Espera o clique em `gameOverPic` (a janela inteira); não tem prazo [código; bancada: 20 s sem fechar, `m9`].
5. Fecha, `_DialogAbortAll`, `_NewGame(0)`: a caixa "Select a Game" [bancada, `m10`].

`_EndGameDialog` é chamado a cada tick enquanto `_IsGameOver` ≠ 0 (simulacao-visao-geral.md); como fica num laço modal, na prática aparece uma vez. **Bancada:** depois do Load do C-10 (jogo pausado) os quadros 10010 e 10011 apareceram, mas o fim de jogo só depois de um clique no mapa, que deu uma ordem à amarela e fez um tick rodar mesmo pausado (§4 de simulacao-visao-geral.md) [bancada, `m7`/`m8`].

**Port:** o diálogo é desenhado com a janela 4; a posição medida fica 1 pixel à esquerda da calculada (`ajuste` em `data/mensagens.json`) e os textos centram sem o pixel a mais à direita, como na faixa [medido]. Ao fechar, o port abre o "Select a Game" sobre a partida, como `_NewGame(0)` no original (`simant/src/modos.rs`).

**Depois do GAME OVER** (`_EndGameDialog` → `_NewGame(0)`, dentro do tick no original): o "Select a Game"; a escolha gera o jogo novo **sem ressemear** S nem R — o mundo sai dos geradores do instante — e o tick volta com o jogo novo em `GameTime` 0 [bancada, vetor `inseticida_morte/21` regravado]. Port: a partida pede o diálogo uma vez; a interface mostra as caixas na ordem e, fechado o GAME OVER, abre o "Select a Game" (`Modal::abre_selecao_de_jogo`); o jogo novo vem de `carregar::jogo_novo_com_geradores` (também no Ctrl+N), conferido 307/307 com S e R contra o vetor.

### 4.1 Pontuação (`_CalcScore`, `1000:5530`)

Oito parciais `p[0..7]` (0 a 100) [código]:

| p | Conta | Sem dados |
|---|---|---|
| 0 | média de `_H_BHeal` nas `_HistCnt` amostras do anel (de `_HistStart − _HistCnt`) | 0 |
| 1 | Σ`_H_BFood` × 100 / (Σ`_H_BFood` + Σ`_H_RFood`) nas mesmas amostras | 0 |
| 2 | (`_TotalEggsLaidB` − `_TotalEggsDiedB`) × 100 / `_TotalEggsLaidB` (32 bits) | 100 |
| 3 | `_BlackLost` × 100 / (`_BlackLost` + `_RedLost`) | 100 |
| 4 | só Full/Experimental: `_BColoniesStarted` × 100 / (`_BColoniesStarted` + `_RColoniesStarted`) | 100 |
| 5 | idem: `_BColoniesStarted` × 100 / (`_BColoniesStarted` + `_BColoniesKilled`) | 100 |
| 6 | idem: patches do quintal com `_YMapPopB` ≠ 0, × 100 / 155 (x ≥ 3 para y < 5, x ≥ 2 para y ≥ 5, x < 12; índice x × 16 + y) | 0 |
| 7 | idem: patches da casa (o resto: 37) com `_YMapPopB` ≠ 0, × 100 / 37 | 0 |

`soma = _MeHealth + Σ peso[i] × p[i] × 51`, com os pesos do DGROUP `1048:0948` = 13, 17, 19, 23, 29, 31, 37, 41. Full Game: se `_GameTime` < 8100, `soma = max(1, _GameTime / 100) × soma / 81`; depois `soma × 5`. Outros tipos: `soma = soma × 29 / 10`; se `_GameTime` < 4100, `soma = max(1, _GameTime / 100) × soma / 41`. **Pontos = `_GameTime` + soma** [código; contas em 32 bits com sinal: o pseudocódigo mostra algumas só na palavra baixa, hipótese do port].

**Bancada:** o C-10 deu "Quick Game", **19785**, "Aardvark Fodder" (`m8`). O port, com o estado do C-10 e `_GameTime` = 79 (um tick depois do Load, o da ordem do clique), dá exatamente 19785 (`cargo run -p engine --example pontuacao -- C10.ANT 79`) [bancada].

## 5. Botões configuráveis da faixa (`_DoUserButton`, `1000:04c4`)

Os objetos `ribUserDef1A`–`1H` (ids `0x2210`–`0x2217`) chamam `_DoUserButton(id, índice)`; a função de cada botão vem da tabela `1048:0012` (padrão: 15, 0, 1, 3, 7, 6, 5, 10; o `WIN.INI` pode trocar via `_SetUserButton`) [código + DGROUP]. Clique simples:

| Função | Efeito |
|---|---|
| 0, 1, 2, 3 | abre a janela Behavior, Caste, History ou Info (port: Behavior, Caste e History em janelas.md; Info ainda não) |
| 4 | liga/desliga as trilhas do mapa (port: só o estado do botão; a janela do mapa não existe) |
| 5 | `_SetPause(!_GamePaused)` (§2.3) |
| 6 | `_GameSpeed` += 1 (3 volta a 0) e a mensagem "Game speed now …" (§2.3) |
| 7, 9, 10, 11, 12, 13 | alternam `_OptionStates[1]` (Music), `[2]` (Sound Effects), `[5]` (Silly), `[0]` (AutoTrack), `[3]` (Events), `[4]` (Messages) |
| 8 | `_MysteryButton` (port: ainda não) |
| 14 | `_SaveGame(1)` (§6) |
| 15 | abre a janela de edição e `_GotoMyAnt` |

Com **Ctrl** apertado (`GetAsyncKeyState(VK_CONTROL)`; o código empurra 0x11 — uma versão anterior deste documento dizia "segurado"), abre a janela 36 (`iconMenu`, bitmap 7200, 16 ícones) para escolher a função do botão; com Shift, só muda este botão, sem Shift os dois botões equivalentes das faixas 34 e 35 [código]. O menu e o estado desenhado dos botões estão em [janelas.md](janelas.md) §4.4 e §5.

**Bancada:** o 6º botão (`ribUserDef1F`, em (159, 96) da tela) mostrou "Game speed now fast." partindo de normal (`m2`) [bancada].

## 6. Salvar (`_SaveGame`, `1010:a23e`)

- File → Save (comando 5, **Ctrl+S**) chama `_SaveGame(1)`: se já há nome de arquivo (`1038:9520`), grava nele; senão (e em Save As, comando 6, **Ctrl+A**) pede o nome no diálogo comum do Windows (`_FileSelect`) [código]. Os comandos de arquivo passam por `_fileWaitFlag` e são tratados fora do temporizador.
- Confere o espaço livre, abre/cria o arquivo e grava os blocos da tabela `DS(8):0x9570` na ordem ([save-ant.md](../formatos/save-ant.md)); depois `_PopMsg("%s Saved correctly")` (caixa de mensagem do Windows, como em `qi_14_salvo.png`) [código + captura].
- **Port:** `Estado::para_save` grava os 307 blocos (a ida e volta é byte a byte, `estado_simulacao.rs`). Ctrl+S grava no arquivo do `--save` (ou no último salvo); sem arquivo, e em Ctrl+A, no nome de `--salvar`, ou no primeiro `SALVOnn.ANT` livre na pasta atual. Desde 2026-10-18, sem arquivo e em Ctrl+A abre o diálogo "Save Ant Hill As" e, depois de gravar, a caixa "Message" com "… Saved correctly" ([arquivos.md](arquivos.md)); `--salvar` pula o diálogo. Sem janela, `--salvar ARQ` grava o estado depois dos `--ticks`.
- **Bancada:** `PORT50.ANT` (QUICK2 + 50 ticks no port, `--salvar`) foi carregado pelo original sem erro; o estado lido logo depois do Load difere do arquivo só em `_GamePaused` (o Load pausa), `_CastePopR` (recontado no Load, como com o `QUICK2.ANT`) e `_MapPnt` (21 → 22: a vista que o Load calcula; jogo.md, "Posição ao carregar") [bancada, `port_carregado.ANT`].

### 6.1 O que o Load faz além de ler os blocos (`_LoadGame` `1010:9fa0`, `_DoLoadInitializations` `1010:9dbe`)

- `_FullCount` (`_CountAnts` + `_Feedback`): recenseia as castas — por isso `_CastePopB/R` mudam no Load (bancada-qemu.md) — e já detecta a rainha que sumiu: no caso C-10 os quadros 10010 e 10011 aparecem **logo no Load**, com `_GameTime` = 78 [código + bancada]. Os quadros são modais e aparecem antes de as janelas serem redesenhadas (a captura mostra o jogo anterior atrás do quadro).
- `_SetDefaultWindows`, `_CenterEdit` na amarela, `_SetDefaultWindPrompt(1)` e, depois de tudo, `_PauseGame(1)`: o jogo carregado fica pausado com "Game Paused" — só depois que os quadros do `_FullCount` fecham (a faixa está vazia com o quadro na tela) [código + bancada m5/m7].
- Antes de ler: `_RandYard` (consome sorteios; rng.md), `9:9c70` = 1, `_TimeTemp` = 300 e mais três variáveis zeradas [código; o port não reproduz as de fora do save].
- Port: `Partida::ao_carregar()` (chamado pelo executável no `--save`): `contar_formigas`, eventos, conselho e a pausa, adiada até os quadros fecharem. Os vetores de tick (72/72) não passam por aqui.
- Vista: o original centraliza a amarela em (34, 9) com `_MapPnt.x` = 22, tanto ao carregar quanto ao criar o Quick Game do QUICK2: `_editWidth` = 25, não 26. O port passou a usar 25 colunas (`ajuste_area_com_faixa` = 0 em `data/tela_jogo.json`); com isso a janela de edição depois do Load bate 99,94% com a captura m7 (só o cursor) [bancada].

## 7. Espera do clique simples (`_processEdit`, `1000:6a38`)

No clique simples (não duplo) com a janela de edição na frente, o original fica num laço até `_TickCount` = início + 6 (≈ 330 ms), sem tratar mensagens: a simulação não anda. Depois, se o botão continua apertado (`_myButton`) e Shift está solto, abre `_AntMenu` (clique na própria amarela) ou `_MagnifyMenu` (Examine, noutra célula); se o menu devolve uma escolha, a ordem não é dada [código]. O port para a simulação pelos mesmos 330 ms (`espera_clique_simples_tickcount` × 55 ms) e decide no fim deles, com o botão e o Shift daquele instante: menu da amarela (janelas.md §4.2), Examine (sem a janela 29, lacuna) ou a ordem.

## 8. Verificação (2026-10-17)

Comparação pixel a pixel com `tools/comparar_captura.py --regiao … --mapear` (cores mapeadas por índice, ver jogo.md), port exportado com `simant --save … --ticks/--botoes/--fechar-avisos … --exportar-tela-jogo`:

| Aviso | Captura | Região (área cliente) | Iguais |
|---|---|---|---|
| quadro 10010 (1 linha) | `C-10/t3_carregado.png` e `mensagens/m5` | (186, 244)–(462, 279) | **100%** |
| quadro 10011 (2 linhas) | `mensagens/m6_apos_18s.png` | (201, 237)–(446, 286) | **100%** |
| faixa "Game Paused" | `mensagens/m1_port_carregado.png` | (190, 4)–(384, 18) | **100%** |
| faixa "Game speed now fast." | `mensagens/m2_velocidade.png` | (190, 4)–(384, 18) | **100%** |
| fim de jogo (janela 4) | `mensagens/m8_apos_clique2.png` | (128, 128)–(464, 346) | 97,95% (pontos de outro `_GameTime`: 19787 contra 19785; a linha da avaliação 1 pixel abaixo; o cursor do mouse) |

Testes: `core/avisos.rs` (prazo e regras de `_EditMessage`, pausa, velocidade, quadros forçados, prazo de 270, imagens, avaliação, pontuação), `engine/tests/partida_quick2.rs` (salvar e recarregar, quadros param a simulação até fechar, botão de velocidade, espera do clique).

**Não verificado:** quadros com imagem (mortes da amarela, tutorial), o prazo de 270 da faixa, o fim de jogo com vitória e no Full Game, a pontuação com parciais 4–7, outras resoluções (800×600).
