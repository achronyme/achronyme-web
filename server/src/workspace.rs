//! Workspace-aware compilation and execution.
//!
//! Reads entry file and backend from achronyme.toml, sets compiler base_path
//! so imports resolve against the workspace directory.

use std::path::{Path, PathBuf};

use crate::pipeline::RunOutput;
use crate::prove_handler::ProveBackend;

/// Minimal achronyme.toml parsing — only what the server needs.
#[derive(serde::Deserialize, Default)]
struct AchronymeToml {
    project: Option<ProjectSection>,
    build: Option<BuildSection>,
    circom: Option<CircomSection>,
}

#[derive(serde::Deserialize)]
struct ProjectSection {
    entry: Option<String>,
}

#[derive(serde::Deserialize)]
struct BuildSection {
    backend: Option<String>,
}

#[derive(serde::Deserialize)]
struct CircomSection {
    libs: Option<Vec<String>>,
}

/// Validate and resolve `[circom] libs = [...]` entries against a workspace.
///
/// Each entry must be a workspace-relative path that resolves (after
/// canonicalization) underneath the workspace root. Absolute paths,
/// `..` traversal, and symlinks pointing outside the workspace are
/// rejected.
///
/// This is the **only** validation layer between the user-controlled
/// `achronyme.toml` and the `circom` crate's include resolver — the
/// crate itself does not re-check that paths stay sandboxed. Any
/// vulnerability here reaches the host filesystem.
pub fn resolve_circom_libs(workspace: &Path, raw: &[String]) -> Result<Vec<PathBuf>, String> {
    let canonical_ws = workspace
        .canonicalize()
        .map_err(|e| format!("workspace error: {e}"))?;

    let mut out = Vec::with_capacity(raw.len());
    for entry in raw {
        if entry.is_empty() {
            return Err("[circom] libs entry is empty".into());
        }
        if entry.starts_with('/') || entry.starts_with('\\') {
            return Err(format!("[circom] libs entry '{entry}' must be relative"));
        }
        if entry.contains("..") {
            return Err(format!(
                "[circom] libs entry '{entry}' contains path traversal"
            ));
        }

        let full = workspace.join(entry);
        if !full.exists() {
            return Err(format!(
                "[circom] libs entry '{entry}' does not exist in workspace"
            ));
        }

        let canonical = full
            .canonicalize()
            .map_err(|e| format!("[circom] libs '{entry}': {e}"))?;
        if !canonical.starts_with(&canonical_ws) {
            return Err(format!("[circom] libs entry '{entry}' escapes workspace"));
        }
        if !canonical.is_dir() {
            return Err(format!("[circom] libs entry '{entry}' is not a directory"));
        }

        out.push(canonical);
    }
    Ok(out)
}

/// Result of parsing achronyme.toml — entry path, backend, and resolved
/// circom library directories. Returned to route handlers that need the
/// circom dispatch branch.
#[derive(Debug)]
pub struct WorkspaceConfig {
    pub entry: PathBuf,
    pub backend: ProveBackend,
    pub circom_libs: Vec<PathBuf>,
}

/// Parse achronyme.toml from a workspace and resolve every reference to
/// an absolute, sandboxed path. Returns `Err(String)` for malformed TOML
/// or any libs entry that fails validation.
pub fn load_workspace_config(workspace: &Path) -> Result<WorkspaceConfig, String> {
    let toml_path = workspace.join("achronyme.toml");
    let config: AchronymeToml = if toml_path.exists() {
        let toml_str =
            std::fs::read_to_string(&toml_path).map_err(|e| format!("cannot read achronyme.toml: {e}"))?;
        toml::from_str(&toml_str).map_err(|e| format!("invalid achronyme.toml: {e}"))?
    } else {
        AchronymeToml::default()
    };

    let entry_rel = config
        .project
        .and_then(|p| p.entry)
        .unwrap_or_else(|| "src/main.ach".to_string());

    // entry is user-controlled via achronyme.toml. Without these checks
    // the circom dispatch path would let a malicious config read arbitrary
    // host files (`entry = "../../../etc/passwd"`) and surface their
    // contents through error messages.
    if entry_rel.is_empty() {
        return Err("[project] entry is empty".into());
    }
    if entry_rel.starts_with('/') || entry_rel.starts_with('\\') {
        return Err(format!(
            "[project] entry '{entry_rel}' must be workspace-relative"
        ));
    }
    if entry_rel.contains("..") {
        return Err(format!(
            "[project] entry '{entry_rel}' contains path traversal"
        ));
    }
    let entry = workspace.join(&entry_rel);
    if entry.exists() {
        let canonical_ws = workspace
            .canonicalize()
            .map_err(|e| format!("workspace error: {e}"))?;
        let canonical_entry = entry
            .canonicalize()
            .map_err(|e| format!("[project] entry '{entry_rel}': {e}"))?;
        if !canonical_entry.starts_with(&canonical_ws) {
            return Err(format!(
                "[project] entry '{entry_rel}' escapes workspace"
            ));
        }
    }

    let backend = match config.build.and_then(|b| b.backend).as_deref() {
        Some("plonkish") => ProveBackend::Plonkish,
        _ => ProveBackend::R1cs,
    };

    let circom_libs = match config.circom.and_then(|c| c.libs) {
        Some(raw) => resolve_circom_libs(workspace, &raw)?,
        None => Vec::new(),
    };

    Ok(WorkspaceConfig {
        entry,
        backend,
        circom_libs,
    })
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
                    proofs: vec![],
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
                    proofs: vec![],
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
                proofs: vec![],
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn mktemp_workspace() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ach-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_libs_accepts_relative_existing_dir() {
        let ws = mktemp_workspace();
        fs::create_dir_all(ws.join("circomlib/circuits")).unwrap();
        let resolved =
            resolve_circom_libs(&ws, &["circomlib/circuits".to_string()]).expect("should accept");
        assert_eq!(resolved.len(), 1);
        assert!(resolved[0].ends_with("circomlib/circuits"));
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn resolve_libs_rejects_absolute_path() {
        let ws = mktemp_workspace();
        let err = resolve_circom_libs(&ws, &["/etc".to_string()]).unwrap_err();
        assert!(err.contains("must be relative"), "got: {err}");
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn resolve_libs_rejects_traversal() {
        let ws = mktemp_workspace();
        let err = resolve_circom_libs(&ws, &["../etc".to_string()]).unwrap_err();
        assert!(err.contains("path traversal"), "got: {err}");
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn resolve_libs_rejects_missing_dir() {
        let ws = mktemp_workspace();
        let err = resolve_circom_libs(&ws, &["nonexistent".to_string()]).unwrap_err();
        assert!(err.contains("does not exist"), "got: {err}");
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn resolve_libs_rejects_file_not_dir() {
        let ws = mktemp_workspace();
        fs::write(ws.join("circomlib"), "not a dir").unwrap();
        let err = resolve_circom_libs(&ws, &["circomlib".to_string()]).unwrap_err();
        assert!(err.contains("not a directory"), "got: {err}");
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn resolve_libs_rejects_symlink_escaping_workspace() {
        let ws = mktemp_workspace();
        let outside = std::env::temp_dir().join(format!("ach-outside-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&outside).unwrap();
        // Create a symlink inside the workspace pointing outside
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, ws.join("escape")).unwrap();
        #[cfg(unix)]
        {
            let err = resolve_circom_libs(&ws, &["escape".to_string()]).unwrap_err();
            assert!(err.contains("escapes workspace"), "got: {err}");
        }
        let _ = fs::remove_dir_all(&ws);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn resolve_libs_empty_input_returns_empty() {
        let ws = mktemp_workspace();
        let resolved = resolve_circom_libs(&ws, &[]).expect("empty should pass");
        assert!(resolved.is_empty());
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn load_workspace_config_parses_circom_libs_section() {
        let ws = mktemp_workspace();
        fs::create_dir_all(ws.join("circomlib/circuits")).unwrap();
        fs::write(
            ws.join("achronyme.toml"),
            r#"
[project]
entry = "src/main.circom"

[circom]
libs = ["circomlib/circuits"]
"#,
        )
        .unwrap();
        let config = load_workspace_config(&ws).expect("should parse");
        assert!(config.entry.ends_with("src/main.circom"));
        assert_eq!(config.circom_libs.len(), 1);
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn load_workspace_config_rejects_entry_traversal() {
        let ws = mktemp_workspace();
        fs::write(
            ws.join("achronyme.toml"),
            r#"
[project]
entry = "../../../etc/passwd"
"#,
        )
        .unwrap();
        let err = load_workspace_config(&ws).unwrap_err();
        assert!(err.contains("path traversal"), "got: {err}");
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn load_workspace_config_rejects_absolute_entry() {
        let ws = mktemp_workspace();
        fs::write(
            ws.join("achronyme.toml"),
            r#"
[project]
entry = "/etc/passwd"
"#,
        )
        .unwrap();
        let err = load_workspace_config(&ws).unwrap_err();
        assert!(err.contains("workspace-relative"), "got: {err}");
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn load_workspace_config_accepts_existing_entry() {
        let ws = mktemp_workspace();
        fs::create_dir_all(ws.join("src")).unwrap();
        fs::write(ws.join("src/main.circom"), "// test").unwrap();
        fs::write(
            ws.join("achronyme.toml"),
            r#"
[project]
entry = "src/main.circom"
"#,
        )
        .unwrap();
        let config = load_workspace_config(&ws).expect("should accept");
        assert!(config.entry.ends_with("src/main.circom"));
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn load_workspace_config_handles_missing_circom_section() {
        let ws = mktemp_workspace();
        fs::write(
            ws.join("achronyme.toml"),
            r#"
[project]
entry = "src/main.ach"
"#,
        )
        .unwrap();
        let config = load_workspace_config(&ws).expect("should parse");
        assert!(config.circom_libs.is_empty());
        let _ = fs::remove_dir_all(&ws);
    }
}
