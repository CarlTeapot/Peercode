import { useState, useEffect, useMemo, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { MetricsPopupField } from "../components/MetricsPopup";
import type { ProcessStatus } from "./useSessionEvents";

interface TunnelMetrics {
  ha_connections: number;
  register_successes: number;
  request_errors: number;
  edge_location: string | null;
}

interface GatewayMetrics {
  healthy: boolean;
  uptime_seconds: number;
  active_rooms: number;
  connected_clients: number;
  active_hosts: number;
  relayed_messages: number;
  relayed_bytes: number;
  replay_successes: number;
  replay_failures: number;
  dropped_frames: number;
  slow_client_disconnects: number;
}

interface MetricsPayload<M> {
  metrics: M | null;
  error: string | null;
}

const EDGE_CITIES: Record<string, string> = {
  ams: "Amsterdam",
  cdg: "Paris",
  fra: "Frankfurt",
  ist: "Istanbul",
  lhr: "London",
  vie: "Vienna",
  waw: "Warsaw",
};

function edgeLocationLabel(location: string | null): string {
  if (!location) return "Unknown edge";
  const city = EDGE_CITIES[location.slice(0, 3).toLowerCase()];
  return city ? `${city} (${location})` : location.toUpperCase();
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}

function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}

/**
 * Gateway and tunnel sidecar metrics: subscribes to the backend metrics
 * events, tracks deltas since the previous sample ("recent" counts) and
 * resets everything when the session or process stops.
 */
export function useMetrics(
  sessionStatus: string,
  processesRunning: ProcessStatus,
) {
  const [gatewayMetrics, setGatewayMetrics] = useState<GatewayMetrics | null>(
    null,
  );
  const [gatewayMetricsError, setGatewayMetricsError] = useState<string | null>(
    null,
  );
  const [recentReplayFailures, setRecentReplayFailures] = useState(0);
  const [recentDroppedFrames, setRecentDroppedFrames] = useState(0);
  const previousGatewayRef = useRef<GatewayMetrics | null>(null);

  const [tunnelMetrics, setTunnelMetrics] = useState<TunnelMetrics | null>(
    null,
  );
  const [tunnelMetricsError, setTunnelMetricsError] = useState<string | null>(
    null,
  );
  const [recentTunnelErrors, setRecentTunnelErrors] = useState(0);
  const previousTunnelRef = useRef<TunnelMetrics | null>(null);
  const registrationAtDropRef = useRef<number | null>(null);

  useEffect(() => {
    const unlisten: (() => void)[] = [];
    let cancelled = false;

    void (async () => {
      const register = (fn: () => void) => {
        if (cancelled) fn();
        else unlisten.push(fn);
      };

      register(
        await listen<MetricsPayload<GatewayMetrics>>(
          "process://gateway-metrics",
          (event) => {
            const { metrics, error } = event.payload;
            if (!metrics) {
              setGatewayMetrics(null);
              setGatewayMetricsError(error ?? "Metrics server unavailable");
              setRecentReplayFailures(0);
              setRecentDroppedFrames(0);
              previousGatewayRef.current = null;
              return;
            }

            const previous = previousGatewayRef.current;
            setRecentReplayFailures(
              previous
                ? Math.max(
                    0,
                    metrics.replay_failures - previous.replay_failures,
                  )
                : 0,
            );
            setRecentDroppedFrames(
              previous
                ? Math.max(0, metrics.dropped_frames - previous.dropped_frames)
                : 0,
            );

            previousGatewayRef.current = metrics;
            setGatewayMetrics(metrics);
            setGatewayMetricsError(null);
          },
        ),
      );
      register(
        await listen<MetricsPayload<TunnelMetrics>>(
          "process://tunnel-metrics",
          (event) => {
            const { metrics, error } = event.payload;
            if (!metrics) {
              setTunnelMetrics(null);
              setTunnelMetricsError(error ?? "Metrics server unavailable");
              setRecentTunnelErrors(0);
              previousTunnelRef.current = null;
              registrationAtDropRef.current = null;
              return;
            }

            const previous = previousTunnelRef.current;
            setRecentTunnelErrors(
              previous
                ? Math.max(0, metrics.request_errors - previous.request_errors)
                : 0,
            );

            if (metrics.ha_connections === 0) {
              registrationAtDropRef.current ??= metrics.register_successes;
            } else {
              registrationAtDropRef.current = null;
            }

            previousTunnelRef.current = metrics;
            setTunnelMetrics(metrics);
            setTunnelMetricsError(null);
          },
        ),
      );
    })();

    return () => {
      cancelled = true;
      unlisten.forEach((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (sessionStatus !== "host" || processesRunning.gateway !== "Enabled") {
      setGatewayMetrics(null);
      setGatewayMetricsError(null);
      setRecentReplayFailures(0);
      setRecentDroppedFrames(0);
      previousGatewayRef.current = null;
    }
  }, [processesRunning.gateway, sessionStatus]);

  useEffect(() => {
    if (sessionStatus !== "host" || processesRunning.tunnel !== "Enabled") {
      setTunnelMetrics(null);
      setTunnelMetricsError(null);
      setRecentTunnelErrors(0);
      previousTunnelRef.current = null;
      registrationAtDropRef.current = null;
    }
  }, [processesRunning.tunnel, sessionStatus]);

  const gatewayFields = useMemo<MetricsPopupField[]>(() => {
    if (!gatewayMetrics) return [];
    return [
      {
        name: "healthy",
        value: gatewayMetrics.healthy ? "true" : "false",
        tone: gatewayMetrics.healthy ? "ok" : "warning",
      },
      {
        name: "connected_clients",
        value: String(gatewayMetrics.connected_clients),
      },
      { name: "active_rooms", value: String(gatewayMetrics.active_rooms) },
      { name: "active_hosts", value: String(gatewayMetrics.active_hosts) },
      {
        name: "relayed_messages",
        value: String(gatewayMetrics.relayed_messages),
      },
      {
        name: "relayed_bytes",
        value: formatBytes(gatewayMetrics.relayed_bytes),
      },
      {
        name: "replay_failures",
        value: `${gatewayMetrics.replay_failures} (${recentReplayFailures} recent)`,
        tone: recentReplayFailures > 0 ? "warning" : undefined,
      },
      {
        name: "dropped_frames",
        value: `${gatewayMetrics.dropped_frames} (${recentDroppedFrames} recent)`,
        tone: recentDroppedFrames > 0 ? "warning" : undefined,
      },
      {
        name: "slow_client_disconnects",
        value: String(gatewayMetrics.slow_client_disconnects),
        tone:
          gatewayMetrics.slow_client_disconnects > 0 ? "warning" : undefined,
      },
      {
        name: "uptime_seconds",
        value: formatUptime(gatewayMetrics.uptime_seconds),
      },
    ];
  }, [gatewayMetrics, recentDroppedFrames, recentReplayFailures]);

  const tunnelFields = useMemo<MetricsPopupField[]>(() => {
    if (!tunnelMetrics) return [];
    const connected = tunnelMetrics.ha_connections > 0;
    const recoveryRegistered =
      registrationAtDropRef.current !== null &&
      registrationAtDropRef.current !== tunnelMetrics.register_successes;

    return [
      {
        name: "ha_connections",
        value: connected ? "Connected" : "Disconnected",
        tone: connected ? "ok" : "warning",
      },
      {
        name: "tunnel_register_success",
        value: connected
          ? "Healthy"
          : recoveryRegistered
            ? "Reconnecting"
            : "Recovery not confirmed",
        tone: connected ? "ok" : "warning",
      },
      {
        name: "request_errors",
        value: `${tunnelMetrics.request_errors} (${recentTunnelErrors} recent)`,
        tone: recentTunnelErrors > 0 ? "warning" : undefined,
      },
      {
        name: "server_locations",
        value: edgeLocationLabel(tunnelMetrics.edge_location),
      },
    ];
  }, [recentTunnelErrors, tunnelMetrics]);

  return {
    gatewayFields,
    gatewayUnavailable: gatewayMetricsError !== null,
    tunnelFields,
    tunnelUnavailable: tunnelMetricsError !== null,
  };
}
