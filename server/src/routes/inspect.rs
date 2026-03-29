//! POST /api/inspect — Build inspector DAG from a circuit.

use std::collections::HashMap;

use axum::Json;
use serde::Deserialize;

use compiler::r1cs_backend::R1CSCompiler;
use ir::prove_ir::ProveIrCompiler;
use ir::types::SsaVar;
use memory::FieldElement;

use crate::error::ApiError;
use crate::sandbox::sandboxed;

const INSPECT_TIMEOUT_SECS: u64 = 10;

#[derive(Deserialize)]
pub struct InspectRequest {
    source: String,
    #[serde(default)]
    inputs: HashMap<String, String>,
}

pub async fn handler(
    axum::extract::State(_store): axum::extract::State<crate::session::SessionStore>,
    Json(req): Json<InspectRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let source = req.source;
    let raw_inputs = req.inputs;

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
    let prove_ir = ProveIrCompiler::compile_circuit(source, Some(source_path))
        .map_err(|e| format!("{e}"))?;
    let prove_ir_text = format!("{prove_ir}");
    let circuit_name = prove_ir.name.clone();

    // Instantiate
    let mut program = prove_ir
        .instantiate(&HashMap::new())
        .map_err(|e| format!("{e}"))?;

    // Optimize
    ir::passes::optimize(&mut program);

    // Evaluate and compile constraints
    let (witness_values, eval_failures): (HashMap<SsaVar, FieldElement>, Vec<usize>) =
        if let Some(ref input_map) = inputs {
            ir::eval::evaluate_lenient(&program, input_map)
        } else {
            (HashMap::new(), Vec::new())
        };

    let proven = ir::passes::bool_prop::compute_proven_boolean(&program);
    let mut r1cs = R1CSCompiler::new();
    r1cs.set_proven_boolean(proven);

    let mut failed_nodes: HashMap<usize, Option<String>> = HashMap::new();
    let mut constraint_counts: HashMap<usize, usize> = HashMap::new();

    for idx in &eval_failures {
        let msg = extract_assert_message(&program.instructions[*idx]);
        failed_nodes.insert(*idx, msg);
    }

    if let Some(ref input_map) = inputs {
        match r1cs.compile_ir_with_witness(&program, input_map) {
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
    } else if r1cs.compile_ir(&program).is_ok() {
        for origin in &r1cs.constraint_origins {
            *constraint_counts.entry(origin.ir_index).or_insert(0) += 1;
        }
    }

    let graph = ir::inspector::build_inspector_graph(
        &program,
        &witness_values,
        &failed_nodes,
        &constraint_counts,
        Some(source.to_string()),
        Some(prove_ir_text),
        circuit_name.as_deref(),
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
