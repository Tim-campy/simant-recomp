# Som e música: arquivos e formatos

Onde estão os sons e as músicas do SimAnt para Windows, em que formato, e por qual caminho o executável os toca. Quando cada um toca está em [regras/som.md](../regras/som.md).

## Resumo

| Conteúdo | Onde | Formato | Quem usa |
|---|---|---|---|
| Efeitos sonoros | banco `SOUND`, tipo **5**, ids 0–56 | amostras de 8 bits comprimidas em deltas de 4 bits ("instrumento DAC") | `_myBeginSound` com MMSYSTEM (`waveOutWrite`) ou Sound Blaster (`vocPlayMemUnFormat`) |
| Efeitos sonoros (versão Mac) | banco `SOUND`, tipo **51**, 34 ids entre 1 e 49 | recurso `snd ` do Macintosh (formato 1 ou 2) | `_myBeginSound` só com `DSOUND.DLL` (Disney Sound Source) |
| Músicas (Windows padrão) | arquivos `SOUND\*.MID` da pasta do jogo (29 arquivos) | MIDI padrão (SMF formato 1), arranjo duplo do Windows 3.1 | `_myBeginSong` via MCI (`mciSendCommand`) |
| Cabeçalhos de música | banco `SOUND`, tipo **18**, 33 ids (10001–20011) | 18 bytes, 9 palavras big-endian | `_myBeginSong` (todos os caminhos) |
| Músicas (Sound Blaster) | banco `SOUND`, tipo **20**, 30 ids | MIDI padrão, um só arranjo (canais 2–4 e 10) | `_myBeginSong` via `SNDBLST.DLL` (`musPlayMemMidi`) |
| Músicas (alto-falante do PC) | banco `SOUND`, tipo **50**, 28 ids | MIDI padrão reduzido a uma voz | `_myBeginSong` via `SOUND.DRV` (`SetVoiceNote`) |

**Fonte:** fato verificado com `tools/db_list.py` e dumps dos objetos (contagens, tamanhos, assinaturas `MThd`); caminhos de reprodução vistos no código (`GR_MODULE`, segmento `1008`), endereços abaixo.

## Saída de áudio escolhida pelo original

`_snd_Install` (`1008:793a`), chamada uma vez na inicialização logo depois de `_db_SetDataBase("sound")`, testa as bibliotecas nesta ordem (fato visto no código; nomes das DLLs e funções são strings do DGROUP, offsets `0xafa`–`0xbcf`):

1. `MMSYSTEM.DLL` (procurada na pasta do Windows, em `SYSTEM\` e na pasta do jogo por `_IsDLLAvail` `1008:780e`). Se `midiOutGetNumDevs()` > 0: **modo multimídia** (`[0x8d0a]` = 1). Músicas por MCI, efeitos por `waveOut*`.
2. Senão, `SNDBLST.DLL` com `sbcGetDLLVersion` e `sbcGetCardName`: **modo Sound Blaster** (`[0x8d0a]` = 0). Músicas por `musOpenDevice`/`musPlayMemMidi`, efeitos por `vocOpenDevice`/`vocPlayMemUnFormat`.
3. Senão, de novo `MMSYSTEM.DLL`, agora só com `waveOutGetNumDevs()` > 0: efeitos por `waveOut*`, música pelo alto-falante (`[0x8d0c]` = 1).
4. Sem nada: música pelo `SOUND.DRV` do Windows 3.0 (importado por ordinal: 1 `OpenSound`, 2 `CloseSound`, 3 `SetVoiceQueueSize`, 4 `SetVoiceNote`, 9 `StartSound`) e efeitos desligados.

Além disso, com MMSYSTEM e sem `waveOut` [hipótese: a condição exata não foi seguida], tenta `DSOUND.DLL` (`GetDSoundVersion` ≥ 2) e, se houver, os efeitos usam os objetos tipo 51 por `PlaySound`.

Numa instalação comum do Windows 3.1 com placa de som (inclusive a de referência, com Sound Blaster 16 no DOSBox-X), o caminho é o **1**: MIDI por MCI a partir de `SOUND\*.MID` e efeitos do tipo 5 por `waveOut`. **O port reproduz esse caminho.** Os tipos 20 e 51 ficam como alternativa de dados; o tipo 50 não é usado.

### `SIMANT.CFG` não escolhe a saída

`_ReadConfig` (`1008:08f4`) grava o dígito de "Sound Mode" em `_musicDevice`, mas nenhuma função lê `_musicDevice` no pseudocódigo: as três referências são escritas (`_IBMInitStuff` `1008:0220` põe −1 ou um dígito; `_ReadConfig` põe o dígito do arquivo). `_snd_Install` não consulta a variável, e `_MusicInit` (`1008:827c`) só registra "MUSIC INIT!" no log de depuração. Conclusão: no Windows o "Sound Mode" não tem efeito (visto no código; uma leitura indireta que o Ghidra não resolveu não está descartada). Ver [simant-cfg.md](simant-cfg.md).

## Tipo 5: efeito sonoro ("instrumento DAC")

57 objetos, ids 0–56, todos com flags 8 (sem compressão LZSS). Tamanho de 67 a 6464 bytes.

| Offset | Tamanho | Campo |
|---|---|---|
| 0 | 16 bytes | tabela de deltas: 16 valores **com sinal** (8 bits), indexados por um nibble |
| 16 | N − 16 bytes | dados: cada byte guarda dois nibbles, primeiro o **alto**, depois o baixo |

Decodificação (fato visto no código, três cópias idênticas: `_UncompressDACInstrument` `1008:768c`, `_MciOutWave` `1008:959c` e o caminho `sndPlaySound` de `_myBeginSound`):

```
amostra = 0x80
para cada byte b dos dados:
    amostra = (amostra + tabela[b >> 4])  mod 256 ; emite amostra
    amostra = (amostra + tabela[b & 0xF]) mod 256 ; emite amostra
```

- Resultado: PCM **8 bits sem sinal** (0x80 = silêncio), mono, **2 × (N − 16)** amostras.
- A soma é em 8 bits com estouro (aritmética de `char`), sem saturação.
- **Taxa:** o `WAVEFORMAT` montado para `waveOutOpen` tem `nSamplesPerSec` = `nAvgBytesPerSec` = **0x1000 = 4096 Hz**, `nChannels` = 1, `nBlockAlign` = 1, `wBitsPerSample` = 8 (fato visto na desmontagem: `mov word [bx+4], 0x1000` em `1008:9d7e`). O caminho Sound Blaster também passa 0x1000 a `vocPlayMemUnFormat` (`1008:9b01`) e o caminho `sndPlaySound` monta um cabeçalho RIFF com os mesmos valores.
- Observação: comparando com as versões Mac (tipo 51), o número de amostras de vários sons bate com uma taxa de ~5563 Hz (ex.: id 41 tem 12 896 amostras aqui e 13 184 no `snd ` a 5563,6 Hz; id 15 tem 102 amostras contra 205 a 11 127 Hz). Ou seja, o Windows toca os efeitos cerca de 26 % mais lentos e graves que o Mac. O port segue o Windows (4096 Hz) por padrão; a taxa fica em `data/som.json`.

## Tipo 51: efeito sonoro em formato Mac

34 objetos (ids 1–49 com lacunas), com e sem LZSS. São recursos `snd ` do Macintosh (fato verificado pelo dump; formato da Apple, *Inside Macintosh: Sound*):

- formato 1: `0001` (formato), `0001` (um sintetizador), `0005` (sampled synth) + 4 bytes de inicialização, `0001` comando, `8051` (`bufferCmd` com o bit de offset), parâmetros `0000` e `00000014` → cabeçalho de som no offset 20;
- formato 2 (só o id 23): `0002`, `0000` (referências), `0001` comando, `8050`... → cabeçalho no offset 14;
- cabeçalho de som (big-endian): ponteiro (0), número de amostras (u32), taxa em ponto fixo 16.16 (ex.: `15bba2e8` = 5563,6 Hz, `2b7745d2` = 11 127 Hz, `56ee8ba3` = 22 254,5 Hz), início e fim do laço, codificação (0), nota base (`3c`), seguidos das amostras de 8 bits sem sinal.

Só a variante com `DSOUND.DLL` usa esses objetos. O port não os toca [em aberto: oferecer como opção "sons do Mac"].

## Tipo 18: cabeçalho de música

33 objetos de 18 bytes = 9 palavras **big-endian** (dados vindos do Mac; `_FlipWord` em `_myBeginSong`).

| Palavra | Campo | Evidência |
|---|---|---|
| 0 | id do objeto MIDI (tipos 20/50) | fato visto no código: lido em `_myBeginSong` e passado a `_db_LoadObject(x, 0x14)` / `(x, 0x32)`; os valores existentes casam com os ids dos tipos 20 e 50 |
| 1 | 4 nos temas longos (10001–10010), 1 nos demais | [significado desconhecido] |
| 2 | sempre 0 | — |
| 3 | **transposição em semitons** (0, 6 ou 12) | fato visto no código: passada a `musTransposeNote` no modo Sound Blaster; o log usa o texto `"myBeginSong(shift=%d)..."` |
| 4, 5 | pares (8,3), (5,5), (6,4), (4,2) | [significado desconhecido] |
| 6 | 0x4410, 0x410C, 0x400C, 0x4010, 0x4008 | [significado desconhecido] |
| 7, 8 | 0 | — |

Uma música é pedida pelo **id do cabeçalho**. Vários cabeçalhos apontam para o mesmo MIDI com transposições diferentes: 11014 = 11013 + 6 semitons, 11015 = 11013 + 12, 11017 = 11016 + 12. Não há cabeçalho para 20006 (`jolly5`), 11008–11012 nem 11019.

Se o cabeçalho não existe, `_myBeginSong` retorna sem tocar nada, em qualquer modo (fato visto no código: `_db_LoadObject(id, 0x12, 1)` nulo → retorno).

## Músicas por MCI: `SOUND\*.MID` e a tabela de nomes

No modo multimídia, `_myBeginSong(id)` procura `id` numa tabela de 31 pares (nome, id) terminada por −1 e toca o arquivo `<pasta do executável>sound\<nome>.mid` (formato `"%ssound\%s.mid"`, DGROUP `0xe3c`) com `mciSendCommand` (abrir com o *MIDI Mapper*, `MCI_SEEK` ao início, `MCI_PLAY` com notificação). O id **não** é usado para escolher o objeto tipo 20 nem a transposição do cabeçalho: no modo MCI a palavra 3 é ignorada.

A tabela fica no DGROUP: ponteiros far para os nomes em `[0x8d2a]` (31 × 4 bytes) e os ids em `[0x8da6]` (31 palavras + −1). No arquivo do executável essa área está zerada; os valores abaixo foram lidos da memória do jogo em execução (fato verificado no dump de RAM `re_trabalho/qemu_ram_quick2.bin`, bancada QEMU; os nomes são as strings do DGROUP `0xd05`–`0xdee`):

| Id | Arquivo | | Id | Arquivo |
|---|---|---|---|---|
| 10001 | gamethme | | 11004 | newcoln2 |
| 10003 | antthme1 | | 11005 | newbody |
| 10004 | antthme2 | | 11006 | qn4day |
| 10005 | antthme3 | | 11007 | badnews |
| 10006 | hmanawar | | 11013 | call2arm |
| 10007 | hmanthme | | 11014 | call2arm |
| 10008 | victory | | 11016 | victqndd |
| 10010 | defeat | | 11017 | victqndd |
| 20000 | happy1 | | 11018 | attack |
| 20001 | happy2 | | 11020 | blackq |
| 20002–20006 | jolly1–jolly5 | | 11021 | redq |
| 20007 | omnious1 | | 20008–20011 | sad1–sad4 |

Consequências (fato visto no código, a confirmar ouvindo o original):

- 11015 tem cabeçalho mas não está na tabela: **não toca** no modo MCI (`_myBeginSong` retorna quando a busca chega ao −1). Idem 11002 e 11003.
- 20006 (`jolly5`) está na tabela mas não tem cabeçalho: também **não toca**. O arquivo `JOLLY5.MID` existe sem uso.
- 11014 e 11017 tocam o mesmo arquivo que 11013 e 11016, **sem** a transposição.

### Formato dos arquivos `.MID`

MIDI padrão (SMF), formato 1, divisão 240 ticks por semínima, com metaeventos de andamento (`FF 51`), SMPTE (`FF 54`) e compasso (`FF 58`). Fato verificado com um analisador simples (contagem de notas por canal):

- Todos seguem as **diretrizes de autoria MIDI do Windows 3.1** (arranjo duplo): a mesma música aparece duas vezes, no arranjo "estendido" (canais 1–10, percussão no 10) e no "básico" (canais 13–16, percussão no 16), com as mesmas notas e os mesmos programas. Exemplo: `GAMETHME.MID` tem 346 notas no canal 3 e 346 no 13, 456 no 10 e 456 no 16.
- O *MIDI Mapper* do Windows 3.1 toca só um dos arranjos, conforme a configuração do sintetizador. Tocar todos os canais dobra cada nota. **O port toca só os canais 1–10** (lista em `data/som.json`, `canais_midi`).
- Os programas são General MIDI (ex.: 34 baixo, 3 piano elétrico, 95 pad), e o canal 10 recebe programa 126 [hipótese: sobra da conversão; em GM, programa no canal de percussão escolhe o kit].

### Tipo 20 (Sound Blaster) e tipo 50 (alto-falante)

MIDI padrão também (assinatura `MThd`), com LZSS em parte deles. Não são cópias dos `.MID` (nenhum hash coincide): o tipo 20 tem um só arranjo, em geral nos canais 2–4 e percussão no 10 (ex.: 10001 com 182/346/98 notas nos canais 2/3/4 e 597 no 10); o tipo 50 é uma redução a uma voz (10001: só o canal 3), para o `SetVoiceNote`. No modo Sound Blaster, `_myBeginSong` carrega o tipo 20 do id da palavra 0 do cabeçalho, aplica a transposição (`musTransposeNote`) e toca com `musPlayMemMidi`; no modo alto-falante interpreta o tipo 50 em `_myBeginSong` (`"Unknown command"`, `"Meta Event"`, `"Program Change"`...).

## Port

- **Leitura:** `engine/src/formats/som.rs` (tipo 5 → PCM, tipo 18 → `CabecalhoMusica`, gravação de WAV) e `engine/src/formats/midi.rs` (SMF → lista de eventos com tempo em segundos, filtro de canais e transposição). Testes em `engine/tests/som_reais.rs`, contra os arquivos do usuário.
- **Reprodução:** `simant/src/som/` (SDL2 + sintetizador), ver [regras/som.md](../regras/som.md) e `docs/decisoes.md`.
- **Mods:** um efeito pode ser trocado com `objetos/5/<id>.bin` (mesmo formato DAC); uma música com `arquivos/sound/<nome>.mid`; a tabela id → arquivo, a taxa, os volumes e os canais ficam em `data/som.json`.
