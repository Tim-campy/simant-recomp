*Read in English: [README-user.md](README-user.md) (no pacote de lançamento: `README.txt`)*

# SimAnt recomp — LEIAME

Versão 1.0.0 (2026-10-20). Reimplementação moderna do **SimAnt: The Electronic Ant Colony** (Maxis, 1991,
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
2. Abra o `simant.exe` (Windows, dois cliques) ou o `simant` (Linux). Na primeira vez aparece a tela de
   **opções de inicialização** (o launcher, seção 3): escolha nela a pasta do jogo original e clique em
   **Jogar**. A escolha fica guardada.

Outros jeitos de indicar a pasta do jogo (o primeiro que existir vale): a opção `--jogo "C:\MAXIS\ANTWIN"`;
a pasta escolhida no launcher; a variável de ambiente `SIMANT_DIR`; ou a pasta `SimAnt-T\MAXIS\ANTWIN`
dentro da pasta do pacote.

## 3. Rodar: o launcher

O `simant` aberto **sem opções** (dois cliques, atalho, menu) mostra o launcher (em português ou em inglês):

- **Pasta da instalação do SimAnt:** *Procurar...* abre o seletor de pastas do sistema (no Linux, o do
  `zenity` ou do `kdialog`, se instalados; sem eles, digite o caminho). Também dá para arrastar a pasta
  para a janela. A linha de baixo diz se a pasta tem os arquivos do jogo (`SHARED.NDX`...) e o que falta.
- **Início:** a abertura com o "Select a Game" (como o original), direto num **jogo novo** (Rápido,
  Completo, Experimental ou Tutorial) ou num **jogo salvo** (`.ANT`, com *Procurar...*).
- **Vídeo:** resolução imitada (640×480, 800×600, 1024×768), escala da janela (1x a 4x ou a máxima que
  cabe), tamanho inicial da janela (`LxA`), a barra de título do Windows 3.1 e o perfil de driver (as
  cores do Windows 3.1 em VGA de 16 cores, SVGA de 256 ou o S3 do DOSBox-X). A "Moldura Win 3.1" vem
  marcada: a janela fica sem a borda do sistema e se comporta como uma janela do Windows 3.1: arraste
  pelo título, duplo clique nele (ou o botão da direita) maximiza ou restaura, a moldura cinza
  redimensiona, minimizar e o menu de sistema funcionam até com um diálogo aberto, e Alt+F4 pergunta
  se quer salvar. Desmarque para usar a janela normal do sistema.
- **Som:** ligado ou não, e o sintetizador das músicas (simples — o padrão —, FM ou SoundFont, com o
  arquivo `.sf2`).
- **Mods:** a pasta (`mods`, `exemplos/mods` ou outra) e uma caixa para ligar ou desligar cada mod dela.
- **Preferências do jogador:** guardar as opções entre partidas e o `WIN.INI` de onde ler as opções do
  original na primeira vez (o automático é o `WINDOWS\WIN.INI` perto da pasta do jogo). "Fast/Ultra do
  486" volta as velocidades rápida e ultra ao ritmo do original (bem mais rápidas que o padrão do port,
  2,5× e 5× a normal); "Lenta sem foco" põe a simulação na lenta enquanto a janela está sem foco.
- **Linha de comando equivalente:** as opções `--` que correspondem às escolhas (as mesmas da seção 5).
  Dá para editá-la e apertar Enter: as escolhas acima mudam junto, e opções que a tela não tem (como
  `--semente 42`) ficam guardadas.
- **Jogar** guarda as escolhas e começa o jogo; **Salvar configuração** só guarda; **Sair** fecha.
  Desmarcando **Abrir esta tela ao iniciar**, os dois cliques vão direto ao jogo com as escolhas
  guardadas; para ver o launcher de novo, rode `simant --launcher` (ou apague o arquivo de configuração).

Teclas no launcher: Enter = Jogar (fora dos campos), Esc = Sair, Tab passa de campo, Ctrl+V cola,
Ctrl+C copia o campo (ou a linha de comando).

As escolhas ficam em `launcher.json` na pasta de dados do usuário: `%APPDATA%\SimAnt recomp\SimAnt\`
(Windows) ou `~/.local/share/SimAnt recomp/SimAnt/` (Linux). O port acha `data/` e `mods/` ao lado do
executável, então funciona de qualquer pasta, por atalho ou pelo terminal.

**Mensagens e erros.** No Windows o `simant.exe` não abre console: as mensagens (dos mods, avisos) vão para
`simant.log` nessa mesma pasta de dados (o da execução anterior fica em `simant-anterior.log`), e um erro
que impede o jogo de abrir aparece numa caixa de mensagem. Para ver as mensagens na hora, use o
`simant-console.bat` (ou `simant.exe --console`). No Linux, rodando de um terminal, as mensagens saem
nele; aberto pelo gerenciador de arquivos, vão para o mesmo `simant.log`.

### Pelo terminal

Com opções, o `simant` vai direto ao jogo; as escolhas guardadas no launcher valem por baixo (a pasta do
jogo, vídeo, som, mods), e as opções digitadas vencem:

```
simant                                  launcher (ou o jogo direto, se assim configurado)
simant --launcher                       sempre o launcher
simant --novo-jogo --tipo completo      direto num jogo novo: tutorial, rapido, completo, experimental
simant --save MEUJOGO.ANT               abre um save (os .ANT do original também)
simant --escala 2                       janela ampliada 2x (0 = a maior que cabe)
simant --resolucao 800x600              layout do original em 800x600
simant --sem-som                        sem áudio
simant --sem-config --novo-jogo         ignora as escolhas guardadas no launcher
```

Se aparecer `erro: não encontrei ...\SHARED.ndx`, a pasta do jogo está errada: abra o launcher e escolha
outra.

## 4. Como se joga

Você é a **formiga amarela** de uma colônia preta, contra as vermelhas, no quintal de uma casa. O jogo
em si é em inglês, como o original (os textos vêm da sua cópia).

- **Mouse:** clique leva a amarela até lá; **duplo clique** age (ataca, pede comida, pega ou larga,
  cava); segurar o botão sobre a amarela abre o menu dela.
- **Amarela:** **1**/**2** recrutam 5/10 operárias, **3**/**4** liberam metade/todas, **0** alarme,
  **X** troca de corpo, **Home** centraliza.
- **Velocidade e partida:** **Shift+0** pausa, **Shift+1..4** velocidades (sem o foco da janela, a
  simulação anda na lenta e volta sozinha), **Ctrl+S** salva,
  **Ctrl+A** salva como, **Ctrl+L** carrega, **Ctrl+N** jogo novo.
- **Janelas:** **Ctrl+E** edição, **Ctrl+B** Behavior, **Ctrl+C** Caste, **Ctrl+H** History,
  **Ctrl+I** Info, **Ctrl+M** mapa; **Ctrl + setas** rolam a vista.
- **Ajuda:** **F1** abre a ajuda do jogo; **Shift+F1** ou os botões *?* explicam o que você clicar.
- **Trapaças do original:** digite as quatro letras durante a partida (`heal`, `just`, `qeen`, `eggs`...).

Todas as teclas podem ser trocadas em `data/partida.json`, `data/navegacao.json` e `data/janelas.json`.

## 5. Opções

| Opção | Efeito |
|---|---|
| `--jogo DIR` | pasta dos arquivos do jogo (padrão: a do launcher, `SIMANT_DIR` ou `SimAnt-T/MAXIS/ANTWIN`) |
| `--dados DIR`, `--mods DIR` | pastas dos dados do port (`data`) e dos mods (`mods`), ao lado do executável |
| `--desligar-mods A,B` | não carrega esses mods (nomes do `mod.json`) |
| `--novo-jogo`, `--tipo T`, `--semente N` | jogo novo do tipo `tutorial`, `rapido`, `completo` ou `experimental`, com semente fixa |
| `--save ARQ.ANT` | abre um save |
| `--resolucao LxA` | tela do Windows 3.1 imitada: `640x480` (padrão), `800x600` ou `1024x768` |
| `--escala N`, `--janela LxA` | ampliação inteira e tamanho inicial da janela |
| `--moldura`, `--sem-moldura` | barra de título do Windows 3.1 no lugar da moldura do sistema (ligada por padrão; `--sem-moldura` desliga; `--moldura-do-sistema`: dentro da moldura do sistema) |
| `--velocidades proporcional\|original` | rápida e ultra a 2,5× e 5× a normal (padrão) ou no ritmo do original |
| `--sem-foco lenta\|original` | sem foco, a lenta (padrão) ou o ritmo de sempre, como o original |
| `--perfil-driver vga\|svga256\|s3_dosbox` | cores e alinhamentos do driver de vídeo do Windows 3.1 imitado (padrão: `vga`) |
| `--sem-som`, `--com-som` | sem áudio (ou com, vencendo o launcher) |
| `--sintetizador simples\|fm\|soundfont`, `--soundfont ARQ.sf2` | como as músicas são tocadas (padrão: o simples; `fm` no estilo da Sound Blaster); com `soundfont`, um SoundFont General MIDI seu |
| `--preferencias ARQ`, `--sem-preferencias`, `--win-ini ARQ` | arquivo das opções guardadas entre partidas; `WIN.INI` lido na primeira vez |
| `--launcher` | abre o launcher mesmo com outras opções ou com ele desligado |
| `--config ARQ`, `--sem-config` | outro arquivo de configuração do launcher, ou nenhum |
| `--console` | (Windows) mostra as mensagens num console em vez do `simant.log` |

As opções de vídeo padrão ficam em `data/video.json`. As preferências do jogador, a configuração do
launcher e o log ficam em `%APPDATA%\SimAnt recomp\SimAnt\` (Windows) ou
`~/.local/share/SimAnt recomp/SimAnt/` (Linux).

## 6. Mods

Um mod é uma pasta dentro de `mods/` com um `mod.json`. Ele pode trocar dados do port (`data/*.json`,
só as chaves que mudam), gráficos e sons do banco original e trazer scripts Lua (que rodam isolados:
sem `io`, `os` nem `require`, com limite de tempo e de memória). Para experimentar os exemplos:

```
simant --mods exemplos/mods
```

(ou, no launcher, *Mods → exemplos/mods*).

- `exemplo-lua`: chuva de comida a cada 500 ticks;
- `trapacas`: menu **F12** com chuva, revoada, menino, gato, fim de jogo e um painel de depuração.

Para deixar um mod sempre ligado, copie a pasta dele para `mods/` (ou escolha `exemplos/mods` no
launcher). As caixas do launcher ligam e desligam cada mod sem tirá-lo da pasta. A referência completa
(API de mods 1.0) está em `docs/api-mods.md` no repositório do projeto.

## 7. Licença e créditos

- O código do SimAnt recomp: ver `LICENSE.txt`.
- Bibliotecas de terceiros incluídas no executável (SDL2, Lua, crates Rust, a fonte do launcher) e as licenças delas:
  `AVISOS-TERCEIROS.txt`. O `SDL2.dll` (Windows) é o binário oficial do SDL 2.30.9, sob a licença zlib.
- SimAnt © 1991 Maxis. Este projeto não usa nem distribui o código ou os arquivos do jogo.
- Código-fonte, documentação e relato de problemas: https://github.com/Tim-campy/simant-recomp
  (documentação em inglês, com versão em português em `README.pt-BR.md` e `docs/pt-BR/`).
- Histórico de versões: `CHANGELOG.pt-BR.txt` (português) e `CHANGELOG.txt` (inglês).
