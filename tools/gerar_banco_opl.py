"""Gera data/opl_banco.json: banco de timbres FM (2 operadores, estilo OPL) próprio do port, para os
128 programas General MIDI e as notas de percussão 35-81.

Os timbres foram desenhados para este projeto (não são cópia de nenhum banco existente): um modelo
por família GM (8 programas), com variações por programa, e ajustes nos programas que as músicas do
SimAnt usam, conferidos contra a gravação do original no DOSBox-X (tools/som_dosbox.py comparar).
Licença: a do projeto. Ver docs/decisoes.md ("Caminho FM").

Uso: python -I tools/gerar_banco_opl.py [data/opl_banco.json]
"""
import json
import os
import sys

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def op(mult=1, tl=0, ar=15, dr=0, sl=0, rr=7, sustenta=True, ksl=0, ksr=False, onda=0, am=False, vib=False):
    d = dict(mult=mult, tl=tl, ar=ar, dr=dr, sl=sl, rr=rr)
    if sustenta:
        d["sustenta"] = True
    if ksl:
        d["ksl"] = ksl
    if ksr:
        d["ksr"] = True
    if onda:
        d["onda"] = onda
    if am:
        d["am"] = True
    if vib:
        d["vib"] = True
    return d


def t(nome, mod, car, fb=0, aditivo=False, transp=0, nota=None):
    d = dict(nome=nome, modulador=mod, portadora=car)
    if fb:
        d["realimentacao"] = fb
    if aditivo:
        d["aditivo"] = True
    if transp:
        d["transposicao"] = transp
    if nota is not None:
        d["nota_fixa"] = nota
    return d


NOMES = """Acoustic Grand Piano|Bright Acoustic Piano|Electric Grand Piano|Honky-tonk Piano|Electric Piano 1|Electric Piano 2|Harpsichord|Clavinet|Celesta|Glockenspiel|Music Box|Vibraphone|Marimba|Xylophone|Tubular Bells|Dulcimer|Drawbar Organ|Percussive Organ|Rock Organ|Church Organ|Reed Organ|Accordion|Harmonica|Tango Accordion|Acoustic Guitar (nylon)|Acoustic Guitar (steel)|Electric Guitar (jazz)|Electric Guitar (clean)|Electric Guitar (muted)|Overdriven Guitar|Distortion Guitar|Guitar Harmonics|Acoustic Bass|Electric Bass (finger)|Electric Bass (pick)|Fretless Bass|Slap Bass 1|Slap Bass 2|Synth Bass 1|Synth Bass 2|Violin|Viola|Cello|Contrabass|Tremolo Strings|Pizzicato Strings|Orchestral Harp|Timpani|String Ensemble 1|String Ensemble 2|Synth Strings 1|Synth Strings 2|Choir Aahs|Voice Oohs|Synth Voice|Orchestra Hit|Trumpet|Trombone|Tuba|Muted Trumpet|French Horn|Brass Section|Synth Brass 1|Synth Brass 2|Soprano Sax|Alto Sax|Tenor Sax|Baritone Sax|Oboe|English Horn|Bassoon|Clarinet|Piccolo|Flute|Recorder|Pan Flute|Blown Bottle|Shakuhachi|Whistle|Ocarina|Lead 1 (square)|Lead 2 (sawtooth)|Lead 3 (calliope)|Lead 4 (chiff)|Lead 5 (charang)|Lead 6 (voice)|Lead 7 (fifths)|Lead 8 (bass + lead)|Pad 1 (new age)|Pad 2 (warm)|Pad 3 (polysynth)|Pad 4 (choir)|Pad 5 (bowed)|Pad 6 (metallic)|Pad 7 (halo)|Pad 8 (sweep)|FX 1 (rain)|FX 2 (soundtrack)|FX 3 (crystal)|FX 4 (atmosphere)|FX 5 (brightness)|FX 6 (goblins)|FX 7 (echoes)|FX 8 (sci-fi)|Sitar|Banjo|Shamisen|Koto|Kalimba|Bag pipe|Fiddle|Shanai|Tinkle Bell|Agogo|Steel Drums|Woodblock|Taiko Drum|Melodic Tom|Synth Drum|Reverse Cymbal|Guitar Fret Noise|Breath Noise|Seashore|Bird Tweet|Telephone Ring|Helicopter|Applause|Gunshot""".split("|")


def melodico(p):
    n = NOMES[p]
    f, k = divmod(p, 8)
    if f == 0:  # pianos: ataque seco, decaimento longo
        brilho = [24, 20, 22, 16, 30, 32, 18, 16][k]
        mm = [1, 1, 1, 1, 1, 14, 4, 3][k]
        onda_c = [0, 0, 0, 0, 0, 0, 1, 1][k]
        # Honky-tonk (3), o piano das músicas do SimAnt: no original o ataque tem os 5 primeiros
        # harmônicos quase iguais (-0, -2, -7, -6, -10 dB) e os agudos caem mais rápido que a
        # fundamental (~8 dB/s), medido na nota 84 de gamethme. Modulador forte e com decaimento
        # mais rápido que o da portadora.
        if k == 3:
            return t(n, op(1, 10, 15, 4, 15, 5, ksr=True), op(1, 0, 15, 2, 15, 5, ksr=True), fb=6)
        return t(n, op(mm, brilho, 15, 4, 6, 6, ksl=1, ksr=True),
                 op(1, 0, 15, 3 if k < 6 else 4, 15, 6, ksl=0, ksr=True, onda=onda_c), fb=[5, 6, 5, 6, 3, 2, 6, 6][k])
    if f == 1:  # percussão cromática: sinos
        mm, mc = [(7, 1), (6, 2), (5, 4), (4, 1), (3, 1), (10, 3), (3, 1), (5, 1)][k]
        dc = [5, 4, 4, 2, 6, 6, 2, 4][k]
        return t(n, op(mm, 34, 15, dc + 1, 15, 6, sustenta=False, ksr=True), op(mc, 0, 15, dc, 15, 6, ksr=True),
                 fb=[0, 0, 2, 0, 0, 3, 1, 4][k])
    if f == 2:  # órgãos: aditivos, sustentados
        if k == 1:  # percussive organ
            return t(n, op(2, 6, 15, 0, 0, 7, onda=1), op(1, 0, 15, 4, 4, 7, onda=1), aditivo=True)
        if k == 3:  # church organ: fundamental + oitava, ondas de meia senoide
            return t(n, op(1, 8, 13, 0, 0, 6, onda=1, vib=True), op(2, 2, 13, 0, 0, 6, onda=2, vib=True), aditivo=True, fb=3)
        if k in (4, 5, 7):  # reed organ, accordion, tango
            return t(n, op(1, 22, 12, 0, 0, 7, vib=k != 4), op(1, 0, 12, 0, 0, 7, onda=0, vib=True), fb=6)
        if k == 6:  # harmonica
            return t(n, op(1, 26, 11, 0, 0, 7, vib=True), op(2, 2, 11, 0, 0, 7, onda=1, vib=True), fb=5)
        return t(n, op([1, 2, 1, 1, 1, 1, 1, 1][k], 4, 15, 0, 0, 7, onda=1), op(1, 0, 15, 0, 0, 7, onda=2), aditivo=True, fb=2)
    if f == 3:  # guitarras: dedilhadas
        if k in (5, 6):  # overdrive, distortion
            return t(n, op(1, 14, 15, 1, 2, 7, onda=1), op(1, 0, 15, 1, 3, 7, onda=1), fb=7)
        if k == 7:  # harmonics: só os harmônicos altos
            return t(n, op(4, 28, 15, 3, 15, 6), op(4, 0, 15, 3, 15, 6), fb=3)
        tl = [26, 18, 16, 22, 20][k]
        return t(n, op([1, 3, 1, 3, 1][k], tl, 15, 3, 15, 6, ksl=1), op(1, 0, 15, 2 if k != 4 else 5, 15, 6), fb=[5, 6, 4, 5, 6][k])
    if f == 4:  # baixos: fundamental forte e brilho do modulador
        if k in (6, 7):  # synth bass
            return t(n, op(1, 16, 15, 3, 4, 7, onda=1), op(1, 0, 15, 1, 2, 7), fb=7)
        # Portadora no 2.º harmônico e modulador na fundamental: o 2.º harmônico domina, o 1.º e o 3.º
        # vêm 6-10 dB abaixo e o resto fica plano (perfil medido no baixo 34 do original, gamethme).
        # Ajuste por tools/ajustar_timbre_fm.py contra o perfil medido (erro médio 1,3 dB no modelo estático).
        tl = [18, 14, 12, 22, 10, 10][k]
        return t(n, op(1, tl, 15, 2, 4, 7), op([1, 2, 2, 1, 2, 2][k], 0, 15, 1, 3, 7),
                 fb=[5, 5, 5, 4, 6, 6][k])
    if f == 5:  # cordas solistas (40-43), tremolo, pizzicato, harpa, tímpano
        if k == 5:
            return t(n, op(1, 26, 15, 5, 15, 7), op(1, 0, 15, 6, 15, 7), fb=4)
        if k == 6:
            return t(n, op(2, 34, 15, 3, 15, 5), op(1, 0, 15, 3, 15, 5), fb=2)
        if k == 7:
            return t(n, op(0, 16, 15, 4, 15, 6), op(1, 0, 15, 3, 15, 6), fb=6)
        return t(n, op(1, 24, 9, 0, 0, 7, vib=True, am=k == 4), op(1, 0, 8, 0, 0, 7, vib=True), fb=6)
    if f == 6:  # conjuntos: cordas lentas, coros, orquestra
        if k in (0, 1, 2, 3):
            ar = [7, 5, 6, 4][k]
            return t(n, op(1, 26, ar + 1, 0, 0, 6, vib=True), op(1, 0, ar, 0, 0, 6, vib=True), fb=[6, 5, 7, 6][k])
        if k in (4, 5, 6):
            return t(n, op(1, 34, 7, 0, 0, 6, vib=True), op(1, 0, 7, 0, 0, 6, vib=True), fb=2)
        return t(n, op(1, 10, 15, 4, 15, 7), op(1, 0, 15, 4, 15, 7), fb=7)
    if f == 7:  # metais
        if k == 3:
            # Muted Trumpet (happy2, happy1, sad4): ~12 dB acima do original em relação à abertura
            # (happy2 sozinha: RMS -38,6 contra -32,0 dBFS da abertura no original; -22,2 contra
            # -25,8 no port). Portadora 16 passos de TL (12 dB) mais baixa.
            return t(n, op(2, 26, 10, 2, 2, 7, vib=True), op(1, 16, 12, 1, 1, 7, vib=True), fb=4)
        tl = [20, 22, 18, 26, 30, 18, 16, 20][k]
        mm = [1, 1, 0, 2, 1, 1, 1, 1][k]
        return t(n, op(mm, tl, 10, 2, 2, 7, vib=True), op(1, 0, 12, 1, 1, 7, vib=True), fb=[6, 6, 5, 4, 3, 6, 7, 6][k])
    if f == 8:  # palhetas: sax, oboé, corne inglês, fagote, clarinete
        if k < 4:  # saxes
            return t(n, op(1, 22, 12, 0, 0, 7, vib=True), op(1, 0, 13, 1, 1, 7, vib=True), fb=6, transp=[0, 0, 0, -12][k])
        mm, mc, tl = [(3, 1, 30), (3, 1, 28), (2, 1, 26), (2, 1, 22)][k - 4]
        return t(n, op(mm, tl, 12, 0, 0, 7, vib=True), op(mc, 0, 12, 0, 0, 7, vib=True), fb=[4, 4, 5, 5][k - 4])
    if f == 9:  # flautas
        return t(n, op(1, 40, 9, 0, 0, 7, vib=True), op(1, 0, 10, 0, 0, 7, vib=True), fb=[1, 2, 2, 4, 6, 5, 0, 1][k])
    if f == 10:  # solos sintéticos
        onda_c = [6, 0, 1, 0, 1, 0, 0, 0][k]
        return t(n, op(1, [63, 18, 28, 22, 14, 30, 24, 20][k], 15, 0, 0, 7), op([1, 1, 2, 1, 1, 1, 1, 1][k], 0, 15, 0, 0, 7, onda=onda_c, vib=True),
                 fb=[0, 7, 3, 5, 7, 3, 5, 6][k])
    if f == 11:  # pads
        if k == 7:  # sweep: o brilho do modulador sobe devagar
            return t(n, op(1, 18, 3, 0, 0, 5, vib=True), op(1, 0, 6, 0, 0, 5, vib=True), fb=6)
        return t(n, op([1, 1, 1, 1, 1, 3, 2, 1][k], [34, 30, 24, 34, 28, 30, 36, 18][k], 5, 0, 0, 5, vib=True),
                 op(1, 0, 4, 0, 0, 5, vib=True), fb=[2, 3, 6, 1, 5, 4, 2, 6][k])
    if f == 12:  # efeitos sintéticos
        return t(n, op([5, 1, 7, 2, 4, 1, 3, 9][k], 26, 6, 1, 6, 5, vib=True), op(1, 0, 6, 2, 6, 5, vib=True), fb=[4, 3, 2, 5, 3, 7, 4, 6][k])
    if f == 13:  # étnicos
        if k in (6, 7):  # fiddle, shanai
            return t(n, op(1 if k == 6 else 3, 24, 10, 0, 0, 7, vib=True), op(1, 0, 11, 0, 0, 7, vib=True), fb=6)
        if k == 5:  # bagpipe
            return t(n, op(1, 20, 14, 0, 0, 7), op(1, 0, 14, 0, 0, 7, onda=1), fb=6)
        return t(n, op([3, 1, 3, 1, 4, 1][k], 28, 15, 4, 15, 6), op(1, 0, 15, 3, 15, 6), fb=[5, 6, 5, 3, 0, 5][k])
    if f == 14:  # percussivos melódicos
        return t(n, op([4, 6, 3, 9, 0, 1, 0, 15][k], [30, 26, 24, 20, 10, 14, 6, 0][k], 15, 5, 15, 6, sustenta=False),
                 op(1, 0, 15, [3, 4, 3, 7, 5, 5, 5, 2][k], 15, 6, sustenta=False), fb=[0, 0, 3, 3, 7, 4, 7, 7][k])
    # efeitos sonoros: ruído (realimentação máxima com multiplicador alto)
    return t(n, op(15, 0, [15, 8, 4, 15, 12, 6, 4, 15][k], 2, 4, 5), op([1, 2, 1, 7, 4, 1, 1, 1][k], 0, [15, 10, 3, 15, 15, 8, 4, 15][k], 3, 6, 5),
             fb=7)


def bumbo(nome, nota):
    return t(nome, op(0, 10, 15, 6, 15, 7, sustenta=False), op(0, 0, 15, 5, 15, 7, sustenta=False), fb=4, nota=nota)


def tom(nome, nota):
    # Tom tonal com decaimento médio (no original o 43 soa em ~69 Hz, nota 37).
    # Nível: no original o 43 fica ~6 dB abaixo do baixo (janelas isoladas de gamethme).
    return t(nome, op(0, 22, 15, 5, 15, 6, sustenta=False), op(1, 8, 15, 4, 15, 6, sustenta=False), fb=3, nota=nota)


def ruido(nome, nota, decai, tl_car=0, mult_car=1):
    return t(nome, op(15, 0, 15, decai, 15, decai, sustenta=False), op(mult_car, tl_car, 15, decai, 15, decai, sustenta=False), fb=7, nota=nota)


def madeira(nome, nota, mm=3):
    return t(nome, op(mm, 30, 15, 8, 15, 8, sustenta=False), op(1, 0, 15, 7, 15, 7, sustenta=False), fb=2, nota=nota)


def percussao():
    p = {
        35: bumbo("Acoustic Bass Drum", 24),
        36: bumbo("Bass Drum 1", 26),
        37: madeira("Side Stick", 72, 5),
        38: ruido("Acoustic Snare", 60, 6),
        39: ruido("Hand Clap", 66, 7),
        40: ruido("Electric Snare", 62, 7),
        41: tom("Low Floor Tom", 35),
        42: ruido("Closed Hi-Hat", 96, 9, 2),
        43: tom("High Floor Tom", 37),
        44: ruido("Pedal Hi-Hat", 96, 8, 4),
        45: tom("Low Tom", 41),
        46: ruido("Open Hi-Hat", 96, 5, 2),
        47: tom("Low-Mid Tom", 44),
        48: tom("Hi-Mid Tom", 47),
        49: ruido("Crash Cymbal 1", 90, 3, 2, 3),
        50: tom("High Tom", 50),
        51: ruido("Ride Cymbal 1", 92, 4, 4, 5),
        52: ruido("Chinese Cymbal", 88, 3, 2, 3),
        53: ruido("Ride Bell", 94, 4, 6, 7),
        54: ruido("Tambourine", 92, 6, 4),
        55: ruido("Splash Cymbal", 92, 4, 2, 3),
        56: madeira("Cowbell", 79, 4),
        57: ruido("Crash Cymbal 2", 88, 3, 2, 3),
        58: ruido("Vibraslap", 70, 4, 6, 2),
        59: ruido("Ride Cymbal 2", 90, 4, 4, 5),
        60: madeira("Hi Bongo", 74, 1),
        61: madeira("Low Bongo", 67, 1),
        62: madeira("Mute Hi Conga", 70, 1),
        63: madeira("Open Hi Conga", 67, 1),
        64: madeira("Low Conga", 60, 1),
        65: madeira("High Timbale", 70, 2),
        66: madeira("Low Timbale", 62, 2),
        67: madeira("High Agogo", 86, 4),
        68: madeira("Low Agogo", 79, 4),
        69: ruido("Cabasa", 98, 8, 6),
        70: ruido("Maracas", 98, 9, 6),
        71: t("Short Whistle", op(1, 40, 13, 0, 0, 9), op(1, 0, 13, 0, 0, 9, vib=True), nota=90),
        72: t("Long Whistle", op(1, 40, 11, 0, 0, 7), op(1, 0, 11, 0, 0, 7, vib=True), nota=90),
        73: ruido("Short Guiro", 80, 8, 8, 2),
        74: ruido("Long Guiro", 80, 6, 8, 2),
        75: madeira("Claves", 84, 6),
        76: madeira("Hi Wood Block", 77, 3),
        77: madeira("Low Wood Block", 72, 3),
        78: t("Mute Cuica", op(2, 20, 15, 6, 15, 7, sustenta=False), op(1, 0, 15, 6, 15, 7, sustenta=False), fb=4, nota=74),
        79: t("Open Cuica", op(2, 24, 15, 4, 15, 6, sustenta=False), op(1, 0, 15, 4, 15, 6, sustenta=False), fb=3, nota=67),
        80: t("Mute Triangle", op(5, 30, 15, 7, 15, 7, sustenta=False), op(3, 0, 15, 7, 15, 7, sustenta=False), nota=96),
        81: t("Open Triangle", op(5, 30, 15, 3, 15, 4, sustenta=False), op(3, 0, 15, 3, 15, 4, sustenta=False), nota=96),
    }
    return {str(k): v for k, v in sorted(p.items())}


def main():
    saida = sys.argv[1] if len(sys.argv) > 1 else os.path.join(RAIZ, "data", "opl_banco.json")
    banco = {
        "_comentario": "Banco FM próprio do port (2 operadores por timbre, campos dos registradores do OPL). "
                       "Gerado por tools/gerar_banco_opl.py; ver docs/spec/formatos/som.md, 'Sintetizador FM'. "
                       "melodicos: 128 programas General MIDI; percussao: nota do canal 10 -> timbre com nota_fixa "
                       "(nota ausente = silêncio). curva_volume_db: atenuação = -curva x log10(vel x volume x expressão). "
                       "ganho: 1 = um canal no máximo dá 1/8 da escala, como o chip.",
        "tremolo_profundo": True,
        "vibrato_profundo": True,
        "curva_volume_db": 40.0,
        "ganho": GANHO,
        "melodicos": [melodico(p) for p in range(128)],
        "percussao": percussao(),
    }
    linhas = json.dumps(banco, ensure_ascii=False, indent=1)
    with open(saida, "w", encoding="utf-8", newline="\n") as f:
        f.write(linhas + "\n")
    print(f"{saida}: {len(banco['melodicos'])} melódicos, {len(banco['percussao'])} de percussão")


# Calibrado para que a abertura (gamethme) tenha, em relação aos efeitos, o mesmo nível que no
# original (RMS da música -32 dBFS com o efeito a ganho 0,25 => -20 dB abaixo de um efeito de
# ganho 1); ver docs/spec/regras/som.md.
GANHO = 2.25

if __name__ == "__main__":
    main()
