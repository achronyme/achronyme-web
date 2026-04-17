//! Session management for the playground mini-IDE.
//!
//! Each session gets a tmpdir workspace. Sessions expire after 2 hours
//! of inactivity. Max 200 concurrent sessions.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use uuid::Uuid;

const MAX_SESSIONS: usize = 200;
const SESSION_TTL: Duration = Duration::from_secs(2 * 60 * 60); // 2 hours
const REAPER_INTERVAL: Duration = Duration::from_secs(60);
const MAX_FILES_PER_SESSION: usize = 20;
const MAX_FILE_SIZE: usize = 32 * 1024; // 32KB
const MAX_WORKSPACE_SIZE: usize = 256 * 1024; // 256KB

/// Allowed file extensions.
fn is_allowed_path(path: &str) -> bool {
    path == "achronyme.toml" || path.ends_with(".ach")
}

/// Validate a user-supplied path is safe (for files).
pub fn validate_path(workspace: &Path, user_path: &str) -> Result<PathBuf, String> {
    if user_path.is_empty() {
        return Err("path is empty".into());
    }
    if user_path.starts_with('/') || user_path.starts_with('\\') {
        return Err("path must be relative".into());
    }
    if user_path.contains("..") {
        return Err("path traversal not allowed".into());
    }
    if !is_allowed_path(user_path) {
        return Err("only .ach files and achronyme.toml are allowed".into());
    }

    let full = workspace.join(user_path);

    // Ensure the resolved path is inside the workspace
    let canonical_ws = workspace
        .canonicalize()
        .map_err(|e| format!("workspace error: {e}"))?;

    // For new files that don't exist yet, check the parent
    if full.exists() {
        let canonical = full
            .canonicalize()
            .map_err(|e| format!("path error: {e}"))?;
        if !canonical.starts_with(&canonical_ws) {
            return Err("path escapes workspace".into());
        }
    } else {
        // Ensure parent exists and is inside workspace
        if let Some(parent) = full.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("cannot create directory: {e}"))?;
            }
            let canonical_parent = parent
                .canonicalize()
                .map_err(|e| format!("parent error: {e}"))?;
            if !canonical_parent.starts_with(&canonical_ws) {
                return Err("path escapes workspace".into());
            }
        }
    }

    Ok(full)
}

/// Validate a user-supplied directory path is safe.
pub fn validate_dir_path(workspace: &Path, user_path: &str) -> Result<PathBuf, String> {
    if user_path.is_empty() {
        return Err("path is empty".into());
    }
    if user_path.starts_with('/') || user_path.starts_with('\\') {
        return Err("path must be relative".into());
    }
    if user_path.contains("..") {
        return Err("path traversal not allowed".into());
    }

    // Directory names: alphanumeric, hyphens, underscores, dots, slashes
    if !user_path
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
    {
        return Err("invalid directory name".into());
    }

    let full = workspace.join(user_path);
    let canonical_ws = workspace
        .canonicalize()
        .map_err(|e| format!("workspace error: {e}"))?;

    // Check parent is inside workspace
    if let Some(parent) = full.parent() {
        if parent.exists() {
            let canonical_parent = parent
                .canonicalize()
                .map_err(|e| format!("parent error: {e}"))?;
            if !canonical_parent.starts_with(&canonical_ws) {
                return Err("path escapes workspace".into());
            }
        }
    }

    // If already exists, verify it's inside workspace
    if full.exists() {
        let canonical = full
            .canonicalize()
            .map_err(|e| format!("path error: {e}"))?;
        if !canonical.starts_with(&canonical_ws) {
            return Err("path escapes workspace".into());
        }
    }

    Ok(full)
}

pub struct Session {
    // `id` and `created_at` are kept as observability hooks for future
    // metrics / structured logging — the reaper currently identifies
    // sessions through the `DashMap` key, and touch()/TTL tracking uses
    // `last_activity` only. `#[allow(dead_code)]` here is intentional so
    // the struct fields stay in-sync with what any introspection tool
    // (ps, dashboards) would expect to find.
    #[allow(dead_code)]
    pub id: Uuid,
    pub workspace: PathBuf,
    #[allow(dead_code)]
    pub created_at: Instant,
    pub last_activity: Instant,
}

impl Session {
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }
}

#[derive(Clone)]
pub struct SessionStore {
    sessions: Arc<DashMap<Uuid, Session>>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    pub fn create(&self, template: Option<&str>) -> Result<(Uuid, PathBuf), String> {
        if self.sessions.len() >= MAX_SESSIONS {
            return Err("session limit reached (200)".into());
        }

        let id = Uuid::new_v4();
        let workspace = std::env::temp_dir()
            .join("ach-sessions")
            .join(id.to_string());
        std::fs::create_dir_all(&workspace).map_err(|e| format!("cannot create workspace: {e}"))?;

        // Populate template if requested
        if let Some(tpl) = template {
            crate::templates::populate_template(tpl, &workspace)?;
        }

        let session = Session {
            id,
            workspace: workspace.clone(),
            created_at: Instant::now(),
            last_activity: Instant::now(),
        };
        self.sessions.insert(id, session);
        Ok((id, workspace))
    }

    pub fn get_workspace(&self, id: Uuid) -> Result<PathBuf, String> {
        let mut entry = self.sessions.get_mut(&id).ok_or("session not found")?;
        entry.touch();
        Ok(entry.workspace.clone())
    }

    pub fn delete(&self, id: Uuid) -> Result<(), String> {
        if let Some((_, session)) = self.sessions.remove(&id) {
            let _ = std::fs::remove_dir_all(&session.workspace);
        }
        Ok(())
    }

    /// List files in workspace, enforcing size limits.
    pub fn list_files(&self, workspace: &Path) -> Result<Vec<FileEntry>, String> {
        let mut entries = Vec::new();
        collect_files(workspace, workspace, &mut entries)?;
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    /// Check workspace constraints before writing.
    pub fn check_write_limits(
        &self,
        workspace: &Path,
        file_path: &Path,
        content: &str,
    ) -> Result<(), String> {
        if content.len() > MAX_FILE_SIZE {
            return Err(format!(
                "file too large ({} bytes, max {})",
                content.len(),
                MAX_FILE_SIZE
            ));
        }

        // Count files and total size
        let mut file_count = 0usize;
        let mut total_size = 0usize;
        let mut entries = Vec::new();
        collect_files(workspace, workspace, &mut entries)?;

        for entry in &entries {
            file_count += 1;
            total_size += entry.size;
        }

        // If we're creating a new file (not updating existing)
        let is_new = !file_path.exists();
        if is_new && file_count >= MAX_FILES_PER_SESSION {
            return Err(format!(
                "too many files ({}, max {})",
                file_count, MAX_FILES_PER_SESSION
            ));
        }

        // Check total workspace size with the new content
        let old_size = if file_path.exists() {
            std::fs::metadata(file_path)
                .map(|m| m.len() as usize)
                .unwrap_or(0)
        } else {
            0
        };
        let new_total = total_size - old_size + content.len();
        if new_total > MAX_WORKSPACE_SIZE {
            return Err(format!(
                "workspace too large ({} bytes, max {})",
                new_total, MAX_WORKSPACE_SIZE
            ));
        }

        Ok(())
    }

    /// Spawn the TTL reaper background task.
    pub fn spawn_reaper(&self) {
        let sessions = Arc::clone(&self.sessions);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(REAPER_INTERVAL);
            loop {
                interval.tick().await;
                let mut expired = Vec::new();
                for entry in sessions.iter() {
                    if entry.last_activity.elapsed() > SESSION_TTL {
                        expired.push(*entry.key());
                    }
                }
                for id in expired {
                    if let Some((_, session)) = sessions.remove(&id) {
                        let _ = std::fs::remove_dir_all(&session.workspace);
                        tracing::info!("reaped session {id}");
                    }
                }
            }
        });
    }
}

#[derive(serde::Serialize, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size: usize,
    /// "file" or "dir" (empty directories only).
    #[serde(rename = "type")]
    pub entry_type: String,
}

fn collect_files(base: &Path, dir: &Path, entries: &mut Vec<FileEntry>) -> Result<(), String> {
    let read_dir = std::fs::read_dir(dir).map_err(|e| format!("read_dir: {e}"))?;
    for item in read_dir {
        let item = item.map_err(|e| format!("dir entry: {e}"))?;
        let path = item.path();
        if path.is_dir() {
            let child_count = std::fs::read_dir(&path).map(|rd| rd.count()).unwrap_or(0);
            if child_count == 0 {
                // Report empty directories so the client can show them
                let rel = path
                    .strip_prefix(base)
                    .map_err(|e| format!("strip_prefix: {e}"))?
                    .to_string_lossy()
                    .to_string();
                entries.push(FileEntry {
                    path: rel,
                    size: 0,
                    entry_type: "dir".into(),
                });
            } else {
                collect_files(base, &path, entries)?;
            }
        } else if path.is_file() {
            let rel = path
                .strip_prefix(base)
                .map_err(|e| format!("strip_prefix: {e}"))?
                .to_string_lossy()
                .to_string();
            let size = std::fs::metadata(&path)
                .map(|m| m.len() as usize)
                .unwrap_or(0);
            entries.push(FileEntry {
                path: rel,
                size,
                entry_type: "file".into(),
            });
        }
    }
    Ok(())
}
