"""Seção "sons" dos arquivos de regras (data/<parte>.json): ids dos efeitos sonoros (objetos tipo 5 do
banco SOUND) que cada parte pede com `_myBeginSound`. Não são tabelas do executável: vêm das chamadas
vistas no pseudocódigo (docs/spec/regras/som.md). Usado pelos scripts tools/extrair_*.py.
"""

SONS = {
    "amarela": {
        "_origem": "docs/spec/regras/som.md: _myBeginSound em _YellowCommand, _YellowCommandKey, _ExchangeLives, "
                   "_YellowDeath, _DoAntMoveY, _SetLife, _Pickup*/_Drop*, _EatMyFood, _AnimYellowFight, "
                   "_YellowBirth, _processSpider",
        "erro": 1, "ok": 15, "passagem": 1, "cavar": 19,
        "ovo": 28, "comida": 29, "pedrinha": 30, "comer": 44, "arroto": 10,
        "luta": 37, "choque": 49, "nascimento_ovo": 28, "nascimento": 49,
        "aranha_faro": 2, "aranha_laser": 41,
    },
    "quintal": {
        "_origem": "docs/spec/regras/som.md: _MakeBark/_SimDog (1030:2dbc, 32ea), _SimCat, _SimBird, _FootFall, "
                   "_SimKidInside (1030:20b6), _DoWater",
        "latidos": [23, 21, 23, 22], "latido_cortador": 21, "gato": 13, "passaro": 7,
        "pisada": 35, "inseticida": 9,
        "menino_casa": 24, "menino_casa_quando": [[3, 20, 900], [1, 21, 903], [6, 20, 6], [3, 21, 1]],
        "chuva": 41,
    },
    "superficie": {
        "_origem": "docs/spec/regras/som.md: _AddFood(n, 1) (1030:6a58)",
        "comida_nova": 32,
    },
    "inimigos": {
        "_origem": "docs/spec/regras/som.md: _DoAntLions (1030:43fa), _MoveSpider (1020:4704), _DoLaserFire (1000:6ec8)",
        "leao_cospe": 30, "aranha_corre": 47, "aranha_mastiga": 44, "aranha_arroto": 10, "aranha_laser": 55,
    },
    "ninho": {
        "_origem": "docs/spec/regras/som.md: _DoDigInB (1028:4bd0) / _DoDigInR (1028:6f0c)",
        "cavar_preta": 17, "cavar_vermelha": 18,
    },
}
