import type { WireResponse } from './types';

interface Pending {
  resolve: (r: WireResponse) => void;
  reject:  (e: Error) => void;
  timer:   ReturnType<typeof setTimeout>;
}

export class RequestBroker {
  private readonly pending = new Map<string, Pending>();

  send(
    sendFn: (json: string) => Promise<void>,
    json:   string,
    id:     string,
    timeoutMs: number,
  ): Promise<WireResponse> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Request '${id}' timed out`));
      }, timeoutMs);

      this.pending.set(id, { resolve, reject, timer });

      sendFn(json).catch((err: Error) => {
        if (this.pending.delete(id)) {
          clearTimeout(timer);
          reject(err);
        }
      });
    });
  }

  complete(id: string, response: WireResponse): void {
    const entry = this.pending.get(id);
    if (!entry) return;
    clearTimeout(entry.timer);
    this.pending.delete(id);
    entry.resolve(response);
  }

  failAll(error: Error): void {
    for (const [id, entry] of this.pending) {
      clearTimeout(entry.timer);
      entry.reject(error);
    }
    this.pending.clear();
  }
}
