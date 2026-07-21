export interface DocumentMeta {
  name: string;
  path: string;
  size_bytes: number;
  modified: number | null;
}

export interface CurrentFileInfo {
  name: string;
  path: string;
  had_crlf: boolean;
}

// Open and save dialogs deliberately have NO filters: any readable file can
// be opened or saved under any name, and platform dialogs default to the
// first filter, hiding everything else behind a dropdown (GTK additionally
// treats "*" as "*.*", which misses extensionless files).

export function parentFolder(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx > 0 ? path.slice(0, idx) : path;
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
}

export function formatDate(ts: number | null): string {
  if (!ts) return "—";
  return new Date(ts * 1000).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}
