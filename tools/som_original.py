"""Grava o áudio do SimAnt original na bancada QEMU (Sound Blaster 16 + OPL2 emulados, saída em WAV).

A cópia do WfW 3.11 da bancada já tem os drivers da SB16 (`sb16snd.drv` para wave, `sb16fm.drv` para o
MIDI em FM, `vsbpd.386`; `[sndblst.drv]` Port=220, Int=7, DmaChannel=1, HDmaChannel=5). O QEMU entra com
a placa nessa configuração e o OPL2 (`adlib`, portas 0x388 e 0x220), sem a porta paralela (que usa a IRQ 7):

    -audiodev wav,id=som,path=ARQ.wav -device sb16,audiodev=som,irq=7 -device adlib,audiodev=som -parallel none

A máquina muda, então os snapshots da bancada não servem: `boot` faz o boot completo num overlay próprio
e grava o snapshot "som" no Quick Game. O WAV registra o tempo da VM (nada sai com a VM parada).

    set SIMANT_RAIZ=...
    python tools/som_original.py boot   OVERLAY SAIDA.wav     abertura (gamethme) até o Quick Game; snapshot "som"
    python tools/som_original.py latido OVERLAY SAIDA.wav     Yard View com o cachorro sentado (_DogDir = 4)
    python tools/som_original.py luta   OVERLAY SAIDA.wav     luta da amarela (como cenarios_tick luta_vizinha)
    python tools/som_original.py analisar ARQ.wav            trechos com som (início, duração, RMS)

Resultado e comparação com o port (`--exportar-som`): docs/spec/regras/som.md, "Áudio do original".
"""
import math
import os
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import qemu_bancada as qb  # noqa: E402


def args_som(wav):
    """Sem o OPL2 por padrão: o `sb16fm.drv` é um driver de OPL3 e, sobre o `adlib` (OPL2) do QEMU, as
    escritas do 2º banco (0x38A/0x38B) caem no 1º e deixam notas presas (ruído contínuo, RMS ≈ 0,19, desde a
    carga do Windows). SIMANT_SOM_OPL=1 liga o `adlib` mesmo assim."""
    opl = " -device adlib,audiodev=som" if os.environ.get("SIMANT_SOM_OPL") == "1" else ""
    return f"-audiodev wav,id=som,path={wav} -device sb16,audiodev=som,irq=7 -parallel none" + opl


def ler(b, nome, tam=None):
    return int.from_bytes(b.mj.ler_variavel(nome, tam), "little")


def boot(overlay, wav):
    p = qb.caminho_raiz(overlay)
    if p.exists():
        p.unlink()
    b = qb.Bancada(overlay, 16, args_som(wav))
    try:
        t0 = time.time()
        for k in range(12):
            time.sleep(5)
            b.executar(f"capturar re_trabalho/som/boot_{k:02d}.png")
        b.log("abertura gravada")
        for k in range(20):
            g = None
            try:
                g = ler(b, "GameTime")
                time.sleep(3)
                if ler(b, "GameTime") > g:
                    break
            except Exception:
                pass
            b.executar("teclas spc")
            time.sleep(4)
            b.executar("clicar_em 345 228")
            time.sleep(6)
        else:
            raise RuntimeError("o Quick Game não começou")
        b.log(f"Quick Game depois de {time.time() - t0:.0f} s")
        time.sleep(10)
        b.executar("capturar re_trabalho/som/quick.png")
        b.executar("snapshot_salvar som")
    finally:
        b.fechar()


def latido(overlay, wav, segundos=40):
    b = qb.Bancada(overlay, 16, args_som(wav))
    try:
        b.executar("snapshot_carregar som")
        time.sleep(1)
        # referência: Home (_YellowCommandKey, som 15) três vezes, 2 s de intervalo, com a edição na frente
        for _ in range(3):
            b.executar("teclas home")
            time.sleep(2)
        b.executar("teclas ctrl+y")
        time.sleep(2)
        b.executar("pausar")
        for nome, v in (("DogDir", 4), ("DogTurnCnt", 30000), ("DogBarkDelay", 0)):
            b.executar(f"escrever_variavel {nome} {v}")
        b.executar("continuar")
        for k in range(segundos // 5):
            time.sleep(5)
            # o cachorro senta de novo se levantar
            b.executar("escrever_variavel DogDir 4")
            b.executar("escrever_variavel DogTurnCnt 30000")
            b.executar("ler_variavel _DogFrame re_trabalho/som/v.bin 2")
            b.executar("ler_variavel DogBarkDelay re_trabalho/som/v.bin 4")
            if k == 0:
                b.executar("capturar re_trabalho/som/latido.png")
    finally:
        b.fechar()


def chuva(overlay, wav, segundos=30):
    """Chuva pedida (_GlobalKey = 0xA8, como o cenário `chuva` de cenarios_tick): _DoWater toca o efeito
    41 (3,1 s, o mais longo e fácil de reconhecer)."""
    b = qb.Bancada(overlay, 16, args_som(wav))
    try:
        b.executar("snapshot_carregar som")
        time.sleep(1)
        b.executar("pausar")
        b.executar("escrever_variavel _GlobalKey 168 2")
        b.executar("continuar")
        for k in range(segundos // 5):
            time.sleep(5)
            b.executar("ler_variavel RainOn re_trabalho/som/v.bin")
            b.executar("ler_variavel WaterTable re_trabalho/som/v.bin")
    finally:
        b.fechar()


def luta(overlay, wav):
    os.environ["SIMANT_QEMU_ARGS"] = args_som(wav)
    import cenarios_tick as ct
    c = ct.Sessao(overlay, "som")
    try:
        ct.preparar_luta_vizinha(c)
        c.dep.retomar()
        for k in range(4):
            time.sleep(2)
            c.b.executar(f"capturar re_trabalho/som/luta_{k}.png")
        time.sleep(4)
        c.log(f"RAntsKilled {c.ler('RAntsKilled')}, MeHealth {c.ler('MeHealth')}")
    finally:
        c.fechar()


def analisar(arq, janela_ms=10, limiar=0.01, juntar_ms=60):
    """Lista os trechos com som: RMS (0..1) em janelas de `janela_ms`, juntando silêncios curtos."""
    ch, taxa, mono = ler_wav(arq)
    n = len(mono)
    return _analisar(arq, ch, taxa, mono, n, janela_ms, limiar, juntar_ms)


def ler_wav(arq):
    """(canais, taxa, amostras mono em -1..1). Aceita o WAV do QEMU com os tamanhos do cabeçalho em 0
    (o QEMU só os escreve ao fechar o dispositivo, o que o `quit` nem sempre faz) e os 8 bits do port."""
    bruto = Path(arq).read_bytes()
    if bruto[:4] != b"RIFF" or bruto[8:12] != b"WAVE":
        raise SystemExit(f"{arq}: não é WAV")
    k, ch, taxa, bits, dados = 12, 1, 0, 16, b""
    while k + 8 <= len(bruto):
        nome, tam = bruto[k:k + 4], struct.unpack("<I", bruto[k + 4:k + 8])[0]
        if nome == b"fmt ":
            _, ch, taxa = struct.unpack("<HHI", bruto[k + 8:k + 16])
            bits = struct.unpack("<H", bruto[k + 22:k + 24])[0]
        elif nome == b"data":
            dados = bruto[k + 8:] if tam == 0 else bruto[k + 8:k + 8 + tam]
            break
        k += 8 + tam + (tam & 1)
    if bits == 16:
        dados = dados[:len(dados) // (2 * ch) * 2 * ch]
        amostras = [v / 32768 for v in struct.unpack(f"<{len(dados) // 2}h", dados)]
    else:
        amostras = [(v - 128) / 128 for v in dados]
    mono = [sum(amostras[i:i + ch]) / ch for i in range(0, len(amostras), ch)]
    return ch, taxa, mono


def _analisar(arq, ch, taxa, mono, n, janela_ms, limiar, juntar_ms):
    passo = taxa * janela_ms // 1000
    rms = [math.sqrt(sum(v * v for v in mono[i:i + passo]) / max(1, len(mono[i:i + passo])))
           for i in range(0, len(mono), passo)]
    trechos, ini, fim_ = [], None, None
    for k, r in enumerate(rms):
        if r >= limiar:
            if ini is None or (k - fim_) * janela_ms > juntar_ms:
                if ini is not None:
                    trechos.append((ini, fim_))
                ini = k
            fim_ = k
    if ini is not None:
        trechos.append((ini, fim_))
    print(f"{arq}: {n / taxa:.2f} s, {taxa} Hz, {ch} canais")
    for a, b in trechos:
        seg = rms[a:b + 1]
        energia = math.sqrt(sum(r * r for r in seg) / len(seg))
        print(f"  {a * janela_ms / 1000:8.3f} s  dura {(b - a + 1) * janela_ms / 1000:6.3f} s  "
              f"RMS {energia:.3f}  pico RMS {max(seg):.3f}")
    return trechos


def procurar(original, efeitos, limiar=0.6):
    """Acha cada efeito do port (WAV de --exportar-som, 4096 Hz) na gravação do original: correlação
    normalizada depois de reamostrar a gravação para 4096 Hz. Para cada ocorrência: instante, correlação,
    ganho (amplitude do original / do port, por mínimos quadrados) e quanto do efeito tocou (a fração da
    energia esperada que aparece na janela do efeito; < 1 = cortado)."""
    import numpy as np
    _, taxa, mono = ler_wav(original)
    x = np.asarray(mono, dtype=float)
    t = np.arange(0, len(x) / taxa, 1 / 4096)
    y = np.interp(t, np.arange(len(x)) / taxa, x)
    y -= y.mean()
    achados = []
    for ef in efeitos:
        _, te, m = ler_wav(ef)
        e = np.asarray(m, dtype=float)
        e -= e.mean()
        n = len(e)
        if n < 16 or len(y) < n:
            continue
        m = 1 << int(np.ceil(np.log2(len(y) + n)))
        c = np.fft.irfft(np.fft.rfft(y, m) * np.conj(np.fft.rfft(e, m)), m)[:len(y) - n + 1]
        acum = np.concatenate(([0.0], np.cumsum(y * y)))
        en = np.sqrt(np.maximum(acum[n:] - acum[:-n], 0)) * np.sqrt((e * e).sum()) + 1e-9
        r = c / en
        k = 0
        while k < len(r):
            if r[k] >= limiar:
                j = k + int(np.argmax(r[k:k + n]))
                janela = y[j:j + n]
                ganho = float((janela * e).sum() / (e * e).sum())
                # fração tocada: correlação acumulada do efeito, quarto a quarto
                quartos = [float(np.dot(janela[a:a + n // 4], e[a:a + n // 4]) /
                                 (np.linalg.norm(janela[a:a + n // 4]) * np.linalg.norm(e[a:a + n // 4]) + 1e-12))
                           for a in range(0, n - n // 4 + 1, n // 4)][:4]
                achados.append((j / 4096, Path(ef).stem, float(r[j]), ganho, quartos))
                k = j + n
            else:
                k += 1
    achados.sort()
    for t0, nome, rr, g, q in achados:
        print(f"  {t0:8.3f} s  {nome:12s} correlação {rr:.3f}  ganho {g:.2f}  "
              f"por quarto {' '.join(f'{v:.2f}' for v in q)}")
    return achados


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    cmd = sys.argv[1]
    if cmd == "analisar":
        return analisar(Path(sys.argv[2]))
    if cmd == "procurar":
        return procurar(Path(sys.argv[2]), [Path(a) for a in sys.argv[3:]])
    (qb.RAIZ / "re_trabalho" / "som").mkdir(parents=True, exist_ok=True)
    {"boot": boot, "latido": latido, "chuva": chuva, "luta": luta}[cmd](sys.argv[2], sys.argv[3])


if __name__ == "__main__":
    main()
