import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

interface DocumentMeta {
  name: string;
  size_bytes: number;
  modified: number | null;
}

interface FileMenuProps {
  onDocumentLoaded: (text: string, name: string) => void;
}

export function FileMenu({ onDocumentLoaded }: FileMenuProps) {
  const [open, setOpen] = useState(false);
  const [view, setView] = useState<"menu" | "save" | "load" | "fork">("menu");
  const [docs, setDocs] = useState<DocumentMeta[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [currentName, setCurrentName] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpen(false);
        setView("menu");
        setError("");
      }
    };
    if (open) document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open]);

  const refreshDocList = useCallback(async () => {
    try {
      const list = await invoke<DocumentMeta[]>("list_saved_documents");
      setDocs(list);
    } catch {
      // linter doesn't like empty catches :(
    }
  }, []);

  const refreshCurrentName = useCallback(async () => {
    try {
      const name = await invoke<string | null>("get_current_document_name");
      setCurrentName(name);
    } catch {
      // linter doesn't like empty catches :(
    }
  }, []);

  const toggleMenu = useCallback(async () => {
    if (!open) {
      await refreshDocList();
      await refreshCurrentName();
    }
    setOpen((prev) => !prev);
    setView("menu");
    setError("");
    setInputValue("");
  }, [open, refreshDocList, refreshCurrentName]);

  const handleSaveAs = useCallback(async () => {
    const name = inputValue.trim();
    if (!name) return;
    setBusy(true);
    setError("");
    try {
      await invoke("save_document", { name });
      setCurrentName(name);
      setOpen(false);
      setView("menu");
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }, [inputValue]);

  const handleSave = useCallback(async () => {
    if (!currentName) {
      setView("save");
      setInputValue("");
      return;
    }
    setBusy(true);
    setError("");
    try {
      await invoke("save_document", { name: currentName });
      setOpen(false);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }, [currentName]);

  const handleExportText = useCallback(async () => {
    setBusy(true);
    setError("");
    try {
      const text = await invoke<string>("get_document_text");
      const filePath = await save({
        title: "Export as text",
        defaultPath: (currentName || "document") + ".txt",
        filters: [
          { name: "Text files", extensions: ["txt"] },
          { name: "All files", extensions: ["*"] },
        ],
      });
      if (filePath) {
        await invoke("save_text_file", { path: filePath, content: text });
      }
      setOpen(false);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }, [currentName]);

  const handleLoad = useCallback(
    async (name: string) => {
      setBusy(true);
      setError("");
      try {
        const text = await invoke<string>("load_document", { name });
        setCurrentName(name);
        onDocumentLoaded(text, name);
        setOpen(false);
        setView("menu");
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [onDocumentLoaded],
  );

  const handleDelete = useCallback(
    async (name: string) => {
      setBusy(true);
      setError("");
      try {
        await invoke("delete_document", { name });
        await refreshDocList();
        if (currentName === name) setCurrentName(null);
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [currentName, refreshDocList],
  );

  const handleFork = useCallback(async () => {
    const name = inputValue.trim();
    if (!name) return;
    setBusy(true);
    setError("");
    try {
      const text = await invoke<string>("fork_document", { newName: name });
      setCurrentName(name);
      onDocumentLoaded(text, name);
      setOpen(false);
      setView("menu");
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }, [inputValue, onDocumentLoaded]);

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    return `${(bytes / 1024).toFixed(1)} KB`;
  };

  const formatDate = (ts: number | null) => {
    if (!ts) return "—";
    return new Date(ts * 1000).toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  return (
    <div ref={menuRef} style={{ position: "relative" }}>
      <button onClick={toggleMenu} className="file-menu-btn">
        File
        {currentName && (
          <span className="file-menu-current">— {currentName}</span>
        )}
      </button>

      {open && (
        <div className="file-dropdown">
          {view === "menu" && (
            <>
              <button
                className="file-dropdown-item"
                onClick={handleSave}
                disabled={busy}
              >
                Save{currentName ? ` "${currentName}"` : ""}
              </button>
              <button
                className="file-dropdown-item"
                onClick={() => {
                  setView("save");
                  setInputValue(currentName || "");
                  setError("");
                }}
                disabled={busy}
              >
                Save as…
              </button>
              <button
                className="file-dropdown-item"
                onClick={handleExportText}
                disabled={busy}
              >
                Export as .txt…
              </button>
              <div className="file-dropdown-separator" />
              <button
                className="file-dropdown-item"
                onClick={async () => {
                  setView("load");
                  setError("");
                  await refreshDocList();
                }}
                disabled={busy}
              >
                Open…
              </button>
              <button
                className="file-dropdown-item"
                onClick={() => {
                  setView("fork");
                  setInputValue(currentName ? currentName + "-fork" : "fork");
                  setError("");
                }}
                disabled={busy}
              >
                Fork…
              </button>
            </>
          )}

          {view === "save" && (
            <div className="file-dropdown-form">
              <div className="file-dropdown-title">Save as</div>
              <div className="file-dropdown-input-row">
                <input
                  className="file-dropdown-input"
                  autoFocus
                  placeholder="Document name"
                  value={inputValue}
                  onChange={(e) => {
                    setInputValue(e.target.value);
                    setError("");
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleSaveAs();
                    if (e.key === "Escape") {
                      setView("menu");
                      setError("");
                    }
                  }}
                />
                <span className="file-dropdown-ext">.pcdoc</span>
              </div>
              <div className="file-dropdown-actions">
                <button
                  className="file-dropdown-btn secondary"
                  onClick={() => {
                    setView("menu");
                    setError("");
                  }}
                >
                  Back
                </button>
                <button
                  className="file-dropdown-btn primary"
                  onClick={handleSaveAs}
                  disabled={busy || !inputValue.trim()}
                >
                  {busy ? "Saving…" : "Save"}
                </button>
              </div>
            </div>
          )}

          {view === "fork" && (
            <div className="file-dropdown-form">
              <div className="file-dropdown-title">Fork document</div>
              <div className="file-dropdown-subtitle">
                Creates an independent copy with a new identity.
              </div>
              <div className="file-dropdown-input-row">
                <input
                  className="file-dropdown-input"
                  autoFocus
                  placeholder="Fork name"
                  value={inputValue}
                  onChange={(e) => {
                    setInputValue(e.target.value);
                    setError("");
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleFork();
                    if (e.key === "Escape") {
                      setView("menu");
                      setError("");
                    }
                  }}
                />
                <span className="file-dropdown-ext">.pcdoc</span>
              </div>
              <div className="file-dropdown-actions">
                <button
                  className="file-dropdown-btn secondary"
                  onClick={() => {
                    setView("menu");
                    setError("");
                  }}
                >
                  Back
                </button>
                <button
                  className="file-dropdown-btn primary"
                  onClick={handleFork}
                  disabled={busy || !inputValue.trim()}
                >
                  {busy ? "Forking…" : "Fork"}
                </button>
              </div>
            </div>
          )}

          {view === "load" && (
            <div className="file-dropdown-form">
              <div className="file-dropdown-title">Open document</div>
              {docs.length === 0 ? (
                <div className="file-dropdown-subtitle">
                  No saved documents yet.
                </div>
              ) : (
                <div className="file-dropdown-list">
                  {docs.map((d) => (
                    <div key={d.name} className="file-dropdown-doc">
                      <button
                        className="file-dropdown-doc-name"
                        onClick={() => handleLoad(d.name)}
                        disabled={busy}
                      >
                        {d.name}
                        <span className="file-dropdown-doc-meta">
                          {formatSize(d.size_bytes)} · {formatDate(d.modified)}
                        </span>
                      </button>
                      <button
                        className="file-dropdown-doc-delete"
                        onClick={() => handleDelete(d.name)}
                        disabled={busy}
                        title="Delete"
                      >
                        ×
                      </button>
                    </div>
                  ))}
                </div>
              )}
              <div className="file-dropdown-actions">
                <button
                  className="file-dropdown-btn secondary"
                  onClick={() => {
                    setView("menu");
                    setError("");
                  }}
                >
                  Back
                </button>
              </div>
            </div>
          )}

          {error && <div className="file-dropdown-error">{error}</div>}
        </div>
      )}
    </div>
  );
}
