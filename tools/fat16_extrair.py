"""Extrai um arquivo de uma imagem de disco rígido FAT16 particionada (como a c.img da bancada QEMU).

Uso: python tools/fat16_extrair.py IMAGEM CAMINHO_NO_DISCO DESTINO
     ex.: python tools/fat16_extrair.py re_trabalho/qemu/c.img MAXIS/ANTWIN/QUICK1.ANT tests/referencias/saves/QUICK1.ANT
"""
import struct
import sys
from pathlib import Path


def ler(imagem: bytes, caminho: str) -> bytes:
    inicio = struct.unpack_from("<I", imagem, 0x1BE + 8)[0] * 512  # primeira partição
    v = imagem[inicio:]
    bps = struct.unpack_from("<H", v, 11)[0]
    spc = v[13]
    reservados = struct.unpack_from("<H", v, 14)[0]
    nfats = v[16]
    nraiz = struct.unpack_from("<H", v, 17)[0]
    spf = struct.unpack_from("<H", v, 22)[0]
    ini_fat = reservados * bps
    ini_raiz = ini_fat + nfats * spf * bps
    ini_dados = ini_raiz + nraiz * 32
    tam_cluster = spc * bps

    def cadeia(cl):
        dados = bytearray()
        while 2 <= cl < 0xFFF8:
            o = ini_dados + (cl - 2) * tam_cluster
            dados += v[o:o + tam_cluster]
            cl = struct.unpack_from("<H", v, ini_fat + cl * 2)[0]
        return bytes(dados)

    bloco = v[ini_raiz:ini_dados]
    partes = caminho.replace("\\", "/").upper().split("/")
    for i, parte in enumerate(partes):
        for k in range(0, len(bloco), 32):
            e = bloco[k:k + 32]
            if not e or e[0] == 0:
                break
            if e[0] == 0xE5 or e[11] & 0x08:
                continue
            nome = e[0:8].decode("latin-1").rstrip()
            ext = e[8:11].decode("latin-1").rstrip()
            if (nome + ("." + ext if ext else "")) == parte:
                cl, tam = struct.unpack_from("<H", e, 26)[0], struct.unpack_from("<I", e, 28)[0]
                if i == len(partes) - 1:
                    return cadeia(cl)[:tam]
                bloco = cadeia(cl)
                break
        else:
            raise FileNotFoundError(caminho)


if __name__ == "__main__":
    dados = ler(Path(sys.argv[1]).read_bytes(), sys.argv[2])
    Path(sys.argv[3]).parent.mkdir(parents=True, exist_ok=True)
    Path(sys.argv[3]).write_bytes(dados)
    print(f"{sys.argv[2]}: {len(dados)} bytes -> {sys.argv[3]}")
