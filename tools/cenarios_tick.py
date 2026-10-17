"""Cenários de vetores de tick que precisam de preparação programada (escrever variáveis na entrada de um
tick, cliques calculados a partir do estado, gravação longa para escolher janelas).

Gravação bruta (fora do repositório) e seleção de janelas para tests/referencias/ticks/<cenário>/:

    set SIMANT_RAIZ=...
    python tools/cenarios_tick.py gravar CENARIO DIR_BRUTO [--overlay OV] [--snapshot rico] [--ticks N]
    python tools/cenarios_tick.py eventos DIR_BRUTO                   lista, por tick, o que mudou de interessante
    python tools/cenarios_tick.py selecionar DIR_BRUTO DESTINO TICK[-TICK] ... [--descricao TEXTO]

Cenários (função cenario_<nome>; cada uma prepara e devolve a lista de passos feitos, gravada no índice):
    chuva     _GlobalKey = 0xA8 na entrada de um tick (_DoSimYard liga a chuva), depois grava N ticks
    revoada   formigas da superfície em modo 15 (Take off) e _FlyAwayB/R = 3 na entrada de um tick
    menino    menino cortando a grama (_BoyHere = 3, _BoyIsMowing = 1) posto no patch atual (_CurYardPnt)
    livre     só grava N ticks (aranha, formiga-leão, menino: escolher janelas com "eventos")
    fast, ultra   velocidade Shift+3 / Shift+4 e grava N ticks
    amarela_sup   a amarela sai do ninho e vai comer (duplo clique numa comida), volta a um buraco e luta com
              uma vermelha (duplo clique), com cliques calculados a partir de _MapPnt
    luta      duplo clique numa vermelha (ordem 3) logo depois de uma compactação da lista A
Protocolo e formato: docs/spec/regras/testes-tick.md. Snapshot "rico": tests/referencias/roteiros/qemu_snapshot_rico.txt.
"""
import argparse
import datetime
import json
import shutil
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import qemu_bancada as qb  # noqa: E402
from ant_editar import Save  # noqa: E402

# área do mapa na janela de edição, tela 640x480 da bancada (docs/spec/telas/jogo.md)
MAPA_X0, MAPA_Y0, TILE, COLS, LINS = 24, 158, 16, 25, 19


class Sessao:
    def __init__(self, overlay, snapshot):
        self.b = qb.Bancada(overlay)
        self.b.executar(f"snapshot_carregar {snapshot}")
        self.dep, self.mj, self.q = self.b.dep, self.b.mj, self.b.q
        self.dep.ir_entrada()
        self.inicio = self.dep.extras()
        self.passos = []

    def log(self, msg):
        self.b.log(msg)
        self.passos.append(msg)

    # memória
    def ler(self, nome, tam=None, sinal=False):
        return int.from_bytes(self.mj.ler_variavel(nome, tam), "little", signed=sinal)

    def escrever(self, nome, valor, tam=None):
        n = self.mj.resolver(nome, tam)[2]
        self.mj.escrever_variavel(nome, (valor & ((1 << 8 * n) - 1)).to_bytes(n, "little"), tam)
        self.log(f"{nome} <- {valor}")

    def escrever_elemento(self, bloco, i, valor, largura=1):
        seg, off, _ = self.mj.resolver(bloco)
        self.mj.escrever_variavel(f"{seg}:{off + i * largura:x}", valor.to_bytes(largura, "little"), largura)

    def save(self):
        return Save(self.mj.estado())

    # execução
    def entrada(self):
        self.dep.ir_entrada()

    def avancar(self, n):
        for _ in range(n):
            self.dep.ir_saida()
            self.dep.ir_entrada()

    def livre(self, seg):
        self.dep.retomar()
        time.sleep(seg)

    # entrada do usuário (a VM precisa estar rodando)
    def clicar_celula(self, x, y, duplo=False, pausado=False):
        """Clique na célula (x, y) do plano mostrado, pondo antes _MapPnt para que ela fique no centro.
        Com `pausado`, a VM fica parada (na entrada de um tick): os eventos do mouse ficam na fila do
        dispositivo e o Windows os trata entre os ticks seguintes, que podem ser gravados."""
        px0 = max(0, min(128 - COLS, x - COLS // 2)) if self.ler("MapPlane") < 2 else max(0, min(64 - COLS, x - COLS // 2))
        py0 = max(0, min(64 - LINS, y - LINS // 2))
        self.escrever_elemento("MapPnt", 0, px0, 2)
        self.escrever_elemento("MapPnt", 1, py0, 2)
        self.dep.retomar()
        time.sleep(0.5)
        qb.mouse_canto(self.q)
        qb.mover_rel(self.q, MAPA_X0 + (x - px0) * TILE + 8, MAPA_Y0 + (y - py0) * TILE + 8)
        time.sleep(0.2)
        if pausado:
            # o mouse já está sobre a célula; os botões vão com a VM correndo até a próxima entrada de
            # _DoAntSim (o QMP não aceita eventos com a VM parada), e o Windows trata o clique entre os
            # ticks seguintes, que já podem ser gravados
            g = self.mj.sessao_gdb()
            entrada = self.dep.entrada()
            g.breakpoint(entrada)
            g.drenar()
            g.s.sendall(b"$c#63")
            self.botoes(duplo)
            while not g._e_parada(g._proximo()):
                pass
            g.breakpoint(entrada, False)
        else:
            self.botoes(duplo)
        self.log(f"{'duplo ' if duplo else ''}clique na célula ({x}, {y}) com _MapPnt = ({px0}, {py0})")

    def botoes(self, duplo):
        for _ in range(2 if duplo else 1):
            for down in (True, False):
                self.q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": down, "button": "left"}}])
                time.sleep(0.06)

    def teclas(self, *combos):
        self.dep.retomar()
        for c in combos:
            self.b.executar(f"teclas {c}")
        self.log(f"teclas {' '.join(combos)}")

    def capturar_tick(self):
        """Como Depurador.capturar_tick, mas um tick que abre uma caixa de mensagem modal (morte da amarela,
        fim de jogo...) não volta sozinho: depois de 15 s sem chegar ao retorno de _DoAntSim, captura a
        tela, aperta Enter e espera o retorno de novo (até 5 vezes). O relógio lido até a caixa é
        guardado; o que a caixa lê depois não."""
        dep, mj = self.dep, self.mj
        dep.ir_entrada()
        out = {"estado_antes.bin": mj.estado(), "vida_antes.bin": dep.vida()}
        ex = {"antes": dep.extras()}
        relogio = []
        try:
            dep.ir_saida(timeout=15, relogio=relogio)
        except RuntimeError as err:
            gt = ex["antes"]["GameTime"]
            for k in range(5):
                self.log(f"tick {gt} parado ({err}); tela em re_trabalho/cen/dialogo_{gt}_{k}.png e Enter")
                self.b.executar(f"capturar re_trabalho/cen/dialogo_{gt}_{k}.png")
                dep.retomar()
                # caixa de morte da amarela: "Click here to continue" no centro da janela de edição
                self.b.executar("clicar_em 320 330")
                time.sleep(1)
                self.b.executar("teclas ret")
                time.sleep(2)
                try:
                    dep._rodar_ate(dep.retorno, 15)
                    break
                except RuntimeError as e2:
                    err = e2
            else:
                raise
            ex["dialogos"] = k + 1
        out["estado_depois.bin"] = mj.estado()
        out["vida_depois.bin"] = dep.vida()
        ex["depois"] = dep.extras()
        ex["antes"]["MacTickCount"] = relogio[0] if relogio else None
        ex["MacTickCount_lidos"] = relogio
        out["extras.json"] = ex
        return out

    def gravar(self, saida, n, parar=None):
        """Grava n ticks consecutivos em saida/tick_NNNNNN; `parar(save_depois)` encerra antes."""
        nomes = []
        for _ in range(n):
            arqs = self.capturar_tick()
            gt = arqs["extras.json"]["antes"]["GameTime"]
            pasta = Path(saida).resolve() / f"tick_{gt:06d}"
            for nome_arq, conteudo in arqs.items():
                qb.gravar(pasta / nome_arq, conteudo)
            nomes.append(pasta.name)
            self.b.log(f"{pasta.name} (relógio {arqs['extras.json']['MacTickCount_lidos']})")
            if parar and parar(Save(arqs["estado_depois.bin"])):
                break
        return nomes

    def gravar_longo(self, saida, n, a_cada):
        """Sequência longa compacta: estado e vida na entrada do 1º tick (inicio/), os extras de cada tick
        (extras.json, uma lista) e o estado e a vida depois de cada `a_cada` ticks (depois_NNNN/)."""
        saida = Path(saida).resolve()  # absoluto: qb.gravar resolve relativos no SIMANT_RAIZ
        self.dep.ir_entrada()
        qb.gravar(saida / "inicio" / "estado.bin", self.mj.estado())
        qb.gravar(saida / "inicio" / "vida.bin", self.dep.vida())
        lista = []
        for k in range(1, n + 1):
            antes = self.dep.extras()
            relogio = []
            self.dep.ir_saida(relogio=relogio)
            depois = self.dep.extras()
            antes["MacTickCount"] = relogio[0] if relogio else None
            lista.append({"antes": antes, "depois": depois, "MacTickCount_lidos": relogio})
            if k % a_cada == 0 or k == n:
                qb.gravar(saida / f"depois_{k:04d}" / "estado.bin", self.mj.estado())
                qb.gravar(saida / f"depois_{k:04d}" / "vida.bin", self.dep.vida())
                self.b.log(f"{k} ticks (GameTime {depois['GameTime']})")
            self.dep.ir_entrada()
        (saida / "extras.json").write_text(json.dumps(lista, separators=(",", ":")) + "\n", encoding="utf-8")
        return [f"{len(lista)} ticks a partir de GameTime {lista[0]['antes']['GameTime']}"]

    def indice(self, saida, ticks, snapshot, descricao):
        ind = {
            "_comentario": "Vetores de tick do SimAnt original (bancada QEMU). Ver docs/spec/regras/testes-tick.md.",
            "gerado_em": datetime.date.today().isoformat(),
            "gerador": "tools/cenarios_tick.py",
            "snapshot": snapshot,
            "descricao": descricao,
            "passos": self.passos,
            "primeira_entrada": self.inicio,
            "seletores_dados": {str(k): f"{v:#06x}" for k, v in self.mj.localizar().items()},
            "retorno_DoAntSim": f"{self.dep.retorno_far[0]:04x}:{self.dep.retorno_far[1]:04x}",
            "extras": {k: list(v) for k, v in qb.EXTRAS.items()},
            "ticks": ticks,
        }
        Path(saida).mkdir(parents=True, exist_ok=True)
        (Path(saida) / "indice.json").write_text(json.dumps(ind, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")

    def fechar(self):
        self.b.fechar()


def formigas(s: Save, lista):
    return [(i, f) for i, f in enumerate(s.formigas(lista)) if f["T"] != 0]


# ------------------------------------------------------------------------------------------------
# Cenários
# ------------------------------------------------------------------------------------------------

def cenario_chuva(c: Sessao, saida, n):
    c.entrada()
    c.escrever("GlobalKey", 0xA8, 2)
    return c.gravar(saida, n)


def cenario_revoada(c: Sessao, saida, n):
    c.entrada()
    s = c.save()
    pretas = [i for i, f in formigas(s, "A") if not f["T"] & 0x80][:6]
    verms = [i for i, f in formigas(s, "A") if f["T"] & 0x80][:12]
    for i in pretas + verms:
        c.escrever_elemento("AlistM", i, 15)
    c.log(f"AlistM <- 15 nas formigas da superfície {pretas} (pretas) e {verms} (vermelhas)")
    c.escrever("FlyAwayB", 3)
    c.escrever("FlyAwayR", 3)
    return c.gravar(saida, n)


def cenario_menino(c: Sessao, saida, n):
    c.entrada()
    p = c.ler("CurYardPnt")
    px, py = p & 0xFFFF, p >> 16
    # _BoyPx = (_BoyY + _BoyX - 200) / 28, _BoyPy = (_BoyY - 38) / 10 (quintal.md §3): meio do patch
    boy_y = 38 + py * 10 + 5
    boy_x = px * 28 + 14 + 200 - boy_y
    c.log(f"patch atual _CurYardPnt = ({px}, {py})")
    for nome, v in (("NodeNum", 0x26), ("BoyHere", 3), ("BoyIsMowing", 1), ("BoyX", boy_x), ("BoyY", boy_y),
                    ("BoyPx", px), ("BoyPy", py), ("LastMowX", px), ("LastMowY", py), ("FootHere", 1)):
        c.escrever(nome, v)
    return c.gravar(saida, n)


def cenario_livre(c: Sessao, saida, n):
    c.entrada()
    return c.gravar(saida, n)


def cenario_longo(c: Sessao, saida, n):
    """n ticks consecutivos em formato compacto (estado a cada 50); ver Sessao.gravar_longo."""
    c.entrada()
    return c.gravar_longo(saida, n, 50)


def cenario_fast(c: Sessao, saida, n):
    c.teclas("shift+3")
    time.sleep(1)
    c.entrada()
    c.log(f"GameSpeed = {c.ler('GameSpeed')}")
    return c.gravar(saida, n)


def cenario_ultra(c: Sessao, saida, n):
    c.teclas("shift+4")
    time.sleep(1)
    c.entrada()
    c.log(f"GameSpeed = {c.ler('GameSpeed')}")
    return c.gravar(saida, n)


def perto(lista, x, y):
    return min(lista, key=lambda p: max(abs(p[0] - x), abs(p[1] - y)))


def cenario_amarela_sup(c: Sessao, saida, n):
    """Sai do ninho até uma comida (duplo clique: pegar), volta a um buraco e luta com uma vermelha."""
    c.teclas("ctrl+8")  # vista da superfície
    time.sleep(1)
    c.entrada()
    s = c.save()
    comidas = [(x, y) for x in range(128) for y in range(64) if 0x48 <= s.celula("MapA", x, y) <= 0x4B]
    buracos = [(x, y) for x in range(128) for y in range(64) if s.celula("MapA", x, y) == 0x50]
    c.log(f"amarela no plano {s.ler('MePlane')} em ({s.ler('MeLocX')}, {s.ler('MeLocY')}); "
          f"{len(comidas)} comidas, buracos {buracos}")
    bx, by = buracos[0] if s.ler("MePlane") != 1 else (s.ler("MeLocX"), s.ler("MeLocY"))
    alvo = perto(comidas, bx, by)
    c.clicar_celula(*alvo, duplo=True)
    nomes = []
    # 1) sair do ninho e pegar a comida
    nomes += c.gravar(saida, n, parar=lambda d: d.ler("MeType") & 0x08 != 0)
    # 2) voltar a um buraco
    c.entrada()
    s = c.save()
    me = (s.ler("MeLocX"), s.ler("MeLocY"))
    c.clicar_celula(*perto(buracos, *me))
    nomes += c.gravar(saida, n, parar=lambda d: d.ler("MePlane") != 1)
    # 3) sair de novo e lutar com a vermelha mais próxima do buraco
    c.entrada()
    s = c.save()
    verms = [(f["X"], f["Y"]) for _, f in formigas(s, "A") if f["T"] & 0x80 and (f["T"] & 0x78) >> 3 in (2, 3, 5, 6, 7, 9)]
    if verms:
        alvo = perto(verms, *perto(buracos, *me))
        c.clicar_celula(*alvo, duplo=True)
        nomes += c.gravar(saida, n, parar=lambda d: d.ler("MeCmd") != 3 and d.ler("MeMoveMe") == 0)
    return nomes


def cenario_luta(c: Sessao, saida, n):
    """Vista da superfície; duplo clique na vermelha (operária ou soldado) mais próxima do buraco preto
    (ordem 3, atacar): a amarela sai do ninho, persegue e luta. Para quando a ordem termina."""
    c.teclas("ctrl+8")
    time.sleep(1)
    c.entrada()
    # a compactação da lista A (_Cycle % 128 == 0) muda o índice do alvo e faz a ordem falhar: começa
    # logo depois de uma, para ter 127 ticks até a próxima
    gt = c.dep.ate_gametime((c.ler("Cycle") // 128 + 1) * 128)
    c.log(f"esperou até GameTime {gt}")
    s = c.save()
    buracos = [(x, y) for x in range(64) for y in range(64) if s.celula("MapA", x, y) == 0x50]
    verms = [(f["X"], f["Y"]) for _, f in formigas(s, "A") if f["T"] & 0x80 and (f["T"] & 0x78) >> 3 in (2, 3, 5, 6, 7, 9)]
    alvo = perto(verms, *buracos[0])
    c.log(f"buraco preto {buracos[0]}; vermelha alvo em {alvo}")
    c.clicar_celula(*alvo, duplo=True)
    return c.gravar(saida, n, parar=lambda d: d.ler("MeMoveMe") == 0)


def cenario_luta_soldado(c: Sessao, saida, n):
    """Como `luta`, mas com a amarela posta como soldado (_MeType = 0x30) e o alvo uma operária vermelha
    (grupo 2): classe de luta 1 contra 0, para gravar uma vitória."""
    c.teclas("ctrl+8")
    time.sleep(1)
    c.entrada()
    gt = c.dep.ate_gametime((c.ler("Cycle") // 128 + 1) * 128)
    c.log(f"esperou até GameTime {gt}")
    c.escrever("MeType", 0x30)
    for tentativa in range(6):
        c.entrada()
        s = c.save()
        buracos = [(x, y) for x in range(64) for y in range(64) if s.celula("MapA", x, y) == 0x50]
        # alvo parado (modos 13 Resting e 17), para que o duplo clique ainda o encontre na célula
        verms = [(f["X"], f["Y"]) for _, f in formigas(s, "A")
                 if f["T"] & 0x80 and (f["T"] & 0x78) >> 3 == 2 and f["M"] in (13, 17)]
        verms = verms or [(f["X"], f["Y"]) for _, f in formigas(s, "A") if f["T"] & 0x80 and (f["T"] & 0x78) >> 3 == 2]
        alvo = perto(verms, *buracos[0])
        c.log(f"tentativa {tentativa}: operária vermelha alvo em {alvo}")
        c.clicar_celula(*alvo, duplo=True)
        time.sleep(1.5)
        c.entrada()
        if c.ler("MeCmd") == 3:
            break
    return c.gravar(saida, n, parar=lambda d: d.ler("MeMoveMe") == 0)


def cenario_revoada_natural(c: Sessao, saida, n):
    """22 adultas vermelhas do ninho viram machos (grupo 4) em M = 14 (pré-voo), com a marca do mapa de
    vida acertada; o resto é do jogo: _CountAnts conta os machos, _GstrR arma _FlyAwayR = 200 (> 20
    reprodutores), o pré-voo vira 15 com F > 100 e a decolagem (16) vem com F = 1 ou S1(F).
    Grava até o armar, ~4 ticks com F perto de 100 e os últimos ~8 ticks até F = 0."""
    c.entrada()
    s = c.save()
    feitas = []
    for lista, vida, tam in (("R", "LifeR", 4096), ("A", "LifeA", 8192)):
        seg, off, _ = c.mj.resolver(vida, tam)
        adultas = [(i, f) for i, f in formigas(s, lista)
                   if f["T"] & 0x80 and (f["T"] & 0x78) >> 3 in (1, 2, 3, 5, 6, 7, 9)][:22 - len(feitas)]
        for i, f in adultas:
            t = 0x80 | 0x20 | (f["T"] & 7)
            c.escrever_elemento(f"{lista}listT", i, t)
            c.escrever_elemento(f"{lista}listM", i, 14)
            c.mj.escrever_variavel(f"{seg}:{off + f['X'] * 64 + f['Y']:x}", bytes([t]), 1)
            feitas.append(f"{lista}{i}")
    c.log(f"{len(feitas)} vermelhas viram machos (T 0xA0 | dir) em M = 14: {feitas}")
    nomes = c.gravar(saida, n, parar=lambda d: d.ler("FlyAwayR") > 0)
    c.log(f"FlyAwayR armado depois de {len(nomes)} ticks")
    while c.ler("FlyAwayR") > 102:
        c.avancar(1)
    nomes += c.gravar(saida, 4)
    while c.ler("FlyAwayR") > 6:
        c.avancar(1)
    nomes += c.gravar(saida, 10)
    return nomes


def cenario_luta_vizinha(c: Sessao, saida, n):
    """A amarela (soldado, vida 100) é posta na superfície numa célula livre vizinha de uma operária
    vermelha, e o duplo clique nela dá a ordem 3: a luta vem em poucos ticks (vitória provável:
    classe 1 contra 0)."""
    c.teclas("ctrl+8")
    time.sleep(1)
    c.entrada()
    gt = c.dep.ate_gametime((c.ler("Cycle") // 128 + 1) * 128)
    c.log(f"esperou até GameTime {gt}")
    s = c.save()
    vida_a = c.mj.ler_variavel("LifeA", 8192)
    livre = lambda x, y: 0 < x < 127 and 0 < y < 63 and s.celula("MapA", x, y) < 0x10 and vida_a[x * 64 + y] == 0
    escolha = None
    for i, f in formigas(s, "A"):
        if f["T"] & 0x80 and (f["T"] & 0x78) >> 3 == 2:
            viz = [(f["X"] + dx, f["Y"] + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if (dx, dy) != (0, 0)]
            viz = [p for p in viz if livre(*p)]
            if viz:
                escolha = (i, (f["X"], f["Y"]), viz[0])
                break
    ia, alvo, (mx, my) = escolha
    # M = 17 (sem efeito na superfície): a operária fica parada até o clique ser processado
    c.escrever_elemento("AlistM", ia, 17)
    c.log(f"AlistM[{ia}] <- 17 (alvo parado)")
    plano, ox, oy = c.ler("MePlane"), c.ler("MeLocX"), c.ler("MeLocY")
    vseg, voff, _ = c.mj.resolver({1: "LifeA", 2: "LifeB", 3: "LifeR"}[plano], 8192 if plano == 1 else 4096)
    c.mj.escrever_variavel(f"{vseg}:{voff + ox * 64 + oy:x}", b"\0", 1)
    aseg, aoff, _ = c.mj.resolver("LifeA", 8192)
    c.mj.escrever_variavel(f"{aseg}:{aoff + mx * 64 + my:x}", b"\xff", 1)
    for nome, v in (("MePlane", 1), ("MeLocX", mx), ("MeLocY", my), ("MeType", 0x30), ("MeHealth", 100)):
        c.escrever(nome, v)
    c.log(f"amarela de ({plano}, {ox}, {oy}) para a superfície em ({mx}, {my}), vizinha da vermelha em {alvo}")
    c.clicar_celula(*alvo, duplo=True, pausado=True)
    return c.gravar(saida, n, parar=lambda d: d.ler("MeCmd") == 3 and d.ler("MeMoveMe") == 0)


# ------------------------------------------------------------------------------------------------
# Ações da interface fora do tick (jogo pausado): estado antes e depois
# ------------------------------------------------------------------------------------------------

YARD_ORIGEM = (120, 157)   # canto da área cliente da janela 25 na bancada (telas/quintal.md §1)


def pixel_patch(px, py):
    """Centro do patch (px, py) na tela 640x480 com a janela do quintal na posição padrão."""
    ox, oy = YARD_ORIGEM
    return ox + 28 * px - 10 * py + 177, oy + 10 * py + 70


def fotografar(c: Sessao, pasta, nome):
    """Estado (formato .ANT), vida e extras (S, R, GameTime...) em pasta/nome.*; a VM fica parada."""
    c.mj.sessao_gdb()
    qb.gravar(Path(pasta).resolve() / f"estado_{nome}.bin", c.mj.estado())
    qb.gravar(Path(pasta).resolve() / f"vida_{nome}.bin", c.dep.vida())
    ex = c.dep.extras()
    c.log(f"{nome}: {ex}")
    return ex


def tela(c: Sessao, pasta, nome):
    # o comando de roteiro separa por espaços: caminho relativo ao SIMANT_RAIZ (sem espaços)
    rel = Path(pasta).resolve().relative_to(qb.RAIZ.resolve()).as_posix()
    c.b.executar(f"capturar {rel}/{nome}.png")


def clicar_tela(c: Sessao, x, y, duplo=False):
    c.dep.retomar()
    time.sleep(0.3)
    qb.mouse_canto(c.q)
    qb.mover_rel(c.q, x, y)
    time.sleep(0.3)
    c.botoes(duplo)
    c.log(f"{'duplo ' if duplo else ''}clique em ({x}, {y})")


def dispensar(c: Sessao):
    """Fecha uma caixa de mensagem ("Click here to continue")."""
    c.dep.retomar()
    c.b.executar("clicar_em 320 330")
    time.sleep(1)


def cenario_xfer(c: Sessao, saida, n):
    """_XferPatch pela Yard View, com o jogo pausado (Shift+0): no Jogo Completo do início, uma colônia
    preta é escrita no patch (10, 8) e em _ColonyTotalBlack = 2; Ctrl+Y abre a Yard View e o duplo
    clique no patch faz a troca. Grava antes/depois (estado, vida, extras) e capturas."""
    alvo = (10, 8) if n != 2 else (1, 5)  # --ticks 2: patch da casa
    c.teclas("shift+0")
    time.sleep(1)
    c.mj.sessao_gdb()
    c.escrever_elemento("YMapPopB", alvo[0] * 16 + alvo[1], 3)
    c.escrever("ColonyTotalBlack", 2)
    c.dep.retomar()
    c.teclas("ctrl+y")
    time.sleep(2)
    antes = fotografar(c, saida, "antes")
    tela(c, saida, "1_yard")
    clicar_tela(c, *pixel_patch(*alvo), duplo=True)
    time.sleep(4)
    tela(c, saida, "2_dialogo")
    dispensar(c)
    time.sleep(2)
    tela(c, saida, "3_depois")
    depois = fotografar(c, saida, "depois")
    gravar_acao(saida, antes, depois, {"xfer": alvo})
    return []


def cenario_place(c: Sessao, saida, n):
    """_PlaceQueenInYard: Jogo Completo pausado, _QueenStorageB = 2, Ctrl+Y, botão PLACE da faixa 35
    (360, 60) e um clique no patch (9, 7) (dentro do alcance) ou, com --ticks 2, (5, 8) (fora)."""
    alvo = (9, 7) if n != 2 else (5, 8)
    c.teclas("shift+0")
    time.sleep(1)
    c.mj.sessao_gdb()
    c.escrever("QueenStorageB", 2)
    c.dep.retomar()
    c.teclas("ctrl+y")
    time.sleep(2)
    antes = fotografar(c, saida, "antes")
    tela(c, saida, "1_yard")
    clicar_tela(c, 360, 60)
    time.sleep(2)
    tela(c, saida, "2_place")
    clicar_tela(c, *pixel_patch(*alvo))
    time.sleep(3)
    tela(c, saida, "3_depois")
    depois = fotografar(c, saida, "depois")
    gravar_acao(saida, antes, depois, {"place": alvo})
    return []


def gravar_acao(saida, antes, depois, acao):
    """extras.json no formato dos vetores de tick (comparável com vetor_tick::comparar_com)."""
    (Path(saida) / "extras.json").write_text(json.dumps({"antes": antes, "depois": depois, "acao": acao}, indent=1) + "\n",
                                             encoding="utf-8")


def cenario_inseticida(c: Sessao, saida, n):
    """Menino dentro de casa parado no nó 32 ((_BoyX, _BoyY) = (45, 166) → patch (0, 12)) com
    _CurYardPnt = (0, 12): pisada e inseticida em todo tick (quintal.md §4). No Jogo Completo, com a
    amarela na superfície, cada tick tem chance 1/4 de _YellowDeath(9)."""
    c.entrada()
    for nome, v in (("NodeNum", 32), ("NodeCnt", 0), ("CurYardPnt", 12 << 16)):
        c.escrever(nome, v)
    return c.gravar(saida, n)


# ------------------------------------------------------------------------------------------------
# Eventos e seleção
# ------------------------------------------------------------------------------------------------

OLHAR = ("RainOn", "RainCnt", "WaterTable", "FlyAwayB", "FlyAwayR", "SwarmCntB", "SwarmCntR", "BAntsEaten",
         "RAntsEaten", "AntsEatenByLions", "BAntsKilled", "RAntsKilled", "SMode", "SpidOn", "BoyHere",
         "BoyIsMowing", "FootHere", "MePlane", "MeType", "MeCmd", "MeMoveMe", "MeHealth", "GameSpeed",
         "ListIndexA", "QueenStorageB", "QueenStorageR")


def eventos(bruto):
    for d in sorted(Path(bruto).glob("tick_*")):
        a, b = Save.abrir(d / "estado_antes.bin"), Save.abrir(d / "estado_depois.bin")
        mud = [f"{k} {a.ler(k)}->{b.ler(k)}" for k in OLHAR if a.ler(k) != b.ler(k)]
        mapas = [m for m in ("MapB", "MapR") if a.bytes_bloco(m) != b.bytes_bloco(m)]
        rel = json.loads((d / "extras.json").read_text(encoding="utf-8")).get("MacTickCount_lidos")
        print(f"{d.name}: {', '.join(mud + mapas)}  relógio {rel}")


def selecionar(bruto, destino, faixas, descricao):
    bruto, destino = Path(bruto), Path(destino)
    ind = json.loads((bruto / "indice.json").read_text(encoding="utf-8"))
    ticks = []
    for fx in faixas:
        a, _, b = fx.partition("-")
        for t in range(int(a), int(b or a) + 1):
            nome = f"tick_{t:06d}"
            shutil.copytree(bruto / nome, destino / nome, dirs_exist_ok=True)
            ticks.append(nome)
    ind["ticks"] = ticks
    if descricao:
        ind["descricao"] = descricao
    ind["selecao"] = faixas
    (destino / "indice.json").write_text(json.dumps(ind, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{len(ticks)} ticks -> {destino}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    g = sub.add_parser("gravar")
    g.add_argument("cenario")
    g.add_argument("saida")
    g.add_argument("--overlay", default=str(qb.OVERLAY_PADRAO))
    g.add_argument("--snapshot", default="rico")
    g.add_argument("--ticks", type=int, default=10)
    g.add_argument("--descricao", default="")
    e = sub.add_parser("eventos")
    e.add_argument("bruto")
    s = sub.add_parser("selecionar")
    s.add_argument("bruto")
    s.add_argument("destino")
    s.add_argument("faixas", nargs="+")
    s.add_argument("--descricao")
    a = ap.parse_args()
    if a.cmd == "eventos":
        return eventos(a.bruto)
    if a.cmd == "selecionar":
        return selecionar(a.bruto, a.destino, a.faixas, a.descricao)
    c = Sessao(a.overlay, a.snapshot)
    try:
        ticks = globals()[f"cenario_{a.cenario}"](c, a.saida, a.ticks)
        c.indice(a.saida, ticks, a.snapshot, a.descricao or a.cenario)
    finally:
        c.fechar()


if __name__ == "__main__":
    main()
