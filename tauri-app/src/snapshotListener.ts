import {
  useEffect,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from "react";
import type { editor } from "monaco-editor";
import type { Monaco } from "@monaco-editor/react";
import { listen } from "@tauri-apps/api/event";
import type { PendingOpStore } from "./opQueue";

interface LogEntry {
  id: number;
  operationClass: string;
  operationLabel: string;
  payload: string;
}

interface UseSnapshotListenerArgs {
  editorRef: MutableRefObject<editor.IStandaloneCodeEditor | null>;
  monacoRef: MutableRefObject<Monaco | null>;
  isApplyingRemote: MutableRefObject<boolean>;
  eventCountRef: MutableRefObject<number>;
  setEventLog: Dispatch<SetStateAction<LogEntry[]>>;
  pendingStore: PendingOpStore;
}

export function useSnapshotListener({
  editorRef,
  monacoRef,
  isApplyingRemote,
  eventCountRef,
  setEventLog,
  pendingStore,
}: UseSnapshotListenerArgs) {
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    listen<{ text: string }>("crdt://snapshot-applied", (e) => {
      const ed = editorRef.current;
      const mn = monacoRef.current;
      if (!ed) return;

      // Normalize to LF before setting — snapshot text from the CRDT should
      // only ever contain \n, but guard against any stale \r\n that crept in.
      const normalizedText = e.payload.text
        .replace(/\r\n/g, "\n")
        .replace(/\r/g, "\n");

      isApplyingRemote.current = true;
      try {
        ed.setValue(normalizedText);
        // setValue re-detects EOL from the content; re-pin to LF so that
        // subsequent local edits on Windows still produce \n offsets.
        if (mn) {
          ed.getModel()?.setEOL(mn.editor.EndOfLineSequence.LF);
        }
      } finally {
        isApplyingRemote.current = false;
      }
      pendingStore.reset();

      const count = ++eventCountRef.current;
      setEventLog((prev) => [
        ...prev,
        {
          id: count,
          operationClass: "op-snapshot",
          operationLabel: "[snapshot-applied]",
          payload: `text_len=${e.payload.text.length}`,
        },
      ]);
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };
  }, [
    editorRef,
    monacoRef,
    isApplyingRemote,
    eventCountRef,
    setEventLog,
    pendingStore,
  ]);
}
