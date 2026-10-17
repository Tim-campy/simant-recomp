"""Gera data/quintal_mapa.json: geometria, cores e sprites da janela do quintal (janela 25) e da faixa 35.

Uso:
    python tools/extrair_quintal_mapa.py SIMANTW.EXE [--json data/quintal_mapa.json]

Sem --json, só imprime. Especificação: docs/spec/telas/quintal.md.

Tabelas lidas do DGROUP (segmento 10, seletor 1048) do executável, sem relocação:

| Chave no JSON                   | Endereço        | Uso                                                        |
|---------------------------------|-----------------|------------------------------------------------------------|
| poligono_cursor                 | 1048:1894 (16)  | `_patchRgn`: 4 vértices (x, y) em palavras; cursor/clique  |
| poligono_colonia                | 1048:18a4 (16)  | `_patchRgn2`: 4 vértices; cor das colônias (modo 2)        |
| sprites.menino.deslocamentos    | 1048:18d4…190b  | bytes com sinal por quadro: 0–11 (18d4/18e0), 20–23 (18d8+q/18dc+q), 100–111 (1890+q/189c+q) |
| sprites.cachorro.deslocamentos  | 1048:1918…1937  | 0–11 (1918/1924), 100–103 (18cc+q/18d0+q)                  |
| sprites.gato.deslocamentos      | 1048:1938…1953  | 10–19 (192e+q/1932+q), 20–29 (192c+q/1936+q)               |

As demais chaves são valores medidos nas capturas da bancada QEMU (re_trabalho/modos/*.png) ou imediatos vistos
no pseudocódigo (endereço indicado no "_origem" de cada uma).
"""
import argparse
import json
import struct
import sys

sys.path.insert(0, __file__.rsplit("tools", 1)[0] + "tools")
from ne_info import parse_ne  # noqa: E402

DGROUP = 10


def extrair(exe):
    s = parse_ne(exe)["segments"][DGROUP - 1]

    def b(o):
        v = exe[s["file_offset"] + o]
        return v - 256 if v > 127 else v

    def pal(o, n):
        return list(struct.unpack_from(f"<{n}h", exe, s["file_offset"] + o))

    def vertices(o):
        v = pal(o, 8)
        return [[v[i], v[i + 1]] for i in range(0, 8, 2)]

    def desl(faixas):
        out = {}
        for (q0, q1, bx, by) in faixas:
            for q in range(q0, q1):
                out[str(q)] = [b(bx + q), b(by + q)]
        return out

    return {
        "_comentario": "Janela do quintal (25) e faixa 35. Gerado por tools/extrair_quintal_mapa.py; ver docs/spec/telas/quintal.md. "
                       "Coordenadas: 'externo' e 'cliente' na área cliente da janela principal da tela de jogo (tela 640x480 menos 38 linhas); "
                       "posições de bitmaps e de patches relativas ao canto da área cliente da janela 25.",
        "janela": 25,
        "faixa": 35,
        "_origem_externo": "medido em re_trabalho/modos/full_yard.png: contorno de (119,137) a (631,413) na tela",
        "externo": [119, 99, 513, 277],
        "altura_titulo": 18,
        "cor_linha": 0,
        "cor_titulo": 4,
        "cor_botao": 8,
        "cor_fundo": 7,
        "titulos": ["Yard View", "House View", "Strategic View", "Graphic View"],
        "_origem_titulos": "lista de textos 4/1000 do SHARED, itens 9..12 (_SetMapTitle 1000:953c); o port ainda não desenha o título",
        "origem_patches": [0, 0],
        "_origem_origem_patches": "_mapTileRect = canto da área cliente; medido: cursor do patch (11,8) em (517,304) na tela",
        "patch": {"colunas": 12, "linhas": 16, "passo_x": 28, "passo_y": 10, "inclinacao": -10},
        "poligono_cursor": vertices(0x1894),
        "poligono_colonia": vertices(0x18A4),
        "ajuste_cursor_modos_0_1": -1,
        "modos": [
            {"bitmaps": [{"id": 7000, "x": 0, "y": 2}, {"id": 7002, "x": 278, "y": 2}], "conteudo": "bichos", "mostra_menino_dentro": False},
            {"bitmaps": [{"id": 7001, "x": 0, "y": 2}, {"id": 7002, "x": 278, "y": 2}], "conteudo": "bichos", "mostra_menino_dentro": True},
            {"bitmaps": [{"id": 7020, "x": 0, "y": 66}], "conteudo": "colonias", "mostra_menino_dentro": False},
            {"bitmaps": [{"id": 7020, "x": 0, "y": 66}], "conteudo": "barras", "mostra_menino_dentro": False},
        ],
        "_origem_modos": "_SetYardMode 1000:9f94 (buffer) e _win_DrawYardWindow 1010:a9e0; posições medidas (7000 em (120,159), 7002 em (398,159), 7020 em (120,223))",
        "rodape": {"id": 7010, "x": -1, "y": 227},
        "cores_colonia": {"preta": 0, "ambas": 8, "vermelha": 9},
        "_origem_cores_colonia": "_DrawSimColonies 1010:bea4 (tabelas 1048:1982/198a, cores convertidas em tempo de execução); medidas em yard_strat2.png",
        "barras": {"x_preta": 168, "x_vermelha": 180, "largura": 9, "base_y": 71, "cor_preta": 0, "cor_vermelha": 9},
        "_origem_barras": "_DrawColonyBars 1010:bf9c (0xa8, +0xc..+0x15, 0x47); altura (pop + 3) >> 2",
        "cursor": [
            {"contorno": 13, "preenchimento": None},
            {"contorno": 13, "preenchimento": None},
            {"contorno": 7, "preenchimento": 8},
            {"contorno": 7, "preenchimento": 8},
        ],
        "_origem_cursor": "medido: modos 0/1 contorno magenta (full_yard.png); modos 2/3 cinza claro com contorno cinza escuro (yard_strat.png)",
        "transparente": 13,
        "sprites": {
            "menino": {"base": 8000, "deslocamentos": desl([(0, 12, 0x18D4, 0x18E0), (20, 24, 0x18D8, 0x18DC), (100, 112, 0x1890, 0x189C)])},
            "cachorro": {"base": 8500, "deslocamentos": desl([(0, 12, 0x1918, 0x1924), (100, 104, 0x18CC, 0x18D0)])},
            "gato": {"base": 1300, "deslocamentos": desl([(10, 20, 0x192E, 0x1932), (20, 30, 0x192C, 0x1936)])},
            "passaro": {"base": 1250, "deslocamentos": {"0": [-10, -3], "1": [-9, -1]}},
            "cortador": {"base": 8800, "parado": [98, 181], "direita": [19, 15], "esquerda": [-15, 15]},
            "placa": {"id": 1260, "x": 170, "y": 186},
            "chuva": {"id": 7005, "gotas": 14, "x_min": 50, "x_faixa": 400, "y_faixa": 150},
        },
        "_origem_sprites": "_Draw_SimYard 1010:b462, _DrawSimKid 1010:af14, _DrawDog 1010:b110, _DrawSimCat 1010:b200, _DrawSimBird 1010:b196, _DrawMower, _DrawRain 1010:ba9e",
        "botao_abrir": "ribToYard",
        "botoes_faixa": {
            "ribPatch": "fechar",
            "ribHouse": "alternar_casa",
            "ribStrat": "modo_estrategico",
            "ribGraphic": "modo_grafico",
            "ribPlace": "colocar_rainha",
            "ribAuto": "rainhas_automatico",
            "ribManual": "rainhas_manual",
        },
        "botao_do_modo": ["ribHouse", "ribHouse", "ribStrat", "ribGraphic"],
        "botoes_rainhas": ["ribAuto", "ribManual"],
        "teclas": [
            {"tecla": "Y", "ctrl": True, "acao": "modo_quintal"},
            {"tecla": "5", "ctrl": True, "acao": "modo_casa"},
            {"tecla": "6", "ctrl": True, "acao": "modo_estrategico"},
            {"tecla": "7", "ctrl": True, "acao": "modo_grafico"},
            {"tecla": "M", "ctrl": True, "acao": "abrir"},
        ],
        "dados_faixa": {"fonte": 0, "rainhas": "ribQueens", "colonias_pretas": "ribBlackCols", "colonias_vermelhas": "ribRedCols"},
        "_origem_dados_faixa": "_DrawYardData 1010:acd8: _font_SetFont(3) [a lista de fontes do port numera diferente: medido, o índice 0 (Gaia8x14) bate 98,5% em full_yard.png], _QueenStorageB em 0x231a, _ColonyTotalBlack/Red em 0x231d/0x231e",
        "_origem_teclas": "ACCELERATORTABLE do SIMANTW.EXE: Ctrl+Y/5/6/7 = comandos 0xFD21-0xFD24 (_ProcMenu: modo 0-3 e abre o quintal), Ctrl+M = 0xFD12 (_OpenMapYard)",
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("exe")
    ap.add_argument("--json")
    a = ap.parse_args()
    d = extrair(open(a.exe, "rb").read())
    texto = json.dumps(d, ensure_ascii=False, indent=1)
    if a.json:
        open(a.json, "w", encoding="utf-8", newline="\n").write(texto + "\n")
        print(f"gravado {a.json}")
    else:
        print(texto)


if __name__ == "__main__":
    main()
