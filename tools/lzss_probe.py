"""Testa a hipótese de que os registros com b1=1 são LZSS (estilo Okumura) precedidos do tamanho u16.

Uso: python tools/lzss_probe.py CAMINHO/SHARED
Imprime, por variante de inicialização da janela, quantos registros descomprimem exatamente
no tamanho declarado.
"""
import struct
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from db_list import read_index, read_records, DAT_HEADER  # noqa: E402


def lzss(src, out_len, fill=0x20, start=4096 - 18, n=4096):
    win = bytearray([fill]) * n
    r = start
    out = bytearray()
    i = 0
    while len(out) < out_len and i < len(src):
        flags = src[i]
        i += 1
        for bit in range(8):
            if len(out) >= out_len or i >= len(src):
                break
            if flags & (1 << bit):
                c = src[i]
                i += 1
                out.append(c)
                win[r] = c
                r = (r + 1) % n
            else:
                if i + 1 >= len(src):
                    return out, i
                a, b = src[i], src[i + 1]
                i += 2
                pos = a | ((b & 0xF0) << 4)
                ln = (b & 0x0F) + 3
                for k in range(ln):
                    c = win[(pos + k) % n]
                    out.append(c)
                    win[r] = c
                    r = (r + 1) % n
    return out, i


def main():
    base = sys.argv[1]
    dat = open(base + ".DAT", "rb").read()
    _, ents = read_index(base + ".NDX")
    recs, _ = read_records(dat)
    by_off = {r["pos"] - DAT_HEADER: r for r in recs}
    comp = [(e, by_off[e["offset"]]) for e in ents if e["b1"] == 1]
    for fill in (0x20, 0x00):
        ok = 0
        consumed = 0
        for e, r in comp:
            p = r["payload"]
            size = struct.unpack_from("<H", p, 0)[0]
            out, used = lzss(p[2:], size, fill)
            if len(out) == size:
                ok += 1
            if used + 2 == len(p):
                consumed += 1
        print(f"fill=0x{fill:02x}: {ok}/{len(comp)} no tamanho declarado, "
              f"{consumed}/{len(comp)} consumiram o payload inteiro")


if __name__ == "__main__":
    main()
