"""Compara uma tela exportada pelo port (--exportar-telas) com uma captura do original (tools/capturar_original.ps1).

Uso: python tools/comparar_captura.py PORT.png CAPTURA.png [--topo 38] [--diff saida.png]
  --topo: linha da captura onde começa a área cliente da janela principal (abaixo do menu).
As cores do port são convertidas como o DAC VGA de 6 bits do DOSBox-X faz (128 -> 130, 192 -> 195).
Imprime a porcentagem de pixels iguais e a caixa que contém as diferenças.
"""
import argparse
import struct
import zlib

from png_simples import ler_png


def dac(v):
    return ((v >> 2) << 2) | (v >> 6)


def salvar_png(caminho, w, h, linhas):
    raw = b"".join(b"\x00" + bytes(c for px in lin for c in px) for lin in linhas)

    def bloco(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d))

    open(caminho, "wb").write(b"\x89PNG\r\n\x1a\n" + bloco(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                             + bloco(b"IDAT", zlib.compress(raw)) + bloco(b"IEND", b""))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("port")
    ap.add_argument("captura")
    ap.add_argument("--topo", type=int, default=38)
    ap.add_argument("--diff")
    a = ap.parse_args()
    pw, ph, p = ler_png(a.port)
    cw, ch, c = ler_png(a.captura)
    w, h = min(pw, cw), min(ph, ch - a.topo)
    iguais, caixa = 0, None
    diff = []
    for y in range(h):
        lin = []
        for x in range(w):
            esperado = tuple(dac(v) for v in p[y][x])
            obtido = c[y + a.topo][x]
            if esperado == obtido:
                iguais += 1
                lin.append(tuple(v // 3 for v in obtido))
            else:
                lin.append((255, 0, 0))
                caixa = (min(caixa[0], x), min(caixa[1], y), max(caixa[2], x), max(caixa[3], y)) if caixa else (x, y, x, y)
        diff.append(lin)
    print(f"área comparada {w}x{h}: {100 * iguais / (w * h):.2f}% iguais; diferenças em {caixa}")
    if a.diff:
        salvar_png(a.diff, w, h, diff)


if __name__ == "__main__":
    main()
