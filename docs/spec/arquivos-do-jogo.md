# Inventário dos arquivos (`SimAnt-T/`)

A pasta fornecida pelo usuário é uma instalação completa de Windows 3.1 com o jogo instalado, no estilo dos pacotes para DOSBox. Os arquivos são do usuário e não são redistribuídos.

| Caminho | O que é |
|---|---|
| `MAXIS/ANTWIN/` | **jogo instalado**: é daqui que o port lê |
| `MAXIS/ANTWIN/SIMANTW.EXE` | executável NE de 16 bits (ver [executavel.md](executavel.md)) |
| `MAXIS/ANTWIN/SIMANTW.SYM` | tabela de símbolos MAPSYM: 2.171 nomes de funções e variáveis |
| `MAXIS/ANTWIN/SHARED.*`, `WINGANT.*`, `MWINNT.*`, `SOUND.*` | bancos de recursos (ver [formatos/banco-dat-ndx.md](formatos/banco-dat-ndx.md)) |
| `MAXIS/ANTWIN/SIMANT.CFG` | modo de vídeo e de som (ver [formatos/simant-cfg.md](formatos/simant-cfg.md)) |
| `MAXIS/ANTWIN/SOUND/*.MID` | 29 músicas em MIDI padrão. O EXE monta o caminho com `"%ssound\%s.mid"` |
| `MAXIS/ANTWIN/FONT1`–`FONT4`, `FONTRES.FON` | fontes. `FONTRES.FON` contém `Gaia8x8`; `FONT1`–`4` têm formato próprio, lido por `_font_ReadFont` [em aberto] |
| `MAXIS/ANTWIN/SIMANT.HLP` | ajuda do WinHelp |
| `MAXIS/ANTWIN/README.WRI` | leia-me em Windows Write |
| `CD/` | mídia de instalação. Os arquivos com `_` no nome (`_IMANTW.EXE` etc.) estão comprimidos em **SZDD** (assinatura `SZDD`, formato do `COMPRESS.EXE` da Microsoft). São cópias do conteúdo instalado [hipótese: idênticas depois de expandidas, não verificado] |
| `WINDOWS/`, `SB16/`, `drivers/`, `AUTOEXEC.BAT`, `CONFIG.SYS` | Windows 3.1, drivers da Sound Blaster 16 e da S3 Trio (800×600, 256 cores, segundo o `SYSTEM.INI`) |
| `SimAnt - The Electronic Ant Colony (1991).ba1` | lançador de pacote eXoWin3x. Ele aponta para `games\SimTower\` e `games\!win3x\SimTower\dosbox.conf`, ou seja, não serve para este jogo do jeito que está. Não há `dosbox.conf` do SimAnt na pasta |

Nenhum jogo salvo `.ANT` veio na pasta.
