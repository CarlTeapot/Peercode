import { useState, useCallback } from "react";
import { useTauriEvents } from "./useTauriEvents";

/** client_id is a string: ids are random u64s and JS numbers lose precision. */
export interface PeerInfo {
  client_id: string;
  username: string;
  is_host: boolean;
  can_write: boolean;
}

export interface RoomState {
  peers: PeerInfo[];
}

/**
 * Live session peer list, fed by the backend's `session://room-state` events
 * (which mirror the gateway's roster). Cleared automatically when the session
 * ends remotely; call `clearRoomState` on locally initiated end/leave.
 */
export function useRoomState() {
  const [roomState, setRoomState] = useState<RoomState | null>(null);
  const clearRoomState = useCallback(() => setRoomState(null), []);

  useTauriEvents(
    useCallback(
      (on) => {
        on<RoomState>("session://room-state", setRoomState);
        on("session://session-ended", clearRoomState);
        on("session://disconnected", clearRoomState);
      },
      [clearRoomState],
    ),
  );

  return { roomState, clearRoomState };
}
