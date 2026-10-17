"""Casos de teste de regras contra o SimAnt original, na bancada QEMU (docs/bancada-qemu.md).

Um caso (tests/referencias/casos/*.json) descreve: o save de partida, o patch (tools/ant_editar.py), até
quando rodar e o que conferir no estado lido da memória. Este script monta o .ANT, injeta-o num disquete,
roda o original (carrega o save pelo menu File > Load, deixa a simulação andar até o GameTime alvo, pausa e
lê o estado) e confere.

Uso:
    python tools/caso_ant.py rodar CASO.json [--saida DIR] [--snapshot quick]
    python tools/caso_ant.py conferir CASO.json ESTADO.ANT
    python tools/caso_ant.py disquete SAIDA.img ARQ.ANT[=NOME.ANT] ...

Injeção (escolha documentada em docs/bancada-qemu.md): os .ANT vão num disquete FAT12 (cópia do disquete de
boot da bancada, com os arquivos acrescentados por tools/fat12.py). Depois do `snapshot_carregar`, o roteiro
troca a mídia do drive A: (`disquete`, QMP blockdev-change-medium) e o jogo abre A:\\NOME.ANT pelo diálogo de
Load. O disco C: e o snapshot não são tocados: cada execução usa uma cópia do overlay padrão em DIR.

Saída em DIR (padrão re_trabalho/casos/ID, relativo a SIMANT_RAIZ): entrada.ANT, a.img, ov.qcow2,
roteiro.txt, telas *.png, estado.ANT (o estado lido da memória no fim) e resultado.json.

Conferências ("conferir" no caso), avaliadas sobre o estado final:
    {"nome": "GameTime", "op": ">=", "valor": 96}          escalar (ou {"nome", "indice"[, "largura"]})
    {"formiga": "B", "indice": 2, "campo": "T", "op": "==", "valor": 50}
    {"formigas": "B", "onde": {"X": 34, "Y": 9, "M": 8}, "campo": "T", "op": "todos==", "valor": 2}
    {"contar": "B", "onde": {"T": 2}, "op": ">=", "valor": 3}
    {"celula": "MapA", "x": 14, "y": 58, "op": "==", "valor": 107}   célula (x, y) de um mapa
op: ==, !=, >=, <=, >, <, "em" (valor = lista), e para "formigas": "todos==" / "todos_em".
"mascara": N aplica valor & N antes de comparar (ex.: 248 = classe de T, sem os bits de direção).
"""
import json
import operator
import os
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import qemu_bancada as qb  # noqa: E402
from ant_editar import Save, montar_caso, REPO  # noqa: E402
from fat12 import Fat12  # noqa: E402

OPS = {"==": operator.eq, "!=": operator.ne, ">=": operator.ge, "<=": operator.le, ">": operator.gt,
       "<": operator.lt, "em": lambda a, b: a in b}


def montar_disquete(saida: Path, arquivos):
    """arquivos: {NOME.ANT (8.3): bytes}. Parte do disquete de boot da bancada (mesmos arquivos e setor de boot)."""
    d = Fat12(qb.DISQUETE.read_bytes())
    for nome, dados in arquivos.items():
        d.gravar(nome, dados)
    saida.parent.mkdir(parents=True, exist_ok=True)
    saida.write_bytes(bytes(d.img))


def passos_load(nome_dos, d):
    """Linhas de roteiro que carregam A:NOME pelo diálogo File > Load (observado em 2026-10-16; ver
    docs/bancada-qemu.md, "Carregar um save injetado")."""
    base, _, ext = nome_dos.upper().partition(".")
    if ext != "ANT" or not base.isalnum() or len(base) > 8:
        raise ValueError(f"arquivo_dos deve ser NOME.ANT (até 8 letras ou números): {nome_dos}")
    return [
        "teclas ctrl+l",  # File > Load
        "esperar 3",
        f"capturar {d}/t1_salvar_atual.png",
        "teclas n",  # "Do you want to save the current ant hill?" -> No
        "esperar 3",
        # diálogo "Load Ant Hill" (abre em C:\MAXIS\ANTWIN, foco no File Name). O `digitar` da bancada só tem
        # letras, números e espaço: ':' vai por qcode. Sem extensão, o diálogo completa .ANT.
        "digitar A", "teclas shift+semicolon", f"digitar {base}",
        "esperar 1",
        f"capturar {d}/t2_nome.png",
        "teclas ret",
        "esperar 5",
        f"capturar {d}/t3_carregado.png",  # o jogo carregado fica PAUSADO (GamePaused = 1)
    ]


def gerar_roteiro(caso, d, snapshot):
    r = caso.get("rodar", {})
    nome = caso.get("arquivo_dos", "CASO.ANT")
    linhas = [f"# gerado por tools/caso_ant.py para o caso {caso['id']}",
              f"snapshot_carregar {snapshot}", "esperar 2",
              f"disquete {d}/a.img", "esperar 1"]
    linhas += passos_load(nome, d)
    linhas += ["pausar", f"estado {d}/carregado.ANT", "continuar",
               "teclas shift+0",  # Speed > Unpause
               "esperar 1"]
    if "ate_gametime" in r:
        linhas.append(f"esperar_variavel GameTime {r['ate_gametime']} {r.get('timeout', 120)}")
    if "segundos" in r:
        linhas.append(f"esperar {r['segundos']}")
    linhas += ["pausar", f"estado {d}/estado.ANT", f"capturar {d}/t4_fim.png", "fim"]
    return "\n".join(linhas) + "\n"


def _valor_conf(s: Save, c):
    if "nome" in c:
        return s.ler(c["nome"], c.get("indice"), c.get("largura"), c.get("sinal", False))
    if "formiga" in c:
        return s.formiga(c["formiga"], c["indice"])[c["campo"]]
    if "formigas" in c:
        return [s.formiga(c["formigas"], i)[c["campo"]] for i in s.procurar(c["formigas"], c.get("onde", {}))]
    if "contar" in c:
        return len(s.procurar(c["contar"], c.get("onde", {})))
    if "celula" in c:
        return s.celula(c["celula"], c["x"], c["y"])
    raise ValueError(f"conferência sem alvo: {c}")


def conferir(caso, estado: Save):
    resultados = []
    for c in caso.get("conferir", []):
        v = _valor_conf(estado, c)
        if "mascara" in c:
            m = int(c["mascara"], 0) if isinstance(c["mascara"], str) else c["mascara"]
            v = [x & m for x in v] if isinstance(v, list) else v & m
        op, esperado = c.get("op", "=="), c["valor"]
        if op == "todos==":
            ok = bool(v) and all(x == esperado for x in v)
        elif op == "todos_em":
            ok = bool(v) and all(x in esperado for x in v)
        else:
            ok = OPS[op](v, esperado)
        resultados.append({"conferencia": c, "obtido": v, "ok": ok})
        print(f"{'OK   ' if ok else 'FALHA'} {json.dumps(c, ensure_ascii=False)} -> {v}")
    return resultados


def rodar(caminho_caso, saida=None, snapshot="quick"):
    caso = json.loads(Path(caminho_caso).read_text(encoding="utf-8"))
    d_rel = saida or f"re_trabalho/casos/{caso['id']}"
    d = qb.caminho_raiz(d_rel)
    d.mkdir(parents=True, exist_ok=True)
    s = montar_caso(caminho_caso)
    s.gravar(d / "entrada.ANT")
    montar_disquete(d / "a.img", {caso.get("arquivo_dos", "CASO.ANT"): bytes(s.dados)})
    # cópia do overlay padrão (com o snapshot): a execução não suja o overlay de ninguém
    shutil.copyfile(qb.OVERLAY_PADRAO, d / "ov.qcow2")
    roteiro = d / "roteiro.txt"
    roteiro.write_text(gerar_roteiro(caso, d_rel, snapshot), encoding="utf-8")
    qb.roteiro(roteiro, overlay=d / "ov.qcow2")
    estado = Save.abrir(d / "estado.ANT")
    carregado = Save.abrir(d / "carregado.ANT")
    print(f"GameTime: entrada {s.ler('GameTime')}, logo após o load {carregado.ler('GameTime')}, "
          f"fim {estado.ler('GameTime')}")
    res = conferir(caso, estado)
    (d / "resultado.json").write_text(json.dumps({
        "caso": caso["id"], "gametime_entrada": s.ler("GameTime"),
        "gametime_carregado": carregado.ler("GameTime"), "gametime_fim": estado.ler("GameTime"),
        "conferencias": res}, ensure_ascii=False, indent=1), encoding="utf-8")
    return all(r["ok"] for r in res)


def main(args):
    if not args:
        sys.exit(__doc__)
    if args[0] == "rodar":
        saida = qb.opcao(args, "--saida")
        snap = qb.opcao(args, "--snapshot", "quick")
        sys.exit(0 if rodar(args[1], saida, snap) else 1)
    elif args[0] == "conferir":
        caso = json.loads(Path(args[1]).read_text(encoding="utf-8"))
        res = conferir(caso, Save.abrir(args[2]))
        sys.exit(0 if all(r["ok"] for r in res) else 1)
    elif args[0] == "disquete":
        arqs = {}
        for a in args[2:]:
            origem, _, nome = a.partition("=")
            arqs[(nome or Path(origem).name).upper()] = Path(origem).read_bytes()
        montar_disquete(Path(args[1]), arqs)
        print(f"{args[1]}: {', '.join(arqs)}")
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv[1:])
