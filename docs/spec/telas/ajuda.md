# Ajuda (WinHelp) e o visor do port

O SimAnt original abre a ajuda no WinHelp do Windows: `WinHelp(hwnd, "SIMANT.HLP", HELP_CONTEXT, n)` com o número de contexto do objeto (botão "?" da faixa, modo de ajuda, botões "?" das janelas) ou `HELP_INDEX` (F1). O WinHelp é outro programa: abre uma janela própria por cima do SimAnt, e o jogo continua rodando. Quem chama e com que contexto está nas telas do jogo (faixa, janelas, teclas); aqui fica só o que o WinHelp mostra e como o port o reproduz. Formato do arquivo: [../formatos/hlp.md](../formatos/hlp.md).

Implementação: `engine/src/core/ajuda.rs` (`VisorAjuda`), dados em `data/ajuda.json`. Exportação sem janela: `cargo run -p engine --example ajuda -- <pasta do jogo> saida.png [contexto] [--topico N] [--rolar P] [--clicar X Y] [--listar]`.

## 1. O que o original mostra (bancada QEMU, 2026-10-18)

Capturas `re_trabalho/agente_ui/a02` (contexto 4, clique na edição em modo de ajuda), `a04` (botão "?" da faixa, contexto 0x2200), `a05` (modo de ajuda + botão de estatísticas, 0x2209) e `a06` (F1, índice), roteiro `tests/referencias/roteiros/qemu_ui_ajuda.txt`. Medidas na tela de 640 × 480 do original:

- Janela "SimAnt Help - SIMANT.HLP" de x 213 a 637 e y 2 a 477, sempre no mesmo lugar nas quatro capturas: moldura grossa de 4 (preto, 2 de cinza, preto) com as marcas dos cantos a 18 + 4 pixels de cada canto.
- Título (y 6–23, azul, texto branco centralizado na fonte System): caixa do menu de sistema de 18 × 18 com o traço comprido do Windows (retângulo preto de 13 × 3 com o meio branco em x +2, y +7), botões de minimizar e maximizar de 18 × 18 à direita (setas de 4 linhas), linha preta embaixo (y 24).
- Menu (y 25–42, branco; linha preta em 43): "File", "Edit", "Bookmark", "Help" na fonte System, células em x +8, +48, +89 e +170 da borda interna (217), letras sublinhadas F, E, m, H.
- Barra de botões (y 44–64): fundo cinza-escuro, seis botões de 59 × 21 colados a partir da borda interna (Contents, Search, Back, His**t**ory, `<<`, `>>`), contorno preto sem os quatro cantos, 2 linhas de luz e 2 de sombra, texto MS Sans Serif 8 pt negrito com a letra sublinhada, centralizado contando o pixel a mais do negrito. Botão sem uso (Back sem histórico, `<<`/`>>` fora de uma sequência): texto acinzentado em xadrez (só os pixels com x + y par).
- Texto do tópico (y 65–473, branco): origem do texto a 8 pixels da borda interna (x 225). Cada parágrafo começa numa linha nova; a altura da linha é a da maior fonte nela (MS Sans Serif 12 pt negrito = 20, 10 pt = 16); um parágrafo vazio ocupa a altura da fonte em vigor. Os saltos são verdes (cor 2), sublinhados 2 linhas abaixo da linha de base, na largura do texto.
- O cursor fica no WinHelp e o SimAnt fica atrás, inativo.

## 2. O visor do port

- **Lugar:** o port não tem a barra de título nem o menu do SimAnt (a área do jogo é a área cliente de 640 × 442, 37 linhas abaixo do topo da tela do original). A janela de ajuda fica em x 213–637 como no original e do topo à base da área cliente (y 0–441; `externo` em `data/ajuda.json`) [escolha do port: o original vai de 2 a 477 da tela, o que daria −35 a 440 na área cliente]. Por isso tudo fica 2 linhas acima do original em relação à moldura, e a janela tem 34 linhas a menos de texto.
- **Fontes:** título, menu: System (`VGASYS.FON`, índice 6 de `interface.json`); botões: MS Sans Serif 8 pt negrito (7); texto: as fontes do `|FONT` do arquivo pelo tamanho e pelo negrito, mapeadas em `fontes` de `data/ajuda.json` para MS Sans Serif 10 pt (8), 12 pt (9), 10 pt negrito (10), 12 pt negrito (11), do `SSERIFE.FON` do Windows do usuário (o WinHelp troca a "Helv" do arquivo por ela).
- **Navegação:** clique num salto troca o tópico e guarda o anterior para o Back; Contents vai ao índice (contexto 0x100); `<<` e `>>` seguem a sequência do cabeçalho do tópico; Back volta. Teclas: Esc fecha, setas/PageUp/PageDown/Home/End rolam, Backspace = Back, a letra sublinhada de um botão aciona o botão. Barra de rolagem vertical quando o texto não cabe (setas: 16 pixels; trilho: uma página).
- **Fechar:** Esc ou duplo clique na caixa do menu de sistema (o menu de sistema e o menu File/Edit/Bookmark/Help não abrem) [escolha do port].
- **Abrir:** `VisorAjuda::abrir_contexto(n)` (HELP_CONTEXT; zera o histórico [hipótese]) e `abrir_indice()` (HELP_INDEX). Um contexto que não existe deixa o visor como estava.
- **Figuras:** as `bmN` (DIB de 16 cores) são convertidas para a cor mais próxima entre as 16 do Windows da paleta do port; ficam na linha do texto com a base na linha de base [hipótese: posição não comparada com o original].

## 3. Conferência

`cargo run -p engine --example ajuda` com os contextos das capturas, comparado na janela inteira (x 213–637, y 0–435 do port contra y 2–437 do original, cores do QEMU mapeadas para as 16 do Windows): índice (a06) **99,94%**, 0x2200 (a04) **99,94%**, 0x2209 (a05) **99,94%**, 4 (a02) **99,97%**. O que sobra: o cursor do mouse nas capturas, um pixel do `>>` acinzentado e a linha de marcas da moldura de baixo (que no port fica mais acima). Teste: `engine/tests/ajuda_reais.rs` (contextos, figuras, saltos e botões do visor).

## 4. Não reproduzido

- Search (diálogo de busca por palavra-chave, `|KWBTREE`) e History (lista dos tópicos vistos): os botões existem mas não fazem nada (divergencias.md).
- Popups (0xE0, 11 no SIMANT.HLP, contra 369 saltos 0xE1): o port troca o tópico em vez de abrir a janela pequena do WinHelp.
- Menus File/Edit/Bookmark/Help, menu de sistema, mover/redimensionar a janela, a janela ir para trás do jogo.
