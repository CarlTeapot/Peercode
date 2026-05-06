import { useState, useRef, useEffect, useCallback } from "react";
import type { editor } from "monaco-editor";
import Editor, { type OnMount, type Monaco } from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useRemoteChangeListener } from "./remoteChangeListener";
import { UsernameGate } from "./usernameSetup";
import { FileMenu } from "./FileMenu";
import "./App.css";

interface LogEntry {
  id: number;
  operationClass: string;
  operationLabel: string;
  payload: string;
  wireMessage?: string;
}

interface AppContentProps {
  username: string;
}

function AppContent({ username }: AppContentProps) {
  const isDevFeaturesEnabled = import.meta.env.VITE_DEV_FEATURES === "true";
  const [status, setStatus] = useState("loading...");
  const [statusReady, setStatusReady] = useState(false);
  const [eventLog, setEventLog] = useState<LogEntry[]>([]);
  const eventCountRef = useRef(0);
  const logRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<Monaco | null>(null);
  const isApplyingRemote = useRef(false);

  const handleDocumentLoaded = useCallback((text: string, name: string) => {
    const ed = editorRef.current;
    if (ed) {
      ed.setValue(text);
    }
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
  }, []);

  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [eventLog]);

  const sendInsert = async (position: number, content: string) => {
    await invoke("insert", { position, content });
  };

  const sendDelete = async (position: number, length: number) => {
    await invoke("delete", { position, length });
  };

  // --- session links ---
  const [lanUrl, setLanUrl] = useState<string | null>(null);
  const [publicUrl, setPublicUrl] = useState<string | null>(null);
  const [sessionStatus, setSessionStatus] = useState<string>("starting...");
  const [copyStatus, setCopyStatus] = useState<string | null>(null);

  const copyUrl = async (label: string, url: string) => {
    try {
      await navigator.clipboard.writeText(url);
      setCopyStatus(`${label} URL copied`);
      window.setTimeout(() => setCopyStatus(null), 1500);
    } catch {
      setCopyStatus(`Failed to copy ${label} URL`);
      window.setTimeout(() => setCopyStatus(null), 1500);
    }
  };

  useEffect(() => {
    invoke<{
      status: string;
      lan_url: string | null;
      public_url: string | null;
    }>("get_session_info").then((info) => {
      setSessionStatus(info.status);
      if (info.lan_url) setLanUrl(info.lan_url);
      if (info.public_url) setPublicUrl(info.public_url);
    });

    const unlisten: (() => void)[] = [];
    (async () => {
      unlisten.push(
        await listen<{
          lan_url: string | null;
          public_url: string | null;
          port: number;
          room_id: string;
        }>("session://session-ready", (e) => {
          setSessionStatus("host");
          if (e.payload.lan_url) setLanUrl(e.payload.lan_url);
          if (e.payload.public_url) setPublicUrl(e.payload.public_url);
        }),
      );
      unlisten.push(
        await listen<{ message: string }>("session://session-error", (e) => {
          setSessionStatus("error: " + e.payload.message);
        }),
      );
    })();

    return () => unlisten.forEach((fn) => fn());
  }, []);
  // --- end session links ---

  useRemoteChangeListener({
    editorRef,
    monacoRef,
    isApplyingRemote,
    eventCountRef,
    setEventLog,
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
    setStatus("editor ready");
    setStatusReady(true);

    editorInstance.onDidChangeModelContent(
      (event: editor.IModelContentChangedEvent) => {
        // Skip changes that we ourselves applied from a remote peer.
        if (isApplyingRemote.current) return;

        void (async () => {
          for (const change of event.changes) {
            const offset = change.rangeOffset;
            const deleteLen = change.rangeLength;
            const insertText = change.text;

            let opType: string, opClass: string, payload: string;
            if (deleteLen > 0 && insertText.length > 0) {
              opType = "replace";
              opClass = "op-replace";
              payload = `offset=${offset}  deleteLength=${deleteLen}  text=${JSON.stringify(insertText)}`;
            } else if (deleteLen > 0) {
              opType = "delete";
              opClass = "op-delete";
              payload = `offset=${offset}  deleteLength=${deleteLen}`;
            } else {
              opType = "insert";
              opClass = "op-insert";
              payload = `offset=${offset}  text=${JSON.stringify(insertText)}`;
            }

            const wireMessage = JSON.stringify({
              type: opType,
              offset,
              ...(deleteLen > 0 && { length: deleteLen }),
              ...(insertText.length > 0 && { text: insertText }),
            });

            try {
              if (deleteLen > 0) {
                await sendDelete(offset, deleteLen);
              }
              if (insertText.length > 0) {
                await sendInsert(offset, insertText);
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
              setStatus("ipc error");
              return;
            }

            const count = ++eventCountRef.current;
            setEventLog((prev) => [
              ...prev,
              {
                id: count,
                operationClass: opClass,
                operationLabel: `[${opType}]`,
                payload,
                wireMessage,
              },
            ]);
          }
        })();
      },
    );
  };

  return (
    <>
      <div className="toolbar">
        <FileMenu onDocumentLoaded={handleDocumentLoaded} />
        <span>Monaco Test Harness</span>
        {username && (
          <span
            style={{
              color: "#7ab",
              fontFamily: "monospace",
              fontSize: 12,
              padding: "2px 8px",
              background: "#1a2a3a",
              borderRadius: 3,
            }}
          >
            {username}
          </span>
        )}
        {isDevFeaturesEnabled && (
          <button
            onClick={toggleLogging}
            style={{
              background: loggingEnabled ? "#4a9" : "#555",
              border: "none",
              color: "white",
              padding: "2px 10px",
              cursor: "pointer",
              borderRadius: "3px",
            }}
          >
            CRDT log {loggingEnabled ? "ON" : "OFF"}
          </button>
        )}
        <span className={`status ${statusReady ? "ready" : ""}`}>{status}</span>
      </div>
      {/* session link panel */}
      <div
        style={{
          padding: "8px",
          background: "#1a1a2e",
          borderBottom: "1px solid #333",
          fontFamily: "monospace",
          fontSize: "12px",
        }}
      >
        <span style={{ color: "#aaa" }}>Session: </span>
        <span
          style={{ color: sessionStatus.startsWith("error") ? "#f55" : "#0f0" }}
        >
          {sessionStatus}
        </span>
        {lanUrl && (
          <div
            style={{
              marginTop: 4,
              display: "flex",
              gap: 8,
              alignItems: "center",
            }}
          >
            <span style={{ color: "#aaa" }}>LAN: </span>
            <span style={{ color: "#0ff" }}>{lanUrl}</span>
            <button
              onClick={() => void copyUrl("LAN", lanUrl)}
              style={{
                fontSize: 11,
                padding: "1px 6px",
                borderRadius: 3,
                border: "1px solid #555",
                background: "#2c2c3d",
                color: "#ddd",
                cursor: "pointer",
              }}
            >
              Copy
            </button>
          </div>
        )}
        {publicUrl && (
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <span style={{ color: "#aaa" }}>Public: </span>
            <span style={{ color: "#0ff" }}>{publicUrl}</span>
            <button
              onClick={() => void copyUrl("Public", publicUrl)}
              style={{
                fontSize: 11,
                padding: "1px 6px",
                borderRadius: 3,
                border: "1px solid #555",
                background: "#2c2c3d",
                color: "#ddd",
                cursor: "pointer",
              }}
            >
              Copy
            </button>
          </div>
        )}
        {copyStatus && (
          <div style={{ color: "#9ad", marginTop: 4 }}>{copyStatus}</div>
        )}
        {!lanUrl && !publicUrl && sessionStatus === "starting..." && (
          <span style={{ color: "#888", marginLeft: 8 }}>
            waiting for session readiness...
          </span>
        )}
      </div>
      <div className="editor-container">
        <Editor
          height="100%"
          defaultLanguage="rust"
          defaultValue={[].join("\n")}
          theme="vs-dark"
          onMount={handleEditorMount}
          options={{
            fontSize: 14,
            automaticLayout: true,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
          }}
        />
      </div>
      <div className="log-header">
        change event log ? this is what your rust process will receive
      </div>
      <div className="event-log" ref={logRef}>
        {eventLog.map((entry) => (
          <div className="entry" key={entry.id}>
            <span className="label">#{entry.id}</span>
            <span className={entry.operationClass}>
              {entry.operationLabel}
            </span>{" "}
            {entry.payload}
            {entry.wireMessage && (
              <span style={{ color: "#555", marginLeft: 12 }}>
                {" "}
                {"->"} wire: {entry.wireMessage}
              </span>
            )}
          </div>
        ))}
      </div>
    </>
  );
}

function App() {
  return (
    <UsernameGate>
      {(username) => <AppContent username={username} />}
    </UsernameGate>
  );
}

export default App;
