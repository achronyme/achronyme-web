// Achronyme LSP bridge: WASM → CodeMirror 6 extensions
//
// Provides inline diagnostics (squiggles), autocomplete (108 items),
// hover docs (91 tokens), and go-to-definition — all via local WASM.

import { linter } from "@codemirror/lint";
import { autocompletion } from "@codemirror/autocomplete";
import { hoverTooltip } from "@codemirror/view";

let wasm = null;
let cachedCompletions = null;

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
    // Cache completions — they're static data
    try {
      cachedCompletions = JSON.parse(wasm.completions());
    } catch {
      cachedCompletions = [];
    }
  } catch (e) {
    console.warn("[ach-lsp] WASM load failed:", e);
  }
}

// ── Helpers ─────────────────────────────────────────────────

/** Convert 0-based {line, character} to CodeMirror offset. */
function posToOffset(doc, pos) {
  if (pos.line >= doc.lines) return doc.length;
  const line = doc.line(pos.line + 1); // CM lines are 1-based
  return Math.min(line.from + pos.character, line.to);
}

/** Map core severity string to CM severity. */
function mapSeverity(s) {
  switch (s) {
    case "Error": return "error";
    case "Warning": return "warning";
    case "Information": return "info";
    case "Hint": return "info";
    default: return "error";
  }
}

/** Map core CompletionKind to CM completion type. */
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

export const achLinter = linter((view) => {
  if (!wasm) return [];
  try {
    const source = view.state.doc.toString();
    const diags = JSON.parse(wasm.check(source));
    return diags.map((d) => {
      const from = posToOffset(view.state.doc, d.range.start);
      const to = posToOffset(view.state.doc, d.range.end);
      return {
        from: Math.min(from, view.state.doc.length),
        to: Math.min(Math.max(to, from + 1), view.state.doc.length),
        severity: mapSeverity(d.severity),
        message: d.message,
        source: d.source || "ach",
      };
    });
  } catch {
    return [];
  }
}, { delay: 300 });

// ── Autocomplete ────────────────────────────────────────────

function achCompletionSource(context) {
  if (!cachedCompletions) return null;

  const word = context.matchBefore(/[\w$]+/);
  if (!word && !context.explicit) return null;

  const options = cachedCompletions.map((item) => ({
    label: item.label,
    type: mapCompletionKind(item.kind),
    detail: item.detail || undefined,
    apply: item.insert_text_format === "Snippet"
      ? undefined  // let CM handle snippet tabstops if supported
      : (item.insert_text || item.label),
  }));

  return {
    from: word ? word.from : context.pos,
    options,
    validFor: /^[\w$]*$/,
  };
}

export const achAutocompletion = autocompletion({
  override: [achCompletionSource],
  defaultKeymap: true,
  icons: true,
});

// ── Hover tooltip ───────────────────────────────────────────

export const achHover = hoverTooltip((view, pos) => {
  if (!wasm) return null;

  const doc = view.state.doc;
  const line = doc.lineAt(pos);
  const lineNum = line.number - 1;  // 0-based
  const col = pos - line.from;

  try {
    const md = wasm.hover(doc.toString(), lineNum, col);
    if (!md) return null;

    // Find word boundaries for tooltip position
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

/**
 * Minimal markdown → HTML for hover content.
 * Handles code blocks (```...```), inline code (`...`), and paragraphs.
 */
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
