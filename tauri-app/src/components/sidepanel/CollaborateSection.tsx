import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { useSessionEvents } from "../../hooks/useSessionEvents";
import { useMetrics } from "../../hooks/useMetrics";
import { MetricsPopup } from "../MetricsPopup";
import { SaveBeforeSessionModal } from "../SaveBeforeSessionModal";
import { JoinModal } from "../JoinModal";

export type SessionState = ReturnType<typeof useSessionEvents>;

export interface CollaborateSectionProps {
  /** Current editor text; used to warn before joining discards it. */
  getEditorContent: () => string;
  /** Clear the CRDT document and the editor (before joining a session). */
  resetDocAndEditor: () => Promise<void>;
  /** Session lifecycle state owned by App. */
  session: SessionState;
  clearRoomState: () => void;
}

/**
 * Collaborate section of the side panel: host/join/end/leave, sidecar
 * health pills, copy-URL actions and the related modals.
 */
export function CollaborateSection({
  getEditorContent,
  resetDocAndEditor,
  session,
  clearRoomState,
}: CollaborateSectionProps) {
  const {
    sessionStatus,
    setSessionStatus,
    lanUrl,
    publicUrl,
    setPublicUrl,
    processesRunning,
    sessionBusy,
    setSessionBusy,
    applyIdleSessionState,
  } = session;

  const { gatewayFields, gatewayUnavailable, tunnelFields, tunnelUnavailable } =
    useMetrics(sessionStatus, processesRunning);

  const [copied, setCopied] = useState<string | null>(null);
  const [showJoinModal, setShowJoinModal] = useState(false);
  const [showSavePrompt, setShowSavePrompt] = useState(false);
  const [showGatewayMetrics, setShowGatewayMetrics] = useState(false);
  const [showTunnelMetrics, setShowTunnelMetrics] = useState(false);
  const [guestsCanWrite, setGuestsCanWrite] = useState(false);

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
      setCopied(label);
    } catch {
      setCopied(null);
    }
    window.setTimeout(() => setCopied(null), 1500);
  }, []);

  const handleHost = useCallback(async () => {
    setSessionBusy(true);
    try {
      await invoke("host_session", { guestsCanWrite });
    } catch (err) {
      setSessionStatus("error: " + String(err));
    } finally {
      setSessionBusy(false);
    }
  }, [guestsCanWrite, setSessionBusy, setSessionStatus]);

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
      .then(() => {
        applyIdleSessionState();
        clearRoomState();
      })
      .finally(() => setSessionBusy(false));
  }, [applyIdleSessionState, clearRoomState, sessionBusy, setSessionBusy]);

  const handleLeaveSession = useCallback(() => {
    void invoke("leave_session").then(() => {
      applyIdleSessionState();
      clearRoomState();
    });
  }, [applyIdleSessionState, clearRoomState]);

  const anyProcessRunning =
    processesRunning.gateway === "Enabled" ||
    processesRunning.tunnel === "Enabled";

  return (
    <div className="panel-section">
      {sessionStatus === "idle" && (
        <>
          <p className="panel-hint">
            Host a session for others to join, or join someone else&apos;s.
          </p>
          <div className="panel-actions">
            <button
              className="btn-primary"
              onClick={() => void handleHost()}
              disabled={sessionBusy}
            >
              {sessionBusy ? "starting…" : "Host"}
            </button>
            {!sessionBusy && (
              <button className="btn" onClick={handleJoinClick}>
                Join
              </button>
            )}
          </div>
          {!sessionBusy && (
            <label
              className="session-guests-edit"
              title="Whether joining guests may edit right away. You can change each guest's access later from the peers panel."
            >
              <input
                type="checkbox"
                checked={guestsCanWrite}
                onChange={(e) => setGuestsCanWrite(e.target.checked)}
              />
              guests can edit
            </label>
          )}
        </>
      )}
      {sessionStatus === "starting" && (
        <span className="pill pill-off">starting…</span>
      )}
      {isHost && (
        <>
          {anyProcessRunning && (
            <div className="panel-actions">
              {processesRunning.gateway === "Enabled" && (
                <button
                  type="button"
                  className="pill pill-gateway"
                  onClick={() => setShowGatewayMetrics(true)}
                  title="Show Peared gateway health"
                >
                  <span className="pill-dot" />
                  gateway
                </button>
              )}
              {processesRunning.tunnel === "Enabled" && (
                <button
                  type="button"
                  className="pill pill-tunnel"
                  onClick={() => setShowTunnelMetrics(true)}
                  title="Show Cloudflare tunnel health"
                >
                  <span className="pill-dot" />
                  tunnel
                </button>
              )}
            </div>
          )}
          <p className="panel-hint">
            Public URL is shared over the internet via a cloud tunnel.
          </p>
          <p className="panel-hint">
            Local URL is a direct connection over the LAN.
          </p>
          <div className="panel-stack">
            {publicUrl && (
              <button
                className="btn"
                onClick={() => void copyUrl("public", publicUrl)}
              >
                {copied === "public" ? "copied ✓" : "Copy Public URL"}
              </button>
            )}
            {lanUrl && (
              <button
                className="btn"
                onClick={() => void copyUrl("lan", lanUrl)}
              >
                {copied === "lan" ? "copied ✓" : "Copy Local URL"}
              </button>
            )}
          </div>
          <div className="panel-footer">
            <button
              className="btn-danger"
              disabled={sessionBusy}
              onClick={handleEndSession}
            >
              {sessionBusy ? "ending…" : "End Session"}
            </button>
          </div>
        </>
      )}
      {sessionStatus === "guest" && (
        <div className="panel-footer">
          <button className="btn-danger" onClick={handleLeaveSession}>
            Leave
          </button>
        </div>
      )}
      {showGatewayMetrics && isHost && (
        <MetricsPopup
          title="Peared gateway"
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
          note="Global peer count is not exposed by cloudflared. It must come from the Peared gateway, not tunnel metrics."
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
