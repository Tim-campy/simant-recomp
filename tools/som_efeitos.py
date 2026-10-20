"""Efeitos sonoros do SimAnt original numa gravação de sessão jogada (DOSBox-X, `tools/som_jogar.py`).

Acha cada ocorrência dos 57 efeitos tipo 5 (decodificados pelo port: `simant --exportar-som DIR`,
pasta `DIR/efeitos`) na gravação, por filtro casado, e mede: instante, taxa de reprodução, ganho,
quanto tocou (inteiro, cortado por outro efeito, ou parado sem outro efeito audível), nível do
efeito e da música no mesmo trecho. Também acha as músicas gravadas sem input
(`som_dosbox.py musicas`) dentro da sessão. Nada toca no PC. Ver docs/spec/regras/som.md, seção
"Efeitos dentro do jogo, gravados numa sessão jogada pelo usuário".

Uso:
  python -I tools/som_efeitos.py analisar SESSAO.wav EFEITOS_DIR [--taxa 5000] [--json SAIDA.json]
  python -I tools/som_efeitos.py taxa SESSAO.wav EFEITOS_DIR INSTANTE ID [--de 3500 --ate 12000 --passo 50]
  python -I tools/som_efeitos.py musicas SESSAO.wav MUSICAS_DIR
"""
import argparse
import json
import os
import wave
from collections import Counter, defaultdict

import numpy as np

FS = 44100
F2 = 11025  # taxa da busca (a gravação é decimada por 4; os efeitos têm banda < 3 kHz)
PREFIXO_S = 0.12  # trecho do começo do efeito usado na busca (um efeito cortado ainda casa)
BLOCO_DRIVER = 504  # amostras: o sb16snd.drv toca os efeitos de 4096 Hz em blocos desse tamanho (regras/som.md)


# ------------------------------------------------------------------------------------------------
# Leitura e modelos
# ------------------------------------------------------------------------------------------------

def ler_wav(arq):
    """Mono em ponto flutuante (−1..1) e a taxa. Estéreo vira a média dos canais."""
    w = wave.open(arq)
    n, ch, larg, taxa = w.getnframes(), w.getnchannels(), w.getsampwidth(), w.getframerate()
    d = w.readframes(n)
    if larg == 1:
        x = (np.frombuffer(d, np.uint8).astype(np.float64) - 128) / 128
    else:
        x = np.frombuffer(d, '<i2').astype(np.float64) / 32768
    if ch > 1:
        x = x.reshape(-1, ch).mean(1)
    return x, taxa


def ler_efeitos(pasta):
    """`efeito_NNN.wav` (PCM de 8 bits do tipo 5, como o port exporta) → {id: amostras u8}."""
    efeitos = {}
    for nome in sorted(os.listdir(pasta)):
        if nome.startswith("efeito_") and nome.endswith(".wav"):
            w = wave.open(os.path.join(pasta, nome))
            efeitos[int(nome[7:-4])] = np.frombuffer(w.readframes(w.getnframes()), np.uint8)
    return efeitos


def modelo(pcm, taxa, fs, modo="linear"):
    """O efeito tocado a `taxa` Hz e amostrado a `fs` Hz: interpolação linear (o que o DOSBox-X faz;
    casa melhor que o retentor de ordem zero) ou `"degrau"`."""
    x = (pcm.astype(np.float64) - 128) / 128
    n = int(len(x) * fs / taxa)
    t = np.arange(n) * taxa / fs
    k = t.astype(int)
    a = x[np.minimum(k, len(x) - 1)]
    if modo == "degrau":
        return a
    b = x[np.minimum(k + 1, len(x) - 1)]
    return a + (b - a) * (t - k)


def passa_altas(x, fs, janela_s=0.01):
    """Tira a média móvel: o DOSBox-X grava a componente contínua (a última amostra do efeito fica
    parada e decai devagar), que atrapalha o ajuste."""
    w = max(3, int(janela_s * fs)) | 1
    cs = np.concatenate([[0.0], np.cumsum(x)])
    mv = (cs[w:] - cs[:-w]) / w
    pad_e = w // 2
    mv = np.concatenate([np.full(pad_e, mv[0]), mv, np.full(len(x) - len(mv) - pad_e, mv[-1])])
    return x - mv


def decimar4(x):
    nt = 101
    t = np.arange(nt) - (nt - 1) / 2
    fir = np.sinc(2 * 4500 / FS * t) * np.hamming(nt)
    return np.convolve(x, fir / fir.sum(), mode="same")[::4]


def correlacao(x, h):
    """Correlação cruzada, NCC e energia da janela de `x` em cada deslocamento."""
    n = len(x) + len(h)
    N = 1 << int(np.ceil(np.log2(n)))
    c = np.fft.irfft(np.fft.rfft(x, N) * np.conj(np.fft.rfft(h, N)), N)[:len(x) - len(h) + 1]
    cs = np.concatenate([[0.0], np.cumsum(x * x)])
    en = cs[len(h):] - cs[:-len(h)]
    return c, c / np.sqrt(np.maximum(en, 1e-15) * (h @ h)), en


def db(v):
    return float(20 * np.log10(max(abs(v), 1e-12)))


# ------------------------------------------------------------------------------------------------
# Busca
# ------------------------------------------------------------------------------------------------

def candidatos(x2, efeitos, taxa):
    """Picos da correlação de cada efeito (prefixo) com a gravação decimada."""
    N = 1 << int(np.ceil(np.log2(len(x2) + F2)))
    X = np.fft.rfft(x2, N)
    cs = np.concatenate([[0.0], np.cumsum(x2 * x2)])
    saida = []
    for i, pcm in efeitos.items():
        h = passa_altas(modelo(pcm, taxa, F2), F2)[:int(PREFIXO_S * F2)]
        L = len(h)
        nh2 = float(h @ h)
        if nh2 < 1e-9:
            continue
        c = np.fft.irfft(X * np.conj(np.fft.rfft(h, N)), N)[:len(x2) - L + 1]
        en = cs[L:] - cs[:-L]
        nc = c / np.sqrt(np.maximum(en, 1e-15) * nh2)
        forte = en > L * 10 ** (-50 / 10)  # janela com som (o silêncio casa com qualquer coisa)
        k = np.where((nc[1:-1] > 0.6) & (nc[1:-1] >= nc[:-2]) & (nc[1:-1] > nc[2:]) & forte[1:-1])[0] + 1
        for kk in k:
            saida.append({"k": int(kk), "id": i, "ncc": float(nc[kk]), "ganho": float(c[kk] / nh2),
                          "c": float(c[kk]), "en": float(en[kk]), "nh2": nh2})
    return saida


def escolher(cands, ganho, limiar):
    """Fração da energia da janela explicada pelo efeito com o ganho fixo da sessão; fica o melhor
    em cada 40 ms."""
    for c in cands:
        c["explica"] = (2 * ganho * c["c"] - ganho ** 2 * c["nh2"]) / max(c["en"], 1e-15)
    cands = sorted((c for c in cands if c["explica"] >= limiar), key=lambda c: -c["explica"])
    aceitos = []
    for c in cands:
        if all(abs(c["k"] - a["k"]) > 0.04 * F2 for a in aceitos):
            aceitos.append(c)
    return sorted(aceitos, key=lambda c: c["k"])


def refinar(x, det, efeitos, taxa):
    """Instante exato (a 44,1 kHz) e NCC do prefixo de 0,1 s."""
    for d in det:
        h = passa_altas(modelo(efeitos[d["id"]], taxa, FS), FS)
        hp = h[:int(0.1 * FS)]
        a = max(0, d["k"] * 4 - int(0.01 * FS))
        seg = x[a:a + len(hp) + int(0.02 * FS)]
        if len(seg) <= len(hp):
            d["inicio"] = a
            continue
        c, nc, _ = correlacao(seg, hp)
        k = int(np.argmax(nc))
        d["inicio"] = a + k
        d["ncc"] = float(nc[k])
        d["ganho"] = float(c[k] / (hp @ hp))
        d["h"] = h


def alinhar_fino(bruto, d, pcm, taxa):
    """Início e fase (fração de amostra do efeito) que melhor explicam os primeiros 80 ms."""
    p = (pcm.astype(np.float64) - 128) / 128
    n = int(0.08 * FS)
    melhor = None
    for desl in range(-4, 5):
        s0 = d["inicio"] + desl
        x = passa_altas(bruto[s0:s0 + n], FS)
        if len(x) < n:
            continue
        for fase in np.linspace(0, 1, 16, endpoint=False):
            h = passa_altas(_interp(p, taxa, n, fase), FS)
            g = (x @ h) / max(h @ h, 1e-12)
            r = x - g * h
            v = (r @ r) / max(x @ x, 1e-12)
            if melhor is None or v < melhor[0]:
                melhor = (v, s0, fase)
    return melhor


def _interp(p, taxa, n, fase):
    t = np.arange(n) * taxa / FS + fase
    k = t.astype(int)
    f = t - k
    a = np.where(k < len(p), p[np.minimum(k, len(p) - 1)], 0.0)
    b = np.where(k + 1 < len(p), p[np.minimum(k + 1, len(p) - 1)], 0.0)
    return a + (b - a) * f


def blocos(bruto, s0, fase, pcm, taxa, ganho, bloco_s=0.002):
    """Estado de cada bloco de 2 ms: '=' a gravação segue o efeito, '0' gravação muda com o efeito
    soando, 'x' outra coisa, '.' os dois mudos, '+' som depois do fim do efeito."""
    p = (pcm.astype(np.float64) - 128) / 128
    n = int((len(p) / taxa + 0.1) * FS)
    h = passa_altas(_interp(p, taxa, n, fase), FS) * ganho
    x = passa_altas(bruto[s0:s0 + n], FS)
    B = int(bloco_s * FS)
    est = []
    for b in range(0, min(len(x), n) - B, B):
        xb, hb = x[b:b + B], h[b:b + B]
        ex, eh = xb @ xb / B, hb @ hb / B
        er = (xb - hb) @ (xb - hb) / B
        if eh < 1e-6 and ex < 1e-6:
            est.append(".")
        elif er < 0.1 * max(ex, eh):
            est.append("=")
        elif ex < 1e-6:
            est.append("0")
        elif eh < 1e-6:
            est.append("+")
        else:
            est.append("x")
    return "".join(est), B


def fim_tocado(estados, B):
    """Amostras (a 44,1 kHz) até o fim do último trecho de 4 ou mais blocos '=' antes do silêncio da
    gravação (8 blocos '0'). As falhas curtas no meio (~10 ms, vistas também no teste controlado)
    e os trechos fracos do efeito não encerram a contagem."""
    import re
    z = re.search(r"0{8,}", estados)
    ate = z.start() if z else len(estados)
    fim = 0
    for m in re.finditer(r"={4,}", estados[:ate]):
        fim = m.end()
    if fim == 0:
        ult = estados[:ate].rfind("=")
        fim = ult + 1 if ult >= 0 else 0
    return fim * B


def musica_em_volta(x, det, j, janela_s=1.0):
    """RMS (dBFS) da gravação no segundo antes do efeito, fora dos trechos de outros efeitos."""
    s = det[j]["inicio"]
    a = max(0, s - int(janela_s * FS))
    livre = np.ones(s - a, bool)
    for o in det:
        oa, ob = o["inicio"], o["inicio"] + o.get("tocou", 0) + int(0.03 * FS)
        if ob > a and oa < s:
            livre[max(0, oa - a):max(0, min(s, ob) - a)] = False
    seg = x[a:s][livre]
    if len(seg) < 0.2 * FS:
        return None
    return db(np.sqrt(np.mean(seg ** 2)))


def analisar(arq, pasta_ef, taxa, saida_json=None, limiar=0.5):
    bruto, fs = ler_wav(arq)
    assert fs == FS, "esperada gravação a 44 100 Hz"
    x = passa_altas(bruto, FS)
    x2 = passa_altas(decimar4(bruto), F2, 0.05)
    efeitos = ler_efeitos(pasta_ef)
    cands = candidatos(x2, efeitos, taxa)
    fortes = [c["ganho"] for c in cands if c["ncc"] > 0.95]
    ganho = float(np.median(fortes)) if fortes else 0.25
    det = escolher(cands, ganho, limiar)
    refinar(x, det, efeitos, taxa)
    det.sort(key=lambda d: d["inicio"])
    for d in det:
        _, d["inicio"], d["fase"] = alinhar_fino(bruto, d, efeitos[d["id"]], taxa)
    for j, d in enumerate(det):
        n = len(efeitos[d["id"]])
        d["natural"] = len(d["h"])
        d["estados"], B = blocos(bruto, d["inicio"], d["fase"], efeitos[d["id"]], taxa, ganho)
        d["tocou"] = fim_tocado(d["estados"], B)
        prox = det[j + 1] if j + 1 < len(det) else None
        d["ate_proximo"] = prox["inicio"] - d["inicio"] if prox else None
        piso = n // BLOCO_DRIVER * BLOCO_DRIVER * FS / taxa
        fim = d["inicio"] + d["tocou"]
        if prox and -0.015 * FS <= prox["inicio"] - fim <= 0.06 * FS and prox["inicio"] < d["inicio"] + d["natural"]:
            d["fim"] = f"cortado por {prox['id']}"
        elif d["tocou"] >= d["natural"] - 0.015 * FS:
            d["fim"] = "inteiro"
        elif piso < d["natural"] and abs(d["tocou"] - piso) <= 0.012 * FS:
            d["fim"] = f"até o bloco de {BLOCO_DRIVER} ({n // BLOCO_DRIVER * BLOCO_DRIVER} de {n} amostras)"
        else:
            d["fim"] = "parou antes, sem efeito audível depois"
        tocado = d["h"][:max(d["tocou"], int(0.02 * FS))]
        d["nivel_dbfs"] = db(ganho * np.sqrt(np.mean(tocado ** 2)))
    for j, d in enumerate(det):
        d["musica_dbfs"] = musica_em_volta(x, det, j)
    linhas = []
    for d in det:
        linhas.append({
            "t": round(d["inicio"] / FS, 4), "id": d["id"], "ncc": round(d["ncc"], 3),
            "ganho_db": round(db(d["ganho"]), 2), "explica": round(d["explica"], 3),
            "natural_ms": round(d["natural"] / FS * 1000, 1), "tocou_ms": round(d["tocou"] / FS * 1000, 1),
            "ate_proximo_ms": round(d["ate_proximo"] / FS * 1000, 1) if d["ate_proximo"] else None,
            "fim": d["fim"], "blocos_2ms": d["estados"], "nivel_dbfs": round(d["nivel_dbfs"], 1),
            "musica_dbfs": round(d["musica_dbfs"], 1) if d["musica_dbfs"] is not None else None,
        })
    resumo = resumir(linhas, ganho, taxa)
    imprimir(linhas, resumo)
    if saida_json:
        with open(saida_json, "w", encoding="utf-8") as f:
            json.dump({"arquivo": os.path.basename(arq), "taxa": taxa, "ganho_db": db(ganho),
                       "eventos": linhas, "resumo": resumo}, f, ensure_ascii=False, indent=1)


def resumir(linhas, ganho, taxa):
    por = defaultdict(list)
    for l in linhas:
        por[l["id"]].append(l)
    tabela = []
    for i in sorted(por):
        ls = por[i]
        fins = Counter(l["fim"] for l in ls)
        mus = [l["nivel_dbfs"] - l["musica_dbfs"] for l in ls if l["musica_dbfs"] is not None and l["musica_dbfs"] > -60]
        tabela.append({
            "id": i, "n": len(ls), "ganho_db": round(float(np.mean([l["ganho_db"] for l in ls])), 1),
            "nivel_dbfs": round(float(np.mean([l["nivel_dbfs"] for l in ls])), 1),
            "efeito_menos_musica_db": round(float(np.median(mus)), 1) if mus else None,
            "fins": dict(fins), "instantes": [l["t"] for l in ls],
        })
    # sequências: efeitos a menos de 1,5 s um do outro
    seqs, atual = [], []
    for l in linhas:
        if atual and l["t"] - atual[-1]["t"] > 1.5:
            seqs.append(atual)
            atual = []
        atual.append(l)
    if atual:
        seqs.append(atual)
    linha_tempo = []
    for s in seqs:
        intervalos = [round((b["t"] - a["t"]) * 1000) for a, b in zip(s, s[1:])]
        linha_tempo.append({"t": s[0]["t"], "ids": [l["id"] for l in s], "intervalos_ms": intervalos})
    return {"taxa": taxa, "ganho_sessao_db": round(db(ganho), 2), "por_efeito": tabela, "linha_do_tempo": linha_tempo}


def imprimir(linhas, resumo):
    print(f"ganho da sessão: {resumo['ganho_sessao_db']} dB (escala cheia de 8 bits); taxa {resumo['taxa']} Hz")
    print("\n   t (s)   id   NCC  ganho  tocou/natural (ms)  até o próximo  fim                 efeito/música dBFS")
    for l in linhas:
        mus = "—" if l["musica_dbfs"] is None else f"{l['musica_dbfs']:.1f}"
        prox = "—" if l["ate_proximo_ms"] is None else f"{l['ate_proximo_ms']:.0f}"
        print(f"{l['t']:9.3f} {l['id']:4d} {l['ncc']:.3f} {l['ganho_db']:6.1f} {l['tocou_ms']:7.0f}/{l['natural_ms']:<6.0f}"
              f" {prox:>10}  {l['fim']:<42} {l['nivel_dbfs']:.1f} / {mus}")
    print("\n id   n  ganho  nível  efeito−música  fins")
    for t in resumo["por_efeito"]:
        print(f"{t['id']:3d} {t['n']:3d} {t['ganho_db']:6.1f} {t['nivel_dbfs']:6.1f} {str(t['efeito_menos_musica_db']):>8}   {t['fins']}")
    print("\nlinha do tempo (efeitos a menos de 1,5 s um do outro):")
    for s in resumo["linha_do_tempo"]:
        print(f"{s['t']:9.3f}  {s['ids']}  {s['intervalos_ms']}")


# ------------------------------------------------------------------------------------------------
# Taxa
# ------------------------------------------------------------------------------------------------

def medir_taxa(arq, pasta_ef, t, ident, de, ate, passo, dur_s=None):
    """NCC do efeito `ident` em volta de `t` para cada taxa (modelos linear e degrau)."""
    bruto, fs = ler_wav(arq)
    efeitos = ler_efeitos(pasta_ef)
    a = int((t - 0.05) * fs)
    res = {}
    for modo in ("linear", "degrau"):
        melhores = []
        for r in np.arange(de, ate + 1e-9, passo):
            h = passa_altas(modelo(efeitos[ident], r, fs, modo), fs)
            if dur_s:
                h = h[:int(dur_s * fs)]
            seg = passa_altas(bruto[a:a + len(h) + int(0.1 * fs)], fs)
            if len(seg) <= len(h):
                continue
            _, nc, _ = correlacao(seg, h)
            k = int(np.argmax(nc))
            melhores.append((float(nc[k]), float(r), (a + k) / fs))
        melhores.sort(reverse=True)
        res[modo] = melhores[:5]
        print(modo, [(round(n, 4), round(r, 1), round(tt, 4)) for n, r, tt in melhores[:5]])
    return res


# ------------------------------------------------------------------------------------------------
# Músicas
# ------------------------------------------------------------------------------------------------

def achar_musicas(arq, pasta):
    """Onde cada música gravada sem input aparece na sessão (trecho de 4 s mais forte de cada uma)."""
    bruto, _ = ler_wav(arq)
    x2 = passa_altas(decimar4(bruto), F2, 0.05)
    N = 1 << int(np.ceil(np.log2(len(x2) + 4 * F2)))
    X = np.fft.rfft(x2, N)
    cs2 = np.concatenate([[0.0], np.cumsum(x2 * x2)])
    achados = []
    for nome in sorted(os.listdir(pasta)):
        if not nome.lower().endswith(".wav"):
            continue
        d, fs = ler_wav(os.path.join(pasta, nome))
        d = passa_altas(decimar4(d) if fs == FS else d, F2, 0.05)
        L = 4 * F2
        if len(d) <= L:
            continue
        cs = np.concatenate([[0.0], np.cumsum(d * d)])
        k0 = int(np.argmax(cs[L:] - cs[:-L]))
        h = d[k0:k0 + L]
        c = np.fft.irfft(X * np.conj(np.fft.rfft(h, N)), N)[:len(x2) - L + 1]
        nc = c / np.sqrt(np.maximum(cs2[L:] - cs2[:-L], 1e-15) * (h @ h))
        v = nc.copy()
        while True:
            k = int(np.argmax(v))
            if v[k] < 0.45:
                break
            achados.append((round((k - k0) / F2, 2), nome[:-4], round(float(v[k]), 3), round(db(c[k] / (h @ h)), 1)))
            v[max(0, k - 5 * F2):k + 5 * F2] = 0
    achados.sort()
    print("início (s)  música     NCC   ganho em relação à gravação sem input (dB)")
    for t, n, v, g in achados:
        print(f"{t:9.2f}  {n:10s} {v:.3f} {g:+.1f}")
    return achados


def main():
    import sys
    sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    a = sub.add_parser("analisar")
    a.add_argument("wav")
    a.add_argument("efeitos")
    a.add_argument("--taxa", type=float, default=5000)
    a.add_argument("--limiar", type=float, default=0.5)
    a.add_argument("--json")
    t = sub.add_parser("taxa")
    t.add_argument("wav")
    t.add_argument("efeitos")
    t.add_argument("instante", type=float)
    t.add_argument("id", type=int)
    t.add_argument("--de", type=float, default=3500)
    t.add_argument("--ate", type=float, default=12000)
    t.add_argument("--passo", type=float, default=50)
    t.add_argument("--duracao", type=float)
    m = sub.add_parser("musicas")
    m.add_argument("wav")
    m.add_argument("pasta")
    p = ap.parse_args()
    if p.cmd == "analisar":
        analisar(p.wav, p.efeitos, p.taxa, p.json, p.limiar)
    elif p.cmd == "taxa":
        medir_taxa(p.wav, p.efeitos, p.instante, p.id, p.de, p.ate, p.passo, p.duracao)
    else:
        achar_musicas(p.wav, p.pasta)


if __name__ == "__main__":
    main()
