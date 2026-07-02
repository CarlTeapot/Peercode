export interface DocumentMeta {
  name: string;
  path: string;
  size_bytes: number;
  modified: number | null;
  external: boolean;
}

// Save dialogs list pcdoc first so it drives the suggested extension.
export const SAVE_FILTERS = [
  { name: "PeerCode documents", extensions: ["pcdoc"] },
  { name: "All files", extensions: ["*"] },
];

// The open dialog deliberately has NO filters: any readable file can be
// imported, and platform dialogs default to the first filter, hiding
// everything else behind a dropdown (GTK additionally treats "*" as "*.*",
// which misses extensionless files).

export const EXPORT_FILTERS = [
  { name: "All files", extensions: ["*"] },
  { name: "Text files", extensions: ["txt"] },
];

export function isPcdoc(path: string): boolean {
  return path.toLowerCase().endsWith(".pcdoc");
}

export function fileName(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx >= 0 ? path.slice(idx + 1) : path;
}

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
