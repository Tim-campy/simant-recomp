"""Cenários de vetores de tick para os ramos raros ainda sem evidência (docs/estado-do-projeto.md §3,
rodada de 2026-10-19): morte natural por casta e da rainha, proporção de vitórias nas lutas, revoada
natural, N-02, N-04, N-05, N-08, N-11 e afogamento de adultas, lista A cheia em _ExitHole.

Mesma interface de tools/cenarios_regras.py (e de cenarios_tick.py):

    set SIMANT_RAIZ=...
    python tools/cenarios_raros.py gravar CENARIO DIR_BRUTO --overlay OV [--snapshot rico] [--ticks N]
    python tools/cenarios_raros.py eventos DIR_BRUTO
    python tools/cenarios_raros.py lutas DIR_BRUTO...         estatística das lutas gravadas (por classe)
    python tools/cenarios_raros.py selecionar_lote LOTE.json DIR_BRUTO_RAIZ

A preparação escreve a memória do original na entrada de um tick; o resto é do jogo.
"""
import json
import sys
import time
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cenarios_regras as cr  # noqa: E402  (aplica os ajustes de Sessao de cenarios_regras)
import cenarios_tick as ct  # noqa: E402
import qemu_bancada as qb  # noqa: E402
from ant_editar import Save  # noqa: E402
from cenarios_regras import (D8, acrescentar_formiga, mapa_escrever, mover_formiga, vida_escrever,  # noqa: E402
                             vida_ler)
from cenarios_tick import Sessao, formigas  # noqa: E402

ct.OLHAR = ct.OLHAR + ("RAntsExpired", "BlkQueens", "RedQueens", "FlyAwayR", "FlyAwayB", "WaterTable",
                       "RedLost", "BlackLost", "TilesDugB", "ListIndexA", "HealthR", "FoodR")

GRUPO = lambda t: (t & 0x78) >> 3  # noqa: E731


def adultas(s, lista, cor, grupos=(2, 3, 5, 6, 7, 9)):
    """Vagas vivas da lista com a cor (0 preta, 0x80 vermelha) e grupo em `grupos`."""
    return [(i, f) for i, f in formigas(s, lista) if f["T"] & 0x80 == cor and GRUPO(f["T"]) in grupos]


def trocar_casta(c: Sessao, s, lista, i, grupo, m=None):
    f = s.formigas(lista)[i]
    t = (f["T"] & 0x87) | (grupo << 3)
    mover_formiga(c, s, lista, i, f["X"], f["Y"], m=m, t=t)
    c.log(f"{lista}[{i}] em ({f['X']}, {f['Y']}) T {f['T']:#x} -> {t:#x}" + (f" M {m}" if m is not None else ""))


# ------------------------------------------------------------------------------------------------
# Morte natural (superficie.md 6.5, ninho.md 3.1, ciclo-de-vida.md 5.2)
# ------------------------------------------------------------------------------------------------

def cenario_morte_natural(c: Sessao, saida, n):
    """Saúde 1 nas duas colônias (_AlwaysHealthy = 1 segura a preta; a rainha não morre de fome com
    saúde > 0): cada vaga da lista A e cada dona dos ninhos morre com Sk(256) = 0 e Sk(32) > 1. Para
    haver todas as castas, pretas e vermelhas da superfície viram soldados (grupo 6) e machos (grupo 4,
    M = 14), e adultas do ninho vermelho, soldados e machos."""
    c.entrada()
    s = c.save()
    for cor in (0, 0x80):
        cand = adultas(s, "A", cor, grupos=(2, 3))
        for k, (i, _) in enumerate(cand[:8]):
            if k % 2 == 0:
                trocar_casta(c, s, "A", i, 6)
            else:
                trocar_casta(c, s, "A", i, 4, m=14)
    cand = adultas(s, "R", 0x80, grupos=(2,))
    for k, (i, _) in enumerate(cand[:6]):
        trocar_casta(c, s, "R", i, 6 if k % 2 == 0 else 4, m=None if k % 2 == 0 else 14)
    for nome, v in (("AlwaysHealthy", 1), ("HealthB", 1), ("HealthR", 1)):
        c.escrever(nome, v)
    return c.gravar(saida, n)


def cenario_rainha_fome(c: Sessao, saida, n):
    """N-02 e a morte da rainha preta: _HealthB = 0, _AlwaysHealthy = 0. A cabeça morre de fome com
    Sk(64) = 0 (diálogo 10015); até lá, a rainha sorteia Sk(256) como as outras e nunca morre por 3.1.
    Grava até a cabeça sumir (_BlkQueens cai no tick seguinte, pela cauda)."""
    c.entrada()
    c.escrever("AlwaysHealthy", 0)
    c.escrever("HealthB", 0)
    nomes = c.gravar(saida, n, parar=lambda d: d.ler("BlkQueens") == 0)
    return nomes


def cenario_rainha_imune(c: Sessao, saida, n):
    """N-02: como rainha_fome, com a semente S da entrada do 1º tick escolhida (busca no port sobre o
    vetor tick_001711 de rainha_fome) para que a cabeça da rainha preta sorteie Sk(256) = 0 no 3.1 com
    _HealthB = 0: qualquer outra dona morreria (Sk(32) > 0), a rainha segue."""
    c.entrada()
    c.escrever("AlwaysHealthy", 0)
    c.escrever("HealthB", 0)
    c.escrever("SementeS", 0x00A5)
    return c.gravar(saida, n)


def cenario_rainha_vermelha_fome(c: Sessao, saida, n):
    """A rainha vermelha morre de fome: _HealthR = 0 (diálogo 10016). As vermelhas comem e a saúde
    sobe, então _HealthR volta a 0 na entrada de cada tick (antes do estado de antes)."""
    nomes = []
    for _ in range(n):
        c.entrada()
        c.escrever("HealthR", 0)
        nomes += c.gravar(saida, 1)
        if c.ler("RedQueens") == 0:
            break
    return nomes


# ------------------------------------------------------------------------------------------------
# Lutas e proporção de vitórias (superficie.md 6.1–6.3)
# ------------------------------------------------------------------------------------------------

def tabuleiro(c: Sessao, grupo_preta, grupo_vermelha, raio=5):
    """Tabuleiro de pretas e vermelhas num lugar aberto da superfície (casas alternadas), cada uma
    virada para uma vizinha da outra cor, em modo 0 (Bored): quase todo passo cai numa inimiga e
    começa uma luta. As vagas novas vão no fim da lista A. `grupo_*` = None mistura castas pela linha
    (operária, soldado, macho, rainha nova)."""
    c.entrada()
    cr.esperar_compactacao(c)
    s = c.save()
    vida_a = vida_ler(c, "A")
    x0, y0 = cr.lugar_aberto(s, vida_a, raio=raio)
    mistura = (2, 6, 4, 8, 2, 6, 2, 6, 4, 2)
    k = 0
    for dy in range(-raio, raio + 1):
        for dx in range(-raio, raio + 1):
            x, y = x0 + dx, y0 + dy
            if s.celula("MapA", x, y) >= 0x10:
                continue
            preta = (dx + dy) % 2 == 0
            g = grupo_preta if preta else grupo_vermelha
            if g is None:
                g = mistura[(dy + raio + (0 if preta else 3)) % len(mistura)]
            d = (2, 6, 4, 0)[k % 4]
            t = (0 if preta else 0x80) | g << 3 | d
            acrescentar_formiga(c, "A", x, y, t, 0)
            k += 1
    c.log(f"{k} formigas no tabuleiro centrado em ({x0}, {y0}): pretas grupo {grupo_preta}, vermelhas {grupo_vermelha}")
    c.escrever("Invincible", 0)


def cenario_lutas(c: Sessao, saida, n):
    """Tabuleiro com as castas misturadas (cobre a matriz de _GetWinner)."""
    tabuleiro(c, None, None, raio=4)
    return c.gravar(saida, n)


def cenario_lutas_oper(c: Sessao, saida, n):
    """Operárias pretas contra operárias vermelhas (classe 0 x 0: 5 em 10)."""
    tabuleiro(c, 2, 2)
    return c.gravar(saida, n)


def cenario_lutas_soldado(c: Sessao, saida, n):
    """Soldados pretos contra operárias vermelhas (classe 1 x 0: o soldado vence 8 em 10, de qualquer
    lado: a matriz é complementar, M[i][j] + M[j][i] = 10)."""
    tabuleiro(c, 6, 2)
    return c.gravar(saida, n)


def cenario_lutas_macho(c: Sessao, saida, n):
    """Machos pretos contra soldados vermelhos (classe 2 x 1: o macho vence 1 em 10)."""
    tabuleiro(c, 4, 6)
    return c.gravar(saida, n)


def contadores_de_luta(brutos):
    """Soma, nos ticks gravados, as lutas decididas por _GetWinner pelos contadores do save: _RedLost
    (a preta venceu) e _BlackLost (a vermelha venceu)."""
    for bruto in brutos:
        rl = bl = 0
        ds = sorted(Path(bruto).glob("tick_*"))
        for d in ds:
            a, b = Save.abrir(d / "estado_antes.bin"), Save.abrir(d / "estado_depois.bin")
            rl += b.ler("RedLost") - a.ler("RedLost")
            bl += b.ler("BlackLost") - a.ler("BlackLost")
        print(f"{bruto}: {len(ds)} ticks, {rl + bl} lutas, pretas venceram {rl} ({rl / max(1, rl + bl):.2f}), "
              f"vermelhas {bl}")


CLASSE = (0, 0, 0, 0, 2, 0, 1, 1, 2, 1, 0, 0, 3, 3, 0, 0)
MATRIZ = ((5, 2, 7, 3), (8, 5, 9, 4), (3, 1, 5, 2), (7, 6, 8, 5))


def lutas(brutos):
    """Conta as lutas começadas na lista A (vaga que vira nuvem 0x70/0xF0 + q com M = 10 e S = T do
    vencedor; a atacante é a vaga zerada no mesmo tick, de T igual a S quando ela vence) por par de
    classes e compara com a matriz de _GetWinner."""
    tot = Counter()
    venceu_a = Counter()
    for bruto in brutos:
        for d in sorted(Path(bruto).glob("tick_*")):
            a, b = Save.abrir(d / "estado_antes.bin"), Save.abrir(d / "estado_depois.bin")
            fa, fb = a.formigas("A"), b.formigas("A")
            for j, (x, y) in enumerate(zip(fa, fb)):
                if x["T"] == 0 or y["M"] != 10 or x["M"] == 10 or (y["T"] & 0x78) != 0x70:
                    continue
                if x["X"] != y["X"] or x["Y"] != y["Y"]:
                    continue
                ta, v = x["T"], y["S"]
                # a atacante: vaga zerada neste tick, vizinha, de cor oposta à "a"
                cands = [fa[i]["T"] for i in range(len(fa)) if fa[i]["T"] and not fb[i]["T"]
                         and (fa[i]["T"] ^ ta) & 0x80 and max(abs(fa[i]["X"] - x["X"]), abs(fa[i]["Y"] - x["Y"])) == 1]
                if v == ta:
                    tb = next((t for t in cands), None)
                    ganhou_a = True
                else:
                    tb = v
                    ganhou_a = False
                if tb is None:
                    continue
                par = (CLASSE[GRUPO(ta)], CLASSE[GRUPO(tb)])
                tot[par] += 1
                venceu_a[par] += ganhou_a
    print("classe a x b  lutas  'a' venceu  observado  matriz")
    for par in sorted(tot):
        print(f"  {par[0]} x {par[1]}      {tot[par]:4d}  {venceu_a[par]:4d}       {venceu_a[par] / tot[par]:.2f}       "
              f"{MATRIZ[par[0]][par[1]] / 10:.1f}")
    print(f"total: {sum(tot.values())} lutas")
    return tot, venceu_a


# ------------------------------------------------------------------------------------------------
# Revoada natural (superficie.md 4, ciclo-de-vida.md 3.2)
# ------------------------------------------------------------------------------------------------

def cenario_revoada_natural(c: Sessao, saida, n):
    """Sem forçar castas: o jogo roda livre em ultra (Shift+4) até a colônia vermelha ter, ela mesma,
    mais de 20 machos + rainhas novas e _GstrR armar _FlyAwayR (consulta a cada 0,5 s); a saúde das duas colônias é reposta a 100
    a cada consulta (só para a rainha preta não morrer de fome e o jogo não acabar, o que é registrado).
    Volta ao normal (Shift+2) e grava em modo tick alguns ticks do pré-voo (F > 100) e os últimos até F = 0."""
    c.teclas("shift+4")
    inicio = time.time()
    ultimo = 0
    while True:
        time.sleep(0.5)
        fa = c.ler("FlyAwayR")
        if fa > 0 or time.time() - inicio > 1800:
            break
        if time.time() - ultimo > 5:
            ultimo = time.time()
            cp = c.mj.ler_variavel("CastePopR")
            rep = int.from_bytes(cp[6:8], "little") + int.from_bytes(cp[8:10], "little")
            c.b.log(f"GameTime {c.ler('GameTime')}: reprodutores vermelhos {rep}, {time.time() - inicio:.0f} s")
            for nome in ("HealthB", "HealthR"):
                if c.ler(nome) < 60:
                    c.escrever(nome, 100)
    c.teclas("shift+2")
    c.entrada()
    cp = c.mj.ler_variavel("CastePopR")
    c.log(f"livre em ultra até GameTime {c.ler('GameTime')}: FlyAwayR {c.ler('FlyAwayR')}, _CastePopR {cp.hex()}")
    nomes = []
    nomes += c.gravar(saida, 3)
    while c.ler("FlyAwayR") > 102:
        c.avancar(1)
    nomes += c.gravar(saida, 4)
    while c.ler("FlyAwayR") > 8:
        c.avancar(1)
    nomes += c.gravar(saida, 10)
    return nomes


# ------------------------------------------------------------------------------------------------
# Ninho: N-04, N-05, N-08, N-11 e afogamento (ninho.md 5.1, 5.4, 7.1, 8.1, 8.3)
# ------------------------------------------------------------------------------------------------

def topo_do_tunel(s, mapa, vida, lista_vida=None):
    """Célula de túnel livre mais rasa (y >= 2) com vizinha de túnel embaixo."""
    for y in range(2, 30):
        for x in range(1, 63):
            if s.celula(mapa, x, y) < 0x10 and vida[x * 64 + y] == 0 and s.celula(mapa, x, y + 1) < 0x10:
                return x, y
    raise RuntimeError("sem túnel")


def cenario_n04(c: Sessao, saida, n):
    """N-04: operária preta carregando ovo (T 0x0A, M 1, S 1) no alto do túnel preto: desce pelo mapa
    de saída até um mínimo, marca o bit 3 de S, e larga o ovo T = 1 no tick seguinte."""
    c.entrada()
    s = c.save()
    x, y = topo_do_tunel(s, "MapB", vida_ler(c, "B"))
    acrescentar_formiga(c, "B", x, y, 0x0A, 1, 1)
    c.escrever("HealthB", 100)
    return c.gravar(saida, n)


def cenario_n05(c: Sessao, saida, n):
    """N-05: vermelhas carregando ovo (T 0x8A, M 1) no túnel vermelho: largam com chance 1/4 por tick
    numa célula sem comida, sempre T = 0x82."""
    c.entrada()
    s = c.save()
    vida_r = vida_ler(c, "R")
    x, y = topo_do_tunel(s, "MapR", vida_r)
    acrescentar_formiga(c, "R", x, y, 0x8A, 1, 0)
    x2, y2 = x, y + 1
    acrescentar_formiga(c, "R", x2, y2, 0x8C, 1, 0)
    c.escrever("HealthR", 100)
    return c.gravar(saida, n)


def cenario_n08(c: Sessao, saida, n):
    """N-08: operárias pretas cavando (T 0x12, M 4) no fundo do túnel preto e uma vermelha (T 0x92, M 4)
    no vermelho: cavam (T + 0x18, M 5, som), sobem com a terra e voltam a cavar."""
    c.entrada()
    s = c.save()
    for lista, mapa, t in (("B", "MapB", 0x12), ("R", "MapR", 0x96)):
        vida = vida_ler(c, lista)
        fundo = [(y, x) for x in range(1, 63) for y in range(2, 62)
                 if s.celula(mapa, x, y) < 0x10 and vida[x * 64 + y] == 0 and 0x20 <= s.celula(mapa, x, y + 1) <= 0x2E]
        y, x = max(fundo)
        acrescentar_formiga(c, lista, x, y, t, 4, 0)
        if lista == "B":
            y2, x2 = sorted(fundo)[len(fundo) // 2]
            acrescentar_formiga(c, lista, x2, y2, 0x34, 4, 0)
    c.escrever("HealthB", 100)
    c.escrever("HealthR", 100)
    return c.gravar(saida, n)


def inundar_linhas(c: Sessao, w):
    """As linhas w..63 dos dois ninhos como _DoWater as deixaria (túnel e comida < 0x20 → 0x4E, terra
    + 0x2F); donas e invasoras de grupo 1..11 nessas linhas em M = 17 (_DrownBList/_DrownRList)."""
    for mapa in ("MapB", "MapR"):
        m = bytearray(c.mj.ler_variavel(mapa))
        for x in range(64):
            for y in range(w, 64):
                v = m[x * 64 + y]
                m[x * 64 + y] = 0x4E if v < 0x20 else (v + 0x2F) & 0xFF
        seg, off, _ = c.mj.resolver(mapa)
        for k in range(0, 4096, 512):
            c.mj.escrever_variavel(f"{seg}:{off + k:x}", bytes(m[k:k + 512]), 512)
        c.log(f"{mapa}: linhas {w}..63 inundadas")
    s = c.save()
    for lista in ("B", "R"):
        for i, f in formigas(s, lista):
            if f["Y"] >= w and 1 <= GRUPO(f["T"]) <= 11:
                c.escrever_elemento(f"{lista}listM", i, 17)
                c.log(f"{lista}[{i}] em ({f['X']}, {f['Y']}) M <- 17")


def cenario_n11(c: Sessao, saida, n):
    """N-11 e afogamento (8.3): antes de inundar, comida (0x11) em duas células de túnel do ninho preto
    e adultas (operárias e soldados, pretas e vermelhas) em túneis das linhas que vão inundar; linhas
    w..63 inundadas como _DoWater faria, sem chuva (_RainOn = 0): a água desce 1 linha com chance 1/10
    por tick seco (0x4E → S(8): a comida some, _FoodB igual) e as afogadas (M = 17) giram e morrem com
    chance 1/100 por tick."""
    c.entrada()
    s = c.save()
    w = 6
    postas = []
    for lista, mapa, ts in (("B", "MapB", (0x12, 0x32, 0x14, 0x36)), ("R", "MapR", (0x92, 0xB2, 0x94, 0xB6))):
        vida = vida_ler(c, lista)
        cels = [(x, y) for y in range(w, 40) for x in range(1, 63)
                if s.celula(mapa, x, y) < 0x10 and vida[x * 64 + y] == 0]
        for k, t in enumerate(ts * 3):
            if k >= len(cels):
                break
            x, y = cels[(k * 7) % len(cels)]
            if (x, y) in postas:
                continue
            postas.append((x, y))
            acrescentar_formiga(c, lista, x, y, t, 13, 0)
        if lista == "B":
            for x, y in cels[-2:]:
                mapa_escrever(c, mapa, x, y, 0x11)
    inundar_linhas(c, w)
    for nome, v in (("WaterTable", w), ("RainOn", 0), ("ClearRainFlag", 0), ("HealthB", 100), ("HealthR", 100)):
        c.escrever(nome, v)
    return c.gravar(saida, n)


# ------------------------------------------------------------------------------------------------
# Lista A cheia em _ExitHole (ninho.md 4.6, "Lista A cheia")
# ------------------------------------------------------------------------------------------------

def saidora(c: Sessao, s):
    """Uma operária preta (T 0x12, M 2) na linha 1 do túnel preto, logo abaixo de uma abertura (0x18
    na linha 0): sai pelo buraco no tick seguinte (N-03)."""
    vida = vida_ler(c, "B")
    for x in range(1, 63):
        if s.celula("MapB", x, 0) == 0x18 and s.celula("MapB", x, 1) < 0x10 and vida[x * 64 + 1] == 0:
            return acrescentar_formiga(c, "B", x, 1, 0x10, 2, 0), (x, 1)
    raise RuntimeError("sem abertura")


def encher_lista_a(c: Sessao, s, com_vagas):
    """_ListIndexA = 1000. Com `com_vagas`, as vagas de _ListIndexA..999 ficam vazias (T = 0) e há
    buracos para a compactação; sem, elas recebem formigas paradas (T 0x10 | d, M 17) numa célula
    livre longe de tudo (só a última fica no mapa de vida)."""
    n0 = c.ler("ListIndexA")
    if not com_vagas:
        vida_a = vida_ler(c, "A")
        x, y = cr.lugar_aberto(s, vida_a, raio=1, perto=(100, 50))
        for nome, v in (("AlistX", x), ("AlistY", y), ("AlistT", 0x12), ("AlistM", 17), ("AlistS", 0)):
            seg, off, _ = c.mj.resolver(nome)
            dados = bytes([v]) * (1000 - n0)
            for k in range(0, len(dados), 256):
                c.mj.escrever_variavel(f"{seg}:{off + n0 + k:x}", dados[k:k + 256], len(dados[k:k + 256]))
        vida_escrever(c, "A", x, y, 0x12)
        # as vagas vazias do meio também: nenhuma vaga com T = 0
        for i, f in enumerate(s.formigas("A")[:n0]):
            if f["T"] == 0:
                for nome, v in (("AlistX", x), ("AlistY", y), ("AlistT", 0x12), ("AlistM", 17), ("AlistS", 0)):
                    c.escrever_elemento(nome, i, v)
        c.log(f"vagas {n0}..999 e as vazias = operárias paradas em ({x}, {y})")
    c.escrever("ListIndexA", 1000)


def cenario_lista_a_cheia(c: Sessao, saida, n):
    """_ExitHole com _ListIndexA = 1000 e buracos na lista: a formiga que sai vai para a folga da vaga
    1000 e se perde; a compactação abre vagas e o contador ainda soma 1."""
    c.entrada()
    cr.esperar_compactacao(c)
    s = c.save()
    encher_lista_a(c, s, com_vagas=True)
    i, pos = saidora(c, s)
    c.log(f"B[{i}] sai por {pos}")
    return c.gravar(saida, n, parar=lambda d: d.formigas("B")[i]["T"] == 0)


def cenario_lista_a_cheia_sem_vagas(c: Sessao, saida, n):
    """Como lista_a_cheia, mas as 1000 vagas ocupadas: a compactação não abre nada e a formiga se perde."""
    c.entrada()
    cr.esperar_compactacao(c)
    s = c.save()
    encher_lista_a(c, s, com_vagas=False)
    i, pos = saidora(c, s)
    c.log(f"B[{i}] sai por {pos}")
    return c.gravar(saida, n, parar=lambda d: d.formigas("B")[i]["T"] == 0)


# ------------------------------------------------------------------------------------------------
# _KillAntLion(-1) pelo inseticida do Experimental (inimigos.md §1.4, telas/experimental.md §4.6)
# ------------------------------------------------------------------------------------------------

def cenario_exp_leao_m1(c: Sessao, saida, n):
    """Experimental pausado (snapshot exp), superfície: um tile de formiga-leão (0x38) sem entrada na
    lista numa célula livre à vista e duas formigas-leão de verdade na lista (se não houver, são
    escritas); inseticida em jato (ferramenta 6, subopção 1: 8 jatos na própria célula, sem sorteio)
    nessa célula. Cada jato chama _KillAntLion(_FindInLionList) = _KillAntLion(-1). Grava a ação como
    em cenarios_tick (exp_ctrl2) e a tela."""
    c.teclas("shift+0")
    time.sleep(1)
    c.mj.sessao_gdb()
    s = c.save()
    mx, my = s.ler("MapPnt", 0, 2), s.ler("MapPnt", 1, 2)
    vida_a = vida_ler(c, "A")
    li = c.ler("LionIndex")
    c.log(f"LionIndex {li}; MapPnt ({mx}, {my})")
    # células livres à vista (a vista tem 25 x 19 tiles)
    livres = [(x, y) for y in range(my + 2, my + 17) for x in range(mx + 2, mx + 23)
              if s.celula("MapA", x, y) < 0x10 and vida_a[x * 64 + y] == 0]
    alvo = livres[len(livres) // 2]
    if li < 2:
        for k in range(2):
            lx, ly = livres[k * 7]
            for nome, v in (("LionListX", lx), ("LionListY", ly), ("LionListT", 0), ("LionListM", 0),
                            ("LionListS", 0)):
                c.escrever_elemento(nome, k, v)
            mapa_escrever(c, "MapA", lx, ly, 0x38)
        c.escrever("LionIndex", 2)
    mapa_escrever(c, "MapA", alvo[0], alvo[1], 0x38)
    for nome, tam in (("BoyMsgCnt", 4), ("LastThemeSong", 2)):
        c.log(f"{nome} antes = {c.mj.ler_variavel(nome, tam).hex()}")
    passos = [{"f": 6, "sub": 1, "pontos": [list(alvo)], "ctrl": False}]
    antes = ct.fotografar(c, saida, "antes")
    ct.tela(c, saida, "antes")
    for p in passos:
        c.mj.sessao_gdb()
        p["plano"] = 1
        p["mappnt"] = [mx, my]
        c.escrever("CurExpTool", p["f"], 2)
        c.mj.escrever_variavel(f"8:{0x8600 + p['f']:x}", bytes([p["sub"]]), 1)
        c.dep.retomar()
        x0, y0 = ct.MAPA_X0 + (alvo[0] - mx) * ct.TILE + 8, ct.MAPA_Y0 + (alvo[1] - my) * ct.TILE + 8
        qb.mouse_canto(c.q)
        qb.mover_rel(c.q, x0, y0)
        time.sleep(0.3)
        c.q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": True, "button": "left"}}])
        time.sleep(0.1)
        c.q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": False, "button": "left"}}])
        time.sleep(1.5)
        c.log(f"passo {p}")
    ct.tela(c, saida, "depois")
    depois = ct.fotografar(c, saida, "depois")
    for nome, tam in (("BoyMsgCnt", 4), ("LastThemeSong", 2)):
        c.log(f"{nome} depois = {c.mj.ler_variavel(nome, tam).hex()}")
    ct.gravar_acao(saida, antes, depois, {"exp": passos})
    return []


# ------------------------------------------------------------------------------------------------
# Rainha amarela: a segunda metade 0xFE no mapa de vida (telas/jogo.md, formiga-amarela.md §3)
# ------------------------------------------------------------------------------------------------

def cenario_amarela_rainha(c: Sessao, saida, n):
    """A amarela vira rainha (_MeType = 0x60) num túnel do ninho preto, virada para a direita, com a
    0xFF na cabeça e a 0xFE atrás; vista do ninho preto (Ctrl+9) e um clique num túnel a algumas
    células (ordem de andar): a cada passo _MoveMyLife regrava as duas metades. Grava os ticks e a
    tela (a 0xFE desenhada com o tile 0x108 + _MeBase + _MeType + _MeDir)."""
    c.teclas("ctrl+9")
    time.sleep(1.5)
    c.entrada()
    s = c.save()
    vida_b = vida_ler(c, "B")
    livre = lambda x, y: 0 < x < 63 and 0 < y < 63 and s.celula("MapB", x, y) < 0x10 and vida_b[x * 64 + y] == 0
    x, y, d = next((x, y, d) for y in range(3, 40) for x in range(2, 60) for d in (2, 6, 4, 0, 1, 3, 5, 7)
                   if livre(x, y) and livre(x + D8[d ^ 4][0], y + D8[d ^ 4][1]))
    cx, cy = x + D8[d ^ 4][0], y + D8[d ^ 4][1]
    cr.por_amarela(c, 2, x, y, tipo=0x60, vida=100)
    c.escrever("MeDir", d)
    vida_escrever(c, "B", cx, cy, 0xFE)
    c.log(f"amarela rainha em ({x}, {y}) direção {d}, 0xFE em ({cx}, {cy})")
    alvos = [(tx, ty) for ty in range(1, 63) for tx in range(1, 63) if livre(tx, ty)
             and 4 <= max(abs(tx - x), abs(ty - y)) <= 7]
    tx, ty = alvos[len(alvos) // 2]
    c.clicar_celula(tx, ty, pausado=True)
    nomes = c.gravar(saida, n)
    ct.tela(c, saida, "rainha_amarela")
    return nomes


# ------------------------------------------------------------------------------------------------
# Botão esquerdo encerrando o laço do MYTIMERFUNC (simulacao-visao-geral.md §2)
# ------------------------------------------------------------------------------------------------

def cenario_ritmo_clique(c: Sessao, saida, n):
    """Em ultra (Shift+4), para na entrada do MYTIMERFUNC (1000:2440) a cada chamada e anota quantos
    ticks a chamada anterior rodou (GameTime) e o relógio do BIOS (0040:006C). `n` chamadas sem clique,
    depois `n` chamadas com um clique esquerdo na barra de título da janela principal (320, 8) logo depois
    de soltar a VM. Grava saida/ritmo.json."""
    c.teclas("shift+4")
    time.sleep(3)
    c.dep.retomar()
    qb.mouse_canto(c.q)
    qb.mover_rel(c.q, 320, 8)
    time.sleep(0.3)
    g = c.mj.sessao_gdb()
    mt = c.mj.linear_codigo(1, 0x2440)

    def chamada(clicar):
        # um passo de instrução antes de religar o breakpoint (continuar dele o atingiria de novo)
        g.drenar()
        g.s.sendall(b"$s#73")
        while not g._e_parada(g._proximo()):
            pass
        g.breakpoint(mt)
        g.drenar()
        g.s.sendall(b"$c#63")
        clicou = False
        if clicar:
            time.sleep(0.01)
            try:
                c.botoes(False)
                clicou = True
            except RuntimeError as e:
                if "not running" not in str(e):
                    raise
        while not g._e_parada(g._proximo()):
            pass
        g.breakpoint(mt, False)
        bios = int.from_bytes(c.mj.ler_linear(0x46C, 4), "little")
        return {"GameTime": c.ler("GameTime"), "bios": bios, "clique": clicou}

    reg = [chamada(False) for _ in range(n)]
    reg += [chamada(k % 2 == 0) for k in range(2 * n)]
    for a, b in zip(reg, reg[1:]):
        b["ticks"] = b["GameTime"] - a["GameTime"]
        b["bios_delta"] = b["bios"] - a["bios"]
    Path(saida).mkdir(parents=True, exist_ok=True)
    (Path(saida) / "ritmo.json").write_text(json.dumps(reg, indent=1) + "\n", encoding="utf-8")
    sem = [r["ticks"] for r in reg[1:n]]
    depois_clique = [r["ticks"] for a, r in zip(reg[n:], reg[n + 1:]) if a["clique"]]
    outras = [r["ticks"] for a, r in zip(reg[n:], reg[n + 1:]) if not a["clique"]]
    c.log(f"ticks por chamada sem clique: {sem}")
    c.log(f"na chamada em que houve clique: {depois_clique}; nas outras: {outras}")
    return []


# ------------------------------------------------------------------------------------------------
# Comida que cai na casa (superficie.md 8.2, _DROPdir)
# ------------------------------------------------------------------------------------------------

def cenario_queda_casa(c: Sessao, saida, n):
    """Jogo Completo (snapshot full), patch da casa (1, 5) como em cenarios_regras.choque; a amarela
    carregando comida (_MeType 0x18) ao lado de uma célula de piso que não aceita comida (tile 4–7 ou
    0x27–0x3F) com ordem de andar até ela: ao chegar, _TryMyDropOrLift → _DropMyFood → _DropFoodA, e a
    comida cai na direção de _DROPdir (ou some). CASA_TILE (ambiente) escolhe o tile procurado."""
    import os
    ct.cenario_xfer(c, str(Path(saida).resolve() / "xfer"), 2)
    c.dep.retomar()
    c.teclas("ctrl+e")
    time.sleep(1)
    c.teclas("ctrl+8")
    time.sleep(1)
    c.teclas("shift+0")
    time.sleep(2)
    c.entrada()
    s = c.save()
    vida_a = vida_ler(c, "A")
    queda = lambda v: 4 <= v <= 7 or 0x27 <= v <= 0x3F  # noqa: E731
    contagem = Counter(s.celula("MapA", x, y) for x in range(128) for y in range(64))
    c.log(f"_CurYardPnt = {c.ler('CurYardPnt'):#x}; tiles {sorted(contagem.items())[:40]}")
    quer = int(os.environ.get("CASA_TILE", "-1"), 0)
    alvos = [(x, y) for x in range(2, 126) for y in range(2, 62)
             if (s.celula("MapA", x, y) == quer if quer >= 0 else queda(s.celula("MapA", x, y)))]
    for tx, ty in alvos:
        viz = [(tx + dx, ty + dy) for dx, dy in D8 if vida_a[(tx + dx) * 64 + ty + dy] == 0
               and s.celula("MapA", tx + dx, ty + dy) <= 3]
        if viz:
            break
    else:
        raise RuntimeError("sem célula de queda com vizinha de piso")
    x, y = viz[0]
    c.log(f"alvo ({tx}, {ty}) tile {s.celula('MapA', tx, ty):#x}; amarela em ({x}, {y})")
    cr.por_amarela(c, 1, x, y, tipo=0x18, vida=90)
    for nome, v in (("MeGoalX", tx), ("MeGoalY", ty), ("MeMoveMe", 1), ("MeCmd", 1), ("MeCrazyCnt", -2)):
        c.escrever(nome, v)
    return c.gravar(saida, n, parar=lambda d: d.ler("MeType") & 8 == 0)


def xfer_para(c: Sessao, saida, alvo):
    """Como cenarios_tick.cenario_xfer, para o patch `alvo` (n = px·16 + py; casa: px < 2)."""
    c.teclas("shift+0")
    time.sleep(1)
    c.mj.sessao_gdb()
    c.escrever_elemento("YMapPopB", alvo[0] * 16 + alvo[1], 3)
    c.escrever("ColonyTotalBlack", 2)
    c.dep.retomar()
    c.teclas("ctrl+y")
    time.sleep(2)
    antes = ct.fotografar(c, saida, "antes")
    ct.clicar_tela(c, *ct.pixel_patch(*alvo), duplo=True)
    time.sleep(4)
    ct.dispensar(c)
    time.sleep(2)
    depois = ct.fotografar(c, saida, "depois")
    ct.gravar_acao(saida, antes, depois, {"xfer": list(alvo)})


def cenario_queda_casa_aranha(c: Sessao, saida, n):
    """Como queda_casa, mas quem larga a comida é a aranha em agonia (inimigos.md 2.5: _SMode 5, ao fim
    de _DeathCnt vira comida pela _DropFoodA na própria célula), posta sobre um fiapo do carpete (0x3E/
    0x3F, que não aceita comida): a comida cai na direção de _DROPdir."""
    import os
    alvo = tuple(int(v) for v in os.environ.get("CASA_PATCH", "1,5").split(","))
    xfer_para(c, str(Path(saida).resolve() / "xfer"), alvo)
    c.dep.retomar()
    c.teclas("ctrl+e")
    time.sleep(1)
    c.teclas("ctrl+8")
    time.sleep(1)
    c.teclas("shift+0")  # o xfer deixa o jogo pausado: Unpause
    time.sleep(2)
    c.entrada()
    s = c.save()
    alvos = [(x, y) for x in range(4, 124) for y in range(4, 60) if s.celula("MapA", x, y) in (0x3E, 0x3F)
             and s.celula("MapA", x - 1, y) <= 3 and s.celula("MapA", x + 1, y) <= 3]
    x, y = alvos[len(alvos) // 2]
    c.log(f"_CurYardPnt = {c.ler('CurYardPnt'):#x}; aranha em agonia em ({x}, {y}) tile {s.celula('MapA', x, y):#x}")
    cr.aranha_em(c, x, y, modo=5)
    c.escrever("DeathCnt", 3)
    return c.gravar(saida, n, parar=lambda d: d.ler("SpidOn") == 0)


# ------------------------------------------------------------------------------------------------
# _LastColonyPopR depois do Load (quintal.md, "Implementação": 1 depois de _InitSimYard e do Load)
# ------------------------------------------------------------------------------------------------

def gravar_ate_caixa(c: Sessao, saida, limite):
    """Grava tick a tick até um tick que abriu caixa (extras "dialogos"); devolve os nomes."""
    nomes = []
    for _ in range(limite):
        nomes += c.gravar(saida, 1)
        ex = json.loads((Path(saida).resolve() / nomes[-1] / "extras.json").read_text(encoding="utf-8"))
        if ex.get("dialogos"):
            c.log(f"{nomes[-1]}: {ex['dialogos']} caixa(s) (capturas re_trabalho/cen/dialogo_*)")
            break
    return nomes


def cenario_ultima_vermelha(c: Sessao, saida, n):
    """Jogo Completo (snapshot full) sem nenhuma vermelha (listas, vida, _YMapPopR, _RpopT, _CastePopR
    zerados na entrada de um tick): na primeira passada de _SimColonies (_YardCycle & 0x1F = 0) com
    _LastColonyPopR = 1 (do _InitSimYard) o original mostra 10052 (e 10055). Depois salva (Ctrl+A,
    ULTVERM), carrega o mesmo arquivo (Ctrl+L) e grava até a passada seguinte: se o Load repõe
    _LastColonyPopR = 1, as caixas voltam."""
    c.entrada()
    for nome, tam in (("RlistT", 500), ("YMapPopR", 192), ("LifeR", 4096)):
        seg, off, _ = c.mj.resolver(nome, tam)
        for k in range(0, tam, 256):
            c.mj.escrever_variavel(f"{seg}:{off + k:x}", bytes(min(256, tam - k)), min(256, tam - k))
        c.log(f"{nome} zerado")
    s = c.save()
    for i, f in formigas(s, "A"):
        if f["T"] & 0x80:
            vida_escrever(c, "A", f["X"], f["Y"], 0)
            c.escrever_elemento("AlistT", i, 0)
    seg, off, _ = c.mj.resolver("CastePopR")
    c.mj.escrever_variavel(f"{seg}:{off:x}", bytes(12), 12)
    c.escrever("RpopT", 0)
    c.escrever("RedQueens", 0)
    c.log(f"YardCycle {c.ler('YardCycle')}, ForSaleState {c.ler('ForSaleState')}")
    nomes = gravar_ate_caixa(c, saida, 40)
    # salvar e carregar
    c.teclas("ctrl+a")
    time.sleep(3)
    ct.tela(c, saida, "salvar_como")
    c.b.executar("digitar ULTVERM")
    c.teclas("ret")
    time.sleep(4)
    ct.tela(c, saida, "salvo")
    c.teclas("ret")
    time.sleep(2)
    c.teclas("ctrl+l")
    time.sleep(3)
    ct.tela(c, saida, "load_pergunta")
    c.teclas("n")
    time.sleep(3)
    c.b.executar("digitar ULTVERM")
    c.teclas("ret")
    time.sleep(6)
    ct.tela(c, saida, "carregado")
    c.teclas("shift+0")
    time.sleep(1)
    c.entrada()
    c.log(f"depois do Load: GameTime {c.ler('GameTime')}, YardCycle {c.ler('YardCycle')}")
    nomes += gravar_ate_caixa(c, saida, 40)
    return nomes


def mortes(bruto):
    """Por tick: vagas que sumiram (T ≠ 0 → 0) em cada lista, com o T e o M de antes, e os contadores
    _BAntsExpired/_RAntsExpired, _BlkQueens/_RedQueens."""
    for d in sorted(Path(bruto).glob("tick_*")):
        a, b = Save.abrir(d / "estado_antes.bin"), Save.abrir(d / "estado_depois.bin")
        sumiram = []
        for lista in ("A", "B", "R"):
            fa, fb = a.formigas(lista), b.formigas(lista)
            if lista == "A" and (a.ler("Cycle") + 1) % 128 == 0:
                continue  # compactação: índices mudam
            for i, (x, y) in enumerate(zip(fa, fb)):
                if x["T"] and not y["T"]:
                    sumiram.append(f"{lista}[{i}] T {x['T']:#04x} M {x['M']} ({x['X']}, {x['Y']})")
        cont = [f"{k} {a.ler(k)}->{b.ler(k)}" for k in ("BAntsExpired", "RAntsExpired", "BlkQueens", "RedQueens",
                                                          "RedLost", "BlackLost") if a.ler(k) != b.ler(k)]
        if sumiram or cont:
            print(f"{d.name}: {'; '.join(sumiram)} | {', '.join(cont)}")


if __name__ == "__main__":
    if len(sys.argv) >= 3 and sys.argv[1] == "mortes":
        mortes(sys.argv[2])
        sys.exit(0)
    if len(sys.argv) >= 3 and sys.argv[1] == "contadores":
        contadores_de_luta(sys.argv[2:])
        sys.exit(0)
    if len(sys.argv) >= 3 and sys.argv[1] == "lutas":
        lutas(sys.argv[2:])
        sys.exit(0)
    if len(sys.argv) >= 4 and sys.argv[1] == "selecionar_lote":
        cr.selecionar_lote(sys.argv[2], sys.argv[3])
        sys.exit(0)
    for nome, f in list(globals().items()):
        if nome.startswith("cenario_"):
            setattr(ct, nome, f)
    for nome, f in list(vars(cr).items()):
        if nome.startswith("cenario_") and not hasattr(ct, nome):
            setattr(ct, nome, f)
    ct.main()
