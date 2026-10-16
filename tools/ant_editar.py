"""Editor do jogo salvo .ANT do SimAnt original, guiado por data/save_layout.json.

Serve para montar casos de teste de regras contra o original (tests/referencias/casos/*.json): parte de um
save real, muda só o que o caso pede e grava um .ANT que o original carrega pelo menu File > Load.
Formato: docs/spec/formatos/save-ant.md (seção "Estruturas principais").

Layout: no data/save_layout.json, `count` é a LARGURA de cada elemento em bytes (1, 2 ou 4) e `tamanho` é o
NÚMERO de elementos (ex.: HealthB = 2×1, uma palavra; H_ADead = 2×64, 64 palavras; MapA = 1×8192). Valores
em little-endian. Por padrão a leitura é sem sinal; `--sinal` mostra com sinal. A escrita aceita negativos.

Uso (CLI):
    python tools/ant_editar.py blocos ARQ.ANT                       lista os 307 blocos (offset, largura × n)
    python tools/ant_editar.py ler ARQ.ANT NOME [INDICE] [--largura L] [--sinal]
    python tools/ant_editar.py escrever ARQ.ANT SAIDA.ANT NOME VALOR [INDICE] [--largura L]
    python tools/ant_editar.py celula ARQ.ANT MAPA X Y [VALOR SAIDA.ANT]
    python tools/ant_editar.py formigas ARQ.ANT A|B|R               lista as entradas 0..ListIndex-1
    python tools/ant_editar.py patch BASE.ANT CASO.json SAIDA.ANT   aplica as "operacoes" de um caso
    python tools/ant_editar.py patch CASO.json SAIDA.ANT            idem, com a base do campo "base" do caso

Funções importáveis: Save (classe), Save.ler/escrever/celula/set_celula/formigas/adicionar_formiga/
remover_formiga/editar_formiga/zerar_lista, aplicar_patch(save, operacoes).

Operações de patch (lista "operacoes" do JSON; campos de formiga em maiúsculas X, Y, M, T, S):
    {"op": "escrever", "nome": "HealthB", "valor": 50}                       escalar (bloco inteiro)
    {"op": "escrever", "nome": "casteLevels", "indice": 2, "valor": 0}       elemento
    {"op": "escrever", "nome": "AMapPnt", "indice": 1, "largura": 2, "valor": 9}   largura forçada
    {"op": "bloco_hex", "nome": "SwarmXB", "hex": "00ff..."}                 bloco inteiro em hexadecimal
    {"op": "celula", "mapa": "MapB", "x": 34, "y": 10, "valor": 0}           célula (x, y) de um mapa
    {"op": "retangulo", "mapa": "MapB", "x0": 34, "y0": 10, "x1": 34, "y1": 20, "valor": 0}   inclusivo
    {"op": "formiga_adicionar", "lista": "B", "X": 32, "Y": 3, "M": 2, "T": 18, "S": 0, ["indice": 0]}
    {"op": "formiga_editar", "lista": "B", "indice": 2, "T": 7}
    {"op": "formiga_editar", "lista": "B", "onde": {"T": 1}, "T": 7, ["todas": true]}
    {"op": "formiga_remover", "lista": "B", "indice": 1}   (ou "onde": {...}, ["todas": true])
    {"op": "lista_zerar", "lista": "A"}
Valores podem ser inteiros ou textos "0x..". Casos e execução: tools/caso_ant.py e tests/referencias/README.md.
"""
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LAYOUT = REPO / "data" / "save_layout.json"

# Mapas coluna-major: byte de (x, y) em x * altura + y (save-ant.md, "Mapas de terreno" e "Mapas de feromônio").
ALTURA_MAPA = {
    "MapA": 64, "MapB": 64, "MapR": 64, "ExitMapB": 64, "ExitMapR": 64,
    "PherMapA": 32, "PherMapBN": 32, "PherMapBT": 32, "PherMapRN": 32, "PherMapRT": 32,
}
CAMPOS = ("X", "Y", "M", "T", "S")


def _int(v):
    return int(v, 0) if isinstance(v, str) else int(v)


class Save:
    def __init__(self, dados: bytes, layout_path: Path = LAYOUT):
        layout = json.loads(Path(layout_path).read_text(encoding="utf-8"))
        self.blocos = {}
        self.ordem = []
        pos = 0
        for b in layout["blocos"]:
            nome = b["simbolo"]
            largura, n = b["count"], b["tamanho"]
            self.blocos[nome] = (pos, largura, n)
            self.ordem.append(nome)
            pos += largura * n
        if len(dados) != pos:
            raise ValueError(f".ANT com {len(dados)} bytes; o layout pede {pos}")
        self.dados = bytearray(dados)

    @classmethod
    def abrir(cls, caminho):
        return cls(Path(caminho).read_bytes())

    def gravar(self, caminho):
        Path(caminho).parent.mkdir(parents=True, exist_ok=True)
        Path(caminho).write_bytes(bytes(self.dados))

    # --- blocos e escalares -------------------------------------------------------------------------
    def _bloco(self, nome):
        nome = nome.lstrip("_")
        if nome not in self.blocos:
            raise KeyError(f"bloco desconhecido: {nome}")
        return self.blocos[nome]

    def bytes_bloco(self, nome) -> bytes:
        pos, largura, n = self._bloco(nome)
        return bytes(self.dados[pos:pos + largura * n])

    def escrever_bloco(self, nome, dados: bytes):
        pos, largura, n = self._bloco(nome)
        if len(dados) != largura * n:
            raise ValueError(f"{nome}: {largura * n} bytes, recebidos {len(dados)}")
        self.dados[pos:pos + len(dados)] = dados

    def _local(self, nome, indice, largura):
        """(offset, largura) do elemento. Sem índice, o bloco inteiro como um número (até 4 bytes)."""
        pos, larg_layout, n = self._bloco(nome)
        total = larg_layout * n
        if indice is None and largura is None:
            if total > 4:
                raise ValueError(f"{nome} tem {total} bytes: informe o índice")
            return pos, total
        largura = largura or larg_layout
        indice = indice or 0
        if indice < 0 or (indice + 1) * largura > total:
            raise IndexError(f"{nome}[{indice}] (largura {largura}) fora do bloco de {total} bytes")
        return pos + indice * largura, largura

    def ler(self, nome, indice=None, largura=None, sinal=False) -> int:
        o, w = self._local(nome, indice, largura)
        return int.from_bytes(self.dados[o:o + w], "little", signed=sinal)

    def escrever(self, nome, valor, indice=None, largura=None):
        o, w = self._local(nome, indice, largura)
        v = _int(valor)
        if not -(1 << (8 * w - 1)) <= v < (1 << (8 * w)):
            raise ValueError(f"{nome}: valor {v} não cabe em {w} bytes")
        self.dados[o:o + w] = (v & ((1 << (8 * w)) - 1)).to_bytes(w, "little")

    def elementos(self, nome, sinal=False):
        pos, largura, n = self._bloco(nome)
        return [int.from_bytes(self.dados[pos + i * largura:pos + (i + 1) * largura], "little", signed=sinal)
                for i in range(n)]

    # --- mapas --------------------------------------------------------------------------------------
    def _celula(self, mapa, x, y):
        mapa = mapa.lstrip("_")
        if mapa not in ALTURA_MAPA:
            raise KeyError(f"mapa sem dimensão conhecida: {mapa} (conhecidos: {', '.join(ALTURA_MAPA)})")
        pos, _, n = self._bloco(mapa)
        h = ALTURA_MAPA[mapa]
        largura = n // h
        if not (0 <= x < largura and 0 <= y < h):
            raise IndexError(f"{mapa}: ({x}, {y}) fora de {largura} × {h}")
        return pos + x * h + y

    def celula(self, mapa, x, y) -> int:
        return self.dados[self._celula(mapa, x, y)]

    def set_celula(self, mapa, x, y, valor):
        self.dados[self._celula(mapa, x, y)] = _int(valor) & 0xFF

    # --- listas de formigas -------------------------------------------------------------------------
    @staticmethod
    def _area(lista):
        lista = lista.upper()
        if lista not in ("A", "B", "R"):
            raise ValueError(f"lista deve ser A, B ou R: {lista}")
        return lista

    def capacidade(self, lista):
        return self._bloco(f"{self._area(lista)}listX")[2]

    def indice_lista(self, lista):
        return self.ler(f"ListIndex{self._area(lista)}")

    def formiga(self, lista, i):
        a = self._area(lista)
        return {c: self.ler(f"{a}list{c}", i) for c in CAMPOS}

    def formigas(self, lista):
        """Entradas 0..ListIndex-1 (as usadas), cada uma {X, Y, M, T, S}."""
        return [self.formiga(lista, i) for i in range(self.indice_lista(lista))]

    def _gravar_formiga(self, lista, i, f):
        a = self._area(lista)
        for c in CAMPOS:
            self.escrever(f"{a}list{c}", f.get(c, 0), i)

    def editar_formiga(self, lista, i, **campos):
        n = self.indice_lista(lista)
        if not 0 <= i < n:
            raise IndexError(f"lista {lista}: índice {i} fora de 0..{n - 1}")
        f = self.formiga(lista, i)
        for c, v in campos.items():
            if c not in CAMPOS:
                raise KeyError(f"campo de formiga desconhecido: {c}")
            f[c] = _int(v)
        self._gravar_formiga(lista, i, f)

    def adicionar_formiga(self, lista, X, Y, M, T, S=0, indice=None):
        """Acrescenta no fim (como _AddAntTo?List) ou insere em `indice`, deslocando as seguintes.
        Atualiza ListIndex e mantém zerada a vaga ListIndex (o load do original também a lê)."""
        n, cap = self.indice_lista(lista), self.capacidade(lista)
        if n + 1 >= cap:  # a vaga ListIndex precisa continuar existindo
            raise ValueError(f"lista {lista} cheia ({n} de {cap})")
        indice = n if indice is None else indice
        if not 0 <= indice <= n:
            raise IndexError(f"lista {lista}: índice {indice} fora de 0..{n}")
        fs = self.formigas(lista)
        fs.insert(indice, {"X": _int(X), "Y": _int(Y), "M": _int(M), "T": _int(T), "S": _int(S)})
        for i in range(indice, n + 1):
            self._gravar_formiga(lista, i, fs[i])
        self._gravar_formiga(lista, n + 1, {})
        self.escrever(f"ListIndex{self._area(lista)}", n + 1)
        return indice

    def remover_formiga(self, lista, i):
        """Remove deslocando as seguintes (como _RemoveFromAList) e zera a vaga que sobra."""
        fs = self.formigas(lista)
        if not 0 <= i < len(fs):
            raise IndexError(f"lista {lista}: índice {i} fora de 0..{len(fs) - 1}")
        del fs[i]
        for k in range(i, len(fs)):
            self._gravar_formiga(lista, k, fs[k])
        self._gravar_formiga(lista, len(fs), {})
        self.escrever(f"ListIndex{self._area(lista)}", len(fs))

    def zerar_lista(self, lista):
        a = self._area(lista)
        for c in CAMPOS:
            self.escrever_bloco(f"{a}list{c}", bytes(self.capacidade(a)))
        self.escrever(f"ListIndex{a}", 0)

    def procurar(self, lista, onde):
        """Índices das formigas usadas cujos campos batem com `onde` ({campo: valor})."""
        onde = {c: _int(v) for c, v in onde.items()}
        return [i for i, f in enumerate(self.formigas(lista)) if all(f[c] == v for c, v in onde.items())]


def _alvos(save, op):
    if "indice" in op:
        return [op["indice"]]
    achados = save.procurar(op["lista"], op["onde"])
    if not achados:
        raise ValueError(f"nenhuma formiga na lista {op['lista']} com {op['onde']}")
    return achados if op.get("todas") else achados[:1]


def aplicar_patch(save: Save, operacoes):
    for op in operacoes:
        k = op["op"]
        if k == "escrever":
            save.escrever(op["nome"], op["valor"], op.get("indice"), op.get("largura"))
        elif k == "bloco_hex":
            save.escrever_bloco(op["nome"], bytes.fromhex(op["hex"]))
        elif k == "celula":
            save.set_celula(op["mapa"], op["x"], op["y"], op["valor"])
        elif k == "retangulo":
            for x in range(op["x0"], op["x1"] + 1):
                for y in range(op["y0"], op["y1"] + 1):
                    save.set_celula(op["mapa"], x, y, op["valor"])
        elif k == "formiga_adicionar":
            save.adicionar_formiga(op["lista"], *(op[c] for c in ("X", "Y", "M", "T")), op.get("S", 0),
                                   op.get("indice"))
        elif k == "formiga_editar":
            campos = {c: op[c] for c in CAMPOS if c in op}
            for i in _alvos(save, op):
                save.editar_formiga(op["lista"], i, **campos)
        elif k == "formiga_remover":
            for i in sorted(_alvos(save, op), reverse=True):
                save.remover_formiga(op["lista"], i)
        elif k == "lista_zerar":
            save.zerar_lista(op["lista"])
        else:
            raise ValueError(f"operação desconhecida: {k}")
    return save


def carregar_caso(caminho):
    caso = json.loads(Path(caminho).read_text(encoding="utf-8"))
    return caso


def montar_caso(caminho_caso, base=None) -> Save:
    """Lê o caso JSON e devolve o Save com o patch aplicado. A base é relativa à raiz do repositório."""
    caso = carregar_caso(caminho_caso)
    base = Path(base or caso["base"])
    if not base.is_absolute():
        base = REPO / base
    return aplicar_patch(Save.abrir(base), caso.get("operacoes", []))


def _opcao(args, nome, padrao=None, flag=False):
    if nome in args:
        k = args.index(nome)
        if flag:
            del args[k]
            return True
        v = args[k + 1]
        del args[k:k + 2]
        return v
    return padrao


def main(argv):
    args = list(argv)
    largura = _opcao(args, "--largura")
    largura = int(largura) if largura else None
    sinal = _opcao(args, "--sinal", flag=True)
    if not args:
        sys.exit(__doc__)
    cmd = args[0]
    if cmd == "blocos":
        s = Save.abrir(args[1])
        for nome in s.ordem:
            pos, w, n = s.blocos[nome]
            extra = f" = {s.ler(nome, sinal=sinal)}" if w * n <= 4 else ""
            print(f"{nome:<20} pos {pos:6d}  {w}×{n}{extra}")
    elif cmd == "ler":
        s = Save.abrir(args[1])
        nome = args[2]
        if len(args) > 3:
            print(s.ler(nome, int(args[3], 0), largura, sinal))
        else:
            pos, w, n = s._bloco(nome)
            if w * n <= 4 and not largura:
                print(s.ler(nome, sinal=sinal))
            else:
                print(" ".join(str(v) for v in s.elementos(nome, sinal)) if not largura else
                      " ".join(str(s.ler(nome, i, largura, sinal)) for i in range(w * n // largura)))
    elif cmd == "escrever":
        s = Save.abrir(args[1])
        s.escrever(args[3], args[4], int(args[5], 0) if len(args) > 5 else None, largura)
        s.gravar(args[2])
    elif cmd == "celula":
        s = Save.abrir(args[1])
        x, y = int(args[3], 0), int(args[4], 0)
        if len(args) > 5:
            s.set_celula(args[2], x, y, args[5])
            s.gravar(args[6])
        else:
            v = s.celula(args[2], x, y)
            print(f"{v} (0x{v:02x})")
    elif cmd == "formigas":
        s = Save.abrir(args[1])
        lista = args[2]
        print(f"ListIndex{lista.upper()} = {s.indice_lista(lista)} (capacidade {s.capacidade(lista)})")
        for i, f in enumerate(s.formigas(lista)):
            t = f["T"]
            print(f"{i:4d}  X {f['X']:3d}  Y {f['Y']:3d}  M {f['M']:3d}  T {t:3d} (0x{t:02x}, "
                  f"{'verm' if t & 0x80 else 'preta'} classe {(t >> 3) & 15} bits {t & 7})  S {f['S']:3d}")
    elif cmd == "patch":
        if len(args) == 3:
            s = montar_caso(args[1])
            s.gravar(args[2])
        else:
            s = montar_caso(args[2], base=args[1])
            s.gravar(args[3])
        print(f"gravado {args[-1]} ({len(s.dados)} bytes)")
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv[1:])
