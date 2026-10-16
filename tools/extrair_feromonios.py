"""Extrai do SIMANTW.EXE as tabelas usadas pelas regras de feromônio e grava data/feromonios.json.

Uso:
    python tools/extrair_feromonios.py SIMANTW.EXE [--json data/feromonios.json]

Sem --json, só imprime as tabelas. Especificação: docs/spec/regras/feromonios.md.

Origem de cada tabela (endereço `seletor:offset`; seletor 1038 = segmento 8 do NE, o
SIMANT_DATA_GROUP). O offset no arquivo vem da tabela de segmentos do NE (tools/ne_info.py).
São dados inicializados do executável, sem relocação.

| Chave no JSON | Endereço  | Símbolo / uso                                                         |
|---------------|-----------|-----------------------------------------------------------------------|
| dx8           | 1038:0000 | `_Dx8`, 8 bytes com sinal (deslocamento x por direção 0–7)             |
| dy8           | 1038:0008 | `_Dy8`, 8 bytes com sinal                                             |
| turn_tab      | 1038:0024 | `_TurnTab`, 8 × 8 bytes: linha = direção atual, coluna = direção alvo; |
|               |           | valor = nova direção (gira no máximo 45° rumo ao alvo)                 |

As constantes da seção "regras" não são tabelas do executável: são valores imediatos lidos no
pseudocódigo das funções citadas ao lado de cada uma (docs/spec/regras/feromonios.md). O script só
as escreve no JSON, para que fiquem fora do código do port e possam ser trocadas por mods.
"""
import argparse
import json
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from ne_info import parse_ne  # noqa: E402

SEG8 = 8

# Valor, função de origem e significado. Vistos no pseudocódigo (Code6.c), não no segmento de dados.
REGRAS = {
    "limiar_trilha": (8, "_ColonySmellBT/RT 1028:9306: abaixo disto a trilha zera; senão perde a metade"),
    "limiar_alarme": (9, "_SmoothAlarm 1028:9380: resultado da suavização abaixo disto vira 0"),
    "teto_alarme_amarela": (200, "_AlarmHere 1028:943c: c = min(c + v, teto)"),
    "valor_buraco": (255, "_FillHolesBN/RN 1028:91de/9244: valor posto na célula do buraco"),
    "tile_buraco_fechado": (0x51, "_FillHolesBN/RN: se o tile do buraco no _MapA for este, a célula recebe 0"),
}


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
    t = {
        "dx8": com_sinal(ler(data, segs, SEG8, 0x0000, 8)),
        "dy8": com_sinal(ler(data, segs, SEG8, 0x0008, 8)),
        "turn_tab": [list(ler(data, segs, SEG8, 0x0024 + 8 * d, 8)) for d in range(8)],
    }
    pares = set(zip(t["dx8"], t["dy8"]))
    if len(pares) != 8 or (0, 0) in pares:
        raise ValueError(f"_Dx8/_Dy8 inesperadas: {t['dx8']} {t['dy8']}")
    # Conferências do _TurnTab: valores 0–7, diagonal identidade e giro de no máximo uma direção.
    for atual, linha in enumerate(t["turn_tab"]):
        for alvo, novo in enumerate(linha):
            if not 0 <= novo <= 7:
                raise ValueError(f"_TurnTab[{atual}][{alvo}] = {novo} fora de 0–7")
            if alvo == atual and novo != atual:
                raise ValueError(f"_TurnTab[{atual}][{atual}] deveria ser {atual}")
            if min((novo - atual) % 8, (atual - novo) % 8) > 1:
                raise ValueError(f"_TurnTab[{atual}][{alvo}] = {novo} gira mais de 45°")
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
        for k, (v, origem) in REGRAS.items():
            print(k, v, "-", origem)
        return
    saida = {
        "_comentario": (
            "Tabelas e constantes das regras de feromônio. dx8/dy8/turn_tab foram extraídas do "
            "SIMANTW.EXE por tools/extrair_feromonios.py (endereços no topo do script); turn_tab[atual][alvo] "
            "= nova direção. As constantes de 'regras' vêm do pseudocódigo (origem em '_origem'). "
            "Ver docs/spec/regras/feromonios.md. Valores em decimal."
        ),
        **t,
        "regras": {k: v for k, (v, _) in REGRAS.items()},
        "_origem": {k: o for k, (_, o) in REGRAS.items()},
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
