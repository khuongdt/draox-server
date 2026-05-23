import { useState, useEffect } from 'react';
import { wsManager, StreamName } from '@/services/wsManager';
import WebSocketIndicator from '@/components/WebSocketIndicator';

const STREAMS: StreamName[] = ['metrics', 'events', 'connections', 'plugins', 'guard'];

type StatusMap = Record<StreamName, 'connected' | 'connecting' | 'closed'>;

/** Header component showing live status for all 5 WebSocket streams. */
const WsHeaderIndicator: React.FC = () => {
  const [statuses, setStatuses] = useState<StatusMap>({
    metrics: 'closed',
    events: 'closed',
    connections: 'closed',
    plugins: 'closed',
    guard: 'closed',
  });

  useEffect(() => {
    // Keep all streams alive for the lifetime of the admin session.
    // No-op listeners ensure the socket stays open even if no page is actively
    // reading from that stream at the moment.
    const unsubs = STREAMS.map((stream) => wsManager.subscribe(stream, () => {}));

    // Poll readyState every 1.5 s so the dots reflect reconnection attempts.
    const poll = setInterval(() => {
      const next = {} as StatusMap;
      STREAMS.forEach((s) => {
        next[s] = wsManager.getStatus(s);
      });
      setStatuses((prev) => {
        const changed = STREAMS.some((s) => prev[s] !== next[s]);
        return changed ? next : prev;
      });
    }, 1500);

    return () => {
      unsubs.forEach((u) => u());
      clearInterval(poll);
    };
  }, []);

  return (
    <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
      {STREAMS.map((s) => (
        <WebSocketIndicator key={s} stream={s} status={statuses[s]} />
      ))}
    </div>
  );
};

export default WsHeaderIndicator;
