// File tree component for the playground mini-IDE.
// Renders a VS Code-like explorer sidebar.

/**
 * Render the file tree into a container element.
 * @param {HTMLElement} container
 * @param {Array<{path: string, size: number}>} files
 * @param {string} activeFile - currently open file path
 * @param {object} callbacks - { onFileClick, onFileDelete, onFileRename }
 */
export function renderFileTree(container, files, activeFile, callbacks) {
  container.innerHTML = "";

  // Sort: achronyme.toml first, then alphabetically
  const sorted = [...files].sort((a, b) => {
    if (a.path === "achronyme.toml") return -1;
    if (b.path === "achronyme.toml") return 1;
    return a.path.localeCompare(b.path);
  });

  for (const file of sorted) {
    const row = document.createElement("div");
    row.className = "file-row" + (file.path === activeFile ? " active" : "");
    row.dataset.path = file.path;

    const icon = document.createElement("span");
    icon.className = "file-icon";
    icon.textContent = file.path === "achronyme.toml" ? "\u2699" : "\u25C7"; // gear or diamond

    const name = document.createElement("span");
    name.className = "file-name";
    name.textContent = file.path;
    name.title = file.path;

    row.appendChild(icon);
    row.appendChild(name);

    // Click to open
    row.addEventListener("click", () => {
      callbacks.onFileClick(file.path);
    });

    // Right-click context menu
    if (file.path !== "achronyme.toml") {
      row.addEventListener("contextmenu", (e) => {
        e.preventDefault();
        showContextMenu(e.clientX, e.clientY, file.path, callbacks);
      });
    }

    container.appendChild(row);
  }
}

/** Show a minimal context menu for file operations. */
function showContextMenu(x, y, path, callbacks) {
  // Remove existing menu
  const old = document.getElementById("file-context-menu");
  if (old) old.remove();

  const menu = document.createElement("div");
  menu.id = "file-context-menu";
  menu.className = "context-menu";
  menu.style.left = x + "px";
  menu.style.top = y + "px";

  const renameItem = document.createElement("div");
  renameItem.className = "context-item";
  renameItem.textContent = "Rename";
  renameItem.addEventListener("click", () => {
    menu.remove();
    callbacks.onFileRename(path);
  });

  const deleteItem = document.createElement("div");
  deleteItem.className = "context-item danger";
  deleteItem.textContent = "Delete";
  deleteItem.addEventListener("click", () => {
    menu.remove();
    callbacks.onFileDelete(path);
  });

  menu.appendChild(renameItem);
  menu.appendChild(deleteItem);
  document.body.appendChild(menu);

  // Close on click outside
  const close = (e) => {
    if (!menu.contains(e.target)) {
      menu.remove();
      document.removeEventListener("click", close);
    }
  };
  setTimeout(() => document.addEventListener("click", close), 0);
}

/**
 * Prompt for a new file name.
 * @returns {string|null} The file name or null if cancelled
 */
export function promptNewFile() {
  const name = prompt("New file name (e.g. src/helpers.ach):");
  if (!name) return null;
  // Ensure .ach extension
  if (!name.endsWith(".ach")) return name + ".ach";
  return name;
}
