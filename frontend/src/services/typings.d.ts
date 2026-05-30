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
    banned: boolean;
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
    server?: Record<string, unknown>;
    tcp?: Record<string, unknown>;
    udp?: Record<string, unknown>;
    websocket?: Record<string, unknown>;
    http?: Record<string, unknown>;
    grpc?: Record<string, unknown>;
    tls?: Record<string, unknown>;
    traffic_guard?: Record<string, unknown>;
    sessions?: Record<string, unknown>;
    storage?: Record<string, unknown>;
    cache?: Record<string, unknown>;
    billing?: Record<string, unknown>;
    admin_api?: Record<string, unknown>;
    logging?: Record<string, unknown>;
    metrics?: Record<string, unknown>;
    marketplace?: Record<string, unknown>;
    plugins?: Record<string, unknown>;
    [key: string]: Record<string, unknown> | undefined;
  }

  interface ConfigDiff {
    path: string;
    old: unknown;
    new: unknown;
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

  // ─── Channels (plugin-messaging) ──────────────────────────────────────────
  type ChannelType = 'Public' | 'Private' | 'Direct' | 'Announcement';

  interface Channel {
    id: string;
    name: string;
    description: string;
    created_by: string;
    created_at: string;
    channel_type: ChannelType;
    topic: string;
    is_system: boolean;
    frozen: boolean;
    member_count: number;
  }

  interface CreateChannelRequest {
    name: string;
    description?: string;
  }

  // ─── Clans (plugin-clans) ─────────────────────────────────────────────────
  interface Clan {
    id: string;
    name: string;
    tag: string;
    description: string;
    owner_id: string;
    member_count: number;
    max_members: number;
    created_at: string;
    is_system: boolean;
    frozen: boolean;
  }

  interface CreateClanRequest {
    name: string;
    tag: string;
  }

  type ClanRole = 'Owner' | 'Officer' | 'Member' | 'Recruit';

  interface ClanMember {
    client_id: string;
    role: ClanRole;
    joined_at: string;
  }
}
