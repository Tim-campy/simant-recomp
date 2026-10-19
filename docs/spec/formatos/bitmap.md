# Bitmap (objeto tipo 2 do banco)

O conteúdo vem depois de descomprimir, se for o caso (ver [banco-dat-ndx.md](banco-dat-ndx.md)). Inteiros em little-endian.

| Offset | Tipo | Campo |
|---|---|---|
| 0 | u16 | `kind`: 0 = opaco, 3 = com máscara (índice 13 transparente), 0xFFFF = empacotado (ver "Campo kind") [código] |
| 2 | u8 | formato: bit 0x80 = 4 bpp compactado (`0x84` no `WINGANT`); sem ele, `0x04` = planar e `0x01`/`0x00` = 1 bpp (`MWINNT`/`SHARED`) |
| 3 | u8 | varia de 0x00 a 0x0F (e 0x65 em 96 sprites do `MWINNT`); não é lido pelos caminhos de desenho analisados (ver "Byte 3") |
| 4 | u32 | tamanho dos pixels em bytes (em alguns registros do `WINGANT` com flag 5 o valor é inconsistente; não é usado para ler) |
| 8 | u16 | largura `w` |
| 10 | u16 | altura `h` |
| 12 | … | pixels |

## Pixels

- **4 bpp (formato 0x84):** dois pixels por byte, nibble alto primeiro. Cada linha ocupa `((w·4 + 31) / 32) · 4` bytes (alinhamento de 4 bytes, como num DIB do Windows). As linhas estão de **baixo para cima**.
- **1 bpp (formato 0x01):** oito pixels por byte, bit mais significativo primeiro, com o mesmo alinhamento de 4 bytes. As linhas estão de **cima para baixo**. No `SHARED`, o campo de tamanho declara o tamanho já convertido para 4 bpp (por exemplo, 9632 bytes para um bitmap de 2408), o que indica que o jogo converte esses bitmaps mono para cor ao carregar (`_ConvertMonoBitmap`).
- **Planar (formato 0x04):** no `WINGANT`, só o menu de ícones (2/7200, 187 × 166); no `MWINNT`, 2/7005 e 2/8905. Cada linha, de **cima para baixo**, tem os 4 planos de 1 bit em sequência, cada um com ⌈w/8⌉ bytes, **sem alinhamento** (`_ConvertBitmap`, `1008:246e`: planos em +0, +⌈w/8⌉, +2⌈w/8⌉, +3⌈w/8⌉ de cada linha); o plano k é o bit k do índice de cor [medido: bate com a captura do menu (janelas.md §4.4); o 2/8905 do `MWINNT`, 19 × 35, tem 420 bytes = 35 × 4 × 3, o que confirma a falta de alinhamento]. O 2/7005 do `MWINNT` (`kind` 3, 7 × 37) tem 185 bytes = 37 × 5: um quinto plano por linha [hipótese: a máscara de `_ConvertMaskBitmap2`]; o port lê os 4 primeiros.

**Fonte:**
- Fato verificado: `tools`/`re_trabalho/sheet.py` renderizou corretamente 352 bitmaps do `WINGANT` (telas "About", "Select a Game", controles de casta, quintal, sprites) e bitmaps do `MWINNT`. Cada um deles bate com o tamanho calculado.
- `_DoBitmap` (`1008:332a`) aloca `((w·4+31)/32)·4·h` bytes e copia as linhas, o que confirma o alinhamento.
- `_ConvertBitmap` (`1008:246e`) converte de 4 **planos** (linhas de `ceil(w/8)` bytes por plano, 4 planos seguidos) para 4 bpp compactado. É chamada quando o último parâmetro de `_DoBitmap` não é zero. Os bitmaps do `WINGANT` testados não precisaram dessa conversão. Fica em aberto quais objetos são planares.
- `_GPutPic` (`1008:3c9a`) escolhe `_DoBitmap` ou `_DoMonoBitmap` pelo bit 0 de `_displayType`.

## Paleta

**Fato verificado (pixel a pixel contra o DOSBox-X, 2026-10-16):** os índices dos bitmaps 4 bpp seguem a paleta padrão de 16 cores do Windows, **nesta ordem**: 0 preto, 1–6 vermelho/verde/amarelo/azul/magenta/ciano escuros (0x80), **7 cinza escuro (0x808080)**, **8 cinza claro (0xC0C0C0)**, 9–14 as versões claras (0xFF), 15 branco. A primeira versão deste documento trocava 7 e 8; a captura mostrou o erro (ver `docs/divergencias.md`).

Nas capturas, as cores aparecem expandidas pelo DAC VGA de 6 bits (0x80 vira 130 e 0xC0 vira 195). Isso é efeito do emulador, não do jogo. `tools/comparar_captura.py` aplica a mesma conversão. O jogo mexe na paleta em `_PatchColorArrays`, `_SetUpPalette` e `_InitPalette`, que ainda não foram analisadas. O magenta claro (índice 13) aparece como fundo de sprites, o que aponta para uma cor-chave de transparência [hipótese].

## Campo kind

**Fato visto no código** (`_DrawBitMapToBuffer`, `1008:418e`, que escolhe a rotina de cópia pelo cabeçalho; `_GPutPic`/`_DoBitmap` recebem só largura, altura e pixels):

- **0**: opaco. Cor (`_displayType` par): bit 0x80 do formato ligado → `_CopyBitmap2` (4 bpp compactado), desligado → `_ConvertBitmap2` (planar); monocromático → `_CopyMonoBitmap`.
- **3**: com máscara. `_CopyMaskBitmap2` (`1030:b110`) / `_ConvertMaskBitmap2` / `_CopyMonoMaskBitmap`. Na cópia 4 bpp, cada nibble igual a **0xD (13, magenta claro)** é pulado: a cor transparente é fixa, não vem do cabeçalho (confirma a [hipótese] da seção "Paleta"). No `WINGANT` são os sprites: formigas (1000–1107), bichos do quintal (1300…), gotas e enxame (7005–7007), balões (8020…, 8100…) etc. (105 objetos).
- **0xFFFF (−1)**: empacotado. O u16 em +2 é o tamanho aberto (cabeçalho de 12 bytes + pixels) e um fluxo LZSS (o mesmo do banco) começa em +4; `_GUnpackPic` (`1008:40d4`) abre o cabeçalho de 12 bytes e `tamanho − 16` bytes de pixels (os 4 últimos bytes da imagem não são abertos e ficam com o que havia na memória) e segue pelo `kind` de dentro. Só 3 objetos, todos no `MWINNT`: 2/2000 (título, 455 × 320), 2/2500 (315 × 258) e 2/7002 (514 × 226), gravados no banco **sem** o bit de compressão do índice. Os de dentro têm formato 0x00 ou 0x01 e linhas de 1 bpp alinhadas a byte. Conferido abrindo o 2/2000 (o título do SimAnt) e o 2/7002 (a grade do quintal).

**Port** (2026-10-19): `Bitmap::decodificar` abre o `kind` 0xFFFF (os 4 bytes não abertos ficam 0), lê o planar sem alinhamento e trata o formato 0x00 como 1 bpp; os três bancos decodificam inteiros (`todos_os_bitmaps_dos_bancos_decodificam`: 352 no `WINGANT`, 58 no `SHARED`, 343 no `MWINNT`). A transparência dos sprites é feita pelo desenho do port com o índice 13 (`kind` não é consultado). O modo monocromático (`MWINNT`) não é usado pelo port.

## Byte 3

Valores por banco (contagem dos objetos tipo 2): `WINGANT` kind 0: 0 (32), 5 (13, a série 17000), 9 (4, a série 12000), 12 (2), 13 (1, o planar 7200), 15 (193); kind 3: 0 (62), 2 (7), 4 (16), 14 (20). `MWINNT`: 0, 1, 2, 3, 4, 13, 15 e 0x65 (96 sprites). Nenhum dos caminhos acima lê o byte [código]; o valor não acompanha a cor transparente (fixa em 13) nem o formato. [hipótese: resto da conversão dos recursos do Mac, sem uso no Windows; outros leitores do cabeçalho não procurados].

## Em aberto

- ~~Um grupo de bitmaps estreitos e altos (ids próximos de 7100 no `WINGANT`) sai com faixas~~ (2026-10-19): não há bitmaps entre 7011 e 7199 no `WINGANT`; os estreitos e altos do grupo 7000 são as gotas da chuva (2/7005, 7 × 37, `kind` 3) e os pontos do enxame (7006/7007), que decodificam e são desenhados pelo port (chuva conferida na bancada, telas/quintal.md). Os que falhavam eram os empacotados e os planares do `MWINNT` (acima).
- Byte 3 (acima) e o quinto plano dos planares com máscara do `MWINNT`.
