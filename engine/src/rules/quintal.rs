//! Quintal (`_DoSimYard`, `_SimColonies`) e água (`_DoWater`).
//! Especificação: docs/spec/regras/simulacao-visao-geral.md, inimigos.md (menino, gato, cachorro, chuva), ninho.md (água).

use super::tick::{RegrasQuintal, RegrasOriginais};

// Ainda vazio: os métodos da trait usam a implementação padrão (não fazem nada).
impl RegrasQuintal for RegrasOriginais {}
