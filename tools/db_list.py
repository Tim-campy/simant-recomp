"""Lista os registros de um par .NDX/.DAT do SimAnt (banco de recursos da Maxis).

Uso:
    python tools/db_list.py CAMINHO/WINGANT          (sem extensão)
    python tools/db_list.py CAMINHO/WINGANT --dump DIR

Formato (ver docs/spec/formatos/banco-dat-ndx.md): estrutura em observação, ainda incompleta.
"""
import argparse
import collections
import os
import struct
import sys

MAGIC = 0x12345678
DAT_HEADER = 14   # magic, u16, u16, u32, u16 (significado parcial)
REC_HEADER = 10   # magic, u16 flags, u32 tamanho do payload


def read_index(path):
    n = open(path, "rb").read()
    count = struct.unpack_from("<I", n, 0)[0]
    ents = []
    for i in range(count):
        off, rid, b0, b1 = struct.unpack_from("<IHBB", n, 0x14 + i * 8)
        ents.append({"offset": off, "id": rid, "b0": b0, "b1": b1})
    return count, ents


def read_records(dat):
    recs = []
    pos = DAT_HEADER
    while pos + REC_HEADER <= len(dat):
        magic, flags, size = struct.unpack_from("<IHI", dat, pos)
        if magic != MAGIC:
            raise ValueError(f"magic ausente em 0x{pos:x}")
        recs.append({"pos": pos, "flags": flags, "size": size,
                     "payload": dat[pos + REC_HEADER:pos + REC_HEADER + size]})
        pos += REC_HEADER + size
    return recs, pos


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("base")
    ap.add_argument("--dump")
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    dat = open(a.base + ".DAT", "rb").read()
    hdr = struct.unpack_from("<IHHIH", dat, 0)
    count, ents = read_index(a.base + ".NDX")
    recs, end = read_records(dat)
    by_off = {r["pos"] - DAT_HEADER: r for r in recs}
    print(f"DAT cabeçalho: {[hex(x) for x in hdr]}  registros={len(recs)} fim=0x{end:x}/0x{len(dat):x}")
    print(f"NDX: {count} entradas")
    missing = [e for e in ents if e["offset"] not in by_off]
    print(f"entradas sem registro correspondente: {len(missing)}")
    groups = collections.OrderedDict()
    for e in ents:
        r = by_off.get(e["offset"])
        groups.setdefault((e["b0"], e["b1"]), []).append((e, r))
    for (b0, b1), items in groups.items():
        flags = collections.Counter(r["flags"] for _, r in items if r)
        sizes = [r["size"] for _, r in items if r]
        ids = [e["id"] for e, _ in items]
        print(f"  tipo ({b0:3},{b1:3}): {len(items):4} regs, ids {min(ids)}..{max(ids)}, "
              f"tam {min(sizes)}..{max(sizes)}, flags {dict(flags)}")
        if a.v:
            for e, r in items[:3]:
                print(f"      id {e['id']}: {r['payload'][:32].hex()}")
    if a.dump:
        os.makedirs(a.dump, exist_ok=True)
        for e in ents:
            r = by_off.get(e["offset"])
            if r:
                name = f"t{e['b0']:03}_{e['b1']:03}_id{e['id']:05}_f{r['flags']:x}.bin"
                with open(os.path.join(a.dump, name), "wb") as f:
                    f.write(r["payload"])


if __name__ == "__main__":
    sys.exit(main())
