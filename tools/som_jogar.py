"""Sessão interativa do SimAnt original no DOSBox-X com o áudio gravado em WAV, para comparar os
efeitos sonoros dentro do jogo com os do port (docs/spec/regras/som.md).

Diferente de `som_dosbox.py capturar` (sem input, minimizado e mudo), aqui a janela abre normal, com
som, e o PRÓPRIO USUÁRIO joga: o DX-CAPTURE /A grava toda a sessão do Windows, do início até o jogo
ser fechado. Usa a mesma cópia do jogo (re_trabalho/som/C, fora do git); SimAnt-T não é alterado.

Uso:
  python -I tools/som_jogar.py SAIDA.wav [--mudo]
"""
import argparse
import os
import shutil
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import som_dosbox as sd  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("saida")
    ap.add_argument("--mudo", action="store_true", help="não tocar o som no PC (a gravação continua)")
    a = ap.parse_args()

    copia = sd.preparar_copia(os.path.join(sd.raiz_principal(), "SimAnt-T"))
    sd.som_de_inicio(copia, None)
    cap = os.path.join(sd.TRABALHO, "cap")
    for x in os.listdir(cap):
        os.remove(os.path.join(cap, x))
    conf_txt = sd.CONF.format(trab=sd.TRABALHO, autotype="", oplemu="default", video="")
    conf_txt = conf_txt.replace("nosound = true", f"nosound = {'true' if a.mudo else 'false'}")
    # o mouse do Windows 3.1 precisa ser capturado pela janela (clique nela para prender; Ctrl+F10 solta)
    conf_txt = conf_txt.replace("autolock = false", "autolock = true")
    conf = os.path.join(sd.TRABALHO, "jogar.conf")
    with open(conf, "w", encoding="ascii") as f:
        f.write(conf_txt)
    exe = os.path.join(sd.raiz_principal(), "referencia", "dosbox-x", "bin", "x64", "Release", "dosbox-x.exe")
    p = subprocess.Popen([exe, "-conf", conf, "-nopromptfolder", "-fastlaunch"], cwd=sd.TRABALHO)
    p.wait()
    wavs = sorted((os.path.join(cap, x) for x in os.listdir(cap) if x.lower().endswith(".wav")), key=os.path.getmtime)
    if not wavs:
        sys.exit("nenhum WAV gravado (ver re_trabalho/som/dosbox.log)")
    sd.corrigir_wav(wavs[-1])
    shutil.copyfile(wavs[-1], a.saida)
    print(f"{a.saida}: {(os.path.getsize(a.saida) - 44) / 176400:.1f} s gravados")


if __name__ == "__main__":
    main()
