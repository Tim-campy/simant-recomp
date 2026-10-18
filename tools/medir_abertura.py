"""Mede na bancada QEMU quanto tempo o título e o registro da abertura ficam na tela (docs/spec/telas/abertura.md).

Liga a VM (boot completo pelo overlay dado), tira um screendump a cada ~0,2 s e classifica a tela por
pixels: "titulo" (fundo escuro do logotipo no canto de cima da janela 3, que é a última parte desenhada,
porque o DIB é pintado de baixo para cima), "registro" (caixa cinza com borda azul da janela 30) ou
"outro". Imprime cada troca com o instante do relógio do host e o contador de ticks do BIOS. Com --espaco, aperta Espaço 2 s depois de
o título ficar completo e 2 s depois de o registro aparecer (com a espera do jogo já armada), e mede quanto cada tela demora a sumir.

    python tools/medir_abertura.py OVERLAY.qcow2 [--espaco] [--limite 150] [--pasta re_trabalho/abertura]
"""
import argparse
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import qemu_bancada as qb  # noqa: E402


def pixel(d, w, x, y):
    o = (y * w + x) * 3
    return tuple(d[o:o + 3])


def classificar(ppm: Path):
    d = ppm.read_bytes()
    partes = d.split(maxsplit=4)
    w = int(partes[1])
    px = partes[4]
    a = pixel(px, w, 80, 130)          # canto de cima do logotipo (janela 3 em (68, 115))
    b = pixel(px, w, 195, 330)         # dentro da caixa do registro, perto da borda esquerda
    c = pixel(px, w, 189, 330)         # borda azul da caixa do registro
    if sum(a) < 150:
        return "titulo"
    if sum(a) > 700 and abs(b[0] - 192) < 20 and c[2] > 100 and c[0] < 60:
        return "registro"
    return "outro"


def ticks_bios(b, pasta):
    """Contador de ticks do BIOS (0040:006C, 18,2 por segundo) lido da RAM física: relógio da VM, que
    não depende da carga do host."""
    arq = pasta / "bios.bin"
    b.q.cmd("pmemsave", val=0x46C, size=4, filename=str(arq))
    return int.from_bytes(arq.read_bytes(), "little")


def esperas(b):
    """Pilha das esperas de _DialogWaitInit (`1000:9bfe`): índice DGROUP:150a, início (_TickCount =
    GetTickCount / 55) em DGROUP:cad4 + 4i e duração em ticks (param × 18) em DGROUP:caf4 + 2i. Só
    com o SIMANTW.EXE carregado (a busca dos seletores usa a tabela do save); None antes disso."""
    try:
        i = int.from_bytes(b.mj.ler_variavel("10:150a", 2), "little", signed=True)
        ini = b.mj.ler_variavel("10:cad4", 32)
        dur = b.mj.ler_variavel("10:caf4", 16)
    except Exception:  # noqa: BLE001 - tabela do save ainda não está na memória
        return None
    if not 0 <= i < 8:
        return (i,)
    return (i, int.from_bytes(ini[4 * i:4 * i + 4], "little"), int.from_bytes(dur[2 * i:2 * i + 2], "little"))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("overlay")
    ap.add_argument("--espaco", action="store_true")
    ap.add_argument("--limite", type=float, default=150)
    ap.add_argument("--pasta", default="re_trabalho/abertura")
    a = ap.parse_args()
    pasta = qb.caminho_raiz(a.pasta)
    pasta.mkdir(parents=True, exist_ok=True)
    ppm = pasta / "medir.ppm"
    b = qb.Bancada(a.overlay)
    trocas, atual, apertou, espera = [], None, set(), None
    try:
        while time.time() - b.t0 < a.limite:
            b.q.cmd("screendump", filename=str(ppm))
            t = time.time() - b.t0
            e = classificar(ppm)
            if e != atual:
                trocas.append((round(t, 2), e, ticks_bios(b, pasta)))
                b.log(f"tela: {e} (ticks do BIOS {trocas[-1][2]})")
                qb.ppm_para_png(ppm, pasta / f"troca_{len(trocas):02d}_{e}.png")
                atual, desde = e, t
            if e != "outro" or espera is not None:
                w = esperas(b)
                if w is not None and w != espera:
                    espera = w
                    trocas.append((round(time.time() - b.t0, 2), f"espera {w}", ticks_bios(b, pasta)))
                    b.log(f"_DialogWaitInit: índice, início, duração = {w}")
            armada = espera is not None and len(espera) == 3 and espera[2] == {"titulo": 720, "registro": 270}.get(e)
            if a.espaco and armada and e not in apertou and t - desde > 2:
                b.q.cmd("send-key", keys=[{"type": "qcode", "data": "spc"}], **{"hold-time": 150})
                apertou.add(e)
                trocas.append((round(time.time() - b.t0, 2), f"espaco em {e}", ticks_bios(b, pasta)))
                b.log(f"Espaço apertado ({e})")
            if len(trocas) > 1 and e == "outro" and "registro" in [x[1] for x in trocas] and t - desde > 3:
                break
            time.sleep(0.2)
    finally:
        b.fechar()
    print(trocas)
    ws = [(t, int(e.split(",")[1])) for t, e, _ in trocas if e.startswith("espera (") and e.count(",") == 2]
    for (t0, i0), (t1, i1) in zip(ws, ws[1:]):
        print(f"entre dois _DialogWaitInit: {i1 - i0} unidades de _TickCount ({(i1 - i0) * 55 / 1000:.2f} s de GetTickCount), {t1 - t0:.2f} s do host")
    for (t0, e0, k0), (t1, _, k1) in zip(trocas, trocas[1:]):
        print(f"{e0}: {t1 - t0:.2f} s do host, {k1 - k0} ticks do BIOS ({(k1 - k0) / 18.2065:.2f} s da VM)")


if __name__ == "__main__":
    main()
