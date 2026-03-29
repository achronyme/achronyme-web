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
            r#"// {name}
let a = 6
let b = 7
let result = prove {{
    public out
    witness x
    witness y
    assert_eq(x * y, out)
}}
print("Proof:", result)
"#
        ),
        // "circuit" or default
        _ => format!(
            r#"// {name}
public out
witness a
witness b
assert_eq(a * b, out)
"#
        ),
    };
    std::fs::write(src_dir.join("main.ach"), main_content)
        .map_err(|e| format!("write src/main.ach: {e}"))?;

    Ok(())
}
