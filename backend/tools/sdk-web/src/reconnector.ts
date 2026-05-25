import type { ReconnectConfig } from './types.js';

interface ResolvedReconnect {
  enabled: boolean;
  maxAttempts: number;
  baseDelayMs: number;
  maxDelayMs: number;
}

export class Reconnector {
  private attempt = 0;

  constructor(private readonly cfg: ResolvedReconnect) {}

  reset(): void { this.attempt = 0; }

  async run(
    tryConnect: () => Promise<boolean>,
    signal: AbortSignal,
  ): Promise<boolean> {
    if (!this.cfg.enabled) return false;

    while (this.attempt < this.cfg.maxAttempts) {
      if (signal.aborted) return false;

      this.attempt++;
      const delay = Math.min(
        this.cfg.baseDelayMs * 2 ** (this.attempt - 1),
        this.cfg.maxDelayMs,
      );

      await new Promise<void>(res => setTimeout(res, delay));

      if (signal.aborted) return false;
      if (await tryConnect()) { this.reset(); return true; }
    }

    return false;
  }
}
