"""Extrai do SIMANTW.EXE a tabela de blocos gravados no .ANT (DS do segmento 8, offset 0x9570).

Cada entrada tem 8 bytes: u16 count, u16 tamanho do elemento, ponteiro far (offset, segmento).
O segmento só é conhecido após aplicar as relocações NE, por isso seguimos as cadeias de relocação.
Uso: python tools/save_table.py SIMANTW.EXE SIMANTW.SYM
"""
import struct
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from ne_info import parse_ne, parse_sym  # noqa: E402

TABLE_SEG = 8
TABLE_OFF = 0x9570


def seg_targets(data, seg):
    base = seg["file_offset"]
    o = base + seg["file_length"]
    n = struct.unpack_from("<H", data, o)[0]
    o += 2
    out = {}
    for _ in range(n):
        src, fl, off, a, _b = struct.unpack_from("<BBHHH", data, o)
        o += 8
        if fl & 3 or src not in (2, 3) or a == 0xFF:
            continue
        p = off
        for _ in range(10000):
            out[p] = a
            if fl & 4 or p == 0xFFFF:
                break
            nxt = struct.unpack_from("<H", data, base + (p if src == 2 else p + 2))[0]
            if nxt == 0xFFFF:
                break
            p = nxt
    return out


def main():
    data = open(sys.argv[1], "rb").read()
    info = parse_ne(data)
    sym = parse_sym(open(sys.argv[2], "rb").read())
    names = {}
    for s in sym["segments"]:
        for x in s["symbols"]:
            names[(s["load_index"], x["offset"])] = x["name"]
    seg = info["segments"][TABLE_SEG - 1]
    tg = seg_targets(data, seg)
    pos = 0
    i = 0
    while True:
        o = seg["file_offset"] + TABLE_OFF + i * 8
        cnt, sz, off, _ = struct.unpack_from("<HHHH", data, o)
        if cnt == 0:
            break
        t = tg.get(TABLE_OFF + i * 8 + 4) or tg.get(TABLE_OFF + i * 8 + 6)
        print(f"{i}\t{pos}\t{cnt * sz}\t{cnt}\t{sz}\t{t}:{off:04x}\t{names.get((t, off), '?')}")
        pos += cnt * sz
        i += 1
    print(f"# total {pos} bytes, {i} blocos")


if __name__ == "__main__":
    main()
