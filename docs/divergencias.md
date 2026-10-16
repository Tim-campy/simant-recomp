# Divergências entre documentação e comportamento

Registro de casos em que algo documentado (manual, nomes, especificação anterior) diverge do que o original faz. O original vence.

| Data | Assunto | Esperado | Observado | Como foi observado |
|---|---|---|---|---|
| 2026-10-16 | Lançador `.ba1` da pasta do jogo | iniciar o SimAnt no DOSBox | aponta para `games\SimTower\` e o `dosbox.conf` do SimTower | leitura do arquivo |
| 2026-10-16 | Abertura (`data/telas.json`, marco 1) | tela própria 640×480: Maxis → título centralizado em fundo preto | o título é desenhado dentro da janela principal do jogo, sob o menu e a barra de ferramentas, com moldura azul. O port ainda mostra a versão hipotética | DOSBox-X, captura `tests/referencias/capturas/abertura_titulo_dosbox.png` |

