// Session API client for the playground mini-IDE.
// Manages workspace sessions, file CRUD, and run operations.

const IS_DEV = typeof window !== 'undefined' && window.location.port === "4321";
const API_BASE = IS_DEV ? "http://localhost:3100" : "";

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
  sessionStorage.setItem("ach-session", sessionId);
  return data;
}

/** Restore session from sessionStorage if available. */
export function restoreSession() {
  sessionId = sessionStorage.getItem("ach-session");
  return sessionId;
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
  });
  sessionId = null;
  sessionStorage.removeItem("ach-session");
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
