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
4. Se o nome (sem a pasta) é o do tutorial (`Tutorial.Ant`, string 1800), gera o tutorial (geracao-mundo.md) em vez de ler [código; o port não trata esse nome].
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
- Botões: contorno preto de cantos cortados (dois contornos no padrão), luz branca de 2 pixels em cima e à esquerda, sombra cinza escura de 2 pixels embaixo e à direita, em escada nos cantos [bancada].

## 4. A caixa de mensagem (`MessageBox`)

Medida com "SimAnt Load" [bancada l1]: 348×129 em (144, 138) da área cliente, contorno preto de 1 pixel (sem borda grossa), título de 18 linhas como o do diálogo, ícone 32×32 em (+19, +36), texto em System a partir de (+69, +44), botões 58×28 em y + 90, Yes/No/Cancel em x + 71, + 145 e + 221. A largura vem do texto (69 + largura + 18); centrada em (318, 202) [hipótese para outros textos: o Windows calcula a caixa pelo texto; só uma foi medida].

## 5. No port

- `engine/src/core/dialogos.rs` desenha e trata a caixa de mensagem e o diálogo de arquivo; `data/dialogos.json` tem a geometria, as cores, os textos e as fontes (índices de `interface.json`: 6 = `VGASYS.FON#System`, 7 = `SSERIFE.FON#MS Sans Serif@13+negrito`, ambas lidas da instalação do Windows do usuário; sem elas, a fonte 0 do jogo).
- `simant/src/arquivos.rs`: o fluxo (pergunta → Save As → "Saved correctly" → Load; Save; Save As) e a nova sessão do Load. Os diálogos são modais: a simulação não anda com um deles aberto. Teclas: Ctrl+L (Load, extensão do port, que não tem barra de menus), Ctrl+S, Ctrl+A; no diálogo, Enter = OK/padrão, Esc = Cancel, letras sublinhadas nos botões da caixa, digitação no campo do nome, clique duplo na lista.
- A árvore de pastas mostra o caminho real (ancestrais e subpastas) com ícones próprios; a unidade é sempre "c:"; "Read Only" e as combinações não fazem nada.
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

**Não verificado:** a caixa "Saved correctly" (sem captura), o comportamento com muitos arquivos (rolagem), o arquivo `Tutorial.Ant`, a mudança de pasta ou de unidade no diálogo.
