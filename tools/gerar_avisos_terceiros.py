"""Gera o AVISOS-TERCEIROS.txt do pacote de lançamento: as licenças do que vai dentro do executável.

Uso: python tools/gerar_avisos_terceiros.py --alvo x86_64-pc-windows-msvc --saida pacote/AVISOS-TERCEIROS.txt
     [--sdl2 vendor/SDL2-2.30.9/COPYING.txt]   (só quando o SDL2.dll vai junto no pacote)

Só biblioteca padrão. Lista as crates ligadas ao executável `simant` no alvo pedido (`cargo tree -e
normal,no-proc-macro`: sem dependências de build, de teste e macros, que não entram no binário), com o
texto de licença que cada uma traz. Nas crates com licença dupla ("MIT OR Apache-2.0") o pacote usa a MIT.
Acrescenta o que não é crate mas entra no binário: o Lua 5.4 compilado pelo `mlua` (feature `vendored`), a
fonte font8x8 do launcher (domínio público), a biblioteca padrão do Rust e, com --sdl2, o SDL2.dll.
"""
import argparse
import json
import re
import subprocess
from pathlib import Path

MIT = """Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE."""

SEP = "=" * 78


def rodar(args, raiz):
    return subprocess.run(args, cwd=raiz, check=True, capture_output=True, text=True, encoding="utf-8").stdout


def crates_ligadas(raiz, alvo):
    saida = rodar(["cargo", "tree", "-p", "simant", "-e", "normal,no-proc-macro", "--prefix", "none",
                   "--target", alvo, "--format", "{p}"], raiz)
    vistos = set()
    for linha in saida.splitlines():
        m = re.match(r"(\S+) v(\S+)", linha.strip())
        if m:
            vistos.add((m.group(1), m.group(2)))
    return vistos


def escolher_arquivo(pasta, licenca):
    """Arquivo de licença da crate, preferindo o da MIT quando há escolha."""
    arquivos = [p for p in pasta.iterdir() if p.is_file() and re.match(r"(?i)(licen[cs]e|copying|copyright)", p.name)]
    if not arquivos:
        return None
    prefer = ["mit", "zlib", "unlicense", "bsd"] if "MIT" in licenca or "Zlib" in licenca else []
    for chave in prefer:
        for p in arquivos:
            if chave in p.name.lower():
                return p
    if len(arquivos) == 1:
        return arquivos[0]
    for p in arquivos:  # LICENSE simples (sem sufixo) costuma ser o principal
        if p.stem.upper() in ("LICENSE", "LICENCE", "COPYING"):
            return p
    return sorted(arquivos)[0]


def aviso_lua(pacotes):
    lua = [p for p in pacotes if p["name"] == "lua-src"]
    if not lua:
        return None
    pasta = Path(lua[0]["manifest_path"]).parent
    for cab in sorted(pasta.glob("lua-5.4*/lua.h")):
        texto = cab.read_text(encoding="latin-1")
        m = re.search(r"/\*{10,}\s*\n(.*Copyright.*?)\*{10,}/", texto, re.S)
        if m:
            corpo = "\n".join(l.lstrip("* ").rstrip() for l in m.group(1).splitlines())
            return f"Lua {cab.parent.name.split('-')[1]} (compilado dentro do executável pelo mlua, feature vendored)\nLicença: MIT\n\n{corpo.strip()}"
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--alvo", required=True)
    ap.add_argument("--saida", required=True)
    ap.add_argument("--sdl2")
    a = ap.parse_args()
    raiz = Path(__file__).resolve().parent.parent

    meta = json.loads(rodar(["cargo", "metadata", "--format-version", "1"], raiz))
    pacotes = meta["packages"]
    por_nome = {(p["name"], p["version"]): p for p in pacotes}
    ligadas = sorted(c for c in crates_ligadas(raiz, a.alvo) if c[0] not in ("simant", "engine"))

    partes = ["Avisos de terceiros do SimAnt recomp",
              "",
              "O executável deste pacote inclui o software abaixo. Cada item traz a licença sob a qual é",
              "distribuído; nas crates com licença dupla (\"MIT OR Apache-2.0\" e semelhantes) usa-se a MIT.",
              f"Gerado por tools/gerar_avisos_terceiros.py para o alvo {a.alvo}.",
              ""]
    resumo = []
    corpos = []
    for nome, versao in ligadas:
        p = por_nome[(nome, versao)]
        lic = p.get("license") or "?"
        resumo.append(f"  {nome} {versao} — {lic}")
        pasta = Path(p["manifest_path"]).parent
        arq = escolher_arquivo(pasta, lic)
        if arq:
            texto = arq.read_text(encoding="utf-8", errors="replace").strip()
        elif "MIT" in lic:
            autores = ", ".join(p.get("authors") or [nome + " authors"])
            texto = f"MIT License\n\nCopyright (c) {autores}\n\n{MIT}"
        else:
            texto = f"(a crate não traz arquivo de licença; licença declarada: {lic})"
        corpos.append(f"{SEP}\n{nome} {versao}\nLicença declarada: {lic}\n{p.get('repository') or ''}\n\n{texto}\n")

    extras = []
    lua = aviso_lua(pacotes)
    if lua:
        extras.append(lua)
        resumo.append("  Lua 5.4 (via mlua/lua-src) — MIT")
    extras.append("font8x8 (fonte 8 x 8 do launcher: font8x8_basic e font8x8_ext_latin, em simant/src/launcher/fonte8x8.rs)\n"
                  "Licença: domínio público\nhttps://github.com/dhepper/font8x8\n\n"
                  "Author: Daniel Hepper <daniel@hepper.net>. License: Public Domain.\n"
                  "Based on font8_8.asm by Marcel Sondaar, from the International Business Machines public\n"
                  "domain VGA fonts. License: Public Domain.")
    resumo.append("  font8x8 (fonte do launcher) — domínio público")
    extras.append("Biblioteca padrão do Rust (std, core, alloc), ligada estaticamente\n"
                  "Licença: MIT OR Apache-2.0 (usa-se a MIT)\n\nCopyright (c) The Rust Project Developers\n\n" + MIT)
    resumo.append("  Biblioteca padrão do Rust — MIT OR Apache-2.0")
    if a.sdl2:
        texto = Path(a.sdl2).read_text(encoding="utf-8", errors="replace").strip()
        extras.append("Simple DirectMedia Layer (SDL2.dll, 2.30.9, binário oficial de libsdl.org sem alterações)\n"
                      "Licença: Zlib\nhttps://www.libsdl.org/\n\n" + texto)
        resumo.append("  SDL2 2.30.9 (SDL2.dll) — Zlib")

    partes.append("Resumo:")
    partes += resumo
    partes.append("")
    for e in extras:
        partes.append(f"{SEP}\n{e}\n")
    partes += corpos
    Path(a.saida).parent.mkdir(parents=True, exist_ok=True)
    Path(a.saida).write_text("\n".join(partes), encoding="utf-8", newline="\n")
    print(f"{len(ligadas)} crates, {len(extras)} extras -> {a.saida}")


if __name__ == "__main__":
    main()
