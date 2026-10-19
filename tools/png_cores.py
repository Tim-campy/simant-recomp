"""Conta as cores de um PNG RGB/RGBA de 8 bits (decodificador mínimo, sem PIL).

Uso: python tools/png_cores.py ARQ.png [X0 Y0 X1 Y1] [N]
     mostra as N cores mais frequentes (padrão 40) da região (padrão: a imagem toda).
"""
import collections
import struct
import sys
import zlib


def ler_png(caminho):
    """Devolve (largura, altura, bytes_por_pixel, linhas)."""
    d = open(caminho, "rb").read()
    pos, idat = 8, b""
    while pos < len(d):
        n, t = struct.unpack(">I4s", d[pos:pos + 8])
        c = d[pos + 8:pos + 8 + n]
        if t == b"IHDR":
            w, h, bd, ct = struct.unpack(">IIBB", c[:10])
            if bd != 8 or ct not in (2, 6):
                raise ValueError(f"só PNG RGB/RGBA de 8 bits (bits={bd}, tipo={ct})")
            bpp = 3 if ct == 2 else 4
        elif t == b"IDAT":
            idat += c
        pos += 12 + n
    raw = zlib.decompress(idat)
    stride = w * bpp
    linhas, ant, o = [], bytearray(stride), 0
    for _ in range(h):
        f = raw[o]
        cur = bytearray(raw[o + 1:o + 1 + stride])
        o += 1 + stride
        if f:
            for i in range(stride):
                a = cur[i - bpp] if i >= bpp else 0
                b = ant[i]
                cc = ant[i - bpp] if i >= bpp else 0
                if f == 1:
                    cur[i] = (cur[i] + a) & 255
                elif f == 2:
                    cur[i] = (cur[i] + b) & 255
                elif f == 3:
                    cur[i] = (cur[i] + (a + b) // 2) & 255
                elif f == 4:
                    p = a + b - cc
                    pa, pb, pc = abs(p - a), abs(p - b), abs(p - cc)
                    pr = a if pa <= pb and pa <= pc else (b if pb <= pc else cc)
                    cur[i] = (cur[i] + pr) & 255
        linhas.append(bytes(cur))
        ant = cur
    return w, h, bpp, linhas


def main():
    w, h, bpp, linhas = ler_png(sys.argv[1])
    x0, y0, x1, y1 = (int(v) for v in sys.argv[2:6]) if len(sys.argv) >= 6 else (0, 0, w, h)
    n = int(sys.argv[6]) if len(sys.argv) > 6 else 40
    c = collections.Counter()
    for y in range(y0, y1):
        r = linhas[y]
        for x in range(x0, x1):
            c[tuple(r[x * bpp:x * bpp + 3])] += 1
    print(f"{sys.argv[1]} {w}x{h} região {x0},{y0}-{x1},{y1}: {len(c)} cores")
    for cor, k in c.most_common(n):
        print("  #%02x%02x%02x" % cor, k)


if __name__ == "__main__":
    main()
