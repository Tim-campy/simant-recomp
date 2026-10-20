*Read in English: [menu.md](../../../spec/formatos/menu.md)*

# Menu da janela principal (objeto tipo 6 do banco)

Leitor: `engine/src/formats/menu.rs` (`MenuBanco::decodificar`, `comando`), testes `objeto_sintetico` e `numeracao_dos_comandos`. Uso: [../telas/menus.md](../telas/menus.md).

O `SIMANTW.EXE` não tem recurso MENU: `_InitMenu(0)` (`1008:4e36`) lê o objeto **6/0** do banco (`SHARED`) e corrige os ponteiros somando o endereço do bloco, antes de montar a barra com `AppendMenu` [código].

## Formato (verificado no objeto 6/0, 726 bytes, 2026-10-18)

- No início, uma lista de ponteiros `u32` little-endian (deslocamentos a partir do início do objeto), terminada em 0. O primeiro aponta para a lista dos **títulos**; os seguintes, um por título, para a lista dos **itens** daquele menu.
- Cada lista é de ponteiros `u32` para textos ASCIIZ, terminada em 0.
- Textos: `&` antes da letra sublinhada, `\t` (0x09) antes do atalho mostrado; `---` = separador.

No 6/0: títulos `&File`, `&Window`, `&View`, `&Options`, `&Speed`; itens:

| Menu | Itens |
|---|---|
| File | `&About`, `---`, `&New\tCtrl+N`, `&Load...\tCtrl+L`, `&Save\tCtrl+S`, `Save &As...\tCtrl+A`, `---`, `&Quit\tCtrl+Q` |
| Window | `&Edit\tCtrl+E`, `&Map\tCtrl+M`, `&Behavior\tCtrl+B`, `&Caste\tCtrl+C`, `His&tory\tCtrl+H`, `&Info\tCtrl+I`, `&Status\tCtrl+T`, `&Help\tF1` |
| View | `&Yard\tCtrl+Y`, `&House\tCtrl+5`, `&Strategic\tCtrl+6`, `&Graphic\tCtrl+7`, `---`, `&Surface\tCtrl+8`, `&Black Nest\tCtrl+9`, `&Red Nest\tCtrl+0` |
| Options | `&AutoTrack\tCtrl+G`, `&Music\tCtrl+J`, `&Sound Effects\tCtrl+K`, `&Events`, `Messa&ges`, `Si&lly` |
| Speed | `&Pause   \tShift+0`, `---`, `&Slow\tShift+1`, `&Normal\tShift+2`, `&Fast\tShift+3`, `&Ultra\tShift+4` |

## Números dos comandos

`_InitMenu` dá a cada item o número `0xFD01 + 16 × menu + item` (separadores incluídos): About 0xFD01, Quit 0xFD08, Status 0xFD17, Pause 0xFD41 etc. São os mesmos números da ACCELERATORTABLE do executável ([ne-recursos.md](ne-recursos.md)).

**Submenus** [código, sem uso no 6/0]: um item cujo texto tem um caractere marcador seguido de um número carregaria outro objeto tipo 6 como lista simples (`_ms_LoadPopUpResource`: contagem no byte 1, textos Pascal), com números `0xF9x0 + item`.

**Mods:** o objeto 6/0 pode ser trocado por mod como qualquer objeto do banco (textos e atalhos mostrados); o que cada número faz fica em `data/barra_menus.json` (`comandos`).
