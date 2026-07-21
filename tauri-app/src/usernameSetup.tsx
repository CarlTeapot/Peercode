import { useState, useEffect, useCallback, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import type React from "react";

const MAX_LEN = 32;

function sanitize(raw: string): string {
  return [...raw]
    .filter((c) => c.charCodeAt(0) >= 0x20)
    .join("")
    .trim()
    .slice(0, MAX_LEN);
}

interface UsernameGateProps {
  children: (username: string) => React.ReactNode;
}

export function UsernameGate({ children }: UsernameGateProps) {
  const [username, setUsername] = useState<string | null>(null);

  useEffect(() => {
    invoke<{ username: string | null }>("get_identity")
      .then((id) => setUsername(id.username ?? ""))
      .catch(() => setUsername(""));
  }, []);

  if (username === null) return null;
  if (username === "") return <FirstRunModal onDone={setUsername} />;
  return <>{children(username)}</>;
}

interface FirstRunModalProps {
  onDone: (username: string) => void;
}

function FirstRunModal({ onDone }: FirstRunModalProps) {
  const [value, setValue] = useState("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);

  const clean = sanitize(value);
  const canSubmit = clean.length > 0 && !saving;

  const handleSubmit = useCallback(
    async (e: FormEvent) => {
      e.preventDefault();
      if (!canSubmit) return;
      setSaving(true);
      setError("");
      try {
        await invoke("set_username", { username: clean });
        onDone(clean);
      } catch (err) {
        setError(String(err));
        setSaving(false);
      }
    },
    [clean, canSubmit, onDone],
  );

  return (
    <div className="modal-overlay">
      <form className="modal-card" onSubmit={handleSubmit}>
        <div className="modal-title">welcome to peercode</div>
        <div className="modal-text">
          Choose a display name. Others in your session will see it.
        </div>
        <input
          className="modal-input"
          autoFocus
          placeholder="Your name"
          maxLength={MAX_LEN}
          value={value}
          onChange={(e) => {
            setValue(e.target.value);
            setError("");
          }}
        />
        <div className="modal-error">{error}</div>
        <button type="submit" className="modal-btn" disabled={!canSubmit}>
          {saving ? "saving…" : "Continue"}
        </button>
      </form>
    </div>
  );
}
