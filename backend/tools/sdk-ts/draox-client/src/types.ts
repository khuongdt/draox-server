export type DraoxProtocol = 'ws' | 'tcp' | 'grpc';
export type ClientState   = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';

export interface GrpcConfig {
  protoPath?:    string;                   // path to draox.proto (defaults to bundled)
  credentials?:  'insecure' | 'tls';      // default 'insecure'
}

export interface DraoxConfig {
  host?:                 string;        // default 'localhost'
  port?:                 number;        // default 9002 (ws) or 9004 (grpc)
  adminPort?:            number;        // default 9100 (Admin API port for login)
  protocol?:             DraoxProtocol; // default 'ws'
  useTls?:               boolean;       // default false
  timeoutMs?:            number;        // default 10_000
  heartbeatIntervalMs?:  number;        // default 30_000
  reconnect?:            ReconnectConfig;
  grpc?:                 GrpcConfig;    // gRPC-specific options (protocol='grpc' only)
}

export interface ReconnectConfig {
  enabled?:      boolean; // default true
  maxAttempts?:  number;  // default 5, 0 = unlimited
  baseDelayMs?:  number;  // default 1000
  maxDelayMs?:   number;  // default 30_000
}

export interface DraoxEvent {
  category:  string;
  name:      string;
  data:      unknown;
  timestamp: string;
}

// Internal resolved config (all fields required).
export interface ResolvedConfig {
  host:                string;
  port:                number;
  adminPort:           number;
  protocol:            DraoxProtocol;
  useTls:              boolean;
  timeoutMs:           number;
  heartbeatIntervalMs: number;
  reconnect: {
    enabled:     boolean;
    maxAttempts: number;
    baseDelayMs: number;
    maxDelayMs:  number;
  };
  grpc: Required<GrpcConfig>;
}

export interface LoginResponse {
  token:    string;
  username: string;
  role:     string;
}

// Internal wire format.
export interface WireResponse {
  id:       string;
  success:  boolean;
  data?:    unknown;
  error?:   string;
}
