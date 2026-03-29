//! Server-side prove handler — simplified version of cli/src/prove_handler.rs.
//!
//! No CLI dependencies (no Styler, ErrorFormat, colored). Just compiles
//! ProveIR, verifies constraints, and generates Groth16 proofs.

use std::collections::HashMap;
use std::path::PathBuf;

use compiler::r1cs_backend::R1CSCompiler;
use memory::FieldElement;
use vm::{ProveError, ProveHandler, ProveResult, VerifyHandler};

pub struct ServerProveHandler {
    cache_dir: PathBuf,
}

impl ServerProveHandler {
    pub fn new() -> Self {
        let cache_dir = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".achronyme").join("cache"))
            .unwrap_or_else(|_| std::env::temp_dir().join("achronyme-cache"));
        Self { cache_dir }
    }
}

impl ProveHandler for ServerProveHandler {
    fn execute_prove_ir(
        &self,
        prove_ir_bytes: &[u8],
        scope_values: &HashMap<String, FieldElement>,
    ) -> Result<ProveResult, ProveError> {
        // 1. Deserialize ProveIR
        let prove_ir = ir::prove_ir::ProveIR::from_bytes(prove_ir_bytes)
            .map_err(|e| ProveError::IrLowering(format!("ProveIR deserialization: {e}")))?;

        // 2. Instantiate with scope values
        let mut program = prove_ir
            .instantiate(scope_values)
            .map_err(|e| ProveError::IrLowering(format!("{e}")))?;

        // 3. Optimize
        ir::passes::optimize(&mut program);

        // 4. Build input map
        let mut inputs = HashMap::new();
        for input in prove_ir
            .public_inputs
            .iter()
            .chain(prove_ir.witness_inputs.iter())
        {
            match &input.array_size {
                Some(ir::prove_ir::ArraySize::Literal(n)) => {
                    for i in 0..*n {
                        let elem_name = format!("{}_{i}", input.name);
                        let fe = scope_values.get(&elem_name).ok_or_else(|| {
                            ProveError::IrLowering(format!(
                                "variable `{elem_name}` not found in scope"
                            ))
                        })?;
                        inputs.insert(elem_name, *fe);
                    }
                }
                None => {
                    let fe = scope_values.get(&input.name).ok_or_else(|| {
                        ProveError::IrLowering(format!(
                            "variable `{}` not found in scope",
                            input.name
                        ))
                    })?;
                    inputs.insert(input.name.clone(), *fe);
                }
                Some(ir::prove_ir::ArraySize::Capture(_)) => {}
            }
        }
        for cap in &prove_ir.captures {
            let fe = scope_values.get(&cap.name).ok_or_else(|| {
                ProveError::IrLowering(format!("capture `{}` not found in scope", cap.name))
            })?;
            inputs.insert(cap.name.clone(), *fe);
        }

        // 5. Compile to R1CS
        let mut r1cs = R1CSCompiler::new();
        let proven = ir::passes::bool_prop::compute_proven_boolean(&program);
        r1cs.set_proven_boolean(proven);
        let witness = r1cs
            .compile_ir_with_witness(&program, &inputs)
            .map_err(|e| ProveError::Compilation(format!("{e}")))?;

        // 6. Verify constraints
        r1cs.cs
            .verify(&witness)
            .map_err(|e| ProveError::Verification(format!("{e}")))?;

        // 7. Generate Groth16 proof
        proving::groth16::generate_proof(&r1cs.cs, &witness, &self.cache_dir)
            .map_err(ProveError::ProofGeneration)
    }
}

impl VerifyHandler for ServerProveHandler {
    fn verify_proof(&self, proof: &memory::ProofObject) -> Result<bool, String> {
        proving::groth16::verify_proof_from_json(
            &proof.proof_json,
            &proof.public_json,
            &proof.vkey_json,
        )
    }
}
