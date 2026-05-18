import {
  useState,
  useRef,
  useEffect,
  useCallback,
  useMemo,
  type FormEvent,
} from "react";
import type { editor } from "monaco-editor";
import Editor, { type OnMount, type Monaco } from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useRemoteChangeListener } from "./remoteChangeListener";
import { useSnapshotListener } from "./snapshotListener";
import { createEnqueueOp, createIpcSenders } from "./opQueue";
import {
  UsernameGate,
  overlayStyle,
  cardStyle,
  inputStyle,
  btnStyle,
  errorStyle,
} from "./usernameSetup";
import { FileMenu } from "./FileMenu";
import "./App.css";

interface LogEntry {
  id: number;
  operationClass: string;
  operationLabel: string;
  payload: string;
  wireMessage?: string;
}

// --- Save-before-session modal ---
interface SavePromptProps {
  onSaveAndContinue: (name: string) => Promise<void>;
  onDiscardAndContinue: () => void;
  onCancel: () => void;
}

function SaveBeforeSessionModal({
  onSaveAndContinue,
  onDiscardAndContinue,
  onCancel,
}: SavePromptProps) {
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    invoke<string | null>("get_current_document_name").then((n) => {
      if (n) setName(n);
    });
  }, []);

  const handleSave = async () => {
    if (!name.trim()) {
      setError("Enter a document name");
      return;
    }
    setBusy(true);
    setError("");
    try {
      await onSaveAndContinue(name.trim());
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  };

  return (
    <div style={overlayStyle}>
      <div style={cardStyle}>
        <div style={{ fontSize: 16, fontWeight: "bold", color: "#eee" }}>
          Unsaved changes
        </div>
        <div style={{ fontSize: 13, color: "#aaa" }}>
          Your document has content. Save it before starting a new session?
        </div>
        <input
          style={inputStyle}
          autoFocus
          placeholder="Document name"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setError("");
          }}
        />
        <div style={errorStyle}>{error}</div>
        <button
          style={btnStyle(busy || !name.trim())}
          disabled={busy || !name.trim()}
          onClick={() => void handleSave()}
        >
          {busy ? "saving…" : "Save & Continue"}
        </button>
        <button
          style={{ ...btnStyle(busy), background: busy ? "#333" : "#7a3a2a" }}
          disabled={busy}
          onClick={onDiscardAndContinue}
        >
          Discard & Continue
        </button>
        <button
          style={{ ...btnStyle(false), background: "#444" }}
          onClick={onCancel}
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

// --- Join URL modal ---
interface JoinModalProps {
  onSuccess: () => void;
  onCancel: () => void;
}

function JoinModal({ onSuccess, onCancel }: JoinModalProps) {
  const [url, setUrl] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    const trimmed = url.trim();
    if (!trimmed) return;
    setBusy(true);
    setError("");
    try {
      await invoke("join_session", { url: trimmed });
      onSuccess();
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  };

  return (
    <div style={overlayStyle}>
      <form style={cardStyle} onSubmit={(e) => void handleSubmit(e)}>
        <div style={{ fontSize: 16, fontWeight: "bold", color: "#eee" }}>
          Join a session
        </div>
        <div style={{ fontSize: 13, color: "#aaa" }}>
          Paste the session URL shared by the host.
        </div>
        <input
          style={inputStyle}
          autoFocus
          placeholder="wss://example.com/ws?room=…"
          value={url}
          onChange={(e) => {
            setUrl(e.target.value);
            setError("");
          }}
        />
        <div style={errorStyle}>{error}</div>
        <div style={{ display: "flex", gap: 8 }}>
          <button
            type="button"
            style={{ ...btnStyle(busy), background: busy ? "#333" : "#444" }}
            disabled={busy}
            onClick={onCancel}
          >
            Cancel
          </button>
          <button
            type="submit"
            style={btnStyle(busy || !url.trim())}
            disabled={busy || !url.trim()}
          >
            {busy ? "connecting…" : "Connect"}
          </button>
        </div>
      </form>
    </div>
  );
}

interface AppContentProps {
  username: string;
}

function installPlainTextPasteHandler(
  editorInstance: editor.IStandaloneCodeEditor,
) {
  const domNode = editorInstance.getDomNode();
  if (!domNode) return;

  const handlePaste = (event: ClipboardEvent) => {
    event.preventDefault();
    event.stopPropagation();

    const text = event.clipboardData?.getData("text/plain") ?? "";

    if (text) {
      editorInstance.focus();
      editorInstance.trigger("plain-text-paste", "type", { text });
      return;
    }

    void navigator.clipboard.readText().then((clipText) => {
      if (!clipText) return;
      editorInstance.focus();
      editorInstance.trigger("plain-text-paste", "type", { text: clipText });
    });
  };

  domNode.addEventListener("paste", handlePaste, { capture: true });
  editorInstance.onDidDispose(() => {
    domNode.removeEventListener("paste", handlePaste, { capture: true });
  });
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
  const lastAppliedSeqRef = useRef(0);
  const opChainRef = useRef<Promise<unknown>>(Promise.resolve());

  const enqueueOp = useMemo(() => createEnqueueOp(opChainRef), []);
  const { sendInsert, sendDelete } = useMemo(
    () => createIpcSenders(enqueueOp),
    [enqueueOp],
  );

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

  // --- session links ---
  const [lanUrl, setLanUrl] = useState<string | null>(null);
  const [publicUrl, setPublicUrl] = useState<string | null>(null);
  const [sessionStatus, setSessionStatus] = useState<string>("loading");
  const sessionStatusRef = useRef(sessionStatus);
  sessionStatusRef.current = sessionStatus;
  const [copyStatus, setCopyStatus] = useState<string | null>(null);
  const [sessionEndedBanner, setSessionEndedBanner] = useState(false);
  const [processesRunning, setProcessesRunning] = useState({
    gateway: "Disabled" as "Enabled" | "Disabled",
    tunnel: "Disabled" as "Enabled" | "Disabled",
  });

  // --- idle session actions ---
  const [sessionBusy, setSessionBusy] = useState(false);
  const [showJoinModal, setShowJoinModal] = useState(false);
  const [showSavePrompt, setShowSavePrompt] = useState(false);
  const [pendingAction, setPendingAction] = useState<"host" | "join" | null>(
    null,
  );

  const resetDocAndEditor = useCallback(async () => {
    await invoke("reset_document");
    isApplyingRemote.current = true;
    editorRef.current?.setValue("");
    isApplyingRemote.current = false;
  }, []);

  const performAction = useCallback(
    async (action: "host" | "join") => {
      await resetDocAndEditor();
      if (action === "host") {
        setSessionBusy(true);
        try {
          await invoke("host_session");
        } catch (err) {
          setSessionStatus("error: " + String(err));
        } finally {
          setSessionBusy(false);
        }
      } else {
        setShowJoinModal(true);
      }
    },
    [resetDocAndEditor],
  );

  const handleHostClick = useCallback(async () => {
    const content = editorRef.current?.getValue() ?? "";
    if (content.length > 0) {
      setPendingAction("host");
      setShowSavePrompt(true);
    } else {
      await performAction("host");
    }
  }, [performAction]);

  const handleJoinClick = useCallback(async () => {
    const content = editorRef.current?.getValue() ?? "";
    if (content.length > 0) {
      setPendingAction("join");
      setShowSavePrompt(true);
    } else {
      await performAction("join");
    }
  }, [performAction]);

  const handleSaveAndContinue = useCallback(
    async (name: string) => {
      await invoke("save_document", { name });
      setShowSavePrompt(false);
      const action = pendingAction!;
      setPendingAction(null);
      await performAction(action);
    },
    [pendingAction, performAction],
  );

  const handleDiscardAndContinue = useCallback(async () => {
    setShowSavePrompt(false);
    const action = pendingAction!;
    setPendingAction(null);
    await performAction(action);
  }, [pendingAction, performAction]);

  const handleJoinSuccess = useCallback(async () => {
    setShowJoinModal(false);
    const info = await invoke<{
      status: string;
      public_url: string | null;
      public_room_url: string | null;
    }>("get_session_info");
    setSessionStatus(info.status);
    if (info.public_room_url ?? info.public_url) {
      setPublicUrl(info.public_room_url ?? info.public_url);
    }
  }, []);

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
      local_room_url: string | null;
      public_room_url: string | null;
      room_id: string | null;
    }>("get_session_info").then((info) => {
      setSessionStatus(info.status);

      if (info.lan_url && !info.room_id) {
        throw new Error(
          "get_session_info: lan_url present but room_id is null",
        );
      }

      const lanRoomUrl =
        info.lan_url && info.room_id
          ? `${info.lan_url}?room=${info.room_id}`
          : null;

      if (lanRoomUrl) setLanUrl(lanRoomUrl);
      if (info.public_room_url ?? info.public_url) {
        setPublicUrl(info.public_room_url ?? info.public_url);
      }
    });

    invoke<{ gateway: "Enabled" | "Disabled"; tunnel: "Enabled" | "Disabled" }>(
      "get_process_status",
    ).then((s) => setProcessesRunning(s));

    const unlisten: (() => void)[] = [];
    (async () => {
      unlisten.push(
        await listen<{
          lan_url: string | null;
          public_url: string | null;
          local_room_url: string;
          public_room_url: string | null;
          port: number;
          room_id: string;
        }>("session://session-ready", (e) => {
          setSessionStatus("host");
          setLanUrl(
            e.payload.lan_url
              ? `${e.payload.lan_url}?room=${e.payload.room_id}`
              : e.payload.local_room_url,
          );
          setPublicUrl(e.payload.public_room_url);
          setProcessesRunning({
            gateway: "Enabled",
            tunnel: e.payload.public_url !== null ? "Enabled" : "Disabled",
          });
        }),
      );
      unlisten.push(
        await listen<{ message: string }>("session://session-error", (e) => {
          setSessionStatus("error: " + e.payload.message);
          setProcessesRunning({ gateway: "Disabled", tunnel: "Disabled" });
        }),
      );
      unlisten.push(
        await listen("session://session-ended", () => {
          // Guests only: the gateway may echo end-session to the host too.
          if (sessionStatusRef.current !== "guest") return;
          void invoke("leave_session").then(() => {
            setSessionStatus("idle");
            setPublicUrl(null);
            setSessionEndedBanner(true);
            window.setTimeout(() => setSessionEndedBanner(false), 5000);
          });
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
    lastAppliedSeqRef,
  });

  useSnapshotListener({
    editorRef,
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
    installPlainTextPasteHandler(editorInstance);
    setStatus("editor ready");
    setStatusReady(true);

    editorInstance.onDidChangeModelContent(
      (event: editor.IModelContentChangedEvent) => {
        // Skip changes that we ourselves applied from a remote peer.
        if (isApplyingRemote.current) return;

        const baseSeq = lastAppliedSeqRef.current;
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
                await sendDelete(offset, deleteLen, baseSeq);
              }
              if (insertText.length > 0) {
                await sendInsert(offset, insertText, baseSeq);
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
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 10,
            flexWrap: "wrap",
          }}
        >
          <span>
            <span style={{ color: "#aaa" }}>Session: </span>
            <span
              style={{
                color: sessionStatus.startsWith("error") ? "#f55" : "#0f0",
              }}
            >
              {sessionStatus}
            </span>
          </span>
          {(processesRunning.gateway === "Enabled" ||
            processesRunning.tunnel === "Enabled") && (
            <span style={{ display: "flex", gap: 6 }}>
              {processesRunning.gateway === "Enabled" && (
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 5,
                    padding: "2px 8px",
                    borderRadius: 12,
                    background: "#0d2a1a",
                    border: "1px solid #2ecc71",
                    color: "#2ecc71",
                    fontSize: 11,
                    fontWeight: 600,
                    letterSpacing: "0.04em",
                  }}
                >
                  <span
                    style={{
                      width: 6,
                      height: 6,
                      borderRadius: "50%",
                      background: "#2ecc71",
                      boxShadow: "0 0 5px #2ecc71",
                      display: "inline-block",
                    }}
                  />
                  gateway
                </span>
              )}
              {processesRunning.tunnel === "Enabled" && (
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 5,
                    padding: "2px 8px",
                    borderRadius: 12,
                    background: "#0d1e2a",
                    border: "1px solid #3498db",
                    color: "#3498db",
                    fontSize: 11,
                    fontWeight: 600,
                    letterSpacing: "0.04em",
                  }}
                >
                  <span
                    style={{
                      width: 6,
                      height: 6,
                      borderRadius: "50%",
                      background: "#3498db",
                      boxShadow: "0 0 5px #3498db",
                      display: "inline-block",
                    }}
                  />
                  tunnel
                </span>
              )}
            </span>
          )}
        </div>
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
        {sessionStatus === "starting" && (
          <span style={{ color: "#888", marginLeft: 8 }}>
            starting session...
          </span>
        )}
        {sessionStatus === "idle" &&
          (processesRunning.gateway === "Enabled" ||
            processesRunning.tunnel === "Enabled") && (
            <button
              onClick={() => {
                void invoke("kill_host_processes").then(() =>
                  setProcessesRunning({
                    gateway: "Disabled",
                    tunnel: "Disabled",
                  }),
                );
              }}
              style={{
                marginTop: 8,
                fontSize: 12,
                fontFamily: "monospace",
                fontWeight: 600,
                padding: "5px 16px",
                borderRadius: 5,
                border: "none",
                background: "linear-gradient(135deg, #e67e22, #d35400)",
                color: "#fff",
                cursor: "pointer",
                boxShadow: "0 2px 8px rgba(230,126,34,0.35)",
                letterSpacing: "0.03em",
              }}
            >
              ⏹ Kill Processes
            </button>
          )}
        {sessionStatus === "idle" && (
          <div style={{ display: "flex", gap: 10, marginTop: 8 }}>
            <button
              onClick={() => void handleHostClick()}
              disabled={sessionBusy}
              style={{
                fontSize: 12,
                fontFamily: "monospace",
                fontWeight: 600,
                padding: "5px 16px",
                borderRadius: 5,
                border: "none",
                background: sessionBusy
                  ? "#2a4a3a"
                  : "linear-gradient(135deg, #2ecc71, #27ae60)",
                color: sessionBusy ? "#6a9a7a" : "#fff",
                cursor: sessionBusy ? "not-allowed" : "pointer",
                boxShadow: sessionBusy
                  ? "none"
                  : "0 2px 8px rgba(46,204,113,0.3)",
                transition: "all 0.15s",
                letterSpacing: "0.03em",
              }}
            >
              {sessionBusy ? "⏳ Starting…" : "⚡ Host Session"}
            </button>
            {!sessionBusy && (
              <button
                onClick={() => void handleJoinClick()}
                style={{
                  fontSize: 12,
                  fontFamily: "monospace",
                  fontWeight: 600,
                  padding: "5px 16px",
                  borderRadius: 5,
                  border: "none",
                  background: "linear-gradient(135deg, #3498db, #2980b9)",
                  color: "#fff",
                  cursor: "pointer",
                  boxShadow: "0 2px 8px rgba(52,152,219,0.3)",
                  transition: "all 0.15s",
                  letterSpacing: "0.03em",
                }}
              >
                🔗 Join Session
              </button>
            )}
          </div>
        )}
        {sessionEndedBanner && (
          <div
            style={{
              marginTop: 6,
              padding: "5px 12px",
              borderRadius: 5,
              background: "#2a1a0a",
              border: "1px solid #e67e22",
              color: "#e67e22",
              fontSize: 12,
              fontFamily: "monospace",
            }}
          >
            ⚠ The host ended the session. Your document is preserved.
          </div>
        )}
        {sessionStatus === "host" && (
          <button
            onClick={() => {
              void invoke("end_session").then(() => {
                setSessionStatus("idle");
                setLanUrl(null);
                setPublicUrl(null);
              });
            }}
            style={{
              marginTop: 8,
              fontSize: 12,
              fontFamily: "monospace",
              fontWeight: 600,
              padding: "5px 16px",
              borderRadius: 5,
              border: "none",
              background: "linear-gradient(135deg, #e74c3c, #c0392b)",
              color: "#fff",
              cursor: "pointer",
              boxShadow: "0 2px 8px rgba(231,76,60,0.3)",
              letterSpacing: "0.03em",
            }}
          >
            ✕ End Session
          </button>
        )}
        {sessionStatus === "guest" && (
          <button
            onClick={() => {
              void invoke("leave_session").then(() => {
                setSessionStatus("idle");
                setPublicUrl(null);
              });
            }}
            style={{
              marginTop: 8,
              fontSize: 12,
              fontFamily: "monospace",
              fontWeight: 600,
              padding: "5px 16px",
              borderRadius: 5,
              border: "none",
              background: "linear-gradient(135deg, #e74c3c, #c0392b)",
              color: "#fff",
              cursor: "pointer",
              boxShadow: "0 2px 8px rgba(231,76,60,0.3)",
              letterSpacing: "0.03em",
            }}
          >
            ✕ Leave Session
          </button>
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
      {showSavePrompt && (
        <SaveBeforeSessionModal
          onSaveAndContinue={handleSaveAndContinue}
          onDiscardAndContinue={() => void handleDiscardAndContinue()}
          onCancel={() => {
            setShowSavePrompt(false);
            setPendingAction(null);
          }}
        />
      )}
      {showJoinModal && (
        <JoinModal
          onSuccess={() => void handleJoinSuccess()}
          onCancel={() => setShowJoinModal(false)}
        />
      )}
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
