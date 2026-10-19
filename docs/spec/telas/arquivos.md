# Arquivos: Load, Save e Save As

Diálogos do Windows que o SimAnt abre para carregar e salvar a partida, e o que `_LoadGame` faz fora dos blocos do save. O formato do arquivo está em [../formatos/save-ant.md](../formatos/save-ant.md); a gravação, em [mensagens.md](mensagens.md) §6; os quadros e a pausa depois do Load, em [mensagens.md](mensagens.md) §6.1.

Fontes: código (`_LoadGame` `1010:9fa0`, `_SaveGame` `1010:a23e`, `_FileSelect` `1010:9920`, `_PopMsg` `1008:1878`, no decomp do Ghidra) e capturas da bancada QEMU em `re_trabalho/load/` (l1 a pergunta, l2 o diálogo de Load, l4 o Save As).

## 1. File → Load (`_LoadGame`)

1. Se `_gGameNeedsSaving` (`8:8370`) ≠ 0 — a simulação o põe em 1 a cada tick (simulacao-visao-geral.md, passo 23) —, `MessageBox` com o título **"SimAnt Load"**, o texto **"Do you want to save the current ant hill?"**, ícone de pergunta e Yes / No / Cancel [código]. Na captura l1 o botão realçado (padrão) é **No** [bancada].
   - Cancel (`IDCANCEL` = 2): volta sem carregar.
   - Yes (6): `_SaveGame(0)`, isto é, Save As com o diálogo de arquivo. Se ele falha ou é cancelado, a pergunta volta [código: o laço só sai com o save feito ou com No].
   - No (7): segue.
2. `_FileSelect` no modo Load: o diálogo comum "Load Ant Hill" (§3). Cancel → volta sem carregar.
3. Guarda o nome em `1038:9520` (o arquivo do próximo Save) e zera `_gGameNeedsSaving` [código].
4. Se o nome (sem a pasta) é o do tutorial (`Tutorial.Ant`, string 1800), gera o tutorial (geracao-mundo.md §6) em vez de ler, com o título "Tutorial Game", `_InitSimVars` e `_SeedSRand`, e segue para `_DoLoadInitializations` e a pausa como num Load [código + bancada; ver abaixo].
5. Senão: título da janela de edição = o nome do arquivo (como o diálogo o devolve, em maiúsculas); abre o arquivo; põe `_JustXfered` (`9:9c70`) = 1, `_LastThemeTime` (`9:806e`, long) = 0, `_TimeTemp` (`9:9b10`, long) = 300 e `_EditMsgDelay` (`9:9e9e`, long) = 0; roda **`_RandYard`** e só então lê os blocos da tabela `8:9570` [código].
6. `_EditMessage(0, 0, −2, −1, 1)`, `_DoLoadInitializations` (contagem, janelas, vista), `_SetMenuEntries`, `_PauseGame(1)` e, sem música ligada, `_StopSong` [código].

### 1.1 Os sorteios de `_RandYard` no Load

`_RandYard` roda com o gerador R da sessão como estava e o `_CurGameType` do jogo que estava aberto (0 no começo do programa, antes de qualquer jogo): 192 `_RRand` para as sementes dos patches e a geração do patch (11, 8), que semeia S (geracao-mundo.md). Os blocos lidos depois sobrescrevem o mundo, mas S e R ficam como `_RandYard` os deixou: **o estado dos geradores depois do Load depende de R antes do Load e do tipo do jogo anterior, não do arquivo** [código + bancada: com R = `0x568a9979` e o tipo Quick, R = `0x7e80df39` e S = `0x98b9` depois do Load, lidos com `ler_variavel` no roteiro `qemu_load_rng.txt`; teste `engine/tests/load_geradores.rs`].

`_JustXfered` está no save (bloco 297) e é sobrescrito; `_TimeTemp` = 300 sobrevive (fora do save); `_LastThemeTime` e `_EditMsgDelay` ficam zerados [código].

**Port:** `carregar::geradores_depois_do_load` (S e R) e `TEMPO_TUTOR_DEPOIS_DO_LOAD`; o resto do estado de fora do save começa zerado numa sessão nova. No começo do programa, o R do `--save` é o semeado pelo relógio (como `_initStuff`) e o tipo é 0; no Load durante a partida (Ctrl+L ou "Load Game" do Select a Game), o R e o `_CurGameType` da partida aberta.

## 2. File → Save e Save As (`_SaveGame`)

- `_SaveGame(1)` (Save, Ctrl+S): com nome guardado (`1038:9520`), grava nele sem diálogo; sem nome, como Save As [código].
- `_SaveGame(0)` (Save As, Ctrl+A, e o Yes da pergunta do Load): `_FileSelect` no modo Save ("Save Ant Hill As") [código + bancada l4].
- Depois de gravar: `_PopMsg("%s Saved correctly")` — `MessageBox` com o título **"Message"**, ícone de informação (`MB_ICONINFORMATION`, 0x40) e OK [código]. `%s` é o caminho escolhido [hipótese: o texto exato do caminho não foi capturado].

## 3. O diálogo de arquivo (`_FileSelect`)

Com `COMMDLG.DLL` disponível, `GetOpenFileName`/`GetSaveFileName` com o filtro "SimAnt Files(*.ANT)" e o nome inicial `*.ant`; sem ela, um diálogo próprio do jogo (`DIALOGBOX` "SaveAs"/outro) [código; só o comum foi observado].

Geometria medida na bancada (tela 640×480, área cliente a partir de y = 38; coordenadas da área cliente) [bancada l2/l4]:

| Elemento | Retângulo (x, y, largura, altura) | Observação |
|---|---|---|
| Moldura | (56, 15, 472, 247) | contorno preto, borda azul de 4 pixels (3 em cima), interior branco |
| Título | (62, 20)–(522, 38) | caixa do menu de sistema 18×18 e linha preta embaixo; texto em System |
| "File Name:" | campo (71, 65, 160, 20) | `*.ant` selecionado (fundo azul) |
| Lista de arquivos | (71, 90, 160, 106) | itens de 13 pixels, x + 5; no Save As, os nomes em cinza |
| "Directories:" | caminho em (254, 68); árvore (253, 90, 163, 114) | itens de 16 pixels, recuo de 4 por nível, ícones de pasta |
| "List Files of Type:" / "Save File as Type:" | combinação (72, 224, 158, 20) | "SimAnt Files(*.ANT)" |
| "Drives:" | combinação (254, 224, 161, 22) | "c:" com ícone de disco |
| OK (padrão) / Cancel | (425, 49, 88, 23) / (425, 78, 88, 23) | |
| "Read Only" | caixa 13×13 em (425, 153) | |

- Os rótulos, botões e listas usam **MS Sans Serif 8 (altura 13) em negrito simulado** — cada glifo desenhado duas vezes com um pixel de deslocamento, avanço + 1; o título usa a fonte **System** [bancada: glifos com traço de 2 pixels e a altura de linha de 13 da lista]. Letras sublinhadas (mnemônicos) na linha de base + 2.
- Barras de rolagem sem polegar (a lista cabe) com as setas desabilitadas: cinza escuro com o relevo branco deslocado (+1, +1); setas de 7 colunas nas linhas 4 a 10 do botão [bancada].
- **Lista que não cabe** (bancada, 2026-10-19, 18 arquivos, 8 visíveis; §6): trilho cinza-claro (a cor dos botões), setas pretas e um polegar quadrado (o mesmo botão em relevo das setas, 15 × 15) entre duas linhas pretas. A linha de cima do polegar vai da linha de baixo da seta de cima (posição 0) até 16 linhas acima da linha de cima da seta de baixo (posição máxima = arquivos − visíveis), proporcional e **arredondada** (posições 2, 8 e 9 de 10: 11, 46 e 51 linhas de um curso de 57). Seta: um item; trilho acima ou abaixo do polegar: uma página de visíveis − 1 (de 2 para 9). Um clique num item põe o nome no campo, sem seleção (o foco sai do campo), e realça o item com o retângulo de foco pontilhado.
- **Troca de unidade e de pasta:** "a:" e Enter no campo leva à unidade A: (o campo volta a `*.ant` selecionado, "Drives:" mostra "a:" com o ícone de disquete, a árvore só "a:\"); "c:" e Enter volta à pasta corrente do C: (`c:\maxis\antwin`, a do DOS para a unidade); duplo clique num item da árvore abre a pasta (o item fica realçado em toda a largura, com o retângulo de foco; o campo mostra `*.ant` sem seleção). Na raiz do C: a árvore mostra `c:\`, `maxis` e `windows` e a lista fica vazia, com a barra desabilitada [bancada d00–d09].
- Botões: contorno preto de cantos cortados (dois contornos no padrão), luz branca de 2 pixels em cima e à esquerda, sombra cinza escura de 2 pixels embaixo e à direita, em escada nos cantos [bancada].

## 4. A caixa de mensagem (`MessageBox`)

Medida com "SimAnt Load" [bancada l1]: 348×129 em (144, 138) da área cliente, contorno preto de 1 pixel (sem borda grossa), título de 18 linhas como o do diálogo, ícone 32×32 em (+19, +36), texto em System a partir de (+69, +44), botões 58×28 em y + 90, Yes/No/Cancel em x + 71, + 145 e + 221. A largura vem do texto (69 + largura + 18); centrada em (318, 202) [hipótese para outros textos: o Windows calcula a caixa pelo texto; só uma foi medida].

## 5. No port

- `engine/src/core/dialogos.rs` desenha e trata a caixa de mensagem e o diálogo de arquivo; `data/dialogos.json` tem a geometria, as cores, os textos e as fontes (índices de `interface.json`: 6 = `VGASYS.FON#System`, 7 = `SSERIFE.FON#MS Sans Serif@13+negrito`, ambas lidas da instalação do Windows do usuário; sem elas, a fonte 0 do jogo).
- `simant/src/arquivos.rs`: o fluxo (pergunta → Save As → "Saved correctly" → Load; Save; Save As) e a nova sessão do Load. Os diálogos são modais: a simulação não anda com um deles aberto [hipótese: o laço de mensagens do diálogo do Windows poderia despachar o `WM_TIMER`; nas capturas o jogo estava pausado]. Teclas: Ctrl+L (Load, extensão do port, que não tem barra de menus), Ctrl+S, Ctrl+A; no diálogo, Enter = OK/padrão, Esc = Cancel, letras sublinhadas nos botões da caixa, digitação no campo do nome, clique duplo na lista.
- A árvore de pastas mostra o caminho real (ancestrais e subpastas) com ícones próprios; "Drives:" mostra a letra da unidade do caminho (ou "c:" quando o caminho não tem uma); "Read Only" e as combinações não fazem nada.
- **Desde 2026-10-19:** a lista de arquivos rola como a do original (polegar, setas, página; `PartesRolagem` em `core/dialogos.rs`); a árvore também rola quando não cabe [hipótese: o original não foi rolado na árvore]; um nome de pasta no campo ("..", "sub", "d:", um caminho) e Enter troca de pasta; duplo clique na árvore abre a pasta e a deixa realçada. O port não guarda a pasta corrente de cada unidade: "a:" vai à raiz de A:. Sem janela: `--clique-dialogo X,Y[,duplo]` (repetível) clica no diálogo aberto por `--dialogo` antes de exportar.
- Sem janela: `--dialogo load|pergunta|salvar_como|salvo` abre o diálogo antes de `--exportar-tela-jogo`.

## 6. Verificação (2026-10-18)

`tools/comparar_captura.py --mapear`, port exportado com `--save … --dialogo …` numa pasta com `quick1.ant` e `quick2.ant` (como a da bancada):

| Diálogo | Captura | Região | Iguais |
|---|---|---|---|
| Load, metade esquerda | l2 | (56, 15)–(250, 262) | 99,97% |
| Load, botões e Read Only | l2 | (416, 15)–(528, 262) | 99,39% |
| Load inteiro (com a árvore de pastas, que mostra outro caminho) | l2 | (56, 15)–(528, 262) | ≈97% |
| Save As, metade esquerda | l4 | (56, 15)–(250, 262) | 99,48% |
| Pergunta do Load | l1 | (144, 138)–(492, 267) | 99,52% (o ícone é desenho próprio) |

**"Saved correctly" (bancada, 2026-10-18, `cenarios_tick.py salvo`):** o Quick Game não tem nome, então Ctrl+S abre "Save Ant Hill As"; com um nome que já existe o diálogo comum pergunta "This file already exists. Replace existing file?" (Yes/No); depois, a caixa "Message" com o ícone de informação **azul-escuro** e o texto `C:\MAXIS\ANTWIN\QUICK1.ANT` / `Saved correctly` em **duas linhas** (o caminho completo, e o Windows quebra a linha). Port (`simant --dialogo salvo`): **87,1%** na caixa (178, 137)–(471, 268). Corrigido: a cor do ícone "i" (`icone_informacao` = 4, era verde). **Corrigido (2026-10-18):** o formato no DGROUP é `"%s
Saved correctly"` (a quebra de linha é do próprio texto); o port passa o caminho completo (o real, `std::path::absolute`) e a caixa usa o estilo de moldura de diálogo medido aqui (`estilos[1]` de `data/dialogos.json`: contorno, borda azul de 4 pixels — 3 em cima —, margem branca no título), com o ícone 18 pixels à direita do interior e 17 abaixo da linha do título, o texto em x + 68 centrado na altura do ícone (aqui duas linhas = a altura dele), o OK 22 abaixo e o interior acabando 10 abaixo do botão; as mesmas regras dão a pergunta do Load (só contorno). `simant --dialogo salvo` contra `salvo.png`: **99,28%** na caixa (176, 134)–(472, 271) (o resto é o ícone, desenho próprio); a pergunta do Load continua 99,52%. O x do centro difere entre as duas caixas (318 e 324 na área cliente) e fica medido por estilo [hipótese: critério do Windows não achado].

**Load de `Tutorial.Ant` (bancada, 2026-10-18, `cenarios_tick.py load_tutorial`):** o arquivo precisa existir para o diálogo de abrir (salvei o Quick Game como TUTORIAL.ANT); carregado, o original **não lê o arquivo**: gera o tutorial (`_GenerateTutorial`, geracao-mundo.md §6) — título "Tutorial Game", `_CurGameType = 2`, `_GameTime = 0`, `_TutLesson = 1`, 64 formigas novas na superfície (`_ListIndexA` = 65), populações do quintal nos 30 patches da tabela (conferidos: 18 pretos e 12 vermelhos, os da spec), a amarela no ninho preto em (29, 10).

Como não passa por `_NewGame`, o que `_RandYard` não escreve fica como estava na partida anterior [bancada]: os modos e o modo estratégico das colônias, colônias fundadas e totais, `_SowDir[0]`, gato (`_CatX/Y/Dir`), `_DogPx/Py`, `_ColonyUpdateFlag`, `_CurEditMode`, `_GamePaused`, `_FuzLocX`, `_MeCrazyCnt`, `_MeFoodInType`, `_MeGoalPlane`, `_MakeMe`, `_ModeMe`, `_StargLife`, `_SuserX/Y` e o X/Y das vagas além do fim das listas B e R (T e M zerados); `_MeGoalX/Y` e `_MeLastX/Y` vão para a posição da amarela; `_MapPnt` é a vista centrada nela (`_SetMapPlane(_MePlane)`).

**Port:** `carregar::estado_do_tutorial` (com `data/tutorial_arquivo.json`: nome, título, os 30 patches e os valores de `_InitSimVars`), usado pelo Load do diálogo e pelo `--save` com esse nome (`Origem::Tutorial`). **Conferido:** `engine/tests/load_tutorial.rs` parte do estado de antes do vetor `tests/referencias/load_tutorial` (copiado de `re_trabalho/cen/load_tut`) e chega ao de depois com **307/307 blocos e R igual** (S vem do relógio: o teste usa o gravado).

**Muitos arquivos e troca de pasta (bancada, 2026-10-19, `roteiros/qemu_dialogo_muitos.txt`, capturas `re_trabalho/ag_fecha/dlg/d00`–`d09`):** um disquete com 18 `.ANT` (`caso_ant.montar_disquete`) no A:, Load, "a:" e Enter, setas e trilho da barra, um item, "c:" e Enter, duplo clique no "c:\" da árvore. Port numa pasta com os mesmos 18 nomes (`--dialogo load` e os mesmos cliques com `--clique-dialogo`; `comparar_captura.py --mapear`, metade esquerda (56, 15)–(250, 262)): lista no começo **99,97%**, depois de duas setas **99,81%**, depois da página **99,74%** (só o cursor do mouse sobre a barra), com um item escolhido **99,56%** (o retângulo de foco pontilhado, que o port não desenha) e a raiz do C: **100%** (lista vazia, barra desabilitada, campo sem seleção). A árvore do port mostra o caminho real do computador, não o da bancada.

**Não verificado:** a árvore com mais pastas do que cabem (a rolagem dela) e a pasta corrente por unidade.

## 7. File → Quit (`_MenuQuit`)

A mesma caixa da pergunta do Load com o título "SimAnt Quit", sempre (sem olhar `_gGameNeedsSaving`); Yes = Save As e sai se salvou, No = sai, Cancel = volta. Ver [menus.md](menus.md) §4.
