"""Inspeciona um executável NE (Windows 3.x) e, opcionalmente, o .SYM (MAPSYM) que o acompanha.

Uso:
    python tools/ne_info.py SIMANTW.EXE [--sym SIMANTW.SYM] [--json saida.json]

Só lê os arquivos; não escreve nada além do JSON pedido.
Referência do formato: Microsoft "Executable-File Header Format" (NE) e MAPSYM.
"""
import argparse
import json
import struct
import sys

RT_NAMES = {
    1: "CURSOR", 2: "BITMAP", 3: "ICON", 4: "MENU", 5: "DIALOG", 6: "STRING",
    7: "FONTDIR", 8: "FONT", 9: "ACCELERATOR", 10: "RCDATA", 12: "GROUP_CURSOR",
    14: "GROUP_ICON", 16: "VERSION",
}


def u8(b, o):
    return b[o]


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def pstr(b, o):
    n = b[o]
    return b[o + 1:o + 1 + n].decode("latin-1")


def parse_ne(data):
    ne = u16(data, 0x3C)
    if data[ne:ne + 2] != b"NE":
        raise ValueError("não é NE")
    h = {
        "ne_offset": ne,
        "linker": f"{data[ne + 2]}.{data[ne + 3]}",
        "flags": u16(data, ne + 0x0C),
        "autodata_seg": u16(data, ne + 0x0E),
        "heap": u16(data, ne + 0x10),
        "stack": u16(data, ne + 0x12),
        "entry_ip": u16(data, ne + 0x14),
        "entry_cs": u16(data, ne + 0x16),
        "seg_count": u16(data, ne + 0x1C),
        "modref_count": u16(data, ne + 0x1E),
        "align_shift": u16(data, ne + 0x32),
        "target_os": data[ne + 0x36],
        "win_version": f"{data[ne + 0x3F]}.{data[ne + 0x3E]:02d}",
    }
    seg_tab = ne + u16(data, ne + 0x22)
    res_tab = ne + u16(data, ne + 0x24)
    resname_tab = ne + u16(data, ne + 0x26)
    modref_tab = ne + u16(data, ne + 0x28)
    impname_tab = ne + u16(data, ne + 0x2A)
    shift = h["align_shift"]

    segs = []
    for i in range(h["seg_count"]):
        o = seg_tab + i * 8
        sector, length, flags, minalloc = struct.unpack_from("<HHHH", data, o)
        segs.append({
            "index": i + 1,
            "file_offset": sector << shift,
            "file_length": length or (0x10000 if sector else 0),
            "flags": flags,
            "kind": "DATA" if flags & 1 else "CODE",
            "has_relocs": bool(flags & 0x100),
            "min_alloc": minalloc or 0x10000,
        })

    modules = []
    for i in range(h["modref_count"]):
        off = u16(data, modref_tab + i * 2)
        modules.append(pstr(data, impname_tab + off))

    names = []
    o = resname_tab
    while data[o]:
        names.append((pstr(data, o), u16(data, o + 1 + data[o])))
        o += 1 + data[o] + 2

    # Tabela de recursos
    resources = []
    rshift = u16(data, res_tab)
    o = res_tab + 2
    while True:
        type_id = u16(data, o)
        if type_id == 0:
            break
        count = u16(data, o + 2)
        o += 8
        tname = RT_NAMES.get(type_id & 0x7FFF, str(type_id & 0x7FFF)) if type_id & 0x8000 \
            else pstr(data, res_tab + type_id)
        for _ in range(count):
            off, ln, fl, rid = struct.unpack_from("<HHHH", data, o)
            o += 12
            rname = (rid & 0x7FFF) if rid & 0x8000 else pstr(data, res_tab + rid)
            resources.append({
                "type": tname, "id": rname,
                "file_offset": off << rshift, "length": ln << rshift, "flags": fl,
            })

    imports = collect_imports(data, segs, modules, impname_tab)
    return {"header": h, "segments": segs, "modules": modules,
            "resident_names": names, "resources": resources, "imports": imports}


def collect_imports(data, segs, modules, impname_tab):
    """Lê as tabelas de relocação dos segmentos e conta as importações por módulo."""
    out = {}
    for s in segs:
        if not s["has_relocs"]:
            continue
        o = s["file_offset"] + s["file_length"]
        n = u16(data, o)
        o += 2
        for _ in range(n):
            src, flags, _off, a, b = struct.unpack_from("<BBHHH", data, o)
            o += 8
            kind = flags & 3
            if kind == 1:  # importado por ordinal
                key = f"{modules[a - 1]}.@{b}"
            elif kind == 2:  # importado por nome
                key = f"{modules[a - 1]}.{pstr(data, impname_tab + b)}"
            else:
                continue
            out[key] = out.get(key, 0) + 1
    return dict(sorted(out.items()))


def parse_sym(data):
    """MAPSYM (.SYM): segmentos e símbolos (offset, nome)."""
    cb_max = data[0x0E]
    mod = pstr(data, 0x0F)
    c_consts = u16(data, 0x06)
    p_const = u16(data, 0x08)
    consts = []
    for i in range(c_consts):
        po = u16(data, p_const + i * 2)
        consts.append({"value": u16(data, po), "name": pstr(data, po + 2)})
    segs = []
    seg_para = u16(data, 0x0C)
    for _ in range(u16(data, 0x0A)):
        base = seg_para * 16
        next_para, n_syms, p_symptr, lsa = struct.unpack_from("<HHHH", data, base)
        flags = data[base + 0x0C]
        name = pstr(data, base + 0x14)
        is32 = bool(flags & 1)
        syms = []
        for i in range(n_syms):
            so = base + u16(data, base + p_symptr + i * 2)
            if is32:
                off = struct.unpack_from("<I", data, so)[0]
                nm = pstr(data, so + 4)
            else:
                off = u16(data, so)
                nm = pstr(data, so + 2)
            syms.append({"offset": off, "name": nm})
        segs.append({"name": name, "load_index": lsa, "symbols": syms})
        seg_para = next_para
        if not seg_para:
            break
    return {"module": mod, "cb_max": cb_max, "constants": consts, "segments": segs}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("exe")
    ap.add_argument("--sym")
    ap.add_argument("--json")
    a = ap.parse_args()
    with open(a.exe, "rb") as f:
        info = parse_ne(f.read())
    if a.sym:
        with open(a.sym, "rb") as f:
            info["sym"] = parse_sym(f.read())
    if a.json:
        with open(a.json, "w", encoding="utf-8") as f:
            json.dump(info, f, indent=1, ensure_ascii=False)
    h = info["header"]
    print(f"NE linker {h['linker']}, Windows {h['win_version']}, flags=0x{h['flags']:04x}, "
          f"entrada {h['entry_cs']}:{h['entry_ip']:04x}, autodata seg {h['autodata_seg']}")
    for s in info["segments"]:
        print(f"  seg {s['index']:2}: {s['kind']} arq=0x{s['file_offset']:06x} "
              f"tam=0x{s['file_length']:04x} flags=0x{s['flags']:04x}")
    print("módulos:", ", ".join(info["modules"]))
    by_type = {}
    for r in info["resources"]:
        by_type.setdefault(r["type"], []).append(r["id"])
    for t, ids in by_type.items():
        print(f"  recurso {t}: {len(ids)} -> {ids[:12]}{' ...' if len(ids) > 12 else ''}")
    if "sym" in info:
        for s in info["sym"]["segments"]:
            print(f"  SYM seg {s['name']} (lsa {s['load_index']}): {len(s['symbols'])} símbolos")


if __name__ == "__main__":
    sys.exit(main())
