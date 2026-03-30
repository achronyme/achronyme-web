//! POST /api/circuit — Compile a standalone circuit, generate R1CS/WTNS/proof artifacts.

use std::collections::HashMap;

use axum::Json;
use base64::Engine;
use serde::{Deserialize, Serialize};

use compiler::r1cs_backend::R1CSCompiler;
use constraints::write_r1cs;
use ir::prove_ir::ProveIrCompiler;
use memory::FieldElement;
use vm::ProveResult;

use crate::error::ApiError;
use crate::sandbox::sandboxed;

const CIRCUIT_TIMEOUT_SECS: u64 = 30;

#[derive(Deserialize)]
pub struct CircuitRequest {
    source: String,
    #[serde(default)]
    inputs: HashMap<String, String>,
    #[serde(default = "default_backend")]
    backend: String,
    #[serde(default)]
    prove: bool,
    #[serde(default)]
    solidity: bool,
}

fn default_backend() -> String {
    "r1cs".to_string()
}

#[derive(Serialize)]
pub struct CircuitResponse {
    success: bool,
    constraints: usize,
    public_inputs: usize,
    private_inputs: usize,
    backend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    r1cs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wtns: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    public_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    solidity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub async fn handler(
    axum::extract::State(_store): axum::extract::State<crate::session::SessionStore>,
    Json(req): Json<CircuitRequest>,
) -> Result<Json<CircuitResponse>, ApiError> {
    let source = req.source;
    let raw_inputs = req.inputs;
    let backend = req.backend;
    let do_prove = req.prove;
    let do_solidity = req.solidity;

    let result = sandboxed(
        move || compile_circuit(&source, &raw_inputs, &backend, do_prove, do_solidity),
        CIRCUIT_TIMEOUT_SECS,
    )
    .await?;

    match result {
        Ok(resp) => Ok(Json(resp)),
        Err(msg) => Err(ApiError::CompileError(msg)),
    }
}

fn compile_circuit(
    source: &str,
    raw_inputs: &HashMap<String, String>,
    backend: &str,
    do_prove: bool,
    do_solidity: bool,
) -> Result<CircuitResponse, String> {
    let source_path = std::path::Path::new("playground.ach");

    // Parse inputs
    let mut inputs = HashMap::new();
    for (name, val_str) in raw_inputs {
        let fe = parse_field_value(name, val_str)?;
        inputs.insert(name.clone(), fe);
    }
    let has_inputs = !inputs.is_empty();

    // Compile circuit to ProveIR
    let prove_ir = ProveIrCompiler::compile_circuit(source, Some(source_path))
        .map_err(|e| format!("{e}"))?;

    let mut program = prove_ir
        .instantiate(&HashMap::new())
        .map_err(|e| format!("{e}"))?;

    ir::passes::optimize(&mut program);

    let n_public = prove_ir.public_inputs.len();
    let n_witness = prove_ir.witness_inputs.len();

    let cache_dir = std::env::temp_dir().join("ach-server-cache");

    match backend {
        "r1cs" => compile_r1cs(
            &program,
            if has_inputs { Some(&inputs) } else { None },
            n_public,
            n_witness,
            do_prove,
            do_solidity,
            &cache_dir,
        ),
        "plonkish" => compile_plonkish(
            &program,
            if has_inputs { Some(&inputs) } else { None },
            n_public,
            n_witness,
            do_prove,
            &cache_dir,
        ),
        _ => Err(format!(
            "unknown backend: {backend} (expected 'r1cs' or 'plonkish')"
        )),
    }
}

fn compile_r1cs(
    program: &ir::IrProgram,
    inputs: Option<&HashMap<String, FieldElement>>,
    n_public: usize,
    n_witness: usize,
    do_prove: bool,
    do_solidity: bool,
    cache_dir: &std::path::Path,
) -> Result<CircuitResponse, String> {
    let b64 = base64::engine::general_purpose::STANDARD;
    let mut r1cs = R1CSCompiler::new();
    let proven = ir::passes::bool_prop::compute_proven_boolean(program);
    r1cs.set_proven_boolean(proven);

    let mut r1cs_b64 = None;
    let mut wtns_b64 = None;
    let mut proof_json = None;
    let mut public_json = None;
    let mut vkey_json = None;
    let mut solidity_src = None;

    if let Some(input_map) = inputs {
        // Compile with witness
        let witness = r1cs
            .compile_ir_with_witness(program, input_map)
            .map_err(|e| format!("{e}"))?;

        r1cs.cs
            .verify(&witness)
            .map_err(|e| format!("constraint verification failed: {e}"))?;

        // Serialize R1CS
        let r1cs_data = write_r1cs(&r1cs.cs);
        r1cs_b64 = Some(b64.encode(&r1cs_data));

        // Serialize WTNS
        let wtns_data = constraints::write_wtns(&witness);
        wtns_b64 = Some(b64.encode(&wtns_data));

        // Generate proof if requested
        if do_prove {
            let result = proving::groth16::generate_proof(&r1cs.cs, &witness, cache_dir)
                .map_err(|e| format!("proof generation failed: {e}"))?;

            if let ProveResult::Proof {
                proof_json: pj,
                public_json: pub_j,
                vkey_json: vk_j,
            } = result
            {
                proof_json = Some(pj);
                public_json = Some(pub_j);
                vkey_json = Some(vk_j);
            }
        }
    } else {
        // Compile constraints only (no witness)
        r1cs.compile_ir(program)
            .map_err(|e| format!("{e}"))?;

        let r1cs_data = write_r1cs(&r1cs.cs);
        r1cs_b64 = Some(b64.encode(&r1cs_data));
    }

    // Generate Solidity verifier if requested
    if do_solidity {
        let vk = proving::groth16::setup_vk_only(&r1cs.cs, cache_dir)
            .map_err(|e| format!("Groth16 setup failed: {e}"))?;
        solidity_src = Some(proving::solidity::generate_solidity_verifier(&vk));
    }

    Ok(CircuitResponse {
        success: true,
        constraints: r1cs.cs.num_constraints(),
        public_inputs: n_public,
        private_inputs: n_witness,
        backend: "r1cs".into(),
        r1cs: r1cs_b64,
        wtns: wtns_b64,
        proof: proof_json,
        public_json,
        vkey: vkey_json,
        solidity: solidity_src,
        error: None,
    })
}

fn compile_plonkish(
    program: &ir::IrProgram,
    inputs: Option<&HashMap<String, FieldElement>>,
    n_public: usize,
    n_witness: usize,
    do_prove: bool,
    cache_dir: &std::path::Path,
) -> Result<CircuitResponse, String> {
    let mut compiler = compiler::plonkish_backend::PlonkishCompiler::new();
    let proven = ir::passes::bool_prop::compute_proven_boolean(program);
    compiler.set_proven_boolean(proven);

    let mut proof_json = None;
    let mut public_json = None;
    let mut vkey_json = None;
    let mut n_constraints = 0;

    if let Some(input_map) = inputs {
        compiler
            .compile_ir_with_witness(program, input_map)
            .map_err(|e| format!("{e}"))?;

        n_constraints = compiler.num_circuit_rows();

        compiler
            .system
            .verify()
            .map_err(|e| format!("plonkish verification failed: {e}"))?;

        if do_prove {
            let result = proving::halo2_proof::generate_plonkish_proof(compiler, cache_dir)
                .map_err(|e| format!("proof generation failed: {e}"))?;

            if let ProveResult::Proof {
                proof_json: pj,
                public_json: pub_j,
                vkey_json: vk_j,
            } = result
            {
                proof_json = Some(pj);
                public_json = Some(pub_j);
                vkey_json = Some(vk_j);
            }
        }
    } else {
        compiler
            .compile_ir(program)
            .map_err(|e| format!("{e}"))?;

        n_constraints = compiler.num_circuit_rows();
    }

    Ok(CircuitResponse {
        success: true,
        constraints: n_constraints,
        public_inputs: n_public,
        private_inputs: n_witness,
        backend: "plonkish".into(),
        r1cs: None,
        wtns: None,
        proof: proof_json,
        public_json,
        vkey: vkey_json,
        solidity: None,
        error: None,
    })
}

fn parse_field_value(name: &str, val_str: &str) -> Result<FieldElement, String> {
    let val_str = val_str.trim();
    if val_str.starts_with("0x") || val_str.starts_with("0X") {
        FieldElement::from_hex_str(val_str)
            .ok_or_else(|| format!("invalid hex value for `{name}`: {val_str:?}"))
    } else if let Some(digits) = val_str.strip_prefix('-') {
        let abs = FieldElement::from_decimal_str(digits)
            .ok_or_else(|| format!("invalid value for `{name}`: {val_str:?}"))?;
        Ok(abs.neg())
    } else {
        FieldElement::from_decimal_str(val_str)
            .ok_or_else(|| format!("invalid value for `{name}`: {val_str:?}"))
    }
}
