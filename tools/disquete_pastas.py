"""Disquete para o diálogo de arquivo (telas/arquivos.md): parte de um disquete com .ANT (o muitos.img de
`qemu_dialogo_muitos.txt`) e acrescenta subpastas na raiz e um rótulo de volume, para a rolagem da árvore de
pastas e a lista "Drives:".

Uso: python tools/disquete_pastas.py ENTRADA.img SAIDA.img [N_PASTAS] [ROTULO]
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fat12 import Fat12  # noqa: E402


def main():
    entrada, saida = Path(sys.argv[1]), Path(sys.argv[2])
    n = int(sys.argv[3]) if len(sys.argv) > 3 else 12
    rotulo = sys.argv[4] if len(sys.argv) > 4 else "ANTLABEL"
    d = Fat12(entrada.read_bytes())
    d.gravar(rotulo, b"", atributos=0x08)
    for k in range(1, n + 1):
        nome = f"PASTA{k:02d}"
        d.gravar(nome, bytes(d.tam_cluster), atributos=0x10)
        for i in range(d.nraiz):
            o = d.ini_raiz + i * 32
            if d.img[o:o + 8] == nome.encode().ljust(8):
                ini = struct.unpack_from("<H", d.img, o + 26)[0]
                struct.pack_into("<I", d.img, o + 28, 0)
                od = d.ini_dados + (ini - 2) * d.tam_cluster
                d.img[od:od + 32] = b".          " + bytes([0x10]) + bytes(14) + struct.pack("<HI", ini, 0)
                d.img[od + 32:od + 64] = b"..         " + bytes([0x10]) + bytes(14) + struct.pack("<HI", 0, 0)
                break
    saida.parent.mkdir(parents=True, exist_ok=True)
    saida.write_bytes(bytes(d.img))
    print(list(Fat12(bytes(d.img)).entradas()))


if __name__ == "__main__":
    main()
