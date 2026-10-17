# Divergências entre documentação e comportamento

Registro de casos em que algo documentado (manual, nomes, especificação anterior) diverge do que o original faz. O original vence.

| Data | Assunto | Esperado | Observado | Como foi observado |
|---|---|---|---|---|
| 2026-10-16 | Lançador `.ba1` da pasta do jogo | iniciar o SimAnt no DOSBox | aponta para `games\SimTower\` e o `dosbox.conf` do SimTower | leitura do arquivo |
| 2026-10-16 | Abertura (`data/telas.json`, marco 1) | tela própria 640×480: Maxis → título centralizado em fundo preto | o título é desenhado na área cliente da janela principal, abaixo da faixa de ferramentas, com borda; não há tela Maxis antes. **Corrigido** no port (janelas 34 + 3, 99,98% igual) | DOSBox-X, capturas `abertura_titulo_dosbox.png` e `titulo_800x600.png` |
| 2026-10-16 | Paleta (`data/paleta.json`, `bitmap.md`) | índice 7 = cinza claro, 8 = cinza escuro | 7 = cinza escuro (0x808080), 8 = cinza claro (0xC0C0C0), ordem padrão do Windows. **Corrigido** | comparação pixel a pixel com `tools/comparar_captura.py` |
| 2026-10-16 | `_InitPillar` (`geracao-mundo.md` §4.3) | duas tentativas, sem repetir se a célula não servir | sorteia de novo até colocar os dois bichos-de-conta. **Corrigido** na spec e no port | desmontagem de `1030:4c4e`–`4cd5` (capstone); o `QUICK2.ANT` não distingue (as duas primeiras tentativas serviram) |
| 2026-10-16 | Setas sem Ctrl na tela de jogo (`data/navegacao.json`) | — | no original, ↑↓←→ sem Ctrl movem o cursor do mouse 8 pixels (`_DoKeyDown`, `1000:0e8c`) e só Ctrl+setas rolam a janela de edição (aceleradores 0xFDA6–0xFDA9). **Extensão do port**: as setas sozinhas e PageUp/PageDown também rolam; um mod pode removê-las | pseudocódigo do Ghidra e tabela de aceleradores do `SIMANTW.EXE` |
| 2026-10-16 | Repetição da rolagem (`data/navegacao.json`, `repeticao`) | — | o original repete Ctrl+seta e o clique nas setas da barra num laço, na velocidade do redesenho; o port repete o clique a cada 50 ms depois de 300 ms e usa a repetição de teclas do sistema | pseudocódigo de `MAINWNDPROC` (`1000:2930`) |
| 2026-10-17 | Morte da formiga-leão por cerco (`inimigos.md` §1.3) | o laço continua no mesmo k (a formiga-leão deslocada roda no mesmo tick) | o laço passa a k + 1: a deslocada só roda no tick seguinte. **Corrigido** na spec; o port segue o código | desmontagem de `1030:4828`–`4ad4` (capstone: o ramo da morte salta para `inc di`) |
| 2026-10-17 | Quadro da formiga-leão `_LionListT` (`inimigos.md` §1.1) | 0–4 | 0–6: durante a refeição o quadro é sorteado `S1(4) + 3`, e o centro vai de 0x38 a 0x3E. **Corrigido** na spec | pseudocódigo de `_DoAntLions`; coerente com `_ExpKillAnts` (testa 0x38–0x3E) |
