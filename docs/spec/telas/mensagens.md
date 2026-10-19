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
| Speed → Slow/Normal/Fast/Ultra e Shift+1..4 (`_ProcMenu` 0x43–0x46) | **nada**: só trocam `_GameSpeed` e as marcas do menu [código; bancada `qemu_ritmo.txt`, simulacao-visao-geral.md §3.2] | — | — |
| `_DoBookMark` | "Bookmark has been placed." / "…has not been placed." (`1048:01a2`, `01bc`) | 0xb4 | sim |
| `_EndGameDialog` | limpa | permanente | sim |

[código; textos do banco e do DGROUP verificados].

Mensagens pedidas pelas regras (`Evento::MensagemFaixa { lista, indice, duracao, importante }`, sem sorteio; os valores ficam nos arquivos de dados das regras) [código]:

| Origem | Texto | Duração | Importante |
|---|---|---|---|
| `_GstrR` / `_StartAttack` (ataque vermelho) | 1020[5] "The red ants are attacking!" | 0x78 | não |
| `_EatMyFood` tipo 1 (trofalaxia) / 2 (doação) / 3 (ovo comido) | 1010[11] / [12] / [13] | 0x78 / 0x78 / 0xb4 | não |
| `_DoAntMoveY`, passagem para o ninho vermelho | 1010[4] "You have entered the red nest" | 0x168 | não |

"You have entered the red nest" só sai quando a amarela atravessa a **passagem entre os ninhos** (tile 0x14, do ninho preto para o vermelho). Entrar no ninho vermelho pelo buraco da superfície (`_EnterNest`, `1028:8408`) não escreve nada na faixa [código]. Como não é importante, também não aparece se a faixa já tem uma mensagem com prazo (um conselho, por exemplo) ou se Options → Messages está desligado (§2.2). O port segue as três regras (`rules/amarela.rs`, teste `mensagem_do_ninho_vermelho_so_pela_passagem` em `engine/tests/partida_quick2.rs`).

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
| 0, 1, 2, 3 | abre a janela Behavior, Caste, History ou Info (port: Behavior, Caste e History em janelas.md; Info em [info.md](info.md)) |
| 4 | liga/desliga as trilhas do mapa (port: `Partida::mostrar_trilhas`, usado pela janela do mapa, [mapa.md](mapa.md)) |
| 5 | `_SetPause(!_GamePaused)` (§2.3) |
| 6 | `_GameSpeed` += 1 (3 volta a 0) e a mensagem "Game speed now …" (§2.3) |
| 7, 9, 10, 11, 12, 13 | alternam `_OptionStates[1]` (Music), `[2]` (Sound Effects), `[5]` (Silly), `[0]` (AutoTrack), `[3]` (Events), `[4]` (Messages) |
| 8 | `_MysteryButton` (port: `rules/misterio.rs`; ações `misterio_*` em `tests/referencias/acoes/`) |
| 14 | `_SaveGame(1)` (§6) |
| 15 | abre a janela de edição e `_GotoMyAnt` |

Com **Ctrl** apertado (`GetAsyncKeyState(VK_CONTROL)`; o código empurra 0x11 — uma versão anterior deste documento dizia "segurado"), abre a janela 36 (`iconMenu`, bitmap 7200, 16 ícones) para escolher a função do botão; com Shift, só muda este botão, sem Shift os dois botões equivalentes das faixas 34 e 35 [código].

A faixa 35 (quintal) tem os seus: `ribUserDef2A`–`2H` (ids `0x230b`–`0x2312`, objetos 11–18 da janela 35), tratados por `_ProcYardRibbonEvent` com `_DoUserButton(id, id − 0x230b)`; a tabela é `_yardUserButton` (`1048:0022`, padrão igual ao da faixa 34: 15, 0, 1, 3, 7, 6, 5, 10; `WIN.INI` `yarduserbuttonN`). `_DoUserButton` escolhe a tabela pelo byte alto do id (0x22 → `1048:0012`, senão `1048:0022`) e, sem Shift, grava a função nas duas tabelas e chama `_SetUserButton` para os dois ids (id ± 0xFB) [código]. **Port** (2026-10-18): `partida.json` → `botoes_usuario_quintal`, `Partida::funcoes_botoes_quintal`; clique, Ctrl+clique (menu de ícones) e o estado desenhado (bitmaps e ligado) como os da faixa 34; preferências `botoes_quintal`. Teste `botoes_da_faixa_e_menu_da_amarela`. **Sem captura da faixa 35 com botões trocados.** O menu e o estado desenhado dos botões estão em [janelas.md](janelas.md) §4.4 e §5.

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

## 9. Barras e números da faixa (`_DrawMapData`, `1010:cb60`)

A cada `_UpdateWindows` (e com o mapa: `_DrawMap`), com a faixa aberta [código]:

- **Números** (`_win_PrintfAtObj` com "%d", fonte 3, cores do objeto): `ribBlacks` = `_BpopT`, `ribReds` = `_RpopT`. Centralizados no objeto, como a mensagem da faixa.
- **Barras** (`_win_DrawHBar(objeto, fração em 1/65536)`): `ribYellowHealth` = `_MeHealth`/100; `ribBlackHealth` = `_HealthB`/100; `ribBlackP` = `_BpopT`/m; `ribRedHealth` = `_HealthR`/100, ou 0 se `_RpopT` = 0; `ribRedP` = `_RpopT`/m; com m = max(1, `_BpopT`, `_RpopT`). As cores vêm de `_win_SetColorFromObjNum`: entrada `cor` do objeto na tabela de cores (WINGANT 0/129), frente até a fração e fundo no resto (amarela: cor 4; pretas: 9; vermelhas: 10).
- **Marcas** (`_GInvBox`, inverte uma coluna): em `ribBlackHealth` no `_BlkWarnHealth` e em `ribYellowHealth` no `_MeWarnHealth` (os dois valem 30 num jogo novo), na coluna `esq + ⌊largura × limiar / 100⌋`.
- `_ConvColor` só muda a cor com `_displayType` = 10 (tabela DGROUP `0x7238`); não é chamado aqui (o fundo da mensagem da faixa usa `_ConvColor(0xc)`).

Geometria **medida** (bancada, roteiro `qemu_barras_faixa.txt`: três capturas com a VM pausada e os sete valores lidos logo depois): as barras pintam as colunas `esq..dir − 1` do objeto (64 pixels; a coluna `dir` é a moldura do bitmap da faixa) e as linhas `topo..base − 1` (5 ou 8); a divisão usa a largura 63: `s = esq + ⌊63 × fração / 65536⌋`, frente em `esq..s − 1` e fundo em `s..dir − 1` (cheia: tudo frente). As 15 barras das três capturas e as duas marcas batem com essa regra; o pseudocódigo (`dir + 1` com `_ribbonBarHeight` ≠ 0, e o retângulo do layout) daria 65, e a diferença deve vir de `_win_Recalc` (não estudado). O port põe os valores em `data/tela_jogo.json` (`barras_faixa`, `encolher` = 1) e desenha em `core/barras_faixa.rs`. **Conferido:** com o `QUICK3.ANT`, a região das barras e dos números (x ≥ 512, linhas 38–110 da tela) é igual à captura `qs_salvo.png` do mesmo instante, exceto a cor exata da marca na barra amarela (azul-escuro da paleta do port, (0, 0, 128), contra (0, 0, 168) do VGA do QEMU).

### 9.1 Clique na área das barras (`mapRibCaste`, `yardRibCaste`)

A área das barras é um objeto da faixa (`mapRibCaste`, objeto 39 da janela 34, id `0x2227`; `yardRibCaste`, objeto 36 da 35, id `0x2324`) que gera evento ao ser clicado. `_ProcMapRibbonEvent` e `_ProcYardRibbonEvent` [código]:

- Ponto dentro da barra de saúde da amarela (`0x2222` `ribYellowHealth` / `0x231F` `ribYellowH2`): `_DoHealthSetY` grava `_MeWarnHealth = (x − esq) × 100 / (dir − esq)`.
- Dentro da barra de saúde da colônia preta (`0x2223` `ribBlackHealth` / `0x2320` `ribBlackH2`): `_DoWarnSetB` grava `_BlkWarnHealth` com a mesma conta.
- Em qualquer outro ponto da área (números de população, barras de população, barra de saúde vermelha, rótulos): `_DrawCastePopUp` (`1000:5d9e`).

Os dois limiares (30 num jogo novo, `_InitSimVars` `1030:5a70`) são as marcas invertidas das barras (§9) e decidem os avisos de saúde (`_MeWarnHealth`: a amarela quer comer; `_BlkWarnHealth`: aviso da colônia). A conta usa os retângulos `ribYellowH2`/`ribBlackH2` da **faixa 35** nas duas faixas (no pseudocódigo, `_win_GetObjRect(0x231F/0x2320)`), com esq = 567 e dir = 631 [código + bancada]. **Bancada** (`qemu_ui_barras_direito.txt`, 2026-10-18, snapshot `quick` pausado): clique em x = 580 da barra preta → `_BlkWarnHealth` 20; x = 620 da amarela → `_MeWarnHealth` 82; apertar em x = 590 → 35 e arrastar até 610 com o botão apertado **não** muda (só o clique conta).

`_DrawCastePopUp` [código + bancada]: guarda `_GamePaused`, `_SetPause(1)`, `_LessonTemp = 1`, abre a janela 23 (`antPopWin`: bitmap 14002 com os rótulos Larvae, Workers, Soldiers, Males, New Queens, Queens e as figuras) encostada na borda direita, embaixo da faixa, e escreve, com a fonte 2, para cada casta i = 0..5 o `_CastePopB[i]` (cor do objeto `antPopBlack`) e, uma linha abaixo, o `_CastePopR[i]` (`antPopRed`), na coluna esquerda de `antPopGraphArea`; para i ≥ 1, barras de comprimento `valor × L / m` (L = direita da área − (esquerda + 4 × largura do maior dígito + 2), m = maior número das castas 1..5 das duas cores, no mínimo 1). As linhas se repartem na altura da área: sobra = altura − 12 × altura da fonte + 3, um quinto entre as castas e o resto distribuído de 1 em 1 pixel; a última sobe 1 pixel. O quadro fica aberto enquanto o botão está apertado (`_ButtonHeld`) e fecha ao soltar, devolvendo a pausa anterior.

Geometria medida (captura `b01_ribBlacks.png`): contorno preto em (451, 76) da área cliente, 189 × 149, 1 linha preta e 4 azul-escuras; o conteúdo começa 2 pixels antes do canto de dentro da moldura e é cortado por ela à direita e embaixo; a direita e a base da área do gráfico contam 1 a menos que o retângulo do layout. **Port** (`core/popup_castas.rs`, `data/popup_castas.json`; `simant/src/modos.rs`): mesmas regras; a simulação não anda com o quadro aberto. Comparado com `b01_ribBlacks.png` (mesmos números): **99,98%** dos pixels do quadro iguais (a diferença é o cursor). Teste: `barras_da_faixa_como_no_original` (simant).

**Não verificado:** quadros com imagem (mortes da amarela, tutorial), o prazo de 270 da faixa, o fim de jogo com vitória e no Full Game, a pontuação com parciais 4–7, outras resoluções (800×600).

## 10. Quadro da formiga amarela (`_YellowDialog`, `1000:8262`)

Chamado com um bitmap e um índice de texto [código]: por `_EatMyFood(1)` (trofalaxia, `0x2396` = 9110, índice −1), por `_YellowDeath` nas causas 1 e 2 (aranha e formiga-leão, `0x238c` = 9100, índice 0) e por `_YellowBirth` (`0x238d` = 9101, índice `param + 1`). Só com Options → Events (`_OptionStates[3]`) ≠ 0. Abre a janela 26 (`dialogPicWin`) com o bitmap no objeto 1 (`dialogPic`), captura o mouse e:

- **9110 (trofalaxia):** `_myBeginSound(0x2a)`, `_myDelay(0x2d)` (45 unidades de `_TickCount`; um evento ou Espaço encerra a espera antes), com a janela ainda aberta desenha o bitmap 9111 em `_YellowEatPnt` (`8:896c` = (72, 62)) a partir do canto do objeto 1, `_myBeginSound(0x2d)`, `_myDelay(0x2d)`.
- **9101:** escreve, na fonte 4, a string `23 + índice` da lista 1010 ("You have been reborn!", "You are now a worker!") no objeto 2 (`dialogText`), e `_myDelay(300)`.
- **9100:** o texto só nos modos monocromáticos; `_myDelay(300)`.

Depois, sem evento pendente, `_DialogWaitInit(3)` (54 unidades) até o prazo, Espaço ou um evento; fecha a janela e redesenha tudo. Tudo isso acontece **dentro do tick** (a simulação fica parada).

**Bancada (2026-10-18, cenário `trofalaxia` de `tools/cenarios_tick.py`, snapshot `rico`):** amarela com `_MeHealth` = 10 e duplo clique numa operária preta da superfície; ao encostar, o quadro aparece: contorno azul-escuro de 4 pixels de (156, 159) a (361, 326) da área cliente (janela de 206 × 168), bitmap em (160, 163) cortado em 198 × 160 (o bitmap tem 199 × 161), e o 9111 sobreposto; `_MeHealth` vai a 100 depois do quadro. O port (`Modal::Amarela`, `core/avisos.rs`, dados em `mensagens.json` → `quadro_amarela`; teste `quadro_da_trofalaxia_como_no_original`) desenha a região do quadro com a sobreposta **100%** igual às capturas `trof_029`–`031`; a captura `trof_028` pegou o 9111 a meio caminho (as linhas de cima já iguais). No port, um clique ou tecla fecha o quadro de uma vez (no original, o evento encerra o `_myDelay` e pula a espera final) [aproximação].

**`_myDelay` volta logo (bancada, 2026-10-18).** `_myDelay(n)` (`1000:9620`) espera `n` unidades de `_TickCount`, mas sai antes com um evento, com Espaço ou quando vence o prazo do nível atual da pilha de `_DialogWaitInit` (`1048:150a`, prazos em `1048:cad4`/`caf4`) [código]. Fora de um `_DialogWaitInit` o nível é −1, e o teste lê a entrada anterior à tabela: na bancada, nenhum desses `_myDelay` aparece. A trofalaxia ficou ~2 s na tela (contra 45 + 45 + 54 unidades ≈ 7,9 s do código) e os quadros 9100 e 9101 ~3 s cada, só o `_DialogWaitInit(3)` = 54 unidades (≈ 3 s), contra 300 + 54 unidades [hipótese apoiada pela bancada e pelo código: o prazo do nível −1 já está vencido]. O port usa `espera_tickcount` = 0 nas três sequências: a imagem sobreposta da trofalaxia aparece junto com a janela, e os sons 42 e 45 saem juntos (o 45 corta o 42).

**Quadros 9100 e 9101 (bancada, cenários `morte_aranha` e `morte_leao` de `tools/cenarios_tick.py`, 2026-10-18).** Morte pela aranha: depois da luta, a 9100 ("Your ant has been killed", com sombra: faz parte do bitmap, porque o texto só é escrito nos modos monocromáticos) e, depois do ovo no ninho, a 9101 com "You have been reborn!" em amarelo (cor 13 da tabela de cores, a do objeto `dialogText`: amarelo sobre preto) centralizado no alto do bitmap, glifos de (183, 164) a (329, 172) da área cliente. As duas janelas ficam **2 pixels à esquerda** das outras do mesmo tamanho (contorno de (154, 159) a (360, 326); o 9200, também de 200 × 161, fica em 156) [causa em aberto; `ajuste` = (−2, 0) em `mensagens.json`]. O port pede a 9100 nas causas 1 e 2 (depois da luta), a 9101 no fim de `_YellowBirth` (índice 1 ao renascer de um ovo, `rules/amarela.rs::nascer_de_ovo`; 2 ao fundar uma colônia, `fundar_colonia`, não conferido na bancada) com `Evento::QuadroAmarela { imagem, indice }`; ids em `amarela.json` → `quadros` (opcional, API 0.24). Comparação (`simant` teste `janela_26_como_no_original` com `SIMANT_SAIDA_TESTE`, `tools/comparar_captura.py --mapear --regiao` na janela com a linha de fora): 9100 contra `le_010` **100%**, 9101 contra `le_018` **100%**, trofalaxia contra `trof_030` **100%** (agora com a linha preta de fora, que a primeira medida não viu porque o fundo era preto).

**Geometria com outros bitmaps (bancada).** A janela 26 do banco tem o objeto `dialogPic` de 1 × 1 em (153, 85); `_win_SetObjBitmap` troca o bitmap e a janela abre no tamanho dele [código]. Medido com os bitmaps 9110 (199 × 161), 9200 (200 × 161), 12000 (189 × 171) e 16200 (254 × 199): o canto do bitmap fica sempre em (160, 163) da área cliente, a área mostrada é o bitmap menos 1 pixel em cada eixo, com o contorno de 4 pixels azul-escuro (cor 4) e **1 pixel preto por fora** (`avisos::geometria_janela_imagem`; `cor_externa`). Com outra janela por cima (o quadro de texto da aranha), o contorno fica **branco** (`cor_borda_inativa`, cor 15), como uma moldura de janela inativa.

## 11. Quadros animados da janela 26 (`_SpiderDialog`, `_LionDialog`, `_DrawSimPayoff`)

Os três usam a mesma janela 26 (`dialogPicWin`) do `_YellowDialog`, com outro bitmap no objeto `dialogPic`, e um laço que troca quadros desenhados por cima, em tempo real, sem rodar a simulação [código]. Relógios: `_TickCount` (55 ms) e `_MacTickCount` = `_TickCount` × 3 (degraus de 3; regras/quintal.md, "Relógios"). Um prazo de *n* unidades do Mac vence no primeiro `_TickCount` em que o relógio do Mac o alcançou, isto é, ⌈*n*/3⌉ unidades de `_TickCount` depois.

### 11.1 `_SpiderDialog` (`1000:673a`): a aranha come a amarela

Chamado por `_YellowDeath(1)` só com Options → Events (`_OptionStates[3]`) [código]:

1. Abre a janela 26 com o bitmap 12000 (189 × 171) e captura o mouse; `_myBeginSound(0x2d)` (45); `_DialogWaitInit(8)` = 144 unidades de `_TickCount`.
2. Laço até o prazo, um clique ou Espaço (`_DialogAbortOrCont`, `_win_Events`): a cada 8 unidades do Mac, o contador de quadros avança (0–3, voltando a 0 com novo som 45) e o quadro `12001 + tabela[contador]` (98 × 80) é desenhado em (+46, +77) do canto do bitmap. A tabela fica em `8:87fc` = (0, 1, 2, 1) [lida do `SIMANTW.EXE` e da memória na bancada]: os quadros saem 12002, 12003, 12002, 12001 (som), 12002… O primeiro sai 8 unidades do Mac (3 de `_TickCount`) depois de abrir.
3. Fim do laço: o último quadro de novo; **um sorteio `_SRand1(2)`** escolhe o texto: 0 → lista 10047 ("Your guts are dissolved and…"), senão 10048 ("You writhe in burning agony…"); `_StopSimulation`, `_PictureDialog` com as linhas do texto, sem imagem e forçado (o quadro de mensagem de §3), `_RestartSimulation`.
4. **A janela 26 não é fechada** (não há `_win_Close`): continua na tela, com o último quadro, embaixo do texto e durante a luta (`_AnimYellowFight`), até o `_YellowDialog(9100)` reabri-la com o bitmap 9100 e fechá-la [código; bancada].

O `_SRand1(2)` é do gerador S e acontece **dentro do tick**, antes do `_SRand1(5)` da música da morte (rng.md). No port ele é feito pelas regras (`rules/amarela.rs::matar_amarela`), que pedem `Evento::QuadroAranha(texto)` já com o texto sorteado (ids em `amarela.json` → `quadros.textos_aranha`). Sem Events, nem quadro nem sorteio, como no original.

**Bancada (cenário `morte_aranha`, snapshot `rico`, 2026-10-18):** amarela na superfície e a aranha uma célula abaixo, no estado de comer com alvo = amarela. Sequência no relógio do host: animação da aranha de 1,0 a 8,7 s (o `_DialogWaitInit(8)` = 7,9 s), quadro de texto 10048 de 9,4 a 23,2 s (fechou pelo prazo de 14,85 s, com o último quadro da aranha atrás e o contorno dela branco), a luta (rápida demais para as capturas), a 9100 de 23,95 a 26,9 s, o ovo no ninho e a 9101 de 29,6 a 32,4 s. A diferença entre duas capturas do laço fica toda em (206, 242)–(301, 317): o quadro em +46/+77. Port contra as capturas: janela da aranha com o quadro 12002 contra `ar_001` **100%**, janela inativa embaixo do texto contra `ar_020` **99,98%** (uma linha na borda do quadro de texto).

### 11.2 `_LionDialog` (`1000:80a0`): a formiga-leão come a amarela

Chamado por `_YellowDeath(2)` só com Events [código]:

1. Abre a janela 26 com o bitmap 9200 (200 × 161); `_myBeginSound(0x26)` (38) e **espera o som acabar** num laço sem eventos (`_mySoundIsDone`): o efeito 38 tem 2880 amostras a 4096 Hz = 703 ms [medido no banco SOUND].
2. Laço por 300 unidades do Mac (≈ 5,5 s), ou até um evento: a cada 30 unidades do Mac (10 de `_TickCount`), desenha o quadro `9200 + n` com n = 1, 2, 3, 2, 3, 2, 3… no canto do bitmap, com `_myBeginSound(0x25)` (37) quando n é ímpar. Fora do modo colorido (`_displayType` ≠ 10) os quadros ganham os deslocamentos da tabela `8:895c` = ((0, 0), (38, 39), (38, 52), (38, 52)); no modo colorido, nenhum [código; tabela lida na bancada].
3. Fecha a janela e redesenha tudo. São 9 quadros em 100 unidades de `_TickCount` depois do som.

Não há sorteio. No port: `Evento::QuadroFormigaLeao`.

**Bancada (cenário `morte_leao`):** a formiga-leão 0 posta na célula da amarela (na superfície). Janela do leão de 1,06 a 5,95 s (300 do Mac = 5,5 s), 9100 de 6,7 a 9,5 s, ovo, 9101 de 12,3 a 15,0 s. Port contra `le_001` **100%**.

### 11.3 `_DrawSimPayoff` (`1000:cda8`): vitória

Chamado pela vitória do Jogo Completo (fim de `_SimColonies`, depois da música 0x4e25 e do quadro 10057, com `_IsGameOver` = 1 e `_BlackWon` = 1; regras/quintal.md) e pela trapaça `fred` (`_CheatKeys` caso 0x14; regras/trapacas.md) [código]. Não olha Events.

1. `_MapToYard` (o quintal toma o lugar do mapa, ou vem para a frente), `_SetYardMode(0)` e `_SetMapPlane(0)` se preciso, `_DrawYard` e `_myDelay(0x96)` (volta logo: §10).
2. Abre a janela 26 com o bitmap 16200 (254 × 199).
3. Laço até 600 unidades do Mac ou `_DialogWaitInit(10)` = 180 unidades de `_TickCount` (o que vencer antes: 180 ≈ 9,9 s), ou um clique/Espaço: pede as músicas 20003 e 20004 em sequência (`_mySongIsDone` é imediato no MCI do Windows, então a segunda corta a primeira na volta seguinte; regras/som.md); a cada 8 unidades do Mac desenha o quadro `16210 + k` (13 × 13: os olhos em espiral), com k = 1, 2, 0, 1, 2…, em seis posições a partir do canto do bitmap, da tabela `1048:15d8` (pares (y, x)) [lida do `SIMANTW.EXE` e da memória]: (72, 74), (87, 66), (175, 64), (191, 66), (159, 122), (179, 124) em (x, y).
4. `_myDelay(300)` (volta logo) e fecha a janela.

**Bancada (cenário `payoff`, trapaça `fred` digitada no snapshot `rico`):** a janela aparece em menos de 1 s, fica ~10 s (o `_DialogWaitInit(10)`) e some; o `GameTime` não anda. Durante a janela a edição passa do ninho para a superfície, o mapa vira o quintal (no mesmo lugar) e a faixa vira a do quintal (35); ao fechar, a faixa volta à da edição (34) e a edição continua na superfície. A diferença entre duas capturas do laço cai exatamente nas seis posições. Port contra `pay_001`: **99,78%** (o resto é o cursor do mouse). As músicas não foram ouvidas (a bancada não tem placa de som).

No port: `Evento::AnimacaoVitoria`; a interface aplica o Ctrl+Y (modo 0 do quintal, plano 0, quintal na frente, `modos::aplicar_acao_quintal`) e mostra o quadro; ao fechar, a faixa volta à 34.

### 11.4 Port

`engine/src/core/quadros_animados.rs` (roteiro: fases antes, som, laço e final, no `_TickCount` da partida), `core/avisos.rs` (`Modal::Animado`, desenho, a janela que a aranha deixa aberta), dados em `mensagens.json` → `quadros_animados` (bitmaps, quadros, posições, intervalos, sons e músicas; opcional, API 0.24). A simulação fica parada enquanto o quadro está na tela (como os outros avisos). Um clique em qualquer lugar ou uma tecla encerra a espera da vez (`Animado::pular`); a espera do som do leão não é interrompida. **Ordem com a luta:** num mesmo tick, o quadro da aranha (ou da formiga-leão) e o texto da aranha vêm antes da animação da luta, e o 9100 depois: a `Partida` segura a animação até fecharem os avisos pedidos antes dela (`modais_antes_da_animacao`). Testes: `core::quadros_animados::testes`, `core::partida::testes::quadro_da_aranha_vem_antes_da_luta_e_a_morte_depois`, `regras_amarela.rs` (`quadros_da_morte_pela_aranha_e_pela_formiga_leao`, `renascer_de_ovo_pede_o_quadro_9101`), `trapacas_originais.rs` (`fred_pede_a_animacao_da_vitoria_e_nao_muda_o_estado`), `simant` (`janela_26_como_no_original`, `quadro_da_trofalaxia_como_no_original`).

**Não conferido:** os sons e as músicas (sem placa de som na bancada); a 9101 da fundação de colônia (índice 2); os quadros da luta entre a aranha e a 9100 (rápidos demais para as capturas); a causa dos 2 pixels da 9100/9101; o tempo exato dos quadros (o relógio da VM TCG é aproximado; os prazos de 7,9 s, 9,9 s e 14,85 s bateram).
