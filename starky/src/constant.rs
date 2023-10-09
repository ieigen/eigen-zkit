#![allow(non_snake_case)]
use crate::field_bn128::Fr as Fr_bn128;
use crate::field_bls12381::Fr as Fr_bls12381;
use crate::poseidon_bls12381::load_constants as load_constants_bls12381;
use crate::poseidon_bls12381::Constants as ConstantsBls12381;
use crate::poseidon_bls12381_opt::load_constants as load_constants_bls12381_opt;
use crate::poseidon_bn128::{load_constants, Constants};
use crate::poseidon_bn128_opt::load_constants as load_constants_opt;
use ff::*;
use plonky::field_gl::{exp, Fr};
use plonky::Field;
use std::collections::HashMap;

lazy_static::lazy_static! {
    pub static ref OFFSET_2_64: Fr_bn128 = Fr_bn128::from_str("18446744073709551616").unwrap();
    pub static ref OFFSET_2_128: Fr_bn128 = Fr_bn128::from_str("340282366920938463463374607431768211456").unwrap();
    pub static ref OFFSET_2_192: Fr_bn128 = Fr_bn128::from_str("6277101735386680763835789423207666416102355444464034512896").unwrap();
    pub static ref OFFSET_BLS12381_2_64: Fr_bls12381 = Fr_bls12381::from_str("18446744073709551616").unwrap();
    pub static ref OFFSET_BLS12381_2_128: Fr_bls12381 = Fr_bls12381::from_str("340282366920938463463374607431768211456").unwrap();
    pub static ref OFFSET_BLS12381_2_192: Fr_bls12381 = Fr_bls12381::from_str("6277101735386680763835789423207666416102355444464034512896").unwrap();
    pub static ref CHALLENGE_MAP: HashMap<&'static str, usize> = {
        let mut m = HashMap::new();
        m.insert("u", 0);
        m.insert("defVal", 1);
        m.insert("gamma", 2);
        m.insert("beta", 3);
        m.insert("vc", 4);
        m.insert("vf1", 5);
        m.insert("vf2", 6);
        m.insert("xi", 7);
        m
    };

    pub static ref SHIFT: Fr = Fr::from(49u64);
    pub static ref SHIFT_INV : Fr= SHIFT.clone().inverse().unwrap();
    pub static ref MG: (Vec<Fr>, Vec<Fr>) = {
        let nqr = Fr::from(7u64);
        let rem = 2u64.pow(32) - 1;
        let s = 32usize;
        let mut w = vec![Fr::ZERO; s+1];
        let mut wi = vec![Fr::ZERO; s+1];
        // w[s] = nqr.exp(rem);
        w[s] = exp(nqr,rem);
        wi[s] = w[s].inverse().unwrap();

        for n in (0..s).rev() {
            w[n] = w[n+1] * w[n+1];
            wi[n] = wi[n+1] * wi[n+1];
        }
        (w, wi)
    };

    pub static ref POSEIDON_BN128_CONSTANTS_OPT: Constants = {
        load_constants_opt()
    };
    pub static ref POSEIDON_BN128_CONSTANTS: Constants = {
        load_constants()
    };
    pub static ref POSEIDON_BLS12381_CONSTANTS_OPT: ConstantsBls12381 = {
        load_constants_bls12381_opt()
    };
    pub static ref POSEIDON_BLS12381_CONSTANTS: ConstantsBls12381 = {
        load_constants_bls12381()
    };
    pub static ref POSEIDON_CONSTANTS_OPT: crate::poseidon_opt::Constants = {
        crate::poseidon_opt::load_constants()
    };
}

pub const MIN_OPS_PER_THREAD: usize = 1 << 12;
pub const MAX_OPS_PER_THREAD: usize = 1 << 18;
pub const GLOBAL_L1: &str = "Global.L1";

pub fn get_max_workers() -> usize {
    num_cpus::get() - 1
}
