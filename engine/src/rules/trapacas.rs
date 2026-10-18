//! Códigos de trapaça do original: o anel de 4 letras de `_DoKeyDown` (`1000:0e8c`) e os efeitos
//! de `_CheatKeys` (`1018:67de`). Especificação: docs/spec/regras/trapacas.md. Dados:
//! data/codigos_trapaca.json (códigos, constantes, sons e mensagens).
//!
//! Fora do tick: é entrada do jogador. Os sorteios (ovos, `joel`) são os mesmos do original.

use std::sync::OnceLock;

use serde::Deserialize;

use super::amarela::{self, TabelasAmarela};
use super::comum::{self, Direcoes};
use super::estado::{Estado, Evento};
use super::geracao::Cor;
use crate::recursos::Recursos;

/// Nome do arquivo de dados (sobrescrevível por mod).
pub const ARQUIVO_TABELAS: &str = "codigos_trapaca.json";

const JSON_PADRAO: &str = include_str!("../../../data/codigos_trapaca.json");

/// Efeito de um código (o caso de `_CheatKeys`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EfeitoTrapaca {
    RainhasGuardadasVermelhas,
    SaudeAmarelaCheia,
    SaudeAmarelaMinima,
    Dinheiro,
    Piada,
    BuracosPretos,
    BuracosVermelhos,
    RainhaPreta,
    RainhaVermelha,
    OvosPretos,
    OvosVermelhos,
    SaudePretaCheia,
    SaudeVermelhaCheia,
    SaudeVermelhaZero,
    SaudePretaZero,
    RainhasGuardadasPretas,
    Invencivel,
    AranhaDeRe,
    ColoniasPretasNoQuintal,
    ColoniasNoQuintal,
    AnimacaoVitoria,
    NuncaComFome,
    SempreSaudavel,
    BancoDeDados,
    RedeServidor,
    RedeCliente,
}

impl EfeitoTrapaca {
    /// O port não tem o que o original faz aqui (animação `_DrawSimPayoff`, DDE, NetBIOS).
    pub fn sem_equivalente(self) -> bool {
        matches!(self, EfeitoTrapaca::AnimacaoVitoria | EfeitoTrapaca::BancoDeDados | EfeitoTrapaca::RedeServidor | EfeitoTrapaca::RedeCliente)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Codigo {
    /// 4 letras A–Z ou a–z.
    pub codigo: String,
    pub efeito: EfeitoTrapaca,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ValoresTrapaca {
    pub rainhas_guardadas: i16,
    pub saude_amarela_cheia: i32,
    pub saude_amarela_minima: i32,
    pub saude_colonia: i16,
    pub ovos: u32,
    pub ovos_sorteio: i16,
    pub ovo_tipo_preto: u8,
    pub ovo_tipo_vermelho: u8,
    pub buracos: i32,
    pub rainha_dx: i32,
    pub rainha_direcao: i32,
    pub populacao_jeff: u8,
    pub colonias_jeff_por_coluna: i16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SonsTrapaca {
    pub rainhas_vermelhas: u16,
    pub rainhas_pretas: u16,
    pub saude_amarela: u16,
    pub jeff: u16,
    pub joel: u16,
    pub ligou: u16,
    pub desligou: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MensagensTrapaca {
    pub dinheiro: u16,
    pub piada: Vec<u16>,
}

/// Conteúdo de `data/codigos_trapaca.json`.
#[derive(Clone, Debug, Deserialize)]
pub struct TabelasTrapaca {
    pub codigos: Vec<Codigo>,
    pub valores: ValoresTrapaca,
    pub sons: SonsTrapaca,
    pub mensagens: MensagensTrapaca,
}

impl TabelasTrapaca {
    pub fn carregar(r: &Recursos) -> Result<TabelasTrapaca, String> {
        let t: TabelasTrapaca = r.dados(ARQUIVO_TABELAS)?;
        t.validar()?;
        Ok(t)
    }

    pub fn de_json(texto: &str) -> Result<TabelasTrapaca, String> {
        let t: TabelasTrapaca = serde_json::from_str(texto).map_err(|e| format!("{ARQUIVO_TABELAS}: {e}"))?;
        t.validar()?;
        Ok(t)
    }

    /// As tabelas embutidas (cópia de `data/codigos_trapaca.json` da compilação).
    pub fn padrao() -> &'static TabelasTrapaca {
        static PADRAO: OnceLock<TabelasTrapaca> = OnceLock::new();
        PADRAO.get_or_init(|| TabelasTrapaca::de_json(JSON_PADRAO).expect("data/codigos_trapaca.json embutido"))
    }

    fn validar(&self) -> Result<(), String> {
        for c in &self.codigos {
            if c.codigo.len() != 4 || !c.codigo.bytes().all(|b| b.is_ascii_alphabetic()) {
                return Err(format!("{ARQUIVO_TABELAS}: código inválido \"{}\" (4 letras A–Z ou a–z)", c.codigo));
            }
        }
        if self.valores.ovos_sorteio < 1 {
            return Err(format!("{ARQUIVO_TABELAS}: valores.ovos_sorteio deve ser ≥ 1"));
        }
        if !(0..=64).contains(&self.valores.buracos) {
            return Err(format!("{ARQUIVO_TABELAS}: valores.buracos fora de 0..64"));
        }
        Ok(())
    }

    /// O efeito do código, se a tabela o tem (o primeiro igual, como a busca do original).
    pub fn efeito(&self, letras: &[u8; 4]) -> Option<EfeitoTrapaca> {
        self.codigos.iter().find(|c| c.codigo.as_bytes() == letras).map(|c| c.efeito)
    }
}

/// O anel de 4 letras (`_edata[0..4]`, posição `DAT_1048_01da`, contagem `DAT_1048_01dc`). Fora do
/// save.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnelTrapaca {
    letras: [u8; 4],
    /// Próxima posição a escrever (= a letra mais antiga quando o anel está cheio).
    pos: usize,
    /// Letras guardadas (até 4).
    n: usize,
}

impl AnelTrapaca {
    /// Enter: zera a posição e a contagem.
    pub fn limpar(&mut self) {
        self.pos = 0;
        self.n = 0;
    }

    /// Uma letra (`b'A'..=b'Z'` com Shift, `b'a'..=b'z'` sem): entra no anel; com 4 letras, devolve
    /// o efeito do código formado pelas 4 últimas, da mais antiga para a mais nova.
    pub fn digitar(&mut self, letra: u8, t: &TabelasTrapaca) -> Option<EfeitoTrapaca> {
        self.letras[self.pos] = letra;
        self.pos = (self.pos + 1) % 4;
        self.n = (self.n + 1).min(4);
        if self.n < 4 {
            return None;
        }
        let ultimas = [0, 1, 2, 3].map(|k| self.letras[(self.pos + k) % 4]);
        t.efeito(&ultimas)
    }
}

/// A letra que `_DoKeyDown` põe no anel para a tecla `nome` (nome do SDL: "A".."Z"): minúscula sem
/// Shift, maiúscula com Shift. `None` para as outras teclas.
pub fn letra_da_tecla(nome: &str, shift: bool) -> Option<u8> {
    let &[b] = nome.as_bytes() else { return None };
    if !b.is_ascii_alphabetic() {
        return None;
    }
    Some(if shift { b.to_ascii_uppercase() } else { b.to_ascii_lowercase() })
}

fn alternar(v: &mut i16) -> bool {
    *v = i16::from(*v == 0);
    *v != 0
}

/// `_CheatKeys(k)`: aplica o efeito ao estado. Pedidos à interface (sons, quadros) saem pela fila
/// de eventos.
pub fn aplicar(e: &mut Estado, t: &TabelasTrapaca, ta: &TabelasAmarela, efeito: EfeitoTrapaca) {
    let v = &t.valores;
    let s = &t.sons;
    use EfeitoTrapaca::*;
    match efeito {
        RainhasGuardadasVermelhas => {
            e.pedir_som(Some(s.rainhas_vermelhas));
            let q = &mut e.vermelha.rainhas_guardadas;
            *q = q.wrapping_add(v.rainhas_guardadas);
        }
        SaudeAmarelaCheia | SaudeAmarelaMinima => {
            e.pedir_som(Some(s.saude_amarela));
            let alvo = if efeito == SaudeAmarelaCheia { v.saude_amarela_cheia } else { v.saude_amarela_minima };
            amarela::definir_saude(e, ta, alvo);
        }
        Dinheiro => e.eventos.push(Evento::Mensagem(t.mensagens.dinheiro)),
        Piada => {
            // _myBeginSoundList(0) entre os dois quadros: lista vazia no Windows (como no Mystery).
            for &id in &t.mensagens.piada {
                e.eventos.push(Evento::Mensagem(id));
            }
        }
        BuracosPretos | BuracosVermelhos => {
            let cor = if efeito == BuracosPretos { Cor::Preta } else { Cor::Vermelha };
            for c in 0..v.buracos {
                comum::novo_buraco(e, ta, &ta.borda_buraco, cor, c);
            }
        }
        RainhaPreta | RainhaVermelha => {
            let cor = if efeito == RainhaPreta { Cor::Preta } else { Cor::Vermelha };
            let (x, y) = (i32::from(e.amarela.x) + v.rainha_dx, i32::from(e.amarela.y));
            amarela::fazer_rainha(e, ta, cor, x, y, v.rainha_direcao);
        }
        OvosPretos | OvosVermelhos => {
            let (cor, base) = if efeito == OvosPretos { (Cor::Preta, v.ovo_tipo_preto) } else { (Cor::Vermelha, v.ovo_tipo_vermelho) };
            for _ in 0..v.ovos {
                // Ordem dos sorteios da desmontagem (trapacas.md §2.1): tipo (R), y (S), x (S).
                let tipo = base.wrapping_add(e.r.rrand(v.ovos_sorteio) as u8);
                let (_, dy) = ta.d8(e.s.mascara(8));
                let (dx, _) = ta.d8(e.s.mascara(8));
                let (x, y) = (i32::from(e.amarela.x) + dx, i32::from(e.amarela.y) + dy);
                comum::por_ovo(e, ta, cor, x, y, tipo);
            }
        }
        SaudePretaCheia => e.preta.saude = v.saude_colonia,
        SaudeVermelhaCheia => e.vermelha.saude = v.saude_colonia,
        SaudeVermelhaZero => e.vermelha.saude = 0,
        SaudePretaZero => {
            e.partida.sempre_saudavel = 0;
            e.preta.saude = 0;
        }
        RainhasGuardadasPretas => {
            let q = &mut e.preta.rainhas_guardadas;
            *q = q.wrapping_add(v.rainhas_guardadas);
            e.pedir_som(Some(s.rainhas_pretas));
        }
        Invencivel | AranhaDeRe => {
            let ligou = if efeito == Invencivel { alternar(&mut e.partida.invencivel) } else { alternar(&mut e.aranha.de_re) };
            e.pedir_som(Some(if ligou { s.ligou } else { s.desligou }));
        }
        ColoniasPretasNoQuintal => {
            e.pedir_som(Some(s.jeff));
            let p = &mut e.preta;
            for coluna in p.populacao_quintal.0.chunks_mut(16) {
                p.colonias_fundadas = p.colonias_fundadas.wrapping_add(v.colonias_jeff_por_coluna);
                for c in coluna {
                    *c = c.wrapping_add(v.populacao_jeff);
                }
            }
        }
        ColoniasNoQuintal => {
            e.pedir_som(Some(s.joel));
            for i in 0..e.preta.populacao_quintal.0.len() {
                let cor = if e.s.mascara(2) == 0 { Cor::Preta } else { Cor::Vermelha };
                let c = e.colonia_mut(cor);
                c.populacao_quintal.0[i] = c.populacao_quintal.0[i].wrapping_add(1);
                c.colonias_fundadas = c.colonias_fundadas.wrapping_add(1);
            }
        }
        NuncaComFome => {
            let ligou = alternar(&mut e.partida.nunca_com_fome);
            if ligou {
                e.amarela.saude = v.saude_amarela_cheia as i16;
            }
            e.pedir_som(Some(if ligou { s.ligou } else { s.desligou }));
        }
        SempreSaudavel => {
            let ligou = alternar(&mut e.partida.sempre_saudavel);
            if ligou {
                e.preta.saude = v.saude_colonia;
            }
            e.pedir_som(Some(if ligou { s.ligou } else { s.desligou }));
        }
        AnimacaoVitoria | BancoDeDados | RedeServidor | RedeCliente => {}
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn tab() -> &'static TabelasTrapaca {
        TabelasTrapaca::padrao()
    }

    fn digitar(anel: &mut AnelTrapaca, texto: &str) -> Vec<EfeitoTrapaca> {
        texto.bytes().filter_map(|b| anel.digitar(b, tab())).collect()
    }

    #[test]
    fn tabela_tem_os_26_codigos_na_ordem_do_original() {
        let c: Vec<&str> = tab().codigos.iter().map(|c| c.codigo.as_str()).collect();
        assert_eq!(c.join(" "), "oops rand erad FUND joke hole HOLE qeen QEEN eggs EGGS heal HEAL zhea ZHEA just will mick jeff joel fred jenn susi data serv clie");
    }

    #[test]
    fn anel_maiusculas_e_enter() {
        let mut a = AnelTrapaca::default();
        assert_eq!(digitar(&mut a, "xxheal"), vec![EfeitoTrapaca::SaudePretaCheia]);
        assert_eq!(digitar(&mut a, "HEAL"), vec![EfeitoTrapaca::SaudeVermelhaCheia]);
        assert!(digitar(&mut a, "Heal").is_empty(), "Shift só no H não casa");
        assert_eq!(digitar(&mut a, "healheal").len(), 2);
        a.limpar();
        assert!(digitar(&mut a, "he").is_empty());
        a.limpar();
        assert!(digitar(&mut a, "al").is_empty(), "Enter limpa o anel");
        assert_eq!(letra_da_tecla("H", false), Some(b'h'));
        assert_eq!(letra_da_tecla("h", true), Some(b'H'));
        assert_eq!(letra_da_tecla("F1", false), None);
        assert_eq!(letra_da_tecla("1", false), None);
    }
}
