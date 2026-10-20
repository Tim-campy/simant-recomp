# Pending: mod API 0.43 (i18n) — text to integrate into docs/api-mods.md

This file holds the additions for `docs/api-mods.md` (and its Portuguese copy) that were kept apart
to avoid conflicts with the documentation translation. Integrate and then delete this file.

## 1. New line for "Compatibility history" (English)

| 0.43 | **Languages (i18n frame, docs/i18n.md)**. New data folder `data/idiomas/`: `idiomas.json` (index: languages by code, with `nome` and `contextos`, merged by key) and `<code>.json` (flat `"key": "text"` objects with `{name}` parameters, merged by key). A mod can add keys or a whole language (`data/idiomas/<code>.json` plus its entry in `data/idiomas/idiomas.json`). New Lua globals: `idioma()` (code of the game's language) and `traduzir(key, params)` (text of `key` in the game's language, `{name}` parameters from the table `params`, English fallback, then the key itself). The messages the host shows about scripts are now in English and come from `data/idiomas/` (`mods.*`, `lua.*`): e.g. the strip warning is "Mod NAME: script turned off by an error", refusals are logged as "ACTION refused: REASON". `API_VERSAO` becomes `"0.43"`. The example mods (`trapacas`, `exemplo-lua`, version 1.1) use `traduzir`, with English texts and a pt-BR example. | Compatible: scripts that do not call the new functions work as before. A script or tool that **matched the Portuguese texts** of the host (log lines such as "script desligado", "recusado", refusal reasons like "já está chovendo") must match the English texts (or, better, not depend on them). A mod that defines global functions named `idioma` or `traduzir` overrides the new ones in its own machine. A script that compares `API_VERSAO` with `"0.42"` must accept `"0.43"`. The ticks, the random draws and the save do not change. |

## 1b. Mesma linha em português (cópia pt-BR do histórico)

| 0.43 | **Idiomas (frame de i18n, docs/i18n.md)**. Nova pasta de dados `data/idiomas/`: `idiomas.json` (índice: idiomas por código, com `nome` e `contextos`, mesclado por chave) e `<código>.json` (objetos planos `"chave": "texto"` com parâmetros `{nome}`, mesclados por chave). Um mod pode acrescentar chaves ou um idioma inteiro (`data/idiomas/<código>.json` e a entrada em `data/idiomas/idiomas.json`). Novas globais Lua: `idioma()` (código do idioma do jogo) e `traduzir(chave, params)` (texto de `chave` no idioma do jogo, parâmetros `{nome}` da tabela `params`, reserva em inglês e, depois, a própria chave). As mensagens que o hospedeiro mostra sobre scripts passam a ser em inglês e vêm de `data/idiomas/` (`mods.*`, `lua.*`): por exemplo, o aviso da faixa é "Mod NOME: script turned off by an error" e as recusas vão para o registro como "AÇÃO refused: MOTIVO". `API_VERSAO` passa a `"0.43"`. Os mods de exemplo (`trapacas`, `exemplo-lua`, versão 1.1) usam `traduzir`, com textos em inglês e um exemplo em pt-BR. | Compatível: scripts que não chamam as funções novas funcionam como antes. Um script ou ferramenta que **comparava os textos em português** do hospedeiro (linhas do registro como "script desligado", "recusado", motivos como "já está chovendo") precisa comparar os textos em inglês (ou, melhor, não depender deles). Um mod que define funções globais chamadas `idioma` ou `traduzir` sobrescreve as novas na própria máquina. Um script que compare `API_VERSAO` com `"0.42"` precisa aceitar `"0.43"`. Os ticks, os sorteios e o save não mudam. |

## 2. New section (English) — suggested place: inside "Lua scripts", after the global functions

### Languages (API 0.43)

The texts a mod shows (menu titles and items, `mostrar_mensagem`, panel lines) should come from
language files instead of the script, so that the mod follows the game's language:

- `data/idiomas/<code>.json` inside the mod: a flat object `"key": "text"`, merged by key with the
  port's file of the same language. Use keys prefixed with `mod.<mod name>.` to avoid collisions.
  `{name}` is a parameter; `{{` and `}}` are literal braces. Bring at least `en.json` (English is
  the fallback language).
- `idioma()` → text: the code of the game's language (today always `"en"`; the index
  `data/idiomas/idiomas.json` lists the languages of the `jogo` context).
- `traduzir(key [, params])` → text: the text of `key` in the game's language; with `params` (a
  table with text keys), each `{name}` is replaced by `tostring(params.name)`. A key missing in the
  language uses the English text (with a warning in the log); missing in English too, the result is
  the key itself. A `params` table with non-text keys is an error.

Example (`exemplos/mods/exemplo-lua`):

```lua
local function t(key, params)
  return traduzir("mod.exemplo-lua." .. key, params)
end
mostrar_mensagem(t("choveu", { n = caidas }))   -- "It rained food! (12 pieces)"
```

with `data/idiomas/en.json` in the mod:

```json
{ "mod.exemplo-lua.choveu": "It rained food! ({n} pieces)" }
```

A mod can also add a language for the whole game: `data/idiomas/<code>.json` with the port's keys
translated, plus `data/idiomas/idiomas.json` with `{"idiomas": {"<code>": {"nome": "...",
"contextos": ["jogo"]}}}`. Details in `docs/i18n.md`.

## 2b. Mesma seção em português

### Idiomas (API 0.43)

Os textos que um mod mostra (título e itens de menu, `mostrar_mensagem`, linhas do painel) devem vir
de arquivos de idioma em vez do script, para que o mod acompanhe o idioma do jogo:

- `data/idiomas/<código>.json` dentro do mod: objeto plano `"chave": "texto"`, mesclado por chave
  com o arquivo do port do mesmo idioma. Use chaves com o prefixo `mod.<nome do mod>.` para evitar
  colisões. `{nome}` é um parâmetro; `{{` e `}}` são chaves literais. Traga pelo menos o `en.json`
  (o inglês é o idioma de reserva).
- `idioma()` → texto: o código do idioma do jogo (hoje sempre `"en"`; o índice
  `data/idiomas/idiomas.json` lista os idiomas do contexto `jogo`).
- `traduzir(chave [, params])` → texto: o texto de `chave` no idioma do jogo; com `params` (tabela
  com chaves de texto), cada `{nome}` é trocado por `tostring(params.nome)`. Chave ausente no
  idioma usa o texto em inglês (com um aviso no log); ausente também em inglês, o resultado é a
  própria chave. Uma tabela `params` com chaves que não são texto é erro.

Exemplo (`exemplos/mods/exemplo-lua`):

```lua
local function t(chave, params)
  return traduzir("mod.exemplo-lua." .. chave, params)
end
mostrar_mensagem(t("choveu", { n = caidas }))   -- "It rained food! (12 pieces)"
```

com `data/idiomas/en.json` no mod:

```json
{ "mod.exemplo-lua.choveu": "It rained food! ({n} pieces)" }
```

Um mod também pode acrescentar um idioma ao jogo inteiro: `data/idiomas/<código>.json` com as
chaves do port traduzidas, e `data/idiomas/idiomas.json` com `{"idiomas": {"<código>": {"nome":
"...", "contextos": ["jogo"]}}}`. Detalhes em `docs/pt-BR/i18n.md`.

## 3. Other notes for the integrator

- `docs/api-mods.md` "Structure" (the list of overridable data files) should mention
  `data/idiomas/*.json`.
- The root README section "Testing with the cheat mod" lists the menu items in Portuguese; the
  items are now English ("Start rain", "Cat shows up"...; full list in
  `exemplos/mods/trapacas/data/idiomas/en.json`).
- `exemplos/mods/trapacas/LEIAME.md` was left untouched (LEIAME* is being handled by the
  translation agents); the English and Portuguese READMEs of the example mods are
  `exemplos/mods/*/README.md` and `README.pt-BR.md`. LEIAME.md can be removed once merged.
