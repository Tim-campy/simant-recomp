*Read in English: [CHANGELOG.md](CHANGELOG.md)*

# Histórico de versões

Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/); as versões seguem o
[Versionamento Semântico](https://semver.org/lang/pt-BR/). A versão do port e a versão da **API de mods**
(`docs/pt-BR/api-mods.md`) são numeradas separadamente.

## [1.0.0] — 2026-10-20

Primeira versão para quem joga. Reimplementação do **SimAnt: The Electronic Ant Colony** (Maxis, 1991,
versão Windows 3.x) em Rust + SDL2, que lê os arquivos do jogo original do próprio usuário. Nenhum
arquivo do jogo é distribuído; até os textos do jogo são lidos da instalação do usuário. Licença
**MIT OR Apache-2.0** (cobre só o código e os dados do port, nunca os arquivos do SimAnt).

### O jogo

- **Os quatro modos do original:** Tutorial (lições 1 a 57), Quick Game, Full Game (o quintal e a casa,
  com troca de patch) e Experimental (ferramentas de cavar, aterrar, comida, pedras, formigas, cheiro,
  inseticida e o menu de ferramentas de segurar).
- **Simulação igual à do original, tick a tick:** 223 vetores de tick gravados do SimAnt rodando, 2
  sequências longas (200 e 600 ticks), 21 ações de interface e o Load do tutorial, todos conferidos bit
  a bit (307/307 blocos de estado), mais 21 casos de regra da bancada (20 confirmados no original e
  iguais no port; um não é conferível). A geração do Quick Game, dos patches da casa e do quintal e do
  jogo novo de cada tipo também bate com o original antes do primeiro tick.
- **Formiga amarela:** andar, comer, cavar, recrutar e liberar operárias, alarme, trofalaxia, troca de
  corpo, lutas e as dez mortes do original; **morte, luta perdida e renascimento apresentados na ordem
  do original** (a mesma linha do tempo de quadros e mensagens). Inimigos (formiga-leão, aranha,
  inclusive a aranha do jogador com AutoTrack, lagarta, tatuzinho), quintal (menino, cortador de
  grama, inseticida, gato, cachorro, pássaro, chuva, revoadas) e fim de jogo com a pontuação e a
  classificação do original.
- **Telas e janelas do original:** abertura e "Select a Game", janela de edição, mapa, mini mapa,
  quintal (modos quintal, casa, estratégico e gráfico, com as barras e populações da faixa 35), Caste
  Control, Behavior Control, History, Info (51 cartões), Examine, balões de fala ("Silly" e os 23 do
  menino), quadros da janela 26, barra de menus do Windows 3.1 com About, Status, Quit e Load Game,
  menu de sistema das janelas, faixa com botões configuráveis (inclusive os botões da aranha e do
  Status), marcadores e barras de saúde. Comparadas pixel a pixel com capturas do original (93–100%).
- **Diálogos Load/Save As completos**, como o diálogo comum do Windows 3.1: arrasto pelo título, listas
  suspensas de tipo e unidade, rolagem completa das listas, troca de pasta, Read Only e edição do nome
  do arquivo.
- **Salvar e carregar** no formato `.ANT` do original (o arquivo gravado pelo port abre no SimAnt).
- **Ajuda:** visor do `SIMANT.HLP` (WinHelp 3.0) que é uma **janela do Windows** própria (menu de
  sistema, mover, redimensionar, minimizar, maximizar; Ctrl+F4 fecha), com Search e History, e ajuda de
  contexto (Shift+F1, botões *?*).
- **Trapaças:** os 26 códigos de trapaça do original (`heal`, `just`, `qeen`, `eggs`...) e, como mod de
  exemplo, um menu de trapaças (seção "Mods").
- **Som e música:** efeitos e as 28 músicas do original. As músicas tocam por um **sintetizador simples,
  o padrão** (escolha do usuário); opcionais: sintetizador FM próprio no estilo do OPL da Sound Blaster
  (`--sintetizador fm`), com o banco de instrumentos ajustado contra gravações do original, e um
  SoundFont General MIDI (`--soundfont`). **Os efeitos tocam a 5000 Hz em blocos de 504 amostras**,
  como o driver da Sound Blaster 16 os toca (medido numa sessão do original jogada no DOSBox-X: um
  efeito menor que um bloco não soa, e o resto do último bloco se perde).
- **Ritmo do original:** relógio de 55 ms do Windows 3.1 e velocidades lenta/normal medidas no original
  (3 e 6 ticks/s).

### Para os sistemas de hoje

- Janela nítida: ciente de DPI, ampliação por inteiros, redimensionável.
- Tela virtual de **640×480** (padrão), **800×600** ou **1024×768**, com a disposição das janelas que o
  original usa em cada uma, e **perfis de driver** (VGA de 16 cores, o padrão; SVGA de 256 cores; o S3
  do DOSBox-X), que mudam o alinhamento ao byte das janelas e as cores do sistema como cada driver.
- **A barra de título do Windows 3.1 é a moldura da janela por padrão**, no lugar da do sistema: janela
  sem borda, arrastada pelo título, maximizar/restaurar/minimizar e menu de sistema agindo na janela de
  verdade (também com um diálogo aberto e no "Select a Game"), moldura grossa de 4 pixels com a janela
  restaurada, Alt+F4 sempre faz a pergunta do Quit. `--moldura-do-sistema` mantém a borda do sistema
  em volta e `--sem-moldura` a desliga. Testes e exportações rodam sem ela.
- **Ritmo proporcional** (escolha do usuário): Fast a 2,5× e Ultra a 5× a Normal, e a simulação em
  velocidade lenta enquanto a janela está sem foco. Para o comportamento do original:
  `--velocidades original` (o ritmo do 486 da bancada) e `--sem-foco original`, também como caixas no
  launcher. O tick, o save e o menu Speed não mudam.
- **Launcher em inglês e português:** o `simant` aberto sem opções mostra uma tela de opções de
  inicialização: pasta da instalação do SimAnt (seletor do sistema, conferida e lembrada), início
  (abertura, jogo novo de cada tipo ou um save), vídeo, perfil de driver, som e sintetizador, mods
  (pasta e cada mod ligado ou não), preferências e a linha de comando equivalente, editável. As
  escolhas ficam em `launcher.json` e valem também para o `simant` rodado com opções (que vencem).
- **Idiomas:** os textos próprios do port (launcher, mensagens sobre mods, console) passam por um frame
  de idiomas em `data/idiomas/`, que pode ser estendido, inclusive por mods (`docs/pt-BR/i18n.md`). O
  jogo é em inglês, como o original; os textos dele vêm da instalação do usuário.
- **Sem console no Windows:** o `simant.exe` grava as mensagens em `simant.log` na pasta do usuário e
  os erros fatais numa caixa de mensagem; `simant-console.bat`/`--console` mostram tudo na hora.
- Preferências guardadas entre partidas na pasta do usuário (na primeira vez, lidas da seção
  `[SimAnt]` do `WIN.INI` perto da pasta do jogo, quando houver).
- Teclas e opções em arquivos de dados (`data/*.json`).
- Linha de comando para rodar sem janela (`--ticks`, `--salvar`, exportação de telas e sons).
- O pacote funciona de qualquer pasta: `data/` e `mods/` são procurados ao lado do executável.
- **Documentação em inglês**, com todos os documentos também em português (`README.pt-BR.md`,
  `CHANGELOG.pt-BR.md`, `docs/pt-BR/`); um conferidor de links roda no CI.

### Mods

- **API de mods 1.0** (congelada neste lançamento; é a 0.43 com o número definitivo; daqui em diante as
  versões 1.x só acrescentam coisas compatíveis, e uma mudança incompatível faz a 2.0; ver
  `docs/pt-BR/api-mods.md`): substituição de objetos dos bancos originais, de arquivos soltos do jogo e
  dos arquivos `data/*.json` do port, mesclados por chave; arquivos de idioma em `data/idiomas/` (chaves
  ou um idioma inteiro); scripts **Lua 5.4** em sandbox, com ganchos fora e dentro do tick, leitura e
  escritas validadas do estado (comida, cheiros, castas, formigas nos ninhos), menu e painel próprios,
  estado salvo ao lado do save (`JOGO.ANT.mods.json`) e textos no idioma do jogo (`idioma()`,
  `traduzir()`).
- Mods de exemplo em `exemplos/mods/`: `exemplo-lua` (chuva de comida) e `trapacas` (mod de trapaças:
  menu F12 com chuva, revoada, menino, gato, comida, fim de jogo e painel de depuração), ambos com
  textos em inglês e um exemplo em português.

### Conferido para este lançamento

- `cargo test --workspace` com os arquivos do jogo: **693 testes passaram, 0 falharam** (1 ignorado por
  padrão: abre o diálogo de pastas do Windows), em 41 binários de teste; também sem os arquivos do jogo
  (os testes que precisam deles avisam e passam). `cargo clippy --workspace --all-targets -- -D
  warnings`: sem avisos.

### Limitações conhecidas

- É preciso ter o SimAnt para Windows 3.x original (os arquivos `.DAT`/`.NDX` da pasta `ANTWIN`, além do
  `SIMANTW.EXE` e do `SIMANT.HLP`, de onde os textos são lidos).
- **O jogo está só em inglês por ora** (como o original); só o launcher tem português.
- **Um efeito que corta outro nem sempre segue a regra dos blocos do driver:** os blocos de 504
  amostras foram medidos com efeitos que tocam até o fim; quando um efeito interrompe outro, o corte do
  port nem sempre cai onde o driver da Sound Blaster 16 cortaria.
- **O ritmo de Fast e Ultra é escolha do port** (2,5× e 5× a Normal), não medida: no original eles
  dependem da velocidade da máquina (`--velocidades original` dá o ritmo do 486 da bancada).
- **Latido do cachorro, pássaro, diálogos da aranha e trofalaxia sem conferência de áudio:** os sons
  deles não foram comparados com uma gravação do original, como os outros efeitos.
- **Linux não foi testado à mão:** o pacote é montado e passa pelo teste rápido (sem os arquivos do
  jogo) no workflow de lançamento, mas o jogo, o launcher e os seletores de arquivo não foram rodados
  numa máquina Linux com os arquivos do jogo.
- Diálogos internos do visor de ajuda (Open, Print, Annotate...) não existem.
- Alguns detalhes de 1 pixel em poucos desenhos (gato, base dos textos do fim de jogo, gráficos do
  History) e o realce do menu em 256 cores ainda diferem do original.
- Rede e DDE do original (NetBIOS, ligações com outros programas) fora do escopo.
- A lista completa está em `docs/pt-BR/estado-do-projeto.md` §3–§5 e `docs/pt-BR/divergencias.md`.

## Desenvolvimento (0.x, sem lançamento)

Resumo do histórico do repositório (422 commits, 16 a 20 de outubro de 2026); os detalhes estão em
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
  640/800/1024 e perfil de driver; `WIN.INI` como origem das preferências; sintetizador FM e banco
  ajustado às 28 músicas do original; barra de título do Windows 3.1; sétima e oitava rodadas de
  vetores (ramos raros, estratégia vermelha, recrutamento, revoada) e geração conferida; licença;
  launcher e executável sem console; textos do SimAnt fora do repositório, lidos da instalação do
  usuário (API 0.35); linha do tempo da morte na ordem do original (0.37); ritmo proporcional, lenta
  sem foco, sintetizador simples como padrão e a barra de título no lugar da moldura do sistema (0.38);
  barras da faixa 35 (0.39); ajuda como janela do Windows, diálogos de arquivo completos, botões da
  aranha e do Status (0.40); a barra de título ligada por padrão; comida e pedras dos mods dentro de
  casa (0.41); efeitos a 5000 Hz em blocos de 504 amostras, medidos numa sessão jogada (0.42).
- **2026-10-20:** documentação traduzida para o inglês, com as cópias em português em `docs/pt-BR/`,
  e conferidor de links no CI; frame de idiomas (`data/idiomas/`), launcher em inglês e português e os
  textos do port em inglês (API 0.43); lançamento 1.0.0 com a API de mods congelada em 1.0.
