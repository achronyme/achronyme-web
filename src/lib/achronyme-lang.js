// Achronyme + circom language support for CodeMirror 6.
// StreamLanguage (tokenizer-based) — one tokenizer per language.

import { StreamLanguage } from "@codemirror/language";

// ── .ach ────────────────────────────────────────────────────

const achKeywords = new Set([
  "let", "mut", "fn", "if", "else", "return", "for", "in",
  "while", "forever", "break", "continue", "import", "export",
  "as", "from", "prove", "circuit",
]);

const achBuiltins = new Set([
  "print", "assert", "assert_eq", "poseidon", "poseidon_many",
  "typeof", "len", "range_check", "merkle_verify", "mux",
]);

const achTypes = new Set([
  "Public", "Witness", "Field", "Int", "BigInt",
]);

const achAtoms = new Set(["true", "false", "nil"]);

const achronymeTokenizer = {
  startState() {
    return {};
  },

  token(stream, _state) {
    if (stream.eatSpace()) return null;

    // Line comment
    if (stream.match("//")) {
      stream.skipToEnd();
      return "lineComment";
    }

    // Strings
    if (stream.match('"')) {
      while (!stream.eol()) {
        const ch = stream.next();
        if (ch === "\\") stream.next();
        else if (ch === '"') break;
      }
      return "string";
    }

    // Field literals: 0p42, 0pxFF, 0pb101
    if (stream.match(/^0p[xXbB]?[0-9a-fA-F]+/)) {
      return "number";
    }

    // Hex numbers: 0xFF
    if (stream.match(/^0[xX][0-9a-fA-F]+/)) {
      return "number";
    }

    // Decimal numbers
    if (stream.match(/^-?\d+(\.\d+)?/)) {
      return "number";
    }

    if (stream.match("::")) {
      return "punctuation";
    }

    if (stream.match(/^[a-zA-Z_][a-zA-Z0-9_]*/)) {
      const word = stream.current();
      if (achKeywords.has(word)) return "keyword";
      if (achBuiltins.has(word)) return "variableName.standard";
      if (achTypes.has(word)) return "typeName";
      if (achAtoms.has(word)) return "atom";
      return "variableName";
    }

    if (stream.match(/^[+\-*/%=<>!&|^~]+/)) {
      return "operator";
    }

    if (stream.match(/^[{}()\[\],;:\.]/)) {
      return "punctuation";
    }

    stream.next();
    return null;
  },
};

export const achronymeLanguage = StreamLanguage.define(achronymeTokenizer);

// ── .circom ─────────────────────────────────────────────────

const circomKeywords = new Set([
  "pragma", "include", "template", "component", "function",
  "signal", "input", "output", "var",
  "for", "while", "if", "else", "return",
  "main", "public",
]);

// circomlib templates the achronyme front-end is round-trip-verified
// against. Kept in a separate set so the highlighter can use a
// distinct token type from generic identifiers — these are the names
// users will type most often when wiring up real circuits.
const circomLibTemplates = new Set([
  "Num2Bits", "Bits2Num", "LessThan", "LessEqThan", "GreaterThan",
  "GreaterEqThan", "IsZero", "IsEqual", "Mux1", "Mux2",
  "Poseidon", "MiMCSponge", "MiMC7", "Pedersen",
  "EdDSAPoseidon", "EdDSAPoseidonVerifier",
  "Sha256", "Sha256compression",
  "BabyAdd", "BabyDbl", "BabyCheck",
  "EscalarMulFix", "EscalarMulAny",
]);

const circomAtoms = new Set(["true", "false"]);

const circomTokenizer = {
  startState() {
    return { inBlockComment: false };
  },

  token(stream, state) {
    // Block comment continuation across lines
    if (state.inBlockComment) {
      while (!stream.eol()) {
        if (stream.match("*/")) {
          state.inBlockComment = false;
          return "blockComment";
        }
        stream.next();
      }
      return "blockComment";
    }

    if (stream.eatSpace()) return null;

    if (stream.match("//")) {
      stream.skipToEnd();
      return "lineComment";
    }

    if (stream.match("/*")) {
      state.inBlockComment = true;
      while (!stream.eol()) {
        if (stream.match("*/")) {
          state.inBlockComment = false;
          return "blockComment";
        }
        stream.next();
      }
      return "blockComment";
    }

    if (stream.match('"')) {
      while (!stream.eol()) {
        const ch = stream.next();
        if (ch === "\\") stream.next();
        else if (ch === '"') break;
      }
      return "string";
    }

    if (stream.match(/^0[xX][0-9a-fA-F]+/)) {
      return "number";
    }

    if (stream.match(/^\d+/)) {
      return "number";
    }

    // Constraint / assignment operators get a distinct color so the
    // user can spot at a glance which lines emit constraints vs which
    // are pure witness hints.
    if (stream.match("<==") || stream.match("==>") ||
        stream.match("<--") || stream.match("-->") ||
        stream.match("===")) {
      return "operator";
    }

    if (stream.match(/^[a-zA-Z_][a-zA-Z0-9_]*/)) {
      const word = stream.current();
      if (circomKeywords.has(word)) return "keyword";
      if (circomLibTemplates.has(word)) return "typeName";
      if (circomAtoms.has(word)) return "atom";
      return "variableName";
    }

    if (stream.match(/^[+\-*/%=<>!&|^~]+/)) {
      return "operator";
    }

    if (stream.match(/^[{}()\[\],;:\.]/)) {
      return "punctuation";
    }

    stream.next();
    return null;
  },
};

export const circomLanguage = StreamLanguage.define(circomTokenizer);

/**
 * Pick the language extension for a given file path.
 * Defaults to `.ach` when the path is missing or unrecognized.
 */
export function languageForPath(path) {
  if (typeof path === "string" && path.endsWith(".circom")) {
    return circomLanguage;
  }
  return achronymeLanguage;
}
