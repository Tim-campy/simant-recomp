*Read in English: [banco-dat-ndx.md](../../../spec/formatos/banco-dat-ndx.md)*

# Banco de recursos `.NDX` / `.DAT`

Os gráficos, textos, janelas, menus, sons e músicas do SimAnt para Windows ficam em pares `NOME.NDX` + `NOME.DAT` na pasta do jogo (`MAXIS\ANTWIN`).

| Banco | Conteúdo | Quando é aberto |
|---|---|---|
| `SHARED` | janelas, menus, listas de strings, textos do sistema de informação, bitmaps mono comuns | sempre |
| `WINGANT` | layouts de janela e bitmaps 4 bpp (16 cores) | modo de vídeo colorido (`_displayType` = 10) |
| `MWINNT` | layouts de janela e bitmaps 1 bpp (monocromático) | modo monocromático (`_displayType` = 9) |
| `SOUND` | instrumentos digitais, cabeçalhos de música, arquivos MIDI | sempre |
| `LANGUAGE` | (não existe na cópia analisada) | tentativa de abrir `language.dat` [hipótese: banco de tradução opcional] |

**Fonte:**
- Nomes dos bancos: `_InitGraphics` (`1008:010c`) monta o nome com `"%s%s"` a partir de uma tabela de prefixos no DGROUP (`1048:b92b`…: `hcega`, `mono`, `tdyga`, `lcega`, `l256`, `mwin`, `winga`) indexada por `_displayType`, e chama `_db_SetDataBase`. O sufixo `nt` vem do chamador. Com `mwin`+`nt` e `winga`+`nt` chega-se aos arquivos existentes, mas a origem do sufixo ainda não foi confirmada no código [hipótese].
- Strings `language.dat`, `language`, `shared` e `lrshare` em `SIMANTW.EXE`, offset de arquivo `0x2b9a0`.
- Fato verificado com `tools/db_list.py`: os quatro pares existentes são lidos de ponta a ponta sem sobras.

## `.NDX`: índice

Inteiros em little-endian.

| Offset | Tipo | Campo |
|---|---|---|
| 0x00 | u32 | número de entradas `N` |
| 0x04 | 16 bytes | zeros em todos os arquivos analisados (significado desconhecido) |
| 0x14 | `N` × 8 bytes | entradas |

Entrada:

| Offset | Tipo | Campo |
|---|---|---|
| 0 | u32 | posição do registro no `.DAT`, relativa ao fim do cabeçalho de 14 bytes |
| 4 | u16 | id do objeto |
| 6 | u8 | tipo do objeto (ver tabela abaixo) |
| 7 | u8 | flags de armazenamento (bit 0 = comprimido) |

O par **(tipo, id)** identifica o objeto. O mesmo id se repete em tipos diferentes (exemplo: o id 1000 existe como bitmap e como lista de strings).

**Fonte:** fato verificado. Os tamanhos dos arquivos batem com `0x14 + 8·N` nos quatro índices, e todas as posições caem exatamente sobre um registro do `.DAT`.

## `.DAT`: registros

Cabeçalho do arquivo (14 bytes):

| Offset | Tipo | Campo |
|---|---|---|
| 0 | u32 | `0x12345678` (assinatura) |
| 4 | u16 | sempre um pouco maior que o número de entradas do `.NDX` (exemplo: 0x19c contra 406) [significado desconhecido] |
| 6 | u16 | desconhecido |
| 8 | u32 | desconhecido (valores de 1 a 5) |
| 12 | u16 | 0 |

Depois vêm os registros, um após o outro:

| Offset | Tipo | Campo |
|---|---|---|
| 0 | u32 | `0x12345678` |
| 4 | u16 | 0 em todos os registros |
| 6 | u32 | tamanho `S` do conteúdo |
| 10 | `S` bytes | conteúdo |

## Compressão (flag bit 0)

Quando o bit 0 do byte de flags está ligado, o conteúdo é:

- u16: tamanho descomprimido;
- fluxo **LZSS** no estilo Okumura: janela circular de 4096 bytes e posição inicial de escrita 4096−18. Cada byte de controle é lido do bit menos significativo para o mais significativo: bit 1 = literal de 1 byte, bit 0 = referência de 2 bytes `a b`, com posição = `a | (b & 0xF0) << 4` e comprimento = `(b & 0x0F) + 3`.

**Fonte:**
- Fato verificado: com esse algoritmo, todos os 828 registros comprimidos dos quatro bancos descomprimem exatamente no tamanho declarado e consomem o conteúdo inteiro (`tools/lzss_probe.py`).
- Os nomes `_InitTree`, `_InsertNode`, `_DeleteNode`, `_Encode` e `_Unpack` no `SIMANTW.SYM` (segmento `SIMTWO_MODULE`) correspondem ao codificador de árvore binária de Okumura.
- Preenchimento inicial da janela: fato visto no código. `_UnpackInit` (`1030:a62a`) preenche as posições 0 a 0xFED com 0x20 (espaço) e começa a escrever em 0xFEE. `_Unpack` (`1030:a668`) lê os bits de controle do menos para o mais significativo e calcula o comprimento como `(b & 0x0F) + _pack_threshold + 1`, com `_pack_threshold` = 2 [o valor 2 foi deduzido da validação acima]. Isso importa: com zero no lugar de espaço, 67 bitmaps sairiam diferentes.

Outros valores observados no byte de flags: 0, 1, 5 e 8. O bit 2 (valor 4) e o bit 3 (valor 8) aparecem em bitmaps e sons, mas ainda não têm significado conhecido [hipótese: indicam descarte/cache em `_mem_MakeDiscardable`].

## Tipos de objeto

| Tipo | Uso | Evidência |
|---|---|---|
| 0 | layout binário de janela, por modo de vídeo (`WINGANT`/`MWINNT`, ids 0–129) | os ids casam com os do tipo 1; estrutura ainda não decodificada |
| 1 | tabela de nomes dos objetos de uma janela: registros de 32 bytes com nome ASCIIZ (`editWin`, `editWinTitle`, …) | dump do `SHARED` 1/0 |
| 2 | bitmap, ver [bitmap.md](bitmap.md) | `_db_LoadObject(id, 2, 1)` em `GR_MODULE`; imagens renderizadas corretamente |
| 4 | lista de strings (u16 big-endian com a contagem, seguido de strings Pascal; `\n` literal marca quebra de linha) | dump do `SHARED` 4/1000 ("BackYard View", …); `_LoadStringAnt` (`1020:03f4`) lê só o byte baixo da contagem e faz de cada string uma linha (parser: `engine/src/formats/textos.rs`) |
| 6 | menu da janela principal (títulos e itens, ver [menu.md](menu.md)) | `_db_LoadObject(param_1, 6, …)` seguido da mensagem "Couldn't load menu %d"; objeto 6/0 decodificado |
| 9 | tilesets 16×16 a 4 bpp: chão (ids 9 e 10) e vida (ids 15–20), ver [tiles.md](tiles.md) | `_OverlayTileSet`, `_LoadTiles` |
| 10 | texto corrido do sistema de informação (ASCII) | dump do `SHARED` 10/150 |
| 17 | layout de cartão do sistema de informação (big-endian, itens `PICT`/`TEXT`) | dump do `SHARED` 17/127 |
| 18 | cabeçalho de música (18 bytes, big-endian), ver [som.md](som.md) | `_db_LoadObject(x, 0x12, 1)` antes de tocar |
| 19 | nome do cartão do sistema de informação (ASCIIZ, ex.: `Top_Card`) | dump |
| 20, 50 | arquivo MIDI padrão (`MThd`…): 20 = Sound Blaster, 50 = alto-falante, ver [som.md](som.md) | assinatura; `_db_LoadObject(x, 0x14 / 0x32, 1)` |
| 5 | efeito sonoro: amostras de 8 bits em deltas de 4 bits, 4096 Hz, ver [som.md](som.md) | `_UncompressDACInstrument`, `_myBeginSound` |
| 51 | efeito sonoro em recurso `snd ` do Macintosh (só com `DSOUND.DLL`), ver [som.md](som.md) | dump; `_myBeginSound` |
| 7, 8, 11, 15, 21, 22 | desconhecido | — |

Os valores big-endian (contagens em listas de strings, cartões) indicam que esses dados vieram da versão Macintosh (o SYM também tem `_MacTickCount`). Já os cabeçalhos de bitmap e o próprio banco são little-endian.

## Impacto nos mods

Como cada objeto é endereçado por (tipo, id), um mod pode sobrescrever, por exemplo, só a lista de strings 4/1220 ou só o bitmap 2/7000, sem mexer nos arquivos originais. O carregador do port deve consultar primeiro os mods e só depois o banco original.
