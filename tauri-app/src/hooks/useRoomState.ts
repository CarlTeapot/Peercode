import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";

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

  useEffect(() => {
    const unlisten: (() => void)[] = [];
    let cancelled = false;

    void (async () => {
      const register = (fn: () => void) => {
        if (cancelled) fn();
        else unlisten.push(fn);
      };

      register(
        await listen<RoomState>("session://room-state", (e) => {
          setRoomState(e.payload);
        }),
      );
      register(await listen("session://session-ended", clearRoomState));
      register(await listen("session://disconnected", clearRoomState));
    })();

    return () => {
      cancelled = true;
      unlisten.forEach((fn) => fn());
    };
  }, [clearRoomState]);

  return { roomState, clearRoomState };
}
