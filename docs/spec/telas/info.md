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
  - `TEXT`: topo, esquerda, base, direita, `n`, e `n` ids de texto. Cada texto (recurso tipo 10) é escrito no retângulo com os estilos do recurso tipo 21 do mesmo id (ver "Texto" abaixo). Nos 53 cartões, todo `TEXT` tem um texto só [dados].
  - Outra etiqueta: fim (o original registra "Unknown Resource").
- **Limpeza do texto** (`_DisplayCard`) [código]: o recurso tipo 10 pode ter um byte 0 no meio, seguido de bytes de controle (restos do Mac: `socia\0\0\x01l insects` no texto 310, `h\0\0\x01\0ave` no 420, dois cortes no 212). Enquanto houver um 0 antes dos 4 últimos bytes do recurso, o original tira esse 0 e os bytes seguintes menores que 0x20 (comparação de `char` com sinal: bytes ≥ 0x80 logo depois também saem), e recua os inícios dos trechos de estilo que vinham depois. O texto termina no primeiro 0 que sobra. **Esse era o defeito do port** (relatado nos cartões Ants, Mouth/"Mandibles" e Soldier): o port parava no primeiro 0 e cortava o texto.
- **Estilos** (tipo 21, o `styl` do Mac): número de trechos (16 bits) e, por trecho, 20 bytes: início (32 bits), altura da linha, ascendente, fonte, faces (byte) + 1 byte, tamanho, cor (RGB de 16 bits). O trecho usa a segunda fonte quando a palavra das faces vale 0x0100 (só negrito) [código: comparação com 0x100 em `_win_PrintStyleTextInRect`]. Faces nos cartões [dados]: 0 (108 trechos), 1 = negrito (55), 4 = sublinhado (8, desenhados com a fonte normal, sem sublinhado).
- **Texto** (`_win_PrintStyleTextInRect`, `1018:5368`) [código]: começa em (esquerda + 1, topo + 4); no máximo (base − topo) / altura linhas, com altura = a maior das duas fontes. Cada linha soma a largura de cada caractere enquanto a soma fica **menor** que a largura do retângulo, guardando o último ponto de quebra de um caractere que coube: antes de espaço, '(', '[', '{' e de fim de linha (0, '\n', '\r'); depois de ')', ']', '}', '?', '!', ',', '-', '.'. Fim de linha e fim do texto (o 0 também é medido) encerram a linha. A linha acaba no ponto de quebra (sem ponto: no caractere que passou da largura, inclusive); depois pulam-se os espaços e um fim de linha. Na entrada de um trecho de estilo, a palavra que começa ali é medida com a fonte de antes: se cabe, o que veio antes é escrito e a fonte troca; se não, a linha acaba um caractere antes. Consequência: a palavra só fica na linha se ela **e o espaço seguinte** couberem (o "using" de j03).
- **Links do texto** [código + dados]: com o último parâmetro ligado, cada trecho negrito escrito é posto em maiúsculas e procurado numa tabela de entradas de 0x22 bytes (nome de 32 + id) em `DGROUP:a458`; achando, registra a área na tabela `DGROUP:a1b2` (contador `a432`). A tabela `a458` está **zerada** no `SIMANTW.EXE` (segmento 10, deslocamento 0xa458 do arquivo + 131072) e o decompilado não tem escrita nela: o original procura, não acha e registra só "ERROR: Cannot find text rez". Logo **as palavras em negrito não abrem popup** no original [código + dados; não testado clicando na bancada]. O port também não registra links.
- Fontes: a 2 do jogo para o texto e a 5 para os trechos destacados (`_PopUpInfoWindow` chama `_win_GetStyleTextHeight(…, 2, 5)`) [código; o corpo do cartão com as mesmas fontes bate pixel a pixel com a bancada].

Exemplo, cartão 128 [dados]: PICT 21005 (faixa de baixo, (180, 0)–(200, 320)) com uma área (184, 128)–(197, 192) de valor 1 (PREVIOUS); PICT 151 (a formiga) em (32, 109); PICT 150 em (29, 4) com 8 áreas para os cartões 134, 130, 157, 139, 146, 152, 167 e 171; TEXT (1, 176)–(180, 320) com o texto 150.

## 4. Cliques (`_ProcInfoEvent`, `1018:65dc`) [código]

Só em `infoArea`. Primeiro as áreas dos `PICT`: valor 1 = volta no histórico (anel de 16 entradas, só se a anterior ≠ 0); valor > 1 = vai para o cartão com esse id (guarda no anel e zera a entrada seguinte); valor ≤ 0 = popup da definição −valor (27 áreas nos cartões, ids 911–967, todas com texto). Senão, os links do texto (vazios, ver acima).

**Popup** (`_PopUpInfoWindow`, `1018:610c`) [código + bancada p911/p912]: texto tipo 10/21 do id, enquanto o botão fica apertado (`_StillDown`); ao soltar, a janela some. Contas, relativas a `infoArea` (que, aqui, começa 1 pixel à esquerda da origem do desenho do cartão e mede 319 × 199 [medido]):

- largura do texto `L` = metade da largura da área (159); centrada no meio da área clicada; presa a 4 pixels da borda (à esquerda: de 4 a 4 + `L` + 8; à direita: até largura − 4, começando 8 antes);
- altura `h` = linhas (`_win_GetStyleTextHeight`: a mesma quebra, mas a fonte troca ao alcançar o trecho, sem medir a palavra) × altura + 5;
- abaixo da área clicada (topo = base da área + 4); se altura da área − 10 < `h` + base + 4, acima (base = topo da área − 4);
- janela filha (`WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_DLGFRAME`, classe "GenericWindow") com x = esquerda − (`SM_CXDLGFRAME` + 4), largura = `L` + `SM_CXDLGFRAME` + 4 (a margem só entra uma vez: a janela acaba no fim do texto), y = topo − (`SM_CYDLGFRAME` + 4), altura = `h` + 2 × (`SM_CYDLGFRAME` + 4), mantida dentro da área na vertical; `SM_C?DLGFRAME` = 4 na bancada. A borda vista tem 5 pixels: 1 preto + 4 azuis; fundo branco; texto no retângulo (4, 4) da área cliente, largura `L` (passa da área cliente e é recortado).

## 5. No port

`engine/src/core/info.rs` e `data/info.json`: `Carta::ler` (tipos 17/19), `TextoCartao` (tipos 10/21 com a limpeza), `compor` (quebra de `_win_PrintStyleTextInRect`), `contar_linhas` (`_win_GetStyleTextHeight`), `JanelaInfo` (carta atual, histórico de 16, cliques nas áreas, popup aberto até `soltar`, chamado no botão solto em `simant/src/jogo.rs`). A janela entra na pilha das janelas de controle (`Controle::Info`); a função 3 dos botões e Ctrl+I (extensão do port) a abrem. **Fora:** os links das palavras em negrito (o original também não os tem, ver acima); o sublinhado (face 4) não é desenhado [o original também passa a face só para escolher a fonte]; o popup do port não suspende o resto da interface como o laço de `_StillDown` [hipótese de impacto pequeno].

**Medidas do port contra as capturas** [bancada]: a janela fica 1 pixel à direita da conta comum das janelas de controle (`janelas.json` → `info.ajuste`); a linha de cima de `infoArea` é a linha do título (o fundo do cartão começa uma abaixo); as figuras mono dos cartões têm o bit 1 como tinta preta (`inverter_mono`) e a 2/150 tem linhas alinhadas a byte (corrigido em `formats/bitmap.rs`); o popup usa a área 1 pixel à esquerda e 1 menor (`popup.deslocamento_area`, `popup.ajuste_area`).

## 6. Verificação (2026-10-18)

Roteiro `tests/referencias/roteiros/qemu_info_cartas.txt` (cópia do overlay `quick` em `re_trabalho/info_cartas/`, save PORT50): capturas `c128`, `c134` (Social Insects), `c135` (Ants), `c136` (Termites), `c130` (Ant Anatomy), `c131` (Head), `c160` (Mouth), `c139` (Castes), `c141` (Soldier) e os popups `p912` (Ocelli, abre abaixo) e `p911` (Maxillae, abre acima). Port: `cargo run -p engine --example exportar_info -- JOGO SAIDA --tela 280 222 128 134 … 131:912 131:911` e `comparar_captura.py --mapear --regiao 280 223 599 422` (área do cartão): **99,65–99,75%** em todos os 9 cartões; popups com a região (270, 223)–(599, 422): **99,29%** e **99,39%**. As diferenças restantes são o cursor do mouse na captura e a última linha da área (borda de baixo da janela). Antes da correção: Top Card 96,36% (quebra de "using") e os textos de Ants, Mouth e Soldier cortados no primeiro 0.

Conferência automática dos 53 cartões: `exportar_info JOGO SAIDA --conferir` (todos com figura legível, textos não vazios depois da limpeza e popups com texto): sem problemas.

Testes: `core/info.rs` (registros do cartão 128, limpeza do texto, faces, quebra de linha, trechos, histórico e popup, geometria dos popups p911/p912), `engine/tests/modos_reais.rs` (cartão 128 lido do banco e os 53 cartões sem texto cortado).

**Todos os cartões alcançáveis (2026-10-18):** `tools/gerar_roteiro_info.py` monta, a partir das áreas de clique, o caminho mais curto do Top Card a cada cartão e gera `tests/referencias/roteiros/qemu_info_todos.txt` (capturas em `re_trabalho/info_todos/`). São **51 cartões** alcançáveis por clique; comparados com `exportar_info … --tela 280 222` na região (280, 223)–(599, 422): **99,64–99,96%** em todos (resta a última linha da área e o cursor). Observado: o cartão 152 (Common Species) escreveu o texto só depois de ~2 s, e um clique antes disso se perdeu (a primeira gravação parou ali); com esperas de 6 s (`qemu_info_especies.txt`) os cartões 152–156 deram 99,65–99,75%.

**Não verificado:** os cartões 127 (Top Card duplicado) e 129 (Index), que nenhuma área de clique alcança a partir do 128; clique numa palavra em negrito no original.
