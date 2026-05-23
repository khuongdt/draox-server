import type { ResolvedConfig } from './types';

export class Reconnector {
  constructor(private readonly cfg: ResolvedConfig['reconnect']) {}

  async attempt(tryConnect: () => Promise<boolean>, signal: AbortSignal): Promise<boolean> {
    let n = 0;
    while (!signal.aborted) {
      if (this.cfg.maxAttempts > 0 && n >= this.cfg.maxAttempts) return false;
      n++;

      const delay = Math.min(this.cfg.baseDelayMs * Math.pow(2, n - 1), this.cfg.maxDelayMs);
      await new Promise<void>((res, rej) => {
        const t = setTimeout(res, delay);
        signal.addEventListener('abort', () => { clearTimeout(t); rej(new Error('aborted')); }, { once: true });
      }).catch(() => null);

      if (signal.aborted) return false;

      try { if (await tryConnect()) return true; }
      catch { /* next attempt */ }
    }
    return false;
  }
}
