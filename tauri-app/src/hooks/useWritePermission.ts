import { useState, useEffect, type MutableRefObject } from "react";
import { listen } from "@tauri-apps/api/event";
import type { editor } from "monaco-editor";

interface CanWritePayload {
  can_write: boolean;
}

/**
 * Tracks the local user's write permission (host-granted for guests) and
 * mirrors it into Monaco's readOnly option. The backend command guard is the
 * authority; this only keeps the UI honest. Editing is re-enabled when the
 * session ends in any way.
 */
export function useWritePermission(
  editorRef: MutableRefObject<editor.IStandaloneCodeEditor | null>,
) {
  const [canWrite, setCanWrite] = useState(true);

  useEffect(() => {
    const unlisten: (() => void)[] = [];
    let cancelled = false;

    void (async () => {
      const register = (fn: () => void) => {
        if (cancelled) fn();
        else unlisten.push(fn);
      };

      register(
        await listen<CanWritePayload>("session://can-write", (e) => {
          setCanWrite(e.payload.can_write);
        }),
      );
      register(
        await listen("session://session-ended", () => setCanWrite(true)),
      );
      register(await listen("session://disconnected", () => setCanWrite(true)));
    })();

    return () => {
      cancelled = true;
      unlisten.forEach((fn) => fn());
    };
  }, []);

  useEffect(() => {
    editorRef.current?.updateOptions({ readOnly: !canWrite });
  }, [canWrite, editorRef]);

  return canWrite;
}
