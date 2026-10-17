"""Extrai do SIMANTW.EXE as tabelas usadas pelos inimigos (formiga-leão, aranha, lagarta e
bicho-de-conta) e grava data/inimigos.json.

Uso:
    python tools/extrair_inimigos.py SIMANTW.EXE [--json data/inimigos.json]

Sem --json, só imprime as tabelas. Especificação: docs/spec/regras/inimigos.md.

Origem de cada tabela (endereço `seletor:offset`; seletor 1038 = segmento 8 do NE, 1048 = segmento
10, o DGROUP). O offset no arquivo vem da tabela de segmentos do NE (tools/ne_info.py). São dados
inicializados do executável, sem relocação.

| Chave no JSON       | Endereço  | Símbolo / uso                                                         |
|---------------------|-----------|-----------------------------------------------------------------------|
| dx8, dy8            | 1038:0000 | `_Dx8`/`_Dy8` (8:0000/0008): deslocamento por direção 0–7               |
| dx9, dy9            | 1038:0010 | `_Dx9`/`_Dy9` (8:0010/001a): índice 0 = parado, 1–8 = direções 0–7    |
| turn_tab            | 1038:0024 | `_TurnTab`, 8 × 8: linha = direção atual, coluna = alvo; valor = nova  |
| passo_aranha_x/y    | 1038:87dc | `_SpX`/`_SpY` (8:87dc/87e4): passo da aranha em 1/16 de célula         |
| boca_aranha_x/y     | 1038:87ec | `_SeatX`/`_SeatY` (8:87ec/87f4): célula da boca relativa à aranha      |
| sow_tab             | 1038:8a90 | `_SowTab`: tile do bicho-de-conta por direção                         |
| borda_formiga_leao  | 1048:25e8 | sem nome; somado a 0x30 por direção (`_AddRandAntLion`, `_DoAntLions`) |

As constantes da seção "regras" não são tabelas do executável: são valores imediatos lidos no
pseudocódigo das funções citadas em "_origem" (docs/spec/regras/inimigos.md). O script só as
escreve no JSON, para que fiquem fora do código do port e possam ser trocadas por mods.
"""
import argparse
import json
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from ne_info import parse_ne  # noqa: E402

SEG8 = 8
DGROUP = 10

# Nome: (valor, origem). Vistos no pseudocódigo (Code5.c: _MoveSpider e companhia; Code7.c:
# _DoAntLions, _DoPillar, _DoSow, _MakePillFood).
REGRAS = {
    # Formiga-leão
    "leao_chance_nova": (1024, "_DoAntLions 1030:43fa: sem nenhuma, S1(1024) == 0 cria uma"),
    "leao_chance_cuspir": (512, "_DoAntLions estado 0: pedrinha/comida no centro, S1(512) == 0 cospe"),
    "leao_tempo_cuspir": (25, "_DoAntLions estado 0: S depois de cuspir"),
    "leao_tempo_comer": (50, "_DoAntLions PEGA: S depois de pegar uma formiga"),
    "leao_cerco": (7, "_DoAntLions estado 1: vizinhas ocupadas para matar a formiga-leão"),
    "leao_maximo": (9, "_LionIndex só incrementa abaixo disto"),
    "leao_tentativas": (200, "sorteio de posição: tentativas"),
    "leao_tentativas_3x3": (100, "sorteio de posição: depois disto basta o centro livre"),
    "tile_leao_centro": (0x38, "centro da armadilha (quadro 0); quadro T = centro + T"),
    "tile_leao_borda": (0x30, "borda da armadilha + borda_formiga_leao[d]"),
    "tile_leao_morta": (0x3F, "formiga-leão morta"),
    "mensagem_leao_morta": (10050, "_PictStrnDialog(0, 0x2742): formigas mataram a formiga-leão"),
    # Aranha
    "aranha_chance_aparecer": (300, "_MoveSpider: S1(300) == 0 traz a aranha de volta"),
    "aranha_cerco_morte": (9, "_MoveSpider: células ocupadas no 3x3 para matar a aranha"),
    "aranha_cerco_fuga": (5, "_MoveSpider: a partir disto a aranha foge"),
    "aranha_agonia": (500, "_MoveSpider/_KillSpider: _DeathCnt inicial"),
    "aranha_vinganca_maxima": (5, "_SpidRevenge não passa disto"),
    "aranha_vinganca_faro": (3, "_SpidRevenge a partir do qual a próxima caça por faro (_MeSMode 7)"),
    "aranha_vinganca_laser": (5, "_SpidRevenge a partir do qual a próxima caça por laser (_MeSMode 8)"),
    "mensagem_aranha_morta": (10046, "_PictStrnDialog(0, 0x273e): aranha morta pelas formigas"),
    "aranha_chance_andar": (150, "estado 0: S1(150) == 0 passa a andar"),
    "aranha_chance_girar_parada": (30, "estado 0: S1(30) == 0 gira"),
    "aranha_chance_girar_andando": (20, "estado 1: S1(20) == 0 gira"),
    "aranha_chance_parar": (50, "estados 1 e 4: S1(50) == 0 para"),
    "aranha_corrida": (5, "multiplicador do passo ao correr"),
    "aranha_desiste": (64, "estado 2: distância |dx|+|dy| acima da qual desiste"),
    "aranha_mastigar": (50, "estado 3: _EatCnt normal (_MeSMode < 7)"),
    "aranha_mastigar_rapido": (11, "estado 3: _EatCnt com faro ou laser"),
    "aranha_arroto": (10, "_SpidBurpCnt reiniciado"),
    "aranha_faro": (801, "_SFoundAnt faro: distância² menor que isto"),
    "aranha_visao": (400, "_SFoundAnt visão: distância² máxima"),
    "aranha_visao_celulas": (20, "_SFoundAnt visão: células olhadas"),
    "aranha_laser_raio": (12, "_SpiderScan: raio = S1(12) + 1"),
    "aranha_agonia_lenta": (400, "estado 5: com _DeathCnt <= isto, quadro S1(2)+2; senão S1(3)+1"),
    "aranha_teto_casa": (0x90, "na casa, tile acima disto sob a aranha a tira do mapa"),
    # Lagarta e bicho-de-conta
    "lagarta_cerco": (6, "_DoPillar: células ocupadas no 3x3 da cabeça para matar a lagarta"),
    "lagarta_fases": (5, "_PillarSeg depois de avançar"),
    "lagarta_fase_inicial": (4, "_PillarSeg ao criar"),
    "lagarta_comida": (0x4B, "_MakePillFood: comida com 4 unidades"),
    "lagarta_limite_comida": (0x18, "_MakePillFood: só vira comida se o tile for menor que isto"),
    "tiles_lagarta_cabeca": ([0x6C, 0x6B, 0x6F, 0x68], "cabeça por direção 0 sobe, 1 direita, 2 desce, 3 esquerda"),
    "tiles_lagarta_corpo_1": ([0x6D, 0x69], "segmento 'atrás(s+1)' e atrás(1): vertical, horizontal"),
    "tiles_lagarta_corpo_2": ([0x6E, 0x6A], "segmento 'atrás(s)': vertical, horizontal"),
    "lagarta_limites": ([-6, 134, -6, 69], "fora de [xmin, xmax] x [ymin, ymax] a lagarta some"),
    "bicho_chao": (0x10, "_DoSow: só anda para tile menor que isto"),
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
        "dx9": com_sinal(ler(data, segs, SEG8, 0x0010, 9)),
        "dy9": com_sinal(ler(data, segs, SEG8, 0x001A, 9)),
        "turn_tab": [list(ler(data, segs, SEG8, 0x0024 + 8 * d, 8)) for d in range(8)],
        "passo_aranha_x": com_sinal(ler(data, segs, SEG8, 0x87DC, 8)),
        "passo_aranha_y": com_sinal(ler(data, segs, SEG8, 0x87E4, 8)),
        "boca_aranha_x": com_sinal(ler(data, segs, SEG8, 0x87EC, 8)),
        "boca_aranha_y": com_sinal(ler(data, segs, SEG8, 0x87F4, 8)),
        "sow_tab": list(ler(data, segs, SEG8, 0x8A90, 8)),
        "borda_formiga_leao": list(ler(data, segs, DGROUP, 0x25E8, 8)),
    }
    # Conferências mínimas.
    if (t["dx9"][0], t["dy9"][0]) != (0, 0) or t["dx9"][1:] != t["dx8"] or t["dy9"][1:] != t["dy8"]:
        raise ValueError(f"_Dx9/_Dy9 inesperadas: {t['dx9']} {t['dy9']}")
    if t["boca_aranha_x"] != t["dx8"] or t["boca_aranha_y"] != t["dy8"]:
        raise ValueError("_SeatX/_SeatY deveriam ser iguais a _Dx8/_Dy8")
    if any(not 0x70 <= v <= 0x73 for v in t["sow_tab"]):
        raise ValueError(f"_SowTab fora de 0x70–0x73: {t['sow_tab']}")
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
            "Tabelas e constantes dos inimigos (formiga-leão, aranha, lagarta, bicho-de-conta). As tabelas "
            "foram extraídas do SIMANTW.EXE por tools/extrair_inimigos.py (endereços no topo do script); "
            "turn_tab[atual][alvo] = nova direção. As constantes de 'regras' vêm do pseudocódigo (origem em "
            "'_origem'). Ver docs/spec/regras/inimigos.md. Valores em decimal."
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
            sub = list(v.items())
            corpo = ",\n".join(f"    {json.dumps(sk)}: {json.dumps(sv, ensure_ascii=False)}" for sk, sv in sub)
            linhas.append(f'  "{k}": {{\n{corpo}\n  }}{fim}')
        else:
            linhas.append(f'  "{k}": {json.dumps(v, ensure_ascii=False)}{fim}')
    linhas.append("}")
    with open(a.json, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(linhas) + "\n")
    print(f"gravado {a.json}")


if __name__ == "__main__":
    main()
