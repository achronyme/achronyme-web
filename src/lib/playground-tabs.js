// Tab management for the playground mini-IDE.
// Class-based TabManager — supports multiple instances for split view.

import { EditorState } from "@codemirror/state";

/**
 * @typedef {Object} Tab
 * @property {string} path - File path
 * @property {boolean} modified - Has unsaved changes
 * @property {any} editorState - Saved CodeMirror EditorState
 * @property {string} savedContent - Last saved content (for modified detection)
 */

export class TabManager {
  constructor() {
    /** @type {Tab[]} */
    this.tabs = [];
    this.activeIndex = -1;
  }

  /** Get current tabs array (read-only). */
  getTabs() { return this.tabs; }

  /** Get active tab index. */
  getActiveIndex() { return this.activeIndex; }

  /** Get the active tab, or null. */
  getActiveTab() { return this.activeIndex >= 0 ? this.tabs[this.activeIndex] : null; }

  /**
   * Open a file in a tab. If already open, switches to it.
   * @returns {number} The tab index
   */
  openTab(path, content, extensions) {
    const existing = this.tabs.findIndex((t) => t.path === path);
    if (existing >= 0) {
      this.activeIndex = existing;
      return existing;
    }
    const editorState = EditorState.create({ doc: content, extensions });
    this.tabs.push({ path, modified: false, editorState, savedContent: content });
    this.activeIndex = this.tabs.length - 1;
    return this.activeIndex;
  }

  /** Close a tab by index. Returns the new active index. */
  closeTab(index) {
    if (index < 0 || index >= this.tabs.length) return this.activeIndex;
    this.tabs.splice(index, 1);
    if (this.tabs.length === 0) {
      this.activeIndex = -1;
    } else if (this.activeIndex >= this.tabs.length) {
      this.activeIndex = this.tabs.length - 1;
    } else if (this.activeIndex > index) {
      this.activeIndex--;
    }
    return this.activeIndex;
  }

  /** Switch to a tab by index. */
  switchTab(index) {
    if (index >= 0 && index < this.tabs.length) this.activeIndex = index;
    return this.activeIndex;
  }

  /** Save the current editor state for the active tab. */
  saveEditorState(editorState) {
    if (this.activeIndex >= 0) this.tabs[this.activeIndex].editorState = editorState;
  }

  /** Mark a tab as modified or saved. */
  setModified(index, modified) {
    if (index >= 0 && index < this.tabs.length) this.tabs[index].modified = modified;
  }

  /** Update saved content after a successful write. */
  markSaved(index, content) {
    if (index >= 0 && index < this.tabs.length) {
      this.tabs[index].savedContent = content;
      this.tabs[index].modified = false;
    }
  }

  /** Check if content differs from saved. */
  isModified(index, currentContent) {
    if (index < 0 || index >= this.tabs.length) return false;
    return currentContent !== this.tabs[index].savedContent;
  }

  /** Find tab index by path. Returns -1 if not found. */
  findTab(path) { return this.tabs.findIndex((t) => t.path === path); }

  /** Update a tab's path (for rename). */
  updateTabPath(oldPath, newPath) {
    const idx = this.findTab(oldPath);
    if (idx >= 0) this.tabs[idx].path = newPath;
  }

  /** Remove a tab by path (for file deletion). */
  removeTabByPath(path) {
    const idx = this.findTab(path);
    if (idx >= 0) return this.closeTab(idx);
    return this.activeIndex;
  }

  /** Get all modified tabs. */
  getModifiedTabs() {
    return this.tabs.filter((t) => t.modified).map((t, i) => ({ ...t, index: i }));
  }

  /** Reset all tabs (for session change). */
  resetTabs() {
    this.tabs = [];
    this.activeIndex = -1;
  }
}
