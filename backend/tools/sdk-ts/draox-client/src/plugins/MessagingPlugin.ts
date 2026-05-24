import type { DraoxClient } from '../DraoxClient';
import type { DraoxEvent } from '../types';

// ── DTOs ──────────────────────────────────────────────────────────────────────

export interface MessageDto {
  id:           string;
  channel_id:   string;
  sender_id:    string;
  text:         string;
  reply_to_id?: string;
  sent_at:      string;
  edited_at?:   string;
}

export interface SendMessageResponse   { message: MessageDto; }
export interface MessageHistoryResponse { messages: MessageDto[]; has_more: boolean; oldest_id?: string; }
export interface MessageReceivedEvent  { message: MessageDto; }
export interface MessageDeletedEvent   { message_id: string; channel_id: string; }
export interface TypingEvent           { channel_id: string; user_id: string; username: string; is_typing: boolean; }

// ── Plugin ────────────────────────────────────────────────────────────────────

export class MessagingPlugin {
  onMessage?:        (e: MessageReceivedEvent)  => void;
  onMessageDeleted?: (e: MessageDeletedEvent)   => void;
  onTyping?:         (e: TypingEvent)            => void;

  private readonly bound: (e: DraoxEvent) => void;

  constructor(private readonly client: DraoxClient) {
    this.bound = this.handleMsgEvent.bind(this);
  }

  registerListeners():   void { this.client.subscribeCategory('msg', this.bound); }
  unregisterListeners(): void { this.client.unsubscribeCategory('msg', this.bound); }

  // ── Request API ─────────────────────────────────────────────────────────────

  sendMessage(channelId: string, text: string, replyToId?: string): Promise<SendMessageResponse> {
    return this.client.request<SendMessageResponse>(
      'msg.send', { channel_id: channelId, text, reply_to_id: replyToId });
  }

  getHistory(channelId: string, limit = 50, before?: string): Promise<MessageHistoryResponse> {
    return this.client.request<MessageHistoryResponse>(
      'msg.history', { channel_id: channelId, limit, before });
  }

  deleteMessage(messageId: string): Promise<unknown> {
    return this.client.request('msg.delete', { message_id: messageId });
  }

  editMessage(messageId: string, newText: string): Promise<MessageDto> {
    return this.client.request<MessageDto>('msg.edit', { message_id: messageId, text: newText });
  }

  sendTyping(channelId: string): Promise<void> {
    return this.client.send('msg.typing', { channel_id: channelId });
  }

  react(messageId: string, emoji: string): Promise<unknown> {
    return this.client.request('msg.react', { message_id: messageId, emoji });
  }

  // ── Internal ─────────────────────────────────────────────────────────────────

  private handleMsgEvent(evt: DraoxEvent): void {
    switch (evt.name) {
      case 'received': this.onMessage?.(evt.data as MessageReceivedEvent); break;
      case 'deleted':  this.onMessageDeleted?.(evt.data as MessageDeletedEvent); break;
      case 'typing':   this.onTyping?.(evt.data as TypingEvent); break;
    }
  }
}
