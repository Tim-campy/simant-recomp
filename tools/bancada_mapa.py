"""Capturas do original para a janela do mapa, o mini mapa e a troca mapa <-> quintal
(docs/spec/telas/mapa.md). Usa a Sessao de cenarios_tick (snapshot "rico", jogo pausado com Shift+0).

    set SIMANT_RAIZ=...
    python tools/bancada_mapa.py CENARIO SAIDA [--overlay OV] [--snapshot rico]

Cenários (função cenario_<nome>): cada um grava PNGs e um `NOME.json` com o que leu da memória
(_MapPnt, _MapPlane, _MapMode, _GamePaused...) em SAIDA. Coordenadas: tela 640x480 da bancada; a
faixa 34 fica em (layout + (0, 21)).
"""
import argparse
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import qemu_bancada as qb  # noqa: E402
from cenarios_tick import Sessao, tela  # noqa: E402

FAIXA_DY = 21


def centro(r):
    """Centro, na tela, de um objeto da faixa 34 (retângulo do layout)."""
    return (r[0] + r[2]) // 2, (r[1] + r[3]) // 2 + FAIXA_DY


RIB_TO_YELLOW = centro((207, 41, 232, 59))
RIB_TO_YARD = centro((13, 21, 52, 49))
RIB_TO_BNEST = centro((61, 21, 89, 49))
RIB_TO_SURFACE = centro((133, 21, 172, 49))


class Mapa:
    def __init__(self, c: Sessao, saida):
        self.c, self.saida = c, Path(saida).resolve()
        self.saida.mkdir(parents=True, exist_ok=True)
        self.reg = []
        self.pos = None

    def ir(self, x, y):
        """Leva o mouse a (x, y) da tela (a VM precisa estar rodando)."""
        q = self.c.q
        if self.pos is None:
            qb.mouse_canto(q)
            self.pos = (0, 0)
        qb.mover_rel(q, x - self.pos[0], y - self.pos[1])
        self.pos = (x, y)
        time.sleep(0.3)

    def botao(self, baixo, botao="left"):
        self.c.q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": baixo, "button": botao}}])
        time.sleep(0.08)

    def tecla(self, qcode, baixa):
        self.c.q.cmd("input-send-event", events=[{"type": "key", "data": {"down": baixa, "key": {"type": "qcode", "data": qcode}}}])
        time.sleep(0.1)

    def clicar(self, x, y, duplo=False):
        self.c.dep.retomar()
        self.ir(x, y)
        for _ in range(2 if duplo else 1):
            self.botao(True)
            self.botao(False)

    def ler(self, *nomes):
        self.c.mj.sessao_gdb()
        v = {}
        for n in nomes:
            if n == "MapPnt":
                s = self.c.save()
                v[n] = [s.ler("MapPnt", 0, 2), s.ler("MapPnt", 1, 2)]
            elif ":" in n:
                nome, tam = n.split(":")
                v[nome] = self.c.ler(nome, int(tam), sinal=True)
            else:
                v[n] = self.c.ler(n)
        self.c.dep.retomar()
        return v

    def foto(self, nome, *nomes, espera=1.0):
        self.c.dep.retomar()
        time.sleep(espera)
        tela(self.c, self.saida, nome)
        v = self.ler(*nomes) if nomes else {}
        self.reg.append({"foto": nome, **v})
        self.c.log(f"{nome}: {v}")
        return v

    def gravar(self, arquivo):
        (self.saida / arquivo).write_text(json.dumps(self.reg, indent=1) + "\n", encoding="utf-8")


LEITURAS = ("MapPnt", "MapPlane", "MapMode", "GamePaused")


def cenario_mini1(c: Sessao, saida):
    """Mini mapa (ribToYellow, _OpenMiniMapWin): apertar e segurar, soltar fora do mini mapa."""
    m = Mapa(c, saida)
    c.teclas("shift+0")
    m.foto("m00_inicio", *LEITURAS)
    c.dep.retomar()
    m.ir(*RIB_TO_YELLOW)
    m.botao(True)
    m.foto("m01_segurando", *LEITURAS, espera=1.5)
    m.ir(RIB_TO_YELLOW[0], 440)
    m.botao(False)
    m.foto("m02_solto_fora", *LEITURAS)
    m.gravar("mini1.json")


def abrir_mini(m: Mapa):
    """Aperta o ribToYellow, leva o mouse para baixo (fora do mini mapa) e solta: fica aberto."""
    m.c.dep.retomar()
    m.ir(*RIB_TO_YELLOW)
    m.botao(True)
    time.sleep(1)
    m.ir(RIB_TO_YELLOW[0], 440)
    m.botao(False)
    time.sleep(1)


def arrastar(m: Mapa, de, para, passos=4):
    m.c.dep.retomar()
    m.ir(*de)
    m.botao(True)
    for k in range(1, passos + 1):
        m.ir(de[0] + (para[0] - de[0]) * k // passos, de[1] + (para[1] - de[1]) * k // passos)
    m.botao(False)


def cenario_mini2(c: Sessao, saida):
    """Mini mapa com o jogo rodando: superfície, clique, arrasto, clique fora (fecha), arrasto do
    botão até dentro do mini mapa (_OpenMapYard) e o mini mapa no ninho preto. Grava o estado com o
    mini mapa aberto (o jogo fica pausado enquanto ele está aberto) em mini_sup.ANT e mini_ninho.ANT."""
    m = Mapa(c, saida)
    m.clicar(*RIB_TO_SURFACE)
    m.foto("m10_superficie", *LEITURAS)
    abrir_mini(m)
    m.foto("m11_mini_sup", *LEITURAS)
    c.mj.sessao_gdb()
    (m.saida / "mini_sup.ANT").write_bytes(c.mj.estado())
    m.clicar(230, 150)
    m.foto("m12_clique", *LEITURAS)
    arrastar(m, (180, 130), (260, 170))
    m.foto("m13_arrasto", *LEITURAS)
    m.clicar(400, 300)
    m.foto("m14_fora", *LEITURAS)
    # apertar o botão, levar até dentro do mini mapa e soltar ali
    c.dep.retomar()
    m.ir(*RIB_TO_YELLOW)
    m.botao(True)
    time.sleep(1)
    m.ir(219, 150)
    m.botao(False)
    m.foto("m15_solto_dentro", *LEITURAS)
    m.clicar(*RIB_TO_BNEST)
    time.sleep(1)
    abrir_mini(m)
    m.foto("m16_mini_ninho", *LEITURAS)
    c.mj.sessao_gdb()
    (m.saida / "mini_ninho.ANT").write_bytes(c.mj.estado())
    m.clicar(400, 300)
    m.foto("m17_fechado", *LEITURAS)
    m.gravar("mini2.json")


# área do mapa (janela 1) na tela da bancada, na posição de origem
MAPA_X, MAPA_Y = 120, 157


def celula(cx, cy):
    return MAPA_X + 4 * cx + 2, MAPA_Y + 4 * cy + 2


AMARELA = ("MeGoalX:2", "MeGoalY:2", "MeGoalPlane:2", "MeMoveMe:2", "MeCmd:2", "MePlane:2")


def cenario_mapa1(c: Sessao, saida):
    """Janela do mapa: Shift+clique (ordem), clique, duplo clique (janela de edição na frente),
    arrasto, Shift+duplo clique; depois o mapa movido e a troca com o quintal (ribToYard, Patch)."""
    m = Mapa(c, saida)
    c.teclas("ctrl+m")
    m.clicar(*RIB_TO_SURFACE)
    m.foto("p00_mapa", *LEITURAS, *AMARELA)
    c.dep.retomar()
    m.ir(*celula(60, 30))
    m.tecla("shift", True)
    m.botao(True)
    m.botao(False)
    m.tecla("shift", False)
    m.foto("p01_shift_clique", *LEITURAS, *AMARELA, espera=0.5)
    m.clicar(*celula(30, 40))
    m.foto("p02_clique", *LEITURAS, *AMARELA)
    m.clicar(*celula(90, 20), duplo=True)
    m.foto("p03_duplo", *LEITURAS, *AMARELA)
    c.teclas("ctrl+m")
    arrastar(m, celula(10, 10), celula(100, 50))
    m.foto("p04_arrasto", *LEITURAS, *AMARELA)
    c.dep.retomar()
    m.ir(*celula(70, 35))
    m.tecla("shift", True)
    for _ in range(2):
        m.botao(True)
        m.botao(False)
    m.tecla("shift", False)
    m.foto("p05_shift_duplo", *LEITURAS, *AMARELA, espera=0.5)
    # troca com o quintal, com o mapa movido
    arrastar(m, (300, 146), (250, 180))
    m.foto("p06_movido", *LEITURAS)
    m.clicar(*RIB_TO_YARD)
    m.foto("p07_quintal", *LEITURAS)
    m.clicar(27, 56)
    m.foto("p08_patch", *LEITURAS)
    m.gravar("mapa1.json")


def cenario_mini3(c: Sessao, saida):
    """Posição do mini mapa em aberturas seguidas: três na superfície e três no ninho preto (cada
    uma aberta com o botão e fechada com um clique fora)."""
    m = Mapa(c, saida)
    for plano, botao in (("sup", RIB_TO_SURFACE), ("ninho", RIB_TO_BNEST)):
        m.clicar(*botao)
        time.sleep(1)
        for k in range(3):
            abrir_mini(m)
            m.foto(f"r_{plano}_{k}", *LEITURAS)
            m.clicar(400, 300)
            time.sleep(0.5)
    m.gravar("mini3.json")


def shift_clique(m: Mapa, x, y, espera=0.6):
    m.c.dep.retomar()
    m.ir(x, y)
    m.tecla("shift", True)
    time.sleep(espera)
    m.botao(True)
    m.botao(False)
    time.sleep(espera)
    m.tecla("shift", False)


def cenario_mapa2(c: Sessao, saida):
    """Shift+clique simples no mapa, com espera entre a tecla e o botão: na superfície e no ninho
    preto (dentro e fora das 64 colunas do meio)."""
    m = Mapa(c, saida)
    c.teclas("ctrl+m")
    m.clicar(*RIB_TO_SURFACE)
    m.foto("q00_mapa", *LEITURAS, *AMARELA)
    shift_clique(m, *celula(60, 30))
    m.foto("q01_shift_clique", *LEITURAS, *AMARELA, espera=0.3)
    m.clicar(*RIB_TO_BNEST)
    m.foto("q02_ninho", *LEITURAS, *AMARELA)
    shift_clique(m, *celula(10, 30))
    m.foto("q03_shift_faixa", *LEITURAS, *AMARELA, espera=0.3)
    shift_clique(m, *celula(32 + 20, 30))
    m.foto("q04_shift_ninho", *LEITURAS, *AMARELA, espera=0.3)
    m.gravar("mapa2.json")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("cenario")
    ap.add_argument("saida")
    ap.add_argument("--overlay", default="re_trabalho/qemu/ag_mapa_r.qcow2")
    ap.add_argument("--snapshot", default="rico")
    a = ap.parse_args()
    c = Sessao(a.overlay, a.snapshot)
    try:
        globals()[f"cenario_{a.cenario}"](c, a.saida)
    finally:
        c.fechar()


if __name__ == "__main__":
    main()
