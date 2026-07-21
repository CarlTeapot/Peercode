import { useCallback, useLayoutEffect, useState } from "react";
import type { ThemeMode } from "../monacoTheme";

const STORAGE_KEY = "peercode-theme";

function initialTheme(): ThemeMode {
  return localStorage.getItem(STORAGE_KEY) === "light" ? "light" : "dark";
}

/**
 * App-wide light/dark mode. The mode lives on `<html data-theme="…">` where
 * the token blocks in App.css pick it up; the caller mirrors it into Monaco
 * via `monacoThemeFor(theme)`. Persisted across restarts.
 */
export function useTheme() {
  const [theme, setTheme] = useState<ThemeMode>(initialTheme);

  useLayoutEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem(STORAGE_KEY, theme);
  }, [theme]);

  const toggleTheme = useCallback(() => {
    setTheme((prev) => (prev === "dark" ? "light" : "dark"));
  }, []);

  return { theme, toggleTheme };
}
