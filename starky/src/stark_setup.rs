#![allow(non_snake_case, dead_code)]
use crate::errors::Result;
use crate::f3g::F3G;
use crate::fft_p::interpolate;
use crate::polsarray::PolsArray;
use crate::starkinfo::{self, Program, StarkInfo};
use crate::traits::MerkleTree;
use crate::types::{StarkStruct, PIL};
use crate::ElementDigest;
use rayon::prelude::*;
use winter_math::{fields::f64::BaseElement, FieldElement};

#[derive(Default)]
pub struct StarkSetup<M: MerkleTree> {
    pub const_tree: M,
    pub const_root: ElementDigest,
    pub starkinfo: StarkInfo,
    pub program: Program,
}

/// STARK SETUP
///
///  calculate the trace polynomial over extended field, return the new polynomial's coefficient.
impl<M: MerkleTree> StarkSetup<M> {
    pub fn new(
        const_pol: &PolsArray,
        pil: &mut PIL,
        stark_struct: &StarkStruct,
    ) -> Result<StarkSetup<M>> {
        let nBits = stark_struct.nBits;
        let nBitsExt = stark_struct.nBitsExt;
        assert_eq!(const_pol.nPols, pil.nConstants);

        let mut p: Vec<Vec<BaseElement>> = vec![Vec::new(); const_pol.nPols];
        for i in 0..const_pol.nPols {
            for j in 0..const_pol.n {
                p[i].push(const_pol.array[i][j])
            }
        }

        let const_buff = const_pol.write_buff();
        //extend and merkelize
        let mut const_pols_array_e = vec![F3G::ZERO; (1 << nBitsExt) * pil.nConstants];
        let mut const_pols_array_e_be = vec![BaseElement::ZERO; (1 << nBitsExt) * pil.nConstants];

        interpolate(
            &const_buff,
            pil.nConstants,
            nBits,
            &mut const_pols_array_e,
            nBitsExt,
        );

        const_pols_array_e_be
            .par_iter_mut()
            .zip(const_pols_array_e)
            .for_each(|(be_out, f3g_in)| {
                *be_out = f3g_in.to_be();
            });

        let mut const_tree = M::new();
        const_tree.merkelize(
            const_pols_array_e_be,
            const_pol.nPols,
            const_pol.n << (nBitsExt - nBits),
        )?;

        let starkinfo = starkinfo::StarkInfo::new(pil, stark_struct)?;
        Ok(StarkSetup {
            const_root: const_tree.root(),
            const_tree: const_tree,
            starkinfo: starkinfo.0,
            program: starkinfo.1,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use crate::polsarray::{PolKind, PolsArray};
    use crate::stark_setup::StarkSetup;
    use crate::types::{load_json, StarkStruct, PIL};

    use crate::field_bn128::Fr;
    use crate::merklehash_bn128::MerkleTreeBN128;
    use ff::*;

    #[test]
    fn test_stark_setup() {
        let mut pil = load_json::<PIL>("data/fib.pil.json").unwrap();
        let mut const_pol = PolsArray::new(&pil, PolKind::Constant);
        const_pol.load("data/fib.const").unwrap();

        let stark_struct = load_json::<StarkStruct>("data/starkStruct.json").unwrap();
        let setup =
            StarkSetup::<MerkleTreeBN128>::new(&const_pol, &mut pil, &stark_struct).unwrap();
        let root: Fr = setup.const_root.into();

        let expect_root =
            "4658128321472362347225942316135505030498162093259225938328465623672244875764";
        assert_eq!(Fr::from_str(expect_root).unwrap(), root);
    }
}
