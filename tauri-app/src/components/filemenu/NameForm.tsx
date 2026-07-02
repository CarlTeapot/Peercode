interface NameFormProps {
  title: string;
  subtitle?: string;
  submitLabel: string;
  busyLabel: string;
  busy: boolean;
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  onBack: () => void;
}

export function NameForm({
  title,
  subtitle,
  submitLabel,
  busyLabel,
  busy,
  value,
  onChange,
  onSubmit,
  onBack,
}: NameFormProps) {
  return (
    <div className="file-dropdown-form">
      <div className="file-dropdown-title">{title}</div>
      {subtitle && <div className="file-dropdown-subtitle">{subtitle}</div>}
      <div className="file-dropdown-input-row">
        <input
          className="file-dropdown-input"
          autoFocus
          placeholder="Document name"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") onSubmit();
            if (e.key === "Escape") onBack();
          }}
        />
        <span className="file-dropdown-ext">.pcdoc</span>
      </div>
      <div className="file-dropdown-actions">
        <button className="file-dropdown-btn secondary" onClick={onBack}>
          Back
        </button>
        <button
          className="file-dropdown-btn primary"
          onClick={onSubmit}
          disabled={busy || !value.trim()}
        >
          {busy ? busyLabel : submitLabel}
        </button>
      </div>
    </div>
  );
}
