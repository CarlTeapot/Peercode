const PEER_COLORS = [
  "var(--accent)",
  "var(--blue)",
  "var(--cyan)",
  "var(--green)",
  "var(--amber)",
  "var(--red)",
];

/**
 * Stable per-peer color. client_id is a decimal u64 string (> 2^53, so it
 * can't be parsed as a number); the last 6 digits are enough for a stable
 * bucket. The peer-cursor feature must use this same function.
 */
export function peerColor(clientId: string): string {
  const tail = Number(clientId.slice(-6));
  const idx = Number.isFinite(tail) ? tail % PEER_COLORS.length : 0;
  return PEER_COLORS[idx];
}

export { PEER_COLORS };
