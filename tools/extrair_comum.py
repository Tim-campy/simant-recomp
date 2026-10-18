"""Extrai do SIMANTW.EXE as tabelas que o original tem uma vez só e várias partes das regras usam, e
grava data/comum.json.

Uso:
    python tools/extrair_comum.py SIMANTW.EXE [--json data/comum.json]

Sem --json, só imprime as tabelas. Antes da API de mods 0.20 cada data/<parte>.json trazia a sua cópia
destas tabelas; agora elas ficam só em comum.json e cada parte lê de lá (engine/src/rules/comum.rs,
`completar_com_comuns`; docs/api-mods.md). Os outros extratores (extrair_amarela.py, extrair_ninho.py...)
importam USOS daqui para não gravar as cópias.

Origem de cada tabela (endereço `segmento lógico:offset` do symbols.tsv; 8 = seletor 1038,
SIMANT_DATA_GROUP; 10 = seletor 1048, DGROUP). O offset no arquivo vem da tabela de segmentos do NE
(tools/ne_info.py). São dados inicializados do executável, sem relocação.

| Chave              | Endereço | Símbolo / uso                                                                  |
|--------------------|----------|--------------------------------------------------------------------------------|
| dx8, dy8           | 8:0000/0008 | `_Dx8`/`_Dy8`: deslocamento por direção 0–7 (0 = cima, 2 = direita)        |
| dx9, dy9           | 8:0010/001a | `_Dx9`/`_Dy9`: 9 entradas, 0 = parado, k = direção k − 1 (`_GetDir`)       |
| turn_tab           | 8:0024   | `_TurnTab`, 8 × 8: linha = direção atual, coluna = alvo; valor = nova direção  |
| grupo_sem_carga    | 8:0094   | `_UnCarryCaste`, 16 bytes: grupo depois de largar a carga                      |
| modos_operaria     | 8:89e6   | `_ModeTabWB`, 6 × 8: modo da operária por modo estratégico e `Sk(8)`           |
| modos_soldado      | 8:8a16   | `_ModeTabSB`, 6 × 8: idem, soldado                                             |
| modo_por_grupo     | 8:8a46   | `_CasteModeTabB`, 16 bytes: modo dos grupos que não sorteiam (`_GetNewMode*`)  |
| sow_tab            | 8:8a90   | `_SowTab`: tile do bicho-de-conta por direção                                  |
| classe_luta        | 10:22c6  | sem nome; classe de luta por grupo (0 operária/ovo, 1 soldado, 2 reprodutor, 3 rainha) |
| matriz_luta        | 10:22d6  | sem nome; 4 × 4, chance em 10 de a formiga da célula ("a", a ocupante) vencer (`_GetWinner`) |
| borda_buraco       | 10:230c  | sem nome; tile da borda do buraco por direção (`_MakeNewHole*`, `_CreateNewHole`) |
| borda_formiga_leao | 10:25e8  | sem nome; somado a 0x30 por direção (`_InitAntLions`, `_AddRandAntLion`, `_DoAntLions`) |
"""
import argparse
import json
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from ne_info import parse_ne  # noqa: E402

SEG8 = 8
DGROUP = 10

# chave -> (segmento, offset, linhas, colunas, com sinal, símbolo/uso). linhas = 0: vetor simples.
TABELAS = {
    "dx8": (SEG8, 0x0000, 0, 8, True, "_Dx8"),
    "dy8": (SEG8, 0x0008, 0, 8, True, "_Dy8"),
    "dx9": (SEG8, 0x0010, 0, 9, True, "_Dx9 (0 = parado, k = direção k - 1)"),
    "dy9": (SEG8, 0x001A, 0, 9, True, "_Dy9"),
    "turn_tab": (SEG8, 0x0024, 8, 8, False, "_TurnTab[atual][alvo]"),
    "grupo_sem_carga": (SEG8, 0x0094, 0, 16, False, "_UnCarryCaste"),
    "modos_operaria": (SEG8, 0x89E6, 6, 8, False, "_ModeTabWB[modo estratégico][Sk(8)]"),
    "modos_soldado": (SEG8, 0x8A16, 6, 8, False, "_ModeTabSB[modo estratégico][Sk(8)]"),
    "modo_por_grupo": (SEG8, 0x8A46, 0, 16, False, "_CasteModeTabB[grupo]"),
    "sow_tab": (SEG8, 0x8A90, 0, 8, False, "_SowTab"),
    "classe_luta": (DGROUP, 0x22C6, 0, 16, False, "sem nome; classe de luta por grupo (_GetWinner)"),
    "matriz_luta": (DGROUP, 0x22D6, 4, 4, False, "sem nome; chance em 10 de a ocupante vencer (_GetWinner)"),
    "borda_buraco": (DGROUP, 0x230C, 0, 8, False, "sem nome; borda do buraco por direção (_MakeNewHole*)"),
    "borda_formiga_leao": (DGROUP, 0x25E8, 0, 8, False, "sem nome; borda da armadilha, + 0x30 (_InitAntLions)"),
}

# Parte -> {chave que a parte usava no seu arquivo (e ainda aceita de mods antigos): chave em comum.json}.
# Espelha `USOS_COMUNS` de cada engine/src/rules/<parte>.rs.
USOS = {
    "amarela": {"dx8": "dx8", "dy8": "dy8", "dx9": "dx9", "dy9": "dy9", "descarregar_grupo": "grupo_sem_carga",
                "classe_luta": "classe_luta", "matriz_luta": "matriz_luta", "borda_buraco": "borda_buraco"},
    "feromonios": {"dx8": "dx8", "dy8": "dy8", "turn_tab": "turn_tab"},
    "geracao": {"dx8": "dx8", "dy8": "dy8", "borda_buraco": "borda_buraco",
                "borda_formiga_leao": "borda_formiga_leao", "sow_tab": "sow_tab"},
    "inimigos": {"dx8": "dx8", "dy8": "dy8", "dx9": "dx9", "dy9": "dy9", "turn_tab": "turn_tab",
                 "sow_tab": "sow_tab", "borda_formiga_leao": "borda_formiga_leao"},
    "ninho": {"dx8": "dx8", "dy8": "dy8", "turn_tab": "turn_tab", "modos_operaria": "modos_operaria",
              "modos_soldado": "modos_soldado", "modo_por_grupo": "modo_por_grupo", "classe_luta": "classe_luta",
              "chance_vitoria": "matriz_luta", "borda_buraco": "borda_buraco"},
    "quintal": {"dx8": "dx8", "dy8": "dy8"},
    "superficie": {"grupo_sem_carga": "grupo_sem_carga", "modos_operaria": "modos_operaria",
                   "modos_soldado": "modos_soldado", "modo_por_grupo": "modo_por_grupo",
                   "classe_luta": "classe_luta", "matriz_luta": "matriz_luta"},
}


def sem_comuns(parte, t):
    """As tabelas da parte sem as que agora ficam em comum.json."""
    return {k: v for k, v in t.items() if k not in USOS[parte]}


def nota_comuns(parte):
    """Frase do `_comentario` de data/<parte>.json que aponta para comum.json."""
    chaves = ", ".join(f"{k} (comum.json: {c})" if k != c else k for k, c in USOS[parte].items())
    return f" As tabelas que o original tem uma vez só ({chaves}) ficam em data/comum.json."


def ler(data, segs, seg, off, n):
    s = segs[seg - 1]
    if off + n > s["file_length"]:
        raise ValueError(f"{seg}:{off:#x}+{n} passa do fim do segmento no arquivo")
    o = s["file_offset"] + off
    return data[o:o + n]


def com_sinal(b):
    return [x - 256 if x > 127 else x for x in b]


def extrair(data):
    segs = parse_ne(data)["segments"]
    t = {}
    for chave, (seg, off, linhas, colunas, sinal, _) in TABELAS.items():
        conv = com_sinal if sinal else list
        if linhas:
            t[chave] = [conv(ler(data, segs, seg, off + colunas * k, colunas)) for k in range(linhas)]
        else:
            t[chave] = conv(ler(data, segs, seg, off, colunas))
    # Conferências: vizinhança de 8, _Dx9 = parado + _Dx8, giros em 0..7, modos em 0..19, classes em 0..3,
    # matriz de luta "soma 10 com a transposta", bicho-de-conta em 0x70..0x73.
    if set(zip(t["dx8"], t["dy8"])) != {(dx, dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1)} - {(0, 0)}:
        raise ValueError(f"_Dx8/_Dy8 inesperadas: {t['dx8']} {t['dy8']}")
    if t["dx9"] != [0] + t["dx8"] or t["dy9"] != [0] + t["dy8"]:
        raise ValueError(f"_Dx9/_Dy9 inesperadas: {t['dx9']} {t['dy9']}")
    if any(not 0 <= d <= 7 for linha in t["turn_tab"] for d in linha):
        raise ValueError("_TurnTab com direção fora de 0..7")
    for k in ("modos_operaria", "modos_soldado"):
        if any(not 0 <= m <= 19 for linha in t[k] for m in linha):
            raise ValueError(f"{k} com modo fora de 0..19")
    if any(not 0 <= m <= 19 for m in t["modo_por_grupo"]):
        raise ValueError("modo_por_grupo com modo fora de 0..19")
    if any(g > 15 for g in t["grupo_sem_carga"]):
        raise ValueError("grupo_sem_carga fora de 0..15")
    if any(c > 3 for c in t["classe_luta"]):
        raise ValueError("classe_luta fora de 0..3")
    p = t["matriz_luta"]
    if any(p[i][j] + p[j][i] != 10 for i in range(4) for j in range(4) if i != j):
        raise ValueError(f"matriz_luta não soma 10 com a transposta: {p}")
    if any(not 0x70 <= v <= 0x73 for v in t["sow_tab"]):
        raise ValueError(f"_SowTab fora de 0x70..0x73: {t['sow_tab']}")
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
    nomes_seg = {SEG8: "seletor 1038, SIMANT_DATA_GROUP", DGROUP: "seletor 1048, DGROUP"}
    enderecos = {
        k: f"{seg}:{off:04x} ({nomes_seg[seg]}): {simb}" for k, (seg, off, _, _, _, simb) in TABELAS.items()
    }
    usado_por = {k: sorted(p for p, u in USOS.items() if k in u.values()) for k in TABELAS}
    saida = {
        "_comentario": (
            "Tabelas que o original tem uma vez só e várias partes das regras usam (desde a API de mods 0.20; "
            "antes cada data/<parte>.json tinha a sua cópia). Extraídas do SIMANTW.EXE por tools/extrair_comum.py. "
            "Um mod que traga este arquivo muda a tabela para todas as partes listadas em '_usado_por'. "
            "Ver docs/api-mods.md. Valores em decimal."
        ),
        "_enderecos": enderecos,
        "_usado_por": usado_por,
        **t,
    }
    linhas = ["{"]
    itens = list(saida.items())
    for i, (k, v) in enumerate(itens):
        fim = "," if i < len(itens) - 1 else ""
        if isinstance(v, list) and v and isinstance(v[0], list):
            corpo = ",\n".join("    " + json.dumps(x) for x in v)
            linhas.append(f'  "{k}": [\n{corpo}\n  ]{fim}')
        elif isinstance(v, dict):
            corpo = ",\n".join(f"    {json.dumps(kk)}: {json.dumps(vv, ensure_ascii=False)}" for kk, vv in v.items())
            linhas.append(f'  "{k}": {{\n{corpo}\n  }}{fim}')
        else:
            linhas.append(f'  "{k}": {json.dumps(v, ensure_ascii=False)}{fim}')
    linhas.append("}")
    with open(a.json, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(linhas) + "\n")
    print(f"gravado {a.json}")


if __name__ == "__main__":
    main()
