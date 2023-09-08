#![allow(non_snake_case)]
use crate::compressor12::compressor12_pil;
use crate::compressor12::plonk_setup::PlonkSetup;
use crate::errors::EigenError;
use crate::r1cs2plonk::{r1cs2plonk, PlonkAdd, PlonkGate};
use crate::{pilcom, polsarray};
use plonky::circom_circuit::R1CS;
use plonky::field_gl::Fr as FGL;
use plonky::field_gl::GL;
use plonky::reader::load_r1cs;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

pub type Result<T> = std::result::Result<T, EigenError>;

pub struct Options {
    pub force_bits: usize,
}

// setup phase:
// input: .r1cs
// output: .pil, .const, .exec,
pub fn setup(
    r1cs_file: &String,
    pil_file: &String,
    const_file: &String,
    exec_file: &String,
    force_n_bits: usize,
) -> Result<()> {
    // 0. readR1cs
    let r1cs = load_r1cs::<GL>(r1cs_file);
    let opts = Options {
        force_bits: force_n_bits,
    };

    // 1. plonk setup: generate plonk circuit, the pil file.
    let res = PlonkSetup::plonk_setup(&r1cs, &opts);

    // 2. And write it into pil_file.
    //      todo- this is a little redunction. as the pil_file has been save in plonk_setup
    let mut file = File::create(pil_file).unwrap();
    write!(file, "{}", res.pil_str).unwrap();

    // 3. write const pols file
    let mut file = File::create(const_file).unwrap();
    write!(file, "{}", res.const_pols).unwrap();

    // 4. construct and save ExecFile: plonk additions + sMap -> BigUint64Array
    write_exec_file(exec_file, res.plonk_additions, res.s_map);

    Ok(())
}

// construct and save ExecFile: plonk additions + sMap -> BigUint64Array
fn write_exec_file(exec_file: &String, adds: Vec<Vec<usize>>, s_map: Vec<Vec<usize>>) {
    let adds_len = adds.len();

    let size = 2 + adds_len * 4 + s_map.len() * s_map[0].len();

    let mut buff = Vec::with_capacity(size);
    // const buff = new BigUint64Array(size);

    buff[0] = adds_len;
    buff[1] = s_map.len();

    for i in 0..adds_len {
        buff[2 + i * 4] = adds[i][0];
        buff[2 + i * 4 + 1] = adds[i][1];
        buff[2 + i * 4 + 2] = adds[i][2];
        buff[2 + i * 4 + 3] = adds[i][3];
    }

    for i in 0..s_map.len() {
        for c in 0..12 {
            buff[2 + adds_len * 4 + 12 * i + c] = s_map[c][i];
        }
    }

    let mut file = File::create(exec_file).unwrap();
    write!(file, "{:?}", buff).unwrap();
}
