# `SIMANTW.EXE`: mapa do executável

Arquivo analisado: `SimAnt-T/MAXIS/ANTWIN/SIMANTW.EXE` (516.096 bytes), acompanhado de `SIMANTW.SYM` (37.060 bytes, tabela de símbolos MAPSYM).

## Ferramentas e como reproduzir

- **REA (MCP):** a ferramenta recusa o executável com "unsupported NE executable in MZ image". O modo Ghidra do REA no Windows só aceita PE. Por isso o REA não foi usado no código, apenas a verificação de que ele está funcional.
- **Ghidra 12.1.4 headless** (`C:\Ghidra`), loader NE, linguagem `x86:LE:16:Protected Mode`. Os scripts ficam em `re_trabalho/ghidra_scripts/`:
  - `ApplySym.java`: aplica os 2.169 nomes do `.SYM` e cria 1.307 funções;
  - `ExportDecomp.java`: exporta o pseudocódigo de 1.538 funções (sem falhas) para `re_trabalho/decomp/CodeN.c`.
- `tools/ne_info.py`: cabeçalho NE, segmentos, recursos, importações e `.SYM`. Saída em `re_trabalho/simantw_ne.json`.
- `tools/save_table.py`: tabela de blocos do jogo salvo (ver [formatos/save-ant.md](formatos/save-ant.md)).

`re_trabalho/` é material de trabalho e **não faz parte do port** (fica no `.gitignore`). Pelas regras do projeto, nenhum trecho decompilado deve ser copiado para o código.

Convenção de endereços: segmento N do NE = seletor Ghidra `0x1000 + (N−1)·8`. Exemplo: segmento 7 = `1030`.

## Cabeçalho NE

Linker 5.30, Windows 3.0 ou superior, flags `0x030a`, ponto de entrada `4:0061` (`__astart`), DGROUP = segmento 10. Importa de `KERNEL`, `GDI`, `USER`, `SOUND`, `KEYBOARD` e `WIN87EM`, num total de 195 funções por ordinal.

Recursos dentro do EXE: só cursores (`HELPCURSOR`, `MAGCURSOR`, `ROCKCURSOR`, `DIGCURSOR`, `ANTCURSOR`, `FOODCURSOR`, `DROPCURSOR`, `SPRAYCURSOR`), ícones (`SIMANT`, `ANT0`–`ANT7`, `REDWIN`, `BLACKWIN`), os diálogos `OPEN`/`SAVEAS` e uma tabela de aceleradores. Todo o resto vem do banco `.DAT`.

## Segmentos e módulos (nomes do `.SYM`)

| Seg | Seletor | Nome no SYM | Tipo | Conteúdo, pelos nomes das funções |
|---|---|---|---|---|
| 1 | 1000 | `SIMANT_MODULE` | código | `WINMAIN`, `MAINWNDPROC`, `MYTIMERFUNC`, menus, pausa, pontuação (`_CalcScore`), cenários, diálogos, histórico, controle de casta/modo (o triângulo), formiga amarela, novo jogo |
| 2 | 1008 | `GR_MODULE` | código | vídeo (`_DetectDisplay`, `_InitGraphics`, `_ReadConfig`), primitivas `G*`, bitmaps, fontes, gerência de memória (`_mem_*`, `_Ralloc*`), animação (`_hanim_*`), som e MIDI |
| 3 | 1010 | `ANTEDIT_MODULE` | código | janela de edição/zoom, desenho de aranha e balões, ferramentas, quintal (`_DrawYard`, gato, cachorro, menino, cortador de grama, chuva), mapa, minimapa, carregar/salvar |
| 4 | 1018 | `_TEXT` | código | runtime C da Microsoft, tiles/escala em assembly, janela de informação, NetBIOS/DDE (modo em rede) |
| 5 | 1020 | `SIMONE_MODULE` | código | inicialização, geradores aleatórios (`_SRand*`, `_RRand`), água, comida, ovos, escavação de túneis, listas de formigas, geração de mapas (casa, quintal, pedras), aranha |
| 6 | 1028 | `SIMANT1_MODULE` | código | **núcleo da simulação de formigas**: `_DoAntSim`, comportamento por colônia (`A` superfície, `B` preta, `R` vermelha, `Y` amarela), feromônios, alarme, lutas, rainha |
| 7 | 1030 | `SIMTWO_MODULE` | código | estratégia de colônia, quintal (menino, pássaro, gato, cachorro), colonização do quintal, formiga-leão, lagarta, geração de mundo/tutorial, banco de dados (`_db_*`, `_DB*`, cache `_ch_*`, índice), LZSS, fontes, sistema de janelas próprio (`_win_*`) |
| 8 | 1038 | `SIMANT_DATA_GROUP` | dados | mapas de túneis, listas de formigas, feromônios, tabela do save (`0x9570`) |
| 9 | 1040 | `PACK` | dados | contadores, históricos, estado do quintal |
| 10 | 1048 | `DGROUP` | dados | variáveis globais, strings, pilha |

## Fluxo principal (visto no código, em alto nível)

- `_ReadConfig` (`1008:08f4`) → `_InitGraphics` (`1008:010c`) → `_db_SetDataBase` (abre `SHARED` e o banco gráfico).
- A simulação avança dentro de `MYTIMERFUNC` (`1000:2440`), callback de `SetTimer`, que chama `_DoAntSim` e depois `_UpdateWindows` e `_UpdateYardMessage`. O `SetTimer` pede intervalo de 0x11 = 17 ms. No Windows 3.1, o timer tem resolução de 55 ms, então na prática dispara a cada 55 ms (medido na bancada: 6,0 ticks/s em normal e 3,0 em slow, simulacao-visao-geral.md §3.1). Correção de uma versão anterior deste documento: `1048:0042` **não** é o intervalo, é o segmento do ponteiro da função de callback (`MakeProcInstance` em `WINMAIN`).
- Sequência de inicialização: ver [telas/abertura.md](telas/abertura.md).
- As janelas do jogo usam um sistema próprio (`_win_LoadWindow`, `_win_Events`, …) sobre uma única janela Windows. As definições vêm dos objetos de tipo 0 e 1 do banco.

## Interface de dados

- Banco de objetos: ver [formatos/banco-dat-ndx.md](formatos/banco-dat-ndx.md). `_db_LoadObject(id, tipo, flag)` em `1030:8292` procura primeiro no cache (`_ch_LookUpId`) e depois em cada banco aberto, na ordem (`_DBRecall`).
- Configuração: [formatos/simant-cfg.md](formatos/simant-cfg.md).
- Jogo salvo: [formatos/save-ant.md](formatos/save-ant.md).
