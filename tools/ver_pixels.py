"""Mostra uma região de uma captura PNG como texto, um caractere por pixel (para medir à mão).

Uso: python tools/ver_pixels.py ARQ.png X0 Y0 X1 Y1
Cores do VGA de 16 cores: '#' preto, 'w' branco, '.' cinza-claro (192), ':' cinza-escuro (128), 'b' azul-escuro
(0,0,128), 'B' azul, 'g' verde-escuro, 'G' verde, 'c' ciano-escuro, 'C' ciano, 'r' vermelho-escuro, 'R' vermelho,
'm' magenta-escuro, 'M' magenta, 'y' oliva, 'Y' amarelo; outras cores: '?'. Na saída, a primeira coluna é o y.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from png_simples import ler_png  # noqa: E402

CORES = {
    (0, 0, 0): "#", (255, 255, 255): "w", (192, 192, 192): ".", (128, 128, 128): ":", (0, 0, 128): "b",
    (0, 0, 255): "B", (0, 128, 0): "g", (0, 255, 0): "G", (0, 128, 128): "c", (0, 255, 255): "C",
    (128, 0, 0): "r", (255, 0, 0): "R", (128, 0, 128): "m", (255, 0, 255): "M", (128, 128, 0): "y",
    (255, 255, 0): "Y",
}


def perto(p):
    melhor = min(CORES, key=lambda c: sum((a - b) ** 2 for a, b in zip(c, p)))
    return CORES[melhor] if sum((a - b) ** 2 for a, b in zip(melhor, p)) < 3 * 40 ** 2 else "?"


def main():
    arq, x0, y0, x1, y1 = sys.argv[1], *map(int, sys.argv[2:6])
    w, h, px = ler_png(arq)
    print("     " + "".join(str((x // 10) % 10) if x % 10 == 0 else " " for x in range(x0, x1)))
    for y in range(y0, min(y1, h)):
        print(f"{y:4} " + "".join(perto(px[y][x]) for x in range(x0, min(x1, w))))


if __name__ == "__main__":
    main()
