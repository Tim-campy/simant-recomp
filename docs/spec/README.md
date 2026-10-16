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
- Formatos:
  - [formatos/banco-dat-ndx.md](formatos/banco-dat-ndx.md): banco de recursos e compressão LZSS.
  - [formatos/bitmap.md](formatos/bitmap.md): bitmaps e paleta.
  - [formatos/janela.md](formatos/janela.md): sistema de janelas (layouts, cores, posições, cálculo e posicionamento na tela).
  - [formatos/simant-cfg.md](formatos/simant-cfg.md): `SIMANT.CFG`.
  - [formatos/save-ant.md](formatos/save-ant.md): jogo salvo e modelo de estado da simulação.

## Próximos subsistemas a especificar

1. Fontes (`FONT1`–`FONT4`, `FONTRES.FON`, `_font_*`): necessárias para os textos das janelas (registro, menus).
2. Mapas de superfície e ninho (`_MapA`, `_MapB`, `_MapR`) e tiles (tipo 9).
3. Regras da simulação (`SIMANT1_MODULE`, `SIMTWO_MODULE`), uma regra por vez, cada uma com caso de teste contra o original.
4. Som e música (tipos 5, 18, 20, 50, 51).
