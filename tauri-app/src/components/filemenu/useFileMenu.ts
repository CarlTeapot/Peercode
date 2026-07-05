import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { CurrentFileInfo, DocumentMeta } from "./format";
import { saveBuffer, saveBufferAs } from "./saveFlow";

export type FileMenuView = "menu" | "recents";

/** State and backend actions behind the File menu; views stay render-only. */
export function useFileMenu(
  onDocumentLoaded: (text: string, name: string) => void,
  onSaved: () => void,
) {
  const [open, setOpen] = useState(false);
  const [view, setView] = useState<FileMenuView>("menu");
  const [recents, setRecents] = useState<DocumentMeta[]>([]);
  const [docsDir, setDocsDir] = useState<string | null>(null);
  const [current, setCurrent] = useState<CurrentFileInfo | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const refreshAll = useCallback(async () => {
    const results = await Promise.allSettled([
      invoke<DocumentMeta[]>("list_recent_files"),
      invoke<CurrentFileInfo | null>("get_current_file"),
      invoke<string>("get_documents_dir"),
    ]);
    // Best-effort refresh: anything that failed simply keeps its last value.
    if (results[0].status === "fulfilled") setRecents(results[0].value);
    if (results[1].status === "fulfilled") setCurrent(results[1].value);
    if (results[2].status === "fulfilled") setDocsDir(results[2].value);
  }, []);

  const toggleMenu = useCallback(async () => {
    if (!open) await refreshAll();
    setOpen((prev) => !prev);
    setView("menu");
    setError("");
  }, [open, refreshAll]);

  const closeMenu = useCallback(() => {
    setOpen(false);
    setView("menu");
  }, []);

  const showView = useCallback((next: FileMenuView) => {
    setView(next);
    setError("");
  }, []);

  const runAction = useCallback(async (action: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }, []);

  const refreshCurrent = useCallback(async () => {
    setCurrent(await invoke<CurrentFileInfo | null>("get_current_file"));
  }, []);

  const saveCurrent = useCallback(async () => {
    await runAction(async () => {
      if (!(await saveBuffer(current, docsDir))) return;
      await refreshCurrent();
      onSaved();
      closeMenu();
    });
  }, [current, docsDir, runAction, refreshCurrent, onSaved, closeMenu]);

  const saveAs = useCallback(async () => {
    await runAction(async () => {
      if (!(await saveBufferAs(current, docsDir))) return;
      await refreshCurrent();
      onSaved();
      closeMenu();
    });
  }, [current, docsDir, runAction, refreshCurrent, onSaved, closeMenu]);

  const openPath = useCallback(
    async (path: string) => {
      await runAction(async () => {
        const text = await invoke<string>("open_file", { path });
        const info = await invoke<CurrentFileInfo | null>("get_current_file");
        setCurrent(info);
        onDocumentLoaded(text, info?.name ?? "untitled");
        closeMenu();
      });
    },
    [runAction, onDocumentLoaded, closeMenu],
  );

  const openFrom = useCallback(async () => {
    let selected: string | string[] | null = null;
    try {
      selected = await openFileDialog({
        title: "Open file…",
        multiple: false,
        defaultPath: docsDir ?? undefined,
      });
    } catch (err) {
      setError(String(err));
      return;
    }
    if (typeof selected === "string") await openPath(selected);
  }, [docsDir, openPath]);

  const removeRecent = useCallback(
    async (path: string) => {
      await runAction(async () => {
        await invoke("remove_recent_file", { path });
        setRecents(await invoke<DocumentMeta[]>("list_recent_files"));
      });
    },
    [runAction],
  );

  const reveal = useCallback(async (path: string) => {
    try {
      await revealItemInDir(path);
    } catch {
      // best-effort; not all file managers support reveal
    }
  }, []);

  const fork = useCallback(async () => {
    await runAction(async () => {
      const text = await invoke<string>("fork_document");
      setCurrent(null);
      onDocumentLoaded(text, "untitled");
      closeMenu();
    });
  }, [runAction, onDocumentLoaded, closeMenu]);

  return {
    open,
    view,
    recents,
    current,
    error,
    busy,
    toggleMenu,
    closeMenu,
    showView,
    saveCurrent,
    saveAs,
    openPath,
    openFrom,
    removeRecent,
    reveal,
    fork,
  };
}
