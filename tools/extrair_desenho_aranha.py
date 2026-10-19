"""Extrai do SIMANTW.EXE as tabelas do desenho da aranha na janela de edição (`_DrawSpider`,
`_DrawLegs`, `_DrawPalps`) e grava data/desenho_aranha.json.

Uso:
    python tools/extrair_desenho_aranha.py SIMANTW.EXE [--json data/desenho_aranha.json]

Sem --json, só imprime as tabelas. Especificação: docs/spec/telas/jogo.md, "Aranha na janela de
edição". Todas as tabelas estão no segmento 8 do NE (seletor 1038), com os nomes da tabela de símbolos
(re_trabalho/symbols.tsv); são dados inicializados, sem relocação.

| Chave no JSON                         | Endereço       | Símbolo e uso                                                   |
|---------------------------------------|----------------|-----------------------------------------------------------------|
| corpo_x, corpo_y                      | 8:8804 / 880c  | `_BodX`/`_BodY`: canto do sprite do corpo (1000 + dir) em relação ao centro da aranha |
| agonia_x, agonia_y                    | 8:8814 / 8818  | `_DBodX`/`_DBodY`: idem para o sprite da agonia (1050 + `_Scycle`) |
| pernas_reto.inicio_x/y                | 8:884c / 8850  | `_Lx`/`_Ly`: onde cada perna sai do corpo (direção 0)           |
| pernas_reto.joelho_x/pe_x             | 8:8854 + 16k / 885c + 16k | `_L{k}xA`/`_L{k}xB`: joelho e pé da perna k por quadro 0–7 |
| pernas_reto.joelho_y/pe_y             | 8:8894 + 16k / 889c + 16k | `_L{k}yA`/`_L{k}yB`                                  |
| pernas_diagonal.inicio_x/y            | 8:88d4 / 88d8  | `_Dx`/`_Dy` (direção 1)                                         |
| pernas_diagonal.joelho_x/joelho_y     | 8:88dc + 16k / 88e4 + 16k | `_D{k}xA`/`_D{k}yA`                                  |
| pernas_diagonal.pe_x/pe_y             | 8:891c + 16k / 8924 + 16k | `_D{k}xB`/`_D{k}yB`                                  |
| palpos_reto.joelho_x/y, ponta_x/y     | 8:882c / 8830 / 8834 / 8838 | `_PxT`/`_PyT`/`_PxT2`/`_PyT2`, índice = `S1(4)`     |
| palpos_diagonal.joelho_x/y, ponta_x/y | 8:883c / 8840 / 8844 / 8848 | `_PxD`/`_PyD`/`_PxD2`/`_PyD2`                       |

Os valores que não são tabelas (ids dos sprites, onde os palpos saem do corpo, cores) são imediatos
do pseudocódigo de `_DrawSpider` (`1010:1d6a`), `_DrawPalps` (`1010:2594`) e `_DrawLegs` (`1010:2be0`),
anotados em "_origem".
"""
import argparse
import json
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from ne_info import parse_ne  # noqa: E402

SEG8 = 8


def ler(data, segs, off, n):
    s = segs[SEG8 - 1]
    o = s["file_offset"] + off
    return [x - 256 if x > 127 else x for x in data[o:o + n]]


def extrair(data):
    segs = parse_ne(data)["segments"]

    def pernas(inicio_x, inicio_y, joelho_x, joelho_y, pe_x, pe_y):
        return {
            "inicio_x": ler(data, segs, inicio_x, 4),
            "inicio_y": ler(data, segs, inicio_y, 4),
            "joelho_x": [ler(data, segs, joelho_x + 16 * k, 8) for k in range(4)],
            "joelho_y": [ler(data, segs, joelho_y + 16 * k, 8) for k in range(4)],
            "pe_x": [ler(data, segs, pe_x + 16 * k, 8) for k in range(4)],
            "pe_y": [ler(data, segs, pe_y + 16 * k, 8) for k in range(4)],
        }

    def palpos(inicio, base):
        return {
            "inicio": inicio,
            "joelho_x": ler(data, segs, base, 4),
            "joelho_y": ler(data, segs, base + 4, 4),
            "ponta_x": ler(data, segs, base + 8, 4),
            "ponta_y": ler(data, segs, base + 12, 4),
        }

    return {
        "corpo_x": ler(data, segs, 0x8804, 8),
        "corpo_y": ler(data, segs, 0x880C, 8),
        "agonia_x": ler(data, segs, 0x8814, 4),
        "agonia_y": ler(data, segs, 0x8818, 4),
        "pernas_reto": pernas(0x884C, 0x8850, 0x8854, 0x8894, 0x885C, 0x889C),
        "pernas_diagonal": pernas(0x88D4, 0x88D8, 0x88DC, 0x88E4, 0x891C, 0x8924),
        "palpos_reto": palpos([4, -10], 0x882C),
        "palpos_diagonal": palpos([10, -3], 0x883C),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("exe")
    ap.add_argument("--json")
    a = ap.parse_args()
    t = extrair(open(a.exe, "rb").read())
    if not a.json:
        for k, v in t.items():
            print(k, v)
        return
    saida = {
        "_comentario": (
            "Desenho da aranha na janela de edição (_DrawSpider 1010:1d6a, _DrawLegs 1010:2be0, _DrawPalps "
            "1010:2594). Tabelas extraídas do SIMANTW.EXE por tools/extrair_desenho_aranha.py (endereços no "
            "topo do script); o resto são imediatos do pseudocódigo (_origem). Coordenadas em pixels, relativas "
            "ao centro da aranha (_SpidX, _SpidY), para a direção 0 (pernas_reto, palpos_reto) ou 1 "
            "(pernas_diagonal, palpos_diagonal); as outras direções são giros e espelhos dessas "
            "(docs/spec/telas/jogo.md, 'Aranha na janela de edição'). Cores em índices da paleta."
        ),
        "sprite_corpo": 1000,
        "sprite_agonia": 1050,
        "cor_transparente": 13,
        "cor_pernas": 0,
        "cor_palpos": 0,
        "cor_palpos_jogador": 11,
        **t,
        "_origem": {
            "sprite_corpo": "_DrawSpider: _db_LoadObject(_SpidDir + 1000, 2) fora da agonia",
            "sprite_agonia": "_DrawSpider: _db_LoadObject(_Scycle + 0x41a, 2) com _SMode 5; sem pernas nem palpos",
            "cor_transparente": "_CopyMaskBitmap2 1030:b110: o nibble 0xD não é copiado",
            "cor_pernas": "_DrawLegs: _ConvColor(0xF) (preto do Mac; telas/mapa.md, _PatchColorArrays)",
            "cor_palpos": "_DrawPalps: _ConvColor(0xF) com _MeMode != 1",
            "cor_palpos_jogador": "_DrawPalps: _ConvColor(1) com _MeMode == 1 (amarelo do Mac)",
            "palpos_reto.inicio": "_DrawPalps direção 0: os palpos saem de (+-4, -10)",
            "palpos_diagonal.inicio": "_DrawPalps direção 1: saem de (10, -3) e (3, -10)",
        },
    }
    with open(a.json, "w", encoding="utf-8") as f:
        json.dump(saida, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(f"gravado {a.json}")


if __name__ == "__main__":
    main()
