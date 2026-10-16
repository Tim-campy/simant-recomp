"""Extrai do SIMANTW.EXE a tabela de blocos gravados no .ANT (DS do segmento 8, offset 0x9570).

Cada entrada tem 8 bytes: u16 count, u16 tamanho do elemento, ponteiro far (offset, segmento).
O segmento só é conhecido após aplicar as relocações NE, por isso seguimos as cadeias de relocação.
Uso: python tools/save_table.py SIMANTW.EXE SIMANTW.SYM [--json data/save_layout.json]
Sem --json, imprime a tabela de docs/spec/formatos/save-ant.md. Com --json, grava o layout
(ordem, símbolo sem o '_' inicial, count, tamanho do elemento) lido por engine/src/formats/save.rs.
"""
import json
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


def ler_tabela(exe, sym_path):
    data = open(exe, "rb").read()
    info = parse_ne(data)
    sym = parse_sym(open(sym_path, "rb").read())
    names = {}
    for s in sym["segments"]:
        for x in s["symbols"]:
            names[(s["load_index"], x["offset"])] = x["name"]
    seg = info["segments"][TABLE_SEG - 1]
    tg = seg_targets(data, seg)
    blocos = []
    i = 0
    while True:
        o = seg["file_offset"] + TABLE_OFF + i * 8
        cnt, sz, off, _ = struct.unpack_from("<HHHH", data, o)
        if cnt == 0:
            break
        t = tg.get(TABLE_OFF + i * 8 + 4) or tg.get(TABLE_OFF + i * 8 + 6)
        blocos.append({"simbolo": names.get((t, off), "?"), "count": cnt, "tamanho": sz, "endereco": f"{t}:{off:04x}"})
        i += 1
    return blocos


def gravar_json(blocos, caminho):
    total = sum(b["count"] * b["tamanho"] for b in blocos)
    comentario = (
        "Layout do jogo salvo .ANT do original: blocos em ordem, cada um com count x tamanho bytes "
        "(little-endian). Gerado por tools/save_table.py --json a partir da tabela em SIMANTW.EXE "
        "(segmento 8, offset 0x9570); nomes do SIMANTW.SYM sem o '_' inicial. "
        "Ver docs/spec/formatos/save-ant.md."
    )
    linhas = [
        "    " + json.dumps({**b, "simbolo": b["simbolo"].lstrip("_")}, ensure_ascii=False) for b in blocos
    ]
    with open(caminho, "w", encoding="utf-8", newline="\n") as f:
        f.write("{\n")
        f.write(f'  "_comentario": {json.dumps(comentario, ensure_ascii=False)},\n')
        f.write(f'  "total_bytes": {total},\n')
        f.write('  "blocos": [\n')
        f.write(",\n".join(linhas))
        f.write("\n  ]\n}\n")
    print(f"{caminho}: {len(blocos)} blocos, {total} bytes")


def main():
    args = sys.argv[1:]
    saida_json = None
    if "--json" in args:
        k = args.index("--json")
        saida_json = args[k + 1]
        del args[k:k + 2]
    blocos = ler_tabela(args[0], args[1])
    if saida_json is not None:
        gravar_json(blocos, saida_json)
        return
    pos = 0
    for i, b in enumerate(blocos):
        n = b["count"] * b["tamanho"]
        print(f"{i}\t{pos}\t{n}\t{b['count']}\t{b['tamanho']}\t{b['endereco']}\t{b['simbolo']}")
        pos += n
    print(f"# total {pos} bytes, {len(blocos)} blocos")


if __name__ == "__main__":
    main()
