//! Compile-and-run pipeline for Achronyme source code.
//!
//! Mirrors the execution model of `wasm/src/lib.rs` and `cli/src/commands/run.rs`,
//! adapted for server-side use with captured print output.

use std::cell::RefCell;
use std::rc::Rc;

use compiler::Compiler;
use memory::{Closure, Function, Value};
use vm::error::RuntimeError;
use vm::native::NativeObj;
use vm::{CallFrame, ValueOps, VM};

use crate::prove_handler::ServerProveHandler;

/// Wrapper to share `ServerProveHandler` via Rc (orphan rule workaround).
struct SharedHandler(Rc<ServerProveHandler>);

impl vm::ProveHandler for SharedHandler {
    fn execute_prove_ir(
        &self,
        prove_ir_bytes: &[u8],
        scope_values: &std::collections::HashMap<String, memory::FieldElement>,
    ) -> Result<vm::ProveResult, vm::ProveError> {
        self.0.execute_prove_ir(prove_ir_bytes, scope_values)
    }
}

impl vm::VerifyHandler for SharedHandler {
    fn verify_proof(&self, proof: &memory::ProofObject) -> Result<bool, String> {
        self.0.verify_proof(proof)
    }
}

// Thread-local buffer for capturing print() output.
// Safe because each request runs in its own `spawn_blocking` thread.
thread_local! {
    static OUTPUT: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

/// Custom print native that writes to the thread-local buffer.
fn captured_print(vm: &mut VM, args: &[Value]) -> Result<Value, RuntimeError> {
    let mut line = String::new();
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            line.push(' ');
        }
        line.push_str(&vm.val_to_string(arg));
    }
    OUTPUT.with(|buf| buf.borrow_mut().push(line));
    Ok(Value::nil())
}

/// Result of compiling and running a program.
pub struct RunOutput {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// Result of a compile-only check.
pub struct CompileOutput {
    pub success: bool,
    pub diagnostics: Vec<DiagnosticInfo>,
}

/// A single diagnostic message.
#[derive(serde::Serialize)]
pub struct DiagnosticInfo {
    pub message: String,
    pub line: usize,
    pub col: usize,
    pub severity: &'static str,
}

/// Compile and run source code, returning captured output.
///
/// `budget` controls the maximum number of VM instructions (0 = unlimited).
/// `max_heap` controls the maximum heap size in bytes.
pub fn run_source(source: &str, budget: u64, max_heap: usize) -> RunOutput {
    run_source_with_base_path(
        source,
        budget,
        max_heap,
        None,
        crate::prove_handler::ProveBackend::R1cs,
    )
}

/// Compile and run with an optional base_path and prove backend.
pub fn run_source_with_base_path(
    source: &str,
    budget: u64,
    max_heap: usize,
    base_path: Option<std::path::PathBuf>,
    backend: crate::prove_handler::ProveBackend,
) -> RunOutput {
    OUTPUT.with(|buf| buf.borrow_mut().clear());

    match run_inner(source, budget, max_heap, base_path, backend) {
        Ok(()) => {
            let output = OUTPUT.with(|buf| buf.borrow().join("\n"));
            RunOutput {
                success: true,
                output,
                error: None,
            }
        }
        Err(msg) => {
            let output = OUTPUT.with(|buf| buf.borrow().join("\n"));
            RunOutput {
                success: false,
                output,
                error: Some(msg),
            }
        }
    }
}

fn run_inner(
    source: &str,
    budget: u64,
    max_heap: usize,
    base_path: Option<std::path::PathBuf>,
    backend: crate::prove_handler::ProveBackend,
) -> Result<(), String> {
    // 1. Compile
    let mut compiler = Compiler::new();
    if let Some(bp) = base_path {
        compiler.base_path = Some(bp);
    }
    let bytecode = compiler.compile(source).map_err(|e| format!("{e}"))?;

    // 2. Create VM
    let mut vm = VM::new();

    // Replace print native (index 0) with captured version
    if !vm.natives.is_empty() {
        vm.natives[0] = NativeObj {
            name: "print".to_string(),
            func: captured_print,
            arity: -1,
        };
    }

    // Set resource limits
    vm.instruction_budget = budget;
    vm.heap.max_heap_bytes = max_heap;

    // Register prove/verify handlers for prove {} blocks
    let handler = Rc::new(ServerProveHandler::new(backend));
    vm.prove_handler = Some(Box::new(SharedHandler(Rc::clone(&handler))));
    vm.verify_handler = Some(Box::new(SharedHandler(handler)));

    // 3. Transfer artifacts from compiler to VM
    vm.import_strings(compiler.interner.strings);
    vm.heap.import_bytes(compiler.bytes_interner.blobs);

    let field_map = vm
        .heap
        .import_fields(compiler.field_interner.fields)
        .map_err(|e| format!("field import: {e}"))?;
    let bigint_map = vm
        .heap
        .import_bigints(compiler.bigint_interner.bigints)
        .map_err(|e| format!("bigint import: {e}"))?;

    // Remap handles in prototypes
    for proto in &mut compiler.prototypes {
        remap_field_handles(&mut proto.constants, &field_map);
        remap_bigint_handles(&mut proto.constants, &bigint_map);
    }

    // 4. Allocate prototypes on heap
    for proto in &compiler.prototypes {
        let handle = vm
            .heap
            .alloc_function(proto.clone())
            .map_err(|e| format!("alloc prototype: {e}"))?;
        vm.prototypes.push(handle);
    }

    // 5. Create main function
    let main_func = compiler
        .compilers
        .last()
        .ok_or_else(|| "no main function".to_string())?;

    let mut main_constants = main_func.constants.clone();
    remap_field_handles(&mut main_constants, &field_map);
    remap_bigint_handles(&mut main_constants, &bigint_map);

    let func = Function {
        name: "main".to_string(),
        arity: 0,
        chunk: bytecode,
        constants: main_constants,
        max_slots: main_func.max_slots,
        upvalue_info: vec![],
        line_info: main_func.line_info.clone(),
    };

    let func_idx = vm
        .heap
        .alloc_function(func)
        .map_err(|e| format!("alloc main: {e}"))?;
    let closure_idx = vm
        .heap
        .alloc_closure(Closure {
            function: func_idx,
            upvalues: vec![],
        })
        .map_err(|e| format!("alloc closure: {e}"))?;

    vm.frames.push(CallFrame {
        closure: closure_idx,
        ip: 0,
        base: 0,
        dest_reg: 0,
    });

    // 6. Execute
    vm.interpret().map_err(|e| {
        if let Some((func_name, line)) = &vm.last_error_location {
            format!("[line {line}] in {func_name}: {e}")
        } else {
            format!("Runtime error: {e}")
        }
    })
}

/// Check source code for errors without executing.
pub fn check_source(source: &str) -> CompileOutput {
    // Parse
    let (_, errors) = achronyme_parser::parse_program(source);
    if !errors.is_empty() {
        let diagnostics = errors
            .iter()
            .map(|e| DiagnosticInfo {
                message: e.message.clone(),
                line: e.primary_span.line_start,
                col: e.primary_span.col_start,
                severity: "error",
            })
            .collect();
        return CompileOutput {
            success: false,
            diagnostics,
        };
    }

    // Compile (catches semantic errors)
    let mut compiler = Compiler::new();
    match compiler.compile(source) {
        Ok(_) => CompileOutput {
            success: true,
            diagnostics: vec![],
        },
        Err(e) => CompileOutput {
            success: false,
            diagnostics: vec![DiagnosticInfo {
                message: format!("{e}"),
                line: 0,
                col: 0,
                severity: "error",
            }],
        },
    }
}

// --- Handle remapping (mirrors wasm/src/lib.rs) ---

fn remap_field_handles(constants: &mut [Value], field_map: &[u32]) {
    for val in constants.iter_mut() {
        if val.is_field() {
            let old_handle = val.as_handle().expect("Field value must have handle");
            if let Some(&new_handle) = field_map.get(old_handle as usize) {
                *val = Value::field(new_handle);
            }
        }
    }
}

fn remap_bigint_handles(constants: &mut [Value], bigint_map: &[u32]) {
    for val in constants.iter_mut() {
        if val.is_bigint() {
            let old_handle = val.as_handle().expect("BigInt value must have handle");
            if let Some(&new_handle) = bigint_map.get(old_handle as usize) {
                *val = Value::bigint(new_handle);
            }
        }
    }
}
