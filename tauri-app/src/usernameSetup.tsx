/* eslint-disable react-refresh/only-export-components */
import { useState, useEffect, useCallback, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import type React from "react";

export const USERNAME_MAX_LEN = 32;

function sanitize(raw: string): string {
  return [...raw]
    .filter((c) => c.charCodeAt(0) >= 0x20)
    .join("")
    .trim()
    .slice(0, USERNAME_MAX_LEN);
}

interface UsernameGateProps {
  children: (
    username: string,
    setUsername: (name: string) => void,
  ) => React.ReactNode;
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
  return <>{children(username, setUsername)}</>;
}

/** Shared submit flow: persist the sanitized name, then report it up. */
export function useNameForm(
  initial: string,
  onDone: (username: string) => void,
) {
  const [value, setValue] = useState(initial);
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
        setSaving(false);
      } catch (err) {
        setError(String(err));
        setSaving(false);
      }
    },
    [clean, canSubmit, onDone],
  );

  return { value, setValue, error, setError, saving, canSubmit, handleSubmit };
}

interface FirstRunModalProps {
  onDone: (username: string) => void;
}

function FirstRunModal({ onDone }: FirstRunModalProps) {
  const form = useNameForm("", onDone);

  return (
    <div className="modal-overlay">
      <form className="modal-card" onSubmit={(e) => void form.handleSubmit(e)}>
        <div className="modal-title">welcome to peared</div>
        <div className="modal-text">
          Choose a display name. Others in your session will see it.
        </div>
        <input
          className="modal-input"
          autoFocus
          placeholder="Your name"
          maxLength={USERNAME_MAX_LEN}
          value={form.value}
          onChange={(e) => {
            form.setValue(e.target.value);
            form.setError("");
          }}
        />
        <div className="modal-error">{form.error}</div>
        <button type="submit" className="modal-btn" disabled={!form.canSubmit}>
          {form.saving ? "saving…" : "Continue"}
        </button>
      </form>
    </div>
  );
}
