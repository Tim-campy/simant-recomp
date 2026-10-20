*Read in English: [CHANGELOG.md](CHANGELOG.md)*

# Histórico de versões

Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/); as versões seguem o
[Versionamento Semântico](https://semver.org/lang/pt-BR/). A versão do port e a versão da **API de mods**
(`docs/pt-BR/api-mods.md`) são numeradas separadamente.

> **Rascunho** para revisão (ver `docs/pt-BR/lancamento.md`). A data e o número só ficam valendo quando a tag
> `v1.0.0` for criada; até lá os `Cargo.toml` continuam em `0.1.0`.

## [1.0.0] — não lançado

Primeira versão para quem joga. Reimplementação do **SimAnt: The Electronic Ant Colony** (Maxis, 1991,
versão Windows 3.x) em Rust + SDL2, que lê os arquivos do jogo original do próprio usuário. Nenhum
arquivo do jogo é distribuído; os textos do jogo também são lidos da instalação do usuário.

### O jogo

- **Os quatro modos do original:** Tutorial (lições 1 a 57), Quick Game, Full Game (o quintal e a casa,
  com troca de patch) e Experimental (ferramentas de cavar, encher, comida, pedras, formigas, cheiro,
  inseticida e o menu de ferramentas de segurar).
- **Simulação igual à do original, tick a tick:** 221 vetores gravados do SimAnt rodando, 2 sequências
  longas (200 e 600 ticks), 21 ações da interface, o Load do tutorial e 20 casos de regra conferidos
  bit a bit (307/307 blocos do estado); a geração do Quick Game, dos 12 patches da casa e do jogo novo de
  Full, Experimental e Tutorial também bate antes do primeiro tick.
- **Formiga amarela:** andar, comer, cavar, recrutar e liberar operárias, alarme, trofalaxia, troca de
  corpo, lutas e as dez mortes do original; inimigos (formiga-leão, aranha, inclusive a aranha do
  jogador com AutoTrack, lagarta, bicho-de-conta), quintal (menino, cortador de grama, inseticida,
  gato, cachorro, pássaro, chuva, revoadas) e fim de jogo.
- **Telas e janelas do original:** abertura e "Select a Game", janela de edição, mapa, mini mapa,
  quintal (modos quintal, casa, estratégico e gráfico), Caste Control, Behavior Control, History, Info
  (51 cartões), Examine, balões de fala ("Silly" e os 23 do menino), quadros da janela 26, barra de
  menus do Windows 3.1 com About, Status, Quit e Load Game, menu de sistema das janelas, faixa com
  botões configuráveis, marcadores e barras de saúde. Comparadas pixel a pixel com capturas do original
  (93–100%).
- **Salvar e carregar** no formato `.ANT` do original (o arquivo gravado pelo port abre no SimAnt).
- **Ajuda:** visor do `SIMANT.HLP` (WinHelp 3.0) e ajuda contextual (Shift+F1, botões *?*).
- **Códigos de trapaça** do original (`heal`, `just`, `qeen`, `eggs`...).
- **Som e música:** os efeitos e as 28 músicas do original. As músicas tocam por padrão num
  sintetizador simples; opcionais: sintetizador FM próprio no estilo do OPL da Sound Blaster
  (`--sintetizador fm`), com o banco de timbres ajustado contra gravações do original, e SoundFont
  General MIDI (`--soundfont`). Os efeitos soam como na Sound
  Blaster 16 do original (5000 Hz, nos blocos do driver), medidos numa partida gravada.
- **Ritmo do original:** relógio de 55 ms do Windows 3.1, velocidades lenta/normal medidas no original
  (3 e 6 ticks/s), rápida e ultra.

### Para os sistemas de hoje

- Janela nítida: ciente de DPI, ampliação por múltiplos inteiros, redimensionável.
- Tela virtual de 640×480 (padrão), 800×600 ou 1024×768, com o layout que o original usa em cada uma.
- Barra de título do Windows 3.1, ligada por padrão (`--sem-moldura` desliga), que substitui a moldura
  do sistema: janela sem borda, arrastada pelo título, maximizar/restaurar/minimizar e menu de sistema
  na janela de verdade (também com um diálogo aberto e no "Select a Game"), moldura grossa de 4 pixels
  rente às bordas com a janela restaurada, Alt+F4 sempre com a pergunta do Quit
  (`--moldura-do-sistema` mantém a borda do sistema). Os testes e exportações ficam sem ela. Escolha
  do usuário. Na ajuda, fechar é Ctrl+F4 (o Alt+F4 fica só com o jogo).
- Velocidades rápida e ultra a 2,5× e 5× a normal (escolha do usuário; `--velocidades original` volta ao
  ritmo do 486 da bancada) e simulação na lenta enquanto a janela está sem foco (`--sem-foco original`
  mantém o ritmo); o tick, o save e o menu Speed não mudam. Caixas no launcher.
- Sintetizador simples como padrão das músicas (escolha do usuário; o FM continua em `--sintetizador fm`).
- Preferências guardadas entre partidas na pasta do usuário (no lugar do `[SimAnt]` do `WIN.INI`).
- Teclas e opções em arquivos de dados (`data/*.json`).
- Linha de comando para rodar sem janela (`--ticks`, `--salvar`, exportação de telas e sons).
- **Launcher:** o `simant` aberto sem opções mostra uma tela de opções de inicialização: pasta da
  instalação do SimAnt (seletor do sistema, conferida e lembrada), início (abertura, jogo novo de cada
  tipo ou save), vídeo, perfil de driver, som e sintetizador, mods (pasta e cada mod ligado ou não),
  preferências e a linha de comando equivalente, editável. As escolhas ficam em `launcher.json` e valem
  também para o `simant` rodado com opções (que vencem).
- No Windows, o `simant.exe` não abre console: mensagens em `simant.log` na pasta do usuário, erros
  fatais numa caixa de mensagem, `simant-console.bat`/`--console` para ver tudo na hora.
- O pacote funciona de qualquer pasta: `data/` e `mods/` são procurados ao lado do executável.
- Opções novas: `--launcher`, `--config`, `--sem-config`, `--console`, `--perfil-driver`,
  `--desligar-mods`, `--com-som`, `--capturar-launcher`.
- Documentação no GitHub em inglês, com todos os documentos também em português
  (`README.pt-BR.md`, `docs/pt-BR/`).

### Mods

- **API de mods**, congelada em 1.0 no lançamento (versões 0.x durante o desenvolvimento; ver `docs/pt-BR/api-mods.md`): substituição de
  objetos do banco original, dos arquivos soltos do jogo e de 42 arquivos `data/*.json` do port,
  mesclados por chave; scripts **Lua 5.4** em sandbox, com ganchos fora e dentro do tick, leitura e
  escritas validadas do estado (comida, cheiros, castas, formigas nos ninhos), menu e painel próprios e
  estado guardado ao lado do save (`JOGO.ANT.mods.json`).
- Mods de exemplo em `exemplos/mods/`: `exemplo-lua` (chuva de comida) e `trapacas` (menu F12 com
  chuva, revoada, menino, gato, fim de jogo e painel de depuração).

### Limitações conhecidas

- É preciso ter o SimAnt para Windows 3.x original (os arquivos `.DAT`/`.NDX` da pasta `ANTWIN`).
- Diálogos internos do visor de ajuda (Open, Print, Annotate...) não existem.
- Alguns detalhes de 1 pixel em poucos desenhos (gato, base dos textos do fim de jogo, gráficos do
  History) e o realce do menu em 256 cores ainda diferem do original.
- Ritmo das velocidades rápida e ultra não foi medido numa máquina real.
- Rede (NetBIOS/DDE) do original fora do escopo.
- A lista completa está em `docs/pt-BR/estado-do-projeto.md` §3–§5 e `docs/pt-BR/divergencias.md`.

## Desenvolvimento (0.x, sem lançamento)

Resumo do histórico do repositório (338 commits, 16 a 19 de outubro de 2026); os detalhes estão em
`git log`, `docs/pt-BR/decisoes.md` e no histórico de compatibilidade de `docs/pt-BR/api-mods.md`.

- **2026-10-16 — Marco 1:** especificação inicial, parsers do banco `.NDX/.DAT` com LZSS, bitmaps,
  fontes e save `.ANT`; abertura em Rust + SDL2; escolha de Rust + SDL2 e Lua; bancada automática QEMU
  + SvarDOS que lê e escreve as variáveis do jogo original e grava vetores tick a tick.
- **2026-10-17:** regras do tick (feromônios, superfície, ninhos, inimigos, quintal, água, conselhos,
  tutorial) conferidas contra os vetores; som e música; modos de jogo, quintal, troca de patch, casa e
  Experimental; janelas Caste/Behavior/History, Info, Examine, mapa, Load/Save As; scripts Lua de mods
  (API 0.12); CI no GitHub Actions.
- **2026-10-18:** fim de jogo dentro do tick, Load do tutorial, tutorial completo no original, janela
  nítida e opções de vídeo, preferências do jogador, barra de menus do Windows, ajuda (`SIMANT.HLP`),
  balões "Silly", menu de ferramentas, quadros da janela 26, desenho da aranha; API 0.17–0.25 com o
  mod de trapaças.
- **2026-10-19:** API 0.26 (mesclagem por chave, ganchos dentro do tick, estado no save); tela virtual
  640/800/1024 e perfil de driver; `WIN.INI` como origem das preferências; sintetizador FM (padrão
  por um tempo; no lançamento o padrão é o simples) e banco ajustado às 28 músicas do original; barra de título do Windows 3.1 opcional; sétima e oitava
  rodadas de vetores (ramos raros, estratégia vermelha, recrutamento, revoada) e geração conferida;
  API 0.34. Textos do SimAnt fora do repositório: os `data/*.json` guardam só a referência (banco,
  executável, layout de janela, `.HLP`) e o port lê o texto da instalação do usuário; API 0.35.
