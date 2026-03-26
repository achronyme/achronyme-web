// Achronyme language support for CodeMirror 6
// Uses StreamLanguage (tokenizer-based) for simplicity.

import { StreamLanguage } from "@codemirror/language";

const keywords = new Set([
  "let", "mut", "fn", "if", "else", "return", "for", "in",
  "while", "forever", "break", "continue", "import", "export",
  "as", "from", "prove", "circuit",
]);

const builtins = new Set([
  "print", "assert", "assert_eq", "poseidon", "poseidon_many",
  "typeof", "len", "range_check", "merkle_verify", "mux",
]);

const types = new Set([
  "Public", "Witness", "Field", "Int", "BigInt",
]);

const atoms = new Set(["true", "false", "nil"]);

const achronymeTokenizer = {
  startState() {
    return {};
  },

  token(stream, _state) {
    // Whitespace
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
        if (ch === "\\") stream.next(); // skip escaped char
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

    // Static access operator ::
    if (stream.match("::")) {
      return "punctuation";
    }

    // Identifiers and keywords
    if (stream.match(/^[a-zA-Z_][a-zA-Z0-9_]*/)) {
      const word = stream.current();
      if (keywords.has(word)) return "keyword";
      if (builtins.has(word)) return "variableName.standard";
      if (types.has(word)) return "typeName";
      if (atoms.has(word)) return "atom";
      return "variableName";
    }

    // Operators
    if (stream.match(/^[+\-*/%=<>!&|^~]+/)) {
      return "operator";
    }

    // Braces, brackets, parens
    if (stream.match(/^[{}()\[\],;:\.]/)) {
      return "punctuation";
    }

    // Advance past any unrecognized character
    stream.next();
    return null;
  },
};

export const achronymeLanguage = StreamLanguage.define(achronymeTokenizer);
