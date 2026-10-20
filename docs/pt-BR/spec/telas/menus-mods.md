*Read in English: [menus-mods.md](../../../spec/telas/menus-mods.md)*

# Menu e painel dos scripts de mods (extensão do port)

**Não existe no original.** É a parte de tela da API de mods 0.17 (`docs/api-mods.md`, "Scripts Lua"): um script pode ter um menu aberto por uma tecla e um painel de texto. O mod de trapaças de exemplo (`exemplos/mods/trapacas/`) é o primeiro uso. O jogo base não tem código específico de nenhum mod.

## Comportamento

- **Tecla.** `definir_menu` registra a tecla do menu (F1–F24, ou uma combinação com Ctrl ou Alt). Na tela de jogo, a ordem das teclas é a do original (aceleradores de `partida.json` e `navegacao.json`, teclas de janela de `janelas.json`) e só depois as dos menus de mods; em seguida `_YellowCommandKey` e o anel dos códigos de trapaça do original (`regras/trapacas.md`). Uma tecla de mod igual a um atalho do jogo nunca abre o menu; o `simant` avisa no console ao abrir a partida.
- **Menu.** Abre centrado em x, logo abaixo da faixa, por cima de todas as janelas e avisos. Desenho de menu suspenso do Windows 3.1: fundo branco, contorno preto, barra de título azul com o título e a tecla, item realçado em azul-escuro com texto branco, separadores cinza. Se os itens não cabem na altura da área cliente, o menu quebra em colunas (com uma linha cinza entre elas), sem começar coluna com separador. Geometria, fontes e cores: `data/menus_mods.json`.
- **Entrada.** Com o menu aberto, ele recebe toda a entrada: o mouse realça o item sob o cursor, o clique escolhe (fora do menu fecha), setas sobem e descem pulando separadores (dando a volta), Enter escolhe o realçado, Esc fecha. A simulação fica parada enquanto o menu está aberto (como a fila de mensagens de um menu do Windows).
- **Escolha.** Fecha o menu e chama o gancho `ao_menu(indice, texto, estado)` do mod dono. As ações pedidas no gancho são executadas logo em seguida (`core/acoes_mods.rs`) e os eventos que elas geram (sons, músicas, quadros) são tratados como depois de um tick.
- **Painel.** `definir_painel` mostra até 16 linhas no canto escolhido em `menus_mods.json` (padrão: em cima, à direita, abaixo da faixa), com o nome do mod; um painel por mod, um embaixo do outro. Fica embaixo do menu e por cima das janelas.
- **Texto.** Os textos dos scripts (UTF-8) são convertidos para a codificação da fonte: Latin-1 nas fontes do Windows (FNT) e Mac Roman nas do Macintosh (FONT); letra sem glifo vira a letra sem acento.

## Implementação

`engine/src/core/menus_mods.rs` (geometria, entrada e desenho, sem janela), `simant/src/mods_ui.rs` (teclas, eventos do SDL e desenho na tela de jogo). Para testes sem janela: `simant --save ARQ --mods DIR --menu mod@F12 --exportar-tela-jogo tela.png` (`mod@F12#N` escolhe o item N antes de exportar).
