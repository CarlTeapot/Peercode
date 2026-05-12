import { useState, useCallback, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  overlayStyle,
  cardStyle,
  inputStyle,
  btnStyle,
  errorStyle,
} from "./usernameSetup";

interface SessionSetupModalProps {
  onDone: () => void;
}

type SessionView = "choice" | "join";

export function SessionSetupModal({ onDone }: SessionSetupModalProps) {
  const [view, setView] = useState<SessionView>("choice");
  const [joinUrl, setJoinUrl] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const handleHost = useCallback(async () => {
    setBusy(true);
    setError("");
    try {
      await invoke("host_session");
      onDone();
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  }, [onDone]);

  const handleJoin = useCallback(
    async (e: FormEvent) => {
      e.preventDefault();
      const url = joinUrl.trim();
      if (!url) return;
      setBusy(true);
      setError("");
      try {
        await invoke("join_session", { url });
        onDone();
      } catch (err) {
        setError(String(err));
        setBusy(false);
      }
    },
    [joinUrl, onDone],
  );

  return (
    <div style={overlayStyle}>
      <div style={cardStyle}>
        <div style={{ fontSize: 16, fontWeight: "bold", color: "#eee" }}>
          Start a session
        </div>

        {view === "choice" && (
          <>
            <div style={{ fontSize: 13, color: "#aaa" }}>
              Host a new collaborative session or join an existing one.
            </div>
            <button style={btnStyle(busy)} disabled={busy} onClick={handleHost}>
              {busy ? "starting…" : "Host a session"}
            </button>
            <button
              style={{
                ...btnStyle(busy),
                background: busy ? "#333" : "#2e5a9c",
              }}
              disabled={busy}
              onClick={() => {
                setError("");
                setView("join");
              }}
            >
              Join a session
            </button>
          </>
        )}

        {view === "join" && (
          <form
            onSubmit={handleJoin}
            style={{ display: "flex", flexDirection: "column", gap: 16 }}
          >
            <div style={{ fontSize: 13, color: "#aaa" }}>
              Paste the session URL shared by the host.
            </div>
            <input
              style={inputStyle}
              autoFocus
              placeholder="wss://example.com/ws?room=…"
              value={joinUrl}
              onChange={(e) => {
                setJoinUrl(e.target.value);
                setError("");
              }}
            />
            <div style={{ display: "flex", gap: 8 }}>
              <button
                type="button"
                style={{
                  ...btnStyle(busy),
                  background: busy ? "#333" : "#444",
                }}
                disabled={busy}
                onClick={() => {
                  setView("choice");
                  setError("");
                }}
              >
                Back
              </button>
              <button
                type="submit"
                style={btnStyle(busy || !joinUrl.trim())}
                disabled={busy || !joinUrl.trim()}
              >
                {busy ? "connecting…" : "Connect"}
              </button>
            </div>
          </form>
        )}

        <div style={errorStyle}>{error}</div>
      </div>
    </div>
  );
}
