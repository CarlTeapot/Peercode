import { invoke } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import type { CurrentFileInfo } from "./format";

/**
 * Saves to the current file, or asks where via the native dialog when the
 * buffer is untitled. Returns the saved path, or null if the user cancelled.
 */
export async function saveBuffer(
  current: CurrentFileInfo | null,
  docsDir: string | null,
): Promise<string | null> {
  if (current) {
    await invoke("save_file");
    return current.path;
  }
  return saveBufferAs(null, docsDir);
}

/** Always asks for a path (Save as…). Returns it, or null if cancelled. */
export async function saveBufferAs(
  current: CurrentFileInfo | null,
  docsDir: string | null,
): Promise<string | null> {
  const filePath = await saveDialog({
    title: "Save as…",
    defaultPath:
      current?.path ?? (docsDir ? `${docsDir}/untitled.txt` : "untitled.txt"),
  });
  if (!filePath) return null;
  await invoke("save_file_as", { path: filePath });
  return filePath;
}
