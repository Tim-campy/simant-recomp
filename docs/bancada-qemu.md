# Bancada de testes automática (QEMU + SvarDOS)

Roda o SimAnt original sem janela e sem usar o mouse/teclado do usuário, controlado só pelo protocolo QMP do QEMU. Serve para gerar referências (capturas, jogos salvos, dumps de memória) e, no futuro, para testes de regras contra o original.

Decisão e alternativas avaliadas: `docs/decisoes.md` e `reports/Automação de testes Win16 emulado.md`.

## Componentes (todos fora do git, em `re_trabalho/` e `referencia/`)

| Peça | Origem | Observação |
|---|---|---|
| QEMU 2026-08-11 (`referencia/qemu/`) | qemu.weilnetz.de, instalador oficial para Windows (SHA-512 conferido) | `qemu-system-i386`, aceleração TCG |
| SvarDOS 20250427 (kernel EDR-DOS + SvarCOM + HIMEMX) | svardos.org, disquete 1,44 MB | Base do disquete de boot mínimo montado por `tools/qemu_bancada.py` |
| Disco C: (`re_trabalho/qemu/c.img`, FAT16, 100 MB) | cópia de `SimAnt-T/WINDOWS` e `SimAnt-T/MAXIS` | criado com `IMGMAKE`/`COPY` do DOSBox-X (o `vvfat` do QEMU falha quando o Windows grava) |

Ajustes na **cópia** do Windows for Workgroups 3.11 (a instalação do usuário não é tocada):

- `SYSTEM.INI`: `display.drv=vga.drv` e `display=*vddvga` (o S3 Trio não existe no QEMU); `;device=vshare.386` (o VSHARE trava o WfW sobre o kernel EDR); `InDOSPolling=TRUE`.
- `WIN.INI`: `MouseSpeed=0`, `MouseThreshold1=0`, `MouseThreshold2=0` (sem aceleração: 1 unidade de movimento = 1 pixel).

Resultado: o WfW 3.11 roda em **modo 386 avançado**, com tela de **640×480 e 16 cores**. Por isso o jogo usa a tabela de posições de janela para telas de até 480 linhas (objeto 9/0). As referências do DOSBox-X (800×600) e do QEMU (640×480) não são intercambiáveis.

## Uso

```
python tools/qemu_bancada.py preparar                 # recria disquete e disco C: (≈1 min)
python tools/qemu_bancada.py roteiro tests/referencias/roteiros/qemu_salvar_quick.txt
python tools/fat16_extrair.py re_trabalho/qemu/c.img MAXIS/ANTWIN/QUICK2.ANT tests/referencias/saves/QUICK2.ANT
```

Comandos de roteiro: `esperar`, `teclas` (qcodes do QMP, `+` combina), `digitar`, `mouse_canto`, `mover`, `clicar`, `clicar_em X Y`, `capturar ARQ.png`, `memoria INICIO TAMANHO ARQ` (`pmemsave`) e `fim`, que encerra o QEMU com `quit` limpo.

## O que foi verificado (2026-10-16)

| Item | Resultado |
|---|---|
| Boot até o jogo | título em cerca de 10 s após iniciar |
| Teclado (`send-key`) | Espaço fecha o título e o registro; Alt+F abre o menu File; Ctrl+A abre "Save As"; a digitação do nome funciona |
| Mouse (`input-send-event` relativo) | depois de `mouse_canto`, um movimento de (100, 100) põe a ponta da seta exatamente em (100, 100) |
| Clique | `clicar_em 345 228` escolhe "Quick Game" no "Select a Game" |
| Captura (`screendump`) | PNG 640×480, sem janela visível |
| Memória (`pmemsave`) | dump de 16 MB da RAM física |
| Jogo salvo | `QUICK2.ANT` com **48.386 bytes**, exatamente o previsto em `spec/formatos/save-ant.md` |

## Cuidados

- O DOS grava com atraso. Antes de encerrar, espere uns 15 s depois de salvar; o `fim` já usa `quit`. O primeiro save (`QUICK1.ANT`) saiu com 0 bytes porque o QEMU foi morto logo depois de salvar.
- O disco `c.img` é persistente entre execuções. Use `preparar` para voltar ao estado limpo, ou nomes de arquivo novos.
- A metade superior do logotipo aparece branca no título em 640×480 [em aberto: investigar memória (`-m 16`) ou o carregamento do bitmap 2/2000].
- Os tempos são do relógio do host (o QEMU não está em modo determinístico). Para determinismo, os próximos passos são `savevm`/`loadvm` e o record/replay por `icount`.
