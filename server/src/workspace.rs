//! Workspace-aware compilation and execution.
//!
//! Reads entry file and backend from achronyme.toml, sets compiler base_path
//! so imports resolve against the workspace directory.

use std::path::Path;

use crate::pipeline::RunOutput;
use crate::prove_handler::ProveBackend;

/// Minimal achronyme.toml parsing — only what the server needs.
#[derive(serde::Deserialize, Default)]
struct AchronymeToml {
    project: Option<ProjectSection>,
    build: Option<BuildSection>,
}

#[derive(serde::Deserialize)]
struct ProjectSection {
    entry: Option<String>,
}

#[derive(serde::Deserialize)]
struct BuildSection {
    backend: Option<String>,
}

/// Run a workspace project. Reads achronyme.toml for entry point and backend.
pub fn run_workspace(workspace: &Path, budget: u64, max_heap: usize) -> RunOutput {
    // 1. Read and parse achronyme.toml
    let toml_path = workspace.join("achronyme.toml");
    let config: AchronymeToml = if toml_path.exists() {
        let toml_str = match std::fs::read_to_string(&toml_path) {
            Ok(s) => s,
            Err(e) => {
                return RunOutput {
                    success: false,
                    output: String::new(),
                    error: Some(format!("cannot read achronyme.toml: {e}")),
                }
            }
        };
        match toml::from_str(&toml_str) {
            Ok(c) => c,
            Err(e) => {
                return RunOutput {
                    success: false,
                    output: String::new(),
                    error: Some(format!("invalid achronyme.toml: {e}")),
                }
            }
        }
    } else {
        AchronymeToml::default()
    };

    let entry = config
        .project
        .and_then(|p| p.entry)
        .unwrap_or_else(|| "src/main.ach".to_string());

    let backend = match config.build.and_then(|b| b.backend).as_deref() {
        Some("plonkish") => ProveBackend::Plonkish,
        _ => ProveBackend::R1cs,
    };

    // 2. Read entry file
    let entry_path = workspace.join(&entry);
    let source = match std::fs::read_to_string(&entry_path) {
        Ok(s) => s,
        Err(e) => {
            return RunOutput {
                success: false,
                output: String::new(),
                error: Some(format!("cannot read {entry}: {e}")),
            }
        }
    };

    // 3. Compile and run with base_path + backend
    let base_path = entry_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| workspace.to_path_buf());

    crate::pipeline::run_source_with_base_path(&source, budget, max_heap, Some(base_path), backend)
}
