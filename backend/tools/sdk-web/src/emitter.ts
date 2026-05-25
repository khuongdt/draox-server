type Fn<T = unknown> = (e: T) => void;

export class Emitter {
  private readonly _m = new Map<string, Set<Fn>>();

  on<T>(evt: string, fn: Fn<T>): void {
    if (!this._m.has(evt)) this._m.set(evt, new Set());
    this._m.get(evt)!.add(fn as Fn);
  }

  off<T>(evt: string, fn: Fn<T>): void {
    this._m.get(evt)?.delete(fn as Fn);
  }

  emit<T>(evt: string, data?: T): void {
    this._m.get(evt)?.forEach(fn => fn(data));
  }

  once<T>(evt: string, fn: Fn<T>): void {
    const wrapper = (e: unknown) => { fn(e as T); this._m.get(evt)?.delete(wrapper); };
    this.on(evt, wrapper);
  }
}
