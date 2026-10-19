"""Compara a área cliente exportada pelo port com uma captura da bancada em 6 bits por canal (cores exatas,
sem o mapeamento de cores do --mapear de comparar_captura.py): serve para conferir CORES nas capturas de 256
cores da bancada SVGA (o DAC do QEMU dá 0x80/0xC0; o port grava 8 bits).

Uso: python tools/comparar_captura6.py PORT.png CAPTURA.png X0 Y0 X1 Y1 [--topo 38] [--diff SAIDA.png [--zoom N]]
  X0 Y0 X1 Y1: retângulo [X0, X1) x [Y0, Y1) em coordenadas do port (área cliente).
Imprime a porcentagem de pixels iguais, a caixa das diferenças e as cores que mais diferem.
"""
import argparse
from collections import Counter

from comparar_captura import salvar_png
from png_simples import ler_png


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("port")
    ap.add_argument("captura")
    ap.add_argument("regiao", nargs=4, type=int)
    ap.add_argument("--topo", type=int, default=38)
    ap.add_argument("--diff")
    ap.add_argument("--zoom", type=int, default=1)
    a = ap.parse_args()
    wp, hp, port = ler_png(a.port)
    wc, hc, cap = ler_png(a.captura)
    x0, y0, x1, y1 = a.regiao
    iguais = total = 0
    caixa = None
    pares = Counter()
    linhas = []
    for y in range(y0, y1):
        lin = []
        for x in range(x0, x1):
            p = port[y][x]
            c = cap[y + a.topo][x]
            if p == (255, 0, 255):  # fora do que o exemplo desenhou (barra_menus): não conta
                lin.append((40, 40, 40))
                continue
            ok = all((u >> 2) == (v >> 2) for u, v in zip(p, c))
            total += 1
            if ok:
                iguais += 1
                lin.append(tuple(v // 3 for v in c))
            else:
                lin.append((255, 0, 0))
                pares[("#%02x%02x%02x" % p, "#%02x%02x%02x" % c)] += 1
                caixa = (x, y, x, y) if caixa is None else (min(caixa[0], x), min(caixa[1], y),
                                                             max(caixa[2], x), max(caixa[3], y))
        linhas.append(lin)
    print(f"{iguais}/{total} pixels iguais ({100 * iguais / total:.2f}%); diferenças em {caixa}")
    for (p, c), n in pares.most_common(6):
        print(f"  port {p} x captura {c}: {n}")
    if a.diff:
        # captura | port | diferenças, lado a lado, ampliados `--zoom` vezes
        z = a.zoom
        lado = []
        for k, y in enumerate(range(y0, y1)):
            lin = ([cap[y + a.topo][x] for x in range(x0, x1)] + [(255, 0, 255)]
                   + [port[y][x] for x in range(x0, x1)] + [(255, 0, 255)] + linhas[k])
            lin = [px for px in lin for _ in range(z)]
            lado += [lin] * z
        salvar_png(a.diff, len(lado[0]), len(lado), lado)


if __name__ == "__main__":
    main()
