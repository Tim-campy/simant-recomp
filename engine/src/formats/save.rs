//! Jogo salvo do original (`.ANT`): concatenação de blocos de memória, sem cabeçalho.
//! Especificação: docs/spec/formatos/save-ant.md (tabela de blocos e seção "Estruturas principais").
//!
//! A ordem e o tamanho dos blocos vêm de `data/save_layout.json` (gerado por `tools/save_table.py --json`),
//! não deste código. Aqui ficam só as vistas tipadas das estruturas cuja forma foi vista no código do
//! original (mapas coluna-major, listas paralelas de formigas).

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// Uma entrada da tabela de blocos: `count × tamanho` bytes do símbolo, em ordem no arquivo.
#[derive(Clone, Debug, Deserialize)]
pub struct BlocoLayout {
    /// Nome do símbolo no `SIMANTW.SYM`, sem o `_` inicial (ex.: `MapA`).
    pub simbolo: String,
    pub count: usize,
    pub tamanho: usize,
    /// Endereço `segmento:offset` no original, só para referência.
    #[serde(default)]
    pub endereco: String,
}

impl BlocoLayout {
    pub fn bytes(&self) -> usize {
        self.count * self.tamanho
    }
}

/// Layout completo do `.ANT` (conteúdo de `data/save_layout.json`).
#[derive(Clone, Debug, Deserialize)]
pub struct LayoutSave {
    pub total_bytes: usize,
    pub blocos: Vec<BlocoLayout>,
}

impl LayoutSave {
    pub fn de_json(texto: &str) -> Result<LayoutSave, String> {
        let l: LayoutSave = serde_json::from_str(texto).map_err(|e| format!("layout do save: {e}"))?;
        l.validar()?;
        Ok(l)
    }

    pub fn carregar(caminho: &Path) -> Result<LayoutSave, String> {
        let texto = std::fs::read_to_string(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
        LayoutSave::de_json(&texto).map_err(|e| format!("{}: {e}", caminho.display()))
    }

    /// Confere que os nomes são únicos e que a soma dos blocos bate com `total_bytes`.
    pub fn validar(&self) -> Result<(), String> {
        let mut vistos = HashMap::new();
        for (i, b) in self.blocos.iter().enumerate() {
            if let Some(j) = vistos.insert(b.simbolo.as_str(), i) {
                return Err(format!("símbolo {} repetido nos blocos {j} e {i}", b.simbolo));
            }
        }
        let soma: usize = self.blocos.iter().map(BlocoLayout::bytes).sum();
        if soma != self.total_bytes {
            return Err(format!("soma dos blocos ({soma}) difere de total_bytes ({})", self.total_bytes));
        }
        Ok(())
    }
}

/// Mapa de bytes guardado coluna a coluna, como no original: índice `x * altura + y`.
#[derive(Clone, Copy, Debug)]
pub struct Mapa<'a> {
    pub largura: usize,
    pub altura: usize,
    pub dados: &'a [u8],
}

impl Mapa<'_> {
    pub fn indice(&self, x: usize, y: usize) -> usize {
        x * self.altura + y
    }

    pub fn em(&self, x: usize, y: usize) -> Option<u8> {
        (x < self.largura && y < self.altura).then(|| self.dados[self.indice(x, y)])
    }
}

/// As três listas de formigas: superfície (A), ninho preto (B) e ninho vermelho (R).
/// Cada lista guarda as formigas que estão naquela área, de qualquer colônia.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lista {
    A,
    B,
    R,
}

impl Lista {
    pub const TODAS: [Lista; 3] = [Lista::A, Lista::B, Lista::R];

    pub fn letra(self) -> char {
        match self {
            Lista::A => 'A',
            Lista::B => 'B',
            Lista::R => 'R',
        }
    }

    /// Símbolo de um campo paralelo (`X`, `Y`, `M`, `T` ou `S`), ex.: `AlistX`.
    pub fn simbolo(self, campo: char) -> String {
        format!("{}list{campo}", self.letra())
    }

    /// Contador de entradas usadas (`ListIndexA/B/R`).
    pub fn simbolo_contador(self) -> String {
        format!("ListIndex{}", self.letra())
    }

    /// Mapa por onde a lista anda: `MapA` (superfície) ou `MapB`/`MapR` (ninhos).
    pub fn simbolo_mapa(self) -> String {
        format!("Map{}", self.letra())
    }
}

/// Uma entrada das listas paralelas `?listX/Y/M/T/S`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Formiga {
    pub x: u8,
    pub y: u8,
    /// Campo M: modo/tarefa atual (índice dos contadores por modo).
    pub modo: u8,
    /// Campo T: tipo. 0 = entrada vazia. Ver [`Formiga::vermelha`], [`Formiga::classe`], [`Formiga::sub`].
    pub tipo: u8,
    /// Campo S: valor auxiliar do modo [hipótese: alvo/contador].
    pub s: u8,
}

impl Formiga {
    /// Bit 7 do tipo: colônia vermelha (fato: `_GetFromAlist`, `_CountAnts`).
    pub fn vermelha(&self) -> bool {
        self.tipo & 0x80 != 0
    }

    /// Bits 3–6 do tipo: classe (0 = cria, 12 = rainha...). Ver a spec.
    pub fn classe(&self) -> u8 {
        (self.tipo >> 3) & 0x0f
    }

    /// Bits 0–2 do tipo: direção (adultos) ou estágio (crias) [hipótese].
    pub fn sub(&self) -> u8 {
        self.tipo & 0x07
    }
}

/// Um `.ANT` carregado: os bytes originais e o índice de cada bloco por símbolo.
#[derive(Clone, Debug)]
pub struct JogoSalvo {
    dados: Vec<u8>,
    blocos: Vec<(String, usize, usize)>,
    indice: HashMap<String, usize>,
}

// Dimensões vistas no código do original (docs/spec/formatos/save-ant.md, "Estruturas principais").
const SUPERFICIE: (usize, usize) = (128, 64);
const NINHO: (usize, usize) = (64, 64);
const FEROMONIO: (usize, usize) = (64, 32);

impl JogoSalvo {
    pub fn ler(layout: &LayoutSave, dados: Vec<u8>) -> Result<JogoSalvo, String> {
        layout.validar()?;
        if dados.len() != layout.total_bytes {
            return Err(format!(
                "save com {} bytes; o layout espera {}",
                dados.len(),
                layout.total_bytes
            ));
        }
        let mut blocos = Vec::with_capacity(layout.blocos.len());
        let mut indice = HashMap::new();
        let mut pos = 0;
        for b in &layout.blocos {
            indice.insert(b.simbolo.clone(), blocos.len());
            blocos.push((b.simbolo.clone(), pos, b.bytes()));
            pos += b.bytes();
        }
        Ok(JogoSalvo { dados, blocos, indice })
    }

    pub fn abrir(layout: &LayoutSave, caminho: &Path) -> Result<JogoSalvo, String> {
        let d = std::fs::read(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
        JogoSalvo::ler(layout, d).map_err(|e| format!("{}: {e}", caminho.display()))
    }

    /// Bytes do arquivo, prontos para gravar (mesma ordem do original).
    pub fn bytes(&self) -> &[u8] {
        &self.dados
    }

    pub fn gravar(&self, caminho: &Path) -> Result<(), String> {
        std::fs::write(caminho, &self.dados).map_err(|e| format!("{}: {e}", caminho.display()))
    }

    /// Símbolos na ordem do arquivo.
    pub fn simbolos(&self) -> impl Iterator<Item = &str> {
        self.blocos.iter().map(|(s, _, _)| s.as_str())
    }

    pub fn bloco(&self, simbolo: &str) -> Option<&[u8]> {
        let &(_, pos, n) = &self.blocos[*self.indice.get(simbolo)?];
        Some(&self.dados[pos..pos + n])
    }

    pub fn bloco_mut(&mut self, simbolo: &str) -> Option<&mut [u8]> {
        let &(_, pos, n) = &self.blocos[*self.indice.get(simbolo)?];
        Some(&mut self.dados[pos..pos + n])
    }

    fn exigir(&self, simbolo: &str, tamanho: usize) -> Result<&[u8], String> {
        let b = self.bloco(simbolo).ok_or_else(|| format!("bloco {simbolo} não existe no layout"))?;
        if b.len() != tamanho {
            return Err(format!("bloco {simbolo} tem {} bytes; esperado {tamanho}", b.len()));
        }
        Ok(b)
    }

    fn exigir_mut(&mut self, simbolo: &str, tamanho: usize) -> Result<&mut [u8], String> {
        let b = self.bloco_mut(simbolo).ok_or_else(|| format!("bloco {simbolo} não existe no layout"))?;
        if b.len() != tamanho {
            return Err(format!("bloco {simbolo} tem {} bytes; esperado {tamanho}", b.len()));
        }
        Ok(b)
    }

    // --- escalares (little-endian) ---

    pub fn i16(&self, simbolo: &str) -> Result<i16, String> {
        let b = self.exigir(simbolo, 2)?;
        Ok(i16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u16(&self, simbolo: &str) -> Result<u16, String> {
        Ok(self.i16(simbolo)? as u16)
    }

    pub fn i32(&self, simbolo: &str) -> Result<i32, String> {
        let b = self.exigir(simbolo, 4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn u32(&self, simbolo: &str) -> Result<u32, String> {
        Ok(self.i32(simbolo)? as u32)
    }

    /// Blocos de 4 bytes que são um par de palavras, como os `*Pnt` (x, y).
    pub fn par_i16(&self, simbolo: &str) -> Result<(i16, i16), String> {
        let b = self.exigir(simbolo, 4)?;
        Ok((i16::from_le_bytes([b[0], b[1]]), i16::from_le_bytes([b[2], b[3]])))
    }

    /// Bloco inteiro lido como vetor de palavras (ex.: `CastePopB`, `H_BPop`).
    pub fn vetor_i16(&self, simbolo: &str) -> Result<Vec<i16>, String> {
        let b = self.bloco(simbolo).ok_or_else(|| format!("bloco {simbolo} não existe no layout"))?;
        if b.len() % 2 != 0 {
            return Err(format!("bloco {simbolo} tem tamanho ímpar ({})", b.len()));
        }
        Ok(b.as_chunks::<2>().0.iter().map(|&c| i16::from_le_bytes(c)).collect())
    }

    pub fn definir_i16(&mut self, simbolo: &str, v: i16) -> Result<(), String> {
        self.exigir_mut(simbolo, 2)?.copy_from_slice(&v.to_le_bytes());
        Ok(())
    }

    pub fn definir_i32(&mut self, simbolo: &str, v: i32) -> Result<(), String> {
        self.exigir_mut(simbolo, 4)?.copy_from_slice(&v.to_le_bytes());
        Ok(())
    }

    // --- mapas ---

    fn mapa(&self, simbolo: &str, (largura, altura): (usize, usize)) -> Result<Mapa<'_>, String> {
        let dados = self.exigir(simbolo, largura * altura)?;
        Ok(Mapa { largura, altura, dados })
    }

    /// Superfície: 128 colunas × 64 linhas (`MapA`).
    pub fn mapa_a(&self) -> Result<Mapa<'_>, String> {
        self.mapa("MapA", SUPERFICIE)
    }

    /// Ninho preto: 64 × 64 (`MapB`); y = profundidade, y = 0 é a linha da superfície.
    pub fn mapa_b(&self) -> Result<Mapa<'_>, String> {
        self.mapa("MapB", NINHO)
    }

    /// Ninho vermelho: 64 × 64 (`MapR`).
    pub fn mapa_r(&self) -> Result<Mapa<'_>, String> {
        self.mapa("MapR", NINHO)
    }

    /// Mapa da área de uma lista (`MapA`, `MapB` ou `MapR`).
    pub fn mapa_da_lista(&self, lista: Lista) -> Result<Mapa<'_>, String> {
        match lista {
            Lista::A => self.mapa_a(),
            Lista::B => self.mapa_b(),
            Lista::R => self.mapa_r(),
        }
    }

    /// Mapa de saída de um ninho (`ExitMapB`/`ExitMapR`), 64 × 64 como o ninho.
    pub fn mapa_saida(&self, simbolo: &str) -> Result<Mapa<'_>, String> {
        self.mapa(simbolo, NINHO)
    }

    /// Feromônio (`PherMapA`, `PherMapBN`, `PherMapBT`, `PherMapRN`, `PherMapRT`): 64 × 32,
    /// uma célula para cada 2 × 2 tiles da superfície (célula = (x >> 1, y >> 1)).
    pub fn feromonio(&self, simbolo: &str) -> Result<Mapa<'_>, String> {
        self.mapa(simbolo, FEROMONIO)
    }

    // --- listas de formigas ---

    /// Número de entradas usadas da lista (`ListIndex?`), limitado à capacidade dos vetores.
    pub fn usadas(&self, lista: Lista) -> Result<usize, String> {
        let n = self.i16(&lista.simbolo_contador())?;
        let cap = self.capacidade(lista)?;
        Ok((n.max(0) as usize).min(cap))
    }

    /// Capacidade da lista: 1000 (A) ou 500 (B, R), segundo o layout.
    pub fn capacidade(&self, lista: Lista) -> Result<usize, String> {
        let s = lista.simbolo('X');
        Ok(self.bloco(&s).ok_or_else(|| format!("bloco {s} não existe no layout"))?.len())
    }

    /// Entrada `i` da lista (sem checar se está em uso).
    pub fn formiga(&self, lista: Lista, i: usize) -> Result<Formiga, String> {
        let cap = self.capacidade(lista)?;
        if i >= cap {
            return Err(format!("índice {i} fora da lista {} ({cap})", lista.letra()));
        }
        let campo = |c: char| -> Result<u8, String> { Ok(self.exigir(&lista.simbolo(c), cap)?[i]) };
        Ok(Formiga { x: campo('X')?, y: campo('Y')?, modo: campo('M')?, tipo: campo('T')?, s: campo('S')? })
    }

    /// Entradas `0..ListIndex` com tipo diferente de 0 (as de tipo 0 são buracos que o
    /// original remove em `_CompactList?`).
    pub fn formigas(&self, lista: Lista) -> Result<Vec<Formiga>, String> {
        let mut v = Vec::new();
        for i in 0..self.usadas(lista)? {
            let f = self.formiga(lista, i)?;
            if f.tipo != 0 {
                v.push(f);
            }
        }
        Ok(v)
    }
}
