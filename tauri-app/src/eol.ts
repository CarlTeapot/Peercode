import type { editor } from "monaco-editor";
import type { Monaco } from "@monaco-editor/react";

// The CRDT stores LF-only text, and every position exchanged with the backend
// is an offset into that text. Monaco must count newlines the same way, so all
// text entering the pipeline is normalized to "\n" and the model EOL is pinned
// to LF everywhere.

export function normalizeToLF(text: string): string {
  return text.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
}

export function forceModelLF(
  ed: editor.IStandaloneCodeEditor,
  mn: Monaco | null,
): void {
  if (!mn) return;
  ed.getModel()?.setEOL(mn.editor.EndOfLineSequence.LF);
}
