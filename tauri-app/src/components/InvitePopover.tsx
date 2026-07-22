import { useCallback, useRef, useState } from "react";
import { useDismiss } from "./KebabMenu";
import "./TopbarMenus.css";

interface InvitePopoverProps {
  publicUrl: string | null;
  lanUrl: string | null;
}

/** Host-only Invite button with a copy-URL popover. */
export function InvitePopover({ publicUrl, lanUrl }: InvitePopoverProps) {
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState<string | null>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  useDismiss(wrapRef, open, () => setOpen(false));

  const copyUrl = useCallback(async (label: string, url: string) => {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(label);
    } catch {
      setCopied(null);
    }
    window.setTimeout(() => setCopied(null), 1500);
  }, []);

  if (!publicUrl && !lanUrl) return null;

  return (
    <div className="topbar-menu-wrap" ref={wrapRef}>
      <button
        className="btn-primary"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((p) => !p)}
      >
        Invite
      </button>
      {open && (
        <div className="topbar-menu" role="menu">
          {publicUrl && (
            <button
              role="menuitem"
              className="topbar-menu-item"
              onClick={() => void copyUrl("public", publicUrl)}
            >
              {copied === "public" ? "copied ✓" : "Copy Public URL"}
            </button>
          )}
          {lanUrl && (
            <button
              role="menuitem"
              className="topbar-menu-item"
              onClick={() => void copyUrl("lan", lanUrl)}
            >
              {copied === "lan" ? "copied ✓" : "Copy Local URL"}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
