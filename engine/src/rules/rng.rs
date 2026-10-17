//! Os dois geradores aleatórios do original. Especificação: docs/spec/regras/rng.md.
//!
//! - [`GeradorS`]: LFSR de Galois de 16 bits (`_SRand*`), usado pela geração do mundo e pela simulação.
//! - [`GeradorR`]: `rand()` do runtime do Microsoft C (`_RRand`), usado nas sementes dos patches, chuva e lutas.
//!
//! Cada gerador é um valor com estado explícito; nada é global. Quem precisar reproduzir o original
//! deve passar o mesmo gerador, na mesma ordem de chamadas, que o jogo usaria.

/// Polinômio do LFSR: aplicado (XOR) quando o bit 15 sai pela esquerda.
const POLINOMIO_S: u16 = 0x1BF5;

/// Gerador S (`DGROUP:0xcbf2` no original). O novo estado é também o valor sorteado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeradorS {
    estado: u16,
}

impl GeradorS {
    /// Cria com a semente dada (`_SetSRandSeed`). Com semente 0 o gerador devolve 0 para sempre.
    pub fn new(semente: u16) -> GeradorS {
        GeradorS { estado: semente }
    }

    /// `_GetSRandSeed`.
    pub fn semente(&self) -> u16 {
        self.estado
    }

    /// `_SetSRandSeed`.
    pub fn definir_semente(&mut self, semente: u16) {
        self.estado = semente;
    }

    /// Um passo do LFSR; devolve o novo estado.
    pub fn passo(&mut self) -> u16 {
        let vai_um = self.estado & 0x8000 != 0;
        self.estado <<= 1;
        if vai_um {
            self.estado ^= POLINOMIO_S;
        }
        self.estado
    }

    /// `_SRand1(n)`: resto **sem sinal** da divisão do passo por `n` (0…n−1). `n` deve ser > 0.
    pub fn rand1(&mut self, n: u16) -> i32 {
        assert!(n > 0, "SRand1(0) dividiria por zero");
        (self.passo() % n) as i32
    }

    /// `_SRand2` … `_SRand256`: passo com máscara `k − 1`. `k` deve ser potência de 2 entre 2 e 256.
    pub fn mascara(&mut self, k: u16) -> i32 {
        debug_assert!(k.is_power_of_two() && (2..=256).contains(&k));
        (self.passo() & (k - 1)) as i32
    }

    /// `_SGRand(n)`: menor de dois `SRand1(n)` (dois passos), puxado para 0.
    pub fn sg_rand(&mut self, n: u16) -> i32 {
        let a = self.rand1(n);
        let b = self.rand1(n);
        a.min(b)
    }

    /// `_SGIRand(n)`: maior de dois `SRand1(n)`.
    pub fn sgi_rand(&mut self, n: u16) -> i32 {
        let a = self.rand1(n);
        let b = self.rand1(n);
        a.max(b)
    }

    /// `_SGSRand(n)`: menor de dois `SRand1(n)` e depois `SRand2`; se der 1, o valor fica negativo.
    pub fn sgs_rand(&mut self, n: u16) -> i32 {
        let m = self.sg_rand(n);
        if self.mascara(2) != 0 {
            -m
        } else {
            m
        }
    }

    /// `_SeedSRand`: semente a partir do relógio (`_TickCount`, ticks de ~55 ms; só a palavra baixa).
    pub fn semear_com_ticks(&mut self, ticks: u32) {
        self.estado = (ticks as u16) ^ 0x3751;
    }
}

/// Gerador R: congruencial linear do runtime do Microsoft C (estado inicial 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeradorR {
    estado: u32,
}

impl Default for GeradorR {
    fn default() -> GeradorR {
        GeradorR { estado: 1 }
    }
}

impl GeradorR {
    /// `srand(v)`: o estado vira `v` (palavra alta zerada).
    pub fn new(semente: u16) -> GeradorR {
        GeradorR { estado: semente as u32 }
    }

    pub fn estado(&self) -> u32 {
        self.estado
    }

    /// Gerador com o estado interno de 32 bits dado (para retomar um estado capturado do original).
    pub fn de_estado(estado: u32) -> GeradorR {
        GeradorR { estado }
    }

    /// `rand()`: 0…0x7FFF.
    pub fn rand(&mut self) -> i32 {
        self.estado = self.estado.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
        ((self.estado >> 16) & 0x7FFF) as i32
    }

    /// `_RRand(n)`: `rand() mod n` com divisão com sinal. `n` deve ser > 0.
    pub fn rrand(&mut self, n: i16) -> i32 {
        assert!(n > 0, "RRand com n ≤ 0");
        self.rand() % n as i32
    }

    /// `_SeedRRand`: semente S do primeiro relógio, `k = SRand128()`, `srand(segundo relógio)` e
    /// descarta `k` valores. Os dois relógios são lidos separadamente no original.
    pub fn semear_com_ticks(&mut self, s: &mut GeradorS, ticks1: u32, ticks2: u32) {
        s.semear_com_ticks(ticks1);
        let k = s.mascara(128);
        *self = GeradorR::new(ticks2 as u16);
        for _ in 0..k {
            self.rand();
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn s_vetores_da_spec() {
        let mut s = GeradorS::new(1);
        for i in 1..=15 {
            assert_eq!(s.passo(), 1 << i);
        }
        let mut s = GeradorS::new(0x1234);
        let esperado = [0x2468, 0x48d0, 0x91a0, 0x38b5, 0x716a, 0xe2d4, 0xde5d, 0xa74f, 0x556b, 0xaad6];
        for e in esperado {
            assert_eq!(s.passo(), e);
        }
    }

    #[test]
    fn s_semente_zero_fica_em_zero() {
        let mut s = GeradorS::new(0);
        for _ in 0..10 {
            assert_eq!(s.rand1(7), 0);
        }
    }

    #[test]
    fn s_ciclo_de_7() {
        let ciclo = [0x3859u16, 0x70b2, 0xe164, 0xd93d, 0xa98f, 0x48eb, 0x91d6];
        let mut s = GeradorS::new(ciclo[0]);
        for i in 1..=7 {
            assert_eq!(s.passo(), ciclo[i % 7]);
        }
    }

    #[test]
    fn s_rand1_e_sem_sinal() {
        // 0xe2d4 = 58068: com sinal daria resto negativo; sem sinal, 58068 % 100 = 68.
        let mut s = GeradorS::new(0x716a);
        assert_eq!(s.rand1(100), 68);
        assert_eq!(s.semente(), 0xe2d4);
    }

    #[test]
    fn s_mascara_igual_ao_resto_para_potencias_de_2() {
        let mut a = GeradorS::new(0x1234);
        let mut b = a;
        for k in [2u16, 4, 8, 16, 32, 64, 128, 256] {
            assert_eq!(a.mascara(k), b.rand1(k));
        }
    }

    #[test]
    fn s_funcoes_g() {
        // Semente 0x1234: passos 0x2468 (9320), 0x48d0 (18640), 0x91a0 (37280).
        let mut s = GeradorS::new(0x1234);
        assert_eq!(s.sg_rand(1000), 320);
        let mut s = GeradorS::new(0x1234);
        assert_eq!(s.sgi_rand(1000), 640);
        // SGSRand: min(320, 640) = 320; depois 0x91a0 & 1 = 0, positivo.
        let mut s = GeradorS::new(0x1234);
        assert_eq!(s.sgs_rand(1000), 320);
        // Semente 0x2468: 18640 % 1000 = 640, 37280 % 1000 = 280, depois 0x38b5 & 1 = 1: negativo.
        let mut s = GeradorS::new(0x2468);
        assert_eq!(s.sgs_rand(1000), -280);
        assert_eq!(s.semente(), 0x38b5);
    }

    #[test]
    fn r_e_o_rand_do_microsoft_c() {
        // Sequência conhecida do rand() do MSVC com a semente padrão 1.
        let mut r = GeradorR::default();
        let esperado = [41, 18467, 6334, 26500, 19169, 15724, 11478, 29358, 26962, 24464];
        for e in esperado {
            assert_eq!(r.rand(), e);
        }
        let mut r = GeradorR::new(1);
        assert_eq!(r.rrand(10), 1);
        assert_eq!(r.rrand(100), 67);
    }

    #[test]
    fn r_semeado_pelo_relogio_descarta_k_valores() {
        let mut s = GeradorS::new(0);
        let mut r = GeradorR::default();
        r.semear_com_ticks(&mut s, 0, 1);
        // ticks1 = 0: S = 0x3751; um passo: 0x6ea2, & 127 = 0x22 = 34 descartes.
        let mut esperado = GeradorR::new(1);
        for _ in 0..34 {
            esperado.rand();
        }
        assert_eq!(r, esperado);
        assert_eq!(s.semente(), 0x6ea2);
    }
}
