import { useEffect, useRef } from "react";
import { MenuList } from "./MenuList";
import { RecentsList } from "./RecentsList";
import { useFileMenu } from "./useFileMenu";
import type { CurrentFileInfo } from "./format";

interface FileMenuProps {
  onDocumentLoaded: (text: string, name: string) => void;
  dirty: boolean;
  onSaved: () => void;
  onCurrentChanged?: (info: CurrentFileInfo | null) => void;
}

export function FileMenu({
  onDocumentLoaded,
  dirty,
  onSaved,
  onCurrentChanged,
}: FileMenuProps) {
  const menu = useFileMenu(onDocumentLoaded, onSaved, onCurrentChanged);
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
        {menu.current && (
          <span className="file-menu-current">{menu.current.name}</span>
        )}
        {dirty && (
          <span className="file-menu-dirty" title="Unsaved changes">
            ●
          </span>
        )}
      </button>

      {menu.open && (
        <div className="file-dropdown">
          {menu.view === "menu" && (
            <MenuList
              currentName={menu.current?.name ?? null}
              busy={menu.busy}
              onSave={() => void menu.saveCurrent()}
              onSaveAs={() => void menu.saveAs()}
              onOpenRecents={() => menu.showView("recents")}
              onOpenFrom={() => void menu.openFrom()}
              onFork={() => void menu.fork()}
            />
          )}

          {menu.view === "recents" && (
            <RecentsList
              recents={menu.recents}
              busy={menu.busy}
              onOpen={(path) => void menu.openPath(path)}
              onReveal={(path) => void menu.reveal(path)}
              onRemove={(path) => void menu.removeRecent(path)}
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
