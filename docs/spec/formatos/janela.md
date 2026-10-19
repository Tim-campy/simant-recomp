# Janelas do jogo (objetos tipo 0, 1 e 9)

O SimAnt tem um sistema de janelas próprio, herdado da versão Mac (funções `_win_*` em `SIMTWO_MODULE`). Cada janela do jogo (edição, mapa, título, controles de casta…) é uma lista de **objetos** retangulares desenhados por tipo. Inteiros em little-endian.

## Identificação

O código se refere a janelas por um número cujo **byte alto** é o id do objeto tipo 0 no banco. Exemplo: `_win_Open(0x300)` abre a janela 3 (`titleWin`).

**Fonte:** fato visto no código. `_win_LoadWindow` (`1030:c662`) chama `_db_LoadObject(param >> 8, 0, 1)`. A mensagem de erro é "CANNOT LOAD WINDOW %03x".

| Objeto | Banco | Conteúdo |
|---|---|---|
| tipo 0, id 0–36 | `WINGANT`/`MWINNT` | layout da janela (este documento) |
| tipo 1, id 0–36 | `SHARED` | nomes dos objetos da janela, 32 bytes ASCIIZ cada, na mesma ordem |
| tipo 0, id 128 (0x80) | `WINGANT`/`MWINNT` | três u16; o segundo é o número de cores da tabela (17 no `WINGANT`) |
| tipo 0, id 129 (0x81) | `WINGANT`/`MWINNT` | tabela de cores das janelas, 6 bytes por entrada |
| tipo 9, ids 0, 5, 7, 8 | `WINGANT`/`MWINNT` | posição de cada janela, conforme a resolução |

As 37 janelas do `WINGANT` (pelo nome do objeto 0): 0 `editWin`, 1 `mapWin`, 2 `scenarioWin`, 3 `titleWin`, 4 `gameOverWin`, 5 `infoWin`, 6 `editScentMenu`, 7 `yllwRecruitWin`, 8 `yllwStartMenu`, 9 `editToolsMenu`, 10 `editToExp`, 11 `editNoExpMenu`, 12 `wallRockMenu`, 13 `placeScentMenu`, 14 `cideMenu`, 15 `digFillMenu`, 16 `blackRedMenu`, 17 `foodMenu`, 18 `modeControlWin`, 19 `casteControlWin`, 20 `miniMapWin`, 21 `historyWin`, 22 `fileSelWin`, 23 `antPopWin`, 24 `antEffWin`, 25 (sem nome), 26 `dialogPicWin`, 27 `msgWindow`, 28 `debugWin`, 29 `examineWin`, 30 `picStrWin`, 31 `aboutWin`, 32 `editSpiderMenu`, 33 `quitWin`, 34 `mapRibbon`, 35 `ribbonYard`, 36 `iconMenu`.

## Layout (tipo 0)

| Offset | Tipo | Campo |
|---|---|---|
| 0x00 | 4 × i16 | retângulo da janela (esq., topo, dir., base), preenchido em tempo de execução a partir do objeto 0 |
| 0x0C | i16 | número de objetos `N`, incluindo o objeto 0, que é a própria janela |
| 0x1C | u8 | flags da janela. O bit 0x20 desliga o desenho; os bits 0x04 e 0x08 mudam o estilo da janela Windows criada (`_win_Open`) |
| 0x2C | `N` × 4 bytes | espaço para ponteiros far (zeros no arquivo, preenchidos ao carregar) |
| 0x2C + 4N | — | os `N` objetos, um após o outro |

**Fonte:** fato verificado. Com essa estrutura, os 37 layouts do `WINGANT` são percorridos até o último byte. `_RepointObjects` (`1030:c606`) avança de objeto em objeto usando o tamanho em +0x22.

### Objeto

| Offset | Tipo | Campo |
|---|---|---|
| 0x00 | 4 × i16 | retângulo absoluto (esq., topo, dir., base), já calculado para a resolução de referência |
| 0x08 | 4 × i16 | deslocamento de cada coordenada (esq., topo, dir., base) |
| 0x10 | 4 × u16 | referência de cada coordenada: id de objeto (`janela << 8 | índice`) ou índice de parâmetro |
| 0x18 | 4 × u16 | modo de cada coordenada (ver "Cálculo do layout") |
| 0x20 | u8 | [em aberto] |
| 0x21 | u8 | **tipo do objeto** (ver abaixo) |
| 0x22 | u16 | tamanho do objeto em bytes |
| 0x24 | u16 | flags: bit 0 = visível; bit 2 = selecionado (usa a cor e o bitmap alternativos); bits 7–8 = alinhamento do texto |
| 0x25 | u8 | (byte alto das flags) bit 1 = recortar o desenho ao retângulo |
| 0x26 | u8 | índice de cor normal (tabela 0/129) |
| 0x27 | u8 | índice de cor quando selecionado |
| 0x28 | … | dados que dependem do tipo |

No objeto 0, os deslocamentos em 0x08 trazem a posição e o tamanho da janela `(x, y, largura, altura)`: `_win_LoadWindow` copia esse valor para a tabela de posições, ou o sobrescreve com ela.

### Cálculo do layout

**Fonte:** fato visto no código, `_win_Recalc` (`1030:e6e2`). Cada coordenada *k* (0 = esq., 1 = topo, 2 = dir., 3 = base) de cada objeto vale:

| Modo (0x18 + 2k) | Valor |
|---|---|
| 0 | deslocamento (valor absoluto) |
| 1–4 | coordenada (modo − 1) do objeto indicado pela referência, mais o deslocamento |
| 5 | parâmetro de `_win_Open` de índice = referência (cabeçalho, 0x10 + 2·ref), mais o deslocamento |
| outro | 1 |

O cálculo se repete até nada mudar, porque as referências podem apontar para objetos posteriores. Exemplo, janela 3: o objeto 0 tem modos (0, 0, 1, 2) e deslocamentos (64, 72, 512, 342), o que dá (64, 72, 576, 414). `titlePic2` tem modos (1, 2, 1, 2), com referência ao objeto 0 nas duas primeiras coordenadas e deslocamentos (0, 168), o que dá (64, 240).

Depois do cálculo:

- O retângulo da janela é o do objeto 0. Se o objeto 0 é **contorno (tipo 15)** com espessura *t* (byte 0x28), a janela encolhe para (L+t, T+t, R−(t+1), B−(t+1)). Exemplo: a janela 2 (`scenarioWin`, t = 2) vai de (136, 41) com 322×258 na tabela para (138, 43) com 317×253.
- As coordenadas dos objetos passam a ser relativas à janela (com ajustes de +2 quando há faixa e +0x12 quando a janela tem barra de título). O port não reproduz essa etapa: usa a geometria medida (seção "Posicionamento na tela").

### Tipos de objeto

**Fonte:** `switch` em `_win_DrawObjectI` (`1030:b710`). `_win_DrawWindow` (`1030:bb96`) desenha os objetos 1…N−1 que têm o bit "visível", na ordem do arquivo.

| Tipo | Ocorrências | Desenho | Dados a partir de 0x28 |
|---|---|---|---|
| 0 | 25 | a janela (não é desenhado como objeto) | — |
| 1 | 181 | área cujo conteúdo é desenhado pelo código (mapas, gráficos…) | — |
| 2 | 6 | retângulo preenchido com a cor de fundo | — |
| 4 | 1 | nada (só log de depuração) | — |
| 5 | 12 | preenche e escreve texto (como o tipo 9) | idem 9 |
| 6 | 41 | **bitmap** no canto superior esquerdo do retângulo | u16 id do bitmap |
| 7, 8 | 1 | nada (só log) | — |
| 9 | 4 | texto | u8 fonte; a partir de 0x2A, texto ASCIIZ |
| 12 | 1 | nada (só log) | — |
| 13 | 76 | **botão de bitmap** | u16 bitmap quando selecionado; u16 bitmap normal (0x2A) |
| 15 | 15 | contorno de 1 pixel com a cor de frente | — |
| 16 | 9 | texto | u8 fonte; ponteiro far (0x2A, zero no arquivo); a partir de 0x2E, texto ASCIIZ (pode ser formato `printf`, ex.: `%s`) |
| 17 | 1 | preenche e escreve texto (como o 16) | idem 16 |
| 18 | 8 | título da janela, desenhado por outra rotina [em aberto] | — |
| 19, 20 | 1 | contorno em relevo (3D), lado claro e escuro trocados entre 19 e 20 | u8 espessura |
| 21, 22 | 0 | preenche e faz o contorno em relevo | u8 espessura |

### Bitmap ausente

Se o bitmap não carregar, `_win_DrawBitMapAtObj` (`1030:be30`) preenche o retângulo com a cor do chão (`_grndTile`).

## Tabela de cores (tipo 0, id 129)

Cada entrada tem 6 bytes: `[0]` cor de frente, `[1]` [em aberto, igual a `[0]` em todas as entradas], `[2]` cor de fundo, `[3]` segunda cor de fundo (no modo mono, a cor de fundo), `[4..5]` sempre `E1 04` [em aberto].

Quando `[2] ≠ [3]` no modo colorido, `_win_RectFill` (`1030:b54a`) preenche com o padrão de `_GPatBox` misturando as duas cores. Hipótese: xadrez 50% [a confirmar comparando com o DOSBox-X]. Exemplo: a entrada 1 tem `[2]=7` (cinza claro) e `[3]=8` (cinza escuro).

## Posição das janelas (tipo 9)

`_win_LoadAllWindows` (`1030:c806`) carrega uma tabela de 40 entradas `(x, y, largura, altura)` em i16 (320 bytes, dentro de um objeto de 480 bytes). O id depende do vídeo e da altura do desktop:

| `_displayType` | desktop com até 480 linhas | desktop com mais de 480 linhas |
|---|---|---|
| 10 (cor) | 2/9/0 | 2/9/8 |
| 9 (mono) | 9/5 | 9/7 |

Exemplo: no `WINGANT`, a janela 3 fica em (64, 3) com até 480 linhas e em (64, 72) com mais. Os retângulos absolutos guardados nos objetos (offset 0x00) coincidem com a tabela de **mais de 480 linhas**: `titleWin` vai de (64,72) a (576,414).

## Posicionamento na tela

**Fato verificado** por medição pixel a pixel na captura `tests/referencias/capturas/titulo_800x600.png` (DOSBox-X, Windows 3.1 a 800×600, jogo maximizado). As coordenadas abaixo são da área cliente da janela principal, que começa na linha 38 da tela, logo abaixo do menu.

- **Faixa** (`_ribbonBarHeight`): ocupa as linhas 0–72 da área cliente (altura 73) em toda a largura, com fundo cinza claro (índice 8). Nela é desenhada a janela 34 (`mapRibbon`), cuja entrada na tabela é (0, 17, 639, 73). Ou seja, a faixa é desenhada em y − 17.
- **Demais janelas:** o retângulo externo começa em (x, y + 73), onde (x, y) vem da tabela tipo 9. Isso bate com `_win_Open` (`1030:ca2e`), que soma `_ribbonBarHeight` na criação da janela. Para a janela 3: externo em (64, 145), conteúdo em (69, 150).
- **Borda:** 5 pixels de cada lado, sendo 1 pixel preto por fora e 4 na cor (166, 202, 240). O conteúdo de 512×342 vai de (69,150) a (580,491).
- **Janela com contorno (t > 0):** a borda envolve a janela já encolhida. Janela 2: externo em (138, 116), interior em (143, 121) com 317×253. O desenho dos objetos é **recortado** pelo interior (o bitmap 2500, de 318×259, aparece cortado). Medido em `novo_jogo.png` (bitmap 2500 localizado pixel a pixel): o objeto 0 cai em (interior.x − t + 1, interior.y − t). O "+1" só em x é do driver S3 do DOSBox-X: na bancada VGA e na SVGA (vbesvga) o objeto 0 cai em (interior.x − t, interior.y − t) [medido, 2026-10-19; hipótese: recorte do driver S3]. Fica no perfil de driver (`data/resolucao.json` → `perfis_driver` → `ajuste_x_contorno`; `telas/resolucao.md` §7). Na VGA de 16 cores a janela ainda vai 1 pixel à direita (área cliente no byte, `CS_BYTEALIGNCLIENT`). Com t = 0 (janela 3), o objeto 0 cai no canto do interior.
- **Fundo da área cliente:** (255, 251, 240).
- A borda e o fundo são **cores do sistema do Windows** (sem seção `[colors]` no `WIN.INI`, logo os padrões), não do SimAnt. No port, elas ficam nos índices 16 e 17 de `data/paleta.json`.

Os números ficam em `data/interface.json`.

## Validação

`tools/comparar_captura.py` contra capturas do DOSBox-X:

| Tela | Janelas | Pixels iguais | Diferenças |
|---|---|---|---|
| título (`titulo_800x600.png`) | 34, 3 | 99,98% | só o cursor do mouse |
| Select a Game (`novo_jogo.png`) | 34, 2 | 99,87% | o cursor e os botões `ribUserDef1E`/`1G` da faixa, que o jogo troca em tempo de execução (`_SetUserButton`, com valores do `WIN.INI` lidos por `GetProfileInt` em `WINMAIN`) |

O teste `engine/tests/telas_reais.rs` trava os dois resultados com hashes.

O padrão de preenchimento com duas cores (xadrez) **não** foi exercitado nessa comparação, porque nenhum objeto visível das janelas 34 e 3 o usa. Continua [hipótese].

## Port

O port usa os retângulos absolutos dos objetos, deslocados para a posição calculada acima. No executável, a tabela (0 ou 8) e o tamanho da área cliente vêm da tela virtual (`data/video.json` → `resolucao_virtual`, regra em `data/resolucao.json`; ver [../telas/resolucao.md](../telas/resolucao.md)); `data/jogo.json` → `tabela_posicoes` ficou para os testes e exemplos. Texto ainda não é desenhado: falta especificar as fontes. As janelas 34 e 3 não têm texto visível.
