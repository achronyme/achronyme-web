//! Project templates — mirrors `ach init --template <name>`.
//!
//! Generates the same project structure as `cli/src/init.rs`:
//! achronyme.toml, .gitignore, src/main.ach (and additional source
//! files for multi-file templates such as `circom`).

use std::path::Path;

/// Available templates (match `ach init --template` options).
pub const TEMPLATES: &[&str] = &["circuit", "vm", "prove", "circom", "circomlib-demo"];

/// Populate a workspace with a template project.
/// Mirrors `ach init <name> --template <template>`.
pub fn populate_template(template: &str, workspace: &Path) -> Result<(), String> {
    let name = template.replace('-', "_");

    if !TEMPLATES.contains(&template) {
        return Err(format!(
            "unknown template: {template}. Available: {}",
            TEMPLATES.join(", ")
        ));
    }

    let src_dir = workspace.join("src");
    std::fs::create_dir_all(&src_dir).map_err(|e| format!("mkdir src: {e}"))?;

    // Templates that pull from circomlib opt in via `[circom] libs`.
    // Other templates ship a minimal toml with no libs section.
    let toml_content = match template {
        "circomlib-demo" => format!(
            r#"[project]
name = "{name}"
version = "0.1.0"
entry = "src/main.ach"

[build]
backend = "r1cs"

[circom]
libs = ["@circomlib"]
"#
        ),
        _ => format!(
            r#"[project]
name = "{name}"
version = "0.1.0"
entry = "src/main.ach"

[build]
backend = "r1cs"
"#
        ),
    };
    std::fs::write(workspace.join("achronyme.toml"), toml_content)
        .map_err(|e| format!("write achronyme.toml: {e}"))?;

    let main_content = match template {
        "vm" => format!(
            r#"// {name}
let message = "Hello from {name}!"
print(message)
"#
        ),
        "prove" => format!(
            r#"// {name} — Zero-Knowledge Proof
let secret = 0p42
let expected = poseidon(secret, 0p0)

print("Public hash: " + expected.to_string())

prove check(expected: Public) {{
    assert_eq(poseidon(secret, 0p0), expected, "hash mismatch")
}}

print("Proof verified!")
"#
        ),
        "circom" => CIRCOM_TUTORIAL_MAIN.to_string(),
        "circomlib-demo" => CIRCOMLIB_DEMO_MAIN.to_string(),
        // "circuit" or default
        _ => format!(
            r#"// {name} — ZK Circuit
//
// Proves: a * b == out, without revealing a or b.

let a = 6
let b = 7
let out = a * b

print("Generating proof for " + a.to_string() + " * " + b.to_string() + " = " + out.to_string())

let proof = prove multiply(out: Public) {{
    assert_eq(a * b, out)
}}

print("Proof generated!")
print("Verified: " + verify_proof(proof).to_string())
"#
        ),
    };
    std::fs::write(src_dir.join("main.ach"), main_content)
        .map_err(|e| format!("write src/main.ach: {e}"))?;

    if template == "circom" {
        std::fs::write(src_dir.join("square.circom"), CIRCOM_TUTORIAL_SQUARE)
            .map_err(|e| format!("write src/square.circom: {e}"))?;
    }
    if template == "circomlib-demo" {
        std::fs::write(src_dir.join("hash.circom"), CIRCOMLIB_DEMO_HASH)
            .map_err(|e| format!("write src/hash.circom: {e}"))?;
    }

    Ok(())
}

/// Achronyme entry point for the `circom` tutorial. Demonstrates the
/// two ways imported Circom templates plug into `.ach`: VM-mode for
/// runtime precomputation, and circuit-mode inside a `prove` block.
const CIRCOM_TUTORIAL_MAIN: &str = r#"// Achronyme + Circom — End-to-End Tutorial
//
// Two complementary uses of the same Circom template:
//
//   1. VM mode      — call Square at runtime to compute the public
//                     output (no proof, just the witness value).
//   2. Circuit mode — embed Square's constraint inside a `prove`
//                     block so the verifier can check that we know
//                     an input that squares to the public output,
//                     without revealing it.
//
// Press the Run button (▶) to execute. The prove block produces a
// Groth16 proof — switch to the Proof tab to inspect it.

import { Square } from "./square.circom"

// ── Step 1: pick a secret and compute its public square in VM mode.
let secret = 0p7
let public_square = Square()(secret)

print("Secret (hidden): " + secret.to_string())
print("Public square:   " + public_square.to_string())

// ── Step 2: prove we know a value that squares to public_square,
// without revealing what it is. The verifier only sees public_square.
prove check(public_square: Public) {
    let computed = Square()(secret)
    assert_eq(computed, public_square, "square mismatch")
}

print("Witness verified — see the Proof tab for the artifact.")
"#;

/// Companion Circom template for the `circom` tutorial. One quadratic
/// constraint (`out === in * in`) that both VM and circuit modes
/// reuse — the same source file participates in runtime evaluation
/// and R1CS extraction without duplication.
const CIRCOM_TUTORIAL_SQUARE: &str = r#"pragma circom 2.0.0;

// Square — squares its input.
//
// Imported by src/main.ach. Demonstrates how a Circom signal-and-
// constraint description plugs into Achronyme: `out <== in * in` is
// one quadratic constraint that the prover and verifier agree on.
template Square() {
    signal input in;
    signal output out;

    out <== in * in;
}
"#;

/// Demo entry point exercising real circomlib via the `@circomlib`
/// server mount. Proves knowledge of a Poseidon preimage — the
/// canonical "hello world" of practical ZK circuits.
const CIRCOMLIB_DEMO_MAIN: &str = r#"// Achronyme + circomlib — Poseidon Preimage Proof
//
// Demonstrates a real ZK use case: prove that you know a secret
// `x` such that `Poseidon(x) == h`, without revealing `x`.
//
// `src/hash.circom` wraps circomlib's Poseidon template — the
// `include "poseidon.circom"` inside it resolves through
// `[circom] libs = ["@circomlib"]` in achronyme.toml, which the
// server backs with the vendored circomlib bundle.
//
// Press Run (▶) to execute. The prove block emits a Groth16 proof;
// switch to the Proof tab to see the artifact.

import { PoseidonHash } from "./hash.circom"

// ── Step 1: pick a secret and compute its public Poseidon hash.
let secret = 0p42
let public_hash = PoseidonHash()(secret)

print("Secret (hidden): " + secret.to_string())
print("Public Poseidon hash: " + public_hash.to_string())

// ── Step 2: prove we know a preimage of public_hash. The verifier
// only sees public_hash and the proof — `secret` never leaves.
prove preimage(public_hash: Public) {
    let computed = PoseidonHash()(secret)
    assert_eq(computed, public_hash, "preimage mismatch")
}

print("Witness verified — Groth16 proof is in the Proof tab.")
"#;

/// Workspace-local wrapper around circomlib's Poseidon. Keeps the
/// public API to a single scalar in / single scalar out so .ach can
/// invoke it as `PoseidonHash()(x)` without juggling array signals.
const CIRCOMLIB_DEMO_HASH: &str = r#"pragma circom 2.0.0;

// Resolves through `[circom] libs = ["@circomlib"]` to the vendored
// circomlib's poseidon.circom on the server's read-only mount.
include "poseidon.circom";

// Fixed-arity wrapper: one input signal, one output signal.
// Matches Poseidon(1) — same as `let h = poseidon(x)` in the .ach
// scripting layer, but driven by circomlib's vetted constraints.
template PoseidonHash() {
    signal input in;
    signal output out;

    component p = Poseidon(1);
    p.inputs[0] <== in;
    out <== p.out;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn mktemp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ach-tpl-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn rejects_unknown_template() {
        let ws = mktemp();
        let err = populate_template("nonsense", &ws).unwrap_err();
        assert!(err.contains("unknown template"));
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn circom_template_writes_companion_circom_file() {
        let ws = mktemp();
        populate_template("circom", &ws).unwrap();

        let toml = std::fs::read_to_string(ws.join("achronyme.toml")).unwrap();
        assert!(toml.contains(r#"entry = "src/main.ach""#));

        let main = std::fs::read_to_string(ws.join("src/main.ach")).unwrap();
        assert!(main.contains(r#"import { Square } from "./square.circom""#));
        assert!(main.contains("prove check(public_square: Public)"));

        let circom = std::fs::read_to_string(ws.join("src/square.circom")).unwrap();
        assert!(circom.contains("template Square()"));
        assert!(circom.contains("out <== in * in;"));

        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn non_circom_templates_skip_companion_file() {
        for tpl in ["vm", "prove", "circuit"] {
            let ws = mktemp();
            populate_template(tpl, &ws).unwrap();
            assert!(
                !ws.join("src/square.circom").exists(),
                "template `{tpl}` must not emit a circom companion"
            );
            assert!(
                !ws.join("src/hash.circom").exists(),
                "template `{tpl}` must not emit a circomlib-demo companion"
            );
            let _ = std::fs::remove_dir_all(&ws);
        }
    }

    #[test]
    fn circomlib_demo_template_opts_into_at_circomlib() {
        let ws = mktemp();
        populate_template("circomlib-demo", &ws).unwrap();

        let toml = std::fs::read_to_string(ws.join("achronyme.toml")).unwrap();
        assert!(
            toml.contains(r#"libs = ["@circomlib"]"#),
            "circomlib-demo must declare the @circomlib mount"
        );

        let main = std::fs::read_to_string(ws.join("src/main.ach")).unwrap();
        assert!(main.contains(r#"import { PoseidonHash } from "./hash.circom""#));
        assert!(main.contains("prove preimage(public_hash: Public)"));

        let hash = std::fs::read_to_string(ws.join("src/hash.circom")).unwrap();
        assert!(hash.contains(r#"include "poseidon.circom";"#));
        assert!(hash.contains("template PoseidonHash()"));

        // Companion files for OTHER templates must not leak into this one.
        assert!(!ws.join("src/square.circom").exists());

        let _ = std::fs::remove_dir_all(&ws);
    }
}
