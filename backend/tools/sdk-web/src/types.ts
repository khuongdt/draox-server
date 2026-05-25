// ── Configuration ────────────────────────────────────────────────────────────

export interface ReconnectConfig {
  enabled?: boolean;
  maxAttempts?: number;
  baseDelayMs?: number;
  maxDelayMs?: number;
}

export interface DraoxConfig {
  host?: string;
  port?: number;
  useTls?: boolean;
  wsPath?: string;
  /** Base URL for REST API calls.
   *  Leave empty (default) to use relative paths — works behind a reverse proxy or Vite dev proxy.
   *  Set explicitly (e.g. 'http://localhost:9100') only for direct / non-proxied access. */
  apiUrl?: string;
  timeoutMs?: number;
  heartbeatIntervalMs?: number;
  reconnect?: ReconnectConfig;
}

export interface ResolvedConfig {
  host: string;
  port: number;
  useTls: boolean;
  wsPath: string;
  apiUrl: string;
  timeoutMs: number;
  heartbeatIntervalMs: number;
  reconnect: Required<ReconnectConfig>;
}

// ── Client State ──────────────────────────────────────────────────────────────

export type ClientState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';

// ── WebSocket Protocol ────────────────────────────────────────────────────────

export interface WsFrame {
  id?: string;
  type: string;
  action?: string;
  payload?: unknown;
  success?: boolean;
  data?: unknown;
  error?: string;
  category?: string;
  name?: string;
  timestamp?: string;
  ts?: number;
}

export interface WsResponse {
  id: string;
  success: boolean;
  data: unknown;
  error?: string;
}

// ── Auth ──────────────────────────────────────────────────────────────────────

export interface LoginResponse {
  token: string;
  username: string;
  role: string;
}

// ── Messaging DTOs ────────────────────────────────────────────────────────────

export interface MessageDto {
  id: string;
  channel_id: string;
  sender_id: string;
  text: string;
  reply_to_id?: string;
  sent_at: string;
  edited_at?: string;
  reactions?: ReactionDto[];
}

export interface ReactionDto {
  emoji: string;
  users: string[];
}

export interface ChannelDto {
  id: string;
  name: string;
  description: string;
  created_by: string;
  created_at: string;
  channel_type: 'Public' | 'Private' | 'Direct' | 'Announcement';
  topic: string;
}

export interface SendMessageResponse {
  message: MessageDto;
}

export interface MessageHistoryResponse {
  messages: MessageDto[];
  has_more: boolean;
  oldest_id?: string;
}

// ── Server Events ─────────────────────────────────────────────────────────────

export interface ServerEvent {
  category: string;
  name: string;
  data: unknown;
  timestamp: string;
}

export interface MessageReceivedEvent {
  message: MessageDto;
}

export interface MessageDeletedEvent {
  message_id: string;
  channel_id: string;
}

export interface TypingEvent {
  channel_id: string;
  user_id: string;
  username: string;
  is_typing: boolean;
}

export interface PresenceEvent {
  user_id: string;
  status: 'online' | 'away' | 'offline';
}

export interface ChannelEvent {
  type: string;
  channel_id: string;
  user_id?: string;
  name?: string;
}
