# Recursos do executável NE (`SIMANTW.EXE`): cursores

Leitor: `engine/src/formats/ne.rs` (`cursores(exe)`), teste `cursores_do_simantw`. Inspeção: `python tools/ne_info.py SIMANTW.EXE` lista os recursos.

## Tabela de recursos (formato NE do Windows 3.x)

- `MZ` no início; em 0x3C, o deslocamento do cabeçalho `NE`; em `NE + 0x24`, o deslocamento (relativo ao `NE`) da tabela de recursos.
- Tabela: `u16` deslocamento de alinhamento (os deslocamentos e tamanhos são em unidades de `1 << alinhamento`); depois, por tipo: `u16` tipo (bit 0x8000 = número), `u16` quantidade, `u32` reservado, e as entradas de 12 bytes: `u16` deslocamento, `u16` tamanho, `u16` flags, `u16` id (bit 0x8000 = número; senão, deslocamento de um nome Pascal a partir do início da tabela), `u32` reservado. Tipo 0 encerra.

## Cursores

- `GROUP_CURSOR` (12): `u16` reservado, `u16` tipo (2), `u16` número de imagens; entradas de 14 bytes, com o id do recurso `CURSOR` nos 2 últimos.
- `CURSOR` (1): `u16` x e `u16` y do ponto quente, depois um `BITMAPINFOHEADER` (altura dobrada), a paleta de 2 cores, a máscara XOR e a máscara AND, 1 bit por pixel, linhas de 4 bytes de baixo para cima. AND 1 + XOR 0 = transparente; AND 0 = preto (XOR 0) ou branco (XOR 1); AND 1 + XOR 1 = inverte.

No `SIMANTW.EXE` do usuário (verificado em 2026-10-18): 8 grupos com nome — `HELPCURSOR`, `MAGCURSOR`, `ROCKCURSOR`, `DIGCURSOR`, `ANTCURSOR`, `FOODCURSOR`, `DROPCURSOR`, `SPRAYCURSOR` —, todos 32×32. O `HELPCURSOR` é a seta com um "?" ao lado, ponto quente (0, 0), igual ao cursor das capturas da bancada no modo de ajuda. Uso: [../telas/ajuda-contexto.md](../telas/ajuda-contexto.md) §5.
