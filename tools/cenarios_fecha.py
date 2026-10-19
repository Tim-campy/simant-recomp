"""Cenários das verificações pequenas da rodada de fechamento (docs/estado-do-projeto.md, "Trabalho de
verificação", 2026-10-19): a alimentação periódica (_AddFood com parâmetro >= 0) no quintal e na casa, e o
retorno de _CalcScore no GAME OVER com notas 3 e tempos variados.

Mesma interface de tools/cenarios_raros.py (e de cenarios_tick.py):

    set SIMANT_RAIZ=...
    python tools/cenarios_fecha.py gravar CENARIO DIR_BRUTO --overlay OV [--snapshot rico] [--ticks N]
    python tools/cenarios_fecha.py selecionar DIR_BRUTO DESTINO TICK

Cenários:
    alimentacao       _NextFeed = 30000 (acima de _FoodA) e _Cycle levado ao fim de um período de 64 na
                      entrada de um tick: o tick chama _AddFood(150, 1) (superficie.md 8.3)
    alimentacao_casa  o mesmo depois de _XferPatch para o patch da casa CASA_PATCH ("px,py", padrão "0,4")
    pontuacao         para cada caso de PONTUACAO (abaixo): recarrega o snapshot, escreve _GameTime,
                      _BlackLost, _RedLost, _MeHealth e _IsGameOver = 1 na entrada de um tick; o fim de
                      _DoAntSim abre o GAME OVER (_EndGameDialog(0)) com a pontuação de _CalcScore. Grava a
                      tela e o estado com a caixa aberta (estado_<caso>.bin), para o port calcular
                      (cargo run -p engine --example pontuacao -- estado_<caso>.bin).
"""
import json
import os
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cenarios_raros as cra  # noqa: E402  (registra os ajustes de cenarios_regras)
import cenarios_tick as ct  # noqa: E402
from cenarios_tick import Sessao  # noqa: E402


def preparar_alimentacao(c: Sessao):
    c.entrada()
    ciclo = c.ler("Cycle")
    alvo = ciclo | 0x3F
    if alvo >= 0x1000:
        alvo = 0x3F
    c.escrever("Cycle", alvo)
    c.escrever("NextFeed", 30000)
    c.log(f"Cycle {ciclo} -> {alvo}; FoodA {c.ler('FoodA', sinal=True)}")


def cenario_alimentacao(c: Sessao, saida, n):
    """_AddFood(150, 1) da alimentação periódica no quintal (_FoodA < _NextFeed, _Cycle ≡ 0 mod 64)."""
    preparar_alimentacao(c)
    return c.gravar(saida, max(n, 1))


def cenario_alimentacao_casa(c: Sessao, saida, n):
    """A mesma alimentação num patch da casa (o chão da casa recebe a comida de outro jeito: 3.E)."""
    alvo = tuple(int(v) for v in os.environ.get("CASA_PATCH", "0,4").split(","))
    cra.xfer_para(c, str(Path(saida).resolve() / "xfer"), alvo)
    c.dep.retomar()
    c.teclas("ctrl+e")
    time.sleep(1)
    c.teclas("shift+0")  # o xfer deixa o jogo pausado: Unpause
    time.sleep(2)
    preparar_alimentacao(c)
    c.log(f"_CurYardPnt = {c.ler('CurYardPnt'):#x}; TERRAINset {c.ler('TERRAINset')}")
    return c.gravar(saida, max(n, 1))


# (nome, snapshot, GameTime, BlackLost, RedLost, MeHealth)
PONTUACAO = [
    ("q_tempo_longo", "rico", 5000, 30, 10, 50),
    ("q_tempo_curto", "rico", 2000, 5, 45, 80),
    ("q_sem_baixas_pretas", "rico", 4100, 0, 25, 100),
    ("q_32bits", "rico", 30000, 3, 1, 1),
    ("f_tempo_longo", "full", 9000, 12, 36, 70),
    ("f_tempo_curto", "full", 3000, 40, 0, 100),
    ("f_limite", "full", 8099, 7, 7, 33),
]


def cenario_pontuacao(c: Sessao, saida, n):
    """Os casos de PONTUACAO do snapshot passado em --snapshot (rico: Quick Game; full: Jogo Completo)."""
    saida = Path(saida).resolve()
    snapshot = os.environ.get("SNAPSHOT_PONTUACAO", "rico")
    so = os.environ.get("PONTUACAO_CASOS")
    resumo = {}
    for nome, snap, tempo, pretas, vermelhas, saude in PONTUACAO:
        if snap != snapshot or (so and nome not in so.split(",")):
            continue
        c.b.executar(f"snapshot_carregar {snapshot}")
        c.entrada()
        for var, v, tam in (("GameTime", tempo, None), ("BlackLost", pretas, 2), ("RedLost", vermelhas, 2),
                            ("MeHealth", saude, 2), ("IsGameOver", 1, 2)):
            c.escrever(var, v, tam)
        ct.fotografar(c, saida, f"{nome}_entrada")
        c.dep.retomar()
        time.sleep(6)
        ct.tela(c, saida, f"{nome}_fim")
        ex = ct.fotografar(c, saida, nome)
        resumo[nome] = {"GameTime": tempo, "BlackLost": pretas, "RedLost": vermelhas, "MeHealth": saude, "extras": ex}
        c.log(f"{nome}: GAME OVER capturado")
    (saida / "pontuacao.json").write_text(json.dumps(resumo, indent=1) + "\n", encoding="utf-8")
    return []


def svga_examine(disco, overlay, saida):
    """Bancada SVGA 640x480 em 256 cores, num boot só (a paleta não volta depois de um loadvm): Quick
    Game pausado, e o Examine segurando o botão sobre duas formigas da janela de edição (a primeira
    abertura da sessão e a segunda). A área do mapa começa em x = 20 na SVGA (a janela de edição fica 4
    pixels à esquerda da VGA, telas/resolucao.md §7)."""
    import qemu_bancada as qb
    from ant_editar import Save
    saida = Path(saida).resolve()
    saida.mkdir(parents=True, exist_ok=True)
    rel = saida.relative_to(qb.RAIZ.resolve()).as_posix()
    b = qb.Bancada(overlay, 16, None, disco)
    try:
        for ln in ("esperar 70", "teclas spc", "esperar 4", "teclas spc", "esperar 6", "clicar_em 345 228",
                   "esperar 20", "teclas shift+0", "esperar 3", f"capturar {rel}/e00_quick.png"):
            b.executar(ln)
        b.executar("pausar")
        s = Save(b.mj.estado())
        b.executar("continuar")
        mx, my, plano = s.ler("MapPnt", 0, 2), s.ler("MapPnt", 1, 2), s.ler("MapPlane")
        lista = {1: "A", 2: "B", 3: "R"}[plano]
        me = (s.ler("MeLocX"), s.ler("MeLocY")) if s.ler("MePlane") == plano else None
        alvos = [(f["X"], f["Y"], f["T"]) for _, f in ct.formigas(s, lista)
                 if mx <= f["X"] < mx + ct.COLS and my <= f["Y"] < my + ct.LINS and (f["X"], f["Y"]) != me]
        b.log(f"vista ({plano}, {mx}, {my}); formigas na vista {alvos[:6]}")
        reg = {"vista": [plano, mx, my], "examine": []}
        for k, (x, y, t) in enumerate(alvos[:2]):
            px, py = 20 + (x - mx) * ct.TILE + 8, ct.MAPA_Y0 + (y - my) * ct.TILE + 8
            for ln in (f"ir_para {px} {py}", "apertar", "esperar 3", f"capturar {rel}/e0{k + 1}_examine.png",
                       "ir_para 2 2", "soltar", "esperar 2"):
                b.executar(ln)
            reg["examine"].append([x, y, t, px, py])
        b.executar("pausar")
        qb.gravar(saida / "e_estado.ANT", b.mj.estado())
        (saida / "examine.json").write_text(json.dumps(reg) + "\n", encoding="utf-8")
        b.executar("fim")
    finally:
        b.fechar()


if __name__ == "__main__":
    if len(sys.argv) == 5 and sys.argv[1] == "svga_examine":
        svga_examine(sys.argv[2], sys.argv[3], sys.argv[4])
        sys.exit(0)
    for nome, f in list(globals().items()):
        if nome.startswith("cenario_"):
            setattr(ct, nome, f)
    sys.argv[0] = "cenarios_tick.py"
    ct.main()
