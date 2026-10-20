*Read in English: [misterio.md](../../../spec/regras/misterio.md)*

# Mystery (`_MysteryButton`)

Fonte: pseudocódigo do Ghidra (`_MysteryButton` `1000:b0bc`, `_KillSomeAnts` `1000:9d5e`, `_AddRedAnts` `1030:6cfe`, `_AddBlackAnts` `1030:6c5a`, `_IsItYellow` `1020:96b6`), análise de 2026-10-18. Tudo [código] salvo onde marcado.

## 1. Onde é chamado

- Função **8** dos botões configuráveis da faixa (`_DoUserButton` `1000:04c4`). A tabela padrão dos botões (15, 0, 1, 3, 7, 6, 5, 10) não tem o 8: só aparece se o jogador trocar a função de um botão (Ctrl+clique, menu de ícones; janelas.md §4.4).
- Objeto 0x112 (`mapMystery`) da janela do mapa — oculto quando há faixa [hipótese: ver telas/mapa.md].
- Nenhum item de menu nem tecla. `_CheatKeys` não o chama.
- Roda no tratamento de mensagens (`_DoMouse`/`_DoKeyDown` → `_DoEvent` → `_DoUserButton`), **fora do tick**, entre dois ticks.

## 2. O sorteio e os efeitos

Um único `_RRand(15)` (gerador R, valor `k` de 0 a 14); depois, só sorteios de S, conforme o efeito:

| k | Efeito | Sorteios de S |
|---|---|---|
| 0, 1 | formigas vermelhas novas: `_AddRedAnts(n)` se `_ListIndexA` < 1000, `_FullCount`, de novo `_AddRedAnts(n)` se ainda < 1000, `_FullCount` | 2 por formiga criada |
| 2 | o mesmo com `_AddBlackAnts(n)` | 2 por formiga |
| 3 | `_KillSomeAnts(0)` duas vezes (vermelhas) | os de `_DeadAntHere` |
| 4 | `_KillSomeAnts(1)` duas vezes (pretas) | idem |
| 5 | `_AddFood(150, 1)` | os de `_AddFood` |
| 6 | som 0x20; cada célula da superfície (x 0..127, y 0..63, coluna a coluna) com comida (`_IsItFood`) recebe `_MapA = SRand16()`; `_FoodA = 0` (o corpo de `_SubtractFood`, que não tem chamador) | 1 por célula com comida |
| 7 | `_myBeginSoundList(0)`, vazia no Windows: nada | — |
| 8 | `_MakeNewHoleR(c)` para c = 0, 2, …, 62 | `S(31)` por chamada e os da escavação |
| 9 | `_MakeNewHoleB(c)` idem | idem |
| 10 | `_AlwaysHealthy = 0`, `_HealthB = 0`, `_HealthR = 0` | — |
| 11 | alterna `_Invincible`; som 2 ao ligar, 1 ao desligar | — |
| 12 | alterna `_RevSpider` | — |
| 13 | quadros 10022 ("How many Glen Campbells does it take to eat a llama?") e 10024 ("#GCs = EPs - 2 (or, Elvis Minus Two)"), não forçados (só com a opção Events) | — |
| 14 | `_MeSMode = 8` (laser da aranha, inimigos.md) e som 2 | — |

`n` = 1000 − `_ListIndexA` com `_CommandKey` (`8:85e2`) = 1, senão 32. O port não tem a tecla de comando (formiga-amarela.md já a trata como 0): `n` = 32.

`_FullCount` (`1020:07b4`) = `_CountAnts` + `_Feedback` (pode abrir os quadros da rainha morta e dar conselhos; mensagens.md).

## 3. `_AddBlackAnts(n)` / `_AddRedAnts(n)`

Percorre a superfície por coluna — pretas: x de 0 a 63; vermelhas: x de 127 **descendo** a 64 — e, em cada coluna, y de 16 a 47. Em cada célula com `_MapA` < 0x50 e `_LifeA` = 0:

1. `r = SRand1(10)`: r de 0 a 3 → base 0x30 e modo 2; senão base 0x10 e modo 4.
2. `T = SRand8() + base` (+ 0x80 nas vermelhas).
3. `_LifeA = T` e `_AddAntToAList(x, y, T, modo, 0)`.
4. `n` − 1; para com `n` < 1 ou `_ListIndexA` > 999 (o teste vem depois de acrescentar).

## 4. `_KillSomeAnts(cor)`

1. `_CompactListA`, `_FullCount`.
2. Da última vaga da lista A até a primeira: com `cor` = 1 (pretas), as vagas sem o bit 7 no tipo; com `cor` = 0 (vermelhas), as com o bit 7, menos quando `_IsItYellow(1, x, y)` (a amarela no plano 1, sem ser a aranha, e a vida da célula é a marca dela, 0xFE/0xFF). Cada uma: `_DeadAntHere(x, y, vermelha)` e tipo = 0. Para depois da 51ª morte (contador > 50).
3. `_CompactListA`, `_FullCount`.

As vítimas são as do fim da lista; no máximo 102 no Mystery 3/4. As pretas não têm o teste da amarela.

## 5. No port

`engine/src/rules/misterio.rs`: `botao_misterio(e, regras, tabelas)` (devolve o `EfeitoMisterio`), `adicionar_formigas(e, cor, n)` (usada também pelos conselhos no lugar do `_AddBlackAnts` antigo) e `matar_formigas(e, regras, cor)`. `_FullCount` = `Estado::contar_formigas` + `Regras::dar_conselhos`. Chamado por `Partida::botao_usuario(8)`; os quadros da piada saem como `Evento::Mensagem` e passam pelo filtro de `nao_forcadas`.

Testes: `rules/misterio.rs` (vermelhas da direita para a esquerda com o bit 7 e dois sorteios de S cada; no máximo 51 mortes por chamada, do fim da lista, poupando a amarela; um só `_RRand(15)`).

**Não verificado na bancada:** nenhum vetor do original com o Mystery (precisa trocar a função de um botão para 8 e gravar a ação com `cenarios_tick.py`, como as ações de `tests/referencias/acoes/`). Os vetores de tick não passam por aqui.
