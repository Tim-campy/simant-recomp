# Geradores de números aleatórios

O SimAnt usa **dois geradores independentes**:

| Nome usado aqui | Rotinas | Estado | Algoritmo |
|---|---|---|---|
| **S** ("semente do mundo") | `_SRand1`, `_SRand2` … `_SRand256`, `_SGRand`, `_SGIRand`, `_SGSRand`, `_SetSRandSeed`, `_GetSRandSeed`, `_SeedSRand` | 1 palavra de 16 bits em `DGROUP:0xcbf2` (sem nome no `.SYM`) | LFSR de Galois de 16 bits |
| **R** ("rand do C") | `_RRand`, `_SeedRRand` (e as funções do runtime `_srand`/`_rand`) | 32 bits em `DGROUP:0xae34` (palavra baixa) e `0xae36` (palavra alta), sem nome no `.SYM` | congruencial linear do runtime do Microsoft C |

Fontes: pseudocódigo do Ghidra (`re_trabalho/decomp/Code5.c`, `Code4.c`, `Code2.c`) e desmontagem direta dos bytes do `SIMANTW.EXE` (capstone, modo 16 bits) para conferir instruções que o pseudocódigo deixa ambíguas (divisão com ou sem sinal, ordem de avaliação).

## Gerador S (LFSR)

### Passo

Fato visto no código (`_SRand1` @ `1020:158a`, e as variantes abaixo, conferido na desmontagem):

```
passo(s):            // s = palavra de 16 bits sem sinal
    vai_um = bit 15 de s
    s = (s << 1) & 0xFFFF
    se vai_um: s = s XOR 0x1BF5
    devolve s        // o novo estado é também o valor sorteado
```

Propriedades (fato verificado por enumeração dos 65.536 estados com um script Python):

- O passo é uma bijeção. Os ciclos são: o estado 0 (fixo: com semente 0 o gerador devolve 0 para sempre), um ciclo de 57.337 estados, um de 8.191 e um de 7 (`0x3859 → 0x70b2 → 0xe164 → 0xd93d → 0xa98f → 0x48eb → 0x91d6 → 0x3859`). Não é um LFSR de período máximo.
- O bit 0 do novo estado é igual ao bit 15 do estado anterior (porque 0x1BF5 é ímpar).
- Vetores de teste: semente 1 → 2, 4, 8, 0x10, …; semente 0x1234 → 0x2468, 0x48d0, 0x91a0, 0x38b5, 0x716a, 0xe2d4, 0xde5d, 0xa74f, 0x556b, 0xaad6.

### Funções de saída

Todas dão exatamente **um** passo, exceto as três "G", que dão dois ou três.

| Função | Endereço | Resultado | Observação |
|---|---|---|---|
| `_SRand1(n)` | `1020:158a` | `passo() mod n` | divisão **sem sinal** de 16 bits (`div`), resultado 0…n−1 |
| `_SRand2()` | `1020:15ae` | `passo() & 1` | |
| `_SRand4()` | `1020:15ce` | `passo() & 3` | |
| `_SRand8()` | `1020:15ee` | `passo() & 7` | |
| `_SRand16()` | `1020:160e` | `passo() & 0xF` | |
| `_SRand32()` | `1020:162e` | `passo() & 0x1F` | |
| `_SRand64()` | `1020:164e` | `passo() & 0x3F` | |
| `_SRand128()` | `1020:166e` | `passo() & 0x7F` | |
| `_SRand256()` | `1020:168e` | `passo() & 0xFF` | |
| `_SGRand(n)` | `1020:14a4` | `min(a, b)` com `a = SRand1(n)` e depois `b = SRand1(n)` | distribuição puxada para 0 |
| `_SGIRand(n)` | `1020:147c` | `max(a, b)`, mesma ordem | nenhum chamador no pseudocódigo |
| `_SGSRand(n)` | `1020:14cc` | `m = min(SRand1(n), SRand1(n))`; depois `SRand2()`; se der 1, devolve `−m` | valor com sinal, concentrado perto de 0 |

Atenção: `_SRandK()` (máscara) e `_SRand1(K)` (resto) dão o mesmo número para K potência de 2, mas o código usa as duas formas; para reproduzir o original basta que cada chamada dê um passo.

### Semente

- Valor inicial no executável: `DGROUP:0xcbf2 = 0` (fato verificado: bytes do segmento 10 no arquivo, offset de arquivo `0x20000 + 0xcbf2`).
- `_SetSRandSeed(v)` (`1020:1506`) grava v; `_GetSRandSeed()` (`1020:1512`) lê (devolve DX = 0). Fato visto no código.
- `_SeedSRand()` (`1020:152a`): `s = (_TickCount() & 0xFFFF) XOR 0x3751`. Fato visto no código.
- `_TickCount()` (`1008:18ac`) = `GetTickCount() / 55` (divisão longa sem sinal por 0x37), isto é, o relógio do Windows em "ticks" de ~55 ms, como no Mac. Os chamadores usam só a palavra baixa. Fato visto no código.

### Quem semeia S

| Momento | Chamada | Fonte |
|---|---|---|
| Inicialização do programa | `_initStuff` (`1020:0000`) chama `_BootUpGame` (`1000:96b2`, que sempre devolve 0) e então `_InitSimVars`, `_SeedSRand`, `_SeedRRand` | fato visto no código |
| Geração de cada patch do quintal | `_RandWorld` (`1030:5ab0`) começa com `_SetSRandSeed(semente do patch)` | fato visto no código |
| Tutorial carregado de `Tutorial.Ant` | `_SpecialTutorialInit` (`1010:9cea`) e `_LoadGame` (`1010:9fa0`) chamam `_GenerateTutorial` e depois `_SeedSRand` | fato visto no código |

Consequência: o mapa de um patch é **função só da semente do patch** (mais o estado do quintal, ver [geracao-mundo.md](geracao-mundo.md)); depois de gerado, S continua de onde a geração parou e passa a servir à simulação.

## Gerador R (`rand` do Microsoft C)

Fato visto no código (`_srand` @ `1018:06f6`, `_rand` @ `1018:070a`, conferido na desmontagem):

```
estado de 32 bits, inicial = 1 (bytes 01 00 00 00 em DGROUP:0xae34)
srand(v):  estado = v (palavra alta zerada)
rand():    estado = (estado * 0x343FD + 0x269EC3) mod 2^32
           devolve (estado >> 16) & 0x7FFF
```

É o gerador clássico do runtime C da Microsoft (multiplicador 214013, incremento 2531011).

- `_RRand(n)` (`1020:156e`): `v = rand()`; se `v < 0`, `v = −v` (nunca acontece, `rand` é 0…0x7FFF); devolve `v mod n` com divisão **com sinal** (`idiv`). Fato visto no código.
- `_SeedRRand()` (`1020:1536`): faz `s = TickCount XOR 0x3751` (igual a `_SeedSRand`), sorteia `k = SRand128()` (avança S), chama `srand(TickCount)` (nova leitura do relógio) e descarta `k` valores de `rand()`. Fato visto no código.
- `_SetRRandSeed` (`1020:1518`) é vazia. `_GetRRandSeed` (`1020:151a`) lê uma palavra dupla em `0x046C:0000` (o endereço linear do contador de ticks do BIOS, 0040:006C, usado como se fosse seletor); não tem chamadores no pseudocódigo. Ambas parecem resíduo do port do DOS [hipótese].

## Quem usa cada gerador

Contagem de chamadas no pseudocódigo (fato visto no código; lista completa obtida com `grep` em `re_trabalho/decomp/*.c`):

- **R (`_RRand`)**: só 9 rotinas. `_RandYard` (sementes dos 192 patches), `_PlaceDrop` (posição das gotas de chuva), `_SimRain`/`_DoSimYard` (sorteio do início da chuva), `_GetWinner` (lutas), `_MysteryButton`, `_CheatKeys`, `_AnimYellowInsane`.
- **S**: todo o resto. Geração do mapa (`_MakeYardPatch`, `_AddRocks`, `_InitAntLions`, `_InitPillar`, `_RandWorld`, `_PlaceBlackQueen`, `_AddFood`, `_DigTileB/R` e `_SmoothEdgesB/R` via `_SRand8`), a simulação das formigas (`SIMANT1_MODULE`), quintal (menino, gato, cachorro, pássaro, aranha, formiga-leão), e **também rotinas de desenho**: `_DrawCurBalloons`, `_DrawSpider`, `_DrawSwarm`, `_Draw_SimYard`, `_DrawRain` chamam `_SRand*`.

Consequência importante para o port: como o desenho consome S, a sequência da simulação no original depende do que foi desenhado (janelas abertas, balões). Uma reprodução bit a bit da simulação exige ou reproduzir essas chamadas de desenho, ou aceitar divergência depois da geração do mundo [hipótese: não medido quanto isso afeta na prática]. A geração do mundo em si não chama desenho entre o `_SetSRandSeed` e o fim do `_RandWorld` (fato visto no código, e confirmado pela reprodução exata descrita abaixo).

## A semente está no save?

- O estado de S (`DGROUP:0xcbf2`) e o de R (`0xae34`) **não** estão na tabela de blocos do `.ANT` (fato visto no código: a tabela `1038:9570` lida por `_SaveGame`/`_LoadGame`, listada em [../formatos/save-ant.md](../formatos/save-ant.md), não tem nenhum bloco nesses endereços).
- O que está no save é **`_YMapSeeds`** (`1038:0224`, 192 palavras, bloco 29, offset 45328 do arquivo): a semente de cada patch do quintal, índice `y * 12 + x`. Com ela o patch atual pode ser regenerado. Fato visto no código e verificado no `QUICK2.ANT`.
- Ao carregar, `_LoadGame` chama `_RandYard` antes de ler os blocos (fato visto no código), o que reinicia S pela semente de um patch novo; o estado exato de S no momento do save se perde. Logo, depois de um load, a simulação do original não segue a mesma sequência aleatória que teria sem salvar [hipótese, a medir].

## Verificação feita

Fato verificado (2026-10-16): com a semente do patch atual lida do `QUICK2.ANT` (`_YMapSeeds[8 * 12 + 11] = 13106 = 0x3332`) e o passo acima, os 8.192 sorteios `_SRand16` da superfície reproduzem **todas** as 7.415 células de chão do `_MapA` salvo, e o restante da geração (ver [geracao-mundo.md](geracao-mundo.md)) reproduz `_MapB` e `_MapR` inteiros e o `_MapA` exceto os bichos que se moveram. Script: `re_trabalho/verifica_geracao_quick2.py` (material de trabalho, fora do git).

## Como testar contra o original

1. **Sem emulador (já feito):** rodar `python re_trabalho/verifica_geracao_quick2.py`; ele lê `tests/referencias/saves/QUICK2.ANT`, regera o patch e imprime as diferenças. No port, o mesmo caso é o teste `engine/tests/geracao_quick2.rs`, e os vetores desta página estão nos testes unitários de `engine/src/rules/rng.rs` (implementação: `GeradorS` e `GeradorR`).
2. **Semente forçada na bancada QEMU** ([../../bancada-qemu.md](../../bancada-qemu.md)): o comando `memoria` dá dumps da RAM física. Para forçar S, é preciso achar o endereço físico do DGROUP (procurar no dump um padrão conhecido do DGROUP, como a tabela `DGROUP:0x230c = 19 1a 1c 1f 1e 1d 1b 18`) e escrever na palavra `0xcbf2` com o monitor do QEMU (comando HMP `pmemsave`/`xp` para ler; escrita exige `gdbstub` ou um comando novo na bancada). **Feito:** a bancada tem `escrever_variavel SementeS ...` (gdbstub) e o modo tick; com as sementes forçadas, duas execuções dão a mesma sequência de S e R tick a tick (verificado, ver [testes-tick.md](testes-tick.md)). Atenção: `_SimBird` só sorteia `SRand2` quando `_gameCycles` (contador do laço do `MYTIMERFUNC`, não de ticks) passou de `_BirdDelay`; em velocidade normal isso vale em todo tick.
3. **R:** para testar `_RRand`, gerar dois Quick Games seguidos na mesma sessão e comparar `_YMapSeeds` dos dois saves: a segunda tabela deve ser a continuação da sequência `rand()` da primeira (192 chamadas depois), desde que nada mais tenha chamado `_RRand` entre os dois (chuva, lutas) [hipótese a confirmar].
