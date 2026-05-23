import WebSocket from 'ws';
import type { ITransport } from './ITransport';

export class WebSocketTransport implements ITransport {
  private ws: WebSocket | null = null;

  onMessage: ((data: string) => void) | null = null;
  onClose:   ((reason: string) => void) | null = null;

  get isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  connect(host: string, port: number, useTls = false): Promise<void> {
    const scheme = useTls ? 'wss' : 'ws';
    const url    = `${scheme}://${host}:${port}`;

    return new Promise((resolve, reject) => {
      const ws = new WebSocket(url);

      ws.once('open',  () => { this.ws = ws; resolve(); });
      ws.once('error', (err) => reject(err));

      ws.on('message', (data) => {
        this.onMessage?.(data.toString());
      });

      ws.on('close', () => {
        this.ws = null;
        this.onClose?.('server_close');
      });
    });
  }

  disconnect(): void {
    this.ws?.close();
    this.ws = null;
  }

  send(json: string): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.ws || !this.isConnected) {
        reject(new Error('WebSocket is not connected'));
        return;
      }
      this.ws.send(json, (err) => {
        if (err) reject(err);
        else resolve();
      });
    });
  }
}
