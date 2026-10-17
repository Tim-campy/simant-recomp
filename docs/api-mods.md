# API de mods

Versão da API: **0.5**. Até aqui só há sobrescrita de dados. Scripts Lua ainda não existem.

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
  - `tela_jogo.json`: geometria da tela de jogo (janela de edição, moldura, botão da faixa de cada plano);
  - `navegacao.json` (API 0.3): teclas (`teclas`: nome da tecla no SDL, `ctrl`/`shift`/`alt`, `acao`), botões da faixa (`botoes_faixa`: nome do objeto → ação), repetição das setas da barra (`repeticao`) e vista inicial ao abrir um save (`posicao_inicial`: `ao_carregar` ou `salva`). Ações: `linha_acima`, `linha_abaixo`, `linha_esquerda`, `linha_direita`, `pagina_acima`, `pagina_abaixo`, `pagina_esquerda`, `pagina_direita`, `superficie`, `ninho_preto`, `ninho_vermelho`;
  - `jogo.json`: lido direto de `data/`, ainda não sobrescrevível por mod;
  - `simulacao.json`: velocidades do jogo (`atrasos_velocidade`) e valores iniciais de um jogo novo (triângulos de casta e tarefa, opções, `_NextFeed`, `_Barrier`...). Ver `docs/spec/regras/simulacao-visao-geral.md`, "Implementação no port".
  - `partida.json` (API 0.4): laço de jogo ao vivo. `relogio` (intervalo das mensagens de temporizador com e sem foco, voltas do laço por velocidade, máximo de mensagens por quadro, redesenho do mapa por velocidade e sem foco), `teclas` (aceleradores: nome da tecla no SDL, `ctrl`/`shift`/`alt`, `acao` entre `pausa`, `velocidade_lenta`, `velocidade_normal`, `velocidade_rapida`, `velocidade_ultra`, `auto_track`), `teclas_amarela` (nome da tecla no SDL → código de tecla virtual do Windows passado a `_YellowCommandKey`), `teclas_clique` (tecla → `esquerdo`/`direito`, imita o mouse) e `duplo_clique_teclado_ms`. Ver `docs/spec/telas/jogo.md`, "Simulação ao vivo".
  - `som.json` (API 0.5): som e música. `musicas` (id do cabeçalho tipo 18 → nome do `.mid` sem extensão, a tabela do original), `pasta_musicas`, `fonte_musica` (`arquivos` = `.MID` pela tabela, como o MCI; `banco` = objeto tipo 20 com a transposição do cabeçalho, como o modo Sound Blaster), `exigir_cabecalho`, `canais_midi`, `canal_percussao`, `taxa_efeitos` (Hz dos efeitos tipo 5), `taxa_saida`, `volume_musica`, `volume_efeitos` (0–1), `soundfont` (caminho de uma `.sf2` ou `null`), `musica_abertura`, `musica_jogo_novo`, `cauda_exportacao_s`. Ver `docs/spec/regras/som.md`.
- **Arquivos soltos do jogo:** `arquivos/<nome>` substitui um arquivo da pasta do jogo lido pelo port (as fontes `font1`–`font4` e `FONTRES.FON`; a partir da API 0.5, também caminhos com subpasta, como `arquivos/sound/gamethme.mid`, sem diferenciar maiúsculas).
- **Ordem:** os mods são ordenados de modo que cada um venha depois das suas dependências; entre mods independentes, a ordem é alfabética. Se dois mods sobrescrevem a mesma coisa, vale o que vier por último.
- **Erros:** dependência ausente, nome duplicado ou ciclo de dependências impedem o jogo de iniciar, com mensagem de erro.
- Os arquivos originais do jogo nunca são modificados.

## Histórico de compatibilidade

| Versão | Mudança | Compatibilidade |
|---|---|---|
| 0.1 | Sobrescrita de objetos por (tipo, id) e de `data/*.json` por nome. | Inicial. |
| 0.2 | `arquivos/<nome>` para arquivos soltos do jogo; `interface.json` ganhou `fontes.lista`. | Compatível com 0.1; um `interface.json` de mod 0.1 precisa da chave `fontes`. |
| 0.3 | `navegacao.json` (teclas, botões da faixa e vista inicial da tela de jogo); `tela_jogo.json` listado como sobrescrevível. | Compatível com 0.2: arquivos novos; um mod que não os traz usa os do port. Como o arquivo é trocado inteiro, um `navegacao.json` de mod precisa de todas as chaves. |
| 0.3 (acréscimo) | Novo arquivo sobrescrevível `data/simulacao.json`. | Compatível: só acrescenta um arquivo. |
| 0.5 | Novo arquivo sobrescrevível `data/som.json`; `arquivos/<subpasta>/<nome>` (músicas em `arquivos/sound/`); efeitos sonoros substituíveis em `objetos/5/<id>.bin` (formato DAC de som.md). | Compatível com 0.4: só acrescenta. Como o arquivo é trocado inteiro, um `som.json` de mod precisa de todas as chaves (exceto `soundfont`). |
| 0.4 | Novo arquivo sobrescrevível `data/partida.json` (relógio e teclas do laço de jogo). Os comandos do jogador passam a usar o `amarela.json` com mods (o tick ainda usa a cópia embutida). | Compatível com 0.3: só acrescenta um arquivo. Como o arquivo é trocado inteiro, um `partida.json` de mod precisa de todas as chaves. |

## Planejado

- Sobrescrita parcial (por chave) dentro dos arquivos JSON, em vez de trocar o arquivo inteiro.
- Bitmaps de mod em PNG com conversão automática para a paleta.
- Scripts Lua em sandbox (`mlua`) com acesso apenas a uma API pública de regras, que será documentada aqui.
