# Registro de decisões

Formato: data, decisão, motivo, alternativas descartadas.

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
