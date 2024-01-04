#![allow(unused_macros)]
#![allow(dead_code)]
use std::io::BufWriter;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;

macro_rules! local_fill {
    ($left:expr, $right:expr, $fun:expr) => {
        if let Some(right) = $right {
            $left = $fun(right.0)
        }
    };
    ($left:expr, $right:expr) => {
        if let Some(right) = $right {
            $left = Address::from(right.as_fixed_bytes())
        }
    };
}

struct FlushWriter {
    writer: Arc<Mutex<BufWriter<std::fs::File>>>,
}

impl FlushWriter {
    fn new(writer: Arc<Mutex<BufWriter<std::fs::File>>>) -> Self {
        Self { writer }
    }
}

impl Write for FlushWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.lock().unwrap().flush()
    }
}

#[cfg(test)]
mod tests {

    use backend::BackendType;
    use compiler::pipeline::{Pipeline, Stage};

    use mktemp::Temp;
    use number::GoldilocksField;

    use riscv::{
        compile_rust,
        continuations::{rust_continuations, rust_continuations_dry_run},
        CoProcessors,
    };

    use std::path::PathBuf;

    static BYTECODE: &str = "61029a60005260206000f3";

    #[test]
    #[ignore = "Too slow"]
    fn test_revm_prove_single_contract() {
        env_logger::try_init().unwrap_or_default();

        type F = GoldilocksField;
        let temp_dir = Temp::new_dir().unwrap();
        log::info!("Write to {:?}", temp_dir);
        let case = "vm/evm";
        let coprocessors = CoProcessors::base().with_poseidon();
        // Compile REVM to powdr asm
        let powdr_asm = compile_rust(case, &temp_dir, true, &coprocessors, true).unwrap();

        let bytes = hex::decode(BYTECODE).unwrap();

        let length: GoldilocksField = (bytes.len() as u64).into();
        let mut bytecode: Vec<GoldilocksField> = vec![length];
        bytecode.extend(bytes.into_iter().map(|x| GoldilocksField::from(x as u64)));

        // Load the powdr asm
        let pipeline_factory = || {
            Pipeline::default()
                .from_asm_string(powdr_asm.1.clone(), Some(PathBuf::from(case)))
                .with_prover_inputs(bytecode.clone())
        };

        // Execute the evm and generate inputs for segment
        let bootloader_inputs =
            rust_continuations_dry_run::<GoldilocksField>(pipeline_factory(), bytecode.clone());

        // Build the wtns and proof
        let prove_with = Some(BackendType::EStark);
        let generate_witness_and_prove_maybe =
            |mut pipeline: Pipeline<F>| -> Result<(), Vec<String>> {
                pipeline.advance_to(Stage::GeneratedWitness).unwrap();
                prove_with.map(|backend| pipeline.with_backend(backend).proof().unwrap());
                Ok(())
            };

        rust_continuations(
            pipeline_factory,
            generate_witness_and_prove_maybe,
            bootloader_inputs,
        )
        .unwrap();
    }
}
