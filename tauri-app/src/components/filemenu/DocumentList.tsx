import {
  type DocumentMeta,
  formatDate,
  formatSize,
  parentFolder,
} from "./format";
import { IconReveal, IconTrash } from "./icons";

interface DocumentRowProps {
  doc: DocumentMeta;
  busy: boolean;
  onOpen: (doc: DocumentMeta) => void;
  onReveal: (path: string) => void;
  onDelete: (name: string) => void;
}

function DocumentRow({
  doc,
  busy,
  onOpen,
  onReveal,
  onDelete,
}: DocumentRowProps) {
  return (
    <div className="file-dropdown-doc">
      <button
        className="file-dropdown-doc-name"
        onClick={() => onOpen(doc)}
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
      {!doc.external && (
        <button
          className="file-dropdown-doc-action danger"
          onClick={() => onDelete(doc.name)}
          disabled={busy}
          title="Delete"
        >
          <IconTrash />
        </button>
      )}
    </div>
  );
}

interface DocumentListProps {
  docs: DocumentMeta[];
  docsDir: string | null;
  busy: boolean;
  onOpen: (doc: DocumentMeta) => void;
  onReveal: (path: string) => void;
  onDelete: (name: string) => void;
  onBack: () => void;
}

export function DocumentList(props: DocumentListProps) {
  return (
    <div className="file-dropdown-form">
      <div className="file-dropdown-title">Open document</div>
      {props.docsDir && (
        <div className="file-dropdown-subtitle" title={props.docsDir}>
          Library: {props.docsDir}
        </div>
      )}
      {props.docs.length === 0 ? (
        <div className="file-dropdown-subtitle">No saved documents yet.</div>
      ) : (
        <div className="file-dropdown-list">
          {props.docs.map((d) => (
            <DocumentRow
              key={d.path}
              doc={d}
              busy={props.busy}
              onOpen={props.onOpen}
              onReveal={props.onReveal}
              onDelete={props.onDelete}
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
