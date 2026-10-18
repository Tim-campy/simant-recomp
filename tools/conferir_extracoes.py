"""Roda os extratores de tabelas das regras e confere que reproduzem o conteúdo dos data/*.json.

Uso:
    python tools/conferir_extracoes.py SIMANTW.EXE

Cada tools/extrair_<parte>.py grava numa pasta temporária e o resultado é comparado com
data/<parte>.json depois de lidos como JSON (a formatação pode ter sido ajustada à mão; os valores não).
Sai com código 1 se algum arquivo diferir, mostrando os caminhos das chaves diferentes.
Útil depois de mexer num extrator ou num data/*.json gerado por ele (ver docs/api-mods.md, data/comum.json).
"""
import json
import pathlib
import subprocess
import sys
import tempfile

RAIZ = pathlib.Path(__file__).resolve().parent.parent

# Extrator -> arquivo de data/ que ele gera.
PARTES = ["comum", "amarela", "feromonios", "geracao", "inimigos", "ninho", "quintal", "superficie"]


def diferencas(a, b, caminho):
    """Caminhos onde `a` (data/) e `b` (extraído) diferem."""
    if isinstance(a, dict) and isinstance(b, dict):
        for k in a.keys() | b.keys():
            if k not in b:
                yield f"{caminho}/{k}: só em data/"
            elif k not in a:
                yield f"{caminho}/{k}: só no extraído"
            else:
                yield from diferencas(a[k], b[k], f"{caminho}/{k}")
    elif a != b:
        yield f"{caminho}: data/ = {json.dumps(a, ensure_ascii=False)[:80]}, extraído = {json.dumps(b, ensure_ascii=False)[:80]}"


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(2)
    exe = sys.argv[1]
    sys.stdout.reconfigure(encoding="utf-8")
    falhas = 0
    with tempfile.TemporaryDirectory() as tmp:
        for parte in PARTES:
            script = RAIZ / "tools" / f"extrair_{parte}.py"
            saida = pathlib.Path(tmp) / f"{parte}.json"
            r = subprocess.run([sys.executable, str(script), exe, "--json", str(saida)], capture_output=True, text=True)
            if r.returncode != 0:
                print(f"{parte}: o extrator falhou\n{r.stderr}")
                falhas += 1
                continue
            novo = json.loads(saida.read_text(encoding="utf-8"))
            atual = json.loads((RAIZ / "data" / f"{parte}.json").read_text(encoding="utf-8"))
            dif = list(diferencas(atual, novo, ""))
            if not dif:
                print(f"{parte}.json: igual")
            else:
                falhas += 1
                print(f"{parte}.json: DIFERE")
                for linha in dif[:40]:
                    print("   ", linha)
    sys.exit(1 if falhas else 0)


if __name__ == "__main__":
    main()
