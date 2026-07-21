import { useState, useRef, useCallback, useMemo, useEffect } from "react";
import type { editor } from "monaco-editor";
import Editor, { type OnMount, type Monaco } from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/core";
import { useRemoteChangeListener } from "./remoteChangeListener";
import { useSnapshotListener } from "./snapshotListener";
import { createEnqueueOp, createIpcSenders } from "./opQueue";
import { normalizeToLF, forceModelLF } from "./eol";
import { UsernameGate, ChangeNameModal } from "./usernameSetup";
import { FileMenu } from "./components/filemenu/FileMenu";
import { SessionPanel } from "./components/SessionPanel";
import { PeersPanel } from "./components/PeersPanel";
import { StatusLine, type CursorPos } from "./components/StatusLine";
import { useWritePermission } from "./hooks/useWritePermission";
import { useSessionEvents, type SessionNotice } from "./hooks/useSessionEvents";
import { useRoomState } from "./hooks/useRoomState";
import type { CurrentFileInfo } from "./components/filemenu/format";
import { PEERCODE_THEME, registerPeercodeTheme } from "./monacoTheme";
import "./App.css";

const SESSION_NOTICE_MESSAGE: Record<SessionNotice, string> = {
  ended: "The host ended the session. Your document is preserved.",
  disconnected: "Connection lost. Your document is preserved locally.",
};

interface LogEntry {
  id: number;
  operationClass: string;
  operationLabel: string;
  payload: string;
  wireMessage?: string;
}

function installPlainTextPasteHandler(
  editorInstance: editor.IStandaloneCodeEditor,
) {
  const domNode = editorInstance.getDomNode();
  if (!domNode) return;

  const handlePaste = (event: ClipboardEvent) => {
    event.preventDefault();
    event.stopPropagation();

    const text = normalizeToLF(
      event.clipboardData?.getData("text/plain") ?? "",
    );

    if (text) {
      editorInstance.focus();
      editorInstance.trigger("plain-text-paste", "type", { text });
      return;
    }

    void navigator.clipboard.readText().then((clipText) => {
      if (!clipText) return;
      editorInstance.focus();
      editorInstance.trigger("plain-text-paste", "type", {
        text: normalizeToLF(clipText),
      });
    });
  };

  domNode.addEventListener("paste", handlePaste, { capture: true });
  editorInstance.onDidDispose(() => {
    domNode.removeEventListener("paste", handlePaste, { capture: true });
  });
}

interface AppContentProps {
  username: string;
  onUsernameChange: (name: string) => void;
}

function AppContent({ username, onUsernameChange }: AppContentProps) {
  const isDevFeaturesEnabled = import.meta.env.VITE_DEV_FEATURES === "true";
  const [statusReady, setStatusReady] = useState(false);
  const [eventLog, setEventLog] = useState<LogEntry[]>([]);
  const eventCountRef = useRef(0);
  const logRef = useRef<HTMLDivElement>(null);
  const [logOpen, setLogOpen] = useState(false);
  const [showRename, setShowRename] = useState(false);
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<Monaco | null>(null);
  const isApplyingRemote = useRef(false);
  const lastAppliedSeqRef = useRef(0);
  const shadowTextRef = useRef("");
  const opChainRef = useRef<Promise<unknown>>(Promise.resolve());
  const [dirty, setDirty] = useState(false);
  const markDirty = useCallback(() => setDirty(true), []);

  const session = useSessionEvents();
  const { roomState, clearRoomState } = useRoomState();
  const [peersOpen, setPeersOpen] = useState(false);
  const isHost = session.sessionStatus === "host";
  const inSession = isHost || session.sessionStatus === "guest";

  useEffect(() => {
    if (!inSession) setPeersOpen(false);
  }, [inSession]);

  const [cursor, setCursor] = useState<CursorPos>({ line: 1, col: 1 });
  const [fileInfo, setFileInfo] = useState<CurrentFileInfo | null>(null);

  const enqueueOp = useMemo(() => createEnqueueOp(opChainRef), []);
  const { sendInsert, sendDelete, sendReplace } = useMemo(
    () => createIpcSenders(enqueueOp),
    [enqueueOp],
  );
  const canWrite = useWritePermission(editorRef);

  // Replaces the whole editor content without echoing ops back to the
  // backend. Normalizes to LF and re-pins the model EOL: on Windows,
  // setValue re-infers CRLF from the text, which would shift every offset
  // relative to peers.
  const replaceEditorText = useCallback((text: string) => {
    const normalized = normalizeToLF(text);
    const ed = editorRef.current;
    if (ed) {
      isApplyingRemote.current = true;
      try {
        ed.setValue(normalized);
        forceModelLF(ed, monacoRef.current);
      } finally {
        isApplyingRemote.current = false;
      }
    }
    shadowTextRef.current = normalized;
    setDirty(false);
  }, []);

  const handleDocumentLoaded = useCallback(
    (text: string, name: string) => {
      replaceEditorText(text);
      const count = ++eventCountRef.current;
      setEventLog((prev) => [
        ...prev,
        {
          id: count,
          operationClass: "op-insert",
          operationLabel: "[loaded]",
          payload: `document "${name}" (${text.length} chars)`,
        },
      ]);
    },
    [replaceEditorText],
  );

  const getEditorContent = useCallback(
    () => editorRef.current?.getValue() ?? "",
    [],
  );

  const resetDocAndEditor = useCallback(async () => {
    await invoke("reset_document");
    replaceEditorText("");
  }, [replaceEditorText]);

  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [eventLog]);

  useRemoteChangeListener({
    editorRef,
    monacoRef,
    isApplyingRemote,
    eventCountRef,
    setEventLog,
    lastAppliedSeqRef,
    shadowTextRef,
    onDocChanged: markDirty,
  });

  useSnapshotListener({
    editorRef,
    monacoRef,
    isApplyingRemote,
    eventCountRef,
    setEventLog,
    shadowTextRef,
  });

  const [loggingEnabled, setLoggingEnabled] = useState(false);
  const toggleLogging = async () => {
    if (!isDevFeaturesEnabled) return;
    await invoke("toggle_crdt_logging");
    setLoggingEnabled((prev) => !prev);
  };

  const handleEditorMount: OnMount = (editorInstance, monacoInstance) => {
    editorRef.current = editorInstance;
    monacoRef.current = monacoInstance;

    // Force LF on all platforms. Without this, Windows/WebView2 defaults to
    // CRLF which makes every newline 2 bytes in the model, shifting all offsets
    // relative to Linux/macOS peers and causing divergence.
    forceModelLF(editorInstance, monacoInstance);

    installPlainTextPasteHandler(editorInstance);
    editorInstance.onDidChangeCursorPosition((e) => {
      setCursor({ line: e.position.lineNumber, col: e.position.column });
    });
    setStatusReady(true);
    shadowTextRef.current = editorInstance.getModel()?.getValue() ?? "";

    editorInstance.onDidChangeModelContent(
      (event: editor.IModelContentChangedEvent) => {
        if (isApplyingRemote.current) return;

        const model = editorInstance.getModel();
        if (!model) return;

        // Capture the user's changes relative to the shadow text (pre-edit state)
        const changes = event.changes.map((c) => ({
          offset: c.rangeOffset,
          deleteLen: c.rangeLength,
          text: normalizeToLF(c.text),
        }));

        // Revert Monaco to the shadow text — the backend will emit events
        // that apply the confirmed edit back to Monaco.
        isApplyingRemote.current = true;
        const fullRange = model.getFullModelRange();
        editorInstance.executeEdits("revert", [
          {
            range: new monacoInstance.Range(
              fullRange.startLineNumber,
              fullRange.startColumn,
              fullRange.endLineNumber,
              fullRange.endColumn,
            ),
            text: shadowTextRef.current,
            forceMoveMarkers: false,
          },
        ]);
        const primaryChange = changes[0];
        if (primaryChange) {
          editorInstance.setPosition(model.getPositionAt(primaryChange.offset));
        }
        isApplyingRemote.current = false;

        const baseSeq = lastAppliedSeqRef.current;
        void (async () => {
          for (const change of changes) {
            try {
              if (change.deleteLen > 0 && change.text.length > 0) {
                await sendReplace(
                  change.offset,
                  change.deleteLen,
                  change.text,
                  baseSeq,
                );
              } else if (change.deleteLen > 0) {
                await sendDelete(change.offset, change.deleteLen, baseSeq);
              } else {
                await sendInsert(change.offset, change.text, baseSeq);
              }
            } catch (error) {
              const count = ++eventCountRef.current;
              setEventLog((prev) => [
                ...prev,
                {
                  id: count,
                  operationClass: "op-delete",
                  operationLabel: "[ipc-error]",
                  payload: String(error),
                },
              ]);
              return;
            }
          }
        })();
      },
    );
  };

  return (
    <>
      <div className="toolbar">
        <span className="toolbar-brand">
          Peer<span className="toolbar-brand-accent">Code</span>
          <span className="brand-cursor">▍</span>
        </span>
        <FileMenu
          onDocumentLoaded={handleDocumentLoaded}
          dirty={dirty}
          onSaved={() => setDirty(false)}
          onCurrentChanged={setFileInfo}
        />
        {isDevFeaturesEnabled && (
          <button
            onClick={() => void toggleLogging()}
            className={"crdt-log-btn" + (loggingEnabled ? " on" : "")}
          >
            CRDT log {loggingEnabled ? "ON" : "OFF"}
          </button>
        )}
        {username && (
          <button
            className="toolbar-username"
            title="Change display name"
            onClick={() => setShowRename(true)}
          >
            {username}
          </button>
        )}
      </div>
      <div className="session-bar">
        <span className="session-bar-label">session</span>
        <SessionPanel
          getEditorContent={getEditorContent}
          resetDocAndEditor={resetDocAndEditor}
          session={session}
          clearRoomState={clearRoomState}
        />
      </div>
      {showRename && (
        <ChangeNameModal
          current={username}
          onDone={(name) => {
            onUsernameChange(name);
            setShowRename(false);
          }}
          onCancel={() => setShowRename(false)}
        />
      )}
      {session.sessionNotice && (
        <div className={`notice-strip ${session.sessionNotice}`}>
          {SESSION_NOTICE_MESSAGE[session.sessionNotice]}
        </div>
      )}
      <div className="editor-container">
        <Editor
          height="100%"
          defaultLanguage="rust"
          defaultValue=""
          theme={PEERCODE_THEME}
          beforeMount={registerPeercodeTheme}
          onMount={handleEditorMount}
          options={{
            fontSize: 14,
            fontFamily:
              '"JetBrains Mono", "Cascadia Code", Consolas, monospace',
            automaticLayout: true,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
          }}
        />
      </div>
      <button
        className="log-header"
        onClick={() => setLogOpen((prev) => !prev)}
        title={logOpen ? "Collapse event log" : "Expand event log"}
      >
        {logOpen ? "▾" : "▸"} change event log ({eventLog.length})
      </button>
      {logOpen && (
        <div className="event-log" ref={logRef}>
          {eventLog.map((entry) => (
            <div className="entry" key={entry.id}>
              <span className="label">#{entry.id}</span>
              <span className={entry.operationClass}>
                {entry.operationLabel}
              </span>{" "}
              {entry.payload}
              {entry.wireMessage && (
                <span className="entry-wire">
                  {" "}
                  {"->"} wire: {entry.wireMessage}
                </span>
              )}
            </div>
          ))}
        </div>
      )}
      <StatusLine
        sessionStatus={session.sessionStatus}
        roomId={session.roomId}
        shareUrl={session.publicUrl ?? session.lanUrl}
        peerCount={roomState?.peers.length ?? 0}
        inSession={inSession}
        onPeersClick={() => setPeersOpen((prev) => !prev)}
        fileName={fileInfo?.name ?? null}
        dirty={dirty}
        hadCrlf={fileInfo?.had_crlf ?? false}
        canWrite={canWrite}
        statusReady={statusReady}
        cursor={cursor}
      />
      {inSession && (
        <PeersPanel
          roomState={roomState}
          isHost={isHost}
          open={peersOpen}
          onClose={() => setPeersOpen(false)}
        />
      )}
    </>
  );
}

function App() {
  return (
    <UsernameGate>
      {(username, setUsername) => (
        <AppContent username={username} onUsernameChange={setUsername} />
      )}
    </UsernameGate>
  );
}

export default App;
