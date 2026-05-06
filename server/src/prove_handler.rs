//! Server-side prove handler — supports R1CS (Groth16) and Plonkish (Halo2).
//!
//! Simplified version of cli/src/prove_handler.rs without CLI dependencies.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use akron::{ProveError, ProveHandler, ProveResult, VerifyHandler};
use memory::FieldElement;
use zkc::plonkish_backend::PlonkishCompiler;
use zkc::r1cs_backend::R1CSCompiler;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProveBackend {
    R1cs,
    Plonkish,
}

/// Captured proof artifact from a prove {} block execution.
#[derive(Clone, serde::Serialize)]
pub struct CapturedProof {
    pub name: String,
    pub constraints: usize,
    pub backend: String,
    pub proof_json: String,
    pub public_json: String,
    pub vkey_json: String,
}

pub struct ServerProveHandler {
    cache_dir: PathBuf,
    pub backend: ProveBackend,
    /// Proof artifacts captured during VM execution.
    pub captured: RefCell<Vec<CapturedProof>>,
}

impl ServerProveHandler {
    pub fn new(backend: ProveBackend) -> Self {
        // /tmp/ach-server-cache: shared with routes/prove.rs and
        // routes/circuit.rs so warm Groth16 keys are reused across
        // /api/run + /api/prove + /api/circuit. $HOME is unsuitable
        // here — systemd's ProtectHome=true masks /home from the
        // service, and the cache is ephemeral anyway.
        let cache_dir = std::env::temp_dir().join("ach-server-cache");
        Self {
            cache_dir,
            backend,
            captured: RefCell::new(Vec::new()),
        }
    }

    /// Drain captured proof artifacts.
    pub fn drain_captured(&self) -> Vec<CapturedProof> {
        self.captured.borrow_mut().drain(..).collect()
    }
}

impl ProveHandler for ServerProveHandler {
    fn execute_prove_ir(
        &self,
        prove_ir_bytes: &[u8],
        scope_values: &HashMap<String, FieldElement>,
    ) -> Result<ProveResult, ProveError> {
        // 1. Deserialize ProveIR
        let (prove_ir, _prime_id) = ir_forge::ProveIR::from_bytes(prove_ir_bytes)
            .map_err(|e| ProveError::IrLowering(format!("ProveIR deserialization: {e}")))?;

        // 2. Instantiate with scope values
        let mut program = prove_ir
            .instantiate_lysis(scope_values)
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
                Some(ir_forge::ArraySize::Literal(n)) => {
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
                Some(ir_forge::ArraySize::Capture(_)) => {}
            }
        }
        for cap in &prove_ir.captures {
            let fe = scope_values.get(&cap.name).ok_or_else(|| {
                ProveError::IrLowering(format!("capture `{}` not found in scope", cap.name))
            })?;
            inputs.insert(cap.name.clone(), *fe);
        }

        // Imported circom templates emit `WitnessHint` nodes for `<--`
        // expressions on internal signals (e.g. `Num2Bits` bit
        // decompositions, `Sha256Compression` round outputs). The IR
        // evaluator treats them as `Input { Witness }` wires, so their
        // values must be computed off-circuit and supplied in the
        // inputs map before R1CS / Plonkish compilation. The hint
        // walker is the same one `cli/src/prove_handler.rs` and `ach
        // circom` use; existing inputs always win over hint-computed
        // ones.
        let hint_env =
            circom::witness::compute_witness_hints::<memory::Bn254Fr>(&prove_ir, &inputs).map_err(
                |e| ProveError::IrLowering(format!("circom witness hint computation failed: {e}")),
            )?;
        for (name, fe) in hint_env {
            inputs.entry(name).or_insert(fe);
        }

        match self.backend {
            ProveBackend::R1cs => self.prove_r1cs(&program, &inputs),
            ProveBackend::Plonkish => self.prove_plonkish(&program, &inputs),
        }
    }
}

impl ServerProveHandler {
    fn prove_r1cs(
        &self,
        program: &ir::IrProgram,
        inputs: &HashMap<String, FieldElement>,
    ) -> Result<ProveResult, ProveError> {
        let mut r1cs = R1CSCompiler::new();
        let proven = ir::passes::bool_prop::compute_proven_boolean(program);
        r1cs.set_proven_boolean(proven);
        let witness = r1cs
            .compile_ir_with_witness(program, inputs)
            .map_err(|e| ProveError::Compilation(format!("{e}")))?;

        r1cs.cs
            .verify(&witness)
            .map_err(|e| ProveError::Verification(format!("{e}")))?;

        let n_constraints = r1cs.cs.num_constraints();

        let result = proving::groth16_bn254::generate_proof(&r1cs.cs, &witness, &self.cache_dir)
            .map_err(ProveError::ProofGeneration)?;

        // Capture proof artifacts
        if let ProveResult::Proof {
            ref proof_json,
            ref public_json,
            ref vkey_json,
        } = result
        {
            self.captured.borrow_mut().push(CapturedProof {
                name: "circuit".into(),
                constraints: n_constraints,
                backend: "r1cs".into(),
                proof_json: proof_json.clone(),
                public_json: public_json.clone(),
                vkey_json: vkey_json.clone(),
            });
        }

        Ok(result)
    }

    fn prove_plonkish(
        &self,
        program: &ir::IrProgram,
        inputs: &HashMap<String, FieldElement>,
    ) -> Result<ProveResult, ProveError> {
        let mut compiler = PlonkishCompiler::new();
        let proven = ir::passes::bool_prop::compute_proven_boolean(program);
        compiler.set_proven_boolean(proven);
        compiler
            .compile_ir_with_witness(program, inputs)
            .map_err(|e| ProveError::Compilation(format!("{e}")))?;

        let n_rows = compiler.num_circuit_rows();

        compiler
            .system
            .verify()
            .map_err(|e| ProveError::Verification(format!("plonkish: {e}")))?;

        let result = proving::halo2_proof::generate_plonkish_proof(compiler, &self.cache_dir)
            .map_err(ProveError::ProofGeneration)?;

        // Capture proof artifacts
        if let ProveResult::Proof {
            ref proof_json,
            ref public_json,
            ref vkey_json,
        } = result
        {
            self.captured.borrow_mut().push(CapturedProof {
                name: "circuit".into(),
                constraints: n_rows,
                backend: "plonkish".into(),
                proof_json: proof_json.clone(),
                public_json: public_json.clone(),
                vkey_json: vkey_json.clone(),
            });
        }

        Ok(result)
    }
}

impl VerifyHandler for ServerProveHandler {
    fn verify_proof(&self, proof: &memory::ProofObject) -> Result<bool, String> {
        proving::groth16_bn254::verify_proof_from_json(
            &proof.proof_json,
            &proof.public_json,
            &proof.vkey_json,
        )
    }
}
