import {
  useEffect,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from "react";
import type { editor } from "monaco-editor";
import type { Monaco } from "@monaco-editor/react";
import { listen } from "@tauri-apps/api/event";

type RemoteChangeEvent =
  | { type: "insert"; seq: number; position: number; content: string }
  | { type: "delete"; seq: number; position: number; length: number };

interface LogEntry {
  id: number;
  operationClass: string;
  operationLabel: string;
  payload: string;
}

interface UseRemoteChangeListenerArgs {
  editorRef: MutableRefObject<editor.IStandaloneCodeEditor | null>;
  monacoRef: MutableRefObject<Monaco | null>;
  isApplyingRemote: MutableRefObject<boolean>;
  eventCountRef: MutableRefObject<number>;
  setEventLog: Dispatch<SetStateAction<LogEntry[]>>;
  lastAppliedSeqRef: MutableRefObject<number>;
}

export function useRemoteChangeListener({
  editorRef,
  monacoRef,
  isApplyingRemote,
  eventCountRef,
  setEventLog,
  lastAppliedSeqRef,
}: UseRemoteChangeListenerArgs) {
  useEffect(() => {
    const unlistens: Array<() => void> = [];
    let cancelled = false;

    listen<void>("crdt://document-reset", () => {
      lastAppliedSeqRef.current = 0;
    }).then((fn) => {
      if (cancelled) fn();
      else unlistens.push(fn);
    });

    listen<RemoteChangeEvent>("crdt://remote-change", (e) => {
      const ed = editorRef.current;
      const mn = monacoRef.current;
      if (!ed || !mn) return;

      const model = ed.getModel();
      if (!model) return;

      const change = e.payload;
      isApplyingRemote.current = true;
      try {
        if (change.type === "insert") {
          const pos = model.getPositionAt(change.position);
          ed.executeEdits("remote", [
            {
              range: new mn.Range(
                pos.lineNumber,
                pos.column,
                pos.lineNumber,
                pos.column,
              ),
              text: change.content,
              forceMoveMarkers: true,
            },
          ]);

          const count = ++eventCountRef.current;
          setEventLog((prev) => [
            ...prev,
            {
              id: count,
              operationClass: "op-insert",
              operationLabel: "[remote-insert]",
              payload: `offset=${change.position}  text=${JSON.stringify(change.content)}`,
            },
          ]);
        } else {
          const startPos = model.getPositionAt(change.position);
          const endPos = model.getPositionAt(change.position + change.length);
          ed.executeEdits("remote", [
            {
              range: new mn.Range(
                startPos.lineNumber,
                startPos.column,
                endPos.lineNumber,
                endPos.column,
              ),
              text: "",
              forceMoveMarkers: true,
            },
          ]);

          const count = ++eventCountRef.current;
          setEventLog((prev) => [
            ...prev,
            {
              id: count,
              operationClass: "op-delete",
              operationLabel: "[remote-delete]",
              payload: `offset=${change.position}  length=${change.length}`,
            },
          ]);
        }
      } finally {
        if (change.seq > lastAppliedSeqRef.current) {
          lastAppliedSeqRef.current = change.seq;
        }
        isApplyingRemote.current = false;
      }
    }).then((fn) => {
      if (cancelled) fn();
      else unlistens.push(fn);
    });

    return () => {
      cancelled = true;
      while (unlistens.length) {
        unlistens.pop()!();
      }
    };
  }, [
    editorRef,
    monacoRef,
    isApplyingRemote,
    eventCountRef,
    setEventLog,
    lastAppliedSeqRef,
  ]);
}
