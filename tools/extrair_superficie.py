"""Extrai do SIMANTW.EXE as tabelas usadas pelas regras da superfície e grava data/superficie.json.

Uso:
    python tools/extrair_superficie.py SIMANTW.EXE [--json data/superficie.json]

Sem --json, só imprime as tabelas. Especificação: docs/spec/regras/superficie.md (seções 3.8, 6.2, 9.1 e
9.3) e formigas.md (seção 5, "Escolha de modo").

Origem de cada tabela (endereço `segmento lógico:offset`; 8 = seletor 1038, SIMANT_DATA_GROUP; 10 = seletor
1048, DGROUP). O offset no arquivo vem da tabela de segmentos do NE (tools/ne_info.py). São dados
inicializados do executável, sem relocação.

| Chave no JSON          | Endereço | Símbolo / uso                                                              |
|------------------------|----------|----------------------------------------------------------------------------|
| carrega_carga          | 8:0074   | `_IsCarryCaste`, 16 palavras: 1 = o grupo carrega algo (ovo, comida, terra)|
| grupo_sem_carga        | 8:0094   | `_UnCarryCaste`, 16 bytes: grupo depois de largar a carga (`_DoAttackAnt`) (só conferida; vai para comum.json) |
| casta_por_escolha      | 8:89ae   | `_CasteTabB`, 4 palavras: grupo do próximo ovo (`_SetCasteProd`)           |
| modo_por_tarefa        | 8:89b6   | `_ModeTabB`, 3 palavras: modo para forragear/cavar/cuidar (`_SetModeProd`) |
| modos_operaria         | 8:89e6   | `_ModeTabWB`, 6 × 8 bytes: linha = modo estratégico, coluna = `SRand8` (só conferida; vai para comum.json) |
| modos_soldado          | 8:8a16   | `_ModeTabSB`, 6 × 8 bytes (só conferida; vai para comum.json) |
| modo_por_grupo         | 8:8a46   | `_CasteModeTabB`, 16 bytes: modo dos grupos que não sorteiam (só conferida; vai para comum.json) |
| classe_luta            | 10:22c6  | 16 bytes: classe de luta por grupo (0 operária/ovo, 1 soldado, 2 reprodutor, 3 rainha) (só conferida; vai para comum.json) |
| matriz_luta            | 10:22d6  | 4 × 4 bytes: chance (em 10) de a formiga da célula ("a") vencer, `_GetWinner` (só conferida; vai para comum.json) |

As constantes da seção "regras" não são tabelas do executável: são valores imediatos lidos no pseudocódigo
das funções citadas em "_origem" (docs/spec/regras/superficie.md). O script só as escreve no JSON, para que
fiquem fora do código do port e possam ser trocadas por mods.

As tabelas marcadas "vai para comum.json" são lidas só para as conferências: desde a API de mods 0.20 elas ficam em
data/comum.json (tools/extrair_comum.py), e este script não as grava.
"""
import argparse
import json
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from ne_info import parse_ne  # noqa: E402
from extrair_comum import sem_comuns, nota_comuns  # noqa: E402
from sons_regras import SONS  # noqa: E402

SEG8 = 8
SEG10 = 10

# Valor, função de origem e significado (pseudocódigo em re_trabalho/decomp/Code6.c e Code7.c).
REGRAS = {
    "s_ao_pegar_comida": (200, "_DoRandAntA/_DoToNestAnt/_DoForageAnt: S = 200 ao pegar comida (trilha de volta)"),
    "alarme_de_luta": (40, "_StartFightA e embutidos: _AlarmHere2(x, y, 40) na célula da nuvem de luta"),
    "cheiro_defender_ninho": (110, "_DoDefendNest (modo 12): cheiro de ninho abaixo disto -> volta ao ninho"),
    "cheiro_decolar": (100, "_DoRepoExit (modo 15): cheiro de ninho abaixo disto -> volta ao ninho"),
    "revoada_pre_voo": (100, "_DoRepoLoit (modo 14): _FlyAway* acima disto -> modo 15"),
    "limite_enxame": (50, "_DoRepoFly (modo 16): só sai do mapa se _SwarmCnt* < limite"),
    "populacao_lider": (30, "_MakeRedInitiator: _BpopT >= isto cria a líder vermelha"),
    "tipo_lider": (0xB0, "_MakeRedInitiator: T gravado na líder (soldado vermelho, direção 0)"),
    "raio_aranha_perseguida": (100, "_GetStrategy: _GetDis(aranha, amarela) < isto liga _ChaseSpid"),
    "saude_desespero": (10, "_GetStrategy/_GstrR: saúde abaixo disto pode disparar o ataque desesperado"),
    "saude_fome": (30, "_GetStrategy/_GstrR: saúde abaixo disto -> modo estratégico 5"),
    "saude_pouca": (50, "_GetStrategy/_GstrR: saúde abaixo disto -> modo estratégico 4"),
    "populacao_grande": (101, "_GetStrategy/_GstrR: população abaixo disto é 'normal' (modo 1)"),
    "revoada_vermelha_reprodutores": (20, "_GstrR: machos + rainhas novas acima disto armam _FlyAwayR"),
    "revoada_vermelha_contagem": (200, "_GstrR: valor posto em _FlyAwayR"),
    "malvadeza_minima_populacao": (21, "_GstrR: população vermelha abaixo disto não ataca por sorteio"),
    "malvadeza_base": (30, "_GstrR: _JustToBeMean = SRand1(malvadeza_sorteio) + malvadeza_base"),
    "malvadeza_sorteio": (100, "_GstrR: ver malvadeza_base"),
    "musica_ataque_vermelho": (0x2B0A, "_GstrR: música tocada no ataque vermelho (_myBeginSong)"),
    "mensagem_ataque_vermelho": (
        {"lista": 1020, "indice": 5, "duracao": 120, "importante": False},
        "_GstrR (1030:03c2) e _StartAttack (1030:050e): _EditMessage(_AdviceStrs[5] = SHARED 4/1020[5] (aviso do ataque vermelho), 0x78, 0)",
    ),
}


def ler(data, segs, seg, off, n):
    s = segs[seg - 1]
    if off + n > s["file_length"]:
        raise ValueError(f"{seg}:{off:#x}+{n} passa do fim do segmento no arquivo")
    o = s["file_offset"] + off
    return data[o:o + n]


def palavras(b):
    return [int.from_bytes(b[i:i + 2], "little", signed=True) for i in range(0, len(b), 2)]


def com_sinal(b):
    return [x - 256 if x > 127 else x for x in b]


def extrair(data):
    segs = parse_ne(data)["segments"]
    t = {
        "carrega_carga": palavras(ler(data, segs, SEG8, 0x0074, 32)),
        "grupo_sem_carga": com_sinal(ler(data, segs, SEG8, 0x0094, 16)),
        "casta_por_escolha": palavras(ler(data, segs, SEG8, 0x89AE, 8)),
        "modo_por_tarefa": palavras(ler(data, segs, SEG8, 0x89B6, 6)),
        "modos_operaria": [list(ler(data, segs, SEG8, 0x89E6 + 8 * k, 8)) for k in range(6)],
        "modos_soldado": [list(ler(data, segs, SEG8, 0x8A16 + 8 * k, 8)) for k in range(6)],
        "modo_por_grupo": list(ler(data, segs, SEG8, 0x8A46, 16)),
        "classe_luta": list(ler(data, segs, SEG10, 0x22C6, 16)),
        "matriz_luta": [list(ler(data, segs, SEG10, 0x22D6 + 4 * k, 4)) for k in range(4)],
    }
    # Conferências contra os valores registrados na spec (superficie.md 3.8 e 6.2, formigas.md 5).
    esperado = {
        "carrega_carga": [0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0],
        "grupo_sem_carga": [0, 2, 2, 2, 4, 2, 6, 6, 8, 6, 10, 11, 12, 13, 14, 15],
        "casta_por_escolha": [2, 6, 4, 8],
        "modo_por_tarefa": [2, 4, 1],
        "modo_por_grupo": [8, 1, 0, 3, 14, 5, 0, 3, 14, 5, 0, 0, 9, 9, 10, 0],
        "classe_luta": [0, 0, 0, 0, 2, 0, 1, 1, 2, 1, 0, 0, 3, 3, 0, 0],
        "matriz_luta": [[5, 2, 7, 3], [8, 5, 9, 4], [3, 1, 5, 2], [7, 6, 8, 5]],
    }
    for k, v in esperado.items():
        if t[k] != v:
            raise ValueError(f"{k} difere da spec: lido {t[k]}, esperado {v}")
    for k in ("modos_operaria", "modos_soldado"):
        if any(not 0 <= m <= 19 for linha in t[k] for m in linha):
            raise ValueError(f"{k} com modo fora de 0–19")
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
            "Tabelas e constantes das regras da superfície. As tabelas foram extraídas do SIMANTW.EXE por "
            "tools/extrair_superficie.py (endereços no topo do script); as constantes de 'regras' vêm do "
            "pseudocódigo (origem em '_origem'). Ver docs/spec/regras/superficie.md. Valores em decimal."
            + nota_comuns("superficie")
        ),
        **sem_comuns("superficie", t),
        "regras": {k: v for k, (v, _) in REGRAS.items()},
        "_origem": {k: o for k, (_, o) in REGRAS.items()},
        "sons": SONS["superficie"],
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
