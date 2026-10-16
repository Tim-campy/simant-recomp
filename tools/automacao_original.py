"""Automação do SimAnt original no DOSBox-X para gerar referências (capturas e saves).

ATENÇÃO: usa o mouse e o teclado reais do Windows enquanto roda.

Uso: python tools/automacao_original.py ROTEIRO
ROTEIRO é um arquivo de texto com um comando por linha (linhas com # são comentários):
    esperar SEG            espera SEG segundos
    clicar X Y             leva o cursor do Windows 3.1 até (X, Y) da tela emulada (800x600) e clica
    mover X Y              só move
    teclas T1 T2 ...       teclas por scancode, separadas por espaço; '+' combina (ex.: alt+f baixo enter)
    digitar TEXTO          digita o texto letra a letra
    capturar ARQUIVO.png   salva a área cliente (tela emulada) em PNG
    fim                    encerra o DOSBox-X
O cursor é localizado procurando a seta padrão do Windows 3.1 na captura.
"""
import ctypes
import ctypes.wintypes as wt
import struct
import subprocess
import sys
import time
import zlib
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
EXE = RAIZ / "referencia" / "dosbox-x" / "bin" / "x64" / "Release" / "dosbox-x.exe"
u32 = ctypes.windll.user32
g32 = ctypes.windll.gdi32
u32.SetProcessDPIAware()

# Seta padrão: '#' preto, 'o' branco, ponta em (0,0). Medida em tests/referencias/capturas/novo_jogo.png.
SETA = [
    "##",
    "#o#",
    "#oo#",
    "#ooo#",
    "#oooo#",
    "#ooooo#",
    "#oooooo#",
    "#ooooooo#",
    "#oooooooo#",
    "#ooooooooo#",
    "#oooooo#####",
]
PONTOS_SETA = [(x, y, c) for y, linha in enumerate(SETA) for x, c in enumerate(linha) if c in "#o"]
PRETO, BRANCO = (0, 0, 0), (255, 255, 255)


class MOUSEINPUT(ctypes.Structure):
    _fields_ = [("dx", wt.LONG), ("dy", wt.LONG), ("mouseData", wt.DWORD), ("dwFlags", wt.DWORD),
                ("time", wt.DWORD), ("dwExtraInfo", ctypes.c_size_t)]


class INPUT(ctypes.Structure):
    _fields_ = [("type", wt.DWORD), ("mi", MOUSEINPUT), ("pad", ctypes.c_byte * 8)]


def mouse(flags, dx=0, dy=0):
    i = INPUT(type=0)
    i.mi.dx, i.mi.dy, i.mi.dwFlags = dx, dy, flags
    u32.SendInput(1, ctypes.byref(i), ctypes.sizeof(INPUT))


def janela_do_processo(pid):
    achadas = []

    @ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)
    def cb(h, _):
        p = wt.DWORD()
        u32.GetWindowThreadProcessId(h, ctypes.byref(p))
        if p.value == pid and u32.IsWindowVisible(h):
            achadas.append(h)
        return True

    u32.EnumWindows(cb, 0)

    def area(h):
        r = wt.RECT()
        u32.GetClientRect(h, ctypes.byref(r))
        return r.right * r.bottom
    achadas = [h for h in achadas if area(h) > 0]
    return max(achadas, key=area) if achadas else None


def capturar(h):
    """(largura, altura, bytes BGRA de cima para baixo) da área cliente da janela.
    Usa PrintWindow, que funciona mesmo com a janela atrás de outras (não precisa de foco)."""
    r = wt.RECT()
    u32.GetClientRect(h, ctypes.byref(r))
    w, a = r.right, r.bottom
    tela = u32.GetDC(0)
    mem = g32.CreateCompatibleDC(tela)
    bmp = g32.CreateCompatibleBitmap(tela, w, a)
    g32.SelectObject(mem, bmp)
    u32.PrintWindow(h, mem, 0x1 | 0x2)  # PW_CLIENTONLY | PW_RENDERFULLCONTENT
    cab = ctypes.create_string_buffer(struct.pack("<IiiHHIIiiII", 40, w, -a, 1, 32, 0, 0, 0, 0, 0, 0) + bytes(16))
    buf = ctypes.create_string_buffer(w * a * 4)
    g32.GetDIBits(mem, bmp, 0, a, buf, cab, 0)
    g32.DeleteObject(bmp)
    g32.DeleteDC(mem)
    u32.ReleaseDC(0, tela)
    return w, a, buf.raw


def salvar_png(caminho, w, a, bgra):
    linhas = bytearray()
    for y in range(a):
        linhas.append(0)
        for x in range(w):
            o = (y * w + x) * 4
            linhas += bytes((bgra[o + 2], bgra[o + 1], bgra[o]))

    def bloco(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d))

    Path(caminho).write_bytes(b"\x89PNG\r\n\x1a\n" + bloco(b"IHDR", struct.pack(">IIBBBBB", w, a, 8, 2, 0, 0, 0))
                              + bloco(b"IDAT", zlib.compress(bytes(linhas))) + bloco(b"IEND", b""))


def achar_cursor(w, a, bgra):
    """Ponta da seta na tela emulada, ou None."""
    def px(x, y):
        o = (y * w + x) * 4
        return bgra[o + 2], bgra[o + 1], bgra[o]

    for y in range(0, a - 12):
        for x in range(0, w - 12):
            if px(x, y) != PRETO or px(x + 1, y + 1) != BRANCO:
                continue
            if all(px(x + dx, y + dy) == (PRETO if c == "#" else BRANCO) for dx, dy, c in PONTOS_SETA):
                return x, y
    return None


def mover_relativo(dx, dy):
    # passos pequenos (≤ 2) evitam a aceleração do mouse do Windows 3.1
    while dx or dy:
        sx, sy = max(-2, min(2, dx)), max(-2, min(2, dy))
        mouse(0x0001, sx, sy)
        dx, dy = dx - sx, dy - sy
        time.sleep(0.008)


def levar_cursor(h, alvo):
    """Com o VBMOUSE.DRV (mouse absoluto) na cópia do Windows 3.1, o cursor do emulador segue o do host 1:1.
    Posiciona o cursor do host sobre o ponto da área cliente e confere onde a seta do Windows 3.1 ficou."""
    p = wt.POINT(alvo[0], alvo[1])
    u32.ClientToScreen(h, ctypes.byref(p))
    larg, alt = u32.GetSystemMetrics(0), u32.GetSystemMetrics(1)
    for _ in range(3):
        for dx in (-3, 0):
            # movimento absoluto normalizado (0..65535) na tela principal, como um mouse de verdade
            nx = ((p.x + dx) * 65535 + larg // 2) // (larg - 1)
            ny = (p.y * 65535 + alt // 2) // (alt - 1)
            mouse(0x8000 | 0x0001, nx, ny)  # ABSOLUTE | MOVE
            time.sleep(0.1)
        time.sleep(0.4)
        pos = achar_cursor(*capturar(h))
        print(f"  seta do emulador em {pos}, alvo {alvo}", flush=True)
        if pos and abs(pos[0] - alvo[0]) <= 1 and abs(pos[1] - alvo[1]) <= 1:
            return pos
    raise RuntimeError(f"o cursor do emulador não chegou a {alvo}")


class KEYBDINPUT(ctypes.Structure):
    _fields_ = [("wVk", wt.WORD), ("wScan", wt.WORD), ("dwFlags", wt.DWORD), ("time", wt.DWORD),
                ("dwExtraInfo", ctypes.c_size_t)]


class INPUT_TECLADO(ctypes.Structure):
    _fields_ = [("type", wt.DWORD), ("ki", KEYBDINPUT), ("pad", ctypes.c_byte * 16)]


# Scancodes do conjunto 1 (o que o SDL/DOSBox-X lê). (código, estendida)
SCANCODES = {
    "esc": (0x01, False), "enter": (0x1C, False), "tab": (0x0F, False), "espaco": (0x39, False),
    "backspace": (0x0E, False), "ctrl": (0x1D, False), "shift": (0x2A, False), "alt": (0x38, False),
    "cima": (0x48, True), "baixo": (0x50, True), "esquerda": (0x4B, True), "direita": (0x4D, True),
    ".": (0x34, False), "-": (0x0C, False), "\\": (0x2B, False), ":": (0x27, False),
}
for i, c in enumerate("1234567890"):
    SCANCODES[c] = (0x02 + i, False)
for linha, base in (("qwertyuiop", 0x10), ("asdfghjkl", 0x1E), ("zxcvbnm", 0x2C)):
    for i, c in enumerate(linha):
        SCANCODES[c] = (base + i, False)
for i in range(10):
    SCANCODES[f"f{i + 1}"] = (0x3B + i, False)


def tecla(nome, soltar):
    codigo, estendida = SCANCODES[nome]
    flags = 0x0008 | (0x0002 if soltar else 0) | (0x0001 if estendida else 0)  # SCANCODE, KEYUP, EXTENDED
    i = INPUT_TECLADO(type=1)
    i.ki.wScan, i.ki.dwFlags = codigo, flags
    u32.SendInput(1, ctypes.byref(i), ctypes.sizeof(INPUT_TECLADO))
    time.sleep(0.15)


def focar(h):
    """Traz a janela para a frente. O Windows bloqueia SetForegroundWindow de processos em segundo
    plano; apertar e soltar Alt antes libera a troca."""
    for _ in range(5):
        if u32.GetForegroundWindow() == h:
            return True
        tecla("alt", False)
        tecla("alt", True)
        u32.ShowWindow(h, 5)
        u32.BringWindowToTop(h)
        u32.SetForegroundWindow(h)
        time.sleep(0.3)
    print("AVISO: não consegui pôr o DOSBox-X em primeiro plano", flush=True)
    return False


def enviar_teclas(arg):
    """'alt+f baixo enter': teclas separadas por espaço; '+' combina modificadores."""
    for combo in arg.lower().split():
        partes = combo.split("+")
        for p in partes:
            tecla(p, False)
        for p in reversed(partes):
            tecla(p, True)
        time.sleep(0.25)


def digitar(texto):
    for c in texto:
        if c.isupper() or c == ":":
            enviar_teclas(f"shift+{c.lower()}" if c != ":" else "shift+:")
        else:
            enviar_teclas("espaco" if c == " " else c)


def main():
    roteiro = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
    proc = subprocess.Popen([str(EXE), "-conf", "referencia/dosbox-x-automacao.conf", "-nopromptfolder"], cwd=RAIZ)
    h = None
    try:
        for linha in roteiro:
            partes = linha.strip().split(maxsplit=1)
            if not partes or partes[0].startswith("#"):
                continue
            cmd, arg = partes[0], (partes[1] if len(partes) > 1 else "")
            if cmd != "esperar":
                h = None
                while h is None:
                    h = janela_do_processo(proc.pid)
                    time.sleep(0.2)
                focar(h)
            if cmd == "esperar":
                time.sleep(float(arg))
            elif cmd in ("clicar", "mover"):
                x, y = map(int, arg.split())
                print(f"cursor em {levar_cursor(h, (x, y))}", flush=True)
                if cmd == "clicar":
                    mouse(0x0002)
                    time.sleep(0.12)
                    mouse(0x0004)
                    time.sleep(0.5)
            elif cmd == "teclas":
                enviar_teclas(arg)
            elif cmd == "digitar":
                digitar(arg)
            elif cmd == "capturar":
                w, a, img = capturar(h)
                salvar_png(RAIZ / arg, w, a, img)
                print(f"capturado {w}x{a} -> {arg}", flush=True)
            elif cmd == "fim":
                break
            else:
                raise ValueError(f"comando desconhecido: {cmd}")
    finally:
        proc.kill()


if __name__ == "__main__":
    main()
