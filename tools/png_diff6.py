"""Compara dois PNG do mesmo tamanho ignorando a expansão do DAC (compara cada canal >> 2, ou seja, em 6 bits).

Uso: python tools/png_diff6.py A.png B.png [X0 Y0 X1 Y1]
Útil para capturas do QEMU (DAC de 8 bits ou 6 bits << 2) contra o DOSBox-X (6 bits expandidos: 0x20 -> 0x82).
Mostra quantos pixels diferem e o retângulo que os contém.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from png_cores import ler_png  # noqa: E402


def main():
    wa, ha, ba, la = ler_png(sys.argv[1])
    wb, hb, bb, lb = ler_png(sys.argv[2])
    x0, y0, x1, y1 = (int(v) for v in sys.argv[3:7]) if len(sys.argv) >= 7 else (0, 0, min(wa, wb), min(ha, hb))
    n, caixa = 0, None
    for y in range(y0, y1):
        ra, rb = la[y], lb[y]
        for x in range(x0, x1):
            a = tuple(v >> 2 for v in ra[x * ba:x * ba + 3])
            b = tuple(v >> 2 for v in rb[x * bb:x * bb + 3])
            if a != b:
                n += 1
                caixa = (x, y, x, y) if caixa is None else (min(caixa[0], x), min(caixa[1], y),
                                                            max(caixa[2], x), max(caixa[3], y))
    print(f"{n} pixels diferentes em {x1 - x0}x{y1 - y0}; retângulo: {caixa}")


if __name__ == "__main__":
    main()
