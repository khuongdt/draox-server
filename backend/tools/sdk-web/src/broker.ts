import type { WsResponse } from './types.js';

interface Pending {
  resolve: (res: WsResponse) => void;
  reject:  (err: Error) => void;
  timer:   ReturnType<typeof setTimeout>;
}

export class RequestBroker {
  private readonly _pending = new Map<string, Pending>();

  send(
    sendFn: (json: string) => void,
    json: string,
    id: string,
    timeoutMs: number,
  ): Promise<WsResponse> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this._pending.delete(id);
        reject(new Error(`Request timed out: ${id}`));
      }, timeoutMs);

      this._pending.set(id, { resolve, reject, timer });
      sendFn(json);
    });
  }

  complete(id: string, res: WsResponse): void {
    const p = this._pending.get(id);
    if (!p) return;
    this._pending.delete(id);
    clearTimeout(p.timer);
    p.resolve(res);
  }

  failAll(err: Error): void {
    for (const [id, p] of this._pending) {
      clearTimeout(p.timer);
      p.reject(err);
      this._pending.delete(id);
    }
  }
}
