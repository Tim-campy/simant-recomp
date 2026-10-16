"""Leitura e montagem mínima de imagens de disquete FAT12 (só diretório raiz), sem dependências.

Usado para montar o disquete de boot FreeDOS da bancada QEMU (tools/montar_disquete_qemu.py).
"""
import struct


class Fat12:
    def __init__(self, imagem: bytes):
        self.img = bytearray(imagem)
        b = self.img
        self.bps = struct.unpack_from("<H", b, 11)[0]
        self.spc = b[13]
        self.reservados = struct.unpack_from("<H", b, 14)[0]
        self.nfats = b[16]
        self.nraiz = struct.unpack_from("<H", b, 17)[0]
        self.spf = struct.unpack_from("<H", b, 22)[0]
        self.ini_fat = self.reservados * self.bps
        self.ini_raiz = self.ini_fat + self.nfats * self.spf * self.bps
        self.ini_dados = self.ini_raiz + self.nraiz * 32
        self.tam_cluster = self.spc * self.bps
        self.total_clusters = (len(b) - self.ini_dados) // self.tam_cluster

    def _fat(self, n):
        o = self.ini_fat + n * 3 // 2
        v = struct.unpack_from("<H", self.img, o)[0]
        return v >> 4 if n & 1 else v & 0xFFF

    def _set_fat(self, n, valor):
        for k in range(self.nfats):
            o = self.ini_fat + k * self.spf * self.bps + n * 3 // 2
            v = struct.unpack_from("<H", self.img, o)[0]
            v = (v & 0x000F) | (valor << 4) if n & 1 else (v & 0xF000) | valor
            struct.pack_into("<H", self.img, o, v)

    def entradas(self):
        for i in range(self.nraiz):
            o = self.ini_raiz + i * 32
            e = self.img[o:o + 32]
            if e[0] == 0:
                break
            if e[0] == 0xE5 or e[11] & 0x08:
                continue
            nome = e[0:8].decode("latin-1").rstrip() + ("." + e[8:11].decode("latin-1").rstrip() if e[8:11].strip() else "")
            yield nome, e[11], struct.unpack_from("<H", e, 26)[0], struct.unpack_from("<I", e, 28)[0]

    def ler(self, nome):
        for n, _atr, cl, tam in self.entradas():
            if n.upper() == nome.upper():
                dados = bytearray()
                while 2 <= cl < 0xFF8:
                    o = self.ini_dados + (cl - 2) * self.tam_cluster
                    dados += self.img[o:o + self.tam_cluster]
                    cl = self._fat(cl)
                return bytes(dados[:tam])
        raise FileNotFoundError(nome)

    def _cadeia(self, cl):
        dados = bytearray()
        while 2 <= cl < 0xFF8:
            o = self.ini_dados + (cl - 2) * self.tam_cluster
            dados += self.img[o:o + self.tam_cluster]
            cl = self._fat(cl)
        return bytes(dados)

    def ler_caminho(self, caminho):
        """Lê um arquivo por caminho (ex.: FREEDOS/BIN/COMMAND.COM), descendo pelos subdiretórios."""
        partes = caminho.replace("\\", "/").upper().split("/")
        bloco = self.img[self.ini_raiz:self.ini_dados]
        for i, parte in enumerate(partes):
            for k in range(0, len(bloco), 32):
                e = bloco[k:k + 32]
                if e[0] in (0, 0xE5) or e[11] & 0x08:
                    continue
                nome = e[0:8].decode("latin-1").rstrip() + ("." + e[8:11].decode("latin-1").rstrip() if e[8:11].strip() else "")
                if nome == parte:
                    cl, tam = struct.unpack_from("<H", e, 26)[0], struct.unpack_from("<I", e, 28)[0]
                    if i == len(partes) - 1:
                        return self._cadeia(cl)[:tam]
                    bloco = self._cadeia(cl)
                    break
            else:
                raise FileNotFoundError(caminho)

    @classmethod
    def vazio_a_partir_de(cls, modelo: "Fat12"):
        """Novo disquete com o mesmo setor de boot (e BPB) do modelo, FAT e raiz zeradas."""
        img = bytearray(len(modelo.img))
        img[:modelo.bps] = modelo.img[:modelo.bps]
        novo = cls(bytes(img))
        midia = modelo.img[21]
        novo._set_fat(0, 0xF00 | midia)
        novo._set_fat(1, 0xFFF)
        return novo

    def gravar(self, nome, dados: bytes, atributos=0x20):
        """Grava um arquivo na raiz em clusters contíguos (o setor de boot do FreeDOS lê pela FAT)."""
        base, _, ext = nome.upper().partition(".")
        entrada = base.ljust(8)[:8].encode("latin-1") + ext.ljust(3)[:3].encode("latin-1")
        livre = next(i for i in range(self.nraiz) if self.img[self.ini_raiz + i * 32] in (0, 0xE5))
        usados = {c for c in range(2, self.total_clusters + 2) if self._fat(c)}
        n = max(1, -(-len(dados) // self.tam_cluster))
        inicio = next(c for c in range(2, self.total_clusters + 2) if all(c + k not in usados for k in range(n)))
        for k in range(n):
            self._set_fat(inicio + k, inicio + k + 1 if k < n - 1 else 0xFFF)
            o = self.ini_dados + (inicio + k - 2) * self.tam_cluster
            pedaco = dados[k * self.tam_cluster:(k + 1) * self.tam_cluster]
            self.img[o:o + len(pedaco)] = pedaco
        o = self.ini_raiz + livre * 32
        self.img[o:o + 32] = entrada + bytes([atributos]) + bytes(14) + struct.pack("<HI", inicio if dados else 0, len(dados))
