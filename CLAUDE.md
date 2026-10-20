# Diretriz do projeto: port moderno de jogo Windows 3.x

## Objetivo

Reimplementar, em linguagem moderna, a lógica de um jogo antigo do Windows 3.x, lendo os dados e assets originais do usuário. O resultado deve rodar em sistemas atuais e ser fácil de modificar por meio de mods.

Não é objetivo: decompilar o executável original por completo, nem redistribuir assets protegidos por direitos autorais. O usuário fornece os próprios arquivos do jogo.

## Princípios

1. **Especificação antes do código.** Antes de implementar um subsistema, documente o comportamento observado em `docs/spec/`. Regras, telas, formatos de arquivo e transições de estado devem estar escritos.
2. **O original é o oráculo.** Quando a documentação e o comportamento real divergirem, o jogo original vence. Registre a divergência em `docs/divergencias.md`, com o que foi observado e como.
3. **Nunca afirmar comportamento sem evidência.** Ao descrever como o jogo funciona, indique se é fato verificado (testado no original, ou visto em dump/código) ou hipótese. Hipóteses vão marcadas como `[hipótese]` até serem confirmadas.
4. **Dados fora do código.** Regras, textos, tabelas e scripts devem ser carregados de arquivos de dados ou scripts, não fixados no código. Isso é o que torna os mods viáveis.
5. **Incremental.** Prefira um fluxo fim a fim pequeno (ex.: carregar e exibir um mapa) antes de expandir para os demais subsistemas.
6. **Código simples e explícito.** Priorize legibilidade. Otimize só com medição.

## Arquitetura (proposta inicial, ajustar conforme o jogo)

- `engine/core`: loop principal, estados e temporizador. Sem dependência de janela ou áudio.
- `engine/formats`: parsers dos formatos originais (gráficos, mapas, sons, dados). Cada parser tem testes com arquivos reais.
- `engine/render`, `engine/audio`, `engine/input`: camadas de sistema, preferencialmente via SDL2 ou equivalente.
- `engine/rules`: lógica do jogo, sem chamadas diretas a renderização ou áudio.
- `mods/`: sistema de carregamento de pacotes que sobrescrevem dados ou scripts do original.
- `tests/`: testes unitários para parsers e regras; testes de comparação com capturas do original.
- `docs/spec/`: especificação dos subsistemas.
- `tools/`: utilitários de extração e inspeção de arquivos originais.

Linguagem e bibliotecas: definir no primeiro marco e registrar em `docs/decisoes.md` (formato de registro de decisões: data, decisão, motivo, alternativas descartadas).

## Engenharia reversa

- Trabalhe com os dados e o comportamento antes do código de máquina.
- Quando analisar binário (Ghidra, dumps hexadecimais), anote no próprio `docs/spec/` os endereços, nomes e estruturas identificados, com a fonte.
- Não copie código de máquina ou trechos decompilados literalmente para o projeto. Reescreva a lógica a partir da especificação.
- Mantenha um glossário de nomes internos em `docs/glossario.md`.

## Testes contra o original

- Use DOSBox ou uma VM com Windows 3.1 para executar o jogo original.
- Para cada regra reimplementada, escreva um caso de teste com entrada, estado esperado e resultado observado no original.
- Capturas de tela e logs de estado de referência ficam em `tests/referencias/`.

## Convenções de trabalho com o Claude

- Antes de começar uma tarefa grande, liste o plano em poucos passos e confirme com o usuário se houver decisão que ele precise tomar.
- Ao propor mudanças de formato ou arquitetura, explique o impacto nos mods.
- Ao terminar uma tarefa, diga o que foi verificado (testes executados, comparação com o original) e o que não foi.
- Não invente nomes de funções, endereços ou formatos. Se não souber, diga e proponha como descobrir.
- Responda ao usuário em português.
- Documentação pública (decisão do usuário, 2026-10-20): **inglês** nos caminhos principais (`README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `docs/**`), com cópia em **português** em `README.pt-BR.md`, `CHANGELOG.pt-BR.md` e `docs/pt-BR/<mesmo caminho>`; a primeira linha de cada arquivo liga à outra língua. Ao mudar um documento, atualize as duas versões. Mensagens de commit em inglês.
- Textos exibidos pelo port passam pelo frame de idiomas (`data/idiomas/`, ver `docs/i18n.md`): launcher em inglês e português; o jogo, por enquanto, só em inglês.

## Mods

- Um mod é um diretório com um arquivo de manifesto (`mod.json`: nome, versão, dependências) e os dados ou scripts que altera.
- Mods sobrescrevem dados por chave, nunca editando os arquivos originais.
- Scripts de mod rodam em sandbox e acessam a lógica apenas pela API pública documentada em `docs/api-mods.md`.
- Toda mudança na API de mods deve ser registrada, com nota de compatibilidade.

## Pendências iniciais

- [x] Nome e gênero do jogo: **SimAnt: The Electronic Ant Colony** (Maxis, 1991), versão Windows 3.x. Simulação/estratégia de colônia de formigas. Ver `docs/spec/README.md`.
- [x] Usuário tem código-fonte? **Não.** Há, porém, a tabela de símbolos `SIMANTW.SYM`, com 2.171 nomes de funções e variáveis (ver `docs/spec/executavel.md`).
- [x] Formatos de dados identificados: banco `.NDX/.DAT` com LZSS, bitmaps, `SIMANT.CFG` e save `.ANT` (ver `docs/spec/formatos/`). Ainda faltam: layouts de janela (tipos 0/1), tiles (tipo 9), menus, sons e fontes.
- [x] Linguagem escolhida: **Rust + SDL2**, mods em **Lua** (ver `docs/decisoes.md`).
- [x] Ambiente de referência: **DOSBox-X** com `referencia/dosbox-x.conf` e `referencia/rodar-original.bat` (DOSBox-X 2026.10.01 em `referencia/dosbox-x/`, testado).
