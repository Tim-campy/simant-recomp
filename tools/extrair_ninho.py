"""Extrai do SIMANTW.EXE as tabelas usadas pelas regras do ninho e do ciclo de vida e grava data/ninho.json.

Uso:
    python tools/extrair_ninho.py SIMANTW.EXE [--json data/ninho.json]

Sem --json, só imprime as tabelas. Especificação: docs/spec/regras/ninho.md, ciclo-de-vida.md e
formigas.md (tabelas de modo e de luta).

Origem de cada tabela (endereço `seletor:offset`; seletor 1038 = segmento 8 do NE, 1048 = segmento 10,
o DGROUP). O offset no arquivo é calculado pela tabela de segmentos do NE (tools/ne_info.py). São dados
inicializados do executável, sem relocação.

| Chave no JSON       | Endereço  | Símbolo / uso                                                           |
|---------------------|-----------|-------------------------------------------------------------------------|
| dx8                 | 1038:0000 | `_Dx8`, 8 bytes com sinal                                               |
| dy8                 | 1038:0008 | `_Dy8`, 8 bytes com sinal                                               |
| turn_tab            | 1038:0024 | `_TurnTab`, 8 × 8 (`_RandTurn`: `_TurnTab[dir][Sk(8)]`)                  |
| modos_operaria      | 1038:89e6 | `_ModeTabWB`, 6 × 8: modo da operária por modo estratégico e `Sk(8)`     |
| modos_soldado       | 1038:8a16 | `_ModeTabSB`, 6 × 8: idem para o soldado                                |
| modo_por_grupo      | 1038:8a46 | `_CasteModeTabB`, 16: modo dos demais grupos (`_GetNewMode*`)           |
| castas_vermelhas    | 1038:897e | `_CasteTabC`, 7 × 8: casta do ovo vermelho por `_StrategicModeR mod 7`  |
|                     |           | e `Sk(8)`; a linha 6 já é o começo de `_CasteTabB` (8:89ae)             |
| vitoria_vermelha    | 1048:22e6 | sem nome; modo da vencedora vermelha no fim da luta (`_DoNestFightR`)   |
| classe_luta         | 1048:22c6 | sem nome; classe de luta por grupo (`_GetWinner`)                        |
| chance_vitoria      | 1048:22d6 | sem nome; 4 × 4, chance em 10 de a ocupante vencer (`_GetWinner`)       |
| borda_buraco        | 1048:230c | sem nome; borda do buraco na superfície por direção (`_MakeNewHoleB/R`) |

As linhas de `_ModeTabWB/SB` são indexadas por `_StrategicModeB/R` (0..5); o `_GetNewMode*` do original
não confere o índice. O segmento das tabelas sem nome foi deduzido do conteúdo (formigas.md §6).
"""
import argparse
import json
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from ne_info import parse_ne  # noqa: E402
from sons_regras import SONS  # noqa: E402

SEG8 = 8
DGROUP = 10


def ler(data, segs, seg, off, n):
    s = segs[seg - 1]
    if off + n > s["file_length"]:
        raise ValueError(f"{seg}:{off:#x}+{n} passa do fim do segmento no arquivo")
    o = s["file_offset"] + off
    return data[o:o + n]


def com_sinal(b):
    return [x - 256 if x > 127 else x for x in b]


def linhas(data, segs, seg, off, n_linhas, largura):
    return [list(ler(data, segs, seg, off + largura * k, largura)) for k in range(n_linhas)]


def extrair(data):
    segs = parse_ne(data)["segments"]
    t = {
        "dx8": com_sinal(ler(data, segs, SEG8, 0x0000, 8)),
        "dy8": com_sinal(ler(data, segs, SEG8, 0x0008, 8)),
        "turn_tab": linhas(data, segs, SEG8, 0x0024, 8, 8),
        "modos_operaria": linhas(data, segs, SEG8, 0x89E6, 6, 8),
        "modos_soldado": linhas(data, segs, SEG8, 0x8A16, 6, 8),
        "modo_por_grupo": list(ler(data, segs, SEG8, 0x8A46, 16)),
        "castas_vermelhas": linhas(data, segs, SEG8, 0x897E, 7, 8),
        "vitoria_vermelha": list(ler(data, segs, DGROUP, 0x22E6, 16)),
        "classe_luta": list(ler(data, segs, DGROUP, 0x22C6, 16)),
        "chance_vitoria": linhas(data, segs, DGROUP, 0x22D6, 4, 4),
        "borda_buraco": list(ler(data, segs, DGROUP, 0x230C, 8)),
    }
    # Conferências: vizinhança de 8, modos 0..19, classes 0..3, e a matriz de luta "soma 10 com a
    # transposta" (P[a][b] + P[b][a] = 10), que é o que identifica a tabela sem nome.
    pares = set(zip(t["dx8"], t["dy8"]))
    if len(pares) != 8 or (0, 0) in pares:
        raise ValueError(f"_Dx8/_Dy8 inesperadas: {t['dx8']} {t['dy8']}")
    for nome in ("modos_operaria", "modos_soldado"):
        if any(not 0 <= m <= 19 for linha in t[nome] for m in linha):
            raise ValueError(f"{nome} com modo fora de 0..19")
    if any(not 0 <= m <= 19 for m in t["modo_por_grupo"] + t["vitoria_vermelha"]):
        raise ValueError("modo_por_grupo/vitoria_vermelha com modo fora de 0..19")
    if any(c > 3 for c in t["classe_luta"]):
        raise ValueError("classe_luta fora de 0..3")
    p = t["chance_vitoria"]
    if any(p[a][b] + p[b][a] != 10 for a in range(4) for b in range(4)):
        raise ValueError(f"chance_vitoria não soma 10 com a transposta: {p}")
    if any(c not in (2, 4, 6, 8) for linha in t["castas_vermelhas"][:6] for c in linha):
        raise ValueError("castas_vermelhas (linhas 0..5) com grupo fora de 2, 4, 6, 8")
    return t


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("exe")
    ap.add_argument("--json")
    a = ap.parse_args()
    data = open(a.exe, "rb").read()
    t = extrair(data)
    if not a.json:
        for k, v in t.items():
            print(k, v)
        return
    saida = {
        "_comentario": (
            "Tabelas das regras do ninho e do ciclo de vida, extraídas do SIMANTW.EXE por "
            "tools/extrair_ninho.py (endereços no topo do script). Ver docs/spec/regras/ninho.md, "
            "ciclo-de-vida.md e formigas.md. Valores em decimal. Linhas de modos_*: modo estratégico "
            "0..5; colunas: Sk(8). chance_vitoria[ocupante][atacante] = chance em 10 de a ocupante vencer."
        ),
        **t,
        "sons": SONS["ninho"],
    }
    # Uma lista por linha, para o arquivo ficar legível e fácil de editar num mod.
    texto = ["{"]
    itens = list(saida.items())
    for i, (k, v) in enumerate(itens):
        fim = "," if i < len(itens) - 1 else ""
        if isinstance(v, list) and v and isinstance(v[0], list):
            corpo = ",\n".join("    " + json.dumps(x) for x in v)
            texto.append(f'  "{k}": [\n{corpo}\n  ]{fim}')
        else:
            texto.append(f'  "{k}": {json.dumps(v, ensure_ascii=False)}{fim}')
    texto.append("}")
    with open(a.json, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(texto) + "\n")
    print(f"gravado {a.json}")


if __name__ == "__main__":
    main()
