// Achronyme LSP bridge: WASM → CodeMirror 6 extensions.
//
// Per-tab factories: each tab carries its own file path, and the
// editor extension list is built once per tab so diagnostics, hover,
// and autocomplete dispatch by `.ach` vs `.circom`.
//
// The two languages overlap semantically (`Poseidon` is a builtin in
// `.ach` and a circomlib component in `.circom` with different
// parameters), so a single shared linter would surface wrong-language
// diagnostics or hover docs the moment a user switches tabs. The
// dispatch is at extension-construction time, not inside the linter
// callback, because the editor's tab-switch path replaces the entire
// `EditorState` (extensions and all) — there is nothing to read mid-
// linter that would tell us which file we're on.

import { linter } from "@codemirror/lint";
import { autocompletion } from "@codemirror/autocomplete";
import { hoverTooltip } from "@codemirror/view";

let wasm = null;
let cachedAchCompletions = null;
let cachedCircomCompletions = null;

/**
 * Load the WASM module. Call once at page load.
 * Extensions degrade gracefully (return empty results) before this resolves.
 */
export async function initLspWasm() {
  try {
    const wasmUrl = new URL("/wasm/achronyme_wasm.js", window.location.origin).href;
    const mod = await import(/* @vite-ignore */ wasmUrl);
    await mod.default();
    wasm = mod;
    try {
      cachedAchCompletions = JSON.parse(wasm.completions());
    } catch {
      cachedAchCompletions = [];
    }
    try {
      cachedCircomCompletions = JSON.parse(wasm.completions_circom());
    } catch {
      cachedCircomCompletions = [];
    }
  } catch (e) {
    console.warn("[ach-lsp] WASM load failed:", e);
  }
}

// ── Helpers ─────────────────────────────────────────────────

function isCircomPath(path) {
  return typeof path === "string" && path.endsWith(".circom");
}

function posToOffset(doc, pos) {
  if (pos.line >= doc.lines) return doc.length;
  const line = doc.line(pos.line + 1); // CM lines are 1-based
  return Math.min(line.from + pos.character, line.to);
}

function mapSeverity(s) {
  switch (s) {
    case "Error": return "error";
    case "Warning": return "warning";
    case "Information": return "info";
    case "Hint": return "info";
    default: return "error";
  }
}

function mapCompletionKind(k) {
  switch (k) {
    case "Keyword": return "keyword";
    case "Function": return "function";
    case "Method": return "method";
    case "Constant": return "constant";
    case "Snippet": return "text";
    default: return "variable";
  }
}

// ── Linter (squiggles) ─────────────────────────────────────

/**
 * Build a CodeMirror linter extension for the given file path.
 * Routes to `wasm.check_circom` for `.circom` files and `wasm.check`
 * otherwise.
 */
export function achLinter(path) {
  const useCircom = isCircomPath(path);
  return linter((view) => {
    if (!wasm) return [];
    try {
      const source = view.state.doc.toString();
      const raw = useCircom ? wasm.check_circom(source) : wasm.check(source);
      const diags = JSON.parse(raw);
      return diags.map((d) => {
        const from = posToOffset(view.state.doc, d.range.start);
        const to = posToOffset(view.state.doc, d.range.end);
        return {
          from: Math.min(from, view.state.doc.length),
          to: Math.min(Math.max(to, from + 1), view.state.doc.length),
          severity: mapSeverity(d.severity),
          message: d.message,
          source: d.source || (useCircom ? "circom" : "ach"),
        };
      });
    } catch {
      return [];
    }
  }, { delay: 300 });
}

// ── Autocomplete ────────────────────────────────────────────

function buildCompletionSource(useCircom) {
  return (context) => {
    const cache = useCircom ? cachedCircomCompletions : cachedAchCompletions;
    if (!cache) return null;

    const word = context.matchBefore(/[\w$]+/);
    if (!word && !context.explicit) return null;

    const options = cache.map((item) => ({
      label: item.label,
      type: mapCompletionKind(item.kind),
      detail: item.detail || undefined,
      apply: item.insert_text_format === "Snippet"
        ? undefined
        : (item.insert_text || item.label),
    }));

    return {
      from: word ? word.from : context.pos,
      options,
      validFor: /^[\w$]*$/,
    };
  };
}

/**
 * Build a CodeMirror autocomplete extension for the given file path.
 * Surfaces circom keywords + circomlib snippets for `.circom` and the
 * `.ach` keyword/builtin/method/snippet tables otherwise.
 */
export function achAutocompletion(path) {
  return autocompletion({
    override: [buildCompletionSource(isCircomPath(path))],
    defaultKeymap: true,
    icons: true,
  });
}

// ── Hover tooltip ───────────────────────────────────────────

/**
 * Build a CodeMirror hover-tooltip extension for the given file path.
 * Routes to the circom hover table for `.circom` so circomlib templates
 * resolve to circom-side docs, not the achronyme builtin of the same
 * name.
 */
export function achHover(path) {
  const useCircom = isCircomPath(path);
  return hoverTooltip((view, pos) => {
    if (!wasm) return null;

    const doc = view.state.doc;
    const line = doc.lineAt(pos);
    const lineNum = line.number - 1;
    const col = pos - line.from;

    try {
      const md = useCircom
        ? wasm.hover_circom(doc.toString(), lineNum, col)
        : wasm.hover(doc.toString(), lineNum, col);
      if (!md) return null;

      const text = line.text;
      let start = col;
      while (start > 0 && /[\w]/.test(text[start - 1])) start--;
      let end = col;
      while (end < text.length && /[\w]/.test(text[end])) end++;

      return {
        pos: line.from + start,
        end: line.from + end,
        above: true,
        create() {
          const dom = document.createElement("div");
          dom.className = "ach-hover-tooltip";
          dom.innerHTML = formatHoverMarkdown(md);
          return { dom };
        },
      };
    } catch {
      return null;
    }
  });
}

function formatHoverMarkdown(md) {
  return md
    .replace(/```(\w*)\n([\s\S]*?)```/g, (_, _lang, code) =>
      `<pre class="ach-hover-code">${escapeHtml(code.trim())}</pre>`)
    .replace(/`([^`]+)`/g, '<code class="ach-hover-inline">$1</code>')
    .replace(/\n\n/g, '<br/>')
    .replace(/\n/g, ' ');
}

function escapeHtml(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
