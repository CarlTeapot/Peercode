import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CurrentFileInfo } from "./filemenu/format";
import { saveBuffer } from "./filemenu/saveFlow";

interface SaveBeforeSessionModalProps {
  onSaved: () => void;
  onDiscardAndContinue: () => void;
  onCancel: () => void;
}

/**
 * Shown before joining a session when the editor has content: joining
 * replaces the local document with the host's, so offer to save it first.
 */
export function SaveBeforeSessionModal({
  onSaved,
  onDiscardAndContinue,
  onCancel,
}: SaveBeforeSessionModalProps) {
  const [current, setCurrent] = useState<CurrentFileInfo | null>(null);
  const [docsDir, setDocsDir] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void invoke<CurrentFileInfo | null>("get_current_file").then(setCurrent);
    void invoke<string>("get_documents_dir")
      .then(setDocsDir)
      .catch(() => {});
  }, []);

  const handleSave = async () => {
    setBusy(true);
    setError("");
    try {
      if (await saveBuffer(current, docsDir)) {
        onSaved();
      } else {
        setBusy(false); // dialog cancelled — stay on the prompt
      }
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
          {current
            ? `Save "${current.name}" before joining?`
            : "Your buffer isn't saved anywhere yet. Save it before joining?"}
        </div>
        <div className="modal-error">{error}</div>
        <button
          className="modal-btn"
          disabled={busy}
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
