"""Gera o roteiro da bancada QEMU que visita e captura todos os cartões do Info alcançáveis por clique.

Lê a lista de registros impressa por `cargo run -p engine --example exportar_info -- JOGO SAIDA` (sem ids:
os 53 cartões), monta o grafo das áreas de clique (valor > 1 = ir para o cartão) e, a partir do Top Card
(128), visita cada cartão pelo caminho mais curto, capturando cada um uma vez. Cliques = canto da área do
cartão na tela + centro da área (docs/spec/telas/info.md; canto (281, 259) com a janela do PORT50).

    python tools/gerar_roteiro_info.py LISTA.txt SAIDA.txt [--pasta re_trabalho/info_todos]
"""
import argparse
import re
from collections import deque

CANTO = (281, 259)


def ler(lista):
    cartas, atual = {}, None
    for linha in open(lista, encoding="utf-8"):
        m = re.match(r"== (\d+) ", linha)
        if m:
            atual = int(m.group(1))
            cartas[atual] = []
            continue
        m = re.match(r"\s+PICT \d+ em .* áreas \[(.*)\]", linha)
        if m and atual is not None:
            for v, e, t, d, b in re.findall(r"\((-?\d+), (-?\d+), (-?\d+), (-?\d+), (-?\d+)\)", m.group(1)):
                v, e, t, d, b = map(int, (v, e, t, d, b))
                if v > 1:
                    cartas[atual].append((v, (CANTO[0] + (e + d) // 2, CANTO[1] + (t + b) // 2)))
    return cartas


def caminho(cartas, de, para):
    ant = {de: None}
    fila = deque([de])
    while fila:
        c = fila.popleft()
        if c == para:
            break
        for v, xy in cartas.get(c, []):
            if v not in ant:
                ant[v] = (c, xy)
                fila.append(v)
    if para not in ant:
        return None
    passos = []
    while ant[para]:
        c, xy = ant[para]
        passos.append((para, xy))
        para = c
    return passos[::-1]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("lista")
    ap.add_argument("saida")
    ap.add_argument("--pasta", default="re_trabalho/info_todos")
    ap.add_argument("--espera", type=int, default=2)
    a = ap.parse_args()
    cartas = ler(a.lista)
    linhas = [
        "# Gerado por tools/gerar_roteiro_info.py: visita e captura os cartões do Info alcançáveis por clique.",
        f"# Pré-requisitos como em qemu_info_cartas.txt ({a.pasta}/ov.qcow2 com o snapshot quick; a.img com PORT50).",
        f"# Uso: python tools/qemu_bancada.py roteiro ESTE --overlay {a.pasta}/ov.qcow2",
        "snapshot_carregar quick", "esperar 2", f"disquete {a.pasta}/a.img", "esperar 1", "teclas ctrl+l", "esperar 3",
        "teclas n", "esperar 3", "digitar A", "teclas shift+semicolon", "digitar PORT50", "esperar 1", "teclas ret",
        "esperar 5", "clicar_em 105 97", "esperar 3", f"capturar {a.pasta}/c128.png",
    ]
    atual, vistos, fora = 128, {128}, []
    for alvo in sorted(cartas):
        if alvo in vistos:
            continue
        p = caminho(cartas, atual, alvo)
        if p is None:
            fora.append(alvo)
            continue
        for carta, (x, y) in p:
            linhas += [f"clicar_em {x} {y}", f"esperar {a.espera}"]
            if carta not in vistos:
                vistos.add(carta)
                linhas.append(f"capturar {a.pasta}/c{carta}.png")
        atual = alvo
    linhas.append("fim")
    open(a.saida, "w", encoding="utf-8").write("\n".join(linhas) + "\n")
    print(f"{len(vistos)} cartões; inalcançáveis por clique a partir do 128: {fora}")


if __name__ == "__main__":
    main()
