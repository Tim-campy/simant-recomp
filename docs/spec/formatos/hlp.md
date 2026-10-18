# Arquivo de ajuda `SIMANT.HLP` (WinHelp 3.0)

O original não desenha a própria ajuda: chama `WinHelp(hwnd, "SIMANT.HLP", comando, n)` (`_DoWinHelp` `1000:5aac`, `_DoEvent`, `_HelpKeyDown`; o nome do arquivo está em `_helpFile`, `9:76ac`) e o WinHelp do Windows mostra o tópico. O port lê o arquivo do usuário (`MAXIS/ANTWIN/SIMANT.HLP`, 197.256 bytes) e desenha a janela ele mesmo (visor: [../telas/ajuda.md](../telas/ajuda.md)).

Implementação: `engine/src/formats/hlp.rs` (leitura), `tools/hlp_inspecionar.py` (inspeção em Python). Testes com o arquivo real: `engine/tests/ajuda_reais.rs`.

Fontes do formato: a descrição pública "Windows Help File Format" (helpfile.txt, de M. Winterhoff, distribuída com o helpdeco) e a conferência byte a byte no `SIMANT.HLP`. Abaixo, "[arquivo]" = conferido no `SIMANT.HLP`; o resto vem da descrição pública e está marcado quando não foi usado.

## 1. Cabeçalho e arquivos internos

| Desloc. | Tipo | Conteúdo | No SIMANT.HLP |
|---|---|---|---|
| 0 | u32 | magia 0x00035F3F | [arquivo] |
| 4 | i32 | início do diretório | 0x8C1 |
| 8 | i32 | primeiro bloco livre (−1) | −1 |
| 12 | u32 | tamanho do arquivo | 197.256 |

Cada arquivo interno começa com 9 bytes: `i32` reservado, `i32` usado (tamanho do conteúdo), `u8` flags; o conteúdo vem logo depois.

O diretório é um arquivo interno com uma árvore B+: depois dos 9 bytes, o cabeçalho da árvore (38 bytes): `u16` magia 0x293B, `u16` flags, `u16` tamanho da página (1024), 16 bytes de estrutura ("z4"), `i16` 0, `i16` divisões, `i16` página raiz, `i16` −1, `i16` total de páginas, `i16` níveis, `i32` entradas. As páginas vêm em seguida. Página folha: `u16` livre, `i16` número de entradas, `i16` página anterior, `i16` página seguinte, e as entradas (nome terminado em 0, `i32` posição do arquivo interno). Página de índice: `u16` livre, `i16` entradas, `i16` primeira página filha, … — para achar a primeira folha basta descer pela primeira filha e seguir as "seguintes". No SIMANT.HLP a árvore tem um nível e uma página [arquivo].

Arquivos internos do SIMANT.HLP [arquivo]: `bm0`…`bm65` (figuras), `|Phrases`, `|SYSTEM`, `|TOPIC`, `|TOMAP`, `|FONT`, `|CTXOMAP`, `|KWDATA`, `|KWMAP`, `|KWBTREE` (palavras-chave da busca) e `|TTLBTREE` (títulos). O port não lê os quatro últimos.

## 2. `|SYSTEM`

`u16` magia 0x036C, `u16` versão menor, `u16` maior (1), `i32` data, `u16` flags. Menor 15 = compilador HC30 (Windows 3.0); 21 = HC31 (3.1); acima de 16, o `|TOPIC` pode ser comprimido (flags 4 ou 8) e o título vem em registros. No HC30 o título é uma cadeia terminada em 0 logo depois do cabeçalho.

SIMANT.HLP: menor 15, flags 8, título "SimAnt" [arquivo]. O port só lê HC30 (o único que o SimAnt traz); um HC31 dá erro.

## 3. `|Phrases` (HC30)

`u16` número de frases N, `u16` 0x0100, N + 1 deslocamentos `u16` **relativos ao início da tabela** (o primeiro vale 2 × (N + 1)), e o texto das frases, sem terminador (uma frase acaba onde começa a seguinte). SIMANT.HLP: 1.147 frases [arquivo].

No texto do tópico, um byte 1–9 seguido de um byte `d` forma o índice `256 × (byte − 1) + d`; a frase é `índice / 2`, seguida de um espaço quando o índice é ímpar. O texto só é comprimido assim quando o tamanho descomprimido (`DataLen2`) é maior que o espaço que ele ocupa no registro.

## 4. `|TOPIC` (HC30)

Blocos de 2048 bytes, cada um com um cabeçalho de 12 bytes (`i32` último TOPICLINK, `i32` primeiro TOPICLINK, `i32` último cabeçalho de tópico); sem compressão LZ77 no HC30 [arquivo]. Os registros (TOPICLINK) atravessam blocos: ao ler um registro, pulam-se os 12 bytes de cada bloco no caminho.

TOPICLINK: `i32` tamanho do registro, `i32` `DataLen2` (tamanho do texto descomprimido), `i32` anterior, `i32` **seguinte = distância em bytes até o próximo registro** (contando os cabeçalhos de bloco pulados; no HC31 é uma posição absoluta), `i32` `DataLen1` (cabeçalho + LinkData1), `u8` tipo; LinkData1 vai até `DataLen1`, o texto (LinkData2) até o fim do registro. O primeiro registro fica na posição 12. SIMANT.HLP: 344 tópicos, 350 registros de texto [arquivo].

Tipos:

- **2 — cabeçalho do tópico.** LinkData1: `u32` tamanho, `i32` tópico anterior, `i32` tópico seguinte (números de tópico da sequência de navegação, −1 = nenhum; botões `<<` e `>>`). LinkData2: o título. No SIMANT.HLP todo título termina em "(id=0x…)" com o número de contexto (ex.: "Patch Ribbon Bar (id=0x2200)") [arquivo]; o port guarda o título bruto e um título sem esse sufixo.
- **1 — texto (HC30).** LinkData1: `compressed signed long` (tamanho), `u8`, `u8`, `u16` id, `u16` bits do parágrafo, os valores opcionais dos bits e a lista de comandos de formatação até 0xFF. No SIMANT.HLP os quatro primeiros campos são sempre 0, 0x80, 0x08CA e um único registro tem bits ≠ 0 (0x0800, centralizado) [arquivo].

Bits do parágrafo (do bit 0 para cima): valor desconhecido (`compressed signed long`), espaço acima, abaixo, entre linhas, recuo esquerdo, direito, da primeira linha (`compressed signed short` cada), sem uso, borda (`u8` + `i16`), tabulações (`compressed signed short` n, e n × `compressed unsigned short`, com bit 0x4000 = mais um valor de tipo), alinhado à direita (0x0400), centralizado (0x0800). As unidades das medidas não foram conferidas (o SIMANT.HLP não usa nenhuma) [hipótese].

Valores comprimidos: *unsigned short* = 1 byte (par) ou 2 bytes (ímpar), metade do valor lido; *signed short* = o mesmo menos 64 (1 byte) ou 16384 (2 bytes); *unsigned long* = 2 ou 4 bytes, metade; *signed long* = o mesmo menos 16384 ou 0x4000000.

Texto: LinkData2 é uma sequência de cadeias separadas por 0. Antes de cada comando vem uma cadeia (pode ser vazia): cadeia 0, comando 0, cadeia 1, comando 1, …, até o 0xFF.

| Comando | Argumentos | Significado |
|---|---|---|
| 0x80 | `u16` | troca a fonte (índice do `|FONT`); a fonte continua no registro seguinte |
| 0x81 | — | quebra de linha |
| 0x82 | — | fim do parágrafo |
| 0x83 | — | tabulação |
| 0x86, 0x87, 0x88 | `u8` tipo, `compressed signed long` tamanho, [tipo 0x22: `compressed unsigned short`], dados (`i16` embutida, `i16` número da figura `bmN` quando 0) | figura no texto, à esquerda ou à direita. SIMANT.HLP: 65 figuras nos tópicos, tipo 3, tamanho 4; 46 com 0x86 (no meio do texto, ícones dos botões) e 19 com 0x87 [arquivo] |
| 0x89 | — | fim do hotspot |
| 0x8B, 0x8C | — | espaço / hífen sem quebra |
| 0xC8, 0xCC | `u16` n, n bytes | macro |
| 0xE0, 0xE1 | `u32` número do tópico | popup / salto (HC30); o número indexa o `|TOMAP` |
| 0xE2–0xE7 | `u32` | popup/salto do HC31 (hash do contexto) [não usado] |
| 0xEA–0xEF | `u16` n, n bytes | salto para outro arquivo/janela [não usado] |
| 0xFF | — | fim |

Exemplo conferido (índice, contexto 0x100): fonte 0, "SimAnt", fonte 1, fim de parágrafo, "Welcome to SimAnt", fim, fim (linha vazia), e 14 vezes "salto 0xE1, texto, 0x89, fim de parágrafo" (Introduction, Ribbon Bars, …, Credits), como a captura a06 do WinHelp do original.

## 5. `|TOMAP`, `|CTXOMAP`

- `|TOMAP` (HC30): vetor de `u32`, posição do cabeçalho de cada tópico pelo número do tópico (360 entradas no SIMANT.HLP). Os saltos 0xE0/0xE1 e a sequência de navegação usam esse número [arquivo: o salto 0x124 do índice leva a "Introduction"].
- `|CTXOMAP`: `u16` n e n pares (`u32` número de contexto, `u32` posição do cabeçalho do tópico). É o que `WinHelp(…, HELP_CONTEXT, n)` usa. Conferidos com o WinHelp do original na bancada: 0x2200 → "Patch Ribbon Bar" (botão "?" da faixa, a04), 0x2209 → "Status Window Button" (a05), 4 → "Edit Window" (a02), e no arquivo 0x102 "Map Window", 0x1300 "Caste Control Window", 0x2308 "Help Button", 0xFD48 "Bookmarks", 0x100 "SimAnt" (o índice; F1 abre o índice, a06).

## 6. `|FONT` (HC30)

`u16` número de faces, `u16` número de descritores, `u16` início das faces, `u16` início dos descritores. As faces têm tamanho fixo ((início dos descritores − início das faces) / faces). Descritor (11 bytes): `u8` atributos (bit 0 negrito, 1 itálico, 2 sublinhado, …), `u8` meios pontos, `u8` família, `u16` índice da face, 3 bytes RGB da frente e 3 do fundo. SIMANT.HLP: 69 faces e 3 descritores: 0 = Helv 12 pt negrito, 1 = Helv 10 pt, 2 = Helv 10 pt negrito [arquivo].

## 7. Figuras `bmN` (MRB/SHG)

`u16` magia 0x706C ("lp"), `u16` número de imagens, `u32` posição de cada imagem. Imagem: `u8` tipo (5 = DDB, 6 = DIB), `u8` empacotamento (0 nenhum, 1 RunLen, 2 LZ77, 3 LZ77 e depois RunLen), e, comprimidos: dpi x e y (`unsigned long`), planos e bits por pixel (`unsigned short`), largura, altura, cores usadas, cores importantes, tamanho dos dados, tamanho dos hotspots (`unsigned long`); depois `u32` posição dos dados e `u32` dos hotspots (relativas ao início da imagem) e a paleta RGBQUAD (DIB). Os pixels são de um DIB: linhas de baixo para cima, alinhadas a 4 bytes.

RunLen: byte n; com o bit 7, copiar os `n & 0x7F` bytes seguintes; senão, repetir o byte seguinte n vezes. LZ77: byte de flags (bit 0 primeiro); bit 1 = par `u16` com comprimento `(v >> 12) + 3` e distância `(v & 0xFFF) + 1`; bit 0 = byte literal.

SIMANT.HLP: os 66 `bmN` são DIB de 4 bits e 16 cores com RunLen (ex.: `bm0`, 75 × 75) [arquivo: as 65 usadas decodificam; a imagem desenhada pelo port não foi comparada com o WinHelp do original].
