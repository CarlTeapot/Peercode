import { useEffect } from "react";
import "./MetricsPopup.css";

export interface MetricsPopupField {
  name: string;
  value: string;
  tone?: "ok" | "warning";
}

interface MetricsPopupProps {
  title: string;
  subtitle: string;
  unavailable: boolean;
  fields: MetricsPopupField[];
  note?: string;
  onClose: () => void;
}

export function MetricsPopup({
  title,
  subtitle,
  unavailable,
  fields,
  note,
  onClose,
}: MetricsPopupProps) {
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose]);

  return (
    <div
      className="metrics-popup-overlay"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section
        className="metrics-popup"
        role="dialog"
        aria-modal="true"
        aria-labelledby="metrics-popup-title"
      >
        <div className="metrics-popup-heading">
          <div>
            <div id="metrics-popup-title" className="metrics-popup-title">
              {title}
            </div>
            <div className="metrics-popup-subtitle">{subtitle}</div>
          </div>
          <button
            type="button"
            className="metrics-popup-close"
            aria-label={`Close ${title} metrics`}
            onClick={onClose}
          >
            ✕
          </button>
        </div>
        {unavailable ? (
          <div className="metrics-popup-unavailable">
            Metrics server unavailable
          </div>
        ) : fields.length > 0 ? (
          <div className="metrics-popup-grid">
            {fields.map((field) => (
              <div key={field.name}>
                <span className="metrics-popup-label">{field.name}</span>
                <strong
                  className={
                    field.tone === "ok"
                      ? "metrics-popup-ok"
                      : field.tone === "warning"
                        ? "metrics-popup-warning"
                        : undefined
                  }
                >
                  {field.value}
                </strong>
              </div>
            ))}
          </div>
        ) : (
          <div className="metrics-popup-loading">Reading metrics...</div>
        )}
        {!unavailable && note && (
          <div className="metrics-popup-note">{note}</div>
        )}
      </section>
    </div>
  );
}
