import type { DraoxClient } from '../client.js';
import type {
  MessageDto,
  ChannelDto,
  MessageHistoryResponse,
  MessageReceivedEvent,
  MessageDeletedEvent,
  TypingEvent,
  PresenceEvent,
  ChannelEvent,
  ServerEvent,
} from '../types.js';

function unwrapMessages(res: unknown): MessageDto[] {
  if (Array.isArray(res)) return res as MessageDto[];
  const r = res as Record<string, unknown>;
  if (Array.isArray(r?.messages)) return r.messages as MessageDto[];
  if (Array.isArray(r?.data)) return r.data as MessageDto[];
  return [];
}

function unwrapChannels(res: unknown): ChannelDto[] {
  if (Array.isArray(res)) return res as ChannelDto[];
  const r = res as Record<string, unknown>;
  if (Array.isArray(r?.channels)) return r.channels as ChannelDto[];
  if (Array.isArray(r?.data)) return r.data as ChannelDto[];
  return [];
}

export class MessagingPlugin {
  constructor(private readonly client: DraoxClient) {}

  // ── Messages ──────────────────────────────────────────────────────────────

  async sendMessage(channelId: string, text: string, replyToId?: string): Promise<MessageDto> {
    const body: Record<string, unknown> = { channel_id: channelId, text };
    if (replyToId) body.reply_to_id = replyToId;
    const res = await this.client.fetchApi<{ message: MessageDto }>('/api/messages/send', {
      method: 'POST',
      body:   JSON.stringify(body),
    });
    return res.message;
  }

  async getHistory(channelId: string, limit = 50, before?: string): Promise<MessageHistoryResponse> {
    const params = new URLSearchParams({ limit: String(limit) });
    if (before) params.set('before', before);
    const res = await this.client.fetchApi<unknown>(`/api/channels/${channelId}/messages?${params}`);
    const messages = unwrapMessages(res);
    const r = res as Record<string, unknown>;
    return {
      messages,
      has_more:   Boolean(r?.has_more),
      oldest_id:  r?.oldest_id as string | undefined,
    };
  }

  async deleteMessage(messageId: string): Promise<void> {
    await this.client.fetchApi(`/api/messages/${messageId}`, { method: 'DELETE' });
  }

  async editMessage(messageId: string, text: string): Promise<MessageDto> {
    const res = await this.client.fetchApi<{ message: MessageDto }>(`/api/messages/${messageId}`, {
      method: 'PATCH',
      body:   JSON.stringify({ text }),
    });
    return res.message ?? (res as unknown as MessageDto);
  }

  async react(messageId: string, emoji: string): Promise<void> {
    await this.client.fetchApi(`/api/messages/${messageId}/react`, {
      method: 'POST',
      body:   JSON.stringify({ emoji }),
    });
  }

  async sendTyping(channelId: string): Promise<void> {
    await this.client.fetchApi(`/api/channels/${channelId}/typing`, { method: 'POST' }).catch(() => {});
  }

  // ── Channels ──────────────────────────────────────────────────────────────

  async getChannels(): Promise<ChannelDto[]> {
    const res = await this.client.fetchApi<unknown>('/api/channels');
    return unwrapChannels(res);
  }

  async createChannel(name: string, description = ''): Promise<ChannelDto> {
    const res = await this.client.fetchApi<{ channel: ChannelDto }>('/api/channels', {
      method: 'POST',
      body:   JSON.stringify({ name, description }),
    });
    return res.channel ?? (res as unknown as ChannelDto);
  }

  async joinChannel(channelId: string): Promise<void> {
    await this.client.fetchApi(`/api/channels/${channelId}/subscribe`, { method: 'POST' });
  }

  async leaveChannel(channelId: string): Promise<void> {
    await this.client.fetchApi(`/api/channels/${channelId}/unsubscribe`, { method: 'POST' });
  }

  // ── Event Subscriptions ───────────────────────────────────────────────────

  onMessage(handler: (e: MessageReceivedEvent) => void): () => void {
    const h = (e: ServerEvent) => { if (e.data) handler(e.data as MessageReceivedEvent); };
    return this.client.subscribe('messaging.message_sent', h);
  }

  onMessageDeleted(handler: (e: MessageDeletedEvent) => void): () => void {
    const h = (e: ServerEvent) => { if (e.data) handler(e.data as MessageDeletedEvent); };
    return this.client.subscribe('messaging.message_deleted', h);
  }

  onTyping(handler: (e: TypingEvent) => void): () => void {
    const h = (e: ServerEvent) => { if (e.data) handler(e.data as TypingEvent); };
    return this.client.subscribe('messaging.typing_started', h);
  }

  onPresence(handler: (e: PresenceEvent) => void): () => void {
    const h = (e: ServerEvent) => { if (e.data) handler(e.data as PresenceEvent); };
    return this.client.subscribe('messaging.presence_changed', h);
  }

  onChannelEvent(handler: (e: ChannelEvent) => void): () => void {
    const created  = this.client.subscribe('messaging.channel_created', (e: ServerEvent) => handler({ type: 'created',  ...(e.data as object) } as ChannelEvent));
    const deleted  = this.client.subscribe('messaging.channel_deleted', (e: ServerEvent) => handler({ type: 'deleted',  ...(e.data as object) } as ChannelEvent));
    const joined   = this.client.subscribe('messaging.user_joined_channel', (e: ServerEvent) => handler({ type: 'joined',   ...(e.data as object) } as ChannelEvent));
    const left     = this.client.subscribe('messaging.user_left_channel',   (e: ServerEvent) => handler({ type: 'left',     ...(e.data as object) } as ChannelEvent));
    return () => { created(); deleted(); joined(); left(); };
  }
}
