*Read in English: [CONTRIBUTING.md](CONTRIBUTING.md)*

# Como contribuir com o SimAnt recomp

Obrigado pelo interesse. O SimAnt recomp reimplementa a lógica do SimAnt (Maxis, 1991, Windows 3.x) a
partir de uma especificação escrita e a confere contra o jogo original. Este documento explica o método
que o projeto segue; leia-o antes de abrir uma issue ou um pull request.

## Regras básicas

- **Nenhum arquivo do jogo original no repositório, nunca.** Nada de gráficos, sons, músicas, textos,
  `.DAT`/`.NDX`, `SIMANTW.EXE`, `SIMANT.HLP`, capturas das telas do original nem gravações WAV dele.
  Quem contribui usa a própria cópia do jogo (em `SimAnt-T/MAXIS/ANTWIN/`, ignorada pelo git, ou via
  `SIMANT_DIR`). Os textos do jogo também não são copiados para os `data/*.json`: os dados guardam só a
  referência (banco, executável, layout de janela, `.HLP`) e o port lê o texto da instalação do usuário.
  Não peça nem compartilhe cópias do jogo nas issues ou discussões.
- **Nada de código de máquina ou código decompilado copiado.** A engenharia reversa (Ghidra, dumps
  hexadecimais, a tabela de símbolos `SIMANTW.SYM`, a bancada QEMU) serve para *entender* o
  comportamento; o port é reescrito a partir da especificação, nunca colando código decompilado.
- **Respeite a licença.** As contribuições são licenciadas em Apache-2.0 e MIT, como o resto do projeto
  (ver [`LICENSE`](LICENSE)).

## O método

### 1. Especificação antes do código

Antes de implementar um subsistema, documente o comportamento observado em
[`docs/pt-BR/spec/`](docs/pt-BR/spec/README.md) (e na versão em inglês, `docs/spec/`): formatos de
arquivo, telas, regras e transições de estado. Ao analisar o binário, anote na spec os endereços, os
nomes originais (por exemplo `_DoAntSim`) e as estruturas identificadas, com a fonte. Nomes internos
novos vão para o glossário, [`docs/pt-BR/glossario.md`](docs/pt-BR/glossario.md).

### 2. O original é o oráculo

Quando a documentação (o manual, a ajuda, a spec) e o comportamento real do original divergirem, o
original vence. Registre a divergência em [`docs/pt-BR/divergencias.md`](docs/pt-BR/divergencias.md),
com o que foi observado e como. O port reproduz o comportamento do original, inclusive as
esquisitices; os afastamentos deliberados (extensões do port, escolhas do usuário) ficam registrados em
[`docs/pt-BR/decisoes.md`](docs/pt-BR/decisoes.md).

### 3. Nenhuma afirmação sem evidência

Ao descrever como o jogo funciona, diga se é fato verificado (testado no original, ou visto em dump ou
no código) ou hipótese. Hipóteses ficam marcadas como `[hipótese]` até serem confirmadas; dúvidas sobre
uma causa, como `[causa em aberto]`. Não invente nomes de funções, endereços ou formatos: se não souber,
diga e proponha como descobrir.

As evidências que o projeto usa:

- **Vetores de tick** (`tests/referencias/`, ver o [`README`](tests/referencias/README.pt-BR.md) de lá):
  o estado do original gravado antes e depois de cada tick pela
  [bancada QEMU](docs/pt-BR/bancada-qemu.md); o port tem de reproduzi-lo bit a bit
  (`engine/tests/ticks_originais.rs`, `ticks_longos.rs`).
- **Casos de regra:** um save injetado no original, rodado por alguns ticks, e o estado resultante.
- **Comparação pixel a pixel** das telas do port com capturas do original (as capturas ficam locais,
  fora do git).
- **Extratores** em `tools/` que leem as tabelas do original no executável e reproduzem os
  `data/*.json` (`tools/conferir_extracoes.py SIMANTW.EXE`).

### 4. Dados fora do código

Regras, textos, tabelas, teclas e layouts são carregados de arquivos de dados (`data/*.json`) ou
scripts, não fixados no código. É isso que torna os mods viáveis. Os textos exibidos pelo port
(launcher, mensagens do port) passam pelos arquivos de idioma, para poderem ser traduzidos.

### 5. Pequeno, incremental e simples

Prefira um fluxo fim a fim pequeno antes de expandir. Mantenha o código legível e explícito; otimize só
com medição.

## Mods

- Um mod é uma pasta com um manifesto `mod.json` (nome, versão, dependências) e os dados ou scripts que
  altera. Mods sobrescrevem dados por chave e nunca editam os arquivos originais.
- Os scripts de mod (Lua 5.4) rodam em sandbox e acessam o jogo apenas pela API pública documentada em
  [`docs/pt-BR/api-mods.md`](docs/pt-BR/api-mods.md).
- **Toda mudança na API de mods deve ser registrada** no histórico de compatibilidade do
  `docs/api-mods.md` (nas duas línguas), com nota de compatibilidade e a nova versão da API. Ao propor
  uma mudança de formato de dados ou de arquitetura, explique o impacto nos mods.

## Documentação em duas línguas

A documentação mostrada no GitHub é em **inglês**; cada documento tem uma cópia em **português**:
`README.pt-BR.md`, `CHANGELOG.pt-BR.md`, `CONTRIBUTING.pt-BR.md`, `docs/pt-BR/<mesmo caminho de docs/>`
e `tests/referencias/README.pt-BR.md`. A primeira linha de cada arquivo liga à outra língua. Ao mudar
um documento, atualize as duas versões (se você só escreve numa das línguas, diga isso no pull request
e alguém completa a outra). Mantenha identificadores de código, nomes de funções do original, caminhos,
chaves JSON e opções de linha de comando iguais nas duas. A terminologia está fixada no glossário.

## Compilar e testar

Ver [Compilar a partir do código](README.pt-BR.md#compilar-a-partir-do-código) no README. Antes de abrir
um pull request:

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Os testes que precisam dos arquivos do jogo avisam e passam sem eles; o CI
(`.github/workflows/ci.yml`) roda sem o jogo, então, se a sua mudança mexe em comportamento coberto por
testes com arquivos reais, rode-os localmente com a sua cópia e diga isso.

## Pull requests

- Mensagens de commit em inglês.
- Descreva o que mudou, **o que foi verificado** (testes executados, comparação com o original, quais
  vetores ou capturas) e o que não foi.
- Se a mudança altera comportamento, atualize a spec e, se ela se afasta do original,
  `docs/divergencias.md` ou `docs/decisoes.md` (data, decisão, motivo, alternativas descartadas).
- Acrescente uma entrada ao histórico de versões (`CHANGELOG.md` e `CHANGELOG.pt-BR.md`) para mudanças
  que quem joga ou faz mods vai notar.
