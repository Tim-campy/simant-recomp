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

Há um único dispositivo `waveOut` aberto. Se, ao pedir um efeito, o anterior ainda está tocando, `_myBeginSound` compara `GetTickCount()` com um prazo gravado no início do efeito anterior (`GetTickCount()` + a posição devolvida por `waveOutGetPosition` em milissegundos, que é ~0 logo depois do `waveOutWrite`). Se o prazo ainda não passou, o pedido é **descartado**; senão, faz `waveOutReset` e toca o novo (fato visto no código de `_myBeginSound`; o prazo fica em `1048:1238`). Na prática o prazo é o próprio instante de início, então só um pedido no mesmo milissegundo é descartado: o efeito novo corta o anterior [hipótese apoiada pelo código; não medido no original]. A comparação é `≤` (o pedido no próprio milissegundo do início é descartado) e só vale com o dispositivo ainda em uso (`DGROUP:0af0/0af2`, o cabeçalho do efeito anterior ainda não liberado pelo `MM_WOM_DONE`) [código]. Em consequência, de dois efeitos pedidos seguidos no mesmo tick sem espera entre eles, **fica o primeiro**. **Port (desde 2026-10-19):** `ControleSom` guarda o instante de início do último efeito aceito (o imediato ou o agendado) e descarta o pedido que começaria no mesmo milissegundo; parar os efeitos (opção ou foco) zera essa memória (`simant/src/som/controle.rs`, teste `efeito_no_mesmo_milissegundo_e_descartado`). [hipótese: a resolução de `GetTickCount` no Windows 3.1 pode ser maior que 1 ms; o port usa o milissegundo do relógio dele]

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
- Efeito: tipo 5 decodificado e reamostrado de `taxa_efeitos` (4096 Hz) para a saída. Um efeito novo corta o anterior, exceto no mesmo milissegundo (descartado; seção acima).
- Opções: segue `_OptionStates[1]` (música) e `[2]` (efeitos) do estado da partida; desligar a música para a que toca.
- Foco (`WM_ACTIVATEAPP`), **em todas as telas** (abertura, "Select a Game" e partida): a cada quadro o executável lê se a janela tem o foco do teclado e não está minimizada (`janela_ativa` em `simant/src/jogo.rs`; antes só os eventos de foco da partida, que um laço modal podia engolir, e a abertura não tratava o foco). Ao ficar inativa: para a música, corta o efeito, esquece os agendados e não toca nada até voltar; ao voltar, nada recomeça. A música da abertura e a do jogo novo são pedidas ao criar a janela, antes de o SDL entregar o foco: ficam guardadas até o primeiro foco [escolha do port; no original, com o aplicativo inativo na abertura, `_songsOnFlag` = 0 e a música não tocaria]. O usuário observou no original a música da abertura parando sem foco. Lógica em `simant/src/som/controle.rs` (`ControleSom`), com testes.
- Abertura e jogo novo tocam `musica_abertura` e `musica_jogo_novo` (10001).

**Linha do tempo dos efeitos.** Dentro de um tick o original pode ficar parado em animações: `_AnimYellowFight` (`1028:7e56`) espera, no começo de cada quadro, `_MacTickCount() > início + 6` [código]; como o relógio do Mac anda de 3 em 3 a cada 55 ms (quintal.md, "Relógios"), são 3 degraus = **165 ms** entre dois quadros (o primeiro espera só o próximo degrau, até 55 ms; depois do último não há espera). O som do quadro (0x25 na luta com `S2` ≠ 0, 0x31 no choque) sai logo depois da espera. Na luta (6 quadros) os 0x25 ficam espalhados em ~825 ms, e o efeito pedido depois da animação (no mesmo tick ou no seguinte) só sai no fim dela — e, como depois do último quadro não há espera, um efeito pedido logo depois do último 0x25 cai no mesmo milissegundo e é descartado (desde 2026-10-19). O port antes recebia os pedidos do tick inteiro no mesmo quadro de vídeo: cada efeito cortava o anterior na hora, e a luta virava, no máximo, um 0x25 — muitas vezes cortado logo em seguida pela escavação (0x11/0x12) ou por outro efeito do mesmo tick; é por isso que o som da luta quase não se ouvia. Agora as regras pedem `Evento::Espera(ms)` entre os quadros (`amarela.json`, `animacao.espera_quadro_ms` = 165) e o `ControleSom` agenda cada efeito no instante certo: os seguintes à espera vão para uma fila, e a fila do quadro seguinte começa depois das esperas já agendadas. Desde 2026-10-18 a simulação do port também fica parada durante a animação e a janela de edição mostra os quadros (`Evento::Animacao`, [formiga-amarela.md](formiga-amarela.md) §10.2), como no original: nenhum som de outro tick pode cortar os da luta. Volume: o 0x25 tem 0,46 s e é dos efeitos mais baixos (RMS 0,19 contra 0,25–0,40 da maioria, medido nos WAV de `--exportar-som`); o port toca efeitos e música com `volume_efeitos` = `volume_musica` = 0,5 de `data/som.json`. A proporção entre o `waveOut` e o MIDI foi medida no DOSBox-X em 2026-10-19 (música 20 dB abaixo de um efeito de ganho 1, seção "Áudio do original no DOSBox-X"); o banco FM está calibrado para ela.
- `--sem-som` desliga tudo; `--exportar-som DIR` grava os efeitos e as músicas decodificados em WAV, sem abrir áudio.

**Sintetizador.** Ver `docs/decisoes.md` (2026-10-17 e 2026-10-19). `data/som.json` → `sintetizador` escolhe (linha de comando: `--sintetizador`): **`"fm"`, o padrão desde 2026-10-19** — sintetizador FM próprio no estilo do OPL2/OPL3 (`simant/src/som/fm.rs`) com um banco General MIDI próprio e livre (`data/opl_banco.json`, gerado por `tools/gerar_banco_opl.py`; ou um `.op2` do usuário em `fm.banco_op2`), o caminho mais próximo da Sound Blaster do original (comparação abaixo); `"soundfont"` — `rustysynth` com a SoundFont de `--soundfont`, `SIMANT_SOUNDFONT` ou `soundfont`; `"simples"` — ondas básicas por família GM e ruído na percussão. Sem a chave, vale o comportamento até a API 0.30 (SoundFont se houver, senão o simples). O timbre FM não é o do driver da Creative (banco próprio, chip não emulado ciclo a ciclo): `docs/divergencias.md`. `--exportar-som` grava as músicas com `volume_musica` (o nível do jogo) e `--exportar-musica ID` limita a exportação a uma música.

**Não reproduzido:** os modos Sound Blaster, alto-falante e Disney Sound Source como caminhos automáticos; `_mySongIsDone` falso no modo Sound Blaster (no port as músicas seguidas tocam como no MCI).

## Áudio do original no DOSBox-X (2026-10-19)

Ferramenta: `tools/som_dosbox.py` (`capturar`, `niveis`, `comparar`, `harmonicos`, `midi`, `quadros`). **Nada toca no PC:** o DOSBox-X roda com `[mixer] nosound = true` (o som continua emulado e a captura em WAV funciona; conferido: os WAV têm o áudio) e `SDL_AUDIODRIVER=dummy`, com a janela minimizada e sem ativação; nenhuma tecla ou clique do usuário é usado. Os arquivos do jogo são copiados para `re_trabalho/som/C` (pasta ignorada pelo controle de versão); `SimAnt-T` e `referencia/dosbox-x.conf` não são tocados. A captura é pelo comando `DX-CAPTURE /A` do DOSBox-X em volta do `win C:\MAXIS\ANTWIN\SIMANTW.EXE` (com `/V`, também vídeo ZMBV, de onde `quadros` extrai os quadros-chave em PNG para saber o que estava na tela).

**Instalação de referência (fato verificado nos arquivos de `SimAnt-T/WINDOWS`):** `SYSTEM.INI` → `[drivers]` `Midi=sb16fm.drv` ("Creative Stereo Music Synthesizer Driver" 2.00, OPL3), `Wave=sb16snd.drv`, `midimapper=midimap.drv`; `[sndblst.drv]` com `VoiceVolume` = `FMVolume` = 16,16. O `MIDIMAP.CFG` tem as configurações da Creative ("SB16 All/Basic/Extended MIDI", "SB16 Basic/Extended/All Music Synthesis"); a palavra no offset 6 do cabeçalho vale 0x0A, o número da configuração **"SB16 Ext Synth"** (Extended Music Synthesis) [inferido: o formato do `MIDIMAP.CFG` não é documentado; o número casa com o registro dessa configuração]. Nela os canais 1–10 vão para o "Creative Music Synthesizer" nos mesmos canais, **sem mapa de programas nem de teclas**, e os 11–16 ficam mudos: o MIDI Mapper entrega ao driver FM só o arranjo estendido dos `.MID`, com os programas GM sem troca — o que o port faz (`canais_midi` 1–10). O `SB16FM.DRV` tem só 4 KB; o banco de timbres FM da Creative não está nele [hipótese: fica no `VSBPD.386`; não procurado] e o port não o usa.

**Capturado (sem input):**

1. **Abertura (10001, `gamethme`)**, 130 s: o som começa 6,26 s depois do início do Windows e dura **102,90 s** (o resto é silêncio, ruído de fundo −74 dBFS). Saída **mono** (canais E e D idênticos, correlação 1,000). Os quadros do vídeo mostram o título até ~40 s (o tempo limite de 720 ticks) e depois o "Select a Game", parado; a música toca até o fim depois que o título fecha.
2. **Nível de um efeito no mesmo caminho:** o efeito 41 (água, 3,15 s) decodificado pelo port, reamostrado para 11 025 Hz e posto como som "Windows Start" no `WIN.INI` da cópia (`capturar --som-inicio`), sai pelo mesmo `sb16snd.drv` (`waveOut`) dos efeitos do jogo, 1,7 s antes da música. Na gravação ele aparece com **correlação 0,99989** com o PCM e **ganho 0,2504 (−12,0 dB)** em relação à escala cheia de 8 bits. A música, na mesma escala, tem **RMS −32,0 dBFS** (pico 0,15). Logo, no original, **a música fica 20,0 dB abaixo de um efeito tocado a ganho 1** (RMS da música contra o ganho do efeito, com os volumes do mixer da SB16 da instalação). Com o efeito a 4096 Hz (a taxa do jogo), o Windows 3.1 travou ao tocar o som de início ("Corrupt UMB chain" e laço de interrupção inválida no log) [observado, não investigado: o `waveOutOpen` do jogo pode seguir outro caminho]; a taxa de 11 025 Hz não muda o nível.

**Não capturado:** as outras músicas e os efeitos dentro do jogo. O `AUTOTYPE` do DOSBox-X não entregou teclas com a janela minimizada e sem foco (testado no DOS, digitando um `mkdir`, e no Windows, com espaço no título e Enter no "Select a Game": nada mudou na tela nem no som), e operar a janela exigiria o foco do PC do usuário. O `-time-limit` também não encerra o DOSBox-X quando o convidado trava (foi preciso encerrar o processo).

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
- **Timbre:** no original o baixo (programa 34) tem o 2.º harmônico como o mais forte, o 1.º 10 dB e o 3.º 6 dB abaixo e os 4.º–12.º planos entre −17 e −24 dB (27 notas isoladas, `som_dosbox.py harmonicos`); o piano (programa 3) começa com os 5 primeiros harmônicos quase iguais e os agudos caem mais rápido que a fundamental; o tom 43 soa como um tom grave de ~69 Hz com a altura caindo; não há ruído na percussão (espectrograma). O sintetizador simples é claro demais (ondas serra/quadrada e percussão de ruído branco: 18 % da energia acima de 4 kHz, contra 6 %); o FM chega mais perto no espectro médio e na distribuição por faixa, mas ainda fica escuro entre 2 e 4 kHz (5 % contra 12 %). O banco FM foi ajustado nos programas de `gamethme` (baixo 34: perfil −2/0/−6/−11/−12/−13… no port, contra −10/0/−6/−22/−17/−17… no original). Os demais programas usados pelas músicas (19, 21, 25, 31, 33, 35, 49, 57, 59, 69, 95) não puderam ser comparados, por falta de captura [em aberto].
- **Volume relativo:** com o simples, a música ficava 3,7 dB mais baixa em relação aos efeitos do que no original; o banco FM tem `ganho` 2,25, calibrado para a abertura dar −19,8 dB (contra −20,0 dB medidos). As outras músicas ficam no nível relativo que o banco der [não medido].

## Áudio do original na bancada QEMU (2026-10-18)

Tentativa de gravar o som do original para comparar com `--exportar-som`. Ferramenta: `tools/som_original.py` (`boot`, `latido`, `chuva`, `luta`, `analisar`, `procurar`). **Resultado: a bancada grava áudio, mas o que sai não serve de referência; a comparação de ouvido continua aberta.**

**Montagem (funciona).** A cópia do WfW 3.11 da bancada já tem os drivers da Sound Blaster 16 (`SYSTEM.INI`: `Wave=sb16snd.drv`, `Midi=sb16fm.drv`, `Midi1=sb16snd.drv`, `device=vsbpd.386`; `[sndblst.drv]` Port=220, Int=7, DmaChannel=1, HDmaChannel=5). O QEMU de `referencia/qemu` tem `sb16`, `adlib` (OPL2) e o backend `wav`: `-audiodev wav,id=som,path=X.wav -device sb16,audiodev=som,irq=7 -parallel none` (a porta paralela ocupa a IRQ 7), mais `-device adlib,audiodev=som` para o FM (`SIMANT_SOM_OPL=1`). A máquina muda, então é preciso boot completo (os snapshots da bancada não carregam) e um snapshot próprio (`som`). No Quick Game assim montado, `_songsOnFlag` = `_effectsOnFlag` = 1 e o cachorro posto sentado (`_DogDir` = 4) latiu ~6 vezes em 40 s (`_DogBarkDelay` renovado: 6270, 6416, 6726, 6861, 7308, 7675).

**Por que não serve (observado):**

1. **Música (FM).** O `sb16fm.drv` é driver de OPL3; o `adlib` do QEMU é OPL2 e trata 0x38A/0x38B como 0x388/0x389, de modo que as escritas do 2º banco caem no 1º. Com o `adlib`, o WAV tem som contínuo (RMS ≈ 0,19) desde a carga do Windows, antes do jogo, e durante todo o Quick Game, sem relação com `gamethme`: notas presas. Sem o `adlib`, nenhuma música toca (o WAV do boot fica vazio). Não há OPL3 no QEMU.
2. **Tempo.** O backend `wav` só grava enquanto alguma voz está ativa: sem o `adlib`, os silêncios entre efeitos somem (3 s de áudio para ~45 s de jogo com 3 Home e ~6 latidos). Ordem e duração no tempo não são recuperáveis; com o `adlib` ativo o tempo é contínuo, mas sob carga do host o WAV sai mais curto que o tempo da VM (52 s gravados num boot de ~150 s).
3. **Conteúdo dos efeitos.** Os efeitos do port (tipo 5 decodificado, `--exportar-som`) **não aparecem** na gravação: a correlação normalizada (`som_original.py procurar`, reamostrando a 4096, 5000, 5512 e 6000 Hz e, para o efeito 15, de 3500 a 12000 Hz) fica abaixo de 0,5 para os latidos 21–23 (efeitos de poucas dezenas de ms dão picos de até 0,7 por acaso). Cada Home produz um trecho de 0,8–1,1 s, quando o efeito 15 tem 102 amostras (25 ms a 4096 Hz); as quebras da onda (interpolação linear do QEMU) vêm a cada 7–8 amostras de 44 100 Hz, isto é, uma fonte de ~5,5–6,3 kHz e não 4096 Hz. [hipótese] o `sb16snd.drv` sobe a taxa para o mínimo da placa (5000 Hz) e o `sb16` do QEMU toca o buffer de DMA inteiro (dados velhos) em vez do bloco pedido; não isolado. A chuva (efeito 41, 3,1 s; `_GlobalKey` = 0xA8, `RainOn` = 1, `WaterTable` 62 → 48 em 25 s) não gerou WAV nenhum.

**Continuava sem verificação:** timbre, volume relativo efeito/música, os sons da luta no tempo e a preempção no mesmo milissegundo. A captura pelo DOSBox-X (seção acima, 2026-10-19) resolveu, para a abertura, andamento, notas, volume relativo e uma referência de timbre; os efeitos dentro do jogo (taxa real na SB16, luta, preempção) continuam sem captura.
