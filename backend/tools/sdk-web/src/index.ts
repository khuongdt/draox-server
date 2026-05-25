export { DraoxClient }     from './client.js';
export { MessagingPlugin } from './plugins/messaging.js';
export { Emitter }         from './emitter.js';

export type {
  DraoxConfig,
  ReconnectConfig,
  ClientState,
  WsFrame,
  ServerEvent,
  LoginResponse,
  MessageDto,
  ReactionDto,
  ChannelDto,
  SendMessageResponse,
  MessageHistoryResponse,
  MessageReceivedEvent,
  MessageDeletedEvent,
  TypingEvent,
  PresenceEvent,
  ChannelEvent,
} from './types.js';
