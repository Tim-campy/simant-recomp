# API de mods

Versão da API: **0.11**. Até aqui só há sobrescrita de dados. Scripts Lua ainda não existem.

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
  - **Regras do tick** (API 0.5): carregados por `RegrasOriginais::carregar` e usados em cada tick. Cada arquivo traz tabelas do executável e as constantes da parte (origem de cada valor em `_origem`/`_comentario` dentro do próprio arquivo):
    - `quintal.json`: menino, cachorro, gato, pássaro, chuva, colônias do quintal e, no bloco `conselhos`, as lições do tutorial (quintal.md, conselhos.md);
    - `inimigos.json`: formiga-leão, aranha, lagarta e bicho-de-conta (inimigos.md);
    - `superficie.json`: formigas da superfície, estratégia, castas e líder vermelha (superficie.md);
    - `feromonios.json`: direções e `_TurnTab` das escolhas pelo cheiro e a manutenção dos mapas de cheiro (feromonios.md);
    - `ninho.json`: formigas dos ninhos, ovos e rainha (ninho.md, ciclo-de-vida.md);
    - `amarela.json`: a formiga amarela; vale também quando outra parte a mata, a alimenta ou luta com ela (formiga-amarela.md);
    - `geracao.json`: geração do mundo e as direções de `_AddFood` (geracao-mundo.md; o seno vem do objeto 9/1000 do banco).
    
    Tabelas que o original tem uma vez só (`dx8`/`dy8`, `turn_tab`, `classe_luta`, a matriz de luta, `borda_buraco`, `modos_operaria`/`modos_soldado`/`modo_por_grupo`) aparecem em mais de um arquivo; cada cópia vale para a parte do seu arquivo (por exemplo, `modos_operaria` de `ninho.json` decide o modo das operárias nos ninhos e o de `superficie.json`, na superfície). Um mod que queira mudar a regra do jogo inteiro precisa mudar todas as cópias. Os arquivos são validados ao carregar (direções em −1..1, direções de giro em 0..7, classes de luta em 0..3, `regras.gato_chance` potência de 2...): um valor inválido impede o jogo de iniciar, com a mensagem de erro.
  - `partida.json` (API 0.4): laço de jogo ao vivo. `relogio` (intervalo das mensagens de temporizador com e sem foco, voltas do laço por velocidade, máximo de mensagens por quadro, redesenho do mapa por velocidade e sem foco), `teclas` (aceleradores: nome da tecla no SDL, `ctrl`/`shift`/`alt`, `acao` entre `pausa`, `velocidade_lenta`, `velocidade_normal`, `velocidade_rapida`, `velocidade_ultra`, `auto_track`, e desde a 0.7 `salvar` e `salvar_como`), `teclas_amarela` (nome da tecla no SDL → código de tecla virtual do Windows passado a `_YellowCommandKey`), `teclas_clique` (tecla → `esquerdo`/`direito`, imita o mouse) e `duplo_clique_teclado_ms`; desde a 0.7, `espera_clique_simples_tickcount` (espera do clique simples) e `botoes_usuario` (`objetos`: nomes dos botões configuráveis da faixa; `funcoes`: número da função de `_DoUserButton` de cada um). Ver `docs/spec/telas/jogo.md`, "Simulação ao vivo".
  - **Modos de jogo** (API 0.9), ver `docs/spec/telas/abertura.md`, `quintal.md` e `experimental.md`:
    - `modos.json`: "Select a Game" — `tela` (nome da tela de `telas.json` onde se escolhe), `janela` (2), `botoes` (objeto → `tutorial`, `rapido`, `completo`, `experimental`, `carregar`, `cancelar`), `teclas` (pares [tecla, escolha]; Enter = Full Game como no original) e `novo_jogo` (tecla do File → New Game, Ctrl+N);
    - `quintal_mapa.json`: janela do quintal (25) e faixa 35 — moldura e área cliente medidas, títulos, geometria dos patches (`patch`, `poligono_cursor`, `poligono_colonia`), bitmaps de cada um dos 4 modos, rodapé, cores das colônias e das barras, estilo do cursor por modo, cor transparente e sprites (base e deslocamento por quadro do menino, cachorro, gato e pássaro; cortador, placa, chuva), botões da faixa 35 → ação (`fechar`, `alternar_casa`, `modo_estrategico`, `modo_grafico`, `colocar_rainha`, `rainhas_automatico`, `rainhas_manual`), `botao_do_modo`, `botoes_rainhas`, `teclas` (`abrir`, `modo_quintal`, `modo_casa`, `modo_estrategico`, `modo_grafico`) e `dados_faixa`. Gerado por `tools/extrair_quintal_mapa.py`;
    - `menu_ferramentas.json`: botão e menu de ferramentas do Experimental (janela 9, submenus 12–17 por ferramenta, bitmaps do botão, posição do menu);
    - `experimental.json`: regras das ferramentas do Experimental (tabela de conexão dos muros, subopções iniciais, constantes, sons, música, avisos e listas do exame), validado ao carregar (nenhum sorteio com n = 0);
    - `casa.json`: patches da casa (`_MakeHousePatch`): `formas` (largura, altura, transparente, celulas), `rotinas` (nome → lista de operações) e `patches` (38 listas, `n = px*16+py`). Operações: `preencher {x:[a,b], y:[c,d], v}`, `celula {x,y,v}`, `sortear {x,y,chance,base,variantes}`, `moldura {...}`, `trocar {...}`, `forma {nome,x,y}`, `ladrilhos {...}`, `queda {valor}`, `rotina {nome}`. Validado ao carregar (nada fora de 128×64, sem S(0), sem rotina em laço). Gerado por `tools/extrair_casa.py`.
  - `mensagens.json` (API 0.7): avisos ao jogador. `ms_por_tickcount`; `quadro` (janela, fonte, folgas e recuos da geometria, prazo, tecla que fecha, `nao_forcadas`: textos que só aparecem com Options → Events, `imagens`: texto → bitmap); `faixa` (objeto, fonte, lista e índices dos textos de pausa, durações, `velocidade`: os quatro textos do botão de velocidade); `fim_de_jogo` (janela, objetos, listas de tipo e avaliação, faixas de pontos, músicas, `ajuste`); `pontuacao` (pesos e escalas de `_CalcScore`). Os textos dos quadros vêm do banco (SHARED tipo 4) e podem ser trocados por `objetos/4/<id>.bin`. Ver `docs/spec/telas/mensagens.md`.
  - `janelas.json` (API 0.10): janelas Caste e Behavior Control (`castas`, `tarefas`: objetos do triângulo, níveis, botões, cores das barras) e History (`historico`: botões, séries e parceiras por símbolo do save, cores), moldura com título, botão do triângulo. `menus.json` (API 0.10): menus da amarela (`formiga`, `rainha`, `aranha`: janela, itens, códigos de `_YellowCommand`) e de ícones, deslocamentos. Ver `docs/spec/telas/janelas.md`.
  - `dialogos.json` (API 0.11): diálogos do Windows da partida (caixa de mensagem do Load e do "Saved correctly", diálogo de arquivo Load/Save As): fontes, cores, geometria e textos. Ver `docs/spec/telas/arquivos.md`.
  - `som.json` (API 0.6): som e música. `musicas` (id do cabeçalho tipo 18 → nome do `.mid` sem extensão, a tabela do original), `pasta_musicas`, `fonte_musica` (`arquivos` = `.MID` pela tabela, como o MCI; `banco` = objeto tipo 20 com a transposição do cabeçalho, como o modo Sound Blaster), `exigir_cabecalho`, `canais_midi`, `canal_percussao`, `taxa_efeitos` (Hz dos efeitos tipo 5), `taxa_saida`, `volume_musica`, `volume_efeitos` (0–1), `soundfont` (caminho de uma `.sf2` ou `null`), `musica_abertura`, `musica_jogo_novo`, `cauda_exportacao_s`. Ver `docs/spec/regras/som.md`.
- **Arquivos soltos do jogo:** `arquivos/<nome>` substitui um arquivo da pasta do jogo lido pelo port (as fontes `font1`–`font4` e `FONTRES.FON`; a partir da API 0.6, também caminhos com subpasta, como `arquivos/sound/gamethme.mid`, sem diferenciar maiúsculas).
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
| 0.4 | Novo arquivo sobrescrevível `data/partida.json` (relógio e teclas do laço de jogo). Os comandos do jogador passam a usar o `amarela.json` com mods (o tick ainda usa a cópia embutida). | Compatível com 0.3: só acrescenta um arquivo. Como o arquivo é trocado inteiro, um `partida.json` de mod precisa de todas as chaves. |
| 0.5 | Os arquivos das regras do tick (`quintal.json`, `inimigos.json`, `superficie.json`, `feromonios.json`, `ninho.json`, `amarela.json`, além de `geracao.json`) passam a chegar à simulação quando um mod os sobrescreve (antes o tick usava as cópias embutidas no executável). Novas validações ao carregar (`regras.gato_chance` potência de 2; direções e giros em `inimigos.json`). | Compatível com 0.4: os formatos não mudaram. Um mod anterior que já trazia um desses arquivos passa a ter efeito no tick; se o arquivo tiver valor inválido, o jogo não inicia (antes era ignorado). |
| 0.6 | Novo arquivo sobrescrevível `data/som.json`; `arquivos/<subpasta>/<nome>` (músicas em `arquivos/sound/`); efeitos sonoros substituíveis em `objetos/5/<id>.bin` (formato DAC de som.md). | Compatível com 0.5: só acrescenta. Como o arquivo é trocado inteiro, um `som.json` de mod precisa de todas as chaves (exceto `soundfont`). |
| 0.7 | Novo arquivo sobrescrevível `data/mensagens.json`; `partida.json` ganhou as chaves `espera_clique_simples_tickcount` e `botoes_usuario` e as ações `salvar`/`salvar_como`. | Arquivo novo: compatível. Um `partida.json` de mod anterior **precisa das duas chaves novas** (o arquivo é trocado inteiro); sem elas o jogo não inicia. |
| 0.8 | Seção opcional `sons` em `amarela.json`, `quintal.json`, `ninho.json`, `inimigos.json` e `superficie.json` (ids dos efeitos sonoros tipo 5 pedidos pelas regras; ver `docs/spec/regras/som.md`) e chave opcional `tema_ninho` em `som.json` (`musicas`, `intervalo_ms`). | Compatível com 0.7: as chaves são opcionais; num arquivo de mod sem elas, aquela parte fica sem efeitos sonoros (ou sem tema do ninho). |
| 0.9 | Novos arquivos sobrescrevíveis `data/modos.json`, `data/quintal_mapa.json`, `data/menu_ferramentas.json`, `data/experimental.json` e `data/casa.json` (escolha do jogo, janela do quintal e troca de patch, ferramentas do Experimental, patches da casa). | Compatível com 0.8: só acrescenta arquivos; um mod que não os traz usa os do port. Como cada arquivo é trocado inteiro, um mod que traga um deles precisa de todas as chaves. |
| 0.10 | Novos arquivos sobrescrevíveis `data/janelas.json` (Caste, Behavior e History) e `data/menus.json` (menus de segurar o botão e de ícones). `partida.json` ganhou `marcadores`, `botoes_usuario.bitmap_selecionado`/`bitmap_normal` e as ações `janela_edicao`, `janela_comportamento`, `janela_castas`, `janela_historico`; `mensagens.json` ganhou `faixa.marcador_posto`/`marcador_ausente`; `amarela.json` (`mensagens.faixa_*`) e `superficie.json` (`regras.mensagem_ataque_vermelho`) ganharam as mensagens da faixa. | Arquivos novos: compatível. Um `partida.json`, `mensagens.json`, `amarela.json` ou `superficie.json` de mod anterior precisa das chaves novas (o arquivo é trocado inteiro); sem elas o jogo não inicia. |
| 0.11 | Novo arquivo sobrescrevível `data/dialogos.json`. `partida.json` ganhou a ação `carregar` (Ctrl+L). `interface.json`: na lista de fontes, `ARQUIVO.FON#Nome` aceita `@altura` (escolhe o tamanho) e `+negrito` (negrito simulado); índice 7 = MS Sans Serif 8 em negrito, do `SSERIFE.FON` do Windows do usuário. | Arquivo novo: compatível. Um `partida.json` de mod anterior continua válido (sem Ctrl+L). |

## Planejado

- Sobrescrita parcial (por chave) dentro dos arquivos JSON, em vez de trocar o arquivo inteiro.
- Bitmaps de mod em PNG com conversão automática para a paleta.
- Scripts Lua em sandbox (`mlua`) com acesso apenas a uma API pública de regras, que será documentada aqui.
