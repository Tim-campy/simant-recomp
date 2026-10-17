"""Extrai do SIMANTW.EXE as tabelas usadas pelas regras da formiga amarela e grava data/amarela.json.

Uso:
    python tools/extrair_amarela.py SIMANTW.EXE [--json data/amarela.json]

Sem --json, só imprime as tabelas. Especificação: docs/spec/regras/formiga-amarela.md.

Origem de cada tabela (endereço `segmento:offset` do symbols.tsv; segmento 8 = seletor 1038,
SIMANT_DATA_GROUP; segmento 10 = seletor 1048, DGROUP). O offset no arquivo vem da tabela de
segmentos do NE (tools/ne_info.py). São dados inicializados do executável, sem relocação.

| Chave no JSON      | Endereço  | Símbolo / uso                                                              |
|--------------------|-----------|----------------------------------------------------------------------------|
| dx8, dy8           | 8:0000/08 | `_Dx8`/`_Dy8`: deslocamento por direção 0–7 (0 = cima)                       |
| dx9, dy9           | 8:0010/1a | `_Dx9`/`_Dy9`: 9 entradas, índice 0 = parado, k = direção k − 1 (`_GetDir`) |
| desvios_busca      | 8:006c    | `_absSearchDirs`: 0, +1, −1, +2, … desvios tentados ao sair do ninho e largar |
| descarregar_grupo  | 8:0094    | `_UnCarryCaste`: 16 bytes, grupo sem a carga                                 |
| classe_luta        | 10:22c6   | classe de luta por grupo (`_GetWinner`)                                      |
| matriz_luta        | 10:22d6   | 4 × 4, chance em 10 de "a" vencer (`_GetWinner`)                             |
| borda_buraco       | 10:230c   | tile da borda do buraco por direção (`_MakeNewHole*`, `_CreateNewHole`)      |

As seções "regras", "animacao", "musicas" e "mensagens" não são tabelas do executável: são valores
imediatos vistos no pseudocódigo das funções citadas em "_origem". O script só as escreve no JSON, para
que fiquem fora do código do port e possam ser trocadas por mods.
"""
import argparse
import json
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from ne_info import parse_ne  # noqa: E402
from sons_regras import SONS  # noqa: E402

SEG8 = 8
SEG10 = 10

REGRAS = {
    "rastro_inicial": (200, "_PickupMyFood 1020:8f4c: _MeScent ao pegar comida"),
    "rastro_minimo": (10, "_DoAntMoveY 1028:75f4: _MeScent só desce enquanto > 10"),
    "alarme_por_passo": (50, "_DoAntMoveY: _AlarmHere(x, y, 0x32) a cada passo com _MeDropAlarm"),
    "teto_alarme": (200, "_AlarmHere 1028:943c: c = min(c + v, 200)"),
    "custo_ovo": (5, "_DoAntSimY 1028:7cf6: vida perdida por ovo posto pela amarela rainha"),
    "vida_maxima": (100, "_SetMyHealth 1020:8c70 / _EatMyFood 1020:8cda"),
    "fome_minima": (9, "_SetMyHealth: com vida ≤ 9 a amarela sempre quer comida"),
    "vida_canibal": (10, "_TryMyDropOrLift 1020:98de: com vida < 10 pega (e come) ovo"),
    "tempo_sem_vida": (99, "_DoAntSimY: morre quando _MeHealthCntDwn passa disto"),
    "raio_aranha": (512, "_DoLifeExchange 1020:6842: (16·dx)² + (16·dy)² < 512 até a aranha"),
    "custo_doar": (10, "_EatMyFood tipo 2: vida doada"),
}

ANIMACAO = {
    "janela_edicao_na_frente": (True, "[hipótese] estado da interface assumido pelo port (_win_IsWinInFront(0))"),
    "quadros_luta": (6, "_AnimYellowFight 1028:7e56, código 0"),
    "quadros_choque": (32, "_AnimYellowFight, código 10"),
    "quadros_morte_atras": (8, "_AnimYellowFight, códigos 1–9 sem a janela de edição à frente"),
    "quadros_morte_frente": (32, "_AnimYellowFight, códigos 1–9, janela à frente, música desligada"),
    "quadros_morte_frente_musica": (64, "_AnimYellowFight, códigos 1–9, janela à frente, música ligada"),
    "quadros_loucura_frente": (32, "_AnimYellowInsane 1028:8078, janela à frente"),
    "quadros_loucura_atras": (8, "_AnimYellowInsane, janela atrás"),
}

MUSICAS = {
    "recrutar_5": (0x2B05, "_YellowCommand 1000:87ba, comando 1"),
    "recrutar_10": (0x2B06, "comando 2"),
    "recrutar_todas": (0x2B07, "comando 3"),
    "liberar_metade": (0x2B08, "comando 4"),
    "liberar_todas": (0x2B09, "comando 5"),
    "morte_base": (0x4E27, "_YellowDeath 1000:7746: música 0x4E27 + S1(variações)"),
    "morte_variacoes": (5, "_YellowDeath: S1(5)"),
    "renascer": (0x2AFD, "_YellowBirth 1000:739e, renascer como ovo"),
    "fundar_inicio": (20000, "_YellowBirth, fundar a colônia (antes do ovo)"),
    "fundar_fim": (0x4E21, "_YellowBirth, fim da fundação"),
    "virou_rainha": (0x2AFE, "_ExchangeLives 1020:67ac: novo corpo é a rainha"),
}

MENSAGENS = {
    "luta_atacou": (10030, "_YellowDeath(0) com _MeStartedFight"),
    "luta_atacada": (10032, "_YellowDeath(0) sem _MeStartedFight"),
    "cercada": (10026, "_YellowDeath(3)"),
    "fugiu": (10028, "_YellowDeath(4)"),
    "pisada": (10034, "_YellowDeath(5)"),
    "cortador": (10036, "_YellowDeath(6)"),
    "afogada": (10038, "_YellowDeath(7)"),
    "fome": (10040, "_YellowDeath(8)"),
    "inseticida": (10042, "_YellowDeath(9)"),
    "choque": (10045, "_YellowDeath(10)"),
    "sem_corpo": (10056, "_YellowDeath: derrota no Jogo Completo"),
    "outra_colonia": (10058, "_YellowDeath: escolher outra colônia (_SpecialXfer)"),
    "transferida": (10059, "_YellowDeath: transferida para um adulto"),
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
    matriz = list(ler(data, segs, SEG10, 0x22D6, 16))
    t = {
        "dx8": com_sinal(ler(data, segs, SEG8, 0x0000, 8)),
        "dy8": com_sinal(ler(data, segs, SEG8, 0x0008, 8)),
        "dx9": com_sinal(ler(data, segs, SEG8, 0x0010, 9)),
        "dy9": com_sinal(ler(data, segs, SEG8, 0x001A, 9)),
        "desvios_busca": com_sinal(ler(data, segs, SEG8, 0x006C, 8)),
        "descarregar_grupo": list(ler(data, segs, SEG8, 0x0094, 16)),
        "classe_luta": list(ler(data, segs, SEG10, 0x22C6, 16)),
        "matriz_luta": [matriz[4 * i:4 * i + 4] for i in range(4)],
        "borda_buraco": list(ler(data, segs, SEG10, 0x230C, 8)),
    }
    # Conferências: _Dx9/_Dy9 = parado + _Dx8/_Dy8; desvios cobrem as 8 direções.
    if t["dx9"][1:] != t["dx8"] or t["dy9"][1:] != t["dy8"] or (t["dx9"][0], t["dy9"][0]) != (0, 0):
        raise ValueError(f"_Dx9/_Dy9 inesperadas: {t['dx9']} {t['dy9']}")
    if sorted(d % 8 for d in t["desvios_busca"]) != list(range(8)):
        raise ValueError(f"_absSearchDirs inesperada: {t['desvios_busca']}")
    if any(c > 3 for c in t["classe_luta"]):
        raise ValueError(f"classe de luta fora de 0–3: {t['classe_luta']}")
    return t


def secao(d):
    return {k: v for k, (v, _) in d.items()}


def origens(*ds):
    o = {}
    for d in ds:
        o.update({k: orig for k, (_, orig) in d.items()})
    return o


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
            "Tabelas e constantes da formiga amarela. As tabelas foram extraídas do SIMANTW.EXE por "
            "tools/extrair_amarela.py (endereços no topo do script); as seções regras/animacao/musicas/"
            "mensagens vêm do pseudocódigo (origem em '_origem'). Ver docs/spec/regras/formiga-amarela.md. "
            "Valores em decimal."
        ),
        **t,
        "regras": secao(REGRAS),
        "animacao": secao(ANIMACAO),
        "musicas": secao(MUSICAS),
        "mensagens": secao(MENSAGENS),
        "_origem": origens(REGRAS, ANIMACAO, MUSICAS, MENSAGENS),
        "sons": SONS["amarela"],
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
