//! Circom front-end pipeline: file path + library_dirs → IrProgram ready
//! for the existing R1CS / Plonkish backend.
//!
//! Only the compile-and-instantiate steps live here. The downstream
//! pipeline (optimize → R1CS / Plonkish → proving) is reused as-is from
//! `routes/{compile,circuit,prove,inspect}.rs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ir::IrProgram;
use ir_forge::ProveIR;
use memory::{Bn254Fr, FieldElement};

/// Result of compiling a `.circom` entry. `prove_ir` and `output_names`
/// drive instantiation; `capture_values` carry the parametric template
/// args from `component main = Foo(...)` and are needed to compute
/// witness hints later.
pub struct CircomCompiled {
    pub prove_ir: ProveIR,
    pub output_names: std::collections::HashSet<String>,
    pub capture_values: HashMap<String, u64>,
    pub warnings: Vec<diagnostics::Diagnostic>,
}

/// Parse + lower a `.circom` file with the given library directories.
///
/// Errors are returned as `Vec<Diagnostic>` so the caller can render
/// them through the same channel `.ach` errors use.
pub fn compile_circom(
    entry: &Path,
    library_dirs: &[PathBuf],
) -> Result<CircomCompiled, Vec<diagnostics::Diagnostic>> {
    // Re-canonicalize each lib dir immediately before invoking circom.
    // The earlier validation in `workspace::resolve_circom_libs` happens
    // when the workspace config loads; the user retains write access to
    // their session afterward, so a symlink swap could otherwise let
    // `circom::compile_file` follow a path outside the workspace.
    // Cheap (one syscall per lib) and tightens the boundary to the
    // moment of use.
    let canonical_libs: Result<Vec<PathBuf>, _> =
        library_dirs.iter().map(|p| p.canonicalize()).collect();
    let canonical_libs = canonical_libs.map_err(|e| {
        vec![diagnostics::Diagnostic::error(
            format!("circom library path no longer resolvable: {e}"),
            diagnostics::SpanRange::point(0, 0, 0),
        )]
    })?;

    let result = circom::compile_file(entry, &canonical_libs).map_err(|e| e.to_diagnostics())?;

    Ok(CircomCompiled {
        prove_ir: result.prove_ir,
        output_names: result.output_names,
        capture_values: result.capture_values,
        warnings: result.warnings,
    })
}

/// Instantiate a circom-produced ProveIR into an SSA `IrProgram` ready
/// for the R1CS / Plonkish backend.
///
/// `output_names` keeps `signal output` wires on the public R1CS
/// boundary instead of duplicating them as witness wires.
pub fn instantiate_circom(compiled: &CircomCompiled) -> Result<IrProgram<Bn254Fr>, String> {
    let fe_captures: HashMap<String, FieldElement<Bn254Fr>> = compiled
        .capture_values
        .iter()
        .map(|(k, v)| (k.clone(), FieldElement::<Bn254Fr>::from_u64(*v)))
        .collect();

    compiled
        .prove_ir
        .instantiate_lysis_with_outputs(&fe_captures, &compiled.output_names)
        .map_err(|e| format!("{e}"))
}

/// Compute witness hints (off-circuit evaluation of `<--` expressions)
/// and merge them with user-provided inputs. Returns the combined map
/// suitable for `R1CSCompiler::compile_ir_with_witness`.
pub fn merge_circom_witness(
    compiled: &CircomCompiled,
    user_inputs: &HashMap<String, FieldElement<Bn254Fr>>,
) -> Result<HashMap<String, FieldElement<Bn254Fr>>, String> {
    let witness_values = circom::witness::compute_witness_hints_with_captures::<Bn254Fr>(
        &compiled.prove_ir,
        user_inputs,
        &compiled.capture_values,
    )
    .map_err(|e| format!("witness computation failed: {e}"))?;

    let mut combined = user_inputs.clone();
    combined.extend(witness_values);
    Ok(combined)
}

/// Serializable diagnostic shape used by HTTP responses. Mirrors
/// `pipeline::DiagnosticInfo` so the playground client renders both
/// `.ach` and `.circom` errors identically.
pub fn diagnostics_to_pipeline_format(
    diags: &[diagnostics::Diagnostic],
) -> Vec<crate::pipeline::DiagnosticInfo> {
    diags
        .iter()
        .map(|d| crate::pipeline::DiagnosticInfo {
            message: d.message.clone(),
            line: d.primary_span.line_start,
            col: d.primary_span.col_start,
            severity: match d.severity {
                diagnostics::Severity::Error => "error",
                diagnostics::Severity::Warning => "warning",
                diagnostics::Severity::Help => "help",
                diagnostics::Severity::Note => "note",
            },
        })
        .collect()
}
