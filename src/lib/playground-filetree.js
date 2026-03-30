// File tree component — VS Code-style explorer with folder hierarchy.

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
      row.style.paddingLeft = (8 + depth * 8) + "px";

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

      row.addEventListener("click", () => {
        node.expanded = !node.expanded;
        chevron.classList.toggle("expanded");
        icon.innerHTML = dirIconSvg(node.expanded);
        childContainer.style.display = node.expanded ? "" : "none";
      });

      container.appendChild(row);
      renderNodes(childContainer, node.children, depth + 1, activeFile, callbacks);
      container.appendChild(childContainer);
    } else {
      const row = document.createElement("div");
      const isActive = node.path === activeFile;
      row.className = "file-row" + (isActive ? " active focused" : "");
      // Files inside dirs get extra indent for missing chevron
      const indent = 8 + depth * 8 + (depth > 0 ? 18 : 0);
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

      row.addEventListener("click", () => {
        // Remove focused from all rows
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

/**
 * Insert an inline input row at the end of the file tree (VS Code-style).
 * Calls callbacks.onCreate(name) on commit, does nothing on cancel.
 */
export function startInlineCreate(container, callbacks) {
  // Prevent double input
  if (container.querySelector(".create-input-row")) return;

  const row = document.createElement("div");
  row.className = "file-row create-input-row";
  row.style.paddingLeft = "8px";

  const icon = document.createElement("span");
  icon.className = "file-icon";
  icon.innerHTML = fileIconSvg(".ach");

  const input = document.createElement("input");
  input.className = "rename-input";
  input.type = "text";
  input.placeholder = "file.ach";

  row.appendChild(icon);
  row.appendChild(input);
  container.appendChild(row);
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
    callbacks.onCreate(name);
  };

  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") { e.preventDefault(); finish(false); }
    if (e.key === "Escape") { e.preventDefault(); finish(true); }
  });
  input.addEventListener("blur", () => setTimeout(() => finish(false), 0));
}
