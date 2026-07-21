import { useState, useEffect, useCallback, type MutableRefObject } from "react";
import type { editor } from "monaco-editor";
import { useTauriEvents } from "./useTauriEvents";

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

  useTauriEvents(
    useCallback((on) => {
      on<CanWritePayload>("session://can-write", (payload) =>
        setCanWrite(payload.can_write),
      );
      on("session://session-ended", () => setCanWrite(true));
      on("session://disconnected", () => setCanWrite(true));
    }, []),
  );

  useEffect(() => {
    editorRef.current?.updateOptions({ readOnly: !canWrite });
  }, [canWrite, editorRef]);

  return canWrite;
}
