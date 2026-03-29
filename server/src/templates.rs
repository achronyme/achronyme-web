//! Project templates — mirrors `ach init --template <name>`.
//!
//! Generates the same project structure as `cli/src/init.rs`:
//! achronyme.toml, .gitignore, src/main.ach

use std::path::Path;

/// Available templates (match `ach init --template` options).
pub const TEMPLATES: &[&str] = &["circuit", "vm", "prove"];

/// Populate a workspace with a template project.
/// Mirrors `ach init <name> --template <template>`.
pub fn populate_template(template: &str, workspace: &Path) -> Result<(), String> {
    let name = template.replace('-', "_");

    // Validate template name
    if !TEMPLATES.contains(&template) {
        return Err(format!(
            "unknown template: {template}. Available: {}",
            TEMPLATES.join(", ")
        ));
    }

    let src_dir = workspace.join("src");
    std::fs::create_dir_all(&src_dir).map_err(|e| format!("mkdir src: {e}"))?;

    // achronyme.toml (same for all templates)
    let toml_content = format!(
        r#"[project]
name = "{name}"
version = "0.1.0"
entry = "src/main.ach"

[build]
backend = "r1cs"
"#
    );
    std::fs::write(workspace.join("achronyme.toml"), toml_content)
        .map_err(|e| format!("write achronyme.toml: {e}"))?;

    // src/main.ach (template-specific)
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

    Ok(())
}
