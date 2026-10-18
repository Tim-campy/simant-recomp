"""Extrai do SIMANTW.EXE as tabelas do quintal (menino, cachorro, gato, colônias) e grava data/quintal.json.

Uso:
    python tools/extrair_quintal.py SIMANTW.EXE [--json data/quintal.json]

Sem --json, só imprime as tabelas. Especificação: docs/spec/regras/quintal.md (tabelas e endereços na
seção "Tabelas do executável") e docs/spec/regras/conselhos.md (lições do tutorial).

Origem das tabelas (endereço `seletor:offset`; seletor 1038 = segmento 8, 1048 = segmento 10 = DGROUP).
São dados inicializados do executável, sem relocação. O offset no arquivo vem da tabela de segmentos do
NE (tools/ne_info.py).

| Chave no JSON                  | Endereço            | Uso                                                      |
|--------------------------------|---------------------|----------------------------------------------------------|
| dx8, dy8                       | 1038:0000 / 0008    | `_Dx8`/`_Dy8`; passo do gato (×4) em `_SimCat` (só conferida; vai para comum.json) |
| menino.passo_x / passo_y       | 1038:8a76 / 8a7a    | `_BxTab`/`_ByTab`: passo do menino por direção 0–3       |
| menino.quadro_por_passo        | 1048:25bc (4 bytes) | quadro somado pela fase `_YardCycle & 3`                  |
| menino.quadro_por_dir          | 1048:25c0 (4 bytes) | quadro base por direção                                  |
| menino.corte_dx / corte_dy     | 1048:25c4 / 25cc    | 4 palavras: posição do cortador relativa ao pé           |
| menino.vizinho_dx / vizinho_dy | 1048:25d4 / 25d8    | 4 bytes com sinal: patch vizinho por direção (cortar)    |
| menino.dir_do_quadro           | 1048:25b0 (12)      | direção do pé pelo quadro (menino dentro de casa)        |
| cachorro.passo_x / passo_y     | 1048:236a / 2370    | 6 bytes com sinal por direção (4 e 5 = sentado)          |
| cachorro.quadro_por_passo      | 1048:2376 (4)       | quadro somado pela fase                                  |
| cachorro.quadro_por_dir        | 1048:237a (6)       | quadro base por direção                                  |
| casa.duracao                   | 1048:2380 (22)      | `_NodeCnt` máximo de cada nó 0–21 do roteiro da casa     |
| casa.proximo                   | 1048:2534/254a/2560 | 3 × 22 bytes: próximo nó, escolhido por `SRand1(3)`      |
| casa.roteiros                  | 1048:2396…2533      | por nó: quadro, x, y de cada passo (endereços abaixo)    |
| casa.nos_fixos                 | 1048:2576…25ae      | nós 30–38: quadro (palavra), x, y, chance, próximo       |
| colonias.vizinho_dx / dy       | 1048:25dc / 25e2    | 6 bytes lidos SEM sinal por `_SimColonies`               |

As constantes de "regras", "valores_iniciais", "mensagens" e "conselhos" não são tabelas do executável:
são valores imediatos lidos no pseudocódigo (Code7.c) das funções citadas em "_origem". O script só as
escreve no JSON, para que fiquem fora do código do port e possam ser trocadas por mods.

As tabelas marcadas "vai para comum.json" são lidas só para as conferências: desde a API de mods 0.20 elas ficam em
data/comum.json (tools/extrair_comum.py), e este script não as grava.
"""
import argparse
import json
import struct
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from ne_info import parse_ne  # noqa: E402
from extrair_comum import sem_comuns, nota_comuns  # noqa: E402
from sons_regras import SONS  # noqa: E402

SEG8 = 8
DGROUP = 10

# (nó, endereço dos quadros, dos x, dos y, quadros em palavras?) — vistos no switch de _SimKidInside (1030:20b6).
ROTEIROS = [
    (0, 0x2396, 0x239A, 0x239E, False), (1, 0x23A2, 0x23A6, 0x23AA, False),
    (2, 0x23AE, 0x23C2, 0x23D6, False), (3, 0x23EA, 0x23FE, 0x2412, False),
    (4, 0x2426, 0x242A, 0x242E, False), (5, 0x2432, 0x2436, 0x243A, False),
    (6, 0x243E, 0x244A, 0x2456, False), (7, 0x2462, 0x246E, 0x247A, False),
    (8, 0x2486, 0x248C, 0x2492, False), (9, 0x2498, 0x249E, 0x24A4, False),
    (10, 0x24AA, 0x24AE, 0x24B2, False), (11, 0x24B6, 0x24BA, 0x24BE, False),
    (12, 0x24C2, 0x24C8, 0x24CE, False), (13, 0x24D4, 0x24DA, 0x24E0, False),
    (14, 0x24E6, 0x24EA, 0x24EE, False), (15, 0x24F2, 0x24F6, 0x24FA, False),
    (20, 0x24FE, 0x250E, 0x2516, True), (21, 0x251E, 0x2528, 0x252E, True),
]

REGRAS = {
    "mac_por_tick": (6, "[hipótese] tiques do relógio do Mac (1/60 s) por tick do port; ~10 ticks/s na velocidade normal"),
    "chuva_tecla": (0xA8, "_DoSimYard 1030:14a8: _GlobalKey que força a chuva"),
    "chuva_chance_1": (32, "_DoSimYard: RRand(32) == 0"),
    "chuva_chance_2": (128, "_DoSimYard: e depois RRand(128) == 0"),
    "chuva_duracao_min": (150, "_DoSimYard: _RainCnt = SRand1(150) + 150"),
    "chuva_duracao_sorteio": (150, "_DoSimYard"),
    "msg_menino_duracao": (300, "_SendBoyMsg 1030:15ae: _BoyMsgCnt = relógio + 300"),
    "msg_menino_limite": (0x17, "_SendBoyMsg: só aceita mensagem < 23"),
    "gato_atraso": (200, "_SimCat 1030:2b24: _CatDelay = relógio + 200 a cada teste"),
    "gato_atraso_fugiu": (600, "_SimCat: _CatDelay = relógio + 600 ao sumir"),
    "gato_chance": (16, "_SimCat: aparece se SRand16() == 0"),
    "gato_tecla": (0x8D, "_SimCat: _GlobalKey que força o gato"),
    "passaro_atraso": (20, "_SimBird 1030:28aa: _BirdDelay = _gameCycles + 20"),
    "passaro_atraso_saiu": (30, "_SimBird: _BirdDelay = _gameCycles + 30 ao sair da tela"),
    "latido_atraso": (60, "_SimDog 1030:2dbc: _DogBarkDelay = relógio + 60 + SRand1(30)"),
    "latido_sorteio": (30, "_SimDog"),
    "colonia_teto": (250, "_SimColonies 1030:35f8: população máxima de um patch"),
    "colonia_chance_reproduzir": (10, "_SimColonies: SRand1(10) == 0 com o patch no teto"),
    "colonia_espalhar": (4, "_SimColonies/_Reproduce: SGSRand(4) em x e em y"),
    "casa_limite": (24, "_SimColonies: casa tomada com _HouseColonies > 24"),
    "nova_vermelha": (20, "_SimColonies e _CountAnts: população da colônia vermelha nova"),
    "enxame_atraso_min": (10, "_SimColonies: revoada guardada solta se SRand8() + 10 < _SwarmDelay"),
    "enxame_teto": (50, "_SimColonies: _SwarmCnt ≤ 50"),
    "inseticida_por_formiga": (8, "_SimKidInside: (_RpopT + _BpopT) × 8 sorteios"),
}

VALORES_INICIAIS = {
    "_origem": "_InitSimYard 1030:1378; demais campos do menino, gato, pássaro e cachorro vão a 0",
    "menino_x": 0xB4, "menino_y": 0x49, "menino_dir": 2, "menino_giro": 0xC, "menino_espera": 0x14,
    "cachorro_x": 0xFA, "cachorro_y": 0x96, "cachorro_giro": 2,
    "passaro_quadro": 1, "menino_msg": -1, "ciclo_quintal": -1,
}

BOTAO_REVOADA = {
    "_origem": "_ProcYardEvent 1010:a51e, comando 0x2306 (ribMate): só com _CurGameType = 2; _CastePopB[3] + [4] "
               "menos 1 se _MeType é 0x40 ou 0x20; >= 1: _FlyAwayB = 200 e 1010[6], senão som 1 e 1010[5]; fora do "
               "Jogo Completo som 1 e 1010[15]. _EditMessage(texto, 0xb4, 0, 1): importante",
    "contagem": 200,
    "castas": [3, 4],
    "tipos_amarela": [0x40, 0x20],
    "som_erro": 1,
    "faixa_armou": {"lista": 1010, "indice": 6, "duracao": 180, "importante": True},
    "faixa_sem_reprodutores": {"lista": 1010, "indice": 5, "duracao": 180, "importante": True},
    "faixa_so_jogo_completo": {"lista": 1010, "indice": 15, "duracao": 180, "importante": True},
}

MENSAGENS = {
    "_origem": "_SimColonies 1030:35f8 (_PictStrnDialog e _myBeginSong); SHARED tipo 4",
    "vermelhas_eliminadas": 10052, "falta_vermelhas": 10053, "casa_tomada": 10054,
    "tome_a_casa": 10055, "derrota": 10056, "vitoria": 10057,
    "musica_vermelhas_eliminadas": 0x4E22, "musica_casa_tomada": 0x4E23, "musica_vitoria": 0x4E25,
}


def texto(n):
    """Texto da lição n (SHARED 4/11000 + 2(n − 1)); visto no switch de _GiveLesson (1030:7b4e)."""
    return 11000 + 2 * (n - 1)


def licoes():
    """Tabela de _GiveLesson (1030:7b4e) e _LessonDone (1030:7f02). Ver docs/spec/regras/conselhos.md §3."""
    nada = {"tipo": "nada"}
    sempre = {"tipo": "sempre"}
    imagens = {2: 17000, 5: 0x4269, 0xB: 0x4276, 0xD: 0x426A, 0xE: 0x426A, 0x11: 0x4275, 0x12: 0x426B,
               0x15: 0x426C, 0x21: 0x426D, 0x24: 0x426E, 0x26: 0x426F, 0x27: 0x4270, 0x28: 0x4270,
               0x29: 0x4270, 0x2B: 0x4271, 0x2C: 0x4271, 0x2D: 0x4272, 0x2E: 0x4272, 0x2F: 0x4273,
               0x30: 0x4273, 0x32: 0x4274}
    ao_dar = {
        3: {"tipo": "marco_escavacao", "soma": 2},
        4: {"tipo": "marco_escavacao", "soma": 10},
        7: {"tipo": "marcar_posicao_e_esperar", "espera": 600},
        0x1A: {"tipo": "marcar_posicao_e_esperar", "espera": 600},
        9: {"tipo": "marcar_vista_e_esperar", "espera": 600},
        8: {"tipo": "zerar_temp"}, 0x1E: {"tipo": "zerar_temp"}, 0x2E: {"tipo": "zerar_temp"},
        0x32: {"tipo": "zerar_temp"},
        0x1B: {"tipo": "marcar_posicao"},
        0x29: {"tipo": "marcar_tarefas"},
        0xE: {"tipo": "saude_amarela", "valor": 8},
        0x17: {"tipo": "formigas_pretas", "n": 10},
        0x31: {"tipo": "formigas_pretas_e_limpar_alarme", "n": 0x50},
        0x2B: {"tipo": "pular_licao"},
    }
    concl = {}
    for n in (1, 2, 6, 0xB, 0xF, 0x11, 0x17, 0x19, 0x1C, 0x1D, 0x1F, 0x22, 0x23, 0x25, 0x28, 0x2C, 0x2D,
              0x30, 0x33, 0x35, 0x37, 0x38):
        concl[n] = sempre
    concl.update({
        3: {"tipo": "escavou_alem_do_marco"}, 4: {"tipo": "escavou_alem_do_marco"},
        5: {"tipo": "no_plano", "plano": 1}, 0x14: {"tipo": "no_plano", "plano": 1},
        0x12: {"tipo": "no_plano", "plano": 2},
        7: {"tipo": "andou_e_esperou"}, 0x1A: {"tipo": "andou_e_esperou"},
        9: {"tipo": "vista_mudou_ou_esperou"},
        10: {"tipo": "janela_na_frente", "janela": 0x100}, 0x20: {"tipo": "janela_na_frente", "janela": 0x100},
        0x2A: {"tipo": "janela_na_frente", "janela": 0x100},
        0xC: {"tipo": "janela_na_frente", "janela": 0}, 0x31: {"tipo": "janela_na_frente", "janela": 0},
        0x27: {"tipo": "janela_na_frente", "janela": 0x1200},
        0x2F: {"tipo": "janela_na_frente", "janela": 0x1300},
        0xD: {"tipo": "superficie_acima_de", "tile": 0x47},
        0xE: {"tipo": "saude_acima_de", "valor": 90},
        0x10: {"tipo": "tipo_amarela", "tipo_formiga": 0x18},
        0x13: {"tipo": "tipo_amarela", "tipo_formiga": 0x10}, 0x16: {"tipo": "tipo_amarela", "tipo_formiga": 0x10},
        0x15: {"tipo": "tipo_amarela", "tipo_formiga": 0x28},
        0x18: {"tipo": "defesa_acima_de", "valor": 1}, 0x34: {"tipo": "defesa_acima_de", "valor": 0x28},
        0x1B: {"tipo": "andou_soltando_alarme"},
        0x1E: {"tipo": "temp_marcado_desliga_alarme"},
        8: {"tipo": "temp_marcado"}, 0x2B: {"tipo": "temp_marcado"}, 0x2E: {"tipo": "temp_marcado"},
        0x32: {"tipo": "temp_marcado"},
        0x21: {"tipo": "plano_do_mapa", "plano": 0}, 0x26: {"tipo": "plano_do_mapa", "plano": 1},
        0x24: {"tipo": "modo_quintal", "modo": 1},
        0x29: {"tipo": "tarefas_mudaram"},
        0x36: {"tipo": "entrou_num_ninho"},
    })
    out = []
    for n in range(1, 0x39):
        t = texto(n)
        if n == 0x2B:
            t = texto(0x2C)  # case 0x2b cai no 0x2c depois de somar 1 a _TutLesson
        if n == 0x38:
            t = 0x2B68
        li = {"n": n, "texto": t, "imagem": imagens.get(n, 0), "ao_dar": ao_dar.get(n, nada),
              "concluida": concl[n]}
        if n == 0x37:
            li["texto"] = 0x2B64
            li["texto_se_temp"] = 0x2B66
        out.append(li)
    return out


def ler(data, segs, seg, off, n):
    s = segs[seg - 1]
    if off + n > s["file_length"]:
        raise ValueError(f"{seg}:{off:#x}+{n} passa do fim do segmento no arquivo")
    o = s["file_offset"] + off
    return data[o:o + n]


def com_sinal(b):
    return [x - 256 if x > 127 else x for x in b]


def palavras(b):
    return list(struct.unpack(f"<{len(b) // 2}h", b))


def extrair(data):
    segs = parse_ne(data)["segments"]

    def d(off, n):
        return ler(data, segs, DGROUP, off, n)

    duracao = list(d(0x2380, 22))
    roteiros = []
    for no, af, ax, ay, pal in ROTEIROS:
        n = duracao[no] + 1
        # Conferência: os n passos cabem antes da tabela seguinte.
        if pal and af + 2 * n > ax or not pal and af + n > ax or ax + n > ay:
            raise ValueError(f"roteiro do nó {no}: {n} passos não cabem nas tabelas")
        quadros = palavras(d(af, 2 * n)) if pal else list(d(af, n))
        roteiros.append({"no": no, "quadro": quadros, "x": list(d(ax, n)), "y": list(d(ay, n))})
    nos_fixos = []
    for no in range(30, 39):
        nos_fixos.append({
            "no": no,
            "quadro": palavras(d(0x2576 + (no - 30) * 2, 2))[0],
            "x": d(0x256A + no, 1)[0], "y": d(0x2574 + no, 1)[0],
            "chance": d(0x257E + no, 1)[0], "proximo": d(0x2588 + no, 1)[0],
        })
    t = {
        "_comentario": "Tabelas do quintal e do tutorial. As de 'menino', 'cachorro', 'casa', 'colonias' "
                       "foram extraídas do SIMANTW.EXE por tools/extrair_quintal.py (endereços no topo do script); "
                       "'regras', 'valores_iniciais', 'mensagens' e 'conselhos' vêm do pseudocódigo (origem em "
                       "'_origem'). Ver docs/spec/regras/quintal.md e conselhos.md. Valores em decimal." + nota_comuns("quintal"),
        "dx8": com_sinal(ler(data, segs, SEG8, 0x0000, 8)),
        "dy8": com_sinal(ler(data, segs, SEG8, 0x0008, 8)),
        "menino": {
            "passo_x": com_sinal(ler(data, segs, SEG8, 0x8A76, 4)),
            "passo_y": com_sinal(ler(data, segs, SEG8, 0x8A7A, 4)),
            "quadro_por_passo": com_sinal(d(0x25BC, 4)),
            "quadro_por_dir": com_sinal(d(0x25C0, 4)),
            "corte_dx": palavras(d(0x25C4, 8)),
            "corte_dy": palavras(d(0x25CC, 8)),
            "vizinho_dx": com_sinal(d(0x25D4, 4)),
            "vizinho_dy": com_sinal(d(0x25D8, 4)),
            "dir_do_quadro": list(d(0x25B0, 12)),
        },
        "cachorro": {
            "passo_x": com_sinal(d(0x236A, 6)),
            "passo_y": com_sinal(d(0x2370, 6)),
            "quadro_por_passo": com_sinal(d(0x2376, 4)),
            "quadro_por_dir": com_sinal(d(0x237A, 6)),
        },
        "casa": {
            "duracao": duracao,
            "proximo": [list(d(0x2534, 22)), list(d(0x254A, 22)), list(d(0x2560, 22))],
            "roteiros": roteiros,
            "nos_fixos": nos_fixos,
        },
        "colonias": {
            "vizinho_dx": list(d(0x25DC, 6)),
            "vizinho_dy": list(d(0x25E2, 6)),
        },
        "regras": {k: v for k, (v, _) in REGRAS.items()},
        "valores_iniciais": VALORES_INICIAIS,
        "mensagens": MENSAGENS,
        "conselhos": {
            "_origem": "_Feedback 1030:78ca (conselhos: lista SHARED 4/1020, _AdviceStrs carregada por _initStuff); "
                       "_GiveLesson 1030:7b4e e _LessonDone 1030:7f02 (lições: textos SHARED 4/11000…11112)",
            "lista": 1020,
            "saude_critica": 10,
            "licao_minima_ticks": 16,
            "licoes": licoes(),
        },
        "_origem": {k: o for k, (_, o) in REGRAS.items()},
        "botao_revoada": BOTAO_REVOADA,
        "sons": SONS["quintal"],
    }
    return t


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("exe")
    ap.add_argument("--json")
    a = ap.parse_args()
    data = open(a.exe, "rb").read()
    t = sem_comuns("quintal", extrair(data))
    texto_json = json.dumps(t, ensure_ascii=False, indent=1)
    if a.json:
        with open(a.json, "w", encoding="utf-8", newline="\n") as f:
            f.write(texto_json + "\n")
        print(f"gravado {a.json}")
    else:
        print(texto_json)


if __name__ == "__main__":
    main()
