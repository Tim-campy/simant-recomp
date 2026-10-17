# Info ("Top Card", janela 5)

Uma enciclopédia estática sobre formigas, em cartões com figuras, textos e áreas de clique. Não lê o estado do jogo nem sorteia. Fontes: pseudocódigo do Ghidra (análise de 2026-10-18), os recursos do SHARED/WINGANT e a captura `re_trabalho/janelas/j03_info.png`.

## 1. Abrir (`_OpenInfoWindow`, `1018:60da`) [código]

Pelo menu Window → Info (`_ProcMenu` comando 0x16), pela função 3 dos botões configuráveis (`_DoUserButton`) e pelo objeto 0x115 da janela do mapa. Se a janela 5 ainda não está aberta: carta atual (DGROUP `a456`) = 128, histórico[0] (`a434`) = 128, índice do histórico (`a454`) = 0. Depois, sempre `_win_Open(0x500)` (aberta, mantém a carta).

## 2. Janela 5 (`infoWin`) [dados]

Objeto 0: contorno, 324 × 220, entrada 5 da tabela tipo 9 em (277, 128) (+ a faixa). `infoTitle` (0x501, tipo 18): a barra de título; `infoArea` (0x502, tipo 2): a área do cartão (320 × 200).

## 3. O cartão (`_win_DrawInfoWindow` `1018:534e` → `_DisplayCard` `1018:5a1c`) [código + dados]

53 cartões, ids 127 a 179 (128 "Top_Card", 129 "Index", 130 "Ant_Anatomy" …). Para o cartão `id`:

- **Título**: o recurso tipo 19 `id` (texto com '_' no lugar dos espaços), escrito em `infoTitle`.
- **Corpo**: o recurso tipo 17 `id`, com inteiros de 16 bits **big-endian** (herança do Mac): 4 bytes de cabeçalho (altura e largura do cartão, 200 e 320) e uma sequência de registros com etiqueta de 4 letras:
  - `PICT`: bitmap, topo, esquerda, base, direita, `nHot`, e `nHot` × (topo, esquerda, base, direita, valor). Desenha o bitmap (tipo 2, de qualquer banco) em `infoArea` + (esquerda, topo) e registra as áreas de clique.
  - `TEXT`: topo, esquerda, base, direita, `n`, e `n` ids de texto. Cada texto (recurso tipo 10, ASCII terminado em 0) é escrito com quebra de linha por palavra no retângulo (da esquerda até a direita, a partir do topo; a base não limita), com os estilos do recurso tipo 21 do mesmo id.
  - Outra etiqueta: fim (o original registra "Unknown Resource").
- **Estilos** (tipo 21, o `styl` do Mac): número de trechos (16 bits) e, por trecho, 20 bytes: início (32 bits), altura da linha, ascendente, fonte, estilo (byte; 1 = negrito), tamanho, cor (RGB de 16 bits). No cartão 128: três trechos, o do meio ("Highlighted") em negrito. Os trechos com estilo usam a segunda fonte do cartão ("palavras destacadas") e são registrados como links (tabela `DGROUP:a1b2`) [código]; o nome da palavra é procurado numa tabela de entradas de 0x22 bytes (`DGROUP:a478`) cujo preenchimento ficou [em aberto].
- Fontes: a 2 do jogo para o texto e a 5 para os trechos destacados (`_PopUpInfoWindow` mede com as duas) [código; hipótese de que o corpo usa as mesmas].

Exemplo, cartão 128 [dados]: PICT 21005 (faixa de baixo, (180, 0)–(200, 320)) com uma área (184, 128)–(197, 192) de valor 1 (PREVIOUS); PICT 151 (a formiga) em (32, 109); PICT 150 em (29, 4) com 8 áreas para os cartões 134, 130, 157, 139, 146, 152, 167 e 171; TEXT (1, 176)–(180, 320) com o texto 150.

## 4. Cliques (`_ProcInfoEvent`, `1018:65dc`) [código]

Só em `infoArea`. Primeiro as áreas dos `PICT`: valor 1 = volta no histórico (anel de 16 entradas, só se a anterior ≠ 0); valor > 1 = vai para o cartão com esse id (guarda no anel e zera a entrada seguinte); valor ≤ 0 = popup da definição −valor. Senão, os links do texto abrem o popup da palavra.

**Popup** (`_PopUpInfoWindow`, `1018:610c`): texto tipo 10/21 do id, numa janela branca centrada na palavra (abaixo ou acima, dentro de `infoArea`), enquanto o botão fica apertado (`_StillDown`).

## 5. No port

`engine/src/core/info.rs` (`Carta::ler` para os tipos 17/19/10/21, `JanelaInfo` com a carta atual, o histórico de 16 e o clique nas áreas) e `data/info.json`; a janela entra na pilha das janelas de controle (`Controle::Info`) com a moldura e o título delas; a função 3 dos botões e Ctrl+I (extensão do port) a abrem. **Fora:** os popups de definição e os links do texto (a tabela de palavras está em aberto); os estilos só trocam a fonte (negrito → fonte 5); a quebra de linha é a do port (por palavra, na largura do retângulo).

**Medidas do port contra a captura j03** [bancada]: a janela fica 1 pixel à direita da conta comum das janelas de controle (`janelas.json` → `info.ajuste`); a linha de cima de `infoArea` é a linha do título (o fundo do cartão começa uma abaixo); o texto começa 1 pixel à direita e 4 abaixo do retângulo do `TEXT` (`deslocamento_texto`); as figuras mono dos cartões têm o bit 1 como tinta preta (`inverter_mono`) e a 2/150 tem linhas alinhadas a byte (corrigido em `formats/bitmap.rs`).

## 6. Verificação (2026-10-18)

`simant --save re_trabalho/janelas/j00_estado.ANT --botoes 3 --exportar-tela-jogo …` contra `j03_info.png` (`comparar_captura.py --mapear`): janela inteira (280, 204)–(600, 424) **96,36%**; a metade esquerda (figuras, botões dos tópicos, PREVIOUS) (280, 204)–(455, 424) **99,90%**. A diferença está na quebra de linha do texto a partir da sétima linha: o original quebra "can always back up by / using…", o port põe "using" na mesma linha [causa em aberto: a medida de largura do TextEdit do Mac].

Testes: `core/info.rs` (registros do cartão 128, histórico), `engine/tests/modos_reais.rs` (cartão 128 lido do banco: título e as três figuras decodificáveis).

**Não verificado:** os outros 52 cartões, os popups e a navegação na bancada.

