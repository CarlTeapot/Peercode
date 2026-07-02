import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openFileDialog, save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  type DocumentMeta,
  EXPORT_FILTERS,
  SAVE_FILTERS,
  fileName,
  isPcdoc,
} from "./format";

export type FileMenuView = "menu" | "save" | "load" | "fork";

/** State and backend actions behind the File menu; views stay render-only. */
export function useFileMenu(
  onDocumentLoaded: (text: string, name: string) => void,
) {
  const [open, setOpen] = useState(false);
  const [view, setView] = useState<FileMenuView>("menu");
  const [docs, setDocs] = useState<DocumentMeta[]>([]);
  const [docsDir, setDocsDir] = useState<string | null>(null);
  const [exportPath, setExportPath] = useState<string | null>(null);
  const [inputValue, setInputValue] = useState("");
  const [currentName, setCurrentName] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const refreshAll = useCallback(async () => {
    const results = await Promise.allSettled([
      invoke<DocumentMeta[]>("list_saved_documents"),
      invoke<string | null>("get_current_document_name"),
      invoke<string>("get_documents_dir"),
      invoke<string | null>("get_current_export_path"),
    ]);
    // Best-effort refresh: anything that failed simply keeps its last value.
    if (results[0].status === "fulfilled") setDocs(results[0].value);
    if (results[1].status === "fulfilled") setCurrentName(results[1].value);
    if (results[2].status === "fulfilled") setDocsDir(results[2].value);
    if (results[3].status === "fulfilled") setExportPath(results[3].value);
  }, []);

  const toggleMenu = useCallback(async () => {
    if (!open) await refreshAll();
    setOpen((prev) => !prev);
    setView("menu");
    setError("");
    setInputValue("");
  }, [open, refreshAll]);

  const closeMenu = useCallback(() => {
    setOpen(false);
    setView("menu");
  }, []);

  const showView = useCallback((next: FileMenuView, input = "") => {
    setView(next);
    setInputValue(input);
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

  const saveAs = useCallback(async () => {
    const name = inputValue.trim();
    if (!name) return;
    await runAction(async () => {
      await invoke("save_document", { name });
      setCurrentName(name);
      closeMenu();
    });
  }, [inputValue, runAction, closeMenu]);

  const saveCurrent = useCallback(async () => {
    if (!currentName) {
      showView("save");
      return;
    }
    await runAction(async () => {
      await invoke("save_current_document");
      closeMenu();
    });
  }, [currentName, runAction, showView, closeMenu]);

  const saveTo = useCallback(async () => {
    await runAction(async () => {
      const filePath = await save({
        title: "Save document to…",
        defaultPath: docsDir
          ? `${docsDir}/${currentName || "document"}.pcdoc`
          : undefined,
        filters: SAVE_FILTERS,
      });
      if (!filePath) return;
      await invoke("save_document_to_path", { path: filePath });
      closeMenu();
    });
  }, [runAction, docsDir, currentName, closeMenu]);

  /** Opens a .pcdoc natively; anything else is imported as chunked text. */
  const openPath = useCallback(
    async (path: string) => {
      await runAction(async () => {
        const command = isPcdoc(path)
          ? "load_document_from_path"
          : "import_text_file";
        const text = await invoke<string>(command, { path });
        const name = await invoke<string | null>("get_current_document_name");
        setCurrentName(name);
        onDocumentLoaded(text, name ?? "document");
        closeMenu();
      });
    },
    [runAction, onDocumentLoaded, closeMenu],
  );

  const openFrom = useCallback(async () => {
    let selected: string | string[] | null = null;
    try {
      // No filters: everything stays visible so any text file is importable.
      selected = await openFileDialog({
        title: "Open document or text file…",
        multiple: false,
        defaultPath: docsDir ?? undefined,
      });
    } catch (err) {
      setError(String(err));
      return;
    }
    if (typeof selected === "string") await openPath(selected);
  }, [docsDir, openPath]);

  const exportAs = useCallback(async () => {
    await runAction(async () => {
      const filePath = await save({
        title: "Export as…",
        defaultPath: exportPath ?? `${currentName || "document"}.txt`,
        filters: EXPORT_FILTERS,
      });
      if (!filePath) return;
      await invoke("export_document_to_path", { path: filePath });
      setExportPath(filePath);
      closeMenu();
    });
  }, [runAction, exportPath, currentName, closeMenu]);

  const exportToLinked = useCallback(async () => {
    await runAction(async () => {
      await invoke<string>("export_current_document");
      closeMenu();
    });
  }, [runAction, closeMenu]);

  const reveal = useCallback(async (path: string) => {
    try {
      await revealItemInDir(path);
    } catch {
      // best-effort; not all file managers support reveal
    }
  }, []);

  const deleteDoc = useCallback(
    async (name: string) => {
      await runAction(async () => {
        await invoke("delete_document", { name });
        setDocs(await invoke<DocumentMeta[]>("list_saved_documents"));
        if (currentName === name) setCurrentName(null);
      });
    },
    [currentName, runAction],
  );

  const fork = useCallback(async () => {
    const name = inputValue.trim();
    if (!name) return;
    await runAction(async () => {
      const text = await invoke<string>("fork_document", { newName: name });
      setCurrentName(name);
      onDocumentLoaded(text, name);
      closeMenu();
    });
  }, [inputValue, onDocumentLoaded, runAction, closeMenu]);

  return {
    open,
    view,
    docs,
    docsDir,
    exportPath,
    exportFileName: exportPath ? fileName(exportPath) : null,
    inputValue,
    setInputValue,
    currentName,
    error,
    busy,
    toggleMenu,
    closeMenu,
    showView,
    saveAs,
    saveCurrent,
    saveTo,
    openPath,
    openFrom,
    exportAs,
    exportToLinked,
    reveal,
    deleteDoc,
    fork,
  };
}
