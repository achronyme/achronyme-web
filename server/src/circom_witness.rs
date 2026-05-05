//! In-process witness dispatcher for `.ach` programs that import
//! `.circom` templates.
//!
//! `akronc` records every `.circom` library it encounters into
//! `compiler.circom_library_registry`, keyed by `library_id`. The
//! pipeline moves that registry into [`DefaultCircomWitnessHandler`]
//! and installs it on the VM before `vm.interpret()` runs. At runtime
//! the `CallCircomTemplate` opcode resolves
//! `handle.library_id → Arc<CircomLibrary>` here and delegates to
//! `circom::evaluate_template_witness` for the actual witness.
//!
//! Mirrors `cli/src/circom_handler.rs` in the achronyme repo — keep
//! the two in sync until the akron crate exposes a shared dispatcher.
//!
//! Bn254 only: the server compiles every workspace against the BN254
//! scalar field. Other curves need a field-generic variant.

use std::collections::HashMap;
use std::sync::Arc;

use akron::{CircomCallError, CircomCallResult, CircomOutputValue, CircomWitnessHandler};
use circom::library::{evaluate_template_witness, resolve_entry, TemplateOutputValue};
use circom::{CircomLibrary, DimensionExpr};
use ir_forge::types::FieldConst;
use memory::{Bn254Fr, CircomHandle, FieldElement};

/// Owns the same `Arc<CircomLibrary>` instances `akronc` allocated at
/// compile time so library ids embedded in every `CircomHandle`
/// always resolve.
pub struct DefaultCircomWitnessHandler {
    libraries: Vec<Arc<CircomLibrary>>,
}

impl DefaultCircomWitnessHandler {
    pub fn new(libraries: Vec<Arc<CircomLibrary>>) -> Self {
        Self { libraries }
    }
}

impl CircomWitnessHandler for DefaultCircomWitnessHandler {
    fn invoke(
        &self,
        handle: &CircomHandle,
        signal_inputs: &[FieldElement],
    ) -> Result<CircomCallResult, CircomCallError> {
        let lib = self
            .libraries
            .get(handle.library_id as usize)
            .ok_or(CircomCallError::UnknownLibraryId(handle.library_id))?;

        let entry = lib.template(&handle.template_name).ok_or_else(|| {
            CircomCallError::WitnessEvaluation(format!(
                "template `{}` no longer present in library id {} \
                 (compiler/runtime registry mismatch)",
                handle.template_name, handle.library_id
            ))
        })?;

        // Resolve the template's input layout against the concrete
        // template args so we know the total flat element count and
        // which inputs are arrays. The VM-mode compiler already
        // flattened array inputs into one register per element; we now
        // map each register slot back to its `signal_name[_i…]` key.
        let mut known_params: HashMap<String, FieldConst> = HashMap::new();
        for (param, arg) in entry.params.iter().zip(handle.template_args.iter()) {
            known_params.insert(param.clone(), FieldConst::from_u64(*arg));
        }
        let resolved = resolve_entry(entry, &known_params);

        let mut expected_flat: usize = 0;
        for sig in &resolved.inputs {
            if sig.is_scalar() {
                expected_flat += 1;
                continue;
            }
            let mut elems: usize = 1;
            for d in &sig.dimensions {
                match d {
                    DimensionExpr::Const(n) => elems *= *n as usize,
                    _ => {
                        return Err(CircomCallError::InvalidSignalInput {
                            index: 0,
                            reason: format!(
                                "circom template `{}` input `{}` has an unresolved \
                                 array dimension for args {:?}",
                                handle.template_name, sig.name, handle.template_args
                            ),
                        });
                    }
                }
            }
            expected_flat += elems;
        }

        if expected_flat != signal_inputs.len() {
            return Err(CircomCallError::InvalidSignalInput {
                index: 0,
                reason: format!(
                    "expected {expected_flat} flat signal input element(s) for circom \
                     template `{}`, got {}",
                    handle.template_name,
                    signal_inputs.len()
                ),
            });
        }

        // Build the name-keyed HashMap the library evaluator expects.
        // Scalars keep their declared names; array inputs expand into
        // `name_i` (or `name_i_j` for multi-dim) keys.
        let mut map: HashMap<String, FieldElement<Bn254Fr>> = HashMap::new();
        let mut cursor: usize = 0;
        for sig in &resolved.inputs {
            if sig.is_scalar() {
                map.insert(sig.name.clone(), signal_inputs[cursor]);
                cursor += 1;
                continue;
            }
            let dims: Vec<u64> = sig
                .dimensions
                .iter()
                .map(|d| match d {
                    DimensionExpr::Const(n) => *n,
                    _ => 0, // unreachable — caught above
                })
                .collect();
            for idx in flatten_row_major_indices(&dims) {
                let suffix = idx
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join("_");
                map.insert(format!("{}_{suffix}", sig.name), signal_inputs[cursor]);
                cursor += 1;
            }
        }

        let raw_outputs = evaluate_template_witness::<Bn254Fr>(
            lib,
            &handle.template_name,
            &handle.template_args,
            &map,
        )
        .map_err(|e| CircomCallError::WitnessEvaluation(e.to_string()))?;

        let mut outputs = HashMap::with_capacity(raw_outputs.len());
        for (name, out) in raw_outputs {
            let converted = match out {
                TemplateOutputValue::Scalar(v) => CircomOutputValue::Scalar(v),
                TemplateOutputValue::Array { dims, values } => {
                    CircomOutputValue::Array { dims, values }
                }
            };
            outputs.insert(name, converted);
        }

        Ok(CircomCallResult { outputs })
    }
}

/// Build the row-major list of multi-dimensional indices for a shape.
///
/// For `dims = [2, 3]` returns
/// `[[0,0], [0,1], [0,2], [1,0], [1,1], [1,2]]`. Used to map each
/// flat `FieldElement` slot delivered by the VM onto its
/// `name_i[_j…]` key in the witness-evaluator input map.
fn flatten_row_major_indices(dims: &[u64]) -> Vec<Vec<u64>> {
    let mut result = vec![Vec::new()];
    for &d in dims {
        let mut next = Vec::with_capacity(result.len() * d as usize);
        for prefix in &result {
            for i in 0..d {
                let mut p = prefix.clone();
                p.push(i);
                next.push(p);
            }
        }
        result = next;
    }
    result
}
