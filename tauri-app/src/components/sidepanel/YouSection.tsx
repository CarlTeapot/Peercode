import { useState } from "react";
import { useNameForm, USERNAME_MAX_LEN } from "../../usernameSetup";

interface YouSectionProps {
  username: string;
  onUsernameChange: (name: string) => void;
}

/** You section of the side panel: change the display name. */
export function YouSection({ username, onUsernameChange }: YouSectionProps) {
  const [saved, setSaved] = useState(false);
  const form = useNameForm(username, (name) => {
    onUsernameChange(name);
    setSaved(true);
    window.setTimeout(() => setSaved(false), 1500);
  });

  return (
    <form className="panel-section" onSubmit={(e) => void form.handleSubmit(e)}>
      <p className="panel-hint">
        Your display name. Peers already in the room keep seeing your old name;
        the new one applies when you next join or host.
      </p>
      <input
        className="modal-input"
        placeholder="Your name"
        maxLength={USERNAME_MAX_LEN}
        value={form.value}
        onChange={(e) => {
          form.setValue(e.target.value);
          form.setError("");
        }}
      />
      {form.error && <div className="modal-error">{form.error}</div>}
      <button type="submit" className="btn" disabled={!form.canSubmit}>
        {form.saving ? "saving…" : saved ? "saved ✓" : "Save name"}
      </button>
    </form>
  );
}
