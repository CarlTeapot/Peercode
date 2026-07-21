import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type SessionNotice = "ended" | "disconnected";
export type SidecarStatus = "Enabled" | "Disabled";

export interface ProcessStatus {
  gateway: SidecarStatus;
  tunnel: SidecarStatus;
}

interface SessionInfo {
  status: string;
  lan_url: string | null;
  public_url: string | null;
  local_room_url: string | null;
  public_room_url: string | null;
  room_id: string | null;
}

interface SessionReadyPayload {
  lan_url: string | null;
  public_url: string | null;
  local_room_url: string;
  public_room_url: string | null;
  port: number;
  room_id: string;
}

const IDLE_PROCESSES: ProcessStatus = {
  gateway: "Disabled",
  tunnel: "Disabled",
};

/**
 * Session lifecycle state: status, share URLs, sidecar process status and
 * transient exit notices. Subscribes to the backend session events and
 * fetches the initial state once on mount.
 */
export function useSessionEvents() {
  const [sessionStatus, setSessionStatus] = useState("loading");
  const sessionStatusRef = useRef(sessionStatus);
  useEffect(() => {
    sessionStatusRef.current = sessionStatus;
  }, [sessionStatus]);
  const [lanUrl, setLanUrl] = useState<string | null>(null);
  const [publicUrl, setPublicUrl] = useState<string | null>(null);
  const [roomId, setRoomId] = useState<string | null>(null);
  const [processesRunning, setProcessesRunning] =
    useState<ProcessStatus>(IDLE_PROCESSES);
  const [sessionNotice, setSessionNotice] = useState<SessionNotice | null>(
    null,
  );
  const noticeTimerRef = useRef<number | null>(null);
  const [sessionBusy, setSessionBusy] = useState(false);

  const applyIdleSessionState = useCallback(() => {
    setSessionStatus("idle");
    setLanUrl(null);
    setPublicUrl(null);
    setRoomId(null);
    setProcessesRunning(IDLE_PROCESSES);
    setSessionBusy(false);
  }, []);

  const showSessionNotice = useCallback((notice: SessionNotice) => {
    if (noticeTimerRef.current !== null) {
      window.clearTimeout(noticeTimerRef.current);
    }
    setSessionNotice(notice);
    noticeTimerRef.current = window.setTimeout(() => {
      setSessionNotice(null);
      noticeTimerRef.current = null;
    }, 5000);
  }, []);

  const handleRemoteSessionExit = useCallback(
    (notice: SessionNotice, allowedRoles: readonly string[]) => {
      if (!allowedRoles.includes(sessionStatusRef.current)) return;
      // Backend disconnect_handler already reset role/WS before emitting.
      applyIdleSessionState();
      showSessionNotice(notice);
    },
    [applyIdleSessionState, showSessionNotice],
  );

  useEffect(() => {
    void invoke<SessionInfo>("get_session_info").then((info) => {
      setSessionStatus(info.status);

      if (info.lan_url && !info.room_id) {
        throw new Error(
          "get_session_info: lan_url present but room_id is null",
        );
      }

      setRoomId(info.room_id);

      const lanRoomUrl =
        info.lan_url && info.room_id
          ? `${info.lan_url}?room=${info.room_id}`
          : null;

      if (lanRoomUrl) setLanUrl(lanRoomUrl);
      if (info.public_room_url ?? info.public_url) {
        setPublicUrl(info.public_room_url ?? info.public_url);
      }
    });

    void invoke<ProcessStatus>("get_process_status").then(setProcessesRunning);
  }, []);

  useEffect(() => {
    const unlisten: (() => void)[] = [];
    let cancelled = false;

    void (async () => {
      const register = (fn: () => void) => {
        if (cancelled) fn();
        else unlisten.push(fn);
      };

      register(
        await listen<SessionReadyPayload>("session://session-ready", (e) => {
          setSessionStatus("host");
          setLanUrl(
            e.payload.lan_url
              ? `${e.payload.lan_url}?room=${e.payload.room_id}`
              : e.payload.local_room_url,
          );
          setPublicUrl(e.payload.public_room_url);
          setRoomId(e.payload.room_id);
          setProcessesRunning({
            gateway: "Enabled",
            tunnel: e.payload.public_url !== null ? "Enabled" : "Disabled",
          });
        }),
      );
      register(
        await listen<{ message: string }>("session://session-error", (e) => {
          setSessionStatus("error: " + e.payload.message);
          setProcessesRunning(IDLE_PROCESSES);
        }),
      );
      register(
        await listen("session://processes-stopped", () => {
          setProcessesRunning(IDLE_PROCESSES);
        }),
      );
      register(
        await listen("session://session-ended", () => {
          // Guests only: the gateway may echo end-session to the host too.
          handleRemoteSessionExit("ended", ["guest"]);
        }),
      );
      register(
        await listen("session://disconnected", () => {
          handleRemoteSessionExit("disconnected", ["host", "guest"]);
        }),
      );
    })();

    return () => {
      cancelled = true;
      unlisten.forEach((fn) => fn());
      if (noticeTimerRef.current !== null) {
        window.clearTimeout(noticeTimerRef.current);
        noticeTimerRef.current = null;
      }
    };
  }, [handleRemoteSessionExit]);

  return {
    sessionStatus,
    setSessionStatus,
    lanUrl,
    publicUrl,
    setPublicUrl,
    roomId,
    processesRunning,
    setProcessesRunning,
    sessionNotice,
    sessionBusy,
    setSessionBusy,
    applyIdleSessionState,
  };
}
