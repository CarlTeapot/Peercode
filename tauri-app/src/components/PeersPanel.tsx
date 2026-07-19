import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RoomState, PeerInfo } from "../hooks/useRoomState";
import "./PeersPanel.css";

interface PeersPanelProps {
  roomState: RoomState | null;
  /** Whether the local user hosts the session (only hosts see the toggles). */
  isHost: boolean;
}

/**
 * Floating peer list for an active session. Every member sees who is in the
 * room and their write permission; the host can grant/revoke write access
 * per guest. Toggles are optimistic-free: the switch flips only when the
 * gateway's authoritative room-state echo arrives.
 */
export function PeersPanel({ roomState, isHost }: PeersPanelProps) {
  const [open, setOpen] = useState(false);

  const togglePermission = useCallback(async (peer: PeerInfo) => {
    try {
      await invoke("set_peer_permission", {
        targetClientId: peer.client_id,
        canWrite: !peer.can_write,
      });
    } catch (e) {
      console.error("Failed to set peer permission:", e);
    }
  }, []);

  if (!roomState) return null;

  // The backend emits peers already sorted (host first, then by id).
  const peers = roomState.peers;

  return (
    <>
      <button
        className="peers-panel-toggle"
        onClick={() => setOpen((prev) => !prev)}
        title="Peers"
      >
        👥
        {peers.length > 0 && <span className="badge">{peers.length}</span>}
      </button>

      {open && (
        <div className="peers-panel">
          <div className="peers-panel-header">
            <h3>Peers ({peers.length})</h3>
            <button
              className="close-btn"
              onClick={() => setOpen(false)}
              title="Close"
            >
              ✕
            </button>
          </div>

          <div className="peers-list">
            {peers.map((peer) => (
              <PeerRow
                key={peer.client_id}
                peer={peer}
                isHost={isHost}
                onToggle={togglePermission}
              />
            ))}
          </div>
        </div>
      )}
    </>
  );
}

interface PeerRowProps {
  peer: PeerInfo;
  isHost: boolean;
  onToggle: (peer: PeerInfo) => Promise<void>;
}

function PeerRow({ peer, isHost, onToggle }: PeerRowProps) {
  const displayName = peer.username || `Client ${peer.client_id.slice(0, 6)}`;
  const initial = displayName.charAt(0).toUpperCase();
  const showToggle = isHost && !peer.is_host;

  return (
    <div className="peer-row">
      <div className={`peer-avatar ${peer.is_host ? "host" : "guest"}`}>
        {initial}
      </div>

      <div className="peer-info">
        <div className="peer-name">{displayName}</div>
        <div className={`peer-role ${peer.is_host ? "host-role" : ""}`}>
          {peer.is_host ? "Host" : "Guest"}
        </div>
      </div>

      {showToggle ? (
        <div className="perm-control">
          <span
            className={`perm-label ${peer.can_write ? "write" : "readonly"}`}
          >
            {peer.can_write ? "Can Edit" : "Read Only"}
          </span>
          <button
            className={`perm-toggle ${peer.can_write ? "can-write" : "read-only"}`}
            onClick={() => void onToggle(peer)}
            title={
              peer.can_write ? "Revoke write access" : "Grant write access"
            }
          >
            <span className="knob" />
          </button>
        </div>
      ) : (
        <span className={`perm-badge ${peer.can_write ? "write" : "readonly"}`}>
          {peer.can_write ? "Can Edit" : "Read Only"}
        </span>
      )}
    </div>
  );
}
