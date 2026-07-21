import type { Monaco } from "@monaco-editor/react";

export const PEERCODE_THEME = "peercode-dark";
export const PEERCODE_LIGHT_THEME = "peercode-light";

export type ThemeMode = "dark" | "light";

export function monacoThemeFor(mode: ThemeMode): string {
  return mode === "light" ? PEERCODE_LIGHT_THEME : PEERCODE_THEME;
}

/**
 * Registers both PeerCode Monaco themes. The editor backgrounds must stay in
 * sync with the `--bg-app` CSS token of the matching `data-theme` block in
 * App.css so chrome and editor read as one continuous surface.
 */
export function registerPeercodeThemes(monaco: Monaco) {
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
      "editor.background": "#1f1e2c",
      "editor.foreground": "#c0caf5",
      "editor.lineHighlightBackground": "#282736",
      "editor.selectionBackground": "#3d3a5e",
      "editor.inactiveSelectionBackground": "#2e2c42",
      "editorCursor.foreground": "#c0caf5",
      "editorLineNumber.foreground": "#3b4261",
      "editorLineNumber.activeForeground": "#737aa2",
      "editorIndentGuide.background1": "#282736",
      "editorWidget.background": "#292b37",
      "editorWidget.border": "#333244",
      "editorSuggestWidget.selectedBackground": "#333546",
      "editorHoverWidget.background": "#292b37",
      "editorGutter.background": "#1f1e2c",
      "scrollbarSlider.background": "#33324466",
      "scrollbarSlider.hoverBackground": "#43425999",
      "scrollbarSlider.activeBackground": "#434259cc",
    },
  });

  monaco.editor.defineTheme(PEERCODE_LIGHT_THEME, {
    base: "vs",
    inherit: true,
    rules: [
      { token: "", foreground: "3b3a52" },
      { token: "comment", foreground: "9a94b8", fontStyle: "italic" },
      { token: "keyword", foreground: "7c5fd3" },
      { token: "operator", foreground: "007197" },
      { token: "delimiter", foreground: "4a4568" },
      { token: "string", foreground: "587539" },
      { token: "number", foreground: "b15c00" },
      { token: "constant", foreground: "b15c00" },
      { token: "type", foreground: "007197" },
      { token: "namespace", foreground: "007197" },
      { token: "function", foreground: "2e7de9" },
      { token: "variable", foreground: "3b3a52" },
      { token: "tag", foreground: "c64343" },
      { token: "attribute.name", foreground: "7c5fd3" },
    ],
    colors: {
      "editor.background": "#f4f2fb",
      "editor.foreground": "#3b3a52",
      "editor.lineHighlightBackground": "#e9e6f5",
      "editor.selectionBackground": "#cdc4ee",
      "editor.inactiveSelectionBackground": "#dfdaf0",
      "editorCursor.foreground": "#2e2a45",
      "editorLineNumber.foreground": "#b4aed0",
      "editorLineNumber.activeForeground": "#7a749c",
      "editorIndentGuide.background1": "#e4e1f0",
      "editorWidget.background": "#ffffff",
      "editorWidget.border": "#ccc7e0",
      "editorSuggestWidget.selectedBackground": "#e4e1f0",
      "editorHoverWidget.background": "#ffffff",
      "editorGutter.background": "#f4f2fb",
      "scrollbarSlider.background": "#ccc7e066",
      "scrollbarSlider.hoverBackground": "#b4aed099",
      "scrollbarSlider.activeBackground": "#b4aed0cc",
    },
  });
}
