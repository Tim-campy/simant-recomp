# Registro de decisões

Formato: data, decisão, motivo, alternativas descartadas.

## 2026-10-17: Módulo comum das regras (`rules/comum.rs`) e tabelas em `RegrasOriginais`

- **Decisão:** as rotinas que o original tem uma vez só e que várias partes chamam ficam num só lugar, `engine/src/rules/comum.rs`: testes de tipo e de tile (`grupo`, cor, terra, comida, sólido, amarela), `_GetDir`, `_GetDis`, `_Bounce`, acesso por plano (`_GetMap`, `_GetLife`, `_IsClearTile`, `_IsNotObstacle`, `_IsThisPebble`, `_IsItHole`, `_TileCanBeMovedOn` na superfície, `_GetBestDir`), listas (`_FindInAList`, `_FindAntIndex`/`_FindInB/RList`), `_DeadAntHere`, `_PickupFoodA`, `_DropFoodA`, `_Recruit`, `_UnRecruit`, `_GetWinner`, `_GetNewMode`/`B`/`R`, `_KillSpider`, comida do ninho (`_EatFood*`, `_StealFood*`), `_DecEat*`, `_PlaceEgg*`, `_ExitHole` e a escavação (`_DigTile*`, `_DigTileThem*`, `_SmoothEdges*`, `_FixExitMap*`, `_IsClear3x3`, `_MakeNewHole*`). As direções (`_Dx8/_Dy8`) e o `_TurnTab` são lidos pelas traits `Direcoes` e `TabelaDeGiro`, implementadas pela tabela de cada parte. A escavação é escrita sobre a trait `Terreno` (com `Superficie`), implementada pelo `Estado` (tick) e pelo `Mundo` da geração junto com o gerador S dela, para que a geração e o tick usem o mesmo código. As rotinas da formiga amarela (`_YellowFight`, `_DoTroph`, `_YellowDeath`, `_MoveMyLife`, `_SetMyHealth`) ficam em `amarela.rs` e são chamadas pelas outras partes.
- **Tabelas:** `RegrasOriginais` guarda as tabelas de cada parte (`quintal`, `inimigos`, `superficie`, `feromonios`, `ninho`, `amarela`, além de `geracao`) e o estado do tutorial fora do save (`memoria_tutor`, `interface_tutor`). `RegrasOriginais::carregar(&Recursos)` lê tudo de `data/` com mods; `RegrasOriginais::default()` usa as cópias embutidas (testes). As funções públicas de cada parte recebem as tabelas explicitamente (as de quem mata ou alimenta a amarela recebem também `&TabelasAmarela`). A fase de cheiro do tick (`_DoSmells`) virou o método `RegrasSuperficie::fase_de_cheiro`, cujo padrão roda com as tabelas embutidas.
- **Motivo:** cada parte foi escrita por um agente diferente e tinha a sua cópia; as cópias divergiam em detalhes (ver abaixo), os pontos que dependiam da amarela estavam vazios ou provisórios, e um mod que trocasse `data/<parte>.json` não chegava ao tick. Uma rotina única garante que uma correção vale para todas as partes, como no original.
- **Divergências entre as cópias, e o que ficou:** (1) `_GetNewMode` com modo estratégico fora de 0..5: o ninho usava a última linha da tabela, a superfície devolvia o modo 0; ficou a última linha (estado inválido; o original leria a memória seguinte). (2) `_DropFoodA` na casa com tile 0x27–0x3F ("a comida cai", tabela que o port não tem): a amarela dava sucesso, a aranha falha; ficou sucesso (o original devolve sucesso depois da queda; a aranha não usa o resultado). (3) `_GetDis`: a superfície truncava a 16 bits; `_GetDis` devolve 32 bits (quintal.md) e só a superfície trunca, no chamador. (4) `_YellowFight`/`_DoTroph` da superfície eram parciais (sem `_YellowDeath`, sem a amarela virar para quem a alimenta, sem `_LessonTemp`); ficaram os de `amarela.rs`. (5) `_YellowDeath`/`_MoveMyLife` dos inimigos eram provisórios (sem animações, mensagens nem reencarnação); ficaram os de `amarela.rs`. (6) A agonia de `_KillSpider` era a constante 500 no quintal e `aranha_agonia` nos inimigos; ficou o dado. (7) `TabelasSuperficie::carregar` lia o arquivo da pasta do jogo (`Recursos::arquivo`) em vez de `data/`; corrigido. (8) O `_SetMyHealth` do tutorial tinha 100 e 9 fixos; usa o de `amarela.rs`. As demais cópias eram equivalentes (diferiam só em testes de limite inalcançáveis). Nenhuma diferença muda os vetores de tick do original (placar idêntico antes e depois, em todos os cenários).
- **Duas rotinas parecidas que ficaram separadas** (o original também tem duas): `_GetBestDir` (comum) × `_GetRedBestDirs` (superfície, usa `_TileCanBeMovedOn`) × `_GetMyBestDirs` (amarela, com as regras das linhas 0–1 e de cavar); `_MakeNewHole*` (comum) × `_CreateNewHole` (amarela, o buraco na própria célula); `_EnterNest`/`_ExitNest` da amarela × `_GoInNest`/`_ExitHole` das outras formigas.
- **Impacto nos mods:** nenhum formato muda. Os `data/*.json` das regras passam a valer no tick (API de mods 0.4, ver `api-mods.md`). As tabelas repetidas entre arquivos (`dx8`, `turn_tab`, luta, modos, borda do buraco) continuam uma cópia por arquivo, cada uma valendo para a sua parte; juntá-las num `data/comum.json` mudaria o formato e fica para quando houver sobrescrita parcial por chave.
- **Descartado:** um `Mundo` convertido para `Estado` antes da geração (a geração precisa do seu próprio gerador e dos vetores crus, e o teste `geracao_quick2` compara byte a byte); um único `data/comum.json` agora (quebraria os mods que já trazem esses arquivos); manter as cópias e só documentar (as divergências acima mostram o custo).

## 2026-10-16: Simulação = `Estado` único + tick fixo + trait `Regras` por assunto

- **Decisão:** todo o estado da simulação fica num valor `Estado` (`engine/src/rules/estado.rs`), com os 307 blocos do `.ANT` em grupos tipados por assunto e os campos só de execução (geradores S e R, mapas de vida, contagens do tick, fila de eventos). O tick (`engine/src/rules/tick.rs`) é uma função `tick(&mut Estado, &mut dyn Regras)` que segue a ordem e as periodicidades de `_DoAntSim` e faz as partes pequenas e já especificadas (contadores, alimentação, compactação, `_CountAnts`, `_HistUpdate`, resumo de modos). As sub-rotinas grandes são métodos de seis traits (`RegrasQuintal`, `RegrasInimigos`, `RegrasSuperficie`, `RegrasNinho`, `RegrasAmarela`, `RegrasConselhos`), com implementação padrão vazia, somadas em `Regras`. `RegrasOriginais` implementa o jogo, uma trait por arquivo de `rules/`.
- **Motivo:** a ordem do tick é parte da regra (sorteios na ordem certa), então ela fica num só lugar e testada. As traits separadas permitem que cada assunto seja escrito num arquivo próprio, em paralelo, sem conflito, e que um assunto ainda não escrito simplesmente não faça nada. O estado explícito, sem globais, deixa os testes determinísticos e permite ida e volta byte a byte com o `.ANT`. As regras não tocam em áudio nem em janela: pedem mensagens, músicas e diálogos por `Estado::eventos`.
- **Impacto nos mods:** a API de scripts ainda não existe, mas a arquitetura define os pontos que serão expostos:
  - **leitura e escrita do `Estado`** pelos grupos nomeados (`colonia(cor)`, `lista(area)`, `feromonios`, `amarela`...), não por endereços do original. Os nomes dos campos são parte da API e passam a ter nota de compatibilidade quando forem expostos;
  - **ganchos por sub-rotina do tick**: um mod poderá envolver ou substituir um método de `Regras` (por exemplo, um `RegrasNinho` próprio) mantendo o resto do jogo. A ordem do tick fica fixa e fora do alcance dos mods, para preservar o determinismo;
  - **eventos**: mensagens e sons pedidos pela simulação passam por `Estado::eventos`, e um mod poderá observar ou acrescentar eventos;
  - **dados**: velocidades e valores iniciais de jogo novo estão em `data/simulacao.json`, já sobrescrevível por mod.
- **Descartado:** variáveis globais espelhando a memória do original (difícil de testar e de expor a mods); um único `trait Regras` com todos os métodos e um único `impl` (cada agente ou mod teria de editar o mesmo bloco); despacho por tabela de ponteiros de função montada em tempo de execução (perde a verificação de tipos sem ganho agora; pode ser a forma da API de mods mais tarde).

## 2026-10-16: Tabelas da geração do mundo em `data/geracao.json`; seno lido do banco

- **Decisão:** as tabelas que a geração do mundo lê do executável (`_Dx8/_Dy8`, bordas do buraco e da formiga-leão, `_SowTab`, formas de pedra) ficam em `data/geracao.json`, gerado por `tools/extrair_geracao.py` a partir do `SIMANTW.EXE`. A tabela de seno, que no original é um objeto do banco (tipo 9 nº 1000), é lida do banco em tempo de execução, como os demais objetos. Os geradores aleatórios (`engine/src/rules/rng.rs`) são valores com estado explícito, passados por parâmetro, sem estado global.
- **Motivo:** dados fora do código (princípio 4) e reprodução exata do original: o teste `engine/tests/geracao_quick2.rs` só é determinístico se cada chamada ao gerador for explícita.
- **Impacto nos mods:** um mod pode substituir `geracao.json` inteiro (sobrescrita por arquivo, como os demais `data/*.json`) ou o objeto 9/1000. Os valores de tile usados pelas regras (buraco 0x50, comida 0x48, terra 0x2E...) e as quantidades fixas da geração (20/30/100/128 objetos, 200 unidades de comida) continuam como constantes nomeadas no código, documentadas em `docs/spec/regras/tiles-valores.md` e `geracao-mundo.md`; movê-los para dados fica para quando houver API de mods de regras, porque mudá-los sem mudar as regras que os testam quebraria a simulação.
- **Descartado:** copiar as tabelas para constantes Rust (impede mods); ler o executável em tempo de execução (exigiria o `.EXE` além dos bancos e um parser NE no motor).

## 2026-10-16: Análise do código com Ghidra headless, fora do REA

- **Decisão:** usar o Ghidra 12.1.4 diretamente (headless, loader NE), com os nomes do `SIMANTW.SYM` aplicados por script.
- **Motivo:** o REA recusa executáveis NE ("unsupported NE executable in MZ image"), e o modo Ghidra dele no Windows só aceita PE.
- **Descartado:** carregar segmentos soltos no REA como binário cru de x86. As chamadas far entre segmentos e as importações do Windows ficariam sem relocação.

## 2026-10-16: Material de engenharia reversa fica em `re_trabalho/`, fora do port

- **Decisão:** pseudocódigo, dumps e PNGs extraídos ficam em `re_trabalho/` (no `.gitignore`). O port só recebe especificação em `docs/spec/` e código reescrito.
- **Motivo:** regra do projeto de não copiar código decompilado, e de não redistribuir assets protegidos.

## 2026-10-16: Utilitários de `tools/` em Python 3 (só biblioteca padrão)

- **Decisão:** os inspetores de formato (`ne_info.py`, `db_list.py`, `lzss_probe.py`, `save_table.py`) são escritos em Python 3.12, sem dependências externas.
- **Motivo:** já está instalado e é rápido para explorar formatos. Isso **não** define a linguagem do port.
- **Descartado:** nada ainda. Se o port for em outra linguagem, os parsers definitivos serão reescritos nela, e os scripts Python continuam como referência e oráculo para os testes.

## 2026-10-16: Save do port em formato próprio

- **Decisão:** o port terá formato de save próprio, versionado e legível. O `.ANT` original será só importado.
- **Motivo:** o `.ANT` é um dump cru de memória, sem versão nem cabeçalho, e amarrado aos tamanhos fixos do original (ver `spec/formatos/save-ant.md`). Um formato próprio deixa os mods adicionarem estado.
- **Impacto nos mods:** mods podem gravar chaves extras no save. Importar `.ANT` continua possível para comparar com o original.

## 2026-10-16: Linguagem do port: Rust + SDL2, mods em Lua

- **Decisão (do usuário):** Rust (stable, toolchain MSVC), SDL2 para janela, entrada e áudio, e Lua para scripts de mod (sandbox via `mlua`, a adotar quando o sistema de mods começar).
- **Motivo:** binário único e rápido, testes nativos (`cargo test`), tipos fortes para os parsers, e Lua é a linguagem de mod mais conhecida e fácil de isolar em sandbox.
- **Descartado:** C#/.NET + MoonSharp (alternativa razoável, mas exigiria runtime .NET), Python + pygame (o mais simples, porém com desempenho e distribuição mais fracos).
- **Organização:** workspace Cargo. O crate `engine` reúne `core`, `formats` e `rules`, sem dependência de janela. O crate `simant` é o executável com `render`, `audio` e `input` sobre SDL2. Os módulos seguem a arquitetura do `CLAUDE.md`.
- **SDL2 no Windows:** sem cmake na máquina, o crate `sdl2` não compila o SDL do fonte (feature `bundled`). Por isso usamos as bibliotecas oficiais pré-compiladas `SDL2-devel-<versão>-VC.zip` (libsdl-org, GitHub), extraídas em `vendor/` (no `.gitignore`). O `SDL2.dll` é copiado ao lado do executável.

## 2026-10-16: Bancada automática: QEMU + QMP sobre SvarDOS

- **Decisão (do usuário, após pesquisa):** testes automáticos contra o original rodam no QEMU (headless), controlado por QMP (`send-key`, `input-send-event`, `screendump`, `pmemsave`), com SvarDOS (kernel EDR-DOS) como DOS. Detalhes em `docs/bancada-qemu.md`.
- **Motivo:** no DOSBox-X oficial, as teclas injetadas pelo host não chegam ao Windows 3.x, o `AUTOTYPE` só funciona em DOS e o mouse é relativo e exige captura. O QMP injeta teclado e mouse no hardware emulado, captura sem foco e lê a memória.
- **Descartado:** FreeDOS (kernel 2043 recusa o Windows; o 2046 386 dá "Invalid Opcode"; com o 2046 8086 o WfW volta ao prompt; além disso, o SHARE do FreeDOS tem bug de gravação), forks do DOSBox-X com QMP (teclado pelo buffer do BIOS), Bochs (lento), 86Box (sem API), otvdm (não é fiel). Comparativo completo em `reports/Automação de testes Win16 emulado.md`.
- **Consequência:** a bancada usa 640×480 (VGA genérico). Para pixel a pixel em 800×600, a referência continua sendo o DOSBox-X.

## 2026-10-16: Ambiente de referência: DOSBox-X

- **Decisão (do usuário):** DOSBox-X rodando o Windows 3.1 já instalado em `SimAnt-T/`.
- **Configuração:** `referencia/dosbox-x.conf` (S3 Trio64, SB16, 32 MB, `C:` = `SimAnt-T`). As capturas vão para `tests/referencias/capturas/`.
- **Situação (2026-10-16):** DOSBox-X 2026.10.01 portátil (build `vsbuild-win64`, release oficial em github.com/joncampbell123/dosbox-x) extraído em `referencia/dosbox-x/` (fora do git). Para iniciar: `referencia/rodar-original.bat`. Testado: o Windows 3.1 inicia e o SimAnt abre direto na tela de título.
- **Descartado:** 86Box/PCem (mais fiel, porém de configuração mais pesada).
