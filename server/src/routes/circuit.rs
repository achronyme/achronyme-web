//! POST /api/circuit — Compile a standalone circuit, generate R1CS/WTNS/proof artifacts.
//!
//! Three modes:
//! - Single-source `.ach` (back-compat): request body carries `source`.
//! - Workspace `.ach`: `X-Ach-Session` header, `[project] entry = "...ach"`.
//! - Workspace `.circom`: `X-Ach-Session` header, `[project] entry = "...circom"`,
//!   `[circom] libs = [...]` resolves to circomlib include paths.

use std::collections::HashMap;
use std::path::PathBuf;

use axum::Json;
use base64::Engine;
use serde::{Deserialize, Serialize};

use akron::ProveResult;
use constraints::write_r1cs;
use ir_forge::ProveIrCompiler;
use memory::field::PrimeId;
use memory::FieldElement;
use zkc::r1cs_backend::R1CSCompiler;

use crate::error::ApiError;
use crate::sandbox::sandboxed;

const CIRCUIT_TIMEOUT_SECS: u64 = 30;

#[derive(Deserialize)]
pub struct CircuitRequest {
    #[serde(default)]
    source: Option<String>,
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
    proof: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    public_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    solidity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    /// Multiple structured diagnostics when the circom front-end rejects
    /// the source. Keeps per-line spans so the client can render
    /// individual squigglies instead of a `\n`-joined blob.
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostics: Option<Vec<crate::pipeline::DiagnosticInfo>>,
}

pub async fn handler(
    axum::extract::State(store): axum::extract::State<crate::session::SessionStore>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CircuitRequest>,
) -> Result<Json<CircuitResponse>, ApiError> {
    let raw_inputs = req.inputs;
    let backend = req.backend;
    let do_prove = req.prove;
    let do_solidity = req.solidity;

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
                move || {
                    compile_circuit_circom(
                        &entry,
                        &libs,
                        &raw_inputs,
                        &backend,
                        do_prove,
                        do_solidity,
                    )
                },
                CIRCUIT_TIMEOUT_SECS,
            )
            .await?;
            return match result {
                Ok(resp) => Ok(Json(resp)),
                Err(msg) => Err(ApiError::CompileError(msg)),
            };
        }

        // .ach workspace mode
        let source = std::fs::read_to_string(&config.entry).map_err(|e| {
            ApiError::BadRequest(format!("cannot read entry {}: {e}", config.entry.display()))
        })?;
        let result = sandboxed(
            move || compile_circuit_ach(&source, &raw_inputs, &backend, do_prove, do_solidity),
            CIRCUIT_TIMEOUT_SECS,
        )
        .await?;
        return match result {
            Ok(resp) => Ok(Json(resp)),
            Err(msg) => Err(ApiError::CompileError(msg)),
        };
    }

    // Single-source mode: .ach only.
    let source = req
        .source
        .ok_or_else(|| ApiError::BadRequest("source is required".into()))?;
    if source.is_empty() {
        return Err(ApiError::BadRequest("source is empty".into()));
    }
    let result = sandboxed(
        move || compile_circuit_ach(&source, &raw_inputs, &backend, do_prove, do_solidity),
        CIRCUIT_TIMEOUT_SECS,
    )
    .await?;

    match result {
        Ok(resp) => Ok(Json(resp)),
        Err(msg) => Err(ApiError::CompileError(msg)),
    }
}

fn compile_circuit_ach(
    source: &str,
    raw_inputs: &HashMap<String, String>,
    backend: &str,
    do_prove: bool,
    do_solidity: bool,
) -> Result<CircuitResponse, String> {
    let source_path = std::path::Path::new("playground.ach");

    let mut inputs = HashMap::new();
    for (name, val_str) in raw_inputs {
        let fe = parse_field_value(name, val_str)?;
        inputs.insert(name.clone(), fe);
    }
    let has_inputs = !inputs.is_empty();

    let prove_ir = ProveIrCompiler::<memory::Bn254Fr>::compile_circuit(source, Some(source_path))
        .map_err(|e| format!("{e}"))?;

    let n_public = prove_ir.public_inputs.len();
    let n_witness = prove_ir.witness_inputs.len();

    let mut program = prove_ir
        .instantiate_lysis(&HashMap::new())
        .map_err(|e| format!("{e}"))?;

    ir::passes::optimize(&mut program);

    run_backend(
        &program,
        if has_inputs { Some(&inputs) } else { None },
        n_public,
        n_witness,
        backend,
        do_prove,
        do_solidity,
    )
}

fn compile_circuit_circom(
    entry: &std::path::Path,
    libs: &[PathBuf],
    raw_inputs: &HashMap<String, String>,
    backend: &str,
    do_prove: bool,
    do_solidity: bool,
) -> Result<CircuitResponse, String> {
    let mut inputs = HashMap::new();
    for (name, val_str) in raw_inputs {
        let fe = parse_field_value(name, val_str)?;
        inputs.insert(name.clone(), fe);
    }

    let compiled = match crate::circom_pipeline::compile_circom(entry, libs) {
        Ok(compiled) => compiled,
        Err(diags) => {
            // Surface every diagnostic with its own line + severity so
            // the playground can render them individually. The handler
            // returns `Ok(success=false)` rather than `Err` so the
            // diagnostics list survives serialization (the unified
            // ApiError path collapses to a single string).
            return Ok(CircuitResponse {
                success: false,
                constraints: 0,
                public_inputs: 0,
                private_inputs: 0,
                backend: backend.to_string(),
                r1cs: None,
                proof: None,
                public_json: None,
                vkey: None,
                solidity: None,
                error: Some("circom compilation failed".into()),
                diagnostics: Some(crate::circom_pipeline::diagnostics_to_pipeline_format(
                    &diags,
                )),
            });
        }
    };

    let n_public = compiled.prove_ir.public_inputs.len();
    let n_witness = compiled.prove_ir.witness_inputs.len();

    let mut program = crate::circom_pipeline::instantiate_circom(&compiled)?;
    ir::passes::optimize(&mut program);

    // Witness hints: only computed when the user supplied inputs (i.e.
    // they want a witness/proof, not just constraints). The hint
    // computation off-circuit-evaluates `<--` expressions, which can be
    // expensive on heavy templates.
    let merged_inputs;
    let inputs_ref = if !inputs.is_empty() {
        merged_inputs = crate::circom_pipeline::merge_circom_witness(&compiled, &inputs)?;
        Some(&merged_inputs)
    } else {
        None
    };

    run_backend(
        &program,
        inputs_ref,
        n_public,
        n_witness,
        backend,
        do_prove,
        do_solidity,
    )
}

fn run_backend(
    program: &ir::IrProgram,
    inputs: Option<&HashMap<String, FieldElement>>,
    n_public: usize,
    n_witness: usize,
    backend: &str,
    do_prove: bool,
    do_solidity: bool,
) -> Result<CircuitResponse, String> {
    let cache_dir = std::env::temp_dir().join("ach-server-cache");

    match backend {
        "r1cs" => compile_r1cs(
            program,
            inputs,
            n_public,
            n_witness,
            do_prove,
            do_solidity,
            &cache_dir,
        ),
        "plonkish" => compile_plonkish(program, inputs, n_public, n_witness, do_prove, &cache_dir),
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

    let r1cs_b64;
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
        let r1cs_data = write_r1cs(&r1cs.cs, PrimeId::Bn254);
        r1cs_b64 = Some(b64.encode(&r1cs_data));

        // Generate proof if requested
        if do_prove {
            let result = proving::groth16_bn254::generate_proof(&r1cs.cs, &witness, cache_dir)
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
        r1cs.compile_ir(program).map_err(|e| format!("{e}"))?;

        let r1cs_data = write_r1cs(&r1cs.cs, PrimeId::Bn254);
        r1cs_b64 = Some(b64.encode(&r1cs_data));
    }

    // Generate Solidity verifier if requested
    if do_solidity {
        let vk = proving::groth16_bn254::setup_vk_only(&r1cs.cs, cache_dir)
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
        proof: proof_json,
        public_json,
        vkey: vkey_json,
        solidity: solidity_src,
        error: None,
        diagnostics: None,
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
    let mut compiler = zkc::plonkish_backend::PlonkishCompiler::new();
    let proven = ir::passes::bool_prop::compute_proven_boolean(program);
    compiler.set_proven_boolean(proven);

    let mut proof_json = None;
    let mut public_json = None;
    let mut vkey_json = None;
    let n_constraints;

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
        compiler.compile_ir(program).map_err(|e| format!("{e}"))?;

        n_constraints = compiler.num_circuit_rows();
    }

    Ok(CircuitResponse {
        success: true,
        constraints: n_constraints,
        public_inputs: n_public,
        private_inputs: n_witness,
        backend: "plonkish".into(),
        r1cs: None,
        proof: proof_json,
        public_json,
        vkey: vkey_json,
        solidity: None,
        error: None,
        diagnostics: None,
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
