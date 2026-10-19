"""Bancada de testes do SimAnt original em QEMU, controlada só por QMP (sem janela, sem usar mouse/teclado do host).

Preparação (uma vez):  python tools/qemu_bancada.py preparar
Roteiro:               python tools/qemu_bancada.py roteiro ARQUIVO [--overlay ARQ.qcow2] [--mem MB] [--qemu-args "ARGS"]
Extrair do disco:      python tools/qemu_bancada.py extrair CAMINHO_DOS DESTINO [--overlay ARQ.qcow2] [--disco ARQ.img]
Disco SVGA (VBE):      python tools/qemu_bancada.py preparar_svga LARGURA ALTURA [BPP] [--disco ARQ.img] [--drivers DIR] [--ajuda-padrao]
    Disco C: à parte (padrão re_trabalho/qemu/svga_c_LARGURAxALTURA.img) com o driver livre vbesvga.drv
    (github.com/PluMGMK/vbesvga.drv) sobre o VBE do -vga std; não toca a c.img nem o overlay padrão.
    Rodar com ele: `roteiro ARQUIVO --disco ARQ.img` (ou com `--overlay svga_X.qcow2`, que nasce sobre o --disco).
    --ajuda-padrao: tira da cópia do WIN.INI a posição guardada do WinHelp ([Windows Help] H_WindowPosition),
    para o visor de ajuda abrir no lugar padrão do Windows, como na bancada VGA (telas/resolucao.md).

Comandos de roteiro (um por linha, # = comentário):
    esperar SEG                 espera SEG segundos (relógio do host)
    teclas T1 T2 ...            teclas QMP (qcodes: esc, ret, spc, alt, f, ...); '+' combina: alt+f
    digitar TEXTO               digita letras/números/espaço
    mouse_canto                 leva o cursor ao canto superior esquerdo (movimento relativo grande)
    mover DX DY                 movimento relativo (em mickeys; com aceleração desligada, 1 = 1 pixel)
    clicar                      clique esquerdo
    clicar_em X Y               leva o cursor a (X, Y) da tela emulada (canto + movimento exato) e clica
    ir_para X Y                 só leva o cursor a (X, Y)
    apertar / soltar            segura / solta o botão esquerdo do mouse
    apertar_direito / soltar_direito   segura / solta o botão direito
    clicar_direito              clique direito
    clicar_direito_em X Y       leva o cursor a (X, Y) e dá um clique direito
    apertar_tecla K / soltar_tecla K   segura / solta a tecla K (qcode do QMP: ctrl, shift...)
    capturar ARQUIVO.png        screendump da tela emulada
    memoria INICIO TAMANHO ARQ  pmemsave (endereço físico)
    pausar / continuar          para / retoma a CPU emulada (QMP stop / cont)
    snapshot_salvar NOME        savevm (exige disco em overlay qcow2; ver abaixo)
    snapshot_carregar NOME      loadvm
    ler_variavel NOME ARQ [TAM] grava em ARQ os bytes da variável global NOME (nome do .SYM, com ou sem '_')
    escrever_variavel NOME VALOR [TAM]  grava VALOR (inteiro little-endian, ou hex:AABB...) via gdbstub
    estado ARQ                  dump de todos os blocos de data/save_layout.json, no formato de um .ANT
    disquete ARQ.img            troca a mídia do drive A: (blockdev-change-medium, só leitura); ver tools/caso_ant.py
    esperar_variavel NOME ALVO [TIMEOUT]  espera (consultando a cada 0,05 s) até NOME (sem sinal) >= ALVO
    extras ARQ.json             variáveis fora do save (EXTRAS: sementes S e R, ...), GameTime e posição
    segmentos PREFIXO           segmentos de dados 8, 9 e 10 inteiros -> PREFIXO_seg8.bin, ...
  modo tick (gdbstub; a VM fica parada entre os comandos; 'continuar' volta a rodar livre):
    tick_entrada                roda até a entrada de _DoAntSim (1028:0000); estado ANTES de um tick
    tick_saida                  da entrada até o retorno de _DoAntSim: exatamente um tick
    ate_gametime N              para na entrada de _DoAntSim com GameTime (ainda não somado) >= N
    passo_tick DIR              DIR/estado_antes.bin, DIR/estado_depois.bin, DIR/extras.json de um tick
    fim                         encerra o QEMU

Snapshots: se o roteiro usa snapshot_*, ou se --overlay é dado, o disco C: é aberto por um overlay qcow2
(padrão re_trabalho/qemu/c_overlay.qcow2, criado com qemu-img sobre c.img, que fica só leitura). O savevm
grava CPU, RAM, dispositivos e o estado do overlay dentro do próprio overlay; os saves feitos pelo jogo
também ficam no overlay (use `extrair ... --overlay`). O disquete é aberto só leitura.

Memória do jogo: ver Memoria.localizar() e docs/bancada-qemu.md.
Variáveis de ambiente: SIMANT_RAIZ (raiz do repositório com referencia/ e re_trabalho/; padrão: a pasta
acima de tools/).

Ver docs/decisoes.md (bancada QEMU) e tests/referencias/README.md.
"""
import json
import os
import re
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import shlex
import time
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fat12 import Fat12  # noqa: E402

REPO = Path(__file__).resolve().parent.parent  # dados versionados (data/) vêm do repositório do script
RAIZ = Path(os.environ.get("SIMANT_RAIZ") or REPO)  # referencia/ e re_trabalho/ (fora do git)
QEMU = RAIZ / "referencia" / "qemu" / "qemu-system-i386.exe"
TRAB = RAIZ / "re_trabalho" / "qemu"
DISCO_C = TRAB / "c"
DISQUETE = TRAB / "boot.img"
IMAGEM_C = TRAB / "c.img"
DOSBOX = RAIZ / "referencia" / "dosbox-x" / "bin" / "x64" / "Release" / "dosbox-x.exe"
FREEDOS = RAIZ / "re_trabalho" / "downloads" / "freedos" / "floppy" / "144m" / "x86BOOT.img"
# Kernel FreeDOS 2046 (github.com/FDOS/kernel, ke2046_86f16.zip): suporta o modo 386 avançado do Windows 3.x.
KERNEL_NOVO = RAIZ / "re_trabalho" / "downloads" / "freedos" / "ke2046_86" / "bin" / "kernel.sys"
QEMU_IMG = RAIZ / "referencia" / "qemu" / "qemu-img.exe"
OVERLAY_PADRAO = TRAB / "c_overlay.qcow2"
SIMBOLOS = RAIZ / "re_trabalho" / "symbols.tsv"
EXE = RAIZ / "SimAnt-T" / "MAXIS" / "ANTWIN" / "SIMANTW.EXE"
LAYOUT = REPO / "data" / "save_layout.json"
# Variáveis sem nome no .SYM, identificadas no código (docs/spec/regras/rng.md): nome -> (SEG:OFF, bytes).
APELIDOS = {
    "SementeS": ("10:cbf2", 2),  # estado do LFSR _SRand* (16 bits)
    "SementeR": ("10:ae34", 4),  # estado do rand() do Microsoft C usado por _RRand (32 bits)
}

# SvarDOS (kernel EDR-DOS + SvarCOM), disquete oficial svardos-20250427-floppy-1.44M. É o DOS padrão da
# bancada: roda o WfW 3.11 em modo 386 avançado com device=vshare.386 desligado (ver o relatório
# reports/Automação de testes Win16 emulado.md). O FreeDOS fica como alternativa (DOS_BANCADA = "freedos").
SVARDOS = RAIZ / "re_trabalho" / "downloads" / "svardos" / "floppy" / "svdos-1.44M-disk-1.img"
DOS_BANCADA = "svardos"

FDCONFIG = "\r\n".join([
    "FILES=40",
    "BUFFERS=30",
    "DEVICE=C:\\WINDOWS\\HIMEM.SYS",
    "SHELL=A:\\COMMAND.COM A:\\ /E:1024 /P=A:\\AUTOEXEC.BAT",
    "",
])
CONFIG_SVARDOS = "\r\n".join([
    "FILES=40",
    "BUFFERS=30",
    "DEVICE=A:\\HIMEMX.EXE",
    "SHELL=A:\\COMMAND.COM /E:1024 /P",
    "",
])
AUTOEXEC = "\r\n".join([
    "@ECHO OFF",
    "C:",
    "SET PATH=C:\\WINDOWS",
    "SET TEMP=C:\\WINDOWS\\TEMP",
    "CD \\WINDOWS",
    # O Windows instalado é o for Workgroups 3.11, que só roda em modo 386 avançado.
    "WIN C:\\MAXIS\\ANTWIN\\SIMANTW.EXE",
    "",
])


def gravar_disquete():
    """Disquete de boot mínimo (kernel, interpretador, configuração que inicia o jogo)."""
    TRAB.mkdir(parents=True, exist_ok=True)
    if DOS_BANCADA == "svardos":
        import io
        import zipfile
        modelo = Fat12(SVARDOS.read_bytes())
        himemx = zipfile.ZipFile(io.BytesIO(modelo.ler("HIMEMX.SVP"))).read("BIN/HIMEMX.EXE")
        d = Fat12.vazio_a_partir_de(modelo)  # mantém o setor de boot do SvarDOS (carrega o kernel EDR)
        d.gravar("KERNEL.SYS", modelo.ler("KERNEL.SYS"))
        d.gravar("COMMAND.COM", modelo.ler("COMMAND.COM"))
        d.gravar("HIMEMX.EXE", himemx)
        d.gravar("CONFIG.SYS", CONFIG_SVARDOS.encode("latin-1"))
    else:
        modelo = Fat12(FREEDOS.read_bytes())
        d = Fat12.vazio_a_partir_de(modelo)
        d.gravar("KERNEL.SYS", KERNEL_NOVO.read_bytes())
        d.gravar("COMMAND.COM", modelo.ler_caminho("FREEDOS/BIN/COMMAND.COM"))
        d.gravar("FDCONFIG.SYS", FDCONFIG.encode("latin-1"))
    d.gravar("AUTOEXEC.BAT", AUTOEXEC.encode("latin-1"))
    DISQUETE.write_bytes(bytes(d.img))


def substituir_ini(caminho: Path, trocas):
    """Troca linhas inteiras de um .INI preservando CRLF. trocas: {linha_antiga: linha_nova}."""
    d = caminho.read_bytes()
    for a, b in trocas.items():
        a, b = a.encode("latin-1") + b"\r\n", b.encode("latin-1") + b"\r\n"
        if d.count(a) != 1:
            raise ValueError(f"{caminho.name}: linha não encontrada uma única vez: {a!r}")
        d = d.replace(a, b)
    caminho.write_bytes(d)


def preparar():
    # Disco C: cópia de WINDOWS e MAXIS da instalação do usuário, com vídeo VGA genérico.
    if DISCO_C.exists():
        shutil.rmtree(DISCO_C)
    origem = RAIZ / "SimAnt-T"
    for pasta in ("WINDOWS", "MAXIS"):
        shutil.copytree(origem / pasta, DISCO_C / pasta)
    (DISCO_C / "WINDOWS" / "TEMP").mkdir(exist_ok=True)
    substituir_ini(DISCO_C / "WINDOWS" / "SYSTEM.INI", {
        "display.drv=S3TRIO.DRV": "display.drv=vga.drv",
        "display=VDDS3764.386": "display=*vddvga",
        # Ajustes para o modo 386 avançado sobre o kernel FreeDOS (relatos em virtuallyfun.com e svardos.org).
        "device=vshare.386": ";device=vshare.386",
        "[386Enh]": "[386Enh]\r\nInDOSPolling=TRUE",
    })
    # Sem aceleração do mouse: cada unidade de movimento relativo do QMP vira 1 pixel
    # (com a aceleração padrão, 100 viravam 200; medido em 2026-10-16).
    substituir_ini(DISCO_C / "WINDOWS" / "WIN.INI", {
        "[windows]": "[windows]\r\nMouseSpeed=0\r\nMouseThreshold1=0\r\nMouseThreshold2=0",
    })
    gravar_disquete()
    criar_imagem_c()
    print(f"preparado: {IMAGEM_C} e {DISQUETE}")


def criar_imagem_c(pasta=DISCO_C, imagem=IMAGEM_C, overlay=OVERLAY_PADRAO, silencioso=False):
    """Cria um disco FAT16 de 100 MB com o conteúdo de `pasta` usando IMGMAKE/COPY do DOSBox-X.
    (O vvfat do QEMU em modo leitura-escrita falha quando o Windows grava no disco.)
    `overlay`: overlay qcow2 nascido da imagem antiga, apagado junto (None: nenhum).
    `silencioso`: DOSBox-X com -silent (sem janela)."""
    if imagem.exists():
        imagem.unlink()
    if overlay is not None and overlay.exists():  # o overlay (e seus snapshots) só vale sobre a imagem de onde nasceu
        overlay.unlink()
    rel =lambda p: str(p.relative_to(RAIZ))  # noqa: E731
    pastas = sorted(str(p.relative_to(pasta)) for p in pasta.rglob("*") if p.is_dir())
    linhas = ["[sdl]", "output = surface", "[autoexec]", f'mount d "{rel(pasta)}"',
              f"imgmake {rel(imagem)} -t hd -size 100", f"imgmount e {rel(imagem)}"]
    barra = "\\"
    linhas += [f"md E:{barra}{p}" for p in pastas]
    linhas += [f"copy D:{barra}{p}{barra}*.* E:{barra}{p}" for p in pastas
               if any(f.is_file() for f in (pasta / p).iterdir())]
    linhas += ["exit"]
    conf = TRAB / ("criar_imagem.conf" if imagem == IMAGEM_C else f"criar_{imagem.stem}.conf")
    conf.write_text("\n".join(linhas) + "\n", encoding="latin-1")
    subprocess.run([str(DOSBOX), "-conf", str(conf), "-nopromptfolder"] + (["-silent"] if silencioso else []),
                   cwd=RAIZ, timeout=300)


# Driver de vídeo livre para VBE (github.com/PluMGMK/vbesvga.drv, release v1.0-beta4, vbesvga-release.zip
# extraído). Só os arquivos do Windows vão para a imagem da VM; nada dele roda no host.
DRIVERS_VBESVGA = RAIZ / "re_trabalho" / "downloads" / "vbesvga" / "x"
ARQUIVOS_VBESVGA = ("VBESVGA.DRV", "VDDVBE.386", "VBEVMDIB.3GR")


def preparar_svga(largura, altura, bpp=8, imagem=None, drivers=DRIVERS_VBESVGA, ajuda_padrao=False):
    """Disco C: à parte com o driver vbesvga.drv em LARGURA×ALTURA×BPP sobre o VBE do `-vga std`.
    Não toca DISCO_C, IMAGEM_C nem o overlay padrão; usa a pasta TRAB/svga_c_LxA e a imagem
    TRAB/svga_c_LxA.img (ou `imagem`). Overlays sobre a imagem antiga ficam inválidos: apague-os."""
    nome = f"svga_c_{largura}x{altura}" + ("" if bpp == 8 else f"x{bpp}")
    imagem = caminho_raiz(imagem) if imagem else TRAB / f"{nome}.img"
    pasta = TRAB / imagem.stem
    if not pasta.name.startswith("svga"):
        raise ValueError("a imagem SVGA deve ter nome começando com 'svga' (protege c.img e c/)")
    if pasta.exists():
        shutil.rmtree(pasta)
    origem = RAIZ / "SimAnt-T"
    for sub in ("WINDOWS", "MAXIS"):
        shutil.copytree(origem / sub, pasta / sub)
    (pasta / "WINDOWS" / "TEMP").mkdir(exist_ok=True)
    for arq in ARQUIVOS_VBESVGA:
        shutil.copyfile(Path(drivers) / arq, pasta / "WINDOWS" / "SYSTEM" / arq)
    ini = pasta / "WINDOWS" / "SYSTEM.INI"
    substituir_ini(ini, {
        "display.drv=S3TRIO.DRV": "display.drv=vbesvga.drv",
        "386grabber=VGA_ENG.3GR": "386grabber=vbevmdib.3gr",
        "display=VDDS3764.386": "display=vddvbe.386",
        "device=vshare.386": ";device=vshare.386",
        "[386Enh]": "[386Enh]\r\nInDOSPolling=TRUE",
    })
    with open(ini, "ab") as f:
        f.write(f"\r\n[VBESVGA.DRV]\r\nWidth={largura}\r\nHeight={altura}\r\nDepth={bpp}\r\n".encode("latin-1"))
    substituir_ini(pasta / "WINDOWS" / "WIN.INI", {
        "[windows]": "[windows]\r\nMouseSpeed=0\r\nMouseThreshold1=0\r\nMouseThreshold2=0",
    })
    if ajuda_padrao:
        win = pasta / "WINDOWS" / "WIN.INI"
        linhas = win.read_bytes().split(b"\r\n")
        win.write_bytes(b"\r\n".join(lin for lin in linhas if not lin.startswith(b"H_WindowPosition=")))
    if not DISQUETE.exists():
        gravar_disquete()
    criar_imagem_c(pasta, imagem, None, silencioso=True)
    print(f"preparado: {imagem} ({largura}x{altura}x{bpp}, vbesvga.drv)")


def porta_livre():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class Qmp:
    def __init__(self, porta):
        for _ in range(100):
            try:
                self.s = socket.create_connection(("127.0.0.1", porta), timeout=120)
                break
            except OSError:
                time.sleep(0.2)
        else:
            raise RuntimeError("QMP não respondeu")
        self.buf = b""
        self._ler()  # saudação
        self.cmd("qmp_capabilities")

    def _ler(self):
        while b"\n" not in self.buf:
            self.buf += self.s.recv(65536)
        linha, self.buf = self.buf.split(b"\n", 1)
        return json.loads(linha)

    def cmd(self, nome, **args):
        self.s.sendall(json.dumps({"execute": nome, "arguments": args}).encode() + b"\n")
        while True:
            r = self._ler()
            if "return" in r:
                return r["return"]
            if "error" in r:
                raise RuntimeError(f"QMP {nome}: {r['error']}")

    def hmp(self, linha):
        """Comando do monitor humano (info registers, savevm, loadvm, ...). Devolve o texto."""
        return self.cmd("human-monitor-command", **{"command-line": linha})

    def rodando(self):
        return self.cmd("query-status")["running"]


class Gdb:
    """Cliente mínimo do protocolo remoto do gdb (RSP) para o gdbstub do QEMU (-gdb tcp:...).
    Ao abrir a conexão o QEMU para a VM; 'D' (detach) a retoma."""

    def __init__(self, porta):
        self.s = socket.create_connection(("127.0.0.1", porta), timeout=30)
        self.buf = b""

    def _proximo(self):
        """Próximo pacote recebido (sem o enquadramento $...#cs); confirma com '+'."""
        while True:
            i = self.buf.find(b"$")
            j = self.buf.find(b"#", i) if i >= 0 else -1
            if i >= 0 and j >= 0 and len(self.buf) >= j + 3:
                resp = self.buf[i + 1:j].decode()
                self.buf = self.buf[j + 3:]
                self.s.sendall(b"+")
                return resp
            d = self.s.recv(65536)
            if not d:
                raise RuntimeError("gdbstub fechou a conexão")
            self.buf += d

    @staticmethod
    def _e_parada(resp):
        # respostas de parada (T05 breakpoint, T02 quando o QMP 'stop' para a VM, W/X fim); nunca são
        # resposta de 'm' (hexadecimal), 'M' ou 'Z' (OK, E.., vazio)
        return resp[:1] in ("T", "S", "W", "X")

    def drenar(self):
        """Descarta o que chegou sem ser pedido (paradas assíncronas causadas por 'stop' do QMP)."""
        self.s.settimeout(0.05)
        try:
            while True:
                d = self.s.recv(65536)
                if not d:
                    break
                self.buf += d
        except (socket.timeout, BlockingIOError):
            pass
        finally:
            self.s.settimeout(30)
        self.buf = b""

    def _pacote(self, dados: str) -> str:
        cs = sum(dados.encode()) & 0xFF
        self.s.sendall(f"${dados}#{cs:02x}".encode())
        while True:
            resp = self._proximo()
            if not self._e_parada(resp):
                return resp

    def breakpoint(self, endereco, ligar=True):
        """Breakpoint de software do gdbstub (no TCG não altera a memória) num endereço LINEAR."""
        r = self._pacote(f"{'Z' if ligar else 'z'}0,{endereco:x},1")
        if r != "OK":
            raise RuntimeError(f"gdb: breakpoint {endereco:#x} {'Z' if ligar else 'z'} falhou ({r!r})")

    def interromper(self):
        """Para a VM pelo gdbstub (byte 0x03) e espera a resposta de parada."""
        self.drenar()
        self.s.sendall(b"\x03")
        while not self._e_parada(self._proximo()):
            pass

    def watchpoint(self, endereco, n, ligar=True, tipo=3):
        """Watchpoint do gdbstub (tipo 2 escrita, 3 leitura, 4 acesso) em n bytes a partir de um endereço LINEAR."""
        r = self._pacote(f"{'Z' if ligar else 'z'}{tipo},{endereco:x},{n:x}")
        if r != "OK":
            raise RuntimeError(f"gdb: watchpoint {endereco:#x}+{n} falhou ({r!r})")

    def continuar_ate_parar(self, timeout=120):
        """'c' e espera a resposta de parada. Devolve o pacote (ex. 'T05thread:01;')."""
        self.drenar()
        self.s.sendall(b"$c#63")
        self.s.settimeout(timeout)
        try:
            while True:
                resp = self._proximo()
                if self._e_parada(resp):
                    return resp
        except socket.timeout:
            self.s.settimeout(30)
            self.s.sendall(b"\x03")  # interrompe
            while not self._e_parada(self._proximo()):
                pass
            raise RuntimeError(f"gdb: nenhuma parada em {timeout} s (jogo pausado ou minimizado?)")
        finally:
            self.s.settimeout(30)

    def ler(self, endereco, n):
        r = self._pacote(f"m{endereco:x},{n:x}")
        if r.startswith("E"):
            raise RuntimeError(f"gdb: leitura em {endereco:#x} falhou ({r})")
        return bytes.fromhex(r)

    def escrever(self, endereco, dados: bytes):
        r = self._pacote(f"M{endereco:x},{len(dados):x}:{dados.hex()}")
        if r != "OK":
            raise RuntimeError(f"gdb: escrita em {endereco:#x} falhou ({r})")

    def desconectar(self):
        # Fecha sem 'D' (detach): o 'D' retomaria a VM na hora, e uma VM pausada andaria alguns
        # milissegundos antes de ser parada de novo. Quem chama restaura o estado pelo QMP.
        self.s.close()


def carregar_simbolos():
    """{nome sem '_': (segmento NE, offset)} das variáveis dos segmentos de dados 8, 9 e 10."""
    out = {}
    for linha in SIMBOLOS.read_text(encoding="latin-1").splitlines():
        p = linha.split("\t")
        if len(p) >= 3 and p[0] in ("8", "9", "10"):
            out.setdefault(p[2].lstrip("_"), (int(p[0]), int(p[1])))
    return out


def carregar_layout():
    blocos = json.loads(LAYOUT.read_text(encoding="utf-8"))["blocos"]
    for b in blocos:
        seg, off = b["endereco"].split(":")
        b["seg"], b["off"], b["n"] = int(seg), int(off, 16), b["count"] * b["tamanho"]
    return blocos


class Memoria:
    """Acesso às variáveis globais do SimAnt na VM.

    Técnica (ver docs/bancada-qemu.md):
    1. O WfW 3.11 em modo 386 avançado usa paginação; cada segmento NE do jogo é um seletor da LDT
       com base LINEAR. Os seletores reais só existem em tempo de execução.
    2. A tabela do save (segmento 8, offset 0x9570; entradas {count, tamanho, offset, seletor}) é corrigida
       pelo carregador NE com os seletores reais. Procuramos na RAM física (pmemsave) a assinatura das
       primeiras entradas (offsets/tamanhos de data/save_layout.json, seletores curinga) e lemos dela o
       seletor de cada segmento de dados (8, 9 e 10 = DGROUP).
    3. A base linear de cada seletor vem do descritor na LDT (base da LDT em 'info registers').
    4. Lemos/escrevemos por endereço linear: QMP memsave (leitura) e gdbstub 'M' (escrita), ambos
       traduzidos pela paginação atual da CPU. A base é relida a cada acesso (segmentos móveis).
    Validação: base(seg 8) + 0x9570 tem de conter a tabela inteira, com seletores coerentes.
    """

    def __init__(self, q: Qmp, porta_gdb, ram_mb):
        self.q, self.porta_gdb, self.ram = q, porta_gdb, ram_mb << 20
        self.seletores = None
        self.seletores_codigo = {}
        self.gdb = None  # sessão persistente do gdbstub (modo tick); None = uma conexão por escrita
        self.layout = carregar_layout()
        self.tmp = Path(tempfile.mkdtemp(prefix="bancada_"))

    def esquecer_seletores(self):
        """Depois de um loadvm a sessão do Windows restaurada pode ter outros seletores."""
        self.seletores = None
        self.seletores_codigo = {}

    def sessao_gdb(self):
        """Conexão persistente com o gdbstub. Abrir a conexão para a VM (estado 'debug' do QEMU)."""
        if self.gdb is None:
            self.gdb = Gdb(self.porta_gdb)
        elif self.q.rodando():
            # com a VM rodando (ex.: 'continuar' pelo QMP), o gdbstub só aceita a interrupção (0x03)
            self.gdb.interromper()
        return self.gdb

    def registradores(self):
        """{'CS': (seletor, base), 'SS': ..., 'EIP': v, 'ESP': v} de 'info registers'."""
        regs = self.q.hmp("info registers")
        out = {}
        for nome in ("CS", "SS", "DS", "ES"):
            m = re.search(nome + r" =([0-9a-f]{4}) ([0-9a-f]{8})", regs)
            out[nome] = (int(m.group(1), 16), int(m.group(2), 16))
        for nome in ("EIP", "ESP", "EAX", "EDX"):
            out[nome] = int(re.search(nome + r"=([0-9a-f]{8})", regs).group(1), 16)
        return out

    def seletor_codigo(self, nseg):
        """Seletor do segmento de CÓDIGO nseg do SIMANTW.EXE (1 = 1000, ..., 6 = 1028, 7 = 1030).

        Os segmentos de código não aparecem na tabela do save. Procuramos na LDT inteira os descritores
        de código presentes com limite compatível com o tamanho do segmento no NE e comparamos os
        primeiros 64 bytes da memória com os do arquivo (as relocações mudam poucos bytes)."""
        if nseg in self.seletores_codigo:
            return self.seletores_codigo[nseg]
        exe = EXE.read_bytes()
        ne = struct.unpack_from("<H", exe, 0x3C)[0]
        tab, shift = (struct.unpack_from("<H", exe, ne + o)[0] for o in (0x22, 0x32))
        setor, tam, _flags, _ = struct.unpack_from("<HHHH", exe, ne + tab + (nseg - 1) * 8)
        tam = tam or 0x10000
        ref = exe[setor << shift:(setor << shift) + 64]
        base_ldt, lim_ldt = self.ldt_base()
        ldt = self.ler_linear(base_ldt, lim_ldt + 1)
        candidatos = []
        for i in range(8, len(ldt) - 7, 8):
            d = ldt[i:i + 8]
            if not d[5] & 0x80 or (d[5] & 0x18) != 0x18:  # presente e de código
                continue
            limite = d[0] | d[1] << 8 | (d[6] & 0x0F) << 16
            if not tam - 1 <= limite < tam + 0x1000:
                continue
            base = d[2] | d[3] << 8 | d[4] << 16 | d[7] << 24
            try:
                mem = self.ler_linear(base, 64)
            except RuntimeError:
                continue
            iguais = sum(a == b for a, b in zip(mem, ref))
            if iguais >= 56:
                candidatos.append((iguais, i | 7, base))
        if not candidatos:
            raise RuntimeError(f"segmento de código {nseg} não encontrado na LDT (descartado?)")
        candidatos.sort(reverse=True)
        if len(candidatos) > 1 and candidatos[1][0] == candidatos[0][0]:
            raise RuntimeError(f"segmento de código {nseg}: candidatos ambíguos {candidatos}")
        sel = candidatos[0][1]
        print(f"memória: segmento de código {nseg} = seletor {sel:#06x} (base linear {candidatos[0][2]:#010x}, "
              f"{candidatos[0][0]}/64 bytes iguais ao arquivo)", flush=True)
        self.seletores_codigo[nseg] = sel
        return sel

    def linear_codigo(self, nseg, off):
        base, _, presente = self.descritor(self.seletor_codigo(nseg))
        if not presente:
            raise RuntimeError(f"segmento de código {nseg} não presente")
        return base + off

    def ler_linear(self, endereco, n):
        f = self.tmp / "m.bin"
        self.q.cmd("memsave", val=endereco, size=n, filename=str(f), **{"cpu-index": 0})
        d = f.read_bytes()
        if len(d) != n:
            raise RuntimeError(f"memsave {endereco:#x}+{n}: {len(d)} bytes")
        return d

    def ldt_base(self):
        regs = self.q.hmp("info registers")
        m = re.search(r"LDT=([0-9a-f]{4}) ([0-9a-f]{8}) ([0-9a-f]{8})", regs)
        if not m:
            raise RuntimeError("LDT não encontrada em 'info registers'")
        return int(m.group(2), 16), int(m.group(3), 16)

    def descritor(self, seletor):
        if not seletor & 4:
            raise RuntimeError(f"seletor {seletor:#06x} não é da LDT")
        base_ldt, _ = self.ldt_base()
        d = self.ler_linear(base_ldt + (seletor & 0xFFF8), 8)
        base = d[2] | d[3] << 8 | d[4] << 16 | d[7] << 24
        limite = d[0] | d[1] << 8 | (d[6] & 0x0F) << 16
        if d[6] & 0x80:
            limite = (limite << 12) | 0xFFF
        return base, limite, bool(d[5] & 0x80)

    def _assinatura(self):
        partes = [re.escape(struct.pack("<HHH", b["count"], b["tamanho"], b["off"])) + b"(..)"
                  for b in self.layout[:6]]
        return re.compile(b"".join(partes), re.S)

    def localizar(self, forcar=False):
        if self.seletores and not forcar:
            return self.seletores
        estava = self.q.rodando()
        if estava:
            self.q.cmd("stop")
        try:
            f = self.tmp / "ram.bin"
            self.q.cmd("pmemsave", val=0, size=self.ram, filename=str(f))
            ram = f.read_bytes()
            f.unlink()
            candidatos = []
            for m in self._assinatura().finditer(ram):
                sel = {}
                for b, g in zip(self.layout, m.groups()):
                    sel.setdefault(b["seg"], set()).add(struct.unpack("<H", g)[0])
                if all(len(v) == 1 for v in sel.values()):
                    candidatos.append((m.start(), {k: v.pop() for k, v in sel.items()}))
            achado = None
            for fis, sel in candidatos:
                if 8 not in sel or not sel[8] & 4:
                    continue
                base8, _, presente = self.descritor(sel[8])
                if not presente:
                    continue
                # o segmento 9 não aparece nas primeiras entradas: confere a tabela inteira pela base do seg. 8
                tab = self.ler_linear(base8 + 0x9570, len(self.layout) * 8)
                ok = True
                for i, b in enumerate(self.layout):
                    cnt, tam, off, s = struct.unpack_from("<HHHH", tab, i * 8)
                    if (cnt, tam, off) != (b["count"], b["tamanho"], b["off"]) or sel.setdefault(b["seg"], s) != s:
                        ok = False
                        break
                if ok:
                    achado = (fis, sel)
                    break
            if not achado:
                raise RuntimeError(f"tabela do save não encontrada na RAM ({len(candidatos)} candidatos)")
            self.seletores = achado[1]
            txt = ", ".join(f"seg {k} = {v:#06x}" for k, v in sorted(self.seletores.items()))
            print(f"memória: tabela do save no endereço físico {achado[0]:#x}; {txt}", flush=True)
            return self.seletores
        finally:
            if estava:
                self.q.cmd("cont")

    def base(self, seg):
        base, limite, presente = self.descritor(self.localizar()[seg])
        if not presente:
            raise RuntimeError(f"segmento {seg} não presente")
        return base, limite

    def resolver(self, nome, tamanho=None):
        """(segmento, offset, tamanho): apelido, endereço SEG:OFF (hex), layout do save ou symbols.tsv.
        Fora do layout e dos apelidos, o tamanho é obrigatório."""
        if nome in APELIDOS:
            nome, padrao = APELIDOS[nome]
            tamanho = tamanho or padrao
        m = re.fullmatch(r"(8|9|10):([0-9a-fA-F]{1,4})", nome)
        if m:
            if not tamanho:
                raise ValueError(f"{nome}: informe o tamanho")
            return int(m.group(1)), int(m.group(2), 16), tamanho
        nome = nome.lstrip("_")
        for b in self.layout:
            if b["simbolo"] == nome:
                return b["seg"], b["off"], tamanho or b["n"]
        simb = carregar_simbolos()
        if nome not in simb:
            raise KeyError(f"variável desconhecida: {nome}")
        if not tamanho:
            raise ValueError(f"{nome} não está no layout do save: informe o tamanho")
        return (*simb[nome], tamanho)

    def ler_variavel(self, nome, tamanho=None):
        seg, off, n = self.resolver(nome, tamanho)
        base, _ = self.base(seg)
        return self.ler_linear(base + off, n)

    def escrever_variavel(self, nome, dados: bytes, tamanho=None):
        seg, off, n = self.resolver(nome, tamanho or len(dados))
        if len(dados) != n:
            raise ValueError(f"{nome}: {n} bytes, valor com {len(dados)}")
        base, _ = self.base(seg)
        if self.gdb is not None:  # sessão persistente (modo tick)
            estava = self.q.rodando()
            g = self.sessao_gdb()  # interrompe se estava rodando
            g.escrever(base + off, dados)
            if g.ler(base + off, n) != dados:
                raise RuntimeError("escrita não confirmada")
            if estava:
                self.q.cmd("cont")
            return
        estava = self.q.rodando()
        g = Gdb(self.porta_gdb)  # a conexão para a VM
        try:
            g.escrever(base + off, dados)
            if g.ler(base + off, n) != dados:
                raise RuntimeError("escrita não confirmada")
        finally:
            g.desconectar()
            time.sleep(0.1)
            if estava != self.q.rodando():
                self.q.cmd("cont" if estava else "stop")

    def estado(self):
        """Todos os blocos do save na ordem de data/save_layout.json (mesmo formato de um .ANT).
        Lê cada segmento de uma vez (3 memsave) com a VM parada."""
        estava = self.q.rodando()
        if estava:
            self.q.cmd("stop")
        try:
            segs = {}
            for seg in sorted({b["seg"] for b in self.layout}):
                base, limite = self.base(seg)
                fim = max(b["off"] + b["n"] for b in self.layout if b["seg"] == seg)
                if fim > limite + 1:
                    raise RuntimeError(f"segmento {seg}: limite {limite:#x} < {fim:#x}")
                segs[seg] = self.ler_linear(base, fim)
            return b"".join(segs[b["seg"]][b["off"]:b["off"] + b["n"]] for b in self.layout)
        finally:
            if estava:
                self.q.cmd("cont")


def ppm_para_png(ppm: Path, png: Path):
    d = ppm.read_bytes()
    partes = d.split(maxsplit=4)
    w, h = int(partes[1]), int(partes[2])
    px = partes[4][: w * h * 3]
    linhas = b"".join(b"\x00" + px[y * w * 3:(y + 1) * w * 3] for y in range(h))

    def bloco(t, c):
        return struct.pack(">I", len(c)) + t + c + struct.pack(">I", zlib.crc32(t + c))

    png.write_bytes(b"\x89PNG\r\n\x1a\n" + bloco(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                    + bloco(b"IDAT", zlib.compress(linhas)) + bloco(b"IEND", b""))
    return w, h


def caminho_raiz(p):
    p = Path(p)
    return p if p.is_absolute() else RAIZ / p


def garantir_overlay(overlay: Path, base: Path = IMAGEM_C):
    if not overlay.exists():
        subprocess.run([str(QEMU_IMG), "create", "-f", "qcow2", "-b", str(base), "-F", "raw", str(overlay)],
                       check=True, capture_output=True)
        print(f"overlay criado: {overlay}", flush=True)


def mover_rel(q, dx, dy):
    while dx or dy:
        sx, sy = max(-50, min(50, dx)), max(-50, min(50, dy))
        q.cmd("input-send-event", events=[{"type": "rel", "data": {"axis": "x", "value": sx}},
                                          {"type": "rel", "data": {"axis": "y", "value": sy}}])
        dx, dy = dx - sx, dy - sy
        time.sleep(0.02)


def mouse_canto(q):
    for _ in range(20):
        q.cmd("input-send-event", events=[{"type": "rel", "data": {"axis": "x", "value": -100}},
                                          {"type": "rel", "data": {"axis": "y", "value": -100}}])
        time.sleep(0.02)


def clicar(q, botao="left"):
    for down in (True, False):
        q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": down, "button": botao}}])
        time.sleep(0.15)


def valor_para_bytes(valor: str, n):
    if valor.startswith("hex:"):
        return bytes.fromhex(valor[4:])
    v = int(valor, 0)
    return (v & ((1 << (8 * n)) - 1)).to_bytes(n, "little")


# _DoAntSim @ 1028:0000 = segmento de código 6 do NE, offset 0 (SIMANTW.SYM; docs/spec/regras/simulacao-visao-geral.md)
DOANTSIM = (6, 0x0000)
# CALL far _DoAntSim (9A 00 00 + relocação) em 1000:2683, logo antes do retorno 1000:2688 (bytes do EXE)
CHAMADA_DOANTSIM = (1, 0x2683)
# _MacTickCount @ 1000:9694 = segmento de código 1: CALL _TickCount (GetTickCount() / 55); DX:AX × 3; RETF em
# 1000:96a5. É o relógio do gato, do latido e do balão do menino (docs/spec/regras/quintal.md, "Relógios").
MACTICKCOUNT = (1, 0x9694)
MACTICKCOUNT_RETF = (1, 0x96A5)
# Leituras do relógio anotadas por tick; passando disso a lista termina com null (truncada).
MAX_LEITURAS_RELOGIO = 200
# Variáveis fora do save capturadas junto com cada estado (docs/spec/regras/testes-tick.md, pergunta 3):
# nome -> (endereço ou símbolo, bytes)
EXTRAS = {
    "SementeS": ("SementeS", 2),       # 10:cbf2, estado do LFSR (gerador S)
    "SementeR": ("SementeR", 4),       # 10:ae34, estado do rand() do C (gerador R)
    "gameCycles": ("gameCycles", 4),   # 9:9b2c, chamadas do laço do MYTIMERFUNC (fora do tick)
    "Tindex": ("Tindex", 2),           # 9:9b6a, índice global das listas
    "GlobalKey": ("GlobalKey", 2),     # 9:7866, última tecla (0xA8 dispara chuva em _DoSimYard)
    "TimeTemp": ("TimeTemp", 4),       # 9:9b10, prazo/marco do tutorial (conselhos.md), fora do save
    "DROPdir": ("DROPdir", 2),         # 9:9c66, direção da comida que cai na casa (superficie.md 8.2), fora do save
}
# Mapas de ocupação (campo T por célula), fora do save; gravados à parte (vida_antes.bin / vida_depois.bin)
VIDA = (("LifeA", 8192), ("LifeB", 4096), ("LifeR", 4096))


class Depurador:
    """Execução tick a tick do original pelo gdbstub do QEMU (breakpoints em endereço LINEAR).

    - "entrada": CPU parada na 1ª instrução de _DoAntSim (estado ANTES do tick; GameTime ainda não somado).
    - "saida": CPU parada no endereço de retorno de _DoAntSim dentro do MYTIMERFUNC (estado DEPOIS do
      tick, antes de _myServiceSong e _UpdateWindows).
    O endereço linear de _DoAntSim = base do seletor do segmento de código 6 (achado na LDT) + 0. O de
    retorno é lido da pilha (far: IP, CS) na entrada. Cada breakpoint é ligado só durante a corrida até
    ele e desligado ao parar, assim nunca se continua a partir de um breakpoint ativo.
    """

    def __init__(self, mj: "Memoria"):
        self.mj = mj
        self.retorno = None

    def entrada(self):
        return self.mj.linear_codigo(*DOANTSIM)

    def pc(self):
        r = self.mj.registradores()
        return r["CS"][1] + r["EIP"], r

    def onde(self):
        pc, _ = self.pc()
        try:
            entrada = self.entrada()
        except RuntimeError:  # segmento 6 ainda não carregado (novo jogo, antes do 1º tick)
            entrada = None
        if pc == entrada:
            return "entrada"
        if self.retorno is not None and pc == self.retorno:
            return "saida"
        return "livre"

    def _rodar_ate(self, endereco, timeout):
        g = self.mj.sessao_gdb()
        g.breakpoint(endereco)
        try:
            r = g.continuar_ate_parar(timeout)
        finally:
            g.breakpoint(endereco, False)
        pc, regs = self.pc()
        if pc != endereco:
            raise RuntimeError(f"parada inesperada ({r!r}): pc linear {pc:#x}, esperado {endereco:#x}")
        return regs

    def ir_entrada(self, timeout=120):
        """Roda até a próxima entrada de _DoAntSim (não anda se já está nela)."""
        self.mj.sessao_gdb()  # para a VM se estava rodando
        if self.onde() != "entrada":
            self._rodar_ate(self.entrada(), timeout)
        _, regs = self.pc()
        _, ss_base = regs["SS"]
        ip, cs = struct.unpack("<HH", self.mj.ler_linear(ss_base + (regs["ESP"] & 0xFFFF), 4))
        base, _, _ = self.mj.descritor(cs)
        self.retorno, self.retorno_far = base + ip, (cs, ip)

    def ir_saida(self, timeout=60, relogio=None):
        """Da entrada de _DoAntSim até o retorno (exatamente um tick).

        Com `relogio` (uma lista), anota nela o valor devolvido por cada chamada de _MacTickCount durante
        o tick (DX:AX no RETF). Para nunca continuar de um breakpoint ativo, alterna dois breakpoints na
        função: a entrada (rearma o RETF) e o RETF (lê o valor e rearma a entrada)."""
        self.ir_entrada()
        if relogio is None:
            self._rodar_ate(self.retorno, timeout)
            return
        g = self.mj.sessao_gdb()
        mac_ini, mac_ret = (self.mj.linear_codigo(*MACTICKCOUNT), self.mj.linear_codigo(*MACTICKCOUNT_RETF))
        armado = mac_ini
        g.breakpoint(self.retorno)
        g.breakpoint(armado)
        try:
            while True:
                r = g.continuar_ate_parar(timeout)
                pc, regs = self.pc()
                if pc == self.retorno:
                    return
                if pc != armado:
                    raise RuntimeError(f"parada inesperada ({r!r}): pc linear {pc:#x}")
                if pc == mac_ret:
                    relogio.append((regs["EDX"] & 0xFFFF) << 16 | (regs["EAX"] & 0xFFFF))
                g.breakpoint(armado, False)
                if len(relogio) >= MAX_LEITURAS_RELOGIO:
                    # animações (_AnimYellowFight, morte da amarela) leem o relógio em laço milhares de
                    # vezes; cada parada custa ~0,1 s. Daqui em diante só o retorno do tick.
                    relogio.append(None)
                    armado = None
                    continue
                armado = mac_ret if armado == mac_ini else mac_ini
                g.breakpoint(armado)
        finally:
            g.breakpoint(self.retorno, False)
            if armado is not None:
                g.breakpoint(armado, False)

    def ate_gametime(self, alvo, timeout=120):
        """Para na entrada de _DoAntSim com GameTime (antes do incremento) >= alvo."""
        self.ir_entrada(timeout)
        while True:
            gt = int.from_bytes(self.mj.ler_variavel("GameTime"), "little", signed=True)
            if gt >= alvo:
                return gt
            self.ir_saida()
            self.ir_entrada(timeout)

    def extras(self):
        out = {}
        for nome, (var, n) in EXTRAS.items():
            out[nome] = int.from_bytes(self.mj.ler_variavel(var, n), "little")
        out["GameTime"] = int.from_bytes(self.mj.ler_variavel("GameTime"), "little", signed=True)
        out["Cycle"] = int.from_bytes(self.mj.ler_variavel("Cycle"), "little", signed=True)
        out["posicao"] = self.onde()
        return out

    def capturar_tick(self, segmentos=False):
        """Um tick: {nome do arquivo: conteúdo} com estado_antes.bin / estado_depois.bin (layout do save),
        vida_antes.bin / vida_depois.bin (VIDA), extras.json e, opcionalmente, os segmentos inteiros."""
        out = {}
        self.ir_entrada()
        ex = {}
        relogio = []
        for fase, ir in (("antes", None), ("depois", lambda: self.ir_saida(relogio=relogio))):
            if ir:
                ir()
            out[f"estado_{fase}.bin"] = self.mj.estado()
            out[f"vida_{fase}.bin"] = self.vida()
            ex[fase] = self.extras()
            if segmentos:
                for seg, d in self.segmentos().items():
                    out[f"seg{seg}_{fase}.bin"] = d
        # Relógio do Mac lido pelo tick: o primeiro valor (null se o tick não chamou _MacTickCount) e todos
        # os valores, na ordem das chamadas (ver docs/spec/regras/quintal.md, "Relógios").
        ex["antes"]["MacTickCount"] = relogio[0] if relogio else None
        ex["MacTickCount_lidos"] = relogio
        out["extras.json"] = ex
        return out

    def vida(self):
        """_LifeA (128x64) + _LifeB (64x64) + _LifeR (64x64): mapas de ocupação fora do save (16.384 bytes)."""
        return b"".join(self.mj.ler_variavel(n, t) for n, t in VIDA)

    def segmentos(self):
        """Segmentos de dados 8, 9 e 10 inteiros (até o limite do descritor)."""
        out = {}
        for seg in (8, 9, 10):
            base, limite = self.mj.base(seg)
            out[seg] = self.mj.ler_linear(base, limite + 1)
        return out

    def mapear_leituras(self, faixas, simbolos, pedaco=64, maximo=20000):
        """Roda UM tick (da entrada ao retorno de _DoAntSim) com watchpoints de leitura nas faixas dadas
        [(seg, ini, fim)] e devolve [(seg, offset, nome, leitor CS:IP)], uma vez por pedaço lido.

        O gdbstub do QEMU informa o INÍCIO do watchpoint atingido, não o byte lido. Por isso as faixas são
        cortadas nas fronteiras dos símbolos e em pedaços de no máximo `pedaco` bytes, e cada pedaço tem o
        seu watchpoint: o pedaço atingido identifica a variável. Depois da 1ª leitura o watchpoint do
        pedaço é desligado. simbolos = {seg: [(offset, nome)]} ordenado."""
        g = self.mj.sessao_gdb()
        self.ir_entrada()
        for nseg in range(1, 8):  # para dizer quem leu (CS:IP -> segmento lógico 1000..1030)
            try:
                self.mj.seletor_codigo(nseg)
            except RuntimeError:
                pass
        bases = {seg: self.mj.base(seg)[0] for seg in (8, 9, 10)}
        pedacos = {}  # endereço linear -> (seg, ini, fim, nome)
        for seg, i, j in faixas:
            cortes = sorted({i, j} | {o for o, _ in simbolos.get(seg, []) if i < o < j})
            for a, b in zip(cortes, cortes[1:]):
                for c in range(a, b, pedaco):
                    nome, base_s = "?", 0
                    for o, n in simbolos.get(seg, []):
                        if o <= c:
                            nome, base_s = n, o
                        else:
                            break
                    pedacos[bases[seg] + c] = (seg, c, min(b, c + pedaco), f"{nome}+{c - base_s:#x}")
        ativos = set()
        try:
            for lin, (seg, i, j, _) in pedacos.items():
                g.watchpoint(lin, j - i)
                ativos.add(lin)
            g.breakpoint(self.retorno)
            lidas = []
            for _ in range(maximo):
                r = g.continuar_ate_parar(120)
                m = re.search(r"(?:r|a)?watch:([0-9a-f]+);", r)
                if not m:
                    pc, _ = self.pc()
                    if pc != self.retorno:
                        raise RuntimeError(f"parada inesperada {r!r} em {pc:#x}")
                    break
                lin = int(m.group(1), 16)
                if lin not in ativos:
                    continue
                seg, i, j, nome = pedacos[lin]
                regs = self.mj.registradores()
                leitor = f"{regs['CS'][0]:04x}:{regs['EIP'] & 0xFFFF:04x}"
                for nseg, sel in self.mj.seletores_codigo.items():
                    if sel == regs["CS"][0]:
                        leitor = f"{0x1000 + (nseg - 1) * 8:04x}:{regs['EIP'] & 0xFFFF:04x}"
                lidas.append((seg, i, nome, leitor))
                g.watchpoint(lin, j - i, False)
                ativos.discard(lin)
            else:
                raise RuntimeError("leituras demais")
        finally:
            for lin in list(ativos):
                seg, i, j, _ = pedacos[lin]
                g.watchpoint(lin, j - i, False)
            try:
                g.breakpoint(self.retorno, False)
            except RuntimeError:
                pass
        return lidas

    def retomar(self):
        """Volta a rodar livre (os breakpoints já estão desligados). A sessão gdb continua aberta."""
        if not self.mj.q.rodando():
            self.mj.q.cmd("cont")


def gravar(destino, dados):
    destino = caminho_raiz(destino)
    destino.parent.mkdir(parents=True, exist_ok=True)
    if isinstance(dados, (dict, list)):
        destino.write_text(json.dumps(dados, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    else:
        destino.write_bytes(dados)
    return destino


class Bancada:
    """Uma VM do QEMU com o SimAnt, controlada por QMP (e gdbstub). Usada pelos roteiros e por
    tools/gerar_vetores_tick.py."""

    def __init__(self, overlay=None, mem=16, extra=None, imagem=None):
        """`extra`: argumentos a mais para o QEMU (lista ou texto; padrão: a variável de ambiente
        SIMANT_QEMU_ARGS), por exemplo "-icount shift=5,sleep=on" ou a placa de som (bancada-qemu.md).
        `imagem`: disco C: raw no lugar da c.img (ex.: o de `preparar_svga`); base do overlay novo."""
        if extra is None:
            extra = os.environ.get("SIMANT_QEMU_ARGS", "")
        if isinstance(extra, str):
            extra = shlex.split(extra, posix=True)
        imagem = caminho_raiz(imagem) if imagem else IMAGEM_C
        if overlay is not None:
            overlay = caminho_raiz(overlay)
            garantir_overlay(overlay, imagem)
            disco = f"file={overlay},format=qcow2,if=ide,index=0"
        else:
            disco = f"file={imagem},format=raw,if=ide,index=0"
        porta_qmp, porta_gdb = porta_livre(), porta_livre()
        args = [
            str(QEMU), "-M", "pc", "-cpu", "486", "-m", str(mem),
            "-drive", f"file={DISQUETE},format=raw,if=floppy,index=0,readonly=on",
            "-drive", disco,
            "-boot", "a", "-vga", "std", "-display", "none",
            "-qmp", f"tcp:127.0.0.1:{porta_qmp},server,wait=off",
            "-gdb", f"tcp:127.0.0.1:{porta_gdb}",
        ] + list(extra)
        self.proc = subprocess.Popen(args, cwd=RAIZ)
        self.q = Qmp(porta_qmp)
        self.mj = Memoria(self.q, porta_gdb, mem)
        self.dep = Depurador(self.mj)
        self.t0 = time.time()

    def log(self, msg):
        print(f"[{time.time() - self.t0:6.1f}s] {msg}", flush=True)

    def fechar(self):
        if self.proc.poll() is None:
            try:
                self.q.cmd("quit")
                self.proc.wait(timeout=30)
            except (RuntimeError, OSError, subprocess.TimeoutExpired):
                pass
        if self.proc.poll() is None:
            self.proc.kill()

    def executar(self, linha):
        """Executa um comando de roteiro. Devolve False em 'fim'."""
        q, mj, dep, log = self.q, self.mj, self.dep, self.log
        p = linha.strip().split()
        if not p or p[0].startswith("#"):
            return True
        c, a = p[0], p[1:]
        if c == "esperar":
            time.sleep(float(a[0]))
        elif c == "teclas":
            for combo in a:
                q.cmd("send-key", keys=[{"type": "qcode", "data": k} for k in combo.split("+")], **{"hold-time": 150})
                time.sleep(0.3)
        elif c == "digitar":
            for ch in " ".join(a):
                k = "spc" if ch == " " else ch.lower()
                keys = ([{"type": "qcode", "data": "shift"}] if ch.isupper() else []) + [{"type": "qcode", "data": k}]
                q.cmd("send-key", keys=keys, **{"hold-time": 100})
                time.sleep(0.15)
        elif c == "mouse_canto":
            mouse_canto(q)
        elif c == "mover":
            mover_rel(q, int(a[0]), int(a[1]))
        elif c == "clicar_em":
            # canto superior esquerdo e depois movimento exato (aceleração desligada no WIN.INI)
            mouse_canto(q)
            mover_rel(q, int(a[0]), int(a[1]))
            time.sleep(0.2)
            clicar(q)
        elif c == "clicar":
            clicar(q)
        elif c == "clicar_duplo_em":
            # duplo clique (os dois cliques em ~0,25 s, como o `botoes(True)` de cenarios_tick.py)
            mouse_canto(q)
            mover_rel(q, int(a[0]), int(a[1]))
            time.sleep(0.2)
            for _ in range(2):
                for down in (True, False):
                    q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": down, "button": "left"}}])
                    time.sleep(0.06)
        elif c == "clicar_direito":
            clicar(q, "right")
        elif c == "clicar_direito_em":
            mouse_canto(q)
            mover_rel(q, int(a[0]), int(a[1]))
            time.sleep(0.2)
            clicar(q, "right")
        elif c in ("apertar_direito", "soltar_direito"):
            q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": c == "apertar_direito", "button": "right"}}])
            time.sleep(0.15)
        elif c == "ir_para":
            # só posiciona o cursor em (X, Y), sem clicar
            mouse_canto(q)
            mover_rel(q, int(a[0]), int(a[1]))
            time.sleep(0.2)
        elif c in ("apertar", "soltar"):
            # botão esquerdo do mouse segurado / solto (menus de "segurar o botão")
            q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": c == "apertar", "button": "left"}}])
            time.sleep(0.15)
        elif c in ("apertar_tecla", "soltar_tecla"):
            # tecla (qcode do QMP, ex.: ctrl, shift) segurada / solta, para Ctrl+clique
            q.cmd("input-send-event", events=[{"type": "key", "data": {
                "down": c == "apertar_tecla", "key": {"type": "qcode", "data": a[0]}}}])
            time.sleep(0.15)
        elif c == "capturar":
            ppm = mj.tmp / "tela.ppm"
            q.cmd("screendump", filename=str(ppm))
            time.sleep(0.3)
            w, h = ppm_para_png(ppm, caminho_raiz(a[0]))
            log(f"capturado {w}x{h} -> {a[0]}")
        elif c == "memoria":
            q.cmd("pmemsave", val=int(a[0], 0), size=int(a[1], 0), filename=str(caminho_raiz(a[2])))
            log(f"memória {a[0]}+{a[1]} -> {a[2]}")
        elif c == "pausar":
            q.cmd("stop")
            log("pausado")
        elif c == "continuar":
            dep.retomar()
            log("continuando")
        elif c in ("snapshot_salvar", "snapshot_carregar"):
            cmd = "savevm" if c == "snapshot_salvar" else "loadvm"
            r = q.hmp(f"{cmd} {a[0]}").strip()
            if r:
                raise RuntimeError(f"{cmd} {a[0]}: {r}")
            mj.esquecer_seletores()
            dep.retorno = None
            log(f"{cmd} {a[0]} ok (VM {'rodando' if q.rodando() else 'parada'})")
        elif c == "ler_variavel":
            d = mj.ler_variavel(a[0], int(a[2], 0) if len(a) > 2 else None)
            gravar(a[1], d)
            extra = (f" = {int.from_bytes(d, 'little', signed=True)} (0x{int.from_bytes(d, 'little'):0{2 * len(d)}x})"
                     if len(d) <= 4 else "")
            log(f"{a[0]}: {len(d)} bytes -> {a[1]}{extra}")
        elif c == "escrever_variavel":
            tam = int(a[2], 0) if len(a) > 2 else None
            n = None if a[1].startswith("hex:") else mj.resolver(a[0], tam)[2]
            dados = valor_para_bytes(a[1], n)
            mj.escrever_variavel(a[0], dados, tam)
            log(f"{a[0]} <- {a[1]} (bytes {dados.hex(' ')})")
        elif c == "estado":
            d = mj.estado()
            gravar(a[0], d)
            log(f"estado: {len(d)} bytes -> {a[0]}")
        elif c == "disquete":
            # Injeção de arquivos (casos de teste): troca o disquete do A: por outra imagem FAT12. O DOS vê a
            # troca pela linha de troca de disco; o jogo abre o arquivo por A:NOME.ANT. Ver docs/bancada-qemu.md.
            q.cmd("blockdev-change-medium", device="floppy0", filename=str(caminho_raiz(a[0])),
                  format="raw", **{"read-only-mode": "read-only"})
            log(f"disquete A: <- {a[0]}")
        elif c == "esperar_variavel":
            alvo, limite = int(a[1], 0), time.time() + (float(a[2]) if len(a) > 2 else 120)
            while True:
                v = int.from_bytes(mj.ler_variavel(a[0]), "little")
                if v >= alvo:
                    break
                if time.time() > limite:
                    raise RuntimeError(f"esperar_variavel: {a[0]} = {v} < {alvo} no tempo limite")
                time.sleep(0.05)
            log(f"{a[0]} = {v} (alvo {alvo})")
        elif c == "tick_entrada":
            dep.ir_entrada()
            log(f"entrada de _DoAntSim: {dep.extras()}")
        elif c == "tick_chamada":
            # Para no CALL far de _DoAntSim dentro do MYTIMERFUNC (segmento 1, sempre carregado): estado
            # antes do primeiro tick mesmo quando o segmento 6 ainda não está na memória (novo jogo).
            regs = dep._rodar_ate(mj.linear_codigo(*CHAMADA_DOANTSIM), float(a[0]) if a else 60)
            log(f"CALL _DoAntSim em 1000:{CHAMADA_DOANTSIM[1]:04x} (EIP {regs['EIP']:#x})")
        elif c == "tick_saida":
            dep.ir_saida()
            log(f"retorno de _DoAntSim: {dep.extras()}")
        elif c == "ate_gametime":
            gt = dep.ate_gametime(int(a[0], 0))
            log(f"entrada de _DoAntSim com GameTime = {gt}: {dep.extras()}")
        elif c == "passo_tick":
            # DIR/estado_antes.bin, estado_depois.bin, vida_antes.bin, vida_depois.bin, extras.json
            arqs = dep.capturar_tick()
            for nome_arq, conteudo in arqs.items():
                gravar(Path(a[0]) / nome_arq, conteudo)
            ex_antes, ex_depois = arqs["extras.json"]["antes"], arqs["extras.json"]["depois"]
            log(f"tick {ex_antes['GameTime']} -> {ex_depois['GameTime']}: S {ex_antes['SementeS']:#06x} -> "
                f"{ex_depois['SementeS']:#06x} -> {a[0]}")
        elif c == "extras":
            gravar(a[0], dep.extras())
            log(f"extras -> {a[0]}: {dep.extras()}")
        elif c == "segmentos":
            for seg, d in dep.segmentos().items():
                gravar(f"{a[0]}_seg{seg}.bin", d)
            log(f"segmentos 8, 9, 10 -> {a[0]}_seg*.bin")
        elif c == "fim":
            self.fechar()
            return False
        else:
            raise ValueError(f"comando desconhecido: {c}")
        return True


def roteiro(arquivo, overlay=None, mem=16, extra=None, imagem=None):
    linhas = Path(arquivo).read_text(encoding="utf-8").splitlines()
    usa_snap = any(ln.split()[:1] in (["snapshot_salvar"], ["snapshot_carregar"]) for ln in linhas)
    if usa_snap and overlay is None:
        if imagem is not None:  # o overlay padrão nasceu da c.img: não serve para outro disco
            sys.exit("com --disco e snapshots, dê também --overlay (um overlay próprio desse disco)")
        overlay = OVERLAY_PADRAO
    b = Bancada(overlay, mem, extra, imagem)
    try:
        for linha in linhas:
            if not b.executar(linha):
                break
    finally:
        b.fechar()


def extrair(caminho_dos, destino, overlay=None, disco=None):
    """Copia um arquivo do disco C: da bancada (c.img ou `disco`, ou o overlay qcow2 convertido para raw)."""
    from fat16_extrair import ler
    if overlay is None:
        imagem = (caminho_raiz(disco) if disco else IMAGEM_C).read_bytes()
    else:
        with tempfile.TemporaryDirectory() as t:
            raw = Path(t) / "c.img"
            subprocess.run([str(QEMU_IMG), "convert", "-f", "qcow2", "-O", "raw", str(caminho_raiz(overlay)), str(raw)],
                           check=True)
            imagem = raw.read_bytes()
    dados = ler(imagem, caminho_dos)
    Path(destino).write_bytes(dados)
    print(f"{caminho_dos}: {len(dados)} bytes -> {destino}")


def opcao(args, nome, padrao=None):
    if nome in args:
        k = args.index(nome)
        v = args[k + 1]
        del args[k:k + 2]
        return v
    return padrao


if __name__ == "__main__":
    args = sys.argv[1:]
    ov = opcao(args, "--overlay")
    mb = int(opcao(args, "--mem", "16"))
    qa = opcao(args, "--qemu-args")
    dc = opcao(args, "--disco")
    dr = opcao(args, "--drivers", str(DRIVERS_VBESVGA))
    ap = "--ajuda-padrao" in args
    if ap:
        args.remove("--ajuda-padrao")
    if not args:
        sys.exit(__doc__)
    if args[0] == "preparar":
        if dc:
            sys.exit("preparar recria só a c.img padrão; para outro disco use preparar_svga --disco")
        preparar()
    elif args[0] == "preparar_svga":
        preparar_svga(int(args[1]), int(args[2]), int(args[3]) if len(args) > 3 else 8, dc, dr, ap)
    elif args[0] == "roteiro":
        roteiro(args[1], ov, mb, qa, dc)
    elif args[0] == "extrair":
        extrair(args[1], args[2], ov, dc)
    else:
        sys.exit(__doc__)


