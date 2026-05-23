import ReconnectingWebSocket from 'reconnecting-websocket';

export type StreamName = 'events' | 'connections' | 'plugins' | 'guard' | 'metrics';
type Listener = (data: unknown) => void;

/** WebSocket URL paths for each available stream. */
const WS_PATHS: Record<StreamName, string> = {
  events: '/ws/events',
  connections: '/ws/connections',
  plugins: '/ws/plugins',
  guard: '/ws/guard',
  metrics: '/ws/metrics',
};

class WsManager {
  private sockets: Partial<Record<StreamName, ReconnectingWebSocket>> = {};
  private listeners: Partial<Record<StreamName, Set<Listener>>> = {};

  /** Return current readyState label for a stream. */
  getStatus(stream: StreamName): 'connected' | 'connecting' | 'closed' {
    const ws = this.sockets[stream];
    if (!ws) return 'closed';
    switch (ws.readyState) {
      case WebSocket.OPEN:
        return 'connected';
      case WebSocket.CONNECTING:
        return 'connecting';
      default:
        return 'closed';
    }
  }

  /** Open a WebSocket connection for the given stream if not already open. */
  connect(stream: StreamName): void {
    if (this.sockets[stream]) return;
    const token = localStorage.getItem('draox_token');
    const base = window.location.origin.replace(/^http/, 'ws');
    const url = `${base}${WS_PATHS[stream]}?token=${token}`;
    const ws = new ReconnectingWebSocket(url, [], {
      maxRetries: Infinity,
      minReconnectionDelay: 1000,
      maxReconnectionDelay: 30000,
      reconnectionDelayGrowFactor: 1.5,
    });
    ws.onmessage = (evt) => {
      try {
        const data = JSON.parse(evt.data as string);
        this.listeners[stream]?.forEach((fn) => fn(data));
      } catch {
        // Ignore malformed frames — do not crash the listener loop
      }
    };
    this.sockets[stream] = ws;
  }

  /**
   * Subscribe a listener to a stream.
   * Opens the WebSocket if not already connected.
   * Returns an unsubscribe function that also closes the socket when no listeners remain.
   */
  subscribe(stream: StreamName, listener: Listener): () => void {
    if (!this.listeners[stream]) this.listeners[stream] = new Set();
    this.listeners[stream]!.add(listener);
    this.connect(stream);
    return () => {
      this.listeners[stream]?.delete(listener);
      if (this.listeners[stream]?.size === 0) this.disconnect(stream);
    };
  }

  /** Close and clean up a stream connection. */
  disconnect(stream: StreamName): void {
    this.sockets[stream]?.close();
    delete this.sockets[stream];
    delete this.listeners[stream];
  }

  /** Close all open WebSocket connections. */
  disconnectAll(): void {
    (Object.keys(this.sockets) as StreamName[]).forEach((s) => this.disconnect(s));
  }
}

export const wsManager = new WsManager();
