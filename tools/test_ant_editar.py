"""Testes do editor de .ANT (tools/ant_editar.py) e das conferências de tools/caso_ant.py com o QUICK2.ANT.

Rodar da raiz do repositório:  python -m unittest discover -s tools -p "test_*.py"
"""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ant_editar import Save, aplicar_patch, montar_caso, REPO  # noqa: E402

QUICK2 = REPO / "tests" / "referencias" / "saves" / "QUICK2.ANT"
CASOS = REPO / "tests" / "referencias" / "casos"


def difs(a: bytes, b: bytes):
    return [i for i, (x, y) in enumerate(zip(a, b)) if x != y]


class TesteAntEditar(unittest.TestCase):
    def setUp(self):
        self.orig = QUICK2.read_bytes()
        self.s = Save(self.orig)

    def test_round_trip_identico(self):
        with tempfile.TemporaryDirectory() as t:
            self.s.gravar(Path(t) / "x.ANT")
            self.assertEqual((Path(t) / "x.ANT").read_bytes(), self.orig)
        self.assertEqual(bytes(aplicar_patch(Save(self.orig), []).dados), self.orig)

    def test_escalares_conhecidos(self):
        self.assertEqual(self.s.ler("GameTime"), 78)
        self.assertEqual(self.s.ler("_HealthB"), 96)
        self.assertEqual(self.s.ler("ListIndexB"), 5)
        self.assertEqual(self.s.elementos("CastePopB"), [2, 2, 0, 0, 0, 1])
        self.assertEqual(self.s.ler("CurYardPnt", 0, largura=2), 11)  # par (x, y) de palavras
        self.assertEqual(self.s.ler("CurYardPnt", 1, largura=2), 8)

    def test_escrever_muda_so_o_escalar(self):
        pos = self.s.blocos["HealthB"][0]
        self.s.escrever("HealthB", 50)
        self.assertEqual(difs(self.orig, self.s.dados), [pos])
        self.s.escrever("HealthB", -1)
        self.assertEqual(self.s.ler("HealthB"), 0xFFFF)
        self.assertEqual(self.s.ler("HealthB", sinal=True), -1)
        with self.assertRaises(ValueError):
            self.s.escrever("HealthB", 70000)

    def test_celula_coluna_major(self):
        pos = self.s.blocos["MapB"][0]
        # túnel do QUICK2: (32, 0) é diferente de terra maciça 0x2E
        self.assertNotEqual(self.s.celula("MapB", 32, 0), 0x2E)
        self.assertEqual(self.s.celula("MapB", 32, 0), self.orig[pos + 32 * 64 + 0])
        self.s.set_celula("MapB", 34, 20, 0)
        self.assertLessEqual(set(difs(self.orig, self.s.dados)), {pos + 34 * 64 + 20})
        with self.assertRaises(IndexError):
            self.s.celula("MapB", 64, 0)
        self.assertEqual(self.s.celula("MapA", 127, 63), self.orig[self.s.blocos["MapA"][0] + 127 * 64 + 63])

    def test_listas(self):
        b = self.s.formigas("B")
        self.assertEqual([f["T"] for f in b], [102, 110, 1, 1, 1])
        self.assertEqual(b[0], {"X": 34, "Y": 9, "M": 9, "T": 102, "S": 0})

        i = self.s.adicionar_formiga("B", 32, 3, 2, 0x12)
        self.assertEqual(i, 5)
        self.assertEqual(self.s.ler("ListIndexB"), 6)
        self.assertEqual(self.s.formiga("B", 5), {"X": 32, "Y": 3, "M": 2, "T": 0x12, "S": 0})
        self.assertEqual(self.s.formiga("B", 6), {"X": 0, "Y": 0, "M": 0, "T": 0, "S": 0})

        # inserir no início desloca as demais
        self.s.adicionar_formiga("B", 34, 9, 0, 0x92, indice=0)
        self.assertEqual([f["T"] for f in self.s.formigas("B")], [0x92, 102, 110, 1, 1, 1, 0x12])

        self.s.remover_formiga("B", 0)
        self.s.remover_formiga("B", 5)
        self.assertEqual(bytes(self.s.dados), self.orig)  # voltou ao original, incluindo as vagas zeradas

    def test_editar_e_zerar(self):
        self.s.editar_formiga("B", 2, T=7)
        pos_t = self.s.blocos["BlistT"][0]
        self.assertEqual(difs(self.orig, self.s.dados), [pos_t + 2])
        self.s.zerar_lista("A")
        self.assertEqual(self.s.ler("ListIndexA"), 0)
        self.assertEqual(self.s.bytes_bloco("AlistT"), bytes(1000))
        with self.assertRaises(IndexError):
            self.s.editar_formiga("A", 0, T=1)

    def test_patch_e_casos_versionados(self):
        s = aplicar_patch(Save(self.orig), [
            {"op": "escrever", "nome": "HealthB", "valor": "0x10"},
            {"op": "formiga_editar", "lista": "B", "onde": {"T": 1}, "T": 7, "todas": True},
            {"op": "formiga_remover", "lista": "A", "onde": {"T": 17}},
            {"op": "retangulo", "mapa": "MapB", "x0": 34, "y0": 10, "x1": 34, "y1": 12, "valor": 0},
        ])
        self.assertEqual(s.ler("HealthB"), 16)
        self.assertEqual([f["T"] for f in s.formigas("B")], [102, 110, 7, 7, 7])
        self.assertEqual([f["T"] for f in s.formigas("A")], [150])
        self.assertEqual([s.celula("MapB", 34, y) for y in (10, 11, 12)], [0, 0, 0])
        for caso in sorted(CASOS.glob("*.json")):
            with self.subTest(caso=caso.name):
                c = json.loads(caso.read_text(encoding="utf-8"))
                self.assertIn("id", c)
                self.assertEqual(len(montar_caso(caso).dados), 48386)

    def test_c02_muda_so_o_esperado(self):
        s = montar_caso(CASOS / "C-02.json")
        nomes = set()
        for i in difs(self.orig, s.dados):
            nomes.add(next(n for n, (p, w, k) in s.blocos.items() if p <= i < p + w * k))
        self.assertEqual(nomes, {"BpopT", "BlistT", "ListIndexA", "AlistX", "AlistY", "AlistT", "AlistM"})

    def test_conferir(self):
        import caso_ant
        caso = {"conferir": [
            {"nome": "GameTime", "op": "==", "valor": 78},
            {"formiga": "B", "indice": 0, "campo": "T", "mascara": 248, "op": "==", "valor": 96},
            {"formigas": "R", "onde": {"M": 8}, "campo": "T", "op": "todos==", "valor": 129},
            {"contar": "B", "onde": {"T": 1}, "op": "==", "valor": 3},
            {"nome": "HealthB", "op": "em", "valor": [1, 2]},
        ]}
        res = caso_ant.conferir(caso, self.s)
        self.assertEqual([r["ok"] for r in res], [True, True, True, True, False])


if __name__ == "__main__":
    unittest.main()
