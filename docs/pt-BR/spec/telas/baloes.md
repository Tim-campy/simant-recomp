*Read in English: [baloes.md](../../../spec/telas/baloes.md)*

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

Texto de cada linha (`_MakeBalloon`): começa na coluna x0 + 8 (a segunda coluna de peças), deslocada `((−largura) & 6) / 2` pixels (`_font_MakeImage(texto, (−w & 6) >> 1)`, a imagem do texto vai para o byte seguinte da máscara); para a linha mais larga isso coincide com centrar, as outras ficam quase alinhadas à esquerda [código + bancada, abaixo]. A linha i fica em y0 + `altura_fonte × i` + 6.

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
- O índice da fala é guardado sem volta no port; a interface o reduz ao tamanho da lista (equivalente à volta a 0 do original enquanto a lista não muda).
- Num Load, os prazos e índices continuam os da sessão (o original também não os grava) [código].

## Verificação

Os vetores de tick do repositório têm todos Silly = 0 (`_OptionStates` = 0,1,1,1,1,0 nos 150 estados conferidos), então não exercitam esta parte. Conferência própria na bancada (2026-10-18, `tools/cenarios_tick.py gravar baloes`, snapshot `rico`): Silly escrito na entrada de um tick, vista do ninho preto com `_MapPnt` = (22, 0) (rainha em (34, 6), ovos em (34, 8)), 60 intervalos entre o fim de um tick e a entrada do seguinte, com 1,5 s de VM livre entre eles; em cada intervalo, as variáveis dos balões (tabela abaixo) e o gerador S.

- **Sorteios:** nos três intervalos em que S andou, o número de passos e os valores batem com o modelo da §2.1: balão novo (contagem 0 → 1 para ovo e rainha, pontos atuais = últimos) com 4 passos, **ovo antes da rainha**, cada um `S32` + `S4`, e prazos `5993 = 2 + agora + 170` e `6003 = 2 + agora + 180` com o mesmo `agora` (5821); prazo vencido do ovo com `S32` = 7, `S4` = 2 (desliga: `_WantEggBalloon` 1 → 0) e prazo `6735 = 7 + 6558 + 170` (6558 > prazo anterior 6557); outro com `S4` = 0 (liga, índice 2 → 3). Nos outros 57 intervalos, sem prazo vencido, S não andou.
- **Desenho:** com as falas do ovo e da rainha ligadas, só o balão do ovo aparece (o primeiro pedido); a primeira fala mostrada é a de índice 1. O port (`balao_do_ovo_como_no_original` e `balao_da_rainha_como_no_original`, `simant/src/modos.rs`, com os estados `tests/referencias/saves/BALAO_OVO.ANT` e `BALAO_RAINHA.ANT`) desenha a região do balão **100%** igual às capturas (`comparar_captura.py --topo 38 --mapear`; ovo: "Sure is dark\nin here!", rainha: "Sometimes I think all I\ndo is lay eggs!"). As duas capturas decidiram o alinhamento do texto (antes centrado [hipótese]).

Não conferidos na bancada: balões de luta, de descanso e da aranha (mesmo código, outras constantes), a pausa e a troca de `_SMode` da aranha.

Nomes das variáveis (`SIMANTW.SYM`): `_FightBalloonCnt` `9:7404`, `_RestBalloonCnt` `9:a07e`, `_QueenBalloonCnt` `9:8122`, `_EggBalloonCnt` `9:7d44`; `_Cur*Pnt`/`_Cur*Plane` (`_CurFightPnt` `9:7a52`, `_CurEggPnt` `9:7d2a`, `_CurQueenPnt` `9:7d64`, `_CurRestPnt` `9:7d40`); `_Last*Pnt`/`_Last*Plane`; prazos `_BFightBalloonTicks` `9:811e`, `_RFightBalloonTicks` `9:9e6a`, `_EggBalloonTicks` `9:a0e6`, `_QueenBalloonTicks` `9:78a8`, `_RestBalloonTicks` `9:9ce6`, `_SpidBalloonTicks` `8:87d6`; falas ligadas `_WantBFightBalloon` `9:9bea`, `_WantRFightBalloon` `9:a0d4`, `_WantEggBalloon` `9:9afe`, `_WantQueenBalloon` `9:7c3c`, `_WantRestBalloon` `9:72be`, `_WantSpiderBalloon` `9:75f8`; índices `_BFightBalloonIndex` `9:7a50`, `_RFightBalloonIndex` `9:810e`, `_EggBalloonIndex` `9:9eaa`, `_QueenBalloonIndex` `9:78d6`, `_RestBalloonIndex` `9:9e86` (o da aranha, `8:87da`, sem nome); `_LastSMode` `9:72f4`; lista de pedidos `_CurBalloonCnt` `9:9fd0`, `_CurBalloonPnts` `9:80d6`, `_CurBalloonPlane` `9:7d68`, `_CurBalloonFlags` `9:78be`, `_CurBalloonMsgs` `9:7c24`.
