import type { RoomState, PeerInfo } from "../hooks/useRoomState";
import { peerColor } from "../lib/peerColor";
import "./PeerAvatars.css";

const MAX_SHOWN = 5;

interface PeerAvatarsProps {
  roomState: RoomState;
  onClick: () => void;
}

/** Google-Docs-style overlapping avatar stack; click opens the peers panel. */
export function PeerAvatars({ roomState, onClick }: PeerAvatarsProps) {
  const peers = roomState.peers;
  const shown = peers.slice(0, MAX_SHOWN);
  const overflow = peers.length - shown.length;

  return (
    <button
      className="peer-avatars"
      onClick={onClick}
      title="Show peers panel"
      aria-label={`${peers.length} ${peers.length === 1 ? "peer" : "peers"} — show peers panel`}
    >
      {shown.map((p) => (
        <AvatarChip key={p.client_id} peer={p} />
      ))}
      {overflow > 0 && (
        <span className="peer-avatar-chip more">+{overflow}</span>
      )}
    </button>
  );
}

function AvatarChip({ peer }: { peer: PeerInfo }) {
  const name = peer.username || `Client ${peer.client_id.slice(0, 6)}`;
  return (
    <span
      className={"peer-avatar-chip" + (peer.is_host ? " host" : "")}
      style={{ background: peerColor(peer.client_id) }}
      title={peer.is_host ? `${name} (host)` : name}
    >
      {name.charAt(0).toUpperCase()}
    </span>
  );
}
