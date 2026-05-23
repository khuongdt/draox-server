// Node.js only — gRPC native transport does not run in browsers or WebGL.
// For browser clients use WebSocketTransport instead.
import * as grpc from '@grpc/grpc-js';
import * as protoLoader from '@grpc/proto-loader';
import * as path from 'path';
import type { ITransport } from './ITransport';

const PROTO_PATH = path.resolve(__dirname, '../../../../../../proto/draox.proto');

// ── Internal helpers ──────────────────────────────────────────────────────────

let _packageDef: protoLoader.PackageDefinition | null = null;

function loadProto(): any {
  if (!_packageDef) {
    _packageDef = protoLoader.loadSync(PROTO_PATH, {
      keepCase: true,
      longs: String,
      enums: String,
      defaults: true,
      oneofs: true,
    });
  }
  const pkg = grpc.loadPackageDefinition(_packageDef) as any;
  return pkg?.draox?.v1 ?? pkg;
}

// ── GrpcTransport ─────────────────────────────────────────────────────────────

export class GrpcTransport implements ITransport {
  private channel:      grpc.Channel | null = null;
  private authStub:     any = null;
  private draoxStub:    any = null;
  private _sessionId:   string | null = null;
  private _subscribeCall: grpc.ClientReadableStream<any> | null = null;

  onMessage: ((data: string) => void) | null = null;
  onClose:   ((reason: string) => void) | null = null;

  get isConnected(): boolean {
    return this.channel !== null;
  }

  async connect(host: string, port: number, useTls = false): Promise<void> {
    const proto = loadProto();
    const address = `${host}:${port}`;
    const creds = useTls
      ? grpc.credentials.createSsl()
      : grpc.credentials.createInsecure();

    this.channel   = new grpc.Channel(address, creds, {});
    this.authStub  = new proto.AuthService(address, creds);
    this.draoxStub = new proto.DraoxService(address, creds);
  }

  disconnect(): void {
    this._subscribeCall?.cancel();
    this._subscribeCall = null;
    this.channel?.close();
    this.channel    = null;
    this.authStub   = null;
    this.draoxStub  = null;
    this._sessionId = null;
    this.onClose?.('client_disconnect');
  }

  // Authenticate and store session_id for subsequent calls.
  async authenticateGrpc(userId: string, token: string): Promise<string> {
    return new Promise((resolve, reject) => {
      this.authStub.Authenticate(
        { user_id: userId, token },
        (err: grpc.ServiceError | null, res: any) => {
          if (err) return reject(err);
          if (!res.success) return reject(new Error(res.error || 'auth failed'));
          this._sessionId = res.session_id;
          resolve(res.session_id);
        },
      );
    });
  }

  // ITransport.send — serialises action+payload as gRPC DraoxService.Send.
  async send(json: string): Promise<void> {
    return new Promise((resolve, reject) => {
      let parsed: { action?: string; payload?: unknown };
      try { parsed = JSON.parse(json); } catch { parsed = {}; }

      const payloadBytes = parsed.payload
        ? Buffer.from(JSON.stringify(parsed.payload))
        : Buffer.alloc(0);

      this.draoxStub.Send(
        {
          id:      crypto.randomUUID(),
          action:  parsed.action ?? '',
          payload: payloadBytes,
        },
        (err: grpc.ServiceError | null, res: any) => {
          if (err) return reject(err);
          if (!res.success) return reject(new Error(res.error || 'send failed'));
          if (this.onMessage && res.data?.length) {
            this.onMessage(Buffer.from(res.data).toString('utf8'));
          }
          resolve();
        },
      );
    });
  }

  // Subscribe to server-streaming events from DraoxService.Subscribe.
  subscribeEvents(categories: string[], onEvent: (e: unknown) => void): () => void {
    if (!this._sessionId) throw new Error('Not authenticated');

    const call: grpc.ClientReadableStream<any> = this.draoxStub.Subscribe({
      session_id: this._sessionId,
      categories,
    });

    call.on('data', onEvent);
    call.on('end',  () => this.onClose?.('stream_end'));
    call.on('error', (err: Error) => {
      if ((err as any).code !== grpc.status.CANCELLED) {
        this.onClose?.(err.message);
      }
    });

    this._subscribeCall = call;
    return () => call.cancel();
  }
}
