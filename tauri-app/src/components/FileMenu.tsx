import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openFileDialog, save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

interface DocumentMeta {
  name: string;
  path: string;
  size_bytes: number;
  modified: number | null;
  external: boolean;
}

interface FileMenuProps {
  onDocumentLoaded: (text: string, name: string) => void;
}

const PCDOC_FILTERS = [
  { name: "PeerCode documents", extensions: ["pcdoc"] },
  { name: "All files", extensions: ["*"] },
];

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
}

function formatDate(ts: number | null): string {
  if (!ts) return "—";
  return new Date(ts * 1000).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatFolder(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx > 0 ? path.slice(0, idx) : path;
}

interface DocumentRowProps {
  doc: DocumentMeta;
  busy: boolean;
  onLoad: (doc: DocumentMeta) => void;
  onReveal: (path: string) => void;
  onDelete: (name: string) => void;
}

function DocumentRow({
  doc,
  busy,
  onLoad,
  onReveal,
  onDelete,
}: DocumentRowProps) {
  return (
    <div className="file-dropdown-doc">
      <button
        className="file-dropdown-doc-name"
        onClick={() => onLoad(doc)}
        disabled={busy}
      >
        <span>
          {doc.name}
          {doc.external && (
            <span className="file-dropdown-doc-badge">external</span>
          )}
        </span>
        <span className="file-dropdown-doc-meta">
          {formatSize(doc.size_bytes)} · {formatDate(doc.modified)}
        </span>
        <span className="file-dropdown-doc-path" title={doc.path}>
          {formatFolder(doc.path)}
        </span>
      </button>
      <button
        className="file-dropdown-doc-reveal"
        onClick={() => onReveal(doc.path)}
        disabled={busy}
        title="Show in file manager"
      >
        ⤴
      </button>
      {!doc.external && (
        <button
          className="file-dropdown-doc-delete"
          onClick={() => onDelete(doc.name)}
          disabled={busy}
          title="Delete"
        >
          ×
        </button>
      )}
    </div>
  );
}

interface NameFormProps {
  title: string;
  subtitle?: string;
  submitLabel: string;
  busyLabel: string;
  busy: boolean;
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  onBack: () => void;
}

function NameForm({
  title,
  subtitle,
  submitLabel,
  busyLabel,
  busy,
  value,
  onChange,
  onSubmit,
  onBack,
}: NameFormProps) {
  return (
    <div className="file-dropdown-form">
      <div className="file-dropdown-title">{title}</div>
      {subtitle && <div className="file-dropdown-subtitle">{subtitle}</div>}
      <div className="file-dropdown-input-row">
        <input
          className="file-dropdown-input"
          autoFocus
          placeholder="Document name"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") onSubmit();
            if (e.key === "Escape") onBack();
          }}
        />
        <span className="file-dropdown-ext">.pcdoc</span>
      </div>
      <div className="file-dropdown-actions">
        <button className="file-dropdown-btn secondary" onClick={onBack}>
          Back
        </button>
        <button
          className="file-dropdown-btn primary"
          onClick={onSubmit}
          disabled={busy || !value.trim()}
        >
          {busy ? busyLabel : submitLabel}
        </button>
      </div>
    </div>
  );
}

export function FileMenu({ onDocumentLoaded }: FileMenuProps) {
  const [open, setOpen] = useState(false);
  const [view, setView] = useState<"menu" | "save" | "load" | "fork">("menu");
  const [docs, setDocs] = useState<DocumentMeta[]>([]);
  const [docsDir, setDocsDir] = useState<string | null>(null);
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
      setDocs(await invoke<DocumentMeta[]>("list_saved_documents"));
    } catch {
      // best-effort refresh; the list simply stays stale
    }
  }, []);

  const refreshCurrentName = useCallback(async () => {
    try {
      setCurrentName(await invoke<string | null>("get_current_document_name"));
    } catch {
      // best-effort refresh
    }
  }, []);

  const refreshDocsDir = useCallback(async () => {
    try {
      setDocsDir(await invoke<string>("get_documents_dir"));
    } catch {
      // best-effort refresh
    }
  }, []);

  const toggleMenu = useCallback(async () => {
    if (!open) {
      await Promise.all([
        refreshDocList(),
        refreshCurrentName(),
        refreshDocsDir(),
      ]);
    }
    setOpen((prev) => !prev);
    setView("menu");
    setError("");
    setInputValue("");
  }, [open, refreshDocList, refreshCurrentName, refreshDocsDir]);

  const closeMenu = useCallback(() => {
    setOpen(false);
    setView("menu");
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

  const handleSaveAs = useCallback(async () => {
    const name = inputValue.trim();
    if (!name) return;
    await runAction(async () => {
      await invoke("save_document", { name });
      setCurrentName(name);
      closeMenu();
    });
  }, [inputValue, runAction, closeMenu]);

  const handleSave = useCallback(async () => {
    if (!currentName) {
      setView("save");
      setInputValue("");
      return;
    }
    await runAction(async () => {
      await invoke("save_current_document");
      closeMenu();
    });
  }, [currentName, runAction, closeMenu]);

  const handleSaveTo = useCallback(async () => {
    await runAction(async () => {
      const filePath = await save({
        title: "Save document to…",
        defaultPath: docsDir
          ? `${docsDir}/${currentName || "document"}.pcdoc`
          : undefined,
        filters: PCDOC_FILTERS,
      });
      if (!filePath) return;
      await invoke("save_document_to_path", { path: filePath });
      await refreshCurrentName();
      closeMenu();
    });
  }, [runAction, docsDir, currentName, refreshCurrentName, closeMenu]);

  const handleOpenFrom = useCallback(async () => {
    await runAction(async () => {
      const selected = await openFileDialog({
        title: "Open document from…",
        multiple: false,
        defaultPath: docsDir ?? undefined,
        filters: PCDOC_FILTERS,
      });
      if (typeof selected !== "string") return;
      const text = await invoke<string>("load_document_from_path", {
        path: selected,
      });
      const name = await invoke<string | null>("get_current_document_name");
      setCurrentName(name);
      onDocumentLoaded(text, name ?? "document");
      closeMenu();
    });
  }, [runAction, docsDir, onDocumentLoaded, closeMenu]);

  const handleExportText = useCallback(async () => {
    await runAction(async () => {
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
      closeMenu();
    });
  }, [currentName, runAction, closeMenu]);

  const handleLoad = useCallback(
    async (doc: DocumentMeta) => {
      await runAction(async () => {
        const text = await invoke<string>("load_document_from_path", {
          path: doc.path,
        });
        setCurrentName(doc.name);
        onDocumentLoaded(text, doc.name);
        closeMenu();
      });
    },
    [onDocumentLoaded, runAction, closeMenu],
  );

  const handleReveal = useCallback(async (path: string) => {
    try {
      await revealItemInDir(path);
    } catch {
      // best-effort; not all file managers support reveal
    }
  }, []);

  const handleDelete = useCallback(
    async (name: string) => {
      await runAction(async () => {
        await invoke("delete_document", { name });
        await refreshDocList();
        if (currentName === name) setCurrentName(null);
      });
    },
    [currentName, refreshDocList, runAction],
  );

  const handleFork = useCallback(async () => {
    const name = inputValue.trim();
    if (!name) return;
    await runAction(async () => {
      const text = await invoke<string>("fork_document", { newName: name });
      setCurrentName(name);
      onDocumentLoaded(text, name);
      closeMenu();
    });
  }, [inputValue, onDocumentLoaded, runAction, closeMenu]);

  const backToMenu = useCallback(() => {
    setView("menu");
    setError("");
  }, []);

  return (
    <div ref={menuRef} className="file-menu">
      <button onClick={() => void toggleMenu()} className="file-menu-btn">
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
                onClick={() => void handleSave()}
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
                onClick={() => void handleSaveTo()}
                disabled={busy}
              >
                Save to…
              </button>
              <button
                className="file-dropdown-item"
                onClick={() => void handleExportText()}
                disabled={busy}
              >
                Export as .txt…
              </button>
              <div className="file-dropdown-separator" />
              <button
                className="file-dropdown-item"
                onClick={() => {
                  setView("load");
                  setError("");
                  void refreshDocList();
                }}
                disabled={busy}
              >
                Open…
              </button>
              <button
                className="file-dropdown-item"
                onClick={() => void handleOpenFrom()}
                disabled={busy}
              >
                Open from…
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
            <NameForm
              title="Save as"
              submitLabel="Save"
              busyLabel="Saving…"
              busy={busy}
              value={inputValue}
              onChange={(v) => {
                setInputValue(v);
                setError("");
              }}
              onSubmit={() => void handleSaveAs()}
              onBack={backToMenu}
            />
          )}

          {view === "fork" && (
            <NameForm
              title="Fork document"
              subtitle="Creates an independent copy with a new identity."
              submitLabel="Fork"
              busyLabel="Forking…"
              busy={busy}
              value={inputValue}
              onChange={(v) => {
                setInputValue(v);
                setError("");
              }}
              onSubmit={() => void handleFork()}
              onBack={backToMenu}
            />
          )}

          {view === "load" && (
            <div className="file-dropdown-form">
              <div className="file-dropdown-title">Open document</div>
              {docsDir && (
                <div className="file-dropdown-subtitle" title={docsDir}>
                  Library: {docsDir}
                </div>
              )}
              {docs.length === 0 ? (
                <div className="file-dropdown-subtitle">
                  No saved documents yet.
                </div>
              ) : (
                <div className="file-dropdown-list">
                  {docs.map((d) => (
                    <DocumentRow
                      key={d.path}
                      doc={d}
                      busy={busy}
                      onLoad={(doc) => void handleLoad(doc)}
                      onReveal={(path) => void handleReveal(path)}
                      onDelete={(name) => void handleDelete(name)}
                    />
                  ))}
                </div>
              )}
              <div className="file-dropdown-actions">
                <button
                  className="file-dropdown-btn secondary"
                  onClick={backToMenu}
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
