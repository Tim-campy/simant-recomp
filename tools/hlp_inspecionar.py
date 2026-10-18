"""Inspeção de um arquivo de ajuda do WinHelp 3.x (SIMANT.HLP): arquivos internos, |SYSTEM, tópicos,
mapa de contextos (|CTXOMAP) e texto dos tópicos. Serve para conferir o leitor do port
(engine/src/formats/hlp.rs). Formato: docs/spec/formatos/hlp.md.

Uso:
    python tools/hlp_inspecionar.py SIMANT.HLP arquivos
    python tools/hlp_inspecionar.py SIMANT.HLP topicos
    python tools/hlp_inspecionar.py SIMANT.HLP contexto 0x2200
    python tools/hlp_inspecionar.py SIMANT.HLP texto N        (texto do tópico N, com os comandos)
"""
import struct
import sys


def u16(d, o):
    return struct.unpack_from("<H", d, o)[0]


def i16(d, o):
    return struct.unpack_from("<h", d, o)[0]


def u32(d, o):
    return struct.unpack_from("<I", d, o)[0]


def i32(d, o):
    return struct.unpack_from("<i", d, o)[0]


def arvore_b(d, inicio, folha):
    """Percorre a árvore B+ que começa em `inicio` (cabeçalho do arquivo interno incluído):
    devolve a lista de entradas lidas por `folha(d, o) -> (entrada, novo_o)`."""
    o = inicio + 9  # FILEHEADER: reservado, usado, flags
    magic, flags, pagina, estrutura = struct.unpack_from("<HHH16s", d, o)
    assert magic == 0x293B, hex(magic)
    raiz, _, total, niveis, n = struct.unpack_from("<hhhhi", d, o + 26)
    paginas = o + 38
    p = raiz
    for _ in range(niveis - 1):
        # página de índice: não usado, n, primeira página filha
        p = i16(d, paginas + p * pagina + 4)
    saida = []
    while p != -1:
        b = paginas + p * pagina
        cont, prox = i16(d, b + 2), i16(d, b + 6)
        o2 = b + 8
        for _ in range(cont):
            e, o2 = folha(d, o2)
            saida.append(e)
        p = prox
    return saida


def diretorio(d):
    dirs = i32(d, 4)

    def folha(d, o):
        f = d.index(b"\0", o)
        return (d[o:f].decode("latin-1"), i32(d, f + 1)), f + 5

    return dict(arvore_b(d, dirs, folha))


def lz77(dados, limite=None):
    saida = bytearray()
    i = 0
    while i < len(dados):
        flags = dados[i]
        i += 1
        for bit in range(8):
            if i >= len(dados):
                break
            if flags & (1 << bit):
                v = u16(dados, i)
                i += 2
                n = ((v >> 12) & 0xF) + 3
                dist = (v & 0xFFF) + 1
                for _ in range(n):
                    saida.append(saida[-dist])
            else:
                saida.append(dados[i])
                i += 1
            if limite and len(saida) >= limite:
                return bytes(saida[:limite])
    return bytes(saida)


class Hlp:
    def __init__(self, caminho):
        self.d = open(caminho, "rb").read()
        self.arquivos = diretorio(self.d)
        self.sistema()
        self.frases()
        self.topicos()

    def interno(self, nome):
        o = self.arquivos[nome]
        usado = i32(self.d, o + 4)
        return self.d[o + 9:o + 9 + usado]

    def sistema(self):
        s = self.interno("|SYSTEM")
        magic, menor, maior, data, flags = struct.unpack_from("<HHHiH", s, 0)
        self.menor, self.flags_sis = menor, flags
        self.comprimido = menor > 16 and flags in (4, 8)
        self.bloco = 2048 if (menor <= 16 or flags == 8) else 4096
        self.titulo = None
        self.registros = []
        if menor <= 16:
            self.titulo = s[12:s.index(b"\0", 12)].decode("latin-1")
        else:
            o = 12
            while o + 4 <= len(s):
                t, n = u16(s, o), u16(s, o + 2)
                self.registros.append((t, s[o + 4:o + 4 + n]))
                if t == 1:
                    self.titulo = s[o + 4:o + 4 + n].split(b"\0")[0].decode("latin-1")
                o += 4 + n

    def frases(self):
        self.lista_frases = []
        if "|Phrases" not in self.arquivos:
            return
        p = self.interno("|Phrases")
        n, cem = u16(p, 0), u16(p, 2)
        if self.comprimido:
            tam = u32(p, 4)
            offs = [u16(p, 8 + 2 * k) for k in range(n + 1)]
            base = 8 + 2 * (n + 1)
            texto = lz77(p[base:], tam)
            ini = 2 * (n + 1)
            self.lista_frases = [texto[offs[k] - ini:offs[k + 1] - ini] for k in range(n)]
        else:
            # HC30: deslocamentos relativos ao início da tabela (o primeiro = 2 * (n + 1))
            offs = [u16(p, 4 + 2 * k) for k in range(n + 1)]
            self.lista_frases = [p[4 + offs[k]:4 + offs[k + 1]] for k in range(n)]

    def expandir(self, dados):
        saida = bytearray()
        i = 0
        while i < len(dados):
            c = dados[i]
            if 0 < c < 10 and self.lista_frases:
                idx = 256 * (c - 1) + dados[i + 1]
                saida += self.lista_frases[idx // 2]
                if idx & 1:
                    saida += b" "
                i += 2
            else:
                saida.append(c)
                i += 1
        return bytes(saida)

    def blocos(self):
        t = self.interno("|TOPIC")
        n = (len(t) + self.bloco - 1) // self.bloco
        saida = []
        for b in range(n):
            bruto = t[b * self.bloco:(b + 1) * self.bloco]
            cab = struct.unpack_from("<iii", bruto, 0)
            corpo = lz77(bruto[12:], 16384) if self.comprimido else bruto[12:]
            saida.append((cab, corpo))
        return saida

    def ler_hc30(self, t, pos, n):
        """n bytes do |TOPIC de um arquivo HC30 a partir da posição bruta `pos`, pulando os cabeçalhos
        de 12 bytes dos blocos de 2048 que estiverem no caminho."""
        saida = bytearray()
        while len(saida) < n and pos < len(t):
            off = pos % self.bloco
            if off < 12:
                pos += 12 - off
                continue
            k = min(n - len(saida), self.bloco - off)
            saida += t[pos:pos + k]
            pos += k
        return bytes(saida)

    def topicos(self):
        """Tópicos: posição do cabeçalho, título e registros de texto (tipo, dados1, dados2)."""
        self.topicos_lista = []
        if self.menor > 16:
            raise SystemExit("só HC30 (Windows 3.0) neste script")
        t = self.interno("|TOPIC")
        pos = 12
        atual = None
        while 0 < pos < len(t):
            cab = self.ler_hc30(t, pos, 21)
            if len(cab) < 21:
                break
            tam, d2len, ant, prox, d1len, tipo = struct.unpack_from("<iiiiiB", cab, 0)
            dados = self.ler_hc30(t, pos, tam)
            d1 = dados[21:d1len]
            d2bruto = dados[d1len:tam]
            d2 = self.expandir(d2bruto) if d2len > len(d2bruto) else d2bruto
            if tipo == 2:
                titulo = d2.split(b"\0")[0].decode("latin-1")
                atual = {"offset": pos, "titulo": titulo, "registros": [], "cab": d1}
                self.topicos_lista.append(atual)
            elif atual is not None:
                atual["registros"].append((tipo, d1, d2))
            if prox <= 0:
                break
            pos += prox

    def contexto(self, n):
        c = self.interno("|CTXOMAP")
        k = u16(c, 0)
        for j in range(k):
            mid, off = struct.unpack_from("<iI", c, 2 + 8 * j)
            if mid == n:
                return off
        return None

    def topico_do_offset(self, off):
        melhor = None
        for t in self.topicos_lista:
            if t["offset"] <= off:
                melhor = t
        return melhor


def ler_long(d, o):
    if d[o] & 1:
        return u32(d, o) >> 1, o + 4
    return u16(d, o) >> 1, o + 2


def ler_short(d, o):
    if d[o] & 1:
        return u16(d, o) >> 1, o + 2
    return d[o] >> 1, o + 1


def main():
    h = Hlp(sys.argv[1])
    cmd = sys.argv[2]
    if cmd == "arquivos":
        print("menor", h.menor, "flags", h.flags_sis, "comprimido", h.comprimido, "bloco", h.bloco, "título", h.titulo)
        for k, v in sorted(h.arquivos.items(), key=lambda x: x[1]):
            print(f"{v:8} {k}")
        print("frases", len(h.lista_frases), h.lista_frases[:10])
        print("registros |SYSTEM", [(t, len(x)) for t, x in h.registros])
    elif cmd == "topicos":
        for i, t in enumerate(h.topicos_lista):
            print(i, hex(t["offset"]), t["titulo"], len(t["registros"]))
    elif cmd == "contexto":
        n = int(sys.argv[3], 0)
        off = h.contexto(n)
        t = h.topico_do_offset(off) if off is not None else None
        print(hex(n), "->", hex(off) if off is not None else None, t and t["titulo"])
    elif cmd == "texto":
        t = h.topicos_lista[int(sys.argv[3])]
        print(t["titulo"])
        for tipo, d1, d2 in t["registros"]:
            print(tipo, d1.hex(), d2)


if __name__ == "__main__":
    main()
