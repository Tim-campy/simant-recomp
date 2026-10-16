"""Leitor mínimo de PNG RGB/RGBA de 8 bits (sem entrelaçamento), só biblioteca padrão. Para comparar capturas."""
import struct
import zlib


def ler_png(caminho):
    d = open(caminho, "rb").read()
    assert d[:8] == b"\x89PNG\r\n\x1a\n"
    o, idat, w = 8, b"", None
    while o < len(d):
        n, t = struct.unpack(">I4s", d[o:o + 8])
        corpo = d[o + 8:o + 8 + n]
        if t == b"IHDR":
            w, h, prof, tipo = struct.unpack(">IIBB", corpo[:10])
            assert prof == 8 and tipo in (2, 6), "só RGB/RGBA 8 bits"
            bpp = 3 if tipo == 2 else 4
        elif t == b"IDAT":
            idat += corpo
        o += 12 + n
    raw = zlib.decompress(idat)
    stride = w * bpp
    linhas, prev = [], bytearray(stride)
    for y in range(h):
        f = raw[y * (stride + 1)]
        lin = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = lin[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if f == 1: lin[i] = (lin[i] + a) & 255
            elif f == 2: lin[i] = (lin[i] + b) & 255
            elif f == 3: lin[i] = (lin[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c; pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                lin[i] = (lin[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        linhas.append([tuple(lin[x * bpp:x * bpp + 3]) for x in range(w)])
        prev = lin
    return w, h, linhas
