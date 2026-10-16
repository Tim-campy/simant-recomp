# Tiles (objetos tipo 9) e montagem da imagem dos mapas

Este documento cobre os tilesets do banco gráfico colorido (`WINGANT`, modo `W`, `_displayType = 10`) e a forma como a janela de edição monta a imagem da superfície e dos ninhos a partir dos mapas.

Implementação: `engine/src/formats/tiles.rs` (tilesets) e `engine/src/core/mapa.rs` (composição). Exemplo: `cargo run -p engine --example exportar_mapa -- <pasta do jogo> <save .ANT> <pasta de saída>`.

**Fontes:** pseudocódigo do Ghidra (só consulta) das funções `_OverlayTileSet` (`1010:00f4`), `_LoadTiles` (`1010:01d8`), `FUN_1010_6250` (`1010:6250`, desenha uma célula da janela de edição), `FUN_1010_63fe` (`1010:63fe`, célula com vida), `_XferTileColor` (`1018:47dd`), `_XferLifeTileColor` (`1018:48fa`) e `_DoCalcTile` (`1018:4a6b`). Os nomes vêm do `SIMANTW.SYM`. Os dados e a imagem foram conferidos com o `QUICK2.ANT` e a captura `qi_14_salvo.png` da bancada QEMU (ver "Verificação").

## Objetos

Todos são do tipo 9 no `WINGANT`. A chamada é `_db_LoadObject(id, 9, 1)`.

| Objeto | Conteúdo | Tamanho | Quem carrega |
|---|---|---|---|
| 9/10 | chão do quintal (fora da casa) | 32.768 bytes = 256 tiles | `_OverlayTileSet(0, 1000)` e `_LoadTiles` |
| 9/9 | chão de dentro da casa | 32.768 bytes = 256 tiles | `_OverlayTileSet(0, 1001)` |
| 9/15, 9/16, 9/17 | vida na superfície (formigas, ovos etc.), buffer `tilebufover` | 16.384 bytes cada = 128 tiles | `_LoadTiles` |
| 9/18, 9/19, 9/20 | vida nos ninhos, buffer `tilebufnest` | 16.384 bytes cada = 128 tiles | `_LoadTiles` |

- O chão ativo fica num único handle (`DS:9de0`). `_OverlayTileSet` troca 9/10 ↔ 9/9 e grava em `_TERRAINset` (`9:9b6e`, também em `DS:8df8`) 0 para o quintal e 1 para a casa. `_MakeYardPatch` e `_MakeHousePatch` chamam essa troca. `_TERRAINset` está no save (offset 48316, ver [save-ant.md](save-ant.md)). No `QUICK2.ANT` vale 0 (quintal). **Fato visto no código.**
- `_OverlayTileSet` também grava em `DS:7604` 0x50 (quintal) ou 0x90 (casa). A simulação compara esse valor com os bytes de `_MapA` em várias funções de `Code6`. É o limite entre tiles passáveis e obstáculos [hipótese]. `_OverlayOutsideTiles` e `_OverlayInsideTiles` (`1030:78b2`/`78be`) só gravam esse valor.
- Os buffers de vida são montados copiando cada objeto para o deslocamento 0, 0x4000 e 0x8000. Cada objeto ocupa, assim, 128 tiles, o que dá 384 tiles por buffer. Nos arquivos analisados cada objeto tem exatamente 16.384 bytes, então não sobram lacunas. **Fato verificado** (teste `tilesets_de_vida_tem_384_tiles_com_transparencia`).
- Nos outros modos coloridos (`_displayType` par e diferente de 10), os objetos de vida passam por `_ConvertMaskBitmap` e cada tile ocupa 0xA0 bytes, com máscara. Esses modos não existem na cópia analisada e o port não os implementa.

## Formato de um tile

- São 16×16 pixels a 4 bpp, 8 bytes por linha, 128 bytes por tile. Os tiles ficam um atrás do outro, de modo que o tile *i* começa em *i* × 128 (`_XferTileColor` soma `índice * 0x80`). **Fato visto no código.**
- No byte, o pixel da esquerda fica no nibble alto, como nos bitmaps 4 bpp ([bitmap.md](bitmap.md)).
- **As linhas estão de baixo para cima.** `_XferTileColor` copia as 16 linhas, em ordem, para um DIB de baixo para cima (o destino começa na linha `(editHeight − linha − 1) × tileHeight` e sobe). Portanto a primeira linha guardada é a de baixo. **Fato visto no código.** Isso foi confirmado ao comparar com a captura (a grama do topo do ninho aparece certa só nessa orientação).
- Os índices são os da paleta de 16 cores (`data/paleta.json`).

### Transparência dos tiles de vida

`_XferLifeTileColor` percorre o tile byte a byte:
- byte 0xDD (os dois pixels com índice 13): não escreve nada;
- caso contrário, um nibble igual a 0xD mantém o pixel de destino e o outro nibble é escrito.

Na prática, **o índice 13 (magenta) é transparente nos tiles de vida**. Nos tiles de chão, o 13 é desenhado normalmente. **Fato visto no código.** No port isso é `TRANSPARENTE = 13`, e o pixel é pulado ao desenhar (`desenhar_tile_de_vida`).

O tile 0 de cada buffer de vida não é usado como vida, porque o valor 0 em `_LifeA/_LifeB/_LifeR` significa "célula sem vida" (ver abaixo). No `WINGANT` ele não tem pixels transparentes.

## Indexação dos mapas

Os mapas são guardados **por colunas**: índice = x × 64 + y, em que y vai de 0 (topo) a 63. **Fato visto no código** (`_DoCalcTile`).

| `_MapPlane` | Mapa | Tamanho (tiles) | Índice no tileset de chão |
|---|---|---|---|
| 0 ou 1 | `_MapA` (superfície), 8192 bytes | 128 × 64 (`x & 0x7f`) | o próprio byte |
| 2 | `_MapB` (ninho preto), 4096 bytes | 64 × 64 (`x & 0x3f`) | `(byte − 0x70) & 0xff` |
| 3 | `_MapR` (ninho vermelho), 4096 bytes | 64 × 64 | `(byte − 0x70) & 0xff` |

- Nos ninhos, a subtração estoura num byte. No `QUICK2.ANT` os ninhos só têm valores de 0x00 a 0x2E, que caem nos tiles 0x90 a 0xBE do tileset de chão: grama e céu na linha 0 (0x1C a 0x1F → 0xAC a 0xAF), entrada (0x18 → 0xA8), túnel e terra (0x2E → 0xBE). **Fato verificado.**
- Os ninhos usam o mesmo handle de chão que a superfície (`DS:9de0`), ou seja, 9/10 ou 9/9 conforme `_TERRAINset`. Os tiles 0x90 a 0xBF dos dois tilesets parecem ser os mesmos [hipótese: não foi conferido byte a byte].
- O valor 0x70 (quintal) vs. 0x90 (casa) de `DS:7604` lembra o deslocamento dos ninhos, mas a relação não foi estudada.

### Camadas de feromônio (superfície)

Na superfície, `_DoCalcTile` consulta antes uma camada de feromônio, conforme `_ELayerMode`:

| `_ELayerMode` | Camada |
|---|---|
| 0 | `_PherMapA` |
| 1 | `_PherMapBN` |
| 2 | `_PherMapBT` |
| 3 | `_PherMapRN` |
| 4 | `_PherMapRT` |
| outro (−1 = desligado, o padrão) | nenhuma |

Cada camada tem 2048 bytes (64 × 32), uma célula para cada 2×2 tiles, também por colunas: índice = (x / 2) × 32 + y / 2. Se o byte for maior que 0x10, o tile de chão vira `((byte >> 4) − 0x10) & 0xff`, isto é, de 0xF1 a 0xFF (a faixa de pontos amarelos a vermelhos no fim do tileset). Caso contrário, vale o `_MapA`. **Fato visto no código.** O significado de cada modo (N = ninho, T = trilha) é [hipótese] pelos nomes. `_ELayerMode` não está no save. O port aceita a camada opcionalmente (`MapaChao::com_feromonio`). No `QUICK2.ANT` a `_PherMapA` é toda zero, então essa parte não foi conferida contra o original.

## Camada de vida

`_DoCalcTile` também calcula `_lifeTile` a partir de `_LifeA` (superfície), `_LifeB` ou `_LifeR` (ninhos), com o mesmo índice da célula:

| Byte de vida | `_lifeTile` |
|---|---|
| 0 | 0 (sem vida: só o chão é desenhado) |
| 0xFF (formiga amarela) | ninho: `0x300 + _MeBase + _MeType + (_MeType < 8 ? _MeEgg : _MeDir)`; superfície: o mesmo com `0x380` |
| 0xFE | ninho: `0x308 + _MeBase + _MeType + _MeDir`; superfície: `0x388 + …` |
| outro *v* | superfície: `v + 0x100`; ninho: `v + 0x200` |

`FUN_1010_63fe` desconta o deslocamento antes de indexar: −0x100 se `_lifeTile` < 0x200 (buffer `tilebufover`), −0x200 se estiver entre 0x200 e 0x37F (buffer `tilebufnest`) e −0x280 se for ≥ 0x380 (buffer `tilebufover`). A formiga amarela cai, assim, na terceira parte (índices 0x100 a 0x17F) de cada buffer, que na folha de contato é justamente a das formigas amarelas. A célula é desenhada com o chão (`_XferTileColor`) e, por cima, o tile de vida com transparência (`_XferLifeTileColor`). **Fato visto no código.**

**Pendente:** `_LifeA`, `_LifeB` e `_LifeR` não estão no save. São reconstruídos pela simulação a partir das listas de formigas (`_AlistX/Y/...`) [hipótese]. A camada de vida fica para quando essa reconstrução estiver especificada. O port já tem o tileset de vida e a primitiva com transparência.

## Montagem da imagem (janela de edição)

`FUN_1010_6250(col, lin)` desenha a célula visível (col, lin) da janela de edição:
1. a célula do mapa é `x = _MapPnt + col` e `y = DAT_1040_7d30 + lin`. Se y passar de 63, soma 0x40 a x e faz `y &= 0x3f`. O efeito na rolagem ainda não foi estudado [hipótese: a vista do ninho não rola além do fundo];
2. `_DoCalcTile(x, y)` calcula `_grndTile` e `_lifeTile`;
3. se nada mudou desde o último desenho (cache por célula em `DAT_1048_143e`/`1442`) e `_editForce` = 0, não redesenha;
4. sem vida, copia o tile de chão; com vida, chama `FUN_1010_63fe` (chão e vida por cima);
5. invalida o retângulo `(col × tileWidth + DS:9b08, lin × tileHeight + DS:9b0a)` com `tileWidth × tileHeight` (16 × 16).

O port monta qualquer retângulo de células (`MapaChao::desenhar`) num `Quadro`. Cada célula vira um tile de 16×16, com a célula (x0, y0) no ponto de destino.

## Verificação

- **Ninho preto do `QUICK2.ANT` contra `qi_14_salvo.png`** (capturado na mesma sessão em que o save foi gravado, vista do ninho de um Quick Game). A área cliente da janela de edição começa em (24, 158) na captura. O melhor encaixe é com a vista começando na célula (22, 0). Fora do diálogo "Saved correctly", 99,9% dos pixels batem com um mapeamento de cores 1:1 entre as duas imagens. A linha de grama e céu bate pixel a pixel. As poucas diferenças restantes estão nas bordas do diálogo e nas formigas (camada de vida, que não é desenhada). O túnel aparece na mesma posição e com a mesma forma onde não está coberto. Ferramenta: script de alinhamento ad hoc em Python com `tools/png_simples.py`.
- A QEMU mostra algumas cores com valores RGB diferentes dos da paleta padrão: o índice 3 sai como (168,168,87), o índice 7 como (135,136,143) e o verde 2 como (0,168,87). O mapeamento continua 1:1 e por isso não muda a conclusão. É uma diferença do driver de vídeo da bancada, não do jogo [hipótese].
- O teste `ninho_preto_do_quick2_bate_com_a_referencia` guarda o SHA-256 dos índices do mapa inteiro do ninho preto (1024 × 1024), gerado depois dessa conferência.
- A superfície e o ninho vermelho do `QUICK2.ANT` foram renderizados e parecem coerentes: pedras, folhas, entradas dos ninhos, montes nas bordas e túnel do ninho vermelho. Não há, porém, captura da superfície nessa sessão para comparar. **Não verificado contra o original.**
