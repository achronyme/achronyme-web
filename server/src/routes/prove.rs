//! POST /api/prove — Generate a ZK proof from a circuit.

use std::collections::HashMap;

use axum::Json;
use serde::{Deserialize, Serialize};

use compiler::r1cs_backend::R1CSCompiler;
use ir::prove_ir::ProveIrCompiler;
use memory::FieldElement;
use vm::ProveResult;

use crate::error::ApiError;
use crate::sandbox::sandboxed;

const PROVE_TIMEOUT_SECS: u64 = 30;

#[derive(Deserialize)]
pub struct ProveRequest {
    source: String,
    inputs: HashMap<String, String>,
    #[serde(default = "default_backend")]
    backend: String,
}

fn default_backend() -> String {
    "r1cs".to_string()
}

#[derive(Serialize)]
pub struct ProveResponse {
    success: bool,
    proof: Option<String>,
    public_inputs: Option<String>,
    vkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    constraints: usize,
}

pub async fn handler(
    axum::extract::State(_store): axum::extract::State<crate::session::SessionStore>,
    Json(req): Json<ProveRequest>,
) -> Result<Json<ProveResponse>, ApiError> {
    if req.inputs.is_empty() {
        return Err(ApiError::BadRequest("inputs are required for proving".into()));
    }

    let source = req.source;
    let raw_inputs = req.inputs;
    let backend = req.backend;

    let result = sandboxed(
        move || prove_circuit(&source, &raw_inputs, &backend),
        PROVE_TIMEOUT_SECS,
    )
    .await?;

    match result {
        Ok(resp) => Ok(Json(resp)),
        Err(msg) => Err(ApiError::CompileError(msg)),
    }
}

fn prove_circuit(
    source: &str,
    raw_inputs: &HashMap<String, String>,
    backend: &str,
) -> Result<ProveResponse, String> {
    let source_path = std::path::Path::new("playground.ach");

    // Parse inputs
    let mut inputs = HashMap::new();
    for (name, val_str) in raw_inputs {
        let fe = parse_field_value(name, val_str)?;
        inputs.insert(name.clone(), fe);
    }

    // Compile circuit
    let prove_ir =
        ProveIrCompiler::<memory::Bn254Fr>::compile_circuit(source, Some(source_path))
            .map_err(|e| format!("{e}"))?;

    let mut program = prove_ir
        .instantiate(&HashMap::new())
        .map_err(|e| format!("{e}"))?;

    ir::passes::optimize(&mut program);

    // Use a temp dir for proof key caching
    let cache_dir = std::env::temp_dir().join("ach-server-cache");

    match backend {
        "r1cs" => prove_r1cs(&program, &inputs, &cache_dir),
        "plonkish" => prove_plonkish(&program, &inputs, &cache_dir),
        _ => Err(format!("unknown backend: {backend} (expected 'r1cs' or 'plonkish')")),
    }
}

fn prove_r1cs(
    program: &ir::IrProgram,
    inputs: &HashMap<String, FieldElement>,
    cache_dir: &std::path::Path,
) -> Result<ProveResponse, String> {
    let mut r1cs = R1CSCompiler::new();
    let proven = ir::passes::bool_prop::compute_proven_boolean(program);
    r1cs.set_proven_boolean(proven);

    let witness = r1cs
        .compile_ir_with_witness(program, inputs)
        .map_err(|e| format!("{e}"))?;

    r1cs.cs
        .verify(&witness)
        .map_err(|e| format!("constraint verification failed: {e}"))?;

    let n_constraints = r1cs.cs.num_constraints();

    let result = proving::groth16_bn254::generate_proof(&r1cs.cs, &witness, cache_dir)
        .map_err(|e| format!("proof generation failed: {e}"))?;

    match result {
        ProveResult::Proof {
            proof_json,
            public_json,
            vkey_json,
        } => Ok(ProveResponse {
            success: true,
            proof: Some(proof_json),
            public_inputs: Some(public_json),
            vkey: Some(vkey_json),
            error: None,
            constraints: n_constraints,
        }),
        ProveResult::VerifiedOnly => Ok(ProveResponse {
            success: true,
            proof: None,
            public_inputs: None,
            vkey: None,
            error: None,
            constraints: n_constraints,
        }),
    }
}

fn prove_plonkish(
    program: &ir::IrProgram,
    inputs: &HashMap<String, FieldElement>,
    cache_dir: &std::path::Path,
) -> Result<ProveResponse, String> {
    let mut compiler = compiler::plonkish_backend::PlonkishCompiler::new();
    let proven = ir::passes::bool_prop::compute_proven_boolean(program);
    compiler.set_proven_boolean(proven);

    compiler
        .compile_ir_with_witness(program, inputs)
        .map_err(|e| format!("{e}"))?;

    let n_rows = compiler.num_circuit_rows();

    compiler
        .system
        .verify()
        .map_err(|e| format!("plonkish verification failed: {e}"))?;

    let result = proving::halo2_proof::generate_plonkish_proof(compiler, cache_dir)
        .map_err(|e| format!("proof generation failed: {e}"))?;

    match result {
        ProveResult::Proof {
            proof_json,
            public_json,
            vkey_json,
        } => Ok(ProveResponse {
            success: true,
            proof: Some(proof_json),
            public_inputs: Some(public_json),
            vkey: Some(vkey_json),
            error: None,
            constraints: n_rows,
        }),
        ProveResult::VerifiedOnly => Ok(ProveResponse {
            success: true,
            proof: None,
            public_inputs: None,
            vkey: None,
            error: None,
            constraints: n_rows,
        }),
    }
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
