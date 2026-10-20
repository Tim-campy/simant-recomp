#!/usr/bin/env python3
"""Check relative links and anchors in the repository's Markdown files.

For every .md file (except target/, .claude/, re_trabalho/, referencia/, vendor/):
  * every relative markdown link [text](path#anchor) must point to an existing file,
    and, when it points to a .md file with an anchor, the anchor must exist there
    (slugs generated the way GitHub does it);
  * every public document in English must start with a link to its Portuguese copy,
    and vice versa (see CLAUDE.md, "Convenções de trabalho com o Claude").

Usage: python tools/conferir_links_docs.py
Prints one line per problem; exit code 1 if any problem was found.
Standard library only.
"""

from __future__ import annotations

import os
import re
import sys
import unicodedata
from pathlib import Path
from urllib.parse import unquote

RAIZ = Path(__file__).resolve().parent.parent
IGNORADOS = {"target", ".claude", "re_trabalho", "referencia", "vendor", ".git", "node_modules"}

# Pairs of public documents (English, Portuguese) outside docs/.
PARES_FIXOS = {
    "README.md": "README.pt-BR.md",
    "CHANGELOG.md": "CHANGELOG.pt-BR.md",
    "CONTRIBUTING.md": "CONTRIBUTING.pt-BR.md",
    "README-user.md": "LEIAME-usuario.md",
    "tests/referencias/README.md": "tests/referencias/README.pt-BR.md",
}

RE_CERCA = re.compile(r"^\s{0,3}(`{3,}|~{3,})")
RE_TITULO = re.compile(r"^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$")
RE_ANCORA_HTML = re.compile(r"<a\s+[^>]*?(?:name|id)\s*=\s*[\"']([^\"']+)[\"']", re.I)
# [text](target) and ![alt](target); target may be <...> or have an optional "title".
RE_LINK = re.compile(r"!?\[(?:[^\[\]]|\[[^\[\]]*\])*\]\(\s*(<[^>]*>|[^)\s]*)(?:\s+\"[^\"]*\")?\s*\)")
RE_DEF = re.compile(r"^\s{0,3}\[[^\]]+\]:\s*(<[^>]*>|\S+)")
RE_CODIGO_INLINE = re.compile(r"(`+)(.+?)\1")


def arquivos_md() -> list[Path]:
    saida = []
    for pasta, subpastas, arquivos in os.walk(RAIZ):
        subpastas[:] = sorted(s for s in subpastas if s not in IGNORADOS)
        for a in sorted(arquivos):
            if a.lower().endswith(".md"):
                saida.append(Path(pasta) / a)
    return saida


def rel(p: Path) -> str:
    return p.relative_to(RAIZ).as_posix()


def linhas_fora_de_codigo(texto: str):
    """Yield (line number, line) skipping fenced code blocks."""
    cerca = None
    for n, linha in enumerate(texto.splitlines(), 1):
        m = RE_CERCA.match(linha)
        if cerca:
            if m and m.group(1)[0] == cerca[0] and len(m.group(1)) >= len(cerca):
                cerca = None
            continue
        if m:
            cerca = m.group(1)
            continue
        yield n, linha


def slug_base(titulo: str) -> str:
    t = RE_CODIGO_INLINE.sub(lambda m: m.group(2), titulo)
    t = re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", t)  # links -> text
    t = re.sub(r"<[^>]+>", "", t)  # HTML tags
    t = t.strip().lower()
    saida = []
    for c in t:
        cat = unicodedata.category(c)
        if c in "-_" or cat[0] in "LN" or cat == "Mn":
            saida.append(c)
        elif c == " ":
            saida.append("-")
        # everything else (punctuation, symbols) is dropped
    return "".join(saida)


_cache_ancoras: dict[Path, set[str]] = {}


def ancoras(arquivo: Path) -> set[str]:
    if arquivo in _cache_ancoras:
        return _cache_ancoras[arquivo]
    texto = arquivo.read_text(encoding="utf-8")
    vistos: dict[str, int] = {}
    res: set[str] = set()
    for _, linha in linhas_fora_de_codigo(texto):
        for m in RE_ANCORA_HTML.finditer(linha):
            res.add(m.group(1))
        m = RE_TITULO.match(linha)
        if not m:
            continue
        base = slug_base(m.group(2))
        if base in vistos:
            vistos[base] += 1
            res.add(f"{base}-{vistos[base]}")
        else:
            vistos[base] = 0
            res.add(base)
    _cache_ancoras[arquivo] = res
    return res


def links(texto: str):
    for n, linha in linhas_fora_de_codigo(texto):
        sem_codigo = RE_CODIGO_INLINE.sub(lambda m: " " * len(m.group(0)), linha)
        for m in RE_LINK.finditer(sem_codigo):
            yield n, m.group(1)
        m = RE_DEF.match(sem_codigo)
        if m:
            yield n, m.group(1)


def externo(alvo: str) -> bool:
    return bool(re.match(r"^[a-zA-Z][a-zA-Z0-9+.-]*:", alvo)) or alvo.startswith("//")


def conferir_links(arquivo: Path, problemas: list[str]) -> None:
    texto = arquivo.read_text(encoding="utf-8")
    for n, alvo in links(texto):
        if alvo.startswith("<") and alvo.endswith(">"):
            alvo = alvo[1:-1]
        if not alvo or externo(alvo):
            continue
        caminho, _, ancora = alvo.partition("#")
        caminho = unquote(caminho)
        ancora = unquote(ancora)
        if caminho.startswith("/"):
            destino = RAIZ / caminho.lstrip("/")
        elif caminho:
            destino = (arquivo.parent / caminho).resolve()
        else:
            destino = arquivo
        onde = f"{rel(arquivo)}:{n}"
        if not destino.exists():
            problemas.append(f"{onde}: broken link: {alvo}")
            continue
        if ancora and destino.is_file() and destino.suffix.lower() == ".md":
            if ancora not in ancoras(destino):
                problemas.append(f"{onde}: missing anchor: {alvo} (no heading '#{ancora}' in {rel(destino)})")


def pares(arquivos: list[Path]) -> list[tuple[str, str]]:
    res = dict(PARES_FIXOS)
    for a in arquivos:
        r = rel(a)
        if r.startswith("docs/") and not r.startswith("docs/pt-BR/"):
            res[r] = "docs/pt-BR/" + r[len("docs/"):]
        elif r.startswith("docs/pt-BR/"):
            en = "docs/" + r[len("docs/pt-BR/"):]
            res.setdefault(en, r)
    return sorted(res.items())


def primeiro_link(arquivo: Path) -> Path | None:
    texto = arquivo.read_text(encoding="utf-8")
    for linha in texto.splitlines():
        if linha.strip():
            m = RE_LINK.search(linha)
            if not m:
                return None
            alvo = m.group(1).strip("<>").partition("#")[0]
            return (arquivo.parent / unquote(alvo)).resolve()
    return None


def conferir_pares(arquivos: list[Path], problemas: list[str]) -> None:
    for en, pt in pares(arquivos):
        p_en, p_pt = RAIZ / en, RAIZ / pt
        if not p_en.exists():
            problemas.append(f"{pt}: no English counterpart ({en} does not exist)")
            continue
        if not p_pt.exists():
            problemas.append(f"{en}: no Portuguese copy ({pt} does not exist)")
            continue
        for origem, outro in ((p_en, p_pt), (p_pt, p_en)):
            alvo = primeiro_link(origem)
            if alvo is None or alvo != outro.resolve():
                problemas.append(f"{rel(origem)}:1: first line must link to {rel(outro)}")


def main() -> int:
    arquivos = arquivos_md()
    problemas: list[str] = []
    for a in arquivos:
        conferir_links(a, problemas)
    conferir_pares(arquivos, problemas)
    for p in problemas:
        print(p)
    if problemas:
        print(f"\n{len(problemas)} problem(s) in {len(arquivos)} Markdown files.")
        return 1
    print(f"OK: {len(arquivos)} Markdown files, no problems.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
