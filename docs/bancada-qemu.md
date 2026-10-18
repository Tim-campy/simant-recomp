# Bancada de testes automática (QEMU + SvarDOS)

Roda o SimAnt original sem janela e sem usar o mouse/teclado do usuário, controlado só pelo protocolo QMP do QEMU (e pelo gdbstub para escrever na memória). Serve para gerar referências (capturas, jogos salvos, estados da memória) e para testes de regras contra o original.

Decisão e alternativas avaliadas: `docs/decisoes.md` e `reports/Automação de testes Win16 emulado.md`.

## Componentes (todos fora do git, em `re_trabalho/` e `referencia/`)

| Peça | Origem | Observação |
|---|---|---|
| QEMU 2026-08-11 (`referencia/qemu/`) | qemu.weilnetz.de, instalador oficial para Windows (SHA-512 conferido) | `qemu-system-i386`, aceleração TCG; `qemu-img` para o overlay |
| SvarDOS 20250427 (kernel EDR-DOS + SvarCOM + HIMEMX) | svardos.org, disquete 1,44 MB | Base do disquete de boot mínimo montado por `tools/qemu_bancada.py` |
| Disco C: (`re_trabalho/qemu/c.img`, FAT16, 100 MB) | cópia de `SimAnt-T/WINDOWS` e `SimAnt-T/MAXIS` | criado com `IMGMAKE`/`COPY` do DOSBox-X (o `vvfat` do QEMU falha quando o Windows grava) |
| Overlay (`re_trabalho/qemu/c_overlay.qcow2`) | `qemu-img create -f qcow2 -b c.img -F raw` | criado pelo script quando há snapshots; guarda os snapshots e o que o jogo grava |

Ajustes na **cópia** do Windows for Workgroups 3.11 (a instalação do usuário não é tocada):

- `SYSTEM.INI`: `display.drv=vga.drv` e `display=*vddvga` (o S3 Trio não existe no QEMU); `;device=vshare.386` (o VSHARE trava o WfW sobre o kernel EDR); `InDOSPolling=TRUE`.
- `WIN.INI`: `MouseSpeed=0`, `MouseThreshold1=0`, `MouseThreshold2=0` (sem aceleração: 1 unidade de movimento = 1 pixel).

Resultado: o WfW 3.11 roda em **modo 386 avançado**, com tela de **640×480 e 16 cores**. Por isso o jogo usa a tabela de posições de janela para telas de até 480 linhas (objeto 9/0). As referências do DOSBox-X (800×600) e do QEMU (640×480) não são intercambiáveis.

## Uso

```
python tools/qemu_bancada.py preparar                 # recria disquete e disco C: (≈1 min); apaga o overlay padrão
python tools/qemu_bancada.py roteiro tests/referencias/roteiros/qemu_salvar_quick.txt
python tools/fat16_extrair.py re_trabalho/qemu/c.img MAXIS/ANTWIN/QUICK2.ANT tests/referencias/saves/QUICK2.ANT

# com snapshots (disco em overlay qcow2):
python tools/qemu_bancada.py roteiro tests/referencias/roteiros/qemu_snapshot_quick.txt   # boot + savevm "quick" (≈1 min, uma vez)
python tools/qemu_bancada.py roteiro tests/referencias/roteiros/qemu_estado_save.txt      # parte do "quick" sem boot
python tools/qemu_bancada.py extrair MAXIS/ANTWIN/QUICK3.ANT re_trabalho/QUICK3.ANT --overlay re_trabalho/qemu/c_overlay.qcow2
python tools/estado_diff.py re_trabalho/QUICK3.ANT re_trabalho/estado_quick3_salvo.bin
```

Opções de `roteiro`: `--overlay ARQ.qcow2` (abre o disco C: por esse overlay; caminho relativo à raiz) e `--mem MB` (RAM da VM, padrão 16). A variável de ambiente `SIMANT_RAIZ` aponta a raiz com `referencia/` e `re_trabalho/` (para rodar o script de um worktree do git); os caminhos de saída dos comandos são relativos a ela. As portas do QMP e do gdbstub são escolhidas livres a cada execução, então duas bancadas podem rodar ao mesmo tempo, desde que não gravem no mesmo disco.

`tools/estado_diff.py A B` compara dois arquivos no formato do `.ANT` (save, `estado` ou dump do port) bloco a bloco, com os nomes do layout e os valores dos blocos de até 4 bytes.

### Comandos de roteiro

| Comando | O que faz |
|---|---|
| `esperar SEG` | espera SEG segundos do relógio do host |
| `teclas T1 T2 ...` | `send-key` com qcodes do QMP (`spc`, `ret`, `esc`, `alt+f`, ...) |
| `digitar TEXTO` | digita letras, números e espaço |
| `mouse_canto`, `mover DX DY`, `clicar`, `clicar_em X Y` | mouse relativo (`input-send-event`); com a aceleração desligada, 1 unidade = 1 pixel |
| `capturar ARQ.png` | `screendump` |
| `memoria INICIO TAMANHO ARQ` | `pmemsave` (RAM **física**) |
| `pausar` / `continuar` | `stop` / `cont` do QMP: congela a CPU, o relógio da VM e a simulação. No modo tick, `continuar` volta a rodar livre (os breakpoints já estão desligados) |
| `snapshot_salvar NOME` / `snapshot_carregar NOME` | `savevm` / `loadvm` (HMP) no overlay qcow2 |
| `ler_variavel NOME ARQ [TAM]` | grava os bytes da variável em ARQ e mostra o valor se tiver até 4 bytes |
| `escrever_variavel NOME VALOR [TAM]` | escreve VALOR (inteiro little-endian no tamanho da variável, ou `hex:AABB...`) pelo gdbstub e confere relendo |
| `estado ARQ` | todos os 307 blocos de `data/save_layout.json`, na ordem do save: o arquivo tem o formato exato de um `.ANT` (48.386 bytes) |
| `disquete ARQ.img` | troca a mídia do drive A: (`blockdev-change-medium` do QMP, só leitura); usado para injetar saves (ver [Casos de teste](#casos-de-teste-injetar-um-save-e-rodar)) |
| `esperar_variavel NOME ALVO [TIMEOUT]` | consulta NOME (sem sinal) a cada 0,05 s até chegar a ALVO; falha depois de TIMEOUT s (padrão 120). Seguido de `pausar`, para no tick ALVO ou 1 depois |
| `extras ARQ.json` | variáveis fora do save capturadas com os vetores de tick (sementes S e R, `_gameCycles`, `_Tindex`, `_GlobalKey`), `GameTime`, `Cycle` e a posição (`entrada`/`saida`/`livre`). O relógio do Mac (`_MacTickCount`, função) só é capturado por `passo_tick`/`gerar_vetores_tick.py`, com breakpoints dentro do tick |
| `segmentos PREFIXO` | segmentos de dados 8, 9 e 10 inteiros (até o limite do descritor) em `PREFIXO_seg8.bin` etc. |
| `tick_entrada` | modo tick: roda até a entrada de `_DoAntSim` (estado **antes** de um tick); não anda se já está lá |
| `tick_saida` | da entrada até o retorno de `_DoAntSim`: exatamente um tick (estado **depois**) |
| `ate_gametime N` | para na entrada de `_DoAntSim` com `GameTime` (ainda não somado) ≥ N, tick a tick |
| `passo_tick DIR` | um tick: `DIR/estado_antes.bin`, `estado_depois.bin`, `vida_antes.bin`, `vida_depois.bin`, `extras.json` |
| `fim` | `quit` limpo |

`NOME` pode ser: um símbolo do layout do save (`GameTime`, `MapA`, ...; tamanho = count × tamanho); qualquer variável dos segmentos 8, 9 e 10 do `re_trabalho/symbols.tsv` (com ou sem `_`; exige `TAM`); um endereço `SEG:OFF` em hexadecimal (`10:cbf2`; exige `TAM`); ou um apelido do script: `SementeS` = `10:cbf2` (2 bytes, estado do LFSR) e `SementeR` = `10:ae34` (4 bytes, estado do `rand` do C), ver [spec/regras/rng.md](spec/regras/rng.md).

### Snapshots

O `savevm` exige que todo disco gravável suporte snapshots, e a `c.img` é raw. Por isso, quando o roteiro usa `snapshot_*` (ou quando se passa `--overlay`), o script cria, se não existir, `re_trabalho/qemu/c_overlay.qcow2` (`qemu-img create -f qcow2 -b c.img -F raw`) e abre o disco por ele. O disquete é sempre aberto só leitura (`readonly=on`). O snapshot interno guarda CPU, RAM, dispositivos e o estado do disco **dentro do overlay**.

- A `c.img` não muda nesse modo: os arquivos que o jogo grava ficam no overlay. Para tirá-los: `qemu_bancada.py extrair CAMINHO DESTINO --overlay ARQ` (converte o overlay para raw num diretório temporário e usa `fat16_extrair`).
- O overlay só vale sobre a `c.img` de onde nasceu: `preparar` apaga o overlay padrão. Se a `c.img` for alterada por um roteiro sem overlay, apague o overlay também.
- O `loadvm` exige a mesma máquina: um snapshot gravado com `--mem 16` não serve para `--mem 32`.
- Medido: `savevm` e `loadvm` de 16 MB levam menos de 1 s. O roteiro que parte do snapshot `quick` está no Quick Game no instante 0, contra ~55 s de boot.

## Leitura e escrita da memória do jogo

### O problema

O WfW 3.11 roda em modo 386 avançado: há paginação, e cada segmento do executável NE é um **seletor da LDT** cuja base é um endereço **linear**. Na RAM física (`pmemsave`) os segmentos não são contíguos, e o `_MapA` (8 KB) atravessa páginas: num dump físico feito logo depois do save, os 8 KB do `_MapA` gravado não aparecem inteiros em lugar nenhum.

### A técnica adotada (`Memoria` em `tools/qemu_bancada.py`)

1. **Seletores pela tabela do save.** O `_SaveGame` usa a tabela de `DS(8):0x9570`, com entradas `{u16 count, u16 tamanho, u16 offset, u16 seletor}` ([spec/formatos/save-ant.md](spec/formatos/save-ant.md)). O carregador NE do Windows corrige os ponteiros far com os **seletores reais** dos segmentos 8, 9 e 10. O script faz um `pmemsave` da RAM inteira e procura a assinatura das 6 primeiras entradas (count, tamanho e offset conhecidos; seletor curinga). São 48 bytes, que quase sempre estão numa página só e aparecem contíguos na RAM física. Daí saem os seletores.
2. **Base linear pela LDT.** `info registers` (HMP) dá a base linear e o limite da LDT (`LDT=...`). O descritor do seletor (8 bytes em `base_LDT + (seletor & ~7)`, lidos com `memsave`) dá a base linear e o limite do segmento.
3. **Validação.** Pela base linear do segmento 8, lê a tabela inteira (307 entradas) em `0x9570` e confere count, tamanho, offset e a coerência dos seletores. É aí que aparece o seletor do segmento 9, que não está nas 6 primeiras entradas. Só então os seletores são aceitos.
4. **Leitura:** `memsave` do QMP com endereço linear (`base + offset`), traduzido pela paginação atual da CPU. O `estado` lê cada segmento de uma vez (3 `memsave`), com a VM parada durante a leitura, e monta os blocos na ordem do layout.
5. **Escrita:** gdbstub do QEMU (`-gdb tcp:127.0.0.1:PORTA`, sempre ligado) e um cliente RSP mínimo em Python: pacote `M endereço,tamanho:hex` (endereço linear) e releitura com `m`. Abrir a conexão para a VM. O script fecha o socket **sem** `D` (o detach retomaria a VM na hora) e restaura pelo QMP o estado anterior (rodando ou pausada). Assim, `pausar` + `escrever_variavel` + `estado` não deixa a simulação andar entre a escrita e a leitura.
6. A base de cada segmento é relida a cada acesso (segmentos de dados do Win16 podem ser móveis). Os seletores ficam em cache até um `snapshot_carregar`.

Seletores observados (2026-10-16, `-m 16`, no boot e depois do `loadvm`): segmento 8 = `0x1537`, 9 = `0x152f`, 10 (DGROUP) = `0x1527`; tabela no endereço físico `0x246c70`. Valem só para essa configuração: o script sempre procura de novo.

Alternativas descartadas: procurar o conteúdo do `_MapA` na RAM física (atravessa páginas e muda a cada tick); fixar seletores (dependem da ordem de carga e da memória); ler o `DS` do processo pelos registradores (no instante da pausa a CPU raramente está no código do jogo). O HMP não tem comando de escrita em memória, por isso o gdbstub.

## O que foi verificado (2026-10-16)

| Item | Resultado |
|---|---|
| Boot até o jogo | logotipo do título completo em ~32 s após iniciar (ver abaixo); Quick Game pronto em ~55 s |
| Teclado (`send-key`) | Espaço fecha o título e o registro; Alt+F abre o menu File; Ctrl+A abre "Save As"; a digitação do nome funciona |
| Mouse (`input-send-event` relativo) | depois de `mouse_canto`, um movimento de (100, 100) põe a ponta da seta exatamente em (100, 100) |
| Clique | `clicar_em 345 228` escolhe "Quick Game" no "Select a Game" |
| Captura (`screendump`) | PNG 640×480, sem janela visível |
| Memória (`pmemsave`) | dump de 16 MB da RAM física |
| Jogo salvo | `QUICK2.ANT` com **48.386 bytes**, exatamente o previsto em `spec/formatos/save-ant.md` |
| Snapshot | `qemu_snapshot_quick.txt` grava `quick`; `qemu_estado_save.txt` e `qemu_semente.txt` partem dele |
| **`estado` = save, byte a byte** | `qemu_estado_save.txt`: Ctrl+A, `QUICK3`, Enter; com a caixa "Saved correctly" na tela, `pausar` e `estado`. O `QUICK3.ANT` extraído do overlay e o `estado` são **idênticos** nos 48.386 bytes (`estado_diff.py`: "iguais"). O save está em `tests/referencias/saves/QUICK3.ANT` |
| Leitura em tempo real | ~10,4 s (host) depois, outro `estado` difere em 70 dos 307 blocos: `GameTime` 92 → 154 (`Cycle` e `Scycle2` acompanham), `YardCycle` 91 → 153, `MapA` (13 bytes), listas `Alist*/Blist*/Rlist*`, `BoyX` 188 → 256, `DogX/Y`, `CatOn` 1 → 0, `SpidY` 205 → −1, `HealthB` 95 → 92, `TotalEggsLaidB` 4 → 6, `HistCnt` 1 → 2, entre outros |
| Escrita | `qemu_semente.txt`: `SementeS` lida 0x8a5e; com a VM pausada, escrita 0x1234, relida 0x1234 e ainda 0x1234 dois segundos depois; ~1 s após `continuar`, vale 0x6acb, que é exatamente o estado do LFSR de [spec/regras/rng.md](spec/regras/rng.md) 230 passos depois de 0x1234 (conferido em Python). `HealthB` escrito 7 com a VM rodando: relido 7, e 7 três segundos depois |
| Injeção e load | `qemu_carregar_save.txt`: disquete trocado depois do `loadvm`, `A:TESTE` carregado pelo File > Load; o estado lido logo depois do load é igual ao `QUICK2.ANT` salvo três blocos (`_GamePaused` e os `_CastePop*` recontados) |
| Casos de teste | `caso_ant.py rodar` com C-01 e C-02 (`tests/referencias/casos/`): todas as conferências passaram, parando nos ticks 128 e 96 exatos (ver `tests/referencias/README.md`) |

Ritmo (relógio do host, sem determinismo): `GameTime` andou 62 ticks em ~10,4 s, cerca de 6 ticks/s. **Explicado (2026-10-18):** não é lentidão do TCG; é o laço de `MYTIMERFUNC` com o temporizador de 55 ms do Windows 3.1, que roda 1 tick a cada 3 mensagens em normal. Medição por velocidade (roteiros `qemu_ritmo.txt` e `qemu_ritmo_rapido.txt`): [spec/regras/simulacao-visao-geral.md](spec/regras/simulacao-visao-geral.md) §3.1.

## Casos de teste: injetar um save e rodar

Ferramentas:

- `tools/ant_editar.py`: lê e grava `.ANT` pelo layout (blocos, escalares, células de mapa, listas de formigas com `ListIndex`, patch em JSON). Testes: `python -m unittest discover -s tools -p "test_*.py"`.
- `tools/caso_ant.py rodar tests/referencias/casos/ID.json`: monta o `.ANT` do caso, injeta, carrega no original, roda até o `GameTime` alvo, pausa, grava o estado e confere. Saída em `re_trabalho/casos/ID/` (`entrada.ANT`, `carregado.ANT` = estado logo depois do load, `estado.ANT` = estado final, telas `t1`–`t4`, `roteiro.txt`, `resultado.json`). Código de saída 0 se todas as conferências passam.
- Formato dos casos e das conferências: docstrings dos dois scripts; lista dos casos em `tests/referencias/README.md`.

### Injeção: troca do disquete do A:

O save vai num **disquete FAT12** (cópia do `boot.img` da bancada com o `.ANT` acrescentado por `tools/fat12.py`; `caso_ant.py disquete SAIDA.img ARQ.ANT=NOME.ANT` monta um à mão). Depois do `snapshot_carregar quick`, o comando `disquete` troca a mídia do drive A: e o jogo abre `A:NOME` pelo diálogo de Load.

Por que essa e não as outras opções:

| Opção | Problema |
|---|---|
| Gravar o arquivo no C: (FAT16 em Python sobre o raw convertido do overlay, ou `IMGMOUNT`+`COPY` do DOSBox-X) | o `loadvm` volta o disco do overlay ao estado do snapshot, apagando o arquivo; e o DOS/Windows restaurado tem FAT e diretório em cache (`BUFFERS`), incoerentes com um disco mudado por fora. Só serviria com boot completo (~55 s por caso) e com uma conversão qcow2 → raw → qcow2 a cada caso |
| Segundo drive (`-drive if=floppy,index=1` = B:) | muda a máquina: o snapshot `quick` foi gravado sem o B:, e o DOS que está na RAM do snapshot detectou um drive só no boot [hipótese, não testado: com um drive só, o DOS trata B: como fantasma do A: e pede troca de disquete]; exigiria boot completo |
| **Trocar a mídia do A:** | não muda a máquina nem o C:; o snapshot continua valendo; o DOS percebe a troca pela linha de troca de disco do controlador. Funcionou na primeira tentativa |

O disquete novo mantém os arquivos do disquete de boot (kernel, `COMMAND.COM`), caso o DOS precise recarregar o interpretador. Cada execução do `caso_ant.py` copia o overlay padrão (com o snapshot `quick`) para `re_trabalho/casos/ID/ov.qcow2`: o overlay padrão e o snapshot não são tocados.

### Carregar um save injetado (passos observados em 2026-10-16)

Roteiro de referência: `tests/referencias/roteiros/qemu_carregar_save.txt`. A partir do snapshot `quick` (Quick Game rodando):

1. `teclas ctrl+l` (File > Load). Aparece a caixa "SimAnt Load: Do you want to save the current ant hill?" (Yes / No / Cancel).
2. `teclas n` (No). Abre o diálogo comum "Load Ant Hill": campo File Name com o foco, diretório `c:\maxis\antwin`, lista `*.ANT`, drive `c:`.
3. Digitar `A:NOME` (`digitar A`, `teclas shift+semicolon`, `digitar NOME`; o `digitar` só tem letras, números e espaço). Sem extensão, o diálogo completa `.ANT`. Na primeira tentativa, sem o passo 2, as letras do nome foram tomadas pela caixa do passo 1 (o "N" respondeu No) e o diálogo recebeu só parte do nome ("t.ant": "Cannot find this file").
4. `teclas ret`. Em ~5 s o jogo está carregado: título da janela = nome do arquivo (`TESTE.ANT`), e o jogo fica **pausado** (barra de mensagem "Game Paused", `_GamePaused` = 1, mesmo com 0 no arquivo).
5. `teclas shift+0` (Speed > Unpause; o menu Speed mostra Unpause Shift+0, Slow Shift+1, Normal Shift+2, Fast Shift+3, Ultra Shift+4).
6. `esperar_variavel GameTime ALVO`, `pausar`, `estado ARQ`.

O que o load muda (QUICK2.ANT carregado e lido logo depois, ainda pausado; `estado_diff.py`): só 3 blocos. `_GamePaused` 0 → 1, e `_CastePopB`/`_CastePopR` são **recontados** pelas listas (`[2,2,0,0,0,1]` → `[3,2,0,0,0,1]` e `[3,1,…]` → `[4,1,…]`: os ovos postos depois da última contagem). Consequência para os casos: `_BpopT`/`_RpopT` e `_CastePop*` não podem ser forçados pelo arquivo; mude as listas. O `_GameTime` e o `_Cycle` ficam como no arquivo.

Ritmo medido: do load ao tick alvo, ~6 ticks/s (78 → 96 em ~3 s; 78 → 128 em ~8 s). Com `esperar_variavel` (consulta a cada 0,05 s) seguido de `pausar`, as três execuções pararam exatamente no tick alvo (96, 128 e 90). Uma execução completa de caso leva ~25 s mais o tempo de simulação.
## Modo tick (breakpoints do gdbstub)

Para e avança a simulação exatamente um tick por vez. Detalhes, experimentos e protocolo de teste: [spec/regras/testes-tick.md](spec/regras/testes-tick.md).

- **Seletor do código.** `_DoAntSim` está em `1028:0000` (segmento de código 6 do NE). Como os de dados, o seletor só existe em tempo de execução, mas não está na tabela do save: `Memoria.seletor_codigo(6)` procura na LDT os descritores de código presentes com limite igual ao tamanho do segmento no NE e compara os 64 primeiros bytes com o `SIMANTW.EXE`. Observado: `0x1547` (base linear `0x80837500`); os 10 segmentos ficam em `0x156f` (1) … `0x1527` (10), decrescendo de 8 em 8.
- **Breakpoints em endereço linear** (`Z0`; o QEMU compara com base do CS + EIP): na entrada de `_DoAntSim` e no retorno far lido da pilha na entrada (`1000:2688`, dentro do `MYTIMERFUNC`). Cada um é ligado só durante a corrida até ele.
- **Sessão persistente.** A conexão com o gdbstub fica aberta; `escrever_variavel` passa a usá-la. Com a VM rodando, o script interrompe pelo gdbstub (byte 0x03) antes de qualquer comando.
- **Watchpoints de leitura** (`Z3`) em `Depurador.mapear_leituras`: o QEMU informa o início do watchpoint atingido, não o byte lido; por isso as faixas são cortadas em pedaços.
- **Ritmo:** ~0,45 s por `passo_tick`; `ate_gametime` avança ~3 ticks/s (a VM livre faz ~6).
- O jogo precisa estar rodando a simulação (não pausado nem minimizado); sem parada em 120 s, o script interrompe e desiste.

## Logotipo com a metade superior branca (investigado)

Sintoma (captura antiga `re_trabalho/q30.png`, aos 30 s): no título, a parte de cima do retângulo do logotipo aparece branca e só a de baixo desenhada.

Teste: `qemu_titulo_logo.txt` captura a tela a cada ~1,4 s, de 24 s a 49 s, com `--mem 16` e com `--mem 32`. Sequência observada nos dois casos (com 32 MB, tudo ~1 s depois):

1. janela do título com o retângulo do logotipo todo branco;
2. área cliente toda preta (o jogo apaga a janela);
3. retângulo branco de novo;
4. o bitmap é pintado **de baixo para cima**: numa captura (31,2 s com 16 MB; 32,4 s com 32 MB) as linhas de cima (até y ≈ 270, ou 340) ainda estão brancas e as de baixo já têm a formiga;
5. a partir de ~32,5 s (16 MB) ou ~33,7 s (32 MB) o logotipo está **completo** e assim fica até o fim (49 s), com a mesma forma do capturado no DOSBox-X.

**Causa:** não é memória, nem a tabela de posições 9/0, nem defeito do driver VGA. É um estado transitório: o bitmap (DIB, guardado de baixo para cima) é desenhado devagar no TCG com o driver VGA de 16 cores (que converte e pontilha as cores), e a captura antiga caiu no meio do desenho. Para capturar o título, espere pelo menos ~35 s após o boot (o `qemu_entrada.txt`, com 35 s, já pega o logotipo completo). As cores diferem das do DOSBox-X (16 cores pontilhadas contra 256 cores), então a comparação pixel a pixel do título continua sendo feita só no DOSBox-X.

## Argumentos extras do QEMU: `-icount` e placa de som (2026-10-18)

`roteiro ... --qemu-args "ARGS"` (ou a variável `SIMANT_QEMU_ARGS`, que vale também para `cenarios_tick.py` e `gerar_vetores_tick.py`, que criam a `Bancada` sem argumentos) acrescenta argumentos à linha do QEMU.

- **`-icount shift=N,sleep=on`** liga o relógio da VM ao número de instruções (2^N ns por instrução). Um snapshot gravado sem `-icount` carrega, mas a simulação fica parada (`GameTime` e `_gameCycles` não andam em 60 s); é preciso boot completo (`tools/ritmo_icount.py`). O TCG com `icount` emula só ~3–8 milhões de instruções por segundo do host, então a VM fica atrás do relógio do host: o tempo da VM tem de ser medido dentro dela. O contador de tiques do BIOS (`0040:006C`, lido no endereço linear `0x46C` com `Memoria.ler_linear`) serve: em normal dá **6,07 ticks/s** em tempo da VM com shift 5, 6 e 7 (o modelo dá 6,06), contra 1,5–3,7 ticks/s no relógio do host. Resultado das medições: [spec/regras/simulacao-visao-geral.md](spec/regras/simulacao-visao-geral.md) §3.3.
- **Som:** `-audiodev wav,... -device sb16,audiodev=...,irq=7 -parallel none [-device adlib,...]`; a gravação funciona, mas não serve de referência (OPL2 contra driver de OPL3, silêncios descartados, efeitos que não batem): [spec/regras/som.md](spec/regras/som.md), "Áudio do original na bancada QEMU". Também exige boot completo.

## Cuidados

- O DOS grava com atraso. Antes de encerrar, espere uns 15 s depois de salvar; o `fim` já usa `quit`. O primeiro save (`QUICK1.ANT`) saiu com 0 bytes porque o QEMU foi morto logo depois de salvar.
- O disco `c.img` é persistente entre execuções. Use `preparar` para voltar ao estado limpo, ou nomes de arquivo novos. Com overlay, `snapshot_carregar` também volta o disco ao estado do snapshot.
- Os tempos são do relógio do host (o QEMU não está em modo determinístico). `pausar` congela a VM para ler um estado coerente, mas o número de ticks entre dois comandos varia de uma execução para outra. Para determinismo, use o modo tick: parando na entrada de `_DoAntSim`, o original é reprodutível tick a tick (exceto `_BirdDelay`), sem precisar de `icount` (ver [spec/regras/testes-tick.md](spec/regras/testes-tick.md)).
- `ler_variavel`, `escrever_variavel` e `estado` só funcionam com o jogo carregado (a tabela do save só existe com o `SIMANTW.EXE` na memória). Antes disso, falham com "tabela do save não encontrada".
