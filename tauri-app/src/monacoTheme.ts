import type { Monaco } from "@monaco-editor/react";

export const PEERCODE_THEME = "peercode-dark";

/** Tokyo Night-flavored Monaco theme matching the app's CSS tokens. */
export function registerPeercodeTheme(monaco: Monaco) {
  monaco.editor.defineTheme(PEERCODE_THEME, {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "", foreground: "c0caf5" },
      { token: "comment", foreground: "565f89", fontStyle: "italic" },
      { token: "keyword", foreground: "bb9af7" },
      { token: "operator", foreground: "89ddff" },
      { token: "delimiter", foreground: "a9b1d6" },
      { token: "string", foreground: "9ece6a" },
      { token: "number", foreground: "ff9e64" },
      { token: "constant", foreground: "ff9e64" },
      { token: "type", foreground: "7dcfff" },
      { token: "namespace", foreground: "7dcfff" },
      { token: "function", foreground: "7aa2f7" },
      { token: "variable", foreground: "c0caf5" },
      { token: "tag", foreground: "f7768e" },
      { token: "attribute.name", foreground: "bb9af7" },
    ],
    colors: {
      "editor.background": "#1a1b26",
      "editor.foreground": "#c0caf5",
      "editor.lineHighlightBackground": "#1f2335",
      "editor.selectionBackground": "#2d3f76",
      "editor.inactiveSelectionBackground": "#292e42",
      "editorCursor.foreground": "#c0caf5",
      "editorLineNumber.foreground": "#3b4261",
      "editorLineNumber.activeForeground": "#737aa2",
      "editorIndentGuide.background1": "#1f2335",
      "editorWidget.background": "#1f2335",
      "editorWidget.border": "#2f334d",
      "editorSuggestWidget.selectedBackground": "#292e42",
      "editorHoverWidget.background": "#1f2335",
      "editorGutter.background": "#1a1b26",
      "scrollbarSlider.background": "#2f334d66",
      "scrollbarSlider.hoverBackground": "#3b426199",
      "scrollbarSlider.activeBackground": "#3b4261cc",
    },
  });
}
