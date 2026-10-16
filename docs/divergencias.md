# Divergências entre documentação e comportamento

Registro de casos em que algo documentado (manual, nomes, especificação anterior) diverge do que o original faz. O original vence.

| Data | Assunto | Esperado | Observado | Como foi observado |
|---|---|---|---|---|
| 2026-10-16 | Lançador `.ba1` da pasta do jogo | iniciar o SimAnt no DOSBox | aponta para `games\SimTower\` e o `dosbox.conf` do SimTower | leitura do arquivo |
| 2026-10-16 | Abertura (`data/telas.json`, marco 1) | tela própria 640×480: Maxis → título centralizado em fundo preto | o título é desenhado na área cliente da janela principal, abaixo da faixa de ferramentas, com borda; não há tela Maxis antes. **Corrigido** no port (janelas 34 + 3, 99,98% igual) | DOSBox-X, capturas `abertura_titulo_dosbox.png` e `titulo_800x600.png` |
| 2026-10-16 | Paleta (`data/paleta.json`, `bitmap.md`) | índice 7 = cinza claro, 8 = cinza escuro | 7 = cinza escuro (0x808080), 8 = cinza claro (0xC0C0C0), ordem padrão do Windows. **Corrigido** | comparação pixel a pixel com `tools/comparar_captura.py` |

