//! POST /api/inspect — Build inspector DAG from a circuit.
//!
//! Three modes, mirroring `/api/circuit` and `/api/prove`:
//! - Single-source `.ach` (back-compat): request body carries `source`.
//! - Workspace `.ach`: `X-Ach-Session` header, `[project] entry = "...ach"`.
//! - Workspace `.circom`: `X-Ach-Session` header, `[project] entry = "...circom"`,
//!   `[circom] libs = [...]` resolves to circomlib include paths.

use std::collections::HashMap;
use std::path::PathBuf;

use axum::Json;
use serde::Deserialize;

use ir::SsaVar;
use ir_forge::ProveIrCompiler;
use memory::FieldElement;
use zkc::r1cs_backend::R1CSCompiler;

use crate::error::ApiError;
use crate::sandbox::sandboxed;

const INSPECT_TIMEOUT_SECS: u64 = 10;

#[derive(Deserialize)]
pub struct InspectRequest {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    inputs: HashMap<String, String>,
}

pub async fn handler(
    axum::extract::State(store): axum::extract::State<crate::session::SessionStore>,
    headers: axum::http::HeaderMap,
    Json(req): Json<InspectRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let raw_inputs = req.inputs;

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
                move || inspect_circuit_circom(&entry, &libs, &raw_inputs),
                INSPECT_TIMEOUT_SECS,
            )
            .await?;
            return match result {
                Ok(graph_json) => Ok(Json(graph_json)),
                Err(msg) => Err(ApiError::CompileError(msg)),
            };
        }

        let source = std::fs::read_to_string(&config.entry).map_err(|e| {
            ApiError::BadRequest(format!("cannot read entry {}: {e}", config.entry.display()))
        })?;
        let result = sandboxed(
            move || inspect_circuit(&source, &raw_inputs),
            INSPECT_TIMEOUT_SECS,
        )
        .await?;
        return match result {
            Ok(graph_json) => Ok(Json(graph_json)),
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
        move || inspect_circuit(&source, &raw_inputs),
        INSPECT_TIMEOUT_SECS,
    )
    .await?;

    match result {
        Ok(graph_json) => Ok(Json(graph_json)),
        Err(msg) => Err(ApiError::CompileError(msg)),
    }
}

fn inspect_circuit(
    source: &str,
    raw_inputs: &HashMap<String, String>,
) -> Result<serde_json::Value, String> {
    let source_path = std::path::Path::new("playground.ach");

    // Parse inputs
    let inputs = if raw_inputs.is_empty() {
        None
    } else {
        let mut map = HashMap::new();
        for (name, val_str) in raw_inputs {
            let fe = parse_field_value(name, val_str)?;
            map.insert(name.clone(), fe);
        }
        Some(map)
    };

    // Compile circuit to ProveIR
    let prove_ir = ProveIrCompiler::<memory::Bn254Fr>::compile_circuit(source, Some(source_path))
        .map_err(|e| format!("{e}"))?;
    let prove_ir_text = format!("{prove_ir}");
    let circuit_name = prove_ir.name.clone();

    // Instantiate
    let mut program = prove_ir
        .instantiate_lysis(&HashMap::new())
        .map_err(|e| format!("{e}"))?;

    // Optimize
    ir::passes::optimize(&mut program);

    build_inspector_response(
        &program,
        inputs.as_ref(),
        Some(source.to_string()),
        prove_ir_text,
        circuit_name.as_deref(),
    )
}

fn inspect_circuit_circom(
    entry: &std::path::Path,
    libs: &[PathBuf],
    raw_inputs: &HashMap<String, String>,
) -> Result<serde_json::Value, String> {
    let inputs = if raw_inputs.is_empty() {
        None
    } else {
        let mut map = HashMap::new();
        for (name, val_str) in raw_inputs {
            let fe = parse_field_value(name, val_str)?;
            map.insert(name.clone(), fe);
        }
        Some(map)
    };

    let compiled = crate::circom_pipeline::compile_circom(entry, libs).map_err(|diags| {
        // Inspector can't render structured diagnostics — collapse to a
        // joined error string. Each diagnostic line carries its span so
        // the user still sees `file:line:col` context.
        diags
            .iter()
            .map(|d| {
                format!(
                    "{}:{}: {}",
                    d.primary_span.line_start, d.primary_span.col_start, d.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    let prove_ir_text = format!("{}", compiled.prove_ir);
    let circuit_name = compiled.prove_ir.name.clone();

    let mut program = crate::circom_pipeline::instantiate_circom(&compiled)?;
    ir::passes::optimize(&mut program);

    // Witness hints: only run when the user supplied inputs. Off-circuit
    // evaluation of `<--` hints is the expensive step on heavy
    // templates, and the inspector also works without inputs (constraint-
    // only view).
    let merged_inputs = if let Some(ref inputs_map) = inputs {
        Some(crate::circom_pipeline::merge_circom_witness(
            &compiled, inputs_map,
        )?)
    } else {
        None
    };

    let entry_source = std::fs::read_to_string(entry).ok();

    build_inspector_response(
        &program,
        merged_inputs.as_ref(),
        entry_source,
        prove_ir_text,
        circuit_name.as_deref(),
    )
}

/// Build the inspector graph JSON from an instantiated `IrProgram` and
/// optional witness inputs. Shared by `.ach` and `.circom` paths so both
/// surface the same node/edge layout, witness values, failed-assert
/// markers, and per-instruction constraint counts.
fn build_inspector_response(
    program: &ir::IrProgram,
    inputs: Option<&HashMap<String, FieldElement>>,
    source_text: Option<String>,
    prove_ir_text: String,
    circuit_name: Option<&str>,
) -> Result<serde_json::Value, String> {
    let (witness_values, eval_failures): (HashMap<SsaVar, FieldElement>, Vec<usize>) =
        if let Some(input_map) = inputs {
            ir::eval::evaluate_lenient(program, input_map)
        } else {
            (HashMap::new(), Vec::new())
        };

    let proven = ir::passes::bool_prop::compute_proven_boolean(program);
    let mut r1cs = R1CSCompiler::new();
    r1cs.set_proven_boolean(proven);

    let mut failed_nodes: HashMap<usize, Option<String>> = HashMap::new();
    let mut constraint_counts: HashMap<usize, usize> = HashMap::new();

    for idx in &eval_failures {
        let msg = extract_assert_message(&program.instructions[*idx]);
        failed_nodes.insert(*idx, msg);
    }

    if let Some(input_map) = inputs {
        match r1cs.compile_ir_with_witness(program, input_map) {
            Ok(witness_vec) => {
                for origin in &r1cs.constraint_origins {
                    *constraint_counts.entry(origin.ir_index).or_insert(0) += 1;
                }
                if let Err(constraints::r1cs::ConstraintError::ConstraintUnsatisfied(idx)) =
                    r1cs.cs.verify(&witness_vec)
                {
                    if let Some(origin) = r1cs.constraint_origins.get(idx) {
                        let msg = extract_assert_message(&program.instructions[origin.ir_index]);
                        failed_nodes.insert(origin.ir_index, msg);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("R1CS compilation failed: {e}");
            }
        }
    } else if r1cs.compile_ir(program).is_ok() {
        for origin in &r1cs.constraint_origins {
            *constraint_counts.entry(origin.ir_index).or_insert(0) += 1;
        }
    }

    let graph = ir::inspector::build_inspector_graph(
        program,
        &witness_values,
        &failed_nodes,
        &constraint_counts,
        source_text,
        Some(prove_ir_text),
        circuit_name,
    );

    serde_json::to_value(&graph).map_err(|e| format!("JSON serialization: {e}"))
}

fn extract_assert_message(inst: &ir::Instruction) -> Option<String> {
    match inst {
        ir::Instruction::AssertEq { message, .. } | ir::Instruction::Assert { message, .. } => {
            message.clone()
        }
        _ => None,
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
