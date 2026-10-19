"""Ritmo do original com o relógio da VM ligado ao número de instruções (QEMU -icount).

Faz o boot completo num overlay novo (um snapshot gravado sem -icount não roda com ele: a simulação fica
parada depois do loadvm), chega ao Quick Game e mede ticks/s em normal, fast e ultra.

    set SIMANT_RAIZ=...
    python tools/ritmo_icount.py SHIFT [--extra "ARGS DO QEMU"] [--segundos 30]

SHIFT "-" mede sem -icount (TCG livre). O tempo da VM vem do contador de tiques do BIOS (0040:006C).
Com `-icount shift=N,sleep=on`, cada instrução vale 2^N ns do relógio da VM (1000 / 2^N milhões de
instruções por segundo) e a VM dorme quando está adiantada; se o TCG não sustenta a taxa, o relógio da VM
atrasa (muito: o TCG com icount emula ~3-8 MIPS) em relação ao host. Por isso o tempo é o do contador do
BIOS, lido com a VM pausada; o normal (6,06 ticks/s no modelo, simulacao-visao-geral.md §2) confere a régua.
A "régua do normal" impressa (ticks/s do host × 6,06 / normal no host) só vale se a VM anda a ritmo
constante e é menos confiável.
Resultado: docs/spec/regras/simulacao-visao-geral.md §3.3. O overlay criado é apagado no fim.
"""
import argparse
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import qemu_bancada as qb  # noqa: E402


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("shift")
    ap.add_argument("--extra", default="")
    ap.add_argument("--segundos", type=float, default=30)
    ap.add_argument("--overlay", default=None)
    a = ap.parse_args()
    ov = a.overlay or f"re_trabalho/qemu/ritmo_ic{a.shift}.qcow2"
    caminho = qb.caminho_raiz(ov)
    if caminho.exists():
        caminho.unlink()
    icount = "" if a.shift == "-" else f"-icount shift={a.shift},sleep=on "
    b = qb.Bancada(ov, 16, icount + a.extra)
    pasta = "re_trabalho/ritmo"
    (qb.RAIZ / pasta).mkdir(parents=True, exist_ok=True)
    nome = f"ic{a.shift}"

    def ler(v, tam=None):
        return int.from_bytes(b.mj.ler_variavel(v, tam), "little")

    def bios():
        # contador de tiques do BIOS (0040:006C, 18,2065 Hz no relógio da VM), lido no endereço linear 0x46C
        return int.from_bytes(b.mj.ler_linear(0x46C, 4), "little")

    try:
        t0 = time.time()
        while True:  # o jogo está na memória quando a tabela do save aparece
            time.sleep(5)
            try:
                b.mj.localizar(forcar=True)
                break
            except Exception:
                if time.time() - t0 > 900:
                    raise
        b.log(f"jogo na memória depois de {time.time() - t0:.0f} s")
        # título -> registro -> "Select a Game" -> Quick Game; em CPU emulada lenta cada tela demora, então
        # repete Espaço e o clique até o GameTime andar
        for k in range(30):
            time.sleep(10)
            b.executar(f"capturar {pasta}/{nome}_boot{k:02d}.png")
            g = ler("GameTime")
            time.sleep(5)
            if ler("GameTime") > g:
                break
            b.executar("teclas spc")
            time.sleep(5)
            b.executar("clicar_em 345 228")
        else:
            raise RuntimeError("o Quick Game não começou")
        b.log(f"Quick Game rodando depois de {time.time() - t0:.0f} s")
        res, vm_res = {}, {}
        for vel, tecla in (("normal", "shift+2"), ("fast", "shift+3"), ("ultra", "shift+4")):
            b.executar(f"teclas {tecla}")
            time.sleep(1)
            b.executar("pausar")
            g0, c0, h0, v0 = ler("GameTime"), ler("_gameCycles", 4), time.time(), bios()
            b.executar("continuar")
            time.sleep(a.segundos)
            b.executar("pausar")
            g1, c1, h1, v1 = ler("GameTime"), ler("_gameCycles", 4), time.time(), bios()
            b.executar("continuar")
            res[vel] = (g1 - g0) / (h1 - h0)
            vm = (v1 - v0) / 18.2065
            vm_res[vel] = (g1 - g0) / vm if vm > 0 else float("nan")
            b.log(f"{vel}: GameTime {g0} -> {g1} em {h1 - h0:.1f} s do host = {res[vel]:.2f} ticks/s; "
                  f"{vm:.1f} s da VM (BIOS {v0} -> {v1}) = {vm_res[vel]:.2f} ticks/s; "
                  f"_gameCycles por tick {(c1 - c0) / max(1, g1 - g0):.2f}")
        b.executar(f"capturar {pasta}/{nome}_fim.png")
        escala = 6.06 / res["normal"] if res["normal"] else float("nan")
        print(f"RESULTADO shift={a.shift} extra='{a.extra}' host: "
              + ", ".join(f"{k} {v:.2f}" for k, v in res.items())
              + f"; pela régua do normal: fast {res['fast'] * escala:.1f}, ultra {res['ultra'] * escala:.1f}"
              + "; pelo relógio do BIOS: " + ", ".join(f"{k} {v:.2f}" for k, v in vm_res.items()), flush=True)
    finally:
        b.fechar()
        try:
            caminho.unlink()
        except OSError:
            pass


if __name__ == "__main__":
    main()
