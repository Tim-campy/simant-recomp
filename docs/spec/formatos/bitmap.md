# Bitmap (objeto tipo 2 do banco)

O conteúdo vem depois de descomprimir, se for o caso (ver [banco-dat-ndx.md](banco-dat-ndx.md)). Inteiros em little-endian.

| Offset | Tipo | Campo |
|---|---|---|
| 0 | u16 | `kind`: 0 ou 3 [significado em aberto; hipótese: 3 = sprite com cor de transparência] |
| 2 | u8 | formato: `0x84` nos bitmaps 4 bpp do `WINGANT`, `0x01` nos 1 bpp do `MWINNT`/`SHARED` |
| 3 | u8 | varia de 0x00 a 0x0F [hipótese: índice da cor transparente ou flags de conversão] |
| 4 | u32 | tamanho dos pixels em bytes (em alguns registros do `WINGANT` com flag 5 o valor é inconsistente; não é usado para ler) |
| 8 | u16 | largura `w` |
| 10 | u16 | altura `h` |
| 12 | … | pixels |

## Pixels

- **4 bpp (formato 0x84):** dois pixels por byte, nibble alto primeiro. Cada linha ocupa `((w·4 + 31) / 32) · 4` bytes (alinhamento de 4 bytes, como num DIB do Windows). As linhas estão de **baixo para cima**.
- **1 bpp (formato 0x01):** oito pixels por byte, bit mais significativo primeiro, com o mesmo alinhamento de 4 bytes. As linhas estão de **cima para baixo**. No `SHARED`, o campo de tamanho declara o tamanho já convertido para 4 bpp (por exemplo, 9632 bytes para um bitmap de 2408), o que indica que o jogo converte esses bitmaps mono para cor ao carregar (`_ConvertMonoBitmap`).

**Fonte:**
- Fato verificado: `tools`/`re_trabalho/sheet.py` renderizou corretamente 352 bitmaps do `WINGANT` (telas "About", "Select a Game", controles de casta, quintal, sprites) e bitmaps do `MWINNT`. Cada um deles bate com o tamanho calculado.
- `_DoBitmap` (`1008:332a`) aloca `((w·4+31)/32)·4·h` bytes e copia as linhas, o que confirma o alinhamento.
- `_ConvertBitmap` (`1008:246e`) converte de 4 **planos** (linhas de `ceil(w/8)` bytes por plano, 4 planos seguidos) para 4 bpp compactado. É chamada quando o último parâmetro de `_DoBitmap` não é zero. Os bitmaps do `WINGANT` testados não precisaram dessa conversão. Fica em aberto quais objetos são planares.
- `_GPutPic` (`1008:3c9a`) escolhe `_DoBitmap` ou `_DoMonoBitmap` pelo bit 0 de `_displayType`.

## Paleta

Os bitmaps 4 bpp parecem corretos com a paleta padrão de 16 cores do Windows (preto, vermelho/verde/amarelo/azul/magenta/ciano escuros, cinza claro, cinza escuro e as versões claras, branco) [hipótese, comparação visual]. O jogo mexe na paleta em `_PatchColorArrays`, `_SetUpPalette` e `_InitPalette`, que ainda não foram analisadas. O magenta claro (índice 13) aparece como fundo de sprites, o que aponta para uma cor-chave de transparência [hipótese].

## Em aberto

- Um grupo de bitmaps estreitos e altos (ids próximos de 7100 no `WINGANT`) sai com faixas verticais sem sentido: possivelmente tiles em outro arranjo ou planares.
- O significado de `kind` e do byte 3.
