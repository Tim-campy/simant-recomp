# SimAnt recomp

Reimplementação moderna do **SimAnt** (Maxis, 1991, versão Windows 3.x) em Rust + SDL2. O port lê os arquivos originais do usuário, que não são redistribuídos. Diretrizes em `CLAUDE.md`; especificação em `docs/spec/`.

## Requisitos

- Rust stable (toolchain MSVC no Windows).
- SDL2 2.30.9 para Visual C++ extraído em `vendor/SDL2-2.30.9/`. O `build.rs` liga com essa versão e copia o `SDL2.dll` para `target/<perfil>/`. Para baixar:
  `curl -L -o vendor/SDL2-devel-2.30.9-VC.zip https://github.com/libsdl-org/SDL/releases/download/release-2.30.9/SDL2-devel-2.30.9-VC.zip` e descompacte em `vendor/`.
- Arquivos do jogo instalado em `SimAnt-T/MAXIS/ANTWIN/`, ou indique a pasta com `--jogo`.

## Uso

```
cargo test                                         # parsers, regras e mods (testes com arquivos reais se presentes)
cargo run -p simant                                # abertura e "Select a Game": o botão escolhido começa o jogo, Esc sai
cargo run -p simant -- --exportar-telas saida/     # grava cada tela da sequência em PNG
cargo run -p simant -- --save tests/referencias/saves/QUICK2.ANT                        # joga a partida do save
cargo run -p simant -- --novo-jogo                                                       # Quick Game novo (semente do relógio)
cargo run -p simant -- --novo-jogo --semente 1234                                        # Quick Game reproduzível
cargo run -p simant -- --tipo completo --semente 1234                                    # Full Game (também tutorial, rapido, experimental)
cargo run -p simant -- --tipo completo --quintal 2 --exportar-tela-jogo q.png            # tela com a janela do quintal no modo 2
cargo run -p simant -- --save QUICK2.ANT --exportar-tela-jogo saida/jogo.png            # tela em PNG, sem janela
cargo run -p simant -- --save QUICK2.ANT --acoes superficie,linha_direita --exportar-tela-jogo s.png
cargo run -p simant -- --save QUICK2.ANT --ticks 50 --exportar-tela-jogo s.png          # 50 ticks sem janela
cargo run -p simant -- --save QUICK2.ANT --ticks 50 --salvar DEPOIS.ANT                  # grava o estado num .ANT
cargo run -p simant -- --exportar-som saida/som/                                         # efeitos e músicas em WAV, sem tocar
```

Opções: `--jogo DIR` (padrão: `SIMANT_DIR` ou `SimAnt-T/MAXIS/ANTWIN`), `--dados DIR` (padrão `data`), `--mods DIR` (padrão `mods`). Os testes procuram o jogo em `SIMANT_DIR` ou em `../SimAnt-T/MAXIS/ANTWIN`.

### Como jogar

`--save ARQ.ANT` ou `--novo-jogo` abre a tela de jogo (faixa e janela de edição) com a simulação rodando: você é a **formiga amarela**. Os avisos do jogo aparecem como no original: a mensagem da faixa ("Game Paused", conselhos, velocidade) e os quadros de mensagem e de fim de jogo, que param a simulação até um clique no quadro (ou Espaço; o quadro de mensagem também fecha sozinho em ~15 s). Ao fechar o fim de jogo (ou com **Ctrl+N**, File → New Game) aparece o "Select a Game". As músicas e os efeitos sonoros pedidos pelas regras tocam (os de menus e diálogos ainda não, ver `docs/spec/regras/som.md`). Ainda não há menus nem as janelas de mapa, castas e história. Partes das regras ainda faltam (ver `docs/spec/`).

- **Clique** numa célula do mapa: a amarela vai até lá. **Duplo clique**: age (numa formiga de outra cor, ataca; da mesma cor, pede comida; em comida/pedrinha, pega ou larga; no ninho, cava). Clique direito funciona igual. **Espaço/Insert** (e **Delete**) clicam onde está o cursor; duas vezes seguidas, duplo clique.
- Clicar no buraco do ninho leva a amarela para dentro; no ninho, clicar na linha de cima a leva para fora.
- **1** / **2** recrutam 5 / 10 operárias (**Ctrl+2**: todas), **3** / **4** liberam metade / todas, **0** liga o alarme, **X** (ou F9) troca de corpo (depois clique numa formiga), **Backspace** cancela a ordem, **Home** centraliza a vista na amarela.
- Velocidade (menu Speed do original): **Shift+0** pausa/despausa, **Shift+1..4** lenta / normal / rápida / ultra. **Ctrl+G** liga o AutoTrack (a vista segue a amarela; desligado por padrão, como no original). O título da janela mostra a velocidade e a pausa. Com o jogo pausado, a amarela ainda anda até o destino (como no original).
- Botões da segunda linha da faixa (como no original): ir até a amarela, pausa, **velocidade** (cicla e mostra "Game speed now …"), música e balões; os que abririam janelas ainda não fazem nada.
- **Ctrl+S** salva no arquivo da partida (o do `--save`); **Ctrl+A** salva com outro nome (o de `--salvar`, ou `SALVO01.ANT`, `SALVO02.ANT`… na pasta atual). O `.ANT` gravado é aceito pelo SimAnt original.
- **Segurar o botão** sobre a formiga amarela abre o menu dela (recrutar, liberar, trocar de corpo; Shift no "Recruit 10" recruta todas). **Ctrl+clique** num botão da segunda linha da faixa abre o menu de ícones para trocar a função do botão. **Ctrl+clique** num marcador (1–7) guarda a vista; clique volta a ela.
- Janelas: **Ctrl+B** Behavior Control, **Ctrl+C** Caste Control (arraste no triângulo para mudar as proporções; Auto, Manual e Set 1–3), **Ctrl+H** History (os botões ligam até 4 gráficos), **Ctrl+E** volta à janela de edição; os botões da faixa também abrem Behavior e Caste. Um save carregado começa pausado (Shift+0), como no original.
- As teclas da partida ficam em `data/partida.json` e os avisos em `data/mensagens.json` (sobrescrevíveis por mod).

Modos de jogo (ver `docs/spec/telas/abertura.md`, `quintal.md` e `experimental.md`):

- **Select a Game**: Tutorial, Quick Game, Full Game ou Experimental (Enter = Full Game, como no original). Load Game ainda não existe.
- **Quintal** (Yard View): botão *Yard* da faixa ou **Ctrl+M**; **Ctrl+Y / Ctrl+5 / Ctrl+6 / Ctrl+7** abrem nos modos quintal, casa por dentro, estratégico (cor das colônias) e gráfico (barras). Na faixa do quintal: *House*, *Strat*, *Graphic*, *Patch* (fecha), *Place* (o próximo clique num patch a até 3 do atual coloca uma rainha guardada), *Automatic/Manual*. Clique num patch escolhe; **duplo clique troca de patch** (só no Full Game e onde há colônia preta, como no original): o patch novo é gerado da semente dele, inclusive os 38 patches da casa.
- **Experimental**: o botão de ferramentas da faixa (ao lado do *?*) abre o menu: info, barreira (muro/pedrinha), cavar/encher, formiga preta/vermelha, comida (1/20), cheiro (5 tipos), inseticida (spray/jato) e "Start Game". Clique e arraste no mapa aplicam a ferramenta; com **Ctrl**, muro, cavar, comida e cheiro fazem o contrário. No Quick Game, o mesmo botão passa a partida para o Experimental.
- **Tutorial**: jogo tipo 0 (sem aranha); as lições saem como mensagens (`rules/conselhos.rs`).

Navegação na tela de jogo:

- **Ctrl + setas** rolam a janela de edição uma célula (como no original); as setas sozinhas e PageUp/PageDown também rolam (extensão do port). Clicar nas setas das barras de rolagem rola, repetindo enquanto o botão fica apertado.
- **Ctrl+8 / Ctrl+9 / Ctrl+0** ou os botões *Surface*, *Nest* (preto) e *Nest* (vermelho) da faixa trocam para a superfície, o ninho preto e o ninho vermelho. Esc fecha.
- As teclas e os botões ficam em `data/navegacao.json` (sobrescrevível por mod). Ações aceitas por `--acoes`: `linha_acima`, `linha_abaixo`, `linha_esquerda`, `linha_direita`, `pagina_acima`, `pagina_abaixo`, `pagina_esquerda`, `pagina_direita`, `superficie`, `ninho_preto`, `ninho_vermelho`.
- Som: `--sem-som` desliga; `--soundfont ARQ.sf2` (ou `SIMANT_SOUNDFONT`) usa uma SoundFont General MIDI para as músicas; sem ela, um sintetizador simples. Volumes, taxa e a tabela de músicas ficam em `data/som.json` (sobrescrevível por mod). `--exportar-som DIR` grava os 57 efeitos (WAV de 8 bits a 4096 Hz, como o original toca) e as músicas sintetizadas, sem abrir o áudio. Ver `docs/spec/regras/som.md`.
- Vídeo: a janela abre em **640×480, escala 1**, como o original. Para mudar, edite `data/video.json` ou use a linha de comando (que vence o arquivo):
  - `"escala"` / `--escala N`: pixels da tela por pixel do jogo (inteiro; `0` = a maior que cabe no monitor);
  - `"janela"` / `--janela LxA`: tamanho inicial da janela em pixels da tela (ex.: `--janela 1280x960`); `null` = 640×480 × escala;
  - `"redimensionavel"`: a janela pode ser redimensionada e maximizada pelo Windows;
  - `"escala_inteira"`: ao redimensionar, o jogo amplia só por múltiplos inteiros, centralizado com tarjas pretas (nítido); com `false`, ocupa o máximo mantendo a proporção.
  
  A janela é ciente de DPI e amplia por vizinho mais próximo (sem borrar com a escala de tela do Windows). Sem foco (outra janela ativa ou minimizada), a música para e os efeitos se calam, em todas as telas, como no original.
- Preferências: como o original faz com o `WIN.INI`, a partida guarda ao sair as opções (AutoTrack, Music, Sound Effects, Events, Messages, Silly) e as funções dos botões configuráveis da faixa em `preferencias.json`, na pasta de dados do usuário (no Windows, `%APPDATA%\SimAnt recomp\SimAnt\`), e as usa no próximo jogo novo (um `.ANT` carregado traz as suas opções). `--preferencias ARQ` usa outro arquivo; `--sem-preferencias` não lê nem grava.
- Janela, para testes sem interação: `--sair-apos-ms N` fecha sozinha e `--capturar-janela ARQ.png` grava o que a janela mostrava. `--ticks N` roda N ticks sem janela (com `--exportar-tela-jogo`, grava a tela depois) e escreve o `GameTime` antes e depois; `--avancar-ms MS` passa em seguida MS ms de tempo real na partida (para exportar o quadro de uma animação da amarela em andamento).

### Mods com script (Lua)

Um mod é uma pasta em `mods/` com um `mod.json`. Para ter script, liste os arquivos `.lua` em `"scripts"`:

```json
{ "nome": "meu-mod", "versao": "1.0", "dependencias": [], "scripts": ["principal.lua"] }
```

No `.lua`, defina como funções globais os ganchos que quiser (`ao_iniciar`, `antes_do_tick`, `depois_do_tick`, `ao_evento`, `ao_menu`) e use só a API de `docs/api-mods.md` (seção "Scripts Lua"):

```lua
function depois_do_tick(estado)
  if estado:tempo() % 500 == 0 then
    local x, y = aleatorio(0, 127), aleatorio(0, 63)   -- gerador do script, não o do jogo
    if estado:colocar_comida("superficie", x, y) then
      mostrar_mensagem("Caiu comida!")
    end
    registrar("comida da preta:", estado:colonia("preta").comida)
  end
end
```

O exemplo completo está em `exemplos/mods/exemplo-lua/` (chuva de comida a cada 500 ticks); ele fica fora de `mods/` para não mudar o jogo por padrão. Todos os mods da pasta de `--mods` (padrão `mods/`) ficam ativos: para experimentar o exemplo, use `--mods exemplos/mods` ou copie a pasta dele para `mods/`. O script roda em sandbox (sem `io`, `os`, `require`, `load`), com limite de instruções, tempo e memória; se der erro, é desligado, o erro sai no console (`[script] ...`) e a faixa avisa, mas o jogo continua.

### Códigos de trapaça do original

O SimAnt tem trapaças embutidas: durante a partida, digite um código de 4 letras (minúsculas sem Shift, maiúsculas com Shift; Enter apaga o que foi digitado). Os mais úteis: `heal` / `HEAL` (saúde da colônia preta / vermelha em 100), `just` / `oops` (+10 rainhas guardadas pretas / vermelhas), `qeen` / `QEEN` (rainha nova no ninho), `eggs` / `EGGS` (16 ovos em volta da amarela), `rand` (amarela com saúde 100), `hole` (64 buracos pretos), `jeff` / `joel` (colônias no quintal), `will` (as vermelhas sempre perdem a luta), `jenn` (amarela nunca com fome), `susi` (colônia preta sempre saudável). Lista completa e o que cada um faz: `docs/spec/regras/trapacas.md` (tabela em `data/codigos_trapaca.json`).

### Testando com o mod de trapaças

Para provocar na hora os eventos raros (chuva, revoada, menino, gato, fim de jogo...), há um **mod de trapaças** em `exemplos/mods/trapacas/` (extensão do port, não existe no original). Ele fica fora de `mods/`, então não muda o jogo por padrão. Para ligar:

```
cargo run --release -p simant -- --save tests/referencias/saves/QUICK2.ANT --mods exemplos/mods
```

(`--mods exemplos/mods` liga também o exemplo da chuva de comida; para só as trapaças, copie `exemplos/mods/trapacas` para `mods/`.)

Na partida, **F12** abre o menu (setas + Enter, ou clique; Esc ou clique fora fecha; a simulação para enquanto ele está aberto). Cada trapaça usada aparece na faixa como "Trapaça: ..."; se o jogo recusar (por exemplo, trocar de patch fora do Full Game), o motivo aparece na faixa e no console (`[script] ...`). O jogo salvo não registra o uso de trapaças. Itens:

| Grupo | Itens |
|---|---|
| Chuva | Começar chuva · Parar chuva |
| Menino e casa | Menino: cortar a grama neste patch · Menino: inseticida dentro de casa · Ir para o patch da cozinha (0, 12) · Ir para o próximo patch da casa · Ir para o patch do quintal ao lado |
| Colônias | Guardar 3 rainhas pretas (botão PLACE) · Revoada vermelha (25 reprodutores) · Revoada preta (25 reprodutores) |
| Bichos | Gato aparecer · Cachorro latir · Aranha aparecer · Formiga-leão perto da amarela |
| Amarela | Luta: soldado vermelho junto da amarela · Comida em volta da amarela · Curar a amarela · Matar a amarela |
| Tempo | Avançar 100 / 500 / 1000 ticks · Pular até o próximo evento (avança até a próxima mensagem, música ou fim de jogo) |
| Jogo | Tutorial: avançar lição · Fim de jogo: vitória · Fim de jogo: derrota · Mystery · Painel de depuração (GameTime, sementes S/R, patch, populações, rainhas guardadas, chuva e `_WaterTable`, `_FlyAway`, menino, lição) |

Roteiro para as pendências de teste:

- **Troca de patch e patches da casa**: Full Game (`--novo-jogo --tipo completo`, ou um save do Full Game) com a colônia já crescida; F12 → *Ir para o patch da cozinha*, depois *Ir para o próximo patch da casa* várias vezes (percorre os 38 patches da casa) e *Ir para o patch do quintal ao lado*. Também dá para usar a Yard View (Ctrl+Y) e dar duplo clique: o mod só garante uma colônia preta no destino.
- **Botão PLACE**: Full Game; F12 → *Guardar 3 rainhas pretas*; Ctrl+Y, botão *Place* e um clique num patch a até 3 do atual.
- **Chuva**: F12 → *Começar chuva*; Ctrl+Y para ver as gotas no quintal; entre num ninho (Ctrl+9) e use *Avançar 500 ticks*: a água sobe (o `_WaterTable` cai no painel). O som da chuva toca de vez em quando.
- **Menino cortando a grama**: F12 → *Menino: cortar a grama neste patch*; Ctrl+Y mostra o menino com o cortador; na superfície, o pé e o cortador passam no patch.
- **Menino com inseticida**: *Ir para o patch da cozinha (0, 12)* (Full Game) e *Menino: inseticida dentro de casa*; a amarela na superfície corre risco.
- **Revoada**: *Revoada vermelha* (ou preta) e *Avançar 100 ticks* algumas vezes: os machos alados saem, decolam e somem; no quintal (Ctrl+Y) aparece o enxame.
- **Gato**: *Gato aparecer* e Ctrl+Y (o gato entra pelo canto e o som toca na vista do quintal).
- **Cheiro no Experimental**: o menu de ferramentas do Experimental já põe cheiro; com *Painel de depuração* ligado dá para acompanhar o tempo. Os botões de cheiro da faixa mostram a camada.
- **Tutorial**: *Tutorial* no "Select a Game" (Ctrl+N); F12 → *Tutorial: avançar lição* mostra a próxima lição (o painel mostra o número).
- **Sons**: *Cachorro latir* (latido), *Gato aparecer* com a vista do quintal (gato), *Luta: soldado vermelho junto da amarela* e duplo clique nele (luta), *Começar chuva* (chuva).
- **Fim de jogo**: *Fim de jogo: vitória* ou *derrota* (quadro final e "Select a Game").
- **Mod Lua de exemplo (500 ticks)**: com `--mods exemplos/mods` ele já está ligado; *Avançar 500 ticks* e a faixa mostra "Choveu comida!".

O menu e o painel usam só a API pública de scripts (`docs/api-mods.md`, API 0.17): qualquer mod pode ter o seu menu (`definir_menu`) e painel (`definir_painel`). Para exportar uma tela com o menu aberto sem janela: `--menu mod@F12 --exportar-tela-jogo tela.png` (`mod@F12#N` escolhe o item N).

## Organização

| Pasta | Conteúdo |
|---|---|
| `engine/` | crate sem janela: `formats` (banco NDX/DAT, LZSS, bitmap), `recursos` (original + mods), `core` (sequência de telas, tela de jogo, navegação), `paleta`, `scripts` (scripts Lua dos mods em sandbox) |
| `simant/` | executável SDL2 (render e entrada) |
| `data/` | dados do port (paleta, telas, bancos usados): é o que os mods sobrescrevem |
| `mods/` | mods instalados (ver `docs/api-mods.md`) |
| `tools/` | inspetores em Python dos formatos originais; `capturar_original.ps1` e `comparar_captura.py` para comparar o port com o original |
| `referencia/` | configuração do DOSBox-X para rodar o original |
| `docs/` | especificação, decisões, glossário, divergências, API de mods |
| `re_trabalho/` | material de engenharia reversa, fora do controle de versão e fora do port |
