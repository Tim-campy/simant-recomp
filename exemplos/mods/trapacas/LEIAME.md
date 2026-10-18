# Mod de trapaças (exemplo, API de mods 0.17)

Menu para provocar na hora os eventos do jogo e testar o port: chuva, menino cortando a grama ou com inseticida, revoada, gato, cachorro, aranha, formiga-leão, luta, comida, saúde da amarela, troca de patch (inclusive os 38 da casa), rainhas guardadas para o botão PLACE, lições do tutorial, fim de jogo, Mystery, avanço rápido de ticks e um painel de depuração.

É um mod comum: só usa a API pública de scripts (`docs/api-mods.md`). O jogo base não tem nenhum código específico dele. Cada ação chama a rotina do jogo original que faz aquilo (o começo da chuva de `_DoSimYard`, `_XferPatch`, `_YellowDeath`, `_MysteryButton`...), então o jogo pode recusar como o original recusaria (por exemplo, trocar de patch só no Full Game).

## Como ligar

- `--mods exemplos/mods` (liga também o exemplo da chuva de comida), ou
- copie esta pasta para `mods/` (fica ligado sempre, até você tirar).

## Como usar

- **F12** abre o menu (para trocar, mude `TECLA` em `trapacas.lua`: uma tecla de função ou uma combinação com Ctrl/Alt).
- Setas + Enter ou clique escolhem; Esc ou clique fora fecham. A simulação para enquanto o menu está aberto.
- Toda trapaça usada aparece na faixa ("Trapaça: ...") e conta no painel. Recusas aparecem na faixa e no console.
- O jogo salvo não guarda que houve trapaça (o `.ANT` do original não tem campo para isso; o estado do mod não vai para o save).

A lista de itens e um roteiro para testar cada evento estão no `README.md` da raiz ("Testando com o mod de trapaças").

## Diferença para os códigos do original

O SimAnt original tem os seus próprios códigos de trapaça (`heal`, `just`, `qeen`...), digitados na partida; eles são parte do jogo base e funcionam com ou sem este mod (`docs/spec/regras/trapacas.md`).
