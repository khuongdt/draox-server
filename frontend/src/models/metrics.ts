import { useState, useCallback } from 'react';

/** Maximum number of snapshots retained — 60 × 5 s = 5 min of history. */
const MAX_SNAPSHOTS = 60;

/**
 * Ring-buffer model for server metrics snapshots.
 * Older snapshots are evicted once the buffer reaches MAX_SNAPSHOTS.
 */
export default function useMetricsModel() {
  const [snapshots, setSnapshots] = useState<API.MetricsSnapshot[]>([]);
  const [latest, setLatest] = useState<API.MetricsSnapshot | null>(null);

  /** Append a new snapshot, evicting the oldest if at capacity. */
  const addSnapshot = useCallback((snap: API.MetricsSnapshot) => {
    setSnapshots((prev) => {
      const next = [...prev, snap];
      return next.length > MAX_SNAPSHOTS ? next.slice(-MAX_SNAPSHOTS) : next;
    });
    setLatest(snap);
  }, []);

  /** Clear all retained snapshots (e.g. on logout or page leave). */
  const clear = useCallback(() => {
    setSnapshots([]);
    setLatest(null);
  }, []);

  return { snapshots, latest, addSnapshot, clear };
}
