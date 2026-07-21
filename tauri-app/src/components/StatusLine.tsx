import { useState, useCallback } from "react";
import "./StatusLine.css";

export interface CursorPos {
  line: number;
  col: number;
}

interface StatusLineProps {
  sessionStatus: string;
  roomId: string | null;
  /** Best shareable URL (public if tunneled, else LAN); click-to-copy. */
  shareUrl: string | null;
  peerCount: number;
  inSession: boolean;
  onPeersClick: () => void;
  fileName: string | null;
  dirty: boolean;
  hadCrlf: boolean;
  canWrite: boolean;
  statusReady: boolean;
  cursor: CursorPos;
  fontSize: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
}

/**
 * Neovim-style statusline: role segment, file + dirty marker and read-only
 * flag on the left; room/peers/connection/EOL/cursor segments on the right.
 */
export function StatusLine({
  sessionStatus,
  roomId,
  shareUrl,
  peerCount,
  inSession,
  onPeersClick,
  fileName,
  dirty,
  hadCrlf,
  canWrite,
  statusReady,
  cursor,
  fontSize,
  onZoomIn,
  onZoomOut,
}: StatusLineProps) {
  const [copied, setCopied] = useState(false);

  const role =
    sessionStatus === "host"
      ? "host"
      : sessionStatus === "guest"
        ? "guest"
        : "solo";

  const isError = sessionStatus.startsWith("error");
  const connClass = isError
    ? "err"
    : sessionStatus === "starting" || !statusReady
      ? "warn"
      : "ok";
  const connTitle = isError
    ? sessionStatus
    : sessionStatus === "starting"
      ? "starting session"
      : statusReady
        ? "ready"
        : "loading editor";

  const copyShareUrl = useCallback(async () => {
    if (!shareUrl) return;
    try {
      await navigator.clipboard.writeText(shareUrl);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  }, [shareUrl]);

  return (
    <div className="statusline">
      <span className={`sl-role ${role}`}>{role.toUpperCase()}</span>
      <span className="sl-seg sl-file">
        {fileName ?? "[untitled]"}
        {dirty && <span className="sl-dirty">●</span>}
      </span>
      {!canWrite && (
        <span className="sl-seg sl-ro" title="The host has made you read-only">
          RO
        </span>
      )}
      <div className="sl-right">
        {roomId &&
          (shareUrl ? (
            <button
              className="sl-seg"
              onClick={() => void copyShareUrl()}
              title={`Copy invite URL\n${shareUrl}`}
            >
              room {copied ? "· copied" : roomId}
            </button>
          ) : (
            <span className="sl-seg">room {roomId}</span>
          ))}
        {inSession && (
          <button
            className="sl-seg"
            onClick={onPeersClick}
            title="Show peers panel"
          >
            {peerCount} {peerCount === 1 ? "peer" : "peers"}
          </button>
        )}
        <span className="sl-seg" title={connTitle}>
          <span className={`sl-conn-dot ${connClass}`} />
        </span>
        <span className="sl-seg sl-zoom">
          <button
            className="sl-zoom-btn"
            onClick={onZoomOut}
            title="Decrease editor font size (Ctrl+-)"
          >
            −
          </button>
          <span className="sl-muted">{fontSize}px</span>
          <button
            className="sl-zoom-btn"
            onClick={onZoomIn}
            title="Increase editor font size (Ctrl+=)"
          >
            +
          </button>
        </span>
        <span className="sl-seg sl-muted">{hadCrlf ? "CRLF" : "LF"}</span>
        <span className="sl-seg sl-muted">
          Ln {cursor.line}, Col {cursor.col}
        </span>
      </div>
    </div>
  );
}
