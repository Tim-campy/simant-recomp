# Códigos de trapaça do original (`_CheatKeys`)

O SimAnt para Windows tem trapaças embutidas: **códigos de 4 letras digitados durante a partida**. Não há menu escondido nem combinação de teclas especial; não há nenhum aviso na tela além do som (ou do quadro de texto) de cada código. Este documento descreve a entrada (`_DoKeyDown`, `1000:0e8c`), a tabela (`_Cheats`, `8:83bc`) e os efeitos (`_CheatKeys`, `1018:67de`).

Fontes: pseudocódigo do Ghidra (`re_trabalho/decomp/Code1.c`, `_DoKeyDown`; `Code4.c`, `_CheatKeys`), desmontagem de `1018:691c`–`69a0` (casos 9 e 10) e a bancada QEMU (roteiro `tests/referencias/roteiros/qemu_trapacas.txt`, 2026-10-18).

## 1. Entrada: o anel de 4 letras

`_DoKeyDown(hwnd, vk, ...)` trata `WM_KEYDOWN` da janela principal. Na ordem [código]:

1. Espaço, Insert e Delete viram cliques (formiga-amarela.md §11.1).
2. `_HelpKeyDown` e `_YellowCommandKey` (formiga-amarela.md §11.4): se um deles tratou a tecla, fim. **Uma letra que é comando da amarela (X; J com Tab) não entra no anel.**
3. Setas: movem o cursor 8 pixels; fim.
4. Enter (`vk = 0x0D`): zera a posição e a contagem do anel (`DAT_1048_01da`, `DAT_1048_01dc`); fim.
5. Só letras A–Z (`vk` 0x41–0x5A) seguem. O caractere é **minúsculo sem Shift e maiúsculo com Shift** (`GetAsyncKeyState(VK_SHIFT)`; a tabela de tipos do C decide se soma ou subtrai 0x20). Caps Lock não conta [código; **verificado** na bancada: `heal` e `HEAL` digitados com e sem Shift fazem efeitos diferentes].
6. O caractere vai para o anel de 4 posições (`_edata[pos]`, `pos = (pos + 1) mod 4`); a contagem sobe até 4.
7. Com 4 letras no anel, procura na tabela `_Cheats` um código igual às 4 últimas letras, da mais antiga para a mais nova. O teste `_Cheats != 0` antes da busca lê o primeiro byte da própria tabela (não é uma chave liga/desliga): os códigos **estão sempre ativos**.
8. Achou o código *k*: `_CheatKeys(k)`. O anel não é limpo: digitar `heal` e depois `l` não repete (o anel fica `eall`), mas `healheal` aplica duas vezes [código].

A tabela tem 26 códigos de 4 bytes guardados com os bits invertidos (`~c`), terminada por um 0. Lida da memória do original na bancada (`ler_variavel 8:83bc`) e do `SIMANTW.EXE` [**verificado**]:

`oops rand erad FUND joke hole HOLE qeen QEEN eggs EGGS heal HEAL zhea ZHEA just will mick jeff joel fred jenn susi data serv clie`

O índice *k* é a posição na tabela (0 = `oops`).

## 2. Efeitos (`_CheatKeys(k)`)

Antes de tudo, `_WinPrintf("CHEAT %d", k)` (saída de depuração, sem efeito no jogo). Sons por `_myBeginSound(id, 0, 0x7E)`. `(x, y)` da amarela = `_MeLocX`, `_MeLocY`.

| k | Código | Efeito [código] | Bancada |
|---|---|---|---|
| 0 | `oops` | som 10; `_QueenStorageR += 10` | **verificado** (0 → 10) |
| 1 | `rand` | som 1; `_SetMyHealth(100)` | |
| 2 | `erad` | som 1; `_SetMyHealth(1)` | |
| 3 | `FUND` | quadro de texto 10020 ("You are now $10,000 richer!! Unfortunately, money is worthless in this game.") | |
| 4 | `joke` | quadro 10022, `_UpdateEverything`, cursor de espera, `_myBeginSoundList(0)` (lista vazia no Windows, como no Mystery), quadro 10024 | |
| 5 | `hole` | `_MakeNewHoleB(c)` para c = 0..63 (64 buracos pretos novos) | |
| 6 | `HOLE` | `_MakeNewHoleR(c)` para c = 0..63 | |
| 7 | `qeen` | `_MakeBlkQueen(x + 2, y, 2)` (rainha preta nova no ninho preto) | **verificado** (`_BlkQueens` 1 → 2) |
| 8 | `QEEN` | `_MakeRedQueen(x + 2, y, 2)` | |
| 9 | `eggs` | 16 vezes: `_PlaceEggB(x + dx, y + dy, RRand(6) + 1)` com `dy = D8y[SRand8()]` sorteado **antes** de `dx = D8x[SRand8()]` (ver §2.1) | |
| 10 | `EGGS` | igual, `_PlaceEggR` e tipo `RRand(6) + 0x81` | |
| 11 | `heal` | `_HealthB = 100` | **verificado** (40 → 100; lido 99 um tick depois) |
| 12 | `HEAL` | `_HealthR = 100` | **verificado** (40 → 100) |
| 13 | `zhea` | `_HealthR = 0` | **verificado** (40 → 0) |
| 14 | `ZHEA` | `_AlwaysHealthy = 0`; `_HealthB = 0` | |
| 15 | `just` | `_QueenStorageB += 10`; `_InvalQueenStorageDisp`; som 0x29 | **verificado** (0 → 10) |
| 16 | `will` | alterna `_Invincible`; som 2 se ligou, 1 se desligou | |
| 17 | `mick` | alterna `_RevSpider`; som 2 / 1 | |
| 18 | `jeff` | som 2; para cada um dos 192 patches, `_YMapPopB[i] += 4`; `_BColoniesStarted += 16` por coluna (+192 no total) | |
| 19 | `joel` | som 1; para cada patch, `SRand2()`: 0 → `_YMapPopB[i] += 1` e `_BColoniesStarted += 1`; senão `_YMapPopR[i] += 1` e `_RColoniesStarted += 1` | |
| 20 | `fred` | `_DrawSimPayoff` (animação de vitória sobre o quintal, com as músicas 20003 e 20004; só desenho) | |
| 21 | `jenn` | alterna `_NeverHungry`; ligou: `_MeHealth = 100` e som 2; desligou: som 1 | |
| 22 | `susi` | alterna `_AlwaysHealthy`; ligou: `_HealthB = 100` e som 2; desligou: som 1 | **verificado** (0 → 1) |
| 23 | `data` | DDE com um "banco de dados" (`_GtInitiateDDE`); falha: caixa "Unable to access database." | |
| 24 | `serv` | NetBIOS, servidor ("SERVANT"); falha: caixa "Unable to access network." | |
| 25 | `clie` | NetBIOS, cliente ("CLIEANT"); falha: igual | |

Também **verificado** na bancada: `Heal` (Shift só no H) não faz nada, e `he` Enter `al` não faz nada (Enter limpa o anel).

Os valores somados não são limitados (`_QueenStorage*` passa de 10 a 20 etc.); os `_YMapPop*` são bytes e dão a volta depois de 255 [código].

### 2.1 Os ovos (`eggs`/`EGGS`)

Desmontagem de `1018:691c`: o tipo `RRand(6) + 1` (ou `+ 0x81`) é empilhado primeiro (terceiro argumento); depois `_SRand8()` indexa a tabela de 16 bytes em `[0xc28a]:8` (o y) e outro `_SRand8()` indexa `[0xc28c]:0` (o x). Os dois são deslocamentos de −1 a 1 [hipótese: a tabela é a `D8` de 16 bytes com o dx nos 8 primeiros e o dy nos 8 últimos, como `dx8`/`dy8` de `amarela.json`]. Como as duas direções são sorteadas separadamente, o ovo cai em qualquer uma das 9 células em volta (inclusive a da amarela). `_PlaceEggB/R` ignora coordenadas fora do ninho (ninho.md §7.6): no quintal o código não tem efeito útil. Sorteios: R 16 vezes, S 32 vezes.

### 2.2 `_GlobalKey` não é trapaça do Windows

`_DoSimYard` começa a chuva se `_GlobalKey` (`9:7866`) = 0xA8, e `_SimCat` testa 0x8D (quintal.md). Nenhuma rotina do pseudocódigo grava `_GlobalKey`, e a bancada sempre o leu 0 [código; provável resto da versão para Mac, hipótese]. Não há como ligar a chuva pelo teclado no original.

## 3. No port

- Regras e anel: `engine/src/rules/trapacas.rs`. Tabela e nome de cada efeito: `data/codigos_trapaca.json` (mod-ável: um mod pode trocar os códigos ou tirar um código da lista).
- A tecla chega por `Partida::tecla_de_trapaca` (`engine/src/core/partida.rs`) depois de `_YellowCommandKey`, como no original; Enter limpa o anel.
- `fred` pede `Evento::AnimacaoVitoria` (2026-10-18): a interface mostra `_DrawSimPayoff` (telas/mensagens.md §11.3; conferido na bancada, cenário `payoff`).
- Efeitos sem equivalente no port: `data`, `serv` e `clie` (DDE e NetBIOS). O port registra o código no console e não muda nada [divergência registrada]. `joke` mostra os dois quadros como o Mystery (não forçados: só com Options → Events).
- O quadro 10020 do `FUND` é não forçado (terceiro parâmetro 0 de `_PictStrnDialog`): entrou em `nao_forcadas` de `data/mensagens.json`.
