// Session API client for the playground mini-IDE.
// Manages workspace sessions, file CRUD, persistence, and run operations.

const IS_DEV = typeof window !== 'undefined' && window.location.port === "4321";
const API_BASE = IS_DEV ? "http://localhost:3100" : "https://play.achrony.me";
const STORAGE_KEY = "ach-session";
const PROJECT_KEY = "ach-project-backup";

let sessionId = null;

function headers() {
  const h = { "Content-Type": "application/json" };
  if (sessionId) h["X-Ach-Session"] = sessionId;
  return h;
}

/** Create a new session, optionally from a template. */
export async function createSession(template) {
  const res = await fetch(`${API_BASE}/api/session/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ template: template || null }),
  });
  if (!res.ok) throw new Error(`session create failed: ${res.status}`);
  const data = await res.json();
  sessionId = data.session_id;
  localStorage.setItem(STORAGE_KEY, sessionId);
  // Backup project files to localStorage
  scheduleBackup();
  return data;
}

/** Restore session — tries localStorage, validates with server. */
export async function restoreSession() {
  sessionId = localStorage.getItem(STORAGE_KEY);
  if (!sessionId) return false;

  // Check if the server still has this session
  try {
    const res = await fetch(`${API_BASE}/api/fs/list`, {
      method: "GET",
      headers: headers(),
    });
    if (res.ok) return true;
  } catch {}

  // Server lost the session — try to restore from backup
  const backup = localStorage.getItem(PROJECT_KEY);
  if (backup) {
    try {
      return await restoreFromBackup(JSON.parse(backup));
    } catch {}
  }

  // Nothing to restore
  sessionId = null;
  localStorage.removeItem(STORAGE_KEY);
  return false;
}

/** Get current session ID. */
export function getSessionId() {
  return sessionId;
}

/** Check if we're in project mode (have a session). */
export function isProjectMode() {
  return !!sessionId;
}

/** Delete the current session. */
export async function deleteSession() {
  if (!sessionId) return;
  await fetch(`${API_BASE}/api/session`, {
    method: "DELETE",
    headers: headers(),
  }).catch(() => {});
  sessionId = null;
  localStorage.removeItem(STORAGE_KEY);
  localStorage.removeItem(PROJECT_KEY);
}

/** Write a file to the workspace. */
export async function writeFile(path, content) {
  const res = await fetch(`${API_BASE}/api/fs/write`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify({ path, content }),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.error || `write failed: ${res.status}`);
  }
  scheduleBackup();
}

/** Read a file from the workspace. */
export async function readFile(path) {
  const res = await fetch(`${API_BASE}/api/fs/read`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify({ path }),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.error || `read failed: ${res.status}`);
  }
  const data = await res.json();
  return data.content;
}

/** List all files in the workspace. */
export async function listFiles() {
  const res = await fetch(`${API_BASE}/api/fs/list`, {
    method: "GET",
    headers: headers(),
  });
  if (!res.ok) throw new Error(`list failed: ${res.status}`);
  const data = await res.json();
  return data.files;
}

/** Delete a file from the workspace. */
export async function deleteFile(path) {
  const res = await fetch(`${API_BASE}/api/fs/delete`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify({ path }),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.error || `delete failed: ${res.status}`);
  }
  scheduleBackup();
}

/** Create a directory. */
export async function mkdir(path) {
  const res = await fetch(`${API_BASE}/api/fs/mkdir`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify({ path }),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.error || `mkdir failed: ${res.status}`);
  }
  scheduleBackup();
}

/** Rename a file. */
export async function renameFile(from, to) {
  const res = await fetch(`${API_BASE}/api/fs/rename`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify({ from, to }),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.error || `rename failed: ${res.status}`);
  }
  scheduleBackup();
}

/** Run the workspace project (or single source in non-project mode). */
export async function runProject(source) {
  const body = sessionId ? {} : { source };
  const res = await fetch(`${API_BASE}/api/run`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: `HTTP ${res.status}` }));
    throw new Error(err.error || `run failed: ${res.status}`);
  }
  return res.json();
}

/** Health check. */
export async function healthCheck() {
  const res = await fetch(`${API_BASE}/health`);
  return res.ok;
}

// ── Backup / Restore ────────────────────────────────────

let backupTimer = null;

/** Schedule a backup to localStorage (debounced 3s). */
function scheduleBackup() {
  clearTimeout(backupTimer);
  backupTimer = setTimeout(doBackup, 3000);
}

/** Read all files from server and save to localStorage. */
async function doBackup() {
  if (!sessionId) return;
  try {
    const fileList = await listFiles();
    const files = {};
    for (const f of fileList) {
      files[f.path] = await readFile(f.path);
    }
    localStorage.setItem(PROJECT_KEY, JSON.stringify({ files, ts: Date.now() }));
  } catch {}
}

/** Create a new session and populate it from a backup. */
async function restoreFromBackup(backup) {
  if (!backup?.files) return false;
  const res = await fetch(`${API_BASE}/api/session/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({}),
  });
  if (!res.ok) return false;
  const data = await res.json();
  sessionId = data.session_id;
  localStorage.setItem(STORAGE_KEY, sessionId);

  for (const [path, content] of Object.entries(backup.files)) {
    await writeFile(path, content);
  }
  return true;
}

// ── Export / Import ─────────────────────────────────────

/** Export project as a downloadable JSON file. */
export async function exportProject() {
  if (!sessionId) throw new Error("no active project");
  const fileList = await listFiles();
  const files = {};
  for (const f of fileList) {
    files[f.path] = await readFile(f.path);
  }
  const blob = new Blob(
    [JSON.stringify({ files, exportedAt: new Date().toISOString() }, null, 2)],
    { type: "application/json" }
  );
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "project.ach.json";
  a.click();
  URL.revokeObjectURL(url);
}

/** Import a project from a JSON file. Returns the file list. */
export async function importProject(jsonString) {
  const data = JSON.parse(jsonString);
  if (!data?.files || typeof data.files !== "object") {
    throw new Error("invalid project file");
  }

  // Create a fresh session
  if (sessionId) await deleteSession();
  const res = await fetch(`${API_BASE}/api/session/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({}),
  });
  if (!res.ok) throw new Error("session create failed");
  const session = await res.json();
  sessionId = session.session_id;
  localStorage.setItem(STORAGE_KEY, sessionId);

  // Write all files
  for (const [path, content] of Object.entries(data.files)) {
    await writeFile(path, content);
  }

  return await listFiles();
}
