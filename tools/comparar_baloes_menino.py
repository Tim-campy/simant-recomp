"""Compara os balões do menino capturados no original (`cenarios_tick.py gravar baloes_menino PASTA`) com
o desenho do port do mesmo estado (docs/spec/telas/quintal.md §2.3, regras/quintal.md).

Para cada `PASTA/balao_KK.ANT` (estado depois do tick) e `balao_KK.png` (captura 640×480 da bancada):
o port desenha a Yard View do estado (`simant --save ... --quintal 0 --exportar-tela-jogo`) com e sem o
balão (`_BoyMessOn` = 0); os pixels que mudam são o balão do port. Compara com a captura (área cliente a
partir da linha 38) na caixa do balão e nos pixels do balão; grava `PASTA/comparacao.json`.

Uso: python -I tools/comparar_baloes_menino.py PASTA [--exe target/release/simant] [--n 23]
"""
import argparse
import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ant_editar import Save  # noqa: E402
from png_simples import ler_png  # noqa: E402

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TOPO = 38


def desenhar(exe, ant, png):
    subprocess.run([exe, "--sem-som", "--save", ant, "--quintal", "0", "--exportar-tela-jogo", png], check=True,
                   capture_output=True, cwd=RAIZ)
    return ler_png(png)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pasta")
    ap.add_argument("--exe", default=os.path.join(RAIZ, "target", "release", "simant"))
    ap.add_argument("--n", type=int, default=23)
    a = ap.parse_args()
    res = []
    for k in range(a.n):
        base = os.path.join(a.pasta, f"balao_{k:02d}")
        s = Save.abrir(base + ".ANT")
        s.escrever("BoyMessOn", 0)
        s.gravar(base + "_sem.ANT")
        w, h, p = desenhar(a.exe, base + ".ANT", base + "_port.png")
        _, _, q = desenhar(a.exe, base + "_sem.ANT", base + "_port_sem.png")
        _, _, c = ler_png(base + ".png")
        mascara = [(x, y) for y in range(h) for x in range(w) if p[y][x] != q[y][x]]
        if not mascara:
            print(f"{k:2d} o port não desenhou balão")
            res.append({"k": k, "erro": "sem balão no port"})
            continue
        x0, x1 = min(m[0] for m in mascara), max(m[0] for m in mascara) + 1
        y0, y1 = min(m[1] for m in mascara), max(m[1] for m in mascara) + 1
        caixa = [(x, y) for y in range(y0, y1) for x in range(x0, x1)]
        ig_caixa = sum(p[y][x] == c[y + TOPO][x] for x, y in caixa)
        ig_mascara = sum(p[y][x] == c[y + TOPO][x] for x, y in mascara)
        r = {"k": k, "caixa": [x0, y0, x1, y1], "caixa_iguais": ig_caixa / len(caixa), "mascara": len(mascara),
             "mascara_iguais": ig_mascara / len(mascara)}
        res.append(r)
        print(f"{k:2d} caixa ({x0}, {y0})-({x1}, {y1}): {100 * r['caixa_iguais']:6.2f}%; "
              f"pixels do balão {len(mascara)}: {100 * r['mascara_iguais']:6.2f}%")
    with open(os.path.join(a.pasta, "comparacao.json"), "w", encoding="utf-8") as f:
        json.dump(res, f, indent=1)


if __name__ == "__main__":
    main()
