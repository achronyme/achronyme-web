// Tab management for the playground mini-IDE.
// Handles multiple open files with per-file editor state preservation.

import { EditorState } from "@codemirror/state";

/**
 * @typedef {Object} Tab
 * @property {string} path - File path
 * @property {boolean} modified - Has unsaved changes
 * @property {any} editorState - Saved CodeMirror EditorState
 * @property {string} savedContent - Last saved content (for modified detection)
 */

/** @type {Tab[]} */
let tabs = [];
let activeIndex = -1;

/** Get current tabs array (read-only). */
export function getTabs() {
  return tabs;
}

/** Get active tab index. */
export function getActiveIndex() {
  return activeIndex;
}

/** Get the active tab, or null. */
export function getActiveTab() {
  return activeIndex >= 0 ? tabs[activeIndex] : null;
}

/**
 * Open a file in a tab. If already open, switches to it.
 * @returns {number} The tab index
 */
export function openTab(path, content, extensions) {
  const existing = tabs.findIndex((t) => t.path === path);
  if (existing >= 0) {
    activeIndex = existing;
    return existing;
  }

  const editorState = EditorState.create({ doc: content, extensions });
  tabs.push({
    path,
    modified: false,
    editorState,
    savedContent: content,
  });
  activeIndex = tabs.length - 1;
  return activeIndex;
}

/**
 * Close a tab by index. Returns the new active index.
 */
export function closeTab(index) {
  if (index < 0 || index >= tabs.length) return activeIndex;
  tabs.splice(index, 1);
  if (tabs.length === 0) {
    activeIndex = -1;
  } else if (activeIndex >= tabs.length) {
    activeIndex = tabs.length - 1;
  } else if (activeIndex > index) {
    activeIndex--;
  }
  return activeIndex;
}

/** Switch to a tab by index. */
export function switchTab(index) {
  if (index >= 0 && index < tabs.length) {
    activeIndex = index;
  }
  return activeIndex;
}

/** Save the current editor state for the active tab. */
export function saveEditorState(editorState) {
  if (activeIndex >= 0) {
    tabs[activeIndex].editorState = editorState;
  }
}

/** Mark a tab as modified or saved. */
export function setModified(index, modified) {
  if (index >= 0 && index < tabs.length) {
    tabs[index].modified = modified;
  }
}

/** Update saved content after a successful write. */
export function markSaved(index, content) {
  if (index >= 0 && index < tabs.length) {
    tabs[index].savedContent = content;
    tabs[index].modified = false;
  }
}

/** Check if content differs from saved. */
export function isModified(index, currentContent) {
  if (index < 0 || index >= tabs.length) return false;
  return currentContent !== tabs[index].savedContent;
}

/** Find tab index by path. Returns -1 if not found. */
export function findTab(path) {
  return tabs.findIndex((t) => t.path === path);
}

/** Update a tab's path (for rename). */
export function updateTabPath(oldPath, newPath) {
  const idx = findTab(oldPath);
  if (idx >= 0) {
    tabs[idx].path = newPath;
  }
}

/** Remove a tab by path (for file deletion). */
export function removeTabByPath(path) {
  const idx = findTab(path);
  if (idx >= 0) return closeTab(idx);
  return activeIndex;
}

/** Get all modified tabs. */
export function getModifiedTabs() {
  return tabs.filter((t) => t.modified).map((t, i) => ({ ...t, index: i }));
}

/** Reset all tabs (for session change). */
export function resetTabs() {
  tabs = [];
  activeIndex = -1;
}
