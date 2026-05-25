export class WsTransport {
  private ws: WebSocket | null = null;

  onMessage: ((data: string) => void) | null = null;
  onClose:   ((reason: string) => void) | null = null;

  get isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  connect(host: string, port: number, useTls = false): Promise<void> {
    const url = `${useTls ? 'wss' : 'ws'}://${host}:${port}`;

    return new Promise((resolve, reject) => {
      const ws = new WebSocket(url);
      let settled = false;

      ws.onopen = () => {
        settled = true;
        this.ws = ws;
        resolve();
      };

      ws.onerror = () => {
        if (!settled) reject(new Error(`WebSocket connection failed: ${url}`));
      };

      ws.onmessage = (e: MessageEvent) => {
        this.onMessage?.(typeof e.data === 'string' ? e.data : String(e.data));
      };

      ws.onclose = () => {
        this.ws = null;
        this.onClose?.('server_close');
      };
    });
  }

  disconnect(): void {
    this.ws?.close();
    this.ws = null;
  }

  send(json: string): void {
    if (this.ws && this.isConnected) this.ws.send(json);
  }
}
