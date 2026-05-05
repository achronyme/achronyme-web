//! POST /api/prove — Generate a ZK proof from a circuit.
//!
//! Three modes, mirroring `/api/circuit`:
//! - Single-source `.ach` (back-compat): request body carries `source`.
//! - Workspace `.ach`: `X-Ach-Session` header, `[project] entry = "...ach"`.
//! - Workspace `.circom`: `X-Ach-Session` header, `[project] entry = "...circom"`,
//!   `[circom] libs = [...]` resolves to circomlib include paths.

use std::collections::HashMap;
use std::path::PathBuf;

use axum::Json;
use serde::{Deserialize, Serialize};

use akron::ProveResult;
use ir_forge::ProveIrCompiler;
use memory::FieldElement;
use zkc::r1cs_backend::R1CSCompiler;

use crate::error::ApiError;
use crate::sandbox::sandboxed;

const PROVE_TIMEOUT_SECS: u64 = 30;

#[derive(Deserialize)]
pub struct ProveRequest {
    #[serde(default)]
    source: Option<String>,
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
    /// Structured diagnostics returned when the circom front-end rejects
    /// the source. Mirrors `/api/circuit` so the playground client can
    /// render per-line squigglies instead of a `\n`-joined blob.
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostics: Option<Vec<crate::pipeline::DiagnosticInfo>>,
}

pub async fn handler(
    axum::extract::State(store): axum::extract::State<crate::session::SessionStore>,
    headers: axum::http::HeaderMap,
    Json(req): Json<ProveRequest>,
) -> Result<Json<ProveResponse>, ApiError> {
    if req.inputs.is_empty() {
        return Err(ApiError::BadRequest(
            "inputs are required for proving".into(),
        ));
    }

    let raw_inputs = req.inputs;
    let backend = req.backend;

    if let Some(session_val) = headers.get("X-Ach-Session") {
        let id: uuid::Uuid = session_val
            .to_str()
            .map_err(|_| ApiError::BadRequest("invalid session header".into()))?
            .parse()
            .map_err(|_| ApiError::BadRequest("invalid session id".into()))?;

        let workspace = store.get_workspace(id).map_err(ApiError::BadRequest)?;
        let config =
            crate::workspace::load_workspace_config(&workspace).map_err(ApiError::BadRequest)?;

        let is_circom = config
            .entry
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.eq_ignore_ascii_case("circom"))
            .unwrap_or(false);

        if is_circom {
            let entry = config.entry.clone();
            let libs = config.circom_libs.clone();
            let result = sandboxed(
                move || prove_circuit_circom(&entry, &libs, &raw_inputs, &backend),
                PROVE_TIMEOUT_SECS,
            )
            .await?;
            return match result {
                Ok(resp) => Ok(Json(resp)),
                Err(msg) => Err(ApiError::CompileError(msg)),
            };
        }

        let source = std::fs::read_to_string(&config.entry).map_err(|e| {
            ApiError::BadRequest(format!("cannot read entry {}: {e}", config.entry.display()))
        })?;
        let result = sandboxed(
            move || prove_circuit(&source, &raw_inputs, &backend),
            PROVE_TIMEOUT_SECS,
        )
        .await?;
        return match result {
            Ok(resp) => Ok(Json(resp)),
            Err(msg) => Err(ApiError::CompileError(msg)),
        };
    }

    let source = req
        .source
        .ok_or_else(|| ApiError::BadRequest("source is required".into()))?;
    if source.is_empty() {
        return Err(ApiError::BadRequest("source is empty".into()));
    }

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
    let prove_ir = ProveIrCompiler::<memory::Bn254Fr>::compile_circuit(source, Some(source_path))
        .map_err(|e| format!("{e}"))?;

    let mut program = prove_ir
        .instantiate_lysis(&HashMap::new())
        .map_err(|e| format!("{e}"))?;

    ir::passes::optimize(&mut program);

    // Use a temp dir for proof key caching
    let cache_dir = std::env::temp_dir().join("ach-server-cache");

    match backend {
        "r1cs" => prove_r1cs(&program, &inputs, &cache_dir),
        "plonkish" => prove_plonkish(&program, &inputs, &cache_dir),
        _ => Err(format!(
            "unknown backend: {backend} (expected 'r1cs' or 'plonkish')"
        )),
    }
}

fn prove_circuit_circom(
    entry: &std::path::Path,
    libs: &[PathBuf],
    raw_inputs: &HashMap<String, String>,
    backend: &str,
) -> Result<ProveResponse, String> {
    let mut inputs = HashMap::new();
    for (name, val_str) in raw_inputs {
        let fe = parse_field_value(name, val_str)?;
        inputs.insert(name.clone(), fe);
    }

    let compiled = match crate::circom_pipeline::compile_circom(entry, libs) {
        Ok(compiled) => compiled,
        Err(diags) => {
            // Mirror `/api/circuit`: surface every diagnostic with its
            // own line + severity so the playground can render
            // individual squigglies. Returning Ok(success=false) keeps
            // the diagnostics list serialised; the unified ApiError
            // path collapses to a single string.
            return Ok(ProveResponse {
                success: false,
                proof: None,
                public_inputs: None,
                vkey: None,
                error: Some("circom compilation failed".into()),
                constraints: 0,
                diagnostics: Some(crate::circom_pipeline::diagnostics_to_pipeline_format(
                    &diags,
                )),
            });
        }
    };

    let mut program = crate::circom_pipeline::instantiate_circom(&compiled)?;
    ir::passes::optimize(&mut program);

    let merged_inputs = crate::circom_pipeline::merge_circom_witness(&compiled, &inputs)?;

    let cache_dir = std::env::temp_dir().join("ach-server-cache");

    match backend {
        "r1cs" => prove_r1cs(&program, &merged_inputs, &cache_dir),
        "plonkish" => prove_plonkish(&program, &merged_inputs, &cache_dir),
        _ => Err(format!(
            "unknown backend: {backend} (expected 'r1cs' or 'plonkish')"
        )),
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
            diagnostics: None,
        }),
        ProveResult::VerifiedOnly => Ok(ProveResponse {
            success: true,
            proof: None,
            public_inputs: None,
            vkey: None,
            error: None,
            constraints: n_constraints,
            diagnostics: None,
        }),
    }
}

fn prove_plonkish(
    program: &ir::IrProgram,
    inputs: &HashMap<String, FieldElement>,
    cache_dir: &std::path::Path,
) -> Result<ProveResponse, String> {
    let mut compiler = zkc::plonkish_backend::PlonkishCompiler::new();
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
            diagnostics: None,
        }),
        ProveResult::VerifiedOnly => Ok(ProveResponse {
            success: true,
            proof: None,
            public_inputs: None,
            vkey: None,
            error: None,
            constraints: n_rows,
            diagnostics: None,
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
