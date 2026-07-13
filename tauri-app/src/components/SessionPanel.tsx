import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  useSessionEvents,
  type SessionNotice,
} from "../hooks/useSessionEvents";
import { useMetrics } from "../hooks/useMetrics";
import { MetricsPopup } from "./MetricsPopup";
import { SaveBeforeSessionModal } from "./SaveBeforeSessionModal";
import { JoinModal } from "./JoinModal";

const SESSION_NOTICE_MESSAGE: Record<SessionNotice, string> = {
  ended: "⚠ The host ended the session. Your document is preserved.",
  disconnected: "⚠ Connection lost. Your document is preserved locally.",
};

interface SessionPanelProps {
  /** Current editor text; used to warn before joining discards it. */
  getEditorContent: () => string;
  /** Clear the CRDT document and the editor (before joining a session). */
  resetDocAndEditor: () => Promise<void>;
}

/**
 * Session status bar: share URLs, sidecar health chips, host/join/end
 * controls and the related modals.
 *
 * Hosting keeps the currently open document — the host seeds joiners with
 * its snapshot, so an opened .pcdoc becomes the session content. Joining
 * replaces the local document, hence the save prompt.
 */
export function SessionPanel({
  getEditorContent,
  resetDocAndEditor,
}: SessionPanelProps) {
  const {
    sessionStatus,
    setSessionStatus,
    lanUrl,
    publicUrl,
    setPublicUrl,
    processesRunning,
    setProcessesRunning,
    sessionNotice,
    sessionBusy,
    setSessionBusy,
    applyIdleSessionState,
  } = useSessionEvents();

  const { gatewayFields, gatewayUnavailable, tunnelFields, tunnelUnavailable } =
    useMetrics(sessionStatus, processesRunning);

  const [copyStatus, setCopyStatus] = useState<string | null>(null);
  const [showJoinModal, setShowJoinModal] = useState(false);
  const [showSavePrompt, setShowSavePrompt] = useState(false);
  const [showGatewayMetrics, setShowGatewayMetrics] = useState(false);
  const [showTunnelMetrics, setShowTunnelMetrics] = useState(false);

  const isHost = sessionStatus === "host";

  useEffect(() => {
    if (!isHost) {
      setShowGatewayMetrics(false);
      setShowTunnelMetrics(false);
    }
  }, [isHost]);

  const copyUrl = useCallback(async (label: string, url: string) => {
    try {
      await navigator.clipboard.writeText(url);
      setCopyStatus(`${label} URL copied`);
    } catch {
      setCopyStatus(`Failed to copy ${label} URL`);
    }
    window.setTimeout(() => setCopyStatus(null), 1500);
  }, []);

  const handleHost = useCallback(async () => {
    setSessionBusy(true);
    try {
      await invoke("host_session");
    } catch (err) {
      setSessionStatus("error: " + String(err));
    } finally {
      setSessionBusy(false);
    }
  }, [setSessionBusy, setSessionStatus]);

  const startJoin = useCallback(async () => {
    await resetDocAndEditor();
    setShowJoinModal(true);
  }, [resetDocAndEditor]);

  const handleJoinClick = useCallback(() => {
    if (getEditorContent().length > 0) {
      setShowSavePrompt(true);
    } else {
      void startJoin();
    }
  }, [getEditorContent, startJoin]);

  const handleSaveAndContinue = useCallback(async () => {
    setShowSavePrompt(false);
    await startJoin();
  }, [startJoin]);

  const handleDiscardAndContinue = useCallback(() => {
    setShowSavePrompt(false);
    void startJoin();
  }, [startJoin]);

  const handleJoinSuccess = useCallback(async () => {
    setShowJoinModal(false);
    const info = await invoke<{
      status: string;
      public_url: string | null;
      public_room_url: string | null;
    }>("get_session_info");
    setSessionStatus(info.status);
    if (info.public_room_url ?? info.public_url) {
      setPublicUrl(info.public_room_url ?? info.public_url);
    }
  }, [setPublicUrl, setSessionStatus]);

  const handleEndSession = useCallback(() => {
    if (sessionBusy) return;
    setSessionBusy(true);
    void invoke("end_session")
      .then(() => applyIdleSessionState())
      .finally(() => setSessionBusy(false));
  }, [applyIdleSessionState, sessionBusy, setSessionBusy]);

  const handleLeaveSession = useCallback(() => {
    void invoke("leave_session").then(() => applyIdleSessionState());
  }, [applyIdleSessionState]);

  const handleKillProcesses = useCallback(() => {
    void invoke("kill_host_processes").then(() =>
      setProcessesRunning({ gateway: "Disabled", tunnel: "Disabled" }),
    );
  }, [setProcessesRunning]);

  const anyProcessRunning =
    processesRunning.gateway === "Enabled" ||
    processesRunning.tunnel === "Enabled";

  return (
    <div className="session-panel">
      <div className="session-panel-row">
        <span>
          <span className="session-label">Session: </span>
          <span
            className={
              "session-status-value" +
              (sessionStatus.startsWith("error") ? " error" : "")
            }
          >
            {sessionStatus}
          </span>
        </span>
        {anyProcessRunning && (
          <span className="session-chips">
            {processesRunning.gateway === "Enabled" && (
              <button
                type="button"
                className={
                  "session-chip gateway" + (isHost ? " clickable" : "")
                }
                onClick={() => {
                  if (isHost) setShowGatewayMetrics(true);
                }}
                title={isHost ? "Show PeerCode gateway health" : undefined}
              >
                <span className="session-chip-dot" />
                gateway
              </button>
            )}
            {processesRunning.tunnel === "Enabled" && (
              <button
                type="button"
                className={"session-chip tunnel" + (isHost ? " clickable" : "")}
                onClick={() => {
                  if (isHost) setShowTunnelMetrics(true);
                }}
                title={isHost ? "Show Cloudflare tunnel health" : undefined}
              >
                <span className="session-chip-dot" />
                tunnel
              </button>
            )}
          </span>
        )}
      </div>
      {lanUrl && (
        <div className="session-url-row">
          <span className="session-label">LAN: </span>
          <span className="session-url">{lanUrl}</span>
          <button
            className="session-copy-btn"
            onClick={() => void copyUrl("LAN", lanUrl)}
          >
            Copy
          </button>
        </div>
      )}
      {publicUrl && (
        <div className="session-url-row">
          <span className="session-label">Public: </span>
          <span className="session-url">{publicUrl}</span>
          <button
            className="session-copy-btn"
            onClick={() => void copyUrl("Public", publicUrl)}
          >
            Copy
          </button>
        </div>
      )}
      {copyStatus && <div className="session-copy-status">{copyStatus}</div>}
      {sessionStatus === "starting" && (
        <span className="session-starting-note">starting session...</span>
      )}
      {sessionStatus === "idle" && anyProcessRunning && (
        <button className="session-btn kill" onClick={handleKillProcesses}>
          ⏹ Kill Processes
        </button>
      )}
      {sessionStatus === "idle" && (
        <div className="session-actions">
          <button
            className="session-btn host"
            onClick={() => void handleHost()}
            disabled={sessionBusy}
          >
            {sessionBusy ? "⏳ Starting…" : "⚡ Host Session"}
          </button>
          {!sessionBusy && (
            <button className="session-btn join" onClick={handleJoinClick}>
              🔗 Join Session
            </button>
          )}
        </div>
      )}
      {sessionNotice && (
        <div className={`session-notice ${sessionNotice}`}>
          {SESSION_NOTICE_MESSAGE[sessionNotice]}
        </div>
      )}
      {isHost && (
        <button
          className="session-btn end"
          disabled={sessionBusy}
          onClick={handleEndSession}
        >
          {sessionBusy ? "Ending…" : "✕ End Session"}
        </button>
      )}
      {sessionStatus === "guest" && (
        <button className="session-btn end" onClick={handleLeaveSession}>
          ✕ Leave Session
        </button>
      )}
      {showGatewayMetrics && isHost && (
        <MetricsPopup
          title="PeerCode gateway"
          subtitle="Live relay and room health"
          unavailable={gatewayUnavailable}
          fields={gatewayFields}
          onClose={() => setShowGatewayMetrics(false)}
        />
      )}
      {showTunnelMetrics && isHost && (
        <MetricsPopup
          title="Cloudflare tunnel"
          subtitle="Live host connection health"
          unavailable={tunnelUnavailable}
          fields={tunnelFields}
          note="Global peer count is not exposed by cloudflared. It must come from the PeerCode gateway, not tunnel metrics."
          onClose={() => setShowTunnelMetrics(false)}
        />
      )}
      {showSavePrompt && (
        <SaveBeforeSessionModal
          onSaved={() => void handleSaveAndContinue()}
          onDiscardAndContinue={handleDiscardAndContinue}
          onCancel={() => setShowSavePrompt(false)}
        />
      )}
      {showJoinModal && (
        <JoinModal
          onSuccess={() => void handleJoinSuccess()}
          onCancel={() => setShowJoinModal(false)}
        />
      )}
    </div>
  );
}
