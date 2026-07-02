import { useEffect, useRef } from "react";
import { DocumentList } from "./DocumentList";
import { MenuList } from "./MenuList";
import { NameForm } from "./NameForm";
import { useFileMenu } from "./useFileMenu";

interface FileMenuProps {
  onDocumentLoaded: (text: string, name: string) => void;
}

export function FileMenu({ onDocumentLoaded }: FileMenuProps) {
  const menu = useFileMenu(onDocumentLoaded);
  const menuRef = useRef<HTMLDivElement>(null);
  const { open, closeMenu } = menu;

  useEffect(() => {
    if (!open) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        closeMenu();
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open, closeMenu]);

  return (
    <div ref={menuRef} className="file-menu">
      <button onClick={() => void menu.toggleMenu()} className="file-menu-btn">
        File
        {menu.currentName && (
          <span className="file-menu-current">{menu.currentName}</span>
        )}
      </button>

      {menu.open && (
        <div className="file-dropdown">
          {menu.view === "menu" && (
            <MenuList
              currentName={menu.currentName}
              exportFileName={menu.exportFileName}
              busy={menu.busy}
              onSave={() => void menu.saveCurrent()}
              onSaveAs={() => menu.showView("save", menu.currentName || "")}
              onSaveTo={() => void menu.saveTo()}
              onExportLinked={() => void menu.exportToLinked()}
              onExportAs={() => void menu.exportAs()}
              onOpenLibrary={() => menu.showView("load")}
              onOpenFrom={() => void menu.openFrom()}
              onFork={() =>
                menu.showView(
                  "fork",
                  menu.currentName ? menu.currentName + "-fork" : "fork",
                )
              }
            />
          )}

          {menu.view === "save" && (
            <NameForm
              title="Save as"
              submitLabel="Save"
              busyLabel="Saving…"
              busy={menu.busy}
              value={menu.inputValue}
              onChange={menu.setInputValue}
              onSubmit={() => void menu.saveAs()}
              onBack={() => menu.showView("menu")}
            />
          )}

          {menu.view === "fork" && (
            <NameForm
              title="Fork document"
              subtitle="Creates an independent copy with a new identity."
              submitLabel="Fork"
              busyLabel="Forking…"
              busy={menu.busy}
              value={menu.inputValue}
              onChange={menu.setInputValue}
              onSubmit={() => void menu.fork()}
              onBack={() => menu.showView("menu")}
            />
          )}

          {menu.view === "load" && (
            <DocumentList
              docs={menu.docs}
              docsDir={menu.docsDir}
              busy={menu.busy}
              onOpen={(doc) => void menu.openPath(doc.path)}
              onReveal={(path) => void menu.reveal(path)}
              onDelete={(name) => void menu.deleteDoc(name)}
              onBack={() => menu.showView("menu")}
            />
          )}

          {menu.error && (
            <div className="file-dropdown-error">{menu.error}</div>
          )}
        </div>
      )}
    </div>
  );
}
