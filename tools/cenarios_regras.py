"""Cenários de vetores de tick para as regras ainda não verificadas na revisão de 2026-10-18
(docs/estado-do-projeto.md §3): cerco da aranha e da formiga-leão, faro, laser, aranha do jogador,
mortes da amarela (aranha, formiga-leão, cercada, fora do mapa, pisada, cortador, afogada, fome,
choque) e os casos estatísticos do ninho (N-01, N-07, N-10, N-12, N-14, N-15).

Mesma interface de tools/cenarios_tick.py (os cenários de lá também valem aqui):

    set SIMANT_RAIZ=...
    python tools/cenarios_regras.py gravar CENARIO DIR_BRUTO --overlay OV [--snapshot rico] [--ticks N]
    python tools/cenarios_regras.py eventos DIR_BRUTO
    python tools/cenarios_regras.py selecionar DIR_BRUTO DESTINO TICK[-TICK] ... [--descricao TEXTO]

A preparação escreve a memória do original na entrada de um tick (o estado de antes do vetor é o que
foi lido depois das escritas, então o vetor vale qualquer que seja o caminho até ali). Onde a regra
depende da interface (troca de corpo para virar a aranha, troca de patch para a casa), a preparação
usa a interface do original (teclas e cliques pela bancada).
"""
import json
import struct
import os
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cenarios_tick as ct  # noqa: E402
import qemu_bancada as qb  # noqa: E402
from cenarios_tick import Sessao, formigas  # noqa: E402

D8 = ((0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1))
TAM_VIDA = {"A": ("LifeA", 8192), "B": ("LifeB", 4096), "R": ("LifeR", 4096)}
PLANO_LISTA = {1: "A", 2: "B", 3: "R"}

# variáveis olhadas por `eventos` (além das de cenarios_tick.OLHAR)
ct.OLHAR = ct.OLHAR + ("SpidRevenge", "MeSMode", "MeMode", "SpidX", "SpidY", "Starg", "EatCnt", "DeathCnt",
                       "LionIndex", "BAntsExpired", "RAntsExpired", "FoodB", "HealthB", "EatCountB",
                       "ListIndexB", "ListIndexR", "MeEgg", "MeLocX", "MeLocY", "IsGameOver")


def _botoes_tolerante(self, duplo):
    """Como Sessao.botoes, mas o tick pode voltar (breakpoint do retorno) entre o botão apertado e o
    solto, parando a VM: o QMP recusa o evento seguinte ("VM not running"), o que não é erro."""
    try:
        _botoes_original(self, duplo)
    except RuntimeError as e:
        if "VM not running" not in str(e):
            raise


_botoes_original = Sessao.botoes
Sessao.botoes = _botoes_tolerante
_fechar_original = Sessao.fechar_caixa_ate_o_retorno
# Só Enter nas 4 primeiras tentativas (k ímpar no original): o clique em (320, 310) que fecha a caixa
# pode chegar à janela de edição depois de a caixa sumir e virar uma ordem para a amarela (visto em
# `fome`: destino (40, 9) e um passo dados pelo clique), e um clique cortado pela parada no retorno
# deixaria o botão apertado para o Windows; o send-key solta a tecla sozinho
Sessao.fechar_caixa_ate_o_retorno = lambda self, k, limite=15: _fechar_original(self, 1 if k < 4 else k, limite)


# ------------------------------------------------------------------------------------------------
# Auxiliares de memória
# ------------------------------------------------------------------------------------------------

def vida_escrever(c: Sessao, lista, x, y, v):
    nome, tam = TAM_VIDA[lista]
    seg, off, _ = c.mj.resolver(nome, tam)
    c.mj.escrever_variavel(f"{seg}:{off + x * 64 + y:x}", bytes([v & 0xFF]), 1)


def vida_ler(c: Sessao, lista):
    return c.mj.ler_variavel(*TAM_VIDA[lista])


def mapa_escrever(c: Sessao, mapa, x, y, v):
    seg, off, _ = c.mj.resolver(mapa)
    c.mj.escrever_variavel(f"{seg}:{off + x * 64 + y:x}", bytes([v & 0xFF]), 1)
    c.log(f"{mapa}({x}, {y}) <- {v:#x}")


def mover_formiga(c: Sessao, s, lista, i, x, y, m=None, t=None):
    """Move a vaga i da lista para (x, y), acertando o mapa de vida (apaga a marca antiga se era dela)."""
    f = s.formigas(lista)[i]
    vida = vida_ler(c, lista)
    if vida[f["X"] * 64 + f["Y"]] == f["T"]:
        vida_escrever(c, lista, f["X"], f["Y"], 0)
    t = f["T"] if t is None else t
    c.escrever_elemento(f"{lista}listX", i, x)
    c.escrever_elemento(f"{lista}listY", i, y)
    if t != f["T"]:
        c.escrever_elemento(f"{lista}listT", i, t)
    if m is not None:
        c.escrever_elemento(f"{lista}listM", i, m)
    vida_escrever(c, lista, x, y, t)


def acrescentar_formiga(c: Sessao, lista, x, y, t, m, s_=0):
    """Acrescenta uma vaga no fim da lista (ListIndex += 1) com a marca no mapa de vida."""
    n = c.ler(f"ListIndex{lista}")
    for campo, v in (("X", x), ("Y", y), ("T", t), ("M", m), ("S", s_)):
        c.escrever_elemento(f"{lista}list{campo}", n, v)
    c.escrever(f"ListIndex{lista}", n + 1)
    vida_escrever(c, lista, x, y, t)
    c.log(f"lista {lista}[{n}] = ({x}, {y}) T {t:#x} M {m} S {s_}")
    return n


def pretas_da_superficie(s, n, grupos=(2, 3, 6, 7)):
    return [i for i, f in formigas(s, "A") if not f["T"] & 0x80 and (f["T"] & 0x78) >> 3 in grupos][:n]


def tirar_amarela(c: Sessao):
    """Apaga a marca da amarela do plano em que está."""
    plano, ox, oy = c.ler("MePlane"), c.ler("MeLocX"), c.ler("MeLocY")
    if plano in PLANO_LISTA:
        vida_escrever(c, PLANO_LISTA[plano], ox, oy, 0)
    return plano, ox, oy


def por_amarela(c: Sessao, plano, x, y, tipo=None, vida=None):
    """Põe a amarela em (plano, x, y), parada (destino = posição), com a marca 0xFF."""
    antigo = tirar_amarela(c)
    vida_escrever(c, PLANO_LISTA[plano], x, y, 0xFF)
    for nome, v in (("MePlane", plano), ("MeLocX", x), ("MeLocY", y), ("MeGoalPlane", plano), ("MeGoalX", x),
                    ("MeGoalY", y), ("MeLastX", x), ("MeLastY", y), ("MeMoveMe", 0), ("MeCmd", 0)):
        c.escrever(nome, v)
    if tipo is not None:
        c.escrever("MeType", tipo)
    if vida is not None:
        c.escrever("MeHealth", vida)
    c.log(f"amarela de {antigo} para ({plano}, {x}, {y})")


def livre_a(s, vida_a, x, y):
    return 0 < x < 127 and 0 < y < 63 and s.celula("MapA", x, y) < 0x10 and vida_a[x * 64 + y] == 0


def aranha_em(c: Sessao, x, y, modo=0, direcao=None):
    """Liga a aranha centrada na célula (x, y), no estado `modo`, sem presa."""
    for nome, v in (("SpidOn", 1), ("SpidX", x * 16 + 8), ("SpidY", y * 16 + 8), ("SMode", modo), ("Starg", -2),
                    ("EatCnt", 0)):
        c.escrever(nome, v)
    if direcao is not None:
        c.escrever("SpidDir", direcao)


def lugar_aberto(s, vida_a, raio=2, perto=(64, 32)):
    """Uma célula (x, y) com o quadrado (2·raio+1)² livre de formigas, perto de `perto`."""
    cands = []
    for x in range(raio + 2, 127 - raio - 2):
        for y in range(raio + 2, 63 - raio - 2):
            if all(vida_a[(x + dx) * 64 + y + dy] == 0 for dx in range(-raio, raio + 1) for dy in range(-raio, raio + 1)):
                cands.append((abs(x - perto[0]) + abs(y - perto[1]), x, y))
    _, x, y = min(cands)
    return x, y


def esperar_compactacao(c: Sessao):
    """Vai até logo depois de uma compactação da lista A (_Cycle múltiplo de 128): os índices ficam
    estáveis por 127 ticks."""
    gt = c.dep.ate_gametime((c.ler("Cycle") // 128 + 1) * 128 + 1)
    c.log(f"esperou até GameTime {gt}")


# ------------------------------------------------------------------------------------------------
# Aranha (inimigos.md §2)
# ------------------------------------------------------------------------------------------------

def cercar_aranha(c: Sessao, n_formigas):
    c.entrada()
    esperar_compactacao(c)
    s = c.save()
    vida_a = vida_ler(c, "A")
    x, y = lugar_aberto(s, vida_a, raio=2)
    aranha_em(c, x, y, modo=0)
    celulas = [(x + dx, y + dy) for dy in (-1, 0, 1) for dx in (-1, 0, 1)][:n_formigas]
    for i, (cx, cy) in zip(pretas_da_superficie(s, n_formigas), celulas):
        mover_formiga(c, s, "A", i, cx, cy, m=17)
    c.log(f"{n_formigas} pretas (M = 17) no 3 x 3 da aranha em ({x}, {y})")
    # o cerco é testado com (_Scycle2 + 1) & 3 == 0 (o tick soma antes de testar)
    c.escrever("Scycle2", 3)


def cenario_cerco_aranha(c: Sessao, saida, n):
    """9 pretas paradas (M = 17) no 3 x 3 da aranha parada; _SpidRevenge = 2 (vira 3: a próxima caça
    por faro). O tick do cerco abre a mensagem 10046."""
    cercar_aranha(c, 9)
    c.escrever("SpidRevenge", 2)
    return c.gravar(saida, n)


def cenario_foge_aranha(c: Sessao, saida, n):
    """6 pretas paradas no 3 x 3 da aranha: estado 4 (foge)."""
    cercar_aranha(c, 6)
    return c.gravar(saida, n)


def cenario_vinganca_laser(c: Sessao, saida, n):
    """Cerco com _SpidRevenge = 4: vira 5 e a próxima aranha caça por laser (_MeSMode = 8)."""
    cercar_aranha(c, 9)
    c.escrever("SpidRevenge", 4)
    return c.gravar(saida, n)


def cenario_faro(c: Sessao, saida, n):
    """Aranha andando com _MeSMode = 7 (faro) e pretas a menos de 28 células: persegue, come
    (_EatCnt = 11) e larga o cadáver. Para quando volta ao estado 0 depois de comer."""
    c.entrada()
    esperar_compactacao(c)
    s = c.save()
    vida_a = vida_ler(c, "A")
    x, y = lugar_aberto(s, vida_a, raio=3)
    aranha_em(c, x, y, modo=1, direcao=2)
    c.escrever("MeSMode", 7)
    comeu = []

    def parar(d):
        if d.ler("SMode") == 3:
            comeu.append(1)
        return comeu and d.ler("SMode") == 0
    return c.gravar(saida, n, parar=parar)


def cenario_laser(c: Sessao, saida, n):
    """Aranha andando para cima (direção 0) com _MeSMode = 8 e um leque de pretas paradas (M = 17) ao
    norte, a 2..9 células: _SpiderScan acha e mata com S4() != 0."""
    c.entrada()
    esperar_compactacao(c)
    s = c.save()
    vida_a = vida_ler(c, "A")
    x, y = lugar_aberto(s, vida_a, raio=1, perto=(64, 40))
    aranha_em(c, x, y, modo=1, direcao=0)
    c.escrever("MeSMode", 8)
    leque = [(x + dx, y - r) for r in range(2, 10) for dx in range(-(r // 2), r // 2 + 1)]
    leque = [p for p in leque if livre_a(s, vida_a, *p)]
    pretas = pretas_da_superficie(s, len(leque))
    for i, p in zip(pretas, leque):
        mover_formiga(c, s, "A", i, *p, m=17)
    c.log(f"{min(len(pretas), len(leque))} pretas no leque ao norte da aranha em ({x}, {y})")
    return c.gravar(saida, n)


# ------------------------------------------------------------------------------------------------
# Formiga-leão (inimigos.md §1.3)
# ------------------------------------------------------------------------------------------------

def cenario_cerco_leao(c: Sessao, saida, n):
    """Formiga-leão 0 posta comendo (M = 1, T = 1, S = 50) com 7 pretas paradas nas vizinhas: morre
    (mensagem 10050, centro 0x3F, lista deslocada)."""
    c.entrada()
    esperar_compactacao(c)
    s = c.save()
    lx, ly = c.mj.ler_variavel("LionListX", 10)[0], c.mj.ler_variavel("LionListY", 10)[0]
    c.escrever_elemento("LionListM", 0, 1)
    c.escrever_elemento("LionListT", 0, 1)
    c.escrever_elemento("LionListS", 0, 50)
    mapa_escrever(c, "MapA", lx, ly, 0x39)
    viz = [(lx + dx, ly + dy) for dx, dy in D8][:7]
    for i, p in zip(pretas_da_superficie(s, 7), viz):
        mover_formiga(c, s, "A", i, *p, m=17)
    c.log(f"formiga-leão 0 em ({lx}, {ly}) comendo, 7 pretas em volta")
    return c.gravar(saida, n)


def cenario_amarela_leao(c: Sessao, saida, n):
    """A amarela posta no centro da formiga-leão 0 (esperando, centro 0x38): PEGA → _YellowDeath(2),
    com _AnimYellowFight (centro animando) e o quadro da formiga-leão."""
    c.entrada()
    lx, ly = c.mj.ler_variavel("LionListX", 10)[0], c.mj.ler_variavel("LionListY", 10)[0]
    c.escrever_elemento("LionListM", 0, 0)
    c.escrever_elemento("LionListT", 0, 0)
    mapa_escrever(c, "MapA", lx, ly, 0x38)
    por_amarela(c, 1, lx, ly, vida=100)
    return c.gravar(saida, n, parar=lambda d: d.ler("MeHealth") == 100 and d.ler("MePlane") != 1)


def cenario_amarela_aranha(c: Sessao, saida, n):
    """A amarela na superfície ao lado da aranha, que já a persegue (_SMode = 2, _Starg = −1): no
    tick a aranha a come (_YellowDeath(1), _AnimYellowFight e o quadro da aranha)."""
    c.entrada()
    esperar_compactacao(c)
    s = c.save()
    vida_a = vida_ler(c, "A")
    x, y = lugar_aberto(s, vida_a, raio=2)
    por_amarela(c, 1, x + 1, y, vida=100)
    aranha_em(c, x, y, modo=2, direcao=2)
    c.escrever("StargLife", 0xFF)
    c.escrever("Starg", -1)
    return c.gravar(saida, n)


# Funções do original contadas por `contar_no_tick` (segmento de código, offset; re_trabalho/symbols.tsv)
FUNCOES_CONTADAS = {
    "_SRand1": (5, 5514), "_SpiderDialog": (1, 26426), "_LionDialog": (1, 32928), "_GotoMyAnt": (1, 42852),
    "_YellowDeath": (1, 30534), "_DoEditUpdateDraw": (3, 3670), "_DrawSpider": (3, 7530),
    "_DrawPalps": (3, 9620), "_AnimYellowFight": (6, 32342), "_mySongIsDone": (2, 41376),
}


def contar_no_tick(c: Sessao, saida, funcoes=FUNCOES_CONTADAS, timeout=6):
    """Roda UM tick (da entrada ao retorno de _DoAntSim) parando na entrada de cada função de
    `funcoes`: anota a ordem, o S nesse instante e, para _SRand1, o endereço de retorno (quem sorteou).
    Uma caixa modal (o tick não para em `timeout` s) é fechada com Enter. Grava saida/contagem.json."""
    dep, mj = c.dep, c.mj
    dep.ir_entrada()
    g = mj.sessao_gdb()
    alvos = {}
    for nome, (seg, off) in funcoes.items():
        try:
            alvos[mj.linear_codigo(seg, off)] = nome
        except RuntimeError as e:
            c.log(f"{nome}: {e}")
    sel_seg = {mj.seletor_codigo(n): n for n in range(1, 8)}
    eventos = []
    armados = set(alvos) | {dep.retorno}
    for a in armados:
        g.breakpoint(a)
    s0 = c.ler("SementeS")
    try:
        while True:
            try:
                g.continuar_ate_parar(timeout)
            except RuntimeError:
                # caixa modal: a VM foi interrompida; Enter com ela correndo
                g.drenar()
                g.s.sendall(b"$c#63")
                time.sleep(0.2)
                c.q.cmd("send-key", keys=[{"type": "qcode", "data": "ret"}], **{"hold-time": 150})
                eventos.append({"f": "(Enter numa caixa)"})
                g.s.settimeout(60)
                while not g._e_parada(g._proximo()):
                    pass
                g.s.settimeout(30)
            pc, regs = dep.pc()
            if pc == dep.retorno:
                break
            if pc not in alvos:
                eventos.append({"f": f"parada em {pc:#x}"})
                continue
            ev = {"f": alvos[pc], "S": c.ler("SementeS")}
            if alvos[pc] == "_SRand1":
                pilha = regs["SS"][1] + (regs["ESP"] & 0xFFFF)
                ip, cs = struct.unpack("<HH", mj.ler_linear(pilha, 4))
                n = int.from_bytes(mj.ler_linear(pilha + 4, 2), "little")
                ev.update({"de": f"{sel_seg.get(cs, cs)}:{ip:04x}", "n": n})
            eventos.append(ev)
            # passa por cima do breakpoint: desliga, um passo, religa
            g.breakpoint(pc, False)
            g.drenar()
            g.s.sendall(b"$s#73")
            while not g._e_parada(g._proximo()):
                pass
            g.breakpoint(pc)
    finally:
        for a in armados:
            g.breakpoint(a, False)
    s1 = c.ler("SementeS")
    res = {"S_antes": s0, "S_depois": s1, "passos_S": ct.passos_s(s0, s1), "eventos": eventos}
    Path(saida).resolve().mkdir(parents=True, exist_ok=True)
    (Path(saida).resolve() / "contagem.json").write_text(json.dumps(res, indent=1) + "\n", encoding="utf-8")
    c.log(f"{len(eventos)} paradas; {res['passos_S']} passos de S")
    return []


def cenario_contar_aranha(c: Sessao, saida, n):
    """Como `amarela_aranha`, mas o tick da morte é rodado por `contar_no_tick` (quem sorteia S)."""
    c.entrada()
    esperar_compactacao(c)
    s = c.save()
    vida_a = vida_ler(c, "A")
    x, y = lugar_aberto(s, vida_a, raio=2)
    por_amarela(c, 1, x + 1, y, vida=100)
    aranha_em(c, x, y, modo=2, direcao=2)
    c.escrever("StargLife", 0xFF)
    c.escrever("Starg", -1)
    return contar_no_tick(c, saida)


def cenario_contar_leao(c: Sessao, saida, n):
    """Controle: a morte pela formiga-leão (que bate com o port) contada do mesmo jeito."""
    c.entrada()
    lx, ly = c.mj.ler_variavel("LionListX", 10)[0], c.mj.ler_variavel("LionListY", 10)[0]
    c.escrever_elemento("LionListM", 0, 0)
    c.escrever_elemento("LionListT", 0, 0)
    mapa_escrever(c, "MapA", lx, ly, 0x38)
    por_amarela(c, 1, lx, ly, vida=100)
    return contar_no_tick(c, saida)


# ------------------------------------------------------------------------------------------------
# Aranha do jogador (inimigos.md §2.7, formiga-amarela.md §8.3, §11.3)
# ------------------------------------------------------------------------------------------------

def virar_aranha(c: Sessao):
    """Pela interface do original: vista da superfície, aranha parada num lugar aberto, tecla X
    (troca de corpo; o jogo pausa) e clique na célula da aranha (_DoLifeExchange caso 1)."""
    c.teclas("ctrl+8")
    time.sleep(1)
    c.entrada()
    esperar_compactacao(c)
    s = c.save()
    vida_a = vida_ler(c, "A")
    x, y = lugar_aberto(s, vida_a, raio=3)
    aranha_em(c, x, y, modo=0, direcao=0)
    c.teclas("x")
    time.sleep(1)
    c.clicar_celula(x, y)
    time.sleep(1.5)
    c.entrada()
    c.log(f"_MeMode = {c.ler('MeMode')}, _MeLoc = ({c.ler('MeLocX')}, {c.ler('MeLocY')}), aranha em ({x}, {y})")
    if c.ler("MeMode") != 1:
        raise RuntimeError("a troca de corpo com a aranha não pegou")
    return x, y


def cenario_aranha_jogador(c: Sessao, saida, n):
    """O jogador vira a aranha; clique simples a 8 células (anda), depois duplo clique a 10 células
    (corre); depois tecla 1 (modo alvo) e clique numa preta: persegue (_MeSMode = 6) e come."""
    x, y = virar_aranha(c)
    nomes = []
    c.clicar_celula(x + 8, y, pausado=True)
    nomes += c.gravar(saida, n, parar=lambda d: d.ler("SpidX") >> 4 == x + 8)
    c.entrada()
    s = c.save()
    c.clicar_celula(x + 8, y + 10, duplo=True, pausado=True)
    nomes += c.gravar(saida, n, parar=lambda d: d.ler("SpidY") >> 4 == y + 10)
    # modo alvo
    c.entrada()
    s = c.save()
    sx, sy = c.ler("SpidX") >> 4, c.ler("SpidY") >> 4
    vida_a = vida_ler(c, "A")
    alvo = next((sx + dx, sy) for dx in range(6, 20) if livre_a(s, vida_a, sx + dx, sy))
    mover_formiga(c, s, "A", pretas_da_superficie(s, 1)[0], *alvo, m=17)
    c.teclas("1")
    time.sleep(1)
    c.clicar_celula(*alvo)
    time.sleep(1)
    c.entrada()
    c.log(f"alvo {alvo}: _SMode = {c.ler('SMode')}, _MeSMode = {c.ler('MeSMode')}")
    comeu = []

    def parar(d):
        if d.ler("SMode") == 3:
            comeu.append(1)
        return comeu and d.ler("SMode") == 0
    nomes += c.gravar(saida, n, parar=parar)
    return nomes


def cenario_aranha_cercada(c: Sessao, saida, n):
    """O jogador é a aranha e 9 pretas a cercam: _YellowDeath(3) (mensagem 10026) e reencarnação."""
    x, y = virar_aranha(c)
    s = c.save()
    for i, (cx, cy) in zip(pretas_da_superficie(s, 9), [(x + dx, y + dy) for dy in (-1, 0, 1) for dx in (-1, 0, 1)]):
        mover_formiga(c, s, "A", i, cx, cy, m=17)
    # com _SMode 0/1 o controle do jogador roda no lugar do resto de _MoveSpider (sem cerco): a aranha
    # do jogador só é cercada perseguindo ou comendo; aqui ela persegue a preta da direita
    s = c.save()
    alvo = next((i, f) for i, f in formigas(s, "A") if (f["X"], f["Y"]) == (x + 1, y))
    c.escrever("SMode", 2)
    c.escrever("Starg", alvo[0])
    c.escrever("StargLife", alvo[1]["T"])
    c.escrever("Scycle2", 3)
    return c.gravar(saida, n, parar=lambda d: d.ler("MeMode") == 0)


def cenario_aranha_fora(c: Sessao, saida, n):
    """O jogador é a aranha, na linha 0 virada para cima, e persegue (duplo clique) uma preta 5
    células à direita na linha 0: o giro parcial e a corrida ×5 a tiram do mapa (_YellowDeath(4))."""
    x, y = virar_aranha(c)
    s = c.save()
    alvo = (x + 5, 0)
    i = pretas_da_superficie(s, 1)[0]
    mover_formiga(c, s, "A", i, *alvo, m=17)
    for nome, v in (("SpidX", x * 16 + 8), ("SpidY", 4), ("SpidDir", 0), ("MeLocX", x), ("MeLocY", 0),
                    ("SuserX", x), ("SuserY", 0)):
        c.escrever(nome, v)
    c.clicar_celula(*alvo, duplo=True, pausado=True)
    return c.gravar(saida, n, parar=lambda d: d.ler("MeMode") == 0)


# ------------------------------------------------------------------------------------------------
# Mortes da amarela (formiga-amarela.md §9)
# ------------------------------------------------------------------------------------------------

def cenario_fome(c: Sessao, saida, n):
    """A amarela na superfície sem vida há 99 ticks: _YellowDeath(8) (fome) com _AnimYellowInsane."""
    c.entrada()
    if c.ler("MePlane") != 1:
        s = c.save()
        x, y = lugar_aberto(s, vida_ler(c, "A"), raio=2)
        por_amarela(c, 1, x, y)
    c.escrever("MeHealth", 0)
    c.escrever("MeHealthCntDwn", 99)
    return c.gravar(saida, n)


def cenario_afogada(c: Sessao, saida, n):
    """A amarela no ninho preto numa célula inundada (0x4E) sem vida há 99 ticks: _YellowDeath(7)."""
    c.entrada()
    plano, x, y = c.ler("MePlane"), c.ler("MeLocX"), c.ler("MeLocY")
    if plano != 2:
        x, y = 32, 3
        por_amarela(c, 2, x, y)
    mapa_escrever(c, "MapB", x, y, 0x4E)
    c.escrever("MeHealth", 0)
    c.escrever("MeHealthCntDwn", 99)
    return c.gravar(saida, n)


def cenario_cortador(c: Sessao, saida, n):
    """Menino cortando a grama no patch atual (como `menino` de cenarios_tick) e a amarela na
    superfície: cada passada (a cada 4 ticks) tem 3/4 de chance de _YellowDeath(6), se o pé não a
    pegar antes (5). Para quando ela morre (vida volta a 100 noutro corpo ou ovo)."""
    c.entrada()
    if c.ler("MePlane") != 1:
        s = c.save()
        x, y = lugar_aberto(s, vida_ler(c, "A"), raio=2)
        por_amarela(c, 1, x, y, vida=90)
    else:
        c.escrever("MeHealth", 90)
    p = c.ler("CurYardPnt")
    px, py = p & 0xFFFF, p >> 16
    boy_y = 38 + py * 10 + 5
    boy_x = px * 28 + 14 + 200 - boy_y
    for nome, v in (("NodeNum", 0x26), ("BoyHere", 3), ("BoyIsMowing", 1), ("BoyX", boy_x), ("BoyY", boy_y),
                    ("BoyPx", px), ("BoyPy", py), ("LastMowX", px), ("LastMowY", py), ("FootHere", 1)):
        c.escrever(nome, v)
    # o gato fora de jogo: o relógio do Mac lido depois de uma caixa modal não é gravado
    c.escrever("CatDelay", 0x7FFFFFFF)
    return c.gravar(saida, n, parar=lambda d: d.ler("MeHealth") == 100)


def cenario_pisada(c: Sessao, saida, n):
    """Pisada (_YellowDeath(5)). Duas execuções: sem PISADA_ALVO, o menino passeia (sem cortador) no
    patch atual e grava, por tick, _FootX/_FootY/_FootHere/_BoyDir em saida/pes.json (sem a amarela).
    Com PISADA_ALVO="GT", repete a mesma preparação e, na entrada do tick GT, põe a amarela na célula
    do pé que a 1ª execução viu nesse tick (o original é determinístico a partir do snapshot)."""
    c.entrada()
    p = c.ler("CurYardPnt")
    px, py = p & 0xFFFF, p >> 16
    boy_y = 38 + py * 10 + 5
    boy_x = px * 28 + 14 + 200 - boy_y
    for nome, v in (("NodeNum", 0x26), ("BoyHere", 3), ("BoyIsMowing", 0), ("BoyX", boy_x), ("BoyY", boy_y),
                    ("BoyPx", px), ("BoyPy", py), ("LastMowX", px), ("LastMowY", py), ("FootHere", 1)):
        c.escrever(nome, v)
    alvo = os.environ.get("PISADA_ALVO")
    pes_arq = Path(saida).resolve() / "pes.json"
    if not alvo:
        pes = {}
        for _ in range(n):
            gt = c.ler("GameTime")
            c.avancar(1)
            pes[gt] = {k: c.ler(k) for k in ("FootHere", "FootX", "FootY", "BoyDir", "BoyPx", "BoyPy")}
            c.log(f"{gt}: {pes[gt]}")
        pes_arq.parent.mkdir(parents=True, exist_ok=True)
        pes_arq.write_text(json.dumps(pes, indent=1) + "\n", encoding="utf-8")
        return []
    gt = int(alvo)
    pes = json.loads(pes_arq.read_text(encoding="utf-8"))
    pe = pes[str(gt)]
    c.dep.ate_gametime(gt)
    # célula do pé: o retângulo começa em (_FootX, _FootY) (quintal.md §3.1)
    por_amarela(c, 1, pe["FootX"], pe["FootY"], vida=90)
    # o gato fora de jogo: o relógio do Mac lido depois de uma caixa modal não é gravado
    c.escrever("CatDelay", 0x7FFFFFFF)
    return c.gravar(saida, n, parar=lambda d: d.ler("MeHealth") == 100)


def cenario_choque(c: Sessao, saida, n):
    """Jogo Completo (snapshot full): troca para o patch da casa (1, 5) pela Yard View (como
    `xfer --ticks 2`), volta a jogar na vista da superfície e, na entrada de um tick, a amarela ao lado
    de uma tomada (0x76) com ordem de andar até ela: a ordem termina em cima do 0x76 e
    _YellowDeath(10) (choque, _AnimYellowFight de 32 quadros)."""
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
    tomadas = [(x, y) for x in range(128) for y in range(64) if s.celula("MapA", x, y) in (0x76, 0x78)]
    c.log(f"_CurYardPnt = {c.ler('CurYardPnt'):#x}; tomadas {tomadas}")
    tx, ty = tomadas[0]
    vida_a = vida_ler(c, "A")
    viz = [(tx + dx, ty + dy) for dx, dy in D8 if vida_a[(tx + dx) * 64 + ty + dy] == 0
           and s.celula("MapA", tx + dx, ty + dy) <= 0x90 and s.celula("MapA", tx + dx, ty + dy) not in (0x76, 0x78)]
    x, y = viz[0]
    por_amarela(c, 1, x, y, vida=90)
    for nome, v in (("MeGoalX", tx), ("MeGoalY", ty), ("MeMoveMe", 1), ("MeCmd", 0), ("MeCrazyCnt", -2)):
        c.escrever(nome, v)
    return c.gravar(saida, n, parar=lambda d: d.ler("MeHealth") == 100)


# ------------------------------------------------------------------------------------------------
# Ninho (ninho.md §13)
# ------------------------------------------------------------------------------------------------

def cenario_n01(c: Sessao, saida, n):
    """N-01: _HealthB = 0 (e _AlwaysHealthy = 0): morte natural das donas do ninho preto com
    Sk(256) == 0 e Sk(32) > 0. Grava n ticks; selecionar os que têm _BAntsExpired subindo e um sem."""
    c.entrada()
    c.escrever("AlwaysHealthy", 0)
    c.escrever("HealthB", 0)
    return c.gravar(saida, n)


def cenario_n01_31(c: Sessao, saida, n):
    """N-01, controle: _HealthB = 31: nenhuma morte natural (Sk(32) nunca passa de 31)."""
    c.entrada()
    c.escrever("AlwaysHealthy", 0)
    c.escrever("HealthB", 31)
    return c.gravar(saida, n)


def cenario_n07(c: Sessao, saida, n):
    """N-07: soldado preto (T 0x32, M 1) preso numa célula de comida 0x13 cercada de terra no ninho
    preto; _HealthB = 50, _EatCountB = 0: mordidas com chance 49/100, comida 0x13 → túnel."""
    c.entrada()
    s = c.save()
    vida_b = vida_ler(c, "B")
    x, y = next((x, y) for y in range(40, 60) for x in range(4, 60)
                if all(0x20 <= s.celula("MapB", x + dx, y + dy) <= 0x2E and vida_b[(x + dx) * 64 + y + dy] == 0
                       for dx in (-1, 0, 1) for dy in (-1, 0, 1)))
    mapa_escrever(c, "MapB", x, y, 0x13)
    acrescentar_formiga(c, "B", x, y, 0x32, 1)
    c.escrever("HealthB", 50)
    c.escrever("EatCountB", 0)
    c.escrever("AlwaysHealthy", 0)
    return c.gravar(saida, n, parar=lambda d: d.celula("MapB", x, y) < 0x10)


def cenario_n10(c: Sessao, saida, n):
    """N-10: chuva (_RainOn = 1, _RainCnt = 600) com o lençol já em 50: as linhas 50..63 dos dois
    ninhos são inundadas na entrada como _DoWater faria (túnel < 0x20 → 0x4E, terra + 0x2F) e as donas
    e invasoras de grupo 1..11 nessas linhas ficam em M = 17 (_DrownBList/_DrownRList). Cada tick: 1/10
    de subir mais uma linha e 1/100 de cada afogada morrer."""
    c.entrada()
    w = 50
    for mapa in ("MapB", "MapR"):
        m = bytearray(c.mj.ler_variavel(mapa))
        for x in range(64):
            for y in range(w, 64):
                v = m[x * 64 + y]
                m[x * 64 + y] = 0x4E if v < 0x20 else (v + 0x2F) & 0xFF
        seg, off, _ = c.mj.resolver(mapa)
        for k in range(0, 4096, 512):  # pacotes do gdbstub pequenos
            c.mj.escrever_variavel(f"{seg}:{off + k:x}", bytes(m[k:k + 512]), 512)
        c.log(f"{mapa}: linhas {w}..63 inundadas")
    s = c.save()
    for lista in ("B", "R"):
        for i, f in formigas(s, lista):
            if f["Y"] >= w and 1 <= (f["T"] & 0x78) >> 3 <= 11:
                c.escrever_elemento(f"{lista}listM", i, 17)
                c.log(f"{lista}[{i}] em ({f['X']}, {f['Y']}) M <- 17")
    for nome, v in (("WaterTable", w), ("RainOn", 1), ("RainCnt", 600), ("ClearRainFlag", 1)):
        c.escrever(nome, v)
    return c.gravar(saida, n)


def tunel_duplo(s, vida, mapa="MapB"):
    """Duas células de túnel vizinhas (a de baixo e a de cima, y >= 3) livres no mapa de vida."""
    for x in range(1, 63):
        for y in range(3, 62):
            if s.celula(mapa, x, y) < 0x10 and s.celula(mapa, x, y - 1) < 0x10                     and vida[x * 64 + y] == 0 and vida[x * 64 + y - 1] == 0:
                return (x, y), (x, y - 1)
    raise RuntimeError("sem túnel livre")


def cenario_n12(c: Sessao, saida, n):
    """N-12: no ninho preto, na mesma célula de túnel, uma vermelha T 0x92 M 0 num índice menor e uma
    preta T 0x12 M 13 num índice maior (as duas vagas eram ovos; a vermelha fica em _LifeB): luta,
    nuvem e vencedor."""
    c.entrada()
    s = c.save()
    (x, y), _ = tunel_duplo(s, vida_ler(c, "B"))
    ovos = [i for i, f in formigas(s, "B") if (f["T"] & 0x78) == 0]
    i, j = ovos[0], ovos[-1]
    mover_formiga(c, s, "B", j, x, y, m=13, t=0x12)
    mover_formiga(c, s, "B", i, x, y, m=0, t=0x92)
    c.escrever_elemento("BlistS", i, 0)
    c.escrever_elemento("BlistS", j, 0)
    c.log(f"B[{i}] vermelha 0x92 M 0 e B[{j}] preta 0x12 M 13 em ({x}, {y}); _LifeB = 0x92")
    return c.gravar(saida, n)


def cenario_n14(c: Sessao, saida, n):
    """N-14: célula 0x14 nos dois ninhos em (20, 60) e operárias vermelhas cavando (M = 4) nas
    vizinhas do ninho vermelho: ao entrar no 0x14, saem da lista R e entram na B (T 0x90+Sk(8), M 3)."""
    c.entrada()
    x0, y0 = 20, 60
    mapa_escrever(c, "MapB", x0, y0, 0x14)
    mapa_escrever(c, "MapR", x0, y0, 0x14)
    s = c.save()
    rs = [i for i, f in formigas(s, "R") if f["T"] & 0x80 and (f["T"] & 0x78) >> 3 == 2][:4]
    for k, i in enumerate(rs):
        dx, dy = D8[(2 * k) % 8]
        x, y = x0 + dx, y0 + dy
        mapa_escrever(c, "MapR", x, y, 0x00)
        d = (2 * k + 4) % 8  # virada para a passagem
        mover_formiga(c, s, "R", i, x, y, m=4, t=0x90 | d)
    return c.gravar(saida, n)


def cenario_n15(c: Sessao, saida, n):
    """N-15: a amarela no ninho preto com fome (_MeWantFood = 1) num túnel e uma invasora vermelha
    (T 0x90, M 0) logo abaixo, no caminho da saída: ao entrar na célula 0xFF, _DoTroph alimenta a
    amarela (o teste X < 0x80 vale para qualquer formiga do ninho preto)."""
    c.entrada()
    s = c.save()
    # a invasora sai do ninho (9.2) pelo mapa de saída: a amarela fica na célula de cima, no caminho
    (x, y), (ax, ay) = tunel_duplo(s, vida_ler(c, "B"))
    por_amarela(c, 2, ax, ay, vida=20)
    c.escrever("MeWarnHealth", 60)
    c.escrever("MeWantFood", 1)
    acrescentar_formiga(c, "B", x, y, 0x90, 0)
    return c.gravar(saida, n, parar=lambda d: d.ler("MeHealth") > 50)


# ------------------------------------------------------------------------------------------------
# Ovos (ciclo-de-vida.md §10)
# ------------------------------------------------------------------------------------------------

def cenario_c04_c05(c: Sessao, saida, n):
    """C-04 e C-05: na entrada do tick 1791 (o _Cycle 1792 é múltiplo de 128: os ovos avançam), todos
    os ovos pretos com T = 7 e _ModeAuto = 0, _modeLevels[2] = 0 (morte na eclosão), e os vermelhos com
    T = 0x87 e _HealthR = 20 (estratégia vermelha 5: casta pela _CasteTabC)."""
    c.entrada()
    c.dep.ate_gametime(1791)
    s = c.save()
    for i, f in formigas(s, "B"):
        if (f["T"] & 0x78) == 0:
            mover_formiga(c, s, "B", i, f["X"], f["Y"], t=7)
    for i, f in formigas(s, "R"):
        if (f["T"] & 0xF8) == 0x80:
            mover_formiga(c, s, "R", i, f["X"], f["Y"], t=0x87)
    c.escrever("ModeAuto", 0)
    seg, off, _ = c.mj.resolver("modeLevels")
    c.mj.escrever_variavel(f"{seg}:{off + 4:x}", bytes(2), 2)
    c.log("modeLevels[2] <- 0")
    c.escrever("HealthR", 20)
    return c.gravar(saida, n)


def cenario_c06(c: Sessao, saida, n):
    """C-06: _HealthB = 100, _EatCountB = 0; o tick 1711 tem _Cycle 1712 (múltiplo de 16): postura."""
    c.entrada()
    c.escrever("HealthB", 100)
    c.escrever("EatCountB", 0)
    return c.gravar(saida, n)


def cenario_c07(c: Sessao, saida, n):
    """C-07: _HealthB = 0 e _AlwaysHealthy = 0 no tick de postura 1711: Sk(128) <= 0 só com 0."""
    c.entrada()
    c.escrever("AlwaysHealthy", 0)
    c.escrever("HealthB", 0)
    return c.gravar(saida, n)


# ------------------------------------------------------------------------------------------------
# Desenho da aranha na janela de edição (telas/jogo.md, "Aranha na janela de edição")
# ------------------------------------------------------------------------------------------------

# (nome, variáveis escritas antes do redesenho). Cada caso redesenha a vista (Ctrl+seta e volta) com
# o jogo pausado e grava a tela, o estado e o S antes/depois (os palpos sorteiam S1(4) por redesenho).
CASOS_ARANHA = (
    [(f"andando_d{d}", {"SMode": 1, "SpidDir": d, "Scycle": d}) for d in range(8)]
    + [("parada_d0_q5", {"SMode": 0, "SpidDir": 0, "Scycle": 5}),
       ("perseguindo_d3", {"SMode": 2, "SpidDir": 3, "Scycle": 2}),
       ("comendo_d6", {"SMode": 3, "SpidDir": 6, "Scycle": 4}),
       ("fugindo_d5", {"SMode": 4, "SpidDir": 5, "Scycle": 7})]
    + [(f"agonia_q{q}", {"SMode": 5, "SpidDir": 2, "Scycle": q, "DeathCnt": 450}) for q in range(4)]
    + [("jogador_d1", {"SMode": 1, "SpidDir": 1, "Scycle": 3, "MeMode": 1, "MeSMode": 6}),
       ("jogador_d4_parado", {"SMode": 0, "SpidDir": 4, "Scycle": 0, "MeMode": 1, "MeSMode": 0})]
)


def cenario_aranha_desenho(c: Sessao, saida, n):
    """Capturas da aranha na janela de edição (snapshot rico): pausa (Shift+0), vista da superfície,
    aranha no meio da vista em cada caso de CASOS_ARANHA (ou só os de ARANHA_CASOS="a,b"), redesenho
    com Ctrl+Up/Ctrl+Down. Grava NOME.png, NOME.ANT e aranha.json (S antes/depois, vista, aranha)."""
    import os
    pasta = Path(saida).resolve()
    pasta.mkdir(parents=True, exist_ok=True)
    c.teclas("shift+0")
    time.sleep(1)
    c.teclas("ctrl+8")
    time.sleep(2)
    c.mj.sessao_gdb()
    c.escrever("MeMoveMe", 0)
    s = c.save()
    mx, my = s.ler("MapPnt", 0, 2), s.ler("MapPnt", 1, 2)
    # a aranha 1 célula à direita e 1 abaixo do meio (fora do centro exato: não cobre a amarela)
    cx, cy = mx + ct.COLS // 2, my + ct.LINS // 2
    escolhidos = [n.strip() for n in os.environ.get("ARANHA_CASOS", "").split(",") if n.strip()]
    reg = {"vista": [mx, my], "celula": [cx, cy], "casos": {}}
    teclas = ("ctrl+down", "ctrl+up") if my == 0 else ("ctrl+up", "ctrl+down")
    for nome, vs in CASOS_ARANHA:
        if escolhidos and nome not in escolhidos:
            continue
        c.mj.sessao_gdb()
        base = {"SpidOn": 1, "SpidX": cx * 16 + 8, "SpidY": cy * 16 + 8, "MeMode": 0, "MeSMode": 0, "Starg": -2}
        base.update(vs)
        for k, v in base.items():
            c.escrever(k, v)
        s0 = c.ler("SementeS")
        c.teclas(*teclas)
        time.sleep(2.5)
        c.mj.sessao_gdb()
        s1 = c.ler("SementeS")
        ct.tela(c, saida, nome)
        (pasta / f"{nome}.ANT").write_bytes(c.mj.estado())
        st = c.save()
        reg["casos"][nome] = {"escrito": base, "S_antes": s0, "S_depois": s1, "passos_S": ct.passos_s(s0, s1),
                              "MapPnt": [st.ler("MapPnt", 0, 2), st.ler("MapPnt", 1, 2)]}
        c.log(f"{nome}: S {s0} -> {s1} ({reg['casos'][nome]['passos_S']} passos)")
    # sem deixar a amarela "virada aranha" na memória
    c.escrever("MeMode", 0)
    (pasta / "aranha.json").write_text(json.dumps(reg, indent=1) + chr(10), encoding="utf-8")
    return []


def cenario_bichos_desenho(c: Sessao, saida, n):
    """Capturas dos bichos grandes que são tiles do _MapA (formiga-leão, lagarta, bicho-de-conta,
    pedrinha): pausa, superfície e, para cada um achado no mapa, a vista centrada nele. Para garantir a
    pedrinha cuspida, escreve 0x51 numa célula de chão livre ao lado da formiga-leão 0. Grava NOME.png,
    NOME.ANT e bichos.json."""
    pasta = Path(saida).resolve()
    pasta.mkdir(parents=True, exist_ok=True)
    c.teclas("shift+0")
    time.sleep(1)
    c.teclas("ctrl+8")
    time.sleep(2)
    c.mj.sessao_gdb()
    c.escrever("MeMoveMe", 0)
    c.escrever("SpidOn", 0)
    s = c.save()
    lx, ly = c.mj.ler_variavel("LionListX", 10)[0], c.mj.ler_variavel("LionListY", 10)[0]
    alvos = {"formiga_leao": (lx, ly)}
    if s.celula("MapA", lx + 2, ly) < 0x10:
        mapa_escrever(c, "MapA", lx + 2, ly, 0x51)
    cel = lambda f: next(((x, y) for x in range(128) for y in range(64) if f(s.celula("MapA", x, y))), None)
    for nome, f in (("lagarta", lambda v: 0x68 <= v <= 0x6F), ("bicho_de_conta", lambda v: 0x70 <= v <= 0x73),
                    ("pedrinha", lambda v: 0x51 <= v <= 0x53)):
        p = cel(f)
        if p:
            alvos[nome] = p
    reg = {}
    for nome, (x, y) in alvos.items():
        c.mj.sessao_gdb()
        px0 = max(0, min(128 - ct.COLS, x - ct.COLS // 2))
        py0 = max(1, min(63 - ct.LINS, y - ct.LINS // 2))
        c.escrever_elemento("MapPnt", 0, px0, 2)
        c.escrever_elemento("MapPnt", 1, py0, 2)
        c.teclas("ctrl+up", "ctrl+down")
        time.sleep(2.5)
        c.mj.sessao_gdb()
        ct.tela(c, saida, nome)
        (pasta / f"{nome}.ANT").write_bytes(c.mj.estado())
        reg[nome] = {"celula": [x, y]}
        c.log(f"{nome} em ({x}, {y})")
    (pasta / "bichos.json").write_text(json.dumps(reg, indent=1) + chr(10), encoding="utf-8")
    return []


# ------------------------------------------------------------------------------------------------
# Desenho dentro do tick: AutoTrack da aranha do jogador, aranha à vista em _DoAntMoveY e balões
# "Silly" (regras/rng.md, "Desenho dentro do tick")
# ------------------------------------------------------------------------------------------------

# _TickCount @ 1008:18ac (segmento de código 2, GetTickCount() / 55); _MacTickCount (1000:9694) o chama
TICKCOUNT = (2, 6316)
MAC_INI, MAC_FIM = 0x9694, 0x96A5
MAX_TICKCOUNT = 400


def retornos_tickcount(c: Sessao, nseg=3):
    """Endereços (linear, "seg:off") logo depois de cada CALL far a _TickCount no segmento de código
    `nseg` (3 = ANTEDIT: _DrawSpider, _DrawCurBalloons, mensagens do quintal): o padrão 9A off sel na
    memória carregada, com o seletor real do segmento 2."""
    mj = c.mj
    sel3, sel2 = mj.seletor_codigo(nseg), mj.seletor_codigo(TICKCOUNT[0])
    base, limite, _ = mj.descritor(sel3)
    codigo = mj.ler_linear(base, limite + 1)
    padrao = bytes([0x9A]) + struct.pack("<HH", TICKCOUNT[1], sel2)
    out, k = [], codigo.find(padrao)
    while k >= 0:
        out.append((base + k + 5, f"{nseg}:{k + 5:04x}"))
        k = codigo.find(padrao, k + 1)
    return out


def capturar_tick_relogio(c: Sessao, timeout=6):
    """Como Sessao.capturar_tick, mais: as variáveis dos balões "Silly" (cenarios_tick.VARS_BALOES) em
    `antes.Baloes` e `depois.Baloes`, e o valor de cada _TickCount lido pelo desenho dentro do tick
    (`TickCount_lidos`: [{"de": "seg:off", "v": valor}], breakpoints logo depois dos CALL a _TickCount
    do segmento 3, DX:AX lido ali; o laço de espera das animações, que lê o relógio sem parar, não é
    interrompido). O relógio do Mac do vetor (`antes.MacTickCount`) é a primeira leitura de
    _MacTickCount (breakpoint no RETF, só a primeira). Uma caixa modal (o tick não volta em `timeout`
    s) é fechada com Enter, como em contar_no_tick."""
    dep, mj = c.dep, c.mj
    dep.ir_entrada()
    out = {"estado_antes.bin": mj.estado(), "vida_antes.bin": dep.vida()}
    ex = {"antes": {**dep.extras(), "Baloes": ct.ler_baloes(c)}}
    g = mj.sessao_gdb()
    sitios = dict(retornos_tickcount(c))
    mac_ret = mj.linear_codigo(*qb.MACTICKCOUNT_RETF)
    lidos, mac, dialogos = [], [], 0
    armados = set(sitios) | {dep.retorno, mac_ret}
    for a in armados:
        g.breakpoint(a)
    try:
        while True:
            try:
                g.continuar_ate_parar(timeout)
            except RuntimeError:
                g.drenar()
                g.s.sendall(b"$c#63")
                time.sleep(0.2)
                c.q.cmd("send-key", keys=[{"type": "qcode", "data": "ret"}], **{"hold-time": 150})
                dialogos += 1
                if dialogos > 6:
                    raise
                g.s.settimeout(60)
                while not g._e_parada(g._proximo()):
                    pass
                g.s.settimeout(30)
            pc, regs = dep.pc()
            if pc == dep.retorno:
                break
            v = (regs["EDX"] & 0xFFFF) << 16 | (regs["EAX"] & 0xFFFF)
            if pc == mac_ret:
                mac.append(v)
                lidos.append({"de": "_MacTickCount", "v": v})
            elif pc in sitios:
                lidos.append({"de": sitios[pc], "v": v})
            else:
                c.log(f"parada inesperada em {pc:#x}")
                continue
            # passa por cima: desliga, um passo, religa (o de _MacTickCount fica desligado)
            g.breakpoint(pc, False)
            if pc == mac_ret:
                armados.discard(pc)
                continue
            g.drenar()
            g.s.sendall(b"$s#73")
            while not g._e_parada(g._proximo()):
                pass
            g.breakpoint(pc)
    finally:
        for a in armados:
            g.breakpoint(a, False)
    out["estado_depois.bin"] = mj.estado()
    out["vida_depois.bin"] = dep.vida()
    ex["depois"] = {**dep.extras(), "Baloes": ct.ler_baloes(c)}
    ex["antes"]["MacTickCount"] = mac[0] if mac else None
    ex["MacTickCount_lidos"] = mac[:1]
    ex["TickCount_lidos"] = lidos
    if dialogos:
        ex["dialogos"] = dialogos
    out["extras.json"] = ex
    return out


def gravar_relogio(c: Sessao, saida, n, parar=None):
    """Como Sessao.gravar, com capturar_tick_relogio."""
    nomes = []
    for _ in range(n):
        arqs = capturar_tick_relogio(c)
        gt = arqs["extras.json"]["antes"]["GameTime"]
        pasta = Path(saida).resolve() / f"tick_{gt:06d}"
        for nome_arq, conteudo in arqs.items():
            qb.gravar(pasta / nome_arq, conteudo)
        nomes.append(pasta.name)
        lidos = arqs["extras.json"]["TickCount_lidos"]
        c.b.log(f"{pasta.name}: {[(x['de'], x['v']) for x in lidos][:40]}")
        if parar and parar(ct.Save(arqs["estado_depois.bin"])):
            break
    return nomes


def cenario_aranha_autotrack(c: Sessao, saida, n):
    """A aranha do jogador com AutoTrack (_OptionStates[0] = 1): _MoveSpider chama _GotoMyAnt a cada
    passo. Anda 8 células (_SMode 1: o desenho de dentro do tick não sorteia), depois o modo alvo
    (tecla 1 e clique numa preta 10 células adiante): persegue (_SMode 2, palpos S1(4) a cada vez que
    a vista anda) e come (_SMode 3)."""
    x, y = virar_aranha(c)
    c.escrever_elemento("OptionStates", 0, 1, 2)
    nomes = []
    c.clicar_celula(x + 8, y, pausado=True)
    nomes += gravar_relogio(c, saida, n, parar=lambda d: d.ler("SpidX") >> 4 == x + 8)
    c.entrada()
    s = c.save()
    sx, sy = c.ler("SpidX") >> 4, c.ler("SpidY") >> 4
    vida_a = vida_ler(c, "A")
    alvo = next((sx + dx, sy) for dx in range(10, 24) if livre_a(s, vida_a, sx + dx, sy))
    mover_formiga(c, s, "A", pretas_da_superficie(s, 1)[0], *alvo, m=17)
    c.teclas("1")
    time.sleep(1)
    c.clicar_celula(*alvo)
    time.sleep(1)
    c.entrada()
    c.log(f"alvo {alvo}: _SMode = {c.ler('SMode')}, _MeSMode = {c.ler('MeSMode')}")
    comeu = []

    def parar(d):
        if d.ler("SMode") == 3:
            comeu.append(1)
        return len(comeu) >= 3
    nomes += gravar_relogio(c, saida, n, parar=parar)
    return nomes


def cenario_aranha_movey(c: Sessao, saida, n):
    """Aranha à vista durante _DoAntMoveY, com "Silly": a aranha come parada (_SMode 3, _EatCnt 300) a
    duas células de um buraco da superfície e a amarela, ao lado do buraco, recebe o clique nele. O
    redesenho de antes de entrar no buraco sorteia os palpos e o balão da aranha (_LastSMode posto em 0,
    diferente de 3: prazo novo com o _TickCount do momento); depois, o _GotoMyAnt leva a vista ao ninho.
    Depois um clique na superfície ao lado do buraco: sair do ninho (o redesenho de antes da saída é no
    ninho; o _GotoMyAnt de depois volta à superfície, com a aranha à vista)."""
    c.teclas("ctrl+8")
    time.sleep(1)
    c.entrada()
    esperar_compactacao(c)
    s = c.save()
    vida_a = vida_ler(c, "A")
    buracos = [(x, y) for x in range(4, 120) for y in range(4, 58) if s.celula("MapA", x, y) == 0x50]
    c.log(f"buracos da superfície: {buracos}")
    livre = lambda x, y: 0 < x < 127 and 0 < y < 63 and s.celula("MapA", x, y) < 0x48 and vida_a[x * 64 + y] in (0, 0xFF)
    escolha = next((x, y, ax, ay, sx, sy) for x, y in buracos for ax, ay in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1))
                   if livre(ax, ay) for sx, sy in ((x - 3, y + 2), (x + 3, y + 2), (x - 3, y - 2), (x + 3, y - 2), (x, y + 3), (x, y - 3))
                   if livre(sx, sy))
    bx, by, ax, ay, sx, sy = escolha
    por_amarela(c, 1, ax, ay, vida=100)
    aranha_em(c, sx, sy, modo=3, direcao=2)
    c.escrever("EatCnt", 300)
    c.escrever_elemento("OptionStates", 5, 1, 2)
    c.escrever("LastSMode", 0, 2)
    c.log(f"buraco ({bx}, {by}); amarela ({ax}, {ay}); aranha ({sx}, {sy})")
    nomes = []
    c.clicar_celula(bx, by, pausado=True)
    # o redesenho do timer entre o clique e este tick já viu _SMode 3: de novo 0, na entrada
    c.escrever("LastSMode", 0, 2)
    nomes += gravar_relogio(c, saida, n, parar=lambda d: d.ler("MePlane") != 1)
    c.entrada()
    c.teclas("ctrl+8")
    time.sleep(1)
    c.entrada()
    c.escrever("EatCnt", 300)
    c.escrever("LastSMode", 0, 2)
    c.clicar_celula(ax, ay, pausado=True)
    c.escrever("LastSMode", 0, 2)
    nomes += gravar_relogio(c, saida, n, parar=lambda d: d.ler("MePlane") == 1)
    return nomes


def cenario_morte_aranha_silly(c: Sessao, saida, n):
    """Como `amarela_aranha` (a aranha come a amarela: _YellowDeath(1) com _AnimYellowFight), com
    "Silly" e a música desligada (todos os quadros): o balão da aranha sorteia nos quadros com o
    _TickCount de cada um. Confere se o desenho de dentro do tick usa o relógio do começo do tick."""
    c.entrada()
    esperar_compactacao(c)
    s = c.save()
    vida_a = vida_ler(c, "A")
    x, y = lugar_aberto(s, vida_a, raio=2)
    por_amarela(c, 1, x + 1, y, vida=100)
    aranha_em(c, x, y, modo=2, direcao=2)
    c.escrever("StargLife", 0xFF)
    c.escrever("Starg", -1)
    c.escrever_elemento("OptionStates", 5, 1, 2)
    c.escrever_elemento("OptionStates", 1, 0, 2)
    # sem Events: a caixa da aranha (_SpiderDialog) pararia o relógio no meio do tick
    c.escrever_elemento("OptionStates", 3, 0, 2)
    c.escrever("LastSMode", 3, 2)
    return gravar_relogio(c, saida, n)


# ------------------------------------------------------------------------------------------------
# Mini mapa aberto: as teclas chegam ao jogo? (telas/mapa.md §7)
# ------------------------------------------------------------------------------------------------



def cenario_mini_teclas(c: Sessao, saida, n):
    """Abre o mini mapa (clique no ribToYellow, solto no botão: fica aberto) e aperta, com ele aberto,
    Shift+3 (velocidade), Ctrl+G (AutoTrack), Ctrl+Down (rolagem da janela de edição), Ctrl+9 (ninho
    preto) e Shift+0 (pausa); depois fecha com um clique fora. Em cada passo lê _GameSpeed,
    _GamePaused, _OptionStates[0], _MapPnt e _MapPlane e captura a tela. Grava mini_teclas.json."""
    pasta = Path(saida).resolve()
    pasta.mkdir(parents=True, exist_ok=True)
    c.teclas("ctrl+8")
    time.sleep(1.5)
    reg = []

    def anotar(rotulo):
        v = {"passo": rotulo, "GameSpeed": c.ler("GameSpeed"), "GamePaused": c.ler("GamePaused"),
             "AutoTrack": int.from_bytes(c.mj.ler_variavel("OptionStates", 12)[:2], "little"),
             "MapPnt": [c.ler("MapPnt", None) & 0xFFFF, c.ler("MapPnt", None) >> 16], "MapPlane": c.ler("MapPlane"),
             "GameTime": c.ler("GameTime")}
        reg.append(v)
        ct.tela(c, saida, rotulo)
        c.log(str(v))

    import bancada_mapa as bm
    c.dep.retomar()
    time.sleep(0.5)
    anotar("antes")
    m = bm.Mapa(c, saida)
    bm.abrir_mini(m)
    time.sleep(2)
    anotar("mini_aberto")
    for tecla in ("shift+3", "ctrl+g", "ctrl+down", "ctrl+9", "shift+0"):
        c.teclas(tecla)
        time.sleep(1.5)
        anotar(tecla.replace("+", "_"))
    m.c.dep.retomar()
    m.ir(560, 420)
    m.botao(True)
    m.botao(False)
    time.sleep(2)
    anotar("fechado")
    time.sleep(2)
    anotar("fechado_2s")
    (pasta / "mini_teclas.json").write_text(json.dumps(reg, indent=1) + chr(10), encoding="utf-8")
    return []


VIDA_RICO = ("vagas mortas (T = 0) paradas nas células dos buracos e formigas que dividem célula: a reconstrução "
             "de _DoLoadInitializations (o menor índice vence) não dá a vida em memória (ver colonia_5min); a "
             "preparação também escreve marcas no mapa de vida")


def registrar(nome, descricao):
    """Põe as chaves de exceção no indice.json de tests/referencias/ticks/NOME e o registra em
    "cenarios" do índice da raiz."""
    raiz = Path(__file__).resolve().parent.parent / "tests" / "referencias" / "ticks"
    arq = raiz / nome / "indice.json"
    ind = json.loads(arq.read_text(encoding="utf-8"))
    ind.setdefault("pendentes", {})
    ind.setdefault("vida_nao_reconstruivel", {"*": VIDA_RICO})
    ind["gerador"] = "tools/cenarios_regras.py"
    if descricao:
        ind["descricao"] = descricao
    arq.write_text(json.dumps(ind, indent=1, ensure_ascii=False) + chr(10), encoding="utf-8")
    ri = raiz / "indice.json"
    r = json.loads(ri.read_text(encoding="utf-8"))
    if nome not in r["cenarios"]:
        r["cenarios"].append(nome)
        ri.write_text(json.dumps(r, indent=1, ensure_ascii=False) + chr(10), encoding="utf-8")
    print(f"{nome}: {len(ind['ticks'])} ticks registrados")


def selecionar_lote(arq, bruto_raiz):
    """[{"bruto": ..., "destino": ..., "descricao": ..., "ticks": ["1793", ...]}, ...]: seleciona de
    bruto_raiz/BRUTO para tests/referencias/ticks/DESTINO (apagando o que havia) e registra."""
    import shutil
    raiz = Path(__file__).resolve().parent.parent / "tests" / "referencias" / "ticks"
    for item in json.loads(Path(arq).read_text(encoding="utf-8")):
        destino = raiz / item["destino"]
        if destino.exists():
            shutil.rmtree(destino)
        ct.selecionar(Path(bruto_raiz) / item["bruto"], destino, item["ticks"], item["descricao"])
        registrar(item["destino"], item["descricao"])


if __name__ == "__main__":
    if len(sys.argv) >= 4 and sys.argv[1] == "selecionar_lote":
        selecionar_lote(sys.argv[2], sys.argv[3])
        sys.exit(0)
    if len(sys.argv) >= 3 and sys.argv[1] == "registrar":
        registrar(sys.argv[2], sys.argv[3] if len(sys.argv) > 3 else "")
        sys.exit(0)
    for nome, f in list(globals().items()):
        if nome.startswith("cenario_"):
            setattr(ct, nome, f)
    ct.main()
