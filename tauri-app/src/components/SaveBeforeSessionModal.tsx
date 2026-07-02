import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SaveBeforeSessionModalProps {
  onSaveAndContinue: (name: string) => Promise<void>;
  onDiscardAndContinue: () => void;
  onCancel: () => void;
}

/**
 * Shown before joining a session when the editor has content: joining
 * replaces the local document with the host's, so offer to save it first.
 */
export function SaveBeforeSessionModal({
  onSaveAndContinue,
  onDiscardAndContinue,
  onCancel,
}: SaveBeforeSessionModalProps) {
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void invoke<string | null>("get_current_document_name").then((n) => {
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
    <div className="modal-overlay">
      <div className="modal-card">
        <div className="modal-title">Unsaved changes</div>
        <div className="modal-text">
          Your document has content. Save it before starting a new session?
        </div>
        <input
          className="modal-input"
          autoFocus
          placeholder="Document name"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setError("");
          }}
        />
        <div className="modal-error">{error}</div>
        <button
          className="modal-btn"
          disabled={busy || !name.trim()}
          onClick={() => void handleSave()}
        >
          {busy ? "saving…" : "Save & Continue"}
        </button>
        <button
          className="modal-btn danger"
          disabled={busy}
          onClick={onDiscardAndContinue}
        >
          Discard & Continue
        </button>
        <button className="modal-btn neutral" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </div>
  );
}
