import { EventEmitter } from 'events';
import { WebSocketTransport } from './transports/WebSocketTransport';
import type { ITransport } from './transports/ITransport';
import type { DraoxConfig, DraoxEvent, ResolvedConfig, ClientState, LoginResponse } from './types';
import { Serializer } from './Serializer';
import { RequestBroker } from './RequestBroker';
import { Reconnector } from './Reconnector';

export declare interface DraoxClient {
  on(event: 'connected',    listener: () => void): this;
  on(event: 'disconnected', listener: (reason: string) => void): this;
  on(event: 'authenticated',listener: () => void): this;
  on(event: 'stateChanged', listener: (state: ClientState) => void): this;
  on(event: 'error',        listener: (err: Error) => void): this;
}

export class DraoxClient extends EventEmitter {
  private readonly cfg: ResolvedConfig;
  private transport:    ITransport | null = null;
  private broker:       RequestBroker;
  private reconnector:  Reconnector;
  private heartbeat:    ReturnType<typeof setInterval> | null = null;
  private abort:        AbortController | null = null;
  private missedPings = 0;

  private savedUserId: string | null = null;
  private savedToken:  string | null = null;

  private _state:     ClientState = 'disconnected';
  private _sessionId: string | null = null;

  private readonly eventHandlers    = new Map<string, Set<(e: DraoxEvent) => void>>();
  private readonly categoryHandlers = new Map<string, Set<(e: DraoxEvent) => void>>();

  get state():           ClientState    { return this._state; }
  get sessionId():       string | null  { return this._sessionId; }
  get isAuthenticated(): boolean        { return this._sessionId !== null; }

  constructor(config: DraoxConfig = {}) {
    super();
    const protocol = config.protocol ?? 'ws';
    const defaultPort = protocol === 'grpc' ? 9004 : 9002;
    this.cfg = {
      host:                config.host                ?? 'localhost',
      port:                config.port                ?? defaultPort,
      adminPort:           config.adminPort           ?? 9100,
      protocol,
      useTls:              config.useTls              ?? false,
      timeoutMs:           config.timeoutMs           ?? 10_000,
      heartbeatIntervalMs: config.heartbeatIntervalMs ?? 30_000,
      reconnect: {
        enabled:     config.reconnect?.enabled     ?? true,
        maxAttempts: config.reconnect?.maxAttempts ?? 5,
        baseDelayMs: config.reconnect?.baseDelayMs ?? 1_000,
        maxDelayMs:  config.reconnect?.maxDelayMs  ?? 30_000,
      },
      grpc: {
        protoPath:   config.grpc?.protoPath   ?? '',
        credentials: config.grpc?.credentials ?? 'insecure',
      },
    };
    this.broker      = new RequestBroker();
    this.reconnector = new Reconnector(this.cfg.reconnect);
  }

  // ── Public API ────────────────────────────────────────────────────────────

  async connect(): Promise<void> {
    if (this._state === 'connected' || this._state === 'connecting') return;

    this.abort = new AbortController();
    this.setState('connecting');

    this.transport = new WebSocketTransport();
    this.transport.onMessage = (msg) => this.onMessage(msg);
    this.transport.onClose   = (reason) => this.onClosed(reason);

    await this.transport.connect(this.cfg.host, this.cfg.port, this.cfg.useTls);

    this.setState('connected');
    this.emit('connected');
    this.startHeartbeat();
  }

  async disconnect(reason = 'client_disconnect'): Promise<void> {
    this.stopHeartbeat();
    this.abort?.abort();

    if (this.transport) {
      this.broker.failAll(new Error(`Disconnected: ${reason}`));
      this.transport.disconnect();
      this.transport = null;
    }

    this._sessionId = null;
    this.setState('disconnected');
    this.emit('disconnected', reason);
  }

  async authenticate(userId: string, token: string): Promise<void> {
    this.savedUserId = userId;
    this.savedToken  = token;

    const data = await this.requestInternal<{ session_id: string }>('auth', { user_id: userId, token });
    this._sessionId = data.session_id;
    this.emit('authenticated');
  }

  async login(username: string, password: string): Promise<void> {
    const scheme = this.cfg.useTls ? 'https' : 'http';
    const url    = `${scheme}://${this.cfg.host}:${this.cfg.adminPort}/api/auth/login`;

    const res = await fetch(url, {
      method:  'POST',
      headers: { 'Content-Type': 'application/json' },
      body:    JSON.stringify({ username, password }),
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Login failed (${res.status}): ${text}`);
    }

    const body = await res.json() as { success: boolean; data: LoginResponse };
    if (!body.success || !body.data?.token)
      throw new Error('Login failed: unexpected response format');

    await this.authenticate(body.data.username, body.data.token);
  }

  async send(action: string, payload?: unknown): Promise<void> {
    this.ensureConnected();
    const json = Serializer.serialize({ id: this.newId(), type: 'request', action, payload });
    await this.transport!.send(json);
  }

  async request<T>(action: string, payload?: unknown): Promise<T> {
    this.ensureConnected();
    return this.requestInternal<T>(action, payload);
  }

  subscribe(eventName: string, handler: (e: DraoxEvent) => void): void {
    this.addHandler(this.eventHandlers, eventName, handler);
  }

  unsubscribe(eventName: string, handler: (e: DraoxEvent) => void): void {
    this.removeHandler(this.eventHandlers, eventName, handler);
  }

  subscribeCategory(category: string, handler: (e: DraoxEvent) => void): void {
    this.addHandler(this.categoryHandlers, category, handler);
  }

  unsubscribeCategory(category: string, handler: (e: DraoxEvent) => void): void {
    this.removeHandler(this.categoryHandlers, category, handler);
  }

  // ── Internal ──────────────────────────────────────────────────────────────

  private async requestInternal<T>(action: string, payload?: unknown): Promise<T> {
    const id   = this.newId();
    const json = Serializer.serialize({ id, type: 'request', action, payload });
    const res  = await this.broker.send(
      (j) => this.transport!.send(j), json, id, this.cfg.timeoutMs,
    );
    if (!res.success) throw new Error(res.error ?? 'request failed');
    return res.data as T;
  }

  private onMessage(json: string): void {
    const msg = Serializer.parse(json);
    if (!msg) return;

    switch (msg.type) {
      case 'response':
        if (msg.id)
          this.broker.complete(msg.id, {
            id:      msg.id,
            success: msg.success ?? false,
            data:    msg.data,
            error:   msg.error,
          });
        break;

      case 'event':
        if (msg.category && msg.name)
          this.dispatchEvent({
            category:  msg.category,
            name:      msg.name,
            data:      msg.data,
            timestamp: msg.timestamp ?? new Date().toISOString(),
          });
        break;

      case 'pong':
        this.missedPings = 0;
        break;
    }
  }

  private dispatchEvent(evt: DraoxEvent): void {
    this.eventHandlers.get(`${evt.category}.${evt.name}`)?.forEach(h => h(evt));
    this.eventHandlers.get(evt.name)?.forEach(h => h(evt));
    this.categoryHandlers.get(evt.category)?.forEach(h => h(evt));
  }

  private onClosed(reason: string): void {
    this.stopHeartbeat();
    this.setState('disconnected');
    this.emit('disconnected', reason);

    if (this.cfg.reconnect.enabled && this.abort && !this.abort.signal.aborted)
      void this.tryReconnect();
  }

  private async tryReconnect(): Promise<void> {
    this.setState('reconnecting');

    const success = await this.reconnector.attempt(async () => {
      try {
        await this.transport!.connect(this.cfg.host, this.cfg.port, this.cfg.useTls);
        if (this.savedUserId) await this.authenticate(this.savedUserId, this.savedToken!);
        return true;
      } catch { return false; }
    }, this.abort!.signal);

    if (success) {
      this.setState('connected');
      this.emit('connected');
      this.startHeartbeat();
    }
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();
    this.heartbeat = setInterval(async () => {
      if (!this.transport?.isConnected) { this.stopHeartbeat(); return; }
      this.missedPings++;
      if (this.missedPings >= 2) { this.onClosed('heartbeat_timeout'); return; }
      try {
        await this.transport.send(Serializer.serialize({ type: 'ping', ts: Date.now() }));
      } catch { this.stopHeartbeat(); }
    }, this.cfg.heartbeatIntervalMs);
  }

  private stopHeartbeat(): void {
    if (this.heartbeat) { clearInterval(this.heartbeat); this.heartbeat = null; }
  }

  private ensureConnected(): void {
    if (this._state !== 'connected')
      throw new Error(`Not connected (state: ${this._state})`);
  }

  private addHandler(map: Map<string, Set<(e: DraoxEvent) => void>>, key: string, h: (e: DraoxEvent) => void): void {
    if (!map.has(key)) map.set(key, new Set());
    map.get(key)!.add(h);
  }

  private removeHandler(map: Map<string, Set<(e: DraoxEvent) => void>>, key: string, h: (e: DraoxEvent) => void): void {
    map.get(key)?.delete(h);
  }

  private newId(): string {
    return `req_${crypto.randomUUID().replace(/-/g, '')}`;
  }

  private setState(state: ClientState): void {
    if (this._state === state) return;
    this._state = state;
    this.emit('stateChanged', state);
  }
}
