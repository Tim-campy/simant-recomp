# `SIMANT.CFG`

Arquivo de texto na pasta do jogo. A cópia analisada contém:

```
Display Mode: ?
Sound Mode: 6
```

## Leitura

**Fonte:** fato visto no código, `_ReadConfig` (`1008:08f4`).

- Lê duas palavras separadas por espaço ("Display", "Mode:") e depois **um único caractere**, que é o modo de vídeo.
- Lê mais duas palavras ("Sound", "Mode:") e um caractere. Se for dígito, `_musicDevice = dígito`.
- Se o arquivo não existir, mostra "SimAnt configuration file missing" e encerra.
- Um caractere de vídeo desconhecido mostra "Bad 'Display Mode' in configuration file" e encerra.

## Modo de vídeo → `_displayType` → banco gráfico

| Caractere | `_displayType` | Prefixo do banco | Banco resultante (com sufixo `nt`) | Na cópia analisada |
|---|---|---|---|---|
| `E` | 0 | `hcega` | `HCEGANT` | ausente |
| `T` | 2 | `tdyga` | `TDYGANT` | ausente |
| `H` | 3 | `mono` | `MONONT` | ausente |
| `e` | 4 | `lcega` | `LCEGANT` | ausente |
| `M` | 5 | `mono` | `MONONT` | ausente |
| `m` | 7 | `mono` | `MONONT` | ausente |
| `V` | 8 | `hcega` | `HCEGANT` | ausente |
| `w` | 9 | `mwin` | `MWINNT` | presente |
| `W` | 10 | `winga` | `WINGANT` | presente |
| `?` | automático | — | 9 se o vídeo tiver 1 plano e 1 bit por pixel (`GetDeviceCaps` PLANES = BITSPIXEL = 1); senão 10 | — |

Os prefixos vêm da tabela de ponteiros far em `1048:b92b` usada por `_InitGraphics` (`1008:010c`). O sufixo `nt` é uma [hipótese] apoiada pelos nomes dos arquivos existentes. Os modos sem arquivo correspondente parecem herança das versões DOS. O port só precisa suportar `W` (cor) e, opcionalmente, `w` (mono).

## `Sound Mode`

O valor vai para `_musicDevice`. O significado de cada número ainda não foi levantado. Os candidatos estão em `_MusicInit`, `_snd_Install`, `_IsMMMidiAvail` e `_IsMMWaveAvail` (`GR_MODULE`) [em aberto].

## Port

O port não precisa deste arquivo. A escolha de banco gráfico e de saída de áudio vai para a configuração própria do port. O `.CFG` original pode ser lido apenas como valor padrão.
