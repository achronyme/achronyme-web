// File tree component for the playground mini-IDE.
// Renders a VS Code-like explorer with folder hierarchy.

/**
 * Build a tree structure from flat file list.
 * @param {Array<{path: string, size: number}>} files
 * @returns {Array} Tree nodes
 */
function buildTree(files) {
  const root = [];
  const dirs = {};

  for (const file of files) {
    const parts = file.path.split("/");
    if (parts.length === 1) {
      root.push({ type: "file", name: parts[0], path: file.path, size: file.size });
    } else {
      const dirName = parts[0];
      if (!dirs[dirName]) {
        dirs[dirName] = { type: "dir", name: dirName, children: [], expanded: true };
        root.push(dirs[dirName]);
      }
      dirs[dirName].children.push({
        type: "file",
        name: parts.slice(1).join("/"),
        path: file.path,
        size: file.size,
      });
    }
  }

  // Sort: dirs first, then files. achronyme.toml always first.
  root.sort((a, b) => {
    if (a.path === "achronyme.toml") return -1;
    if (b.path === "achronyme.toml") return 1;
    if (a.type !== b.type) return a.type === "dir" ? -1 : 1;
    return a.name.localeCompare(b.name);
  });

  for (const dir of Object.values(dirs)) {
    dir.children.sort((a, b) => a.name.localeCompare(b.name));
  }

  return root;
}

/**
 * Get file icon SVG based on file type.
 */
function fileIcon(name) {
  if (name === "achronyme.toml") {
    return `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M8 1.5L2 4.5v7l6 3 6-3v-7L8 1.5z" stroke="#8b8b9a" stroke-width="1.2" fill="none"/><circle cx="8" cy="8" r="2" fill="#8b8b9a"/></svg>`;
  }
  // .ach files get the diamond icon in accent color
  return `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M8 2L13 8L8 14L3 8L8 2Z" fill="none" stroke="#a855f7" stroke-width="1.3"/><path d="M8 5L10.5 8L8 11L5.5 8L8 5Z" fill="#a855f7" opacity="0.3"/></svg>`;
}

function dirIcon(expanded) {
  if (expanded) {
    return `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M1.5 3h5l1.5 1.5H14.5v9h-13V3z" stroke="#c4a24d" stroke-width="1.1" fill="rgba(196,162,77,0.12)"/><path d="M1.5 6.5h13" stroke="#c4a24d" stroke-width="0.8" opacity="0.5"/></svg>`;
  }
  return `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M1.5 3h5l1.5 1.5H14.5v9h-13V3z" stroke="#c4a24d" stroke-width="1.1" fill="rgba(196,162,77,0.08)"/></svg>`;
}

/**
 * Render the file tree into a container element.
 */
export function renderFileTree(container, files, activeFile, callbacks) {
  container.innerHTML = "";
  const tree = buildTree(files);
  renderNodes(container, tree, 0, activeFile, callbacks);
}

function renderNodes(container, nodes, depth, activeFile, callbacks) {
  for (const node of nodes) {
    if (node.type === "dir") {
      // Directory row
      const row = document.createElement("div");
      row.className = "file-row dir-row";
      row.style.paddingLeft = (12 + depth * 16) + "px";

      const chevron = document.createElement("span");
      chevron.className = "tree-chevron" + (node.expanded ? " expanded" : "");
      chevron.innerHTML = `<svg width="10" height="10" viewBox="0 0 10 10"><path d="M3 2L7 5L3 8" fill="none" stroke="currentColor" stroke-width="1.3"/></svg>`;

      const icon = document.createElement("span");
      icon.className = "file-icon";
      icon.innerHTML = dirIcon(node.expanded);

      const name = document.createElement("span");
      name.className = "file-name";
      name.textContent = node.name;

      row.appendChild(chevron);
      row.appendChild(icon);
      row.appendChild(name);

      const childContainer = document.createElement("div");
      childContainer.className = "tree-children";
      if (!node.expanded) childContainer.style.display = "none";

      row.addEventListener("click", () => {
        node.expanded = !node.expanded;
        chevron.classList.toggle("expanded");
        icon.innerHTML = dirIcon(node.expanded);
        childContainer.style.display = node.expanded ? "" : "none";
      });

      container.appendChild(row);
      renderNodes(childContainer, node.children, depth + 1, activeFile, callbacks);
      container.appendChild(childContainer);
    } else {
      // File row
      const row = document.createElement("div");
      row.className = "file-row" + (node.path === activeFile ? " active" : "");
      row.style.paddingLeft = (12 + depth * 16 + (depth > 0 ? 0 : 0)) + "px";
      row.dataset.path = node.path;

      // Indent spacer for files inside directories (no chevron)
      if (depth > 0) {
        const spacer = document.createElement("span");
        spacer.style.width = "10px";
        spacer.style.flexShrink = "0";
        row.appendChild(spacer);
      }

      const icon = document.createElement("span");
      icon.className = "file-icon";
      icon.innerHTML = fileIcon(node.path);

      const name = document.createElement("span");
      name.className = "file-name";
      name.textContent = node.name;
      name.title = node.path;

      row.appendChild(icon);
      row.appendChild(name);

      row.addEventListener("click", () => callbacks.onFileClick(node.path));

      if (node.path !== "achronyme.toml") {
        row.addEventListener("contextmenu", (e) => {
          e.preventDefault();
          showContextMenu(e.clientX, e.clientY, node.path, callbacks);
        });
      }

      container.appendChild(row);
    }
  }
}

function showContextMenu(x, y, path, callbacks) {
  const old = document.getElementById("file-context-menu");
  if (old) old.remove();

  const menu = document.createElement("div");
  menu.id = "file-context-menu";
  menu.className = "context-menu";
  menu.style.left = x + "px";
  menu.style.top = y + "px";

  const items = [
    { label: "Rename", action: () => callbacks.onFileRename(path) },
    { label: "Delete", cls: "danger", action: () => callbacks.onFileDelete(path) },
  ];

  for (const item of items) {
    const el = document.createElement("div");
    el.className = "context-item" + (item.cls ? ` ${item.cls}` : "");
    el.textContent = item.label;
    el.addEventListener("click", () => { menu.remove(); item.action(); });
    menu.appendChild(el);
  }

  document.body.appendChild(menu);
  const close = (e) => {
    if (!menu.contains(e.target)) { menu.remove(); document.removeEventListener("click", close); }
  };
  setTimeout(() => document.addEventListener("click", close), 0);
}

export function promptNewFile() {
  const name = prompt("New file name (e.g. src/helpers.ach):");
  if (!name) return null;
  if (!name.endsWith(".ach")) return name + ".ach";
  return name;
}
