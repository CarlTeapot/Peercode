import { useEffect } from "react";
import type { ComponentType, SVGProps } from "react";
import type { FileMenuApi } from "../filemenu/useFileMenu";
import {
  IconFolderOpen,
  IconFork,
  IconReveal,
  IconSave,
  IconSaveAs,
  IconTrash,
} from "../filemenu/icons";
import {
  type DocumentMeta,
  formatDate,
  formatSize,
  parentFolder,
} from "../filemenu/format";

interface FilesSectionProps {
  menu: FileMenuApi;
  locked: boolean;
}

/** Files section of the side panel: save/open actions + recent files. */
export function FilesSection({ menu, locked }: FilesSectionProps) {
  const { refreshAll } = menu;

  // Re-list recents (pruning dead paths) every time the section opens.
  useEffect(() => {
    void refreshAll();
  }, [refreshAll]);

  return (
    <div className="panel-section files-section">
      <FileAction
        icon={IconSave}
        label="Save"
        detail={menu.current?.name ?? "untitled"}
        disabled={menu.busy}
        onClick={() => void menu.saveCurrent()}
      />
      <FileAction
        icon={IconSaveAs}
        label="Save as…"
        disabled={menu.busy}
        onClick={() => void menu.saveAs()}
      />
      <FileAction
        icon={IconFolderOpen}
        label="Open from…"
        disabled={menu.busy || locked}
        onClick={() => void menu.openFrom()}
      />
      <FileAction
        icon={IconFork}
        label="Fork"
        disabled={menu.busy || locked}
        onClick={() => void menu.fork()}
      />
      {locked && (
        <p className="panel-hint">
          In a session — leave it to open a different file.
        </p>
      )}
      <div className="file-dropdown-separator files-separator" />
      <div className="file-dropdown-section">recent files</div>
      {menu.recents.length === 0 ? (
        <p className="panel-hint">
          Nothing opened yet. Use Open from… to browse.
        </p>
      ) : (
        <div className="file-dropdown-list files-recents">
          {menu.recents.map((d) => (
            <RecentRow key={d.path} doc={d} menu={menu} locked={locked} />
          ))}
        </div>
      )}
      {menu.error && <div className="file-dropdown-error">{menu.error}</div>}
    </div>
  );
}

interface FileActionProps {
  icon: ComponentType<SVGProps<SVGSVGElement>>;
  label: string;
  detail?: string;
  disabled: boolean;
  onClick: () => void;
}

function FileAction({
  icon: Icon,
  label,
  detail,
  disabled,
  onClick,
}: FileActionProps) {
  return (
    <button
      className="file-dropdown-item"
      onClick={onClick}
      disabled={disabled}
    >
      <Icon className="file-dropdown-item-icon" />
      <span>{label}</span>
      {detail && <span className="file-dropdown-item-detail">{detail}</span>}
    </button>
  );
}

function RecentRow({
  doc,
  menu,
  locked,
}: {
  doc: DocumentMeta;
  menu: FileMenuApi;
  locked: boolean;
}) {
  return (
    <div className="file-dropdown-doc">
      <button
        className="file-dropdown-doc-name"
        onClick={() => void menu.openPath(doc.path)}
        disabled={menu.busy || locked}
      >
        <span>{doc.name}</span>
        <span className="file-dropdown-doc-meta">
          {formatSize(doc.size_bytes)} · {formatDate(doc.modified)}
        </span>
        <span className="file-dropdown-doc-path" title={doc.path}>
          {parentFolder(doc.path)}
        </span>
      </button>
      <button
        className="file-dropdown-doc-action"
        onClick={() => void menu.reveal(doc.path)}
        disabled={menu.busy}
        title="Show in file manager"
      >
        <IconReveal />
      </button>
      <button
        className="file-dropdown-doc-action"
        onClick={() => void menu.removeRecent(doc.path)}
        disabled={menu.busy}
        title="Remove from list (keeps the file)"
      >
        <IconTrash />
      </button>
    </div>
  );
}
