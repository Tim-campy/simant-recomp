# Lançamento da versão 1.0 (preparação)

Documento de trabalho para o lançamento da **1.0.0**, preparado em 2026-10-19 para revisão. **Nada foi
publicado:** não há tag, não houve push, os `Cargo.toml` continuam em `0.1.0` e a API de mods continua
0.34. O que precisa de decisão do usuário está marcado com **[DECIDIR]**.

Arquivos preparados neste branch:

| Arquivo | O que é |
|---|---|
| `.github/workflows/release.yml` | build e pacote de Windows x64 e Linux x64, disparado **só** por tag `v*`; cria o release como **rascunho** |
| `tools/gerar_avisos_terceiros.py` | gera o `AVISOS-TERCEIROS.txt` do pacote (licenças do que entra no executável) |
| `CHANGELOG.md` | rascunho do histórico, com a seção `[1.0.0] — não lançado` |
| `LEIAME-usuario.md` | LEIAME de quem joga, em pt-BR (vai no pacote como `LEIAME.txt`) |
| `README-user.md` | versão em inglês, opcional (vai como `README.txt` se existir) |
| `docs/lancamento.md` | este documento |
| `simant/src/launcher/`, `config_launcher.rs`, `registro.rs`, `pastas.rs` | launcher, configuração, saída sem console e pastas ao lado do executável (seção 4, "Melhorias feitas") |

## 1. Licença do projeto — **decidido (2026-10-19): "MIT OR Apache-2.0"**

O usuário escolheu a opção recomendada. Arquivos: `LICENSE` (aviso da licença dupla e da exclusão dos arquivos do SimAnt original), `LICENSE-MIT`, `LICENSE-APACHE` (texto oficial de apache.org); `license = "MIT OR Apache-2.0"` no `Cargo.toml` do workspace; o pacote leva os três. A análise que levou à escolha segue abaixo.

O repositório **não tem `LICENSE`** (conferido: nem `LICENSE*`, nem `COPYING*`, nem campo `license` nos
`Cargo.toml`). Sem licença, o código público no GitHub é "todos os direitos reservados": dá para ler,
mas ninguém pode legalmente redistribuir, modificar ou empacotar. A escolha é do autor; o workflow de
lançamento **falha de propósito** enquanto não existir `LICENSE` na raiz.

Todas as dependências (seção 2) são MIT, Apache-2.0, zlib ou duplas: **as três opções abaixo são
compatíveis com elas.** O que muda é o que os outros podem fazer com o port e com os mods.

| Opção | Prós | Contras |
|---|---|---|
| **MIT** | A mais simples e conhecida; a mesma da maioria das dependências (Lua, mlua, sdl2, rustysynth); qualquer um pode empacotar (distribuições Linux, lojas, Flatpak) e reaproveitar partes (parsers dos formatos da Maxis, sintetizador FM) sem atrito. | Permite forks fechados e uso comercial sem devolver melhorias. Sem cláusula de patentes. |
| **Apache-2.0** | Permissiva como a MIT, mais concessão explícita de patentes e regras claras para contribuições (seção 5); padrão de muitos projetos Rust (em geral como "MIT OR Apache-2.0"). | Texto longo; exige marcar arquivos modificados e manter um `NOTICE` se houver; incompatível com GPL-2.0-only (compatível com GPL-3.0). Patentes são pouco relevantes aqui. |
| **GPL-3.0** (ou **-or-later**) | Copyleft: quem distribuir uma versão modificada precisa publicar o código; mantém o port e os derivados abertos (é a escolha de vários ports e reimplementações de jogos, como OpenTTD e OpenRCT2). | Afasta quem quer reaproveitar partes em projetos permissivos ou fechados; dúvidas sobre mods: scripts Lua que só usam a API e arquivos de dados em geral não são "obra derivada", mas convém declarar isso (exceção ou nota no `docs/api-mods.md`). Distribuir binário exige oferecer o código-fonte correspondente (o link do GitHub com a tag basta). |
| Combinação **"MIT OR Apache-2.0"** | Convenção do ecossistema Rust; cada usuário escolhe. | Dois arquivos de licença; pouco ganho para um jogo. |

Pontos que valem para qualquer escolha:

- A licença cobre **só o código e os dados do port**, nunca os arquivos da Maxis. Vale repetir isso no
  `LICENSE` (parágrafo à parte) e no README.
- **[DECIDIR] Conteúdo de `data/` vindo do original.** Os `data/*.json` trazem tabelas numéricas
  extraídas do `SIMANTW.EXE` (`tools/extrair_*.py`) e algumas dezenas de frases curtas de interface do
  jogo em inglês (por exemplo em `mensagens.json`, `dialogos.json`, `amarela.json`,
  `superficie.json`, `ajuda.json`). Números de regras costumam ser tratados como fatos; frases do jogo
  são texto da Maxis. O repositório já é público com eles; antes de empacotá-los num lançamento,
  convém decidir: (a) manter como está, (b) ler essas frases do banco/executável do usuário em tempo de
  execução (como já acontece com a maior parte dos textos, que vêm de `SHARED`) ou (c) declarar na
  licença que esses trechos não são cobertos por ela.
- Depois da escolha: criar `LICENSE`, pôr `license = "<SPDX>"` e `repository =
  "https://github.com/Tim-campy/simant-recomp"` em `[workspace.package]` (e `license.workspace = true`
  nos dois crates), e citar a licença no `README.md`.

## 2. Licenças das dependências

Levantadas com `cargo metadata` e `cargo tree -p simant -e normal,no-proc-macro --target <alvo>`
(o `cargo-deny` não está instalado; ver o item opcional no checklist). Só o que é **ligado ao
executável** entra no pacote; dependências de build (`cc`, `lua-src` como ferramenta, `pkg-config`...),
macros (`serde_derive`, `syn`, `quote`, `proc-macro2`, `unicode-ident` — esta Unicode-3.0) e de teste
(`sha2` e cia.) não vão no binário.

Ligadas ao `simant` (33 crates no Windows; no Linux sai `windows-link`):

| Licença declarada | Crates | Aviso no pacote? |
|---|---|---|
| MIT | `mlua`, `mlua-sys`, `rustysynth`, `sdl2`, `simd-adler32`, `zmij` | **Sim**: a MIT exige o aviso de copyright e de permissão em "todas as cópias", inclusive binárias |
| MIT AND Zlib | `sdl2-sys` (a parte zlib são os cabeçalhos do SDL) | Sim (a parte MIT) |
| MIT OR Apache-2.0 (e variantes: `Apache-2.0 OR MIT`, `MIT/Apache-2.0`) | `bitflags`, `bstr`, `cfg-if`, `crc32fast`, `either`, `fdeflate`, `flate2`, `itoa`, `lazy_static`, `libc`, `lock_api`, `num-traits`, `parking_lot`, `parking_lot_core`, `png`, `rustc-hash`, `scopeguard`, `serde`, `serde_core`, `serde_json`, `smallvec`, `windows-link` | Sim, pela MIT (escolhida no pacote) |
| 0BSD OR MIT OR Apache-2.0 | `adler2` | Opcional (0BSD não pede aviso); incluído pela MIT |
| Unlicense OR MIT | `memchr` | Opcional; incluído pela MIT |
| MIT OR Zlib OR Apache-2.0 | `miniz_oxide` 0.8 e 0.9 | Incluído pela MIT |

Fora das crates, também dentro do pacote:

| Componente | Licença | Aviso |
|---|---|---|
| **Lua 5.4.8**, compilado dentro do executável (`mlua`, feature `vendored`, via `lua-src`) | MIT (© Lua.org, PUC-Rio) | **Sim** |
| Biblioteca padrão do Rust (ligada estaticamente) | MIT OR Apache-2.0 | Sim (MIT) |
| **font8x8** (fonte 8 × 8 do launcher, `simant/src/launcher/fonte8x8.rs`; Daniel Hepper, derivada das fontes VGA da IBM) | **domínio público** | Não exige; o pacote cita a origem |
| **SDL2 2.30.9** (`SDL2.dll`, só no pacote de Windows; binário oficial de libsdl.org, sem alteração) | **zlib** (`vendor/SDL2-2.30.9/COPYING.txt`, conferido) | A zlib **não exige** aviso em binários (só pede para não deturpar a origem e manter o aviso nas distribuições de *código*), mas o pacote inclui o texto e o `README-SDL.txt`, como o próprio SDL recomenda |

Nenhuma dependência é copyleft (GPL/LGPL/MPL). O `tools/gerar_avisos_terceiros.py` monta o
`AVISOS-TERCEIROS.txt` a partir do que está no `Cargo.lock` e dos arquivos de licença de cada crate
(46 KB no Windows), então a lista acompanha as atualizações de dependências sem edição manual. No
Linux o SDL2 é o do sistema e não vai no pacote.

## 3. O pacote

### Conteúdo

```
simant-recomp-1.0.0-windows-x64/          simant-recomp-1.0.0-linux-x64/
  simant.exe   (com o launcher dentro)      simant      (com o launcher dentro)
  simant-console.bat (mensagens no console) simant.sh   (atalho; não é mais preciso)
  SDL2.dll, README-SDL.txt
  data/          (44 JSON, 353 KB)          data/
  exemplos/mods/ (exemplo-lua, trapacas)    exemplos/mods/
  mods/LEIAME.txt (pasta vazia para mods)   mods/LEIAME.txt
  LICENSE.txt, LEIAME.txt, README.txt,      LICENSE.txt, LEIAME.txt, README.txt,
  CHANGELOG.txt, AVISOS-TERCEIROS.txt       CHANGELOG.txt, AVISOS-TERCEIROS.txt
```

**Nunca entram:** arquivos do jogo (`SimAnt-T/`, `*.DAT`, `*.NDX`, `*.HLP`, `*.FON`, `*.CFG`, `*.SYM`,
`SIMANTW.EXE`, `WIN.INI`), saves `*.ANT` (nem os de `tests/referencias/saves/`), `re_trabalho/`, `.re/`,
capturas (`tests/referencias/capturas/`, `*.png`), `referencia/` (DOSBox-X, `qemu/`, imagens),
`research_notes/`, WAVs, MIDIs e SoundFonts. O pacote é montado copiando só a lista acima (nunca a raiz
do repositório), e o passo "Conferir o conteúdo" do workflow **falha o build** se aparecer qualquer
arquivo com essas extensões ou caminhos.

### Tamanho (medido localmente, Windows x64)

Pacote montado à mão com os mesmos passos do workflow, a partir do `cargo build --release -p simant`
de 2026-10-19 (sem o `LICENSE`, que ainda não existe):

| Item | Tamanho |
|---|---|
| `simant.exe` | 5,8 MB (5.784.576 bytes) |
| `SDL2.dll` | 1,6 MB |
| `data/` + `exemplos/` + textos | ~0,45 MB |
| **Total descompactado** | **7,7 MB em 57 arquivos** |
| **`.zip`** | **2,9 MB** (2.864.192 bytes) |

O pacote de Linux não foi montado localmente (só no runner): sem o `SDL2.dll`, deve ficar em torno de
6 MB descompactado e 2–2,5 MB em `.tar.gz` [estimativa, conferir no rascunho do release].

### Verificado localmente (2026-10-19, Windows 11, `SIMANT_DIR` = a instalação de `SimAnt-T`)

- `cargo build --release -p simant`: compila sem avisos (2 min 44 s do zero).
- Sem janela: `simant --novo-jogo --ticks 5 --sem-preferencias` → `5 ticks: GameTime 0 -> 5`, código 0
  (0,5 s); o mesmo com `--sem-som` (0,05 s); `--save tests/referencias/saves/QUICK2.ANT --ticks 5
  --sem-som` → `GameTime 78 -> 83`.
- De dentro da pasta do pacote: `--novo-jogo --ticks 600 --sem-som --mods exemplos/mods` carrega os
  dois mods de exemplo (`[exemplo-lua] tick 500: 27 pedaços de comida`), código 0.
- Com janela: `--sair-apos-ms 2000` com e sem `--sem-som` abre a janela 640×461 e sai com código 0.
- Sem os arquivos do jogo: `--novo-jogo --ticks 5 --jogo <pasta inexistente>` → `erro: não encontrei
  .../SHARED.ndx`, código 1, sem pânico (é o que o smoke test do workflow no Linux espera).
- `.github/workflows/release.yml` é YAML válido (lido com PyYAML; gatilho só `push: tags: ["v*"]`).

Verificado em 2026-10-19 depois do launcher (Windows 11, build release, executável de janela):

- `cargo build --release -p simant`: `simant.exe` com 6.053.376 bytes (era 5.784.576).
- Com a saída redirecionada (pipes, como no CI): `--config <cfg> --novo-jogo --ticks 600 --mods
  exemplos/mods` imprime os `[script]` dos mods e `600 ticks: GameTime 0 -> 600`; `--sem-config
  --novo-jogo --ticks 5 --jogo /nao/existe` → `erro: não encontrei .../SHARED.ndx`, código 1.
- Sem console nem redirecionamento (`Start-Process`): a mesma saída foi para `%APPDATA%\SimAnt
  recomp\SimAnt\simant.log`; com a pasta do jogo errada apareceu a caixa de erro (fechada pela fila de
  mensagens dela) e o código foi 1; com `--console`, código 0 e o log não foi tocado.
- Pacote de demonstração em outra pasta (exe, `SDL2.dll`, `data/`, `exemplos/mods`), rodado com a pasta
  atual em `%TEMP%`: `--mods exemplos/mods --ticks 600` achou `data/` e os mods ao lado do executável.
- Launcher: `--launcher-jogar-apos-ms 800 --config <cfg>` abriu a janela do launcher, apertou Jogar,
  gravou o `launcher.json` e abriu o jogo no mesmo processo (`--sair-apos-ms 1500 --capturar-janela`
  fechou e gravou a tela); o diálogo de pasta e de arquivo do Windows foi conferido pelo teste
  `dialogo_do_windows_escolhe_pasta_e_arquivo_e_cancela` (ignorado por padrão: abre o diálogo ~1 s).
- `cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` passa (com
  um compilador C de mentira para o Lua: só o Rust do alvo Linux foi conferido, nada foi ligado nem
  rodado no Linux).

**Não verificado:** o workflow em si (não foi disparado); o launcher no Linux (janela, `zenity`/`kdialog`,
`simant.log` aberto pelo gerenciador de arquivos); o build e a execução no Linux; o jogo no Linux
com os arquivos do jogo em nomes minúsculos (o banco aceita `.NDX`/`.ndx`, os arquivos soltos são
procurados sem diferença de caixa, mas não há teste num sistema de arquivos que diferencie caixa);
`simant-console.bat` no duplo clique (o `--console` foi conferido só pelo `AllocConsole`).

## 4. Checklist da 1.0

### Antes da tag

- [ ] **[DECIDIR]** Licença (seção 1); criar `LICENSE`; `license`/`repository` nos `Cargo.toml`.
- [ ] **[DECIDIR]** Frases do original em `data/` (seção 1).
- [ ] **Versão 1.0.0 nos `Cargo.toml`** — proposta, **não aplicada**. Uma linha, porque os dois crates
      herdam do workspace (`version.workspace = true`):
      ```diff
       [workspace.package]
       edition = "2021"
      -version = "0.1.0"
      +version = "1.0.0"
      ```
      e `cargo build` para atualizar o `Cargo.lock` (os pacotes `engine` e `simant`). O workflow
      confere que a tag `vX.Y.Z` é igual a essa versão e falha se não for.
- [ ] **Congelar a API de mods em 1.0** — proposta, **não aplicada**. Em `docs/api-mods.md`:
      - topo: "Versão da API: **1.0** (estável)";
      - linha nova no histórico: `| 1.0 | Congelamento: a API 0.34 passa a ser a 1.0, sem mudança de
        formato nem de função. | Compatível com 0.34. A partir da 1.0, mudanças seguem o versionamento
        semântico: 1.x só acrescenta (chaves opcionais, arquivos, funções, ganchos); remover ou mudar o
        sentido de algo exige 2.0, com aviso numa 1.x anterior. |`;
      - em "Planejado", trocar "Remover o aceite das cópias antigas das tabelas comuns (0.20) numa versão
        futura" por "... na 2.0" (é a única remoção pendente e quebraria a regra da 1.x);
      - `API_VERSAO` dos scripts passa a `"1.0"`: **avisar** que scripts que comparam com `"0.34"` como
        texto precisam aceitar `"1.0"` (o valor está em `engine/src/scripts.rs`,
        único lugar com `"0.34"`; os mods de exemplo só imprimem a versão);
      - registrar em `docs/decisoes.md` (data, decisão, motivo, alternativas: manter 0.x; numerar a API
        junto com o port).
- [ ] Revisar e datar o `CHANGELOG.md` (trocar "não lançado" pela data; números de testes e vetores
      conferidos com o `docs/estado-do-projeto.md` da data).
- [ ] Revisar `LEIAME-usuario.md` e `README-user.md` (tirar "rascunho"; conferir a tabela de arquivos do
      jogo com uma instalação limpa).
- [ ] `README.md` do repositório: seção "Baixar" apontando para os releases, aviso legal (o mesmo do
      LEIAME: sem afiliação com a Maxis/EA, não distribui arquivos do jogo) e a licença.
- [ ] `cargo test --workspace` com e sem `SIMANT_DIR`; `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] Testar o pacote de Linux numa máquina real com os arquivos do jogo (inclusive com nomes em
      minúsculas, como saem de alguns CDs e cópias).
- [ ] Opcional: `cargo install cargo-deny` e um `deny.toml` com a lista de licenças aceitas
      (MIT, Apache-2.0, Zlib, 0BSD, Unlicense, Unicode-3.0), rodando no CI.

### Melhorias feitas para a 1.0 (2026-10-19)

- **Pastas relativas ao executável:** `data/`, `mods/` e `SimAnt-T/MAXIS/ANTWIN` são procurados ao lado
  do executável e só depois na pasta atual (`simant/src/pastas.rs`); o `simant.sh` do Linux fica só como
  atalho.
- **Sem console no Windows, com log e caixa de erro:** `windows_subsystem = "windows"` no perfil release,
  `simant.log` na pasta de dados do usuário, `SDL_ShowSimpleMessageBox` nos erros fatais, `--console` e
  `simant-console.bat` (`simant/src/registro.rs`).
- **Launcher:** o `simant` sem opções abre a tela de opções de inicialização, com a pasta do jogo
  (seletor nativo e conferência dos arquivos), início, vídeo, som, mods, preferências e a linha de comando
  equivalente editável; a escolha fica em `launcher.json` e vale também para o `simant` rodado com
  opções (`simant/src/launcher/`, `simant/src/config_launcher.rs`). Captura em `docs/imagens/launcher.png`.
  Decisões em `docs/decisoes.md` (2026-10-19: launcher, executável sem console, pastas).

### Melhorias sugeridas (não feitas)

- Ícone e metadados do `.exe` (recurso de versão).
- Launcher: campos de texto sem seleção com o mouse (só cursor, Ctrl+V cola e Ctrl+C copia o campo
  inteiro); sem leitor de tela; no Linux sem `zenity`/`kdialog` não há seletor (digita-se o caminho).

### Lançar (feito pelo usuário)

1. Fazer merge deste branch e das decisões acima no `main`; esperar o CI verde.
2. `git tag -a v1.0.0 -m "SimAnt recomp 1.0.0"` e `git push origin v1.0.0`.
3. O workflow "Lançamento" roda: testes, build, pacote, conferência de conteúdo e smoke test, e cria o
   release **v1.0.0 em rascunho** com os dois pacotes e os `.sha256`, e com as notas tiradas da seção
   `[1.0.0]` do `CHANGELOG.md` mais o aviso legal.
4. Baixar os dois pacotes do rascunho, conferir o conteúdo e rodar com os arquivos do jogo.
5. Publicar o rascunho na página de releases do GitHub (o workflow nunca publica sozinho).

Se algo falhar depois da tag: apagar o rascunho e a tag (`git push --delete origin v1.0.0`,
`git tag -d v1.0.0`), corrigir e recriar a tag.

## 5. Detalhes do workflow

- **Gatilho:** só `push` de tag `v*`. Não roda em push de branch nem em PR (o `ci.yml` continua cuidando
  disso).
- **Windows** (`windows-2022`, `x86_64-pc-windows-msvc`): baixa `SDL2-devel-2.30.9-VC.zip` do release
  oficial do SDL e confere o SHA-256 (`8c91d91e…52ae`, igual ao `vendor/` local) antes de extrair em
  `vendor/`, onde o `simant/build.rs` já procura; o `SDL2.dll` vai no pacote.
- **Linux** (`ubuntu-22.04`, glibc 2.35, para rodar em distribuições não tão novas): `libsdl2-dev` do
  sistema; o executável liga dinamicamente com `libSDL2-2.0.so.0`, que o usuário instala (LEIAME §1).
- **Passos:** versão da tag = `Cargo.toml` e `LICENSE` presente → `cargo test --workspace --release` →
  `cargo build --release --locked` → pacote → conferência de conteúdo → smoke test nos dois sistemas,
  de fora da pasta do pacote (erro sem os arquivos do jogo, código 1; captura do launcher sem janela,
  enviada como artefato `launcher-<so>`, fora do pacote) → `.zip`/`.tar.gz` com `.sha256` → artefatos → job `rascunho` cria o release com
  `gh release create --draft --verify-tag`.
- **Permissões:** `contents: read` por padrão; só o job `rascunho` tem `contents: write`.
