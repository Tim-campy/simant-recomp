"""Extrai do SIMANTW.EXE as tabelas usadas pela geração do mundo e grava data/geracao.json.

Uso:
    python tools/extrair_geracao.py SIMANTW.EXE [--json data/geracao.json]

Sem --json, só imprime as tabelas. Especificação: docs/spec/regras/geracao-mundo.md.

Origem de cada tabela (endereço `seletor:offset`; seletor 1038 = segmento 8 do NE,
1048 = segmento 10, o DGROUP). O offset no arquivo é calculado pela tabela de segmentos do NE
(tools/ne_info.py), não fixado aqui. São dados inicializados do executável, sem relocação.

| Chave no JSON          | Endereço    | Símbolo / uso                                              |
|------------------------|-------------|------------------------------------------------------------|
| dx8                    | 1038:0000   | `_Dx8`, 8 bytes com sinal (deslocamento x por direção 0–7) (só conferida; vai para comum.json) |
| dy8                    | 1038:0008   | `_Dy8`, 8 bytes com sinal (só conferida; vai para comum.json) |
| borda_buraco           | 1048:230c   | sem nome; borda do buraco por direção (`_MakeNewHoleB/R`) (só conferida; vai para comum.json) |
| borda_formiga_leao     | 1048:25e8   | sem nome; somado a 0x30 por direção (`_InitAntLions`) (só conferida; vai para comum.json) |
| sow_tab                | 1038:8a90   | `_SowTab`, tile do bicho-de-conta por direção (`_InitPillar`) (só conferida; vai para comum.json) |
| pedras5                | 1038:870a   | 6 formas 5 × 5 (25 bytes cada, `_AddRock5`)                |
| pedras3                | 1038:87a0   | 6 formas 3 × 3 (9 bytes cada, `_AddRock3`)                 |

A tabela de seno (`_FracSineTab`) não é extraída aqui: vem do objeto tipo 9 nº 1000 do banco
SHARED e é lida em tempo de execução (engine/src/rules/geracao.rs).

As tabelas marcadas "vai para comum.json" são lidas só para as conferências: desde a API de mods 0.20 elas ficam em
data/comum.json (tools/extrair_comum.py), e este script não as grava.
"""
import argparse
import json
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from ne_info import parse_ne  # noqa: E402
from extrair_comum import sem_comuns, nota_comuns  # noqa: E402

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


def extrair(data):
    segs = parse_ne(data)["segments"]
    t = {
        "dx8": com_sinal(ler(data, segs, SEG8, 0x0000, 8)),
        "dy8": com_sinal(ler(data, segs, SEG8, 0x0008, 8)),
        "borda_buraco": list(ler(data, segs, DGROUP, 0x230C, 8)),
        "borda_formiga_leao": list(ler(data, segs, DGROUP, 0x25E8, 8)),
        "sow_tab": list(ler(data, segs, SEG8, 0x8A90, 8)),
        "pedras5": [list(ler(data, segs, SEG8, 0x870A + 25 * k, 25)) for k in range(6)],
        "pedras3": [list(ler(data, segs, SEG8, 0x87A0 + 9 * k, 9)) for k in range(6)],
    }
    # Conferências mínimas: as direções formam a vizinhança de 8 e as formas só têm tiles de pedra.
    pares = set(zip(t["dx8"], t["dy8"]))
    if len(pares) != 8 or (0, 0) in pares:
        raise ValueError(f"_Dx8/_Dy8 inesperadas: {t['dx8']} {t['dy8']}")
    for forma in t["pedras5"] + t["pedras3"]:
        if any(v and not 0x54 <= v <= 0x5F for v in forma):
            raise ValueError(f"forma de pedra com valor fora de 0x54–0x5F: {forma}")
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
            "Tabelas da geração do mundo extraídas do SIMANTW.EXE por tools/extrair_geracao.py "
            "(endereços no topo do script). Formas de pedra: célula (x+i, y+j) recebe forma[j*N+i] "
            "quando não é 0. Ver docs/spec/regras/geracao-mundo.md. Valores em decimal."
            + nota_comuns("geracao")
        ),
        **sem_comuns("geracao", t),
    }
    # Uma lista por linha, para o arquivo ficar legível e fácil de editar num mod.
    linhas = ["{"]
    itens = list(saida.items())
    for i, (k, v) in enumerate(itens):
        fim = "," if i < len(itens) - 1 else ""
        if isinstance(v, list) and v and isinstance(v[0], list):
            corpo = ",\n".join("    " + json.dumps(x) for x in v)
            linhas.append(f'  "{k}": [\n{corpo}\n  ]{fim}')
        else:
            linhas.append(f'  "{k}": {json.dumps(v, ensure_ascii=False)}{fim}')
    linhas.append("}")
    with open(a.json, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(linhas) + "\n")
    print(f"gravado {a.json}")


if __name__ == "__main__":
    main()
