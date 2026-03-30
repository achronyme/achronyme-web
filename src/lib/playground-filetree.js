// File tree component — VS Code-style explorer with folder hierarchy.
// Supports arbitrary nesting depth, drag-and-drop file moving, and directory ops.

/**
 * Build a recursive tree from a flat list of files.
 * Handles files with paths like "src/lib/utils.ach" at any depth.
 * Also handles entries with type "dir" (empty directories from server).
 */
function buildTree(files) {
  const root = [];
  const dirMap = {};

  function ensureDir(parts) {
    const key = parts.join("/");
    if (dirMap[key]) return dirMap[key];
    const node = { type: "dir", name: parts[parts.length - 1], path: key, children: [], expanded: true };
    dirMap[key] = node;
    if (parts.length === 1) {
      root.push(node);
    } else {
      const parent = ensureDir(parts.slice(0, -1));
      if (!parent.children.some(c => c.path === key)) {
        parent.children.push(node);
      }
    }
    return node;
  }

  for (const file of files) {
    const parts = file.path.split("/");

    // Empty directory entry from server
    if (file.type === "dir") {
      ensureDir(parts);
      continue;
    }

    if (parts.length === 1) {
      root.push({ type: "file", name: parts[0], path: file.path, size: file.size });
    } else {
      const dirParts = parts.slice(0, -1);
      const dir = ensureDir(dirParts);
      dir.children.push({
        type: "file",
        name: parts[parts.length - 1],
        path: file.path,
        size: file.size,
      });
    }
  }

  // Sort: achronyme.toml first, then dirs before files, then alphabetical
  const sortNodes = (nodes) => {
    nodes.sort((a, b) => {
      if (a.path === "achronyme.toml") return -1;
      if (b.path === "achronyme.toml") return 1;
      if (a.type !== b.type) return a.type === "dir" ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    for (const node of nodes) {
      if (node.type === "dir" && node.children) sortNodes(node.children);
    }
  };
  sortNodes(root);
  return root;
}

function fileIconSvg(name) {
  if (name.endsWith(".toml")) {
    return `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M8 1.5L2 4.5v7l6 3 6-3v-7L8 1.5z" stroke="#7d7d8a" stroke-width="1" fill="none"/><circle cx="8" cy="8" r="1.8" fill="#7d7d8a"/></svg>`;
  }
  return `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M8 2.5L12.5 8L8 13.5L3.5 8L8 2.5Z" stroke="#a855f7" stroke-width="1.2" fill="rgba(168,85,247,0.12)"/></svg>`;
}

function dirIconSvg(expanded) {
  if (expanded) {
    return `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M1.5 3.5h4.5l1.5 1.5H14.5V13h-13V3.5z" stroke="#c09553" stroke-width="1" fill="rgba(192,149,83,0.15)"/></svg>`;
  }
  return `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M1.5 3.5h4.5l1.5 1.5H14.5V13h-13V3.5z" stroke="#c09553" stroke-width="1" fill="rgba(192,149,83,0.05)"/></svg>`;
}

export function renderFileTree(container, files, activeFile, callbacks) {
  container.innerHTML = "";
  const tree = buildTree(files);
  renderNodes(container, tree, 0, activeFile, callbacks);
}

function renderNodes(container, nodes, depth, activeFile, callbacks) {
  for (const node of nodes) {
    if (node.type === "dir") {
      const row = document.createElement("div");
      row.className = "file-row dir-row";
      row.style.paddingLeft = (8 + depth * 12) + "px";
      row.dataset.dirPath = node.path;

      const chevron = document.createElement("span");
      chevron.className = "tree-chevron" + (node.expanded ? " expanded" : "");
      chevron.innerHTML = `<svg width="10" height="10" viewBox="0 0 10 10"><path d="M3 2L7 5L3 8" fill="none" stroke="currentColor" stroke-width="1.3"/></svg>`;

      const icon = document.createElement("span");
      icon.className = "file-icon";
      icon.innerHTML = dirIconSvg(node.expanded);

      const name = document.createElement("span");
      name.className = "file-name";
      name.textContent = node.name;

      row.appendChild(chevron);
      row.appendChild(icon);
      row.appendChild(name);

      const childContainer = document.createElement("div");
      childContainer.className = "tree-children";
      if (!node.expanded) childContainer.style.display = "none";

      row.addEventListener("click", (e) => {
        if (e.target.closest(".rename-input")) return;
        node.expanded = !node.expanded;
        chevron.classList.toggle("expanded");
        icon.innerHTML = dirIconSvg(node.expanded);
        childContainer.style.display = node.expanded ? "" : "none";
      });

      // Context menu for directories
      row.addEventListener("contextmenu", (e) => {
        e.preventDefault();
        showDirContextMenu(e.clientX, e.clientY, node.path, row, callbacks);
      });

      // Double-click to rename directory
      row.addEventListener("dblclick", (e) => {
        e.stopPropagation();
        startInlineDirRename(row, node.path, node.name, callbacks);
      });

      // ── Drag-and-drop: directory as drop target ──
      row.addEventListener("dragover", (e) => {
        if (!e.dataTransfer.types.includes("text/x-ach-filetree")) return;
        e.preventDefault();
        e.stopPropagation();
        e.dataTransfer.dropEffect = "move";
        row.classList.add("drop-target");
      });
      row.addEventListener("dragleave", (e) => {
        if (!row.contains(e.relatedTarget)) row.classList.remove("drop-target");
      });
      row.addEventListener("drop", (e) => {
        if (!e.dataTransfer.types.includes("text/x-ach-filetree")) return;
        e.preventDefault();
        e.stopPropagation();
        row.classList.remove("drop-target");
        const srcPath = e.dataTransfer.getData("text/x-ach-filetree");
        if (srcPath && callbacks.onFileMove) {
          const fileName = srcPath.split("/").pop();
          const newPath = node.path + "/" + fileName;
          if (newPath !== srcPath) callbacks.onFileMove(srcPath, newPath);
        }
      });

      // Also make child container a drop target (for dropping into expanded dir's space)
      childContainer.addEventListener("dragover", (e) => {
        if (!e.dataTransfer.types.includes("text/x-ach-filetree")) return;
        e.preventDefault();
        e.stopPropagation();
        e.dataTransfer.dropEffect = "move";
        row.classList.add("drop-target");
      });
      childContainer.addEventListener("dragleave", (e) => {
        if (!childContainer.contains(e.relatedTarget)) row.classList.remove("drop-target");
      });
      childContainer.addEventListener("drop", (e) => {
        if (!e.dataTransfer.types.includes("text/x-ach-filetree")) return;
        e.preventDefault();
        e.stopPropagation();
        row.classList.remove("drop-target");
        const srcPath = e.dataTransfer.getData("text/x-ach-filetree");
        if (srcPath && callbacks.onFileMove) {
          const fileName = srcPath.split("/").pop();
          const newPath = node.path + "/" + fileName;
          if (newPath !== srcPath) callbacks.onFileMove(srcPath, newPath);
        }
      });

      container.appendChild(row);
      renderNodes(childContainer, node.children, depth + 1, activeFile, callbacks);
      container.appendChild(childContainer);
    } else {
      const row = document.createElement("div");
      const isActive = node.path === activeFile;
      row.className = "file-row" + (isActive ? " active focused" : "");
      const indent = 8 + depth * 12 + (depth > 0 ? 18 : 0);
      row.style.paddingLeft = indent + "px";
      row.dataset.path = node.path;

      const icon = document.createElement("span");
      icon.className = "file-icon";
      icon.innerHTML = fileIconSvg(node.path);

      const name = document.createElement("span");
      name.className = "file-name";
      name.textContent = node.name;

      row.appendChild(icon);
      row.appendChild(name);

      // ── Drag source: make files draggable ──
      row.draggable = true;
      row.addEventListener("dragstart", (e) => {
        e.dataTransfer.setData("text/x-ach-filetree", node.path);
        e.dataTransfer.effectAllowed = "move";
        row.classList.add("dragging");
      });
      row.addEventListener("dragend", () => {
        row.classList.remove("dragging");
        // Clean up any lingering drop targets
        container.closest(".file-tree")?.querySelectorAll(".drop-target").forEach(r => r.classList.remove("drop-target"));
      });

      row.addEventListener("click", () => {
        container.closest(".file-tree").querySelectorAll(".file-row.focused").forEach(r => r.classList.remove("focused"));
        row.classList.add("focused");
        callbacks.onFileClick(node.path);
      });

      if (node.path !== "achronyme.toml") {
        row.addEventListener("contextmenu", (e) => {
          e.preventDefault();
          showContextMenu(e.clientX, e.clientY, node.path, row, callbacks);
        });
        // Double-click to rename
        row.addEventListener("dblclick", (e) => {
          e.stopPropagation();
          startInlineRename(row, node.path, node.name, callbacks);
        });
      }

      container.appendChild(row);
    }
  }
}

// ── Drop on root (move file to root level) ──
export function setupRootDrop(container, callbacks) {
  container.addEventListener("dragover", (e) => {
    if (!e.dataTransfer.types.includes("text/x-ach-filetree")) return;
    // Only if dropping on the container itself (not on a child row)
    if (e.target !== container && e.target.closest(".file-row")) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    container.classList.add("drop-target-root");
  });
  container.addEventListener("dragleave", (e) => {
    if (!container.contains(e.relatedTarget)) container.classList.remove("drop-target-root");
  });
  container.addEventListener("drop", (e) => {
    if (!e.dataTransfer.types.includes("text/x-ach-filetree")) return;
    // Only if dropping on the container itself
    if (e.target !== container && e.target.closest(".file-row")) return;
    e.preventDefault();
    container.classList.remove("drop-target-root");
    const srcPath = e.dataTransfer.getData("text/x-ach-filetree");
    if (srcPath && callbacks.onFileMove) {
      const fileName = srcPath.split("/").pop();
      // Move to root
      if (fileName !== srcPath) callbacks.onFileMove(srcPath, fileName);
    }
  });
}

/** Start inline rename — replaces file name with an input field. */
function startInlineRename(row, path, currentName, callbacks) {
  const nameEl = row.querySelector(".file-name");
  if (!nameEl) return;

  const input = document.createElement("input");
  input.className = "rename-input";
  input.type = "text";
  input.value = currentName;

  // Compute new path helper
  const dir = path.substring(0, path.length - currentName.length);

  nameEl.style.display = "none";
  row.insertBefore(input, nameEl.nextSibling);
  input.focus();
  // Select name without extension
  const dotIdx = currentName.lastIndexOf(".");
  input.setSelectionRange(0, dotIdx > 0 ? dotIdx : currentName.length);

  const commit = () => {
    const newName = input.value.trim();
    input.remove();
    nameEl.style.display = "";
    if (newName && newName !== currentName) {
      const newPath = dir + newName;
      callbacks.onFileRename(path, newPath);
    }
  };

  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") { e.preventDefault(); commit(); }
    if (e.key === "Escape") { input.remove(); nameEl.style.display = ""; }
  });
  input.addEventListener("blur", commit);
}

/** Start inline rename for a directory. */
function startInlineDirRename(row, path, currentName, callbacks) {
  const nameEl = row.querySelector(".file-name");
  if (!nameEl) return;

  const input = document.createElement("input");
  input.className = "rename-input";
  input.type = "text";
  input.value = currentName;

  const dir = path.substring(0, path.length - currentName.length);

  nameEl.style.display = "none";
  row.insertBefore(input, nameEl.nextSibling);
  input.focus();
  input.select();

  const commit = () => {
    const newName = input.value.trim();
    input.remove();
    nameEl.style.display = "";
    if (newName && newName !== currentName && callbacks.onDirRename) {
      const newPath = dir ? dir + newName : newName;
      callbacks.onDirRename(path, newPath);
    }
  };

  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") { e.preventDefault(); commit(); }
    if (e.key === "Escape") { input.remove(); nameEl.style.display = ""; }
  });
  input.addEventListener("blur", commit);
}

function showContextMenu(x, y, path, row, callbacks) {
  const old = document.getElementById("file-context-menu");
  if (old) old.remove();

  const menu = document.createElement("div");
  menu.id = "file-context-menu";
  menu.className = "context-menu";
  menu.style.left = x + "px";
  menu.style.top = y + "px";

  const items = [
    {
      label: "Rename",
      shortcut: "F2",
      action: () => {
        const name = path.split("/").pop();
        startInlineRename(row, path, name, callbacks);
      },
    },
    { label: "Delete", shortcut: "Del", cls: "danger", action: () => callbacks.onFileDelete(path) },
  ];

  for (const item of items) {
    const el = document.createElement("div");
    el.className = "context-item" + (item.cls ? ` ${item.cls}` : "");

    const label = document.createElement("span");
    label.textContent = item.label;
    el.appendChild(label);

    if (item.shortcut) {
      const sc = document.createElement("span");
      sc.className = "context-shortcut";
      sc.textContent = item.shortcut;
      el.appendChild(sc);
    }

    el.addEventListener("click", () => { menu.remove(); item.action(); });
    menu.appendChild(el);
  }

  document.body.appendChild(menu);
  const close = (e) => {
    if (!menu.contains(e.target)) { menu.remove(); document.removeEventListener("click", close); }
  };
  setTimeout(() => document.addEventListener("click", close), 0);
}

/** Context menu for directories. */
function showDirContextMenu(x, y, dirPath, row, callbacks) {
  const old = document.getElementById("file-context-menu");
  if (old) old.remove();

  const menu = document.createElement("div");
  menu.id = "file-context-menu";
  menu.className = "context-menu";
  menu.style.left = x + "px";
  menu.style.top = y + "px";

  const items = [
    {
      label: "New File",
      action: () => {
        if (callbacks.onNewFileInDir) callbacks.onNewFileInDir(dirPath);
      },
    },
    {
      label: "New Folder",
      action: () => {
        if (callbacks.onNewDirInDir) callbacks.onNewDirInDir(dirPath);
      },
    },
    {
      label: "Rename",
      shortcut: "F2",
      action: () => {
        const name = dirPath.split("/").pop();
        startInlineDirRename(row, dirPath, name, callbacks);
      },
    },
    {
      label: "Delete",
      shortcut: "Del",
      cls: "danger",
      action: () => {
        if (callbacks.onDirDelete) callbacks.onDirDelete(dirPath);
      },
    },
  ];

  for (const item of items) {
    const el = document.createElement("div");
    el.className = "context-item" + (item.cls ? ` ${item.cls}` : "");

    const label = document.createElement("span");
    label.textContent = item.label;
    el.appendChild(label);

    if (item.shortcut) {
      const sc = document.createElement("span");
      sc.className = "context-shortcut";
      sc.textContent = item.shortcut;
      el.appendChild(sc);
    }

    el.addEventListener("click", () => { menu.remove(); item.action(); });
    menu.appendChild(el);
  }

  document.body.appendChild(menu);
  const close = (e) => {
    if (!menu.contains(e.target)) { menu.remove(); document.removeEventListener("click", close); }
  };
  setTimeout(() => document.addEventListener("click", close), 0);
}

/**
 * Insert an inline input row at the end of the file tree (VS Code-style).
 * Calls callbacks.onCreate(name) on commit, does nothing on cancel.
 */
export function startInlineCreate(container, callbacks, parentDir) {
  // Prevent double input
  if (container.querySelector(".create-input-row")) return;

  const row = document.createElement("div");
  row.className = "file-row create-input-row";
  row.style.paddingLeft = parentDir ? "32px" : "8px";

  const icon = document.createElement("span");
  icon.className = "file-icon";
  icon.innerHTML = fileIconSvg(".ach");

  const input = document.createElement("input");
  input.className = "rename-input";
  input.type = "text";
  input.placeholder = "file.ach";

  row.appendChild(icon);
  row.appendChild(input);

  // If creating inside a directory, find the directory's child container
  if (parentDir) {
    const dirRow = container.querySelector(`[data-dir-path="${CSS.escape(parentDir)}"]`);
    if (dirRow) {
      const childContainer = dirRow.nextElementSibling;
      if (childContainer && childContainer.classList.contains("tree-children")) {
        childContainer.style.display = "";
        childContainer.appendChild(row);
      } else {
        container.appendChild(row);
      }
    } else {
      container.appendChild(row);
    }
  } else {
    container.appendChild(row);
  }
  input.focus();

  let done = false;
  const finish = (cancelled) => {
    if (done) return;
    done = true;
    row.remove();
    if (cancelled) return;
    let name = input.value.trim();
    if (!name) return;
    if (!name.endsWith(".ach") && !name.endsWith(".toml")) name += ".ach";
    const fullPath = parentDir ? parentDir + "/" + name : name;
    callbacks.onCreate(fullPath);
  };

  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") { e.preventDefault(); finish(false); }
    if (e.key === "Escape") { e.preventDefault(); finish(true); }
  });
  input.addEventListener("blur", () => setTimeout(() => finish(false), 0));
}

/**
 * Insert an inline input row for creating a directory.
 */
export function startInlineCreateDir(container, callbacks, parentDir) {
  if (container.querySelector(".create-input-row")) return;

  const row = document.createElement("div");
  row.className = "file-row create-input-row";
  row.style.paddingLeft = parentDir ? "32px" : "8px";

  const icon = document.createElement("span");
  icon.className = "file-icon";
  icon.innerHTML = dirIconSvg(false);

  const input = document.createElement("input");
  input.className = "rename-input";
  input.type = "text";
  input.placeholder = "folder";

  row.appendChild(icon);
  row.appendChild(input);

  if (parentDir) {
    const dirRow = container.querySelector(`[data-dir-path="${CSS.escape(parentDir)}"]`);
    if (dirRow) {
      const childContainer = dirRow.nextElementSibling;
      if (childContainer && childContainer.classList.contains("tree-children")) {
        childContainer.style.display = "";
        childContainer.appendChild(row);
      } else {
        container.appendChild(row);
      }
    } else {
      container.appendChild(row);
    }
  } else {
    container.appendChild(row);
  }
  input.focus();

  let done = false;
  const finish = (cancelled) => {
    if (done) return;
    done = true;
    row.remove();
    if (cancelled) return;
    const name = input.value.trim();
    if (!name) return;
    const fullPath = parentDir ? parentDir + "/" + name : name;
    callbacks.onCreateDir(fullPath);
  };

  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") { e.preventDefault(); finish(false); }
    if (e.key === "Escape") { e.preventDefault(); finish(true); }
  });
  input.addEventListener("blur", () => setTimeout(() => finish(false), 0));
}
