# Recursos do executável NE (`SIMANTW.EXE`): cursores e aceleradores

Leitor: `engine/src/formats/ne.rs` (`cursores(exe)`, `aceleradores(exe)`), testes `cursores_do_simantw` e `aceleradores_do_simantw`. Inspeção: `python tools/ne_info.py SIMANTW.EXE` lista os recursos.

## Tabela de recursos (formato NE do Windows 3.x)

- `MZ` no início; em 0x3C, o deslocamento do cabeçalho `NE`; em `NE + 0x24`, o deslocamento (relativo ao `NE`) da tabela de recursos.
- Tabela: `u16` deslocamento de alinhamento (os deslocamentos e tamanhos são em unidades de `1 << alinhamento`); depois, por tipo: `u16` tipo (bit 0x8000 = número), `u16` quantidade, `u32` reservado, e as entradas de 12 bytes: `u16` deslocamento, `u16` tamanho, `u16` flags, `u16` id (bit 0x8000 = número; senão, deslocamento de um nome Pascal a partir do início da tabela), `u32` reservado. Tipo 0 encerra.

## Cursores

- `GROUP_CURSOR` (12): `u16` reservado, `u16` tipo (2), `u16` número de imagens; entradas de 14 bytes, com o id do recurso `CURSOR` nos 2 últimos.
- `CURSOR` (1): `u16` x e `u16` y do ponto quente, depois um `BITMAPINFOHEADER` (altura dobrada), a paleta de 2 cores, a máscara XOR e a máscara AND, 1 bit por pixel, linhas de 4 bytes de baixo para cima. AND 1 + XOR 0 = transparente; AND 0 = preto (XOR 0) ou branco (XOR 1); AND 1 + XOR 1 = inverte.

No `SIMANTW.EXE` do usuário (verificado em 2026-10-18): 8 grupos com nome — `HELPCURSOR`, `MAGCURSOR`, `ROCKCURSOR`, `DIGCURSOR`, `ANTCURSOR`, `FOODCURSOR`, `DROPCURSOR`, `SPRAYCURSOR` —, todos 32×32. O `HELPCURSOR` é a seta com um "?" ao lado, ponto quente (0, 0), igual ao cursor das capturas da bancada no modo de ajuda. Uso: [../telas/ajuda-contexto.md](../telas/ajuda-contexto.md) §5.

## Aceleradores e menu

- `ACCELERATOR` (9): entradas de 5 bytes — `u8` flags (1 = tecla virtual, 4 = Shift, 8 = Ctrl, 0x10 = Alt; 0x80 marca a última), `u16` tecla, `u16` comando. A `ACCELERATORTABLE` do `SIMANTW.EXE` tem 47 entradas (verificado em 2026-10-18): Ctrl+N/L/S/A/Q (0xFD03–0xFD08), Ctrl+E/M/B/C/H/I/T (0xFD11–0xFD17), Ctrl+Y/5/6/7 (0xFD21–0xFD24), Ctrl+8/9/0 (0xFD26–0xFD28), Ctrl+G/J/K (0xFD31–0xFD33), Shift+0..4 e Shift + NumPad 0..4 (0xFD41, 0xFD43–0xFD46), e os de janela 0xFDA0–0xFDAC (janelas.md §8, navegacao.json). Os números são os dos itens do menu ([menu.md](menu.md)).
- **Não há recurso `MENU`**: a barra é montada por `_InitMenu` a partir do objeto 6/0 do banco ([menu.md](menu.md)).

## Segmentos (textos do DGROUP)

**Fonte:** formato NE (Microsoft "Executable-File Header Format"); `tools/ne_info.py`; `engine/src/formats/ne.rs`, `segmento`.

A tabela de segmentos começa em `ne + [ne + 0x22]`, com `[ne + 0x1C]` entradas de 8 bytes: setor (u16), tamanho no arquivo (u16, 0 = 64 KB), flags (u16; bit 0 = dados) e alocação mínima (u16). O segmento começa no arquivo em setor << `[ne + 0x32]` (alinhamento). A numeração é a do `SIMANTW.SYM` e dos endereços `n:offset` da documentação, a partir de 1 (segmento 10 = DGROUP, seletor 1048; segmento 8 = `SIMANT_DATA_GROUP`, seletor 1038). Os bytes do arquivo são os dados inicializados; a relocação vem depois e não entra.

O port lê daqui os textos do DGROUP que usa (API de mods 0.35: referências `{"exe": "SIMANTW.EXE", "segmento": 10, "offset": ...}` nos `data/*.json`; `docs/api-mods.md`, "Textos do original"), como strings ASCIIZ: por exemplo `10:014b`–`10:01bc` (os quatro textos de velocidade e os dois dos marcadores, `_DoUserButton`/`_DoBookMark`), `10:0900`/`10:0911` (item de pausa, `_SetMenuOptionText`), `10:0626` (título da janela principal, `CreateWindow` em `_InitInstance`), `10:1650`–`10:185a` (caixas e diálogos de arquivo de `_MenuQuit`, `_LoadGame`, `_FileSelect`, `_PopMsg`) e as peças do balão em `10:1c6e` (11 × 16 bytes; `_MakeBalloon`). Conferido: com o `SIMANTW.EXE` da instalação de referência, cada texto lido é igual ao que os `data/*.json` traziam (`engine/tests/textos_originais.rs`).
