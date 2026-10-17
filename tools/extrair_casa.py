"""Extrai do SIMANTW.EXE as formas usadas pelos patches da casa e grava data/casa.json com a receita
de cada patch (`_MakeHousePatch`, docs/spec/regras/geracao-mundo.md §5).

Uso:
    python tools/extrair_casa.py SIMANTW.EXE [--json data/casa.json]

Sem --json, só imprime as formas.

Formas (dados inicializados do DGROUP, seletor 1048 = segmento 10 do NE; o offset no arquivo vem da
tabela de segmentos, tools/ne_info.py). Célula (x + i, y + j) recebe celulas[j * largura + i]:

| Chave      | Endereço  | Tamanho (largura × altura) | Usada por                                  |
|------------|-----------|----------------------------|--------------------------------------------|
| tomada_v   | 1048:2314 | 5 × 4                      | `_MakeOutletV` (1020:3c00), `_MakePlugV`   |
| tomada_h   | 1048:2328 | 4 × 5                      | `_MakeOutletH` (1020:3d44), `_MakePlugH`   |
| botao      | 1048:233c | 5 × 5                      | `_MakeKnob` (1020:3e88), `_MakeSink`       |
| moeda      | 1048:2356 | 3 × 3                      | `_MakePenny` (1020:3eca), patches 5, 6, 15 |
| clipe      | 1048:2360 | 3 × 3, zeros transparentes | `_MakeClip` (1020:3f0c), patches 6, 7, 8   |

As receitas (rotinas e patches) não são tabelas do executável: são as operações que o código de
`_MakeHousePatch` e das rotinas que ele chama faz, com as coordenadas já convertidas de endereço
(`_MapA` = DGROUP:0x28e8, índice x * 64 + y) para (x, y). Cada uma tem o endereço da rotina ao lado.
"""
import argparse
import json
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from ne_info import parse_ne  # noqa: E402

DGROUP = 10
FORMAS = {
    "tomada_v": (0x2314, 5, 4, False),
    "tomada_h": (0x2328, 4, 5, False),
    "botao": (0x233C, 5, 5, False),
    "moeda": (0x2356, 3, 3, False),
    "clipe": (0x2360, 3, 3, True),
}


def ler(data, segs, seg, off, n):
    s = segs[seg - 1]
    if off + n > s["file_length"]:
        raise ValueError(f"{seg}:{off:#x}+{n} passa do fim do segmento no arquivo")
    o = s["file_offset"] + off
    return data[o:o + n]


# --- operações (docs/spec/regras/geracao-mundo.md §5.1) ---------------------------------------

def preencher(x1, x2, y1, y2, v):
    return {"op": "preencher", "x": [x1, x2], "y": [y1, y2], "v": v}


def celula(x, y, v):
    return {"op": "celula", "x": x, "y": y, "v": v}


def sortear(x1, x2, y1, y2, chance, base, variantes):
    return {"op": "sortear", "x": [x1, x2], "y": [y1, y2], "chance": chance, "base": base, "variantes": variantes}


def moldura1(x1, x2, y1, y2):
    """`_TileFrame1` (1020:3944)."""
    return {"op": "moldura", "x": [x1, x2], "y": [y1, y2], "y1": 0x54, "y2": 0x51, "x1": 0x5A, "x2": 0x5B,
            "cantos": [0x53, 0x55, 0x50, 0x52]}


def moldura2(x1, x2, y1, y2):
    """`_TileFrame2` (1020:3aa2)."""
    return {"op": "moldura", "x": [x1, x2], "y": [y1, y2], "y1": 0x51, "y2": 0x54, "x1": 0x5B, "x2": 0x5A,
            "cantos": [0x56, 0x58, 0x57, 0x59]}


def forma(nome, x, y):
    return {"op": "forma", "nome": nome, "x": x, "y": y}


def queda(v):
    """`_DROPdir` (9:9c66)."""
    return {"op": "queda", "valor": v}


def rotina(nome):
    return {"op": "rotina", "nome": nome}


def tomada_v(x, y):
    """`_MakeOutletV(x, y)` (1020:3c00)."""
    return [preencher(x, x + 8, y, y + 12, 0x63), moldura1(x, x + 8, y, y + 12),
            forma("tomada_v", x + 2, y + 2), forma("tomada_v", x + 2, y + 7), celula(x + 4, y + 6, 0x65)]


def tomada_h(x, y):
    """`_MakeOutletH(x, y)` (1020:3d44)."""
    return [preencher(x, x + 12, y, y + 8, 0x63), moldura1(x, x + 12, y, y + 8),
            forma("tomada_h", x + 2, y + 2), forma("tomada_h", x + 7, y + 2), celula(x + 6, y + 4, 0x65)]


def pernas(x1, x2, y1, y2, v):
    """`_FillMapLegs(x1, x2, y1, y2, v)` (1020:3faa)."""
    return [preencher(x1, x2, y1, y2, v), sortear(x1, x2, y1, y2, 20, 0x38, 5),
            preencher(x1, x1 + 3, y1, y1 + 3, 0xC0), preencher(x2 - 3, x2, y1, y1 + 3, 0xC0),
            preencher(x1, x1 + 3, y2 - 3, y2, 0xC0), preencher(x2 - 3, x2, y2 - 3, y2, 0xC0)]


ROTINAS = {
    # _FloorTiles (1020:3406): ladrilhos 16 × 16 com juntas; _DROPdir = 0.
    "ladrilhos": [
        {"op": "ladrilhos", "lado": 16, "par": 1, "impar": 0, "junta_y": [0x61, 0x60], "junta_x": [0x5F, 0x5E],
         "cruzamento": [0x5C, 0x5D]},
        queda(0),
    ],
    # _CarpetFloorL (1020:34c2).
    "carpete_esq": [
        preencher(0, 20, 0, 63, 0x64), preencher(21, 21, 0, 63, 0x7A), preencher(22, 22, 0, 63, 0x7B),
        preencher(23, 127, 0, 63, 0x03), sortear(23, 127, 0, 63, 200, 0x3E, 2),
        *tomada_h(2, 25), queda(1),
    ],
    # _CarpetFloorR (1020:35e2). A coluna 106 recebe 0x7C e logo depois 0x64 (conferido na desmontagem).
    "carpete_dir": [
        preencher(0, 105, 0, 63, 0x03), sortear(0, 105, 0, 63, 200, 0x3E, 2), preencher(106, 106, 0, 63, 0x7C),
        preencher(104, 105, 0, 63, 0x7B), preencher(106, 127, 0, 63, 0x64),
        *tomada_h(0x71, 0x23), queda(3),
    ],
    # _MakeKitchenWall (1020:3698).
    "parede_cozinha": [
        preencher(0, 127, 0, 23, 0x62), preencher(0, 127, 24, 63, 0),
        preencher(0, 127, 0, 0, 0x68), preencher(0, 127, 8, 8, 0x68), preencher(0, 127, 16, 16, 0x68),
        {"op": "trocar", "x": [0, 127], "y": [0, 23], "passo_x": 8, "se": 0x62, "entao": 0x66, "senao": 0x67},
        {"op": "trocar", "x": [0, 127], "y": [23, 23], "passo_x": 1, "se": 0x62, "entao": 0x68, "senao": 0x69},
        *tomada_v(0x24, 2), *tomada_v(0x54, 2), queda(2),
    ],
    # _MakeSink (1020:375c).
    "pia": [
        preencher(37, 91, 26, 56, 0x01), preencher(41, 68, 33, 53, 0xC2), preencher(42, 68, 35, 53, 0xC1),
        moldura2(0x28, 0x44, 0x20, 0x35),
        preencher(73, 88, 33, 53, 0xC2), preencher(74, 88, 35, 53, 0xC1), moldura2(0x48, 0x58, 0x20, 0x35),
        moldura1(0x25, 0x5B, 0x1A, 0x38),
        forma("botao", 44, 27), forma("botao", 60, 27), celula(62, 29, 0x4D), forma("botao", 52, 27),
        forma("botao", 52, 36),
        preencher(52, 56, 29, 38, 0x4E), preencher(52, 52, 29, 38, 0x43), preencher(56, 56, 29, 38, 0x44),
        celula(52, 40, 0x45), celula(56, 40, 0x48),
        preencher(54, 56, 41, 42, 0xC2), preencher(57, 58, 35, 42, 0xC2),
    ],
}


def receita(n):
    """Operações de `_MakeHousePatch(n)` (1020:3120) depois de `_InitAntLions(0)` e `_InitPillar`."""
    if n == 0:
        return [rotina("parede_cozinha"), rotina("pia")]
    if n in (2, 3, 17, 18, 19, 33, 34, 35):
        return [rotina("ladrilhos")]
    if n in (4, 10, 11, 13, 14):
        return [rotina("carpete_esq")]
    if n == 5:
        return [rotina("carpete_esq"), preencher(24, 70, 20, 63, 2), forma("moeda", 40, 45)]
    if n == 6:
        return [rotina("carpete_esq"), preencher(24, 70, 0, 63, 2), forma("moeda", 32, 20), forma("clipe", 38, 55)]
    if n == 7:
        return [rotina("carpete_esq"), preencher(24, 70, 0, 20, 2), forma("clipe", 42, 10)]
    if n == 8:
        return [rotina("carpete_esq"), *pernas(0x18, 0x36, 0x0F, 0x2D, 2), forma("clipe", 28, 34)]
    if n == 9:
        return [preencher(0, 127, 0, 63, 3), queda(2)]
    if n == 12:
        return [rotina("carpete_esq"), *pernas(0x2D, 0x5F, 8, 0x2E, 2)]
    if n == 15:
        return [rotina("carpete_esq"), *pernas(0x32, 100, 10, 0x2D, 2), forma("moeda", 75, 34)]
    if 20 <= n <= 31:
        return [rotina("carpete_dir")]
    if n in (36, 37):
        return [preencher(0, 127, 0, 63, 0), queda(2)]
    return [rotina("parede_cozinha")]  # 1, 16, 32 (caso default)


def extrair(data):
    segs = parse_ne(data)["segments"]
    formas = {}
    for nome, (off, w, h, transp) in FORMAS.items():
        b = list(ler(data, segs, DGROUP, off, w * h))
        formas[nome] = {"largura": w, "altura": h, "transparente": transp, "celulas": b}
    # Conferências: os bytes entre as formas são zero e as tomadas têm moldura (cantos 0x6B..0x70).
    if ler(data, segs, DGROUP, 0x2355, 1) != b"\0" or ler(data, segs, DGROUP, 0x235F, 1) != b"\0":
        raise ValueError("separadores das formas da casa inesperados")
    for nome in ("tomada_v", "tomada_h"):
        c = formas[nome]["celulas"]
        if c[0] != 0x6B or c[-1] != 0x70:
            raise ValueError(f"{nome}: cantos inesperados {c}")
    return formas


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("exe")
    ap.add_argument("--json")
    a = ap.parse_args()
    formas = extrair(open(a.exe, "rb").read())
    if not a.json:
        for k, v in formas.items():
            print(k, v)
        return
    saida = {
        "_comentario": (
            "Patches da casa (_MakeHousePatch, n = px * 16 + py <= 37). Formas extraídas do SIMANTW.EXE e "
            "receitas escritas por tools/extrair_casa.py. Célula (x+i, y+j) de uma forma recebe "
            "celulas[j*largura+i]. Operações e ordem dos sorteios: docs/spec/regras/geracao-mundo.md §5. "
            "Valores em decimal."
        ),
        "formas": formas,
        "rotinas": ROTINAS,
        "patches": [receita(n) for n in range(38)],
    }
    # Uma operação por linha.
    linhas = ["{", f'  "_comentario": {json.dumps(saida["_comentario"], ensure_ascii=False)},', '  "formas": {']
    itens = list(formas.items())
    for i, (k, v) in enumerate(itens):
        linhas.append(f'    "{k}": {json.dumps(v)}' + ("," if i < len(itens) - 1 else ""))
    linhas.append("  },")
    linhas.append('  "rotinas": {')
    itens = list(ROTINAS.items())
    for i, (k, ops) in enumerate(itens):
        corpo = ",\n".join("      " + json.dumps(o) for o in ops)
        linhas.append(f'    "{k}": [\n{corpo}\n    ]' + ("," if i < len(itens) - 1 else ""))
    linhas.append("  },")
    linhas.append('  "patches": [')
    for n in range(38):
        ops = ", ".join(json.dumps(o) for o in receita(n))
        linhas.append(f"    [{ops}]" + ("," if n < 37 else ""))
    linhas.append("  ]")
    linhas.append("}")
    with open(a.json, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(linhas) + "\n")
    print(f"gravado {a.json}")


if __name__ == "__main__":
    main()
