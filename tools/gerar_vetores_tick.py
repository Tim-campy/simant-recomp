"""Gera vetores de teste "estado antes -> estado depois" de ticks consecutivos do SimAnt original.

Usa a bancada QEMU (tools/qemu_bancada.py) em modo tick: breakpoints do gdbstub na entrada de _DoAntSim
(1028:0000) e no endereço de retorno dele no MYTIMERFUNC. Para cada tick grava, em SAIDA/tick_NNNNNN/:

    estado_antes.bin   307 blocos do save (formato .ANT, 48.386 bytes) na entrada de _DoAntSim
    estado_depois.bin  idem, no retorno de _DoAntSim (exatamente um tick depois)
    vida_antes.bin     _LifeA + _LifeB + _LifeR (16.384 bytes), mapas de ocupação fora do save
    vida_depois.bin    idem, depois do tick
    extras.json        variáveis escalares fora do save (sementes S e R, ...) antes e depois, ver EXTRAS

e SAIDA/indice.json com a origem (snapshot, sementes forçadas, seletores, data).

Uso:
    set SIMANT_RAIZ=...   (raiz com referencia/ e re_trabalho/)
    python tools/gerar_vetores_tick.py [--ticks 5] [--saida tests/referencias/ticks]
        [--snapshot quick] [--alvo GAMETIME] [--semente-s 0x1234] [--semente-r 1]
        [--segmentos] [--livre SEG]

--alvo          avança (tick a tick, com breakpoints) até a entrada com GameTime >= alvo antes de gravar
--semente-s/-r  força as sementes na primeira entrada de _DoAntSim (antes de --alvo)
--livre SEG     depois de forçar as sementes, deixa a VM rodar SEG segundos sem breakpoints antes do --alvo
--segmentos     grava também os segmentos de dados 8, 9 e 10 inteiros (seg8_antes.bin, ...), para achar
                variáveis fora do save que o tick altera
--mapear-leituras ARQ.json  em vez de gravar vetores, roda --ticks ticks com watchpoints de leitura em tudo o
                que está fora do save (segmentos 8, 9, 10; menos pilha e mapas derivados) e lista o que o tick lê

O caminho de --saida é relativo ao diretório atual (não ao SIMANT_RAIZ). Ver docs/spec/regras/testes-tick.md.
"""
import argparse
import datetime
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import qemu_bancada as qb  # noqa: E402


# Fora do save, mas já conhecidos (capturados à parte ou derivados): não precisam de watchpoint.
CONHECIDOS = {"_LifeA", "_LifeB", "_LifeR", "_TemBModePop", "_TemRModePop", "_TemCensus"}


def segmento_exe(nseg):
    """Bytes iniciais do segmento de dados nseg no SIMANTW.EXE (o resto, até o limite, é BSS = 0)."""
    import struct
    exe = qb.EXE.read_bytes()
    ne = struct.unpack_from("<H", exe, 0x3C)[0]
    tab, shift = (struct.unpack_from("<H", exe, ne + o)[0] for o in (0x22, 0x32))
    setor, tam, _, _ = struct.unpack_from("<HHHH", exe, ne + tab + (nseg - 1) * 8)
    return exe[setor << shift:(setor << shift) + tam]


def mapear(b, nticks, destino):
    """Watchpoints de leitura em tudo o que está fora do save nos segmentos 8, 9 e 10 (menos pilha e
    CONHECIDOS) durante nticks ticks: lista as variáveis fora do save que o tick LÊ."""
    simb = {}
    for linha in qb.SIMBOLOS.read_text(encoding="latin-1").splitlines():
        p = linha.split("\t")
        if len(p) >= 3 and p[0] in ("8", "9", "10"):
            simb.setdefault(int(p[0]), []).append((int(p[1]), p[2]))
    for v in simb.values():
        v.sort()
    cobertos = {}
    for bl in qb.carregar_layout():
        cobertos.setdefault(bl["seg"], set()).update(range(bl["off"], bl["off"] + bl["n"]))
    lim = {seg: b.mj.base(seg)[1] + 1 for seg in (8, 9, 10)}
    fim_dados = dict(lim)
    fim_dados[10] = next(o for o, n in simb[10] if n == "_end")  # acima: heap local e pilha
    for seg, lista in simb.items():
        for k, (o, n) in enumerate(lista):
            if n in CONHECIDOS:
                prox = lista[k + 1][0] if k + 1 < len(lista) else lim[seg]
                cobertos[seg].update(range(o, prox))
    cobertos[10].update(range(0xcbf2, 0xcbf4))  # SementeS
    cobertos[10].update(range(0xae34, 0xae38))  # SementeR
    faixas = []
    for seg in (8, 9, 10):
        ini = None
        for off in range(fim_dados[seg] + 1):
            livre = off < fim_dados[seg] and off not in cobertos[seg]
            if livre and ini is None:
                ini = off
            elif not livre and ini is not None:
                faixas.append((seg, ini, off))
                ini = None
    b.log(f"{len(faixas)} faixas fora do save, {sum(j - i for _, i, j in faixas)} bytes")
    exe = {seg: segmento_exe(seg) for seg in (8, 9, 10)}
    vistos = {}
    for _ in range(nticks):
        gt = int.from_bytes(b.mj.ler_variavel("GameTime"), "little")
        segs = b.dep.segmentos()
        for seg, off, nome, leitor in b.dep.mapear_leituras(faixas, simb):
            ini = exe[seg][off:off + 2].ljust(2, b"\0") if off < len(exe[seg]) else b"\0\0"
            atual = segs[seg][off:off + 2]
            item = vistos.setdefault(f"{seg}:{off:04x}", {"nome": nome, "ticks": [], "valor": atual.hex(),
                                                          "igual_ao_exe": atual == ini, "leitores": []})
            item["ticks"].append(gt)
            if leitor not in item["leitores"]:
                item["leitores"].append(leitor)
        b.log(f"tick {gt}: {len(vistos)} endereços lidos fora do save até agora")
        b.dep.ir_entrada()
    destino.parent.mkdir(parents=True, exist_ok=True)
    destino.write_text(json.dumps(vistos, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    # resumo por símbolo (o JSON tem cada endereço)
    grupos = {}
    for k, v in sorted(vistos.items(), key=lambda x: (int(x[0].split(":")[0]), int(x[0].split(":")[1], 16))):
        g = grupos.setdefault(k.split(":")[0] + ":" + v["nome"].split("+")[0],
                              {"primeiro": k, "n": 0, "exe": set(), "leitores": set()})
        g["n"] += 1
        g["exe"].add(v["igual_ao_exe"])
        g["leitores"].update(x.split(":")[0] for x in v["leitores"])
    for nome, g in grupos.items():
        exe = {frozenset({True}): "= EXE", frozenset({False}): "≠ EXE"}.get(frozenset(g["exe"]), "misto")
        print(f"{g['primeiro']:<10} {nome:<30} {g['n']:4d} endereço(s)  {exe:6}  código: {' '.join(sorted(g['leitores']))}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--ticks", type=int, default=5)
    ap.add_argument("--saida", default="tests/referencias/ticks")
    ap.add_argument("--snapshot", default="quick")
    ap.add_argument("--alvo", type=int)
    ap.add_argument("--semente-s", type=lambda v: int(v, 0))
    ap.add_argument("--semente-r", type=lambda v: int(v, 0))
    ap.add_argument("--livre", type=float)
    ap.add_argument("--segmentos", action="store_true")
    ap.add_argument("--overlay", default=str(qb.OVERLAY_PADRAO))
    ap.add_argument("--mem", type=int, default=16)
    ap.add_argument("--mapear-leituras", metavar="ARQ.json")
    a = ap.parse_args()

    saida = Path(a.saida).resolve()
    b = qb.Bancada(a.overlay, a.mem)
    try:
        b.executar(f"snapshot_carregar {a.snapshot}")
        dep, mj = b.dep, b.mj
        dep.ir_entrada()
        inicio = dep.extras()
        b.log(f"primeira entrada de _DoAntSim: {inicio}")
        if a.semente_s is not None:
            mj.escrever_variavel("SementeS", a.semente_s.to_bytes(2, "little"))
        if a.semente_r is not None:
            mj.escrever_variavel("SementeR", a.semente_r.to_bytes(4, "little"))
        if a.livre:
            dep.retomar()
            qb.time.sleep(a.livre)
        if a.alvo is not None:
            gt = dep.ate_gametime(a.alvo)
            b.log(f"entrada com GameTime = {gt}")
        dep.ir_entrada()
        if a.mapear_leituras:
            mapear(b, a.ticks, Path(a.mapear_leituras))
            return
        ticks = []
        for _ in range(a.ticks):
            arqs = dep.capturar_tick(a.segmentos)
            ex_antes, ex_depois = arqs["extras.json"]["antes"], arqs["extras.json"]["depois"]
            pasta = saida / f"tick_{ex_antes['GameTime']:06d}"
            for nome_arq, conteudo in arqs.items():
                qb.gravar(pasta / nome_arq, conteudo)
            ticks.append(pasta.name)
            b.log(f"{pasta.name}: GameTime {ex_antes['GameTime']} -> {ex_depois['GameTime']}, "
                  f"S {ex_antes['SementeS']:#06x} -> {ex_depois['SementeS']:#06x}, "
                  f"R {ex_antes['SementeR']:#010x} -> {ex_depois['SementeR']:#010x}")
        indice = {
            "_comentario": "Vetores de tick do SimAnt original (bancada QEMU). Ver docs/spec/regras/testes-tick.md.",
            "gerado_em": datetime.date.today().isoformat(),
            "snapshot": a.snapshot,
            "semente_s_forcada": a.semente_s,
            "semente_r_forcada": a.semente_r,
            "alvo": a.alvo,
            "livre_segundos": a.livre,
            "primeira_entrada": inicio,
            "seletores_dados": {str(k): f"{v:#06x}" for k, v in mj.localizar().items()},
            "seletor_codigo_6": f"{mj.seletor_codigo(6):#06x}",
            "retorno_DoAntSim": f"{dep.retorno_far[0]:04x}:{dep.retorno_far[1]:04x}",
            "extras": {k: list(v) for k, v in qb.EXTRAS.items()},
            "ticks": ticks,
        }
        (saida / "indice.json").write_text(json.dumps(indice, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
        b.log(f"{len(ticks)} ticks -> {saida}")
    finally:
        b.fechar()


if __name__ == "__main__":
    main()
