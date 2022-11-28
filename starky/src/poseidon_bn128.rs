#![allow(clippy::derive_hash_xor_eq, clippy::too_many_arguments)]
use crate::poseidon_bn128_constants as constants;
use ff::*;

use crate::ElementDigest;
use winter_crypto::Hasher;
use winter_math::fields::f64::BaseElement;
use winter_math::FieldElement;

#[derive(PrimeField)]
#[PrimeFieldModulus = "21888242871839275222246405745257275088548364400416034343698204186575808495617"]
#[PrimeFieldGenerator = "7"]
pub struct Fr(pub FrRepr);

/// Using recommended parameters from whitepaper https://eprint.iacr.org/2019/458.pdf (table 2, table 8)
/// Generated by https://extgit.iaik.tugraz.at/krypto/hadeshash/-/blob/master/code/calc_round_numbers.py
/// And rounded up to nearest integer that divides by t
#[derive(Debug)]
pub struct Constants {
    pub c: Vec<Vec<Fr>>,
    pub m: Vec<Vec<Vec<Fr>>>,
    pub n_rounds_f: usize,
    pub n_rounds_p: Vec<usize>,
}

/// TODO: implement singleton instance
pub fn load_constants() -> Constants {
    let (c_str, m_str) = constants::constants();
    let mut c: Vec<Vec<Fr>> = Vec::new();
    for v1 in c_str {
        let mut cci: Vec<Fr> = Vec::new();
        for v2 in v1 {
            let b: Fr = from_hex(v2).unwrap();
            cci.push(b);
        }
        c.push(cci);
    }
    let mut m: Vec<Vec<Vec<Fr>>> = Vec::new();
    for v1 in m_str {
        let mut mi: Vec<Vec<Fr>> = Vec::new();
        for v2 in v1 {
            let mut mij: Vec<Fr> = Vec::new();
            for s in v2 {
                let b: Fr = from_hex(s).unwrap();
                mij.push(b);
            }
            mi.push(mij);
        }
        m.push(mi);
    }
    Constants {
        c,
        m,
        n_rounds_f: 8,
        n_rounds_p: vec![
            56, 57, 56, 60, 60, 63, 64, 63, 60, 66, 60, 65, 70, 60, 64, 68,
        ],
    }
}

pub struct Poseidon {
    constants: Constants,
}

impl Default for Poseidon {
    fn default() -> Self {
        Self::new()
    }
}

impl Poseidon {
    pub fn new() -> Poseidon {
        Poseidon {
            constants: load_constants(),
        }
    }
    pub fn ark(&self, state: &mut Vec<Fr>, c: &[Fr], it: usize) {
        for i in 0..state.len() {
            state[i].add_assign(&c[it + i]);
        }
    }

    pub fn sbox(&self, n_rounds_f: usize, n_rounds_p: usize, state: &mut Vec<Fr>, i: usize) {
        if i < n_rounds_f / 2 || i >= n_rounds_f / 2 + n_rounds_p {
            for x in state {
                let aux = *x;
                x.square();
                x.square();
                x.mul_assign(&aux);
            }
        } else {
            let aux = state[0];
            state[0].square();
            state[0].square();
            state[0].mul_assign(&aux);
        }
    }

    pub fn mix(&self, state: &Vec<Fr>, m: &[Vec<Fr>]) -> Vec<Fr> {
        let mut new_state: Vec<Fr> = Vec::new();
        for i in 0..state.len() {
            new_state.push(Fr::zero());
            for (j, x) in state.iter().enumerate() {
                let mut mij = m[i][j];
                mij.mul_assign(x);
                new_state[i].add_assign(&mij);
            }
        }
        new_state.clone()
    }

    /// Hash function
    /// init_state would be Fr::zero() initially
    pub fn hash(&self, inp: &Vec<Fr>, init_state: &Fr) -> Result<Fr, String> {
        let result = self.hash_inner(inp, init_state, 1)?;
        Ok(result[0])
    }

    pub fn hash_ex(&self, inp: &Vec<Fr>, init_state: &Fr, out: usize) -> Result<Vec<Fr>, String> {
        self.hash_inner(inp, init_state, out)
    }

    fn hash_inner(&self, inp: &Vec<Fr>, init_state: &Fr, out: usize) -> Result<Vec<Fr>, String> {
        if inp.is_empty() || inp.len() > self.constants.n_rounds_p.len() {
            return Err(format!(
                "Wrong inputs length {} > {}",
                inp.len(),
                self.constants.n_rounds_p.len()
            ));
        }

        let t = inp.len() + 1;
        let n_rounds_f = self.constants.n_rounds_f;
        let n_rounds_p = self.constants.n_rounds_p[t - 2];

        let mut state = vec![init_state.clone(); t];
        state[1..].clone_from_slice(&inp);

        for i in 0..(n_rounds_f + n_rounds_p) {
            self.ark(&mut state, &self.constants.c[t - 2], i * t);
            self.sbox(n_rounds_f, n_rounds_p, &mut state, i);
            state = self.mix(&state, &self.constants.m[t - 2]);
        }

        Ok((&state[0..out]).to_vec())
    }
}

/// hasher element over BN128
impl Hasher for Poseidon {
    type Digest = ElementDigest;

    fn hash(bytes: &[u8]) -> Self::Digest {
        let hasher = Poseidon::new();
        let elems: &[BaseElement] = unsafe { BaseElement::bytes_as_elements(bytes).unwrap() };
        debug_assert_eq!(elems.len(), 16 * 4);
        let elems: Vec<Fr> = elems
            .chunks(4)
            .map(|e| ElementDigest::to_BN128(e.try_into().unwrap()))
            .collect();
        let init_state = Fr::zero();
        let digest = hasher.hash(&elems, &init_state).unwrap();
        Self::Digest::from(&digest)
    }

    /// Returns a hash of two digests. This method is intended for use in construction of
    /// Merkle trees.
    fn merge(values: &[Self::Digest; 2]) -> Self::Digest {
        let hasher = Poseidon::new();
        let inp = vec![values[0].into(), values[1].into()];
        let init_state = Fr::zero();
        Self::Digest::from(&hasher.hash(&inp, &init_state).unwrap())
    }

    /// Returns hash(`seed` || `value`). This method is intended for use in PRNG and PoW contexts.
    fn merge_with_int(_seed: Self::Digest, _value: u64) -> Self::Digest {
        panic!("Unimplemented method");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ff() {
        let a = Fr::from_repr(FrRepr::from(2)).unwrap();
        assert_eq!(
            "0000000000000000000000000000000000000000000000000000000000000002",
            to_hex(&a)
        );

        let b: Fr = Fr::from_str(
            "21888242871839275222246405745257275088548364400416034343698204186575808495619",
        )
        .unwrap();
        assert_eq!(
            "0000000000000000000000000000000000000000000000000000000000000002",
            to_hex(&b)
        );
        assert_eq!(&a, &b);
    }

    #[test]
    fn test_load_constants() {
        let cons = load_constants();
        assert_eq!(
            cons.c[0][0].to_string(),
            "Fr(0x09c46e9ec68e9bd4fe1faaba294cba38a71aa177534cdd1b6c7dc0dbd0abd7a7)"
        );
        assert_eq!(
            cons.c[cons.c.len() - 1][0].to_string(),
            "Fr(0x2fb583762b37592c6c5a95eb1d06694b6c6f9dc4f1ad4862dd8f5e67cb7a3f5c)"
        );
        assert_eq!(
            cons.m[0][0][0].to_string(),
            "Fr(0x066f6f85d6f68a85ec10345351a23a3aaf07f38af8c952a7bceca70bd2af7ad5)"
        );
        assert_eq!(
            cons.m[cons.m.len() - 1][0][0].to_string(),
            "Fr(0x196b76cefdcc7f6a54c71d40114a0bb82694c936f1573ac7ac1ea3fcce1fe938)"
        );
    }

    #[test]
    fn test_poseidon_hash() {
        let poseidon = Poseidon::new();

        let b0: Fr = Fr::from_str("0").unwrap();
        let b1: Fr = Fr::from_str("1").unwrap();
        let b2: Fr = Fr::from_str("2").unwrap();
        let b3: Fr = Fr::from_str("3").unwrap();
        let b4: Fr = Fr::from_str("4").unwrap();
        let b5: Fr = Fr::from_str("5").unwrap();
        let b6: Fr = Fr::from_str("6").unwrap();

        let is = Fr::zero();
        let h = poseidon.hash(&vec![b1], &is).unwrap();
        assert_eq!(
            h.to_string(),
            "Fr(0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133)" // "18586133768512220936620570745912940619677854269274689475585506675881198879027"
        );

        let h = poseidon.hash(&vec![b1, b2], &is).unwrap();
        assert_eq!(
            h.to_string(),
            "Fr(0x115cc0f5e7d690413df64c6b9662e9cf2a3617f2743245519e19607a4417189a)" // "7853200120776062878684798364095072458815029376092732009249414926327459813530"
        );

        let h = poseidon.hash(&vec![b1, b2, b0, b0, b0], &is).unwrap();
        assert_eq!(
            h.to_string(),
            "Fr(0x024058dd1e168f34bac462b6fffe58fd69982807e9884c1c6148182319cee427)" // "1018317224307729531995786483840663576608797660851238720571059489595066344487"
        );

        let h = poseidon.hash(&vec![b1, b2, b0, b0, b0, b0], &is).unwrap();
        assert_eq!(
            h.to_string(),
            "Fr(0x21e82f465e00a15965e97a44fe3c30f3bf5279d8bf37d4e65765b6c2550f42a1)" // "15336558801450556532856248569924170992202208561737609669134139141992924267169"
        );

        let h = poseidon.hash(&vec![b3, b4, b0, b0, b0], &is).unwrap();
        assert_eq!(
            h.to_string(),
            "Fr(0x0cd93f1bab9e8c9166ef00f2a1b0e1d66d6a4145e596abe0526247747cc71214)" // "5811595552068139067952687508729883632420015185677766880877743348592482390548"
        );

        let h = poseidon.hash(&vec![b3, b4, b0, b0, b0, b0], &is).unwrap();
        assert_eq!(
            h.to_string(),
            "Fr(0x1b1caddfc5ea47e09bb445a7447eb9694b8d1b75a97fff58e884398c6b22825a)" // "12263118664590987767234828103155242843640892839966517009184493198782366909018"
        );

        let h = poseidon.hash(&vec![b1, b2, b3, b4, b5, b6], &is).unwrap();
        assert_eq!(
            h.to_string(),
            "Fr(0x2d1a03850084442813c8ebf094dea47538490a68b05f2239134a4cca2f6302e1)" // "20400040500897583745843009878988256314335038853985262692600694741116813247201"
        );
    }

    #[test]
    fn test_batch_hash() {
        let poseidon = Poseidon::new();

        let inputs: Vec<_> = (0..16).collect::<Vec<u64>>();
        let inp: Vec<Fr> = inputs
            .iter()
            .map(|e| Fr::from_str(&e.to_string()).unwrap())
            .collect();

        let is = Fr::zero();
        let h = poseidon.hash(&inp, &is).unwrap();
        assert_eq!(
            h.to_string(),
            "Fr(0x1b733f2ff41971b23819a16bc8c16bbe13d98173358429fcc12f6f0826407a56)",
        );
    }
}
