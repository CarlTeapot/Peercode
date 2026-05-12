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

export const overlayStyle: React.CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "rgba(0,0,0,0.75)",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  zIndex: 9999,
};

export const cardStyle: React.CSSProperties = {
  background: "#1e1e2e",
  border: "1px solid #444",
  borderRadius: 8,
  padding: "28px 32px",
  width: 360,
  display: "flex",
  flexDirection: "column",
  gap: 16,
  color: "#ccc",
  fontFamily: "monospace",
};

export const inputStyle: React.CSSProperties = {
  background: "#12121f",
  border: "1px solid #555",
  borderRadius: 4,
  color: "#eee",
  fontFamily: "monospace",
  fontSize: 14,
  padding: "6px 10px",
  width: "100%",
  boxSizing: "border-box",
};

export const btnStyle = (disabled: boolean): React.CSSProperties => ({
  background: disabled ? "#333" : "#4a7fd4",
  border: "none",
  borderRadius: 4,
  color: disabled ? "#666" : "#fff",
  cursor: disabled ? "not-allowed" : "pointer",
  fontFamily: "monospace",
  fontSize: 14,
  padding: "7px 0",
  width: "100%",
});

export const errorStyle: React.CSSProperties = {
  color: "#f77",
  fontSize: 12,
  minHeight: 16,
};

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
    <div style={overlayStyle}>
      <form style={cardStyle} onSubmit={handleSubmit}>
        <div style={{ fontSize: 16, fontWeight: "bold", color: "#eee" }}>
          Welcome to PeerCode
        </div>
        <div style={{ fontSize: 13, color: "#aaa" }}>
          Choose a display name. Others in your session will see it.
        </div>
        <input
          style={inputStyle}
          autoFocus
          placeholder="Your name"
          maxLength={MAX_LEN}
          value={value}
          onChange={(e) => {
            setValue(e.target.value);
            setError("");
          }}
        />
        <div style={errorStyle}>{error}</div>
        <button
          type="submit"
          style={btnStyle(!canSubmit)}
          disabled={!canSubmit}
        >
          {saving ? "saving…" : "Continue"}
        </button>
      </form>
    </div>
  );
}
