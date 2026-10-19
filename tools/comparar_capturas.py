"""Gera as telas do port equivalentes às capturas de `cenarios_tick.py capturas` e compara.

Uso: python tools/comparar_capturas.py DIR_CAPTURAS --jogo PASTA_DO_JOGO [--simant target/debug/simant.exe]

DIR_CAPTURAS tem capt.ANT, balao.ANT, capturas.json e as capturas (examine_K, mapa_K, balao). Para cada
uma, roda o simant sem janela (--save, --examinar / --janela-mapa / --quintal, --exportar-tela-jogo) e
`comparar_captura.py --mapear` na região da janela. Imprime a tabela de porcentagens.
"""
import argparse
import json
import subprocess
import sys
from pathlib import Path

AQUI = Path(__file__).resolve().parent

# Regiões em coordenadas da área cliente (tela − 38 linhas): x0 y0 x1 y1
REGIOES = {
    "examine": None,           # calculada pela posição da janela do port (ver abaixo)
    "mapa": (118, 60, 634, 376),
    "balao": (119, 61, 632, 338),
    "balao_texto": (255, 170, 380, 205),
}


def rodar(simant, jogo, args, png):
    r = subprocess.run([simant, "--jogo", jogo, *args, "--exportar-tela-jogo", str(png)], capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit(f"simant falhou ({args}): {r.stderr[-500:]}")
    return r.stdout.strip().splitlines()[-1] if r.stdout.strip() else ""


def comparar(port, captura, regiao):
    args = [sys.executable, str(AQUI / "comparar_captura.py"), str(port), str(captura), "--mapear"]
    if regiao:
        args += ["--regiao", *map(str, regiao)]
    r = subprocess.run(args, capture_output=True, text=True)
    return (r.stdout + r.stderr).strip()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("dir")
    ap.add_argument("--jogo", required=True)
    ap.add_argument("--simant", default=str(AQUI.parent / "target" / "debug" / "simant.exe"))
    a = ap.parse_args()
    d = Path(a.dir)
    reg = json.loads((d / "capturas.json").read_text(encoding="utf-8"))
    linhas = []
    plano, mx, my = reg["vista"]
    for k, (x, y, t, m) in enumerate(reg.get("examine", [])):
        p = d / f"p_examine_{k}.png"
        # só a primeira janela da sessão tem o ajuste de 3 pixels (janelas.md §4.3)
        ja = ["--exame-ja-aberto"] if k > 0 else []
        rodar(a.simant, a.jogo, ["--save", str(d / "capt.ANT"), "--examinar", f"{x},{y}", *ja], p)
        # janela de 92 x 87 uma célula à direita e uma acima da clicada (janelas.md §4.3), com folga
        x0 = 24 + (x - mx + 1) * 16 - 4
        y0 = 158 - 38 + (y - my - 1) * 16 - 4
        linhas.append((f"examine_{k} ({x}, {y}) T {t:#x} M {m}", comparar(p, d / f"examine_{k}.png", (x0, y0, x0 + 100, y0 + 95))))
    for modo, nome, *_ in reg.get("mapa", []):
        p = d / f"p_{nome}.png"
        rodar(a.simant, a.jogo, ["--save", str(d / "capt.ANT"), "--janela-mapa", str(modo)], p)
        linhas.append((f"{nome} (modo {modo})", comparar(p, d / f"{nome}.png", REGIOES["mapa"])))
    extra = d / "capturas_mapa.json"
    if extra.exists():
        sys.path.insert(0, str(AQUI))
        from ant_editar import Save
        for modo, nome, save in json.loads(extra.read_text(encoding="utf-8"))["mapa"]:
            p = d / f"p_{nome}.png"
            # O save já tem _MapMode = modo (gravado depois de um tick): o mapa abre com ele, e
            # --janela-mapa N com o mesmo N não aperta o botão.
            rodar(a.simant, a.jogo, ["--save", str(d / save), "--janela-mapa", str(modo)], p)
            linhas.append((f"{nome} (modo {modo}, depois de um tick)", comparar(p, d / f"{nome}.png", REGIOES["mapa"])))
    if (d / "balao.png").exists():
        p = d / "p_balao.png"
        rodar(a.simant, a.jogo, ["--save", str(d / "balao.ANT"), "--quintal", "0"], p)
        linhas.append(("balao (janela)", comparar(p, d / "balao.png", REGIOES["balao"])))
        linhas.append(("balao (texto)", comparar(p, d / "balao.png", REGIOES["balao_texto"])))
    for nome, r in linhas:
        print(f"== {nome}\n{r}")


if __name__ == "__main__":
    main()
