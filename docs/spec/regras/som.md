# Som e música: quando tocam

Formatos e caminhos de reprodução: [formatos/som.md](../formatos/som.md). Aqui: as funções que o resto do jogo chama, as opções do jogador, a preempção, e o catálogo de quem toca o quê.

## Funções de som (`GR_MODULE`)

| Função | Endereço | O que faz (modo multimídia, o caminho do Windows 3.1 com placa de som) |
|---|---|---|
| `_myBeginSong(id, prioridade)` | `1008:858e` | Sai se o som não foi instalado ou se `_songsOnFlag` = 0. Carrega o cabeçalho tipo 18 `id` (sai se não existe), procura `id` na tabela de nomes (sai se não está), fecha a música MCI aberta, se houver, e abre/toca `sound\<nome>.mid`. A prioridade (0x7e, 0x3f) não é usada. |
| `_StopSong()` | `1008:8294` | Fecha o dispositivo MCI (`MCI_CLOSE`). |
| `_mySongIsDone()` | `1008:a1a0` | No modo multimídia devolve **sempre verdadeiro**; só o modo Sound Blaster consulta o fim real. |
| `_myServiceSong()` | `1008:a216` | Com música MCI aberta, um `PeekMessage` para a notificação do MCI andar. |
| `_myBeginSound(id, taxa, prioridade)` | `1008:98b0` | Sai se o som não foi instalado ou se `_effectsOnFlag` = 0. Carrega o tipo 5 `id`, decodifica (formatos/som.md) e pede 4096 Hz a `waveOutOpen`/`waveOutWrite` (na Sound Blaster 16 sai a 5000 Hz, em blocos de 504 amostras: seção "Efeitos dentro do jogo, gravados numa sessão jogada pelo usuário"). O segundo argumento só é usado com `DSOUND.DLL` (é a palavra alta da taxa do Mac: 0x56ee = 22 254 Hz, 0x2b77 = 11 127 Hz). A prioridade não é usada. |
| `_myBeginSoundReverse(id, …)` | `1008:a18c` | Igual a `_myBeginSound` (o "reverso" não é feito no Windows). |
| `_mySoundIsDone()` | `1008:a188` | Sempre 1. |
| `_myBeginSoundList`, `_myBeginSoundSection` | `1008:8292`, `1008:a264` | Vazias. |
| `_SysBeep`, `_Beep` | `1008:828a`, `1008:a266` | `MessageBeep`. |

Fato visto no código para toda a tabela.

### Um efeito por vez; o novo interrompe o anterior

Há um único dispositivo `waveOut` aberto. Se, ao pedir um efeito, o anterior ainda está tocando, `_myBeginSound` compara `GetTickCount()` com um prazo gravado no início do efeito anterior (`GetTickCount()` + a posição devolvida por `waveOutGetPosition` em milissegundos, que é ~0 logo depois do `waveOutWrite`). Se o prazo ainda não passou, o pedido é **descartado**; senão, faz `waveOutReset` e toca o novo (fato visto no código de `_myBeginSound`; o prazo fica em `1048:1238`). Na prática o prazo é o próprio instante de início, então só um pedido no mesmo milissegundo é descartado: o efeito novo corta o anterior [código; **o corte foi medido** na sessão gravada (2026-10-19): o anterior para ~35 ms antes de o novo soar; o descarte no mesmo milissegundo não aparece na gravação]. A comparação é `≤` (o pedido no próprio milissegundo do início é descartado) e só vale com o dispositivo ainda em uso (`DGROUP:0af0/0af2`, o cabeçalho do efeito anterior ainda não liberado pelo `MM_WOM_DONE`) [código]. Em consequência, de dois efeitos pedidos seguidos no mesmo tick sem espera entre eles, **fica o primeiro**. **Port (desde 2026-10-19):** `ControleSom` guarda o instante de início do último efeito aceito (o imediato ou o agendado) e descarta o pedido que começaria no mesmo milissegundo; parar os efeitos (opção ou foco) zera essa memória (`simant/src/som/controle.rs`, teste `efeito_no_mesmo_milissegundo_e_descartado`). [hipótese: a resolução de `GetTickCount` no Windows 3.1 pode ser maior que 1 ms; o port usa o milissegundo do relógio dele]

Músicas também: abrir uma nova fecha a anterior. Música e efeito tocam juntos (dispositivos diferentes).

## Opções do jogador

- **Menu Options → Music** (comando 0x32, `_ProcMenu` `1000:4fc2`): alterna `_OptionStates[1]` (`[0x85f4]`); se desligou, `_StopSong()`.
- **Options → Sound Effects** (comando 0x33): alterna `_OptionStates[2]` (`[0x85f6]`).
- `_songsOnFlag` e `_effectsOnFlag` copiam essas opções; são as que `_myBeginSong`/`_myBeginSound` testam.
- **Persistência:** no começo do `WINMAIN` (`1000:3fda`), antes do `_NewGame(1)`, as seis opções vêm do `WIN.INI`, seção `[SimAnt]`, chaves `autotrack`, `music`, `effects`, `events`, `messages`, `silly` (`GetProfileInt`, padrão = valor atual da variável), e `_songsOnFlag`/`_effectsOnFlag` as copiam; em seguida, as funções dos botões configuráveis `mapuserbutton0`–`7` (faixa 34) e `yarduserbutton0`–`7` (faixa 35, `_SetUserButton`). Ao sair do laço de mensagens, o `WINMAIN` grava tudo de volta com `WriteProfileString` ("1"/"0" para as opções). `_NewGame` não escreve em `_OptionStates`: num jogo novo valem as opções do momento; um `.ANT` carregado traz as suas (bloco `_OptionStates` do save), que são as gravadas ao sair. Não vêm do `SIMANT.CFG` [código; chaves conferidas no `WIN.INI` da instalação do usuário]. **Port:** `simant/src/preferencias.rs`, arquivo `preferencias.json` na pasta de dados do usuário (`SDL_GetPrefPath`, no Windows `%APPDATA%\SimAnt recomp\SimAnt\`), lido ao abrir a partida com janela e gravado ao sair; opções e botões das faixas 34 (`botoes_faixa`) e 35 (`botoes_quintal`, desde 2026-10-18). Jogo novo (Ctrl+N, fim de jogo) e Load de `Tutorial.Ant` herdam as opções da partida aberta; Load de `.ANT` usa as do arquivo (`Sessao::herdar_opcoes`). **Origem inicial** (2026-10-19): enquanto `preferencias.json` não existe, o port lê a seção `[SimAnt]` do `WIN.INI` da instalação do usuário (`--win-ini ARQ`, ou `WINDOWS/WIN.INI` na pasta do jogo ou em uma das duas acima, sem diferença de maiúsculas; na instalação do usuário, `SimAnt-T/WINDOWS/WIN.INI` com as 6 opções e os 16 botões), com a conversão do `GetProfileInt` (dígitos do começo; chave ausente = valor do momento; seção e chaves sem diferença de maiúsculas). Só lê: ao sair grava o `preferencias.json`, que vale dali em diante. Uma opção diferente de 0 vira 1 (o original guarda o número lido). `--sem-preferencias` desliga os dois. Testes `win_ini_como_o_getprofileint` e `win_ini_achado_subindo_da_pasta_do_jogo`.
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
| 13 | `_SimCat` (`1030:2b24`): gato andando, quando `_MapMode` = 0, `_YardMode` < 2 e `_SRand64()` = 0 (0,49 s) |
| 14 | `_SimCat`: o gato começa a fugir do cachorro (`_CatOn` = 3), com `_MapMode` = 0 e `_YardMode` < 2, sem sorteio (1,27 s) |
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
| 36 | cortador de grama: `_DrawMower` (`1010:b2c4`), a cada redesenho do quintal (modos 0 e 1) com `_BoyHere` 3 ou 4 e o jogo não pausado |
| 37 | luta: `_AnimYellowFight` (`1028:7e56`), `_LionDialog` |
| 38 | formiga-leão: `_LionDialog`, `_DoAntLions` |
| 41 | água: `_DoWater` (`1020:07c6`); aranha: comando 12 (laser) |
| 42, 45 | `_YellowDialog`, `_SpiderDialog` |
| 44 | comer: `_EatMyFood`, `_MoveSpider` |
| 47 | `_MoveSpider` |
| 49 | `_YellowBirth`, `_AnimYellowFight` |
| 55 | laser: `_DoLaserFire` (`1000:6ec8`) |

Os ids 0, 3–6, 8, 11, 12, 16, 20, 25, 26, 31, 33, 34, 39, 40, 43, 46, 48, 50–54 e 56 não aparecem como constante nas chamadas vistas [em aberto: podem vir de variáveis, como em `_MysteryButton` e `_CheatKeys`].

## Port

**Regras → interface.** As regras não tocam som: pedem `Evento::Musica(id)` (id do cabeçalho tipo 18), `Evento::Som(id)` (id do tipo 5) e `Evento::TemaDoNinho` (`_TryAntTheme`) na fila do `Estado` (`engine/src/rules/estado.rs`, `Estado::pedir_som`). Os ids dos efeitos ficam na seção `sons` de cada `data/<parte>.json` (opcional: id ausente = sem som). Pedir um som **não sorteia nem muda o estado**: os sorteios que o original faz por causa de um som (latido `S(4)`, gato `S(64)`, pássaro `S2`, chuva `S(50)`, mastigação `S2`/`S256`, arroto `S2`, luta `S2`) já eram feitos e continuam no mesmo lugar. Conferido por `engine/tests/sons_regras.rs` (cada caso roda com e sem os ids e compara os estados, inclusive em 400 ticks de seis saves) e pelos 24 vetores de tick do original (307/307 blocos).

### O que está ligado

| Parte (`data/…json`) | Original | Pedido no port |
|---|---|---|
| quintal | `_SimDog`/`_MakeBark`: latido pelo sorteio `S(4)` (0x17, 0x15, 0x17, 0x16); latido 0x15 acompanhando o cortador | `cachorro` → `latir` |
| quintal | `_SimCat`: 13 com a vista no quintal e `S(64)` = 0; 14 ao começar a fugir do cachorro (desde 2026-10-18) | `gato`, `gato_fugindo` |
| quintal | `_DrawMower`: 0x24 a cada redesenho do quintal com o menino cortando ou guardando o cortador, fora da pausa (desde 2026-10-18; pedido por `rules::desenho::desenhar_cortador`, chamado pela partida depois do tick) | `cortador` |
| quintal | `_SimBird`: 7 quando o pássaro pega o enxame, vista no quintal e `S2` = 0 | `passaro` |
| quintal | `_FootFall`: 0x23 a cada pisada (fora e dentro de casa); `_SimKidInside`: 9 (inseticida) e 0x18 nas combinações (contagem do nó, nó, quadro) = (3, 20, 900), (1, 21, 903), (6, 20, 6), (3, 21, 1) | `menino_fora`, `menino_dentro` |
| quintal | `_DoWater`: 0x29 com chuva e `S(50)` = 0 | `simular_agua` |
| ninho | `_DoDigInB/R`: 0x11 / 0x12 ao pegar a terra cavada | `cavar_para_dentro` |
| superfície | `_AddFood(n, 1)`: 0x20 (comida periódica) | `RegrasSuperficie::adicionar_comida` |
| inimigos | `_DoAntLions`: 0x1E ao cuspir pedrinha/comida | `formiga_leao_esperando` |
| inimigos | `_MoveSpider`: 0x2F correndo atrás da presa; 0x2C mastigando; 10 arroto (opção Silly) | `aranha_perseguir`, `aranha_comer` |
| inimigos | `_DoLaserFire`: 0x37 a cada formiga atingida | `laser` |
| amarela | `_SetLife` cavando a terra: 0x13 | `por_vida` |
| amarela | `_PickupMyRock`/`_DropMyRock`: 0x1E; `_PickupMyFood` (para carregar)/`_DropMyFood`: 0x1D; pegar ovo/`_DropMyEgg`: 0x1C | `pegar_*`, `largar_*`, `tentar_largar_ou_pegar` |
| amarela | `_EatMyFood`: 0x2C (comida e ovo) e 10 (arroto do canibalismo) | `comer` |
| amarela | `_AnimYellowFight`: 0x25 por quadro de luta com `S2` ≠ 0; 0x31 por quadro de choque | `animar_luta` |
| amarela | `_DoAntMoveY`: 1 ao atravessar a passagem entre os ninhos e ao fracassar uma ordem; `_TryAntTheme` ao entrar/sair do ninho (também em `_EnterNest`/`_ExitNest`) | `passo_da_ordem`, `mover_amarela`, `entrar_no_ninho`, `sair_do_ninho` |
| amarela | `_YellowDeath` 3 e 4 (cercada, fugiu): 1 | `matar_amarela` |
| amarela | `_YellowBirth`: 0x1C ao pôr o ovo e 0x31 ao nascer; antes do ovo os efeitos ficam desligados (`_OptionStates[2]` = 0), então sons pedidos ali (cavar) são descartados | `nascer_de_ovo`, `fundar_colonia` |
| amarela | `_YellowCommand`: alarme 0xF (liga/desliga) ou 1; comandos 8 e 9 impossíveis: 1; aranha 11: 2; aranha 12: 0x29 ao ligar o laser, 2 ao desligar | `comando` |
| amarela | `_YellowCommandKey` Home: 0xF; `_ExchangeLives`: 0xF (ou a música da rainha), 1 se falha; `_processSpider` no modo alvo: 0xF ou 1 | `tecla_de_comando`, `trocar_de_corpo`, `clicar_no_mapa` |
| partida (`core/partida.rs`) | `_GotoMyAnt` no Experimental: 1 | `ir_para_amarela` |
| simant | `_TryAntTheme`: 10004, 10005, 10003 em ciclo, se passaram 396 s (7200 × 55 ms) desde o último; o relógio anda mesmo com a música desligada | `som::Audio`, `data/som.json` → `tema_ninho` |

### Interface: o que está ligado e o que falta

Conferido contra o código em 2026-10-18 (antes esta seção dizia que o Experimental não existia no port).

- **Ligados fora do tick** (os pedidos chegam a `simant/src/som/` como os das regras): ferramentas do Experimental, `_DoTool` e o "Start Game" do menu de ferramentas (`rules/experimental.rs`, `data/experimental.json` → `sons`, `musicas.comecar_jogo` = 11003; o botão de ferramentas no Quick Game pede 2, `para_experimental`); `_GotoMyAnt` no Experimental (`core/partida.rs`, som 1); marcador vazio de `_DoBookMark` (`partida.json` → `marcadores.som_vazio`); `_CheatKeys` (`data/codigos_trapaca.json` → `sons`); `_MysteryButton` (`rules/misterio.rs`); `_EndGameDialog` (10008/10010, `mensagens.json` → `fim_de_jogo`); `_XferPatch`/`_SpecialXfer`/`_PlaceQueenInYard` (1, 11002, 11003; `rules/troca_patch.rs` e `core/quintal_mapa.rs`); botão MATE de `_ProcYardEvent` (`quintal.json` → `botao_revoada`).
- **Ligados nos quadros da janela 26** (2026-10-18, `telas/mensagens.md` §10–11): `_YellowDialog` (42 e 45 da trofalaxia), `_SpiderDialog` (45 ao abrir e a cada volta dos quadros), `_LionDialog` (38 ao abrir, com a espera do som acabar, e 37 nos quadros ímpares) e `_DrawSimPayoff` (músicas 20003 e 20004); `core/quadros_animados.rs` e `core/avisos.rs` pedem os sons pela fila da partida, no instante do quadro (`mensagens.json` → `quadro_amarela`, `quadros_animados`).
- **Não ligados**, porque a tela ou o caminho não existe no port: `_StartAttack` pelo menu (11018); os menus de ferramentas da janela de edição e da janela do mapa (`_EditToolsMenu`, `_MapToolsMenu`: 2 e 11003, os mesmos do botão da faixa), porque esses botões não existem no port; os outros botões da faixa 35 (`_ProcYardEvent`).
- **Barra de menus** (2026-10-18, `telas/menus.md`): só File → New toca algo no original (`_ProcMenu` 3: `_myBeginSong(0x2711)` antes de `_NewGame(0)`); ligado em `simant/src/menus_ui.rs` (`barra_menus.json` → `musica_novo`), também pelo Ctrl+N. Abrir os menus, About, Status e o Quit não pedem som ao jogo (as caixas do Windows podem tocar o som do sistema, que o port não reproduz) [código: `_ProcMenu`, `_AboutDialog`, `_ScoreDialog`, `_MenuQuit` não chamam `_myBeginSound`].
- `_myBeginSoundSection` (0x26 da formiga-leão) é vazia no Windows: nada a pedir (o `S8()` continua).

**Executável.** `simant/src/som/` recebe os eventos e toca com SDL2:

- Música: procura o id em `data/som.json` → `musicas` (a tabela MCI acima) e, se `exigir_cabecalho`, exige o tipo 18 no banco, como o original. Lê `sound/<nome>.mid` da pasta do jogo (`Recursos::arquivo`, sem diferenciar maiúsculas; mod: `arquivos/sound/<nome>.mid`). Com `fonte_musica` = `"banco"`, usa o tipo 20 da palavra 0 do cabeçalho e a transposição da palavra 3 (o caminho Sound Blaster). Toca só `canais_midi` (1–10). Uma música nova corta a anterior.
- Efeito: tipo 5 decodificado, cortado nos blocos do driver (`bloco_efeitos` = 504: só blocos inteiros; menos de um bloco = mudo, mas ainda corta o anterior) e reamostrado de `taxa_efeitos` (5000 Hz, a taxa medida na SB16; o código pede 4096) para a saída, com interpolação linear (a do DOSBox-X, medida). Um efeito novo corta o anterior, exceto no mesmo milissegundo (descartado; seção acima).
- Opções: segue `_OptionStates[1]` (música) e `[2]` (efeitos) do estado da partida; desligar a música para a que toca.
- Foco (`WM_ACTIVATEAPP`), **em todas as telas** (abertura, "Select a Game" e partida): a cada quadro o executável lê se a janela tem o foco do teclado e não está minimizada (`janela_ativa` em `simant/src/jogo.rs`; antes só os eventos de foco da partida, que um laço modal podia engolir, e a abertura não tratava o foco). Ao ficar inativa: para a música, corta o efeito, esquece os agendados e não toca nada até voltar; ao voltar, nada recomeça. A música da abertura e a do jogo novo são pedidas ao criar a janela, antes de o SDL entregar o foco: ficam guardadas até o primeiro foco [escolha do port; no original, com o aplicativo inativo na abertura, `_songsOnFlag` = 0 e a música não tocaria]. O usuário observou no original a música da abertura parando sem foco. Lógica em `simant/src/som/controle.rs` (`ControleSom`), com testes.
- Abertura e jogo novo tocam `musica_abertura` e `musica_jogo_novo` (10001).

**Linha do tempo dos efeitos.** Dentro de um tick o original pode ficar parado em animações: `_AnimYellowFight` (`1028:7e56`) espera, no começo de cada quadro, `_MacTickCount() > início + 6` [código]; como o relógio do Mac anda de 3 em 3 a cada 55 ms (quintal.md, "Relógios"), são 3 degraus = **165 ms** entre dois quadros (o primeiro espera só o próximo degrau, até 55 ms; depois do último não há espera). O som do quadro (0x25 na luta com `S2` ≠ 0, 0x31 no choque) sai logo depois da espera. Na luta (6 quadros) os 0x25 ficam espalhados em ~825 ms (**medido em 2026-10-19** na sessão gravada: intervalos de 161–174 ms entre quadros, média 164,8 ms em 11 intervalos; seção "Efeitos dentro do jogo"), e o efeito pedido depois da animação (no mesmo tick ou no seguinte) só sai no fim dela — e, como depois do último quadro não há espera, um efeito pedido logo depois do último 0x25 cai no mesmo milissegundo e é descartado (desde 2026-10-19). O port antes recebia os pedidos do tick inteiro no mesmo quadro de vídeo: cada efeito cortava o anterior na hora, e a luta virava, no máximo, um 0x25 — muitas vezes cortado logo em seguida pela escavação (0x11/0x12) ou por outro efeito do mesmo tick; é por isso que o som da luta quase não se ouvia. Agora as regras pedem `Evento::Espera(ms)` entre os quadros (`amarela.json`, `animacao.espera_quadro_ms` = 165) e o `ControleSom` agenda cada efeito no instante certo: os seguintes à espera vão para uma fila, e a fila do quadro seguinte começa depois das esperas já agendadas. Desde 2026-10-18 a simulação do port também fica parada durante a animação e a janela de edição mostra os quadros (`Evento::Animacao`, [formiga-amarela.md](formiga-amarela.md) §10.2), como no original: nenhum som de outro tick pode cortar os da luta. Volume: o 0x25 tem 0,46 s e é dos efeitos mais baixos (RMS 0,19 contra 0,25–0,40 da maioria, medido nos WAV de `--exportar-som`); o port toca efeitos e música com `volume_efeitos` = `volume_musica` = 0,5 de `data/som.json`. A proporção entre o `waveOut` e o MIDI foi medida no DOSBox-X em 2026-10-19 (música 20 dB abaixo de um efeito de ganho 1, seção "Áudio do original no DOSBox-X"); o banco FM está calibrado para ela.
- `--sem-som` desliga tudo; `--exportar-som DIR` grava os efeitos e as músicas decodificados em WAV, sem abrir áudio.

**Sintetizador.** Ver `docs/decisoes.md` (2026-10-17 e 2026-10-19). `data/som.json` → `sintetizador` escolhe (linha de comando: `--sintetizador`): **`"fm"`, o padrão desde 2026-10-19** — sintetizador FM próprio no estilo do OPL2/OPL3 (`simant/src/som/fm.rs`) com um banco General MIDI próprio e livre (`data/opl_banco.json`, gerado por `tools/gerar_banco_opl.py`; ou um `.op2` do usuário em `fm.banco_op2`), o caminho mais próximo da Sound Blaster do original (comparação abaixo); `"soundfont"` — `rustysynth` com a SoundFont de `--soundfont`, `SIMANT_SOUNDFONT` ou `soundfont`; `"simples"` — ondas básicas por família GM e ruído na percussão. Sem a chave, vale o comportamento até a API 0.30 (SoundFont se houver, senão o simples). O timbre FM não é o do driver da Creative (banco próprio, chip não emulado ciclo a ciclo): `docs/divergencias.md`. `--exportar-som` grava as músicas com `volume_musica` (o nível do jogo) e `--exportar-musica ID` limita a exportação a uma música.

**Não reproduzido:** os modos Sound Blaster, alto-falante e Disney Sound Source como caminhos automáticos; `_mySongIsDone` falso no modo Sound Blaster (no port as músicas seguidas tocam como no MCI).

## Áudio do original no DOSBox-X (2026-10-19)

Ferramenta: `tools/som_dosbox.py` (`capturar`, `niveis`, `comparar`, `harmonicos`, `midi`, `quadros`). **Nada toca no PC:** o DOSBox-X roda com `[mixer] nosound = true` (o som continua emulado e a captura em WAV funciona; conferido: os WAV têm o áudio) e `SDL_AUDIODRIVER=dummy`, com a janela minimizada e sem ativação; nenhuma tecla ou clique do usuário é usado. Os arquivos do jogo são copiados para `re_trabalho/som/C` (pasta ignorada pelo controle de versão); `SimAnt-T` e `referencia/dosbox-x.conf` não são tocados. A captura é pelo comando `DX-CAPTURE /A` do DOSBox-X em volta do `win C:\MAXIS\ANTWIN\SIMANTW.EXE` (com `/V`, também vídeo ZMBV, de onde `quadros` extrai os quadros-chave em PNG para saber o que estava na tela).

**Instalação de referência (fato verificado nos arquivos de `SimAnt-T/WINDOWS`):** `SYSTEM.INI` → `[drivers]` `Midi=sb16fm.drv` ("Creative Stereo Music Synthesizer Driver" 2.00, OPL3), `Wave=sb16snd.drv`, `midimapper=midimap.drv`; `[sndblst.drv]` com `VoiceVolume` = `FMVolume` = 16,16. O `MIDIMAP.CFG` tem as configurações da Creative ("SB16 All/Basic/Extended MIDI", "SB16 Basic/Extended/All Music Synthesis"); a palavra no offset 6 do cabeçalho vale 0x0A, o número da configuração **"SB16 Ext Synth"** (Extended Music Synthesis) [inferido: o formato do `MIDIMAP.CFG` não é documentado; o número casa com o registro dessa configuração]. Nela os canais 1–10 vão para o "Creative Music Synthesizer" nos mesmos canais, **sem mapa de programas nem de teclas**, e os 11–16 ficam mudos: o MIDI Mapper entrega ao driver FM só o arranjo estendido dos `.MID`, com os programas GM sem troca — o que o port faz (`canais_midi` 1–10). O `SB16FM.DRV` tem só 4 KB; o banco de timbres FM da Creative não está nele [hipótese: fica no `VSBPD.386`; não procurado] e o port não o usa.

**Capturado (sem input):**

1. **Abertura (10001, `gamethme`)**, 130 s: o som começa 6,26 s depois do início do Windows e dura **102,90 s** (o resto é silêncio, ruído de fundo −74 dBFS). Saída **mono** (canais E e D idênticos, correlação 1,000). Os quadros do vídeo mostram o título até ~40 s (o tempo limite de 720 ticks) e depois o "Select a Game", parado; a música toca até o fim depois que o título fecha.
2. **Nível de um efeito no mesmo caminho:** o efeito 41 (água, 3,15 s) decodificado pelo port, reamostrado para 11 025 Hz e posto como som "Windows Start" no `WIN.INI` da cópia (`capturar --som-inicio`), sai pelo mesmo `sb16snd.drv` (`waveOut`) dos efeitos do jogo, 1,7 s antes da música. Na gravação ele aparece com **correlação 0,99989** com o PCM e **ganho 0,2504 (−12,0 dB)** em relação à escala cheia de 8 bits. A música, na mesma escala, tem **RMS −32,0 dBFS** (pico 0,15). Logo, no original, **a música fica 20,0 dB abaixo de um efeito tocado a ganho 1** (RMS da música contra o ganho do efeito, com os volumes do mixer da SB16 da instalação). Com o efeito a 4096 Hz (a taxa do jogo), o Windows 3.1 travou ao tocar o som de início ("Corrupt UMB chain" e laço de interrupção inválida no log) [observado, não investigado: o `waveOutOpen` do jogo pode seguir outro caminho]; a taxa de 11 025 Hz não muda o nível.

**Não capturado (nesta seção):** as outras músicas (capturadas depois, seção seguinte) e os efeitos dentro do jogo. O `AUTOTYPE` do DOSBox-X não entregou teclas com a janela minimizada e sem foco (testado no DOS, digitando um `mkdir`, e no Windows, com espaço no título e Enter no "Select a Game": nada mudou na tela nem no som), e operar a janela exigiria o foco do PC do usuário. No DOS a causa era o layout de teclado (seção seguinte); no Windows continua não chegando. O `-time-limit` também não encerra o DOSBox-X quando o convidado trava (foi preciso encerrar o processo).

**Comparação com o port** (`som_dosbox.py comparar`, música inteira, port exportado a volume 1; 10001 é também a música do jogo novo):

| Medida | Original (DOSBox-X) | Port, sintetizador simples | Port, FM (banco do port) |
|---|---|---|---|
| Duração do som | 102,90 s | 103,12 s | 103,18 s |
| Andamento (melhor escala do port, passo 0,25 %) | — | 0,9975 (croma 0,825; com 1,000: 0,819) | 0,9975 (0,857; com 1,000: 0,849) |
| Notas e ordem: croma quadro a quadro, alinhada (controle: port deslocado 0,9–4 s) | — | 0,819 (0,391) | 0,849 (0,431) |
| Centróide espectral médio | 1038 Hz | 1713 Hz | 854 Hz |
| Energia 0–200 / 200–500 / 500–1k / 1–2k / 2–4k / 4k+ Hz | 30 / 29 / 12 / 12 / 12 / 6 % | 34 / 17 / 10 / 13 / 7 / 18 % | 36 / 27 / 16 / 12 / 5 / 5 % |
| Espectro médio (60 Hz–8 kHz): correlação; diferença média | — | 0,590; 9,6 dB | 0,677; 8,2 dB |
| Música em relação a um efeito de ganho 1 (`volume_musica` = `volume_efeitos`) | −20,0 dB | −23,7 dB | −19,8 dB (calibrado) |

- **Duração e andamento:** batem. O som do original dura 102,90 s para 102,99 s de notas no MIDI (razão 0,999); a escala que melhor alinha o port (0,9975) melhora a croma em menos de 0,01, dentro da resolução do teste [verificado].
- **Notas e ordem:** a croma do port alinhada ao original dá 0,82–0,85, contra 0,39–0,43 com o port deslocado: as notas estão na mesma ordem e no mesmo tempo, com o original 6,20 s depois do início da gravação [verificado na abertura]. A percussão do canal 10 usa notas fora do mapa GM (85, 86, 89, 91, 95 nos `.MID`): nas janelas em que só a 89 soa, o original não mostra som próprio dela [observado em duas janelas; o banco do port as deixa em silêncio, hipótese para as demais].
- **Timbre:** no original o baixo (programa 34) tem o 2.º harmônico como o mais forte, o 1.º 10 dB e o 3.º 6 dB abaixo e os 4.º–12.º planos entre −17 e −24 dB (27 notas isoladas, `som_dosbox.py harmonicos`); o piano (programa 3) começa com os 5 primeiros harmônicos quase iguais e os agudos caem mais rápido que a fundamental; o tom 43 soa como um tom grave de ~69 Hz com a altura caindo; não há ruído na percussão (espectrograma). O sintetizador simples é claro demais (ondas serra/quadrada e percussão de ruído branco: 18 % da energia acima de 4 kHz, contra 6 %); o FM chega mais perto no espectro médio e na distribuição por faixa, mas ainda fica escuro entre 2 e 4 kHz (5 % contra 12 %). O banco FM foi ajustado nos programas de `gamethme` e, desde a captura das outras músicas, no 59 (seção seguinte) (baixo 34: perfil −2/0/−6/−11/−12/−13… no port, contra −10/0/−6/−22/−17/−17… no original). Os demais programas usados pelas músicas foram comparados em 2026-10-19 (seção seguinte).
- **Volume relativo:** com o simples, a música ficava 3,7 dB mais baixa em relação aos efeitos do que no original; o banco FM tinha `ganho` 2,25, calibrado para a abertura dar −19,8 dB (contra −20,0 dB medidos); desde o ajuste de 2026-10-19 (seção "Ajuste do banco FM"), 1,65 e −20,0 dB. As outras músicas ficam no nível relativo que o banco der [não medido].

## As outras músicas do original, sem input (2026-10-19)

**Rota:** `tools/som_dosbox.py musicas PASTA [NOME ...]`. Na **cópia** do jogo (`re_trabalho/som/C`), o `GAMETHME.MID` é trocado pela música pedida e o próprio SimAnt a toca ao abrir, pelo mesmo caminho de todas as músicas no Windows (MCI → MIDI Mapper "SB16 Ext Synth" → `sb16fm.drv`, OPL3 do DOSBox-X); o original é reposto no fim. Nenhuma tecla, nenhum clique, nada tocado no PC (mesmas condições da captura da abertura, acima). As 28 músicas com `.MID` foram gravadas (as 27 além da abertura, mais a abertura de novo como referência de nível); `jolly5` também, mas o original não a toca (20006 não tem cabeçalho, formatos/som.md) e o port também não. Comparação: `som_dosbox.py comparar_todas ORIGINAIS PORT/musicas saida.json` (port exportado com `--exportar-som`, FM).

| Id, música | Som (s): original / port | Croma alinhada (controle deslocado) | Envelope em dB (controle) | Centróide (Hz): original / port | Nível em relação à abertura (dB): original / port (diferença) |
|---|---|---|---|---|---|
| 10001 gamethme | 102.90 / 103.15 | 0.86 (0.43) | 0.64 (0.29) | 1038 / 854 | +0.0 / +0.0 (+0.0) |
| 10003 antthme1 | 55.37 / 55.70 | 0.51 (0.48) | 0.37 (-0.05) | 1053 / 2085 | -9.1 / -7.7 (+1.4) |
| 10004 antthme2 | 47.48 / 47.93 | 0.54 (0.48) | 0.75 (0.02) | 1094 / 4081 | -8.1 / -5.5 (+2.5) |
| 10005 antthme3 | 41.12 / 41.38 | 0.64 (0.56) | 0.58 (0.16) | 345 / 496 | -8.1 / -8.1 (+0.1) |
| 10006 hmanawar | 19.99 / 20.12 | 0.82 (0.32) | 0.81 (0.09) | 953 / 1100 | -0.3 / +3.4 (+3.7) |
| 10007 hmanthme | 20.07 / 20.19 | 0.82 (0.32) | 0.93 (0.18) | 913 / 1130 | -0.4 / +3.0 (+3.4) |
| 10008 victory | 30.04 / 30.13 | 0.87 (0.43) | 0.96 (0.01) | 458 / 769 | -3.0 / +0.2 (+3.3) |
| 10010 defeat | 34.40 / 34.58 | 0.90 (0.42) | 0.97 (0.10) | 622 / 564 | -0.8 / +2.7 (+3.5) |
| 11004 newcoln2 | 4.47 / 4.11 | 0.69 (0.21) | 0.96 (-0.14) | 767 / 1526 | -5.1 / -1.9 (+3.2) |
| 11005 newbody | 6.98 / 6.99 | 0.75 (0.22) | 0.90 (-0.08) | 731 / 911 | -2.8 / +3.1 (+5.9) |
| 11006 qn4day | 19.96 / 17.65 | 0.90 (0.56) | 0.42 (-0.00) | 742 / 1104 | +1.5 / +5.8 (+4.4) |
| 11007 badnews | 8.31 / 6.12 | 0.83 (0.53) | -0.02 (0.04) | 110 / 253 | +3.2 / +8.3 (+5.1) |
| 11014 call2arm | 6.80 / 6.11 | 0.85 (0.46) | 0.81 (0.01) | 364 / 585 | +3.1 / +7.7 (+4.6) |
| 11017 victqndd | 8.07 / 5.63 | 0.82 (0.60) | 0.07 (0.04) | 467 / 761 | +3.1 / +8.3 (+5.1) |
| 11018 attack | 7.46 / 5.86 | 0.76 (0.25) | 0.85 (0.03) | 622 / 339 | -7.7 / -6.3 (+1.4) |
| 11020 blackq | 4.84 / 4.91 | 0.67 (0.24) | 0.96 (-0.13) | 458 / 701 | -12.4 / -13.8 (-1.3) |
| 11021 redq | 7.47 / 8.10 | 0.94 (0.25) | 0.84 (-0.13) | 601 / 641 | +1.5 / +3.8 (+2.3) |
| 20000 happy1 | 5.47 / 4.63 | 0.69 (0.23) | 0.75 (-0.17) | 1591 / 1418 | -9.4 / +2.1 (+11.6) |
| 20001 happy2 | 5.28 / 4.49 | 0.65 (0.18) | 0.88 (-0.14) | 1093 / 784 | -6.5 / -6.4 (+0.1) |
| 20002 jolly1 | 7.79 / 7.83 | 0.71 (0.34) | 0.92 (-0.12) | 934 / 282 | -9.9 / -4.9 (+5.0) |
| 20003 jolly2 | 6.45 / 6.62 | 0.56 (0.21) | 0.92 (-0.12) | 1000 / 316 | -3.7 / +4.9 (+8.6) |
| 20004 jolly3 | 4.06 / 4.17 | 0.50 (0.31) | 0.90 (-0.19) | 1039 / 340 | -8.8 / -3.0 (+5.8) |
| 20005 jolly4 | 7.98 / 8.09 | 0.70 (0.35) | 0.81 (-0.08) | 946 / 346 | -11.8 / -5.3 (+6.4) |
| 20007 omnious1 | 7.87 / 7.21 | 0.74 (0.37) | 0.62 (-0.04) | 341 / 351 | -5.5 / -0.2 (+5.3) |
| 20008 sad1 | 14.85 / 13.58 | 0.84 (0.43) | 0.63 (0.13) | 861 / 1176 | -7.6 / -1.3 (+6.2) |
| 20009 sad2 | 1.21 / 7.49 | 0.96 (0.32) | 0.79 (-0.12) | 342 / 452 | -9.4 / -2.9 (+6.5) |
| 20010 sad3 | 8.64 / 8.70 | 0.96 (0.18) | 0.91 (-0.07) | 670 / 359 | -5.0 / -4.8 (+0.3) |
| 20011 sad4 | 10.18 / 9.04 | 0.89 (0.90) | -0.14 (0.23) | 795 / 552 | -6.2 / +1.4 (+7.6) |

Leitura (fato medido nas gravações; "controle" = o port deslocado 0,9–4 s):

- **Notas, ordem e andamento batem em todas.** A croma alinhada fica acima do controle em todas as músicas melódicas (0,50–0,96 contra 0,18–0,60), e o envelope de energia quadro a quadro dá 0,6–0,97 na maioria; a duração do som bate (diferenças até ~1 s vêm das caudas de liberação das notas). As exceções são de timbre, não de notas: `antthme1–3` são só percussão (canal 10; a croma não se aplica, o envelope dá 0,37–0,75 contra controle ≤ 0,16); `badnews` e `victqndd` (programa 95, Pad 8) e `sad4`/`qn4day` têm envelope baixo porque no original o pad e as cordas soam com outro envelope. **Revisto em 2026-10-19 (seção seguinte):** o envelope baixo vinha quase todo da componente contínua que o DOSBox-X grava (cauda subsônica depois das notas); tirada com um passa-altas de 20 Hz nas duas gravações, `badnews`/`victqndd` dão 0,95/0,93 com o banco antigo.
- **Programa 49 (String Ensemble 2):** no original um ataque forte e, meio segundo depois, a sustentação ~17 dB abaixo, com as notas seguintes (curtas, 0,23 s, mesma velocidade, sem controladores no `.MID`) baixas (`sad2`: som "útil" de 1,2 s contra 7,5 s no port). Um decaimento na portadora do banco do port não mudou a medida (as notas são curtas demais) e foi desfeito [em aberto: provavelmente o timbre e a alocação de vozes do driver da Creative]. **Explicado em 2026-10-19 (seção seguinte):** a nota forte não é do 49: no `.MID` a primeira nota vem antes da troca de programa e toca com o programa 0; o 49 do original tem ataque lento (~0,5 s), e as notas de 0,23 s não chegam ao nível cheio.
- **Programa 59 (Muted Trumpet):** ~10–12 dB alto demais no port em relação à abertura (`happy2`, só com ele: +10,2 dB). **Corrigido no banco** (`tools/gerar_banco_opl.py`, portadora 16 passos de TL mais baixa): `happy2` +0,1 dB.
- **Nível:** com o 59 corrigido, as outras músicas ficam em média ~4 dB acima do original em relação à abertura (mediana 4,1; de −1,3 a +8,6, fora `happy1`, +11,6, e `sad4`, +7,6, que misturam o 49). A abertura está calibrada (−19,8 dB contra um efeito de ganho 1); não foi mexido no ganho geral, porque a diferença é por programa [em aberto: um ganho por programa, medido música a música]. **Feito em 2026-10-19** (seção seguinte: `NIVEL_TL`).
- **Timbre:** o centróide do original fica entre 340 e 1600 Hz; no port, perto nas músicas de baixo e cordas e mais escuro nas `jolly*` (programas 19 e 69: 282–346 Hz contra ~1000 Hz) e mais claro nas `antthme*` (percussão). Os programas 19, 21, 25, 31, 33, 35, 49, 57, 69 e 95 continuam com o timbre do banco próprio, sem ajuste fino. **Revisto em 2026-10-19:** o "escuro" das `jolly*` era a componente contínua do 19 do port (aditivo com meias senoides, +5 dB de CC em `jolly2`); sem ela o port estava claro demais (2–4 kHz 28–37 % contra 8 %).

## Ajuste do banco FM contra as 28 músicas (2026-10-19)

Rota: as mesmas gravações (`re_trabalho/som/musicas/`, seção anterior), o port exportado com `--exportar-som` e `som_dosbox.py comparar_todas`, mais dois comandos novos: `perfil ORIGINAIS PORT PROGRAMA músicas` (perfil harmônico médio de um programa nas janelas em que nenhuma outra nota começa, pulando os parciais que coincidem com os de outra nota) e `percussao ORIGINAIS PORT músicas` (nível, frequência dominante e centróide de cada nota do canal 10 nos inícios isolados). O banco sai de `tools/gerar_banco_opl.py` (`medidos()`, `NIVEL_TL`, `percussao()`, `CURVA_VOLUME_DB`, `GANHO`, com o valor do original comentado em cada timbre).

**Mudança na medida (fato medido):** o DOSBox-X grava a componente contínua das ondas do OPL (as meias senoides e as ondas 2–7 têm média diferente de zero; uma placa real tem a saída acoplada por capacitor) e, depois das notas, uma cauda subsônica que decai ~8 dB/s (`badnews`: com passa-altas de 40 Hz a nota acaba em menos de 0,2 s; sem ele, "dura" mais 3 s com a frequência dominante em 0 Hz). O port também gera componente contínua (no 19 antigo, +5 dB no RMS de `jolly2`). Desde esta data `comparar` passa as duas gravações por um passa-altas de 20 Hz e grava também `energia_alinhada_db` (a energia nos mesmos quadros alinhados, que não depende de onde cada gravação acha o fim do som). Com isso, parte do que a seção anterior via mudou: o "envelope diferente" do 95 e de `sad4`/`qn4day` era a cauda subsônica (envelope 0,85–0,95 com o banco antigo) e as `jolly*` estavam claras demais, não escuras.

**Achados (fatos medidos nas gravações):**

1. **Curva de velocidade.** `sad2` toca 32 notas do 49, uma por vez, com a velocidade caindo de 126 a 20: no original o nível cai ~23–29 dB por década de velocidade (110 → 68: −4,8 dB; 110 → 52: −7,4 dB), contra 40 no banco anterior. `curva_volume_db` = 25.
2. **Programa antes da troca.** Em 13 dos 29 `.MID` a primeira nota de um canal vem antes do Program Change no mesmo instante (ordem dos eventos na trilha); essa nota toca com o programa que o canal tinha. Na gravação (Windows recém-iniciado) soa como piano (programa 0): é o "ataque forte" do 49 em `sad2` e `omnious1`. O port faz o mesmo (o canal começa no programa 0 a cada música). [hipótese] numa partida o driver da Creative guardaria o programa da música anterior; não há como gravar duas músicas seguidas sem input.
3. **Perfis harmônicos** (harmônicos 1–12, dB; "." = coberto por outra nota): 0 `0 -10 -21 -8 -9 -11 -16 -17 -25 -28 -43 -42` (o anterior morria em ~1,5 s; o original decai ~12 dB/s e depois ~5 dB/s); 19 `0 -10 -27 -3 -5 -40 -5 -17 -41 -18 -17 -49` (faltam os múltiplos de 3: FM com o modulador em 3×); 21 `0 -71 -9 -64 -9 -69 … -18` (só ímpares); 33 `0 -11 -7 -21 -25 -16 -14 -13 -14 -15 -17 -18`; 35 `-3 0 -5 -13 -22 -31 -37 -47 -51 -56 -57 -60`; 49 `0 -2 -5 -9 -15 -20 -22 . -34 -34 -40 -37`; 57 `. . 0 0 -10 -7 -10 . -17 -38 -27 -42`; 59 `0 -6 -1 -1 -2 -3 -5 0 -5 -7 -10 -15` (todos fortes; o anterior só tinha ímpares); 95 `0 -85 -64 -88 -69 -96 …` (quase um seno puro, contra −27…−101 no anterior). Com o banco novo, o 19 do port dá `0 -7 -59 -3 -5 -57 -6 -14 -58 -16 -18 -55`. Os programas 3, 25, 26, 31 e 69 não têm janelas isoladas suficientes: ajustados pelo espectro por oitava das músicas (`gamethme`: o piano 3 deixava a faixa de 2–4 kHz 5–6 dB abaixo nos trechos em que toca; sem KSR nos dois operadores, as notas agudas, 70–101, sustentam como no original).
4. **Envelopes:** 49 com ataque de ~0,5 s até o nível cheio (acorde final de `omnious1`; as notas de 0,23 s de `sad2` ficam ~13 dB abaixo do cheio); 95 com ataque imediato, sustentação plana e soltura curta (mais de 30 dB em 0,2 s).
5. **Nível por programa** (passos de TL de 0,75 dB, `NIVEL_TL`): 0 +6, 19 +8, 21 +4, 25 +4, 31 +6, 33 +4, 35 +3, 57 +6, 59 −14 (sobre os 16 passos de antes), 69 +6, 95 −2. O `ganho` geral foi recalibrado de 2,25 para 1,65: a abertura exportada (volume 0,5) dá −26,0 dBFS, isto é, **−20,0 dB** em relação a um efeito de ganho 1, como no original.
6. **Percussão** (canal 10, inícios isolados; nível em dB relativos ao RMS da abertura, frequência dominante dos primeiros 55 ms; tabela inteira com `som_dosbox.py percussao`):

| Nota | Original | Port antes | Port depois | O que mudou |
|---|---|---|---|---|
| 36 | −6,2 dB; 91 Hz; centróide 576 Hz | −6,1; 64 Hz; 65 Hz | −9,4; fundamental 92 Hz; 359 Hz | portadora na nota 42 (era ½ × nota 26, subsônica) |
| 39 | −3,9; 1950 Hz (tonal) | −2,8; ruído (centróide 9,2 kHz) | −5,8; 1977 Hz | tom na nota 95 |
| 40 | −6,8; ruído (9,7 kHz) | −1,8 | −9,0 | 8 passos mais baixo |
| 43 | −2,1; 68 Hz; 281 Hz | −6,8; 68 Hz; 137 Hz | −4,2; 68 Hz; 156 Hz | sem o modulador em ½; estalo de ruído no ataque |
| 47 | −2,6 (3 inícios); ~100 Hz | −8,7 | −7,2; 100 Hz | nota 43 |
| 48 | −2,5; 91 Hz; 452 Hz | −8,1; 245 Hz | −4,8; 95 Hz; 515 Hz | nota 42 |
| 49 | −12,4; ruído (7,9 kHz) | −4,9; 9,4 kHz | −15,1; 8,6 kHz | 13 passos, portadora em 1× |
| 50 | −4,4; 123 Hz; 624 Hz | −14,1; 145 Hz (bandas em 0,5× e 1,5×) | −4,6; 123 Hz | nota 47, modulador em 1× |
| 58 | −17,7; centróide 2,5 kHz | −7,3; 9,2 kHz | −19,2 | 19 passos |
| 59, 77, 80 | **sem som** (67, 18 e 9 inícios) | soavam (−6,5 a −9,2) | sem timbre | mudas |
| 60 | −4,5; ruído (8,7 kHz) | −5,2; tom de 586 Hz | −6,4; ruído | ruído |
| 67 | sem aumento em nenhuma faixa nos 12 inícios isolados de `gamethme` | soava | sem timbre | muda |
| 68 | −1,4; tom de 2081 Hz que dura a nota | −3,3; 786 Hz curto | −5,6; 2095 Hz sustentado | nota 96, sustentado |
| 70 | inconclusivo (`gamethme`, música densa) | já inaudível (nota 98) | sem timbre | muda |
| 76 | −7,4; 823 Hz | −3,3; 700 Hz | −8,1 | nota 80 |
| 85, 86, 89, 91, 95 | **sem som** (86, 89, 91: 5–26 inícios; 86 e 95 juntos em `hmanawar`, 4,38 s: silêncio; 85 só aparece com outras) | sem timbre | sem timbre | — |

Os níveis "depois" ficam 1–3 dB abaixo do original nesta tabela porque a abertura do port (a referência) ficou mais clara e mais cheia; as músicas só de percussão ficaram entre +0,4 e +1,4 dB (tabela seguinte). Notas não medidas (35, 37, 42, 46, 51, 65, 72, 75: até 2 inícios isolados) ficaram com o timbre do modelo, com o nível acertado pelo das vizinhas.

**Antes e depois** (`comparar_todas` com o passa-altas; nível = `energia_alinhada_db` da música menos o da abertura, port contra original; envelope = correlação da energia por quadro em dB; espectro = correlação do espectro médio 60 Hz–8 kHz; "antes" = o banco de 2026-10-19 com o 59 já corrigido, o da seção anterior):

| Música | Nível do original em relação à abertura (dB) | Port − original (dB): antes / depois | Envelope: antes / depois | Espectro: antes / depois | Centróide (Hz): original / antes / depois | 2–4 kHz (%): original / antes / depois |
|---|---|---|---|---|---|---|
| antthme1 | -9,2 | +1,4 / +0,7 | 0,40 / 0,65 | 0,59 / 0,73 | 1281 / 2095 / 1289 | 35 / 11 / 17 |
| antthme2 | -8,2 | +2,9 / +1,4 | 0,52 / 0,87 | 0,47 / 0,41 | 1766 / 4023 / 2434 | 31 / 14 / 30 |
| antthme3 | -8,1 | -0,2 / +0,4 | 0,62 / 0,90 | 0,72 / 0,70 | 348 / 494 / 502 | 1 / 2 / 2 |
| attack | -7,6 | +0,3 / -0,5 | 0,72 / 0,95 | 0,90 / 0,96 | 623 / 309 / 588 | 1 / 0 / 1 |
| badnews | +3,9 | +4,9 / +0,5 | 0,95 / 0,94 | 0,82 / 0,89 | 162 / 259 / 120 | 0 / 0 / 0 |
| blackq | -12,4 | -1,4 / +0,3 | 0,96 / 0,96 | 0,64 / 0,88 | 464 / 703 / 416 | 0 / 6 / 0 |
| call2arm | +4,1 | +4,1 / -0,7 | 0,90 / 0,97 | 0,40 / 0,91 | 379 / 604 / 367 | 0 / 2 / 0 |
| defeat | -0,8 | +1,9 / -1,2 | 0,96 / 0,97 | 0,61 / 0,77 | 623 / 895 / 794 | 5 / 6 / 6 |
| gamethme | +0,0 | +0,0 / +0,0 | 0,83 / 0,85 | 0,68 / 0,63 | 1056 / 852 / 1296 | 12 / 5 / 7 |
| happy1 | -9,0 | +11,4 / +1,8 | 0,92 / 0,98 | 0,64 / 0,70 | 1996 / 1433 / 1694 | 21 / 25 / 24 |
| happy2 | -5,9 | -0,8 / -1,0 | 0,97 / 0,98 | 0,69 / 0,75 | 1153 / 778 / 1171 | 13 / 8 / 12 |
| hmanawar | -0,2 | +3,6 / -0,5 | 0,96 / 0,95 | 0,66 / 0,69 | 956 / 1112 / 1126 | 6 / 4 / 11 |
| hmanthme | -0,3 | +3,4 / -0,1 | 0,97 / 0,95 | 0,56 / 0,69 | 916 / 1143 / 1005 | 8 / 4 / 9 |
| jolly1 | -9,8 | -1,1 / -0,2 | 0,95 / 0,96 | 0,58 / 0,74 | 959 / 1543 / 1121 | 8 / 37 / 13 |
| jolly2 | -3,6 | +3,5 / +0,4 | 0,97 / 0,96 | 0,60 / 0,73 | 1012 / 1501 / 1194 | 8 / 28 / 12 |
| jolly3 | -8,8 | -0,2 / +0,1 | 0,23 / 0,98 | 0,59 / 0,70 | 1073 / 1586 / 1273 | 9 / 31 / 13 |
| jolly4 | -11,7 | +3,9 / +2,2 | 0,93 / 0,94 | 0,63 / 0,70 | 960 / 1389 / 1028 | 6 / 6 / 7 |
| newbody | -2,7 | +5,8 / -1,2 | 0,92 / 0,93 | 0,55 / 0,88 | 732 / 915 / 915 | 3 / 6 / 6 |
| newcoln2 | -4,7 | +2,9 / -1,0 | 0,98 / 0,99 | 0,75 / 0,76 | 787 / 1527 / 1189 | 4 / 13 / 3 |
| omnious1 | -5,1 | +5,0 / -1,5 | 0,88 / 0,92 | 0,88 / 0,90 | 359 / 351 / 385 | 0 / 0 / 0 |
| qn4day | +1,7 | +4,1 / -0,4 | 0,85 / 0,97 | 0,57 / 0,76 | 922 / 1173 / 770 | 4 / 4 / 0 |
| redq | +1,6 | +2,6 / -1,4 | 0,88 / 0,98 | 0,71 / 0,96 | 601 / 641 / 612 | 0 / 0 / 0 |
| sad1 | -7,4 | +6,1 / -1,8 | 0,93 / 0,97 | 0,62 / 0,72 | 950 / 1189 / 1146 | 5 / 15 / 13 |
| sad2 | -9,3 | +15,0 / +1,5 | 0,95 / 0,97 | 0,89 / 0,92 | 425 / 456 / 339 | 0 / 0 / 0 |
| sad3 | -5,0 | +0,2 / -1,0 | 0,87 / 0,94 | 0,89 / 0,94 | 670 / 359 / 737 | 2 / 0 / 2 |
| sad4 | -5,8 | +7,5 / +0,7 | 0,88 / 0,95 | 0,68 / 0,73 | 832 / 603 / 892 | 8 / 1 / 5 |
| victory | -3,0 | +3,2 / -0,6 | 0,93 / 0,90 | 0,86 / 0,87 | 471 / 779 / 556 | 1 / 1 / 0 |
| victqndd | +4,3 | +4,3 / -0,1 | 0,93 / 0,98 | 0,51 / 0,82 | 606 / 807 / 456 | 1 / 2 / 0 |
| **Média** | | **+3,4 / −0,1** (média do módulo: 3,6 / 0,8) | 0,85 / 0,94 | 0,67 / 0,78 | módulo de log₂(port/original): 0,53 / 0,24 | 6,8 / 8,2 / 6,8 |

- **Nível:** de +3,4 dB em média (o "~4 dB acima" da seção anterior, que sem o passa-altas dava +4,0) para −0,1 dB; nenhuma música passa de ±2,2 dB (`jolly4` +2,2, `sad1` −1,8). A abertura continua −20,0 dB abaixo de um efeito de ganho 1.
- **Timbre:** o espectro médio sobe de 0,67 para 0,78 de correlação e a diferença média cai de 14,3 para 10,2 dB; o centróide fica a 18 % do original em média (era 44 %). A faixa de 2–4 kHz fica igual em média (6,8 %), mas por música ainda há desvios: a abertura tem 7 % contra 12 % e o centróide dela passou do original (1296 contra 1056 Hz, pelos 8 kHz da percussão); `antthme1` 17 % contra 35 %; `sad1`, `hmanawar` e as `jolly*` 4–8 pontos acima.
- **Envelope:** de 0,85 para 0,94 de média (`jolly3` 0,23 → 0,98, `attack` 0,72 → 0,95, `antthme2` 0,52 → 0,87).
- **Croma alinhada** (notas): 0,78 → 0,83 de média.

**Continua diferente [em aberto]:** o banco da Creative não foi lido (a imitação é por medida, com timbres próprios de 2 operadores); `antthme1` (35 % da energia em 2–4 kHz no original contra 17 %) e `antthme2` (espectro 0,41): a percussão do original tem componentes que o modelo de 2 operadores por nota não reproduz bem (o 47 e o 60 são tom + ruído); o 26 (guitarra de `gamethme`) só tem 2 janelas isoladas e ficou como estava; as notas da tabela de percussão com menos de 3 inícios isolados.

**Efeitos dentro do jogo: capturados depois numa sessão jogada pelo usuário** (seção seguinte). Antes, tentado (2026-10-19): o `AUTOTYPE` do DOSBox-X **funciona com a janela minimizada e sem foco** quando o layout de teclado do DOS é US (`[dos] keyboardlayout = us`, agora no `som_dosbox.py`): com o layout automático desta máquina (BR) não entregava nada, que era a falha registrada antes. No DOS (`mkdir` digitado) passa; **no Windows 3.1 em modo 386 avançado nenhuma tecla chega**: Espaço no título aos 25–27 s não o fecha (continua aos 38 s, quadros do vídeo), Enter e Alt no "Select a Game" não fazem nada [hipótese: o `AUTOTYPE` põe as teclas no buffer do BIOS, que o Windows não lê]. A bancada QEMU (alternativa a) segue sem servir para isso: o `sb16` do QEMU toca os efeitos com a taxa e o buffer errados e não há OPL3 (seção abaixo); a instalação do usuário não tem driver para AC97/ES1370. Rotas que sobram: entrada pela porta serial (mouse serial emulado via `nullmodem`) no DOSBox-X, ou um driver de teclado no Windows da cópia [não tentadas].

## Efeitos dentro do jogo, gravados numa sessão jogada pelo usuário (2026-10-19)

**Gravação:** o usuário jogou o original no DOSBox-X (`tools/som_jogar.py`: a mesma cópia `re_trabalho/som/C`, SB16 + OPL3, Windows 3.1, efeitos por `waveOut` no `sb16snd.drv`) por 9 min 35 s, com o áudio inteiro gravado (`re_trabalho/som/sessao/efeitos_original.wav`, 44 100 Hz, 16 bits, estéreo com os dois canais idênticos; fora do git). Roteiro sem tempos marcados: andar e cavar no ninho, pegar/largar comida e pedrinha, comer, recrutar/liberar, alarme, trocar de corpo, Home, lutas, ordem impossível e marcador vazio, Yard View, aranha e formiga-leão, Mystery, Experimental.

**Análise:** `tools/som_efeitos.py analisar SESSAO.wav EFEITOS_DIR` (os 57 efeitos tipo 5 como o port os decodifica, `--exportar-som`). Filtro casado do começo de cada efeito (120 ms) contra a gravação decimada a 11 025 Hz, com o ganho fixo da sessão (o que tira os falsos positivos da música); depois, na gravação a 44,1 kHz, alinhamento fino (amostra e fase), e a duração tocada por blocos de 2 ms (a gravação segue o efeito, fica muda, ou mostra outra coisa). `som_efeitos.py taxa` mede a taxa (3500–12 000 Hz) e `som_efeitos.py musicas` acha as músicas gravadas sem input (`som_dosbox.py musicas`) na sessão. Nada toca no PC. Teste controlado: `som_dosbox.py capturar --som-inicio` com um efeito do port como som de início do Windows (o mesmo driver), mudo, sem input.

### Separação da trilha

- **Músicas** (MCI → OPL3), achadas por correlação com as gravações sem input: `gamethme` a 6,2 s (a abertura, amostra por amostra igual à gravação de referência até ~24 s, ganho 1,000; depois o andamento do sequenciador escorrega uns ms com o jogo rodando), `call2arm` a 100,8 e 108,2 s (recrutar), `victqndd` a 114,2 e 120,0 s (liberar), `sad3` a 212,4 s (morte da amarela), `newbody` a 224,1 e 407,8 s (renascer), `antthme2` a 393,4 s e `antthme1` a 537,8 s (temas do ninho), `gamethme` de novo a 430,1 e 448,0 s [hipótese: jogo novo]. Nenhuma música fora das que o port pede nessas situações.
- **Efeitos** (`waveOut`): **156 ocorrências de 14 efeitos**, todas com o mesmo ganho (−12,2 dB, ver abaixo). Fora deles, nenhum som com começo abrupto que não seja nota da música (a música subtraída da gravação de referência entre 7 e 24 s não deixa nada além dos efeitos achados).

### Taxa, interpolação e nível (fatos medidos)

- **Taxa: 5000 Hz** (5000 ± 0,5 Hz: o efeito 0x20 inteiro, 0,6 s, dá NCC 0,975 a 5000 Hz e cai a 0,95 a ±2 Hz; o 0x26, varrido de 3500 a 12 000 Hz em passos de 100, dá 0,998 a 5000 e 0,19 na segunda melhor). O jogo pede 4096 Hz (formatos/som.md), a SB16 não desce de 5000 Hz e o `sb16snd.drv` toca os bytes a 5000 Hz sem reamostrar: **os efeitos soam 22 % mais rápidos e agudos que a 4096 Hz** (e 10 % mais lentos que no Mac, ~5563 Hz). Confirmado no teste controlado: um WAV de 4096 Hz pelo mesmo driver também sai a 5000 Hz.
- **Interpolação:** a gravação segue o PCM decodificado pelo port **amostra por amostra** (resíduo −45 dB em trechos sem música) com interpolação **linear** de 5000 para 44 100 Hz (NCC 0,988 contra 0,968 do degrau), que é o reamostrador do DOSBox-X, não o filtro de uma placa real [hipótese: numa SB16 real o filtro de saída arredonda mais]. O port já reamostra com interpolação linear.
- **Ganho:** −12,2 dB (0,245) em relação à escala cheia de 8 bits, igual para todos os efeitos (−12,1 a −12,9 dB nos inteiros), o mesmo que a medida anterior com o efeito 41 (−12,0 dB). **Nível em relação à música:** os efeitos têm RMS de −21 a −30 dBFS e a música, nos mesmos trechos, −25 a −40 dBFS (mediana por efeito: 4 a 16 dB acima da música). Como o ganho dos efeitos e o nível da música (a abertura com ganho 1,000 em relação à referência) são os já medidos, continua valendo "música 20 dB abaixo de um efeito de ganho 1" (seção "Áudio do original no DOSBox-X"); nada a mudar nos volumes.

### O driver toca os efeitos em blocos de 504 amostras (fato medido)

1. **Efeito pedido com o `waveOut` livre:** soam só **blocos inteiros de 504 amostras**; o resto do último bloco se perde. Medido em todos os efeitos isolados: 0x2C 1512 de 1824 amostras (10 vezes), 0x25 1512 de 1888 (10), 1 504 de 752 (4), 0x18 1008 de 1480 (7), 0x26 2520 de 2880, 0x20 3024 de 3374 (3), 9 504 de 800, 10 2520 de 2706; o gato 0xD (2016 = 4 × 504) toca inteiro (4 vezes).
2. **Efeito com menos de 504 amostras não soa.** Nenhuma ocorrência de 15 (102 amostras, Home/alarme), 0x11 e 0x12 (358 e 406, escavação das pretas e vermelhas), 0x13 (148, a amarela cavando), 0x1C (436, ovo), 0x1D (172, comida) e 0x28 (184), embora o roteiro tenha andado, cavado, pegado comida e apertado Home; todos os efeitos com 752 amostras ou mais que o roteiro pedia apareceram. **Teste controlado** (som de início do Windows a 4096 Hz pelo mesmo driver, sem input): 15, 0x1C e 0x23 (484) não soam; 0x2C sai com 1512 de 1824 amostras, com a mesma falha no meio; a 5000 Hz o 0x2C sai inteiro. Logo, o corte e o silêncio são do driver (ou da emulação dele no DOSBox-X) com buffers de 4096 Hz, não do jogo [hipótese: numa SB16 real com o mesmo driver seria igual; não há como conferir aqui]. No código, `_myBeginSound` não tem tamanho mínimo; o cabeçalho de um efeito que não tocou não é liberado (sem `MM_WOM_DONE`), então o pedido seguinte faz `waveOutReset` e toca normalmente.
3. **Falha curta:** em vários efeitos há ~10 ms no meio em que a gravação não segue o PCM (0x2C perto de 186 ms, 0x25 perto de 192 e 212 ms), igual no teste controlado [hipótese: troca de bloco de DMA do driver]. O port não reproduz.
4. **Preempção:** um efeito novo corta o que está tocando (medido: 0x25 da luta, 0x24 do cortador, 0x2C, 10); o anterior para **~35 ms antes** de o novo soar (na luta, cada 0x25 toca 126 ms dos 162 ms até o seguinte). A duração do efeito que corta outro nem sempre segue a regra dos blocos: o último 0x25 de cada luta toca 1512 amostras (a regra), mas alguns 0x24 do cortador param no fim do bloco do efeito anterior (363,18 s: 539 amostras, terminando em 3 × 504 a contar do 0x24 de 195 ms antes), e o único 0x23 ouvido (484 amostras, menor que um bloco) tocou inteiro cortando um 0x24 [em aberto: a regra exata do driver com o DMA já rodando]. O efeito 0x29 (chuva, 2,6 s) parou 4 vezes entre 126 e 475 ms, duas sem nenhum efeito audível depois [hipótese: cortado por um pedido de efeito curto, que não soa mas faz o `waveOutReset`].
5. **Descarte no mesmo milissegundo:** não aparece (não há como ver na gravação um pedido que não soou).

### Tabela por efeito

| Efeito (quem pede) | Vezes | Amostras tocadas / do efeito | Como acabou | Nível RMS (dBFS) | Efeito − música (dB, mediana) |
|---|---|---|---|---|---|
| 1 (erro: ordem impossível, marcador vazio) | 4 | 504 / 752 | bloco de 504 | −22,0 | +10,0 |
| 9 (inseticida do Experimental) | 7 | 504 / 800 | bloco de 504 (4); outros sob a música | −25,9 | +5,5 |
| 10 (arroto: aranha ou trapaça; 13 em 2,5 s, 471–474 s) | 13 | 2520 / 2706 (o último) | cortados uns pelos outros (137–327 ms) | −21,1 | +6,7 |
| 0xD (gato) | 5 | 2016 / 2016 | inteiro (4) | −26,9 | +7,4 |
| 0x18 (menino dentro de casa) | 9 | 1008 / 1480 | bloco de 504 (7) | −24,8 | +7,7 |
| 0x1E (pedrinha) | 2 | 490 e 350 / 962 (504 esperado; o final fraco fica sob a música) | bloco de 504 [provável] | −22,9 | +7,9 |
| 0x20 (comida: Mystery/Experimental) | 4 | 3024 / 3374 | bloco de 504 (3) | −24,3 | +15,8 |
| 0x23 (passo do menino) | 1 | 484 / 484 | inteiro (cortando um 0x24) | −23,2 | +10,2 |
| 0x24 (cortador de grama) | 39 | 1512 / 1864 quando livre | cortado pelo seguinte (64–390 ms) ou bloco | −29,8 | +4,4 |
| 0x25 (luta; quadros do leão) | 20 | 1512 / 1888 | cortado pelo seguinte (9); bloco (10) | −27,1 | sem música nas lutas |
| 0x26 (formiga-leão) | 1 | 2520 / 2880 | bloco de 504 | −22,5 | sem música |
| 0x29 (chuva) | 4 | 630–2380 / 12 896 | parou (2), cortado por 0x2C (2) | −27,2 | — |
| 0x2C (comer; aranha mastigando) | 25 | 1512 / 1824 | bloco (10), cortado (8), parou (7) | −26,9 | +5,8 |
| 0x31 (choque da luta / nascer) | 2 | sob a música | — | −23,5 | +13,0 |

**Não ouvidos:** 15, 0x11, 0x12, 0x13, 0x1C, 0x1D, 0x28 e 0x2F (menores que um bloco: o driver não os toca); 2, 7, 0x15–0x17 (latidos), 0x1B, 0x2A, 0x2D, 0x2E, 0x37 (o roteiro não os pediu ou o sorteio não os trouxe: nenhum latido nem pássaro no Yard View gravado; nenhum diálogo da aranha).

### Linha do tempo (fatos medidos)

- **Lutas:** 199,95 s: 0x25 em 0, 332, 495, 656 e 818 ms (o 2.º quadro sem som: `S2` = 0); 206,64 s: 0, 174, 336, 660, 822 ms; 217,66 s: 0, 174, 813 ms. Intervalo entre quadros: 161–174 ms, **média 164,8 ms** em 11 intervalos (o port usa 165 ms; o primeiro intervalo às vezes 174 ms, o degrau de 55 ms do relógio). Cada 0x25 é cortado pelo seguinte; o último toca 1512 amostras. O que vem depois da luta (0x2C, 0,4–1,4 s depois) não cai no mesmo milissegundo.
- **Formiga-leão** (403,48 s): 0x26 e, **562 ms depois**, o primeiro 0x25; depois 0x25 a cada 1,08–1,18 s (dois quadros de 30 do Mac) até 5,03 s, 5 vezes. O primeiro 0x25 vem no intervalo normal do laço (550 ms), não depois do som (o 0x26 dura 576 ms a 5000 Hz e toca 504): `_mySoundIsDone` volta verdadeiro na hora no modo multimídia. O port esperava 703 ms (o som a 4096 Hz) antes do laço, e o primeiro 0x25 saía ~1,27 s depois do 0x26.
- **Cortador** (354,9–363,2 s): 39 × 0x24, um a cada redesenho do quintal (intervalos de 64 a 390 ms, a maioria 160–250), cada um cortando o anterior, misturados com 0x2C e um 0x23.
- **Yard View** (335–364 s): 0x29 (chuva), 0x2C, 0x18 (menino em casa) e 0xD (gato) intercalados, a cada 0,2–0,8 s.

### Comparação com o port e correções

| Situação do roteiro | Port pede | Original na sessão |
|---|---|---|
| Andar e cavar no ninho | 0x13, 0x11/0x12, 0x1E | só 0x1E (2 vezes); 0x11/0x12/0x13 nunca (menores que um bloco) |
| Pegar/largar comida, ovo | 0x1D, 0x1C | nunca (menores que um bloco) |
| Comer | 0x2C | 25 vezes |
| Recrutar/liberar | músicas 11013/11014, 11016/11017 | `call2arm` e `victqndd`, 2 vezes cada |
| Alarme, Home, troca de corpo | 0xF (1 se falha) | 0xF nunca (102 amostras); 1 nas falhas |
| Lutas, morte, renascer | 0x25 por quadro com `S2` ≠ 0, 0x31; músicas `sad*`, `newbody` | igual |
| Yard View | 0xD/0xE, 7, 0x15–0x17, 0x18, 9, 0x23, 0x24, 0x29 | 0xD, 0x18, 0x23 (1), 0x24, 0x29 |
| Aranha, formiga-leão | 0x2F, 0x2C, 10; 0x26 + 0x25 | 0x2C, 10; 0x26 + 0x25 (0x2F nunca: 466 amostras) |
| Mystery, Experimental | 0x20, 9 e outros | 0x20, 9 |

Nenhum efeito que o original tocou ficou sem o pedido correspondente no port. **Corrigido no port (2026-10-19, API 0.42):** `data/som.json` → `taxa_efeitos` 4096 → **5000** e a chave nova `bloco_efeitos` = **504** (`simant/src/som/mixer.rs::blocos_do_driver`: só blocos inteiros; um efeito menor que um bloco fica mudo, mas ainda corta o que estava tocando, como o `waveOutReset` do original); `--exportar-som` grava os efeitos a 5000 Hz e, com a chave, também `efeitos_tocados/` (o trecho que o driver toca); `mensagens.json` → `quadros_animados.formiga_leao.espera_som_ms` 703 → **0**. Testes: `driver_toca_so_blocos_inteiros`, `efeito_mudo_ainda_corta_o_anterior`, `efeitos_soam_como_na_sound_blaster_16` (com os arquivos do usuário: 1512, 504, 2520, 3024, 2016 amostras e os curtos mudos) e `formiga_leao_toca_o_som_e_troca_de_quadro_a_cada_30_do_mac`. Para tocar os efeitos como o código pede, sem o driver: `taxa_efeitos` 4096 e `bloco_efeitos` 0.

**Não mudou:** volumes (já batiam), linha do tempo da luta (165 ms), descarte no mesmo milissegundo (não observável). **Não reproduzido:** a falha de ~10 ms no meio de alguns efeitos; a duração exata de um efeito que corta outro (o port aplica a regra dos blocos também nesse caso) e o 0x23 curto que soou ao cortar um 0x24 [em aberto].

## Áudio do original na bancada QEMU (2026-10-18)

Tentativa de gravar o som do original para comparar com `--exportar-som`. Ferramenta: `tools/som_original.py` (`boot`, `latido`, `chuva`, `luta`, `analisar`, `procurar`). **Resultado: a bancada grava áudio, mas o que sai não serve de referência; a comparação de ouvido continua aberta.**

**Montagem (funciona).** A cópia do WfW 3.11 da bancada já tem os drivers da Sound Blaster 16 (`SYSTEM.INI`: `Wave=sb16snd.drv`, `Midi=sb16fm.drv`, `Midi1=sb16snd.drv`, `device=vsbpd.386`; `[sndblst.drv]` Port=220, Int=7, DmaChannel=1, HDmaChannel=5). O QEMU de `referencia/qemu` tem `sb16`, `adlib` (OPL2) e o backend `wav`: `-audiodev wav,id=som,path=X.wav -device sb16,audiodev=som,irq=7 -parallel none` (a porta paralela ocupa a IRQ 7), mais `-device adlib,audiodev=som` para o FM (`SIMANT_SOM_OPL=1`). A máquina muda, então é preciso boot completo (os snapshots da bancada não carregam) e um snapshot próprio (`som`). No Quick Game assim montado, `_songsOnFlag` = `_effectsOnFlag` = 1 e o cachorro posto sentado (`_DogDir` = 4) latiu ~6 vezes em 40 s (`_DogBarkDelay` renovado: 6270, 6416, 6726, 6861, 7308, 7675).

**Por que não serve (observado):**

1. **Música (FM).** O `sb16fm.drv` é driver de OPL3; o `adlib` do QEMU é OPL2 e trata 0x38A/0x38B como 0x388/0x389, de modo que as escritas do 2º banco caem no 1º. Com o `adlib`, o WAV tem som contínuo (RMS ≈ 0,19) desde a carga do Windows, antes do jogo, e durante todo o Quick Game, sem relação com `gamethme`: notas presas. Sem o `adlib`, nenhuma música toca (o WAV do boot fica vazio). Não há OPL3 no QEMU.
2. **Tempo.** O backend `wav` só grava enquanto alguma voz está ativa: sem o `adlib`, os silêncios entre efeitos somem (3 s de áudio para ~45 s de jogo com 3 Home e ~6 latidos). Ordem e duração no tempo não são recuperáveis; com o `adlib` ativo o tempo é contínuo, mas sob carga do host o WAV sai mais curto que o tempo da VM (52 s gravados num boot de ~150 s).
3. **Conteúdo dos efeitos.** Os efeitos do port (tipo 5 decodificado, `--exportar-som`) **não aparecem** na gravação: a correlação normalizada (`som_original.py procurar`, reamostrando a 4096, 5000, 5512 e 6000 Hz e, para o efeito 15, de 3500 a 12000 Hz) fica abaixo de 0,5 para os latidos 21–23 (efeitos de poucas dezenas de ms dão picos de até 0,7 por acaso). Cada Home produz um trecho de 0,8–1,1 s, quando o efeito 15 tem 102 amostras (25 ms a 4096 Hz); as quebras da onda (interpolação linear do QEMU) vêm a cada 7–8 amostras de 44 100 Hz, isto é, uma fonte de ~5,5–6,3 kHz e não 4096 Hz. [hipótese] o `sb16snd.drv` sobe a taxa para o mínimo da placa (5000 Hz) e o `sb16` do QEMU toca o buffer de DMA inteiro (dados velhos) em vez do bloco pedido; não isolado. A chuva (efeito 41, 3,1 s; `_GlobalKey` = 0xA8, `RainOn` = 1, `WaterTable` 62 → 48 em 25 s) não gerou WAV nenhum.

**Continuava sem verificação:** timbre, volume relativo efeito/música, os sons da luta no tempo e a preempção no mesmo milissegundo. A captura pelo DOSBox-X (seção acima, 2026-10-19) resolveu, para a abertura, andamento, notas, volume relativo e uma referência de timbre; os efeitos dentro do jogo (taxa real na SB16, luta, preempção) foram medidos na sessão jogada pelo usuário (seção "Efeitos dentro do jogo, gravados numa sessão jogada pelo usuário"). A "fonte de ~5,5–6,3 kHz" vista aqui era a SB16 tocando a 5000 Hz.
