"""Grava o áudio do SimAnt original no DOSBox-X (SB16 + OPL3, Windows 3.1 do usuário) e compara
com as músicas exportadas pelo port. Ver docs/spec/regras/som.md, "Áudio do original no DOSBox-X".

Nada toca no PC: o DOSBox-X roda com `[mixer] nosound = true` (o som continua emulado e a captura
funciona) e `SDL_AUDIODRIVER=dummy`; a janela abre minimizada e sem ativação. Nenhuma tecla ou clique
do usuário é usado; teclas para o jogo, se pedidas, vão pelo AUTOTYPE do próprio DOSBox-X.

Os arquivos do jogo são copiados para re_trabalho/som/C (fora do git); SimAnt-T não é alterado.

Uso:
  python -I tools/som_dosbox.py capturar SAIDA.wav [--segundos 120] [--video] [--som-inicio EFEITO.wav]
                                    [--autotype "-w 40 -p 1 enter"]
  python -I tools/som_dosbox.py musicas PASTA [NOME ...]     (as outras músicas, sem input)
  python -I tools/som_dosbox.py midi ARQ.mid [--canais 1-10] [--ate 10]
  python -I tools/som_dosbox.py niveis ARQ.wav [--janela 1]
  python -I tools/som_dosbox.py comparar ORIGINAL.wav PORT.wav [--midi ARQ.mid] [--png saida.png]
  python -I tools/som_dosbox.py harmonicos ARQ.mid ORIGINAL.wav INICIO_S PORT.wav CANAL
  python -I tools/som_dosbox.py comparar_todas ORIGINAIS PORT/musicas saida.json
  python -I tools/som_dosbox.py perfil ORIGINAIS PORT/musicas PROGRAMA nome,nome,...
  python -I tools/som_dosbox.py percussao ORIGINAIS PORT/musicas nome,nome,...
  python -I tools/som_dosbox.py quadros VIDEO.avi PASTA [--cada 5]
"""
import argparse
import os
import shutil
import struct
import subprocess
import sys
import time

import numpy as np

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TRABALHO = os.path.join(RAIZ, "re_trabalho", "som")


def raiz_principal():
    """Pasta do checkout principal (os worktrees ficam em .claude/worktrees/<nome>)."""
    partes = os.path.normpath(RAIZ).split(os.sep)
    if ".claude" in partes:
        return os.sep.join(partes[: partes.index(".claude")])
    return RAIZ


# ---------------------------------------------------------------------------------------------
# Captura
# ---------------------------------------------------------------------------------------------

CONF = r"""[sdl]
output = surface
autolock = false
windowresolution = original
[log]
logfile = {trab}\dosbox.log
[dosbox]
machine = svga_s3trio64
memsize = 32
captures = {trab}\cap
show recorded filename = false
priority = normal,normal
[cpu]
core = normal
cputype = pentium
cycles = fixed 30000
[mixer]
nosound = true
rate = 44100
[sblaster]
sbtype = sb16
sbbase = 220
irq = 7
dma = 1
hdma = 5
oplmode = auto
oplemu = {oplemu}
[midi]
mpu401 = intelligent
mididevice = none
[dos]
ver = 6.22
# layout US: com o automatico (BR nesta maquina) o AUTOTYPE nao entrega nada (som.md)
keyboardlayout = us
[autoexec]
mount c "{trab}\C"
c:
set PATH=C:\WINDOWS
set TEMP=C:\WINDOWS\TEMP
set BLASTER=A220 I7 D1 H5 P330 T6
{autotype}
cd \WINDOWS
DX-CAPTURE {video}/A win C:\MAXIS\ANTWIN\SIMANTW.EXE
"""


def preparar_copia(origem):
    destino = os.path.join(TRABALHO, "C")
    if not os.path.isdir(destino):
        print(f"copiando {origem} -> {destino}")
        shutil.copytree(origem, destino)
    os.makedirs(os.path.join(TRABALHO, "cap"), exist_ok=True)
    return destino


def corrigir_wav(arq):
    """O DOSBox-X morto pelo -time-limit pode deixar o cabeçalho RIFF desatualizado."""
    with open(arq, "r+b") as f:
        d = f.read(44)
        tam = os.path.getsize(arq)
        if d[:4] != b"RIFF" or d[36:40] != b"data":
            return
        f.seek(4)
        f.write(struct.pack("<I", tam - 8))
        f.seek(40)
        f.write(struct.pack("<I", tam - 44))


def som_de_inicio(copia, wav):
    """Põe `wav` como som "Windows Start" do WIN.INI da cópia (o mesmo waveOut que o jogo usa),
    para medir o nível de um efeito na mesma gravação da música. `None` desliga."""
    ini = os.path.join(copia, "WINDOWS", "WIN.INI")
    linhas = open(ini, encoding="latin-1").read().splitlines()
    valor = ""
    if wav:
        shutil.copyfile(wav, os.path.join(copia, "WINDOWS", "SOMTESTE.WAV"))
        valor = r"C:\WINDOWS\SOMTESTE.WAV"
    linhas = [f"SystemStart={valor},Windows Start" if x.lower().startswith("systemstart=") else x for x in linhas]
    with open(ini, "w", encoding="latin-1", newline="\r\n") as f:
        f.write("\n".join(linhas) + "\n")


def capturar(saida, segundos, autotype, oplemu, video=False, som_inicio=None):
    origem = os.path.join(raiz_principal(), "SimAnt-T")
    copia = preparar_copia(origem)
    som_de_inicio(copia, som_inicio)
    cap = os.path.join(TRABALHO, "cap")
    for x in os.listdir(cap):
        os.remove(os.path.join(cap, x))
    # AUTOTYPE roda em segundo plano no DOS e digita depois de -w segundos (tempo do emulador).
    linha = f"AUTOTYPE {autotype}" if autotype else ""
    conf = os.path.join(TRABALHO, "som.conf")
    with open(conf, "w", encoding="ascii") as f:
        f.write(CONF.format(trab=TRABALHO, autotype=linha, oplemu=oplemu, video="/V " if video else ""))
    exe = os.path.join(raiz_principal(), "referencia", "dosbox-x", "bin", "x64", "Release", "dosbox-x.exe")
    amb = dict(os.environ, SDL_AUDIODRIVER="dummy", SDL_WINDOW_NO_ACTIVATION_WHEN_SHOWN="1")
    si = subprocess.STARTUPINFO()
    si.dwFlags |= subprocess.STARTF_USESHOWWINDOW
    si.wShowWindow = 7  # SW_SHOWMINNOACTIVE
    args = [exe, "-conf", conf, "-nopromptfolder", "-fastlaunch", "-time-limit", str(segundos)]
    p = subprocess.Popen(args, cwd=TRABALHO, env=amb, startupinfo=si)
    minimizar(p.pid)
    p.wait()
    wavs = sorted((os.path.join(cap, x) for x in os.listdir(cap) if x.lower().endswith(".wav")), key=os.path.getmtime)
    if not wavs:
        sys.exit("nenhum WAV gravado (ver re_trabalho/som/dosbox.log)")
    corrigir_wav(wavs[-1])
    shutil.copyfile(wavs[-1], saida)
    avis = sorted((os.path.join(cap, x) for x in os.listdir(cap) if x.lower().endswith(".avi")), key=os.path.getsize)
    if avis:
        shutil.copyfile(avis[-1], os.path.splitext(saida)[0] + ".avi")
        print(f"vídeo: {os.path.splitext(saida)[0]}.avi (quadros-chave: subcomando 'quadros')")
    print(f"{saida}: {(os.path.getsize(saida) - 44) / 176400:.1f} s")


def capturar_musicas(pasta, nomes, oplemu, folga=20):
    """Grava as outras músicas pelo mesmo caminho da abertura, sem input: na CÓPIA do jogo
    (re_trabalho/som/C), o GAMETHME.MID é trocado pela música pedida, e o jogo a toca ao abrir (MCI →
    MIDI Mapper → sb16fm.drv, o mesmo caminho de todas as músicas no Windows). O original é reposto no
    fim. Uma gravação por música, com a duração das notas do MIDI + `folga` s."""
    copia = preparar_copia(os.path.join(raiz_principal(), "SimAnt-T"))
    som = os.path.join(copia, "MAXIS", "ANTWIN", "SOUND")
    arqs = {x.upper(): os.path.join(som, x) for x in os.listdir(som)}
    tema = arqs["GAMETHME.MID"]
    guarda = os.path.join(TRABALHO, "GAMETHME.ORIGINAL.MID")
    if not os.path.exists(guarda):
        shutil.copyfile(tema, guarda)
    os.makedirs(pasta, exist_ok=True)
    try:
        for nome in nomes:
            arq = arqs[nome.upper() + ".MID"]
            fim = max(t + d for t, _, _, _, d, _ in ler_midi(arq))
            if os.path.normcase(arq) != os.path.normcase(tema):
                shutil.copyfile(arq, tema)
            else:
                shutil.copyfile(guarda, tema)
            saida = os.path.join(pasta, nome.lower() + ".wav")
            print(f"{nome}: notas até {fim:.1f} s")
            capturar(saida, int(fim + folga), "", oplemu)
    finally:
        shutil.copyfile(guarda, tema)


def minimizar(pid):
    """Minimiza (sem ativar) as janelas do processo assim que aparecem."""
    import ctypes
    from ctypes import wintypes

    u = ctypes.windll.user32
    proto = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    achadas = []

    def cb(h, _):
        p = wintypes.DWORD()
        u.GetWindowThreadProcessId(h, ctypes.byref(p))
        if p.value == pid and u.IsWindowVisible(h):
            achadas.append(h)
        return True

    for _ in range(80):
        achadas.clear()
        u.EnumWindows(proto(cb), 0)
        if achadas:
            for h in achadas:
                u.ShowWindow(h, 7)
            return
        time.sleep(0.1)


# ---------------------------------------------------------------------------------------------
# WAV e MIDI
# ---------------------------------------------------------------------------------------------


def ler_wav(arq):
    """(taxa, mono float em -1..1)."""
    d = open(arq, "rb").read()
    i = 12
    fmt = None
    while i + 8 <= len(d):
        cid, tam = d[i : i + 4], struct.unpack("<I", d[i + 4 : i + 8])[0]
        if cid == b"fmt ":
            fmt = struct.unpack("<HHIIHH", d[i + 8 : i + 24])
        elif cid == b"data":
            dados = d[i + 8 : i + 8 + tam]
            break
        i += 8 + tam + (tam & 1)
    codigo, canais, taxa, _, _, bits = fmt
    if codigo == 3:
        x = np.frombuffer(dados[: len(dados) // 4 * 4], dtype="<f4").astype(float)
    elif bits == 16:
        x = np.frombuffer(dados[: len(dados) // 2 * 2], dtype="<i2").astype(float) / 32768
    else:
        x = (np.frombuffer(dados, dtype=np.uint8).astype(float) - 128) / 128
    x = x[: len(x) // canais * canais].reshape(-1, canais).mean(1)
    return taxa, x


def ler_midi(arq):
    """Notas [(segundos, canal 1-16, nota, velocidade, duração, programa)] de um SMF formato 0/1."""
    d = open(arq, "rb").read()
    assert d[:4] == b"MThd"
    _, ntrilhas, divisao = struct.unpack(">HHH", d[8:14])
    i = 14
    eventos = []  # (tick, ordem, tipo, dados)
    for _ in range(ntrilhas):
        tam = struct.unpack(">I", d[i + 4 : i + 8])[0]
        t = d[i + 8 : i + 8 + tam]
        i += 8 + tam
        j, tick, status = 0, 0, 0

        def vlq():
            nonlocal j
            v = 0
            while True:
                b = t[j]
                j += 1
                v = (v << 7) | (b & 0x7F)
                if b < 0x80:
                    return v

        while j < len(t):
            tick += vlq()
            b = t[j]
            if b >= 0x80:
                status = b
                j += 1
            if status == 0xFF:
                tipo = t[j]
                j += 1
                n = vlq()
                if tipo == 0x51:
                    eventos.append((tick, len(eventos), "tempo", int.from_bytes(t[j : j + 3], "big")))
                j += n
            elif status in (0xF0, 0xF7):
                j += vlq()
            else:
                n = 1 if status & 0xF0 in (0xC0, 0xD0) else 2
                dados = t[j : j + n]
                j += n
                eventos.append((tick, len(eventos), "canal", (status, *dados)))
    eventos.sort()
    tempo, ult_tick, seg = 500000, 0, 0.0
    abertas, notas, programa = {}, [], {}
    for tick, _, tipo, v in eventos:
        seg += (tick - ult_tick) * tempo / divisao / 1e6
        ult_tick = tick
        if tipo == "tempo":
            tempo = v
            continue
        st, c = v[0] & 0xF0, (v[0] & 0x0F) + 1
        if st == 0xC0:
            programa[c] = v[1]
        if st == 0x90 and v[2] > 0:
            abertas[(c, v[1])] = (seg, v[2])
        elif st in (0x80, 0x90):
            if (c, v[1]) in abertas:
                ini, vel = abertas.pop((c, v[1]))
                notas.append((ini, c, v[1], vel, seg - ini, programa.get(c, 0)))
    notas.sort()
    return notas


def canais(txt):
    r = set()
    for p in txt.split(","):
        a, _, b = p.partition("-")
        r.update(range(int(a), int(b or a) + 1))
    return r


# ---------------------------------------------------------------------------------------------
# Análise
# ---------------------------------------------------------------------------------------------

QUADRO = 2048
PASSO = 512


def espectrograma(x, taxa):
    n = 1 + max(0, (len(x) - QUADRO) // PASSO)
    janela = np.hanning(QUADRO)
    idx = np.arange(QUADRO)[None, :] + PASSO * np.arange(n)[:, None]
    q = x[np.minimum(idx, len(x) - 1)] * janela
    return np.abs(np.fft.rfft(q, axis=1)), np.fft.rfftfreq(QUADRO, 1 / taxa)


def croma(mag, freqs):
    """Energia por classe de altura (12) por quadro, de 55 Hz a 2 kHz."""
    ok = (freqs >= 55) & (freqs <= 2000)
    classe = (np.round(12 * np.log2(freqs[ok] / 440.0)) % 12).astype(int)
    c = np.zeros((mag.shape[0], 12))
    for k in range(12):
        c[:, k] = (mag[:, ok][:, classe == k] ** 2).sum(1)
    return c / (np.linalg.norm(c, axis=1, keepdims=True) + 1e-12)


def fluxo(mag):
    """Força de ataque (fluxo espectral positivo, log)."""
    lm = np.log1p(100 * mag)
    f = np.maximum(0, np.diff(lm, axis=0)).sum(1)
    f = np.concatenate([[0], f])
    return (f - f.mean()) / (f.std() + 1e-12)


def trecho_com_som(x, taxa, limiar=0.002):
    """(início, fim) em segundos do trecho em que o RMS de 50 ms passa do limiar."""
    n = int(taxa * 0.05)
    r = np.sqrt(np.convolve(x**2, np.ones(n) / n, mode="same"))
    ix = np.nonzero(r > limiar)[0]
    if len(ix) == 0:
        return 0.0, 0.0
    return ix[0] / taxa, ix[-1] / taxa


def reamostrar(x, de, para):
    if de == para:
        return x
    n = int(len(x) * para / de)
    return np.interp(np.arange(n) * de / para, np.arange(len(x)), x)


def passa_altas(x, taxa, corte=20.0):
    """Tira o que fica abaixo de `corte` Hz (CC e subsônicos). O DOSBox-X grava a componente contínua
    das ondas de meia senoide do OPL (a placa real tinha saída acoplada por capacitor) e o port também a
    gera; não se ouve, mas pesa no RMS (ex.: +5 dB no jolly2 do port antes do ajuste de 2026-10-19)."""
    X = np.fft.rfft(x)
    X[np.fft.rfftfreq(len(x), 1 / taxa) < corte] = 0
    return np.fft.irfft(X, len(x))


def melhor_alinhamento(fa, fb, centro, max_desloc):
    """Deslocamento (quadros) de b em relação a a que maximiza a correlação dos ataques."""
    melhor = (-1e9, 0)
    for k in range(centro - max_desloc, centro + max_desloc + 1):
        if k >= 0:
            a, b = fa[k:], fb[: len(fa) - k]
        else:
            a, b = fa[: len(fa) + k], fb[-k:]
        m = min(len(a), len(b))
        if m < 50:
            continue
        c = float(np.dot(a[:m], b[:m]) / m)
        melhor = max(melhor, (c, k))
    return melhor[1], melhor[0]


def semelhanca(ca, cb, ativo, k):
    """Média do produto escalar das cromas com cb deslocado k quadros (só quadros ativos de ca)."""
    if k >= 0:
        a, b, at = ca[k:], cb, ativo[k:]
    else:
        a, b, at = ca, cb[-k:], ativo
    m = min(len(a), len(b))
    if m < 50:
        return -1.0
    at = at[:m]
    return float((a[:m][at] * b[:m][at]).sum(1).mean())


def niveis(arq, janela):
    taxa, x = ler_wav(arq)
    n = int(taxa * janela)
    for i in range(0, len(x) // n):
        s = x[i * n : (i + 1) * n]
        print(f"{i * janela:7.1f} s  RMS {np.sqrt((s**2).mean()):.4f}  pico {np.abs(s).max():.3f}")


def comparar(orig, port, arq_midi=None, png=None, desloc_max_s=2.0):
    taxa_o, xo = ler_wav(orig)
    taxa_p, xp = ler_wav(port)
    taxa = 22050
    xo, xp = passa_altas(reamostrar(xo, taxa_o, taxa), taxa), passa_altas(reamostrar(xp, taxa_p, taxa), taxa)
    (io, fo), (ip, fp) = trecho_com_som(xo, taxa), trecho_com_som(xp, taxa)
    print(f"original: {len(xo) / taxa:.1f} s gravados, som de {io:.2f} a {fo:.2f} s")
    print(f"port:     {len(xp) / taxa:.1f} s gravados, som de {ip:.2f} a {fp:.2f} s ({fp - ip:.2f} s)")
    mo, fr = espectrograma(xo, taxa)
    mp, _ = espectrograma(xp, taxa)
    qps = taxa / PASSO
    co_t, cp_t = croma(mo, fr), croma(mp, fr)
    eo_t = (mo**2).sum(1)
    ativo_t = eo_t > np.percentile(eo_t, 30)
    # Andamento e deslocamento: escala do port que maximiza a semelhança de croma com o original,
    # procurando o deslocamento perto da diferença entre os inícios do som.
    resultados = []
    for escala in np.arange(0.97, 1.0301, 0.0025):
        n = int(len(cp_t) * escala)
        cp_e = np.stack([np.interp(np.arange(n) / escala, np.arange(len(cp_t)), cp_t[:, j]) for j in range(12)], 1)
        centro = int(round((io - ip * escala) * qps))
        for k in range(centro - int(desloc_max_s * qps), centro + int(desloc_max_s * qps) + 1):
            s_ = semelhanca(co_t, cp_e, ativo_t, k)
            resultados.append((s_, escala, k))
    sim_m, escala, k = max(resultados)
    sim1 = max(r for r in resultados if abs(r[1] - 1.0) < 1e-9)
    print(f"andamento: melhor escala do port {escala:.4f} (croma {sim_m:.3f}, deslocamento {k / qps:.2f} s); "
          f"com 1,0000: croma {sim1[0]:.3f}, deslocamento {sim1[2] / qps:.2f} s")
    controle = np.mean([semelhanca(co_t, np.roll(cp_t, r, axis=0), ativo_t, sim1[2]) for r in (37, 91, 173)])
    res = dict(dur_o=fo - io, dur_p=fp - ip, escala_melhor=float(escala), croma_melhor=float(sim_m), croma1=float(sim1[0]),
               controle=float(controle))
    print(f"   controle (port deslocado 0,9-4 s): croma {controle:.3f}")
    k = sim1[2]
    notas = [x for x in ler_midi(arq_midi) if x[1] <= 9] if arq_midi else []
    if notas:
        rolo = np.zeros((len(mo), 12))
        for t, _, nota, vel, dur, _ in notas:
            a_ = int((t + io) * qps)
            b_ = int((t + dur + io) * qps) + 1
            rolo[max(0, a_) : max(0, min(len(rolo), b_)), nota % 12] += vel
        rolo /= np.linalg.norm(rolo, axis=1, keepdims=True) + 1e-12
        tem = (rolo.sum(1) > 0) & ativo_t
        melhor = max((float((co_t[tem.nonzero()[0]] * np.roll(rolo, d, axis=0)[tem.nonzero()[0]]).sum(1).mean()), d)
                     for d in range(-int(qps), int(qps) + 1))
        ctrl = np.mean([float((co_t[tem] * np.roll(rolo, melhor[1] + r, axis=0)[tem]).sum(1).mean()) for r in (37, 91, 173)])
        res["croma_midi"], res["controle_midi"] = float(melhor[0]), float(ctrl)
        print(f"notas do MIDI (canais 1-9, croma da transcrição) contra o original: {melhor[0]:.3f} "
              f"(ajuste {melhor[1] / qps:+.2f} s); controle deslocado {ctrl:.3f}")
        fim_m = max(x[0] + x[4] for x in notas)
        print(f"MIDI: {len(notas)} notas, última termina em {fim_m:.2f} s; som do original dura {fo - io:.2f} s")
    if k >= 0:
        a, b = mo[k:], mp
    else:
        a, b = mo, mp[-k:]
    m = min(len(a), len(b))
    a, b = a[:m], b[:m]
    eo = (a**2).sum(1)
    ativo = eo > np.percentile(eo, 30)
    sim = (croma(a, fr) * croma(b, fr)).sum(1)
    # Envelope (energia por quadro em dB, alinhada): serve também para a percussão pura.
    la, lb = 10 * np.log10((a**2).sum(1) + 1e-9), 10 * np.log10((b**2).sum(1) + 1e-9)
    res["envelope"] = float(np.corrcoef(la, lb)[0, 1])
    # Nível no mesmo trecho alinhado (soma da energia dos quadros): não depende de onde cada gravação
    # acha o fim do som (caudas de soltura longas baixavam o RMS "no trecho com som" do port).
    res["energia_alinhada_db"] = float(10 * np.log10((b**2).sum() / (a**2).sum()))
    res["envelope_controle"] = float(np.mean([np.corrcoef(la, np.roll(lb, r))[0, 1] for r in (37, 91, 173)]))
    # Timbre: centróide espectral e distribuição de energia por faixa.
    def centroide(mag):
        e = mag**2
        return (e * fr).sum(1) / (e.sum(1) + 1e-12)

    print(f"centróide espectral médio: original {centroide(a)[ativo].mean():.0f} Hz, port {centroide(b)[ativo].mean():.0f} Hz")
    faixas = [(0, 200), (200, 500), (500, 1000), (1000, 2000), (2000, 4000), (4000, 11025)]
    def dist(mag):
        e = (mag[ativo] ** 2).sum(0)
        v = np.array([e[(fr >= lo) & (fr < hi)].sum() for lo, hi in faixas])
        return v / v.sum()

    do, dp = dist(a), dist(b)
    print("energia por faixa (Hz):  " + "  ".join(f"{lo}-{hi}" for lo, hi in faixas))
    print("   original:             " + "  ".join(f"{v:6.1%}" for v in do))
    print("   port:                 " + "  ".join(f"{v:6.1%}" for v in dp))
    # Espectro médio em dB (envelope do timbre), distância simples.
    so = 10 * np.log10((a[ativo] ** 2).mean(0) + 1e-12)
    sp = 10 * np.log10((b[ativo] ** 2).mean(0) + 1e-12)
    ok = (fr > 60) & (fr < 8000)
    so, sp = so[ok] - so[ok].max(), sp[ok] - sp[ok].max()
    res.update(cent_o=float(centroide(a)[ativo].mean()), cent_p=float(centroide(b)[ativo].mean()), dist_o=do.tolist(), dist_p=dp.tolist(),
               esp_corr=float(np.corrcoef(so, sp)[0, 1]), esp_dif=float(np.abs(so - sp).mean()))
    print(f"espectro médio (60 Hz-8 kHz): correlação {np.corrcoef(so, sp)[0, 1]:.3f}, diferença média {np.abs(so - sp).mean():.1f} dB")
    ro = np.sqrt((xo[int(io * taxa) : int(fo * taxa)] ** 2).mean())
    rp = np.sqrt((xp[int(ip * taxa) : int(fp * taxa)] ** 2).mean())
    print(f"RMS no trecho com som: original {ro:.4f} ({20 * np.log10(ro):.1f} dBFS), port {rp:.4f} ({20 * np.log10(rp):.1f} dBFS)")
    res.update(rms_o=float(20 * np.log10(ro)), rms_p=float(20 * np.log10(rp)))
    if png:
        desenhar(png, a, b, fr, qps)
    res.update(escala=float(escala), desloc_s=k / qps, croma=float(sim[ativo].mean()))
    return res


def comparar_todas(pasta_orig, pasta_port, pasta_midi, saida_json):
    """Compara cada WAV de `pasta_orig` (nome.wav, de `musicas`) com o do port (ID_nome.wav de
    --exportar-som) e grava as medidas em JSON; imprime uma tabela."""
    import io
    import json
    from contextlib import redirect_stdout
    port = {x.split("_", 1)[1][:-4].lower(): os.path.join(pasta_port, x) for x in os.listdir(pasta_port) if x.endswith(".wav")}
    midis = {x[:-4].lower(): os.path.join(pasta_midi, x) for x in os.listdir(pasta_midi) if x.upper().endswith(".MID")}
    tudo = {}
    for x in sorted(os.listdir(pasta_orig)):
        nome = x[:-4].lower()
        if not x.endswith(".wav") or nome not in port:
            continue
        with redirect_stdout(io.StringIO()):
            try:
                tudo[nome] = comparar(os.path.join(pasta_orig, x), port[nome], midis.get(nome))
            except Exception as e:  # noqa: BLE001
                tudo[nome] = {"erro": str(e)}
        r = tudo[nome]
        if "erro" in r:
            print(f"{nome:10s} erro: {r['erro']}")
            continue
        print(f"{nome:10s} dur {r['dur_o']:6.2f}/{r['dur_p']:6.2f} s  croma {r['croma1']:.3f} (controle {r['controle']:.3f})  "
              f"centróide {r['cent_o']:5.0f}/{r['cent_p']:5.0f} Hz  2-4k {r['dist_o'][4]:.0%}/{r['dist_p'][4]:.0%}  "
              f"env {r['envelope']:.2f} ({r['envelope_controle']:.2f})  esp {r['esp_corr']:.2f}  RMS {r['rms_o']:6.1f}/{r['rms_p']:6.1f} dBFS")
    with open(saida_json, "w", encoding="utf-8") as f:
        json.dump(tudo, f, indent=1)


def _par_de_gravacoes(pasta_orig, pasta_port, nome, corte=None):
    """(taxa, sinal, início do som) do original (nome.wav) e do port (ID_nome.wav), e as notas do .MID."""
    snd = os.path.join(raiz_principal(), "SimAnt-T", "MAXIS", "ANTWIN", "SOUND")
    notas = [n for n in ler_midi(os.path.join(snd, nome.upper() + ".MID")) if n[1] <= 10]
    port = [f for f in os.listdir(pasta_port) if f.lower().endswith("_" + nome + ".wav")][0]
    sinais = {}
    for q, arq in (("original", os.path.join(pasta_orig, nome + ".wav")), ("port", os.path.join(pasta_port, port))):
        tx, x = ler_wav(arq)
        if corte:
            x = passa_altas(x, tx, corte)
        sinais[q] = (tx, x, trecho_com_som(x, tx)[0])
    return notas, sinais


def perfil_programa(pasta_orig, pasta_port, programa, nomes, janela=(0.03, 0.3), nh=12):
    """Perfil harmônico médio (dB relativos ao maior, harmônicos 1-nh) de um programa, nas janelas em
    que nenhuma outra nota começa (acordes e outros canais valem; percussão começando perto, não) e
    pulando os parciais que coincidem (3 %) com os de outra nota soando. Original e port alinhados pelo
    início do som. Ver docs/spec/regras/som.md, "Ajuste do banco FM contra as 28 músicas"."""
    res = {"original": [], "port": []}
    for nome in nomes:
        notas, sinais = _par_de_gravacoes(pasta_orig, pasta_port, nome)
        t0 = min(n[0] for n in notas)
        for n in notas:
            t, c, nota, vel, dur, prog = n
            if c == 10 or prog != programa or dur < janela[0] + 0.08:
                continue
            a, b = t + janela[0], t + min(dur, janela[1])
            outras = [m for m in notas if m is not n and m[0] < b + 0.02 and m[0] + m[4] + 0.15 > a]
            if any((m[1] == 10 and a - 0.15 < m[0] < b) or (m[1] != 10 and a - 0.1 < m[0] < b) for m in outras):
                continue
            f0 = 440 * 2 ** ((nota - 69) / 12)
            fs = [440 * 2 ** ((m[2] - 69) / 12) for m in outras if m[1] != 10]
            valido = [all(abs(h * f0 / (k * g) - 1) > 0.03 for g in fs for k in range(1, nh + 1)) for h in range(1, nh + 1)]
            if not any(valido):
                continue
            for q, (tx, x, io) in sinais.items():
                i0, i1 = int((io + a - t0) * tx), int((io + b - t0) * tx)
                seg = x[i0:i1] * np.hanning(i1 - i0)
                mag = np.abs(np.fft.rfft(seg, 8 * len(seg)))
                fr = np.fft.rfftfreq(8 * len(seg), 1 / tx)
                hh = np.array([mag[(fr > f0 * h * 0.985) & (fr < f0 * h * 1.015)].max() if f0 * h * 1.015 < fr[-1] else 1e-9
                               for h in range(1, nh + 1)])
                ref = max(hh[v] for v in range(nh) if valido[v])
                res[q].append(np.where(valido, 20 * np.log10(hh / ref + 1e-6), np.nan))
    for q, v in res.items():
        if v:
            with np.errstate(all="ignore"):
                m = np.nanmean(np.array(v), 0)
            print(f"programa {programa} {q:8s} ({len(v)} janelas) " + " ".join("    ." if np.isnan(x) else f"{x:5.0f}" for x in m))
        else:
            print(f"programa {programa} {q:8s}: nenhuma janela")


def percussao_notas(pasta_orig, pasta_port, nomes, livre=0.14):
    """Para cada nota do canal 10, nos inícios isolados (nenhum outro início de 30 ms antes a `livre`
    depois): o aumento de potência (máximo dos quadros de 20 ms depois menos o de antes, em dB relativos
    ao RMS da abertura), a frequência dominante e o centróide dos primeiros 55 ms. Sem som: < -80 dB.
    Os sinais passam antes por um passa-altas de 40 Hz (a componente contínua do DOSBox-X)."""
    ref = {}
    _, sinais = _par_de_gravacoes(pasta_orig, pasta_port, "gamethme", 40)
    for q, (tx, x, io) in sinais.items():
        a, b = trecho_com_som(x, tx)
        ref[q] = 10 * np.log10((x[int(a * tx) : int(b * tx)] ** 2).mean())
    res = {}
    for nome in nomes:
        notas, sinais = _par_de_gravacoes(pasta_orig, pasta_port, nome, 40)
        t0 = min(n[0] for n in notas)
        inicios = [n[0] for n in notas]
        for q, (tx, x, io) in sinais.items():
            def pot(a, b):
                return (x[int(a * tx) : int(b * tx)] ** 2).mean()

            for t, c, nota, vel, dur, prog in notas:
                if c != 10 or inicios.count(t) > 1 or any(u != t and t - 0.03 < u < t + livre for u in inicios):
                    continue
                tt = io + t - t0
                antes = pot(tt - 0.03, tt - 0.002)
                depois = [pot(tt + 0.005 + 0.02 * k, tt + 0.025 + 0.02 * k) for k in range(int(livre / 0.02) - 1)]
                s = x[int((tt + 0.005) * tx) : int((tt + 0.06) * tx)]
                m = np.abs(np.fft.rfft(s * np.hanning(len(s)), 4 * len(s))) ** 2
                f = np.fft.rfftfreq(4 * len(s), 1 / tx)
                res.setdefault((nota, q), []).append((10 * np.log10(max(max(depois) - antes, 1e-12)) - ref[q], f[m.argmax()],
                                                      (m * f).sum() / m.sum()))
    for nota in sorted({k[0] for k in res}):
        linha = f"nota {nota:3d}"
        for q in ("original", "port"):
            v = np.array(res.get((nota, q), [(np.nan,) * 3]))
            linha += f" | {q} n={len(v):3d} nível {np.median(v[:, 0]):6.1f} dB pico {np.median(v[:, 1]):5.0f} Hz centróide {np.median(v[:, 2]):5.0f} Hz"
        print(linha)


def desenhar(png, a, b, fr, qps):
    """Dois espectrogramas (log) um sobre o outro, em PNG cinza, sem dependências extras."""
    ok = fr <= 4000
    def img(m):
        v = 20 * np.log10(m[:, ok].T + 1e-9)
        v = np.clip((v - (v.max() - 70)) / 70, 0, 1)
        return (255 * (1 - v[::-1])).astype(np.uint8)

    ia, ib = img(a), img(b)
    sep = np.zeros((4, ia.shape[1]), np.uint8)
    tudo = np.vstack([ia, sep, ib])
    gravar_png(png, tudo, 0, tudo.shape[1])
    print(f"{png}: original em cima, port embaixo (0-4 kHz, {qps:.0f} quadros/s)")


def quadros_chave(avi, pasta, cada_s=5.0):
    """Extrai quadros-chave ZMBV (8 ou 32 bits) do AVI do DOSBox-X como PNG, um a cada `cada_s`."""
    import zlib

    d = open(avi, "rb").read()
    i = d.index(b"avih") + 8
    us_por_quadro = struct.unpack("<I", d[i : i + 4])[0]
    j = d.index(b"strf") + 8
    w, h = struct.unpack("<ii", d[j + 4 : j + 12])
    os.makedirs(pasta, exist_ok=True)
    pos, n, proximo = d.index(b"movi") + 4, 0, 0.0
    while pos + 8 <= len(d):
        cid, tam = d[pos : pos + 4], struct.unpack("<I", d[pos + 4 : pos + 8])[0]
        corpo = d[pos + 8 : pos + 8 + tam]
        pos += 8 + tam + (tam & 1)
        if cid == b"LIST":
            pos -= tam + (tam & 1) - 4
            continue
        if cid[2:] != b"dc":
            continue
        t = n * us_por_quadro / 1e6
        n += 1
        if not corpo or not (corpo[0] & 1) or t < proximo:
            continue
        fmt = corpo[4]
        cru = zlib.decompressobj().decompress(corpo[7:])
        if fmt == 4:  # 8 bits com paleta
            pal = np.frombuffer(cru[:768], np.uint8).reshape(256, 3)
            img = pal[np.frombuffer(cru[768 : 768 + w * h], np.uint8)].reshape(h, w, 3)
        elif fmt == 8:  # 32 bits BGRA
            img = np.ascontiguousarray(np.frombuffer(cru[: w * h * 4], np.uint8).reshape(h, w, 4)[:, :, 2::-1])
        else:
            continue
        gravar_png_rgb(os.path.join(pasta, f"q_{t:07.2f}.png"), img)
        proximo = t + cada_s
    print(f"{n} quadros ({1e6 / us_por_quadro:.2f}/s), quadros-chave em {pasta}")


def gravar_png_rgb(arq, img):
    """PNG RGB de 8 bits (sem filtro), só biblioteca padrão."""
    gravar_png(arq, img.reshape(img.shape[0], -1), 2, img.shape[1])


def gravar_png(arq, linhas, tipo, largura):
    import zlib

    cru = b"".join(bytes([0]) + linhas[y].tobytes() for y in range(linhas.shape[0]))

    def bloco(t, c):
        return struct.pack(">I", len(c)) + t + c + struct.pack(">I", zlib.crc32(t + c) & 0xFFFFFFFF)

    cab = struct.pack(">IIBBBBB", largura, linhas.shape[0], 8, tipo, 0, 0, 0)
    with open(arq, "wb") as f:
        f.write(bytes([0x89]) + b"PNG" + bytes([13, 10, 26, 10]) + bloco(b"IHDR", cab)
                + bloco(b"IDAT", zlib.compress(cru, 6)) + bloco(b"IEND", b""))


def harmonicos(midi, orig, desloc, port, canal):
    """Perfil harmônico médio (dB relativos ao maior, harmônicos 1-12) das notas isoladas de um
    canal (nenhuma outra nota melódica começa 0,35 s antes nem durante; a percussão é ignorada), no original e no port."""
    notas = [n for n in ler_midi(midi) if n[1] <= 10]
    to, xo = ler_wav(orig)
    tp, xp = ler_wav(port)
    res = {"original": [], "port": []}
    progs = set()
    for n in notas:
        t, c, nota, vel, dur, prog = n
        if c != canal or dur < 0.12:
            continue
        if any(m is not n and m[1] != 10 and t - 0.35 < m[0] < t + dur for m in notas):
            continue
        progs.add(prog)
        f0 = 440 * 2 ** ((nota - 69) / 12)
        for k, (tx, x, o) in {"original": (to, xo, desloc), "port": (tp, xp, 0.0)}.items():
            a, b = int((t + o + 0.03) * tx), int((t + o + min(dur, 0.25)) * tx)
            seg = x[a:b] * np.hanning(b - a)
            mag = np.abs(np.fft.rfft(seg, 8 * len(seg)))
            fr = np.fft.rfftfreq(8 * len(seg), 1 / tx)
            h = np.array([mag[(fr > f0 * i * 0.97) & (fr < f0 * i * 1.03)].max() for i in range(1, 13)])
            res[k].append(20 * np.log10(h / h.max() + 1e-6))
    print(f"canal {canal}, programa(s) {sorted(progs)}: {len(res['port'])} notas isoladas; harmônicos 1-12 (dB)")
    for k, v in res.items():
        if v:
            print(f"  {k:8s} " + " ".join(f"{x:5.0f}" for x in np.mean(v, 0)))


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("capturar")
    c.add_argument("saida")
    c.add_argument("--segundos", type=int, default=100)
    c.add_argument("--autotype", default="")
    c.add_argument("--oplemu", default="default")
    c.add_argument("--video", action="store_true")
    c.add_argument("--som-inicio", help="WAV tocado pelo Windows ao iniciar (som 'Windows Start')")
    mu = sub.add_parser("musicas")
    mu.add_argument("pasta")
    mu.add_argument("nomes", nargs="*", help="nomes dos .MID (padrão: todos menos GAMETHME)")
    mu.add_argument("--oplemu", default="default")
    q = sub.add_parser("quadros")
    q.add_argument("avi")
    q.add_argument("pasta")
    q.add_argument("--cada", type=float, default=5.0)
    m = sub.add_parser("midi")
    m.add_argument("arq")
    m.add_argument("--canais", default="1-16")
    m.add_argument("--ate", type=float, default=1e9)
    n = sub.add_parser("niveis")
    n.add_argument("arq")
    n.add_argument("--janela", type=float, default=1.0)
    h = sub.add_parser("harmonicos")
    h.add_argument("midi")
    h.add_argument("original")
    h.add_argument("desloc", type=float, help="início da música no original (s)")
    h.add_argument("port")
    h.add_argument("canal", type=int)
    pf = sub.add_parser("perfil")
    pf.add_argument("pasta_orig")
    pf.add_argument("pasta_port")
    pf.add_argument("programa", type=int)
    pf.add_argument("nomes", help="músicas separadas por vírgula")
    pc = sub.add_parser("percussao")
    pc.add_argument("pasta_orig")
    pc.add_argument("pasta_port")
    pc.add_argument("nomes", help="músicas separadas por vírgula")
    ct = sub.add_parser("comparar_todas")
    ct.add_argument("pasta_orig")
    ct.add_argument("pasta_port")
    ct.add_argument("saida_json")
    k = sub.add_parser("comparar")
    k.add_argument("original")
    k.add_argument("port")
    k.add_argument("--midi")
    k.add_argument("--png")
    k.add_argument("--desloc-max", type=float, default=2.0)
    a = ap.parse_args()
    if a.cmd == "capturar":
        capturar(a.saida, a.segundos, a.autotype, a.oplemu, a.video, a.som_inicio)
    elif a.cmd == "musicas":
        nomes = a.nomes or sorted(x[:-4].lower() for x in os.listdir(os.path.join(raiz_principal(), "SimAnt-T", "MAXIS", "ANTWIN", "SOUND"))
                                   if x.upper().endswith(".MID") and x.upper() != "GAMETHME.MID")
        capturar_musicas(a.pasta, nomes, a.oplemu)
    elif a.cmd == "comparar_todas":
        comparar_todas(a.pasta_orig, a.pasta_port, os.path.join(raiz_principal(), "SimAnt-T", "MAXIS", "ANTWIN", "SOUND"), a.saida_json)
    elif a.cmd == "perfil":
        perfil_programa(a.pasta_orig, a.pasta_port, a.programa, a.nomes.split(","))
    elif a.cmd == "percussao":
        percussao_notas(a.pasta_orig, a.pasta_port, a.nomes.split(","))
    elif a.cmd == "quadros":
        quadros_chave(a.avi, a.pasta, a.cada)
    elif a.cmd == "midi":
        cs = canais(a.canais)
        for t, ch, nota, vel, dur, prog in ler_midi(a.arq):
            if ch in cs and t <= a.ate:
                print(f"{t:8.3f} s  canal {ch:2d}  programa {prog:3d}  nota {nota:3d}  vel {vel:3d}  dur {dur:.3f}")
    elif a.cmd == "niveis":
        niveis(a.arq, a.janela)
    elif a.cmd == "harmonicos":
        harmonicos(a.midi, a.original, a.desloc, a.port, a.canal)
    else:
        comparar(a.original, a.port, a.midi, a.png, a.desloc_max)


if __name__ == "__main__":
    main()
