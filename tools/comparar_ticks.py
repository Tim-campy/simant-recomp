"""Compara dois diretórios de vetores de tick (saída de tools/gerar_vetores_tick.py), tick a tick.

Uso: python tools/comparar_ticks.py DIR_A DIR_B [--detalhe]

Para cada tick_NNNNNN presente em DIR_A, diz se os .bin (estado_antes, estado_depois e, se houver, os
segmentos inteiros seg*_*.bin) e o extras.json são iguais aos de DIR_B. Com --detalhe, mostra os blocos
diferentes dos estados (via estado_diff) e as faixas de bytes diferentes dos segmentos.
Sai com código 1 se houver diferença.

Uso 2: python tools/comparar_ticks.py --fora-do-save DIR [DIR ...]
Com vetores gerados com --segmentos, lista as variáveis FORA do layout do save que o tick altera
(antes -> depois), com o símbolo mais próximo de re_trabalho/symbols.tsv.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))


def simbolos():
    """{seg: [(offset, nome), ...]} das variáveis de re_trabalho/symbols.tsv (via SIMANT_RAIZ)."""
    import qemu_bancada as qb
    out = {}
    if qb.SIMBOLOS.exists():
        for linha in qb.SIMBOLOS.read_text(encoding="latin-1").splitlines():
            p = linha.split("\t")
            if len(p) >= 3 and p[0] in ("8", "9", "10"):
                out.setdefault(int(p[0]), []).append((int(p[1]), p[2]))
    for v in out.values():
        v.sort()
    return out


def nome(simb, seg, off):
    """Símbolo anterior mais próximo: '_Nome+desloc'."""
    melhor = None
    for o, n in simb.get(seg, []):
        if o <= off:
            melhor = (o, n)
    return f"{melhor[1]}+{off - melhor[0]:#x}" if melhor else "?"


def faixas(da, db):
    out, ini = [], None
    for i in range(max(len(da), len(db))):
        dif = i >= len(da) or i >= len(db) or da[i] != db[i]
        if dif and ini is None:
            ini = i
        if not dif and ini is not None:
            out.append((ini, i))
            ini = None
    if ini is not None:
        out.append((ini, max(len(da), len(db))))
    return out


def fora_do_save(dirs):
    """Bytes dos segmentos 8, 9 e 10 que o tick (antes -> depois) muda FORA dos blocos do save.
    Agrega vários diretórios tick_* e lista cada faixa com o símbolo e quantos ticks a mudaram."""
    import qemu_bancada as qb
    cobertos = {}
    for b in qb.carregar_layout():
        cobertos.setdefault(b["seg"], set()).update(range(b["off"], b["off"] + b["n"]))
    simb = simbolos()
    contagem = {}
    for d in dirs:
        for seg in (8, 9, 10):
            fa, fb = d / f"seg{seg}_antes.bin", d / f"seg{seg}_depois.bin"
            if not fa.exists():
                continue
            for i, j in faixas(fa.read_bytes(), fb.read_bytes()):
                for k in range(i, j):
                    if k not in cobertos.get(seg, ()):
                        n = nome(simb, seg, k)
                        chave = (seg, n.split("+")[0])
                        contagem.setdefault(chave, set()).add((d.name, k))
    for (seg, n), ocorr in sorted(contagem.items(), key=lambda x: (x[0][0], min(k for _, k in x[1]))):
        offs = sorted({k for _, k in ocorr})
        ticks = len({t for t, _ in ocorr})
        print(f"{seg}:{offs[0]:04x}..{offs[-1]:04x}  {n:<28} {len(offs):5d} bytes, mudou em {ticks} tick(s)")


def continuidade(d):
    """Entre ticks consecutivos de UMA execução: o que muda do retorno do tick N até a entrada do N+1
    (código fora de _DoAntSim: desenho, mensagens, laço do MYTIMERFUNC)."""
    import estado_diff
    layout = json.loads(estado_diff.LAYOUT.read_text(encoding="utf-8"))["blocos"]
    ticks = sorted(d.glob("tick_*"))
    for pa, pb in zip(ticks, ticks[1:]):
        ea = json.loads((pa / "extras.json").read_text(encoding="utf-8"))["depois"]
        eb = json.loads((pb / "extras.json").read_text(encoding="utf-8"))["antes"]
        mud = {k: (ea[k], eb[k]) for k in ea if k not in ("posicao",) and ea[k] != eb[k]}
        da, db = (pa / "estado_depois.bin").read_bytes(), (pb / "estado_antes.bin").read_bytes()
        blocos = []
        if da != db:
            pos = 0
            for b in layout:
                n = b["count"] * b["tamanho"]
                if da[pos:pos + n] != db[pos:pos + n]:
                    blocos.append(b["simbolo"])
                pos += n
        segs = []
        simb = None
        for seg in (8, 9, 10):
            fa, fb = pa / f"seg{seg}_depois.bin", pb / f"seg{seg}_antes.bin"
            if fa.exists() and fb.exists():
                simb = simb or simbolos()
                nomes = {nome(simb, seg, i).split("+")[0] for i, _ in faixas(fa.read_bytes(), fb.read_bytes())}
                segs += [f"{seg}:{n}" for n in sorted(nomes)]
        print(f"{pa.name} -> {pb.name}: save {'igual' if da == db else 'DIFERENTE ' + ', '.join(blocos)}; "
              f"extras que mudaram: {mud}" + (f"; segmentos: {', '.join(segs)}" if segs else ""))


def main():
    args = sys.argv[1:]
    if args and args[0] == "--continuidade":
        continuidade(Path(args[1]))
        return
    if args and args[0] == "--fora-do-save":
        dirs = [p for a in args[1:] for p in sorted(Path(a).glob("tick_*"))]
        fora_do_save(dirs)
        return
    detalhe = "--detalhe" in args
    args = [x for x in args if x != "--detalhe"]
    a, b = Path(args[0]), Path(args[1])
    difs = 0
    for pa in sorted(a.glob("tick_*")):
        pb = b / pa.name
        if not pb.exists():
            print(f"{pa.name}: ausente em {b}")
            difs += 1
            continue
        for f in sorted(pa.glob("*.bin")):
            if not (pb / f.name).exists():
                print(f"{pa.name} {f.name}: ausente em {b}")
                continue
            da, db = f.read_bytes(), (pb / f.name).read_bytes()
            fx = faixas(da, db)
            n = sum(j - i for i, j in fx)
            print(f"{pa.name} {f.name}: " + ("iguais" if not fx else f"{n} bytes diferentes"))
            if fx:
                difs += 1
                if detalhe and f.name.startswith("estado"):
                    import estado_diff
                    sys.argv = ["estado_diff", str(f), str(pb / f.name)]
                    try:
                        estado_diff.main()
                    except SystemExit:
                        pass
                elif detalhe:
                    seg = int(f.name[3:].split("_")[0])
                    simb = simbolos()
                    for i, j in fx[:40]:
                        print(f"   {seg}:{i:04x}-{j - 1:04x}  {nome(simb, seg, i)}  "
                              f"{da[i:j].hex()} -> {db[i:j].hex()}")
        ea = json.loads((pa / "extras.json").read_text(encoding="utf-8"))
        eb = json.loads((pb / "extras.json").read_text(encoding="utf-8"))
        if ea != eb:
            difs += 1
            for fase in ea:
                for k in ea[fase]:
                    if ea[fase][k] != eb[fase].get(k):
                        print(f"{pa.name} extras {fase}.{k}: {ea[fase][k]} != {eb[fase].get(k)}")
        else:
            print(f"{pa.name} extras.json: iguais")
    sys.exit(1 if difs else 0)


if __name__ == "__main__":
    main()
