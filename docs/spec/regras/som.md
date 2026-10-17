# Som e música: quando tocam

Formatos e caminhos de reprodução: [formatos/som.md](../formatos/som.md). Aqui: as funções que o resto do jogo chama, as opções do jogador, a preempção, e o catálogo de quem toca o quê.

## Funções de som (`GR_MODULE`)

| Função | Endereço | O que faz (modo multimídia, o caminho do Windows 3.1 com placa de som) |
|---|---|---|
| `_myBeginSong(id, prioridade)` | `1008:858e` | Sai se o som não foi instalado ou se `_songsOnFlag` = 0. Carrega o cabeçalho tipo 18 `id` (sai se não existe), procura `id` na tabela de nomes (sai se não está), fecha a música MCI aberta, se houver, e abre/toca `sound\<nome>.mid`. A prioridade (0x7e, 0x3f) não é usada. |
| `_StopSong()` | `1008:8294` | Fecha o dispositivo MCI (`MCI_CLOSE`). |
| `_mySongIsDone()` | `1008:a1a0` | No modo multimídia devolve **sempre verdadeiro**; só o modo Sound Blaster consulta o fim real. |
| `_myServiceSong()` | `1008:a216` | Com música MCI aberta, um `PeekMessage` para a notificação do MCI andar. |
| `_myBeginSound(id, taxa, prioridade)` | `1008:98b0` | Sai se o som não foi instalado ou se `_effectsOnFlag` = 0. Carrega o tipo 5 `id`, decodifica (formatos/som.md) e toca a 4096 Hz por `waveOutOpen`/`waveOutWrite`. O segundo argumento só é usado com `DSOUND.DLL` (é a palavra alta da taxa do Mac: 0x56ee = 22 254 Hz, 0x2b77 = 11 127 Hz). A prioridade não é usada. |
| `_myBeginSoundReverse(id, …)` | `1008:a18c` | Igual a `_myBeginSound` (o "reverso" não é feito no Windows). |
| `_mySoundIsDone()` | `1008:a188` | Sempre 1. |
| `_myBeginSoundList`, `_myBeginSoundSection` | `1008:8292`, `1008:a264` | Vazias. |
| `_SysBeep`, `_Beep` | `1008:828a`, `1008:a266` | `MessageBeep`. |

Fato visto no código para toda a tabela.

### Um efeito por vez; o novo interrompe o anterior

Há um único dispositivo `waveOut` aberto. Se, ao pedir um efeito, o anterior ainda está tocando, `_myBeginSound` compara `GetTickCount()` com um prazo gravado no início do efeito anterior (`GetTickCount()` + a posição devolvida por `waveOutGetPosition` em milissegundos, que é ~0 logo depois do `waveOutWrite`). Se o prazo ainda não passou, o pedido é **descartado**; senão, faz `waveOutReset` e toca o novo (fato visto no código de `_myBeginSound`; o prazo fica em `1048:1238`). Na prática o prazo é o próprio instante de início, então só um pedido no mesmo milissegundo é descartado: o efeito novo corta o anterior [hipótese apoiada pelo código; não medido no original].

Músicas também: abrir uma nova fecha a anterior. Música e efeito tocam juntos (dispositivos diferentes).

## Opções do jogador

- **Menu Options → Music** (comando 0x32, `_ProcMenu` `1000:4fc2`): alterna `_OptionStates[1]` (`[0x85f4]`); se desligou, `_StopSong()`.
- **Options → Sound Effects** (comando 0x33): alterna `_OptionStates[2]` (`[0x85f6]`).
- `_songsOnFlag` e `_effectsOnFlag` copiam essas opções; são as que `_myBeginSong`/`_myBeginSound` testam.
- **Persistência:** na inicialização (`1000:3832`–`3865`) as opções vêm do `WIN.INI`, seção `[SimAnt]`, chaves `autotrack`, `music`, `effects`, `events`, `messages`, `silly` (`GetProfileInt`, padrão = valor atual da variável). Não vêm do `SIMANT.CFG`. [em aberto: onde são gravadas de volta.]
- **Janela inativa** (`MAINWNDPROC`, `WM_ACTIVATEAPP`, `1000:2930`): ao perder o foco, `_StopSong()`, guarda as duas opções e zera opções e flags (silêncio total); ao voltar, restaura. A música interrompida não recomeça sozinha.

Fato visto no código.

## Músicas: quem pede

Ids de cabeçalho (decimal). "Arquivo" pela tabela MCI de [formatos/som.md](../formatos/som.md); "—" = não toca no modo MCI.

| Id | Arquivo | Quem pede | Situação |
|---|---|---|---|
| 10001 | gamethme | `_ShowIntro` (`1000:d3e2`), `_ProcMenu` comando 3 (Game → New) antes de `_NewGame` | abertura e jogo novo |
| 10003–10005 | antthme1–3 | `_TryAntTheme` (`1020:1414`), chamada por `_EnterNest`, `_ExitNest` e `_DoAntMoveY` | ao entrar/sair do ninho com a amarela, se já passaram **7200 ticks de `_TickCount`** (~396 s) desde o último tema; cicla 10004, 10005, 10003, … (contador em `[0x80ba]`) |
| 10003/10004 | antthme1/2 | `_EndGameDialog` (`1000:6602`) | depois da música de fim, `10003 + _SRand2(…)` [hipótese: sorteio entre 0 e 1], uma vez, quando `_mySongIsDone` (imediato no MCI) |
| 10008 / 10010 | victory / defeat | `_EndGameDialog` | abre o diálogo de fim com vitória (`[0x9cea]` ≠ 0) ou derrota, se `_OptionStates[1]` |
| 11002 | — | `_XferPatch` (`1000:ab90`) | mudança de patch no quintal |
| 11003 | — | `_SpecialXfer`, `_PlaceQueenInYard`, `_EditToolsMenu`, `_MapToolsMenu`, `_RibbonToolsMenu` | — |
| 11005 / 20001 | newbody / happy2 | `_YellowBirth` (`1000:739e`) | renascer como nova formiga / fundar colônia |
| 20000 | happy1 | `_YellowBirth` | início da fundação |
| 11006 | qn4day | `_ExchangeLives` (`1020:67ac`) | a amarela vira rainha |
| 11013–11017 | call2arm / victqndd | `_YellowCommand` (`1000:87ba`) | recrutar 5, 10, todas; liberar metade, todas (11015 não toca no MCI) |
| 11018 | attack | `_GstrR` (`1030:03c2`), `_StartAttack` (`1030:050e`) | ataque vermelho |
| 11020 / 11021 | blackq / redq | `_CountAnts` (`1020:04de`) | rainha preta / vermelha morreu (com o diálogo 10010/10012) |
| 20002, 20003, 20005 | jolly1, jolly2, jolly4 | `_SimColonies` (`1030:35f8`) | quintal: vermelhas eliminadas, casa tomada, vitória (quintal.md) |
| 20003, 20004 | jolly2, jolly3 | `_DrawSimPayoff` (`1000:cda8`) | em sequência enquanto mostra a pontuação (no MCI, como `_mySongIsDone` é imediato, a segunda corta a primeira) |
| 20007–20011 | omnious1, sad1–4 | `_YellowDeath` (`1000:7746`) | morte da amarela: `0x4E27 + S1(5)` |

Fato visto no código (chamadas em `Code1/3/5/6/7`), com os ids conferidos nos cabeçalhos tipo 18.

## Efeitos: quem pede

Ids do tipo 5. Os nomes são o contexto da chamada, não um rótulo do original. Os que dependem de sorteio ou de interface já têm os sorteios reproduzidos nas regras (`_MakeBark`, `_SimCat`, mastigação etc.).

| Id | Quem pede (função, endereço) |
|---|---|
| 1 | erro: `_GotoMyAnt` no Experimental, `_DoBookMark`, `_XferPatch`, `_PlaceQueenInYard`, `_YellowDeath`, `_SpecialXfer`, `_processSpider`, `_DoAntMoveY`, `_ExpDig`, `_ProcYardEvent`, `_CheatKeys`, comandos da amarela que falham (`_YellowCommand`, `_SetAlarmDropState`) |
| 2 | `_EditToolsMenu`, `_MapToolsMenu`, `_RibbonToolsMenu`, `_CheatKeys`, aranha: comando 11 |
| 7 | `_SimBird` (`1030:28aa`): pássaro |
| 9 | `_SimKidInside` (`1030:20b6`), `_ExpKillAnts` |
| 10 | `_EatMyFood`, `_MoveSpider`, `_CheatKeys` |
| 13 | `_SimCat` (`1030:2b24`): gato, quando `_SRand64()` = 0 no quintal |
| 15 | ok/centralizar: `_YellowCommandKey` (Home), `_processSpider`, `_ExchangeLives`, `_SetAlarmDropState` (ligou) |
| 17 / 18 | escavação no ninho preto / vermelho: `_DoDigInB` (`1028:4bd0`) / `_DoDigInR` (`1028:6f0c`) (ninho.md: "som 0x11 (R: 0x12)") |
| 19 | `_SetLife` (`1020:5d18`): a amarela cava ao entrar na terra |
| 21 / 22 / 23 | latido do cachorro: `_MakeBark` (`1030:32ea`) tipo 1 / 3 / 0 e 2; `_SimDog`, `_FollowBoyDir` (21) |
| 24 | `_SimKidInside` |
| 27 | `_ExpIncSmell` |
| 28 | ovo/objeto: `_DropMyEgg`, `_PickupMyEgg`, `_PickupMyObject`, `_TryMyDropOrLift`, `_ExpAddAnt`, `_YellowBirth` |
| 29 | comida: `_DropMyFood`, `_PickupMyFood`, `_ExpAddFood`, `_DoTool` |
| 30 | terra/pedrinha: `_DropMyRock`, `_PickupMyRock`, `_DoAntLions` |
| 32 | `_AddFood`, `_DoTool`, `_ExpAddFood`, `_MysteryButton` |
| 35 | passo humano: `_FootFall` (`1030:3386`) |
| 36 | cortador de grama: `_DrawMower` (`1010:b2c4`) |
| 37 | luta: `_AnimYellowFight` (`1028:7e56`), `_LionDialog` |
| 38 | formiga-leão: `_LionDialog`, `_DoAntLions` |
| 41 | água: `_DoWater` (`1020:07c6`); aranha: comando 12 (laser) |
| 42, 45 | `_YellowDialog`, `_SpiderDialog` |
| 44 | comer: `_EatMyFood`, `_MoveSpider` |
| 47 | `_MoveSpider` |
| 49 | `_YellowBirth`, `_AnimYellowFight` |
| 55 | laser: `_DoLaserFire` (`1000:6ec8`) |

Os ids 0, 3–6, 8, 11, 12, 14, 16, 20, 25, 26, 31, 33, 34, 39, 40, 43, 46, 48, 50–54 e 56 não aparecem como constante nas chamadas vistas [em aberto: podem vir de variáveis, como em `_MysteryButton` e `_CheatKeys`].

## Port

**Regras → interface.** As regras não tocam som: pedem `Evento::Musica(id)` (id do cabeçalho tipo 18) e `Evento::Som(id)` (id do tipo 5) na fila do `Estado` (`engine/src/rules/estado.rs`). Hoje as regras emitem as músicas da amarela, dos inimigos, do quintal e da superfície; **nenhuma regra emite `Evento::Som` ainda** (o variante foi criado para isso; as chamadas acima indicam onde).

**Executável.** `simant/src/som/` recebe os eventos e toca com SDL2:

- Música: procura o id em `data/som.json` → `musicas` (a tabela MCI acima) e, se `exigir_cabecalho`, exige o tipo 18 no banco, como o original. Lê `sound/<nome>.mid` da pasta do jogo (`Recursos::arquivo`, sem diferenciar maiúsculas; mod: `arquivos/sound/<nome>.mid`). Com `fonte_musica` = `"banco"`, usa o tipo 20 da palavra 0 do cabeçalho e a transposição da palavra 3 (o caminho Sound Blaster). Toca só `canais_midi` (1–10). Uma música nova corta a anterior.
- Efeito: tipo 5 decodificado e reamostrado de `taxa_efeitos` (4096 Hz) para a saída. Um efeito novo corta o anterior.
- Opções: segue `_OptionStates[1]` (música) e `[2]` (efeitos) do estado da partida; desligar a música para a que toca. Perder o foco da janela para a música (como `WM_ACTIVATEAPP`).
- Abertura e jogo novo tocam `musica_abertura` e `musica_jogo_novo` (10001).
- `--sem-som` desliga tudo; `--exportar-som DIR` grava os efeitos e as músicas decodificados em WAV, sem abrir áudio.

**Sintetizador.** Ver `docs/decisoes.md` (2026-10-17). Com uma SoundFont (`--soundfont`, `SIMANT_SOUNDFONT` ou `soundfont` em `data/som.json`), usa `rustysynth`; sem ela, um sintetizador simples próprio (ondas básicas por família de instrumento GM, ruído para a percussão). O timbre, portanto, não é o do original (que dependia da placa e do *MIDI Mapper*): isso está em `docs/divergencias.md`.

**Não reproduzido:** os modos Sound Blaster, alto-falante e Disney Sound Source como caminhos automáticos; o descarte de efeito pedido no mesmo milissegundo; `_mySongIsDone` falso no modo Sound Blaster (no port as músicas seguidas tocam como no MCI).
