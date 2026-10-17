# Especificação do SimAnt (Windows 3.x)

Jogo: **SimAnt: The Electronic Ant Colony** (Maxis, 1991), versão Windows por "Lightning" Rod Lai. Gênero: simulação/estratégia (colônia de formigas). O jogador controla uma formiga amarela da colônia preta, que disputa o quintal e a casa com a colônia vermelha. Os créditos e a descrição saem de bitmaps do `WINGANT` com ids entre 1000 e 4999 (folha de contato em `re_trabalho/png/sheet_wingant_a.png`): a tela de créditos e o menu "Select a Game", que oferece Tutorial, Quick Game, Full Game, Experimental e Load Game.

## Convenção de evidência

- **Fato verificado:** testado no jogo original ou validado sobre os arquivos reais com ferramenta reproduzível (o texto diz qual).
- **Fato visto no código:** lido no pseudocódigo do Ghidra, com o endereço `seletor:offset` e o nome do `.SYM`.
- **[hipótese]:** inferência ainda não confirmada.
- **[em aberto]:** pergunta conhecida, sem resposta ainda.

Ambiente de referência: DOSBox-X (`referencia/`). Comparações com o original estão em `tests/referencias/README.md`.

## Índice

- [executavel.md](executavel.md): mapa do `SIMANTW.EXE`, módulos e fluxo principal.
- [arquivos-do-jogo.md](arquivos-do-jogo.md): inventário dos arquivos da instalação.
- Telas:
  - [telas/abertura.md](telas/abertura.md): abertura (`_ShowIntro`), implementada no marco 1.
  - [telas/jogo.md](telas/jogo.md): tela de jogo (faixa + janela de edição com chão e vida), planos, rolagem e reconstrução de `_LifeA/B/R`.
  - [telas/mensagens.md](telas/mensagens.md): avisos ao jogador (mensagem da faixa, quadros de mensagem, fim de jogo e pontuação), botões configuráveis da faixa e salvar.
- Formatos:
  - [formatos/banco-dat-ndx.md](formatos/banco-dat-ndx.md): banco de recursos e compressão LZSS.
  - [formatos/bitmap.md](formatos/bitmap.md): bitmaps e paleta.
  - [formatos/fonte.md](formatos/fonte.md): fontes (FONT do Mac e Gaia do `FONTRES.FON`) e posicionamento de texto.
  - [formatos/janela.md](formatos/janela.md): sistema de janelas (layouts, cores, posições, cálculo e posicionamento na tela).
  - [formatos/simant-cfg.md](formatos/simant-cfg.md): `SIMANT.CFG`.
  - [formatos/save-ant.md](formatos/save-ant.md): jogo salvo e modelo de estado da simulação.
  - [formatos/som.md](formatos/som.md): efeitos (tipo 5, tipo 51), cabeçalhos de música (tipo 18), MIDI (`SOUND\*.MID`, tipos 20 e 50) e as saídas de áudio do original.
- Regras:
  - [regras/rng.md](regras/rng.md): geradores aleatórios (LFSR `_SRand*` e `rand` do C em `_RRand`), sementes e quem usa cada um.
  - [regras/geracao-mundo.md](regras/geracao-mundo.md): novo jogo (`_NewGame`, `_RandYard`, `_RandWorld`), patches do quintal e da casa, rainhas, comida e tutorial; verificada contra o `QUICK2.ANT`.
  - [regras/tiles-valores.md](regras/tiles-valores.md): significado dos valores de `_MapA`, `_MapB` e `_MapR` e das funções de teste de tile.
- Regras:
  - [regras/simulacao-visao-geral.md](regras/simulacao-visao-geral.md): tick (`MYTIMERFUNC`/`_DoAntSim`), ordem e frequência das sub-rotinas, velocidade, pausa, fim de jogo e pontuação.
  - [regras/formigas.md](regras/formigas.md): listas A/B/R, campos X/Y/M/T/S, castas, modos, triângulos de controle e máquina de estados.
  - [regras/feromonios.md](regras/feromonios.md): mapas de cheiro (`_PherMap*`), depósito, decaimento e leitura.
  - [regras/superficie.md](regras/superficie.md): formigas da superfície (`_DoAntSimA` e cada modo), funções de direção, luta, cadáveres, entrada no ninho, comida, estratégia preta/vermelha, recrutamento e formiga vermelha líder.
  - [regras/inimigos.md](regras/inimigos.md): formiga-leão, aranha (inclusive controlada pelo jogador), lagarta e bicho-de-conta.
  - [regras/formiga-amarela.md](regras/formiga-amarela.md): formiga amarela (estado `_Me*`, movimento e caminho, ninho, pegar/largar, comer, luta, troca de corpo, recrutamento, morte e reencarnação) e controles do jogador (mouse, teclas, menus, aceleradores, modo Experimental).
  - [regras/ninho.md](regras/ninho.md): formigas dentro dos ninhos preto e vermelho (`_DoAntSimB/R`, `_DoNestAntB/R`): despacho por modo, movimento, luta, comida guardada, escavação, mapa de saída, invasoras, água e inundação; diferenças R × B e casos de teste.
  - [regras/ciclo-de-vida.md](regras/ciclo-de-vida.md): postura, desenvolvimento e eclosão dos ovos (`_SimEggB/R`), rainha (cabeça e cauda, andar, fome), escolha de casta (`_SetCasteProd`, `_CasteTabC`) e casos de teste.
  - [regras/testes-tick.md](regras/testes-tick.md): determinismo do original na bancada QEMU, vetores "estado antes → depois" de um tick e o que capturar além do save.
  - [regras/quintal.md](regras/quintal.md): quintal (`_DoSimYard`): menino dentro e fora de casa, cortador, pé e inseticida, gato, cachorro, pássaro, chuva e água, colônias do quintal 12 × 16 (`_SimColonies`, revoada, migração) e fim do Jogo Completo; casos Q-01 e Q-02 confirmados no original.
  - [regras/som.md](regras/som.md): funções de som, opções Music/Sound Effects, preempção e catálogo de quem toca cada música e efeito.
  - [regras/conselhos.md](regras/conselhos.md): `_Feedback`: mensagens de conselho (lista 4/1020) e tutorial (`_GiveLesson`/`_LessonDone`, lições 4/11000…11112).

## Próximos subsistemas a especificar

1. Mapa de jogo: tiles (objetos tipo 9, ids 13–20) e mapas de superfície/ninho (`_MapA`, `_MapB`, `_MapR`), para exibir um mapa (marco seguinte do plano).
2. Mapas de superfície e ninho (`_MapA`, `_MapB`, `_MapR`) e tiles (tipo 9).
3. Regras da simulação (`SIMANT1_MODULE`, `SIMTWO_MODULE`), uma regra por vez, cada uma com caso de teste contra o original.
4. ~~Som e música~~: especificados em formatos/som.md e regras/som.md; falta as regras emitirem `Evento::Som`.
