import {
  type DocumentMeta,
  formatDate,
  formatSize,
  parentFolder,
} from "./format";
import { IconReveal, IconTrash } from "./icons";

interface RecentRowProps {
  doc: DocumentMeta;
  busy: boolean;
  onOpen: (path: string) => void;
  onReveal: (path: string) => void;
  onRemove: (path: string) => void;
}

function RecentRow({ doc, busy, onOpen, onReveal, onRemove }: RecentRowProps) {
  return (
    <div className="file-dropdown-doc">
      <button
        className="file-dropdown-doc-name"
        onClick={() => onOpen(doc.path)}
        disabled={busy}
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
        onClick={() => onReveal(doc.path)}
        disabled={busy}
        title="Show in file manager"
      >
        <IconReveal />
      </button>
      <button
        className="file-dropdown-doc-action"
        onClick={() => onRemove(doc.path)}
        disabled={busy}
        title="Remove from list (keeps the file)"
      >
        <IconTrash />
      </button>
    </div>
  );
}

interface RecentsListProps {
  recents: DocumentMeta[];
  busy: boolean;
  onOpen: (path: string) => void;
  onReveal: (path: string) => void;
  onRemove: (path: string) => void;
  onBack: () => void;
}

export function RecentsList(props: RecentsListProps) {
  return (
    <div className="file-dropdown-form">
      <div className="file-dropdown-title">Recent files</div>
      {props.recents.length === 0 ? (
        <div className="file-dropdown-subtitle">
          Nothing opened yet. Use Open from… to browse.
        </div>
      ) : (
        <div className="file-dropdown-list">
          {props.recents.map((d) => (
            <RecentRow
              key={d.path}
              doc={d}
              busy={props.busy}
              onOpen={props.onOpen}
              onReveal={props.onReveal}
              onRemove={props.onRemove}
            />
          ))}
        </div>
      )}
      <div className="file-dropdown-actions">
        <button className="file-dropdown-btn secondary" onClick={props.onBack}>
          Back
        </button>
      </div>
    </div>
  );
}
