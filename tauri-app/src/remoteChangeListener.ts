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
  | {
      type: "insert";
      seq: number;
      position: number;
      content: string;
    }
  | {
      type: "delete";
      seq: number;
      position: number;
      length: number;
    };

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
  shadowTextRef: MutableRefObject<string>;
  /** Called once per applied change; feeds the unsaved-changes indicator. */
  onDocChanged: () => void;
}

function executeRemoteEdit(
  ed: editor.IStandaloneCodeEditor,
  edits: editor.IIdentifiedSingleEditOperation[],
) {
  const wasReadOnly = ed.getRawOptions().readOnly === true;
  if (wasReadOnly) {
    ed.updateOptions({ readOnly: false });
  }
  try {
    ed.executeEdits("crdt", edits);
  } finally {
    if (wasReadOnly) {
      ed.updateOptions({ readOnly: true });
    }
  }
}

export function useRemoteChangeListener({
  editorRef,
  monacoRef,
  isApplyingRemote,
  eventCountRef,
  setEventLog,
  lastAppliedSeqRef,
  shadowTextRef,
  onDocChanged,
}: UseRemoteChangeListenerArgs) {
  useEffect(() => {
    const unlistens: Array<() => void> = [];
    let cancelled = false;

    listen<void>("crdt://document-reset", () => {
      lastAppliedSeqRef.current = 0;
      shadowTextRef.current = "";
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
          executeRemoteEdit(ed, [
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
              operationLabel: "[crdt-insert]",
              payload: `offset=${change.position}  text=${JSON.stringify(change.content)}`,
            },
          ]);
        } else {
          const startPos = model.getPositionAt(change.position);
          const endPos = model.getPositionAt(change.position + change.length);
          executeRemoteEdit(ed, [
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
              operationLabel: "[crdt-delete]",
              payload: `offset=${change.position}  length=${change.length}`,
            },
          ]);
        }
      } finally {
        shadowTextRef.current = model.getValue();
        if (change.seq > lastAppliedSeqRef.current) {
          lastAppliedSeqRef.current = change.seq;
        }
        isApplyingRemote.current = false;
        onDocChanged();
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
    shadowTextRef,
    onDocChanged,
  ]);
}
