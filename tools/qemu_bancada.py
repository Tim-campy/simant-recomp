"""Bancada de testes do SimAnt original em QEMU, controlada só por QMP (sem janela, sem usar mouse/teclado do host).

Preparação (uma vez):  python tools/qemu_bancada.py preparar
Roteiro:               python tools/qemu_bancada.py roteiro ARQUIVO [--overlay ARQ.qcow2] [--mem MB]
Extrair do disco:      python tools/qemu_bancada.py extrair CAMINHO_DOS DESTINO [--overlay ARQ.qcow2]

Comandos de roteiro (um por linha, # = comentário):
    esperar SEG                 espera SEG segundos (relógio do host)
    teclas T1 T2 ...            teclas QMP (qcodes: esc, ret, spc, alt, f, ...); '+' combina: alt+f
    digitar TEXTO               digita letras/números/espaço
    mouse_canto                 leva o cursor ao canto superior esquerdo (movimento relativo grande)
    mover DX DY                 movimento relativo (em mickeys; com aceleração desligada, 1 = 1 pixel)
    clicar                      clique esquerdo
    clicar_em X Y               leva o cursor a (X, Y) da tela emulada (canto + movimento exato) e clica
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


def criar_imagem_c():
    """Cria um disco FAT16 de 100 MB com o conteúdo de DISCO_C usando IMGMAKE/COPY do DOSBox-X.
    (O vvfat do QEMU em modo leitura-escrita falha quando o Windows grava no disco.)"""
    if IMAGEM_C.exists():
        IMAGEM_C.unlink()
    if OVERLAY_PADRAO.exists():  # o overlay (e seus snapshots) só vale sobre a c.img de onde nasceu
        OVERLAY_PADRAO.unlink()
    rel =lambda p: str(p.relative_to(RAIZ))  # noqa: E731
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

    def _pacote(self, dados: str) -> str:
        cs = sum(dados.encode()) & 0xFF
        self.s.sendall(f"${dados}#{cs:02x}".encode())
        while True:
            i = self.buf.find(b"$")
            j = self.buf.find(b"#", i) if i >= 0 else -1
            if i >= 0 and j >= 0 and len(self.buf) >= j + 3:
                resp = self.buf[i + 1:j].decode()
                self.buf = self.buf[j + 3:]
                self.s.sendall(b"+")
                return resp
            self.buf += self.s.recv(65536)

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
        self.layout = carregar_layout()
        self.tmp = Path(tempfile.mkdtemp(prefix="bancada_"))

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
        estava = self.q.rodando()
        base, _ = self.base(seg)
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


def garantir_overlay(overlay: Path):
    if not overlay.exists():
        subprocess.run([str(QEMU_IMG), "create", "-f", "qcow2", "-b", str(IMAGEM_C), "-F", "raw", str(overlay)],
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


def clicar(q):
    for down in (True, False):
        q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": down, "button": "left"}}])
        time.sleep(0.15)


def valor_para_bytes(valor: str, n):
    if valor.startswith("hex:"):
        return bytes.fromhex(valor[4:])
    v = int(valor, 0)
    return (v & ((1 << (8 * n)) - 1)).to_bytes(n, "little")


def roteiro(arquivo, overlay=None, mem=16):
    linhas = Path(arquivo).read_text(encoding="utf-8").splitlines()
    usa_snap = any(ln.split()[:1] in (["snapshot_salvar"], ["snapshot_carregar"]) for ln in linhas)
    if usa_snap and overlay is None:
        overlay = OVERLAY_PADRAO
    if overlay is not None:
        overlay = caminho_raiz(overlay)
        garantir_overlay(overlay)
        disco = f"file={overlay},format=qcow2,if=ide,index=0"
    else:
        disco = f"file={IMAGEM_C},format=raw,if=ide,index=0"
    porta_qmp, porta_gdb = porta_livre(), porta_livre()
    proc = subprocess.Popen([
        str(QEMU), "-M", "pc", "-cpu", "486", "-m", str(mem),
        "-drive", f"file={DISQUETE},format=raw,if=floppy,index=0,readonly=on",
        "-drive", disco,
        "-boot", "a", "-vga", "std", "-display", "none",
        "-qmp", f"tcp:127.0.0.1:{porta_qmp},server,wait=off",
        "-gdb", f"tcp:127.0.0.1:{porta_gdb}",
    ], cwd=RAIZ)
    try:
        q = Qmp(porta_qmp)
        mj = Memoria(q, porta_gdb, mem)
        t0 = time.time()

        def log(msg):
            print(f"[{time.time() - t0:6.1f}s] {msg}", flush=True)

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
                q.cmd("cont")
                log("continuando")
            elif c in ("snapshot_salvar", "snapshot_carregar"):
                cmd = "savevm" if c == "snapshot_salvar" else "loadvm"
                r = q.hmp(f"{cmd} {a[0]}").strip()
                if r:
                    raise RuntimeError(f"{cmd} {a[0]}: {r}")
                mj.seletores = None  # a sessão do Windows restaurada pode ter outros seletores
                log(f"{cmd} {a[0]} ok")
            elif c == "ler_variavel":
                d = mj.ler_variavel(a[0], int(a[2], 0) if len(a) > 2 else None)
                destino = caminho_raiz(a[1])
                destino.parent.mkdir(parents=True, exist_ok=True)
                destino.write_bytes(d)
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
                destino = caminho_raiz(a[0])
                destino.parent.mkdir(parents=True, exist_ok=True)
                destino.write_bytes(d)
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
            elif c == "fim":
                try:
                    q.cmd("quit")  # encerra o QEMU de forma limpa (grava o que o host tiver em cache)
                except (RuntimeError, OSError):
                    pass
                proc.wait(timeout=30)
                break
            else:
                raise ValueError(f"comando desconhecido: {c}")
    finally:
        if proc.poll() is None:
            proc.kill()


def extrair(caminho_dos, destino, overlay=None):
    """Copia um arquivo do disco C: da bancada (c.img, ou o overlay qcow2 convertido para raw)."""
    from fat16_extrair import ler
    if overlay is None:
        imagem = IMAGEM_C.read_bytes()
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
    if not args:
        sys.exit(__doc__)
    if args[0] == "preparar":
        preparar()
    elif args[0] == "roteiro":
        roteiro(args[1], ov, mb)
    elif args[0] == "extrair":
        extrair(args[1], args[2], ov)
    else:
        sys.exit(__doc__)


