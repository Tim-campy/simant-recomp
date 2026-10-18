# Balões de fala da janela de edição (opção "Silly")

Com Options → Silly ligado (`_OptionStates[5]`, `8:85fc`), a janela de edição mostra balões de fala sobre lutas, formigas descansando, a rainha, os ovos e a aranha. Fonte: pseudocódigo do Ghidra (endereços abaixo) [código]; listas de textos lidas do banco (`inspecionar strings`) [dado]. Conferência na bancada: ver "Verificação".

Implementação: `engine/src/rules/baloes.rs` (anotação no tick, sorteios do redesenho, escolha do balão), `engine/src/core/baloes.rs` (forma, `_MakeBalloon`), `engine/src/core/tela_jogo.rs` (`desenhar_balao`). Dados: `data/baloes.json` (listas, esperas, chances, geometria) e as peças 8×8 de `data/quintal_mapa.json` (`balao_menino.pecas`, as mesmas do balão do menino).

## 1. No tick: onde há balão

Passo 3 de `_DoAntSim` (`1028:0000`), com Silly = 1: zera `_FightBalloonCnt`, `_RestBalloonCnt`, `_QueenBalloonCnt`, `_EggBalloonCnt` e põe −1 nos últimos pontos (`_LastFightPnt`, `_LastRestPnt`, `_LastQueenPnt`, `_LastEggPnt`, x e y) [código]. Os planos `_Last*Plane` ficam.

Quem anota (sempre conferindo Silly antes, como o original; nenhuma delas sorteia) [código]:

| Rotina | Chamada por | Condição | Plano |
|---|---|---|---|
| `_FightBalloons(x, y, p)` (`1010:499a`) | `_DoFightA` (`1028:27e6`); `_DoNestFightB/R` (`1028:3a54`/`6072`) | a nuvem de luta continua (o `SRand16` não deu 0) | 1, 2, 3 |
| `_RestBalloons(x, y, p)` (`1010:4aee`) | descanso da superfície (`_DoRestAnt` `1028:0b76` e o mesmo trecho em `_DoAntSimA`); descanso nos ninhos (`_DoRestB/R` e o trecho de `_DoNestAntB/R`) | a formiga continua descansando (`SRand4` ≠ 0 na superfície, `SRand1(20)` ≠ 0 nos ninhos) | 1, 2, 3 |
| `_QueenBalloons(x, y, p)` (`1010:4a44`) | `_SimQueenB/R`, cabeça da rainha | no fim, depois da conferência da cauda (não quando a rainha andou nem quando morreu de fome) | 2, 3 |
| `_EggBalloons(x, y, p)` (`1010:48f0`) | `_SimEggB/R` | o ovo não eclodiu neste tick (um ovo preto que morreu conta) | 2, 3 |

Cada uma faz [código]: se a contagem do tipo ainda é 0 e a célula está na vista (`_BalloonIsVisible`, abaixo), então, se (x, y, p) é o balão atual (`_CurFightPnt`/`_CurFightPlane` etc.), a contagem vira 1 (o balão continua); senão (x, y, p) passa a ser o último ponto. Depois da contagem 1 as chamadas seguintes do mesmo tick não mudam nada.

`_BalloonIsVisible(p, x, y)` (`1010:48a8`): `p = _MapPlane`, `_MapPnt.x ≤ x < _MapPnt.x + _editWidth` e `_MapPnt.y ≤ y − 3 < …`, `y < _MapPnt.y + _editHeight`: a célula precisa ter ao menos 3 linhas da vista acima dela (lugar para o balão) [código].

## 2. No redesenho da janela de edição: sorteios

`FUN_1010_16d4` (o redesenho do buffer da janela de edição: depois de cada tick em `_UpdateEdit`, na rolagem e em `_UpdateEditIfBufInvalid`) faz, nesta ordem: `_PreDrawSpider`, **`_DrawCurBalloons`**, as células, `_DrawSpider` com a aranha visível (palpos e **balão da aranha**), **`_DrawBalloons`** [código]. As duas rotinas marcadas consomem o gerador S; os prazos são em `_TickCount` (`1008:18ac` = `GetTickCount() / 55`, tempo real) [código].

### 2.1 `_DrawCurBalloons` (`1010:4cac`)

Só com Silly ≠ 0 e `_ELayerMode` = −1 (sem camada de cheiro). Para cada tipo, na ordem **luta, ovo, rainha, descanso** [código]:

1. **Balão novo:** contagem 0 e último ponto ≥ 0 (x e y) → o último ponto e plano passam a ser os atuais, a contagem vira 1 e os prazos das falas do tipo vão a 0. Para ovo, rainha e descanso a fala também é desligada; para a luta, não.
2. Com contagem > 0 e o jogo **não pausado** (`_GamePaused` = 0), para cada fala do tipo cujo prazo venceu (prazo < `_TickCount`, 32 bits): prazo = `SRandK` + `_TickCount` + espera; depois `SRandC` = 0 liga a fala e avança o índice da fala (volta a 0 quando o ponteiro seguinte da lista é nulo), ≠ 0 desliga.
3. Cada fala ligada pede o balão (`_AddMsgBalloon`, embutido): ponto = célula atual × 16 + 8 (meio do tile), plano atual, texto = entrada do índice na lista; até 6 pedidos e só se a célula do ponto está na vista (a mesma conta, com o ponto dividido por 16).

| Fala | Tipo | Lista | `K` do prazo | espera | `C` da chance |
|---|---|---|---|---|---|
| luta, preta | luta | 1220 (8 falas: "Take that\nyou fiend!"…) | 32 | 60 | 2 |
| luta, vermelha | luta | 1230 (8: "Your queen mates\nwith termites!"…) | 32 | 60 | 2 |
| ovo | ovo | 1240 (15: "Where am I?"…) | 32 | 170 | 4 |
| rainha | rainha | 1250 (8: "Being a queen is\nterribly overrated!"…) | 32 | 180 | 4 |
| descanso | descanso | 1260 (8: "Whew! I'm\nexhausted!"…) | 64 | 120 | 2 |

A luta tem duas falas (um balão de cada cor, cada uma com o seu prazo), sorteadas as duas antes dos pedidos. Com um balão novo os prazos são 0, então os sorteios acontecem no mesmo redesenho. Consumo de S por redesenho: 2 por fala com prazo vencido (`S32`/`S64` e `S2`/`S4`), nada enquanto o prazo não vence. As listas são carregadas em `_initStuff` (`_LoadStringAnt(0x4c4, 0x4ce, 0x4d8, 0x4e2, 0x4ec)`) [código]. Os índices começam em 0 e nunca são zerados; como o índice anda antes de ser usado, a primeira fala mostrada de cada lista é a de índice 1 [código].

Por fim, havendo pedidos e o buffer válido, `_editForce = 1` e `InvalidateRect` (redesenho completo da janela) [código; no port o quadro inteiro já é redesenhado].

### 2.2 Balão da aranha (fim de `_DrawSpider`, `1010:1d6a`)

Só com a aranha visível (`_PreDrawSpider`) e Silly ≠ 0, depois dos palpos [código]:

1. Sem pausa e com o prazo (`DGROUP:0x87d6`) vencido: prazo = `SRand64` + `_TickCount` + 180; `SRand2` = 0 liga e avança o índice (`0x87da`, de 0 a 4: depois de 4 volta a 0), senão desliga.
2. Se `_SMode` é diferente do último visto (`0x72f4`) ou > 4: prazo = `SRand32` + `_TickCount` + 30, desliga, guarda `_SMode` e acaba. **Esse sorteio acontece mesmo pausado**, e a cada redesenho enquanto `_SMode` = 5 (aranha morrendo).
3. Ligado: texto = entrada `_SMode × 5 + índice` da lista 1210 (25 falas, 5 por modo: parada, andando, perseguindo, comendo, fugindo); ponto = (`_SpidX` + 8·`_Dx8[_SpidDir]`, `_SpidY` + 8·`_Dy8[_SpidDir]`) em pixels do mapa, plano 1; pedido com a mesma conferência de vista (a superfície tem de ser o plano mostrado).

O balão da aranha entra na lista depois dos de `_DrawCurBalloons`.

### 2.3 `_DrawBalloons` (`1010:57aa`)

Com a fonte 2, percorre os pedidos e **desenha só o primeiro** que está na vista (o laço para depois do primeiro desenhado) [código]; zera a lista. O balão é feito por `_MakeBalloon(texto, 0)` (cunha da esquerda; a mesma rotina do balão do menino, telas/quintal.md §2.3) e copiado com máscara no buffer da janela de edição com o canto em (x + 4, y − (altura + 4)) em pixels do mapa (x, y = ponto do pedido; altura = a registrada por `_MakeBalloon`, `altura_fonte × n + 15`, uma a menos com uma linha). As células cobertas ficam marcadas para o próximo redesenho, que apaga o balão se ele não for pedido de novo [código]. O balão fica na tela até o próximo redesenho do buffer (pausado, fica parado).

## 3. Gráficos de saúde da janela de edição (`_DrawEditGraphs`, `1010:198a`)

`_DrawEditGraphs` (chamada depois de cada cópia do buffer, `_win_DrawEditWindow`, e na rolagem) desenha três barras verticais nos objetos 17–19 da janela 0 (`editHealthY`, `editHealthB`, `editHealthR`, na coluna de ícones `editIcons`), com `_MeHealth`, `_HealthB` e `_HealthR` (ponteiros na tabela `8:8e00`, objetos em `8:8dfa`), a marca do `_MeWarnHealth` (linha branca) e do `_BlkWarnHealth` (linha invertida) [código]. **Mas a rotina não faz nada com `_displayType` 9 ou 10** — os dois modos cujo banco gráfico existe (`MWINNT` e `WINGANT`; os outros são herança do DOS, formatos/simant-cfg.md). Logo, no SimAnt para Windows, a janela de edição **não tem gráficos de saúde**: a saúde aparece só nas barras da faixa (telas/mensagens.md §9). O port não os desenha, de propósito. [hipótese: resto das versões do DOS e do Mac. Com a faixa, a coluna de ícones da janela 0 nem aparece: o mapa começa na borda de dentro da moldura, telas/jogo.md.]

## 4. Port

- Estado: `Estado::baloes` (`rules::estado::Baloes`, fora do save): contagens, últimos e atuais pontos, `_editWidth/_editHeight` (`vista`, posto pela interface antes do tick com `_MapPnt`), prazos, falas ligadas, índices, `_SMode` visto, a lista de pedidos e o balão desenhado.
- Tick: `baloes::anotar` nos lugares da tabela da §1 (`rules/superficie.rs`, `rules/ninho.rs`).
- Redesenho: `desenho::redesenho_do_timer` → `desenho::desenhar_edicao` (balões, palpos, balão da aranha, escolha), nas mesmas condições do redesenho da janela de edição já modeladas (rng.md, "Sorteios do desenho"); `_TickCount` = `ms_total / 55` da partida (`JanelasDesenho::tick_count`).
- Desenho: `TelaDeJogo::desenhar_balao` desenha `Baloes::desenhado` com as peças e a fonte, se a célula ainda está na vista.

Lacunas e hipóteses:

- O redesenho da rolagem (`_DoEditScroll`, `FUN_1010_16d4` fora do timer) e os de dentro do tick (`_DoEditUpdateDraw`) não sorteiam no port (como os palpos) [lacuna].
- O texto centrado na largura do balão [hipótese, a mesma do balão do menino].
- O índice da fala é guardado sem volta no port; a interface o reduz ao tamanho da lista (equivalente à volta a 0 do original enquanto a lista não muda).
- Num Load, os prazos e índices continuam os da sessão (o original também não os grava) [código].

## Verificação

Pendente de captura da bancada (ver `docs/estado-do-projeto.md`). Os vetores de tick do repositório têm todos Silly = 0 (`_OptionStates` = 0,1,1,1,1,0 nos 150 estados conferidos), então não exercitam esta parte.
