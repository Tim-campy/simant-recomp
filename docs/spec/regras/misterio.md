*Leia em português: [misterio.md](../../pt-BR/spec/regras/misterio.md)*

# Mystery (`_MysteryButton`)

Source: Ghidra pseudocode (`_MysteryButton` `1000:b0bc`, `_KillSomeAnts` `1000:9d5e`, `_AddRedAnts` `1030:6cfe`, `_AddBlackAnts` `1030:6c5a`, `_IsItYellow` `1020:96b6`), analysis of 2026-10-18. Everything is [code] except where marked.

## 1. Where it is called

- Function **8** of the configurable ribbon buttons (`_DoUserButton` `1000:04c4`). The default button table (15, 0, 1, 3, 7, 6, 5, 10) does not contain 8: it only appears if the player changes a button's function (Ctrl+click, icon menu; janelas.md §4.4).
- Object 0x112 (`mapMystery`) of the map window — hidden when there is a ribbon [hypothesis: see telas/mapa.md].
- No menu item and no key. `_CheatKeys` does not call it.
- Runs in message handling (`_DoMouse`/`_DoKeyDown` → `_DoEvent` → `_DoUserButton`), **outside the tick**, between two ticks.

## 2. The random draw and the effects

A single `_RRand(15)` (R generator, value `k` from 0 to 14); after that, only S draws, depending on the effect:

| k | Effect | S draws |
|---|---|---|
| 0, 1 | new red ants: `_AddRedAnts(n)` if `_ListIndexA` < 1000, `_FullCount`, again `_AddRedAnts(n)` if still < 1000, `_FullCount` | 2 per ant created |
| 2 | the same with `_AddBlackAnts(n)` | 2 per ant |
| 3 | `_KillSomeAnts(0)` twice (red) | those of `_DeadAntHere` |
| 4 | `_KillSomeAnts(1)` twice (black) | same |
| 5 | `_AddFood(150, 1)` | those of `_AddFood` |
| 6 | sound 0x20; every surface cell (x 0..127, y 0..63, column by column) with food (`_IsItFood`) gets `_MapA = SRand16()`; `_FoodA = 0` (the body of `_SubtractFood`, which has no caller) | 1 per cell with food |
| 7 | `_myBeginSoundList(0)`, empty on Windows: nothing | — |
| 8 | `_MakeNewHoleR(c)` for c = 0, 2, …, 62 | `S(31)` per call plus those of the digging |
| 9 | `_MakeNewHoleB(c)` same | same |
| 10 | `_AlwaysHealthy = 0`, `_HealthB = 0`, `_HealthR = 0` | — |
| 11 | toggles `_Invincible`; sound 2 when turning on, 1 when turning off | — |
| 12 | toggles `_RevSpider` | — |
| 13 | frames 10022 ("How many Glen Campbells does it take to eat a llama?") and 10024 ("#GCs = EPs - 2 (or, Elvis Minus Two)"), not forced (only with the Events option) | — |
| 14 | `_MeSMode = 8` (spider laser, inimigos.md) and sound 2 | — |

`n` = 1000 − `_ListIndexA` with `_CommandKey` (`8:85e2`) = 1, otherwise 32. The port has no command key (formiga-amarela.md already treats it as 0): `n` = 32.

`_FullCount` (`1020:07b4`) = `_CountAnts` + `_Feedback` (may open the dead-queen frames and give advice; mensagens.md).

## 3. `_AddBlackAnts(n)` / `_AddRedAnts(n)`

Walks the surface by column — black: x from 0 to 63; red: x from 127 **down** to 64 — and, in each column, y from 16 to 47. In each cell with `_MapA` < 0x50 and `_LifeA` = 0:

1. `r = SRand1(10)`: r from 0 to 3 → base 0x30 and mode 2; otherwise base 0x10 and mode 4.
2. `T = SRand8() + base` (+ 0x80 for red).
3. `_LifeA = T` and `_AddAntToAList(x, y, T, mode, 0)`.
4. `n` − 1; stops when `n` < 1 or `_ListIndexA` > 999 (the test comes after adding).

## 4. `_KillSomeAnts(color)`

1. `_CompactListA`, `_FullCount`.
2. From the last slot of list A to the first: with `color` = 1 (black), the slots without bit 7 in the type; with `color` = 0 (red), those with bit 7, except when `_IsItYellow(1, x, y)` (the yellow ant on plane 1, not being the spider, and the cell's life is its mark, 0xFE/0xFF). Each one: `_DeadAntHere(x, y, red)` and type = 0. Stops after the 51st death (counter > 50).
3. `_CompactListA`, `_FullCount`.

The victims are those at the end of the list; at most 102 in Mystery 3/4. Black ants do not have the yellow ant test.

## 5. In the port

`engine/src/rules/misterio.rs`: `botao_misterio(e, regras, tabelas)` (returns the `EfeitoMisterio`), `adicionar_formigas(e, cor, n)` (also used by the advice instead of the old `_AddBlackAnts`) and `matar_formigas(e, regras, cor)`. `_FullCount` = `Estado::contar_formigas` + `Regras::dar_conselhos`. Called by `Partida::botao_usuario(8)`; the joke frames come out as `Evento::Mensagem` and go through the `nao_forcadas` filter.

Tests: `rules/misterio.rs` (red ants from right to left with bit 7 and two S draws each; at most 51 deaths per call, from the end of the list, sparing the yellow ant; a single `_RRand(15)`).

**Not verified on the test bench:** no vector from the original with the Mystery (it requires changing a button's function to 8 and recording the action with `cenarios_tick.py`, like the actions in `tests/referencias/acoes/`). The tick vectors do not go through here.
