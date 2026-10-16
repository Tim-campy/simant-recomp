"""Bancada de testes do SimAnt original em QEMU, controlada só por QMP (sem janela, sem usar mouse/teclado do host).

Preparação (uma vez):  python tools/qemu_bancada.py preparar
Roteiro:               python tools/qemu_bancada.py roteiro ARQUIVO

Comandos de roteiro (um por linha, # = comentário):
    esperar SEG                 espera SEG segundos
    teclas T1 T2 ...            teclas QMP (qcodes: esc, ret, spc, alt, f, ...); '+' combina: alt+f
    digitar TEXTO               digita letras/números/espaço
    mouse_canto                 leva o cursor ao canto superior esquerdo (movimento relativo grande)
    mover DX DY                 movimento relativo (em mickeys; com aceleração desligada, 1 = 1 pixel)
    clicar                      clique esquerdo
    clicar_em X Y               leva o cursor a (X, Y) da tela emulada (canto + movimento exato) e clica
    capturar ARQUIVO.png        screendump da tela emulada
    memoria INICIO TAMANHO ARQ  pmemsave (endereço físico)
    fim                         encerra o QEMU

Ver docs/decisoes.md (bancada QEMU) e tests/referencias/README.md.
"""
import json
import shutil
import socket
import struct
import subprocess
import sys
import time
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fat12 import Fat12  # noqa: E402

RAIZ = Path(__file__).resolve().parent.parent
QEMU = RAIZ / "referencia" / "qemu" / "qemu-system-i386.exe"
TRAB = RAIZ / "re_trabalho" / "qemu"
DISCO_C = TRAB / "c"
DISQUETE = TRAB / "boot.img"
IMAGEM_C = TRAB / "c.img"
DOSBOX = RAIZ / "referencia" / "dosbox-x" / "bin" / "x64" / "Release" / "dosbox-x.exe"
FREEDOS = RAIZ / "re_trabalho" / "downloads" / "freedos" / "floppy" / "144m" / "x86BOOT.img"
# Kernel FreeDOS 2046 (github.com/FDOS/kernel, ke2046_86f16.zip): suporta o modo 386 avançado do Windows 3.x.
KERNEL_NOVO = RAIZ / "re_trabalho" / "downloads" / "freedos" / "ke2046_86" / "bin" / "kernel.sys"
PORTA_QMP = 4445

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


def criar_imagem_c():
    """Cria um disco FAT16 de 100 MB com o conteúdo de DISCO_C usando IMGMAKE/COPY do DOSBox-X.
    (O vvfat do QEMU em modo leitura-escrita falha quando o Windows grava no disco.)"""
    if IMAGEM_C.exists():
        IMAGEM_C.unlink()
    rel = lambda p: str(p.relative_to(RAIZ))  # noqa: E731
    pastas = sorted(str(p.relative_to(DISCO_C)) for p in DISCO_C.rglob("*") if p.is_dir())
    linhas = ["[sdl]", "output = surface", "[autoexec]", f'mount d "{rel(DISCO_C)}"',
              f"imgmake {rel(IMAGEM_C)} -t hd -size 100", f"imgmount e {rel(IMAGEM_C)}"]
    barra = "\\"
    linhas += [f"md E:{barra}{p}" for p in pastas]
    linhas += [f"copy D:{barra}{p}{barra}*.* E:{barra}{p}" for p in pastas
               if any(f.is_file() for f in (DISCO_C / p).iterdir())]
    linhas += ["exit"]
    conf = TRAB / "criar_imagem.conf"
    conf.write_text("\n".join(linhas) + "\n", encoding="latin-1")
    subprocess.run([str(DOSBOX), "-conf", str(conf), "-nopromptfolder"], cwd=RAIZ, timeout=300)


class Qmp:
    def __init__(self, porta):
        for _ in range(100):
            try:
                self.s = socket.create_connection(("127.0.0.1", porta), timeout=30)
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


def roteiro(arquivo):
    linhas = Path(arquivo).read_text(encoding="utf-8").splitlines()
    proc = subprocess.Popen([
        str(QEMU), "-M", "pc", "-cpu", "486", "-m", "16",
        "-drive", f"file={DISQUETE},format=raw,if=floppy,index=0",
        "-drive", f"file={IMAGEM_C},format=raw,if=ide,index=0",
        "-boot", "a", "-vga", "std", "-display", "none",
        "-qmp", f"tcp:127.0.0.1:{PORTA_QMP},server,wait=off",
    ], cwd=RAIZ)
    try:
        q = Qmp(PORTA_QMP)
        for linha in linhas:
            p = linha.strip().split()
            if not p or p[0].startswith("#"):
                continue
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
                for _ in range(20):
                    q.cmd("input-send-event", events=[{"type": "rel", "data": {"axis": "x", "value": -100}},
                                                      {"type": "rel", "data": {"axis": "y", "value": -100}}])
                    time.sleep(0.02)
            elif c == "mover":
                dx, dy = int(a[0]), int(a[1])
                while dx or dy:
                    sx, sy = max(-50, min(50, dx)), max(-50, min(50, dy))
                    q.cmd("input-send-event", events=[{"type": "rel", "data": {"axis": "x", "value": sx}},
                                                      {"type": "rel", "data": {"axis": "y", "value": sy}}])
                    dx, dy = dx - sx, dy - sy
                    time.sleep(0.02)
            elif c == "clicar_em":
                # canto superior esquerdo e depois movimento exato (aceleração desligada no WIN.INI)
                for _ in range(20):
                    q.cmd("input-send-event", events=[{"type": "rel", "data": {"axis": "x", "value": -100}},
                                                      {"type": "rel", "data": {"axis": "y", "value": -100}}])
                    time.sleep(0.02)
                dx, dy = int(a[0]), int(a[1])
                while dx or dy:
                    sx, sy = max(-50, min(50, dx)), max(-50, min(50, dy))
                    q.cmd("input-send-event", events=[{"type": "rel", "data": {"axis": "x", "value": sx}},
                                                      {"type": "rel", "data": {"axis": "y", "value": sy}}])
                    dx, dy = dx - sx, dy - sy
                    time.sleep(0.02)
                time.sleep(0.2)
                for down in (True, False):
                    q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": down, "button": "left"}}])
                    time.sleep(0.15)
            elif c == "clicar":
                for down in (True, False):
                    q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": down, "button": "left"}}])
                    time.sleep(0.15)
            elif c == "capturar":
                ppm = TRAB / "tela.ppm"
                q.cmd("screendump", filename=str(ppm))
                time.sleep(0.3)
                w, h = ppm_para_png(ppm, RAIZ / a[0])
                print(f"capturado {w}x{h} -> {a[0]}", flush=True)
            elif c == "memoria":
                q.cmd("pmemsave", val=int(a[0], 0), size=int(a[1], 0), filename=str(RAIZ / a[2]))
                print(f"memória {a[0]}+{a[1]} -> {a[2]}", flush=True)
            elif c == "fim":
                try:
                    q.cmd("quit")  # encerra o QEMU de forma limpa (grava o que o host tiver em cache)
                except (RuntimeError, OSError):
                    pass
                break
            else:
                raise ValueError(f"comando desconhecido: {c}")
    finally:
        proc.kill()


if __name__ == "__main__":
    if sys.argv[1] == "preparar":
        preparar()
    elif sys.argv[1] == "roteiro":
        roteiro(sys.argv[2])
