# API de mods

Versão da API: **0.2**. Até aqui só há sobrescrita de dados. Scripts Lua ainda não existem.

## Estrutura

Cada mod é um diretório dentro de `mods/`:

```
mods/
  meu-mod/
    mod.json
    objetos/<tipo>/<id>.bin     (opcional) substitui um objeto do banco original
    data/<arquivo>.json         (opcional) substitui um arquivo de data/ do port
```

`mod.json`:

```json
{ "nome": "meu-mod", "versao": "1.0", "dependencias": ["outro-mod"] }
```

## Regras

- **Objetos do banco:** `objetos/<tipo>/<id>.bin` substitui o objeto (tipo, id) de qualquer banco original. O conteúdo deve estar **descomprimido**, no formato descrito em `docs/spec/formatos/`. Exemplo: `objetos/2/2000.bin` troca a metade de cima do logotipo (bitmap, ver `bitmap.md`).
- **Arquivos de dados:** `data/<nome>` substitui o arquivo inteiro de mesmo nome em `data/`. Arquivos disponíveis no marco 1:
  - `paleta.json`: as 16 cores;
  - `telas.json`: sequência de abertura (janelas, bitmaps soltos, duração em ticks, gatilhos de fechamento);
  - `interface.json`: geometria da área cliente (faixa, borda das janelas, cores do sistema);
  - `jogo.json`: lido direto de `data/`, ainda não sobrescrevível por mod.
- **Arquivos soltos do jogo:** `arquivos/<nome>` substitui um arquivo da pasta do jogo lido pelo port (hoje, as fontes `font1`–`font4` e `FONTRES.FON`).
- **Ordem:** os mods são ordenados de modo que cada um venha depois das suas dependências; entre mods independentes, a ordem é alfabética. Se dois mods sobrescrevem a mesma coisa, vale o que vier por último.
- **Erros:** dependência ausente, nome duplicado ou ciclo de dependências impedem o jogo de iniciar, com mensagem de erro.
- Os arquivos originais do jogo nunca são modificados.

## Histórico de compatibilidade

| Versão | Mudança | Compatibilidade |
|---|---|---|
| 0.1 | Sobrescrita de objetos por (tipo, id) e de `data/*.json` por nome. | Inicial. |
| 0.2 | `arquivos/<nome>` para arquivos soltos do jogo; `interface.json` ganhou `fontes.lista`. | Compatível com 0.1; um `interface.json` de mod 0.1 precisa da chave `fontes`. |

## Planejado

- Sobrescrita parcial (por chave) dentro dos arquivos JSON, em vez de trocar o arquivo inteiro.
- Bitmaps de mod em PNG com conversão automática para a paleta.
- Scripts Lua em sandbox (`mlua`) com acesso apenas a uma API pública de regras, que será documentada aqui.
