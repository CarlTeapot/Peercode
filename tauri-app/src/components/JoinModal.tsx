import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface JoinModalProps {
  onSuccess: () => void;
  onCancel: () => void;
}

export function JoinModal({ onSuccess, onCancel }: JoinModalProps) {
  const [url, setUrl] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const handleSubmit = async () => {
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
    <div className="modal-overlay">
      <form
        className="modal-card"
        onSubmit={(e) => {
          e.preventDefault();
          void handleSubmit();
        }}
      >
        <div className="modal-title">Join a session</div>
        <div className="modal-text">
          Paste the session URL shared by the host.
        </div>
        <input
          className="modal-input"
          autoFocus
          placeholder="wss://example.com/ws?room=…"
          value={url}
          onChange={(e) => {
            setUrl(e.target.value);
            setError("");
          }}
        />
        <div className="modal-error">{error}</div>
        <div className="modal-btn-row">
          <button
            type="button"
            className="modal-btn neutral"
            disabled={busy}
            onClick={onCancel}
          >
            Cancel
          </button>
          <button
            type="submit"
            className="modal-btn"
            disabled={busy || !url.trim()}
          >
            {busy ? "connecting…" : "Connect"}
          </button>
        </div>
      </form>
    </div>
  );
}
