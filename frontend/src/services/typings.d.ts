declare namespace API {
  // ─── Health ───────────────────────────────────────────────────────────────
  interface HealthResponse {
    status: string;
    uptime_secs: number;
    version: string;
  }

  interface DetailedHealth {
    status: string;
    components: Record<string, ComponentHealth>;
  }

  interface ComponentHealth {
    status: 'healthy' | 'degraded' | 'unhealthy' | 'unknown';
    message?: string;
  }

  interface ServerInfo {
    name: string;
    version: string;
    protocols: string[];
    max_connections: number;
    uptime_secs: number;
  }

  // ─── Auth ─────────────────────────────────────────────────────────────────
  interface LoginResult {
    token: string;
    role: string;
    username: string;
  }

  // ─── Users (Admin Management) ─────────────────────────────────────────────
  type AdminRole = 'admin' | 'operator' | 'viewer';

  interface AdminUser {
    username: string;
    role: AdminRole;
  }

  interface CreateUserRequest {
    username: string;
    password: string;
    role: AdminRole;
  }

  interface UpdateUserRequest {
    password?: string;
    role?: AdminRole;
  }

  interface CurrentUser {
    token: string;
    role: string;
    username: string;
    avatar?: string;
  }

  // ─── Connections ──────────────────────────────────────────────────────────
  interface Connection {
    id: string;
    remote_addr: string;
    protocol: 'tcp' | 'udp' | 'websocket' | 'http';
    connected_at: string;
    bytes_sent: number;
    bytes_received: number;
    state: string;
    session_id?: string;
  }

  interface ConnectionStats {
    total: number;
    active: number;
    by_protocol: Record<string, number>;
  }

  // ─── Sessions ─────────────────────────────────────────────────────────────
  interface Session {
    id: string;
    client_id: string;
    connections: string[];
    created_at: string;
    state: string;
    metadata: Record<string, unknown>;
  }

  interface SessionMetrics {
    bytes_sent: number;
    bytes_received: number;
    duration_secs: number;
    connection_count: number;
  }

  // ─── Plugins ──────────────────────────────────────────────────────────────
  type PluginState = 'Installed' | 'ActiveEnabled' | 'ActiveDisabled' | 'Uninstalled';
  type PluginType = 'builtin' | 'wasm';

  interface Plugin {
    id: string;
    name: string;
    version: string;
    author: string;
    description: string;
    plugin_type: PluginType;
    state: PluginState;
    enabled: boolean;
  }

  interface PluginHealth {
    plugin_id: string;
    status: string;
    details: Record<string, unknown>;
  }

  // ─── Traffic Guard ────────────────────────────────────────────────────────
  interface GuardStats {
    active_bans: number;
    blacklisted_entries: number;
    whitelisted_entries: number;
  }

  interface BanEntry {
    ip: string;
    reason: string;
    expires_at: string;
    ban_count: number;
  }

  interface BanListResponse {
    total: number;
    bans: BanEntry[];
  }

  interface ReputationResponse {
    ip: string;
    score: number;
  }

  // ─── Config ───────────────────────────────────────────────────────────────
  interface ServerConfig {
    [section: string]: Record<string, unknown>;
  }

  // ─── Billing ──────────────────────────────────────────────────────────────
  interface BillingPlan {
    id: string;
    name: string;
    max_connections: number;
    max_bandwidth: number;
    price_cents: number;
    features: string[];
  }

  interface UsageInfo {
    client_id: string;
    plan_id: string;
    connections_used: number;
    bandwidth_used: number;
    api_calls: number;
  }

  // ─── Cache ────────────────────────────────────────────────────────────────
  interface CacheStats {
    backend: string;
    hits: number;
    misses: number;
    keys: number;
    memory_bytes: number;
    hit_rate: number;
  }

  interface CacheHealth {
    status: string;
    latency_ms: number;
  }

  // ─── Audit ────────────────────────────────────────────────────────────────
  interface AuditEntry {
    id: string;
    timestamp: string;
    action: string;
    actor: string;
    target: string;
    severity: 'critical' | 'high' | 'medium' | 'low';
    details: Record<string, unknown>;
  }

  // ─── Metrics ──────────────────────────────────────────────────────────────
  interface MetricsSnapshot {
    timestamp: string;
    connections_active: number;
    connections_total: number;
    bytes_sent: number;
    bytes_received: number;
    requests_total: number;
    errors_total: number;
  }

  interface ActivityMetrics {
    total_events: number;
    events_per_minute: number;
    top_actions: Record<string, number>;
  }

  // ─── Marketplace ──────────────────────────────────────────────────────────
  interface MarketplacePlugin {
    id: string;
    name: string;
    author: string;
    description: string;
    version: string;
    downloads: number;
    rating: number;
    price_cents: number;
    category: string;
    icon_url?: string;
  }

  interface PluginVersion {
    version: string;
    changelog: string;
    published_at: string;
    downloads: number;
  }

  interface PluginReview {
    id: string;
    author: string;
    rating: number;
    comment: string;
    created_at: string;
  }

  interface PluginAnalytics {
    total_downloads: number;
    monthly_downloads: number;
    average_rating: number;
    review_count: number;
  }

  // ─── Dynamic Routes ───────────────────────────────────────────────────────
  interface DynamicRoute {
    plugin_id: string;
    path: string;
    methods: string[];
    created_at: string;
  }

  // ─── WebSocket Events ─────────────────────────────────────────────────────
  type EventCategory = 'connection' | 'session' | 'guard' | 'plugin' | 'server' | 'custom';

  interface ServerEvent {
    type: string;
    data: unknown;
    timestamp: string;
    category: EventCategory;
  }
}
