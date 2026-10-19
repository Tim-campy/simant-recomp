"""Procura parâmetros de um timbre FM de 2 operadores (modelo estático do sintetizador do port,
simant/src/som/fm.rs: mesma modulação, realimentação e formas de onda, sem envelopes) cujo perfil
harmônico se aproxime de um perfil medido no original (tools/som_dosbox.py harmonicos).

Uso: python -I tools/ajustar_timbre_fm.py "-10 0 -6 -22 -17 -17 -17 -18 -20 -21 -23 -24" [--ondas 0,1,2]
         [--mults 0.5,1,2,3] [--mults-portadora 1,2,3]
Um harmônico desconhecido (coberto por outra nota na medição) vai como "x" e fica fora do erro.
Imprime os melhores candidatos (mult do modulador, mult da portadora, TL do modulador,
realimentação, ondas). Serve de ponto de partida; o timbre final vai em tools/gerar_banco_opl.py.
"""
import argparse
import math

import numpy as np

TAXA = 49716.0
MULT = [0.5, 1, 2, 3, 4, 5, 6, 7, 8]


def onda(n, x):
    s = np.sin(2 * np.pi * x)
    if n == 0:
        return s
    if n == 1:
        return np.maximum(s, 0)
    if n == 2:
        return np.abs(s)
    if n == 3:
        return np.where(np.mod(x, 0.5) < 0.25, np.abs(s), 0.0)
    if n == 4:
        return np.where(x < 0.5, np.sin(4 * np.pi * x), 0.0)
    if n == 5:
        return np.where(x < 0.5, np.abs(np.sin(4 * np.pi * x)), 0.0)
    if n == 6:
        return np.where(x < 0.5, 1.0, -1.0)
    p = x * 1024
    return np.where(p < 512, 2 ** (-p / 32), -(2 ** (-(1023 - p) / 32)))


def onda1(n, x):
    return float(onda(n, np.array([x]))[0])


def modulador(mm, tl, fb, w, f0, n):
    amp = 10 ** (-tl * 0.75 / 20)
    passo = f0 * mm / TAXA
    if fb == 0:
        return onda(w, np.mod(np.arange(n) * passo, 1.0)) * amp
    out = np.zeros(n)
    o1 = o2 = 0.0
    k = 4.0 / 2 ** (9 - fb)
    for i in range(n):
        m = onda1(w, (i * passo + (o1 + o2) * k) % 1.0) * amp
        out[i] = m
        o2, o1 = o1, m
    return out


def perfil(mm, mc, tl, fb, wm, wc, f0=110.0, n=4096):
    m = modulador(mm, tl, fb, wm, f0, n)
    fase = np.mod(np.arange(n) * f0 * mc / TAXA + 4.0 * m, 1.0)
    s = onda(wc, fase)[n // 4 :]
    mag = np.abs(np.fft.rfft(s * np.hanning(len(s)), 8 * len(s)))
    fr = np.fft.rfftfreq(8 * len(s), 1 / TAXA)
    h = np.array([mag[(fr > f0 * i * 0.97) & (fr < f0 * i * 1.03)].max() for i in range(1, 13)])
    return np.maximum(20 * np.log10(h / h.max() + 1e-9), -40)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("alvo")
    ap.add_argument("--ondas", default="0,1,2")
    ap.add_argument("--mults", default="0.5,1,2,3")
    ap.add_argument("--mults-portadora", default="1,2,3")
    a = ap.parse_args()
    alvo = np.maximum(np.array([np.nan if x in ("x", "nan") else float(x) for x in a.alvo.split()]), -40)
    conhecido = ~np.isnan(alvo)
    ondas = [int(x) for x in a.ondas.split(",")]
    mults = [float(x) for x in a.mults.split(",")]
    res = []
    for mm in mults:
        for mc in [float(x) for x in a.mults_portadora.split(",")]:
            for wm in ondas:
                for wc in ondas:
                    for fb in [0, 3, 5, 6, 7]:
                        for tl in range(0, 48, 3):
                            p = perfil(mm, mc, tl, fb, wm, wc)
                            # Perfil relativo ao maior harmônico conhecido, como na medição.
                            p = p - p[conhecido].max()
                            res.append((float(np.abs(p - alvo)[conhecido].mean()), mm, mc, tl, fb, wm, wc))
    res.sort()
    for r in res[:10]:
        print("erro %.1f dB  mod mult %s  car mult %s  mod TL %d  fb %d  ondas %d/%d" % r)
        p = perfil(*r[1:])
        print("   ", " ".join("%4.0f" % v for v in p - p[conhecido].max()))


if __name__ == "__main__":
    main()
