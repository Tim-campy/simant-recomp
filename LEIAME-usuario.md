# SimAnt recomp — LEIAME

Versão 1.0.0 (rascunho). Reimplementação moderna do **SimAnt: The Electronic Ant Colony** (Maxis, 1991,
versão Windows 3.x) para Windows e Linux atuais. O jogo é refeito do zero; os gráficos, os sons, as
músicas e a ajuda vêm da **sua cópia** do SimAnt original, que este pacote não contém.

> **Aviso legal.** Este é um projeto de fãs, sem fins lucrativos, **sem afiliação nem endosso da Maxis
> ou da Electronic Arts**. "SimAnt" e "Maxis" são marcas de seus donos. Este pacote **não distribui
> nenhum arquivo do jogo original** (gráficos, sons, músicas, textos, executável): para jogar, você
> precisa ter o SimAnt para Windows 3.x. Não peça nem compartilhe cópias do jogo nos canais do projeto.

## 1. Requisitos

- **Windows 10 ou 11, 64 bits:** nada além do pacote (o `SDL2.dll` já vem junto).
- **Linux x86-64** (glibc 2.35 ou mais nova, como Ubuntu 22.04, Debian 12, Fedora 36 em diante) com o
  **SDL2** do sistema:
  - Debian/Ubuntu: `sudo apt install libsdl2-2.0-0`
  - Fedora: `sudo dnf install SDL2` (ou `sdl2-compat`)
  - Arch: `sudo pacman -S sdl2-compat` (ou `sdl2`)
- **Os arquivos do SimAnt para Windows 3.x**, da sua instalação ou do disquete/CD original. O port
  precisa da pasta onde ficam:

  | Arquivo | Para quê |
  |---|---|
  | `SHARED.NDX`/`.DAT`, `WINGANT.NDX`/`.DAT` | gráficos, telas, textos (obrigatórios) |
  | `SOUND.NDX`/`.DAT` e a pasta `SOUND` (`*.MID`) | efeitos e músicas (sem eles o jogo roda mudo) |
  | `SIMANTW.EXE` | cursores do modo de ajuda e das ferramentas (só lido, nunca executado) |
  | `SIMANT.HLP` | a ajuda (F1) |
  | `FONTRES.FON`, `FONT1`…`FONT4` | fontes do jogo |

  Os demais arquivos da pasta (`MWINNT.*`, `SIMANT.CFG`...) não são usados; não faz mal deixá-los lá.

  Opcional: se a pasta do jogo estiver numa instalação do Windows 3.1 (`...\MAXIS\ANTWIN` com
  `...\WINDOWS\SYSTEM` ao lado), o port usa as fontes do sistema (`VGASYS.FON`, `SSERIFE.FON`) nos
  títulos e diálogos e lê as opções do `[SimAnt]` do `WIN.INI`. Sem elas, usa a fonte do jogo.

  Copie os arquivos de uma instalação ou do disco (uma pasta como `C:\MAXIS\ANTWIN`). O port só **lê**
  esses arquivos; nunca os altera.

## 2. Instalar

1. Descompacte o pacote numa pasta qualquer (por exemplo `C:\Jogos\simant-recomp` ou `~/jogos/simant-recomp`).
2. Diga ao port onde estão os arquivos do jogo, de um destes jeitos:
   - **pela opção** `--jogo "C:\MAXIS\ANTWIN"`;
   - **pela variável de ambiente** `SIMANT_DIR` (Windows: `setx SIMANT_DIR "C:\MAXIS\ANTWIN"` e abra um
     terminal novo; Linux: `export SIMANT_DIR=~/simant/ANTWIN` no `~/.profile`);
   - ou **copiando** a pasta do jogo para `SimAnt-T\MAXIS\ANTWIN` dentro da pasta do pacote.

## 3. Rodar

O port procura `data/` e `mods/` **na pasta atual**. Rode de dentro da pasta do pacote:

- **Windows:** dê dois cliques em `simant.exe` (com `SIMANT_DIR` definido ou a pasta `SimAnt-T` ao
  lado), ou num terminal: `cd C:\Jogos\simant-recomp` e `simant.exe --jogo C:\MAXIS\ANTWIN`.
- **Linux:** `./simant.sh` (entra na pasta do pacote e roda o `simant`), ou `cd` para a pasta e `./simant`.

Exemplos:

```
simant                                  abertura e "Select a Game" (Enter = Full Game)
simant --novo-jogo --tipo completo      direto num jogo novo: tutorial, rapido, completo, experimental
simant --save MEUJOGO.ANT               abre um save (os .ANT do original também)
simant --escala 2                       janela ampliada 2x (0 = a maior que cabe)
simant --resolucao 800x600              layout do original em 800x600
simant --sem-som                        sem áudio
```

Se aparecer `erro: não encontrei ...\SHARED.ndx`, o caminho dos arquivos do jogo está errado (passo 2).

## 4. Como se joga

Você é a **formiga amarela** de uma colônia preta, contra as vermelhas, no quintal de uma casa.

- **Mouse:** clique leva a amarela até lá; **duplo clique** age (ataca, pede comida, pega ou larga,
  cava); segurar o botão sobre a amarela abre o menu dela.
- **Amarela:** **1**/**2** recrutam 5/10 operárias, **3**/**4** liberam metade/todas, **0** alarme,
  **X** troca de corpo, **Home** centraliza.
- **Velocidade e partida:** **Shift+0** pausa, **Shift+1..4** velocidades, **Ctrl+S** salva,
  **Ctrl+A** salva como, **Ctrl+L** carrega, **Ctrl+N** jogo novo.
- **Janelas:** **Ctrl+E** edição, **Ctrl+B** Behavior, **Ctrl+C** Caste, **Ctrl+H** History,
  **Ctrl+I** Info, **Ctrl+M** mapa; **Ctrl + setas** rolam a vista.
- **Ajuda:** **F1** abre a ajuda do jogo; **Shift+F1** ou os botões *?* explicam o que você clicar.
- **Trapaças do original:** digite as quatro letras durante a partida (`heal`, `just`, `qeen`, `eggs`...).

Todas as teclas podem ser trocadas em `data/partida.json`, `data/navegacao.json` e `data/janelas.json`.

## 5. Opções

| Opção | Efeito |
|---|---|
| `--jogo DIR` | pasta dos arquivos do jogo (padrão: `SIMANT_DIR` ou `SimAnt-T/MAXIS/ANTWIN`) |
| `--dados DIR`, `--mods DIR` | pastas dos dados do port (`data`) e dos mods (`mods`) |
| `--novo-jogo`, `--tipo T`, `--semente N` | jogo novo do tipo `tutorial`, `rapido`, `completo` ou `experimental`, com semente fixa |
| `--save ARQ.ANT` | abre um save |
| `--resolucao LxA` | tela do Windows 3.1 imitada: `640x480` (padrão), `800x600` ou `1024x768` |
| `--escala N`, `--janela LxA` | ampliação inteira e tamanho inicial da janela |
| `--moldura`, `--sem-moldura` | barra de título do Windows 3.1 dentro da janela |
| `--sem-som` | sem áudio |
| `--sintetizador fm\|soundfont\|simples`, `--soundfont ARQ.sf2` | como as músicas são tocadas (padrão: FM no estilo da Sound Blaster); com `soundfont`, um SoundFont General MIDI seu |
| `--preferencias ARQ`, `--sem-preferencias` | arquivo das opções guardadas entre partidas |

As opções de vídeo padrão ficam em `data/video.json`. As preferências do jogador ficam em
`%APPDATA%\SimAnt recomp\SimAnt\` (Windows) ou `~/.local/share/SimAnt recomp/SimAnt/` (Linux).

## 6. Mods

Um mod é uma pasta dentro de `mods/` com um `mod.json`. Ele pode trocar dados do port (`data/*.json`,
só as chaves que mudam), gráficos e sons do banco original e trazer scripts Lua (que rodam isolados:
sem `io`, `os` nem `require`, com limite de tempo e de memória). Para experimentar os exemplos:

```
simant --mods exemplos/mods
```

- `exemplo-lua`: chuva de comida a cada 500 ticks;
- `trapacas`: menu **F12** com chuva, revoada, menino, gato, fim de jogo e um painel de depuração.

Para deixar um mod sempre ligado, copie a pasta dele para `mods/`. A referência completa (API de mods
1.0) está em `docs/api-mods.md` no repositório do projeto.

## 7. Licença e créditos

- O código do SimAnt recomp: ver `LICENSE.txt`.
- Bibliotecas de terceiros incluídas no executável (SDL2, Lua, crates Rust) e as licenças delas:
  `AVISOS-TERCEIROS.txt`. O `SDL2.dll` (Windows) é o binário oficial do SDL 2.30.9, sob a licença zlib.
- SimAnt © 1991 Maxis. Este projeto não usa nem distribui o código ou os arquivos do jogo.
- Código-fonte, documentação e relato de problemas: https://github.com/Tim-campy/simant-recomp
