import { useCallback, useEffect, useState } from "react";

const STORAGE_KEY = "peercode-editor-font-size";
const MIN = 10;
const MAX = 24;
const DEFAULT = 14;

function initialSize(): number {
  const stored = Number(localStorage.getItem(STORAGE_KEY));
  return Number.isInteger(stored) && stored >= MIN && stored <= MAX
    ? stored
    : DEFAULT;
}

/**
 * Editor font size (zoom): statusline − / + buttons and Ctrl/Cmd +/−
 * shortcuts, clamped to 10–24px and persisted. Only Monaco scales; the app
 * chrome keeps its fixed size.
 */
export function useEditorFontSize() {
  const [fontSize, setFontSize] = useState(initialSize);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, String(fontSize));
  }, [fontSize]);

  const zoomIn = useCallback(
    () => setFontSize((size) => Math.min(MAX, size + 1)),
    [],
  );
  const zoomOut = useCallback(
    () => setFontSize((size) => Math.max(MIN, size - 1)),
    [],
  );

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey) return;
      // "=" is the unshifted "+" key on most layouts.
      if (e.key === "+" || e.key === "=") {
        e.preventDefault();
        zoomIn();
      } else if (e.key === "-") {
        e.preventDefault();
        zoomOut();
      }
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [zoomIn, zoomOut]);

  return { fontSize, zoomIn, zoomOut };
}
