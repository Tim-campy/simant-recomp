*Read in English: [README.md](README.md)*

# Chuva de comida (mod de exemplo com script Lua, API de mods 0.12 e 0.43)

A cada 500 ticks cai uma "chuva de comida" na superfície: 30 tentativas em células sorteadas pelo gerador do próprio script (não mexe nos sorteios do jogo), com uma mensagem na faixa ("It rained food! (N pieces)" com o jogo em inglês).

- Ligue com `--mods exemplos/mods` (liga também o mod de trapaças) ou copiando esta pasta para `mods/`.
- Script: `chuva_de_comida.lua`. API: `docs/pt-BR/api-mods.md`.
- Textos: `data/idiomas/en.json` (inglês, a reserva) e `data/idiomas/pt-BR.json` (uma segunda tradução de exemplo), chaves `mod.exemplo-lua.*`, lidos com `traduzir()` no idioma do jogo (`docs/pt-BR/i18n.md`).
