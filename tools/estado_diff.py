"""Compara dois arquivos no formato do save .ANT (jogo salvo, `estado` da bancada QEMU ou dump do port)
bloco a bloco, segundo data/save_layout.json.

Uso: python tools/estado_diff.py A.ANT B.ANT [--max N]
Para cada bloco diferente: nome, bytes diferentes e, para blocos de até 4 bytes, os valores (inteiro
little-endian com sinal). Termina com o total. Código de saída 0 se iguais, 1 se diferentes.
"""
import json
import sys
from pathlib import Path

LAYOUT = Path(__file__).resolve().parent.parent / "data" / "save_layout.json"


def main():
    args = sys.argv[1:]
    maximo = 40
    if "--max" in args:
        k = args.index("--max")
        maximo = int(args[k + 1])
        del args[k:k + 2]
    a, b = Path(args[0]).read_bytes(), Path(args[1]).read_bytes()
    blocos = json.loads(LAYOUT.read_text(encoding="utf-8"))["blocos"]
    if len(a) != len(b):
        print(f"tamanhos diferentes: {len(a)} x {len(b)}")
    pos = difs = 0
    for bl in blocos:
        n = bl["count"] * bl["tamanho"]
        x, y = a[pos:pos + n], b[pos:pos + n]
        if x != y:
            nd = sum(1 for i, j in zip(x, y) if i != j)
            difs += 1
            if difs <= maximo:
                extra = ""
                if n <= 4:
                    extra = f": {int.from_bytes(x, 'little', signed=True)} -> {int.from_bytes(y, 'little', signed=True)}"
                print(f"{bl['simbolo']:<18} pos {pos:6d} tam {n:5d}  {nd:5d} bytes diferentes{extra}")
        pos += n
    print(f"{difs} de {len(blocos)} blocos diferentes" if difs else f"iguais ({len(a)} bytes, {len(blocos)} blocos)")
    sys.exit(1 if difs else 0)


if __name__ == "__main__":
    main()
