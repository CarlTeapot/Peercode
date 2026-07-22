import { useEffect, useRef, useState, type RefObject } from "react";
import "./TopbarMenus.css";

/** Close an anchored menu on Escape or a click outside `ref`. */
export function useDismiss(
  ref: RefObject<HTMLElement | null>,
  open: boolean,
  onClose: () => void,
) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    window.addEventListener("keydown", onKey);
    document.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey);
      document.removeEventListener("mousedown", onDown);
    };
  }, [ref, open, onClose]);
}

export interface KebabItem {
  label: string;
  onClick: () => void;
  tone?: "danger";
  active?: boolean;
}

interface KebabMenuProps {
  items: KebabItem[];
}

/** ⋮ overflow menu for rarely-used actions. */
export function KebabMenu({ items }: KebabMenuProps) {
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);
  useDismiss(wrapRef, open, () => setOpen(false));

  if (items.length === 0) return null;

  return (
    <div className="topbar-menu-wrap" ref={wrapRef}>
      <button
        className="kebab-btn"
        title="More"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((p) => !p)}
      >
        ⋮
      </button>
      {open && (
        <div className="topbar-menu" role="menu">
          {items.map((item) => (
            <button
              key={item.label}
              role="menuitem"
              className={
                "topbar-menu-item" +
                (item.tone === "danger" ? " danger" : "") +
                (item.active ? " active" : "")
              }
              onClick={() => {
                item.onClick();
                setOpen(false);
              }}
            >
              {item.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
