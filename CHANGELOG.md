# Histórico de versões

Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/); as versões seguem o
[Versionamento Semântico](https://semver.org/lang/pt-BR/). A versão do port e a versão da **API de mods**
(`docs/api-mods.md`) são numeradas separadamente.

> **Rascunho** para revisão (ver `docs/lancamento.md`). A data e o número só ficam valendo quando a tag
> `v1.0.0` for criada; até lá os `Cargo.toml` continuam em `0.1.0`.

## [1.0.0] — não lançado

Primeira versão para quem joga. Reimplementação do **SimAnt: The Electronic Ant Colony** (Maxis, 1991,
versão Windows 3.x) em Rust + SDL2, que lê os arquivos do jogo original do próprio usuário. Nenhum
arquivo do jogo é distribuído.

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
- **Som e música:** os efeitos e as 28 músicas do original. Sintetizador FM próprio no estilo do OPL
  da Sound Blaster (padrão), com o banco de timbres ajustado contra gravações do original; opção de
  SoundFont General MIDI (`--soundfont`) e de um sintetizador simples.
- **Ritmo do original:** relógio de 55 ms do Windows 3.1, velocidades lenta/normal medidas no original
  (3 e 6 ticks/s), rápida e ultra.

### Para os sistemas de hoje

- Janela nítida: ciente de DPI, ampliação por múltiplos inteiros, redimensionável.
- Tela virtual de 640×480 (padrão), 800×600 ou 1024×768, com o layout que o original usa em cada uma.
- Barra de título do Windows 3.1 opcional (`--moldura`).
- Preferências guardadas entre partidas na pasta do usuário (no lugar do `[SimAnt]` do `WIN.INI`).
- Teclas e opções em arquivos de dados (`data/*.json`).
- Linha de comando para rodar sem janela (`--ticks`, `--salvar`, exportação de telas e sons).

### Mods

- **API de mods 1.0** (era 0.34 durante o desenvolvimento; ver `docs/api-mods.md`): substituição de
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
- A lista completa está em `docs/estado-do-projeto.md` §3–§5 e `docs/divergencias.md`.

## Desenvolvimento (0.x, sem lançamento)

Resumo do histórico do repositório (338 commits, 16 a 19 de outubro de 2026); os detalhes estão em
`git log`, `docs/decisoes.md` e no histórico de compatibilidade de `docs/api-mods.md`.

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
  640/800/1024 e perfil de driver; `WIN.INI` como origem das preferências; sintetizador FM como padrão
  e banco ajustado às 28 músicas do original; barra de título do Windows 3.1 opcional; sétima e oitava
  rodadas de vetores (ramos raros, estratégia vermelha, recrutamento, revoada) e geração conferida;
  API 0.34.
