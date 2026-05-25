import { Emitter } from './emitter.js';
import { WsTransport } from './transport.js';
import { RequestBroker } from './broker.js';
import { Reconnector } from './reconnector.js';
import type {
  DraoxConfig,
  ResolvedConfig,
  ClientState,
  WsFrame,
  WsResponse,
  ServerEvent,
  LoginResponse,
} from './types.js';

export class DraoxClient extends Emitter {
  private readonly cfg: ResolvedConfig;
  private readonly transport: WsTransport;
  private readonly broker: RequestBroker;
  private readonly reconnector: Reconnector;

  private heartbeat: ReturnType<typeof setInterval> | null = null;
  private abort: AbortController | null = null;
  private missedPings = 0;

  private _state: ClientState = 'disconnected';
  private _sessionId: string | null = null;
  private _token: string | null = null;
  private _savedUserId: string | null = null;
  private _savedToken: string | null = null;

  get state():           ClientState   { return this._state; }
  get sessionId():       string | null { return this._sessionId; }
  get isAuthenticated(): boolean       { return this._sessionId !== null; }
  get token():           string | null { return this._token; }

  get baseUrl(): string {
    return this.cfg.apiUrl;
  }

  constructor(config: DraoxConfig = {}) {
    super();
    this.cfg = {
      host:                config.host                ?? 'localhost',
      port:                config.port                ?? 9002,
      useTls:              config.useTls              ?? false,
      wsPath:              config.wsPath              ?? '/ws',
      apiUrl:              config.apiUrl              ?? '',
      timeoutMs:           config.timeoutMs           ?? 10_000,
      heartbeatIntervalMs: config.heartbeatIntervalMs ?? 30_000,
      reconnect: {
        enabled:     config.reconnect?.enabled     ?? true,
        maxAttempts: config.reconnect?.maxAttempts ?? 5,
        baseDelayMs: config.reconnect?.baseDelayMs ?? 1_000,
        maxDelayMs:  config.reconnect?.maxDelayMs  ?? 30_000,
      },
    };
    this.transport   = new WsTransport();
    this.broker      = new RequestBroker();
    this.reconnector = new Reconnector(this.cfg.reconnect);

    this.transport.onMessage = (msg) => this._onMessage(msg);
    this.transport.onClose   = (reason) => this._onClosed(reason);
  }

  // ── Public API ────────────────────────────────────────────────────────────

  async connect(): Promise<void> {
    if (this._state === 'connected' || this._state === 'connecting') return;

    this.abort = new AbortController();
    this._setState('connecting');

    await this.transport.connect(this.cfg.host, this.cfg.port, this.cfg.useTls, this.cfg.wsPath);

    this._setState('connected');
    this.emit('connected');
    this._startHeartbeat();
  }

  async disconnect(reason = 'client_disconnect'): Promise<void> {
    this._stopHeartbeat();
    this.abort?.abort();

    this.broker.failAll(new Error(`Disconnected: ${reason}`));
    this.transport.disconnect();

    this._sessionId = null;
    this._token     = null;
    this._setState('disconnected');
    this.emit('disconnected', reason);
  }

  async login(username: string, password: string): Promise<void> {
    const res = await fetch(`${this.baseUrl}/api/auth/login`, {
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

    this._token = body.data.token;
    await this._authenticate(body.data.username, body.data.token);
  }

  send(action: string, payload?: unknown): void {
    this._ensureConnected();
    const frame: WsFrame = { id: this._newId(), type: 'request', action, payload };
    this.transport.send(JSON.stringify(frame));
  }

  async request<T>(action: string, payload?: unknown): Promise<T> {
    this._ensureConnected();
    const id   = this._newId();
    const json = JSON.stringify({ id, type: 'request', action, payload } satisfies WsFrame);
    const res  = await this.broker.send(j => this.transport.send(j), json, id, this.cfg.timeoutMs);
    if (!res.success) throw new Error(res.error ?? 'Request failed');
    return res.data as T;
  }

  subscribe(eventName: string, handler: (e: ServerEvent) => void): () => void {
    this.on(eventName, handler);
    return () => this.off(eventName, handler);
  }

  async fetchApi<T>(path: string, init: RequestInit = {}): Promise<T> {
    const res = await fetch(`${this.baseUrl}${path}`, {
      ...init,
      headers: {
        'Content-Type': 'application/json',
        ...(this._token ? { Authorization: `Bearer ${this._token}` } : {}),
        ...(init.headers ?? {}),
      },
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(`API error (${res.status}): ${text}`);
    }

    const body = await res.json() as { success?: boolean; data?: T } | T;
    if (body !== null && typeof body === 'object' && 'success' in (body as object)) {
      return (body as { success: boolean; data: T }).data;
    }
    return body as T;
  }

  // ── Internal ──────────────────────────────────────────────────────────────

  private async _authenticate(userId: string, token: string): Promise<void> {
    this._savedUserId = userId;
    this._savedToken  = token;

    const data = await this.request<{ session_id: string }>('auth', { user_id: userId, token });
    this._sessionId = data.session_id;
    this.emit('authenticated');
  }

  private _onMessage(json: string): void {
    let msg: WsFrame;
    try { msg = JSON.parse(json) as WsFrame; } catch { return; }

    switch (msg.type) {
      case 'response':
        if (msg.id)
          this.broker.complete(msg.id, {
            id:      msg.id,
            success: msg.success ?? false,
            data:    msg.data,
            error:   msg.error,
          } satisfies WsResponse);
        break;

      case 'event':
        if (msg.category && msg.name) {
          const evt: ServerEvent = {
            category:  msg.category,
            name:      msg.name,
            data:      msg.data,
            timestamp: msg.timestamp ?? new Date().toISOString(),
          };
          this.emit(`${evt.category}.${evt.name}`, evt);
          this.emit(evt.name, evt);
          this.emit(evt.category, evt);
        }
        break;

      case 'pong':
        this.missedPings = 0;
        break;
    }
  }

  private _onClosed(reason: string): void {
    this._stopHeartbeat();
    this._setState('disconnected');
    this.emit('disconnected', reason);

    if (this.cfg.reconnect.enabled && this.abort && !this.abort.signal.aborted)
      void this._reconnect();
  }

  private async _reconnect(): Promise<void> {
    this._setState('reconnecting');
    this.emit('stateChanged', 'reconnecting');

    const ok = await this.reconnector.run(async () => {
      try {
        await this.transport.connect(this.cfg.host, this.cfg.port, this.cfg.useTls, this.cfg.wsPath);
        if (this._savedUserId) await this._authenticate(this._savedUserId, this._savedToken!);
        return true;
      } catch { return false; }
    }, this.abort!.signal);

    if (ok) {
      this._setState('connected');
      this.emit('connected');
      this._startHeartbeat();
    }
  }

  private _startHeartbeat(): void {
    this._stopHeartbeat();
    this.heartbeat = setInterval(() => {
      if (!this.transport.isConnected) { this._stopHeartbeat(); return; }
      this.missedPings++;
      if (this.missedPings >= 2) { this._onClosed('heartbeat_timeout'); return; }
      this.transport.send(JSON.stringify({ type: 'ping', ts: Date.now() }));
    }, this.cfg.heartbeatIntervalMs);
  }

  private _stopHeartbeat(): void {
    if (this.heartbeat) { clearInterval(this.heartbeat); this.heartbeat = null; }
  }

  private _ensureConnected(): void {
    if (this._state !== 'connected')
      throw new Error(`Not connected (state: ${this._state})`);
  }

  private _newId(): string {
    return `req_${crypto.randomUUID().replace(/-/g, '')}`;
  }

  private _setState(state: ClientState): void {
    if (this._state === state) return;
    this._state = state;
    this.emit('stateChanged', state);
  }
}
