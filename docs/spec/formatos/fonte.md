# Fontes e texto das janelas

O jogo usa dois tipos de fonte, escolhidos por `_font_SetFont(n)` (`1008:611c`) a partir do byte "fonte" (0x28) dos objetos de texto:

| n | Fonte | Desenho |
|---|---|---|
| 0 | **Gaia8x14**, do `FONTRES.FON` | GDI: `AddFontResource("fontres.fon")` e `CreateFont(altura 14, "Gaia8x14")` em `_GSetBigFont` (`1008:1664`), depois `TextOut` |
| 1 | **Gaia8x8**, do `FONTRES.FON` | GDI: idem com altura 8 (`_GSetSmallFont`, `1008:1560`) |
| 2–5 | arquivos `font1`–`font4` da pasta do jogo (formato `FONT` do Macintosh) | renderizador próprio (`_font_MakeImage` + `_DrawChar`) |
| < 0 ou > 5 | tratado como 2 | |

`_font_InitFonts` (`1008:616c`) também tenta `font5` e `font6`, que não existem na instalação. Uso nos layouts do `WINGANT`: a fonte 0 aparece em 15 objetos de texto, a 3 em 6, a 4 em 4 e a 2 em 1.

## `font1`–`font4`: formato FONT do Macintosh

**Fonte:** `_font_ReadFont` (`1030:aa4c`) e verificação dos quatro arquivos: os tamanhos batem e `owTLoc` aponta exatamente para a tabela de larguras e deslocamentos. Valores em **big-endian**.

| Offset | Campo (nome Mac) | font1 | font2 | font3 | font4 |
|---|---|---|---|---|---|
| 0x00 | `fontType` | 0x9801 | 0x9001 | 0x9000 | 0x9001 |
| 0x02 | `firstChar` | 0 | 0 | 0 | 0 |
| 0x04 | `lastChar` | 255 | 255 | 216 | 255 |
| 0x06 | `widMax` | | | | |
| 0x08 | `kernMax` | −2 | 0 | 0 | −1 |
| 0x0A | `nDescent` | | | | |
| 0x0C | `fRectWidth` | | | | |
| 0x0E | `fRectHeight` | 10 | 10 | 15 | 11 |
| 0x10 | `owTLoc` (em palavras, a partir de 0x10) | | | | |
| 0x12–0x16 | `ascent`, `descent`, `leading` | 8/2/1 | 8/2/1 | 12/3/1 | 9/2/0 |
| 0x18 | `rowWords` | 56 | 55 | 68 | 68 |
| 0x1A | faixa de glifos: `rowWords·2` bytes por linha × `fRectHeight` linhas, 1 bit por pixel, bit mais significativo à esquerda | | | | |
| … | `locTable`: (`lastChar − firstChar + 3`) u16, posição x de cada glifo na faixa | | | | |
| … | `owTable`: mesma contagem; byte alto = deslocamento, byte baixo = avanço; 0xFFFF = caractere ausente | | | | |
| … | 516 bytes extras em três arquivos (tabela de larguras do Mac), **não lidos** pelo jogo | | | | |

`fontType & 0x2000 = 0` marca fonte proporcional (todas as quatro).

### Desenho de uma string (`_font_MakeImage`, `1030:ade0`; `_DrawChar`, `1030:b033`)

- Caractere ausente (`owTable[c] = 0xFFFF`) **mede** como o glifo de índice `lastChar − firstChar + 1` (`__font_StringWidth`, `__font_CharWidth`), mas `_font_MakeImage` lê esse índice como **byte** (`*(byte *)(fonte + 0x28)`) e desenha o glifo `(lastChar − firstChar + 1) & 0xFF`, andando o avanço dele: nas fontes com `lastChar` = 255 (`font1`, `font2`, `font4`) é o glifo 0, vazio, e o ausente não aparece (a imagem fica com a largura medida). Na `font3` (`lastChar` 216) é o próprio glifo 217. [código; conferido no balão do menino nº 4, que tem um 0x7F: telas/quintal.md §2.3, 2026-10-19.] A tabela é indexada pelo próprio código do caractere (o `firstChar` é 0 em todas).
- **Avanço** = byte baixo de `owTable[c]`, **mais 1 se `kernMax < 0`**.
- **Deslocamento x** do glifo = byte alto + `kernMax` se `kernMax ≥ 0`; se `kernMax < 0`, o deslocamento é 0. É uma particularidade do código, não a regra do Mac.
- O glifo é a coluna `locTable[c] .. locTable[c+1]` da faixa. O jogo copia a partir da **linha 1** da faixa (a linha 0 é ignorada), com `fRectHeight − 1` linhas. A altura da imagem e `__font_FontHeight` valem `fRectHeight − 1`.
- Os glifos são combinados por OU numa imagem de 1 bit, de largura igual à soma dos avanços. A imagem é desenhada **opaca** (`_DoMonoBitmap` → `SetDIBitsToDevice`): bit 1 na cor de frente, bit 0 na cor de fundo.

## `FONTRES.FON`: Gaia8x8 e Gaia8x14

Executável NE com dois recursos `FONT` no formato `.FNT` versão 2.0 do Windows (cabeçalho de 118 bytes; tabela de caracteres com `(largura u16, offset u16)`; glifos armazenados coluna a coluna, 8 pixels por coluna, um byte por linha). Autor: "Rodney S.K. Lai", 1991.

| Recurso | Nome | Célula | Ascent | Caracteres | Padrão |
|---|---|---|---|---|---|
| FONT 1 | Gaia8x8 | 8×8 | 6 | 32–255 | 128 |
| FONT 2 | Gaia8x14 | 8×14 | 12 | 32–255 | 128 |

`TextOut` desenha a célula inteira com fundo **opaco** (`SetBkMode(OPAQUE)` em `_GPutStr`, `1008:1792`), canto superior esquerdo em (x, y). Para as fontes GDI, `_fontHeight` = `tmHeight` + `tmExternalLeading` (14 na Gaia8x14) e `_fontWidth` = média de `tmAveCharWidth` e `tmMaxCharWidth` (8).

## Posição do texto num objeto (`_gr_JustifyStrInRect`, `1030:b300`)

Com o retângulo (L, T, R, B), a largura W da string e a altura H da fonte:

| Alinhamento (bits 7–8 das flags do objeto) | x |
|---|---|
| 0 ou 3 | (L + R − W) / 2 |
| 1 | L + 4 |
| 2 | R − W − 4 |

x nunca fica menor que L. y = (T + B − H) / 2 (divisão inteira). No alinhamento 3, o resto da linha à esquerda e à direita do texto é preenchido com a cor de fundo.

**Cores** (`_win_DrawObjectI`): frente = entrada `[0]` da tabela de cores do objeto, fundo = `[2]` no modo colorido.

## Textos dinâmicos

Textos com `%s` ou `%d` (ex.: `fileSelDO`, `antEffEH`) são formatos de `printf` preenchidos em tempo de execução. O port ainda não os desenha.

## Validação

**Fato verificado** contra `tests/referencias/capturas/apos_titulo.png` (janela de registro no DOSBox-X): renderizando com o port e procurando o bloco inteiro de 1 bit na captura, o padrão aparece exatamente uma vez.

| Texto | Fonte | Tamanho | Posição na captura | Tinta |
|---|---|---|---|---|
| "Click here to continue" | slot 3 (`font2`) | 84×9 | (362, 397) | preta sobre cinza claro (cor 9 da tabela: `[0]`=0, `[2]`=8) |
| "Customer ID:" | slot 4 (`font3`) | 85×14 | (361, 356) | azul (índice 12) |

Isso confirma o recorte da linha 0, a altura `fRectHeight − 1`, os avanços e o caso `kernMax ≥ 0`. **Ainda não validados:** o caso `kernMax < 0` (`font1`, `font4`) e as fontes GDI Gaia, porque nenhuma tela capturada até agora usa essas fontes. O teste `fontes_mac_batem_com_o_original` (`engine/tests/arquivos_reais.rs`) trava os dois resultados.

## Em aberto

- Validar Gaia8x14 (usada em `fileSelWin`, `quitWin`) e as fontes com `kernMax < 0`.
- O texto do diálogo de registro é desenhado por `_win_DrawPictureWindow`, não por objetos de texto: título em `font3`, azul.
